//! The closure checkpoint (C1, C6, C9, C10): the `Close ticket <ULID>`
//! commit written inside the integration lease, the context's move to it,
//! the reconciliation of a stopped `closing` step, and the post-checkpoint
//! resume that merges a written closure without re-consent.
//!
//! Nothing here acquires a lease: the integration holds it. Every object is
//! built in the repository ODB, unreferenced, before any ref moves; the one
//! ref this module moves is the context's: a safe checkout with no ref lock
//! held, then one short guarded ref update (R-T6-3).
use super::{
    BranchAnswer, ClosureAction, PlannedClosure, PreviewTicketClosureRequest,
    observe::{
        self, ClosureContentError, ClosureObservation, closed_by_text, closed_ticket_bytes,
        ticket_path, withdrawn_ticket_bytes,
    },
    record::{self, BranchWrite, ClosureKind, ClosureRecord, ClosureStep, Refusal},
};
use crate::{
    canonical::{self, CanonicalItem, ForgeSetting, ItemId},
    forge::{self, PullRequestLink},
    repository::{
        ClosureBlocker, CommitIdentity, ConfirmedCommitIdentity, FailurePoint, PlannedIntegration,
        RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
        SynchronizationError, SynchronizationOutcome, SynchronizationTarget,
        canonical_repository_root,
        discovery::ContextKey,
        lifecycle_contexts, mark_contexts_dirty,
        promotion::{
            integrate,
            lifecycle::{
                BranchStep, LeasedIntegration, LifecycleError, LifecycleKind, PreIntegration,
            },
            observe as shared,
        },
        remote::{state, sync},
        repository_lease,
        transport::SshTransportErrorKind,
    },
};
use git2::Oid;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

const KIND: LifecycleKind = LifecycleKind::TicketClosure;

/// The checkpoint commit's subject. The merge uses `Integrate closed ticket`:
/// integration reconciliation keys on the merge subject.
fn checkpoint_subject(item: &ItemId) -> String {
    format!("Close ticket {item}")
}

/// Every object a `Record` closure needs, in the ODB and unreferenced.
pub(in super::super) struct PreparedClosure {
    /// Whole UTC seconds; also the checkpoint's author and committer time.
    pub closed_at: OffsetDateTime,
    #[cfg_attr(not(test), allow(dead_code))]
    // Evidence for the tests; production reads it through the tree.
    pub ticket_blob: Oid,
    pub checkpoint_commit: Oid,
    /// The checkpoint tree when `base ≤ context`, else the clean in-memory
    /// merge of the base and the checkpoint.
    pub result_tree: Oid,
}

/// Why no checkpoint can be built as consented. Categories only.
#[derive(Debug)]
pub(in super::super) enum ClosureStop {
    /// The closed ticket or its merge cannot be made as consented (C10, a
    /// conflict or an invalid result): `ReconfirmationRequired`.
    Refused,
    Repository(RepositoryError),
}

/// `at` in UTC, truncated to whole seconds (C6).
pub(in super::super) fn whole_utc_seconds(at: OffsetDateTime) -> OffsetDateTime {
    at.to_offset(UtcOffset::UTC)
        .replace_nanosecond(0)
        .expect("zero nanoseconds are valid")
}
pub(in super::super) fn now_whole_utc_seconds() -> OffsetDateTime {
    whole_utc_seconds(OffsetDateTime::now_utc())
}

fn root_of(repo: &git2::Repository) -> &Path {
    repo.workdir().unwrap_or(repo.path())
}
fn unavailable(repo: &git2::Repository) -> RepositoryError {
    shared::unavailable(root_of(repo))
}

/// Step 5: the closed ticket Blob (C10, mode kept), the checkpoint tree and
/// commit `Close ticket <ULID>` (parent the synchronized context, the
/// identity as author and committer at `closed_at`), and the result tree.
/// The same inputs always build the same commit. Nothing is referenced.
pub(in super::super) fn prepare_closure(
    repo: &git2::Repository,
    observation: &ClosureObservation,
    closed_at: OffsetDateTime,
    identity: &CommitIdentity,
) -> Result<PreparedClosure, ClosureStop> {
    let target = ClosureTarget {
        item: &observation.item_id,
        path: &observation.ticket_path,
        context: observation.context_oid,
        base: observation.base,
    };
    match build_closure(repo, MergeHandle::Stored, target, closed_at, identity) {
        Ok(ClosureBuild::Built(prepared)) => Ok(prepared),
        Ok(ClosureBuild::Invalid | ClosureBuild::Conflict) => Err(ClosureStop::Refused),
        Err(error) => Err(ClosureStop::Repository(error)),
    }
}

/// What a `Record` closure is built from.
#[derive(Clone, Copy)]
pub(in super::super) struct ClosureTarget<'a> {
    pub item: &'a ItemId,
    pub path: &'a Path,
    pub context: Oid,
    pub base: Oid,
}

/// Where the checkpoint merge runs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum MergeHandle {
    /// `repo` writes the repository ODB (execution): the merge is prepared
    /// on a separately opened memory-backed handle and its new objects
    /// imported.
    Stored,
    /// `repo` itself has a high-priority memory ODB (the preview): the
    /// checkpoint lives only there, so the merge runs on `repo`.
    Memory,
}

/// What building a `Record` closure gives (C10).
pub(in super::super) enum ClosureBuild {
    Built(PreparedClosure),
    /// The closed ticket, its checkpoint or the merged result's closure is
    /// not as consented.
    Invalid,
    /// The base and the checkpoint conflict.
    Conflict,
}

/// The closure commit a `Record` closure writes on the context, built and
/// not yet referenced.
pub(in super::super) struct ClosureCommit {
    /// Whole UTC seconds; also the commit's author and committer time.
    pub closed_at: OffsetDateTime,
    pub ticket_blob: Oid,
    pub commit: Oid,
    closed_by: String,
    mode: i32,
}

/// The closed ticket Blob, the checkpoint tree and the commit `Close ticket
/// <ULID>` (parent the context, the identity as author and committer at
/// `closed_at`): the part of a closure that does not depend on where it is
/// merged. `None` when the closed ticket or its commit cannot be made as
/// consented (C10). The same inputs always build the same commit.
pub(in super::super) fn build_closure_commit(
    repo: &git2::Repository,
    item: &ItemId,
    path: &Path,
    context: Oid,
    closed_at: OffsetDateTime,
    identity: &CommitIdentity,
) -> Result<Option<ClosureCommit>, RepositoryError> {
    let closed_at = whole_utc_seconds(closed_at);
    let closed_by = closed_by_text(identity);
    let Some(built) = commit_edited_ticket(
        repo,
        path,
        context,
        closed_at,
        identity,
        &checkpoint_subject(item),
        |original| closed_ticket_bytes(original, closed_at, &closed_by),
    )?
    else {
        return Ok(None);
    };
    Ok(Some(ClosureCommit {
        closed_at,
        ticket_blob: built.ticket_blob,
        commit: built.commit,
        closed_by,
        mode: built.mode,
    }))
}

/// A withdraw's commit subject.
fn withdraw_subject(item: &ItemId) -> String {
    format!("Withdraw closure of ticket {item}")
}

