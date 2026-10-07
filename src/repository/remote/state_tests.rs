use super::*;
use crate::repository::{REGISTRY_FILE, RepositoryService};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const ADVERTISED: &str = "1111111111111111111111111111111111111111";
const TRACKING: &str = "2222222222222222222222222222222222222222";

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",[root.path().to_str().unwrap()]).unwrap();
    (data, root, service)
}

#[test]
fn stored_batch_is_queryable_without_changing_local_discovery() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main';
      INSERT INTO remote_observation_batches VALUES(1,1,'origin','main',0,123,1);
      INSERT INTO remote_ref_observations VALUES(1,0,'refs/heads/main','refs/remotes/origin/main','primary','1111111111111111111111111111111111111111',NULL);
      INSERT INTO remote_ref_observations VALUES(1,1,NULL,NULL,'malformed','2222222222222222222222222222222222222222',NULL);").unwrap();
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(snapshot.observations().len(), 2);
    assert_eq!(
        snapshot.observations()[0].advertised_oid().to_string(),
        ADVERTISED
    );
    assert_eq!(
        snapshot.contexts()[0].state(),
        RemoteContextState::Malformed
    );
    assert!(snapshot.contexts()[0].remote_ref().is_none());
}

#[test]
fn invalid_stored_ref_is_rejected_without_leaking_response() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main';
      INSERT INTO remote_observation_batches VALUES(1,1,'origin','main',0,123,1);
      INSERT INTO remote_ref_observations VALUES(1,0,'ssh://SECRET_URL_SENTINEL','SECRET_KEY_PATH_SENTINEL','primary','1111111111111111111111111111111111111111',NULL);").unwrap();
    let error = service.remote_snapshot(root.path()).unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert!(!format!("{error:?} {error}").contains("SENTINEL"));
}

#[test]
fn orphan_remote_rows_require_recovery_during_automatic_startup_audit() {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO remote_ref_observations VALUES(999,0,NULL,NULL,'malformed','1111111111111111111111111111111111111111',NULL)").unwrap();
    let error = match RepositoryService::open_at(data.path()) {
        Ok(_) => panic!("orphan remote row accepted at startup"),
        Err(error) => error,
    };
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
}

fn plan() -> RemoteRefPlan {
    RemoteRefPlan::from_configuration("origin", "main").unwrap()
}

fn advertised() -> RemoteRefObservation {
    RemoteRefObservation::from_advertisement(
        &plan(),
        &format!("refs/heads/manyhands/ticket/{ITEM}"),
        Oid::from_str(ADVERTISED).unwrap(),
        None,
    )
    .unwrap()
}

#[test]
fn current_advertisement_requires_matching_publication_evidence() {
    for corruption in [
        "DELETE FROM remote_context_states",
        "UPDATE remote_context_states SET state='remotely_deleted'",
        "UPDATE remote_context_states SET state='history_unknown',publication_evidence='never_published',last_advertised_oid=NULL",
        "UPDATE remote_context_states SET last_advertised_oid='2222222222222222222222222222222222222222'",
    ] {
        let (data, root, service) = fixture();
        let generation = with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, Some(&plan()), false)
        })
        .unwrap();
        with_transaction(&service, root.path(), |tx, id| {
            complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
        })
        .unwrap();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        connection.execute_batch(corruption).unwrap();
        assert_eq!(
            service.remote_snapshot(root.path()).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "{corruption}"
        );
        assert!(
            with_transaction(&service, root.path(), |tx, id| complete_batch(
                tx,
                id,
                &plan(),
                generation,
                &[],
                124
            ))
            .is_err(),
            "corrupt current evidence must not be silently replaced"
        );
    }
}

#[test]
fn live_context_state_requires_a_matching_current_advertisement() {
    let (data, root, service) = fixture();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute("DELETE FROM remote_ref_observations", [])
        .unwrap();
    assert_eq!(
        service.remote_snapshot(root.path()).unwrap_err().kind,
        RepositoryErrorKind::RecoveryRequired
    );
}

