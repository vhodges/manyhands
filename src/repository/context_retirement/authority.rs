//! One ordinary local executor, acquired from either a real standalone observer
//! or a sealed original-live initial input with no fabricated observer identity.
use super::super::request_scope::Scope;
use super::super::{ContextRetirementReport, RetireContextRequest};
use crate::repository::{
    OperationId, RemoteOperationAction, RemoteOperationPriority, RemoteOperationTarget,
    RemoteReservation, RepositoryError, RepositoryService, authoring_kind_segment, local_sync,
    recovery,
    remote::{observation::ObservationConfiguration, reservation, state},
};
use rusqlite::{OptionalExtension, Transaction, params};
use state::recovery_required;
use std::{path::PathBuf, sync::Arc};

#[cfg(test)]
#[path = "authority_tests.rs"]
mod tests;

pub(in super::super) enum BranchClaim {
    Execute(Box<RemoteReservation>),
    Refused(ContextRetirementReport),
}

/// Non-Clone, process-only authority, created solely by the atomic claim below.
pub(in super::super::super) struct ObserverExecution {
    scope: Arc<()>,
    operation: OperationId,
    repository: i64,
    root: PathBuf,
    epoch: i64,
    observer: Option<OperationId>,
    observer_epoch: Option<i64>,
    generation: i64,
    plan: crate::repository::RemoteRefPlan,
    configuration: ObservationConfiguration,
    request_scope: Scope,
    cancellation: crate::repository::CancellationToken,
    initial: Option<super::super::handoff::InitialInput>,
}

pub(in super::super) enum LocalOwner {
    Observer(Box<RemoteReservation>),
    Initial(Box<ObserverExecution>),
}
pub(in super::super) trait LocalAuthority {
    fn retirement_execution(&self) -> Result<&ObserverExecution, RepositoryError>;
}
impl LocalAuthority for RemoteReservation {
    fn retirement_execution(&self) -> Result<&ObserverExecution, RepositoryError> {
        RemoteReservation::retirement_execution(self)
    }
}
impl<T: LocalAuthority + ?Sized> LocalAuthority for Box<T> {
    fn retirement_execution(&self) -> Result<&ObserverExecution, RepositoryError> {
        (**self).retirement_execution()
    }
}
impl LocalAuthority for LocalOwner {
    fn retirement_execution(&self) -> Result<&ObserverExecution, RepositoryError> {
        match self {
            Self::Observer(o) => o.retirement_execution(),
            Self::Initial(e) => Ok(e),
        }
    }
}
impl From<Box<RemoteReservation>> for LocalOwner {
    fn from(o: Box<RemoteReservation>) -> Self {
        Self::Observer(o)
    }
}
impl LocalOwner {
    pub(in super::super) fn observer(&self) -> Option<&RemoteReservation> {
        match self {
            Self::Observer(o) => Some(o),
            Self::Initial(_) => None,
        }
    }
}
impl std::fmt::Debug for ObserverExecution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetirementExecution")
            .field("operation", &self.operation)
            .field("epoch", &self.epoch)
            .finish_non_exhaustive()
    }
}
impl ObserverExecution {
    pub(in super::super::super) fn set_cancellation(
        &mut self,
        cancellation: crate::repository::CancellationToken,
    ) {
        self.cancellation = cancellation;
    }
    pub(in super::super) fn cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
    pub(in super::super) fn request_scope(&self) -> Scope {
        self.request_scope
    }
    pub(in super::super) fn operation(&self) -> OperationId {
        self.operation
    }
    pub(in super::super) fn epoch(&self) -> i64 {
        self.epoch
    }
    pub(in super::super) fn plan(&self) -> &crate::repository::RemoteRefPlan {
        &self.plan
    }
    pub(in super::super) fn expectation(
        &self,
    ) -> Result<crate::repository::transport::SshScopeExpectation, RepositoryError> {
        self.configuration
            .transport_expectation(crate::repository::transport::SshDirection::Fetch)
            .ok_or_else(recovery_required)
    }

