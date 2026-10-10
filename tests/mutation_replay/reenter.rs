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

#[test]
fn a_retry_after_lost_output_reports_the_commit_whatever_the_file_has_become() {
    let mut world = losing_output();
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    let commit = world.branch_tip(TICKET_A).unwrap();

    // Someone edits the file afterwards and does not commit. The request
    // finished before that: its operation completed and its commit is
    // there. Calling the save again would only be refused for the edit.
    let file = world
        .worktree(TICKET_A)
        .join(crate::support::items::ticket_path(TICKET_A));
    std::fs::write(&file, foreign_source()).unwrap();

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(retry.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(world.branch_tip(TICKET_A), Some(commit));
    assert_eq!(world.worktree_source(TICKET_A), foreign_source());
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

/// A save of `TICKET_A` in the repository at `root`, as `World::save`
/// makes it, for a caller that holds no `World`.
fn save_at(root: &std::path::Path, token: &str) -> Mutation {
    Mutation::TicketSave(TicketSaveInput {
        root: root.to_owned(),
        id: crate::support::items::item_id(TICKET_A),
        draft: World::draft(TITLE, BODY),
        deps: manyhands::repository::RelationshipWrite::Unchanged,
        parent: manyhands::repository::RelationshipWrite::Unchanged,
        observation: token.to_owned(),
    })
}

/// While one call is inside the domain operation, a second call of the
/// same request finds the repository busy.
#[cfg(unix)]
#[test]
fn a_concurrent_duplicate_is_busy_and_a_third_call_finishes_the_record() {
    use std::sync::mpsc;

    use crate::support::mutation::{execute_with, owned_path_hook_test_lock};
    use manyhands::repository::{OwnedPathBoundary, RepositoryService};

    let _hook_guard = owned_path_hook_test_lock();
    let world = World::new();
    let token = world.token(TICKET_A);

    // The first call is held at its save's first read of the file, which
    // is made under the repository lease.
    let (entered, has_entered) = mpsc::channel::<()>();
    let (release, released) = mpsc::channel::<()>();
    let first_service = RepositoryService::open_at(world.data.path()).unwrap();
    first_service.set_owned_path_hook_for_root_for_testing(
        world.worktree(TICKET_A),
        crate::support::items::ticket_path(TICKET_A).into(),
        OwnedPathBoundary::Read,
        move || {
            entered.send(()).unwrap();
            released.recv().unwrap();
        },
    );
    let first = {
        let root = world.root.clone();
        let token = token.clone();
        std::thread::spawn(move || execute_with(&first_service, REQUEST_1, save_at(&root, &token)))
    };
    has_entered.recv().unwrap();
    let record = world.record(REQUEST_1).expect("accepted");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));

    // The second call enters the record and cannot take the lease.
    let second = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (second.outcome, second.code, second.failure_class()),
        (
            Outcome::Error,
            ResultCode::Busy,
            Some(FailureClass::Transient)
        )
    );
    assert_eq!(second.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&second), ["request.retry"]);
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 2));
    assert_eq!(world.branch_commits(TICKET_A), 0);

    // The first completes and returns its commit to its caller. Its
    // attempt is no longer the record's, so it does not settle it.
    release.send(()).unwrap();
    let first = first.join().unwrap();
    assert_eq!(
        (first.outcome, first.code),
        (Outcome::Success, ResultCode::Ok)
    );
    let commit = world.branch_tip(TICKET_A).unwrap().to_string();
    assert_eq!(first.effects.commit_oid, Some(commit.clone()));
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 2));
    assert_eq!(record.result, None);

    // A third call finishes the record with that same commit.
    let third = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &third, 1, "the third call");
    assert_eq!(third.effects.commit_oid, Some(commit));
    assert_eq!(world.record(REQUEST_1).unwrap().attempt, 3);
}