/// The commit `Withdraw closure of ticket <ULID>`: the context's ticket with
/// both closure fields removed (`withdrawn_ticket_bytes`, mode kept), parent
/// the context, the identity as author and committer at `at`. Unreferenced.
/// `None` when the ticket is not a canonical regular file or holds no
/// closure to remove.
pub(in super::super) fn build_withdraw_commit(
    repo: &git2::Repository,
    item: &ItemId,
    path: &Path,
    context: Oid,
    at: OffsetDateTime,
    identity: &CommitIdentity,
) -> Result<Option<Oid>, RepositoryError> {
    Ok(commit_edited_ticket(
        repo,
        path,
        context,
        whole_utc_seconds(at),
        identity,
        &withdraw_subject(item),
        withdrawn_ticket_bytes,
    )?
    .map(|built| built.commit))
}

/// The withdraw commit a record stored, when it is the one `build_withdraw_commit`
/// makes over `context`: one parent, `context`, and a withdraw's subject.
fn stored_withdraw(
    repo: &git2::Repository,
    item: &ItemId,
    context: Oid,
    stored: Option<Oid>,
) -> Option<Oid> {
    let commit = repo.find_commit(stored?).ok()?;
    (commit.parent_count() == 1
        && commit.parent_id(0).ok()? == context
        && commit.summary() == Some(withdraw_subject(item).as_str()))
    .then(|| commit.id())
}

/// What `commit_edited_ticket` wrote.
struct EditedTicket {
    ticket_blob: Oid,
    commit: Oid,
    mode: i32,
}

/// The commit `subject` over `context`: its ticket as `edit` gives it from
/// the ticket's bytes (mode kept), the identity as author and committer at
/// `at`. `None` when the ticket is not a regular file in the tree or `edit`
/// refuses it.
fn commit_edited_ticket(
    repo: &git2::Repository,
    path: &Path,
    context: Oid,
    at: OffsetDateTime,
    identity: &CommitIdentity,
    subject: &str,
    edit: impl FnOnce(&[u8]) -> Result<Vec<u8>, ClosureContentError>,
) -> Result<Option<EditedTicket>, RepositoryError> {
    let failed = |_| unavailable(repo);
    let context = repo.find_commit(context).map_err(failed)?;
    let context_tree = context.tree().map_err(failed)?;
    let Ok(entry) = context_tree.get_path(path) else {
        return Ok(None);
    };
    let mode = match (entry.kind(), entry.filemode()) {
        (Some(git2::ObjectType::Blob), 0o100644) => git2::FileMode::Blob,
        (Some(git2::ObjectType::Blob), 0o100755) => git2::FileMode::BlobExecutable,
        _ => return Ok(None),
    };
    let original = repo.find_blob(entry.id()).map_err(failed)?;
    let Ok(edited) = edit(original.content()) else {
        return Ok(None);
    };
    let ticket_blob = repo.blob(&edited).map_err(failed)?;
    let mut update = git2::build::TreeUpdateBuilder::new();
    update.upsert(path, ticket_blob, mode);
    let tree = update.create_updated(repo, &context_tree).map_err(failed)?;
    let tree = repo.find_tree(tree).map_err(failed)?;
    let when = git2::Time::new(at.unix_timestamp(), 0);
    let Ok(signature) = git2::Signature::new(&identity.name, &identity.email, &when) else {
        return Ok(None);
    };
    let commit = repo
        .commit(None, &signature, &signature, subject, &tree, &[&context])
        .map_err(failed)?;
    Ok(Some(EditedTicket {
        ticket_blob,
        commit,
        mode: entry.filemode(),
    }))
}

/// The closure commit, and the result of merging it into the base: the one
/// function preview and execution both run (C10, final review I-1).
pub(in super::super) fn build_closure(
    repo: &git2::Repository,
    handle: MergeHandle,
    target: ClosureTarget<'_>,
    closed_at: OffsetDateTime,
    identity: &CommitIdentity,
) -> Result<ClosureBuild, RepositoryError> {
    use ClosureBuild::{Conflict, Invalid};
    let Some(built) = build_closure_commit(
        repo,
        target.item,
        target.path,
        target.context,
        closed_at,
        identity,
    )?
    else {
        return Ok(Invalid);
    };
    let Ok(result_tree) = post_checkpoint_result_on(repo, handle, target.base, built.commit)?
    else {
        return Ok(Conflict);
    };
    // The result must carry exactly this closure on a canonical entry.
    if tree_closure(repo, result_tree, target.path)
        != Some((built.closed_at, built.closed_by.clone(), built.mode))
    {
        return Ok(Invalid);
    }
    Ok(ClosureBuild::Built(PreparedClosure {
        closed_at: built.closed_at,
        ticket_blob: built.ticket_blob,
        checkpoint_commit: built.commit,
        result_tree,
    }))
}

/// The closure a tree's ticket carries, with its mode, if it parses as a
/// conforming closed ticket.
fn tree_closure(
    repo: &git2::Repository,
    tree: Oid,
    path: &Path,
) -> Option<(OffsetDateTime, String, i32)> {
    let tree = repo.find_tree(tree).ok()?;
    let entry = tree.get_path(path).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    let source = std::str::from_utf8(blob.content()).ok()?;
    match canonical::parse_item(path, source).ok()? {
        CanonicalItem::Ticket(ticket) => {
            Some((ticket.closed_at?, ticket.closed_by?, entry.filemode()))
        }
        _ => None,
    }
}

/// The closure fields of a commit's ticket, exactly as its front matter
/// holds them; `None` for an open or missing ticket.
fn commit_closure_text(
    repo: &git2::Repository,
    commit: Oid,
    path: &Path,
) -> Option<(String, String)> {
    let tree = repo.find_commit(commit).ok()?.tree().ok()?;
    let entry = tree.get_path(path).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    let values = canonical::front_matter(std::str::from_utf8(blob.content()).ok()?)?;
    Some((
        values.get("closed_at")?.as_str()?.to_owned(),
        values.get("closed_by")?.as_str()?.to_owned(),
    ))
}

/// Whether a commit's ticket names either closure key; `None` when the
/// path is not a ticket that reads.
fn names_closure_key(repo: &git2::Repository, commit: Oid, path: &Path) -> Option<bool> {
    let tree = repo.find_commit(commit).ok()?.tree().ok()?;
    let entry = tree.get_path(path).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    let values = canonical::front_matter(std::str::from_utf8(blob.content()).ok()?)?;
    Some(values.get("closed_at").is_some() || values.get("closed_by").is_some())
}

/// Whether a commit's ticket names either closure key, as the observation
/// reads the primary: a malformed closure is never open.
fn commit_names_closure(repo: &git2::Repository, commit: Oid, path: &Path) -> bool {
    names_closure_key(repo, commit, path).unwrap_or(false)
}

/// Whether a commit's ticket reads and names neither closure key: the state
/// a withdraw leaves.
fn commit_holds_open_ticket(repo: &git2::Repository, commit: Oid, path: &Path) -> bool {
    names_closure_key(repo, commit, path) == Some(false)
}

/// The tree a merge of `tip` into `base` has: the tip's own tree when
/// `base ≤ tip`, otherwise the clean in-memory merge, even with a remote
/// (C9). `Err(())` is a conflict. Only the merge's new objects are written.
pub(in super::super) fn post_checkpoint_result(
    repo: &git2::Repository,
    base: Oid,
    tip: Oid,
) -> Result<Result<Oid, ()>, RepositoryError> {
    post_checkpoint_result_on(repo, MergeHandle::Stored, base, tip)
}

