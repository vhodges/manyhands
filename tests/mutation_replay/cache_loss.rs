//! After the cache is lost: the request records, the journals and the
//! registration are gone, and a retried request is decided from Git and
//! the file system.

use manyhands::{
    repository::{FailurePoint, Mutation, MutationDataDto},
    results::{
        CheckpointEffect, Effects, Envelope, FailureClass, Outcome, ResultCode, WriteEffect,
    },
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{
        BODY, REQUEST_1, REQUEST_2, TICKET_NEW, TITLE, World, recovery_actions, ticket_data,
    },
};

/// Asserts that a request sent before the rebuild is refused with the
/// action that brings the repository back, and has done nothing.
fn assert_not_registered(world: &World, envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::RepositoryNotRegistered,
            Some(FailureClass::Blocked)
        )
    );
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(recovery_actions(envelope), ["index.rebuild"]);
    assert_eq!(
        envelope.recovery[0].arguments["root"],
        world.root.to_str().unwrap()
    );
    assert_eq!(world.request_rows(), 0);
}

fn assert_already_applied(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
    assert_eq!(envelope.effects.write, WriteEffect::Unchanged);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(envelope.effects.commit_oid, None);
}

fn assert_external_change(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::ExternalChange,
            Some(FailureClass::Blocked)
        )
    );
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(recovery_actions(envelope).is_empty());
}

#[test]
fn a_completed_create_retried_after_the_cache_is_lost_is_already_applied() {
    let mut world = World::new();
    let first = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_eq!(first.outcome, Outcome::Success);
    let slug = ticket_data(&first).slug.clone().expect("a short code");
    let tip = world.branch_tip(TICKET_NEW);
    let source = world.worktree_source(TICKET_NEW);

    world.lose_cache();
    let before = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_not_registered(&world, &before);

    world.rebuild();
    // The initials have changed since: a create that had to compose a
    // short code would compose another. The item's own is written once.
    world.set_local_config("manyhands.initials", "QZ");
    let retry = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_already_applied(&retry);
    assert_eq!(ticket_data(&retry).slug, Some(slug));
    // No commit is added, nothing is written, and nothing is recorded as
    // in flight.
    assert_eq!(world.branch_tip(TICKET_NEW), tip);
    assert_eq!(world.branch_commits(TICKET_NEW), 1);
    assert_eq!(world.worktree_source(TICKET_NEW), source);
    assert_eq!(world.pending_journal_rows(), 0);

    // The same ID with other content is occupied.
    let other = world.execute(
        REQUEST_2,
        Mutation::TicketCreate(manyhands::repository::TicketCreateInput {
            draft: World::draft(TITLE, "Another body.\n"),
            ..world.create_input(TICKET_NEW)
        }),
    );
    assert_eq!(
        (other.outcome, other.code, other.failure_class()),
        (
            Outcome::Error,
            ResultCode::OccupiedPath,
            Some(FailureClass::Input)
        )
    );
    assert_eq!(other.effects, Effects::not_requested());
    assert_eq!(world.branch_commits(TICKET_NEW), 1);
    assert_eq!(world.worktree_source(TICKET_NEW), source);
    assert!(world.record(REQUEST_2).is_none());
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_completed_save_retried_after_the_cache_is_lost_is_already_applied() {
    let mut world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Success);
    let tip = world.branch_tip(TICKET_A);

    world.lose_cache();
    let before = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_not_registered(&world, &before);

    world.rebuild();
    let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_already_applied(&retry);
    assert_eq!(world.branch_tip(TICKET_A), tip);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert!(world.record(REQUEST_1).is_none(), "nothing was accepted");
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_write_that_was_not_committed_is_an_external_change_after_the_cache_is_lost() {
    // The first attempt of a save, and of a create, died between its
    // write and its checkpoint.
    for create in [false, true] {
        let what = format!("create: {create}");
        let id = if create { TICKET_NEW } else { TICKET_A };
        let mut world = World::with_service(|data| {
            FailOnce::at(FailurePoint::BeforeCheckpointCommit).open_service(data)
        });
        let token = world.token(TICKET_A);
        let request = |world: &World| {
            if create {
                world.create(TICKET_NEW)
            } else {
                world.save(TICKET_A, &token)
            }
        };
        let first = world.execute(REQUEST_1, request(&world));
        assert_eq!(first.outcome, Outcome::Partial, "{what}");
        assert_eq!(world.branch_commits(id), 0, "{what}");
        let written = world.worktree_source(id);
        assert!(written.contains(TITLE) && written.contains(BODY), "{what}");

        world.lose_cache();
        let before = world.execute(REQUEST_1, request(&world));
        assert_not_registered(&world, &before);

        world.rebuild();
        let retry = world.execute(REQUEST_1, request(&world));
        assert_external_change(&retry);
        assert_eq!(world.branch_commits(id), 0, "{what}: no commit is added");
        assert_eq!(world.worktree_source(id), written, "{what}");
        assert!(world.record(REQUEST_1).is_none(), "{what}");
        assert_eq!(world.pending_journal_rows(), 0, "{what}");

        // The caller reads the item again and saves it: that commits it.
        let fresh = world.execute(REQUEST_2, world.save(id, &world.token(id)));
        assert_eq!(
            (fresh.outcome, fresh.code),
            (Outcome::Success, ResultCode::Ok),
            "{what}"
        );
        assert_eq!(world.branch_commits(id), 1, "{what}");
        assert_eq!(
            fresh.effects.commit_oid,
            world.branch_tip(id).map(|tip| tip.to_string()),
            "{what}"
        );
        assert_eq!(world.worktree_source(id), written, "{what}");
    }
}

#[test]
fn a_file_someone_else_changed_is_an_external_change_after_the_cache_is_lost() {
    // Whether the change was committed or only written.
    for committed in [true, false] {
        let what = format!("committed: {committed}");
        let mut world = World::new();
        let token = world.token(TICKET_A);
        let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_eq!(first.outcome, Outcome::Success, "{what}");
        let path = items::ticket_path(TICKET_A);
        let other = items::ticket_source(TICKET_A, "Changed by someone else", "");
        if committed {
            world.commit_in_context(TICKET_A, &path, &other);
        } else {
            std::fs::write(world.worktree(TICKET_A).join(&path), &other).unwrap();
        }
        let tip = world.branch_tip(TICKET_A);

        world.lose_cache();
        let before = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_not_registered(&world, &before);

        world.rebuild();
        let retry = world.execute(REQUEST_1, world.save(TICKET_A, &token));
        assert_external_change(&retry);
        assert_eq!(
            world.branch_tip(TICKET_A),
            tip,
            "{what}: no commit is added"
        );
        assert_eq!(world.worktree_source(TICKET_A), other, "{what}");
        assert!(world.record(REQUEST_1).is_none(), "{what}");
        assert_eq!(world.pending_journal_rows(), 0, "{what}");
    }
}
