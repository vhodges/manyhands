//! The confirmation dialog over a real pending operation: one request
//! for two presses of Enter, Escape with nothing asked and focus given
//! back, and a stale preview that offers Prepare again.

use std::{path::PathBuf, time::Duration};

use gpui_kit::{TestAppContext, component::dialog::Confirm};
use manyhands::{
    repository::{LeaseKind, RepositoryService},
    results::{Outcome, ResultCode},
};

use super::{CLOSE, PREPARE_AGAIN, RETRY, STALE_CODES, SUBMITTED, local_time, prepare_again};
use crate::desktop::{
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::fixtures::{Fixture, resume_through},
    view::{harness::Shell, workspace::Region},
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH0";

/// A ticket save whose index hand-off failed, pending in `a`, with the
/// window open: its operation ID.
fn stranded(cx: &mut TestAppContext) -> (Shell, PathBuf, String) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, TICKET, "Stranded", "open");
    let operation = fixture.strand_ticket_save(&root, TICKET, "Edited");
    let shell = Shell::open(fixture, cx);
    (shell, root, operation)
}

fn abandons(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationAbandon { .. })
    })
}

fn finished(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> (Outcome, ResultCode) {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(outcome, code, ..) => (*outcome, *code),
            _ => panic!("not finished"),
        }
    })
}

/// Review Focus 4: Enter on Confirm submits the request once and makes
/// Confirm unavailable; Enter again does nothing. One request, one
/// abandon.
#[gpui_kit::test]
async fn pressing_enter_twice_on_confirm_submits_one_request(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, _, _) = stranded(cx);
    shell.click(("pending-abandon", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));

    let held = shell.backend.hold_job(JobKind::OperationAbandon);
    shell.keys("enter", cx);
    held.wait_reached();
    assert_eq!(shell.label("confirm-state", cx).as_deref(), Some(SUBMITTED));
    shell.keys("enter", cx);
    assert_eq!(abandons(&shell, cx).len(), 1);

    held.release();
    let id = abandons(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert_eq!(abandons(&shell, cx), [id]);
    let ran = shell
        .backend
        .jobs_seen()
        .into_iter()
        .filter(|kind| *kind == JobKind::OperationAbandon)
        .count();
    assert_eq!(ran, 1);
    assert!(!shell.present("confirm-dialog", cx));
}

/// Escape closes the dialog with nothing asked of the worker, and focus
/// goes back to the Abandon control that opened it.
#[gpui_kit::test]
fn escape_closes_the_dialog_with_no_request_and_focus_returns_to_abandon(cx: &mut TestAppContext) {
    let (shell, root, operation) = stranded(cx);
    shell.click(("pending-abandon", 0usize), cx);
    assert!(shell.present("confirm-dialog", cx));

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(shell.requests(cx, |_| true).is_empty());
    let focus = shell
        .workspace
        .read_with(cx, |workspace, _| {
            workspace.abandon_focus(root.to_str().unwrap(), &operation)
        })
        .expect("the Abandon control has a focus");
    assert!(shell.frame(cx, |window, _| focus.is_focused(window)));
    assert!(shell.present(Region::Operations.ring_id(), cx));
    // The pending row is untouched.
    assert!(shell.present(("pending-row", 0usize), cx));
}

/// A stale preview: the operation is finished through the library after
/// the preview was taken. Confirm is answered as a change or a spent
/// confirmation, shown with Prepare again, which takes a fresh preview
/// under the same request.
#[gpui_kit::test]
async fn a_stale_preview_is_shown_with_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, operation) = stranded(cx);
    shell.click(("pending-abandon", 0usize), cx);
    assert!(shell.present("confirm-ok", cx));

    let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    resume_through(&other, &root, &operation);

    shell.click("confirm-ok", cx);
    let id = abandons(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let (_, code) = finished(&shell, id, cx);
    assert!(STALE_CODES.contains(&code), "{}", code.as_str());
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(code.message())
    );
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some(PREPARE_AGAIN)
    );
    assert!(!shell.present("confirm-ok", cx));

    // A fresh preview of the same request: the operation has ended, so
    // there is nothing left to confirm, and no second request.
    shell.click("confirm-prepare-again", cx);
    let code = shell
        .label("confirm-code", cx)
        .expect("the new preview's answer");
    assert_ne!(code, ResultCode::Ok.as_str());
    assert_eq!(abandons(&shell, cx), [id]);
}

