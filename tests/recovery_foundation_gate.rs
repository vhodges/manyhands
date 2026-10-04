use std::{
    any::Any,
    fs,
    fs::OpenOptions,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use fs4::fs_std::FileExt;
use manyhands::repository::{
    AddRemoteRequest, AuthoringKind, AuthoringTarget, CommitIdentity, ConfigurationInspection,
    ContextIntent, CreateRepositoryRequest, DocumentDraft, EnableRepositoryOutcome,
    ExpectedPathObservation, FailurePoint, IndexPending, LeaseKind, LifecycleLeasePhase,
    OperationId, PublicationRemoteOutcome, RebuildRepositoryRequest, RecoveryInspection,
    RefreshRepositoryRequest, RemoveRegistrationOutcome, RemoveRegistrationRequest,
    RemoveRemoteRequest, RepositoryErrorKind, RepositoryService, SaveDocumentRequest, SaveOutcome,
    SetPublicationRemoteRequest,
};
use rusqlite::{Connection, params};

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
fn root_scoped_refresh_is_busy_until_the_common_git_lease_releases() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    let holder = support::hold_lease_in_child(&fixture.root, data.path(), LeaseKind::Repository);

    let error = service
        .refresh_repository(RefreshRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);

    holder.release();
    assert!(
        service
            .refresh_repository(RefreshRepositoryRequest {
                root: fixture.root.clone(),
                operation_id: OperationId::new(),
            })
            .is_ok()
    );
}

#[test]
fn authoring_context_and_save_wait_for_common_git_lease_then_replay_after_release() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    stage_configuration(&fixture);
    let context_operation = OperationId::new();
    let save_operation = OperationId::new();
    let context_target = || AuthoringTarget {
        root: fixture.root.clone(),
        kind: AuthoringKind::Document,
        item_id: support::document_id(),
        intent: ContextIntent::Create,
        operation_id: context_operation,
    };
    let save_request = || SaveDocumentRequest {
        target: AuthoringTarget {
            root: fixture.root.clone(),
            kind: AuthoringKind::Document,
            item_id: support::document_id(),
            intent: ContextIntent::Create,
            operation_id: save_operation,
        },
        source_path: None,
        destination_path: PathBuf::from("docs/lease.md"),
        draft: DocumentDraft {
            title: "Lease replay".to_owned(),
            body: "draft remains caller-owned\n".to_owned(),
        },
        expected_source: None,
        expected_destination: ExpectedPathObservation::Missing,
    };
    let holder = support::hold_lease_in_child(&fixture.root, data.path(), LeaseKind::Repository);

    assert_eq!(
        match service.prepare_context(context_target()) {
            Ok(_) => panic!("lease-held context provision must be busy"),
            Err(error) => error.kind,
        },
        RepositoryErrorKind::RepositoryBusy
    );
    assert_eq!(
        match service.save_document(save_request()) {
            Ok(_) => panic!("lease-held document save must be busy"),
            Err(error) => error.kind,
        },
        RepositoryErrorKind::RepositoryBusy
    );

    holder.release();
    let context = service.prepare_context(context_target()).unwrap();
    assert!(matches!(
        context,
        manyhands::repository::ContextProvisionOutcome::Created(_)
            | manyhands::repository::ContextProvisionOutcome::Reused(_)
    ));
    assert!(matches!(
        service.save_document(save_request()).unwrap(),
        SaveOutcome::Saved { .. }
    ));
}

