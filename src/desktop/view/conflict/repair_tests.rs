//! Repair locally… in the lifecycle dialog (D5 design, B6; L1's
//! `RepairLocally`; P9, P16, A13), over a real close: one stopped after its
//! checkpoint (`FailurePoint::AfterClosureCheckpoint`, as
//! `tests/cli/lifecycle.rs` `Refused` strands one), whose merge main then
//! refuses. Its local synchronization's conflict, its resolution and the
//! close's Retry are real too.

use std::path::Path;

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    repository::{AuthoringKind, FailurePoint, OperationAction, OperationId, RepositoryService},
    results::ResultCode,
};

use super::REVIEW_CHANGES;
use super::{
    ResolveSheet, SheetSource,
    entry_tests::{opened_on, row_of, ticket_shell},
    tests::{TICKET, code, opened, until_long, until_present},
};
use crate::desktop::{
    model::activity::rows::RowKey,
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::fixtures::item_id,
    view::{
        activity_support::{act, offered, refresh, row, until_offered},
        harness::Shell,
        item::LIFECYCLE_ACTION,
        lifecycle::REPAIR_LOCALLY,
        operations::{OPEN_LIFECYCLE, RESUME},
    },
};

/// What the dialog says once main is in a ticket's working copy (B6).
/// The primary branch is named as the repository names it (R-35a).
const REPAIRED_TICKET: &str = "The main branch is in this ticket's working copy now.";

/// The sheet once it has opened, past a real synchronization's wait.
async fn until_opened(shell: &Shell, cx: &mut TestAppContext) -> Entity<ResolveSheet> {
    until_long(shell, "the sheet's opening", cx, |shell, cx| {
        shell.present("conflict-sheet", cx)
    })
    .await;
    opened(cx).expect("the sheet is open")
}

fn closes(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketClose { .. })
    })
}

fn local_syncs(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::LocalContextSync { .. })
    })
}

/// The local synchronization `root` lists pending, as the library does.
fn listed_local_sync(shell: &Shell, root: &Path) -> OperationId {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let row = service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .find(|row| row.action == OperationAction::ContextSyncLocal)
        .expect("a local synchronization is pending");
    OperationId::parse(row.operation_id.as_deref().unwrap()).unwrap()
}

/// P9, real: the close stops after its checkpoint, main then moves on the
/// ticket, and Retry is refused for the merge, offering Repair locally….
/// Gives that refused close's request.
async fn to_the_repair(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> DesktopRequestId {
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterClosureCheckpoint,
        )
        .unwrap(),
    );
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(shell, "lifecycle-confirm", cx).await;
    shell.click("lifecycle-confirm", cx);
    let first = closes(shell, cx)[0];
    shell.until_ended(first, cx).await;
    assert_ne!(code(shell, first, cx), Some(ResultCode::Ok), "stopped");
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    shell.fixture.commit_ticket(root, TICKET, "Moved", "");

    until_present(shell, "lifecycle-retry", cx).await;
    shell.click("lifecycle-retry", cx);
    let refused = closes(shell, cx)[0];
    shell.until_ended(refused, cx).await;
    assert_eq!(code(shell, refused, cx), Some(ResultCode::RecoveryRequired));
    until_present(shell, "lifecycle-repair-locally", cx).await;
    assert_eq!(
        shell.label("lifecycle-repair-locally", cx).as_deref(),
        Some(REPAIR_LOCALLY)
    );
    refused
}

