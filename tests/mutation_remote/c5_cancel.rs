//! Cancellation of `document promote` and `ticket close`: a `c5_cancel_` case
//! per row and verb.
//!
//! Each row stops a request where it says, by a failure point or from inside
//! the credentials a child asks for, since a lifecycle reports no progress.

use std::path::Path;

use super::*;
use lifecycle_world::*;
use results::{CleanupEffect, FailureClass, IntegrationEffect};

const BOTH: &[ssh_harness::Case] = both_verbs!["c5_cancel_";
    "stage_after_confirmed" => |verb| stage(verb, FailurePoint::AfterConfirmed, false),
    "stage_after_context_sync" => |verb| stage(verb, FailurePoint::AfterContextSync, false),
    "stage_before_integration_ref" => |verb| stage(verb, FailurePoint::BeforeIntegrationRef, false),
    "stage_after_integration_ref" => |verb| stage(verb, FailurePoint::AfterIntegrationRef, true),
    "stage_after_publication" => |verb| stage(verb, FailurePoint::AfterPublication, true),
    "stage_before_remote_deletion" => |verb| stage(verb, FailurePoint::BeforeRemoteDeletion, true),
    "stage_during_worktree_removal" => |verb| stage(verb, FailurePoint::DuringWorktreeRemoval, true),
    "stage_before_branch_deletion" => |verb| stage(verb, FailurePoint::BeforeBranchDeletion, true),
    "stage_before_completion" => |verb| stage(verb, FailurePoint::BeforeCompletion, true),
    "no_parent_yet" => no_parent_yet,
    "token_at_first_safe_point" => token_at_first_safe_point,
    "token_during_child" => token_during_child,
    "request_during_child" => request_during_child,
    "publication_pending_cancelled" => publication_pending_cancelled,
    "cleanup_pending_by_token" => cleanup_pending_by_token,
    "cleanup_pending_by_request" => cleanup_pending_by_request,
    "new_id_before_integration" => new_id_before_integration,
    "new_id_after_integration" => new_id_after_integration,
    "between_attempts" => between_attempts,
    "killed_before_the_domain_call" => killed_before_the_domain_call,
    "answered_without_taking_the_flag" => answered_without_taking_the_flag,
    "flag_unreadable_at_a_safe_point" => flag_unreadable_at_a_safe_point,
    "killed_after_acknowledging" => killed_after_acknowledging,
    "complete_with_record_accepted" => complete_with_record_accepted,
    "child_flag_unowned" => child_flag_unowned,
    "child_push_unknown_stays" => child_push_unknown_stays,
    "publication_child_released" => publication_child_released,
    "release_by_token" => release_by_token,
    "request_during_failed_child" => request_during_failed_child,
    "token_during_failed_child" => token_during_failed_child,
    "retained_conflict_is_kept" => retained_conflict_is_kept,
    "released_resolution_publishes" => released_resolution_publishes,
    "stays_released_resolution" => stays_released_resolution,
    "stays_reconciliation_required" => stays_reconciliation_required,
    "stays_recorded_push" => stays_recorded_push,
    "stays_past_its_fetch" => stays_past_its_fetch,
    "stays_local_prepared" => stays_local_prepared,
    "stays_applied_merge_at_fetch_observed" => stays_applied_merge_at_fetch_observed,
    "deletion_rejected_keeps_its_answer" => deletion_rejected_keeps_its_answer,
    "deletion_transport_keeps_its_answer" => deletion_transport_keeps_its_answer,
    "retained_child_remote_branch_deleted" => retained_child_remote_branch_deleted,
    "walk_remote_branch_deleted" => walk_remote_branch_deleted,
    "child_merged_then_refused_push" => child_merged_then_refused_push,
    "child_push_landed_then_cancelled" => child_push_landed_then_cancelled,
    "supersede_after_a_cancelled_answer" => supersede_after_a_cancelled_answer,
    "supersede_straight_after_the_cancel" => supersede_straight_after_the_cancel,
];

const ONE: &[ssh_harness::Case] = &[
    ("c5_cancel_stage_closing_recorded_close", || {
        stage(
            Verb::Close,
            FailurePoint::ClosureAfterClosingRecorded,
            false,
        )
    }),
    ("c5_cancel_stage_after_checkpoint_close", || {
        stage(Verb::Close, FailurePoint::AfterClosureCheckpoint, false)
    }),
    // The runner's rules for a fenced child are the same for either verb, and
    // these stagings are slow.
    ("c5_cancel_poll_leaves_a_retained_conflict_close", || {
        poll_leaves_a_retained_conflict(Verb::Close)
    }),
    (
        "c5_cancel_key_select_fences_a_retained_conflict_close",
        || key_select_fences_a_retained_conflict(Verb::Close),
    ),
    (
        "c5_cancel_poll_plan_failure_fences_a_retained_conflict_close",
        || poll_plan_failure_fences_a_retained_conflict(Verb::Close),
    ),
    ("c5_cancel_fenced_released_resolution_close", || {
        fenced_released_resolution(Verb::Close)
    }),
    ("c5_cancel_fenced_past_publication_close", || {
        fenced_past_publication(Verb::Close)
    }),
];

pub(crate) const CASES: &[ssh_harness::Case] =
    &joined_cases::<{ BOTH.len() + ONE.len() }>(BOTH, ONE);

const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ1";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ2";
const REPUBLISH: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ3";
const POLL: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ4";

fn confirmation_of(service: &RepositoryService, verb: Verb, root: &Path) -> String {
    let envelope = service.prepare(&verb.mutation(root, verb.id()));
    assert_eq!(envelope.code, ResultCode::Ok, "a preview");
    envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .expect("a preview")
        .confirmation_id
        .clone()
}

/// A provider that does `asks` when a child first needs the key: the one seam
/// in the middle of a child that a lifecycle gives from outside.
struct Asking<F: FnMut()> {
    asks: F,
}

impl<F: FnMut()> SessionCredentialProvider for Asking<F> {
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse {
        (self.asks)();
        Provider.request_passphrase(request)
    }
}

/// One call of `execute` with what a call is given, checked against the
/// published schemas.
fn call_with(
    service: &RepositoryService,
    id: &str,
    confirmation: Option<&str>,
    mutation: Mutation,
    provider: impl SessionCredentialProvider,
    cancel: &CancellationToken,
) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: confirmation.map(|id| ConfirmationId::parse(id).unwrap()),
            mutation,
        },
        &mut SessionCredentials::new(provider),
        &mut NoProgress,
        cancel,
    );
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    if !value["data"].is_null() {
        schema::check_published("lifecycle_mutation.schema.json", &value["data"])
            .expect("the lifecycle data schema");
    }
    envelope
}