#[test]
fn authoring_save_respects_an_in_process_repository_lease() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    stage_configuration(&fixture);
    let repository = git2::Repository::open(&fixture.root).unwrap();
    let lock_path = repository.commondir().join("manyhands-operation.lock");
    let (acquired_sender, acquired) = mpsc::channel();
    let (release, release_receiver) = mpsc::channel();
    let holder = thread::spawn(move || {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .unwrap();
        file.lock_exclusive().unwrap();
        acquired_sender.send(()).unwrap();
        release_receiver.recv().unwrap();
        file.unlock().unwrap();
    });
    acquired.recv().unwrap();

    let start = Instant::now();
    let error = match service.save_document(SaveDocumentRequest {
        target: AuthoringTarget {
            root: fixture.root.clone(),
            kind: AuthoringKind::Document,
            item_id: support::document_id(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        },
        source_path: None,
        destination_path: PathBuf::from("docs/in-process-lease.md"),
        draft: DocumentDraft {
            title: "Lease holder".to_owned(),
            body: "must not block indefinitely\n".to_owned(),
        },
        expected_source: None,
        expected_destination: ExpectedPathObservation::Missing,
    }) {
        Ok(_) => panic!("lease-held document save unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    assert!(start.elapsed() < Duration::from_secs(1));

    release.send(()).unwrap();
    holder.join().unwrap();
}

#[test]
fn remove_remote_holds_the_lease_through_its_unselected_observation() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let holder = RepositoryService::open_at(data.path()).unwrap();
    let contender = RepositoryService::open_at(data.path()).unwrap();
    holder
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    holder
        .add_remote(AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:project.git".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    stage_configuration(&fixture);
    let (observed_at, observed) = mpsc::channel();
    let (released, release) = mpsc::channel();
    holder.set_lifecycle_lease_hook_for_testing(
        LifecycleLeasePhase::RemoveRemoteUnselected,
        move || {
            observed_at.send(()).unwrap();
            release.recv().unwrap();
        },
    );

    let root = fixture.root.clone();
    let removing = thread::spawn(move || {
        holder.remove_remote(RemoveRemoteRequest {
            root,
            name: "origin".to_owned(),
            operation_id: OperationId::new(),
        })
    });
    observed.recv().unwrap();

    let busy = contender
        .set_publication_remote(SetPublicationRemoteRequest {
            root: fixture.root.clone(),
            name: Some("origin".to_owned()),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(busy.kind, RepositoryErrorKind::RepositoryBusy);

    released.send(()).unwrap();
    assert!(removing.join().unwrap().is_ok());
    let unavailable = contender
        .set_publication_remote(SetPublicationRemoteRequest {
            root: fixture.root.clone(),
            name: Some("origin".to_owned()),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(
        unavailable.kind,
        RepositoryErrorKind::UnavailablePublicationRemote
    );
    assert_remote_configuration(&contender, &fixture.root, None, &[]);
}

#[test]
fn publication_selection_holds_the_lease_through_its_remote_observation() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let holder = RepositoryService::open_at(data.path()).unwrap();
    let contender = RepositoryService::open_at(data.path()).unwrap();
    holder
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    holder
        .add_remote(AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:project.git".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    stage_configuration(&fixture);
    let (observed_at, observed) = mpsc::channel();
    let (released, release) = mpsc::channel();
    holder.set_lifecycle_lease_hook_for_testing(
        LifecycleLeasePhase::SetPublicationRemoteExisting,
        move || {
            observed_at.send(()).unwrap();
            release.recv().unwrap();
        },
    );

    let root = fixture.root.clone();
    let selecting = thread::spawn(move || {
        holder.set_publication_remote(SetPublicationRemoteRequest {
            root,
            name: Some("origin".to_owned()),
            operation_id: OperationId::new(),
        })
    });
    observed.recv().unwrap();

    let busy = contender
        .remove_remote(RemoveRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(busy.kind, RepositoryErrorKind::RepositoryBusy);

    released.send(()).unwrap();
    assert!(selecting.join().unwrap().is_ok());
    let selected = contender
        .remove_remote(RemoveRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(selected.kind, RepositoryErrorKind::SelectedRemoteRemoval);
    contender
        .set_publication_remote(SetPublicationRemoteRequest {
            root: fixture.root.clone(),
            name: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    contender
        .remove_remote(RemoveRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert_remote_configuration(&contender, &fixture.root, None, &[]);
}

fn assert_remote_configuration(
    service: &RepositoryService,
    root: &std::path::Path,
    publication_remote: Option<&str>,
    remotes: &[&str],
) {
    let inspection = service.inspect(root).unwrap();
    assert!(matches!(
        inspection.configuration,
        ConfigurationInspection::Valid(ref config)
            if config.publication_remote.as_deref() == publication_remote
    ));
    let names = inspection
        .remotes
        .iter()
        .map(|remote| remote.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, remotes);
}

fn stage_configuration(fixture: &support::TestRepository) {
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(".manyhands/config.toml"))
        .unwrap();
    index.write().unwrap();
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
fn create_and_enable_reports_busy_while_the_root_bootstrap_lease_is_held_then_replays() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("created");
    let request = CreateRepositoryRequest {
        root: root.clone(),
        primary_branch: "main".to_owned(),
        identity: Some(CommitIdentity {
            name: "Created Author".to_owned(),
            email: "created@example.invalid".to_owned(),
        }),
        operation_id: OperationId::new(),
    };
    let holder = support::hold_lease_in_child(&root, data.path(), LeaseKind::Bootstrap);

    let error = RepositoryService::open_at(data.path())
        .unwrap()
        .create_and_enable(request.clone())
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RepositoryBusy);
    assert!(!root.exists());
    assert_eq!(registered_repository_count(data.path()), 0);
    assert_eq!(operation_record_count(data.path()), 0);

    holder.release();
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        service.create_and_enable(request).unwrap(),
        EnableRepositoryOutcome::Enabled { .. }
    ));
    assert_eq!(commit_count(&git2::Repository::open(&root).unwrap()), 1);
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
fn pending_lifecycle_records_block_differently_identified_mutations_without_side_effects() {
    for action in [
        "create_and_enable",
        "enable",
        "set_publication_remote",
        "remove_registration",
        "add_remote",
        "remove_remote",
        "refresh",
        "rebuild",
    ] {
        let fixture = support::born_repository();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(support::enable_request(&fixture.root))
            .unwrap();
        let root = fixture.root.canonicalize().unwrap();
        service
            .with_registry_connection_for_testing(|connection| {
                connection
                    .execute(
                        "INSERT INTO operation_records (
                            root_path, operation_ulid, action, target, state, observed_at
                         ) VALUES (?1, ?2, ?3, 'pending-target', 'created', 0)",
                        params![
                            root.to_str().unwrap(),
                            OperationId::new().to_string(),
                            action
                        ],
                    )
                    .unwrap();
            })
            .unwrap();
        let repository_before = support::repository_and_worktree_snapshot(&fixture);
        let cache_before = cache_snapshot(data.path());

        let error = service
            .enable(support::enable_request_with_operation_id(
                &fixture.root,
                OperationId::new(),
            ))
            .unwrap_err();

        assert_eq!(
            error.kind,
            RepositoryErrorKind::RecoveryRequired,
            "{action}"
        );
        assert_eq!(
            support::repository_and_worktree_snapshot(&fixture),
            repository_before
        );
        assert_eq!(cache_snapshot(data.path()), cache_before, "{action}");
    }
}

#[test]
fn pending_create_blocks_a_different_create_before_repository_initialization() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("created");
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute(
                    "INSERT INTO operation_records (
                        root_path, operation_ulid, action, target, state, observed_at
                     ) VALUES (?1, ?2, 'create_and_enable', 'main', 'created', 0)",
                    params![root.to_str().unwrap(), OperationId::new().to_string()],
                )
                .unwrap();
        })
        .unwrap();
    let cache_before = cache_snapshot(data.path());

    let error = service
        .create_and_enable(CreateRepositoryRequest {
            root: root.clone(),
            primary_branch: "main".to_owned(),
            identity: Some(CommitIdentity {
                name: "Created Author".to_owned(),
                email: "created@example.invalid".to_owned(),
            }),
            operation_id: OperationId::new(),
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert!(!root.exists());
    assert_eq!(cache_snapshot(data.path()), cache_before);
}

#[test]
fn pending_lifecycle_record_rejects_a_same_id_different_target() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    let operation_id = OperationId::new();
    let root = fixture.root.canonicalize().unwrap();
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute(
                    "INSERT INTO operation_records (
                        root_path, operation_ulid, action, target, state, observed_at
                     ) VALUES (?1, ?2, 'add_remote', 'origin\u{1f}git@example.invalid:first.git', 'created', 0)",
                    params![root.to_str().unwrap(), operation_id.to_string()],
                )
                .unwrap();
        })
        .unwrap();
    let repository_before = support::repository_and_worktree_snapshot(&fixture);
    let cache_before = cache_snapshot(data.path());

    let error = service
        .add_remote(AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:second.git".to_owned(),
            operation_id,
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
    assert_eq!(
        support::repository_and_worktree_snapshot(&fixture),
        repository_before
    );
    assert_eq!(cache_snapshot(data.path()), cache_before);
}

#[test]
fn migrated_legacy_rebuild_resumes_after_structural_context_migration() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    support::create_cycle_04_registry(data.path(), &fixture.root, "rebuild", "retry");
    let service = RepositoryService::open_at(data.path()).unwrap();
    support::assert_legacy_operation_records_are_redacted_and_reset(data.path());

    let snapshot = service
        .rebuild_repository(RebuildRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();

    assert_eq!(snapshot.root, fixture.root.canonicalize().unwrap());
    assert!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty()
    );
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
fn successful_rebuild_attaches_a_pre_registration_record() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let operation_id = OperationId::new();
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    assert!(
        failing
            .rebuild_repository(RebuildRepositoryRequest {
                root: fixture.root.clone(),
                operation_id,
            })
            .is_err()
    );
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .rebuild_repository(RebuildRepositoryRequest {
            root: fixture.root.clone(),
            operation_id,
        })
        .unwrap();

    let attached = service
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row(
                    "SELECT operation_records.repository_id = repositories.id
                     FROM operation_records
                     JOIN repositories ON repositories.root_path = operation_records.root_path
                     WHERE operation_records.operation_ulid = ?1",
                    [operation_id.to_string()],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap()
        })
        .unwrap();
    assert!(attached);
}

