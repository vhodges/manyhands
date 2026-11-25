//! Ticket closure over the one generic lifecycle runner (C3). Every method
//! forwards to the closure's record, observation and checkpoint modules; the
//! runner owns the order, the children, retry and cleanup.
use super::{
    CloseTicketRequest, ClosureAction, ClosureBlocker, ClosureExpectation, ClosureRecording,
    PlannedClosure, PreviewTicketClosureRequest, TicketClosureError, TicketClosureOutcome,
    TicketClosureResult, checkpoint,
    observe::{self, ClosureObservation, PreparedClosureObservation},
    record::{self, ClosureKind, ClosureRecord},
};
use crate::{
    canonical::{Integration, ItemId, RepositoryConfig},
    repository::{
        ConfirmedCommitIdentity, ExpectedConfiguration, IndexPending, OperationId,
        PlannedIntegration, PromotionCleanup, PromotionIntegration, PromotionPublication,
        RepositoryError, RepositoryService, SynchronizationError,
        cleanup::RetirementEvidence,
        keys::{SessionCredentialProvider, SessionCredentials},
        promotion::{
            integrate,
            lifecycle::{
                CancellationProbe, LeasedIntegration, Lifecycle, LifecycleError, LifecycleKind,
                LifecyclePreview, LifecycleRequest, PreIntegration,
            },
            record::{ChildExecution, ChildRole, ChildState},
            run,
        },
        remote::{reservation::RemoteReservation, state},
    },
};
use git2::Oid;
use rusqlite::{Connection, Transaction};
use std::path::{Path, PathBuf};

pub(in super::super) fn close<P: SessionCredentialProvider>(
    service: &RepositoryService,
    request: CloseTicketRequest,
    session: &mut SessionCredentials<P>,
    probe: Option<CancellationProbe>,
    progress: &mut dyn run::RunObserver,
) -> Result<TicketClosureResult, TicketClosureError> {
    run::run_lifecycle(
        &TicketClosureLifecycle,
        service,
        LifecycleRequest {
            probe,
            ..lifecycle_request(request)
        },
        session,
        progress,
    )
}

pub(in super::super) fn lifecycle_request(
    request: CloseTicketRequest,
) -> LifecycleRequest<ClosureExpectation> {
    let CloseTicketRequest {
        root,
        operation_id,
        item_id,
        expected,
        approval,
        confirmed_identity,
    } = request;
    LifecycleRequest {
        root,
        operation_id,
        item_id,
        expected,
        approval,
        confirmed_identity,
        probe: None,
    }
}

/// What a closure's observation is asked for: the request, and the mode to
/// observe in, `None` being the configuration's. A new request observes in
/// the configuration's mode; a continuation of a record, in the record's.
pub(in super::super) struct ClosurePreviewRequest {
    pub(in super::super) request: PreviewTicketClosureRequest,
    pub(in super::super) mode: Option<Integration>,
}

/// The path of a direct record, today's closure.
const DIRECT: (Integration, ClosureAction) = (Integration::Direct, ClosureAction::Close);

/// The closure effect a new record is inserted with: the observed one.
fn kind_of(observation: &ClosureObservation) -> ClosureKind {
    match observation.closure {
        PlannedClosure::Record { .. } => ClosureKind::Record,
        PlannedClosure::Existing { .. } => ClosureKind::Existing,
    }
}

