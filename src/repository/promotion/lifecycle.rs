//! The seam between the one durable parent runner and the lifecycle it drives.
//! The runner reaches records, errors, observation and integration only
//! through these traits. Document promotion is the only implementation at
//! Cycle 10 Task 1; its methods forward to the promotion module's existing
//! functions and fields.
#![allow(clippy::too_many_arguments)]
use super::{
    DocumentPromotionError, DocumentPromotionOutcome, DocumentPromotionPreflight,
    DocumentPromotionPreview, DocumentPromotionResult, PlannedIntegration,
    PreviewDocumentPromotionRequest, PromotionCleanup, PromotionExpectation, PromotionIntegration,
    PromotionPublication, integrate, observe,
    record::{
        self, ChildExecution, ChildRole, ChildState, CleanupCategory, IntegrationKind,
        PromotionRecord, PromotionStep, PublicationCategory,
    },
};
use crate::{
    canonical::{ItemId, RepositoryConfig},
    repository::{
        AuthoringKind, ConfirmedCommitIdentity, IndexPending, OperationId, RemoteOperationAction,
        RepositoryError, RepositoryService, SynchronizationError, cleanup::RetirementEvidence,
        remote::reservation::RemoteReservation, transport::HostApproval,
    },
};
use git2::Oid;
use rusqlite::{Connection, Transaction};
use std::path::{Path, PathBuf};

/// Which parent lifecycle a runner, child fence or reservation arm serves.
#[allow(dead_code)] // TicketClosure is constructed by the closure lifecycle (Task 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum LifecycleKind {
    DocumentPromotion,
    TicketClosure,
}
impl LifecycleKind {
    pub(in super::super) fn authoring(self) -> AuthoringKind {
        match self {
            Self::DocumentPromotion => AuthoringKind::Document,
            Self::TicketClosure => AuthoringKind::Ticket,
        }
    }
    pub(in super::super) fn action(self) -> RemoteOperationAction {
        match self {
            Self::DocumentPromotion => RemoteOperationAction::Promote,
            Self::TicketClosure => RemoteOperationAction::Close,
        }
    }
    /// The merge commit subject, also the integration reconciliation key.
    pub(in super::super) fn merge_subject(self, item: &ItemId) -> String {
        match self {
            Self::DocumentPromotion => format!("Promote document {item}"),
            Self::TicketClosure => format!("Integrate closed ticket {item}"),
        }
    }
    pub(in super::super) fn context_ref(self, item: &ItemId) -> String {
        format!("refs/heads/{}", self.context_branch(item))
    }
    pub(in super::super) fn context_branch(self, item: &ItemId) -> String {
        match self {
            Self::DocumentPromotion => format!("manyhands/document/{item}"),
            Self::TicketClosure => format!("manyhands/ticket/{item}"),
        }
    }
    /// The canonical directory whose primary and context metadata the
    /// observation guard hashes.
    pub(in super::super) fn guarded_directory(self) -> &'static str {
        match self {
            Self::DocumentPromotion => "docs",
            Self::TicketClosure => ".manyhands/tickets",
        }
    }
}

/// The runner's kind-neutral request. Each lifecycle's public request
/// converts into it field for field.
#[derive(Clone)]
pub(in super::super) struct LifecycleRequest<E> {
    pub(in super::super) root: PathBuf,
    pub(in super::super) operation_id: OperationId,
    pub(in super::super) item_id: ItemId,
    pub(in super::super) expected: E,
    pub(in super::super) approval: Option<HostApproval>,
    pub(in super::super) confirmed_identity: Option<ConfirmedCommitIdentity>,
}

/// The parent steps the runner branches on, shared by every lifecycle. A
/// lifecycle's own extra steps (closure's `closing`) stay behind its
/// `pre_integration` hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ParentStep {
    Confirmed,
    ContextSynchronized,
    Integrating,
    Integrated,
    Published,
    PublicationPending,
    RemoteDeleted,
    WorktreeRemoving,
    WorktreeRemoved,
    BranchDeleted,
    Complete,
    Abandoned,
    Superseded,
}