/// `post_checkpoint_result` with the merge run where `handle` says.
fn post_checkpoint_result_on(
    repo: &git2::Repository,
    handle: MergeHandle,
    base: Oid,
    tip: Oid,
) -> Result<Result<Oid, ()>, RepositoryError> {
    let base_in_tip = base == tip
        || repo
            .graph_descendant_of(tip, base)
            .map_err(|_| unavailable(repo))?;
    if base_in_tip {
        return Ok(Ok(repo
            .find_commit(tip)
            .map_err(|_| unavailable(repo))?
            .tree_id()));
    }
    let preparation = match handle {
        MergeHandle::Stored => sync::prepare_clean_merge(repo.path(), base, tip),
        MergeHandle::Memory => sync::prepare_clean_merge_in(repo, repo.path(), base, tip),
    };
    match preparation.map_err(|_| unavailable(repo))? {
        sync::MergePreparation::Clean(prepared) => Ok(Ok(sync::import_prepared_tree(
            repo, &prepared,
        )
        .map_err(|_| unavailable(repo))?)),
        sync::MergePreparation::Conflict => Ok(Err(())),
    }
}

/// The ticket context's own worktree repository.
pub(in super::super) fn context_repository(
    root: &Path,
    item: &ItemId,
) -> Result<git2::Repository, RepositoryError> {
    git2::Repository::open(root.join(".manyhands/worktrees").join(item.to_string()))
        .map_err(|_| shared::unavailable(root))
}

/// Step 6: the safe checkout of the checkpoint in the context worktree, then
/// the guarded ref update `from → to`. No ref lock is held across the
/// checkout, so a stop there leaves none behind (R-T6-3).
#[cfg_attr(not(test), allow(dead_code))] // The tests' form; production passes its failure point.
pub(in super::super) fn move_context_to_checkpoint(
    context_repo: &git2::Repository,
    item: &ItemId,
    from: Oid,
    to: Oid,
) -> Result<(), RepositoryError> {
    move_context_to_checkpoint_with(context_repo, item, from, to, || Ok(()))
}
/// `move_context_to_checkpoint` with `between` run after the checkout and
/// before the ref update (`ClosureDuringCheckpointCheckout`).
pub(in super::super) fn move_context_to_checkpoint_with(
    context_repo: &git2::Repository,
    item: &ItemId,
    from: Oid,
    to: Oid,
    between: impl FnOnce() -> Result<(), RepositoryError>,
) -> Result<(), RepositoryError> {
    let branch = KIND.context_ref(item);
    let commit = context_repo
        .find_commit(to)
        .map_err(|_| unavailable(context_repo))?;
    // Under the integration lease and without the ref lock: a stop during
    // the checkout or at `between` leaves no lock behind (R-T6-3).
    if !context_at(context_repo, &branch, from) {
        return Err(shared::changed());
    }
    let mut checkout = git2::build::CheckoutBuilder::new();
    // SAFE alone defaults to overwriting ignored user entries in libgit2.
    checkout.safe().overwrite_ignored(false);
    context_repo
        .checkout_tree(commit.as_object(), Some(&mut checkout))
        .map_err(|_| unavailable(context_repo))?;
    between()?;
    guarded_context_move(context_repo, &branch, from, to)
}

/// The ref is at `oid` and HEAD names it.
fn context_at(repo: &git2::Repository, branch: &str, oid: Oid) -> bool {
    repo.refname_to_id(branch).ok() == Some(oid)
        && repo
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(str::to_owned))
            .as_deref()
            == Some(branch)
}

/// The one ref effect of step 6: lock, re-check the ref and HEAD, move
/// `from → to`, commit. The lock is held only for this transaction.
fn guarded_context_move(
    repo: &git2::Repository,
    branch: &str,
    from: Oid,
    to: Oid,
) -> Result<(), RepositoryError> {
    let failed = |_| unavailable(repo);
    let mut transaction = repo.transaction().map_err(failed)?;
    transaction.lock_ref(branch).map_err(failed)?;
    if !context_at(repo, branch, from) {
        return Err(shared::changed());
    }
    transaction
        .set_target(branch, to, None, "manyhands closure checkpoint")
        .map_err(failed)?;
    transaction.commit().map_err(failed)
}

/// What a `closing` record left in the context, read under the lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ClosingState {
    /// The ref is at `context_oid` and the ticket path is unchanged in the
    /// index and the worktree; other entries are revalidation's to report.
    NothingWritten,
    /// The ref is at `context_oid` and only the ticket path differs, in the
    /// worktree and possibly the index (R-T0-3), holding exactly the stored
    /// checkpoint's Blob: the checkout of step 6 ran.
    CheckoutDone,
    /// The ref is at the record's written commit (`closure_oid`, or
    /// `withdraw_oid` for a withdraw), or at a descendant whose ticket keeps
    /// the state that commit gave it (a deliberate synchronization): the
    /// same closure fields for a close, no closure at all for a withdraw.
    CheckpointWritten {
        tip: Oid,
    },
    Unrecognized,
}

/// The design's reconciliation of a `closing` record.
pub(in super::super) fn reconcile_closing(
    context_repo: &git2::Repository,
    record: &ClosureRecord,
) -> Result<ClosingState, RepositoryError> {
    let (Some(context), Some(closure)) = (record.context_oid, record.written_commit()) else {
        return Ok(ClosingState::Unrecognized);
    };
    let context_ref = KIND.context_ref(&record.item_id);
    let path = ticket_path(&record.item_id);
    let head_on_context = context_repo
        .find_reference("HEAD")
        .ok()
        .and_then(|r| r.symbolic_target().map(str::to_owned))
        .as_deref()
        == Some(context_ref.as_str());
    let Some(tip) = context_repo.refname_to_id(&context_ref).ok() else {
        return Ok(ClosingState::Unrecognized);
    };
    if !head_on_context {
        return Ok(ClosingState::Unrecognized);
    }
    if tip == context {
        let mut options = git2::StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);
        let statuses = context_repo
            .statuses(Some(&mut options))
            .map_err(|_| unavailable(context_repo))?;
        // An unchanged ticket path, in the index and the worktree, proves
        // the checkout never ran, whatever else the worktree holds; the
        // revalidation then reports any other entry as `ContextDirty`
        // (final review M-1).
        let ticket_changed = statuses
            .iter()
            .any(|entry| entry.path_bytes() == path.as_os_str().as_encoded_bytes());
        if !ticket_changed {
            return Ok(ClosingState::NothingWritten);
        }
        let Some(blob) = context_repo
            .find_commit(closure)
            .ok()
            .and_then(|c| c.tree().ok())
            .and_then(|t| t.get_path(&path).ok())
            .map(|e| e.id())
        else {
            return Ok(ClosingState::Unrecognized);
        };
        let allowed = git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED;
        let only_ticket = statuses.len() == 1
            && statuses.iter().all(|entry| {
                entry.path_bytes() == path.as_os_str().as_encoded_bytes()
                    && !entry.status().is_empty()
                    && allowed.contains(entry.status())
            });
        if !only_ticket {
            return Ok(ClosingState::Unrecognized);
        }
        // Git decides, with its filters: never a raw byte comparison.
        let status = statuses
            .get(0)
            .map(|e| e.status())
            .unwrap_or(git2::Status::CURRENT);
        let staged = context_repo
            .index()
            .ok()
            .and_then(|index| index.get_path(&path, 0))
            .map(|entry| entry.id);
        let matches = if status.contains(git2::Status::INDEX_MODIFIED) {
            // The checkout staged the Blob and the worktree equals the index.
            staged == Some(blob) && !status.contains(git2::Status::WT_MODIFIED)
        } else {
            // The worktree alone changed: it must equal the checkpoint's tree.
            let checkpoint_tree = context_repo
                .find_commit(closure)
                .and_then(|c| c.tree())
                .map_err(|_| unavailable(context_repo))?;
            let mut options = git2::DiffOptions::new();
            options.pathspec(&path).disable_pathspec_match(true);
            context_repo
                .diff_tree_to_workdir(Some(&checkpoint_tree), Some(&mut options))
                .map_err(|_| unavailable(context_repo))?
                .deltas()
                .len()
                == 0
        };
        return Ok(if matches {
            ClosingState::CheckoutDone
        } else {
            ClosingState::Unrecognized
        });
    }
    if tip == closure {
        return Ok(ClosingState::CheckpointWritten { tip });
    }
    let descendant = context_repo
        .graph_descendant_of(tip, closure)
        .map_err(|_| unavailable(context_repo))?;
    // The state the written commit gave the ticket, which the descendant
    // keeps: a withdraw's is an open ticket, a close's its closure.
    let keeps_state = || match record.action {
        ClosureAction::Withdraw => commit_holds_open_ticket(context_repo, tip, &path),
        _ => {
            let closure_fields = commit_closure_text(context_repo, closure, &path);
            closure_fields.is_some()
                && commit_closure_text(context_repo, tip, &path) == closure_fields
        }
    };
    Ok(if descendant && keeps_state() {
        ClosingState::CheckpointWritten { tip }
    } else {
        ClosingState::Unrecognized
    })
}

