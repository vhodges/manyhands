//! Private A0/A1 original-P adapter and the owned A2 capture boundary. No batch
//! store, manifest producer or coordinator is installed here. Absent canonical
//! projections refuse; tests install explicitly INERT contract projections.
use crate::repository::{
    OperationFamily, OperationId, RemoteOperationPhase, RemoteOperationPriority,
    RemoteOperationTarget, RemoteRefPlan, RepositoryError, RepositoryService,
    mutation::{
        Accepted, RequestId,
        request_store::{DigestSalt, FieldValue, IntentDigest, ScopeKey},
    },
    recovery,
    remote::{observation::ObservationConfiguration, state},
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use state::recovery_required;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

const CONTRACT: &str = "poll_retirement_source_contract_v1";
#[cfg(test)]
thread_local! {static FAIL_CAPTURE:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};}
#[cfg(test)]
pub(super) fn fail_capture() {
    FAIL_CAPTURE.with(|slot| slot.set(true));
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Projection {
    pub(super) operation: String,
    pub(super) repository: i64,
    pub(super) root: String,
    pub(super) request: String,
    pub(super) request_attempt: i64,
    pub(super) epoch: i64,
    pub(super) source_epoch: i64,
    pub(super) generation: i64,
    pub(super) remote: String,
    pub(super) primary: String,
    pub(super) endpoint: Vec<u8>,
    pub(super) base: String,
    pub(super) phase: String,
    pub(super) stage: String,
}

pub(super) fn projection(
    db: &Connection,
    operation: &str,
    epoch: i64,
) -> Result<Projection, RepositoryError> {
    for table in [
        "poll_retirement_batches",
        "poll_retirement_attempts",
        "request_records",
        "request_operations",
    ] {
        if !recovery::has_table(db, table).map_err(|_| recovery_required())? {
            return Err(recovery_required());
        }
    }
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CONTRACT],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    if !marked {
        return Err(recovery_required());
    }
    let p=db.query_row("SELECT b.operation_ulid,b.repository_id,repo.root_path,b.request_ulid,a.request_attempt,a.owner_epoch,a.source_owner_epoch,b.configuration_generation,b.remote_name,b.primary_branch,b.endpoint_digest,b.base_oid,a.phase,b.stage
      FROM poll_retirement_batches b JOIN poll_retirement_attempts a USING(operation_ulid)
      JOIN repositories repo ON repo.id=b.repository_id
      WHERE b.operation_ulid=?1 AND a.owner_epoch=?2 AND typeof(a.owner_epoch)='integer' AND a.owner_epoch>0
        AND typeof(b.owner_epoch)='integer' AND b.owner_epoch>=a.owner_epoch
        AND typeof(a.request_attempt)='integer' AND a.request_attempt>0
        AND typeof(a.source_owner_epoch)='integer' AND a.source_owner_epoch>=0
        AND (SELECT count(*) FROM poll_retirement_attempts same_source WHERE same_source.operation_ulid=a.operation_ulid AND same_source.source_owner_epoch=a.source_owner_epoch)=1
        AND typeof(b.configuration_generation)='integer' AND b.configuration_generation>=0
         AND b.mode='retire_completed' AND b.stage IN ('unplanned','frozen')
        AND a.phase IN ('active','ended','fenced') AND typeof(b.endpoint_digest)='blob' AND length(b.endpoint_digest)=32",
      params![operation,epoch],|r|Ok(Projection{operation:r.get(0)?,repository:r.get(1)?,root:r.get(2)?,request:r.get(3)?,request_attempt:r.get(4)?,epoch:r.get(5)?,source_epoch:r.get(6)?,generation:r.get(7)?,remote:r.get(8)?,primary:r.get(9)?,endpoint:r.get(10)?,base:r.get(11)?,phase:r.get(12)?,stage:r.get(13)?})).map_err(|_|recovery_required())?;
    let request = RequestId::parse(&p.request).map_err(|_| recovery_required())?;
    if request.to_string() != p.request
        || !OperationId::parse(&p.operation).is_ok_and(|id| id.to_string() == p.operation)
        || !git2::Oid::from_str(&p.base)
            .is_ok_and(|oid| !oid.is_zero() && oid.to_string() == p.base)
    {
        return Err(recovery_required());
    }
    let scope = ScopeKey::from_stored(&p.root);
    let digest = IntentDigest::builder(DigestSalt::Request(request), "poll once", &scope, "")
        .field("retire_completed", FieldValue::Flag(true))
        .finish();
    // Historical attempt <= current is correlation only, never write authority.
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM request_records r WHERE r.request_ulid=?1 AND typeof(r.attempt)='integer' AND r.attempt>=?2
      AND r.scope_key=?3 AND r.command='poll once' AND r.target='' AND r.intent_digest=?4
      AND r.confirmation_ulid IS NULL AND r.expected_digest IS NULL AND r.base_ref=?5 AND r.base_oid=?6
      AND EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=r.request_ulid AND m.ordinal=0 AND m.family='remote' AND m.operation_ulid=?7)
      AND (SELECT count(*) FROM request_operations m WHERE m.request_ulid=r.request_ulid)=1
      AND (SELECT count(*) FROM request_operations m WHERE m.operation_ulid=?7)=1)",params![p.request,p.request_attempt,p.root,digest.to_string(),format!("refs/heads/{}",p.primary),p.base,p.operation],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !valid {
        return Err(recovery_required());
    }
    Ok(p)
}

