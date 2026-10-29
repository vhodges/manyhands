//! The published contract of `execute`: each envelope against the envelope
//! schema, its data schema and its golden fixture under
//! `tests/fixtures/mutation_v1`.

use std::{collections::BTreeSet, fs, path::Path};

use manyhands::{
    repository::{
        AuthoringKind, CommitIdentity, DocumentSaveInput, FailurePoint, IndexRebuildInput,
        IndexRefreshInput, ItemSyncInput, KeyClearInput, KeyDeleteInput, KeyImportInput,
        KeyRemoveInput, KeySelectInput, Mutation, MutationDataDto, OperationAbandonInput,
        OperationId, OperationResumeInput, RelationshipWrite, RemoteAddInput, RemoteRemoveInput,
        RemoteSelectInput, RepoCreateInput, RepoRemoveInput, RepoSyncInput, SharedKeyId,
        TicketCreateInput, TicketSaveInput, TicketSlugAssignInput, request_store,
    },
    results::{Envelope, Outcome, ResultCode},
};
use serde_json::Value;

mod support;

use support::mutation::Previewed as _;
use support::{
    FailOnce,
    confirmation::Unregistered,
    golden::{self, ContractCase},
    items::TICKET_A,
    mutation::{
        self, BODY, DOCUMENT, DOCUMENT_MOVED_PATH, DOCUMENT_NEW, DOCUMENT_NEW_PATH, DOCUMENT_PATH,
        KeyWorld, REQUEST_1, REQUEST_2, SENTINELS, TICKET_NEW, TITLE, World,
    },
    schema,
};

const PROGRESS_SCHEMA: &str = "progress_event.schema.json";

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

/// A save of a ticket nothing holds, and a document made at a path that
/// is not written as one: refused before anything is accepted.
#[test]
fn a_request_that_names_no_item_or_no_path_matches_its_golden() {
    let world = World::new();
    let absent = world.execute(REQUEST_1, world.save(TICKET_NEW, "v1:any"));
    assert_eq!(absent.code, ResultCode::ItemNotFound);
    assert_ticket_contract("ticket_save_item_not_found", &world, &absent);

    let nowhere = world.execute(
        REQUEST_2,
        world.document_create(DOCUMENT_NEW, "docs/../new.md"),
    );
    assert_eq!(nowhere.code, ResultCode::InvalidPath);
    assert_document_contract("document_create_invalid_path", &world, &nowhere);
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

/// A synchronization needs a server for every result but one: a
/// repository with no publication remote is refused before anything is
/// recorded. The results a server gives are checked against the schemas
/// by the test target `mutation_remote`, where no fixture can be kept:
/// each names a host, a port and a commit that differ from run to run.
#[test]
fn a_synchronization_with_no_publication_remote_matches_the_envelope_schema_and_its_goldens() {
    let world = World::new();
    let repository = world.execute(
        REQUEST_1,
        Mutation::RepoSync(RepoSyncInput {
            root: world.root.clone(),
        }),
    );
    assert_eq!(repository.outcome, Outcome::Blocked);
    assert_eq!(repository.code, ResultCode::PublicationRemoteRequired);
    assert_contract("repo_sync_publication_remote_required", &world, &repository);

    let item = world.execute(
        REQUEST_2,
        Mutation::ItemSync(ItemSyncInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: support::items::item_id(TICKET_A),
        }),
    );
    assert_eq!(item.code, ResultCode::PublicationRemoteRequired);
    assert_contract("item_sync_publication_remote_required", &world, &item);
    assert!(world.record(REQUEST_1).is_none());
    assert!(world.record(REQUEST_2).is_none());
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

const KEY_SCHEMA: &str = "key_mutation.schema.json";

/// Checks a key command's envelope against its fixture, with the IDs of
/// the keys the fixture registered replaced: they differ from run to run.
/// A key command names no repository, so there is no root to replace.
fn assert_key_contract(
    name: &str,
    envelope: &Envelope<MutationDataDto>,
    keys: &[(&SharedKeyId, &str)],
) {
    let keys: Vec<(String, &str)> = keys
        .iter()
        .map(|(id, placeholder)| (id.to_string(), *placeholder))
        .collect();
    let operation = envelope.operation_id.clone();
    let mut placeholders: Vec<(&str, &str)> = keys
        .iter()
        .map(|(id, placeholder)| (id.as_str(), *placeholder))
        .collect();
    if let Some(operation) = operation.as_deref() {
        placeholders.push((operation, "<operation>"));
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: Some(KEY_SCHEMA),
            placeholders: &placeholders,
            sentinels: &SENTINELS,
        },
        envelope,
    );
}

#[test]
fn a_key_select_and_clear_match_their_schema_and_goldens() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let keys = [(&work, "<key>")];
    let select = || Mutation::KeySelect(KeySelectInput { id: work });
    let clear = || Mutation::KeyClear(KeyClearInput {});

    let selected = world.execute(REQUEST_1, select());
    assert_eq!(selected.outcome, Outcome::Success);
    assert_key_contract("key_select", &selected, &keys);

    let again = world.execute(REQUEST_2, select());
    assert_eq!(again.outcome, Outcome::Noop);
    assert_key_contract("key_select_noop", &again, &keys);

    let cleared = world.execute(support::mutation::REQUEST_3, clear());
    assert_eq!(cleared.outcome, Outcome::Success);
    assert_key_contract("key_clear", &cleared, &keys);

    let again = world.execute("01ARZ3NDEKTSV4RRFFQ69G5FX4", clear());
    assert_eq!(again.outcome, Outcome::Noop);
    assert_key_contract("key_clear_noop", &again, &keys);
}

