//! The lifecycle dialog's credential stops (D5 design, A3), with D4's
//! recovery: Unlock…, Open Sharing & keys and Check host, and a Retry that
//! takes D4's unlock path. Canned (decision 9), over the helpers of
//! `outcome_canned_tests.rs`: no key or host is used here.

use std::{
    fs,
    sync::mpsc::{self, Receiver},
    thread,
};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleCleanupPendingReason, LifecycleCleanupState, RequestId, SharedKeyId,
        keys::{
            KeySourceToken, PassphraseResponse, SessionCredentialProvider, UnlockReason,
            UnlockRequest,
        },
    },
    results::{CleanupEffect, Effects, Outcome, ResultCode},
};

use super::{
    outcome::{CHECK_HOST, OPEN_SHARING_AND_KEYS, UNLOCK},
    outcome_canned_tests::{
        answer, buttons, finished, inspect, intent, reached, reached_effects, reopen, retry,
        short_of_main,
    },
    tests::{TICKET, closes, request, saved_ticket, shell, ticket, until_present},
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::JobKind,
    view::{
        harness::{Shell, WAIT, wait_until},
        item::LIFECYCLE_ACTION,
        settings::Shown,
    },
};

/// A3, D4's credential recovery, canned (decision 9): no key is needed
/// here. The answers are the library's: the shared branch's deletion
/// stopped for a locked key (`tests/mutation_remote/c5_lifecycle.rs`
/// `deletion_unlock`: `unlock_required`, partial, cleanup pending
/// `remote_deletion_unlock`, `request.retry`), for no usable key and for an
/// unknown host (`src/repository/mutation/bind/lifecycle_tests.rs`
/// `a_deletion_that_stopped_answers_with_its_category`: `no_selected_key`,
/// `host_approval_required`; `c5_lifecycle.rs` `deletion_host`). A locked
/// key offers Unlock… and Open Sharing & keys; no usable key, Open Sharing
/// & keys and Retry; an unknown host, Check host and Retry. Open Sharing &
/// keys opens the Keys page.
#[gpui_kit::test]
async fn a_refused_key_offers_unlock_and_sharing_and_keys(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Keys", ""));
    let request = RequestId::new();
    let stopped = |code, reason, recovery| {
        let mut data = reached(ItemDtoKind::Ticket, TICKET, vec![]);
        data.cleanup = Some(LifecycleCleanupState::Pending);
        data.cleanup_pending = Some(reason);
        answer(
            "ticket close",
            &root,
            code,
            Outcome::Partial,
            data,
            reached_effects(CleanupEffect::Pending),
            recovery,
            request,
        )
    };
    let rows = [
        (
            stopped(
                ResultCode::UnlockRequired,
                LifecycleCleanupPendingReason::RemoteDeletionUnlock,
                vec![retry(request)],
            ),
            vec!["lifecycle-unlock", "lifecycle-sharing-and-keys"],
        ),
        (
            stopped(
                ResultCode::NoSelectedKey,
                LifecycleCleanupPendingReason::RemoteDeletionKey,
                vec![retry(request)],
            ),
            vec!["lifecycle-retry", "lifecycle-sharing-and-keys"],
        ),
        (
            stopped(
                ResultCode::HostApprovalRequired,
                LifecycleCleanupPendingReason::RemoteDeletionHost,
                // As the `ticket_close_host_approval_required` golden: the
                // same request, then the repository (review M-2).
                vec![retry(request), inspect(&root)],
            ),
            vec![
                "lifecycle-retry",
                "lifecycle-check-host",
                "lifecycle-inspect",
            ],
        ),
    ];
    for (n, (envelope, expected)) in rows.into_iter().enumerate() {
        let id = finished(
            &shell,
            n as u64,
            intent(&root, ItemDtoKind::Ticket),
            request,
            envelope,
            cx,
        );
        reopen(&shell, id, cx).await;
        assert_eq!(buttons(&shell, cx), expected, "row {n}");
        match n {
            0 => {
                assert_eq!(shell.label("lifecycle-unlock", cx).as_deref(), Some(UNLOCK));
                assert_eq!(
                    shell.label("lifecycle-sharing-and-keys", cx).as_deref(),
                    Some(OPEN_SHARING_AND_KEYS)
                );
                let outcome = shell.rows("lifecycle-outcome", cx);
                assert_eq!(
                    outcome.last().map(String::as_str),
                    Some("Not tidied up yet: the key must be unlocked")
                );
                shell.click("lifecycle-sharing-and-keys", cx);
                shell.run(cx);
                assert!(!shell.present("lifecycle-dialog", cx));
                let shown = shell
                    .workspace
                    .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
                assert!(matches!(shown, Shown::Settings(SettingsPage::Keys)));
                assert!(
                    shell.holds_focus("keys-make", cx),
                    "W-21 lands on the key list, with no key Make a new key (Part D)"
                );
                // Settings is a sheet: it closes before the next row reopens
                // the dialog.
                shell.keys("escape", cx);
            }
            2 => {
                assert_eq!(
                    shell.label("lifecycle-check-host", cx).as_deref(),
                    Some(CHECK_HOST)
                );
                shell.keys("escape", cx);
            }
            _ => shell.keys("escape", cx),
        }
    }
}

