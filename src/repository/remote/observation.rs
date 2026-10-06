//! Complete authenticated advertisements; no transfer or Git mutation.
#![allow(clippy::result_large_err)]
use super::*;
use crate::repository::{OperationId, RepositoryError, RepositoryService, keys::*, transport::*};
use std::{fmt, path::PathBuf};

#[derive(Clone, Debug)]
pub struct ObservePublicationRemoteRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
    /// Identifies an already eligible caller-owned attempt. This service does
    /// not schedule, sleep, or override repository pause policy.
    pub invocation: RemotePollInvocation,
    pub approval: Option<HostApproval>,
    /// Explicit recovery authorization: fence an abandoned executor and resume
    /// its read-only operation. Ordinary duplicate calls never steal ownership.
    pub restart: bool,
}

#[derive(Debug)]
pub enum RemoteObservationError {
    Repository(RepositoryError),
    Transport(SshTransportError),
    Busy,
    PollYielding,
    Interrupted,
}
impl fmt::Display for RemoteObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Repository(_) => "remote observation repository unavailable",
            Self::Transport(_) => "remote observation transport unavailable",
            Self::Busy => "remote operation busy",
            Self::PollYielding => "automatic observation yielding",
            Self::Interrupted => "remote observation interrupted",
        })
    }
}
impl std::error::Error for RemoteObservationError {}
impl From<RepositoryError> for RemoteObservationError {
    fn from(value: RepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl RepositoryService {
    /// Observe a complete Fetch-direction advertisement using the selected SSH
    /// key and the caller's shared process session. No refs or objects transfer.
    /// Duplicate operation IDs read durable outcomes; unfinished duplicates
    /// require explicit `restart` authorization to fence the prior executor.
    pub fn observe_publication_remote<P: SessionCredentialProvider>(
        &self,
        request: ObservePublicationRemoteRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<RemoteObservationOutcome, RemoteObservationError> {
        let root = &request.root;
        let plan = match observation_plan(root) {
            Ok(plan) => plan,
            Err(error) => {
                session.clear();
                state::with_transaction(self, root, |tx, id| {
                    state::configure(tx, id, None, false)?;
                    state::record_outcome(
                        tx,
                        id,
                        RemoteOutcomeCategory::ConfigurationRequired,
                        None,
                    )
                })?;
                return Err(RemoteObservationError::Transport(error));
            }
        };
        state::with_transaction(self, root, |tx, id| {
            state::configure(tx, id, Some(&plan), false)
        })?;
        let target = RemoteOperationTarget::for_poll(&plan);
        let reservation = self.reserve_remote_operation_with_priority(
            root,
            request.operation_id,
            &target,
            match request.invocation {
                RemotePollInvocation::Explicit => RemoteOperationPriority::Manual,
                RemotePollInvocation::Automatic => RemoteOperationPriority::Poll,
            },
        )?;
        let reservation =
            if request.restart && matches!(reservation, RemoteReservationOutcome::Replay(_)) {
                self.restart_remote_observation(root, request.operation_id, &target)?
            } else {
                reservation
            };
        let owner = match reservation {
            RemoteReservationOutcome::Reserved(owner) => owner,
            RemoteReservationOutcome::Busy => return Err(RemoteObservationError::Busy),
            RemoteReservationOutcome::PollYielding => {
                return Err(RemoteObservationError::PollYielding);
            }
            RemoteReservationOutcome::Replay(record) => {
                return match record.phase() {
                    RemoteOperationPhase::Completed
                    | RemoteOperationPhase::Cancelled
                    | RemoteOperationPhase::Failed => self.observation_result(
                        root,
                        record.outcome().ok_or_else(state::recovery_required)?,
                    ),
                    RemoteOperationPhase::Interrupted => Err(RemoteObservationError::Interrupted),
                    _ => Err(RemoteObservationError::Busy),
                };
            }
        };
        if let Some(result) =
            self.observation_safe_point(root, &owner, RemoteOperationSafePoint::BeforeTransport)?
        {
            return Ok(result);
        }
        let advertised = self.with_authenticated_remote_policy(
            VerifySshTransportRequest {
                root: root.clone(),
                direction: SshDirection::Fetch,
                approval: request.approval,
            },
            session,
            request.invocation == RemotePollInvocation::Automatic,
            |remote| remote.advertisement(),
        );
        // The scoped adapter (and all transport borrows) has dropped here.
        if let Some(result) =
            self.observation_safe_point(root, &owner, RemoteOperationSafePoint::AfterAdvertisement)?
        {
            return Ok(result);
        }
        let advertised = match advertised {
            Ok(advertised) => advertised,
            Err(error) => {
                let decision =
                    self.finish_remote_operation(root, &owner, transport_category(&error.kind))?;
                if let Some(result) = self.observation_decision(root, decision)? {
                    return Ok(result);
                }
                return Err(RemoteObservationError::Transport(error));
            }
        };
        let repository = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
        let mut observations = Vec::new();
        for (name, oid) in advertised {
            if let Some(result) = self.observation_safe_point(
                root,
                &owner,
                RemoteOperationSafePoint::BetweenObservations,
            )? {
                return Ok(result);
            }
            let tracking_oid = match plan.tracking_ref_for(&name) {
                None => None,
                Some(tracking) => match repository.refname_to_id(&tracking) {
                    Ok(oid) => Some(oid),
                    Err(error) if error.code() == git2::ErrorCode::NotFound => None,
                    Err(_) => return Err(state::recovery_required().into()),
                },
            };
            if let Some(observation) =
                RemoteRefObservation::from_advertisement(&plan, &name, oid, tracking_oid)
            {
                observations.push(observation);
            }
        }
        drop(repository);
        if let Some(result) =
            self.observation_safe_point(root, &owner, RemoteOperationSafePoint::BeforeBatchCommit)?
        {
            return Ok(result);
        }
        // Recheck after all prompt/network and local observation work. The
        // transaction below independently fences registered configuration edits.
        if observation_plan(root).as_ref() != Ok(&plan) {
            let decision = self.finish_remote_operation(
                root,
                &owner,
                RemoteOutcomeCategory::ConfigurationRequired,
            )?;
            if let Some(result) = self.observation_decision(root, decision)? {
                return Ok(result);
            }
            return Err(RemoteObservationError::Interrupted);
        }
        let decision = reservation::commit_observation_batch(
            self,
            root,
            &owner,
            &plan,
            &observations,
            time::OffsetDateTime::now_utc().unix_timestamp().max(0),
        )?;
        if let Some(result) = self.observation_decision(root, decision)? {
            return Ok(result);
        }
        if let Some(result) =
            self.observation_safe_point(root, &owner, RemoteOperationSafePoint::AfterBatchCommit)?
        {
            return Ok(result);
        }
        let decision =
            self.finish_remote_operation(root, &owner, RemoteOutcomeCategory::Completed)?;
        if let Some(result) = self.observation_decision(root, decision)? {
            return Ok(result);
        }
        self.observation_result(root, RemoteOutcomeCategory::Completed)
    }

    fn observation_result(
        &self,
        root: &std::path::Path,
        category: RemoteOutcomeCategory,
    ) -> Result<RemoteObservationOutcome, RemoteObservationError> {
        Ok(RemoteObservationOutcome::new(
            category,
            self.remote_snapshot(root)?,
        ))
    }
    fn observation_decision(
        &self,
        root: &std::path::Path,
        decision: RemoteSafePointOutcome,
    ) -> Result<Option<RemoteObservationOutcome>, RemoteObservationError> {
        match decision {
            RemoteSafePointOutcome::Continue => Ok(None),
            RemoteSafePointOutcome::Interrupted => Err(RemoteObservationError::Interrupted),
            RemoteSafePointOutcome::Cancelled => self
                .observation_result(root, RemoteOutcomeCategory::Cancelled)
                .map(Some),
        }
    }
    fn observation_safe_point(
        &self,
        root: &std::path::Path,
        owner: &RemoteReservation,
        point: RemoteOperationSafePoint,
    ) -> Result<Option<RemoteObservationOutcome>, RemoteObservationError> {
        #[cfg(test)]
        super::observation_tests::checkpoint(point);
        self.observation_decision(root, self.remote_safe_point(root, owner, point)?)
    }
}

fn observation_plan(root: &std::path::Path) -> Result<RemoteRefPlan, SshTransportError> {
    let mut error = SshTransportError {
        root: root.to_owned(),
        remote_name: String::new(),
        direction: SshDirection::Fetch,
        selected_key_id: None,
        authority: None,
        kind: SshTransportErrorKind::ConfigurationInvalid,
    };
    let crate::repository::ConfigurationInspection::Valid(config) =
        crate::repository::read_configuration(root).map_err(|_| error.clone())?
    else {
        return Err(error);
    };
    let Some(remote) = config.publication_remote else {
        error.kind = SshTransportErrorKind::PublicationRemoteMissing;
        return Err(error);
    };
    RemoteRefPlan::from_configuration(&remote, &config.primary_branch).map_err(|_| error)
}

fn transport_category(kind: &SshTransportErrorKind) -> RemoteOutcomeCategory {
    use SshTransportErrorKind::*;
    match kind {
        ConfigurationInvalid | PublicationRemoteMissing | UsernameRequired | EndpointChanged => {
            RemoteOutcomeCategory::ConfigurationRequired
        }
        NoSelectedKey
        | KeyMissing
        | KeyUnreadable
        | KeySourceChanged
        | SelectionChanged
        | KeyInvalidOrUnsupported
        | KeyRejected => RemoteOutcomeCategory::SelectedKeyUnavailable,
        UnlockCancelled | ProviderUnavailable | UnlockFailed => {
            RemoteOutcomeCategory::UnlockRequired
        }
        HostApprovalRequired { .. }
        | HostReplacementRequired { .. }
        | HostTrustChanged
        | HostVerificationUnavailable => RemoteOutcomeCategory::HostApprovalRequired,
        RegistryUnavailable => RemoteOutcomeCategory::RepositoryUnavailable,
        ProtocolFailure | PushRejected => RemoteOutcomeCategory::ProtocolRejected,
        RuntimeUninitialized | TransportUnavailable | RemoteUnavailable => {
            RemoteOutcomeCategory::TransportUnavailable
        }
    }
}
