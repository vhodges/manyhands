//! Deliberate synchronization composes the owned state and scoped transport seams.
#![allow(clippy::result_large_err)]
#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
use super::*;
use crate::repository::{keys::*, transport::*, *};
use state::{
    SynchronizationAuthority as Authority, SynchronizationCheckpoint as Checkpoint,
    SynchronizationEvidence as Evidence,
};
use std::{fmt, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublishPendingReason {
    NoPublicationRemote,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SynchronizationOutcome {
    Published {
        target: SynchronizationTarget,
        oid: git2::Oid,
    },
    AlreadyCurrent {
        target: SynchronizationTarget,
        oid: git2::Oid,
    },
    PublishPending {
        target: SynchronizationTarget,
        local_oid: git2::Oid,
        reason: PublishPendingReason,
    },
}
/// A redacted inspected conflict entry. The path is represented only by its
/// capability token; callers cannot substitute a repository path.
#[derive(Clone, Debug)]
pub struct SynchronizationConflictPath {
    pub token: merge::ConflictPathToken,
    pub eligibility: merge::ConflictEligibility,
}

/// Actual Git state correlated with one retained durable integration step.
#[derive(Clone, Debug)]
pub struct SynchronizationConflictInspection {
    pub operation_id: OperationId,
    pub target: SynchronizationTarget,
    pub stage: SynchronizationStage,
    pub local_parent: git2::Oid,
    pub incoming_parent: git2::Oid,
    pub observation: merge::ConflictObservation,
    pub paths: Vec<SynchronizationConflictPath>,
}

/// Explicit, ephemeral conflict sides. Debug formatting redacts every body.
#[derive(Clone)]
pub struct EphemeralSynchronizationConflictSides {
    pub base: Option<merge::RedactedConflictBytes>,
    pub local: Option<merge::RedactedConflictBytes>,
    pub incoming: Option<merge::RedactedConflictBytes>,
    pub current: Option<merge::RedactedConflictBytes>,
}

impl fmt::Debug for EphemeralSynchronizationConflictSides {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EphemeralSynchronizationConflictSides(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynchronizationStage {
    Context,
    Primary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SynchronizationResult {
    Complete(SynchronizationOutcome),
    IndexPending(IndexPending<SynchronizationOutcome>),
}
#[derive(Debug)]
pub enum SynchronizationError {
    Repository(RepositoryError),
    Transport(SshTransportError),
    Busy,
    PollYielding,
    Interrupted,
    TargetNotMaterialized,
    WorktreeNotClean {
        target: SynchronizationTarget,
    },
    WorktreeConflicted {
        target: SynchronizationTarget,
    },
    PrimaryMissing,
    RemoteContextDeleted,
    HistoryUnknown,
    /// A real Git merge is installed in the target worktree. The branch HEAD
    /// remains on the local parent and callers must inspect/repair explicitly.
    ConflictPending {
        target: SynchronizationTarget,
        operation_id: OperationId,
        stage: SynchronizationStage,
    },
    /// A divergent merge needs a committing identity; no candidate or merge
    /// state was created.
    IdentityRequired {
        target: SynchronizationTarget,
    },
    ExternalResolutionRequired {
        target: SynchronizationTarget,
        operation_id: OperationId,
    },
    MergeRequired {
        target: SynchronizationTarget,
    },
    PushRejected,
    ExternalChange,
    RecoveryRequired,
}
impl fmt::Display for SynchronizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Repository(_) => "synchronization repository unavailable",
            Self::Transport(_) => "synchronization transport unavailable",
            Self::Busy => "remote operation busy",
            Self::PollYielding => "automatic observation yielding",
            Self::Interrupted => "synchronization interrupted",
            Self::TargetNotMaterialized => "target not materialized",
            Self::WorktreeNotClean { .. } => "target worktree not clean",
            Self::WorktreeConflicted { .. } => "target worktree conflicted",
            Self::PrimaryMissing => "primary missing",
            Self::RemoteContextDeleted => "remote context deleted",
            Self::HistoryUnknown => "publication history unknown",
            Self::ConflictPending { .. } => "synchronization conflict pending",
            Self::IdentityRequired { .. } => "commit identity required",
            Self::ExternalResolutionRequired { .. } => "external conflict resolution required",
            Self::MergeRequired { .. } => "merge required",
            Self::PushRejected => "push rejected",
            Self::ExternalChange => "external change",
            Self::RecoveryRequired => "synchronization recovery required",
        })
    }
}
impl std::error::Error for SynchronizationError {}
impl From<RepositoryError> for SynchronizationError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}
impl From<SshTransportError> for SynchronizationError {
    fn from(error: SshTransportError) -> Self {
        if error.kind == SshTransportErrorKind::PushRejected {
            Self::PushRejected
        } else {
            Self::Transport(error)
        }
    }
}
#[derive(Clone, Copy)]
enum PushAbsenceBoundary {
    Deleted,
    Unknown,
    Ambiguous,
}
impl PushAbsenceBoundary {
    fn error(self) -> SynchronizationError {
        match self {
            Self::Deleted => SynchronizationError::RemoteContextDeleted,
            Self::Unknown => SynchronizationError::HistoryUnknown,
            Self::Ambiguous => SynchronizationError::RecoveryRequired,
        }
    }
}
fn unverified_push_error(error: SynchronizationError) -> SynchronizationError {
    match error {
        SynchronizationError::ExternalChange => SynchronizationError::RecoveryRequired,
        SynchronizationError::Transport(error)
            if error.kind == SshTransportErrorKind::EndpointChanged =>
        {
            SynchronizationError::RecoveryRequired
        }
        other => other,
    }
}
fn decision(value: RemoteSafePointOutcome) -> Result<(), SynchronizationError> {
    match value {
        RemoteSafePointOutcome::Continue => Ok(()),
        _ => Err(SynchronizationError::Interrupted),
    }
}

// Worktree paths are derived here and never enter requests, records or errors.
fn local_target(
    root: &Path,
    primary: &str,
    target: &SynchronizationTarget,
) -> Result<git2::Repository, SynchronizationError> {
    let repository =
        git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
    let (path, branch) = match target {
        SynchronizationTarget::Primary => (root.to_owned(), primary.to_owned()),
        SynchronizationTarget::Context { kind, item_id } => {
            let context = ItemContext {
                root: root.to_owned(),
                kind: *kind,
                item_id: item_id.clone(),
                branch: format!("manyhands/{}/{}", authoring_kind_segment(kind), item_id),
                worktree: root.join(".manyhands/worktrees").join(item_id.to_string()),
            };
            validate_context_worktree(
                &repository,
                &context,
                true,
                RepositoryOperation::RepositorySnapshot,
            )
            .map_err(|_| SynchronizationError::TargetNotMaterialized)?;
            (context.worktree, context.branch)
        }
    };
    if repository.is_worktree() {
        return Err(SynchronizationError::TargetNotMaterialized);
    }
    if matches!(target, SynchronizationTarget::Primary)
        && repository
            .find_reference(&format!("refs/heads/{primary}"))
            .is_err()
    {
        return Err(SynchronizationError::PrimaryMissing);
    }
    let linked =
        git2::Repository::open(path).map_err(|_| SynchronizationError::TargetNotMaterialized)?;
    if linked
        .find_reference("HEAD")
        .ok()
        .and_then(|r| r.symbolic_target().map(str::to_owned))
        != Some(format!("refs/heads/{branch}"))
    {
        return Err(SynchronizationError::TargetNotMaterialized);
    }
    // A new synchronization must never adopt a foreign merge, rebase, or
    // cherry-pick merely because libgit2's state cache or index looks clean.
    let foreign_state = [
        "MERGE_HEAD",
        "REBASE_HEAD",
        "CHERRY_PICK_HEAD",
        "rebase-apply",
        "rebase-merge",
    ]
    .iter()
    .any(|name| linked.path().join(name).exists() || linked.commondir().join(name).exists());
    if foreign_state
        || linked.state() != git2::RepositoryState::Clean
        || linked
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .has_conflicts()
    {
        return Err(SynchronizationError::WorktreeConflicted {
            target: target.clone(),
        });
    }
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = linked
        .statuses(Some(&mut options))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if !statuses.is_empty() {
        return Err(SynchronizationError::WorktreeNotClean {
            target: target.clone(),
        });
    }
    drop(statuses);
    // A clean index/worktree must actually describe the checked-out commit.
    let commit = linked
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let tree = linked
        .index()
        .and_then(|mut i| i.write_tree())
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if tree != commit.tree_id() {
        return Err(SynchronizationError::RecoveryRequired);
    }
    drop(commit);
    Ok(linked)
}
fn local_oid(repository: &git2::Repository) -> Result<git2::Oid, SynchronizationError> {
    repository
        .head()
        .and_then(|h| h.peel_to_commit())
        .map(|c| c.id())
        .map_err(|_| SynchronizationError::RecoveryRequired)
}
fn advertised_oid(
    advertised: &[(String, git2::Oid)],
    name: &str,
) -> Result<Option<git2::Oid>, SynchronizationError> {
    let mut found = advertised.iter().filter(|(n, _)| n == name);
    let oid = found.next().map(|(_, oid)| *oid);
    if found.next().is_some() {
        return Err(SynchronizationError::ExternalChange);
    }
    Ok(oid)
}
fn target_ref(plan: &RemoteRefPlan, target: &SynchronizationTarget) -> RemoteRefTarget {
    match target {
        SynchronizationTarget::Primary => plan.primary().clone(),
        SynchronizationTarget::Context { kind, item_id } => plan.context(*kind, item_id),
    }
}
struct PreparedMergeEntry {
    entry: git2::IndexEntry,
    kind: git2::ObjectType,
    bytes: Vec<u8>,
}

