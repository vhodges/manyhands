//! L1's routes, on constructed answers: one test per way out, and the rules the
//! CLI's `human_lifecycle.rs` renders, in its order.

use super::*;
use crate::{
    repository::{
        LifecycleBlockedDto, LifecycleCleanupState, LifecycleIntegrationKind,
        LifecycleMadeReadyState, LifecycleMutationDto, LifecycleMutationType,
        LifecyclePublicationState, LifecycleRecording,
    },
    results::{Effects, RecoveryAction, Scope},
};

use LifecycleWayOut::*;

const ROOT: &str = "/repo";
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";
const OTHER_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY2";
const OID: &str = "0123456789012345678901234567890123456789";

fn data(kind: ItemDtoKind, blockers: Vec<LifecycleBlocker>) -> LifecycleMutationDto {
    let closes = kind == ItemDtoKind::Ticket;
    LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind,
        item_id: ITEM.into(),
        recording: closes.then_some(LifecycleRecording::Record),
        checkpoint_oid: closes.then(|| OID.into()),
        integration: Some(LifecycleIntegrationKind::Merged),
        integration_oid: Some(OID.into()),
        publication: Some(LifecyclePublicationState::Published),
        publication_oid: Some(OID.into()),
        publication_pending: None,
        cleanup: Some(LifecycleCleanupState::Complete),
        cleanup_pending: None,
        blockers,
        made_ready: closes.then(Vec::new),
        made_ready_state: closes.then_some(LifecycleMadeReadyState::Current),
    }
}

fn scope() -> Scope {
    Scope {
        repository: Some(ROOT.into()),
        item_id: Some(ITEM.into()),
        ..Scope::default()
    }
}

fn answer(
    command: &str,
    code: ResultCode,
    data: LifecycleMutationDto,
    recovery: Vec<RecoveryAction>,
) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        command,
        scope(),
        false,
        code,
        Effects::not_requested(),
        false,
    );
    envelope.recovery = recovery;
    envelope.request_id = Some(REQUEST.into());
    envelope.operation_id = Some(OPERATION.into());
    envelope.data = Some(MutationDataDto::Lifecycle(Box::new(data)));
    envelope
}

fn blocked(
    blockers: Vec<LifecycleBlocker>,
    recovery: Vec<RecoveryAction>,
) -> Envelope<PrepareDataDto> {
    let mut envelope = Envelope::<PrepareDataDto>::failure(
        "ticket close",
        scope(),
        ResultCode::RecoveryRequired,
        recovery,
    );
    envelope.data = Some(PrepareDataDto::LifecycleBlocked(LifecycleBlockedDto {
        blockers,
    }));
    envelope
}

fn held(action: RecoveryAction) -> Envelope<PrepareDataDto> {
    let mut envelope = Envelope::<PrepareDataDto>::failure(
        "ticket close",
        scope(),
        ResultCode::RecoveryRequired,
        vec![action],
    );
    envelope.data = None;
    envelope
}

fn retry() -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", REQUEST.into())],
    )
}

fn cancel() -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestCancel,
        [("request_id", REQUEST.into())],
    )
}

fn resume(operation: &str) -> RecoveryAction {
    RecoveryAction::for_operation(RecoveryActionKind::OperationResume, operation, [])
}

fn inspect() -> RecoveryAction {
    RecoveryAction::new(RecoveryActionKind::RepoInspect, [("root", ROOT.into())])
}

fn item() -> String {
    ITEM.into()
}

fn ticket_prepare(blockers: Vec<LifecycleBlocker>) -> Vec<LifecycleWayOut> {
    lifecycle_prepare_way_out(ItemDtoKind::Ticket, &blocked(blockers, vec![inspect()]))
}

#[test]
fn send_again_follows_a_republish_when_nothing_is_cancelled() {
    let envelope = answer(
        "document promote",
        ResultCode::RemoteBranchDeleted,
        data(ItemDtoKind::Document, vec![]),
        vec![],
    );
    assert_eq!(
        lifecycle_way_out(&envelope),
        [Republish { item_id: item() }, SendAgain]
    );
}

