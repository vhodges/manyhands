//! Re-entering a request: what `execute` does with a request whose record
//! is `accepted`, because an earlier call of it started and its end was
//! not recorded.

use manyhands::{
    repository::{
        FailurePoint, Mutation, MutationDataDto, RequestState, TicketSaveInput,
        request_store::JournalRow,
    },
    results::{CheckpointEffect, DiscoveryEffect, Envelope, Outcome, ResultCode, WriteEffect},
};

use crate::support::{
    FailOnce,
    items::TICKET_A,
    mutation::{BODY, REQUEST_1, REQUEST_2, TITLE, World, recovery_actions},
};

fn failing_at(point: FailurePoint) -> World {
    World::with_service(|data| FailOnce::at(point).open_service(data))
}

/// A save of `TICKET_A` with another body than `World::save` gives it.
fn earlier_save(world: &World, token: &str) -> Mutation {
    Mutation::TicketSave(TicketSaveInput {
        draft: World::draft("An earlier title", "An earlier body.\n"),
        ..world.save_input(TICKET_A, token)
    })
}

/// Asserts that `envelope` is the result of a save that wrote and
/// committed, that its commit is the newest of `commits` checkpoints on
/// the ticket's branch, and that the request's record is finished with it.
fn assert_finished_with_its_commit(
    world: &World,
    request_id: &str,
    envelope: &Envelope<MutationDataDto>,
    commits: usize,
    what: &str,
) {
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok),
        "{what}"
    );
    assert_eq!(envelope.effects.write, WriteEffect::Written, "{what}");
    assert_eq!(
        envelope.effects.checkpoint,
        CheckpointEffect::Committed,
        "{what}"
    );
    assert_eq!(
        envelope.effects.discovery,
        DiscoveryEffect::Current,
        "{what}"
    );
    assert_eq!(world.branch_commits(TICKET_A), commits, "{what}");
    assert_eq!(
        envelope.effects.commit_oid,
        world.branch_tip(TICKET_A).map(|tip| tip.to_string()),
        "{what}"
    );
    assert!(recovery_actions(envelope).is_empty(), "{what}");
    let source = world.worktree_source(TICKET_A);
    assert!(source.contains(TITLE) && source.contains(BODY), "{what}");

    let record = world.record(request_id).expect("a record");
    assert_eq!(record.state, RequestState::Finished, "{what}");
    let result = record.result.expect("a stored result");
    assert_eq!(result.outcome, Outcome::Success, "{what}");
    assert_eq!(result.effects, envelope.effects, "{what}");
    // The operation the request began is the one that finished.
    assert_eq!(
        Some(record.operations[0].operation_id.to_string()),
        envelope.operation_id,
        "{what}"
    );
    assert!(
        matches!(
            world.journal_row(&envelope.operation_id.clone().unwrap()),
            JournalRow::Final {
                owes_work: false,
                ..
            }
        ),
        "{what}"
    );
    assert_eq!(world.pending_journal_rows(), 0, "{what}");
}

#[test]
fn a_retry_finishes_a_request_that_stopped_after_its_write() {
    let mut world = failing_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Partial);
    assert_eq!(first.effects.checkpoint, CheckpointEffect::Pending);
    assert_eq!(world.branch_commits(TICKET_A), 0);

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &retry, 1, "the retry");
    assert_eq!(retry.operation_id, first.operation_id);
    assert_eq!(world.record(REQUEST_1).unwrap().attempt, 2);

    // And the finished request is then answered from its record.
    let replay = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(replay.effects, retry.effects);
    assert_eq!(world.branch_commits(TICKET_A), 1);
}

#[test]
fn a_retry_finishes_a_request_that_stopped_before_discovery() {
    let mut world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (first.outcome, first.code),
        (Outcome::Partial, ResultCode::DiscoveryPending)
    );
    let commit = first.effects.commit_oid.clone().expect("a commit");

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    // The domain's second call changes nothing and names no commit; the
    // commit is the one the request's first attempt made.
    assert_finished_with_its_commit(&world, REQUEST_1, &retry, 1, "the retry");
    assert_eq!(retry.effects.commit_oid, Some(commit));
}

/// The four points a save can be interrupted at, and whether the file was
/// written by then.
const INTERRUPTIONS: [(FailurePoint, bool); 4] = [
    (FailurePoint::BeforeItemWrite, false),
    (
        FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        true,
    ),
    (FailurePoint::BeforeCheckpointCommit, true),
    (FailurePoint::BeforeIndexTransactionCommit, true),
];