/// A second call reads its evidence before the first commits and makes
/// its domain call after: the commit it finds was not there before, and
/// is the request's. `edited`: someone also edits the file between the
/// two, so the second call's save is refused.
fn second_call_after_the_first_commits(edited: bool) {
    use std::sync::mpsc;

    use crate::support::mutation::execute_with;
    use manyhands::repository::RepositoryService;

    let world = World::new();
    let token = world.token(TICKET_A);

    // The first call is accepted and held before its domain call: no
    // journal row and no commit exist yet.
    let (accepted, is_accepted) = mpsc::channel::<()>();
    let (go, gone) = mpsc::channel::<()>();
    let first_service = RepositoryService::open_at(world.data.path()).unwrap();
    first_service.set_request_hook_for_testing(move || {
        accepted.send(()).unwrap();
        gone.recv().unwrap();
    });
    let (done, is_done) = mpsc::channel();
    let first = {
        let root = world.root.clone();
        let token = token.clone();
        std::thread::spawn(move || {
            let envelope = execute_with(&first_service, REQUEST_1, save_at(&root, &token));
            done.send(()).unwrap();
            envelope
        })
    };
    is_accepted.recv().unwrap();
    let operation_id = world.record(REQUEST_1).unwrap().operations[0]
        .operation_id
        .to_string();
    assert_eq!(world.journal_row(&operation_id), JournalRow::Absent);

    // The second call has read the journal row and the evidence when its
    // hook runs. Only then does the first go on, to its end.
    let file = world
        .worktree(TICKET_A)
        .join(crate::support::items::ticket_path(TICKET_A));
    world.service.set_request_hook_for_testing(move || {
        go.send(()).unwrap();
        is_done.recv().unwrap();
        if edited {
            std::fs::write(&file, foreign_source()).unwrap();
        }
    });
    let second = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    let first = first.join().unwrap();
    assert_eq!(
        (first.outcome, first.code),
        (Outcome::Success, ResultCode::Ok)
    );
    let commit = world.branch_tip(TICKET_A).unwrap().to_string();
    assert_eq!(first.effects.commit_oid, Some(commit.clone()));

    // Not a no-op: the request committed, and the second call says so.
    assert_eq!(
        (second.outcome, second.code),
        (Outcome::Success, ResultCode::Ok),
        "edited: {edited}"
    );
    assert_eq!(second.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(second.effects.commit_oid, Some(commit));
    assert_eq!(world.branch_commits(TICKET_A), 1);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!((record.state, record.attempt), (RequestState::Finished, 2));
    assert_eq!(record.result.unwrap().effects, second.effects);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_second_call_that_read_its_evidence_before_the_first_committed_reports_that_commit() {
    second_call_after_the_first_commits(false);
}

#[test]
fn a_second_call_whose_save_is_refused_after_the_first_committed_reports_that_commit() {
    second_call_after_the_first_commits(true);
}

const SAME_REQUEST_ROOT: &str = "MANYHANDS_SAME_REQUEST_ROOT";
const SAME_REQUEST_DATA: &str = "MANYHANDS_SAME_REQUEST_DATA";
const SAME_REQUEST_TOKEN: &str = "MANYHANDS_SAME_REQUEST_TOKEN";
const SAME_REQUEST_GO: &str = "MANYHANDS_SAME_REQUEST_GO";
const SAME_REQUEST_OUT: &str = "MANYHANDS_SAME_REQUEST_OUT";

/// The process `two_processes_...` starts twice: it submits the request
/// when it is told to and writes the envelope it gets.
#[test]
fn same_request_child() {
    let Some(root) = std::env::var_os(SAME_REQUEST_ROOT) else {
        return;
    };
    let variable = |name: &str| std::path::PathBuf::from(std::env::var_os(name).unwrap());
    let service =
        manyhands::repository::RepositoryService::open_at(&variable(SAME_REQUEST_DATA)).unwrap();
    let token = std::env::var(SAME_REQUEST_TOKEN).unwrap();
    let out = variable(SAME_REQUEST_OUT);
    std::fs::write(out.with_extension("ready"), b"").unwrap();
    let go = variable(SAME_REQUEST_GO);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !go.exists() {
        assert!(std::time::Instant::now() < deadline, "never told to go");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let envelope = crate::support::mutation::execute_with(
        &service,
        REQUEST_1,
        save_at(std::path::Path::new(&root), &token),
    );
    std::fs::write(&out, serde_json::to_vec(&envelope).unwrap()).unwrap();
}

#[test]
fn two_processes_submitting_the_same_new_request_make_one_commit() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let scratch = tempfile::tempdir().unwrap();
    let go = scratch.path().join("go");
    let outs = [scratch.path().join("a.json"), scratch.path().join("b.json")];
    let mut children: Vec<std::process::Child> = outs
        .iter()
        .map(|out| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "reenter::same_request_child", "--nocapture"])
                .env(SAME_REQUEST_ROOT, &world.root)
                .env(SAME_REQUEST_DATA, world.data.path())
                .env(SAME_REQUEST_TOKEN, &token)
                .env(SAME_REQUEST_GO, &go)
                .env(SAME_REQUEST_OUT, out)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for (child, out) in children.iter_mut().zip(&outs) {
        crate::support::wait_for_path(child, &out.with_extension("ready"));
    }
    std::fs::write(&go, b"").unwrap();
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "a child failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // One set of effects.
    assert_eq!(world.branch_commits(TICKET_A), 1);
    let commit = world.branch_tip(TICKET_A).unwrap().to_string();
    assert_eq!(world.request_rows(), 1);
    // Each caller gets the request's result, or is told the repository
    // was busy; at least one of them made the commit and says so.
    let mut succeeded = 0;
    for out in &outs {
        let envelope: serde_json::Value =
            serde_json::from_slice(&std::fs::read(out).unwrap()).unwrap();
        let what = envelope.to_string();
        match envelope["code"].as_str().unwrap() {
            "ok" => {
                succeeded += 1;
                assert_eq!(envelope["outcome"], "success", "{what}");
                assert_eq!(envelope["effects"]["commit_oid"], commit, "{what}");
                assert_eq!(envelope["effects"]["checkpoint"], "committed", "{what}");
            }
            "busy" => {
                assert_eq!(envelope["outcome"], "error", "{what}");
                assert!(envelope["effects"]["commit_oid"].is_null(), "{what}");
            }
            other => panic!("neither a result nor busy: {other}: {what}"),
        }
    }
    assert!(succeeded >= 1);

    // Whatever each was told, the request is answered with its commit
    // from here on, and nothing is left in flight.
    let after = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (after.outcome, after.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(after.effects.commit_oid, Some(commit));
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

const KILLED_ROOT: &str = "MANYHANDS_KILLED_ROOT";
const KILLED_DATA: &str = "MANYHANDS_KILLED_DATA";
const KILLED_TOKEN: &str = "MANYHANDS_KILLED_TOKEN";
const KILLED_POINT: &str = "MANYHANDS_KILLED_POINT";

/// The process `a_retry_completes_a_save_whose_process_was_killed...`
/// starts: it submits the save and its process ends at the fault point,
/// inside the domain operation, with nothing after it run.
#[test]
fn killed_at_fault_point_child() {
    let Some(root) = std::env::var_os(KILLED_ROOT) else {
        return;
    };
    let name = std::env::var(KILLED_POINT).unwrap();
    let point = INTERRUPTIONS
        .into_iter()
        .map(|(point, _)| point)
        .chain([FailurePoint::BeforeRequestSettlement])
        .find(|point| format!("{point:?}") == name)
        .expect("a known fault point");
    let service = manyhands::repository::RepositoryService::open_at_with_exit_point_for_testing(
        std::path::Path::new(&std::env::var_os(KILLED_DATA).unwrap()),
        point,
    )
    .unwrap();
    let token = std::env::var(KILLED_TOKEN).unwrap();
    crate::support::mutation::execute_with(
        &service,
        REQUEST_1,
        save_at(std::path::Path::new(&root), &token),
    );
    // Not reached: the process ended at the fault point.
}

#[test]
fn a_retry_completes_a_save_whose_process_was_killed_at_each_fault_point() {
    for (point, written) in INTERRUPTIONS {
        let what = format!("{point:?}");
        let mut world = World::new();
        let token = world.token(TICKET_A);
        killed_at(&world, &token, point);

        // What a dead process leaves: the record accepted and never
        // settled, and the journal row pending, in every case.
        let record = world.record(REQUEST_1).expect("the record stays");
        assert_eq!(
            (record.state, record.attempt),
            (RequestState::Accepted, 1),
            "{what}"
        );
        let operation_id = record.operations[0].operation_id.to_string();
        assert!(
            matches!(world.journal_row(&operation_id), JournalRow::Pending(_)),
            "{what}"
        );
        assert_eq!(
            world.worktree_source(TICKET_A).contains(TITLE),
            written,
            "{what}"
        );
        let committed = point == FailurePoint::BeforeIndexTransactionCommit;
        assert_eq!(
            world.branch_commits(TICKET_A),
            usize::from(committed),
            "{what}"
        );

        // The retry, with the same body, completes.
        world.reopen();
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_finished_with_its_commit(&world, REQUEST_1, &retry, 1, &what);
        assert_eq!(retry.operation_id, Some(operation_id), "{what}");
    }
}

/// Runs the save in a child process that ends at `point`.
fn killed_at(world: &World, token: &str, point: FailurePoint) {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "reenter::killed_at_fault_point_child",
            "--nocapture",
        ])
        .env(KILLED_ROOT, &world.root)
        .env(KILLED_DATA, world.data.path())
        .env(KILLED_TOKEN, token)
        .env(KILLED_POINT, format!("{point:?}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(manyhands::repository::FAILURE_POINT_EXIT_STATUS),
        "{point:?}: the child ended at the fault point"
    );
}

#[test]
fn a_retry_after_a_process_killed_before_it_settled_reports_its_commit() {
    let mut world = World::new();
    let token = world.token(TICKET_A);
    killed_at(&world, &token, FailurePoint::BeforeRequestSettlement);
    // The save ran to its end in the process that died.
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    let commit = world.branch_tip(TICKET_A).unwrap().to_string();

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &retry, 1, "the retry");
    assert_eq!(retry.effects.commit_oid, Some(commit));
}

#[test]
fn a_retry_that_cannot_read_git_changes_nothing_and_claims_nothing() {
    let mut world = losing_output();
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    let commit = world.branch_tip(TICKET_A).unwrap();

    // The file the request committed cannot be read from Git: its object
    // is there and is not an object.
    let repository = git2::Repository::open(&world.root).unwrap();
    let blob = repository
        .find_commit(commit)
        .unwrap()
        .tree()
        .unwrap()
        .get_path(std::path::Path::new(&crate::support::items::ticket_path(
            TICKET_A,
        )))
        .unwrap()
        .id()
        .to_string();
    let object = repository
        .path()
        .join("objects")
        .join(&blob[..2])
        .join(&blob[2..]);
    drop(repository);
    let intact = std::fs::read(&object).unwrap();
    let mut permissions = std::fs::metadata(&object).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&object, permissions).unwrap();
    std::fs::write(&object, b"not an object").unwrap();

    // Whether the request committed cannot be told. It is not called a
    // no-op, which would be stored and replayed for good; no commit is
    // reported; and the record is neither finished nor deleted.
    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (retry.outcome, retry.code, retry.failure_class()),
        (
            Outcome::Error,
            ResultCode::InternalError,
            Some(FailureClass::Internal)
        )
    );
    assert_eq!(retry.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&retry), ["request.retry"]);
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 2));
    assert_eq!(record.result, None);
    assert_eq!(world.branch_tip(TICKET_A), Some(commit));

    // Once Git can be read again the same request is finished with its
    // commit.
    std::fs::write(&object, intact).unwrap();
    world.reopen();
    let again = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &again, 1, "the next retry");
    assert_eq!(again.effects.commit_oid, Some(commit.to_string()));
}

