//! Registration identity outlives the repository on disk. Only removal may
//! select an unavailable root this way; ordinary repository resolution stays live.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use git2::{Repository, RepositoryOpenFlags};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use super::{
    FailurePoint, OperationId, RemoveRegistrationOutcome, RemoveRegistrationRequest,
    RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
    begin_or_reconcile_operation, bootstrap_lease, cache_read_guard, cache_write_guard,
    canonical_repository_root, migrate_registry, open_registry, open_registry_read_only,
    registry_root_key, repository_lease,
};

const OPERATION: RepositoryOperation = RepositoryOperation::RemoveRegistration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RegistrationStamp {
    pub(super) id: i64,
    pub(super) enabled_at: i64,
}

/// `None` expects absence; it never means an unchecked deletion.
#[derive(Clone, Debug)]
pub(super) struct RemovalTarget {
    pub(super) root: PathBuf,
    pub(super) registration: Option<RegistrationStamp>,
}

fn error(root: &Path, kind: RepositoryErrorKind, message: &'static str) -> RepositoryError {
    RepositoryError::new(OPERATION, Some(root.to_owned()), kind, message)
}

/// Only a positively absent root/`.git` is handled as absence by the probe.
/// Permission failures (and a vanished target after a positive observation)
/// mean inaccessible state; they never authorize registry-only removal.
fn path_failure(root: &Path, error: io::Error) -> RepositoryError {
    let kind = match error.kind() {
        io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound => {
            RepositoryErrorKind::InaccessibleRepository
        }
        io::ErrorKind::NotADirectory | io::ErrorKind::InvalidInput => {
            RepositoryErrorKind::InvalidPath
        }
        _ => RepositoryErrorKind::Io,
    };
    RepositoryError::with_source(OPERATION, Some(root.to_owned()), kind, error)
}

