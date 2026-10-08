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
            connection
                .query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))
                .unwrap()
        })
        .unwrap()
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
        .read_session_for_testing(|_| ())
        .unwrap_err();
    let elapsed = started.elapsed();

    assert_eq!(error.code, ResultCode::Busy);
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
fn read_session_cannot_write_and_keeps_nothing() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let statements = [
        "INSERT INTO repositories DEFAULT VALUES",
        "DELETE FROM repositories",
        "CREATE TABLE read_boundary_probe (value INTEGER)",
        "CREATE TEMP TABLE read_boundary_probe AS SELECT * FROM repositories",
    ];

    let failures = enabled
        .service
        .read_session_for_testing(|connection| {
            statements.map(|statement| connection.execute(statement, []).is_err())
        })
        .unwrap();

    // A temporary table is not the index; the rest must be refused.
    assert_eq!(failures, [true, true, true, false]);
    assert_eq!(registered_repositories(&enabled.service), 1);
    enabled
        .service
        .read_session_for_testing(|connection| {
            // A new session is a new connection: nothing carried over.
            assert!(
                connection
                    .prepare("SELECT * FROM read_boundary_probe")
                    .is_err()
            );
            assert!(!connection.is_autocommit());
        })
        .unwrap();
    assert_git_transport_uninitialized();
}

#[test]
fn read_session_reports_a_degraded_index_as_unavailable() {
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service.read_session_for_testing(|_| ()).unwrap_err();

    assert_eq!(error.code, ResultCode::IndexUnavailable);
    let envelope = error.to_envelope::<Value>("item list");
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(envelope.recovery.len(), 1);
    assert_eq!(envelope.recovery[0].action, "index.rebuild");
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