fn called(
    service: &RepositoryService,
    id: &str,
    confirmation: Option<&str>,
    verb: Verb,
    root: &Path,
) -> Envelope<MutationDataDto> {
    call_with(
        service,
        id,
        confirmation,
        verb.mutation(root, verb.id()),
        Provider,
        &CancellationToken::new(),
    )
}

fn resent(w: &World, id: &str, verb: Verb) -> Envelope<MutationDataDto> {
    called(&w.service, id, None, verb, &w.root)
}

/// A preview, then the request stopped by `point`.
fn stopped_at(w: &mut World, verb: Verb, point: FailurePoint) -> Result<(), FixtureError> {
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        point,
    ))?;
    called(&failing, FIRST, Some(&confirmation), verb, &w.root);
    w.service = fixed(RepositoryService::open_at(&w.data()))?;
    assert_eq!(state_of(w, FIRST), Some(RequestState::Accepted));
    Ok(())
}

fn lifecycle_of(envelope: &Envelope<MutationDataDto>) -> &LifecycleMutationDto {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Lifecycle(data)) => data,
        other => panic!("lifecycle data: {other:?}"),
    }
}

fn parent_operation(w: &World, id: &str) -> Result<OperationId, FixtureError> {
    let found = record(w, id).ok_or(FixtureError)?;
    Ok(found.operations.first().ok_or(FixtureError)?.operation_id)
}

/// The parent's own flag, and the context child's row: its phase and flag.
fn child_row(w: &World, verb: Verb, id: &str) -> Result<(String, bool), FixtureError> {
    let operation = parent_operation(w, id)?;
    fixed(fixed(w.db())?.query_row(
        &format!(
            "SELECT o.phase, o.cancel_requested FROM {} p
             JOIN remote_operation_records o ON o.operation_ulid = p.context_sync_id
             WHERE p.operation_id = ?1",
            table(verb)
        ),
        [operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))
}

fn child_id(w: &World, verb: Verb, id: &str) -> Result<OperationId, FixtureError> {
    let operation = parent_operation(w, id)?;
    let child: String = fixed(fixed(w.db())?.query_row(
        &format!(
            "SELECT context_sync_id FROM {} WHERE operation_id = ?1",
            table(verb)
        ),
        [operation.to_string()],
        |row| row.get(0),
    ))?;
    fixed(OperationId::parse(&child))
}

/// A remote operation's phase and flag.
fn row_of(w: &World, operation: OperationId) -> Result<(String, bool), FixtureError> {
    fixed(fixed(w.db())?.query_row(
        "SELECT phase, cancel_requested FROM remote_operation_records WHERE operation_ulid = ?1",
        [operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))
}

/// Asks the request to stop, which leaves its record as it is, whatever its
/// operations' rows hold.
fn asked(service: &RepositoryService, id: &str) -> Result<(), FixtureError> {
    let answer = service.cancel_request(request(id));
    assert_eq!(answer.code, ResultCode::Ok);
    let data = answer.data.ok_or(FixtureError)?;
    assert!(data.requested, "asked");
    assert_eq!(data.state, RequestState::Accepted);
    assert_eq!(
        service
            .request_record(request(id))
            .unwrap()
            .map(|record| record.state),
        Some(RequestState::Accepted)
    );
    Ok(())
}

/// A `cancelled` answer: exit 130's outcome, the same request first, and the
/// request's flag acknowledged, so that the next call starts clean.
fn assert_cancelled(
    w: &World,
    id: &str,
    envelope: &Envelope<MutationDataDto>,
) -> Result<(), FixtureError> {
    assert_eq!(envelope.code, ResultCode::Cancelled, "{:?}", envelope.data);
    assert_eq!(envelope.outcome, Outcome::Cancelled);
    assert_eq!(actions(envelope), [RecoveryActionKind::RequestRetry]);
    let found = record(w, id).ok_or(FixtureError)?;
    assert_eq!(found.state, RequestState::Accepted);
    assert!(!found.cancel_requested, "the flag is acknowledged");
    Ok(())
}

fn assert_finishes(w: &World, id: &str, verb: Verb) -> Result<(), FixtureError> {
    let done = resent(w, id, verb);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.cleanup, CleanupEffect::Complete);
    assert_eq!(state_of(w, id), Some(RequestState::Finished));
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

// Each stage.

/// Stopped at `point`, with nothing running: asked, the next call answers
/// `cancelled` at its first safe point, and the one after finishes, with one
/// merge.
fn stage(verb: Verb, point: FailurePoint, integrated: bool) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, point)?;
    asked(&w.service, FIRST)?;
    let stopped = resent(&w, FIRST, verb);
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(
        stopped.effects.integration,
        if integrated {
            IntegrationEffect::Complete
        } else {
            IntegrationEffect::NotRequested
        }
    );
    assert_eq!(
        merges_on_primary(&w.repo()?)?,
        usize::from(integrated),
        "a cancellation adds no merge"
    );
    assert_finishes(&w, FIRST, verb)
}

/// Accepted with no parent yet: `requested`, nothing else. The request's flag
/// stops the run at its first safe point, by when the parent stands.
fn no_parent_yet(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::BeforeRequestDomainCall)?;
    assert_eq!(parent_count(&w, verb)?, 0);
    asked(&w.service, FIRST)?;
    assert_eq!(parent_count(&w, verb)?, 0);
    let stopped = resent(&w, FIRST, verb);
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(parent_count(&w, verb)?, 1);
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    assert_finishes(&w, FIRST, verb)
}

fn parent_count(w: &World, verb: Verb) -> Result<i64, FixtureError> {
    fixed(fixed(w.db())?.query_row(
        &format!("SELECT COUNT(*) FROM {}", table(verb)),
        [],
        |row| row.get(0),
    ))
}

// The call's token.

/// A token cancelled once the request is accepted stops it at the runner's
/// first safe point, before a connection.
fn token_at_first_safe_point(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    w.service
        .set_request_hook_for_testing(move || token.cancel());
    let before = connections(&w);
    let stopped = call_with(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
        Provider,
        &cancel,
    );
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(connections(&w), before, "no connection");
    assert_finishes(&w, FIRST, verb)
}

