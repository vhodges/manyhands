//! A Git repository that was never enabled: it has no Manyhands
//! configuration, in its working tree or in what its head commits. No
//! command enables it as a side effect. `index rebuild` and `index
//! refresh` refuse it and register nothing, and every command that needs
//! an enabled repository says that it is not enabled and offers nothing:
//! enabling is the user's to ask for.
//!
//! A repository that was enabled and whose registration went with the
//! index is another thing, and keeps its answer: `repository_not_registered`
//! with the rebuild that brings it back. `cache_loss.rs` and `index.rs`
//! hold those tests.

use std::fs;

use manyhands::{
    repository::{
        AuthoringKind, IndexRebuildInput, IndexRefreshInput, ItemSyncInput, Mutation,
        MutationDataDto, RebuildRepositoryRequest, RemoteAddInput, RemoteRemoveInput,
        RemoteSelectInput, RepoRemoveInput, RepoSyncInput,
    },
    results::{Effects, Envelope, FailureClass, Outcome, ResultCode},
};

use crate::support::{
    self, TestRepository,
    items::{self, TICKET_A},
    mutation::{
        DOCUMENT, DOCUMENT_MOVED_PATH, DOCUMENT_NEW, DOCUMENT_NEW_PATH, REQUEST_1, REQUEST_2,
        REQUEST_3, TICKET_NEW, World, recovery_actions,
    },
};

/// The code, as the contract spells it.
const NOT_ENABLED: &str = "repository_not_enabled";

/// A service that has an enabled repository registered, and a plain Git
/// repository beside it that was never enabled. The world's root is the
/// plain one, so every request the world builds names it.
fn never_enabled() -> (World, TestRepository) {
    let mut world = World::new();
    let plain = support::born_repository();
    world.root = plain.root.canonicalize().unwrap();
    (world, plain)
}

fn rebuild(world: &World) -> Mutation {
    Mutation::IndexRebuild(IndexRebuildInput {
        root: world.root.clone(),
    })
}

fn refresh(world: &World) -> Mutation {
    Mutation::IndexRefresh(IndexRefreshInput {
        root: world.root.clone(),
    })
}

fn count(world: &World, table: &str) -> i64 {
    items::index(world.data.path())
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn registered(world: &World) -> bool {
    world.service.resolve_repository(&world.root).is_ok()
}

/// Asserts that `envelope` says the repository is not enabled, offers
/// nothing, and that the request left nothing.
fn assert_not_enabled(world: &World, what: &str, envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.code.as_str(), NOT_ENABLED, "{what}");
    assert_eq!(
        (envelope.outcome, envelope.failure_class()),
        (Outcome::Error, Some(FailureClass::Input)),
        "{what}"
    );
    assert_eq!(
        envelope.code.message(),
        "The repository has never been enabled in Manyhands; enable it first.",
        "{what}"
    );
    assert_eq!(recovery_actions(envelope), [""; 0], "{what}");
    assert_eq!(envelope.effects, Effects::not_requested(), "{what}");
    assert_eq!(
        envelope.scope.repository.as_deref(),
        world.root.to_str(),
        "{what}"
    );
}

/// Every unconfirmed command that needs an enabled repository, as a
/// request of the world's root.
fn requests(world: &World) -> Vec<(&'static str, Mutation)> {
    vec![
        ("ticket save", world.save(TICKET_A, "v1:any")),
        ("ticket create", world.create(TICKET_NEW)),
        ("ticket slug-assign", world.slug_assign(TICKET_A, "v1:any")),
        (
            "document create",
            world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
        ),
        ("document save", world.document_save(DOCUMENT, "v1:any")),
        (
            "document move",
            world.document_move(DOCUMENT, "v1:any", DOCUMENT_MOVED_PATH, "v1:any"),
        ),
        (
            "remote add",
            Mutation::RemoteAdd(RemoteAddInput {
                root: world.root.clone(),
                name: "origin".to_owned(),
                url: "https://example.invalid/team/project.git".to_owned(),
            }),
        ),
        (
            "remote select",
            Mutation::RemoteSelect(RemoteSelectInput {
                root: world.root.clone(),
                name: None,
            }),
        ),
        ("index refresh", refresh(world)),
        (
            "repo sync",
            Mutation::RepoSync(RepoSyncInput {
                root: world.root.clone(),
            }),
        ),
        (
            "item sync",
            Mutation::ItemSync(ItemSyncInput {
                root: world.root.clone(),
                kind: AuthoringKind::Ticket,
                id: items::item_id(TICKET_A),
            }),
        ),
    ]
}

