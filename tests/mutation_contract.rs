//! The published contract of `execute`: each envelope against the envelope
//! schema, its data schema and its golden fixture under
//! `tests/fixtures/mutation_v1`.

use manyhands::{
    repository::{
        DocumentSaveInput, FailurePoint, IndexRebuildInput, IndexRefreshInput, Mutation,
        MutationDataDto, RelationshipWrite, RemoteAddInput, RemoteSelectInput, TicketCreateInput,
        TicketSaveInput, TicketSlugAssignInput,
    },
    results::{Envelope, Outcome, ResultCode},
};

mod support;

use support::{
    FailOnce,
    golden::ContractCase,
    items::TICKET_A,
    mutation::{
        self, BODY, DOCUMENT, DOCUMENT_MOVED_PATH, DOCUMENT_NEW, DOCUMENT_NEW_PATH, DOCUMENT_PATH,
        REQUEST_1, REQUEST_2, SENTINELS, TICKET_NEW, TITLE, World,
    },
};

const TICKET_SCHEMA: &str = "ticket_mutation.schema.json";
const DOCUMENT_SCHEMA: &str = "document_mutation.schema.json";

/// Checks a ticket envelope against its fixture, with what differs from
/// run to run replaced: the repository's path, the commit and the
/// operation ID the boundary allocated.
fn assert_ticket_contract(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    assert_data_contract(name, TICKET_SCHEMA, world, envelope);
}

/// As `assert_ticket_contract`, for a document's envelope.
fn assert_document_contract(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    assert_data_contract(name, DOCUMENT_SCHEMA, world, envelope);
}

fn assert_data_contract(
    name: &str,
    schema: &str,
    world: &World,
    envelope: &Envelope<MutationDataDto>,
) {
    assert_contract_excluding(name, schema, world, envelope, &SENTINELS);
}

/// As `assert_data_contract`, for an envelope that may hold none of
/// `sentinels`.
fn assert_contract_excluding(
    name: &str,
    schema: &str,
    world: &World,
    envelope: &Envelope<MutationDataDto>,
    sentinels: &[&str],
) {
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
            data_schema: envelope.data.as_ref().map(|_| schema),
            placeholders: &placeholders,
            sentinels,
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

#[test]
fn a_short_code_assignment_matches_its_schema_and_goldens() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let envelope = world.execute(REQUEST_1, world.slug_assign(TICKET_A, &token));
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_ticket_contract("ticket_slug_assign", &world, &envelope);

    // Another request asks for another code: the ticket has one.
    let envelope = world.execute(
        REQUEST_2,
        Mutation::TicketSlugAssign(TicketSlugAssignInput {
            initials: Some("zz".to_owned()),
            ..world.slug_assign_input(TICKET_A, &world.token(TICKET_A))
        }),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Error, ResultCode::SlugAlreadyAssigned)
    );
    assert_ticket_contract("ticket_slug_assign_already_assigned", &world, &envelope);
}

#[test]
fn a_document_create_matches_its_schema_and_goldens() {
    let world = World::new();
    let envelope = world.execute(
        REQUEST_1,
        world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_document_contract("document_create", &world, &envelope);

    // A create at a path a document is at.
    let envelope = world.execute(
        REQUEST_2,
        world.document_create(DOCUMENT_NEW, DOCUMENT_PATH),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Error, ResultCode::OccupiedPath)
    );
    assert_document_contract("document_create_occupied", &world, &envelope);
}

#[test]
fn a_document_save_matches_its_schema_and_goldens() {
    let world = World::new();
    let token = world.token(DOCUMENT);

    // Without the caller's observation nothing is read and nothing is
    // said of the document.
    let envelope = world.execute(
        REQUEST_1,
        Mutation::DocumentSave(DocumentSaveInput {
            observation: None,
            ..world.document_save_input(DOCUMENT, &token)
        }),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Error, ResultCode::InvalidInput)
    );
    assert_eq!(envelope.data, None);
    assert_document_contract("document_save_invalid_input", &world, &envelope);

    let envelope = world.execute(REQUEST_1, world.document_save(DOCUMENT, &token));
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_document_contract("document_save", &world, &envelope);
}

#[test]
fn a_document_move_matches_its_schema_and_goldens() {
    let world = World::new();
    let token = world.token(DOCUMENT);
    let absent = world.observe(DOCUMENT, DOCUMENT_MOVED_PATH);

    // The destination is not as the caller says it saw it.
    let envelope = world.execute(
        REQUEST_1,
        world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, "v1:stale"),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Blocked, ResultCode::ExternalChange)
    );
    assert_document_contract("document_move_external_change", &world, &envelope);

    let envelope = world.execute(
        REQUEST_1,
        world.document_move(DOCUMENT, &token, DOCUMENT_MOVED_PATH, &absent),
    );
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_document_contract("document_move", &world, &envelope);
}

/// As `assert_ticket_contract`, for a command that reports no data.
fn assert_contract(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.data, None, "{name}");
    assert_data_contract(name, "", world, envelope);
}

fn index_refresh(world: &World) -> Mutation {
    Mutation::IndexRefresh(IndexRefreshInput {
        root: world.root.clone(),
    })
}