/// Non-Clone process authority; cannot be obtained from a public poll report,
/// historical provenance, IDs, a manifest placeholder or a caller-set flag.
pub(in super::super) struct OriginalPollSource {
    scope: Arc<()>,
    projection: Projection,
    plan: RemoteRefPlan,
    configuration: ObservationConfiguration,
    root: PathBuf,
}
impl std::fmt::Debug for OriginalPollSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalPollSource").finish_non_exhaustive()
    }
}
impl OriginalPollSource {
    pub(in super::super) fn seal(
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        root: &Path,
        record: &state::StoredRemoteOperation,
        accepted: &Accepted,
    ) -> Result<Self, RepositoryError> {
        let epoch: i64 = tx
            .query_row(
                "SELECT owner_epoch FROM poll_retirement_batches WHERE operation_ulid=?1",
                [record.operation_id.to_string()],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        let p = projection(tx, &record.operation_id.to_string(), epoch)?;
        if p.stage != "unplanned"
            || record.phase != RemoteOperationPhase::Reserved
            || record.completed_step.is_some()
            || accepted.request_id.to_string() != p.request
            || i64::try_from(accepted.attempt).ok() != Some(p.request_attempt)
            || accepted.operation_id != record.operation_id
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
        {
            return Err(recovery_required());
        }
        let plan = super::super::remote::observation::observation_plan(root)
            .map_err(|_| recovery_required())?;
        let configuration = ObservationConfiguration::capture_owned(service, tx, root, &plan)?;
        let this = Self {
            scope: Arc::clone(&service.remote_reservation_scope),
            projection: p,
            plan,
            configuration,
            root: root.into(),
        };
        this.validate(service, tx, id, record)?;
        // Sealing cannot adopt a prior advertisement, even one left by this P.
        let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance WHERE repository_id=?1 AND source_operation_ulid=?2 AND source_owner_epoch=?3)
          OR EXISTS(SELECT 1 FROM poll_retirement_attempts WHERE operation_ulid=?2 AND owner_epoch=?4 AND active_provenance_id IS NOT NULL)",params![id,record.operation_id.to_string(),record.owner_epoch,this.projection.epoch],|r|r.get(0)).map_err(|_|recovery_required())?;
        if used {
            return Err(recovery_required());
        }
        Ok(this)
    }
    pub(in super::super) fn validate(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        record: &state::StoredRemoteOperation,
    ) -> Result<(), RepositoryError> {
        if self.validate_ownership(service, tx, id, record)? || record.cancel_requested {
            return Err(recovery_required());
        }
        Ok(())
    }

