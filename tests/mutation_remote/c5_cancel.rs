//! Cancellation of `document promote` and `ticket close` (Task 2c, decisions 8
//! and 12): a `c5_cancel_` case per row and verb.
//!
//! Each row stops a request where it says, by a failure point or from inside
//! the credentials a child asks for, since a lifecycle reports no progress.

use std::path::Path;

use super::*;
use lifecycle_world::*;
use results::{CleanupEffect, IntegrationEffect};

macro_rules! both {
    ($( $name:literal => $row:expr ),+ $(,)?) => {
        &[$(
            (concat!("c5_cancel_", $name, "_promote"), || $row(Verb::Promote)),
            (concat!("c5_cancel_", $name, "_close"), || $row(Verb::Close)),
        )+]
    };
}

const BOTH: &[ssh_harness::Case] = both![
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
    "killed_before_acknowledging" => killed_before_acknowledging,
    "killed_after_acknowledging" => killed_after_acknowledging,
    "complete_with_record_accepted" => complete_with_record_accepted,
    "child_flag_unowned" => child_flag_unowned,
    "child_push_unknown_stays" => child_push_unknown_stays,
    "publication_child_released" => publication_child_released,
    "walk_remote_branch_deleted" => walk_remote_branch_deleted,
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
    ("c5_cancel_child_flag_retained_close", child_flag_retained),
];

const fn joined<const N: usize>(
    first: &[ssh_harness::Case],
    second: &[ssh_harness::Case],
) -> [ssh_harness::Case; N] {
    let mut all = [first[0]; N];
    let mut at = 0;
    while at < first.len() {
        all[at] = first[at];
        at += 1;
    }
    while at < N {
        all[at] = second[at - first.len()];
        at += 1;
    }
    all
}

pub(crate) const CASES: &[ssh_harness::Case] = &joined::<{ BOTH.len() + ONE.len() }>(BOTH, ONE);

const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ1";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ2";
const REPUBLISH: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ3";

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

fn state_of(w: &World, id: &str) -> Option<RequestState> {
    record(w, id).map(|record| record.state)
}

fn lifecycle_of(envelope: &Envelope<MutationDataDto>) -> &LifecycleMutationDto {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Lifecycle(data)) => data,
        other => panic!("lifecycle data: {other:?}"),
    }
}

/// The commits on primary that merged a context: one, however many calls.
fn merges_on_primary(repo: &git2::Repository) -> Result<usize, FixtureError> {
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push(fixed(repo.refname_to_id(MAIN))?))?;
    fixed(walk.simplify_first_parent())?;
    let mut merges = 0;
    for oid in walk {
        if fixed(repo.find_commit(fixed(oid)?))?.parent_count() == 2 {
            merges += 1;
        }
    }
    Ok(merges)
}

fn parent_operation(w: &World, id: &str) -> Result<OperationId, FixtureError> {
    let found = record(w, id).ok_or(FixtureError)?;
    Ok(found.operations.first().ok_or(FixtureError)?.operation_id)
}

fn table(verb: Verb) -> &'static str {
    match verb {
        Verb::Promote => "document_promotions",
        Verb::Close => "ticket_closures",
    }
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
    let w = remote(verb)?;
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
    let w = remote(verb)?;
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
    let mut w = remote(verb)?;
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
    let mut w = remote(verb)?;
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
    let mut w = remote(verb)?;
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

/// A process killed before it acknowledged: the flag stands, so the resend
/// answers `cancelled` once more, and the one after continues (R-20).
fn killed_before_acknowledging(verb: Verb) -> Result<(), FixtureError> {
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
    let row = row_of(&w, child)?;
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert!(
        reserved(&w),
        "the push's fate is still the child's to learn"
    );
    assert_eq!(row_of(&w, child)?, row, "untouched");
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

/// A conflict the context's child retains has already ended interrupted, so
/// `request cancel` sets no flag on its row. The cancellation is answered once,
/// and the same request then finds the conflict as it left it.
fn child_flag_retained() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let w = remote(verb)?;
    fixed(w.sync(w.context_request()))?;
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    let peer = fixed(w.peer.workdir().ok_or(FixtureError).map(Path::to_owned))?;
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(&w.server));
    fixed(fixed(w.peer.find_remote("origin"))?.fetch(
        &[format!("+{0}:{0}", verb.branch())],
        Some(&mut fetch),
        None,
    ))?;
    commit_in(&peer, verb.branch(), &path, b"the peer's\n")?;
    push_peer(&w.peer, &w.server, verb.branch())?;
    let worktree = verb.worktree(&w.root);
    let source = fixed(std::fs::read_to_string(worktree.join(&path)))?;
    commit_in(
        &worktree,
        verb.branch(),
        &path,
        source.replacen("task", "bug", 1).as_bytes(),
    )?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let stopped = called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    asked(&w.service, FIRST)?;
    let interrupted = ("interrupted".to_owned(), false);
    assert_eq!(child_row(&w, verb, FIRST)?, interrupted);
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_eq!(child_row(&w, verb, FIRST)?, interrupted);
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

fn context_target(verb: Verb) -> Result<SynchronizationTarget, FixtureError> {
    Ok(SynchronizationTarget::Context {
        kind: match verb {
            Verb::Promote => AuthoringKind::Document,
            Verb::Close => AuthoringKind::Ticket,
        },
        item_id: fixed(verb.id().parse())?,
    })
}
