use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use time::OffsetDateTime;

use super::{
    OperationId, ReadError, RecoveryInspection, RemoteOperationPhase, RepositoryError,
    RepositoryErrorKind, RepositoryOperation, keys::KeyMaterialPhase,
};
use crate::results::ResultCode;

#[derive(Clone, Copy)]
pub(super) struct RecoveryRecord {
    pub(super) id: i64,
    pub(super) is_new: bool,
    pub(super) is_pending: bool,
    #[allow(dead_code)]
    pub(super) completed_step: Option<LocalStep>,
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
    // An abandoned operation is neither replayed as completed nor begun
    // again: its ID is spent. Nothing has been written at this point.
    if let Some((_, _, _, _, _, step)) = &existing
        && listed(step.as_deref()) == Some(LocalStep::Abandoned)
    {
        return Err(abandoned(operation, root));
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
                    .and_then(|(_, _, _, _, _, completed_step)| listed(completed_step.as_deref()))
                    .and_then(LocalStep::continued),
            });
        }
        return Err(recovery_required(operation, root));
    }
    if let Some((id, _, _, _, _, completed_step)) = existing {
        // A refresh may borrow any operation ID; it must not erase the marker.
        if listed(completed_step.as_deref()) == Some(LocalStep::Rejected) && action != "refresh" {
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
            completed_step: listed(completed_step.as_deref()).and_then(LocalStep::continued),
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

/// A step of the local journal: a name an operation records on its row,
/// in the column `completed_step`, once that much of its work is done.
/// These are all the names a build writes there. Each is stored as
/// `as_str` gives it and read back with `parse`, so the name is spelled
/// in one place.
///
/// The journal keeps the column as free text. A row may so hold a name
/// that has the form of one and is none of these; see `RecordedStep`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalStep {
    /// The branch of an authoring context was created or found.
    BranchObserved,
    /// The worktree of an authoring context was created or found.
    WorktreeObserved,
    /// An authoring write reached its destination file.
    AuthoringDestinationObserved,
    /// A document write reached its destination file.
    DocumentDestinationObserved,
    /// A document move removed its source file.
    DocumentMoveObserved,
    /// An authoring operation passed its checkpoint, whether the
    /// checkpoint committed or found nothing to commit.
    AuthoringCheckpointObserved,
    /// The operation's work is observed in the authoritative branch and
    /// its index hand-off has begun.
    AuthoritativeObserved,
    /// A remote was added or removed.
    RemoteChanged,
    /// The publication remote was written to the configuration file.
    PublicationConfigurationWritten,
    /// The publication remote's configuration was committed.
    PublicationCommitted,
    /// A new repository was initialized.
    RepositoryInitialized,
    /// The Manyhands configuration file was written.
    ConfigurationWritten,
    /// The Manyhands configuration was committed.
    InitializationCommitted,
    /// A rebuild or refresh was left to be run again.
    Retry,
    /// A rebuild failed.
    Error,
    /// The row completed having recorded no step of its own.
    Completed,
    /// The row of a call that was rejected and left nothing behind. A
    /// repeat of its ID begins again.
    Rejected,
    /// The row was closed by `abandon_operation`. Its ID is never begun
    /// again and never replayed.
    Abandoned,
}

impl LocalStep {
    /// Every step, in the order of the declaration.
    pub const ALL: [Self; 18] = [
        Self::BranchObserved,
        Self::WorktreeObserved,
        Self::AuthoringDestinationObserved,
        Self::DocumentDestinationObserved,
        Self::DocumentMoveObserved,
        Self::AuthoringCheckpointObserved,
        Self::AuthoritativeObserved,
        Self::RemoteChanged,
        Self::PublicationConfigurationWritten,
        Self::PublicationCommitted,
        Self::RepositoryInitialized,
        Self::ConfigurationWritten,
        Self::InitializationCommitted,
        Self::Retry,
        Self::Error,
        Self::Completed,
        Self::Rejected,
        Self::Abandoned,
    ];

    /// The name as the journal stores it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BranchObserved => "branch_observed",
            Self::WorktreeObserved => "worktree_observed",
            Self::AuthoringDestinationObserved => "authoring_destination_observed",
            Self::DocumentDestinationObserved => "document_destination_observed",
            Self::DocumentMoveObserved => "document_move_observed",
            Self::AuthoringCheckpointObserved => "authoring_checkpoint_observed",
            Self::AuthoritativeObserved => "authoritative_observed",
            Self::RemoteChanged => "remote_changed",
            Self::PublicationConfigurationWritten => "publication_configuration_written",
            Self::PublicationCommitted => "publication_committed",
            Self::RepositoryInitialized => "repository_initialized",
            Self::ConfigurationWritten => "configuration_written",
            Self::InitializationCommitted => "initialization_committed",
            Self::Retry => "retry",
            Self::Error => "error",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Abandoned => "abandoned",
        }
    }

    /// The step stored as `stored`. `None` for any other text.
    pub fn parse(stored: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|step| step.as_str() == stored)
    }

    /// Whether the step is at or past the point where the operation's
    /// work is committed.
    pub const fn is_checkpoint(self) -> bool {
        match self {
            Self::AuthoringCheckpointObserved
            | Self::AuthoritativeObserved
            | Self::InitializationCommitted
            | Self::PublicationCommitted => true,
            Self::BranchObserved
            | Self::WorktreeObserved
            | Self::AuthoringDestinationObserved
            | Self::DocumentDestinationObserved
            | Self::DocumentMoveObserved
            | Self::RemoteChanged
            | Self::PublicationConfigurationWritten
            | Self::RepositoryInitialized
            | Self::ConfigurationWritten
            | Self::Retry
            | Self::Error
            | Self::Completed
            | Self::Rejected
            | Self::Abandoned => false,
        }
    }

    /// Whether the step says the operation's own work is done and
    /// recorded: a checkpoint step, or the step of a remote that was
    /// added or removed, which commits nothing.
    fn is_authoritative(self) -> bool {
        self.is_checkpoint() || self == Self::RemoteChanged
    }

    /// The step as a call under the same ID is told it when the call
    /// finds the row: the steps a domain call continues from. `None` for
    /// a step no call reads back.
    const fn continued(self) -> Option<Self> {
        match self {
            Self::AuthoringDestinationObserved
            | Self::DocumentDestinationObserved
            | Self::DocumentMoveObserved
            | Self::AuthoringCheckpointObserved
            | Self::AuthoritativeObserved
            | Self::RemoteChanged
            | Self::PublicationCommitted
            | Self::InitializationCommitted => Some(self),
            Self::BranchObserved
            | Self::WorktreeObserved
            | Self::PublicationConfigurationWritten
            | Self::RepositoryInitialized
            | Self::ConfigurationWritten
            | Self::Retry
            | Self::Error
            | Self::Completed
            | Self::Rejected
            | Self::Abandoned => None,
        }
    }
}