#[test]
fn shared_operation_id_across_roots_returns_operation_mismatch() {
    let first = support::born_repository();
    let second = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let operation_id = OperationId::new();
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    assert!(
        failing
            .rebuild_repository(RebuildRepositoryRequest {
                root: first.root.clone(),
                operation_id,
            })
            .is_err()
    );
    let service = RepositoryService::open_at(data.path()).unwrap();
    let error = service
        .rebuild_repository(RebuildRepositoryRequest {
            root: second.root.clone(),
            operation_id,
        })
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
}

#[test]
fn failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    support::create_cycle_04_registry(data.path(), &fixture.root, "refresh", "retry");
    let database = data.path().join(manyhands::repository::REGISTRY_FILE);
    let connection = Connection::open(&database).unwrap();
    connection.execute("PRAGMA foreign_keys = OFF", []).unwrap();
    connection
        .execute(
            "ALTER TABLE index_operations RENAME TO broken_index_operations",
            [],
        )
        .unwrap();
    connection.execute_batch("CREATE TABLE index_operations (id INTEGER PRIMARY KEY, repository_id INTEGER NOT NULL, operation TEXT NOT NULL, state TEXT, context_path TEXT, persisted_context_count INTEGER, observed_at INTEGER);") .unwrap();
    connection
        .execute(
            "INSERT INTO index_operations SELECT * FROM broken_index_operations",
            [],
        )
        .unwrap();
    connection
        .execute("UPDATE index_operations SET observed_at = NULL", [])
        .unwrap();
    connection
        .execute("DROP TABLE broken_index_operations", [])
        .unwrap();
    drop(connection);

    assert!(RepositoryService::open_at(data.path()).is_err());
    let connection = Connection::open(&database).unwrap();
    for table in [
        "operation_records",
        "operation_record_contexts",
        "registry_migrations",
    ] {
        assert!(
            !connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get::<_, bool>(0)
                )
                .unwrap()
        );
    }
    let legacy_columns = connection
        .prepare("SELECT name FROM pragma_table_info('index_operations') ORDER BY cid")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        legacy_columns,
        vec![
            "id",
            "repository_id",
            "operation",
            "state",
            "context_path",
            "persisted_context_count",
            "observed_at",
        ]
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM index_operations", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM index_operation_contexts", [], |row| {
                row.get::<_, i64>(0)
            },)
            .unwrap(),
        2
    );
    connection
        .execute("UPDATE index_operations SET observed_at = 1", [])
        .unwrap();
    drop(connection);
    RepositoryService::open_at(data.path()).unwrap();
    support::assert_legacy_operation_records_are_redacted_and_reset(data.path());
    let connection = Connection::open(&database).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM registry_migrations WHERE name = 'cycle_05_operation_records'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
    drop(connection);
    RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(&database).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM registry_migrations WHERE name = 'cycle_05_operation_records'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
}