#[test]
fn a_rebuild_or_a_refresh_of_a_repository_that_was_never_enabled_is_refused_and_writes_nothing() {
    let (world, plain) = never_enabled();
    let before = support::repository_and_worktree_snapshot(&plain);
    let repositories = count(&world, "repositories");
    let operations = count(&world, "operation_records");
    type Request = fn(&World) -> Mutation;
    let commands: [(&str, Request); 2] = [("index rebuild", rebuild), ("index refresh", refresh)];
    for (command, request) in commands {
        let refused = world.execute(REQUEST_1, request(&world));
        assert_eq!(refused.command, command);
        assert_not_enabled(&world, command, &refused);
        assert_eq!(refused.operation_id, None, "{command}");
        // Not registered, and nothing recorded: no request, no operation.
        assert!(!registered(&world), "{command}");
        assert_eq!(count(&world, "repositories"), repositories, "{command}");
        assert_eq!(count(&world, "operation_records"), operations, "{command}");
        assert!(world.record(REQUEST_1).is_none(), "{command}");
        assert_eq!(world.request_rows(), 0, "{command}");
        // Nothing of the repository was written.
        assert!(
            before == support::repository_and_worktree_snapshot(&plain),
            "{command}"
        );
        assert!(!world.root.join(".manyhands").exists(), "{command}");
    }
}

#[test]
fn every_command_that_needs_an_enabled_repository_says_a_never_enabled_one_is_not_enabled() {
    let (world, _plain) = never_enabled();
    for (command, request) in requests(&world) {
        let refused = world.execute(REQUEST_1, request);
        assert_eq!(refused.command, command);
        assert_not_enabled(&world, command, &refused);
        // Refused before it was accepted: no operation was allotted.
        assert_eq!(refused.operation_id, None, "{command}");
        assert_eq!(world.request_rows(), 0, "{command}");
    }
    // The rebuild that used to be offered changes nothing of that: it is
    // refused, and the same request is answered as it was.
    assert_not_enabled(
        &world,
        "index rebuild",
        &world.execute(REQUEST_2, rebuild(&world)),
    );
    for (command, request) in requests(&world) {
        assert_not_enabled(&world, command, &world.execute(REQUEST_3, request));
    }
    assert!(!registered(&world));
    assert_eq!(world.request_rows(), 0);
}

/// A confirmed command that needs an enabled repository is refused its
/// preview.
#[test]
fn a_preview_of_a_remote_removal_in_a_never_enabled_repository_says_it_is_not_enabled() {
    let (world, _plain) = never_enabled();
    let preview = world
        .service
        .prepare(&Mutation::RemoteRemove(RemoteRemoveInput {
            root: world.root.clone(),
            name: "origin".to_owned(),
        }));
    assert_eq!(preview.code.as_str(), NOT_ENABLED);
    assert_eq!(preview.outcome, Outcome::Error);
    assert!(preview.recovery.is_empty());
    assert!(preview.data.is_none());
}

/// `repo remove` needs no enabled repository: a root that is not
/// registered is what it leaves, whether it was ever enabled or not.
#[test]
fn a_preview_of_repo_remove_for_a_never_enabled_repository_is_still_a_noop() {
    let (world, _plain) = never_enabled();
    let preview = world
        .service
        .prepare(&Mutation::RepoRemove(RepoRemoveInput {
            root: world.root.clone(),
        }));
    assert_eq!(
        (preview.outcome, preview.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
}

/// A request that would be refused is refused before anything is
/// replaced for it: the corrupt index is left as it is.
#[test]
fn a_rebuild_of_a_never_enabled_repository_does_not_replace_a_corrupt_database() {
    let (mut world, _plain) = never_enabled();
    let database = world.data.path().join("manyhands.sqlite3");
    for entry in fs::read_dir(world.data.path()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
            fs::remove_file(&path).unwrap();
        }
    }
    fs::write(&database, b"not sqlite").unwrap();
    world.reopen();

    let refused = world.execute(REQUEST_1, rebuild(&world));
    assert_eq!(refused.code.as_str(), NOT_ENABLED);
    assert_eq!(refused.outcome, Outcome::Error);
    assert!(refused.recovery.is_empty());
    assert_eq!(refused.operation_id, None);
    assert_eq!(fs::read(&database).unwrap(), b"not sqlite");

    // The world's own scan needs a database to read: the enabled
    // repository's rebuild brings one back.
    world.root = world.fixture.root.canonicalize().unwrap();
    let rebuilt = world.execute(REQUEST_2, rebuild(&world));
    assert_eq!(
        (rebuilt.outcome, rebuilt.code),
        (Outcome::Success, ResultCode::Ok)
    );
}

/// A repository that is registered and has no configuration: what an
/// earlier build's rebuild left of one that was never enabled. It is not
/// enabled either, and the rebuild that changes nothing is not offered.
#[test]
fn a_registered_repository_with_no_configuration_is_not_enabled_and_is_offered_nothing() {
    let (world, _plain) = never_enabled();
    world
        .service
        .rebuild_repository(RebuildRepositoryRequest {
            root: world.root.clone(),
            operation_id: support::new_operation_id(),
        })
        .unwrap();
    assert!(registered(&world));
    let requests = [
        ("ticket create", world.create(TICKET_NEW)),
        (
            "document create",
            world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
        ),
        (
            "repo sync",
            Mutation::RepoSync(RepoSyncInput {
                root: world.root.clone(),
            }),
        ),
    ];
    for (command, request) in requests {
        let refused = world.execute(REQUEST_1, request);
        assert_eq!(refused.command, command);
        assert_not_enabled(&world, command, &refused);
        // The domain refuses a document create once it is accepted; its
        // record is released with nothing in flight.
        assert!(world.record(REQUEST_1).is_none(), "{command}");
        assert_eq!(world.pending_journal_rows(), 0, "{command}");
    }
}
