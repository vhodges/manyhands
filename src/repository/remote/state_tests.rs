use super::*;
use crate::repository::{REGISTRY_FILE, RepositoryService};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const ADVERTISED: &str = "1111111111111111111111111111111111111111";
const TRACKING: &str = "2222222222222222222222222222222222222222";

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    fixture_in(&std::env::temp_dir())
}

fn fixture_in(parent: &Path) -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    // Match canonical registry lookups, including Windows verbatim prefixes and
    // macOS /var aliases, before creating either fixture directory.
    let parent = parent.canonicalize().unwrap();
    let data = tempfile::tempdir_in(&parent).unwrap();
    let root = tempfile::tempdir_in(&parent).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)",[root.path().to_str().unwrap()]).unwrap();
    (data, root, service)
}

#[cfg(unix)]
#[test]
fn state_fixture_symlink_parent_configures_and_reads_registered_root() {
    let temporary = tempfile::tempdir().unwrap();
    let parent = temporary.path().canonicalize().unwrap();
    let real = parent.join("real");
    let alias = parent.join("alias");
    std::fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    let (data, root, service) = fixture_in(&alias);

    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    assert_eq!(
        service.remote_snapshot(root.path()).unwrap().observations(),
        &[advertised()]
    );
    assert_eq!(data.path(), data.path().canonicalize().unwrap());
    assert_eq!(root.path(), root.path().canonicalize().unwrap());
    let registered: String = Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .query_row("SELECT root_path FROM repositories", [], |row| row.get(0))
        .unwrap();
    assert_eq!(registered, root.path().to_str().unwrap());
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

fn legacy_preflight_schema() -> String {
    legacy_window_fixture_schema()
        .replace(
            "            preflight_digest BLOB NOT NULL CHECK(typeof(preflight_digest)='blob' AND length(preflight_digest)=32),\n",
            "",
        )
        .replace(
            "observation_digest,input_digest,preflight_digest,identity_confirmation_id",
            "observation_digest,input_digest,identity_confirmation_id",
        )
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
        .execute_batch("DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows; DROP TABLE remote_operation_records")
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
    connection.execute_batch("DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows;").unwrap();
    drop(connection);
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    for table in [
        "remote_integration_steps",
        "remote_integration_windows",
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
fn legacy_preflight_digest_migration_upgrades_empty_attempts_and_rejects_populated_attempts() {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows;")
        .unwrap();
    connection
        .execute_batch(&legacy_preflight_schema())
        .unwrap();
    drop(connection);
    // Empty legacy evidence has no unobserved attempt to preserve, so it is
    // rebuilt with the immutable preflight column.
    let _reopened = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert!(
        connection
            .prepare("PRAGMA table_info(remote_resolution_attempts)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(Result::unwrap)
            .any(|column| column == "preflight_digest")
    );

    let (data, root, service) = fixture();
    let operation = crate::repository::OperationId::new();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan());
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)?;
        insert_operation(
            tx,
            id,
            operation,
            &target,
            RemoteOperationPriority::Manual,
            123,
        )
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let (operation_id, generation): (i64, i64) = connection
        .query_row(
            "SELECT id,configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    connection
        .execute_batch("DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows;")
        .unwrap();
    connection
        .execute_batch(&legacy_preflight_schema())
        .unwrap();
    connection.execute(
        "INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,conflict_digest,phase) VALUES(?1,?2,0,0,'primary',?3,?4,?5,zeroblob(32),zeroblob(32),'conflict_pending')",
        params![operation_id,generation,ADVERTISED,TRACKING,ADVERTISED],
    ).unwrap();
    connection.execute(
        "INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,identity_confirmation_id,phase) VALUES(?1,?2,1,?3,0,zeroblob(32),zeroblob(32),NULL,'prepared')",
        params![crate::repository::OperationId::new().to_string(),operation_id,generation],
    ).unwrap();
    drop(connection);
    // A populated old attempt has no authentic preflight observation and must
    // fail closed rather than receive a manufactured digest.
    assert!(
        matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
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
fn weakened_complete_task2_evidence_schema_requires_recovery() {
    for (label, schema) in [
        (
            "foreign key",
            MERGE_EVIDENCE_SCHEMA.replace(" ON DELETE RESTRICT", ""),
        ),
        (
            "check constraint",
            MERGE_EVIDENCE_SCHEMA.replace(
                "CHECK(phase!='applied' OR checkpoint_oid IS NOT NULL)",
                "CHECK(1)",
            ),
        ),
        (
            "index",
            MERGE_EVIDENCE_SCHEMA.replace(
                "CREATE INDEX remote_resolution_attempts_operation ON remote_resolution_attempts(operation_record_id,integration_step_id);",
                "",
            ),
        ),
        (
            "trigger behavior",
            MERGE_EVIDENCE_SCHEMA.replace(
                "SELECT RAISE(ABORT,'immutable resolution evidence')",
                "SELECT 1",
            ),
        ),
    ] {
        let (data, _root, _service) = fixture();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        connection
            .execute_batch(
                "DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows;",
            )
            .unwrap();
        connection.execute_batch(&schema).unwrap();
        drop(connection);
        assert!(
            matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired),
            "accepted weakened {label} schema"
        );
    }
}

#[test]
fn incompatible_context_integration_stage_requires_recovery() {
    let (data, root, service) = fixture();
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
    }
    .operation_target(&plan());
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)?;
        insert_operation(
            tx,
            id,
            crate::repository::OperationId::new(),
            &target,
            RemoteOperationPriority::Manual,
            123,
        )?;
        Ok(())
    })
    .unwrap();
    Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .execute_batch("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,phase) VALUES(1,1,0,0,'primary','1111111111111111111111111111111111111111','2222222222222222222222222222222222222222','3333333333333333333333333333333333333333',zeroblob(32),'prepared')")
        .unwrap();
    assert!(
        matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
    );
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
    let schema: String = connection.prepare("SELECT group_concat(sql, '\n') FROM sqlite_master WHERE name IN ('remote_integration_windows','remote_integration_steps','remote_identity_confirmations','remote_resolution_attempts','remote_resolution_paths')").unwrap().query_row([], |row| row.get(0)).unwrap();
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

#[test]
fn index_artifact_schema_migration_is_additive_and_rejects_weakened_provenance() {
    let (data, root, service) = fixture();
    let operation = crate::repository::OperationId::new();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)?;
        insert_operation(
            tx,
            id,
            operation,
            &RemoteOperationTarget::for_primary_synchronization(&plan()),
            RemoteOperationPriority::Manual,
            123,
        )
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let (parent,generation): (i64,i64) = connection.query_row("SELECT id,configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",[operation.to_string()],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
    connection
        .execute_batch("DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts;")
        .unwrap();
    connection.execute("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,conflict_digest,phase) VALUES(?1,?2,0,0,'primary',?3,?4,?5,zeroblob(32),zeroblob(32),'resolution_prepared')",params![parent,generation,ADVERTISED,TRACKING,ADVERTISED]).unwrap();
    let attempt = crate::repository::OperationId::new();
    connection.execute("INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,phase) VALUES(?1,?2,1,?3,0,zeroblob(32),zeroblob(32),zeroblob(32),'paths_applying')",params![attempt.to_string(),parent,generation]).unwrap();
    drop(connection);
    let _reopened = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT phase FROM remote_resolution_attempts WHERE attempt_ulid=?1",
                [attempt.to_string()],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "paths_applying"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM remote_resolution_index_artifacts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0,
        "legacy attempts gain no inferred ownership"
    );
    connection.execute_batch("DROP TRIGGER remote_resolution_index_artifact_immutable; CREATE TRIGGER remote_resolution_index_artifact_immutable BEFORE UPDATE ON remote_resolution_index_artifacts BEGIN SELECT 1; END;").unwrap();
    assert!(
        matches!(RepositoryService::open_at(data.path()),Err(error) if error.kind==RepositoryErrorKind::RecoveryRequired)
    );
}

