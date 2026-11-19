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
//! - a heartbeat, for the two kinds of call that work under a row with
//!   no lease held: an index owner, whose row is in the state `indexing`
//!   from the call that claimed it, and a rebuild, which observes the
//!   whole repository unleased and touches its row throughout. A row
//!   whose heartbeat is no older than the time after which another call
//!   may take an index claim over is busy.
//!
//! **A row that was closed is never reopened.** The checks above leave
//! instants in which a call is at work with neither sign showing: a
//! creation between its leases, before it goes on as an enablement; a
//! local synchronization between binding its row and refreshing; an
//! authoring call between letting its lease go and completing its row.
//! What makes those safe is not the checks but the journal: every
//! statement that changes a row by its ID refuses an abandoned one, so
//! the call that was still running stops with `OperationAbandoned` at
//! its next step and writes nothing further under the row.
//!
//! **Only an operation of the local journal alone.** An ID the remote
//! journal or the key journal holds is not abandoned by `abandon_operation`,
//! even when the local journal holds a row under it, as it does for the
//! index hand-off of a synchronization: that row is the synchronization's to
//! end.
//!
//! **A synchronization nothing else can clear.** A synchronization whose
//! process stopped can leave a push that only the person can judge: one
//! not known to have returned, of a context the remote has been seen to
//! hold, or one that returned and that the remote does not hold now. Its
//! resume cannot say what became of the push, and refuses. Such an
//! operation is closed by `abandon_remote_operation`, with the person's
//! consent to what the remote journal shows of it; see
//! `RemoteAbandonEvidence`. The push is recorded refused, so that it no
//! longer counts as one nothing answered, and nothing else is done: no
//! file, reference or worktree is touched and the remote is not contacted.
//! An operation whose process still works under it is never closed.

use std::path::{Path, PathBuf};

use git2::{Repository, RepositoryOpenFlags, StatusOptions};
use rusqlite::TransactionBehavior;
use time::OffsetDateTime;

use super::super::{
    AuthoringKind, INDEX_OWNER_STALE_AFTER, OperationFamily, OperationId, ReadError,
    RemoteOperationPhase, RemoteOperationSafePoint, RepositoryError, RepositoryErrorKind,
    RepositoryOperation, RepositoryService, authoring_kind_segment, bootstrap_lease,
    cache_write_guard, canonical, liveness, migrate_registry, mutation, open_registry,
    recovery::{self, LocalRow, LocalStanding},
    remote::{
        reservation::{self, AbandonEvidence},
        state::{self, StoredRemoteOperation},
    },
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
    /// Whether the operation recorded that it passed the point where its
    /// work is committed. An abandon leaves that commit where it is: it
    /// is not undone, and nothing says it was abandoned but this.
    ///
    /// A comment that stopped at `comment_checkpoint_intent` reads
    /// `true` and may have made no commit: the step is recorded before
    /// the commit is attempted. `paths` says which: with no commit made
    /// the comment's file is among them.
    pub committed: bool,
    /// The request whose record holds the operation and is not finished:
    /// the one an abandon releases.
    pub owner: Option<String>,
}

/// How an operation ID stands for someone who would abandon it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbandonableOperation {
    /// The local journal holds no row of the ID for the root.
    NotFound,
    /// The remote journal or the key journal holds an operation of the
    /// ID: it is not a local operation, whatever the local journal holds
    /// under it.
    AnotherJournal,
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
    /// The remote journal or the key journal holds an operation of the
    /// ID. Nothing was changed.
    AnotherJournal,
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

/// What an operation writes in a worktree, by its kind.
enum Writes {
    /// The files of one item, in that item's own worktree.
    Item,
    /// The configuration file, in the primary worktree.
    Configuration,
    /// No file of any worktree.
    Nothing,
}

