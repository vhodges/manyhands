//! A close that makes a new branch (Part F): the ticket's context is not the
//! one the consent saw, so the closure is a commit over the tracking primary,
//! held by a pending ref of the record's own, the old context, if there is
//! one to retire, is retired, and the new context is made at the commit.
//!
//! Step 1 builds the commit, records it and the hold it gives the ticket, and
//! makes the pending ref. Step 2 retires the old context (a record with a
//! `retired_oid`): its worktree and branch are removed under the repository
//! lease, then, under the record's own `close` reservation, the stale
//! tracking ref is deleted and the retirement committed, which ends the
//! reservation with it. Step 3 makes the context branch and its worktree at
//! the commit, then lets the pending ref go. Every write is recorded before
//! the Git state it describes, so a retry from any stop finds what the last
//! call left, and never builds a second commit.
use super::{
    BranchAnswer, ClosureAction, ClosureExpectation, PlannedClosure, PreviewTicketClosureRequest,
    TicketClosureError,
    checkpoint::{
        build_closure_commit, build_kept_closure_commit, commit_closure, context_moving,
        whole_utc_seconds,
    },
    lifecycle::TicketClosureLifecycle,
    observe::{self, PreparedClosureObservation, closed_by_text, ticket_path},
    record::{self, ClosureKind, ClosureRecord, ClosureStep, Refusal},
};
use crate::{
    canonical::Integration,
    repository::{
        AuthoringKind, ClosureBlocker, FailurePoint, OperationId, RemoteOperationSafePoint,
        RemoteOperationTarget, RemoteRefPlan, RemoteReservationOutcome, RepositoryErrorKind,
        RepositoryLease, RepositoryOperation, RepositoryService, canonical_repository_root,
        cleanup::{BranchDeletion, ContextCleanup, Precheck, TrackingDeletion, WorktreeRemoval},
        promotion::{
            integrate,
            lifecycle::{BranchStep, LifecycleKind, LifecycleRequest},
            record::CleanupCategory,
            run,
        },
        remote::{reservation::RemoteReservation, state, sync},
        repository_lease,
    },
};
use git2::Oid;
use std::path::Path;

#[cfg(test)]
use crate::repository::promotion::run_tests;

const KIND: LifecycleKind = LifecycleKind::TicketClosure;

/// A new branch's steps, under the repository lease (R-34), as the runner
/// calls it for a record whose action is `new_branch`: with the commit not
/// yet recorded, the closure is built and recorded, or the record ends with
/// the answer that says why not; then the old context, if the record has one
/// to retire, is retired; then the new context is made at the commit. Nothing
/// here merges into the primary, or moves it.
///
/// An abandon is one transaction. The pending ref goes first, and the record
/// that ends under its own reservation ends the reservation with it.
pub(in super::super) fn step(
    service: &RepositoryService,
    request: &LifecycleRequest<ClosureExpectation>,
    r: &ClosureRecord,
) -> Result<BranchStep, TicketClosureError> {
    let item = &r.item_id;
    // The consent is observed outside the lease, in the record's own mode. A
    // synchronization of the context that this record does not own is `busy`,
    // and names the row: nothing is written.
    let prepared = if r.step == ClosureStep::Confirmed && r.closure_oid.is_none() {
        let preview = PreviewTicketClosureRequest {
            root: request.root.clone(),
            item_id: item.clone(),
            confirmed_identity: request.confirmed_identity.clone(),
        };
        let prepared =
            observe::prepare_observation_in(service, &preview, Some(r.integration_mode))?;
        if let Some(operation) = state::with_transaction(service, &request.root, |tx, id| {
            observe::foreign_synchronization(tx, id, item)
        })? {
            return Err(observe::foreign_synchronization_busy(&request.root, &operation).into());
        }
        Some(prepared)
    } else {
        None
    };
    let (repo, root) =
        canonical_repository_root(&request.root, RepositoryOperation::RepositorySnapshot)?;
    let root = root.as_path();
    let lease = repository_lease(&repo, root, RepositoryOperation::RepositorySnapshot)?;
    let mut rec = match &prepared {
        Some(prepared) => match record_commit(service, request, r, prepared, &repo, root)? {
            Recorded::Commit => run::load::<TicketClosureLifecycle>(service, root, r.operation_id)?,
            Recorded::Abandoned => return Ok(BranchStep::Finished),
        },
        None => {
            let commit = r.closure_oid.ok_or(TicketClosureError::RecoveryRequired)?;
            // A retry from before the pending ref was made.
            if r.step == ClosureStep::Confirmed {
                hold_commit(&repo, r.operation_id, commit)?;
            }
            r.clone()
        }
    };
    let commit = rec
        .closure_oid
        .ok_or(TicketClosureError::RecoveryRequired)?;
    let _lease = if rec.retired_oid.is_some()
        && matches!(
            rec.step,
            ClosureStep::Confirmed
                | ClosureStep::WorktreeRemoving
                | ClosureStep::WorktreeRemoved
                | ClosureStep::BranchDeleted
        ) {
        match retire(service, request, &mut rec, &repo, root, lease)? {
            Retirement::Done(lease) => lease,
            Retirement::Abandoned => return Ok(BranchStep::Finished),
        }
    } else {
        lease
    };
    make_context(service, request, &rec, &repo, root, commit)?;
    Ok(BranchStep::Closing)
}