/// The closure's authority after a checkpoint (C9): `closed_by` from the
/// checkpoint's ticket Blob and the merge signature from its author.
pub(in super::super) struct CheckpointAuthority {
    #[cfg_attr(not(test), allow(dead_code))]
    // Proven to name the author; the merge needs only the signature.
    pub closed_by: String,
    pub signature: CommitIdentity,
}

/// Read from the record's checkpoint commit itself, never from current
/// configuration: its subject is `Close ticket <item>`, it has exactly one
/// parent whose ticket is open, its own ticket is closed, and `closed_by`
/// names its author.
pub(in super::super) fn checkpoint_authority(
    repo: &git2::Repository,
    item: &ItemId,
    closure: Oid,
) -> Result<CheckpointAuthority, RepositoryError> {
    let failed = || unavailable(repo);
    let commit = repo.find_commit(closure).map_err(|_| failed())?;
    if commit.summary() != Some(checkpoint_subject(item).as_str()) || commit.parent_count() != 1 {
        return Err(failed());
    }
    let path = ticket_path(item);
    let parent = commit.parent_id(0).map_err(|_| failed())?;
    let (Some((_, closed_by)), None) = (
        commit_closure_text(repo, closure, &path),
        commit_closure_text(repo, parent, &path),
    ) else {
        return Err(failed());
    };
    let author = commit.author();
    let (Some(name), Some(email)) = (author.name(), author.email()) else {
        return Err(failed());
    };
    let signature = CommitIdentity {
        name: name.to_owned(),
        email: email.to_owned(),
    };
    if closed_by_text(&signature) != closed_by {
        return Err(failed());
    }
    Ok(CheckpointAuthority {
        closed_by,
        signature,
    })
}

fn boundary(
    service: &RepositoryService,
    root: &Path,
    point: FailurePoint,
) -> Result<(), RepositoryError> {
    service.check_failure(point, RepositoryOperation::RepositorySnapshot, root)
}

/// The context ref is about to move to the checkpoint, its worktree checked
/// out at it: the ticket's context is marked owed an indexing first, under
/// the integration's lease, as a synchronization marks the context its local
/// update moves before Git is touched (R-SDD-49). Every store takes that
/// lease, so none can clear the mark before the move is made, and a process
/// lost during the checkout or before `context_moved` leaves the context
/// reading owed until an indexing stores it (the final review's F-1). A mark
/// that cannot be written refuses the move, with nothing changed. The move
/// changes the ticket's context alone; `context_moved` marks the primary too.
pub(super) fn context_moving(
    service: &RepositoryService,
    root: &Path,
    item: &ItemId,
) -> Result<(), RepositoryError> {
    mark_contexts_dirty(
        &service.registry_path,
        root,
        &[ContextKey::Item(item.clone())],
        RepositoryOperation::RepositorySnapshot,
    )
}

/// The context ref moved: the context now reads closed. The closure's
/// contexts are marked owed an indexing, its own and the primary's.
fn context_moved(
    service: &RepositoryService,
    root: &Path,
    item: &ItemId,
) -> Result<(), RepositoryError> {
    mark_contexts_dirty(
        &service.registry_path,
        root,
        &lifecycle_contexts(item),
        RepositoryOperation::RepositorySnapshot,
    )
}

