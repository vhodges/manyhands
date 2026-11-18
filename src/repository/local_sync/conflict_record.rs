//! Content-free conflict installation evidence; audited in production as well.
use crate::repository::*;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const MARKER: &str = "local_context_sync_conflicts_v7";
const SCHEMA: &str = "CREATE TABLE local_context_sync_conflicts (
 operation_ulid TEXT PRIMARY KEY NOT NULL REFERENCES local_context_sync_records(operation_ulid) ON DELETE CASCADE,
 generation INTEGER NOT NULL CHECK(typeof(generation)='integer' AND generation>0),
 manifest BLOB CHECK(manifest IS NULL OR (typeof(manifest)='blob' AND length(manifest)>0))
)";
const CONTROL: &str = "CREATE TABLE local_context_sync_resolution_cancellation (
 operation_ulid TEXT PRIMARY KEY NOT NULL REFERENCES local_context_sync_records(operation_ulid) ON DELETE CASCADE,
 attempt_ulid TEXT NOT NULL UNIQUE,
 issued INTEGER NOT NULL DEFAULT 0 CHECK(typeof(issued)='integer' AND issued>=0),
 acknowledged INTEGER NOT NULL DEFAULT 0 CHECK(typeof(acknowledged)='integer' AND acknowledged>=0 AND acknowledged<=issued)
)";

type PathImage = (Vec<u8>, Option<[u8; 32]>, Option<u32>);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Image {
    pub index: Vec<(Vec<u8>, u32, String, u16)>,
    pub paths: Vec<PathImage>,
    pub metadata: [Option<[u8; 32]>; 3],
}
fn failure() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::Read,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "local conflict authority is unavailable",
    )
}
pub(super) fn migrate(db: &Connection) -> Result<(), RepositoryError> {
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if !marked {
        if recovery::has_table(db, "local_context_sync_conflicts").map_err(|_| failure())? {
            return Err(failure());
        }
        db.execute_batch(SCHEMA).map_err(|_| failure())?;
        db.execute_batch(CONTROL).map_err(|_| failure())?;
        db.execute("INSERT INTO registry_migrations(name) VALUES(?1)", [MARKER])
            .map_err(|_| failure())?;
    }
    audit(db)
}
pub(super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    let marked: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MARKER],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if !marked {
        return Err(failure());
    }
    let missing:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_records p WHERE p.execution_required=1 AND (p.state IN ('conflict_pending','resolution_prepared') OR EXISTS(SELECT 1 FROM local_context_sync_steps s WHERE s.operation_ulid=p.operation_ulid AND s.kind='conflict_install')) AND NOT EXISTS(SELECT 1 FROM local_context_sync_conflicts c WHERE c.operation_ulid=p.operation_ulid)) OR EXISTS(SELECT 1 FROM local_context_sync_conflicts c JOIN local_context_sync_records p ON p.operation_ulid=c.operation_ulid WHERE p.active_resolution IS NOT NULL AND NOT EXISTS(SELECT 1 FROM local_context_sync_resolution_cancellation k WHERE k.operation_ulid=p.operation_ulid AND k.attempt_ulid=p.active_resolution))",[],|r|r.get(0)).map_err(|_|failure())?;
    if missing {
        return Err(failure());
    }
    let control: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='local_context_sync_resolution_cancellation'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if control.split_whitespace().collect::<String>()
        != CONTROL.split_whitespace().collect::<String>()
    {
        return Err(failure());
    }
    if db
        .prepare("PRAGMA foreign_key_check(local_context_sync_resolution_cancellation)")
        .and_then(|mut s| s.exists([]))
        .map_err(|_| failure())?
    {
        return Err(failure());
    }
    let invalid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_resolution_cancellation WHERE typeof(issued)!='integer' OR typeof(acknowledged)!='integer' OR issued<0 OR acknowledged<0 OR acknowledged>issued)",[],|r|r.get(0)).map_err(|_|failure())?;
    if invalid {
        return Err(failure());
    }
    let orphan:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_resolution_cancellation c LEFT JOIN local_context_sync_resolutions a ON a.attempt_ulid=c.attempt_ulid AND a.operation_ulid=c.operation_ulid WHERE a.attempt_ulid IS NULL)",[],|r|r.get(0)).map_err(|_|failure())?;
    if orphan {
        return Err(failure());
    }
    let sql: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='local_context_sync_conflicts'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if sql.split_whitespace().collect::<String>() != SCHEMA.split_whitespace().collect::<String>() {
        return Err(failure());
    }
    if db
        .prepare("PRAGMA foreign_key_check(local_context_sync_conflicts)")
        .and_then(|mut s| s.exists([]))
        .map_err(|_| failure())?
    {
        return Err(failure());
    }
    let mut statement = db.prepare("SELECT c.generation,p.generation,c.manifest FROM local_context_sync_conflicts c JOIN local_context_sync_records p ON p.operation_ulid=c.operation_ulid").map_err(|_| failure())?;
    let mut rows = statement.query([]).map_err(|_| failure())?;
    while let Some(row) = rows.next().map_err(|_| failure())? {
        let generation: i64 = row.get(0).map_err(|_| failure())?;
        if generation <= 0 || generation != row.get::<_, i64>(1).map_err(|_| failure())? {
            return Err(failure());
        }
        if let Some(bytes) = row.get::<_, Option<Vec<u8>>>(2).map_err(|_| failure())? {
            let image: Image = serde_json::from_slice(&bytes).map_err(|_| failure())?;
            if image.index.is_empty()
                || image.paths.is_empty()
                || image.metadata[0].is_none()
                || image.index.iter().any(|(_, mode, oid, stage)| {
                    *stage > 3 || *mode == 0 || git2::Oid::from_str(oid).is_err()
                })
                || image.paths.windows(2).any(|p| p[0].0 >= p[1].0)
            {
                return Err(failure());
            }
        }
    }
    Ok(())
}