/// A token cancelled while a child runs stops it at the child's own safe
/// point: the probe reaches it.
fn token_during_child(verb: Verb) -> Result<(), FixtureError> {
    let w = protected(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let stopped = call_with(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
        Asking {
            asks: || token.cancel(),
        },
        &cancel,
    );
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    assert_eq!(child_row(&w, verb, FIRST)?.0, "cancelled");
    assert!(!reserved(&w), "the child let its reservation go");
    assert_finishes(&w, FIRST, verb)
}

/// `request cancel` from elsewhere while a child runs.
fn request_during_child(verb: Verb) -> Result<(), FixtureError> {
    let w = protected(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let stopped = call_with(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
        Asking {
            asks: || asked(&w.service, FIRST).expect("asked"),
        },
        &CancellationToken::new(),
    );
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    assert!(!reserved(&w), "the child let its reservation go");
    assert_finishes(&w, FIRST, verb)
}

// `Pending{Cancelled}`.

/// Stopped during the primary's child: integration stands, publication is
/// pending `cancelled`, and the same request publishes it.
fn publication_pending_cancelled(verb: Verb) -> Result<(), FixtureError> {
    let mut w = protected(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterIntegrationRef)?;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let stopped = call_with(
        &w.service,
        FIRST,
        None,
        verb.mutation(&w.root, verb.id()),
        Asking {
            asks: || token.cancel(),
        },
        &cancel,
    );
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(stopped.effects.integration, IntegrationEffect::Complete);
    assert_eq!(stopped.effects.publication, PublicationEffect::Pending);
    let data = lifecycle_of(&stopped);
    assert_eq!(
        data.publication_pending,
        Some(LifecyclePublicationPendingReason::Cancelled)
    );
    assert_eq!(data.cleanup, Some(LifecycleCleanupState::Pending));
    assert_finishes(&w, FIRST, verb)
}

/// A token cancelled while the remote branch is being deleted: the deletion
/// under way completes, the effects report it, and cleanup is pending.
fn cleanup_pending_by_token(verb: Verb) -> Result<(), FixtureError> {
    let mut w = protected(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let stopped = call_with(
        &w.service,
        FIRST,
        None,
        verb.mutation(&w.root, verb.id()),
        Asking {
            asks: || token.cancel(),
        },
        &cancel,
    );
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(stopped.effects.publication, PublicationEffect::Published);
    assert_eq!(stopped.effects.cleanup, CleanupEffect::Pending);
    assert_eq!(
        lifecycle_of(&stopped).cleanup_pending,
        Some(LifecycleCleanupPendingReason::Cancelled)
    );
    assert!(fixed(w.bare())?.find_reference(verb.branch()).is_err());
    assert_finishes(&w, FIRST, verb)
}

/// `request cancel` at the same place reaches the parent's own reservation,
/// which refuses the deletion. That refusal is the cancellation, and the next
/// call's restart does not read it as a stop of its own.
fn cleanup_pending_by_request(verb: Verb) -> Result<(), FixtureError> {
    let mut w = protected(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    let stopped = call_with(
        &w.service,
        FIRST,
        None,
        verb.mutation(&w.root, verb.id()),
        Asking {
            asks: || asked(&w.service, FIRST).expect("asked"),
        },
        &CancellationToken::new(),
    );
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(
        lifecycle_of(&stopped).cleanup_pending,
        Some(LifecycleCleanupPendingReason::Cancelled)
    );
    assert!(fixed(w.bare())?.find_reference(verb.branch()).is_ok());
    assert_finishes(&w, FIRST, verb)
}

// Cancelled, then the same request, then a new ID.

/// Before integration: the parent stands, so a new ID answers `busy`, and is
/// not the way on. The same request is.
fn new_id_before_integration(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    asked(&w.service, FIRST)?;
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_new_id_is_refused(&w, verb)?;
    assert_finishes(&w, FIRST, verb)?;
    // Once it has finished nothing is left for a new ID to do.
    let again = w.service.prepare(&verb.mutation(&w.root, verb.id()));
    assert_ne!(again.code, ResultCode::Ok, "{:?}", again.data);
    Ok(())
}

/// Past integration a new ID has no preview to confirm: until a poll has read
/// what the request published, the state is the parent's to finish. The same
/// request is the way on.
fn new_id_after_integration(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    asked(&w.service, FIRST)?;
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    let refused = w.service.prepare(&verb.mutation(&w.root, verb.id()));
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    // What refuses it is that the remote is ahead of what was last fetched.
    // `remote_context_deleted`, named beside it, reads a missing tracking ref
    // and isn't so: the branch stands, so its value isn't pinned.
    match refused.data.as_ref() {
        Some(PrepareDataDto::LifecycleBlocked(data)) => {
            assert!(
                data.blockers
                    .contains(&LifecycleBlocker::PrimaryAheadOfRemote)
            );
        }
        other => panic!("blockers: {other:?}"),
    }
    assert!(fixed(w.bare())?.find_reference(verb.branch()).is_ok());
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    assert_finishes(&w, FIRST, verb)?;
    let again = w.service.prepare(&verb.mutation(&w.root, verb.id()));
    assert_ne!(again.code, ResultCode::Ok, "{:?}", again.data);
    Ok(())
}

fn assert_new_id_is_refused(w: &World, verb: Verb) -> Result<(), FixtureError> {
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let refused = called(&w.service, SECOND, Some(&confirmation), verb, &w.root);
    assert_eq!(refused.code, ResultCode::Busy, "{:?}", refused.data);
    assert_eq!(
        actions(&refused),
        [RecoveryActionKind::RequestPrepare],
        "a new ID's own record is gone"
    );
    assert_eq!(state_of(w, SECOND), None);
    assert_eq!(state_of(w, FIRST), Some(RequestState::Accepted));
    Ok(())
}

// Automatic retries, and a killed process.

/// Decision 12: `request cancel` between two automatic attempts stops the
/// second at its first safe point, with no connection, and the next continues.
fn between_attempts(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let first = called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    w.server.clear_fault();
    assert_eq!(first.code, ResultCode::TransportUnavailable);
    assert_eq!(actions(&first), [RecoveryActionKind::RequestRetry]);
    asked(&w.service, FIRST)?;
    let before = connections(&w);
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_eq!(connections(&w), before, "attempt 2 connected to nothing");
    assert_finishes(&w, FIRST, verb)
}

/// A process killed before its domain call asked nothing of the flag: it
/// stands, so the resend answers `cancelled`, and the one after continues.
fn killed_before_the_domain_call(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    asked(&w.service, FIRST)?;
    let killed = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestDomainCall,
    ))?;
    called(&killed, FIRST, None, verb, &w.root);
    assert!(record(&w, FIRST).ok_or(FixtureError)?.cancel_requested);
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_finishes(&w, FIRST, verb)
}

/// A process killed after its answer was acknowledged and before it was
/// settled: nothing was asked since, so the resend continues.
fn killed_after_acknowledging(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    asked(&w.service, FIRST)?;
    let killed = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestSettlement,
    ))?;
    called(&killed, FIRST, None, verb, &w.root);
    assert!(!record(&w, FIRST).ok_or(FixtureError)?.cancel_requested);
    assert_finishes(&w, FIRST, verb)
}

