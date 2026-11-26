//! Confirmed ticket closure: its read-only preflight, its parent record, the
//! closure checkpoint, and `close_ticket` over the one generic lifecycle
//! runner (C3).
use crate::canonical::{Integration, ItemId};
use crate::forge::PullRequestLink;
use crate::repository::context_retirement::proof::UnheldEntry;
use crate::repository::promotion::{
    lifecycle::CancellationProbe,
    run::{RunObserver, Unobserved},
};
use crate::repository::{
    ConfirmedCommitIdentity, ExpectedConfiguration, IndexPending, OperationId, PlannedCleanup,
    PlannedIntegration, PlannedPublication, PromotionCleanup, PromotionIntegration,
    PromotionPathChange, PromotionPublication, RepositoryError, RepositoryService,
    SynchronizationError,
    keys::{SessionCredentialProvider, SessionCredentials},
    remote::state,
    transport::{HostApproval, SshTransportError},
};
use git2::Oid;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

pub(super) mod checkpoint;
pub(super) mod lifecycle;
pub(super) mod new_branch;
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
    /// Last, so the stored blocker masks keep their bits.
    HistoryUnresolved,
    /// A pull-request close of a ticket with no context whose remote branch
    /// exists and is held by neither primary. A preflight blocker only: a
    /// confirm that meets it is `external_change`, so it is never stored and
    /// has no bit in `CLOSURE_BLOCKERS`.
    RemoteContextUnmaterialized,
}

/// What a closure record does to the ticket. With the integration mode it
/// is the record's path: `(direct, close)` is a direct record, every other
/// pair a branch record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureAction {
    Close,
    Withdraw,
    NewBranch,
}
impl ClosureAction {
    pub(in super::super) fn name(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Withdraw => "withdraw",
            Self::NewBranch => "new_branch",
        }
    }
    pub(in super::super) fn parse(text: &str) -> Result<Self, RepositoryError> {
        match text {
            "close" => Ok(Self::Close),
            "withdraw" => Ok(Self::Withdraw),
            "new_branch" => Ok(Self::NewBranch),
            _ => Err(state::recovery_required()),
        }
    }
}

/// How a finished branch record answers, stored as its `answer_code`. The
/// first nine, and a withdraw's `Withdrawn`, `WithdrawClosed` and
/// `ClosedSince`, are what a completed record answers after its publication
/// child; the rest are what an abandoned one answers with its refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchAnswer {
    Merged,
    ClosedElsewhere,
    RemoteBranchDeleted,
    TicketClosed,
    RemoteChanged,
    Existing,
    Published,
    KeptPublished,
    WithdrawnSince,
    AlreadyClosed,
    AlreadyClosing,
    ExternalChange,
    ReconfirmationRequired,
    ContextRemoteBranchDeleted,
    ContextHistoryUnresolved,
    ContextTicketClosed,
    /// A new branch's, abandoned: the old context's full check refused its
    /// removal (dirty, moved, locked, or holding a repository of its own).
    ContextChanged,
    /// A withdraw's, after its publication child: the closure is removed and
    /// published.
    Withdrawn,
    /// A withdraw's, after its publication child: the tracking primary holds
    /// a closure, so the ticket is closed.
    WithdrawClosed,
    /// A withdraw's, after its publication child: the published head holds a
    /// closure again.
    ClosedSince,
    /// A withdraw's, abandoned: the head holds no closure to withdraw.
    AlreadyApplied,
}
impl BranchAnswer {
    /// The stored value, which `parse` reads back.
    pub(in super::super) fn name(self) -> &'static str {
        match self {
            Self::Merged => "merged",
            Self::ClosedElsewhere => "closed_elsewhere",
            Self::RemoteBranchDeleted => "remote_branch_deleted",
            Self::TicketClosed => "ticket_closed",
            Self::RemoteChanged => "remote_changed",
            Self::Existing => "existing",
            Self::Published => "published",
            Self::KeptPublished => "kept_published",
            Self::WithdrawnSince => "withdrawn_since",
            Self::AlreadyClosed => "already_closed",
            Self::AlreadyClosing => "already_closing",
            Self::ExternalChange => "external_change",
            Self::ReconfirmationRequired => "reconfirmation_required",
            Self::ContextRemoteBranchDeleted => "context_remote_branch_deleted",
            Self::ContextHistoryUnresolved => "context_history_unresolved",
            Self::ContextTicketClosed => "context_ticket_closed",
            Self::ContextChanged => "context_changed",
            Self::Withdrawn => "withdrawn",
            Self::WithdrawClosed => "withdraw_closed",
            Self::ClosedSince => "closed_since",
            Self::AlreadyApplied => "already_applied",
        }
    }
    /// The stored value; any other text is recovery.
    pub(in super::super) fn parse(text: &str) -> Result<Self, RepositoryError> {
        match text {
            "merged" => Ok(Self::Merged),
            "closed_elsewhere" => Ok(Self::ClosedElsewhere),
            "remote_branch_deleted" => Ok(Self::RemoteBranchDeleted),
            "ticket_closed" => Ok(Self::TicketClosed),
            "remote_changed" => Ok(Self::RemoteChanged),
            "existing" => Ok(Self::Existing),
            "published" => Ok(Self::Published),
            "kept_published" => Ok(Self::KeptPublished),
            "withdrawn_since" => Ok(Self::WithdrawnSince),
            "already_closed" => Ok(Self::AlreadyClosed),
            "already_closing" => Ok(Self::AlreadyClosing),
            "external_change" => Ok(Self::ExternalChange),
            "reconfirmation_required" => Ok(Self::ReconfirmationRequired),
            "context_remote_branch_deleted" => Ok(Self::ContextRemoteBranchDeleted),
            "context_history_unresolved" => Ok(Self::ContextHistoryUnresolved),
            "context_ticket_closed" => Ok(Self::ContextTicketClosed),
            "context_changed" => Ok(Self::ContextChanged),
            "withdrawn" => Ok(Self::Withdrawn),
            "withdraw_closed" => Ok(Self::WithdrawClosed),
            "closed_since" => Ok(Self::ClosedSince),
            "already_applied" => Ok(Self::AlreadyApplied),
            _ => Err(state::recovery_required()),
        }
    }
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