#[test]
fn orphan_index_artifact_evidence_fails_startup_audit() {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO remote_resolution_index_artifacts(attempt_id,device,inode,sentinel_digest,baseline_digest,baseline_device,baseline_inode,phase) VALUES(999,1,2,zeroblob(32),zeroblob(32),1,3,'intent');").unwrap();
    drop(connection);
    assert!(
        matches!(RepositoryService::open_at(data.path()),Err(error) if error.kind==RepositoryErrorKind::RecoveryRequired)
    );
}

#[test]
fn ref_log_evidence_additive_migration_partial_schema_and_orphans_fail_closed() {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE remote_resolution_ref_log_artifacts;")
        .unwrap();
    drop(connection);
    RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM remote_resolution_ref_log_artifacts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert!(connection.execute("INSERT INTO remote_resolution_ref_log_artifacts(attempt_id,role,device,inode,digest) VALUES(99,'arbitrary-path',1,2,zeroblob(32))", []).is_err());
    connection
        .execute_batch("DROP TRIGGER remote_resolution_ref_log_artifact_immutable;")
        .unwrap();
    drop(connection);
    assert!(
        matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
    );

    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO remote_resolution_ref_log_artifacts(attempt_id,role,device,inode,digest) VALUES(99,'baseline',1,2,zeroblob(32));").unwrap();
    drop(connection);
    assert!(
        matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
    );
}

