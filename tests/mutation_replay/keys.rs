//! The shared-key commands: requests of the application, which name no
//! repository. Every effect is `not_requested`, `data` names the key, and
//! the outcome says whether anything changed.
//!
//! `key import`, `key select`, `key clear` and `key remove` change the key
//! registry in one step: a request of one is finished when its end state
//! holds, and leaves nothing when it is refused. An import, a selection
//! and a clearing are recorded against an operation in that step, and a
//! retry is answered from the record; a removal has no operation.

use std::path::Path;

use manyhands::{
    repository::{
        FailurePoint, KeyClearInput, KeyImportInput, KeyMutationDto, KeyRemoveInput,
        KeySelectInput, Mutation, MutationDataDto, RequestState, SharedKeyId,
        request_store::ScopeKey,
    },
    results::{Effects, Envelope, FailureClass, Outcome, ResultCode, Scope},
};

use crate::support::{
    FailOnce,
    conformance::{Ask, Expect, Row, Subject},
    golden,
    items::{self, TICKET_A},
    mutation::{
        self, KeyWorld, PUBLIC_KEY_FINGERPRINT, REQUEST_1, REQUEST_2, REQUEST_3, World,
        execute_cancellable, execute_confirmed, execute_with, key_data, recovery_actions,
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
/// effect, and a scope that names nothing.
fn assert_of_the_application(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.scope, Scope::default());
}

/// The same for a request that was not accepted: it names no operation.
fn assert_not_accepted(envelope: &Envelope<MutationDataDto>) {
    assert_of_the_application(envelope);
    assert_eq!(envelope.operation_id, None);
}

/// Whether a change of the command is recorded against an operation: an
/// import, a selection and a clearing are, and a removal is not.
fn recorded(command: &str) -> bool {
    match command {
        "key import" | "key select" | "key clear" => true,
        "key remove" => false,
        other => panic!("no change to the key registry: {other}"),
    }
}

/// A result of a request that was accepted names the request's
/// operation, for a command that has one.
fn assert_accepted(envelope: &Envelope<MutationDataDto>) {
    assert_of_the_application(envelope);
    assert_eq!(
        envelope.operation_id.is_some(),
        recorded(&envelope.command),
        "{}",
        envelope.command
    );
}

fn assert_success(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_accepted(envelope);
    assert!(envelope.recovery.is_empty());
}

fn assert_noop(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_accepted(envelope);
    assert!(envelope.recovery.is_empty());
}

/// The rows of the recorded registry changes, by operation: what was
/// asked, the key and whether anything changed.
fn changes(world: &KeyWorld) -> Vec<(String, String, Option<String>, bool)> {
    items::index(world.data.path())
        .prepare(
            "SELECT operation_id, action, key_id, changed FROM key_registry_changes
              ORDER BY rowid",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Asserts that the record holds what its command's records hold of an
/// operation: one of the key journal's family for a change that is
/// recorded, which the table of recorded changes holds once under that
/// ID exactly when `made`, and none for a removal.
fn assert_operation(world: &KeyWorld, request_id: &str, made: bool) {
    let record = world.record(request_id).expect("a record");
    if !recorded(&record.command) {
        assert!(record.operations.is_empty(), "{:?}", record.operations);
        return;
    }
    let [operation] = &record.operations[..] else {
        panic!("{}: one operation: {:?}", record.command, record.operations);
    };
    assert_eq!(
        operation.family,
        manyhands::repository::OperationFamily::KeyMaterial,
        "{}",
        record.command
    );
    let action = record.command.trim_start_matches("key ");
    let held = changes(world)
        .into_iter()
        .filter(|(id, _, _, _)| *id == operation.operation_id.to_string())
        .map(|(_, action, _, _)| action)
        .collect::<Vec<_>>();
    assert_eq!(
        held,
        if made {
            vec![action.to_owned()]
        } else {
            Vec::new()
        },
        "{}",
        record.command
    );
}

/// What a request reports of the key `id` when it read its registration.
fn read(id: SharedKeyId, label: &str) -> KeyMutationDto {
    KeyMutationDto {
        id: Some(id.to_string()),
        label: Some(label.to_owned()),
        // The fixture registers a key with no public key file.
        fingerprint: None,
        generation: None,
        deletion: None,
    }
}

/// What a request reports of the key `id` when it read nothing of it.
fn named(id: Option<SharedKeyId>) -> KeyMutationDto {
    KeyMutationDto {
        id: id.map(|id| id.to_string()),
        label: None,
        fingerprint: None,
        generation: None,
        deletion: None,
    }
}

/// Asserts that the request left a finished record of the application,
/// with the operation its change is recorded against. A selection, a
/// clearing and an import record a digest of the setting their first
/// call observed, for a retry whose operation recorded nothing to
/// compare; a removal records none, and has no operation.
fn assert_finished(world: &KeyWorld, request_id: &str, command: &str, target: &str) {
    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.scope, ScopeKey::application());
    assert_eq!(record.scope.as_str(), "application");
    assert_eq!(record.command, command);
    assert_eq!(record.target, target);
    assert_operation(world, request_id, true);
    assert_eq!((record.base_ref, record.base_oid), (None, None));
    assert_eq!(
        matches!(
            record.expected_digest,
            Some(manyhands::repository::ExpectedPathObservation::Blake3(_))
        ),
        command != "key remove",
        "{command}"
    );
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
    assert_not_accepted(&refused);
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
    assert_not_accepted(&refused);
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
    assert_not_accepted(&refused);
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
        assert_not_accepted(&mismatch);
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
/// request's end state is there. A change that is recorded is answered
/// as its first call would have been, from its record; a removal finds
/// its key gone, which is a no-op.
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
    assert_operation(world, request_id, true);

    world.reopen();
    let retry = world.execute(request_id, request());
    if recorded(command) {
        assert_success(&retry);
    } else {
        assert_noop(&retry);
    }
    assert!(holds(world), "{command}");
    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Finished, "{command}");
    assert_eq!(record.attempt, 2, "{command}");
    // And from then on it is replayed.
    assert_eq!(world.execute(request_id, request()), retry, "{command}");
}

/// A registry change is one step. A call that made it and whose result
/// was lost left its record accepted; the same request is answered as
/// that call would have been.
#[test]
fn a_registry_change_whose_result_was_lost_is_answered_on_its_retry() {
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
        assert_not_accepted(&refused);
        // A rebuild is of a repository, by its root, and this result
        // names none: no action is offered that could not be carried out.
        assert!(recovery_actions(&refused).is_empty(), "{command}");
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
    assert_not_accepted(&confirmed);

    let cancel = manyhands::repository::CancellationToken::new();
    cancel.cancel();
    let cancelled = execute_cancellable(&world.service, REQUEST_1, select(work), &cancel);
    assert_eq!(
        (cancelled.outcome, cancelled.code),
        (Outcome::Cancelled, ResultCode::Cancelled)
    );
    assert_not_accepted(&cancelled);
    assert_eq!(world.selected(), None);
    assert_eq!(world.request_rows(), 0);
    assert!(recovery_actions(&cancelled).is_empty());
}

fn import(label: &str, private: &Path, public: Option<&Path>) -> Mutation {
    Mutation::KeyImport(KeyImportInput {
        label: label.to_owned(),
        private_key_path: private.to_owned(),
        public_key_path: public.map(Path::to_owned),
    })
}

/// A directory name that marks the paths of an imported key: no request
/// store and no result may hold where a key file is.
const PATH_SENTINEL: &str = "SENTINEL-KEYPATH-5d1e";

/// Runs a request and checks that the path of the key it imports is in
/// neither its envelope nor any request store.
fn execute_import(
    world: &KeyWorld,
    request_id: &str,
    request: Mutation,
) -> Envelope<MutationDataDto> {
    let envelope = world.execute(request_id, request);
    let text = serde_json::to_string(&envelope).unwrap();
    assert!(
        !text.contains(PATH_SENTINEL),
        "the envelope holds a path: {text}"
    );
    mutation::assert_request_stores_exclude(world.data.path(), &[PATH_SENTINEL]);
    envelope
}

fn imported(id: &str, label: &str) -> KeyMutationDto {
    KeyMutationDto {
        id: Some(id.to_owned()),
        label: Some(label.to_owned()),
        fingerprint: Some(PUBLIC_KEY_FINGERPRINT.to_owned()),
        generation: None,
        deletion: None,
    }
}

#[test]
fn a_key_is_imported_and_the_same_source_again_is_a_no_op_naming_it() {
    let world = KeyWorld::new();
    let (private, public) = world.key_files(PATH_SENTINEL);
    let source = std::fs::read(&private).unwrap();

    let first = execute_import(
        &world,
        REQUEST_1,
        import("Work laptop", &private, Some(&public)),
    );
    assert_success(&first);
    let [(id, false)] = world.keys()[..] else {
        panic!("one key, not selected: {:?}", world.keys());
    };
    assert_eq!(key_data(&first), &imported(&id.to_string(), "Work laptop"));
    // The registration names the files where they are: neither is changed.
    assert_eq!(std::fs::read(&private).unwrap(), source);
    // An import names a source and no key, so its record has no target:
    // the path is in the digest alone.
    assert_finished(&world, REQUEST_1, "key import", "");

    // Replay: the stored result, and nothing runs.
    assert_eq!(
        execute_import(
            &world,
            REQUEST_1,
            import("Work laptop", &private, Some(&public))
        ),
        first
    );

    // A new request for the same source, under another label, registers
    // nothing and names the key that holds it, by its own label.
    let again = execute_import(
        &world,
        REQUEST_2,
        import("Renamed", &private, Some(&public)),
    );
    assert_noop(&again);
    assert_eq!(key_data(&again), &imported(&id.to_string(), "Work laptop"));
    assert_eq!(world.keys(), [(id, false)]);
    assert_finished(&world, REQUEST_2, "key import", "");
}

#[test]
fn an_import_with_a_changed_label_or_source_is_another_request() {
    let world = KeyWorld::new();
    let (private, public) = world.key_files(PATH_SENTINEL);
    let (other, _) = world.key_files("other");
    assert_success(&execute_import(
        &world,
        REQUEST_1,
        import("Work laptop", &private, Some(&public)),
    ));

    for changed in [
        import("Renamed", &private, Some(&public)),
        import("Work laptop", &other, Some(&public)),
        import("Work laptop", &private, None),
        import("Work laptop", &private, Some(&other)),
    ] {
        let mismatch = execute_import(&world, REQUEST_1, changed);
        assert_eq!(
            (mismatch.outcome, mismatch.code),
            (Outcome::Error, ResultCode::RequestMismatch)
        );
        assert_not_accepted(&mismatch);
    }
    assert_eq!(world.keys().len(), 1);
    assert_eq!(world.request_rows(), 1);
}

#[test]
fn an_import_the_registry_refuses_is_invalid_input_and_leaves_nothing() {
    let world = KeyWorld::new();
    let (private, public) = world.key_files(PATH_SENTINEL);

    for refused in [
        import("  ", &private, Some(&public)),
        import("Work laptop", Path::new("relative/key"), None),
        import("Work laptop", &private, Some(Path::new("relative/key.pub"))),
    ] {
        let refused = execute_import(&world, REQUEST_1, refused);
        assert_eq!(
            (refused.outcome, refused.code, refused.failure_class()),
            (
                Outcome::Error,
                ResultCode::InvalidInput,
                Some(FailureClass::Input)
            )
        );
        // Refused by the domain call, after the request was accepted:
        // the result names the operation nothing was recorded against.
        assert_accepted(&refused);
        assert_eq!(key_data(&refused), &named(None));
        assert!(refused.recovery.is_empty());
        assert!(world.keys().is_empty());
        assert_eq!(world.request_rows(), 0);
        assert!(changes(&world).is_empty());
    }

    // The same request ID, with the input corrected, runs at once.
    assert_success(&execute_import(
        &world,
        REQUEST_1,
        import("Work laptop", &private, Some(&public)),
    ));
}

/// An import whose result was lost registered the key. The same request
/// is answered with that key, as registered by it: one key.
#[test]
fn an_import_whose_result_was_lost_names_its_key_on_its_retry() {
    let mut world = KeyWorld::new();
    let (private, public) = world.key_files(PATH_SENTINEL);
    lost_then_retried(
        &mut world,
        REQUEST_1,
        || import("Work laptop", &private, Some(&public)),
        |world| world.keys().len() == 1,
    );
    let id = world.keys()[0].0;
    let replayed = execute_import(
        &world,
        REQUEST_1,
        import("Work laptop", &private, Some(&public)),
    );
    assert_eq!(
        key_data(&replayed),
        &imported(&id.to_string(), "Work laptop")
    );
}

/// Makes the request under a service that dies before it settles the
/// record, and leaves the world as a new process would find it.
fn lose_the_result(world: &mut KeyWorld, request_id: &str, request: Mutation) {
    world.fail_once_at(FailurePoint::BeforeRequestSettlement);
    let lost = world.execute(request_id, request);
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(
        world.record(request_id).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    world.reopen();
}

/// Asserts that a retry was refused because the setting it would change
/// has moved since its request was accepted: nothing is changed, and the
/// request is finished with that answer, which is given again from then
/// on.
fn assert_refused_as_late(
    world: &KeyWorld,
    request_id: &str,
    request: impl Fn() -> Mutation,
    retry: &Envelope<MutationDataDto>,
) {
    let command = request().command();
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::ExternalChange,
            Some(FailureClass::Blocked)
        ),
        "{command}"
    );
    assert_accepted(retry);
    assert!(retry.recovery.is_empty(), "{command}");
    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Finished, "{command}");
    // Nothing was made, so nothing is recorded against the operation.
    assert_operation(world, request_id, false);
    assert_eq!(&world.execute(request_id, request()), retry, "{command}");
}