fn vm_work<T>(connection: &Connection, operation: impl FnOnce() -> T) -> (T, usize) {
    use std::cell::Cell;
    unsafe extern "C" fn count(context: *mut std::ffi::c_void) -> std::ffi::c_int {
        // SQLite invokes this synchronously while the stack-owned counter lives.
        let counter = unsafe { &*context.cast::<Cell<usize>>() };
        counter.set(counter.get() + 1);
        0
    }
    struct Reset<'a>(&'a Connection);
    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            // Remove the callback before its stack-owned counter is dropped.
            unsafe {
                rusqlite::ffi::sqlite3_progress_handler(
                    self.0.handle(),
                    0,
                    None,
                    std::ptr::null_mut(),
                );
            }
        }
    }
    let counter = Cell::new(0usize);
    // Connection remains borrowed and counter alive through the entire call.
    unsafe {
        rusqlite::ffi::sqlite3_progress_handler(
            connection.handle(),
            1,
            Some(count),
            std::ptr::from_ref(&counter).cast_mut().cast(),
        );
    }
    let reset = Reset(connection);
    let result = operation();
    drop(reset);
    (result, counter.get())
}

#[test]
fn routine_reads_and_batch_writes_do_not_scale_with_retained_history() {
    let (data, root, service) = fixture();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let (snapshot, base_read) = vm_work(&connection, || read_snapshot(&connection, 1));
    snapshot.unwrap();
    let base_write = with_transaction(&service, root.path(), |tx, id| {
        let (result, work) = vm_work(tx, || {
            complete_batch(tx, id, &plan(), generation, &[advertised()], 124)
        });
        result.map(|_| work)
    })
    .unwrap();
    connection.execute_batch("WITH RECURSIVE numbers(n) AS (VALUES(1000) UNION ALL SELECT n+1 FROM numbers WHERE n<4999)
      INSERT INTO remote_observation_batches SELECT n,1,'origin','main',1,123,0 FROM numbers;
      INSERT INTO remote_ref_observations SELECT id,0,'refs/heads/main','refs/remotes/origin/main','primary','1111111111111111111111111111111111111111',NULL FROM remote_observation_batches WHERE id>=1000;
      INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,created_at,updated_at) SELECT 1,printf('%026d',id),1,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','poll','completed',123,123 FROM remote_observation_batches WHERE id>=1000;").unwrap();
    let (snapshot, history_read) = vm_work(&connection, || read_snapshot(&connection, 1));
    assert_eq!(snapshot.unwrap().observations().len(), 1);
    let history_write = with_transaction(&service, root.path(), |tx, id| {
        let (result, work) = vm_work(tx, || {
            complete_batch(tx, id, &plan(), generation, &[advertised()], 125)
        });
        result.map(|_| work)
    })
    .unwrap();
    eprintln!(
        "SQLite VM work: snapshot {base_read} -> {history_read}; batch {base_write} -> {history_write}; 4000 retained batches and operations"
    );
    assert!(
        history_read <= base_read + 5_000,
        "snapshot VM work grew from {base_read} to {history_read}"
    );
    assert!(
        history_write <= base_write + 5_000,
        "batch VM work grew from {base_write} to {history_write}"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM remote_observation_batches WHERE id BETWEEN 1000 AND 4999",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        4000,
        "historical evidence must be retained"
    );
}

#[test]
fn startup_automatically_audits_historical_refs_and_foreign_keys() {
    for corruption in [
        "UPDATE remote_ref_observations SET tracking_ref='refs/remotes/origin/wrong' WHERE batch_id=1",
        "PRAGMA foreign_keys=OFF; INSERT INTO remote_ref_observations VALUES(999,0,NULL,NULL,'malformed','1111111111111111111111111111111111111111',NULL)",
        "DROP TRIGGER remote_operation_target_immutable; UPDATE remote_operation_records SET primary_ref='refs/heads/wrong'",
    ] {
        let (data, root, service) = fixture();
        let generation = with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, Some(&plan()), false)
        })
        .unwrap();
        with_transaction(&service, root.path(), |tx, id| {
            complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
        })
        .unwrap();
        with_transaction(&service, root.path(), |tx, id| {
            complete_batch(tx, id, &plan(), generation, &[], 124)
        })
        .unwrap();
        with_transaction(&service, root.path(), |tx, id| {
            insert_operation(
                tx,
                id,
                crate::repository::OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan()),
                RemoteOperationPriority::Poll,
                123,
            )
        })
        .unwrap();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        connection
            .execute("UPDATE remote_operation_records SET phase='completed'", [])
            .unwrap();
        connection.execute_batch(corruption).unwrap();
        let error = match RepositoryService::open_at(data.path()) {
            Ok(_) => panic!("startup skipped historical audit: {corruption}"),
            Err(error) => error,
        };
        assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    }
}