fn index_rebuild(world: &World) -> Mutation {
    Mutation::IndexRebuild(IndexRebuildInput {
        root: world.root.clone(),
    })
}

#[test]
fn an_index_refresh_and_rebuild_match_the_envelope_schema_and_their_goldens() {
    let world = World::new();
    let refreshed = world.execute(REQUEST_1, index_refresh(&world));
    assert_eq!(refreshed.outcome, Outcome::Success);
    assert_contract("index_refresh", &world, &refreshed);

    let rebuilt = world.execute(REQUEST_2, index_rebuild(&world));
    assert_eq!(rebuilt.outcome, Outcome::Success);
    assert_contract("index_rebuild", &world, &rebuilt);

    // The request ID of the refresh, for the other command.
    let mismatch = world.execute(REQUEST_1, index_rebuild(&world));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_contract("index_refresh_mismatch", &world, &mismatch);
}

#[test]
fn a_failed_index_refresh_matches_the_envelope_schema_and_its_golden() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let failed = world.execute(REQUEST_1, index_refresh(&world));
    assert_eq!(failed.outcome, Outcome::Error);
    assert_contract("index_refresh_failed", &world, &failed);
}

const REMOTE_SCHEMA: &str = "remote_mutation.schema.json";
/// A password in a remote's location, which no envelope may hold.
const REMOTE_SECRET: &str = "SENTINEL-PASSWORD-3b7c";

fn remote_location() -> String {
    format!("https://deploy:{REMOTE_SECRET}@example.invalid/team/project.git")
}

fn remote_add(world: &World, name: &str, url: &str) -> Mutation {
    Mutation::RemoteAdd(RemoteAddInput {
        root: world.root.clone(),
        name: name.to_owned(),
        url: url.to_owned(),
    })
}

fn assert_remote_contract(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    assert_contract_excluding(name, REMOTE_SCHEMA, world, envelope, &[REMOTE_SECRET]);
}

#[test]
fn a_remote_add_matches_its_schema_and_goldens() {
    let world = World::new();
    let added = world.execute(REQUEST_1, remote_add(&world, "origin", &remote_location()));
    assert_eq!(added.outcome, Outcome::Success);
    assert_remote_contract("remote_add", &world, &added);

    let again = world.execute(REQUEST_2, remote_add(&world, "origin", &remote_location()));
    assert_eq!(again.outcome, Outcome::Noop);
    assert_remote_contract("remote_add_noop", &world, &again);

    let conflict = world.execute(
        support::mutation::REQUEST_3,
        remote_add(&world, "origin", "git@example.invalid:team/other.git"),
    );
    assert_eq!(conflict.code, ResultCode::RemoteNameConflict);
    assert_remote_contract("remote_add_conflict", &world, &conflict);

    let invalid = world.execute(
        support::mutation::REQUEST_3,
        remote_add(&world, "not a name", &remote_location()),
    );
    assert_eq!(invalid.code, ResultCode::InvalidRemote);
    assert_remote_contract("remote_add_invalid", &world, &invalid);
}

#[test]
fn a_remote_add_whose_discovery_is_pending_matches_its_schema_and_golden() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let pending = world.execute(REQUEST_1, remote_add(&world, "origin", &remote_location()));
    assert_eq!(pending.outcome, Outcome::Partial);
    assert_remote_contract("remote_add_discovery_pending", &world, &pending);
}

fn remote_select(world: &World, name: Option<&str>) -> Mutation {
    Mutation::RemoteSelect(RemoteSelectInput {
        root: world.root.clone(),
        name: name.map(str::to_owned),
    })
}

/// Adds a remote that can be published to, as `git remote add` would.
fn add_origin(world: &World) {
    git2::Repository::open(&world.root)
        .unwrap()
        .remote("origin", "git@example.invalid:team/project.git")
        .unwrap();
}

#[test]
fn a_remote_select_matches_its_schema_and_goldens() {
    let world = World::new();
    add_origin(&world);
    let selected = world.execute(REQUEST_1, remote_select(&world, Some("origin")));
    assert_eq!(selected.outcome, Outcome::Success);
    assert_remote_contract("remote_select", &world, &selected);

    let again = world.execute(REQUEST_2, remote_select(&world, Some("origin")));
    assert_eq!(again.outcome, Outcome::Noop);
    assert_remote_contract("remote_select_noop", &world, &again);

    let invalid = world.execute(
        support::mutation::REQUEST_3,
        remote_select(&world, Some("absent")),
    );
    assert_eq!(invalid.code, ResultCode::InvalidRemote);
    assert_remote_contract("remote_select_invalid", &world, &invalid);

    let cleared = world.execute(support::mutation::REQUEST_3, remote_select(&world, None));
    assert_eq!(cleared.outcome, Outcome::Success);
    assert_remote_contract("remote_select_cleared", &world, &cleared);
}

#[test]
fn a_remote_select_whose_discovery_is_pending_matches_its_schema_and_golden() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    add_origin(&world);
    let pending = world.execute(REQUEST_1, remote_select(&world, Some("origin")));
    assert_eq!(pending.outcome, Outcome::Partial);
    assert_remote_contract("remote_select_discovery_pending", &world, &pending);
}