/// What `operation` writes in a worktree. Every operation is named: one
/// added later does not compile until it is decided here, so that an
/// operation that writes a file is never reported as having left none.
fn writes(operation: RepositoryOperation) -> Writes {
    match operation {
        RepositoryOperation::PrepareContext
        | RepositoryOperation::SaveDocument
        | RepositoryOperation::SaveTicket
        | RepositoryOperation::SubmitComment
        // A repair writes the corrected file in the item's own worktree.
        | RepositoryOperation::RepairItem
        | RepositoryOperation::ContextSyncLocal => Writes::Item,
        RepositoryOperation::CreateAndEnable
        | RepositoryOperation::Enable
        | RepositoryOperation::SetPublicationRemote => Writes::Configuration,
        // A remote is Git configuration, a registration and the index
        // are Manyhands' own, and the rest begin no row.
        RepositoryOperation::AddRemote
        | RepositoryOperation::RemoveRemote
        | RepositoryOperation::RemoveRegistration
        | RepositoryOperation::RefreshRepository
        | RepositoryOperation::RebuildRepository
        | RepositoryOperation::OpenRegistry
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
        // An identity is Git configuration too, and is set under no row.
        | RepositoryOperation::SetLocalIdentity
        // A folder is a directory, which is no file, and is made under
        // no row: the row of its request is its refresh's.
        | RepositoryOperation::CreateFolder
        // A release removes a context that holds nothing of anyone's,
        // and writes no file and no remote state.
        | RepositoryOperation::ReleaseContext
        | RepositoryOperation::RetireContext => Writes::Nothing,
    }
}

/// The item an authoring operation writes, from its row's target:
/// `authoring-context-v1/<kind>/<item ID>/<intent>`, followed for a write
/// by the paths it names.
pub(in super::super) fn authoring_item(target: Option<&str>) -> Option<String> {
    let context = target?.split('\0').next()?;
    let mut parts = context.split('/');
    if parts.next() != Some("authoring-context-v1") {
        return None;
    }
    let kind = parts.next()?;
    if ![AuthoringKind::Document, AuthoringKind::Ticket]
        .iter()
        .any(|known| authoring_kind_segment(known) == kind)
    {
        return None;
    }
    let item_id = parts.next()?;
    // Only an item ID is passed on: it becomes a directory name.
    item_id
        .parse::<canonical::ItemId>()
        .ok()
        .map(|item_id| item_id.to_string())
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
    // An action this build does not know is one it cannot say anything
    // of: nothing is named, and the list is not called complete.
    let (item_id, worktree, paths, paths_complete) = match row.operation().map(writes) {
        Some(Writes::Item) => match authoring_item(row.target.as_deref()) {
            Some(item_id) => {
                let relative = Path::new(WORKTREES_DIRECTORY).join(&item_id);
                let directory = root.join(&relative);
                if directory.is_dir() {
                    let (paths, complete) = differing_paths(&directory, &relative, None);
                    (
                        Some(item_id),
                        relative_path_string(&relative),
                        paths,
                        complete,
                    )
                } else {
                    // No worktree: the operation stopped before it made
                    // one.
                    (Some(item_id), None, Vec::new(), true)
                }
            }
            None => (None, None, Vec::new(), false),
        },
        Some(Writes::Configuration) => {
            if root.join(".git").exists() {
                let (paths, complete) =
                    differing_paths(root, Path::new(""), Some(canonical::CONFIG_PATH));
                (None, Some(".".to_owned()), paths, complete)
            } else {
                // No repository: the creation stopped before it made one.
                (None, None, Vec::new(), true)
            }
        }
        Some(Writes::Nothing) => (None, None, Vec::new(), true),
        None => (None, None, Vec::new(), false),
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
        committed: row.is_checkpointed(),
        owner,
    }
}

/// The time before which a heartbeat is too old to be a live call's.
pub(in super::super) fn stale_heartbeat() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp() - INDEX_OWNER_STALE_AFTER
}

/// Where a pending remote operation stands, and the epoch of its owner:
/// what a confirmed abandon is told to expect, so that a row that moved
/// on, or that a restart took over, since the preview is left alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemoteStanding {
    pub(crate) phase: RemoteOperationPhase,
    pub(crate) step: Option<RemoteOperationSafePoint>,
    /// The synchronization's checkpoint, by the name its row stores.
    pub(crate) checkpoint: Option<&'static str>,
    pub(crate) owner_epoch: i64,
}

impl RemoteStanding {
    fn of(record: &StoredRemoteOperation) -> Self {
        Self {
            phase: record.phase,
            step: record.completed_step,
            checkpoint: record.sync_checkpoint.map(|checkpoint| checkpoint.name()),
            owner_epoch: record.owner_epoch,
        }
    }
}