enum Recorded {
    Commit,
    Abandoned,
}

/// Step 1: re-observe under the lease, build the closure commit over the
/// tracking primary, record it with the hold it gives the ticket, and hold it
/// by its pending ref. The record is abandoned, with nothing written, when
/// what the consent bound has changed.
fn record_commit(
    service: &RepositoryService,
    request: &LifecycleRequest<ClosureExpectation>,
    r: &ClosureRecord,
    prepared: &PreparedClosureObservation,
    repo: &git2::Repository,
    root: &Path,
) -> Result<Recorded, TicketClosureError> {
    let item = &r.item_id;
    let now = (request.clock)();
    let abandon = |refusal: Refusal, answer: BranchAnswer| {
        abandon(service, root, r, refusal, answer, None).map(|()| Recorded::Abandoned)
    };
    let observed = match observe::revalidate_under_lease(prepared, service, repo) {
        Ok(observed) => observed,
        Err(error) if error.kind == RepositoryErrorKind::ExternalChange => {
            return abandon(
                Refusal::ReconfirmationRequired,
                BranchAnswer::ReconfirmationRequired,
            );
        }
        Err(error) => return Err(error.into()),
    };
    let observation = match observed {
        Ok(observation) => observation,
        // A situation that changed since the consent, not a blocker the
        // record could store.
        Err(blockers) if blockers.contains(&ClosureBlocker::RemoteContextUnmaterialized) => {
            return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange);
        }
        Err(_) => {
            return abandon(
                Refusal::ReconfirmationRequired,
                BranchAnswer::ReconfirmationRequired,
            );
        }
    };
    // What the request consented to: no context child ran, so the whole
    // digest compares, its head included.
    if observation.path != (Integration::PullRequest, ClosureAction::NewBranch)
        || observation.expectation != r.expectation
    {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange);
    }
    // A synchronization of this context still running holds the commit off
    // before anything is written, the identity included.
    if state::with_transaction(service, root, |tx, id| {
        record::context_synchronization_unfinished(tx, id, item)
    })? {
        return Err(TicketClosureError::Busy);
    }
    let identity = integrate::promotion_identity::<TicketClosureError>(
        service,
        repo,
        root,
        request.confirmed_identity.as_ref(),
    )?;
    let path = ticket_path(item);
    // The closure the commit carries, and the time the record keeps for it:
    // the clock's for a `record` closure, which the commit is stamped with,
    // and the old context's own for the closure it keeps, which is stamped
    // with the clock like any commit.
    let (closed_at, commit) = match (&observation.closure, r.closure_kind) {
        (PlannedClosure::Record { closed_by }, Some(ClosureKind::Record)) => {
            if closed_by_text(&identity) != *closed_by {
                return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange);
            }
            match build_closure_commit(
                repo,
                item,
                &path,
                observation.base,
                whole_utc_seconds(now),
                &identity,
            )? {
                Some(built) => (built.closed_at, built.commit),
                None => {
                    return abandon(
                        Refusal::ReconfirmationRequired,
                        BranchAnswer::ReconfirmationRequired,
                    );
                }
            }
        }
        // The kept closure is the old tip's own, which the digest bound.
        (PlannedClosure::Existing { .. }, Some(ClosureKind::Existing)) => {
            let built = match r
                .consented_context_oid
                .and_then(|tip| commit_closure(repo, tip, &path))
            {
                Some((kept_at, kept_by)) => build_kept_closure_commit(
                    repo,
                    item,
                    &path,
                    observation.base,
                    now,
                    &identity,
                    kept_at,
                    &kept_by,
                )?
                .map(|commit| (whole_utc_seconds(kept_at), commit)),
                None => None,
            };
            match built {
                Some(built) => built,
                None => {
                    return abandon(
                        Refusal::ReconfirmationRequired,
                        BranchAnswer::ReconfirmationRequired,
                    );
                }
            }
        }
        _ => return Err(TicketClosureError::RecoveryRequired),
    };
    if state::with_transaction(service, root, |tx, id| {
        if record::context_synchronization_unfinished(tx, id, item)? {
            return Ok(true);
        }
        record::begin_new_branch(
            tx,
            id,
            r.operation_id,
            observation.base,
            closed_at,
            commit,
            observation.tracking_context,
        )?;
        Ok(false)
    })? {
        return Err(TicketClosureError::Busy);
    }
    run::boundary(service, root, FailurePoint::NewBranchCommitRecorded)?;
    hold_commit(repo, r.operation_id, commit)?;
    run::boundary(service, root, FailurePoint::NewBranchCommitHeld)?;
    Ok(Recorded::Commit)
}

