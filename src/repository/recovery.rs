use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use time::OffsetDateTime;

use super::{
    OperationId, RecoveryInspection, RepositoryError, RepositoryErrorKind, RepositoryOperation,
};

#[derive(Clone, Copy)]
pub(super) struct RecoveryRecord {
    pub(super) id: i64,
    pub(super) is_new: bool,
    pub(super) is_pending: bool,
    #[allow(dead_code)]
    pub(super) completed_step: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub(super) struct IndexOwner {
    pub(super) record_id: i64,
    pub(super) epoch: i64,
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
        CREATE INDEX IF NOT EXISTS operation_records_pending_root ON operation_records(root_path) WHERE state != 'completed';
        CREATE INDEX IF NOT EXISTS operation_records_id_lookup ON operation_records(operation_ulid) WHERE operation_ulid IS NOT NULL;
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
    let has_owner_epoch = transaction
        .prepare("SELECT name FROM pragma_table_info('operation_records') WHERE name = 'index_owner_epoch'")
        .and_then(|mut statement| statement.exists([]))
        .map_err(RepositoryError::sqlite)?;
    if !has_owner_epoch {
        transaction.execute("ALTER TABLE operation_records ADD COLUMN index_owner_epoch INTEGER NOT NULL DEFAULT 0", []).map_err(RepositoryError::sqlite)?;
    }
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

/// Remote reservations cannot consume, replace or reinterpret local recovery.
pub(super) fn require_no_pending_local(
    connection: &Connection,
    repository_id: i64,
) -> Result<(), RepositoryError> {
    let pending: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE root_path=(SELECT root_path FROM repositories WHERE id=?1) AND state!='completed')",
        [repository_id], |row| row.get(0)).map_err(|_| super::remote::state::recovery_required())?;
    if pending {
        return Err(super::remote::state::recovery_required());
    }
    Ok(())
}

/// Tagged synchronization-local identity cannot share an ID with any remote row.
/// The collision check belongs in the insertion transaction, not prior inspection.
pub(super) fn begin_or_reconcile_local_synchronization(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
) -> Result<RecoveryRecord, RepositoryError> {
    begin_or_reconcile(connection, root, operation, operation_id, target, true)
}

pub(super) fn begin_or_reconcile_operation(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
) -> Result<RecoveryRecord, RepositoryError> {
    begin_or_reconcile(connection, root, operation, operation_id, target, false)
}

fn begin_or_reconcile(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
    tagged_local_binding: bool,
) -> Result<RecoveryRecord, RepositoryError> {
    let root_path = root.to_str().ok_or_else(|| invalid_path(operation, root))?;
    let action = action_name(operation);
    let requested = operation_id.to_string();
    #[cfg(test)]
    pause_before_begin_for_testing_if_installed(&requested);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(RepositoryError::sqlite)?;
    if tagged_local_binding {
        let remote_id: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1)",
                [&requested],
                |row| row.get(0),
            )
            .map_err(|_| super::remote::state::recovery_required())?;
        if remote_id {
            return Err(super::remote::state::recovery_required());
        }
    }
    // A context synchronization owns only its exact authoring kind/item. Other
    // contexts may continue authoring; the matching context is rejected again
    // before any owned write by RepositoryService's checkpoint guard.
    let remote_active: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_operation_records JOIN repositories ON repositories.id=remote_operation_records.repository_id WHERE repositories.root_path=?1 AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling') AND (action != 'synchronize_context' OR ?2 LIKE ('authoring-context-v1/' || kind || '/' || item_id || '/%')))",
        rusqlite::params![root_path, target], |row| row.get(0)).map_err(|_| super::remote::state::recovery_required())?;
    if remote_active {
        return Err(super::remote::state::recovery_required());
    }
    let existing = transaction
        .query_row(
            "SELECT id, root_path, action, target, state, completed_step FROM operation_records WHERE operation_ulid = ?1",
            [&requested],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(RepositoryError::sqlite)?;
    if let Some((_, existing_root, existing_action, existing_target, _, _)) = &existing {
        if existing_root != root_path {
            return Err(mismatch(operation, root));
        }
        // Tagged callers supply the complete typed identity + full Git OID.
        // Equality validates the whole frozen matcher, not refresh's aliases.
        if tagged_local_binding
            && (action != "refresh"
                || existing_action != action
                || existing_target.as_deref() != Some(target))
        {
            return Err(mismatch(operation, root));
        }
        if !(same_lifecycle_action(existing_action, action)
            || action == "refresh" && existing_action != "refresh")
            || action != "refresh" && existing_target.as_deref() != Some(target)
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
            let compatible = if tagged_local_binding {
                existing_id.as_deref() == Some(&requested)
                    && action == "refresh"
                    && existing_action == action
                    && existing_target.as_deref() == Some(target)
            } else {
                existing_id.as_deref() == Some(&requested)
                    && (same_lifecycle_action(&existing_action, action)
                        || action == "refresh" && existing_action != "refresh")
                    && (action == "refresh" || existing_target.as_deref() == Some(target))
                    || existing_id.is_none()
                        && matches!(
                            (existing_action.as_str(), action),
                            ("refresh", "refresh") | ("rebuild", "rebuild")
                        )
            };
            if compatible {
                matching = Some(id);
            } else {
                return Err(recovery_required(operation, root));
            }
        }
        if let Some(id) = matching {
            transaction.commit().map_err(RepositoryError::sqlite)?;
            return Ok(RecoveryRecord {
                id,
                is_new: false,
                is_pending: true,
                completed_step: existing
                    .as_ref()
                    .and_then(|(_, _, _, _, _, completed_step)| completed_step.as_deref())
                    .and_then(authoring_observation_step),
            });
        }
        return Err(recovery_required(operation, root));
    }
    if let Some((id, _, _, _, _, completed_step)) = existing {
        // A refresh may borrow any operation ID; it must not erase the marker.
        if completed_step.as_deref() == Some(REJECTED_STEP) && action != "refresh" {
            // The earlier call left nothing behind, so this one begins again.
            // Its row was kept so that the ID stays bound to its target.
            transaction
                .execute(
                    "UPDATE operation_records SET state = 'created', completed_step = NULL, observed_at = ?2 WHERE id = ?1",
                    params![id, now()],
                )
                .map_err(RepositoryError::sqlite)?;
            transaction.commit().map_err(RepositoryError::sqlite)?;
            return Ok(RecoveryRecord {
                id,
                is_new: true,
                is_pending: false,
                completed_step: None,
            });
        }
        transaction.commit().map_err(RepositoryError::sqlite)?;
        return Ok(RecoveryRecord {
            id,
            is_new: false,
            is_pending: false,
            completed_step: completed_step
                .as_deref()
                .and_then(authoring_observation_step),
        });
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
    Ok(RecoveryRecord {
        id,
        is_new: true,
        is_pending: false,
        completed_step: None,
    })
}

