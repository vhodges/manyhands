//! Abandoning a local operation: the way out for a pending row of the
//! local journal that the operation's own repeat cannot clear.
//!
//! A pending row refuses every other operation on its repository until
//! the same operation is repeated with input it accepts. Where it cannot
//! be, because the input is lost or the files have since changed, the row
//! is closed here instead. Nothing else is done: no file, reference,
//! index entry or worktree is touched, and what the operation wrote stays
//! where it is, named in the result for its user to deal with in Git.
//!
//! **A row that is in flight is not abandoned.** Two things show that a
//! call of this or another process is working under a row, and the row
//! is closed only when neither does:
//!
//! - the repository lease, which every local operation holds from before
//!   it begins its row until it ends or hands off to indexing, and the
//!   bootstrap lease, which a creation holds before there is a
//!   repository to have a lease. Both are taken here, so a call that
//!   holds either makes this one `RepositoryBusy`;
//! - the heartbeat of an index owner, which a row in the state
//!   `indexing` has from the call that claimed it after that call let
//!   the repository lease go. A row whose heartbeat is no older than the
//!   time after which another call may take the claim over is busy.
//!
//! A call that is between two leases, as a creation is for an instant
//! before it goes on as an enablement, finds its row closed when it
//! takes the next one and is refused there, having written nothing more.

use std::path::{Path, PathBuf};

use git2::{Repository, RepositoryOpenFlags, StatusOptions};
use rusqlite::TransactionBehavior;
use time::OffsetDateTime;

use super::super::{
    INDEX_OWNER_STALE_AFTER, OperationId, ReadError, RepositoryError, RepositoryErrorKind,
    RepositoryOperation, RepositoryService, bootstrap_lease, cache_write_guard, canonical,
    migrate_registry, mutation, open_registry,
    recovery::{self, LocalRow, LocalStanding},
    repository_lease,
};
use crate::results::{ResultCode, relative_path_string};

/// The paths an abandon names at most. An operation writes one file or
/// two; a context that holds more uncommitted paths than this is shown by
/// `git status`, which the result's `paths_complete` sends its reader to.
const MOST_PATHS: usize = 100;

/// Where item worktrees are, below the repository root.
const WORKTREES_DIRECTORY: &str = ".manyhands/worktrees";

/// A request to close one pending row of the local journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbandonOperationRequest {
    /// The repository's root, as its journal rows name it.
    pub root: PathBuf,
    /// The operation to abandon.
    pub operation_id: OperationId,
    /// The state and step the caller saw the row in. The row is closed
    /// only while it still stands so; `None` closes it however it is
    /// pending.
    pub expected: Option<PendingStanding>,
}

/// Where a pending local row stands: its state and the last step it
/// recorded, by the names the journal stores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingStanding {
    pub state: String,
    pub step: Option<String>,
}

/// What a pending local operation has left: what its row says, and what
/// its target shows on disk. It holds names and paths and no content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationRemains {
    /// The operation's action, by the name the journal stores:
    /// `save_ticket`, `enable` and so on.
    pub action: String,
    pub standing: PendingStanding,
    /// The item the operation writes, for an authoring operation.
    pub item_id: Option<String>,
    /// The worktree the operation writes in, relative to the repository's
    /// root with forward slashes: the item's own for an authoring
    /// operation, and `.` for one that writes the configuration file in
    /// the primary worktree. `None` for an operation that writes no file
    /// of a worktree, and for an item whose worktree is not there.
    pub worktree: Option<String>,
    /// The paths of the operation's target that differ from the head of
    /// their branch, uncommitted and untracked alike, relative to the
    /// repository's root with forward slashes. For an authoring operation
    /// they are every such path of the item's worktree, which is that
    /// item's alone; for one that writes the configuration file, that
    /// file when it differs.
    pub paths: Vec<String>,
    /// Whether `paths` is all of them. It is not when there are more than
    /// are listed, or when the worktree could not be read.
    pub paths_complete: bool,
    /// The request whose record holds the operation and is not finished:
    /// the one an abandon releases.
    pub owner: Option<String>,
}

/// How an operation ID stands for someone who would abandon it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbandonableOperation {
    /// The local journal holds no row of the ID for the root.
    NotFound,
    /// The operation is pending, and this is what it has left.
    Pending(OperationRemains),
    /// The operation ran to its end.
    Completed,
    /// The operation was rejected and left nothing behind.
    Rejected,
    /// The operation was abandoned before.
    Abandoned,
}

/// What became of a request to abandon an operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbandonOperationOutcome {
    /// The row was pending and is closed. Nothing else was changed.
    Abandoned(OperationRemains),
    /// The local journal holds no row of the ID for the root.
    NotFound,
    /// The row is pending and no longer stands as the caller saw it.
    /// Nothing was changed.
    Changed,
    /// The operation ran to its end. Nothing was changed.
    Completed,
    /// The operation was rejected and left nothing behind. Nothing was
    /// changed.
    Rejected,
    /// The operation was abandoned before. Nothing was changed.
    AlreadyAbandoned,
}

