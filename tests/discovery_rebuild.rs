use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};

use manyhands::{
    canonical,
    repository::{
        AuthoringKind, AuthoringTarget, ContextIntent, DiscoveredCommentThread, DiscoveredContext,
        DiscoveredItem, DiscoveryActivitySource, DiscoveryContextKind, DiscoveryProblem,
        DocumentDraft, FailurePoint, LeaseKind, LocalCheckpoint, RefreshOutcome,
        RepositoryErrorKind, RepositoryOperation, RepositoryService, RepositorySnapshot,
        SaveDocumentRequest, SaveOutcome, SnapshotConfiguration,
    },
};
use rusqlite::{Connection, params};
use time::OffsetDateTime;

mod support;

#[test]
fn cache_lease_child() {
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
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !release.exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

macro_rules! refresh_request {
    ($root:expr, $operation_id:expr) => {
        manyhands::repository::RefreshRepositoryRequest {
            root: $root.to_owned(),
            operation_id: $operation_id,
        }
    };
    ($root:expr) => {
        refresh_request!($root, support::operation_id())
    };
}

macro_rules! rebuild_request {
    ($root:expr, $operation_id:expr) => {
        manyhands::repository::RebuildRepositoryRequest {
            root: $root.to_owned(),
            operation_id: $operation_id,
        }
    };
    ($root:expr) => {
        rebuild_request!($root, support::operation_id())
    };
}

#[test]
fn retry_requests_retain_the_same_operation_id() {
    let root = PathBuf::from("/repository");
    let operation_id = support::operation_id();
    let initial = refresh_request!(&root, operation_id);
    let retry = refresh_request!(&root, operation_id);

    assert_eq!(initial.operation_id, retry.operation_id);
}

#[test]
fn concurrent_refreshes_with_the_same_operation_id_have_one_index_owner() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let second_service = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    second_service.set_observation_hook_for_testing(|| panic!("non-owner must not scan"));
    let operation_id = support::operation_id();
    let (claimed_send, claimed_receive) = mpsc::sync_channel(0);
    let (release_send, release_receive) = mpsc::sync_channel(0);
    enabled.service.set_refresh_claim_hook_for_testing(move || {
        claimed_send.send(()).unwrap();
        release_receive.recv().unwrap();
    });

    let outcomes = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            enabled
                .service
                .refresh_repository(refresh_request!(&fixture.root, operation_id))
        });
        claimed_receive.recv().unwrap();
        let second = scope.spawn(|| {
            second_service.refresh_repository(refresh_request!(&fixture.root, operation_id))
        });
        release_send.send(()).unwrap();
        [
            first.join().unwrap().unwrap(),
            second.join().unwrap().unwrap(),
        ]
    });

    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, RefreshOutcome::Refreshed { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, RefreshOutcome::IndexPending { .. }))
            .count(),
        1
    );
}

#[test]
fn refresh_reclaims_an_interrupted_index_owner_on_exact_replay() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let operation_id = support::operation_id();
    let interrupted = support::FailOnce::at(FailurePoint::AfterIndexClaim)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        interrupted
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert!(matches!(
        RepositoryService::open_at(enabled.data_directory.path())
            .unwrap()
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap(),
        RefreshOutcome::Refreshed { .. }
    ));
}

#[derive(Debug, PartialEq, Eq)]
struct AvailableState {
    head: Option<git2::Oid>,
    fixture: Vec<u8>,
    registry_files: BTreeMap<PathBuf, Vec<u8>>,
}

fn available_state(fixture: &support::TestRepository, data: &Path) -> AvailableState {
    let registry_files = fs::read_dir(data)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = path.file_name().unwrap().to_owned();
            (PathBuf::from(name), fs::read(path).unwrap())
        })
        .collect();

    AvailableState {
        head: fixture.repository.head().unwrap().target(),
        fixture: fs::read(fixture.root.join("fixture.txt")).unwrap(),
        registry_files,
    }
}

#[test]
fn snapshot_reads_stored_metadata_in_deterministic_order() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let root = fixture.root.canonicalize().unwrap();
    let primary = root.clone();
    let active = root
        .join(".manyhands/worktrees")
        .join(support::document_id().to_string());
    let document_id = support::document_id();
    let ticket_id = support::ticket_id();
    let root_comment_id = support::root_comment_id();
    let reply_id = support::reply_id();
    let later_reply_id = "01J00000000000000000000002"
        .parse::<canonical::ItemId>()
        .unwrap();
    let first_root_comment_id = "01B00000000000000000000001"
        .parse::<canonical::ItemId>()
        .unwrap();
    let first_reply_id = "01C00000000000000000000001"
        .parse::<canonical::ItemId>()
        .unwrap();

    service
        .with_registry_connection_for_testing(|connection| {
            connection.execute(
                "INSERT INTO repositories (root_path, enabled_at, accessibility, config_blob_oid, refresh_required)
                 VALUES (?1, 1, 'accessible', '0123456789012345678901234567890123456789', 1)",
                [root.to_str().unwrap()],
            ).unwrap();
            let repository_id = connection.last_insert_rowid();
            connection.execute(
                "INSERT INTO configuration_observations (
                    repository_id, state, primary_branch, publication_remote
                ) VALUES (?1, 'valid', 'main', 'origin')",
                [repository_id],
            ).unwrap();
            connection.execute(
                "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id, head_oid)
                 VALUES (?1, 'active', ?2, ?3, ?4, ?5)",
                params![repository_id, format!("manyhands/document/{document_id}"), active.to_str().unwrap(), document_id.to_string(), "0123456789012345678901234567890123456789"],
            ).unwrap();
            let active_context_id = connection.last_insert_rowid();
            connection.execute(
                "INSERT INTO contexts (repository_id, kind, branch, worktree_path)
                 VALUES (?1, 'primary', 'main', ?2)",
                params![repository_id, primary.to_str().unwrap()],
            ).unwrap();
            let primary_context_id = connection.last_insert_rowid();
            connection.execute(
                "INSERT INTO discovered_items (context_id, item_id, kind, canonical_path, title, ticket_type, status, project, team, closed_at, activity_at, activity_source)
                 VALUES (?1, ?2, 'ticket', 'tickets/z.md', 'Zulu ticket', 'bug', 'open', 'Core', 'Platform', 30, 20, 'filesystem')",
                params![primary_context_id, ticket_id.to_string()],
            ).unwrap();
            connection.execute(
                "INSERT INTO discovered_items (context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
                 VALUES (?1, ?2, 'document', 'docs/a.md', 'Alpha document', 10, 'git')",
                params![active_context_id, document_id.to_string()],
            ).unwrap();
            let document_row_id = connection.last_insert_rowid();
            for (comment_id, parent_comment_id, path, created_at) in [
                (first_reply_id.to_string(), Some(root_comment_id.to_string()), ".manyhands/comments/first-reply.md", 10_i64),
                (first_root_comment_id.to_string(), None, ".manyhands/comments/first-root.md", 10),
                (later_reply_id.to_string(), Some(reply_id.to_string()), ".manyhands/comments/later.md", 11_i64),
                (reply_id.to_string(), Some(root_comment_id.to_string()), ".manyhands/comments/reply.md", 10),
                (root_comment_id.to_string(), None, ".manyhands/comments/root.md", 10),
            ] {
                connection.execute(
                    "INSERT INTO discovered_comments (item_id, comment_id, parent_comment_id, canonical_path, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![document_row_id, comment_id, parent_comment_id, path, created_at],
                ).unwrap();
            }
            connection.execute(
                "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
                 VALUES (?1, ?2, 'docs/a.md', 'later', 'later guidance', 20)",
                params![repository_id, primary_context_id],
            ).unwrap();
            connection.execute(
                "INSERT INTO problems (repository_id, code, guidance, observed_at)
                 VALUES (?1, 'earlier', 'earlier guidance', 10)",
                [repository_id],
            ).unwrap();
        })
        .unwrap();

    let snapshot = service.repository_snapshot(&fixture.root).unwrap();

    assert_eq!(snapshot.root, root);
    assert_eq!(
        snapshot.configuration,
        SnapshotConfiguration::Valid {
            primary_branch: "main".to_owned(),
            publication_remote: Some("origin".to_owned()),
        }
    );
    assert!(snapshot.refresh_required);
    assert_eq!(snapshot.contexts.len(), 2);
    assert_eq!(snapshot.contexts[0].worktree, primary);
    assert_eq!(snapshot.contexts[1].worktree, active);
    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(snapshot.items[0].id, ticket_id);
    assert_eq!(snapshot.items[0].kind, AuthoringKind::Ticket);
    assert_eq!(snapshot.items[1].id, document_id);
    assert_eq!(snapshot.items[1].kind, AuthoringKind::Document);
    assert_eq!(snapshot.items[0].ticket_type.as_deref(), Some("bug"));
    assert_eq!(snapshot.items[0].status.as_deref(), Some("open"));
    assert_eq!(snapshot.items[0].project.as_deref(), Some("Core"));
    assert_eq!(snapshot.items[0].team.as_deref(), Some("Platform"));
    assert_eq!(
        snapshot.items[0].closed_at,
        Some(OffsetDateTime::from_unix_timestamp(30).unwrap())
    );
    assert_eq!(
        snapshot.items[0].activity_source,
        DiscoveryActivitySource::UncommittedFilesystem
    );
    assert_eq!(snapshot.items[1].comments.len(), 2);
    assert_eq!(snapshot.items[1].comments[0].id, root_comment_id);
    assert_eq!(snapshot.items[1].comments[1].id, first_root_comment_id);
    assert_eq!(snapshot.items[1].comments[0].replies[0].id, reply_id);
    assert_eq!(snapshot.items[1].comments[0].replies[1].id, first_reply_id);
    assert_eq!(
        snapshot.items[1].comments[0].replies[0].replies[0].id,
        later_reply_id
    );
    assert_eq!(snapshot.problems.len(), 2);
    assert_eq!(snapshot.problems[0].code, "earlier");
    assert_eq!(snapshot.problems[1].code, "later");
}