/// "Before a checkpoint exists", steps 2–6, as the integration hook runs them
/// after revalidation (step 1) under the lease. A `Record` closure gets its
/// checkpoint and returns it to merge; `Existing` and `AlreadyContained` write
/// nothing (`None`): the shared integration resolves their merge signer or
/// records the base. A refusal before step 5's SQL write changes nothing; a
/// stop after it leaves `closing` for `resume_written`.
pub(in super::super) fn pre_integration<E: LifecycleError>(
    service: &RepositoryService,
    observation: &ClosureObservation,
    confirmation: Option<&ConfirmedCommitIdentity>,
    stage: &mut LeasedIntegration<'_, ClosureRecord>,
) -> Result<PreIntegration, E> {
    let PlannedClosure::Record { closed_by } = &observation.closure else {
        return Ok(PreIntegration::None);
    };
    if observation.integration == PlannedIntegration::AlreadyContained {
        return Ok(PreIntegration::None);
    }
    let r = stage.record.ok_or_else(E::recovery_required)?;
    let closing = r.step == ClosureStep::Closing;
    if !(closing || r.step == ClosureStep::ContextSynchronized)
        || r.closure_kind != Some(ClosureKind::Record)
    {
        return Err(E::recovery_required());
    }
    let (repo, root, base) = (stage.repository, stage.root, stage.base);
    let context = stage.context.oid;
    if Some(context) != r.context_oid || context != observation.context_oid {
        return Err(E::external_change());
    }
    let ancestor =
        |older: Oid, newer: Oid| shared::ancestor(repo, older, newer, root).map_err(E::from);
    let contained = ancestor(context, base)?;
    // Step 2: never merge further work into a ticket the base closed; the
    // next preview reports `AlreadyClosed`.
    if !contained && commit_names_closure(repo, base, &observation.ticket_path) {
        return Err(E::reconfirmation_required());
    }
    // Step 3: with a remote, a context that is not contained must hold the
    // base. A `closing` record cannot step back to `confirmed`: it abandons.
    if observation.remote.is_some() && !contained && !ancestor(base, context)? {
        return Err(if closing {
            E::reconfirmation_required()
        } else {
            E::primary_advanced()
        });
    }
    if closing && r.base_oid != Some(base) {
        return Err(E::reconfirmation_required());
    }
    // A synchronization of this context still running holds the checkpoint
    // off before anything is written, the identity included; the check is
    // repeated in the `closing` transaction below.
    if state::with_transaction(service, root, |tx, id| {
        record::context_synchronization_unfinished(tx, id, &r.item_id)
    })? {
        return Err(E::busy());
    }
    // Step 4: persist a confirmed identity before any commit (T3-1, Q3-1).
    // A live identity other than the consented `closed_by` is
    // `ExternalChange` (C2, final review M-3). It is read first without
    // persisting anything (M-4), and re-checked after the persisting call,
    // which reads configuration afresh. A selection error is left to that
    // call, which reports it as promotion does.
    if let Ok(sync::CommittingIdentity::Effective(live) | sync::CommittingIdentity::Confirmed(live)) =
        sync::select_committing_identity(repo, confirmation, &SynchronizationTarget::Primary)
        && closed_by_text(&live) != *closed_by
    {
        return Err(E::external_change());
    }
    let identity = integrate::promotion_identity::<E>(service, repo, root, confirmation)?;
    if closed_by_text(&identity) != *closed_by {
        return Err(E::external_change());
    }
    // Step 5: the stored `closed_at` on a rebuild, so the same commit results.
    let closed_at = r.closed_at.unwrap_or_else(now_whole_utc_seconds);
    let prepared = match prepare_closure(repo, observation, closed_at, &identity) {
        Ok(prepared) => prepared,
        Err(ClosureStop::Refused) => return Err(E::reconfirmation_required()),
        Err(ClosureStop::Repository(error)) => return Err(error.into()),
    };
    let operation = r.operation_id;
    // C1, C9: a synchronization of this context still running would push
    // the checkpoint; this transaction and its reservation exclude each
    // other (`written_closure_unfinished` refuses it from `closing` on).
    let held_off = state::with_transaction(service, root, |tx, id| {
        if record::context_synchronization_unfinished(tx, id, &r.item_id)? {
            return Ok(true);
        }
        if !closing {
            record::begin_closing(
                tx,
                id,
                operation,
                base,
                prepared.closed_at,
                prepared.checkpoint_commit,
            )?;
        } else if r.closure_oid != Some(prepared.checkpoint_commit) {
            record::replace_unmoved_closure(tx, id, operation, prepared.checkpoint_commit)?;
        }
        Ok(false)
    })?;
    if held_off {
        return Err(E::busy());
    }
    boundary(service, root, FailurePoint::ClosureAfterClosingRecorded)?;
    // Step 6, on the context worktree's repository.
    let context_repo = context_repository(root, &r.item_id)?;
    context_moving(service, root, &r.item_id)?;
    move_context_to_checkpoint_with(
        &context_repo,
        &r.item_id,
        context,
        prepared.checkpoint_commit,
        || boundary(service, root, FailurePoint::ClosureDuringCheckpointCheckout),
    )?;
    context_moved(service, root, &r.item_id)?;
    boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
    Ok(PreIntegration::Checkpoint {
        base,
        merged_context: prepared.checkpoint_commit,
        result_tree: prepared.result_tree,
        signer: identity,
    })
}

/// "After a checkpoint exists" (C9), before any consent check: a record at
/// `closing` whose checkout or ref move happened, or at `integrating` with a
/// checkpoint, is merged with the checkpoint's own authority, never
/// abandoned. `None` when nothing was written (`closing`, nothing moved) or
/// nothing ever will be. A conflicting merge is `RecoveryRequired` with the
/// record unchanged.
pub(in super::super) fn resume_written<E: LifecycleError>(
    service: &RepositoryService,
    stage: &mut LeasedIntegration<'_, ClosureRecord>,
) -> Result<Option<PreIntegration>, E> {
    let Some(r) = stage.record else {
        return Ok(None);
    };
    let Some(closure) = r.closure_oid else {
        return Ok(None);
    };
    let (repo, root) = (stage.repository, stage.root);
    let (tip, base) = match r.step {
        ClosureStep::Closing => {
            let context_repo = context_repository(root, &r.item_id)?;
            let tip = match reconcile_closing(&context_repo, r)? {
                ClosingState::NothingWritten => return Ok(None),
                ClosingState::CheckoutDone => {
                    // The checkout holds no ref lock (R-T6-3). A lock held
                    // by another writer is never adopted or removed: the
                    // record is kept for a retry. A store since the stopped
                    // call may have cleared its mark, so the move this call
                    // makes is marked first, as the stopped call's was.
                    context_moving(service, root, &r.item_id)?;
                    finish_context_move(&context_repo, r).map_err(|_| E::recovery_required())?;
                    context_moved(service, root, &r.item_id)?;
                    boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
                    closure
                }
                ClosingState::CheckpointWritten { tip } => {
                    // The ref moved before this call (or after a synchronization):
                    // the mark is idempotent and may not have been made.
                    context_moved(service, root, &r.item_id)?;
                    tip
                }
                ClosingState::Unrecognized => return Err(E::recovery_required()),
            };
            (
                tip,
                current_base::<E>(repo, r, &stage.context.primary_branch)?,
            )
        }
        ClosureStep::Integrating => {
            let base = r.base_oid.ok_or_else(E::recovery_required)?;
            // The primary as read under this lease must not have moved past the
            // recorded base since the runner recorded `integrating`.
            let old_primary = stage.base;
            if old_primary != base
                && !repo
                    .graph_descendant_of(base, old_primary)
                    .map_err(|_| E::recovery_required())?
            {
                return Err(E::recovery_required());
            }
            (r.merged_context().ok_or_else(E::recovery_required)?, base)
        }
        _ => return Ok(None),
    };
    // The authority is always the checkpoint commit itself, never an adopted
    // synchronization over it.
    let authority =
        checkpoint_authority(repo, &r.item_id, closure).map_err(|_| E::recovery_required())?;
    let Ok(result_tree) = post_checkpoint_result(repo, base, tip)? else {
        return Err(E::recovery_required());
    };
    let adopted = (tip != closure).then_some(tip);
    if r.step == ClosureStep::Closing
        && (r.base_oid != Some(base) || r.adopted_context_oid != adopted)
    {
        let operation = r.operation_id;
        state::with_transaction(service, root, |tx, id| {
            record::adopt_written_closure(tx, id, operation, repo, base, adopted)
        })?;
    }
    Ok(Some(PreIntegration::Checkpoint {
        base,
        merged_context: tip,
        result_tree,
        signer: authority.signature,
    }))
}

/// C9 step 3: with a remote, the tracking primary, the local primary equal
/// to it or behind (D2); without one, the local primary.
fn current_base<E: LifecycleError>(
    repo: &git2::Repository,
    r: &ClosureRecord,
    primary: &str,
) -> Result<Oid, E> {
    let local = repo
        .refname_to_id(&format!("refs/heads/{primary}"))
        .map_err(|_| E::recovery_required())?;
    let Some(remote) = r.remote_name.as_deref() else {
        return Ok(local);
    };
    let tracking = repo
        .refname_to_id(&format!("refs/remotes/{remote}/{primary}"))
        .map_err(|_| E::recovery_required())?;
    let behind = local == tracking
        || repo
            .graph_descendant_of(tracking, local)
            .map_err(|_| E::recovery_required())?;
    if !behind {
        return Err(E::recovery_required());
    }
    Ok(tracking)
}