/// The last step a local row holds, as a reader of the journal is given
/// it. The column is free text, so a row can hold a name that has the
/// form of a step and is none this build writes; such a row has
/// recorded a step, and nothing more is known of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordedStep {
    /// One of the steps this build writes.
    Listed(LocalStep),
    /// A name with the form of a step that is none of `LocalStep`.
    Unlisted,
}

impl RecordedStep {
    /// The step stored as `stored`, which has the form of a stored name.
    pub fn of(stored: &str) -> Self {
        LocalStep::parse(stored).map_or(Self::Unlisted, Self::Listed)
    }

    /// Whether the row's last step is `step`.
    pub fn is(self, step: LocalStep) -> bool {
        self == Self::Listed(step)
    }

    /// Whether the step is at or past the point where the operation's
    /// work is committed; see `LocalStep::is_checkpoint`.
    pub const fn is_checkpoint(self) -> bool {
        match self {
            Self::Listed(step) => step.is_checkpoint(),
            Self::Unlisted => false,
        }
    }
}

/// The step `stored`, a value of the column `completed_step`, when it is
/// one this build writes.
fn listed(stored: Option<&str>) -> Option<LocalStep> {
    stored.and_then(LocalStep::parse)
}

fn same_lifecycle_action(existing: &str, requested: &str) -> bool {
    existing == requested || (existing == "create_and_enable" && requested == "enable")
}

/// The state of a row whose index hand-off a call has claimed.
const INDEXING_STATE: &str = "indexing";

/// The states of a pending row whose operation has handed off to
/// indexing: the state the hand-off begins in, the state of a call that
/// has claimed it, and the two a hand-off that did not bring the index
/// up to date leaves.
const HAND_OFF_STATES: [&str; 4] = ["authoritative_observed", INDEXING_STATE, "failed", "retry"];

/// Closes the row of a call that was rejected and left nothing behind. The row
/// no longer blocks the repository, and a repeat of its ID begins again.
///
/// A row that was abandoned stays abandoned: the call that would discard it
/// is told so with `OperationAbandoned`, and nothing is written.
pub(super) fn discard_operation(
    connection: &Connection,
    record_id: i64,
) -> Result<(), RepositoryError> {
    let discarded = connection
        .execute(
            &format!(
                "UPDATE operation_records SET state = 'completed', completed_step = ?2, observed_at = ?3
                  WHERE id = ?1 AND {NOT_ABANDONED}"
            ),
            params![record_id, LocalStep::Rejected.as_str(), now()],
        )
        .map_err(RepositoryError::sqlite)?;
    if discarded == 0 {
        refuse_abandoned(connection, record_id)?;
    }
    Ok(())
}