#[test]
fn discovery_public_types_hold_metadata_only() {
    let root = PathBuf::from("/repository");
    let context = root.join(".manyhands/worktrees/document");
    let item_id = support::document_id();
    let comment_id = support::root_comment_id();
    let reply_id = support::reply_id();
    let observed_at = OffsetDateTime::UNIX_EPOCH;
    let snapshot = RepositorySnapshot {
        root: root.clone(),
        configuration: SnapshotConfiguration::Valid {
            primary_branch: "main".to_owned(),
            publication_remote: Some("origin".to_owned()),
        },
        refresh_required: true,
        contexts: vec![DiscoveredContext {
            kind: DiscoveryContextKind::Active,
            branch: Some("manyhands/document/item".to_owned()),
            worktree: context.clone(),
            item_id: Some(item_id.clone()),
            head_oid: None,
        }],
        items: vec![DiscoveredItem {
            context: context.clone(),
            id: item_id.clone(),
            kind: AuthoringKind::Document,
            path: PathBuf::from("docs/item.md"),
            title: "Item title".to_owned(),
            ticket_type: None,
            status: None,
            project: None,
            team: None,
            closed_at: None,
            activity_at: observed_at,
            activity_source: DiscoveryActivitySource::GitCommit,
            comments: vec![DiscoveredCommentThread {
                id: comment_id,
                path: PathBuf::from(".manyhands/comments/item/comment.md"),
                created_at: observed_at,
                replies: vec![DiscoveredCommentThread {
                    id: reply_id,
                    path: PathBuf::from(".manyhands/comments/item/reply.md"),
                    created_at: observed_at,
                    replies: Vec::new(),
                }],
            }],
        }],
        problems: vec![DiscoveryProblem {
            context: Some(context.clone()),
            path: Some(PathBuf::from("docs/item.md")),
            code: "invalid-front-matter".to_owned(),
            guidance: "repair front matter".to_owned(),
            observed_at,
        }],
    };
    let outcome = RefreshOutcome::Refreshed { snapshot };

    let RefreshOutcome::Refreshed { snapshot } = outcome else {
        panic!("expected a refreshed snapshot");
    };
    assert_eq!(snapshot.root, root);
    assert!(snapshot.refresh_required);
    assert!(matches!(
        snapshot.configuration,
        SnapshotConfiguration::Valid {
            ref primary_branch,
            ref publication_remote,
        } if primary_branch == "main" && publication_remote.as_deref() == Some("origin")
    ));
    assert_eq!(snapshot.contexts[0].worktree, context);
    assert_eq!(snapshot.items[0].id, item_id);
    assert_eq!(snapshot.items[0].activity_at, observed_at);
    assert!(matches!(
        snapshot.items[0].activity_source,
        DiscoveryActivitySource::GitCommit
    ));
    assert_eq!(snapshot.items[0].comments[0].replies.len(), 1);
    assert_eq!(snapshot.problems[0].code, "invalid-front-matter");
    assert!(matches!(
        SnapshotConfiguration::Invalid {
            code: canonical::ValidationCode::InvalidField,
            guidance: "fix field".to_owned(),
        },
        SnapshotConfiguration::Invalid { .. }
    ));
    assert!(matches!(
        SnapshotConfiguration::Missing,
        SnapshotConfiguration::Missing
    ));
    assert!(matches!(
        DiscoveryActivitySource::UncommittedFilesystem,
        DiscoveryActivitySource::UncommittedFilesystem
    ));
    assert!(matches!(
        RefreshOutcome::RetryRequired {
            root: PathBuf::from("/repository"),
            context: Some(PathBuf::from("/repository/.manyhands/worktrees/document")),
        },
        RefreshOutcome::RetryRequired { .. }
    ));
}

#[test]
fn snapshot_reads_a_registered_cache_without_mutating_available_state() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .with_registry_connection_for_testing(|connection| {
            connection.execute(
                "INSERT INTO repositories (root_path, enabled_at, accessibility, config_blob_oid, refresh_required)
                 VALUES (?1, 1, 'accessible', NULL, 0)",
                [fixture.root.to_str().unwrap()],
            )
            .unwrap();
        })
        .unwrap();
    let connection = Connection::open(data.path().join("manyhands.sqlite3")).unwrap();
    connection
        .execute_batch("PRAGMA journal_mode=DELETE")
        .unwrap();
    drop(connection);
    fs::File::create(data.path().join("manyhands.sqlite3.recovery.lock")).unwrap();
    let before = available_state(&fixture, data.path());

    let snapshot = service.repository_snapshot(&fixture.root).unwrap();

    assert_eq!(snapshot.root, fixture.root);
    assert_eq!(snapshot.configuration, SnapshotConfiguration::Missing);
    assert_eq!(available_state(&fixture, data.path()), before);
}

#[test]
fn refresh_requires_registration_but_rebuild_registers_the_explicit_root() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    let error = service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::RepositoryNotRegistered);
    assert_eq!(error.operation, RepositoryOperation::RefreshRepository);
    assert!(
        service
            .rebuild_repository(rebuild_request!(&fixture.root))
            .is_ok()
    );
}

#[test]
fn refresh_persists_primary_metadata_comments_and_activity() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    support::write_document_source(&fixture.root, "docs/visible.md");
    let comment = fixture
        .root
        .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
    fs::create_dir_all(comment.parent().unwrap()).unwrap();
    fs::write(comment, support::root_comment_source()).unwrap();

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap()
    else {
        panic!("expected stable refresh");
    };

    assert_eq!(snapshot.contexts.len(), 1);
    assert_eq!(snapshot.contexts[0].kind, DiscoveryContextKind::Primary);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(snapshot.items[0].id, support::document_id());
    assert_eq!(snapshot.items[0].comments[0].id, support::root_comment_id());
    assert_eq!(
        snapshot.items[0].activity_source,
        DiscoveryActivitySource::UncommittedFilesystem
    );
    assert!(!snapshot.refresh_required);
}

#[test]
fn refresh_keeps_primary_and_active_contexts_with_active_item_precedence() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    let failing = support::FailOnce::at(FailurePoint::BeforeRegistryWrite)
        .open_service(enabled.data_directory.path());
    let outcome = failing
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/active.md"),
            draft: DocumentDraft {
                title: "Active".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { .. },
            ..
        }
    ));
    support::write_document_source(&fixture.root, "docs/primary.md");

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap()
    else {
        panic!("expected stable refresh");
    };

    assert_eq!(snapshot.contexts.len(), 2);
    assert_eq!(snapshot.contexts[0].kind, DiscoveryContextKind::Primary);
    assert_eq!(snapshot.contexts[1].kind, DiscoveryContextKind::Active);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(snapshot.items[0].context, snapshot.contexts[1].worktree);
}

