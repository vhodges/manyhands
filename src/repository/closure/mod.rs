//! Confirmed ticket closure: its read-only preflight, its parent record, the
//! closure checkpoint, and `close_ticket` over the one generic lifecycle
//! runner (C3).
use crate::canonical::ItemId;
use crate::repository::{
    ConfirmedCommitIdentity, ExpectedConfiguration, IndexPending, OperationId, PlannedCleanup,
    PlannedIntegration, PlannedPublication, PromotionCleanup, PromotionIntegration,
    PromotionPathChange, PromotionPublication, RepositoryError, RepositoryService,
    SynchronizationError,
    keys::{SessionCredentialProvider, SessionCredentials},
    transport::{HostApproval, SshTransportError},
};
use git2::Oid;
use std::{
    fmt,
    path::{Path, PathBuf},
};

pub(super) mod checkpoint;
pub(super) mod lifecycle;
pub(super) mod observe;
pub(super) mod record;
#[cfg(test)]
#[path = "run_tests.rs"]
mod run_tests;
#[cfg(test)]
mod test_support;

/// Declaration order is the stable order of preflight blockers and the bit
/// order of the stored refusal mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureBlocker {
    NotATicket,
    ContextMissing,
    ContextNonconforming,
    ContextDirty,
    ContextConflicted,
    ConflictPending,
    LocalOperationPending,
    CommentPublicationPending,
    PrimaryMissing,
    PrimaryNotCheckedOut,
    PrimaryDirty,
    PrimaryConflicted,
    PrimaryAheadOfRemote,
    ContextBehindRemote,
    IntegrationConflict,
    RemoteContextDeleted,
    HistoryUnknown,
    AlreadyClosed,
    PrimaryReopened,
    TicketInvalidAfterClosure,
    IdentityRequired,
}

/// The 32-byte consent digest of a ticket closure. Its `Debug` never shows
/// the bytes: they hash low-entropy input that includes `closed_by`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClosureExpectation([u8; 32]);
impl ClosureExpectation {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}
impl fmt::Debug for ClosureExpectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClosureExpectation(<redacted>)")
    }
}

#[derive(Clone)]
pub struct PreviewTicketClosureRequest {
    pub root: PathBuf,
    pub item_id: ItemId,
    /// A configuration-bound identity confirmation (C2), used only when the
    /// effective Git configuration has no complete identity.
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
}
impl fmt::Debug for PreviewTicketClosureRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreviewTicketClosureRequest")
            .field("item_id", &self.item_id)
            .finish_non_exhaustive()
    }
}

/// The closure effect a confirmed closure has. `closed_by` is personal data:
/// `Debug` shows the variant only.
#[derive(Clone, PartialEq, Eq)]
pub enum PlannedClosure {
    /// The checkpoint will record `closed_at` (taken at execution) and this
    /// `closed_by` (`"<name> <<email>>"` of the selected identity).
    Record { closed_by: String },
    /// The ticket already carries both fields, exactly as in the file.
    Existing {
        closed_at: String,
        closed_by: String,
    },
}
impl fmt::Debug for PlannedClosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Record { .. } => "Record",
            Self::Existing { .. } => "Existing",
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TicketClosurePreview {
    item_id: ItemId,
    ticket_path: PathBuf,
    context_branch: String,
    context_oid: Oid,
    primary_branch: String,
    primary_oid: Oid,
    publication_remote: Option<String>,
    tracking_primary: Option<Oid>,
    tracking_context: Option<Oid>,
    changes: Vec<PromotionPathChange>,
    closure: PlannedClosure,
    integration: PlannedIntegration,
    publication: PlannedPublication,
    cleanup: PlannedCleanup,
    expectation: ClosureExpectation,
}
impl TicketClosurePreview {
    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }
    /// The ticket's canonical path, also bound by the expectation.
    pub fn ticket_path(&self) -> &Path {
        &self.ticket_path
    }
    pub fn context_branch(&self) -> &str {
        &self.context_branch
    }
    pub fn context_oid(&self) -> Oid {
        self.context_oid
    }
    pub fn primary_branch(&self) -> &str {
        &self.primary_branch
    }
    pub fn primary_oid(&self) -> Oid {
        self.primary_oid
    }
    pub fn publication_remote(&self) -> Option<&str> {
        self.publication_remote.as_deref()
    }
    pub fn tracking_primary(&self) -> Option<Oid> {
        self.tracking_primary
    }
    pub fn tracking_context(&self) -> Option<Oid> {
        self.tracking_context
    }
    /// The pre-closure change set, sorted by path. The checkpoint's own
    /// change to the ticket is the closure effect, not an entry here.
    pub fn changes(&self) -> &[PromotionPathChange] {
        &self.changes
    }
    pub fn closure(&self) -> &PlannedClosure {
        &self.closure
    }
    pub fn integration(&self) -> PlannedIntegration {
        self.integration
    }
    pub fn publication(&self) -> PlannedPublication {
        self.publication
    }
    pub fn cleanup(&self) -> PlannedCleanup {
        self.cleanup
    }
    pub fn expectation(&self) -> ClosureExpectation {
        self.expectation
    }
}
impl fmt::Debug for TicketClosurePreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TicketClosurePreview")
            .field("item_id", &self.item_id)
            .field("context_oid", &self.context_oid)
            .field("primary_oid", &self.primary_oid)
            .field("closure", &self.closure)
            .field("integration", &self.integration)
            .field("publication", &self.publication)
            .field("cleanup", &self.cleanup)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // The approved preflight API returns the ready preview by value.
