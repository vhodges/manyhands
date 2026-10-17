//! The seam between the durable parent runner and the lifecycle it drives.
//! Document promotion is the only implementation at Cycle 10 Task 1; every
//! method below forwards to the promotion module's existing function.
#![allow(clippy::too_many_arguments)]
use super::{
    DocumentPromotionError, DocumentPromotionPreflight, DocumentPromotionPreview,
    PreviewDocumentPromotionRequest, PromotionExpectation, integrate, observe,
    record::{self, ChildExecution, ChildRole, ChildState, PromotionRecord},
};
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, ConfirmedCommitIdentity, OperationId, RemoteOperationAction,
        RepositoryError, RepositoryService, cleanup::RetirementEvidence,
        remote::reservation::RemoteReservation,
    },
};
use git2::Oid;
use rusqlite::{Connection, Transaction};
use std::path::Path;

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

/// The hook's input: what the integration stage holds under its lease.
#[allow(dead_code)] // Read by the closure checkpoint hook (Task 4).
pub(in super::super) struct LeasedIntegration<'a> {
    pub(in super::super) repository: &'a git2::Repository,
    pub(in super::super) root: &'a Path,
    pub(in super::super) context: &'a integrate::IntegrationContext,
    pub(in super::super) base: Oid,
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
    type Record;
    type Error;
    type Preview;
    type Expectation;
    type Preflight;
    type PreviewRequest;
    type Observation;
    type Refusal;
    type ReplacementGuard: PartialEq;
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
    fn preview(
        service: &RepositoryService,
        request: Self::PreviewRequest,
    ) -> Result<Self::Preflight, RepositoryError>;
    fn prepare(
        service: &RepositoryService,
        request: Self::PreviewRequest,
    ) -> Result<Self::Observation, RepositoryError>;

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
    fn prepare_integration(
        &self,
        service: &RepositoryService,
        root: &Path,
        request: &integrate::IntegrationRequest,
        identity: Option<&ConfirmedCommitIdentity>,
    ) -> Result<integrate::PreparedIntegration, Self::Error>;
    /// Runs inside the integration lease, after revalidation and before the
    /// result tree is chosen.
    fn pre_integration(
        &self,
        stage: &mut LeasedIntegration<'_>,
    ) -> Result<PreIntegration, Self::Error>;
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
    fn prepare_integration(
        &self,
        service: &RepositoryService,
        root: &Path,
        request: &integrate::IntegrationRequest,
        identity: Option<&ConfirmedCommitIdentity>,
    ) -> Result<integrate::PreparedIntegration, DocumentPromotionError> {
        integrate::prepare_integration_for(self, service, root, request, identity)
    }
    fn pre_integration(
        &self,
        _stage: &mut LeasedIntegration<'_>,
    ) -> Result<PreIntegration, DocumentPromotionError> {
        Ok(PreIntegration::None)
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
