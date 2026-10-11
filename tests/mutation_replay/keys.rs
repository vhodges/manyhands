//! The shared-key commands: requests of the application, which name no
//! repository. Every effect is `not_requested`, `data` names the key, and
//! the outcome says whether anything changed.
//!
//! `key select`, `key clear` and `key remove` change the key registry in
//! one step with no operation: a request of one names none, is finished
//! when its end state holds, and leaves nothing when it is refused.

use manyhands::{
    repository::{
        FailurePoint, KeyClearInput, KeyMutationDto, KeyRemoveInput, KeySelectInput, Mutation,
        MutationDataDto, RequestState, SharedKeyId, request_store::ScopeKey,
    },
    results::{Effects, Envelope, FailureClass, Outcome, ResultCode, Scope},
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{
        KeyWorld, REQUEST_1, REQUEST_2, REQUEST_3, World, execute_cancellable, execute_confirmed,
        execute_with, key_data, recovery_actions,
    },
};

fn select(id: SharedKeyId) -> Mutation {
    Mutation::KeySelect(KeySelectInput { id })
}

fn clear() -> Mutation {
    Mutation::KeyClear(KeyClearInput {})
}

fn remove(id: SharedKeyId) -> Mutation {
    Mutation::KeyRemove(KeyRemoveInput { id })
}

/// What every result of a key command holds, whatever its outcome: no
/// effect, a scope that names nothing, and no operation for a change to
/// the registry.
fn assert_of_the_application(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.scope, Scope::default());
    assert_eq!(envelope.operation_id, None);
}

fn assert_success(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_of_the_application(envelope);
    assert!(envelope.recovery.is_empty());
}

fn assert_noop(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_of_the_application(envelope);
    assert!(envelope.recovery.is_empty());
}

/// What a request reports of the key `id` when it read its registration.
fn read(id: SharedKeyId, label: &str) -> KeyMutationDto {
    KeyMutationDto {
        id: Some(id.to_string()),
        label: Some(label.to_owned()),
        // The fixture registers a key with no public key file.
        fingerprint: None,
        generation: None,
    }
}

/// What a request reports of the key `id` when it read nothing of it.
fn named(id: Option<SharedKeyId>) -> KeyMutationDto {
    KeyMutationDto {
        id: id.map(|id| id.to_string()),
        label: None,
        fingerprint: None,
        generation: None,
    }
}

/// Asserts that the request left a finished record of the application
/// that holds no operation.
fn assert_finished(world: &KeyWorld, request_id: &str, command: &str, target: &str) {
    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.scope, ScopeKey::application());
    assert_eq!(record.scope.as_str(), "application");
    assert_eq!(record.command, command);
    assert_eq!(record.target, target);
    assert!(record.operations.is_empty(), "{:?}", record.operations);
    assert_eq!((record.base_ref, record.base_oid), (None, None));
    assert_eq!(record.expected_digest, None);
}

#[test]
fn a_key_is_selected_and_selecting_it_again_is_a_no_op() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");

    let selected = world.execute(REQUEST_1, select(work));
    assert_success(&selected);
    assert_eq!(key_data(&selected), &read(work, "Work laptop"));
    assert_eq!(world.selected(), Some(work));
    assert_finished(&world, REQUEST_1, "key select", &work.to_string());

    // Replay: the stored result, whatever is selected now.
    world.service.select_shared_key(other).unwrap();
    assert_eq!(world.execute(REQUEST_1, select(work)), selected);
    assert_eq!(world.selected(), Some(other));

    // A new request for the key that is selected changes nothing.
    let again = world.execute(REQUEST_2, select(other));
    assert_noop(&again);
    assert_eq!(key_data(&again), &read(other, "Other"));
    assert_eq!(world.selected(), Some(other));
    assert_finished(&world, REQUEST_2, "key select", &other.to_string());

    // Another key, selected in its place.
    let replaced = world.execute(REQUEST_3, select(work));
    assert_success(&replaced);
    assert_eq!(world.selected(), Some(work));
}

#[test]
fn selecting_a_key_that_is_not_registered_is_key_not_found_and_leaves_nothing() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let unknown = SharedKeyId::new();

    let refused = world.execute(REQUEST_1, select(unknown));
    assert_eq!(
        (refused.outcome, refused.code, refused.failure_class()),
        (
            Outcome::Error,
            ResultCode::KeyNotFound,
            Some(FailureClass::Input)
        )
    );
    assert_of_the_application(&refused);
    assert_eq!(key_data(&refused), &named(Some(unknown)));
    assert!(refused.recovery.is_empty());
    assert_eq!(world.selected(), None);
    assert_eq!(world.request_rows(), 0);

    // The same request ID, with the key corrected, runs at once.
    assert_success(&world.execute(REQUEST_1, select(work)));
}