fn registration(
    connection: &Connection,
    root: &Path,
) -> Result<Option<RegistrationStamp>, RepositoryError> {
    let key = registry_root_key(root, OPERATION)?;
    connection
        .query_row(
            "SELECT id, enabled_at FROM repositories WHERE root_path=?1",
            [key],
            |row| {
                Ok(RegistrationStamp {
                    id: row.get(0)?,
                    enabled_at: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(RepositoryError::sqlite)
}

/// An absent root or positively absent `.git` permits registry-only cleanup.
/// Broken Git indirection, permissions and arbitrary Git errors do not.
pub(super) fn repository_at(root: &Path) -> Result<Option<Repository>, RepositoryError> {
    match fs::symlink_metadata(root) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(path_failure(root, e)),
        Ok(_) => {}
    }
    let metadata = fs::metadata(root).map_err(|e| path_failure(root, e))?;
    if !metadata.is_dir() {
        return Err(error(
            root,
            RepositoryErrorKind::InvalidPath,
            "the selected path is not a directory",
        ));
    }
    let repo = match Repository::open_ext(root, RepositoryOpenFlags::NO_SEARCH, &[] as &[&OsStr]) {
        Ok(repo) => repo,
        Err(git) => {
            if git.code() == git2::ErrorCode::NotFound {
                match fs::symlink_metadata(root.join(".git")) {
                    Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                    Err(e) => return Err(path_failure(root, e)),
                    Ok(_) => {}
                }
            }
            return Err(RepositoryError::with_source(
                OPERATION,
                Some(root.to_owned()),
                RepositoryErrorKind::InaccessibleRepository,
                git,
            ));
        }
    };
    if repo.is_bare() {
        return Err(error(
            root,
            RepositoryErrorKind::BareRepository,
            "bare repositories have no working directory",
        ));
    }
    let directory = repo.workdir().ok_or_else(|| {
        error(
            root,
            RepositoryErrorKind::InaccessibleRepository,
            "repository has no working directory",
        )
    })?;
    let actual = fs::canonicalize(directory).map_err(|e| path_failure(root, e))?;
    if repo.is_worktree() || actual != root {
        return Err(error(
            root,
            RepositoryErrorKind::InvalidPath,
            "the selected path is not the repository working-directory root",
        ));
    }
    Ok(Some(repo))
}

fn git_directory(repo: &Repository, root: &Path) -> Result<PathBuf, RepositoryError> {
    fs::canonicalize(repo.commondir()).map_err(|e| path_failure(root, e))
}

/// Without a Git lease, preserve all unfinished work, not just another local
/// operation. In particular, deleting a registration is never an unrelated
/// context write that can pass a context synchronization's reservation.
fn require_registry_only_removal(
    db: &Connection,
    root: &Path,
    operation_id: OperationId,
) -> Result<(), RepositoryError> {
    super::local_sync::record::audit(db)?;
    let key = registry_root_key(root, OPERATION)?;
    let id = operation_id.to_string();
    // Let the existing begin path persist its yield request and return busy.
    let poll: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote_operation_records o JOIN repositories r ON r.id=o.repository_id
         WHERE r.root_path=?1 AND o.action='poll' AND o.priority='poll'
         AND o.phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling'))",
        [key], |r| r.get(0),
    ).map_err(RepositoryError::sqlite)?;
    if poll {
        return Ok(());
    }
    // Preserve the domain's operation-identity answer even when this root is
    // unavailable. The shared begin path repeats this validation before writing.
    let existing: Option<(String, String, Option<String>)> = db
        .query_row(
            "SELECT root_path,action,target FROM operation_records WHERE operation_ulid=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(RepositoryError::sqlite)?;
    if existing
        .as_ref()
        .is_some_and(|(stored_root, action, target)| {
            stored_root != key || action != "remove_registration" || target.as_deref() != Some("")
        })
    {
        return Err(error(
            root,
            RepositoryErrorKind::OperationMismatch,
            "the operation ID belongs to another operation",
        ));
    }
    let mut statement = db.prepare("SELECT id,action,target,state,completed_step,observed_at FROM operation_records WHERE root_path=?1")
        .map_err(RepositoryError::sqlite)?;
    let rows = statement
        .query_map([key], |r| {
            Ok(super::recovery::LocalRow {
                id: r.get(0)?,
                action: r.get(1)?,
                target: r.get(2)?,
                state: r.get(3)?,
                step: r.get(4)?,
                observed_at: r.get(5)?,
            })
        })
        .map_err(RepositoryError::sqlite)?;
    for row in rows {
        let row = row.map_err(RepositoryError::sqlite)?;
        if !row.is_well_formed()
            || row.operation().is_none()
            || row.step.as_ref().is_some_and(|step| {
                !super::recovery::LocalStep::ALL
                    .iter()
                    .any(|known| known.as_str() == step)
            })
        {
            return Err(error(
                root,
                RepositoryErrorKind::RecoveryRequired,
                "the local journal contains unrecognized recovery evidence",
            ));
        }
    }
    let pending: bool = db.query_row(
        "SELECT
         EXISTS(SELECT 1 FROM operation_records WHERE root_path=?1
           AND (state!='completed' OR completed_step='comment_registration_pending')
           AND NOT (operation_ulid IS ?2 AND action='remove_registration' AND target IS ''))
         OR EXISTS(SELECT 1 FROM remote_operation_records o JOIN repositories r ON r.id=o.repository_id
           WHERE r.root_path=?1 AND (o.phase NOT IN ('completed','cancelled') OR o.index_pending!=0 OR o.reconciliation_required!=0))
         OR EXISTS(SELECT 1 FROM document_promotions p JOIN repositories r ON r.id=p.repository_id
           WHERE r.root_path=?1 AND p.step NOT IN ('complete','abandoned','superseded'))
         OR EXISTS(SELECT 1 FROM ticket_closures c JOIN repositories r ON r.id=c.repository_id
           WHERE r.root_path=?1 AND c.step NOT IN ('complete','abandoned','superseded'))
         OR EXISTS(SELECT 1 FROM request_records r WHERE r.scope_key=?1 AND r.state='accepted'
           AND NOT (r.command='repo remove' AND r.target='' AND EXISTS(
             SELECT 1 FROM request_operations o WHERE o.request_ulid=r.request_ulid
             AND o.family='local' AND o.operation_ulid=?2)))
         OR (SELECT COUNT(*) FROM request_records r JOIN request_operations o ON o.request_ulid=r.request_ulid
           WHERE r.scope_key=?1 AND r.state='accepted' AND o.family='local' AND o.operation_ulid=?2)>1",
        params![key, id], |r| r.get(0),
    ).map_err(RepositoryError::sqlite)?;
    if pending {
        return Err(error(
            root,
            RepositoryErrorKind::RecoveryRequired,
            "unfinished repository work must be recovered before removing this registration",
        ));
    }
    Ok(())
}

impl RepositoryService {
    /// Selects the stored root before touching the repository. An unavailable
    /// alias cannot identify a registration; only its exact stored root can.
    pub(super) fn removal_target(&self, selected: &Path) -> Result<RemovalTarget, RepositoryError> {
        self.require_index_available(OPERATION, Some(selected))?;
        let lookup = |root: &Path| {
            let _guard = cache_read_guard(&self.registry_path, root, OPERATION)?;
            let db = open_registry_read_only(&self.registry_path)?;
            registration(&db, root)
        };
        if selected.is_absolute()
            && let Some(stamp) = lookup(selected)?
        {
            return Ok(RemovalTarget {
                root: selected.to_owned(),
                registration: Some(stamp),
            });
        }
        let (repo, root) = canonical_repository_root(selected, OPERATION)?;
        if repo.is_worktree() {
            return Err(error(
                &root,
                RepositoryErrorKind::InvalidPath,
                "a linked worktree is not a repository registration root",
            ));
        }
        let registration = lookup(&root)?;
        Ok(RemovalTarget { root, registration })
    }

    /// Removes only application-local registration and dependent state. Files,
    /// Git and durable mutation receipts are preserved.
    pub fn remove_registration(
        &self,
        request: RemoveRegistrationRequest,
    ) -> Result<RemoveRegistrationOutcome, RepositoryError> {
        let target = self.removal_target(&request.root)?;
        self.remove_registration_checked(target, request.operation_id)
    }

    pub(super) fn remove_registration_checked(
        &self,
        target: RemovalTarget,
        operation_id: OperationId,
    ) -> Result<RemoveRegistrationOutcome, RepositoryError> {
        let RemovalTarget {
            root,
            registration: expected,
        } = target;
        self.require_index_available(OPERATION, Some(&root))?;
        let key = registry_root_key(&root, OPERATION)?;
        let at_root = |e: RepositoryError| e.for_operation(OPERATION, &root);
        let sqlite = |e| at_root(RepositoryError::sqlite(e));
        let data = self
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let _bootstrap = bootstrap_lease(data, &root, OPERATION)?;
        let repository = repository_at(&root)?;
        let lease = repository
            .as_ref()
            .map(|repo| repository_lease(repo, &root, OPERATION))
            .transpose()?;
        let directory = repository
            .as_ref()
            .map(|repo| git_directory(repo, &root))
            .transpose()?;
        #[cfg(test)]
        tests::before_cache();
        let _guard = cache_write_guard(&self.registry_path, &root, OPERATION)?;
        let current = repository_at(&root)?;
        let current_directory = current
            .as_ref()
            .map(|repo| git_directory(repo, &root))
            .transpose()?;
        let same_lease = match (&lease, &current) {
            (Some(lease), Some(repo)) => lease
                .still_names(repo)
                .map_err(|e| RepositoryError::io(OPERATION, Some(root.clone()), e))?,
            (None, None) => true,
            _ => false,
        };
        if directory != current_directory || !same_lease {
            return Err(error(
                &root,
                RepositoryErrorKind::RepositoryBusy,
                "repository changed while acquiring removal guards",
            ));
        }
        let mut db = open_registry(&self.registry_path, &mut |_| {}).map_err(at_root)?;
        migrate_registry(&mut db).map_err(at_root)?;
        let check_registration = |db: &Connection| {
            if registration(db, &root)? != expected {
                return Err(error(
                    &root,
                    RepositoryErrorKind::ExternalChange,
                    "the registration changed before removal",
                ));
            }
            Ok(())
        };
        check_registration(&db).map_err(at_root)?;
        if repository.is_none() {
            require_registry_only_removal(&db, &root, operation_id).map_err(at_root)?;
        }
        begin_or_reconcile_operation(&mut db, &root, OPERATION, operation_id, "")
            .map_err(at_root)?;
        self.check_failure(
            FailurePoint::BeforeRegistrationRemovalTransaction,
            OPERATION,
            &root,
        )?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        check_registration(&tx).map_err(at_root)?;
        tx.execute("DELETE FROM operation_records WHERE root_path=?1", [key])
            .map_err(sqlite)?;
        let deleted = tx
            .execute("DELETE FROM repositories WHERE root_path=?1", [key])
            .map_err(sqlite)?;
        tx.commit().map_err(sqlite)?;
        Ok(if deleted == 0 {
            RemoveRegistrationOutcome::NotRegistered
        } else {
            RemoveRegistrationOutcome::Removed
        })
    }
}

#[cfg(test)]
#[path = "registration_removal_tests.rs"]
mod tests;
