//! Private no-child-gap observation. A current attempt references ONE immutable
//! original freeze/cursor, never another selection or an aggregate local owner.
//! No resumed child, cleanup, index, OriginalCapture or branch permit is minted.
use super::*;
use crate::repository::{
    OperationFamily, RemoteOperationPhase, RemoteOperationPriority, RemoteOperationSafePoint,
    RemoteOperationTarget, RemoteReservation, RemoteSafePointOutcome, RepositoryOperation,
    mutation::Accepted, recovery, remote::reservation, repository_lease,
};
use std::path::PathBuf;

pub(in super::super) const MARKER: &str = "context_retirement_gap_v1";
pub(in super::super) const TABLE: &str = "retirement_batch_gap_attempts";

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Fault {
    Fence,
    Epoch,
    Observer,
    Bind,
    Release,
    Provenance,
    Receipt,
    Pin,
    Capture,
    End,
}
#[cfg(test)]
thread_local! {static FAILURE:std::cell::Cell<Option<Fault>>=const {std::cell::Cell::new(None)};}
#[cfg(test)]
pub(in super::super) fn set_failure(point: Option<Fault>) -> Option<Fault> {
    FAILURE.with(|slot| slot.replace(point))
}
#[cfg(test)]
fn checkpoint(point: Fault) -> Result<(), RepositoryError> {
    if FAILURE.with(|slot| {
        if slot.get() == Some(point) {
            slot.set(None);
            true
        } else {
            false
        }
    }) {
        return Err(recovery_required());
    }
    Ok(())
}

pub(in super::super) fn object(name: &str) -> bool {
    name == TABLE || name.starts_with("retirement_gap_")
}

pub(in super::super) fn objects() -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("table", TABLE, "CREATE TABLE retirement_batch_gap_attempts (
 operation_ulid TEXT NOT NULL,
 owner_epoch INTEGER NOT NULL CHECK(typeof(owner_epoch)='integer' AND owner_epoch>0),
 previous_epoch INTEGER NOT NULL CHECK(typeof(previous_epoch)='integer' AND previous_epoch>0 AND owner_epoch=previous_epoch+1),
 original_epoch INTEGER NOT NULL CHECK(typeof(original_epoch)='integer' AND original_epoch>0 AND previous_epoch>=original_epoch),
 repository_id INTEGER NOT NULL,
 root_path TEXT NOT NULL,
 request_ulid TEXT NOT NULL REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 request_attempt INTEGER NOT NULL CHECK(typeof(request_attempt)='integer' AND request_attempt>0),
 next_ordinal INTEGER NOT NULL CHECK(typeof(next_ordinal)='integer' AND next_ordinal>=0),
 frozen_worklist_digest BLOB NOT NULL CHECK(typeof(frozen_worklist_digest)='blob' AND length(frozen_worklist_digest)=32),
 observer_operation_ulid TEXT NOT NULL UNIQUE,
 observer_owner_epoch INTEGER NOT NULL CHECK(typeof(observer_owner_epoch)='integer' AND observer_owner_epoch>=0),
 provenance_id INTEGER UNIQUE REFERENCES retirement_advertisement_provenance(id) ON DELETE NO ACTION,
 phase TEXT NOT NULL CHECK(phase IN ('active','ended','fenced')),
 ended_kind TEXT CHECK(ended_kind IN ('completed','stopped')),
 PRIMARY KEY(operation_ulid,owner_epoch),
 UNIQUE(operation_ulid,request_attempt),
 FOREIGN KEY(repository_id,operation_ulid) REFERENCES remote_operation_records(repository_id,operation_ulid) ON DELETE NO ACTION,
 FOREIGN KEY(repository_id,observer_operation_ulid) REFERENCES remote_operation_records(repository_id,operation_ulid) ON DELETE NO ACTION,
 CHECK(operation_ulid!=observer_operation_ulid),
 CHECK(phase!='active' OR ended_kind IS NULL),
 CHECK(phase!='ended' OR ended_kind IS NOT NULL)
)".into()),
        ("trigger", "retirement_gap_identity", "CREATE TRIGGER retirement_gap_identity BEFORE UPDATE OF operation_ulid,owner_epoch,previous_epoch,original_epoch,repository_id,root_path,request_ulid,request_attempt,next_ordinal,frozen_worklist_digest,observer_operation_ulid,observer_owner_epoch ON retirement_batch_gap_attempts BEGIN SELECT RAISE(ABORT,'immutable gap scope'); END".into()),
        ("trigger", "retirement_gap_capture", "CREATE TRIGGER retirement_gap_capture BEFORE UPDATE OF provenance_id ON retirement_batch_gap_attempts WHEN OLD.provenance_id IS NOT NULL OR OLD.phase!='active' OR NEW.provenance_id IS NULL BEGIN SELECT RAISE(ABORT,'single gap capture'); END".into()),
        ("trigger", "retirement_gap_phase", "CREATE TRIGGER retirement_gap_phase BEFORE UPDATE OF phase ON retirement_batch_gap_attempts WHEN NOT((OLD.phase='active' AND NEW.phase IN ('ended','fenced')) OR (OLD.phase='ended' AND NEW.phase='fenced')) BEGIN SELECT RAISE(ABORT,'ended gap executor'); END".into()),
        ("trigger", "retirement_gap_end", "CREATE TRIGGER retirement_gap_end BEFORE UPDATE OF ended_kind ON retirement_batch_gap_attempts WHEN OLD.ended_kind IS NOT NULL OR OLD.phase!='active' OR NEW.ended_kind IS NULL BEGIN SELECT RAISE(ABORT,'immutable gap ending'); END".into()),
        ("trigger", "retirement_gap_delete", "CREATE TRIGGER retirement_gap_delete BEFORE DELETE ON retirement_batch_gap_attempts BEGIN SELECT RAISE(ABORT,'retained gap spend'); END".into()),
    ]
}

