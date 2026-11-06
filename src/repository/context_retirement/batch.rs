//! Original same-call C: ordered refusal or original branch-only cleanup/index.
//! Present worktrees, gap observers and resumed cleanup remain closed.
use super::*;
use crate::repository::{RepositoryOperation, repository_lease};

const CONTRACT: &str = "poll_retirement_consumption_contract_v1";

#[path = "batch_execution.rs"]
mod execution;
pub(in super::super::super) use execution::IndexScope;
#[cfg(test)]
pub(in super::super::super) use execution::{Boundary, observe, observe_claim};

pub(super) struct BranchLifetime {
    child: String,
    tip: String,
}
impl BranchLifetime {
    // Only handoff::freeze calls this, after full original selection validation.
    pub(super) fn from_freeze(k: &Candidate) -> Self {
        Self {
            child: k.child_operation_id.clone(),
            tip: k.expected_tip.clone(),
        }
    }
}

/// Closed positive history, distinct from the unchanged strict refusal history.
/// No current config/exclusion gate here: history cannot create an executor.
fn positive_row(db: &Connection, p: &Projection, k: &Candidate) -> Result<bool, RepositoryError> {
    if k.initial_resources != "branch_only" {
        return Ok(false);
    }
    let valid: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch WHERE
      c.operation_ulid=?1 AND c.repository_id=?2 AND o.repository_id=?2 AND o.root_path=?3 AND o.operation_ulid=?1 AND o.action='retire_context' AND o.target='authoring-context-v1/'||c.kind||'/'||c.item_id||'/Retire'
      AND c.kind=?4 AND c.item_id=?5 AND c.initial_resources='branch_only' AND c.expected_tip=?6 AND c.selection_epoch=1 AND c.owner_epoch=1 AND c.instance_nonce IS NULL AND c.instance_verified=0
      AND c.remote_name=?7 AND c.primary_branch=?8 AND c.configuration_generation=?9 AND c.endpoint_digest=?10 AND c.request_ulid=?11 AND c.batch_operation_ulid=?12 AND c.batch_ordinal=?13
      AND c.materialization_capture=?14 AND c.materialization_operation_ulid IS ?15 AND c.materialization_oid IS ?16 AND c.materialization_step IS ?17 AND c.worktree_removal='none'
      AND a.repository_id=?2 AND a.request_attempt=?18 AND a.batch_attempt_epoch=?19 AND a.observer_operation_ulid IS NULL AND a.observer_owner_epoch IS NULL
      AND (SELECT count(*) FROM context_retirement_attempts WHERE operation_ulid=c.operation_ulid)=1
      AND NOT EXISTS(SELECT 1 FROM context_retirement_reentries WHERE operation_ulid=c.operation_ulid)
      AND NOT EXISTS(SELECT 1 FROM request_operations WHERE operation_ulid=c.operation_ulid)
      AND ((c.phase='selected' AND a.phase='active' AND o.state='created' AND o.index_owner_epoch=0 AND c.initial_primary_oid IS NULL AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid))
        OR (c.phase='branch_deleting' AND a.phase='active' AND o.state='created' AND o.index_owner_epoch=0 AND c.initial_primary_oid IS NOT NULL AND EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.observation IS NULL))
        OR (c.phase='index_pending' AND a.phase='active' AND o.state IN ('authoritative_observed','indexing','failed','observed','persisted','retry','index_invalidated') AND c.materialization_settled=1 AND c.absence_basis='owned_intent' AND c.branch_removal='observed' AND EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.observation='removed'))
        OR (c.phase='complete' AND a.phase='ended' AND o.state='completed' AND o.index_owner_epoch>0 AND c.materialization_settled=1 AND c.absence_basis='owned_intent' AND c.final_code='ok' AND c.branch_removal='observed' AND EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.observation='removed'))
        OR (c.phase='refused' AND a.phase='ended' AND o.state='completed' AND o.completed_step IS NULL AND o.index_owner_epoch=0 AND o.persisted_context_count=0 AND o.context_path IS NULL AND c.materialization_settled=0 AND c.absence_basis='none' AND c.final_code='external_change' AND c.branch_removal='none' AND c.initial_primary_oid IS NOT NULL AND EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND e.observation='changed')))
      AND (c.phase='selected' OR EXISTS(SELECT 1 FROM context_retirement_effect_intents e JOIN poll_retirement_freezes f ON f.binding_id=e.receipt_id WHERE e.operation_ulid=c.operation_ulid AND e.owner_epoch=1 AND e.resource='branch' AND e.primary_oid=c.initial_primary_oid AND f.operation_ulid=?12 AND f.owner_epoch=?19))
      AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents e WHERE e.operation_ulid=c.operation_ulid AND (e.resource!='branch' OR e.owner_epoch!=1)))", params![k.child_operation_id,p.repository,p.root,k.kind,k.item_id,k.expected_tip,p.remote,p.primary,p.generation,p.endpoint,p.request,p.operation,k.ordinal as i64,k.materialization_capture,k.materialization_operation_id,k.materialization_oid,k.materialization_step,p.request_attempt,p.epoch], |r|r.get(0)).map_err(|_|recovery_required())?;
    if !valid {
        return Ok(false);
    }
    let result: Option<String> = db
        .query_row(
            "SELECT final_result FROM context_retirement_records WHERE operation_ulid=?1",
            [&k.child_operation_id],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    if let Some(result) = result {
        let report = super::super::ContextRetirementReport::decode(&result)?;
        let refused: bool = db
            .query_row(
                "SELECT phase='refused' FROM context_retirement_records WHERE operation_ulid=?1",
                [&k.child_operation_id],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        let expected = if refused {
            super::super::ContextRetirementReport::refused(
                crate::results::ResultCode::ExternalChange,
            )
        } else {
            super::super::ContextRetirementReport::retired(
                super::super::RetirementRemoval::None,
                super::super::RetirementRemoval::Observed,
            )
        };
        if report != expected {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Live C exclusion, not historical correlation. A released remote slot does
/// not settle its integration stages or native resolution artifacts. Match the
/// existing next-window prerequisites in state::prepare_integration_window;
/// applied/released history and window retention alone are not live debt.
fn require_no_retained_integration(
    db: &Connection,
    repository: i64,
) -> Result<(), RepositoryError> {
    let pending: bool = db.query_row("SELECT
      EXISTS(SELECT 1 FROM remote_integration_steps step JOIN remote_operation_records operation ON operation.id=step.operation_record_id WHERE operation.repository_id=?1 AND step.phase!='applied')
      OR EXISTS(SELECT 1 FROM remote_resolution_attempts attempt JOIN remote_operation_records operation ON operation.id=attempt.operation_record_id LEFT JOIN remote_resolution_index_artifacts artifact ON artifact.attempt_id=attempt.id WHERE operation.repository_id=?1 AND (attempt.phase!='applied' OR artifact.phase IS NULL OR artifact.phase!='released' OR artifact.ref_phase!='observed'))",
      [repository], |r| r.get(0)).map_err(|_| recovery_required())?;
    if pending {
        return Err(recovery_required());
    }
    Ok(())
}

fn contract(db: &Connection) -> Result<(), RepositoryError> {
    for table in ["poll_retirement_cursors", "poll_retirement_targets"] {
        if !crate::repository::recovery::has_table(db, table).map_err(|_| recovery_required())? {
            return Err(recovery_required());
        }
    }
    if !db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CONTRACT],
            |r| r.get::<_, bool>(0),
        )
        .map_err(|_| recovery_required())?
    {
        return Err(recovery_required());
    }
    Ok(())
}

fn candidate_bytes(k: &Candidate) -> Result<String, RepositoryError> {
    Ok(serde_json::to_value(k)
        .map_err(|_| recovery_required())?
        .to_string())
}

fn refused_row(
    db: &Connection,
    p: &Projection,
    k: &Candidate,
    result: &str,
) -> Result<bool, RepositoryError> {
    let report = super::super::ContextRetirementReport::decode(result)?;
    if report
        != super::super::ContextRetirementReport::refused(
            crate::results::ResultCode::ExternalChange,
        )
    {
        return Ok(false);
    }
    db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch WHERE c.operation_ulid=?1 AND c.repository_id=?2 AND o.repository_id=?2 AND o.root_path=?3 AND o.operation_ulid=?1 AND o.action='retire_context' AND o.target='authoring-context-v1/'||c.kind||'/'||c.item_id||'/Retire' AND o.state='completed' AND o.completed_step IS NULL AND o.index_owner_epoch=0 AND o.persisted_context_count=0 AND o.context_path IS NULL AND c.kind=?4 AND c.item_id=?5 AND c.initial_resources=?6 AND c.expected_tip=?7 AND c.selection_epoch=1 AND c.owner_epoch=1 AND c.instance_nonce IS ?8 AND c.instance_verified=?9 AND c.remote_name=?10 AND c.primary_branch=?11 AND c.configuration_generation=?12 AND c.endpoint_digest=?13 AND c.initial_primary_oid IS NULL AND c.request_ulid=?14 AND c.batch_operation_ulid=?15 AND c.batch_ordinal=?16 AND c.materialization_capture=?17 AND c.materialization_operation_ulid IS ?18 AND c.materialization_oid IS ?19 AND c.materialization_step IS ?20 AND c.materialization_settled=0 AND c.phase='refused' AND c.worktree_removal='none' AND c.branch_removal='none' AND c.absence_basis='none' AND c.final_code='external_change' AND c.final_result=?21 AND a.repository_id=?2 AND a.request_attempt=?22 AND a.batch_attempt_epoch=?23 AND a.phase='ended' AND a.observer_operation_ulid IS NULL AND a.observer_owner_epoch IS NULL AND (SELECT count(*) FROM context_retirement_attempts WHERE operation_ulid=c.operation_ulid)=1 AND NOT EXISTS(SELECT 1 FROM context_retirement_effect_intents WHERE operation_ulid=c.operation_ulid) AND NOT EXISTS(SELECT 1 FROM context_retirement_index_claims WHERE operation_ulid=c.operation_ulid) AND NOT EXISTS(SELECT 1 FROM context_retirement_reentries WHERE operation_ulid=c.operation_ulid) AND NOT EXISTS(SELECT 1 FROM request_operations WHERE operation_ulid=c.operation_ulid))",params![k.child_operation_id,p.repository,p.root,k.kind,k.item_id,k.initial_resources,k.expected_tip,k.instance_nonce,k.instance_verified,p.remote,p.primary,p.generation,p.endpoint,p.request,p.operation,k.ordinal as i64,k.materialization_capture,k.materialization_operation_id,k.materialization_oid,k.materialization_step,result,p.request_attempt,p.epoch],|r|r.get(0)).map_err(|_|recovery_required())
}

/// Historical recognition only. It cannot produce any live C or child permit.
pub(super) fn admitted(db: &Connection, child: &str) -> Result<bool, RepositoryError> {
    let row:Option<(String,i64,Option<String>)>=db.query_row("SELECT c.batch_operation_ulid,a.batch_attempt_epoch,c.final_result FROM context_retirement_records c JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch WHERE c.operation_ulid=?1 AND c.batch_operation_ulid IS NOT NULL",[child],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|recovery_required())?;
    let Some((operation, epoch, result)) = row else {
        return Ok(false);
    };
    contract(db)?;
    let p = source::projection(db, &operation, epoch)?;
    let f = read(db, &p)?;
    let entries = decode_image(&f.image, f.count)?;
    let Some(k) = entries.iter().find(|k| k.child_operation_id == child) else {
        return Err(recovery_required());
    };
    if let Some(result) = result.as_deref()
        && refused_row(db, &p, k, result)?
    {
        return Ok(true);
    }
    positive_row(db, &p, k)
}

fn projected(
    db: &Connection,
    p: &Projection,
    f: &Frozen,
) -> Result<(Vec<Candidate>, usize, String), RepositoryError> {
    contract(db)?;
    let entries = decode_image(&f.image, f.count)?;
    let (cursor,stage):(i64,String)=db.query_row("SELECT next_ordinal,stage FROM poll_retirement_cursors WHERE operation_ulid=?1 AND owner_epoch=?2",params![p.operation,p.epoch],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
    let cursor = usize::try_from(cursor).map_err(|_| recovery_required())?;
    if cursor > entries.len() || !matches!(stage.as_str(), "active" | "complete") {
        return Err(recovery_required());
    }
    let rows:Vec<(i64,String,String,Option<String>,String)>=db.prepare("SELECT ordinal,candidate,state,final_result,handoff FROM poll_retirement_targets WHERE operation_ulid=?1 AND owner_epoch=?2 ORDER BY ordinal").map_err(|_|recovery_required())?.query_map(params![p.operation,p.epoch],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|_|recovery_required())?.collect::<Result<_,_>>().map_err(|_|recovery_required())?;
    if rows.len() != entries.len() {
        return Err(recovery_required());
    }
    for (i, (k, (ordinal, bytes, state, result, handoff))) in entries.iter().zip(rows).enumerate() {
        if ordinal != i as i64 || bytes != candidate_bytes(k)? {
            return Err(recovery_required());
        }
        if i < cursor {
            let result = result.as_deref().ok_or_else(recovery_required)?;
            let final_ok = match state.as_str() {
                "refused" => refused_row(db,p,k,result)? || (positive_row(db,p,k)? && db.query_row("SELECT phase='refused' AND final_result=?2 FROM context_retirement_records WHERE operation_ulid=?1",params![k.child_operation_id,result],|r|r.get::<_,bool>(0)).map_err(|_|recovery_required())?),
                "complete" => positive_row(db,p,k)? && db.query_row("SELECT phase='complete' AND final_result=?2 FROM context_retirement_records WHERE operation_ulid=?1",params![k.child_operation_id,result],|r|r.get::<_,bool>(0)).map_err(|_|recovery_required())?,
                _ => false,
            };
            if handoff != "final" || !final_ok {
                return Err(recovery_required());
            }
        } else {
            if i == cursor
                && matches!(
                    (state.as_str(), handoff.as_str()),
                    ("running", "cleanup") | ("running", "index_pending")
                )
            {
                let phase: String = db
                    .query_row(
                        "SELECT phase FROM context_retirement_records WHERE operation_ulid=?1",
                        [&k.child_operation_id],
                        |r| r.get(0),
                    )
                    .map_err(|_| recovery_required())?;
                if result.is_some()
                    || !positive_row(db, p, k)?
                    || (phase == "index_pending") != (handoff == "index_pending")
                    || phase == "complete"
                {
                    return Err(recovery_required());
                }
                continue;
            }
            if state != "waiting" || result.is_some() || handoff != "not_started" {
                return Err(recovery_required());
            }
            let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1) OR EXISTS(SELECT 1 FROM context_retirement_records WHERE operation_ulid=?1)",[&k.child_operation_id],|r|r.get(0)).map_err(|_|recovery_required())?;
            if exists {
                return Err(recovery_required());
            }
        }
    }
    if stage == "complete" {
        if cursor != entries.len()
            || p.phase != "ended"
            || f.eligibility != "eligible"
            || f.code.as_deref() != Some("ok")
        {
            return Err(recovery_required());
        }
    } else if (cursor == entries.len() && !(entries.is_empty() && f.eligibility != "eligible"))
        || f.code.is_some() && f.eligibility == "eligible"
        || f.eligibility != "eligible" && cursor != 0
    {
        return Err(recovery_required());
    }
    Ok((entries, cursor, stage))
}

/// Optional inert cursor projection: A1 never depends on a manifest, and the
/// original empty A3 remains valid without installing any Task 4 projection.
pub(super) fn settle_empty(tx: &Transaction<'_>, p: &Projection) -> Result<(), RepositoryError> {
    if !crate::repository::recovery::has_table(tx, "poll_retirement_cursors")
        .map_err(|_| recovery_required())?
    {
        return Ok(());
    }
    contract(tx)?;
    if tx.execute("UPDATE poll_retirement_cursors SET stage='complete' WHERE operation_ulid=?1 AND owner_epoch=?2 AND next_ordinal=0 AND stage='active' AND NOT EXISTS(SELECT 1 FROM poll_retirement_targets WHERE operation_ulid=?1 AND owner_epoch=?2)",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
    Ok(())
}

pub(in super::super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    let children:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records WHERE batch_operation_ulid IS NOT NULL)",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CONTRACT],
            |r| r.get(0),
        )
        .map_err(|_| recovery_required())?;
    if !marked {
        return if children {
            Err(recovery_required())
        } else {
            Ok(())
        };
    }
    contract(db)?;
    let orphan:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_targets t LEFT JOIN poll_retirement_cursors c USING(operation_ulid,owner_epoch) WHERE c.operation_ulid IS NULL) OR EXISTS(SELECT 1 FROM context_retirement_records c WHERE c.batch_operation_ulid IS NOT NULL AND NOT EXISTS(SELECT 1 FROM poll_retirement_targets t JOIN poll_retirement_cursors cursor USING(operation_ulid,owner_epoch) WHERE t.operation_ulid=c.batch_operation_ulid AND t.ordinal=c.batch_ordinal AND ((t.ordinal<cursor.next_ordinal AND t.state IN ('refused','complete') AND t.final_result=c.final_result) OR (t.ordinal=cursor.next_ordinal AND t.state='running' AND t.final_result IS NULL AND c.final_result IS NULL)) AND json_extract(t.candidate,'$.child_operation_id')=c.operation_ulid))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if orphan {
        return Err(recovery_required());
    }
    let mut query = db
        .prepare(
            "SELECT operation_ulid,owner_epoch,next_ordinal,stage FROM poll_retirement_cursors",
        )
        .map_err(|_| recovery_required())?;
    for row in query
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| recovery_required())?
    {
        let (operation, epoch, cursor, stage) = row.map_err(|_| recovery_required())?;
        let p = source::projection(db, &operation, epoch)?;
        if p.stage == "unplanned" {
            if cursor!=0 || stage!="active" || db.query_row("SELECT EXISTS(SELECT 1 FROM poll_retirement_targets WHERE operation_ulid=?1 AND (state!='waiting' OR final_result IS NOT NULL OR handoff!='not_started'))",[operation],|r|r.get::<_,bool>(0)).map_err(|_|recovery_required())? {return Err(recovery_required());}
        } else {
            projected(db, &p, &read(db, &p)?)?;
        }
    }
    Ok(())
}

