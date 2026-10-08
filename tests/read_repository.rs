//! Target resolution and the repository, identity and remote reads.

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::{Config, ConfigLevel, Repository, RepositoryInitOptions, Signature, Time};
use manyhands::{
    repository::{
        Accessibility, ConfigurationState, IdentityAvailability, IdentitySource, IndexState,
        ReadError, RepositoryService,
    },
    results::{ProblemCode, ResultCode},
};
use serde_json::{Value, json};

mod support;

const SENTINEL: &str = "SENTINEL-41c9";

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap()
}

fn path_string(path: &Path) -> String {
    canonical(path).to_str().unwrap().to_owned()
}

fn linked_worktree(fixture: &support::TestRepository, name: &str, path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fixture.repository.worktree(name, path, None).unwrap();
}

fn recovery(error: &ReadError) -> Value {
    serde_json::to_value(&error.to_envelope::<Value>("item list").recovery).unwrap()
}

fn isolated_config(level: ConfigLevel, source: &str) -> (tempfile::TempDir, Config) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gitconfig");
    fs::write(&path, source).unwrap();
    let mut config = Config::new().unwrap();
    config.add_file(&path, level, false).unwrap();
    (directory, config)
}

#[test]
fn a_root_a_linked_worktree_and_an_item_worktree_resolve_to_one_registration() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let outside = tempfile::tempdir().unwrap();
    let linked = outside.path().join("linked");
    linked_worktree(&fixture, "linked", &linked);
    let item = fixture
        .root
        .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV");
    linked_worktree(&fixture, "01ARZ3NDEKTSV4RRFFQ69G5FAV", &item);

    let root = enabled.service.resolve_repository(&fixture.root).unwrap();
    let from_linked = enabled.service.resolve_repository(&linked).unwrap();
    let from_item = enabled.service.resolve_repository(&item).unwrap();

    assert_eq!(root.root(), canonical(&fixture.root));
    assert_eq!(from_linked, root);
    assert_eq!(from_item, root);
    assert_eq!(
        root.scope().repository,
        Some(path_string(&fixture.root)),
        "the scope of a resolved repository is its root"
    );
    assert_eq!(root.scope().worktree, None);
    assert_git_transport_uninitialized();
}

#[test]
fn a_path_inside_a_repository_is_not_its_root_and_the_recovery_names_the_root() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let subdirectory = fixture.root.join("nested/deeper");
    fs::create_dir_all(&subdirectory).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let linked = outside.path().join("linked");
    linked_worktree(&fixture, "linked", &linked);
    let linked_subdirectory = linked.join("nested");
    fs::create_dir_all(&linked_subdirectory).unwrap();

    for (path, root) in [
        (subdirectory, path_string(&fixture.root)),
        // The Git directory can be opened, and is still not the root.
        (fixture.root.join(".git"), path_string(&fixture.root)),
        (fixture.root.join("fixture.txt"), path_string(&fixture.root)),
        // The root to use is the worktree the path lies in.
        (linked_subdirectory, path_string(&linked)),
    ] {
        let error = enabled.service.resolve_repository(&path).unwrap_err();

        assert_eq!(error.code(), ResultCode::NotRepositoryRoot, "{path:?}");
        assert_eq!(error.scope.repository.as_deref(), Some(root.as_str()));
        assert_eq!(
            recovery(&error),
            json!([{
                "action": "repo.inspect",
                "operation_id": null,
                "arguments": {"root": root},
            }]),
            "{path:?}"
        );
    }
    assert_git_transport_uninitialized();
}

#[test]
fn an_unregistered_repository_is_not_registered() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let other = support::born_repository();

    let error = enabled.service.resolve_repository(&other.root).unwrap_err();

    assert_eq!(error.code(), ResultCode::RepositoryNotRegistered);
    assert_eq!(error.scope.repository, Some(path_string(&other.root)));
    assert_eq!(recovery(&error), json!([]));
    // A linked worktree of it is no more registered than it is.
    let outside = tempfile::tempdir().unwrap();
    let linked = outside.path().join("linked");
    linked_worktree(&other, "linked", &linked);
    let error = enabled.service.resolve_repository(&linked).unwrap_err();
    assert_eq!(error.code(), ResultCode::RepositoryNotRegistered);
    assert_eq!(error.scope.repository, Some(path_string(&other.root)));
    assert_git_transport_uninitialized();
}

