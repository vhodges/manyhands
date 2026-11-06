//! Local consumer execution, deliberately not a remote observer reservation.
//! The original freeze's one-use branch lifetime never enters the index scope.
use super::*;
use crate::repository::{
    RefreshOutcome,
    cleanup::{BranchDeletion, ContextCleanup},
    context_retirement::{
        ContextRetirementOutcome, ContextRetirementReport, RetireContextRequest, RetirementRemoval,
        record, run,
    },
    recovery::IndexOwner,
};

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super::super::super) enum Boundary {
    Begin,
    IntentCommitted,
    Intent,
    LockedBranch,
    Deleted,
    BeforeClaim,
    Finalizer,
    NoEffect,
}
#[cfg(test)]
type Hook = Box<dyn FnOnce(&Connection)>;
#[cfg(test)]
thread_local! {static HOOK:std::cell::RefCell<Option<(Boundary,Hook)>>=const{std::cell::RefCell::new(None)};}
#[cfg(test)]
type ClaimHook = Box<dyn FnOnce(IndexOwner)>;
#[cfg(test)]
thread_local! {static CLAIM_HOOK:std::cell::RefCell<Option<ClaimHook>>=const{std::cell::RefCell::new(None)};}
#[cfg(test)]
pub(in super::super::super::super) fn observe_claim(hook: impl FnOnce(IndexOwner) + 'static) {
    CLAIM_HOOK.with(|slot| assert!(slot.borrow_mut().replace(Box::new(hook)).is_none()));
}
#[cfg(test)]
pub(in super::super::super::super) fn observe(
    boundary: Boundary,
    hook: impl FnOnce(&Connection) + 'static,
) {
    HOOK.with(|slot| {
        assert!(
            slot.borrow_mut()
                .replace((boundary, Box::new(hook)))
                .is_none()
        )
    });
}
#[cfg(test)]
fn boundary(point: Boundary, db: &Connection) {
    let hook = HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|(p, _)| *p == point) {
            slot.take().map(|(_, h)| h)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook(db);
    }
}

#[derive(Clone)]
pub(in super::super::super::super) struct IndexScope {
    capture: Arc<OriginalCapture>,
    ordinal: usize,
    local: i64,
}

// Non-Clone process witnesses. Created only by this live begin/intent chain,
// never from a persisted intent, result, candidate or new executor.
struct UninvokedBranch {
    operation: OperationId,
    epoch: i64,
    receipt: i64,
    primary: git2::Oid,
    proof: crate::repository::context_retirement::proof::PreservationProof,
}
pub(super) struct RefusalWitness {
    scope: IndexScope,
    intent: Option<UninvokedBranch>,
}