#[test]
fn refresh_after_pending_checkpoint_only_indexes_existing_git_state() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    enabled
        .service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/pending.md"),
            draft: DocumentDraft {
                title: "Pending".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();
    let before = commit_count(&fixture.repository);

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap()
    else {
        panic!("expected stable refresh");
    };

    assert_eq!(commit_count(&fixture.repository), before);
    assert!(!snapshot.refresh_required);
}

#[test]
fn refresh_removes_disappeared_context_and_item_rows() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    support::write_document_source(&fixture.root, "docs/visible.md");
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    fs::remove_file(fixture.root.join("docs/visible.md")).unwrap();

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap()
    else {
        panic!("expected stable refresh");
    };

    assert!(snapshot.items.is_empty());
    assert_eq!(snapshot.contexts.len(), 1);
}

#[test]
fn refresh_transaction_failure_retains_prior_rows_and_retries() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let initial = RepositoryService::open_at(data.path()).unwrap();
    initial
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    support::write_document_source(&fixture.root, "docs/visible.md");
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    let operation_id = support::operation_id();

    assert_eq!(
        failing
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert!(
        initial
            .repository_snapshot(&fixture.root)
            .unwrap()
            .items
            .is_empty()
    );
    let RefreshOutcome::Refreshed { snapshot } = initial
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap()
    else {
        panic!("expected stable retry");
    };
    assert_eq!(snapshot.items.len(), 1);
}

#[test]
fn refresh_sqlite_context_write_failure_preserves_git_and_recovers() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    support::write_document_source(&fixture.root, "docs/visible.md");
    let head = fixture.repository.head().unwrap().target();
    let index = support::index_bytes(&fixture.repository).unwrap();
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute_batch(
                    "CREATE TRIGGER fail_refresh_context_insert BEFORE INSERT ON contexts
             BEGIN SELECT RAISE(ABORT, 'injected context write failure'); END;",
                )
                .unwrap();
        })
        .unwrap();
    let operation_id = support::operation_id();

    let error = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::Sqlite);
    assert_eq!(error.operation, RepositoryOperation::RefreshRepository);
    assert_eq!(fixture.repository.head().unwrap().target(), head);
    assert_eq!(support::index_bytes(&fixture.repository).unwrap(), index);
    assert!(
        !enabled
            .service
            .repository_snapshot(&fixture.root)
            .unwrap()
            .refresh_required
    );

    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute_batch("DROP TRIGGER fail_refresh_context_insert;")
                .unwrap();
        })
        .unwrap();
    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap()
    else {
        panic!("expected retry recovery")
    };
    assert_eq!(snapshot.items.len(), 1);
    assert!(!snapshot.refresh_required);
}

#[test]
fn refresh_after_observation_failure_preserves_prior_rows_for_retry() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let initial = RepositoryService::open_at(data.path()).unwrap();
    initial
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    let failing =
        support::FailOnce::at(FailurePoint::AfterContextObservation).open_service(data.path());
    let operation_id = support::operation_id();

    assert_eq!(
        failing
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert!(
        !initial
            .repository_snapshot(&fixture.root)
            .unwrap()
            .refresh_required
    );
    assert!(matches!(
        initial
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap(),
        RefreshOutcome::Refreshed { .. }
    ));
}

#[test]
fn refresh_scan_race_retains_previous_rows_then_converges_on_retry() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let document = support::write_document_source(&fixture.root, "docs/visible.md");
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    enabled.service.set_observation_hook_for_testing({
        let document = document.clone();
        move || fs::write(document, "ordinary markdown\n").unwrap()
    });
    let operation_id = support::operation_id();

    assert!(matches!(
        enabled
            .service
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap(),
        RefreshOutcome::RetryRequired {
            context: Some(_),
            ..
        }
    ));
    let retained = enabled.service.repository_snapshot(&fixture.root).unwrap();
    assert_eq!(retained.items.len(), 1);
    assert!(retained.refresh_required);

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap()
    else {
        panic!("expected stable retry");
    };
    assert!(snapshot.items.is_empty());
    assert!(!snapshot.refresh_required);
    assert!(
        !snapshot
            .problems
            .iter()
            .any(|problem| problem.code == "retry-required")
    );
}

#[test]
fn refresh_does_not_mutate_git_or_canonical_content() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    support::write_document_source(&fixture.root, "docs/visible.md");
    let before = available_state(&fixture, enabled.data_directory.path());
    let index = support::index_bytes(&fixture.repository).unwrap();

    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();

    assert_eq!(
        available_state(&fixture, enabled.data_directory.path()).head,
        before.head
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("docs/visible.md")).unwrap(),
        support::document_source()
    );
    assert_eq!(support::index_bytes(&fixture.repository).unwrap(), index);
}

#[test]
fn no_mutation_refresh_preserves_root_and_linked_worktree_state() {
    let (fixture, enabled) = repository_with_primary_and_context_content();
    let before = support::repository_and_worktree_snapshot(&fixture);

    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();

    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn no_mutation_healthy_rebuild_preserves_root_and_linked_worktree_state() {
    let (fixture, enabled) = repository_with_primary_and_context_content();
    let before = support::repository_and_worktree_snapshot(&fixture);

    enabled
        .service
        .rebuild_repository(rebuild_request!(&fixture.root))
        .unwrap();

    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn no_mutation_corrupt_rebuild_preserves_root_and_linked_worktree_state() {
    let (fixture, enabled) = repository_with_primary_and_context_content();
    let data = enabled.data_directory.path();
    fs::remove_file(data.join("manyhands.sqlite3")).unwrap();
    fs::write(data.join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);

    service
        .rebuild_repository(rebuild_request!(&fixture.root))
        .unwrap();

    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn no_mutation_invalid_source_observation_preserves_root_and_linked_worktree_state() {
    let (fixture, enabled) = repository_with_primary_and_context_content();
    std::os::unix::fs::symlink("missing.md", fixture.root.join("docs/inaccessible.md")).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);

    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();

    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn no_mutation_sqlite_persistence_failure_preserves_root_and_linked_worktree_state() {
    let (fixture, enabled) = repository_with_primary_and_context_content();
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute_batch(
                    "CREATE TRIGGER fail_no_mutation_refresh BEFORE INSERT ON contexts
                     BEGIN SELECT RAISE(ABORT, 'injected persistence failure'); END;",
                )
                .unwrap();
        })
        .unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);

    assert_eq!(
        enabled
            .service
            .refresh_repository(refresh_request!(&fixture.root))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::Sqlite
    );

    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn no_mutation_scan_race_retry_preserves_state_after_external_change() {
    let (fixture, enabled) = repository_with_primary_and_context_content();
    let document = fixture.root.join("docs/primary.md");
    let root = fixture.root.clone();
    let observed = Arc::new(Mutex::new(None));
    enabled.service.set_observation_hook_for_testing({
        let observed = Arc::clone(&observed);
        move || {
            fs::write(&document, "ordinary markdown\n").unwrap();
            *observed.lock().unwrap() = Some(support::repository_and_worktree_snapshot_at(&root));
        }
    });

    assert!(matches!(
        enabled
            .service
            .refresh_repository(refresh_request!(&fixture.root))
            .unwrap(),
        RefreshOutcome::RetryRequired { .. }
    ));

    assert_eq!(
        support::repository_and_worktree_snapshot(&fixture),
        observed.lock().unwrap().take().unwrap()
    );
}

#[test]
fn fixture_snapshot_detects_head_and_symlink_target_changes_without_following_links() {
    let fixture = support::born_repository();
    let initial = support::repository_and_worktree_snapshot(&fixture);
    let initial_oid = fixture.repository.head().unwrap().target().unwrap();
    assert!(matches!(
        &initial.root.head,
        support::HeadState::Attached {
            symbolic_target,
            target,
        } if symbolic_target == "refs/heads/main" && *target == initial_oid
    ));

    let commit = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch("other", &commit, false).unwrap();
    fixture.repository.set_head("refs/heads/other").unwrap();
    let attached = support::repository_and_worktree_snapshot(&fixture);

    assert_ne!(attached, initial);
    assert!(matches!(
        &attached.root.head,
        support::HeadState::Attached {
            symbolic_target,
            target,
        } if symbolic_target == "refs/heads/other" && *target == initial_oid
    ));

    fixture.repository.set_head_detached(initial_oid).unwrap();
    let detached = support::repository_and_worktree_snapshot(&fixture);

    assert_ne!(detached, attached);
    assert!(matches!(
        detached.root.head,
        support::HeadState::Detached { target } if target == initial_oid
    ));

    let link = fixture.root.join("docs/link.md");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("first-target.md", &link).unwrap();
    let first_link = support::repository_and_worktree_snapshot(&fixture);
    assert!(matches!(
        first_link.root.files.get(Path::new("docs/link.md")),
        Some(support::FilesystemEntry::Symlink { target }) if target == Path::new("first-target.md")
    ));

    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink("second-target.md", &link).unwrap();
    let second_link = support::repository_and_worktree_snapshot(&fixture);

    assert_ne!(second_link, first_link);
    fs::remove_file(&link).unwrap();
    fs::write(&link, "replacement file\n").unwrap();
    let replacement_file = support::repository_and_worktree_snapshot(&fixture);

    assert_ne!(replacement_file, second_link);
    assert!(matches!(
        replacement_file.root.files.get(Path::new("docs/link.md")),
        Some(support::FilesystemEntry::File { bytes }) if bytes == b"replacement file\n"
    ));
}

fn repository_with_primary_and_context_content()
-> (support::TestRepository, support::EnabledRepository) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    enabled
        .service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/active.md"),
            draft: DocumentDraft {
                title: "Active".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();
    support::write_document_source(&fixture.root, "docs/primary.md");
    (fixture, enabled)
}

fn commit_count(repository: &git2::Repository) -> usize {
    repository.revwalk().unwrap().count()
}

fn refresh_operation_states(service: &RepositoryService) -> Vec<(String, Option<String>)> {
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .prepare("SELECT state, context_path FROM operation_records WHERE action = 'refresh' ORDER BY id")
                .unwrap()
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        })
        .unwrap()
}