/// A call that died before its step changed and recorded nothing. Its
/// retry does not make its change over one made since: the setting is
/// neither as the first call observed it nor as the request asks.
#[test]
fn a_registry_change_that_died_before_its_step_is_not_made_over_a_later_one() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(data)
    });
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");
    let died = world.execute(REQUEST_1, select(work));
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(world.selected(), None);
    world.reopen();
    world.service.select_shared_key(other).unwrap();

    let retry = world.execute(REQUEST_1, select(work));
    assert_refused_as_late(&world, REQUEST_1, || select(work), &retry);
    assert_eq!(world.selected(), Some(other));
}

/// Every table of the index that holds `text` in any column of any row.
fn tables_holding(world: &KeyWorld, text: &str) -> Vec<String> {
    let connection = items::index(world.data.path());
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut holding = Vec::new();
    for table in tables {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{table}\""))
            .unwrap();
        let columns = statement.column_count();
        let mut rows = statement.query([]).unwrap();
        let mut found = false;
        while let Some(row) = rows.next().unwrap() {
            for column in 0..columns {
                let bytes = match row.get_ref(column).unwrap() {
                    rusqlite::types::ValueRef::Text(bytes)
                    | rusqlite::types::ValueRef::Blob(bytes) => bytes,
                    rusqlite::types::ValueRef::Null
                    | rusqlite::types::ValueRef::Integer(_)
                    | rusqlite::types::ValueRef::Real(_) => continue,
                };
                found |= String::from_utf8_lossy(bytes).contains(text);
            }
        }
        if found {
            holding.push(table);
        }
    }
    holding
}