#[test]
fn registration_removal_does_not_delete_a_different_pending_record() {
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
                [fixture.root.canonicalize().unwrap().to_str().unwrap(), OperationId::new().to_string().as_str()],
            ).unwrap();
            connection.execute_batch(
                "CREATE TRIGGER fail_registration_removal BEFORE DELETE ON repositories
                 BEGIN SELECT RAISE(ABORT, 'injected registration removal failure'); END;",
            ).unwrap();
        })
        .unwrap();

    let error = service
        .remove_registration(RemoveRegistrationRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert_eq!(service.recovery_inspection(&fixture.root).unwrap().len(), 1);
}

#[test]
fn registration_removal_requires_its_own_pending_record() {
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

    let error = service
        .remove_registration(RemoveRegistrationRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert_eq!(service.recovery_inspection(&fixture.root).unwrap().len(), 1);
}

#[test]
fn enable_replay_records_completed_lifecycle_once() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let operation_id = OperationId::new();
    let service = RepositoryService::open_at(data.path()).unwrap();

    service
        .enable(support::enable_request_with_operation_id(
            &fixture.root,
            operation_id,
        ))
        .unwrap();
    drop(service);

    let replay = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        replay
            .enable(support::enable_request_with_operation_id(
                &fixture.root,
                operation_id
            ))
            .is_ok()
    );
    let records = replay
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row(
                    "SELECT COUNT(*) FROM operation_records
                     WHERE root_path = ?1 AND operation_ulid = ?2 AND action = 'enable'
                       AND state = 'completed'",
                    [
                        fixture.root.canonicalize().unwrap().to_str().unwrap(),
                        operation_id.to_string().as_str(),
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap()
        })
        .unwrap();
    assert_eq!(records, 1);
}