/// A new branch's end before it has a context: the record's pending ref is
/// deleted first, when it holds a commit, and the record ends in one
/// transaction with the refusal and the answer it stores. With `owner` it is
/// the record's own `close` reservation that ends with it, `cancelled`. A
/// record holds no ref once abandoned.
pub(in super::super) fn abandon(
    service: &RepositoryService,
    root: &Path,
    r: &ClosureRecord,
    refusal: Refusal,
    answer: BranchAnswer,
    owner: Option<&RemoteReservation>,
) -> Result<(), TicketClosureError> {
    if let Some(commit) = r.closure_oid {
        let repo =
            git2::Repository::open(root).map_err(|_| TicketClosureError::RecoveryRequired)?;
        release_commit(&repo, r.operation_id, commit)?;
    }
    state::with_transaction(service, root, |tx, id| match owner {
        None => record::abandon_branch_record(tx, id, r.operation_id, &refusal, answer, None, 0),
        Some(owner) => record::abandon_reserved_new_branch(
            tx,
            service,
            id,
            r.operation_id,
            &refusal,
            answer,
            owner,
        ),
    })
    .map_err(TicketClosureError::from)
}

/// Step 3: the context branch and its worktree are made at the commit, and
/// the pending ref, which held the commit until then, is let go. Each part is
/// idempotent, so a record at `closing` runs it again from wherever the last
/// call stopped.
fn make_context(
    service: &RepositoryService,
    request: &LifecycleRequest<ClosureExpectation>,
    r: &ClosureRecord,
    repo: &git2::Repository,
    root: &Path,
    commit: Oid,
) -> Result<(), TicketClosureError> {
    let item = &r.item_id;
    if r.step != ClosureStep::Closing {
        if run::cancelled::<TicketClosureLifecycle>(service, request, r)? {
            return Err(TicketClosureError::Interrupted);
        }
        if state::with_transaction(service, root, |tx, id| {
            if record::context_synchronization_unfinished(tx, id, item)? {
                return Ok(true);
            }
            record::begin_new_branch_closing(tx, id, r.operation_id)?;
            Ok(false)
        })? {
            return Err(TicketClosureError::Busy);
        }
    }
    // The ticket's context is marked owed an indexing before Git is touched.
    context_moving(service, root, item)?;
    service.context_at_commit(root, AuthoringKind::Ticket, item, commit)?;
    release_commit(repo, r.operation_id, commit)?;
    state::with_transaction(service, root, |tx, id| {
        record::clear_pending_ref(tx, id, r.operation_id)
    })?;
    context_moving(service, root, item)?;
    Ok(())
}

