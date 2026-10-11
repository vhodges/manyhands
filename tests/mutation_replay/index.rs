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
fn a_rebuild_with_a_corrupt_database_runs_without_a_record_and_registers_the_repository() {
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

    let rebuilt = world.execute(REQUEST_2, rebuild(&world));
    assert_current(&world, "index rebuild", &rebuilt);
    // It ran under an operation, and no record was written for it.
    let operation = rebuilt.operation_id.clone().expect("an operation");
    assert_eq!(world.request_rows(), 0);
    assert!(world.record(REQUEST_2).is_none());
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
/// be read, so the rebuild runs with none. The domain operation refuses:
/// it finds a database with no tables, which only opening the index anew
/// gives it. The request stopped with nothing known of its operation, so
/// it is recorded as soon as the index can hold a record, and its retry
/// rebuilds.
#[test]
fn a_rebuild_with_the_database_deleted_under_an_open_service_is_finished_by_its_retry() {
    let world = World::new();
    remove_index(&world);
    let refused = world.execute(REQUEST_1, refresh(&world));
    assert_eq!(refused.code, ResultCode::IndexUnavailable);

    let first = world.execute(REQUEST_2, rebuild(&world));
    assert_eq!(
        (first.outcome, first.code),
        (Outcome::Blocked, ResultCode::RecoveryRequired)
    );
    assert_eq!(first.effects, Effects::not_requested());
    let operation = first.operation_id.clone().expect("an operation");
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Accepted
    );

    let retry = world.execute(REQUEST_2, rebuild(&world));
    assert_current(&world, "index rebuild", &retry);
    assert_eq!(retry.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(world.pending_journal_rows(), 0);
    assert_registered_again(&world);
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

/// A rebuild that ran with no record and stopped with its row pending
/// would refuse every later operation, its own retry among them, since
/// the retry would begin another operation. Its record is written when it
/// stops, so the same request finishes it.
#[test]
fn a_rebuild_that_fails_with_no_record_is_recorded_so_that_its_retry_finishes_it() {
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
