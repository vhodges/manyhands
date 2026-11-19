//! `operation resume` of a parent: the design's resume table, a `c5_resume_`
//! case per row and verb.

use std::{path::Path, sync::Arc};

use super::c5_lifecycle::*;
use super::*;
use lifecycle_world::*;
use results::{CleanupEffect, IntegrationEffect};

const RESUMES: &[ssh_harness::Case] = both_verbs!["c5_resume_";
    "held_by_its_request" => resume_held_by_its_request,
    "no_record_local" => resume_no_record_local,
    "no_record_pre_integration" => resume_no_record_pre_integration,
    "no_record_past_integration" => resume_no_record_past_integration,
    "stop_leaves_no_record" => resume_stop_leaves_no_record,
    "running_elsewhere" => resume_running_elsewhere,
    "complete" => resume_complete,
    "complete_with_record_accepted" => resume_complete_with_record_accepted,
    "refused_and_ended" => resume_refused_and_ended,
    "superseded" => resume_superseded,
    "grants_no_new_consent" => resume_grants_no_new_consent,
    "pending_child" => resume_pending_child,
    "ctrl_c" => resume_ctrl_c,
    "entered_again" => resume_entered_again,
    "index_pending" => resume_index_pending,
    "child_identity_stop" => resume_child_identity_stop,
    "cleanup_recovery" => resume_cleanup_recovery,
    "superseded_no_record" => resume_superseded_no_record,
    "refused_no_record" => resume_refused_no_record,
    "before_integration_ref" => resume_before_integration_ref,
    "after_integration_ref" => resume_after_integration_ref,
    "ctrl_c_releases_child" => resume_ctrl_c_releases_child,
    "request_cancel_asks_nothing" => resume_request_cancel_asks_nothing,
    "walk_remote_branch_deleted" => resume_walk_remote_branch_deleted,
    "kept_child_stays" => resume_kept_child_stays,
    "concurrent_same_id" => resume_concurrent_same_id,
    "prepare_names_the_resume" => prepare_names_the_resume,
];

const RESUMES_ONE: &[ssh_harness::Case] = &[
    (
        "c5_resume_no_record_checkpoint_close",
        resume_no_record_checkpoint,
    ),
    ("c5_resume_identity_stop_promote", resume_identity_stop),
    (
        "c5_resume_identity_after_checkpoint_close",
        resume_identity_after_checkpoint,
    ),
    (
        "c5_resume_refused_checkpoint_merge_close",
        resume_refused_checkpoint_merge,
    ),
];

pub(crate) const CASES: &[ssh_harness::Case] =
    &joined_cases::<{ RESUMES.len() + RESUMES_ONE.len() }>(RESUMES, RESUMES_ONE);

// The design's resume table. A parent
// no request record holds is begun through the domain's own entry point, as a
// caller that is not the boundary begins one, so it has neither a record nor,
// short of its deletion, a journal row.

const RESUME: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ5";

fn resuming(root: &Path, operation: &str) -> Mutation {
    Mutation::OperationResume(OperationResumeInput {
        root: root.to_owned(),
        operation_id: OperationId::parse(operation).unwrap(),
    })
}

/// One `operation resume`, checked as `called` checks, on no request of the
/// boundary but `id`.
fn resumed_under(
    service: &RepositoryService,
    id: &str,
    root: &Path,
    operation: &str,
) -> Envelope<MutationDataDto> {
    let envelope = called(service, id, None, resuming(root, operation));
    assert_eq!(envelope.command, "operation resume");
    assert_eq!(envelope.operation_id, None, "the operation is another's");
    envelope
}

fn operation_named(envelope: &Envelope<MutationDataDto>) -> Result<String, FixtureError> {
    envelope.operation_id.clone().ok_or(FixtureError)
}

/// A remote world whose parent was begun outside the boundary and stopped at
/// `point`.
fn outside_remote(verb: Verb, point: FailurePoint) -> Result<(World, String), FixtureError> {
    outside_in(remote(verb)?, verb, point)
}