/// Past the last safe point cancellation asks nothing: a complete parent keeps
/// its answer.
fn complete_with_record_accepted(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::BeforeRequestSettlement)?;
    let answer = w.service.cancel_request(request(FIRST));
    assert_eq!(answer.code, ResultCode::Ok);
    assert_finishes(&w, FIRST, verb)
}

// A pending child's flag, set while nothing runs.

/// A child stopped by a transport fault before any local or remote effect
/// holds its reservation and no resolution. `request cancel` sets its row's
/// flag with nothing running. The cancellation stops the parent, which retires
/// the child with no connection (decision 18): `restart` acknowledges the flag
/// rather than clearing it, so the row ends `cancelled`, flag and all, and the
/// next call runs a new attempt under a new ID.
fn child_flag_unowned(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    w.server.clear_fault();
    assert!(reserved(&w));
    let child = child_id(&w, verb, FIRST)?;
    asked(&w.service, FIRST)?;
    let (phase, flag) = row_of(&w, child)?;
    assert_ne!(phase, "cancelled");
    assert!(flag, "set with nothing running");
    let before = connections(&w);
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_eq!(connections(&w), before, "a retire connects to nothing");
    assert_eq!(row_of(&w, child)?, ("cancelled".to_owned(), true));
    assert!(!reserved(&w), "the cancellation let the child go");
    assert_finishes(&w, FIRST, verb)?;
    assert_ne!(child_id(&w, verb, FIRST)?, child);
    Ok(())
}

/// A child whose push may have landed is not proven safe to retire: the
/// `cancelled` answer leaves it holding the reservation, and the same request
/// finds out what became of the push and finishes, with no second merge.
fn child_push_unknown_stays(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    w.server.clear_fault();
    assert!(reserved(&w));
    let child = child_id(&w, verb, FIRST)?;
    asked(&w.service, FIRST)?;
    assert_kept(&w, verb, child)?;
    assert_finishes(&w, FIRST, verb)
}

/// A child's reservation of the primary's publication is let go the same way,
/// and the integration stands.
fn publication_child_released(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterIntegrationRef)?;
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = resent(&w, FIRST, verb);
    w.server.clear_fault();
    assert_eq!(stopped.code, ResultCode::TransportUnavailable);
    assert!(reserved(&w));
    asked(&w.service, FIRST)?;
    let cancelled = resent(&w, FIRST, verb);
    assert_cancelled(&w, FIRST, &cancelled)?;
    assert_eq!(cancelled.effects.integration, IntegrationEffect::Complete);
    assert!(!reserved(&w));
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    assert_finishes(&w, FIRST, verb)
}

// What a release leaves as it is.

/// The context's branch, with a local commit and a peer's on it.
fn diverged(w: &World, verb: Verb, path: &str, peer_bytes: &[u8]) -> Result<(), FixtureError> {
    fixed(w.sync(w.request(context_target(verb)?)))?;
    let peer = fixed(w.peer.workdir().ok_or(FixtureError).map(Path::to_owned))?;
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(&w.server));
    fixed(fixed(w.peer.find_remote("origin"))?.fetch(
        &[format!("+{0}:{0}", verb.branch())],
        Some(&mut fetch),
        None,
    ))?;
    commit_in(&peer, verb.branch(), path, peer_bytes)?;
    push_peer(&w.peer, &w.server, verb.branch())?;
    Ok(())
}

fn item_path(verb: Verb) -> String {
    match verb {
        Verb::Promote => DOCUMENT_PATH.to_owned(),
        Verb::Close => format!(".manyhands/tickets/{ITEM}/ticket.md"),
    }
}

/// A conflict the context's child stops on, retained.
fn conflicted(w: &World, verb: Verb) -> Result<(), FixtureError> {
    let path = item_path(verb);
    let worktree = verb.worktree(&w.root);
    let source = fixed(std::fs::read_to_string(worktree.join(&path)))?;
    diverged(
        w,
        verb,
        &path,
        format!("{source}\nthe peer's edit\n").as_bytes(),
    )?;
    commit_in(
        &worktree,
        verb.branch(),
        &path,
        format!("{source}\nthe local edit\n").as_bytes(),
    )?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let stopped = called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    Ok(())
}

/// A retained conflict has already ended `interrupted`, so `request cancel`
/// sets no flag on its row. The cancellation is answered once and removes
/// nothing: the same request finds the conflict as it left it.
fn retained_conflict_is_kept(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    asked(&w.service, FIRST)?;
    let interrupted = ("interrupted".to_owned(), false);
    assert_eq!(child_row(&w, verb, FIRST)?, interrupted);
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_eq!(child_row(&w, verb, FIRST)?, interrupted);
    let index = fixed(fixed(git2::Repository::open(verb.worktree(&w.root)))?.index())?;
    assert!(index.has_conflicts(), "the conflict stands");
    let next = resent(&w, FIRST, verb);
    assert_eq!(next.code, ResultCode::RecoveryRequired);
    assert_eq!(
        actions(&next),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(child_row(&w, verb, FIRST)?, interrupted);
    Ok(())
}

/// A conflict resolved by the user, published by nothing yet, with the child
/// stopped by a transport fault as the same request goes on. It returns the
/// child and the resolution's commit.
fn resolved_then_stopped(w: &World, verb: Verb) -> Result<(OperationId, git2::Oid), FixtureError> {
    conflicted(w, verb)?;
    let child = child_id(w, verb, FIRST)?;
    let inspection = fixed(w.service.inspect_synchronization_recovery(&w.root, child))?;
    let mut results = Vec::new();
    for path in &inspection.paths {
        let sides = fixed(w.service.read_synchronization_conflict(&path.token))?;
        results.push((path.token.clone(), sides.local.ok_or(FixtureError)?));
    }
    let resolved = match fixed(w.service.resolve_synchronization(
        ResolveSynchronizationRequest::new(
            w.root.clone(),
            child,
            OperationId::new(),
            inspection.observation,
            results,
            None,
        ),
    ))? {
        ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } => commit_oid,
        _ => return Err(FixtureError),
    };
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = resent(w, FIRST, verb);
    w.server.clear_fault();
    assert_eq!(stopped.code, ResultCode::TransportUnavailable);
    assert!(reserved(w));
    Ok((child, resolved))
}