/// P9, A13, real: Repair locally… on a close whose merge main refused
/// observes the work context and sends `context sync-local` with that
/// observation; its conflict opens the sheet on the synchronization, its
/// folder the work context. Resolved there, the dialog opens again on its
/// way on (B6): main is in the working copy, and Retry finishes the same
/// close.
#[gpui_kit::test]
async fn repair_locally_syncs_and_opens_the_sheet_on_conflict(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let close = to_the_repair(&shell, &root, cx).await;
    shell.click("lifecycle-repair-locally", cx);
    let sheet = until_opened(&shell, cx).await;
    let syncs = local_syncs(&shell, cx);
    assert_eq!(syncs.len(), 1, "one synchronization sent");
    let sync = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(syncs[0]).cloned())
        .unwrap();
    assert!(matches!(
        &sync.intent,
        Intent::LocalContextSync { root: asked, kind: AuthoringKind::Ticket, id, observation }
            if *asked == root && *id == item_id(TICKET) && !observation.is_empty()
    ));
    let worktree = shell.fixture.context_worktree(&root, TICKET);
    assert_eq!(
        opened_on(&sheet, cx),
        (
            SheetSource::Local,
            listed_local_sync(&shell, &root),
            worktree
        )
    );
    assert!(
        !shell.present("lifecycle-dialog", cx),
        "the sheet took its place"
    );

    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell.click("conflict-save", cx);
    until_long(&shell, "the resolution's answer", cx, |shell, cx| {
        !shell.present("conflict-sheet", cx)
    })
    .await;

    // Re-review 3: the applied local resolution clears the
    // synchronization's entry's Review changes.
    let entry = RowKey::Request(syncs[0]);
    refresh(&shell, cx);
    assert!(row(&shell, &entry, cx).is_some(), "the entry stays listed");
    assert!(offered(&shell, &entry, REVIEW_CHANGES, cx).is_none());

    // Then Retry (P9): the dialog again, on the same close.
    until_present(&shell, "lifecycle-retry", cx).await;
    assert_eq!(
        shell.label("lifecycle-repair", cx).as_deref(),
        Some(REPAIRED_TICKET)
    );
    shell.click("lifecycle-retry", cx);
    let retried = closes(&shell, cx)[0];
    assert_ne!(retried, close);
    until_long(&shell, "the close's answer", cx, |shell, cx| {
        code(shell, retried, cx).is_some()
    })
    .await;
    assert_eq!(code(&shell, retried, cx), Some(ResultCode::Ok));
}

/// P16, real: Repair locally… pressed again while the synchronization it
/// sent waits on its conflict is refused at its observation, which sends
/// nothing; the dialog shows the answer. The synchronization's pending row
/// offers Resume and Review changes (the close's row, with no conflict of
/// its own, offers no Review changes), and Resume continues it, to its
/// conflict again.
#[gpui_kit::test]
async fn a_refused_local_sync_shows_its_answer_and_resume(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let close = to_the_repair(&shell, &root, cx).await;
    shell.click("lifecycle-repair-locally", cx);
    until_opened(&shell, cx).await;
    shell.keys("escape", cx);
    assert!(!shell.present("conflict-sheet", cx));

    act(&shell, &RowKey::Request(close), OPEN_LIFECYCLE, cx);
    until_present(&shell, "lifecycle-repair-locally", cx).await;
    shell.click("lifecycle-repair-locally", cx);
    until_present(&shell, "lifecycle-repair-code", cx).await;
    assert_eq!(
        shell.label("lifecycle-repair-code", cx).as_deref(),
        Some(ResultCode::WorktreeConflicted.as_str())
    );
    assert_eq!(
        shell.label("lifecycle-repair", cx).as_deref(),
        Some(ResultCode::WorktreeConflicted.message())
    );
    assert_eq!(local_syncs(&shell, cx).len(), 1, "nothing more was sent");
    shell.keys("escape", cx);

    let operation = listed_local_sync(&shell, &root);
    let pending = row_of(&shell, &root, "context_sync_local", cx).await;
    until_offered(&shell, &pending, REVIEW_CHANGES, cx).await;
    assert!(offered(&shell, &pending, RESUME, cx).is_some_and(|offer| offer.enabled));
    let close_row = row_of(&shell, &root, "close", cx).await;
    refresh(&shell, cx);
    assert!(row(&shell, &close_row, cx).is_some(), "the close's row");
    assert!(offered(&shell, &close_row, REVIEW_CHANGES, cx).is_none());

    act(&shell, &pending, RESUME, cx);
    let resumed = shell.requests(cx, move |request| {
        matches!(request.intent, Intent::OperationResume { operation_id, .. }
            if operation_id == operation)
    });
    assert_eq!(resumed.len(), 1, "Resume sent its operation");
    shell.until_ended(resumed[0], cx).await;
    assert_eq!(
        code(&shell, resumed[0], cx),
        Some(ResultCode::RecoveryRequired),
        "still conflicted"
    );
    // M3: the resume stopped on the conflict, whose entry offers it.
    let stopped = RowKey::Request(resumed[0]);
    until_offered(&shell, &stopped, REVIEW_CHANGES, cx).await;
    act(&shell, &stopped, REVIEW_CHANGES, cx);
    let sheet = until_opened(&shell, cx).await;
    assert_eq!(
        opened_on(&sheet, cx).0,
        SheetSource::Local,
        "the resume's own conflict"
    );
}

