use std::{
    fs::OpenOptions,
    path::PathBuf,
    time::{Duration, Instant},
};

use manyhands::repository::{
    ExpectedPathObservation, IndexPending, LeaseKind, OperationId, RebuildRepositoryRequest,
    RefreshRepositoryRequest, RepositoryErrorKind, RepositoryService,
};

mod support;

#[test]
fn common_git_lease_child() {
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
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !release.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for lease release"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn common_git_lease_blocks_primary_and_linked_worktree_operations() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    let branch = fixture.repository.branch("linked", &head, false).unwrap();
    let linked = fixture.tempdir.path().join("linked");
    let mut options = git2::WorktreeAddOptions::new();
    let reference = branch.into_reference();
    options.reference(Some(&reference));
    fixture
        .repository
        .worktree("linked", &linked, Some(&options))
        .unwrap();
    let holder = support::hold_lease_in_child(&fixture.root, data.path(), LeaseKind::Repository);

    let error = service
        .rebuild_repository(RebuildRepositoryRequest {
            root: linked,
            operation_id: OperationId::new(),
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    holder.release();
    assert!(
        service
            .refresh_repository(RefreshRepositoryRequest {
                root: fixture.root.clone(),
                operation_id: OperationId::new()
            })
            .is_ok()
    );
}

#[test]
fn common_git_lease_does_not_block_unrelated_roots_or_after_child_termination() {
    let first = support::born_repository();
    let second = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&first.root))
        .unwrap();
    service
        .enable(support::enable_request(&second.root))
        .unwrap();
    let holder = support::hold_lease_in_child(&first.root, data.path(), LeaseKind::Repository);

    assert!(
        service
            .refresh_repository(RefreshRepositoryRequest {
                root: second.root.clone(),
                operation_id: OperationId::new()
            })
            .is_ok()
    );
    holder.terminate();
    assert!(
        service
            .refresh_repository(RefreshRepositoryRequest {
                root: first.root.clone(),
                operation_id: OperationId::new()
            })
            .is_ok()
    );
}

#[test]
fn bootstrap_lease_is_bounded_and_releases() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let holder = support::hold_lease_in_child(&fixture.root, data.path(), LeaseKind::Bootstrap);

    let error = match RepositoryService::hold_lease_for_testing(
        &fixture.root,
        data.path(),
        LeaseKind::Bootstrap,
    ) {
        Ok(_) => panic!("bootstrap lease unexpectedly acquired"),
        Err(error) => error,
    };

    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    holder.release();
    assert!(
        RepositoryService::hold_lease_for_testing(&fixture.root, data.path(), LeaseKind::Bootstrap)
            .is_ok()
    );
}

#[test]
fn cache_lease_allows_shared_readers_and_blocks_exclusive_access() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let reader = support::hold_lease_in_child(&fixture.root, data.path(), LeaseKind::CacheRead);

    assert!(
        RepositoryService::hold_lease_for_testing(&fixture.root, data.path(), LeaseKind::CacheRead)
            .is_ok()
    );
    reader.release();
    let writer = support::hold_lease_in_child(&fixture.root, data.path(), LeaseKind::CacheWrite);

    let error = match RepositoryService::hold_lease_for_testing(
        &fixture.root,
        data.path(),
        LeaseKind::CacheRead,
    ) {
        Ok(_) => panic!("cache read lease unexpectedly acquired"),
        Err(error) => error,
    };

    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    writer.release();
}

#[test]
fn operation_ids_round_trip_in_canonical_uppercase_only() {
    let source = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let operation_id = OperationId::parse(source).unwrap();

    assert_eq!(operation_id.to_string(), source);
    assert!(OperationId::parse(&source.to_ascii_lowercase()).is_err());
}

#[test]
fn default_operation_id_is_canonical_and_parseable() {
    let operation_id = OperationId::default();

    assert_eq!(
        OperationId::parse(&operation_id.to_string()).unwrap(),
        operation_id
    );
}

#[test]
fn expected_path_observations_hash_exact_bytes() {
    assert_ne!(
        ExpectedPathObservation::from_bytes(b"before"),
        ExpectedPathObservation::from_bytes(b"after")
    );
}

#[test]
fn index_pending_retains_the_authoritative_result() {
    let pending = IndexPending::new("authoritative result");

    assert_eq!(pending.authoritative, "authoritative result");
}

#[test]
fn default_fixture_ids_are_unique_and_retry_ids_are_explicitly_shared() {
    let operation_id = support::operation_id();
    let root = std::path::Path::new("/repository");
    let first_default = support::enable_request(root);
    let second_default = support::enable_request(root);
    let initial = support::enable_request_with_operation_id(root, operation_id);
    let retry = support::enable_request_with_operation_id(root, operation_id);

    assert_ne!(first_default.operation_id, second_default.operation_id);
    assert_eq!(initial.operation_id, retry.operation_id);
}

#[test]
fn reconstructed_enable_retry_requires_a_shared_operation_id() {
    let root = std::path::Path::new("/repository");
    let operation_id = support::new_operation_id();

    assert_eq!(
        support::enable_request_with_operation_id(root, operation_id).operation_id,
        support::enable_request_with_operation_id(root, operation_id).operation_id
    );
}