/// Whether `step`, the last step of a row that is `completed`, says the
/// row was closed as abandoned. Whoever reads a row's state for itself
/// asks this before it takes `completed` to mean that the work was done.
pub(super) fn is_abandoned_step(step: Option<&str>) -> bool {
    listed(step) == Some(LocalStep::Abandoned)
}

/// What every statement that changes a row by its ID holds of the row: it
/// was not closed as abandoned. An abandoned row is final. No call
/// reopens it, completes it, rejects it or records a step on it, whatever
/// it was doing when the row was closed under it.
pub(super) const NOT_ABANDONED: &str =
    "NOT (state = 'completed' AND completed_step IS 'abandoned')";

/// Fails with `OperationAbandoned` when the row `record_id` was closed
/// as abandoned. A statement guarded by `NOT_ABANDONED` that changed no
/// row asks this, and its caller stops there, having written nothing.
///
/// The error names no operation or root: the caller knows both and sets
/// them with `for_operation`.
fn refuse_abandoned(connection: &Connection, record_id: i64) -> Result<(), RepositoryError> {
    let is_abandoned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM operation_records
              WHERE id = ?1 AND state = 'completed' AND completed_step = ?2)",
            params![record_id, LocalStep::Abandoned.as_str()],
            |row| row.get(0),
        )
        .map_err(RepositoryError::sqlite)?;
    if is_abandoned {
        return Err(RepositoryError::new(
            RepositoryOperation::Read,
            None,
            RepositoryErrorKind::OperationAbandoned,
            ABANDONED_MESSAGE,
        ));
    }
    Ok(())
}

const ABANDONED_MESSAGE: &str = "the operation was abandoned and its ID cannot be used again";

/// Where one operation stands in its journal: the local one here, the
/// remote one or the key-material one. Each journal answers through a
/// lookup of its own, by operation ID, that needs no registration and no
/// repository on disk.
///
/// This is what settling a request asks of a journal. The question "is
/// anything in flight?" is `in_flight`; which way a row ended is `kind`.
/// The two are separate: a row can have ended and still owe work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalRow {
    /// The journal holds no row for the ID, or holds the row of a call
    /// that was rejected and left nothing behind. Nothing is in flight:
    /// a record whose request failed is deleted, and its ID is free.
    Absent,
    /// The operation has not ended. It has work that only a call under
    /// its ID takes up again, so the record stays `accepted`.
    Pending(PendingOperation),
    /// The operation ended, in the way `kind` says: the domain will not
    /// change what it came to under this ID.
    ///
    /// `owes_work` is set when the row has nevertheless left something to
    /// be done under its ID: a remote operation whose index hand-off is
    /// pending or whose reconciliation is recorded as required. Such a
    /// row is in flight for settlement, exactly as a pending one is, and
    /// is still told apart from one: a cancelled row that owes work is
    /// `Final { kind: Cancelled, owes_work: true }`.
    ///
    /// `checkpointed` says whether the row records that the operation
    /// passed the point where its work is committed. For a local
    /// operation that is one of the steps `LocalStep::is_checkpoint`
    /// names. An authoring operation records its
    /// checkpoint step whether the checkpoint committed or found nothing
    /// to commit, so the step does not say a commit was made; its absence
    /// says none was: the operation stopped before its checkpoint, for
    /// want of an identity, say, with only a context step or none
    /// recorded. A local operation with no such step, a refresh among
    /// them, reads `false`. The remote and key-material journals record
    /// phases and not steps, and their final rows read `true`.
    Final {
        kind: FinalKind,
        owes_work: bool,
        checkpointed: bool,
    },
}

/// Where an operation that has not ended stands, in its journal's terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingOperation {
    /// The local journal has no enumeration of its states: `state` is
    /// its own name, in lower_snake_case, as it stores it. `step` is the
    /// last step the operation recorded as done.
    Local {
        state: String,
        step: Option<RecordedStep>,
    },
    /// Never `Completed` or `Cancelled`, which are final. `Interrupted`
    /// and `Failed` are pending: the operation stopped with work that a
    /// call under its ID takes up again, a merge conflict among it.
    Remote { phase: RemoteOperationPhase },
    /// Never `Completed` or `RetainedForInspection`, which are final.
    KeyMaterial { phase: KeyMaterialPhase },
}

/// The way an operation ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalKind {
    /// It ran to its end. Any journal.
    Completed,
    /// A remote operation that was cancelled. It is never given its
    /// reservation back, so the result is final for its ID.
    Cancelled,
    /// A key-material operation whose files were kept for inspection.
    /// Nothing continues it, so the result is final for its ID.
    RetainedForInspection,
}

