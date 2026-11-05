//! Explicit epoch-checked recovery; ordinary duplicate calls remain non-owning.
use super::{
    ContextRetirementOutcome, ContextRetirementReport, RetireContextRequest, RetirementRemoval,
    execution::{Inventory, initial_inventory},
    instance, record, run,
};
use crate::{
    repository::{
        RemoteOperationPhase, RemoteOperationPriority, RemoteOperationTarget, RemoteRefPlan,
        RemoteReservation, RepositoryError, RepositoryErrorKind, RepositoryOperation,
        RepositoryService, authoring_kind_segment, canonical_repository_root,
        cleanup::ContextCleanup,
        keys::*,
        remote::{
            observation::{ObservationConfiguration, observation_plan},
            state,
        },
        repository_lease,
        transport::*,
    },
    results::ResultCode,
};
use rusqlite::{OptionalExtension, params};
use state::recovery_required;

#[derive(Clone, Debug)]
pub struct ReenterContextRetirementRequest {
    pub retirement: RetireContextRequest,
    pub expected_owner_epoch: i64,
}

enum Claim {
    Report(ContextRetirementOutcome),
    Index,
    Observe(Box<RemoteReservation>, git2::Oid),
}
struct FrozenSelection {
    initial: String,
    tip: Option<String>,
    generation: Option<i64>,
    remote_name: Option<String>,
    primary: Option<String>,
    digest: Option<Vec<u8>>,
    worktree: String,
    branch: String,
}

