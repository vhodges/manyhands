//! Stable request identity for local synchronization, independent of its current
//! executor attempt. Acceptance and handover use the caller's F2 transaction.
use super::identity::{RequestId, ScopeKey};
use super::records::NewRequest;
use crate::repository::RepositoryService;
use crate::{
    repository::{
        OperationFamily, OperationId, ReadError, RepositoryError, RepositoryErrorKind,
        RepositoryOperation,
    },
    results::ResultCode,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

const TABLE: &str = "local_context_sync_request_control";
const MARKER: &str = "local_context_sync_request_control_v1";
const SCHEMA: &str = "CREATE TABLE local_context_sync_request_control (
 request_ulid TEXT PRIMARY KEY NOT NULL REFERENCES request_records(request_ulid) ON DELETE CASCADE,
 intended_parent TEXT NOT NULL,
 scope_key TEXT NOT NULL,
 role TEXT NOT NULL CHECK(role IN ('start','resolve','resume')),
 execution_attempt INTEGER NOT NULL CHECK(typeof(execution_attempt)='integer' AND execution_attempt>0),
 parent_operation TEXT REFERENCES local_context_sync_records(operation_ulid),
 resolution_attempt TEXT,
 cancel_issued INTEGER NOT NULL DEFAULT 0 CHECK(typeof(cancel_issued)='integer' AND cancel_issued>=0),
 cancel_acknowledged INTEGER NOT NULL DEFAULT 0 CHECK(typeof(cancel_acknowledged)='integer' AND cancel_acknowledged>=0 AND cancel_acknowledged<=cancel_issued),
 effect_write TEXT NOT NULL DEFAULT 'not_requested' CHECK(effect_write IN ('not_requested','unchanged','written')),
 effect_checkpoint TEXT NOT NULL DEFAULT 'not_requested' CHECK(effect_checkpoint IN ('not_requested','unchanged','pending','committed')),
 effect_discovery TEXT NOT NULL DEFAULT 'not_requested' CHECK(effect_discovery IN ('not_requested','pending','current')),
 effect_integration TEXT NOT NULL DEFAULT 'not_requested' CHECK(effect_integration IN ('not_requested','pending','complete')),
 commit_oid TEXT CHECK(commit_oid IS NULL OR (length(commit_oid)=40 AND commit_oid NOT GLOB '*[^0-9a-f]*')),
 owner_epoch INTEGER CHECK(owner_epoch IS NULL OR (typeof(owner_epoch)='integer' AND owner_epoch>0)),
 CHECK(parent_operation IS NULL OR parent_operation=intended_parent),
 CHECK(resolution_attempt IS NULL OR (parent_operation IS NOT NULL AND role='resolve')),
 CHECK(owner_epoch IS NULL OR parent_operation IS NOT NULL)
)";

fn unavailable() -> ReadError {
    ReadError::new(ResultCode::RecoveryRequired)
}
fn migration_error() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::OpenRegistry,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "local request authority is unavailable",
    )
}

/// This runs after both the domain and F2 tables exist, in one migration
/// transaction. A marked but missing table is authority loss, not an upgrade.
pub(super) fn migrate(db: &Transaction<'_>) -> Result<(), RepositoryError> {
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| migration_error())?;
    if !marked {
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
                [TABLE],
                |r| r.get(0),
            )
            .map_err(|_| migration_error())?;
        if exists {
            return Err(migration_error());
        }
        db.execute_batch(SCHEMA).map_err(|_| migration_error())?;
        db.execute("INSERT INTO registry_migrations(name) VALUES(?1)", [MARKER])
            .map_err(|_| migration_error())?;
    }
    audit(db).map_err(|_| migration_error())
}

pub(super) fn schema_current(db: &Connection) -> Result<bool, RepositoryError> {
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| migration_error())?;
    if marked {
        audit(db).map_err(|_| migration_error())?;
    }
    Ok(marked)
}