impl JournalRow {
    /// The settlement question: is anything in flight under this
    /// operation ID? True for a pending row and for a final row that
    /// still owes work; false for an absent row and for a final row that
    /// owes none. A failed request's record is deleted only when this is
    /// false.
    ///
    /// Settling asks which way a final row ended, its `kind`, before it
    /// asks this. A row the domain has made final as `Cancelled` or
    /// `RetainedForInspection` gives the request its result even while
    /// it owes work; asked in the other order, such a row would read as
    /// in flight and its record would stay accepted for good.
    pub const fn in_flight(&self) -> bool {
        match self {
            Self::Absent => false,
            Self::Pending(_) => true,
            Self::Final { owes_work, .. } => *owes_work,
        }
    }
}

/// The longest state or step name a local operation is taken to have.
const LONGEST_STORED_NAME: usize = 64;

/// Whether `stored` has the form of a local operation's state or step
/// name. The journal keeps these as free text, so only what has that form
/// is passed on: a lowercase letter followed by lowercase letters, digits
/// and underscores.
fn is_stored_name(stored: &str) -> bool {
    stored.len() <= LONGEST_STORED_NAME
        && stored.starts_with(|first: char| first.is_ascii_lowercase())
        && stored
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Where the local operation `operation_id` on `root_path` stands.
///
/// The row is matched by root, as recovery matches it, and not by
/// registration: an operation that creates or enables a repository
/// precedes its registration, and the root may since have gone. A row
/// that is completed with the step `rejected` is the kept row of a call
/// that left nothing behind, and is reported as absent. So is a row that
/// was abandoned: nothing is in flight under its ID, the request that
/// owned it was released with it, and what tells it apart from a rejected
/// row is the domain's to act on, which refuses a repeat of its ID.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
pub(super) fn lookup_operation(
    connection: &Connection,
    root_path: &str,
    operation_id: OperationId,
) -> Result<JournalRow, ReadError> {
    let row: Option<(String, Option<String>)> = connection
        .query_row(
            "SELECT state, completed_step FROM operation_records
              WHERE root_path = ?1 AND operation_ulid = ?2",
            params![root_path, operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((state, step)) = row else {
        return Ok(JournalRow::Absent);
    };
    if !is_stored_name(&state) || step.as_deref().is_some_and(|step| !is_stored_name(step)) {
        return Err(ReadError::new(ResultCode::InternalError));
    }
    let step = step.as_deref().map(RecordedStep::of);
    Ok(if state != "completed" {
        JournalRow::Pending(PendingOperation::Local { state, step })
    } else if step.is_some_and(|step| step.is(LocalStep::Rejected) || step.is(LocalStep::Abandoned))
    {
        JournalRow::Absent
    } else {
        // A local operation that completed owes nothing more.
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
            checkpointed: step.is_some_and(RecordedStep::is_checkpoint),
        }
    })
}

/// The last step the local operation `operation_id` on `root_path`
/// recorded, when the operation has completed. `None` for an operation
/// that has not completed, that the journal does not hold, or that
/// completed having recorded no step of its own: such a row holds
/// `completed`, `rejected` when its call left nothing behind, or
/// `abandoned` when it was closed by `abandon_operation`.
///
/// `lookup_operation` says whether a row has completed. This says what
/// the completed row recorded, for an operation whose step tells what it
/// did: `add_remote` records `remote_changed` only on the row of the
/// call that added the remote.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
pub(super) fn completed_step(
    connection: &Connection,
    root_path: &str,
    operation_id: OperationId,
) -> Result<Option<RecordedStep>, ReadError> {
    let step: Option<Option<String>> = connection
        .query_row(
            "SELECT completed_step FROM operation_records
              WHERE root_path = ?1 AND operation_ulid = ?2 AND state = 'completed'",
            params![root_path, operation_id.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    match step.flatten() {
        Some(step) if !is_stored_name(&step) => Err(ReadError::new(ResultCode::InternalError)),
        Some(step) => {
            let step = RecordedStep::of(&step);
            let closes_only = step.is(LocalStep::Completed)
                || step.is(LocalStep::Rejected)
                || step.is(LocalStep::Abandoned);
            Ok((!closes_only).then_some(step))
        }
        None => Ok(None),
    }
}

/// One row of the local journal, as `abandon_operation` reads it.
pub(super) struct LocalRow {
    pub(super) id: i64,
    pub(super) action: String,
    pub(super) target: Option<String>,
    pub(super) state: String,
    pub(super) step: Option<String>,
    pub(super) observed_at: i64,
}

/// How a local row stands for someone who would abandon it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LocalStanding {
    Pending,
    Completed,
    Rejected,
    Abandoned,
}

impl LocalRow {
    pub(super) fn standing(&self) -> LocalStanding {
        let step = listed(self.step.as_deref());
        if self.state != "completed" {
            LocalStanding::Pending
        } else if step == Some(LocalStep::Rejected) {
            LocalStanding::Rejected
        } else if step == Some(LocalStep::Abandoned) {
            LocalStanding::Abandoned
        } else {
            LocalStanding::Completed
        }
    }

    /// Whether the row shows a call at work under it that holds no
    /// lease: the heartbeat of an index owner, which a row in the state
    /// `indexing` has from the call that claimed it, or of a rebuild,
    /// which touches its row for as long as it runs. `stale_before` is
    /// the time before which a heartbeat is too old to be anyone's. A
    /// clock that reads earlier than the heartbeat does not make it
    /// stale.
    ///
    /// Every other pending row is worked under with a lease held, which
    /// whoever asks this takes or cannot take.
    pub(super) fn has_live_heartbeat(&self, stale_before: i64) -> bool {
        let beats = self.state == INDEXING_STATE
            || operation_from_name(&self.action) == Some(RepositoryOperation::RebuildRepository);
        self.state != "completed" && beats && self.observed_at >= stale_before
    }

    /// Whether the row records that the operation passed the point where
    /// its work is committed; see `LocalStep::is_checkpoint`.
    pub(super) fn is_checkpointed(&self) -> bool {
        listed(self.step.as_deref()).is_some_and(LocalStep::is_checkpoint)
    }

    /// Whether all the row still owes is its index hand-off: it is
    /// pending, the last step it recorded says its operation's own work
    /// is done, and its state is one a row has only once the operation
    /// has handed off to indexing.
    ///
    /// Both are asked. The step alone is not enough: an enablement or a
    /// selection of the publication remote records its commit before it
    /// brings the registration up to it, so a row that holds the step and
    /// has not handed off can owe more than the hand-off. Nor is the
    /// state: a refresh that failed is in one of these states with no
    /// work of its own recorded.
    ///
    /// Such a row is completed by a refresh under its ID, which is what
    /// `operation resume` runs. A refresh completes any pending row it is
    /// given, so whoever would resume a row asks this first.
    pub(super) fn owes_only_its_hand_off(&self) -> bool {
        self.standing() == LocalStanding::Pending
            && listed(self.step.as_deref()).is_some_and(LocalStep::is_authoritative)
            && HAND_OFF_STATES.contains(&self.state.as_str())
    }

    /// The operation the row is of. `None` for an action this build
    /// does not know.
    pub(super) fn operation(&self) -> Option<RepositoryOperation> {
        operation_from_name(&self.action)
    }

    /// Whether the row's state and step have the form of the names the
    /// journal stores; see `is_stored_name`.
    pub(super) fn is_well_formed(&self) -> bool {
        is_stored_name(&self.state) && self.step.as_deref().is_none_or(is_stored_name)
    }
}

/// The row of the local operation `operation_id` on `root_path`.
pub(super) fn local_row(
    connection: &Connection,
    root_path: &str,
    operation_id: OperationId,
) -> rusqlite::Result<Option<LocalRow>> {
    connection
        .query_row(
            "SELECT id, action, target, state, completed_step, observed_at
               FROM operation_records WHERE root_path = ?1 AND operation_ulid = ?2",
            params![root_path, operation_id.to_string()],
            |row| {
                Ok(LocalRow {
                    id: row.get(0)?,
                    action: row.get(1)?,
                    target: row.get(2)?,
                    state: row.get(3)?,
                    step: row.get(4)?,
                    observed_at: row.get(5)?,
                })
            },
        )
        .optional()
}

/// Closes the pending row `record_id` as abandoned. The step it had
/// reached is replaced by the marker: what it was is in the result of the
/// abandon. Returns whether a pending row was closed.
pub(super) fn abandon_row(connection: &Connection, record_id: i64) -> rusqlite::Result<bool> {
    Ok(connection.execute(
        "UPDATE operation_records SET state = 'completed', completed_step = ?2, observed_at = ?3
          WHERE id = ?1 AND state != 'completed'",
        params![record_id, LocalStep::Abandoned.as_str(), now()],
    )? == 1)
}

/// Sets the heartbeat of the pending row `record_id` to `rested_at`, a
/// time at which no heartbeat is fresh: the call that touched the row has
/// ended and left it pending. A row that has ended is not touched.
pub(super) fn rest_pending(
    connection: &Connection,
    record_id: i64,
    rested_at: i64,
) -> rusqlite::Result<()> {
    connection.execute(
        "UPDATE operation_records SET observed_at = ?2 WHERE id = ?1 AND state != 'completed'",
        params![record_id, rested_at],
    )?;
    Ok(())
}

/// The pending local rows of `root_path`, oldest first. A row without an
/// ID, which only an index made before operations had IDs holds, is left
/// out: nothing can name it.
pub(super) fn pending_rows(
    connection: &Connection,
    root_path: &str,
) -> rusqlite::Result<Vec<(String, LocalRow)>> {
    connection
        .prepare(
            "SELECT operation_ulid, id, action, target, state, completed_step, observed_at
               FROM operation_records
              WHERE root_path = ?1 AND state != 'completed' AND operation_ulid IS NOT NULL
              ORDER BY id",
        )?
        .query_map([root_path], |row| {
            Ok((
                row.get(0)?,
                LocalRow {
                    id: row.get(1)?,
                    action: row.get(2)?,
                    target: row.get(3)?,
                    state: row.get(4)?,
                    step: row.get(5)?,
                    observed_at: row.get(6)?,
                },
            ))
        })?
        .collect()
}

/// The table of the remote journal, and that of the key journal.
pub(super) const REMOTE_JOURNAL_TABLE: &str = "remote_operation_records";
pub(super) const KEY_JOURNAL_TABLE: &str = "key_material_operations";

/// Whether the index holds the table `table`. An index that has never
/// held an operation of the remote journal or of the key journal may
/// lack that journal's table.
pub(super) fn has_table(connection: &Connection, table: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )
}

/// Whether the remote journal or the key journal holds an operation of
/// the ID, in any repository. Such an operation is not a local one even
/// when the local journal holds a row under its ID, as it does for the
/// index hand-off of a synchronization: the row is that operation's to
/// end, and the operation has its own way out.
pub(super) fn held_by_another_journal(
    connection: &Connection,
    operation_id: &str,
) -> rusqlite::Result<bool> {
    for (table, column) in [
        (REMOTE_JOURNAL_TABLE, "operation_ulid"),
        (KEY_JOURNAL_TABLE, "operation_id"),
    ] {
        if !has_table(connection, table)? {
            continue;
        }
        let held: bool = connection.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column} = ?1)"),
            [operation_id],
            |row| row.get(0),
        )?;
        if held {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Records the state or step `state` on the row `record_id`.
///
/// A row that was abandoned is not changed: the call fails with
/// `OperationAbandoned`, and its caller stops, having written nothing
/// under the row. This is what keeps a call that was still at work when
/// its row was closed from reopening it.
pub(super) fn advance_after_observation(
    connection: &Connection,
    record_id: i64,
    state: &str,
    context: Option<&Path>,
    persisted_context_count: Option<i64>,
) -> Result<(), RepositoryError> {
    let advanced = connection
        .execute(
            &format!(
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
          WHERE id = ?1 AND NOT (state = 'completed' AND ?2 = 'error') AND {NOT_ABANDONED}"
            ),
            params![
                record_id,
                state,
                context.and_then(Path::to_str),
                persisted_context_count,
                now()
            ],
        )
        .map_err(RepositoryError::sqlite)?;
    // A completed row that is told of an error is left as it is, as
    // before. Recording an error on an abandoned row is the same: the
    // call that failed is already stopping.
    if advanced == 0 && state != "error" {
        refuse_abandoned(connection, record_id)?;
    }
    Ok(())
}

/// Marks the pending row `record_id` as touched now: the heartbeat of a
/// call that works under the row with no lease held and no index claim,
/// as a rebuild does while it observes. Nothing else of the row changes,
/// and a row that has ended is not touched.
pub(super) fn touch_pending(connection: &Connection, record_id: i64) -> rusqlite::Result<()> {
    connection.execute(
        "UPDATE operation_records SET observed_at = ?2 WHERE id = ?1 AND state != 'completed'",
        params![record_id, now()],
    )?;
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

pub(super) fn abandoned(operation: RepositoryOperation, root: &Path) -> RepositoryError {
    RepositoryError::new(
        operation,
        Some(root.to_owned()),
        RepositoryErrorKind::OperationAbandoned,
        ABANDONED_MESSAGE,
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

#[cfg(test)]
mod abandoned_row_tests {
    //! An abandoned row is final: every statement that changes a row by
    //! its ID leaves one as it is, and the two a running call makes under
    //! its own row say so with `OperationAbandoned`.

    use super::*;

    const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAB";

    /// A journal holding one row, in `state` with `step`, touched at
    /// `observed_at`.
    fn journal(action: &str, state: &str, step: Option<&str>, observed_at: i64) -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        // The table the journal refers to; no row of it is needed.
        connection
            .execute_batch("CREATE TABLE repositories (id INTEGER PRIMARY KEY, root_path TEXT)")
            .unwrap();
        migrate_operation_records(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO operation_records
                    (id, root_path, operation_ulid, action, target, state, completed_step, observed_at)
                 VALUES (1, '/repository', ?1, ?2, '', ?3, ?4, ?5)",
                params![ID, action, state, step, observed_at],
            )
            .unwrap();
        connection
    }

    fn abandoned_journal() -> Connection {
        let connection = journal(
            "save_ticket",
            "worktree_observed",
            Some("worktree_observed"),
            7,
        );
        assert!(abandon_row(&connection, 1).unwrap());
        connection
    }

    fn row(connection: &Connection) -> (String, Option<String>, i64, i64) {
        connection
            .query_row(
                "SELECT state, completed_step, index_owner_epoch, persisted_context_count
                   FROM operation_records WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
    }

    fn abandoned_row() -> (String, Option<String>, i64, i64) {
        ("completed".to_owned(), Some("abandoned".to_owned()), 0, 0)
    }

    #[test]
    fn every_step_is_stored_under_its_own_name_and_read_back() {
        for step in LocalStep::ALL {
            assert!(is_stored_name(step.as_str()), "{step:?}");
            assert_eq!(LocalStep::parse(step.as_str()), Some(step));
            assert_eq!(RecordedStep::of(step.as_str()), RecordedStep::Listed(step));
        }
        let mut names = LocalStep::ALL.map(LocalStep::as_str).to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), LocalStep::ALL.len());
        // A name with the form of a step that no build writes is a step
        // recorded, and no more is known of it.
        assert_eq!(LocalStep::parse("some_other_step"), None);
        assert_eq!(RecordedStep::of("some_other_step"), RecordedStep::Unlisted);
        assert!(!RecordedStep::Unlisted.is_checkpoint());
    }

    #[test]
    fn no_step_or_state_is_recorded_on_an_abandoned_row() {
        for state in [
            "observed",
            "persisted",
            "retry",
            "failed",
            "indexing",
            "completed",
            "authoring_checkpoint_observed",
            "authoritative_observed",
            "repository_initialized",
        ] {
            let connection = abandoned_journal();
            let refused =
                advance_after_observation(&connection, 1, state, None, Some(3)).unwrap_err();
            assert_eq!(
                refused.kind,
                RepositoryErrorKind::OperationAbandoned,
                "{state}"
            );
            assert_eq!(row(&connection), abandoned_row(), "{state}");
        }
        // A call that failed and records its error is already stopping:
        // nothing is recorded, and nothing more is said.
        let connection = abandoned_journal();
        advance_after_observation(&connection, 1, "error", None, None).unwrap();
        assert_eq!(row(&connection), abandoned_row());
    }

    #[test]
    fn an_abandoned_row_is_not_discarded_as_rejected() {
        let connection = abandoned_journal();
        let refused = discard_operation(&connection, 1).unwrap_err();
        assert_eq!(refused.kind, RepositoryErrorKind::OperationAbandoned);
        assert_eq!(row(&connection), abandoned_row());
    }

    #[test]
    fn an_abandoned_row_is_not_claimed_touched_or_abandoned_again() {
        let connection = abandoned_journal();
        assert!(claim_indexing(&connection, 1, i64::MAX).unwrap().is_none());
        let owner = IndexOwner {
            record_id: 1,
            epoch: 0,
        };
        assert!(!transition_indexing(&connection, owner, "completed", None).unwrap());
        assert!(!touch_indexing(&connection, owner).unwrap());
        assert!(
            !record_owned_persisted_context(&connection, owner, Path::new("/context")).unwrap()
        );
        // The time the row was closed at is set apart from any time a
        // heartbeat could write, so that a statement that moved it shows.
        const CLOSED_AT: i64 = 424_242;
        connection
            .execute("UPDATE operation_records SET observed_at = ?1", [CLOSED_AT])
            .unwrap();
        record_persisted_context(&connection, 1, Path::new("/context")).unwrap();
        touch_pending(&connection, 1).unwrap();
        rest_pending(&connection, 1, 0).unwrap();
        assert!(!abandon_row(&connection, 1).unwrap());
        assert_eq!(row(&connection), abandoned_row());
        let (observed_at, context): (i64, Option<String>) = connection
            .query_row(
                "SELECT observed_at, context_path FROM operation_records",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        // No heartbeat, rest or claim moved it, and no context was
        // recorded on it.
        assert_eq!(observed_at, CLOSED_AT);
        assert_eq!(context, None);
        assert!(lookup_row(&connection).standing() == LocalStanding::Abandoned);
    }

    fn lookup_row(connection: &Connection) -> LocalRow {
        local_row(connection, "/repository", OperationId::parse(ID).unwrap())
            .unwrap()
            .unwrap()
    }

    #[test]
    fn a_row_that_is_not_abandoned_is_advanced_and_discarded_as_before() {
        let connection = journal("rebuild", "created", None, 7);
        advance_after_observation(&connection, 1, "observed", None, None).unwrap();
        assert_eq!(row(&connection).0, "observed");
        discard_operation(&connection, 1).unwrap();
        assert_eq!(
            row(&connection),
            ("completed".to_owned(), Some("rejected".to_owned()), 0, 0)
        );
        // An error is not recorded on a row that completed, as before.
        advance_after_observation(&connection, 1, "error", None, None).unwrap();
        assert_eq!(row(&connection).0, "completed");
    }

    #[test]
    fn a_row_owes_only_its_hand_off_once_its_work_is_recorded_and_it_has_handed_off() {
        let owes = |action: &str, state: &str, step: Option<&str>| {
            lookup_row(&journal(action, state, step, 7)).owes_only_its_hand_off()
        };
        // Each step that records an operation's own work as done, in
        // each state a row has once it has handed off to indexing.
        for step in [
            "authoring_checkpoint_observed",
            "authoritative_observed",
            "initialization_committed",
            "publication_committed",
            "remote_changed",
        ] {
            for state in ["authoritative_observed", "indexing", "failed", "retry"] {
                assert!(owes("save_ticket", state, Some(step)), "{step} in {state}");
            }
            // The step is recorded and the hand-off has not begun: an
            // enablement or a selection can still owe its registration.
            // `authoritative_observed` is the one step the hand-off
            // itself records, on a row that had recorded none.
            assert_eq!(
                owes("save_ticket", step, Some(step)),
                step == "authoritative_observed",
                "{step}"
            );
            // Nothing is owed by a row that has ended.
            assert!(!owes("save_ticket", "completed", Some(step)), "{step}");
        }
        // A step that precedes the operation's work, or none: the write
        // may not have happened, whatever state the row is in.
        for step in [
            None,
            Some("worktree_observed"),
            Some("authoring_destination_observed"),
            Some("document_destination_observed"),
            Some("document_move_observed"),
            Some("repository_initialized"),
            Some("configuration_written"),
        ] {
            for state in [
                "created",
                "worktree_observed",
                "authoritative_observed",
                "indexing",
                "failed",
                "retry",
            ] {
                assert!(!owes("save_ticket", state, step), "{step:?} in {state}");
            }
        }
        // A refresh that failed has no work of its own recorded, and a
        // rebuild's states are not a hand-off's.
        assert!(!owes("refresh", "failed", None));
        assert!(!owes("rebuild", "observed", None));
        assert!(!owes("rebuild", "error", None));
        // An abandoned row and a rejected one have ended.
        assert!(!lookup_row(&abandoned_journal()).owes_only_its_hand_off());
        assert!(!owes("save_ticket", "completed", Some("rejected")));
    }

    #[test]
    fn only_an_index_owner_and_a_rebuild_show_a_call_at_work_by_a_heartbeat() {
        let now = 1_000;
        let stale_before = now - 3;
        for (action, state, observed_at, live) in [
            ("refresh", "indexing", now, true),
            ("save_ticket", "indexing", stale_before, true),
            ("save_ticket", "indexing", stale_before - 1, false),
            ("rebuild", "created", now, true),
            ("rebuild", "observed", now, true),
            ("rebuild", "retry", stale_before - 1, false),
            // A clock set back does not make a heartbeat stale.
            ("rebuild", "created", now + 60, true),
            // Worked under with a lease held, which a read cannot see.
            ("save_ticket", "worktree_observed", now, false),
            ("enable", "created", now, false),
            // Ended.
            ("rebuild", "completed", now, false),
        ] {
            let connection = journal(action, state, None, observed_at);
            assert_eq!(
                lookup_row(&connection).has_live_heartbeat(stale_before),
                live,
                "{action} {state} {observed_at}"
            );
        }
    }

    #[test]
    fn an_id_the_remote_or_the_key_journal_holds_is_not_a_local_operation_s() {
        let connection = journal("refresh", "failed", None, 7);
        // Neither table is there: nothing holds it.
        assert!(!held_by_another_journal(&connection, ID).unwrap());
        connection
            .execute_batch(
                "CREATE TABLE remote_operation_records (operation_ulid TEXT);
                 CREATE TABLE key_material_operations (operation_id TEXT);",
            )
            .unwrap();
        assert!(!held_by_another_journal(&connection, ID).unwrap());
        connection
            .execute("INSERT INTO key_material_operations VALUES (?1)", [ID])
            .unwrap();
        assert!(held_by_another_journal(&connection, ID).unwrap());
        connection
            .execute("DELETE FROM key_material_operations", [])
            .unwrap();
        connection
            .execute("INSERT INTO remote_operation_records VALUES (?1)", [ID])
            .unwrap();
        assert!(held_by_another_journal(&connection, ID).unwrap());
    }
}
