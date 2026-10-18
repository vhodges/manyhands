// Preserve the synchronization seam's existing typed errors, as sync.rs does.
#![allow(clippy::result_large_err)]
use rusqlite::params;
use std::path::Path;
use time::OffsetDateTime;

use super::{RemoteOutcomeCategory, RemotePollInvocation, RemotePollingConfiguration, state};
use super::{RemoteRefPlan, SynchronizationError, SynchronizationTarget, refs, sync};
use crate::repository::{RepositoryError, RepositoryService};
use crate::{
    canonical,
    repository::{
        AuthoringKind, ConfigurationInspection, RepositoryOperation, read_configuration,
        repository_lease,
    },
};

// Task 4 internal seams are consumed by the Task 6 orchestrator.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollDisposition {
    Current,
    FastForwarded,
    LocalAhead,
    Diverged,
    Dirty,
    Conflicted,
    Inaccessible,
    Materialized,
    Nonconforming,
    BlockedByLocalState,
    MaterializationIncomplete,
    Malformed,
    RemotelyDeleted,
    HistoryUnknown,
    NotReached,
}
#[allow(dead_code)]
impl PollDisposition {
    pub fn guidance(self) -> &'static str {
        match self {
            Self::Current => "Local state is current.",
            Self::FastForwarded => "Clean local state was fast-forwarded.",
            Self::LocalAhead => "Publish local commits with deliberate synchronization.",
            Self::Diverged => "Resolve divergent history with deliberate synchronization.",
            Self::Dirty => "Save or otherwise handle local changes before polling again.",
            Self::Conflicted => {
                "Resolve the conflict or in-progress Git operation before polling again."
            }
            Self::Inaccessible => {
                "Restore the expected checked-out branch and registered worktree before polling again."
            }
            Self::Materialized => "The remote context was materialized locally.",
            Self::Nonconforming => {
                "Repair the remote context's canonical tree before materialization."
            }
            Self::BlockedByLocalState => {
                "Inspect the existing local branch or path before materialization."
            }
            Self::MaterializationIncomplete => {
                "Inspect and repair the retained partial materialization."
            }
            Self::Malformed => "Repair the remote context's branch name.",
            Self::RemotelyDeleted => "The remote context is absent; local state was preserved.",
            Self::HistoryUnknown => {
                "Publication history is unknown; inspect remote state before proceeding."
            }
            Self::NotReached => "This target was not reached; retry the poll when ready.",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PollRelation {
    Current,
    Behind,
    LocalAhead,
    Diverged,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Absent-ref states are supplied by the Task 6 batch orchestrator.
pub(super) enum PollTrackingState {
    Present(git2::Oid),
    RemotelyDeleted,
    HistoryUnknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LocalPollClassification {
    Unchanged {
        disposition: PollDisposition,
        local_oid: Option<git2::Oid>,
    },
    FastForward {
        local_oid: git2::Oid,
        tracking_oid: git2::Oid,
    },
}
pub(super) struct PollFastForward {
    pub target: super::SynchronizationTarget,
    pub before: git2::Oid,
    pub after: git2::Oid,
}
#[allow(dead_code)]
pub(super) fn classify_relation<E>(
    local: git2::Oid,
    remote: git2::Oid,
    mut is_ancestor: impl FnMut(git2::Oid, git2::Oid) -> Result<bool, E>,
) -> Result<PollRelation, E> {
    match refs::clean_descendant(local, remote, &mut is_ancestor) {
        Ok(_) if local == remote => Ok(PollRelation::Current),
        Ok(oid) if oid == remote => Ok(PollRelation::Behind),
        Ok(_) => Ok(PollRelation::LocalAhead),
        Err(refs::CleanIntegrationError::MergeRequired) => Ok(PollRelation::Diverged),
        Err(refs::CleanIntegrationError::Ancestry(error)) => Err(error),
        // clean_descendant only returns MergeRequired or Ancestry errors.
        Err(_) => unreachable!("clean descendant error contract"),
    }
}

/// Poll-local ownership proof. Location and branch spelling alone cannot bind
/// an opened repository to the root whose lease the poll owns.
fn poll_materialized_target(
    root: &Path,
    primary: &str,
    target: &SynchronizationTarget,
) -> Result<git2::Repository, SynchronizationError> {
    let invalid = || SynchronizationError::TargetNotMaterialized;
    let linked = sync::materialized_target(root, primary, target)?;
    let expected_path = match target {
        SynchronizationTarget::Primary => root.to_owned(),
        SynchronizationTarget::Context { item_id, .. } => {
            root.join(".manyhands/worktrees").join(item_id.to_string())
        }
    }
    .canonicalize()
    .map_err(|_| invalid())?;
    if linked
        .workdir()
        .and_then(|p| p.canonicalize().ok())
        .as_ref()
        != Some(&expected_path)
    {
        return Err(invalid());
    }
    if let SynchronizationTarget::Context { item_id, .. } = target {
        let owner = git2::Repository::open(root).map_err(|_| invalid())?;
        let common = owner.commondir().canonicalize().map_err(|_| invalid())?;
        if !linked.is_worktree() || linked.commondir().canonicalize().ok().as_ref() != Some(&common)
        {
            return Err(invalid());
        }
        let name = item_id.to_string();
        let registered = owner.find_worktree(&name).map_err(|_| invalid())?;
        if registered.name() != Some(name.as_str())
            || registered.path().canonicalize().ok().as_ref() != Some(&expected_path)
        {
            return Err(invalid());
        }
        let worktrees = common.join("worktrees");
        let admin = worktrees.join(&name);
        for path in [&worktrees, &admin] {
            let metadata = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(invalid());
            }
        }
        let admin = admin.canonicalize().map_err(|_| invalid())?;
        if linked.path().canonicalize().ok().as_ref() != Some(&admin) {
            return Err(invalid());
        }
        let gitfile = expected_path.join(".git");
        let backlink_file = admin.join("gitdir");
        for path in [&gitfile, &backlink_file] {
            let metadata = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(invalid());
            }
        }
        // libgit2's worktree link reader resolves relative links against the
        // admin directory. Validate the exact .git file, not just its parent.
        let backlink = std::fs::read_to_string(&backlink_file).map_err(|_| invalid())?;
        let backlink = Path::new(backlink.trim_end());
        if backlink.as_os_str().is_empty() {
            return Err(invalid());
        }
        let backlink = if backlink.is_absolute() {
            backlink.to_owned()
        } else {
            admin.join(backlink)
        };
        if backlink.canonicalize().map_err(|_| invalid())?
            != gitfile.canonicalize().map_err(|_| invalid())?
        {
            return Err(invalid());
        }
    }
    Ok(linked)
}

#[allow(dead_code)]
pub(super) fn classify_local_target(
    root: &Path,
    primary: &str,
    target: &SynchronizationTarget,
    tracking: PollTrackingState,
) -> Result<LocalPollClassification, SynchronizationError> {
    let unchanged = |disposition, local_oid| LocalPollClassification::Unchanged {
        disposition,
        local_oid,
    };
    let incoming = match tracking {
        PollTrackingState::Present(oid) => oid,
        PollTrackingState::RemotelyDeleted => {
            return Ok(unchanged(PollDisposition::RemotelyDeleted, None));
        }
        PollTrackingState::HistoryUnknown => {
            return Ok(unchanged(PollDisposition::HistoryUnknown, None));
        }
    };
    let Ok(linked) = poll_materialized_target(root, primary, target) else {
        return Ok(unchanged(PollDisposition::Inaccessible, None));
    };
    let Ok(local) = sync::local_oid(&linked) else {
        return Ok(unchanged(PollDisposition::Inaccessible, None));
    };
    if let Err(error) = sync::require_clean_target(&linked, target) {
        let disposition = match error {
            SynchronizationError::WorktreeConflicted { .. } => PollDisposition::Conflicted,
            SynchronizationError::WorktreeNotClean { .. } => PollDisposition::Dirty,
            _ => PollDisposition::Inaccessible,
        };
        return Ok(unchanged(disposition, Some(local)));
    }
    let relation = classify_relation(local, incoming, |a, b| linked.graph_descendant_of(b, a))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    Ok(match relation {
        PollRelation::Behind => LocalPollClassification::FastForward {
            local_oid: local,
            tracking_oid: incoming,
        },
        PollRelation::Current => unchanged(PollDisposition::Current, Some(local)),
        PollRelation::LocalAhead => unchanged(PollDisposition::LocalAhead, Some(local)),
        PollRelation::Diverged => unchanged(PollDisposition::Diverged, Some(local)),
    })
}
impl RepositoryService {
    #[allow(dead_code)]
    pub(super) fn materialize_context(
        &self,
        root: &Path,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
        request: &PollMaterializationRequest,
    ) -> Result<PollMaterializationOutcome, SynchronizationError> {
        let repository =
            git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        self.materialize_context_unlocked(root, plan, configuration, request, &repository)
    }
    fn materialize_context_unlocked(
        &self,
        root: &Path,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
        request: &PollMaterializationRequest,
        repository: &git2::Repository,
    ) -> Result<PollMaterializationOutcome, SynchronizationError> {
        use crate::repository::{
            FailurePoint, ItemContext, branch_checked_out_in_another_worktree,
            ensure_authoring_worktree_base, validate_context_worktree,
            validate_deterministic_worktree_path_chain,
        };
        use state::{MaterializationIntent, MaterializationStep};
        let operation = RepositoryOperation::RepositorySnapshot;
        let target = SynchronizationTarget::Context {
            kind: request.kind,
            item_id: request.item_id.clone(),
        };
        let reference = plan.context(request.kind, &request.item_id);
        let context = ItemContext {
            root: root.to_owned(),
            kind: request.kind,
            item_id: request.item_id.clone(),
            branch: reference
                .remote_ref()
                .trim_start_matches("refs/heads/")
                .to_owned(),
            worktree: root
                .join(".manyhands/worktrees")
                .join(request.item_id.to_string()),
        };
        let refused = PollMaterializationOutcome::Refused;

        // No canonical file collection here: this is the unchanged path-chain
        // validator only. Refuse redirected parents before reading configuration.
        if validate_deterministic_worktree_path_chain(&context.worktree, root, operation).is_err() {
            let owned = state::with_transaction(self, root, |tx, id| {
                state::materialization_intents(tx, id, &request.item_id)
            })?;
            return Ok(refused(if owned.iter().any(|i| i.kind == request.kind) {
                PollDisposition::MaterializationIncomplete
            } else {
                PollDisposition::BlockedByLocalState
            }));
        }
        let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
            return Err(SynchronizationError::ExternalChange);
        };
        let actual_plan = config
            .publication_remote
            .as_ref()
            .and_then(|r| RemoteRefPlan::from_configuration(r, &config.primary_branch).ok());
        if actual_plan.as_ref() != Some(plan)
            || self.observation_configuration(root, plan)? != *configuration
            || repository.refname_to_id(reference.tracking_ref()).ok() != Some(request.oid)
        {
            return Err(SynchronizationError::ExternalChange);
        }

        // Observe Git/filesystem presence first, then consult durable ownership.
        let observed =
            MaterializationGitState::observe(repository, &context, &target, plan.primary_branch());
        let intents = state::with_transaction(self, root, |tx, id| {
            state::materialization_intents(tx, id, &request.item_id)
        })?;
        if intents.iter().any(|i| i.kind != request.kind) {
            return Ok(refused(PollDisposition::BlockedByLocalState));
        }
        let mut intent = intents.into_iter().next();
        let action = reconcile_materialization(&observed, intent.as_ref());
        match action {
            MaterializationAction::Refused(disposition) => return Ok(refused(disposition)),
            MaterializationAction::Present => {
                let retained = intent
                    .as_ref()
                    .ok_or(SynchronizationError::RecoveryRequired)?;
                if retained.step != MaterializationStep::Complete {
                    // Resuming step 5 only: owner's narrow scan exception.
                    if validate_context_worktree(repository, &context, true, operation).is_err() {
                        return Ok(refused(PollDisposition::MaterializationIncomplete));
                    }
                    state::with_transaction(self, root, |tx, id| {
                        state::advance_materialization(
                            tx,
                            id,
                            retained,
                            MaterializationStep::Complete,
                        )
                    })?;
                }
                return Ok(PollMaterializationOutcome::Present);
            }
            MaterializationAction::Start | MaterializationAction::ResumeWorktree => {}
        }
        if branch_checked_out_in_another_worktree(
            repository,
            &context.worktree,
            &context.branch,
            operation,
            root,
        )
        .unwrap_or(true)
        {
            return Ok(refused(if intent.is_some() {
                PollDisposition::MaterializationIncomplete
            } else {
                PollDisposition::BlockedByLocalState
            }));
        }
        let target_oid = intent.as_ref().map_or(request.oid, |i| i.oid);
        let commit = repository
            .find_commit(target_oid)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if !conforming_context_tree(
            repository,
            &commit
                .tree()
                .map_err(|_| SynchronizationError::RecoveryRequired)?,
            request.kind,
            &request.item_id,
        )? {
            return Ok(refused(PollDisposition::Nonconforming));
        }
        if intent.is_none() {
            state::with_transaction(self, root, |tx, id| {
                state::insert_materialization(
                    tx,
                    id,
                    request.kind,
                    &request.item_id,
                    request.operation_id,
                    target_oid,
                )
            })?;
            intent = Some(MaterializationIntent {
                kind: request.kind,
                item_id: request.item_id.clone(),
                operation_id: request.operation_id,
                oid: target_oid,
                step: MaterializationStep::Intended,
            });
        } else if action == MaterializationAction::Start {
            // Git-first all-absent restart. Keep the original context, owner and
            // target; do not synthesize adoption evidence from a cached step.
            let retained = intent
                .as_ref()
                .ok_or(SynchronizationError::RecoveryRequired)?;
            state::with_transaction(self, root, |tx, id| {
                state::advance_materialization(tx, id, retained, MaterializationStep::Intended)
            })?;
        }
        let intent = intent
            .as_ref()
            .ok_or(SynchronizationError::RecoveryRequired)?;
        self.check_failure(
            FailurePoint::PollAfterMaterializationIntent,
            operation,
            root,
        )?;
        // Parent creation is part of the guarded local provisioning step, after
        // refusal/conformity/intent. No pre-existing deterministic entry is used.
        ensure_authoring_worktree_base(root, operation)?;
        let branch = if action == MaterializationAction::Start {
            if poll_entry_present(&context.worktree)
                || poll_registration_present(repository, &context.item_id.to_string())
                || repository.find_reference(reference.remote_ref()).is_ok()
            {
                return Ok(refused(PollDisposition::MaterializationIncomplete));
            }
            // libgit2's non-force branch() checks absence before acquiring the
            // ref lock. External Git does not share our repository lease, so
            // lock first and prove absence while that exact lock is held.
            let mut ref_tx = repository
                .transaction()
                .map_err(|_| SynchronizationError::ExternalChange)?;
            if ref_tx.lock_ref(reference.remote_ref()).is_err() {
                return Ok(refused(PollDisposition::MaterializationIncomplete));
            }
            match repository.find_reference(reference.remote_ref()) {
                Err(error) if error.code() == git2::ErrorCode::NotFound => {}
                _ => return Ok(refused(PollDisposition::MaterializationIncomplete)),
            }
            // This is creation of a proven-absent ref, never a forced update.
            ref_tx
                .set_target(
                    reference.remote_ref(),
                    target_oid,
                    None,
                    "poll context creation",
                )
                .and_then(|_| ref_tx.commit())
                .map_err(|_| SynchronizationError::ExternalChange)?;
            let branch = repository
                .find_reference(reference.remote_ref())
                .map_err(|_| SynchronizationError::ExternalChange)?;
            state::with_transaction(self, root, |tx, id| {
                state::advance_materialization(tx, id, intent, MaterializationStep::BranchCreated)
            })?;
            branch
        } else {
            repository
                .find_reference(reference.remote_ref())
                .map_err(|_| SynchronizationError::ExternalChange)?
        };
        self.check_failure(
            FailurePoint::PollAfterMaterializationBranch,
            operation,
            root,
        )?;
        if branch.target() != Some(target_oid)
            || repository.refname_to_id(reference.remote_ref()).ok() != Some(target_oid)
            || repository.refname_to_id(reference.tracking_ref()).ok() != Some(request.oid)
            || poll_entry_present(&context.worktree)
            || poll_registration_present(repository, &context.item_id.to_string())
        {
            return Ok(refused(PollDisposition::MaterializationIncomplete));
        }
        validate_deterministic_worktree_path_chain(&context.worktree, root, operation)?;
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&branch)); // exactly local authoring's options
        if repository
            .worktree(
                &context.item_id.to_string(),
                &context.worktree,
                Some(&options),
            )
            .is_err()
        {
            self.check_failure(
                FailurePoint::PollAfterFailedMaterializationWorktree,
                operation,
                root,
            )?;
            return Ok(refused(PollDisposition::MaterializationIncomplete));
        }
        self.check_failure(
            FailurePoint::PollAfterMaterializationWorktree,
            operation,
            root,
        )?;
        // Step 5: canonical collection is the sole approved under-lease scan.
        // Retain Task 4's stronger ownership proof in addition to the shared check.
        let valid = poll_materialized_target(root, plan.primary_branch(), &target)
            .ok()
            .is_some_and(|linked| {
                sync::local_oid(&linked).ok() == Some(target_oid)
                    && sync::require_clean_target(&linked, &target).is_ok()
            });
        if !valid || validate_context_worktree(repository, &context, true, operation).is_err() {
            return Ok(refused(PollDisposition::MaterializationIncomplete));
        }
        state::with_transaction(self, root, |tx, id| {
            state::advance_materialization(tx, id, intent, MaterializationStep::Complete)
        })?;
        Ok(PollMaterializationOutcome::Materialized)
    }

    #[allow(dead_code)]
    pub(super) fn poll_fast_forward(
        &self,
        root: &Path,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
        update: &PollFastForward,
    ) -> Result<PollDisposition, SynchronizationError> {
        let repository =
            git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        self.poll_fast_forward_unlocked(root, plan, configuration, update, &repository)
    }
    fn poll_fast_forward_unlocked(
        &self,
        root: &Path,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
        update: &PollFastForward,
        repository: &git2::Repository,
    ) -> Result<PollDisposition, SynchronizationError> {
        let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
            return Err(SynchronizationError::ExternalChange);
        };
        let actual_plan = config.publication_remote.as_ref().and_then(|remote| {
            RemoteRefPlan::from_configuration(remote, &config.primary_branch).ok()
        });
        if actual_plan.as_ref() != Some(plan)
            || self.observation_configuration(root, plan)? != *configuration
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let target_ref = match &update.target {
            SynchronizationTarget::Primary => plan.primary().clone(),
            SynchronizationTarget::Context { kind, item_id } => plan.context(*kind, item_id),
        };
        let branch = target_ref.remote_ref();
        if repository.refname_to_id(target_ref.tracking_ref()).ok() != Some(update.after)
            || repository.refname_to_id(branch).ok() != Some(update.before)
        {
            return Err(SynchronizationError::ExternalChange);
        }
        match classify_local_target(
            root,
            plan.primary_branch(),
            &update.target,
            PollTrackingState::Present(update.after),
        )? {
            LocalPollClassification::Unchanged { disposition, .. } => return Ok(disposition),
            LocalPollClassification::FastForward { local_oid, .. }
                if local_oid != update.before =>
            {
                return Err(SynchronizationError::ExternalChange);
            }
            LocalPollClassification::FastForward { .. } => {}
        }
        // R-T6-4: from `closing` until retirement the closure reconciles a
        // ticket context's ref against its own record; a poll never moves
        // it. Tracking is already updated, so the closure's re-check sees
        // the remote change. The target is left as not reached.
        if let SynchronizationTarget::Context {
            kind: crate::repository::AuthoringKind::Ticket,
            item_id,
        } = &update.target
            && state::with_transaction(self, root, |tx, id| {
                crate::repository::closure::record::written_closure_unfinished(tx, id, item_id)
            })?
        {
            return Ok(PollDisposition::NotReached);
        }
        let Ok(linked) = poll_materialized_target(root, plan.primary_branch(), &update.target)
        else {
            return Ok(PollDisposition::Inaccessible);
        };
        let commit = linked
            .find_commit(update.after)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        // Checkout-NONE (git2 dry_run) bypasses libgit2's ignored-collision
        // classification. Inspect incoming paths before calling the unchanged
        // safe checkout; it remains the final guarded authority for effects.
        let old_tree = linked
            .find_commit(update.before)
            .and_then(|c| c.tree())
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let new_tree = commit
            .tree()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let diff = linked
            .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let workdir = linked
            .workdir()
            .ok_or(SynchronizationError::TargetNotMaterialized)?;
        for delta in diff.deltas() {
            if delta.status() == git2::Delta::Deleted {
                continue;
            }
            let path = delta
                .new_file()
                .path()
                .ok_or(SynchronizationError::RecoveryRequired)?;
            let ancestors = path
                .ancestors()
                .filter(|p| !p.as_os_str().is_empty())
                .collect::<Vec<_>>();
            for ancestor in ancestors.into_iter().rev() {
                if let Ok(entry) = old_tree.get_path(ancestor) {
                    if entry.kind() == Some(git2::ObjectType::Tree) {
                        let local = workdir.join(ancestor);
                        match std::fs::symlink_metadata(&local) {
                            Ok(metadata)
                                if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
                            Ok(_) => return Ok(PollDisposition::Dirty),
                            Err(_) => return Ok(PollDisposition::Inaccessible),
                        }
                        if ancestor == path
                            && new_tree.get_path(path).ok().and_then(|e| e.kind())
                                == Some(git2::ObjectType::Blob)
                        {
                            // An old tracked directory is not automatically
                            // disposable: ignored descendants can obstruct its
                            // replacement after checkout has changed other paths.
                            match poll_retiring_directory_obstructed(&linked, &local, entry.id()) {
                                Ok(true) => return Ok(PollDisposition::Dirty),
                                Ok(false) => {}
                                Err(_) => return Ok(PollDisposition::Inaccessible),
                            }
                        }
                    }
                    // A clean tracked file/symlink becoming a directory is an
                    // authorized checkout transition. Do not inspect children
                    // through it; they do not exist locally before checkout.
                    if entry.kind() != Some(git2::ObjectType::Tree) && ancestor != path {
                        break;
                    }
                    continue;
                }
                match std::fs::symlink_metadata(workdir.join(ancestor)) {
                    Ok(metadata)
                        if ancestor == path
                            || !metadata.is_dir()
                            || metadata.file_type().is_symlink() =>
                    {
                        return Ok(PollDisposition::Dirty);
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => return Ok(PollDisposition::Inaccessible),
                }
            }
        }
        let Ok(fresh) = poll_materialized_target(root, plan.primary_branch(), &update.target)
        else {
            return Ok(PollDisposition::Inaccessible);
        };
        if sync::local_oid(&fresh)? != update.before
            || repository.refname_to_id(target_ref.tracking_ref()).ok() != Some(update.after)
        {
            return Err(SynchronizationError::ExternalChange);
        }
        sync::fast_forward(&fresh, branch, update.before, update.after)?;
        if sync::local_oid(&fresh)? != update.after {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(PollDisposition::FastForwarded)
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub(super) struct PollMaterializationRequest {
    pub operation_id: crate::repository::OperationId,
    pub kind: AuthoringKind,
    pub item_id: canonical::ItemId,
    pub oid: git2::Oid,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PollMaterializationOutcome {
    Materialized,
    // The caller classifies an existing context against its own tracking OID.
    Present,
    Refused(PollDisposition),
}

fn poll_entry_present(path: &Path) -> bool {
    !matches!(std::fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
}

fn poll_registration_present(repository: &git2::Repository, name: &str) -> bool {
    // The admin directory counts even when libgit2 cannot parse a registration.
    let base = repository.commondir().join("worktrees");
    let redirected = match std::fs::symlink_metadata(&base) {
        Ok(m) => !m.is_dir() || m.file_type().is_symlink(),
        Err(e) => e.kind() != std::io::ErrorKind::NotFound,
    };
    redirected
        || poll_entry_present(&base.join(name))
        || repository
            .worktrees()
            .map_or(true, |names| names.iter().any(|n| n == Some(name)))
}

struct MaterializationGitState {
    branch: Option<git2::Oid>,
    branch_present: bool,
    path_present: bool,
    registration_present: bool,
    valid: bool,
}
impl MaterializationGitState {
    fn observe(
        repository: &git2::Repository,
        context: &crate::repository::ItemContext,
        target: &SynchronizationTarget,
        primary: &str,
    ) -> Self {
        let branch_ref = format!("refs/heads/{}", context.branch);
        let branch = repository
            .find_reference(&branch_ref)
            .ok()
            .and_then(|r| r.target());
        let branch_present = !matches!(repository.find_reference(&branch_ref), Err(e) if e.code() == git2::ErrorCode::NotFound);
        let path_present = poll_entry_present(&context.worktree);
        let registration_present =
            poll_registration_present(repository, &context.item_id.to_string());
        let linked = if branch_present && path_present && registration_present {
            poll_materialized_target(&context.root, primary, target).ok()
        } else {
            None
        };
        let valid = branch.is_some()
            && linked.as_ref().is_some_and(|linked| {
                std::fs::symlink_metadata(linked.path().join("index"))
                    .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
            });
        Self {
            branch,
            branch_present,
            path_present,
            registration_present,
            valid,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MaterializationAction {
    Start,
    ResumeWorktree,
    Present,
    Refused(PollDisposition),
}

/// No effects and no canonical scan. Git presence is authoritative; a cache
/// step alone cannot authorize adoption or cleanup of a partial worktree.
fn reconcile_materialization(
    observed: &MaterializationGitState,
    intent: Option<&state::MaterializationIntent>,
) -> MaterializationAction {
    use state::MaterializationStep;
    let none = !observed.branch_present && !observed.path_present && !observed.registration_present;
    let Some(intent) = intent else {
        return if none {
            MaterializationAction::Start
        } else {
            MaterializationAction::Refused(PollDisposition::BlockedByLocalState)
        };
    };
    // Fully materialized ownership is independent of later local edits or
    // commits. Step 5's canonical validator still gates unfinished completion;
    // the caller then classifies local work normally without changing Git.
    if observed.valid {
        return MaterializationAction::Present;
    }
    if none {
        return MaterializationAction::Start;
    }
    if intent.step == MaterializationStep::BranchCreated
        && observed.branch == Some(intent.oid)
        && !observed.path_present
        && !observed.registration_present
    {
        return MaterializationAction::ResumeWorktree;
    }
    MaterializationAction::Refused(PollDisposition::MaterializationIncomplete)
}

/// Inspect only directories a pending file/symlink replacement must retire.
/// Never follow local symlinks or read file bodies; every on-disk entry must be
/// accounted for by the old tracked tree. Extra entries (even empty ignored
/// directories) are local obstructions, and unreadable state is never clean.
fn poll_retiring_directory_obstructed(
    repository: &git2::Repository,
    directory: &Path,
    tree_oid: git2::Oid,
) -> Result<bool, SynchronizationError> {
    let mut pending = vec![(directory.to_owned(), tree_oid)];
    while let Some((path, oid)) = pending.pop() {
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| SynchronizationError::RecoveryRequired)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Ok(true);
        }
        let tree = repository
            .find_tree(oid)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        for child in std::fs::read_dir(&path).map_err(|_| SynchronizationError::RecoveryRequired)? {
            let child = child.map_err(|_| SynchronizationError::RecoveryRequired)?;
            let name = child.file_name();
            let Some(entry) = tree.get_name_bytes(name.as_encoded_bytes()) else {
                return Ok(true);
            };
            let child_path = child.path();
            let metadata = std::fs::symlink_metadata(&child_path)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            match (entry.kind(), entry.filemode()) {
                (Some(git2::ObjectType::Tree), _)
                    if metadata.is_dir() && !metadata.file_type().is_symlink() =>
                {
                    pending.push((child_path, entry.id()))
                }
                (Some(git2::ObjectType::Blob), 0o120000) if metadata.file_type().is_symlink() => {}
                (Some(git2::ObjectType::Blob), 0o100644 | 0o100755)
                    if metadata.is_file() && !metadata.file_type().is_symlink() => {}
                _ => return Ok(true),
            }
        }
    }
    Ok(false)
}

#[allow(dead_code)]
pub(super) fn conforming_context_tree(
    repository: &git2::Repository,
    tree: &git2::Tree<'_>,
    kind: AuthoringKind,
    id: &canonical::ItemId,
) -> Result<bool, SynchronizationError> {
    let mut forbidden = false;
    let mut document_entries = 0;
    let mut document_overflow = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |parent, entry| {
        let mut path = parent.as_bytes().to_vec();
        path.extend_from_slice(entry.name_bytes());
        if path == b".manyhands/worktrees"
            || path.starts_with(b".manyhands/worktrees/")
            || entry.kind() == Some(git2::ObjectType::Commit)
        {
            forbidden = true;
        }
        if path.starts_with(b"docs/") {
            document_entries += 1;
            // Discovery counts all directory entries and starts depth at docs=0.
            let depth = path.iter().filter(|b| **b == b'/').count();
            if document_entries > crate::repository::MAX_DOCUMENT_DIRECTORY_ENTRIES
                || depth > crate::repository::MAX_DOCUMENT_DIRECTORY_DEPTH + 1
            {
                document_overflow = true;
            }
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if forbidden || (kind == AuthoringKind::Document && document_overflow) {
        return Ok(false);
    }
    let snapshot = RepositoryService::canonical_tree_sources(repository, tree)?;
    let ticket_path = format!(".manyhands/tickets/{id}/ticket.md");
    let comments = format!(".manyhands/comments/{id}/");
    let mut sources = Vec::new();
    for (path, bytes) in &snapshot.sources {
        let Ok(path_text) = std::str::from_utf8(path) else {
            continue;
        };
        let is_ticket = kind == AuthoringKind::Ticket && path_text == ticket_path;
        let is_document = kind == AuthoringKind::Document
            && path_text.starts_with("docs/")
            && Path::new(path_text)
                .extension()
                .is_some_and(|extension| extension == "md");
        let is_comment = path_text.starts_with(&comments);
        if !is_ticket && !is_document && !is_comment {
            continue;
        }
        if Path::new(path_text)
            .components()
            .any(|part| part.as_os_str() == ".git")
        {
            continue;
        }
        let Ok(source) = std::str::from_utf8(bytes) else {
            continue;
        };
        let parsed = canonical::parse_item(Path::new(path_text), source);
        if is_document
            && !parsed.as_ref().is_ok_and(|item| {
                crate::repository::discovery::authoring_item_matches(item, kind, id)
            })
        {
            continue;
        }
        let regular = matches!(
            snapshot.entries.get(path),
            Some((_, 0o100644, Some(git2::ObjectType::Blob)))
        );
        if !regular {
            if is_ticket || is_document {
                return Ok(false);
            }
            // A mode-invalid comment does not itself reject the item. A parsed
            // duplicate of the item ID would invalidate it in discovery too.
            if parsed.is_ok_and(|item| matches!(item, canonical::CanonicalItem::Comment(comment) if comment.id == *id)) { return Ok(false); }
            continue;
        }
        sources.push((std::path::PathBuf::from(path_text), source.to_owned()));
    }
    let validation = canonical::validate_context(sources);
    Ok(validation
        .items
        .iter()
        .any(|item| crate::repository::discovery::authoring_item_matches(item, kind, id)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollIneligibility {
    Disabled,
    Paused,
    RecoverySuspended,
    NotDue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomaticPollEligibility {
    Eligible,
    NotEligible {
        reason: PollIneligibility,
        next_eligible_at: Option<OffsetDateTime>,
    },
}

pub(super) fn eligibility(
    policy: &RemotePollingConfiguration,
    next: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> AutomaticPollEligibility {
    let reason = if !policy.enabled() {
        Some(PollIneligibility::Disabled)
    } else if policy.paused() {
        Some(PollIneligibility::Paused)
    } else if policy.recovery_suspended() {
        Some(PollIneligibility::RecoverySuspended)
    } else {
        None
    };
    if let Some(reason) = reason {
        return AutomaticPollEligibility::NotEligible {
            reason,
            next_eligible_at: next,
        };
    }
    match next {
        Some(next) if next > now => {
            let bound = now.saturating_add(time::Duration::seconds(
                policy.interval().as_secs().max(900) as i64,
            ));
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(next.min(bound)),
            }
        }
        _ => AutomaticPollEligibility::Eligible,
    }
}

/// `None` means keep the stored deadline, not clear it. Busy, yield, interruption,
/// conflict-pending and not-eligible have no outcome category and do not call this.
/// The policy is the pre-finish policy; reservation finish still owns backoff writes.
#[allow(dead_code)] // Consumed by reservation finish in Cycle 08 Task 2.
pub(super) fn next_eligible(
    category: RemoteOutcomeCategory,
    invocation: RemotePollInvocation,
    completed_at: OffsetDateTime,
    policy: &RemotePollingConfiguration,
) -> Option<OffsetDateTime> {
    let seconds = match category {
        RemoteOutcomeCategory::Cancelled => return None,
        RemoteOutcomeCategory::Completed => policy.interval().as_secs(),
        _ if invocation == RemotePollInvocation::Explicit => return None,
        RemoteOutcomeCategory::TransportUnavailable | RemoteOutcomeCategory::ProtocolRejected => {
            policy
                .automatic_backoff()
                .map_or(60, |backoff| backoff.as_secs().saturating_mul(2).min(900))
        }
        RemoteOutcomeCategory::SelectedKeyUnavailable
        | RemoteOutcomeCategory::UnlockRequired
        | RemoteOutcomeCategory::HostApprovalRequired
        | RemoteOutcomeCategory::ConfigurationRequired
        | RemoteOutcomeCategory::RepositoryUnavailable => policy.interval().as_secs(),
    };
    Some(completed_at.saturating_add(time::Duration::seconds(seconds as i64)))
}

fn automatic_poll_eligibility_at(
    service: &RepositoryService,
    root: &Path,
    now: OffsetDateTime,
) -> Result<AutomaticPollEligibility, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        let (policy, _, next) = state::select_polling_status(tx, id)
            .map_err(|_| state::recovery_required())?
            .ok_or_else(state::recovery_required)??;
        let result = eligibility(&policy, next, now);
        if let AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(clamped),
        } = result
            && Some(clamped) != next
        {
            tx.execute(
                "UPDATE remote_polling_state SET next_eligible_at=?2 WHERE repository_id=?1",
                params![id, clamped.unix_timestamp()],
            )
            .map_err(|_| state::recovery_required())?;
        }
        Ok(result)
    })
}

impl RepositoryService {
    /// Check automatic polling policy without contacting or configuring an
    /// endpoint or reserving an operation. Only a clock-rollback clamp is written.
    pub fn automatic_poll_eligibility(
        &self,
        root: &Path,
    ) -> Result<AutomaticPollEligibility, RepositoryError> {
        automatic_poll_eligibility_at(self, root, OffsetDateTime::now_utc())
    }
}

// Task 6 contracts; behavior is introduced after the recorded regression RED.
#[derive(Clone)]
pub struct PollPublicationRemoteRequest {
    pub root: std::path::PathBuf,
    pub operation_id: crate::repository::OperationId,
    pub invocation: RemotePollInvocation,
    pub approval: Option<crate::repository::transport::HostApproval>,
    pub restart: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollCompletion {
    Completed,
    Interrupted,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PollTargetReport {
    pub(super) target: Option<super::RemoteRefTarget>,
    pub(super) kind: Option<AuthoringKind>,
    pub(super) item_id: Option<canonical::ItemId>,
    pub(super) disposition: PollDisposition,
    pub(super) before_oid: Option<git2::Oid>,
    pub(super) after_oid: Option<git2::Oid>,
}
impl PollTargetReport {
    pub fn target(&self) -> Option<&super::RemoteRefTarget> {
        self.target.as_ref()
    }
    pub fn kind(&self) -> Option<AuthoringKind> {
        self.kind
    }
    pub fn item_id(&self) -> Option<&canonical::ItemId> {
        self.item_id.as_ref()
    }
    pub fn disposition(&self) -> PollDisposition {
        self.disposition
    }
    pub fn before_oid(&self) -> Option<git2::Oid> {
        self.before_oid
    }
    pub fn after_oid(&self) -> Option<git2::Oid> {
        self.after_oid
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemotePollReport {
    operation_id: crate::repository::OperationId,
    completion: PollCompletion,
    completed_at: OffsetDateTime,
    next_eligible_at: Option<OffsetDateTime>,
    targets: Vec<PollTargetReport>,
    outcome: Option<RemoteOutcomeCategory>,
}
impl RemotePollReport {
    pub fn operation_id(&self) -> crate::repository::OperationId {
        self.operation_id
    }
    pub fn completion(&self) -> PollCompletion {
        self.completion
    }
    pub fn completed_at(&self) -> OffsetDateTime {
        self.completed_at
    }
    pub fn next_eligible_at(&self) -> Option<OffsetDateTime> {
        self.next_eligible_at
    }
    pub fn targets(&self) -> &[PollTargetReport] {
        &self.targets
    }
    pub fn outcome(&self) -> Option<RemoteOutcomeCategory> {
        self.outcome
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemotePollOutcome {
    NotEligible {
        reason: PollIneligibility,
        next_eligible_at: Option<OffsetDateTime>,
    },
    Polled(RemotePollReport),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemotePollResult {
    Complete(RemotePollOutcome),
    IndexPending(crate::repository::IndexPending<RemotePollOutcome>),
}
pub enum RemotePollError {
    Repository(RepositoryError),
    Transport(crate::repository::transport::SshTransportError),
    Busy,
    PollYielding,
    Interrupted,
    ConflictPending,
}
impl std::fmt::Debug for RemotePollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::fmt::Display for RemotePollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Repository(_) => "remote poll repository unavailable",
            Self::Transport(_) => "remote poll transport unavailable",
            Self::Busy => "remote operation busy",
            Self::PollYielding => "remote poll yielding",
            Self::Interrupted => "remote poll interrupted",
            Self::ConflictPending => "remote poll conflict pending",
        })
    }
}
impl std::error::Error for RemotePollError {}
impl From<RepositoryError> for RemotePollError {
    fn from(e: RepositoryError) -> Self {
        Self::Repository(e)
    }
}
impl RepositoryService {
    pub fn poll_publication_remote<P: crate::repository::keys::SessionCredentialProvider>(
        &self,
        request: PollPublicationRemoteRequest,
        session: &mut crate::repository::keys::SessionCredentials<P>,
    ) -> Result<RemotePollResult, RemotePollError> {
        use super::{
            RemoteOperationPriority, RemoteOperationTarget, RemoteReservationOutcome as R,
            reservation::RemotePollReservationOutcome as PR,
        };
        use crate::repository::transport::*;
        let (_, root) = crate::repository::canonical_repository_root(
            &request.root,
            RepositoryOperation::RepositorySnapshot,
        )?;
        let priority = if request.invocation == RemotePollInvocation::Automatic {
            RemoteOperationPriority::Poll
        } else {
            RemoteOperationPriority::Manual
        };
        // Terminal replay is checked before policy/endpoint/authentication work.
        let existing = state::with_transaction(self, &root, |tx, id| {
            state::read_operation(tx, id, request.operation_id)
        })?;
        let replay_reservation = if let Some(record) = existing {
            // The immutable stored target validates kind/invocation/ID. Live
            // configuration cannot redefine a terminal attempt's identity.
            match self.reserve_remote_poll(&root, request.operation_id, &record.target, priority)? {
                PR::Reservation(r) => Some(r),
                PR::ConflictPending => return Err(RemotePollError::ConflictPending),
            }
        } else {
            None
        };
        if let Some(R::Replay(record)) = &replay_reservation {
            if matches!(
                record.phase(),
                super::RemoteOperationPhase::Completed
                    | super::RemoteOperationPhase::Cancelled
                    | super::RemoteOperationPhase::Failed
            ) {
                return self.poll_stored_result(&root, request.operation_id);
            }
            if !request.restart {
                return Err(RemotePollError::Interrupted);
            }
        }
        if request.invocation == RemotePollInvocation::Automatic
            && let AutomaticPollEligibility::NotEligible {
                reason,
                next_eligible_at,
            } = self.automatic_poll_eligibility(&root)?
        {
            return Ok(RemotePollResult::Complete(RemotePollOutcome::NotEligible {
                reason,
                next_eligible_at,
            }));
        }
        // Only a new execution or an eligible deliberate restart needs a live plan.
        let plan = match super::observation::observation_plan(&root) {
            Ok(plan) => plan,
            Err(error) => {
                session.clear_cached_passphrase();
                state::with_transaction(self, &root, |tx, id| {
                    let policy = state::read_snapshot(tx, id)?.polling().clone();
                    // Configuration fencing clears observer retry state. Full
                    // poll attention uses the pre-failure policy in this same TX.
                    state::configure(tx, id, None, false)?;
                    let now = OffsetDateTime::now_utc();
                    state::record_outcome(
                        tx,
                        id,
                        RemoteOutcomeCategory::ConfigurationRequired,
                        policy.automatic_backoff().map(|b| {
                            super::AutomaticBackoff::from_seconds(b.as_secs())
                                .expect("validated backoff")
                        }),
                    )?;
                    state::record_poll_attempt(
                        tx,
                        id,
                        now,
                        next_eligible(
                            RemoteOutcomeCategory::ConfigurationRequired,
                            request.invocation,
                            now,
                            &policy,
                        ),
                    )
                })?;
                return Err(RemotePollError::Transport(error));
            }
        };
        let target = RemoteOperationTarget::for_poll(&plan);
        if replay_reservation.is_some() {
            // Restart still requires the original target, before configuration writes.
            self.reserve_remote_poll(&root, request.operation_id, &target, priority)?;
        }
        let configuration = self.observation_configuration(&root, &plan)?;
        let may_reserve = state::with_transaction(self, &root, |tx, id| {
            crate::repository::recovery::require_no_pending_local(tx, id)?;
            if state::has_pending_conflict(tx, id)? {
                return Ok(false);
            }
            state::configure_poll_endpoints(tx, id, &plan, &configuration.endpoint_digest())?;
            Ok(true)
        })?;
        if !may_reserve {
            return Err(RemotePollError::ConflictPending);
        }
        let reserved = if request.restart && replay_reservation.is_some() {
            self.restart_remote_poll(&root, request.operation_id, &target)?
        } else {
            match self.reserve_remote_poll(&root, request.operation_id, &target, priority)? {
                PR::Reservation(r) => r,
                PR::ConflictPending => return Err(RemotePollError::ConflictPending),
            }
        };
        let owner = match reserved {
            R::Reserved(o) => o,
            R::Busy => return Err(RemotePollError::Busy),
            R::PollYielding => return Err(RemotePollError::PollYielding),
            R::Replay(_) => return self.poll_stored_result(&root, request.operation_id),
        };
        if let Some(result) = self.poll_safe_point(
            &root,
            &owner,
            super::RemoteOperationSafePoint::BeforeTransport,
        )? {
            return Ok(result);
        }
        let mut stopped = None;
        let mut last = None;
        let fetched = self.with_authenticated_remote_policy_expected(
            VerifySshTransportRequest {
                root: root.clone(),
                direction: SshDirection::Fetch,
                approval: request.approval,
            },
            session,
            request.invocation == RemotePollInvocation::Automatic,
            configuration
                .transport_expectation(SshDirection::Fetch)
                .as_ref(),
            |remote| {
                remote.fetch_poll_set(&plan, |_| {
                    if stopped.is_some() {
                        return false;
                    }
                    let now = std::time::Instant::now();
                    if last.is_some_and(|at: std::time::Instant| {
                        now.duration_since(at) < std::time::Duration::from_millis(250)
                    }) {
                        return true;
                    }
                    last = Some(now);
                    let decision = super::reservation::poll_progress(self, &owner)
                        .unwrap_or(super::RemoteSafePointOutcome::Interrupted);
                    if decision != super::RemoteSafePointOutcome::Continue {
                        stopped = Some(decision);
                        false
                    } else {
                        true
                    }
                })
            },
        );
        // Captured callback intent wins over libgit2's platform-specific abort code.
        if let Some(decision) = stopped {
            if let Some(result) = self.poll_safe_point(
                &root,
                &owner,
                super::RemoteOperationSafePoint::AfterAdvertisement,
            )? {
                return Ok(result);
            }
            let interrupted = self.poll_interrupt(&root, &owner, None)?;
            return if decision == super::RemoteSafePointOutcome::Cancelled {
                interrupted.ok_or(RemotePollError::Interrupted)
            } else {
                Err(RemotePollError::Interrupted)
            };
        }
        match fetched {
            Ok(advertised) => {
                self.poll_after_fetch(&root, &owner, &plan, &configuration, advertised)
            }
            Err(error) => {
                if let Some(result) = self.poll_safe_point(
                    &root,
                    &owner,
                    super::RemoteOperationSafePoint::AfterAdvertisement,
                )? {
                    return Ok(result);
                }
                let (decision, captured) = self.finish_remote_operation_with(
                    &root,
                    &owner,
                    super::observation::transport_category(&error.kind),
                    |tx, id, decision| {
                        self.poll_capture_stop(tx, id, owner.operation_id(), decision, None)
                    },
                )?;
                if decision != super::RemoteSafePointOutcome::Continue {
                    return captured.ok_or(RemotePollError::Interrupted);
                }
                Err(RemotePollError::Transport(error))
            }
        }
    }

    /// Everything after a successful real fetch. Local tests supply actual ODB
    /// commits and test-installed tracking refs; this is not a transport adapter.
    pub(crate) fn poll_after_fetch(
        &self,
        root: &Path,
        owner: &super::RemoteReservation,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
        advertised: Vec<(String, git2::Oid)>,
    ) -> Result<RemotePollResult, RemotePollError> {
        use super::{
            RemoteOperationSafePoint as S, RemoteRefClassification as C,
            RemoteSafePointOutcome as D,
        };
        if let Some(result) = self.poll_safe_point(root, owner, S::AfterAdvertisement)? {
            return Ok(result);
        }
        let repository = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
        let previous = state::with_transaction(self, root, |tx, id| state::read_snapshot(tx, id))?;
        let mut observations = Vec::new();
        for (name, oid) in advertised {
            if let Some(result) = self.poll_safe_point(root, owner, S::BetweenObservations)? {
                return Ok(result);
            }
            if let Some(observation) = super::RemoteRefObservation::from_advertisement(
                plan,
                &name,
                oid,
                plan.tracking_ref_for(&name)
                    .and_then(|r| repository.refname_to_id(&r).ok()),
            ) {
                observations.push(observation);
            }
        }
        let mut existing = std::collections::BTreeMap::new();
        for branch in repository
            .branches(Some(git2::BranchType::Local))
            .map_err(|_| state::recovery_required())?
        {
            let (branch, _) = branch.map_err(|_| state::recovery_required())?;
            if let Some(name) = branch.get().name()
                && let Some(C::RecognizedContext { kind, item_id }) =
                    plan.classify_advertised_ref(name)
            {
                existing.insert(name.to_owned(), (kind, item_id));
            }
        }
        for context in previous.contexts() {
            if let (Some(kind), Some(item), Some(target)) =
                (context.kind(), context.item_id(), context.target())
                && (context.state() == super::RemoteContextState::Observed
                    || poll_registration_present(&repository, &item.to_string()))
            {
                existing
                    .entry(target.remote_ref().into())
                    .or_insert((kind, item.clone()));
            }
        }
        let mut targets = vec![PollTargetReport {
            target: Some(plan.primary().clone()),
            kind: None,
            item_id: None,
            disposition: PollDisposition::NotReached,
            before_oid: None,
            after_oid: None,
        }];
        for (kind, item) in existing.values() {
            targets.push(PollTargetReport {
                target: Some(plan.context(*kind, item)),
                kind: Some(*kind),
                item_id: Some(item.clone()),
                disposition: PollDisposition::NotReached,
                before_oid: None,
                after_oid: None,
            });
        }
        let mut new = observations
            .iter()
            .filter(|o| {
                matches!(o.classification(), C::RecognizedContext { .. })
                    && o.remote_ref().is_some_and(|r| !existing.contains_key(r))
            })
            .collect::<Vec<_>>();
        new.sort_by_key(|o| o.remote_ref());
        for o in new {
            if let C::RecognizedContext { kind, item_id } = o.classification() {
                targets.push(PollTargetReport {
                    target: Some(plan.context(*kind, item_id)),
                    kind: Some(*kind),
                    item_id: Some(item_id.clone()),
                    disposition: PollDisposition::NotReached,
                    before_oid: None,
                    after_oid: None,
                });
            }
        }
        for o in &observations {
            if o.classification() == &C::MalformedContext {
                targets.push(PollTargetReport {
                    target: None,
                    kind: None,
                    item_id: None,
                    disposition: PollDisposition::NotReached,
                    before_oid: None,
                    after_oid: None,
                });
            }
        }
        let frozen = targets
            .iter()
            .map(|t| {
                t.target.as_ref().map(|r| {
                    (
                        direct_oid(&repository, r.remote_ref()),
                        repository.refname_to_id(r.tracking_ref()).ok(),
                    )
                })
            })
            .collect::<Vec<_>>();
        if let Some(result) = self.poll_safe_point(root, owner, S::BeforeBatchCommit)? {
            return Ok(result);
        }
        if !self.poll_configuration_matches(root, plan, configuration)? {
            let _ = self.poll_interrupt(root, owner, None)?;
            return Err(RemotePollError::Interrupted);
        }
        let at = OffsetDateTime::now_utc().unix_timestamp();
        let (decision, captured) = super::reservation::commit_observation_batch_with_capture(
            self,
            root,
            owner,
            plan,
            &observations,
            at,
            |tx, id| state::replace_poll_dispositions(tx, id, owner.operation_id(), &targets, at),
            |tx, id, decision| self.poll_capture_stop(tx, id, owner.operation_id(), decision, None),
        )?;
        if decision != D::Continue {
            return captured.ok_or(RemotePollError::Interrupted);
        }
        if let Some(result) = self.poll_safe_point_with_targets(
            root,
            owner,
            S::AfterBatchCommit,
            Some((&targets, false)),
        )? {
            return Ok(result);
        }
        let mut local_changes = false;
        for ordinal in 0..targets.len() {
            if let Some(result) = self.poll_safe_point_with_targets(
                root,
                owner,
                S::BeforeLocalMutation,
                Some((&targets, local_changes)),
            )? {
                return Ok(result);
            }
            // Publish to the caller's shadow only after the row write commits.
            // A failed/incomplete target retains its durable NotReached image.
            let mut row = targets[ordinal].clone();
            self.poll_boundary(
                crate::repository::FailurePoint::PollBeforeTarget,
                Some(ordinal),
                root,
            )?;
            let result = (|| -> Result<(), RemotePollError> {
                let _lease =
                    repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
                if !self.poll_configuration_matches(root, plan, configuration)? {
                    return Err(RemotePollError::Interrupted);
                }
                let flags = super::reservation::poll_progress(self, owner)?;
                if flags != D::Continue {
                    return Err(RemotePollError::Interrupted);
                }
                state::with_transaction(self, root, |tx, id| {
                    super::reservation::owned(self, tx, id, owner).map(|_| ())
                })?;
                if let Some(reference) = &row.target {
                    let current = (
                        direct_oid(&repository, reference.remote_ref()),
                        repository.refname_to_id(reference.tracking_ref()).ok(),
                    );
                    if Some(current) != frozen[ordinal] {
                        return Err(RemotePollError::Interrupted);
                    }
                    let observed = observations
                        .iter()
                        .find(|o| o.remote_ref() == Some(reference.remote_ref()));
                    let tracking = match observed {
                        Some(o) if o.tracking_oid() == Some(o.advertised_oid()) => {
                            PollTrackingState::Present(o.advertised_oid())
                        }
                        Some(_) => return Err(RemotePollError::Interrupted),
                        None if row.kind.is_some()
                            && previous.fetch_publication_evidence_for(
                                row.kind.expect("context"),
                                row.item_id.as_ref().expect("context"),
                            ) == super::RemotePublicationEvidence::ObservedPublished =>
                        {
                            PollTrackingState::RemotelyDeleted
                        }
                        None => PollTrackingState::HistoryUnknown,
                    };
                    let target = match (row.kind, row.item_id.as_ref()) {
                        (Some(kind), Some(item)) => SynchronizationTarget::Context {
                            kind,
                            item_id: item.clone(),
                        },
                        _ => SynchronizationTarget::Primary,
                    };
                    row.before_oid = current.0;
                    let path = row
                        .item_id
                        .as_ref()
                        .map(|id| root.join(".manyhands/worktrees").join(id.to_string()));
                    let before_present = path.as_ref().is_some_and(|p| poll_entry_present(p));
                    let before_admin = row
                        .item_id
                        .as_ref()
                        .is_some_and(|id| poll_registration_present(&repository, &id.to_string()));
                    let mut newly_materialized = false;
                    if let (
                        PollTrackingState::Present(oid),
                        SynchronizationTarget::Context { kind, item_id },
                    ) = (tracking, &target)
                    {
                        let intent = state::with_transaction(self, root, |tx, id| {
                            state::materialization_intents(tx, id, item_id)
                        })?;
                        let registered_local =
                            repository.find_worktree(&item_id.to_string()).is_ok()
                                || previous.contexts().iter().any(|context| {
                                    context.kind() == Some(*kind)
                                        && context.item_id() == Some(item_id)
                                        && context.state() == super::RemoteContextState::Observed
                                });
                        if !intent.is_empty() || !registered_local {
                            // Independently durable before any provisioning
                            // effect: later intent/disposition writes may fail.
                            self.poll_prepare_local_effect(root, owner)?;
                            match self
                                .materialize_context_unlocked(
                                    root,
                                    plan,
                                    configuration,
                                    &PollMaterializationRequest {
                                        operation_id: owner.operation_id(),
                                        kind: *kind,
                                        item_id: item_id.clone(),
                                        oid,
                                    },
                                    &repository,
                                )
                                .map_err(poll_local_error)?
                            {
                                PollMaterializationOutcome::Materialized => {
                                    row.disposition = PollDisposition::Materialized;
                                    newly_materialized = true;
                                }
                                PollMaterializationOutcome::Present => {}
                                PollMaterializationOutcome::Refused(d) => {
                                    row.disposition = d;
                                }
                            }
                        }
                    }
                    if row.disposition == PollDisposition::NotReached {
                        row.disposition = match classify_local_target(
                            root,
                            plan.primary_branch(),
                            &target,
                            tracking,
                        )
                        .map_err(poll_local_error)?
                        {
                            LocalPollClassification::Unchanged { disposition, .. } => disposition,
                            LocalPollClassification::FastForward {
                                local_oid,
                                tracking_oid,
                            } => {
                                self.poll_prepare_local_effect(root, owner)?;
                                self.poll_fast_forward_unlocked(
                                    root,
                                    plan,
                                    configuration,
                                    &PollFastForward {
                                        target,
                                        before: local_oid,
                                        after: tracking_oid,
                                    },
                                    &repository,
                                )
                                .map_err(poll_local_error)?
                            }
                        };
                    }
                    row.after_oid = direct_oid(&repository, reference.remote_ref());
                    let changed = newly_materialized
                        || row.before_oid != row.after_oid
                        || path.as_ref().is_some_and(|p| poll_entry_present(p)) != before_present
                        || row.item_id.as_ref().is_some_and(|id| {
                            poll_registration_present(&repository, &id.to_string())
                        }) != before_admin;
                    state::with_transaction(self, root, |tx, id| {
                        super::reservation::owned(self, tx, id, owner)?;
                        state::update_poll_disposition(tx, id, owner.operation_id(), ordinal, &row)
                    })?;
                    local_changes |= changed;
                } else {
                    row.disposition = PollDisposition::Malformed;
                    state::with_transaction(self, root, |tx, id| {
                        super::reservation::owned(self, tx, id, owner)?;
                        state::update_poll_disposition(tx, id, owner.operation_id(), ordinal, &row)
                    })?;
                }
                Ok(())
            })();
            if let Err(error) = result {
                if matches!(error, RemotePollError::Interrupted) {
                    // A pending request gets its normal durable acknowledgement;
                    // otherwise fence a changed frozen input without retrying.
                    if let Some(result) = self.poll_safe_point_with_targets(
                        root,
                        owner,
                        S::BeforeLocalMutation,
                        Some((&targets, local_changes)),
                    )? {
                        return Ok(result);
                    }
                    return self
                        .poll_interrupt(root, owner, Some((&targets, local_changes)))?
                        .ok_or(RemotePollError::Interrupted);
                }
                return Err(error);
            }
            targets[ordinal] = row;
        }
        self.poll_boundary(
            crate::repository::FailurePoint::PollBeforeDiscovery,
            None,
            root,
        )?;
        if !self.poll_configuration_matches(root, plan, configuration)? {
            return self
                .poll_interrupt(root, owner, Some((&targets, local_changes)))?
                .ok_or(RemotePollError::Interrupted);
        }
        if let Some(result) = self.poll_safe_point_with_targets(
            root,
            owner,
            S::BeforeDiscovery,
            Some((&targets, local_changes)),
        )? {
            return Ok(result);
        }
        state::with_transaction(self, root, |tx, id| {
            super::reservation::owned(self, tx, id, owner)?;
            tx.execute(
                "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                [id],
            )
            .map_err(|_| state::recovery_required())?;
            Ok(())
        })?;
        let (decision, captured) = self.finish_remote_operation_with(
            root,
            owner,
            RemoteOutcomeCategory::Completed,
            |tx, id, decision| {
                Ok((
                    decision,
                    self.poll_result_in_transaction(
                        tx,
                        id,
                        owner.operation_id(),
                        Some((&targets, local_changes)),
                    )?,
                ))
            },
        )?;
        let result = captured.ok_or(RemotePollError::Interrupted)?;
        if decision != D::Continue {
            self.poll_boundary(
                crate::repository::FailurePoint::PollAfterCompletion,
                None,
                root,
            )?;
            return Ok(result);
        }
        self.poll_boundary(
            crate::repository::FailurePoint::PollAfterCompletion,
            None,
            root,
        )?;
        let refreshed = self.refresh_repository(crate::repository::RefreshRepositoryRequest {
            root: root.to_owned(),
            operation_id: crate::repository::OperationId::new(),
        });
        if matches!(
            refreshed,
            Ok(crate::repository::RefreshOutcome::Refreshed { .. })
        ) {
            Ok(match result {
                RemotePollResult::IndexPending(p) => RemotePollResult::Complete(p.authoritative),
                other => other,
            })
        } else {
            Ok(match result {
                RemotePollResult::Complete(o) => {
                    RemotePollResult::IndexPending(crate::repository::IndexPending::new(o))
                }
                other => other,
            })
        }
    }
    fn poll_prepare_local_effect(
        &self,
        root: &Path,
        owner: &super::RemoteReservation,
    ) -> Result<(), RepositoryError> {
        // Called under the lease after frozen-input/ownership validation, but
        // before Git can change. A failed effect can conservatively leave the
        // index stale; a surviving effect must never be hidden by SQLite rollback.
        state::with_transaction(self, root, |tx, id| {
            super::reservation::owned(self, tx, id, owner)?;
            tx.execute(
                "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                [id],
            )
            .map_err(|_| state::recovery_required())?;
            Ok(())
        })
    }

    fn poll_configuration_matches(
        &self,
        root: &Path,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
    ) -> Result<bool, RepositoryError> {
        Ok(
            super::observation::observation_plan(root).ok().as_ref() == Some(plan)
                && self.observation_configuration(root, plan)? == *configuration,
        )
    }
    fn poll_interrupt(
        &self,
        root: &Path,
        owner: &super::RemoteReservation,
        targets: Option<(&[PollTargetReport], bool)>,
    ) -> Result<Option<RemotePollResult>, RepositoryError> {
        state::with_transaction(self, root, |tx, id| {
            if let Ok(record) = super::reservation::owned(self, tx, id, owner) {
                tx.execute("UPDATE remote_operation_records SET phase='interrupted',outcome=NULL,updated_at=max(updated_at,?2) WHERE id=?1",params![record.id,OffsetDateTime::now_utc().unix_timestamp()]).map_err(|_|state::recovery_required())?;
                state::record_poll_completion(
                    tx,
                    id,
                    owner.operation_id(),
                    OffsetDateTime::now_utc().unix_timestamp(),
                )?;
            } else {
                super::reservation::fenced_poll_record(self, tx, id, owner)?;
            }
            self.poll_result_in_transaction(tx, id, owner.operation_id(), targets)
        })
    }
    fn poll_safe_point(
        &self,
        root: &Path,
        owner: &super::RemoteReservation,
        point: super::RemoteOperationSafePoint,
    ) -> Result<Option<RemotePollResult>, RemotePollError> {
        self.poll_safe_point_with_targets(root, owner, point, None)
    }
    fn poll_safe_point_with_targets(
        &self,
        root: &Path,
        owner: &super::RemoteReservation,
        point: super::RemoteOperationSafePoint,
        targets: Option<(&[PollTargetReport], bool)>,
    ) -> Result<Option<RemotePollResult>, RemotePollError> {
        let (decision, captured) =
            match self.remote_safe_point_with(root, owner, point, |tx, id, decision| {
                self.poll_capture_stop(tx, id, owner.operation_id(), decision, targets)
            }) {
                Ok(result) => result,
                Err(e) => {
                    let stopped = state::with_transaction(self, root, |tx, id| {
                        if super::reservation::fenced_poll_record(self, tx, id, owner).is_err() {
                            return Ok(None);
                        }
                        Ok(Some(self.poll_result_in_transaction(
                            tx,
                            id,
                            owner.operation_id(),
                            targets,
                        )?))
                    })?;
                    match stopped {
                        Some(result) => (super::RemoteSafePointOutcome::Interrupted, result),
                        None => return Err(e.into()),
                    }
                }
            };
        match decision {
            super::RemoteSafePointOutcome::Continue => Ok(None),
            _ => {
                // The callback may let a later poll replace the latest set;
                // captured is already the original executor's owned response.
                self.poll_boundary(
                    crate::repository::FailurePoint::PollAfterCompletion,
                    None,
                    root,
                )?;
                captured.map(Some).ok_or(RemotePollError::Interrupted)
            }
        }
    }
    fn poll_capture_stop(
        &self,
        tx: &rusqlite::Transaction<'_>,
        id: i64,
        op: crate::repository::OperationId,
        decision: super::RemoteSafePointOutcome,
        targets: Option<(&[PollTargetReport], bool)>,
    ) -> Result<(super::RemoteSafePointOutcome, Option<RemotePollResult>), RepositoryError> {
        Ok((
            decision,
            if decision == super::RemoteSafePointOutcome::Continue {
                None
            } else {
                self.poll_result_in_transaction(tx, id, op, targets)?
            },
        ))
    }
    fn poll_stored_result(
        &self,
        root: &Path,
        op: crate::repository::OperationId,
    ) -> Result<RemotePollResult, RemotePollError> {
        state::with_transaction(self, root, |tx, id| {
            self.poll_result_in_transaction(tx, id, op, None)
        })?
        .ok_or(RemotePollError::Interrupted)
    }
    fn poll_result_in_transaction(
        &self,
        tx: &rusqlite::Transaction<'_>,
        id: i64,
        op: crate::repository::OperationId,
        original_targets: Option<(&[PollTargetReport], bool)>,
    ) -> Result<Option<RemotePollResult>, RepositoryError> {
        let record = state::read_operation(tx, id, op)?.ok_or_else(state::recovery_required)?;
        if record.phase == super::RemoteOperationPhase::Interrupted && original_targets.is_none() {
            let latest:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_poll_dispositions WHERE repository_id=?1 AND operation_ulid=?2)",params![id,op.to_string()],|r|r.get(0)).map_err(|_|state::recovery_required())?;
            if !latest {
                return Ok(None);
            }
        }
        let (completed_at, next_eligible_at, latest) = state::poll_completion_metadata(tx, id, op)?;
        let completion = match record.phase {
            super::RemoteOperationPhase::Interrupted => PollCompletion::Interrupted,
            super::RemoteOperationPhase::Cancelled => PollCompletion::Cancelled,
            _ => PollCompletion::Completed,
        };
        let targets = if let Some((original, _)) = original_targets {
            original.to_vec()
        } else if latest {
            state::read_poll_dispositions(tx, id)?
        } else {
            Vec::new()
        };
        let outcome = RemotePollOutcome::Polled(RemotePollReport {
            operation_id: op,
            completion,
            completed_at,
            next_eligible_at,
            targets,
            outcome: record.outcome,
        });
        let pending: bool = tx
            .query_row(
                "SELECT refresh_required FROM repositories WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|_| state::recovery_required())?;
        // A foreign generation fence may release this owner before it observes
        // the stop. Later index repair cannot rewrite its original handoff.
        let pending = pending
            || original_targets
                .is_some_and(|(_, changed)| changed && completion != PollCompletion::Completed);
        Ok(Some(if pending {
            RemotePollResult::IndexPending(crate::repository::IndexPending::new(outcome))
        } else {
            RemotePollResult::Complete(outcome)
        }))
    }
}

fn direct_oid(repository: &git2::Repository, reference: &str) -> Option<git2::Oid> {
    repository
        .find_reference(reference)
        .ok()
        .and_then(|r| r.target())
}
fn poll_local_error(e: SynchronizationError) -> RemotePollError {
    match e {
        SynchronizationError::ExternalChange => RemotePollError::Interrupted,
        SynchronizationError::Repository(e) => RemotePollError::Repository(e),
        _ => RemotePollError::Repository(state::recovery_required()),
    }
}

#[cfg(test)]
#[path = "poll_tests.rs"]
mod tests;