impl RefusalWitness {
    fn settle(
        &self,
        service: &RepositoryService,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        let scope = &self.scope;
        let p = &scope.capture.p;
        if !Arc::ptr_eq(&scope.capture.scope, &service.remote_reservation_scope)
            || scope.capture.registry != service.registry_path
        {
            return Err(recovery_required());
        }
        state::with_transaction(service, Path::new(&p.root), |tx, _| {
            #[cfg(test)]
            boundary(Boundary::NoEffect, tx);
            let (f, k, count) = scope.current(tx)?;
            let safe:bool=tx.query_row("SELECT absence_basis='none' AND materialization_settled=0 AND worktree_removal='none' AND ((?2=0 AND phase='selected' AND branch_removal='none' AND initial_primary_oid IS NULL AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid)) OR (?2=1 AND phase='branch_deleting' AND branch_removal='possible' AND (SELECT count(*) FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid)=1)) FROM context_retirement_records c WHERE operation_ulid=?1 AND owner_epoch=1",params![k.child_operation_id,self.intent.is_some()],|r|r.get(0)).map_err(|_|recovery_required())?;
            if !safe {
                return Err(recovery_required());
            }
            if let Some(w) = &self.intent {
                if w.operation.to_string() != k.child_operation_id
                    || w.epoch != k.selection_epoch
                    || w.receipt != f.binding
                {
                    return Err(recovery_required());
                }
                if tx.execute("UPDATE context_retirement_effect_intents SET observation='changed' WHERE operation_ulid=?1 AND owner_epoch=?5 AND resource='branch' AND receipt_id=?2 AND primary_oid=?3 AND proof_kind=?4 AND observation IS NULL",params![k.child_operation_id,w.receipt,w.primary.to_string(),run::proof_name(w.proof),w.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            }
            let report =
                ContextRetirementReport::refused(crate::results::ResultCode::ExternalChange);
            let result = report.encode()?;
            if tx.execute("UPDATE context_retirement_records SET phase='refused',branch_removal='none',worktree_removal='none',final_code='external_change',final_result=?2 WHERE operation_ulid=?1 AND owner_epoch=1 AND phase IN ('selected','branch_deleting')",params![k.child_operation_id,result]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE context_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=1 AND phase='active'",[&k.child_operation_id]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE operation_records SET state='completed' WHERE id=?1 AND action='retire_context' AND state='created' AND completed_step IS NULL AND index_owner_epoch=0",[scope.local]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE poll_retirement_targets SET state='refused',handoff='final',final_result=?4 WHERE operation_ulid=?1 AND owner_epoch=?2 AND ordinal=?3 AND state='running' AND handoff='cleanup' AND final_result IS NULL",params![p.operation,p.epoch,scope.ordinal as i64,result]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            let last = scope.ordinal + 1 == count;
            if tx.execute("UPDATE poll_retirement_cursors SET next_ordinal=next_ordinal+1,stage=?4 WHERE operation_ulid=?1 AND owner_epoch=?2 AND next_ordinal=?3 AND stage='active'",params![p.operation,p.epoch,scope.ordinal as i64,if last{"complete"}else{"active"}]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            if last {
                if tx.execute("UPDATE poll_retirement_freezes SET final_code='ok' WHERE operation_ulid=?1 AND owner_epoch=?2 AND final_code IS NULL AND eligibility='eligible'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
                if tx.execute("UPDATE poll_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
                record::release_bound_source(tx, f.provenance, f.binding, true)?;
            }
            record::audit_connection(tx)?;
            Ok(ContextRetirementOutcome::Final(report))
        })
    }
}

impl IndexScope {
    fn current(&self, db: &Connection) -> Result<(Frozen, Candidate, usize), RepositoryError> {
        let p = &self.capture.p;
        let (f, entries, cursor) =
            Live::validate_metadata(&self.capture, db, p.repository, Path::new(&p.root))?;
        if cursor != self.ordinal {
            return Err(recovery_required());
        }
        let count = entries.len();
        let k = entries
            .into_iter()
            .nth(cursor)
            .ok_or_else(recovery_required)?;
        if !positive_row(db, p, &k)? {
            return Err(recovery_required());
        }
        let own: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch WHERE c.local_record_id=?1 AND c.operation_ulid=?2 AND c.phase IN ('selected','branch_deleting','index_pending') AND a.phase='active' AND o.state!='completed')",params![self.local,k.child_operation_id],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !own {
            return Err(recovery_required());
        }
        Ok((f, k, count))
    }

    pub(in super::super::super::super) fn validate_index(
        &self,
        db: &Connection,
        local: i64,
        epoch: i64,
    ) -> Result<(), RepositoryError> {
        if local != self.local {
            return Err(recovery_required());
        }
        let (_, k, _) = self.current(db)?;
        let p = &self.capture.p;
        let valid: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_index_claims i ON i.operation_ulid=c.operation_ulid WHERE c.operation_ulid=?1 AND c.phase='index_pending' AND c.absence_basis='owned_intent' AND c.materialization_settled=1 AND o.completed_step='context_branch_released' AND o.state='indexing' AND o.index_owner_epoch=?2 AND i.repository_id=c.repository_id AND i.request_ulid=?3 AND i.request_attempt=?4 AND i.index_owner_epoch=?2)",params![k.child_operation_id,epoch,p.request,p.request_attempt],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !valid {
            return Err(recovery_required());
        }
        Ok(())
    }

    fn receipt(&self, db: &Connection) -> Result<(i64, Candidate), RepositoryError> {
        // Fresh cleanup alone requires C/G/F and original process configuration.
        // Index and known-no-effect metadata settlement use current() instead.
        let p = &self.capture.p;
        Live::validate_capture(
            &self.capture,
            db,
            p.repository,
            Path::new(&p.root),
            Some(self.local),
        )?;
        let (f, k, _) = self.current(db)?;
        let p = &self.capture.p;
        let plan = crate::repository::RemoteRefPlan::from_configuration(&p.remote, &p.primary)
            .map_err(|_| recovery_required())?;
        let kind = if k.kind == "ticket" {
            crate::repository::AuthoringKind::Ticket
        } else {
            crate::repository::AuthoringKind::Document
        };
        record::require_fetch_absence(
            db,
            f.binding,
            p.repository,
            kind,
            &k.item_id.parse().map_err(|_| recovery_required())?,
            &plan,
        )?;
        Ok((f.binding, k))
    }

    pub(in super::super::super::super) fn finalize(
        &self,
        tx: &Transaction<'_>,
        owner: &IndexOwner,
        repository: i64,
        root: &Path,
    ) -> Result<bool, RepositoryError> {
        if owner.retirement_request.is_some() {
            return Err(recovery_required());
        }
        #[cfg(test)]
        boundary(Boundary::Finalizer, tx);
        self.validate_index(tx, owner.record_id, owner.epoch)?;
        let p = &self.capture.p;
        if repository != p.repository || root.to_str() != Some(p.root.as_str()) {
            return Err(recovery_required());
        }
        let (f, k, count) = self.current(tx)?;
        let report =
            ContextRetirementReport::retired(RetirementRemoval::None, RetirementRemoval::Observed);
        let result = report.encode()?;
        if tx.execute("UPDATE context_retirement_records SET phase='complete',final_code='ok',final_result=?2 WHERE operation_ulid=?1 AND phase='index_pending' AND owner_epoch=1 AND branch_removal='observed'",params![k.child_operation_id,result]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
        if tx.execute("UPDATE context_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=1 AND phase='active'",[&k.child_operation_id]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
        if tx.execute("UPDATE operation_records SET state='completed',context_path=NULL,observed_at=?3 WHERE id=?1 AND state='indexing' AND index_owner_epoch=?2",params![self.local,owner.epoch,time::OffsetDateTime::now_utc().unix_timestamp()]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
        if tx.execute("UPDATE poll_retirement_targets SET state='complete',handoff='final',final_result=?4 WHERE operation_ulid=?1 AND owner_epoch=?2 AND ordinal=?3 AND state='running' AND handoff='index_pending' AND final_result IS NULL",params![p.operation,p.epoch,self.ordinal as i64,result]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
        let last = self.ordinal + 1 == count;
        if tx.execute("UPDATE poll_retirement_cursors SET next_ordinal=next_ordinal+1,stage=?4 WHERE operation_ulid=?1 AND owner_epoch=?2 AND next_ordinal=?3 AND stage='active'",params![p.operation,p.epoch,self.ordinal as i64,if last{"complete"}else{"active"}]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
        if last {
            if tx.execute("UPDATE poll_retirement_freezes SET final_code='ok' WHERE operation_ulid=?1 AND owner_epoch=?2 AND final_code IS NULL AND eligibility='eligible'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
            if tx.execute("UPDATE poll_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
            record::release_bound_source(tx, f.provenance, f.binding, true)?;
        }
        record::audit_connection(tx)?;
        Ok(true)
    }

    fn refresh(
        &self,
        service: &RepositoryService,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        let p = &self.capture.p;
        if !Arc::ptr_eq(&self.capture.scope, &service.remote_reservation_scope)
            || service.registry_path != self.capture.registry
        {
            return Err(recovery_required());
        }
        let root = std::path::PathBuf::from(&p.root);
        let owner = state::with_transaction(service, &root, |tx, _| {
            #[cfg(test)]
            boundary(Boundary::BeforeClaim, tx);
            let (_, k, _) = self.current(tx)?;
            let valid: bool=tx.query_row("SELECT phase='index_pending' AND materialization_settled=1 FROM context_retirement_records WHERE operation_ulid=?1",[&k.child_operation_id],|r|r.get(0)).map_err(|_|recovery_required())?;
            if !valid {
                return Err(recovery_required());
            }
            let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_index_claims WHERE operation_ulid=?1 AND request_attempt>=?2)",params![k.child_operation_id,p.request_attempt],|r|r.get(0)).map_err(|_|recovery_required())?;
            if used {
                return Ok(None);
            }
            let old: i64 = tx
                .query_row(
                    "SELECT index_owner_epoch FROM operation_records WHERE id=?1",
                    [self.local],
                    |r| r.get(0),
                )
                .map_err(|_| recovery_required())?;
            let next = old
                .checked_add(1)
                .filter(|_| old >= 0)
                .ok_or_else(recovery_required)?;
            if tx.execute("UPDATE operation_records SET state='indexing',index_owner_epoch=?2,observed_at=?3 WHERE id=?1 AND index_owner_epoch=?4 AND state IN ('authoritative_observed','failed','observed','persisted','retry','index_invalidated')",params![self.local,next,service.clock.now().unix_timestamp(),old]).map_err(|_|recovery_required())?!=1 { return Err(recovery_required()); }
            tx.execute(
                "INSERT INTO context_retirement_index_claims VALUES(?1,?2,?3,?4,?5)",
                params![
                    k.child_operation_id,
                    p.repository,
                    p.request,
                    p.request_attempt,
                    next
                ],
            )
            .map_err(|_| recovery_required())?;
            Ok(Some(IndexOwner {
                record_id: self.local,
                epoch: next,
                retirement_request: None,
                retirement_batch: Some(self.clone()),
            }))
        })?;
        let Some(owner) = owner else {
            return Ok(ContextRetirementOutcome::IndexPending);
        };
        #[cfg(test)]
        CLAIM_HOOK.with(|slot| {
            if let Some(hook) = slot.borrow_mut().take() {
                hook(owner.clone());
            }
        });
        let git = git2::Repository::open(&root).map_err(|_| recovery_required())?;
        let heartbeat = crate::repository::IndexOwnerHeartbeat::start(
            service.registry_path.clone(),
            owner.clone(),
            root.clone(),
            service.take_index_owner_heartbeat_hook(),
        );
        service.run_refresh_claim_hook();
        let refreshed = service.refresh_repository_claimed(&git, root, p.repository, owner);
        drop(heartbeat);
        match refreshed {
            Ok(RefreshOutcome::Refreshed { .. }) => {
                let db = crate::repository::open_registry_read_only(&service.registry_path)?;
                record::audit_connection(&db)?;
                let result:String=db.query_row("SELECT final_result FROM context_retirement_records WHERE local_record_id=?1 AND phase='complete'",[self.local],|r|r.get(0)).map_err(|_|recovery_required())?;
                Ok(ContextRetirementOutcome::Final(
                    ContextRetirementReport::decode(&result)?,
                ))
            }
            _ => Ok(ContextRetirementOutcome::IndexPending),
        }
    }
}

impl Child<'_> {
    pub(super) fn execute(
        self,
        service: &RepositoryService,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        let c = &self.live.capture;
        let p = &c.p;
        let root = Path::new(&p.root);
        let git = git2::Repository::open(root).map_err(|_| recovery_required())?;
        let cleanup = ContextCleanup::acquire(root).map_err(|_| recovery_required())?;
        let (scope, request, permit) = state::with_transaction(service, root, |tx, id| {
            let (_, entries, cursor) = self.live.validate(service, tx, id, root)?;
            if cursor != self.ordinal {
                return Err(recovery_required());
            }
            let k = entries.get(cursor).ok_or_else(recovery_required)?;
            if k.initial_resources != "branch_only"
                || compare_candidate(tx, p, k)?
                    != super::super::super::instance::FrozenComparison::Unchanged
            {
                return Err(recovery_required());
            }
            let permit = self
                .live
                .branches
                .lock()
                .map_err(|_| recovery_required())?
                .get_mut(cursor)
                .ok_or_else(recovery_required)?
                .take()
                .ok_or_else(recovery_required)?;
            if permit.child != k.child_operation_id || permit.tip != k.expected_tip {
                return Err(recovery_required());
            }
            // No generic journal begin/collision exemption: the entire C scope
            // and exact current reserved Ki were validated in this IMMEDIATE tx.
            tx.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,?3,'retire_context',?4,'created',?5)",params![id,p.root,k.child_operation_id,format!("authoring-context-v1/{}/{}/Retire",k.kind,k.item_id),service.clock.now().unix_timestamp()]).map_err(|_|recovery_required())?;
            let local = tx.last_insert_rowid();
            tx.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,selection_epoch,instance_verified,remote_name,primary_branch,configuration_generation,endpoint_digest,request_ulid,batch_operation_ulid,batch_ordinal,materialization_capture,materialization_operation_ulid,materialization_oid,materialization_step,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) VALUES(?1,?2,?3,1,?4,?5,'branch_only',?6,1,0,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,0,1,'selected','none','none','none')",params![k.child_operation_id,local,id,k.kind,k.item_id,k.expected_tip,p.remote,p.primary,p.generation,p.endpoint,p.request,p.operation,cursor as i64,k.materialization_capture,k.materialization_operation_id,k.materialization_oid,k.materialization_step]).map_err(|_|recovery_required())?;
            tx.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,request_attempt,batch_attempt_epoch,phase) VALUES(?1,1,?2,?3,?4,'active')",params![k.child_operation_id,id,p.request_attempt,p.epoch]).map_err(|_|recovery_required())?;
            if tx.execute("UPDATE poll_retirement_targets SET state='running',handoff='cleanup' WHERE operation_ulid=?1 AND owner_epoch=?2 AND ordinal=?3 AND state='waiting' AND handoff='not_started' AND final_result IS NULL",params![p.operation,p.epoch,cursor as i64]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            record::audit_connection(tx)?;
            let request = RetireContextRequest {
                root: root.into(),
                kind: if k.kind == "ticket" {
                    crate::repository::AuthoringKind::Ticket
                } else {
                    crate::repository::AuthoringKind::Document
                },
                item_id: k.item_id.parse().map_err(|_| recovery_required())?,
                operation_id: OperationId::parse(&k.child_operation_id)
                    .map_err(|_| recovery_required())?,
            };
            Ok((
                IndexScope {
                    capture: Arc::clone(c),
                    ordinal: cursor,
                    local,
                },
                request,
                permit,
            ))
        })?;
        let tip = git2::Oid::from_str(&permit.tip).map_err(|_| recovery_required())?;
        let prepared = state::with_transaction(service, root, |tx, _| {
            #[cfg(test)]
            boundary(Boundary::Begin, tx);
            let (receipt, _) = scope.receipt(tx)?;
            if run::branch_only(&git, &request)? != Some(tip) {
                return Err(recovery_required());
            }
            let (primary, proof) = match run::compare_local_proof(&git, &request, tip, &p.primary)?
            {
                run::LocalProofComparison::Preserved(primary, proof) => (primary, proof),
                run::LocalProofComparison::Changed => return Ok(None),
            };
            run::require_materialization(tx, request.operation_id, false)?;
            if tx.execute("UPDATE context_retirement_records SET initial_primary_oid=?2,phase='branch_deleting',branch_removal='possible' WHERE operation_ulid=?1 AND owner_epoch=1 AND phase='selected'",params![request.operation_id.to_string(),primary.to_string()]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            tx.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) VALUES(?1,1,'branch',?2,?3,?4)",params![request.operation_id.to_string(),receipt,primary.to_string(),run::proof_name(proof)]).map_err(|_|recovery_required())?;
            record::audit_connection(tx)?;
            Ok(Some((primary, proof, receipt)))
        });
        let (primary, proof, receipt) = match prepared {
            Ok(Some(ready)) => ready,
            Ok(None) => {
                return self.live.finish_no_effect(
                    service,
                    RefusalWitness {
                        scope,
                        intent: None,
                    },
                );
            }
            Err(_) => {
                return Ok(ContextRetirementOutcome::CleanupPending {
                    worktree_removal: RetirementRemoval::None,
                    branch_removal: RetirementRemoval::None,
                });
            }
        };
        #[cfg(test)]
        {
            let db = Connection::open(&service.registry_path).map_err(|_| recovery_required())?;
            boundary(Boundary::IntentCommitted, &db);
        }
        let mut uninvoked = Some(UninvokedBranch {
            operation: request.operation_id,
            epoch: 1,
            receipt,
            primary,
            proof,
        });
        let mut changed = false;
        let mut invoked = false;
        let settled = state::with_transaction(service, root, |tx, _| {
            #[cfg(test)]
            boundary(Boundary::Intent, tx);
            scope.receipt(tx)?;
            run::require_materialization(tx, request.operation_id, false)?;
            if run::branch_only(&git, &request)? != Some(tip) {
                return Err(recovery_required());
            }
            match run::compare_local_proof(&git, &request, tip, &p.primary)? {
                run::LocalProofComparison::Changed => {
                    changed = true;
                    return Err(recovery_required());
                }
                run::LocalProofComparison::Preserved(now, kind)
                    if (now, kind) == (primary, proof) => {}
                _ => return Err(recovery_required()),
            }
            let removal=cleanup.delete_retirement_branch(request.kind,&request.item_id,tip,||{
                #[cfg(test)] boundary(Boundary::LockedBranch,tx);
                if scope.receipt(tx).is_err() || run::require_materialization(tx,request.operation_id,false).is_err() || crate::repository::context_retirement::instance::require_checkout_inventory(&git,&request,None).is_err() {return false;}
                match run::compare_local_proof(&git,&request,tip,&p.primary) {
                    Ok(run::LocalProofComparison::Changed)=>{changed=true;return false;},
                    Ok(run::LocalProofComparison::Preserved(now,kind)) if (now,kind)==(primary,proof)=>{},
                    _=>return false,
                }
                // Consumes only the original in-process permit, never a saved K.
                if permit.child!=request.operation_id.to_string(){return false;}
                if uninvoked.take().is_none(){return false;}
                invoked=true;
                true
            });
            if !matches!(removal, BranchDeletion::Deleted) {
                return Err(recovery_required());
            }
            #[cfg(test)]
            boundary(Boundary::Deleted, tx);
            scope.receipt(tx)?;
            if run::local_proof(&git, &request, tip, &p.primary)? != (primary, proof) {
                return Err(recovery_required());
            }
            run::require_absent(&git, &request)?;
            run::require_materialization(tx, request.operation_id, true)?;
            if tx.execute("UPDATE context_retirement_effect_intents SET observation='removed' WHERE operation_ulid=?1 AND owner_epoch=1 AND resource='branch' AND observation IS NULL",[request.operation_id.to_string()]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE context_retirement_records SET phase='index_pending',absence_basis='owned_intent',materialization_settled=1,branch_removal='observed' WHERE operation_ulid=?1 AND phase='branch_deleting' AND owner_epoch=1 AND materialization_settled=0",[request.operation_id.to_string()]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released' WHERE id=?1 AND state='created'",[scope.local]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE poll_retirement_targets SET handoff='index_pending' WHERE operation_ulid=?1 AND owner_epoch=?2 AND ordinal=?3 AND state='running' AND handoff='cleanup'",params![p.operation,p.epoch,self.ordinal as i64]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
            tx.execute(
                "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                [p.repository],
            )
            .map_err(|_| recovery_required())?;
            record::audit_connection(tx)
        });
        drop(cleanup);
        if settled.is_err() {
            if changed && let Some(witness) = uninvoked {
                return self.live.finish_no_effect(
                    service,
                    RefusalWitness {
                        scope,
                        intent: Some(witness),
                    },
                );
            }
            return Ok(ContextRetirementOutcome::CleanupPending {
                worktree_removal: RetirementRemoval::None,
                branch_removal: if invoked {
                    RetirementRemoval::Possible
                } else {
                    RetirementRemoval::None
                },
            });
        }
        // Cleanup/D14 already committed. An index-claim/inspection error cannot
        // erase that partial truth or turn it into a no-effect error/Final.
        Ok(scope
            .refresh(service)
            .unwrap_or(ContextRetirementOutcome::IndexPending))
    }
}

impl Live {
    fn finish_no_effect(
        &self,
        service: &RepositoryService,
        witness: RefusalWitness,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        match witness.settle(service) {
            Ok(result) => Ok(result),
            Err(_) => {
                let ordinal = witness.scope.ordinal;
                self.refusals
                    .lock()
                    .map_err(|_| recovery_required())?
                    .insert(ordinal, witness);
                Ok(ContextRetirementOutcome::CleanupPending {
                    worktree_removal: RetirementRemoval::None,
                    branch_removal: RetirementRemoval::None,
                })
            }
        }
    }

    /// Metadata-only retry retains the original known-change/uninvoked witness;
    /// it never recreates it from SQL or invokes Git/proof/cleanup again.
    #[allow(dead_code)]
    pub(in super::super::super::super) fn continue_no_effect(
        &self,
        service: &RepositoryService,
        root: &Path,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        if !Arc::ptr_eq(&self.capture.scope, &service.remote_reservation_scope)
            || self.capture.registry != service.registry_path
            || root.to_str() != Some(self.capture.p.root.as_str())
        {
            return Err(recovery_required());
        }
        let witness = self
            .refusals
            .lock()
            .map_err(|_| recovery_required())?
            .pop_first()
            .map(|(_, w)| w)
            .ok_or_else(recovery_required)?;
        self.finish_no_effect(service, witness)
    }
    /// Same original C, index-only. Does not spend another cleanup permit, prove
    /// resources, consume M, advertise, or start the next Ki. Same-F2 spends are
    /// deliberately not reclaimable, even after failed/invalidation history.
    #[allow(dead_code)]
    pub(in super::super::super::super) fn continue_index(
        &self,
        service: &RepositoryService,
        root: &Path,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        if !Arc::ptr_eq(&self.capture.scope, &service.remote_reservation_scope)
            || self.capture.registry != service.registry_path
            || root.to_str() != Some(self.capture.p.root.as_str())
        {
            return Err(recovery_required());
        }
        let scope = state::with_transaction(service, root, |tx, _| {
            let local:i64=tx.query_row("SELECT c.local_record_id FROM poll_retirement_cursors cursor JOIN context_retirement_records c ON c.batch_operation_ulid=cursor.operation_ulid AND c.batch_ordinal=cursor.next_ordinal WHERE cursor.operation_ulid=?1 AND cursor.owner_epoch=?2 AND c.phase='index_pending'",params![self.capture.p.operation,self.capture.p.epoch],|r|r.get(0)).map_err(|_|recovery_required())?;
            let ordinal: i64 = tx
                .query_row(
                    "SELECT batch_ordinal FROM context_retirement_records WHERE local_record_id=?1",
                    [local],
                    |r| r.get(0),
                )
                .map_err(|_| recovery_required())?;
            let scope = IndexScope {
                capture: Arc::clone(&self.capture),
                ordinal: usize::try_from(ordinal).map_err(|_| recovery_required())?,
                local,
            };
            scope.current(tx)?;
            Ok(scope)
        })?;
        scope.refresh(service)
    }
}
