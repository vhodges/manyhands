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
    let Ok(linked) = sync::materialized_target(root, primary, target) else {
        return Ok(unchanged(PollDisposition::Inaccessible, None));
    };
    let expected_path = match target {
        SynchronizationTarget::Primary => root.to_owned(),
        SynchronizationTarget::Context { item_id, .. } => {
            root.join(".manyhands/worktrees").join(item_id.to_string())
        }
    };
    if linked.workdir().and_then(|p| p.canonicalize().ok()) != expected_path.canonicalize().ok() {
        return Ok(unchanged(PollDisposition::Inaccessible, None));
    }
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
        let linked = sync::materialized_target(root, plan.primary_branch(), &update.target)?;
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
        if sync::local_oid(&linked)? != update.before
            || repository.refname_to_id(target_ref.tracking_ref()).ok() != Some(update.after)
        {
            return Err(SynchronizationError::ExternalChange);
        }
        sync::fast_forward(&linked, branch, update.before, update.after)?;
        if sync::local_oid(&linked)? != update.after {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(PollDisposition::FastForwarded)
    }
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
        let is_document = kind == AuthoringKind::Document && path_text.starts_with("docs/");
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

#[cfg(test)]
#[path = "poll_tests.rs"]
mod tests;