/// What the preview of the abandon of a synchronization names, so that
/// the person decides with the facts. Nothing is fetched and the remote is
/// not contacted: what is said of the remote is what the operation's row
/// last saw of it. Each is `None` where there is nothing to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RemoteAbandonEvidence {
    /// Whether the ref the operation publishes to was there when the row
    /// last looked; `None` for a row that recorded no look, and for one
    /// whose push the remote took after that look.
    pub(crate) remote_ref_present: Option<bool>,
    /// The commit the ref named then.
    pub(crate) remote_oid: Option<git2::Oid>,
    /// The commit the operation's local branch names now, read from Git.
    pub(crate) local_oid: Option<git2::Oid>,
    /// The commit the operation set out to push.
    pub(crate) push_oid: Option<git2::Oid>,
}

/// What a pending synchronization has left, as an abandon names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemoteRemains {
    pub(crate) action: crate::repository::RemoteOperationAction,
    pub(crate) standing: RemoteStanding,
    /// The item whose context it synchronizes; `None` for primary's.
    pub(crate) item_id: Option<String>,
    pub(crate) evidence: RemoteAbandonEvidence,
    /// Whether the push it intended is one nothing answered, which the
    /// abandon records refused.
    pub(crate) answers_push: bool,
    /// The request whose unfinished record holds the operation: the one an
    /// abandon releases.
    pub(crate) owner: Option<String>,
}

/// How an ID of the remote journal stands for someone who would abandon
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AbandonableRemote {
    /// The remote journal holds no operation of the ID for the root.
    NotFound,
    /// It holds one this command does not close; see
    /// `reservation::abandonable`.
    Refused,
    /// The operation ran to its end, or was cancelled, and owes nothing.
    Ended { cancelled: bool },
    /// The operation is pending, and this is what it has left.
    Pending(Box<RemoteRemains>),
}

/// What became of a request to abandon a remote operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RemoteAbandonOutcome {
    /// The operation was pending and is closed.
    Abandoned(Box<RemoteRemains>),
    /// It had ended already. Nothing was changed.
    AlreadyEnded,
    /// The remote journal holds no operation of the ID for the root.
    NotFound,
    /// It is one this command does not close. Nothing was changed.
    Refused,
    /// It no longer stands as the caller saw it. Nothing was changed.
    Changed,
}

/// Whether the remote operation `record` has ended with nothing owed:
/// `Some(true)` when it was cancelled, `Some(false)` when it completed. A
/// cancelled synchronization that a restart left owing its reconciliation
/// is still listed, and refuses the retirement of its context: it has not
/// ended for an abandon, which lets that go. Nor has one whose push nothing
/// answered (`unanswered`, `reservation::abandon_evidence`), cancelled as
/// it returned or before it was issued, and not yet recorded refused: a
/// new synchronization of its context counts that push as unanswered, and
/// is ambiguous where the branch is absent, until an abandon records it.
fn ended(record: &StoredRemoteOperation, unanswered: bool) -> Option<bool> {
    let refused =
        record.outcome == Some(crate::repository::RemoteOutcomeCategory::ProtocolRejected);
    match record.phase {
        RemoteOperationPhase::Completed if !record.index_pending => Some(false),
        RemoteOperationPhase::Cancelled
            if !record.index_pending
                && !record.reconciliation_required
                && (!unanswered || refused) =>
        {
            Some(true)
        }
        _ => None,
    }
}

/// The registration whose root the journals name `root_key`, if any.
fn registration(
    connection: &rusqlite::Connection,
    root_key: &str,
) -> rusqlite::Result<Option<i64>> {
    use rusqlite::OptionalExtension;
    connection
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root_key],
            |row| row.get(0),
        )
        .optional()
}

/// The commit `branch` names in the repository at `root`, when it names
/// one that can be read.
fn branch_tip(root: &Path, branch: Option<&str>) -> Option<git2::Oid> {
    let repository = Repository::open_ext(
        root,
        RepositoryOpenFlags::NO_SEARCH,
        &[] as &[&std::ffi::OsStr],
    )
    .ok()?;
    repository
        .find_reference(&format!("refs/heads/{}", branch?))
        .ok()?
        .target()
}

/// What the pending `record` has left, with the request that holds it and
/// what the journal says of its push. The local branch is read afterwards,
/// from Git.
fn remote_remains(
    record: &StoredRemoteOperation,
    owner: Option<String>,
    push: AbandonEvidence,
) -> RemoteRemains {
    RemoteRemains {
        action: record.target.action(),
        standing: RemoteStanding::of(record),
        item_id: record.target.item().map(|(_, item_id)| item_id.to_string()),
        evidence: RemoteAbandonEvidence {
            remote_ref_present: push.observed.map(|observed| observed.is_some()),
            remote_oid: push.observed.flatten(),
            local_oid: None,
            push_oid: push.push_oid,
        },
        answers_push: push.unanswered,
        owner,
    }
}

