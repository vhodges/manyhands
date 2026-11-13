//! L1: the way out of a lifecycle answer, for both front ends (D5 design,
//! L1). The routes were the CLI's (`human_lifecycle.rs`); they are decided
//! here, in its order, and each front end words them: the values carry no
//! text, and the CLI keeps its notes.
//!
//! A route lists what to do in turn. When the answer offers `repo.inspect`,
//! [`LifecycleWayOut::Inspect`] closes it, as the CLI's last `next:` line does.

use crate::{
    repository::ItemDtoKind,
    results::{Envelope, RecoveryAction, RecoveryActionKind, ResultCode},
};

use super::{
    LifecycleBlocker, LifecycleMutationDto, LifecycleRecording, MutationDataDto, PrepareDataDto,
    ResolveHistoryChoice,
};

/// One step of the way out of a lifecycle answer. Its IDs are the answer's;
/// what a step is called, and the notes that go with it, are the front end's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleWayOut {
    /// The same request ID, once what comes before is done.
    SendAgain,
    /// The unfinished request that holds the item, sent again (C5 decision
    /// 33).
    RetryRequest { request_id: String },
    /// A child's deleted branch (C5 decision 18): `request cancel`, then the
    /// same request, which answers `cancelled` and lets the child's
    /// reservation go.
    CancelThenSendAgain { request_id: String },
    /// `operation resume` of this operation.
    ResumeOperation { operation_id: String },
    /// The local repair of a refused merge: `context sync-local` and its
    /// siblings.
    RepairLocally { item_id: String },
    /// The conflict of this operation's context child, which the conflict
    /// verbs take by this operation's ID (C5 decision 34).
    ResolveChildConflict { operation_id: String },
    /// `item republish` of the item, whose branch the remote deleted.
    Republish { item_id: String },
    /// `repo sync`, which publishes primary if it is still ahead.
    SharePrimary,
    /// `poll once`, which reads primary's tracking ref and whether a
    /// published context still stands.
    PollOnce,
    /// `remote history`, then `item resolve-history`: the shared copy's
    /// history is unknown from evidence, which a poll can't read.
    ResolveHistory,
    /// A save of the item, which makes its missing context.
    SaveItem { item_id: String },
    /// A refused merge no other step routes, of a promotion or a closure,
    /// which only Git outside Manyhands repairs: `sync-local` admits only a
    /// closure recording its own checkpoint, with a way to go on (ruling
    /// R-13). Of an execute result only; the CLI prints nothing for it.
    RepairOutside,
    /// `repo inspect`, when the answer offers it.
    Inspect,
}

/// The verb that follows an answer of `item resolve-history`. The values
/// carry no text: each front end words them, as for [`LifecycleWayOut`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryWayOut {
    /// `context release` of the item: `release` is no choice of the verb
    /// (Decision 2).
    ReleaseContext { item_id: String },
    /// `item republish` of the item, whose published branch the remote
    /// deleted. Resolving never stands in for its consent.
    Republish { item_id: String },
    /// `item sync` of the item, which publishes it or integrates what the
    /// remote has.
    Sync { item_id: String },
    /// `remote history`, then `item resolve-history` for what it lists:
    /// some context still reads unknown, so the flag stays.
    RemoteHistory,
}

/// The way out of an answer of `remote resolve-history`: `recovery_required`
/// means a context still reads unknown.
pub fn clear_cache_loss_way_out(code: ResultCode) -> Option<HistoryWayOut> {
    (code == ResultCode::RecoveryRequired).then_some(HistoryWayOut::RemoteHistory)
}

/// The way out of an answer of `item resolve-history`, from its code alone:
/// a refused `release`, a deleted branch (or a tracking ref that says it was
/// published), a context already resolved or present on the remote, a
/// resolution that was made, and `recovery_required` (no current batch to
/// read absence from: `remote history` first). Nothing for any other code.
///
/// `InvalidInput` names the release only for the choice `release`, which is
/// the one input the verb refuses that way; any other `InvalidInput` is a
/// malformed request and is told nothing.
pub fn resolve_history_way_out(
    choice: ResolveHistoryChoice,
    code: ResultCode,
    item_id: &str,
) -> Option<HistoryWayOut> {
    let item_id = item_id.to_owned();
    match code {
        ResultCode::InvalidInput if choice == ResolveHistoryChoice::Release => {
            Some(HistoryWayOut::ReleaseContext { item_id })
        }
        ResultCode::RemoteBranchDeleted => Some(HistoryWayOut::Republish { item_id }),
        ResultCode::RecoveryRequired => Some(HistoryWayOut::RemoteHistory),
        ResultCode::Ok | ResultCode::AlreadyApplied => Some(HistoryWayOut::Sync { item_id }),
        _ => None,
    }
}