#[test]
fn a_bare_repository_a_non_repository_and_a_missing_path_each_have_their_code() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let bare = support::bare_repository();
    let plain = tempfile::tempdir().unwrap();
    let missing = plain.path().join("missing");

    let error = enabled.service.resolve_repository(&bare.root).unwrap_err();
    assert_eq!(error.code(), ResultCode::BareRepository);
    assert_eq!(error.scope.repository, Some(path_string(&bare.root)));
    assert_eq!(recovery(&error), json!([]));

    let error = enabled
        .service
        .resolve_repository(&bare.root.join("objects"))
        .unwrap_err();
    assert_eq!(error.code(), ResultCode::BareRepository);
    assert_eq!(error.scope.repository, Some(path_string(&bare.root)));

    let error = enabled
        .service
        .resolve_repository(plain.path())
        .unwrap_err();
    assert_eq!(error.code(), ResultCode::NotRepository);
    assert_eq!(error.scope.repository, None);
    assert_eq!(recovery(&error), json!([]));

    let error = enabled.service.resolve_repository(&missing).unwrap_err();
    assert_eq!(error.code(), ResultCode::InvalidPath);
    assert_eq!(error.scope.repository, None);
    assert_eq!(recovery(&error), json!([]));
    assert_git_transport_uninitialized();
}

#[test]
fn a_degraded_index_names_the_root_to_rebuild() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let root = path_string(&fixture.root);
    let rebuild = json!([{
        "action": "index.rebuild",
        "operation_id": null,
        "arguments": {"root": root},
    }]);

    let error = service.resolve_repository(&fixture.root).unwrap_err();
    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    assert_eq!(error.scope.repository.as_deref(), Some(root.as_str()));
    assert_eq!(recovery(&error), rebuild);

    let error = service.inspect_repository(&fixture.root).unwrap_err();
    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    assert_eq!(error.scope.repository.as_deref(), Some(root.as_str()));
    assert_eq!(recovery(&error), rebuild);

    // No repository is in question, so there is no root to name.
    let error = service.list_repositories().unwrap_err();
    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    assert_eq!(error.scope.repository, None);
    assert_eq!(
        recovery(&error),
        json!([{"action": "index.rebuild", "operation_id": null, "arguments": {}}])
    );
    assert_git_transport_uninitialized();
}

