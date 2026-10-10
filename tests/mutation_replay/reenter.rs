//! Re-entering a request: what `execute` does with a request whose record
//! is `accepted`, because an earlier call of it started and its end was
//! not recorded.

use manyhands::{
    canonical::short_code,
    repository::{
        FailurePoint, Mutation, MutationDataDto, RequestState, TicketSaveInput,
        request_store::JournalRow,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, FailureClass, Outcome, ResultCode,
        WriteEffect,
    },
};

use crate::support::{
    FailOnce,
    items::TICKET_A,
    mutation::{
        BODY, REQUEST_1, REQUEST_2, REQUEST_3, TICKET_NEW, TITLE, World, recovery_actions,
        ticket_data,
    },
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

/// A path no request touches.
const UNRELATED: &str = "notes/unrelated.md";

#[test]
fn unrelated_commits_do_not_stop_a_retry() {
    // An item with no editing context: the first save is interrupted
    // before the context exists, and then once it does, and someone
    // commits to primary in between.
    for point in [
        FailurePoint::BeforeContextBranchCreation,
        FailurePoint::BeforeItemWrite,
    ] {
        let what = format!("no context, {point:?}");
        let mut world = failing_at(point);
        let token = world.token(TICKET_A);
        let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_eq!(first.outcome, Outcome::Error, "{what}");
        assert_eq!(
            world.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Accepted),
            "{what}"
        );
        world.commit_on_primary(UNRELATED, "A developer's commit.\n");

        world.reopen();
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_eq!(
            (retry.outcome, retry.code),
            (Outcome::Success, ResultCode::Ok),
            "{what}"
        );
        assert_eq!(
            retry.effects.commit_oid,
            world.branch_tip(TICKET_A).map(|tip| tip.to_string()),
            "{what}"
        );
        assert_eq!(world.branch_commits(TICKET_A), 1, "{what}");
        assert!(world.worktree_source(TICKET_A).contains(TITLE), "{what}");
        assert_eq!(
            world.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Finished),
            "{what}"
        );
        assert_eq!(world.pending_journal_rows(), 0, "{what}");
    }

    // An item with an editing context: someone commits another path to
    // its branch between the interruption and the retry.
    for (point, _) in INTERRUPTIONS {
        let what = format!("context, {point:?}");
        let mut world = World::new();
        let earlier = world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
        assert_eq!(earlier.outcome, Outcome::Success, "{what}");
        world.service = FailOnce::at(point).open_service(world.data.path());
        let token = world.token(TICKET_A);
        let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_ne!(first.outcome, Outcome::Success, "{what}");
        let unrelated = world.commit_in_context(TICKET_A, UNRELATED, "A comment, say.\n");

        world.reopen();
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_eq!(
            (retry.outcome, retry.code),
            (Outcome::Success, ResultCode::Ok),
            "{what}"
        );
        // One commit of the request's, whichever side of the unrelated
        // commit it is on, and it is the one reported.
        assert_eq!(world.branch_commits(TICKET_A), 3, "{what}");
        let reported = retry.effects.commit_oid.clone().expect("a commit");
        assert_ne!(reported, unrelated.to_string(), "{what}");
        assert_ne!(Some(reported), earlier.effects.commit_oid, "{what}");
        assert!(world.worktree_source(TICKET_A).contains(TITLE), "{what}");
        assert_eq!(
            world.record(REQUEST_1).map(|record| record.state),
            Some(RequestState::Finished),
            "{what}"
        );
        assert_eq!(world.pending_journal_rows(), 0, "{what}");
    }
}

/// The ticket's file as someone else would leave it: not what any request
/// here intends.
fn foreign_source() -> String {
    crate::support::items::ticket_source(TICKET_A, "Changed by someone else", "")
}

/// Asserts that `envelope` is `external_change` with nothing done, and
/// that it offers no action: the caller reads the item again and submits
/// a new request.
fn assert_external_change(envelope: &Envelope<MutationDataDto>, what: &str) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::ExternalChange,
            Some(FailureClass::Blocked)
        ),
        "{what}"
    );
    assert_eq!(envelope.effects, Effects::not_requested(), "{what}");
    assert!(recovery_actions(envelope).is_empty(), "{what}");
}

