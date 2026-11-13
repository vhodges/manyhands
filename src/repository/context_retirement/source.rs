//! Owned original-P advertisement capture and per-target enrollment. Ordinary
//! and automatic polling never receive this private, non-Clone capability.
use super::{
    pass::{self, Candidate},
    record,
};
use crate::repository::{
    self, OperationFamily, OperationId, RemoteOperationPhase, RemoteOperationPriority,
    RemoteOperationTarget, RemoteRefPlan, RepositoryError, RepositoryService,
    cleanup::ContextCleanup,
    mutation::Accepted,
    remote::{observation::ObservationConfiguration, state},
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use state::recovery_required;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(test)]
thread_local! {static FAIL_CAPTURE:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};}
#[cfg(test)]
pub(super) fn fail_capture() {
    FAIL_CAPTURE.with(|s| s.set(true));
}

pub(in super::super) struct OriginalPollSource {
    pub(super) scope: Arc<()>,
    pub(super) accepted: Accepted,
    pub(super) repository: i64,
    pub(super) epoch: i64,
    pub(super) generation: i64,
    pub(super) nonce: String,
    pub(super) plan: RemoteRefPlan,
    pub(super) configuration: ObservationConfiguration,
    pub(super) root: PathBuf,
    branches: std::cell::RefCell<std::collections::BTreeMap<usize, Candidate>>,
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
        r: &state::StoredRemoteOperation,
        a: &Accepted,
    ) -> Result<Self, RepositoryError> {
        if a.family != Some(OperationFamily::Remote)
            || a.operation_id != r.operation_id
            || r.phase != RemoteOperationPhase::Reserved
            || r.completed_step.is_some()
            || pass::validate_parent(tx, a, true)?
        {
            return Err(recovery_required());
        }
        let plan = repository::remote::observation::observation_plan(root)
            .map_err(|_| recovery_required())?;
        let configuration = ObservationConfiguration::capture_owned(service, tx, root, &plan)?;
        let nonce:String=tx.query_row("SELECT live_nonce FROM poll_retirement_passes WHERE operation_ulid=?1 AND source_owner_epoch IS NULL AND image IS NULL AND provenance_id IS NULL",[a.operation_id.to_string()],|r|r.get(0)).map_err(|_|recovery_required())?;
        tx.execute("UPDATE poll_retirement_passes SET source_owner_epoch=?2,configuration_generation=?3,remote_name=?4,primary_branch=?5,endpoint_digest=?6 WHERE operation_ulid=?1",params![a.operation_id.to_string(),r.owner_epoch,r.generation,plan.remote_name(),plan.primary_branch(),configuration.endpoint_digest().as_slice()]).map_err(|_|recovery_required())?;
        let this = Self {
            scope: Arc::clone(&service.remote_reservation_scope),
            accepted: Accepted {
                request_id: a.request_id,
                attempt: a.attempt,
                scope: a.scope.clone(),
                family: a.family,
                operation_id: a.operation_id,
                position: a.position.clone(),
                expected: a.expected.clone(),
            },
            repository: id,
            epoch: r.owner_epoch,
            generation: r.generation,
            nonce,
            plan,
            configuration,
            root: root.into(),
            branches: Default::default(),
        };
        this.validate(service, tx, id, r)?;
        Ok(this)
    }
    pub(in super::super) fn validate_ownership(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        r: &state::StoredRemoteOperation,
    ) -> Result<bool, RepositoryError> {
        if !Arc::ptr_eq(&self.scope, &service.remote_reservation_scope)
            || id != self.repository
            || r.operation_id != self.accepted.operation_id
            || r.owner_epoch != self.epoch
            || r.generation != self.generation
            || state::generation(tx, id)? != self.generation
            || r.poll_kind != Some(state::PollKind::Update)
            || r.priority != RemoteOperationPriority::Manual
            || r.target != RemoteOperationTarget::for_poll(&self.plan)
            || !self
                .configuration
                .matches_owned(service, tx, &self.root, &self.plan)?
        {
            return Err(recovery_required());
        }
        let nonce:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_passes WHERE operation_ulid=?1 AND live_nonce=?2 AND source_owner_epoch=?3 AND image IS NULL)",params![self.accepted.operation_id.to_string(),self.nonce,self.epoch],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !nonce {
            return Err(recovery_required());
        }
        repository::recovery::require_no_pending_local(tx, id)?;
        pass::validate_parent(tx, &self.accepted, true)
    }
    pub(in super::super) fn validate(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        r: &state::StoredRemoteOperation,
    ) -> Result<(), RepositoryError> {
        if self.validate_ownership(service, tx, id, r)? || r.cancel_requested {
            return Err(recovery_required());
        }
        Ok(())
    }
    pub(super) fn validate_current(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
    ) -> Result<(), RepositoryError> {
        let r = state::read_operation(tx, id, self.accepted.operation_id)?
            .ok_or_else(recovery_required)?;
        self.validate(service, tx, id, &r)
    }
    pub(in super::super) fn capture(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        r: &state::StoredRemoteOperation,
        batch: i64,
    ) -> Result<(), RepositoryError> {
        self.validate(service, tx, id, r)?;
        let observed:i64=tx.query_row("SELECT observed_at FROM remote_observation_batches WHERE id=?1 AND repository_id=?2 AND is_current=1 AND configuration_generation=?3",params![batch,id,self.generation],|r|r.get(0)).map_err(|_|recovery_required())?;
        tx.execute("INSERT INTO retirement_advertisement_provenance(repository_id,source_operation_ulid,source_owner_epoch,observation_batch_id,observed_at,configuration_generation,remote_name,primary_branch,endpoint_digest,execution_kind,execution_parent_ulid,execution_parent_epoch,execution_request_ulid,execution_request_attempt,execution_batch_operation_ulid,execution_batch_attempt_epoch) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'poll_parent',?2,?3,?10,?11,?2,1)",params![id,r.operation_id.to_string(),self.epoch,batch,observed,self.generation,self.plan.remote_name(),self.plan.primary_branch(),self.configuration.endpoint_digest().as_slice(),self.accepted.request_id.to_string(),self.accepted.attempt]).map_err(|_|recovery_required())?;
        let provenance = tx.last_insert_rowid();
        tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id) VALUES(?1,'source_attempt',?2,?3,?4,?5)",params![provenance,r.operation_id.to_string(),self.epoch,id,batch]).map_err(|_|recovery_required())?;
        tx.execute("UPDATE poll_retirement_passes SET provenance_id=?2 WHERE operation_ulid=?1 AND provenance_id IS NULL",params![r.operation_id.to_string(),provenance]).map_err(|_|recovery_required())?;
        #[cfg(test)]
        if FAIL_CAPTURE.with(|s| s.replace(false)) {
            return Err(recovery_required());
        }
        record::audit_connection(tx)
    }
    pub(in super::super) fn capture_target(
        &self,
        service: &RepositoryService,
        git: &git2::Repository,
        cleanup: &ContextCleanup,
        ordinal: usize,
        row: &repository::PollTargetReport,
    ) -> Result<(), RepositoryError> {
        if !matches!(
            row.disposition(),
            repository::PollDisposition::Closed | repository::PollDisposition::Promoted
        ) {
            return Ok(());
        }
        let kind = row.kind().ok_or_else(recovery_required)?;
        let item = row.item_id().ok_or_else(recovery_required)?;
        let request = super::RetireContextRequest {
            root: self.root.clone(),
            kind,
            item_id: item.clone(),
            operation_id: OperationId::new(),
        };
        let mut c = Candidate {
            poll_ordinal: ordinal,
            kind: repository::authoring_kind_segment(&kind).into(),
            item_id: item.to_string(),
            resources: "invalid".into(),
            tip: row.after_oid().map(|o| o.to_string()),
            nonce: None,
            materialization: None,
            refusal: None,
            child_request: repository::mutation::RequestId::new().to_string(),
        };
        let mut selection = None;
        match super::run::branch_only(git, &request) {
            Ok(Some(tip)) => {
                c.resources = "branch_only".into();
                c.tip = Some(tip.to_string());
            }
            _ => match super::execution::initial_inventory(git, &self.root, &request) {
                super::execution::Inventory::Absent => {
                    c.resources = "absent".into();
                    c.tip = None;
                }
                super::execution::Inventory::PresentOrBranchOnly => {
                    match super::instance::Selection::capture(git, &request, cleanup) {
                        Ok(s) => {
                            c.resources = "present".into();
                            c.tip = Some(s.tip().to_string());
                            c.nonce = Some(s.nonce().to_string());
                            selection = Some(s);
                        }
                        Err(code) => {
                            c.refusal = Some(code.as_str().into());
                        }
                    }
                }
                _ => c.refusal = Some("recovery_required".into()),
            },
        }
        if row.after_oid().map(|o| o.to_string()) != c.tip && c.refusal.is_none() {
            return Err(recovery_required());
        }
        state::with_transaction(service, &self.root, |tx, id| {
            self.validate_current(service, tx, id)?;
            match pass::materialization(tx, id, &c) {
                Ok(m) => c.materialization = m,
                Err(_) => c.refusal = Some("recovery_required".into()),
            }
            tx.execute("INSERT INTO poll_retirement_candidates(operation_ulid,poll_ordinal,candidate,verified) VALUES(?1,?2,?3,?4)",params![self.accepted.operation_id.to_string(),ordinal,pass::encode(&c)?,selection.is_none()]).map_err(|_|recovery_required())?;
            Ok(())
        })?;
        if let Some(s) = selection {
            s.install_for_poll(
                service,
                &request,
                cleanup,
                self.accepted.operation_id,
                ordinal,
                |tx, id| self.validate_current(service, tx, id),
            )?;
        }
        if c.resources == "branch_only" && c.refusal.is_none() {
            self.branches.borrow_mut().insert(ordinal, c);
        }
        Ok(())
    }
    pub(super) fn take_branch(&self, c: &Candidate) -> bool {
        self.branches.borrow_mut().remove(&c.poll_ordinal).as_ref() == Some(c)
    }
    pub(in super::super) fn freeze(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        r: &state::StoredRemoteOperation,
        report: &repository::RemotePollReport,
    ) -> Result<super::handoff::Capture, RepositoryError> {
        self.validate(service, tx, id, r)?;
        super::handoff::freeze(service, tx, self, report)
    }
}