fn refresh_operation_progress(service: &RepositoryService) -> (String, i64) {
    service.with_registry_connection_for_testing(|connection| {
        connection.query_row(
            "SELECT state, persisted_context_count FROM operation_records WHERE action = 'refresh' ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap()
    }).unwrap()
}

#[test]
fn refresh_does_not_persist_private_markdown_while_detecting_source_races() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let private_body = "PRIVATE-MARKDOWN-BODY-01ARZ3NDEKTSV4RRFFQ69G5FAZ";
    let document = fixture.root.join("docs/private.md");
    let source = format!("{}\n{private_body}\n", support::document_source());
    fs::create_dir_all(document.parent().unwrap()).unwrap();
    fs::write(&document, &source).unwrap();

    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    support::assert_operation_records_hold_no_content(enabled.data_directory.path());

    let changed_source = source.replace(private_body, "changed private Markdown body");
    enabled
        .service
        .set_observation_hook_for_testing(move || fs::write(document, changed_source).unwrap());
    let operation_id = support::operation_id();
    assert!(matches!(
        enabled
            .service
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap(),
        RefreshOutcome::RetryRequired { .. }
    ));

    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap();
    support::assert_operation_records_hold_no_content(enabled.data_directory.path());
}

#[test]
fn refresh_records_each_stable_context_persisted_before_completion() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    enabled
        .service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/active.md"),
            draft: DocumentDraft {
                title: "Active".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();

    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();

    assert_eq!(
        refresh_operation_progress(&enabled.service),
        ("completed".to_owned(), 2)
    );
}

#[test]
fn refresh_removes_rows_for_a_stably_disappeared_active_worktree() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    enabled
        .service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/active.md"),
            draft: DocumentDraft {
                title: "Active".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();
    support::write_ticket_source(
        &fixture.root,
        ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md",
    );
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    let active = fixture
        .root
        .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV");
    fs::remove_dir_all(&active).unwrap();
    fixture
        .repository
        .find_worktree("01ARZ3NDEKTSV4RRFFQ69G5FAV")
        .unwrap()
        .prune(None)
        .unwrap();

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap()
    else {
        panic!("expected stable absence")
    };
    assert_eq!(snapshot.contexts.len(), 1);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(snapshot.items[0].kind, AuthoringKind::Ticket);
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM contexts", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM discovered_comments", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM problems WHERE context_id IS NOT NULL",
                        [],
                        |row| row.get::<_, i64>(0)
                    )
                    .unwrap(),
                0
            );
        })
        .unwrap();
}

#[test]
fn refresh_active_worktree_race_keeps_prior_active_rows_and_updates_root() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    enabled
        .service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/active.md"),
            draft: DocumentDraft {
                title: "Active".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();
    let ticket = fixture
        .root
        .join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
    fs::create_dir_all(ticket.parent().unwrap()).unwrap();
    fs::write(&ticket, support::ticket_source()).unwrap();
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    fs::write(
        &ticket,
        support::ticket_source().replace("Fixture ticket", "Updated ticket"),
    )
    .unwrap();
    let active = fixture
        .root
        .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV/docs/active.md");
    let changed_active = fs::read_to_string(&active)
        .unwrap()
        .replace("title: Active", "title: Changed");
    enabled
        .service
        .set_observation_hook_for_testing(move || fs::write(active, changed_active).unwrap());
    let operation_id = support::operation_id();

    let RefreshOutcome::RetryRequired {
        context: Some(context),
        ..
    } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap()
    else {
        panic!("expected active retry")
    };
    assert_eq!(
        context,
        fixture
            .root
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV")
    );
    let snapshot = enabled.service.repository_snapshot(&fixture.root).unwrap();
    assert!(
        snapshot
            .items
            .iter()
            .any(|item| item.title == "Updated ticket")
    );
    assert!(snapshot.items.iter().any(|item| item.title == "Active"));

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap()
    else {
        panic!("expected stable retry")
    };
    assert!(snapshot.items.iter().any(|item| item.title == "Changed"));
    assert!(
        snapshot
            .items
            .iter()
            .any(|item| item.title == "Updated ticket")
    );
}

#[test]
fn refresh_retry_replaces_a_previously_persisted_context_when_its_observation_changes() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let root_ticket = fixture
        .root
        .join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
    fs::create_dir_all(root_ticket.parent().unwrap()).unwrap();
    fs::write(&root_ticket, support::ticket_source()).unwrap();
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    enabled
        .service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/active.md"),
            draft: DocumentDraft {
                title: "Active".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    let active = fixture
        .root
        .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV/docs/active.md");
    let active_changed = fs::read_to_string(&active)
        .unwrap()
        .replace("title: Active", "title: Raced");
    enabled
        .service
        .set_observation_hook_for_testing(move || fs::write(active, active_changed).unwrap());
    let operation_id = support::operation_id();
    assert!(matches!(
        enabled
            .service
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap(),
        RefreshOutcome::RetryRequired { .. }
    ));
    fs::write(
        &root_ticket,
        support::ticket_source().replace("Fixture ticket", "Retry ticket"),
    )
    .unwrap();

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap()
    else {
        panic!("expected retry")
    };
    assert!(
        snapshot
            .items
            .iter()
            .any(|item| item.title == "Retry ticket")
    );
    assert!(!snapshot.refresh_required);
}

#[test]
fn refresh_retry_replaces_a_persisted_root_when_configuration_problem_changes() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    enabled
        .service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: fixture.root.clone(),
                kind: AuthoringKind::Document,
                item_id: support::document_id(),
                intent: ContextIntent::Create,
                operation_id: support::operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from("docs/active.md"),
            draft: DocumentDraft {
                title: "Active".to_owned(),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: manyhands::repository::ExpectedPathObservation::Missing,
        })
        .unwrap();
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    let active = fixture
        .root
        .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV/docs/active.md");
    let changed = fs::read_to_string(&active)
        .unwrap()
        .replace("title: Active", "title: Raced");
    enabled
        .service
        .set_observation_hook_for_testing(move || fs::write(active, changed).unwrap());
    let operation_id = support::operation_id();
    assert!(matches!(
        enabled
            .service
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap(),
        RefreshOutcome::RetryRequired { .. }
    ));
    fs::write(fixture.root.join(".manyhands/config.toml"), "not = [valid").unwrap();

    let RefreshOutcome::Refreshed { snapshot } = enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap()
    else {
        panic!("expected retry")
    };
    assert!(matches!(
        snapshot.configuration,
        SnapshotConfiguration::Invalid { .. }
    ));
    assert!(
        snapshot
            .problems
            .iter()
            .any(|problem| problem.code == "malformed-configuration")
    );
    assert!(!snapshot.refresh_required);
}

