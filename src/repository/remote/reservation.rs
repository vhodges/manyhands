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
    sync_checkpoint: Option<state::SynchronizationCheckpoint>,
    sync_evidence: state::SynchronizationEvidence,
    authority: Option<state::SynchronizationAuthority>,
    index_pending: bool,
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
            sync_checkpoint: record.sync_checkpoint,
            sync_evidence: record.sync_evidence,
            authority: record.authority,
            index_pending: record.index_pending,
        }
    }
}

impl RemoteOperationInspection {
    pub(crate) fn sync_checkpoint(&self) -> Option<state::SynchronizationCheckpoint> {
        self.sync_checkpoint
    }
    pub(crate) fn sync_evidence(&self) -> &state::SynchronizationEvidence {
        &self.sync_evidence
    }
    pub(crate) fn authority(&self) -> Option<state::SynchronizationAuthority> {
        self.authority
    }
    pub(crate) fn index_pending(&self) -> bool {
        self.index_pending
    }

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

/// The sole allowed local-ID coexistence is the exact authoritative action's
/// ordinary refresh handoff. Check every row, including failed/in-progress rows.
fn validate_index_identity(
    tx: &Transaction<'_>,
    id: i64,
    operation_id: OperationId,
) -> Result<(), RepositoryError> {
    let unrelated: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1 AND (repository_id IS NULL OR repository_id!=?2 OR root_path!=(SELECT root_path FROM repositories WHERE id=?2) OR action!='refresh' OR target IS NULL OR target!=''))",params![operation_id.to_string(),id],|row|row.get(0)).map_err(|_| state::recovery_required())?;
    if unrelated { Err(mismatch()) } else { Ok(()) }
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
            | RemoteOperationPhase::FetchPrepared
            | RemoteOperationPhase::FetchObserved
            | RemoteOperationPhase::LocalPrepared
            | RemoteOperationPhase::LocalFastForwarded
            | RemoteOperationPhase::PushPrepared
            | RemoteOperationPhase::PushReturned
            | RemoteOperationPhase::PushVerified
            | RemoteOperationPhase::Reconciling
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

fn is_sync(target: &RemoteOperationTarget) -> bool {
    matches!(
        target.action(),
        RemoteOperationAction::SynchronizePrimary | RemoteOperationAction::SynchronizeContext
    )
}
fn phase_name(phase: RemoteOperationPhase) -> &'static str {
    match phase {
        RemoteOperationPhase::Reserved => "reserved",
        RemoteOperationPhase::Advertising => "advertising",
        RemoteOperationPhase::Persisting => "persisting",
        RemoteOperationPhase::Completed => "completed",
        RemoteOperationPhase::Interrupted => "interrupted",
        RemoteOperationPhase::Cancelled => "cancelled",
        RemoteOperationPhase::Failed => "failed",
        RemoteOperationPhase::FetchPrepared => "fetch_prepared",
        RemoteOperationPhase::FetchObserved => "fetch_observed",
        RemoteOperationPhase::LocalPrepared => "local_prepared",
        RemoteOperationPhase::LocalFastForwarded => "local_fast_forwarded",
        RemoteOperationPhase::PushPrepared => "push_prepared",
        RemoteOperationPhase::PushReturned => "push_returned",
        RemoteOperationPhase::PushVerified => "push_verified",
        RemoteOperationPhase::Reconciling => "reconciling",
    }
}
fn validate_sync_point(
    record: &state::StoredRemoteOperation,
    point: RemoteOperationSafePoint,
) -> Result<(), RepositoryError> {
    use RemoteOperationSafePoint as P;
    use state::SynchronizationCheckpoint as C;
    let valid = match point {
        P::BeforeFetch => matches!(
            record.phase,
            RemoteOperationPhase::FetchPrepared | RemoteOperationPhase::Reconciling
        ),
        P::AfterFetch => record.completed_step == Some(P::AfterFetch),
        P::BeforeLocalUpdate => {
            !record.reconciliation_required && record.sync_checkpoint == Some(C::LocalPrepared)
        }
        P::AfterLocalUpdate => {
            !record.reconciliation_required && record.sync_checkpoint == Some(C::LocalFastForwarded)
        }
        P::BeforePush => {
            !record.reconciliation_required && record.sync_checkpoint == Some(C::PushPrepared)
        }
        P::AfterPushReturn => {
            !record.reconciliation_required && record.sync_checkpoint == Some(C::PushReturned)
        }
        P::AfterPushVerification => {
            !record.reconciliation_required && record.sync_checkpoint == Some(C::PushVerified)
        }
        P::BeforeDiscovery => false, // classified outcomes release the remote reservation for index-only replay
        // Cycle 04 exposed this boundary for manual targets before action
        // checkpoints existed. Retain it, but never infer a retry-safe effect.
        P::BeforeLocalMutation => record.sync_checkpoint.is_none(),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(state::recovery_required())
    }
}

