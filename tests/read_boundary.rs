//! What a read may touch: the shared index lock, a read-only connection and
//! nothing else. Each read added later joins the checks in this file.

use std::{
    error::Error,
    fs,
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureFilter, LeaseKind, ReadError, RepositoryService, ResolvedRepository, SharedKeyId,
        TicketFilter, transport::SshAuthority,
    },
    results::{Outcome, ResultCode},
};
use serde_json::Value;
use support::{credentials, items};

mod support;

/// Far above the 250 ms lease bound and far below a blocked wait.
const NOT_BLOCKED: Duration = Duration::from_secs(5);

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

#[test]
fn index_lease_child() {
    let Ok(root) = std::env::var("MANYHANDS_LEASE_ROOT") else {
        return;
    };
    let data_directory = PathBuf::from(std::env::var("MANYHANDS_LEASE_DATA_DIRECTORY").unwrap());
    let kind = LeaseKind::parse(&std::env::var("MANYHANDS_LEASE_KIND").unwrap()).unwrap();
    let ready = PathBuf::from(std::env::var("MANYHANDS_LEASE_READY").unwrap());
    let release = PathBuf::from(std::env::var("MANYHANDS_LEASE_RELEASE").unwrap());
    let _holder = RepositoryService::hold_lease_for_testing(
        std::path::Path::new(&root),
        &data_directory,
        kind,
    )
    .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !release.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn hold_index_lock(
    fixture: &support::TestRepository,
    enabled: &support::EnabledRepository,
    kind: LeaseKind,
) -> support::LeaseHolder {
    support::hold_lease_in_child_for_test(
        &fixture.root,
        enabled.data_directory.path(),
        kind,
        "index_lease_child",
    )
}

/// Pins one host in the index and returns its authority.
fn pinned_authority(enabled: &support::EnabledRepository) -> SshAuthority {
    credentials::pin_host(
        &enabled.service,
        "pinned.example",
        22,
        "ssh-ed25519",
        credentials::PUBLIC_FIXTURE_FINGERPRINT,
    );
    SshAuthority {
        host: "pinned.example".to_owned(),
        port: 22,
    }
}

fn registered_repositories(service: &RepositoryService) -> i64 {
    service
        .read_session_for_testing(|connection| {
            connection.query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))
        })
        .unwrap()
}

fn assert_rebuild_is_the_recovery(error: &manyhands::repository::ReadError) {
    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    let envelope = error.to_envelope::<Value>("document list");
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(envelope.recovery.len(), 1);
    assert_eq!(envelope.recovery[0].action, "index.rebuild");
}

