//! The lifecycle lock's derivation (D5 design, A2; R-21, R-22), apart from
//! any view: the steps that lock, the rows a read leaves, and L-4's
//! exemption. The rows and answers are built from library types.

use gpui_kit::EntityId;
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleCleanupPendingReason, LifecycleCleanupState, LifecycleMutationDto,
        LifecycleMutationType, OperationAction, OperationDto, OperationFamily, OperationListDto,
        OperationNextAction, OperationOwner,
    },
    results::ResultCode,
};

use super::{LifecycleLocks, LockKey, LockSource, LockState, Untidied, step_locks};
use crate::desktop::services::{ReadFailure, ReadResult, requests::DesktopRequestId};

const ROOT: &str = "/repo";
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5KC0";
const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";

fn key() -> LockKey {
    LockKey::new(ROOT, ITEM)
}

/// A closure's row at `step`, last updated at `updated`.
fn row(step: &str, updated: &str) -> OperationDto {
    OperationDto {
        operation_id: Some(OPERATION.to_owned()),
        family: OperationFamily::Remote,
        owner: OperationOwner::Repository,
        action: OperationAction::Close,
        state: step.to_owned(),
        completed_step: Some(step.to_owned()),
        next_action: Some(OperationNextAction::Resume),
        item_id: Some(ITEM.to_owned()),
        key_id: None,
        worktree: None,
        updated_at: Some(updated.to_owned()),
        failure_code: None,
    }
}

fn read(rows: Vec<OperationDto>) -> ReadResult<OperationListDto> {
    ReadResult::Ok(OperationListDto {
        items: rows,
        complete: true,
    })
}

fn state(locks: &LifecycleLocks) -> LockState {
    locks.state(&key(), false, false)
}

/// A lifecycle answer whose tidy-up is pending for `reason`.
fn pending(reason: Option<LifecycleCleanupPendingReason>) -> LifecycleMutationDto {
    LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind: ItemDtoKind::Ticket,
        item_id: ITEM.to_owned(),
        recording: None,
        checkpoint_oid: None,
        integration: None,
        integration_oid: None,
        publication: None,
        publication_oid: None,
        publication_pending: None,
        cleanup: reason.map(|_| LifecycleCleanupState::Pending),
        cleanup_pending: reason,
        blockers: Vec::new(),
        made_ready: None,
        made_ready_state: None,
    }
}

/// R-21: L-1's steps leave the item editable; every other listed step,
/// closure `closing` and the tidy-up steps included, and a step the
/// desktop does not know, lock it.
#[test]
fn every_step_from_closing_or_integrating_on_locks() {
    for step in ["confirmed", "context_synchronized"] {
        assert!(!step_locks(step), "{step}");
    }
    for step in [
        "closing",
        "integrating",
        "integrated",
        "published",
        "publication_pending",
        "remote_deleted",
        "worktree_removing",
        "worktree_removed",
        "branch_deleted",
        "a_step_not_known_yet",
    ] {
        assert!(step_locks(step), "{step}");
    }
}

/// The record source, from a read; a busy read keeps the rows, a failed
/// one clears them (R-21).
#[test]
fn a_busy_read_keeps_the_rows_and_a_failed_one_clears_them() {
    let mut locks = LifecycleLocks::default();
    assert_eq!(state(&locks), LockState::Open, "nothing read");
    assert_eq!(locks.state(&key(), false, true), LockState::Awaiting);
    locks.rows_read(ROOT, &read(vec![row("integrated", "t1")]));
    assert_eq!(state(&locks), LockState::Locked(LockSource::Record));
    assert_eq!(locks.operations(&key()), [OPERATION]);
    locks.rows_read(ROOT, &ReadResult::Busy);
    assert_eq!(state(&locks), LockState::Locked(LockSource::Record));
    locks.rows_read(
        ROOT,
        &ReadResult::Failed(ReadFailure {
            code: ResultCode::RepositoryNotRegistered,
            recovery: Vec::new(),
        }),
    );
    assert_eq!(state(&locks), LockState::Open);
    locks.rows_read(ROOT, &read(vec![row("context_synchronized", "t2")]));
    assert_eq!(state(&locks), LockState::Open, "L-1");
}

