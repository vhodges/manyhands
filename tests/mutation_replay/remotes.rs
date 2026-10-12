//! `remote add`: a change that no effect shows. Every effect is
//! `not_requested`, `data` names the remote with its location redacted,
//! and the location itself reaches no record and no result.
//!
//! `remote select`: a commit of the configuration file on primary. Its
//! commit is read from Git, never taken from what the domain names.

use manyhands::{
    repository::{
        FailurePoint, Mutation, MutationDataDto, RemoteAddInput, RemoteMutationDto,
        RemoteSelectInput, RepositoryService, RequestState,
        request_store::{FinalKind, JournalRow},
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, FailureClass, Outcome, ResultCode,
        WriteEffect,
    },
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

/// Asserts that `envelope` names the remote `origin` and that the
/// index hand-off, the one effect an added remote has, is `discovery`:
/// a remote is no canonical content and no commit.
fn assert_names_origin(envelope: &Envelope<MutationDataDto>, discovery: DiscoveryEffect) {
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
    assert_eq!(
        envelope.effects,
        Effects {
            discovery,
            ..Effects::not_requested()
        }
    );
}

#[test]
fn a_remote_is_added_and_the_same_again_is_a_no_op() {
    let world = World::new();
    let added = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (added.outcome, added.code, added.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_names_origin(&added, DiscoveryEffect::Current);
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
    assert_names_origin(&again, DiscoveryEffect::NotRequested);
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
    assert_names_origin(&pending, DiscoveryEffect::Pending);
    assert_eq!(
        crate::support::mutation::recovery_before_own_abandon(&pending),
        ["operation.resume"]
    );
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
    assert_names_origin(&retry, DiscoveryEffect::Current);
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
    assert_names_origin(&stopped, DiscoveryEffect::NotRequested);
    assert_eq!(
        crate::support::mutation::recovery_before_own_abandon(&stopped),
        ["request.retry"]
    );
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
/// released and the request run as new. The step it does have says that
/// this operation added the remote, and the retry is given that result.
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
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_names_origin(&retry, DiscoveryEffect::Current);
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

fn select(world: &World, name: Option<&str>) -> Mutation {
    Mutation::RemoteSelect(RemoteSelectInput {
        root: world.root.clone(),
        name: name.map(str::to_owned),
    })
}

/// Adds a remote by hand, as `git remote add` would.
fn git_remote(world: &World, name: &str, url: &str) {
    git2::Repository::open(&world.root)
        .unwrap()
        .remote(name, url)
        .unwrap();
}

/// A world whose repository has a remote that can be published to.
fn world_with_origin(open: impl FnOnce(&std::path::Path) -> RepositoryService) -> World {
    let world = World::with_service(open);
    git_remote(&world, "origin", "git@example.invalid:team/project.git");
    world
}

fn plain(data: &std::path::Path) -> RepositoryService {
    RepositoryService::open_at(data).unwrap()
}

/// The publication remote the committed configuration names at `commit`.
fn selected_at(world: &World, commit: git2::Oid) -> Option<String> {
    let repository = git2::Repository::open(&world.root).unwrap();
    let tree = repository.find_commit(commit).unwrap().tree().unwrap();
    let entry = tree
        .get_path(std::path::Path::new(".manyhands/config.toml"))
        .unwrap();
    let blob = repository.find_blob(entry.id()).unwrap();
    manyhands::canonical::parse_repository_config(std::str::from_utf8(blob.content()).unwrap())
        .unwrap()
        .publication_remote
}

fn parent_of(world: &World, commit: git2::Oid) -> git2::Oid {
    git2::Repository::open(&world.root)
        .unwrap()
        .find_commit(commit)
        .unwrap()
        .parent_id(0)
        .unwrap()
}

/// Asserts that the configuration file in the primary worktree is the
/// one its head commits. The file's status is not asked: the domain
/// commits the configuration without bringing Git's index up to it.
fn assert_configuration_clean(world: &World) {
    let repository = git2::Repository::open(&world.root).unwrap();
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    let entry = head
        .tree()
        .unwrap()
        .get_path(std::path::Path::new(".manyhands/config.toml"))
        .unwrap();
    assert_eq!(
        std::fs::read(world.root.join(".manyhands/config.toml")).unwrap(),
        repository.find_blob(entry.id()).unwrap().content()
    );
}

fn committed(commit: git2::Oid) -> Effects {
    Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        discovery: DiscoveryEffect::Current,
        commit_oid: Some(commit.to_string()),
        ..Effects::not_requested()
    }
}

fn unchanged() -> Effects {
    Effects {
        write: WriteEffect::Unchanged,
        checkpoint: CheckpointEffect::Unchanged,
        ..Effects::not_requested()
    }
}

fn assert_selected(
    world: &World,
    envelope: &Envelope<MutationDataDto>,
    name: Option<&str>,
    commit: git2::Oid,
) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_eq!(envelope.effects, committed(commit));
    assert_eq!(
        remote_data(envelope),
        &RemoteMutationDto {
            name: name.map(str::to_owned),
            location: None,
        }
    );
    assert_eq!(envelope.scope.remote.as_deref(), name);
    assert_eq!(envelope.scope.branch.as_deref(), Some("main"));
    assert_eq!(
        (&envelope.scope.item_id, &envelope.scope.worktree),
        (&None, &None)
    );
    assert!(envelope.recovery.is_empty());
    assert_eq!(selected_at(world, commit).as_deref(), name);
    assert_configuration_clean(world);
}

fn assert_selection_unchanged(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_eq!(envelope.effects, unchanged());
    assert!(envelope.recovery.is_empty());
}

#[test]
fn a_remote_is_selected_with_its_commit_and_selecting_it_again_is_a_no_op() {
    let world = world_with_origin(plain);
    let before = world.primary_head();

    let selected = world.execute(REQUEST_1, select(&world, Some("origin")));
    let commit = world.primary_head();
    assert_ne!(commit, before);
    assert_eq!(parent_of(&world, commit), before);
    assert_selected(&world, &selected, Some("origin"), commit);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);

    // Replay, and changed input under the same request ID.
    assert_eq!(
        world.execute(REQUEST_1, select(&world, Some("origin"))),
        selected
    );
    let changed = world.execute(REQUEST_1, select(&world, None));
    assert_eq!(
        (changed.outcome, changed.code),
        (Outcome::Error, ResultCode::RequestMismatch)
    );
    assert_eq!(changed.effects, Effects::not_requested());
    assert_eq!(world.primary_head(), commit);

    // Selecting the selected remote, as a new request: the domain names
    // the head it found as its commit, and the head is not this
    // request's.
    let again = world.execute(REQUEST_2, select(&world, Some("origin")));
    assert_selection_unchanged(&again);
    assert_eq!(remote_data(&again).name.as_deref(), Some("origin"));
    assert_eq!(world.primary_head(), commit);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );

    // Clearing the selection commits, and clearing it again does not.
    let cleared = world.execute(REQUEST_3, select(&world, None));
    let clearing = world.primary_head();
    assert_eq!(parent_of(&world, clearing), commit);
    assert_selected(&world, &cleared, None, clearing);
    let cleared_again = world.execute("01ARZ3NDEKTSV4RRFFQ69G5FX4", select(&world, None));
    assert_selection_unchanged(&cleared_again);
    assert_eq!(remote_data(&cleared_again).name, None);
    assert_eq!(world.primary_head(), clearing);
    assert_eq!(world.pending_journal_rows(), 0);
}

