use std::{
    any::Any,
    fs::OpenOptions,
    path::PathBuf,
    time::{Duration, Instant},
};

use manyhands::repository::{
    ExpectedPathObservation, FailurePoint, IndexPending, LeaseKind, OperationId,
    RebuildRepositoryRequest, RecoveryInspection, RefreshRepositoryRequest,
    RemoveRegistrationRequest, RepositoryErrorKind, RepositoryService,
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
fn failed_lease_holder_is_reported_before_ready_timeout() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let start = Instant::now();
    let panic = std::panic::catch_unwind(|| {
        support::hold_lease_in_child(
            &fixture.root.join("missing"),
            data.path(),
            LeaseKind::Repository,
        );
    })
    .unwrap_err();

    assert!(start.elapsed() < Duration::from_secs(2));
    let message = panic_message(panic.as_ref());
    assert!(message.contains("child exited before ready"));
    assert!(message.contains("exit status"));
    assert!(message.contains("No such file or directory"));
}

fn panic_message(panic: &(dyn Any + Send)) -> &str {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or_default()
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
    service
        .refresh_repository(RefreshRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
}

#[test]
fn legacy_migration_copies_multiple_contexts_once_without_retaining_their_fingerprints() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    support::create_cycle_04_registry(data.path(), &fixture.root, "refresh", "retry");

    let first = RepositoryService::open_at(data.path()).unwrap();
    support::assert_legacy_operation_records_are_redacted_and_reset(data.path());
    drop(first);
    let second = RepositoryService::open_at(data.path()).unwrap();
    support::assert_legacy_operation_records_are_redacted_and_reset(data.path());
    assert!(matches!(
        second
            .recovery_inspection(&fixture.root)
            .unwrap()
            .as_slice(),
        [RecoveryInspection::LegacyIndexOperation {
            operation: manyhands::repository::RepositoryOperation::RefreshRepository,
            ..
        }]
    ));
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

#[test]
fn migration_moves_incomplete_cycle_04_refresh_to_a_resumable_legacy_record() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    support::create_cycle_04_registry(data.path(), &fixture.root, "refresh", "retry");

    let service = RepositoryService::open_at(data.path()).unwrap();
    support::assert_legacy_operation_records_are_redacted_and_reset(data.path());
    support::assert_legacy_operation_records_are_redacted_and_reset(data.path());
    assert!(matches!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .as_slice(),
        [RecoveryInspection::LegacyIndexOperation {
            operation: manyhands::repository::RepositoryOperation::RefreshRepository,
            next_action: manyhands::repository::RepositoryOperation::RefreshRepository,
            ..
        }]
    ));

    service
        .refresh_repository(RefreshRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn legacy_index_record_can_only_resume_with_its_matching_action() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    support::create_cycle_04_registry(data.path(), &fixture.root, "refresh", "retry");
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service
        .rebuild_repository(RebuildRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
}

#[test]
fn root_operation_is_recorded_before_registration_without_content() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    let operation_id = OperationId::new();

    assert!(
        service
            .rebuild_repository(RebuildRepositoryRequest {
                root: fixture.root.clone(),
                operation_id,
            })
            .is_err()
    );
    assert!(matches!(
        service.recovery_inspection(&fixture.root).unwrap().as_slice(),
        [RecoveryInspection::Pending {
            operation_id: found,
            operation: manyhands::repository::RepositoryOperation::RebuildRepository,
            ..
        }] if *found == operation_id
    ));
    support::assert_operation_records_hold_no_content(data.path());
}

#[test]
fn removing_registration_clears_root_recovery_records() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    service
        .with_registry_connection_for_testing(|connection| {
            connection.execute(
                "INSERT INTO operation_records (root_path, operation_ulid, action, state, observed_at)
                 VALUES (?1, ?2, 'refresh', 'observed', 1)",
                [
                    fixture.root.canonicalize().unwrap().to_str().unwrap(),
                    OperationId::new().to_string().as_str(),
                ],
            )
        })
        .unwrap()
        .unwrap();

    service
        .remove_registration(RemoveRegistrationRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();

    assert!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty()
    );
}