#[test]
fn refresh_lifecycle_failure_resumes_the_same_operation_to_completion() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    let failing =
        support::FailOnce::at(FailurePoint::AfterContextObservation).open_service(data.path());
    let operation_id = support::operation_id();
    assert_eq!(
        failing
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert_eq!(
        refresh_operation_states(&service),
        vec![("failed".to_owned(), None)]
    );
    service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap();
    assert_eq!(
        refresh_operation_states(&service),
        vec![("completed".to_owned(), None)]
    );
}

#[test]
fn refresh_lifecycle_marks_transaction_failure_then_resumes_without_duplicate_record() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    let operation_id = support::operation_id();
    assert_eq!(
        failing
            .refresh_repository(refresh_request!(&fixture.root, operation_id))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert_eq!(
        refresh_operation_states(&service),
        vec![("failed".to_owned(), None)]
    );
    service
        .refresh_repository(refresh_request!(&fixture.root, operation_id))
        .unwrap();
    assert_eq!(
        refresh_operation_states(&service),
        vec![("completed".to_owned(), None)]
    );
}

#[test]
fn refresh_records_only_a_valid_configuration_committed_blob_oid() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    let expected = fixture
        .repository
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(Path::new(".manyhands/config.toml"))
        .unwrap()
        .id()
        .to_string();
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            let oid: Option<String> = connection
                .query_row("SELECT config_blob_oid FROM repositories", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(oid.as_deref(), Some(expected.as_str()));
        })
        .unwrap();
    fs::remove_file(fixture.root.join(".manyhands/config.toml")).unwrap();
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            let oid: Option<String> = connection
                .query_row("SELECT config_blob_oid FROM repositories", [], |row| {
                    row.get(0)
                })
                .unwrap();
            let state: String = connection
                .query_row("SELECT state FROM configuration_observations", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(oid, None);
            assert_eq!(state, "missing");
        })
        .unwrap();
    fs::create_dir_all(fixture.root.join(".manyhands")).unwrap();
    fs::write(fixture.root.join(".manyhands/config.toml"), "not = [valid").unwrap();
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            let oid: Option<String> = connection
                .query_row("SELECT config_blob_oid FROM repositories", [], |row| {
                    row.get(0)
                })
                .unwrap();
            let state: String = connection
                .query_row("SELECT state FROM configuration_observations", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(oid, None);
            assert_eq!(state, "invalid");
        })
        .unwrap();
}

#[test]
fn snapshot_rejects_corrupt_cached_metadata() {
    for corruption in [
        "item-id",
        "path",
        "oid",
        "timestamp",
        "enum",
        "configuration",
        "configuration-branch",
        "title",
        "document-ticket-data",
        "dot-path",
        "backslash-path",
        "active-context-without-item",
        "primary-external-path",
        "active-external-path",
        "active-branch-kind-mismatch",
        "active-branch-id-mismatch",
        "active-path-id-mismatch",
        "global-duplicate-id",
    ] {
        let fixture = support::born_repository();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .with_registry_connection_for_testing(|connection| {
                connection.execute(
                    "INSERT INTO repositories (root_path, enabled_at, accessibility, config_blob_oid, refresh_required)
                     VALUES (?1, 1, 'accessible', NULL, 0)",
                    [fixture.root.to_str().unwrap()],
                )
                .unwrap();
                let repository_id = connection.last_insert_rowid();
                connection.execute(
                    "INSERT INTO contexts (repository_id, kind, branch, worktree_path, head_oid)
                     VALUES (?1, 'primary', 'main', ?2, ?3)",
                    params![repository_id, fixture.root.to_str().unwrap(), "0123456789012345678901234567890123456789"],
                )
                .unwrap();
                let context_id = connection.last_insert_rowid();
                connection.execute(
                    "INSERT INTO discovered_items (context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
                     VALUES (?1, ?2, 'document', 'docs/item.md', 'Item', 1, 'git')",
                    params![context_id, support::document_id().to_string()],
                )
                .unwrap();
                let item_row_id = connection.last_insert_rowid();
                match corruption {
                    "item-id" => connection.execute("UPDATE discovered_items SET item_id = 'bad'", []).unwrap(),
                    "path" => connection.execute("UPDATE discovered_items SET canonical_path = '../outside.md'", []).unwrap(),
                    "oid" => connection.execute("UPDATE contexts SET head_oid = 'not-an-oid'", []).unwrap(),
                    "timestamp" => connection.execute("UPDATE discovered_items SET activity_at = ?1", [i64::MAX]).unwrap(),
                    "enum" => connection.execute("UPDATE discovered_items SET kind = 'unknown'", []).unwrap(),
                    "configuration" => connection.execute(
                        "INSERT INTO configuration_observations (repository_id, state, invalid_code, guidance)
                         VALUES (?1, 'invalid', 'unknown', 'repair')",
                        [repository_id],
                    ).unwrap(),
                    "configuration-branch" => connection.execute(
                        "INSERT INTO configuration_observations (repository_id, state, primary_branch)
                         VALUES (?1, 'valid', 'HEAD')",
                        [repository_id],
                    ).unwrap(),
                    "title" => connection.execute("UPDATE discovered_items SET title = ''", []).unwrap(),
                    "document-ticket-data" => connection.execute("UPDATE discovered_items SET ticket_type = 'bug'", []).unwrap(),
                    "dot-path" => connection.execute("UPDATE discovered_items SET canonical_path = 'docs/./item.md'", []).unwrap(),
                    "backslash-path" => connection.execute("UPDATE discovered_items SET canonical_path = 'docs\\item.md'", []).unwrap(),
                    "active-context-without-item" => connection.execute("UPDATE contexts SET kind = 'active'", []).unwrap(),
                    "primary-external-path" => connection.execute(
                        "UPDATE contexts SET worktree_path = '/outside/registered-root'", [],
                    ).unwrap(),
                    "active-external-path" => connection.execute(
                        "UPDATE contexts SET kind = 'active', branch = 'manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV', item_id = ?1, worktree_path = '/outside/registered-root'",
                        [support::document_id().to_string()],
                    ).unwrap(),
                    "active-branch-kind-mismatch" => connection.execute(
                        "UPDATE contexts SET kind = 'active', branch = 'manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV', item_id = ?1, worktree_path = ?2",
                        params![support::document_id().to_string(), fixture.root.join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV").to_str().unwrap()],
                    ).unwrap(),
                    "active-branch-id-mismatch" => connection.execute(
                        "UPDATE contexts SET kind = 'active', branch = 'manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAW', item_id = ?1, worktree_path = ?2",
                        params![support::document_id().to_string(), fixture.root.join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV").to_str().unwrap()],
                    ).unwrap(),
                    "active-path-id-mismatch" => connection.execute(
                        "UPDATE contexts SET kind = 'active', branch = 'manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV', item_id = ?1, worktree_path = ?2",
                        params![support::document_id().to_string(), fixture.root.join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAW").to_str().unwrap()],
                    ).unwrap(),
                    "global-duplicate-id" => connection.execute(
                        "INSERT INTO discovered_comments (item_id, comment_id, canonical_path, created_at)
                         VALUES (?1, ?2, 'comments/duplicate.md', 1)",
                        params![item_row_id, support::document_id().to_string()],
                    ).unwrap(),
                    _ => unreachable!(),
                };
            })
            .unwrap();

        let error = service.repository_snapshot(&fixture.root).unwrap_err();
        assert_eq!(
            error.kind,
            RepositoryErrorKind::IndexUnavailable,
            "{corruption}"
        );
        assert_eq!(
            error.operation,
            RepositoryOperation::RepositorySnapshot,
            "{corruption}"
        );
    }
}

#[test]
fn snapshot_maps_invalid_configuration_observation() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let root = PathBuf::from("/stored/root/does-not-need-to-exist");
    service
        .with_registry_connection_for_testing(|connection| {
            connection.execute(
                "INSERT INTO repositories (root_path, enabled_at, accessibility, config_blob_oid, refresh_required)
                 VALUES (?1, 1, 'accessible', NULL, 0)",
                [root.to_str().unwrap()],
            )
            .unwrap();
            let repository_id = connection.last_insert_rowid();
            connection.execute(
                "INSERT INTO configuration_observations (repository_id, state, invalid_code, guidance)
                 VALUES (?1, 'invalid', 'malformed-configuration', 'repair configuration')",
                [repository_id],
            )
            .unwrap();
        })
        .unwrap();

    let snapshot = service.repository_snapshot(&root).unwrap();

    assert_eq!(
        snapshot.configuration,
        SnapshotConfiguration::Invalid {
            code: canonical::ValidationCode::MalformedConfiguration,
            guidance: "repair configuration".to_owned(),
        }
    );
}

#[test]
fn snapshot_resolves_equivalent_and_symlinked_root_paths() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    let alias_parent = tempfile::tempdir().unwrap();
    let alias = alias_parent.path().join("repository-alias");
    std::os::unix::fs::symlink(&fixture.root, &alias).unwrap();

    assert!(
        enabled
            .service
            .repository_snapshot(&fixture.root.join("."))
            .is_ok()
    );
    assert!(enabled.service.repository_snapshot(&alias).is_ok());
}

#[test]
fn rebuild_keeps_prior_snapshot_stale_when_observation_changes() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let document = support::write_document_source(&fixture.root, "docs/visible.md");
    enabled
        .service
        .refresh_repository(refresh_request!(&fixture.root))
        .unwrap();
    enabled.service.set_observation_hook_for_testing({
        let document = document.clone();
        move || {
            fs::write(
                document,
                support::document_source().replace("Fixture document", "Changed during rebuild"),
            )
            .unwrap()
        }
    });

    let snapshot = enabled
        .service
        .rebuild_repository(rebuild_request!(&fixture.root))
        .unwrap();

    assert!(snapshot.refresh_required);
    assert_eq!(snapshot.items[0].title, "Fixture document");
}

