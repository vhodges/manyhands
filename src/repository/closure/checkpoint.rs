//! The closure checkpoint (C1, C6, C9, C10): the `Close ticket <ULID>`
//! commit written inside the integration lease, the context's move to it,
//! the reconciliation of a stopped `closing` step, and the post-checkpoint
//! resume that merges a written closure without re-consent.
//!
//! Nothing here acquires a lease: the integration holds it. Every object is
//! built in the repository ODB, unreferenced, before any ref moves; the one
//! ref this module moves is the context's, through the proven
//! `sync::fast_forward` order.
#![allow(dead_code)] // `TicketClosureLifecycle` (Task 6) calls the hook and resume; remove there.
use super::{
    PlannedClosure,
    observe::{ClosureObservation, closed_by_text, closed_ticket_bytes, ticket_path},
    record::{self, ClosureKind, ClosureRecord, ClosureStep},
};
use crate::{
    canonical::{self, CanonicalItem, ItemId},
    repository::{
        CommitIdentity, ConfirmedCommitIdentity, FailurePoint, PlannedIntegration, RepositoryError,
        RepositoryOperation, RepositoryService, SynchronizationError,
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
    let failed = |_| ClosureStop::Repository(unavailable(repo));
    let closed_at = whole_utc_seconds(closed_at);
    let closed_by = closed_by_text(identity);
    let path = &observation.ticket_path;
    let context = repo.find_commit(observation.context_oid).map_err(failed)?;
    let context_tree = context.tree().map_err(failed)?;
    let entry = context_tree
        .get_path(path)
        .map_err(|_| ClosureStop::Refused)?;
    let mode = match (entry.kind(), entry.filemode()) {
        (Some(git2::ObjectType::Blob), 0o100644) => git2::FileMode::Blob,
        (Some(git2::ObjectType::Blob), 0o100755) => git2::FileMode::BlobExecutable,
        _ => return Err(ClosureStop::Refused),
    };
    let original = repo.find_blob(entry.id()).map_err(failed)?;
    let closed = closed_ticket_bytes(original.content(), closed_at, &closed_by)
        .map_err(|_| ClosureStop::Refused)?;
    let ticket_blob = repo.blob(&closed).map_err(failed)?;
    let mut update = git2::build::TreeUpdateBuilder::new();
    update.upsert(path, ticket_blob, mode);
    let tree = update.create_updated(repo, &context_tree).map_err(failed)?;
    let tree = repo.find_tree(tree).map_err(failed)?;
    let when = git2::Time::new(closed_at.unix_timestamp(), 0);
    let signature = git2::Signature::new(&identity.name, &identity.email, &when)
        .map_err(|_| ClosureStop::Refused)?;
    let checkpoint_commit = repo
        .commit(
            None,
            &signature,
            &signature,
            &checkpoint_subject(&observation.item_id),
            &tree,
            &[&context],
        )
        .map_err(failed)?;
    let result_tree = match post_checkpoint_result(repo, observation.base, checkpoint_commit)
        .map_err(ClosureStop::Repository)?
    {
        Ok(tree) => tree,
        Err(()) => return Err(ClosureStop::Refused),
    };
    // The result must carry exactly this closure on a canonical entry.
    if tree_closure(repo, result_tree, path)
        != Some((closed_at, closed_by.clone(), entry.filemode()))
    {
        return Err(ClosureStop::Refused);
    }
    Ok(PreparedClosure {
        closed_at,
        ticket_blob,
        checkpoint_commit,
        result_tree,
    })
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

/// The tree a merge of `tip` into `base` has: the tip's own tree when
/// `base ≤ tip`, otherwise the clean in-memory merge, even with a remote
/// (C9). `Err(())` is a conflict. Only the merge's new objects are written.
pub(in super::super) fn post_checkpoint_result(
    repo: &git2::Repository,
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
    match sync::prepare_clean_merge(repo.path(), base, tip).map_err(|_| unavailable(repo))? {
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
/// the guarded ref update `from → to` (`sync::fast_forward` order).
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
    sync::fast_forward_with(context_repo, &KIND.context_ref(item), from, to, between).map_err(
        |error| match error {
            SynchronizationError::Repository(error) => error,
            SynchronizationError::ExternalChange => shared::changed(),
            _ => unavailable(context_repo),
        },
    )
}

/// What a `closing` record left in the context, read under the lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ClosingState {
    /// The ref is at `context_oid` and the worktree is clean.
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
        if statuses.is_empty() {
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
        let worktree = context_repo
            .workdir()
            .map(|dir| std::fs::read(dir.join(&path)));
        let expected = context_repo
            .find_blob(blob)
            .map_err(|_| unavailable(context_repo))?;
        let worktree_matches = matches!(worktree, Some(Ok(bytes)) if bytes == expected.content());
        let index_matches = context_repo
            .index()
            .ok()
            .and_then(|index| index.get_path(&path, 0))
            .is_some_and(|entry| {
                entry.id == blob
                    || !statuses
                        .iter()
                        .any(|e| e.status().contains(git2::Status::INDEX_MODIFIED))
            });
        return Ok(if worktree_matches && index_matches {
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
    pub closed_by: String,
    pub signature: CommitIdentity,
}

/// Read from the checkpoint commit `Close ticket <ULID>`, never from current
/// configuration. `closure` is the checkpoint or a descendant of it whose
/// first-parent chain reaches it (a deliberate synchronization keeps the
/// context side first).
pub(in super::super) fn checkpoint_authority(
    repo: &git2::Repository,
    closure: Oid,
) -> Result<CheckpointAuthority, RepositoryError> {
    let failed = || unavailable(repo);
    let mut current = closure;
    // A bounded first-parent walk: synchronizations add a few commits.
    for _ in 0..4096 {
        let commit = repo.find_commit(current).map_err(|_| failed())?;
        if let Some(item) = commit
            .summary()
            .and_then(|s| s.strip_prefix("Close ticket "))
            .and_then(|s| s.parse::<ItemId>().ok())
            && commit.parent_count() == 1
        {
            let path = ticket_path(&item);
            let parent = commit.parent_id(0).map_err(|_| failed())?;
            let (Some((_, closed_by)), None) = (
                commit_closure_text(repo, current, &path),
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
            return Ok(CheckpointAuthority {
                closed_by,
                signature,
            });
        }
        if commit.parent_count() == 0 {
            break;
        }
        current = commit.parent_id(0).map_err(|_| failed())?;
    }
    Err(failed())
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
    // Step 4: persist a confirmed identity before any commit (T3-1, Q3-1).
    let identity = integrate::promotion_identity::<E>(service, repo, root, confirmation)?;
    if closed_by_text(&identity) != *closed_by {
        return Err(E::reconfirmation_required());
    }
    // Step 5: the stored `closed_at` on a rebuild, so the same commit results.
    let closed_at = r.closed_at.unwrap_or_else(now_whole_utc_seconds);
    let prepared = match prepare_closure(repo, observation, closed_at, &identity) {
        Ok(prepared) => prepared,
        Err(ClosureStop::Refused) => return Err(E::reconfirmation_required()),
        Err(ClosureStop::Repository(error)) => return Err(error.into()),
    };
    let operation = r.operation_id;
    state::with_transaction(service, root, |tx, id| {
        if !closing {
            record::begin_closing(
                tx,
                id,
                operation,
                base,
                prepared.closed_at,
                prepared.checkpoint_commit,
            )
        } else if r.closure_oid != Some(prepared.checkpoint_commit) {
            record::replace_unmoved_closure(tx, id, operation, prepared.checkpoint_commit)
        } else {
            Ok(())
        }
    })?;
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
                    finish_context_move(&context_repo, r)?;
                    context_moved(service, root)?;
                    boundary(service, root, FailurePoint::AfterClosureCheckpoint)?;
                    closure
                }
                ClosingState::CheckpointWritten { tip } => tip,
                ClosingState::Unrecognized => return Err(E::recovery_required()),
            };
            (
                tip,
                current_base::<E>(repo, r, &stage.context.primary_branch)?,
            )
        }
        ClosureStep::Integrating => (closure, r.base_oid.ok_or_else(E::recovery_required)?),
        _ => return Ok(None),
    };
    let authority = checkpoint_authority(repo, tip).map_err(|_| E::recovery_required())?;
    let Ok(result_tree) = post_checkpoint_result(repo, base, tip)? else {
        return Err(E::recovery_required());
    };
    if r.step == ClosureStep::Closing && (r.base_oid != Some(base) || tip != closure) {
        let operation = r.operation_id;
        state::with_transaction(service, root, |tx, id| {
            record::adopt_written_closure(tx, id, operation, base, tip)
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
    let branch = KIND.context_ref(&r.item_id);
    let failed = |_| unavailable(context_repo);
    let mut transaction = context_repo.transaction().map_err(failed)?;
    transaction.lock_ref(&branch).map_err(failed)?;
    if context_repo.refname_to_id(&branch).ok() != Some(from) {
        return Err(shared::changed());
    }
    transaction
        .set_target(&branch, to, None, "manyhands closure checkpoint")
        .map_err(failed)?;
    transaction.commit().map_err(failed)
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