#[test]
fn a_key_remove_matches_its_schema_and_goldens() {
    let world = KeyWorld::new();
    let work = world.register("work", "Work laptop");
    let unknown = SharedKeyId::new();
    let keys = [(&work, "<key>"), (&unknown, "<unknown>")];
    let remove = |id| Mutation::KeyRemove(KeyRemoveInput { id });

    world.service.select_shared_key(work).unwrap();
    let selected = world.execute(REQUEST_1, remove(work));
    assert_eq!(selected.code, ResultCode::SelectedKeyInUse);
    assert_key_contract("key_remove_selected", &selected, &keys);

    let not_found = world.execute(REQUEST_1, remove(unknown));
    assert_eq!(not_found.code, ResultCode::KeyNotFound);
    assert_key_contract("key_remove_not_found", &not_found, &keys);

    world.service.clear_shared_key_selection().unwrap();
    let removed = world.execute(REQUEST_1, remove(work));
    assert_eq!(removed.outcome, Outcome::Success);
    assert_key_contract("key_remove", &removed, &keys);
}

#[test]
fn a_key_import_matches_its_schema_and_goldens() {
    let world = KeyWorld::new();
    let (private, public) = world.key_files("work");
    let import = |label: &str, private: &std::path::Path| {
        Mutation::KeyImport(KeyImportInput {
            label: label.to_owned(),
            private_key_path: private.to_owned(),
            public_key_path: Some(public.clone()),
        })
    };

    let invalid = world.execute(
        REQUEST_1,
        import("Work laptop", std::path::Path::new("work")),
    );
    assert_eq!(invalid.code, ResultCode::InvalidInput);
    assert_key_contract("key_import_invalid", &invalid, &[]);

    let imported = world.execute(REQUEST_1, import("Work laptop", &private));
    assert_eq!(imported.outcome, Outcome::Success);
    let id = world.keys()[0].0;
    let keys = [(&id, "<key>")];
    assert_key_contract("key_import", &imported, &keys);

    let again = world.execute(REQUEST_2, import("Renamed", &private));
    assert_eq!(again.outcome, Outcome::Noop);
    assert_key_contract("key_import_noop", &again, &keys);

    // No result holds where the key's files are.
    let home = world.home.path().canonicalize().unwrap();
    for envelope in [&invalid, &imported, &again] {
        let text = serde_json::to_string(envelope).unwrap();
        assert!(!text.contains(home.to_str().unwrap()), "{text}");
    }
}

/// As `assert_key_contract`, for a generation: the fingerprint of the
/// key it made differs from run to run as well.
fn assert_generation_contract(name: &str, envelope: &Envelope<MutationDataDto>) {
    let data = mutation::key_data(envelope);
    let id = data.id.as_deref().map(|id| SharedKeyId::parse(id).unwrap());
    let fingerprint = data.fingerprint.clone();
    let operation = envelope.operation_id.clone();
    let id = id.map(|id| id.to_string());
    let mut placeholders: Vec<(&str, &str)> = Vec::new();
    if let Some(id) = id.as_deref() {
        placeholders.push((id, "<key>"));
    }
    if let Some(fingerprint) = fingerprint.as_deref() {
        placeholders.push((fingerprint, "<fingerprint>"));
    }
    if let Some(operation) = operation.as_deref() {
        placeholders.push((operation, "<operation>"));
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: Some(KEY_SCHEMA),
            placeholders: &placeholders,
            sentinels: &[mutation::PASSPHRASE_SENTINEL],
        },
        envelope,
    );
}

#[test]
fn a_key_generate_matches_its_schema_and_goldens() {
    let world = KeyWorld::new();
    let generate = |label: &str| world.generate(label, Some(mutation::PASSPHRASE));

    let invalid = world.execute(REQUEST_1, generate(""));
    assert_eq!(invalid.code, ResultCode::InvalidInput);
    assert_generation_contract("key_generate_invalid", &invalid);

    let generated = world.execute(REQUEST_1, generate("Work laptop"));
    assert_eq!(generated.outcome, Outcome::Success);
    assert_generation_contract("key_generate", &generated);
    // No result holds where the key's files are.
    let home = world.home.path().canonicalize().unwrap();
    let text = serde_json::to_string(&generated).unwrap();
    assert!(!text.contains(home.to_str().unwrap()), "{text}");
}

#[test]
fn an_interrupted_key_generate_matches_its_schema_and_goldens() {
    // Interrupted with its pair written: the same request registers it.
    let world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationAfterPublicWrite).open_service(data)
    });
    let interrupted = world.execute(
        REQUEST_1,
        world.generate("Work laptop", Some(mutation::PASSPHRASE)),
    );
    assert_eq!(interrupted.outcome, Outcome::Partial);
    assert_generation_contract("key_generate_interrupted", &interrupted);

    // Interrupted before its pair was complete: its retry is final.
    let mut world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationAfterPrivateWrite).open_service(data)
    });
    world.execute(
        REQUEST_1,
        world.generate("Work laptop", Some(mutation::PASSPHRASE)),
    );
    world.reopen();
    let retained = world.execute(
        REQUEST_1,
        world.generate("Work laptop", Some(mutation::PASSPHRASE)),
    );
    assert_eq!(retained.code, ResultCode::RecoveryRequired);
    assert_generation_contract("key_generate_retained", &retained);
}

const PREVIEW_SCHEMA: &str = "preview.schema.json";

/// Checks an envelope of a repository command, or of its preview,
/// against its fixture. What differs from run to run is replaced: the
/// root, the commit, and the operation and confirmation IDs.
fn assert_repository_contract<T: serde::Serialize>(
    name: &str,
    root: &std::path::Path,
    envelope: &Envelope<T>,
) {
    assert_confirmed_contract(name, root, PREVIEW_SCHEMA, &[], envelope);
}

