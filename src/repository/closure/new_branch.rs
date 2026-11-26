//! A close that makes a new branch (Part F): the ticket's context is not the
//! one the consent saw, so the closure is a commit over the tracking primary,
//! held by a pending ref of the record's own, and the context is made at it.
//! This is steps 1 and 3 of the design; step 2, retiring an old context, is
//! not here yet: a new branch with something to retire does not reach this
//! module.
//!
//! Step 1 builds the commit, records it and the hold it gives the ticket, and
//! makes the pending ref. Step 3 makes the context branch and its worktree at
//! the commit, then lets the pending ref go. Every write is recorded before
//! the Git state it describes, so a retry from any stop finds what the last
//! call left, and never builds a second commit.
use super::{
    BranchAnswer, ClosureAction, ClosureExpectation, PlannedClosure, PreviewTicketClosureRequest,
    TicketClosureError,
    checkpoint::{build_closure_commit, context_moving, whole_utc_seconds},
    lifecycle::TicketClosureLifecycle,
    observe::{self, PreparedClosureObservation, closed_by_text, ticket_path},
    record::{self, ClosureRecord, ClosureStep, Refusal},
};
use crate::{
    canonical::Integration,
    repository::{
        AuthoringKind, ClosureBlocker, FailurePoint, OperationId, RepositoryErrorKind,
        RepositoryOperation, RepositoryService, canonical_repository_root,
        promotion::{
            integrate,
            lifecycle::{BranchStep, LifecycleRequest},
            run,
        },
        remote::state,
        repository_lease,
    },
};
use git2::Oid;
use std::path::Path;

/// A new branch's step 1 and step 3, under the repository lease (R-34), as
/// the runner calls it for a record whose action is `new_branch`: with the
/// commit not yet recorded, the closure is built and recorded, or the record
/// ends with the answer that says why not; then the context is made at the
/// commit. Nothing here merges into the primary, or moves it.
///
/// An abandon is one transaction under the lease, and only before the commit
/// is recorded: from then on the record holds the ticket and runs on.
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
    let _lease = repository_lease(&repo, root, RepositoryOperation::RepositorySnapshot)?;
    let commit = match &prepared {
        Some(prepared) => match record_commit(service, request, r, prepared, &repo, root)? {
            Recorded::Commit(commit) => commit,
            Recorded::Abandoned => return Ok(BranchStep::Finished),
        },
        None => {
            let commit = r.closure_oid.ok_or(TicketClosureError::RecoveryRequired)?;
            // A retry from before the pending ref was made.
            if r.step == ClosureStep::Confirmed {
                hold_commit(&repo, r.operation_id, commit)?;
            }
            commit
        }
    };
    make_context(service, request, r, &repo, root, commit)?;
    Ok(BranchStep::Closing)
}

enum Recorded {
    Commit(Oid),
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
        state::with_transaction(service, root, |tx, id| {
            record::abandon_branch_record(
                tx,
                id,
                r.operation_id,
                &refusal,
                answer,
                None,
                now.unix_timestamp(),
            )
        })
        .map(|()| Recorded::Abandoned)
        .map_err(TicketClosureError::from)
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
    let PlannedClosure::Record { closed_by } = &observation.closure else {
        return Err(TicketClosureError::RecoveryRequired);
    };
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
    if closed_by_text(&identity) != *closed_by {
        return abandon(Refusal::ExternalChange, BranchAnswer::ExternalChange);
    }
    let Some(built) = build_closure_commit(
        repo,
        item,
        &ticket_path(item),
        observation.base,
        whole_utc_seconds(now),
        &identity,
    )?
    else {
        return abandon(
            Refusal::ReconfirmationRequired,
            BranchAnswer::ReconfirmationRequired,
        );
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
            built.closed_at,
            built.commit,
            observation.tracking_context,
        )?;
        Ok(false)
    })? {
        return Err(TicketClosureError::Busy);
    }
    run::boundary(service, root, FailurePoint::NewBranchCommitRecorded)?;
    hold_commit(repo, r.operation_id, built.commit)?;
    run::boundary(service, root, FailurePoint::NewBranchCommitHeld)?;
    Ok(Recorded::Commit(built.commit))
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