/// As it really stands, a resolved child owes a reconciliation too. Asked, it
/// stays, and the same request publishes what the user resolved.
fn released_resolution_publishes(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let (_, resolved) = resolved_then_stopped(&w, verb)?;
    asked(&w.service, FIRST)?;
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert!(reserved(&w));
    assert_finishes(&w, FIRST, verb)?;
    let bare = fixed(w.bare())?;
    let served = fixed(bare.refname_to_id(MAIN))?;
    assert!(fixed(bare.graph_descendant_of(served, resolved))?);
    Ok(())
}

/// The serious guard, alone: a released resolution in an unfinished phase, with
/// no reconciliation owed, which is put there since it is what would otherwise
/// be retired. A retire would drop the publication of what the user resolved.
fn stays_released_resolution(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let (child, _) = resolved_then_stopped(&w, verb)?;
    fixed(fixed(w.db())?.execute(
        "UPDATE remote_operation_records
            SET reconciliation_required = 0, phase = sync_checkpoint
          WHERE operation_ulid = ?1",
        [child.to_string()],
    ))?;
    asked(&w.service, FIRST)?;
    assert_eq!(
        row_of(&w, child)?.0,
        "fetch_observed",
        "its checkpoint allows a release"
    );
    assert_kept(&w, verb, child)
}

/// A transport-stopped child, whose row a guard's row then changes by one
/// column; asked, it is kept as it is, and the reservation with it.
fn stays_with(verb: Verb, column: &str, value: &str) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    w.server.clear_fault();
    let child = child_id(&w, verb, FIRST)?;
    fixed(fixed(w.db())?.execute(
        &format!("UPDATE remote_operation_records SET {column} = ?2 WHERE operation_ulid = ?1"),
        [child.to_string(), value.to_owned()],
    ))?;
    asked(&w.service, FIRST)?;
    assert_kept(&w, verb, child)
}

/// Asked, it's answered `cancelled` and kept, phase and reservation: only the
/// flag goes, since the cancellation has been answered.
fn assert_kept(w: &World, verb: Verb, child: OperationId) -> Result<(), FixtureError> {
    let (phase, flag) = row_of(w, child)?;
    assert!(flag, "asked of its row");
    assert_cancelled(w, FIRST, &resent(w, FIRST, verb))?;
    assert_eq!(row_of(w, child)?, (phase, false), "kept");
    assert!(reserved(w));
    Ok(())
}

fn stays_reconciliation_required(verb: Verb) -> Result<(), FixtureError> {
    stays_with(verb, "reconciliation_required", "1")
}

/// A recorded push, at a checkpoint that alone would allow the release.
fn stays_recorded_push(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let tip = fixed(w.repo()?.refname_to_id(verb.branch()))?;
    drop(w);
    stays_with(verb, "push_oid", &tip.to_string())
}

/// A local update prepared, whose ref may or may not have moved, alone: the
/// row has no recorded push, no integration and nothing else to keep it.
fn stays_local_prepared(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    w.server.clear_fault();
    let child = child_id(&w, verb, FIRST)?;
    let tip = fixed(w.repo()?.refname_to_id(verb.branch()))?.to_string();
    fixed(fixed(w.db())?.execute(
        "UPDATE remote_operation_records
            SET phase = 'local_prepared', sync_checkpoint = 'local_prepared',
                expected_oid = ?2, local_oid = ?2, primary_tracking_oid = ?2
          WHERE operation_ulid = ?1",
        [child.to_string(), tip],
    ))?;
    asked(&w.service, FIRST)?;
    assert_eq!(row_of(&w, child)?.0, "local_prepared");
    assert_kept(&w, verb, child)
}

/// Runs `at` once, while `run` is held at the first advertisement of a call
/// that finds a row in `phase`: there is no safe point from there to whatever
/// that connection is for.
fn at_advertisement_in_phase<T>(
    w: &World,
    phase: &'static str,
    at: impl FnOnce() + Send,
    run: impl FnOnce() -> T,
) -> Result<T, FixtureError> {
    let server = &w.server;
    let registry = w.data().join("manyhands.sqlite3");
    let first = server.hold_advertisement()?;
    std::thread::scope(|scope| {
        let holder = scope.spawn(move || -> Result<(), FixtureError> {
            let mut held = first;
            loop {
                held.wait_until_held()?;
                let found: bool = fixed(fixed(rusqlite::Connection::open(&registry))?.query_row(
                    "SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE phase = ?1)",
                    [phase],
                    |row| row.get(0),
                ))?;
                if found {
                    at();
                    return held.release();
                }
                let next = server.hold_advertisement()?;
                held.release()?;
                held = next;
            }
        });
        let ran = run();
        holder.join().map_err(|_| FixtureError)??;
        Ok(ran)
    })
}

/// A peer's commit and a local one on the branch, which a child will merge
/// cleanly. It returns the confirmation of a preview taken before that.
fn merges_cleanly(w: &World, verb: Verb) -> Result<String, FixtureError> {
    let path = item_path(verb);
    let worktree = verb.worktree(&w.root);
    let source = fixed(std::fs::read_to_string(worktree.join(&path)))?;
    diverged(
        w,
        verb,
        &path,
        format!("{source}\nthe peer's edit\n").as_bytes(),
    )?;
    commit_in(
        &worktree,
        verb.branch(),
        &path,
        source.replacen(TITLE, "the local title", 1).as_bytes(),
    )?;
    Ok(confirmation_of(&w.service, verb, &w.root))
}

