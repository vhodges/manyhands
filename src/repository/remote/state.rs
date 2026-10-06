use std::{fmt, time::Duration};

use git2::Oid;

use super::{RemoteRefClassification, RemoteRefPlan, RemoteRefTarget};
use crate::{canonical::ItemId, repository::AuthoringKind};

const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 300;
const MINIMUM_POLL_INTERVAL_SECONDS: u64 = 60;
const MAXIMUM_POLL_INTERVAL_SECONDS: u64 = 3_600;
const MINIMUM_AUTOMATIC_BACKOFF_SECONDS: u64 = 60;
const MAXIMUM_AUTOMATIC_BACKOFF_SECONDS: u64 = 900;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollingInterval(Duration);

impl PollingInterval {
    pub fn from_seconds(seconds: u64) -> Result<Self, RemotePollingValueError> {
        if !(MINIMUM_POLL_INTERVAL_SECONDS..=MAXIMUM_POLL_INTERVAL_SECONDS).contains(&seconds) {
            return Err(RemotePollingValueError::IntervalOutOfRange);
        }
        Ok(Self(Duration::from_secs(seconds)))
    }

    pub fn duration(self) -> Duration {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutomaticBackoff(Duration);

impl AutomaticBackoff {
    pub fn from_seconds(seconds: u64) -> Result<Self, RemotePollingValueError> {
        if !(MINIMUM_AUTOMATIC_BACKOFF_SECONDS..=MAXIMUM_AUTOMATIC_BACKOFF_SECONDS)
            .contains(&seconds)
        {
            return Err(RemotePollingValueError::BackoffOutOfRange);
        }
        Ok(Self(Duration::from_secs(seconds)))
    }

    pub fn duration(self) -> Duration {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemotePollingValueError {
    IntervalOutOfRange,
    BackoffOutOfRange,
}

impl fmt::Display for RemotePollingValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::IntervalOutOfRange => "poll interval must be between 60 and 3600 seconds",
            Self::BackoffOutOfRange => "automatic backoff must be between 60 and 900 seconds",
        })
    }
}

impl std::error::Error for RemotePollingValueError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemotePollingConfiguration {
    enabled: bool,
    paused: bool,
    interval: PollingInterval,
    automatic_backoff: Option<AutomaticBackoff>,
    recovery_suspended: bool,
}

