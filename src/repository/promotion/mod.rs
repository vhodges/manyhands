//! Consent evidence and guarded primary integration for managed documents.
use super::{RepositoryError, RepositoryService};
use crate::canonical::ItemId;
use crate::repository::{
    ConfirmedCommitIdentity, IndexPending, OperationId, SynchronizationError,
    keys::{SessionCredentialProvider, SessionCredentials},
    transport::{HostApproval, SshTransportError},
};
use git2::Oid;
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[allow(dead_code)] // Internal integration stages are composed by the later lifecycle runner.
pub(super) mod integrate;
pub(super) mod lifecycle;
pub(super) mod lifecycle_record;
pub(super) mod observe;
pub(super) mod record;
pub(super) mod run;

#[cfg(test)]
#[path = "run_tests.rs"]
pub(super) mod run_tests;

/// A refusal before promotion has changed the primary. Integration uncertainty
/// is carried by the internal pending state, never by this error.
pub enum DocumentPromotionError {
    Repository(RepositoryError),
    Transport(Box<SshTransportError>),
    Blocked(Vec<PromotionBlocker>),
    ExternalChange,
    ReconfirmationRequired,
    PrimaryAdvanced,
    IdentityRequired {
        expected_configuration: super::ExpectedConfiguration,
    },
    ContextSynchronization(Box<SynchronizationError>),
    Busy,
    PollYielding,
    Interrupted,
    OperationMismatch,
    RecoveryRequired,
}

impl fmt::Debug for DocumentPromotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository(error) => f.debug_tuple("Repository").field(&error.kind).finish(),
            Self::Transport(error) => f.debug_tuple("Transport").field(&error.kind).finish(),
            Self::Blocked(blockers) => f.debug_tuple("Blocked").field(blockers).finish(),
            Self::ExternalChange => f.write_str("ExternalChange"),
            Self::ReconfirmationRequired => f.write_str("ReconfirmationRequired"),
            Self::PrimaryAdvanced => f.write_str("PrimaryAdvanced"),
            Self::IdentityRequired { .. } => f.write_str("IdentityRequired"),
            // SynchronizationError's legacy Debug can contain repository paths.
            Self::ContextSynchronization(error) => f
                .debug_tuple("ContextSynchronization")
                .field(&error.to_string())
                .finish(),
            Self::Busy => f.write_str("Busy"),
            Self::PollYielding => f.write_str("PollYielding"),
            Self::Interrupted => f.write_str("Interrupted"),
            Self::OperationMismatch => f.write_str("OperationMismatch"),
            Self::RecoveryRequired => f.write_str("RecoveryRequired"),
        }
    }
}

impl fmt::Display for DocumentPromotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Repository(_) => "promotion repository unavailable",
            Self::Transport(_) => "promotion transport unavailable",
            Self::Blocked(_) => "promotion blocked",
            Self::ExternalChange => "promotion state changed",
            Self::ReconfirmationRequired => "promotion reconfirmation required",
            Self::PrimaryAdvanced => "promotion primary advanced",
            Self::IdentityRequired { .. } => "promotion commit identity required",
            Self::ContextSynchronization(_) => "promotion context synchronization incomplete",
            Self::Busy => "promotion busy",
            Self::PollYielding => "promotion waiting for automatic observation",
            Self::Interrupted => "promotion interrupted",
            Self::OperationMismatch => "promotion operation mismatch",
            Self::RecoveryRequired => "promotion recovery required",
        })
    }
}
impl std::error::Error for DocumentPromotionError {}

impl From<RepositoryError> for DocumentPromotionError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