#[test]
fn read_session_reads_the_index_of_an_enabled_repository() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);

    assert_eq!(registered_repositories(&enabled.service), 1);
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_changes_nothing_in_the_repository_or_its_worktrees() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    fixture
        .repository
        .remote("origin", "ssh://git@example.invalid/team/repo.git")
        .unwrap();
    let linked = fixture.root.join(".manyhands/worktrees/linked");
    fs::create_dir_all(linked.parent().unwrap()).unwrap();
    fixture
        .repository
        .worktree("linked", &linked, None)
        .unwrap();
    let keys = credentials::register_keys(&enabled.service);
    let pinned = pinned_authority(&enabled);
    credentials::require_host_reapproval(&enabled.service);
    // Taken before the first read, which finds the index as its last
    // writer left it.
    let before = support::repository_and_worktree_snapshot(&fixture);
    let data_before = credentials::data_directory_files(enabled.data_directory.path());
    assert!(
        data_before
            .iter()
            .any(|(name, _)| name == credentials::REAPPROVAL_MARKER_FILE)
    );
    let key_files_before = credentials::key_file_states(&keys.key_files());
    assert_eq!(key_files_before.len(), 5);

    assert_eq!(registered_repositories(&enabled.service), 1);
    let _ = enabled.service.new_item_id();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    assert_eq!(enabled.service.resolve_repository(&linked).unwrap(), repo);
    enabled
        .service
        .resolve_repository(&fixture.root.join(".manyhands"))
        .unwrap_err();
    assert_eq!(enabled.service.list_repositories().unwrap().items.len(), 1);
    assert!(
        enabled
            .service
            .inspect_repository(&fixture.root)
            .unwrap()
            .registered
    );
    enabled.service.inspect_repository(&linked).unwrap();
    enabled.service.repository_identity(&repo).unwrap();
    assert_eq!(
        enabled
            .service
            .list_remotes_redacted(&repo)
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(enabled.service.list_keys().unwrap().items.len(), 3);
    for registration in [&keys.imported, &keys.generated, &keys.without_public] {
        enabled.service.show_key(registration.id).unwrap();
    }
    enabled.service.public_key_text(keys.imported.id).unwrap();
    enabled.service.public_key_text(keys.generated.id).unwrap();
    enabled
        .service
        .public_key_text(keys.without_public.id)
        .unwrap_err();
    enabled.service.show_key(SharedKeyId::new()).unwrap_err();
    assert_eq!(enabled.service.list_host_pins().unwrap().items.len(), 1);
    assert!(
        enabled
            .service
            .inspect_host(&pinned)
            .unwrap()
            .reapproval_required
    );
    enabled
        .service
        .inspect_host(&SshAuthority {
            host: "unpinned.example".to_owned(),
            port: 22,
        })
        .unwrap_err();

    // The index and the reapproval marker are byte for byte what they were;
    // the only files that may appear are SQLite's two journal files.
    credentials::assert_reads_left_the_data_directory(
        &data_before,
        &credentials::data_directory_files(enabled.data_directory.path()),
    );
    assert_eq!(
        credentials::key_file_states(&keys.key_files()),
        key_files_before
    );

    assert!(before == support::repository_and_worktree_snapshot(&fixture));
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_is_busy_while_another_process_holds_the_exclusive_lock() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let keys = credentials::register_keys(&enabled.service);
    let pinned = pinned_authority(&enabled);
    let holder = hold_index_lock(&fixture, &enabled, LeaseKind::CacheWrite);

    let started = Instant::now();
    let error = enabled
        .service
        .read_session_for_testing(|_| Ok(()))
        .unwrap_err();
    let elapsed = started.elapsed();

    assert_eq!(error.code(), ResultCode::Busy);
    assert!(elapsed < NOT_BLOCKED, "{elapsed:?}");
    assert_eq!(error.to_string(), ResultCode::Busy.message());
    assert!(error.source().is_some());
    let envelope = error.to_envelope::<Value>("item list");
    assert_eq!(envelope.outcome, Outcome::Error);
    assert!(envelope.recovery.is_empty());

    // Every read that uses the session is refused the same way, and says
    // which repository it could not read once it knows.
    let root = fs::canonicalize(&fixture.root).unwrap();
    let busy = [
        enabled.service.list_repositories().unwrap_err(),
        enabled
            .service
            .resolve_repository(&fixture.root)
            .unwrap_err(),
        enabled
            .service
            .inspect_repository(&fixture.root)
            .unwrap_err(),
    ];
    for (error, repository) in busy.iter().zip([None, root.to_str(), root.to_str()]) {
        assert_eq!(error.code(), ResultCode::Busy);
        assert_eq!(error.scope.repository.as_deref(), repository);
        assert!(error.recovery.is_empty());
    }
    // The credential reads are application-wide and name no repository.
    let busy = [
        enabled.service.list_keys().unwrap_err(),
        enabled.service.show_key(keys.imported.id).unwrap_err(),
        enabled
            .service
            .public_key_text(keys.imported.id)
            .unwrap_err(),
        enabled.service.list_host_pins().unwrap_err(),
        enabled.service.inspect_host(&pinned).unwrap_err(),
    ];
    for error in &busy {
        assert_eq!(error.code(), ResultCode::Busy);
        assert_eq!(error.scope.repository, None);
        assert!(error.recovery.is_empty());
    }
    assert!(started.elapsed() < NOT_BLOCKED);

    // The lock, not the index, was the obstacle.
    holder.release();
    assert_eq!(registered_repositories(&enabled.service), 1);
    assert_eq!(enabled.service.list_keys().unwrap().items.len(), 3);
    assert_eq!(enabled.service.list_host_pins().unwrap().items.len(), 1);
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_succeeds_while_another_process_holds_the_shared_lock() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let keys = credentials::register_keys(&enabled.service);
    let pinned = pinned_authority(&enabled);
    let holder = hold_index_lock(&fixture, &enabled, LeaseKind::CacheRead);

    let started = Instant::now();
    assert_eq!(registered_repositories(&enabled.service), 1);
    assert_eq!(enabled.service.list_keys().unwrap().items.len(), 3);
    enabled.service.show_key(keys.generated.id).unwrap();
    enabled.service.public_key_text(keys.generated.id).unwrap();
    assert_eq!(enabled.service.list_host_pins().unwrap().items.len(), 1);
    enabled.service.inspect_host(&pinned).unwrap();
    assert!(started.elapsed() < NOT_BLOCKED);

    holder.release();
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_refuses_every_write_as_read_only() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    // Each is valid against a writable index, so only the session refuses it.
    let statements = [
        "UPDATE repositories SET rowid = rowid",
        "DELETE FROM repositories",
        "CREATE TABLE read_boundary_probe (value INTEGER)",
        "CREATE TEMP TABLE read_boundary_probe (value INTEGER)",
        "PRAGMA user_version = 7",
    ];
    let writable =
        rusqlite::Connection::open(enabled.data_directory.path().join("manyhands.sqlite3"))
            .unwrap();
    for statement in statements {
        let transaction = writable.unchecked_transaction().unwrap();
        transaction.execute_batch(statement).unwrap();
        transaction.rollback().unwrap();
    }
    drop(writable);

    let refusals = enabled
        .service
        .read_session_for_testing(|connection| {
            Ok(statements.map(|statement| connection.execute_batch(statement).unwrap_err()))
        })
        .unwrap();

    for (statement, refusal) in statements.into_iter().zip(refusals) {
        assert_eq!(
            refusal.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ReadOnly),
            "{statement}"
        );
    }
    assert_eq!(registered_repositories(&enabled.service), 1);
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_cannot_write_a_file_after_ending_its_transaction() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let scratch = tempfile::tempdir().unwrap();
    let copy = scratch.path().join("copy.sqlite3");
    let attached = scratch.path().join("attached.sqlite3");

    let (committed, vacuum, attach) = enabled
        .service
        .read_session_for_testing(|connection| {
            let committed = connection.execute_batch("COMMIT");
            let vacuum =
                connection.execute_batch(&format!("VACUUM INTO '{}'", copy.to_str().unwrap()));
            let attach = connection.execute_batch(&format!(
                "ATTACH DATABASE '{}' AS other; CREATE TABLE other.probe (value INTEGER)",
                attached.to_str().unwrap()
            ));
            Ok((committed, vacuum, attach))
        })
        .unwrap();

    // Ending the transaction early is not itself a write.
    committed.unwrap();
    assert!(vacuum.is_err());
    assert!(attach.is_err());
    assert!(!copy.exists());
    assert!(!attached.exists());
    assert_eq!(fs::read_dir(scratch.path()).unwrap().count(), 0);
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_reports_a_degraded_index_as_unavailable() {
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service.read_session_for_testing(|_| Ok(())).unwrap_err();

    assert_rebuild_is_the_recovery(&error);
    assert_git_transport_uninitialized();
}

