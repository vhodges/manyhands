//! Same-call present execution. No enrollment, observer or resumable B permit.
use super::*;
type Pair = (
    git2::Oid,
    crate::repository::context_retirement::proof::PreservationProof,
);
pub(super) enum Selected {
    Branch(BranchLifetime),
    Present(instance::Verified),
}
pub(super) enum BranchPermit {
    Frozen(BranchLifetime),
    Pruned(instance::Pruned),
}
pub(super) enum Pruned {
    Removed(instance::Pruned, Pair),
    Stop(ContextRetirementOutcome),
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prune(
    live: &Live,
    service: &RepositoryService,
    scope: &IndexScope,
    request: &RetireContextRequest,
    git: &git2::Repository,
    cleanup: &ContextCleanup,
    tip: git2::Oid,
    verified: instance::Verified,
) -> Result<Pruned, RepositoryError> {
    let root = &request.root;
    let p = &scope.capture.p;
    let pending = |worktree| {
        Pruned::Stop(ContextRetirementOutcome::CleanupPending {
            worktree_removal: worktree,
            branch_removal: RetirementRemoval::None,
        })
    };
    let prepared = state::with_transaction(service, root, |tx, _| {
        #[cfg(test)]
        boundary(Boundary::Begin, tx);
        let (receipt, _) = scope.receipt(tx)?;
        verified.verify(git, request, cleanup)?;
        let (primary, proof) = match run::compare_local_proof(git, request, tip, &p.primary)? {
            run::LocalProofComparison::Preserved(primary, proof) => (primary, proof),
            run::LocalProofComparison::Changed => return Ok(None),
        };
        run::require_materialization(tx, request.operation_id, false)?;
        if tx.execute("UPDATE context_retirement_records SET initial_primary_oid=?2,phase='worktree_removing',worktree_removal='possible' WHERE operation_ulid=?1 AND owner_epoch=1 AND phase='selected' AND instance_verified=1",params![request.operation_id.to_string(),primary.to_string()]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        tx.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) VALUES(?1,1,'worktree',?2,?3,?4)",params![request.operation_id.to_string(),receipt,primary.to_string(),run::proof_name(proof)]).map_err(|_|recovery_required())?;
        record::audit_connection(tx)?;
        Ok(Some((primary, proof, receipt)))
    });
    let (primary, proof, receipt) = match prepared {
        Ok(Some(ready)) => ready,
        Ok(None) => {
            return Ok(Pruned::Stop(live.finish_no_effect(
                service,
                RefusalWitness {
                    scope: scope.clone(),
                    intent: None,
                    worktree: RetirementRemoval::None,
                },
            )?));
        }
        Err(_) => return Ok(pending(RetirementRemoval::None)),
    };
    #[cfg(test)]
    {
        let db = Connection::open(&service.registry_path).map_err(|_| recovery_required())?;
        boundary(Boundary::WorktreeIntentCommitted, &db);
    }
    let mut uninvoked = Some(UninvokedEffect {
        operation: request.operation_id,
        epoch: 1,
        receipt,
        primary,
        proof,
        resource: "worktree",
    });
    let mut changed = false;
    let mut invoked = false;
    let removed = state::with_transaction(service, root, |tx, _| {
        let (_, candidate) = scope.receipt(tx)?;
        run::require_materialization(tx, request.operation_id, false)?;
        match instance::compare_frozen(git, request, tip, candidate.instance_nonce.as_deref())? {
            instance::FrozenComparison::Changed => {
                changed = true;
                return Err(recovery_required());
            }
            instance::FrozenComparison::Unchanged => {}
        }
        verified.verify(git, request, cleanup)?;
        let outcome = verified.prune(git, request, cleanup, || {
            #[cfg(test)]
            boundary(Boundary::ImmediatePrune, tx);
            if scope.receipt(tx).is_err()
                || run::require_materialization(tx, request.operation_id, false).is_err()
            {
                return false;
            }
            // Successful mismatches are known changes. Missing/partial/foreign
            // or inaccessible identities stay unknown and never install/repair.
            match instance::compare_frozen(git, request, tip, candidate.instance_nonce.as_deref()) {
                Ok(instance::FrozenComparison::Changed) => {
                    changed = true;
                    return false;
                }
                Ok(instance::FrozenComparison::Unchanged) => {}
                Err(_) => return false,
            }
            match run::compare_local_proof(git, request, tip, &p.primary) {
                Ok(run::LocalProofComparison::Changed) => {
                    changed = true;
                    return false;
                }
                Ok(run::LocalProofComparison::Preserved(now, kind)) => {
                    if (now, kind) != (primary, proof) {
                        changed = true;
                        return false;
                    }
                }
                Err(_) => return false,
            }
            // Verified::prune then rechecks full nonce/identity/cleanliness at
            // this immediate boundary; no intent/SQL can stand in for success.
            uninvoked.take();
            invoked = true;
            true
        });
        let permit = match outcome {
            instance::Prune::Removed(permit) => permit,
            instance::Prune::NotInvoked => {
                invoked = false;
                return Err(recovery_required());
            }
            instance::Prune::Uncertain => return Err(recovery_required()),
        };
        #[cfg(test)]
        boundary(Boundary::Pruned, tx);
        scope.receipt(tx)?;
        if run::local_proof(git, request, tip, &p.primary)? != (primary, proof) {
            return Err(recovery_required());
        }
        run::require_materialization(tx, request.operation_id, false)?;
        if run::branch_only(git, request)? != Some(tip) {
            return Err(recovery_required());
        }
        if tx.execute("UPDATE context_retirement_effect_intents SET observation='removed' WHERE operation_ulid=?1 AND owner_epoch=1 AND resource='worktree' AND receipt_id=?2 AND primary_oid=?3 AND proof_kind=?4 AND observation IS NULL",params![request.operation_id.to_string(),receipt,primary.to_string(),run::proof_name(proof)]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        if tx.execute("UPDATE context_retirement_records SET phase='worktree_removed',worktree_removal='observed' WHERE operation_ulid=?1 AND owner_epoch=1 AND phase='worktree_removing'",[request.operation_id.to_string()]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        if tx.execute("UPDATE operation_records SET completed_step='context_worktree_released' WHERE id=?1 AND state='created' AND completed_step IS NULL",[scope.local]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        record::audit_connection(tx)?;
        Ok(permit)
    });
    match removed {
        Ok(permit) => {
            #[cfg(test)]
            {
                let db =
                    Connection::open(&service.registry_path).map_err(|_| recovery_required())?;
                boundary(Boundary::WorktreeCommitted, &db);
            }
            Ok(Pruned::Removed(permit, (primary, proof)))
        }
        Err(_) if changed && uninvoked.is_some() => Ok(Pruned::Stop(live.finish_no_effect(
            service,
            RefusalWitness {
                scope: scope.clone(),
                intent: uninvoked,
                worktree: RetirementRemoval::None,
            },
        )?)),
        Err(_) => Ok(pending(if invoked {
            RetirementRemoval::Possible
        } else {
            RetirementRemoval::None
        })),
    }
}