/// A clean merge of a peer's commit with a local one, on the branch, with the
/// child stopped by a transport fault at the push endpoint.
fn merged_then_stopped(w: &World, verb: Verb) -> Result<OperationId, FixtureError> {
    let confirmation = merges_cleanly(w, verb)?;
    let server = &w.server;
    let stopped = at_advertisement_in_phase(
        w,
        "local_fast_forwarded",
        move || server.disconnect_at(FixtureBoundary::Advertisement),
        || called(&w.service, FIRST, Some(&confirmation), verb, &w.root),
    )?;
    w.server.clear_fault();
    assert_eq!(
        stopped.code,
        ResultCode::TransportUnavailable,
        "{:?}",
        stopped.data
    );
    child_id(w, verb, FIRST)
}

/// A child past its fetch: a local update has been made, whose branch the
/// same request must carry on from. Kept.
fn stays_past_its_fetch(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let child = merged_then_stopped(&w, verb)?;
    asked(&w.service, FIRST)?;
    assert_eq!(row_of(&w, child)?.0, "local_fast_forwarded");
    assert_kept(&w, verb, child)
}

/// A merge is applied and its checkpoint isn't written, which is a kill between
/// the two, and is not otherwise reachable: the checkpoint is put back. The
/// branch holds the merge, so the child isn't one with nothing to find out.
fn stays_applied_merge_at_fetch_observed(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let child = merged_then_stopped(&w, verb)?;
    fixed(fixed(w.db())?.execute(
        "UPDATE remote_operation_records
            SET phase = 'fetch_observed', sync_checkpoint = 'fetch_observed'
          WHERE operation_ulid = ?1",
        [child.to_string()],
    ))?;
    asked(&w.service, FIRST)?;
    assert_eq!(row_of(&w, child)?.0, "fetch_observed");
    assert_kept(&w, verb, child)
}

/// A release the call's own token reaches, with no `request cancel`.
fn release_by_token(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    w.server.clear_fault();
    assert!(reserved(&w));
    let cancel = CancellationToken::new();
    cancel.cancel();
    let before = connections(&w);
    let stopped = call_with(
        &w.service,
        FIRST,
        None,
        verb.mutation(&w.root, verb.id()),
        Provider,
        &cancel,
    );
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(connections(&w), before);
    assert!(!reserved(&w));
    assert_finishes(&w, FIRST, verb)
}

/// `request cancel` lands during a child that then ends `remote_branch_deleted`
/// with no safe point between: the child failed, the parent stops `cancelled`,
/// and that lets the child go, with no second cancel.
fn request_during_failed_child(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    fixed(w.sync(w.request(context_target(verb)?)))?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    fixed(fixed(w.bare()?.find_reference(verb.branch()))?.delete())?;
    let elsewhere = fixed(RepositoryService::open_at(&w.data()))?;
    let stopped = at_advertisement_in_phase(
        &w,
        "fetch_observed",
        move || asked(&elsewhere, FIRST).expect("asked"),
        || called(&w.service, FIRST, Some(&confirmation), verb, &w.root),
    )?;
    assert_cancelled(&w, FIRST, &stopped)?;
    assert!(!reserved(&w), "the cancellation let the child go");
    republished(&w, verb)?;
    assert_finishes(&w, FIRST, verb)
}

/// The same with the call's token, which no boundary of the child reads: the
/// child ends `remote_branch_deleted`, and only the parent then stops.
fn token_during_failed_child(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    fixed(w.sync(w.request(context_target(verb)?)))?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    fixed(fixed(w.bare()?.find_reference(verb.branch()))?.delete())?;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let stopped = at_advertisement_in_phase(
        &w,
        "fetch_observed",
        move || token.cancel(),
        || {
            call_with(
                &w.service,
                FIRST,
                Some(&confirmation),
                verb.mutation(&w.root, verb.id()),
                Provider,
                &cancel,
            )
        },
    )?;
    assert_cancelled(&w, FIRST, &stopped)?;
    assert!(!reserved(&w), "the cancellation let the child go");
    assert_eq!(child_row(&w, verb, FIRST)?.0, "cancelled");
    republished(&w, verb)?;
    assert_finishes(&w, FIRST, verb)
}

/// Item republish, previewed and confirmed: `ok`.
fn republished(w: &World, verb: Verb) -> Result<(), FixtureError> {
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
    let done = w.service.execute(
        MutationCall {
            request_id: request(REPUBLISH),
            confirmation: Some(ConfirmationId::parse(&consent).unwrap()),
            mutation: republish,
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert!(fixed(w.bare())?.find_reference(verb.branch()).is_ok());
    Ok(())
}

/// A refusal of the deletion itself, with cancellation asked, keeps its own
/// answer: it isn't the cancellation that stopped it.
fn deletion_rejected_keeps_its_answer(verb: Verb) -> Result<(), FixtureError> {
    let mut w = protected(verb)?;
    fixed(w.sync(w.request(context_target(verb)?)))?;
    w.server.reject_context_deletion(true)?;
    fixed(fixed(w.bare())?.set_head(verb.branch()))?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let stopped = call_with(
        &w.service,
        FIRST,
        None,
        verb.mutation(&w.root, verb.id()),
        Asking {
            asks: || token.cancel(),
        },
        &cancel,
    );
    assert_eq!(stopped.code, ResultCode::PushRejected, "{:?}", stopped.data);
    assert_eq!(
        lifecycle_of(&stopped).cleanup_pending,
        Some(LifecycleCleanupPendingReason::RemoteDeletionRejected)
    );
    Ok(())
}

/// So does a transport failure of the deletion: what became of it is not known,
/// and the cancellation isn't what stopped it.
fn deletion_transport_keeps_its_answer(verb: Verb) -> Result<(), FixtureError> {
    let mut w = protected(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let stopped = call_with(
        &w.service,
        FIRST,
        None,
        verb.mutation(&w.root, verb.id()),
        Asking {
            asks: || token.cancel(),
        },
        &cancel,
    );
    w.server.clear_fault();
    assert_eq!(
        stopped.code,
        ResultCode::TransportUnavailable,
        "{:?}",
        stopped.data
    );
    assert_eq!(
        lifecycle_of(&stopped).cleanup_pending,
        Some(LifecycleCleanupPendingReason::RemoteDeletionTransport)
    );
    assert_finishes(&w, FIRST, verb)
}

/// A retained conflict whose branch is then deleted at the remote never answers
/// `remote_branch_deleted`: the conflict is what it answers, with no reservation
/// and no `request.cancel` offered that would not release it.
fn retained_child_remote_branch_deleted(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    fixed(fixed(w.bare()?.find_reference(verb.branch()))?.delete())?;
    let stopped = resent(&w, FIRST, verb);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert!(!reserved(&w));
    Ok(())
}

// A flag that was not taken, or can't be read.

/// The real window of a kill: a run that answered `cancelled` and didn't take
/// the flag, which its read's failure stands in for. The resend answers
/// `cancelled` once more, and the one after continues (R-20).
fn answered_without_taking_the_flag(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    asked(&w.service, FIRST)?;
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestCancellationRead,
    ))?;
    // The token answers each probe, so the one read is the taking.
    let cancel = CancellationToken::new();
    cancel.cancel();
    let answered = call_with(
        &failing,
        FIRST,
        None,
        verb.mutation(&w.root, verb.id()),
        Provider,
        &cancel,
    );
    assert_eq!(answered.code, ResultCode::Cancelled, "{:?}", answered.data);
    assert!(record(&w, FIRST).ok_or(FixtureError)?.cancel_requested);
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_finishes(&w, FIRST, verb)
}

/// A flag that can't be read at a safe point isn't taken for "no": the run stops
/// there, its record kept, with nothing done past it. The observer's half is a
/// unit row, `an_unreadable_flag_stops_a_child`, since a safe point always
/// reads first here and the failure point fires once.
fn flag_unreadable_at_a_safe_point(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeRequestCancellationRead,
    ))?;
    let stopped = called(&failing, FIRST, None, verb, &w.root);
    assert_eq!(
        stopped.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        stopped.data
    );
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    assert_finishes(&w, FIRST, verb)
}