// Task 5 source-first regressions. Execution is intentionally left to native CI.
fn legacy_window_fixture_schema() -> String {
    // Pinned Task 2/4 SQL: do not derive the changed objects through the
    // production legacy decoder, or the migration test could share its mistake.
    const STEPS: &str = r#"CREATE TABLE remote_integration_steps (
            id INTEGER PRIMARY KEY,
            operation_record_id INTEGER NOT NULL REFERENCES remote_operation_records(id) ON DELETE CASCADE,
            configuration_generation INTEGER NOT NULL CHECK(configuration_generation >= 0),
            owner_epoch INTEGER NOT NULL CHECK(owner_epoch >= 0),
            ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 1),
            stage TEXT NOT NULL CHECK(stage IN ('context','primary')),
            local_oid TEXT NOT NULL CHECK(length(local_oid)=40 AND local_oid NOT GLOB '*[^0-9a-f]*'),
            incoming_oid TEXT NOT NULL CHECK(length(incoming_oid)=40 AND incoming_oid NOT GLOB '*[^0-9a-f]*'),
            baseline_tree_oid TEXT NOT NULL CHECK(length(baseline_tree_oid)=40 AND baseline_tree_oid NOT GLOB '*[^0-9a-f]*'),
            baseline_index_digest BLOB NOT NULL CHECK(typeof(baseline_index_digest)='blob' AND length(baseline_index_digest)=32),
            candidate_oid TEXT CHECK(length(candidate_oid)=40 AND candidate_oid NOT GLOB '*[^0-9a-f]*'),
            result_oid TEXT CHECK(length(result_oid)=40 AND result_oid NOT GLOB '*[^0-9a-f]*'),
            observed_tree_oid TEXT CHECK(length(observed_tree_oid)=40 AND observed_tree_oid NOT GLOB '*[^0-9a-f]*'),
            conflict_digest BLOB CHECK(conflict_digest IS NULL OR (typeof(conflict_digest)='blob' AND length(conflict_digest)=32)),
            phase TEXT NOT NULL CHECK(phase IN ('prepared','applying','conflict_pending','resolution_prepared','commit_prepared','applied','recovery_required')),
            UNIQUE(operation_record_id,ordinal), UNIQUE(operation_record_id,stage),
            CHECK(phase!='conflict_pending' OR conflict_digest IS NOT NULL),
            CHECK(phase!='commit_prepared' OR candidate_oid IS NOT NULL),
            CHECK(phase!='applied' OR (result_oid IS NOT NULL AND observed_tree_oid IS NOT NULL))
        );"#;
    MERGE_SCHEMA_OBJECTS
        .iter()
        .filter_map(|&(kind, name)| match name {
            "remote_integration_windows"
            | "remote_integration_windows_batch"
            | "remote_integration_window_immutable" => None,
            "remote_integration_steps" => Some(STEPS),
            "remote_integration_steps_operation" => Some("CREATE INDEX remote_integration_steps_operation ON remote_integration_steps(operation_record_id,ordinal);"),
            "remote_integration_step_immutable" => Some("CREATE TRIGGER remote_integration_step_immutable BEFORE UPDATE OF operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest ON remote_integration_steps BEGIN SELECT RAISE(ABORT,'immutable integration evidence'); END;"),
            _ => merge_schema_object_sql(kind, name),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn replace_with_legacy_merge_schema(connection: &Connection) {
    connection.execute_batch("DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows;").unwrap();
    connection
        .execute_batch(&legacy_window_fixture_schema())
        .unwrap();
}

#[test]
fn window_migration_preserves_legacy_step_ids_attempt_paths_and_native_provenance() {
    let (data, root, service) = fixture();
    let operation = crate::repository::OperationId::new();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), false)?;
        insert_operation(
            tx,
            id,
            operation,
            &RemoteOperationTarget::for_primary_synchronization(&plan()),
            RemoteOperationPriority::Manual,
            123,
        )
    })
    .unwrap();
    let mut connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    replace_with_legacy_merge_schema(&connection);
    connection.execute_batch("PRAGMA foreign_keys=ON;
        INSERT INTO remote_integration_steps(id,operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,conflict_digest,phase) VALUES(41,1,1,0,0,'primary','1111111111111111111111111111111111111111','2222222222222222222222222222222222222222','3333333333333333333333333333333333333333',zeroblob(32),zeroblob(32),'resolution_prepared');
        INSERT INTO remote_identity_confirmations(id,confirmation_ulid,operation_record_id,configuration_generation,owner_epoch,input_digest,configuration_digest,phase) VALUES(61,'01ARZ3NDEKTSV4RRFFQ69G5FAY',1,1,0,zeroblob(32),zeroblob(32),'prepared');
        INSERT INTO remote_resolution_attempts(id,attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,identity_confirmation_id,phase) VALUES(51,'01ARZ3NDEKTSV4RRFFQ69G5FAZ',1,41,1,0,zeroblob(32),zeroblob(32),zeroblob(32),61,'paths_applying');
        INSERT INTO remote_resolution_paths VALUES(51,0,zeroblob(32),zeroblob(32),zeroblob(32),zeroblob(32),NULL,NULL,NULL,33188,1);
        INSERT INTO remote_resolution_index_artifacts(attempt_id,device,inode,sentinel_digest,baseline_digest,baseline_device,baseline_inode,phase) VALUES(51,7,8,zeroblob(32),zeroblob(32),9,10,'intent');
        INSERT INTO remote_resolution_ref_log_artifacts VALUES(51,'baseline',11,12,zeroblob(32));").unwrap();
    let snapshot = |connection: &Connection, table: &str| {
        let mut query = connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1"))
            .unwrap();
        let columns = query.column_count();
        query
            .query_map([], |row| {
                (0..columns)
                    .map(|index| row.get::<_, rusqlite::types::Value>(index))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    let tables = [
        "remote_resolution_attempts",
        "remote_resolution_paths",
        "remote_resolution_index_artifacts",
        "remote_resolution_ref_log_artifacts",
        "remote_identity_confirmations",
        "remote_operation_records",
    ];
    let before = tables.map(|table| snapshot(&connection, table));
    let original_steps = snapshot(&connection, "remote_integration_steps");
    for _ in 0..2 {
        let tx = connection.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.commit().unwrap();
        for (table, expected) in tables.iter().zip(&before) {
            assert_eq!(&snapshot(&connection, table), expected, "changed {table}");
        }
        let mut migrated_steps = snapshot(&connection, "remote_integration_steps");
        for row in &mut migrated_steps {
            assert_eq!(row.pop(), Some(rusqlite::types::Value::Integer(0)));
        }
        assert_eq!(migrated_steps, original_steps);
        let record = read_operation(&connection, 1, operation).unwrap().unwrap();
        assert_eq!(
            integration_window(&connection, &record, 0).unwrap(),
            Some(IntegrationWindowEvidence {
                number: 0,
                intent: None,
            })
        );
        assert_eq!(
            connection.query_row("SELECT kind,observation_batch_id,local_oid,primary_oid,context_oid FROM remote_integration_windows", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, Option<String>>(4)?))).unwrap(),
            ("legacy".into(), None, None, None, None)
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_temp_master WHERE name LIKE '%_window_upgrade'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
    for mutation in [
        "UPDATE remote_integration_steps SET window_number=1 WHERE id=41",
        "UPDATE remote_resolution_attempts SET integration_step_id=99 WHERE id=51",
        "UPDATE remote_resolution_index_artifacts SET inode=99 WHERE attempt_id=51",
        "UPDATE remote_resolution_ref_log_artifacts SET digest=randomblob(32) WHERE attempt_id=51",
    ] {
        assert!(connection.execute(mutation, []).is_err());
    }
}

fn window_fixture(service: &RepositoryService, root: &Path) -> crate::repository::OperationId {
    let operation = crate::repository::OperationId::new();
    with_transaction(service, root, |tx, id| {
        configure(tx, id, Some(&plan()), false)?;
        let target = RemoteOperationTarget::for_context(
            &plan(),
            RemoteOperationAction::SynchronizeContext,
            crate::repository::AuthoringKind::Ticket,
            ITEM.parse().unwrap(),
        )
        .unwrap();
        insert_operation(
            tx,
            id,
            operation,
            &target,
            RemoteOperationPriority::Manual,
            123,
        )
    })
    .unwrap();
    operation
}

fn pinned_window(
    service: &RepositoryService,
    root: &Path,
    operation: crate::repository::OperationId,
    context: Option<Oid>,
) -> IntegrationWindowIntent {
    with_transaction(service, root, |tx, id| {
        let mut observations = vec![
            RemoteRefObservation::from_advertisement(
                &plan(),
                "refs/heads/main",
                Oid::from_str(ADVERTISED).unwrap(),
                Some(Oid::from_str(ADVERTISED).unwrap()),
            )
            .unwrap(),
        ];
        if let Some(context) = context {
            observations.push(
                RemoteRefObservation::from_advertisement(
                    &plan(),
                    &format!("refs/heads/manyhands/ticket/{ITEM}"),
                    context,
                    Some(context),
                )
                .unwrap(),
            );
        }
        let record = read_operation(tx, id, operation)?.unwrap();
        let batch = complete_batch(tx, id, &plan(), record.generation, &observations, 123)?;
        Ok(IntegrationWindowIntent {
            observation_batch_id: batch,
            local_oid: Oid::from_str(TRACKING).unwrap(),
            primary_oid: Oid::from_str(ADVERTISED).unwrap(),
            context_oid: context,
        })
    })
    .unwrap()
}

#[test]
fn absent_context_window_keeps_primary_at_slot_one_and_freezes_the_ref_pass() {
    let (_data, root, service) = fixture();
    let operation = window_fixture(&service, root.path());
    let intent = pinned_window(&service, root.path(), operation, None);
    with_transaction(&service, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        let window = prepare_integration_window(tx, &record, 1, &intent)?;
        assert_eq!(window.intent, Some(intent.clone()));
        assert_eq!(prepare_integration_window(tx, &record, 1, &intent)?, window);
        let step = IntegrationStepIntent {
            ordinal: 1,
            stage: IntegrationStage::Primary,
            local_oid: intent.local_oid,
            incoming_oid: intent.primary_oid,
            baseline_tree_oid: intent.local_oid,
            baseline_index_digest: [5; 32],
        };
        assert!(
            prepare_integration_step_in_window(
                tx,
                &record,
                1,
                &IntegrationStepIntent {
                    ordinal: 0,
                    ..step.clone()
                },
            )
            .is_err()
        );
        prepare_integration_step_in_window(tx, &record, 1, &step)?;
        assert!(
            prepare_integration_window(tx, &record, 2, &intent).is_err(),
            "cannot append past a prepared effect"
        );
        begin_integration_effect_in_window(tx, &record, 1, 1, None)?;
        observe_integration_effect_in_window(
            tx,
            &record,
            1,
            1,
            step.incoming_oid,
            step.baseline_tree_oid,
        )?;
        assert!(
            prepare_integration_window(
                tx,
                &record,
                1,
                &IntegrationWindowIntent {
                    local_oid: intent.primary_oid,
                    ..intent.clone()
                },
            )
            .is_err()
        );
        assert!(
            prepare_integration_window(tx, &record, 2, &intent).is_err(),
            "unchanged pass must not duplicate candidates"
        );
        let later = prepare_integration_window(
            tx,
            &record,
            2,
            &IntegrationWindowIntent {
                local_oid: step.incoming_oid,
                ..intent.clone()
            },
        )?;
        assert_eq!(later.number, 2);
        assert!(
            integration_step(tx, record.id, 1)?.is_none(),
            "legacy lookup must never select a later window"
        );
        assert_eq!(
            integration_step_in_window(tx, record.id, 1, 1)?
                .unwrap()
                .phase,
            IntegrationStepPhase::Applied
        );
        audit_merge_evidence(tx, &record)
    })
    .unwrap();
}

#[test]
fn new_window_requires_exact_batch_generation_and_tracking_oids() {
    let (_data, root, service) = fixture();
    let operation = window_fixture(&service, root.path());
    let intent = pinned_window(&service, root.path(), operation, None);
    for mutation in [
        "UPDATE remote_ref_observations SET tracking_oid=NULL",
        "UPDATE remote_observation_batches SET configuration_generation=99",
    ] {
        with_transaction(&service, root.path(), |tx, id| {
            let record = read_operation(tx, id, operation)?.unwrap();
            tx.execute_batch("SAVEPOINT corrupt_batch").unwrap();
            tx.execute_batch(mutation).unwrap();
            assert!(prepare_integration_window(tx, &record, 1, &intent).is_err());
            tx.execute_batch("ROLLBACK TO corrupt_batch; RELEASE corrupt_batch")
                .unwrap();
            Ok(())
        })
        .unwrap();
    }
}

#[test]
fn append_cannot_skip_applied_context_with_unprepared_primary_or_unreleased_resolution() {
    let (_data, root, service) = fixture();
    let operation = window_fixture(&service, root.path());
    let context = Oid::from_str(TRACKING).unwrap();
    let intent = pinned_window(&service, root.path(), operation, Some(context));
    with_transaction(&service, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        let step = IntegrationStepIntent {
            ordinal: 0,
            stage: IntegrationStage::Context,
            local_oid: context,
            incoming_oid: context,
            baseline_tree_oid: context,
            baseline_index_digest: [4; 32],
        };
        prepare_integration_step(tx, &record, &step)?;
        begin_integration_effect(tx, &record, 0, None)?;
        observe_integration_effect(tx, &record, 0, context, context)?;
        assert!(
            prepare_integration_window(tx, &record, 1, &intent).is_err(),
            "frozen primary must finish first"
        );
        prepare_integration_step(
            tx,
            &record,
            &IntegrationStepIntent {
                ordinal: 1,
                stage: IntegrationStage::Primary,
                ..step
            },
        )?;
        begin_integration_effect(tx, &record, 1, None)?;
        observe_integration_effect(tx, &record, 1, context, context)?;
        tx.execute("INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,candidate_oid,checkpoint_oid,phase) VALUES(?1,?2,(SELECT id FROM remote_integration_steps WHERE operation_record_id=?2 AND window_number=0 AND ordinal=1),?3,0,zeroblob(32),zeroblob(32),zeroblob(32),?4,?4,'applied')", params![crate::repository::OperationId::new().to_string(), record.id, record.generation, context.to_string()]).unwrap();
        assert!(
            prepare_integration_window(tx, &record, 1, &intent).is_err(),
            "applied SQL without sentinel release is not completion"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn resolution_candidate_follows_its_step_fk_across_repeated_window_ordinals() {
    let (_data, root, service) = fixture();
    let operation = window_fixture(&service, root.path());
    let intent = pinned_window(&service, root.path(), operation, None);
    with_transaction(&service, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        let original = IntegrationStepIntent {
            ordinal: 0,
            stage: IntegrationStage::Context,
            local_oid: intent.local_oid,
            incoming_oid: intent.local_oid,
            baseline_tree_oid: intent.local_oid,
            baseline_index_digest: [1; 32],
        };
        prepare_integration_step(tx, &record, &original)?;
        begin_integration_effect(tx, &record, 0, None)?;
        observe_integration_effect(tx, &record, 0, intent.local_oid, intent.local_oid)?;
        let primary = IntegrationStepIntent {
            ordinal: 1,
            stage: IntegrationStage::Primary,
            incoming_oid: intent.primary_oid,
            ..original
        };
        prepare_integration_step(tx, &record, &primary)?;
        begin_integration_effect(tx, &record, 1, None)?;
        observe_integration_effect(tx, &record, 1, intent.local_oid, intent.local_oid)?;
        let legacy = integration_step(tx, record.id, 1)?.unwrap();
        prepare_integration_window(tx, &record, 1, &intent)?;
        prepare_integration_step_in_window(tx, &record, 1, &primary)?;
        begin_integration_effect_in_window(tx, &record, 1, 1, None)?;
        record_integration_conflict_in_window(tx, &record, 1, 1, [8; 32])?;
        let attempt = ResolutionAttemptIntent {
            attempt_id: crate::repository::OperationId::new(),
            step_ordinal: 1,
            observation_digest: [8; 32],
            input_digest: [9; 32],
            preflight_digest: [10; 32],
            identity_confirmation_id: None,
        };
        let path = ResolutionPathIntent {
            ordinal: 0,
            path_digest: [11; 32],
            expected_digest: [12; 32],
            result_digest: [13; 32],
            prewrite_digest: [14; 32],
            base_blob_oid: None,
            local_blob_oid: None,
            incoming_blob_oid: None,
            mode: 33188,
        };
        let paths = [path];
        prepare_resolution_attempt_in_window(tx, &record, 1, &attempt, &paths)?;
        assert!(prepare_resolution_attempt(tx, &record, &attempt, &paths).is_err());
        begin_resolution_path_effects(tx, &record, attempt.attempt_id)?;
        observe_resolution_path_effect(tx, &record, attempt.attempt_id, 0)?;
        let candidate = Oid::from_str("3333333333333333333333333333333333333333").unwrap();
        prepare_resolution_candidate(tx, &record, attempt.attempt_id, candidate)?;
        let (bound, observed, _, _, _, _) =
            resolution_candidate_for_attempt(tx, &record, attempt.attempt_id)?.unwrap();
        assert_eq!(bound.window_number, 1);
        assert_eq!(bound.intent.ordinal, 1);
        assert_eq!(observed, candidate);
        assert_eq!(integration_step(tx, record.id, 1)?.unwrap(), legacy);
        audit_merge_evidence(tx, &record)
    })
    .unwrap();
}

#[test]
fn legacy_window_upgrade_refuses_orphans_without_discarding_old_evidence() {
    let (data, _root, _service) = fixture();
    let mut connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    replace_with_legacy_merge_schema(&connection);
    connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO remote_resolution_attempts(id,attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,phase) VALUES(51,'01ARZ3NDEKTSV4RRFFQ69G5FAZ',999,41,0,0,zeroblob(32),zeroblob(32),zeroblob(32),'prepared'); PRAGMA foreign_keys=ON;").unwrap();
    let tx = connection.transaction().unwrap();
    assert!(migrate(&tx).is_err());
    tx.rollback().unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT integration_step_id FROM remote_resolution_attempts WHERE id=51",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        41
    );
    assert!(
        !connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='remote_integration_windows')", [], |row| row.get::<_, bool>(0)).unwrap()
    );
}

#[test]
fn legacy_window_upgrade_sql_fault_rolls_back_rebuilt_tables_and_temp_copies() {
    let (data, _root, _service) = fixture();
    let mut connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    replace_with_legacy_merge_schema(&connection);
    let original: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='remote_integration_steps'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    // An occupied new trigger name fails CREATE after the dependency-ordered
    // rebuild began. The surrounding transaction must restore the old schema.
    connection.execute_batch("PRAGMA foreign_keys=ON; CREATE TRIGGER remote_integration_window_immutable BEFORE UPDATE ON repositories BEGIN SELECT 1; END;").unwrap();
    let tx = connection.transaction().unwrap();
    assert!(migrate(&tx).is_err());
    tx.rollback().unwrap();
    let restored: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='remote_integration_steps'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(restored, original);
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_temp_master WHERE name LIKE '%_window_upgrade'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='remote_integration_windows'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn window_schema_loss_or_weakened_immutability_cannot_be_recreated_as_legacy() {
    for mutation in [
        "DROP TABLE remote_integration_windows",
        "DROP TRIGGER remote_integration_window_immutable; CREATE TRIGGER remote_integration_window_immutable BEFORE UPDATE ON remote_integration_windows BEGIN SELECT 1; END;",
    ] {
        let (data, _root, _service) = fixture();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute_batch(mutation).unwrap();
        assert!(
            matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
        );
    }
}

/// An envelope written before publication attempts existed gains both additive
/// tables empty, with foreign keys intact and no inferred intent, disposition
/// or metadata ownership. A partial or weakened set is evidence loss.
#[test]
fn publication_schema_migration_is_additive_and_fails_closed_on_partial_loss() {
    let (data, root, service) = fixture();
    let operation = window_fixture(&service, root.path());
    drop(service);
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let before: (String, Option<String>, i64) = db
        .query_row(
            "SELECT phase,push_oid,(SELECT count(*) FROM remote_integration_windows) FROM remote_operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    db.execute_batch(
        "DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata;",
    )
    .unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    for table in [
        "remote_publication_attempts",
        "remote_integration_merge_metadata",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(
            !db.prepare(&format!("PRAGMA foreign_key_check({table})"))
                .unwrap()
                .exists([])
                .unwrap()
        );
    }
    assert_eq!(
        db.query_row(
            "SELECT phase,push_oid,(SELECT count(*) FROM remote_integration_windows) FROM remote_operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, i64>(2)?)),
        )
        .unwrap(),
        before
    );
    with_transaction(&reopened, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        assert_eq!(latest_publication_attempt(tx, &record)?, None);
        assert_eq!(integration_merge_metadata(tx, &record, 0, 0)?, None);
        // Without a reconciled legacy Push intent nothing can be appended.
        let oid = Oid::from_str(TRACKING).unwrap();
        assert!(
            open_publication_attempt(
                tx,
                &record,
                oid,
                None,
                PublicationDisposition::NotAccepted,
                oid
            )
            .is_err()
        );
        Ok(())
    })
    .unwrap();
    drop(reopened);
    // The stored names carry only identifiers, OIDs, digests and categories.
    let columns: Vec<String> = db
        .prepare("SELECT name FROM pragma_table_info('remote_publication_attempts') UNION ALL SELECT name FROM pragma_table_info('remote_integration_merge_metadata')")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        columns,
        [
            "operation_record_id",
            "number",
            "configuration_generation",
            "owner_epoch",
            "previous_oid",
            "previous_advertised_oid",
            "previous_disposition",
            "local_oid",
            "window_number",
            "candidate_oid",
            "advertised_oid",
            "phase",
            "intent_recorded",
            "integration_step_id",
            "merge_head_digest",
            "merge_msg_digest",
            "merge_mode_digest",
            "phase",
        ]
    );
    for mutation in [
        "DROP TABLE remote_publication_attempts",
        "DROP TABLE remote_integration_merge_metadata",
        "DROP TRIGGER remote_publication_candidate_immutable",
        "DROP TRIGGER remote_publication_phase_forward; CREATE TRIGGER remote_publication_phase_forward BEFORE UPDATE OF phase ON remote_publication_attempts BEGIN SELECT 1; END;",
        "DROP TRIGGER remote_integration_merge_metadata_immutable",
        "DROP TRIGGER remote_integration_merge_metadata_phase_forward",
        "DROP TRIGGER remote_integration_merge_metadata_phase_forward; CREATE TRIGGER remote_integration_merge_metadata_phase_forward BEFORE UPDATE OF phase ON remote_integration_merge_metadata BEGIN SELECT 1; END;",
    ] {
        let (data, _root, service) = fixture();
        drop(service);
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute_batch(mutation).unwrap();
        assert!(
            matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired),
            "{mutation}"
        );
    }
}