pub(super) fn completed(
    db: &Connection,
    p: &Projection,
    f: &Frozen,
) -> Result<bool, RepositoryError> {
    if !crate::repository::recovery::has_table(db, "poll_retirement_cursors")
        .map_err(|_| recovery_required())?
    {
        return Ok(false);
    }
    let (_, _, stage) = projected(db, p, f)?;
    Ok(stage == "complete")
}

pub(super) fn require_completed_release(
    db: &Connection,
    provenance: i64,
    binding: i64,
) -> Result<(), RepositoryError> {
    let (operation,epoch):(String,i64)=db.query_row("SELECT operation_ulid,owner_epoch FROM poll_retirement_freezes WHERE provenance_id=?1 AND binding_id=?2",params![provenance,binding],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
    let p = source::projection(db, &operation, epoch)?;
    let f = read(db, &p)?;
    if !completed(db, &p, &f)? {
        return Err(recovery_required());
    }
    let source_ended:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND operation_ulid=?2 AND owner_epoch=?3 AND action='poll' AND poll_kind='update' AND priority='manual' AND phase='completed' AND outcome='completed' AND configuration_generation=?4 AND remote_name=?5 AND primary_branch=?6)",params![p.repository,p.operation,p.source_epoch,p.generation,p.remote,p.primary],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !source_ended {
        return Err(recovery_required());
    }
    crate::repository::recovery::require_no_pending_local(db, p.repository)
}