#[test]
fn retry_request_resends_the_request_that_holds_the_item() {
    assert_eq!(
        lifecycle_prepare_way_out(ItemDtoKind::Document, &held(retry())),
        [RetryRequest {
            request_id: REQUEST.into()
        }]
    );
}

#[test]
fn cancel_then_send_again_names_the_refused_request() {
    let envelope = answer(
        "document promote",
        ResultCode::RemoteBranchDeleted,
        data(ItemDtoKind::Document, vec![]),
        vec![cancel()],
    );
    assert_eq!(
        lifecycle_way_out(&envelope)[0],
        CancelThenSendAgain {
            request_id: REQUEST.into()
        }
    );
}

/// The action's operation, or the answer's when the action names none.
#[test]
fn resume_operation_goes_on_after_the_repair_under_a_resume() {
    let refused = || {
        data(
            ItemDtoKind::Ticket,
            vec![LifecycleBlocker::IntegrationConflict],
        )
    };
    let envelope = answer(
        "operation resume",
        ResultCode::RecoveryRequired,
        refused(),
        vec![resume(OTHER_OPERATION), inspect()],
    );
    assert_eq!(
        lifecycle_way_out(&envelope),
        [
            RepairLocally { item_id: item() },
            ResumeOperation {
                operation_id: OTHER_OPERATION.into()
            },
            Inspect,
        ]
    );
    let unnamed = RecoveryAction {
        action: RecoveryActionKind::OperationResume,
        operation_id: None,
        arguments: serde_json::Map::new(),
    };
    let envelope = answer(
        "operation resume",
        ResultCode::RecoveryRequired,
        refused(),
        vec![unnamed],
    );
    assert_eq!(
        lifecycle_way_out(&envelope),
        [
            RepairLocally { item_id: item() },
            ResumeOperation {
                operation_id: OPERATION.into()
            },
        ]
    );
}

/// Only a closure recording its own checkpoint, with a way to go on.
#[test]
fn repair_locally_comes_before_the_same_request() {
    let refused = data(
        ItemDtoKind::Ticket,
        vec![LifecycleBlocker::IntegrationConflict],
    );
    let envelope = answer(
        "ticket close",
        ResultCode::RecoveryRequired,
        refused.clone(),
        vec![retry(), inspect()],
    );
    assert_eq!(
        lifecycle_way_out(&envelope),
        [RepairLocally { item_id: item() }, SendAgain, Inspect]
    );
    for (recording, recovery) in [
        (Some(LifecycleRecording::Record), vec![inspect()]),
        (Some(LifecycleRecording::Existing), vec![retry(), inspect()]),
        (None, vec![retry(), inspect()]),
    ] {
        let mut other = refused.clone();
        other.recording = recording;
        let envelope = answer(
            "ticket close",
            ResultCode::RecoveryRequired,
            other,
            recovery,
        );
        assert_eq!(
            lifecycle_way_out(&envelope),
            [RepairOutside, Inspect],
            "{recording:?}"
        );
    }
}

#[test]
fn resolve_child_conflict_names_this_operation_then_the_way_on() {
    let stopped = || {
        data(
            ItemDtoKind::Document,
            vec![
                LifecycleBlocker::ContextConflicted,
                LifecycleBlocker::ConflictPending,
                LifecycleBlocker::ContextBehindRemote,
            ],
        )
    };
    let conflict = || ResolveChildConflict {
        operation_id: OPERATION.into(),
    };
    for (recovery, expected) in [
        (
            vec![retry(), inspect()],
            vec![conflict(), SendAgain, Inspect],
        ),
        (
            vec![resume(OPERATION), inspect()],
            vec![
                conflict(),
                ResumeOperation {
                    operation_id: OPERATION.into(),
                },
                Inspect,
            ],
        ),
        (vec![inspect()], vec![conflict(), Inspect]),
    ] {
        let envelope = answer(
            "document promote",
            ResultCode::RecoveryRequired,
            stopped(),
            recovery,
        );
        assert_eq!(lifecycle_way_out(&envelope), expected);
    }
    // `conflict_pending` alone is the repository's, not this operation's.
    let envelope = answer(
        "document promote",
        ResultCode::RecoveryRequired,
        data(
            ItemDtoKind::Document,
            vec![LifecycleBlocker::ConflictPending],
        ),
        vec![retry(), inspect()],
    );
    assert_eq!(lifecycle_way_out(&envelope), [Inspect]);
}