/// The root a journal row names for `root`: its canonical path, or, for
/// a root that is not there, its canonical parent joined with its name,
/// which is how a creation names the root it is about to make.
fn journal_root(root: &Path) -> Result<PathBuf, RepositoryError> {
    let operation = RepositoryOperation::AbandonOperation;
    let invalid = || {
        RepositoryError::new(
            operation,
            None,
            RepositoryErrorKind::InvalidPath,
            "the selected root cannot be resolved",
        )
    };
    if let Ok(root) = std::fs::canonicalize(root) {
        return Ok(root);
    }
    let name = root.file_name().ok_or_else(invalid)?;
    let parent = root.parent().ok_or_else(invalid)?;
    Ok(std::fs::canonicalize(parent)
        .map_err(|_| invalid())?
        .join(name))
}

/// The item an authoring operation writes, from its row's target:
/// `authoring-context-v1/<kind>/<item ID>/<intent>`, followed for a write
/// by the paths it names.
fn authoring_item(action: &str, target: Option<&str>) -> Option<String> {
    if !matches!(
        action,
        "prepare_context" | "save_document" | "save_ticket" | "submit_comment"
    ) {
        return None;
    }
    let context = target?.split('\0').next()?;
    let mut parts = context.split('/');
    if parts.next() != Some("authoring-context-v1") {
        return None;
    }
    let _kind = parts.next()?;
    let item_id = parts.next()?;
    // Only an item ID is passed on: it becomes a directory name.
    item_id
        .parse::<canonical::ItemId>()
        .ok()
        .map(|item_id| item_id.to_string())
}

/// Whether the action writes the configuration file in the primary
/// worktree.
fn writes_configuration(action: &str) -> bool {
    matches!(
        action,
        "create_and_enable" | "enable" | "set_publication_remote"
    )
}

/// The paths of `worktree` that differ from its head, each joined to
/// `prefix`, and whether that is all of them. With `only`, that path
/// alone is looked at. Nothing is written: the index is read as it is on
/// disk and is not refreshed.
fn differing_paths(worktree: &Path, prefix: &Path, only: Option<&str>) -> (Vec<String>, bool) {
    let read = || -> Result<(Vec<String>, bool), git2::Error> {
        // Exactly this worktree, never a repository above it.
        let repository = Repository::open_ext(
            worktree,
            RepositoryOpenFlags::NO_SEARCH,
            &[] as &[&std::ffi::OsStr],
        )?;
        let mut options = StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false)
            .exclude_submodules(true)
            .update_index(false);
        if let Some(only) = only {
            options.pathspec(only).disable_pathspec_match(true);
        }
        let statuses = repository.statuses(Some(&mut options))?;
        let mut paths = Vec::new();
        let mut complete = true;
        for entry in statuses.iter() {
            if entry.status().is_empty() || entry.status() == git2::Status::CURRENT {
                continue;
            }
            if paths.len() == MOST_PATHS {
                complete = false;
                break;
            }
            // A path that is not text cannot be named in a result.
            match entry
                .path()
                .and_then(|path| relative_path_string(&prefix.join(path)))
            {
                Some(path) => paths.push(path),
                None => complete = false,
            }
        }
        paths.sort_unstable();
        Ok((paths, complete))
    };
    read().unwrap_or_else(|_| (Vec::new(), false))
}

/// What the pending `row` of the repository at `root` has left. Git is
/// read and nothing is written.
fn remains(root: &Path, row: &LocalRow, owner: Option<String>) -> OperationRemains {
    let item_id = authoring_item(&row.action, row.target.as_deref());
    let (worktree, paths, paths_complete) = if let Some(item_id) = &item_id {
        let relative = Path::new(WORKTREES_DIRECTORY).join(item_id);
        let directory = root.join(&relative);
        if directory.is_dir() {
            let (paths, complete) = differing_paths(&directory, &relative, None);
            (relative_path_string(&relative), paths, complete)
        } else {
            // No worktree: the operation stopped before it made one.
            (None, Vec::new(), true)
        }
    } else if writes_configuration(&row.action) {
        if root.join(".git").exists() {
            let (paths, complete) =
                differing_paths(root, Path::new(""), Some(canonical::CONFIG_PATH));
            (Some(".".to_owned()), paths, complete)
        } else {
            // No repository: the creation stopped before it made one.
            (None, Vec::new(), true)
        }
    } else {
        (None, Vec::new(), true)
    };
    OperationRemains {
        action: row.action.clone(),
        standing: PendingStanding {
            state: row.state.clone(),
            step: row.step.clone(),
        },
        item_id,
        worktree,
        paths,
        paths_complete,
        owner,
    }
}