/// Exactly the parent fields the runner reads and writes.
pub(in super::super) trait LifecycleRecord: Clone {
    type Expectation;
    type Refusal;
    fn repository_id(&self) -> i64;
    fn operation_id(&self) -> OperationId;
    fn item_id(&self) -> &ItemId;
    fn expectation(&self) -> Self::Expectation;
    fn remote_name(&self) -> Option<&str>;
    fn step(&self) -> ParentStep;
    fn set_step(&mut self, step: ParentStep);
    fn context_sync_id(&self) -> Option<OperationId>;
    fn primary_sync_id(&self) -> Option<OperationId>;
    fn base_oid(&self) -> Option<Oid>;
    fn set_base_oid(&mut self, oid: Option<Oid>);
    fn context_oid(&self) -> Option<Oid>;
    fn set_context_oid(&mut self, oid: Option<Oid>);
    fn integration_oid(&self) -> Option<Oid>;
    fn set_integration_oid(&mut self, oid: Option<Oid>);
    fn integration_kind(&self) -> Option<IntegrationKind>;
    fn set_integration_kind(&mut self, kind: Option<IntegrationKind>);
    fn remote_context_oid(&self) -> Option<Oid>;
    fn set_remote_context_oid(&mut self, oid: Option<Oid>);
    fn publication(&self) -> Option<PublicationCategory>;
    fn set_publication(&mut self, category: Option<PublicationCategory>);
    fn cleanup(&self) -> Option<CleanupCategory>;
    fn set_cleanup(&mut self, category: Option<CleanupCategory>);
    fn cancel_requested(&self) -> bool;
    fn set_cancel_requested(&mut self, requested: bool);
    fn refusal(&self) -> Option<&Self::Refusal>;
    fn set_refusal(&mut self, refusal: Option<Self::Refusal>);
}

/// Exactly the error variants the runner produces or matches.
pub(in super::super) trait LifecycleError: From<RepositoryError> {
    fn busy() -> Self;
    fn operation_mismatch() -> Self;
    fn recovery_required() -> Self;
    fn interrupted() -> Self;
    fn primary_advanced() -> Self;
    fn context_synchronization(error: Box<SynchronizationError>) -> Self;
    fn is_primary_advanced(&self) -> bool;
    /// `Blocked`, `ReconfirmationRequired` or `ExternalChange`: a refusal of
    /// the consented effect, abandoned before integration.
    fn is_consent_refusal(&self) -> bool;
}

/// The two refusals the runner records itself; `Blocked` comes from `ready`.
pub(in super::super) trait LifecycleRefusal: Clone {
    fn external_change() -> Self;
    fn reconfirmation_required() -> Self;
}

/// The preview facts the runner reads.
pub(in super::super) trait LifecyclePreview {
    type Expectation;
    fn expectation(&self) -> Self::Expectation;
    fn context_oid(&self) -> Oid;
    fn tracking_context(&self) -> Option<Oid>;
    fn integration(&self) -> PlannedIntegration;
}

/// The hook's input: what the integration stage holds under its lease. The
/// operation and record are absent only for an integration called outside
/// the runner (unit tests).
#[allow(dead_code)] // Read by the closure checkpoint hook (Task 4).
pub(in super::super) struct LeasedIntegration<'a, R> {
    pub(in super::super) repository: &'a git2::Repository,
    pub(in super::super) root: &'a Path,
    pub(in super::super) context: &'a integrate::IntegrationContext,
    pub(in super::super) base: Oid,
    pub(in super::super) operation_id: Option<OperationId>,
    pub(in super::super) record: Option<&'a R>,
}

/// The hook's output. `Checkpoint` names the context commit to merge in place
/// of the observed context tip.
#[allow(dead_code)] // Checkpoint is returned by the closure hook (Task 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum PreIntegration {
    None,
    Checkpoint { merged_context: Oid },
}

/// One method per record, observation and integration function the runner
/// calls, with the promotion argument and return types translated to the
/// associated types.
pub(in super::super) trait Lifecycle {
    type Record: LifecycleRecord<Expectation = Self::Expectation, Refusal = Self::Refusal>;
    type Error: LifecycleError;
    type Preview: LifecyclePreview<Expectation = Self::Expectation>;
    type Expectation: Copy + PartialEq;
    type Preflight;
    type PreviewRequest;
    type Observation;
    type Refusal: LifecycleRefusal;
    type ReplacementGuard: PartialEq;
    type IntegrationRequest;
    type Outcome;
    type Output;
    const KIND: LifecycleKind;