/// Row 9 of the design's table, for a selection: a request that changes
/// nothing, whose result is lost, is a no-op on its retry too, though an
/// earlier commit holds exactly what it asked for.
#[test]
fn selecting_the_selected_remote_is_a_no_op_on_a_retry_after_a_lost_result() {
    let world = world_with_origin(|data| {
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
    });
    // Another process selects the remote first.
    let first = mutation::execute_with(
        &plain(world.data.path()),
        REQUEST_1,
        select(&world, Some("origin")),
    );
    assert_eq!(first.outcome, Outcome::Success);
    let commit = world.primary_head();

    let lost = world.execute(REQUEST_2, select(&world, Some("origin")));
    assert_eq!(lost.code, ResultCode::InternalError);
    let record = world.record(REQUEST_2).expect("a record");
    assert_eq!(record.state, RequestState::Accepted);
    // Its operation completed without the step that marks a commit.
    assert_eq!(
        world.journal_row(&record.operations[0].operation_id.to_string()),
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
            checkpointed: false,
        }
    );

    let retry = world.execute(REQUEST_2, select(&world, Some("origin")));
    assert_selection_unchanged(&retry);
    assert_eq!(world.primary_head(), commit);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_completed_selection_whose_result_was_lost_reports_its_commit_and_commits_nothing_more() {
    let world = world_with_origin(|data| {
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
    });
    let before = world.primary_head();
    let lost = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(lost.code, ResultCode::InternalError);
    let commit = world.primary_head();
    assert_eq!(parent_of(&world, commit), before);
    let operation = world.record(REQUEST_1).unwrap().operations[0]
        .operation_id
        .to_string();
    let rows = operation_rows(&world);

    let retry = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selected(&world, &retry, Some("origin"), commit);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(world.primary_head(), commit);
    assert_eq!(operation_rows(&world), rows);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn a_failure_after_the_selection_s_commit_is_partial_and_its_retry_reports_that_commit() {
    let world = world_with_origin(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let before = world.primary_head();
    let partial = world.execute(REQUEST_1, select(&world, Some("origin")));
    let commit = world.primary_head();
    assert_eq!(parent_of(&world, commit), before);
    assert_eq!(
        (partial.outcome, partial.code, partial.failure_class()),
        (
            Outcome::Partial,
            ResultCode::DiscoveryPending,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(
        partial.effects,
        Effects {
            discovery: DiscoveryEffect::Pending,
            ..committed(commit)
        }
    );
    assert_eq!(
        crate::support::mutation::recovery_before_own_abandon(&partial),
        ["operation.resume"]
    );
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );
    assert_eq!(world.pending_journal_rows(), 1);

    // The request ID with another input, after that commit.
    let changed = world.execute(REQUEST_1, select(&world, None));
    assert_eq!(
        (changed.outcome, changed.code),
        (Outcome::Partial, ResultCode::RequestMismatch)
    );
    assert_eq!(changed.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(changed.operation_id, None);

    let retry = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selected(&world, &retry, Some("origin"), commit);
    assert_eq!(retry.operation_id, partial.operation_id);
    assert_eq!(world.primary_head(), commit);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_selection_whose_registration_failed_is_partial_and_finished_by_its_retry() {
    let world = world_with_origin(|data| {
        FailOnce::at(FailurePoint::BeforeRegistryWrite).open_service(data)
    });
    let pending = world.execute(REQUEST_1, select(&world, Some("origin")));
    let commit = world.primary_head();
    assert_eq!(
        (pending.outcome, pending.code, pending.failure_class()),
        (
            Outcome::Partial,
            ResultCode::RegistrationPending,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(
        pending.effects,
        Effects {
            discovery: DiscoveryEffect::Pending,
            ..committed(commit)
        }
    );
    assert_eq!(
        crate::support::mutation::recovery_before_own_abandon(&pending),
        ["request.retry"]
    );
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    let retry = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selected(&world, &retry, Some("origin"), commit);
    assert_eq!(world.primary_head(), commit);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_selection_that_failed_before_its_commit_leaves_nothing_and_can_be_made_again() {
    let world = world_with_origin(|data| {
        FailOnce::at(FailurePoint::BeforePublicationConfigurationCommit).open_service(data)
    });
    let before = world.primary_head();
    let failed = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(
        (failed.outcome, failed.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_eq!(failed.effects, Effects::not_requested());
    assert!(failed.recovery.is_empty());
    // The configuration file was put back.
    assert_configuration_clean(&world);
    assert_eq!(world.primary_head(), before);
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.pending_journal_rows(), 0);

    let selected = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selected(&world, &selected, Some("origin"), world.primary_head());
}

/// Where a remote that can be published to is: a selection is given the
/// remote's name, and gives and keeps nothing of its location.
const LOCATION_SENTINEL: &str = "SENTINEL-LOCATION-4c0d";

/// A selection is given a remote's name and reads the remote: its
/// location, and in one that cannot be published to, the password the
/// location holds. Nothing it returns or records holds either: not when
/// it commits, stops with its hand-off owed, is taken up again, changes
/// nothing, clears the selection or is refused.
#[test]
fn nothing_of_a_selection_holds_the_location_it_read() {
    let sentinels = [LOCATION_SENTINEL, SECRET, OTHER_SECRET];
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    git_remote(
        &world,
        "origin",
        &format!("git@example.invalid:team/{LOCATION_SENTINEL}.git"),
    );
    git_remote(
        &world,
        "guarded",
        &format!("ssh://git:{SECRET}@example.invalid/team/project.git"),
    );
    git_remote(&world, "web", &location(OTHER_SECRET));
    // Each really is in what a selection reads, so the scans can fail.
    for (name, sentinel) in [
        ("origin", LOCATION_SENTINEL),
        ("guarded", SECRET),
        ("web", OTHER_SECRET),
    ] {
        assert!(remote_url(&world, name).unwrap().contains(sentinel));
    }
    let scanned = |request_id: &str, name: Option<&str>| {
        let envelope = world.execute(request_id, select(&world, name));
        let text = serde_json::to_string(&envelope).unwrap();
        for sentinel in sentinels {
            assert!(
                !text.contains(sentinel),
                "the envelope holds {sentinel}: {text}"
            );
        }
        mutation::assert_request_stores_exclude(world.data.path(), &sentinels);
        envelope
    };

    let pending = scanned(REQUEST_1, Some("origin"));
    assert_eq!(
        (pending.outcome, pending.code),
        (Outcome::Partial, ResultCode::DiscoveryPending)
    );
    let selected = scanned(REQUEST_1, Some("origin"));
    assert_eq!(
        (selected.outcome, selected.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(scanned(REQUEST_1, Some("origin")), selected);
    assert_selection_unchanged(&scanned(REQUEST_2, Some("origin")));
    // A remote that cannot be published to is read and refused.
    for name in ["guarded", "web"] {
        let refused = scanned(REQUEST_3, Some(name));
        assert_eq!(refused.code, ResultCode::InvalidRemote, "{name}");
    }
    let cleared = scanned(REQUEST_3, None);
    assert_eq!(
        (cleared.outcome, cleared.code),
        (Outcome::Success, ResultCode::Ok)
    );
    // The scans read the records of the three requests that were
    // accepted, and the rows of their operations.
    assert_eq!(world.request_rows(), 3);
    assert!(operation_rows(&world) >= 3);
}

#[test]
fn a_selection_that_is_refused_leaves_nothing() {
    let world = world_with_origin(plain);
    git_remote(&world, "web", "https://example.invalid/team/project.git");
    let before = world.primary_head();
    let token = world.token(TICKET_A);

    for (name, code) in [
        // No such remote, a remote that is not SSH, and a name Git does
        // not take.
        ("absent", ResultCode::InvalidRemote),
        ("web", ResultCode::InvalidRemote),
        ("not a name", ResultCode::InvalidRemote),
    ] {
        let refused = world.execute(REQUEST_1, select(&world, Some(name)));
        assert_eq!(
            (refused.outcome, refused.code, refused.failure_class()),
            (Outcome::Error, code, Some(FailureClass::Input)),
            "{name}"
        );
        assert_eq!(refused.effects, Effects::not_requested(), "{name}");
        assert!(world.record(REQUEST_1).is_none(), "{name}");
        assert_eq!(world.pending_journal_rows(), 0, "{name}");
        assert_eq!(world.primary_head(), before, "{name}");
        assert_configuration_clean(&world);
    }

    // A different request runs at once, and the request ID is free.
    let saved = world.execute(REQUEST_2, world.save(TICKET_A, &token));
    assert_eq!(saved.outcome, Outcome::Success);
    let selected = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selected(&world, &selected, Some("origin"), world.primary_head());
}

const UNIDENTIFIED_HOME: &str = "MANYHANDS_UNIDENTIFIED_HOME";

/// Runs the calling test, `name`, in a process of its own in which no
/// identity is found outside the repository: where libgit2 looks for
/// configuration is process-wide. `false` in the parent, once the child
/// has passed; `true` in the child, which goes on with the test.
fn in_a_process_without_an_identity(name: &str) -> bool {
    let Some(home) = std::env::var_os(UNIDENTIFIED_HOME) else {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("remotes::{name}"), "--nocapture"])
            .env(UNIDENTIFIED_HOME, home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "the child failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "the child ran no test"
        );
        return false;
    };
    for level in [
        git2::ConfigLevel::System,
        git2::ConfigLevel::Global,
        git2::ConfigLevel::XDG,
        git2::ConfigLevel::ProgramData,
    ] {
        // This exact-test child runs nothing else, so no other use of
        // libgit2 overlaps the change.
        unsafe { git2::opts::set_search_path(level, &home).unwrap() };
    }
    true
}

#[test]
fn a_selection_with_no_identity_to_commit_as_is_identity_required_and_leaves_nothing() {
    if !in_a_process_without_an_identity(
        "a_selection_with_no_identity_to_commit_as_is_identity_required_and_leaves_nothing",
    ) {
        return;
    }
    let world = world_with_origin(plain);
    let before = world.primary_head();
    let mut local = git2::Repository::open(&world.root)
        .unwrap()
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    local.remove("user.name").unwrap();
    local.remove("user.email").unwrap();

    let refused = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(
        (refused.outcome, refused.code, refused.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::IdentityRequired,
            Some(FailureClass::Blocked)
        )
    );
    assert_eq!(refused.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&refused), ["repo.identity_set"]);
    assert_eq!(remote_data(&refused).name.as_deref(), Some("origin"));
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(world.primary_head(), before);
    assert_configuration_clean(&world);

    // With an identity, the same request ID selects the remote.
    world.set_local_config("user.name", "Manyhands Test");
    world.set_local_config("user.email", "manyhands-test@example.invalid");
    let selected = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selected(&world, &selected, Some("origin"), world.primary_head());
}

/// A stale lock on the repository's Git configuration, as a Git process
/// that died leaves one: Git refuses to write a remote while it is there.
fn lock_git_configuration(world: &World) -> std::path::PathBuf {
    let lock = world.root.join(".git/config.lock");
    std::fs::write(&lock, b"").unwrap();
    lock
}

/// An attempt that stopped inside Git's write, with its row pending and
/// nothing written. Returns the request's operation ID.
fn stopped_inside_git(world: &World) -> String {
    let lock = lock_git_configuration(world);
    let stopped = execute(world, REQUEST_1, add(world, "origin", &location(SECRET)));
    std::fs::remove_file(lock).unwrap();
    assert_eq!(
        (stopped.outcome, stopped.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    // Git's own message names the lock. The result holds the code's
    // fixed message and nothing of the error's.
    let text = serde_json::to_string(&stopped).unwrap();
    assert!(!text.contains("config.lock"), "{text}");
    assert_eq!(stopped.message, ResultCode::InternalError.message());
    assert_eq!(stopped.effects, Effects::not_requested());
    assert_eq!(
        crate::support::mutation::recovery_before_own_abandon(&stopped),
        ["request.retry"]
    );
    assert_eq!(remote_url(world, "origin"), None);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Accepted);
    let operation = record.operations[0].operation_id.to_string();
    assert!(matches!(
        world.journal_row(&operation),
        JournalRow::Pending(_)
    ));
    operation
}

/// `add_remote` completes a row an earlier attempt began when it refuses
/// the request for a name conflict. Such a row has completed and the
/// remote is not the request's.
#[test]
fn a_completed_row_of_an_add_refused_for_a_conflict_is_not_taken_for_the_remote_in_place() {
    let mut world = World::new();
    let operation = stopped_inside_git(&world);

    // Someone gives the name another location, and the retry's refusal
    // is lost.
    git_remote(&world, "origin", "git@example.invalid:team/other.git");
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
    let lost = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(lost.code, ResultCode::InternalError);
    assert_eq!(
        world.journal_row(&operation),
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
            checkpointed: false,
        }
    );
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    world.reopen();
    let retry = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (
            Outcome::Error,
            ResultCode::RemoteNameConflict,
            Some(FailureClass::Input)
        )
    );
    assert_eq!(retry.effects, Effects::not_requested());
    assert_eq!(
        remote_url(&world, "origin").as_deref(),
        Some("git@example.invalid:team/other.git")
    );
    // Nothing is left: the request ID is free for corrected input.
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.pending_journal_rows(), 0);
    let corrected = execute(
        &world,
        REQUEST_1,
        add(&world, "upstream", &location(SECRET)),
    );
    assert_eq!(corrected.outcome, Outcome::Success);
}

/// The same row, with the remote someone added being the one asked for:
/// the request's end state holds and no commit or change is its own.
#[test]
fn a_completed_row_of_an_add_that_found_the_remote_in_place_is_a_no_op() {
    let mut world = World::new();
    let operation = stopped_inside_git(&world);
    git_remote(&world, "origin", &location(SECRET));
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
    let lost = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(lost.code, ResultCode::InternalError);
    assert!(matches!(
        world.journal_row(&operation),
        JournalRow::Final { .. }
    ));

    world.reopen();
    let rows = operation_rows(&world);
    let retry = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_names_origin(&retry, DiscoveryEffect::NotRequested);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(operation_rows(&world), rows);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
}

/// The same row once more, with the conflicting remote gone again: no
/// remote has the name, the request is not done, and the domain
/// operation is called under the request's operation.
#[test]
fn a_completed_row_of_an_add_whose_remote_is_absent_is_run() {
    let mut world = World::new();
    let operation = stopped_inside_git(&world);
    git_remote(&world, "origin", "git@example.invalid:team/other.git");
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
    let lost = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(lost.code, ResultCode::InternalError);
    git2::Repository::open(&world.root)
        .unwrap()
        .remote_delete("origin")
        .unwrap();

    world.reopen();
    let retry = execute(&world, REQUEST_1, add(&world, "origin", &location(SECRET)));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_names_origin(&retry, DiscoveryEffect::Current);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(remote_url(&world, "origin"), Some(location(SECRET)));
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn an_add_or_a_selection_that_died_before_its_domain_call_is_run_by_its_retry() {
    let world = world_with_origin(|data| {
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(data)
    });
    let died = execute(
        &world,
        REQUEST_1,
        add(&world, "upstream", &location(SECRET)),
    );
    assert_eq!(died.code, ResultCode::InternalError);
    let record = world.record(REQUEST_1).expect("a record");
    let operation = record.operations[0].operation_id.to_string();
    assert_eq!(world.journal_row(&operation), JournalRow::Absent);
    assert_eq!(remote_url(&world, "upstream"), None);

    let retry = execute(
        &world,
        REQUEST_1,
        add(&world, "upstream", &location(SECRET)),
    );
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(remote_url(&world, "upstream"), Some(location(SECRET)));
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );

    let mut world = world;
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(world.data.path());
    let before = world.primary_head();
    let died = world.execute(REQUEST_2, select(&world, Some("origin")));
    assert_eq!(died.code, ResultCode::InternalError);
    let operation = world.record(REQUEST_2).unwrap().operations[0]
        .operation_id
        .to_string();
    assert_eq!(world.journal_row(&operation), JournalRow::Absent);
    assert_eq!(world.primary_head(), before);

    let retry = world.execute(REQUEST_2, select(&world, Some("origin")));
    let commit = world.primary_head();
    assert_eq!(parent_of(&world, commit), before);
    assert_selected(&world, &retry, Some("origin"), commit);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

const CONFIG: &str = ".manyhands/config.toml";

/// Commits the configuration on primary by hand, with `remote` as its
/// publication remote and a key the library does not know, in the form
/// the library writes a configuration in.
fn commit_configuration_by_hand(world: &World, remote: Option<&str>) -> git2::Oid {
    let source = std::fs::read_to_string(world.root.join(CONFIG)).unwrap();
    let mut config = manyhands::canonical::parse_repository_config(&source).unwrap();
    config.publication_remote = remote.map(str::to_owned);
    config
        .unknown
        .insert("note".to_owned(), toml::Value::String("by hand".to_owned()));
    world.commit_on_primary(
        CONFIG,
        &manyhands::canonical::serialize_repository_config(&config).unwrap(),
    )
}

/// A selection that committed and whose index hand-off failed: its row
/// is pending. Returns the world and the selection's commit.
fn selection_in_flight() -> (World, git2::Oid) {
    let world = world_with_origin(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    git_remote(&world, "other", "git@example.invalid:team/other.git");
    let partial = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(partial.code, ResultCode::DiscoveryPending);
    let own = world.primary_head();
    assert_eq!(partial.effects.commit_oid, Some(own.to_string()));
    (world, own)
}

/// "As intended" for a selection is that the configuration names the
/// remote, whatever else it holds. A commit from elsewhere that keeps the
/// remote does not stand in the request's way, and is not its commit.
#[test]
fn a_foreign_commit_that_keeps_the_selected_remote_does_not_stop_a_selection_in_flight() {
    let (world, own) = selection_in_flight();
    let foreign = commit_configuration_by_hand(&world, Some("origin"));
    assert_ne!(foreign, own);

    let retry = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.effects, committed(own));
    // Nothing more was committed, and the other's change is in place.
    assert_eq!(world.primary_head(), foreign);
    assert!(
        std::fs::read_to_string(world.root.join(CONFIG))
            .unwrap()
            .contains("by hand")
    );
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

/// Open case 1, for a selection: someone else selects another remote
/// while the request's row is pending. The request is refused with its
/// own commit reported, and its record and its row stay.
#[test]
fn a_foreign_commit_that_selects_another_remote_is_an_external_change_to_a_selection_in_flight() {
    let (world, own) = selection_in_flight();
    let foreign = commit_configuration_by_hand(&world, Some("other"));

    let retry = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (
            Outcome::Partial,
            ResultCode::ExternalChange,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(
        retry.effects,
        Effects {
            discovery: DiscoveryEffect::Pending,
            ..committed(own)
        }
    );
    assert!(crate::support::mutation::recovery_before_own_abandon(&retry).is_empty());
    // Nothing was written over the other's selection.
    assert_eq!(world.primary_head(), foreign);
    assert_eq!(selected_at(&world, foreign).as_deref(), Some("other"));
    assert_configuration_clean(&world);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );
    assert_eq!(world.pending_journal_rows(), 1);
}

/// Open case 2 of the design's journal section, pinned as the limit it
/// is: a process killed after `remote select` wrote the configuration
/// file and before it committed leaves the file changed and uncommitted.
/// The retry is refused for it with no effects, the domain completes the
/// row, and the record is deleted. Every later selection is refused the
/// same way until the file is committed or restored by hand.
#[test]
fn a_selection_killed_between_its_write_and_its_commit_leaves_the_worktree_dirty() {
    let mut world = world_with_origin(plain);
    let before = world.primary_head();
    let committed_source = std::fs::read_to_string(world.root.join(CONFIG)).unwrap();
    super::index::killed_at(
        &world,
        "remote select origin",
        FailurePoint::BeforePublicationConfigurationCommit,
    );
    world.reopen();

    // What the dead process left.
    assert_eq!(world.primary_head(), before);
    let written = std::fs::read_to_string(world.root.join(CONFIG)).unwrap();
    assert_ne!(written, committed_source);
    assert!(written.contains("origin"));
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!(record.state, RequestState::Accepted);
    let operation = record.operations[0].operation_id.to_string();
    assert!(matches!(
        world.journal_row(&operation),
        JournalRow::Pending(_)
    ));

    for request_id in [REQUEST_1, REQUEST_2] {
        let refused = world.execute(request_id, select(&world, Some("origin")));
        assert_eq!(
            (refused.outcome, refused.code, refused.failure_class()),
            (
                Outcome::Blocked,
                ResultCode::WorktreeNotClean,
                Some(FailureClass::Blocked)
            ),
            "{request_id}"
        );
        assert_eq!(refused.effects, Effects::not_requested(), "{request_id}");
        assert!(refused.recovery.is_empty(), "{request_id}");
        assert!(world.record(request_id).is_none(), "{request_id}");
        assert_eq!(world.pending_journal_rows(), 0, "{request_id}");
    }
    assert_eq!(world.primary_head(), before);
    assert_eq!(
        std::fs::read_to_string(world.root.join(CONFIG)).unwrap(),
        written
    );

    // Restored by hand, the selection can be made.
    std::fs::write(world.root.join(CONFIG), committed_source).unwrap();
    let selected = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selected(&world, &selected, Some("origin"), world.primary_head());
}

/// The domain names the head it found as a selection's commit whenever
/// the configuration equals the request. A commit of the same selection
/// that someone made after the request was accepted is in the request's
/// range and changed its path, and is still not the request's.
#[test]
fn a_selection_another_request_committed_first_is_not_reported_as_this_request_s_commit() {
    let world = world_with_origin(plain);
    let (root, data) = (world.root.clone(), world.data.path().to_owned());
    // Between this request's acceptance and its domain call.
    world.service.set_request_hook_for_testing(move || {
        let other = mutation::execute_with(
            &plain(&data),
            REQUEST_2,
            Mutation::RemoteSelect(RemoteSelectInput {
                root,
                name: Some("origin".to_owned()),
            }),
        );
        assert_eq!(other.outcome, Outcome::Success);
    });
    let late = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_selection_unchanged(&late);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.result.unwrap().effects.commit_oid, None);
    assert_eq!(world.pending_journal_rows(), 0);
}

/// The review's I1, for a selection. It dies between its commit and the
/// step that records it; its own request takes it up, finds the
/// configuration as it intended, and its index hand-off fails. The row
/// owes its hand-off and holds no commit step, so a resume leaves it to
/// its request: the result offers that request, which finishes it.
#[test]
fn a_selection_whose_hand_off_is_owed_without_its_commit_step_offers_the_request() {
    let mut world = world_with_origin(plain);
    // The step that records the commit cannot be written.
    let index = items::index(world.data.path());
    index
        .execute_batch(
            "CREATE TRIGGER lose_the_step BEFORE UPDATE ON operation_records
              WHEN NEW.state = 'publication_committed'
             BEGIN SELECT RAISE(ABORT, 'fixed failure'); END",
        )
        .unwrap();
    let cut_off = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert!(cut_off.code.failure_class().is_some());
    let commit = world.primary_head();
    assert_eq!(selected_at(&world, commit).as_deref(), Some("origin"));
    index.execute_batch("DROP TRIGGER lose_the_step").unwrap();
    drop(index);

    world.service =
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(world.data.path());
    let owed = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(owed.code, ResultCode::DiscoveryPending);
    let operation_id = owed
        .operation_id
        .clone()
        .expect("the selection's operation");
    let step: Option<String> = items::index(world.data.path())
        .query_row(
            "SELECT completed_step FROM operation_records WHERE operation_ulid = ?1",
            [&operation_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_ne!(step.as_deref(), Some("publication_committed"));

    assert_eq!(
        crate::support::mutation::recovery_before_own_abandon(&owed),
        ["request.retry"]
    );
    assert_eq!(owed.recovery[0].arguments["request_id"], REQUEST_1);
    let refused = world.execute(
        REQUEST_2,
        Mutation::OperationResume(manyhands::repository::OperationResumeInput {
            root: world.root.clone(),
            operation_id: manyhands::repository::OperationId::parse(&operation_id).unwrap(),
        }),
    );
    assert_eq!(refused.code, ResultCode::OriginalRequestRequired);

    // The offered action, taken, succeeds, and makes no second commit.
    let finished = world.execute(REQUEST_1, select(&world, Some("origin")));
    assert_eq!(finished.outcome, Outcome::Success);
    assert_eq!(world.primary_head(), commit);
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
}