/// Decision 19: nothing to cancel, since the resume asked already.
#[test]
fn republish_under_a_resume_comes_before_the_resume() {
    let envelope = answer(
        "operation resume",
        ResultCode::RemoteBranchDeleted,
        data(ItemDtoKind::Ticket, vec![]),
        vec![resume(OPERATION), inspect()],
    );
    assert_eq!(
        lifecycle_way_out(&envelope),
        [
            Republish { item_id: item() },
            ResumeOperation {
                operation_id: OPERATION.into()
            },
            Inspect,
        ]
    );
}

#[test]
fn share_primary_follows_the_poll_when_primary_is_ahead() {
    for blockers in [
        vec![LifecycleBlocker::PrimaryAheadOfRemote],
        vec![
            LifecycleBlocker::ContextMissing,
            LifecycleBlocker::PrimaryAheadOfRemote,
        ],
        vec![
            LifecycleBlocker::PrimaryMissing,
            LifecycleBlocker::PrimaryAheadOfRemote,
        ],
    ] {
        assert_eq!(
            ticket_prepare(blockers.clone()),
            [PollOnce, SharePrimary, Inspect],
            "{blockers:?}"
        );
    }
}

#[test]
fn poll_once_reads_a_missing_primary_tracking_ref() {
    for blockers in [
        vec![LifecycleBlocker::PrimaryMissing],
        vec![
            LifecycleBlocker::ContextMissing,
            LifecycleBlocker::PrimaryMissing,
        ],
    ] {
        assert_eq!(ticket_prepare(blockers), [PollOnce, Inspect]);
    }
    // With anything else in the way, the poll alone is no way out.
    assert_eq!(
        ticket_prepare(vec![
            LifecycleBlocker::PrimaryMissing,
            LifecycleBlocker::PrimaryDirty,
        ]),
        [Inspect]
    );
}

#[test]
fn save_item_makes_a_missing_context() {
    assert_eq!(
        ticket_prepare(vec![LifecycleBlocker::ContextMissing]),
        [SaveItem { item_id: item() }, Inspect]
    );
}

/// What the answer offers when no route applies, and nothing when it offers
/// no inspection.
#[test]
fn inspect_is_the_way_out_the_answer_offers_last() {
    let mut stopped = data(
        ItemDtoKind::Document,
        vec![
            LifecycleBlocker::PrimaryDirty,
            LifecycleBlocker::ContextDirty,
        ],
    );
    let envelope = answer(
        "document promote",
        ResultCode::RecoveryRequired,
        stopped.clone(),
        vec![inspect()],
    );
    assert_eq!(lifecycle_way_out(&envelope), [Inspect]);
    stopped.blockers.clear();
    let envelope = answer("document promote", ResultCode::Ok, stopped, vec![]);
    assert!(lifecycle_way_out(&envelope).is_empty());
    assert_eq!(
        ticket_prepare(vec![LifecycleBlocker::IdentityRequired]),
        [Inspect]
    );
    let envelope = blocked(vec![LifecycleBlocker::IdentityRequired], vec![]);
    assert!(lifecycle_prepare_way_out(ItemDtoKind::Ticket, &envelope).is_empty());
}

/// Decision 33: the request sent again, or the resume of an operation no
/// request holds.
#[test]
fn a_held_prepare_names_the_holding_request() {
    assert_eq!(
        lifecycle_prepare_way_out(ItemDtoKind::Ticket, &held(retry())),
        [RetryRequest {
            request_id: REQUEST.into()
        }]
    );
    assert_eq!(
        lifecycle_prepare_way_out(ItemDtoKind::Ticket, &held(resume(OPERATION))),
        [ResumeOperation {
            operation_id: OPERATION.into()
        }]
    );
    // Anything else, or more than the one action, is not held.
    assert!(lifecycle_prepare_way_out(ItemDtoKind::Ticket, &held(inspect())).is_empty());
    let mut two = held(retry());
    two.recovery.push(inspect());
    assert!(lifecycle_prepare_way_out(ItemDtoKind::Ticket, &two).is_empty());
}

