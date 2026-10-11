//! `remote add`: a change that no effect shows. Every effect is
//! `not_requested`, `data` names the remote with its location redacted,
//! and the location itself reaches no record and no result.

use manyhands::{
    repository::{
        FailurePoint, Mutation, MutationDataDto, RemoteAddInput, RemoteMutationDto, RequestState,
        request_store::{FinalKind, JournalRow},
    },
    results::{Effects, Envelope, FailureClass, Outcome, ResultCode},
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{self, REQUEST_1, REQUEST_2, REQUEST_3, World, recovery_actions, remote_data},
};

/// What a remote's location can hold and nothing may keep: a password.
const SECRET: &str = "SENTINEL-PASSWORD-3b7c";
const OTHER_SECRET: &str = "SENTINEL-PASSWORD-e94d";

fn location(secret: &str) -> String {
    format!("https://deploy:{secret}@example.invalid/team/project.git")
}

/// The location above as a result gives it.
const REDACTED: &str = "https://example.invalid/team/project.git";

fn add(world: &World, name: &str, url: &str) -> Mutation {
    Mutation::RemoteAdd(RemoteAddInput {
        root: world.root.clone(),
        name: name.to_owned(),
        url: url.to_owned(),
    })
}

/// The location Git holds for the remote, if the repository has it.
fn remote_url(world: &World, name: &str) -> Option<String> {
    git2::Repository::open(&world.root)
        .unwrap()
        .find_remote(name)
        .ok()
        .and_then(|remote| remote.url().map(str::to_owned))
}