#[test]
fn the_selection_is_cleared_and_clearing_it_again_is_a_no_op() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    world.service.select_shared_key(work).unwrap();

    let cleared = world.execute(REQUEST_1, clear());
    assert_success(&cleared);
    assert_eq!(key_data(&cleared), &named(None));
    assert_eq!(world.selected(), None);
    assert_finished(&world, REQUEST_1, "key clear", "");

    // Replay: the stored result, although a key is selected again.
    world.service.select_shared_key(work).unwrap();
    assert_eq!(world.execute(REQUEST_1, clear()), cleared);
    assert_eq!(world.selected(), Some(work));
    world.service.clear_shared_key_selection().unwrap();

    let again = world.execute(REQUEST_2, clear());
    assert_noop(&again);
    assert_eq!(key_data(&again), &named(None));
    assert_finished(&world, REQUEST_2, "key clear", "");
}

#[test]
fn a_key_is_removed_and_its_file_is_left() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let file = world.home.path().join("work");

    let removed = world.execute(REQUEST_1, remove(work));
    assert_success(&removed);
    assert_eq!(key_data(&removed), &named(Some(work)));
    assert!(world.keys().is_empty());
    assert!(file.exists());
    assert_finished(&world, REQUEST_1, "key remove", &work.to_string());

    // Replay: the stored result, and nothing runs.
    assert_eq!(world.execute(REQUEST_1, remove(work)), removed);
}

#[test]
fn removing_the_selected_key_is_selected_key_in_use() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    world.service.select_shared_key(work).unwrap();

    let refused = world.execute(REQUEST_1, remove(work));
    assert_eq!(
        (refused.outcome, refused.code, refused.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::SelectedKeyInUse,
            Some(FailureClass::Blocked)
        )
    );
    assert_of_the_application(&refused);
    assert_eq!(key_data(&refused), &named(Some(work)));
    assert!(refused.recovery.is_empty());
    // Nothing is left of it: the key, its selection, and no record.
    assert_eq!(world.keys(), [(work, true)]);
    assert_eq!(world.request_rows(), 0);

    // Once the selection is cleared, the same request ID removes it.
    assert_success(&world.execute(REQUEST_2, clear()));
    assert_success(&world.execute(REQUEST_1, remove(work)));
    assert!(world.keys().is_empty());
}

#[test]
fn removing_a_key_that_is_not_registered_is_key_not_found() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let unknown = SharedKeyId::new();

    let refused = world.execute(REQUEST_1, remove(unknown));
    assert_eq!(
        (refused.outcome, refused.code, refused.failure_class()),
        (
            Outcome::Error,
            ResultCode::KeyNotFound,
            Some(FailureClass::Input)
        )
    );
    assert_of_the_application(&refused);
    assert_eq!(key_data(&refused), &named(Some(unknown)));
    assert!(refused.recovery.is_empty());
    assert_eq!(world.keys(), [(work, false)]);
    assert_eq!(world.request_rows(), 0);

    // A key that was removed is no longer registered either: a new
    // request for it is refused, and is no no-op.
    assert_success(&world.execute(REQUEST_2, remove(work)));
    let gone = world.execute(REQUEST_3, remove(work));
    assert_eq!(gone.code, ResultCode::KeyNotFound);
}

#[test]
fn a_changed_key_request_is_rejected_and_nothing_runs() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");
    assert_success(&world.execute(REQUEST_1, select(work)));

    // The same request ID with another key, and with other commands.
    for changed in [select(other), clear(), remove(work), remove(other)] {
        let command = changed.command();
        let mismatch = world.execute(REQUEST_1, changed);
        assert_eq!(
            (mismatch.outcome, mismatch.code, mismatch.failure_class()),
            (
                Outcome::Error,
                ResultCode::RequestMismatch,
                Some(FailureClass::Input)
            ),
            "{command}"
        );
        assert_of_the_application(&mismatch);
        assert_eq!(mismatch.command, command);
    }
    assert_eq!(world.keys(), [(work, true), (other, false)]);
    assert_eq!(world.request_rows(), 1);
}