    fn local_owner(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
    ) -> Result<(), RepositoryError> {
        if id != self.repository || !Arc::ptr_eq(&self.scope, &service.remote_reservation_scope) {
            return Err(recovery_required());
        }
        super::audit_connection(tx)?;
        self.request_scope
            .validate(tx, &self.operation.to_string(), &self.root)?;
        let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch
          WHERE c.operation_ulid=?1 AND c.repository_id=?2 AND o.root_path=?3 AND o.state!='completed'
            AND c.owner_epoch=?4 AND a.phase='active' AND a.observer_operation_ulid IS ?5 AND a.observer_owner_epoch IS ?6
            AND c.request_ulid IS ?11 AND c.batch_operation_ulid IS NULL AND a.request_attempt IS ?12 AND a.batch_attempt_epoch IS NULL
            AND c.configuration_generation=?7 AND c.remote_name=?8 AND c.primary_branch=?9 AND c.endpoint_digest=?10
            AND c.phase IN ('selected','worktree_removing','worktree_removed','branch_deleting','local_absent','index_pending'))", params![self.operation.to_string(),id,self.root.to_str(),self.epoch,self.observer.map(|o|o.to_string()),self.observer_epoch,self.generation,self.plan.remote_name(),self.plan.primary_branch(),self.configuration.endpoint_digest().as_slice(),self.request_scope.request_id(),self.request_scope.attempt()], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
        let blocked: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records o WHERE (o.repository_id=?1 OR o.root_path=?2) AND o.operation_ulid IS NOT ?3 AND (o.state!='completed' OR o.completed_step='comment_registration_pending'))
          OR EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND ((operation_ulid IS NOT ?4 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')) OR index_pending!=0 OR reconciliation_required!=0))", params![id,self.root.to_str(),self.operation.to_string(),self.observer.map(|o|o.to_string())], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
        if !valid || blocked {
            return Err(recovery_required());
        }
        Ok(())
    }

    pub(in super::super) fn validate_local(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
    ) -> Result<(), RepositoryError> {
        self.local_owner(service, tx, id)?;
        if self.cancelled() {
            return Err(recovery_required());
        }
        self.request_scope
            .effect(tx, &self.operation.to_string(), &self.root)?;
        if let Some(initial) = &self.initial {
            initial.validate(service, tx, id)?;
            let receipt = initial.receipt(tx, self.operation)?;
            let (kind, item) = initial.identity()?;
            super::require_fetch_absence(tx, receipt, id, kind, &item, &self.plan)?;
        }
        if state::generation(tx, id)? != self.generation
            || !self
                .configuration
                .matches_owned(service, tx, &self.root, &self.plan)?
        {
            return Err(recovery_required());
        }
        Ok(())
    }
    pub(in super::super) fn validate_install(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
    ) -> Result<(), RepositoryError> {
        self.validate_local(service, tx, id)
    }

    pub(in super::super::super) fn validate_observer(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        record: &state::StoredRemoteOperation,
    ) -> Result<(), RepositoryError> {
        self.validate_local(service, tx, id)?;
        let installed:bool=tx.query_row("SELECT initial_resources!='present' OR instance_verified=1 FROM context_retirement_records WHERE operation_ulid=?1",[self.operation.to_string()],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        if !installed {
            return Err(recovery_required());
        }
        if Some(record.operation_id) != self.observer
            || Some(record.owner_epoch) != self.observer_epoch
            || record.generation != self.generation
            || record.target != RemoteOperationTarget::for_poll(&self.plan)
            || record.target.action() != RemoteOperationAction::Poll
            || record.poll_kind != Some(state::PollKind::Observe)
            || record.priority != RemoteOperationPriority::Manual
        {
            return Err(recovery_required());
        }
        Ok(())
    }

    /// Receives the actual batch returned by this owned advertisement write.
    pub(in super::super::super) fn capture_advertisement(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        remote: &state::StoredRemoteOperation,
        batch: i64,
    ) -> Result<(), RepositoryError> {
        self.validate_observer(service, tx, id, remote)?;
        let observed: i64 = tx.query_row("SELECT observed_at FROM remote_observation_batches WHERE id=?1 AND repository_id=?2 AND configuration_generation=?3 AND is_current=1 AND remote_name=?4 AND primary_branch=?5", params![batch,id,self.generation,self.plan.remote_name(),self.plan.primary_branch()], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
        tx.execute("INSERT INTO retirement_advertisement_provenance(repository_id,source_operation_ulid,source_owner_epoch,observation_batch_id,observed_at,configuration_generation,remote_name,primary_branch,endpoint_digest,execution_kind,execution_parent_ulid,execution_parent_epoch,execution_request_ulid,execution_request_attempt) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'local_parent',?10,?11,?12,?13)", params![id,self.observer.map(|o|o.to_string()),self.observer_epoch,batch,observed,self.generation,self.plan.remote_name(),self.plan.primary_branch(),self.configuration.endpoint_digest().as_slice(),self.operation.to_string(),self.epoch,self.request_scope.request_id(),self.request_scope.attempt()]).map_err(RepositoryError::sqlite)?;
        let provenance = tx.last_insert_rowid();
        tx.execute("INSERT INTO retirement_observation_receipts(provenance_id,repository_id,consumer_kind,consumer_operation_ulid,consumer_attempt_epoch,configuration_generation,consumer_request_ulid,consumer_request_attempt) VALUES(?1,?2,'single',?3,?4,?5,?6,?7)", params![provenance,id,self.operation.to_string(),self.epoch,self.generation,self.request_scope.request_id(),self.request_scope.attempt()]).map_err(RepositoryError::sqlite)?;
        let binding = tx.last_insert_rowid();
        tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id) VALUES(?1,'consumer_attempt',?2,?3,?4,?5,?6)", params![provenance,self.operation.to_string(),self.epoch,id,batch,binding]).map_err(RepositoryError::sqlite)?;
        super::audit_connection(tx)
    }

    /// Consumer authority is independent of O's now-terminal execution, but
    /// requires the exact still-current live pin/binding and Fetch history.
    pub(in super::super) fn receipt(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        kind: crate::repository::AuthoringKind,
        item: &crate::canonical::ItemId,
    ) -> Result<i64, RepositoryError> {
        self.validate_local(service, tx, id)?;
        if let Some(initial) = &self.initial {
            let receipt = initial.receipt(tx, self.operation)?;
            super::require_fetch_absence(tx, receipt, id, kind, item, &self.plan)?;
            return Ok(receipt);
        }
        let receipt: i64=tx.query_row("SELECT b.id FROM retirement_observation_receipts b JOIN retirement_advertisement_provenance p ON p.id=b.provenance_id JOIN retirement_observation_pins pin ON pin.binding_id=b.id AND pin.pin_kind='consumer_attempt' JOIN remote_observation_batches batch ON batch.id=pin.observation_batch_id
          WHERE b.consumer_kind='single' AND b.consumer_operation_ulid=?1 AND b.consumer_attempt_epoch=?2 AND b.repository_id=?3
          AND p.source_operation_ulid=?4 AND p.source_owner_epoch=?5 AND b.configuration_generation=?6
          AND batch.is_current=1 AND batch.configuration_generation=?6 AND batch.repository_id=?3
          AND p.observation_batch_id=batch.id AND pin.holder_operation_ulid=?1 AND pin.holder_epoch=?2", params![self.operation.to_string(),self.epoch,id,self.observer.map(|o|o.to_string()),self.observer_epoch,self.generation], |r|r.get(0)).map_err(RepositoryError::sqlite)?;
        super::require_fetch_absence(tx, receipt, id, kind, item, &self.plan)?;
        Ok(receipt)
    }
}

#[cfg(test)]
pub(in super::super) fn claim_branch(
    service: &RepositoryService,
    request: &RetireContextRequest,
    tip: git2::Oid,
    plan: crate::repository::RemoteRefPlan,
    configuration: ObservationConfiguration,
) -> Result<BranchClaim, RepositoryError> {
    claim_scoped(
        service,
        request,
        tip,
        plan,
        configuration,
        None,
        Scope::Domain,
    )
}
#[cfg(test)]
pub(in super::super) fn claim_present(
    service: &RepositoryService,
    request: &RetireContextRequest,
    selection: &super::super::instance::Selection,
    plan: crate::repository::RemoteRefPlan,
    configuration: ObservationConfiguration,
) -> Result<BranchClaim, RepositoryError> {
    claim_scoped(
        service,
        request,
        selection.tip(),
        plan,
        configuration,
        Some(selection.nonce()),
        Scope::Domain,
    )
}
pub(in super::super) fn claim_scoped(
    service: &RepositoryService,
    request: &RetireContextRequest,
    tip: git2::Oid,
    plan: crate::repository::RemoteRefPlan,
    configuration: ObservationConfiguration,
    nonce: Option<OperationId>,
    scope: Scope,
) -> Result<BranchClaim, RepositoryError> {
    state::with_transaction(service, &request.root, |tx, id| {
        super::audit_connection(tx)?;
        let operation = request.operation_id.to_string();
        scope.effect(tx, &operation, &request.root)?;
        if local_sync::record::foreign_id_collision(tx, &operation, None)?
            || local_sync::record::owned_identity(tx, &operation)?
            || super::owned_identity(tx, &operation)?
        {
            return Err(recovery_required());
        }
        recovery::require_no_pending_local(tx, id)?;
        let blocked: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND (phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') OR index_pending!=0 OR reconciliation_required!=0))", [id],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        if blocked {
            return Err(recovery_required());
        }
        if !configuration.matches_owned(service, tx, &request.root, &plan)? {
            return Err(recovery_required());
        }
        state::configure_endpoints(tx, id, &plan, &configuration.endpoint_digest())?;
        let generation = state::generation(tx, id)?;
        let materialization: Option<(String,String,String)>=tx.query_row("SELECT operation_ulid,target_oid,step FROM remote_poll_materializations WHERE repository_id=?1 AND kind=?2 AND item_id=?3", params![id,authoring_kind_segment(&request.kind),request.item_id.to_string()], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(RepositoryError::sqlite)?;
        let count:i64=tx.query_row("SELECT count(*) FROM remote_poll_materializations WHERE repository_id=?1 AND item_id=?2",params![id,request.item_id.to_string()],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        let valid = count == i64::from(materialization.is_some())
            && if let Some((owner, oid, step)) = &materialization {
                step=="complete" && OperationId::parse(owner).is_ok_and(|v|v.to_string()==*owner) && git2::Oid::from_str(oid).is_ok_and(|v|!v.is_zero() && v.to_string()==*oid)
                && tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND operation_ulid=?2 AND action='poll' AND poll_kind='update')",params![id,owner],|r|r.get::<_,bool>(0)).map_err(RepositoryError::sqlite)?
            } else {
                true
            };
        let captured = materialization.as_ref().filter(|_| valid);
        let report = ContextRetirementReport::refused(crate::results::ResultCode::RecoveryRequired);
        tx.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,?3,'retire_context',?4,?5,?6)",params![id,request.root.to_str(),operation,format!("authoring-context-v1/{}/{}/Retire",authoring_kind_segment(&request.kind),request.item_id),if valid {"created"}else{"completed"},service.clock.now().unix_timestamp()]).map_err(RepositoryError::sqlite)?;
        let local = tx.last_insert_rowid();
        tx.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,selection_epoch,instance_verified,remote_name,primary_branch,configuration_generation,endpoint_digest,materialization_capture,materialization_operation_ulid,materialization_oid,materialization_step,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis,final_code,final_result,instance_nonce,request_ulid) VALUES(?1,?2,?3,1,?4,?5,?18,?6,1,0,?7,?8,?9,?10,?11,?12,?13,?14,0,1,?15,'none','none','none',?16,?17,?19,?20)",params![operation,local,id,authoring_kind_segment(&request.kind),request.item_id.to_string(),tip.to_string(),plan.remote_name(),plan.primary_branch(),generation,configuration.endpoint_digest().as_slice(),if !valid {"unverified"}else if captured.is_some(){"complete"}else{"absent"},captured.map(|v|&v.0),captured.map(|v|&v.1),captured.map(|v|&v.2),if valid{"selected"}else{"refused"},(!valid).then_some(report.code.as_str()),if valid{None}else{Some(report.encode()?)},if nonce.is_some(){"present"}else{"branch_only"},nonce.map(|v|v.to_string()),scope.request_id()]).map_err(RepositoryError::sqlite)?;
        tx.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase,request_attempt) VALUES(?1,1,?2,?3,?4)",params![operation,id,if valid{"active"}else{"ended"},scope.attempt()]).map_err(RepositoryError::sqlite)?;
        if !valid {
            super::audit_connection(tx)?;
            return Ok(BranchClaim::Refused(report));
        }
        let observer = OperationId::new();
        if local_sync::record::foreign_id_collision(tx, &observer.to_string(), None)?
            || super::owned_identity(tx, &observer.to_string())?
        {
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
        let remote = state::read_operation(tx, id, observer)?.ok_or_else(recovery_required)?;
        if tx.execute("UPDATE context_retirement_attempts SET observer_operation_ulid=?2,observer_owner_epoch=?3 WHERE operation_ulid=?1 AND owner_epoch=1 AND phase='active' AND observer_operation_ulid IS NULL",params![operation,observer.to_string(),remote.owner_epoch]).map_err(RepositoryError::sqlite)?!=1 { return Err(recovery_required()); }
        let execution = ObserverExecution {
            scope: Arc::clone(&service.remote_reservation_scope),
            operation: request.operation_id,
            repository: id,
            root: request.root.clone(),
            epoch: 1,
            observer: Some(observer),
            observer_epoch: Some(remote.owner_epoch),
            generation,
            plan,
            configuration,
            request_scope: scope,
            cancellation: crate::repository::CancellationToken::new(),
            initial: None,
        };
        if nonce.is_some() {
            execution.validate_install(service, tx, id)?;
        } else {
            execution.validate_observer(service, tx, id, &remote)?;
        }
        Ok(BranchClaim::Execute(Box::new(
            reservation::retirement_token(service, id, &remote, execution),
        )))
    })
}

pub(in super::super) fn claim_initial(
    service: &RepositoryService,
    request: &RetireContextRequest,
    input: super::super::handoff::InitialInput,
    scope: Scope,
    cancellation: crate::repository::CancellationToken,
) -> Result<LocalOwner, RepositoryError> {
    let plan = input.plan().clone();
    let configuration = input.configuration().clone();
    let generation = input.generation();
    state::with_transaction(service, &request.root, |tx, id| {
        super::audit_connection(tx)?;
        input.validate(service, tx, id)?;
        if super::super::pass::blocking_debt(tx, id)? {
            return Err(recovery_required());
        }
        scope.effect(tx, &request.operation_id.to_string(), &request.root)?;
        if !super::super::pass::unclaimed(tx, &request.operation_id.to_string(), scope)? {
            return Err(recovery_required());
        }
        recovery::require_no_pending_local(tx, id)?;
        let c = &input.candidate;
        if super::super::pass::materialization(tx, id, c)? != c.materialization {
            return Err(recovery_required());
        }
        let m = c.materialization.as_ref();
        let operation = request.operation_id.to_string();
        tx.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,?3,'retire_context',?4,'created',?5)",params![id,request.root.to_str(),operation,format!("authoring-context-v1/{}/{}/Retire",c.kind,c.item_id),service.clock.now().unix_timestamp()]).map_err(RepositoryError::sqlite)?;
        let local = tx.last_insert_rowid();
        tx.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,selection_epoch,instance_nonce,instance_verified,remote_name,primary_branch,configuration_generation,endpoint_digest,request_ulid,materialization_capture,materialization_operation_ulid,materialization_oid,materialization_step,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) VALUES(?1,?2,?3,1,?4,?5,?6,?7,1,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,0,1,'selected','none','none','none')",params![operation,local,id,c.kind,c.item_id,c.resources,c.tip,c.nonce,c.resources=="present",plan.remote_name(),plan.primary_branch(),generation,configuration.endpoint_digest().as_slice(),scope.request_id(),if m.is_some(){"complete"}else{"absent"},m.map(|m|&m.0),m.map(|m|&m.1),m.map(|m|&m.2)]).map_err(RepositoryError::sqlite)?;
        tx.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,request_attempt,phase) VALUES(?1,1,?2,?3,'active')",params![operation,id,scope.attempt()]).map_err(RepositoryError::sqlite)?;
        input.bind(tx, request, scope)?;
        if c.resources == "absent" {
            let git = git2::Repository::open(&request.root).map_err(|_| recovery_required())?;
            super::super::run::require_absent(&git, request)?;
            super::super::run::require_materialization(tx, request.operation_id, true)?;
            tx.execute("UPDATE context_retirement_records SET phase='index_pending',absence_basis='initial',materialization_settled=1 WHERE operation_ulid=?1",[&operation]).map_err(RepositoryError::sqlite)?;
            tx.execute("UPDATE operation_records SET state='authoritative_observed',completed_step='context_retirement_absent' WHERE operation_ulid=?1",[&operation]).map_err(RepositoryError::sqlite)?;
            tx.execute(
                "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                [id],
            )
            .map_err(RepositoryError::sqlite)?;
        }
        super::audit_connection(tx)?;
        Ok(())
    })?;
    let execution = ObserverExecution {
        scope: Arc::clone(&service.remote_reservation_scope),
        operation: request.operation_id,
        repository: state::with_transaction(service, &request.root, |_, id| Ok(id))?,
        root: request.root.clone(),
        epoch: 1,
        observer: None,
        observer_epoch: None,
        generation,
        plan,
        configuration,
        request_scope: scope,
        cancellation,
        initial: Some(input),
    };
    Ok(LocalOwner::Initial(Box::new(execution)))
}