/// Decision 18's route: cancel, the same request (`cancelled`), republish,
/// the same request again.
#[test]
fn a_deleted_branch_cancels_then_sends_again() {
    for kind in [ItemDtoKind::Ticket, ItemDtoKind::Document] {
        let envelope = answer(
            "ticket close",
            ResultCode::RemoteBranchDeleted,
            data(kind, vec![]),
            vec![cancel(), inspect()],
        );
        assert_eq!(
            lifecycle_way_out(&envelope),
            [
                CancelThenSendAgain {
                    request_id: REQUEST.into()
                },
                Republish { item_id: item() },
                SendAgain,
                Inspect,
            ]
        );
    }
    // No request to send again: no route.
    let mut envelope = answer(
        "ticket close",
        ResultCode::RemoteBranchDeleted,
        data(ItemDtoKind::Ticket, vec![]),
        vec![cancel(), inspect()],
    );
    envelope.request_id = None;
    assert_eq!(lifecycle_way_out(&envelope), [Inspect]);
}

/// The CLI's `route_of` leaves a promotion's refused merge without a route:
/// `sync-local` admits only a closure recording its own checkpoint.
#[test]
fn an_execute_result_for_a_documents_integration_conflict_is_repair_outside() {
    let envelope = answer(
        "document promote",
        ResultCode::RecoveryRequired,
        data(
            ItemDtoKind::Document,
            vec![LifecycleBlocker::IntegrationConflict],
        ),
        vec![retry(), inspect()],
    );
    assert_eq!(lifecycle_way_out(&envelope), [RepairOutside, Inspect]);
    // A document's child conflict takes its own route first.
    let envelope = answer(
        "document promote",
        ResultCode::RecoveryRequired,
        data(
            ItemDtoKind::Document,
            vec![
                LifecycleBlocker::IntegrationConflict,
                LifecycleBlocker::ContextConflicted,
                LifecycleBlocker::ConflictPending,
            ],
        ),
        vec![retry()],
    );
    assert_eq!(
        lifecycle_way_out(&envelope),
        [
            ResolveChildConflict {
                operation_id: OPERATION.into()
            },
            SendAgain
        ]
    );
}

#[test]
fn a_blocked_prepares_integration_conflict_is_repair_locally_for_either_kind() {
    for kind in [ItemDtoKind::Ticket, ItemDtoKind::Document] {
        let envelope = blocked(vec![LifecycleBlocker::IntegrationConflict], vec![inspect()]);
        assert_eq!(
            lifecycle_prepare_way_out(kind, &envelope),
            [RepairLocally { item_id: item() }, Inspect]
        );
        // `sync-local` refuses a dirty or conflicted context, and a pending
        // conflict or local operation has its own way on.
        for other in [
            LifecycleBlocker::ContextDirty,
            LifecycleBlocker::ContextConflicted,
            LifecycleBlocker::ConflictPending,
            LifecycleBlocker::LocalOperationPending,
        ] {
            let envelope = blocked(
                vec![LifecycleBlocker::IntegrationConflict, other],
                vec![inspect()],
            );
            assert_eq!(lifecycle_prepare_way_out(kind, &envelope), [Inspect]);
        }
    }
}

/// The kind is the item's, which a blocked answer doesn't hold; the save
/// carries the item's ID for both, and a front end words it by the kind.
#[test]
fn save_item_routes_the_same_for_either_kind() {
    for kind in [ItemDtoKind::Ticket, ItemDtoKind::Document] {
        let envelope = blocked(vec![LifecycleBlocker::ContextMissing], vec![]);
        assert_eq!(
            lifecycle_prepare_way_out(kind, &envelope),
            [SaveItem { item_id: item() }],
            "{kind:?}"
        );
    }
    for blockers in [
        vec![LifecycleBlocker::ContextMissing],
        vec![LifecycleBlocker::IntegrationConflict],
        vec![LifecycleBlocker::HistoryUnknown],
        vec![LifecycleBlocker::PrimaryAheadOfRemote],
    ] {
        let envelope = blocked(blockers.clone(), vec![inspect()]);
        assert_eq!(
            lifecycle_prepare_way_out(ItemDtoKind::Ticket, &envelope),
            lifecycle_prepare_way_out(ItemDtoKind::Document, &envelope),
            "{blockers:?}"
        );
    }
}