/// `outside_remote` in the world given.
fn outside_in(
    mut w: World,
    verb: Verb,
    point: FailurePoint,
) -> Result<(World, String), FixtureError> {
    let failing = failing_service(&w.data(), point)?;
    let operation = begun_outside(&failing, verb, &w.root)?;
    w.service = clean_service(&w.data())?;
    assert_eq!(parents(&w.db()?, verb)?, 1);
    Ok((w, operation))
}

/// The same on a world with no remote.
fn outside_local(verb: Verb, point: FailurePoint) -> Result<(Local, String), FixtureError> {
    let mut local = Local::holding(verb)?;
    let failing = failing_service(&local_data(&local), point)?;
    let operation = begun_outside(&failing, verb, &local.root)?;
    local.service = clean_service(&local_data(&local))?;
    assert_eq!(parents(&local.db()?, verb)?, 1);
    Ok((local, operation))
}

/// A resume refused before anything is recorded of it.
fn assert_refused_resume(
    envelope: &Envelope<MutationDataDto>,
    code: ResultCode,
    state: Option<RequestState>,
) {
    assert_eq!(envelope.code, code, "{:?}", envelope.data);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.data, None);
    assert_eq!(state, None, "no record of the resume");
}

/// A finished resume: its data is the lifecycle's, and a replay gives it back
/// unchanged, by the tag `from_stored` reads.
fn assert_finished_resume(
    service: &RepositoryService,
    root: &Path,
    operation: &str,
    done: &Envelope<MutationDataDto>,
    state: Option<RequestState>,
) -> Result<(), FixtureError> {
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(state, Some(RequestState::Finished));
    assert!(lifecycle_of(done).is_some(), "the lifecycle's data");
    let again = resumed_under(service, RESUME, root, operation);
    assert_eq!(again.code, ResultCode::Ok);
    assert_eq!(again.data, done.data, "the replay's data");
    assert_eq!(again.effects, done.effects);
    Ok(())
}

/// An accepted request record holds it: that request goes on.
fn resume_held_by_its_request(verb: Verb) -> Result<(), FixtureError> {
    let (local, stopped) = local_stopped_at(verb, FailurePoint::AfterContextSync)?;
    let operation = operation_named(&stopped)?;
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Accepted));
    let refused = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_refused_resume(
        &refused,
        ResultCode::OriginalRequestRequired,
        local_state(&local, RESUME),
    );
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestRetry]);
    assert!(
        refused.recovery[0]
            .arguments
            .values()
            .any(|value| value == FIRST),
        "names the request: {:?}",
        refused.recovery
    );
    assert_eq!(merges_on_primary(&local.repo()?)?, 0);
    let done = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    Ok(())
}

/// No record, no journal row, no remote: continued with the record's
/// expectation, past ten minutes, since nothing new is consented to.
fn resume_no_record_local(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    let clock = RowClock::now();
    clock.past_ten_minutes();
    let later = clean_service(&local_data(&local))?.with_clock_for_testing(clock);
    let done = resumed_under(&later, RESUME, &local.root, &operation);
    assert_finished_resume(
        &later,
        &local.root,
        &operation,
        &done,
        local_state(&local, RESUME),
    )?;
    assert_eq!(done.effects.integration, IntegrationEffect::Complete);
    assert_eq!(done.effects.publication, PublicationEffect::Pending);
    let data = lifecycle_of(&done).ok_or(FixtureError)?;
    assert_eq!(data.item_id, verb.id());
    assert_eq!(
        data.publication_pending,
        Some(LifecyclePublicationPendingReason::NoPublicationRemote)
    );
    let repo = local.repo()?;
    assert_eq!(merges_on_primary(&repo)?, 1);
    assert!(repo.refname_to_id(verb.branch()).is_err());
    assert!(!verb.worktree(&local.root).exists());
    Ok(())
}

