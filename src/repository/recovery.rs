use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use time::OffsetDateTime;

use super::{
    Blocker, OperationId, ReadError, RecoveryInspection, RemoteOperationPhase, RepositoryError,
    RepositoryErrorKind, RepositoryOperation, keys::KeyMaterialPhase,
};
use crate::results::ResultCode;

/// Correlation only. No body, body digest, endpoint, path or remote phase copy.
#[derive(Clone)]
pub(crate) struct CommentBinding {
    pub(super) operation_id: OperationId,
    pub(super) synchronization_id: OperationId,
    pub(super) kind: super::AuthoringKind,
    pub(super) item_id: crate::canonical::ItemId,
    pub(super) comment_id: crate::canonical::ItemId,
    pub(super) parent_id: Option<crate::canonical::ItemId>,
    pub(super) pre_checkpoint_oid: git2::Oid,
    pub(super) checkpoint_oid: Option<git2::Oid>,
    pub(super) created_at: Option<String>,
    pub(super) receipt_recorded: bool,
}
impl CommentBinding {
    pub(super) fn matches_request(
        &self,
        request: &super::SubmitCommentRequest,
    ) -> Result<(), RepositoryError> {
        if self.operation_id != request.target.operation_id
            || self.kind != request.target.kind
            || self.item_id != request.target.item_id
            || self.comment_id != request.comment_id
            || self.parent_id != request.parent_id
            || request.target.intent != super::ContextIntent::Edit
        {
            return Err(mismatch(
                RepositoryOperation::SubmitComment,
                &request.target.root,
            ));
        }
        Ok(())
    }
}

const COMMENT_BINDING_SCHEMA: &str = "CREATE TABLE comment_publication_bindings (
        operation_ulid TEXT PRIMARY KEY NOT NULL,
        local_record_id INTEGER NOT NULL UNIQUE REFERENCES operation_records(id) ON DELETE CASCADE,
        root_digest BLOB NOT NULL CHECK(typeof(root_digest)='blob' AND length(root_digest)=32),
        kind TEXT NOT NULL CHECK(kind IN ('document','ticket')),
        item_ulid TEXT NOT NULL,
        comment_ulid TEXT NOT NULL,
        parent_ulid TEXT,
        synchronization_ulid TEXT NOT NULL UNIQUE,
        pre_checkpoint_oid TEXT NOT NULL CHECK(length(pre_checkpoint_oid)=40 AND pre_checkpoint_oid NOT GLOB '*[^0-9a-f]*'),
        checkpoint_oid TEXT CHECK(length(checkpoint_oid)=40 AND checkpoint_oid NOT GLOB '*[^0-9a-f]*'),
        created_at TEXT,
        CHECK(checkpoint_oid IS NULL OR created_at IS NOT NULL),
        CHECK(operation_ulid!=synchronization_ulid),
        UNIQUE(root_digest,comment_ulid)
    )";

pub(super) fn migrate_comment_bindings(connection: &Connection) -> Result<(), RepositoryError> {
    let failure = super::comment_publication::recovery_error;
    connection
        .execute_batch("CREATE TABLE IF NOT EXISTS registry_migrations (name TEXT PRIMARY KEY);")
        .map_err(|_| failure())?;
    let migrated: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name='cycle_07_comment_publication_bindings')", [], |r| r.get(0)).map_err(|_| failure())?;
    let actual: Option<String> = connection.query_row("SELECT sql FROM sqlite_master WHERE type='table' AND name='comment_publication_bindings'", [], |r| r.get(0)).optional().map_err(|_| failure())?;
    match actual {
        None if !migrated => {
            connection
                .execute_batch(COMMENT_BINDING_SCHEMA)
                .map_err(|_| failure())?;
        }
        Some(actual)
            if actual.split_whitespace().collect::<String>()
                == COMMENT_BINDING_SCHEMA
                    .split_whitespace()
                    .collect::<String>() => {}
        _ => return Err(failure()),
    }
    // An already migrated registry is an audit only. Even INSERT OR IGNORE of
    // its existing marker promotes this read transaction to a writer and can
    // fail with SQLITE_BUSY_SNAPSHOT when another opener commits meanwhile.
    if !migrated {
        connection.execute("INSERT INTO registry_migrations(name) VALUES('cycle_07_comment_publication_bindings')", []).map_err(|_| failure())?;
    }
    // A partial or substituted table is not repaired on startup.
    let columns = [
        "operation_ulid",
        "local_record_id",
        "root_digest",
        "kind",
        "item_ulid",
        "comment_ulid",
        "parent_ulid",
        "synchronization_ulid",
        "pre_checkpoint_oid",
        "checkpoint_oid",
        "created_at",
    ];
    let actual = connection
        .prepare("SELECT name FROM pragma_table_info('comment_publication_bindings') ORDER BY cid")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| super::comment_publication::recovery_error())?;
    if actual != columns {
        return Err(super::comment_publication::recovery_error());
    }
    Ok(())
}

fn decode_comment_binding(row: &rusqlite::Row<'_>) -> rusqlite::Result<CommentBinding> {
    let invalid = || rusqlite::Error::InvalidQuery;
    let operation: String = row.get(0)?;
    let child: String = row.get(1)?;
    let kind: String = row.get(2)?;
    let item: String = row.get(3)?;
    let comment: String = row.get(4)?;
    let parent: Option<String> = row.get(5)?;
    let pre: String = row.get(6)?;
    let checkpoint: Option<String> = row.get(7)?;
    let created_at: Option<String> = row.get(8)?;
    if created_at.as_ref().is_some_and(|time| {
        time.parse::<i128>()
            .ok()
            .and_then(|time| OffsetDateTime::from_unix_timestamp_nanos(time).ok())
            .is_none()
    }) {
        return Err(invalid());
    }
    if checkpoint.is_some() && created_at.is_none() {
        return Err(invalid());
    }
    let receipt_recorded = checkpoint.is_some();
    Ok(CommentBinding {
        operation_id: OperationId::parse(&operation).map_err(|_| invalid())?,
        synchronization_id: OperationId::parse(&child).map_err(|_| invalid())?,
        kind: match kind.as_str() {
            "document" => super::AuthoringKind::Document,
            "ticket" => super::AuthoringKind::Ticket,
            _ => return Err(invalid()),
        },
        item_id: item.parse().map_err(|_| invalid())?,
        comment_id: comment.parse().map_err(|_| invalid())?,
        parent_id: parent
            .map(|parent| parent.parse().map_err(|_| invalid()))
            .transpose()?,
        pre_checkpoint_oid: git2::Oid::from_str(&pre).map_err(|_| invalid())?,
        checkpoint_oid: checkpoint
            .map(|oid| git2::Oid::from_str(&oid).map_err(|_| invalid()))
            .transpose()?,
        created_at,
        receipt_recorded,
    })
}

/// Audits every binding, as registry open's migration does.
pub(super) fn validate_comment_bindings(connection: &Connection) -> Result<(), RepositoryError> {
    let failure = super::comment_publication::recovery_error;
    let rows = connection.prepare("SELECT operation_ulid,synchronization_ulid,kind,item_ulid,comment_ulid,parent_ulid,pre_checkpoint_oid,checkpoint_oid,created_at FROM comment_publication_bindings")
        .and_then(|mut statement| statement.query_map([], decode_comment_binding)?.collect::<Result<Vec<_>, _>>()).map_err(|_| failure())?;
    for binding in rows {
        audit_comment_binding(connection, &binding)?;
    }
    Ok(())
}

/// Audits the binding of the comment operation `operation` exactly as
/// `validate_comment_bindings` audits each one, with the same refusal: a
/// call that takes a binding up again after registry open audits it here,
/// since no later migration does (design decision 9). Nothing is refused
/// when no binding has that operation ID.
pub(super) fn validate_comment_binding(
    connection: &Connection,
    operation: &str,
) -> Result<(), RepositoryError> {
    let failure = super::comment_publication::recovery_error;
    let binding = connection.query_row("SELECT operation_ulid,synchronization_ulid,kind,item_ulid,comment_ulid,parent_ulid,pre_checkpoint_oid,checkpoint_oid,created_at FROM comment_publication_bindings WHERE operation_ulid=?1",
        [operation], decode_comment_binding).optional().map_err(|_| failure())?;
    binding.map_or(Ok(()), |binding| {
        audit_comment_binding(connection, &binding)
    })
}

/// One binding's root digest, its submission's target, and the
/// correlation of its synchronization's hand-off row.
fn audit_comment_binding(
    connection: &Connection,
    binding: &CommentBinding,
) -> Result<(), RepositoryError> {
    let failure = super::comment_publication::recovery_error;
    let (root, target, digest): (String, String, Vec<u8>) = connection.query_row("SELECT o.root_path,o.target,b.root_digest
        FROM operation_records o JOIN comment_publication_bindings b ON b.local_record_id=o.id WHERE b.operation_ulid=?1",
        [binding.operation_id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_| failure())?;
    let expected_target = format!(
        "authoring-context-v1/{}/{}/Edit\0.manyhands/comments\0{}",
        super::authoring_kind_segment(&binding.kind),
        binding.item_id,
        binding.comment_id
    );
    if digest != blake3::hash(root.as_bytes()).as_bytes().as_slice() || target != expected_target {
        return Err(failure());
    }
    // The one local row under the synchronization's ID is its index
    // hand-off: a refresh with the bound context's hand-off target at
    // the authoritative commit (P-4), or with no target, as older builds
    // began it.
    let authoritative: Option<String> = connection
        .query_row(
            "SELECT r.authoritative_oid FROM remote_operation_records r
               JOIN repositories p ON p.id=r.repository_id
              WHERE r.operation_ulid=?1 AND p.root_path=?2",
            params![binding.synchronization_id.to_string(), root],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| failure())?
        .flatten();
    let hand_off = authoritative.map(|oid| {
        hand_off_target(
            &format!(
                "{}/{}",
                super::authoring_kind_segment(&binding.kind),
                binding.item_id
            ),
            &oid,
        )
    });
    let invalid: bool = connection.query_row("SELECT
        NOT EXISTS(SELECT 1 FROM operation_records o JOIN comment_publication_bindings b ON b.local_record_id=o.id
            WHERE b.operation_ulid=?1 AND o.operation_ulid=?1 AND o.action='submit_comment')
        OR EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?2 AND (action!='refresh' OR target IS NULL OR (target!='' AND target IS NOT ?6) OR root_path!=?3))
        OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1)
        OR EXISTS(SELECT 1 FROM remote_operation_records r JOIN repositories p ON p.id=r.repository_id WHERE r.operation_ulid=?2
            AND (p.root_path!=?3 OR r.action!='synchronize_context' OR r.kind!=?4 OR r.item_id!=?5))
        OR EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?2) AND NOT EXISTS(
            SELECT 1 FROM remote_operation_records r JOIN repositories p ON p.id=r.repository_id WHERE r.operation_ulid=?2 AND p.root_path=?3 AND r.authoritative_kind IS NOT NULL)
        OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?2 OR synchronization_ulid=?1)",
        params![binding.operation_id.to_string(), binding.synchronization_id.to_string(), root, super::authoring_kind_segment(&binding.kind),binding.item_id.to_string(),hand_off], |r| r.get(0)).map_err(|_| failure())?;
    if invalid {
        return Err(failure());
    }
    Ok(())
}

impl super::RepositoryService {
    pub(super) fn read_comment_binding(
        &self,
        root: &Path,
        id: OperationId,
    ) -> Result<Option<CommentBinding>, RepositoryError> {
        let _guard = super::cache_read_guard(
            &self.registry_path,
            root,
            RepositoryOperation::SubmitComment,
        )?;
        let connection = super::open_registry_read_only(&self.registry_path)?;
        let row = connection.query_row("SELECT b.operation_ulid,b.synchronization_ulid,b.kind,b.item_ulid,b.comment_ulid,b.parent_ulid,b.pre_checkpoint_oid,b.checkpoint_oid,b.created_at,o.root_path,b.root_digest,o.target,o.action,o.state,o.completed_step
            FROM comment_publication_bindings b JOIN operation_records o ON o.id=b.local_record_id WHERE b.operation_ulid=?1",
            [id.to_string()], |r| Ok((decode_comment_binding(r)?, r.get::<_, String>(9)?,r.get::<_, Vec<u8>>(10)?,r.get::<_, String>(11)?,r.get::<_, String>(12)?,r.get::<_, String>(13)?,r.get::<_, Option<String>>(14)?))).optional().map_err(|_| super::comment_publication::recovery_error())?;
        match row {
            Some((_, bound_root, _, _, _, _, _)) if Some(bound_root.as_str()) != root.to_str() => {
                Err(mismatch(RepositoryOperation::SubmitComment, root))
            }
            // The binding of an abandoned submission is kept with its row
            // and continues nothing: the ID is spent for its replay, its
            // publication-only retry and its cancellation alike. Each of
            // those reads the binding first and proves the checkpoint
            // from Git, so none would otherwise come to the journal's
            // own refusal.
            Some((_, _, _, _, _, state, step))
                if state == "completed" && is_abandoned_step(step.as_deref()) =>
            {
                Err(abandoned(RepositoryOperation::SubmitComment, root))
            }
            Some((binding, bound_root, digest, target, action, _, _)) => {
                let expected = format!(
                    "authoring-context-v1/{}/{}/Edit\0.manyhands/comments\0{}",
                    super::authoring_kind_segment(&binding.kind),
                    binding.item_id,
                    binding.comment_id
                );
                if action != "submit_comment"
                    || target != expected
                    || digest != blake3::hash(bound_root.as_bytes()).as_bytes().as_slice()
                {
                    return Err(super::comment_publication::recovery_error());
                }
                Ok(Some(binding))
            }
            None => Ok(None),
        }
    }