fn index_files(enabled: &support::EnabledRepository) -> Vec<PathBuf> {
    fs::read_dir(enabled.data_directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_str().unwrap();
            name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock")
        })
        .collect()
}

// The service opened a good index, so it does not know the index is gone:
// the failure arrives from SQLite, wrapped by the function that opens it.
#[test]
fn read_session_reports_an_index_deleted_after_open_as_unavailable() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    assert_eq!(registered_repositories(&enabled.service), 1);
    for file in index_files(&enabled) {
        fs::remove_file(file).unwrap();
    }

    let error = enabled
        .service
        .read_session_for_testing(|_| Ok(()))
        .unwrap_err();

    assert_rebuild_is_the_recovery(&error);
    assert!(index_files(&enabled).is_empty());
    assert_git_transport_uninitialized();
}

// Here the failure arrives unwrapped, from the first statement that reads.
#[test]
fn read_session_reports_an_index_corrupted_after_open_as_unavailable() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    assert_eq!(registered_repositories(&enabled.service), 1);
    for file in index_files(&enabled) {
        fs::remove_file(file).unwrap();
    }
    let index = enabled.data_directory.path().join("manyhands.sqlite3");
    fs::write(
        &index,
        b"not sqlite, and long enough to be read as a header",
    )
    .unwrap();

    let error = enabled
        .service
        .read_session_for_testing(|connection| {
            connection.query_row("SELECT COUNT(*) FROM repositories", [], |row| {
                row.get::<_, i64>(0)
            })
        })
        .unwrap_err();

    assert_rebuild_is_the_recovery(&error);
    assert_eq!(
        fs::read(&index).unwrap(),
        b"not sqlite, and long enough to be read as a header"
    );
    assert_git_transport_uninitialized();
}