/// `CheckoutDone`: the guarded ref update `context_oid → closure_oid` that
/// the stop interrupted; the checkout itself is already in place.
fn finish_context_move(
    context_repo: &git2::Repository,
    r: &ClosureRecord,
) -> Result<(), RepositoryError> {
    let (Some(from), Some(to)) = (r.context_oid, r.written_commit()) else {
        return Err(state::recovery_required());
    };
    guarded_context_move(context_repo, &KIND.context_ref(&r.item_id), from, to)
}

/// The pull request link of the ticket's branch: where to open the request
/// of `item`'s context branch into `primary`, from the forge the setting or
/// the remote's address names. A remote that is gone gives no address, and a
/// link with no URL.
pub(in super::super) fn branch_link(
    repo: &git2::Repository,
    primary: &str,
    forge_setting: Option<&ForgeSetting>,
    remote: &str,
    item: &ItemId,
) -> PullRequestLink {
    let address = repo
        .find_remote(remote)
        .ok()
        .and_then(|remote| remote.url().map(str::to_owned));
    forge::pull_request_link(
        forge::recognise(address.as_deref(), forge_setting).as_ref(),
        &KIND.context_branch(item),
        primary,
    )
}

/// Step 3 of a branch `close`, under its own lease (R-34): with the
/// context synchronized, the closure it consented to is written on the
/// context, or the record ends with the answer that says why not. A
/// `record` closure is built and recorded as `closing` before the context
/// moves to it, as the direct close's is; an `existing` one writes nothing.
/// Nothing here merges into the primary, or moves it.
///
/// An abandon is one transaction under the lease, and a record at `closing`
/// is abandoned only while its reconciliation proves nothing was written.
pub(in super::super) fn branch_step<E: LifecycleError>(
    service: &RepositoryService,
    root: &Path,
    confirmation: Option<&ConfirmedCommitIdentity>,
    now: OffsetDateTime,
    r: &ClosureRecord,
) -> Result<BranchStep, E> {
    let item = &r.item_id;
    // The consent is observed outside the lease, in the record's own mode.
    // A synchronization of the context that this record does not own is
    // `busy`, and names the row: the observation leaves it to the hold.
    let request = PreviewTicketClosureRequest {
        root: root.to_owned(),
        item_id: item.clone(),
        confirmed_identity: confirmation.cloned(),
    };
    let prepared = observe::prepare_observation_in(service, &request, Some(r.integration_mode))?;
    if let Some(operation) = state::with_transaction(service, root, |tx, id| {
        observe::foreign_synchronization(tx, id, item)
    })? {
        return Err(observe::foreign_synchronization_busy(root, &operation).into());
    }
    let (repo, root) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
    let root = root.as_path();
    let _lease = repository_lease(&repo, root, RepositoryOperation::RepositorySnapshot)?;
    let primary = prepared.configuration.primary_branch.as_str();
    let remote = r.remote_name.as_deref().ok_or_else(E::recovery_required)?;
    let context = r.context_oid.ok_or_else(E::recovery_required)?;
    let path = ticket_path(item);
    let context_ref = KIND.context_ref(item);
    let record = r.closure_kind == Some(ClosureKind::Record);
    let given_at = now.unix_timestamp();
    let link = || {
        branch_link(
            &repo,
            primary,
            prepared.configuration.forge().as_ref(),
            remote,
            item,
        )
    };
    let abandon = |refusal: Refusal, answer: BranchAnswer, link: Option<PullRequestLink>| {
        state::with_transaction(service, root, |tx, id| {
            record::abandon_branch_record(
                tx,
                id,
                r.operation_id,
                &refusal,
                answer,
                link.as_ref(),
                given_at,
            )
        })
        .map(|()| BranchStep::Finished)
        .map_err(E::from)
    };
    // A record at `closing` holds what an earlier call recorded.
    if r.step == ClosureStep::Closing {
        if !record {
            return Ok(BranchStep::Closing);
        }
        let context_repo = context_repository(root, item)?;
        match reconcile_closing(&context_repo, r)? {
            ClosingState::CheckoutDone => {
                context_moving(service, root, item)?;
                finish_context_move(&context_repo, r).map_err(|_| E::recovery_required())?;
                context_moving(service, root, item)?;
                boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
                return Ok(BranchStep::Closing);
            }
            ClosingState::CheckpointWritten { .. } => {
                context_moving(service, root, item)?;
                return Ok(BranchStep::Closing);
            }
            ClosingState::Unrecognized => return Err(E::recovery_required()),
            ClosingState::NothingWritten => (),
        }
    }
    // The context holds what the child left it.
    if repo.refname_to_id(&context_ref).ok() != Some(context) {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange, None);
    }
    // 3.1: the tracking primary holds a closure already.
    if repo
        .refname_to_id(&format!("refs/remotes/{remote}/{primary}"))
        .is_ok_and(|tracking| commit_names_closure(&repo, tracking, &path))
    {
        return abandon(
            Refusal::Blocked(vec![ClosureBlocker::AlreadyClosed]),
            BranchAnswer::AlreadyClosed,
            None,
        );
    }
    // 3.2: the context's tip holds one: a `record` closure was made
    // elsewhere, and what is left is its pull request.
    if record && commit_names_closure(&repo, context, &path) {
        return abandon(
            Refusal::ExternalChange,
            BranchAnswer::AlreadyClosing,
            Some(link()),
        );
    }
    let observed = match observe::revalidate_under_lease(&prepared, service, &repo) {
        Ok(observed) => observed,
        Err(error) if error.kind == RepositoryErrorKind::ExternalChange => {
            return abandon(
                Refusal::ReconfirmationRequired,
                BranchAnswer::ReconfirmationRequired,
                None,
            );
        }
        Err(error) => return Err(error.into()),
    };
    let Ok(observation) = observed else {
        return abandon(
            Refusal::ReconfirmationRequired,
            BranchAnswer::ReconfirmationRequired,
            None,
        );
    };
    // What the request consented to, with the head the child may have moved
    // as the confirm bound it (D21).
    if observe::branch_expectation_with_head(&observation, r.consented_context_oid) != r.expectation
    {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange, None);
    }
    // The digest holds the closure's effect to the record's kind: an
    // `existing` one is written already, a `record` one is built below.
    let closed_by = match (&observation.closure, record) {
        (PlannedClosure::Record { closed_by }, true) => closed_by,
        (PlannedClosure::Existing { .. }, false) => {
            let held_off = state::with_transaction(service, root, |tx, id| {
                if record::context_synchronization_unfinished(tx, id, item)? {
                    return Ok(true);
                }
                record::begin_branch_closing(
                    tx,
                    id,
                    r.operation_id,
                    observation.base,
                    BranchWrite::Existing,
                )?;
                Ok(false)
            })?;
            return if held_off {
                Err(E::busy())
            } else {
                Ok(BranchStep::Closing)
            };
        }
        _ => return Err(E::recovery_required()),
    };
    // 3.3: a synchronization of this context still running holds the commit
    // off before anything is written, the identity included.
    if state::with_transaction(service, root, |tx, id| {
        record::context_synchronization_unfinished(tx, id, item)
    })? {
        return Err(E::busy());
    }
    if let Ok(sync::CommittingIdentity::Effective(live) | sync::CommittingIdentity::Confirmed(live)) =
        sync::select_committing_identity(&repo, confirmation, &SynchronizationTarget::Primary)
        && closed_by_text(&live) != *closed_by
    {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange, None);
    }
    let identity = integrate::promotion_identity::<E>(service, &repo, root, confirmation)?;
    if closed_by_text(&identity) != *closed_by {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange, None);
    }
    // The stored `closed_at` on a rebuild, so the same commit results.
    let closed_at = r.closed_at.unwrap_or_else(|| whole_utc_seconds(now));
    let Some(built) = build_closure_commit(&repo, item, &path, context, closed_at, &identity)?
    else {
        return abandon(
            Refusal::ReconfirmationRequired,
            BranchAnswer::ReconfirmationRequired,
            None,
        );
    };
    let held_off = state::with_transaction(service, root, |tx, id| {
        if record::context_synchronization_unfinished(tx, id, item)? {
            return Ok(true);
        }
        if r.step != ClosureStep::Closing {
            record::begin_branch_closing(
                tx,
                id,
                r.operation_id,
                observation.base,
                BranchWrite::Closure {
                    closed_at: built.closed_at,
                    commit: built.commit,
                },
            )?;
        } else if r.closure_oid != Some(built.commit) {
            record::replace_unmoved_closure(tx, id, r.operation_id, built.commit)?;
        }
        Ok(false)
    })?;
    if held_off {
        return Err(E::busy());
    }
    boundary(service, root, FailurePoint::ClosureAfterClosingRecorded)?;
    let context_repo = context_repository(root, item)?;
    context_moving(service, root, item)?;
    move_context_to_checkpoint_with(&context_repo, item, context, built.commit, || {
        boundary(service, root, FailurePoint::ClosureDuringCheckpointCheckout)
    })?;
    // The move marks the ticket's context alone: nothing reaches the primary.
    context_moving(service, root, item)?;
    boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
    Ok(BranchStep::Closing)
}