/// The way out of a result of `document promote`, `ticket close` or an
/// `operation resume` that continued one. Nothing for an answer with no
/// lifecycle data.
pub fn lifecycle_way_out(envelope: &Envelope<MutationDataDto>) -> Vec<LifecycleWayOut> {
    let Some(MutationDataDto::Lifecycle(data)) = &envelope.data else {
        return Vec::new();
    };
    let mut route = route_of(envelope, data);
    close_with_inspect(&envelope.recovery, &mut route);
    route
}

/// The way out of a refused prepare of `document promote` or `ticket close`:
/// the request or operation that holds the item, or a blocked prepare's way
/// out, which a new prepare follows.
///
/// `_kind` is the item's, which the blocked answer doesn't hold. No route
/// differs by it: a blocked prepare's `integration_conflict` is
/// `RepairLocally` for either kind, and a front end words `SaveItem` by it.
pub fn lifecycle_prepare_way_out(
    _kind: ItemDtoKind,
    envelope: &Envelope<PrepareDataDto>,
) -> Vec<LifecycleWayOut> {
    if let Some(held) = held(envelope) {
        return vec![held];
    }
    let Some(PrepareDataDto::LifecycleBlocked(data)) = &envelope.data else {
        return Vec::new();
    };
    let mut route = blocked(&data.blockers, envelope.scope.item_id.as_deref());
    close_with_inspect(&envelope.recovery, &mut route);
    route
}

fn offers(actions: &[RecoveryAction], kind: RecoveryActionKind) -> bool {
    actions.iter().any(|action| action.action == kind)
}

fn close_with_inspect(actions: &[RecoveryAction], route: &mut Vec<LifecycleWayOut>) {
    if offers(actions, RecoveryActionKind::RepoInspect) {
        route.push(LifecycleWayOut::Inspect);
    }
}

/// The resume a recovery action names: its operation, or the answer's.
fn resume(envelope: &Envelope<MutationDataDto>) -> Option<Option<LifecycleWayOut>> {
    let action = envelope
        .recovery
        .iter()
        .find(|action| action.action == RecoveryActionKind::OperationResume)?;
    Some(
        action
            .operation_id
            .as_deref()
            .or(envelope.operation_id.as_deref())
            .map(|operation| LifecycleWayOut::ResumeOperation {
                operation_id: operation.to_owned(),
            }),
    )
}

/// The first rule that applies, in the CLI's order; a refused merge it leaves
/// without a step is repaired outside (ruling R-13).
fn route_of(
    envelope: &Envelope<MutationDataDto>,
    data: &LifecycleMutationDto,
) -> Vec<LifecycleWayOut> {
    let blocked_by = |blocker| data.blockers.contains(&blocker);
    let refused = blocked_by(LifecycleBlocker::IntegrationConflict);
    let route = match envelope.code {
        ResultCode::RecoveryRequired
            if refused
                && data.kind == ItemDtoKind::Ticket
                && data.recording == Some(LifecycleRecording::Record) =>
        {
            merge_refused(envelope, &data.item_id)
        }
        ResultCode::RecoveryRequired
            if blocked_by(LifecycleBlocker::ConflictPending)
                && blocked_by(LifecycleBlocker::ContextConflicted) =>
        {
            child_conflict(envelope)
        }
        ResultCode::RemoteBranchDeleted => branch_deleted(envelope, &data.item_id),
        _ => Vec::new(),
    };
    if route.is_empty() && refused && envelope.code == ResultCode::RecoveryRequired {
        return vec![LifecycleWayOut::RepairOutside];
    }
    route
}

/// The repair, when the same request or a resume can then go on: `sync-local`
/// admits a closure only while it is recording its own
/// (`local_sync/record.rs`).
fn merge_refused(envelope: &Envelope<MutationDataDto>, item: &str) -> Vec<LifecycleWayOut> {
    let then = if offers(&envelope.recovery, RecoveryActionKind::RequestRetry) {
        Some(LifecycleWayOut::SendAgain)
    } else {
        match resume(envelope) {
            Some(resume) => resume,
            None => return Vec::new(),
        }
    };
    let mut route = vec![LifecycleWayOut::RepairLocally {
        item_id: item.to_owned(),
    }];
    route.extend(then);
    route
}

/// A context child's conflict (C5 decision 34), named by this operation's ID,
/// then the same request or the resume. An `operation resume` answer carries
/// no operation ID of its own (it is another's), so the parent the resume
/// action names stands for it: the conflict verbs accept the parent's.
fn child_conflict(envelope: &Envelope<MutationDataDto>) -> Vec<LifecycleWayOut> {
    let resumed = envelope
        .recovery
        .iter()
        .find(|action| action.action == RecoveryActionKind::OperationResume)
        .and_then(|action| action.operation_id.as_deref());
    let Some(operation) = envelope.operation_id.as_deref().or(resumed) else {
        return Vec::new();
    };
    let mut route = vec![LifecycleWayOut::ResolveChildConflict {
        operation_id: operation.to_owned(),
    }];
    if offers(&envelope.recovery, RecoveryActionKind::RequestRetry) {
        route.push(LifecycleWayOut::SendAgain);
    } else if let Some(resume) = resume(envelope) {
        route.extend(resume);
    }
    route
}