/// The evidence a retry is answered from. An import, a selection and a
/// clearing are each recorded against the operation their accepted
/// record names, in the transaction that makes the change: a call whose
/// result was lost has left the row. The key journal gains no row. Once
/// the key an import registered is removed, the record of the import is
/// what still names it.
#[test]
fn a_registry_change_is_recorded_against_its_operation_by_the_transaction_that_makes_it() {
    let mut world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let (private, public) = world.key_files("imported");
    const REQUEST_4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";

    lose_the_result(
        &mut world,
        REQUEST_1,
        import("Imported", &private, Some(&public)),
    );
    let imported_key = world.keys()[1].0;
    let imported = imported_key.to_string();
    lose_the_result(&mut world, REQUEST_2, select(work));
    lose_the_result(&mut world, REQUEST_3, clear());
    // A change that changes nothing is recorded as that.
    lose_the_result(&mut world, REQUEST_4, clear());

    let mut operations = Vec::new();
    for request in [REQUEST_1, REQUEST_2, REQUEST_3, REQUEST_4] {
        let record = world.record(request).expect("a record");
        assert_eq!(record.state, RequestState::Accepted, "{}", record.command);
        assert_eq!(record.result, None, "{}", record.command);
        assert_operation(&world, request, true);
        operations.push(record.operations[0].operation_id.to_string());
    }
    assert_eq!(
        changes(&world),
        [
            (
                operations[0].clone(),
                "import".to_owned(),
                Some(imported.clone()),
                true
            ),
            (
                operations[1].clone(),
                "select".to_owned(),
                Some(work.to_string()),
                true
            ),
            (operations[2].clone(), "clear".to_owned(), None, true),
            (operations[3].clone(), "clear".to_owned(), None, false),
        ]
    );
    // The key journal is of generations and deletions alone.
    assert_eq!(world.key_operations(), 0);
    assert_eq!(
        tables_holding(&world, &imported),
        ["key_registry_changes", "shared_ssh_keys"]
    );
    world.service.unregister_shared_key(imported_key).unwrap();
    assert_eq!(tables_holding(&world, &imported), ["key_registry_changes"]);
}