fn authoring_observation_step(step: &str) -> Option<&'static str> {
    match step {
        "authoring_destination_observed" => Some("authoring_destination_observed"),
        "document_destination_observed" => Some("document_destination_observed"),
        "document_move_observed" => Some("document_move_observed"),
        "authoring_checkpoint_observed" => Some("authoring_checkpoint_observed"),
        "authoritative_observed" => Some("authoritative_observed"),
        "remote_changed" => Some("remote_changed"),
        "publication_committed" => Some("publication_committed"),
        "initialization_committed" => Some("initialization_committed"),
        _ => None,
    }
}

fn same_lifecycle_action(existing: &str, requested: &str) -> bool {
    existing == requested || (existing == "create_and_enable" && requested == "enable")
}

const REJECTED_STEP: &str = "rejected";

/// Closes the row of a call that was rejected and left nothing behind. The row
/// no longer blocks the repository, and a repeat of its ID begins again.
pub(super) fn discard_operation(
    connection: &Connection,
    record_id: i64,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "UPDATE operation_records SET state = 'completed', completed_step = ?2, observed_at = ?3 WHERE id = ?1",
            params![record_id, REJECTED_STEP, now()],
        )
        .map_err(RepositoryError::sqlite)?;
    Ok(())
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
         SET state = ?2,
               completed_step = CASE
                    WHEN ?2 = 'completed' THEN COALESCE(completed_step, 'completed')
                    WHEN ?2 IN ('indexing', 'observed', 'failed', 'persisted') THEN completed_step
                    WHEN ?2 = 'authoritative_observed' THEN COALESCE(completed_step, ?2)
                    ELSE ?2
               END,
               context_path = ?3,
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

pub(super) fn claim_indexing(
    connection: &Connection,
    record_id: i64,
    stale_before: i64,
) -> Result<Option<IndexOwner>, RepositoryError> {
    let claimed = connection
        .execute(
            "UPDATE operation_records SET state = 'indexing', index_owner_epoch = index_owner_epoch + 1, observed_at = ?2
              WHERE id = ?1 AND state IN (
                 'created', 'worktree_observed', 'authoring_checkpoint_observed', 'remote_changed',
                 'publication_committed', 'initialization_committed', 'authoritative_observed',
                 'failed', 'observed', 'persisted', 'retry'
             ) OR (id = ?1 AND state = 'indexing' AND observed_at < ?3)",
            params![record_id, now(), stale_before],
        )
        .map_err(RepositoryError::sqlite)?;
    if claimed != 1 {
        return Ok(None);
    }
    let epoch = connection
        .query_row(
            "SELECT index_owner_epoch FROM operation_records WHERE id = ?1",
            [record_id],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)?;
    Ok(Some(IndexOwner { record_id, epoch }))
}