struct PreparedMerge {
    entries: Vec<PreparedMergeEntry>,
}

enum MergePreparation {
    Clean(PreparedMerge),
    Conflict,
}

fn stage_name(stage: merge::IntegrationStage) -> SynchronizationStage {
    match stage {
        merge::IntegrationStage::Context => SynchronizationStage::Context,
        merge::IntegrationStage::Primary => SynchronizationStage::Primary,
    }
}

fn index_digest(tree: git2::Oid) -> [u8; 32] {
    *blake3::hash(tree.as_bytes()).as_bytes()
}

/// Merge preparation is deliberately done through a separately opened handle
/// with a high-priority memory ODB. The returned index has only regular stage-0
/// entries; generated blob bytes are imported and verified only while applying.
fn prepare_clean_merge(
    path: &Path,
    local: git2::Oid,
    incoming: git2::Oid,
) -> Result<MergePreparation, SynchronizationError> {
    let worker =
        git2::Repository::open(path).map_err(|_| SynchronizationError::RecoveryRequired)?;
    let odb = worker
        .odb()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let _mempack = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let local = worker
        .find_commit(local)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let incoming = worker
        .find_commit(incoming)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let index = worker
        .merge_commits(&local, &incoming, None)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if index.has_conflicts() {
        return Ok(MergePreparation::Conflict);
    }
    let entries = index
        .iter()
        .filter(|entry| entry.flags & 0x3000 == 0)
        .map(|entry| {
            let object = odb
                .read(entry.id)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            Ok(PreparedMergeEntry {
                entry,
                kind: object.kind(),
                bytes: object.data().to_vec(),
            })
        })
        .collect::<Result<Vec<_>, SynchronizationError>>()?;
    Ok(MergePreparation::Clean(PreparedMerge { entries }))
}

fn import_prepared_tree(
    repository: &git2::Repository,
    path: &Path,
    prepared: &PreparedMerge,
) -> Result<git2::Oid, SynchronizationError> {
    let _ = path; // The result bytes were captured while the worker mempack lived.
    let destination = repository
        .odb()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let mut index = git2::Index::new().map_err(|_| SynchronizationError::RecoveryRequired)?;
    for prepared_entry in &prepared.entries {
        if destination
            .write(prepared_entry.kind, &prepared_entry.bytes)
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            != prepared_entry.entry.id
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        index
            .add(&prepared_entry.entry)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
    }
    index
        .write_tree_to(repository)
        .map_err(|_| SynchronizationError::RecoveryRequired)
}

