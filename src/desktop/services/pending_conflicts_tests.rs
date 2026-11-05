//! Which pending rows offer Review changes (D5 B5; Task 13 review I-2).
//! The rows and reads are canned (decision 9): a lifecycle's parent row and
//! its context child's row are listed only beside a remote, which desktop
//! tests do not reach. Their shape is
//! `tests/mutation_remote/c5_child_conflict.rs` `shown_by_either_id`'s:
//! `show_conflict` of the parent reads the child's conflict, naming the
//! child's operation.

use manyhands::repository::{
    OperationAction, OperationDto, OperationFamily, OperationId, OperationListDto,
    OperationNextAction, OperationOwner,
};

use super::{Candidate, PendingSource, Probe, candidates, offered};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH0";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";
const CHILD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY2";
const LOCAL: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY3";

fn row(id: &str, family: OperationFamily, action: OperationAction, item: &str) -> OperationDto {
    OperationDto {
        operation_id: Some(id.to_owned()),
        family,
        owner: OperationOwner::Repository,
        action,
        state: "failed".to_owned(),
        completed_step: None,
        next_action: Some(OperationNextAction::Resume),
        item_id: Some(item.to_owned()),
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    }
}

fn operation(id: &str) -> OperationId {
    OperationId::parse(id).unwrap()
}

/// A closure's parent and its context child, of `TICKET`, and a local
/// synchronization of `OTHER`.
fn listed() -> Vec<Candidate> {
    let list = OperationListDto {
        items: vec![
            row(
                CHILD,
                OperationFamily::Remote,
                OperationAction::SynchronizeContext,
                TICKET,
            ),
            row(
                PARENT,
                OperationFamily::Remote,
                OperationAction::Close,
                TICKET,
            ),
            row(
                LOCAL,
                OperationFamily::Local,
                OperationAction::ContextSyncLocal,
                OTHER,
            ),
            row(
                "01ARZ3NDEKTSV4RRFFQ69G5FY4",
                OperationFamily::Local,
                OperationAction::SaveTicket,
                OTHER,
            ),
        ],
        complete: true,
    };
    let listed = candidates(&list);
    assert_eq!(
        listed
            .iter()
            .map(|candidate| candidate.source)
            .collect::<Vec<_>>(),
        [
            PendingSource::Synchronization,
            PendingSource::Close,
            PendingSource::Local
        ]
    );
    listed
}

/// I-2: the child's row is not offered beside its parent's, whose read
/// names the child's operation; the parent's and a local synchronization's
/// are.
#[test]
fn a_lifecycle_childs_row_is_offered_on_its_parent_only() {
    let listed = listed();
    let probes = [
        Probe::Waits(operation(CHILD)),
        Probe::Waits(operation(CHILD)),
        Probe::Waits(operation(LOCAL)),
    ];
    assert_eq!(offered(&listed, &probes), [1, 2]);
}

/// I-2's fallback: a parent of the same item that could not be read
/// leaves the child's row out too; with no parent waiting on it, or a
/// parent of a settled conflict, the child's row is offered.
#[test]
fn a_child_beside_an_unread_parent_is_left_out_by_its_item() {
    let listed = listed();
    let waits = Probe::Waits(operation(CHILD));
    assert_eq!(
        offered(
            &listed,
            &[waits.clone(), Probe::Unread, Probe::Waits(operation(LOCAL))]
        ),
        [2]
    );
    assert_eq!(
        offered(&listed, &[waits, Probe::Settled, Probe::Settled]),
        [0]
    );
}
