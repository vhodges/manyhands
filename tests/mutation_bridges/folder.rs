//! Folder creation: `create_folder`, and `folder create` through the
//! boundary. What a folder path is, segment by segment, is in the unit
//! tests of `src/repository/bridges/folder.rs`.

use std::{collections::BTreeMap, fs, path::Path};

use manyhands::{
    repository::{
        CancellationToken, CreateFolderOutcome, CreateFolderRequest, DocumentFolderPath,
        FailurePoint, FolderCreateInput, FolderDiscovery, IndexRefreshInput, IndexState, LeaseKind,
        Mutation, MutationCall, MutationDataDto, OperationId, RepositoryErrorKind,
        RepositoryService, RequestId, RequestState, keys::SessionCredentials,
    },
    results::{DiscoveryEffect, Effects, Envelope, Outcome, ResultCode},
};

use super::support::{
    FailOnce,
    confirmation::{Desk, Plain},
    git_home,
    golden::ContractCase,
    items,
    mutation::{
        self, Events, NoPassphrase, REQUEST_1, REQUEST_2, REQUEST_3, TICKET_NEW, World,
        recovery_actions,
    },
};

fn create(world: &World, path: &str) -> Mutation {
    create_at(&world.root, path)
}

fn create_at(root: &Path, path: &str) -> Mutation {
    Mutation::FolderCreate(FolderCreateInput {
        root: root.to_owned(),
        path: path.to_owned(),
    })
}

fn refresh(world: &World) -> Mutation {
    Mutation::IndexRefresh(IndexRefreshInput {
        root: world.root.clone(),
    })
}

/// A world whose service fails once at `point`.
fn failing_at(point: FailurePoint) -> World {
    World::with_service(|data| FailOnce::at(point).open_service(data))
}

/// What the folder list holds, and how far behind it says the index is.
fn listed(world: &World) -> (Vec<(String, bool)>, IndexState) {
    let list = world
        .service
        .list_document_folders(&world.resolved())
        .unwrap();
    (
        list.items
            .into_iter()
            .map(|folder| (folder.path, folder.empty))
            .collect(),
        list.index.state,
    )
}

fn folder(path: &str, empty: bool) -> (String, bool) {
    (path.to_owned(), empty)
}

/// Every row of the local journal: its operation ID, action, state and
/// step.
fn journal(world: &World) -> Vec<(Option<String>, String, String, Option<String>)> {
    items::index(world.data.path())
        .prepare(
            "SELECT operation_ulid, action, state, completed_step
               FROM operation_records ORDER BY id",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// What Git holds and shows of the repository: every reference with
/// what it names, and every path `git status` would list.
fn git_state(world: &World) -> (BTreeMap<String, String>, Vec<String>) {
    let repository = git2::Repository::open(&world.root).unwrap();
    let references = repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            let target = reference
                .symbolic_target()
                .map(str::to_owned)
                .or_else(|| reference.target().map(|oid| oid.to_string()))
                .unwrap_or_default();
            (reference.name().unwrap().to_owned(), target)
        })
        .collect();
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let status = repository
        .statuses(Some(&mut options))
        .unwrap()
        .iter()
        .map(|entry| entry.path().unwrap().to_owned())
        .collect();
    (references, status)
}

fn discovery(effect: DiscoveryEffect) -> Effects {
    Effects {
        discovery: effect,
        ..Effects::not_requested()
    }
}

fn state(world: &World, request_id: &str) -> Option<RequestState> {
    world.record(request_id).map(|record| record.state)
}

/// Asserts that a request was refused as `code` with nothing left of
/// it: no record, no effect, no data.
fn assert_refused(
    world: &World,
    request_id: &str,
    refused: &Envelope<MutationDataDto>,
    code: ResultCode,
) {
    assert_eq!(refused.code, code, "{refused:?}");
    assert_ne!(refused.outcome, Outcome::Success);
    assert_ne!(refused.outcome, Outcome::Noop);
    assert_ne!(refused.outcome, Outcome::Partial);
    assert_eq!(refused.effects, Effects::not_requested());
    assert_eq!(refused.data, None);
    assert_eq!(world.record(request_id), None, "{code:?}");
}

/// Checks an envelope of `folder create` against its fixture. What
/// differs from run to run is replaced.
fn assert_golden(name: &str, world: &World, envelope: &Envelope<MutationDataDto>) {
    assert_eq!(
        envelope.data, None,
        "{name}: a folder creation carries no data"
    );
    let root = world.root.to_str().unwrap().to_owned();
    let operation = envelope.operation_id.clone();
    let mut placeholders = vec![(root.as_str(), "<root>")];
    if let Some(operation) = operation.as_deref() {
        placeholders.push((operation, "<operation>"));
    }
    mutation::assert_contract(
        &ContractCase {
            name,
            data_schema: None,
            placeholders: &placeholders,
            sentinels: &[world.data.path().to_str().unwrap()],
        },
        envelope,
    );
}

mod domain {
    use super::*;

    fn request(world: &World, path: &str) -> CreateFolderRequest {
        CreateFolderRequest {
            root: world.root.clone(),
            path: DocumentFolderPath::parse(path).unwrap(),
            operation_id: OperationId::new(),
        }
    }