impl RepositoryService {
    /// Explicitly replace this domain operation's executor at the expected old
    /// epoch. IDs and stored markers alone never grant destructive authority.
    /// Request/batch-bound operations remain closed until their F2 fence joins.
    pub fn reenter_context_retirement_authenticated<P: SessionCredentialProvider>(
        &self,
        mut reentry: ReenterContextRetirementRequest,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        let request = &mut reentry.retirement;
        self.require_index_available(RepositoryOperation::RetireContext, Some(&request.root))?;
        let (git, root) =
            canonical_repository_root(&request.root, RepositoryOperation::RetireContext)?;
        if git.is_worktree() {
            return Err(recovery_required());
        }
        request.root = root;
        let lease = repository_lease(&git, &request.root, RepositoryOperation::RetireContext)?;
        let cleanup = ContextCleanup::holding(&request.root, lease)?;
        let claim = state::with_transaction(self, &request.root, |tx, id| {
            record::audit_connection(tx)?;
            let operation = request.operation_id.to_string();
            let target = format!(
                "authoring-context-v1/{}/{}/Retire",
                authoring_kind_segment(&request.kind),
                request.item_id
            );
            let row:Option<(String,String,String,Option<String>,i64)>=tx.query_row("SELECT o.root_path,o.target,c.phase,c.final_result,c.owner_epoch FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id WHERE c.operation_ulid=?1 AND c.repository_id=?2",params![operation,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(RepositoryError::sqlite)?;
            let (bound_root, bound_target, phase, result, old_epoch) =
                row.ok_or_else(recovery_required)?;
            if Some(bound_root.as_str()) != request.root.to_str() || bound_target != target {
                return Err(RepositoryError::new(
                    RepositoryOperation::RetireContext,
                    Some(request.root.clone()),
                    RepositoryErrorKind::OperationMismatch,
                    "retirement inputs do not match the original operation",
                ));
            }
            let domain:bool=tx.query_row("SELECT request_ulid IS NULL AND batch_operation_ulid IS NULL AND NOT EXISTS(SELECT 1 FROM request_operations ro WHERE ro.operation_ulid=c.operation_ulid) FROM context_retirement_records c WHERE operation_ulid=?1",[&operation],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
            if !domain {
                return Err(recovery_required());
            }
            // Terminal and authoritative I-only replay never inspect a new
            // lifetime or D14 tuple, even when the supplied old epoch is stale.
            if let Some(result) = result {
                return Ok(Claim::Report(ContextRetirementOutcome::Final(
                    ContextRetirementReport::decode(&result)?,
                )));
            }
            if matches!(phase.as_str(), "local_absent" | "index_pending") {
                if !record::refresh_allowed(tx, &operation, &request.root)? {
                    return Err(recovery_required());
                }
                return Ok(Claim::Index);
            }
            if old_epoch != reentry.expected_owner_epoch {
                return Err(recovery_required());
            }
            let epoch = old_epoch.checked_add(1).ok_or_else(recovery_required)?;
            let (old_observer,old_remote_epoch):(Option<String>,Option<i64>)=tx.query_row("SELECT observer_operation_ulid,observer_owner_epoch FROM context_retirement_attempts WHERE operation_ulid=?1 AND owner_epoch=?2 AND repository_id=?3 AND phase='active' AND request_attempt IS NULL AND batch_attempt_epoch IS NULL",params![operation,old_epoch,id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(RepositoryError::sqlite)?;
            let blocked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records o WHERE (o.repository_id=?1 OR o.root_path=?2) AND o.operation_ulid IS NOT ?3 AND (o.state!='completed' OR o.completed_step='comment_registration_pending')) OR EXISTS(SELECT 1 FROM remote_operation_records o WHERE o.repository_id=?1 AND ((o.operation_ulid IS NOT ?4 AND o.phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')) OR o.index_pending!=0 OR o.reconciliation_required!=0))",params![id,request.root.to_str(),operation,old_observer],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
            if blocked {
                return Err(recovery_required());
            }
            let FrozenSelection{initial,tip,generation,remote_name,primary,digest,worktree,branch}=tx.query_row("SELECT initial_resources,expected_tip,configuration_generation,remote_name,primary_branch,endpoint_digest,worktree_removal,branch_removal FROM context_retirement_records WHERE operation_ulid=?1",[&operation],|r|Ok(FrozenSelection{initial:r.get(0)?,tip:r.get(1)?,generation:r.get(2)?,remote_name:r.get(3)?,primary:r.get(4)?,digest:r.get(5)?,worktree:r.get(6)?,branch:r.get(7)?})).map_err(RepositoryError::sqlite)?;
            let plan = RemoteRefPlan::from_configuration(
                remote_name.as_deref().ok_or_else(recovery_required)?,
                primary.as_deref().ok_or_else(recovery_required)?,
            )
            .map_err(|_| recovery_required())?;
            let generation = generation.ok_or_else(recovery_required)?;
            let old_remote = if let Some(observer) = &old_observer {
                let observer = crate::repository::OperationId::parse(observer)
                    .map_err(|_| recovery_required())?;
                let remote =
                    state::read_operation(tx, id, observer)?.ok_or_else(recovery_required)?;
                if Some(remote.owner_epoch) != old_remote_epoch
                    || remote.generation != generation
                    || remote.target != RemoteOperationTarget::for_poll(&plan)
                    || remote.poll_kind != Some(state::PollKind::Observe)
                    || remote.priority != RemoteOperationPriority::Manual
                    || !matches!(
                        remote.phase,
                        RemoteOperationPhase::Reserved
                            | RemoteOperationPhase::Advertising
                            | RemoteOperationPhase::Persisting
                            | RemoteOperationPhase::Completed
                            | RemoteOperationPhase::Cancelled
                            | RemoteOperationPhase::Interrupted
                            | RemoteOperationPhase::Failed
                    )
                {
                    return Err(recovery_required());
                }
                Some(remote)
            } else {
                None
            };
            let possible = worktree != "none" || branch != "none";
            let intent:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_effect_intents WHERE operation_ulid=?1 AND (observation IS NULL OR observation IN ('removed','already_absent','incomplete')))",[&operation],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
            let absent = initial_inventory(&git, &request.root, request) == Inventory::Absent;
            let resumable:bool=tx.query_row("SELECT c.instance_verified=1 AND c.phase='worktree_removing' AND c.worktree_removal='possible' AND c.branch_removal='none' AND EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.resource='worktree' AND (e.observation IS NULL OR e.observation='incomplete')) AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND (e.resource='branch' OR e.observation='removed')) FROM context_retirement_records c WHERE c.operation_ulid=?1",[&operation],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
            let mut configuration = None;
            let purpose = if absent && possible && intent {
                "metadata"
            } else if (phase == "selected" || resumable)
                && initial == "present"
                && instance::Verified::matches_stored(tx, &git, request, &cleanup)?
                && state::generation(tx, id)? == generation
                && observation_plan(&request.root).ok().as_ref() == Some(&plan)
            {
                let current =
                    ObservationConfiguration::capture_owned(self, tx, &request.root, &plan)?;
                if digest.as_deref() == Some(current.endpoint_digest().as_slice()) {
                    configuration = Some(current);
                    if resumable {
                        "resume_worktree"
                    } else {
                        "observe"
                    }
                } else {
                    if possible { "preserve" } else { "no_effect" }
                }
            } else {
                if possible { "preserve" } else { "no_effect" }
            };
            // Every replacement write, including exact O fencing, old Z,
            // new local epoch/attempt/purpose and fresh O, commits or rolls back
            // together under this same common-Git lease and immediate TX.
            if let Some(remote)=old_remote
                && tx.execute("UPDATE remote_operation_records SET owner_epoch=owner_epoch+1,phase=CASE WHEN phase IN ('reserved','advertising','persisting') THEN 'interrupted' ELSE phase END,outcome=CASE WHEN phase IN ('reserved','advertising','persisting') THEN 'repository_unavailable' ELSE outcome END WHERE id=?1 AND owner_epoch=?2 AND poll_kind='observe' AND phase IN ('reserved','advertising','persisting','completed','failed','cancelled','interrupted')",params![remote.id,remote.owner_epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE context_retirement_attempts SET phase='fenced' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![operation,old_epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE context_retirement_records SET owner_epoch=?3 WHERE operation_ulid=?1 AND owner_epoch=?2 AND final_result IS NULL",params![operation,old_epoch,epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            tx.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) VALUES(?1,?2,?3,'active')",params![operation,epoch,id]).map_err(RepositoryError::sqlite)?;
            tx.execute("INSERT INTO context_retirement_reentries(operation_ulid,owner_epoch,previous_epoch,purpose) VALUES(?1,?2,?3,?4)",params![operation,epoch,old_epoch,purpose]).map_err(RepositoryError::sqlite)?;
            if purpose=="resume_worktree" && tx.execute("INSERT INTO context_retirement_worktree_resumptions(operation_ulid,owner_epoch,intent_owner_epoch,intent_resource,intent_receipt_id) SELECT operation_ulid,?2,owner_epoch,resource,receipt_id FROM context_retirement_effect_intents WHERE operation_ulid=?1 AND resource='worktree' AND owner_epoch<?2 AND (observation IS NULL OR observation='incomplete') ORDER BY owner_epoch DESC LIMIT 1",params![operation,epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            record::release_replaced_consumer(tx, &operation, old_epoch)?;
            let outcome = match purpose {
                "observe" | "resume_worktree" => Claim::Observe(
                    record::bind_reentry_observer(
                        self,
                        tx,
                        id,
                        request,
                        epoch,
                        configuration.ok_or_else(recovery_required)?,
                    )?,
                    git2::Oid::from_str(&tip.ok_or_else(recovery_required)?)
                        .map_err(|_| recovery_required())?,
                ),
                "metadata" => {
                    run::require_materialization(tx, request.operation_id, true)?;
                    if initial_inventory(&git, &request.root, request) != Inventory::Absent {
                        return Err(recovery_required());
                    }
                    tx.execute("UPDATE context_retirement_records SET phase='index_pending',absence_basis='owned_intent',materialization_settled=1 WHERE operation_ulid=?1",[&operation]).map_err(RepositoryError::sqlite)?;
                    if tx.execute("INSERT INTO context_retirement_absence_receipts(operation_ulid,owner_epoch,intent_owner_epoch,intent_resource,intent_receipt_id,inventory,worktree_removal,branch_removal) SELECT c.operation_ulid,c.owner_epoch,e.owner_epoch,e.resource,e.receipt_id,'all_absent',c.worktree_removal,c.branch_removal FROM context_retirement_records c JOIN context_retirement_effect_intents e ON e.operation_ulid=c.operation_ulid WHERE c.operation_ulid=?1 AND c.owner_epoch=?2 AND e.owner_epoch<?2 AND (e.observation IS NULL OR e.observation IN ('removed','already_absent','incomplete')) ORDER BY e.owner_epoch DESC,e.resource LIMIT 1",params![operation,epoch]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
                    tx.execute("UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released' WHERE operation_ulid=?1",[&operation]).map_err(RepositoryError::sqlite)?;
                    tx.execute(
                        "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                        [id],
                    )
                    .map_err(RepositoryError::sqlite)?;
                    Claim::Index
                }
                "no_effect" => {
                    let has_intent:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_effect_intents WHERE operation_ulid=?1)",[&operation],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
                    if has_intent {
                        return Err(recovery_required());
                    }
                    let report = ContextRetirementReport::refused(ResultCode::ExternalChange);
                    tx.execute("UPDATE context_retirement_records SET phase='refused',final_code=?2,final_result=?3 WHERE operation_ulid=?1",params![operation,report.code.as_str(),report.encode()?]).map_err(RepositoryError::sqlite)?;
                    tx.execute("UPDATE context_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2",params![operation,epoch]).map_err(RepositoryError::sqlite)?;
                    tx.execute(
                        "UPDATE operation_records SET state='completed' WHERE operation_ulid=?1",
                        [&operation],
                    )
                    .map_err(RepositoryError::sqlite)?;
                    Claim::Report(ContextRetirementOutcome::Final(report))
                }
                _ => Claim::Report(ContextRetirementOutcome::CleanupPending {
                    worktree_removal: removal(&worktree)?,
                    branch_removal: removal(&branch)?,
                }),
            };
            record::audit_connection(tx)?;
            Ok(outcome)
        })?;
        match claim {
            Claim::Report(report) => Ok(report),
            Claim::Index => {
                drop(cleanup);
                self.retire_context(reentry.retirement)
            }
            Claim::Observe(owner, tip) => {
                let verified = match instance::Verified::resume(self, request, &owner, &cleanup) {
                    Ok(verified) => verified,
                    Err(_) => {
                        return run::stop_execution(
                            self,
                            &request.root,
                            &owner,
                            ResultCode::ExternalChange,
                        );
                    }
                };
                drop(cleanup);
                self.continue_context_retirement(
                    reentry.retirement,
                    run::LiveRetirement {
                        git,
                        owner,
                        verified: Some(verified),
                        tip,
                    },
                    approval,
                    session,
                )
            }
        }
    }
}

fn removal(value: &str) -> Result<RetirementRemoval, RepositoryError> {
    match value {
        "none" => Ok(RetirementRemoval::None),
        "possible" => Ok(RetirementRemoval::Possible),
        "observed" => Ok(RetirementRemoval::Observed),
        _ => Err(recovery_required()),
    }
}
