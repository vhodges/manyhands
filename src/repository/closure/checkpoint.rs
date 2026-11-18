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
    PlannedClosure,
    observe::{ClosureObservation, closed_by_text, closed_ticket_bytes, ticket_path},
    record::{self, ClosureKind, ClosureRecord, ClosureStep},
};
use crate::{
    canonical::{self, CanonicalItem, ItemId},
    repository::{
        CommitIdentity, ConfirmedCommitIdentity, FailurePoint, PlannedIntegration, RepositoryError,
        RepositoryOperation, RepositoryService, SynchronizationTarget,
        mark_document_refresh_required,
        promotion::{
            integrate,
            lifecycle::{LeasedIntegration, LifecycleError, LifecycleKind, PreIntegration},
            observe as shared,
        },
        remote::{state, sync},
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

/// The closed ticket Blob, the checkpoint tree and commit, and the result
/// of merging it into the base: the one function preview and execution
/// both run (C10, final review I-1).
pub(in super::super) fn build_closure(
    repo: &git2::Repository,
    handle: MergeHandle,
    target: ClosureTarget<'_>,
    closed_at: OffsetDateTime,
    identity: &CommitIdentity,
) -> Result<ClosureBuild, RepositoryError> {
    use ClosureBuild::{Conflict, Invalid};
    let failed = |_| unavailable(repo);
    let closed_at = whole_utc_seconds(closed_at);
    let closed_by = closed_by_text(identity);
    let path = target.path;
    let context = repo.find_commit(target.context).map_err(failed)?;
    let context_tree = context.tree().map_err(failed)?;
    let Ok(entry) = context_tree.get_path(path) else {
        return Ok(Invalid);
    };
    let mode = match (entry.kind(), entry.filemode()) {
        (Some(git2::ObjectType::Blob), 0o100644) => git2::FileMode::Blob,
        (Some(git2::ObjectType::Blob), 0o100755) => git2::FileMode::BlobExecutable,
        _ => return Ok(Invalid),
    };
    let original = repo.find_blob(entry.id()).map_err(failed)?;
    let Ok(closed) = closed_ticket_bytes(original.content(), closed_at, &closed_by) else {
        return Ok(Invalid);
    };
    let ticket_blob = repo.blob(&closed).map_err(failed)?;
    let mut update = git2::build::TreeUpdateBuilder::new();
    update.upsert(path, ticket_blob, mode);
    let tree = update.create_updated(repo, &context_tree).map_err(failed)?;
    let tree = repo.find_tree(tree).map_err(failed)?;
    let when = git2::Time::new(closed_at.unix_timestamp(), 0);
    let Ok(signature) = git2::Signature::new(&identity.name, &identity.email, &when) else {
        return Ok(Invalid);
    };
    let checkpoint_commit = repo
        .commit(
            None,
            &signature,
            &signature,
            &checkpoint_subject(target.item),
            &tree,
            &[&context],
        )
        .map_err(failed)?;
    let Ok(result_tree) = post_checkpoint_result_on(repo, handle, target.base, checkpoint_commit)?
    else {
        return Ok(Conflict);
    };
    // The result must carry exactly this closure on a canonical entry.
    if tree_closure(repo, result_tree, path)
        != Some((closed_at, closed_by.clone(), entry.filemode()))
    {
        return Ok(Invalid);
    }
    Ok(ClosureBuild::Built(PreparedClosure {
        closed_at,
        ticket_blob,
        checkpoint_commit,
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

/// Whether a commit's ticket names either closure key, as the observation
/// reads the primary: a malformed closure is never open.
fn commit_names_closure(repo: &git2::Repository, commit: Oid, path: &Path) -> bool {
    let names = || -> Option<bool> {
        let tree = repo.find_commit(commit).ok()?.tree().ok()?;
        let entry = tree.get_path(path).ok()?;
        let blob = repo.find_blob(entry.id()).ok()?;
        let values = canonical::front_matter(std::str::from_utf8(blob.content()).ok()?)?;
        Some(values.get("closed_at").is_some() || values.get("closed_by").is_some())
    };
    names().unwrap_or(false)
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
    /// The ref is at `closure_oid`, or at a descendant whose ticket carries
    /// the same closure fields (a deliberate synchronization).
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
    let (Some(context), Some(closure)) = (record.context_oid, record.closure_oid) else {
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
    let closure_fields = commit_closure_text(context_repo, closure, &path);
    Ok(
        if descendant
            && closure_fields.is_some()
            && commit_closure_text(context_repo, tip, &path) == closure_fields
        {
            ClosingState::CheckpointWritten { tip }
        } else {
            ClosingState::Unrecognized
        },
    )
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

/// The context ref moved: the context now reads closed.
fn context_moved(service: &RepositoryService, root: &Path) -> Result<(), RepositoryError> {
    mark_document_refresh_required(
        &service.registry_path,
        root,
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
    move_context_to_checkpoint_with(
        &context_repo,
        &r.item_id,
        context,
        prepared.checkpoint_commit,
        || boundary(service, root, FailurePoint::ClosureDuringCheckpointCheckout),
    )?;
    context_moved(service, root)?;
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
                    // record is kept for a retry.
                    finish_context_move(&context_repo, r).map_err(|_| E::recovery_required())?;
                    context_moved(service, root)?;
                    boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
                    closure
                }
                ClosingState::CheckpointWritten { tip } => {
                    // The ref moved before this call (or after a synchronization):
                    // the mark is idempotent and may not have been made.
                    context_moved(service, root)?;
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
    let (Some(from), Some(to)) = (r.context_oid, r.closure_oid) else {
        return Err(state::recovery_required());
    };
    guarded_context_move(context_repo, &KIND.context_ref(&r.item_id), from, to)
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
