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
    WorktreeNotClean { target: SynchronizationTarget },
    WorktreeConflicted { target: SynchronizationTarget },
    PrimaryMissing,
    RemoteContextDeleted,
    HistoryUnknown,
    MergeRequired { target: SynchronizationTarget },
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
    if linked
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
            state::read_operation(tx, id, request.operation_id)
        })?;
        if let Some(record) = &existing
            && record.authority.is_some()
        {
            validate_record_target(&record.target, &request.target)?;
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
            validate_record_target(&record.target, &request.target)?;
            self.reserve_remote_operation(&root, request.operation_id, &record.target)?;
            return Err(SynchronizationError::Interrupted);
        }
        if let Some(record) = &existing
            && !request.restart
        {
            validate_record_target(&record.target, &request.target)?;
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
        } else if let Some(record) = &existing {
            validate_record_target(&record.target, &request.target)?;
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
        let mut resumed = None;
        if prior.is_some() {
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
        begin_or_reconcile_operation(
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
    checkout.safe();
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
    SynchronizationError::Repository(RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::OperationMismatch,
        "synchronization identity mismatch",
    ))
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