/// The same for any command that needs confirmation: `data_schema` is
/// the schema of the data the envelope carries, when it carries any.
fn assert_confirmed_contract<T: serde::Serialize>(
    name: &str,
    root: &std::path::Path,
    data_schema: &str,
    sentinels: &[&str],
    envelope: &Envelope<T>,
) {
    assert_confirmed_contract_replacing(name, root, data_schema, sentinels, &[], envelope);
}

/// The same, with `replaced` added to what differs from run to run.
fn assert_confirmed_contract_replacing<T: serde::Serialize>(
    name: &str,
    root: &std::path::Path,
    data_schema: &str,
    sentinels: &[&str],
    replaced: &[(&str, &str)],
    envelope: &Envelope<T>,
) {
    let value = serde_json::to_value(envelope).unwrap();
    let text = |pointer: &str| value.pointer(pointer).and_then(|found| found.as_str());
    let mut placeholders = vec![(root.to_str().unwrap(), "<root>")];
    placeholders.extend_from_slice(replaced);
    for (pointer, placeholder) in [
        ("/effects/commit_oid", "<commit>"),
        ("/operation_id", "<operation>"),
        ("/data/confirmation_id", "<confirmation>"),
    ] {
        if let Some(found) = text(pointer) {
            placeholders.push((found, placeholder));
        }
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: (!value["data"].is_null()).then_some(data_schema),
            placeholders: &placeholders,
            sentinels,
        },
        envelope,
    );
}

#[test]
fn a_repo_enable_and_its_preview_match_their_schemas_and_goldens() {
    let world = Unregistered::new();

    let required = mutation::execute_with(&world.service, REQUEST_1, world.enable());
    assert_eq!(required.code, ResultCode::ConfirmationRequired);
    assert_repository_contract("repo_enable_confirmation_required", &world.root, &required);

    let stale = world.service.prepare(&world.enable()).previewed();
    assert_eq!(stale.outcome, Outcome::Success);
    assert_repository_contract("repo_enable_preview", &world.root, &stale);
    let stale = stale.data.unwrap().confirmation_id;
    std::fs::write(world.root.join("untracked.txt"), "new\n").unwrap();
    let changed = mutation::execute_confirmed(&world.service, REQUEST_1, &stale, world.enable());
    assert_eq!(changed.code, ResultCode::ExternalChange);
    assert_repository_contract("repo_enable_external_change", &world.root, &changed);
    std::fs::remove_file(world.root.join("untracked.txt")).unwrap();

    let enabled = mutation::execute_confirmed(&world.service, REQUEST_1, &stale, world.enable());
    assert_eq!(enabled.outcome, Outcome::Success);
    assert_repository_contract("repo_enable", &world.root, &enabled);

    let noop = world.service.prepare(&world.enable()).previewed();
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_repository_contract("repo_enable_preview_noop", &world.root, &noop);
    drop(world.fixture);
}

/// A repository whose worktree is in a merge that conflicted: its
/// enablement is not previewed, and nothing is offered.
#[test]
fn a_repo_enable_of_a_conflicted_worktree_matches_its_golden() {
    let world = Unregistered::new();
    support::conflict_primary_worktree(&world.fixture);

    let refused = world.service.prepare(&world.enable()).previewed();

    assert_eq!(refused.code, ResultCode::WorktreeConflicted);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_repository_contract("repo_enable_worktree_conflicted", &world.root, &refused);
    drop(world.fixture);
}

/// A repository that was never enabled: the rebuild that would have
/// registered it is refused, and so is a command that needs it enabled.
/// Neither offers an action.
#[test]
fn a_request_of_a_repository_that_was_never_enabled_matches_its_goldens() {
    let world = Unregistered::new();
    let rebuild = mutation::execute_with(
        &world.service,
        REQUEST_1,
        Mutation::IndexRebuild(IndexRebuildInput {
            root: world.root.clone(),
        }),
    );
    assert_eq!(rebuild.code, ResultCode::RepositoryNotEnabled);
    assert_repository_contract("index_rebuild_not_enabled", &world.root, &rebuild);

    let add = mutation::execute_with(
        &world.service,
        REQUEST_2,
        Mutation::RemoteAdd(RemoteAddInput {
            root: world.root.clone(),
            name: "origin".to_owned(),
            url: "https://example.invalid/team/project.git".to_owned(),
        }),
    );
    assert_eq!(add.code, ResultCode::RepositoryNotEnabled);
    assert_repository_contract("remote_add_not_enabled", &world.root, &add);
}

#[test]
fn a_repo_enable_whose_registration_is_pending_matches_its_golden() {
    let mut world = Unregistered::new();
    let confirmation = world
        .service
        .prepare(&world.enable())
        .previewed()
        .data
        .unwrap()
        .confirmation_id;
    world.fail_once_at(FailurePoint::BeforeRegistryWrite);
    let pending =
        mutation::execute_confirmed(&world.service, REQUEST_1, &confirmation, world.enable());
    assert_eq!(pending.outcome, Outcome::Partial);
    assert_repository_contract("repo_enable_registration_pending", &world.root, &pending);
}

