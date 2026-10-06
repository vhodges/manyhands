use std::fs;

use manyhands::repository::{
    AuthoringKind, OperationId, PollingInterval, REGISTRY_FILE, RebuildRepositoryRequest,
    RemotePublicationEvidence, RepositoryErrorKind, RepositoryService,
};
use rusqlite::Connection;

mod support;

#[test]
fn migration_preserves_cycle03_rows_and_installs_one_default_policy() {
    let data = tempfile::tempdir().unwrap();
    let registry = data.path().join(REGISTRY_FILE);
    let connection = Connection::open(&registry).unwrap();
    connection.execute_batch("CREATE TABLE repositories (
        id INTEGER PRIMARY KEY, root_path TEXT NOT NULL UNIQUE, enabled_at INTEGER NOT NULL,
        accessibility TEXT NOT NULL, config_blob_oid TEXT, refresh_required INTEGER NOT NULL);
        INSERT INTO repositories VALUES (7, '/fixture', 123, 'accessible', NULL, 0);
        CREATE TABLE operation_records (
          id INTEGER PRIMARY KEY, repository_id INTEGER, root_path TEXT NOT NULL,
          operation_ulid TEXT, action TEXT NOT NULL, target TEXT, item_id TEXT,
          context_path TEXT, state TEXT NOT NULL, completed_step TEXT, observed_at INTEGER NOT NULL,
          persisted_context_count INTEGER NOT NULL DEFAULT 0, redacted_error TEXT);
        INSERT INTO operation_records VALUES (13,7,'/fixture',NULL,'refresh',NULL,NULL,NULL,'completed',NULL,123,0,NULL);").unwrap();
    drop(connection);
    for _ in 0..3 {
        RepositoryService::open_at(data.path()).unwrap();
    }
    let connection = Connection::open(&registry).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT root_path FROM repositories WHERE id=7", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "/fixture"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT action FROM operation_records WHERE id=13",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "refresh"
    );
    let tables: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('remote_polling_state','remote_observation_batches','remote_ref_observations','remote_context_states','remote_operation_records')", [], |r| r.get(0)).unwrap();
    assert_eq!(tables, 5, "dedicated remote schema missing");
    assert_eq!(connection.query_row("SELECT enabled,paused,interval_seconds,recovery_suspended FROM remote_polling_state WHERE repository_id=7", [], |r| Ok((r.get::<_, i64>(0)?,r.get::<_, i64>(1)?,r.get::<_, i64>(2)?,r.get::<_, i64>(3)?))).unwrap(), (1,0,300,0));
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM remote_polling_state", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn cache_replacement_publishes_remote_history_marker() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join(REGISTRY_FILE), b"corrupt registry").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .rebuild_repository(RebuildRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert_eq!(
        fs::read(data.path().join("remote-history-recovery-required")).unwrap(),
        b"manyhands remote history recovery required v1\n"
    );
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT paused,recovery_suspended,history_unknown FROM remote_polling_state",
                [],
                |r| Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?
                ))
            )
            .unwrap(),
        (0, 1, 1)
    );
}

#[test]
fn remote_marker_failure_aborts_replacement_before_renaming_registry() {
    for directory in [false, true] {
        let fixture = support::born_repository();
        let data = tempfile::tempdir().unwrap();
        let marker = data.path().join("remote-history-recovery-required");
        if directory {
            fs::create_dir(&marker).unwrap();
        } else {
            fs::write(&marker, b"invalid marker").unwrap();
        }
        let registry = data.path().join(REGISTRY_FILE);
        fs::write(&registry, b"corrupt registry").unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        assert!(
            service
                .rebuild_repository(RebuildRepositoryRequest {
                    root: fixture.root.clone(),
                    operation_id: OperationId::new()
                })
                .is_err()
        );
        assert_eq!(fs::read(registry).unwrap(), b"corrupt registry");
        assert!(!fs::read_dir(data.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".corrupt-")
        }));
    }
}

#[test]
fn polling_policy_survives_reopen_and_local_snapshot_stays_intact() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let before = enabled.service.repository_snapshot(&fixture.root).unwrap();
    enabled
        .service
        .set_remote_polling(
            &fixture.root,
            true,
            true,
            PollingInterval::from_seconds(60).unwrap(),
        )
        .unwrap();
    let reopened = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let after = reopened.repository_snapshot(&fixture.root).unwrap();
    assert_eq!(before.items, after.items);
    assert_eq!(before.contexts, after.contexts);
    assert_eq!(before.problems, after.problems);
    assert!(after.remote.polling().paused());
    assert_eq!(after.remote.polling().interval().as_secs(), 60);
}

#[test]
fn corrupt_remote_policy_requires_recovery_without_repair_or_raw_error() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let connection = Connection::open(enabled.data_directory.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE remote_polling_state SET interval_seconds=1,latest_outcome='SECRET_RESPONSE_SENTINEL'").unwrap();
    let error = enabled
        .service
        .repository_snapshot(&fixture.root)
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert!(!format!("{error:?} {error}").contains("SECRET_RESPONSE_SENTINEL"));
    assert!(
        enabled
            .service
            .set_remote_polling(
                &fixture.root,
                true,
                false,
                PollingInterval::from_seconds(300).unwrap()
            )
            .is_err()
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT interval_seconds FROM remote_polling_state",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn absent_context_remains_history_unknown_after_cache_loss_and_restart() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join(REGISTRY_FILE), b"corrupt registry").unwrap();
    RepositoryService::open_at(data.path())
        .unwrap()
        .rebuild_repository(RebuildRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let remote = service.remote_snapshot(&fixture.root).unwrap();
    let id = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    assert!(remote.polling().recovery_suspended());
    assert!(!remote.polling().paused());
    assert_eq!(
        remote.publication_evidence_for(AuthoringKind::Ticket, &id),
        RemotePublicationEvidence::HistoryUnknown
    );
}

#[test]
fn reopening_does_not_repair_deleted_policy_or_partial_remote_schema() {
    for corruption in [
        "DELETE FROM remote_polling_state",
        "DROP TABLE remote_context_states",
    ] {
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        let connection =
            Connection::open(enabled.data_directory.path().join(REGISTRY_FILE)).unwrap();
        connection.execute_batch(corruption).unwrap();
        let error = match RepositoryService::open_at(enabled.data_directory.path()) {
            Ok(_) => panic!("corrupt remote state was silently repaired"),
            Err(error) => error,
        };
        assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    }
}