fn corrupt_diagnostic_exists(data: &Path) -> bool {
    corrupt_diagnostic_count(data) != 0
}

fn corrupt_diagnostic_count(data: &Path) -> usize {
    fs::read_dir(data)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("manyhands.sqlite3.corrupt-")
        })
        .count()
}

#[test]
fn rebuild_corrupt_cache_restores_only_the_explicit_root() {
    let first = support::born_repository();
    let second = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let initial = RepositoryService::open_at(data.path()).unwrap();
    initial
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: first.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    initial
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: second.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    support::write_document_source(&first.root, "docs/visible.md");
    let head = first.repository.head().unwrap().target();
    let index = support::index_bytes(&first.repository).unwrap();
    fs::remove_file(data.path().join("manyhands.sqlite3")).unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();

    let service = RepositoryService::open_at(data.path()).unwrap();
    for error in [
        service.inspect(&first.root).unwrap_err(),
        service
            .refresh_repository(refresh_request!(&first.root))
            .unwrap_err(),
        service.repository_snapshot(&first.root).unwrap_err(),
        service
            .remove_registration(manyhands::repository::RemoveRegistrationRequest {
                root: first.root.clone(),
                operation_id: support::operation_id(),
            })
            .unwrap_err(),
    ] {
        assert_eq!(error.kind, RepositoryErrorKind::IndexUnavailable);
    }

    let snapshot = service
        .rebuild_repository(rebuild_request!(&first.root.join(".")))
        .unwrap();

    assert_eq!(snapshot.root, first.root.canonicalize().unwrap());
    assert_eq!(snapshot.items.len(), 1);
    assert!(service.repository_snapshot(&first.root).is_ok());
    assert_eq!(
        service.repository_snapshot(&second.root).unwrap_err().kind,
        RepositoryErrorKind::RepositoryNotRegistered
    );
    assert!(corrupt_diagnostic_exists(data.path()));
    assert_eq!(first.repository.head().unwrap().target(), head);
    assert_eq!(support::index_bytes(&first.repository).unwrap(), index);
    assert_eq!(
        fs::read_to_string(first.root.join("docs/visible.md")).unwrap(),
        support::document_source()
    );
}

#[test]
fn rebuild_healthy_cache_replaces_only_requested_root_derived_rows() {
    let first = support::born_repository();
    let second = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    for fixture in [&first, &second] {
        service
            .enable(manyhands::repository::EnableRepositoryRequest {
                root: fixture.root.clone(),
                primary_branch: "main".to_owned(),
                identity: None,
                operation_id: support::operation_id(),
            })
            .unwrap();
    }
    support::write_document_source(&first.root, "docs/first.md");
    support::write_document_source(&second.root, "docs/second.md");
    service
        .refresh_repository(refresh_request!(&first.root))
        .unwrap();
    service
        .refresh_repository(refresh_request!(&second.root))
        .unwrap();
    fs::remove_file(second.root.join("docs/second.md")).unwrap();

    let snapshot = service
        .rebuild_repository(rebuild_request!(&first.root))
        .unwrap();

    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        service
            .repository_snapshot(&second.root)
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(!corrupt_diagnostic_exists(data.path()));
}

#[test]
fn corrupt_rebuild_failure_preserves_diagnostics_for_retry() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let failing = support::FailOnce::at(FailurePoint::BeforeCorruptCacheReplacement)
        .open_service(data.path());
    let operation_id = support::operation_id();

    let error = failing
        .rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    assert_eq!(error.operation, RepositoryOperation::RebuildRepository);
    assert_eq!(
        fs::read(data.path().join("manyhands.sqlite3")).unwrap(),
        b"not sqlite"
    );
    assert!(!corrupt_diagnostic_exists(data.path()));
    assert_eq!(
        failing.repository_snapshot(&fixture.root).unwrap_err().kind,
        RepositoryErrorKind::IndexUnavailable
    );

    let snapshot = failing
        .rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        .unwrap();
    assert_eq!(snapshot.root, fixture.root.canonicalize().unwrap());
    assert!(corrupt_diagnostic_exists(data.path()));
}

fn rebuild_operation_states(service: &RepositoryService) -> Vec<(String, i64)> {
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .prepare(
                    "SELECT state, persisted_context_count FROM operation_records
                      WHERE action = 'rebuild' ORDER BY id",
                )
                .unwrap()
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        })
        .unwrap()
}

fn rebuild_operation_states_at(data: &Path) -> Vec<(String, i64)> {
    Connection::open(data.join("manyhands.sqlite3"))
        .unwrap()
        .prepare(
            "SELECT state, persisted_context_count FROM operation_records
              WHERE action = 'rebuild' ORDER BY id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn rebuild_records_one_durable_operation_and_resumes_after_a_persistence_error() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute_batch(
                    "CREATE TRIGGER fail_rebuild_context_insert BEFORE INSERT ON contexts
                     BEGIN SELECT RAISE(ABORT, 'injected rebuild persistence failure'); END;",
                )
                .unwrap();
        })
        .unwrap();
    let operation_id = support::operation_id();

    let error = service
        .rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::Sqlite);
    assert_eq!(
        rebuild_operation_states_at(data.path()),
        vec![("error".to_owned(), 0)]
    );
    Connection::open(data.path().join("manyhands.sqlite3"))
        .unwrap()
        .execute_batch("DROP TRIGGER fail_rebuild_context_insert;")
        .unwrap();

    service
        .rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        .unwrap();

    assert_eq!(
        rebuild_operation_states(&service),
        vec![("completed".to_owned(), 1)]
    );
}