#[test]
fn a_repo_create_and_its_preview_match_their_schemas_and_goldens() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().canonicalize().unwrap().join("new");
    let data = tempfile::tempdir().unwrap();
    let create = || {
        Mutation::RepoCreate(RepoCreateInput {
            root: root.clone(),
            primary_branch: "main".to_owned(),
            identity: Some(CommitIdentity {
                name: "Ada Example".to_owned(),
                email: "ada@example.invalid".to_owned(),
            }),
            slug_prefix: None,
        })
    };
    // The directory is there and empty, so that a failure after the
    // repository is initialized leaves it.
    std::fs::create_dir(&root).unwrap();
    let service = Unregistered::open(data.path(), None);
    let preview = service.prepare(&create()).previewed();
    assert_eq!(preview.outcome, Outcome::Success);
    assert_repository_contract("repo_create_preview", &root, &preview);
    let confirmation = preview.data.unwrap().confirmation_id;

    let service = Unregistered::open(
        data.path(),
        Some(FailurePoint::AfterRepositoryInitialization),
    );
    let partial = mutation::execute_confirmed(&service, REQUEST_1, &confirmation, create());
    assert_eq!(partial.outcome, Outcome::Partial);
    assert_repository_contract("repo_create_partial", &root, &partial);

    let service = Unregistered::open(data.path(), None);
    let created = mutation::execute_confirmed(&service, REQUEST_1, &confirmation, create());
    assert_eq!(created.outcome, Outcome::Success);
    assert_repository_contract("repo_create", &root, &created);
}

/// An enabled, registered repository under a clock that stands still,
/// so that a preview's expiry is fixed.
fn world_with_a_still_clock() -> World {
    World::with_service(|data| {
        manyhands::repository::RepositoryService::open_at(data)
            .unwrap()
            .with_clock_for_testing(mutation::TestClock::at(support::operations::STORED_AT))
    })
}

const REMOTE_REMOVAL_SCHEMA: &str = "remote_removal_preview.schema.json";

#[test]
fn a_remote_remove_and_its_preview_match_their_schemas_and_goldens() {
    let world = world_with_a_still_clock();
    world
        .fixture
        .repository
        .remote("origin", &remote_location())
        .unwrap();
    let remove = || {
        Mutation::RemoteRemove(RemoteRemoveInput {
            root: world.root.clone(),
            name: "origin".to_owned(),
        })
    };
    let secret = [REMOTE_SECRET];

    let preview = world.service.prepare(&remove()).previewed();
    assert_eq!(preview.outcome, Outcome::Success);
    assert_confirmed_contract(
        "remote_remove_preview",
        &world.root,
        REMOTE_REMOVAL_SCHEMA,
        &secret,
        &preview,
    );
    let confirmation = preview.data.unwrap().confirmation_id;

    let removed = mutation::execute_confirmed(&world.service, REQUEST_1, &confirmation, remove());
    assert_eq!(removed.outcome, Outcome::Success);
    assert_confirmed_contract(
        "remote_remove",
        &world.root,
        REMOTE_SCHEMA,
        &secret,
        &removed,
    );

    let noop = world.service.prepare(&remove()).previewed();
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_confirmed_contract(
        "remote_remove_preview_noop",
        &world.root,
        REMOTE_REMOVAL_SCHEMA,
        &secret,
        &noop,
    );
}

const REPOSITORY_REMOVAL_SCHEMA: &str = "repository_removal_preview.schema.json";

#[test]
fn a_repo_remove_and_its_preview_match_their_schemas_and_goldens() {
    let world = world_with_a_still_clock();
    let remove = || {
        Mutation::RepoRemove(RepoRemoveInput {
            root: world.root.clone(),
        })
    };

    let preview = world.service.prepare(&remove()).previewed();
    assert_eq!(preview.outcome, Outcome::Success);
    // When the repository was registered differs from run to run.
    let enabled_at = match &preview.data.as_ref().unwrap().details {
        manyhands::repository::PreviewDetailsDto::RepositoryRemoval(details) => {
            details.enabled_at.clone().unwrap()
        }
        other => panic!("not a preview of a registration's removal: {other:?}"),
    };
    assert_confirmed_contract_replacing(
        "repo_remove_preview",
        &world.root,
        REPOSITORY_REMOVAL_SCHEMA,
        &[],
        &[(&enabled_at, "<enabled-at>")],
        &preview,
    );
    let confirmation = preview.data.as_ref().unwrap().confirmation_id.clone();

    let removed = mutation::execute_confirmed(&world.service, REQUEST_1, &confirmation, remove());
    assert_eq!(removed.outcome, Outcome::Success);
    assert_confirmed_contract(
        "repo_remove",
        &world.root,
        REPOSITORY_REMOVAL_SCHEMA,
        &[],
        &removed,
    );

    let noop = world.service.prepare(&remove()).previewed();
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_confirmed_contract(
        "repo_remove_preview_noop",
        &world.root,
        REPOSITORY_REMOVAL_SCHEMA,
        &[],
        &noop,
    );
}

const OPERATION_ABANDON_SCHEMA: &str = "operation_abandon_preview.schema.json";

#[test]
fn a_repo_remove_missing_root_and_its_preview_match_the_existing_wire_contract() {
    for git_only in [false, true] {
        let world = world_with_a_still_clock();
        let displaced = tempfile::tempdir().unwrap();
        std::fs::rename(
            if git_only {
                world.root.join(".git")
            } else {
                world.root.clone()
            },
            displaced.path().join("moved"),
        )
        .unwrap();
        let remove = || {
            Mutation::RepoRemove(RepoRemoveInput {
                root: world.root.clone(),
            })
        };
        let preview = world.service.prepare(&remove()).previewed();
        assert_eq!(preview.outcome, Outcome::Success);
        let enabled_at = match &preview.data.as_ref().unwrap().details {
            manyhands::repository::PreviewDetailsDto::RepositoryRemoval(details) => {
                details.enabled_at.clone().unwrap()
            }
            other => panic!("not a removal preview: {other:?}"),
        };
        assert_confirmed_contract_replacing(
            "repo_remove_missing_preview",
            &world.root,
            REPOSITORY_REMOVAL_SCHEMA,
            &[],
            &[(&enabled_at, "<enabled-at>")],
            &preview,
        );
        let confirmation = preview.data.as_ref().unwrap().confirmation_id.clone();
        let removed =
            mutation::execute_confirmed(&world.service, REQUEST_1, &confirmation, remove());
        assert_eq!(removed.outcome, Outcome::Success);
        assert_confirmed_contract(
            "repo_remove_missing",
            &world.root,
            REPOSITORY_REMOVAL_SCHEMA,
            &[],
            &removed,
        );
    }
}
const OPERATION_SCHEMA: &str = "operation_mutation.schema.json";