#[derive(Clone)]
pub struct PromoteDocumentRequest {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub item_id: ItemId,
    pub expected: PromotionExpectation,
    pub approval: Option<HostApproval>,
    pub confirmed_identity: Option<ConfirmedCommitIdentity>,
}
impl fmt::Debug for PromoteDocumentRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromoteDocumentRequest")
            .field("operation_id", &self.operation_id)
            .field("item_id", &self.item_id)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionIntegration {
    Merged { oid: Oid },
    AlreadyContained { oid: Oid },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionSynchronizationCategory {
    Conflict,
    Rejected,
    Transport,
    Identity,
    Busy,
    Yielding,
    RecoveryRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionPublicationPending {
    NoPublicationRemote,
    Synchronization(PromotionSynchronizationCategory),
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionPublication {
    Published { oid: Oid },
    AlreadyCurrent { oid: Oid },
    Pending { reason: PromotionPublicationPending },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionRemoteDeletionCategory {
    Transport,
    Rejected,
    Configuration,
    Key,
    Unlock,
    Host,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionCleanupPending {
    AwaitingPublication,
    ContextChanged,
    IgnoredFiles,
    RemoteContextChanged,
    RemoteDeletionFailed(PromotionRemoteDeletionCategory),
    RecoveryRequired,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionCleanup {
    Complete,
    Pending { reason: PromotionCleanupPending },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentPromotionOutcome {
    operation_id: OperationId,
    integration: PromotionIntegration,
    publication: PromotionPublication,
    cleanup: PromotionCleanup,
}
impl DocumentPromotionOutcome {
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
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
pub enum DocumentPromotionResult {
    Complete(DocumentPromotionOutcome),
    IndexPending(IndexPending<DocumentPromotionOutcome>),
}

#[derive(Clone)]
pub struct PreviewDocumentPromotionRequest {
    pub root: PathBuf,
    pub item_id: ItemId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Declaration order is the stable order of preflight blockers.
pub enum PromotionBlocker {
    NotADocument,
    ContextMissing,
    ContextNonconforming,
    ContextDirty,
    ContextConflicted,
    ConflictPending,
    PrimaryMissing,
    PrimaryNotCheckedOut,
    PrimaryDirty,
    PrimaryConflicted,
    PrimaryAheadOfRemote,
    ContextBehindRemote,
    IntegrationConflict,
    RemoteContextDeleted,
    HistoryUnknown,
    DocumentInvalidAfterIntegration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlannedIntegration {
    MergeCommit,
    AlreadyContained,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlannedPublication {
    Publish,
    PendingNoRemote,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedCleanup {
    pub remote_branch: bool,
    pub worktree: bool,
    pub local_branch: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathChange {
    Added,
    Modified,
    Deleted,
}
#[derive(Clone, PartialEq, Eq)]
pub struct PromotionPathChange {
    pub(super) path: PathBuf,
    pub(super) change: PathChange,
}
impl PromotionPathChange {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn change(&self) -> PathChange {
        self.change
    }
}
impl fmt::Debug for PromotionPathChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromotionPathChange")
            .field("change", &self.change)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PromotionExpectation([u8; 32]);
impl PromotionExpectation {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}
impl fmt::Debug for PromotionExpectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PromotionExpectation(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DocumentPromotionPreview {
    item_id: ItemId,
    document_path: PathBuf,
    context_branch: String,
    context_oid: Oid,
    primary_branch: String,
    primary_oid: Oid,
    publication_remote: Option<String>,
    tracking_primary: Option<Oid>,
    tracking_context: Option<Oid>,
    changes: Vec<PromotionPathChange>,
    integration: PlannedIntegration,
    publication: PlannedPublication,
    cleanup: PlannedCleanup,
    expectation: PromotionExpectation,
}
impl DocumentPromotionPreview {
    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }
    /// The document's path on its authoring context, also bound by the expectation.
    pub fn document_path(&self) -> &Path {
        &self.document_path
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
    pub fn changes(&self) -> &[PromotionPathChange] {
        &self.changes
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
    pub fn expectation(&self) -> PromotionExpectation {
        self.expectation
    }
}
impl fmt::Debug for DocumentPromotionPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentPromotionPreview")
            .field("item_id", &self.item_id)
            .field("context_oid", &self.context_oid)
            .field("primary_oid", &self.primary_oid)
            .field("integration", &self.integration)
            .field("publication", &self.publication)
            .field("cleanup", &self.cleanup)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // The approved preflight API returns the ready preview by value.
pub enum DocumentPromotionPreflight {
    Ready(DocumentPromotionPreview),
    Blocked { blockers: Vec<PromotionBlocker> },
}

/// What the mutation boundary reads of a parent record: a lifecycle parent
/// usually has no journal row (fact 29), so this is where it stands. It holds
/// IDs, OIDs, the consent's digest and fixed categories.
pub(super) struct ParentView<B> {
    pub(super) step: lifecycle::ParentStep,
    /// What ended the record, when a consent refusal did.
    pub(super) refusal: Option<lifecycle_record::Refusal<B>>,
    /// The digest the record was begun with, which the same operation ID
    /// is called with again.
    pub(super) expectation: [u8; 32],
    pub(super) item_id: ItemId,
    /// The closure checkpoint the record holds. Its ref may not have moved
    /// while the step is `closing`.
    pub(super) checkpoint: Option<Oid>,
    pub(super) integration: Option<Oid>,
    pub(super) publication: Option<record::PublicationCategory>,
}

impl<B> ParentView<B> {
    /// The same view, with a refusal's blockers as `blocker` gives them.
    pub(super) fn of_blockers<C>(self, blocker: impl Fn(B) -> C) -> ParentView<C> {
        use lifecycle_record::Refusal;
        ParentView {
            step: self.step,
            refusal: self.refusal.map(|refusal| match refusal {
                Refusal::Blocked(blockers) => {
                    Refusal::Blocked(blockers.into_iter().map(blocker).collect())
                }
                Refusal::ExternalChange => Refusal::ExternalChange,
                Refusal::ReconfirmationRequired => Refusal::ReconfirmationRequired,
            }),
            expectation: self.expectation,
            item_id: self.item_id,
            checkpoint: self.checkpoint,
            integration: self.integration,
            publication: self.publication,
        }
    }
}

/// The parent record of the promotion `operation`, when there is one.
pub(super) fn lifecycle_parent(
    service: &RepositoryService,
    root: &Path,
    operation: OperationId,
) -> Result<Option<ParentView<PromotionBlocker>>, RepositoryError> {
    use lifecycle::LifecycleRecord;
    crate::repository::remote::state::with_transaction(service, root, |tx, repository_id| {
        Ok(
            record::read_record(tx, repository_id, operation)?.map(|parent| ParentView {
                step: parent.step(),
                refusal: parent.refusal().cloned(),
                expectation: parent.expectation().to_bytes(),
                item_id: parent.item_id().clone(),
                checkpoint: None,
                integration: parent.integration_oid(),
                publication: parent.publication(),
            }),
        )
    })
}

impl RepositoryService {
    pub fn promote_document<P: SessionCredentialProvider>(
        &self,
        request: PromoteDocumentRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<DocumentPromotionResult, DocumentPromotionError> {
        run::promote(self, request, session)
    }
    pub fn cancel_document_promotion(
        &self,
        root: &Path,
        id: OperationId,
    ) -> Result<bool, RepositoryError> {
        crate::repository::remote::state::with_transaction(self, root, |tx, repository_id| {
            record::request_cancellation(tx, repository_id, id)
        })
    }
    /// Observe local Git and stored publication history without a repository
    /// lease, network access, or a Git/registry mutation. Consent must be checked
    /// again against a fresh observation before any future integration.
    pub fn preview_document_promotion(
        &self,
        request: PreviewDocumentPromotionRequest,
    ) -> Result<DocumentPromotionPreflight, RepositoryError> {
        observe::preview(self, request)
    }
}
