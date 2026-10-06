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
fn orphan_remote_rows_require_recovery_instead_of_disappearing_from_queries() {
    let (data, root, service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO remote_ref_observations VALUES(999,0,NULL,NULL,'malformed','1111111111111111111111111111111111111111',NULL)").unwrap();
    assert_eq!(
        service.remote_snapshot(root.path()).unwrap_err().kind,
        RepositoryErrorKind::RecoveryRequired
    );
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
