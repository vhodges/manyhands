//! Consent evidence and guarded primary integration for managed documents.
use super::{RepositoryError, RepositoryService};
use crate::canonical::ItemId;
use git2::Oid;
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[allow(dead_code)] // Internal integration stages are composed by the later lifecycle runner.
pub(super) mod integrate;
pub(super) mod observe;
pub(super) mod record;

/// A refusal before promotion has changed the primary. Integration uncertainty
/// is carried by the internal pending state, never by this error.
pub enum DocumentPromotionError {
    Repository(RepositoryError),
    Blocked(Vec<PromotionBlocker>),
    ExternalChange,
    ReconfirmationRequired,
    PrimaryAdvanced,
    IdentityRequired {
        expected_configuration: super::ExpectedConfiguration,
    },
}

impl fmt::Debug for DocumentPromotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository(error) => f.debug_tuple("Repository").field(&error.kind).finish(),
            Self::Blocked(blockers) => f.debug_tuple("Blocked").field(blockers).finish(),
            Self::ExternalChange => f.write_str("ExternalChange"),
            Self::ReconfirmationRequired => f.write_str("ReconfirmationRequired"),
            Self::PrimaryAdvanced => f.write_str("PrimaryAdvanced"),
            Self::IdentityRequired { .. } => f.write_str("IdentityRequired"),
        }
    }
}

impl fmt::Display for DocumentPromotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Repository(_) => "promotion repository unavailable",
            Self::Blocked(_) => "promotion blocked",
            Self::ExternalChange => "promotion state changed",
            Self::ReconfirmationRequired => "promotion reconfirmation required",
            Self::PrimaryAdvanced => "promotion primary advanced",
            Self::IdentityRequired { .. } => "promotion commit identity required",
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
    path: PathBuf,
    change: PathChange,
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

impl RepositoryService {
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