#[test]
fn new_item_id_needs_no_session() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let _holder = hold_index_lock(&fixture, &enabled, LeaseKind::CacheWrite);

    let id = enabled.service.new_item_id().id;

    assert_eq!(ItemId::from_str(&id).unwrap().to_string(), id);
    assert_git_transport_uninitialized();
}

// Identity and remotes are read from Git alone once the repository is
// resolved, so the index lock does not stand in their way.
#[test]
fn identity_and_remotes_of_a_resolved_repository_need_no_session() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let _holder = hold_index_lock(&fixture, &enabled, LeaseKind::CacheWrite);

    enabled.service.repository_identity(&repo).unwrap();
    enabled.service.list_remotes_redacted(&repo).unwrap();

    assert_git_transport_uninitialized();
}

/// A repository with every kind of thing an item read meets: a primary
/// document and ticket, an item worktree with its own document, and a file
/// that is not an item.
fn repository_with_items() -> (support::TestRepository, support::EnabledRepository) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let root = &fixture.root;
    items::write(
        root,
        "docs/a.md",
        &items::document_source(items::DOCUMENT_A, "A", "extra: kept\n"),
    );
    items::write(
        root,
        &items::ticket_path(items::TICKET_A),
        &items::ticket_source(items::TICKET_A, "A", items::CLOSURE),
    );
    items::commit(
        &fixture,
        &["docs/a.md", &items::ticket_path(items::TICKET_A)],
        1_000,
    );
    items::create_document_context(&enabled.service, root, items::DOCUMENT_B, "docs/b.md");
    items::write(
        root,
        "docs/marker.md",
        "---\nmanyhands_managed: true\n---\n",
    );
    items::refresh_completely(&enabled.service, root);
    (fixture, enabled)
}