/// What a new branch (Part F) retires and leaves behind, as its observation
/// read it. The digest binds all of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NewBranchFacts {
    /// The old context's tip `T`; none when the ticket has no context.
    pub(super) old_tip: Option<Oid>,
    /// What step 2 retires: `T`, or for no context the remote history's
    /// last published commit; none when nothing is retired.
    pub(super) retired_oid: Option<Oid>,
    /// What the old tip holds that the tracking primary lacks (R-19).
    pub(super) unheld: Vec<UnheldEntry>,
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
    path: (Integration, ClosureAction),
    new_branch: Option<NewBranchFacts>,
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
    /// The integration mode and action the closure runs as: `(direct,
    /// close)` is today's closure, any other pair a branch record.
    pub fn path(&self) -> (Integration, ClosureAction) {
        self.path
    }
    /// A new branch's old tip, retirement and unheld entries; `None` for
    /// every other path.
    pub(super) fn new_branch(&self) -> Option<&NewBranchFacts> {
        self.new_branch.as_ref()
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
            .field("path", &self.path)
            .field(
                "new_branch_unheld",
                &self.new_branch.as_ref().map(|facts| facts.unheld.len()),
            )
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

/// A withdraw's preflight: a close's two answers, and the two with nothing
/// to confirm.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // As `TicketClosurePreflight`: the ready preview by value.
pub enum ClosureWithdrawalPreflight {
    /// A withdraw to confirm. While an unfinished record that a new request
    /// cannot supersede holds the ticket, and no closure is found, it is
    /// ready with a placeholder closure: `PlannedClosure::Existing` with an
    /// empty `closed_at` and `closed_by`, and nothing to identify (ruling
    /// 35). The caller applies the hold (decision 33), which answers
    /// `recovery_required` naming the holder, before it shows or consents to
    /// a ready withdrawal.
    Ready(TicketClosurePreview),
    Blocked {
        /// Every blocker, in declaration order.
        blockers: Vec<ClosureBlocker>,
        /// `Some` exactly when `blockers` is `[IdentityRequired]`.
        identity_configuration: Option<ExpectedConfiguration>,
    },
    /// `already_applied`, with no data and nothing to confirm: neither the
    /// context's ticket nor its tracking ref's holds a closure.
    NothingToWithdraw,
    /// `publication_remote_required`: a withdraw's children publish.
    PublicationRemoteRequired,
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

/// What a finished branch record did with the closure: wrote one, kept the
/// one the ticket held, removed it, or wrote nothing (it was abandoned, or
/// answered before it got to the closure).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchRecording {
    Recorded,
    Existing,
    Withdrawn,
    Nothing,
}

/// A branch record's answer: a pull-request close never reaches the primary,
/// so it has no integration or cleanup to report. `Debug` shows IDs, OIDs and
/// categories only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchClosureOutcome {
    operation_id: OperationId,
    action: ClosureAction,
    answer: BranchAnswer,
    checkpoint_oid: Option<Oid>,
    publication: Option<PromotionPublication>,
    recording: BranchRecording,
    /// The closure's own timestamp, as the record stored it, for a record
    /// that completed holding one.
    closed_at: Option<String>,
    /// What a new branch retired of the old context, for one that completed.
    retired_oid: Option<Oid>,
}
impl BranchClosureOutcome {
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    pub fn action(&self) -> ClosureAction {
        self.action
    }
    pub fn answer(&self) -> BranchAnswer {
        self.answer
    }
    /// An outcome of the answer given, for the binding's tests.
    #[cfg(test)]
    pub(in super::super) fn for_testing(
        answer: BranchAnswer,
        checkpoint_oid: Option<Oid>,
        publication: Option<PromotionPublication>,
    ) -> Self {
        Self {
            operation_id: OperationId::new(),
            action: ClosureAction::Close,
            answer,
            checkpoint_oid,
            publication,
            recording: BranchRecording::Nothing,
            closed_at: None,
            retired_oid: None,
        }
    }
    /// The same outcome, as the action and the closure's parts a test names.
    #[cfg(test)]
    pub(in super::super) fn for_testing_with(
        mut self,
        action: ClosureAction,
        recording: BranchRecording,
        closed_at: Option<&str>,
        retired_oid: Option<Oid>,
    ) -> Self {
        self.action = action;
        self.recording = recording;
        self.closed_at = closed_at.map(str::to_owned);
        self.retired_oid = retired_oid;
        self
    }
    /// The commit the record wrote on the context, for a record that
    /// completed having written one: none for an abandoned record, which
    /// leaves no commit behind, or for one that found its closure made.
    pub fn checkpoint_oid(&self) -> Option<Oid> {
        self.checkpoint_oid
    }
    /// The publication child's result, when that child completed.
    pub fn publication(&self) -> Option<PromotionPublication> {
        self.publication
    }
    /// What the record did with the closure.
    pub fn recording(&self) -> BranchRecording {
        self.recording
    }
    /// The closure's `closed_at`, exactly as the record stored it: RFC 3339,
    /// UTC, whole seconds. None for a record that wrote or kept no closure
    /// of its own.
    pub fn closed_at(&self) -> Option<&str> {
        self.closed_at.as_deref()
    }
    /// The commit a new branch retired of the old context.
    pub fn retired_oid(&self) -> Option<Oid> {
        self.retired_oid
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TicketClosureResult {
    Complete(TicketClosureOutcome),
    IndexPending(IndexPending<TicketClosureOutcome>),
    /// A pull-request close, finished: whatever way it answered.
    Branch(BranchClosureOutcome),
    BranchIndexPending(IndexPending<BranchClosureOutcome>),
}

/// What the closure lifecycle's runner builds an outcome of: a direct
/// close's, or a branch record's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum ClosureOutcome {
    Direct(TicketClosureOutcome),
    Branch(BranchClosureOutcome),
}

/// What a branch record's publication answers with: the answer, the link to
/// give the item, and when it was given.
#[derive(Clone)]
pub(in super::super) struct BranchFinish {
    pub(in super::super) answer: BranchAnswer,
    pub(in super::super) link: Option<PullRequestLink>,
    pub(in super::super) given_at: i64,
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
    /// The preflight of a withdraw of the ticket's provisional closure
    /// (Part E): what `preview_ticket_closure` reads, observed as a withdraw
    /// in either mode, without a repository lease, network access, or a
    /// Git/registry mutation. A `Ready` answer may hold a placeholder
    /// closure with no identity (ruling 35: an unfinished record holds the
    /// ticket): the caller applies the hold (decision 33) before it shows or
    /// consents to it.
    pub fn preview_closure_withdrawal(
        &self,
        request: PreviewTicketClosureRequest,
    ) -> Result<ClosureWithdrawalPreflight, RepositoryError> {
        observe::withdrawal_preview(self, &request)
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
        lifecycle::close(
            self,
            request,
            session,
            None,
            &mut Unobserved,
            OffsetDateTime::now_utc,
        )
    }
    /// Withdraw a ticket's provisional closure as previewed
    /// (`preview_closure_withdrawal`): synchronize its context, write the
    /// commit that removes the closure and publish it. The same operation ID
    /// finishes an interrupted withdraw; a completed one replays its stored
    /// outcome.
    pub fn withdraw_ticket_closure<P: SessionCredentialProvider>(
        &self,
        request: CloseTicketRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        lifecycle::withdraw(
            self,
            request,
            session,
            None,
            &mut Unobserved,
            OffsetDateTime::now_utc,
        )
    }
    /// `close_ticket`, stopping at each safe point when `probe` says so, and
    /// telling `progress` of its steps (D5 decision 11).
    pub(super) fn close_ticket_observed<P: SessionCredentialProvider>(
        &self,
        request: CloseTicketRequest,
        session: &mut SessionCredentials<P>,
        probe: CancellationProbe,
        progress: &mut dyn RunObserver,
        clock: fn() -> OffsetDateTime,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        lifecycle::close(self, request, session, Some(probe), progress, clock)
    }
    /// `withdraw_ticket_closure`, stopping at each safe point when `probe` says
    /// so, and telling `progress` of its steps (D5 decision 11).
    pub(super) fn withdraw_ticket_closure_observed<P: SessionCredentialProvider>(
        &self,
        request: CloseTicketRequest,
        session: &mut SessionCredentials<P>,
        probe: CancellationProbe,
        progress: &mut dyn RunObserver,
        clock: fn() -> OffsetDateTime,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        lifecycle::withdraw(self, request, session, Some(probe), progress, clock)
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