/// Ruling R-A1: a prepare that stays busy through its retries is shown
/// as busy with Retry and Close. Retry, once the lease is let go, takes
/// the preview, and Confirm is offered.
#[gpui_kit::test]
fn a_busy_preview_offers_retry_which_prepares_again(cx: &mut TestAppContext) {
    let (shell, root, _) = stranded(cx);
    let lease = RepositoryService::hold_lease_for_testing(
        &root,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    shell.click(("pending-abandon", 0usize), cx);
    cx.executor().advance_clock(Duration::from_secs(1));
    shell.run(cx);

    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(ResultCode::Busy.as_str())
    );
    assert_eq!(shell.label("confirm-retry", cx).as_deref(), Some(RETRY));
    assert_eq!(shell.label("confirm-cancel", cx).as_deref(), Some(CLOSE));
    assert!(!shell.present("confirm-ok", cx));

    drop(lease);
    shell.click("confirm-retry", cx);
    assert!(shell.present("confirm-ok", cx));
    assert!(shell.present("confirm-command-name", cx));
    assert!(!shell.present("confirm-retry", cx));
    assert!(shell.requests(cx, |_| true).is_empty());
}

/// The expiry is shown as a wall-clock time; an unreadable one as given.
#[test]
fn the_expiry_is_shown_in_local_time() {
    let shown = local_time("2026-10-19T12:34:56Z");
    // Local time where the offset is read; UTC, said so, where not.
    let time = shown.strip_suffix(" UTC").unwrap_or(&shown);
    assert_eq!(time.len(), "2026-10-19 12:34:56".len(), "{shown}");
    assert!(time.ends_with(":56"), "{shown}");
    assert_eq!(local_time("not a time"), "not a time");
}

/// Prepare again re-queues the same request, whose `Queued` event comes
/// through the worker's channel. A notify of the request model before it
/// arrives finds the earlier attempt's answer still there: the dialog is
/// not that answer's to show. It stays Submitted and closes once this
/// attempt succeeds.
#[gpui_kit::test]
async fn an_answer_to_an_earlier_attempt_is_not_shown_after_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _) = stranded(cx);

    // The first attempt is answered `busy`: another call holds the lease.
    shell.click(("pending-abandon", 0usize), cx);
    let lease = RepositoryService::hold_lease_for_testing(
        &root,
        shell.fixture.data_dir(),
        LeaseKind::Repository,
    )
    .unwrap();
    shell.click("confirm-ok", cx);
    let id = abandons(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(finished(&shell, id, cx).1, ResultCode::Busy);
    drop(lease);
    assert!(!shell.present("confirm-dialog", cx));

    // Prepare again, as the operation area's `request.prepare` does.
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    shell.visual(cx).update(|window, cx| {
        let origin = cx.focus_handle();
        prepare_again(id, origin, &services, window, cx);
    });
    shell.run(cx);
    assert!(shell.present("confirm-ok", cx));

    // Confirm, and notify the model before the re-queue's event is
    // applied: both in one update, so the worker's channel is not drained.
    let operations = shell.operations(cx);
    shell.visual(cx).update(|window, cx| {
        window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
        operations.update(cx, |_, cx| cx.notify());
    });
    shell.frame(cx, |_, _| ());
    assert_eq!(shell.label("confirm-state", cx).as_deref(), Some(SUBMITTED));
    assert!(!shell.present("confirm-prepare-again", cx));

    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    assert_eq!(finished(&shell, id, cx), (Outcome::Success, ResultCode::Ok));
    assert!(!shell.present("confirm-dialog", cx));
    assert_eq!(abandons(&shell, cx), [id]);
}