/// A key command and a repository command are requests in two scopes: the
/// ID of one is not the other's.
#[test]
fn a_request_id_of_a_repository_command_is_not_a_key_command_s() {
    let world = World::new();
    let saved = world.execute(REQUEST_1, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(saved.outcome, Outcome::Success);

    let mismatch = world.execute(REQUEST_1, clear());
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.scope, Scope::default());
    assert_eq!(world.request_rows(), 1);

    // And a key command's ID is not a repository command's.
    assert_noop(&world.execute(REQUEST_2, clear()));
    let mismatch = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(
        mismatch.scope.repository.as_deref(),
        Some(world.root.to_str().unwrap())
    );
}

/// Makes the request under a service that dies before it settles the
/// record, and then again as a new process would: `holds` says the
/// request's end state is there.
fn lost_then_retried(
    world: &mut KeyWorld,
    request_id: &str,
    request: impl Fn() -> Mutation,
    holds: impl Fn(&KeyWorld) -> bool,
) {
    let command = request().command();
    assert!(!holds(world), "{command}: the request has something to do");
    world.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let lost = world.execute(request_id, request());
    assert_eq!(
        (lost.outcome, lost.code),
        (Outcome::Error, ResultCode::InternalError),
        "{command}"
    );
    assert!(holds(world), "{command}: the change was made");
    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Accepted, "{command}");
    assert!(record.operations.is_empty(), "{command}");

    world.reopen();
    let retry = world.execute(request_id, request());
    assert_noop(&retry);
    assert!(holds(world), "{command}");
    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Finished, "{command}");
    assert_eq!(record.attempt, 2, "{command}");
    // And from then on it is replayed.
    assert_eq!(world.execute(request_id, request()), retry, "{command}");
}

/// A registry change is one step with no operation. A call that made it
/// and whose result was lost left its record accepted; the same request
/// finds the state as it asked for, and that is its answer.
#[test]
fn a_registry_change_whose_result_was_lost_is_a_no_op_on_its_retry() {
    let mut world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");

    lost_then_retried(
        &mut world,
        REQUEST_1,
        || select(work),
        |world| world.selected() == Some(work),
    );
    lost_then_retried(
        &mut world,
        REQUEST_2,
        || remove(other),
        |world| world.keys() == [(work, true)],
    );
    lost_then_retried(&mut world, REQUEST_3, clear, |world| {
        world.selected().is_none()
    });
}

/// A call that was accepted and died before its domain call changed
/// nothing; the same request makes the change.
#[test]
fn a_registry_change_that_died_before_its_step_is_made_by_its_retry() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(data)
    });
    let work = world.register("work", "Work laptop");

    let died = world.execute(REQUEST_1, select(work));
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(world.selected(), None);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    world.reopen();
    let retry = world.execute(REQUEST_1, select(work));
    assert_success(&retry);
    assert_eq!(world.selected(), Some(work));
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

/// A key command on a store that cannot be read is answered as any
/// command but a rebuild is: nothing is restored for it.
#[test]
fn a_key_command_on_an_unreadable_index_is_index_unavailable() {
    let fixture = crate::support::born_repository();
    let (data, service) = items::degraded_service(crate::support::enabled_repository(&fixture));
    let index = data.path().join("manyhands.sqlite3");
    let unreadable = std::fs::read(&index).unwrap();

    for request in [
        clear(),
        select(SharedKeyId::new()),
        remove(SharedKeyId::new()),
    ] {
        let command = request.command();
        let refused = execute_with(&service, REQUEST_1, request);
        assert_eq!(refused.code, ResultCode::IndexUnavailable, "{command}");
        assert_of_the_application(&refused);
        // The action names no repository: the request has none.
        assert_eq!(recovery_actions(&refused), ["index.rebuild"], "{command}");
        assert!(refused.recovery[0].arguments.is_empty(), "{command}");
        assert_eq!(std::fs::read(&index).unwrap(), unreadable, "{command}");
    }
}

#[test]
fn a_request_with_a_confirmation_or_cancelled_before_acceptance_does_nothing() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");

    let confirmed = execute_confirmed(
        &world.service,
        REQUEST_1,
        "01ARZ3NDEKTSV4RRFFQ69G5FN0",
        select(work),
    );
    assert_eq!(confirmed.code, ResultCode::NotConfirmable);
    assert_of_the_application(&confirmed);

    let cancel = manyhands::repository::CancellationToken::new();
    cancel.cancel();
    let cancelled = execute_cancellable(&world.service, REQUEST_1, select(work), &cancel);
    assert_eq!(
        (cancelled.outcome, cancelled.code),
        (Outcome::Cancelled, ResultCode::Cancelled)
    );
    assert_of_the_application(&cancelled);
    assert_eq!(world.selected(), None);
    assert_eq!(world.request_rows(), 0);
    assert!(recovery_actions(&cancelled).is_empty());
}