/// What step 2 came to: the old context retired, with the repository lease
/// back for step 3, or the record abandoned.
enum Retirement {
    Done(RepositoryLease),
    Abandoned,
}

/// Step 2: the old context is retired, in finish_steps' order. Its worktree
/// and branch are removed under the lease, with the record's step stored
/// between them; then, under the record's own `close` reservation (R-31,
/// taken for this part alone), the tracking ref is deleted and the
/// retirement committed, which ends the reservation with it.
fn retire(
    service: &RepositoryService,
    request: &LifecycleRequest<ClosureExpectation>,
    rec: &mut ClosureRecord,
    repo: &git2::Repository,
    root: &Path,
    lease: RepositoryLease,
) -> Result<Retirement, TicketClosureError> {
    // The configuration the record's base holds, as every step is checked.
    let primary = run::recorded_config(root, rec)?.primary_branch;
    if rec.step != ClosureStep::BranchDeleted {
        let cleanup = ContextCleanup::holding(root, lease)?;
        if !retire_locally(service, request, rec, &cleanup, root, &primary)? {
            return Ok(Retirement::Abandoned);
        }
        // The reservation waits for holders, which may need the lease.
        drop(cleanup);
    } else {
        drop(lease);
    }
    retire_remotely(service, rec, repo, root, &primary)
}

/// The configuration is the record's: the remote and the primary branch it
/// was made for. Anything else is `recovery_required`, and the way out is
/// restoring it (R-35).
fn configured(root: &Path, primary: &str, remote: Option<&str>) -> Result<(), TicketClosureError> {
    run::require_configuration(root, primary, remote)
        .map_err(|_| TicketClosureError::RecoveryRequired)
}

/// A stop of the old context's removal: the record stays at its step with
/// the category, holding nothing remote, and the same request retries it.
fn stop(
    service: &RepositoryService,
    root: &Path,
    rec: &mut ClosureRecord,
    category: CleanupCategory,
) -> TicketClosureError {
    rec.cleanup = Some(category);
    match run::save::<TicketClosureLifecycle>(service, root, rec) {
        Ok(()) => TicketClosureError::RecoveryRequired,
        Err(error) => error.into(),
    }
}