    pub(super) fn bind_comment_publication(
        &self,
        repository: &git2::Repository,
        root: &Path,
        request: &super::SubmitCommentRequest,
        record: RecoveryRecord,
    ) -> Result<CommentBinding, RepositoryError> {
        let failure = super::comment_publication::recovery_error;
        if record.id == 0 {
            return Err(failure());
        }
        let _guard = super::cache_write_guard(
            &self.registry_path,
            root,
            RepositoryOperation::SubmitComment,
        )?;
        let mut connection = super::open_registry(&self.registry_path, &mut |_| {})?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| failure())?;
        // A binding this call takes up again was audited by the migration
        // its lifecycle's open ran; that open no longer migrates.
        validate_comment_binding(&tx, &request.target.operation_id.to_string())?;
        let comment_id = request.comment_id.to_string();
        let target = super::authoring_write_target(
            &request.target,
            &[Path::new(".manyhands/comments"), Path::new(&comment_id)],
        );
        let parent_matches: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE id=?1 AND root_path=?2 AND operation_ulid=?3 AND action='submit_comment' AND target=?4)",
            params![record.id,root.to_str(),request.target.operation_id.to_string(),target], |r| r.get(0)).map_err(|_| failure())?;
        if !parent_matches {
            return Err(mismatch(RepositoryOperation::SubmitComment, root));
        }
        let existing = tx.query_row("SELECT operation_ulid,synchronization_ulid,kind,item_ulid,comment_ulid,parent_ulid,pre_checkpoint_oid,checkpoint_oid,created_at FROM comment_publication_bindings WHERE operation_ulid=?1",
            [request.target.operation_id.to_string()], decode_comment_binding).optional().map_err(|_| failure())?;
        if let Some(binding) = existing {
            binding.matches_request(request)?;
            tx.commit().map_err(|_| failure())?;
            return Ok(binding);
        }
        // Missing correlation on a completed legacy action cannot be adopted.
        if !record.is_new {
            return Err(failure());
        }
        let root_text = root.to_str().ok_or_else(failure)?;
        let digest = blake3::hash(root_text.as_bytes());
        let collision: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1)
            OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE synchronization_ulid=?1 OR (root_digest=?2 AND comment_ulid=?3))",
            params![request.target.operation_id.to_string(), digest.as_bytes().as_slice(), request.comment_id.to_string()], |r| r.get(0)).map_err(|_| failure())?;
        if collision {
            return Err(mismatch(RepositoryOperation::SubmitComment, root));
        }
        let child = OperationId::new();
        super::local_sync::record::require_foreign_id_available(&tx, child)?;
        let collision: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1)
            OR EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1)
            OR EXISTS(SELECT 1 FROM comment_publication_bindings WHERE operation_ulid=?1 OR synchronization_ulid=?1)",
            [child.to_string()], |r| r.get(0)).map_err(|_| failure())?;
        if collision {
            return Err(mismatch(RepositoryOperation::SubmitComment, root));
        }
        let branch = format!(
            "refs/heads/manyhands/{}/{}",
            super::authoring_kind_segment(&request.target.kind),
            request.target.item_id
        );
        let pre_checkpoint_oid = match repository.refname_to_id(&branch) {
            Ok(oid) => oid,
            Err(error) if error.code() == git2::ErrorCode::NotFound => repository
                .head()
                .and_then(|head| head.peel_to_commit())
                .map(|commit| commit.id())
                .map_err(|_| failure())?,
            Err(_) => return Err(failure()),
        };
        tx.execute("INSERT INTO comment_publication_bindings(operation_ulid,local_record_id,root_digest,kind,item_ulid,comment_ulid,parent_ulid,synchronization_ulid,pre_checkpoint_oid)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![request.target.operation_id.to_string(),record.id,digest.as_bytes().as_slice(),
            super::authoring_kind_segment(&request.target.kind),request.target.item_id.to_string(),request.comment_id.to_string(),request.parent_id.as_ref().map(ToString::to_string),child.to_string(),pre_checkpoint_oid.to_string()]).map_err(|_| failure())?;
        tx.commit().map_err(|_| failure())?;
        Ok(CommentBinding {
            operation_id: request.target.operation_id,
            synchronization_id: child,
            kind: request.target.kind,
            item_id: request.target.item_id.clone(),
            comment_id: request.comment_id.clone(),
            parent_id: request.parent_id.clone(),
            pre_checkpoint_oid,
            checkpoint_oid: None,
            created_at: None,
            receipt_recorded: false,
        })
    }

    pub(super) fn store_comment_receipt(
        &self,
        root: &Path,
        binding: &CommentBinding,
        oid: git2::Oid,
        time: &str,
    ) -> Result<(), RepositoryError> {
        if binding.receipt_recorded
            && binding.checkpoint_oid == Some(oid)
            && binding.created_at.as_deref() == Some(time)
        {
            return Ok(());
        }
        let _guard = super::cache_write_guard(
            &self.registry_path,
            root,
            RepositoryOperation::SubmitComment,
        )?;
        let connection = super::open_registry(&self.registry_path, &mut |_| {})?;
        let count = connection.execute("UPDATE comment_publication_bindings SET checkpoint_oid=?2,created_at=?3
            WHERE operation_ulid=?1 AND synchronization_ulid=?4 AND (created_at IS NULL OR created_at=?3) AND (checkpoint_oid IS NULL OR (checkpoint_oid=?2 AND created_at=?3))",
            params![binding.operation_id.to_string(),oid.to_string(),time,binding.synchronization_id.to_string()]).map_err(|_| super::comment_publication::recovery_error())?;
        if count != 1 {
            return Err(super::comment_publication::recovery_error());
        }
        Ok(())
    }

    pub(super) fn store_comment_creation_time(
        &self,
        root: &Path,
        binding: &CommentBinding,
        time: &str,
    ) -> Result<(), RepositoryError> {
        let _guard = super::cache_write_guard(
            &self.registry_path,
            root,
            RepositoryOperation::SubmitComment,
        )?;
        let connection = super::open_registry(&self.registry_path, &mut |_| {})?;
        let changed = connection.execute("UPDATE comment_publication_bindings SET created_at=?3 WHERE operation_ulid=?1 AND synchronization_ulid=?2 AND created_at IS NULL",
            params![binding.operation_id.to_string(),binding.synchronization_id.to_string(),time]).map_err(|_| super::comment_publication::recovery_error())?;
        if changed == 0 {
            let matches: bool = connection.query_row("SELECT created_at=?3 FROM comment_publication_bindings WHERE operation_ulid=?1 AND synchronization_ulid=?2",
                params![binding.operation_id.to_string(),binding.synchronization_id.to_string(),time], |r| r.get(0)).map_err(|_| super::comment_publication::recovery_error())?;
            if !matches {
                return Err(super::comment_publication::recovery_error());
            }
        }
        Ok(())
    }

    /// Releases the binding of a comment submission that wrote nothing
    /// (R-34): in one transaction, the binding is deleted and the row
    /// `record` is marked `rejected`. The row is kept, so the operation ID
    /// stays bound to its target, and a repeat of it begins again as new
    /// and binds afresh.
    ///
    /// It has two callers, both `checkpoint_comment_locally_with` with
    /// the binding's semantics: a submission that stopped for an identity,
    /// and one refused after its binding row was inserted, having begun no
    /// effect (controller ruling, Task 2b concern 2).
    ///
    /// The caller has found the comment file absent. Here the row must
    /// hold no step past the context's own, the binding no checkpoint, and
    /// the child synchronization no row. Otherwise nothing changes and
    /// `false` is returned: the binding and the row are the caller's to
    /// keep, as for a submission that wrote. An error is a registry that
    /// could not be read or written.
    pub(super) fn release_unwritten_comment_binding(
        &self,
        root: &Path,
        record: RecoveryRecord,
        operation_id: OperationId,
    ) -> Result<bool, RepositoryError> {
        let failure = super::comment_publication::recovery_error;
        if record.id == 0 {
            return Ok(false);
        }
        let _guard = super::cache_write_guard(
            &self.registry_path,
            root,
            RepositoryOperation::SubmitComment,
        )?;
        let mut connection = super::open_registry(&self.registry_path, &mut |_| {})?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| failure())?;
        let operation = operation_id.to_string();
        let unwritten: bool = tx
            .query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM operation_records o
                       JOIN comment_publication_bindings b ON b.local_record_id = o.id
                      WHERE o.id = ?1 AND o.operation_ulid = ?2 AND b.operation_ulid = ?2
                        AND o.action = 'submit_comment' AND {NOT_ABANDONED}
                        AND (o.completed_step IS NULL OR o.completed_step IN (?3, ?4, ?5))
                        AND b.checkpoint_oid IS NULL
                        AND NOT EXISTS(SELECT 1 FROM remote_operation_records
                                        WHERE operation_ulid = b.synchronization_ulid))"
                ),
                params![
                    record.id,
                    operation,
                    LocalStep::BranchObserved.as_str(),
                    LocalStep::WorktreeObserved.as_str(),
                    LocalStep::Completed.as_str(),
                ],
                |row| row.get(0),
            )
            .map_err(|_| failure())?;
        if !unwritten {
            return Ok(false);
        }
        let deleted = tx
            .execute(
                "DELETE FROM comment_publication_bindings WHERE operation_ulid = ?1 AND local_record_id = ?2",
                params![operation, record.id],
            )
            .map_err(|_| failure())?;
        let rejected = tx
            .execute(
                "UPDATE operation_records SET state = 'completed', completed_step = ?2, observed_at = ?3
                  WHERE id = ?1",
                params![record.id, LocalStep::Rejected.as_str(), now()],
            )
            .map_err(|_| failure())?;
        if deleted != 1 || rejected != 1 {
            return Err(failure());
        }
        tx.commit().map_err(|_| failure())?;
        Ok(true)
    }
    pub(super) fn comment_local_pending(
        &self,
        root: &Path,
        operation: OperationId,
    ) -> Result<bool, RepositoryError> {
        let _guard = super::cache_read_guard(
            &self.registry_path,
            root,
            RepositoryOperation::SubmitComment,
        )?;
        let connection = super::open_registry_read_only(&self.registry_path)?;
        connection.query_row("SELECT state!='completed' OR completed_step='comment_registration_pending' FROM operation_records WHERE operation_ulid=?1 AND root_path=?2",
            params![operation.to_string(),root.to_str()], |r| r.get(0)).map_err(|_| super::comment_publication::recovery_error())
    }

    pub(super) fn park_comment_registration_handoff(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<(), RepositoryError> {
        let _guard = super::cache_write_guard(
            &self.registry_path,
            root,
            RepositoryOperation::SubmitComment,
        )?;
        let connection = super::open_registry(&self.registry_path, &mut |_| {})?;
        connection.execute("UPDATE operation_records SET state='completed',completed_step='comment_registration_pending' WHERE operation_ulid=?1 AND root_path=?2 AND action='submit_comment'
            AND (state IN ('created','worktree_observed','comment_destination_prepared','authoring_destination_observed','comment_checkpoint_intent','authoring_checkpoint_observed','authoritative_observed','failed') OR (state='completed' AND completed_step='comment_registration_pending'))",
            params![operation_id.to_string(),root.to_str()]).map_err(|_| super::comment_publication::recovery_error())?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(super) struct RecoveryRecord {
    pub(super) id: i64,
    pub(super) is_new: bool,
    pub(super) is_pending: bool,
    #[allow(dead_code)]
    pub(super) completed_step: Option<LocalStep>,
}

#[derive(Clone)]
pub(super) struct IndexOwner {
    pub(super) record_id: i64,
    pub(super) epoch: i64,
    pub(super) retirement_request: Option<super::context_retirement::request_scope::RequestFence>,
}

impl IndexOwner {
    fn validate(&self, db: &Connection, epoch: Option<i64>) -> Result<(), RepositoryError> {
        super::context_retirement::request_scope::validate_index(
            db,
            self.record_id,
            self.retirement_request,
            epoch,
        )
    }
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
        CREATE INDEX IF NOT EXISTS operation_records_comment_registration_pending ON operation_records(root_path) WHERE completed_step='comment_registration_pending';
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
    migrate_comment_bindings(&transaction)?;
    validate_comment_bindings(&transaction)?;
    super::local_sync::record::migrate(&transaction)?;
    migrate_operation_attribution(&transaction)?;
    migrate_context_basis(&transaction)?;
    transaction.commit().map_err(RepositoryError::sqlite)
}

/// The marker of the migration that adds the attribution columns.
const ATTRIBUTION_MIGRATION: &str = "cycle_operation_attribution";

/// The columns that say whose an operation's row is, on both journals, each
/// with its type: the request that began the row and that request's
/// command, the commit identity it ran under as `Name <email>`, the host
/// and the process it ran in, and when it began, in seconds since the
/// epoch by the service's clock. All are nullable: a row a build before
/// them wrote has none, which is unknown, and none is inferred for it.
const ATTRIBUTION_COLUMNS: [(&str, &str); 6] = [
    ("request_ulid", "TEXT"),
    ("command", "TEXT"),
    ("author", "TEXT"),
    ("host", "TEXT"),
    ("pid", "INTEGER"),
    ("started_at", "INTEGER"),
];

/// The local journal also names the comment a comment's submission writes.
/// Its item is in `item_id`, which the table has always had.
const LOCAL_ATTRIBUTION_COLUMNS: [(&str, &str); 1] = [("comment_id", "TEXT")];

fn attribution_error() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::OpenRegistry,
        None,
        RepositoryErrorKind::RecoveryRequired,
        "operation attribution requires recovery",
    )
}

/// Adds the attribution columns to the two journals, after every column
/// they have, and records the marker; it runs in the recovery migration's
/// transaction, which commits or rolls back with it.
///
/// Additive and idempotent: a column already there is left as it is, and
/// no row is written. A registry that has the marker is only read, as the
/// comment bindings' audit is: a journal of it that lacks a column was
/// replaced, and is not repaired here. The marker is recorded only once
/// both journals have the columns, so a registry whose remote journal has
/// not been created yet gains them when it is.
///
/// A SQLite failure is reported as one, as the migrations around this one
/// do: a conflict with another process opening the same registry
/// (`SQLITE_BUSY`) is transient and a retry succeeds, so it must not read as
/// damage. Only a marked registry that lacks a column is `recovery_required`.
fn migrate_operation_attribution(db: &Connection) -> Result<(), RepositoryError> {
    let failure = RepositoryError::sqlite;
    let migrated: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [ATTRIBUTION_MIGRATION],
            |row| row.get(0),
        )
        .map_err(failure)?;
    let mut complete = true;
    for (table, own) in [
        ("operation_records", &LOCAL_ATTRIBUTION_COLUMNS[..]),
        (REMOTE_JOURNAL_TABLE, &[][..]),
    ] {
        if !has_table(db, table).map_err(failure)? {
            if migrated {
                return Err(attribution_error());
            }
            complete = false;
            continue;
        }
        let present = db
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .and_then(|mut statement| {
                statement
                    .query_map([table], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(failure)?;
        for (name, definition) in ATTRIBUTION_COLUMNS.iter().chain(own) {
            if present.iter().any(|column| column == name) {
                continue;
            }
            if migrated {
                return Err(attribution_error());
            }
            db.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {name} {definition}"
            ))
            .map_err(failure)?;
        }
    }
    if !migrated && complete {
        db.execute(
            "INSERT INTO registry_migrations(name) VALUES(?1)",
            [ATTRIBUTION_MIGRATION],
        )
        .map_err(failure)?;
    }
    Ok(())
}

/// The marker of the migration that adds the context basis columns.
const CONTEXT_BASIS_MIGRATION: &str = "cycle_context_basis";

/// What a context row records of its last indexing, each with its type: the
/// observation it was stored from as JSON, which none is for a row stored
/// before the column (unknown, so the next whole refresh observes the
/// context again); when that observation began, in nanoseconds since the
/// epoch; whether the context is owed an indexing; and, for the primary
/// only, the head its activity map was built for.
///
/// `dirty` is clean for a row inserted without it: a build before the column
/// stores a context only after a whole observation, and its missing basis
/// makes the next refresh observe it again. The migration marks the rows
/// that exist when it runs.
const CONTEXT_BASIS_COLUMNS: [(&str, &str); 4] = [
    ("basis", "TEXT"),
    ("indexed_at", "INTEGER"),
    (
        "dirty",
        "INTEGER NOT NULL DEFAULT 0 CHECK (dirty IN (0, 1))",
    ),
    ("activity_base", "TEXT"),
];

/// The contexts a row's operation owes an indexing, as a JSON array of
/// encoded context keys, written when the row is made. NULL is unknown, and
/// is owed a whole refresh of the repository.
const OWED_CONTEXTS_COLUMNS: [(&str, &str); 1] = [("owed_contexts", "TEXT")];

/// Adds the context basis columns to the contexts and to the local journal,
/// and marks every context the registry holds dirty, once; it runs in the
/// recovery migration's transaction, which commits or rolls back with it.
///
/// The same pattern as the attribution migration: additive, idempotent, and
/// the marker is recorded only once both tables have the columns. Unlike
/// it, a marked registry that lacks a column is not refused: the contexts
/// are an index, which a refresh builds again, so a column that is missing is
/// added. When that is `dirty`, what it held is unknown, so every context
/// is marked dirty again.
///
/// A SQLite failure is reported as one, as the migrations around this one do.
fn migrate_context_basis(db: &Connection) -> Result<(), RepositoryError> {
    let failure = RepositoryError::sqlite;
    let migrated: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CONTEXT_BASIS_MIGRATION],
            |row| row.get(0),
        )
        .map_err(failure)?;
    let mut complete = true;
    let mut dirty_added = false;
    for (table, columns) in [
        ("contexts", &CONTEXT_BASIS_COLUMNS[..]),
        ("operation_records", &OWED_CONTEXTS_COLUMNS[..]),
    ] {
        if !has_table(db, table).map_err(failure)? {
            complete = false;
            continue;
        }
        let present = db
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .and_then(|mut statement| {
                statement
                    .query_map([table], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(failure)?;
        for (name, definition) in columns {
            if present.iter().any(|column| column == name) {
                continue;
            }
            db.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {name} {definition}"
            ))
            .map_err(failure)?;
            dirty_added |= *name == "dirty";
        }
    }
    if complete && (!migrated || dirty_added) {
        db.execute("UPDATE contexts SET dirty = 1", [])
            .map_err(failure)?;
    }
    if complete && !migrated {
        db.execute(
            "INSERT INTO registry_migrations(name) VALUES(?1)",
            [CONTEXT_BASIS_MIGRATION],
        )
        .map_err(failure)?;
    }
    Ok(())
}