#[test]
fn rebuild_error_does_not_overwrite_a_completed_same_id_retry() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let failing =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    let retry = RepositoryService::open_at(data.path()).unwrap();
    let operation_id = support::operation_id();
    let (paused_send, paused_receive) = mpsc::sync_channel(0);
    let (resume_send, resume_receive) = mpsc::sync_channel(0);
    failing.set_rebuild_error_hook_for_testing(move || {
        paused_send.send(()).unwrap();
        resume_receive.recv().unwrap();
    });

    std::thread::scope(|scope| {
        let original = scope
            .spawn(|| failing.rebuild_repository(rebuild_request!(&fixture.root, operation_id)));
        paused_receive.recv().unwrap();
        let retried = retry.rebuild_repository(rebuild_request!(&fixture.root, operation_id));
        resume_send.send(()).unwrap();
        let original = original.join().unwrap();
        retried.unwrap();
        assert_eq!(
            original.unwrap_err().kind,
            RepositoryErrorKind::InjectedFailure
        );
    });

    assert!(retry.recovery_inspection(&fixture.root).unwrap().is_empty());
}

#[test]
fn corrupt_rebuild_resumes_without_replacing_diagnostics_twice() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service =
        support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    let operation_id = support::operation_id();

    let error = service
        .rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    assert_eq!(
        rebuild_operation_states_at(data.path()),
        vec![("error".to_owned(), 0)]
    );
    let diagnostics = corrupt_diagnostic_count(data.path());
    assert_eq!(
        service.repository_snapshot(&fixture.root).unwrap_err().kind,
        RepositoryErrorKind::RepositoryNotRegistered
    );

    service
        .rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        .unwrap();

    assert_eq!(corrupt_diagnostic_count(data.path()), diagnostics);
    assert_eq!(
        rebuild_operation_states_at(data.path()),
        vec![("completed".to_owned(), 1)]
    );
}

#[test]
fn root_scoped_incomplete_rebuild_does_not_block_other_root_discovery() {
    let first = support::born_repository();
    let second = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    for fixture in [&first, &second] {
        service
            .enable(manyhands::repository::EnableRepositoryRequest {
                root: fixture.root.clone(),
                primary_branch: "main".to_owned(),
                identity: None,
                operation_id: support::operation_id(),
            })
            .unwrap();
    }
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute_batch(&format!(
                    "CREATE TRIGGER fail_first_rebuild BEFORE INSERT ON contexts
                     WHEN NEW.worktree_path = '{}'
                     BEGIN SELECT RAISE(ABORT, 'first rebuild failure'); END;",
                    first.root.to_str().unwrap()
                ))
                .unwrap();
        })
        .unwrap();
    let operation_id = support::operation_id();

    assert_eq!(
        service
            .rebuild_repository(rebuild_request!(&first.root, operation_id))
            .unwrap_err()
            .kind,
        RepositoryErrorKind::Sqlite
    );
    Connection::open(data.path().join("manyhands.sqlite3"))
        .unwrap()
        .execute_batch("DROP TRIGGER fail_first_rebuild;")
        .unwrap();

    assert!(service.repository_snapshot(&second.root).is_ok());
    assert!(matches!(
        service
            .refresh_repository(refresh_request!(&second.root))
            .unwrap(),
        RefreshOutcome::Refreshed { .. }
    ));
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(restarted.repository_snapshot(&second.root).is_ok());

    restarted
        .rebuild_repository(rebuild_request!(&first.root, operation_id))
        .unwrap();

    assert!(restarted.inspect(&second.root).is_ok());
}

#[test]
fn refresh_scan_releases_repository_lease_before_observation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let root = fixture.root.clone();
    let data = enabled.data_directory.path().to_owned();
    enabled.service.set_observation_hook_for_testing(move || {
        support::hold_lease_in_child_for_test(
            &root,
            &data,
            LeaseKind::Repository,
            "cache_lease_child",
        )
        .release();
    });

    assert!(
        enabled
            .service
            .refresh_repository(refresh_request!(&fixture.root))
            .is_ok()
    );
}

#[test]
fn registration_git_reads_release_cache_guard_for_exclusive_replacement() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    enabled.service.set_registration_git_hook_for_testing({
        let root = fixture.root.clone();
        let data = enabled.data_directory.path().to_owned();
        move || {
            support::hold_lease_in_child_for_test(
                &root,
                &data,
                LeaseKind::CacheWrite,
                "cache_lease_child",
            )
            .release();
        }
    });

    assert!(
        enabled
            .service
            .enable(support::enable_request(&fixture.root))
            .is_ok()
    );
}

#[test]
fn rebuild_clears_config_blob_oid_for_a_valid_uncommitted_config_edit() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            operation_id: support::operation_id(),
        })
        .unwrap();
    fs::write(
        fixture.root.join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\n# live edit\n",
    )
    .unwrap();

    let snapshot = service
        .rebuild_repository(rebuild_request!(&fixture.root))
        .unwrap();
    let oid: Option<String> = service
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row("SELECT config_blob_oid FROM repositories", [], |row| {
                    row.get(0)
                })
                .unwrap()
        })
        .unwrap();

    assert_eq!(oid, None);
    assert!(matches!(
        snapshot.configuration,
        SnapshotConfiguration::Valid { .. }
    ));
}

#[test]
fn concurrent_corrupt_rebuilds_replace_the_cache_once() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let (observed_send, observed_receive) = mpsc::sync_channel(0);
    let (release_send, release_receive) = mpsc::sync_channel(0);
    service.set_observation_hook_for_testing(move || {
        observed_send.send(()).unwrap();
        release_receive.recv().unwrap();
    });
    let operation_id = support::operation_id();

    std::thread::scope(|scope| {
        let first = scope
            .spawn(|| service.rebuild_repository(rebuild_request!(&fixture.root, operation_id)));
        observed_receive.recv().unwrap();
        let second = scope
            .spawn(|| service.rebuild_repository(rebuild_request!(&fixture.root, operation_id)));
        release_send.send(()).unwrap();
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });

    assert_eq!(corrupt_diagnostic_count(data.path()), 1);
    assert!(service.repository_snapshot(&fixture.root).is_ok());
}

#[test]
fn services_sharing_a_corrupt_cache_replace_it_once() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let first_service = RepositoryService::open_at(data.path()).unwrap();
    let second_service = RepositoryService::open_at(data.path()).unwrap();
    let (observed_send, observed_receive) = mpsc::sync_channel(0);
    let (release_send, release_receive) = mpsc::sync_channel(0);
    first_service.set_observation_hook_for_testing(move || {
        observed_send.send(()).unwrap();
        release_receive.recv().unwrap();
    });
    let operation_id = support::operation_id();

    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            first_service.rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        });
        observed_receive.recv().unwrap();
        let second = scope.spawn(|| {
            second_service.rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        });
        release_send.send(()).unwrap();
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });

    assert_eq!(corrupt_diagnostic_count(data.path()), 1);
    assert!(first_service.repository_snapshot(&fixture.root).is_ok());
    assert!(second_service.repository_snapshot(&fixture.root).is_ok());
}

#[test]
fn corrupt_cache_replacement_rechecks_after_the_exclusive_guard() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let first_service = RepositoryService::open_at(data.path()).unwrap();
    let second_service = RepositoryService::open_at(data.path()).unwrap();
    let decisions = Arc::new(std::sync::Barrier::new(2));
    for service in [&first_service, &second_service] {
        service.set_corrupt_cache_decision_hook_for_testing({
            let decisions = Arc::clone(&decisions);
            move || {
                decisions.wait();
            }
        });
    }
    let (entered_send, entered_receive) = mpsc::sync_channel(0);
    let (release_send, release_receive) = mpsc::sync_channel(0);
    let entered = Arc::new(Mutex::new(false));
    let release_receive = Arc::new(Mutex::new(release_receive));
    for service in [&first_service, &second_service] {
        service.set_corrupt_cache_critical_hook_for_testing({
            let entered = Arc::clone(&entered);
            let entered_send = entered_send.clone();
            let release_receive = Arc::clone(&release_receive);
            move || {
                let mut entered = entered.lock().unwrap();
                if !*entered {
                    *entered = true;
                    entered_send.send(()).unwrap();
                    release_receive.lock().unwrap().recv().unwrap();
                }
            }
        });
    }
    let operation_id = support::operation_id();

    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            first_service.rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        });
        let second = scope.spawn(|| {
            second_service.rebuild_repository(rebuild_request!(&fixture.root, operation_id))
        });
        entered_receive.recv().unwrap();
        release_send.send(()).unwrap();
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });

    assert_eq!(corrupt_diagnostic_count(data.path()), 1);
    assert!(first_service.repository_snapshot(&fixture.root).is_ok());
    assert!(second_service.repository_snapshot(&fixture.root).is_ok());
}