    /// Exact CURRENT ownership survives cancellation only to acknowledge STOP.
    /// The returned flag is read from that same accepted F2 row, not supplied by
    /// a caller. General owned writes/capture still use strict validate above.
    pub(in super::super) fn validate_ownership(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        record: &state::StoredRemoteOperation,
    ) -> Result<bool, RepositoryError> {
        let p = &self.projection;
        if !Arc::ptr_eq(&self.scope, &service.remote_reservation_scope)
            || id != p.repository
            || self.root.to_str() != Some(p.root.as_str())
            || record.operation_id.to_string() != p.operation
            || record.owner_epoch != p.source_epoch
            || record.generation != p.generation
            || state::generation(tx, id)? != p.generation
            || record.poll_kind != Some(state::PollKind::Update)
            || record.priority != RemoteOperationPriority::Manual
            || record.target != RemoteOperationTarget::for_poll(&self.plan)
            || self.plan.remote_name() != p.remote
            || self.plan.primary_branch() != p.primary
            || self.configuration.endpoint_digest().as_slice() != p.endpoint
            || !self
                .configuration
                .matches_owned(service, tx, &self.root, &self.plan)?
        {
            return Err(recovery_required());
        }
        let current = projection(tx, &p.operation, p.epoch)?;
        if current != *p || current.phase != "active" {
            return Err(recovery_required());
        }
        let cancelled:bool=tx.query_row("SELECT cancel_requested FROM request_records WHERE request_ulid=?1 AND attempt=?2 AND state='accepted'
          AND EXISTS(SELECT 1 FROM poll_retirement_batches WHERE operation_ulid=?3 AND owner_epoch=?4)",params![p.request,p.request_attempt,p.operation,p.epoch],|r|r.get(0)).optional().map_err(|_|recovery_required())?.ok_or_else(recovery_required)?;
        recovery::require_no_pending_local(tx, id)?;
        let other:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND operation_ulid!=?2 AND (phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') OR index_pending!=0 OR reconciliation_required!=0))",params![id,p.operation],|r|r.get(0)).map_err(|_|recovery_required())?;
        if other || record.index_pending || record.reconciliation_required {
            return Err(recovery_required());
        }
        super::record::audit_connection(tx)?;
        Ok(cancelled)
    }
    pub(in super::super) fn capture(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        record: &state::StoredRemoteOperation,
        batch: i64,
    ) -> Result<(), RepositoryError> {
        self.validate(service, tx, id, record)?;
        let p = &self.projection;
        let observed:i64=tx.query_row("SELECT observed_at FROM remote_observation_batches WHERE id=?1 AND repository_id=?2 AND is_current=1 AND configuration_generation=?3 AND remote_name=?4 AND primary_branch=?5",params![batch,id,p.generation,p.remote,p.primary],|r|r.get(0)).map_err(|_|recovery_required())?;
        tx.execute("INSERT INTO retirement_advertisement_provenance(repository_id,source_operation_ulid,source_owner_epoch,observation_batch_id,observed_at,configuration_generation,remote_name,primary_branch,endpoint_digest,execution_kind,execution_parent_ulid,execution_parent_epoch,execution_request_ulid,execution_request_attempt,execution_batch_operation_ulid,execution_batch_attempt_epoch) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'poll_parent',?2,?3,?10,?11,?2,?12)",params![id,p.operation,p.source_epoch,batch,observed,p.generation,p.remote,p.primary,p.endpoint,p.request,p.request_attempt,p.epoch]).map_err(|_|recovery_required())?;
        let provenance = tx.last_insert_rowid();
        tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id) VALUES(?1,'source_attempt',?2,?3,?4,?5)",params![provenance,p.operation,p.source_epoch,id,batch]).map_err(|_|recovery_required())?;
        if tx.execute("UPDATE poll_retirement_attempts SET active_provenance_id=?3 WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active' AND active_provenance_id IS NULL",params![p.operation,p.epoch,provenance]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
        #[cfg(test)]
        if FAIL_CAPTURE.with(|slot| slot.replace(false)) {
            return Err(recovery_required());
        }
        super::record::audit_connection(tx)
    }

    /// Only reservation's preterminal successful-pass continuation calls this.
    /// A public terminal replay cannot revalidate this ended P execution.
    pub(in super::super) fn freeze(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        record: &state::StoredRemoteOperation,
        report: &super::super::remote::RemotePollReport,
    ) -> Result<super::handoff::Capture, RepositoryError> {
        self.validate(service, tx, id, record)?;
        super::handoff::freeze(
            service,
            tx,
            &self.projection,
            self.configuration.clone(),
            report,
        )
    }
}

pub(super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    super::handoff::audit_correlations(db)?;
    if recovery::has_table(db, "poll_retirement_attempts").map_err(|_| recovery_required())? {
        let orphan:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_attempts a LEFT JOIN retirement_advertisement_provenance p ON p.id=a.active_provenance_id WHERE a.active_provenance_id IS NOT NULL AND (p.id IS NULL OR p.execution_kind!='poll_parent' OR p.source_operation_ulid!=a.operation_ulid OR p.source_owner_epoch!=a.source_owner_epoch OR p.execution_batch_attempt_epoch!=a.owner_epoch OR p.execution_request_attempt!=a.request_attempt))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
        if orphan {
            return Err(recovery_required());
        }
    }
    let mut q=db.prepare("SELECT id,source_operation_ulid,execution_batch_attempt_epoch FROM retirement_advertisement_provenance WHERE execution_kind='poll_parent'").map_err(|_|recovery_required())?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| recovery_required())?;
    for row in rows {
        let (provenance, operation, epoch) = row.map_err(|_| recovery_required())?;
        let p = projection(db, &operation, epoch)?;
        if p.stage == "frozen" {
            super::handoff::audit_source(db, &p, provenance)?;
            continue;
        }
        let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance p JOIN remote_operation_records o ON o.repository_id=p.repository_id AND o.operation_ulid=p.source_operation_ulid JOIN poll_retirement_attempts a ON a.operation_ulid=p.execution_batch_operation_ulid AND a.owner_epoch=p.execution_batch_attempt_epoch
          WHERE p.id=?1 AND p.repository_id=?2 AND p.source_operation_ulid=?3 AND p.source_owner_epoch=?4
          AND p.execution_parent_ulid=?3 AND p.execution_parent_epoch=?4 AND p.execution_batch_operation_ulid=?3
          AND p.execution_request_ulid=?5 AND p.execution_request_attempt=?6 AND p.configuration_generation=?7
          AND p.remote_name=?8 AND p.primary_branch=?9 AND p.endpoint_digest=?10
          AND a.active_provenance_id=p.id AND o.action='poll' AND o.poll_kind='update' AND o.priority='manual'
          AND o.configuration_generation=p.configuration_generation AND o.remote_name=p.remote_name AND o.primary_branch=p.primary_branch
          AND (o.owner_epoch=p.source_owner_epoch OR (a.phase='fenced' AND o.owner_epoch>p.source_owner_epoch))
          AND NOT EXISTS(SELECT 1 FROM retirement_observation_receipts b WHERE b.provenance_id=p.id)
          AND ((a.phase='active' AND (o.phase IN ('reserved','advertising','persisting','local_prepared') OR (o.phase='interrupted' AND o.outcome='configuration_required' AND o.configuration_generation!=(SELECT configuration_generation FROM remote_polling_state WHERE repository_id=p.repository_id))) AND EXISTS(SELECT 1 FROM retirement_observation_pins pin WHERE pin.provenance_id=p.id AND pin.pin_kind='source_attempt'))
            OR (a.phase IN ('ended','fenced') AND EXISTS(SELECT 1 FROM retirement_observation_pin_releases r WHERE r.provenance_id=p.id AND r.pin_kind='source_attempt' AND r.reason=CASE a.phase WHEN 'ended' THEN 'source_terminal' ELSE 'source_fenced' END)
              AND o.phase IN ('failed','cancelled','interrupted'))))",params![provenance,p.repository,p.operation,p.source_epoch,p.request,p.request_attempt,p.generation,p.remote,p.primary,p.endpoint],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !valid {
            return Err(recovery_required());
        }
    }
    super::handoff::batch::audit(db)?;
    Ok(())
}