impl RemotePollingConfiguration {
    pub fn new(
        enabled: bool,
        paused: bool,
        interval: PollingInterval,
        automatic_backoff: Option<AutomaticBackoff>,
        recovery_suspended: bool,
    ) -> Self {
        Self {
            enabled,
            paused,
            interval,
            automatic_backoff,
            recovery_suspended,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn interval(&self) -> Duration {
        self.interval.duration()
    }

    pub fn automatic_backoff(&self) -> Option<Duration> {
        self.automatic_backoff.map(AutomaticBackoff::duration)
    }

    pub fn recovery_suspended(&self) -> bool {
        self.recovery_suspended
    }

    pub fn with_automatic_backoff(mut self, backoff: AutomaticBackoff) -> Self {
        self.automatic_backoff = Some(backoff);
        self
    }

    pub fn delay_for(&self, invocation: RemotePollInvocation) -> Option<Duration> {
        match invocation {
            RemotePollInvocation::Automatic => self.automatic_backoff(),
            RemotePollInvocation::Explicit => None,
        }
    }
}

impl Default for RemotePollingConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            paused: false,
            interval: PollingInterval(Duration::from_secs(DEFAULT_POLL_INTERVAL_SECONDS)),
            automatic_backoff: None,
            recovery_suspended: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemotePollInvocation {
    Automatic,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationAction {
    Poll,
    SynchronizeContext,
    SynchronizePrimary,
    Promote,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationPriority {
    Poll,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationPhase {
    Reserved,
    Advertising,
    Persisting,
    Completed,
    Interrupted,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationSafePoint {
    BeforeTransport,
    AfterAdvertisement,
    BetweenObservations,
    BeforeBatchCommit,
    AfterBatchCommit,
    BeforeLocalMutation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteOperationTarget {
    action: RemoteOperationAction,
    remote_name: String,
    primary_ref: RemoteRefTarget,
    context_ref: Option<RemoteRefTarget>,
    item: Option<(AuthoringKind, ItemId)>,
    local_branch: Option<String>,
}

impl RemoteOperationTarget {
    pub fn for_poll(plan: &RemoteRefPlan) -> Self {
        Self {
            action: RemoteOperationAction::Poll,
            remote_name: plan.remote_name().to_owned(),
            primary_ref: plan.primary().clone(),
            context_ref: None,
            item: None,
            local_branch: None,
        }
    }

    pub fn for_primary_synchronization(plan: &RemoteRefPlan) -> Self {
        Self {
            action: RemoteOperationAction::SynchronizePrimary,
            remote_name: plan.remote_name().to_owned(),
            primary_ref: plan.primary().clone(),
            context_ref: None,
            item: None,
            local_branch: Some(plan.primary_branch().to_owned()),
        }
    }

    pub fn for_context(
        plan: &RemoteRefPlan,
        action: RemoteOperationAction,
        kind: AuthoringKind,
        item_id: ItemId,
    ) -> Result<Self, RemoteOperationTargetError> {
        if !matches!(
            action,
            RemoteOperationAction::SynchronizeContext
                | RemoteOperationAction::Promote
                | RemoteOperationAction::Close
        ) {
            return Err(RemoteOperationTargetError::ContextNotApplicable);
        }
        let context_ref = plan.context(kind, &item_id);
        let local_branch = context_ref
            .remote_ref()
            .strip_prefix("refs/heads/")
            .expect("derived context refs are always local heads")
            .to_owned();
        Ok(Self {
            action,
            remote_name: plan.remote_name().to_owned(),
            primary_ref: plan.primary().clone(),
            context_ref: Some(context_ref),
            item: Some((kind, item_id)),
            local_branch: Some(local_branch),
        })
    }

    pub fn action(&self) -> RemoteOperationAction {
        self.action
    }

    pub fn remote_name(&self) -> &str {
        &self.remote_name
    }

    pub fn primary_ref(&self) -> &RemoteRefTarget {
        &self.primary_ref
    }

    pub fn context_ref(&self) -> Option<&RemoteRefTarget> {
        self.context_ref.as_ref()
    }

    pub fn item(&self) -> Option<(AuthoringKind, &ItemId)> {
        self.item.as_ref().map(|(kind, item_id)| (*kind, item_id))
    }

    pub fn local_branch(&self) -> Option<&str> {
        self.local_branch.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOperationTargetError {
    ContextNotApplicable,
}

impl fmt::Display for RemoteOperationTargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("remote operation action does not accept a context target")
    }
}

impl std::error::Error for RemoteOperationTargetError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOutcomeCategory {
    Completed,
    ConfigurationRequired,
    SelectedKeyUnavailable,
    UnlockRequired,
    HostApprovalRequired,
    TransportUnavailable,
    ProtocolRejected,
    Cancelled,
    RepositoryUnavailable,
}

impl fmt::Display for RemoteOutcomeCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Completed => "completed",
            Self::ConfigurationRequired => "configuration required",
            Self::SelectedKeyUnavailable => "selected key unavailable",
            Self::UnlockRequired => "unlock required",
            Self::HostApprovalRequired => "host approval required",
            Self::TransportUnavailable => "transport unavailable",
            Self::ProtocolRejected => "protocol rejected",
            Self::Cancelled => "cancelled",
            Self::RepositoryUnavailable => "repository unavailable",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRefObservation {
    target: Option<RemoteRefTarget>,
    advertised_oid: Oid,
    tracking_oid: Option<Oid>,
    classification: RemoteRefClassification,
}

impl RemoteRefObservation {
    #[allow(dead_code)]
    pub(crate) fn from_advertisement(
        plan: &RemoteRefPlan,
        remote_ref: &str,
        advertised_oid: Oid,
        tracking_oid: Option<Oid>,
    ) -> Option<Self> {
        let classification = plan.classify_advertised_ref(remote_ref)?;
        let target = plan.target_for_advertised_ref(remote_ref);
        Some(Self {
            target,
            advertised_oid,
            tracking_oid,
            classification,
        })
    }

    pub fn target(&self) -> Option<&RemoteRefTarget> {
        self.target.as_ref()
    }

    pub fn remote_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::remote_ref)
    }

    pub fn tracking_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::tracking_ref)
    }

    pub fn advertised_oid(&self) -> Oid {
        self.advertised_oid
    }

    pub fn tracking_oid(&self) -> Option<Oid> {
        self.tracking_oid
    }

    pub fn classification(&self) -> &RemoteRefClassification {
        &self.classification
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemotePublicationEvidence {
    NeverPublished,
    ObservedPublished,
    HistoryUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteContextState {
    Observed,
    Unmaterialized,
    Malformed,
    RemotelyDeleted,
    HistoryUnknown,
}

impl fmt::Display for RemoteContextState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Observed => "observed",
            Self::Unmaterialized => "unmaterialized",
            Self::Malformed => "malformed",
            Self::RemotelyDeleted => "remotely deleted",
            Self::HistoryUnknown => "history unknown",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteContextSnapshot {
    target: Option<RemoteRefTarget>,
    kind: Option<AuthoringKind>,
    item_id: Option<ItemId>,
    advertised_oid: Option<Oid>,
    tracking_oid: Option<Oid>,
    publication_evidence: RemotePublicationEvidence,
    state: RemoteContextState,
}

impl RemoteContextSnapshot {
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        target: Option<RemoteRefTarget>,
        kind: Option<AuthoringKind>,
        item_id: Option<ItemId>,
        advertised_oid: Option<Oid>,
        tracking_oid: Option<Oid>,
        publication_evidence: RemotePublicationEvidence,
        state: RemoteContextState,
    ) -> Self {
        Self {
            target,
            kind,
            item_id,
            advertised_oid,
            tracking_oid,
            publication_evidence,
            state,
        }
    }

    pub fn target(&self) -> Option<&RemoteRefTarget> {
        self.target.as_ref()
    }

    pub fn remote_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::remote_ref)
    }

    pub fn tracking_ref(&self) -> Option<&str> {
        self.target.as_ref().map(RemoteRefTarget::tracking_ref)
    }

    pub fn kind(&self) -> Option<AuthoringKind> {
        self.kind
    }

    pub fn item_id(&self) -> Option<&ItemId> {
        self.item_id.as_ref()
    }

    pub fn advertised_oid(&self) -> Option<Oid> {
        self.advertised_oid
    }

    pub fn tracking_oid(&self) -> Option<Oid> {
        self.tracking_oid
    }

    pub fn publication_evidence(&self) -> RemotePublicationEvidence {
        self.publication_evidence
    }

    pub fn state(&self) -> RemoteContextState {
        self.state
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteSnapshot {
    polling: RemotePollingConfiguration,
    latest_outcome: Option<RemoteOutcomeCategory>,
    observations: Vec<RemoteRefObservation>,
    contexts: Vec<RemoteContextSnapshot>,
}

impl RemoteSnapshot {
    #[allow(dead_code)]
    pub(crate) fn new(
        polling: RemotePollingConfiguration,
        latest_outcome: Option<RemoteOutcomeCategory>,
        observations: Vec<RemoteRefObservation>,
        contexts: Vec<RemoteContextSnapshot>,
    ) -> Self {
        Self {
            polling,
            latest_outcome,
            observations,
            contexts,
        }
    }

    pub fn polling(&self) -> &RemotePollingConfiguration {
        &self.polling
    }

    pub fn latest_outcome(&self) -> Option<RemoteOutcomeCategory> {
        self.latest_outcome
    }

    pub fn observations(&self) -> &[RemoteRefObservation] {
        &self.observations
    }

    pub fn contexts(&self) -> &[RemoteContextSnapshot] {
        &self.contexts
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteObservationOutcome {
    category: RemoteOutcomeCategory,
    snapshot: RemoteSnapshot,
}

impl RemoteObservationOutcome {
    #[allow(dead_code)]
    pub(crate) fn new(category: RemoteOutcomeCategory, snapshot: RemoteSnapshot) -> Self {
        Self { category, snapshot }
    }

    pub fn category(&self) -> RemoteOutcomeCategory {
        self.category
    }

    pub fn snapshot(&self) -> &RemoteSnapshot {
        &self.snapshot
    }
}
