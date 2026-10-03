use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use time::OffsetDateTime;

use super::{
    OperationId, RecoveryInspection, RepositoryError, RepositoryErrorKind, RepositoryOperation,
};

#[derive(Clone, Copy)]
pub(super) struct RecoveryRecord {
    pub(super) id: i64,
}

#[cfg(test)]
static BEGIN_OPERATION_BARRIER: std::sync::OnceLock<std::sync::Mutex<Option<BeginOperationPause>>> =
    std::sync::OnceLock::new();

#[cfg(test)]
pub(super) struct BeginOperationBarrierGuard;

#[cfg(test)]
struct BeginOperationPause {
    operation_id: String,
    barrier: std::sync::Arc<std::sync::Barrier>,
}

#[cfg(test)]
pub(super) fn pause_before_begin_for_testing(
    operation_id: OperationId,
    barrier: std::sync::Arc<std::sync::Barrier>,
) -> BeginOperationBarrierGuard {
    let mut installed = BEGIN_OPERATION_BARRIER
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert!(
        installed
            .replace(BeginOperationPause {
                operation_id: operation_id.to_string(),
                barrier,
            })
            .is_none(),
        "test barrier is already installed"
    );
    BeginOperationBarrierGuard
}

#[cfg(test)]
impl Drop for BeginOperationBarrierGuard {
    fn drop(&mut self) {
        *BEGIN_OPERATION_BARRIER
            .get()
            .unwrap()
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
    }
}

#[cfg(test)]
fn pause_before_begin_for_testing_if_installed(operation_id: &str) {
    let barrier = BEGIN_OPERATION_BARRIER.get().and_then(|installed| {
        installed
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .filter(|pause| pause.operation_id == operation_id)
            .map(|pause| pause.barrier.clone())
    });
    if let Some(barrier) = barrier {
        barrier.wait();
    }
}