/// Ruling R-13: a closure's refused merge that `sync-local` can't take, or
/// that has no way on, is repaired outside, as a promotion's is.
#[test]
fn a_tickets_unrouted_integration_conflict_is_repair_outside() {
    for (recording, recovery) in [
        (Some(LifecycleRecording::Existing), vec![retry(), inspect()]),
        (None, vec![retry(), inspect()]),
        (Some(LifecycleRecording::Record), vec![inspect()]),
    ] {
        let mut refused = data(
            ItemDtoKind::Ticket,
            vec![LifecycleBlocker::IntegrationConflict],
        );
        refused.recording = recording;
        let envelope = answer(
            "ticket close",
            ResultCode::RecoveryRequired,
            refused,
            recovery,
        );
        assert_eq!(
            lifecycle_way_out(&envelope),
            [RepairOutside, Inspect],
            "{recording:?}"
        );
    }
    // Not for another code, nor without the blocker.
    let envelope = answer(
        "ticket close",
        ResultCode::TransportUnavailable,
        data(
            ItemDtoKind::Ticket,
            vec![LifecycleBlocker::IntegrationConflict],
        ),
        vec![retry()],
    );
    assert!(lifecycle_way_out(&envelope).is_empty());
}

/// C5 decision 35: a published context whose remote state is unknown is read
/// by a poll, like a missing primary tracking ref.
#[test]
fn an_unknown_remote_context_polls_once() {
    for blockers in [
        vec![LifecycleBlocker::HistoryUnknown],
        vec![
            LifecycleBlocker::HistoryUnknown,
            LifecycleBlocker::ContextMissing,
        ],
        vec![
            LifecycleBlocker::HistoryUnknown,
            LifecycleBlocker::PrimaryMissing,
        ],
    ] {
        assert_eq!(ticket_prepare(blockers), [PollOnce, Inspect]);
    }
    assert_eq!(
        ticket_prepare(vec![
            LifecycleBlocker::HistoryUnknown,
            LifecycleBlocker::PrimaryAheadOfRemote,
        ]),
        [PollOnce, SharePrimary, Inspect]
    );
}

/// The first rule that applies wins, in the CLI's order: the poll before the
/// save, the save only alone, the repair after both; at execute, a closure's
/// refused merge before a child's conflict.
#[test]
fn mixed_blockers_keep_the_cli_order() {
    use LifecycleBlocker::{
        ConflictPending, ContextConflicted, ContextMissing, IntegrationConflict, PrimaryMissing,
    };
    for (blockers, expected) in [
        (
            vec![ContextMissing, PrimaryMissing],
            vec![PollOnce, Inspect],
        ),
        (
            vec![ContextMissing, IntegrationConflict],
            vec![RepairLocally { item_id: item() }, Inspect],
        ),
        (
            vec![PrimaryMissing, IntegrationConflict],
            vec![RepairLocally { item_id: item() }, Inspect],
        ),
        (
            vec![ContextMissing, ContextConflicted, IntegrationConflict],
            vec![Inspect],
        ),
    ] {
        assert_eq!(ticket_prepare(blockers.clone()), expected, "{blockers:?}");
    }
    let both = data(
        ItemDtoKind::Ticket,
        vec![IntegrationConflict, ContextConflicted, ConflictPending],
    );
    let envelope = answer(
        "ticket close",
        ResultCode::RecoveryRequired,
        both.clone(),
        vec![retry(), inspect()],
    );
    assert_eq!(
        lifecycle_way_out(&envelope),
        [RepairLocally { item_id: item() }, SendAgain, Inspect]
    );
    // The closure's rule applies with no way on, and the conflict's isn't
    // tried after it: the merge is repaired outside (ruling R-13).
    let envelope = answer(
        "ticket close",
        ResultCode::RecoveryRequired,
        both,
        vec![inspect()],
    );
    assert_eq!(lifecycle_way_out(&envelope), [RepairOutside, Inspect]);
}