#[test]
fn remote_replay_records_completed_lifecycle_once() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    let operation_id = OperationId::new();

    service
        .add_remote(manyhands::repository::AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:project.git".to_owned(),
            operation_id,
        })
        .unwrap();
    drop(service);

    let replay = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        replay
            .add_remote(manyhands::repository::AddRemoteRequest {
                root: fixture.root.clone(),
                name: "origin".to_owned(),
                url: "git@example.invalid:project.git".to_owned(),
                operation_id,
            })
            .is_ok()
    );
    let records = replay
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row(
                    "SELECT COUNT(*) FROM operation_records
                     WHERE root_path = ?1 AND operation_ulid = ?2 AND action = 'add_remote'
                       AND state = 'completed'",
                    [
                        fixture.root.canonicalize().unwrap().to_str().unwrap(),
                        operation_id.to_string().as_str(),
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap()
        })
        .unwrap();
    assert_eq!(records, 1);
}

#[test]
fn remote_replay_rejects_a_different_url_for_the_same_operation_id() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    let operation_id = OperationId::new();

    service
        .add_remote(manyhands::repository::AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:first.git".to_owned(),
            operation_id,
        })
        .unwrap();

    let error = service
        .add_remote(manyhands::repository::AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:second.git".to_owned(),
            operation_id,
        })
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
}