// R-24.

/// A child's `remote_branch_deleted`, walked to a finish (decision 18). The
/// cancellation stops the parent, which retires the child and frees the
/// reservation, so `item republish` runs, and the same request finishes.
fn walk_remote_branch_deleted(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let context = context_target(verb)?;
    fixed(w.sync(w.request(context)))?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    fixed(fixed(w.bare()?.find_reference(verb.branch()))?.delete())?;
    let stopped = called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    assert_eq!(stopped.code, ResultCode::RemoteBranchDeleted);
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestCancel,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert!(reserved(&w));
    asked(&w.service, FIRST)?;
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert!(!reserved(&w), "the cancellation let the child go");

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
    assert!(fixed(w.bare())?.find_reference(verb.branch()).is_ok());
    assert_finishes(&w, FIRST, verb)
}

// What a child did before the parent stopped (decision 25).

/// The child merges on the context's branch and its push is refused. The merge
/// stays, whatever stopped the request, so the answer reports it and exits 4,
/// with nothing on primary, and so does a call that only finds the record.
fn child_merged_then_refused_push(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = merges_cleanly(&w, verb)?;
    // The served branch is what the receiver refuses to update.
    fixed(fixed(w.bare())?.set_head(verb.branch()))?;
    w.server.reject_primary_updates(true)?;
    let stopped = called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    assert_eq!(stopped.code, ResultCode::PushRejected, "{:?}", stopped.data);
    assert_eq!(stopped.outcome, Outcome::Partial, "{:?}", stopped.effects);
    assert_eq!(stopped.failure_class(), Some(FailureClass::Incomplete));
    let repo = w.repo()?;
    let merge = fixed(repo.refname_to_id(verb.branch()))?;
    assert_eq!(fixed(repo.find_commit(merge))?.parent_count(), 2);
    assert_eq!(stopped.effects.integration, IntegrationEffect::Complete);
    assert_eq!(stopped.effects.commit_oid, Some(merge.to_string()));
    assert_eq!(stopped.effects.publication, PublicationEffect::NotRequested);
    assert_eq!(merges_on_primary(&repo)?, 0, "nothing reached primary");
    // Another request under its ID finds the record, and is shown what it did.
    let mismatch = call_with(
        &w.service,
        FIRST,
        None,
        verb.mutation(&w.root, "01ARZ3NDEKTSV4RRFFQ69G5FJ9"),
        Provider,
        &CancellationToken::new(),
    );
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.outcome, Outcome::Partial);
    assert_eq!(mismatch.effects, stopped.effects);
    Ok(())
}

/// The child's push lands and the parent is then cancelled at its next safe
/// point: the push is reported, and the answer stays `cancelled`, exit 130,
/// which the RFC gives a cancellation even after a durable effect.
fn child_push_landed_then_cancelled(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    let landed = fixed(w.bare()?.refname_to_id(verb.branch()))?;
    assert_eq!(landed, fixed(w.repo()?.refname_to_id(verb.branch()))?);
    asked(&w.service, FIRST)?;
    let stopped = resent(&w, FIRST, verb);
    assert_cancelled(&w, FIRST, &stopped)?;
    assert_eq!(stopped.effects.publication, PublicationEffect::Published);
    assert_eq!(stopped.effects.commit_oid, Some(landed.to_string()));
    assert_eq!(stopped.effects.integration, IntegrationEffect::NotRequested);
    assert_eq!(stopped.failure_class(), Some(FailureClass::Cancelled));
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    assert_finishes(&w, FIRST, verb)
}

// A cancellation beside a supersedable stop.

/// Stopped in cleanup on a file the owner left in the worktree, with the request
/// accepted. It returns that file.
fn stopped_on_context_changed(
    w: &mut World,
    verb: Verb,
) -> Result<std::path::PathBuf, FixtureError> {
    stopped_at(w, verb, FailurePoint::AfterPublication)?;
    let untracked = verb.worktree(&w.root).join("untracked.txt");
    fixed(std::fs::write(&untracked, b"the owner's\n"))?;
    assert_context_changed(&resent(w, FIRST, verb));
    Ok(untracked)
}

fn assert_context_changed(answer: &Envelope<MutationDataDto>) {
    assert_eq!(
        answer.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        answer.data
    );
    assert_eq!(
        lifecycle_of(answer).cleanup_pending,
        Some(LifecycleCleanupPendingReason::ContextChanged)
    );
}

/// The file gone and what was published observed, so that a preview is ready,
/// a new ID answers.
fn new_id_answer(
    w: &World,
    verb: Verb,
    untracked: &Path,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    fixed(std::fs::remove_file(untracked))?;
    fixed(w.sync(w.primary()))?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    Ok(called(
        &w.service,
        SECOND,
        Some(&confirmation),
        verb,
        &w.root,
    ))
}