#[test]
fn only_complete_batches_replace_current_and_deletion_retains_last_successful_oid() {
    let (data, root, service) = fixture();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(
        snapshot.contexts()[0].state(),
        RemoteContextState::Unmaterialized
    );
    assert_eq!(
        snapshot.contexts()[0].publication_evidence(),
        RemotePublicationEvidence::ObservedPublished
    );
    // Failing/cancelled work records only a fixed outcome, never an empty batch.
    for category in [
        RemoteOutcomeCategory::TransportUnavailable,
        RemoteOutcomeCategory::Cancelled,
    ] {
        with_transaction(&service, root.path(), |tx, id| {
            record_outcome(
                tx,
                id,
                category,
                Some(AutomaticBackoff::from_seconds(60).unwrap()),
            )
        })
        .unwrap();
        let snapshot = service.remote_snapshot(root.path()).unwrap();
        assert_eq!(snapshot.observations().len(), 1);
        assert_eq!(
            snapshot.contexts()[0].state(),
            RemoteContextState::Unmaterialized
        );
        assert_eq!(
            snapshot.polling().delay_for(RemotePollInvocation::Explicit),
            None
        );
    }
    // A failure after the transition rolls the whole replacement back.
    let result: Result<(), RepositoryError> = with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[], 124)?;
        Err(recovery_required())
    });
    assert!(result.is_err());
    assert_eq!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .len(),
        1
    );
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[], 125)
    })
    .unwrap();
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert!(snapshot.observations().is_empty());
    assert_eq!(
        snapshot.contexts()[0].state(),
        RemoteContextState::RemotelyDeleted
    );
    assert_eq!(
        snapshot.contexts()[0].advertised_oid().unwrap().to_string(),
        ADVERTISED
    );
    assert_eq!(
        snapshot.latest_outcome(),
        Some(RemoteOutcomeCategory::Completed)
    );
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM remote_observation_batches WHERE is_current=1",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn configuration_changes_invalidate_remote_state_but_preserve_repository_policy() {
    let (data, root, service) = fixture();
    service
        .set_remote_polling(
            root.path(),
            true,
            true,
            PollingInterval::from_seconds(600).unwrap(),
        )
        .unwrap();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,created_at,updated_at) VALUES (1,'01ARZ3NDEKTSV4RRFFQ69G5FAV',1,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','poll','advertising',123,123)").unwrap();
    for next in [
        None,
        Some(RemoteRefPlan::from_configuration("upstream", "main").unwrap()),
        Some(plan()),
    ] {
        with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, next.as_ref(), true)
        })
        .unwrap();
        let snapshot = service.remote_snapshot(root.path()).unwrap();
        assert!(snapshot.polling().paused());
        assert_eq!(snapshot.polling().interval().as_secs(), 600);
        assert!(snapshot.observations().is_empty());
    }
    assert_eq!(
        connection
            .query_row("SELECT phase FROM remote_operation_records", [], |r| r
                .get::<_, String>(
                0
            ))
            .unwrap(),
        "interrupted"
    );
    assert!(
        with_transaction(&service, root.path(), |tx, id| complete_batch(
            tx,
            id,
            &plan(),
            generation,
            &[advertised()],
            124
        ))
        .is_err()
    );
    assert!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .is_empty()
    );
}

