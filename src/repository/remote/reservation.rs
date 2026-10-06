//! Durable arbitration only. Tokens hold no SQLite guard, Git lease or transport.
#[cfg(test)]
#[path = "reservation_tests.rs"]
mod tests;
use std::{path::Path, sync::Arc};

use rusqlite::{Transaction, params};

use super::{
    RemoteOperationAction, RemoteOperationPhase, RemoteOperationPriority, RemoteOperationSafePoint,
    RemoteOperationTarget, RemoteOutcomeCategory, state,
};
use crate::repository::{
    OperationId, RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
};

/// Ownership is granted once. Exact replay returns inspection, never another token.
#[derive(Debug)]
pub struct RemoteReservation {
    scope: Arc<()>,
    operation_id: OperationId,
    repository_id: i64,
    generation: i64,
    epoch: i64,
}

impl RemoteReservation {
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
}

#[derive(Clone, Debug)]
pub struct RemoteOperationInspection {
    operation_id: OperationId,
    phase: RemoteOperationPhase,
    completed_step: Option<RemoteOperationSafePoint>,
    yield_requested: bool,
    cancel_requested: bool,
    outcome: Option<RemoteOutcomeCategory>,
}

impl From<state::StoredRemoteOperation> for RemoteOperationInspection {
    fn from(record: state::StoredRemoteOperation) -> Self {
        Self {
            operation_id: record.operation_id,
            phase: record.phase,
            completed_step: record.completed_step,
            yield_requested: record.yield_requested,
            cancel_requested: record.cancel_requested,
            outcome: record.outcome,
        }
    }
}

impl RemoteOperationInspection {
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    pub fn phase(&self) -> RemoteOperationPhase {
        self.phase
    }
    pub fn completed_step(&self) -> Option<RemoteOperationSafePoint> {
        self.completed_step
    }
    pub fn yield_requested(&self) -> bool {
        self.yield_requested
    }
    pub fn cancel_requested(&self) -> bool {
        self.cancel_requested
    }
    pub fn outcome(&self) -> Option<RemoteOutcomeCategory> {
        self.outcome
    }
}

#[derive(Debug)]
pub enum RemoteReservationOutcome {
    Reserved(RemoteReservation),
    Replay(RemoteOperationInspection),
    PollYielding,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteSafePointOutcome {
    Continue,
    Interrupted,
    Cancelled,
}

fn mismatch() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::OperationMismatch,
        "the remote operation ID is bound to a different action or target",
    )
}

fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp().max(0)
}

fn token(
    service: &RepositoryService,
    repository_id: i64,
    record: &state::StoredRemoteOperation,
) -> RemoteReservation {
    RemoteReservation {
        scope: Arc::clone(&service.remote_reservation_scope),
        operation_id: record.operation_id,
        repository_id,
        generation: record.generation,
        epoch: record.owner_epoch,
    }
}

fn active(phase: RemoteOperationPhase) -> bool {
    matches!(
        phase,
        RemoteOperationPhase::Reserved
            | RemoteOperationPhase::Advertising
            | RemoteOperationPhase::Persisting
    )
}

/// Called inside the same immediate transaction as the following state write.
pub(super) fn owned(
    service: &RepositoryService,
    tx: &Transaction<'_>,
    repository_id: i64,
    owner: &RemoteReservation,
) -> Result<state::StoredRemoteOperation, RepositoryError> {
    if !Arc::ptr_eq(&service.remote_reservation_scope, &owner.scope) {
        return Err(state::recovery_required());
    }
    let record = state::read_operation(tx, repository_id, owner.operation_id)?
        .ok_or_else(state::recovery_required)?;
    if repository_id != owner.repository_id
        || record.generation != owner.generation
        || state::generation(tx, repository_id)? != owner.generation
        || record.owner_epoch != owner.epoch
        || !active(record.phase)
    {
        return Err(state::recovery_required());
    }
    crate::repository::recovery::require_no_pending_local(tx, repository_id)?;
    Ok(record)
}