#[test]
fn a_retry_completes_a_save_interrupted_at_each_fault_point() {
    for (point, written) in INTERRUPTIONS {
        // `false`: the ticket's first save, which creates its editing
        // context. `true`: a ticket that has one, which the index knows,
        // so the caller's token is of the file the save rewrites.
        for has_context in [false, true] {
            let what = format!("{point:?}, context {has_context}");
            let mut world = World::new();
            let mut commits = 0;
            if has_context {
                let earlier =
                    world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
                assert_eq!(earlier.outcome, Outcome::Success, "{what}");
                commits = 1;
            }
            world.service = FailOnce::at(point).open_service(world.data.path());
            let token = world.token(TICKET_A);
            let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
            // Nothing was written before `BeforeItemWrite`: the file is
            // not what the request intended, so the result is an error.
            // The domain marked its effect before that point, so its row
            // is pending and the record stays all the same.
            let expected = if written {
                Outcome::Partial
            } else {
                Outcome::Error
            };
            assert_eq!(first.outcome, expected, "{what}");
            assert_eq!(
                world.worktree_source(TICKET_A).contains(TITLE),
                written,
                "{what}"
            );
            let record = world.record(REQUEST_1).expect("the record stays");
            assert_eq!(record.state, RequestState::Accepted, "{what}");
            assert!(
                world
                    .journal_row(&first.operation_id.clone().unwrap())
                    .in_flight(),
                "{what}"
            );
            // The first attempt's own write made the caller's token stale.
            assert_eq!(
                world.token(TICKET_A) != token,
                written && has_context,
                "{what}"
            );

            // A fresh service, as the next process would open, and the
            // same input, token included.
            world.reopen();
            let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
            assert_finished_with_its_commit(&world, REQUEST_1, &retry, commits + 1, &what);
            assert_eq!(retry.operation_id, first.operation_id, "{what}");
        }
    }
}

/// A world whose first accepted request loses its output: the domain call
/// runs, and the result is discarded before the record is settled, as a
/// process that died there would leave it.
fn losing_output() -> World {
    failing_at(FailurePoint::BeforeRequestSettlement)
}

/// Asserts that the request's record is as a dead process leaves it:
/// accepted, with no result, and its operation completed.
fn assert_output_lost(world: &World, request_id: &str, lost: &Envelope<MutationDataDto>) {
    assert_eq!(
        (lost.outcome, lost.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_eq!(lost.effects.commit_oid, None);
    let record = world.record(request_id).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));
    assert_eq!(record.result, None);
    assert!(matches!(
        world.journal_row(&record.operations[0].operation_id.to_string()),
        JournalRow::Final {
            owes_work: false,
            ..
        }
    ));
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_retry_after_lost_output_reports_the_commit_and_makes_none() {
    let mut world = losing_output();
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    assert_eq!(world.branch_commits(TICKET_A), 1, "the save committed");
    let commit = world.branch_tip(TICKET_A).unwrap();

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &retry, 1, "the retry");
    assert_eq!(retry.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(world.branch_tip(TICKET_A), Some(commit), "no second commit");
}

#[test]
fn a_retry_after_lost_output_and_another_save_reports_the_first_commit() {
    let mut world = losing_output();
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    let first_commit = world.branch_tip(TICKET_A).unwrap();

    // Another request saves the ticket with other content.
    world.reopen();
    let second = world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(second.outcome, Outcome::Success);
    let second_commit = world.branch_tip(TICKET_A).unwrap();
    assert_ne!(second_commit, first_commit);
    let saved = world.worktree_source(TICKET_A);
    assert!(saved.contains("An earlier body."));

    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.effects.write, WriteEffect::Written);
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(retry.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(retry.effects.commit_oid, Some(first_commit.to_string()));
    assert!(recovery_actions(&retry).is_empty());
    // The later save is still what the branch and the file hold.
    assert_eq!(world.branch_tip(TICKET_A), Some(second_commit));
    assert_eq!(world.branch_commits(TICKET_A), 2);
    assert_eq!(world.worktree_source(TICKET_A), saved);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.result.unwrap().effects, retry.effects);

    // And again, from the record.
    let replay = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(replay.effects, retry.effects);
    assert_eq!(world.worktree_source(TICKET_A), saved);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_retry_after_the_lost_output_of_a_save_that_changed_nothing_is_a_no_op() {
    // `false`: the ticket's first save, of what its file already holds.
    // `true`: the same content saved twice, so an earlier commit on the
    // branch holds exactly what the request intends.
    for committed_before in [false, true] {
        let what = format!("committed before: {committed_before}");
        let mut world = World::new();
        let mut commits = 0;
        if committed_before {
            let earlier = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
            assert_eq!(earlier.outcome, Outcome::Success, "{what}");
            commits = 1;
        }
        world.service =
            FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
        let token = world.token(TICKET_A);
        let draft = if committed_before {
            World::draft(TITLE, BODY)
        } else {
            World::unchanged_draft()
        };
        let save = |world: &World| {
            Mutation::TicketSave(TicketSaveInput {
                draft: draft.clone(),
                ..world.save_input(TICKET_A, &token)
            })
        };
        let lost = world.execute(REQUEST_1, save(&world));
        assert_output_lost(&world, REQUEST_1, &lost);
        assert_eq!(world.branch_commits(TICKET_A), commits, "{what}");

        world.reopen();
        let retry = world.execute(REQUEST_1, save(&world));
        assert_eq!(
            (retry.outcome, retry.code),
            (Outcome::Noop, ResultCode::AlreadyApplied),
            "{what}"
        );
        assert_eq!(retry.effects.write, WriteEffect::Unchanged, "{what}");
        assert_eq!(
            retry.effects.checkpoint,
            CheckpointEffect::Unchanged,
            "{what}"
        );
        assert_eq!(
            retry.effects.discovery,
            DiscoveryEffect::NotRequested,
            "{what}"
        );
        assert_eq!(retry.effects.commit_oid, None, "{what}");
        assert_eq!(world.branch_commits(TICKET_A), commits, "{what}");
        let record = world.record(REQUEST_1).expect("a record");
        assert_eq!(record.state, RequestState::Finished, "{what}");
        assert_eq!(record.result.unwrap().outcome, Outcome::Noop, "{what}");
    }
}
