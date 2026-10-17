//! Durable parent orchestration. Children own transport/integration recovery;
//! this journal owns consent, primary integration and ordered retirement.
#![allow(clippy::result_large_err)]
use super::*;
use crate::repository::{
    cleanup::{BranchDeletion, ContextCleanup, Precheck, TrackingDeletion, WorktreeRemoval},
    remote::{observation::ObservationConfiguration, reservation, state},
    transport::*,
    *,
};
use record::{
    ChildRole, ChildState, CleanupCategory as Clean, IntegrationKind, PromotionRecord,
    PromotionStep as Step, PublicationCategory as Pub, Refusal,
};

// The production adapter calls the existing children and exact-delete transport.
// Tests inject only those external calls; all parent storage/Git/cleanup remains real.
pub(super) trait PromotionIo {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError>;
    #[allow(clippy::too_many_arguments)]
    fn delete(
        &mut self,
        service: &RepositoryService,
        request: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        configuration: &ObservationConfiguration,
        expected: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError>;
}

pub(super) fn promote<P: SessionCredentialProvider>(
    service: &RepositoryService,
    request: PromoteDocumentRequest,
    session: &mut SessionCredentials<P>,
) -> Result<DocumentPromotionResult, DocumentPromotionError> {
    promote_with_io(service, request, &mut Direct(session))
}

struct Direct<'a, P>(&'a mut SessionCredentials<P>);
impl<P: SessionCredentialProvider> PromotionIo for Direct<'_, P> {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        service.synchronize_promotion_child_observed(request, self.0, observer, execution)
    }
    fn delete(
        &mut self,
        service: &RepositoryService,
        request: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        configuration: &ObservationConfiguration,
        expected: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        let root = request.root.clone();
        let boundary = |root: &Path| {
            state::with_transaction(service, root, |tx, id| {
                let r = reservation::owned(service, tx, id, owner)?;
                if r.cancel_requested || r.yield_requested {
                    return Err(state::recovery_required());
                }
                Ok(())
            })?;
            Ok(())
        };
        let observed = service.with_scoped_action_remote(
            request,
            self.0,
            plan,
            configuration,
            boundary,
            |remote| {
                // Reach the advertised safe point before the guarded update,
                // after credentials/host review, without a repository lease.
                remote.fresh_advertisement()?;
                match service.remote_safe_point(
                    &root,
                    owner,
                    RemoteOperationSafePoint::AfterAdvertisement,
                ) {
                    Ok(RemoteSafePointOutcome::Continue) => (),
                    Ok(_) => return Ok(Err(SynchronizationError::Interrupted)),
                    Err(e) => return Ok(Err(e.into())),
                }
                Ok(remote
                    .delete_exact(
                        plan,
                        &SynchronizationTarget::Context {
                            kind: AuthoringKind::Document,
                            item_id: item.clone(),
                        },
                        expected,
                    )
                    .map_err(SynchronizationError::from))
            },
        )??;
        if matches!(observed, DeleteObservation::Changed(_))
            && configuration.fetch_and_push_share_endpoint()
        {
            // Preserve the collaborator's replacement as fetched tracking/object
            // evidence too. Reuse exact scoped Fetch, outside the lease, under
            // this same owner/configuration fence; never merge or republish it.
            service.with_scoped_action_remote(
                VerifySshTransportRequest {
                    root,
                    direction: SshDirection::Fetch,
                    approval: None,
                },
                self.0,
                plan,
                configuration,
                boundary,
                |remote| {
                    remote.fetch_exact(
                        plan,
                        &SynchronizationTarget::Context {
                            kind: AuthoringKind::Document,
                            item_id: item.clone(),
                        },
                    )
                },
            )?;
        }
        Ok(observed)
    }
}

fn mapped(error: RepositoryError) -> DocumentPromotionError {
    match error.kind {
        RepositoryErrorKind::OperationMismatch => DocumentPromotionError::OperationMismatch,
        RepositoryErrorKind::RepositoryBusy => DocumentPromotionError::Busy,
        RepositoryErrorKind::RecoveryRequired => DocumentPromotionError::RecoveryRequired,
        _ => DocumentPromotionError::Repository(error),
    }
}
fn load(
    service: &RepositoryService,
    root: &Path,
    op: OperationId,
) -> Result<PromotionRecord, RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        record::read_record(tx, id, op)?.ok_or_else(state::recovery_required)
    })
}
fn save(
    service: &RepositoryService,
    root: &Path,
    r: &PromotionRecord,
) -> Result<(), RepositoryError> {
    state::with_transaction(service, root, |tx, id| {
        let mut updated = r.clone();
        // Cancellation is owned by the requesting thread, never overwritten by
        // the executor's earlier snapshot of the parent.
        updated.cancel_requested = record::read_record(tx, id, r.operation_id)?
            .ok_or_else(state::recovery_required)?
            .cancel_requested;
        record::save_record(tx, id, &updated)
    })
}
fn boundary(
    service: &RepositoryService,
    root: &Path,
    point: FailurePoint,
) -> Result<(), RepositoryError> {
    #[cfg(test)]
    super::run_tests::checkpoint(service, root, point);
    service.check_failure(point, RepositoryOperation::RepositorySnapshot, root)
}
fn preview(
    service: &RepositoryService,
    root: &Path,
    item: &ItemId,
) -> Result<DocumentPromotionPreflight, RepositoryError> {
    observe::preview(
        service,
        PreviewDocumentPromotionRequest {
            root: root.into(),
            item_id: item.clone(),
        },
    )
}
fn refusal_error(refusal: &Refusal) -> DocumentPromotionError {
    match refusal {
        Refusal::Blocked(b) => DocumentPromotionError::Blocked(b.clone()),
        Refusal::ExternalChange => DocumentPromotionError::ExternalChange,
        Refusal::ReconfirmationRequired => DocumentPromotionError::ReconfirmationRequired,
    }
}
fn abandon(
    service: &RepositoryService,
    root: &Path,
    r: &mut PromotionRecord,
    refusal: Refusal,
) -> DocumentPromotionError {
    let error = refusal_error(&refusal);
    r.step = Step::Abandoned;
    r.refusal = Some(refusal);
    match save(service, root, r) {
        Ok(()) => error,
        Err(e) => mapped(e),
    }
}
fn cancelled(
    service: &RepositoryService,
    root: &Path,
    r: &PromotionRecord,
) -> Result<bool, RepositoryError> {
    Ok(load(service, root, r.operation_id)?.cancel_requested)
}

