//! Fresh standalone execution. Explicit new-executor recovery remains closed.
use super::{
    ContextRetirementOutcome, RetireContextRequest, RetirementRemoval, execution::*, instance,
    proof, record,
};
use crate::{
    canonical,
    repository::{
        RemoteOperationSafePoint as Point, RemoteOutcomeCategory, RemoteRefObservation,
        RemoteReservation, RemoteSafePointOutcome, RepositoryError, RepositoryOperation,
        RepositoryService, canonical_repository_root,
        cleanup::{BranchDeletion, ContextCleanup},
        item_context,
        keys::*,
        remote::{
            observation::{observation_plan, transport_category},
            reservation, state,
        },
        repository_lease,
        transport::*,
    },
    results::ResultCode,
};
use rusqlite::{Transaction, params};
use state::recovery_required;
use std::path::Path;

// The source-included SSH runner uses the same thread-local, one-shot fault
// boundary pattern as reservation tests. No runtime/public authority surface.
#[cfg(test)]
thread_local! {static INTENT_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = std::cell::RefCell::new(None);}
#[cfg(test)]
thread_local! {static WORKTREE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = std::cell::RefCell::new(None);}
#[cfg(test)]
thread_local! {static SELECTED_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = std::cell::RefCell::new(None);static OBSERVED_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = std::cell::RefCell::new(None);}