fn conflict_digest(repository: &mut git2::Repository) -> Result<[u8; 32], SynchronizationError> {
    let index = repository
        .index()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if !index.has_conflicts() {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let mut digest = blake3::Hasher::new();
    for conflict in index
        .conflicts()
        .map_err(|_| SynchronizationError::RecoveryRequired)?
    {
        let conflict = conflict.map_err(|_| SynchronizationError::RecoveryRequired)?;
        for entry in [conflict.ancestor, conflict.our, conflict.their]
            .into_iter()
            .flatten()
        {
            digest.update(&entry.mode.to_le_bytes());
            digest.update(entry.id.as_bytes());
            digest.update(&entry.path);
        }
    }
    repository
        .mergehead_foreach(|oid| {
            digest.update(oid.as_bytes());
            true
        })
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    Ok(*digest.finalize().as_bytes())
}

fn configuration_identity_digest(
    repository: &git2::Repository,
) -> Result<[u8; 32], SynchronizationError> {
    let config = repository
        .config()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let mut digest = blake3::Hasher::new();
    for key in ["user.name", "user.email"] {
        digest.update(key.as_bytes());
        if let Ok(value) = config.get_string(key) {
            digest.update(value.as_bytes());
        }
        digest.update(&[0]);
    }
    Ok(*digest.finalize().as_bytes())
}

fn effective_identity(
    repository: &git2::Repository,
) -> Result<Option<CommitIdentity>, SynchronizationError> {
    let config = repository
        .config()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let name = config.get_string("user.name").ok();
    let email = config.get_string("user.email").ok();
    Ok(match (name, email) {
        (Some(name), Some(email)) if !name.is_empty() && !email.is_empty() => {
            Some(CommitIdentity { name, email })
        }
        _ => None,
    })
}

fn committing_identity(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    repository: &git2::Repository,
    request: &SynchronizeRemoteRequest,
) -> Result<CommitIdentity, SynchronizationError> {
    if let Some(identity) = effective_identity(repository)? {
        return Ok(identity);
    }
    let confirmation = request.confirmed_identity.as_ref().ok_or_else(|| {
        SynchronizationError::IdentityRequired {
            target: request.target.clone(),
        }
    })?;
    if confirmation.identity.name.is_empty()
        || confirmation.identity.email.is_empty()
        || confirmation.identity.name.contains('\0')
        || confirmation.identity.email.contains('\0')
        || configuration_identity_digest(repository)? != confirmation.expected_configuration
    {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut input = blake3::Hasher::new();
    input.update(confirmation.confirmation_id.to_string().as_bytes());
    input.update(confirmation.identity.name.as_bytes());
    input.update(&[0]);
    input.update(confirmation.identity.email.as_bytes());
    service.prepare_synchronization_identity_confirmation(
        root,
        owner,
        &state::IdentityConfirmationIntent {
            confirmation_id: confirmation.confirmation_id,
            input_digest: *input.finalize().as_bytes(),
            configuration_digest: confirmation.expected_configuration,
        },
    )?;
    service.begin_synchronization_identity_confirmation_effect(
        root,
        owner,
        confirmation.confirmation_id,
    )?;
    let mut config = repository
        .config()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    config
        .set_str("user.name", &confirmation.identity.name)
        .and_then(|_| config.set_str("user.email", &confirmation.identity.email))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let observed = configuration_identity_digest(repository)?;
    service.observe_synchronization_identity_confirmation_effect(
        root,
        owner,
        confirmation.confirmation_id,
        observed,
    )?;
    Ok(confirmation.identity.clone())
}

#[derive(Clone, Copy)]
struct ReconciledCandidate {
    ordinal: u8,
    oid: git2::Oid,
    tree: git2::Oid,
}

/// Observe a candidate's local ref transition on restart, but keep the outer
/// synchronization envelope reconciling until Fetch is observed again. The
/// re-fetch is mandatory before any push/publication continuation.
fn reconcile_pending_candidate(
    service: &RepositoryService,
    root: &Path,
    primary_branch: &str,
    target: &SynchronizationTarget,
    owner: &RemoteReservation,
    evidence: &mut state::SynchronizationEvidence,
) -> Result<Option<ReconciledCandidate>, SynchronizationError> {
    let Some(step) = service.applying_synchronization_candidate(root, owner)? else {
        return Ok(None);
    };
    let candidate = step
        .candidate_oid
        .ok_or(SynchronizationError::RecoveryRequired)?;
    let repository = local_target(root, primary_branch, target)?;
    let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
    let head = local_oid(&repository)?;
    let commit = repository
        .find_commit(candidate)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let parents = [commit.parent_id(0).ok(), commit.parent_id(1).ok()];
    if commit.parent_count() != 2
        || parents != [Some(step.intent.local_oid), Some(step.intent.incoming_oid)]
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let tree = commit.tree_id();
    if head == step.intent.local_oid {
        let old = repository
            .find_commit(head)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if old.tree_id() != step.intent.baseline_tree_oid
            || index_digest(old.tree_id()) != step.intent.baseline_index_digest
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let branch = repository
            .find_reference("HEAD")
            .ok()
            .and_then(|reference| reference.symbolic_target().map(str::to_owned))
            .ok_or(SynchronizationError::RecoveryRequired)?;
        fast_forward(&repository, &branch, head, candidate)?;
    } else if head != candidate {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let observed = local_oid(&repository)?;
    if observed != candidate {
        return Err(SynchronizationError::RecoveryRequired);
    }
    service.observe_synchronization_integration_effect(
        root,
        owner,
        step.intent.ordinal,
        candidate,
        tree,
    )?;
    evidence.local_oid = Some(candidate);
    Ok(Some(ReconciledCandidate {
        ordinal: step.intent.ordinal,
        oid: candidate,
        tree,
    }))
}

/// Once the restarted Fetch has replaced the frozen tracking evidence, commit
/// the already-observed candidate into the outer synchronization envelope.
fn finalize_reconciled_candidate(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    candidate: ReconciledCandidate,
    evidence: &state::SynchronizationEvidence,
) -> Result<(), SynchronizationError> {
    decision(service.reconcile_synchronization_candidate_applied(
        root,
        owner,
        candidate.ordinal,
        candidate.oid,
        candidate.tree,
        evidence,
    )?)
}

#[derive(Clone, Copy)]
struct DivergenceInputs<'a> {
    root: &'a Path,
    primary_branch: &'a str,
    target: &'a SynchronizationTarget,
    request: &'a SynchronizeRemoteRequest,
    owner: &'a RemoteReservation,
    plan: &'a RemoteRefPlan,
    configuration: &'a super::observation::ObservationConfiguration,
    selected: &'a RemoteRefTarget,
    primary_tracking: Option<git2::Oid>,
    selected_tracking: Option<git2::Oid>,
    context: Option<git2::Oid>,
    primary: git2::Oid,
}

/// Re-observe every input that was frozen after Fetch before mutating the
/// destination. Preparation is intentionally outside the lease; it is never
/// authority to apply a merge after configuration, tracking, branch, index or
/// worktree state changes.
fn recheck_divergence_inputs(
    service: &RepositoryService,
    input: &DivergenceInputs<'_>,
    expected_local: git2::Oid,
) -> Result<(), SynchronizationError> {
    let target_repository = local_target(input.root, input.primary_branch, input.target)?;
    if local_oid(&target_repository)? != expected_local
        || service.observation_configuration(input.root, input.plan)? != *input.configuration
        || target_repository
            .refname_to_id(input.plan.primary().tracking_ref())
            .ok()
            != input.primary_tracking
        || target_repository
            .refname_to_id(input.selected.tracking_ref())
            .ok()
            != input.selected_tracking
    {
        return Err(SynchronizationError::ExternalChange);
    }
    Ok(())
}

fn integrate_divergence(
    service: &RepositoryService,
    input: DivergenceInputs<'_>,
) -> Result<git2::Oid, SynchronizationError> {
    let DivergenceInputs {
        root,
        primary_branch,
        target,
        request,
        owner,
        context,
        primary,
        ..
    } = input;
    let target_repository = local_target(root, primary_branch, target)?;
    let path = target_repository
        .workdir()
        .ok_or(SynchronizationError::RecoveryRequired)?
        .to_owned();
    let mut local = local_oid(&target_repository)?;
    let stages = merge::integration_stages(target, context.is_some());
    for (ordinal, stage) in stages.into_iter().enumerate() {
        let incoming = match stage {
            merge::IntegrationStage::Context => {
                context.ok_or(SynchronizationError::RecoveryRequired)?
            }
            merge::IntegrationStage::Primary => primary,
        };
        let classification = merge::classify_integration(
            local,
            incoming,
            |older, newer| target_repository.graph_descendant_of(newer, older),
            |left, right| {
                target_repository
                    .merge_base(left, right)
                    .map(|_| true)
                    .or_else(|error| {
                        if error.code() == git2::ErrorCode::NotFound {
                            Ok(false)
                        } else {
                            Err(error)
                        }
                    })
            },
        )
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let local_commit = target_repository
            .find_commit(local)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let intent = state::IntegrationStepIntent {
            ordinal: ordinal as u8,
            stage,
            local_oid: local,
            incoming_oid: incoming,
            baseline_tree_oid: local_commit.tree_id(),
            baseline_index_digest: index_digest(local_commit.tree_id()),
        };
        drop(local_commit);
        // Persist the exact ordered parents before even preparing a candidate;
        // a restart must never synthesize another candidate for this stage.
        service.prepare_synchronization_integration(root, owner, &intent)?;
        let prepared = if classification == merge::IntegrationDisposition::MergeRequired {
            Some(prepare_clean_merge(&path, local, incoming)?)
        } else {
            None
        };
        match classification {
            merge::IntegrationDisposition::Equal
            | merge::IntegrationDisposition::IncomingAlreadyIntegrated => {
                service.begin_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
                    None,
                )?;
                let tree = target_repository
                    .find_commit(local)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?
                    .tree_id();
                service.observe_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
                    local,
                    tree,
                )?;
            }
            merge::IntegrationDisposition::FastForward => {
                let _lease = repository_lease(
                    &target_repository,
                    root,
                    RepositoryOperation::RepositorySnapshot,
                )?;
                #[cfg(test)]
                super::observation_tests::checkpoint(RemoteOperationSafePoint::BeforeLocalMutation);
                recheck_divergence_inputs(service, &input, local)?;
                if target_repository.find_commit(incoming).is_err() {
                    return Err(SynchronizationError::ExternalChange);
                }
                service.begin_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
                    None,
                )?;
                let branch = target_repository
                    .find_reference("HEAD")
                    .ok()
                    .and_then(|head| head.symbolic_target().map(str::to_owned))
                    .ok_or(SynchronizationError::ExternalChange)?;
                fast_forward(&target_repository, &branch, local, incoming)?;
                let tree = target_repository
                    .find_commit(incoming)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?
                    .tree_id();
                service.observe_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
                    incoming,
                    tree,
                )?;
                local = incoming;
            }
            merge::IntegrationDisposition::MergeRequired => {
                let prepared = match prepared.ok_or(SynchronizationError::RecoveryRequired)? {
                    MergePreparation::Clean(prepared) => Some(prepared),
                    MergePreparation::Conflict => None,
                };
                let mut repository = git2::Repository::open(&path)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?;
                let _lease =
                    repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
                #[cfg(test)]
                super::observation_tests::checkpoint(RemoteOperationSafePoint::BeforeLocalMutation);
                recheck_divergence_inputs(service, &input, local)?;
                if repository.find_commit(incoming).is_err() {
                    return Err(SynchronizationError::ExternalChange);
                }
                if let Some(prepared) = prepared {
                    let identity = committing_identity(service, root, owner, &repository, request)?;
                    let tree = import_prepared_tree(&repository, &path, &prepared)?;
                    let local_commit = repository
                        .find_commit(local)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let incoming_commit = repository
                        .find_commit(incoming)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let tree = repository
                        .find_tree(tree)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let signature = git2::Signature::now(&identity.name, &identity.email)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let subject = match stage {
                        merge::IntegrationStage::Context => {
                            format!("Merge remote context {}", request.operation_id)
                        }
                        merge::IntegrationStage::Primary
                            if matches!(target, SynchronizationTarget::Primary) =>
                        {
                            "Merge remote primary".into()
                        }
                        merge::IntegrationStage::Primary => match target {
                            SynchronizationTarget::Context { kind, item_id } => format!(
                                "Merge primary into {} {item_id}",
                                authoring_kind_segment(kind)
                            ),
                            SynchronizationTarget::Primary => unreachable!(),
                        },
                    };
                    let candidate = repository
                        .commit(
                            None,
                            &signature,
                            &signature,
                            &subject,
                            &tree,
                            &[&local_commit, &incoming_commit],
                        )
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    service.begin_synchronization_integration_effect(
                        root,
                        owner,
                        ordinal as u8,
                        Some(candidate),
                    )?;
                    let branch = repository
                        .find_reference("HEAD")
                        .ok()
                        .and_then(|head| head.symbolic_target().map(str::to_owned))
                        .ok_or(SynchronizationError::ExternalChange)?;
                    fast_forward(&repository, &branch, local, candidate)?;
                    service.observe_synchronization_integration_effect(
                        root,
                        owner,
                        ordinal as u8,
                        candidate,
                        tree.id(),
                    )?;
                    local = candidate;
                } else {
                    service.begin_synchronization_integration_effect(
                        root,
                        owner,
                        ordinal as u8,
                        None,
                    )?;
                    let annotated = repository
                        .find_annotated_commit(incoming)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let mut checkout = git2::build::CheckoutBuilder::new();
                    checkout.safe().overwrite_ignored(false);
                    repository
                        .merge(&[&annotated], None, Some(&mut checkout))
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    drop(annotated);
                    let fingerprint = conflict_digest(&mut repository)?;
                    service.release_synchronization_conflict(
                        root,
                        owner,
                        ordinal as u8,
                        fingerprint,
                    )?;
                    return Err(SynchronizationError::ConflictPending {
                        target: target.clone(),
                        operation_id: request.operation_id,
                        stage: stage_name(stage),
                    });
                }
            }
        }
    }
    Ok(local)
}