pub(super) fn promote_with_io(
    service: &RepositoryService,
    request: PromoteDocumentRequest,
    io: &mut impl PromotionIo,
) -> Result<DocumentPromotionResult, DocumentPromotionError> {
    let result = promote_inner(service, request.clone(), io);
    let Err(error) = result else {
        return result;
    };
    // An executor may stop before its first reconciliation (lease/config/SQL
    // failure), or another same-ID executor may have just recorded the effect.
    // Only return an error after checking the matching durable/Git authority.
    let Ok((repo, root)) =
        canonical_repository_root(&request.root, RepositoryOperation::RepositorySnapshot)
    else {
        return Err(error);
    };
    let Ok(mut r) = load(service, &root, request.operation_id) else {
        return Err(error);
    };
    if r.item_id != request.item_id
        || r.expectation != request.expected
        || r.integration_oid.is_none()
    {
        return Err(error);
    }
    if r.step == Step::Complete || r.step == Step::Superseded {
        return Ok(refresh(service, &root, &r, r.step == Step::Complete));
    }
    if r.step == Step::Integrating {
        let state = recorded_primary(&root, &r)
            .ok()
            .and_then(|primary| integration_context(&r, &primary).ok())
            .map(|context| {
                if r.integration_kind == Some(IntegrationKind::AlreadyContained) {
                    let tip = repo
                        .refname_to_id(&format!("refs/heads/{}", context.primary_branch))
                        .ok();
                    if tip.is_some_and(|tip| {
                        tip == r.base_oid.unwrap()
                            || repo
                                .graph_descendant_of(tip, r.base_oid.unwrap())
                                .unwrap_or(false)
                    }) {
                        integrate::IntegrationState::Integrated {
                            oid: r.base_oid.unwrap(),
                        }
                    } else {
                        integrate::IntegrationState::RecoveryRequired
                    }
                } else {
                    integrate::reconcile_integration(&repo, r.base_oid.unwrap(), &context)
                }
            })
            .unwrap_or(integrate::IntegrationState::RecoveryRequired);
        match state {
            integrate::IntegrationState::NotIntegrated if integrate::primary_clean(&root) => {
                return Err(error);
            }
            integrate::IntegrationState::Integrated { oid } if Some(oid) == r.integration_oid => {
                r.step = Step::Integrated;
                let _ = save(service, &root, &r);
            }
            _ => (),
        }
    }
    Ok(recovery(service, &root, &mut r))
}
fn recorded_primary(root: &Path, r: &PromotionRecord) -> Result<String, RepositoryError> {
    let repo = git2::Repository::open(root).map_err(|_| state::recovery_required())?;
    let tree = repo
        .find_commit(r.base_oid.ok_or_else(state::recovery_required)?)
        .and_then(|c| c.tree())
        .map_err(|_| state::recovery_required())?;
    let entry = tree
        .get_path(Path::new(".manyhands/config.toml"))
        .map_err(|_| state::recovery_required())?;
    let blob = repo
        .find_blob(entry.id())
        .map_err(|_| state::recovery_required())?;
    let source = std::str::from_utf8(blob.content()).map_err(|_| state::recovery_required())?;
    Ok(crate::canonical::parse_repository_config(source)
        .map_err(|_| state::recovery_required())?
        .primary_branch)
}
fn promote_inner(
    service: &RepositoryService,
    mut request: PromoteDocumentRequest,
    io: &mut impl PromotionIo,
) -> Result<DocumentPromotionResult, DocumentPromotionError> {
    let (_, root) =
        canonical_repository_root(&request.root, RepositoryOperation::RepositorySnapshot)?;
    request.root = root.clone();
    let prior = state::with_transaction(service, &root, |tx, id| {
        record::read_record(tx, id, request.operation_id)
    })
    .map_err(mapped)?;
    if let Some(prior) = &prior {
        let mut validated = state::with_transaction(service, &root, |tx, id| {
            record::begin_record(
                tx,
                id,
                request.operation_id,
                &request.item_id,
                request.expected,
                prior.remote_name.as_deref(),
            )
        })
        .map_err(mapped)?;
        if let Some(refusal) = &validated.refusal {
            return Err(refusal_error(refusal));
        }
        if validated.step == Step::Complete {
            return Ok(refresh(service, &root, &validated, true));
        }
        if validated.step == Step::Superseded {
            return Err(DocumentPromotionError::OperationMismatch);
        }
        // Live configuration is not a prerequisite for reporting an existing
        // primary effect. Invalid/unreadable configuration stops unfinished work
        // as an Ok recovery outcome, and cannot turn integration into an Err.
        if !matches!(
            read_configuration(&root),
            Ok(ConfigurationInspection::Valid(_))
        ) && validated.integration_oid.is_some()
        {
            return Ok(recovery(service, &root, &mut validated));
        }
    }
    let ConfigurationInspection::Valid(mut config) = read_configuration(&root)? else {
        return Err(DocumentPromotionError::RecoveryRequired);
    };
    let mut preflight_lease = None;
    let predecessor = if prior.is_none() {
        Some(
            state::with_transaction(service, &root, |tx, id| {
                record::replacement_guard(tx, id, &request.item_id)
            })
            .map_err(mapped)?,
        )
    } else {
        None
    };
    let refusal = if prior.is_none() {
        let observation = observe::prepare(
            service,
            PreviewDocumentPromotionRequest {
                root: root.clone(),
                item_id: request.item_id.clone(),
            },
        )?;
        config = observation.configuration.clone();
        let (repository, _) =
            canonical_repository_root(&root, RepositoryOperation::RepositorySnapshot)?;
        // Discovery is complete before acquiring the common lease. Only bounded
        // checkout/Git/configuration revalidation and the atomic record change
        // run inside it; no filesystem discovery is called from this interval.
        preflight_lease = Some(repository_lease(
            &repository,
            &root,
            RepositoryOperation::RepositorySnapshot,
        )?);
        match observation.revalidate(service, &repository) {
            Ok(DocumentPromotionPreflight::Blocked { blockers }) => {
                Some(Refusal::Blocked(blockers))
            }
            Ok(DocumentPromotionPreflight::Ready(p)) if p.expectation() != request.expected => {
                Some(Refusal::ExternalChange)
            }
            Ok(_) => None,
            Err(error) if error.kind == RepositoryErrorKind::ExternalChange => {
                Some(Refusal::ExternalChange)
            }
            Err(error) => return Err(mapped(error)),
        }
    } else {
        None
    };
    let (mut r, existing) = state::with_transaction(service, &root, |tx, id| {
        let existing = record::read_record(tx, id, request.operation_id)?.is_some();
        let predecessor_changed = match &predecessor {
            Some(expected) => record::replacement_guard(tx, id, &request.item_id)? != *expected,
            None => false,
        };
        let refusal = if predecessor_changed {
            Some(Refusal::ExternalChange)
        } else {
            refusal.clone()
        };
        let r = if let Some(refusal) = &refusal {
            record::refuse_record(
                tx,
                id,
                request.operation_id,
                &request.item_id,
                request.expected,
                config.publication_remote.as_deref(),
                refusal,
            )?
        } else {
            record::begin_record(
                tx,
                id,
                request.operation_id,
                &request.item_id,
                request.expected,
                config.publication_remote.as_deref(),
            )?
        };
        Ok((r, existing))
    })
    .map_err(mapped)?;
    drop(preflight_lease);
    if let Some(refusal) = &r.refusal {
        return Err(refusal_error(refusal));
    }
    if r.step == Step::Superseded {
        return Err(DocumentPromotionError::OperationMismatch);
    }
    if r.step == Step::Complete {
        return Ok(refresh(service, &root, &r, true));
    }
    // A second call explicitly continues an inter-step cancellation. Child
    // ownership/cancel acknowledgement is reconciled separately below.
    if existing && r.cancel_requested {
        state::with_transaction(service, &root, |tx, id| {
            record::clear_cancellation(tx, id, r.operation_id)
        })
        .map_err(mapped)?;
        r.cancel_requested = false;
    }
    // Before integration the digest binds the branch name. Afterwards use its
    // immutable base commit's configuration, never a newly edited live setting,
    // to locate the primary whose effect must be reconciled/reported.
    let bound_primary = if r.base_oid.is_some() {
        match recorded_primary(&root, &r) {
            Ok(primary) => primary,
            Err(_) if r.integration_oid.is_some() => return Ok(recovery(service, &root, &mut r)),
            Err(e) => return Err(mapped(e)),
        }
    } else {
        config.primary_branch.clone()
    };
    // Reconcile Git before configuration/preflight errors can escape.
    if r.step == Step::Integrating {
        let (repo, _) = canonical_repository_root(&root, RepositoryOperation::RepositorySnapshot)?;
        let _lease = repository_lease(&repo, &root, RepositoryOperation::RepositorySnapshot)?;
        let context = integration_context(&r, &bound_primary)?;
        let base = r.base_oid.ok_or(DocumentPromotionError::RecoveryRequired)?;
        let state = if r.integration_kind == Some(IntegrationKind::AlreadyContained) {
            let tip = repo.refname_to_id(&format!("refs/heads/{bound_primary}"));
            if tip.ok().is_some_and(|tip| {
                tip == base || repo.graph_descendant_of(tip, base).unwrap_or(false)
            }) {
                integrate::IntegrationState::Integrated { oid: base }
            } else {
                integrate::IntegrationState::RecoveryRequired
            }
        } else {
            integrate::reconcile_integration(&repo, base, &context)
        };
        match state {
            integrate::IntegrationState::Integrated { oid } => {
                if r.integration_oid.is_some_and(|stored| stored != oid) {
                    drop(_lease);
                    return Ok(recovery(service, &root, &mut r));
                }
                r.integration_oid = Some(oid);
                r.step = Step::Integrated;
                if save(service, &root, &r).is_err() {
                    drop(_lease);
                    return Ok(recovery(service, &root, &mut r));
                }
            }
            integrate::IntegrationState::NotIntegrated => (),
            integrate::IntegrationState::RecoveryRequired => {
                drop(_lease);
                return Ok(recovery(service, &root, &mut r));
            }
        }
    }
    if r.integration_oid.is_some() && r.step != Step::Integrating {
        return Ok(finish_integrated(
            service,
            &request,
            &bound_primary,
            &mut r,
            io,
        ));
    }
    if r.remote_name != config.publication_remote {
        return Err(DocumentPromotionError::Interrupted);
    }
    if !existing {
        boundary(service, &root, FailurePoint::AfterConfirmed)?;
    }
    if cancelled(service, &root, &r)? {
        return Err(DocumentPromotionError::Interrupted);
    }
    if r.step == Step::Confirmed {
        let cleanup_only = matches!(preview(service,&root,&r.item_id),Ok(DocumentPromotionPreflight::Ready(ref p)) if p.integration()==PlannedIntegration::AlreadyContained && p.tracking_context().is_none());
        if r.remote_name.is_some() && !cleanup_only {
            if let Err(error) = child(service, &request, &mut r, ChildRole::Context, io) {
                if cancelled(service, &root, &r)? {
                    return Err(DocumentPromotionError::Interrupted);
                }
                return Err(DocumentPromotionError::ContextSynchronization(Box::new(
                    error,
                )));
            }
        }
        let p = match preview(service, &root, &r.item_id)? {
            DocumentPromotionPreflight::Ready(p) if p.expectation() == r.expectation => p,
            _ => {
                return Err(abandon(
                    service,
                    &root,
                    &mut r,
                    Refusal::ReconfirmationRequired,
                ));
            }
        };
        r.context_oid = Some(p.context_oid());
        r.remote_context_oid = p.tracking_context();
        r.step = Step::ContextSynchronized;
        save(service, &root, &r)?;
        boundary(service, &root, FailurePoint::AfterContextSync)?;
    }
    if cancelled(service, &root, &r)? {
        return Err(DocumentPromotionError::Interrupted);
    }
    let p = match preview(service, &root, &r.item_id)? {
        DocumentPromotionPreflight::Ready(p) => p,
        _ => {
            return Err(abandon(
                service,
                &root,
                &mut r,
                Refusal::ReconfirmationRequired,
            ));
        }
    };
    let mut input = integrate::IntegrationRequest::from_preview(&p);
    input.expected = r.expectation;
    // The exact synchronized context, not a later edit, is integrated.
    input.context.oid = r
        .context_oid
        .ok_or(DocumentPromotionError::RecoveryRequired)?;
    let prepared = match integrate::prepare_integration(
        service,
        &root,
        &input,
        request.confirmed_identity.as_ref(),
    ) {
        Ok(p) => p,
        Err(DocumentPromotionError::PrimaryAdvanced) if r.step == Step::ContextSynchronized => {
            state::with_transaction(service, &root, |tx, id| {
                record::reset_context_after_primary_advance(tx, id, r.operation_id)
            })
            .map_err(mapped)?;
            return Err(DocumentPromotionError::PrimaryAdvanced);
        }
        Err(
            DocumentPromotionError::Blocked(_)
            | DocumentPromotionError::ReconfirmationRequired
            | DocumentPromotionError::ExternalChange,
        ) if r.step != Step::Integrating => {
            return Err(abandon(
                service,
                &root,
                &mut r,
                Refusal::ReconfirmationRequired,
            ));
        }
        Err(e) => return Err(e),
    };
    let evidence = prepared.evidence();
    if r.step == Step::Integrating && r.base_oid != Some(evidence.base) {
        drop(prepared);
        return Ok(recovery(service, &root, &mut r));
    }
    r.base_oid = Some(evidence.base);
    r.integration_kind = Some(match evidence.kind {
        PlannedIntegration::MergeCommit => IntegrationKind::Merged,
        PlannedIntegration::AlreadyContained => IntegrationKind::AlreadyContained,
    });
    r.step = Step::Integrating;
    save(service, &root, &r)?;
    let candidate = if let Some(oid) = r.integration_oid {
        prepared.resume_candidate(oid)?
    } else {
        prepared.write_commit()?
    };
    r.integration_oid = Some(candidate.oid());
    save(service, &root, &r)?;
    boundary(service, &root, FailurePoint::BeforeIntegrationRef)?;
    if cancelled(service, &root, &r)? {
        return Err(DocumentPromotionError::Interrupted);
    }
    let integrated = candidate.apply()?;
    if integrated == integrate::Integrated::RecoveryRequired {
        return Ok(recovery(service, &root, &mut r));
    }
    // F4 may exit after the ref and before SQL. No post-effect error escapes.
    if boundary(service, &root, FailurePoint::AfterIntegrationRef).is_err() {
        return Ok(recovery(service, &root, &mut r));
    }
    r.step = Step::Integrated;
    if save(service, &root, &r).is_err() {
        return Ok(recovery(service, &root, &mut r));
    }
    Ok(finish_integrated(
        service,
        &request,
        &bound_primary,
        &mut r,
        io,
    ))
}