/// Before integration, on a remote, with the child's synchronization done.
fn resume_no_record_pre_integration(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = outside_remote(verb, FailurePoint::AfterContextSync)?;
    let done = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_finished_resume(&w.service, &w.root, &operation, &done, state_of(&w, RESUME))?;
    assert_eq!(done.effects.publication, PublicationEffect::Published);
    assert_eq!(done.effects.cleanup, CleanupEffect::Complete);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// Past integration only publication's remainder and cleanup run: nothing
/// repeats a merge or a push.
fn resume_no_record_past_integration(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = outside_remote(verb, FailurePoint::AfterPublication)?;
    let main = fixed(w.repo()?.refname_to_id(MAIN))?;
    let pushes = accepted(&w.server);
    let done = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_finished_resume(&w.service, &w.root, &operation, &done, state_of(&w, RESUME))?;
    assert_eq!(done.effects.cleanup, CleanupEffect::Complete);
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, main);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    assert_eq!(
        accepted(&w.server),
        pushes + 1,
        "the context's deletion and nothing else"
    );
    assert!(w.repo()?.refname_to_id(verb.branch()).is_err());
    Ok(())
}

/// Decision 33: a parent no request record holds refuses a new prepare, and the
/// way on it names is a resume of that operation. Once resumed, nothing is
/// left for a new prepare to do.
fn prepare_names_the_resume(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = outside_remote(verb, FailurePoint::AfterPublication)?;
    let refused = w.service.prepare(&verb.mutation(&w.root, verb.id()));
    assert_eq!(
        refused.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        refused.data
    );
    assert_eq!(refused.data, None);
    assert_eq!(
        refused
            .recovery
            .iter()
            .map(|a| a.action)
            .collect::<Vec<_>>(),
        [RecoveryActionKind::OperationResume]
    );
    assert_eq!(
        refused.recovery[0].operation_id.as_deref(),
        Some(operation.as_str())
    );
    let done = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_finished_resume(&w.service, &w.root, &operation, &done, state_of(&w, RESUME))?;
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    let again = w.service.prepare(&verb.mutation(&w.root, verb.id()));
    assert_ne!(again.code, ResultCode::Ok, "{:?}", again.data);
    Ok(())
}