pub enum TicketClosurePreflight {
    Ready(TicketClosurePreview),
    Blocked {
        /// Every blocker, in declaration order.
        blockers: Vec<ClosureBlocker>,
        /// `Some` exactly when `blockers` is `[IdentityRequired]`.
        identity_configuration: Option<ExpectedConfiguration>,
    },
}

/// A confirmed closure: the operation, the ticket and the consent digest of
/// its preview. `Debug` shows the IDs only.
#[derive(Clone)]
pub struct CloseTicketRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub item_id: ItemId,
    pub expected: ClosureExpectation,
    pub approval: Option<HostApproval>,
    /// A configuration-bound identity confirmation (C2), used only when the
    /// effective Git configuration has no complete identity.
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
}
impl fmt::Debug for CloseTicketRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloseTicketRequest")
            .field("operation_id", &self.operation_id)
            .field("item_id", &self.item_id)
            .finish_non_exhaustive()
    }
}

/// How the closure reached the ticket: this operation wrote the checkpoint
/// commit, or the context already carried both closure fields (C4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureRecording {
    Recorded { checkpoint_oid: Oid },
    Existing,
}

/// A closure that reached the primary, with what is still pending. `Debug`
/// shows IDs, OIDs and categories only, never `closed_by`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketClosureOutcome {
    operation_id: OperationId,
    closure: ClosureRecording,
    integration: PromotionIntegration,
    publication: PromotionPublication,
    cleanup: PromotionCleanup,
}
impl TicketClosureOutcome {
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    pub fn closure(&self) -> ClosureRecording {
        self.closure
    }
    pub fn integration(&self) -> PromotionIntegration {
        self.integration
    }
    pub fn publication(&self) -> PromotionPublication {
        self.publication
    }
    pub fn cleanup(&self) -> PromotionCleanup {
        self.cleanup
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TicketClosureResult {
    Complete(TicketClosureOutcome),
    IndexPending(IndexPending<TicketClosureOutcome>),
}

/// A refusal before the closure reached the primary. A written checkpoint
/// exists after an error only while the record is at `closing` (ref moved)
/// or `integrating`; the same ID finishes it.
pub enum TicketClosureError {
    Repository(RepositoryError),
    Transport(Box<SshTransportError>),
    Blocked(Vec<ClosureBlocker>),
    ExternalChange,
    ReconfirmationRequired,
    PrimaryAdvanced,
    ContextSynchronization(Box<SynchronizationError>),
    IdentityRequired {
        expected_configuration: ExpectedConfiguration,
    },
    Busy,
    PollYielding,
    Interrupted,
    OperationMismatch,
    RecoveryRequired,
}
impl fmt::Debug for TicketClosureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository(error) => f.debug_tuple("Repository").field(&error.kind).finish(),
            Self::Transport(error) => f.debug_tuple("Transport").field(&error.kind).finish(),
            Self::Blocked(blockers) => f.debug_tuple("Blocked").field(blockers).finish(),
            Self::ExternalChange => f.write_str("ExternalChange"),
            Self::ReconfirmationRequired => f.write_str("ReconfirmationRequired"),
            Self::PrimaryAdvanced => f.write_str("PrimaryAdvanced"),
            // SynchronizationError's legacy Debug can contain repository paths.
            Self::ContextSynchronization(error) => f
                .debug_tuple("ContextSynchronization")
                .field(&error.to_string())
                .finish(),
            Self::IdentityRequired { .. } => f.write_str("IdentityRequired"),
            Self::Busy => f.write_str("Busy"),
            Self::PollYielding => f.write_str("PollYielding"),
            Self::Interrupted => f.write_str("Interrupted"),
            Self::OperationMismatch => f.write_str("OperationMismatch"),
            Self::RecoveryRequired => f.write_str("RecoveryRequired"),
        }
    }
}
impl fmt::Display for TicketClosureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Repository(_) => "closure repository unavailable",
            Self::Transport(_) => "closure transport unavailable",
            Self::Blocked(_) => "closure blocked",
            Self::ExternalChange => "closure state changed",
            Self::ReconfirmationRequired => "closure reconfirmation required",
            Self::PrimaryAdvanced => "closure primary advanced",
            Self::ContextSynchronization(_) => "closure context synchronization incomplete",
            Self::IdentityRequired { .. } => "closure commit identity required",
            Self::Busy => "closure busy",
            Self::PollYielding => "closure waiting for automatic observation",
            Self::Interrupted => "closure interrupted",
            Self::OperationMismatch => "closure operation mismatch",
            Self::RecoveryRequired => "closure recovery required",
        })
    }
}
impl std::error::Error for TicketClosureError {}