/// What `request show` gives of a change to the key registry: the one
/// operation its change is recorded against, of the key journal's
/// family, and for a removal, which has no operation, an empty list,
/// which its schema allows.
#[test]
fn a_registry_change_is_shown_with_its_operation_and_a_removal_with_none() {
    let mut world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");
    assert_success(&world.execute(REQUEST_1, select(work)));
    lose_the_result(&mut world, REQUEST_2, clear());
    assert_success(&world.execute(REQUEST_3, remove(other)));

    for (request, state, operations) in [
        (REQUEST_1, "finished", 1),
        (REQUEST_2, "accepted", 1),
        (REQUEST_3, "finished", 0),
    ] {
        let shown = world
            .service
            .show_request(manyhands::repository::RequestId::parse(request).unwrap());
        let value = serde_json::to_value(&shown).unwrap();
        crate::support::schema::check_published(golden::ENVELOPE_SCHEMA, &value).unwrap();
        crate::support::schema::check_published("request.schema.json", &value["data"]).unwrap();
        assert_eq!(value["code"], "ok", "{request}");
        assert_eq!(value["data"]["state"], state, "{request}");
        let shown = value["data"]["operations"].as_array().unwrap();
        assert_eq!(shown.len(), operations, "{request}");
        for operation in shown {
            assert_eq!(operation["family"], "key_material", "{request}");
            assert_eq!(
                operation["operation_id"],
                world.record(request).unwrap().operations[0]
                    .operation_id
                    .to_string(),
                "{request}"
            );
        }
        assert_eq!(value["scope"]["repository"], serde_json::Value::Null);
    }
}

/// What a private key file holds and nothing but the file may: the key.
const KEY_SENTINEL: &str = "SENTINEL-KEYBYTES-8e2c";