impl RepositoryService {
    /// How the remote operation `operation_id` of the repository at `root`
    /// stands, and what it has left when it is pending; see
    /// `AbandonableRemote`. Nothing is written, no lease is taken and the
    /// remote is not contacted. It does not say whether a process still
    /// works under it: the caller judges that from the row's holder.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(crate) fn abandonable_remote_operation(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<AbandonableRemote, ReadError> {
        let root = journal_root(root)?;
        let Some(root_key) = root.to_str() else {
            return Err(ReadError::new(ResultCode::InvalidPath));
        };
        let invalid = |_| ReadError::new(ResultCode::RecoveryRequired);
        let found = self.read_session(RepositoryOperation::Read, |connection| {
            if !recovery::has_table(connection, recovery::REMOTE_JOURNAL_TABLE)? {
                return Ok(Err(AbandonableRemote::NotFound));
            }
            let Some(id) = registration(connection, root_key)? else {
                return Ok(Err(AbandonableRemote::NotFound));
            };
            let Some(record) =
                state::read_operation(connection, id, operation_id).map_err(invalid)?
            else {
                return Ok(Err(AbandonableRemote::NotFound));
            };
            let unanswered = reservation::abandon_evidence(connection, &record)
                .map_err(invalid)?
                .unanswered;
            if let Some(cancelled) = ended(&record, unanswered) {
                return Ok(Err(AbandonableRemote::Ended { cancelled }));
            }
            if !reservation::abandonable(connection, &record).map_err(invalid)? {
                return Ok(Err(AbandonableRemote::Refused));
            }
            let owner = mutation::operation_holder(
                connection,
                OperationFamily::Remote,
                root_key,
                operation_id,
                None,
            )?;
            let push = reservation::abandon_evidence(connection, &record).map_err(invalid)?;
            Ok(Ok((
                remote_remains(&record, owner, push),
                record.target.local_branch().map(str::to_owned),
            )))
        })?;
        let (mut remains, branch) = match found {
            Ok(found) => found,
            Err(standing) => return Ok(standing),
        };
        remains.evidence.local_oid = branch_tip(&root, branch.as_deref());
        Ok(AbandonableRemote::Pending(Box::new(remains)))
    }

    /// Whether the holder a remote row names, `host` and `pid`, still works
    /// under it while it is in `phase`, as an operation read judges it
    /// (`liveness::holder_liveness`): a process of `this_host` that exists,
    /// and is not this one, while the row holds the reservation. A row this
    /// very process left is no call's, as for a resume.
    fn remote_holder_works(
        &self,
        this_host: &str,
        host: Option<String>,
        pid: Option<i64>,
        phase: RemoteOperationPhase,
    ) -> bool {
        let holder = liveness::AttributionRow {
            host,
            pid,
            index_heartbeat_fresh: false,
        };
        let worked = liveness::Worked::ByProcess {
            holds_reservation: reservation::holds_reservation(phase),
        };
        liveness::holder_liveness(
            &holder,
            this_host,
            std::process::id(),
            &*self.process_probe,
            worked,
        ) == liveness::Liveness::Live
    }

    /// The remote operations of the repository whose journals name it
    /// `root_key` that `abandon_remote_operation` would close now, oldest
    /// first: synchronizations it allows that no process works under. What
    /// a result that says to look at the repository offers to abandon,
    /// beside the pending local rows; see `outcome::offering_abandon`.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(crate) fn abandonable_remote_operations(
        &self,
        connection: &rusqlite::Connection,
        root_key: &str,
    ) -> Result<Vec<String>, ReadError> {
        if !recovery::has_table(connection, recovery::REMOTE_JOURNAL_TABLE)? {
            return Ok(Vec::new());
        }
        let Some(id) = registration(connection, root_key)? else {
            return Ok(Vec::new());
        };
        let candidates: Vec<(String, Option<String>, Option<i64>)> = connection
            .prepare(
                "SELECT operation_ulid, host, pid FROM remote_operation_records
                  WHERE repository_id = ?1
                    AND action IN ('synchronize_context', 'synchronize_primary')
                    AND authoritative_kind IS NULL AND index_pending = 0
                    AND (phase NOT IN ('completed', 'cancelled')
                         OR (phase = 'cancelled' AND reconciliation_required = 1))
                  ORDER BY id",
            )?
            .query_map([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<_, _>>()?;
        let host = super::super::this_host();
        let invalid = |_| ReadError::new(ResultCode::RecoveryRequired);
        let mut offered = Vec::new();
        for (operation, holder_host, holder_pid) in candidates {
            let operation_id = OperationId::parse(&operation)
                .map_err(|_| ReadError::new(ResultCode::InternalError))?;
            let Some(record) =
                state::read_operation(connection, id, operation_id).map_err(invalid)?
            else {
                continue;
            };
            if reservation::abandonable(connection, &record).map_err(invalid)?
                && !self.remote_holder_works(&host, holder_host, holder_pid, record.phase)
            {
                offered.push(operation);
            }
        }
        Ok(offered)
    }

    /// Closes the pending remote operation `operation_id` of the
    /// repository at `root`, with the person's consent: a synchronization
    /// that `reservation::abandonable` allows, which no process of this
    /// host still works under. In one transaction under the index's
    /// exclusive guard it ends `cancelled`, with a push it intended that
    /// nothing answered recorded refused, and releases the unfinished
    /// request whose record holds it, with that request's confirmation; see
    /// `reservation::close_abandoned`. It touches no file, reference or
    /// worktree, and contacts no remote. A row that is not pending, or that
    /// no longer stands as `expected` says, is left as it is.
    ///
    /// A row whose holder is a process of this host that exists, while the
    /// row holds the reservation, is `RepositoryBusy`, naming it. A process
    /// that is gone, one of another host and one the row does not name are
    /// none at work: it is judged as an operation read judges it
    /// (`liveness::holder_liveness`).
    pub(crate) fn abandon_remote_operation(
        &self,
        root: &Path,
        operation_id: OperationId,
        expected: Option<&RemoteStanding>,
    ) -> Result<RemoteAbandonOutcome, RepositoryError> {
        let operation = RepositoryOperation::AbandonOperation;
        self.require_index_available(operation, Some(root))?;
        let root = journal_root(root)?;
        let root_key = super::super::registry_root_key(&root, operation)?;
        let host = super::super::this_host();
        let _cache_guard = cache_write_guard(&self.registry_path, &root, operation)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(operation, &root))?;
        migrate_registry(&mut connection).map_err(|error| error.for_operation(operation, &root))?;
        let sqlite = |error| RepositoryError::sqlite(error).for_operation(operation, &root);
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        let Some(id) = registration(&transaction, root_key).map_err(sqlite)? else {
            return Ok(RemoteAbandonOutcome::NotFound);
        };
        let Some(record) = state::read_operation(&transaction, id, operation_id)? else {
            return Ok(RemoteAbandonOutcome::NotFound);
        };
        let unanswered = reservation::abandon_evidence(&transaction, &record)?.unanswered;
        if ended(&record, unanswered).is_some() {
            return Ok(RemoteAbandonOutcome::AlreadyEnded);
        }
        if !reservation::abandonable(&transaction, &record)? {
            return Ok(RemoteAbandonOutcome::Refused);
        }
        if expected.is_some_and(|expected| *expected != RemoteStanding::of(&record)) {
            return Ok(RemoteAbandonOutcome::Changed);
        }
        let (holder_host, holder_pid): (Option<String>, Option<i64>) = transaction
            .query_row(
                "SELECT host, pid FROM remote_operation_records WHERE id = ?1",
                [record.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sqlite)?;
        if self.remote_holder_works(&host, holder_host, holder_pid, record.phase) {
            return Err(RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::RepositoryBusy,
                "another process is at work under the operation",
            )
            .with_blocker(Some(crate::repository::Blocker {
                family: OperationFamily::Remote,
                operation_id,
                live: Some(true),
                seen: None,
            })));
        }
        let push = reservation::abandon_evidence(&transaction, &record)?;
        let owner = mutation::release_operation_holder(
            &transaction,
            OperationFamily::Remote,
            root_key,
            operation_id,
        )
        .map_err(sqlite)?;
        let mut remains = remote_remains(&record, owner, push);
        reservation::close_abandoned(&transaction, id, &record)?;
        transaction.commit().map_err(sqlite)?;
        remains.evidence.local_oid = branch_tip(&root, record.target.local_branch());
        Ok(RemoteAbandonOutcome::Abandoned(Box::new(remains)))
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
        let bound_local = super::super::local_sync::bound_to_request(self, &root, operation_id)?;
        let found = self.read_session(RepositoryOperation::Read, |connection| {
            if super::super::context_retirement::record::requires_guarded_abandon(
                connection,
                operation_id,
            )
            .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?
            {
                super::super::context_retirement::record::audit_connection(connection)
                    .map_err(|_| ReadError::new(ResultCode::RecoveryRequired))?;
            }
            // Asked first: such an ID is not a local operation's,
            // whatever the local journal holds under it.
            if !bound_local
                && recovery::held_by_another_journal(connection, &operation_id.to_string())?
            {
                return Ok(Err(AbandonableOperation::AnotherJournal));
            }
            let Some(row) = recovery::local_row(connection, root_key, operation_id)? else {
                return Ok(Err(AbandonableOperation::NotFound));
            };
            if !row.is_well_formed() {
                return Err(ReadError::new(ResultCode::InternalError));
            }
            let owner = mutation::operation_owner(connection, root_key, operation_id)?;
            Ok(Ok((row, owner)))
        })?;
        let (row, owner) = match found {
            Ok(found) => found,
            Err(standing) => return Ok(standing),
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
        let map_local = |code| {
            RepositoryError::new(
                operation,
                Some(root.clone()),
                if code == crate::results::ResultCode::Busy {
                    RepositoryErrorKind::RepositoryBusy
                } else {
                    RepositoryErrorKind::RecoveryRequired
                },
                "local synchronization abandonment requires current authority",
            )
        };
        if super::super::local_sync::bound_to_request(self, &root, operation_id)
            .map_err(|e| map_local(e.code()))?
        {
            let remains = match self
                .abandonable_operation(&root, operation_id)
                .map_err(|e| map_local(e.code()))?
            {
                AbandonableOperation::Pending(left) => left,
                AbandonableOperation::Abandoned => {
                    return Ok(AbandonOperationOutcome::AlreadyAbandoned);
                }
                AbandonableOperation::Completed => return Ok(AbandonOperationOutcome::Completed),
                _ => return Err(map_local(crate::results::ResultCode::RecoveryRequired)),
            };
            let repo = self
                .resolve_repository(&root)
                .map_err(|e| map_local(e.code()))?;
            super::super::local_sync::abandon_bound(self, &repo, operation_id, expected.as_ref())
                .map_err(|e| map_local(e.code))?;
            return Ok(AbandonOperationOutcome::Abandoned(remains));
        }
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
        if super::super::context_retirement::record::requires_guarded_abandon(
            &transaction,
            operation_id,
        )? {
            super::super::context_retirement::record::audit_connection(&transaction)?;
            if let Some(row) =
                recovery::local_row(&transaction, root_key, operation_id).map_err(sqlite)?
            {
                match row.standing() {
                    LocalStanding::Completed => return Ok(AbandonOperationOutcome::Completed),
                    LocalStanding::Rejected => return Ok(AbandonOperationOutcome::Rejected),
                    LocalStanding::Abandoned => {
                        return Ok(AbandonOperationOutcome::AlreadyAbandoned);
                    }
                    LocalStanding::Pending => {}
                }
            }
            return Err(RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::RecoveryRequired,
                "retirement requires guarded preserving abandonment",
            ));
        }
        if recovery::held_by_another_journal(&transaction, &operation_id.to_string())
            .map_err(sqlite)?
        {
            return Ok(AbandonOperationOutcome::AnotherJournal);
        }
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
        // An index owner or a rebuild whose heartbeat is fresh is at
        // work, with no lease held.
        if row.has_live_heartbeat(stale_heartbeat()) {
            return Err(RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::RepositoryBusy,
                "another call is at work under the operation",
            ));
        }
        if let Some(expected) = &expected
            && (expected.state != row.state || expected.step != row.step)
        {
            return Ok(AbandonOperationOutcome::Changed);
        }
        // Local synchronization owns companion Git/recovery authority. The old
        // generic release deletes request identity and cannot implement its atomic
        // S/R/U disposition. Task 7 supplies that route; never fall through here.
        if transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM local_context_sync_records WHERE operation_ulid=?1)",
                [operation_id.to_string()],
                |r| r.get::<_, bool>(0),
            )
            .map_err(sqlite)?
        {
            return Err(RepositoryError::new(
                operation,
                Some(root.clone()),
                RepositoryErrorKind::RecoveryRequired,
                "local synchronization requires guarded abandonment",
            ));
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