pub(in super::super::super) struct Live {
    capture: Arc<OriginalCapture>,
    branches: std::sync::Mutex<Vec<Option<BranchLifetime>>>,
    refusals: std::sync::Mutex<std::collections::BTreeMap<usize, execution::RefusalWitness>>,
}
pub(in super::super::super) struct Child<'a> {
    live: &'a Live,
    ordinal: usize,
}
impl Live {
    pub(super) fn from_original(capture: Capture) -> Self {
        Self {
            capture: Arc::new(capture.evidence),
            branches: std::sync::Mutex::new(capture.branches),
            refusals: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }
    #[allow(dead_code)] // Task 4 owns invoking individual private transitions.
    pub(in super::super::super) fn next<'a>(
        &'a self,
        service: &RepositoryService,
        root: &Path,
    ) -> Result<Child<'a>, RepositoryError> {
        state::with_transaction(service, root, |tx, id| {
            let (_, _, ordinal) = self.validate(service, tx, id, root)?;
            Ok(Child {
                live: self,
                ordinal,
            })
        })
    }
    fn validate(
        &self,
        service: &RepositoryService,
        tx: &Transaction<'_>,
        id: i64,
        root: &Path,
    ) -> Result<(Frozen, Vec<Candidate>, usize), RepositoryError> {
        if !Arc::ptr_eq(&self.capture.scope, &service.remote_reservation_scope)
            || self.capture.registry != service.registry_path
        {
            return Err(recovery_required());
        }
        self.validate_owned(tx, id, root, None)
    }

    fn validate_owned(
        &self,
        tx: &Connection,
        id: i64,
        root: &Path,
        own: Option<i64>,
    ) -> Result<(Frozen, Vec<Candidate>, usize), RepositoryError> {
        Self::validate_capture(&self.capture, tx, id, root, own)
    }

    fn validate_capture(
        c: &OriginalCapture,
        tx: &Connection,
        id: i64,
        root: &Path,
        own: Option<i64>,
    ) -> Result<(Frozen, Vec<Candidate>, usize), RepositoryError> {
        let (f, entries, cursor) = Self::validate_metadata(c, tx, id, root)?;
        let p = &c.p;
        let deletion:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM request_records WHERE request_ulid=?1 AND attempt=?2 AND state='accepted' AND cancel_requested=0) AND EXISTS(SELECT 1 FROM retirement_observation_pins pin JOIN remote_observation_batches batch ON batch.id=pin.observation_batch_id WHERE pin.binding_id=?3 AND pin.pin_kind='consumer_attempt' AND batch.is_current=1 AND batch.configuration_generation=?4)",params![p.request,p.request_attempt,f.binding,p.generation],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !deletion || state::generation(tx, id)? != p.generation {
            return Err(recovery_required());
        }
        if let Some(local) = own {
            // The validated exact current Ki is the ONLY pending exemption.
            let foreign: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE root_path=?1 AND (state!='completed' OR completed_step='comment_registration_pending') AND id!=?2)",params![p.root,local],|r|r.get(0)).map_err(|_|recovery_required())?;
            if foreign {
                return Err(recovery_required());
            }
            crate::repository::local_sync::record::require_no_pending(tx, id)?;
        } else {
            crate::repository::recovery::require_no_pending_local(tx, id)?;
        }
        require_no_retained_integration(tx, id)?;
        let blocked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND (phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') OR index_pending!=0 OR reconciliation_required!=0))",[id],|r|r.get(0)).map_err(|_|recovery_required())?;
        let plan = crate::repository::RemoteRefPlan::from_configuration(&p.remote, &p.primary)
            .map_err(|_| recovery_required())?;
        if blocked
            || !c.configuration.matches_owned_at(
                tx,
                root,
                &plan,
                c.registry.parent().ok_or_else(recovery_required)?,
            )?
        {
            return Err(recovery_required());
        }
        Ok((f, entries, cursor))
    }

    /// Exact current L/R/batch ownership and immutable original binding history.
    /// This is NOT C/G/F deletion eligibility: authorized index/no-effect
    /// settlement must survive endpoint/key/G/ad changes and current cancellation.
    fn validate_metadata(
        c: &OriginalCapture,
        tx: &Connection,
        id: i64,
        root: &Path,
    ) -> Result<(Frozen, Vec<Candidate>, usize), RepositoryError> {
        if tx.path() != c.registry.to_str() {
            return Err(recovery_required());
        }
        let p = &c.p;
        if root.to_str() != Some(p.root.as_str()) || id != p.repository {
            return Err(recovery_required());
        }
        record::audit_connection(tx)?;
        let mut current = source::projection(tx, &p.operation, p.epoch)?;
        current.stage = "unplanned".into();
        if current != *p {
            return Err(recovery_required());
        }
        let f = read(tx, p)?;
        if f.eligibility != "eligible"
            || f.code.is_some()
            || f.nonce != c.nonce
            || f.report != c.report
            || f.digest != c.digest
        {
            return Err(recovery_required());
        }
        let (entries, cursor, stage) = projected(tx, p, &f)?;
        if stage != "active" {
            return Err(recovery_required());
        }
        let live:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM request_records WHERE request_ulid=?1 AND attempt=?2 AND state='accepted') AND EXISTS(SELECT 1 FROM poll_retirement_batches WHERE operation_ulid=?3 AND owner_epoch=?4) AND EXISTS(SELECT 1 FROM retirement_observation_receipts b JOIN retirement_advertisement_provenance provenance ON provenance.id=b.provenance_id JOIN retirement_observation_pins pin ON pin.binding_id=b.id AND pin.pin_kind='consumer_attempt' JOIN remote_observation_batches batch ON batch.id=pin.observation_batch_id WHERE b.id=?5 AND provenance.id=?6 AND batch.id=provenance.observation_batch_id AND batch.repository_id=?7 AND batch.configuration_generation=?8 AND pin.holder_operation_ulid=?3 AND pin.holder_epoch=?4)",params![p.request,p.request_attempt,p.operation,p.epoch,f.binding,f.provenance,p.repository,p.generation],|r|r.get(0)).map_err(|_|recovery_required())?;
        if !live {
            return Err(recovery_required());
        }
        Ok((f, entries, cursor))
    }
}
impl Child<'_> {
    #[allow(dead_code)]
    pub(in super::super::super) fn execute_branch_only(
        self,
        service: &RepositoryService,
    ) -> Result<super::super::ContextRetirementOutcome, RepositoryError> {
        self.execute(service)
    }

    #[allow(dead_code)] // No public command/coordinator is installed.
    pub(in super::super::super) fn refuse_if_changed(
        self,
        service: &RepositoryService,
    ) -> Result<super::super::ContextRetirementReport, RepositoryError> {
        let c = &self.live.capture;
        let p = &c.p;
        let root = Path::new(&p.root);
        let git = git2::Repository::open(root).map_err(|_| recovery_required())?;
        let _lease = repository_lease(&git, root, RepositoryOperation::RetireContext)?;
        state::with_transaction(service, root, |tx, id| {
            let (f, entries, cursor) = self.live.validate(service, tx, id, root)?;
            if self.ordinal != cursor {
                return Err(recovery_required());
            }
            let k = entries.get(cursor).ok_or_else(recovery_required)?;
            // No unchanged target acquires destructive/index authority here.
            // Only positively inspected mismatches may become a no-effect Final;
            // unknown evidence (including SQL after a nonce change) stays closed.
            if compare_candidate(tx, p, k)? != super::super::instance::FrozenComparison::Changed {
                return Err(recovery_required());
            }
            let report = super::super::ContextRetirementReport::refused(
                crate::results::ResultCode::ExternalChange,
            );
            let result = report.encode()?;
            if tx.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,?3,'retire_context',?4,'completed',?5)",params![id,p.root,k.child_operation_id,format!("authoring-context-v1/{}/{}/Retire",k.kind,k.item_id),service.clock.now().unix_timestamp()]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            let local = tx.last_insert_rowid();
            if tx.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,selection_epoch,instance_nonce,instance_verified,remote_name,primary_branch,configuration_generation,endpoint_digest,request_ulid,batch_operation_ulid,batch_ordinal,materialization_capture,materialization_operation_ulid,materialization_oid,materialization_step,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis,final_code,final_result) VALUES(?1,?2,?3,1,?4,?5,?6,?7,1,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,0,1,'refused','none','none','none','external_change',?21)",params![k.child_operation_id,local,id,k.kind,k.item_id,k.initial_resources,k.expected_tip,k.instance_nonce,k.instance_verified,p.remote,p.primary,p.generation,p.endpoint,p.request,p.operation,cursor as i64,k.materialization_capture,k.materialization_operation_id,k.materialization_oid,k.materialization_step,result]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            if tx.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,request_attempt,batch_attempt_epoch,phase) VALUES(?1,1,?2,?3,?4,'ended')",params![k.child_operation_id,id,p.request_attempt,p.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            if tx.execute("UPDATE poll_retirement_targets SET state='refused',final_result=?4,handoff='final' WHERE operation_ulid=?1 AND owner_epoch=?2 AND ordinal=?3 AND state='waiting' AND final_result IS NULL",params![p.operation,p.epoch,cursor as i64,result]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            let last = cursor + 1 == entries.len();
            if tx.execute("UPDATE poll_retirement_cursors SET next_ordinal=next_ordinal+1,stage=?4 WHERE operation_ulid=?1 AND owner_epoch=?2 AND next_ordinal=?3 AND stage='active'",params![p.operation,p.epoch,cursor as i64,if last{"complete"}else{"active"}]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
            if last {
                if tx.execute("UPDATE poll_retirement_freezes SET final_code='ok' WHERE operation_ulid=?1 AND owner_epoch=?2 AND eligibility='eligible' AND final_code IS NULL",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
                if tx.execute("UPDATE poll_retirement_attempts SET phase='ended' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![p.operation,p.epoch]).map_err(|_|recovery_required())?!=1 {return Err(recovery_required());}
                record::release_bound_source(tx, f.provenance, f.binding, true)?;
            }
            record::audit_connection(tx)?;
            Ok(report)
        })
    }
}

/// Historical Final read only; never reconstructs a live batch/child scope.
#[allow(dead_code)]
pub(in super::super::super) fn read_refusal(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
    ordinal: usize,
) -> Result<Option<super::super::ContextRetirementReport>, RepositoryError> {
    read_final_kind(service, root, operation, ordinal, true)
}

#[allow(dead_code)]
pub(in super::super::super) fn read_final(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
    ordinal: usize,
) -> Result<Option<super::super::ContextRetirementReport>, RepositoryError> {
    read_final_kind(service, root, operation, ordinal, false)
}

fn read_final_kind(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
    ordinal: usize,
    refusal: bool,
) -> Result<Option<super::super::ContextRetirementReport>, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        contract(tx)?;
        record::audit_connection(tx)?;
        let epoch: i64 = tx
            .query_row(
                "SELECT owner_epoch FROM poll_retirement_freezes WHERE operation_ulid=?1",
                [operation.to_string()],
                |r| r.get(0),
            )
            .map_err(|_| recovery_required())?;
        let p = source::projection(tx, &operation.to_string(), epoch)?;
        if p.repository != id || root.to_str() != Some(p.root.as_str()) {
            return Err(recovery_required());
        }
        let f = read(tx, &p)?;
        let (entries, cursor, _) = projected(tx, &p, &f)?;
        if ordinal >= entries.len() {
            return Err(recovery_required());
        }
        if ordinal >= cursor {
            return Ok(None);
        }
        let result:String=tx.query_row("SELECT final_result FROM poll_retirement_targets WHERE operation_ulid=?1 AND owner_epoch=?2 AND ordinal=?3 AND (state='refused' OR (?4=0 AND state='complete')) AND handoff='final'",params![p.operation,p.epoch,ordinal as i64,refusal],|r|r.get(0)).map_err(|_|recovery_required())?;
        Ok(Some(super::super::ContextRetirementReport::decode(
            &result,
        )?))
    })
}