#[test]
fn list_repositories_reports_every_registration_in_root_order_from_the_index_alone() {
    let fixtures = [
        support::born_repository(),
        support::born_repository(),
        support::born_repository(),
    ];
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    for fixture in &fixtures {
        service
            .enable(support::enable_request(&fixture.root))
            .unwrap();
    }
    let mut roots: Vec<_> = fixtures
        .iter()
        .map(|fixture| path_string(&fixture.root))
        .collect();
    roots.sort();
    // One root is removed from disk after it was registered.
    let [removed, kept, _] = fixtures;
    let removed_root = path_string(&removed.root);
    drop(removed);
    assert!(!Path::new(&removed_root).exists());
    // One registration is waiting for a refresh.
    let stale_root = path_string(&kept.root);
    rusqlite::Connection::open(data.path().join("manyhands.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE repositories SET refresh_required = 1 WHERE root_path = ?1",
            [&stale_root],
        )
        .unwrap();

    let list = service.list_repositories().unwrap();

    assert!(list.complete);
    assert_eq!(
        list.items
            .iter()
            .map(|item| item.root.clone())
            .collect::<Vec<_>>(),
        roots
    );
    for item in &list.items {
        // What the index last stored, not what the disk now holds.
        assert_eq!(item.accessibility, Accessibility::Accessible, "{item:?}");
        assert!(item.enabled_at.is_some());
        assert_eq!(item.configuration.state, ConfigurationState::Valid);
        assert_eq!(item.configuration.primary_branch.as_deref(), Some("main"));
        assert_eq!(item.configuration.publication_remote, None);
        assert!(item.configuration.problems.is_empty());
        assert_eq!(
            item.index.state,
            if item.root == stale_root {
                IndexState::Stale
            } else {
                IndexState::Current
            }
        );
        assert_eq!(item.index.refreshed_at, None);
        assert_eq!(item.problem_count, 0);
    }
    assert!(list.items.iter().any(|item| item.root == removed_root));
    assert!(!Path::new(&removed_root).exists());
    assert_git_transport_uninitialized();
}

#[test]
fn list_repositories_reports_stored_configuration_states_and_problem_counts() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let index = rusqlite::Connection::open(enabled.data_directory.path().join("manyhands.sqlite3"))
        .unwrap();
    let set_configuration = |statement: &str| {
        index
            .execute("DELETE FROM configuration_observations", [])
            .unwrap();
        if !statement.is_empty() {
            index.execute(statement, []).unwrap();
        }
    };
    let configuration = || {
        enabled
            .service
            .list_repositories()
            .unwrap()
            .items
            .remove(0)
            .configuration
    };

    set_configuration(
        "INSERT INTO configuration_observations
             (repository_id, state, primary_branch, publication_remote)
         SELECT id, 'valid', 'trunk', 'publish' FROM repositories",
    );
    let valid = configuration();
    assert_eq!(valid.state, ConfigurationState::Valid);
    assert_eq!(valid.primary_branch.as_deref(), Some("trunk"));
    assert_eq!(valid.publication_remote.as_deref(), Some("publish"));
    assert!(valid.problems.is_empty());

    set_configuration(&format!(
        "INSERT INTO configuration_observations (repository_id, state, invalid_code, guidance)
         SELECT id, 'invalid', 'malformed-configuration', '{SENTINEL}' FROM repositories"
    ));
    let invalid = configuration();
    assert_eq!(invalid.state, ConfigurationState::Invalid);
    assert_eq!(invalid.primary_branch, None);
    assert_eq!(invalid.publication_remote, None);
    assert_eq!(invalid.problems.len(), 1);
    assert_eq!(
        invalid.problems[0].code,
        ProblemCode::MalformedConfiguration
    );
    assert_eq!(
        invalid.problems[0].path.as_deref(),
        Some(".manyhands/config.toml")
    );
    // Stored guidance is never published.
    assert!(!serde_json::to_string(&invalid).unwrap().contains(SENTINEL));

    set_configuration(
        "INSERT INTO configuration_observations (repository_id, state)
         SELECT id, 'missing' FROM repositories",
    );
    assert_eq!(configuration().state, ConfigurationState::Missing);
    // Never observed is reported as the snapshot reports it.
    set_configuration("");
    let unobserved = configuration();
    assert_eq!(unobserved.state, ConfigurationState::Missing);
    assert!(unobserved.problems.is_empty());

    index
        .execute_batch(
            "INSERT INTO problems (repository_id, path, code, guidance, observed_at)
             SELECT id, 'docs/a.md', 'missing-field', 'unused', 0 FROM repositories;
             INSERT INTO problems (repository_id, path, code, guidance, observed_at)
             SELECT id, NULL, 'branch', 'unused', 0 FROM repositories;",
        )
        .unwrap();
    assert_eq!(
        enabled.service.list_repositories().unwrap().items[0].problem_count,
        2
    );
    assert_git_transport_uninitialized();
}

