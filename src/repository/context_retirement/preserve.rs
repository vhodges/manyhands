//! Confirmed per-item preserving escape. No Git deletion, proof or transport.
//! The old Ri mapping is permanent; only the confirmed abandon request can
//! claim preservation's local index work. Resume takes that same request up.
use super::{ContextRetirementReport, RetirementRemoval, record, request_scope::RequestFence};
use crate::repository::{
    self, OperationId, RefreshOutcome, RepositoryError, RepositoryOperation, RepositoryService,
    mutation, recovery, remote::state,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use state::recovery_required;
use std::path::Path;

pub(super) const MARKER: &str = "context_retirement_preserving_v1";
pub(super) fn objects() -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("table", "context_retirement_preservations", "CREATE TABLE context_retirement_preservations (
 operation_ulid TEXT PRIMARY KEY NOT NULL REFERENCES context_retirement_records(operation_ulid) ON DELETE NO ACTION,
 abandon_request_ulid TEXT NOT NULL UNIQUE REFERENCES request_records(request_ulid) ON DELETE NO ACTION,
 accepted_attempt INTEGER NOT NULL CHECK(typeof(accepted_attempt)='integer' AND accepted_attempt>0),
 owner_epoch INTEGER NOT NULL CHECK(typeof(owner_epoch)='integer' AND owner_epoch>0),
 snapshot TEXT NOT NULL CHECK(json_valid(snapshot))
)".into()),
        ("table", "context_retirement_preservation_claims", "CREATE TABLE context_retirement_preservation_claims (
 operation_ulid TEXT NOT NULL REFERENCES context_retirement_preservations(operation_ulid) ON DELETE NO ACTION,
 request_attempt INTEGER NOT NULL CHECK(typeof(request_attempt)='integer' AND request_attempt>0),
 index_owner_epoch INTEGER NOT NULL CHECK(typeof(index_owner_epoch)='integer' AND index_owner_epoch>0),
 PRIMARY KEY(operation_ulid,request_attempt), UNIQUE(operation_ulid,index_owner_epoch)
)".into()),
        ("trigger", "retirement_preservation_immutable", "CREATE TRIGGER retirement_preservation_immutable BEFORE UPDATE ON context_retirement_preservations BEGIN SELECT RAISE(ABORT,'immutable preserving consent'); END".into()),
        ("trigger", "retirement_preservation_retained", "CREATE TRIGGER retirement_preservation_retained BEFORE DELETE ON context_retirement_preservations BEGIN SELECT RAISE(ABORT,'retained preserving consent'); END".into()),
        ("trigger", "retirement_preservation_claim_immutable", "CREATE TRIGGER retirement_preservation_claim_immutable BEFORE UPDATE ON context_retirement_preservation_claims BEGIN SELECT RAISE(ABORT,'immutable preserving spend'); END".into()),
        ("trigger", "retirement_preservation_claim_retained", "CREATE TRIGGER retirement_preservation_claim_retained BEFORE DELETE ON context_retirement_preservation_claims BEGIN SELECT RAISE(ABORT,'retained preserving spend'); END".into()),
    ]
}
pub(super) fn marked(db: &Connection) -> Result<bool, RepositoryError> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
        [MARKER],
        |r| r.get(0),
    )
    .map_err(|_| recovery_required())
}

pub(super) fn validate_closure(
    db: &Connection,
    definitions: &[(&str, &str, String)],
) -> Result<(), RepositoryError> {
    for (table, implicit_count) in [
        ("context_retirement_preservations", 2),
        ("context_retirement_preservation_claims", 2),
    ] {
        let mut q=db.prepare("SELECT type,name,sql FROM sqlite_master WHERE tbl_name=?1 COLLATE NOCASE AND type IN ('index','trigger')").map_err(|_|recovery_required())?;
        for row in q
            .query_map([table], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|_| recovery_required())?
        {
            let (kind, name, sql) = row.map_err(|_| recovery_required())?;
            let implicit = kind == "index"
                && sql.is_none()
                && (1..=implicit_count)
                    .any(|i| name.eq_ignore_ascii_case(&format!("sqlite_autoindex_{table}_{i}")));
            if !implicit
                && !definitions
                    .iter()
                    .any(|(k, n, _)| *k == kind && name.eq_ignore_ascii_case(n))
            {
                return Err(recovery_required());
            }
        }
        let mut q=db.prepare("SELECT m.name FROM sqlite_master m JOIN pragma_foreign_key_list(m.name) fk WHERE m.type='table' AND fk.\"table\"=?1 COLLATE NOCASE").map_err(|_|recovery_required())?;
        for child in q
            .query_map([table], |r| r.get::<_, String>(0))
            .map_err(|_| recovery_required())?
        {
            let child = child.map_err(|_| recovery_required())?;
            if !definitions
                .iter()
                .any(|(kind, name, _)| *kind == "table" && child.eq_ignore_ascii_case(name))
            {
                return Err(recovery_required());
            }
        }
    }
    let mut q=db.prepare("SELECT name FROM sqlite_master WHERE name LIKE 'retirement_preservation_%' OR name LIKE 'context_retirement_preserv%'").map_err(|_|recovery_required())?;
    for name in q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| recovery_required())?
    {
        let name = name.map_err(|_| recovery_required())?;
        if !definitions
            .iter()
            .any(|(_, n, _)| name.eq_ignore_ascii_case(n))
        {
            return Err(recovery_required());
        }
    }
    Ok(())
}