/// A selection, a clearing and a removal name a key by its ID and read
/// its registration, which says where the key's files are. Nothing they
/// return or record holds where the files are or what they hold: not
/// when they change the registry, change nothing, are refused or are
/// given again from their records.
#[test]
fn nothing_of_a_selection_a_clearing_or_a_removal_holds_the_key_or_where_it_is() {
    let sentinels = [PATH_SENTINEL, KEY_SENTINEL];
    let world = KeyWorld::new();
    let (private, public) = world.key_files(PATH_SENTINEL);
    std::fs::write(&private, format!("{KEY_SENTINEL}\n")).unwrap();
    let registered = execute_import(
        &world,
        REQUEST_1,
        import("Work laptop", &private, Some(&public)),
    );
    assert_success(&registered);
    let id = SharedKeyId::parse(key_data(&registered).id.as_deref().unwrap()).unwrap();
    // Both really are in what the commands read, so the scans can fail:
    // the registration names the file, and the file holds the key.
    let registration = world.service.show_key(id).unwrap();
    assert!(format!("{registration:?}").contains(PATH_SENTINEL));
    assert!(
        std::fs::read_to_string(&private)
            .unwrap()
            .contains(KEY_SENTINEL)
    );
    let scanned = |request_id: &str, request: Mutation| {
        let envelope = world.execute(request_id, request);
        assert_eq!(
            golden::find_sentinel(&serde_json::to_value(&envelope).unwrap(), &sentinels),
            None
        );
        mutation::assert_request_stores_exclude(world.data.path(), &sentinels);
        envelope
    };
    const REQUEST_4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
    const REQUEST_5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX5";
    const REQUEST_6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX6";

    let selected = scanned(REQUEST_2, select(id));
    assert_success(&selected);
    assert_eq!(scanned(REQUEST_2, select(id)), selected);
    assert_noop(&scanned(REQUEST_3, select(id)));
    // Refused: the key is selected.
    let refused = scanned(REQUEST_4, remove(id));
    assert_eq!(refused.code, ResultCode::SelectedKeyInUse);
    assert_success(&scanned(REQUEST_4, clear()));
    assert_noop(&scanned(REQUEST_5, clear()));
    let removed = scanned(REQUEST_6, remove(id));
    assert_success(&removed);
    assert_eq!(scanned(REQUEST_6, remove(id)), removed);
    // The scans read the records the six requests left.
    assert_eq!(world.request_rows(), 6);
    // The file is where it was, and holds what it held.
    assert!(
        std::fs::read_to_string(&private)
            .unwrap()
            .contains(KEY_SENTINEL)
    );
}

/// A world with two registered keys, `work` and `other`, and `rule`
/// driven through the requests `request` gives of them.
fn conforming(
    request: fn(Ask, SharedKeyId, SharedKeyId) -> Mutation,
    rule: &mut dyn FnMut(&Subject<'_>),
) {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");
    mutation::drive_plain(
        &world.service,
        world.data.path(),
        &|ask| request(ask, work, other),
        rule,
    );
}

/// The rows of the four changes to the key registry in the conformance
/// table.
pub(crate) const CONFORMANCE: [Row; 4] = [
    Row::new("key import", false, "invalid_input", |rule| {
        let world = KeyWorld::new();
        let (private, public) = world.key_files(PATH_SENTINEL);
        mutation::drive_plain(
            &world.service,
            world.data.path(),
            &|ask| match ask {
                Ask::Same | Ask::Unconsented => import("Work laptop", &private, Some(&public)),
                Ask::Changed => import("Renamed", &private, Some(&public)),
                // No label.
                Ask::Refused => import("  ", &private, Some(&public)),
            },
            rule,
        );
        mutation::assert_request_stores_exclude(world.data.path(), &[PATH_SENTINEL]);
        Ok(())
    }),
    Row::new("key select", false, "key_not_found", |rule| {
        conforming(
            |ask, work, other| match ask {
                Ask::Same | Ask::Unconsented => select(work),
                Ask::Changed => select(other),
                Ask::Refused => select(SharedKeyId::new()),
            },
            rule,
        );
        Ok(())
    }),
    Row {
        // `key clear` has no input and no state it is refused for: with
        // nothing selected it is a no-op, which is a finished request.
        // The only request of it that is refused is one that presents a
        // confirmation, which the consent rule drives.
        refusal: Expect::Exempt(
            "the command has no input, and no state of the registry refuses it: a clearing \
             with nothing selected is a no-op and is recorded as one",
        ),
        ..Row::new("key clear", false, "", |rule| {
            conforming(
                |ask, work, _| match ask {
                    Ask::Same | Ask::Unconsented | Ask::Refused => clear(),
                    // The command has no input: another command.
                    Ask::Changed => select(work),
                },
                rule,
            );
            Ok(())
        })
    },
    Row::new("key remove", false, "key_not_found", |rule| {
        conforming(
            |ask, work, other| match ask {
                Ask::Same | Ask::Unconsented => remove(work),
                Ask::Changed => remove(other),
                Ask::Refused => remove(SharedKeyId::new()),
            },
            rule,
        );
        Ok(())
    }),
];

/// Everything the index holds but the request tables, which a retry that
/// finishes its record changes: every other table, with every row.
fn all_but_the_requests(world: &KeyWorld) -> Vec<(String, Vec<String>)> {
    let connection = items::index(world.data.path());
    let tables: Vec<String> = connection
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table'
                AND name NOT IN ('request_records', 'request_operations')
              ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let mut statement = connection
                .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
                .unwrap();
            let columns = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    Ok((0..columns)
                        .map(|column| format!("{:?}", row.get_ref(column).unwrap()))
                        .collect::<Vec<_>>()
                        .join("|"))
                })
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}