/// The dialog and the worker lock whatever the rows say; a lifecycle
/// answer awaits its rows.
#[test]
fn the_dialog_and_the_worker_lock_and_an_answer_awaits_its_rows() {
    let mut locks = LifecycleLocks::default();
    locks.rows_read(ROOT, &read(Vec::new()));
    let dialog = EntityId::from(7u64);
    assert!(locks.set_dialog(dialog, key(), true));
    assert_eq!(state(&locks), LockState::Locked(LockSource::Dialog));
    assert!(locks.set_dialog(dialog, key(), false));
    assert_eq!(state(&locks), LockState::Open);
    assert_eq!(
        locks.state(&key(), true, false),
        LockState::Locked(LockSource::Worker)
    );
    locks.answered(key(), DesktopRequestId(1), None);
    assert_eq!(state(&locks), LockState::Awaiting);
    locks.rows_read(ROOT, &read(Vec::new()));
    assert_eq!(state(&locks), LockState::Open, "L-5");
}

/// L-4 and R-22: a tidy-up refused for a change the person can undo
/// exempts the row, until a later answer or the row's update; a refused
/// remote deletion does not (L-3).
#[test]
fn an_undoable_tidy_up_stop_exempts_its_row_until_it_changes() {
    let request = DesktopRequestId(4);
    for reason in [
        LifecycleCleanupPendingReason::ContextChanged,
        LifecycleCleanupPendingReason::IgnoredFiles,
        LifecycleCleanupPendingReason::RemoteContextChanged,
    ] {
        let mut locks = LifecycleLocks::default();
        locks.answered(key(), request, Some(&pending(Some(reason))));
        locks.rows_read(ROOT, &read(vec![row("publication_pending", "t1")]));
        let untidied = LockState::Untidied(Untidied { reason, request });
        assert_eq!(state(&locks), untidied, "{reason:?}");
        locks.rows_read(ROOT, &read(vec![row("publication_pending", "t1")]));
        assert_eq!(state(&locks), untidied, "the same row");
        locks.rows_read(ROOT, &read(vec![row("publication_pending", "t2")]));
        assert_eq!(
            state(&locks),
            LockState::Locked(LockSource::Record),
            "the row changed outside: {reason:?}"
        );
    }
    let mut locks = LifecycleLocks::default();
    locks.answered(
        key(),
        request,
        Some(&pending(Some(
            LifecycleCleanupPendingReason::RemoteDeletionRejected,
        ))),
    );
    locks.rows_read(ROOT, &read(vec![row("published", "t1")]));
    assert_eq!(state(&locks), LockState::Locked(LockSource::Record));

    let mut locks = LifecycleLocks::default();
    let changed = Some(LifecycleCleanupPendingReason::ContextChanged);
    locks.answered(key(), request, Some(&pending(changed)));
    locks.rows_read(ROOT, &read(vec![row("publication_pending", "t1")]));
    locks.answered(key(), request, Some(&pending(None)));
    locks.rows_read(ROOT, &read(vec![row("publication_pending", "t1")]));
    assert_eq!(
        state(&locks),
        LockState::Locked(LockSource::Record),
        "a later answer"
    );
}

/// M-5: any of the item's rows in a locking step locks it, whatever order
/// the read lists them in.
#[test]
fn any_locking_row_of_the_item_locks_it() {
    let mut locks = LifecycleLocks::default();
    locks.rows_read(
        ROOT,
        &read(vec![
            row("context_synchronized", "t1"),
            row("closing", "t1"),
        ]),
    );
    assert_eq!(state(&locks), LockState::Locked(LockSource::Record));
}

/// M-6: a repository no longer listed is forgotten: listed again, it
/// awaits its first read.
#[test]
fn a_forgotten_repository_awaits_its_first_read_again() {
    let mut locks = LifecycleLocks::default();
    locks.rows_read(ROOT, &read(Vec::new()));
    locks.forget(ROOT);
    assert!(!locks.read(ROOT));
    assert_eq!(locks.state(&key(), false, true), LockState::Awaiting);
}
