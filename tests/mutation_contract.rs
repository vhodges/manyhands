//! The published contract of `execute`: each envelope against the envelope
//! schema, its data schema and its golden fixture under
//! `tests/fixtures/mutation_v1`.

use manyhands::{
    repository::{
        FailurePoint, Mutation, MutationDataDto, RelationshipWrite, TicketCreateInput,
        TicketSaveInput,
    },
    results::{Envelope, Outcome, ResultCode},
};

mod support;

use support::{
    FailOnce,
    golden::ContractCase,
    items::TICKET_A,
    mutation::{self, BODY, REQUEST_1, REQUEST_2, SENTINELS, TICKET_NEW, TITLE, World},
};

const TICKET_SCHEMA: &str = "ticket_mutation.schema.json";

/// Checks a ticket envelope against its fixture, with what differs from
/// run to run replaced: the repository's path, the commit and the
/// operation ID the boundary allocated.
fn assert_ticket_contract(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    let root = world.root.to_str().unwrap().to_owned();
    let commit = envelope.effects.commit_oid.clone();
    let operation = envelope.operation_id.clone();
    let mut placeholders = vec![(root.as_str(), "<root>")];
    if let Some(commit) = commit.as_deref() {
        placeholders.push((commit, "<commit>"));
    }
    if let Some(operation) = operation.as_deref() {
        placeholders.push((operation, "<operation>"));
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: envelope.data.as_ref().map(|_| TICKET_SCHEMA),
            placeholders: &placeholders,
            sentinels: &SENTINELS,
        },
        envelope,
    );
}

#[test]
fn a_ticket_save_matches_its_schema_and_golden() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_ticket_contract("ticket_save", &world, &envelope);

    // The replay of the finished request is the same envelope.
    let replayed = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(replayed, envelope);
    assert_ticket_contract("ticket_save_replay", &world, &replayed);

    // The same request ID with another body.
    let mismatch = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft(TITLE, "Another body.\n"),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_ticket_contract("ticket_save_mismatch", &world, &mismatch);
}

#[test]
fn a_ticket_create_matches_its_schema_and_golden() {
    let world = World::new();
    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketCreate(TicketCreateInput {
            deps: vec![
                TICKET_A.to_owned(),
                support::items::TICKET_ABSENT.to_owned(),
            ],
            parent: Some(TICKET_A.to_owned()),
            ..world.create_input(TICKET_NEW)
        }),
    );
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_ticket_contract("ticket_create", &world, &envelope);
}

#[test]
fn a_partial_ticket_save_matches_its_schema_and_golden() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence).open_service(data)
    });
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(envelope.outcome, Outcome::Partial);
    assert_ticket_contract("ticket_save_partial", &world, &envelope);

    // The same request ID with another body, after that partial result.
    let mismatch = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::draft(TITLE, "Another body.\n"),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(mismatch.outcome, Outcome::Partial);
    assert_ticket_contract("ticket_save_mismatch_partial", &world, &mismatch);
}

#[test]
fn a_ticket_save_whose_discovery_is_pending_matches_its_schema_and_golden() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(envelope.code, ResultCode::DiscoveryPending);
    assert_ticket_contract("ticket_save_discovery_pending", &world, &envelope);
}

#[test]
fn a_rejected_cycle_matches_its_schema_and_golden() {
    let world = World::new();
    let created = world.execute(
        REQUEST_1,
        Mutation::TicketCreate(TicketCreateInput {
            deps: vec![TICKET_A.to_owned()],
            ..world.create_input(TICKET_NEW)
        }),
    );
    assert_eq!(created.outcome, Outcome::Success);
    let token = world.token(TICKET_A);
    let envelope = world.execute(
        REQUEST_2,
        Mutation::TicketSave(TicketSaveInput {
            deps: RelationshipWrite::Set(vec![TICKET_NEW.to_owned()]),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(envelope.code, ResultCode::RelationshipCycle);
    assert_ticket_contract("ticket_save_cycle", &world, &envelope);
}

#[test]
fn a_ticket_save_that_changes_nothing_matches_its_schema_and_golden() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let envelope = world.execute(
        REQUEST_1,
        Mutation::TicketSave(TicketSaveInput {
            draft: World::unchanged_draft(),
            ..world.save_input(TICKET_A, &token)
        }),
    );
    assert_eq!(envelope.outcome, Outcome::Noop);
    assert_ticket_contract("ticket_save_noop", &world, &envelope);
    // Nothing of what the other scenarios save is in this one.
    assert!(!world.worktree_source(TICKET_A).contains(BODY));
}

#[test]
fn a_ticket_request_stopped_before_it_is_accepted_carries_no_data() {
    let world = World::new();
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, "v1:stale"));
    assert_eq!(envelope.code, ResultCode::ExternalChange);
    assert_ticket_contract("ticket_save_external_change", &world, &envelope);
}

#[test]
fn a_re_entered_ticket_save_matches_its_schema_and_golden() {
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence).open_service(data)
    });
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Partial);

    // The same request again finishes what the first attempt left.
    world.reopen();
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.operation_id, first.operation_id);
    assert_ticket_contract("ticket_save_reentered", &world, &envelope);
}

#[test]
fn a_re_entered_ticket_save_stopped_by_a_foreign_commit_matches_its_schema_and_golden() {
    let path = support::items::ticket_path(TICKET_A);
    let foreign = support::items::ticket_source(TICKET_A, "Changed by someone else", "");

    // The first attempt wrote and did not commit.
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeCheckpointCommit).open_service(data)
    });
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.outcome, Outcome::Partial);
    world.commit_in_context(TICKET_A, &path, &foreign);
    world.reopen();
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Blocked, ResultCode::ExternalChange)
    );
    assert_ticket_contract("ticket_save_reentered_external_change", &world, &envelope);

    // The first attempt committed, and the foreign commit is on top.
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let token = world.token(TICKET_A);
    let first = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(first.code, ResultCode::DiscoveryPending);
    world.commit_in_context(TICKET_A, &path, &foreign);
    world.reopen();
    let envelope = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Partial, ResultCode::ExternalChange)
    );
    assert_eq!(envelope.effects.commit_oid, first.effects.commit_oid);
    assert_ticket_contract("ticket_save_reentered_superseded", &world, &envelope);
}

#[test]
fn a_ticket_create_whose_item_already_exists_matches_its_schema_and_golden() {
    let world = World::new();
    let created = world.execute(REQUEST_1, world.create(TICKET_NEW));
    assert_eq!(created.outcome, Outcome::Success);

    // Another request for the same ticket: what it asks for already holds.
    let envelope = world.execute(REQUEST_2, world.create(TICKET_NEW));
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_ticket_contract("ticket_create_already_applied", &world, &envelope);
}
