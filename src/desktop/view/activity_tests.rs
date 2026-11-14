//! Activity's window tests that belong to the log (Wave 04 Cycle 03b): the
//! footer's count against the log's attention rows through a journey
//! (`count_and_popover_agree`, P13's count half) and the coalescing of
//! refreshes (`refreshes_are_coalesced`, P3's T10), and each action's
//! handler through `choose_activity` (`each_action_runs_its_handler`, P18).

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        OperationAction, OperationDto, OperationFamily, OperationListDto, OperationNextAction,
        OperationOwner,
    },
    results::{Outcome, ResultCode},
};

use super::super::{
    footer::{
        Control, attention_text,
        tests::support::{finish, read},
    },
    harness::Shell,
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityRow, RowKey},
        layout::PanelKind,
        operations::Operations,
    },
    services::{ReadFailure, ReadResult, requests::DesktopRequestId},
    test_support::fixtures::Fixture,
    view::needs_attention::tests::support as popover,
};

const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

/// A pending operation waiting to be resumed.
fn waiting() -> OperationDto {
    OperationDto {
        operation_id: Some(OPERATION.to_owned()),
        family: OperationFamily::Local,
        owner: OperationOwner::Repository,
        action: OperationAction::SaveTicket,
        state: "prepared".to_owned(),
        completed_step: None,
        next_action: Some(OperationNextAction::Resume),
        item_id: None,
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    }
}

/// `root`'s pending list, read as `result`.
fn read_pending(
    shell: &Shell,
    root: &str,
    result: ReadResult<OperationListDto>,
    cx: &mut TestAppContext,
) {
    shell
        .operations(cx)
        .update(cx, |operations: &mut Operations, cx| {
            let stamp = operations.load_pending(root.to_owned());
            assert!(operations.apply_pending(stamp, result));
            cx.notify();
        });
    shell.run(cx);
}

/// The keys of the log's attention rows, sorted: what the popover will list.
fn attention_keys(shell: &Shell, cx: &mut TestAppContext) -> Vec<RowKey> {
    shell.workspace.read_with(cx, |workspace, _| {
        let mut keys: Vec<RowKey> = workspace
            .activity()
            .rows()
            .iter()
            .filter(|row: &&ActivityRow| row.row.attention)
            .map(|row| row.row.key.clone())
            .collect();
        keys.sort();
        keys
    })
}

/// The footer says how many attention rows the log has, the rows are the
/// ones the N-table names, and the popover lists those rows and no others:
/// open while there are any, closed at 0 (P13, the popover half).
fn agree(shell: &Shell, expected: Vec<RowKey>, step: &str, cx: &mut TestAppContext) {
    let mut expected = expected;
    expected.sort();
    assert_eq!(attention_keys(shell, cx), expected, "{step}: the rows");
    let count = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity().attention_count());
    assert_eq!(count, expected.len(), "{step}: the log's count");
    assert_eq!(
        read(shell, Control::Attention, cx),
        attention_text(expected.len()).map(|text| (text, None)),
        "{step}: the footer says the same"
    );
    // The popover is a filtered view of the same rows.
    if expected.is_empty() {
        assert!(!popover::is_open(shell, cx), "{step}: closed at 0");
        assert!(!popover::drawn(shell, cx), "{step}: and gone");
        return;
    }
    if !popover::is_open(shell, cx) {
        popover::open_by_click(shell, cx);
    }
    popover::settle_popover(shell, cx);
    let mut listed = popover::keys_of(&popover::applied(shell, cx));
    listed.sort();
    assert_eq!(listed, expected, "{step}: the popover lists the same rows");
    assert!(popover::drawn(shell, cx), "{step}: and draws them");
    assert!(
        !shell.present(("needs-attention-row", expected.len()), cx),
        "{step}: no more than the rows"
    );
}