#[test]
fn corrupt_rebuild_releases_cache_replacement_guard_before_observation() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let (observed_send, observed_receive) = mpsc::sync_channel(0);
    let (release_send, release_receive) = mpsc::sync_channel(0);
    service.set_observation_hook_for_testing(move || {
        observed_send.send(()).unwrap();
        release_receive.recv().unwrap();
    });

    std::thread::scope(|scope| {
        let rebuild = scope.spawn(|| service.rebuild_repository(rebuild_request!(&fixture.root)));
        observed_receive.recv().unwrap();
        let reader = support::hold_lease_in_child_for_test(
            &fixture.root,
            data.path(),
            LeaseKind::CacheRead,
            "cache_lease_child",
        );
        reader.release();
        release_send.send(()).unwrap();
        rebuild.join().unwrap().unwrap();
    });
}

#[test]
fn rebuild_persists_observed_configuration_blob_or_null_for_every_state() {
    for corrupt in [false, true] {
        for state in ["valid", "missing", "malformed"] {
            let fixture = support::born_repository();
            let data = tempfile::tempdir().unwrap();
            let service = RepositoryService::open_at(data.path()).unwrap();
            service
                .enable(manyhands::repository::EnableRepositoryRequest {
                    root: fixture.root.clone(),
                    primary_branch: "main".to_owned(),
                    identity: None,
                    operation_id: support::operation_id(),
                })
                .unwrap();
            let expected = fixture
                .repository
                .head()
                .unwrap()
                .peel_to_tree()
                .unwrap()
                .get_path(Path::new(".manyhands/config.toml"))
                .unwrap()
                .id()
                .to_string();
            match state {
                "valid" => {}
                "missing" => fs::remove_file(fixture.root.join(".manyhands/config.toml")).unwrap(),
                "malformed" => {
                    fs::write(fixture.root.join(".manyhands/config.toml"), "not = [valid").unwrap()
                }
                _ => unreachable!(),
            }
            if corrupt {
                fs::remove_file(data.path().join("manyhands.sqlite3")).unwrap();
                fs::write(data.path().join("manyhands.sqlite3"), b"not sqlite").unwrap();
            }
            let service = RepositoryService::open_at(data.path()).unwrap();

            let snapshot = service
                .rebuild_repository(rebuild_request!(&fixture.root))
                .unwrap();
            let oid: Option<String> = service
                .with_registry_connection_for_testing(|connection| {
                    connection
                        .query_row("SELECT config_blob_oid FROM repositories", [], |row| {
                            row.get(0)
                        })
                        .unwrap()
                })
                .unwrap();

            match state {
                "valid" => {
                    assert_eq!(oid.as_deref(), Some(expected.as_str()), "corrupt={corrupt}");
                    assert!(matches!(
                        snapshot.configuration,
                        SnapshotConfiguration::Valid { .. }
                    ));
                }
                "missing" => {
                    assert_eq!(oid, None, "corrupt={corrupt}");
                    assert_eq!(snapshot.configuration, SnapshotConfiguration::Missing);
                }
                "malformed" => {
                    assert_eq!(oid, None, "corrupt={corrupt}");
                    assert!(matches!(
                        snapshot.configuration,
                        SnapshotConfiguration::Invalid { .. }
                    ));
                }
                _ => unreachable!(),
            }
        }
    }
}

const CYCLE_02_REPOSITORIES_SCHEMA: &str = "
    CREATE TABLE repositories (
        id INTEGER PRIMARY KEY,
        root_path TEXT NOT NULL UNIQUE,
        enabled_at INTEGER NOT NULL,
        accessibility TEXT NOT NULL,
        config_blob_oid TEXT NOT NULL,
        refresh_required INTEGER NOT NULL CHECK (refresh_required IN (0, 1))
    );
";

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn migration_retains_cycle_02_registration_and_adds_discovery_tables() {
    let data = tempfile::tempdir().unwrap();
    let connection = Connection::open(data.path().join("manyhands.sqlite3")).unwrap();
    connection
        .execute_batch(CYCLE_02_REPOSITORIES_SCHEMA)
        .unwrap();
    connection
        .execute(
            "INSERT INTO repositories (
                root_path, enabled_at, accessibility, config_blob_oid, refresh_required
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["/fixture", 42_i64, "accessible", "config-oid", 1_i64],
        )
        .unwrap();
    drop(connection);

    let service = RepositoryService::open_at(data.path()).unwrap();

    service
        .with_registry_connection_for_testing(|connection| {
            let registration: (String, i64, String, String, i64) = connection
                .query_row(
                    "SELECT root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                     FROM repositories WHERE root_path = '/fixture'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .unwrap();
            assert_eq!(registration, ("/fixture".to_owned(), 42, "accessible".to_owned(), "config-oid".to_owned(), 1));
            assert_eq!(
                connection
                    .query_row(
                        "SELECT \"notnull\" FROM pragma_table_info('repositories') WHERE name = 'config_blob_oid'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                0
            );
            for table in [
                "contexts",
                "discovered_items",
                "discovered_comments",
                "problems",
                "operation_records",
                "configuration_observations",
            ] {
                assert!(table_exists(connection, table), "missing {table}");
            }
        })
        .unwrap();
}

#[test]
fn migration_is_idempotent() {
    let data = tempfile::tempdir().unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join("manyhands.sqlite3")).unwrap();
    let schema_before: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'repositories'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);

    RepositoryService::open_at(data.path()).unwrap();

    let connection = Connection::open(data.path().join("manyhands.sqlite3")).unwrap();
    let schema_after: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'repositories'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(schema_after, schema_before);
}

#[test]
fn remove_registration_cascades_only_its_derived_rows() {
    let data = tempfile::tempdir().unwrap();
    let first = support::born_repository();
    let second = support::born_repository();
    let service = RepositoryService::open_at(data.path()).unwrap();

    service
        .with_registry_connection_for_testing(|connection| {
            for root in [&first.root, &second.root] {
                connection
                    .execute(
                        "INSERT INTO repositories (
                            root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                        ) VALUES (?1, 1, 'accessible', 'oid', 0)",
                        [root.to_str().unwrap()],
                    )
                    .unwrap();
            }
            for repository_id in [1_i64, 2] {
                connection.execute("INSERT INTO contexts (repository_id, kind, worktree_path) VALUES (?1, 'primary', ?2)", params![repository_id, format!("/worktree/{repository_id}")]).unwrap();
                connection.execute("INSERT INTO discovered_items (context_id, item_id, kind, canonical_path, title, activity_at, activity_source) VALUES (?1, ?2, 'document', 'docs/item.md', 'Item', 1, 'git')", params![repository_id, format!("item-{repository_id}")]).unwrap();
                connection.execute("INSERT INTO discovered_comments (item_id, comment_id, canonical_path, created_at) VALUES (?1, ?2, 'comments/item.md', 1)", params![repository_id, format!("comment-{repository_id}")]).unwrap();
                connection.execute("INSERT INTO problems (repository_id, code, guidance, observed_at) VALUES (?1, 'problem', 'repair', 1)", [repository_id]).unwrap();
                connection.execute("INSERT INTO operation_records (repository_id, root_path, action, state, observed_at) VALUES (?1, ?2, 'refresh', 'completed', 1)", params![repository_id, if repository_id == 1 { first.root.to_str().unwrap() } else { second.root.to_str().unwrap() }]).unwrap();
            }
        })
        .unwrap();

    assert_eq!(
        service
            .remove_registration(manyhands::repository::RemoveRegistrationRequest {
                root: first.root.clone(),
                operation_id: support::operation_id(),
            })
            .unwrap(),
        manyhands::repository::RemoveRegistrationOutcome::Removed
    );

    service
        .with_registry_connection_for_testing(|connection| {
            for table in [
                "contexts",
                "discovered_items",
                "discovered_comments",
                "problems",
                "operation_records",
            ] {
                let count: i64 = connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .unwrap();
                assert_eq!(count, 1, "unexpected surviving rows in {table}");
            }
        })
        .unwrap();
}