fn assert_no_op_without_a_commit(envelope: &Envelope<MutationDataDto>, what: &str) {
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Noop, ResultCode::AlreadyApplied),
        "{what}"
    );
    assert_eq!(envelope.effects.write, WriteEffect::Unchanged, "{what}");
    assert_eq!(
        envelope.effects.checkpoint,
        CheckpointEffect::Unchanged,
        "{what}"
    );
    assert_eq!(envelope.effects.commit_oid, None, "{what}");
}

#[test]
fn a_save_that_changed_nothing_does_not_take_a_later_commit_of_the_same_content() {
    // The first request asks for what the file already holds. It runs, as
    // a no-op, and its output is lost.
    let mut world = World::new();
    let earlier = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(earlier.outcome, Outcome::Success);
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    assert_eq!(world.branch_commits(TICKET_A), 1);

    // Other requests save something else and then the same content again.
    world.reopen();
    let other = world.execute(REQUEST_3, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(other.outcome, Outcome::Success);
    let again = world.execute(
        "01ARZ3NDEKTSV4RRFFQ69G5FX4",
        world.save(TICKET_A, &world.token(TICKET_A)),
    );
    assert_eq!(again.outcome, Outcome::Success);
    let tip = world.branch_tip(TICKET_A);
    assert_eq!(world.branch_commits(TICKET_A), 3);

    // That commit left the file as the first request intends, and it was
    // not made from the state the first request was accepted against.
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_no_op_without_a_commit(&retry, "the retry");
    assert_eq!(world.branch_tip(TICKET_A), tip);
    assert_eq!(world.branch_commits(TICKET_A), 3);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.result.unwrap().outcome, Outcome::Noop);
}