#[test]
fn create_replay_after_registration_failure_resumes_the_create_action() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("created");
    let operation_id = OperationId::new();
    let request = CreateRepositoryRequest {
        root: root.clone(),
        primary_branch: "main".to_owned(),
        identity: Some(CommitIdentity {
            name: "Created Author".to_owned(),
            email: "created@example.invalid".to_owned(),
        }),
        operation_id,
    };
    let failing =
        support::FailOnce::at(FailurePoint::BeforeRegistryWrite).open_service(data.path());

    let EnableRepositoryOutcome::RegistrationPending { commit_oid } =
        failing.create_and_enable(request.clone()).unwrap()
    else {
        panic!("expected registration to remain pending");
    };
    drop(failing);
    let repository = git2::Repository::open(&root).unwrap();
    assert_eq!(support::head_commit(&repository), Some(commit_oid));
    assert_eq!(
        RepositoryService::open_at(data.path())
            .unwrap()
            .recovery_inspection(&root)
            .unwrap()
            .len(),
        1
    );

    let replay = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        replay.create_and_enable(request).unwrap(),
        EnableRepositoryOutcome::AlreadyEnabled
    );
    assert_eq!(support::head_commit(&repository), Some(commit_oid));
    assert!(!replay.repository_snapshot(&root).unwrap().refresh_required);
}

#[test]
fn create_replay_after_repository_initialization_uses_the_observed_step() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("created");
    let request = CreateRepositoryRequest {
        root: root.clone(),
        primary_branch: "main".to_owned(),
        identity: Some(CommitIdentity {
            name: "Created Author".to_owned(),
            email: "created@example.invalid".to_owned(),
        }),
        operation_id: OperationId::new(),
    };
    let failing = support::FailOnce::at(FailurePoint::AfterRepositoryInitialization)
        .open_service(data.path());
    assert!(failing.create_and_enable(request.clone()).is_err());
    drop(failing);
    let repository = git2::Repository::open(&root).unwrap();
    assert!(repository.is_empty().unwrap());

    let replay = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        replay.create_and_enable(request).unwrap(),
        EnableRepositoryOutcome::Enabled { .. }
    ));
    assert!(repository.head().unwrap().peel_to_commit().is_ok());
}

#[test]
fn enable_replay_after_initialization_commit_retains_commit_and_registers_once() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let request = support::enable_request_with_operation_id(&fixture.root, OperationId::new());
    let failing =
        support::FailOnce::at(FailurePoint::BeforeRegistryWrite).open_service(data.path());

    let EnableRepositoryOutcome::RegistrationPending { commit_oid } =
        failing.enable(request.clone()).unwrap()
    else {
        panic!("expected registration to remain pending");
    };
    let before = support::repository_and_worktree_snapshot(&fixture);
    assert_eq!(commit_count(&fixture.repository), 2);
    assert_eq!(registered_rows(data.path(), &fixture.root), 0);
    assert_eq!(operation_rows(data.path(), &fixture.root), 1);
    drop(failing);

    assert_eq!(
        RepositoryService::open_at(data.path())
            .unwrap()
            .enable(request)
            .unwrap(),
        EnableRepositoryOutcome::AlreadyEnabled
    );
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_eq!(commit_count(&fixture.repository), 2);
    assert_eq!(registered_rows(data.path(), &fixture.root), 1);
    assert!(
        !RepositoryService::open_at(data.path())
            .unwrap()
            .repository_snapshot(&fixture.root)
            .unwrap()
            .refresh_required
    );
}