    #[test]
    fn a_folder_is_made_found_refused_or_occupied() {
        git_home::isolated();
        let world = World::new();
        let made = world.service.create_folder(request(&world, "docs/a/b"));
        assert_eq!(
            made.unwrap(),
            CreateFolderOutcome::Created(FolderDiscovery::Current)
        );
        assert!(world.root.join("docs/a/b").is_dir());

        let found = world.service.create_folder(request(&world, "docs/a"));
        assert_eq!(
            found.unwrap(),
            CreateFolderOutcome::AlreadyPresent(FolderDiscovery::Current)
        );

        // A file at the path, and a file where a directory above it
        // would be.
        for occupied in ["docs/guide.md", "docs/guide.md/below"] {
            let outcome = world.service.create_folder(request(&world, occupied));
            assert_eq!(
                outcome.unwrap(),
                CreateFolderOutcome::Occupied,
                "{occupied}"
            );
        }
        assert!(world.root.join("docs/guide.md").is_file());
        assert_eq!(
            listed(&world).0,
            [folder("docs/a", false), folder("docs/a/b", true)]
        );
    }

    #[test]
    fn a_repository_that_is_not_registered_is_refused_and_nothing_is_made() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let failed = desk
            .service
            .create_folder(CreateFolderRequest {
                root: plain.root.clone(),
                path: DocumentFolderPath::parse("docs/a").unwrap(),
                operation_id: OperationId::new(),
            })
            .unwrap_err();
        assert_eq!(
            failed.error.kind,
            RepositoryErrorKind::RepositoryNotRegistered
        );
        assert!(!failed.left_directories);
        assert!(!plain.root.join("docs").exists());
    }

    // A call that fails once its directories are made says so, and a
    // second call goes on from them.
    #[test]
    fn a_call_that_fails_after_it_made_a_directory_says_it_left_one() {
        git_home::isolated();
        let world = failing_at(FailurePoint::AfterFolderCreation);
        let attempt = request(&world, "docs/a/b");
        let failed = world.service.create_folder(attempt.clone()).unwrap_err();
        assert_eq!(failed.error.kind, RepositoryErrorKind::InjectedFailure);
        assert!(failed.left_directories);
        assert!(world.root.join("docs/a/b").is_dir());
        // No refresh was begun, so there is no row.
        assert!(
            journal(&world)
                .iter()
                .all(|(id, ..)| id.as_deref() != Some(attempt.operation_id.to_string().as_str()))
        );

        let again = world.service.create_folder(attempt);
        assert_eq!(
            again.unwrap(),
            CreateFolderOutcome::AlreadyPresent(FolderDiscovery::Current)
        );
    }

    // What a link names is read by nothing here, so no error can say
    // it. Unix only: making a symbolic link needs a privilege on
    // Windows that a test run does not have.
    #[cfg(unix)]
    #[test]
    fn an_error_names_nothing_outside_the_repository() {
        git_home::isolated();
        let world = World::new();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join(OUTSIDE_SENTINEL);
        fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, world.root.join("docs/link")).unwrap();

        let failed = world
            .service
            .create_folder(request(&world, "docs/link/x"))
            .unwrap_err();

        assert_eq!(failed.error.kind, RepositoryErrorKind::InvalidPath);
        assert!(!failed.left_directories);
        for text in [format!("{failed}"), format!("{failed:?}")] {
            assert!(!text.contains(OUTSIDE_SENTINEL), "{text}");
            assert!(!text.contains(outside.path().to_str().unwrap()), "{text}");
        }
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    }
}

/// The name of a directory outside the repository, which no result,
/// event, record or error may hold.
const OUTSIDE_SENTINEL: &str = "SENTINEL-outside-7b3e";

mod boundary {
    use super::*;

    #[test]
    fn a_folder_is_made_with_the_directories_above_it_and_nothing_else() {
        git_home::isolated();
        let world = World::new();
        let git_before = git_state(&world);
        let journal_before = journal(&world);
        assert!(!world.root.join("docs/a").exists());

        let made = world.execute(REQUEST_1, create(&world, "docs/a/b"));

        assert_eq!(
            (made.outcome, made.code),
            (Outcome::Success, ResultCode::Ok),
            "{made:?}"
        );
        // Discovery and no other effect: nothing was written as
        // canonical content, committed or published.
        assert_eq!(made.effects, discovery(DiscoveryEffect::Current));
        assert_eq!(made.data, None);
        assert!(made.recovery.is_empty());
        assert_eq!(made.scope.repository.as_deref(), world.root.to_str());
        assert_eq!(
            (made.scope.item_id.as_ref(), made.scope.branch.as_ref()),
            (None, None)
        );
        // Both are directories, and both are empty of files.
        assert!(world.root.join("docs/a").is_dir());
        assert!(world.root.join("docs/a/b").is_dir());
        assert_eq!(
            fs::read_dir(world.root.join("docs/a/b")).unwrap().count(),
            0
        );
        assert_eq!(fs::read_dir(world.root.join("docs/a")).unwrap().count(), 1);
        // No commit and no branch, and `git status` shows what it showed:
        // an empty directory is nothing to Git.
        assert_eq!(git_state(&world), git_before);
        // The one row is the refresh's, under the request's operation.
        let rows = journal(&world);
        assert_eq!(rows.len(), journal_before.len() + 1, "{rows:?}");
        assert_eq!(rows[..journal_before.len()], journal_before[..]);
        assert_eq!(
            rows.last().unwrap(),
            &(
                made.operation_id.clone(),
                "refresh".to_owned(),
                "completed".to_owned(),
                None
            )
        );
        assert!(made.operation_id.is_some());
        assert_eq!(world.pending_journal_rows(), 0);
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
        // The index holds them: the innermost is empty.
        assert_eq!(
            listed(&world),
            (
                vec![folder("docs/a", false), folder("docs/a/b", true)],
                IndexState::Current
            )
        );
    }

