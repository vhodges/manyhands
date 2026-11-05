//! Which conflict an answer stopped on, for the resolve conflict sheet's
//! entry points (D5 design, B5; decision 16): a share's or a post's
//! synchronization, a local synchronization's, or a lifecycle's context
//! child, which the conflict verbs take by the lifecycle's operation (L1
//! `ResolveChildConflict`; C5 decision 34). It names an operation, never a
//! path or any text.

use manyhands::{
    repository::{
        LifecycleWayOut, LocalContextSyncNextAction, MutationDataDto, OperationId,
        lifecycle_way_out,
    },
    results::Envelope,
};

/// The operation whose conflict `envelope` stopped on and waits to be
/// resolved, here or outside: `None` for any other answer.
pub(crate) fn conflict_of(envelope: &Envelope<MutationDataDto>) -> Option<OperationId> {
    let operation = match envelope.data.as_ref()? {
        MutationDataDto::SynchronizationConflict(data) => data.conflict.operation_id.clone(),
        MutationDataDto::Comment(data) => data.conflict.as_ref()?.operation_id.clone(),
        MutationDataDto::LocalContextSync(data) => {
            if !matches!(
                data.next_action,
                LocalContextSyncNextAction::Resolve
                    | LocalContextSyncNextAction::RetryResolution
                    | LocalContextSyncNextAction::ExternalRepair
            ) {
                return None;
            }
            data.operation_id.clone()?
        }
        MutationDataDto::Lifecycle(_) => {
            lifecycle_way_out(envelope)
                .into_iter()
                .find_map(|step| match step {
                    LifecycleWayOut::ResolveChildConflict { operation_id } => Some(operation_id),
                    _ => None,
                })?
        }
        _ => return None,
    };
    OperationId::parse(&operation).ok()
}