#[test]
fn inspect_repository_reports_an_unregistered_repository_without_enabling_it() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);

    let inspection = service.inspect_repository(&fixture.root).unwrap();

    assert_eq!(inspection.root, path_string(&fixture.root));
    assert_eq!(inspection.selected_path, inspection.root);
    assert!(!inspection.registered);
    assert_eq!(inspection.head_branch.as_deref(), Some("main"));
    assert_eq!(inspection.local_branches, ["main"]);
    assert_eq!(inspection.configuration.state, ConfigurationState::Missing);
    assert_eq!(inspection.identity_state, IdentityAvailability::Available);
    assert!(inspection.remotes.is_empty());
    assert!(service.list_repositories().unwrap().items.is_empty());
    assert!(before == support::repository_and_worktree_snapshot(&fixture));
    assert_git_transport_uninitialized();
}

#[test]
fn inspect_repository_reports_registration_configuration_and_redacted_remotes() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    fixture
        .repository
        .remote(
            "origin",
            &format!("https://user:{SENTINEL}@example.invalid/team/repo.git"),
        )
        .unwrap();
    let outside = tempfile::tempdir().unwrap();
    let linked = outside.path().join("linked");
    linked_worktree(&fixture, "linked", &linked);

    let inspection = enabled.service.inspect_repository(&fixture.root).unwrap();

    assert!(inspection.registered);
    assert_eq!(inspection.configuration.state, ConfigurationState::Valid);
    assert_eq!(
        inspection.configuration.primary_branch.as_deref(),
        Some("main")
    );
    assert_eq!(inspection.local_branches, ["linked", "main"]);
    assert_eq!(inspection.remotes.len(), 1);
    assert_eq!(
        inspection.remotes[0].fetch_location,
        "https://example.invalid/team/repo.git"
    );
    assert!(
        !serde_json::to_string(&inspection)
            .unwrap()
            .contains(SENTINEL)
    );
    // A linked worktree is inspected as the repository that owns it.
    let mut from_linked = enabled.service.inspect_repository(&linked).unwrap();
    assert_eq!(from_linked.selected_path, path_string(&linked));
    assert_eq!(from_linked.root, path_string(&fixture.root));
    from_linked.selected_path = inspection.selected_path.clone();
    assert_eq!(from_linked, inspection);

    fs::write(
        fixture.root.join(".manyhands/config.toml"),
        "format_version = 2\n",
    )
    .unwrap();
    let invalid = enabled
        .service
        .inspect_repository(&fixture.root)
        .unwrap()
        .configuration;
    assert_eq!(invalid.state, ConfigurationState::Invalid);
    assert_eq!(invalid.primary_branch, None);
    assert_eq!(invalid.problems.len(), 1);
    assert_eq!(
        invalid.problems[0].path.as_deref(),
        Some(".manyhands/config.toml")
    );
    assert_git_transport_uninitialized();
}

#[test]
fn inspect_repository_refuses_what_resolution_refuses() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let subdirectory = fixture.root.join("nested");
    fs::create_dir_all(&subdirectory).unwrap();
    let bare = support::bare_repository();
    let plain = tempfile::tempdir().unwrap();

    let code = |path: &Path| enabled.service.inspect_repository(path).unwrap_err().code();

    assert_eq!(code(&subdirectory), ResultCode::NotRepositoryRoot);
    assert_eq!(code(&bare.root), ResultCode::BareRepository);
    assert_eq!(code(plain.path()), ResultCode::NotRepository);
    assert_eq!(code(&plain.path().join("missing")), ResultCode::InvalidPath);
    assert_git_transport_uninitialized();
}