/// The process that holds a row: this one, on this host. A row records
/// it when it is begun, and again whenever a call takes it over: a resume,
/// an index claim, a restart. Only these two columns move then; the
/// request that began the row, its command, the author and the start are
/// left as they were, and keep naming where the row came from.
pub(super) struct Holder {
    pub(super) host: String,
    pub(super) pid: i64,
}

impl Holder {
    /// This process. Reading it never fails: a host name that cannot be
    /// read is `unknown`.
    pub(super) fn this_process() -> Self {
        Self {
            host: super::liveness::this_host(),
            pid: i64::from(std::process::id()),
        }
    }
}

/// Whose a row is as it is begun: this process, on this host, at
/// `started_at` by the service's clock, under the commit identity the
/// repository's configuration gives. Reading it never fails: what cannot be
/// read is recorded as unknown.
pub(super) struct RowOrigin {
    holder: Holder,
    started_at: i64,
    author: Option<String>,
}

impl RowOrigin {
    /// This process, beginning a row of the repository at `root` at
    /// `started_at`.
    pub(super) fn here(root: &Path, started_at: i64) -> Self {
        Self {
            holder: Holder::this_process(),
            started_at,
            author: configured_author(root),
        }
    }

    /// The same origin, naming no author.
    pub(super) fn unauthored(self) -> Self {
        Self {
            author: None,
            ..self
        }
    }

    /// The author a row records whose request is of `command`: none for a
    /// promotion's or a closure's (Cycle 10: the identity whoever closed
    /// or promoted commits with is kept by the index alone), the
    /// configured one for any other.
    pub(super) fn author_for(&self, command: Option<&str>) -> Option<&str> {
        if command.is_some_and(|command| LIFECYCLE_COMMANDS.contains(&command)) {
            None
        } else {
            self.author.as_deref()
        }
    }
}

/// The commands of a promotion and a closure, as their records name them.
const LIFECYCLE_COMMANDS: [&str; 2] = ["document promote", "ticket close"];

/// Records on the local row `id` that `holder` holds it now. Nothing else of
/// the row changes. A row an index owner holds is left to its claim, which
/// records the holder itself, so a call that only finds the row is never
/// named in the owner's place.
pub(super) fn hold_local_row(db: &Connection, id: i64, holder: &Holder) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE operation_records SET host = ?2, pid = ?3 WHERE id = ?1 AND state != 'indexing'",
        params![id, holder.host, holder.pid],
    )?;
    Ok(())
}

/// The commit identity the repository at `root` commits with, as the text
/// a comment's `created_by` is given: `Name <email>`. It is resolved as
/// `repo identity` and a commit resolve it, from the repository's own
/// configuration, its local level first; `None` when there is none or Git
/// cannot say.
fn configured_author(root: &Path) -> Option<String> {
    let repository = git2::Repository::open(root).ok()?;
    let config = repository.config().ok()?;
    let (identity, _) =
        super::resolve_identity(&config, &config, root, RepositoryOperation::Read).ok()??;
    Some(format!("{} <{}>", identity.name, identity.email))
}