fn operation_rows(world: &World) -> i64 {
    items::index(world.data.path())
        .query_row("SELECT COUNT(*) FROM operation_records", [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// Runs a request and checks that the location's secret is in neither
/// its envelope nor any request store.
fn execute(world: &World, request_id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
    let envelope = world.execute(request_id, mutation);
    let text = serde_json::to_string(&envelope).unwrap();
    for secret in [SECRET, OTHER_SECRET] {
        assert!(
            !text.contains(secret),
            "the envelope holds {secret}: {text}"
        );
    }
    mutation::assert_request_stores_exclude(world.data.path(), &[SECRET, OTHER_SECRET]);
    envelope
}

fn assert_names_origin(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        remote_data(envelope),
        &RemoteMutationDto {
            name: Some("origin".to_owned()),
            location: Some(REDACTED.to_owned()),
        }
    );
    assert_eq!(envelope.scope.remote.as_deref(), Some("origin"));
    assert_eq!(
        (
            &envelope.scope.item_id,
            &envelope.scope.branch,
            &envelope.scope.worktree
        ),
        (&None, &None, &None)
    );
    // A remote is no canonical content and no commit.
    assert_eq!(envelope.effects, Effects::not_requested());
}

#[test]
fn a_remote_is_added_and_the_same_again_is_a_no_op() {
    let world = World::new();
    let added = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (added.outcome, added.code, added.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_names_origin(&added);
    assert!(added.recovery.is_empty());
    // Git holds the location as it was given: the scans above are of
    // stores that could have held it.
    assert_eq!(remote_url(&world, "origin"), Some(location(SECRET)));
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.target, "origin");
    assert_eq!(world.pending_journal_rows(), 0);
    let rows = operation_rows(&world);

    // Replay: the stored result, and nothing runs.
    let replayed = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(replayed, added);
    assert_eq!(operation_rows(&world), rows);

    // Changed input under the same request ID: another location, which
    // differs from the first only in what is redacted.
    let changed = execute(
        &world,
        REQUEST_1,
        add(&world, "origin", &location(OTHER_SECRET)),
    );
    assert_eq!(
        (changed.outcome, changed.code, changed.failure_class()),
        (
            Outcome::Error,
            ResultCode::RequestMismatch,
            Some(FailureClass::Input)
        )
    );
    assert_eq!(changed.effects, Effects::not_requested());
    assert_eq!(changed.data, None);
    assert_eq!(operation_rows(&world), rows);
    assert_eq!(remote_url(&world, "origin"), Some(location(SECRET)));

    // The same remote asked for again, as a new request.
    let again = execute(&world, REQUEST_2, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (again.outcome, again.code, again.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_names_origin(&again);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_name_or_a_location_git_cannot_write_is_invalid_remote_and_leaves_nothing() {
    let world = World::new();
    let rows = operation_rows(&world);
    for (name, url) in [
        ("bad name", location(SECRET)),
        ("", location(SECRET)),
        ("with\0nul", location(SECRET)),
        ("refs/heads/..", location(SECRET)),
        ("origin", String::new()),
        ("origin", format!("{}\0", location(SECRET))),
        ("origin", format!("{}\nurl = elsewhere", location(SECRET))),
    ] {
        let refused = execute(&world, REQUEST_1, add(&world, name, &url));
        assert_eq!(
            (refused.outcome, refused.code, refused.failure_class()),
            (
                Outcome::Error,
                ResultCode::InvalidRemote,
                Some(FailureClass::Input)
            ),
            "{name:?} {url:?}"
        );
        assert_eq!(refused.effects, Effects::not_requested());
        assert_eq!(refused.operation_id, None);
        assert!(refused.recovery.is_empty());
        assert!(world.record(REQUEST_1).is_none(), "{name:?}");
        assert_eq!(operation_rows(&world), rows, "{name:?}");
    }
    assert_eq!(remote_url(&world, "origin"), None);

    // The request ID is free for corrected input.
    let added = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(added.outcome, Outcome::Success);
}

#[test]
fn a_name_in_use_with_another_location_is_a_conflict_and_leaves_nothing() {
    let world = World::new();
    let added = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(added.outcome, Outcome::Success);

    let conflict = execute(
        &world,
        REQUEST_2,
        add(&world, "origin", &location(OTHER_SECRET)),
    );
    assert_eq!(
        (conflict.outcome, conflict.code, conflict.failure_class()),
        (
            Outcome::Error,
            ResultCode::RemoteNameConflict,
            Some(FailureClass::Input)
        )
    );
    assert_eq!(conflict.effects, Effects::not_requested());
    assert!(conflict.recovery.is_empty());
    // The data is of the request, not of the remote that is there.
    assert_eq!(remote_data(&conflict).location.as_deref(), Some(REDACTED));
    assert_eq!(remote_url(&world, "origin"), Some(location(SECRET)));
    assert!(world.record(REQUEST_2).is_none());
    assert_eq!(world.pending_journal_rows(), 0);

    // Nothing is left behind: a different request runs at once, and the
    // request ID can be used with corrected input.
    let saved = world.execute(REQUEST_3, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(saved.outcome, Outcome::Success);
    let corrected = execute(
        &world,
        REQUEST_2,
        add(&world, "upstream", &location(OTHER_SECRET)),
    );
    assert_eq!(corrected.outcome, Outcome::Success);
}

#[test]
fn a_remote_added_whose_index_hand_off_failed_is_partial_and_finished_by_its_retry() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let pending = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (pending.outcome, pending.code, pending.failure_class()),
        (
            Outcome::Partial,
            ResultCode::DiscoveryPending,
            Some(FailureClass::Incomplete)
        )
    );
    assert_names_origin(&pending);
    assert_eq!(recovery_actions(&pending), ["operation.resume"]);
    assert_eq!(pending.recovery[0].operation_id, pending.operation_id);
    assert_eq!(remote_url(&world, "origin"), Some(location(SECRET)));
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );
    assert_eq!(world.pending_journal_rows(), 1);

    let retry = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_names_origin(&retry);
    assert_eq!(retry.operation_id, pending.operation_id);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_failure_after_the_remote_was_added_is_partial_though_no_effect_shows_it() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterRemoteMutation).open_service(data)
    });
    let stopped = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (stopped.outcome, stopped.code, stopped.failure_class()),
        (
            Outcome::Partial,
            ResultCode::InternalError,
            Some(FailureClass::Incomplete)
        )
    );
    assert_names_origin(&stopped);
    assert_eq!(recovery_actions(&stopped), ["request.retry"]);
    assert_eq!(remote_url(&world, "origin"), Some(location(SECRET)));
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    // The request ID with another location: input rejected after an
    // earlier attempt's change.
    let changed = execute(
        &world,
        REQUEST_1,
        add(&world, "origin", &location(OTHER_SECRET)),
    );
    assert_eq!(
        (changed.outcome, changed.code),
        (Outcome::Partial, ResultCode::RequestMismatch)
    );
    assert_eq!(changed.effects, Effects::not_requested());
    assert_eq!(changed.operation_id, None);

    let retry = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.operation_id, stopped.operation_id);
    assert_eq!(world.pending_journal_rows(), 0);
}

/// The row of an `add_remote` that ran to its end has no checkpoint step.
/// By the authoring reading it did nothing, and its record would be
/// released and the request run as new.
#[test]
fn a_completed_add_whose_result_was_lost_is_answered_from_its_row_and_not_run_again() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
    });
    let lost = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(remote_url(&world, "origin"), Some(location(SECRET)));
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Accepted);
    let operation = record.operations[0].operation_id.to_string();
    assert_eq!(
        world.journal_row(&operation),
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
            checkpointed: false,
        }
    );
    let rows = operation_rows(&world);

    let retry = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    // The design's answer for a completed `add_remote`: a no-op.
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_names_origin(&retry);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(operation_rows(&world), rows);
    let finished = world.record(REQUEST_1).expect("a record");
    assert_eq!(finished.state, RequestState::Finished);
    assert_eq!(finished.attempt, 2);
    assert_eq!(finished.operations[0].operation_id.to_string(), operation);
    assert_eq!(
        execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET))),
        retry
    );
}

#[test]
fn an_add_to_a_repository_that_is_not_registered_is_refused() {
    let mut world = World::new();
    world.lose_cache();
    let refused = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (refused.outcome, refused.code),
        (Outcome::Blocked, ResultCode::RepositoryNotRegistered)
    );
    assert_eq!(recovery_actions(&refused), ["index.rebuild"]);
    assert_eq!(remote_url(&world, "origin"), None);
    assert_eq!(world.request_rows(), 0);
}