#[test]
fn an_operation_abandon_and_its_preview_match_their_schemas_and_goldens() {
    let mut world = world_with_a_still_clock();
    // A save that wrote its file and did not record the step: its
    // operation is pending.
    let token = world.token(TICKET_A);
    world.service = FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
        .open_service(world.data.path());
    let stranded = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    let operation_id = stranded.operation_id.expect("the save's operation");
    world.service = manyhands::repository::RepositoryService::open_at(world.data.path())
        .unwrap()
        .with_clock_for_testing(mutation::TestClock::at(support::operations::STORED_AT));
    let abandon = |operation_id: &str| {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: world.root.clone(),
            operation_id: OperationId::parse(operation_id).unwrap(),
        })
    };
    let replaced = [(operation_id.as_str(), "<abandoned>")];

    let preview = world.service.prepare(&abandon(&operation_id)).previewed();
    assert_eq!(preview.outcome, Outcome::Success);
    assert_confirmed_contract_replacing(
        "operation_abandon_preview",
        &world.root,
        OPERATION_ABANDON_SCHEMA,
        &SENTINELS,
        &replaced,
        &preview,
    );
    let confirmation = preview.data.as_ref().unwrap().confirmation_id.clone();

    let abandoned = mutation::execute_confirmed(
        &world.service,
        REQUEST_2,
        &confirmation,
        abandon(&operation_id),
    );
    assert_eq!(abandoned.outcome, Outcome::Success);
    assert_confirmed_contract_replacing(
        "operation_abandon",
        &world.root,
        OPERATION_SCHEMA,
        &SENTINELS,
        &replaced,
        &abandoned,
    );

    let noop = world.service.prepare(&abandon(&operation_id)).previewed();
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_confirmed_contract_replacing(
        "operation_abandon_preview_noop",
        &world.root,
        OPERATION_ABANDON_SCHEMA,
        &SENTINELS,
        &replaced,
        &noop,
    );

    let unknown = world
        .service
        .prepare(&abandon("01ARZ3NDEKTSV4RRFFQ69G5FZZ"))
        .previewed();
    assert_eq!(unknown.code, ResultCode::OperationNotFound);
    assert_confirmed_contract(
        "operation_abandon_preview_not_found",
        &world.root,
        OPERATION_ABANDON_SCHEMA,
        &SENTINELS,
        &unknown,
    );
}

/// A resume reports no data: its target is the operation, which its
/// request names, and what it did is in its effects. The envelope of
/// one that took up a synchronization names a commit and can name a
/// host, and is checked against the schemas by the test target
/// `mutation_remote`.
#[test]
fn an_operation_resume_matches_the_envelope_schema_and_its_goldens() {
    let resume = |world: &World, operation_id: &str| {
        Mutation::OperationResume(OperationResumeInput {
            root: world.root.clone(),
            operation_id: OperationId::parse(operation_id).unwrap(),
        })
    };
    // A save whose index hand-off is owed: the resume runs it.
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let token = world.token(TICKET_A);
    let owed = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    let operation_id = owed.operation_id.expect("the save's operation");
    let resumed = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(resumed.outcome, Outcome::Success);
    assert_contract("operation_resume", &world, &resumed);

    // The operation has ended: nothing is owed.
    let finished = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(finished.outcome, Outcome::Success);
    let noop = world.execute("01ARZ3NDEKTSV4RRFFQ69G5FX3", resume(&world, &operation_id));
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_contract("operation_resume_noop", &world, &noop);

    let unknown = world.execute(
        "01ARZ3NDEKTSV4RRFFQ69G5FX3",
        resume(&world, "01ARZ3NDEKTSV4RRFFQ69G5FZZ"),
    );
    assert_eq!(unknown.code, ResultCode::OperationNotFound);
    assert_contract("operation_resume_not_found", &world, &unknown);

    // A save that stopped before its write: only its own request goes
    // on, and the result names it.
    let world =
        World::with_service(|data| FailOnce::at(FailurePoint::BeforeItemWrite).open_service(data));
    let token = world.token(TICKET_A);
    let interrupted = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    let operation_id = interrupted.operation_id.expect("the save's operation");
    let refused = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(refused.code, ResultCode::OriginalRequestRequired);
    assert_contract(
        "operation_resume_original_request_required",
        &world,
        &refused,
    );
}

const KEY_DELETION_SCHEMA: &str = "key_deletion_preview.schema.json";