#[test]
fn malformed_response_names_never_enter_database_wal_backups_or_public_formatting() {
    let (data, root, service) = fixture();
    let registry = data.path().join(REGISTRY_FILE);
    // Keep a reader alive while transitions write, so the scan covers an
    // actual live WAL rather than relying only on checkpointed database bytes.
    let connection = Connection::open(&registry).unwrap();
    connection
        .execute_batch("PRAGMA journal_mode=WAL; SELECT count(*) FROM repositories;")
        .unwrap();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    let sentinels = [
        "SECRET_URL_SENTINEL",
        "SECRET_PASSPHRASE_SENTINEL",
        "SECRET_KEY_PATH_SENTINEL",
        "SECRET_RESPONSE_SENTINEL",
        "SECRET_MARKDOWN_SENTINEL",
    ];
    let observations: Vec<_> = sentinels
        .iter()
        .map(|sentinel| {
            RemoteRefObservation::from_advertisement(
                &plan(),
                &format!("refs/heads/manyhands/ticket/{sentinel}"),
                Oid::from_str(ADVERTISED).unwrap(),
                Some(Oid::from_str(TRACKING).unwrap()),
            )
            .unwrap()
        })
        .collect();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &observations, 123)
    })
    .unwrap();
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(snapshot.contexts().len(), 5);
    let formatted = format!("{snapshot:?} {}", snapshot.contexts()[0].state());
    let backup = data.path().join("remote-cache-backup.sqlite3");
    connection
        .execute("VACUUM INTO ?1", [backup.to_str().unwrap()])
        .unwrap();
    assert!(data.path().join(format!("{REGISTRY_FILE}-wal")).is_file());
    for entry in std::fs::read_dir(data.path()).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(path).unwrap();
        for sentinel in sentinels {
            assert!(
                !bytes
                    .windows(sentinel.len())
                    .any(|bytes| bytes == sentinel.as_bytes())
            );
        }
    }
    for sentinel in sentinels {
        assert!(!formatted.contains(sentinel));
    }
}

#[test]
fn invalid_audit_envelope_is_recovery_required_even_when_policy_is_valid() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,created_at,updated_at) VALUES(1,'01ARZ3NDEKTSV4RRFFQ69G5FAV',0,'origin','main','refs/heads/SECRET_RESPONSE_SENTINEL','refs/remotes/origin/main','poll','poll','reserved',123,123)").unwrap();
    let error = service.remote_snapshot(root.path()).unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert!(!format!("{error:?} {error}").contains("SENTINEL"));
}

#[test]
fn stored_context_states_require_valid_targets_and_publication_evidence() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main';
      INSERT INTO remote_context_states VALUES(1,'refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV','refs/remotes/origin/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV','ticket','01ARZ3NDEKTSV4RRFFQ69G5FAV',NULL,NULL,'history_unknown','history_unknown',123);").unwrap();
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(
        snapshot.contexts()[0].state(),
        RemoteContextState::HistoryUnknown
    );
    assert_eq!(
        snapshot.publication_evidence_for(AuthoringKind::Ticket, &ITEM.parse().unwrap()),
        RemotePublicationEvidence::HistoryUnknown
    );
    for state in [
        "observed",
        "unmaterialized",
        "malformed",
        "remotely_deleted",
    ] {
        connection
            .execute("UPDATE remote_context_states SET state=?1", [state])
            .unwrap();
        assert_eq!(
            service.remote_snapshot(root.path()).unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired
        );
    }
}

#[test]
fn operation_target_envelope_is_immutable_and_irrelevant_oid_fields_are_absent() {
    let (data, root, service) = fixture();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    let target = RemoteOperationTarget::for_poll(&plan());
    let operation_id = crate::repository::OperationId::new();
    with_transaction(&service, root.path(), |tx, id| {
        insert_operation(
            tx,
            id,
            operation_id,
            &target,
            RemoteOperationPriority::Poll,
            123,
        )
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert!(
        connection
            .execute(
                "UPDATE remote_operation_records SET remote_name='upstream'",
                []
            )
            .is_err()
    );
    let operations = read_operations(&connection, 1).unwrap();
    assert_eq!(operations[0].target, target);
    assert_eq!(operations[0].phase, RemoteOperationPhase::Reserved);
    assert_eq!(operations[0].completed_step, None);
    assert_eq!(operations[0].local_oid, None);
    assert_eq!(operations[0].tracking_oid, None);
    assert_eq!(operations[0].advertised_oid, None);
}

#[test]
fn local_context_with_lost_history_is_projected_as_unknown_without_a_deletion() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("UPDATE remote_polling_state SET history_unknown=1,recovery_suspended=1,remote_name='origin',primary_branch='main';
      INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active','manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV','/local-context','01ARZ3NDEKTSV4RRFFQ69G5FAV');").unwrap();
    // Exercise the remote projection directly; existing local snapshot path
    // validation deliberately remains independent and unchanged.
    let snapshot = read_snapshot(&connection, 1).unwrap();
    assert_eq!(snapshot.contexts().len(), 1);
    assert_eq!(
        snapshot.contexts()[0].state(),
        RemoteContextState::HistoryUnknown
    );
    assert_eq!(
        snapshot.contexts()[0].publication_evidence(),
        RemotePublicationEvidence::HistoryUnknown
    );
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), 0, &[], 123)
    })
    .unwrap();
    assert_eq!(
        read_snapshot(&connection, 1).unwrap().contexts()[0].state(),
        RemoteContextState::HistoryUnknown
    );
}