/// P13: a journey with a row of each kind the window can make here, then
/// each resolved, with Activity hidden (the popover reads the log, not the
/// panel). At every step the footer's count is the number of the log's
/// attention rows, their keys are the N-table's, and the popover lists them:
/// a failed request (N3), a pending row with a next action (N1's row, counted
/// by itself), and a pending list that failed to read (N13).
#[gpui_kit::test]
fn count_and_popover_agree(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    shell.click(Control::Panel(PanelKind::Activity).id(), cx);
    assert!(!shell.present("operation-area", cx), "Activity is hidden");
    let (a, b) = ("/fixture/a", "/fixture/b");
    let pending = RowKey::Pending {
        root: a.to_owned(),
        operation: OPERATION.to_owned(),
    };
    let request = RowKey::Request(DesktopRequestId(1));
    let list = RowKey::PendingList(b.to_owned());
    // A request that went well is a row of the log and needs no attention:
    // the popover must not list it.
    finish(&shell, 2, Outcome::Success, cx);
    let quiet = RowKey::Request(DesktopRequestId(2));
    let in_log = |shell: &Shell, cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, _| {
            workspace
                .activity()
                .rows()
                .iter()
                .any(|row| row.row.key == quiet && !row.row.attention)
        })
    };
    assert!(in_log(&shell, cx), "a quiet row is in the log");
    agree(&shell, vec![], "nothing", cx);

    finish(&shell, 1, Outcome::Error, cx);
    shell.run(cx);
    agree(&shell, vec![request.clone()], "a failed request (N3)", cx);

    let rows = OperationListDto {
        items: vec![waiting()],
        complete: true,
    };
    read_pending(&shell, a, ReadResult::Ok(rows), cx);
    agree(
        &shell,
        vec![request.clone(), pending.clone()],
        "a pending row with a next action (N12 is not it)",
        cx,
    );

    let failed = ReadFailure {
        code: ResultCode::IndexUnavailable,
        recovery: Vec::new(),
    };
    read_pending(&shell, b, ReadResult::Failed(failed), cx);
    agree(
        &shell,
        vec![request.clone(), pending.clone(), list.clone()],
        "a pending list that failed (N13)",
        cx,
    );

    // Each resolved: the request dismissed, the rows read again as empty.
    shell.operations(cx).update(cx, |operations, cx| {
        operations.dismiss(DesktopRequestId(1));
        cx.notify();
    });
    shell.run(cx);
    agree(
        &shell,
        vec![pending.clone(), list.clone()],
        "the request dismissed (N16)",
        cx,
    );
    let empty = || OperationListDto {
        items: Vec::new(),
        complete: true,
    };
    read_pending(&shell, b, ReadResult::Ok(empty()), cx);
    agree(&shell, vec![pending], "the list read again", cx);
    read_pending(&shell, a, ReadResult::Ok(empty()), cx);
    agree(&shell, vec![], "the pending row gone", cx);
    assert!(
        in_log(&shell, cx),
        "the quiet row was in the log throughout"
    );
}

/// T10: several notifications in one effect cycle make one derivation and one
/// refresh (R-43): the request model's, the drafts' and the scheduler's all
/// notify together, and the log derives once. A request the model then
/// hears of shows its row (its own reads of the lists are later cycles).
#[gpui_kit::test]
fn refreshes_are_coalesced(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    shell.run(cx);
    let derived = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.activity_derivations())
    };
    let before = derived(&shell, cx);
    let (operations, drafts, scheduler) = shell.workspace.read_with(cx, |workspace, _| {
        let services = workspace.services();
        (
            services.operations().clone(),
            services.drafts().clone(),
            services.scheduler().clone(),
        )
    });
    cx.update(|cx| {
        for _ in 0..3 {
            operations.update(cx, |_, cx| cx.notify());
            drafts.update(cx, |_, cx| cx.notify());
            scheduler.update(cx, |_, cx| cx.notify());
        }
    });
    shell.run(cx);
    assert_eq!(
        derived(&shell, cx),
        before + 1,
        "one derivation for the burst"
    );

    finish(&shell, 1, Outcome::Error, cx);
    shell.run(cx);
    assert_eq!(
        attention_keys(&shell, cx),
        vec![RowKey::Request(DesktopRequestId(1))],
        "a request the model heard of shows its row"
    );
}

#[path = "activity_tests/cases.rs"]
pub(in crate::desktop::view) mod cases;
#[path = "activity_tests/clear.rs"]
mod clear;
#[path = "activity_tests/drawn.rs"]
mod drawn_cases;
#[path = "activity_tests/focus.rs"]
mod focus;
#[path = "activity_tests/follow.rs"]
mod follow;
#[path = "activity_tests/layers.rs"]
mod layers;
#[path = "activity_tests/list.rs"]
mod list;
#[path = "activity_tests/list_support.rs"]
pub(in crate::desktop::view) mod list_support;
#[path = "activity_tests/menu.rs"]
mod menu;
#[path = "activity_tests/narrow.rs"]
mod narrow;
#[path = "activity_tests/pending.rs"]
pub(in crate::desktop::view) mod pending_cases;
#[path = "activity_tests/refused.rs"]
mod refused_cases;
#[path = "activity_tests/states.rs"]
mod state_cases;
#[path = "activity_tests/toolbar.rs"]
mod toolbar;

/// P18: each `ActivityAction` kind, `ResumeLifecycle` included, chosen
/// through `choose_activity` from the row that offers it, runs its handler
/// and sends exactly what today's control sends: read from the request
/// model, the recording backend, or the dialog, sheet or page it opens. A
/// choice made against an older revision of its row (T11) and one of a
/// disabled or absent offer (T12) run nothing.
#[gpui_kit::test]
async fn each_action_runs_its_handler(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    // Cancel, Stop, Offer, Recovery, Open and Dismiss of a request.
    cases::requests(cx).await;
    // Review.
    cases::review(cx).await;
    // Resume, ResumeLifecycle, Abandon and RetryPendingList.
    pending_cases::pending(cx).await;
    // ReviewPending.
    pending_cases::review_pending(cx).await;
    // Share, Share main, OpenItem, Retire, Attend and Dismiss of a key.
    state_cases::states(cx).await;
    // Repository.
    state_cases::repository(cx).await;
    // ReviewDrafts, DismissRecovered, DismissNote and RetryFlush.
    state_cases::drafts(cx).await;
    // The stale revision (T11) and the refused offers (T12).
    refused_cases::refused(cx).await;
    // A drawn button and a row's menu (Task 9).
    drawn_cases::drawn(cx).await;
}