/// Every item read, with inputs that succeed and inputs that fail. Returns
/// the outcome of each, in order: success, or the code it failed with.
fn every_item_read(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    root: &std::path::Path,
) -> Vec<Result<(), ResultCode>> {
    let worktree = root.join(".manyhands/worktrees").join(items::DOCUMENT_B);
    let closed = TicketFilter {
        closure: ClosureFilter::Closed,
        status: Some("open".to_owned()),
        ..Default::default()
    };
    let path = std::path::Path::new;
    let ticket = items::ticket_path(items::TICKET_A);
    let outcome = |result: Result<(), ReadError>| result.map_err(|error| error.code());
    vec![
        outcome(service.list_documents(repo).map(drop)),
        outcome(
            service
                .list_tickets(repo, &TicketFilter::default())
                .map(drop),
        ),
        outcome(service.list_tickets(repo, &closed).map(drop)),
        outcome(
            service
                .show_item(repo, &items::item_id(items::DOCUMENT_A))
                .map(drop),
        ),
        outcome(
            service
                .show_item(repo, &items::item_id(items::DOCUMENT_B))
                .map(drop),
        ),
        outcome(
            service
                .show_item(repo, &items::item_id(items::TICKET_A))
                .map(drop),
        ),
        outcome(
            service
                .show_item(repo, &items::item_id(items::DOCUMENT_C))
                .map(drop),
        ),
        outcome(service.show_path(repo, None, path("docs/a.md")).map(drop)),
        outcome(service.show_path(repo, None, path(&ticket)).map(drop)),
        outcome(
            service
                .show_path(repo, None, path("docs/marker.md"))
                .map(drop),
        ),
        outcome(
            service
                .show_path(repo, Some(&worktree), path("docs/b.md"))
                .map(drop),
        ),
        outcome(
            service
                .show_path(repo, Some(&worktree), path("docs/a.md"))
                .map(drop),
        ),
        outcome(
            service
                .show_path(repo, None, path("docs/absent.md"))
                .map(drop),
        ),
        outcome(service.show_path(repo, None, path("../outside.md")).map(drop)),
    ]
}

#[test]
fn item_reads_change_nothing_in_the_repository_or_its_worktrees() {
    let (fixture, enabled) = repository_with_items();
    let before = support::repository_and_worktree_snapshot(&fixture);
    let git_before = support::repository_git_file_bytes(&fixture);
    let data_before = credentials::data_directory_files(enabled.data_directory.path());
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let outcomes = every_item_read(&enabled.service, &repo, &fixture.root);

    assert_eq!(
        outcomes,
        [
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Err(ResultCode::ItemNotFound),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Err(ResultCode::PathNotFound),
            Err(ResultCode::InvalidPath),
        ]
    );
    credentials::assert_reads_left_the_data_directory(
        &data_before,
        &credentials::data_directory_files(enabled.data_directory.path()),
    );
    assert!(before == support::repository_and_worktree_snapshot(&fixture));
    // Objects, refs, reflogs and each worktree's administrative files.
    assert!(git_before == support::repository_git_file_bytes(&fixture));
    assert_git_transport_uninitialized();
}

#[test]
fn item_reads_are_busy_under_the_exclusive_lock_and_succeed_under_the_shared_one() {
    let (fixture, enabled) = repository_with_items();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let root = fs::canonicalize(&fixture.root).unwrap();
    let unlocked = every_item_read(&enabled.service, &repo, &fixture.root);

    let holder = hold_index_lock(&fixture, &enabled, LeaseKind::CacheWrite);
    let started = Instant::now();
    let outcomes = every_item_read(&enabled.service, &repo, &fixture.root);
    let error = enabled.service.list_documents(&repo).unwrap_err();
    assert!(
        started.elapsed() < NOT_BLOCKED * 3,
        "{:?}",
        started.elapsed()
    );
    holder.release();

    // The last is refused for how its path is written, before any lock is
    // asked for.
    let (invalid, busy) = outcomes.split_last().unwrap();
    assert_eq!(invalid, &Err(ResultCode::InvalidPath));
    assert!(
        busy.iter().all(|outcome| outcome == &Err(ResultCode::Busy)),
        "{outcomes:?}"
    );
    assert_eq!(error.code(), ResultCode::Busy);
    assert_eq!(error.scope.repository.as_deref(), root.to_str());
    assert!(error.recovery.is_empty());

    let holder = hold_index_lock(&fixture, &enabled, LeaseKind::CacheRead);
    let started = Instant::now();
    assert_eq!(
        every_item_read(&enabled.service, &repo, &fixture.root),
        unlocked
    );
    assert!(started.elapsed() < NOT_BLOCKED);
    holder.release();
    assert_git_transport_uninitialized();
}

#[test]
fn no_read_test_initializes_the_git_transport() {
    assert_git_transport_uninitialized();
}
