//! `index refresh` and `index rebuild`: the first bound commands whose
//! completed journal row has no checkpoint step. A request whose row has
//! completed is answered from the row; a failed one is an error whose
//! record stays accepted while its row is pending.

use std::fs;

use manyhands::{
    repository::{
        FailurePoint, IndexRebuildInput, IndexRefreshInput, Mutation, MutationDataDto,
        RequestState,
        request_store::{FinalKind, JournalRow},
    },
    results::{DiscoveryEffect, Effects, Envelope, FailureClass, Outcome, ResultCode},
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3, World, recovery_actions},
};

type Request = fn(&World) -> Mutation;

fn refresh(world: &World) -> Mutation {
    Mutation::IndexRefresh(IndexRefreshInput {
        root: world.root.clone(),
    })
}

fn rebuild(world: &World) -> Mutation {
    Mutation::IndexRebuild(IndexRebuildInput {
        root: world.root.clone(),
    })
}

const COMMANDS: [(&str, Request, Request); 2] = [
    ("index refresh", refresh, rebuild),
    ("index rebuild", rebuild, refresh),
];

fn discovered() -> Effects {
    Effects {
        discovery: DiscoveryEffect::Current,
        ..Effects::not_requested()
    }
}

/// Asserts that the index was brought up to date and nothing else done.
fn assert_current(world: &World, command: &str, envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.command, command);
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Success, ResultCode::Ok, None),
        "{command}"
    );
    assert_eq!(envelope.effects, discovered(), "{command}");
    assert_eq!(envelope.data, None, "{command}");
    assert!(envelope.recovery.is_empty(), "{command}");
    assert_eq!(
        envelope.scope.repository.as_deref(),
        world.root.to_str(),
        "{command}"
    );
    assert_eq!(
        (
            &envelope.scope.item_id,
            &envelope.scope.branch,
            &envelope.scope.worktree,
            &envelope.scope.remote
        ),
        (&None, &None, &None, &None),
        "{command}"
    );
}