#[test]
fn index_pending_enable_replays_only_discovery_after_initialization_commit() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let request = support::enable_request_with_operation_id(&fixture.root, OperationId::new());
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());

    let EnableRepositoryOutcome::IndexPending(IndexPending {
        authoritative: commit_oid,
    }) = failing.enable(request.clone()).unwrap()
    else {
        panic!("expected discovery to remain pending");
    };
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_eq!(commit_count(&fixture.repository), 2);
    drop(failing);

    assert_eq!(
        RepositoryService::open_at(data.path())
            .unwrap()
            .enable(request)
            .unwrap(),
        EnableRepositoryOutcome::AlreadyEnabled
    );
    assert_eq!(commit_count(&fixture.repository), 2);
    assert!(
        RepositoryService::open_at(data.path())
            .unwrap()
            .repository_snapshot(&fixture.root)
            .is_ok()
    );
}

#[test]
fn publication_remote_replay_after_configuration_commit_retains_selection_and_registers_once() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let setup = RepositoryService::open_at(data.path()).unwrap();
    setup
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    setup
        .add_remote(AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:project.git".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(".manyhands/config.toml"))
        .unwrap();
    index.write().unwrap();
    setup
        .remove_registration(RemoveRegistrationRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let request = SetPublicationRemoteRequest {
        root: fixture.root.clone(),
        name: Some("origin".to_owned()),
        operation_id: OperationId::new(),
    };
    let failing =
        support::FailOnce::at(FailurePoint::BeforeRegistryWrite).open_service(data.path());

    let PublicationRemoteOutcome::RegistrationPending { commit_oid } =
        failing.set_publication_remote(request.clone()).unwrap()
    else {
        panic!("expected registration to remain pending");
    };
    let before = support::repository_and_worktree_snapshot(&fixture);
    assert_eq!(commit_count(&fixture.repository), 3);
    assert_eq!(registered_rows(data.path(), &fixture.root), 0);
    assert!(
        String::from_utf8(support::tracked_configuration(&fixture.root).unwrap())
            .unwrap()
            .contains("publication_remote = \"origin\"")
    );
    drop(failing);

    assert_eq!(
        RepositoryService::open_at(data.path())
            .unwrap()
            .set_publication_remote(request)
            .unwrap(),
        PublicationRemoteOutcome::Changed { commit_oid }
    );
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_eq!(commit_count(&fixture.repository), 3);
    assert_eq!(registered_rows(data.path(), &fixture.root), 1);
    assert!(
        !RepositoryService::open_at(data.path())
            .unwrap()
            .repository_snapshot(&fixture.root)
            .unwrap()
            .refresh_required
    );
}

#[test]
fn registration_removal_replay_preserves_failed_state_then_removes_once_and_noops() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let setup = RepositoryService::open_at(data.path()).unwrap();
    setup
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    let request = RemoveRegistrationRequest {
        root: fixture.root.clone(),
        operation_id: OperationId::new(),
    };
    let failing = support::FailOnce::at(FailurePoint::BeforeRegistrationRemovalTransaction)
        .open_service(data.path());
    let before = support::repository_and_worktree_snapshot(&fixture);

    assert_eq!(
        failing
            .remove_registration(request.clone())
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(registered_rows(data.path(), &fixture.root), 1);
    assert_eq!(operation_rows(data.path(), &fixture.root), 2);
    drop(failing);

    let replay = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        replay.remove_registration(request.clone()).unwrap(),
        RemoveRegistrationOutcome::Removed
    );
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(registered_rows(data.path(), &fixture.root), 0);
    assert_eq!(operation_rows(data.path(), &fixture.root), 0);
    assert_eq!(
        replay.remove_registration(request).unwrap(),
        RemoveRegistrationOutcome::NotRegistered
    );
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

fn commit_count(repository: &git2::Repository) -> usize {
    let mut commits = repository.revwalk().unwrap();
    commits.push_head().unwrap();
    commits.count()
}

fn cache_snapshot(data_directory: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    [
        manyhands::repository::REGISTRY_FILE.to_owned(),
        format!("{}-wal", manyhands::repository::REGISTRY_FILE),
        format!("{}-shm", manyhands::repository::REGISTRY_FILE),
    ]
    .into_iter()
    .filter_map(|name| {
        let path = data_directory.join(name);
        fs::read(&path).ok().map(|bytes| (path, bytes))
    })
    .collect()
}

fn registered_rows(data_directory: &std::path::Path, root: &std::path::Path) -> i64 {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    connection
        .query_row(
            "SELECT COUNT(*) FROM repositories WHERE root_path = ?1",
            [root.canonicalize().unwrap().to_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap()
}

fn operation_rows(data_directory: &std::path::Path, root: &std::path::Path) -> i64 {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    connection
        .query_row(
            "SELECT COUNT(*) FROM operation_records WHERE root_path = ?1",
            [root.canonicalize().unwrap().to_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap()
}

fn registered_repository_count(data_directory: &std::path::Path) -> i64 {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    connection
        .query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))
        .unwrap()
}

fn operation_record_count(data_directory: &std::path::Path) -> i64 {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    connection
        .query_row("SELECT COUNT(*) FROM operation_records", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn add_remote_replay_after_authoritative_mutation_uses_one_remote() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let setup = RepositoryService::open_at(data.path()).unwrap();
    setup
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    let request = AddRemoteRequest {
        root: fixture.root.clone(),
        name: "origin".to_owned(),
        url: "git@example.invalid:project.git".to_owned(),
        operation_id: OperationId::new(),
    };
    let failing =
        support::FailOnce::at(FailurePoint::AfterRemoteMutation).open_service(data.path());
    assert_eq!(
        failing.add_remote(request.clone()).unwrap_err().kind,
        RepositoryErrorKind::InjectedFailure
    );
    let before = support::repository_and_worktree_snapshot(&fixture);
    drop(failing);

    RepositoryService::open_at(data.path())
        .unwrap()
        .add_remote(request)
        .unwrap();
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(
        fixture.repository.find_remote("origin").unwrap().url(),
        Some("git@example.invalid:project.git")
    );
}

#[test]
fn remove_remote_replay_after_authoritative_mutation_keeps_it_absent() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let setup = RepositoryService::open_at(data.path()).unwrap();
    setup
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    setup
        .add_remote(AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:project.git".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let request = RemoveRemoteRequest {
        root: fixture.root.clone(),
        name: "origin".to_owned(),
        operation_id: OperationId::new(),
    };
    let failing =
        support::FailOnce::at(FailurePoint::AfterRemoteMutation).open_service(data.path());
    assert_eq!(
        failing.remove_remote(request.clone()).unwrap_err().kind,
        RepositoryErrorKind::InjectedFailure
    );
    let before = support::repository_and_worktree_snapshot(&fixture);
    drop(failing);

    RepositoryService::open_at(data.path())
        .unwrap()
        .remove_remote(request)
        .unwrap();
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert!(fixture.repository.find_remote("origin").is_err());
}

#[test]
fn pending_remote_removal_blocks_a_different_lifecycle_request_until_replayed() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let setup = RepositoryService::open_at(data.path()).unwrap();
    setup
        .enable(support::enable_request(&fixture.root))
        .unwrap();
    setup
        .add_remote(AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:project.git".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let remove = RemoveRemoteRequest {
        root: fixture.root.clone(),
        name: "origin".to_owned(),
        operation_id: OperationId::new(),
    };
    let failing =
        support::FailOnce::at(FailurePoint::AfterRemoteMutation).open_service(data.path());
    assert!(failing.remove_remote(remove.clone()).is_err());
    drop(failing);
    let before = support::repository_and_worktree_snapshot(&fixture);
    let fresh = RepositoryService::open_at(data.path()).unwrap();
    let error = fresh
        .add_remote(AddRemoteRequest {
            root: fixture.root.clone(),
            name: "other".to_owned(),
            url: "git@example.invalid:other.git".to_owned(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    fresh.remove_remote(remove).unwrap();
}