#[test]
fn replacing_configuration_does_not_turn_known_publication_into_first_publication() {
    let (_data, root, service) = fixture();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, None, true)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), true)
    })
    .unwrap();
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(
        snapshot.publication_evidence_for(AuthoringKind::Ticket, &ITEM.parse().unwrap()),
        RemotePublicationEvidence::HistoryUnknown
    );
}

// Exact Cycle 04 envelope (before synchronization checkpoints), including its
// original CHECKs. Testing an empty/new database would not exercise this upgrade.
const CYCLE04_OPERATION_SCHEMA: &str = r#"
        CREATE TABLE IF NOT EXISTS remote_operation_records (
            id INTEGER PRIMARY KEY,
            repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            operation_ulid TEXT NOT NULL, configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            remote_name TEXT NOT NULL, primary_branch TEXT NOT NULL,
            primary_ref TEXT NOT NULL, primary_tracking_ref TEXT NOT NULL,
            context_ref TEXT, context_tracking_ref TEXT, kind TEXT CHECK(kind IN ('document','ticket')),
            item_id TEXT, local_branch TEXT,
            action TEXT NOT NULL CHECK(action IN ('poll','synchronize_context','synchronize_primary','promote','close')),
            priority TEXT NOT NULL CHECK(priority IN ('poll','manual')),
            phase TEXT NOT NULL CHECK(phase IN ('reserved','advertising','persisting','completed','interrupted','cancelled','failed')),
            completed_step TEXT CHECK(completed_step IN ('before_transport','after_advertisement','between_observations','before_batch_commit','after_batch_commit','before_local_mutation')),
            local_oid TEXT CHECK(length(local_oid)=40 AND local_oid NOT GLOB '*[^0-9a-f]*'),
            tracking_oid TEXT CHECK(length(tracking_oid)=40 AND tracking_oid NOT GLOB '*[^0-9a-f]*'),
            advertised_oid TEXT CHECK(length(advertised_oid)=40 AND advertised_oid NOT GLOB '*[^0-9a-f]*'),
            created_at INTEGER NOT NULL CHECK(created_at >= 0), updated_at INTEGER NOT NULL CHECK(updated_at >= created_at),
            outcome TEXT CHECK(outcome IN ('completed','configuration_required','selected_key_unavailable','unlock_required','host_approval_required','transport_unavailable','protocol_rejected','cancelled','repository_unavailable')),
            UNIQUE(repository_id,operation_ulid),
            CHECK((context_ref IS NULL)=(context_tracking_ref IS NULL)),
            CHECK((context_ref IS NULL)=(kind IS NULL)), CHECK((kind IS NULL)=(item_id IS NULL)),
            CHECK((action IN ('poll','synchronize_primary'))=(context_ref IS NULL)),
            CHECK((action='poll')=(local_branch IS NULL))
        );
        CREATE INDEX IF NOT EXISTS remote_operations_repository ON remote_operation_records(repository_id,phase,created_at);
        CREATE TRIGGER IF NOT EXISTS remote_operation_target_immutable BEFORE UPDATE OF
            repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,context_ref,context_tracking_ref,kind,item_id,local_branch,action,priority,created_at
            ON remote_operation_records BEGIN SELECT RAISE(ABORT,'immutable remote operation target'); END;
ALTER TABLE remote_operation_records ADD COLUMN yield_requested INTEGER NOT NULL DEFAULT 0 CHECK(yield_requested IN (0,1));
ALTER TABLE remote_operation_records ADD COLUMN cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1));
ALTER TABLE remote_operation_records ADD COLUMN owner_epoch INTEGER NOT NULL DEFAULT 0 CHECK(owner_epoch>=0);
CREATE UNIQUE INDEX remote_active_reservation ON remote_operation_records(repository_id) WHERE phase IN ('reserved','advertising','persisting');
"#;