pub(in super::super) fn audit(db: &Connection) -> Result<(), ReadError> {
    let sql: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name=?1",
            [TABLE],
            |r| r.get(0),
        )
        .map_err(|_| unavailable())?;
    if sql.split_whitespace().collect::<String>() != SCHEMA.split_whitespace().collect::<String>() {
        return Err(unavailable());
    }
    if db
        .prepare("PRAGMA foreign_key_check(local_context_sync_request_control)")?
        .exists([])?
    {
        return Err(unavailable());
    }
    let invalid: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM local_context_sync_request_control c JOIN request_records r USING(request_ulid)
         WHERE c.scope_key!=r.scope_key OR c.execution_attempt!=r.attempt
         OR typeof(c.execution_attempt)!='integer' OR c.execution_attempt<1
         OR typeof(c.cancel_issued)!='integer' OR typeof(c.cancel_acknowledged)!='integer'
         OR c.cancel_acknowledged<0 OR c.cancel_issued<c.cancel_acknowledged
         OR (c.role='start' AND (r.command!='context sync-local' OR NOT EXISTS(SELECT 1 FROM request_operations o WHERE o.request_ulid=c.request_ulid AND o.family='local' AND o.operation_ulid=c.intended_parent)))
         OR (c.role='resolve' AND (r.command!='context resolve-local' OR r.target!=c.intended_parent))
         OR (c.role='resume' AND (r.command!='operation resume' OR r.target!=c.intended_parent))
         OR (c.parent_operation IS NOT NULL AND (c.parent_operation!=c.intended_parent OR NOT EXISTS(SELECT 1 FROM local_context_sync_records p JOIN repositories repo ON repo.id=p.repository_id WHERE p.operation_ulid=c.parent_operation AND repo.root_path=c.scope_key)))
         OR (c.resolution_attempt IS NOT NULL AND NOT EXISTS(SELECT 1 FROM local_context_sync_resolutions a WHERE a.attempt_ulid=c.resolution_attempt AND a.operation_ulid=c.parent_operation AND a.request_ulid=c.request_ulid)))
         OR EXISTS(SELECT 1 FROM request_records r WHERE (r.command IN ('context sync-local','context resolve-local') OR (r.command='operation resume' AND EXISTS(SELECT 1 FROM operation_records p WHERE p.operation_ulid=r.target AND p.root_path=r.scope_key AND p.action='context_sync_local'))) AND NOT EXISTS(SELECT 1 FROM local_context_sync_request_control c WHERE c.request_ulid=r.request_ulid))",
        [], |r| r.get(0))?;
    if invalid {
        return Err(unavailable());
    }
    let mut ids = db.prepare("SELECT intended_parent FROM local_context_sync_request_control")?;
    let mut rows = ids.query([])?;
    while let Some(row) = rows.next()? {
        OperationId::parse(&row.get::<_, String>(0)?).map_err(|_| unavailable())?;
    }
    Ok(())
}

pub(super) fn accept(db: &Transaction<'_>, request: &NewRequest) -> Result<(), ReadError> {
    let (role, parent) = match request.command.as_str() {
        "context sync-local" => {
            let [operation] = request.operations.as_slice() else {
                return Err(unavailable());
            };
            if operation.family != OperationFamily::Local {
                return Err(unavailable());
            }
            ("start", operation.operation_id.to_string())
        }
        "context resolve-local" => {
            if !request.operations.is_empty() {
                return Err(unavailable());
            }
            OperationId::parse(&request.target).map_err(|_| unavailable())?;
            ("resolve", request.target.clone())
        }
        "operation resume" => {
            let local: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_records p JOIN repositories r ON r.id=p.repository_id WHERE p.operation_ulid=?1 AND r.root_path=?2)", params![request.target,request.scope.as_str()],|r|r.get(0))?;
            if !local {
                return Ok(());
            }
            if !request.operations.is_empty() {
                return Err(unavailable());
            }
            ("resume", request.target.clone())
        }
        _ => return Ok(()),
    };
    if request.confirmation.is_some() {
        return Err(unavailable());
    }
    if db.execute("INSERT INTO local_context_sync_request_control(request_ulid,intended_parent,scope_key,role,execution_attempt) VALUES(?1,?2,?3,?4,1)",params![request.request_id.to_string(),parent,request.scope.as_str(),role])? != 1 { return Err(unavailable()); }
    audit(db)
}