#[test]
fn pinned_window_pass_survives_configuration_fencing_and_registration_cascade() {
    let (data, root, service) = fixture();
    let operation = window_fixture(&service, root.path());
    let intent = pinned_window(&service, root.path(), operation, None);
    let next_plan = RemoteRefPlan::from_configuration("upstream", "trunk").unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        prepare_integration_window(tx, &record, 1, &intent)?;
        let generation = configure(tx, id, Some(&next_plan), false)?;
        // A retained old pass is not live endpoint history and must not cause
        // another generation bump when the new endpoint is first bound.
        assert_eq!(
            configure_endpoints(tx, id, &next_plan, &[3; 32])?,
            generation
        );
        let record = read_operation(tx, id, operation)?.unwrap();
        assert_eq!(record.phase, RemoteOperationPhase::Interrupted);
        assert_eq!(
            integration_window(tx, &record, 1)?.unwrap().intent,
            Some(intent.clone())
        );
        assert!(read_snapshot(tx, id)?.observations().is_empty());
        Ok(())
    })
    .unwrap();
    let mut db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    let tx = db.transaction().unwrap();
    // Removing just the pass cannot cascade away its frozen local intent.
    tx.execute(
        "DELETE FROM remote_observation_batches WHERE id=?1",
        [intent.observation_batch_id],
    )
    .unwrap();
    assert!(tx.commit().is_err());
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        reopened
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .is_empty()
    );
    with_transaction(&reopened, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        assert_eq!(
            integration_window(tx, &record, 1)?.unwrap().intent,
            Some(intent.clone())
        );
        // Exercise the FK cascade used by explicit registration removal, not a
        // Git cleanup. Retained pass evidence must not deadlock the root delete.
        tx.execute("DELETE FROM repositories WHERE id=?1", [id])
            .unwrap();
        assert_eq!(
            tx.query_row(
                "SELECT count(*) FROM remote_integration_windows",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            tx.query_row(
                "SELECT count(*) FROM remote_observation_batches",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        Ok(())
    })
    .unwrap();
}

// P2 source-first regression: deleting an UPDATE-protected marker is still
// possible and its FK cascade can erase otherwise-valid child effect evidence.
fn assert_missing_initial_window_requires_recovery(populated: bool) {
    for context in [false, true] {
        let (data, root, service) = fixture();
        let operation = crate::repository::OperationId::new();
        let target = if context {
            RemoteOperationTarget::for_context(
                &plan(),
                RemoteOperationAction::SynchronizeContext,
                crate::repository::AuthoringKind::Ticket,
                ITEM.parse().unwrap(),
            )
            .unwrap()
        } else {
            RemoteOperationTarget::for_primary_synchronization(&plan())
        };
        with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, Some(&plan()), false)?;
            insert_operation(
                tx,
                id,
                operation,
                &target,
                RemoteOperationPriority::Manual,
                123,
            )?;
            let record = read_operation(tx, id, operation)?.unwrap();
            assert_eq!(
                integration_window(tx, &record, 0)?,
                Some(IntegrationWindowEvidence {
                    number: 0,
                    intent: None,
                })
            );
            if populated {
                let local = Oid::from_str(ADVERTISED).unwrap();
                let incoming = Oid::from_str(TRACKING).unwrap();
                prepare_integration_step(
                    tx,
                    &record,
                    &IntegrationStepIntent {
                        ordinal: 0,
                        stage: if context {
                            IntegrationStage::Context
                        } else {
                            IntegrationStage::Primary
                        },
                        local_oid: local,
                        incoming_oid: incoming,
                        baseline_tree_oid: local,
                        baseline_index_digest: [1; 32],
                    },
                )?;
                begin_integration_effect(tx, &record, 0, None)?;
                record_integration_conflict(tx, &record, 0, [2; 32])?;
                let confirmation = crate::repository::OperationId::new();
                prepare_identity_confirmation(
                    tx,
                    &record,
                    &IdentityConfirmationIntent {
                        confirmation_id: confirmation,
                        input_digest: [3; 32],
                        configuration_digest: [4; 32],
                    },
                )?;
                begin_identity_confirmation_effect(tx, &record, confirmation)?;
                observe_identity_confirmation_effect(tx, &record, confirmation, [5; 32])?;
                let attempt = crate::repository::OperationId::new();
                prepare_resolution_attempt(
                    tx,
                    &record,
                    &ResolutionAttemptIntent {
                        attempt_id: attempt,
                        step_ordinal: 0,
                        observation_digest: [2; 32],
                        input_digest: [6; 32],
                        preflight_digest: [7; 32],
                        identity_confirmation_id: Some(confirmation),
                    },
                    &[ResolutionPathIntent {
                        ordinal: 0,
                        path_digest: [8; 32],
                        expected_digest: [9; 32],
                        result_digest: [10; 32],
                        prewrite_digest: [11; 32],
                        base_blob_oid: Some(local),
                        local_blob_oid: Some(local),
                        incoming_blob_oid: Some(incoming),
                        mode: 33188,
                    }],
                )?;
                begin_resolution_path_effects(tx, &record, attempt)?;
                prepare_resolution_index_artifact(
                    tx,
                    &record,
                    attempt,
                    &ResolutionIndexArtifact {
                        device: 1,
                        inode: 2,
                        sentinel_digest: [12; 32],
                        baseline_digest: [13; 32],
                        baseline_identity: (1, 3),
                        metadata: [Some([14; 32]), Some([15; 32]), Some([16; 32])],
                        output: None,
                        ref_phase: "not_started".into(),
                        phase: "intent".into(),
                    },
                )?;
                advance_resolution_index_artifact(tx, &record, attempt, "published")?;
                prepare_resolution_ref_log_artifact(
                    tx,
                    &record,
                    attempt,
                    "baseline",
                    &ResolutionRefLogArtifact {
                        device: 1,
                        inode: 4,
                        digest: [17; 32],
                    },
                )?;
                observe_resolution_path_effect(tx, &record, attempt, 0)?;
                prepare_resolution_candidate(
                    tx,
                    &record,
                    attempt,
                    Oid::from_str("3333333333333333333333333333333333333333").unwrap(),
                )?;
                prepare_resolution_ref_log_artifact(
                    tx,
                    &record,
                    attempt,
                    "transition",
                    &ResolutionRefLogArtifact {
                        device: 1,
                        inode: 5,
                        digest: [18; 32],
                    },
                )?;
                prepare_resolution_index_output(tx, &record, attempt, (1, 6, [19; 32]))?;
                advance_resolution_ref_effect(tx, &record, attempt, "intent")?;
            }
            Ok(())
        })
        .unwrap();
        // Establish that the entire profile passes startup before corrupting
        // it, including candidate/attempt/path and both native manifest roles.
        RepositoryService::open_at(data.path()).unwrap();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        let rows = |table: &str| {
            let mut query = db
                .prepare(&format!("SELECT * FROM {table} ORDER BY 1"))
                .unwrap();
            let columns = query.column_count();
            query
                .query_map([], |row| {
                    (0..columns)
                        .map(|index| row.get::<_, rusqlite::types::Value>(index))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        let parent = rows("remote_operation_records");
        let confirmations = rows("remote_identity_confirmations");
        for (table, expected) in [
            ("remote_integration_steps", i64::from(populated)),
            ("remote_resolution_attempts", i64::from(populated)),
            ("remote_resolution_paths", i64::from(populated)),
            ("remote_resolution_index_artifacts", i64::from(populated)),
            (
                "remote_resolution_ref_log_artifacts",
                2 * i64::from(populated),
            ),
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                expected
            );
        }
        assert_eq!(
            db.execute("DELETE FROM remote_integration_windows WHERE number=0", [])
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        validate_merge_evidence_schema(&db).unwrap();
        for table in [
            "remote_integration_steps",
            "remote_resolution_attempts",
            "remote_resolution_paths",
            "remote_resolution_index_artifacts",
            "remote_resolution_ref_log_artifacts",
        ] {
            assert!(rows(table).is_empty(), "cascade did not delete {table}");
        }
        assert!(
            matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
        );
        assert!(
            with_transaction(&service, root.path(), |tx, id| read_operation(
                tx, id, operation
            ))
            .is_err()
        );
        assert!(
            matches!(service.reserve_remote_operation(root.path(), operation, &target), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
        );
        assert_eq!(rows("remote_operation_records"), parent);
        assert_eq!(rows("remote_identity_confirmations"), confirmations);
        assert!(
            rows("remote_integration_windows").is_empty(),
            "no legacy marker may be inferred or recreated"
        );
    }
}

#[test]
fn deleted_fresh_sync_window_zero_requires_recovery_without_recreation() {
    assert_missing_initial_window_requires_recovery(false);
}

#[test]
fn deleted_populated_sync_window_zero_requires_recovery_despite_valid_schema_and_fks() {
    assert_missing_initial_window_requires_recovery(true);
}

#[test]
fn window_zero_requirement_excludes_non_sync_operations_and_removed_registrations() {
    for action in [
        RemoteOperationAction::Poll,
        RemoteOperationAction::Promote,
        RemoteOperationAction::Close,
    ] {
        let (data, root, service) = fixture();
        with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, Some(&plan()), false)?;
            let target = if action == RemoteOperationAction::Poll {
                RemoteOperationTarget::for_poll(&plan())
            } else {
                RemoteOperationTarget::for_context(
                    &plan(),
                    action,
                    crate::repository::AuthoringKind::Ticket,
                    ITEM.parse().unwrap(),
                )
                .unwrap()
            };
            let priority = if action == RemoteOperationAction::Poll {
                RemoteOperationPriority::Poll
            } else {
                RemoteOperationPriority::Manual
            };
            insert_operation(
                tx,
                id,
                crate::repository::OperationId::new(),
                &target,
                priority,
                123,
            )?;
            assert_eq!(read_operations(tx, id)?.len(), 1);
            Ok(())
        })
        .unwrap();
        RepositoryService::open_at(data.path()).unwrap();
    }
    let (data, root, service) = fixture();
    window_fixture(&service, root.path());
    with_transaction(&service, root.path(), |tx, id| {
        tx.execute("DELETE FROM repositories WHERE id=?1", [id])
            .unwrap();
        assert_eq!(
            tx.query_row("SELECT count(*) FROM remote_operation_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        Ok(())
    })
    .unwrap();
    RepositoryService::open_at(data.path()).unwrap();
}