#[test]
fn cycle04_poll_migration_preserves_terminal_rows_without_inferred_publication() {
    let (data, root, service) = fixture();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE remote_operation_records")
        .unwrap();
    connection.execute_batch(CYCLE04_OPERATION_SCHEMA).unwrap();
    let cases = [
        ("completed", "after_batch_commit", Some("completed"), 0, 0),
        ("interrupted", "after_advertisement", None, 0, 0),
        ("interrupted", "before_batch_commit", None, 1, 0),
        ("cancelled", "before_transport", Some("cancelled"), 0, 1),
        (
            "failed",
            "after_advertisement",
            Some("transport_unavailable"),
            0,
            0,
        ),
    ];
    let mut ids = Vec::new();
    for (phase, step, outcome, yield_requested, cancel_requested) in cases {
        let id = crate::repository::OperationId::new();
        ids.push(id);
        connection.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,tracking_oid,advertised_oid,created_at,updated_at,outcome,yield_requested,cancel_requested,owner_epoch) VALUES(1,?1,1,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','poll',?2,?3,?4,?5,123,124,?6,?7,?8,3)",params![id.to_string(),phase,step,TRACKING,ADVERTISED,outcome,yield_requested,cancel_requested]).unwrap();
    }
    drop(connection);
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    with_transaction(&reopened, root.path(), |tx, id| {
        let records = read_operations(tx, id)?;
        assert_eq!(records.len(), cases.len());
        for ((record, id), (phase, step, outcome, yield_requested, cancel_requested)) in
            records.iter().zip(ids).zip(cases)
        {
            assert_eq!(record.operation_id, id);
            let stored: (String, String, Option<String>) = tx
                .query_row(
                    "SELECT phase,completed_step,outcome FROM remote_operation_records WHERE id=?1",
                    [record.id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap();
            assert_eq!(
                stored,
                (phase.into(), step.into(), outcome.map(str::to_owned))
            );
            assert_eq!(record.tracking_oid, Some(Oid::from_str(TRACKING).unwrap()));
            assert_eq!(
                record.advertised_oid,
                Some(Oid::from_str(ADVERTISED).unwrap())
            );
            assert_eq!(record.created_at, 123);
            assert_eq!(record.updated_at, 124);
            assert_eq!(record.owner_epoch, 3);
            assert_eq!(record.yield_requested, yield_requested == 1);
            assert_eq!(record.cancel_requested, cancel_requested == 1);
            assert_eq!(record.sync_checkpoint, None);
            assert_eq!(record.authority, None);
            assert!(!record.index_pending);
            assert!(!record.reconciliation_required);
            assert_eq!(record.sync_evidence.expected_oid, None);
            assert_eq!(record.sync_evidence.push_oid, None);
        }
        assert!(read_snapshot(tx, id)?.contexts().is_empty());
        Ok(())
    })
    .unwrap();
}

#[test]
fn partial_sync_schema_does_not_discard_durable_candidate() {
    let (data, _root, _service) = fixture();
    Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .execute_batch("ALTER TABLE remote_operation_records DROP COLUMN push_oid")
        .unwrap();
    assert!(
        matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind==RepositoryErrorKind::RecoveryRequired)
    );
}

#[test]
fn task2_merge_evidence_migration_preserves_cycle05_authority_and_is_idempotent() {
    let (data, root, service) = fixture();
    let operation_id = crate::repository::OperationId::new();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan());
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)?;
        insert_operation(
            tx,
            id,
            operation_id,
            &target,
            RemoteOperationPriority::Manual,
            123,
        )?;
        tx.execute(
            "UPDATE remote_operation_records SET phase='completed',sync_checkpoint='discovery_pending',expected_oid=?2,local_oid=?3,primary_tracking_oid=?2,push_oid=?3,push_advertised_oid=?3,authoritative_kind='published',authoritative_oid=?3,index_pending=1 WHERE operation_ulid=?1",
            params![operation_id.to_string(), ADVERTISED, TRACKING],
        )
        .map_err(|_| recovery_required())?;
        Ok(())
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps;").unwrap();
    drop(connection);
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    for table in [
        "remote_integration_steps",
        "remote_identity_confirmations",
        "remote_resolution_attempts",
        "remote_resolution_paths",
    ] {
        assert!(
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                    [table],
                    |row| row.get::<_, bool>(0)
                )
                .unwrap(),
            "missing {table}"
        );
    }
    assert_eq!(
        connection.query_row("SELECT authoritative_kind,index_pending FROM remote_operation_records WHERE operation_ulid=?1", [operation_id.to_string()], |row| Ok((row.get::<_, String>(0)?,row.get::<_, i64>(1)?))).unwrap(),
        ("published".to_owned(), 1)
    );
    drop(connection);
    let mut connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let transaction = connection.transaction().unwrap();
    migrate(&transaction).unwrap();
    transaction.commit().unwrap();
    assert!(
        matches!(reopened.reserve_remote_operation(root.path(), operation_id, &target).unwrap(), super::super::reservation::RemoteReservationOutcome::Replay(record) if record.index_pending())
    );
}