    // Record storage.
    fn read_record(
        db: &Connection,
        id: i64,
        op: OperationId,
    ) -> Result<Option<Self::Record>, RepositoryError>;
    fn save_record(tx: &Transaction<'_>, id: i64, r: &Self::Record) -> Result<(), RepositoryError>;
    fn begin_record(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        item: &ItemId,
        expectation: Self::Expectation,
        remote: Option<&str>,
    ) -> Result<Self::Record, RepositoryError>;
    fn replacement_guard(
        tx: &Transaction<'_>,
        id: i64,
        item: &ItemId,
    ) -> Result<Self::ReplacementGuard, RepositoryError>;
    fn refuse_record(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        item: &ItemId,
        expectation: Self::Expectation,
        remote: Option<&str>,
        refusal: &Self::Refusal,
    ) -> Result<Self::Record, RepositoryError>;
    fn clear_cancellation(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
    ) -> Result<(), RepositoryError>;
    fn reset_context_after_primary_advance(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
    ) -> Result<Self::Record, RepositoryError>;
    fn child_state(
        db: &Connection,
        r: &Self::Record,
        role: ChildRole,
    ) -> Result<ChildState, RepositoryError>;
    fn child_execution(
        r: &Self::Record,
        role: ChildRole,
    ) -> Result<ChildExecution, RepositoryError>;
    fn next_attempt(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        role: ChildRole,
    ) -> Result<Self::Record, RepositoryError>;
    fn complete_record(
        tx: &Transaction<'_>,
        service: &RepositoryService,
        id: i64,
        op: OperationId,
        evidence: &RetirementEvidence<'_>,
        owner: Option<&RemoteReservation>,
    ) -> Result<(), RepositoryError>;
    fn reconcile_completion(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
    ) -> Result<bool, RepositoryError>;

    // Observation.
    fn preview_request(root: PathBuf, item_id: ItemId) -> Self::PreviewRequest;
    fn preview(
        service: &RepositoryService,
        request: Self::PreviewRequest,
    ) -> Result<Self::Preflight, RepositoryError>;
    fn prepare(
        service: &RepositoryService,
        request: Self::PreviewRequest,
    ) -> Result<Self::Observation, RepositoryError>;
    fn configuration(observation: &Self::Observation) -> &RepositoryConfig;
    fn revalidate(
        observation: &Self::Observation,
        service: &RepositoryService,
        repository: &git2::Repository,
    ) -> Result<Self::Preflight, RepositoryError>;
    /// A ready preview, or the `Blocked` refusal of a blocked preflight.
    fn ready(preflight: Self::Preflight) -> Result<Self::Preview, Self::Refusal>;
    fn refusal_error(refusal: &Self::Refusal) -> Self::Error;

    // Integration.
    fn reconcile_integration(
        repository: &git2::Repository,
        base: Oid,
        context: &integrate::IntegrationContext,
    ) -> integrate::IntegrationState {
        integrate::reconcile_integration_for(Self::KIND, repository, base, context)
    }
    fn primary_clean(root: &Path) -> bool {
        integrate::primary_clean(root)
    }
    /// The preview's integration request, bound to the recorded consent and
    /// the exact synchronized context.
    fn integration_request(
        preview: &Self::Preview,
        expected: Self::Expectation,
        context: Oid,
    ) -> Self::IntegrationRequest;
    fn prepare_integration(
        &self,
        service: &RepositoryService,
        root: &Path,
        request: &Self::IntegrationRequest,
        identity: Option<&ConfirmedCommitIdentity>,
        parent: Option<&Self::Record>,
    ) -> Result<integrate::PreparedIntegration, Self::Error>;
    fn write_commit(
        prepared: integrate::PreparedIntegration,
    ) -> Result<integrate::IntegrationCandidate, Self::Error>;
    fn resume_candidate(
        prepared: integrate::PreparedIntegration,
        oid: Oid,
    ) -> Result<integrate::IntegrationCandidate, Self::Error>;
    fn apply(
        candidate: integrate::IntegrationCandidate,
    ) -> Result<integrate::Integrated, Self::Error>;
    /// Runs inside the integration lease, after revalidation and before the
    /// result tree is chosen.
    fn pre_integration(
        &self,
        stage: &mut LeasedIntegration<'_, Self::Record>,
    ) -> Result<PreIntegration, Self::Error>;

    // Outcome.
    fn outcome(
        r: &Self::Record,
        integration: PromotionIntegration,
        publication: PromotionPublication,
        cleanup: PromotionCleanup,
    ) -> Self::Outcome;
    fn complete(outcome: Self::Outcome) -> Self::Output;
    fn index_pending(outcome: Self::Outcome) -> Self::Output;
}

impl From<PromotionStep> for ParentStep {
    fn from(step: PromotionStep) -> Self {
        match step {
            PromotionStep::Confirmed => Self::Confirmed,
            PromotionStep::ContextSynchronized => Self::ContextSynchronized,
            PromotionStep::Integrating => Self::Integrating,
            PromotionStep::Integrated => Self::Integrated,
            PromotionStep::Published => Self::Published,
            PromotionStep::PublicationPending => Self::PublicationPending,
            PromotionStep::RemoteDeleted => Self::RemoteDeleted,
            PromotionStep::WorktreeRemoving => Self::WorktreeRemoving,
            PromotionStep::WorktreeRemoved => Self::WorktreeRemoved,
            PromotionStep::BranchDeleted => Self::BranchDeleted,
            PromotionStep::Complete => Self::Complete,
            PromotionStep::Abandoned => Self::Abandoned,
            PromotionStep::Superseded => Self::Superseded,
        }
    }
}
impl From<ParentStep> for PromotionStep {
    fn from(step: ParentStep) -> Self {
        match step {
            ParentStep::Confirmed => Self::Confirmed,
            ParentStep::ContextSynchronized => Self::ContextSynchronized,
            ParentStep::Integrating => Self::Integrating,
            ParentStep::Integrated => Self::Integrated,
            ParentStep::Published => Self::Published,
            ParentStep::PublicationPending => Self::PublicationPending,
            ParentStep::RemoteDeleted => Self::RemoteDeleted,
            ParentStep::WorktreeRemoving => Self::WorktreeRemoving,
            ParentStep::WorktreeRemoved => Self::WorktreeRemoved,
            ParentStep::BranchDeleted => Self::BranchDeleted,
            ParentStep::Complete => Self::Complete,
            ParentStep::Abandoned => Self::Abandoned,
            ParentStep::Superseded => Self::Superseded,
        }
    }
}

impl LifecycleRecord for PromotionRecord {
    type Expectation = PromotionExpectation;
    type Refusal = record::Refusal;
    fn repository_id(&self) -> i64 {
        self.repository_id
    }
    fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    fn item_id(&self) -> &ItemId {
        &self.item_id
    }
    fn expectation(&self) -> PromotionExpectation {
        self.expectation
    }
    fn remote_name(&self) -> Option<&str> {
        self.remote_name.as_deref()
    }
    fn step(&self) -> ParentStep {
        self.step.into()
    }
    fn set_step(&mut self, step: ParentStep) {
        self.step = step.into();
    }
    fn context_sync_id(&self) -> Option<OperationId> {
        self.context_sync_id
    }
    fn primary_sync_id(&self) -> Option<OperationId> {
        self.primary_sync_id
    }
    fn base_oid(&self) -> Option<Oid> {
        self.base_oid
    }
    fn set_base_oid(&mut self, oid: Option<Oid>) {
        self.base_oid = oid;
    }
    fn context_oid(&self) -> Option<Oid> {
        self.context_oid
    }
    fn set_context_oid(&mut self, oid: Option<Oid>) {
        self.context_oid = oid;
    }
    fn integration_oid(&self) -> Option<Oid> {
        self.integration_oid
    }
    fn set_integration_oid(&mut self, oid: Option<Oid>) {
        self.integration_oid = oid;
    }
    fn integration_kind(&self) -> Option<IntegrationKind> {
        self.integration_kind
    }
    fn set_integration_kind(&mut self, kind: Option<IntegrationKind>) {
        self.integration_kind = kind;
    }
    fn remote_context_oid(&self) -> Option<Oid> {
        self.remote_context_oid
    }
    fn set_remote_context_oid(&mut self, oid: Option<Oid>) {
        self.remote_context_oid = oid;
    }
    fn publication(&self) -> Option<PublicationCategory> {
        self.publication
    }
    fn set_publication(&mut self, category: Option<PublicationCategory>) {
        self.publication = category;
    }
    fn cleanup(&self) -> Option<CleanupCategory> {
        self.cleanup
    }
    fn set_cleanup(&mut self, category: Option<CleanupCategory>) {
        self.cleanup = category;
    }
    fn cancel_requested(&self) -> bool {
        self.cancel_requested
    }
    fn set_cancel_requested(&mut self, requested: bool) {
        self.cancel_requested = requested;
    }
    fn refusal(&self) -> Option<&record::Refusal> {
        self.refusal.as_ref()
    }
    fn set_refusal(&mut self, refusal: Option<record::Refusal>) {
        self.refusal = refusal;
    }
}

impl LifecycleError for DocumentPromotionError {
    fn busy() -> Self {
        Self::Busy
    }
    fn operation_mismatch() -> Self {
        Self::OperationMismatch
    }
    fn recovery_required() -> Self {
        Self::RecoveryRequired
    }
    fn interrupted() -> Self {
        Self::Interrupted
    }
    fn primary_advanced() -> Self {
        Self::PrimaryAdvanced
    }
    fn context_synchronization(error: Box<SynchronizationError>) -> Self {
        Self::ContextSynchronization(error)
    }
    fn is_primary_advanced(&self) -> bool {
        matches!(self, Self::PrimaryAdvanced)
    }
    fn is_consent_refusal(&self) -> bool {
        matches!(
            self,
            Self::Blocked(_) | Self::ReconfirmationRequired | Self::ExternalChange
        )
    }
}

impl LifecycleRefusal for record::Refusal {
    fn external_change() -> Self {
        Self::ExternalChange
    }
    fn reconfirmation_required() -> Self {
        Self::ReconfirmationRequired
    }
}

impl LifecyclePreview for DocumentPromotionPreview {
    type Expectation = PromotionExpectation;
    fn expectation(&self) -> PromotionExpectation {
        self.expectation
    }
    fn context_oid(&self) -> Oid {
        self.context_oid
    }
    fn tracking_context(&self) -> Option<Oid> {
        self.tracking_context
    }
    fn integration(&self) -> PlannedIntegration {
        self.integration
    }
}

pub(in super::super) struct DocumentPromotionLifecycle;

impl Lifecycle for DocumentPromotionLifecycle {
    type Record = PromotionRecord;
    type Error = DocumentPromotionError;
    type Preview = DocumentPromotionPreview;
    type Expectation = PromotionExpectation;
    type Preflight = DocumentPromotionPreflight;
    type PreviewRequest = PreviewDocumentPromotionRequest;
    type Observation = observe::PreparedObservation;
    type Refusal = record::Refusal;
    type ReplacementGuard = record::ReplacementGuard;
    type IntegrationRequest = integrate::IntegrationRequest;
    type Outcome = DocumentPromotionOutcome;
    type Output = DocumentPromotionResult;
    const KIND: LifecycleKind = LifecycleKind::DocumentPromotion;