fn point_name(point: RemoteOperationSafePoint) -> &'static str {
    match point {
        RemoteOperationSafePoint::BeforeTransport => "before_transport",
        RemoteOperationSafePoint::AfterAdvertisement => "after_advertisement",
        RemoteOperationSafePoint::BetweenObservations => "between_observations",
        RemoteOperationSafePoint::BeforeBatchCommit => "before_batch_commit",
        RemoteOperationSafePoint::AfterBatchCommit => "after_batch_commit",
        RemoteOperationSafePoint::BeforeLocalMutation => "before_local_mutation",
    }
}

fn point_order(point: RemoteOperationSafePoint) -> u8 {
    match point {
        RemoteOperationSafePoint::BeforeTransport => 0,
        RemoteOperationSafePoint::AfterAdvertisement => 1,
        RemoteOperationSafePoint::BetweenObservations => 2,
        RemoteOperationSafePoint::BeforeBatchCommit => 3,
        RemoteOperationSafePoint::AfterBatchCommit => 4,
        RemoteOperationSafePoint::BeforeLocalMutation => 5,
    }
}

fn acknowledge(
    tx: &Transaction<'_>,
    record: &state::StoredRemoteOperation,
    point: RemoteOperationSafePoint,
) -> Result<RemoteSafePointOutcome, RepositoryError> {
    let (phase, outcome, result) = if record.cancel_requested {
        (
            "cancelled",
            Some("cancelled"),
            RemoteSafePointOutcome::Cancelled,
        )
    } else if record.yield_requested {
        ("interrupted", None, RemoteSafePointOutcome::Interrupted)
    } else {
        (
            match point {
                RemoteOperationSafePoint::BeforeTransport
                | RemoteOperationSafePoint::AfterAdvertisement => "advertising",
                _ => "persisting",
            },
            None,
            RemoteSafePointOutcome::Continue,
        )
    };
    tx.execute("UPDATE remote_operation_records SET phase=?2,completed_step=?3,outcome=?4,updated_at=max(updated_at,?5) WHERE id=?1",
        params![record.id,phase,point_name(point),outcome,now()]).map_err(|_| state::recovery_required())?;
    if result == RemoteSafePointOutcome::Cancelled {
        tx.execute("UPDATE remote_polling_state SET latest_outcome='cancelled' WHERE repository_id=(SELECT repository_id FROM remote_operation_records WHERE id=?1)",
            [record.id]).map_err(|_| state::recovery_required())?;
    }
    Ok(result)
}

/// The only observation publication path for a reservation. Ownership, requests,
/// complete-batch writes and the durable post-commit checkpoint commit together.
pub(in super::super) fn commit_observation_batch(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    plan: &super::RemoteRefPlan,
    observations: &[super::RemoteRefObservation],
    observed_at: i64,
) -> Result<RemoteSafePointOutcome, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        let record = owned(service, tx, id, owner)?;
        if record.target.action() != RemoteOperationAction::Poll
            || record.completed_step != Some(RemoteOperationSafePoint::BeforeBatchCommit)
        {
            return Err(state::recovery_required());
        }
        let decision = acknowledge(tx, &record, RemoteOperationSafePoint::BeforeBatchCommit)?;
        if decision != RemoteSafePointOutcome::Continue {
            return Ok(decision);
        }
        state::complete_batch(tx, id, plan, owner.generation, observations, observed_at)?;
        acknowledge(tx, &record, RemoteOperationSafePoint::AfterBatchCommit)
    })
}