const UNIDENTIFIED_HOME: &str = "MANYHANDS_UNIDENTIFIED_HOME";

/// Runs in a process of its own: where libgit2 looks for configuration is
/// process-wide, and this points every level but the repository's at an
/// empty directory so that no identity is found.
#[test]
fn a_request_that_stopped_for_an_identity_does_not_take_another_request_s_commit() {
    let Some(home) = std::env::var_os(UNIDENTIFIED_HOME) else {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "reenter::a_request_that_stopped_for_an_identity_does_not_take_another_request_s_commit",
                "--nocapture",
            ])
            .env(UNIDENTIFIED_HOME, home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "the child failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
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
    let mut world = losing_output();
    let token = world.token(TICKET_A);
    let mut local = git2::Repository::open(&world.root)
        .unwrap()
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    local.remove("user.name").unwrap();
    local.remove("user.email").unwrap();

    // The first request stops for want of an identity. The domain
    // completes its row, having written nothing, and the output is lost.
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    // The step the journal kept is the context's, not a checkpoint's.
    let step: String = crate::support::items::index(world.data.path())
        .query_row(
            "SELECT completed_step FROM operation_records WHERE action = 'save_ticket'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(step, "worktree_observed");
    let operation_id = world.record(REQUEST_1).unwrap().operations[0]
        .operation_id
        .to_string();
    assert_eq!(
        world.journal_row(&operation_id),
        JournalRow::Final {
            kind: manyhands::repository::request_store::FinalKind::Completed,
            owes_work: false,
            checkpointed: false,
        },
        "the row never reached a checkpoint"
    );
    assert_eq!(world.branch_commits(TICKET_A), 0);
    assert!(!world.worktree_source(TICKET_A).contains(TITLE));

    // An identity is set and another request saves the same content.
    world.set_local_config("user.name", "Manyhands Test");
    world.set_local_config("user.email", "manyhands-test@example.invalid");
    world.reopen();
    let second = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(second.outcome, Outcome::Success);
    let tip = world.branch_tip(TICKET_A);
    assert_eq!(world.branch_commits(TICKET_A), 1);

    // The commit was made from exactly the state the first request was
    // accepted against, and leaves the file as it intends. Only the
    // journal says the first request never reached a checkpoint.
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_no_op_without_a_commit(&retry, "the retry");
    assert_eq!(world.branch_tip(TICKET_A), tip);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

#[test]
fn a_retry_completes_after_the_branch_was_rewritten_under_a_request_in_flight() {
    // A save of a ticket with a context is killed after its write: its
    // row is pending and its file is written and not committed.
    let mut world = World::new();
    let earlier = world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(earlier.outcome, Outcome::Success);
    world.service =
        FailOnce::at(FailurePoint::BeforeCheckpointCommit).open_service(world.data.path());
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Partial);
    assert!(world.worktree_source(TICKET_A).contains(TITLE));

    // Someone amends the branch's tip. The recorded commit is no longer
    // an ancestor of the branch, so the evidence is looked for in all of
    // it, where the newest change to the file is the earlier save: not
    // what this request intends, and not a change from elsewhere either.
    let recorded = world.record(REQUEST_1).unwrap().base_oid.unwrap();
    let amended = world.amend_branch_tip(TICKET_A);
    assert_ne!(amended, recorded);
    assert_eq!(world.branch_commits(TICKET_A), 1);

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
    // One commit of the request's, on top of the amended one, and it is
    // the one reported: it was not there before the call.
    assert_eq!(world.branch_commits(TICKET_A), 2);
    let tip = world.branch_tip(TICKET_A).unwrap();
    assert_eq!(retry.effects.commit_oid, Some(tip.to_string()));
    assert_eq!(
        git2::Repository::open(&world.root)
            .unwrap()
            .find_commit(tip)
            .unwrap()
            .parent_id(0)
            .unwrap(),
        amended
    );
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn an_older_commit_of_the_same_content_is_not_reported_after_the_branch_was_reset() {
    // The branch's history holds the intended content, then something
    // else, which is what the request is accepted against.
    let mut world = World::new();
    let old = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(old.outcome, Outcome::Success);
    let old_commit = world.branch_tip(TICKET_A).unwrap();
    let between = world.execute(REQUEST_3, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(between.outcome, Outcome::Success);

    // The request saves the intended content again, commits, and its
    // output is lost.
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    let own = world.branch_tip(TICKET_A).unwrap();
    assert_eq!(world.branch_commits(TICKET_A), 3);

    // The branch is reset behind the commit the request was accepted at:
    // the request's own commit is gone from it, and the range is all of
    // the branch, where an old commit holds the same content.
    world.reset_context(TICKET_A, old_commit);
    assert_eq!(world.branch_commits(TICKET_A), 1);

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    // The old commit is not the request's and is not reported. Nor is
    // the request's own, which the branch no longer holds. What it asked
    // for is what the branch holds, committed: a no-op.
    assert_no_op_without_a_commit(&retry, "the retry");
    assert_ne!(retry.effects.commit_oid, Some(old_commit.to_string()));
    assert_ne!(retry.effects.commit_oid, Some(own.to_string()));
    assert_eq!(world.branch_tip(TICKET_A), Some(old_commit));
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

/// Starts the save on another thread, with a service of its own, and
/// holds it after the request is accepted and before its domain call.
/// Returns the thread and what lets it go on.
fn first_call_held_before_the_domain(
    world: &World,
    token: &str,
) -> (
    std::thread::JoinHandle<Envelope<MutationDataDto>>,
    std::sync::mpsc::Sender<()>,
) {
    use std::sync::mpsc;

    let (accepted, is_accepted) = mpsc::channel::<()>();
    let (go, gone) = mpsc::channel::<()>();
    let service = manyhands::repository::RepositoryService::open_at(world.data.path()).unwrap();
    service.set_request_hook_for_testing(move || {
        accepted.send(()).unwrap();
        gone.recv().unwrap();
    });
    let root = world.root.clone();
    let token = token.to_owned();
    let thread = std::thread::spawn(move || {
        crate::support::mutation::execute_with(&service, REQUEST_1, save_at(&root, &token))
    });
    is_accepted.recv().unwrap();
    (thread, go)
}

/// The known limit of settling from the journal row alone: a call that
/// re-enters while the first call has not yet begun its row finds nothing
/// in flight. The backstop is the already-applied rule.
#[test]
fn a_record_deleted_under_a_first_call_that_had_not_begun_is_answered_as_already_applied() {
    use crate::support::hold_lease_in_child;
    use manyhands::repository::LeaseKind;

    let world = World::new();
    let token = world.token(TICKET_A);
    let (first, go) = first_call_held_before_the_domain(&world, &token);

    // An unrelated operation holds the repository lease. The second call
    // of the request reads no journal row, cannot take the lease, and
    // still finds no row: nothing is in flight, so the record is deleted.
    let lease = hold_lease_in_child(&world.root, world.data.path(), LeaseKind::Repository);
    let second = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (second.outcome, second.code),
        (Outcome::Error, ResultCode::Busy)
    );
    assert_eq!(second.effects, Effects::not_requested());
    assert!(world.record(REQUEST_1).is_none(), "the record is deleted");
    lease.release();

    // The first call then runs. Its caller gets its commit; there is no
    // record left for it to settle.
    go.send(()).unwrap();
    let first = first.join().unwrap();
    assert_eq!(
        (first.outcome, first.code),
        (Outcome::Success, ResultCode::Ok)
    );
    let commit = world.branch_tip(TICKET_A).unwrap();
    assert_eq!(first.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.pending_journal_rows(), 0);

    // A later retry is a request no record holds. What it asks for is
    // there and committed: nothing is written and no commit is claimed.
    let source = world.worktree_source(TICKET_A);
    let later = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_no_op_without_a_commit(&later, "the later retry");
    assert_eq!(world.branch_tip(TICKET_A), Some(commit));
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(world.worktree_source(TICKET_A), source);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_second_call_that_runs_before_the_first_reaches_the_domain_commits_and_finishes() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let (first, go) = first_call_held_before_the_domain(&world, &token);

    // The second call enters the record, finds nothing begun, and does
    // the work itself.
    let second = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &second, 1, "the second call");
    assert_eq!(world.record(REQUEST_1).unwrap().attempt, 2);
    let commit = second.effects.commit_oid.clone();

    // The first call's save then finds everything done under its
    // operation ID. It made no commit and says so; the record is the
    // second call's and is not changed.
    go.send(()).unwrap();
    let first = first.join().unwrap();
    assert_no_op_without_a_commit(&first, "the first call");
    assert_eq!(first.operation_id, second.operation_id);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!((record.state, record.attempt), (RequestState::Finished, 2));
    assert_eq!(record.result.unwrap().effects.commit_oid, commit);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_third_call_finishes_what_a_killed_second_call_committed() {
    // The first call writes and stops.
    let mut world = failing_at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Partial);
    assert_eq!(world.branch_commits(TICKET_A), 0);

    // The second re-enters, commits, and its process is killed before it
    // settles the record.
    killed_at(&world, &token, FailurePoint::BeforeRequestSettlement);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    let commit = world.branch_tip(TICKET_A).unwrap().to_string();
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 2));
    assert_eq!(world.pending_journal_rows(), 0);

    world.reopen();
    let third = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &third, 1, "the third call");
    assert_eq!(third.effects.commit_oid, Some(commit));
    assert_eq!(world.record(REQUEST_1).unwrap().attempt, 3);
}