impl LifecycleError for TicketClosureError {
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
    fn external_change() -> Self {
        Self::ExternalChange
    }
    fn reconfirmation_required() -> Self {
        Self::ReconfirmationRequired
    }
    fn identity_required(expected_configuration: ExpectedConfiguration) -> Self {
        Self::IdentityRequired {
            expected_configuration,
        }
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

/// The runner reads the closure's own observation as its preview: the
/// checkpoint hook consumes the same value the consent check revalidated.
impl LifecyclePreview for ClosureObservation {
    type Expectation = ClosureExpectation;
    fn expectation(&self) -> ClosureExpectation {
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
    fn primary_branch(&self) -> &str {
        &self.primary_branch
    }
    fn publication_remote(&self) -> Option<&str> {
        self.remote.as_deref()
    }
    fn primary_oid(&self) -> Oid {
        self.primary_oid
    }
    fn tracking_primary(&self) -> Option<Oid> {
        self.tracking_primary
    }
    /// A contained context is never synchronized: the remote context holds
    /// nothing beyond the base, and the checkpoint is never pushed (C1).
    fn runs_context_child(&self) -> bool {
        self.needs_context_child
    }
}

pub(in super::super) struct TicketClosureLifecycle;

impl Lifecycle for TicketClosureLifecycle {
    type Record = ClosureRecord;
    type Error = TicketClosureError;
    type Preview = ClosureObservation;
    type Expectation = ClosureExpectation;
    type Preflight = Result<ClosureObservation, Vec<ClosureBlocker>>;
    type PreviewRequest = ClosurePreviewRequest;
    type Observation = PreparedClosureObservation;
    type Refusal = record::Refusal;
    type ReplacementGuard = record::ReplacementGuard;
    type Outcome = TicketClosureOutcome;
    type Output = TicketClosureResult;
    const KIND: LifecycleKind = LifecycleKind::TicketClosure;

    fn read_record(
        db: &Connection,
        id: i64,
        op: OperationId,
    ) -> Result<Option<ClosureRecord>, RepositoryError> {
        record::read_record(db, id, op)
    }
    fn save_record(
        tx: &Transaction<'_>,
        id: i64,
        r: &ClosureRecord,
    ) -> Result<(), RepositoryError> {
        record::save_record(tx, id, r)
    }
    /// A new record takes its closure effect from the revalidated
    /// observation (R-T2-1). Without one the record must already exist:
    /// `begin_record` then returns it, validated, before any insert.
    fn begin_record(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        item: &ItemId,
        expectation: ClosureExpectation,
        remote: Option<&str>,
        observed: Option<&ClosureObservation>,
    ) -> Result<ClosureRecord, RepositoryError> {
        let kind = match observed {
            Some(observation) => kind_of(observation),
            // `begin_record` returns the existing record before it would use
            // the kind, so a first-write refusal without one is never
            // inserted again.
            None => match record::read_record(tx, id, op)? {
                Some(existing) => existing.closure_kind.unwrap_or(ClosureKind::Record),
                None => return Err(state::recovery_required()),
            },
        };
        match observed.map(|observation| observation.path) {
            Some(path) if path != DIRECT => record::begin_branch_record(
                tx,
                id,
                op,
                item,
                expectation,
                remote,
                kind,
                path,
                observed.map(|observation| observation.context_oid),
            ),
            _ => record::begin_record(tx, id, op, item, expectation, remote, kind),
        }
    }
    fn replacement_guard(
        tx: &Transaction<'_>,
        id: i64,
        item: &ItemId,
    ) -> Result<record::ReplacementGuard, RepositoryError> {
        record::replacement_guard(tx, id, item)
    }
    /// `closure_kind` is absent only for a refusal recorded before any
    /// closure effect was observed (R-T2-1).
    fn refuse_record(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        item: &ItemId,
        expectation: ClosureExpectation,
        config: &RepositoryConfig,
        observed: Option<&ClosureObservation>,
        refusal: &record::Refusal,
    ) -> Result<ClosureRecord, RepositoryError> {
        let remote = config.publication_remote.as_deref();
        let kind = observed.map(kind_of);
        // The refused request's own path: the observed one, else the
        // configuration's, which a new request always observes in (R25).
        match observed.map_or((config.integration(), ClosureAction::Close), |o| o.path) {
            DIRECT => record::refuse_record(tx, id, op, item, expectation, remote, kind, refusal),
            path => {
                record::refuse_record_on(tx, id, op, item, expectation, remote, kind, refusal, path)
            }
        }
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
    ) -> Result<ClosureRecord, RepositoryError> {
        record::reset_context_after_primary_advance(tx, id, op)
    }
    fn child_state(
        db: &Connection,
        r: &ClosureRecord,
        role: ChildRole,
    ) -> Result<ChildState, RepositoryError> {
        record::child_state(db, r, role)
    }
    fn child_execution(
        r: &ClosureRecord,
        role: ChildRole,
    ) -> Result<ChildExecution, RepositoryError> {
        record::child_execution(r, role)
    }
    fn next_attempt(
        tx: &Transaction<'_>,
        id: i64,
        op: OperationId,
        role: ChildRole,
    ) -> Result<ClosureRecord, RepositoryError> {
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

    fn preview_request(
        root: PathBuf,
        item_id: ItemId,
        identity: Option<&ConfirmedCommitIdentity>,
        record: Option<&ClosureRecord>,
    ) -> ClosurePreviewRequest {
        ClosurePreviewRequest {
            request: PreviewTicketClosureRequest {
                root,
                item_id,
                confirmed_identity: identity.cloned(),
            },
            mode: record.map(|record| record.integration_mode),
        }
    }
    fn preview(
        service: &RepositoryService,
        request: ClosurePreviewRequest,
    ) -> Result<Result<ClosureObservation, Vec<ClosureBlocker>>, RepositoryError> {
        Ok(
            observe::prepare_observation_in(service, &request.request, request.mode)?
                .observed
                .map_err(|refusal| refusal.blockers),
        )
    }
    fn prepare(
        service: &RepositoryService,
        request: ClosurePreviewRequest,
    ) -> Result<PreparedClosureObservation, RepositoryError> {
        observe::prepare_observation_in(service, &request.request, request.mode)
    }
    fn configuration(observation: &PreparedClosureObservation) -> &RepositoryConfig {
        &observation.configuration
    }
    fn revalidate(
        observation: &PreparedClosureObservation,
        service: &RepositoryService,
        repository: &git2::Repository,
    ) -> Result<Result<ClosureObservation, Vec<ClosureBlocker>>, RepositoryError> {
        observe::revalidate_under_lease(observation, service, repository)
    }
    /// A `Record` closure whose identity is gone since the preview has
    /// changed effective identity (C2): `ExternalChange`, never
    /// `Blocked([IdentityRequired])`.
    fn ready(
        preflight: Result<ClosureObservation, Vec<ClosureBlocker>>,
    ) -> Result<ClosureObservation, record::Refusal> {
        match preflight {
            Ok(observation) => Ok(observation),
            Err(blockers) if blockers == [ClosureBlocker::IdentityRequired] => {
                Err(record::Refusal::ExternalChange)
            }
            Err(blockers) => Err(record::Refusal::Blocked(blockers)),
        }
    }
    /// A pull-request close writes no record until its runner exists: a
    /// confirm whose consent holds stops here, before a record is inserted.
    fn require_identity(
        preview: &ClosureObservation,
        _repository: &git2::Repository,
        _confirmation: Option<&ConfirmedCommitIdentity>,
    ) -> Result<(), TicketClosureError> {
        if preview.path == DIRECT {
            Ok(())
        } else {
            Err(TicketClosureError::RecoveryRequired)
        }
    }
    fn refusal_error(refusal: &record::Refusal) -> TicketClosureError {
        match refusal {
            record::Refusal::Blocked(b) => TicketClosureError::Blocked(b.clone()),
            record::Refusal::ExternalChange => TicketClosureError::ExternalChange,
            record::Refusal::ReconfirmationRequired => TicketClosureError::ReconfirmationRequired,
        }
    }

    fn integration_request(
        preview: &ClosureObservation,
        expected: ClosureExpectation,
        context: Oid,
    ) -> integrate::IntegrationRequest<ClosureExpectation> {
        integrate::IntegrationRequest {
            expected,
            context: integrate::IntegrationContext {
                item_id: preview.item_id.clone(),
                primary_branch: preview.primary_branch.clone(),
                oid: context,
            },
            primary_oid: preview.primary_oid,
            publication_remote: preview.remote.clone(),
        }
    }
    fn prepare_integration(
        &self,
        service: &RepositoryService,
        root: &Path,
        request: &integrate::IntegrationRequest<ClosureExpectation>,
        identity: Option<&ConfirmedCommitIdentity>,
        parent: Option<&ClosureRecord>,
    ) -> Result<integrate::PreparedIntegration, TicketClosureError> {
        integrate::prepare_integration_for(self, service, root, request, identity, parent)
    }
    fn write_commit(
        prepared: integrate::PreparedIntegration,
    ) -> Result<integrate::IntegrationCandidate, TicketClosureError> {
        prepared.write_commit_for()
    }
    fn resume_candidate(
        prepared: integrate::PreparedIntegration,
        oid: Oid,
    ) -> Result<integrate::IntegrationCandidate, TicketClosureError> {
        prepared.resume_candidate_for(oid)
    }
    fn apply(
        candidate: integrate::IntegrationCandidate,
    ) -> Result<integrate::Integrated, TicketClosureError> {
        candidate.apply_for()
    }
    /// The closure hook needs the revalidated observation, which only
    /// `pre_integration_with` carries; the integration never calls this.
    fn pre_integration(
        &self,
        _stage: &mut LeasedIntegration<'_, ClosureRecord>,
    ) -> Result<PreIntegration, TicketClosureError> {
        Err(TicketClosureError::RecoveryRequired)
    }
    fn pre_integration_with(
        &self,
        service: &RepositoryService,
        preview: &ClosureObservation,
        identity: Option<&ConfirmedCommitIdentity>,
        stage: &mut LeasedIntegration<'_, ClosureRecord>,
    ) -> Result<PreIntegration, TicketClosureError> {
        checkpoint::pre_integration(service, preview, identity, stage)
    }
    fn resume_written(
        &self,
        service: &RepositoryService,
        stage: &mut LeasedIntegration<'_, ClosureRecord>,
    ) -> Result<Option<PreIntegration>, TicketClosureError> {
        checkpoint::resume_written(service, stage)
    }

    fn outcome(
        r: &ClosureRecord,
        integration: PromotionIntegration,
        publication: PromotionPublication,
        cleanup: PromotionCleanup,
    ) -> TicketClosureOutcome {
        TicketClosureOutcome {
            operation_id: r.operation_id,
            closure: match r.closure_oid {
                Some(checkpoint_oid) => ClosureRecording::Recorded { checkpoint_oid },
                None => ClosureRecording::Existing,
            },
            integration,
            publication,
            cleanup,
        }
    }
    fn complete(outcome: TicketClosureOutcome) -> TicketClosureResult {
        TicketClosureResult::Complete(outcome)
    }
    fn index_pending(outcome: TicketClosureOutcome) -> TicketClosureResult {
        TicketClosureResult::IndexPending(IndexPending::new(outcome))
    }
}