fn finish(
    tx: &Transaction<'_>,
    id: i64,
    record: &state::StoredRemoteOperation,
    category: RemoteOutcomeCategory,
) -> Result<(), RepositoryError> {
    let phase = match category {
        RemoteOutcomeCategory::Completed => "completed",
        RemoteOutcomeCategory::Cancelled => "cancelled",
        _ => "failed",
    };
    // Persist future automatic retry policy, without scheduling or delaying this
    // call. Attention states and explicit failures retain the existing policy.
    let previous = state::read_snapshot(tx, id)?.polling().automatic_backoff();
    let delay = match category {
        RemoteOutcomeCategory::Completed => None,
        RemoteOutcomeCategory::TransportUnavailable | RemoteOutcomeCategory::ProtocolRejected
            if record.priority == RemoteOperationPriority::Poll =>
        {
            Some(previous.map_or(60, |delay| delay.as_secs().saturating_mul(2).min(900)))
        }
        _ => previous.map(|delay| delay.as_secs()),
    };
    let backoff = delay
        .map(super::AutomaticBackoff::from_seconds)
        .transpose()
        .map_err(|_| state::recovery_required())?;
    state::record_outcome(tx, id, category, backoff)?;
    tx.execute("UPDATE remote_operation_records SET phase=?2,outcome=?3,updated_at=max(updated_at,?4) WHERE id=?1",
        params![record.id,phase,state::outcome_name(category),now()]).map_err(|_| state::recovery_required())?;
    Ok(())
}

impl RepositoryService {
    /// Reserves an already-preflighted, validated target. Configuration is supplied
    /// to the state layer by the caller; this method performs no Git/transport work.
    pub fn reserve_remote_operation(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        let priority = if target.action() == RemoteOperationAction::Poll {
            RemoteOperationPriority::Poll
        } else {
            RemoteOperationPriority::Manual
        };
        self.reserve_remote_operation_with_priority(root, operation_id, target, priority)
    }

