use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;

use super::{
    OperationId, RecoveryInspection, RepositoryError, RepositoryErrorKind, RepositoryOperation,
};

#[derive(Clone, Copy)]
pub(super) struct RecoveryRecord {
    pub(super) id: i64,
}

pub(super) fn migrate_operation_records(
    connection: &mut Connection,
) -> Result<(), RepositoryError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS operation_records (
            id INTEGER PRIMARY KEY,
            repository_id INTEGER REFERENCES repositories(id) ON DELETE SET NULL,
            root_path TEXT NOT NULL,
            operation_ulid TEXT,
            action TEXT NOT NULL,
            target TEXT,
            item_id TEXT,
            context_path TEXT,
            state TEXT NOT NULL,
            completed_step TEXT,
            observed_at INTEGER NOT NULL,
            persisted_context_count INTEGER NOT NULL DEFAULT 0,
            observation_fingerprint TEXT,
            git_head_oid TEXT,
            git_reference TEXT,
            git_observed_at INTEGER,
            redacted_error TEXT
        );
        CREATE INDEX IF NOT EXISTS operation_records_root_path_idx ON operation_records(root_path, observed_at);
        CREATE UNIQUE INDEX IF NOT EXISTS operation_records_root_operation_ulid_idx
            ON operation_records(root_path, operation_ulid) WHERE operation_ulid IS NOT NULL;
        CREATE TABLE IF NOT EXISTS registry_migrations (name TEXT PRIMARY KEY);
        ",
    ).map_err(RepositoryError::sqlite)?;
    let transaction = connection.transaction().map_err(RepositoryError::sqlite)?;
    let migrated: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name = 'cycle_05_operation_records')",
        [],
        |row| row.get(0),
    ).map_err(RepositoryError::sqlite)?;
    if !migrated {
        transaction
            .execute(
                "INSERT INTO operation_records (
                repository_id, root_path, operation_ulid, action, context_path, state,
                completed_step, observed_at, persisted_context_count, observation_fingerprint
             )
             SELECT index_operations.repository_id, repositories.root_path, NULL,
                    index_operations.operation, index_operations.context_path,
                    index_operations.state, index_operations.state,
                    index_operations.observed_at, index_operations.persisted_context_count,
                    (SELECT observation_fingerprint FROM index_operation_contexts
                     WHERE index_operation_contexts.operation_id = index_operations.id
                     ORDER BY worktree_path LIMIT 1)
             FROM index_operations
             JOIN repositories ON repositories.id = index_operations.repository_id
             WHERE index_operations.operation IN ('refresh', 'rebuild')",
                [],
            )
            .map_err(RepositoryError::sqlite)?;
        transaction
            .execute(
                "INSERT INTO registry_migrations (name) VALUES ('cycle_05_operation_records')",
                [],
            )
            .map_err(RepositoryError::sqlite)?;
    }
    transaction.commit().map_err(RepositoryError::sqlite)
}

