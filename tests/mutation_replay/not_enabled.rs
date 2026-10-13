//! A Git repository that was never enabled: it is not registered, and it
//! has no Manyhands configuration in its working tree, in what its head
//! commits, or at the tip of any local branch. No
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
        AuthoringKind, FailurePoint, IndexRebuildInput, IndexRefreshInput, ItemSyncInput, Mutation,
        MutationDataDto, RebuildRepositoryRequest, RemoteAddInput, RemoteRemoveInput,
        RemoteSelectInput, RepoRemoveInput, RepoSyncInput,
    },
    results::{Effects, Envelope, FailureClass, Outcome, ResultCode},
};

use crate::support::mutation::Previewed as _;
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
        }))
        .previewed();
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
        }))
        .previewed();
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

/// A repository that is registered and has no configuration at its root:
/// what an earlier build's rebuild left of a plain repository, and also
/// what an enabled repository shows when its root is on a branch from
/// before it was enabled. Whether it was ever enabled is not asked of a
/// repository that is registered: it keeps the answers it had, and the
/// synchronization keeps the action that shows what is missing.
#[test]
fn a_registered_repository_with_no_configuration_keeps_the_answers_it_had() {
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
        (
            "ticket create",
            world.create(TICKET_NEW),
            ResultCode::RepositoryNotRegistered,
            "index.rebuild",
        ),
        (
            "document create",
            world.document_create(DOCUMENT_NEW, DOCUMENT_NEW_PATH),
            ResultCode::RepositoryNotRegistered,
            "index.rebuild",
        ),
        (
            "repo sync",
            Mutation::RepoSync(RepoSyncInput {
                root: world.root.clone(),
            }),
            ResultCode::InvalidConfiguration,
            "repo.inspect",
        ),
    ];
    for (command, request, code, action) in requests {
        let refused = world.execute(REQUEST_1, request);
        assert_eq!(refused.command, command);
        assert_eq!(
            (refused.outcome, refused.code),
            (Outcome::Blocked, code),
            "{command}"
        );
        assert_eq!(recovery_actions(&refused), [action], "{command}");
        assert!(world.record(REQUEST_1).is_none(), "{command}");
        assert_eq!(world.pending_journal_rows(), 0, "{command}");
    }
}

/// Puts the root of the world's repository on the commit it began with,
/// from before it was enabled: on a branch made there, or detached.
fn check_out_from_before_enabling(world: &World, detached: bool) {
    let repository = git2::Repository::open(&world.root).unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push_head().unwrap();
    let first = walk.last().unwrap().unwrap();
    let commit = repository.find_commit(first).unwrap();
    assert!(
        commit
            .tree()
            .unwrap()
            .get_path(std::path::Path::new(".manyhands/config.toml"))
            .is_err(),
        "the first commit is from before enabling"
    );
    if detached {
        repository.set_head_detached(first).unwrap();
    } else {
        repository.branch("old", &commit, false).unwrap();
        repository.set_head("refs/heads/old").unwrap();
    }
    repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    assert!(!world.root.join(".manyhands/config.toml").exists());
}

/// The review's sequence. The repository was enabled on `main`; its root
/// is then put on a branch from before that, or detached there, and the
/// index is lost. It was enabled: the tip of a local branch holds its
/// configuration. So it is not registered, and the rebuild that is
/// offered brings it back, as before.
#[test]
fn an_enabled_repository_on_a_commit_from_before_enabling_is_restored_by_its_rebuild() {
    for detached in [false, true] {
        let mut world = World::new();
        check_out_from_before_enabling(&world, detached);
        world.lose_cache();

        let refused = world.execute(REQUEST_1, world.save(TICKET_A, "v1:any"));
        assert_eq!(
            (refused.outcome, refused.code),
            (Outcome::Blocked, ResultCode::RepositoryNotRegistered),
            "detached {detached}"
        );
        assert_eq!(recovery_actions(&refused), ["index.rebuild"]);
        assert_eq!(world.request_rows(), 0);

        let rebuilt = world.execute(REQUEST_2, rebuild(&world));
        assert_eq!(
            (rebuilt.outcome, rebuilt.code),
            (Outcome::Success, ResultCode::Ok),
            "detached {detached}"
        );
        assert!(registered(&world), "detached {detached}");
        assert_eq!(world.execute(REQUEST_2, rebuild(&world)), rebuilt);
    }
}

/// Replaces the world's index with a file that is not a database, and
/// opens a service over it as the next process would.
fn corrupt_index(world: &mut World) -> std::path::PathBuf {
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
    database
}

/// What a root is refused for as no repository's working tree comes
/// before whether it was enabled: a bare repository is `bare_repository`
/// and a directory that is no repository is `not_repository`, with an
/// index that can be read and with one that cannot.
#[test]
fn a_bare_repository_and_a_directory_that_is_none_are_refused_as_what_they_are() {
    for corrupt in [false, true] {
        let mut world = World::new();
        let enabled_root = world.root.clone();
        let bare = support::bare_repository();
        let nothing = tempfile::tempdir().unwrap();
        let database = corrupt.then(|| corrupt_index(&mut world));
        for (root, code) in [
            (bare.root.clone(), ResultCode::BareRepository),
            (nothing.path().to_owned(), ResultCode::NotRepository),
        ] {
            world.root = root.canonicalize().unwrap();
            let refused = world.execute(REQUEST_1, rebuild(&world));
            assert_eq!(
                (refused.outcome, refused.code),
                (Outcome::Error, code),
                "corrupt {corrupt}"
            );
            if let Some(database) = &database {
                // Refused before anything is replaced or accepted for it.
                assert_eq!(refused.operation_id, None);
                assert_eq!(fs::read(database).unwrap(), b"not sqlite");
            }
        }
        // The world's own scan needs a database to read.
        world.root = enabled_root;
        let rebuilt = world.execute(REQUEST_2, rebuild(&world));
        assert_eq!(rebuilt.code, ResultCode::Ok, "corrupt {corrupt}");
    }
}

/// A rebuild that was accepted and never ran is asked again, when it is
/// entered again, whether its repository is enabled, before it registers
/// anything: what it was accepted on may no longer be there.
#[test]
fn a_rebuild_entered_again_does_not_register_a_repository_that_is_no_longer_enabled() {
    let (mut world, plain) = never_enabled();
    // Enabled, as far as Git says: its head commits a configuration.
    let before = plain.repository.head().unwrap().peel_to_commit().unwrap();
    support::commit_tracked_configuration(&plain, &support::config_source());
    let operations = count(&world, "operation_records");
    world.service = support::FailOnce::at(FailurePoint::BeforeRequestDomainCall)
        .open_service(world.data.path());
    let died = world.execute(REQUEST_1, rebuild(&world));
    assert_eq!(died.code, ResultCode::InternalError);
    assert!(world.record(REQUEST_1).is_some());
    assert!(!registered(&world));

    // The configuration goes: the branch is put back where it was.
    plain
        .repository
        .reset(before.as_object(), git2::ResetType::Hard, None)
        .unwrap();
    assert!(!world.root.join(".manyhands/config.toml").exists());

    world.reopen();
    let retry = world.execute(REQUEST_1, rebuild(&world));
    assert_not_enabled(&world, "index rebuild", &retry);
    assert!(!registered(&world));
    // No operation was begun for it, and its record is released.
    assert_eq!(count(&world, "operation_records"), operations);
    assert!(world.record(REQUEST_1).is_none());
}