#[test]
fn a_key_delete_and_its_preview_match_their_schemas_and_goldens() {
    let open = |data: &std::path::Path, point: Option<FailurePoint>| {
        match point {
            Some(point) => FailOnce::at(point).open_service(data),
            None => manyhands::repository::RepositoryService::open_at(data).unwrap(),
        }
        .with_clock_for_testing(mutation::TestClock::at(support::operations::STORED_AT))
    };
    let mut world = KeyWorld::with_service(|data| open(data, None));
    let generated = world.execute(
        REQUEST_2,
        world.generate("Work laptop", Some(mutation::PASSPHRASE)),
    );
    let data = mutation::key_data(&generated);
    let key = data.id.clone().unwrap();
    let fingerprint = data.fingerprint.clone().unwrap();
    let id = SharedKeyId::parse(&key).unwrap();
    let delete = |world: &KeyWorld| {
        Mutation::KeyDelete(KeyDeleteInput {
            store: world.store(),
            id,
        })
    };
    // What differs from run to run: the key, its fingerprint, its two
    // files, and the operation and confirmation IDs.
    let check = |name: &str, data_schema: &str, value: serde_json::Value| {
        let text = |pointer: &str| value.pointer(pointer).and_then(|found| found.as_str());
        let mut placeholders = vec![
            (key.as_str(), "<key>"),
            (fingerprint.as_str(), "<fingerprint>"),
        ];
        for (pointer, placeholder) in [
            ("/operation_id", "<operation>"),
            ("/data/confirmation_id", "<confirmation>"),
            ("/data/details/private_key_path", "<private-key-file>"),
            ("/data/details/public_key_path", "<public-key-file>"),
        ] {
            if let Some(found) = text(pointer) {
                placeholders.push((found, placeholder));
            }
        }
        mutation::assert_contract(
            &ContractCase {
                name,
                data_schema: Some(data_schema),
                placeholders: &placeholders,
                sentinels: &[mutation::PASSPHRASE_SENTINEL],
            },
            &value,
        );
    };

    let preview = world.service.prepare(&delete(&world)).previewed();
    assert_eq!(preview.outcome, Outcome::Success);
    check(
        "key_delete_preview",
        KEY_DELETION_SCHEMA,
        serde_json::to_value(&preview).unwrap(),
    );
    let confirmation = preview.data.unwrap().confirmation_id;

    // Interrupted with its private key file removed: the same request
    // completes the deletion.
    world.service = open(
        world.data.path(),
        Some(FailurePoint::DeletionAfterPrivatePhase),
    );
    let interrupted =
        mutation::execute_confirmed(&world.service, REQUEST_1, &confirmation, delete(&world));
    assert_eq!(interrupted.outcome, Outcome::Partial);
    check(
        "key_delete_interrupted",
        KEY_SCHEMA,
        serde_json::to_value(&interrupted).unwrap(),
    );

    world.service = open(world.data.path(), None);
    let deleted = mutation::execute_with(&world.service, REQUEST_1, delete(&world));
    assert_eq!(deleted.outcome, Outcome::Success);
    check(
        "key_delete",
        KEY_SCHEMA,
        serde_json::to_value(&deleted).unwrap(),
    );
}

// Completeness: what the contract owes as a whole, whatever any one command
// does. Each rule reads the registered lists and the files themselves, so a
// command, a schema, a fixture or a code that is added without its part of
// the contract fails here.

const READ_GOLDENS: &str = "read_v1";
const MUTATION_GOLDENS: &str = "mutation_v1";