pub(super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    pass::audit(db)?;
    super::handoff::audit(db)
}
pub(super) fn require_ended(db: &Connection, provenance: i64) -> Result<(), RepositoryError> {
    let ended:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance p JOIN remote_operation_records o ON o.repository_id=p.repository_id AND o.operation_ulid=p.source_operation_ulid JOIN poll_retirement_passes pass ON pass.operation_ulid=p.source_operation_ulid AND pass.provenance_id=p.id WHERE p.id=?1 AND pass.live=0 AND pass.image IS NULL AND (o.owner_epoch>p.source_owner_epoch OR o.phase IN ('failed','cancelled','interrupted')))",[provenance],|r|r.get(0)).map_err(|_|recovery_required())?;
    if ended {
        Ok(())
    } else {
        Err(recovery_required())
    }
}
pub(in super::super) fn close_stopped(
    tx: &Transaction<'_>,
    r: &state::StoredRemoteOperation,
) -> Result<(), RepositoryError> {
    let provenance:Option<Option<i64>>=tx.query_row("SELECT provenance_id FROM poll_retirement_passes WHERE operation_ulid=?1 AND image IS NULL AND live=1",[r.operation_id.to_string()],|r|r.get(0)).optional().map_err(|_|recovery_required())?;
    if let Some(provenance) = provenance {
        tx.execute(
            "UPDATE poll_retirement_passes SET live=0 WHERE operation_ulid=?1 AND live=1",
            [r.operation_id.to_string()],
        )
        .map_err(|_| recovery_required())?;
        if let Some(p) = provenance {
            record::release_source_pin(tx, p)?;
        }
    }
    Ok(())
}