/// Step 3 of a `withdraw` (Part E), under its own lease: with the context
/// synchronized, the commit that removes the closure it consented to is
/// written on the context, or the record ends with the answer that says why
/// not. The commit is built and recorded as `closing` before the context
/// moves to it, as a closure's is. It shares `branch_step`'s parts, and none
/// of the direct hook's checks of the base: nothing here merges into the
/// primary, or moves it, and a withdraw gives no link.
///
/// An abandon is one transaction under the lease, and a record at `closing`
/// is abandoned only while its reconciliation proves nothing was written.
pub(in super::super) fn withdraw_step<E: LifecycleError>(
    service: &RepositoryService,
    root: &Path,
    confirmation: Option<&ConfirmedCommitIdentity>,
    now: OffsetDateTime,
    r: &ClosureRecord,
) -> Result<BranchStep, E> {
    let item = &r.item_id;
    // The consent is observed outside the lease, in the record's own mode,
    // as a withdraw.
    let request = PreviewTicketClosureRequest {
        root: root.to_owned(),
        item_id: item.clone(),
        confirmed_identity: confirmation.cloned(),
    };
    let prepared = observe::prepare_observation_for(
        service,
        &request,
        Some(r.integration_mode),
        observe::Verb::Withdraw,
    )?;
    if let Some(operation) = state::with_transaction(service, root, |tx, id| {
        observe::foreign_synchronization(tx, id, item)
    })? {
        return Err(observe::foreign_synchronization_busy(root, &operation).into());
    }
    let (repo, root) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
    let root = root.as_path();
    let _lease = repository_lease(&repo, root, RepositoryOperation::RepositorySnapshot)?;
    let primary = prepared.configuration.primary_branch.as_str();
    let remote = r.remote_name.as_deref().ok_or_else(E::recovery_required)?;
    let context = r.context_oid.ok_or_else(E::recovery_required)?;
    let path = ticket_path(item);
    let context_ref = KIND.context_ref(item);
    let given_at = now.unix_timestamp();
    let abandon = |refusal: Refusal, answer: BranchAnswer| {
        state::with_transaction(service, root, |tx, id| {
            record::abandon_branch_record(tx, id, r.operation_id, &refusal, answer, None, given_at)
        })
        .map(|()| BranchStep::Finished)
        .map_err(E::from)
    };
    // A record at `closing` holds what an earlier call recorded: the commit
    // it stores is the one the context moves to.
    if r.step == ClosureStep::Closing {
        let context_repo = context_repository(root, item)?;
        match reconcile_closing(&context_repo, r)? {
            ClosingState::CheckoutDone => {
                context_moving(service, root, item)?;
                finish_context_move(&context_repo, r).map_err(|_| E::recovery_required())?;
                context_moving(service, root, item)?;
                boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
                return Ok(BranchStep::Closing);
            }
            ClosingState::CheckpointWritten { .. } => {
                context_moving(service, root, item)?;
                return Ok(BranchStep::Closing);
            }
            ClosingState::Unrecognized => return Err(E::recovery_required()),
            ClosingState::NothingWritten => (),
        }
    }
    // The context holds what the child left it.
    if repo.refname_to_id(&context_ref).ok() != Some(context) {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange);
    }
    // 3.1: the tracking primary holds a closure: withdrawing it would be a
    // reopen.
    if repo
        .refname_to_id(&format!("refs/remotes/{remote}/{primary}"))
        .is_ok_and(|tracking| commit_names_closure(&repo, tracking, &path))
    {
        return abandon(
            Refusal::Blocked(vec![ClosureBlocker::AlreadyClosed]),
            BranchAnswer::AlreadyClosed,
        );
    }
    // 3.2: the head holds no closure to withdraw (another clone's withdraw
    // arrived), or one that does not read.
    if !commit_names_closure(&repo, context, &path) {
        return abandon(Refusal::ExternalChange, BranchAnswer::AlreadyApplied);
    }
    if commit_closure_text(&repo, context, &path).is_none() {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange);
    }
    let observed = match observe::revalidate_under_lease(&prepared, service, &repo) {
        Ok(observed) => observed,
        Err(error) if error.kind == RepositoryErrorKind::ExternalChange => {
            return abandon(
                Refusal::ReconfirmationRequired,
                BranchAnswer::ReconfirmationRequired,
            );
        }
        Err(error) => return Err(error.into()),
    };
    let Ok(observation) = observed else {
        return abandon(
            Refusal::ReconfirmationRequired,
            BranchAnswer::ReconfirmationRequired,
        );
    };
    // What the request consented to, with the head the child may have moved
    // as the confirm bound it. Another closure on the head is another
    // digest.
    if observe::branch_expectation_with_head(&observation, r.consented_context_oid) != r.expectation
    {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange);
    }
    if !matches!(observation.closure, PlannedClosure::Existing { .. }) {
        return Err(E::recovery_required());
    }
    // A synchronization of this context still running holds the commit off
    // before anything is written, the identity included.
    if state::with_transaction(service, root, |tx, id| {
        record::context_synchronization_unfinished(tx, id, item)
    })? {
        return Err(E::busy());
    }
    // A withdraw stores no time to rebuild with, and a rebuild would be a
    // second commit: a record at `closing` moves to the commit it stored.
    let built = if r.step == ClosureStep::Closing {
        stored_withdraw(&repo, item, context, r.withdraw_oid).ok_or_else(E::recovery_required)?
    } else {
        let identity = integrate::promotion_identity::<E>(service, &repo, root, confirmation)?;
        let Some(built) = build_withdraw_commit(&repo, item, &path, context, now, &identity)?
        else {
            return abandon(
                Refusal::ReconfirmationRequired,
                BranchAnswer::ReconfirmationRequired,
            );
        };
        built
    };
    let held_off = state::with_transaction(service, root, |tx, id| {
        if record::context_synchronization_unfinished(tx, id, item)? {
            return Ok(true);
        }
        if r.step != ClosureStep::Closing {
            record::begin_branch_closing(
                tx,
                id,
                r.operation_id,
                observation.base,
                BranchWrite::Withdraw { commit: built },
            )?;
        }
        Ok(false)
    })?;
    if held_off {
        return Err(E::busy());
    }
    boundary(service, root, FailurePoint::ClosureAfterClosingRecorded)?;
    let context_repo = context_repository(root, item)?;
    context_moving(service, root, item)?;
    move_context_to_checkpoint_with(&context_repo, item, context, built, || {
        boundary(service, root, FailurePoint::ClosureDuringCheckpointCheckout)
    })?;
    // The move marks the ticket's context alone: nothing reaches the primary.
    context_moving(service, root, item)?;
    boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
    Ok(BranchStep::Closing)
}