/// Sends a request again whose first call made its change and lost its
/// result, and asserts that it is answered as that call would have been
/// and that the retry wrote nothing but its own record: the key registry
/// and everything else the index holds are as they were before it.
fn answered_from_its_evidence(
    world: &KeyWorld,
    request_id: &str,
    request: impl Fn() -> Mutation,
    data: &KeyMutationDto,
) -> Envelope<MutationDataDto> {
    let command = request().command();
    let before = all_but_the_requests(world);
    let retry = world.execute(request_id, request());
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (Outcome::Success, ResultCode::Ok, None),
        "{command}"
    );
    assert_eq!(key_data(&retry), data, "{command}");
    assert_eq!(all_but_the_requests(world), before, "{command}");
    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Finished, "{command}");
    // And from then on it is replayed.
    assert_eq!(world.execute(request_id, request()), retry, "{command}");
    assert_eq!(all_but_the_requests(world), before, "{command}");
    retry
}

/// An import registered a key and its result was lost. Someone removes
/// the key. The same request is answered with the key it registered: the
/// source is not registered a second time, under another ID.
#[test]
fn an_import_whose_result_was_lost_is_answered_with_its_key_though_the_key_was_removed() {
    let mut world = KeyWorld::new();
    let (private, public) = world.key_files(PATH_SENTINEL);
    let import_it = || import("Imported", &private, Some(&public));
    lose_the_result(&mut world, REQUEST_1, import_it());
    let [(first, false)] = world.keys()[..] else {
        panic!("one key: {:?}", world.keys());
    };
    world.service.unregister_shared_key(first).unwrap();
    assert!(world.keys().is_empty());

    let retry = answered_from_its_evidence(
        &world,
        REQUEST_1,
        import_it,
        &imported(&first.to_string(), "Imported"),
    );
    assert_eq!(key_data(&retry).id, Some(first.to_string()));
    // No second key.
    assert!(world.keys().is_empty());
    mutation::assert_request_stores_exclude(world.data.path(), &[PATH_SENTINEL]);
}

/// A selection was made and its result lost. Whatever was selected since,
/// by anyone, the same request is answered as its first call would have
/// been and selects nothing: not when another key is selected now, and
/// not when the first was selected again.
#[test]
fn a_selection_whose_result_was_lost_is_answered_and_not_made_again() {
    // Changed and put back.
    let mut world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");
    lose_the_result(&mut world, REQUEST_1, select(work));
    world.service.select_shared_key(other).unwrap();
    world.service.select_shared_key(work).unwrap();
    answered_from_its_evidence(
        &world,
        REQUEST_1,
        || select(work),
        &read(work, "Work laptop"),
    );
    assert_eq!(world.selected(), Some(work));

    // Changed: the later choice stands.
    world.service.clear_shared_key_selection().unwrap();
    lose_the_result(&mut world, REQUEST_2, select(work));
    world.service.select_shared_key(other).unwrap();
    answered_from_its_evidence(
        &world,
        REQUEST_2,
        || select(work),
        &read(work, "Work laptop"),
    );
    assert_eq!(world.selected(), Some(other));

    // Removed: the key the request selected is not registered any more.
    world.service.clear_shared_key_selection().unwrap();
    lose_the_result(&mut world, REQUEST_3, select(work));
    world.service.clear_shared_key_selection().unwrap();
    world.service.unregister_shared_key(work).unwrap();
    answered_from_its_evidence(
        &world,
        REQUEST_3,
        || select(work),
        &read(work, "Work laptop"),
    );
    assert_eq!(world.keys(), [(other, false)]);
}

/// The same for a clearing: a key that was selected again since, the same
/// one or another, stays selected.
#[test]
fn a_clearing_whose_result_was_lost_is_answered_and_not_made_again() {
    let mut world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let other = world.register("other", "Other");

    // Put back: the key that was selected is selected again.
    world.service.select_shared_key(work).unwrap();
    lose_the_result(&mut world, REQUEST_1, clear());
    assert_eq!(world.selected(), None);
    world.service.select_shared_key(work).unwrap();
    answered_from_its_evidence(&world, REQUEST_1, clear, &named(None));
    assert_eq!(world.selected(), Some(work));

    // Changed: another key is selected.
    lose_the_result(&mut world, REQUEST_2, clear());
    assert_eq!(world.selected(), None);
    world.service.select_shared_key(other).unwrap();
    answered_from_its_evidence(&world, REQUEST_2, clear, &named(None));
    assert_eq!(world.selected(), Some(other));
}