/// A written checkpoint and no integration: the closure keeps its one
/// checkpoint.
fn resume_no_record_checkpoint() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let (w, operation) = outside_remote(verb, FailurePoint::AfterClosureCheckpoint)?;
    let done = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_finished_resume(&w.service, &w.root, &operation, &done, state_of(&w, RESUME))?;
    let data = lifecycle_of(&done).ok_or(FixtureError)?;
    assert_eq!(data.item_id, verb.id());
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// A resume that stops leaves no holding record: the parent is what stands,
/// and a later resume, under any ID, continues it.
fn resume_stop_leaves_no_record(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = outside_remote(verb, FailurePoint::AfterConfirmed)?;
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = resumed_under(&w.service, RESUME, &w.root, &operation);
    w.server.clear_fault();
    assert_eq!(
        stopped.code,
        ResultCode::TransportUnavailable,
        "{:?}",
        stopped.data
    );
    assert_eq!(state_of(&w, RESUME), None, "no holding record");
    assert_eq!(actions(&stopped)[0], RecoveryActionKind::OperationResume);
    assert_eq!(
        stopped.recovery[0].operation_id.as_deref(),
        Some(operation.as_str())
    );
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    let done = resumed_under(&w.service, SECOND, &w.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(state_of(&w, SECOND), Some(RequestState::Finished));
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// Whoever is running it holds the lease: the domain's `busy`, and no liveness
/// check of the route's own.
fn resume_running_elsewhere(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    let held = fixed(RepositoryService::hold_lease_for_testing(
        &local.root,
        &local_data(&local),
        LeaseKind::Repository,
    ))?;
    let busy = resumed_under(&local.service, RESUME, &local.root, &operation);
    drop(held);
    assert_eq!(busy.code, ResultCode::Busy, "{:?}", busy.data);
    assert_eq!(local_state(&local, RESUME), None, "no holding record");
    assert_eq!(actions(&busy), [RecoveryActionKind::OperationResume]);
    assert_eq!(merges_on_primary(&local.repo()?)?, 0);
    let done = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

fn resume_complete(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let done = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    let operation = operation_named(&done)?;
    let refused = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_refused_resume(
        &refused,
        ResultCode::AlreadyApplied,
        local_state(&local, RESUME),
    );
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    Ok(())
}

/// Complete with its result lost: `already_applied` all the same, and the
/// request that holds it still gets its stored outcome.
fn resume_complete_with_record_accepted(verb: Verb) -> Result<(), FixtureError> {
    let (local, _) = local_stopped_at(verb, FailurePoint::BeforeRequestSettlement)?;
    // A call that dies there names no operation.
    let operation: String = fixed(local.db()?.query_row(
        &format!("SELECT operation_id FROM {}", table(verb)),
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Accepted));
    let refused = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_refused_resume(
        &refused,
        ResultCode::AlreadyApplied,
        local_state(&local, RESUME),
    );
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Accepted));
    let done = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

/// A parent a consent refusal ended: nothing is left to continue.
fn resume_refused_and_ended(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let root = local.root.clone();
    local.service.set_request_hook_for_testing(move || {
        edit_context(verb, &root).expect("an edited context");
    });
    let ended = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(ended.code, ResultCode::ExternalChange);
    let operation = operation_named(&ended)?;
    assert_eq!(parents(&local.db()?, verb)?, 1);
    let refused = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_refused_resume(
        &refused,
        ResultCode::InvalidInput,
        local_state(&local, RESUME),
    );
    assert_eq!(refused.recovery, []);
    Ok(())
}

/// A superseded parent, under either the old record or none: nothing is left
/// to continue, and the record that holds it is not named.
fn resume_superseded(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let (_, stopped) = stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    let operation = operation_named(&stopped)?;
    let untracked = verb.worktree(&w.root).join("untracked.txt");
    fixed(std::fs::write(&untracked, b"the owner's\n"))?;
    assert_eq!(
        resent(&w.service, FIRST, verb, &w.root).code,
        ResultCode::RecoveryRequired
    );
    fixed(std::fs::remove_file(&untracked))?;
    poll(&w)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let replaced = called(
        &w.service,
        SECOND,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(replaced.code, ResultCode::Ok, "{:?}", replaced.data);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    let refused = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_refused_resume(&refused, ResultCode::InvalidInput, state_of(&w, RESUME));
    assert_eq!(refused.recovery, []);
    Ok(())
}

/// What was consented to moved before integration began: the domain's own
/// check refuses it, the record ends, and a resume then has nothing to go on
/// with. A resume is no new consent.
fn resume_grants_no_new_consent(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    edit_context(verb, &local.root)?;
    let refused = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(
        refused.code,
        ResultCode::ExternalChange,
        "{:?}",
        refused.data
    );
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestPrepare]);
    let data = lifecycle_of(&refused).ok_or(FixtureError)?;
    assert_eq!(data.item_id, verb.id());
    assert_eq!(data.kind, verb_kind(verb));
    assert_eq!(merges_on_primary(&local.repo()?)?, 0);
    assert_eq!(
        local_state(&local, RESUME),
        Some(RequestState::Finished),
        "the parent's end is final"
    );
    // A replay gives what was stored, and no actions are stored (R-31).
    let replayed = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(replayed.code, ResultCode::ExternalChange);
    assert_eq!(replayed.data, refused.data);
    assert_eq!(replayed.recovery, []);
    let again = resumed_under(&local.service, SECOND, &local.root, &operation);
    assert_refused_resume(
        &again,
        ResultCode::InvalidInput,
        local_state(&local, SECOND),
    );
    assert_eq!(again.recovery, []);
    Ok(())
}

/// A pending child's ID keeps what a resume of a synchronization does: it
/// publishes the context and goes no further. The parent's request then goes
/// on, with one merge.
fn resume_pending_child(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    w.server.clear_fault();
    assert_eq!(stopped.code, ResultCode::TransportUnavailable);
    let operation = operation_named(&stopped)?;
    let child: String = fixed(fixed(w.db())?.query_row(
        &format!(
            "SELECT context_sync_id FROM {} WHERE operation_id = ?1",
            table(verb)
        ),
        [operation],
        |row| row.get(0),
    ))?;
    let main = fixed(w.repo()?.refname_to_id(MAIN))?;
    let taken_up = child_resumed(&w, &child);
    assert_eq!(taken_up.command, "operation resume");
    assert_eq!(taken_up.code, ResultCode::Ok, "{:?}", taken_up.data);
    assert!(
        matches!(
            taken_up.data,
            None | Some(MutationDataDto::Synchronization(_))
        ),
        "a synchronization's, not a lifecycle's: {:?}",
        taken_up.data
    );
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, main, "no merge");
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// Not `resumed_under`, which scans for a lifecycle's data.
fn child_resumed(w: &World, child: &str) -> Envelope<MutationDataDto> {
    w.service.execute(
        MutationCall {
            request_id: request(RESUME),
            confirmation: None,
            mutation: resuming(&w.root, child),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// Ctrl-C is as the verb's, at the first safe point once accepted: `cancelled`,
/// no merge, and the parent stands for the next resume.
fn resume_ctrl_c(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    local
        .service
        .set_request_hook_for_testing(move || token.cancel());
    let stopped = local.service.execute(
        MutationCall {
            request_id: request(RESUME),
            confirmation: None,
            mutation: resuming(&local.root, &operation),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &cancel,
    );
    assert_eq!(stopped.code, ResultCode::Cancelled, "{:?}", stopped.data);
    assert_eq!(local_state(&local, RESUME), None, "no holding record");
    assert_eq!(actions(&stopped), [RecoveryActionKind::OperationResume]);
    assert_eq!(merges_on_primary(&local.repo()?)?, 0);
    let done = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    Ok(())
}

/// A resume that died once accepted is entered again and continues; one that
/// died after finishing the parent gets the stored outcome (R-33).
fn resume_entered_again(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    let dies = failing_service(&local_data(&local), FailurePoint::BeforeRequestDomainCall)?;
    let died = resumed_under(&dies, RESUME, &local.root, &operation);
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(local_state(&local, RESUME), Some(RequestState::Accepted));
    let lost = failing_service(&local_data(&local), FailurePoint::BeforeRequestSettlement)?;
    let died = resumed_under(&lost, RESUME, &local.root, &operation);
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    assert_eq!(local_state(&local, RESUME), Some(RequestState::Accepted));
    // The stored outcome and its report, from no transport: a local world has
    // none to reach.
    let answered = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(answered.code, ResultCode::Ok, "{:?}", answered.data);
    assert_eq!(answered.effects.integration, IntegrationEffect::Complete);
    assert!(lifecycle_of(&answered).is_some(), "the stored report");
    assert_eq!(local_state(&local, RESUME), Some(RequestState::Finished));
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    // A fresh resume of the same parent stays a no-op.
    let fresh = resumed_under(&local.service, SECOND, &local.root, &operation);
    assert_refused_resume(
        &fresh,
        ResultCode::AlreadyApplied,
        local_state(&local, SECOND),
    );
    Ok(())
}

/// The hand-off owed by a parent a resume took to its end: the answer the
/// verb gives, no record, and the parent is complete for the next resume.
fn resume_index_pending(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    let failing = failing_service(&local_data(&local), FailurePoint::RefreshFailure)?;
    let pending = resumed_under(&failing, RESUME, &local.root, &operation);
    assert_eq!(
        pending.code,
        ResultCode::DiscoveryPending,
        "{:?}",
        pending.data
    );
    assert_eq!(local_state(&local, RESUME), None, "no holding record");
    assert!(lifecycle_of(&pending).is_some());
    assert_eq!(actions(&pending), [RecoveryActionKind::IndexRefresh]);
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    let next = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_refused_resume(
        &next,
        ResultCode::AlreadyApplied,
        local_state(&local, RESUME),
    );
    Ok(())
}

fn verb_kind(verb: Verb) -> ItemDtoKind {
    match verb {
        Verb::Promote => ItemDtoKind::Document,
        Verb::Close => ItemDtoKind::Ticket,
    }
}

/// A promotion's merge with its identity gone: the resume first, then the
/// verb's own action. No request is offered, since none holds the parent.
fn resume_identity_stop() -> Result<(), FixtureError> {
    let verb = Verb::Promote;
    let (local, operation) = outside_local(verb, FailurePoint::AfterContextSync)?;
    forget_identity(&local.repo()?)?;
    let stopped = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_identity_stop(&stopped);
    assert_eq!(local_state(&local, RESUME), None, "no holding record");
    assert_eq!(merges_on_primary(&local.repo()?)?, 0);
    Ok(())
}

fn assert_identity_stop(stopped: &Envelope<MutationDataDto>) {
    assert_eq!(
        stopped.code,
        ResultCode::IdentityRequired,
        "{:?}",
        stopped.data
    );
    assert_eq!(
        actions(stopped),
        [
            RecoveryActionKind::OperationResume,
            RecoveryActionKind::RepoIdentitySet
        ]
    );
}

/// After a closure's checkpoint `closed_by` comes from the commit, so none is
/// needed (the design's Identity table): the closure finishes.
fn resume_identity_after_checkpoint() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let (local, operation) = outside_local(verb, FailurePoint::AfterClosureCheckpoint)?;
    forget_identity(&local.repo()?)?;
    let done = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    Ok(())
}

/// A context child that must merge a peer's commit, with the identity gone.
fn resume_child_identity_stop(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    fixed(w.sync(w.request(context_target(verb)?)))?;
    let peer = fixed(w.peer.workdir().ok_or(FixtureError).map(Path::to_owned))?;
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(&w.server));
    fixed(fixed(w.peer.find_remote("origin"))?.fetch(
        &[format!("+{0}:{0}", verb.branch())],
        Some(&mut fetch),
        None,
    ))?;
    commit_in(&peer, verb.branch(), "peer.txt", b"the peer's\n")?;
    push_peer(&w.peer, &w.server, verb.branch())?;
    commit_in(
        &verb.worktree(&w.root),
        verb.branch(),
        "own.txt",
        b"the owner's\n",
    )?;
    let failing = failing_service(&w.data(), FailurePoint::AfterConfirmed)?;
    let operation = begun_outside(&failing, verb, &w.root)?;
    w.service = clean_service(&w.data())?;
    forget_identity(&w.repo()?)?;
    let stopped = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_identity_stop(&stopped);
    assert_eq!(state_of(&w, RESUME), None, "no holding record");
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    Ok(())
}

/// `recovery_required` from cleanup: the whole list.
fn resume_cleanup_recovery(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = outside_remote(verb, FailurePoint::AfterPublication)?;
    let untracked = verb.worktree(&w.root).join("untracked.txt");
    fixed(std::fs::write(&untracked, b"the owner's\n"))?;
    let stopped = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_eq!(
        stopped.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        stopped.data
    );
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::OperationResume,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(state_of(&w, RESUME), None, "no holding record");
    fixed(std::fs::remove_file(&untracked))?;
    let done = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// A closure checkpoint's refused merge under a resume: the whole list, with
/// the blocker in the data.
fn resume_refused_checkpoint_merge() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let (w, operation) = outside_remote(verb, FailurePoint::AfterClosureCheckpoint)?;
    let conflicting = fixed(w.peer.workdir().ok_or(FixtureError).map(Path::to_owned))?;
    commit_in(
        &conflicting,
        MAIN,
        &format!(".manyhands/tickets/{ITEM}/ticket.md"),
        b"conflicting\n",
    )?;
    push_peer(&w.peer, &w.server, MAIN)?;
    poll(&w)?;
    let refused = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_eq!(
        refused.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        refused.data
    );
    assert_eq!(
        actions(&refused),
        [
            RecoveryActionKind::OperationResume,
            RecoveryActionKind::RepoInspect
        ]
    );
    let data = lifecycle_of(&refused).ok_or(FixtureError)?;
    assert!(
        data.blockers
            .contains(&LifecycleBlocker::IntegrationConflict),
        "{:?}",
        data.blockers
    );
    assert_eq!(state_of(&w, RESUME), None, "no holding record");
    Ok(())
}

/// Superseded with no request record ever holding it.
fn resume_superseded_no_record(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = outside_remote(verb, FailurePoint::AfterPublication)?;
    let untracked = verb.worktree(&w.root).join("untracked.txt");
    fixed(std::fs::write(&untracked, b"the owner's\n"))?;
    let stopped = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    fixed(std::fs::remove_file(&untracked))?;
    poll(&w)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let replaced = called(
        &w.service,
        SECOND,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(replaced.code, ResultCode::Ok, "{:?}", replaced.data);
    let refused = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_refused_resume(&refused, ResultCode::InvalidInput, state_of(&w, RESUME));
    assert_eq!(refused.recovery, []);
    Ok(())
}

/// A consent refusal ended it, with no request record ever holding it.
fn resume_refused_no_record(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let root = local.root.clone();
    let operation = begun_outside_after(&local.service, verb, &local.root, || {
        edit_context(verb, &root).map(|_| ())
    })?;
    assert_eq!(parents(&local.db()?, verb)?, 1);
    let refused = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_refused_resume(
        &refused,
        ResultCode::InvalidInput,
        local_state(&local, RESUME),
    );
    assert_eq!(refused.recovery, []);
    Ok(())
}

fn resume_integrating(verb: Verb, point: FailurePoint) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, point)?;
    let done = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    Ok(())
}

fn resume_before_integration_ref(verb: Verb) -> Result<(), FixtureError> {
    resume_integrating(verb, FailurePoint::BeforeIntegrationRef)
}

fn resume_after_integration_ref(verb: Verb) -> Result<(), FixtureError> {
    resume_integrating(verb, FailurePoint::AfterIntegrationRef)
}

/// A provider that runs `asks` when the child's connection asks for its key.
struct AskingProvider<F: FnMut()> {
    asks: F,
}

impl<F: FnMut()> SessionCredentialProvider for AskingProvider<F> {
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse {
        (self.asks)();
        Provider.request_passphrase(request)
    }
}

/// One `operation resume` with what a call is given.
fn resumed_with(
    w: &World,
    operation: &str,
    provider: impl SessionCredentialProvider,
    cancel: &CancellationToken,
) -> Envelope<MutationDataDto> {
    w.service.execute(
        MutationCall {
            request_id: request(RESUME),
            confirmation: None,
            mutation: resuming(&w.root, operation),
        },
        &mut SessionCredentials::new(provider),
        &mut NoProgress,
        cancel,
    )
}

/// A Ctrl-C while a resume's child runs lets the child go, as the verb's does.
fn resume_ctrl_c_releases_child(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = outside_in(protected(verb)?, verb, FailurePoint::AfterConfirmed)?;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let stopped = resumed_with(
        &w,
        &operation,
        AskingProvider {
            asks: || token.cancel(),
        },
        &cancel,
    );
    assert_eq!(stopped.code, ResultCode::Cancelled, "{:?}", stopped.data);
    assert_eq!(actions(&stopped), [RecoveryActionKind::OperationResume]);
    assert_eq!(state_of(&w, RESUME), None, "no holding record");
    assert!(!reserved(&w), "the child let its reservation go");
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    let done = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// `request cancel` asks nothing of a resume, also while it continues a parent.
fn resume_request_cancel_asks_nothing(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    let data = local_data(&local);
    let answered = Arc::new(std::sync::Mutex::new(None));
    let seen = Arc::clone(&answered);
    local.service.set_request_hook_for_testing(move || {
        let other = RepositoryService::open_at(&data).expect("a second service");
        let cancel = other.cancel_request(request(RESUME));
        *seen.lock().unwrap() = cancel.data.map(|dto| dto.requested);
    });
    let done = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(*answered.lock().unwrap(), Some(false), "requested: false");
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    Ok(())
}

/// A remote world whose context's branch was deleted at the remote after a
/// parent, no record holding it, was begun.
fn branch_deleted_outside(verb: Verb) -> Result<(World, String), FixtureError> {
    branch_deleted_outside_in(remote(verb)?, verb)
}

/// `branch_deleted_outside` in the world given.
fn branch_deleted_outside_in(mut w: World, verb: Verb) -> Result<(World, String), FixtureError> {
    fixed(w.sync(w.request(context_target(verb)?)))?;
    let failing = failing_service(&w.data(), FailurePoint::AfterConfirmed)?;
    let operation = begun_outside(&failing, verb, &w.root)?;
    w.service = clean_service(&w.data())?;
    fixed(fixed(w.bare()?.find_reference(verb.branch()))?.delete())?;
    Ok((w, operation))
}

const REPUBLISH: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ6";

/// Decision 19: with no request to cancel, the resume cancels and calls once
/// more, the runner releases the child, and the route out finishes.
fn resume_walk_remote_branch_deleted(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = branch_deleted_outside(verb)?;
    let stopped = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_eq!(
        stopped.code,
        ResultCode::RemoteBranchDeleted,
        "{:?}",
        stopped.data
    );
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::OperationResume,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(state_of(&w, RESUME), None, "no holding record");
    assert!(!reserved(&w), "the runner let the child go");

    let republish = Mutation::ItemRepublish(ItemRepublishInput {
        root: w.root.clone(),
        kind: match verb {
            Verb::Promote => AuthoringKind::Document,
            Verb::Close => AuthoringKind::Ticket,
        },
        id: fixed(verb.id().parse())?,
    });
    let preview = w.service.prepare(&republish);
    let consent = preview
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?
        .confirmation_id
        .clone();
    let republished = w.service.execute(
        MutationCall {
            request_id: request(REPUBLISH),
            confirmation: Some(ConfirmationId::parse(&consent).unwrap()),
            mutation: republish,
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(republished.code, ResultCode::Ok, "{:?}", republished.data);

    let done = resumed_under(&w.service, RESUME, &w.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// A child the guards keep: the answer is the same, and the reservation stays.
/// The row is marked while the first call's child runs, since nothing can come
/// between that call and the one after it.
fn resume_kept_child_stays(verb: Verb) -> Result<(), FixtureError> {
    let (w, operation) = branch_deleted_outside_in(protected(verb)?, verb)?;
    let stopped = resumed_with(
        &w,
        &operation,
        AskingProvider {
            asks: || {
                w.db()
                    .map_err(|_| rusqlite::Error::InvalidQuery)
                    .and_then(|db| {
                        db.execute(
                            "UPDATE remote_operation_records SET reconciliation_required = 1
                              WHERE phase NOT IN ('completed','interrupted','cancelled','failed')",
                            [],
                        )
                    })
                    .expect("the child's row is marked");
            },
        },
        &CancellationToken::new(),
    );
    assert_eq!(
        stopped.code,
        ResultCode::RemoteBranchDeleted,
        "{:?}",
        stopped.data
    );
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::OperationResume,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert!(reserved(&w), "the child stays");
    assert_eq!(state_of(&w, RESUME), None, "no holding record");
    Ok(())
}

/// A second call of a resume's ID, while the first holds the lease, leaves the
/// record under that call (R-27's rule, R-35).
fn resume_concurrent_same_id(verb: Verb) -> Result<(), FixtureError> {
    let (local, operation) = outside_local(verb, FailurePoint::AfterConfirmed)?;
    let dies = failing_service(&local_data(&local), FailurePoint::BeforeRequestDomainCall)?;
    let first = resumed_under(&dies, RESUME, &local.root, &operation);
    assert_eq!(first.code, ResultCode::InternalError);
    assert_eq!(local_state(&local, RESUME), Some(RequestState::Accepted));
    let held = fixed(RepositoryService::hold_lease_for_testing(
        &local.root,
        &local_data(&local),
        LeaseKind::Repository,
    ))?;
    let busy = resumed_under(&local.service, RESUME, &local.root, &operation);
    drop(held);
    assert_eq!(busy.code, ResultCode::Busy, "{:?}", busy.data);
    assert_eq!(
        local_state(&local, RESUME),
        Some(RequestState::Accepted),
        "not deleted under the other call"
    );
    let done = resumed_under(&local.service, RESUME, &local.root, &operation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(local_state(&local, RESUME), Some(RequestState::Finished));
    Ok(())
}