    /// Explicit observations are Poll actions with Manual priority. Only
    /// automatic observations use Poll priority and yield to manual requests.
    pub fn reserve_remote_operation_with_priority(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        priority: RemoteOperationPriority,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        if priority == RemoteOperationPriority::Poll
            && target.action() != RemoteOperationAction::Poll
        {
            return Err(mismatch());
        }
        state::with_transaction(self, root, |tx, repository_id| {
            let local_id: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1)",
                    [operation_id.to_string()],
                    |row| row.get(0),
                )
                .map_err(|_| state::recovery_required())?;
            if local_id {
                return Err(mismatch());
            }
            if let Some(record) = state::read_operation(tx, repository_id, operation_id)? {
                if &record.target != target || record.priority != priority {
                    return Err(mismatch());
                }
                return Ok(RemoteReservationOutcome::Replay(record.into()));
            }
            crate::repository::recovery::require_no_pending_local(tx, repository_id)?;
            state::read_snapshot(tx, repository_id)?;
            if let Some(record) = state::read_operation_rows(tx, repository_id, true)?
                .into_iter()
                .next()
            {
                if priority == RemoteOperationPriority::Manual
                    && record.priority == RemoteOperationPriority::Poll
                {
                    tx.execute("UPDATE remote_operation_records SET yield_requested=1,updated_at=max(updated_at,?2) WHERE id=?1",params![record.id,now()]).map_err(|_| state::recovery_required())?;
                    return Ok(RemoteReservationOutcome::PollYielding);
                }
                return Ok(RemoteReservationOutcome::Busy);
            }
            state::insert_operation(tx, repository_id, operation_id, target, priority, now())?;
            let record = state::read_operation(tx, repository_id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            Ok(RemoteReservationOutcome::Reserved(token(
                self,
                repository_id,
                &record,
            )))
        })
    }

    pub fn active_remote_operation(
        &self,
        root: &Path,
    ) -> Result<Option<RemoteOperationInspection>, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            Ok(state::read_operation_rows(tx, id, true)?
                .into_iter()
                .next()
                .map(RemoteOperationInspection::from))
        })
    }

    pub fn cancel_remote_operation(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if active(record.phase) {
                tx.execute("UPDATE remote_operation_records SET cancel_requested=1,updated_at=max(updated_at,?2) WHERE id=?1",params![record.id,now()]).map_err(|_| state::recovery_required())?;
            }
            Ok(())
        })
    }

    /// Also usable by future transfer-progress callbacks; Cycle 04 invokes only
    /// these non-transfer boundaries. Returns only after the durable transition.
    pub fn remote_safe_point(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        point: RemoteOperationSafePoint,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if record
                .completed_step
                .is_some_and(|previous| point_order(previous) > point_order(point))
                || point == RemoteOperationSafePoint::AfterBatchCommit
                    && record.completed_step != Some(point)
                    && !record.cancel_requested
                    && !record.yield_requested
            {
                return Err(state::recovery_required());
            }
            acknowledge(tx, &record, point)
        })
    }

    /// Explicit recovery of read-only polling. Opening another service never
    /// steals ownership. Restart fences the old executor before returning a token.
    pub fn restart_remote_observation(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if &record.target != target {
                return Err(mismatch());
            }
            if matches!(
                record.phase,
                RemoteOperationPhase::Completed | RemoteOperationPhase::Cancelled
            ) {
                return Ok(RemoteReservationOutcome::Replay(record.into()));
            }
            crate::repository::recovery::require_no_pending_local(tx, id)?;
            if record.generation != state::generation(tx, id)?
                || target.action() != RemoteOperationAction::Poll
                || record.completed_step == Some(RemoteOperationSafePoint::BeforeLocalMutation)
            {
                return Err(state::recovery_required());
            }
            if state::read_operation_rows(tx, id, true)?
                .iter()
                .any(|other| other.id != record.id)
            {
                return Ok(RemoteReservationOutcome::Busy);
            }
            // A restart itself is before any new transport. A yield on an
            // active row is pending; Interrupted already durably acknowledged
            // it. Cancellation remains terminal and is never cleared here.
            if record.cancel_requested || record.yield_requested && active(record.phase) {
                acknowledge(
                    tx,
                    &record,
                    record
                        .completed_step
                        .unwrap_or(RemoteOperationSafePoint::BeforeTransport),
                )?;
            } else if record.completed_step == Some(RemoteOperationSafePoint::AfterBatchCommit) {
                finish(tx, id, &record, RemoteOutcomeCategory::Completed)?;
            } else {
                let epoch = record
                    .owner_epoch
                    .checked_add(1)
                    .ok_or_else(state::recovery_required)?;
                tx.execute("UPDATE remote_operation_records SET phase='reserved',completed_step=NULL,outcome=NULL,yield_requested=0,owner_epoch=?2,updated_at=max(updated_at,?3) WHERE id=?1",
                    params![record.id,epoch,now()]).map_err(|_| state::recovery_required())?;
                let restarted = state::read_operation(tx, id, operation_id)?
                    .ok_or_else(state::recovery_required)?;
                return Ok(RemoteReservationOutcome::Reserved(token(
                    self, id, &restarted,
                )));
            }
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            Ok(RemoteReservationOutcome::Replay(record.into()))
        })
    }

    /// A transport failure completes only at a previously acknowledged boundary.
    /// Successful polling requires an atomically persisted complete batch.
    pub fn finish_remote_operation(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        category: RemoteOutcomeCategory,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            let point = record.completed_step.ok_or_else(state::recovery_required)?;
            if record.target.action() != RemoteOperationAction::Poll
                || category == RemoteOutcomeCategory::Completed
                    && point != RemoteOperationSafePoint::AfterBatchCommit
            {
                return Err(state::recovery_required());
            }
            let decision = acknowledge(tx, &record, point)?;
            if decision != RemoteSafePointOutcome::Continue {
                return Ok(decision);
            }
            finish(tx, id, &record, category)?;
            Ok(if category == RemoteOutcomeCategory::Cancelled {
                RemoteSafePointOutcome::Cancelled
            } else {
                RemoteSafePointOutcome::Continue
            })
        })
    }
}