/// A request that found its setting already as it asks changed nothing,
/// and that is its answer for good: a no-op naming the key, also once the
/// key is gone.
#[test]
fn a_registry_change_that_changed_nothing_is_answered_as_a_no_op_whatever_changed_since() {
    let mut world = KeyWorld::new();
    let (private, public) = world.key_files("imported");
    let import_it = || import("Renamed", &private, Some(&public));
    assert_success(&world.execute(REQUEST_1, import("Imported", &private, Some(&public))));
    let first = world.keys()[0].0;
    lose_the_result(&mut world, REQUEST_2, import_it());
    world.service.unregister_shared_key(first).unwrap();

    let before = all_but_the_requests(&world);
    let retry = world.execute(REQUEST_2, import_it());
    assert_noop(&retry);
    assert_eq!(key_data(&retry), &imported(&first.to_string(), "Imported"));
    assert_eq!(all_but_the_requests(&world), before);
    assert!(world.keys().is_empty());
    assert_eq!(world.execute(REQUEST_2, import_it()), retry);
}

/// The look a retry takes at its setting and the change it then makes
/// are one transaction. A change someone else makes after the retry has
/// read its record, and before its domain call, is seen by that call: the
/// request is an external change, and the later choice is not overwritten.
#[test]
fn a_change_made_between_a_retry_s_look_and_its_change_is_not_overwritten() {
    for clearing in [false, true] {
        let mut world = KeyWorld::with_service(|data| {
            FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(data)
        });
        let work = world.register("work", "Work laptop");
        let other = world.register("other", "Other");
        let third = world.register("third", "Third");
        if clearing {
            world.service.select_shared_key(third).unwrap();
        }
        let request = move || if clearing { clear() } else { select(work) };
        let unchanged = clearing.then_some(third);
        let died = world.execute(REQUEST_1, request());
        assert_eq!(died.code, ResultCode::InternalError, "{clearing}");
        assert_eq!(world.selected(), unchanged, "{clearing}");
        world.reopen();

        // After the retry has entered its record, and before its change.
        let data = world.data.path().to_owned();
        world.service.set_request_hook_for_testing(move || {
            manyhands::repository::RepositoryService::open_at(&data)
                .unwrap()
                .select_shared_key(other)
                .unwrap();
        });
        let retry = world.execute(REQUEST_1, request());
        assert_eq!(world.selected(), Some(other), "{clearing}");
        assert_refused_as_late(&world, REQUEST_1, request, &retry);
        assert_eq!(world.selected(), Some(other), "{clearing}");
    }
}

/// What a recorded registry change keeps: the operation, what was done,
/// the key's ID and the label and fingerprint its result gives. Nothing
/// of where the key's files are, of what they hold, or of a passphrase.
#[test]
fn nothing_of_a_recorded_registry_change_holds_the_key_its_passphrase_or_where_it_is() {
    let sentinels = [PATH_SENTINEL, KEY_SENTINEL, mutation::PASSPHRASE];
    let mut world = KeyWorld::new();
    let (private, public) = world.key_files(PATH_SENTINEL);
    std::fs::write(&private, format!("{KEY_SENTINEL}\n")).unwrap();
    // A generated key, protected by the passphrase, selected and cleared
    // beside the imported one.
    let generated = world.execute(
        REQUEST_1,
        world.generate("Generated", Some(mutation::PASSPHRASE)),
    );
    assert_eq!(generated.outcome, Outcome::Success);
    let generated = SharedKeyId::parse(key_data(&generated).id.as_deref().unwrap()).unwrap();
    const REQUEST_4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
    const REQUEST_5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX5";
    const REQUEST_6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX6";
    const REQUEST_7: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX7";

    let scanned = |world: &KeyWorld, request_id: &str, request: Mutation| {
        let envelope = world.execute(request_id, request);
        assert_eq!(
            golden::find_sentinel(&serde_json::to_value(&envelope).unwrap(), &sentinels),
            None
        );
        let record = world.record(request_id);
        let record = format!("{record:?}");
        for sentinel in sentinels {
            assert!(!record.contains(sentinel), "the record holds {sentinel}");
        }
        assert_eq!(
            mutation::find_stored_sentinel(world.data.path(), &sentinels),
            None
        );
        assert_eq!(tables_holding(world, PATH_SENTINEL), ["shared_ssh_keys"]);
        for sentinel in [KEY_SENTINEL, mutation::PASSPHRASE] {
            assert_eq!(tables_holding(world, sentinel), [""; 0], "{sentinel}");
        }
        envelope
    };

    let import_it = || import("Imported", &private, Some(&public));
    let registered = scanned(&world, REQUEST_2, import_it());
    assert_success_with_operation(&registered);
    let id = SharedKeyId::parse(key_data(&registered).id.as_deref().unwrap()).unwrap();
    assert_success_with_operation(&scanned(&world, REQUEST_3, select(id)));
    assert_success_with_operation(&scanned(&world, REQUEST_4, select(generated)));
    assert_success_with_operation(&scanned(&world, REQUEST_5, clear()));
    // A no-op is recorded as well.
    scanned(&world, REQUEST_6, clear());
    // And a retry that answers from the table.
    lose_the_result(&mut world, REQUEST_7, select(id));
    scanned(&world, REQUEST_7, select(id));
    assert_eq!(
        changes(&world)
            .into_iter()
            .map(|(_, action, key, changed)| (action, key, changed))
            .collect::<Vec<_>>(),
        [
            ("import".to_owned(), Some(id.to_string()), true),
            ("select".to_owned(), Some(id.to_string()), true),
            ("select".to_owned(), Some(generated.to_string()), true),
            ("clear".to_owned(), None, true),
            ("clear".to_owned(), None, false),
            ("select".to_owned(), Some(id.to_string()), true),
        ]
    );

    // The scan reads the table: a label is the one text of the caller's
    // it keeps, and one planted there is found. The import's result is
    // lost, so that no finished record holds the label before it.
    const LABEL_SENTINEL: &str = "SENTINEL-LABEL-2b7d";
    let (other, _) = world.key_files("other");
    const REQUEST_8: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX8";
    lose_the_result(&mut world, REQUEST_8, import(LABEL_SENTINEL, &other, None));
    assert_eq!(
        mutation::find_stored_sentinel(world.data.path(), &[LABEL_SENTINEL])
            .map(|found| (found.table, found.column)),
        Some(("key_registry_changes", "label".to_owned()))
    );
}