    #[test]
    fn a_folder_that_holds_a_document_is_listed_as_not_empty() {
        git_home::isolated();
        let world = World::new();
        let made = world.execute(REQUEST_1, create(&world, "docs/notes"));
        assert_eq!(made.effects.discovery, DiscoveryEffect::Current);
        assert_eq!(listed(&world).0, [folder("docs/notes", true)]);

        items::write(
            &world.root,
            "docs/notes/plan.md",
            &items::document_source(items::DOCUMENT_B, "Plan", ""),
        );
        // The list is the index's: nothing says so until a refresh.
        assert_eq!(listed(&world).0, [folder("docs/notes", true)]);
        let refreshed = world.execute(REQUEST_2, refresh(&world));
        assert_eq!(refreshed.outcome, Outcome::Success);
        assert_eq!(listed(&world).0, [folder("docs/notes", false)]);
    }

    #[test]
    fn a_directory_that_is_there_is_a_no_op_and_a_file_occupies_its_path() {
        git_home::isolated();
        let world = World::new();
        fs::create_dir_all(world.root.join("docs/by-hand/inner")).unwrap();
        // Made by hand since the last refresh: the index does not hold it.
        assert_eq!(listed(&world).0, []);
        let git_before = git_state(&world);

        let found = world.execute(REQUEST_1, create(&world, "docs/by-hand"));

        assert_eq!(
            (found.outcome, found.code),
            (Outcome::Noop, ResultCode::AlreadyApplied),
            "{found:?}"
        );
        // Nothing was made, and the index holds the folder all the same.
        assert_eq!(found.effects, discovery(DiscoveryEffect::Current));
        assert!(found.recovery.is_empty());
        assert_eq!(
            listed(&world).0,
            [
                folder("docs/by-hand", false),
                folder("docs/by-hand/inner", true)
            ]
        );
        assert_eq!(git_state(&world), git_before);
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));

        // A file at the path, and a file where a directory above the
        // path would be.
        let journal_before = journal(&world);
        for (request, path) in [
            (REQUEST_2, "docs/guide.md"),
            (REQUEST_3, "docs/guide.md/below"),
        ] {
            let occupied = world.execute(request, create(&world, path));
            assert_refused(&world, request, &occupied, ResultCode::OccupiedPath);
            assert!(occupied.recovery.is_empty(), "{path}");
        }
        assert!(world.root.join("docs/guide.md").is_file());
        assert_eq!(journal(&world), journal_before);
        assert_eq!(git_state(&world), git_before);
    }

    // A file system that folds case takes `docs/reports` for
    // `docs/Reports`, and one that does not would make a second
    // directory that cannot be cloned onto the first. So a name that is
    // there in another case occupies the path, on every platform.
    #[test]
    fn a_name_that_is_there_in_another_case_occupies_the_path_on_every_platform() {
        git_home::isolated();
        let world = World::new();
        fs::create_dir(world.root.join("docs/Reports")).unwrap();

        for (request, path) in [
            (REQUEST_1, "docs/reports"),
            (REQUEST_2, "docs/REPORTS/2026"),
            // The file `docs/guide.md`.
            (REQUEST_3, "docs/Guide.MD"),
        ] {
            let occupied = world.execute(request, create(&world, path));
            assert_refused(&world, request, &occupied, ResultCode::OccupiedPath);
        }
        let names: Vec<String> = fs::read_dir(world.root.join("docs"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_owned())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert_eq!(
            fs::read_dir(world.root.join("docs/Reports"))
                .unwrap()
                .count(),
            0
        );

        // As it is written, it is the directory that is there.
        let found = world.execute(REQUEST_1, create(&world, "docs/Reports"));
        assert_eq!(found.code, ResultCode::AlreadyApplied);
        let below = world.execute(REQUEST_2, create(&world, "docs/Reports/2026"));
        assert_eq!(below.code, ResultCode::Ok);
        assert!(world.root.join("docs/Reports/2026").is_dir());
    }

    #[test]
    fn a_path_that_is_not_a_folder_s_is_invalid_input_on_every_platform() {
        git_home::isolated();
        let world = World::new();
        let outside = tempfile::tempdir().unwrap();
        let absolute = outside.path().join(OUTSIDE_SENTINEL);
        let seventeen = format!(
            "docs/{}",
            (1..=17)
                .map(|level| format!("l{level:02}"))
                .collect::<Vec<_>>()
                .join("/")
        );
        let before = super::tree(&world.root);
        let journal_before = journal(&world);
        let paths = [
            // Outside `docs/`.
            "notes/a",
            ".manyhands/tickets/a",
            "docs",
            "Docs/a",
            // Absolute.
            absolute.to_str().unwrap(),
            "/docs/a",
            "C:/docs/a",
            "C:\\docs\\a",
            "\\\\server\\share\\docs\\a",
            // `..` and `.`.
            "docs/../a",
            "docs/a/../../../etc",
            "docs/./a",
            // An empty segment.
            "docs//a",
            "docs/a/",
            "docs/",
            "",
            // Seventeen directories below `docs`.
            seventeen.as_str(),
            // What Windows would refuse or take for another name.
            "docs/CON",
            "docs/nul",
            "docs/NUL.txt",
            "docs/com1",
            "docs/LPT9",
            "docs/aux/reports",
            "docs/trailing.",
            "docs/trailing ",
            "docs/a\\b",
            "docs/C:",
            "docs/notes:stream",
            "docs/a*",
            "docs/a?",
            "docs/a|b",
            "docs/<a>",
            "docs/a\"b",
            "docs/tab\there",
            "docs/nul\0byte",
            // Git's own directory.
            "docs/.git",
            "docs/a/.GIT",
        ];
        for path in paths {
            let refused = world.execute(REQUEST_1, create(&world, path));
            assert_refused(&world, REQUEST_1, &refused, ResultCode::InvalidInput);
            assert_eq!(refused.outcome, Outcome::Error, "{path:?}");
            assert!(refused.recovery.is_empty(), "{path:?}");
            // The path it was given is nowhere in what it answers.
            let text = serde_json::to_string(&refused).unwrap();
            assert!(!text.contains(OUTSIDE_SENTINEL), "{text}");
        }
        assert!(before == super::tree(&world.root));
        assert_eq!(journal(&world), journal_before);
        assert_eq!(world.request_rows(), 0);
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    }

    // Discovery reads 1,024 entries under `docs/`. A folder is made as
    // the 1,024th and refused as the 1,025th, and one whose path would
    // take two is refused with neither made.
    #[test]
    fn a_folder_that_would_cross_a_limit_of_discovery_is_refused_and_nothing_is_made() {
        git_home::isolated();
        let world = World::new();
        // `docs/guide.md`, `docs/bulk` and 1,021 files in it: 1,023.
        fs::create_dir(world.root.join("docs/bulk")).unwrap();
        for entry in 0..1021 {
            fs::write(world.root.join(format!("docs/bulk/{entry:04}.txt")), "").unwrap();
        }

        let two = world.execute(REQUEST_1, create(&world, "docs/outer/inner"));
        assert_refused(&world, REQUEST_1, &two, ResultCode::InvalidInput);
        assert!(!world.root.join("docs/outer").exists());

        let last = world.execute(REQUEST_1, create(&world, "docs/last"));
        assert_eq!(
            (last.outcome, last.code, last.effects.discovery),
            (Outcome::Success, ResultCode::Ok, DiscoveryEffect::Current),
            "{last:?}"
        );
        // The refresh read the whole tree: it is exactly as much as it
        // reads.
        let list = world
            .service
            .list_document_folders(&world.resolved())
            .unwrap();
        assert!(list.complete);

        for (request, path) in [(REQUEST_2, "docs/one-more"), (REQUEST_3, "docs/last/inner")] {
            let refused = world.execute(request, create(&world, path));
            assert_refused(&world, request, &refused, ResultCode::InvalidInput);
            assert!(refused.recovery.is_empty());
            assert!(!world.root.join(path).exists());
        }
        // One that is there is found, whatever the tree holds.
        let found = world.execute(REQUEST_2, create(&world, "docs/bulk"));
        assert_eq!(found.code, ResultCode::AlreadyApplied);
        // The tree is still one discovery reads whole, so a save works.
        let saved = world.execute(REQUEST_3, world.create(TICKET_NEW));
        assert_eq!(
            (saved.outcome, saved.code),
            (Outcome::Success, ResultCode::Ok),
            "{saved:?}"
        );
        assert!(
            world
                .service
                .list_document_folders(&world.resolved())
                .unwrap()
                .complete
        );
    }

    #[test]
    fn a_folder_seventeen_levels_deep_is_refused_and_sixteen_is_made() {
        git_home::isolated();
        let world = World::new();
        let levels: Vec<String> = (1..=17).map(|level| format!("l{level:02}")).collect();
        let seventeen = format!("docs/{}", levels.join("/"));
        let sixteen = format!("docs/{}", levels[..16].join("/"));

        let refused = world.execute(REQUEST_1, create(&world, &seventeen));
        assert_refused(&world, REQUEST_1, &refused, ResultCode::InvalidInput);
        assert!(!world.root.join("docs/l01").exists());

        let made = world.execute(REQUEST_1, create(&world, &sixteen));
        assert_eq!(made.code, ResultCode::Ok, "{made:?}");
        assert!(world.root.join(&sixteen).is_dir());
        let list = world
            .service
            .list_document_folders(&world.resolved())
            .unwrap();
        assert!(list.complete);
        assert_eq!(list.items.len(), 16);
        // A save in the repository still works.
        let saved = world.execute(REQUEST_2, world.create(TICKET_NEW));
        assert_eq!(saved.outcome, Outcome::Success, "{saved:?}");
    }

    // Unix only: making a symbolic link needs a privilege on Windows
    // that a test run does not have. The walk that makes the
    // directories opens each one without following a link on Unix, and
    // on Windows refuses any directory that is a link or a junction.
    #[cfg(unix)]
    #[test]
    fn nothing_is_made_through_a_symbolic_link_inside_or_outside_the_repository() {
        git_home::isolated();
        let world = World::new();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join(OUTSIDE_SENTINEL);
        fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, world.root.join("docs/link")).unwrap();
        fs::create_dir(world.root.join("docs/real")).unwrap();
        std::os::unix::fs::symlink("real", world.root.join("docs/inside")).unwrap();
        std::os::unix::fs::symlink("nowhere", world.root.join("docs/dangling")).unwrap();

        for path in [
            "docs/link/x",
            "docs/link/x/y",
            // A link that stays inside is a link all the same, and so is
            // one that is the folder itself or names nothing.
            "docs/inside/x",
            "docs/link",
            "docs/inside",
            "docs/dangling",
            "docs/dangling/x",
        ] {
            let refused = world.execute(REQUEST_1, create(&world, path));
            assert_refused(&world, REQUEST_1, &refused, ResultCode::InvalidPath);
            assert_eq!(refused.outcome, Outcome::Error, "{path}");
            assert!(refused.recovery.is_empty(), "{path}");
        }
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 1);
        assert_eq!(
            fs::read_dir(world.root.join("docs/real")).unwrap().count(),
            0
        );
        assert!(!world.root.join("docs/nowhere").exists());
    }

    #[test]
    fn a_refresh_that_fails_after_the_folder_is_made_is_partial_and_any_refresh_finishes_it() {
        git_home::isolated();
        let world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
        let git_before = git_state(&world);

        let pending = world.execute(REQUEST_1, create(&world, "docs/a/b"));

        assert_eq!(
            (pending.outcome, pending.code),
            (Outcome::Partial, ResultCode::DiscoveryPending),
            "{pending:?}"
        );
        assert_eq!(pending.effects, discovery(DiscoveryEffect::Pending));
        assert_eq!(recovery_actions(&pending), ["index.refresh"]);
        let action = &pending.recovery[0];
        assert_eq!(action.operation_id, None);
        assert_eq!(
            action.arguments.get("root").and_then(|root| root.as_str()),
            world.root.to_str()
        );
        // The directory is there, and Git shows nothing of it.
        assert!(world.root.join("docs/a/b").is_dir());
        assert_eq!(git_state(&world), git_before);
        // The index does not hold it, and says it is behind.
        assert_eq!(listed(&world), (vec![], IndexState::Stale));
        // Nothing is pending: the refresh's row was closed, as that of a
        // call that left nothing behind.
        assert_eq!(world.pending_journal_rows(), 0);
        assert_eq!(
            journal(&world).last().unwrap(),
            &(
                pending.operation_id.clone(),
                "refresh".to_owned(),
                "completed".to_owned(),
                Some("rejected".to_owned())
            )
        );
        // The request is over: it is answered again as it was.
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
        let again = world.execute(REQUEST_1, create(&world, "docs/a/b"));
        assert_eq!(
            serde_json::to_value(&again).unwrap(),
            serde_json::to_value(&pending).unwrap()
        );
        assert_eq!(listed(&world), (vec![], IndexState::Stale));

        // What it offers is a refresh, which any request can make.
        let refreshed = world.execute(REQUEST_2, refresh(&world));
        assert_eq!(
            (refreshed.outcome, refreshed.code),
            (Outcome::Success, ResultCode::Ok),
            "{refreshed:?}"
        );
        assert_eq!(
            listed(&world),
            (
                vec![folder("docs/a", false), folder("docs/a/b", true)],
                IndexState::Current
            )
        );
    }

    // The directory was there before the request: nothing of the
    // request is durable, and the index is behind all the same.
    #[test]
    fn a_refresh_that_fails_for_a_folder_that_was_there_is_discovery_pending_too() {
        git_home::isolated();
        let world = failing_at(FailurePoint::AfterIndexClaim);
        fs::create_dir(world.root.join("docs/by-hand")).unwrap();

        let pending = world.execute(REQUEST_1, create(&world, "docs/by-hand"));

        assert_eq!(
            (pending.outcome, pending.code),
            (Outcome::Partial, ResultCode::DiscoveryPending),
            "{pending:?}"
        );
        assert_eq!(recovery_actions(&pending), ["index.refresh"]);
        assert_eq!(world.pending_journal_rows(), 0);
        assert_eq!(listed(&world), (vec![], IndexState::Stale));
        let refreshed = world.execute(REQUEST_2, refresh(&world));
        assert_eq!(refreshed.outcome, Outcome::Success, "{refreshed:?}");
        assert_eq!(
            listed(&world),
            (vec![folder("docs/by-hand", true)], IndexState::Current)
        );
    }

    #[test]
    fn a_finished_request_is_replayed_and_a_changed_input_is_a_mismatch() {
        git_home::isolated();
        let world = World::new();
        let made = world.execute(REQUEST_1, create(&world, "docs/a"));
        assert_eq!(made.code, ResultCode::Ok);
        let record = world.record(REQUEST_1);
        let rows = journal(&world);
        // The folder goes, and something else takes its place.
        fs::remove_dir(world.root.join("docs/a")).unwrap();
        fs::write(world.root.join("docs/a"), "a file now\n").unwrap();

        // The stored result, whatever is there now: nothing is looked at
        // and nothing is made.
        let again = world.execute(REQUEST_1, create(&world, "docs/a"));
        assert_eq!(
            serde_json::to_value(&again).unwrap(),
            serde_json::to_value(&made).unwrap()
        );
        assert!(world.root.join("docs/a").is_file());
        assert_eq!(world.record(REQUEST_1), record);
        assert_eq!(journal(&world), rows);

        let changed = world.execute(REQUEST_1, create(&world, "docs/b"));
        assert_eq!(
            (changed.outcome, changed.code),
            (Outcome::Error, ResultCode::RequestMismatch),
            "{changed:?}"
        );
        assert!(!world.root.join("docs/b").exists());
        assert_eq!(world.record(REQUEST_1), record);
        // Another repository's folder of the same path is another
        // request too.
        let other = World::new();
        let elsewhere = world.execute(REQUEST_1, create_at(&other.root, "docs/a"));
        assert_eq!(elsewhere.code, ResultCode::RequestMismatch);
        assert!(!other.root.join("docs/a").exists());
    }

    // The call made the directory, refreshed the index and lost its
    // answer. Its retry is a no-op, and is never told that the
    // directory its own first call made occupies the path.
    #[test]
    fn a_request_whose_result_was_lost_is_a_no_op_on_retry() {
        git_home::isolated();
        let world = failing_at(FailurePoint::BeforeRequestSettlement);
        let lost = world.execute(REQUEST_1, create(&world, "docs/a/b"));
        assert_eq!(lost.code, ResultCode::InternalError, "{lost:?}");
        assert!(world.root.join("docs/a/b").is_dir());
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
        let rows = journal(&world);
        assert_eq!(world.pending_journal_rows(), 0);
        let git_before = git_state(&world);

        let retried = world.execute(REQUEST_1, create(&world, "docs/a/b"));

        assert_eq!(
            (retried.outcome, retried.code),
            (Outcome::Noop, ResultCode::AlreadyApplied),
            "{retried:?}"
        );
        assert_eq!(retried.effects, discovery(DiscoveryEffect::Current));
        assert!(retried.recovery.is_empty());
        // No second refresh: the row of the first is the evidence, and
        // the request's operation is still that one.
        assert_eq!(journal(&world), rows);
        assert_eq!(retried.operation_id, rows.last().unwrap().0);
        assert!(retried.operation_id.is_some());
        assert_eq!(git_state(&world), git_before);
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
        assert_eq!(
            listed(&world),
            (
                vec![folder("docs/a", false), folder("docs/a/b", true)],
                IndexState::Current
            )
        );
        // And from then on it is replayed.
        let replayed = world.execute(REQUEST_1, create(&world, "docs/a/b"));
        assert_eq!(
            serde_json::to_value(&replayed).unwrap(),
            serde_json::to_value(&retried).unwrap()
        );
    }

    // What the first call did is recorded by its refresh's row. A retry
    // answers from that, as a replay answers from a stored result: a
    // folder that was removed since is not made a second time.
    #[test]
    fn a_retry_after_a_lost_result_does_not_make_a_folder_that_was_removed_since() {
        git_home::isolated();
        let world = failing_at(FailurePoint::BeforeRequestSettlement);
        let lost = world.execute(REQUEST_1, create(&world, "docs/a"));
        assert_eq!(lost.code, ResultCode::InternalError);
        fs::remove_dir(world.root.join("docs/a")).unwrap();
        fs::write(world.root.join("docs/a"), "a file now\n").unwrap();

        let retried = world.execute(REQUEST_1, create(&world, "docs/a"));

        assert_eq!(retried.code, ResultCode::AlreadyApplied, "{retried:?}");
        assert!(world.root.join("docs/a").is_file());
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
    }

    // The call made the directories and failed before the index was
    // told: no row was begun. The same request goes on from what is
    // there.
    #[test]
    fn a_request_that_failed_after_it_made_the_folder_is_finished_by_its_retry() {
        git_home::isolated();
        let world = failing_at(FailurePoint::AfterFolderCreation);
        let journal_before = journal(&world);

        let failed = world.execute(REQUEST_1, create(&world, "docs/a/b"));

        // A durable change with work remaining, whatever stopped it.
        assert_eq!(
            (failed.outcome, failed.code),
            (Outcome::Partial, ResultCode::InternalError),
            "{failed:?}"
        );
        assert_eq!(failed.effects, Effects::not_requested());
        assert_eq!(recovery_actions(&failed), ["request.retry"]);
        assert!(world.root.join("docs/a/b").is_dir());
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
        assert_eq!(journal(&world), journal_before);
        // The index was not told, and nothing says it is behind yet.
        assert_eq!(listed(&world).0, []);

        let retried = world.execute(REQUEST_1, create(&world, "docs/a/b"));

        // It finds the directory its first call made, and is not told
        // that anything occupies the path.
        assert_eq!(
            (retried.outcome, retried.code),
            (Outcome::Noop, ResultCode::AlreadyApplied),
            "{retried:?}"
        );
        assert_eq!(retried.effects, discovery(DiscoveryEffect::Current));
        assert_eq!(retried.operation_id, failed.operation_id);
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
        assert_eq!(journal(&world).len(), journal_before.len() + 1);
        assert_eq!(
            listed(&world),
            (
                vec![folder("docs/a", false), folder("docs/a/b", true)],
                IndexState::Current
            )
        );
    }

    // The refresh must be tried again, and its row could not be closed:
    // it is pending, and refuses every other refresh. The request is
    // not over, and the same request is what takes the row up.
    #[test]
    fn a_refresh_whose_row_could_not_be_closed_is_taken_up_by_the_same_request() {
        git_home::isolated();
        let world = failing_at(FailurePoint::BeforeFolderRefreshClose);
        // The repository changes while the refresh observes it.
        let changed = world.root.join("docs/guide.md");
        world
            .service
            .set_observation_hook_for_testing(move || fs::write(changed, "plain\n").unwrap());

        let pending = world.execute(REQUEST_1, create(&world, "docs/a"));

        assert_eq!(
            (pending.outcome, pending.code),
            (Outcome::Partial, ResultCode::DiscoveryPending),
            "{pending:?}"
        );
        assert_eq!(pending.effects, discovery(DiscoveryEffect::Pending));
        assert!(world.root.join("docs/a").is_dir());
        assert_eq!(world.pending_journal_rows(), 1);
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Accepted));
        // The same request, and the way out of its own pending
        // operation. No refresh is offered: one would be refused.
        assert_eq!(
            mutation::recovery_before_own_abandon(&pending),
            ["request.retry"]
        );
        let other = world.execute(REQUEST_2, refresh(&world));
        assert_eq!(other.code, ResultCode::RecoveryRequired, "{other:?}");

        let retried = world.execute(REQUEST_1, create(&world, "docs/a"));

        assert_eq!(
            (retried.outcome, retried.code),
            (Outcome::Noop, ResultCode::AlreadyApplied),
            "{retried:?}"
        );
        assert_eq!(retried.effects, discovery(DiscoveryEffect::Current));
        assert_eq!(retried.operation_id, pending.operation_id);
        assert_eq!(world.pending_journal_rows(), 0);
        assert_eq!(state(&world, REQUEST_1), Some(RequestState::Finished));
        assert_eq!(
            listed(&world),
            (vec![folder("docs/a", true)], IndexState::Current)
        );
    }

    // A refresh that failed leaves its row pending, which refuses every
    // other refresh. A folder is not made into a repository whose index
    // could not then be told of it.
    #[test]
    fn a_repository_with_another_operation_pending_is_refused_before_anything_is_made() {
        git_home::isolated();
        let world = failing_at(FailurePoint::BeforeIndexTransactionCommit);
        let failed = world.execute(REQUEST_1, refresh(&world));
        assert_eq!(failed.code, ResultCode::InternalError);
        assert_eq!(world.pending_journal_rows(), 1);
        let rows = journal(&world);

        let refused = world.execute(REQUEST_2, create(&world, "docs/a"));

        assert_refused(&world, REQUEST_2, &refused, ResultCode::RecoveryRequired);
        // What is offered is to look at the repository, and to abandon
        // the operation that is in the way: the refresh, not this
        // request's own, which never began.
        assert_eq!(
            recovery_actions(&refused),
            ["repo.inspect", "operation.abandon"]
        );
        assert_eq!(refused.recovery[1].operation_id, failed.operation_id);
        assert!(failed.operation_id.is_some());
        assert!(!world.root.join("docs/a").exists());
        assert_eq!(journal(&world), rows);

        // Once the refresh is taken up by its own request, the folder is
        // made.
        let refreshed = world.execute(REQUEST_1, refresh(&world));
        assert_eq!(refreshed.outcome, Outcome::Success, "{refreshed:?}");
        let made = world.execute(REQUEST_2, create(&world, "docs/a"));
        assert_eq!(made.code, ResultCode::Ok, "{made:?}");
    }

    #[test]
    fn a_repository_another_operation_holds_is_busy_and_nothing_is_made() {
        git_home::isolated();
        let world = World::new();
        let held = RepositoryService::hold_lease_for_testing(
            &world.root,
            world.data.path(),
            LeaseKind::Repository,
        )
        .unwrap();

        let busy = world.execute(REQUEST_1, create(&world, "docs/a"));

        assert_refused(&world, REQUEST_1, &busy, ResultCode::Busy);
        assert_eq!(recovery_actions(&busy), ["request.retry"]);
        assert!(!world.root.join("docs/a").exists());
        drop(held);
        let made = world.execute(REQUEST_1, create(&world, "docs/a"));
        assert_eq!(made.code, ResultCode::Ok, "{made:?}");
    }

    #[test]
    fn a_repository_that_is_not_enabled_or_a_root_that_is_not_one_is_refused() {
        git_home::isolated();
        let plain = Plain::new();
        let desk = Desk::new();
        let never =
            mutation::execute_with(&desk.service, REQUEST_1, create_at(&plain.root, "docs/a"));
        assert_eq!(never.code, ResultCode::RepositoryNotEnabled, "{never:?}");
        assert!(!plain.root.join("docs").exists());
        assert_eq!(desk.request_rows(), 0);

        // A directory inside a registered repository is not its root,
        // and `docs/a` below it is not made.
        let world = World::new();
        let inside = world.root.join("docs");
        let refused = world.execute(REQUEST_1, create_at(&inside, "docs/a"));
        assert_ne!(refused.outcome, Outcome::Success);
        assert_eq!(world.record(REQUEST_1), None);
        assert!(!inside.join("docs").exists());
    }

    // What the command reads that it may not give out: where a link in
    // the path leads, and a path it was given that is not the
    // repository's. Unix only, for the link.
    #[cfg(unix)]
    #[test]
    fn no_path_outside_the_repository_reaches_an_envelope_an_event_or_a_record() {
        git_home::isolated();
        let world = World::new();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join(OUTSIDE_SENTINEL);
        fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, world.root.join("docs/link")).unwrap();
        let outside_path = outside.path().to_str().unwrap().to_owned();
        let sentinels = [OUTSIDE_SENTINEL, outside_path.as_str()];
        let absolute = target.join("x");
        let escaping = format!("docs/../../{OUTSIDE_SENTINEL}/x");

        let mut seen = Vec::new();
        for (request, path) in [
            (REQUEST_1, "docs/link/x"),
            (REQUEST_1, absolute.to_str().unwrap()),
            (REQUEST_1, escaping.as_str()),
            // One that is made, beside the link: its record is kept.
            (REQUEST_2, "docs/beside-the-link"),
        ] {
            let mut events = Events::default();
            let envelope = world.service.execute(
                MutationCall {
                    request_id: RequestId::parse(request).unwrap(),
                    confirmation: None,
                    mutation: create(&world, path),
                },
                &mut SessionCredentials::new(NoPassphrase),
                &mut events,
                &CancellationToken::new(),
            );
            seen.push(serde_json::to_string(&envelope).unwrap());
            seen.push(format!("{envelope:?}"));
            seen.extend(
                events
                    .0
                    .iter()
                    .map(|event| serde_json::to_string(event).unwrap()),
            );
        }
        // A read of the folder list, with the link in the tree.
        let list = world
            .service
            .list_document_folders(&world.resolved())
            .unwrap();
        seen.push(serde_json::to_string(&list).unwrap());
        // The refresh reported the link as a problem of the index.
        let status = world.service.index_status(&world.resolved()).unwrap();
        seen.push(serde_json::to_string(&status).unwrap());

        for text in &seen {
            for sentinel in sentinels {
                assert!(!text.contains(sentinel), "{sentinel}: {text}");
            }
        }
        mutation::assert_request_stores_exclude(world.data.path(), &sentinels);
        // The scan can fail: the stores hold the path of the request
        // that was accepted, and events were sent.
        assert!(
            mutation::find_stored_sentinel(world.data.path(), &["docs/beside-the-link"]).is_some()
        );
        assert!(seen.len() > 10, "{}", seen.len());
        assert!(world.root.join("docs/beside-the-link").is_dir());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    }

    #[test]
    fn a_folder_create_matches_the_envelope_schema_and_its_goldens() {
        git_home::isolated();
        let world = World::new();
        let made = world.execute(REQUEST_1, create(&world, "docs/notes/2026"));
        assert_eq!(made.outcome, Outcome::Success);
        assert_golden("folder_create", &world, &made);

        let found = world.execute(REQUEST_2, create(&world, "docs/notes"));
        assert_eq!(found.outcome, Outcome::Noop);
        assert_golden("folder_create_noop", &world, &found);

        let occupied = world.execute(REQUEST_3, create(&world, "docs/guide.md"));
        assert_eq!(occupied.code, ResultCode::OccupiedPath);
        assert_golden("folder_create_occupied", &world, &occupied);

        let invalid = world.execute(REQUEST_3, create(&world, "docs/../notes"));
        assert_eq!(invalid.code, ResultCode::InvalidInput);
        assert_golden("folder_create_invalid_input", &world, &invalid);

        let failing = failing_at(FailurePoint::BeforeIndexTransactionCommit);
        let pending = failing.execute(REQUEST_1, create(&failing, "docs/notes"));
        assert_eq!(pending.outcome, Outcome::Partial);
        assert_golden("folder_create_discovery_pending", &failing, &pending);
    }
}

/// Every path under `root` outside `.git`, with a file's bytes.
fn tree(root: &Path) -> BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
    let mut found = BTreeMap::new();
    let mut directories = vec![root.to_owned()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.file_name() == Some(std::ffi::OsStr::new(".git")) {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                found.insert(path.clone(), None);
                directories.push(path);
            } else {
                found.insert(path.clone(), fs::read(&path).ok());
            }
        }
    }
    found
}