#[test]
fn a_hand_off_that_keeps_failing_keeps_the_commit_and_the_record() {
    let mut world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.code, ResultCode::DiscoveryPending);
    let commit = first.effects.commit_oid.clone().expect("a commit");

    // The hand-off fails twice more.
    for attempt in [2, 3] {
        world.service = FailOnce::at(FailurePoint::BeforeIndexTransactionCommit)
            .open_service(world.data.path());
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_eq!(
            (retry.outcome, retry.code, retry.failure_class()),
            (
                Outcome::Partial,
                ResultCode::DiscoveryPending,
                Some(FailureClass::Incomplete)
            ),
            "attempt {attempt}"
        );
        assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
        assert_eq!(retry.effects.discovery, DiscoveryEffect::Pending);
        assert_eq!(retry.effects.commit_oid, Some(commit.clone()));
        assert_eq!(recovery_actions(&retry), ["operation.resume"]);
        assert_eq!(world.branch_commits(TICKET_A), 1, "attempt {attempt}");
        let record = world.record(REQUEST_1).expect("the record stays");
        assert_eq!(
            (record.state, record.attempt),
            (RequestState::Accepted, attempt)
        );
    }

    world.reopen();
    let last = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &last, 1, "the last retry");
    assert_eq!(last.effects.commit_oid, Some(commit));
}