#[test]
fn a_foreign_commit_to_the_file_of_a_request_in_flight_is_an_external_change() {
    // Whether the first attempt died before it wrote or after, and
    // whether the foreign commit was made in the worktree, which changes
    // the file there too, or behind it, which leaves the worktree's file
    // as the first attempt left it. In the second case the domain would
    // find its file as expected, write, and commit over the foreign
    // change: only the evidence check stands in the way.
    for (point, _) in &INTERRUPTIONS[..3] {
        for behind in [false, true] {
            let what = format!("{point:?}, behind the worktree: {behind}");
            let mut world = failing_at(*point);
            let token = world.token(TICKET_A);
            let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
            let operation_id = first.operation_id.clone().unwrap();
            assert!(world.journal_row(&operation_id).in_flight(), "{what}");
            let path = crate::support::items::ticket_path(TICKET_A);
            let directory = world.worktree(TICKET_A);
            let foreign = if behind {
                World::commit_behind(&directory, &path, &foreign_source())
            } else {
                World::commit_in(&directory, &path, &foreign_source())
            };
            let left = world.worktree_source(TICKET_A);

            world.reopen();
            let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
            assert_external_change(&retry, &what);
            // Nothing is written and nothing is run.
            assert_eq!(world.branch_tip(TICKET_A), Some(foreign), "{what}");
            assert_eq!(world.worktree_source(TICKET_A), left, "{what}");
            // The operation is still in flight, so the record stays: this
            // is the journal's open case, and only abandoning the
            // operation clears it.
            assert!(world.journal_row(&operation_id).in_flight(), "{what}");
            let record = world.record(REQUEST_1).expect("the record stays");
            assert_eq!(
                (record.state, record.attempt),
                (RequestState::Accepted, 2),
                "{what}"
            );
            // The same answer again, and still nothing written.
            let again = world.execute(REQUEST_1, world.save(TICKET_A, &token));
            assert_external_change(&again, &what);
            assert_eq!(world.branch_tip(TICKET_A), Some(foreign), "{what}");
            assert_eq!(world.worktree_source(TICKET_A), left, "{what}");
        }
    }
}

#[test]
fn a_foreign_commit_to_the_file_of_a_request_that_never_ran_frees_the_request() {
    // The request was accepted and its process died before the domain
    // call: no journal row exists.
    // `behind`: the foreign commit leaves the worktree as it is, so the
    // domain would find the file it expects and save over the commit.
    for (has_context, behind) in [(true, false), (true, true), (false, false)] {
        let what = format!("context {has_context}, behind the worktree: {behind}");
        let mut world = World::new();
        if has_context {
            let earlier = world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
            assert_eq!(earlier.outcome, Outcome::Success, "{what}");
        }
        world.service =
            FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(world.data.path());
        let token = world.token(TICKET_A);
        let died = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_eq!(died.code, ResultCode::InternalError, "{what}");
        let record = world.record(REQUEST_1).expect("the record stays");
        assert_eq!(record.state, RequestState::Accepted, "{what}");
        let operation_id = record.operations[0].operation_id.to_string();
        assert_eq!(world.journal_row(&operation_id), JournalRow::Absent);

        // Someone else commits other content to the ticket's file: in its
        // context when it has one, on primary when it has none.
        let path = crate::support::items::ticket_path(TICKET_A);
        let foreign = match (has_context, behind) {
            (true, false) => world.commit_in_context(TICKET_A, &path, &foreign_source()),
            (true, true) => {
                World::commit_behind(&world.worktree(TICKET_A), &path, &foreign_source())
            }
            (false, _) => world.commit_on_primary(&path, &foreign_source()),
        };

        world.reopen();
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_external_change(&retry, &what);
        // Nothing is in flight, so nothing is left of the request.
        assert!(world.record(REQUEST_1).is_none(), "{what}");
        assert_eq!(world.pending_journal_rows(), 0, "{what}");
        assert_eq!(world.journal_row(&operation_id), JournalRow::Absent);
        if has_context {
            assert_eq!(world.branch_tip(TICKET_A), Some(foreign), "{what}");
        } else {
            assert_eq!(world.primary_head(), foreign, "{what}");
            assert_eq!(world.branch_commits(TICKET_A), 0, "{what}");
        }

        // The caller reads the item again and saves with a new request.
        if !has_context {
            crate::support::items::refresh_completely(&world.service, &world.root);
        }
        let fresh = world.execute(REQUEST_3, world.save(TICKET_A, &world.token(TICKET_A)));
        assert_eq!(
            (fresh.outcome, fresh.code),
            (Outcome::Success, ResultCode::Ok),
            "{what}"
        );
    }
}