impl RepositoryService {
    /// How the local operation `operation_id` of the repository at `root`
    /// stands, and what it has left when it is pending. Nothing is
    /// written and no lease is taken: this is what a preview of an
    /// abandon shows, and it says nothing of whether a call is working
    /// under the row, which only the abandon itself finds out.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub fn abandonable_operation(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<AbandonableOperation, ReadError> {
        let root = journal_root(root)?;
        let Some(root_key) = root.to_str() else {
            return Err(ReadError::new(ResultCode::InvalidPath));
        };
        let found = self.read_session(RepositoryOperation::Read, |connection| {
            let Some(row) = recovery::local_row(connection, root_key, operation_id)? else {
                return Ok(None);
            };
            if !row.is_well_formed() {
                return Err(ReadError::new(ResultCode::InternalError));
            }
            let owner = mutation::operation_owner(connection, root_key, operation_id)?;
            Ok(Some((row, owner)))
        })?;
        let Some((row, owner)) = found else {
            return Ok(AbandonableOperation::NotFound);
        };
        Ok(match row.standing() {
            LocalStanding::Pending => AbandonableOperation::Pending(remains(&root, &row, owner)),
            LocalStanding::Completed => AbandonableOperation::Completed,
            LocalStanding::Rejected => AbandonableOperation::Rejected,
            LocalStanding::Abandoned => AbandonableOperation::Abandoned,
        })
    }

    /// Closes one pending row of the local journal, and with it releases
    /// the unfinished request whose record holds the operation, and that
    /// request's confirmation, in the same transaction.
    ///
    /// It touches no file, reference, index entry or worktree. A row that
    /// is not pending is left as it is, and so is one that no longer
    /// stands as `expected` says. A row a call is working under is
    /// `RepositoryBusy`; see the module's notes.
    ///
    /// The closed row is kept, `completed` with the step `abandoned`: it
    /// blocks nothing, `operation show` reads it so, and a repeat of its
    /// operation ID is refused with `OperationAbandoned` before anything
    /// is written.
    pub fn abandon_operation(
        &self,
        request: AbandonOperationRequest,
    ) -> Result<AbandonOperationOutcome, RepositoryError> {
        let operation = RepositoryOperation::AbandonOperation;
        let AbandonOperationRequest {
            root,
            operation_id,
            expected,
        } = request;
        self.require_index_available(operation, Some(&root))?;
        let root = journal_root(&root)?;
        let root_key = super::super::registry_root_key(&root, operation)?;
        let data_directory = self
            .registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        // A creation works under the bootstrap lease before there is a
        // repository, and every other local operation under the
        // repository's own.
        let _bootstrap = bootstrap_lease(data_directory, &root, operation)?;
        let _lease = match Repository::open_ext(
            &root,
            RepositoryOpenFlags::NO_SEARCH,
            &[] as &[&std::ffi::OsStr],
        ) {
            Ok(repository) => Some(repository_lease(&repository, &root, operation)?),
            // No repository is there to have a lease or a call under it.
            Err(_) => None,
        };
        let _cache_guard = cache_write_guard(&self.registry_path, &root, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(operation, &root))?;
        migrate_registry(&mut connection).map_err(|error| error.for_operation(operation, &root))?;
        let sqlite = |error| RepositoryError::sqlite(error).for_operation(operation, &root);
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        let Some(row) =
            recovery::local_row(&transaction, root_key, operation_id).map_err(sqlite)?
        else {
            return Ok(AbandonOperationOutcome::NotFound);
        };
        if !row.is_well_formed() {
            return Err(RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::Sqlite,
                "the journal holds a row that cannot be read",
            ));
        }
        match row.standing() {
            LocalStanding::Pending => {}
            LocalStanding::Completed => return Ok(AbandonOperationOutcome::Completed),
            LocalStanding::Rejected => return Ok(AbandonOperationOutcome::Rejected),
            LocalStanding::Abandoned => return Ok(AbandonOperationOutcome::AlreadyAbandoned),
        }
        // An index owner whose heartbeat is fresh is at work, with no
        // lease held. A clock that reads earlier than the heartbeat does
        // not make it stale.
        let stale_before = OffsetDateTime::now_utc().unix_timestamp() - INDEX_OWNER_STALE_AFTER;
        if row.state == "indexing" && row.observed_at >= stale_before {
            return Err(RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::RepositoryBusy,
                "the operation is being indexed by another call",
            ));
        }
        if let Some(expected) = &expected
            && (expected.state != row.state || expected.step != row.step)
        {
            return Ok(AbandonOperationOutcome::Changed);
        }
        let owner = mutation::release_operation_owner(&transaction, root_key, operation_id)
            .map_err(sqlite)?;
        let left = remains(&root, &row, owner);
        if !recovery::abandon_row(&transaction, row.id).map_err(sqlite)? {
            // The row was pending when this transaction read it.
            return Err(RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::Sqlite,
                "the journal row could not be closed",
            ));
        }
        transaction.commit().map_err(sqlite)?;
        Ok(AbandonOperationOutcome::Abandoned(left))
    }
}