fn integration_context(
    r: &PromotionRecord,
    primary: &str,
) -> Result<integrate::IntegrationContext, DocumentPromotionError> {
    Ok(integrate::IntegrationContext {
        item_id: r.item_id.clone(),
        primary_branch: primary.into(),
        oid: r
            .context_oid
            .ok_or(DocumentPromotionError::RecoveryRequired)?,
    })
}
struct CancellationObserver<'a> {
    service: &'a RepositoryService,
    root: &'a Path,
    op: OperationId,
}
impl SynchronizationObserver for CancellationObserver<'_> {
    fn safe_point(&mut self, _: RemoteOperationSafePoint) -> SafePointRequest {
        match load(self.service, self.root, self.op) {
            Ok(r) if r.cancel_requested => SafePointRequest::Cancel,
            Ok(_) => SafePointRequest::Continue,
            Err(_) => SafePointRequest::Stop,
        }
    }
}
fn child_id(r: &PromotionRecord, role: ChildRole) -> Option<OperationId> {
    match role {
        ChildRole::Context => r.context_sync_id,
        ChildRole::Primary => r.primary_sync_id,
    }
}
fn child(
    service: &RepositoryService,
    request: &PromoteDocumentRequest,
    r: &mut PromotionRecord,
    role: ChildRole,
    io: &mut impl PromotionIo,
) -> Result<SynchronizationOutcome, SynchronizationError> {
    let root = &request.root;
    let observed =
        state::with_transaction(service, root, |tx, _| record::child_state(tx, r, role))?;
    let mut restart = false;
    let mut replace = child_id(r, role).is_none();
    match observed {
        ChildState::Complete(stored) => return stored_child_outcome(&stored, role, &r.item_id),
        ChildState::Retained(_) => restart = true,
        // Busy/yielding insert no child row. A bound but unrun attempt can be
        // retired safely too; its replacement is persisted before execution.
        ChildState::Absent => replace = true,
        ChildState::Unowned(stored) => {
            // Released resolution history continues a nonterminal child, but
            // cannot resurrect a terminal cancellation/failure. Retained
            // ownership was classified first, so replacing a terminal unowned
            // child below still requires actual absence of pending artifacts.
            if !matches!(
                stored.phase,
                RemoteOperationPhase::Cancelled
                    | RemoteOperationPhase::Failed
                    | RemoteOperationPhase::Completed
            ) && state::with_transaction(service, root, |tx, _| {
                reservation::promotion_child_has_resolution(tx, &stored)
            })? {
                restart = true;
            } else {
                // IdentityRequired is the one no-candidate continuation. It has an
                // identity step/window, classified as retained by Task 5's seam.
                // All other unowned failed attempts retire their active slot first.
                if !matches!(
                    stored.phase,
                    RemoteOperationPhase::Cancelled
                        | RemoteOperationPhase::Interrupted
                        | RemoteOperationPhase::Failed
                ) {
                    service.cancel_remote_operation(root, stored.operation_id)?;
                    let ack = SynchronizeRemoteRequest {
                        root: root.clone(),
                        operation_id: stored.operation_id,
                        target: synchronization_target(role, &r.item_id),
                        approval: request.approval.clone(),
                        confirmed_identity: request.confirmed_identity.clone(),
                        restart: true,
                    };
                    let mut observer = CancellationObserver {
                        service,
                        root,
                        op: r.operation_id,
                    };
                    let execution = record::child_execution(r, role)?;
                    let _ack_result = io.synchronize(service, ack, &mut observer, &execution);
                }
                // A cancellation acknowledgement may itself return RecoveryRequired;
                // durable ownership/terminal state, never that error, decides.
                let retired = state::with_transaction(service, root, |tx, _| {
                    record::child_state(tx, r, role)
                })?;
                match retired {
                    ChildState::Complete(stored) => {
                        return stored_child_outcome(&stored, role, &r.item_id);
                    }
                    ChildState::Unowned(stored)
                        if matches!(
                            stored.phase,
                            RemoteOperationPhase::Cancelled
                                | RemoteOperationPhase::Interrupted
                                | RemoteOperationPhase::Failed
                        ) =>
                    {
                        replace = true
                    }
                    ChildState::Retained(_) => restart = true,
                    _ => return Err(SynchronizationError::RecoveryRequired),
                }
            }
        }
    }
    if replace {
        *r = state::with_transaction(service, root, |tx, id| {
            record::next_attempt(tx, id, r.operation_id, role)
        })?;
        restart = false;
    }
    let mut observer = CancellationObserver {
        service,
        root,
        op: r.operation_id,
    };
    let execution = record::child_execution(r, role)?;
    let result = io.synchronize(
        service,
        SynchronizeRemoteRequest {
            root: root.clone(),
            operation_id: child_id(r, role).ok_or(SynchronizationError::RecoveryRequired)?,
            target: synchronization_target(role, &r.item_id),
            approval: request.approval.clone(),
            confirmed_identity: request.confirmed_identity.clone(),
            restart,
        },
        &mut observer,
        &execution,
    );
    // Durable completion wins over both a lost outer return and a concurrent
    // cancellation. Cancellation can stop unfinished work, not undo authority.
    match state::with_transaction(service, root, |tx, _| record::child_state(tx, r, role))? {
        ChildState::Complete(stored) => stored_child_outcome(&stored, role, &r.item_id),
        _ => {
            if result.is_err() && cancelled(service, root, r)? {
                return Err(SynchronizationError::Interrupted);
            }
            match result {
                Ok(SynchronizationResult::Complete(o)) => Ok(o),
                Ok(SynchronizationResult::IndexPending(p)) => Ok(p.authoritative),
                Err(e) => Err(e),
            }
        }
    }
}
fn synchronization_target(role: ChildRole, item: &ItemId) -> SynchronizationTarget {
    match role {
        ChildRole::Primary => SynchronizationTarget::Primary,
        ChildRole::Context => SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: item.clone(),
        },
    }
}
fn stored_child_outcome(
    stored: &state::StoredRemoteOperation,
    role: ChildRole,
    item: &ItemId,
) -> Result<SynchronizationOutcome, SynchronizationError> {
    let target = synchronization_target(role, item);
    match stored
        .authority
        .ok_or(SynchronizationError::RecoveryRequired)?
    {
        state::SynchronizationAuthority::Published(oid) => {
            Ok(SynchronizationOutcome::Published { target, oid })
        }
        state::SynchronizationAuthority::AlreadyCurrent(oid) => {
            Ok(SynchronizationOutcome::AlreadyCurrent { target, oid })
        }
    }
}
fn publication_error(error: &SynchronizationError) -> Pub {
    match error {
        SynchronizationError::ConflictPending { .. } => Pub::Conflict,
        SynchronizationError::PushRejected => Pub::Rejected,
        SynchronizationError::Transport(_) => Pub::Transport,
        SynchronizationError::IdentityRequired { .. } => Pub::Identity,
        SynchronizationError::Busy => Pub::Busy,
        SynchronizationError::PollYielding => Pub::Yielding,
        SynchronizationError::Interrupted => Pub::Cancelled,
        _ => Pub::RecoveryRequired,
    }
}