/// What a branch record answers after its publication child, by the table
/// of its stop and its result, in its order: the tracking primary as the
/// child left it (rows 1 and 2: whether the forge merged this closure or
/// another, whatever the child answered), a deleted remote branch or a
/// closed ticket (rows 3 and 4), a publication remote that changed or is
/// gone (row 5), then, for a child that completed, whether the closure it
/// published is the record's own, another one, or none (rows 6 to 9).
/// `child` is `None` when none ran, because the remote is not the one the
/// record was made for. `None` when the child's stop stands. This record's
/// closure is the one it wrote, or the one the context already held; the
/// tracking primary is the record's own remote's, at `primary`.
pub(in super::super) fn publication_answer(
    repo: &git2::Repository,
    r: &ClosureRecord,
    primary: &str,
    child: Option<&Result<SynchronizationOutcome, SynchronizationError>>,
) -> Result<Option<BranchAnswer>, RepositoryError> {
    let path = ticket_path(&r.item_id);
    let own_commit = match r.closure_kind {
        Some(ClosureKind::Existing) => r.context_oid,
        _ => r.written_commit(),
    };
    let own = own_commit.and_then(|commit| commit_closure_text(repo, commit, &path));
    // Rows 1 and 2.
    let tracking = r.remote_name.as_deref().and_then(|remote| {
        repo.refname_to_id(&format!("refs/remotes/{remote}/{primary}"))
            .ok()
    });
    if let Some(tracking) = tracking {
        match commit_closure_text(repo, tracking, &path) {
            Some(held) if Some(&held) == own.as_ref() => return Ok(Some(BranchAnswer::Merged)),
            Some(_) => return Ok(Some(BranchAnswer::ClosedElsewhere)),
            None if commit_names_closure(repo, tracking, &path) => {
                return Ok(Some(BranchAnswer::ClosedElsewhere));
            }
            None => (),
        }
    }
    let head = match child {
        // Row 5: the child could not run.
        None => return Ok(Some(BranchAnswer::RemoteChanged)),
        Some(Ok(SynchronizationOutcome::Published { oid, .. }))
        | Some(Ok(SynchronizationOutcome::AlreadyCurrent { oid, .. })) => *oid,
        Some(Err(SynchronizationError::RemoteContextDeleted)) => {
            return Ok(Some(BranchAnswer::RemoteBranchDeleted));
        }
        Some(Err(SynchronizationError::TicketClosed)) => {
            return Ok(Some(BranchAnswer::TicketClosed));
        }
        Some(Err(SynchronizationError::Transport(error)))
            if matches!(
                error.kind,
                SshTransportErrorKind::PublicationRemoteMissing
                    | SshTransportErrorKind::EndpointChanged
                    | SshTransportErrorKind::ConfigurationInvalid
            ) =>
        {
            return Ok(Some(BranchAnswer::RemoteChanged));
        }
        // Any other stop stands, and a result with no head to read is the
        // runner's to store: neither answers the record.
        Some(Err(_)) | Some(Ok(SynchronizationOutcome::PublishPending { .. })) => {
            return Ok(None);
        }
    };
    if r.closure_kind == Some(ClosureKind::Existing) {
        return Ok(Some(BranchAnswer::Existing));
    }
    Ok(Some(match commit_closure_text(repo, head, &path) {
        Some(published) if Some(&published) == own.as_ref() => BranchAnswer::Published,
        Some(_) => BranchAnswer::KeptPublished,
        None if commit_names_closure(repo, head, &path) => BranchAnswer::KeptPublished,
        None => BranchAnswer::WithdrawnSince,
    }))
}

/// What a withdraw answers after its publication child, by the table of its
/// stop and its result, in its order: the tracking primary as the child left
/// it (a closure there, the withdrawn one or another, is a closed ticket),
/// the remote that is not the record's (no child ran), a closed ticket or a
/// deleted remote branch the child stopped with, a publication remote that
/// changed or is gone, then, for a child that completed, whether the head it
/// published holds a closure again. `None` when the child's stop stands.
/// `child` is `None` when none ran, because the remote is not the one the
/// record was made for. The tracking primary is the record's own remote's,
/// at `primary`.
pub(in super::super) fn withdrawal_answer(
    repo: &git2::Repository,
    r: &ClosureRecord,
    primary: &str,
    child: Option<&Result<SynchronizationOutcome, SynchronizationError>>,
) -> Result<Option<BranchAnswer>, RepositoryError> {
    let path = ticket_path(&r.item_id);
    // Row 1.
    let tracking = r.remote_name.as_deref().and_then(|remote| {
        repo.refname_to_id(&format!("refs/remotes/{remote}/{primary}"))
            .ok()
    });
    if tracking.is_some_and(|tracking| commit_names_closure(repo, tracking, &path)) {
        return Ok(Some(BranchAnswer::WithdrawClosed));
    }
    let head = match child {
        // Row 2: the child could not run.
        None => return Ok(Some(BranchAnswer::RemoteChanged)),
        Some(Ok(SynchronizationOutcome::Published { oid, .. }))
        | Some(Ok(SynchronizationOutcome::AlreadyCurrent { oid, .. })) => *oid,
        // Row 3.
        Some(Err(SynchronizationError::TicketClosed)) => {
            return Ok(Some(BranchAnswer::WithdrawClosed));
        }
        // Row 4.
        Some(Err(SynchronizationError::RemoteContextDeleted)) => {
            return Ok(Some(BranchAnswer::RemoteBranchDeleted));
        }
        // Row 5.
        Some(Err(SynchronizationError::Transport(error)))
            if matches!(
                error.kind,
                SshTransportErrorKind::PublicationRemoteMissing
                    | SshTransportErrorKind::EndpointChanged
                    | SshTransportErrorKind::ConfigurationInvalid
            ) =>
        {
            return Ok(Some(BranchAnswer::RemoteChanged));
        }
        // Any other stop stands, and a result with no head to read is the
        // runner's to store: neither answers the record.
        Some(Err(_)) | Some(Ok(SynchronizationOutcome::PublishPending { .. })) => {
            return Ok(None);
        }
    };
    // Rows 6 and 7.
    Ok(Some(if commit_names_closure(repo, head, &path) {
        BranchAnswer::ClosedSince
    } else {
        BranchAnswer::Withdrawn
    }))
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