fn graph_plan(
    repository: &git2::Repository,
    target: &SynchronizationTarget,
    local: git2::Oid,
    primary: Option<git2::Oid>,
    context: Option<git2::Oid>,
    publication: RemotePublicationEvidence,
) -> Result<refs::CleanIntegrationPlan, SynchronizationError> {
    refs::plan_clean_integration(target, local, primary, context, publication, |a, b| {
        repository.graph_descendant_of(b, a)
    })
    .map_err(|error| match error {
        refs::CleanIntegrationError::PrimaryMissing => SynchronizationError::PrimaryMissing,
        refs::CleanIntegrationError::RemoteContextDeleted => {
            SynchronizationError::RemoteContextDeleted
        }
        refs::CleanIntegrationError::HistoryUnknown => SynchronizationError::HistoryUnknown,
        refs::CleanIntegrationError::MergeRequired => SynchronizationError::MergeRequired {
            target: target.clone(),
        },
        refs::CleanIntegrationError::Ancestry(_) => SynchronizationError::RecoveryRequired,
    })
}
impl RepositoryService {
    fn synchronization_boundary(
        &self,
        root: &Path,
        owner: &RemoteReservation,
    ) -> Result<(), SynchronizationError> {
        decision(self.check_synchronization_requests(root, owner)?)
    }
    fn synchronization_point(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        point: RemoteOperationSafePoint,
    ) -> Result<(), SynchronizationError> {
        #[cfg(test)]
        super::observation_tests::checkpoint(point);
        decision(self.remote_safe_point(root, owner, point)?)
    }
    fn inspect_synchronization_local(
        &self,
        root: &Path,
        primary: &str,
        target: &SynchronizationTarget,
    ) -> Result<git2::Oid, SynchronizationError> {
        let repository =
            git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        local_oid(&local_target(root, primary, target)?)
    }
    fn synchronization_configuration_matches(
        &self,
        root: &Path,
        plan: &RemoteRefPlan,
        expected: &super::observation::ObservationConfiguration,
    ) -> Result<bool, SynchronizationError> {
        let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
            return Ok(false);
        };
        let actual = config.publication_remote.as_ref().and_then(|remote| {
            RemoteRefPlan::from_configuration(remote, &config.primary_branch).ok()
        });
        Ok(actual.as_ref() == Some(plan)
            && self.observation_configuration(root, plan)? == *expected)
    }

    /// Bind every authenticated scope to one action snapshot, including after
    /// credentials and safe-point hooks. The adapter pins its actual handle too.
    fn with_synchronization_remote<P: SessionCredentialProvider, T>(
        &self,
        request: VerifySshTransportRequest,
        session: &mut SessionCredentials<P>,
        owner: &RemoteReservation,
        plan: &RemoteRefPlan,
        expected: &super::observation::ObservationConfiguration,
        operation: impl FnOnce(
            &mut crate::repository::transport::AuthenticatedSshRemote<'_, '_>,
        ) -> Result<T, SshTransportError>,
    ) -> Result<T, SynchronizationError> {
        self.synchronization_boundary(&request.root, owner)?;
        if !self.synchronization_configuration_matches(&request.root, plan, expected)? {
            return Err(SynchronizationError::ExternalChange);
        }
        let expectation = expected
            .transport_expectation(request.direction)
            .ok_or(SynchronizationError::RecoveryRequired)?;
        let root = request.root.clone();
        #[cfg(test)]
        crate::repository::transport::operation_tests::checkpoint(
            crate::repository::transport::operation_tests::Checkpoint::ActionSnapshotChecked,
        );
        self.with_authenticated_remote_expected(request, session, &expectation, |remote| {
            if let Err(error) = self.synchronization_boundary(&root, owner) {
                return Ok(Err(error));
            }
            match self.synchronization_configuration_matches(&root, plan, expected) {
                Ok(true) => {}
                Ok(false) => return Ok(Err(SynchronizationError::ExternalChange)),
                Err(error) => return Ok(Err(error)),
            }
            remote.require_action_expectation(&expectation)?;
            let result = operation(remote)?;
            remote.require_action_expectation(&expectation)?;
            if let Err(error) = self.synchronization_boundary(&root, owner) {
                return Ok(Err(error));
            }
            match self.synchronization_configuration_matches(&root, plan, expected) {
                Ok(true) => {}
                Ok(false) => return Ok(Err(SynchronizationError::ExternalChange)),
                Err(error) => return Ok(Err(error)),
            }
            Ok(Ok(result))
        })?
    }

    fn inspect_conflict_target(
        root: &Path,
        target: &SynchronizationTarget,
    ) -> Result<git2::Repository, SynchronizationError> {
        let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let (path, branch) = match target {
            SynchronizationTarget::Primary => (root.to_owned(), config.primary_branch),
            SynchronizationTarget::Context { kind, item_id } => (
                root.join(".manyhands/worktrees").join(item_id.to_string()),
                format!("manyhands/{}/{}", authoring_kind_segment(kind), item_id),
            ),
        };
        let repository =
            git2::Repository::open(path).map_err(|_| SynchronizationError::RecoveryRequired)?;
        if repository
            .find_reference("HEAD")
            .ok()
            .and_then(|head| head.symbolic_target().map(str::to_owned))
            != Some(format!("refs/heads/{branch}"))
        {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(repository)
    }

    fn conflict_configuration(root: &Path) -> Result<[u8; 32], SynchronizationError> {
        let bytes = std::fs::read(root.join(".manyhands/config.toml"))
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        Ok(*blake3::hash(&bytes).as_bytes())
    }

    /// Inspect only the exact conflict retained by this synchronization. This
    /// reads actual Git state and never treats a foreign merge as owned.
    pub fn inspect_synchronization_recovery(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<SynchronizationConflictInspection, SynchronizationError> {
        let (_, root) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
        let record = state::with_transaction(self, &root, |tx, id| {
            state::read_operation(tx, id, operation_id)
        })?
        .ok_or(SynchronizationError::RecoveryRequired)?;
        let target = match record.target.action() {
            RemoteOperationAction::SynchronizePrimary => SynchronizationTarget::Primary,
            RemoteOperationAction::SynchronizeContext => {
                let (kind, item_id) = record.target.item().ok_or_else(state::recovery_required)?;
                SynchronizationTarget::Context {
                    kind,
                    item_id: item_id.clone(),
                }
            }
            _ => return Err(SynchronizationError::RecoveryRequired),
        };
        // State reads are bounded to this operation; exactly one retained
        // conflict-pending child is required for an inspectable conflict.
        let step = state::with_transaction(self, &root, |tx, _| {
            let mut pending = None;
            for ordinal in 0..=1 {
                if let Some(step) = state::integration_step(tx, record.id, ordinal)?
                    && step.phase == state::IntegrationStepPhase::ConflictPending
                {
                    if pending.is_some() {
                        return Err(state::recovery_required());
                    }
                    pending = Some((ordinal, step));
                }
            }
            pending.ok_or_else(state::recovery_required)
        })?;
        let (ordinal, step) = step;
        let mut repository = Self::inspect_conflict_target(&root, &target)?;
        let _lease = repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
        let head = local_oid(&repository)?;
        let fingerprint = conflict_digest(&mut repository)?;
        if head != step.intent.local_oid
            || fingerprint
                != step
                    .conflict_digest
                    .ok_or(SynchronizationError::RecoveryRequired)?
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let configuration = Self::conflict_configuration(&root)?;
        let observation = merge::ConflictObservation {
            operation_id,
            ordinal,
            fingerprint,
            head,
            configuration,
            root: root.clone(),
        };
        let index = repository
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let mut paths = Vec::new();
        for (entry_ordinal, conflict) in index
            .conflicts()
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .enumerate()
        {
            let conflict = conflict.map_err(|_| SynchronizationError::RecoveryRequired)?;
            let entry = conflict
                .our
                .as_ref()
                .or(conflict.their.as_ref())
                .or(conflict.ancestor.as_ref())
                .ok_or(SynchronizationError::RecoveryRequired)?;
            paths.push(SynchronizationConflictPath {
                token: merge::ConflictPathToken {
                    observation: observation.clone(),
                    ordinal: entry_ordinal as u32,
                    path: entry.path.clone(),
                    base: conflict.ancestor.as_ref().map(|entry| entry.id),
                    base_mode: conflict.ancestor.as_ref().map(|entry| entry.mode),
                    local: conflict.our.as_ref().map(|entry| entry.id),
                    local_mode: conflict.our.as_ref().map(|entry| entry.mode),
                    incoming: conflict.their.as_ref().map(|entry| entry.id),
                    incoming_mode: conflict.their.as_ref().map(|entry| entry.mode),
                },
                eligibility: merge::ConflictEligibility::ExternalResolutionRequired,
            });
        }
        Ok(SynchronizationConflictInspection {
            operation_id,
            target,
            stage: stage_name(step.intent.stage),
            local_parent: step.intent.local_oid,
            incoming_parent: step.intent.incoming_oid,
            observation,
            paths,
        })
    }

    /// Explicitly read the three Git index sides for an inspection-issued token.
    /// Every optimistic precondition is re-observed first; no content is saved.
    pub fn read_synchronization_conflict(
        &self,
        token: &merge::ConflictPathToken,
    ) -> Result<EphemeralSynchronizationConflictSides, SynchronizationError> {
        let inspection = self.inspect_synchronization_recovery(
            &token.observation.root,
            token.observation.operation_id,
        )?;
        if inspection.observation != token.observation
            || !inspection.paths.iter().any(|path| path.token == *token)
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let safe_path = std::str::from_utf8(&token.path).ok().filter(|path| {
            !path.is_empty()
                && std::path::Path::new(path)
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)))
        });
        if safe_path.is_none()
            || [token.base_mode, token.local_mode, token.incoming_mode]
                .into_iter()
                .flatten()
                .any(|mode| mode != 0o100644)
        {
            return Err(SynchronizationError::ExternalResolutionRequired {
                target: inspection.target,
                operation_id: token.observation.operation_id,
            });
        }
        let mut repository =
            Self::inspect_conflict_target(&token.observation.root, &inspection.target)?;
        let _lease = repository_lease(
            &repository,
            &token.observation.root,
            RepositoryOperation::RepositorySnapshot,
        )?;
        if local_oid(&repository)? != token.observation.head
            || Self::conflict_configuration(&token.observation.root)?
                != token.observation.configuration
            || conflict_digest(&mut repository)? != token.observation.fingerprint
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let read = |oid: Option<git2::Oid>| -> Result<Option<merge::RedactedConflictBytes>, SynchronizationError> {
            oid.map(|oid| {
                let blob = repository.find_blob(oid).map_err(|_| SynchronizationError::RecoveryRequired)?;
                if blob.is_binary() {
                    return Err(SynchronizationError::ExternalResolutionRequired {
                        target: inspection.target.clone(),
                        operation_id: token.observation.operation_id,
                    });
                }
                Ok(merge::RedactedConflictBytes::from_bytes(blob.content().to_vec()))
            }).transpose()
        };
        // Current worktree bytes are deliberately unavailable: a path-based
        // read could follow a post-inspection symlink. The three immutable index
        // stages above are the complete safe inspection surface.
        let sides = EphemeralSynchronizationConflictSides {
            base: read(token.base)?,
            local: read(token.local)?,
            incoming: read(token.incoming)?,
            current: None,
        };
        if [&sides.base, &sides.local, &sides.incoming]
            .into_iter()
            .flatten()
            .any(|bytes| std::str::from_utf8(bytes.bytes()).is_err())
        {
            return Err(SynchronizationError::ExternalResolutionRequired {
                target: inspection.target,
                operation_id: token.observation.operation_id,
            });
        }
        Ok(sides)
    }

    /// Synchronize one existing clean target; every publication is independently
    /// observed, and an authoritative replay performs only its index handoff.
    pub fn synchronize_remote<P: SessionCredentialProvider>(
        &self,
        mut request: SynchronizeRemoteRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let (_, root) =
            canonical_repository_root(&request.root, RepositoryOperation::RepositorySnapshot)?;
        request.root = root.clone();
        let existing = state::with_transaction(self, &root, |tx, id| {
            let other_root: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",rusqlite::params![request.operation_id.to_string(),id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
            if other_root {
                return Err(identity_error());
            }
            state::read_operation(tx, id, request.operation_id)
        })?;
        // Historical identity is independent of current publication settings.
        // Check it before every replay/restart or live local-only selection.
        if let Some(record) = &existing {
            validate_record_target(&record.target, &request.target)?;
        }
        if let Some(record) = &existing
            && record.authority.is_some()
        {
            let RemoteReservationOutcome::Replay(inspection) =
                self.reserve_remote_operation(&root, request.operation_id, &record.target)?
            else {
                return Err(SynchronizationError::RecoveryRequired);
            };
            let outcome = authority_outcome(
                &request.target,
                inspection
                    .authority()
                    .ok_or(SynchronizationError::RecoveryRequired)?,
            );
            return Ok(if inspection.index_pending() {
                self.synchronization_refresh(&request, Some(&record.target), outcome)
            } else {
                SynchronizationResult::Complete(outcome)
            });
        }
        if let Some(record) = &existing
            && record.phase == RemoteOperationPhase::Cancelled
        {
            self.reserve_remote_operation(&root, request.operation_id, &record.target)?;
            return Err(SynchronizationError::Interrupted);
        }
        if let Some(record) = &existing
            && !request.restart
        {
            self.reserve_remote_operation(&root, request.operation_id, &record.target)?;
            return Err(SynchronizationError::RecoveryRequired);
        }
        if existing.is_none()
            && let Some(oid) = self.local_synchronization_replay(&request)?
        {
            return Ok(self.synchronization_refresh(
                &request,
                None,
                SynchronizationOutcome::PublishPending {
                    target: request.target.clone(),
                    local_oid: oid,
                    reason: PublishPendingReason::NoPublicationRemote,
                },
            ));
        }
        let ConfigurationInspection::Valid(config) = read_configuration(&root)? else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let Some(remote_name) = config.publication_remote.as_ref() else {
            if existing.is_some() {
                return Err(SynchronizationError::RecoveryRequired);
            }
            let oid = self.bind_local_synchronization(&request, &config.primary_branch)?;
            return Ok(self.synchronization_refresh(
                &request,
                None,
                SynchronizationOutcome::PublishPending {
                    target: request.target.clone(),
                    local_oid: oid,
                    reason: PublishPendingReason::NoPublicationRemote,
                },
            ));
        };
        let plan = RemoteRefPlan::from_configuration(remote_name, &config.primary_branch)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let target = request.target.operation_target(&plan);
        // Inspect same-ID rows before reconfiguring or preflighting mutable Git.
        // The reservation controller validates root, target and local ID coexistence.
        if existing.is_none() {
            self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        }
        // Freeze exactly the snapshot that establishes this generation. A same-ID
        // restart reconciles persisted endpoint identity BEFORE receiving ownership.
        let configuration = self.observation_configuration(&root, &plan)?;
        state::with_transaction(self, &root, |tx, id| {
            state::configure_endpoints(tx, id, &plan, &configuration.endpoint_digest())
        })?;
        let reservation = self.reserve_remote_operation(&root, request.operation_id, &target)?;
        let mut prior = None;
        let reservation = match reservation {
            RemoteReservationOutcome::Replay(record) => {
                if let Some(authority) = record.authority() {
                    let outcome = authority_outcome(&request.target, authority);
                    return Ok(if record.index_pending() {
                        self.synchronization_refresh(&request, Some(&target), outcome)
                    } else {
                        SynchronizationResult::Complete(outcome)
                    });
                }
                if record.phase() == RemoteOperationPhase::Cancelled {
                    return Err(SynchronizationError::Interrupted);
                }
                if !request.restart {
                    return Err(SynchronizationError::RecoveryRequired);
                }
                prior = Some(record);
                self.restart_remote_synchronization(&root, request.operation_id, &target)
                    .map_err(|error| {
                        if error.kind == RepositoryErrorKind::RecoveryRequired {
                            SynchronizationError::RecoveryRequired
                        } else {
                            error.into()
                        }
                    })?
            }
            other => other,
        };
        let owner = match reservation {
            RemoteReservationOutcome::Reserved(owner) => owner,
            RemoteReservationOutcome::Busy => return Err(SynchronizationError::Busy),
            RemoteReservationOutcome::PollYielding => {
                return Err(SynchronizationError::PollYielding);
            }
            RemoteReservationOutcome::Replay(_) => {
                return Err(SynchronizationError::RecoveryRequired);
            }
        };
        if !self.synchronization_configuration_matches(&root, &plan, &configuration)? {
            return Err(SynchronizationError::ExternalChange);
        }
        let initial = self
            .inspect_synchronization_local(&root, &config.primary_branch, &request.target)
            .map_err(|error| {
                if prior.as_ref().is_some_and(|record| {
                    matches!(
                        record.sync_checkpoint(),
                        Some(
                            Checkpoint::LocalPrepared
                                | Checkpoint::LocalFastForwarded
                                | Checkpoint::PushPrepared
                                | Checkpoint::PushReturned
                                | Checkpoint::PushVerified
                        )
                    )
                }) {
                    SynchronizationError::RecoveryRequired
                } else {
                    error
                }
            })?;
        let mut evidence = prior
            .as_ref()
            .map(|r| r.sync_evidence().clone())
            .unwrap_or(Evidence {
                expected_oid: Some(initial),
                local_oid: Some(initial),
                ..Evidence::default()
            });
        let pending_candidate = if prior.is_some() {
            reconcile_pending_candidate(
                self,
                &root,
                &config.primary_branch,
                &request.target,
                &owner,
                &mut evidence,
            )?
        } else {
            None
        };
        let candidate_reconciled = pending_candidate.is_some();
        if prior.is_none() {
            decision(self.checkpoint_synchronization(
                &root,
                &owner,
                Checkpoint::FetchPrepared,
                &evidence,
            )?)?;
        }
        self.synchronization_point(&root, &owner, RemoteOperationSafePoint::BeforeFetch)?;
        let transport_request = |direction| VerifySshTransportRequest {
            root: root.clone(),
            direction,
            approval: request.approval.clone(),
        };
        let before = self.with_synchronization_remote(
            transport_request(SshDirection::Fetch),
            session,
            &owner,
            &plan,
            &configuration,
            |r| r.fresh_advertisement(),
        )?;
        self.synchronization_boundary(&root, &owner)?;
        self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        self.with_synchronization_remote(
            transport_request(SshDirection::Fetch),
            session,
            &owner,
            &plan,
            &configuration,
            |r| r.fetch_exact(&plan, &request.target),
        )?;
        self.synchronization_boundary(&root, &owner)?;
        self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        let after = self.with_synchronization_remote(
            transport_request(SshDirection::Fetch),
            session,
            &owner,
            &plan,
            &configuration,
            |r| r.fresh_advertisement(),
        )?;
        self.synchronization_boundary(&root, &owner)?;
        self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        let selected = target_ref(&plan, &request.target);
        let primary = advertised_oid(&after, plan.primary().remote_ref())?;
        let context = if matches!(request.target, SynchronizationTarget::Context { .. }) {
            advertised_oid(&after, selected.remote_ref())?
        } else {
            None
        };
        let repository =
            git2::Repository::open(&root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let mut changed = false;
        for relevant in [plan.primary(), &selected] {
            let final_oid = advertised_oid(&after, relevant.remote_ref())?;
            if advertised_oid(&before, relevant.remote_ref())? != final_oid
                || final_oid.is_some_and(|oid| {
                    repository.refname_to_id(relevant.tracking_ref()).ok() != Some(oid)
                })
            {
                changed = true;
            }
        }
        let observations = after
            .iter()
            .filter_map(|(name, oid)| {
                RemoteRefObservation::from_advertisement(
                    &plan,
                    name,
                    *oid,
                    plan.tracking_ref_for(name)
                        .and_then(|name| repository.refname_to_id(&name).ok()),
                )
            })
            .collect::<Vec<_>>();
        if self.observation_configuration(&root, &plan)? != configuration {
            return Err(SynchronizationError::ExternalChange);
        }
        decision(reservation::commit_observation_batch(
            self,
            &root,
            &owner,
            &plan,
            &observations,
            time::OffsetDateTime::now_utc().unix_timestamp().max(0),
        )?)?;
        self.synchronization_point(&root, &owner, RemoteOperationSafePoint::AfterFetch)?;
        if changed {
            return Err(SynchronizationError::ExternalChange);
        }
        evidence.primary_tracking_oid = primary;
        evidence.tracking_oid = if matches!(request.target, SynchronizationTarget::Primary) {
            primary
        } else {
            context
        };
        let publication = match &request.target {
            SynchronizationTarget::Primary => RemotePublicationEvidence::HistoryUnknown,
            SynchronizationTarget::Context { kind, item_id } => self
                .remote_snapshot(&root)?
                .publication_evidence_for(*kind, item_id),
        };
        // Fetch and Push history are direction-specific. A verified publication
        // at this endpoint must not be recreated after deletion, even when its
        // Fetch advertisement has always been absent (distinct pushurl).
        if primary.is_none() {
            return Err(SynchronizationError::PrimaryMissing);
        }
        let mut resumed = None;
        if let Some(candidate) = pending_candidate {
            finalize_reconciled_candidate(self, &root, &owner, candidate, &evidence)?;
            let record = state::with_transaction(self, &root, |tx, id| {
                state::read_operation(tx, id, request.operation_id)
            })?
            .ok_or(SynchronizationError::RecoveryRequired)?;
            evidence = record.sync_evidence;
            resumed = record.sync_checkpoint;
        }
        let push_absence_boundary =
            self.synchronization_push_absence_boundary(&root, &owner, prior.is_some())?;
        if let Some(boundary) = push_absence_boundary {
            let actual =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            let observed = self.synchronization_push_observation(
                &request,
                session,
                &owner,
                &plan,
                &configuration,
                actual,
            )?;
            if self.observation_configuration(&root, &plan)? != configuration {
                return Err(SynchronizationError::ExternalChange);
            }
            if observed.is_none() {
                return Err(boundary.error());
            }
        }
        if prior.is_some() && !candidate_reconciled {
            let actual =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            let push = self.synchronization_push_observation(
                &request,
                session,
                &owner,
                &plan,
                &configuration,
                actual,
            )?;
            let ancestor = push.is_some_and(|oid| {
                oid != actual && repository.graph_descendant_of(actual, oid).unwrap_or(false)
            });
            decision(
                self.reconcile_synchronization(&root, &owner, actual, actual, push, ancestor)?,
            )?;
            let record = state::with_transaction(self, &root, |tx, id| {
                state::read_operation(tx, id, request.operation_id)
            })?
            .ok_or(SynchronizationError::RecoveryRequired)?;
            evidence = record.sync_evidence;
            evidence.primary_tracking_oid = primary;
            evidence.tracking_oid = if matches!(request.target, SynchronizationTarget::Primary) {
                primary
            } else {
                context
            };
            resumed = record.sync_checkpoint;
        }
        // Preserve Cycle 05's all-clean virtual plan. Only its deliberate
        // divergence result enters the ordered, durable merge path.
        let divergence = {
            let local =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            matches!(
                graph_plan(
                    &repository,
                    &request.target,
                    local,
                    primary,
                    context,
                    publication,
                ),
                Err(SynchronizationError::MergeRequired { .. })
            )
        };
        if divergence {
            let merged = integrate_divergence(
                self,
                DivergenceInputs {
                    root: &root,
                    primary_branch: &config.primary_branch,
                    target: &request.target,
                    request: &request,
                    owner: &owner,
                    plan: &plan,
                    configuration: &configuration,
                    selected: &selected,
                    primary_tracking: evidence.primary_tracking_oid,
                    selected_tracking: evidence.tracking_oid,
                    context,
                    primary: primary.ok_or(SynchronizationError::PrimaryMissing)?,
                },
            )?;
            evidence.local_oid = Some(merged);
            decision(self.checkpoint_synchronization_merge_applied(&root, &owner, &evidence)?)?;
        } else {
            // Compute the entire virtual graph under one short lease before any write.
            {
                let _lease =
                    repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
                if self.observation_configuration(&root, &plan)? != configuration {
                    return Err(SynchronizationError::ExternalChange);
                }
                for (reference, oid) in [
                    (plan.primary(), primary),
                    (&selected, evidence.tracking_oid),
                ] {
                    if oid.is_some_and(|oid| {
                        repository.refname_to_id(reference.tracking_ref()).ok() != Some(oid)
                    }) {
                        return Err(SynchronizationError::ExternalChange);
                    }
                }
                let linked = local_target(&root, &config.primary_branch, &request.target)?;
                let local = local_oid(&linked)?;
                if evidence.expected_oid != Some(local)
                    && !matches!(
                        resumed,
                        Some(
                            Checkpoint::LocalFastForwarded
                                | Checkpoint::PushPrepared
                                | Checkpoint::PushVerified
                        )
                    )
                {
                    return Err(SynchronizationError::ExternalChange);
                }
                let integration = graph_plan(
                    &repository,
                    &request.target,
                    local,
                    primary,
                    context,
                    publication,
                )?;
                if resumed.is_some_and(|c| {
                    matches!(
                        c,
                        Checkpoint::LocalFastForwarded
                            | Checkpoint::PushPrepared
                            | Checkpoint::PushVerified
                    )
                }) && integration.final_oid != local
                {
                    return Err(SynchronizationError::RecoveryRequired);
                }
                evidence.local_oid = Some(integration.final_oid);
                if integration.local_update {
                    decision(self.checkpoint_synchronization(
                        &root,
                        &owner,
                        Checkpoint::LocalPrepared,
                        &evidence,
                    )?)?;
                    self.synchronization_point(
                        &root,
                        &owner,
                        RemoteOperationSafePoint::BeforeLocalUpdate,
                    )?;
                    // Reopen at the actual mutation boundary after durable intent.
                    let fresh = local_target(&root, &config.primary_branch, &request.target)?;
                    if local_oid(&fresh)? != local {
                        return Err(SynchronizationError::ExternalChange);
                    }
                    fast_forward(&fresh, selected.remote_ref(), local, integration.final_oid)?;
                    local_target(&root, &config.primary_branch, &request.target)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    if local_oid(&fresh)? != integration.final_oid {
                        return Err(SynchronizationError::RecoveryRequired);
                    }
                    decision(self.checkpoint_synchronization(
                        &root,
                        &owner,
                        Checkpoint::LocalFastForwarded,
                        &evidence,
                    )?)?;
                    self.synchronization_point(
                        &root,
                        &owner,
                        RemoteOperationSafePoint::AfterLocalUpdate,
                    )?;
                }
            }
        }
        self.synchronization_boundary(&root, &owner)?;
        let candidate = evidence
            .local_oid
            .ok_or(SynchronizationError::RecoveryRequired)?;
        let push = self.synchronization_push_observation(
            &request,
            session,
            &owner,
            &plan,
            &configuration,
            candidate,
        )?;
        if self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?
            != candidate
        {
            return Err(SynchronizationError::ExternalChange);
        }
        if self.observation_configuration(&root, &plan)? != configuration {
            return Err(SynchronizationError::ExternalChange);
        }
        if push.is_none()
            && let Some(boundary) = push_absence_boundary
        {
            return Err(boundary.error());
        }
        evidence.push_oid = Some(candidate);
        evidence.push_advertised_oid = push;
        let authority = if push == Some(candidate) {
            if resumed != Some(Checkpoint::PushVerified) {
                decision(self.checkpoint_synchronization(
                    &root,
                    &owner,
                    Checkpoint::PushVerified,
                    &evidence,
                )?)?;
            }
            if prior.as_ref().is_some_and(|r| {
                r.sync_checkpoint().is_some() && r.sync_evidence().push_oid.is_some()
            }) {
                Authority::Published(candidate)
            } else {
                Authority::AlreadyCurrent(candidate)
            }
        } else {
            if resumed == Some(Checkpoint::PushVerified) {
                return Err(SynchronizationError::RecoveryRequired);
            }
            if push.is_some_and(|oid| {
                !repository
                    .graph_descendant_of(candidate, oid)
                    .unwrap_or(false)
            }) {
                return Err(SynchronizationError::PushRejected);
            }
            if resumed != Some(Checkpoint::PushPrepared) {
                decision(self.checkpoint_synchronization(
                    &root,
                    &owner,
                    Checkpoint::PushPrepared,
                    &evidence,
                )?)?;
            }
            self.synchronization_point(&root, &owner, RemoteOperationSafePoint::BeforePush)?;
            if self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?
                != candidate
            {
                return Err(SynchronizationError::ExternalChange);
            }
            self.with_synchronization_remote(
                transport_request(SshDirection::Push),
                session,
                &owner,
                &plan,
                &configuration,
                |r| r.push_exact(&plan, &request.target),
            )
            .map_err(unverified_push_error)?;
            decision(self.checkpoint_synchronization(
                &root,
                &owner,
                Checkpoint::PushReturned,
                &evidence,
            )?)?;
            self.synchronization_point(&root, &owner, RemoteOperationSafePoint::AfterPushReturn)?;
            let verified = self
                .with_synchronization_remote(
                    transport_request(SshDirection::Push),
                    session,
                    &owner,
                    &plan,
                    &configuration,
                    |r| r.fresh_advertisement(),
                )
                .map_err(unverified_push_error)?;
            self.synchronization_boundary(&root, &owner)?;
            if self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?
                != candidate
            {
                return Err(SynchronizationError::RecoveryRequired);
            }
            evidence.push_advertised_oid = advertised_oid(&verified, selected.remote_ref())?;
            if evidence.push_advertised_oid != Some(candidate) {
                return Err(SynchronizationError::RecoveryRequired);
            }
            if !self.synchronization_configuration_matches(&root, &plan, &configuration)? {
                return Err(SynchronizationError::RecoveryRequired);
            }
            decision(self.checkpoint_synchronization(
                &root,
                &owner,
                Checkpoint::PushVerified,
                &evidence,
            )?)?;
            Authority::Published(candidate)
        };
        self.synchronization_point(
            &root,
            &owner,
            RemoteOperationSafePoint::AfterPushVerification,
        )?;
        if !self.synchronization_configuration_matches(&root, &plan, &configuration)? {
            return Err(SynchronizationError::RecoveryRequired);
        }
        self.synchronization_boundary(&root, &owner)?;
        decision(self.classify_synchronization(&root, &owner, authority)?)?;
        Ok(self.synchronization_refresh(
            &request,
            Some(&target),
            authority_outcome(&request.target, authority),
        ))
    }
    /// Endpoint-qualified proof stays in the existing envelope, never in Fetch
    /// history. Endpoint/plan edits monotonically fence configuration generations.
    fn synchronization_push_absence_boundary(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        inherited: bool,
    ) -> Result<Option<PushAbsenceBoundary>, SynchronizationError> {
        state::with_transaction(self, root, |tx, id| {
            let current = reservation::owned(self, tx, id, owner)?;
            let Some((kind, item_id)) = current.target.item() else { return Ok(None); };
            let mut query = tx.prepare("SELECT operation_ulid FROM remote_operation_records WHERE repository_id=?1 AND action='synchronize_context' AND kind=?2 AND item_id=?3").map_err(|_|state::recovery_required())?;
            let ids = query.query_map(rusqlite::params![id,authoring_kind_segment(&kind),item_id.to_string()],|row|row.get::<_,String>(0)).map_err(|_|state::recovery_required())?.collect::<Result<Vec<_>,_>>().map_err(|_|state::recovery_required())?;
            let mut verified = false;
            let mut ambiguous = false;
            let mut incompatible = false;
            for operation in ids {
                let operation = OperationId::parse(&operation).map_err(|_|state::recovery_required())?;
                if operation == owner.operation_id() && !inherited { continue; }
                let record = state::read_operation(tx,id,operation)?.ok_or_else(state::recovery_required)?;
                if record.generation > current.generation || record.target.local_branch() != current.target.local_branch() || record.target.context_ref().map(RemoteRefTarget::remote_ref) != current.target.context_ref().map(RemoteRefTarget::remote_ref) { return Err(state::recovery_required()); }
                let evidence = &record.sync_evidence;
                let proven = record.authority.is_some() || record.sync_checkpoint == Some(Checkpoint::PushVerified) && evidence.push_oid.is_some() && evidence.push_advertised_oid == evidence.push_oid;
                let intent = evidence.push_oid.is_some() && matches!(record.sync_checkpoint,Some(Checkpoint::PushPrepared | Checkpoint::PushReturned));
                if proven || intent {
                    if record.generation != current.generation || record.target != current.target { incompatible = true; }
                    else if proven { verified = true; }
                    else { ambiguous = true; }
                }
            }
            Ok(if verified { Some(PushAbsenceBoundary::Deleted) }
                else if ambiguous { Some(PushAbsenceBoundary::Ambiguous) }
                else if incompatible { Some(PushAbsenceBoundary::Unknown) }
                else { None })
        }).map_err(SynchronizationError::Repository)
    }

    fn synchronization_push_observation<P: SessionCredentialProvider>(
        &self,
        request: &SynchronizeRemoteRequest,
        session: &mut SessionCredentials<P>,
        owner: &RemoteReservation,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
        candidate: git2::Oid,
    ) -> Result<Option<git2::Oid>, SynchronizationError> {
        let transport = VerifySshTransportRequest {
            root: request.root.clone(),
            direction: SshDirection::Push,
            approval: request.approval.clone(),
        };
        let advertised = self.with_synchronization_remote(
            transport.clone(),
            session,
            owner,
            plan,
            configuration,
            |r| r.fresh_advertisement(),
        )?;
        self.synchronization_boundary(&request.root, owner)?;
        self.inspect_synchronization_local(
            &request.root,
            plan.primary()
                .remote_ref()
                .strip_prefix("refs/heads/")
                .ok_or(SynchronizationError::RecoveryRequired)?,
            &request.target,
        )?;
        let selected = target_ref(plan, &request.target);
        let oid = advertised_oid(&advertised, selected.remote_ref())?;
        if let Some(oid) = oid
            && oid != candidate
        {
            let repository = git2::Repository::open(&request.root)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            if repository.find_commit(oid).is_err() {
                let downloaded = self.with_synchronization_remote(
                    transport.clone(),
                    session,
                    owner,
                    plan,
                    configuration,
                    |r| r.download_push_target(plan, &request.target),
                )?;
                self.synchronization_boundary(&request.root, owner)?;
                if downloaded != Some(oid) || repository.find_commit(oid).is_err() {
                    return Err(SynchronizationError::ExternalChange);
                }
            }
            // Receive-pack proof after upload-pack object acquisition, never assume
            // that an upload-pack advertisement names the same current target.
            let fresh = self.with_synchronization_remote(
                transport,
                session,
                owner,
                plan,
                configuration,
                |r| r.fresh_advertisement(),
            )?;
            self.synchronization_boundary(&request.root, owner)?;
            if advertised_oid(&fresh, selected.remote_ref())? != Some(oid) {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        Ok(oid)
    }
    fn bind_local_synchronization(
        &self,
        request: &SynchronizeRemoteRequest,
        primary: &str,
    ) -> Result<git2::Oid, SynchronizationError> {
        #[cfg(test)]
        tests::run_local_binding_hook(request.operation_id);
        let repository = git2::Repository::open(&request.root)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let _lease = repository_lease(
            &repository,
            &request.root,
            RepositoryOperation::RefreshRepository,
        )?;
        let oid = local_oid(&local_target(&request.root, primary, &request.target)?)?;
        let _guard = cache_write_guard(
            &self.registry_path,
            &request.root,
            RepositoryOperation::RefreshRepository,
        )?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        crate::repository::recovery::begin_or_reconcile_local_synchronization(
            &mut connection,
            &request.root,
            RepositoryOperation::RefreshRepository,
            request.operation_id,
            &format!("{}/{oid}", local_refresh_identity(&request.target)),
        )
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
        Ok(oid)
    }

    fn local_synchronization_replay(
        &self,
        request: &SynchronizeRemoteRequest,
    ) -> Result<Option<git2::Oid>, SynchronizationError> {
        let row = refresh_row(self, request)?;
        let Some((root, action, matcher, _)) = row else {
            return Ok(None);
        };
        if root != request.root.to_string_lossy() || action != "refresh" {
            return Err(identity_mismatch());
        }
        let Some((identity, oid)) = matcher.rsplit_once('/') else {
            return Err(identity_mismatch());
        };
        let oid = git2::Oid::from_str(oid).map_err(|_| identity_mismatch())?;
        if identity != local_refresh_identity(&request.target) {
            return Err(identity_mismatch());
        }
        Ok(Some(oid))
    }
    fn synchronization_refresh(
        &self,
        request: &SynchronizeRemoteRequest,
        target: Option<&RemoteOperationTarget>,
        outcome: SynchronizationOutcome,
    ) -> SynchronizationResult {
        let matcher = match &outcome {
            SynchronizationOutcome::PublishPending {
                target, local_oid, ..
            } => format!("{}/{local_oid}", local_refresh_identity(target)),
            _ => String::new(),
        };
        let completed = refresh_row(self, request).is_ok_and(|row| {
            row.is_some_and(|(root, action, stored, state)| {
                root == request.root.to_string_lossy()
                    && action == "refresh"
                    && stored == matcher
                    && state == "completed"
            })
        });
        let refreshed = completed
            || matches!(
                self.refresh_repository_target(
                    RefreshRepositoryRequest {
                        root: request.root.clone(),
                        operation_id: request.operation_id
                    },
                    &matcher
                ),
                Ok(RefreshOutcome::Refreshed { .. })
            );
        if refreshed
            && target.is_none_or(|target| {
                self.finish_synchronization_index(&request.root, request.operation_id, target)
                    .is_ok()
            })
        {
            SynchronizationResult::Complete(outcome)
        } else {
            SynchronizationResult::IndexPending(IndexPending::new(outcome))
        }
    }
}
fn authority_outcome(
    target: &SynchronizationTarget,
    authority: Authority,
) -> SynchronizationOutcome {
    match authority {
        Authority::Published(oid) => SynchronizationOutcome::Published {
            target: target.clone(),
            oid,
        },
        Authority::AlreadyCurrent(oid) => SynchronizationOutcome::AlreadyCurrent {
            target: target.clone(),
            oid,
        },
    }
}
fn fast_forward(
    repository: &git2::Repository,
    branch: &str,
    old: git2::Oid,
    new: git2::Oid,
) -> Result<(), SynchronizationError> {
    let commit = repository
        .find_commit(new)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    // Lock/compare before effects. Locked git2 has no checkout baseline setter:
    // safe checkout must see the OLD HEAD, then the single ref commit follows.
    let mut transaction = repository
        .transaction()
        .map_err(|_| SynchronizationError::ExternalChange)?;
    transaction
        .lock_ref(branch)
        .map_err(|_| SynchronizationError::ExternalChange)?;
    if repository.refname_to_id(branch).ok() != Some(old)
        || repository
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(str::to_owned))
            .as_deref()
            != Some(branch)
    {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut checkout = git2::build::CheckoutBuilder::new();
    // SAFE alone defaults to overwriting ignored user entries in locked libgit2.
    checkout.safe().overwrite_ignored(false);
    repository
        .checkout_tree(commit.as_object(), Some(&mut checkout))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    #[cfg(test)]
    super::observation_tests::checkpoint(RemoteOperationSafePoint::BeforeLocalMutation);
    if repository.refname_to_id(branch).ok() != Some(old)
        || repository
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(str::to_owned))
            .as_deref()
            != Some(branch)
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    transaction
        .set_target(branch, new, None, "manyhands synchronization")
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    transaction
        .commit()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    Ok(())
}
fn identity_mismatch() -> SynchronizationError {
    identity_error().into()
}
fn identity_error() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::OperationMismatch,
        "synchronization identity mismatch",
    )
}
fn local_refresh_identity(target: &SynchronizationTarget) -> String {
    match target {
        SynchronizationTarget::Primary => "synchronization-local-v1/primary".into(),
        SynchronizationTarget::Context { kind, item_id } => format!(
            "synchronization-local-v1/{}/{item_id}",
            authoring_kind_segment(kind)
        ),
    }
}
fn validate_record_target(
    record: &RemoteOperationTarget,
    target: &SynchronizationTarget,
) -> Result<(), SynchronizationError> {
    let valid = match target {
        SynchronizationTarget::Primary => {
            record.action() == RemoteOperationAction::SynchronizePrimary && record.item().is_none()
        }
        SynchronizationTarget::Context { kind, item_id } => {
            record.action() == RemoteOperationAction::SynchronizeContext
                && record.item() == Some((*kind, item_id))
        }
    };
    if valid {
        Ok(())
    } else {
        Err(identity_mismatch())
    }
}
type RefreshRow = (String, String, String, String);
fn refresh_row(
    service: &RepositoryService,
    request: &SynchronizeRemoteRequest,
) -> Result<Option<RefreshRow>, RepositoryError> {
    use rusqlite::OptionalExtension;
    state::with_transaction(service, &request.root, |tx, _| {
        tx.query_row("SELECT root_path,action,coalesce(target,''),state FROM operation_records WHERE operation_ulid=?1",[request.operation_id.to_string()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional().map_err(|_|state::recovery_required())
    })
}