/// A child's `remote_branch_deleted`: under a resume, the republish then the
/// resume (C5 decision 19), since the resume has already asked for the
/// cancellation; otherwise the cancellation and the same request, when
/// offered, the republish, and the same request (C5 decision 18).
fn branch_deleted(envelope: &Envelope<MutationDataDto>, item: &str) -> Vec<LifecycleWayOut> {
    let republish = LifecycleWayOut::Republish {
        item_id: item.to_owned(),
    };
    if let Some(resume) = resume(envelope) {
        let mut route = vec![republish];
        route.extend(resume);
        return route;
    }
    let Some(request) = envelope.request_id.as_deref() else {
        return Vec::new();
    };
    let mut route = Vec::new();
    if offers(&envelope.recovery, RecoveryActionKind::RequestCancel) {
        route.push(LifecycleWayOut::CancelThenSendAgain {
            request_id: request.to_owned(),
        });
    }
    route.extend([republish, LifecycleWayOut::SendAgain]);
    route
}

/// A prepare refused because an unfinished request, or an operation no
/// request holds, has the item (C5 decision 33): its one action.
fn held(envelope: &Envelope<PrepareDataDto>) -> Option<LifecycleWayOut> {
    let (ResultCode::RecoveryRequired, None, [action]) =
        (envelope.code, &envelope.data, envelope.recovery.as_slice())
    else {
        return None;
    };
    match action.action {
        RecoveryActionKind::RequestRetry => Some(LifecycleWayOut::RetryRequest {
            request_id: action.arguments.get("request_id")?.as_str()?.to_owned(),
        }),
        RecoveryActionKind::OperationResume => Some(LifecycleWayOut::ResumeOperation {
            operation_id: action.operation_id.clone()?,
        }),
        _ => None,
    }
}

/// A blocked prepare's way out: the poll that reads primary's tracking ref,
/// left behind or missing, or a published context whose remote state is
/// unknown (C5 decision 35), with `repo sync` when primary is ahead; the save
/// that makes a missing context; or the local repair of a refused merge.
fn blocked(blockers: &[LifecycleBlocker], item: Option<&str>) -> Vec<LifecycleWayOut> {
    use LifecycleBlocker::{
        ConflictPending, ContextConflicted, ContextDirty, ContextMissing, HistoryUnknown,
        HistoryUnresolved, IntegrationConflict, LocalOperationPending, PrimaryAheadOfRemote,
        PrimaryMissing,
    };
    let tracking = |blocker: &LifecycleBlocker| {
        matches!(
            blocker,
            PrimaryAheadOfRemote | PrimaryMissing | HistoryUnknown
        )
    };
    if blockers.contains(&HistoryUnresolved)
        && blockers.iter().all(|blocker| {
            tracking(blocker) || matches!(blocker, HistoryUnresolved | ContextMissing)
        })
    {
        // The poll still reads what it can; the repair is the way out of
        // the unresolved history.
        let mut route = Vec::new();
        if blockers
            .iter()
            .any(|blocker| *blocker != HistoryUnknown && tracking(blocker))
        {
            route.push(LifecycleWayOut::PollOnce);
            if blockers.contains(&PrimaryAheadOfRemote) {
                route.push(LifecycleWayOut::SharePrimary);
            }
        }
        route.push(LifecycleWayOut::ResolveHistory);
        return route;
    }
    if blockers.iter().any(tracking)
        && blockers
            .iter()
            .all(|blocker| tracking(blocker) || *blocker == ContextMissing)
    {
        // Missing, a poll makes it. Ahead, primary may hold work never
        // published, which a poll doesn't push.
        let mut route = vec![LifecycleWayOut::PollOnce];
        if blockers.contains(&PrimaryAheadOfRemote) {
            route.push(LifecycleWayOut::SharePrimary);
        }
        return route;
    }
    let Some(item) = item else {
        return Vec::new();
    };
    if blockers == [ContextMissing] {
        return vec![LifecycleWayOut::SaveItem {
            item_id: item.to_owned(),
        }];
    }
    // `sync-local` refuses a dirty or conflicted context, and a pending conflict
    // or local operation has its own way on.
    let reachable = blockers.contains(&IntegrationConflict)
        && ![
            ContextDirty,
            ContextConflicted,
            ConflictPending,
            LocalOperationPending,
        ]
        .iter()
        .any(|other| blockers.contains(other));
    if reachable {
        vec![LifecycleWayOut::RepairLocally {
            item_id: item.to_owned(),
        }]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
#[path = "way_out_tests.rs"]
mod tests;