pub(in super::super) fn bind_reentry_observer(
    service: &RepositoryService,
    tx: &Transaction<'_>,
    id: i64,
    request: &RetireContextRequest,
    epoch: i64,
    configuration: ObservationConfiguration,
    scope: Scope,
) -> Result<Box<RemoteReservation>, RepositoryError> {
    scope.effect(tx, &request.operation_id.to_string(), &request.root)?;
    let (remote_name,primary,generation):(String,String,i64)=tx.query_row("SELECT c.remote_name,c.primary_branch,c.configuration_generation FROM context_retirement_records c JOIN context_retirement_reentries rr ON rr.operation_ulid=c.operation_ulid AND rr.owner_epoch=c.owner_epoch WHERE c.operation_ulid=?1 AND c.owner_epoch=?2 AND rr.purpose IN ('observe','resume_worktree') AND c.request_ulid IS ?3 AND c.batch_operation_ulid IS NULL",params![request.operation_id.to_string(),epoch,scope.request_id()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(RepositoryError::sqlite)?;
    let plan = crate::repository::RemoteRefPlan::from_configuration(&remote_name, &primary)
        .map_err(|_| recovery_required())?;
    let observer = OperationId::new();
    if local_sync::record::foreign_id_collision(tx, &observer.to_string(), None)?
        || super::owned_identity(tx, &observer.to_string())?
    {
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
    let remote = state::read_operation(tx, id, observer)?.ok_or_else(recovery_required)?;
    if tx.execute("UPDATE context_retirement_attempts SET observer_operation_ulid=?3,observer_owner_epoch=?4 WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active' AND observer_operation_ulid IS NULL",params![request.operation_id.to_string(),epoch,observer.to_string(),remote.owner_epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
    let execution = ObserverExecution {
        scope: Arc::clone(&service.remote_reservation_scope),
        operation: request.operation_id,
        repository: id,
        root: request.root.clone(),
        epoch,
        observer: Some(observer),
        observer_epoch: Some(remote.owner_epoch),
        generation,
        plan,
        configuration,
        request_scope: scope,
        cancellation: crate::repository::CancellationToken::new(),
        initial: None,
    };
    execution.validate_install(service, tx, id)?;
    Ok(Box::new(reservation::retirement_token(
        service, id, &remote, execution,
    )))
}

pub(in super::super) fn refuse_fresh_scoped(
    service: &RepositoryService,
    request: &RetireContextRequest,
    code: crate::results::ResultCode,
    scope: Scope,
) -> Result<ContextRetirementReport, RepositoryError> {
    state::with_transaction(service, &request.root, |tx, id| {
        super::audit_connection(tx)?;
        let operation = request.operation_id.to_string();
        let cancelled = scope.validate(tx, &operation, &request.root)?;
        if local_sync::record::foreign_id_collision(tx, &operation, None)?
            || local_sync::record::owned_identity(tx, &operation)?
            || (super::owned_identity(tx, &operation)?
                && !super::super::pass::unclaimed(tx, &operation, scope)?)
        {
            return Err(recovery_required());
        }
        recovery::require_no_pending_local(tx, id)?;
        let blocked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND (phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') OR index_pending!=0 OR reconciliation_required!=0))",[id],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        if blocked {
            return Err(recovery_required());
        }
        let report = ContextRetirementReport::refused(if cancelled {
            crate::results::ResultCode::Cancelled
        } else {
            code
        });
        tx.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,?3,'retire_context',?4,'completed',?5)",params![id,request.root.to_str(),operation,format!("authoring-context-v1/{}/{}/Retire",authoring_kind_segment(&request.kind),request.item_id),service.clock.now().unix_timestamp()]).map_err(RepositoryError::sqlite)?;
        let local = tx.last_insert_rowid();
        tx.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis,final_code,final_result,request_ulid) VALUES(?1,?2,?3,1,?4,?5,'invalid',1,0,'unverified',0,1,'refused','none','none','none',?6,?7,?8)",params![operation,local,id,authoring_kind_segment(&request.kind),request.item_id.to_string(),report.code.as_str(),report.encode()?,scope.request_id()]).map_err(RepositoryError::sqlite)?;
        tx.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase,request_attempt) VALUES(?1,1,?2,'ended',?3)",params![operation,id,scope.attempt()]).map_err(RepositoryError::sqlite)?;
        if cancelled {
            tx.execute("UPDATE request_records SET cancel_requested=0 WHERE request_ulid=?1 AND attempt=?2 AND state='accepted'",params![scope.request_id(),scope.attempt()]).map_err(RepositoryError::sqlite)?;
        }
        super::audit_connection(tx)?;
        Ok(report)
    })
}

pub(in super::super) fn refuse_execution(
    service: &RepositoryService,
    root: &std::path::Path,
    owner: &impl LocalAuthority,
    code: crate::results::ResultCode,
) -> Result<ContextRetirementReport, RepositoryError> {
    refuse(service, root, owner, code, None)
}

pub(in super::super) fn refuse_before_branch_effect(
    service: &RepositoryService,
    root: &std::path::Path,
    owner: &impl LocalAuthority,
    code: crate::results::ResultCode,
    uninvoked: super::super::run::UninvokedBranch,
) -> Result<ContextRetirementReport, RepositoryError> {
    refuse(
        service,
        root,
        owner,
        code,
        Some(NoEffect::Branch(uninvoked)),
    )
}
enum NoEffect {
    Branch(super::super::run::UninvokedBranch),
    Worktree(super::super::run::UninvokedWorktree),
}
pub(in super::super) fn refuse_before_worktree_effect(
    service: &RepositoryService,
    root: &std::path::Path,
    owner: &impl LocalAuthority,
    witness: super::super::run::UninvokedWorktree,
) -> Result<ContextRetirementReport, RepositoryError> {
    refuse(
        service,
        root,
        owner,
        crate::results::ResultCode::ExternalChange,
        Some(NoEffect::Worktree(witness)),
    )
}

fn refuse(
    service: &RepositoryService,
    root: &std::path::Path,
    owner: &impl LocalAuthority,
    code: crate::results::ResultCode,
    uninvoked: Option<NoEffect>,
) -> Result<ContextRetirementReport, RepositoryError> {
    let execution = owner.retirement_execution()?;
    state::with_transaction(service, root, |tx, id| {
        execution.local_owner(service, tx, id)?;
        let saved_cancelled =
            execution
                .request_scope
                .validate(tx, &execution.operation.to_string(), root)?;
        let cancelled = saved_cancelled || execution.cancelled();
        let kind = match &uninvoked {
            None => 0,
            Some(NoEffect::Branch(_)) => 1,
            Some(NoEffect::Worktree(_)) => 2,
        };
        let safe:bool=tx.query_row("SELECT absence_basis='none' AND materialization_settled=0 AND
          ((?2=0 AND worktree_removal='none' AND phase='selected' AND branch_removal='none' AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid)) OR
           (?2=1 AND worktree_removal='none' AND initial_resources='branch_only' AND phase='branch_deleting' AND branch_removal='possible' AND (SELECT count(*) FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid)=1) OR
           (?2=2 AND worktree_removal='possible' AND branch_removal='none' AND initial_resources='present' AND phase='worktree_removing' AND (SELECT count(*) FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid)=1))
          FROM context_retirement_records c WHERE operation_ulid=?1",params![execution.operation.to_string(),kind],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        if !safe {
            return Err(recovery_required());
        }
        if let Some(observer) = execution.observer {
            let remote = state::read_operation(tx, id, observer)?.ok_or_else(recovery_required)?;
            if Some(remote.owner_epoch) != execution.observer_epoch
                || remote.poll_kind != Some(state::PollKind::Observe)
                || remote.target != RemoteOperationTarget::for_poll(&execution.plan)
                || remote.priority != RemoteOperationPriority::Manual
            {
                return Err(recovery_required());
            }
            tx.execute("UPDATE remote_operation_records SET phase=?2,outcome=?3,owner_epoch=owner_epoch+1 WHERE id=?1 AND phase IN ('reserved','advertising','persisting')",params![remote.id,if cancelled{"cancelled"}else{"interrupted"},if cancelled{"cancelled"}else{"repository_unavailable"}]).map_err(RepositoryError::sqlite)?;
        }
        if let Some(uninvoked) = uninvoked {
            match uninvoked {
                NoEffect::Branch(w) => w.settle(tx, execution.operation, execution.epoch)?,
                NoEffect::Worktree(w) => w.settle(tx, execution.operation, execution.epoch)?,
            }
        }
        let report = ContextRetirementReport::refused(if cancelled {
            crate::results::ResultCode::Cancelled
        } else {
            code
        });
        if saved_cancelled && tx.execute("UPDATE request_records SET cancel_requested=0 WHERE request_ulid=?1 AND attempt=?2 AND state='accepted' AND cancel_requested=1",params![execution.request_scope.request_id(),execution.request_scope.attempt()]).map_err(RepositoryError::sqlite)?!=1 {return Err(recovery_required());}
        if tx.execute("UPDATE context_retirement_records SET phase='refused',worktree_removal='none',branch_removal='none',final_code=?2,final_result=?3 WHERE operation_ulid=?1 AND owner_epoch=?4 AND phase IN ('selected','branch_deleting','worktree_removing')",params![execution.operation.to_string(),report.code.as_str(),report.encode()?,execution.epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        if tx.execute("UPDATE context_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![execution.operation.to_string(),execution.epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        super::release_single_consumer(tx, &execution.operation.to_string(), execution.epoch)?;
        if tx.execute("UPDATE operation_records SET state='completed' WHERE operation_ulid=?1 AND action='retire_context' AND state!='completed'",[execution.operation.to_string()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        super::audit_connection(tx)?;
        Ok(report)
    })
}

pub(in super::super) fn settle_branch_after_prune(
    service: &RepositoryService,
    root: &std::path::Path,
    owner: &impl LocalAuthority,
    uninvoked: super::super::run::UninvokedBranch,
) -> Result<(), RepositoryError> {
    let execution = owner.retirement_execution()?;
    state::with_transaction(service, root, |tx, id| {
        execution.local_owner(service, tx, id)?;
        let valid:bool=tx.query_row("SELECT phase='branch_deleting' AND worktree_removal='observed' AND branch_removal='possible' AND initial_resources='present' AND absence_basis='none' AND materialization_settled=0 FROM context_retirement_records WHERE operation_ulid=?1",[execution.operation.to_string()],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        if !valid {
            return Err(recovery_required());
        }
        uninvoked.settle(tx, execution.operation, execution.epoch)?;
        if tx.execute("UPDATE context_retirement_records SET phase='worktree_removed',branch_removal='none' WHERE operation_ulid=?1 AND owner_epoch=?2",params![execution.operation.to_string(),execution.epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        super::audit_connection(tx)
    })
}

/// A new no-effect witness describes only this attempt, never the old crash.
/// Keep the old possible W debt and the local blocker even when a new proof,
/// authentication or immediate pre-prune guard refuses.
pub(in super::super) fn preserve_worktree_resumption(
    service: &RepositoryService,
    root: &std::path::Path,
    owner: &impl LocalAuthority,
    uninvoked: &mut Option<super::super::run::UninvokedWorktree>,
) -> Result<Option<crate::repository::ContextRetirementOutcome>, RepositoryError> {
    let execution = owner.retirement_execution()?;
    state::with_transaction(service, root, |tx, id| {
        execution.local_owner(service, tx, id)?;
        let prior:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_worktree_resumptions w JOIN context_retirement_records c ON c.operation_ulid=w.operation_ulid AND c.owner_epoch=w.owner_epoch WHERE w.operation_ulid=?1 AND w.owner_epoch=?2 AND c.phase='worktree_removing' AND c.worktree_removal='possible' AND c.branch_removal='none')",params![execution.operation.to_string(),execution.epoch],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        if !prior {
            return Ok(None);
        }
        if let Some(witness) = uninvoked.take() {
            witness.settle(tx, execution.operation, execution.epoch)?;
        }
        tx.execute("UPDATE remote_operation_records SET phase='interrupted',outcome='repository_unavailable' WHERE repository_id=?1 AND operation_ulid=?2 AND owner_epoch=?3 AND phase IN ('reserved','advertising','persisting')",params![id,execution.observer.map(|o|o.to_string()),execution.observer_epoch]).map_err(RepositoryError::sqlite)?;
        super::audit_connection(tx)?;
        Ok(Some(
            crate::repository::ContextRetirementOutcome::CleanupPending {
                worktree_removal: crate::repository::RetirementRemoval::Possible,
                branch_removal: crate::repository::RetirementRemoval::None,
            },
        ))
    })
}
