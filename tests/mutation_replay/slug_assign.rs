//! `ticket slug-assign`: a generated short code for a ticket that has
//! none.

use manyhands::{
    canonical::short_code,
    repository::{FailurePoint, Mutation, RequestState, TicketSlugAssignInput},
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, FailureClass, Outcome, ResultCode, WriteEffect,
    },
};

use crate::support::{
    FailOnce,
    items::TICKET_A,
    mutation::{
        REQUEST_1, REQUEST_2, REQUEST_3, TICKET_CLOSED, World, recovery_actions, ticket_data,
    },
};

/// The short code the fixture's identity, "Manyhands Test", gives a ticket.
fn derived(id: &str) -> String {
    format!("mt-{}", short_code(id, 5))
}

fn with_initials(world: &World, id: &str, token: &str, initials: &str) -> Mutation {
    Mutation::TicketSlugAssign(TicketSlugAssignInput {
        initials: Some(initials.to_owned()),
        ..world.slug_assign_input(id, token)
    })
}

#[test]
fn a_short_code_is_assigned_to_a_ticket_that_has_none() {
    let world = World::new();
    let primary = world.primary_source(TICKET_A);
    assert!(!primary.contains("slug:"));
    let token = world.token(TICKET_A);

    let envelope = world.execute(REQUEST_1, world.slug_assign(TICKET_A, &token));

    assert_eq!(
        (envelope.outcome, envelope.code, envelope.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    assert_eq!(envelope.command, "ticket slug-assign");
    assert_eq!(envelope.scope.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(envelope.effects.write, WriteEffect::Written);
    assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(
        envelope.effects.commit_oid,
        world.branch_tip(TICKET_A).map(|tip| tip.to_string())
    );
    let slug = derived(TICKET_A);
    assert_eq!(ticket_data(&envelope).slug.as_deref(), Some(&*slug));
    assert_eq!(world.slug(TICKET_A).as_deref(), Some(&*slug));
    // Nothing else of the ticket changed: the file is written in its
    // canonical form, with the fields it had.
    let written = world.worktree_source(TICKET_A);
    let lines = |source: &str| {
        let mut lines: Vec<String> = source.lines().map(str::to_owned).collect();
        lines.sort();
        lines
    };
    assert_eq!(
        lines(&written.replace(&format!("slug: {slug}\n"), "")),
        lines(&primary)
    );
    assert!(
        written.ends_with("---\nBody of Open ticket.\n"),
        "{written}"
    );
    let record = world.record(REQUEST_1).expect("a record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.command, "ticket slug-assign");
    assert_eq!(world.pending_journal_rows(), 0);

    // Initials given for the one call are used and stored nowhere.
    let other = World::new();
    let before = other.local_config_bytes();
    let token = other.token(TICKET_A);
    let envelope = other.execute(REQUEST_1, with_initials(&other, TICKET_A, &token, "Ab3"));
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(
        other.slug(TICKET_A),
        Some(format!("ab3-{}", short_code(TICKET_A, 5)))
    );
    assert_eq!(other.local_config_bytes(), before);
}

#[test]
fn a_ticket_that_has_another_short_code_is_slug_already_assigned() {
    let world = World::new();
    let first = world.execute(
        REQUEST_1,
        world.slug_assign(TICKET_A, &world.token(TICKET_A)),
    );
    assert_eq!(first.code, ResultCode::Ok);
    let tip = world.branch_tip(TICKET_A);
    let source = world.worktree_source(TICKET_A);

    // The caller has seen the ticket as it is and asks for another code.
    let token = world.token(TICKET_A);
    for (what, token) in [("a current token", &*token), ("a stale token", "v1:stale")] {
        let envelope = world.execute(REQUEST_2, with_initials(&world, TICKET_A, token, "zz"));
        assert_eq!(
            (envelope.outcome, envelope.code, envelope.failure_class()),
            (
                Outcome::Error,
                ResultCode::SlugAlreadyAssigned,
                Some(FailureClass::Input)
            ),
            "{what}"
        );
        assert_eq!(envelope.effects, Effects::not_requested(), "{what}");
        assert!(recovery_actions(&envelope).is_empty(), "{what}");
        assert_eq!(envelope.operation_id, None, "{what}");
        // The code the ticket holds is reported.
        assert_eq!(
            ticket_data(&envelope).slug,
            Some(derived(TICKET_A)),
            "{what}"
        );
        assert!(world.record(REQUEST_2).is_none(), "{what}");
        assert_eq!(world.branch_tip(TICKET_A), tip, "{what}");
        assert_eq!(world.worktree_source(TICKET_A), source, "{what}");
    }
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_new_request_for_the_short_code_the_ticket_already_holds_is_already_applied() {
    let mut world = World::new();
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.slug_assign(TICKET_A, &token));
    assert_eq!(first.code, ResultCode::Ok);
    let tip = world.branch_tip(TICKET_A);

    let assert_already_applied = |world: &World, request: &str, token: &str, what: &str| {
        let envelope = world.execute(request, world.slug_assign(TICKET_A, token));
        assert_eq!(
            (envelope.outcome, envelope.code, envelope.failure_class()),
            (Outcome::Noop, ResultCode::AlreadyApplied, None),
            "{what}"
        );
        assert_eq!(envelope.effects.write, WriteEffect::Unchanged, "{what}");
        assert_eq!(envelope.effects.commit_oid, None, "{what}");
        assert_eq!(
            ticket_data(&envelope).slug,
            Some(derived(TICKET_A)),
            "{what}"
        );
        assert!(world.record(request).is_none(), "{what}");
        assert_eq!(world.branch_tip(TICKET_A), tip, "{what}");
    };
    // Another request, late: with the token it first saw, and with the
    // token of the ticket as it now is.
    assert_already_applied(&world, REQUEST_2, &token, "the first token");
    assert_already_applied(&world, REQUEST_3, &world.token(TICKET_A), "a current token");

    // The same request after the cache was lost: its record is gone, and
    // it is a new request.
    world.lose_cache();
    world.rebuild();
    assert_already_applied(&world, REQUEST_1, &token, "after the cache was lost");
}

#[test]
fn a_stale_token_is_an_external_change_and_a_closed_ticket_is_refused() {
    let world = World::new();
    let stale = world.execute(REQUEST_1, world.slug_assign(TICKET_A, "v1:stale"));
    assert_eq!(
        (stale.outcome, stale.code, stale.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::ExternalChange,
            Some(FailureClass::Blocked)
        )
    );
    assert_eq!(stale.effects, Effects::not_requested());
    assert!(stale.recovery.is_empty());
    world.assert_nothing_exists(REQUEST_1, TICKET_A);

    let token = world.token(TICKET_CLOSED);
    let closed = world.execute(REQUEST_1, world.slug_assign(TICKET_CLOSED, &token));
    assert_eq!(
        (closed.outcome, closed.code, closed.failure_class()),
        (
            Outcome::Blocked,
            ResultCode::TicketClosed,
            Some(FailureClass::Blocked)
        )
    );
    assert_eq!(closed.effects, Effects::not_requested());
    world.assert_nothing_exists(REQUEST_1, TICKET_CLOSED);
    assert!(!world.primary_source(TICKET_CLOSED).contains("slug:"));
}

#[test]
fn a_retry_after_a_lost_result_reports_the_assigning_commit() {
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(data)
    });
    let token = world.token(TICKET_A);
    let lost = world.execute(REQUEST_1, world.slug_assign(TICKET_A, &token));
    assert_eq!(
        (lost.outcome, lost.code),
        (Outcome::Error, ResultCode::InternalError)
    );
    assert_eq!(lost.effects.commit_oid, None);
    assert_eq!(
        world.branch_commits(TICKET_A),
        1,
        "the assignment committed"
    );
    let commit = world.branch_tip(TICKET_A).unwrap();
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    // Another request saves the ticket in between: the short code stays,
    // and the newest commit is not the assigning one.
    world.reopen();
    let later = world.execute(REQUEST_2, world.save(TICKET_A, &world.token(TICKET_A)));
    assert_eq!(later.code, ResultCode::Ok);
    assert_eq!(world.branch_commits(TICKET_A), 2);

    // The initials have changed since: nothing is composed again.
    world.set_local_config("manyhands.initials", "QZ");
    let retry = world.execute(REQUEST_1, world.slug_assign(TICKET_A, &token));
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(retry.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(
        retry.operation_id,
        world
            .record(REQUEST_1)
            .map(|record| record.operations[0].operation_id.to_string())
    );
    assert_eq!(ticket_data(&retry).slug, Some(derived(TICKET_A)));
    assert_eq!(world.branch_commits(TICKET_A), 2, "no further commit");
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

#[test]
fn an_edit_made_under_an_interrupted_assignment_is_not_committed_as_its_own() {
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence).open_service(data)
    });
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.slug_assign(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Partial);
    assert_eq!(first.effects.checkpoint, CheckpointEffect::Pending);
    assert_eq!(world.slug(TICKET_A), Some(derived(TICKET_A)));

    // Someone edits the file in the context and does not commit.
    let path = world
        .worktree(TICKET_A)
        .join(crate::support::items::ticket_path(TICKET_A));
    let edited = world
        .worktree_source(TICKET_A)
        .replace("Body of", "Edited body of");
    std::fs::write(&path, &edited).unwrap();

    world.reopen();
    let retry = world.execute(REQUEST_1, world.slug_assign(TICKET_A, &token));
    // The first attempt's write is an effect of the request, so the
    // refusal is partial; the same request is not offered again.
    assert_eq!(
        (retry.outcome, retry.code),
        (Outcome::Partial, ResultCode::ExternalChange)
    );
    assert_eq!(retry.effects.checkpoint, CheckpointEffect::Pending);
    assert_eq!(retry.effects.commit_oid, None);
    assert!(recovery_actions(&retry).is_empty());
    assert_eq!(world.branch_commits(TICKET_A), 0, "nothing was committed");
    assert_eq!(
        world.worktree_source(TICKET_A),
        edited,
        "nothing was written"
    );
}