#[cfg(test)]
pub(super) fn selected_hook() {
    let hook = SELECTED_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

/// Only this uninterrupted call can prove the primitive was never invoked.
/// Created after the durable intent and consumed BEFORE calling Git. Neither a
/// saved intent nor a new executor can reconstruct this no-effect authority.
pub(super) struct UninvokedBranch {
    operation: crate::repository::OperationId,
    epoch: i64,
    receipt: i64,
    primary: git2::Oid,
    proof: proof::PreservationProof,
}
impl UninvokedBranch {
    pub(super) fn settle(
        self,
        tx: &Transaction<'_>,
        operation: crate::repository::OperationId,
        epoch: i64,
    ) -> Result<(), RepositoryError> {
        if self.operation != operation || self.epoch != epoch {
            return Err(recovery_required());
        }
        if tx.execute("UPDATE context_retirement_effect_intents SET observation='changed' WHERE operation_ulid=?1 AND owner_epoch=?2 AND resource='branch' AND receipt_id=?3 AND primary_oid=?4 AND proof_kind=?5 AND observation IS NULL",params![operation.to_string(),epoch,self.receipt,self.primary.to_string(),proof_name(self.proof)]).map_err(RepositoryError::sqlite)?!=1 {return Err(recovery_required());}
        Ok(())
    }
}
pub(super) struct UninvokedWorktree(UninvokedBranch);
pub(super) struct LiveRetirement {
    pub(super) git: git2::Repository,
    pub(super) owner: Box<RemoteReservation>,
    pub(super) verified: Option<instance::Verified>,
    pub(super) tip: git2::Oid,
}
impl UninvokedWorktree {
    pub(super) fn settle(
        self,
        tx: &Transaction<'_>,
        operation: crate::repository::OperationId,
        epoch: i64,
    ) -> Result<(), RepositoryError> {
        let witness = self.0;
        if witness.operation != operation || witness.epoch != epoch {
            return Err(recovery_required());
        }
        if tx.execute("UPDATE context_retirement_effect_intents SET observation='changed' WHERE operation_ulid=?1 AND owner_epoch=?2 AND resource='worktree' AND receipt_id=?3 AND primary_oid=?4 AND proof_kind=?5 AND observation IS NULL",params![operation.to_string(),epoch,witness.receipt,witness.primary.to_string(),proof_name(witness.proof)]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        Ok(())
    }
}

impl RepositoryService {
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn set_retirement_selected_hook_for_testing(hook: impl FnOnce() + 'static) {
        SELECTED_HOOK.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Box::new(hook));
        });
    }
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn set_retirement_observed_hook_for_testing(hook: impl FnOnce() + 'static) {
        OBSERVED_HOOK.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Box::new(hook));
        });
    }
    #[cfg(test)]
    #[allow(dead_code)] // Source-included SSH fault seam only.
    pub(crate) fn set_retirement_worktree_hook_for_testing(hook: impl FnOnce() + 'static) {
        WORKTREE_HOOK.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Box::new(hook));
        });
    }
    #[cfg(test)]
    #[allow(dead_code)] // Used by the source-included SSH target, not unit fixtures.
    pub(crate) fn set_retirement_intent_hook_for_testing(hook: impl FnOnce() + 'static) {
        INTENT_HOOK.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Box::new(hook));
        });
    }
    /// Explicit authenticated standalone retirement. This increment accepts a
    /// fresh lifetime, never a saved branch-only or worktree-removal permit.
    pub fn retire_context_authenticated<P: SessionCredentialProvider>(
        &self,
        request: RetireContextRequest,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        self.retire_context_authenticated_scoped(
            request,
            approval,
            session,
            super::request_scope::Scope::Domain,
        )
    }
    pub(super) fn retire_context_authenticated_scoped<P: SessionCredentialProvider>(
        &self,
        mut request: RetireContextRequest,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
        scope: super::request_scope::Scope,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        self.require_index_available(RepositoryOperation::RetireContext, Some(&request.root))?;
        let (git, root) =
            canonical_repository_root(&request.root, RepositoryOperation::RetireContext)?;
        if git.is_worktree() {
            return Err(recovery_required());
        }
        request.root = root;
        let (existing, cancelled) = state::with_transaction(self, &request.root, |tx, _| {
            record::audit_connection(tx)?;
            let cancelled = scope.validate(tx, &request.operation_id.to_string(), &request.root)?;
            let existing = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1)",
                    [request.operation_id.to_string()],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(RepositoryError::sqlite)?;
            Ok((existing, cancelled))
        })?;
        // Includes terminal and index-only replay, before configuration/key reads.
        if existing {
            return self.retire_context_scoped(request, scope);
        }
        if cancelled {
            return record::refuse_fresh_scoped(self, &request, ResultCode::Cancelled, scope)
                .map(ContextRetirementOutcome::Final);
        }
        let lease = repository_lease(&git, &request.root, RepositoryOperation::RetireContext)?;
        let selection_cleanup = ContextCleanup::holding(&request.root, lease)?;
        let (tip, selection) = match branch_only(&git, &request) {
            Ok(Some(tip)) => (tip, None),
            _ => {
                if initial_inventory(&git, &request.root, &request)
                    != Inventory::PresentOrBranchOnly
                {
                    drop(selection_cleanup);
                    return self.retire_context_scoped(request, scope);
                }
                match instance::Selection::capture(&git, &request, &selection_cleanup) {
                    Ok(selection) => (selection.tip(), Some(selection)),
                    Err(code) => {
                        return record::refuse_fresh_scoped(self, &request, code, scope)
                            .map(ContextRetirementOutcome::Final);
                    }
                }
            }
        };
        let plan = observation_plan(&request.root).map_err(|_| recovery_required())?;
        let configuration = self.observation_configuration(&request.root, &plan)?;
        let claimed = record::claim_scoped(
            self,
            &request,
            tip,
            plan,
            configuration,
            selection.as_ref().map(|s| s.nonce()),
            scope,
        )?;
        let owner = match claimed {
            record::BranchClaim::Refused(report) => {
                return Ok(ContextRetirementOutcome::Final(report));
            }
            record::BranchClaim::Execute(owner) => owner,
        };
        let verified = match selection {
            Some(selection) => {
                match selection.install(self, &request, &owner, &selection_cleanup) {
                    Ok(instance) => Some(instance),
                    Err(_) => {
                        return record::refuse_execution(
                            self,
                            &request.root,
                            &owner,
                            ResultCode::ExternalChange,
                        )
                        .map(ContextRetirementOutcome::Final);
                    }
                }
            }
            None => None,
        };
        drop(selection_cleanup);
        self.continue_context_retirement(
            request,
            LiveRetirement {
                git,
                owner,
                verified,
                tip,
            },
            approval,
            session,
        )
    }
    pub(super) fn continue_context_retirement<P: SessionCredentialProvider>(
        &self,
        request: RetireContextRequest,
        live: LiveRetirement,
        approval: Option<HostApproval>,
        session: &mut SessionCredentials<P>,
    ) -> Result<ContextRetirementOutcome, RepositoryError> {
        let LiveRetirement {
            git,
            owner,
            verified,
            tip,
        } = live;
        #[cfg(test)]
        selected_hook();
        // No common Git lease or database guard spans authentication/advertising.
        let observed = observe(self, &request, &owner, approval, session);
        if let Err(code) = observed {
            return stop_execution(self, &request.root, &owner, code);
        }
        #[cfg(test)]
        {
            let hook = OBSERVED_HOOK.with(|slot| slot.borrow_mut().take());
            if let Some(hook) = hook {
                hook();
            }
        }
        let cleanup = match ContextCleanup::acquire(&request.root) {
            Ok(cleanup) => cleanup,
            Err(_) => {
                // This call has invoked no primitive. Fresh no-effect attempts
                // end through TF/Z; a resumed attempt retains its old debt.
                // Neither route reconstructs a branch-lifetime permit.
                return stop_execution(self, &request.root, &owner, ResultCode::ExternalChange);
            }
        };
        let mut pruned = None;
        let mut prior_proof = None;
        if let Some(verified) = verified {
            match prune_present(self, &request, &git, &owner, &cleanup, tip, verified)? {
                PresentPrune::Removed(permit, primary, proof) => {
                    pruned = Some(permit);
                    prior_proof = Some((primary, proof));
                }
                PresentPrune::Stop(outcome) => return Ok(outcome),
            }
        }
        let worktree = if pruned.is_some() {
            RetirementRemoval::Observed
        } else {
            RetirementRemoval::None
        };
        let prepared = state::with_transaction(self, &request.root, |tx, id| {
            let execution = owner.retirement_execution()?;
            let receipt = execution.receipt(self, tx, id, request.kind, &request.item_id)?;
            if branch_only(&git, &request)? != Some(tip) {
                return Err(recovery_required());
            }
            let (primary, proof) =
                local_proof(&git, &request, tip, execution.plan().primary_branch())?;
            if prior_proof.is_some_and(|prior| prior != (primary, proof)) {
                return Err(recovery_required());
            }
            require_materialization(tx, execution.operation(), false)?;
            if pruned.is_none() && tx.execute("UPDATE context_retirement_records SET initial_primary_oid=?2 WHERE operation_ulid=?1 AND initial_primary_oid IS NULL AND phase='selected' AND owner_epoch=?3",params![request.operation_id.to_string(),primary.to_string(),execution.epoch()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            tx.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) SELECT operation_ulid,owner_epoch,'branch',?2,?3,?4 FROM context_retirement_records WHERE operation_ulid=?1 AND phase IN ('selected','worktree_removed')",params![request.operation_id.to_string(),receipt,primary.to_string(),proof_name(proof)]).map_err(RepositoryError::sqlite)?;
            if tx.execute("UPDATE context_retirement_records SET phase='branch_deleting',branch_removal='possible' WHERE operation_ulid=?1 AND phase IN ('selected','worktree_removed') AND owner_epoch=?2",params![request.operation_id.to_string(),execution.epoch()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            record::audit_connection(tx)?;
            Ok((primary, proof, receipt))
        });
        let (primary, proof, receipt) = match prepared {
            Ok(value) => value,
            Err(_) => {
                if pruned.is_some() {
                    return Ok(ContextRetirementOutcome::CleanupPending {
                        worktree_removal: worktree,
                        branch_removal: RetirementRemoval::None,
                    });
                }
                return record::refuse_execution(
                    self,
                    &request.root,
                    &owner,
                    ResultCode::ExternalChange,
                )
                .map(ContextRetirementOutcome::Final);
            }
        };
        // Intent committed before the primitive. Keep the same live capability
        // and lease; no persisted row/step can recreate this branch permit.
        let mut uninvoked = Some(UninvokedBranch {
            operation: request.operation_id,
            epoch: owner.retirement_execution()?.epoch(),
            receipt,
            primary,
            proof,
        });
        #[cfg(test)]
        INTENT_HOOK.with(|slot| {
            if let Some(hook) = slot.borrow_mut().take() {
                hook();
            }
        });
        let settled = state::with_transaction(self, &request.root, |tx, id| {
            owner
                .retirement_execution()?
                .receipt(self, tx, id, request.kind, &request.item_id)?;
            require_materialization(tx, request.operation_id, false)?;
            if branch_only(&git, &request)? != Some(tip)
                || local_proof(
                    &git,
                    &request,
                    tip,
                    owner.retirement_execution()?.plan().primary_branch(),
                )? != (primary, proof)
            {
                return Err(recovery_required());
            }
            let removal =
                cleanup.delete_retirement_branch(request.kind, &request.item_id, tip, || {
                    // Same complete authorization, now under the exact ref lock and
                    // immediately before Git removal. No selected-admin exemption
                    // or marker read can recreate the consumed W lifetime permit.
                    if owner
                        .retirement_execution()
                        .and_then(|e| e.validate_local(self, tx, id))
                        .is_err()
                        || instance::require_checkout_inventory(&git, &request, None).is_err()
                    {
                        return false;
                    }
                    // Only this actual effect boundary spends the no-effect witness.
                    // Earlier primitive refusals must retain truthful B None.
                    if uninvoked.take().is_none() {
                        return false;
                    }
                    let _ = pruned.take();
                    true
                });
            let (observation, flag) = match removal {
                BranchDeletion::Deleted => ("removed", "observed"),
                BranchDeletion::AlreadyAbsent => {
                    let _ = uninvoked.take();
                    let _ = pruned.take();
                    ("already_absent", "possible")
                }
                BranchDeletion::Changed => return Err(recovery_required()),
            };
            if tx.execute("UPDATE context_retirement_effect_intents SET observation=?2 WHERE operation_ulid=?1 AND resource='branch' AND observation IS NULL",params![request.operation_id.to_string(),observation]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            require_absent(&git, &request)?;
            require_materialization(tx, request.operation_id, true)?;
            if tx.execute("UPDATE context_retirement_records SET phase='index_pending',absence_basis='owned_intent',materialization_settled=1,branch_removal=?2 WHERE operation_ulid=?1 AND phase='branch_deleting' AND owner_epoch=?3 AND materialization_settled=0",params![request.operation_id.to_string(),flag,owner.retirement_execution()?.epoch()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            if tx.execute("UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released' WHERE operation_ulid=?1 AND action='retire_context' AND state IN ('created','context_worktree_released')",[request.operation_id.to_string()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
            tx.execute(
                "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                [id],
            )
            .map_err(RepositoryError::sqlite)?;
            record::audit_connection(tx)
        });
        drop(cleanup);
        if settled.is_err() {
            if let Some(uninvoked) = uninvoked {
                if worktree == RetirementRemoval::Observed {
                    let settled =
                        record::settle_branch_after_prune(self, &request.root, &owner, uninvoked);
                    return Ok(ContextRetirementOutcome::CleanupPending {
                        worktree_removal: worktree,
                        branch_removal: if settled.is_ok() {
                            RetirementRemoval::None
                        } else {
                            RetirementRemoval::Possible
                        },
                    });
                }
                return record::refuse_before_branch_effect(
                    self,
                    &request.root,
                    &owner,
                    ResultCode::ExternalChange,
                    uninvoked,
                )
                .map(ContextRetirementOutcome::Final);
            }
            return Ok(ContextRetirementOutcome::CleanupPending {
                worktree_removal: worktree,
                branch_removal: RetirementRemoval::Possible,
            });
        }
        // Existing I-only path owns its separate claimed index epoch/finalizer.
        self.retire_context_scoped(request, owner.retirement_execution()?.request_scope())
    }
}

enum PresentPrune {
    Removed(instance::Pruned, git2::Oid, proof::PreservationProof),
    Stop(ContextRetirementOutcome),
}
fn prune_present(
    service: &RepositoryService,
    request: &RetireContextRequest,
    git: &git2::Repository,
    owner: &RemoteReservation,
    cleanup: &ContextCleanup,
    tip: git2::Oid,
    verified: instance::Verified,
) -> Result<PresentPrune, RepositoryError> {
    let prepared = state::with_transaction(service, &request.root, |tx, id| {
        let execution = owner.retirement_execution()?;
        let receipt = execution.receipt(service, tx, id, request.kind, &request.item_id)?;
        verified.verify(git, request, cleanup)?;
        let (primary, proof) = local_proof(git, request, tip, execution.plan().primary_branch())?;
        require_materialization(tx, request.operation_id, false)?;
        tx.execute("UPDATE context_retirement_records SET initial_primary_oid=?2 WHERE operation_ulid=?1 AND owner_epoch=?3 AND initial_primary_oid IS NULL",params![request.operation_id.to_string(),primary.to_string(),execution.epoch()]).map_err(RepositoryError::sqlite)?;
        if tx.execute("UPDATE context_retirement_records SET phase='worktree_removing',worktree_removal='possible' WHERE operation_ulid=?1 AND owner_epoch=?2 AND instance_verified=1 AND (phase='selected' OR (phase='worktree_removing' AND EXISTS(SELECT 1 FROM context_retirement_worktree_resumptions w WHERE w.operation_ulid=context_retirement_records.operation_ulid AND w.owner_epoch=?2)))",params![request.operation_id.to_string(),execution.epoch()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        tx.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) SELECT operation_ulid,owner_epoch,'worktree',?2,?3,?4 FROM context_retirement_records WHERE operation_ulid=?1",params![request.operation_id.to_string(),receipt,primary.to_string(),proof_name(proof)]).map_err(RepositoryError::sqlite)?;
        record::audit_connection(tx)?;
        Ok((primary, proof, receipt))
    });
    let (primary, proof, receipt) = match prepared {
        Ok(v) => v,
        Err(_) => {
            return stop_execution(service, &request.root, owner, ResultCode::ExternalChange)
                .map(PresentPrune::Stop);
        }
    };
    let mut uninvoked = Some(UninvokedWorktree(UninvokedBranch {
        operation: request.operation_id,
        epoch: owner.retirement_execution()?.epoch(),
        receipt,
        primary,
        proof,
    }));
    #[cfg(test)]
    WORKTREE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
    let mut permit = None;
    let settled = state::with_transaction(service, &request.root, |tx, id| {
        owner
            .retirement_execution()?
            .receipt(service, tx, id, request.kind, &request.item_id)?;
        verified.verify(git, request, cleanup)?;
        if local_proof(
            git,
            request,
            tip,
            owner.retirement_execution()?.plan().primary_branch(),
        )? != (primary, proof)
        {
            return Err(recovery_required());
        }
        require_materialization(tx, request.operation_id, false)?;
        match verified.prune(git, request, cleanup, || {
            owner
                .retirement_execution()
                .and_then(|e| e.validate_local(service, tx, id))
                .is_ok()
        }) {
            instance::Prune::Removed(pruned) => {
                let _ = uninvoked.take();
                permit = Some(pruned);
            }
            instance::Prune::NotInvoked => return Err(recovery_required()),
            instance::Prune::Uncertain => {
                let _ = uninvoked.take();
                return Err(recovery_required());
            }
        }
        if tx.execute("UPDATE context_retirement_effect_intents SET observation='removed' WHERE operation_ulid=?1 AND owner_epoch=?2 AND resource='worktree' AND observation IS NULL",params![request.operation_id.to_string(),owner.retirement_execution()?.epoch()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        if tx.execute("UPDATE context_retirement_records SET phase='worktree_removed',worktree_removal='observed' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='worktree_removing'",params![request.operation_id.to_string(),owner.retirement_execution()?.epoch()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        if tx.execute("UPDATE operation_records SET state='context_worktree_released',completed_step='context_worktree_released' WHERE operation_ulid=?1 AND state='created'",[request.operation_id.to_string()]).map_err(RepositoryError::sqlite)?!=1{return Err(recovery_required());}
        record::audit_connection(tx)
    });
    if settled.is_err() {
        if uninvoked.is_some() {
            if let Some(outcome) =
                record::preserve_worktree_resumption(service, &request.root, owner, &mut uninvoked)?
            {
                return Ok(PresentPrune::Stop(outcome));
            }
            return record::refuse_before_worktree_effect(
                service,
                &request.root,
                owner,
                uninvoked.ok_or_else(recovery_required)?,
            )
            .map(|report| PresentPrune::Stop(ContextRetirementOutcome::Final(report)));
        }
        return Ok(PresentPrune::Stop(
            ContextRetirementOutcome::CleanupPending {
                worktree_removal: if permit.is_some() {
                    RetirementRemoval::Observed
                } else {
                    RetirementRemoval::Possible
                },
                branch_removal: RetirementRemoval::None,
            },
        ));
    }
    Ok(PresentPrune::Removed(
        permit.ok_or_else(recovery_required)?,
        primary,
        proof,
    ))
}

pub(super) fn stop_execution(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    code: ResultCode,
) -> Result<ContextRetirementOutcome, RepositoryError> {
    if let Some(outcome) = record::preserve_worktree_resumption(service, root, owner, &mut None)? {
        return Ok(outcome);
    }
    record::refuse_execution(service, root, owner, code).map(ContextRetirementOutcome::Final)
}

fn safe(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    point: Point,
) -> Result<(), ResultCode> {
    match service.remote_safe_point(root, owner, point) {
        Ok(RemoteSafePointOutcome::Continue) => Ok(()),
        Ok(RemoteSafePointOutcome::Cancelled) => Err(ResultCode::Cancelled),
        _ => Err(ResultCode::ExternalChange),
    }
}

#[allow(clippy::result_large_err)] // Existing typed transport diagnostic contract.
fn observe<P: SessionCredentialProvider>(
    service: &RepositoryService,
    request: &RetireContextRequest,
    owner: &RemoteReservation,
    approval: Option<HostApproval>,
    session: &mut SessionCredentials<P>,
) -> Result<(), ResultCode> {
    safe(service, &request.root, owner, Point::BeforeTransport)?;
    let execution = owner
        .retirement_execution()
        .map_err(|_| ResultCode::RecoveryRequired)?;
    let expectation = execution
        .expectation()
        .map_err(|_| ResultCode::InvalidRemote)?;
    let advertised = service.with_authenticated_remote_expected(
        VerifySshTransportRequest {
            root: request.root.clone(),
            direction: SshDirection::Fetch,
            approval,
        },
        session,
        &expectation,
        |remote| remote.advertisement(),
    );
    safe(service, &request.root, owner, Point::AfterAdvertisement)?;
    let advertised = match advertised {
        Ok(value) => value,
        Err(error) => {
            service
                .finish_remote_operation(&request.root, owner, transport_category(&error.kind))
                .map_err(|_| ResultCode::RecoveryRequired)?;
            return Err(transport_code(&error.kind));
        }
    };
    let git = git2::Repository::open(&request.root).map_err(|_| ResultCode::RecoveryRequired)?;
    let mut observations = Vec::new();
    for (name, oid) in advertised {
        safe(service, &request.root, owner, Point::BetweenObservations)?;
        let tracking = match execution.plan().tracking_ref_for(&name) {
            Some(name) => match git.refname_to_id(&name) {
                Ok(oid) => Some(oid),
                Err(e) if e.code() == git2::ErrorCode::NotFound => None,
                Err(_) => return Err(ResultCode::RecoveryRequired),
            },
            None => None,
        };
        if let Some(observation) =
            RemoteRefObservation::from_advertisement(execution.plan(), &name, oid, tracking)
        {
            observations.push(observation);
        }
    }
    safe(service, &request.root, owner, Point::BeforeBatchCommit)?;
    let decision = reservation::commit_observation_batch(
        service,
        &request.root,
        owner,
        execution.plan(),
        &observations,
        time::OffsetDateTime::now_utc().unix_timestamp().max(0),
    )
    .map_err(|_| ResultCode::ExternalChange)?;
    if decision != RemoteSafePointOutcome::Continue {
        return Err(ResultCode::Cancelled);
    }
    safe(service, &request.root, owner, Point::AfterBatchCommit)?;
    let decision = service
        .finish_remote_operation(&request.root, owner, RemoteOutcomeCategory::Completed)
        .map_err(|_| ResultCode::ExternalChange)?;
    if decision != RemoteSafePointOutcome::Continue {
        return Err(ResultCode::Cancelled);
    }
    Ok(())
}

pub(super) fn branch_only(
    git: &git2::Repository,
    request: &RetireContextRequest,
) -> Result<Option<git2::Oid>, RepositoryError> {
    let context = item_context(&request.root, request.kind, request.item_id.clone());
    // Validate storage and all checkout registrations, not just deterministic A/W.
    plain_registration_path(&git.path().join("HEAD"), false).map_err(|_| recovery_required())?;
    ref_file_absent(&request.root, &git.commondir().join("packed-refs"))?;
    ref_file_absent(
        &request.root,
        &git.commondir()
            .join(format!("refs/heads/{}", context.branch)),
    )?;
    let tip = match git.find_reference(&format!("refs/heads/{}", context.branch)) {
        Ok(reference) if reference.symbolic_target().is_none() => {
            reference.target().filter(|oid| !oid.is_zero())
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
        _ => return Ok(None),
    };
    for path in [
        context.worktree,
        git.commondir()
            .join("worktrees")
            .join(request.item_id.to_string()),
        git.commondir()
            .join(format!("refs/heads/{}.lock", context.branch)),
        git.commondir().join("packed-refs.lock"),
    ] {
        if !absent_path(&request.root, &path)? {
            return Ok(None);
        }
    }
    instance::require_checkout_inventory(git, request, None)?;
    Ok(tip)
}

pub(super) fn require_absent(
    git: &git2::Repository,
    request: &RetireContextRequest,
) -> Result<(), RepositoryError> {
    if initial_inventory(git, &request.root, request) == Inventory::Absent {
        Ok(())
    } else {
        Err(recovery_required())
    }
}

pub(super) fn local_proof(
    git: &git2::Repository,
    request: &RetireContextRequest,
    tip: git2::Oid,
    primary: &str,
) -> Result<(git2::Oid, proof::PreservationProof), RepositoryError> {
    // libgit2 follows loose-ref symlinks (including ancestors). Trust the
    // configured primary's actual storage, not just HEAD's resolved name/OID or
    // matching item bytes. Repeat on each proof, including immediately before
    // the effect; an absent loose slot may legitimately resolve from packed-refs.
    plain_registration_path(&git.path().join("HEAD"), false).map_err(|_| recovery_required())?;
    let primary_ref = format!("refs/heads/{primary}");
    let primary_slot = git.commondir().join(&primary_ref);
    ref_file_absent(&request.root, &primary_slot)?;
    ref_file_absent(&request.root, &git.commondir().join("packed-refs"))?;
    for lock in [
        git.commondir().join(format!("{primary_ref}.lock")),
        git.commondir().join("packed-refs.lock"),
        git.path().join("HEAD.lock"),
    ] {
        if !absent_path(&request.root, &lock)? {
            return Err(recovery_required());
        }
    }
    let head = git.head().map_err(|_| recovery_required())?;
    if head.name() != Some(format!("refs/heads/{primary}").as_str())
        || git.workdir().and_then(|p| p.canonicalize().ok()).as_ref() != Some(&request.root)
    {
        return Err(recovery_required());
    }
    let primary = head.target().ok_or_else(recovery_required)?;
    let proof = proof::preservation(git, tip, primary, request.kind, &request.item_id)
        .map_err(|_| recovery_required())?;
    // Preservation is committed-object authority; independently require the
    // primary item's current disk bytes and executable mode to match that blob.
    let tree = git
        .find_commit(primary)
        .and_then(|commit| commit.tree())
        .map_err(|_| recovery_required())?;
    let mut item = None;
    tree.walk(git2::TreeWalkMode::PreOrder, |prefix, entry| {
        let Some(name) = entry.name() else {
            return git2::TreeWalkResult::Ok;
        };
        let path = format!("{prefix}{name}");
        if entry.kind() != Some(git2::ObjectType::Blob)
            || !path.ends_with(".md")
            || !(path.starts_with("docs/")
                || path.starts_with(".manyhands/tickets/")
                || path.starts_with(".manyhands/comments/"))
        {
            return git2::TreeWalkResult::Ok;
        }
        if let Ok(blob) = git.find_blob(entry.id())
            && std::str::from_utf8(blob.content())
                .ok()
                .and_then(canonical::front_matter)
                .and_then(|fm| fm.get("id").and_then(|v| v.as_str()).map(str::to_owned))
                .as_deref()
                == Some(request.item_id.to_string().as_str())
        {
            item = Some((path, entry.id(), entry.filemode_raw()));
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|_| recovery_required())?;
    let (path, oid, mode) = item.ok_or_else(recovery_required)?;
    let path = request.root.join(path);
    plain_registration_path(&path, false).map_err(|_| recovery_required())?;
    if std::fs::read(&path).map_err(|_| recovery_required())?
        != git
            .find_blob(oid)
            .map_err(|_| recovery_required())?
            .content()
    {
        return Err(recovery_required());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = std::fs::symlink_metadata(&path)
            .map_err(|_| recovery_required())?
            .permissions()
            .mode()
            & 0o111
            != 0;
        if executable != (mode == 0o100755) {
            return Err(recovery_required());
        }
    }
    if !matches!(mode, 0o100644 | 0o100755) {
        return Err(recovery_required());
    }
    Ok((primary, proof))
}
pub(super) fn proof_name(proof: proof::PreservationProof) -> &'static str {
    match proof {
        proof::PreservationProof::Ancestor => "ancestor",
        proof::PreservationProof::Folded => "folded",
    }
}

pub(super) fn require_materialization(
    tx: &Transaction<'_>,
    operation: crate::repository::OperationId,
    consume: bool,
) -> Result<(), RepositoryError> {
    let id = operation.to_string();
    let valid:bool=tx.query_row("SELECT (materialization_capture='absent' AND NOT EXISTS(SELECT 1 FROM remote_poll_materializations m WHERE m.repository_id=c.repository_id AND m.item_id=c.item_id)) OR (materialization_capture='complete' AND (SELECT count(*) FROM remote_poll_materializations m WHERE m.repository_id=c.repository_id AND m.item_id=c.item_id)=1 AND EXISTS(SELECT 1 FROM remote_poll_materializations m WHERE m.repository_id=c.repository_id AND m.kind=c.kind AND m.item_id=c.item_id AND m.operation_ulid=c.materialization_operation_ulid AND m.target_oid=c.materialization_oid AND m.step=c.materialization_step AND m.step='complete')) FROM context_retirement_records c WHERE operation_ulid=?1 AND materialization_settled=0",[&id],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
    if !valid {
        return Err(recovery_required());
    }
    if consume {
        let expected:i64=tx.query_row("SELECT materialization_capture='complete' FROM context_retirement_records WHERE operation_ulid=?1",[&id],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        let deleted=tx.execute("DELETE FROM remote_poll_materializations WHERE EXISTS(SELECT 1 FROM context_retirement_records c WHERE c.operation_ulid=?1 AND c.materialization_capture='complete' AND c.repository_id=remote_poll_materializations.repository_id AND c.kind=remote_poll_materializations.kind AND c.item_id=remote_poll_materializations.item_id AND c.materialization_operation_ulid=remote_poll_materializations.operation_ulid AND c.materialization_oid=remote_poll_materializations.target_oid AND c.materialization_step=remote_poll_materializations.step)",[&id]).map_err(RepositoryError::sqlite)?;
        let remaining:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_poll_materializations m JOIN context_retirement_records c ON c.repository_id=m.repository_id AND c.item_id=m.item_id WHERE c.operation_ulid=?1)",[&id],|r|r.get(0)).map_err(RepositoryError::sqlite)?;
        if deleted as i64 != expected || remaining {
            return Err(recovery_required());
        }
    }
    Ok(())
}

fn transport_code(kind: &SshTransportErrorKind) -> ResultCode {
    use SshTransportErrorKind::*;
    match kind {
        ConfigurationInvalid | UsernameRequired => ResultCode::InvalidRemote,
        PublicationRemoteMissing => ResultCode::PublicationRemoteRequired,
        NoSelectedKey => ResultCode::NoSelectedKey,
        KeyMissing | KeyUnreadable | KeyInvalidOrUnsupported => ResultCode::KeyUnavailable,
        KeySourceChanged | SelectionChanged | EndpointChanged | HostTrustChanged => {
            ResultCode::ExternalChange
        }
        KeyRejected => ResultCode::KeyRejected,
        UnlockCancelled | ProviderUnavailable => ResultCode::UnlockRequired,
        UnlockFailed => ResultCode::UnlockFailed,
        HostApprovalRequired { .. } => ResultCode::HostApprovalRequired,
        HostReplacementRequired { .. } => ResultCode::HostReplacementRequired,
        RegistryUnavailable => ResultCode::IndexUnavailable,
        HostVerificationUnavailable => ResultCode::RecoveryRequired,
        TransportUnavailable => ResultCode::TransportUnavailable,
        RemoteUnavailable => ResultCode::RemoteUnavailable,
        PushRejected => ResultCode::PushRejected,
        RuntimeUninitialized | ProtocolFailure => ResultCode::InternalError,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::repository::{AuthoringKind, EnableRepositoryRequest, OperationId};

    fn fixture() -> (tempfile::TempDir, git2::Repository, RetireContextRequest) {
        let root = tempfile::tempdir().unwrap();
        let git = git2::Repository::init(root.path()).unwrap();
        git.set_head("refs/heads/main").unwrap();
        let tree = git
            .find_tree(git.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        git.commit(Some("HEAD"), &sig, &sig, "fixture", &tree, &[])
            .unwrap();
        let request = RetireContextRequest {
            root: root.path().canonicalize().unwrap(),
            kind: AuthoringKind::Ticket,
            item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
            operation_id: OperationId::new(),
        };
        drop(tree);
        (root, git, request)
    }

    #[test]
    fn retirement_live_inventory_owned_absence_rejects_redirected_packed_store() {
        let (_root, git, request) = fixture();
        std::os::unix::fs::symlink("missing-packed-store", git.commondir().join("packed-refs"))
            .unwrap();
        assert!(
            require_absent(&git, &request).is_err(),
            "M must use the same complete no-follow ref inventory as initial absence"
        );
        assert_eq!(
            std::fs::read_link(git.commondir().join("packed-refs")).unwrap(),
            Path::new("missing-packed-store")
        );
    }

    #[test]
    fn retirement_live_inventory_branch_only_rejects_redirected_primary_head() {
        let (_root, git, request) = fixture();
        let tip = git.head().unwrap().target().unwrap();
        git.reference(
            &format!("refs/heads/manyhands/ticket/{}", request.item_id),
            tip,
            false,
            "fixture branch",
        )
        .unwrap();
        std::fs::rename(git.path().join("HEAD"), git.path().join("fixture-HEAD")).unwrap();
        std::os::unix::fs::symlink("fixture-HEAD", git.path().join("HEAD")).unwrap();
        assert!(
            !matches!(branch_only(&git, &request), Ok(Some(_))),
            "a redirected primary HEAD is not branch-only deletion authority"
        );
        assert_eq!(
            std::fs::read_link(git.path().join("HEAD")).unwrap(),
            Path::new("fixture-HEAD")
        );
        assert_eq!(
            git.refname_to_id(&format!("refs/heads/manyhands/ticket/{}", request.item_id))
                .unwrap(),
            tip
        );
    }

    struct Provider;
    impl SessionCredentialProvider for Provider {
        fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
            PassphraseResponse::Unavailable
        }
    }

    #[test]
    fn retirement_live_inventory_invalid_fresh_authenticated_request_replays_refusal() {
        let (_root, git, request) = fixture();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: request.root.clone(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let slot = git
            .commondir()
            .join(format!("refs/heads/manyhands/ticket/{}", request.item_id));
        std::fs::create_dir_all(slot.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink("missing-ref", &slot).unwrap();
        let first = service.retire_context_authenticated(
            request.clone(),
            None,
            &mut SessionCredentials::new(Provider),
        );
        assert!(
            matches!(&first,Ok(ContextRetirementOutcome::Final(report)) if report.outcome==super::super::RetirementDisposition::Refused && !report.materialization_settled)
        );
        assert_eq!(
            service
                .retire_context_authenticated(request, None, &mut SessionCredentials::new(Provider))
                .unwrap(),
            first.unwrap()
        );
        assert_eq!(std::fs::read_link(slot).unwrap(), Path::new("missing-ref"));
    }
}