fn file_names(directory: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn read_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Every registered golden envelope, of the reads and of the mutation
/// boundary, as `(directory/case, envelope)`.
fn golden_envelopes() -> Vec<(String, Value)> {
    let read = golden::CASES
        .iter()
        .map(|case| (READ_GOLDENS, golden::fixture_directory(), case));
    let mutation = mutation::CASES
        .iter()
        .map(|case| (MUTATION_GOLDENS, mutation::fixture_directory(), case));
    let cli = support::cli::CLI_CASES
        .iter()
        .map(|case| ("cli_v1", support::cli::cli_fixture_directory(), case));
    read.chain(mutation)
        .chain(cli)
        .map(|(set, directory, case)| {
            (
                format!("{set}/{case}"),
                read_json(&directory.join(format!("{case}.json"))),
            )
        })
        .collect()
}

/// What a set of fixtures and its registered cases differ by: a file no
/// case registers, and a case with no file.
fn fixture_differences(directory: &Path, cases: &[&str]) -> Vec<String> {
    let files: BTreeSet<String> = file_names(directory).into_iter().collect();
    let cases: BTreeSet<String> = cases.iter().map(|case| format!("{case}.json")).collect();
    let unregistered = files
        .difference(&cases)
        .map(|file| format!("{file}: a fixture no case registers"));
    let missing = cases
        .difference(&files)
        .map(|file| format!("{file}: a registered case with no fixture"));
    unregistered.chain(missing).collect()
}

#[test]
fn the_fixture_directories_are_exactly_the_registered_cases() {
    assert_eq!(
        fixture_differences(&mutation::fixture_directory(), mutation::CASES),
        [""; 0],
        "{MUTATION_GOLDENS}"
    );
    assert_eq!(
        fixture_differences(
            &mutation::progress_fixture_directory(),
            mutation::PROGRESS_CASES
        ),
        [""; 0],
        "progress_v1"
    );

    // Each is what the contract check would write, and what the published
    // schema describes: a fixture edited by hand fails here.
    for case in mutation::CASES {
        let path = mutation::fixture_directory().join(format!("{case}.json"));
        let text = fs::read_to_string(&path).unwrap();
        let value = read_json(&path);
        assert_eq!(golden::render(&value, &[]), text, "{path:?}");
        schema::check_published(golden::ENVELOPE_SCHEMA, &value).unwrap();
    }
    for case in mutation::PROGRESS_CASES {
        let path = mutation::progress_fixture(case);
        let text = fs::read_to_string(&path).unwrap();
        let value = read_json(&path);
        assert_eq!(golden::render(&value, &[]), text, "{path:?}");
        let events = value.as_array().unwrap();
        assert!(!events.is_empty(), "{path:?}");
        for event in events {
            schema::check_published(PROGRESS_SCHEMA, event).unwrap();
        }
    }
}

/// The commands a golden of the mutation boundary may be of that `execute`
/// does not bind, each with what answers it.
const UNBOUND_COMMANDS: [(&str, &str); 3] = [
    (
        "request show",
        "a read of the request store, which says what a request did",
    ),
    (
        "request cancel",
        "a signal to a running request, which records nothing of its own",
    ),
    (
        "host observe",
        "the observation of a host's key, which host approve consents to",
    ),
];

/// What the commands of the goldens and the commands the boundary answers
/// differ by.
fn command_differences(bound: &[&str], golden_commands: &BTreeSet<String>) -> Vec<String> {
    let without = bound
        .iter()
        .filter(|command| !golden_commands.contains(**command))
        .map(|command| format!("{command}: a bound command with no golden"));
    let unknown = golden_commands
        .iter()
        .filter(|command| {
            !bound.contains(&command.as_str())
                && !UNBOUND_COMMANDS
                    .iter()
                    .any(|(unbound, _)| unbound == command)
        })
        .map(|command| format!("{command}: a golden of a command the boundary does not answer"));
    let stale = UNBOUND_COMMANDS
        .iter()
        .filter(|(unbound, _)| bound.contains(unbound) || !golden_commands.contains(*unbound))
        .map(|(unbound, _)| format!("{unbound}: listed as unbound, and is bound or has no golden"));
    without.chain(unknown).chain(stale).collect()
}

#[test]
fn every_bound_command_has_a_golden() {
    let golden_commands: BTreeSet<String> = mutation::CASES
        .iter()
        .map(|case| {
            let path = mutation::fixture_directory().join(format!("{case}.json"));
            read_json(&path)["command"].as_str().unwrap().to_owned()
        })
        .collect();
    let bound = request_store::bound_commands();

    assert_eq!(command_differences(&bound, &golden_commands), [""; 0]);
}

/// The schemas that describe the `data` of an answer: a read's, a
/// command's, a preview's. With the envelope and the progress event they are
/// the roots the contract publishes; every other schema is a part of one.
const DATA_ROOTS: [&str; 52] = [
    "local_context_sync_mutation.schema.json",
    "local_context_sync_inspection.schema.json",
    "local_context_conflict_content.schema.json",
    "local_context_sync_observation.schema.json",
    "cancel.schema.json",
    "cli_help.schema.json",
    "cli_version.schema.json",
    "comment_mutation.schema.json",
    "comment_list.schema.json",
    "cycle_list.schema.json",
    "dependency_tree.schema.json",
    "document_folder_list.schema.json",
    "document_mutation.schema.json",
    "host_mutation.schema.json",
    "host_observation.schema.json",
    "host_pin.schema.json",
    "host_pin_list.schema.json",
    "host_preview.schema.json",
    "host_replacement.schema.json",
    "identity.schema.json",
    "identity_mutation.schema.json",
    "identity_preview.schema.json",
    "index_status.schema.json",
    "item.schema.json",
    "item_list.schema.json",
    "key.schema.json",
    "key_deletion_preview.schema.json",
    "key_list.schema.json",
    "key_mutation.schema.json",
    "new_id.schema.json",
    "operation.schema.json",
    "operation_abandon_preview.schema.json",
    "operation_list.schema.json",
    "operation_mutation.schema.json",
    "path_observation.schema.json",
    "plan.schema.json",
    "polling_status.schema.json",
    "preview.schema.json",
    "public_key.schema.json",
    "relationship_check.schema.json",
    "remote_list.schema.json",
    "remote_mutation.schema.json",
    "remote_removal_preview.schema.json",
    "repair_mutation.schema.json",
    "repair_preview.schema.json",
    "repository_inspection.schema.json",
    "repository_list.schema.json",
    "repository_removal_preview.schema.json",
    "request.schema.json",
    "synchronization_conflict_mutation.schema.json",
    "synchronization_mutation.schema.json",
    "ticket_mutation.schema.json",
];

/// The roots that describe no `data`: every envelope, and every event a
/// command reports while it runs.
const OTHER_ROOTS: [&str; 2] = [golden::ENVELOPE_SCHEMA, PROGRESS_SCHEMA];

/// Adds the files `schema` refers to.
fn references(schema: &Value, found: &mut BTreeSet<String>) {
    match schema {
        Value::Object(object) => {
            for (keyword, value) in object {
                match value.as_str() {
                    Some(file) if keyword == "$ref" => {
                        found.insert(file.to_owned());
                    }
                    _ => references(value, found),
                }
            }
        }
        Value::Array(values) => values.iter().for_each(|value| references(value, found)),
        _ => {}
    }
}

/// What is wrong with the schema files of `directory`, given its roots: a
/// root that is not there, a reference to a file that is not there, and a
/// file no root reaches.
fn schema_differences(directory: &Path, roots: &[&str]) -> Vec<String> {
    let files: BTreeSet<String> = file_names(directory).into_iter().collect();
    let mut problems = Vec::new();
    let mut reached = BTreeSet::new();
    let mut pending: Vec<String> = Vec::new();
    for root in roots {
        if files.contains(*root) {
            pending.push((*root).to_owned());
        } else {
            problems.push(format!("{root}: a root that is not published"));
        }
    }
    while let Some(file) = pending.pop() {
        if !reached.insert(file.clone()) {
            continue;
        }
        let mut referred = BTreeSet::new();
        references(&read_json(&directory.join(&file)), &mut referred);
        for target in referred {
            if files.contains(&target) {
                pending.push(target);
            } else {
                problems.push(format!(
                    "{file}: refers to {target}, which is not published"
                ));
            }
        }
    }
    problems.extend(
        files
            .difference(&reached)
            .map(|file| format!("{file}: no root reaches it")),
    );
    problems
}

#[test]
fn every_schema_is_reachable_from_a_published_root() {
    let roots: Vec<&str> = DATA_ROOTS.into_iter().chain(OTHER_ROOTS).collect();
    assert_eq!(
        schema_differences(&schema::schema_directory(), &roots),
        [""; 0]
    );
}

/// What the data of the goldens and the data roots differ by: data that is
/// not exactly one root's, and a root no golden carries the data of.
fn data_root_differences(goldens: &[(String, Value)], roots: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut carried = BTreeSet::new();
    for (name, envelope) in goldens {
        let data = &envelope["data"];
        if data.is_null() {
            continue;
        }
        let described: Vec<&str> = roots
            .iter()
            .copied()
            .filter(|root| schema::check_published(root, data).is_ok())
            .collect();
        match described.as_slice() {
            [root] => {
                carried.insert(*root);
            }
            _ => problems.push(format!(
                "{name}: its data is described by {described:?}, not by one root"
            )),
        }
    }
    problems.extend(
        roots
            .iter()
            .filter(|root| !carried.contains(*root))
            .map(|root| format!("{root}: a root no golden carries the data of")),
    );
    problems
}

/// A root is a root because an answer carries it, and the goldens are the
/// answers: so a schema cannot be listed as a root to make it reachable.
#[test]
fn every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root() {
    assert_eq!(
        data_root_differences(&golden_envelopes(), &DATA_ROOTS),
        [""; 0]
    );
}

/// The result codes no fixture can produce, each with the reason. Every
/// other code is the code of a golden envelope.
const CODES_NO_FIXTURE_CAN_PRODUCE: [(ResultCode, &str); 2] = [
    (
        ResultCode::RemoteUnavailable,
        "the transport gives it only when libgit2 says the remote is not found, and the test \
         server answers a path it does not serve by closing the connection, which is \
         transport_unavailable; no test of the transport produces it either",
    ),
    (
        ResultCode::MergeRequired,
        "item sync and repo sync merge divergent history (Wave 03 C4, R-16), so no command \
         of execute or prepare returns it; it stays a code of the domain for a caller set to \
         refuse divergence, which no binding is",
    ),
];

/// What the codes of the goldens and the published codes differ by: a code
/// with neither a golden nor a reason, and a reason for a code that has a
/// golden.
fn code_differences(
    golden_codes: &BTreeSet<String>,
    excused: &[(ResultCode, &str)],
) -> Vec<String> {
    let mut problems = Vec::new();
    for code in ResultCode::ALL {
        let name = code.as_str();
        let reason = excused.iter().find(|(excused, _)| *excused == code);
        match (golden_codes.contains(name), reason) {
            (true, None) => {}
            (false, Some((_, reason))) => {
                assert!(!reason.trim().is_empty(), "{name}: a reason is required");
            }
            (false, None) => problems.push(format!(
                "{name}: no golden has this code, and no reason is given"
            )),
            (true, Some(_)) => problems.push(format!(
                "{name}: listed as a code no fixture can produce, and a golden has it"
            )),
        }
    }
    problems
}

/// The result codes no golden of the mutation boundary has, each with the
/// reason: a code only a read returns, or one no fixture of a mutation
/// can produce. Every other code is the code of a golden under
/// `tests/fixtures/mutation_v1`, so that a front end can see what the
/// boundary's own envelope of it looks like, with the actions it offers.
const CODES_NO_MUTATION_GOLDEN_HAS: [(ResultCode, &str); 7] = [
    (
        ResultCode::UsageError,
        "a CLI parse result; no command of execute or prepare returns it",
    ),
    (
        ResultCode::CommandUnavailable,
        "a CLI parse result; no command of execute or prepare returns it",
    ),
    (
        ResultCode::RemoteUnavailable,
        "no fixture can produce it; see CODES_NO_FIXTURE_CAN_PRODUCE",
    ),
    (
        ResultCode::MergeRequired,
        "no command of execute or prepare returns it since item sync and repo sync merge \
         divergent history; see CODES_NO_FIXTURE_CAN_PRODUCE",
    ),
    (
        ResultCode::InvalidId,
        "a read's code: an ID that is not written as one. A mutation takes its IDs as typed \
         values, and no command of execute or prepare returns it",
    ),
    (
        ResultCode::PublicKeyUnavailable,
        "a read's code, of key public: no command of execute or prepare returns it",
    ),
    (
        ResultCode::AuthorityNotFound,
        "a read's code, of host inspect: no command of execute or prepare returns it",
    ),
];

#[test]
fn every_result_code_has_a_mutation_golden_or_a_reason_none_has_it() {
    let mutation_codes: BTreeSet<String> = golden_envelopes()
        .iter()
        .filter(|(name, _)| name.starts_with(MUTATION_GOLDENS))
        .map(|(_, envelope)| envelope["code"].as_str().unwrap().to_owned())
        .collect();

    assert_eq!(
        code_differences(&mutation_codes, &CODES_NO_MUTATION_GOLDEN_HAS),
        [""; 0]
    );
}

#[test]
fn every_result_code_has_a_golden_or_a_reason_no_fixture_can_produce_it() {
    let golden_codes: BTreeSet<String> = golden_envelopes()
        .iter()
        .map(|(_, envelope)| envelope["code"].as_str().unwrap().to_owned())
        .collect();

    assert_eq!(
        code_differences(&golden_codes, &CODES_NO_FIXTURE_CAN_PRODUCE),
        [""; 0]
    );
}