pub(super) fn transition_indexing(
    connection: &Connection,
    owner: IndexOwner,
    state: &str,
    context: Option<&Path>,
) -> Result<bool, RepositoryError> {
    Ok(connection
        .execute(
            "UPDATE operation_records SET state = ?2, context_path = ?3, observed_at = ?4
             WHERE id = ?1 AND state = 'indexing' AND index_owner_epoch = ?5",
            params![
                owner.record_id,
                state,
                context.and_then(Path::to_str),
                now(),
                owner.epoch
            ],
        )
        .map_err(RepositoryError::sqlite)?
        == 1)
}

pub(super) fn touch_indexing(
    connection: &Connection,
    owner: IndexOwner,
) -> Result<bool, RepositoryError> {
    Ok(connection
        .execute(
            "UPDATE operation_records SET observed_at = ?2 WHERE id = ?1 AND state = 'indexing' AND index_owner_epoch = ?3",
            params![owner.record_id, now(), owner.epoch],
        )
        .map_err(RepositoryError::sqlite)?
        == 1)
}

pub(super) fn owns_indexing(
    connection: &Connection,
    owner: IndexOwner,
) -> Result<bool, RepositoryError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM operation_records WHERE id = ?1 AND state = 'indexing' AND index_owner_epoch = ?2)",
            params![owner.record_id, owner.epoch],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)
}

pub(super) fn record_persisted_context(
    connection: &Connection,
    record_id: i64,
    context: &Path,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "UPDATE operation_records
         SET context_path = ?2, persisted_context_count = persisted_context_count + 1, observed_at = ?3
          WHERE id = ?1 AND state = 'indexing'",
            params![record_id, context.to_str(), now()],
        )
        .map_err(RepositoryError::sqlite)?;
    Ok(())
}

pub(super) fn record_owned_persisted_context(
    connection: &Connection,
    owner: IndexOwner,
    context: &Path,
) -> Result<bool, RepositoryError> {
    Ok(connection
        .execute(
            "UPDATE operation_records
         SET context_path = ?2, persisted_context_count = persisted_context_count + 1, observed_at = ?3
          WHERE id = ?1 AND state = 'indexing' AND index_owner_epoch = ?4",
            params![owner.record_id, context.to_str(), now(), owner.epoch],
        )
        .map_err(RepositoryError::sqlite)?
        == 1)
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

pub(super) fn action_name(operation: RepositoryOperation) -> &'static str {
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
const REMOTE_HISTORY_MARKER: &str = "remote-history-recovery-required";
const REMOTE_HISTORY_BYTES: &[u8] = b"manyhands remote history recovery required v1\n";

pub(super) fn remote_history_lost(data: &std::path::Path) -> std::io::Result<bool> {
    use std::io::Read;
    let invalid = || std::io::Error::other("remote history recovery marker is unavailable");
    let path = data.join(REMOTE_HISTORY_MARKER);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(invalid()),
        Ok(metadata)
            if !metadata.is_file() || metadata.len() != REMOTE_HISTORY_BYTES.len() as u64 =>
        {
            return Err(invalid());
        }
        Ok(_) => {}
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path).map_err(|_| invalid())?;
    if !file.metadata()?.is_file() {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(REMOTE_HISTORY_BYTES.len() as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes != REMOTE_HISTORY_BYTES {
        return Err(invalid());
    }
    Ok(true)
}

/// The caller holds the exclusive cache recovery guard. Publication and sync
/// must succeed before any registry or sidecar is renamed.
pub(super) fn publish_remote_history_marker(data: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    if !remote_history_lost(data)? {
        let mut temporary = tempfile::NamedTempFile::new_in(data)?;
        temporary.write_all(REMOTE_HISTORY_BYTES)?;
        temporary.as_file().sync_all()?;
        #[cfg(unix)]
        temporary
            .persist_noclobber(data.join(REMOTE_HISTORY_MARKER))
            .map_err(|error| error.error)?;
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
            let source: Vec<u16> = temporary
                .path()
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();
            let destination: Vec<u16> = data
                .join(REMOTE_HISTORY_MARKER)
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();
            // Both NUL-terminated paths remain alive; existing destinations are never replaced.
            if unsafe {
                MoveFileExW(
                    source.as_ptr(),
                    destination.as_ptr(),
                    MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::other(
                    "remote history recovery marker is unavailable",
                ));
            }
        }
    }
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(data.join(REMOTE_HISTORY_MARKER))?
        .sync_all()?;
    #[cfg(unix)]
    std::fs::File::open(data)?.sync_all()?;
    if !remote_history_lost(data)? {
        return Err(std::io::Error::other(
            "remote history recovery marker is unavailable",
        ));
    }
    Ok(())
}
