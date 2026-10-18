//! Durable arbitration only. Tokens hold no SQLite guard, Git lease or transport.
#[cfg(test)]
#[path = "reservation_tests.rs"]
mod tests;
use std::{path::Path, sync::Arc};

use rusqlite::{OptionalExtension, Transaction, params};

use super::{
    RemoteOperationAction, RemoteOperationPhase, RemoteOperationPriority, RemoteOperationSafePoint,
    RemoteOperationTarget, RemoteOutcomeCategory, state,
};
use crate::repository::{
    OperationId, RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
    promotion::lifecycle::LifecycleKind,
};

#[cfg(test)]
type PromotionReservationHook = Box<dyn FnOnce()>;
#[cfg(test)]
thread_local! {static PROMOTION_RESERVATION_HOOKS:std::cell::RefCell<std::collections::BTreeMap<String,PromotionReservationHook>>=std::cell::RefCell::new(std::collections::BTreeMap::new());}
#[cfg(test)]
pub(in super::super) fn set_promotion_reservation_hook(
    op: OperationId,
    hook: impl FnOnce() + 'static,
) {
    PROMOTION_RESERVATION_HOOKS.with(|h| h.borrow_mut().insert(op.to_string(), Box::new(hook)));
}
#[cfg(test)]
fn promotion_reservation_checkpoint(op: OperationId) {
    if let Some(hook) = PROMOTION_RESERVATION_HOOKS.with(|h| h.borrow_mut().remove(&op.to_string()))
    {
        hook();
    }
}

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

/// Internal full-poll gate; the public poll result belongs to Task 6.
#[derive(Debug)]
pub(crate) enum RemotePollReservationOutcome {
    Reservation(RemoteReservationOutcome),
    ConflictPending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteSafePointOutcome {
    Continue,
    Interrupted,
    Cancelled,
}

/// How one fresh Push-direction advertisement relates to a recorded immutable
/// push intent. The caller computes it from actual commit ancestry; a Fetch
/// advertisement is never an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PushIntentRelation {
    Equal,
    /// The advertised commit is a strict ancestor of the intent.
    Behind,
    /// The advertised commit is a strict descendant of the intent.
    Ahead,
    Diverged,
    Absent,
}

/// The newest publication intent of a synchronization, read under ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublicationIntent {
    None,
    /// The immutable `push_oid` of the legacy envelope; no attempt follows it.
    Legacy(git2::Oid),
    /// The candidate of the newest prepared or returned publication attempt.
    Attempt(git2::Oid),
    /// The earlier intent is reconciled and no new push intent exists yet.
    Open,
    /// The newest attempt is verified; only classification remains.
    Verified(git2::Oid),
}