pub(super) fn require_ended(db: &Connection, provenance: i64) -> Result<(), RepositoryError> {
    let (operation,epoch):(String,i64)=db.query_row("SELECT source_operation_ulid,execution_batch_attempt_epoch FROM retirement_advertisement_provenance WHERE id=?1 AND execution_kind='poll_parent'",[provenance],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
    let p = projection(db, &operation, epoch)?;
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_attempts a JOIN remote_operation_records o ON o.operation_ulid=a.operation_ulid JOIN retirement_advertisement_provenance p ON p.id=a.active_provenance_id
      WHERE a.operation_ulid=?1 AND a.owner_epoch=?2 AND a.phase='ended' AND a.active_provenance_id=?3
      AND o.repository_id=p.repository_id AND o.owner_epoch=p.source_owner_epoch AND o.poll_kind='update' AND o.priority='manual'
      AND o.phase IN ('failed','cancelled','interrupted') AND NOT EXISTS(SELECT 1 FROM retirement_observation_receipts b WHERE b.provenance_id=p.id))",params![operation,epoch,provenance],|r|r.get(0)).map_err(|_|recovery_required())?;
    if p.phase != "ended" || !valid {
        return Err(recovery_required());
    }
    Ok(())
}

/// Closes only an already stopped UNPLANNED source. No live update P is fenced by
/// this helper, and it grants no observation/child/consumption authority.
pub(in super::super) fn close_stopped(
    tx: &Transaction<'_>,
    record: &state::StoredRemoteOperation,
) -> Result<(), RepositoryError> {
    if !recovery::has_table(tx, "retirement_advertisement_provenance")
        .map_err(|_| recovery_required())?
    {
        return Ok(());
    }
    if !recovery::has_table(tx, "poll_retirement_attempts").map_err(|_| recovery_required())? {
        return Ok(());
    }
    let row:Option<(Option<i64>,i64)>=tx.query_row("SELECT active_provenance_id,owner_epoch FROM poll_retirement_attempts WHERE operation_ulid=?1 AND source_owner_epoch=?2",params![record.operation_id.to_string(),record.owner_epoch],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|recovery_required())?;
    let Some((provenance, epoch)) = row else {
        return Ok(());
    };
    let p = projection(tx, &record.operation_id.to_string(), epoch)?;
    if p.stage != "unplanned" {
        return Err(recovery_required());
    }
    let stopped:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id=?2 AND owner_epoch=?3 AND phase IN ('failed','cancelled','interrupted'))",params![p.operation,p.repository,p.source_epoch],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !stopped {
        return Err(recovery_required());
    }
    if p.phase == "active" {
        if tx.execute("UPDATE poll_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active' AND active_provenance_id IS ?3",params![p.operation,p.epoch,provenance]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
    } else if p.phase != "ended" {
        return Err(recovery_required());
    }
    if let Some(provenance) = provenance {
        super::record::release_source_pin(tx, provenance)?;
    }
    super::record::audit_connection(tx)
}