/// Step 2's local part, under the lease the cleanup holds, up to
/// `branch_deleted`. `false` when the old context's full check refused and the
/// record was abandoned.
fn retire_locally(
    service: &RepositoryService,
    request: &LifecycleRequest<ClosureExpectation>,
    rec: &mut ClosureRecord,
    cleanup: &ContextCleanup,
    root: &Path,
    primary: &str,
) -> Result<bool, TicketClosureError> {
    let item = rec.item_id.clone();
    let oid = rec
        .retired_oid
        .ok_or(TicketClosureError::RecoveryRequired)?;
    // A context with a tip the consent bound; a deleted history with none
    // has no worktree or branch to check.
    let old_context = rec.consented_context_oid.is_some();
    let remote = rec.remote_name.clone();
    let cancelled = |rec: &ClosureRecord| -> Result<(), TicketClosureError> {
        if run::cancelled::<TicketClosureLifecycle>(service, request, rec)? {
            return Err(TicketClosureError::Interrupted);
        }
        Ok(())
    };
    cancelled(rec)?;
    configured(root, primary, remote.as_deref())?;
    let mut fresh = false;
    if rec.step == ClosureStep::Confirmed {
        if old_context
            && cleanup.full_check(service, AuthoringKind::Ticket, &item, oid) != Precheck::Ok
        {
            abandon(
                service,
                root,
                rec,
                Refusal::ReconfirmationRequired,
                BranchAnswer::ContextChanged,
                None,
            )?;
            return Ok(false);
        }
        // The ticket's context is marked owed an indexing before Git is
        // touched.
        context_moving(service, root, &item)?;
        rec.step = ClosureStep::WorktreeRemoving;
        rec.cleanup = None;
        run::save::<TicketClosureLifecycle>(service, root, rec)?;
        fresh = true;
    }
    if rec.step == ClosureStep::WorktreeRemoving {
        run::boundary(service, root, FailurePoint::DuringWorktreeRemoval)?;
        cancelled(rec)?;
        configured(root, primary, remote.as_deref())?;
        let removal = if fresh && old_context {
            cleanup.remove_fresh_context_worktree(service, AuthoringKind::Ticket, &item, oid)
        } else {
            cleanup.remove_context_worktree(service, AuthoringKind::Ticket, &item, oid)
        };
        match removal {
            WorktreeRemoval::Removed => {
                #[cfg(test)]
                run_tests::local_effect(service, root, "worktree_remove");
            }
            WorktreeRemoval::AlreadyAbsent => (),
            WorktreeRemoval::Changed => {
                return Err(stop(service, root, rec, CleanupCategory::ContextChanged));
            }
            WorktreeRemoval::RecoveryRequired => {
                return Err(stop(service, root, rec, CleanupCategory::RecoveryRequired));
            }
        }
        rec.step = ClosureStep::WorktreeRemoved;
        rec.cleanup = None;
        run::save::<TicketClosureLifecycle>(service, root, rec)?;
    }
    cancelled(rec)?;
    if rec.step == ClosureStep::WorktreeRemoved {
        run::boundary(service, root, FailurePoint::BeforeBranchDeletion)?;
        cancelled(rec)?;
        configured(root, primary, remote.as_deref())?;
        match cleanup.delete_context_branch(AuthoringKind::Ticket, &item, oid) {
            BranchDeletion::Deleted => {
                #[cfg(test)]
                run_tests::local_effect(service, root, "branch_delete");
            }
            BranchDeletion::AlreadyAbsent => (),
            BranchDeletion::Changed => {
                return Err(stop(service, root, rec, CleanupCategory::ContextChanged));
            }
        }
        rec.step = ClosureStep::BranchDeleted;
        rec.cleanup = None;
        run::save::<TicketClosureLifecycle>(service, root, rec)?;
    }
    cancelled(rec)?;
    if rec.step != ClosureStep::BranchDeleted {
        return Err(TicketClosureError::RecoveryRequired);
    }
    configured(root, primary, remote.as_deref())?;
    run::boundary(service, root, FailurePoint::BeforeCompletion)?;
    cancelled(rec)?;
    Ok(true)
}