fn point_name(point: RemoteOperationSafePoint) -> &'static str {
    match point {
        RemoteOperationSafePoint::BeforeTransport => "before_transport",
        RemoteOperationSafePoint::AfterAdvertisement => "after_advertisement",
        RemoteOperationSafePoint::BetweenObservations => "between_observations",
        RemoteOperationSafePoint::BeforeBatchCommit => "before_batch_commit",
        RemoteOperationSafePoint::AfterBatchCommit => "after_batch_commit",
        RemoteOperationSafePoint::BeforeLocalMutation => "before_local_mutation",
        RemoteOperationSafePoint::BeforeFetch => "before_fetch",
        RemoteOperationSafePoint::AfterFetch => "after_fetch",
        RemoteOperationSafePoint::BeforeLocalUpdate => "before_local_update",
        RemoteOperationSafePoint::AfterLocalUpdate => "after_local_update",
        RemoteOperationSafePoint::BeforePush => "before_push",
        RemoteOperationSafePoint::AfterPushReturn => "after_push_return",
        RemoteOperationSafePoint::AfterPushVerification => "after_push_verification",
        RemoteOperationSafePoint::BeforeDiscovery => "before_discovery",
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
        RemoteOperationSafePoint::BeforeFetch => 0,
        RemoteOperationSafePoint::AfterFetch => 1,
        RemoteOperationSafePoint::BeforeLocalUpdate => 2,
        RemoteOperationSafePoint::AfterLocalUpdate => 3,
        RemoteOperationSafePoint::BeforePush => 4,
        RemoteOperationSafePoint::AfterPushReturn => 5,
        RemoteOperationSafePoint::AfterPushVerification => 6,
        RemoteOperationSafePoint::BeforeDiscovery => 7,
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
    let phase = if result == RemoteSafePointOutcome::Continue && is_sync(&record.target) {
        phase_name(record.phase)
    } else {
        phase
    };
    tx.execute("UPDATE remote_operation_records SET phase=?2,completed_step=?3,outcome=?4,updated_at=max(updated_at,?5) WHERE id=?1",
        params![record.id,phase,point_name(point),outcome,now()]).map_err(|_| state::recovery_required())?;
    if result == RemoteSafePointOutcome::Cancelled && !is_sync(&record.target) {
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
        let synchronization = is_sync(&record.target);
        if synchronization {
            if record.authority.is_some()
                || !(record.phase == RemoteOperationPhase::FetchPrepared
                    || record.phase == RemoteOperationPhase::Reconciling)
                || record.completed_step != Some(RemoteOperationSafePoint::BeforeFetch)
            {
                return Err(state::recovery_required());
            }
        } else if record.target.action() != RemoteOperationAction::Poll
            || record.completed_step != Some(RemoteOperationSafePoint::BeforeBatchCommit)
        {
            return Err(state::recovery_required());
        }
        let decision = acknowledge(
            tx,
            &record,
            if synchronization {
                RemoteOperationSafePoint::BeforeFetch
            } else {
                RemoteOperationSafePoint::BeforeBatchCommit
            },
        )?;
        if decision != RemoteSafePointOutcome::Continue {
            return Ok(decision);
        }
        if synchronization {
            state::persist_complete_advertisement(
                tx,
                id,
                plan,
                owner.generation,
                observations,
                observed_at,
            )?;
        } else {
            state::complete_batch(tx, id, plan, owner.generation, observations, observed_at)?;
        }
        if synchronization {
            // Fresh Fetch evidence cannot erase an unresolved local/push checkpoint.
            tx.execute("UPDATE remote_operation_records SET phase=CASE WHEN reconciliation_required=1 THEN 'reconciling' ELSE 'fetch_observed' END,sync_checkpoint=CASE WHEN reconciliation_required=1 THEN sync_checkpoint ELSE 'fetch_observed' END WHERE id=?1",[record.id]).map_err(|_| state::recovery_required())?;
            let current = state::read_operation(tx, id, owner.operation_id)?
                .ok_or_else(state::recovery_required)?;
            acknowledge(tx, &current, RemoteOperationSafePoint::AfterFetch)
        } else {
            acknowledge(tx, &record, RemoteOperationSafePoint::AfterBatchCommit)
        }
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
    if !is_sync(&record.target) {
        let previous = state::read_snapshot(tx, id)?.polling().automatic_backoff();
        let delay = match category {
            RemoteOutcomeCategory::Completed => None,
            RemoteOutcomeCategory::TransportUnavailable
            | RemoteOutcomeCategory::ProtocolRejected
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
    }
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
            let other_root: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",params![operation_id.to_string(), repository_id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
            if is_sync(target) && other_root {
                return Err(mismatch());
            }
            if let Some(record) = state::read_operation(tx, repository_id, operation_id)? {
                if &record.target != target || record.priority != priority {
                    return Err(mismatch());
                }
                if record.authority.is_some()
                    && record.phase == RemoteOperationPhase::Completed
                    && is_sync(target)
                {
                    validate_index_identity(tx, repository_id, operation_id)?;
                } else if local_id {
                    return Err(mismatch());
                }
                return Ok(RemoteReservationOutcome::Replay(record.into()));
            }
            if local_id {
                return Err(mismatch());
            }
            crate::repository::recovery::require_no_pending_local(tx, repository_id)?;
            if is_sync(target) && state::has_pending_conflict(tx, repository_id)? {
                return Ok(RemoteReservationOutcome::Busy);
            }
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
            if is_sync(&record.target) {
                validate_sync_point(&record, point)?;
            } else if matches!(
                point,
                RemoteOperationSafePoint::BeforeFetch
                    | RemoteOperationSafePoint::AfterFetch
                    | RemoteOperationSafePoint::BeforeLocalUpdate
                    | RemoteOperationSafePoint::AfterLocalUpdate
                    | RemoteOperationSafePoint::BeforePush
                    | RemoteOperationSafePoint::AfterPushReturn
                    | RemoteOperationSafePoint::AfterPushVerification
                    | RemoteOperationSafePoint::BeforeDiscovery
            ) {
                return Err(state::recovery_required());
            }
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
            if !(record.target.action() == RemoteOperationAction::Poll || is_sync(&record.target))
                || category == RemoteOutcomeCategory::Completed
                    && (is_sync(&record.target)
                        || point != RemoteOperationSafePoint::AfterBatchCommit)
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

// These crate-private transitions are consumed by Task 4, never by a caller's
// request. Their OIDs must come from re-opened Git/worktree and scoped observations.
impl RepositoryService {
    /// Repeated inter-call boundary: fence ownership and honor requests without
    /// advancing the action checkpoint or claiming any effect proof.
    pub(crate) fn check_synchronization_requests(
        &self,
        root: &Path,
        owner: &RemoteReservation,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target) {
                return Err(state::recovery_required());
            }
            if record.cancel_requested || record.yield_requested {
                acknowledge(
                    tx,
                    &record,
                    record
                        .completed_step
                        .unwrap_or(RemoteOperationSafePoint::BeforeFetch),
                )
            } else {
                Ok(RemoteSafePointOutcome::Continue)
            }
        })
    }

    /// Child intent is durable before a merge/index effect. The owner epoch is
    /// captured in the immutable row; a later reacquisition receives a new
    /// token and cannot rewrite the earlier intent.
    #[allow(dead_code)] // Consumed by the ordered merge/resolution tasks.
    pub(super) fn prepare_synchronization_integration(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        intent: &state::IntegrationStepIntent,
    ) -> Result<state::IntegrationStepEvidence, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_integration_step(tx, &record, intent)
        })
    }

    /// Read one owned, fenced candidate application during explicit restart.
    /// The returned immutable intent is never reconstructed from live refs.
    pub(super) fn applying_synchronization_candidate(
        &self,
        root: &Path,
        owner: &RemoteReservation,
    ) -> Result<Option<state::IntegrationStepEvidence>, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target) || !record.reconciliation_required {
                return Err(state::recovery_required());
            }
            // Prefer the latest stage. A locally completed resolution retains
            // its publication handoff until explicit restart observes Fetch.
            for ordinal in (0..=1).rev() {
                if let Some(step) = state::integration_step(tx, record.id, ordinal)?
                    && step.candidate_oid.is_some()
                {
                    let released_resolution: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM remote_resolution_attempts attempt JOIN remote_resolution_index_artifacts artifact ON artifact.attempt_id=attempt.id WHERE attempt.operation_record_id=?1 AND attempt.integration_step_id=(SELECT id FROM remote_integration_steps WHERE operation_record_id=?1 AND ordinal=?2) AND attempt.phase='applied' AND artifact.phase='released' AND artifact.ref_phase='observed')",
                        params![record.id, ordinal], |row| row.get(0),
                    ).map_err(|_| state::recovery_required())?;
                    if step.phase == state::IntegrationStepPhase::Applying
                        || (step.phase == state::IntegrationStepPhase::Applied
                            && step.result_oid != record.sync_evidence.local_oid
                            && released_resolution)
                    {
                        return Ok(Some(step));
                    }
                }
            }
            Ok(None)
        })
    }

    /// Commit the already-observed child application into the legacy envelope.
    /// This is the only restart transition permitted for a pending candidate.
    pub(super) fn reconcile_synchronization_candidate_applied(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        candidate: git2::Oid,
        tree: git2::Oid,
        evidence: &state::SynchronizationEvidence,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            use state::SynchronizationCheckpoint as C;
            let step = state::integration_step(tx, record.id, ordinal)?
                .ok_or_else(state::recovery_required)?;
            if !is_sync(&record.target)
                || !record.reconciliation_required
                || record.sync_checkpoint != Some(C::FetchObserved)
                || step.phase != state::IntegrationStepPhase::Applied
                || step.candidate_oid != Some(candidate)
                || step.result_oid != Some(candidate)
                || step.observed_tree_oid != Some(tree)
                || evidence.expected_oid != record.sync_evidence.expected_oid
                || evidence.local_oid != Some(candidate)
                || evidence.primary_tracking_oid.is_none()
            {
                return Err(state::recovery_required());
            }
            tx.execute(
                "UPDATE remote_operation_records SET phase='local_fast_forwarded', sync_checkpoint='local_fast_forwarded', reconciliation_required=0, local_oid=?2, tracking_oid=?3, primary_tracking_oid=?4, updated_at=max(updated_at,?5) WHERE id=?1",
                params![record.id, candidate.to_string(), evidence.tracking_oid.map(|oid| oid.to_string()), evidence.primary_tracking_oid.map(|oid| oid.to_string()), now()],
            ).map_err(|_| state::recovery_required())?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    #[allow(dead_code)] // Consumed by the ordered merge/resolution tasks.
    pub(super) fn begin_synchronization_integration_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        candidate_oid: Option<git2::Oid>,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::begin_integration_effect(tx, &record, ordinal, candidate_oid)
        })
    }

    #[allow(dead_code)] // Consumed by the ordered merge/resolution tasks.
    pub(super) fn observe_synchronization_integration_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        result_oid: git2::Oid,
        observed_tree_oid: git2::Oid,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::observe_integration_effect(tx, &record, ordinal, result_oid, observed_tree_oid)
        })
    }

    /// A conflict is observed after the Git effect and then releases only the
    /// active slot. The immutable operation/stage remains for explicit matching
    /// reconciliation; another synchronization cannot adopt it.
    #[allow(dead_code)] // Consumed by the ordered merge/resolution tasks.
    pub(super) fn release_synchronization_conflict(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        conflict_digest: [u8; 32],
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::record_integration_conflict(tx, &record, ordinal, conflict_digest)?;
            tx.execute(
                "UPDATE remote_operation_records SET phase='interrupted',outcome=NULL,updated_at=max(updated_at,?2) WHERE id=?1 AND owner_epoch=?3",
                params![record.id, now(), owner.epoch],
            )
            .map_err(|_| state::recovery_required())?;
            Ok(())
        })
    }

    /// Reacquisition is deliberately separate from ordinary restart: it proves
    /// the exact durable operation, target, generation and conflict digest.
    #[allow(dead_code)] // Consumed by the ordered merge/resolution tasks.
    pub(super) fn reacquire_synchronization_conflict(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        ordinal: u8,
        conflict_digest: [u8; 32],
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if &record.target != target
                || !is_sync(target)
                || !matches!(
                    record.phase,
                    RemoteOperationPhase::Interrupted | RemoteOperationPhase::Reconciling
                )
                || record.generation != state::generation(tx, id)?
                || record.cancel_requested
            {
                return Err(state::recovery_required());
            }
            let step = state::integration_step(tx, record.id, ordinal)?
                .ok_or_else(state::recovery_required)?;
            if !matches!(
                step.phase,
                state::IntegrationStepPhase::ConflictPending
                    | state::IntegrationStepPhase::ResolutionPrepared
                    | state::IntegrationStepPhase::CommitPrepared
                    | state::IntegrationStepPhase::Applied
            ) || step.conflict_digest != Some(conflict_digest)
            {
                return Err(state::recovery_required());
            }
            if state::read_operation_rows(tx, id, true)?
                .iter()
                .any(|other| other.id != record.id)
            {
                return Ok(RemoteReservationOutcome::Busy);
            }
            let epoch = record
                .owner_epoch
                .checked_add(1)
                .ok_or_else(state::recovery_required)?;
            tx.execute(
                "UPDATE remote_operation_records SET phase='reconciling',reconciliation_required=1,completed_step=NULL,yield_requested=0,owner_epoch=?2,updated_at=max(updated_at,?3) WHERE id=?1",
                params![record.id, epoch, now()],
            )
            .map_err(|_| state::recovery_required())?;
            let reacquired = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            Ok(RemoteReservationOutcome::Reserved(token(
                self,
                id,
                &reacquired,
            )))
        })
    }

    /// Release only local execution ownership after verified metadata/sentinel
    /// retirement. Retain the original synchronization and frozen child evidence
    /// for explicit re-fetch/publication reconciliation, never published authority.
    pub(super) fn finalize_synchronization_resolution(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        checkpoint: git2::Oid,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            let (step, candidate, phase, _, _, _) =
                state::resolution_candidate_for_attempt(tx, &record, attempt)?
                    .ok_or_else(state::recovery_required)?;
            let artifact = state::resolution_index_artifact(tx, &record, attempt)?
                .ok_or_else(state::recovery_required)?;
            if record.phase != RemoteOperationPhase::Reconciling
                || !record.reconciliation_required
                || record.authority.is_some()
                || phase != "applied"
                || candidate != checkpoint
                || step.phase != state::IntegrationStepPhase::Applied
                || step.result_oid != Some(checkpoint)
                || artifact.phase != "released"
                || artifact.ref_phase != "observed"
            {
                return Err(state::recovery_required());
            }
            tx.execute(
                "UPDATE remote_operation_records SET phase='interrupted',outcome=NULL,updated_at=max(updated_at,?2) WHERE id=?1 AND owner_epoch=?3",
                params![record.id, now(), owner.epoch],
            ).map_err(|_| state::recovery_required())?;
            Ok(())
        })
    }

    #[allow(dead_code)] // Consumed by the ordered merge/resolution tasks.
    pub(super) fn prepare_synchronization_identity_confirmation(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        intent: &state::IdentityConfirmationIntent,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_identity_confirmation(tx, &record, intent)
        })
    }

    #[allow(dead_code)] // Consumed by the ordered merge/resolution tasks.
    pub(super) fn prepare_synchronization_resolution_attempt(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        intent: &state::ResolutionAttemptIntent,
        paths: &[state::ResolutionPathIntent],
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_resolution_attempt(tx, &record, intent, paths)
        })
    }

    pub(super) fn prepare_synchronization_ref_log_artifact(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        role: &str,
        artifact: &state::ResolutionRefLogArtifact,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_resolution_ref_log_artifact(tx, &record, attempt, role, artifact)
        })
    }

    pub(super) fn prepare_synchronization_index_artifact(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        artifact: &state::ResolutionIndexArtifact,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_resolution_index_artifact(tx, &record, attempt, artifact)
        })
    }

    pub(super) fn prepare_synchronization_index_output(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        output: (u64, u64, [u8; 32]),
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_resolution_index_output(tx, &record, attempt, output)
        })
    }

    pub(super) fn advance_synchronization_resolution_ref_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        next: &str,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::advance_resolution_ref_effect(tx, &record, attempt, next)
        })
    }

    pub(super) fn advance_synchronization_index_artifact(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        next: &str,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::advance_resolution_index_artifact(tx, &record, attempt, next)
        })
    }

    #[allow(dead_code)] // Task 4 supplies observed owned-path writes.
    pub(super) fn begin_synchronization_resolution_path_effects(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::begin_resolution_path_effects(tx, &record, attempt)
        })
    }

    #[allow(dead_code)] // Task 4 observes each guarded path after its write.
    pub(super) fn observe_synchronization_resolution_path_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        ordinal: u32,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::observe_resolution_path_effect(tx, &record, attempt, ordinal)
        })
    }

    #[allow(dead_code)] // Task 4 records the detached two-parent candidate before ref movement.
    pub(super) fn prepare_synchronization_resolution_candidate(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        candidate_oid: git2::Oid,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_resolution_candidate(tx, &record, attempt, candidate_oid)
        })
    }

    #[allow(dead_code)] // Task 4 writes completion only after re-observing ref/tree state.
    pub(super) fn observe_synchronization_resolution_checkpoint(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        checkpoint_oid: git2::Oid,
        observed_tree_oid: git2::Oid,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::observe_resolution_checkpoint(
                tx,
                &record,
                attempt,
                checkpoint_oid,
                observed_tree_oid,
            )
        })
    }

    #[allow(dead_code)] // Task 3 applies only caller-confirmed identity under the lease.
    pub(super) fn begin_synchronization_identity_confirmation_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        confirmation: OperationId,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::begin_identity_confirmation_effect(tx, &record, confirmation)
        })
    }

    #[allow(dead_code)] // Task 3 records the observed configuration result after its effect.
    pub(super) fn observe_synchronization_identity_confirmation_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        confirmation: OperationId,
        applied_configuration_digest: [u8; 32],
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::observe_identity_confirmation_effect(
                tx,
                &record,
                confirmation,
                applied_configuration_digest,
            )
        })
    }

    /// Records a completed ordered merge after the child step has durably
    /// observed its guarded ref transition. Unlike a fast-forward, the merge
    /// application has its own child intent/effect journal rather than the
    /// legacy before-local-update safe point.
    pub(super) fn checkpoint_synchronization_merge_applied(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        evidence: &state::SynchronizationEvidence,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            use state::SynchronizationCheckpoint as C;
            let applied: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM remote_integration_steps WHERE operation_record_id=?1 AND phase='applied')",
                [record.id],
                |row| row.get(0),
            ).map_err(|_| state::recovery_required())?;
            if !is_sync(&record.target)
                || record.reconciliation_required
                || record.sync_checkpoint != Some(C::FetchObserved)
                || !applied
                || evidence.expected_oid != record.sync_evidence.expected_oid
                || evidence.local_oid.is_none()
            {
                return Err(state::recovery_required());
            }
            tx.execute(
                "UPDATE remote_operation_records SET phase='local_fast_forwarded',sync_checkpoint='local_fast_forwarded',local_oid=?2,tracking_oid=?3,primary_tracking_oid=?4,updated_at=max(updated_at,?5) WHERE id=?1",
                params![record.id,evidence.local_oid.map(|oid|oid.to_string()),evidence.tracking_oid.map(|oid|oid.to_string()),evidence.primary_tracking_oid.map(|oid|oid.to_string()),now()],
            ).map_err(|_| state::recovery_required())?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    pub(crate) fn checkpoint_synchronization(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        checkpoint: state::SynchronizationCheckpoint,
        evidence: &state::SynchronizationEvidence,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        use state::SynchronizationCheckpoint as C;
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target)
                || record.authority.is_some()
                || record.reconciliation_required
            {
                return Err(state::recovery_required());
            }
            let valid = match checkpoint {
                C::FetchPrepared => record.sync_checkpoint.is_none(),
                C::FetchObserved | C::DiscoveryPending => false, // only atomic batch/classification writes
                C::LocalPrepared => record.sync_checkpoint == Some(C::FetchObserved),
                C::LocalFastForwarded => {
                    record.sync_checkpoint == Some(C::LocalPrepared)
                        && record.completed_step
                            == Some(RemoteOperationSafePoint::BeforeLocalUpdate)
                }
                C::PushPrepared => matches!(
                    record.sync_checkpoint,
                    Some(C::FetchObserved | C::LocalFastForwarded)
                ),
                C::PushReturned => {
                    record.sync_checkpoint == Some(C::PushPrepared)
                        && record.completed_step == Some(RemoteOperationSafePoint::BeforePush)
                }
                C::PushVerified => matches!(
                    record.sync_checkpoint,
                    Some(
                        C::FetchObserved
                            | C::LocalFastForwarded
                            | C::PushPrepared
                            | C::PushReturned
                    )
                ),
            };
            if !valid {
                return Err(state::recovery_required());
            }
            let previous = &record.sync_evidence;
            if previous.expected_oid.is_some() && previous.expected_oid != evidence.expected_oid
                || previous.push_oid.is_some() && previous.push_oid != evidence.push_oid
                || previous.local_oid.is_some()
                    && previous.local_oid != evidence.local_oid
                    && !(checkpoint == C::LocalPrepared
                        && previous.local_oid == previous.expected_oid)
                || checkpoint != C::FetchPrepared
                    && (evidence.expected_oid.is_none()
                        || evidence.local_oid.is_none()
                        || evidence.primary_tracking_oid.is_none())
                || matches!(
                    checkpoint,
                    C::PushPrepared | C::PushReturned | C::PushVerified
                ) && (evidence.push_oid.is_none() || evidence.local_oid != evidence.push_oid)
                || checkpoint == C::PushVerified
                    && evidence.push_advertised_oid != evidence.push_oid
                || checkpoint == C::FetchPrepared
                    && (evidence.push_oid.is_some() || evidence.push_advertised_oid.is_some())
            {
                return Err(state::recovery_required());
            }
            if let Some(point) = record.completed_step {
                let decision = acknowledge(tx, &record, point)?;
                if decision != RemoteSafePointOutcome::Continue {
                    return Ok(decision);
                }
            } else if record.cancel_requested || record.yield_requested {
                return acknowledge(tx, &record, RemoteOperationSafePoint::BeforeFetch);
            }
            tx.execute("UPDATE remote_operation_records SET phase=?2,sync_checkpoint=?2,expected_oid=?3,local_oid=?4,tracking_oid=?5,primary_tracking_oid=?6,push_oid=?7,push_advertised_oid=?8,updated_at=max(updated_at,?9) WHERE id=?1",
                params![record.id,checkpoint.name(),evidence.expected_oid.map(|v|v.to_string()),evidence.local_oid.map(|v|v.to_string()),evidence.tracking_oid.map(|v|v.to_string()),evidence.primary_tracking_oid.map(|v|v.to_string()),evidence.push_oid.map(|v|v.to_string()),evidence.push_advertised_oid.map(|v|v.to_string()),now()]).map_err(|_| state::recovery_required())?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    /// Explicit restart fences the old executor but retains every unproved effect.
    /// An ordinary duplicate and any authoritative outcome can never get a token.
    pub(crate) fn restart_remote_synchronization(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if &record.target != target || !is_sync(target) {
                return Err(mismatch());
            }
            if record.authority.is_some() {
                validate_index_identity(tx, id, operation_id)?;
            } else if tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1)",
                    [operation_id.to_string()],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(|_| state::recovery_required())?
            {
                return Err(mismatch());
            }
            if record.authority.is_some() || record.phase == RemoteOperationPhase::Cancelled {
                return Ok(RemoteReservationOutcome::Replay(record.into()));
            }
            if state::has_pending_conflict(tx, id)? {
                return Err(state::recovery_required());
            }
            crate::repository::recovery::require_no_pending_local(tx, id)?;
            if record.generation != state::generation(tx, id)?
                || record.sync_checkpoint.is_none()
                    && record.completed_step == Some(RemoteOperationSafePoint::BeforeLocalMutation)
            {
                return Err(state::recovery_required());
            }
            if state::read_operation_rows(tx, id, true)?
                .iter()
                .any(|other| other.id != record.id)
            {
                return Ok(RemoteReservationOutcome::Busy);
            }
            if record.cancel_requested {
                acknowledge(
                    tx,
                    &record,
                    record
                        .completed_step
                        .unwrap_or(RemoteOperationSafePoint::BeforeFetch),
                )?;
                return Ok(RemoteReservationOutcome::Replay(
                    state::read_operation(tx, id, operation_id)?
                        .ok_or_else(state::recovery_required)?
                        .into(),
                ));
            }
            let epoch = record
                .owner_epoch
                .checked_add(1)
                .ok_or_else(state::recovery_required)?;
            tx.execute("UPDATE remote_operation_records SET phase='reconciling',reconciliation_required=1,completed_step=NULL,outcome=NULL,yield_requested=0,owner_epoch=?2,updated_at=max(updated_at,?3) WHERE id=?1",params![record.id,epoch,now()]).map_err(|_| state::recovery_required())?;
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            Ok(RemoteReservationOutcome::Reserved(token(self, id, &record)))
        })
    }

    /// Caller must freshly prove identity, cleanliness, actual ref/worktree OIDs,
    /// complete Fetch and independent Push advertisement/ancestry. Neither a fresh
    /// Fetch nor the old push intent by itself authorizes another push.
    pub(crate) fn reconcile_synchronization(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        local_oid: git2::Oid,
        worktree_oid: git2::Oid,
        push_advertised_oid: Option<git2::Oid>,
        push_is_ancestor: bool,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        use state::SynchronizationCheckpoint as C;
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target)
                || !record.reconciliation_required
                || record.phase != RemoteOperationPhase::Reconciling
                || record.completed_step != Some(RemoteOperationSafePoint::AfterFetch)
                || local_oid != worktree_oid
            {
                return Err(state::recovery_required());
            }
            let evidence = &record.sync_evidence;
            let candidate = evidence.push_oid.or(evidence.local_oid);
            if push_is_ancestor
                && (push_advertised_oid.is_none() || push_advertised_oid == candidate)
            {
                return Err(state::recovery_required());
            }
            // No effect was prepared: actual graph planning remains Task 4's job.
            let (checkpoint, phase) = if matches!(
                record.sync_checkpoint,
                None | Some(C::FetchPrepared | C::FetchObserved)
            ) {
                if evidence
                    .expected_oid
                    .is_some_and(|expected| expected != local_oid)
                {
                    return Err(state::recovery_required());
                }
                (C::FetchObserved, "fetch_observed")
            } else {
                let candidate = candidate.ok_or_else(state::recovery_required)?;
                if local_oid != candidate {
                    if record.sync_checkpoint == Some(C::LocalPrepared)
                        && evidence.expected_oid == Some(local_oid)
                    {
                        (C::FetchObserved, "fetch_observed")
                    } else {
                        return Err(state::recovery_required());
                    }
                } else if evidence.push_oid.is_some() {
                    if record.sync_checkpoint == Some(C::PushVerified)
                        && push_advertised_oid != Some(candidate)
                    {
                        return Err(state::recovery_required());
                    }
                    if push_advertised_oid == Some(candidate) {
                        (C::PushVerified, "push_verified")
                    } else if (push_advertised_oid.is_some() && push_is_ancestor)
                        || (push_advertised_oid.is_none()
                            && evidence.push_advertised_oid.is_none()
                            && !push_is_ancestor)
                    {
                        (C::PushPrepared, "push_prepared")
                    } else {
                        return Err(state::recovery_required());
                    }
                } else {
                    (C::LocalFastForwarded, "local_fast_forwarded")
                }
            };
            let decision = acknowledge(tx, &record, RemoteOperationSafePoint::AfterFetch)?;
            if decision != RemoteSafePointOutcome::Continue {
                return Ok(decision);
            }
            tx.execute("UPDATE remote_operation_records SET phase=?2,sync_checkpoint=?3,reconciliation_required=0,local_oid=?4,push_advertised_oid=?5 WHERE id=?1",
                params![record.id,phase,checkpoint.name(),local_oid.to_string(),push_advertised_oid.map(|v|v.to_string())]).map_err(|_|state::recovery_required())?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    /// Persist authority and index-only handoff together before discovery. This
    /// releases the remote slot; exact-ID replay returns inspection, not ownership.
    pub(crate) fn classify_synchronization(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        authority: state::SynchronizationAuthority,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target)
                || record.reconciliation_required
                || record.sync_checkpoint != Some(state::SynchronizationCheckpoint::PushVerified)
                || record.sync_evidence.local_oid != Some(authority.oid())
                || record.sync_evidence.push_oid != Some(authority.oid())
                || record.sync_evidence.push_advertised_oid != Some(authority.oid())
            {
                return Err(state::recovery_required());
            }
            // Verified Git authority survives cancellation/yield. Discovery is
            // non-mutating and must remain the only possible exact-ID retry.
            tx.execute("UPDATE remote_operation_records SET phase='completed',sync_checkpoint='discovery_pending',completed_step='before_discovery',authoritative_kind=?2,authoritative_oid=?3,index_pending=1,outcome='completed',updated_at=max(updated_at,?4) WHERE id=?1",
                params![record.id,authority.name(),authority.oid().to_string(),now()]).map_err(|_| state::recovery_required())?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    pub(crate) fn finish_synchronization_index(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if &record.target != target || !is_sync(target) {
                return Err(mismatch());
            }
            if record.authority.is_none() || record.phase != RemoteOperationPhase::Completed {
                return Err(state::recovery_required());
            }
            validate_index_identity(tx, id, operation_id)?;
            tx.execute("UPDATE remote_operation_records SET index_pending=0,updated_at=max(updated_at,?2) WHERE id=?1",params![record.id,now()]).map_err(|_| state::recovery_required())?;
            Ok(())
        })
    }
}