/// The request whose record holds the operation `operation_id` of
/// `family`, and its command: the request that began it, since each
/// acceptance is given an operation ID of its own. One still accepted is
/// preferred to one that has finished. `None` when no record holds it, or
/// the request tables are not there.
fn linked_request(
    db: &Connection,
    family: &str,
    operation_id: &str,
) -> rusqlite::Result<Option<(String, String)>> {
    if !has_table(db, "request_operations")? || !has_table(db, "request_records")? {
        return Ok(None);
    }
    db.query_row(
        "SELECT r.request_ulid, r.command FROM request_operations m
           JOIN request_records r ON r.request_ulid = m.request_ulid
          WHERE m.family = ?1 AND m.operation_ulid = ?2
          ORDER BY r.state = 'accepted' DESC, r.accepted_at DESC, r.request_ulid
          LIMIT 1",
        params![family, operation_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

/// Records on the local row `id` whose it is: `origin`, and the request
/// that began it with its command. That is the request whose record holds
/// the row's operation; else, for a row begun under a remote operation's
/// ID, as a synchronization's index hand-off is, the request that remote
/// row names, which has already resolved a lifecycle's or a comment's.
/// A row begun again, after it was rejected, is a fresh begin and the new
/// call's: its host, process and start are replaced, and a request it
/// already names is kept when none holds it now.
pub(super) fn attribute_local_row(
    db: &Connection,
    id: i64,
    origin: &RowOrigin,
) -> rusqlite::Result<()> {
    let operation: Option<String> = db.query_row(
        "SELECT operation_ulid FROM operation_records WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let mut request = match &operation {
        Some(operation) => linked_request(db, "local", operation)?,
        None => None,
    };
    if let Some(operation) = &operation
        && request.is_none()
        && has_table(db, REMOTE_JOURNAL_TABLE)?
    {
        request = db
            .query_row(
                "SELECT request_ulid, command FROM remote_operation_records
                  WHERE operation_ulid = ?1 AND request_ulid IS NOT NULL AND command IS NOT NULL
                  ORDER BY id DESC LIMIT 1",
                [operation],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
    }
    let (request_id, command) = request.unzip();
    let author = origin.author_for(command.as_deref());
    db.execute(
        "UPDATE operation_records
            SET request_ulid = COALESCE(?2, request_ulid), command = COALESCE(?3, command),
                author = COALESCE(?4, author), host = ?5, pid = ?6, started_at = ?7
          WHERE id = ?1",
        params![
            id,
            request_id,
            command,
            author,
            origin.holder.host,
            origin.holder.pid,
            origin.started_at
        ],
    )?;
    Ok(())
}

/// Records on the remote row `id` whose it is: `origin`, and the request
/// that began it with its command. That is the request whose record holds
/// the operation; else, for the child of a promotion or a closure, the one
/// whose record holds that lifecycle (decision 34); else, for the
/// synchronization a comment's binding keeps, the request the comment's
/// own row names, so the publication is the comment's request's.
pub(super) fn attribute_remote_row(
    db: &Connection,
    id: i64,
    origin: &RowOrigin,
) -> rusqlite::Result<()> {
    let (repository_id, operation): (i64, String) = db.query_row(
        "SELECT repository_id, operation_ulid FROM remote_operation_records WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut request = linked_request(db, "remote", &operation)?;
    let mut lifecycle_child = false;
    for lifecycle in ["document_promotions", "ticket_closures"] {
        if request.is_some() || !has_table(db, lifecycle)? {
            continue;
        }
        let parent: Option<String> = db
            .query_row(
                &format!(
                    "SELECT operation_id FROM {lifecycle}
                      WHERE repository_id = ?1 AND (context_sync_id = ?2 OR primary_sync_id = ?2)
                      LIMIT 1"
                ),
                params![repository_id, operation],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(parent) = parent {
            lifecycle_child = true;
            request = linked_request(db, "remote", &parent)?;
        }
    }
    if request.is_none() && has_table(db, "comment_publication_bindings")? {
        request = db
            .query_row(
                "SELECT o.request_ulid, o.command FROM comment_publication_bindings b
                   JOIN operation_records o ON o.id = b.local_record_id
                  WHERE b.synchronization_ulid = ?1
                    AND o.request_ulid IS NOT NULL AND o.command IS NOT NULL",
                [&operation],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
    }
    let (request_id, command) = request.unzip();
    // A lifecycle's child is the lifecycle's, whatever request it is linked
    // to.
    let author = if lifecycle_child {
        None
    } else {
        origin.author_for(command.as_deref())
    };
    db.execute(
        "UPDATE remote_operation_records
            SET request_ulid = ?2, command = ?3, author = ?4, host = ?5, pid = ?6,
                started_at = ?7
          WHERE id = ?1",
        params![
            id,
            request_id,
            command,
            author,
            origin.holder.host,
            origin.holder.pid,
            origin.started_at
        ],
    )?;
    Ok(())
}

/// The actions whose rows are each of one item: the authoring operations,
/// which write the item's editing context and nothing of the primary
/// worktree, of any other item's context or of the repository's
/// configuration. A row of one of them that names its item holds that item,
/// and while it is mid-write the work of the whole repository; see `Scope`
/// and `holds`.
///
/// A local synchronization names its item too and is not one of them: its
/// index step rewrites the whole index without an index claim, and its
/// resume asks after no other row. Nor is a retirement, which removes a
/// context and claims the index for its request.
const ITEM_ACTIONS: [&str; 6] = [
    "save_ticket",
    "save_document",
    "submit_comment",
    "repair_item",
    "prepare_context",
    "release_context",
];

/// What the target of a synchronization's index hand-off row begins with
/// (P-4): `synchronization-index-v1/<kind>/<item>/<oid>` for a context,
/// `synchronization-index-v1/primary/<oid>` for the primary, `<oid>` being
/// the commit the synchronization published or found current. The row is a
/// `refresh` under the synchronization's ID, of that item or of the primary
/// (`Scope`), and is reported as a refresh. Older builds wrote it with no
/// target, of the whole repository, and it is still read so.
const SYNCHRONIZATION_INDEX_TARGET: &str = "synchronization-index-v1/";

/// The target of the index hand-off row of a synchronization of `target`
/// that published, or found current, `oid`.
pub(super) fn synchronization_index_target(
    target: &super::SynchronizationTarget,
    oid: git2::Oid,
) -> String {
    match target {
        super::SynchronizationTarget::Primary => hand_off_target("primary", &oid.to_string()),
        super::SynchronizationTarget::Context { kind, item_id } => hand_off_target(
            &format!("{}/{item_id}", super::authoring_kind_segment(kind)),
            &oid.to_string(),
        ),
    }
}

/// The hand-off target of what a synchronization is `of` (`primary`, or
/// `<kind>/<item>`) at `oid`, as `synchronization_index_target` spells it.
fn hand_off_target(of: &str, oid: &str) -> String {
    format!("{SYNCHRONIZATION_INDEX_TARGET}{of}/{oid}")
}

/// The kinds of item a hand-off target names, as it spells them.
const HAND_OFF_KINDS: [super::AuthoringKind; 2] =
    [super::AuthoringKind::Document, super::AuthoringKind::Ticket];

/// The item a synchronization's index hand-off target of an item names:
/// what follows `synchronization-index-v1/<kind>/`, up to the next `/`.
/// `None` for a hand-off of the primary and for any other target. The
/// commit after it is not read here: the reservation checks the whole
/// target against its synchronization's (`validate_index_identity`), and so
/// does a comment's binding (`comment_child_may_begin`).
fn synchronization_index_item(target: &str) -> Option<crate::canonical::ItemId> {
    let (kind, rest) = target
        .strip_prefix(SYNCHRONIZATION_INDEX_TARGET)?
        .split_once('/')?;
    if !HAND_OFF_KINDS
        .iter()
        .any(|known| super::authoring_kind_segment(known) == kind)
    {
        return None;
    }
    rest.split_once('/')?.0.parse().ok()
}

/// The item a row of `action` that names `item_id` and was begun with
/// `target` is a synchronization's index hand-off of, by
/// `ContextKey::encode`: `primary` for a hand-off of the primary, whatever
/// item the row names; else the item the row names, when its target is a
/// hand-off of that item. `None` for any other row. Only the beginning of
/// the target is read, as `scope_sql` reads it.
fn synchronization_hand_off_of(
    action: &str,
    item_id: Option<&str>,
    target: Option<&str>,
) -> Option<String> {
    let rest = target?.strip_prefix(SYNCHRONIZATION_INDEX_TARGET)?;
    if action != "refresh" {
        return None;
    }
    if rest.starts_with("primary/") {
        return Some(super::discovery::ContextKey::Primary.encode());
    }
    let item = item_id?;
    HAND_OFF_KINDS
        .iter()
        .any(|kind| rest.starts_with(&format!("{}/{item}/", super::authoring_kind_segment(kind))))
        .then(|| item.to_owned())
}

/// The item and the comment a local row names, read from the target it is
/// begun with: `authoring-context-v1/<kind>/<item>/<intent>`, then the
/// paths, each after a NUL; a comment's are `.manyhands/comments` and its
/// ID. A row of one of the `ITEM_ACTIONS` names its item, and only a
/// comment's submission names its comment. A synchronization's index
/// hand-off of an item names that item (P-4); one of the primary names
/// none.
fn authored_ids(action: &str, target: &str) -> (Option<String>, Option<String>) {
    if action == "refresh" {
        return (
            synchronization_index_item(target).map(|item| item.to_string()),
            None,
        );
    }
    if !ITEM_ACTIONS.contains(&action) {
        return (None, None);
    }
    let item = super::bridges::abandon::authoring_item(Some(target));
    let comment = if action == "submit_comment" {
        match target.split('\0').collect::<Vec<_>>()[..] {
            [_, ".manyhands/comments", comment] => comment
                .parse::<crate::canonical::ItemId>()
                .ok()
                .map(|comment| comment.to_string()),
            _ => None,
        }
    } else {
        None
    };
    (item, comment)
}

/// The `owed_contexts` a row of `action` begun with `target` records when it
/// is begun: the context of the item it is of (`Scope::of_call`), which its
/// index step indexes once its work is done. For one of the `ITEM_ACTIONS`
/// that is its item's, which reads as owed an indexing before the write
/// commits to it; for a synchronization's index hand-off, the context the
/// synchronization changed or published, the item's or the primary (P-4). A
/// row of the whole repository owes no context of its own, so it records
/// nothing (NULL), and its hand-off is a refresh of the whole repository.
fn owed_at_creation(action: &str, target: &str) -> Option<String> {
    let Scope::Item(own) = Scope::of_call(action, target) else {
        return None;
    };
    Some(super::encode_owed_contexts(&[
        super::discovery::ContextKey::decode(&own)?,
    ]))
}

/// The state and last step a new row of `action` begun with `target` is
/// written with: a synchronization's index hand-off is begun once its
/// synchronization is classified, its work done and recorded, so it owes
/// only its hand-off from the first (`is_committed_and_owing`); every other
/// row is `created`, with no step.
fn begun_standing(action: &str, target: &str) -> (&'static str, Option<&'static str>) {
    let (item, _) = authored_ids(action, target);
    if synchronization_hand_off_of(action, item.as_deref(), Some(target)).is_some() {
        let step = LocalStep::AuthoritativeObserved.as_str();
        (step, Some(step))
    } else {
        ("created", None)
    }
}

/// The contexts the row `record_id` records it owes an indexing. `None` for
/// a row that records none (NULL), and for one whose list this build cannot
/// read: both are handed off to a refresh of the whole repository.
pub(super) fn recorded_owed_contexts(
    connection: &Connection,
    record_id: i64,
) -> rusqlite::Result<super::OwedContexts> {
    let owed: Option<rusqlite::types::Value> = connection
        .query_row(
            "SELECT owed_contexts FROM operation_records WHERE id = ?1",
            [record_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(match owed {
        Some(rusqlite::types::Value::Text(owed)) => super::decode_owed_contexts(&owed),
        _ => None,
    })
}

/// Refuses `call`, a call of the repository `repository_id` that begins
/// no local row of its own (a remote operation's reservation, a
/// retirement), while a pending local row `holds` it: remote operations and
/// retirements cannot consume, replace or reinterpret local recovery. A
/// row of the call's own item refuses it; see `holds` for the others.
pub(super) fn require_no_pending_local(
    connection: &Connection,
    repository_id: i64,
    call: &CallScope,
) -> Result<(), RepositoryError> {
    let holds = holds_sql(call, "");
    let pending: bool = connection.query_row(&format!("SELECT EXISTS(SELECT 1 FROM operation_records WHERE root_path=(SELECT root_path FROM repositories WHERE id=?1) AND state!='completed' AND {holds})
        OR EXISTS(SELECT 1 FROM operation_records WHERE root_path=(SELECT root_path FROM repositories WHERE id=?1) AND completed_step='comment_registration_pending' AND {holds})"),
        [repository_id], |row| row.get(0)).map_err(|_| super::remote::state::recovery_required())?;
    if pending {
        return Err(
            super::remote::state::recovery_required().with_blocker(first_pending_local(
                connection,
                repository_id,
                &holds,
            )),
        );
    }
    Ok(())
}

/// Refuses `call`, a new synchronization of the repository
/// `repository_id`, while a remote operation's index debt `holds` it
/// (`holds_remote_index_debt_sql`): a classified synchronization of the
/// same item that owes its index step and has no local hand-off row yet
/// (design decision 19, question 3). Once its hand-off row is written,
/// `require_no_pending_local` holds the call for it as well, naming that
/// row. The refusal names the remote operation.
pub(super) fn require_no_remote_index_debt(
    connection: &Connection,
    repository_id: i64,
    call: &CallScope,
) -> Result<(), RepositoryError> {
    let debt: Option<String> = connection
        .query_row(
            &format!(
                "SELECT operation_ulid FROM remote_operation_records
                  WHERE repository_id = ?1 AND {}
                  ORDER BY id LIMIT 1",
                holds_remote_index_debt_sql(call, "")
            ),
            [repository_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| super::remote::state::recovery_required())?;
    match debt {
        Some(operation_id) => {
            Err(super::remote::state::recovery_required()
                .with_blocker(Blocker::remote(&operation_id)))
        }
        None => Ok(()),
    }
}

/// The operation of the oldest local row of the repository `repository_id`
/// that `require_no_pending_local` is refused for, those `holds` selects,
/// read by the two indexes the refusal itself reads. `None` when it names
/// no operation or cannot be read: the refusal stands without a name.
fn first_pending_local(
    connection: &Connection,
    repository_id: i64,
    holds: &str,
) -> Option<Blocker> {
    let first = |condition: &str| -> Option<(i64, Option<String>)> {
        connection
            .query_row(
                &format!(
                    "SELECT id, operation_ulid FROM operation_records
                      WHERE root_path=(SELECT root_path FROM repositories WHERE id=?1)
                        AND {condition} AND {holds}
                      ORDER BY id LIMIT 1"
                ),
                [repository_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .ok()
            .flatten()
    };
    let pending = first("state!='completed'");
    let registering = first("completed_step='comment_registration_pending'");
    let (_, operation_id) = match (pending, registering) {
        (Some(pending), Some(registering)) => std::cmp::min(pending, registering),
        (Some(row), None) | (None, Some(row)) => row,
        (None, None) => return None,
    };
    Blocker::local(operation_id.as_deref())
}

/// The tagged local binding of a synchronization with no remote to publish
/// to, under its index hand-off target (P-4), cannot share an ID with any
/// remote row. The collision check belongs in the insertion transaction, not
/// prior inspection.
pub(super) fn begin_or_reconcile_local_synchronization(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
    started_at: i64,
) -> Result<RecoveryRecord, RepositoryError> {
    begin_or_reconcile(
        connection,
        root,
        operation,
        operation_id,
        target,
        true,
        CallReach::OwnItem,
        RowOrigin::here(root, started_at),
    )
}

/// Begins the row of `operation_id`, or finds the one it has. A row that
/// is begun, new or again after it was rejected, records whose it is: this
/// process and host, `started_at` by the service's clock, the commit
/// identity, and the request that holds the operation; see `RowOrigin`.
pub(super) fn begin_or_reconcile_operation(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
    started_at: i64,
) -> Result<RecoveryRecord, RepositoryError> {
    begin_or_reconcile_operation_reaching(
        connection,
        root,
        operation,
        operation_id,
        target,
        started_at,
        CallReach::OwnItem,
    )
}

/// `begin_or_reconcile_operation`, for a call that reached as far as
/// `reach` says before it began; see `CallReach`.
pub(super) fn begin_or_reconcile_operation_reaching(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
    started_at: i64,
    reach: CallReach,
) -> Result<RecoveryRecord, RepositoryError> {
    begin_or_reconcile(
        connection,
        root,
        operation,
        operation_id,
        target,
        false,
        reach,
        RowOrigin::here(root, started_at),
    )
}

/// `begin_or_reconcile_operation`, recording no author: for a row a
/// promotion or a closure begins under an ID of its own, such as the
/// refresh that hands its result to the index. The identity it commits
/// with is whoever closed or promoted, which only the index keeps.
pub(super) fn begin_or_reconcile_unauthored_operation(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
    started_at: i64,
) -> Result<RecoveryRecord, RepositoryError> {
    begin_or_reconcile(
        connection,
        root,
        operation,
        operation_id,
        target,
        false,
        CallReach::OwnItem,
        RowOrigin::here(root, started_at).unauthored(),
    )
}

#[allow(clippy::too_many_arguments)] // The one begin every entry point shares.
fn begin_or_reconcile(
    connection: &mut Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
    tagged_local_binding: bool,
    reach: CallReach,
    // Read before the registry transaction begins, so no Git configuration
    // is read while it holds the database's write lock.
    origin: RowOrigin,
) -> Result<RecoveryRecord, RepositoryError> {
    let root_path = root.to_str().ok_or_else(|| invalid_path(operation, root))?;
    let action = action_name(operation);
    let requested = operation_id.to_string();
    #[cfg(test)]
    pause_before_begin_for_testing_if_installed(&requested);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(RepositoryError::sqlite)?;
    if action == "refresh"
        && (!super::local_sync::record::refresh_allowed(&transaction, &requested, root)?
            || !super::context_retirement::record::refresh_allowed(&transaction, &requested, root)?)
    {
        return Err(recovery_required(operation, root));
    }
    if action != "refresh" {
        super::local_sync::record::require_foreign_id_available(&transaction, operation_id)?;
    }
    // Both parent journals: promotion and closure IDs are never local ones.
    super::promotion::record::validate_local_identity(
        &transaction,
        root_path,
        action,
        &requested,
        target,
    )?;
    super::closure::record::validate_local_identity(
        &transaction,
        root_path,
        action,
        &requested,
        target,
    )?;
    if !comment_child_may_begin(&transaction, root_path, action, &requested, target)? {
        return Err(mismatch(operation, root));
    }
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
    // An automatic poll yields to local work: request it and refuse
    // retryably. Any other active remote operation is refused below.
    if let Some(id) = active_automatic_poll(&transaction, root_path)? {
        transaction.execute("UPDATE remote_operation_records SET yield_requested=1,updated_at=max(updated_at,?2) WHERE id=?1", params![id, time::OffsetDateTime::now_utc().unix_timestamp().max(0)])
            .map_err(|_| super::remote::state::recovery_required())?;
        // Commit the request before returning a retryable error. No local
        // journal row has been inserted and no Git effect has begun.
        transaction.commit().map_err(RepositoryError::sqlite)?;
        return Err(poll_yielding(operation));
    }
    // Read here, where it always was, and acted on below: a row of the
    // requested ID that is of another root or was abandoned is answered
    // as that before a pending row of another operation is.
    let pending = in_the_way(
        &transaction,
        root_path,
        action,
        &requested,
        target,
        tagged_local_binding,
        reach,
        false,
    )?;
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

    match pending {
        Pending::Another(blocker) => {
            return Err(recovery_required(operation, root).with_blocker(blocker));
        }
        Pending::Own(id) => {
            // Taken up by this call, which every caller makes under the
            // repository's lease: the row is held by this process now.
            hold_local_row(&transaction, id, &origin.holder).map_err(RepositoryError::sqlite)?;
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
        Pending::Nothing => {}
    }
    if let Some((id, _, _, _, _, completed_step)) = existing {
        // A refresh may borrow any operation ID; it must not erase the marker.
        if listed(completed_step.as_deref()) == Some(LocalStep::Rejected) && action != "refresh" {
            // The earlier call left nothing behind, so this one begins again.
            // Its row was kept so that the ID stays bound to its target. It
            // is a fresh begin: it is attributed anew, as a new row is.
            transaction
                .execute(
                    "UPDATE operation_records SET state = 'created', completed_step = NULL, observed_at = ?2,
                            owed_contexts = ?3
                      WHERE id = ?1",
                    params![id, now(), owed_at_creation(action, target)],
                )
                .map_err(RepositoryError::sqlite)?;
            attribute_local_row(&transaction, id, &origin).map_err(RepositoryError::sqlite)?;
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
    let (item_id, comment_id) = authored_ids(action, target);
    let owed = owed_at_creation(action, target);
    let (state, step) = begun_standing(action, target);
    // `observed_at` is the time of the begin, after whatever work of its
    // own a row records as done from the first: a synchronization's push
    // or local update is done before its hand-off row is begun (design
    // decision 32).
    transaction
        .execute(
            "INSERT INTO operation_records (repository_id, root_path, operation_ulid, action, target, item_id, comment_id, state, completed_step, observed_at, owed_contexts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                repository_id,
                root_path,
                requested,
                action,
                target,
                item_id,
                comment_id,
                state,
                step,
                now(),
                owed
            ],
        )
        .map_err(RepositoryError::sqlite)?;
    let id = transaction.last_insert_rowid();
    attribute_local_row(&transaction, id, &origin).map_err(RepositoryError::sqlite)?;
    transaction.commit().map_err(RepositoryError::sqlite)?;
    Ok(RecoveryRecord {
        id,
        is_new: true,
        is_pending: false,
        completed_step: None,
    })
}

/// Declares `LocalStep`, `LocalStep::ALL` and `LocalStep::as_str` from one
/// list, so a step cannot be declared and left out of `ALL`, which
/// `parse` reads.
macro_rules! local_steps {
    ($($(#[$doc:meta])+ $variant:ident => $name:literal;)+) => {
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
            $($(#[$doc])+ $variant),+
        }

        impl LocalStep {
            /// Every step, in the order of the declaration.
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            /// The name as the journal stores it.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),+
                }
            }
        }
    };
}

local_steps! {
    /// The branch of an authoring context was created or found.
    BranchObserved => "branch_observed";
    /// The worktree of an authoring context was created or found.
    WorktreeObserved => "worktree_observed";
    /// A release removed the worktree of a context, and has its branch
    /// still to delete.
    ContextWorktreeReleased => "context_worktree_released";
    /// A release deleted the branch of a context: the context is gone,
    /// and all the release still owes is its index hand-off. It commits
    /// nothing.
    ContextBranchReleased => "context_branch_released";
    /// Retirement observed initial absence, not an owned removal or checkpoint.
    ContextRetirementAbsent => "context_retirement_absent";
    /// Confirmed preservation forbids further removal and owes local discovery.
    ContextRetirementPreserved => "context_retirement_preserved";
    /// A comment's creation time is recorded and its file is about to be
    /// written.
    CommentDestinationPrepared => "comment_destination_prepared";
    /// A comment's checkpoint is about to be committed: from here the
    /// commit may exist, and only Git says whether it does.
    CommentCheckpointIntent => "comment_checkpoint_intent";
    /// A comment is committed in a repository that is not registered,
    /// and its index hand-off waits for the registration. The row is
    /// completed and still owes that hand-off.
    CommentRegistrationPending => "comment_registration_pending";
    /// An authoring write reached its destination file.
    AuthoringDestinationObserved => "authoring_destination_observed";
    /// A document write reached its destination file.
    DocumentDestinationObserved => "document_destination_observed";
    /// A document move removed its source file.
    DocumentMoveObserved => "document_move_observed";
    /// An authoring operation passed its checkpoint, whether the
    /// checkpoint committed or found nothing to commit.
    AuthoringCheckpointObserved => "authoring_checkpoint_observed";
    /// The operation's work is observed in the authoritative branch and
    /// its index hand-off has begun.
    AuthoritativeObserved => "authoritative_observed";
    /// A remote was added or removed.
    RemoteChanged => "remote_changed";
    /// The publication remote was written to the configuration file.
    PublicationConfigurationWritten => "publication_configuration_written";
    /// The publication remote's configuration was committed.
    PublicationCommitted => "publication_committed";
    /// A new repository was initialized.
    RepositoryInitialized => "repository_initialized";
    /// The Manyhands configuration file was written.
    ConfigurationWritten => "configuration_written";
    /// The Manyhands configuration was committed.
    InitializationCommitted => "initialization_committed";
    /// A rebuild or refresh was left to be run again.
    Retry => "retry";
    /// A rebuild failed.
    Error => "error";
    /// The row completed having recorded no step of its own.
    Completed => "completed";
    /// The row of a call that was rejected and left nothing behind. A
    /// repeat of its ID begins again.
    Rejected => "rejected";
    /// The row was closed by `abandon_operation`. Its ID is never begun
    /// again and never replayed.
    Abandoned => "abandoned";
    /// Frozen local context inputs, before any Git effect.
    LocalSyncPrepared => "local_sync_prepared";
    /// A pull-request setup was recorded, and its repository registered
    /// with no configuration, in one registry transaction. The setup
    /// request re-enters through the record, not through this step.
    SetupRecorded => "setup_recorded";
    /// A setup cancellation was recorded on its own row. The cancellation
    /// re-enters through the record, not through this step.
    SetupCancelRecorded => "setup_cancel_recorded";
}

impl LocalStep {
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
            | Self::PublicationCommitted
            // From the intent on the commit may exist, and a step here
            // never says more than that. With the registration pending
            // it does exist.
            | Self::CommentCheckpointIntent
            | Self::CommentRegistrationPending
            // A recorded setup is the point a re-entered setup request
            // takes up as recorded.
            | Self::SetupRecorded
            | Self::SetupCancelRecorded => true,
            Self::BranchObserved
            | Self::LocalSyncPrepared
            | Self::WorktreeObserved
            // A release removes what holds no work, and commits nothing.
            | Self::ContextWorktreeReleased
            | Self::ContextBranchReleased
            | Self::ContextRetirementAbsent
            | Self::ContextRetirementPreserved
            | Self::CommentDestinationPrepared
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
    /// recorded: a checkpoint step, the step of a remote that was added
    /// or removed, or that of a release that deleted its context's
    /// branch. Neither of the last two commits anything.
    ///
    /// Not the intent to checkpoint a comment: whether its commit was
    /// made is still to be proved from Git, which the submission's own
    /// retry does and a refresh does not. Nor a release that has removed
    /// only its worktree: its branch is still to delete, which a refresh
    /// does not do.
    fn is_authoritative(self) -> bool {
        (self.is_checkpoint() && self != Self::CommentCheckpointIntent)
            || self == Self::RemoteChanged
            || self == Self::ContextBranchReleased
            || self == Self::ContextRetirementAbsent
            || self == Self::ContextRetirementPreserved
    }

    /// The step as a call under the same ID is told it when the call
    /// finds the row: the steps a domain call continues from. `None` for
    /// a step no call reads back.
    const fn continued(self) -> Option<Self> {
        match self {
            Self::CommentDestinationPrepared
            | Self::CommentCheckpointIntent
            | Self::CommentRegistrationPending
            | Self::AuthoringDestinationObserved
            | Self::DocumentDestinationObserved
            | Self::DocumentMoveObserved
            | Self::AuthoringCheckpointObserved
            | Self::AuthoritativeObserved
            | Self::RemoteChanged
            | Self::PublicationCommitted
            // Enablement repairs an index write interrupted between its
            // configuration commit and recording InitializationCommitted.
            | Self::ConfigurationWritten
            | Self::InitializationCommitted
            // A release goes on from the removal it recorded: to the
            // branch, or to the hand-off.
            | Self::ContextWorktreeReleased
            | Self::ContextBranchReleased
            | Self::ContextRetirementAbsent
            | Self::ContextRetirementPreserved => Some(self),
            Self::BranchObserved
            | Self::LocalSyncPrepared
            | Self::WorktreeObserved
            | Self::PublicationConfigurationWritten
            | Self::RepositoryInitialized
            | Self::Retry
            | Self::Error
            | Self::Completed
            | Self::Rejected
            | Self::Abandoned
            // The setup bindings re-enter through the record.
            | Self::SetupRecorded
            | Self::SetupCancelRecorded => None,
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
/// has claimed it, the explicit fence when its observations are invalidated,
/// and the two a hand-off that did not bring the index up to date leaves.
const HAND_OFF_STATES: [&str; 5] = [
    "authoritative_observed",
    INDEXING_STATE,
    "index_invalidated",
    "failed",
    "retry",
];

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

/// Whether a row in `state` whose last step is `step` is one that is
/// `completed` and has not ended: a comment that was committed while its
/// repository had no registration, whose index hand-off waits for one.
/// No synchronization is reserved while such a row stands, so whoever
/// reads a row's state for itself asks this, as it asks
/// `is_abandoned_step`, before it takes `completed` to mean that
/// nothing is owed.
pub(super) fn waits_for_registration(state: &str, step: Option<&str>) -> bool {
    state == "completed" && listed(step) == Some(LocalStep::CommentRegistrationPending)
}

/// What a statement holds of a row that has not ended: it is not
/// `completed`, or it is and waits for its registration; see
/// `waits_for_registration`.
const NOT_ENDED: &str =
    "(state != 'completed' OR completed_step IS 'comment_registration_pending')";

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
    /// pending or whose reconciliation is recorded as required, or a
    /// comment whose index hand-off waits for its repository's
    /// registration. Such a
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

/// Whether the local operation `requested` of `action` and `target` may
/// begin on `root_path` as far as comment publication is concerned.
///
/// The ID a comment's binding keeps for its synchronization is that
/// synchronization's alone. The one local operation under it is the
/// synchronization's index hand-off, once the remote journal holds an
/// authoritative result for the bound context: a refresh with that
/// context's hand-off target at the authoritative commit (P-4), or with no
/// target, as older builds began it. Anything else under the ID is another
/// operation's use of it. An ID that is no binding's synchronization is not
/// this question's.
///
/// Decided here for `begin_or_reconcile`, which asks it first, and for
/// `require_refresh_can_begin`, so that a call which asks before it
/// changes anything is refused what its refresh would be.
fn comment_child_may_begin(
    connection: &Connection,
    root_path: &str,
    action: &str,
    requested: &str,
    target: &str,
) -> Result<bool, RepositoryError> {
    let child: Option<String> = connection
        .query_row(
            "SELECT operation_ulid FROM comment_publication_bindings WHERE synchronization_ulid=?1",
            [requested],
            |r| r.get(0),
        )
        .optional()
        .map_err(RepositoryError::sqlite)?;
    let Some(comment) = child else {
        return Ok(true);
    };
    // The binding is the authority for what may begin under its
    // synchronization's ID; the open of the call that asks no longer
    // migrates, and so no longer audits it first.
    validate_comment_binding(connection, &comment)?;
    let allowed: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM comment_publication_bindings b
        JOIN operation_records o ON o.id=b.local_record_id JOIN remote_operation_records r ON r.operation_ulid=b.synchronization_ulid
        JOIN repositories p ON p.id=r.repository_id WHERE b.synchronization_ulid=?1 AND o.root_path=?2 AND p.root_path=?2
        AND r.authoritative_kind IS NOT NULL AND r.action='synchronize_context' AND r.kind=b.kind AND r.item_id=b.item_ulid)",
        params![requested,root_path], |r| r.get(0)).map_err(RepositoryError::sqlite)?;
    if action != "refresh" || !allowed {
        return Ok(false);
    }
    if target.is_empty() {
        return Ok(true);
    }
    // The bound context's hand-off target, at the authoritative commit.
    let hand_off: Option<String> = connection
        .query_row(
            "SELECT r.kind || '/' || r.item_id, r.authoritative_oid
               FROM remote_operation_records r JOIN repositories p ON p.id = r.repository_id
              WHERE r.operation_ulid = ?1 AND p.root_path = ?2",
            params![requested, root_path],
            |row| {
                let of: String = row.get(0)?;
                let oid: Option<String> = row.get(1)?;
                Ok(oid.map(|oid| hand_off_target(&of, &oid)))
            },
        )
        .optional()
        .map_err(RepositoryError::sqlite)?
        .flatten();
    Ok(hand_off.as_deref() == Some(target))
}

/// The table of the bindings of comment submissions.
const COMMENT_BINDINGS_TABLE: &str = "comment_publication_bindings";

/// Whether `operation_id` is the ID a comment's binding keeps for its
/// synchronization. That synchronization is the comment submission's: it
/// is taken up by the submission's own retry, which proves first that
/// the branch still holds the comment. Whoever would take a
/// synchronization up by its ID alone, or say that it can be, asks this.
/// An index that has never held a submission may lack the table.
pub(super) fn is_comment_synchronization(
    connection: &Connection,
    operation_id: &str,
) -> rusqlite::Result<bool> {
    if !has_table(connection, COMMENT_BINDINGS_TABLE)? {
        return Ok(false);
    }
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM comment_publication_bindings WHERE synchronization_ulid = ?1)",
        [operation_id],
        |row| row.get(0),
    )
}

/// The phases in which a remote operation holds its repository's
/// reservation, as an SQL list: what `in_the_way` refuses a local operation
/// for, and the only phases in which a call can be at work under a remote
/// row (see `liveness::Worked`).
pub(super) const RESERVATION_PHASES: &str = "('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')";

/// What the local journal holds pending for a root, as a call that would
/// begin or take up an operation there reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Pending {
    /// No row is pending that is in the call's way.
    Nothing,
    /// The call's own row is pending, the row `id`, and it takes it up.
    /// Rows of other items that are not in its way may be pending too.
    Own(i64),
    /// A row is pending that the call may not take up and that is in its
    /// way; see `Scope`. The call is refused until that row is ended. The
    /// blocker is that row's operation, the first such row stored; `None`
    /// for a row recorded before operations had IDs.
    Another(Option<Blocker>),
}

/// What a row of the local journal is of while it is pending, and what a
/// call that would begin or take up a row is of; `holds` decides what the
/// row holds.
///
/// The scope is per item for the `ITEM_ACTIONS`, and for a
/// synchronization's index hand-off (P-4), which is of its item or of the
/// primary. Everything else is of the whole repository: the one discovery
/// index, which a refresh or a rebuild rewrites for every context; the
/// primary branch and its checkout, which an item's context is cut from;
/// the registration and the repository's remotes.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Scope {
    /// One item, by its ID, or the primary, as `primary` (design decision
    /// 16): what `ContextKey::encode` writes, so an item's ID never reads
    /// as the primary.
    Item(String),
    /// The whole repository.
    Repository,
}

impl Scope {
    /// The scope of a row of `action` that names `item_id` and was begun
    /// with `target`. A row of an item action that names no item, as each
    /// one an older build wrote does, may be of any item: it is of the
    /// whole repository. So is a synchronization's index hand-off with no
    /// target, as older builds wrote it.
    fn of_row(action: &str, item_id: Option<&str>, target: Option<&str>) -> Self {
        match item_id {
            Some(item) if ITEM_ACTIONS.contains(&action) => Self::Item(item.to_owned()),
            _ => synchronization_hand_off_of(action, item_id, target)
                .map_or(Self::Repository, Self::Item),
        }
    }

    /// The scope of a call of `action` with `target` that takes no row
    /// up: the item the row it begins would name, or the repository.
    fn of_call(action: &str, target: &str) -> Self {
        Self::of_row(
            action,
            authored_ids(action, target).0.as_deref(),
            Some(target),
        )
    }
}

/// The states in which a pending row whose commit is observed owes only its
/// index hand-off: the hand-off's own states (`HAND_OFF_STATES`), and the
/// state a write records with its checkpoint, which a crash before the
/// hand-off begins leaves (design decision 19, question 1). An index step
/// that claimed a row (`indexing`), or that a promotion fenced
/// (`index_invalidated`), stores only what a basis re-check under the lease
/// still finds, so it rewrites no other context's rows and is in no other
/// item's way.
const OWING_STATES: [&str; 6] = [
    "authoritative_observed",
    INDEXING_STATE,
    "index_invalidated",
    "failed",
    "retry",
    "authoring_checkpoint_observed",
];

/// Whether a pending row in `state` whose last step is `step` has its
/// commit observed and owes nothing more than its index hand-off (Table B's
/// "owes only its hand-off"): its last step says the operation's own work is
/// done (`LocalStep::is_authoritative`), and its state is one of the
/// `OWING_STATES`, or it is a comment's that waits for its registration.
/// Any other pending row is mid-write. `holds` asks it, and so does a read
/// of what a pending row owes an indexing (design decision 32).
///
/// The step is asked as well as the state: a refresh that failed is in one
/// of these states with no work of its own recorded. It is wider than
/// `LocalRow::owes_only_its_hand_off`, which asks a different question:
/// whether a refresh under the row's ID may complete it. A write stopped at
/// its checkpoint is completed by its own retry, which advances it to its
/// hand-off, and a comment waiting for its registration by its own retry
/// once the repository is registered.
pub(super) fn is_committed_and_owing(state: &str, step: Option<&str>) -> bool {
    listed(step).is_some_and(LocalStep::is_authoritative)
        && (OWING_STATES.contains(&state) || waits_for_registration(state, step))
}

/// A pending row of the local journal, as `holds` reads it: what it is of
/// (its action, target and item) and where it stands (its state and last
/// step). A row that has ended holds nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PendingRow {
    pub(super) action: String,
    pub(super) target: Option<String>,
    pub(super) item_id: Option<String>,
    pub(super) state: String,
    pub(super) completed_step: Option<String>,
}

impl PendingRow {
    /// Whether the row has not ended: it is not `completed`, or it is a
    /// comment's that waits for its registration (see `NOT_ENDED`).
    fn is_pending(&self) -> bool {
        self.state != "completed" || waits_for_registration(&self.state, self.step())
    }

    /// See `is_committed_and_owing`.
    fn is_committed_and_owing(&self) -> bool {
        is_committed_and_owing(&self.state, self.step())
    }

    fn step(&self) -> Option<&str> {
        self.completed_step.as_deref()
    }

    fn scope(&self) -> Scope {
        Scope::of_row(
            &self.action,
            self.item_id.as_deref(),
            self.target.as_deref(),
        )
    }

    fn standing(&self, call: &CallScope) -> Standing {
        match self.scope() {
            Scope::Repository => Standing::WholeRepository,
            Scope::Item(item) if call.own.contains(&item) => Standing::CallsItem,
            Scope::Item(_) => Standing::AnotherItem {
                committed_and_owing: self.is_committed_and_owing(),
            },
        }
    }
}

/// What a call is of, as `holds` reads it: the items it is of (each by its
/// ID, or `primary` for the primary; none for a call of no one item), how
/// far it reached before it began, and whether it is of the whole
/// repository. A close or a promotion is of two: its item and the primary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CallScope {
    own: std::collections::BTreeSet<String>,
    reach: CallReach,
    repository_wide: bool,
}

impl CallScope {
    /// A write or a synchronization of `item`, which reads nothing of
    /// another item.
    pub(super) fn item(item: &crate::canonical::ItemId) -> Self {
        Self::of_key(&super::discovery::ContextKey::Item(item.clone()))
    }

    /// A synchronization of the primary.
    pub(super) fn primary() -> Self {
        Self::of_key(&super::discovery::ContextKey::Primary)
    }

    /// A call of the whole repository and of no one item: a whole refresh,
    /// a rebuild, a poll, a retirement pass before it has its candidates.
    pub(super) fn repository() -> Self {
        Self {
            own: std::collections::BTreeSet::new(),
            reach: CallReach::OwnItem,
            repository_wide: true,
        }
    }

    /// A retirement of `item`'s context: of that item, and of the whole
    /// repository, since it removes a context and claims the index for its
    /// request (Table B's retirement column).
    pub(super) fn retirement(item: &crate::canonical::ItemId) -> Self {
        Self::item(item).and_the_repository()
    }

    /// The call of a remote operation of `target`. A poll is of the whole
    /// repository; a synchronization of its item or of the primary. A close
    /// or a promotion is of its item, which it merges and retires, and of
    /// the primary, which it writes, and of the whole repository as well: it
    /// is held by its own item's rows as by the primary's (Table B's own
    /// item and retirement columns).
    pub(super) fn of_remote(target: &super::RemoteOperationTarget) -> Self {
        use super::RemoteOperationAction::{
            Close, Poll, Promote, SynchronizeContext, SynchronizePrimary,
        };
        match (target.action(), target.item()) {
            (SynchronizeContext, Some((_, item))) => Self::item(item),
            (SynchronizePrimary, _) => Self::primary(),
            (Promote | Close, Some((_, item))) => {
                Self::item(item).with_the_primary().and_the_repository()
            }
            (Promote | Close, None) => Self::primary().and_the_repository(),
            (Poll | SynchronizeContext, _) => Self::repository(),
        }
    }

    /// This call, of the whole repository as well as of its own item: one
    /// that removes a context or rewrites the index for every context.
    pub(super) fn and_the_repository(self) -> Self {
        Self {
            repository_wide: true,
            ..self
        }
    }

    /// This call, of the primary as well as of the whole repository: a
    /// folder's creation, which writes the primary's worktree and is held by
    /// the primary's own rows, a primary synchronization's hand-off among
    /// them (design decision 19, question 7).
    fn and_the_primary(self) -> Self {
        self.with_the_primary().and_the_repository()
    }

    /// This call, of the primary as well as of what it is of.
    fn with_the_primary(mut self) -> Self {
        self.own
            .insert(super::discovery::ContextKey::Primary.encode());
        self
    }

    fn of_key(key: &super::discovery::ContextKey) -> Self {
        Self {
            own: std::collections::BTreeSet::from([key.encode()]),
            reach: CallReach::OwnItem,
            repository_wide: false,
        }
    }

    /// The items the call is of, as an SQL list of their keys; `None` for a
    /// call of no one item.
    fn own_sql(&self) -> Option<String> {
        let own: Vec<&str> = self.own.iter().map(String::as_str).collect();
        (!own.is_empty()).then(|| sql_list(&own))
    }

    /// The call `begin_or_reconcile` makes of `scope`, the scope of the row
    /// it takes up or would begin, having reached as far as `reach` says.
    fn of_begin(scope: Scope, reach: CallReach) -> Self {
        let call = match scope {
            Scope::Item(item) => Self {
                own: std::collections::BTreeSet::from([item]),
                reach: CallReach::OwnItem,
                repository_wide: false,
            },
            Scope::Repository => Self::repository(),
        };
        Self { reach, ..call }
    }
}

/// How a pending row stands to a call: the two facts of it Table B decides
/// on, whose it is and, for a row of another item, whether it is mid-write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standing {
    /// The row is of the whole repository; see `Scope`.
    WholeRepository,
    /// The row is of the item the call is of.
    CallsItem,
    /// The row is of another item.
    AnotherItem { committed_and_owing: bool },
}

impl Standing {
    const ALL: [Self; 4] = [
        Self::WholeRepository,
        Self::CallsItem,
        Self::AnotherItem {
            committed_and_owing: false,
        },
        Self::AnotherItem {
            committed_and_owing: true,
        },
    ];

    /// Table B: whether a pending row that stands so to `call` holds it.
    /// A row of the call's own item holds it, and so does a row of the
    /// whole repository. A row of another item holds it only while the row
    /// is mid-write and the call is of the whole repository or was checked
    /// against other items: a call that reads or rewrites what that row is
    /// still writing. A row of another item that owes only its hand-off
    /// holds nothing: its commit is there to be read, and a store of its
    /// context checks its basis again under the lease.
    fn holds(self, call: &CallScope) -> bool {
        match self {
            Self::WholeRepository | Self::CallsItem => true,
            Self::AnotherItem {
                committed_and_owing,
            } => {
                !committed_and_owing
                    && (call.repository_wide || call.reach == CallReach::OtherItems)
            }
        }
    }
}

/// Whether the pending row `row` holds `call`, which may not take it up:
/// the one place Table B ("what a pending row holds") is decided. The SQL
/// checks ask it through `holds_sql` and `holds_remote_index_debt_sql`.
/// Whether a held call then waits (its holder live) or is refused at once
/// (stale) is the liveness judgement's, after it.
pub(super) fn holds(row: &PendingRow, call: &CallScope) -> bool {
    row.is_pending() && row.standing(call).holds(call)
}

/// `holds` for a row of `operation_records`, as an SQL condition on its
/// columns, each named `{columns}<column>`: `columns` is `""`, or an alias
/// and a dot. It selects exactly the rows `holds` holds
/// (`holds_agrees_with_table_b`).
pub(super) fn holds_sql(call: &CallScope, columns: &str) -> String {
    let c = columns;
    let scope = scope_sql(c);
    let own = call.own_sql();
    let owes = format!(
        "({c}completed_step IS NOT NULL AND {c}completed_step IN {steps}
          AND ({c}state IN {states} OR ({c}state = 'completed' AND {c}completed_step = {registering})))",
        steps = sql_list(&authoritative_steps()),
        states = sql_list(&OWING_STATES),
        registering = sql_text(LocalStep::CommentRegistrationPending.as_str()),
    );
    let rows = |standing: Standing| match standing {
        Standing::WholeRepository => Some(format!("{scope} IS NULL")),
        Standing::CallsItem => own.as_ref().map(|own| format!("{scope} IN {own}")),
        Standing::AnotherItem {
            committed_and_owing,
        } => Some(format!(
            "({scope} IS NOT NULL{} AND {}{owes})",
            own.as_ref()
                .map(|own| format!(" AND {scope} NOT IN {own}"))
                .unwrap_or_default(),
            if committed_and_owing { "" } else { "NOT " }
        )),
    };
    format!(
        "(({c}state != 'completed' OR {c}completed_step IS {registering}) AND {held})",
        registering = sql_text(LocalStep::CommentRegistrationPending.as_str()),
        held = held_sql(call, rows)
    )
}

/// `Scope::of_row` as an SQL expression on the columns of a row of
/// `operation_records`, named as for `holds_sql`: the item the row is of
/// (its ID, or `primary`), or NULL for a row of the whole repository. A
/// target's beginning is compared exactly, as `synchronization_hand_off_of`
/// compares it.
fn scope_sql(c: &str) -> String {
    let begins = |prefix: String| format!("substr({c}target, 1, length({prefix})) = {prefix}");
    let hand_off =
        |of: String| format!("{} || {of} || '/'", sql_text(SYNCHRONIZATION_INDEX_TARGET));
    let of_its_item = HAND_OFF_KINDS
        .iter()
        .map(|kind| {
            begins(hand_off(format!(
                "{} || {c}item_id",
                sql_text(&format!("{}/", super::authoring_kind_segment(kind)))
            )))
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    format!(
        "(CASE WHEN {c}action IN {items} THEN {c}item_id
               WHEN {c}action = 'refresh' AND {of_the_primary} THEN {primary}
               WHEN {c}action = 'refresh' AND ({of_its_item}) THEN {c}item_id END)",
        items = sql_list(&ITEM_ACTIONS),
        of_the_primary = begins(hand_off(sql_text("primary"))),
        primary = sql_text(&super::discovery::ContextKey::Primary.encode()),
    )
}

/// `holds` for the index debt a remote operation's row keeps after it ended
/// (`index_pending`), as an SQL condition on a row of
/// `remote_operation_records`, its columns named as for `holds_sql`. Such a
/// debt owes only its hand-off. A synchronization's is of its item, or of
/// the primary (Table B's row of a synchronization's hand-off); any other
/// remote operation's, a poll's, a promotion's or a close's, is handed off
/// to a refresh of the whole repository, and is of the whole repository.
pub(super) fn holds_remote_index_debt_sql(call: &CallScope, columns: &str) -> String {
    let c = columns;
    let item = format!(
        "(CASE {c}action WHEN 'synchronize_context' THEN {c}item_id WHEN 'synchronize_primary' THEN {} END)",
        sql_text(&super::discovery::ContextKey::Primary.encode())
    );
    let own = call.own_sql();
    let rows = |standing: Standing| match standing {
        Standing::WholeRepository => Some(format!("{item} IS NULL")),
        Standing::CallsItem => own.as_ref().map(|own| format!("{item} IN {own}")),
        Standing::AnotherItem {
            committed_and_owing: true,
        } => Some(format!(
            "({item} IS NOT NULL{})",
            own.as_ref()
                .map(|own| format!(" AND {item} NOT IN {own}"))
                .unwrap_or_default()
        )),
        // An index debt is never mid-write.
        Standing::AnotherItem {
            committed_and_owing: false,
        } => None,
    };
    format!("({c}index_pending != 0 AND {})", held_sql(call, rows))
}

/// The rows of each `Standing` that holds `call`, as `rows` selects them,
/// joined into one SQL condition: false when none does.
fn held_sql(call: &CallScope, rows: impl Fn(Standing) -> Option<String>) -> String {
    let held: Vec<String> = Standing::ALL
        .into_iter()
        .filter(|standing| standing.holds(call))
        .filter_map(rows)
        .collect();
    if held.is_empty() {
        "0".to_owned()
    } else {
        format!("({})", held.join(" OR "))
    }
}

/// The steps `LocalStep::is_authoritative` accepts, as stored.
fn authoritative_steps() -> Vec<&'static str> {
    LocalStep::ALL
        .into_iter()
        .filter(|step| step.is_authoritative())
        .map(LocalStep::as_str)
        .collect()
}

/// `text` as an SQL string literal.
fn sql_text(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// `items` as an SQL list of string literals.
fn sql_list(items: &[&str]) -> String {
    let items: Vec<String> = items.iter().map(|item| sql_text(item)).collect();
    format!("({})", items.join(","))
}

/// How far a call that begins or takes up a row reached before it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallReach {
    /// It read nothing of other items: it is of its own item, or of the
    /// whole repository, as its action and target say; see `Scope`.
    OwnItem,
    /// It was checked against other items as the discovery index holds
    /// them, as a ticket save that sets `deps` or `parent` is for cycles
    /// and kinds. A row of another item still mid-write may hold a change
    /// that neither the index nor the disk has yet, so such a row holds the
    /// call; one that owes only its hand-off has its commit on disk, where
    /// the check reads every context whose basis moved, and does not. See
    /// `holds`.
    OtherItems,
}

/// What stands in the way of a call that would begin, or take up, the
/// local operation `requested` of `action` and `target` on `root_path`.
/// This is the one place that is decided: `begin_or_reconcile` asks it
/// in its transaction, and a call that would change something before
/// its operation begins asks it first, through `require_refresh_can_begin`.
///
/// `Err` when a remote operation of the repository holds its
/// reservation. A context synchronization owns only its exact authoring
/// kind and item: other contexts may go on authoring, and the matching
/// context is rejected again before any owned write by
/// `RepositoryService`'s checkpoint guard. Otherwise what the local
/// journal holds pending, of what is in the call's way.
///
/// The call is of the scope of the row it would take up, when there is
/// one: a refresh under a row's own ID, which is what `operation resume`
/// runs for a row that owes only its hand-off, is of that row's item.
/// Otherwise it is of the item its row would name, or of the whole
/// repository; see `Scope`. It reached as far as `reach` says. With
/// `of_the_primary`, a call of the whole repository is of the primary as
/// well (`CallScope::and_the_primary`). Each other pending row, a comment's
/// that waits for its registration among them, is in its way when it
/// `holds` the call, and is passed over otherwise.
#[allow(clippy::too_many_arguments)] // The one question both callers share.
fn in_the_way(
    connection: &Connection,
    root_path: &str,
    action: &str,
    requested: &str,
    target: &str,
    tagged_local_binding: bool,
    reach: CallReach,
    of_the_primary: bool,
) -> Result<Pending, RepositoryError> {
    let remote_active: Option<String> = connection.query_row(
        &format!("SELECT remote_operation_records.operation_ulid FROM remote_operation_records JOIN repositories ON repositories.id=remote_operation_records.repository_id WHERE repositories.root_path=?1 AND phase IN {RESERVATION_PHASES} AND (action != 'synchronize_context' OR ?2 LIKE ('authoring-context-v1/' || kind || '/' || item_id || '/%')) ORDER BY remote_operation_records.id LIMIT 1"),
        rusqlite::params![root_path, target], |row| row.get(0)).optional().map_err(|_| super::remote::state::recovery_required())?;
    if let Some(operation_id) = remote_active {
        return Err(
            super::remote::state::recovery_required().with_blocker(Blocker::remote(&operation_id))
        );
    }
    struct Row {
        id: i64,
        operation_id: Option<String>,
        row: PendingRow,
    }
    // Each of the two kinds of row that has not ended, read by the index
    // kept for it: a row that is not completed, and a completed comment's
    // that waits for its registration. No row is of both.
    let pending = connection
        .prepare(
            "SELECT id, operation_ulid, action, target, item_id, state, completed_step
               FROM operation_records WHERE root_path = ?1 AND state != 'completed'
             UNION ALL
             SELECT id, operation_ulid, action, target, item_id, state, completed_step
               FROM operation_records
              WHERE root_path = ?1 AND completed_step = 'comment_registration_pending'
                AND state = 'completed'
             ORDER BY id",
        )
        .and_then(|mut statement| {
            statement
                .query_map([root_path], |row| {
                    Ok(Row {
                        id: row.get(0)?,
                        operation_id: row.get(1)?,
                        row: PendingRow {
                            action: row.get(2)?,
                            target: row.get(3)?,
                            item_id: row.get(4)?,
                            state: row.get(5)?,
                            completed_step: row.get(6)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(RepositoryError::sqlite)?;
    // Whether the row is the call's own: the row of its operation, or one
    // it takes up.
    let compatible = |Row {
                          operation_id, row, ..
                      }: &Row| {
        if tagged_local_binding {
            operation_id.as_deref() == Some(requested)
                && action == "refresh"
                && row.action == action
                && row.target.as_deref() == Some(target)
        } else {
            operation_id.as_deref() == Some(requested)
                && (same_lifecycle_action(&row.action, action)
                    || action == "refresh" && row.action != "refresh")
                && (action == "refresh" || row.target.as_deref() == Some(target))
                || operation_id.is_none()
                    && matches!(
                        (row.action.as_str(), action),
                        ("refresh", "refresh") | ("rebuild", "rebuild")
                    )
        }
    };
    // Whether the call takes the row up. A comment's row that waits for
    // its registration is completed: its own call finds it as the row its
    // ID has, below, as before.
    let takes_up = |row: &Row| row.row.state != "completed" && compatible(row);
    let scope = pending
        .iter()
        .find(|row| takes_up(row))
        .map_or_else(|| Scope::of_call(action, target), |row| row.row.scope());
    let call = match scope {
        Scope::Repository if of_the_primary => CallScope::of_begin(scope, reach).and_the_primary(),
        scope => CallScope::of_begin(scope, reach),
    };
    let mut matching = None;
    for row in &pending {
        if compatible(row) {
            if takes_up(row) {
                matching = Some(row.id);
            }
        } else if holds(&row.row, &call) {
            return Ok(Pending::Another(Blocker::local(
                row.operation_id.as_deref(),
            )));
        }
    }
    // Every row found was the call's own or out of its way.
    Ok(matching.map_or(Pending::Nothing, Pending::Own))
}

/// Refuses, in the caller's transaction, a call of `operation` under
/// `operation_id` and `target` while another row of the repository is in its
/// way: `recovery_required` naming the blocker. The call's own row, taken up or
/// completed, is not. A setup's cancellation asks it before it deletes the
/// record, as a removal's reconciliation would be refused.
pub(super) fn require_not_in_the_way(
    connection: &Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
    target: &str,
) -> Result<(), RepositoryError> {
    let root_path = root.to_str().ok_or_else(|| invalid_path(operation, root))?;
    match in_the_way(
        connection,
        root_path,
        action_name(operation),
        &operation_id.to_string(),
        target,
        false,
        CallReach::OwnItem,
        false,
    )? {
        Pending::Another(blocker) => Err(recovery_required(operation, root).with_blocker(blocker)),
        Pending::Own(_) | Pending::Nothing => Ok(()),
    }
}

/// Refuses a root on which a refresh under `operation_id` would be
/// refused before it began: with `OperationMismatch` for an ID that is a
/// promotion's parent or child whose exact authoritative hand-off is not due,
/// or a comment's synchronization whose hand-off is not yet due, with
/// `RepositoryBusy` while an automatic poll holds the reservation, and
/// with `RecoveryRequired` for any other remote operation that holds it,
/// or for a pending local row the refresh may not take up that `holds` it.
/// It is the same test the refresh then makes,
/// for a call that would otherwise change something first and find the
/// refresh refused after. Nothing is written: the refresh, not this
/// question, asks an automatic poll to yield.
///
/// The one call that asks it, a folder's creation, writes the primary's
/// worktree before its refresh: it is asked as a call of the primary as
/// well as of the whole repository, which a primary synchronization's
/// hand-off holds (design decision 19, question 7).
pub(super) fn require_refresh_can_begin(
    connection: &Connection,
    root: &Path,
    operation: RepositoryOperation,
    operation_id: OperationId,
) -> Result<(), RepositoryError> {
    let root_path = root.to_str().ok_or_else(|| invalid_path(operation, root))?;
    let refresh = action_name(RepositoryOperation::RefreshRepository);
    if !super::local_sync::record::refresh_allowed(connection, &operation_id.to_string(), root)?
        || !super::context_retirement::record::refresh_allowed(
            connection,
            &operation_id.to_string(),
            root,
        )?
    {
        return Err(recovery_required(operation, root));
    }
    super::promotion::record::validate_local_identity(
        connection,
        root_path,
        refresh,
        &operation_id.to_string(),
        "",
    )?;
    super::closure::record::validate_local_identity(
        connection,
        root_path,
        refresh,
        &operation_id.to_string(),
        "",
    )?;
    if !comment_child_may_begin(
        connection,
        root_path,
        refresh,
        &operation_id.to_string(),
        "",
    )? {
        return Err(mismatch(operation, root));
    }
    if active_automatic_poll(connection, root_path)?.is_some() {
        return Err(poll_yielding(operation));
    }
    match in_the_way(
        connection,
        root_path,
        refresh,
        &operation_id.to_string(),
        "",
        false,
        CallReach::OwnItem,
        true,
    )? {
        Pending::Nothing | Pending::Own(_) => Ok(()),
        Pending::Another(blocker) => Err(recovery_required(operation, root).with_blocker(blocker)),
    }
}

/// The automatic poll holding `root_path`'s remote reservation, if one
/// does. At most one remote operation holds a repository's reservation.
fn active_automatic_poll(
    connection: &Connection,
    root_path: &str,
) -> Result<Option<i64>, RepositoryError> {
    connection.query_row(
        "SELECT remote_operation_records.id FROM remote_operation_records JOIN repositories ON repositories.id=remote_operation_records.repository_id WHERE repositories.root_path=?1 AND action='poll' AND priority='poll' AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')",
        [root_path], |row| row.get(0)).optional().map_err(|_| super::remote::state::recovery_required())
}

/// The retryable refusal of local work while an automatic poll yields.
fn poll_yielding(operation: RepositoryOperation) -> RepositoryError {
    RepositoryError::new(
        operation,
        None,
        RepositoryErrorKind::RepositoryBusy,
        "repository is busy; retry after the poll yields",
    )
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
    let waits = waits_for_registration(&state, step.as_deref());
    let step = step.as_deref().map(RecordedStep::of);
    Ok(if state != "completed" {
        JournalRow::Pending(PendingOperation::Local { state, step })
    } else if waits {
        // Completed, with its commit made and its index hand-off owed.
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: true,
            checkpointed: true,
        }
    } else if step.is_some_and(|step| step.is(LocalStep::Rejected) || step.is(LocalStep::Abandoned))
    {
        JournalRow::Absent
    } else {
        // Any other local operation that completed owes nothing more.
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
        if self.state != "completed" || waits_for_registration(&self.state, self.step.as_deref()) {
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

    /// Whether the row's last step is `step`.
    pub(super) fn step_is(&self, step: LocalStep) -> bool {
        listed(self.step.as_deref()) == Some(step)
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
        &format!(
            "UPDATE operation_records SET state = 'completed', completed_step = ?2, observed_at = ?3
              WHERE id = ?1 AND {NOT_ENDED}"
        ),
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
        .prepare(&format!(
            "SELECT operation_ulid, id, action, target, state, completed_step, observed_at
                   FROM operation_records
                  WHERE root_path = ?1 AND {NOT_ENDED} AND operation_ulid IS NOT NULL
                  ORDER BY id"
        ))?
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

/// The table of the remote journal, that of the key journal, and that of
/// the recorded key registry changes, whose operations are the key
/// journal's family too.
pub(super) const REMOTE_JOURNAL_TABLE: &str = "remote_operation_records";
pub(super) const KEY_JOURNAL_TABLE: &str = "key_material_operations";
pub(super) const KEY_REGISTRY_CHANGES_TABLE: &str = "key_registry_changes";

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
/// the ID, in any repository, or a change to the key registry is recorded
/// against it. Such an operation is not a local one even
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
        (KEY_REGISTRY_CHANGES_TABLE, "operation_id"),
        // A local integration companion also requires its domain's atomic
        // disposition. Generic row-only abandonment cannot release it.
        ("local_context_sync_records", "operation_ulid"),
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

/// Completes the row `record_id` at `step` in the caller's transaction, as a
/// setup's recording does: the step and the completion are written with the
/// record they belong to, or not at all. A row that is not there, or was
/// abandoned, is an error.
pub(super) fn complete_with_step(
    connection: &Connection,
    record_id: i64,
    step: LocalStep,
) -> Result<(), RepositoryError> {
    let completed = connection
        .execute(
            &format!(
                "UPDATE operation_records
                    SET state = 'completed', completed_step = ?2, observed_at = ?3
                  WHERE id = ?1 AND {NOT_ABANDONED}"
            ),
            params![record_id, step.as_str(), now()],
        )
        .map_err(RepositoryError::sqlite)?;
    if completed == 0 {
        refuse_abandoned(connection, record_id)?;
        return Err(super::remote::state::recovery_required());
    }
    Ok(())
}

/// Whether the row `record_id` has completed at `setup_recorded`: the call
/// that owns it has recorded its setup, whatever became of the record since.
/// A call that is told so by `begin_or_reconcile` is not: it reads the step as
/// none (`LocalStep::continued`), so a recording transaction asks here.
pub(super) fn recorded_a_setup(
    connection: &Connection,
    record_id: i64,
) -> Result<bool, RepositoryError> {
    let step: Option<String> = connection
        .query_row(
            "SELECT completed_step FROM operation_records WHERE id = ?1 AND state = 'completed'",
            [record_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(RepositoryError::sqlite)?
        .flatten();
    Ok(step.as_deref() == Some(LocalStep::SetupRecorded.as_str()))
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

/// `touch_pending`, by a call that takes the row over as it begins to work
/// under it, as a rebuild does: the row is touched now and held by
/// `holder`.
pub(super) fn claim_pending(
    connection: &Connection,
    record_id: i64,
    holder: &Holder,
) -> rusqlite::Result<()> {
    connection.execute(
        "UPDATE operation_records SET observed_at = ?2, host = ?3, pid = ?4
          WHERE id = ?1 AND state != 'completed'",
        params![record_id, now(), holder.host, holder.pid],
    )?;
    Ok(())
}

pub(super) fn claim_indexing(
    connection: &Connection,
    record_id: i64,
    stale_before: i64,
) -> Result<Option<IndexOwner>, RepositoryError> {
    // Request-bound retirement uses its private, current-F2 claim, never this
    // generic claim which can mint only an unscoped index owner.
    super::context_retirement::request_scope::validate_index(connection, record_id, None, None)?;
    // The claim takes the row over, a stale owner's among them: it records
    // this process as the row's holder. A live owner's row is not claimed.
    let holder = Holder::this_process();
    let claimed = connection
        .execute(
            "UPDATE operation_records SET state = 'indexing', index_owner_epoch = index_owner_epoch + 1, observed_at = ?2,
                    host = ?4, pid = ?5
              WHERE id = ?1 AND state IN (
                 'created', 'worktree_observed', 'authoring_checkpoint_observed', 'remote_changed',
                 'publication_committed', 'initialization_committed', 'authoritative_observed',
                  'failed', 'observed', 'persisted', 'retry', 'index_invalidated'
             ) OR (id = ?1 AND state = 'indexing' AND observed_at < ?3)",
            params![record_id, now(), stale_before, holder.host, holder.pid],
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
    Ok(Some(IndexOwner {
        record_id,
        epoch,
        retirement_request: None,
    }))
}

/// A committed promotion invalidates observations collected before its cleanup.
/// Preserve every journal identity/authoritative step; explicitly fence only live
/// index owners of this repository in the SAME transaction as completion/stale.
pub(super) fn invalidate_index_observations(
    transaction: &rusqlite::Transaction<'_>,
    repository_id: i64,
) -> Result<(), RepositoryError> {
    let rows = transaction.prepare(
        "SELECT id,index_owner_epoch FROM operation_records WHERE root_path=(SELECT root_path FROM repositories WHERE id=?1) AND state='indexing'"
    ).and_then(|mut q|q.query_map([repository_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?)))?.collect::<Result<Vec<_>,_>>())
        .map_err(RepositoryError::sqlite)?;
    for (id, epoch) in rows {
        let next = epoch
            .checked_add(1)
            .filter(|_| epoch >= 0)
            .ok_or_else(super::remote::state::recovery_required)?;
        let changed=transaction.execute(
            "UPDATE operation_records SET state='index_invalidated',index_owner_epoch=?3,observed_at=?4 WHERE id=?1 AND state='indexing' AND index_owner_epoch=?2",
            params![id,epoch,next,now()],
        ).map_err(RepositoryError::sqlite)?;
        if changed != 1 {
            return Err(super::remote::state::recovery_required());
        }
    }
    Ok(())
}

/// Only the exact prior owner may automatically reclaim this explicit fence.
/// Foreign/reclaimed/completed/abandoned rows are never adopted. The caller must
/// discard BOTH observations and re-observe after receiving the new epoch.
pub(super) fn reclaim_invalidated_indexing(
    connection: &Connection,
    owner: IndexOwner,
    root_key: &str,
) -> Result<Option<IndexOwner>, RepositoryError> {
    if owner.retirement_request.is_some() {
        // A new current F2 call claims request-bound indexing explicitly. Never
        // inflate its index epoch from a stale executor's cached request fence.
        owner.validate(connection, Some(owner.epoch))?;
        return Ok(None);
    }
    let fenced = owner
        .epoch
        .checked_add(1)
        .ok_or_else(super::remote::state::recovery_required)?;
    let next = fenced
        .checked_add(1)
        .ok_or_else(super::remote::state::recovery_required)?;
    // Domain-only reclaim is authorized by the exact invalidation tombstone in
    // the UPDATE below, not by still owning the pre-invalidation live epoch.
    // Keep envelope/handoff validation without changing that established CAS.
    super::context_retirement::request_scope::validate_index(
        connection,
        owner.record_id,
        owner.retirement_request,
        None,
    )?;
    let changed=connection.execute(&format!(
        "UPDATE operation_records SET state='indexing',index_owner_epoch=?4,observed_at=?5 WHERE id=?1 AND root_path=?2 AND state='index_invalidated' AND index_owner_epoch=?3 AND {NOT_ABANDONED}"
    ),params![owner.record_id,root_key,fenced,next,now()]).map_err(RepositoryError::sqlite)?;
    Ok((changed == 1).then_some(IndexOwner {
        record_id: owner.record_id,
        epoch: next,
        retirement_request: owner.retirement_request,
    }))
}

pub(super) fn transition_indexing(
    connection: &Connection,
    owner: IndexOwner,
    state: &str,
    context: Option<&Path>,
) -> Result<bool, RepositoryError> {
    owner.validate(connection, Some(owner.epoch))?;
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
    owner.validate(connection, Some(owner.epoch))?;
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
    owner.validate(connection, Some(owner.epoch))?;
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
    owner.validate(connection, Some(owner.epoch))?;
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
         FROM operation_records WHERE root_path = ?1 AND (state != 'completed' OR completed_step='comment_registration_pending') ORDER BY id",
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
        RepositoryOperation::CancelSetup => "cancel_setup",
        RepositoryOperation::AddRemote => "add_remote",
        RepositoryOperation::RemoveRemote => "remove_remote",
        RepositoryOperation::SetPublicationRemote => "set_publication_remote",
        RepositoryOperation::RefreshRepository => "refresh",
        RepositoryOperation::RebuildRepository => "rebuild",
        RepositoryOperation::PrepareContext => "prepare_context",
        RepositoryOperation::ReleaseContext => "release_context",
        RepositoryOperation::RetireContext => "retire_context",
        RepositoryOperation::SaveDocument => "save_document",
        RepositoryOperation::SaveTicket => "save_ticket",
        RepositoryOperation::SubmitComment => "submit_comment",
        RepositoryOperation::RepairItem => "repair_item",
        RepositoryOperation::ContextSyncLocal => "context_sync_local",
        // No row is begun under the name of any of these: they have no
        // operation ID, or, as a folder's creation does, they begin the
        // row of the refresh that follows. Each is named, so that an
        // operation added later does not compile until it is said
        // whether it begins a row, and under what name.
        RepositoryOperation::OpenRegistry
        | RepositoryOperation::RegisterSharedKey
        | RepositoryOperation::SelectSharedKey
        | RepositoryOperation::ClearSharedKeySelection
        | RepositoryOperation::UnregisterSharedKey
        | RepositoryOperation::ListSharedKeys
        | RepositoryOperation::PreflightGeneratedKeyDeletion
        | RepositoryOperation::Inspect
        | RepositoryOperation::ListRemotes
        | RepositoryOperation::RepositorySnapshot
        | RepositoryOperation::Read
        | RepositoryOperation::AbandonOperation
        | RepositoryOperation::SetLocalIdentity
        | RepositoryOperation::CreateFolder => "other",
    }
}

fn operation_from_name(action: &str) -> Option<RepositoryOperation> {
    match action {
        "create_and_enable" => Some(RepositoryOperation::CreateAndEnable),
        "enable" => Some(RepositoryOperation::Enable),
        "remove_registration" => Some(RepositoryOperation::RemoveRegistration),
        "cancel_setup" => Some(RepositoryOperation::CancelSetup),
        "add_remote" => Some(RepositoryOperation::AddRemote),
        "remove_remote" => Some(RepositoryOperation::RemoveRemote),
        "set_publication_remote" => Some(RepositoryOperation::SetPublicationRemote),
        "refresh" => Some(RepositoryOperation::RefreshRepository),
        "rebuild" => Some(RepositoryOperation::RebuildRepository),
        "prepare_context" => Some(RepositoryOperation::PrepareContext),
        "release_context" => Some(RepositoryOperation::ReleaseContext),
        "save_document" => Some(RepositoryOperation::SaveDocument),
        "save_ticket" => Some(RepositoryOperation::SaveTicket),
        "submit_comment" => Some(RepositoryOperation::SubmitComment),
        "repair_item" => Some(RepositoryOperation::RepairItem),
        "context_sync_local" => Some(RepositoryOperation::ContextSyncLocal),
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

/// Deletes the cache-loss marker once an explicit repair has cleared the flag
/// (Decision 7). Returns whether a marker was there to delete. A marker that
/// is not the one this build writes is left alone and reported as an error.
pub(super) fn retire_remote_history_marker(data: &std::path::Path) -> std::io::Result<bool> {
    if !remote_history_lost(data)? {
        return Ok(false);
    }
    std::fs::remove_file(data.join(REMOTE_HISTORY_MARKER))?;
    #[cfg(unix)]
    std::fs::File::open(data)?.sync_all()?;
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

    #[test]
    fn promotion_parent_id_is_refused_before_local_journal_begin() {
        let mut db = Connection::open_in_memory().unwrap();
        crate::repository::discovery::migrate_registry(&mut db).unwrap();
        db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,'/repository',7,'accessible',0)",[]).unwrap();
        let op = OperationId::parse(ID).unwrap();
        {
            let tx = db.transaction().unwrap();
            crate::repository::promotion::record::begin_record(
                &tx,
                1,
                op,
                &"01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
                crate::repository::PromotionExpectation::from_bytes([7; 32]),
                None,
            )
            .unwrap();
            tx.commit().unwrap();
        }
        assert_eq!(
            begin_or_reconcile_operation(
                &mut db,
                Path::new("/repository"),
                RepositoryOperation::RefreshRepository,
                op,
                "",
                7
            )
            .err()
            .unwrap()
            .kind,
            RepositoryErrorKind::OperationMismatch
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM operation_records", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

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

    /// The ID a comment's binding keeps for its synchronization is that
    /// synchronization's alone. A refresh under it is its index hand-off
    /// and begins only once the synchronization has an authoritative
    /// result; `begin_or_reconcile` refuses any other. A call that asks
    /// first whether its refresh can begin is refused the same way, and
    /// so has changed nothing when it is.
    #[test]
    fn a_refresh_under_a_comment_s_synchronization_id_is_refused_before_it_is_begun() {
        const CHILD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAC";
        let connection = journal("submit_comment", "completed", Some("completed"), 7);
        connection
            .execute_batch(
                "CREATE TABLE remote_operation_records (
                    id INTEGER PRIMARY KEY, operation_ulid TEXT, repository_id INTEGER,
                    action TEXT, priority TEXT, kind TEXT, item_id TEXT, phase TEXT,
                    authoritative_kind TEXT, authoritative_oid TEXT)",
            )
            .unwrap();
        // A well-formed binding: the refresh under its synchronization's ID
        // audits it first (design decision 9).
        connection
            .execute(
                "UPDATE operation_records SET target=?1 WHERE id=1",
                [format!(
                    "authoring-context-v1/document/{ID}/Edit\0.manyhands/comments\0{ID}"
                )],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO comment_publication_bindings
                    (operation_ulid, local_record_id, root_digest, kind, item_ulid, comment_ulid,
                     synchronization_ulid, pre_checkpoint_oid)
                 VALUES (?1, 1, ?2, 'document', ?1, ?1, ?3, ?4)",
                params![
                    ID,
                    blake3::hash(b"/repository").as_bytes().as_slice(),
                    CHILD,
                    "0".repeat(40)
                ],
            )
            .unwrap();
        let root = Path::new("/repository");
        let begin = |id: &str| {
            require_refresh_can_begin(
                &connection,
                root,
                RepositoryOperation::CreateFolder,
                OperationId::parse(id).unwrap(),
            )
            .map_err(|error| error.kind)
        };

        assert_eq!(begin(CHILD), Err(RepositoryErrorKind::OperationMismatch));
        // Any other ID is asked only what is pending.
        assert_eq!(begin("01ARZ3NDEKTSV4RRFFQ69G5FAD"), Ok(()));
    }

    /// An automatic poll yields to local work, so a call that asks first
    /// whether its refresh can begin is told to retry, as the refresh
    /// itself would be. Asking writes nothing: the refresh, not the
    /// question, requests the yield. Any other active remote operation
    /// still refuses the refresh outright.
    #[test]
    fn a_refresh_asked_about_during_an_automatic_poll_is_busy_not_recovery() {
        let connection = journal("save_ticket", "completed", Some("completed"), 7);
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path) VALUES (1, '/repository');
                 CREATE TABLE remote_operation_records (
                    id INTEGER PRIMARY KEY, operation_ulid TEXT, repository_id INTEGER,
                    action TEXT, priority TEXT, kind TEXT, item_id TEXT, phase TEXT,
                    authoritative_kind TEXT, yield_requested INTEGER NOT NULL DEFAULT 0);",
            )
            .unwrap();
        let root = Path::new("/repository");
        let begin = || {
            require_refresh_can_begin(
                &connection,
                root,
                RepositoryOperation::CreateFolder,
                OperationId::parse("01ARZ3NDEKTSV4RRFFQ69G5FAD").unwrap(),
            )
            .map_err(|error| error.kind)
        };
        let active = |action: &str, priority: &str| {
            connection
                .execute_batch("DELETE FROM remote_operation_records")
                .unwrap();
            connection
                .execute(
                    "INSERT INTO remote_operation_records
                        (id, repository_id, action, priority, phase) VALUES (1, 1, ?1, ?2, 'advertising')",
                    params![action, priority],
                )
                .unwrap();
        };

        active("poll", "poll");
        assert_eq!(begin(), Err(RepositoryErrorKind::RepositoryBusy));
        let requested: i64 = connection
            .query_row(
                "SELECT yield_requested FROM remote_operation_records WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(requested, 0, "asking requests no yield");

        // An explicit poll holds manual priority and does not yield.
        active("poll", "manual");
        assert_eq!(begin(), Err(RepositoryErrorKind::RecoveryRequired));
        active("synchronize", "manual");
        assert_eq!(begin(), Err(RepositoryErrorKind::RecoveryRequired));
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
        // `ALL` is every variant by construction: `local_steps!` declares
        // the enum and `ALL` from the same list. What is held here is
        // what the journal stores: these names, each read back as its
        // own step. A step that is added, renamed or moved fails this
        // until its stored name is written here.
        assert_eq!(
            LocalStep::ALL.map(LocalStep::as_str),
            [
                "branch_observed",
                "worktree_observed",
                "context_worktree_released",
                "context_branch_released",
                "context_retirement_absent",
                "context_retirement_preserved",
                "comment_destination_prepared",
                "comment_checkpoint_intent",
                "comment_registration_pending",
                "authoring_destination_observed",
                "document_destination_observed",
                "document_move_observed",
                "authoring_checkpoint_observed",
                "authoritative_observed",
                "remote_changed",
                "publication_configuration_written",
                "publication_committed",
                "repository_initialized",
                "configuration_written",
                "initialization_committed",
                "retry",
                "error",
                "completed",
                "rejected",
                "abandoned",
                "local_sync_prepared",
                "setup_recorded",
                "setup_cancel_recorded",
            ]
        );
        for step in LocalStep::ALL {
            assert!(is_stored_name(step.as_str()), "{step:?}");
            assert_eq!(LocalStep::parse(step.as_str()), Some(step));
            assert_eq!(RecordedStep::of(step.as_str()), RecordedStep::Listed(step));
        }
        let mut names = LocalStep::ALL.map(LocalStep::as_str).to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            LocalStep::ALL.len(),
            "no two steps share a name"
        );
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
            retirement_request: None,
        };
        assert!(!transition_indexing(&connection, owner.clone(), "completed", None).unwrap());
        assert!(!touch_indexing(&connection, owner.clone()).unwrap());
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
            for state in [
                "authoritative_observed",
                "indexing",
                "index_invalidated",
                "failed",
                "retry",
            ] {
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
            Some("comment_checkpoint_intent"),
            Some("repository_initialized"),
            Some("configuration_written"),
        ] {
            for state in [
                "created",
                "worktree_observed",
                "authoritative_observed",
                "indexing",
                "index_invalidated",
                "failed",
                "retry",
            ] {
                assert!(!owes("save_ticket", state, step), "{step:?} in {state}");
            }
        }
        // A refresh that failed has no work of its own recorded, and a
        // rebuild's states are not a hand-off's.
        assert!(!owes("refresh", "failed", None));
        assert!(!owes("refresh", "index_invalidated", None));
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
    fn an_id_a_key_registry_change_is_recorded_against_is_not_a_local_operation_s() {
        let connection = journal("refresh", "failed", None, 7);
        connection
            .execute_batch("CREATE TABLE key_registry_changes (operation_id TEXT);")
            .unwrap();
        assert!(!held_by_another_journal(&connection, ID).unwrap());
        connection
            .execute("INSERT INTO key_registry_changes VALUES (?1)", [ID])
            .unwrap();
        assert!(held_by_another_journal(&connection, ID).unwrap());
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

#[cfg(test)]
mod attribution_tests {
    //! Whose a row is: the migration that adds the columns, the request a
    //! remote row is attributed to, and the item and comment a local row
    //! names. Each over a journal of only the columns it reads.

    use super::*;

    fn origin() -> RowOrigin {
        RowOrigin {
            holder: Holder {
                host: "host-a".to_owned(),
                pid: 42,
            },
            started_at: 7,
            author: Some("A <a@example.invalid>".to_owned()),
        }
    }

    fn marked(db: &Connection) -> bool {
        db.query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [ATTRIBUTION_MIGRATION],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// Without the remote journal the local one gains its columns and no
    /// marker is recorded; once the remote journal is there it gains them
    /// too and the marker is. A marked registry whose journal lost a
    /// column is refused, not repaired.
    #[test]
    fn the_attribution_migration_adds_what_is_missing_and_refuses_a_lost_column() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE registry_migrations (name TEXT PRIMARY KEY);
             CREATE TABLE operation_records (id INTEGER PRIMARY KEY, action TEXT);",
        )
        .unwrap();
        migrate_operation_attribution(&db).unwrap();
        assert!(has_column(&db, "operation_records", "comment_id"));
        assert!(!marked(&db));

        db.execute_batch("CREATE TABLE remote_operation_records (id INTEGER PRIMARY KEY);")
            .unwrap();
        migrate_operation_attribution(&db).unwrap();
        for (name, _) in ATTRIBUTION_COLUMNS {
            assert!(has_column(&db, "operation_records", name), "{name}");
            assert!(has_column(&db, REMOTE_JOURNAL_TABLE, name), "{name}");
        }
        assert!(!has_column(&db, REMOTE_JOURNAL_TABLE, "comment_id"));
        assert!(marked(&db));
        // Marked: an audit only.
        migrate_operation_attribution(&db).unwrap();

        db.execute_batch("ALTER TABLE remote_operation_records DROP COLUMN host")
            .unwrap();
        assert_eq!(
            migrate_operation_attribution(&db).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired
        );
        assert!(!has_column(&db, REMOTE_JOURNAL_TABLE, "host"));
    }

    /// A conflict with another process that holds the registry's write lock
    /// is a SQLite failure carrying `SQLITE_BUSY`, which is transient: the
    /// same as the sibling migrations give, not `recovery_required`.
    #[test]
    fn a_busy_registry_fails_the_attribution_migration_as_sqlite_not_as_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("registry.sqlite3");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE registry_migrations (name TEXT PRIMARY KEY);
             CREATE TABLE operation_records (id INTEGER PRIMARY KEY, action TEXT);",
        )
        .unwrap();
        let other = Connection::open(&path).unwrap();
        other.busy_timeout(std::time::Duration::ZERO).unwrap();
        db.execute_batch("BEGIN IMMEDIATE").unwrap();

        let error = migrate_operation_attribution(&other).unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::Sqlite, "{error:?}");
        let source = error
            .source
            .as_deref()
            .and_then(|source| source.downcast_ref::<rusqlite::Error>())
            .expect("the SQLite error is the source");
        assert_eq!(
            source.sqlite_error_code(),
            Some(rusqlite::ErrorCode::DatabaseBusy)
        );
        // Once the other process is done a retry migrates.
        db.execute_batch("ROLLBACK").unwrap();
        migrate_operation_attribution(&other).unwrap();
        assert!(has_column(&other, "operation_records", "comment_id"));
    }

    fn has_column(db: &Connection, table: &str, column: &str) -> bool {
        db.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name=?2)",
            [table, column],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// A remote row is the request's whose record holds its operation, an
    /// accepted one before a finished one; else the request of the
    /// promotion or closure it is a child of; else, for a comment's
    /// synchronization, the request the comment's row names. A row no
    /// request began names none, and still records its process.
    ///
    /// A local row begun under a remote operation's ID, as a
    /// synchronization's index hand-off is, is the request's that remote
    /// row names, unless a request's record holds it as a local operation.
    #[test]
    fn a_remote_row_is_the_request_s_that_began_it_or_its_parent_s() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE registry_migrations (name TEXT PRIMARY KEY);
             CREATE TABLE request_records (request_ulid TEXT, command TEXT, state TEXT, accepted_at INTEGER);
             CREATE TABLE request_operations (request_ulid TEXT, ordinal INTEGER, family TEXT, operation_ulid TEXT);
             CREATE TABLE operation_records (id INTEGER PRIMARY KEY, operation_ulid TEXT);
             CREATE TABLE remote_operation_records (id INTEGER PRIMARY KEY, repository_id INTEGER, operation_ulid TEXT);
             CREATE TABLE document_promotions (repository_id INTEGER, operation_id TEXT, context_sync_id TEXT, primary_sync_id TEXT);
             CREATE TABLE ticket_closures (repository_id INTEGER, operation_id TEXT, context_sync_id TEXT, primary_sync_id TEXT);
             CREATE TABLE comment_publication_bindings (operation_ulid TEXT, local_record_id INTEGER, synchronization_ulid TEXT);",
        )
        .unwrap();
        migrate_operation_attribution(&db).unwrap();
        db.execute_batch(
            "INSERT INTO request_records VALUES
                ('sync-request', 'item sync', 'accepted', 1),
                ('earlier-request', 'item sync', 'finished', 9),
                ('promote-request', 'document promote', 'accepted', 1),
                ('close-request', 'ticket close', 'accepted', 1),
                ('comment-request', 'comment add', 'accepted', 1);
             INSERT INTO request_operations VALUES
                ('sync-request', 0, 'remote', 'sync'),
                ('earlier-request', 0, 'remote', 'sync'),
                ('promote-request', 0, 'remote', 'promote'),
                ('close-request', 0, 'remote', 'close'),
                ('comment-request', 0, 'local', 'comment');
             INSERT INTO document_promotions VALUES (1, 'promote', 'promote-context', 'promote-primary');
             INSERT INTO ticket_closures VALUES (1, 'close', 'close-context', 'close-primary');
             INSERT INTO operation_records (id, operation_ulid, request_ulid, command)
                VALUES (1, 'comment', 'comment-request', 'comment add');
             INSERT INTO comment_publication_bindings VALUES ('comment', 1, 'comment-sync');",
        )
        .unwrap();
        // A promotion's or a closure's child names no author: whoever
        // promoted or closed is kept by the index alone.
        let author = Some("A <a@example.invalid>");
        for (operation, request, author) in [
            ("sync", Some(("sync-request", "item sync")), author),
            (
                "promote-context",
                Some(("promote-request", "document promote")),
                None,
            ),
            (
                "close-primary",
                Some(("close-request", "ticket close")),
                None,
            ),
            (
                "comment-sync",
                Some(("comment-request", "comment add")),
                author,
            ),
            ("poll", None, author),
        ] {
            db.execute(
                "INSERT INTO remote_operation_records (repository_id, operation_ulid) VALUES (1, ?1)",
                [operation],
            )
            .unwrap();
            attribute_remote_row(&db, db.last_insert_rowid(), &origin()).unwrap();
            let stored: (
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                i64,
                i64,
            ) = db
                .query_row(
                    "SELECT request_ulid, command, author, host, pid, started_at
                       FROM remote_operation_records WHERE operation_ulid = ?1",
                    [operation],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .unwrap();
            assert_eq!(
                stored,
                (
                    request.map(|(id, _)| id.to_owned()),
                    request.map(|(_, command)| command.to_owned()),
                    author.map(str::to_owned),
                    "host-a".to_owned(),
                    42,
                    7
                ),
                "{operation}"
            );
        }
        for (operation, request) in [
            ("sync", Some(("sync-request", "item sync"))),
            (
                "promote-context",
                Some(("promote-request", "document promote")),
            ),
            ("comment-sync", Some(("comment-request", "comment add"))),
            ("poll", None),
            ("unknown", None),
        ] {
            db.execute(
                "INSERT INTO operation_records (operation_ulid) VALUES (?1)",
                [operation],
            )
            .unwrap();
            let id = db.last_insert_rowid();
            attribute_local_row(&db, id, &origin()).unwrap();
            let stored: (Option<String>, Option<String>, String, i64) = db
                .query_row(
                    "SELECT request_ulid, command, host, pid FROM operation_records WHERE id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .unwrap();
            assert_eq!(
                stored,
                (
                    request.map(|(id, _)| id.to_owned()),
                    request.map(|(_, command)| command.to_owned()),
                    "host-a".to_owned(),
                    42
                ),
                "local {operation}"
            );
        }
        // A local link is preferred: the comment's own row is the
        // comment's request's, not a remote row's under its ID.
        db.execute(
            "INSERT INTO remote_operation_records (repository_id, operation_ulid, request_ulid, command)
             VALUES (1, 'comment', 'sync-request', 'item sync')",
            [],
        )
        .unwrap();
        attribute_local_row(&db, 1, &origin()).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT request_ulid FROM operation_records WHERE id = 1",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "comment-request"
        );
    }

    /// A row of an authoring action names its item, and only a comment's
    /// submission its comment, each read from a target of the form it is
    /// begun with; so does a synchronization's index hand-off of an item
    /// (P-4), and one of the primary names none. No other row names an item
    /// here: a local synchronization names its own when it is begun, and a
    /// retirement none.
    #[test]
    fn an_authoring_row_names_its_item_and_only_a_comment_its_comment() {
        const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
        const OID: &str = "0123456789abcdef0123456789abcdef01234567";
        let item = || Some(ITEM.to_owned());
        for (action, target, expected) in [
            (
                "save_ticket",
                format!("authoring-context-v1/ticket/{ITEM}/Edit\0"),
                (item(), None),
            ),
            (
                "submit_comment",
                format!("authoring-context-v1/ticket/{ITEM}/Edit\0.manyhands/comments\0{COMMENT}"),
                (item(), Some(COMMENT.to_owned())),
            ),
            (
                "submit_comment",
                format!("authoring-context-v1/document/{ITEM}/Edit\0.manyhands/comments\0x"),
                (item(), None),
            ),
            (
                "save_document",
                format!("authoring-context-v1/document/{ITEM}/Edit\0docs/a.md"),
                (item(), None),
            ),
            (
                "repair_item",
                format!(
                    "authoring-context-v1/ticket/{ITEM}/Repair\0.manyhands/tickets/{ITEM}/ticket.md"
                ),
                (item(), None),
            ),
            (
                "prepare_context",
                format!("authoring-context-v1/document/{ITEM}/Create"),
                (item(), None),
            ),
            (
                "release_context",
                format!("authoring-context-v1/ticket/{ITEM}/Release"),
                (item(), None),
            ),
            (
                "retire_context",
                format!("authoring-context-v1/ticket/{ITEM}/Retire"),
                (None, None),
            ),
            ("context_sync_local", "observation".to_owned(), (None, None)),
            ("save_ticket", "remote-v1-0123".to_owned(), (None, None)),
            ("refresh", String::new(), (None, None)),
            (
                "refresh",
                format!("synchronization-index-v1/ticket/{ITEM}/{OID}"),
                (item(), None),
            ),
            (
                "refresh",
                format!("synchronization-index-v1/document/{ITEM}/{OID}"),
                (item(), None),
            ),
            (
                "refresh",
                format!("synchronization-index-v1/primary/{OID}"),
                (None, None),
            ),
            (
                "refresh",
                format!("synchronization-index-v1/comment/{ITEM}/{OID}"),
                (None, None),
            ),
            (
                "refresh",
                format!("synchronization-local-v1/ticket/{ITEM}/{OID}"),
                (None, None),
            ),
            (
                "save_ticket",
                format!("synchronization-index-v1/ticket/{ITEM}/{OID}"),
                (None, None),
            ),
        ] {
            assert_eq!(
                authored_ids(action, &target),
                expected,
                "{action} {target:?}"
            );
        }
    }
}

#[cfg(test)]
#[path = "recovery_scope_tests.rs"]
mod scope_tests;
