use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use manyhands::{
    canonical,
    repository::{
        AuthoringKind, DiscoveredCommentThread, DiscoveredContext, DiscoveredItem,
        DiscoveryActivitySource, DiscoveryContextKind, DiscoveryProblem, RefreshOutcome,
        RepositoryErrorKind, RepositoryOperation, RepositoryService, RepositorySnapshot,
        SnapshotConfiguration,
    },
};
use rusqlite::{Connection, params};
use time::OffsetDateTime;

mod support;

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
    let active = root.join(".manyhands/worktrees/active");
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
                 VALUES (?1, 'active', 'manyhands/document/active', ?2, ?3, ?4)",
                params![repository_id, active.to_str().unwrap(), document_id.to_string(), "0123456789012345678901234567890123456789"],
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
                params![active_context_id, ticket_id.to_string()],
            ).unwrap();
            connection.execute(
                "INSERT INTO discovered_items (context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
                 VALUES (?1, ?2, 'document', 'docs/a.md', 'Alpha document', 10, 'git')",
                params![primary_context_id, document_id.to_string()],
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
    assert_eq!(snapshot.items[0].id, document_id);
    assert_eq!(snapshot.items[0].kind, AuthoringKind::Document);
    assert_eq!(snapshot.items[1].id, ticket_id);
    assert_eq!(snapshot.items[1].kind, AuthoringKind::Ticket);
    assert_eq!(snapshot.items[1].ticket_type.as_deref(), Some("bug"));
    assert_eq!(snapshot.items[1].status.as_deref(), Some("open"));
    assert_eq!(snapshot.items[1].project.as_deref(), Some("Core"));
    assert_eq!(snapshot.items[1].team.as_deref(), Some("Platform"));
    assert_eq!(
        snapshot.items[1].closed_at,
        Some(OffsetDateTime::from_unix_timestamp(30).unwrap())
    );
    assert_eq!(
        snapshot.items[1].activity_source,
        DiscoveryActivitySource::UncommittedFilesystem
    );
    assert_eq!(snapshot.items[0].comments.len(), 2);
    assert_eq!(snapshot.items[0].comments[0].id, root_comment_id);
    assert_eq!(snapshot.items[0].comments[1].id, first_root_comment_id);
    assert_eq!(snapshot.items[0].comments[0].replies[0].id, reply_id);
    assert_eq!(snapshot.items[0].comments[0].replies[1].id, first_reply_id);
    assert_eq!(
        snapshot.items[0].comments[0].replies[0].replies[0].id,
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
    let before = available_state(&fixture, data.path());

    let snapshot = service.repository_snapshot(&fixture.root).unwrap();

    assert_eq!(snapshot.root, fixture.root);
    assert_eq!(snapshot.configuration, SnapshotConfiguration::Missing);
    assert_eq!(available_state(&fixture, data.path()), before);
}

#[test]
fn discovery_task_1_placeholders_remain_not_registered() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    for (operation, error) in [
        (
            RepositoryOperation::RefreshRepository,
            service.refresh_repository(&fixture.root).unwrap_err(),
        ),
        (
            RepositoryOperation::RebuildRepository,
            service.rebuild_repository(&fixture.root).unwrap_err(),
        ),
    ] {
        assert_eq!(error.kind, RepositoryErrorKind::RepositoryNotRegistered);
        assert_eq!(error.operation, operation);
    }
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
                "index_operations",
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
                connection.execute("INSERT INTO index_operations (repository_id, operation, observed_at) VALUES (?1, 'refresh', 1)", [repository_id]).unwrap();
            }
        })
        .unwrap();

    assert_eq!(
        service.remove_registration(&first.root).unwrap(),
        manyhands::repository::RemoveRegistrationOutcome::Removed
    );

    service
        .with_registry_connection_for_testing(|connection| {
            for table in [
                "contexts",
                "discovered_items",
                "discovered_comments",
                "problems",
                "index_operations",
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