pub(super) fn begin_or_reconcile_operation(
    connection: &Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
) -> Result<RecoveryRecord, RepositoryError> {
    let root_path = root.to_str().ok_or_else(|| invalid_path(operation, root))?;
    let action = action_name(operation);
    let requested = operation_id.to_string();
    if let Some((id, existing_root, existing_action)) = connection
        .query_row(
            "SELECT id, root_path, action FROM operation_records WHERE operation_ulid = ?1",
            [&requested],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(RepositoryError::sqlite)?
    {
        if existing_root != root_path || existing_action != action {
            return Err(mismatch(operation, root));
        }
        return record(connection, id);
    }
    if let Some((id, existing_action)) = connection
        .query_row(
            "SELECT id, action FROM operation_records
         WHERE root_path = ?1 AND state != 'completed' ORDER BY id DESC LIMIT 1",
            [root_path],
            |row| Ok((row.get(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(RepositoryError::sqlite)?
    {
        if existing_action == action {
            let legacy: Option<String> = connection
                .query_row(
                    "SELECT operation_ulid FROM operation_records WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .map_err(RepositoryError::sqlite)?;
            if legacy.is_none() {
                return record(connection, id);
            }
        }
        return Err(recovery_required(operation, root));
    }
    let repository_id: Option<i64> = connection
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(RepositoryError::sqlite)?;
    connection.execute(
        "INSERT INTO operation_records (repository_id, root_path, operation_ulid, action, state, observed_at)
         VALUES (?1, ?2, ?3, ?4, 'created', ?5)",
        params![repository_id, root_path, requested, action, now()],
    ).map_err(RepositoryError::sqlite)?;
    record(connection, connection.last_insert_rowid())
}

pub(super) fn advance_after_observation(
    connection: &Connection,
    record_id: i64,
    state: &str,
    context: Option<&Path>,
    persisted_context_count: Option<i64>,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "UPDATE operation_records
         SET state = ?2, completed_step = ?2, context_path = ?3,
             persisted_context_count = COALESCE(?4, persisted_context_count), observed_at = ?5
         WHERE id = ?1",
            params![
                record_id,
                state,
                context.and_then(Path::to_str),
                persisted_context_count,
                now()
            ],
        )
        .map_err(RepositoryError::sqlite)?;
    Ok(())
}

pub(super) fn record_persisted_context(
    connection: &Connection,
    record_id: i64,
    context: &Path,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "UPDATE operation_records
         SET state = 'persisted', completed_step = 'persisted', context_path = ?2,
             persisted_context_count = persisted_context_count + 1, observed_at = ?3
         WHERE id = ?1",
            params![record_id, context.to_str(), now()],
        )
        .map_err(RepositoryError::sqlite)?;
    Ok(())
}

pub(super) fn pending_for_root(
    connection: &Connection,
    root: &Path,
) -> Result<Vec<RecoveryInspection>, RepositoryError> {
    let root_path = root
        .to_str()
        .ok_or_else(|| invalid_path(RepositoryOperation::Inspect, root))?;
    let mut statement = connection
        .prepare(
            "SELECT operation_ulid, action, item_id, context_path, completed_step
         FROM operation_records WHERE root_path = ?1 AND state != 'completed' ORDER BY id",
        )
        .map_err(RepositoryError::sqlite)?;
    statement
        .query_map([root_path], |row| {
            let operation_id: Option<String> = row.get(0)?;
            let action: String = row.get(1)?;
            let item_id: Option<String> = row.get(2)?;
            let context: Option<String> = row.get(3)?;
            let completed_step: Option<String> = row.get(4)?;
            Ok((operation_id, action, item_id, context, completed_step))
        })
        .map_err(RepositoryError::sqlite)?
        .map(|row| {
            let (operation_id, action, item_id, context, completed_step) =
                row.map_err(RepositoryError::sqlite)?;
            let operation = operation_from_name(&action)
                .ok_or_else(|| recovery_required(RepositoryOperation::Inspect, root))?;
            Ok(match operation_id {
                Some(operation_id) => RecoveryInspection::Pending {
                    operation_id: OperationId::parse(&operation_id)
                        .map_err(|_| recovery_required(RepositoryOperation::Inspect, root))?,
                    operation,
                    root: root.to_owned(),
                    item_id: item_id.and_then(|id| id.parse().ok()),
                    context: context.map(PathBuf::from),
                    completed_step,
                    next_action: operation,
                },
                None => RecoveryInspection::LegacyIndexOperation {
                    root: root.to_owned(),
                    operation,
                    next_action: operation,
                },
            })
        })
        .collect()
}

pub(super) fn clear_root(connection: &Connection, root: &Path) -> Result<(), RepositoryError> {
    let root_path = root
        .to_str()
        .ok_or_else(|| invalid_path(RepositoryOperation::RemoveRegistration, root))?;
    connection
        .execute(
            "DELETE FROM operation_records WHERE root_path = ?1",
            [root_path],
        )
        .map_err(RepositoryError::sqlite)?;
    Ok(())
}

pub(super) fn has_incomplete_rebuild(connection: &Connection) -> Result<bool, RepositoryError> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM operation_records WHERE action = 'rebuild' AND state != 'completed')",
        [], |row| row.get(0),
    ).map_err(RepositoryError::sqlite)
}

fn record(connection: &Connection, id: i64) -> Result<RecoveryRecord, RepositoryError> {
    connection
        .query_row(
            "SELECT id, repository_id FROM operation_records WHERE id = ?1",
            [id],
            |row| Ok(RecoveryRecord { id: row.get(0)? }),
        )
        .map_err(RepositoryError::sqlite)
}

fn action_name(operation: RepositoryOperation) -> &'static str {
    match operation {
        RepositoryOperation::RefreshRepository => "refresh",
        RepositoryOperation::RebuildRepository => "rebuild",
        _ => "other",
    }
}

fn operation_from_name(action: &str) -> Option<RepositoryOperation> {
    match action {
        "refresh" => Some(RepositoryOperation::RefreshRepository),
        "rebuild" => Some(RepositoryOperation::RebuildRepository),
        _ => None,
    }
}

fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn invalid_path(operation: RepositoryOperation, root: &Path) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(root.to_owned()),
        RepositoryErrorKind::InvalidPath,
        "the canonical root path is not valid UTF-8",
    )
}

fn mismatch(operation: RepositoryOperation, root: &Path) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(root.to_owned()),
        RepositoryErrorKind::OperationMismatch,
        "the operation ID is already bound to a different root or action",
    )
}

fn recovery_required(operation: RepositoryOperation, root: &Path) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(root.to_owned()),
        RepositoryErrorKind::RecoveryRequired,
        "an unresolved operation for this root must be resumed first",
    )
}