#[test]
fn identity_reports_the_configuration_level_it_came_from() {
    let local = support::born_repository();
    let local_enabled = support::enabled_repository(&local);
    let local_repo = local_enabled
        .service
        .resolve_repository(&local.root)
        .unwrap();
    // Enabling needs an identity, so the local one is removed afterwards.
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut config = Config::open(&fixture.repository.path().join("config")).unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    drop(config);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let complete = "[user]\nname = Other Name\nemail = other@example.invalid\n";
    let identity = |config: &mut Config| {
        enabled
            .service
            .repository_identity_with_config_for_testing(&repo, config)
            .unwrap()
    };

    let from_repository = local_enabled
        .service
        .repository_identity(&local_repo)
        .unwrap();
    assert_eq!(from_repository.source, IdentitySource::Repository);
    assert_eq!(from_repository.name.as_deref(), Some("Manyhands Test"));
    assert_eq!(
        from_repository.email.as_deref(),
        Some("manyhands-test@example.invalid")
    );

    let (_global_directory, mut global) = isolated_config(ConfigLevel::Global, complete);
    let from_global = identity(&mut global);
    assert_eq!(from_global.source, IdentitySource::Global);
    assert_eq!(from_global.name.as_deref(), Some("Other Name"));
    assert_eq!(from_global.email.as_deref(), Some("other@example.invalid"));

    let (_xdg_directory, mut xdg) = isolated_config(ConfigLevel::XDG, complete);
    assert_eq!(identity(&mut xdg).source, IdentitySource::Xdg);

    let none = identity(&mut Config::new().unwrap());
    assert_eq!(none.source, IdentitySource::None);
    assert_eq!((none.name, none.email), (None, None));

    // A name alone is not an identity, and is not reported as one.
    let (_partial_directory, mut partial) =
        isolated_config(ConfigLevel::Global, "[user]\nname = Other Name\n");
    let incomplete = identity(&mut partial);
    assert_eq!(incomplete.source, IdentitySource::None);
    assert_eq!(
        serde_json::to_value(&incomplete).unwrap(),
        json!({"name": null, "email": null, "source": "none"})
    );
    assert_git_transport_uninitialized();
}

#[test]
fn remotes_are_listed_by_name_redacted_with_the_publication_selection() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    assert_eq!(
        serde_json::to_value(enabled.service.list_remotes_redacted(&repo).unwrap()).unwrap(),
        json!({"items": [], "complete": true})
    );
    fixture
        .repository
        .remote("publish", "ssh://git@example.invalid/team/repo.git")
        .unwrap();
    fixture
        .repository
        .remote(
            "origin",
            &format!("https://user:{SENTINEL}@example.invalid/team/repo.git"),
        )
        .unwrap();
    fixture
        .repository
        .remote_set_pushurl(
            "origin",
            Some(&format!(
                "https://example.invalid/team/repo.git?token={SENTINEL}"
            )),
        )
        .unwrap();
    fs::write(
        fixture.root.join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"publish\"\n",
    )
    .unwrap();

    let remotes = enabled.service.list_remotes_redacted(&repo).unwrap();

    assert!(
        !serde_json::to_string(&remotes).unwrap().contains(SENTINEL),
        "an embedded credential must not be published"
    );
    assert_eq!(
        serde_json::to_value(&remotes).unwrap(),
        json!({
            "items": [
                {
                    "name": "origin",
                    "fetch_location": "https://example.invalid/team/repo.git",
                    "push_location": "https://example.invalid/team/repo.git",
                    "publication_eligible": false,
                    "selected_for_publication": false,
                },
                {
                    "name": "publish",
                    "fetch_location": "ssh://git@example.invalid/team/repo.git",
                    "push_location": "ssh://git@example.invalid/team/repo.git",
                    "publication_eligible": true,
                    "selected_for_publication": true,
                },
            ],
            "complete": true,
        })
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_registered_root_removed_from_disk_is_inaccessible_to_the_reads_that_open_it() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let root = path_string(&fixture.root);
    drop(fixture);

    let error = enabled.service.repository_identity(&repo).unwrap_err();
    assert_eq!(error.code(), ResultCode::RepositoryInaccessible);
    assert_eq!(error.scope.repository.as_deref(), Some(root.as_str()));

    let error = enabled.service.list_remotes_redacted(&repo).unwrap_err();
    assert_eq!(error.code(), ResultCode::RepositoryInaccessible);
    assert_eq!(error.scope.repository.as_deref(), Some(root.as_str()));
    assert_git_transport_uninitialized();
}

fn commit_empty_tree(repository: &Repository) {
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(0, 0),
    )
    .unwrap();
    let tree = repository
        .find_tree(repository.treebuilder(None).unwrap().write().unwrap())
        .unwrap();
    repository
        .commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
        .unwrap();
}