fn finish_integrated(
    service: &RepositoryService,
    request: &PromoteDocumentRequest,
    primary: &str,
    r: &mut PromotionRecord,
    io: &mut impl PromotionIo,
) -> DocumentPromotionResult {
    if let Err(category) = finish_steps(service, request, primary, r, io) {
        // A concurrent same-ID executor may have completed while this child's
        // index handoff was outside the lease. Stored completion outranks this
        // executor's now-obsolete unfinished-step snapshot.
        if let Ok(stored) = load(service, &request.root, r.operation_id)
            && stored.step == Step::Complete
            && stored.item_id == r.item_id
            && stored.expectation == r.expectation
        {
            return refresh(service, &request.root, &stored, true);
        }
        r.cleanup = Some(category);
        let _ = save(service, &request.root, r);
    }
    refresh(service, &request.root, r, false)
}
fn finish_steps(
    service: &RepositoryService,
    request: &PromoteDocumentRequest,
    primary: &str,
    r: &mut PromotionRecord,
    io: &mut impl PromotionIo,
) -> Result<(), Clean> {
    let root = &request.root;
    let recovery = |_| Clean::RecoveryRequired;
    let ConfigurationInspection::Valid(config) = read_configuration(root).map_err(recovery)? else {
        return Err(Clean::RecoveryRequired);
    };
    if config.primary_branch != primary || config.publication_remote != r.remote_name {
        return Err(Clean::RecoveryRequired);
    }
    if cancelled(service, root, r).map_err(recovery)? {
        if matches!(r.step, Step::Integrated | Step::PublicationPending) {
            r.publication = Some(Pub::Cancelled);
            r.step = Step::PublicationPending;
        }
        return Err(Clean::Cancelled);
    }
    if matches!(r.step, Step::Integrated | Step::PublicationPending) {
        if r.remote_name.is_none() {
            r.step = Step::PublicationPending;
            r.publication = Some(Pub::NoPublicationRemote);
            r.cleanup = None;
        } else {
            match child(service, request, r, ChildRole::Primary, io) {
                Ok(SynchronizationOutcome::Published { .. }) => {
                    r.publication = Some(Pub::Published);
                    r.step = Step::Published;
                    r.cleanup = None;
                }
                Ok(SynchronizationOutcome::AlreadyCurrent { .. }) => {
                    r.publication = Some(Pub::AlreadyCurrent);
                    r.step = Step::Published;
                    r.cleanup = None;
                }
                Ok(_) => {
                    r.publication = Some(Pub::RecoveryRequired);
                    r.step = Step::PublicationPending;
                }
                Err(error) => {
                    r.publication = Some(publication_error(&error));
                    r.step = Step::PublicationPending;
                }
            }
        }
        save(service, root, r).map_err(recovery)?;
        if r.remote_name.is_some() && r.step == Step::PublicationPending {
            return Err(Clean::AwaitingPublication);
        }
        boundary(service, root, FailurePoint::AfterPublication).map_err(recovery)?;
    }
    if cancelled(service, root, r).map_err(recovery)? {
        return Err(Clean::Cancelled);
    }
    require_configuration(root, primary, r.remote_name.as_deref())?;
    let oid = r.context_oid.ok_or(Clean::RecoveryRequired)?;
    if matches!(r.step, Step::Published | Step::PublicationPending) {
        let cleanup = ContextCleanup::acquire(root).map_err(recovery)?;
        match cleanup.precheck_context(AuthoringKind::Document, &r.item_id, oid) {
            Precheck::Ok => (),
            Precheck::IgnoredFiles => return Err(Clean::IgnoredFiles),
            _ => return Err(Clean::ContextChanged),
        }
    }
    let plan = r
        .remote_name
        .as_ref()
        .map(|name| RemoteRefPlan::from_configuration(name, primary))
        .transpose()
        .map_err(|_| Clean::RecoveryRequired)?;
    let needs_owner = plan.is_some()
        && !(r.integration_kind == Some(IntegrationKind::AlreadyContained)
            && r.remote_context_oid.is_none());
    let mut owner = None;
    let mut configuration = None;
    if needs_owner {
        let plan = plan.as_ref().ok_or(Clean::RecoveryRequired)?;
        let frozen = service
            .observation_configuration(root, plan)
            .map_err(recovery)?;
        state::with_transaction(service, root, |tx, id| {
            state::configure_endpoints(tx, id, plan, &frozen.endpoint_digest())?;
            if let Some(child) = r.primary_sync_id {
                let child =
                    state::read_operation(tx, id, child)?.ok_or_else(state::recovery_required)?;
                if child.generation != state::generation(tx, id)? {
                    return Err(state::recovery_required());
                }
            }
            Ok(())
        })
        .map_err(recovery)?;
        let target = RemoteOperationTarget::for_context(
            plan,
            RemoteOperationAction::Promote,
            AuthoringKind::Document,
            r.item_id.clone(),
        )
        .map_err(|_| Clean::RecoveryRequired)?;
        let exists = state::with_transaction(service, root, |tx, id| {
            Ok(state::read_operation(tx, id, r.operation_id)?.is_some())
        })
        .map_err(recovery)?;
        owner = Some(
            match if exists {
                service.restart_remote_promotion(root, r.operation_id, &target)
            } else {
                service.reserve_remote_operation(root, r.operation_id, &target)
            }
            .map_err(recovery)?
            {
                RemoteReservationOutcome::Reserved(o) => o,
                _ => return Err(Clean::RecoveryRequired),
            },
        );
        safe(
            service,
            root,
            owner.as_ref().unwrap(),
            RemoteOperationSafePoint::BeforeTransport,
        )?;
        configuration = Some(frozen);
    }
    if matches!(r.step, Step::Published | Step::PublicationPending) && plan.is_some() {
        if needs_owner {
            boundary(service, root, FailurePoint::BeforeRemoteDeletion).map_err(recovery)?;
            if cancelled(service, root, r).map_err(recovery)? {
                return Err(Clean::Cancelled);
            }
            validate_cleanup_owner(
                service,
                root,
                owner.as_ref(),
                plan.as_ref(),
                configuration.as_ref(),
                primary,
            )?;
            {
                let cleanup = ContextCleanup::acquire(root).map_err(recovery)?;
                match cleanup.precheck_context(AuthoringKind::Document, &r.item_id, oid) {
                    Precheck::Ok => (),
                    Precheck::IgnoredFiles => return Err(Clean::IgnoredFiles),
                    _ => return Err(Clean::ContextChanged),
                }
            } // Drop the precheck lease before credentials/advertisement/delete.
            // T(C) is the fetched, pre-push observation, not necessarily the
            // context tip the child verified as published. Keep that tracking
            // evidence for guarded local retirement, but guard the remote delete
            // with the child's durable publication authority.
            let published_context = state::with_transaction(service, root, |tx, _| {
                match record::child_state(tx, r, ChildRole::Context)? {
                    ChildState::Complete(child) => child
                        .authority
                        .map(|authority| authority.oid())
                        .ok_or_else(state::recovery_required),
                    _ => Err(state::recovery_required()),
                }
            })
            .map_err(recovery)?;
            let observed = io
                .delete(
                    service,
                    VerifySshTransportRequest {
                        root: root.clone(),
                        direction: SshDirection::Delete,
                        approval: request.approval.clone(),
                    },
                    plan.as_ref().unwrap(),
                    owner.as_ref().unwrap(),
                    configuration.as_ref().unwrap(),
                    published_context,
                    &r.item_id,
                )
                .map_err(|e| deletion_error(&e))?;
            safe(
                service,
                root,
                owner.as_ref().unwrap(),
                RemoteOperationSafePoint::AfterAdvertisement,
            )?;
            match observed {
                DeleteObservation::Deleted => (),
                DeleteObservation::Changed(_) => return Err(Clean::RemoteContextChanged),
                DeleteObservation::StillPresent(_) => return Err(Clean::RemoteDeletionRejected),
            }
        }
        r.step = Step::RemoteDeleted;
        r.cleanup = None;
        save(service, root, r).map_err(recovery)?;
        boundary(service, root, FailurePoint::AfterRemoteDeletion).map_err(recovery)?;
    } else if let Some(owner) = &owner {
        // Restart from a local cleanup step needs no network. Re-establish the
        // exact safe-point evidence under the new epoch before SQL completion.
        safe(
            service,
            root,
            owner,
            RemoteOperationSafePoint::AfterAdvertisement,
        )?;
    }
    if cancelled(service, root, r).map_err(recovery)? {
        return Err(Clean::Cancelled);
    }
    let cleanup = ContextCleanup::acquire(root).map_err(recovery)?;
    validate_cleanup_owner(
        service,
        root,
        owner.as_ref(),
        plan.as_ref(),
        configuration.as_ref(),
        primary,
    )?;
    if matches!(
        r.step,
        Step::PublicationPending | Step::RemoteDeleted | Step::Published
    ) {
        match cleanup.precheck_context(AuthoringKind::Document, &r.item_id, oid) {
            Precheck::Ok => (),
            Precheck::IgnoredFiles => return Err(Clean::IgnoredFiles),
            _ => return Err(Clean::ContextChanged),
        }
        r.step = Step::WorktreeRemoving;
        save(service, root, r).map_err(recovery)?;
    }
    if r.step == Step::WorktreeRemoving {
        boundary(service, root, FailurePoint::DuringWorktreeRemoval).map_err(recovery)?;
        if cancelled(service, root, r).map_err(recovery)? {
            return Err(Clean::Cancelled);
        }
        validate_cleanup_owner(
            service,
            root,
            owner.as_ref(),
            plan.as_ref(),
            configuration.as_ref(),
            primary,
        )?;
        match cleanup.remove_context_worktree(AuthoringKind::Document, &r.item_id, oid) {
            WorktreeRemoval::Removed => {
                #[cfg(test)]
                super::run_tests::local_effect(service, root, "worktree_remove");
            }
            WorktreeRemoval::AlreadyAbsent => (),
            WorktreeRemoval::Changed => return Err(Clean::ContextChanged),
            WorktreeRemoval::RecoveryRequired => return Err(Clean::RecoveryRequired),
        }
        r.step = Step::WorktreeRemoved;
        r.cleanup = None;
        save(service, root, r).map_err(recovery)?;
    }
    if cancelled(service, root, r).map_err(recovery)? {
        return Err(Clean::Cancelled);
    }
    if r.step == Step::WorktreeRemoved {
        boundary(service, root, FailurePoint::BeforeBranchDeletion).map_err(recovery)?;
        if cancelled(service, root, r).map_err(recovery)? {
            return Err(Clean::Cancelled);
        }
        validate_cleanup_owner(
            service,
            root,
            owner.as_ref(),
            plan.as_ref(),
            configuration.as_ref(),
            primary,
        )?;
        match cleanup.delete_context_branch(AuthoringKind::Document, &r.item_id, oid) {
            BranchDeletion::Deleted => {
                #[cfg(test)]
                super::run_tests::local_effect(service, root, "branch_delete");
            }
            BranchDeletion::AlreadyAbsent => (),
            BranchDeletion::Changed => return Err(Clean::ContextChanged),
        }
        r.step = Step::BranchDeleted;
        save(service, root, r).map_err(recovery)?;
    }
    if cancelled(service, root, r).map_err(recovery)? {
        return Err(Clean::Cancelled);
    }
    if r.step != Step::BranchDeleted {
        return Err(Clean::RecoveryRequired);
    }
    // T4-1: durable branch_deleted, then Git first with its absence guard held
    // through ONE transaction for history, retirement, parent, reservation.
    validate_cleanup_owner(
        service,
        root,
        owner.as_ref(),
        plan.as_ref(),
        configuration.as_ref(),
        primary,
    )?;
    boundary(service, root, FailurePoint::BeforeCompletion).map_err(recovery)?;
    if cancelled(service, root, r).map_err(recovery)? {
        return Err(Clean::Cancelled);
    }
    cleanup
        .require_local_cleanup_absent(AuthoringKind::Document, &r.item_id)
        .map_err(|error| match error {
            TrackingDeletion::Changed => Clean::ContextChanged,
            TrackingDeletion::RecoveryRequired => Clean::RecoveryRequired,
        })?;
    // A permitted poll between publication and F6 may fetch the just-published
    // context into T(C). Both the original fetched value and this exact durable
    // child authority are bound evidence; arbitrary/newer or symbolic refs are
    // still refused by the guarded cleanup helper.
    let mut expected_tracking = r.remote_context_oid;
    if let Some(plan) = &plan {
        let published = state::with_transaction(service, root, |tx, _| {
            Ok(match record::child_state(tx, r, ChildRole::Context)? {
                ChildState::Complete(child) => child.authority.map(|a| a.oid()),
                _ => None,
            })
        })
        .map_err(recovery)?;
        let repo = git2::Repository::open(root).map_err(|_| Clean::RecoveryRequired)?;
        let target = plan.context(AuthoringKind::Document, &r.item_id);
        match repo.find_reference(target.tracking_ref()) {
            Ok(reference) if published.is_some() && reference.target() == published => {
                expected_tracking = published
            }
            Ok(_) => (),
            Err(error) if error.code() == git2::ErrorCode::NotFound => (),
            Err(_) => return Err(Clean::RecoveryRequired),
        }
    }
    let evidence = cleanup
        .delete_tracking_ref(
            plan.as_ref(),
            AuthoringKind::Document,
            &r.item_id,
            expected_tracking,
            r.operation_id,
            oid,
        )
        .map_err(|_| Clean::RecoveryRequired)?;
    #[cfg(test)]
    super::run_tests::tracking_deleted(service, root);
    boundary(service, root, FailurePoint::PromotionAfterTrackingDeletion).map_err(recovery)?;
    let completed = state::with_transaction(service, root, |tx, id| {
        // Reappearance after the Git tracking effect is also a refusal, with
        // SQL history/retirement/parent/owner unchanged by transaction rollback.
        cleanup
            .require_local_cleanup_absent(AuthoringKind::Document, &r.item_id)
            .map_err(|_| state::recovery_required())?;
        record::complete_record(tx, service, id, r.operation_id, &evidence, owner.as_ref())
    });
    #[cfg(test)]
    let completed = super::run_tests::completion_returned(service, root, completed);
    let completed = completed
        .and_then(|()| boundary(service, root, FailurePoint::PromotionAfterCompletionCommit));
    // A commit error does not prove rollback. Reopen the real durable authority
    // before returning recovery, retaining the Git absence guard throughout.
    if completed.is_err()
        && !state::with_transaction(service, root, |tx, id| {
            record::reconcile_completion(tx, id, r.operation_id)
        })
        .unwrap_or(false)
    {
        return Err(Clean::RecoveryRequired);
    }
    r.step = Step::Complete;
    r.cleanup = None;
    drop(evidence);
    drop(cleanup);
    boundary(service, root, FailurePoint::AfterCompletion).map_err(recovery)?;
    Ok(())
}
fn safe(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    point: RemoteOperationSafePoint,
) -> Result<(), Clean> {
    match service
        .remote_safe_point(root, owner, point)
        .map_err(|_| Clean::RecoveryRequired)?
    {
        RemoteSafePointOutcome::Continue => Ok(()),
        _ => Err(Clean::Cancelled),
    }
}
fn validate_cleanup_owner(
    service: &RepositoryService,
    root: &Path,
    owner: Option<&RemoteReservation>,
    plan: Option<&RemoteRefPlan>,
    configuration: Option<&ObservationConfiguration>,
    primary: &str,
) -> Result<(), Clean> {
    require_configuration(root, primary, plan.map(RemoteRefPlan::remote_name))?;
    if let Some(owner) = owner {
        state::with_transaction(service, root, |tx, id| {
            reservation::owned(service, tx, id, owner)?;
            Ok(())
        })
        .map_err(|_| Clean::RecoveryRequired)?;
    }
    if let (Some(plan), Some(configuration)) = (plan, configuration) {
        if !service
            .synchronization_configuration_matches(root, plan, configuration)
            .unwrap_or(false)
        {
            return Err(Clean::RecoveryRequired);
        }
    }
    Ok(())
}
fn require_configuration(root: &Path, primary: &str, remote: Option<&str>) -> Result<(), Clean> {
    match read_configuration(root) {
        Ok(ConfigurationInspection::Valid(config))
            if config.primary_branch == primary
                && config.publication_remote.as_deref() == remote =>
        {
            Ok(())
        }
        _ => Err(Clean::RecoveryRequired),
    }
}
fn deletion_error(error: &SynchronizationError) -> Clean {
    if matches!(error, SynchronizationError::Interrupted) {
        return Clean::Cancelled;
    }
    if let SynchronizationError::Transport(error) = error {
        match error.kind {
            SshTransportErrorKind::PushRejected => Clean::RemoteDeletionRejected,
            SshTransportErrorKind::ConfigurationInvalid
            | SshTransportErrorKind::PublicationRemoteMissing
            | SshTransportErrorKind::UsernameRequired
            | SshTransportErrorKind::EndpointChanged => Clean::RemoteDeletionConfiguration,
            SshTransportErrorKind::NoSelectedKey
            | SshTransportErrorKind::KeyMissing
            | SshTransportErrorKind::KeyUnreadable
            | SshTransportErrorKind::KeySourceChanged
            | SshTransportErrorKind::SelectionChanged
            | SshTransportErrorKind::KeyInvalidOrUnsupported
            | SshTransportErrorKind::KeyRejected => Clean::RemoteDeletionKey,
            SshTransportErrorKind::UnlockCancelled
            | SshTransportErrorKind::ProviderUnavailable
            | SshTransportErrorKind::UnlockFailed => Clean::RemoteDeletionUnlock,
            SshTransportErrorKind::HostApprovalRequired { .. }
            | SshTransportErrorKind::HostReplacementRequired { .. }
            | SshTransportErrorKind::HostTrustChanged
            | SshTransportErrorKind::HostVerificationUnavailable => Clean::RemoteDeletionHost,
            _ => Clean::RemoteDeletionTransport,
        }
    } else if matches!(error, SynchronizationError::PushRejected) {
        Clean::RemoteDeletionRejected
    } else {
        Clean::RecoveryRequired
    }
}
fn recovery(
    service: &RepositoryService,
    root: &Path,
    r: &mut PromotionRecord,
) -> DocumentPromotionResult {
    r.cleanup = Some(Clean::RecoveryRequired);
    let _ = save(service, root, r);
    refresh(service, root, r, false)
}
fn refresh(
    service: &RepositoryService,
    root: &Path,
    r: &PromotionRecord,
    replay: bool,
) -> DocumentPromotionResult {
    let outcome = make_outcome(service, root, r);
    if replay
        && service
            .repository_snapshot(root)
            .is_ok_and(|s| !s.refresh_required)
    {
        return DocumentPromotionResult::Complete(outcome);
    }
    let marked = mark_document_refresh_required(
        &service.registry_path,
        root,
        RepositoryOperation::RepositorySnapshot,
    )
    .is_ok();
    let refreshed = marked
        && boundary(service, root, FailurePoint::RefreshFailure).is_ok()
        && matches!(
            service.refresh_repository(RefreshRepositoryRequest {
                root: root.into(),
                operation_id: OperationId::new()
            }),
            Ok(RefreshOutcome::Refreshed { .. })
        );
    if refreshed {
        DocumentPromotionResult::Complete(outcome)
    } else {
        DocumentPromotionResult::IndexPending(IndexPending::new(outcome))
    }
}
fn make_outcome(
    service: &RepositoryService,
    root: &Path,
    r: &PromotionRecord,
) -> DocumentPromotionOutcome {
    let oid = r
        .integration_oid
        .or(r.base_oid)
        .expect("post-integration evidence");
    let publication_oid = state::with_transaction(service, root, |tx, _| {
        Ok(match record::child_state(tx, r, ChildRole::Primary)? {
            ChildState::Complete(s) => s.authority.map(|a| a.oid()),
            _ => None,
        })
    })
    .ok()
    .flatten()
    .unwrap_or(oid);
    let publication = match r.publication {
        Some(Pub::Published) => PromotionPublication::Published {
            oid: publication_oid,
        },
        Some(Pub::AlreadyCurrent) => PromotionPublication::AlreadyCurrent {
            oid: publication_oid,
        },
        category => PromotionPublication::Pending {
            reason: match category {
                Some(Pub::NoPublicationRemote) => PromotionPublicationPending::NoPublicationRemote,
                Some(Pub::Cancelled) => PromotionPublicationPending::Cancelled,
                category => PromotionPublicationPending::Synchronization(match category {
                    Some(Pub::Conflict) => PromotionSynchronizationCategory::Conflict,
                    Some(Pub::Rejected) => PromotionSynchronizationCategory::Rejected,
                    Some(Pub::Transport) => PromotionSynchronizationCategory::Transport,
                    Some(Pub::Identity) => PromotionSynchronizationCategory::Identity,
                    Some(Pub::Busy) => PromotionSynchronizationCategory::Busy,
                    Some(Pub::Yielding) => PromotionSynchronizationCategory::Yielding,
                    _ => PromotionSynchronizationCategory::RecoveryRequired,
                }),
            },
        },
    };
    let cleanup = if r.step == Step::Complete {
        PromotionCleanup::Complete
    } else {
        PromotionCleanup::Pending {
            reason: match r.cleanup {
                Some(Clean::AwaitingPublication) => PromotionCleanupPending::AwaitingPublication,
                Some(Clean::ContextChanged) => PromotionCleanupPending::ContextChanged,
                Some(Clean::IgnoredFiles) => PromotionCleanupPending::IgnoredFiles,
                Some(Clean::RemoteContextChanged) => PromotionCleanupPending::RemoteContextChanged,
                Some(Clean::Cancelled) => PromotionCleanupPending::Cancelled,
                Some(Clean::RemoteDeletionTransport) => {
                    PromotionCleanupPending::RemoteDeletionFailed(
                        PromotionRemoteDeletionCategory::Transport,
                    )
                }
                Some(Clean::RemoteDeletionRejected) => {
                    PromotionCleanupPending::RemoteDeletionFailed(
                        PromotionRemoteDeletionCategory::Rejected,
                    )
                }
                Some(Clean::RemoteDeletionConfiguration) => {
                    PromotionCleanupPending::RemoteDeletionFailed(
                        PromotionRemoteDeletionCategory::Configuration,
                    )
                }
                Some(Clean::RemoteDeletionKey) => PromotionCleanupPending::RemoteDeletionFailed(
                    PromotionRemoteDeletionCategory::Key,
                ),
                Some(Clean::RemoteDeletionUnlock) => PromotionCleanupPending::RemoteDeletionFailed(
                    PromotionRemoteDeletionCategory::Unlock,
                ),
                Some(Clean::RemoteDeletionHost) => PromotionCleanupPending::RemoteDeletionFailed(
                    PromotionRemoteDeletionCategory::Host,
                ),
                _ => PromotionCleanupPending::RecoveryRequired,
            },
        }
    };
    DocumentPromotionOutcome {
        operation_id: r.operation_id,
        integration: match r.integration_kind {
            Some(IntegrationKind::AlreadyContained) => {
                PromotionIntegration::AlreadyContained { oid }
            }
            _ => PromotionIntegration::Merged { oid },
        },
        publication,
        cleanup,
    }
}
