//! Exact original-present history, never a cleanup or reconstruction capability.
use super::*;
pub(super) fn positive_row(
    db: &Connection,
    p: &Projection,
    k: &Candidate,
) -> Result<bool, RepositoryError> {
    // A spent original-present child must retain this batch's original F2 spend,
    // not merely some older claim allowed by standalone resumption history.
    // The independent invalidation tombstone refers to the prior index epoch;
    // neither counter is compared with today's current request attempt here.
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch WHERE
      c.operation_ulid=?1 AND c.repository_id=?2 AND o.repository_id=?2 AND o.root_path=?3 AND o.operation_ulid=?1 AND o.action='retire_context' AND o.target='authoring-context-v1/'||c.kind||'/'||c.item_id||'/Retire'
      AND c.kind=?4 AND c.item_id=?5 AND c.initial_resources='present' AND c.expected_tip=?6 AND c.selection_epoch=1 AND c.owner_epoch=1 AND c.instance_nonce=?7 AND c.instance_verified=1
      AND c.remote_name=?8 AND c.primary_branch=?9 AND c.configuration_generation=?10 AND c.endpoint_digest=?11 AND c.request_ulid=?12 AND c.batch_operation_ulid=?13 AND c.batch_ordinal=?14
      AND c.materialization_capture=?15 AND c.materialization_operation_ulid IS ?16 AND c.materialization_oid IS ?17 AND c.materialization_step IS ?18
      AND a.repository_id=?2 AND a.request_attempt=?19 AND a.batch_attempt_epoch=?20 AND a.observer_operation_ulid IS NULL AND a.observer_owner_epoch IS NULL
      AND (SELECT count(*) FROM context_retirement_attempts WHERE operation_ulid=c.operation_ulid)=1
      AND NOT EXISTS(SELECT 1 FROM context_retirement_reentries WHERE operation_ulid=c.operation_ulid)
      AND NOT EXISTS(SELECT 1 FROM context_retirement_worktree_resumptions WHERE operation_ulid=c.operation_ulid)
      AND NOT EXISTS(SELECT 1 FROM context_retirement_absence_receipts WHERE operation_ulid=c.operation_ulid)
      AND NOT EXISTS(SELECT 1 FROM request_operations WHERE operation_ulid=c.operation_ulid)
      AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND (e.owner_epoch!=1 OR NOT EXISTS(SELECT 1 FROM poll_retirement_freezes f WHERE f.binding_id=e.receipt_id AND f.operation_ulid=?13 AND f.owner_epoch=?20) OR e.primary_oid IS NOT c.initial_primary_oid))
      AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents b JOIN context_retirement_effect_intents w USING(operation_ulid,owner_epoch) WHERE b.operation_ulid=c.operation_ulid AND b.resource='branch' AND w.resource='worktree' AND (b.receipt_id!=w.receipt_id OR b.primary_oid!=w.primary_oid OR b.proof_kind!=w.proof_kind))
      AND ((c.phase='selected' AND a.phase='active' AND o.state='created' AND o.completed_step IS NULL AND o.index_owner_epoch=0 AND c.initial_primary_oid IS NULL AND c.worktree_removal='none' AND c.branch_removal='none' AND c.materialization_settled=0 AND c.absence_basis='none' AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid))
        OR (c.phase='worktree_removing' AND a.phase='active' AND o.state='created' AND o.completed_step IS NULL AND o.index_owner_epoch=0 AND c.worktree_removal='possible' AND c.branch_removal='none' AND c.materialization_settled=0 AND c.absence_basis='none' AND (SELECT count(*) FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid)=1 AND EXISTS(SELECT 1 FROM context_retirement_effect_intents w WHERE w.operation_ulid=c.operation_ulid AND w.resource='worktree' AND w.observation IS NULL))
        OR (c.phase IN ('worktree_removed','branch_deleting') AND a.phase='active' AND o.state='created' AND o.completed_step='context_worktree_released' AND o.index_owner_epoch=0 AND c.worktree_removal='observed' AND c.materialization_settled=0 AND c.absence_basis='none' AND EXISTS(SELECT 1 FROM context_retirement_effect_intents w WHERE w.operation_ulid=c.operation_ulid AND w.resource='worktree' AND w.observation='removed')
          AND ((c.phase='worktree_removed' AND c.branch_removal='none' AND ((SELECT count(*) FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid)=1 OR ((SELECT count(*) FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid)=2 AND EXISTS(SELECT 1 FROM context_retirement_effect_intents b WHERE b.operation_ulid=c.operation_ulid AND b.resource='branch' AND b.observation='changed'))))
            OR (c.phase='branch_deleting' AND c.branch_removal='possible' AND (SELECT count(*) FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid)=2 AND EXISTS(SELECT 1 FROM context_retirement_effect_intents b WHERE b.operation_ulid=c.operation_ulid AND b.resource='branch' AND b.observation IS NULL))))
        OR (c.phase IN ('index_pending','complete') AND o.completed_step='context_branch_released' AND c.worktree_removal='observed' AND c.branch_removal='observed' AND c.materialization_settled=1 AND c.absence_basis='owned_intent' AND (SELECT count(*) FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid AND observation='removed')=2
          AND (o.index_owner_epoch=0 OR EXISTS(SELECT 1 FROM context_retirement_index_claims i WHERE i.operation_ulid=c.operation_ulid AND i.repository_id=c.repository_id AND i.request_ulid=c.request_ulid AND i.request_attempt=a.request_attempt AND i.index_owner_epoch=CASE WHEN o.state='index_invalidated' THEN o.index_owner_epoch-1 ELSE o.index_owner_epoch END))
          AND ((c.phase='index_pending' AND a.phase='active' AND o.state IN ('authoritative_observed','indexing','failed','observed','persisted','retry','index_invalidated')) OR (c.phase='complete' AND a.phase='ended' AND o.state='completed' AND o.index_owner_epoch>0 AND c.final_code='ok')))
        OR (c.phase='refused' AND a.phase='ended' AND o.state='completed' AND o.completed_step IS NULL AND o.index_owner_epoch=0 AND o.persisted_context_count=0 AND o.context_path IS NULL AND c.worktree_removal='none' AND c.branch_removal='none' AND c.materialization_settled=0 AND c.absence_basis='none' AND c.final_code='external_change' AND (SELECT count(*) FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid)=1 AND EXISTS(SELECT 1 FROM context_retirement_effect_intents w WHERE w.operation_ulid=c.operation_ulid AND w.resource='worktree' AND w.observation='changed'))))",
      params![k.child_operation_id,p.repository,p.root,k.kind,k.item_id,k.expected_tip,k.instance_nonce,p.remote,p.primary,p.generation,p.endpoint,p.request,p.operation,k.ordinal as i64,k.materialization_capture,k.materialization_operation_id,k.materialization_oid,k.materialization_step,p.request_attempt,p.epoch],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !valid {
        return Ok(false);
    }
    let row: (String, Option<String>) = db
        .query_row(
            "SELECT phase,final_result FROM context_retirement_records WHERE operation_ulid=?1",
            [&k.child_operation_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| recovery_required())?;
    if let Some(result) = row.1 {
        let expected = if row.0 == "refused" {
            super::super::super::ContextRetirementReport::refused(
                crate::results::ResultCode::ExternalChange,
            )
        } else {
            super::super::super::ContextRetirementReport::retired(
                super::super::super::RetirementRemoval::Observed,
                super::super::super::RetirementRemoval::Observed,
            )
        };
        if super::super::super::ContextRetirementReport::decode(&result)? != expected {
            return Ok(false);
        }
    }
    Ok(true)
}