fn operation_rows(world: &World) -> i64 {
    items::index(world.data.path())
        .query_row("SELECT COUNT(*) FROM operation_records", [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// The operation the request's record names.
fn operation_of(world: &World, request_id: &str) -> String {
    world.record(request_id).expect("a record").operations[0]
        .operation_id
        .to_string()
}

#[test]
fn a_refresh_and_a_rebuild_report_discovery_and_are_replayed() {
    for (command, request, other) in COMMANDS {
        let world = World::new();
        let first = world.execute(REQUEST_1, request(&world));
        assert_current(&world, command, &first);
        let record = world.record(REQUEST_1).expect("a record");
        assert_eq!(record.state, RequestState::Finished, "{command}");
        assert_eq!(
            first.operation_id.as_deref(),
            Some(operation_of(&world, REQUEST_1).as_str())
        );
        assert_eq!(world.pending_journal_rows(), 0, "{command}");
        let rows = operation_rows(&world);

        // The same request again is its stored result: nothing runs.
        let replayed = world.execute(REQUEST_1, request(&world));
        assert_eq!(replayed, first, "{command}");
        assert_eq!(operation_rows(&world), rows, "{command}");

        // The same request ID for another command is refused.
        let changed = world.execute(REQUEST_1, other(&world));
        assert_eq!(
            (changed.outcome, changed.code, changed.failure_class()),
            (
                Outcome::Error,
                ResultCode::RequestMismatch,
                Some(FailureClass::Input)
            ),
            "{command}"
        );
        assert_eq!(changed.effects, Effects::not_requested());
        assert_eq!(changed.operation_id, None);
        assert_eq!(operation_rows(&world), rows, "{command}");
        assert_eq!(world.record(REQUEST_1), Some(record));
    }
}

/// A completed row of a refresh or a rebuild never has a checkpoint step.
/// By the authoring reading such a row did nothing and its record is
/// released. Here it is the request's work done.
#[test]
fn a_completed_request_whose_result_was_lost_is_answered_from_its_row_and_not_run_again() {
    for (command, request, _) in COMMANDS {
        let world = World::with_service(|data| {
            FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
        });
        // The domain call ran to its end and the result was lost.
        let lost = world.execute(REQUEST_1, request(&world));
        assert_eq!(lost.code, ResultCode::InternalError, "{command}");
        let record = world.record(REQUEST_1).expect("a record");
        assert_eq!(record.state, RequestState::Accepted, "{command}");
        let operation = operation_of(&world, REQUEST_1);
        assert_eq!(
            world.journal_row(&operation),
            JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: false,
            },
            "{command}"
        );
        let rows = operation_rows(&world);
        let journal = journal_dump(&world);

        let retry = world.execute(REQUEST_1, request(&world));
        assert_current(&world, command, &retry);
        // The record was kept, not released: the same operation, and no
        // operation begun or touched for the retry.
        assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
        assert_eq!(operation_rows(&world), rows, "{command}");
        assert_eq!(journal_dump(&world), journal, "{command}");
        let finished = world.record(REQUEST_1).expect("a record");
        assert_eq!(finished.state, RequestState::Finished, "{command}");
        assert_eq!(finished.attempt, 2, "{command}");
        assert_eq!(operation_of(&world, REQUEST_1), operation);
        let stored = finished.result.expect("a stored result");
        assert_eq!(
            (stored.outcome, stored.code, stored.effects),
            (Outcome::Success, ResultCode::Ok, discovered()),
            "{command}"
        );

        assert_eq!(world.execute(REQUEST_1, request(&world)), retry);
    }
}

/// Every row of the local journal, as it is stored.
fn journal_dump(world: &World) -> Vec<(String, String, Option<String>, i64, i64)> {
    let connection = items::index(world.data.path());
    let mut statement = connection
        .prepare(
            "SELECT operation_ulid, state, completed_step, observed_at, index_owner_epoch
               FROM operation_records ORDER BY id",
        )
        .unwrap();
    statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn a_request_that_never_reached_the_domain_is_run_by_its_retry() {
    for (command, request, _) in COMMANDS {
        let world = World::with_service(|data| {
            FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(data)
        });
        let died = world.execute(REQUEST_1, request(&world));
        assert_eq!(died.code, ResultCode::InternalError, "{command}");
        let operation = operation_of(&world, REQUEST_1);
        assert_eq!(world.journal_row(&operation), JournalRow::Absent);

        let retry = world.execute(REQUEST_1, request(&world));
        assert_current(&world, command, &retry);
        assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
        assert_eq!(
            world.record(REQUEST_1).unwrap().state,
            RequestState::Finished
        );
        assert_eq!(world.pending_journal_rows(), 0, "{command}");
    }
}

/// The stated exception to "a request that did nothing leaves nothing".
#[test]
fn a_failed_refresh_or_rebuild_is_an_error_that_only_its_own_retry_clears() {
    for (command, request, other) in COMMANDS {
        let world = World::with_service(|data| {
            FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
        });
        let token = world.token(TICKET_A);

        let failed = world.execute(REQUEST_1, request(&world));
        // An error with its code: not partial, and not a hand-off owed.
        assert_eq!(
            (failed.outcome, failed.code, failed.failure_class()),
            (
                Outcome::Error,
                ResultCode::InternalError,
                Some(FailureClass::Internal)
            ),
            "{command}"
        );
        assert_eq!(failed.effects, Effects::not_requested(), "{command}");
        assert_eq!(failed.data, None);
        assert_eq!(recovery_actions(&failed), ["request.retry"], "{command}");
        assert_eq!(
            failed.recovery[0].arguments["request_id"], REQUEST_1,
            "{command}"
        );
        // Its record stays accepted while its row is pending.
        let operation = operation_of(&world, REQUEST_1);
        assert_eq!(
            world.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
        assert!(matches!(
            world.journal_row(&operation),
            JournalRow::Pending(_)
        ));
        assert_eq!(world.pending_journal_rows(), 1, "{command}");

        // A different request is refused until it is retried, and leaves
        // nothing: a write, and the other of the two index commands.
        for (request_id, mutation) in [
            (REQUEST_2, world.save(TICKET_A, &token)),
            (REQUEST_3, other(&world)),
            (REQUEST_3, request(&world)),
        ] {
            let refused = world.execute(request_id, mutation);
            assert_eq!(
                (refused.outcome, refused.code, refused.failure_class()),
                (
                    Outcome::Blocked,
                    ResultCode::RecoveryRequired,
                    Some(FailureClass::Blocked)
                ),
                "{command}"
            );
            assert_eq!(refused.effects, Effects::not_requested());
            assert!(world.record(request_id).is_none(), "{command}");
        }
        assert_eq!(world.pending_journal_rows(), 1, "{command}");
        assert!(world.branch_tip(TICKET_A).is_none());

        // The retry completes, under the same operation.
        let retry = world.execute(REQUEST_1, request(&world));
        assert_current(&world, command, &retry);
        assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
        assert_eq!(
            world.record(REQUEST_1).unwrap().state,
            RequestState::Finished
        );
        assert_eq!(world.pending_journal_rows(), 0, "{command}");

        // And the other requests run.
        let saved = world.execute(REQUEST_2, world.save(TICKET_A, &token));
        assert_eq!(saved.outcome, Outcome::Success, "{command}");
        assert_current(
            &world,
            other(&world).command(),
            &world.execute(REQUEST_3, other(&world)),
        );
    }
}

#[test]
fn a_refresh_of_a_repository_that_is_not_registered_is_refused_and_leaves_nothing() {
    let mut world = World::new();
    world.lose_cache();
    let refused = world.execute(REQUEST_1, refresh(&world));
    assert_eq!(
        (refused.outcome, refused.code),
        (Outcome::Blocked, ResultCode::RepositoryNotRegistered)
    );
    assert_eq!(recovery_actions(&refused), ["index.rebuild"]);
    assert_eq!(refused.operation_id, None);
    assert_eq!(world.request_rows(), 0);
    assert_eq!(operation_rows(&world), 0);
}

/// Replaces the index with a file that is not a database, and opens a
/// service over it as the next process would: every record is out of
/// reach, and the service is degraded.
fn corrupt_index(
    world: &mut World,
    open: impl FnOnce(&std::path::Path) -> manyhands::repository::RepositoryService,
) {
    remove_index(world);
    fs::write(world.data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    world.service = open(world.data.path());
}

fn remove_index(world: &World) {
    for entry in fs::read_dir(world.data.path()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
            fs::remove_file(&path).unwrap();
        }
    }
}

fn assert_registered_again(world: &World) {
    assert_eq!(world.resolved().root(), world.root);
    // The repository's items are indexed: a save finds its ticket.
    let saved = world.execute(REQUEST_3, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(saved.outcome, Outcome::Success);
}

#[test]
fn a_rebuild_with_a_corrupt_database_is_recorded_and_registers_the_repository() {
    let mut world = World::new();
    corrupt_index(&mut world, |data| {
        manyhands::repository::RepositoryService::open_at(data).unwrap()
    });

    // Every other request is refused, with the action that brings the
    // index back.
    for mutation in [refresh(&world), world.save(TICKET_A, "v1:any")] {
        let refused = world.execute(REQUEST_1, mutation);
        assert_eq!(
            (refused.outcome, refused.code),
            (Outcome::Blocked, ResultCode::IndexUnavailable)
        );
        assert_eq!(recovery_actions(&refused), ["index.rebuild"]);
        assert_eq!(
            refused.recovery[0].arguments["root"],
            world.root.to_str().unwrap()
        );
        assert_eq!(refused.operation_id, None);
    }

    // Those refusals replaced nothing: the index is still not a database.
    assert_eq!(
        fs::read(world.data.path().join("manyhands.sqlite3")).unwrap(),
        b"not sqlite"
    );

    let rebuilt = world.execute(REQUEST_2, rebuild(&world));
    assert_current(&world, "index rebuild", &rebuilt);
    // The index was brought back first, so the request was recorded
    // before its operation began, as any request is.
    let operation = rebuilt.operation_id.clone().expect("an operation");
    assert_eq!(operation_of(&world, REQUEST_2), operation);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.request_rows(), 1);
    assert_eq!(world.execute(REQUEST_2, rebuild(&world)), rebuilt);
    assert!(matches!(
        world.journal_row(&operation),
        JournalRow::Final {
            kind: FinalKind::Completed,
            ..
        }
    ));
    assert_eq!(world.pending_journal_rows(), 0);
    assert_registered_again(&world);
}

/// The index deleted under a service that is still open. No record can
/// be read, so the index is made anew, and the rebuild is recorded and
/// runs.
#[test]
fn a_rebuild_with_the_database_deleted_under_an_open_service_registers_the_repository() {
    let world = World::new();
    remove_index(&world);
    let refused = world.execute(REQUEST_1, refresh(&world));
    assert_eq!(refused.code, ResultCode::IndexUnavailable);
    assert!(!world.data.path().join("manyhands.sqlite3").exists());

    let rebuilt = world.execute(REQUEST_2, rebuild(&world));
    assert_current(&world, "index rebuild", &rebuilt);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
    assert_registered_again(&world);
}

/// A root a rebuild would refuse brings nothing back: the corrupt index
/// is left as it is.
#[test]
fn a_rebuild_of_what_is_not_a_repository_does_not_replace_a_corrupt_database() {
    let mut world = World::new();
    corrupt_index(&mut world, |data| {
        manyhands::repository::RepositoryService::open_at(data).unwrap()
    });
    let elsewhere = tempfile::tempdir().unwrap();
    let refused = world.execute(
        REQUEST_1,
        Mutation::IndexRebuild(IndexRebuildInput {
            root: elsewhere.path().to_owned(),
        }),
    );
    assert_eq!(
        (refused.outcome, refused.code),
        (Outcome::Error, ResultCode::NotRepository)
    );
    assert_eq!(refused.operation_id, None);
    assert_eq!(
        fs::read(world.data.path().join("manyhands.sqlite3")).unwrap(),
        b"not sqlite"
    );
    // The world's own scan needs a database to read.
    let rebuilt = world.execute(REQUEST_2, rebuild(&world));
    assert_current(&world, "index rebuild", &rebuilt);
}

#[test]
fn a_rebuild_after_the_database_was_deleted_and_made_anew_registers_the_repository() {
    // Deleted, and a new process has opened an empty database since: the
    // repository is not registered, and records can be written again.
    let mut world = World::new();
    world.lose_cache();
    let refused = world.execute(REQUEST_1, world.save(TICKET_A, "v1:any"));
    assert_eq!(refused.code, ResultCode::RepositoryNotRegistered);
    assert_eq!(recovery_actions(&refused), ["index.rebuild"]);

    let rebuilt = world.execute(REQUEST_2, rebuild(&world));
    assert_current(&world, "index rebuild", &rebuilt);
    assert_registered_again(&world);
    // The index was there to hold a record, so the request has one.
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.execute(REQUEST_2, rebuild(&world)), rebuilt);
}

/// A rebuild of a corrupt database that fails after the database was
/// replaced: its record was written before its operation began, so the
/// same request finishes it.
#[test]
fn a_rebuild_of_a_corrupt_database_that_fails_is_finished_by_its_retry() {
    let mut world = World::new();
    corrupt_index(&mut world, |data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });

    let failed = world.execute(REQUEST_1, rebuild(&world));
    assert_eq!(
        (failed.outcome, failed.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_eq!(failed.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&failed), ["request.retry"]);
    let operation = failed.operation_id.clone().expect("an operation");
    assert!(matches!(
        world.journal_row(&operation),
        JournalRow::Pending(_)
    ));
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );
    assert_eq!(operation_of(&world, REQUEST_1), operation);

    let retry = world.execute(REQUEST_1, rebuild(&world));
    assert_current(&world, "index rebuild", &retry);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(world.pending_journal_rows(), 0);
    assert_registered_again(&world);
}

const KILLED_ROOT: &str = "MANYHANDS_KILLED_INDEX_ROOT";
const KILLED_DATA: &str = "MANYHANDS_KILLED_INDEX_DATA";
const KILLED_POINT: &str = "MANYHANDS_KILLED_INDEX_POINT";
const KILLED_REQUEST: &str = "MANYHANDS_KILLED_INDEX_REQUEST";

/// The fault points a test of this file or of `remotes` has a process end
/// at.
const KILL_POINTS: [FailurePoint; 3] = [
    FailurePoint::BeforeIndexTransactionCommit,
    FailurePoint::BeforePublicationConfigurationCommit,
    FailurePoint::AfterIndexClaim,
];

/// The process `killed_at` starts: it submits `REQUEST_1` and ends at the
/// fault point, inside the domain operation, with nothing after it run.
#[test]
fn killed_at_fault_point_child() {
    let Some(root) = std::env::var_os(KILLED_ROOT) else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let name = std::env::var(KILLED_POINT).unwrap();
    let point = KILL_POINTS
        .into_iter()
        .find(|point| format!("{point:?}") == name)
        .expect("a known fault point");
    let service = manyhands::repository::RepositoryService::open_at_with_exit_point_for_testing(
        std::path::Path::new(&std::env::var_os(KILLED_DATA).unwrap()),
        point,
    )
    .unwrap();
    let mutation = match std::env::var(KILLED_REQUEST).unwrap().as_str() {
        "index refresh" => Mutation::IndexRefresh(IndexRefreshInput { root }),
        "index rebuild" => Mutation::IndexRebuild(IndexRebuildInput { root }),
        "remote select origin" => {
            Mutation::RemoteSelect(manyhands::repository::RemoteSelectInput {
                root,
                name: Some("origin".to_owned()),
            })
        }
        other => panic!("{other}: not a request the child makes"),
    };
    crate::support::mutation::execute_with(&service, REQUEST_1, mutation);
    // Not reached: the process ended at the fault point.
}

/// Runs `request` as `REQUEST_1` in a child process that ends at `point`.
pub(super) fn killed_at(world: &World, request: &str, point: FailurePoint) {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "index::killed_at_fault_point_child",
            "--nocapture",
        ])
        .env(KILLED_ROOT, &world.root)
        .env(KILLED_DATA, world.data.path())
        .env(KILLED_POINT, format!("{point:?}"))
        .env(KILLED_REQUEST, request)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(manyhands::repository::FAILURE_POINT_EXIT_STATUS),
        "{request} at {point:?}: the child ended at the fault point"
    );
}