/// Called after F2 raises its fence, but before the same transaction commits.
/// Never replace a missing row or treat an older control as absent evidence.
pub(super) fn enter(db: &Transaction<'_>, id: &str, next: i64) -> Result<(), ReadError> {
    let old: Option<i64> = db.query_row("SELECT execution_attempt FROM local_context_sync_request_control WHERE request_ulid=?1",[id],|r|r.get(0)).optional()?;
    match old {
        Some(old) if old.checked_add(1) == Some(next) => {
            if db.execute("UPDATE local_context_sync_request_control SET execution_attempt=?2 WHERE request_ulid=?1 AND execution_attempt=?3",params![id,next,old])? != 1 { return Err(unavailable()); }
            audit(db)
        }
        Some(_) => Err(unavailable()),
        None if requires_control(db, id)? => Err(unavailable()),
        None => Ok(()),
    }
}

fn requires_control(db: &Connection, id: &str) -> Result<bool, ReadError> {
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM request_records r WHERE r.request_ulid=?1 AND (r.command IN ('context sync-local','context resolve-local') OR (r.command='operation resume' AND EXISTS(SELECT 1 FROM operation_records p WHERE p.operation_ulid=r.target AND p.root_path=r.scope_key AND p.action='context_sync_local'))))",[id],|r|r.get(0))?)
}

pub(super) fn starting_request(
    service: &RepositoryService,
    scope: &ScopeKey,
    parent: OperationId,
) -> Result<Option<RequestId>, ReadError> {
    service.read_session(RepositoryOperation::Read,|db|{
        let id:Option<String>=db.query_row("SELECT request_ulid FROM local_context_sync_request_control WHERE intended_parent=?1 AND scope_key=?2 AND role='start'",params![parent.to_string(),scope.as_str()],|r|r.get(0)).optional()?;
        id.map(|id|RequestId::parse(&id).map_err(|_|unavailable())).transpose()
    })
}
pub(super) fn linked(service: &RepositoryService, id: RequestId) -> Result<bool, ReadError> {
    service.read_session(RepositoryOperation::Read,|db|Ok(db.query_row("SELECT parent_operation IS NOT NULL FROM local_context_sync_request_control WHERE request_ulid=?1",[id.to_string()],|r|r.get(0))?))
}