/// A successful registry change, which names its operation.
fn assert_success_with_operation(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert!(envelope.operation_id.is_some(), "{}", envelope.command);
}

/// A recorded registry change is finished the moment it is written:
/// nothing is left of it to take up. A resume of its operation is the
/// no-op a resume of any operation that has ended is, and records nothing.
#[test]
fn a_resume_of_a_recorded_registry_change_is_a_no_op() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let selected = world.execute(REQUEST_1, select(work));
    assert_eq!(selected.outcome, Outcome::Success);
    let operation_id = selected.operation_id.clone().expect("an operation ID");
    assert_eq!(
        world.record(REQUEST_1).unwrap().operations[0]
            .operation_id
            .to_string(),
        operation_id
    );
    let before = all_but_the_requests(&world);

    let resumed = world.execute(
        REQUEST_2,
        Mutation::OperationResume(manyhands::repository::OperationResumeInput {
            root: world.home.path().to_owned(),
            operation_id: manyhands::repository::OperationId::parse(&operation_id).unwrap(),
        }),
    );
    assert_eq!(
        (resumed.outcome, resumed.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert!(recovery_actions(&resumed).is_empty());
    assert!(world.record(REQUEST_2).is_none());
    assert_eq!(all_but_the_requests(&world), before);
    assert_eq!(world.selected(), Some(work));
}

/// A record an earlier build accepted for one of these commands holds no
/// operation: that build gave them none. It is entered as it was then,
/// and its change is made: the request is not refused for good because
/// its record is from before.
#[test]
fn a_record_accepted_before_these_commands_had_an_operation_is_still_entered() {
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(data)
    });
    let work = world.register("work", "Work laptop");
    let died = world.execute(REQUEST_1, select(work));
    assert_eq!(died.code, ResultCode::InternalError);
    // What that build left: the record, and no operation of it.
    let removed = items::index(world.data.path())
        .execute("DELETE FROM request_operations", [])
        .unwrap();
    assert_eq!(removed, 1);
    assert!(world.record(REQUEST_1).unwrap().operations.is_empty());
    world.reopen();

    let retry = world.execute(REQUEST_1, select(work));
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_eq!(key_data(&retry), &read(work, "Work laptop"));
    assert_eq!(retry.operation_id, None);
    assert_eq!(world.selected(), Some(work));
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(world.execute(REQUEST_1, select(work)), retry);

    // A removal's record never held one, and one of a command that has
    // always had an operation is still not entered without it.
    world.fail_once_at(FailurePoint::BeforeRequestDomainCall);
    let died = world.execute(
        REQUEST_2,
        world.generate("Generated", Some(mutation::PASSPHRASE)),
    );
    assert_eq!(died.code, ResultCode::InternalError);
    items::index(world.data.path())
        .execute("DELETE FROM request_operations", [])
        .unwrap();
    world.reopen();
    let refused = world.execute(
        REQUEST_2,
        world.generate("Generated", Some(mutation::PASSPHRASE)),
    );
    assert_eq!(refused.code, ResultCode::InternalError);
    assert_eq!(world.keys(), [(work, true)]);
    assert_eq!(
        world.record(REQUEST_2).map(|record| record.state),
        Some(RequestState::Accepted)
    );
}