/// Step 2's remote part: the record's own `close` reservation, taken for this
/// alone, with no transport. Under it the evidence of the deletion is read
/// again, the tracking ref is deleted, and the retirement is committed in one
/// transaction with the reservation's end (U-1). A change found under it
/// abandons the record and ends the reservation `cancelled`; nothing here
/// holds it across a stop but a Git or SQL error, which the retry restarts.
fn retire_remotely(
    service: &RepositoryService,
    rec: &mut ClosureRecord,
    repo: &git2::Repository,
    root: &Path,
    primary: &str,
) -> Result<Retirement, TicketClosureError> {
    let (item, op) = (rec.item_id.clone(), rec.operation_id);
    let oid = rec
        .retired_oid
        .ok_or(TicketClosureError::RecoveryRequired)?;
    let remote = rec
        .remote_name
        .clone()
        .ok_or(TicketClosureError::RecoveryRequired)?;
    configured(root, primary, Some(&remote))?;
    let plan = RemoteRefPlan::from_configuration(&remote, primary)
        .map_err(|_| TicketClosureError::RecoveryRequired)?;
    let frozen = service.observation_configuration(root, &plan)?;
    state::with_transaction(service, root, |tx, id| {
        state::configure_endpoints(tx, id, &plan, &frozen.endpoint_digest())
    })?;
    let target = RemoteOperationTarget::for_context(
        &plan,
        KIND.action(),
        AuthoringKind::Ticket,
        item.clone(),
    )
    .map_err(|_| TicketClosureError::RecoveryRequired)?;
    let exists = state::with_transaction(service, root, |tx, id| {
        Ok(state::read_operation(tx, id, op)?.is_some())
    })?;
    // A new reservation waits for a live holder in its way, holding nothing;
    // a restart takes up its own row.
    let reserved = if exists {
        service.restart_remote_lifecycle(KIND, root, op, &target)
    } else {
        service.reserving(root, op, || {
            service.reserve_remote_operation(root, op, &target)
        })
    }?;
    let owner = match reserved {
        RemoteReservationOutcome::Reserved(owner) => owner,
        RemoteReservationOutcome::Busy => return Err(TicketClosureError::Busy),
        RemoteReservationOutcome::PollYielding => return Err(TicketClosureError::PollYielding),
        // The row ends with the record's retirement, in one transaction: a
        // completed row under a record that is not retired is not this
        // call's to continue.
        RemoteReservationOutcome::Replay(_) => return Err(TicketClosureError::RecoveryRequired),
    };
    // Nothing is fetched or pushed between the safe points: the part that
    // follows is local, and `finish_parent` needs the second.
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
    ] {
        service
            .remote_safe_point(root, &owner, point)
            .map_err(|_| TicketClosureError::RecoveryRequired)?;
    }
    let cleanup = ContextCleanup::acquire(root)?;
    run::validate_cleanup_owner(
        service,
        root,
        Some(&owner),
        Some(&plan),
        Some(&frozen),
        primary,
    )
    .map_err(|_| TicketClosureError::RecoveryRequired)?;
    // A change found from here is an abandon under the reservation.
    let changed = || {
        abandon(
            service,
            root,
            rec,
            Refusal::ExternalChange,
            BranchAnswer::ExternalChange,
            Some(&owner),
        )
    };
    // The deletion is still evidenced, and the tracking ref is the one the
    // consent bound. A fetch never prunes it, so one that is gone was deleted
    // by this record's own earlier attempt, under the evidence it bound.
    let tracking = plan.context(AuthoringKind::Ticket, &item);
    let tracked = match repo.find_reference(tracking.tracking_ref()) {
        Ok(reference) if reference.symbolic_target().is_none() => reference.target(),
        Ok(_) => return Err(TicketClosureError::RecoveryRequired),
        Err(error) if error.code() == git2::ErrorCode::NotFound => None,
        Err(_) => return Err(TicketClosureError::RecoveryRequired),
    };
    let evidenced = match (tracked, rec.remote_context_oid) {
        (None, Some(_)) => true,
        (tracked, bound) if tracked == bound => {
            state::with_transaction(service, root, |tx, id| {
                sync::republish_evidence_in(tx, id, root, &plan, AuthoringKind::Ticket, &item)
                    .map(|(evidence, ..)| evidence.deleted())
            })?
        }
        _ => false,
    };
    if !evidenced {
        changed()?;
        return Ok(Retirement::Abandoned);
    }
    match cleanup.require_local_cleanup_absent(AuthoringKind::Ticket, &item) {
        Ok(()) => (),
        Err(TrackingDeletion::Changed) => {
            changed()?;
            return Ok(Retirement::Abandoned);
        }
        Err(TrackingDeletion::RecoveryRequired) => {
            return Err(TicketClosureError::RecoveryRequired);
        }
    }
    let evidence = match cleanup.delete_tracking_ref(
        Some(&plan),
        AuthoringKind::Ticket,
        &item,
        rec.remote_context_oid,
        op,
        oid,
    ) {
        Ok(evidence) => evidence,
        Err(TrackingDeletion::Changed) => {
            changed()?;
            return Ok(Retirement::Abandoned);
        }
        Err(TrackingDeletion::RecoveryRequired) => {
            return Err(TicketClosureError::RecoveryRequired);
        }
    };
    #[cfg(test)]
    run_tests::tracking_deleted(service, root);
    run::boundary(service, root, FailurePoint::PromotionAfterTrackingDeletion)?;
    let committed = state::with_transaction(service, root, |tx, id| {
        // Reappearance after the Git effect is a refusal too: the transaction
        // rolls back, with the reservation and the record as they were.
        cleanup
            .require_local_cleanup_absent(AuthoringKind::Ticket, &item)
            .map_err(|_| state::recovery_required())?;
        record::retire_new_branch(tx, service, id, op, &evidence, &owner)
    });
    #[cfg(test)]
    let committed = run_tests::completion_returned(service, root, committed);
    let committed = committed
        .and_then(|()| run::boundary(service, root, FailurePoint::PromotionAfterCompletionCommit));
    // A commit error does not prove rollback: the record and its row say.
    if committed.is_err()
        && !state::with_transaction(service, root, |tx, id| {
            record::reconcile_retirement(tx, id, op)
        })
        .unwrap_or(false)
    {
        return Err(TicketClosureError::RecoveryRequired);
    }
    drop(evidence);
    *rec = run::load::<TicketClosureLifecycle>(service, root, op)?;
    Ok(Retirement::Done(cleanup.into_lease()))
}