/// A provider call for the key `key_id` in flight on a plain thread, as
/// the worker makes one (D4's `outcomes_route_tests`).
fn ask(shell: &Shell, key_id: SharedKeyId, cx: &mut TestAppContext) -> Receiver<&'static str> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("key");
    fs::write(&path, b"opaque fixture bytes").unwrap();
    let request = UnlockRequest {
        key_id,
        label: "primary".to_owned(),
        source: KeySourceToken::observe(&path).unwrap(),
        reason: UnlockReason::ProtectedKey,
    };
    let mut provider = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().provider());
    let (done, answer) = mpsc::channel();
    thread::spawn(move || {
        let _directory = directory;
        let answer = match provider.request_passphrase(&request) {
            PassphraseResponse::Supplied(_) => "supplied",
            PassphraseResponse::Cancelled => "cancelled",
            PassphraseResponse::Unavailable => "unavailable",
        };
        let _ = done.send(answer);
    });
    answer
}

/// Asks for `key_id`, waits for the passphrase dialog, and declines it with
/// Not now.
async fn declined(shell: &Shell, key_id: SharedKeyId, cx: &mut TestAppContext) {
    let asked = ask(shell, key_id, cx);
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.passphrase().is_some()
    })
    .await;
    shell.run(cx);
    shell.click("passphrase-cancel", cx);
    assert_eq!(asked.recv_timeout(WAIT).unwrap(), "cancelled");
}

/// A2, R-3, canned (decision 9): no protected key is used here. The answer
/// is the library's (`tests/mutation_remote/c5_lifecycle.rs` `unlock_stop`,
/// the `ticket_close_unlock_required` golden: `unlock_required`, blocked,
/// `request.retry`). The close's key was declined with Not now while it
/// ran. Unlock… clears that, and sends the same request again under its own
/// request ID with no confirmation and no passphrase: the key is then asked
/// for again, through D4's prompt, rather than refused unasked.
#[gpui_kit::test]
async fn a_resend_without_a_secret_takes_the_unlock_path(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Locked", "")
    });
    super::tests::open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    let named = RequestId::new();
    let held = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        answer(
            "ticket close",
            &root,
            ResultCode::UnlockRequired,
            Outcome::Blocked,
            short_of_main(ItemDtoKind::Ticket, TICKET, vec![]),
            Effects::not_requested(),
            vec![retry(named)],
            named,
        ),
    );
    shell.click("lifecycle-confirm", cx);
    held.wait_reached();
    let key_id = SharedKeyId::new();
    declined(&shell, key_id, cx).await;
    // Declined: asked again, it answers at once.
    assert_eq!(
        ask(&shell, key_id, cx).recv_timeout(WAIT).unwrap(),
        "cancelled"
    );
    held.release();
    let first = closes(&shell, cx)[0];
    shell.until_ended(first, cx).await;
    until_present(&shell, "lifecycle-unlock", cx).await;

    let again = shell.backend.hold_job(JobKind::TicketClose);
    shell.click("lifecycle-unlock", cx);
    again.wait_reached();
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2, "Unlock… sent the close again");
    let (asked, resent) = (request(&shell, first, cx), request(&shell, sent[0], cx));
    assert_eq!(resent.request_id, asked.request_id, "the same request");
    assert!(resent.confirmation.is_none(), "no confirmation");
    declined(&shell, key_id, cx).await;
    again.release();
    shell.until_ended(sent[0], cx).await;
}

/// Review M-6, canned as `a_refused_key_offers_unlock_and_sharing_and_keys`
/// (`deletion_unlock`): on a locked-key stop, which offers Unlock… and no
/// Retry, Enter unlocks. It sends the same request again under its own
/// request ID, with no confirmation.
#[gpui_kit::test]
async fn enter_on_a_locked_key_unlocks(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Enter", ""));
    let named = RequestId::new();
    let mut data = reached(ItemDtoKind::Ticket, TICKET, vec![]);
    data.cleanup = Some(LifecycleCleanupState::Pending);
    data.cleanup_pending = Some(LifecycleCleanupPendingReason::RemoteDeletionUnlock);
    let id = finished(
        &shell,
        0,
        intent(&root, ItemDtoKind::Ticket),
        named,
        answer(
            "ticket close",
            &root,
            ResultCode::UnlockRequired,
            Outcome::Partial,
            data,
            reached_effects(CleanupEffect::Pending),
            vec![retry(named)],
            named,
        ),
        cx,
    );
    reopen(&shell, id, cx).await;
    assert!(shell.present("lifecycle-unlock", cx));
    assert!(!shell.present("lifecycle-retry", cx));
    let held = shell.backend.hold_job(JobKind::TicketClose);
    shell.keys("enter", cx);
    held.wait_reached();
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2, "Enter sent the close again");
    let resent = request(&shell, sent[0], cx);
    assert_eq!(resent.request_id, named, "the same request");
    assert!(resent.confirmation.is_none());
    held.release();
    shell.until_ended(sent[0], cx).await;
}