#[test]
fn partial_task2_evidence_schema_and_orphan_rows_fail_closed() {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE remote_resolution_paths;")
        .unwrap();
    let mut connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let transaction = connection.transaction().unwrap();
    assert!(migrate(&transaction).is_err());
    transaction.rollback().unwrap();

    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,phase) VALUES(999,0,0,0,'primary','1111111111111111111111111111111111111111','2222222222222222222222222222222222222222','3333333333333333333333333333333333333333',zeroblob(32),'prepared')").unwrap();
    drop(connection);
    let mut connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert!(audit_registry(&mut connection).is_err());
}

#[test]
fn task2_evidence_schema_has_no_content_or_transport_columns_and_wal_stays_redacted() {
    let (data, root, service) = fixture();
    let registry = data.path().join(REGISTRY_FILE);
    let connection = Connection::open(&registry).unwrap();
    connection
        .execute_batch("PRAGMA journal_mode=WAL; SELECT count(*) FROM repositories;")
        .unwrap();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    assert_eq!(generation, 1);
    let schema: String = connection.prepare("SELECT group_concat(sql, '\n') FROM sqlite_master WHERE name IN ('remote_integration_steps','remote_identity_confirmations','remote_resolution_attempts','remote_resolution_paths')").unwrap().query_row([], |row| row.get(0)).unwrap();
    for forbidden in ["body", "credential", "endpoint", "server", "path TEXT"] {
        assert!(
            !schema.to_ascii_lowercase().contains(forbidden),
            "persisted forbidden column {forbidden}"
        );
    }
    let backup = data.path().join("task2-backup.sqlite3");
    connection
        .execute("VACUUM INTO ?1", [backup.to_str().unwrap()])
        .unwrap();
    for entry in std::fs::read_dir(data.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        for sentinel in [
            b"PRIVATE_BODY_SENTINEL".as_slice(),
            b"PRIVATE_CREDENTIAL_SENTINEL".as_slice(),
            b"PRIVATE_ENDPOINT_SENTINEL".as_slice(),
        ] {
            assert!(
                !bytes
                    .windows(sentinel.len())
                    .any(|window| window == sentinel)
            );
        }
    }
}

#[test]
fn idempotent_sync_migration_does_not_scan_retained_operation_history() {
    let (data, _root, _service) = fixture();
    let mut connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let transaction = connection.transaction().unwrap();
    let (result, before) = vm_work(&transaction, || migrate(&transaction));
    result.unwrap();
    transaction.commit().unwrap();
    connection.execute_batch("WITH RECURSIVE numbers(n) AS (VALUES(1000) UNION ALL SELECT n+1 FROM numbers WHERE n<4999)
        INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,created_at,updated_at)
        SELECT 1,printf('%026d',n),0,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','poll','completed',123,123 FROM numbers").unwrap();
    let transaction = connection.transaction().unwrap();
    let (result, after) = vm_work(&transaction, || migrate(&transaction));
    result.unwrap();
    assert!(
        after <= before + 100,
        "idempotent migration walked completed remote history: {before} -> {after} VM steps"
    );
    transaction.commit().unwrap();
}