pub(super) fn migrate_operation_records(
    connection: &mut Connection,
) -> Result<(), RepositoryError> {
    let transaction = connection.transaction().map_err(RepositoryError::sqlite)?;
    transaction.execute_batch(
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
            redacted_error TEXT
        );
        CREATE INDEX IF NOT EXISTS operation_records_root_path_idx ON operation_records(root_path, observed_at);
         CREATE UNIQUE INDEX IF NOT EXISTS operation_records_root_operation_ulid_idx
             ON operation_records(root_path, operation_ulid) WHERE operation_ulid IS NOT NULL;
         CREATE TABLE IF NOT EXISTS operation_record_contexts (
             operation_record_id INTEGER NOT NULL REFERENCES operation_records(id) ON DELETE CASCADE,
             worktree_path TEXT NOT NULL,
             PRIMARY KEY (operation_record_id, worktree_path)
         );
         CREATE TABLE IF NOT EXISTS registry_migrations (name TEXT PRIMARY KEY);
        ",
    ).map_err(RepositoryError::sqlite)?;
    let has_legacy_operations: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'index_operations')",
            [],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)?;
    let has_legacy_contexts: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'index_operation_contexts')",
            [],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)?;
    let migrated: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name = 'cycle_05_operation_records')",
        [],
        |row| row.get(0),
    ).map_err(RepositoryError::sqlite)?;
    if !migrated && has_legacy_operations {
        let legacy_operations = transaction
            .prepare(
                "SELECT index_operations.id, index_operations.repository_id, repositories.root_path,
                        index_operations.operation, index_operations.state,
                        index_operations.observed_at, index_operations.persisted_context_count
                 FROM index_operations
                 JOIN repositories ON repositories.id = index_operations.repository_id
                 WHERE index_operations.operation IN ('refresh', 'rebuild')",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, i64>(5)?,
                            row.get::<_, i64>(6)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(RepositoryError::sqlite)?;
        for (legacy_id, repository_id, root_path, action, state, observed_at, persisted_count) in
            legacy_operations
        {
            let completed = state == "completed";
            transaction
                .execute(
                    "INSERT INTO operation_records (
                    repository_id, root_path, operation_ulid, action, state,
                    completed_step, observed_at, persisted_context_count
                 ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        repository_id,
                        root_path,
                        action,
                        if completed { "completed" } else { "created" },
                        completed.then_some("completed"),
                        observed_at,
                        if completed { persisted_count } else { 0 },
                    ],
                )
                .map_err(RepositoryError::sqlite)?;
            if has_legacy_contexts {
                transaction.execute(
                    "INSERT OR IGNORE INTO operation_record_contexts (operation_record_id, worktree_path)
                     SELECT ?1, worktree_path FROM index_operation_contexts WHERE operation_id = ?2",
                    params![transaction.last_insert_rowid(), legacy_id],
                ).map_err(RepositoryError::sqlite)?;
            }
        }
        transaction
            .execute_batch(
                "DROP TABLE IF EXISTS index_operation_contexts;
                 DROP TABLE index_operations;",
            )
            .map_err(RepositoryError::sqlite)?;
    }
    if !migrated {
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
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
) -> Result<RecoveryRecord, RepositoryError> {
    let root_path = root.to_str().ok_or_else(|| invalid_path(operation, root))?;
    let action = action_name(operation);
    let requested = operation_id.to_string();
    #[cfg(test)]
    pause_before_begin_for_testing_if_installed(&requested);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(RepositoryError::sqlite)?;
    let existing = transaction
        .query_row(
            "SELECT id, root_path, action, target, state FROM operation_records WHERE operation_ulid = ?1",
            [&requested],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(RepositoryError::sqlite)?;
    if let Some((_, existing_root, existing_action, existing_target, _)) = &existing {
        if existing_root != root_path {
            return Err(mismatch(operation, root));
        }
        if !same_lifecycle_action(existing_action, action)
            || existing_target.as_deref() != Some(target)
        {
            return Err(mismatch(operation, root));
        }
    }

    let pending = transaction
        .prepare(
            "SELECT id, operation_ulid, action, target FROM operation_records
             WHERE root_path = ?1 AND state != 'completed' ORDER BY id",
        )
        .and_then(|mut statement| {
            statement
                .query_map([root_path], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(RepositoryError::sqlite)?;
    if !pending.is_empty() {
        let mut matching = None;
        for (id, existing_id, existing_action, existing_target) in pending {
            if existing_id.as_deref() == Some(&requested)
                && same_lifecycle_action(&existing_action, action)
                && existing_target.as_deref() == Some(target)
                || existing_id.is_none()
                    && matches!(
                        (existing_action.as_str(), action),
                        ("refresh", "refresh") | ("rebuild", "rebuild")
                    )
            {
                matching = Some(id);
            } else {
                return Err(recovery_required(operation, root));
            }
        }
        if let Some(id) = matching {
            transaction.commit().map_err(RepositoryError::sqlite)?;
            return Ok(RecoveryRecord { id });
        }
        return Err(recovery_required(operation, root));
    }
    if let Some((id, ..)) = existing {
        transaction.commit().map_err(RepositoryError::sqlite)?;
        return Ok(RecoveryRecord { id });
    }
    let repository_id: Option<i64> = transaction
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(RepositoryError::sqlite)?;
    transaction
        .execute(
            "INSERT INTO operation_records (repository_id, root_path, operation_ulid, action, target, state, observed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'created', ?6)",
            params![repository_id, root_path, requested, action, target, now()],
        )
        .map_err(RepositoryError::sqlite)?;
    let id = transaction.last_insert_rowid();
    transaction.commit().map_err(RepositoryError::sqlite)?;
    Ok(RecoveryRecord { id })
}

fn same_lifecycle_action(existing: &str, requested: &str) -> bool {
    existing == requested || (existing == "create_and_enable" && requested == "enable")
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
          WHERE id = ?1 AND NOT (state = 'completed' AND ?2 = 'error')",
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

fn action_name(operation: RepositoryOperation) -> &'static str {
    match operation {
        RepositoryOperation::CreateAndEnable => "create_and_enable",
        RepositoryOperation::Enable => "enable",
        RepositoryOperation::RemoveRegistration => "remove_registration",
        RepositoryOperation::AddRemote => "add_remote",
        RepositoryOperation::RemoveRemote => "remove_remote",
        RepositoryOperation::SetPublicationRemote => "set_publication_remote",
        RepositoryOperation::RefreshRepository => "refresh",
        RepositoryOperation::RebuildRepository => "rebuild",
        RepositoryOperation::PrepareContext => "prepare_context",
        RepositoryOperation::SaveDocument => "save_document",
        RepositoryOperation::SaveTicket => "save_ticket",
        RepositoryOperation::SubmitComment => "submit_comment",
        _ => "other",
    }
}

fn operation_from_name(action: &str) -> Option<RepositoryOperation> {
    match action {
        "create_and_enable" => Some(RepositoryOperation::CreateAndEnable),
        "enable" => Some(RepositoryOperation::Enable),
        "remove_registration" => Some(RepositoryOperation::RemoveRegistration),
        "add_remote" => Some(RepositoryOperation::AddRemote),
        "remove_remote" => Some(RepositoryOperation::RemoveRemote),
        "set_publication_remote" => Some(RepositoryOperation::SetPublicationRemote),
        "refresh" => Some(RepositoryOperation::RefreshRepository),
        "rebuild" => Some(RepositoryOperation::RebuildRepository),
        "prepare_context" => Some(RepositoryOperation::PrepareContext),
        "save_document" => Some(RepositoryOperation::SaveDocument),
        "save_ticket" => Some(RepositoryOperation::SaveTicket),
        "submit_comment" => Some(RepositoryOperation::SubmitComment),
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