/// A `cancelled` answer overwrites the supersedable reason, so a new ID is `busy`
/// until one more resend of the same request finds the reason again.
fn supersede_after_a_cancelled_answer(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let untracked = stopped_on_context_changed(&mut w, verb)?;
    asked(&w.service, FIRST)?;
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    let refused = new_id_answer(&w, verb, &untracked)?;
    assert_eq!(refused.code, ResultCode::Busy, "{:?}", refused.data);
    assert_eq!(state_of(&w, SECOND), None);
    fixed(std::fs::write(&untracked, b"the owner's\n"))?;
    assert_context_changed(&resent(&w, FIRST, verb));
    let replaced = new_id_answer(&w, verb, &untracked)?;
    assert_eq!(replaced.code, ResultCode::Ok, "{:?}", replaced.data);
    assert_eq!(resent(&w, FIRST, verb).code, ResultCode::RequestMismatch);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// `request cancel` sets flags and leaves the reason, so a new ID supersedes the
/// parent at once, and the old request answers from that.
fn supersede_straight_after_the_cancel(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let untracked = stopped_on_context_changed(&mut w, verb)?;
    asked(&w.service, FIRST)?;
    let replaced = new_id_answer(&w, verb, &untracked)?;
    assert_eq!(replaced.code, ResultCode::Ok, "{:?}", replaced.data);
    assert_eq!(resent(&w, FIRST, verb).code, ResultCode::RequestMismatch);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

// A poll's fence over a child that can't be replaced (design, Known Limits).

/// The generation that fences a remote operation.
fn generation(w: &World) -> Result<i64, FixtureError> {
    fixed(fixed(w.db())?.query_row(
        "SELECT configuration_generation FROM remote_polling_state",
        [],
        |row| row.get(0),
    ))
}

/// A poll's answer under a changed push URL, with the URL then put back: only
/// the generation is left to answer for.
fn polled_under_a_changed_url(w: &World) -> Result<ResultCode, FixtureError> {
    let url = w.server.url();
    let repo = w.repo()?;
    fixed(repo.remote_set_pushurl("origin", Some(&format!("{url}-changed"))))?;
    let poll = w.service.execute(
        MutationCall {
            request_id: request(POLL),
            confirmation: None,
            mutation: Mutation::PollOnce(PollOnceInput {
                root: w.root.clone(),
            }),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    fixed(repo.remote_set_pushurl("origin", None))?;
    Ok(poll.code)
}

/// A poll that raises the generation.
fn fence(w: &World) -> Result<(), FixtureError> {
    let before = generation(w)?;
    assert_eq!(polled_under_a_changed_url(w)?, ResultCode::Ok);
    assert!(generation(w)? > before, "fenced");
    Ok(())
}

fn assert_stuck(w: &World, verb: Verb) -> Result<(), FixtureError> {
    let answer = resent(w, FIRST, verb);
    assert_eq!(
        answer.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        answer.data
    );
    assert_eq!(state_of(w, FIRST), Some(RequestState::Accepted));
    Ok(())
}

/// A poll doesn't fence a retained `conflict_pending` child through an endpoint
/// change: it refuses on the pending conflict before it configures its
/// endpoints, so the generation stays, and the conflict stands as it was. A plan
/// failure is the exception, below.
fn poll_leaves_a_retained_conflict(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let before = generation(&w)?;
    assert_eq!(
        polled_under_a_changed_url(&w)?,
        ResultCode::RecoveryRequired
    );
    assert_eq!(generation(&w)?, before, "not fenced");
    assert_stuck(&w, verb)
}

/// A retained conflict once a generation has been raised over it: the resend
/// stays `recovery_required`, with its record, however often it is sent, and a
/// new prepare is refused the same way, so no new ID has a confirmation to run
/// with. These answers are also a retained conflict's unfenced, so what the
/// fence costs is the way out, which Task 13b's walk owes.
fn assert_stuck_under_a_new_generation(w: &World, verb: Verb) -> Result<(), FixtureError> {
    assert_stuck(w, verb)?;
    assert_stuck(w, verb)?;
    let preview = w.service.prepare(&verb.mutation(&w.root, verb.id()));
    assert_eq!(preview.code, ResultCode::RecoveryRequired);
    assert!(
        preview
            .data
            .as_ref()
            .and_then(PrepareDataDto::preview)
            .is_none()
    );
    assert_stuck(w, verb)
}

/// `key select` fences every remote: it raises the generation with no look at
/// what a retained child holds.
fn key_select_fences_a_retained_conflict(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let before = generation(&w)?;
    let key = fixed(w.service.list_shared_keys())?
        .into_iter()
        .next()
        .ok_or(FixtureError)?;
    fixed(w.service.clear_shared_key_selection())?;
    fixed(w.service.select_shared_key(key.id))?;
    assert!(generation(&w)? > before, "fenced");
    assert_stuck_under_a_new_generation(&w, verb)
}

/// A plan failure configures with no plan before any look at a pending conflict
/// (`remote/poll.rs`). `poll once` doesn't reach it: its binding refuses an
/// invalid configuration first, and the generation stays. The domain's own entry
/// point, which an automatic poll takes, does, and is fenced.
fn poll_plan_failure_fences_a_retained_conflict(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let before = generation(&w)?;
    let path = w.root.join(".manyhands/config.toml");
    let configuration = fixed(std::fs::read(&path))?;
    fixed(std::fs::write(&path, b"not = [valid\n"))?;
    let verb_answer = w.service.execute(
        MutationCall {
            request_id: request(POLL),
            confirmation: None,
            mutation: Mutation::PollOnce(PollOnceInput {
                root: w.root.clone(),
            }),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(verb_answer.code, ResultCode::InvalidConfiguration);
    assert_eq!(generation(&w)?, before, "the verb doesn't reach the domain");
    let domain = w.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: w.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: None,
            restart: false,
        },
        &mut SessionCredentials::new(Provider),
    );
    fixed(std::fs::write(&path, configuration))?;
    assert!(domain.is_err());
    assert!(generation(&w)? > before, "fenced");
    assert_stuck_under_a_new_generation(&w, verb)
}

/// A released resolution is restarted, not replaced, and a restart under another
/// generation is refused. Unfenced, the same request publishes it
/// (`released_resolution_publishes`).
fn fenced_released_resolution(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    resolved_then_stopped(&w, verb)?;
    fence(&w)?;
    assert_stuck(&w, verb)
}

/// Past `published`, the cleanup's owner requires that the primary child is of
/// the generation that holds now.
fn fenced_past_publication(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    fence(&w)?;
    assert_stuck(&w, verb)
}