pub(super) fn journal(
    service: &RepositoryService,
    id: RequestId,
) -> Result<Option<crate::repository::recovery::JournalRow>, ReadError> {
    use crate::repository::recovery::{JournalRow, PendingOperation};
    service.read_session(RepositoryOperation::Read,|db|{
        let control:Option<(String,Option<String>)>=db.query_row("SELECT scope_key,parent_operation FROM local_context_sync_request_control WHERE request_ulid=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        match control {
            Some((scope,Some(parent)))=>crate::repository::recovery::lookup_operation(db,&scope,OperationId::parse(&parent).map_err(|_|unavailable())?).map(Some),
            Some((_,None))=>Ok(Some(JournalRow::Pending(PendingOperation::Local{state:"created".into(),step:None}))),
            None=>{
                let required=requires_control(db,&id.to_string())?;
                if required {Err(unavailable())} else {Ok(None)}
            }
        }
    })
}
pub(super) fn effects(
    service: &RepositoryService,
    id: RequestId,
) -> Result<Option<crate::results::Effects>, ReadError> {
    use crate::results::*;
    let mut found=service.read_session(RepositoryOperation::Read,|db|{
        let row:Option<(String,String,String,String,Option<String>)>=db.query_row("SELECT effect_write,effect_checkpoint,effect_discovery,effect_integration,commit_oid FROM local_context_sync_request_control WHERE request_ulid=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
        row.map(|(w,c,d,i,oid)|Ok(Effects{write:WriteEffect::ALL.into_iter().find(|v|v.as_str()==w).ok_or_else(unavailable)?,checkpoint:CheckpointEffect::ALL.into_iter().find(|v|v.as_str()==c).ok_or_else(unavailable)?,discovery:DiscoveryEffect::ALL.into_iter().find(|v|v.as_str()==d).ok_or_else(unavailable)?,integration:IntegrationEffect::ALL.into_iter().find(|v|v.as_str()==i).ok_or_else(unavailable)?,commit_oid:oid,..Effects::not_requested()})).transpose()
    })?;
    if let Some(effects) = found.as_mut()
        && effects.write != WriteEffect::Written
        && crate::repository::local_sync::resolution_written(service, id).unwrap_or(false)
    {
        effects.write = WriteEffect::Written;
    }
    Ok(found)
}
pub(super) fn may_settle(
    db: &Connection,
    id: RequestId,
    attempt: u64,
    deleting: bool,
) -> Result<bool, ReadError> {
    let control:Option<(Option<String>,Option<i64>,u64)>=db.query_row("SELECT parent_operation,owner_epoch,execution_attempt FROM local_context_sync_request_control WHERE request_ulid=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
    let Some((parent, epoch, current)) = control else {
        let required = requires_control(db, &id.to_string())?;
        return if required {
            Err(unavailable())
        } else {
            Ok(true)
        };
    };
    if current != attempt {
        return Ok(false);
    }
    let Some(parent) = parent else {
        return Ok(true);
    };
    if deleting {
        return Ok(false);
    }
    let current: Option<i64> = db
        .query_row(
            "SELECT owner_epoch FROM local_context_sync_records WHERE operation_ulid=?1",
            [parent],
            |r| r.get(0),
        )
        .optional()?;
    Ok(epoch.is_some() && epoch == current)
}
pub(super) fn counters(db: &Connection, id: RequestId) -> Result<Option<(i64, i64)>, ReadError> {
    Ok(db.query_row("SELECT cancel_issued,cancel_acknowledged FROM local_context_sync_request_control WHERE request_ulid=?1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?)
}

pub(in super::super) fn abandon_requests(
    db: &Transaction<'_>,
    repo: &crate::repository::ResolvedRepository,
    row: &crate::repository::local_sync::record::LocalContextSyncRecord,
    now: i64,
) -> Result<(), ReadError> {
    use crate::{repository::*, results::*};
    audit(db)?;
    local_sync::authority_controls(db, repo.root(), row.operation_id)?;
    let requests:Vec<(String,String,String,String,String,Option<String>)>=db.prepare("SELECT c.request_ulid,c.effect_write,c.effect_checkpoint,c.effect_discovery,c.effect_integration,c.commit_oid FROM local_context_sync_request_control c JOIN request_records r USING(request_ulid) WHERE c.intended_parent=?1 AND c.scope_key=?2 AND r.state='accepted' ORDER BY c.request_ulid")?.query_map(params![row.operation_id.to_string(),repo.root().to_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?.collect::<Result<_,_>>()?;
    for (id, w, c, d, i, oid) in requests {
        let effects = Effects {
            write: WriteEffect::ALL
                .into_iter()
                .find(|v| v.as_str() == w)
                .ok_or_else(unavailable)?,
            checkpoint: CheckpointEffect::ALL
                .into_iter()
                .find(|v| v.as_str() == c)
                .ok_or_else(unavailable)?,
            discovery: DiscoveryEffect::ALL
                .into_iter()
                .find(|v| v.as_str() == d)
                .ok_or_else(unavailable)?,
            integration: IntegrationEffect::ALL
                .into_iter()
                .find(|v| v.as_str() == i)
                .ok_or_else(unavailable)?,
            commit_oid: oid.clone(),
            ..Effects::not_requested()
        };
        let resolution:Option<String>=db.query_row("SELECT CASE WHEN role='resolve' THEN request_ulid ELSE NULL END FROM local_context_sync_request_control WHERE request_ulid=?1",[&id],|r|r.get(0))?;
        let data = LocalContextSyncMutationDto {
            data_type: LocalContextSyncDataType::LocalContextSync,
            item_id: row.item_id.to_string(),
            kind: authoring_kind_segment(&row.kind).into(),
            operation_id: Some(row.operation_id.to_string()),
            resolution_request_id: resolution,
            required_request_id: None,
            result_oid: oid.clone(),
            state: LocalContextSyncState::Abandoned,
            reason: LocalContextSyncReason::Abandoned,
            next_action: LocalContextSyncNextAction::None,
            eligibility: None,
            cancel_acknowledged: true,
        };
        let data = serde_json::to_string(&data).map_err(|_| unavailable())?;
        let outcome = Outcome::of_mutation(true, ResultCode::Cancelled, &effects, false);
        if db.execute("UPDATE request_records SET state='finished',outcome=?2,code='cancelled',effect_write=?3,effect_checkpoint=?4,effect_discovery=?5,effect_publication='not_requested',effect_integration=?6,effect_cleanup='not_requested',commit_oid=?7,result_data=?8,finished_at=?9 WHERE request_ulid=?1 AND state='accepted'",params![id,outcome.as_str(),w,c,d,i,oid,data,now])?!=1{return Err(unavailable());}
    }
    Ok(())
}