/// Every repository error, whatever its kind, stays a repository error. An
/// `ExternalChange` kind in particular never becomes the consent refusal
/// `ExternalChange`: the runner abandons a consent refusal, and an error from
/// inside the lease may follow a written checkpoint.
impl From<RepositoryError> for TicketClosureError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

/// The parent record of the closure `operation`, when there is one.
pub(super) fn lifecycle_parent(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
) -> Result<Option<crate::repository::promotion::ParentView>, RepositoryError> {
    use crate::repository::promotion::lifecycle::LifecycleRecord;
    crate::repository::remote::state::with_transaction(service, root, |tx, repository_id| {
        Ok(
            record::read_record(tx, repository_id, operation)?.map(|parent| {
                crate::repository::promotion::ParentView {
                    step: parent.step(),
                    refused: parent.refusal().is_some(),
                    checkpoint: parent.closure_oid,
                    integration: parent.integration_oid(),
                }
            }),
        )
    })
}

impl RepositoryService {
    /// Observe local Git, the stored publication history and pending local
    /// work without a repository lease, network access, or a Git/registry
    /// mutation. Consent is checked again against a fresh observation before
    /// any closure effect.
    pub fn preview_ticket_closure(
        &self,
        request: PreviewTicketClosureRequest,
    ) -> Result<TicketClosurePreflight, RepositoryError> {
        observe::preview(self, &request)
    }
    /// Close a ticket as previewed: synchronize its context (with a remote),
    /// write the closure checkpoint, merge it into the primary, publish and
    /// retire the context. The same operation ID finishes an interrupted
    /// closure; a completed one replays its stored outcome.
    pub fn close_ticket<P: SessionCredentialProvider>(
        &self,
        request: CloseTicketRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        lifecycle::close(self, request, session)
    }
    /// Ask an unfinished closure to stop at its next safe point. `false` when
    /// no unfinished closure has this ID.
    pub fn cancel_ticket_closure(
        &self,
        root: &Path,
        id: OperationId,
    ) -> Result<bool, RepositoryError> {
        crate::repository::remote::state::with_transaction(self, root, |tx, repository_id| {
            record::request_cancellation(tx, repository_id, id)
        })
    }
}