/// A rebuild whose process dies after its journal row was begun leaves
/// that row pending. Only a call under its operation ID takes it up
/// again, and only a record holds that ID: so the request is recorded
/// before its domain call, on a corrupt database too.
#[test]
fn a_rebuild_killed_on_a_corrupt_database_after_its_row_began_is_finished_by_its_retry() {
    let mut world = World::new();
    corrupt_index(&mut world, |data| {
        manyhands::repository::RepositoryService::open_at(data).unwrap()
    });
    killed_at(
        &world,
        "index rebuild",
        FailurePoint::BeforeIndexTransactionCommit,
    );
    world.reopen();

    // What the dead process left: the record, and its row pending.
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));
    let operation = operation_of(&world, REQUEST_1);
    assert!(matches!(
        world.journal_row(&operation),
        JournalRow::Pending(_)
    ));

    // A different request is refused while the row is pending.
    let refused = world.execute(REQUEST_2, rebuild(&world));
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert!(world.record(REQUEST_2).is_none());

    let retry = world.execute(REQUEST_1, rebuild(&world));
    assert_current(&world, "index rebuild", &retry);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(world.pending_journal_rows(), 0);
    // And nothing is refused any longer.
    assert_current(
        &world,
        "index rebuild",
        &world.execute(REQUEST_2, rebuild(&world)),
    );
    assert_registered_again(&world);
}

/// Two calls of one request at once: the second runs the refresh to its
/// end while the first is between its acceptance and its domain call.
/// The first's own call then finds the row completed and is told the
/// index is pending. Its request's work is done, and it says so.
#[test]
fn a_refresh_that_a_call_beside_it_completed_is_answered_as_done() {
    let world = World::new();
    let (root, data) = (world.root.clone(), world.data.path().to_owned());
    world.service.set_request_hook_for_testing(move || {
        let beside = crate::support::mutation::execute_with(
            &manyhands::repository::RepositoryService::open_at(&data).unwrap(),
            REQUEST_1,
            Mutation::IndexRefresh(IndexRefreshInput { root }),
        );
        assert_eq!(beside.code, ResultCode::Ok);
    });
    let first = world.execute(REQUEST_1, refresh(&world));
    assert_current(&world, "index refresh", &first);
    let record = world.record(REQUEST_1).expect("the record is kept");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(world.execute(REQUEST_1, refresh(&world)), first);
}