/// A repository whose Git directory is `git_directory` and whose working
/// directory is elsewhere, with one commit.
fn separate_git_directory_repository(git_directory: &Path, working_directory: &Path) -> Repository {
    fs::create_dir_all(working_directory).unwrap();
    let mut options = RepositoryInitOptions::new();
    options
        .no_dotgit_dir(true)
        .workdir_path(working_directory)
        .initial_head("main");
    let repository = Repository::init_opts(git_directory, &options).unwrap();
    commit_empty_tree(&repository);
    repository
}

/// Removes `core.worktree`, as `git init --separate-git-dir` leaves it: the
/// Git directory then no longer says where its working directory is.
fn forget_working_directory(git_directory: &Path) {
    Config::open(&git_directory.join("config"))
        .unwrap()
        .remove("core.worktree")
        .unwrap();
}

// Without `core.worktree`, Git takes the parent of a Git directory to be its
// working directory. That parent is not thereby the owner of its worktrees.
#[test]
fn a_linked_worktree_never_resolves_to_a_repository_that_does_not_own_it() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    fixture
        .repository
        .remote("origin", "ssh://git@example.invalid/registered.git")
        .unwrap();
    let outside = tempfile::tempdir().unwrap();

    // An unrelated repository keeps its Git directory inside the registered
    // root and does not record where its own working directory is.
    let other_git = fixture.root.join("other.git");
    let other = separate_git_directory_repository(&other_git, &outside.path().join("other"));
    let other_worktree = outside.path().join("other-worktree");
    other
        .worktree("other-worktree", &other_worktree, None)
        .unwrap();
    forget_working_directory(&other_git);

    // A bare repository inside the registered root, marked as not bare.
    let not_bare_git = fixture.root.join("nb.git");
    let not_bare = Repository::init_bare(&not_bare_git).unwrap();
    commit_empty_tree(&not_bare);
    let not_bare_worktree = outside.path().join("nb-worktree");
    not_bare
        .worktree("nb-worktree", &not_bare_worktree, None)
        .unwrap();
    Config::open(&not_bare_git.join("config"))
        .unwrap()
        .set_bool("core.bare", false)
        .unwrap();

    for worktree in [&other_worktree, &not_bare_worktree] {
        let error = enabled.service.resolve_repository(worktree).unwrap_err();
        assert_eq!(error.code(), ResultCode::NotRepository, "{worktree:?}");
        assert_eq!(error.scope.repository, Some(path_string(worktree)));
        assert_eq!(recovery(&error), json!([]));
        assert_eq!(
            enabled
                .service
                .inspect_repository(worktree)
                .unwrap_err()
                .code(),
            ResultCode::NotRepository
        );
    }
    // The registered repository still resolves, and its own worktrees do.
    let linked = outside.path().join("linked");
    linked_worktree(&fixture, "linked", &linked);
    let root = enabled.service.resolve_repository(&fixture.root).unwrap();
    assert_eq!(enabled.service.resolve_repository(&linked).unwrap(), root);
    assert_git_transport_uninitialized();
}

#[test]
fn a_linked_worktree_whose_owner_cannot_be_verified_names_no_directory_that_is_not_a_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let plain = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let git_directory = plain.path().join("other.git");
    let other = separate_git_directory_repository(&git_directory, &outside.path().join("other"));
    let worktree = outside.path().join("worktree");
    other.worktree("worktree", &worktree, None).unwrap();
    forget_working_directory(&git_directory);

    let error = enabled.service.resolve_repository(&worktree).unwrap_err();

    // Not `repository_not_registered` for the parent of the Git directory.
    assert_eq!(error.code(), ResultCode::NotRepository);
    assert_eq!(error.scope.repository, Some(path_string(&worktree)));
    assert_ne!(error.scope.repository, Some(path_string(plain.path())));
    assert_git_transport_uninitialized();
}