#[test]
fn a_rejected_attempt_that_died_before_its_record_was_deleted_leaves_nothing_on_the_retry() {
    let mut world = World::new();
    let earlier = world.execute(REQUEST_2, earlier_save(&world, &world.token(TICKET_A)));
    assert_eq!(earlier.outcome, Outcome::Success);
    let token = world.token(TICKET_A);

    // Between the boundary's check and the domain's, someone edits the
    // file. The domain rejects the save and closes its row; the process
    // dies before the record is deleted.
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
    let file = world
        .worktree(TICKET_A)
        .join(crate::support::items::ticket_path(TICKET_A));
    world
        .service
        .set_request_hook_for_testing(move || std::fs::write(&file, foreign_source()).unwrap());
    let died = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(died.code, ResultCode::InternalError);
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!(record.state, RequestState::Accepted);
    let operation_id = record.operations[0].operation_id.to_string();
    assert_eq!(world.journal_row(&operation_id), JournalRow::Absent);
    assert_eq!(world.pending_journal_rows(), 0);

    // The same input again: rejected again, and this time nothing is left.
    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_external_change(&retry, "the retry");
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(world.request_rows(), 1, "only the earlier request's");
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(world.journal_row(&operation_id), JournalRow::Absent);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(world.worktree_source(TICKET_A), foreign_source());

    // The request ID is free: with a fresh read it runs.
    let fresh = world.execute(REQUEST_1, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(
        (fresh.outcome, fresh.code),
        (Outcome::Success, ResultCode::Ok)
    );
}

#[test]
fn a_re_entry_that_cannot_set_up_its_call_keeps_the_record_of_a_request_that_committed() {
    let mut world = losing_output();
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    let commit = world.branch_tip(TICKET_A).unwrap();

    // The repository cannot be resolved for a while: its Git directory is
    // away. The index, with the record and the journal, is where it was.
    let git = world.root.join(".git");
    let away = world.root.join(".git-away");
    std::fs::rename(&git, &away).unwrap();
    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    std::fs::rename(&away, &git).unwrap();
    assert_ne!(retry.outcome, Outcome::Success);
    assert_ne!(retry.outcome, Outcome::Noop);
    assert_eq!(retry.code, ResultCode::NotRepository);
    assert_eq!(retry.effects, Effects::not_requested());
    // The request committed and its operation is complete: nothing is in
    // flight, and the record stays all the same, because this call
    // learned nothing of what the request did.
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 2));
    assert_eq!(record.result, None);

    // The next retry finishes it with its commit.
    world.reopen();
    let again = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &again, 1, "the next retry");
    assert_eq!(again.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(world.record(REQUEST_1).unwrap().attempt, 3);
}