/// The record's pending ref points at `commit`: made when absent, kept when
/// there. The name is the record's own, and the record is written before the
/// ref, so a ref anywhere else is not Manyhands'.
fn hold_commit(
    repo: &git2::Repository,
    operation: OperationId,
    commit: Oid,
) -> Result<(), TicketClosureError> {
    // On a retry the commit is the record's: it must still be here.
    repo.find_commit(commit)
        .map_err(|_| TicketClosureError::RecoveryRequired)?;
    let name = record::pending_ref_name(operation);
    match repo.find_reference(&name) {
        Ok(held) if held.target() == Some(commit) => Ok(()),
        Ok(_) => Err(TicketClosureError::RecoveryRequired),
        Err(error) if error.code() == git2::ErrorCode::NotFound => repo
            .reference(&name, commit, false, "hold a new branch's closure")
            .map(drop)
            .map_err(|_| TicketClosureError::RecoveryRequired),
        Err(_) => Err(TicketClosureError::RecoveryRequired),
    }
}

/// The record's pending ref is deleted when it points at `commit`; absent is
/// fine, and anywhere else is not Manyhands'.
fn release_commit(
    repo: &git2::Repository,
    operation: OperationId,
    commit: Oid,
) -> Result<(), TicketClosureError> {
    match repo.find_reference(&record::pending_ref_name(operation)) {
        Ok(mut held) if held.target() == Some(commit) => held
            .delete()
            .map_err(|_| TicketClosureError::RecoveryRequired),
        Ok(_) => Err(TicketClosureError::RecoveryRequired),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(()),
        Err(_) => Err(TicketClosureError::RecoveryRequired),
    }
}
