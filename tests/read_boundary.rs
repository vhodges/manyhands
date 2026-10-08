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
    repository::{LeaseKind, RepositoryService},
    results::{Outcome, ResultCode},
};
use serde_json::Value;

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
    let before = support::repository_and_worktree_snapshot(&fixture);

    assert_eq!(registered_repositories(&enabled.service), 1);
    let _ = enabled.service.new_item_id();

    assert!(before == support::repository_and_worktree_snapshot(&fixture));
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_is_busy_while_another_process_holds_the_exclusive_lock() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
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

    // The lock, not the index, was the obstacle.
    holder.release();
    assert_eq!(registered_repositories(&enabled.service), 1);
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_succeeds_while_another_process_holds_the_shared_lock() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let holder = hold_index_lock(&fixture, &enabled, LeaseKind::CacheRead);

    let started = Instant::now();
    assert_eq!(registered_repositories(&enabled.service), 1);
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

#[test]
fn no_read_test_initializes_the_git_transport() {
    assert_git_transport_uninitialized();
}