#[test]
fn a_retry_during_which_git_becomes_unreadable_claims_nothing() {
    // The first attempt committed and stopped before its hand-off.
    let mut world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.code, ResultCode::DiscoveryPending);
    let commit = world.branch_tip(TICKET_A).unwrap();

    // The file as it was committed when the request was accepted: the
    // evidence reads it to tell the request's own commit, and the save,
    // which finds its file written and committed, never opens it.
    let repository = git2::Repository::open(&world.root).unwrap();
    let blob = repository
        .find_commit(world.primary_head())
        .unwrap()
        .tree()
        .unwrap()
        .get_path(std::path::Path::new(&crate::support::items::ticket_path(
            TICKET_A,
        )))
        .unwrap()
        .id()
        .to_string();
    let object = repository
        .path()
        .join("objects")
        .join(&blob[..2])
        .join(&blob[2..]);
    drop(repository);
    let intact = std::fs::read(&object).unwrap();
    let mut permissions = std::fs::metadata(&object).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&object, permissions).unwrap();

    // The retry reads its journal row and its evidence, which are fine.
    // Then, before its domain call, the committed file's object stops
    // being one: the evidence cannot be read after the call.
    world.reopen();
    {
        let object = object.clone();
        world.service.set_request_hook_for_testing(move || {
            std::fs::write(&object, b"not an object").unwrap()
        });
    }
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_eq!(retry.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&retry), ["request.retry"]);
    // The save itself ran to its end: its row and its hand-off are
    // complete. Only what the request committed could not be read.
    assert!(
        !world
            .journal_row(&first.operation_id.clone().unwrap())
            .in_flight()
    );
    assert_eq!(world.pending_journal_rows(), 0);
    let record = world.record(REQUEST_1).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 2));
    assert_eq!(record.result, None);
    assert_eq!(world.branch_tip(TICKET_A), Some(commit));

    std::fs::write(&object, intact).unwrap();
    world.reopen();
    let again = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_finished_with_its_commit(&world, REQUEST_1, &again, 1, "the next retry");
    assert_eq!(again.effects.commit_oid, Some(commit.to_string()));
}