#[test]
fn a_linked_worktree_of_a_repository_with_a_separate_git_directory_resolves_to_its_owner() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let outside = tempfile::tempdir().unwrap();
    let working_directory = outside.path().join("owner");
    let owner =
        separate_git_directory_repository(&outside.path().join("owner.git"), &working_directory);
    let worktree = outside.path().join("worktree");
    owner.worktree("worktree", &worktree, None).unwrap();

    // The owner is found and verified; it is simply not registered.
    for path in [&working_directory, &worktree] {
        let error = enabled.service.resolve_repository(path).unwrap_err();
        assert_eq!(
            error.code(),
            ResultCode::RepositoryNotRegistered,
            "{path:?}"
        );
        assert_eq!(
            error.scope.repository,
            Some(path_string(&working_directory))
        );
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_linked_worktree_whose_owner_has_lost_its_working_directory_is_inaccessible() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let outside = tempfile::tempdir().unwrap();
    let working_directory = outside.path().join("owner");
    let owner =
        separate_git_directory_repository(&outside.path().join("owner.git"), &working_directory);
    let worktree = outside.path().join("worktree");
    owner.worktree("worktree", &worktree, None).unwrap();
    fs::remove_dir_all(&working_directory).unwrap();

    let error = enabled.service.resolve_repository(&worktree).unwrap_err();

    assert_eq!(error.code(), ResultCode::RepositoryInaccessible);
    assert_eq!(error.scope.repository, Some(path_string(&worktree)));
    assert_git_transport_uninitialized();
}

#[test]
fn a_repository_git_cannot_open_is_inaccessible_not_absent() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let subdirectory = fixture.root.join("nested");
    fs::create_dir_all(&subdirectory).unwrap();
    // A repository format this build of Git does not support.
    Config::open(&fixture.repository.path().join("config"))
        .unwrap()
        .set_i32("core.repositoryformatversion", 99)
        .unwrap();

    for path in [&fixture.root, &subdirectory] {
        for error in [
            enabled.service.resolve_repository(path).unwrap_err(),
            enabled.service.inspect_repository(path).unwrap_err(),
        ] {
            assert_eq!(error.code(), ResultCode::RepositoryInaccessible, "{path:?}");
            assert_eq!(error.scope.repository, Some(path_string(path)));
            assert_eq!(recovery(&error), json!([]));
        }
    }
    // Already resolved, it is inaccessible to the reads that open it too.
    assert_eq!(
        enabled
            .service
            .repository_identity(&repo)
            .unwrap_err()
            .code(),
        ResultCode::RepositoryInaccessible
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_path_through_a_file_is_an_invalid_path() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);

    let error = enabled
        .service
        .resolve_repository(&fixture.root.join("fixture.txt/below"))
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::InvalidPath);
    assert_eq!(error.scope.repository, None);
    assert_git_transport_uninitialized();
}

#[test]
fn a_repository_with_a_detached_head_is_read_like_any_other() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    fixture
        .repository
        .remote("origin", "ssh://git@example.invalid/team/repo.git")
        .unwrap();
    let head = fixture.repository.head().unwrap().target().unwrap();
    fixture.repository.set_head_detached(head).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);

    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let inspection = enabled.service.inspect_repository(&fixture.root).unwrap();

    assert_eq!(inspection.head_branch, None);
    assert!(inspection.registered);
    assert_eq!(inspection.local_branches, ["main"]);
    assert_eq!(inspection.configuration.state, ConfigurationState::Valid);
    assert_eq!(inspection.remotes.len(), 1);
    assert_eq!(
        serde_json::to_value(&inspection).unwrap()["head_branch"],
        Value::Null
    );
    assert_eq!(
        enabled.service.repository_identity(&repo).unwrap().source,
        IdentitySource::Repository
    );
    assert_eq!(
        enabled
            .service
            .list_remotes_redacted(&repo)
            .unwrap()
            .items
            .len(),
        1
    );
    // The existing inspection still refuses a detached HEAD.
    assert!(enabled.service.inspect(&fixture.root).is_err());
    assert!(before == support::repository_and_worktree_snapshot(&fixture));
    assert_git_transport_uninitialized();
}