/// B6, real: a prepare blocked because main conflicts with the ticket
/// (`a_blocked_prepare_offers_its_way_out` in `tests/cli/lifecycle.rs`)
/// offers Repair locally… beside its guidance; pressed, its conflict opens
/// the sheet in the dialog's place, on the local synchronization. Resolved
/// there, the dialog opens again saying main is in the working copy, with
/// Prepare again, which prepares the close afresh (B6).
#[gpui_kit::test]
async fn a_blocked_prepare_offers_repair_locally(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    shell.fixture.commit_ticket(&root, TICKET, "Moved", "");
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-repair-locally", cx).await;
    assert!(shell.present("lifecycle-prepare-again", cx));
    assert!(!shell.rows("lifecycle-way-out", cx).is_empty(), "guidance");

    shell.click("lifecycle-repair-locally", cx);
    let sheet = until_opened(&shell, cx).await;
    assert!(!shell.present("lifecycle-dialog", cx));
    assert_eq!(
        opened_on(&sheet, cx),
        (
            SheetSource::Local,
            listed_local_sync(&shell, &root),
            shell.fixture.context_worktree(&root, TICKET)
        )
    );

    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell.click("conflict-save", cx);
    until_long(&shell, "the dialog again", cx, |shell, cx| {
        shell.present("lifecycle-dialog", cx)
    })
    .await;
    assert_eq!(
        shell.label("lifecycle-repair", cx).as_deref(),
        Some(REPAIRED_TICKET)
    );
    assert!(!shell.present("lifecycle-retry", cx));
    shell.click("lifecycle-prepare-again", cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
}

/// M1 (P22), real: Repair locally… is not offered while a local
/// synchronization of the item is queued or running. The dialog that sent
/// it closed and opened again shows none; once the synchronization stops
/// on its conflict, its entry offers Review changes.
#[gpui_kit::test]
async fn repair_locally_waits_for_a_queued_sync(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    shell.fixture.commit_ticket(&root, TICKET, "Moved", "");
    let held = shell.backend.hold_job(JobKind::LocalContextSync);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-repair-locally", cx).await;
    shell.click("lifecycle-repair-locally", cx);
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::LocalContextSync { .. }))
        })
        .await;
    held.wait_reached();
    assert!(!shell.present("lifecycle-repair-locally", cx), "under way");
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-prepare-again", cx).await;
    assert!(
        !shell.present("lifecycle-repair-locally", cx),
        "a synchronization of the item is running"
    );
    shell.keys("escape", cx);

    held.release();
    let sync = local_syncs(&shell, cx)[0];
    until_long(&shell, "the synchronization's answer", cx, |shell, cx| {
        code(shell, sync, cx).is_some()
    })
    .await;
    let stopped = RowKey::Request(sync);
    until_offered(&shell, &stopped, REVIEW_CHANGES, cx).await;
    act(&shell, &stopped, REVIEW_CHANGES, cx);
    let sheet = until_opened(&shell, cx).await;
    assert_eq!(
        opened_on(&sheet, cx),
        (
            SheetSource::Local,
            listed_local_sync(&shell, &root),
            shell.fixture.context_worktree(&root, TICKET)
        )
    );
}

/// Re-review 5 (P22), real, with the close held: a blocked prepare's
/// Repair locally… is not offered while a close of the item is in flight,
/// as an ended run's is not.
#[gpui_kit::test]
async fn a_blocked_prepares_repair_waits_for_the_lifecycle(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    shell.fixture.commit_ticket(&root, TICKET, "Moved", "");
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-repair-locally", cx).await;

    let held = shell.backend.hold_job(JobKind::TicketClose);
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    services.worker().submit(
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        Lane::User,
        None,
        None,
    );
    held.wait_reached();
    shell
        .until_drawn(cx, "Repair locally… withdrawn", |shell, cx| {
            !shell.present("lifecycle-repair-locally", cx)
        })
        .await;
    assert!(shell.present("lifecycle-prepare-again", cx));
    held.release();
    let close = closes(&shell, cx)[0];
    shell.until_ended(close, cx).await;
}