/// Pins what a retry returns once the branch the request committed to is
/// gone, and its commit with it. Neither case is a no-op: nothing holds
/// what the request asked for, committed.
#[test]
fn a_retry_after_lost_output_and_the_deletion_of_the_context_branch() {
    // `false`: only the branch is deleted, and the worktree is left with
    // its head pointing at nothing. `true`: the whole editing context is
    // removed, worktree and branch.
    for whole in [false, true] {
        let what = format!("the whole context removed: {whole}");
        let mut world = losing_output();
        let token = world.token(TICKET_A);
        let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_output_lost(&world, REQUEST_1, &lost);
        let first_commit = world.branch_tip(TICKET_A).unwrap();

        let repository = git2::Repository::open(&world.root).unwrap();
        if whole {
            std::fs::remove_dir_all(world.worktree(TICKET_A)).unwrap();
            for name in repository.worktrees().unwrap().iter().flatten() {
                repository
                    .find_worktree(name)
                    .unwrap()
                    .prune(Some(
                        git2::WorktreePruneOptions::new()
                            .working_tree(true)
                            .valid(true),
                    ))
                    .unwrap();
            }
        }
        repository
            .find_reference(&format!("refs/heads/{}", World::branch(TICKET_A)))
            .unwrap()
            .delete()
            .unwrap();
        drop(repository);

        world.reopen();
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        if whole {
            // The save, called again, makes the context from primary,
            // finds the file as the request expected it, and writes and
            // commits: the commit is new and is the request's.
            assert_eq!(
                (retry.outcome, retry.code),
                (Outcome::Success, ResultCode::Ok),
                "{what}"
            );
            assert_eq!(world.branch_commits(TICKET_A), 1, "{what}");
            let tip = world.branch_tip(TICKET_A).unwrap();
            assert_ne!(tip, first_commit, "{what}");
            assert_eq!(retry.effects.commit_oid, Some(tip.to_string()), "{what}");
            assert!(world.worktree_source(TICKET_A).contains(TITLE), "{what}");
            let record = world.record(REQUEST_1).expect("a record");
            assert_eq!(record.state, RequestState::Finished, "{what}");
        } else {
            // The save cannot work in a worktree whose branch is gone.
            // Nothing is claimed; the operation had completed, so nothing
            // is in flight and the record is deleted.
            assert_eq!(
                (retry.outcome, retry.code),
                (Outcome::Error, ResultCode::InternalError),
                "{what}"
            );
            assert_eq!(retry.effects, Effects::not_requested(), "{what}");
            assert!(world.record(REQUEST_1).is_none(), "{what}");
            assert_eq!(world.branch_tip(TICKET_A), None, "{what}");
            // The file still holds what the request wrote, uncommitted.
            assert!(world.worktree_source(TICKET_A).contains(TITLE), "{what}");
        }
        assert_eq!(world.pending_journal_rows(), 0, "{what}");
    }
}

#[test]
fn a_save_that_changed_nothing_does_not_take_a_commit_that_leaves_its_fields_alone() {
    // The first request asks for what the file already holds. It runs, as
    // a no-op, and its output is lost.
    let mut world = World::new();
    let earlier = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(earlier.outcome, Outcome::Success);
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_output_lost(&world, REQUEST_1, &lost);
    assert_eq!(world.branch_commits(TICKET_A), 1);

    // Another request saves the same title and body and sets the
    // dependencies, which the first request leaves unchanged. Its commit
    // is made from exactly what the first expected, and leaves every
    // field the first sets as the first intends.
    world.reopen();
    let other = world.execute(
        REQUEST_3,
        Mutation::TicketSave(TicketSaveInput {
            deps: manyhands::repository::RelationshipWrite::Set(vec![
                crate::support::items::TICKET_ABSENT.to_owned(),
            ]),
            ..world.save_input(TICKET_A, &world.token(TICKET_A))
        }),
    );
    assert_eq!(other.outcome, Outcome::Success);
    let tip = world.branch_tip(TICKET_A);
    assert_eq!(world.branch_commits(TICKET_A), 2);

    // The first request had nothing to commit when it was accepted, so no
    // commit is its own.
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_no_op_without_a_commit(&retry, "the retry");
    assert_eq!(world.branch_tip(TICKET_A), tip);
    assert_eq!(world.branch_commits(TICKET_A), 2);
    assert!(world.worktree_source(TICKET_A).contains("deps:"));
}

#[test]
fn a_foreign_revert_of_the_commit_of_a_request_in_flight_is_not_undone() {
    // The first attempt committed and stopped before its hand-off.
    let mut world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
    let token = world.token(TICKET_A);
    let before = world.primary_source(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.code, ResultCode::DiscoveryPending);
    let own = first.effects.commit_oid.clone().expect("a commit");

    // Someone reverts it: the branch's tip holds the file exactly as the
    // request expected it again.
    let revert = world.commit_in_context(
        TICKET_A,
        &crate::support::items::ticket_path(TICKET_A),
        &before,
    );

    world.reopen();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    // A change from elsewhere over the request's own commit: the request
    // is not run again, and what it did is reported.
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Partial, ResultCode::ExternalChange)
    );
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(retry.effects.commit_oid, Some(own));
    assert!(recovery_actions(&retry).is_empty());
    assert_eq!(world.branch_tip(TICKET_A), Some(revert));
    assert_eq!(world.branch_commits(TICKET_A), 2);
    assert_eq!(world.worktree_source(TICKET_A), before);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
}
