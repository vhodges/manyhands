//! The pending-operation rows of `each_action_runs_its_handler` (P18).

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        OperationAction, OperationDto, OperationFamily, OperationListDto, OperationNextAction,
        OperationOwner,
    },
    results::ResultCode,
};

use super::cases::{OPERATION, TICKET, dialog_open, sent};
use crate::desktop::{
    model::{
        activity::{copy::REVIEW_CHANGES, rows::RowKey},
        operations::Operations,
    },
    services::{
        ReadKind, ReadResult,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::Fixture,
    view::{
        activity_support::{act, pending_key},
        harness::{Shell, key},
    },
};

const CLOSING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

fn waiting(action: OperationAction, id: &str, item: Option<&str>) -> OperationDto {
    OperationDto {
        operation_id: Some(id.to_owned()),
        family: OperationFamily::Local,
        owner: OperationOwner::Repository,
        action,
        state: "prepared".to_owned(),
        completed_step: None,
        next_action: Some(OperationNextAction::Resume),
        item_id: item.map(str::to_owned),
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

fn listing(items: Vec<OperationDto>) -> ReadResult<OperationListDto> {
    ReadResult::Ok(OperationListDto {
        items,
        complete: true,
    })
}

/// A window over no repository whose pending list for `root` was read as
/// `operations`. Each action below gets its own: an answer that comes back
/// for a repository that is not there reads its list again.
fn pending_shell(operations: Vec<OperationDto>, cx: &mut TestAppContext) -> Shell {
    let shell = Shell::open(Fixture::new(), cx);
    read_pending(&shell, PENDING_ROOT, listing(operations), cx);
    shell
}

const PENDING_ROOT: &str = "/fixture/a";

/// The `operation resume` requests of `operation`.
fn resumes(shell: &Shell, operation: &str, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    sent(shell, cx, |intent| {
        matches!(intent, Intent::OperationResume { operation_id, .. }
            if operation_id.to_string() == operation)
    })
}

/// Abandon, Retry of a list, Resume, and a lifecycle's Resume.
pub(super) async fn pending(cx: &mut TestAppContext) {
    let save = || vec![waiting(OperationAction::SaveTicket, OPERATION, None)];
    let key_of = |shell: &Shell, action: OperationAction, cx: &mut TestAppContext| {
        pending_key(shell, PENDING_ROOT, action.as_str(), cx)
    };

    // Abandon asks first, and sends nothing until confirmed.
    let shell = pending_shell(save(), cx);
    let row = key_of(&shell, OperationAction::SaveTicket, cx);
    act(&shell, &row, "Abandon", cx);
    assert!(dialog_open(&shell, cx));
    assert!(
        shell.present("confirm-dialog", cx),
        "the confirmation dialog"
    );
    shell.keys("escape", cx);
    assert!(sent(&shell, cx, |_| true).is_empty());

    // Resume sends `operation resume` of the operation.
    let shell = pending_shell(save(), cx);
    let row = key_of(&shell, OperationAction::SaveTicket, cx);
    act(&shell, &row, "Resume", cx);
    assert_eq!(resumes(&shell, OPERATION, cx).len(), 1);

    // A list that failed to read offers Retry, which reads it again.
    let shell = Shell::open(Fixture::new(), cx);
    let unreadable = "/fixture/b";
    let failed = crate::desktop::services::ReadFailure {
        code: ResultCode::IndexUnavailable,
        recovery: Vec::new(),
    };
    read_pending(&shell, unreadable, ReadResult::Failed(failed), cx);
    let reads = shell.backend.reads_seen(ReadKind::Operations);
    act(
        &shell,
        &RowKey::PendingList(unreadable.to_owned()),
        "Retry",
        cx,
    );
    assert!(
        shell.backend.reads_seen(ReadKind::Operations) > reads,
        "the list was read again"
    );

    // A lifecycle's Resume sends the same, and opens its dialog on it.
    let shell = pending_shell(
        vec![waiting(OperationAction::Close, CLOSING, Some(TICKET))],
        cx,
    );
    let row = key_of(&shell, OperationAction::Close, cx);
    act(&shell, &row, "Resume", cx);
    assert_eq!(resumes(&shell, CLOSING, cx).len(), 1);
    assert!(dialog_open(&shell, cx), "the lifecycle dialog follows it");
    shell.keys("escape", cx);
}

/// Review changes on a pending row whose conflict waits to be resolved.
pub(super) async fn review_pending(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let shell = Shell::open(fixture, cx);
    let root = key(&root);
    let row = pending_key(&shell, &root, "context_sync_local", cx);
    assert!(!shell.present("conflict-sheet", cx));
    act(&shell, &row, REVIEW_CHANGES, cx);
    shell
        .until_drawn(cx, "the sheet", |shell, cx| {
            shell.present("conflict-sheet", cx)
        })
        .await;
}