/// Freshly proved inputs for reconciling the newest publication intent.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PublicationSettlement {
    pub intent: PublicationIntent,
    /// Clean, validated HEAD of the target.
    pub local_oid: git2::Oid,
    /// The caller validated `local_oid` as a clean descendant of a released
    /// local checkpoint rather than the recorded intent itself.
    pub continuation: bool,
    pub advertised_oid: Option<git2::Oid>,
    /// None only for an already open attempt, which needs no observation.
    pub relation: Option<PushIntentRelation>,
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
pub(in super::super) fn owned(
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

/// A configuration fence can end this executor before its next safe point.
/// Read its terminal evidence only if scope, repository, ID and epoch still
/// identify this executor; a restarted owner is never adopted as its result.
pub(super) fn fenced_poll_record(
    service: &RepositoryService,
    tx: &Transaction<'_>,
    id: i64,
    owner: &RemoteReservation,
) -> Result<state::StoredRemoteOperation, RepositoryError> {
    let record =
        state::read_operation(tx, id, owner.operation_id)?.ok_or_else(state::recovery_required)?;
    if !Arc::ptr_eq(&service.remote_reservation_scope, &owner.scope)
        || id != owner.repository_id
        || record.generation != owner.generation
        || record.owner_epoch != owner.epoch
        || record.poll_kind != Some(state::PollKind::Update)
        || record.phase != RemoteOperationPhase::Interrupted
    {
        return Err(state::recovery_required());
    }
    Ok(record)
}

fn publication_intent(
    tx: &Transaction<'_>,
    record: &state::StoredRemoteOperation,
) -> Result<PublicationIntent, RepositoryError> {
    Ok(match state::latest_publication_attempt(tx, record)? {
        Some(attempt) => match (attempt.phase, attempt.candidate_oid) {
            (state::PublicationPhase::Open, None) => PublicationIntent::Open,
            (state::PublicationPhase::Verified, Some(candidate)) => {
                PublicationIntent::Verified(candidate)
            }
            (
                state::PublicationPhase::Prepared | state::PublicationPhase::Returned,
                Some(candidate),
            ) => PublicationIntent::Attempt(candidate),
            _ => return Err(state::recovery_required()),
        },
        None => record
            .sync_evidence
            .push_oid
            .map_or(PublicationIntent::None, PublicationIntent::Legacy),
    })
}

/// C1, C9: from `closing` until retirement a ticket context may hold its
/// closure checkpoint, which a synchronization of that context would push to
/// the remote context branch. Such a reservation or restart is `Busy`.
fn closure_holds_context(
    tx: &Transaction<'_>,
    id: i64,
    target: &RemoteOperationTarget,
) -> Result<bool, RepositoryError> {
    match target.item() {
        Some((crate::repository::AuthoringKind::Ticket, item))
            if target.action() == RemoteOperationAction::SynchronizeContext =>
        {
            crate::repository::closure::record::written_closure_unfinished(tx, id, item)
        }
        _ => Ok(false),
    }
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

/// Whether the operation's newest recorded pass holds an installed conflict or
/// an owned resolution that only this same operation can finish.
fn pending_recovery_stage(
    tx: &Transaction<'_>,
    record: &state::StoredRemoteOperation,
) -> Result<bool, RepositoryError> {
    let window = state::latest_integration_window(tx, record)?;
    for ordinal in 0..=1 {
        let Some(step) = state::integration_step_in_window(tx, record.id, window.number, ordinal)?
        else {
            continue;
        };
        if matches!(
            step.phase,
            state::IntegrationStepPhase::ConflictPending
                | state::IntegrationStepPhase::ResolutionPrepared
                | state::IntegrationStepPhase::CommitPrepared
        ) {
            return Ok(true);
        }
        // An applied resolution still owns cleanup until its existing artifact
        // and ref-release evidence is complete. Ordinary applied clean merges
        // have no attempt, and released resolutions retain terminal cancellation.
        if step.phase == state::IntegrationStepPhase::Applied
            && state::integration_has_resolution_attempt(tx, record, &step)?
            && !state::integration_resolution_released(tx, record, &step)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Ownership-first parent probe. A clean candidate still belongs to the child
/// until it releases/reconciles its integration; outer errors cannot erase it.
pub(in super::super) fn retains_promotion_child(
    db: &rusqlite::Connection,
    record: &state::StoredRemoteOperation,
) -> Result<bool, RepositoryError> {
    let window = state::latest_integration_window(db, record)?;
    for ordinal in 0..=1 {
        if let Some(step) =
            state::integration_step_in_window(db, record.id, window.number, ordinal)?
            && (step.phase != state::IntegrationStepPhase::Applied
                || state::integration_has_resolution_attempt(db, record, &step)?
                    && !state::integration_resolution_released(db, record, &step)?)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// A user's completed canonical resolution has released its local artifacts,
/// but publication still belongs to that bound synchronization. This is distinct
/// from retained (unfinished) ownership and from a no-local-work transient failure.
pub(in super::super) fn promotion_child_has_resolution(
    db: &rusqlite::Connection,
    record: &state::StoredRemoteOperation,
) -> Result<bool, RepositoryError> {
    let window = state::latest_integration_window(db, record)?;
    for ordinal in 0..=1 {
        if let Some(step) =
            state::integration_step_in_window(db, record.id, window.number, ordinal)?
            && state::integration_has_resolution_attempt(db, record, &step)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn acknowledge(
    tx: &Transaction<'_>,
    record: &state::StoredRemoteOperation,
    point: RemoteOperationSafePoint,
) -> Result<RemoteSafePointOutcome, RepositoryError> {
    if matches!(
        record.target.action(),
        RemoteOperationAction::Promote | RemoteOperationAction::Close
    ) && (record.cancel_requested || record.yield_requested)
    {
        // After F6 a promotion or closure slot excludes polls until
        // completion or explicit supersession. A recoverable stop must not
        // release it or write poll bookkeeping; the parent records the
        // cancellation between whole steps.
        tx.execute("UPDATE remote_operation_records SET phase='reserved',outcome=NULL,completed_step=?2,cancel_requested=0,yield_requested=0,updated_at=max(updated_at,?3) WHERE id=?1",params![record.id,point_name(point),now()]).map_err(|_|state::recovery_required())?;
        return Ok(if record.cancel_requested {
            RemoteSafePointOutcome::Cancelled
        } else {
            RemoteSafePointOutcome::Interrupted
        });
    }
    // A pending conflict or owned resolution is the recoverable transition a
    // cancellation must preserve: stop this owner without ending the
    // operation, so restart, reacquisition and external repair stay possible.
    if record.cancel_requested && is_sync(&record.target) && pending_recovery_stage(tx, record)? {
        tx.execute(
            "UPDATE remote_operation_records SET phase='interrupted',outcome=NULL,cancel_requested=0,updated_at=max(updated_at,?2) WHERE id=?1",
            params![record.id, now()],
        )
        .map_err(|_| state::recovery_required())?;
        return Ok(RemoteSafePointOutcome::Cancelled);
    }
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
                RemoteOperationSafePoint::BeforeLocalMutation
                | RemoteOperationSafePoint::BeforeDiscovery
                    if record.poll_kind == Some(state::PollKind::Update) =>
                {
                    "local_prepared"
                }
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
        let id = tx
            .query_row(
                "SELECT repository_id FROM remote_operation_records WHERE id=?1",
                [record.id],
                |row| row.get(0),
            )
            .map_err(|_| state::recovery_required())?;
        state::record_poll_attempt(
            tx,
            id,
            time::OffsetDateTime::from_unix_timestamp(now())
                .map_err(|_| state::recovery_required())?,
            None,
        )?;
    }
    if result != RemoteSafePointOutcome::Continue
        && record.poll_kind == Some(state::PollKind::Update)
    {
        let id = tx
            .query_row(
                "SELECT repository_id FROM remote_operation_records WHERE id=?1",
                [record.id],
                |r| r.get(0),
            )
            .map_err(|_| state::recovery_required())?;
        state::record_poll_completion(tx, id, record.operation_id, now())?;
    }
    Ok(result)
}

/// Read-only, bounded progress probe. Never migrates, takes a lease or writes.
pub(super) fn poll_progress(
    service: &RepositoryService,
    owner: &RemoteReservation,
) -> Result<RemoteSafePointOutcome, RepositoryError> {
    if !Arc::ptr_eq(&service.remote_reservation_scope, &owner.scope) {
        return Err(state::recovery_required());
    }
    let db = rusqlite::Connection::open_with_flags(
        &service.registry_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| state::recovery_required())?;
    db.busy_timeout(std::time::Duration::from_millis(250))
        .map_err(|_| state::recovery_required())?;
    let (yielded,cancelled,generation,epoch,phase,policy_generation):(bool,bool,i64,i64,String,i64)=db.query_row("SELECT o.yield_requested,o.cancel_requested,o.configuration_generation,o.owner_epoch,o.phase,p.configuration_generation FROM remote_operation_records o JOIN remote_polling_state p ON p.repository_id=o.repository_id WHERE o.repository_id=?1 AND o.operation_ulid=?2",params![owner.repository_id,owner.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|_|state::recovery_required())?;
    Ok(
        if generation != owner.generation
            || policy_generation != owner.generation
            || epoch != owner.epoch
            || !matches!(
                phase.as_str(),
                "reserved" | "advertising" | "persisting" | "local_prepared"
            )
        {
            RemoteSafePointOutcome::Interrupted
        } else if cancelled {
            RemoteSafePointOutcome::Cancelled
        } else if yielded {
            RemoteSafePointOutcome::Interrupted
        } else {
            RemoteSafePointOutcome::Continue
        },
    )
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
    commit_observation_batch_with(
        service,
        root,
        owner,
        plan,
        observations,
        observed_at,
        |_, _| Ok(()),
    )
}

/// Full poll supplies metadata writes in the same ownership-checked transaction.
/// Existing observation/synchronization callers use the no-op wrapper above.
#[allow(clippy::too_many_arguments)]
pub(super) fn commit_observation_batch_with(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    plan: &super::RemoteRefPlan,
    observations: &[super::RemoteRefObservation],
    observed_at: i64,
    write: impl FnOnce(&Transaction<'_>, i64) -> Result<(), RepositoryError>,
) -> Result<RemoteSafePointOutcome, RepositoryError> {
    commit_observation_batch_with_capture(
        service,
        root,
        owner,
        plan,
        observations,
        observed_at,
        write,
        |_, _, decision| Ok(decision),
    )
}

/// Capture an original executor's result before a terminal transition releases
/// the reservation. Existing callers keep the identity capture above.
#[allow(clippy::too_many_arguments)]
pub(super) fn commit_observation_batch_with_capture<T>(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    plan: &super::RemoteRefPlan,
    observations: &[super::RemoteRefObservation],
    observed_at: i64,
    write: impl FnOnce(&Transaction<'_>, i64) -> Result<(), RepositoryError>,
    capture: impl FnOnce(&Transaction<'_>, i64, RemoteSafePointOutcome) -> Result<T, RepositoryError>,
) -> Result<T, RepositoryError> {
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
            return capture(tx, id, decision);
        }
        if synchronization || record.poll_kind == Some(state::PollKind::Update) {
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
        write(tx, id)?;
        let decision = if synchronization {
            // Fresh Fetch evidence cannot erase an unresolved local/push checkpoint.
            tx.execute("UPDATE remote_operation_records SET phase=CASE WHEN reconciliation_required=1 THEN 'reconciling' ELSE 'fetch_observed' END,sync_checkpoint=CASE WHEN reconciliation_required=1 THEN sync_checkpoint ELSE 'fetch_observed' END WHERE id=?1",[record.id]).map_err(|_| state::recovery_required())?;
            let current = state::read_operation(tx, id, owner.operation_id)?
                .ok_or_else(state::recovery_required)?;
            acknowledge(tx, &current, RemoteOperationSafePoint::AfterFetch)
        } else {
            acknowledge(tx, &record, RemoteOperationSafePoint::AfterBatchCommit)
        }?;
        capture(tx, id, decision)
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
    let completed_at =
        time::OffsetDateTime::from_unix_timestamp(now()).map_err(|_| state::recovery_required())?;
    if !is_sync(&record.target) {
        let snapshot = state::read_snapshot(tx, id)?;
        let policy = snapshot.polling();
        let previous = policy.automatic_backoff();
        let next = super::poll::next_eligible(
            category,
            if record.priority == RemoteOperationPriority::Poll {
                super::RemotePollInvocation::Automatic
            } else {
                super::RemotePollInvocation::Explicit
            },
            completed_at,
            policy,
        );
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
        state::record_poll_attempt(tx, id, completed_at, next)?;
    }
    tx.execute("UPDATE remote_operation_records SET phase=?2,outcome=?3,updated_at=max(updated_at,?4) WHERE id=?1",
        params![record.id,phase,state::outcome_name(category),completed_at.unix_timestamp()]).map_err(|_| state::recovery_required())?;
    if record.poll_kind == Some(state::PollKind::Update) {
        state::record_poll_completion(tx, id, record.operation_id, completed_at.unix_timestamp())?;
    }
    Ok(())
}

/// SQL-only finalization. Retirement and parent completion belong to this same
/// caller-owned transaction; promotion never updates automatic poll policy.
pub(in super::super) fn finish_parent(
    kind: LifecycleKind,
    service: &RepositoryService,
    tx: &Transaction<'_>,
    id: i64,
    owner: &RemoteReservation,
) -> Result<(), RepositoryError> {
    let record = owned(service, tx, id, owner)?;
    if record.target.action() != kind.action()
        || record.priority != RemoteOperationPriority::Manual
        || record.completed_step != Some(RemoteOperationSafePoint::AfterAdvertisement)
    {
        return Err(state::recovery_required());
    }
    // The parent is complete and its own kind's context is retired at the
    // merged context: promotion's `context_oid`; a closure's adopted
    // synchronization, else its checkpoint, else its synchronized context.
    let complete: bool = match kind {
        LifecycleKind::DocumentPromotion => tx.query_row("SELECT EXISTS(SELECT 1 FROM document_promotions p JOIN retired_contexts r ON r.repository_id=p.repository_id AND r.item_id=p.item_id AND r.kind='document' AND r.operation_id=p.operation_id AND r.retired_oid=p.context_oid WHERE p.repository_id=?1 AND p.operation_id=?2 AND p.step='complete')",params![id,owner.operation_id.to_string()],|r|r.get(0)),
        LifecycleKind::TicketClosure => tx.query_row("SELECT EXISTS(SELECT 1 FROM ticket_closures c JOIN retired_contexts r ON r.repository_id=c.repository_id AND r.item_id=c.item_id AND r.kind='ticket' AND r.operation_id=c.operation_id AND r.retired_oid=coalesce(c.adopted_context_oid,c.closure_oid,c.context_oid) WHERE c.repository_id=?1 AND c.operation_id=?2 AND c.step='complete')",params![id,owner.operation_id.to_string()],|r|r.get(0)),
    }
    .map_err(|_| state::recovery_required())?;
    if !complete {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE remote_operation_records SET phase='completed',outcome='completed',cancel_requested=0,yield_requested=0,updated_at=max(updated_at,?2) WHERE id=?1",params![record.id,now()]).map_err(|_|state::recovery_required())?;
    Ok(())
}

impl RepositoryService {
    /// Test-only composition bridge for parent ownership-policy cases. It uses
    /// the real child journal, without exposing synchronization internals to the
    /// production promotion runner.
    #[cfg(test)]
    pub(in super::super) fn prepare_promotion_test_child(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        primary: bool,
        local: git2::Oid,
        incoming: git2::Oid,
    ) -> Result<(), RepositoryError> {
        let repository = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
        self.prepare_synchronization_integration(
            root,
            owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: if primary {
                    super::merge::IntegrationStage::Primary
                } else {
                    super::merge::IntegrationStage::Context
                },
                local_oid: local,
                incoming_oid: incoming,
                baseline_tree_oid: repository
                    .find_commit(local)
                    .map_err(|_| state::recovery_required())?
                    .tree_id(),
                baseline_index_digest: [7; 32],
            },
        )?;
        Ok(())
    }
    #[allow(dead_code)] // Composed by the Task 6 promotion runner.
    pub(crate) fn restart_remote_promotion(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        self.restart_remote_lifecycle(LifecycleKind::DocumentPromotion, root, operation_id, target)
    }
    pub(in super::super) fn restart_remote_lifecycle(
        &self,
        kind: LifecycleKind,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            crate::repository::promotion::record::validate_remote_identity(
                tx,
                id,
                operation_id,
                target,
                RemoteOperationPriority::Manual,
            )?;
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if record.target != *target
                || target.action() != kind.action()
                || record.priority != RemoteOperationPriority::Manual
            {
                return Err(mismatch());
            }
            // A refused or replaced parent of this kind never restarts its slot.
            if crate::repository::promotion::lifecycle_record::ParentFacts::read(
                tx,
                kind,
                id,
                operation_id,
            )?
            .is_some_and(|parent| parent.ended())
            {
                return Err(state::recovery_required());
            }
            if record.phase == RemoteOperationPhase::Completed {
                return Ok(RemoteReservationOutcome::Replay(record.into()));
            }
            if record.generation != state::generation(tx, id)? {
                return Err(state::recovery_required());
            }
            crate::repository::recovery::require_no_pending_local(tx, id)?;
            if let Some(other) = state::read_operation_rows(tx, id, true)?
                .into_iter()
                .find(|other| other.id != record.id)
            {
                if other.priority == RemoteOperationPriority::Poll {
                    tx.execute("UPDATE remote_operation_records SET yield_requested=1,updated_at=max(updated_at,?2) WHERE id=?1",params![other.id,now()]).map_err(|_|state::recovery_required())?;
                    return Ok(RemoteReservationOutcome::PollYielding);
                }
                return Ok(RemoteReservationOutcome::Busy);
            }
            let epoch = record
                .owner_epoch
                .checked_add(1)
                .ok_or_else(state::recovery_required)?;
            tx.execute("UPDATE remote_operation_records SET phase='reserved',completed_step=NULL,outcome=NULL,cancel_requested=0,yield_requested=0,owner_epoch=?2,updated_at=max(updated_at,?3) WHERE id=?1",params![record.id,epoch,now()]).map_err(|_|state::recovery_required())?;
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            Ok(RemoteReservationOutcome::Reserved(token(self, id, &record)))
        })
    }
    #[allow(dead_code)] // Full-poll orchestration is introduced in Task 6.
    pub(crate) fn reserve_remote_poll(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        priority: RemoteOperationPriority,
    ) -> Result<RemotePollReservationOutcome, RepositoryError> {
        if target.action() != RemoteOperationAction::Poll {
            return Err(mismatch());
        }
        self.reserve_remote_operation_kind(
            root,
            operation_id,
            target,
            priority,
            Some(state::PollKind::Update),
            None,
        )
    }

    #[allow(dead_code)] // Full-poll orchestration is introduced in Task 6.
    pub(crate) fn restart_remote_poll(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        if target.action() != RemoteOperationAction::Poll {
            return Err(mismatch());
        }
        self.restart_remote_operation_kind(root, operation_id, target, state::PollKind::Update)
    }

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
        let kind =
            (target.action() == RemoteOperationAction::Poll).then_some(state::PollKind::Observe);
        match self.reserve_remote_operation_kind(
            root,
            operation_id,
            target,
            priority,
            kind,
            None,
        )? {
            RemotePollReservationOutcome::Reservation(outcome) => Ok(outcome),
            RemotePollReservationOutcome::ConflictPending => Err(state::recovery_required()),
        }
    }

    pub(in super::super) fn reserve_promotion_child(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        execution: &crate::repository::promotion::record::ChildExecution,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        match self.reserve_remote_operation_kind(
            root,
            operation_id,
            target,
            RemoteOperationPriority::Manual,
            None,
            Some(execution),
        )? {
            RemotePollReservationOutcome::Reservation(outcome) => Ok(outcome),
            RemotePollReservationOutcome::ConflictPending => Err(state::recovery_required()),
        }
    }

    fn reserve_remote_operation_kind(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        priority: RemoteOperationPriority,
        poll_kind: Option<state::PollKind>,
        execution: Option<&crate::repository::promotion::record::ChildExecution>,
    ) -> Result<RemotePollReservationOutcome, RepositoryError> {
        if priority == RemoteOperationPriority::Poll
            && target.action() != RemoteOperationAction::Poll
        {
            return Err(mismatch());
        }
        #[cfg(test)]
        promotion_reservation_checkpoint(operation_id);
        state::with_transaction(self, root, |tx, repository_id| {
            if let Some(execution) = execution {
                execution.validate(tx, repository_id, operation_id)?;
            }
            crate::repository::promotion::record::validate_remote_identity(
                tx,
                repository_id,
                operation_id,
                target,
                priority,
            )?;
            let binding = tx.query_row("SELECT b.kind,b.item_ulid,b.checkpoint_oid,o.root_path,o.state,o.completed_step FROM comment_publication_bindings b
                JOIN operation_records o ON o.id=b.local_record_id WHERE b.synchronization_ulid=?1", [operation_id.to_string()],
                |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?,r.get::<_, Option<String>>(2)?,r.get::<_, String>(3)?,r.get::<_, String>(4)?,r.get::<_, Option<String>>(5)?))).optional().map_err(|_| state::recovery_required())?;
            if let Some((kind, item, checkpoint, parent_root, parent_state, parent_step)) = binding
            {
                let registered_root: String = tx
                    .query_row(
                        "SELECT root_path FROM repositories WHERE id=?1",
                        [repository_id],
                        |r| r.get(0),
                    )
                    .map_err(|_| state::recovery_required())?;
                let identity_matches = target.item().is_some_and(|(target_kind, target_item)| {
                    crate::repository::authoring_kind_segment(&target_kind) == kind
                        && target_item.to_string() == item
                });
                if parent_root != registered_root
                    || target.action() != RemoteOperationAction::SynchronizeContext
                    || !identity_matches
                {
                    return Err(mismatch());
                }
                // The submission this synchronization is of was abandoned.
                // Its row is under another ID than the one reserved here,
                // so the refusal below, which reads the row of this ID,
                // does not see it. A replay or a retry that read its
                // binding before the abandon comes here next, and is
                // stopped in the transaction that would reserve: nothing
                // is begun on the remote for it.
                if parent_state == "completed"
                    && crate::repository::recovery::is_abandoned_step(parent_step.as_deref())
                {
                    return Err(crate::repository::recovery::abandoned(
                        RepositoryOperation::SubmitComment,
                        root,
                    ));
                }
                if checkpoint.is_none() {
                    return Err(state::recovery_required());
                }
            }
            // The last step of the local row under the ID, when there is
            // one.
            let local: Option<Option<String>> = tx
                .query_row(
                    "SELECT completed_step FROM operation_records WHERE operation_ulid=?1 ORDER BY id LIMIT 1",
                    [operation_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| state::recovery_required())?;
            // An ID whose local operation was abandoned is closed for
            // good: nothing is reserved or replayed under it, so no
            // remote work is begun whose index hand-off, a local row
            // under the same ID, would then be refused.
            if local
                .as_ref()
                .is_some_and(|step| crate::repository::recovery::is_abandoned_step(step.as_deref()))
            {
                return Err(crate::repository::recovery::abandoned(
                    RepositoryOperation::RepositorySnapshot,
                    root,
                ));
            }
            let local_id = local.is_some();
            let other_root: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",params![operation_id.to_string(), repository_id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
            if is_sync(target) && other_root {
                return Err(mismatch());
            }
            if let Some(record) = state::read_operation(tx, repository_id, operation_id)? {
                if &record.target != target
                    || record.priority != priority
                    || record.poll_kind != poll_kind
                {
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
                return Ok(RemotePollReservationOutcome::Reservation(
                    RemoteReservationOutcome::Replay(record.into()),
                ));
            }
            if local_id {
                return Err(mismatch());
            }
            crate::repository::recovery::require_no_pending_local(tx, repository_id)?;
            if closure_holds_context(tx, repository_id, target)? {
                return Ok(RemotePollReservationOutcome::Reservation(
                    RemoteReservationOutcome::Busy,
                ));
            }
            if poll_kind == Some(state::PollKind::Update)
                && state::has_pending_conflict(tx, repository_id)?
            {
                return Ok(RemotePollReservationOutcome::ConflictPending);
            }
            if is_sync(target) && state::has_pending_conflict(tx, repository_id)? {
                return Ok(RemotePollReservationOutcome::Reservation(
                    RemoteReservationOutcome::Busy,
                ));
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
                    return Ok(RemotePollReservationOutcome::Reservation(
                        RemoteReservationOutcome::PollYielding,
                    ));
                }
                return Ok(RemotePollReservationOutcome::Reservation(
                    RemoteReservationOutcome::Busy,
                ));
            }
            if poll_kind == Some(state::PollKind::Update) {
                state::insert_operation_with_kind(
                    tx,
                    repository_id,
                    operation_id,
                    target,
                    priority,
                    poll_kind,
                    now(),
                )?;
            } else {
                state::insert_operation(tx, repository_id, operation_id, target, priority, now())?;
            }
            let record = state::read_operation(tx, repository_id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            Ok(RemotePollReservationOutcome::Reservation(
                RemoteReservationOutcome::Reserved(token(self, repository_id, &record)),
            ))
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

    /// Read-only correlation adapter: existence and historical authority, never
    /// a reservation token or permission to restart.
    pub(crate) fn comment_child_authority(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<Option<bool>, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            Ok(state::read_operation(tx, id, operation_id)?
                .map(|record| record.authority.is_some()))
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

    /// The cancellation an executor asks of its own operation: recorded
    /// as `cancel_remote_operation` records one, and only while `owner`
    /// still owns the operation. The next safe point honors it.
    pub(crate) fn request_synchronization_cancellation(
        &self,
        root: &Path,
        owner: &RemoteReservation,
    ) -> Result<(), RepositoryError> {
        self.check_failure(
            crate::repository::FailurePoint::BeforeSynchronizationCancellation,
            RepositoryOperation::RepositorySnapshot,
            root,
        )?;
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            tx.execute("UPDATE remote_operation_records SET cancel_requested=1,updated_at=max(updated_at,?2) WHERE id=?1",params![record.id,now()]).map_err(|_| state::recovery_required())?;
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
        self.remote_safe_point_with(root, owner, point, |_, _, decision| Ok(decision))
    }

    pub(super) fn remote_safe_point_with<T>(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        point: RemoteOperationSafePoint,
        capture: impl FnOnce(
            &Transaction<'_>,
            i64,
            RemoteSafePointOutcome,
        ) -> Result<T, RepositoryError>,
    ) -> Result<T, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if is_sync(&record.target) {
                if let Some(attempt) = state::latest_publication_attempt(tx, &record)? {
                    // The legacy push checkpoint is frozen behind this attempt
                    // and is never replayed; push boundaries follow the
                    // attempt's own forward-only phase instead.
                    use RemoteOperationSafePoint as P;
                    use state::PublicationPhase as A;
                    let valid = match point {
                        P::BeforeFetch => record.phase == RemoteOperationPhase::Reconciling,
                        P::AfterFetch => record.completed_step == Some(P::AfterFetch),
                        P::BeforePush => {
                            !record.reconciliation_required && attempt.phase == A::Prepared
                        }
                        P::AfterPushReturn => {
                            !record.reconciliation_required && attempt.phase == A::Returned
                        }
                        P::AfterPushVerification => {
                            !record.reconciliation_required && attempt.phase == A::Verified
                        }
                        _ => false,
                    };
                    if !valid {
                        return Err(state::recovery_required());
                    }
                } else {
                    validate_sync_point(&record, point)?;
                }
            } else if record.poll_kind == Some(state::PollKind::Update) {
                use RemoteOperationSafePoint as P;
                let valid = match point {
                    P::BeforeTransport
                    | P::AfterAdvertisement
                    | P::BetweenObservations
                    | P::BeforeBatchCommit => true,
                    P::AfterBatchCommit => record.completed_step == Some(P::AfterBatchCommit),
                    P::BeforeLocalMutation => matches!(
                        record.completed_step,
                        Some(P::AfterBatchCommit | P::BeforeLocalMutation)
                    ),
                    P::BeforeDiscovery => matches!(
                        record.completed_step,
                        Some(P::AfterBatchCommit | P::BeforeLocalMutation)
                    ),
                    _ => false,
                };
                if !valid {
                    return Err(state::recovery_required());
                }
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
            let decision = acknowledge(tx, &record, point)?;
            capture(tx, id, decision)
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
        self.restart_remote_operation_kind(root, operation_id, target, state::PollKind::Observe)
    }

    fn restart_remote_operation_kind(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        poll_kind: state::PollKind,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if &record.target != target
                || target.action() == RemoteOperationAction::Poll
                    && record.poll_kind != Some(poll_kind)
            {
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
                || poll_kind == state::PollKind::Observe
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
            } else if poll_kind == state::PollKind::Observe
                && record.completed_step == Some(RemoteOperationSafePoint::AfterBatchCommit)
            {
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
        self.finish_remote_operation_with(root, owner, category, |_, _, decision| Ok(decision))
    }

    pub(super) fn finish_remote_operation_with<T>(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        category: RemoteOutcomeCategory,
        capture: impl FnOnce(
            &Transaction<'_>,
            i64,
            RemoteSafePointOutcome,
        ) -> Result<T, RepositoryError>,
    ) -> Result<T, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            let point = record.completed_step.ok_or_else(state::recovery_required)?;
            let completion_point = if record.poll_kind == Some(state::PollKind::Update) {
                RemoteOperationSafePoint::BeforeDiscovery
            } else {
                RemoteOperationSafePoint::AfterBatchCommit
            };
            if !(record.target.action() == RemoteOperationAction::Poll || is_sync(&record.target))
                || category == RemoteOutcomeCategory::Completed
                    && (is_sync(&record.target) || point != completion_point)
            {
                return Err(state::recovery_required());
            }
            let decision = acknowledge(tx, &record, point)?;
            if decision != RemoteSafePointOutcome::Continue {
                return capture(tx, id, decision);
            }
            finish(tx, id, &record, category)?;
            let decision = if category == RemoteOutcomeCategory::Cancelled {
                RemoteSafePointOutcome::Cancelled
            } else {
                RemoteSafePointOutcome::Continue
            };
            capture(tx, id, decision)
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
    #[cfg(test)] // Window-zero form, used by the journal tests only.
    pub(super) fn prepare_synchronization_integration(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        intent: &state::IntegrationStepIntent,
    ) -> Result<state::IntegrationStepEvidence, RepositoryError> {
        self.prepare_synchronization_integration_in_window(root, owner, 0, intent)
    }

    /// Append-only pass evidence. Git identity, clean/continuation ancestry and
    /// the exact advertised/tracking pass must be observed by the caller; this
    /// transition only fences and persists that intent, never proves an effect.
    pub(super) fn prepare_synchronization_window(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        number: u32,
        intent: &state::IntegrationWindowIntent,
    ) -> Result<state::IntegrationWindowEvidence, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if record.cancel_requested || record.yield_requested {
                return Err(state::recovery_required());
            }
            state::prepare_integration_window(tx, &record, number, intent)
        })
    }

    pub(super) fn prepare_synchronization_integration_in_window(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window_number: u32,
        intent: &state::IntegrationStepIntent,
    ) -> Result<state::IntegrationStepEvidence, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if window_number == 0 {
                state::prepare_integration_step(tx, &record, intent)
            } else {
                if record.cancel_requested || record.yield_requested {
                    return Err(state::recovery_required());
                }
                state::prepare_integration_step_in_window(tx, &record, window_number, intent)
            }
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
            let window = state::latest_integration_window(tx, &record)?;
            for ordinal in (0..=1).rev() {
                if let Some(step) =
                    state::integration_step_in_window(tx, record.id, window.number, ordinal)?
                {
                    if step.phase == state::IntegrationStepPhase::Applied
                        && state::integration_has_resolution_attempt(tx, &record, &step)?
                        && !state::integration_resolution_released(tx, &record, &step)?
                    {
                        return Err(state::recovery_required());
                    }
                    if step.phase == state::IntegrationStepPhase::Applied
                        && record.sync_evidence.push_oid.is_some()
                        && state::latest_publication_attempt(tx, &record)?.is_none()
                        && step.result_oid != record.sync_evidence.local_oid
                    {
                        // The frozen legacy envelope must still describe this
                        // stage. Once an append-only publication attempt
                        // exists, the newest window is the local authority.
                        return Err(state::recovery_required());
                    }
                    // A recorded Push intent is never cleared or rewritten by
                    // this local observation; the caller routes it to
                    // publication reconciliation instead of local handoff.
                    // Never fall back to an earlier applied context while the
                    // primary has an unfinished effect or owned resolution.
                    return Ok(Some(step));
                }
            }
            Ok(None)
        })
    }

    /// Commit the already-observed child application into the legacy envelope.
    /// This is the only restart transition permitted for a pending candidate.
    #[cfg(test)] // Window-zero form, used by the journal tests only.
    pub(super) fn reconcile_synchronization_candidate_applied(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        candidate: git2::Oid,
        tree: git2::Oid,
        evidence: &state::SynchronizationEvidence,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        self.reconcile_synchronization_candidate_applied_in_window(
            root, owner, 0, ordinal, candidate, tree, evidence,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn reconcile_synchronization_candidate_applied_in_window(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window_number: u32,
        ordinal: u8,
        candidate: git2::Oid,
        tree: git2::Oid,
        evidence: &state::SynchronizationEvidence,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            use state::SynchronizationCheckpoint as C;
            let step = state::integration_step_in_window(tx, record.id, window_number, ordinal)?
                .ok_or_else(state::recovery_required)?;
            let continuation = evidence.local_oid != Some(candidate);
            if continuation
                && (evidence.local_oid.is_none()
                    || step.intent.stage != super::merge::IntegrationStage::Primary
                    || step.candidate_oid != Some(candidate)
                    || !state::integration_resolution_released(tx, &record, &step)?)
            {
                return Err(state::recovery_required());
            }
            if !is_sync(&record.target)
                || !record.reconciliation_required
                || !matches!(
                    record.sync_checkpoint,
                    Some(C::FetchObserved | C::LocalFastForwarded)
                )
                || record.completed_step != Some(RemoteOperationSafePoint::AfterFetch)
                || record.sync_evidence.push_oid.is_some()
                || step.phase != state::IntegrationStepPhase::Applied
                || step.candidate_oid.is_some_and(|oid| oid != candidate)
                || (state::integration_has_resolution_attempt(tx, &record, &step)?
                    && !state::integration_resolution_released(tx, &record, &step)?)
                || step.result_oid != Some(candidate)
                || step.observed_tree_oid != Some(tree)
                || evidence.expected_oid != record.sync_evidence.expected_oid
                || evidence.primary_tracking_oid.is_none()
                || state::latest_integration_window(tx, &record)?.number != window_number
            {
                return Err(state::recovery_required());
            }
            tx.execute(
                "UPDATE remote_operation_records SET phase='local_fast_forwarded', sync_checkpoint='local_fast_forwarded', reconciliation_required=0, local_oid=?2, tracking_oid=?3, primary_tracking_oid=?4, updated_at=max(updated_at,?5) WHERE id=?1",
                params![record.id, evidence.local_oid.map(|oid| oid.to_string()), evidence.tracking_oid.map(|oid| oid.to_string()), evidence.primary_tracking_oid.map(|oid| oid.to_string()), now()],
            ).map_err(|_| state::recovery_required())?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    pub(super) fn observe_external_synchronization_integration(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        step: &state::IntegrationStepEvidence,
        candidate: git2::Oid,
        tree: git2::Oid,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if record.cancel_requested || record.yield_requested || !record.reconciliation_required
            {
                return Err(state::recovery_required());
            }
            state::observe_external_integration(
                tx,
                &record,
                step.window_number,
                step.intent.ordinal,
                candidate,
                tree,
            )
        })
    }

    pub(super) fn release_inspected_synchronization_conflict(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        step: &state::IntegrationStepEvidence,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if state::integration_step_in_window(
                tx,
                record.id,
                step.window_number,
                step.intent.ordinal,
            )?
            .as_ref()
                != Some(step)
                || step.phase != state::IntegrationStepPhase::ConflictPending
            {
                return Err(state::recovery_required());
            }
            tx.execute(
                "UPDATE remote_operation_records SET phase='interrupted',outcome=NULL,cancel_requested=0 WHERE id=?1",
                [record.id],
            )
            .map_err(|_| state::recovery_required())?;
            Ok(())
        })
    }

    #[cfg(test)] // Window-zero form, used by the journal tests only.
    pub(super) fn begin_synchronization_integration_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        candidate_oid: Option<git2::Oid>,
    ) -> Result<(), RepositoryError> {
        self.begin_synchronization_integration_effect_in_window(
            root,
            owner,
            0,
            ordinal,
            candidate_oid,
        )
    }

    pub(super) fn begin_synchronization_integration_effect_in_window(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window_number: u32,
        ordinal: u8,
        candidate_oid: Option<git2::Oid>,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if window_number == 0 {
                state::begin_integration_effect(tx, &record, ordinal, candidate_oid)
            } else {
                if record.cancel_requested || record.yield_requested {
                    return Err(state::recovery_required());
                }
                state::begin_integration_effect_in_window(
                    tx,
                    &record,
                    window_number,
                    ordinal,
                    candidate_oid,
                )
            }
        })
    }

    #[cfg(test)] // Window-zero form, used by the journal tests only.
    pub(super) fn observe_synchronization_integration_effect(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        result_oid: git2::Oid,
        observed_tree_oid: git2::Oid,
    ) -> Result<(), RepositoryError> {
        self.observe_synchronization_integration_effect_in_window(
            root,
            owner,
            0,
            ordinal,
            result_oid,
            observed_tree_oid,
        )
    }

    pub(super) fn observe_synchronization_integration_effect_in_window(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window_number: u32,
        ordinal: u8,
        result_oid: git2::Oid,
        observed_tree_oid: git2::Oid,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if window_number == 0 {
                state::observe_integration_effect(
                    tx,
                    &record,
                    ordinal,
                    result_oid,
                    observed_tree_oid,
                )
            } else {
                // Observation must remain durable even if cancellation arrived
                // after the effect. The next mutation boundary handles it.
                state::observe_integration_effect_in_window(
                    tx,
                    &record,
                    window_number,
                    ordinal,
                    result_oid,
                    observed_tree_oid,
                )
            }
        })
    }

    /// A conflict is observed after the Git effect and then releases only the
    /// active slot. The immutable operation/stage remains for explicit matching
    /// reconciliation; another synchronization cannot adopt it.
    #[cfg(test)] // Window-zero form, used by the journal tests only.
    pub(super) fn release_synchronization_conflict(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        ordinal: u8,
        conflict_digest: [u8; 32],
    ) -> Result<(), RepositoryError> {
        self.release_synchronization_conflict_in_window(root, owner, 0, ordinal, conflict_digest)
    }

    pub(super) fn release_synchronization_conflict_in_window(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window_number: u32,
        ordinal: u8,
        conflict_digest: [u8; 32],
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if window_number == 0 {
                state::record_integration_conflict(tx, &record, ordinal, conflict_digest)?;
            } else {
                state::record_integration_conflict_in_window(
                    tx,
                    &record,
                    window_number,
                    ordinal,
                    conflict_digest,
                )?;
            }
            tx.execute(
                "UPDATE remote_operation_records SET phase='interrupted',outcome=NULL,cancel_requested=0,updated_at=max(updated_at,?2) WHERE id=?1 AND owner_epoch=?3",
                params![record.id, now(), owner.epoch],
            )
            .map_err(|_| state::recovery_required())?;
            Ok(())
        })
    }

    /// Reacquisition is deliberately separate from ordinary restart: it proves
    /// the exact durable operation, target, generation and conflict digest.
    #[cfg(test)] // Window-zero form, used by the journal tests only.
    pub(super) fn reacquire_synchronization_conflict(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        ordinal: u8,
        conflict_digest: [u8; 32],
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        self.reacquire_synchronization_conflict_in_window(
            root,
            operation_id,
            target,
            0,
            ordinal,
            conflict_digest,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn reacquire_synchronization_conflict_in_window(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        window_number: u32,
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
            let step = state::integration_step_in_window(tx, record.id, window_number, ordinal)?
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
            // Like both conflict releases, completion consumes a cancellation
            // requested while this owner ran: left set, the next restart would
            // end the operation with its resolution commit unpublished.
            tx.execute(
                "UPDATE remote_operation_records SET phase='interrupted',outcome=NULL,cancel_requested=0,updated_at=max(updated_at,?2) WHERE id=?1 AND owner_epoch=?3",
                params![record.id, now(), owner.epoch],
            ).map_err(|_| state::recovery_required())?;
            Ok(())
        })
    }

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

    #[cfg(test)] // Window-zero form, used by the journal tests only.
    pub(super) fn prepare_synchronization_resolution_attempt(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        intent: &state::ResolutionAttemptIntent,
        paths: &[state::ResolutionPathIntent],
    ) -> Result<(), RepositoryError> {
        self.prepare_synchronization_resolution_attempt_in_window(root, owner, 0, intent, paths)
            .map(|_| ())
    }

    pub(super) fn prepare_synchronization_resolution_attempt_in_window(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window_number: u32,
        intent: &state::ResolutionAttemptIntent,
        paths: &[state::ResolutionPathIntent],
    ) -> Result<(i64, i32), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::prepare_resolution_attempt_in_window(tx, &record, window_number, intent, paths)
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
            let window = state::latest_integration_window(tx, &record)?;
            let ordinal = u8::from(record.target.context_ref().is_some());
            let applied = state::integration_step_in_window(tx, record.id, window.number, ordinal)?
                .is_some_and(|step| {
                    step.phase == state::IntegrationStepPhase::Applied
                        && step.result_oid == evidence.local_oid
                });
            if state::latest_publication_attempt(tx, &record)?
                .is_some_and(|attempt| attempt.phase == state::PublicationPhase::Open)
            {
                // The legacy checkpoint and local OID stay frozen behind the
                // old Push intent. The applied child stage is the durable
                // record; binding the publication candidate rechecks it.
                if !is_sync(&record.target)
                    || record.reconciliation_required
                    || record.authority.is_some()
                    || !applied
                    || evidence.expected_oid != record.sync_evidence.expected_oid
                {
                    return Err(state::recovery_required());
                }
                return Ok(if record.cancel_requested || record.yield_requested {
                    acknowledge(
                        tx,
                        &record,
                        record
                            .completed_step
                            .unwrap_or(RemoteOperationSafePoint::BeforeFetch),
                    )?
                } else {
                    RemoteSafePointOutcome::Continue
                });
            }
            if !is_sync(&record.target)
                || record.reconciliation_required
                || !matches!(
                    record.sync_checkpoint,
                    Some(C::FetchObserved | C::LocalFastForwarded)
                )
                || record.sync_evidence.push_oid.is_some()
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
            // Once a publication attempt exists the legacy envelope is frozen:
            // no legacy checkpoint may move it, even with unchanged evidence.
            if !is_sync(&record.target)
                || record.authority.is_some()
                || record.reconciliation_required
                || state::latest_publication_attempt(tx, &record)?.is_some()
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
            let record_id = record.id;
            let write = || {
                tx.execute("UPDATE remote_operation_records SET phase=?2,sync_checkpoint=?2,expected_oid=?3,local_oid=?4,tracking_oid=?5,primary_tracking_oid=?6,push_oid=?7,push_advertised_oid=?8,updated_at=max(updated_at,?9) WHERE id=?1",
                    params![record_id,checkpoint.name(),evidence.expected_oid.map(|v|v.to_string()),evidence.local_oid.map(|v|v.to_string()),evidence.tracking_oid.map(|v|v.to_string()),evidence.primary_tracking_oid.map(|v|v.to_string()),evidence.push_oid.map(|v|v.to_string()),evidence.push_advertised_oid.map(|v|v.to_string()),now()]).map_err(|_| state::recovery_required())
            };
            // That the push returned is an effect that has happened, and
            // is recorded before a requested cancellation or yield is
            // acknowledged: the operation may then end, and its row says
            // the remote took its push. Every other checkpoint is an
            // intent or a local step, and a request is honored before it
            // is written.
            let record = if checkpoint == C::PushReturned {
                write()?;
                state::read_operation(tx, id, owner.operation_id)?
                    .ok_or_else(state::recovery_required)?
            } else {
                record
            };
            if let Some(point) = record.completed_step {
                let decision = acknowledge(tx, &record, point)?;
                if decision != RemoteSafePointOutcome::Continue {
                    return Ok(decision);
                }
            } else if record.cancel_requested || record.yield_requested {
                return acknowledge(tx, &record, RemoteOperationSafePoint::BeforeFetch);
            }
            if checkpoint != C::PushReturned {
                write()?;
            }
            if checkpoint == C::PushVerified {
                state::consume_retirement_on_publication(tx, &record)?;
            }
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
        self.restart_synchronization_guarded(root, operation_id, target, None)
    }
    pub(in super::super) fn restart_promotion_child(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        execution: &crate::repository::promotion::record::ChildExecution,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        self.restart_synchronization_guarded(root, operation_id, target, Some(execution))
    }
    fn restart_synchronization_guarded(
        &self,
        root: &Path,
        operation_id: OperationId,
        target: &RemoteOperationTarget,
        execution: Option<&crate::repository::promotion::record::ChildExecution>,
    ) -> Result<RemoteReservationOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            if let Some(execution) = execution {
                execution.validate(tx, id, operation_id)?;
            }
            let record = state::read_operation(tx, id, operation_id)?
                .ok_or_else(state::recovery_required)?;
            if &record.target != target || !is_sync(target) {
                return Err(mismatch());
            }
            crate::repository::promotion::record::validate_remote_identity(
                tx,
                id,
                operation_id,
                target,
                record.priority,
            )?;
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
            if closure_holds_context(tx, id, target)? {
                return Ok(RemoteReservationOutcome::Busy);
            }
            let foreign_conflict: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM remote_integration_steps step JOIN remote_operation_records operation ON operation.id=step.operation_record_id WHERE operation.repository_id=?1 AND operation.id!=?2 AND step.phase IN ('conflict_pending','resolution_prepared','commit_prepared'))",
                params![id, record.id], |row| row.get(0),
            ).map_err(|_| state::recovery_required())?;
            if foreign_conflict {
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
                // A frozen legacy envelope is reconciled only through its
                // append-only publication attempts.
                || state::latest_publication_attempt(tx, &record)?.is_some()
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
            if checkpoint == C::PushVerified {
                state::consume_retirement_on_publication(tx, &record)?;
            }
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    /// Read the newest publication intent under ownership. The legacy Push
    /// intent and every earlier attempt are immutable history.
    pub(crate) fn synchronization_publication_intent(
        &self,
        root: &Path,
        owner: &RemoteReservation,
    ) -> Result<PublicationIntent, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target) {
                return Err(state::recovery_required());
            }
            publication_intent(tx, &record)
        })
    }

    /// Reconcile the newest immutable push intent against fresh, independent
    /// Push-direction evidence and leave restart reconciliation.
    ///
    /// The caller must have completed Fetch, proved the target clean at
    /// `local_oid`, and computed `relation` from a receive-pack advertisement
    /// and actual ancestry. An intent that is equal-and-current stays on the
    /// legacy route; everything else appends one open publication attempt that
    /// records the fixed disposition. Nothing here rewrites `push_oid`, an
    /// earlier attempt, the legacy checkpoint or the legacy local OID, and
    /// nothing authorizes a push: the new attempt has no candidate yet.
    pub(crate) fn settle_synchronization_publication(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        settlement: &PublicationSettlement,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        use state::PublicationDisposition as D;
        use state::SynchronizationCheckpoint as C;
        enum Action {
            Resume,
            Verify,
            Open(git2::Oid, D),
        }
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            let Some(checkpoint) = record.sync_checkpoint else {
                return Err(state::recovery_required());
            };
            if !is_sync(&record.target)
                || !record.reconciliation_required
                || record.phase != RemoteOperationPhase::Reconciling
                || record.completed_step != Some(RemoteOperationSafePoint::AfterFetch)
                || record.authority.is_some()
                || record.sync_evidence.push_oid.is_none()
                || !matches!(
                    checkpoint,
                    C::PushPrepared | C::PushReturned | C::PushVerified
                )
                || publication_intent(tx, &record)? != settlement.intent
            {
                return Err(state::recovery_required());
            }
            let window = state::latest_integration_window(tx, &record)?;
            let ordinal = u8::from(record.target.context_ref().is_some());
            let last = state::integration_step_in_window(tx, record.id, window.number, ordinal)?
                .filter(|step| step.phase == state::IntegrationStepPhase::Applied);
            let last_result = last.as_ref().and_then(|step| step.result_oid);
            let released = match &last {
                Some(step) => state::integration_resolution_released(tx, &record, step)?,
                None => false,
            };
            let local = settlement.local_oid;
            let action = match settlement.intent {
                PublicationIntent::None => return Err(state::recovery_required()),
                PublicationIntent::Open => {
                    let attempt = state::latest_publication_attempt(tx, &record)?
                        .ok_or_else(state::recovery_required)?;
                    if settlement.relation.is_some()
                        || !(last_result == Some(local)
                            || (attempt.local_oid == local && !settlement.continuation)
                            || (settlement.continuation && released))
                    {
                        return Err(state::recovery_required());
                    }
                    Action::Resume
                }
                PublicationIntent::Verified(candidate) => {
                    if local != candidate
                        || settlement.relation != Some(PushIntentRelation::Equal)
                        || settlement.advertised_oid != Some(candidate)
                    {
                        return Err(state::recovery_required());
                    }
                    Action::Resume
                }
                PublicationIntent::Legacy(intent) | PublicationIntent::Attempt(intent) => {
                    let legacy = matches!(settlement.intent, PublicationIntent::Legacy(_));
                    let current = local == intent;
                    let relation = settlement.relation.ok_or_else(state::recovery_required)?;
                    if (current && settlement.continuation)
                        || (!current
                            && !(settlement.continuation
                                && released
                                && last_result == Some(intent)))
                        || (relation == PushIntentRelation::Absent)
                            != settlement.advertised_oid.is_none()
                        || (relation == PushIntentRelation::Equal)
                            != (settlement.advertised_oid == Some(intent))
                        || (legacy && checkpoint == C::PushVerified)
                    {
                        return Err(state::recovery_required());
                    }
                    match relation {
                        // An equal, behind or never-advertised intent that is
                        // still HEAD stays on the unchanged legacy route.
                        PushIntentRelation::Equal if current && legacy => {
                            return Err(state::recovery_required());
                        }
                        PushIntentRelation::Equal if current => Action::Verify,
                        PushIntentRelation::Equal => Action::Open(intent, D::Accepted),
                        PushIntentRelation::Behind if current && legacy => {
                            return Err(state::recovery_required());
                        }
                        PushIntentRelation::Behind => Action::Open(intent, D::NotAccepted),
                        PushIntentRelation::Ahead => Action::Open(intent, D::Contained),
                        PushIntentRelation::Diverged => Action::Open(intent, D::Displaced),
                        PushIntentRelation::Absent => {
                            // Absence after any recorded advertisement is
                            // remote deletion and is never recreated here.
                            if (current && legacy)
                                || state::publication_advertisement_recorded(tx, &record)?
                            {
                                return Err(state::recovery_required());
                            }
                            Action::Open(intent, D::NotAccepted)
                        }
                    }
                }
            };
            let decision = acknowledge(tx, &record, RemoteOperationSafePoint::AfterFetch)?;
            if decision != RemoteSafePointOutcome::Continue {
                return Ok(decision);
            }
            match action {
                Action::Resume => {}
                Action::Verify => {
                    state::advance_publication_attempt(
                        tx,
                        &record,
                        state::PublicationPhase::Verified,
                        settlement.advertised_oid,
                    )?;
                }
                Action::Open(previous, disposition) => {
                    state::open_publication_attempt(
                        tx,
                        &record,
                        previous,
                        settlement.advertised_oid,
                        disposition,
                        local,
                    )?;
                }
            }
            // Leave reconciliation without moving the frozen legacy checkpoint,
            // push intent or local OID.
            tx.execute(
                "UPDATE remote_operation_records SET phase=?2,reconciliation_required=0,updated_at=max(updated_at,?3) WHERE id=?1",
                params![record.id, checkpoint.name(), now()],
            )
            .map_err(|_| state::recovery_required())?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    /// Durable push intent of the open attempt, written BEFORE any push effect.
    /// With `verified`, the fresh Push advertisement already equals the
    /// candidate and no push will follow.
    pub(crate) fn prepare_synchronization_publication(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        candidate: git2::Oid,
        advertised_oid: Option<git2::Oid>,
        verified: bool,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            // An endpoint that advertised this target before and now advertises
            // nothing was deleted remotely; publication never recreates it.
            if !is_sync(&record.target)
                || record.reconciliation_required
                || (advertised_oid.is_none()
                    && state::publication_advertisement_recorded(tx, &record)?)
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
            state::bind_publication_candidate(tx, &record, candidate, advertised_oid, verified)?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    /// Published or AlreadyCurrent for the verified newest attempt, derived
    /// only from stored evidence so it is identical before and after a restart.
    pub(crate) fn synchronization_publication_authority(
        &self,
        root: &Path,
        owner: &RemoteReservation,
    ) -> Result<state::SynchronizationAuthority, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target) {
                return Err(state::recovery_required());
            }
            state::publication_authority(tx, &record)
        })
    }

    /// Forward-only observation of the newest attempt's own push.
    pub(crate) fn advance_synchronization_publication(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        phase: state::PublicationPhase,
        advertised_oid: Option<git2::Oid>,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if !is_sync(&record.target)
                || record.reconciliation_required
                || (phase == state::PublicationPhase::Returned
                    && record.completed_step != Some(RemoteOperationSafePoint::BeforePush))
            {
                return Err(state::recovery_required());
            }
            if let Some(point) = record.completed_step {
                let decision = acknowledge(tx, &record, point)?;
                if decision != RemoteSafePointOutcome::Continue {
                    return Ok(decision);
                }
            }
            state::advance_publication_attempt(tx, &record, phase, advertised_oid)?;
            Ok(RemoteSafePointOutcome::Continue)
        })
    }

    /// Record this operation's own merge metadata digests in the same fenced
    /// step that just produced them. Later retirement accepts nothing else.
    pub(super) fn record_synchronization_merge_metadata(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        window_number: u32,
        ordinal: u8,
        digests: [Option<[u8; 32]>; 3],
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::record_integration_merge_metadata(tx, &record, window_number, ordinal, digests)
        })
    }

    pub(super) fn synchronization_merge_metadata(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        step: &state::IntegrationStepEvidence,
    ) -> Result<Option<state::MergeMetadataEvidence>, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            state::integration_merge_metadata(tx, &record, step.window_number, step.intent.ordinal)
        })
    }

    pub(super) fn advance_synchronization_merge_metadata(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        step: &state::IntegrationStepEvidence,
        phase: &str,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            let record = owned(self, tx, id, owner)?;
            if record.cancel_requested || record.yield_requested || !record.reconciliation_required
            {
                return Err(state::recovery_required());
            }
            state::advance_integration_merge_metadata(
                tx,
                &record,
                step.window_number,
                step.intent.ordinal,
                phase,
            )
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
            if let Some(attempt) = state::latest_publication_attempt(tx, &record)? {
                // Authority comes from the newest verified attempt alone. The
                // legacy push columns keep describing the superseded intent.
                if !is_sync(&record.target)
                    || record.reconciliation_required
                    || attempt.phase != state::PublicationPhase::Verified
                    || attempt.candidate_oid != Some(authority.oid())
                    || attempt.advertised_oid != Some(authority.oid())
                {
                    return Err(state::recovery_required());
                }
            } else if !is_sync(&record.target)
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
            // Verification already consumed R1 in its own transaction. Merely
            // classifying that older proof is not a new publication observation:
            // a concurrent confirmed cleanup may have earned NEW retirement
            // since verification, and that later proof must be preserved.
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