#[test]
fn a_foreign_commit_over_the_commit_of_a_request_in_flight_reports_that_commit() {
    // The first attempt committed and stopped before its index hand-off:
    // its row is pending. Someone then commits other content on top.
    let mut world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.code, ResultCode::DiscoveryPending);
    let own = first.effects.commit_oid.clone().expect("a commit");
    let foreign = world.commit_in_context(
        TICKET_A,
        &crate::support::items::ticket_path(TICKET_A),
        &foreign_source(),
    );

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    // The request is not run again; what it did is still reported.
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (
            Outcome::Partial,
            ResultCode::ExternalChange,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(retry.effects.write, WriteEffect::Written);
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(retry.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(retry.effects.commit_oid, Some(own));
    assert!(recovery_actions(&retry).is_empty());
    assert_eq!(world.branch_tip(TICKET_A), Some(foreign));
    assert_eq!(world.worktree_source(TICKET_A), foreign_source());
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!(record.state, RequestState::Accepted);
    assert!(
        world
            .journal_row(&first.operation_id.clone().unwrap())
            .in_flight()
    );
}

#[test]
fn identical_content_from_another_request_is_already_applied_and_not_this_request_s_commit() {
    for has_context in [false, true] {
        let what = format!("context {has_context}");
        let mut world = World::new();
        let mut commits = 0;
        if has_context {
            let earlier = world.execute(REQUEST_3, earlier_save(&world, &world.token(TICKET_A)));
            assert_eq!(earlier.outcome, Outcome::Success, "{what}");
            commits = 1;
        }
        // The first request is accepted and never reaches the domain.
        world.service =
            FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(world.data.path());
        let token = world.token(TICKET_A);
        let died = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_eq!(died.code, ResultCode::InternalError, "{what}");
        assert_eq!(world.branch_commits(TICKET_A), commits, "{what}");

        // A second request saves the same content.
        world.reopen();
        let second = world.execute(REQUEST_2, world.save(TICKET_A, &token));
        assert_eq!(second.outcome, Outcome::Success, "{what}");
        let commit = second.effects.commit_oid.clone().expect("a commit");
        assert_eq!(world.branch_commits(TICKET_A), commits + 1, "{what}");

        // The first, retried, asks for the state that already exists. The
        // commit that holds it is in range and is not this request's.
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
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
        assert_eq!(retry.effects.commit_oid, None, "{what}");
        assert_eq!(world.branch_commits(TICKET_A), commits + 1, "{what}");
        assert_eq!(
            world.branch_tip(TICKET_A).map(|tip| tip.to_string()),
            Some(commit),
            "{what}"
        );
        let record = world.record(REQUEST_1).expect("a record");
        assert_eq!(record.state, RequestState::Finished, "{what}");
        assert_eq!(record.result.unwrap().outcome, Outcome::Noop, "{what}");
        assert_eq!(world.pending_journal_rows(), 0, "{what}");
    }
}

#[test]
fn intended_content_that_is_not_committed_is_an_external_change_for_a_request_that_never_ran() {
    let mut world = World::new();
    let earlier = world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(earlier.outcome, Outcome::Success);
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(world.data.path());
    let token = world.token(TICKET_A);
    let died = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(died.code, ResultCode::InternalError);

    // Someone edits the file to exactly what the request intends and does
    // not commit it.
    let edited = world
        .worktree_source(TICKET_A)
        .replace("An earlier title", TITLE)
        .replace("An earlier body.\n", BODY);
    std::fs::write(
        world
            .worktree(TICKET_A)
            .join(crate::support::items::ticket_path(TICKET_A)),
        &edited,
    )
    .unwrap();

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_external_change(&retry, "the retry");
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(world.worktree_source(TICKET_A), edited);
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_request_that_never_ran_does_not_take_another_request_s_identical_commit() {
    // The first request asks for what the file already holds, is
    // accepted, and never reaches the domain.
    let mut world = World::new();
    let earlier = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(earlier.outcome, Outcome::Success);
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(world.data.path());
    let token = world.token(TICKET_A);
    let died = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(died.code, ResultCode::InternalError);

    // Other requests save something else and then the same content again:
    // a commit in range now leaves the file exactly as the first intends.
    world.reopen();
    let other = world.execute(REQUEST_3, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(other.outcome, Outcome::Success);
    let again = world.execute(
        "01ARZ3NDEKTSV4RRFFQ69G5FX4",
        world.save(TICKET_A, &world.token(TICKET_A)),
    );
    assert_eq!(again.outcome, Outcome::Success);
    assert_eq!(world.branch_commits(TICKET_A), 3);
    // The file is byte for byte what it was when the first was accepted,
    // so its token is current again and the domain finds what it expects.
    assert_eq!(world.token(TICKET_A), token);

    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(retry.effects.commit_oid, None);
    assert_eq!(world.branch_commits(TICKET_A), 3);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

/// What someone else would commit for a created ticket with every field
/// the request sets and no short code.
fn created_without_slug(id: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {TITLE}\n\
         type: task\nstatus: open\n---\n{BODY}"
    )
}

#[test]
fn a_create_retried_after_the_initials_and_the_prefix_changed_keeps_its_short_code() {
    let mut world = failing_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
    let first = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_eq!(first.outcome, Outcome::Partial);
    assert_eq!(first.effects.checkpoint, CheckpointEffect::Pending);
    let written = world.slug(TICKET_NEW).expect("a short code");
    assert_eq!(written, format!("mt-{}", short_code(TICKET_NEW, 5)));
    assert_eq!(world.branch_commits(TICKET_NEW), 0);

    // The prefix and the code length change, and the user's initials
    // become something no short code can be composed from.
    world.commit_configuration("ticket_slug_prefix = \"mh\"\nticket_slug_code_length = 7\n");
    world.set_local_config("manyhands.initials", "!!");
    world.reopen();
    let retry = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(world.branch_commits(TICKET_NEW), 1);
    assert_eq!(
        retry.effects.commit_oid,
        world.branch_tip(TICKET_NEW).map(|tip| tip.to_string())
    );
    // The short code first written is the one committed and reported.
    assert_eq!(world.slug(TICKET_NEW), Some(written.clone()));
    assert_eq!(ticket_data(&retry).slug, Some(written));
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
    assert_eq!(world.pending_journal_rows(), 0);

    // A create that has to compose one is refused for the initials.
    let refused = world.execute(REQUEST_3, world.create("01ARZ3NDEKTSV4RRFFQ69G5FCQ"));
    assert_eq!(refused.code, ResultCode::InitialsRequired);
}

#[test]
fn a_foreign_commit_of_a_created_ticket_without_a_short_code_is_not_the_request_s_own() {
    // The create stops before it writes: its context exists and its row
    // is pending.
    let mut world = failing_at(FailurePoint::BeforeItemWrite);
    let first = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_eq!(first.outcome, Outcome::Error);
    assert!(world.worktree(TICKET_NEW).is_dir());

    // Someone commits the ticket with every other field equal.
    let path = crate::support::items::ticket_path(TICKET_NEW);
    let foreign = world.commit_in_context(TICKET_NEW, &path, &created_without_slug(TICKET_NEW));

    world.reopen();
    let retry = world.execute(REQUEST_1, world.create(TICKET_NEW));
    // A create's own commit holds a short code. This one is someone
    // else's: it is not reported, and the request is not run over it.
    assert_external_change(&retry, "the retry");
    assert_eq!(retry.effects.commit_oid, None);
    assert_eq!(world.branch_tip(TICKET_NEW), Some(foreign));
    assert_eq!(world.slug(TICKET_NEW), None);
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!(record.state, RequestState::Accepted);
    assert!(
        world
            .journal_row(&first.operation_id.clone().unwrap())
            .in_flight()
    );
}

/// The journal update that follows a save's commit finds the cache guard
/// held by another process.
#[cfg(unix)]
#[test]
fn a_commit_whose_journal_step_was_refused_is_finished_by_a_retry() {
    use std::sync::{Arc, Mutex};

    use crate::support::{LeaseHolder, hold_lease_in_child, mutation::owned_path_hook_test_lock};
    use manyhands::repository::{LeaseKind, OwnedPathBoundary, RepositoryService};

    let _hook_guard = owned_path_hook_test_lock();
    let mut world = World::new();
    let earlier = world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(earlier.outcome, Outcome::Success);

    // The file in the context already holds what the request intends,
    // uncommitted, and the caller has seen it: the save writes nothing,
    // so the first journal update after the lease is the one that
    // follows the commit.
    let path = crate::support::items::ticket_path(TICKET_A);
    let intended = world
        .worktree_source(TICKET_A)
        .replace("An earlier title", TITLE)
        .replace("An earlier body.\n", BODY);
    std::fs::write(world.worktree(TICKET_A).join(&path), &intended).unwrap();
    let token = world.token(TICKET_A);

    // Once the request is accepted and its domain call is about to start,
    // a hook is set on the save's first read of the file under its lease.
    // There, a child process takes the cache guard for reading: the
    // save's own reads of the index go on, its commit is made, and the
    // journal update after it cannot take the guard for writing.
    let holder: Arc<Mutex<Option<LeaseHolder>>> = Arc::default();
    {
        let holder = holder.clone();
        let root = world.root.clone();
        let data = world.data.path().to_owned();
        let worktree = world.worktree(TICKET_A);
        let relative = std::path::PathBuf::from(&path);
        world.service.set_request_hook_for_testing(move || {
            let hooks = RepositoryService::open_at(&data).unwrap();
            let held = holder.clone();
            hooks.set_owned_path_hook_for_root_for_testing(
                worktree,
                relative,
                OwnedPathBoundary::Read,
                move || {
                    *held.lock().unwrap() =
                        Some(hold_lease_in_child(&root, &data, LeaseKind::CacheRead));
                },
            );
        });
    }

    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    let held = holder.lock().unwrap().take().expect("the guard was taken");
    assert_eq!(
        (first.outcome, first.code, first.failure_class()),
        (
            Outcome::Partial,
            ResultCode::Busy,
            Some(FailureClass::Incomplete)
        )
    );
    assert_eq!(world.branch_commits(TICKET_A), 2, "the save committed");
    let commit = world.branch_tip(TICKET_A).unwrap().to_string();
    assert_eq!(first.effects.write, WriteEffect::Written);
    assert_eq!(first.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(first.effects.commit_oid, Some(commit.clone()));
    assert_eq!(recovery_actions(&first), ["request.retry"]);
    held.release();
    let operation_id = first.operation_id.clone().unwrap();
    assert!(world.journal_row(&operation_id).in_flight());
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));

    // The retry calls the save again. It writes and commits nothing,
    // completes its row and its hand-off, and the record is finished
    // with the commit the first attempt made.
    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &retry, 2, "the retry");
    assert_eq!(retry.effects.commit_oid, Some(commit));

    // A different request then succeeds.
    let next = world.execute(REQUEST_3, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(
        (next.outcome, next.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(world.branch_commits(TICKET_A), 3);
}
