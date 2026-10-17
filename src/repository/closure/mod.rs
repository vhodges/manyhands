//! Confirmed ticket closure. Cycle 10 Task 2 adds its parent record and Task
//! 3 its read-only preflight; the checkpoint and lifecycle arrive in later
//! tasks.
use crate::canonical::ItemId;
use crate::repository::{
    ConfirmedCommitIdentity, ExpectedConfiguration, PlannedCleanup, PlannedIntegration,
    PlannedPublication, PromotionPathChange, RepositoryError, RepositoryService,
};
use git2::Oid;
use std::{
    fmt,
    path::{Path, PathBuf},
};

pub(super) mod observe;
pub(super) mod record;

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
}