    fn read_record(
        db: &Connection,
        id: i64,
        op: OperationId,
    ) -> Result<Option<PromotionRecord>, RepositoryError> {
        record::read_record(db, id, op)
    }
    fn save_record(
        tx: &Transaction<'_>,
        id: i64,
        r: &PromotionRecord,
    ) -> Result<(), RepositoryError> {
        record::save_record(tx, id, r)
    }
    fn begin_record(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        item: &ItemId,
        expectation: PromotionExpectation,
        remote: Option<&str>,
    ) -> Result<PromotionRecord, RepositoryError> {
        record::begin_record(tx, id, op, item, expectation, remote)
    }
    fn replacement_guard(
        tx: &Transaction<'_>,
        id: i64,
        item: &ItemId,
    ) -> Result<record::ReplacementGuard, RepositoryError> {
        record::replacement_guard(tx, id, item)
    }
    fn refuse_record(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        item: &ItemId,
        expectation: PromotionExpectation,
        remote: Option<&str>,
        refusal: &record::Refusal,
    ) -> Result<PromotionRecord, RepositoryError> {
        record::refuse_record(tx, id, op, item, expectation, remote, refusal)
    }
    fn clear_cancellation(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
    ) -> Result<(), RepositoryError> {
        record::clear_cancellation(tx, id, op)
    }
    fn reset_context_after_primary_advance(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
    ) -> Result<PromotionRecord, RepositoryError> {
        record::reset_context_after_primary_advance(tx, id, op)
    }
    fn child_state(
        db: &Connection,
        r: &PromotionRecord,
        role: ChildRole,
    ) -> Result<ChildState, RepositoryError> {
        record::child_state(db, r, role)
    }
    fn child_execution(
        r: &PromotionRecord,
        role: ChildRole,
    ) -> Result<ChildExecution, RepositoryError> {
        record::child_execution(r, role)
    }
    fn next_attempt(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        role: ChildRole,
    ) -> Result<PromotionRecord, RepositoryError> {
        record::next_attempt(tx, id, op, role)
    }
    fn complete_record(
        tx: &Transaction<'_>,
        service: &RepositoryService,
        id: i64,
        op: OperationId,
        evidence: &RetirementEvidence<'_>,
        owner: Option<&RemoteReservation>,
    ) -> Result<(), RepositoryError> {
        record::complete_record(tx, service, id, op, evidence, owner)
    }
    fn reconcile_completion(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
    ) -> Result<bool, RepositoryError> {
        record::reconcile_completion(tx, id, op)
    }
    fn preview_request(root: PathBuf, item_id: ItemId) -> PreviewDocumentPromotionRequest {
        PreviewDocumentPromotionRequest { root, item_id }
    }
    fn preview(
        service: &RepositoryService,
        request: PreviewDocumentPromotionRequest,
    ) -> Result<DocumentPromotionPreflight, RepositoryError> {
        observe::preview(service, request)
    }
    fn prepare(
        service: &RepositoryService,
        request: PreviewDocumentPromotionRequest,
    ) -> Result<observe::PreparedObservation, RepositoryError> {
        observe::prepare(service, request)
    }
    fn configuration(observation: &observe::PreparedObservation) -> &RepositoryConfig {
        &observation.configuration
    }
    fn revalidate(
        observation: &observe::PreparedObservation,
        service: &RepositoryService,
        repository: &git2::Repository,
    ) -> Result<DocumentPromotionPreflight, RepositoryError> {
        observation.revalidate(service, repository)
    }
    fn ready(
        preflight: DocumentPromotionPreflight,
    ) -> Result<DocumentPromotionPreview, record::Refusal> {
        match preflight {
            DocumentPromotionPreflight::Ready(preview) => Ok(preview),
            DocumentPromotionPreflight::Blocked { blockers } => {
                Err(record::Refusal::Blocked(blockers))
            }
        }
    }
    fn refusal_error(refusal: &record::Refusal) -> DocumentPromotionError {
        match refusal {
            record::Refusal::Blocked(b) => DocumentPromotionError::Blocked(b.clone()),
            record::Refusal::ExternalChange => DocumentPromotionError::ExternalChange,
            record::Refusal::ReconfirmationRequired => {
                DocumentPromotionError::ReconfirmationRequired
            }
        }
    }
    fn integration_request(
        preview: &DocumentPromotionPreview,
        expected: PromotionExpectation,
        context: Oid,
    ) -> integrate::IntegrationRequest {
        let mut input = integrate::IntegrationRequest::from_preview(preview);
        input.expected = expected;
        input.context.oid = context;
        input
    }
    fn prepare_integration(
        &self,
        service: &RepositoryService,
        root: &Path,
        request: &integrate::IntegrationRequest,
        identity: Option<&ConfirmedCommitIdentity>,
        parent: Option<&PromotionRecord>,
    ) -> Result<integrate::PreparedIntegration, DocumentPromotionError> {
        integrate::prepare_integration_for(self, service, root, request, identity, parent)
    }
    fn write_commit(
        prepared: integrate::PreparedIntegration,
    ) -> Result<integrate::IntegrationCandidate, DocumentPromotionError> {
        prepared.write_commit()
    }
    fn resume_candidate(
        prepared: integrate::PreparedIntegration,
        oid: Oid,
    ) -> Result<integrate::IntegrationCandidate, DocumentPromotionError> {
        prepared.resume_candidate(oid)
    }
    fn apply(
        candidate: integrate::IntegrationCandidate,
    ) -> Result<integrate::Integrated, DocumentPromotionError> {
        candidate.apply()
    }
    fn pre_integration(
        &self,
        _stage: &mut LeasedIntegration<'_, PromotionRecord>,
    ) -> Result<PreIntegration, DocumentPromotionError> {
        Ok(PreIntegration::None)
    }
    fn outcome(
        r: &PromotionRecord,
        integration: PromotionIntegration,
        publication: PromotionPublication,
        cleanup: PromotionCleanup,
    ) -> DocumentPromotionOutcome {
        DocumentPromotionOutcome {
            operation_id: r.operation_id,
            integration,
            publication,
            cleanup,
        }
    }
    fn complete(outcome: DocumentPromotionOutcome) -> DocumentPromotionResult {
        DocumentPromotionResult::Complete(outcome)
    }
    fn index_pending(outcome: DocumentPromotionOutcome) -> DocumentPromotionResult {
        DocumentPromotionResult::IndexPending(IndexPending::new(outcome))
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