/// Closed confirmation metadata. Retained means present OR not safely known
/// absent; it never grants authority to act on a resource.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementAbandonMetadata {
    pub request_id: Option<String>,
    pub phase: String,
    pub owner_epoch: i64,
    pub index_owner_epoch: i64,
    pub worktree_removal: RetirementRemoval,
    pub branch_removal: RetirementRemoval,
    pub worktree_retained: bool,
    pub branch_retained: bool,
    pub index_pending: bool,
}
fn removal(text: &str) -> Result<RetirementRemoval, RepositoryError> {
    match text {
        "none" => Ok(RetirementRemoval::None),
        "possible" => Ok(RetirementRemoval::Possible),
        "observed" => Ok(RetirementRemoval::Observed),
        _ => Err(recovery_required()),
    }
}
pub(crate) fn metadata(
    db: &Connection,
    root: &Path,
    operation: OperationId,
) -> Result<Option<RetirementAbandonMetadata>, RepositoryError> {
    if !record::requires_guarded_abandon(db, operation)? {
        return Ok(None);
    }
    record::audit_connection(db)?;
    let (request,phase,epoch,index_epoch,w,b,kind,item):(Option<String>,String,i64,i64,String,String,String,String)=db.query_row("SELECT c.request_ulid,c.phase,c.owner_epoch,o.index_owner_epoch,c.worktree_removal,c.branch_removal,c.kind,c.item_id FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id WHERE c.operation_ulid=?1 AND o.root_path=?2",params![operation.to_string(),root.to_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).map_err(|_|recovery_required())?;
    let worktree_retained = !matches!(
        super::execution::absent_path(root, &root.join(".manyhands/worktrees").join(&item)),
        Ok(true)
    );
    let branch_ref = format!("refs/heads/manyhands/{kind}/{item}");
    let branch_retained = match git2::Repository::open(root) {
        Ok(git) => {
            !git.find_reference(&branch_ref)
                .is_err_and(|e| e.code() == git2::ErrorCode::NotFound)
                || !matches!(
                    super::execution::absent_path(root, &git.commondir().join(&branch_ref)),
                    Ok(true)
                )
        }
        Err(_) => true,
    };
    Ok(Some(RetirementAbandonMetadata {
        request_id: request,
        phase: phase.clone(),
        owner_epoch: epoch,
        index_owner_epoch: index_epoch,
        worktree_removal: removal(&w)?,
        branch_removal: removal(&b)?,
        worktree_retained,
        branch_retained,
        index_pending: !matches!(phase.as_str(), "complete" | "refused" | "abandoned"),
    }))
}

fn accepted(
    db: &Connection,
    root: &Path,
    operation: OperationId,
    request: mutation::RequestId,
    attempt: i64,
) -> Result<(), RepositoryError> {
    use mutation::request_store::{DigestSalt, IntentDigest, ScopeKey};
    let scope = ScopeKey::from_stored(root.to_str().ok_or_else(recovery_required)?);
    let digest = IntentDigest::builder(
        DigestSalt::Request(request),
        "operation abandon",
        &scope,
        &operation.to_string(),
    )
    .finish();
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM request_records r JOIN confirmation_records c ON c.confirmation_ulid=r.confirmation_ulid WHERE r.request_ulid=?1 AND r.attempt=?2 AND r.state='accepted' AND r.scope_key=?3 AND r.command='operation abandon' AND r.target=?4 AND r.intent_digest=?5 AND r.base_ref IS NULL AND r.base_oid IS NULL AND r.expected_digest IS NULL AND c.accepted_by_request=r.request_ulid AND NOT EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=r.request_ulid))",params![request.to_string(),attempt,root.to_str(),operation.to_string(),digest.to_string()],|r|r.get(0)).map_err(|_|recovery_required())?;
    if !valid {
        return Err(recovery_required());
    }
    Ok(())
}
pub(crate) fn owner(
    db: &Connection,
    operation: OperationId,
) -> Result<Option<mutation::RequestId>, RepositoryError> {
    db.query_row(
        "SELECT abandon_request_ulid FROM context_retirement_preservations WHERE operation_ulid=?1",
        [operation.to_string()],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(|_| recovery_required())?
    .map(|id| mutation::RequestId::parse(&id).map_err(|_| recovery_required()))
    .transpose()
}
pub(crate) fn owns(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
    request: mutation::RequestId,
) -> Result<bool, RepositoryError> {
    state::with_transaction(service, root, |tx, _| {
        record::audit_connection(tx)?;
        Ok(owner(tx, operation)? == Some(request))
    })
}

pub(crate) fn snapshot(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
) -> Result<Option<RetirementAbandonMetadata>, RepositoryError> {
    state::with_transaction(service, root, |tx, _| {
        record::audit_connection(tx)?;
        let text: Option<String> = tx
            .query_row(
                "SELECT snapshot FROM context_retirement_preservations WHERE operation_ulid=?1",
                [operation.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| recovery_required())?;
        text.map(|text| serde_json::from_str(&text).map_err(|_| recovery_required()))
            .transpose()
    })
}

pub(crate) fn begin(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
    expected: &RetirementAbandonMetadata,
    request: &mutation::Accepted,
) -> Result<RefreshOutcome, RepositoryError> {
    let (git, root) =
        repository::canonical_repository_root(root, RepositoryOperation::AbandonOperation)?;
    let lease = repository::repository_lease(&git, &root, RepositoryOperation::AbandonOperation)?;
    state::with_transaction(service, &root, |tx, id| {
        record::audit_connection(tx)?;
        let attempt = i64::try_from(request.attempt).map_err(|_| recovery_required())?;
        accepted(tx, &root, operation, request.request_id, attempt)?;
        if let Some(owner) = owner(tx, operation)? {
            if owner != request.request_id {
                return Err(recovery_required());
            }
            return Ok(());
        }
        let current = metadata(tx, &root, operation)?.ok_or_else(recovery_required)?;
        if current != *expected || !current.index_pending {
            return Err(RepositoryError::new(
                RepositoryOperation::AbandonOperation,
                Some(root.clone()),
                repository::RepositoryErrorKind::ExternalChange,
                "the confirmation snapshot changed",
            ));
        }
        let row = recovery::local_row(tx, root.to_str().ok_or_else(recovery_required)?, operation)
            .map_err(RepositoryError::sqlite)?
            .ok_or_else(recovery_required)?;
        if row.has_live_heartbeat(
            service.clock.now().unix_timestamp() - repository::INDEX_OWNER_STALE_AFTER,
        ) {
            return Err(RepositoryError::new(
                RepositoryOperation::AbandonOperation,
                Some(root.clone()),
                repository::RepositoryErrorKind::RepositoryBusy,
                "a local index owner is active",
            ));
        }
        let (observer,observer_epoch):(Option<String>,Option<i64>)=tx.query_row("SELECT a.observer_operation_ulid,a.observer_owner_epoch FROM context_retirement_records c JOIN context_retirement_attempts a ON a.operation_ulid=c.operation_ulid AND a.owner_epoch=c.owner_epoch WHERE c.operation_ulid=?1 AND c.batch_operation_ulid IS NULL AND a.phase='active'",[operation.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|recovery_required())?;
        let blocked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE (repository_id=?1 OR root_path=?2) AND operation_ulid IS NOT ?3 AND (state!='completed' OR completed_step='comment_registration_pending')) OR EXISTS(SELECT 1 FROM remote_operation_records WHERE repository_id=?1 AND ((operation_ulid IS NOT ?4 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')) OR index_pending!=0 OR reconciliation_required!=0))",params![id,root.to_str(),operation.to_string(),observer],|r|r.get(0)).map_err(|_|recovery_required())?;
        if blocked {
            return Err(RepositoryError::new(
                RepositoryOperation::AbandonOperation,
                Some(root.clone()),
                repository::RepositoryErrorKind::RepositoryBusy,
                "another operation owns repository work",
            ));
        }
        if let Some(observer) = observer {
            // Full immutable parent/O correlation was audited before this write.
            if tx.execute("UPDATE remote_operation_records SET owner_epoch=owner_epoch+1,phase=CASE WHEN phase IN ('reserved','advertising','persisting') THEN 'interrupted' ELSE phase END,outcome=CASE WHEN phase IN ('reserved','advertising','persisting') THEN 'repository_unavailable' ELSE outcome END WHERE operation_ulid=?1 AND repository_id=?2 AND owner_epoch=?3 AND action='poll' AND poll_kind='observe' AND priority='manual' AND phase IN ('reserved','advertising','persisting','completed','cancelled','failed','interrupted')",params![observer,id,observer_epoch]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        }
        tx.execute("INSERT INTO context_retirement_preservations(operation_ulid,abandon_request_ulid,accepted_attempt,owner_epoch,snapshot) VALUES(?1,?2,?3,?4,?5)",params![operation.to_string(),request.request_id.to_string(),attempt,current.owner_epoch,serde_json::to_string(&current).map_err(|_|recovery_required())?]).map_err(|_|recovery_required())?;
        if tx.execute("UPDATE context_retirement_attempts SET phase='fenced' WHERE operation_ulid=?1 AND owner_epoch=?2 AND phase='active'",params![operation.to_string(),current.owner_epoch]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        if let Some(old) = &current.request_id {
            tx.execute("UPDATE request_records SET attempt=attempt+1 WHERE request_ulid=?1 AND state='accepted'",[old]).map_err(|_|recovery_required())?;
        }
        tx.execute("UPDATE context_retirement_records SET phase='preserving' WHERE operation_ulid=?1 AND final_result IS NULL",[operation.to_string()]).map_err(|_|recovery_required())?;
        // Invalidation advanced the index epoch without spending it. Preserve
        // that tombstone until refresh makes the abandon request's real claim;
        // relabeling it here would falsely require a spend at the fenced epoch.
        tx.execute("UPDATE operation_records SET state=CASE WHEN state='index_invalidated' THEN state ELSE 'authoritative_observed' END,completed_step='context_retirement_preserved' WHERE id=?1",[row.id]).map_err(|_|recovery_required())?;
        tx.execute(
            "UPDATE repositories SET refresh_required=1 WHERE id=?1",
            [id],
        )
        .map_err(|_| recovery_required())?;
        record::audit_connection(tx)
    })?;
    drop(lease);
    refresh(service, &root, operation, request)
}

pub(crate) fn refresh(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
    request: &mutation::Accepted,
) -> Result<RefreshOutcome, RepositoryError> {
    let (git, root) =
        repository::canonical_repository_root(root, RepositoryOperation::AbandonOperation)?;
    let fence = RequestFence::preserving(request, operation)?;
    let (id, claim) = state::with_transaction(service, &root, |tx, id| {
        record::audit_connection(tx)?;
        accepted(
            tx,
            &root,
            operation,
            request.request_id,
            i64::try_from(request.attempt).map_err(|_| recovery_required())?,
        )?;
        let (local,phase,epoch):(i64,String,i64)=tx.query_row("SELECT o.id,c.phase,o.index_owner_epoch FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_preservations p ON p.operation_ulid=c.operation_ulid WHERE c.operation_ulid=?1 AND p.abandon_request_ulid=?2",params![operation.to_string(),request.request_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_|recovery_required())?;
        if phase == "abandoned" {
            return Ok((id, None));
        }
        if phase != "preserving" {
            return Err(recovery_required());
        }
        let spent:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_preservation_claims WHERE operation_ulid=?1 AND request_attempt>=?2)",params![operation.to_string(),request.attempt],|r|r.get(0)).map_err(|_|recovery_required())?;
        let row = recovery::local_row(tx, root.to_str().ok_or_else(recovery_required)?, operation)
            .map_err(|_| recovery_required())?
            .ok_or_else(recovery_required)?;
        if spent
            || row.has_live_heartbeat(
                service.clock.now().unix_timestamp() - repository::INDEX_OWNER_STALE_AFTER,
            )
        {
            return Ok((id, None));
        }
        let next = epoch.checked_add(1).ok_or_else(recovery_required)?;
        if tx.execute("UPDATE operation_records SET state='indexing',index_owner_epoch=?2,observed_at=?3 WHERE id=?1 AND state IN ('authoritative_observed','indexing','failed','observed','persisted','retry','index_invalidated') AND index_owner_epoch=?4",params![local,next,service.clock.now().unix_timestamp(),epoch]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
        tx.execute(
            "INSERT INTO context_retirement_preservation_claims VALUES(?1,?2,?3)",
            params![operation.to_string(), request.attempt, next],
        )
        .map_err(|_| recovery_required())?;
        Ok((
            id,
            Some(recovery::IndexOwner {
                record_id: local,
                epoch: next,
                retirement_request: Some(fence),
                retirement_batch: None,
            }),
        ))
    })?;
    let Some(owner) = claim else {
        return Ok(RefreshOutcome::IndexPending { root });
    };
    let heartbeat = repository::IndexOwnerHeartbeat::start(
        service.registry_path.clone(),
        owner.clone(),
        root.clone(),
        service.take_index_owner_heartbeat_hook(),
    );
    service.run_refresh_claim_hook();
    let result = service.refresh_repository_claimed(&git, root.clone(), id, owner.clone());
    drop(heartbeat);
    match result {
        Ok(result) => Ok(result),
        Err(_) => {
            // A claimed preserving refresh is real index debt. Fence no other
            // owner, release nothing, and never send this back to cleanup.
            let _ = repository::set_refresh_operation(
                &service.registry_path,
                owner,
                "failed",
                None,
                &root,
            );
            Ok(RefreshOutcome::IndexPending { root })
        }
    }
}

pub(super) fn validate_index(
    db: &Connection,
    local: i64,
    fence: RequestFence,
    index_epoch: Option<i64>,
) -> Result<(), RepositoryError> {
    record::audit_connection(db)?;
    let (root, operation): (String, String) = db
        .query_row(
            "SELECT root_path,operation_ulid FROM operation_records WHERE id=?1",
            [local],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| recovery_required())?;
    if !fence.is_preserving_for(&operation) {
        return Err(recovery_required());
    }
    accepted(
        db,
        Path::new(&root),
        OperationId::parse(&operation).map_err(|_| recovery_required())?,
        fence.request(),
        fence.attempt(),
    )?;
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id JOIN context_retirement_preservations p ON p.operation_ulid=c.operation_ulid WHERE o.id=?1 AND c.phase='preserving' AND o.completed_step='context_retirement_preserved' AND p.abandon_request_ulid=?2 AND (?3 IS NULL OR (o.state='indexing' AND o.index_owner_epoch=?3 AND EXISTS(SELECT 1 FROM context_retirement_preservation_claims i WHERE i.operation_ulid=c.operation_ulid AND i.request_attempt=?4 AND i.index_owner_epoch=?3))))",params![local,fence.id(),index_epoch,fence.attempt()],|r|r.get(0)).map_err(|_|recovery_required())?;
    if valid {
        Ok(())
    } else {
        Err(recovery_required())
    }
}
pub(super) fn finalize(
    tx: &Transaction<'_>,
    owner: &recovery::IndexOwner,
) -> Result<(), RepositoryError> {
    let f = owner.retirement_request.ok_or_else(recovery_required)?;
    validate_index(tx, owner.record_id, f, Some(owner.epoch))?;
    let (operation,epoch,w,b,settled):(String,i64,String,String,bool)=tx.query_row("SELECT c.operation_ulid,c.owner_epoch,c.worktree_removal,c.branch_removal,c.materialization_settled FROM context_retirement_records c WHERE local_record_id=?1",[owner.record_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|_|recovery_required())?;
    let report = ContextRetirementReport::preserved(removal(&w)?, removal(&b)?, settled);
    tx.execute("UPDATE context_retirement_records SET phase='abandoned',final_code='external_change',final_result=?2 WHERE operation_ulid=?1 AND phase='preserving'",params![operation,report.encode()?]).map_err(|_|recovery_required())?;
    record::release_single_consumer(tx, &operation, epoch)?;
    if tx.execute("UPDATE operation_records SET state='completed',completed_step='abandoned',context_path=NULL WHERE id=?1 AND state='indexing' AND index_owner_epoch=?2",params![owner.record_id,owner.epoch]).map_err(|_|recovery_required())?!=1{return Err(recovery_required());}
    mutation::finish_preserved_request(tx, &operation, &report)?;
    record::audit_connection(tx)
}

pub(super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    let bad:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_records c LEFT JOIN context_retirement_preservations p ON p.operation_ulid=c.operation_ulid WHERE (c.phase IN ('preserving','abandoned'))!=(p.operation_ulid IS NOT NULL)) OR EXISTS(SELECT 1 FROM context_retirement_preservations p JOIN context_retirement_records c ON c.operation_ulid=p.operation_ulid JOIN request_records r ON r.request_ulid=p.abandon_request_ulid JOIN operation_records o ON o.id=c.local_record_id WHERE c.batch_operation_ulid IS NOT NULL OR p.owner_epoch!=c.owner_epoch OR p.accepted_attempt>r.attempt OR r.command!='operation abandon' OR r.target!=c.operation_ulid OR r.scope_key!=o.root_path OR r.confirmation_ulid IS NULL OR EXISTS(SELECT 1 FROM request_operations m WHERE m.request_ulid=r.request_ulid) OR (c.phase='preserving' AND o.completed_step!='context_retirement_preserved') OR (c.phase='abandoned' AND o.completed_step!='abandoned') OR EXISTS(SELECT 1 FROM context_retirement_attempts a WHERE a.operation_ulid=c.operation_ulid AND a.phase='active'))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if bad {
        return Err(recovery_required());
    }
    let claims_bad:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM context_retirement_preservation_claims i LEFT JOIN context_retirement_preservations p ON p.operation_ulid=i.operation_ulid LEFT JOIN context_retirement_records c ON c.operation_ulid=p.operation_ulid LEFT JOIN operation_records o ON o.id=c.local_record_id LEFT JOIN request_records r ON r.request_ulid=p.abandon_request_ulid WHERE p.operation_ulid IS NULL OR c.operation_ulid IS NULL OR r.request_ulid IS NULL OR i.request_attempt<p.accepted_attempt OR i.request_attempt>r.attempt OR i.index_owner_epoch>o.index_owner_epoch OR i.index_owner_epoch<=json_extract(p.snapshot,'$.index_owner_epoch') OR EXISTS(SELECT 1 FROM context_retirement_preservation_claims old WHERE old.operation_ulid=i.operation_ulid AND old.request_attempt<i.request_attempt AND old.index_owner_epoch>=i.index_owner_epoch)) OR EXISTS(SELECT 1 FROM context_retirement_preservations p JOIN context_retirement_records c ON c.operation_ulid=p.operation_ulid JOIN operation_records o ON o.id=c.local_record_id WHERE (c.phase='abandoned' OR o.index_owner_epoch>json_extract(p.snapshot,'$.index_owner_epoch')) AND NOT EXISTS(SELECT 1 FROM context_retirement_preservation_claims i WHERE i.operation_ulid=p.operation_ulid AND i.index_owner_epoch=CASE WHEN o.state='index_invalidated' THEN o.index_owner_epoch-1 ELSE o.index_owner_epoch END))",[],|r|r.get(0)).map_err(|_|recovery_required())?;
    if claims_bad {
        return Err(recovery_required());
    }
    let mut q=db.prepare("SELECT p.snapshot,c.owner_epoch,c.request_ulid,c.worktree_removal,c.branch_removal FROM context_retirement_preservations p JOIN context_retirement_records c ON c.operation_ulid=p.operation_ulid").map_err(|_|recovery_required())?;
    for row in q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(|_| recovery_required())?
    {
        let (json, epoch, request, w, b) = row.map_err(|_| recovery_required())?;
        let s: RetirementAbandonMetadata =
            serde_json::from_str(&json).map_err(|_| recovery_required())?;
        if !s.index_pending
            || !matches!(
                s.phase.as_str(),
                "selected"
                    | "worktree_removing"
                    | "worktree_removed"
                    | "branch_deleting"
                    | "local_absent"
                    | "index_pending"
            )
            || s.owner_epoch < 1
            || s.index_owner_epoch < 0
        {
            return Err(recovery_required());
        }
        if s.owner_epoch != epoch
            || s.request_id != request
            || s.worktree_removal != removal(&w)?
            || s.branch_removal != removal(&b)?
        {
            return Err(recovery_required());
        }
    }
    Ok(())
}