#[derive(Clone, PartialEq, Eq)]
struct Attempt {
    operation: String,
    epoch: i64,
    previous: i64,
    original: i64,
    repository: i64,
    root: String,
    request: String,
    request_attempt: i64,
    cursor: i64,
    digest: Vec<u8>,
    observer: String,
    observer_epoch: i64,
    provenance: Option<i64>,
    phase: String,
    ended: Option<String>,
}
fn read_attempt(db: &Connection, operation: &str, epoch: i64) -> Result<Attempt, RepositoryError> {
    db.query_row("SELECT operation_ulid,owner_epoch,previous_epoch,original_epoch,repository_id,root_path,request_ulid,request_attempt,next_ordinal,frozen_worklist_digest,observer_operation_ulid,observer_owner_epoch,provenance_id,phase,ended_kind FROM retirement_batch_gap_attempts WHERE operation_ulid=?1 AND owner_epoch=?2",params![operation,epoch],|r|Ok(Attempt{operation:r.get(0)?,epoch:r.get(1)?,previous:r.get(2)?,original:r.get(3)?,repository:r.get(4)?,root:r.get(5)?,request:r.get(6)?,request_attempt:r.get(7)?,cursor:r.get(8)?,digest:r.get(9)?,observer:r.get(10)?,observer_epoch:r.get(11)?,provenance:r.get(12)?,phase:r.get(13)?,ended:r.get(14)?})).map_err(|_|recovery_required())
}
fn original(db: &Connection, operation: &str) -> Result<(Projection, Frozen), RepositoryError> {
    let epoch: i64 = db
        .query_row(
            "SELECT owner_epoch FROM poll_retirement_freezes WHERE operation_ulid=?1",
            [operation],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    let p = source::projection(db, operation, epoch)?;
    let f = read(db, &p)?;
    if p.stage != "frozen" || f.eligibility != "eligible" || f.code.is_some() || f.count == 0 {
        return Err(recovery_required());
    }
    Ok((p, f))
}
fn current_epoch(db: &Connection, p: &Projection) -> Result<i64, RepositoryError> {
    db.query_row("SELECT owner_epoch FROM poll_retirement_batches WHERE operation_ulid=?1 AND repository_id=?2 AND request_ulid=?3 AND mode='retire_completed' AND stage='frozen'",params![p.operation,p.repository,p.request],|r|r.get(0)).map_err(|_|recovery_required())
}
fn request(db: &Connection, p: &Projection, attempt: i64) -> Result<bool, RepositoryError> {
    // projection() already proves the complete historical digest/base/remote-P
    // mapping. Current accepted state and F2 are an additional execution fence.
    db.query_row("SELECT cancel_requested FROM request_records WHERE request_ulid=?1 AND attempt=?2 AND state='accepted'",params![p.request,attempt],|r|r.get(0)).optional().map_err(|_|recovery_required())?.ok_or_else(recovery_required)
}
fn accepted_scope(accepted: &Accepted, p: &Projection) -> Result<i64, RepositoryError> {
    let attempt = i64::try_from(accepted.attempt).map_err(|_| recovery_required())?;
    if accepted.request_id.to_string() != p.request
        || accepted.operation_id.to_string() != p.operation
        || accepted.family != Some(OperationFamily::Remote)
        || accepted.scope.repository() != Some(p.root.as_str())
        || accepted.expected.is_some()
        || accepted.position.base_ref.as_deref()
            != Some(format!("refs/heads/{}", p.primary).as_str())
        || accepted
            .position
            .base_oid
            .map(|oid| oid.to_string())
            .as_deref()
            != Some(p.base.as_str())
        || attempt <= 0
    {
        return Err(recovery_required());
    }
    Ok(attempt)
}
fn exclusion(
    db: &Connection,
    p: &Projection,
    observer: Option<&str>,
) -> Result<(), RepositoryError> {
    recovery::require_no_pending_local(db, p.repository)?;
    let local:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE (repository_id=?1 OR root_path=?2) AND (state!='completed' OR completed_step='comment_registration_pending'))",params![p.repository,p.root],|r|r.get(0)).map_err(|_|recovery_required())?;
    if local {
        return Err(recovery_required());
    }
    crate::repository::local_sync::record::require_no_pending(db, p.repository)?;
    batch::require_no_retained_integration(db, p.repository)?;
    let other: bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND ((operation_ulid IS NOT ?2 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')) OR index_pending!=0 OR reconciliation_required!=0))",params![p.repository,observer],|r|r.get(0)).map_err(|_|recovery_required())?;
    if other {
        return Err(recovery_required());
    }
    Ok(())
}
fn readonly(
    db: &Connection,
    p: &Projection,
    a: &Attempt,
    fenced: bool,
) -> Result<state::StoredRemoteOperation, RepositoryError> {
    let id = OperationId::parse(&a.observer).map_err(|_| recovery_required())?;
    let remote = state::read_operation(db, p.repository, id)?.ok_or_else(recovery_required)?;
    let plan = crate::repository::RemoteRefPlan::from_configuration(&p.remote, &p.primary)
        .map_err(|_| recovery_required())?;
    let extra:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_integration_windows WHERE operation_record_id=?1) OR EXISTS(SELECT 1 FROM remote_publication_attempts WHERE operation_record_id=?1) OR EXISTS(SELECT 1 FROM context_retirement_attempts WHERE observer_operation_ulid=?2) OR EXISTS(SELECT 1 FROM request_operations WHERE operation_ulid=?2)",params![remote.id,a.observer],|r|r.get(0)).map_err(|_|recovery_required())?;
    if remote.owner_epoch
        != a.observer_epoch
            .checked_add(i64::from(fenced))
            .ok_or_else(recovery_required)?
        || remote.generation != p.generation
        || remote.target != RemoteOperationTarget::for_poll(&plan)
        || remote.poll_kind != Some(state::PollKind::Observe)
        || remote.priority != RemoteOperationPriority::Manual
        || remote.index_pending
        || remote.reconciliation_required
        || remote.authority.is_some()
        || remote.sync_checkpoint.is_some()
        || remote.local_oid.is_some()
        || remote.tracking_oid.is_some()
        || remote.advertised_oid.is_some()
        || !matches!(
            remote.phase,
            RemoteOperationPhase::Reserved
                | RemoteOperationPhase::Advertising
                | RemoteOperationPhase::Persisting
                | RemoteOperationPhase::Completed
                | RemoteOperationPhase::Interrupted
                | RemoteOperationPhase::Cancelled
                | RemoteOperationPhase::Failed
        )
        || !matches!(
            remote.completed_step,
            None | Some(
                RemoteOperationSafePoint::BeforeTransport
                    | RemoteOperationSafePoint::AfterAdvertisement
                    | RemoteOperationSafePoint::BetweenObservations
                    | RemoteOperationSafePoint::BeforeBatchCommit
                    | RemoteOperationSafePoint::AfterBatchCommit
            )
        )
        || extra
    {
        return Err(recovery_required());
    }
    for (table, column) in [
        ("operation_records", "operation_ulid"),
        ("local_context_sync_records", "operation_ulid"),
        ("key_material_operations", "operation_id"),
        ("key_registry_changes", "operation_id"),
    ] {
        let collision: bool = db
            .query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column}=?1)"),
                [&a.observer],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        if collision {
            return Err(recovery_required());
        }
    }
    if reserved_child(db, &a.observer)? || a.observer == p.operation {
        return Err(recovery_required());
    }
    Ok(remote)
}

