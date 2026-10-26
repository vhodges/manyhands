//! Stable request identity for local synchronization, independent of its current
//! executor attempt. Acceptance and handover use the caller's F2 transaction.
use super::records::NewRequest;
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
 resolution_attempt TEXT REFERENCES local_context_sync_resolutions(attempt_ulid),
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

pub(super) fn audit(db: &Connection) -> Result<(), ReadError> {
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
         OR EXISTS(SELECT 1 FROM request_records r WHERE r.command IN ('context sync-local','context resolve-local') AND NOT EXISTS(SELECT 1 FROM local_context_sync_request_control c WHERE c.request_ulid=r.request_ulid))",
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
    let command: String = db.query_row(
        "SELECT command FROM request_records WHERE request_ulid=?1",
        [id],
        |r| r.get(0),
    )?;
    let old: Option<i64> = db.query_row("SELECT execution_attempt FROM local_context_sync_request_control WHERE request_ulid=?1",[id],|r|r.get(0)).optional()?;
    match old {
        Some(old) if old.checked_add(1) == Some(next) => {
            if db.execute("UPDATE local_context_sync_request_control SET execution_attempt=?2 WHERE request_ulid=?1 AND execution_attempt=?3",params![id,next,old])? != 1 { return Err(unavailable()); }
            audit(db)
        }
        Some(_) => Err(unavailable()),
        None if matches!(
            command.as_str(),
            "context sync-local" | "context resolve-local"
        ) =>
        {
            Err(unavailable())
        }
        None => Ok(()),
    }
}
