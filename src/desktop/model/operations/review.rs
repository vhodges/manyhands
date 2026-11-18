//! Which conflict an answer stopped on, for the resolve conflict sheet's
//! entry points (D5 design, B5; decision 16): a share's or a post's
//! synchronization, a local synchronization's, or a lifecycle's context
//! child, which the conflict verbs take by the lifecycle's operation (L1
//! `ResolveChildConflict`; C5 decision 34). It names an operation and an
//! item, never a path or any text.

use manyhands::{
    canonical::ItemId,
    repository::{
        ItemDtoKind, LifecycleWayOut, LocalContextSyncNextAction, MutationDataDto, OperationId,
        lifecycle_way_out,
    },
    results::{Envelope, ResultCode},
};

use crate::desktop::services::requests::Intent;

/// What an answer's conflict is of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ReviewOf {
    /// A remote synchronization's, a share's or a post's.
    Synchronization,
    /// A local synchronization's.
    Local,
    /// A lifecycle's context child, by the lifecycle's operation.
    Lifecycle,
}

/// The conflict an answer stopped on, waiting to be resolved here or
/// outside.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Review {
    pub(crate) operation_id: OperationId,
    pub(crate) of: ReviewOf,
    /// The item and its kind, when the answer's data names them: a local
    /// synchronization's and a lifecycle's do.
    pub(crate) item: Option<(ItemId, ItemDtoKind)>,
}

fn kind_named(kind: &str) -> Option<ItemDtoKind> {
    match kind {
        "ticket" => Some(ItemDtoKind::Ticket),
        "document" => Some(ItemDtoKind::Document),
        _ => None,
    }
}

/// The conflict `envelope` stopped on: `None` for any other answer.
pub(crate) fn conflict_of(envelope: &Envelope<MutationDataDto>) -> Option<Review> {
    let (operation, of, item) = match envelope.data.as_ref()? {
        MutationDataDto::SynchronizationConflict(data) => (
            data.conflict.operation_id.clone(),
            ReviewOf::Synchronization,
            None,
        ),
        MutationDataDto::Comment(data) => (
            data.conflict.as_ref()?.operation_id.clone(),
            ReviewOf::Synchronization,
            None,
        ),
        MutationDataDto::LocalContextSync(data) => {
            if !matches!(
                data.next_action,
                LocalContextSyncNextAction::Resolve
                    | LocalContextSyncNextAction::RetryResolution
                    | LocalContextSyncNextAction::ExternalRepair
            ) {
                return None;
            }
            let item = data.item_id.parse().ok().zip(kind_named(&data.kind));
            (data.operation_id.clone()?, ReviewOf::Local, item)
        }
        MutationDataDto::Lifecycle(data) => {
            let operation =
                lifecycle_way_out(envelope)
                    .into_iter()
                    .find_map(|step| match step {
                        LifecycleWayOut::ResolveChildConflict { operation_id } => {
                            Some(operation_id)
                        }
                        _ => None,
                    })?;
            let item = data.item_id.parse().ok().map(|item| (item, data.kind));
            (operation, ReviewOf::Lifecycle, item)
        }
        _ => return None,
    };
    Some(Review {
        operation_id: OperationId::parse(&operation).ok()?,
        of,
        item,
    })
}

/// The operation whose conflict `intent`, answered `envelope`, settled: a
/// resolution applied, found already applied, or applied with its hand-off
/// owed (`discovery_pending`: a Resume on the row finishes it, and nothing
/// is left to choose) (B3: the banner clears); or a resume answered ok, or
/// already applied (the repair was pushed outside), that stopped on no
/// conflict, which took up a repair made outside (B4, P15).
pub(crate) fn resolved_by(
    intent: &Intent,
    envelope: &Envelope<MutationDataDto>,
) -> Option<OperationId> {
    match intent {
        Intent::ConflictResolve { operation_id, .. }
        | Intent::LocalContextResolve { operation_id, .. }
            if matches!(
                envelope.code,
                ResultCode::Ok | ResultCode::AlreadyApplied | ResultCode::DiscoveryPending
            ) =>
        {
            Some(*operation_id)
        }
        Intent::OperationResume { operation_id, .. }
            if matches!(envelope.code, ResultCode::Ok | ResultCode::AlreadyApplied)
                && conflict_of(envelope).is_none() =>
        {
            Some(*operation_id)
        }
        _ => None,
    }
}