/// Non-Clone, execution-only. The new explicit attempt captures fresh key/source
/// equality evidence, as standalone reentry does; it cannot restore original Live.
pub(in super::super::super) struct Execution {
    scope: Arc<()>,
    registry: PathBuf,
    attempt: Attempt,
    p: Projection,
    plan: crate::repository::RemoteRefPlan,
    configuration: ObservationConfiguration,
}
impl std::fmt::Debug for Execution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchGapExecution").finish_non_exhaustive()
    }
}

#[allow(dead_code)] // Private Task2 seam; no Task3/4 caller/binding is installed.
pub(in super::super::super) fn claim(
    service: &RepositoryService,
    root: &Path,
    accepted: &Accepted,
    expected_epoch: i64,
) -> Result<RemoteReservation, RepositoryError> {
    let (git, canonical) =
        crate::repository::canonical_repository_root(root, RepositoryOperation::RetireContext)?;
    if canonical != root || git.is_worktree() {
        return Err(recovery_required());
    }
    let _lease = repository_lease(&git, root, RepositoryOperation::RetireContext)?;
    state::with_transaction(service, root, |tx, id| {
        record::audit_connection(tx)?;
        let (mut p, f) = original(tx, &accepted.operation_id.to_string())?;
        let attempt = accepted_scope(accepted, &p)?;
        if id != p.repository
            || root.to_str() != Some(p.root.as_str())
            || current_epoch(tx, &p)? != expected_epoch
            || request(tx, &p, attempt)?
        {
            return Err(recovery_required());
        }
        let (_, cursor, stage) = batch::projected(tx, &p, &f)?;
        if stage != "active" {
            return Err(recovery_required());
        }
        let old = if expected_epoch == p.epoch {
            None
        } else {
            Some(read_attempt(tx, &p.operation, expected_epoch)?)
        };
        if attempt
            <= old
                .as_ref()
                .map_or(p.request_attempt, |a| a.request_attempt)
        {
            return Err(recovery_required());
        }
        exclusion(tx, &p, old.as_ref().map(|a| a.observer.as_str()))?;
        let old_remote = old
            .as_ref()
            .map(|a| readonly(tx, &p, a, false))
            .transpose()?;
        let plan = crate::repository::remote::observation::observation_plan(root)
            .map_err(|_| recovery_required())?;
        if state::generation(tx, id)? != p.generation
            || plan.remote_name() != p.remote
            || plan.primary_branch() != p.primary
        {
            return Err(recovery_required());
        }
        let configuration = ObservationConfiguration::capture_owned(service, tx, root, &plan)?;
        if configuration.endpoint_digest().as_slice() != p.endpoint {
            return Err(recovery_required());
        }
        let epoch = expected_epoch
            .checked_add(1)
            .ok_or_else(recovery_required)?;
        if let Some(remote) = old_remote {
            if tx.execute("UPDATE remote_operation_records SET owner_epoch=owner_epoch+1,phase=CASE WHEN phase IN ('reserved','advertising','persisting') THEN 'interrupted' ELSE phase END,outcome=CASE WHEN phase IN ('reserved','advertising','persisting') THEN 'repository_unavailable' ELSE outcome END WHERE id=?1 AND owner_epoch=?2",params![remote.id,remote.owner_epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            #[cfg(test)]
            checkpoint(Fault::Fence)?;
            if tx.execute("UPDATE retirement_batch_gap_attempts SET phase='fenced' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase IN ('active','ended')",params![p.operation,expected_epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
        } else {
            if tx.execute("UPDATE poll_retirement_attempts SET phase='fenced' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            p.phase = "fenced".into();
        }
        if tx.execute("UPDATE poll_retirement_batches SET owner_epoch=?3 WHERE operation_ulid=?1 AND owner_epoch=?2",params![p.operation,expected_epoch,epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
        #[cfg(test)]
        checkpoint(Fault::Epoch)?;
        let observer = OperationId::new();
        no_child_collision(tx, &observer.to_string())?;
        if record::owned_identity(tx, &observer.to_string())? {
            return Err(recovery_required());
        }
        state::insert_operation(
            tx,
            id,
            observer,
            &RemoteOperationTarget::for_poll(&plan),
            RemoteOperationPriority::Manual,
            service.clock.now().unix_timestamp(),
        )?;
        #[cfg(test)]
        checkpoint(Fault::Observer)?;
        let remote = state::read_operation(tx, id, observer)?.ok_or_else(recovery_required)?;
        tx.execute("INSERT INTO retirement_batch_gap_attempts(operation_ulid,owner_epoch,previous_epoch,original_epoch,repository_id,root_path,request_ulid,request_attempt,next_ordinal,frozen_worklist_digest,observer_operation_ulid,observer_owner_epoch,phase) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'active')",params![p.operation,epoch,expected_epoch,p.epoch,id,p.root,p.request,attempt,cursor as i64,f.digest,observer.to_string(),remote.owner_epoch]).map_err(|_|recovery_required())?;
        #[cfg(test)]
        checkpoint(Fault::Bind)?;
        let binding = if let Some(old) = old {
            // A stopped observer already released this exact consumer. Do not
            // resolve its scalar batch ID again, or rewrite its terminal Z.
            if old.ended.as_deref() == Some("stopped") {
                None
            } else {
                old.provenance
                    .map(|provenance| {
                        tx.query_row(
                            "SELECT id FROM retirement_observation_receipts WHERE provenance_id=?1",
                            [provenance],
                            |r| r.get(0),
                        )
                        .map_err(|_| recovery_required())
                    })
                    .transpose()?
            }
        } else {
            Some(f.binding)
        };
        if let Some(binding) = binding {
            record::release_gap_consumer(tx, binding, "consumer_fenced")?;
        }
        #[cfg(test)]
        checkpoint(Fault::Release)?;
        let execution = Execution {
            scope: Arc::clone(&service.remote_reservation_scope),
            registry: service.registry_path.clone(),
            attempt: read_attempt(tx, &p.operation, epoch)?,
            p,
            plan,
            configuration,
        };
        execution.validate(service, tx, id, &remote)?;
        Ok(reservation::gap_token(service, id, &remote, execution))
    })
}

/// Ownership is enough only for STOP, never for a configuration write/rebind,
/// advertisement or result claiming an observed removal. Unknown reads refuse.
#[allow(dead_code)] // Used by the private observation runner, not a public binding.
pub(in super::super::super) fn stop_configuration(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
) -> Result<(), RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        let remote = reservation::gap_record_for_stop(service, tx, id, owner)?;
        let execution = owner.gap_execution()?;
        let cancelled =
            execution.validate_ownership(service, tx, id, &remote)? || remote.cancel_requested;
        let changed = state::generation(tx, id)? != execution.p.generation
            || !execution
                .configuration
                .matches_owned(service, tx, root, &execution.plan)?;
        if !changed && !cancelled {
            return Err(recovery_required());
        }
        tx.execute(
            "UPDATE remote_operation_records SET phase=?2,outcome=?3 WHERE id=?1",
            params![
                remote.id,
                if cancelled {
                    "cancelled"
                } else {
                    "interrupted"
                },
                if cancelled {
                    "cancelled"
                } else {
                    "configuration_required"
                }
            ],
        )
        .map_err(|_| recovery_required())?;
        execution.end(tx, false)
    })
}

/// Private composition only. Task4 does not yet expose/schedule this operation.
#[allow(dead_code, clippy::result_large_err)] // Existing observation/SSH error contract.
pub(in super::super::super) fn observe_for_request<
    P: crate::repository::keys::SessionCredentialProvider,
>(
    service: &RepositoryService,
    root: &Path,
    accepted: &Accepted,
    expected_epoch: i64,
    approval: Option<crate::repository::transport::HostApproval>,
    session: &mut crate::repository::keys::SessionCredentials<P>,
) -> Result<RemoteSafePointOutcome, crate::repository::remote::RemoteObservationError> {
    let owner = claim(service, root, accepted, expected_epoch)?;
    observe_with(service, root, &owner, |expected| {
        service.with_authenticated_remote_expected(
            crate::repository::transport::VerifySshTransportRequest {
                root: root.into(),
                direction: crate::repository::transport::SshDirection::Fetch,
                approval,
            },
            session,
            expected,
            |remote| remote.advertisement(),
        )
    })
}

#[allow(dead_code, clippy::result_large_err)] // Same typed runner as normal observation.
pub(in super::super::super) fn observe_with(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    advertise: impl FnOnce(
        &crate::repository::transport::SshScopeExpectation,
    ) -> Result<
        Vec<(String, git2::Oid)>,
        crate::repository::transport::SshTransportError,
    >,
) -> Result<RemoteSafePointOutcome, crate::repository::remote::RemoteObservationError> {
    let execution = owner.gap_execution()?;
    let point = |point| {
        service
            .remote_safe_point(root, owner, point)
            .inspect_err(|_| {
                // Positive current config differences can acknowledge only STOP.
                // Stale/unknown ownership or inspection cannot authorize even that.
                let _ = stop_configuration(service, root, owner);
            })
    };
    let decision = point(RemoteOperationSafePoint::BeforeTransport)?;
    if decision != RemoteSafePointOutcome::Continue {
        return Ok(decision);
    }
    let expected = execution
        .configuration
        .transport_expectation(crate::repository::transport::SshDirection::Fetch)
        .ok_or_else(recovery_required)?;
    // claim's common-Git lease and immediate registry transaction have dropped.
    // Only a complete read-only Fetch advertisement is requested from transport.
    let advertised = advertise(&expected);
    let decision = point(RemoteOperationSafePoint::AfterAdvertisement)?;
    if decision != RemoteSafePointOutcome::Continue {
        return Ok(decision);
    }
    let advertised = match advertised {
        Ok(advertised) => advertised,
        Err(error) => {
            let decision = service.finish_remote_operation(
                root,
                owner,
                crate::repository::remote::observation::transport_category(&error.kind),
            )?;
            if decision != RemoteSafePointOutcome::Continue {
                return Ok(decision);
            }
            return Err(crate::repository::remote::RemoteObservationError::Transport(error));
        }
    };
    let git = git2::Repository::open(root).map_err(|_| recovery_required())?;
    let mut observations = Vec::new();
    for (name, oid) in advertised {
        let decision = point(RemoteOperationSafePoint::BetweenObservations)?;
        if decision != RemoteSafePointOutcome::Continue {
            return Ok(decision);
        }
        let tracking = match execution.plan.tracking_ref_for(&name) {
            None => None,
            Some(name) => match git.refname_to_id(&name) {
                Ok(oid) => Some(oid),
                Err(e) if e.code() == git2::ErrorCode::NotFound => None,
                Err(_) => return Err(recovery_required().into()),
            },
        };
        if let Some(observation) = crate::repository::RemoteRefObservation::from_advertisement(
            &execution.plan,
            &name,
            oid,
            tracking,
        ) {
            observations.push(observation);
        }
    }
    drop(git);
    let decision = point(RemoteOperationSafePoint::BeforeBatchCommit)?;
    if decision != RemoteSafePointOutcome::Continue {
        return Ok(decision);
    }
    let decision = reservation::commit_observation_batch(
        service,
        root,
        owner,
        &execution.plan,
        &observations,
        service.clock.now().unix_timestamp(),
    )?;
    if decision != RemoteSafePointOutcome::Continue {
        return Ok(decision);
    }
    service
        .finish_remote_operation(
            root,
            owner,
            crate::repository::RemoteOutcomeCategory::Completed,
        )
        .map_err(Into::into)
}

impl Execution {
    pub(in super::super::super) fn validate_ownership(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        remote: &state::StoredRemoteOperation,
    ) -> Result<bool, RepositoryError> {
        if !Arc::ptr_eq(&self.scope, &service.remote_reservation_scope)
            || self.registry != service.registry_path
            || tx.path() != self.registry.to_str()
            || id != self.p.repository
        {
            return Err(recovery_required());
        }
        record::audit_connection(tx)?;
        let a = read_attempt(tx, &self.attempt.operation, self.attempt.epoch)?;
        let mut expected = self.attempt.clone();
        expected.provenance = a.provenance; // The one owned capture may have committed.
        if a != expected
            || a.phase != "active"
            || current_epoch(tx, &self.p)? != a.epoch
            || source::projection(tx, &self.p.operation, self.p.epoch)? != self.p
        {
            return Err(recovery_required());
        }
        let f = read(tx, &self.p)?;
        let (_, cursor, stage) = batch::projected(tx, &self.p, &f)?;
        if f.digest != a.digest
            || f.eligibility != "eligible"
            || f.code.is_some()
            || cursor as i64 != a.cursor
            || stage != "active"
        {
            return Err(recovery_required());
        }
        let bound = readonly(tx, &self.p, &a, false)?;
        if remote.operation_id != bound.operation_id
            || remote.owner_epoch != bound.owner_epoch
            || remote.generation != bound.generation
            || remote.target != bound.target
        {
            return Err(recovery_required());
        }
        exclusion(tx, &self.p, Some(&a.observer))?;
        request(tx, &self.p, a.request_attempt)
    }
    pub(in super::super::super) fn validate(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        remote: &state::StoredRemoteOperation,
    ) -> Result<(), RepositoryError> {
        if self.validate_ownership(service, tx, id, remote)?
            || remote.cancel_requested
            || state::generation(tx, id)? != self.p.generation
            || !self.configuration.matches_owned(
                service,
                tx,
                Path::new(&self.p.root),
                &self.plan,
            )?
        {
            return Err(recovery_required());
        }
        let current:bool=tx.query_row("SELECT NOT EXISTS(SELECT 1 FROM retirement_batch_gap_attempts g WHERE g.operation_ulid=?1 AND g.owner_epoch=?2 AND g.provenance_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM retirement_advertisement_provenance p JOIN retirement_observation_pins pin ON pin.provenance_id=p.id JOIN remote_observation_batches b ON b.id=pin.observation_batch_id WHERE p.id=g.provenance_id AND b.is_current=1 AND b.configuration_generation=?3 AND pin.pin_kind='consumer_attempt'))",params![self.attempt.operation,self.attempt.epoch,self.p.generation],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !current {
            return Err(recovery_required());
        }
        Ok(())
    }
    pub(in super::super::super) fn capture(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        remote: &state::StoredRemoteOperation,
        batch: i64,
    ) -> Result<(), RepositoryError> {
        self.validate(service, tx, id, remote)?;
        let a = &self.attempt;
        let p = &self.p;
        let observed:i64=tx.query_row("SELECT observed_at FROM remote_observation_batches WHERE id=?1 AND repository_id=?2 AND is_current=1 AND configuration_generation=?3 AND remote_name=?4 AND primary_branch=?5",params![batch,id,p.generation,p.remote,p.primary],|r|r.get(0)).map_err(|_|recovery_required())?;
        tx.execute("INSERT INTO retirement_advertisement_provenance(repository_id,source_operation_ulid,source_owner_epoch,observation_batch_id,observed_at,configuration_generation,remote_name,primary_branch,endpoint_digest,execution_kind,execution_request_ulid,execution_request_attempt,execution_batch_operation_ulid,execution_batch_attempt_epoch) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'batch_gap',?10,?11,?12,?13)",params![id,a.observer,a.observer_epoch,batch,observed,p.generation,p.remote,p.primary,p.endpoint,p.request,a.request_attempt,p.operation,a.epoch]).map_err(|_|recovery_required())?;
        let provenance = tx.last_insert_rowid();
        #[cfg(test)]
        checkpoint(Fault::Provenance)?;
        tx.execute("INSERT INTO retirement_observation_receipts(provenance_id,repository_id,consumer_kind,consumer_operation_ulid,consumer_request_ulid,consumer_request_attempt,consumer_attempt_epoch,configuration_generation,frozen_worklist_digest) VALUES(?1,?2,'batch',?3,?4,?5,?6,?7,?8)",params![provenance,id,p.operation,p.request,a.request_attempt,a.epoch,p.generation,a.digest]).map_err(|_|recovery_required())?;
        let binding = tx.last_insert_rowid();
        #[cfg(test)]
        checkpoint(Fault::Receipt)?;
        tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id) VALUES(?1,'consumer_attempt',?2,?3,?4,?5,?6)",params![provenance,p.operation,a.epoch,id,batch,binding]).map_err(|_|recovery_required())?;
        #[cfg(test)]
        checkpoint(Fault::Pin)?;
        if tx.execute("UPDATE retirement_batch_gap_attempts SET provenance_id=?3 WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active' AND provenance_id IS NULL",params![p.operation,a.epoch,provenance]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
        #[cfg(test)]
        checkpoint(Fault::Capture)?;
        Ok(())
    }
    pub(in super::super::super) fn end(
        &self,
        tx: &Transaction<'_>,
        success: bool,
    ) -> Result<(), RepositoryError> {
        let a = &self.attempt;
        if tx.execute("UPDATE retirement_batch_gap_attempts SET phase='ended',ended_kind=?3 WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![a.operation,a.epoch,if success {"completed"}else{"stopped"}]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
        let provenance = read_attempt(tx, &a.operation, a.epoch)?.provenance;
        if success && provenance.is_none() {
            return Err(recovery_required());
        }
        if !success && let Some(provenance) = provenance {
            let binding = tx
                .query_row(
                    "SELECT id FROM retirement_observation_receipts WHERE provenance_id=?1",
                    [provenance],
                    |r| r.get(0),
                )
                .map_err(|_| recovery_required())?;
            record::release_gap_consumer(tx, binding, "consumer_terminal")?;
        }
        if tx.execute("UPDATE request_records SET cancel_requested=0 WHERE request_ulid=?1 AND attempt=?2 AND state='accepted' AND cancel_requested=1",params![a.request,a.request_attempt]).map_err(|_|recovery_required())?>1 {return Err(recovery_required());}
        #[cfg(test)]
        checkpoint(Fault::End)?;
        record::audit_connection(tx)
    }
    pub(in super::super::super) fn transition(
        &self,
        tx: &Transaction<'_>,
        decision: RemoteSafePointOutcome,
    ) -> Result<(), RepositoryError> {
        if decision != RemoteSafePointOutcome::Continue {
            self.end(tx, false)?;
        }
        Ok(())
    }
}

/// Original A3 eligibility is immutable; only this exact adapter lineage admits
/// its fenced consumer history. Original P/source identity never changes.
pub(super) fn original_fenced(
    db: &Connection,
    p: &Projection,
    f: &Frozen,
) -> Result<bool, RepositoryError> {
    if !recovery::has_table(db, TABLE).map_err(|_| recovery_required())? {
        return Ok(false);
    }
    db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_batch_gap_attempts WHERE operation_ulid=?1 AND original_epoch=?2 AND previous_epoch=?2 AND repository_id=?3 AND request_ulid=?4 AND request_attempt>?5 AND frozen_worklist_digest=?6)",params![p.operation,p.epoch,p.repository,p.request,p.request_attempt,f.digest],|r|r.get(0)).map_err(|_|recovery_required())
}
pub(in super::super) fn require_release(
    db: &Connection,
    binding: i64,
    reason: &str,
) -> Result<(), RepositoryError> {
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_receipts b JOIN poll_retirement_freezes f ON f.binding_id=b.id JOIN poll_retirement_attempts a ON a.operation_ulid=f.operation_ulid AND a.owner_epoch=f.owner_epoch JOIN retirement_batch_gap_attempts g ON g.operation_ulid=f.operation_ulid AND g.previous_epoch=f.owner_epoch AND g.original_epoch=f.owner_epoch WHERE b.id=?1 AND ?2='consumer_fenced' AND a.phase='fenced' AND f.eligibility='eligible' AND f.final_code IS NULL AND g.frozen_worklist_digest=f.digest)
 OR EXISTS(SELECT 1 FROM retirement_observation_receipts b JOIN retirement_batch_gap_attempts g ON g.provenance_id=b.provenance_id JOIN remote_operation_records o ON o.operation_ulid=g.observer_operation_ulid AND o.repository_id=g.repository_id WHERE b.id=?1 AND ((?2='consumer_fenced' AND g.phase='fenced') OR (?2='consumer_terminal' AND g.phase='ended' AND o.phase IN ('failed','cancelled','interrupted'))))",params![binding,reason],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !valid {
        return Err(recovery_required());
    }
    Ok(())
}

pub(in super::super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    if !recovery::has_table(db, TABLE).map_err(|_| recovery_required())? {
        let unsupported:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance WHERE execution_kind='batch_gap')",[],|r|r.get(0)).map_err(|_|recovery_required())?;
        return if unsupported {
            Err(recovery_required())
        } else {
            Ok(())
        };
    }
    let mut q=db.prepare("SELECT operation_ulid,owner_epoch FROM retirement_batch_gap_attempts ORDER BY operation_ulid,owner_epoch").map_err(|_|recovery_required())?;
    let rows = q
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|_| recovery_required())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| recovery_required())?;
    for (op, epoch) in rows {
        let a = read_attempt(db, &op, epoch)?;
        let (p, f) = original(db, &op)?;
        let current = current_epoch(db, &p)?;
        let (_, cursor, stage) = batch::projected(db, &p, &f)?;
        if p.phase != "fenced"
            || a.original != p.epoch
            || a.repository != p.repository
            || a.root != p.root
            || a.request != p.request
            || a.digest != f.digest
            || a.cursor != cursor as i64
            || stage != "active"
            || a.epoch > current
            || (a.epoch < current) != (a.phase == "fenced")
        {
            return Err(recovery_required());
        }
        let previous = if a.previous == p.epoch {
            p.request_attempt
        } else {
            let old = read_attempt(db, &op, a.previous)?;
            if old.phase != "fenced" || old.original != a.original {
                return Err(recovery_required());
            }
            old.request_attempt
        };
        let request_attempt: i64 = db
            .query_row(
                "SELECT attempt FROM request_records WHERE request_ulid=?1",
                [&a.request],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        if a.request_attempt <= previous || a.request_attempt > request_attempt {
            return Err(recovery_required());
        }
        let remote = readonly(db, &p, &a, a.phase == "fenced")?;
        let active = matches!(
            remote.phase,
            RemoteOperationPhase::Reserved
                | RemoteOperationPhase::Advertising
                | RemoteOperationPhase::Persisting
        );
        let configuration_fenced = remote.phase == RemoteOperationPhase::Interrupted
            && remote.outcome
                == Some(crate::repository::RemoteOutcomeCategory::ConfigurationRequired)
            && state::generation(db, p.repository)? != p.generation;
        let complete = remote.phase == RemoteOperationPhase::Completed
            && remote.outcome == Some(crate::repository::RemoteOutcomeCategory::Completed);
        if (a.phase == "active" && (!active && !configuration_fenced || a.ended.is_some()))
            || (a.phase != "active" && active)
            || (active && remote.outcome.is_some())
            || (remote.phase == RemoteOperationPhase::Completed
                && (!complete || a.ended.as_deref() != Some("completed")))
            || (a.ended.as_deref() == Some("completed") && !complete)
            || (a.ended.as_deref() == Some("stopped") && complete)
            || (a.phase == "ended" && a.ended.is_none())
            || (a.phase == "fenced"
                && a.ended.is_none()
                && (remote.phase != RemoteOperationPhase::Interrupted
                    || remote.outcome
                        != Some(crate::repository::RemoteOutcomeCategory::RepositoryUnavailable)))
        {
            return Err(recovery_required());
        }
        if let Some(provenance) = a.provenance {
            let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance p JOIN retirement_observation_receipts b ON b.provenance_id=p.id WHERE p.id=?1 AND p.repository_id=?2 AND p.source_operation_ulid=?3 AND p.source_owner_epoch=?4 AND p.configuration_generation=?5 AND p.remote_name=?6 AND p.primary_branch=?7 AND p.endpoint_digest=?8 AND p.execution_kind='batch_gap' AND p.execution_parent_ulid IS NULL AND p.execution_parent_epoch IS NULL AND p.execution_request_ulid=?9 AND p.execution_request_attempt=?10 AND p.execution_batch_operation_ulid=?11 AND p.execution_batch_attempt_epoch=?12 AND b.consumer_kind='batch' AND b.consumer_operation_ulid=?11 AND b.consumer_request_ulid=?9 AND b.consumer_request_attempt=?10 AND b.consumer_attempt_epoch=?12 AND b.frozen_worklist_digest=?13 AND b.repository_id=?2 AND b.configuration_generation=?5)",params![provenance,a.repository,a.observer,a.observer_epoch,p.generation,p.remote,p.primary,p.endpoint,a.request,a.request_attempt,a.operation,a.epoch,a.digest],|r|r.get(0)).map_err(|_|recovery_required())?;
            if !valid || remote.completed_step != Some(RemoteOperationSafePoint::AfterBatchCommit) {
                return Err(recovery_required());
            }
            let (pins,releases):(i64,i64)=db.query_row("SELECT (SELECT count(*) FROM retirement_observation_pins WHERE provenance_id=?1),(SELECT count(*) FROM retirement_observation_pin_releases WHERE provenance_id=?1)",[provenance],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
            let retained = a.phase == "active"
                || (a.phase == "ended"
                    && remote.phase == RemoteOperationPhase::Completed
                    && remote.outcome == Some(crate::repository::RemoteOutcomeCategory::Completed));
            if (pins, releases) != if retained { (1, 0) } else { (0, 1) } {
                return Err(recovery_required());
            }
            if !retained {
                let reason = if a.ended.as_deref() == Some("stopped") {
                    "consumer_terminal"
                } else {
                    "consumer_fenced"
                };
                let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_observation_pin_releases WHERE provenance_id=?1 AND reason=?2)",params![provenance,reason],|r|r.get(0)).map_err(|_|recovery_required())?;
                if !valid {
                    return Err(recovery_required());
                }
            }
        } else if remote.phase == RemoteOperationPhase::Completed {
            return Err(recovery_required());
        }
    }
    let orphan:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance p WHERE p.execution_kind='batch_gap' AND NOT EXISTS(SELECT 1 FROM retirement_batch_gap_attempts g WHERE g.provenance_id=p.id))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if orphan {
        return Err(recovery_required());
    }
    if recovery::has_table(db, "poll_retirement_batches").map_err(|_| recovery_required())?
        && recovery::has_table(db, "poll_retirement_freezes").map_err(|_| recovery_required())?
    {
        let missing:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_batches b WHERE b.owner_epoch>(SELECT f.owner_epoch FROM poll_retirement_freezes f WHERE f.operation_ulid=b.operation_ulid) AND NOT EXISTS(SELECT 1 FROM retirement_batch_gap_attempts g WHERE g.operation_ulid=b.operation_ulid AND g.owner_epoch=b.owner_epoch))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
        if missing {
            return Err(recovery_required());
        }
    }
    Ok(())
}
