use super::*;
use crate::repository::{REGISTRY_FILE, RepositoryService};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const ADVERTISED: &str = "1111111111111111111111111111111111111111";
const TRACKING: &str = "2222222222222222222222222222222222222222";

fn seed_retirement(db: &Connection, kind: AuthoringKind, item: &str) {
    db.execute("INSERT INTO retired_contexts(repository_id,kind,item_id,operation_id,retired_oid) VALUES(1,?1,?2,?3,?4)",params![crate::repository::authoring_kind_segment(&kind),item,crate::repository::OperationId::new().to_string(),TRACKING]).unwrap();
}

#[test]
fn retirement_r1_missing_row_only_overrides_unknown_history_for_that_item() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let (data, root, service) = fixture();
        with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, Some(&plan()), ConfigurationChange::Credentials)
        })
        .unwrap();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute("UPDATE remote_polling_state SET history_unknown=1", [])
            .unwrap();
        seed_retirement(&db, kind, ITEM);
        let snapshot = service.remote_snapshot(root.path()).unwrap();
        assert_eq!(
            snapshot.publication_evidence_for(kind, &ITEM.parse().unwrap()),
            RemotePublicationEvidence::NeverPublished
        );
        assert_eq!(
            snapshot.publication_evidence_for(kind, &"01BX5ZZKBKACTAV9WEVGEMMVRZ".parse().unwrap()),
            RemotePublicationEvidence::HistoryUnknown
        );
        let other_kind = if kind == AuthoringKind::Document {
            AuthoringKind::Ticket
        } else {
            AuthoringKind::Document
        };
        assert_eq!(
            snapshot.publication_evidence_for(other_kind, &ITEM.parse().unwrap()),
            RemotePublicationEvidence::HistoryUnknown
        );
        assert_eq!(
            db.query_row(
                "SELECT history_unknown FROM remote_polling_state",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        RepositoryService::open_at(data.path()).unwrap();
    }
}

#[test]
fn retirement_batch_absence_preserves_retirement_and_publication_consumes_it_atomically() {
    let (data, root, service) = fixture();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Credentials)
    })
    .unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    seed_retirement(&db, AuthoringKind::Ticket, ITEM);
    seed_retirement(&db, AuthoringKind::Ticket, "01BX5ZZKBKACTAV9WEVGEMMVRZ");
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[], 123)
    })
    .unwrap();
    assert!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .contexts()
            .is_empty()
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    db.execute_batch("CREATE TRIGGER fail_retirement_consume BEFORE DELETE ON retired_contexts BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
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
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_context_states", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .is_empty()
    );
    db.execute_batch("DROP TRIGGER fail_retirement_consume")
        .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 124)
    })
    .unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .publication_evidence_for(AuthoringKind::Ticket, &ITEM.parse().unwrap()),
        RemotePublicationEvidence::ObservedPublished
    );
}

#[test]
fn an_explicit_never_published_row_is_accepted_under_the_cache_loss_flag() {
    // Named edit (item resolve-history): the old rule rejected a
    // `never_published` row while the flag was set, which left no way to
    // record an owner's resolution before the last one clears the flag. An
    // explicit row is a creation or owner resolution and is safe; contexts
    // with no row stay unknown under the flag.
    let (data, root, service) = fixture();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Credentials)
    })
    .unwrap();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    seed_retirement(&db, AuthoringKind::Ticket, ITEM);
    let target = plan().context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
    db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,'never_published','history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),ITEM]).unwrap();
    let item: ItemId = ITEM.parse().unwrap();
    assert_eq!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .fetch_publication_evidence_for(AuthoringKind::Ticket, &item),
        RemotePublicationEvidence::NeverPublished
    );
    RepositoryService::open_at(data.path()).unwrap();
}

#[test]
fn retirement_migration_additive_idempotent_atomic_and_audited() {
    let (data, root, service) = fixture();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Credentials)
    })
    .unwrap();
    let mut db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    db.execute_batch("DROP TABLE retired_contexts; DELETE FROM registry_migrations WHERE name='cycle_09_retired_contexts';").unwrap();
    {
        let tx = db.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.rollback().unwrap();
    }
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='retired_contexts'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(service.remote_snapshot(root.path()).unwrap(), before);
    seed_retirement(&db, AuthoringKind::Document, ITEM);
    let version = db
        .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
        .unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        version
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let columns = db
        .prepare("SELECT name FROM pragma_table_info('retired_contexts') ORDER BY cid")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        columns,
        [
            "repository_id",
            "kind",
            "item_id",
            "operation_id",
            "retired_oid"
        ]
    );
    for column in ["kind", "operation_id", "item_id", "retired_oid"] {
        // Startup must audit values even when CHECK constraints were bypassed.
        db.execute_batch("PRAGMA ignore_check_constraints=ON")
            .unwrap();
        db.execute(
            &format!("UPDATE retired_contexts SET {column}='invalid'"),
            [],
        )
        .unwrap();
        assert!(
            matches!(RepositoryService::open_at(data.path()),Err(e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        );
        db.execute("DELETE FROM retired_contexts", []).unwrap();
        seed_retirement(&db, AuthoringKind::Document, ITEM);
    }
}

#[test]
fn retirement_partial_or_lost_schema_is_refused_not_repaired() {
    for case in ["missing", "partial", "orphan"] {
        let (data, _root, _service) = fixture();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        match case {
            "missing"=>db.execute_batch("DROP TABLE retired_contexts").unwrap(),
            "partial"=>db.execute_batch("DROP TABLE retired_contexts; DELETE FROM registry_migrations WHERE name='cycle_09_retired_contexts'; CREATE TABLE retired_contexts(repository_id INTEGER)").unwrap(),
            _=>{db.execute_batch("PRAGMA foreign_keys=OFF").unwrap();db.execute("INSERT INTO retired_contexts VALUES(999,'ticket',?1,?2,?3)",params![ITEM,crate::repository::OperationId::new().to_string(),TRACKING]).unwrap();},
        }
        let version = db
            .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap();
        assert!(
            matches!(RepositoryService::open_at(data.path()),Err(e) if e.kind==RepositoryErrorKind::RecoveryRequired),
            "{case}"
        );
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version
        );
    }
}

#[test]
fn retirement_preserves_pinned_sync_window_pass_when_retiring_current_observation() {
    let (data, root, service) = fixture();
    let repository = git2::Repository::init(root.path()).unwrap();
    let operation = window_fixture(&service, root.path());
    let intent = pinned_window(
        &service,
        root.path(),
        operation,
        Some(TRACKING.parse().unwrap()),
    );
    with_transaction(&service, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        prepare_integration_window(tx, &record, 1, &intent)?;
        Ok(())
    })
    .unwrap();
    let c = crate::repository::cleanup::ContextCleanup::acquire(root.path()).unwrap();
    let evidence = c
        .delete_tracking_ref(
            Some(&plan()),
            AuthoringKind::Ticket,
            &ITEM.parse().unwrap(),
            Some(TRACKING.parse().unwrap()),
            crate::repository::OperationId::new(),
            TRACKING.parse().unwrap(),
        )
        .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        crate::repository::cleanup::retire_context(tx, id, &evidence)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.unwrap();
        assert_eq!(
            integration_window(tx, &record, 1)?.unwrap().intent,
            Some(intent.clone())
        );
        Ok(())
    })
    .unwrap();
    assert_eq!(
        service
            .remote_snapshot(root.path())
            .unwrap()
            .observations()
            .len(),
        1
    );
    RepositoryService::open_at(data.path()).unwrap();
    drop(repository);
}

fn poll_kind_row(db: &Connection) -> Vec<rusqlite::types::Value> {
    db.query_row("SELECT * FROM remote_operation_records", [], |row| {
        (0..row.as_ref().column_count())
            .map(|i| row.get(i))
            .collect()
    })
    .unwrap()
}

#[test]
fn poll_kind_direct_sql_changes_are_denied_and_preserve_the_entire_row_after_reopen() {
    for kind in [Some(PollKind::Observe), Some(PollKind::Update), None] {
        let (data, root, service) = fixture();
        with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
            insert_operation_with_kind(
                tx,
                id,
                crate::repository::OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan()),
                RemoteOperationPriority::Poll,
                kind,
                123,
            )?;
            tx.execute(
                "UPDATE remote_operation_records SET phase='interrupted'",
                [],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
        drop(service);
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        let before = poll_kind_row(&db);
        for _ in 0..2 {
            let reopened = RepositoryService::open_at(data.path()).unwrap();
            for replacement in [Some("observe"), Some("update"), None] {
                assert!(
                    db.execute(
                        "UPDATE remote_operation_records SET poll_kind=?1",
                        [replacement]
                    )
                    .is_err()
                );
                assert_eq!(poll_kind_row(&db), before);
            }
            // Legacy NULL retains observe semantics; no explicit-kind backfill
            // by later writers is allowed to change its durable identity.
            reopened.remote_snapshot(root.path()).unwrap();
            assert_eq!(
                read_operations(&db, 1).unwrap()[0].poll_kind,
                Some(kind.unwrap_or(PollKind::Observe))
            );
        }
    }
}

#[test]
fn poll_kind_legacy_migration_freezes_observe_identity_and_preserves_rows_on_reopen() {
    let (data, root, service) = fixture();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
        insert_operation(
            tx,
            id,
            crate::repository::OperationId::new(),
            &RemoteOperationTarget::for_poll(&plan()),
            RemoteOperationPriority::Poll,
            123,
        )?;
        tx.execute(
            "UPDATE remote_operation_records SET phase='interrupted'",
            [],
        )
        .unwrap();
        Ok(())
    })
    .unwrap();
    drop(service);
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let before = poll_kind_row(&db);
    // Reconstruct the genuinely pre-kind, pre-materialization/disposition
    // envelope, rather than asking startup to repair a recognizable partial one.
    db.execute_batch(
        "DROP TABLE remote_poll_dispositions;
        DROP TRIGGER remote_poll_disposition_owner_immutable;
        ALTER TABLE remote_operation_records DROP COLUMN poll_completed_at;
        ALTER TABLE remote_operation_records DROP COLUMN poll_next_eligible_at;
        DROP TABLE remote_poll_materializations;
        DROP TRIGGER remote_poll_materialization_owner_immutable;
        DROP TRIGGER remote_poll_kind_immutable;
        ALTER TABLE remote_operation_records DROP COLUMN poll_kind;",
    )
    .unwrap();
    // This historical envelope also predates the unpublished migration marker.
    assert_eq!(
        db.execute(
            "DELETE FROM registry_migrations WHERE name='remote_poll_dispositions_unpublished_v1'",
            []
        )
        .unwrap(),
        1
    );
    for _ in 0..2 {
        let reopened = RepositoryService::open_at(data.path()).unwrap();
        assert_eq!(poll_kind_row(&db), before);
        for replacement in [Some("update"), None] {
            assert!(
                db.execute(
                    "UPDATE remote_operation_records SET poll_kind=?1",
                    [replacement]
                )
                .is_err()
            );
            assert_eq!(poll_kind_row(&db), before);
        }
        reopened.remote_snapshot(root.path()).unwrap();
        assert_eq!(
            read_operations(&db, 1).unwrap()[0].poll_kind,
            Some(PollKind::Observe)
        );
    }
}

fn review_completed_state() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    crate::repository::OperationId,
) {
    let (data, root, service) = fixture();
    let op = crate::repository::OperationId::new();
    with_transaction(&service,root.path(),|tx,id|{
        configure(tx,id,Some(&plan()),ConfigurationChange::Plan)?;
        insert_operation_with_kind(tx,id,op,&RemoteOperationTarget::for_poll(&plan()),RemoteOperationPriority::Poll,Some(PollKind::Update),123)?;
        tx.execute("UPDATE remote_operation_records SET phase='completed',completed_step='before_discovery',outcome='completed',updated_at=124,poll_completed_at=124,poll_next_eligible_at=424",[]).unwrap();
        replace_poll_dispositions(tx,id,op,&[super::super::poll::PollTargetReport{target:Some(plan().primary().clone()),kind:None,item_id:None,disposition:super::super::poll::PollDisposition::Current,before_oid:Some(ADVERTISED.parse().unwrap()),after_oid:Some(ADVERTISED.parse().unwrap())}],124)
    }).unwrap();
    (data, root, service, op)
}

#[test]
fn review_disposition_envelope_rejects_lost_column_pair_without_recreation() {
    let (data, _, service, _) = review_completed_state();
    drop(service);
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("ALTER TABLE remote_operation_records DROP COLUMN poll_completed_at;ALTER TABLE remote_operation_records DROP COLUMN poll_next_eligible_at;").unwrap();
    let before = db
        .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
        .unwrap();
    assert!(
        matches!(RepositoryService::open_at(data.path()),Err(e) if e.kind==crate::repository::RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        before
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_poll_dispositions", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        1
    );
}

#[test]
fn review_disposition_envelope_rejects_lost_object_pair_without_recreation() {
    let (data, _, service, op) = review_completed_state();
    drop(service);
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch(
        "DROP TABLE remote_poll_dispositions;DROP TRIGGER remote_poll_disposition_owner_immutable;",
    )
    .unwrap();
    let before = db
        .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
        .unwrap();
    assert!(
        matches!(RepositoryService::open_at(data.path()),Err(e) if e.kind==crate::repository::RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        before
    );
    assert!(
        !db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='remote_poll_dispositions')",
            [],
            |r| r.get::<_, bool>(0)
        )
        .unwrap()
    );
    assert_eq!(db.query_row("SELECT poll_completed_at,poll_next_eligible_at FROM remote_operation_records WHERE operation_ulid=?1",[op.to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?))).unwrap(),(124,424));
}

#[test]
fn review_disposition_metadata_loss_is_not_hidden_by_updated_at_fallback() {
    for latest in [false, true] {
        let (data, _, service, op) = review_completed_state();
        drop(service);
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        if !latest {
            db.execute("DELETE FROM remote_poll_dispositions", [])
                .unwrap();
        }
        db.execute(
            "UPDATE remote_operation_records SET poll_completed_at=NULL",
            [],
        )
        .unwrap();
        assert!(
            poll_completion_metadata(&db, 1, op).is_err(),
            "missing new-row completion must not become updated_at"
        );
        assert!(
            matches!(RepositoryService::open_at(data.path()),Err(e) if e.kind==crate::repository::RepositoryErrorKind::RecoveryRequired)
        );
    }
}

#[test]
fn review_disposition_genuine_old_envelope_migrates_once_preserving_original_rows() {
    let (data, _, service, op) = review_completed_state();
    drop(service);
    let mut db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("DROP TABLE remote_poll_dispositions;DROP TRIGGER remote_poll_disposition_owner_immutable;ALTER TABLE remote_operation_records DROP COLUMN poll_completed_at;ALTER TABLE remote_operation_records DROP COLUMN poll_next_eligible_at;").unwrap();
    // The current fixture has a marker. Losing the entire envelope while that
    // marker survives is evidence loss, not a genuine pre-envelope registry.
    let original_version: i64 = db
        .query_row("PRAGMA schema_version", [], |r| r.get(0))
        .unwrap();
    let tx = db.transaction().unwrap();
    assert!(migrate_poll_dispositions(&tx).is_err());
    assert_eq!(
        tx.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        original_version,
        "the marked form must be refused before DDL"
    );
    tx.rollback().unwrap();
    assert!(
        matches!(RepositoryService::open_at(data.path()), Err(e) if e.kind == crate::repository::RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        original_version
    );
    assert_eq!(
        db.execute(
            "DELETE FROM registry_migrations WHERE name='remote_poll_dispositions_unpublished_v1'",
            []
        )
        .unwrap(),
        1
    );
    let before = db
        .query_row(
            "SELECT phase,outcome,created_at,updated_at FROM remote_operation_records",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            },
        )
        .unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT phase,outcome,created_at,updated_at FROM remote_operation_records",
            [],
            |r| Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?
            ))
        )
        .unwrap(),
        before
    );
    assert_eq!(
        poll_completion_metadata(&db, 1, op).unwrap(),
        (
            OffsetDateTime::from_unix_timestamp(124).unwrap(),
            None,
            false
        )
    );
    let version = db
        .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
        .unwrap();
    drop(service);
    assert!(RepositoryService::open_at(data.path()).is_ok());
    assert_eq!(
        db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        version
    );
}

// Frozen from the common pre-retirement/pre-unpublished catalog, not the current
// builder. All four published profiles share this table/trigger shape; only the
// independently named literal enum additions differ.
const LEGACY_POLL_DISPOSITIONS: &str = r#"
    CREATE TABLE remote_poll_dispositions (
        repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
        operation_ulid TEXT NOT NULL,
        ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
        classification TEXT NOT NULL CHECK(classification IN ('primary','context','malformed')),
        remote_ref TEXT,
        kind TEXT CHECK(kind IN ('document','ticket')),
        item_id TEXT,
        disposition TEXT NOT NULL CHECK(disposition IN ('current','fast_forwarded','local_ahead','diverged','dirty','conflicted','inaccessible','materialized','nonconforming','blocked_by_local_state','materialization_incomplete','malformed','remotely_deleted','history_unknown','not_reached')),
        before_oid TEXT CHECK(length(before_oid)=40 AND before_oid NOT GLOB '*[^0-9a-f]*'),
        after_oid TEXT CHECK(length(after_oid)=40 AND after_oid NOT GLOB '*[^0-9a-f]*'),
        observed_at INTEGER NOT NULL CHECK(typeof(observed_at)='integer' AND observed_at BETWEEN 0 AND 253402300799),
        PRIMARY KEY(repository_id,ordinal), UNIQUE(repository_id,remote_ref),
        FOREIGN KEY(repository_id,operation_ulid) REFERENCES remote_operation_records(repository_id,operation_ulid) ON DELETE CASCADE,
        CHECK((classification='context')=(kind IS NOT NULL)),
        CHECK((kind IS NULL)=(item_id IS NULL)),
        CHECK((classification='malformed')=(remote_ref IS NULL))
    );
    CREATE TRIGGER remote_poll_disposition_owner_immutable BEFORE UPDATE OF repository_id,operation_ulid,action,poll_kind ON remote_operation_records WHEN EXISTS(SELECT 1 FROM remote_poll_dispositions WHERE repository_id=OLD.repository_id AND operation_ulid=OLD.operation_ulid) BEGIN SELECT RAISE(ABORT,'immutable disposition owner'); END;
"#;

fn historical_disposition_schema(profile: u8) -> String {
    assert!([15, 16, 17, 18].contains(&profile));
    let mut schema = LEGACY_POLL_DISPOSITIONS.to_owned();
    if matches!(profile, 17 | 18) {
        schema = schema.replace(
            "'remotely_deleted',",
            "'remotely_deleted','closed','promoted',",
        );
    }
    if matches!(profile, 16 | 18) {
        schema = schema.replace("'history_unknown',", "'history_unknown','unpublished',");
    }
    schema
}

fn install_historical_dispositions(db: &Connection, profile: u8) {
    db.execute_batch("DROP TRIGGER remote_poll_disposition_owner_immutable; ALTER TABLE remote_poll_dispositions RENAME TO old_dispositions;").unwrap();
    db.execute_batch(&historical_disposition_schema(profile))
        .unwrap();
    db.execute_batch("INSERT INTO remote_poll_dispositions SELECT * FROM old_dispositions; DROP TABLE old_dispositions;").unwrap();
    db.execute(
        "DELETE FROM registry_migrations WHERE name='remote_poll_dispositions_unpublished_v1'",
        [],
    )
    .unwrap();
    if matches!(profile, 16 | 18) {
        db.execute("INSERT INTO registry_migrations(name) VALUES('remote_poll_dispositions_unpublished_v1')", []).unwrap();
    }
}

#[test]
fn retirement_disposition_exact_old_enum_upgrade_preserves_evidence_and_owner() {
    let legacy = [
        "current",
        "fast_forwarded",
        "local_ahead",
        "diverged",
        "dirty",
        "conflicted",
        "inaccessible",
        "materialized",
        "nonconforming",
        "blocked_by_local_state",
        "materialization_incomplete",
        "malformed",
        "remotely_deleted",
        "history_unknown",
        "not_reached",
    ];
    for profile in [15, 16, 17, 18] {
        let (data, _, service, op) = review_completed_state();
        drop(service);
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        let mut values = legacy.to_vec();
        if matches!(profile, 17 | 18) {
            values.extend(["closed", "promoted"]);
        }
        if matches!(profile, 16 | 18) {
            values.push("unpublished");
        }
        for (ordinal, value) in values.iter().enumerate() {
            db.execute("INSERT OR REPLACE INTO remote_poll_dispositions(repository_id,operation_ulid,ordinal,classification,disposition,before_oid,after_oid,observed_at) VALUES(1,?1,?2,'malformed',?3,?4,?4,124)",params![op.to_string(),ordinal as i64,value,ADVERTISED]).unwrap();
        }
        let rows_before = read_poll_dispositions(&db, 1).unwrap();
        install_historical_dispositions(&db, profile);
        let service = RepositoryService::open_at(data.path())
            .unwrap_or_else(|_| panic!("valid historical profile {profile} must upgrade"));
        validate_poll_disposition_schema(&db).unwrap();
        assert_eq!(read_poll_dispositions(&db, 1).unwrap(), rows_before);
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_poll_dispositions", [], |r| {
                r.get::<_, usize>(0)
            })
            .unwrap(),
            values.len()
        );
        assert_eq!(
            poll_completion_metadata(&db, 1, op)
                .unwrap()
                .0
                .unix_timestamp(),
            124
        );
        assert!(
            db.execute(
                "UPDATE remote_operation_records SET operation_ulid=operation_ulid",
                []
            )
            .is_err()
        );
        let version: i64 = db
            .query_row("PRAGMA schema_version", [], |r| r.get(0))
            .unwrap();
        drop(service);
        RepositoryService::open_at(data.path()).unwrap();
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version
        );
        for (ordinal, value) in ["closed", "promoted", "unpublished"].iter().enumerate() {
            db.execute(
                "UPDATE remote_poll_dispositions SET disposition=?1 WHERE ordinal=?2",
                params![value, ordinal as i64],
            )
            .unwrap();
        }
        let rows = read_poll_dispositions(&db, 1).unwrap();
        assert_eq!(
            rows[0].disposition(),
            super::super::poll::PollDisposition::Closed
        );
        assert_eq!(
            rows[1].disposition(),
            super::super::poll::PollDisposition::Promoted
        );
        assert_eq!(
            rows[2].disposition(),
            super::super::poll::PollDisposition::Unpublished
        );
        RepositoryService::open_at(data.path()).unwrap();
        assert_eq!(read_poll_dispositions(&db, 1).unwrap(), rows);
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version
        );
    }
}

#[test]
fn retirement_disposition_old_corrupt_and_partial_envelopes_roll_back() {
    let corruptions = [
        "DROP TRIGGER remote_poll_disposition_owner_immutable",
        "DROP TRIGGER remote_poll_disposition_owner_immutable; CREATE TRIGGER remote_poll_disposition_owner_immutable BEFORE UPDATE ON remote_operation_records BEGIN SELECT 1; END;",
        "ALTER TABLE remote_poll_dispositions ADD COLUMN substituted TEXT",
        "ALTER TABLE remote_operation_records DROP COLUMN poll_completed_at",
        "ALTER TABLE remote_operation_records DROP COLUMN poll_next_eligible_at",
        "ALTER TABLE remote_operation_records DROP COLUMN poll_completed_at; ALTER TABLE remote_operation_records ADD COLUMN poll_completed_at TEXT",
        "PRAGMA writable_schema=ON; UPDATE sqlite_master SET sql=replace(sql,'''remotely_deleted''','''REMOTELY_DELETED''') WHERE name='remote_poll_dispositions'; PRAGMA writable_schema=OFF",
        "PRAGMA ignore_check_constraints=ON; UPDATE remote_poll_dispositions SET disposition='closed'",
        "PRAGMA ignore_check_constraints=ON; UPDATE remote_poll_dispositions SET disposition='promoted'",
        "PRAGMA ignore_check_constraints=ON; UPDATE remote_poll_dispositions SET disposition='substituted'",
    ];
    let dependencies = [
        "CREATE INDEX unknown_index ON \"REMOTE_POLL_DISPOSITIONS\"(disposition)",
        "CREATE TRIGGER unknown_attached AFTER UPDATE ON [Remote_Poll_Dispositions] BEGIN SELECT 1; END",
        "CREATE TRIGGER unknown_external AFTER INSERT ON retention_payload BEGIN SELECT count(*) FROM `REMOTE_POLL_DISPOSITIONS`; END",
        "CREATE VIEW unknown_view AS SELECT * FROM \"Remote_Poll_Dispositions\"",
        "CREATE TABLE incoming_dispositions(repository_id INTEGER, ordinal INTEGER, payload TEXT, FOREIGN KEY(repository_id,ordinal) REFERENCES remote_poll_dispositions(repository_id,ordinal) ON DELETE CASCADE)",
        "CREATE TABLE incoming_dispositions(repository_id INTEGER, ordinal INTEGER, payload TEXT, FOREIGN KEY(repository_id,ordinal) REFERENCES \"REMOTE_POLL_DISPOSITIONS\"(repository_id,ordinal) ON DELETE CASCADE); INSERT INTO incoming_dispositions SELECT repository_id,ordinal,'retained cascade payload' FROM remote_poll_dispositions",
        "CREATE TABLE incoming_dispositions(repository_id INTEGER, ordinal INTEGER, payload TEXT, FOREIGN KEY(repository_id,ordinal) REFERENCES [Remote_Poll_Dispositions](repository_id,ordinal) ON DELETE RESTRICT)",
        "CREATE TABLE incoming_dispositions(repository_id INTEGER, ordinal INTEGER, payload TEXT, FOREIGN KEY(repository_id,ordinal) REFERENCES `Remote_Poll_Dispositions`(repository_id,ordinal) ON DELETE RESTRICT); INSERT INTO incoming_dispositions SELECT repository_id,ordinal,'retained restrict payload' FROM remote_poll_dispositions",
        "CREATE TABLE incoming_dispositions(repository_id INTEGER, ordinal INTEGER, payload TEXT, FOREIGN KEY(repository_id,ordinal) REFERENCES 'REMOTE_POLL_DISPOSITIONS'(repository_id,ordinal) ON DELETE SET NULL); INSERT INTO incoming_dispositions SELECT repository_id,ordinal,'retained set-null payload' FROM remote_poll_dispositions",
    ];
    let cases = corruptions.into_iter().map(|change| (15, change))
        .chain(dependencies.into_iter().flat_map(|change| [15,16,17,18].map(|profile| (profile,change))))
        .chain([
            (16, corruptions[7]),
            (16, corruptions[8]),
            (17, "PRAGMA ignore_check_constraints=ON; UPDATE remote_poll_dispositions SET disposition='unpublished'"),
            (15, "INSERT INTO registry_migrations(name) VALUES('remote_poll_dispositions_unpublished_v1')"),
            (17, "INSERT INTO registry_migrations(name) VALUES('remote_poll_dispositions_unpublished_v1')"),
            (16, "DELETE FROM registry_migrations WHERE name='remote_poll_dispositions_unpublished_v1'"),
            (18, "DELETE FROM registry_migrations WHERE name='remote_poll_dispositions_unpublished_v1'"),
        ]);
    let mut accepted = Vec::new();
    for (profile, change) in cases {
        let case = format!("profile={profile}: {change}");
        let (data, _, service, _) = review_completed_state();
        drop(service);
        let mut db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        // Match the real service connection, including CASCADE effects. The
        // unknown populated child must survive public open, not just our rollback.
        db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        if profile != 18 {
            install_historical_dispositions(&db, profile);
        }
        db.execute_batch("CREATE TABLE retention_payload(payload TEXT NOT NULL); INSERT INTO retention_payload(rowid,payload) VALUES(91,'retained independent payload')").unwrap();
        db.execute_batch(change).unwrap();
        // Match a newly opened registry connection: retain the corrupt row,
        // but do not carry the fixture writer's CHECK bypass into migration.
        db.execute_batch("PRAGMA ignore_check_constraints=OFF")
            .unwrap();
        let corrupt_enum = change.contains("ignore_check_constraints");
        if corrupt_enum {
            let integrity: String = db
                .query_row(
                    "PRAGMA integrity_check(remote_poll_dispositions)",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(
                integrity.contains("CHECK constraint failed"),
                "{change}: {integrity}"
            );
        }
        let values = |db: &Connection, sql: &str| {
            let mut query = db.prepare(sql).unwrap();
            let columns = query.column_count();
            query
                .query_map([], |r| {
                    (0..columns)
                        .map(|column| r.get::<_, rusqlite::types::Value>(column))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        let image = |db: &Connection| {
            let tables = db
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap()
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let mut image = vec![values(
                db,
                "SELECT type,name,tbl_name,rootpage,sql FROM sqlite_master ORDER BY name",
            )];
            for table in tables {
                image.push(values(
                    db,
                    &format!(
                        "SELECT rowid,* FROM \"{}\" ORDER BY rowid",
                        table.replace('"', "\"\"")
                    ),
                ));
            }
            image
        };
        let before = image(&db);
        assert!(!values(&db, "SELECT * FROM remote_poll_dispositions").is_empty());
        let version: i64 = db
            .query_row("PRAGMA schema_version", [], |r| r.get(0))
            .unwrap();
        let tx = db.transaction().unwrap();
        let refused = migrate_poll_dispositions(&tx).is_err();
        {
            // The closure guard refuses BEFORE any DDL, not after a destructive
            // rename/drop that happens to be hidden by the explicit rollback.
            assert_eq!(image(&tx), before, "before rollback: {case}");
            assert_eq!(
                tx.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                version,
                "before rollback: {case}"
            );
        }
        tx.rollback().unwrap();
        assert_eq!(image(&db), before, "{case}");
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version,
            "{case}"
        );
        if corrupt_enum {
            let owner_update = db.execute(
                "UPDATE remote_operation_records SET operation_ulid=operation_ulid",
                [],
            );
            assert!(
                matches!(owner_update, Err(rusqlite::Error::SqliteFailure(_, Some(ref message))) if message == "immutable disposition owner"),
                "{change}: {owner_update:?}"
            );
            assert_eq!(image(&db), before, "{case}");
        }
        if !refused {
            accepted.push(case.clone());
        }
        let opened = RepositoryService::open_at(data.path()).map(|_| ());
        assert!(
            matches!(opened, Err(ref error) if error.kind == RepositoryErrorKind::RecoveryRequired),
            "public open must refuse {case}: {opened:?}"
        );
        assert_eq!(image(&db), before, "public open: {case}");
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version,
            "public open: {case}"
        );
    }
    assert!(
        accepted.is_empty(),
        "old-schema corruption unexpectedly accepted: {accepted:?}"
    );
}

#[test]
fn review_disposition_every_recognizable_partial_envelope_is_retained() {
    for retained in 1..15 {
        let (data, _, service, _) = review_completed_state();
        drop(service);
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        // Alter columns while schema references are still valid, then remove
        // objects. The retained bitmask is table/trigger/completion/deadline.
        if retained & 4 == 0 {
            db.execute_batch("ALTER TABLE remote_operation_records DROP COLUMN poll_completed_at")
                .unwrap();
        }
        if retained & 8 == 0 {
            db.execute_batch(
                "ALTER TABLE remote_operation_records DROP COLUMN poll_next_eligible_at",
            )
            .unwrap();
        }
        if retained & 2 == 0 {
            db.execute_batch("DROP TRIGGER remote_poll_disposition_owner_immutable")
                .unwrap();
        }
        if retained & 1 == 0 {
            db.execute_batch("DROP TABLE remote_poll_dispositions")
                .unwrap();
        }
        let before = db
            .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap();
        assert!(
            matches!(RepositoryService::open_at(data.path()),Err(e) if e.kind==crate::repository::RepositoryErrorKind::RecoveryRequired),
            "retained mask {retained}"
        );
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            before,
            "partial schema rewritten for {retained}"
        );
        assert_eq!(
            db.query_row(
                "SELECT phase,outcome,updated_at FROM remote_operation_records",
                [],
                |r| Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?
                ))
            )
            .unwrap(),
            ("completed".into(), "completed".into(), 124)
        );
    }
}

#[test]
fn task6_local_history_unknown_context_also_exposes_its_latest_disposition() {
    let (_data, root, service) = fixture();
    with_transaction(&service,root.path(),|tx,id|{
        configure(tx,id,Some(&plan()),ConfigurationChange::Plan)?;
        tx.execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(?1,'active','manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV','/local-context','01ARZ3NDEKTSV4RRFFQ69G5FAV')",[id]).unwrap();
        let op=crate::repository::OperationId::new();
        insert_operation_with_kind(tx,id,op,&RemoteOperationTarget::for_poll(&plan()),RemoteOperationPriority::Poll,Some(PollKind::Update),123)?;
        replace_poll_dispositions(tx,id,op,&[super::super::poll::PollTargetReport{target:Some(plan().context(AuthoringKind::Ticket,&ITEM.parse().unwrap())),kind:Some(AuthoringKind::Ticket),item_id:Some(ITEM.parse().unwrap()),disposition:super::super::poll::PollDisposition::HistoryUnknown,before_oid:None,after_oid:None}],123)
    }).unwrap();
    let snapshot = with_transaction(&service, root.path(), |tx, id| read_snapshot(tx, id)).unwrap();
    assert_eq!(
        snapshot.contexts()[0].latest_disposition(),
        Some(super::super::poll::PollDisposition::HistoryUnknown)
    );
}

#[test]
fn task6_disposition_schema_upgrade_rollback_corruption_and_privacy() {
    for change in [
        "ALTER TABLE remote_poll_dispositions ADD COLUMN body TEXT",
        "DROP TRIGGER remote_poll_disposition_owner_immutable",
        "PRAGMA ignore_check_constraints=ON; UPDATE remote_operation_records SET poll_completed_at=-1 WHERE poll_kind='update'",
    ] {
        let (data, root, service) = fixture();
        with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
            let op = crate::repository::OperationId::new();
            insert_operation_with_kind(
                tx,
                id,
                op,
                &RemoteOperationTarget::for_poll(&plan()),
                RemoteOperationPriority::Poll,
                Some(PollKind::Update),
                123,
            )?;
            replace_poll_dispositions(
                tx,
                id,
                op,
                &[super::super::poll::PollTargetReport {
                    target: None,
                    kind: None,
                    item_id: None,
                    disposition: super::super::poll::PollDisposition::Malformed,
                    before_oid: None,
                    after_oid: Some(ADVERTISED.parse().unwrap()),
                }],
                123,
            )
        })
        .unwrap();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute_batch(change).unwrap();
        assert!(RepositoryService::open_at(data.path()).is_err(), "{change}");
    }
    let (data, _, service) = fixture();
    drop(service);
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("DROP TABLE remote_poll_dispositions;DROP TRIGGER remote_poll_disposition_owner_immutable;ALTER TABLE remote_operation_records DROP COLUMN poll_completed_at;").unwrap();
    assert!(RepositoryService::open_at(data.path()).is_err());
    assert!(
        !db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='remote_poll_dispositions')",
            [],
            |r| r.get::<_, bool>(0)
        )
        .unwrap()
    );
}

#[test]
fn task6_disposition_schema_is_empty_atomic_metadata_only() {
    let (data, root, service) = fixture();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let cols: Vec<String> = db
        .prepare("PRAGMA table_info(remote_poll_dispositions)")
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        cols,
        [
            "repository_id",
            "operation_ulid",
            "ordinal",
            "classification",
            "remote_ref",
            "kind",
            "item_id",
            "disposition",
            "before_oid",
            "after_oid",
            "observed_at"
        ]
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_poll_dispositions", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    let before = service.remote_snapshot(root.path()).unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(reopened.remote_snapshot(root.path()).unwrap(), before);
}

#[test]
fn task6_partial_disposition_schema_is_not_recreated() {
    let (data, _, service) = fixture();
    drop(service);
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("DROP TABLE remote_poll_dispositions")
        .unwrap();
    let error = match RepositoryService::open_at(data.path()) {
        Err(e) => e,
        Ok(_) => panic!("partial schema adopted"),
    };
    assert_eq!(
        error.kind,
        crate::repository::RepositoryErrorKind::RecoveryRequired
    );
}

#[test]
fn poll_materialization_schema_migrates_empty_and_is_idempotent() {
    let (data, root, service) = fixture();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let columns: Vec<String> = db
        .prepare("PRAGMA table_info(remote_poll_materializations)")
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        columns,
        [
            "repository_id",
            "kind",
            "item_id",
            "operation_ulid",
            "target_oid",
            "step"
        ]
    );
    let before = service.remote_snapshot(root.path()).unwrap();
    db.execute_batch("DROP TRIGGER remote_poll_materialization_immutable; DROP TRIGGER remote_poll_materialization_forward; DROP TRIGGER remote_poll_materialization_owner_immutable; DROP TABLE remote_poll_materializations;").unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(reopened.remote_snapshot(root.path()).unwrap(), before);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_poll_materializations",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let version: i64 = db
        .query_row("PRAGMA schema_version", [], |r| r.get(0))
        .unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        version
    );
}

#[test]
fn poll_materialization_partial_or_altered_schema_fails_without_repair() {
    for sql in [
        "DROP TRIGGER remote_poll_materialization_immutable",
        "DROP TRIGGER remote_poll_materialization_forward",
        "DROP TRIGGER remote_poll_materialization_owner_immutable",
        "DROP TABLE remote_poll_materializations",
        "DROP TRIGGER remote_poll_materialization_forward; CREATE TRIGGER remote_poll_materialization_forward BEFORE UPDATE ON remote_poll_materializations BEGIN SELECT 1; END;",
        "ALTER TABLE remote_poll_materializations ADD COLUMN body TEXT",
    ] {
        let (data, _, _) = fixture();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute_batch(sql).unwrap();
        let version: i64 = db
            .query_row("PRAGMA schema_version", [], |r| r.get(0))
            .unwrap();
        assert!(
            matches!(RepositoryService::open_at(data.path()), Err(e) if e.kind == RepositoryErrorKind::RecoveryRequired),
            "{sql}"
        );
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version
        );
    }
}

fn materialization_state_fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    crate::repository::OperationId,
) {
    let (data, root, service) = fixture();
    let operation = crate::repository::OperationId::new();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
        insert_operation_with_kind(
            tx,
            id,
            operation,
            &RemoteOperationTarget::for_poll(&plan()),
            RemoteOperationPriority::Poll,
            Some(PollKind::Update),
            123,
        )?;
        insert_materialization(
            tx,
            id,
            AuthoringKind::Document,
            &ITEM.parse().unwrap(),
            operation,
            Oid::from_str(ADVERTISED).unwrap(),
        )
    })
    .unwrap();
    (data, root, service, operation)
}

#[test]
fn poll_materialization_identity_constraints_and_cascade_preserve_redacted_evidence() {
    let (data, root, service, operation) = materialization_state_fixture();
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    for column in [
        "repository_id",
        "kind",
        "item_id",
        "operation_ulid",
        "target_oid",
    ] {
        assert!(
            db.execute_batch(&format!(
                "UPDATE remote_poll_materializations SET {column}={column}"
            ))
            .is_err(),
            "{column}"
        );
    }
    for value in [
        "'body sentinel'",
        "'MATERIALIZATION_PRIVATE_SENTINEL'",
        "x'ff'",
        "NULL",
    ] {
        assert!(
            db.execute_batch(&format!(
                "UPDATE remote_poll_materializations SET step={value}"
            ))
            .is_err()
        );
    }
    with_transaction(&service, root.path(), |tx, id| {
        let intent = materialization_intents(tx, id, &ITEM.parse().unwrap())?.remove(0);
        assert_eq!(intent.operation_id, operation);
        advance_materialization(tx, id, &intent, MaterializationStep::Complete)
    })
    .unwrap();
    assert!(
        db.execute_batch("UPDATE remote_poll_materializations SET step='branch_created'")
            .is_err()
    );
    // Only the explicitly Git-first restart step may go back to intended.
    db.execute_batch("UPDATE remote_poll_materializations SET step='intended'")
        .unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    let backup = data.path().join("materialization-backup.sqlite3");
    db.execute("VACUUM INTO ?1", [backup.to_str().unwrap()])
        .unwrap();
    for path in [
        data.path().join(REGISTRY_FILE),
        data.path().join(format!("{REGISTRY_FILE}-wal")),
        backup,
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            assert!(
                !bytes
                    .windows(b"MATERIALIZATION_PRIVATE_SENTINEL".len())
                    .any(|w| w == b"MATERIALIZATION_PRIVATE_SENTINEL")
            );
        }
    }
    db.execute_batch("DELETE FROM repositories").unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_poll_materializations",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    RepositoryService::open_at(data.path()).unwrap();
}

#[test]
fn poll_materialization_startup_audits_corrupt_ids_oids_and_nonpoll_owners() {
    for column in [
        "kind",
        "item_id",
        "operation_ulid",
        "target_oid",
        "step",
        "owner_kind",
    ] {
        let (data, _, _, _) = materialization_state_fixture();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        // External SQL corruption bypasses normal constraints; startup must not
        // repair it or print the supplied contents.
        db.execute_batch("PRAGMA foreign_keys=OFF; PRAGMA ignore_check_constraints=ON; DROP TRIGGER remote_poll_materialization_immutable;").unwrap();
        if column == "owner_kind" {
            db.execute_batch("DROP TRIGGER remote_poll_kind_immutable; DROP TRIGGER remote_poll_materialization_owner_immutable; UPDATE remote_operation_records SET poll_kind='observe';").unwrap();
            db.execute_batch(
                schema_object_sql(
                    POLL_MATERIALIZATION_SCHEMA,
                    "TRIGGER",
                    "remote_poll_materialization_owner_immutable",
                )
                .unwrap(),
            )
            .unwrap();
            db.execute_batch("CREATE TRIGGER remote_poll_kind_immutable BEFORE UPDATE OF poll_kind ON remote_operation_records BEGIN SELECT RAISE(ABORT,'immutable poll kind'); END;").unwrap();
        } else {
            db.execute_batch(&format!(
                "UPDATE remote_poll_materializations SET {column}='CORRUPT_PRIVATE_SENTINEL'"
            ))
            .unwrap();
        }
        db.execute_batch(
            schema_object_sql(
                POLL_MATERIALIZATION_SCHEMA,
                "TRIGGER",
                "remote_poll_materialization_immutable",
            )
            .unwrap(),
        )
        .unwrap();
        let version: i64 = db
            .query_row("PRAGMA schema_version", [], |r| r.get(0))
            .unwrap();
        let error = match RepositoryService::open_at(data.path()) {
            Err(e) => e,
            Ok(_) => panic!("corruption accepted: {column}"),
        };
        assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
        assert!(!format!("{error:?}").contains("CORRUPT_PRIVATE_SENTINEL"));
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version
        );
    }
}

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    fixture_in(&std::env::temp_dir())
}

fn remove_attempt_columns(connection: &Connection) {
    let columns = connection
        .prepare("PRAGMA table_info(remote_polling_state)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for column in ["latest_attempt_at", "next_eligible_at"] {
        if columns.iter().any(|name| name == column) {
            connection
                .execute_batch(&format!(
                    "ALTER TABLE remote_polling_state DROP COLUMN {column}"
                ))
                .unwrap();
        }
    }
}

#[test]
fn attempt_migration_preserves_existing_rows_and_is_a_no_op_on_second_start() {
    let (data, root, service) = fixture();
    let generation = with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    service
        .set_remote_polling(
            root.path(),
            false,
            true,
            PollingInterval::from_seconds(600).unwrap(),
        )
        .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("UPDATE remote_polling_state SET automatic_backoff_seconds=120; INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES('/second-registration',124,'inaccessible',0)").unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    remove_attempt_columns(&connection);
    let schema_version: i64 = connection
        .query_row("PRAGMA schema_version", [], |row| row.get(0))
        .unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(reopened.remote_snapshot(root.path()).unwrap(), before);
    let times: Vec<(Option<i64>, Option<i64>)> = connection.prepare("SELECT latest_attempt_at,next_eligible_at FROM remote_polling_state ORDER BY repository_id").unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(times, vec![(None, None), (None, None)]);
    let upgraded_version: i64 = connection
        .query_row("PRAGMA schema_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(upgraded_version, schema_version + 2);
    let _again = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA schema_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        upgraded_version
    );
    let mut audit = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    audit_registry(&mut audit).unwrap();
}

#[test]
fn half_migrated_attempt_columns_require_recovery_without_repair() {
    for column in ["latest_attempt_at", "next_eligible_at"] {
        let (data, _root, _service) = fixture();
        let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        remove_attempt_columns(&connection);
        connection
            .execute_batch(&format!(
                "ALTER TABLE remote_polling_state ADD COLUMN {column} INTEGER"
            ))
            .unwrap();
        let version: i64 = connection
            .query_row("PRAGMA schema_version", [], |row| row.get(0))
            .unwrap();
        assert!(
            matches!(RepositoryService::open_at(data.path()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
        );
        assert_eq!(
            connection
                .query_row("PRAGMA schema_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            version
        );
    }
}

#[test]
fn attempt_columns_reject_non_integer_negative_and_unrenderable_timestamps() {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    for column in ["latest_attempt_at", "next_eligible_at"] {
        for value in [
            "-1",
            "253402300800",
            "1.5",
            "'PRIVATE_RESPONSE_SENTINEL'",
            "x'ff'",
        ] {
            assert!(
                connection
                    .execute_batch(&format!("UPDATE remote_polling_state SET {column}={value}"))
                    .is_err()
            );
        }
        connection
            .execute_batch(&format!(
                "UPDATE remote_polling_state SET {column}=253402300799"
            ))
            .unwrap();
    }
}

#[test]
fn finishing_attempt_timestamps_commit_and_roll_back_with_the_outcome() {
    let (_data, root, service) = fixture();
    let at = |seconds| time::OffsetDateTime::from_unix_timestamp(seconds).unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        record_outcome(tx, id, RemoteOutcomeCategory::Completed, None)?;
        record_poll_attempt(tx, id, at(1000), Some(at(1300)))
    })
    .unwrap();
    let before = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(before.latest_attempt_at(), Some(at(1000)));
    assert_eq!(before.next_eligible_at(), Some(at(1300)));
    let result: Result<(), RepositoryError> = with_transaction(&service, root.path(), |tx, id| {
        record_outcome(
            tx,
            id,
            RemoteOutcomeCategory::TransportUnavailable,
            Some(AutomaticBackoff::from_seconds(60).unwrap()),
        )?;
        record_poll_attempt(tx, id, at(1100), Some(at(1160)))?;
        Err(recovery_required())
    });
    assert!(result.is_err());
    assert_eq!(service.remote_snapshot(root.path()).unwrap(), before);
    with_transaction(&service, root.path(), |tx, id| {
        record_outcome(tx, id, RemoteOutcomeCategory::TransportUnavailable, None)?;
        record_poll_attempt(tx, id, at(1200), None)
    })
    .unwrap();
    let after = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(after.latest_attempt_at(), Some(at(1200)));
    assert_eq!(after.next_eligible_at(), Some(at(1300)));
    assert_eq!(
        after.latest_outcome(),
        Some(RemoteOutcomeCategory::TransportUnavailable)
    );
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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

/// The attempt shape before its commit time was recorded.
fn legacy_commit_time_schema() -> String {
    legacy_window_fixture_schema()
        .replace(
            "            commit_time INTEGER NOT NULL CHECK(typeof(commit_time)='integer' AND commit_time > 0),\n",
            "",
        )
        .replace(
            "            commit_offset_minutes INTEGER NOT NULL CHECK(typeof(commit_offset_minutes)='integer' AND commit_offset_minutes BETWEEN -1439 AND 1439),\n",
            "",
        )
        .replace(
            "identity_confirmation_id,commit_time,commit_offset_minutes ON",
            "identity_confirmation_id ON",
        )
}

fn legacy_preflight_schema() -> String {
    legacy_commit_time_schema()
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
            configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
            configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
            configure(tx, id, next.as_ref(), ConfigurationChange::Credentials)
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, None, ConfigurationChange::Credentials)
    })
    .unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Credentials)
    })
    .unwrap();
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(
        snapshot.publication_evidence_for(AuthoringKind::Ticket, &ITEM.parse().unwrap()),
        RemotePublicationEvidence::ObservedPublished
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
    })
    .unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE remote_poll_dispositions; DROP TABLE remote_poll_materializations; DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows; DROP TABLE remote_operation_records")
        .unwrap();
    // This is an authentic pre-Cycle-09 envelope, not marked loss of its new
    // context-proof index (DROP TABLE also removed that index). The independent
    // marked-index-loss acceptance control must continue to require recovery.
    connection
        .execute(
            "DELETE FROM registry_migrations WHERE name='cycle_09_context_publication_evidence'",
            [],
        )
        .unwrap();
    // Reconstruct pre-disposition history, not marked loss of its whole envelope.
    assert_eq!(connection.execute("DELETE FROM registry_migrations WHERE name='remote_poll_dispositions_unpublished_v1'", []).unwrap(), 1);
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
    let schema = legacy_preflight_schema();
    assert!(!schema.contains("preflight_digest") && !schema.contains("commit_time"));
    legacy_attempt_shape_upgrades_empty_attempts_and_rejects_populated_attempts(
        &schema,
        "INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,identity_confirmation_id,phase) VALUES(?1,?2,1,?3,0,zeroblob(32),zeroblob(32),NULL,'prepared')",
    );
}

#[test]
fn legacy_commit_time_migration_upgrades_empty_attempts_and_rejects_populated_attempts() {
    let schema = legacy_commit_time_schema();
    assert!(schema.contains("preflight_digest") && !schema.contains("commit_time"));
    legacy_attempt_shape_upgrades_empty_attempts_and_rejects_populated_attempts(
        &schema,
        "INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,identity_confirmation_id,phase) VALUES(?1,?2,1,?3,0,zeroblob(32),zeroblob(32),zeroblob(32),NULL,'prepared')",
    );
}

fn legacy_attempt_shape_upgrades_empty_attempts_and_rejects_populated_attempts(
    schema: &str,
    populated: &str,
) {
    let (data, _root, _service) = fixture();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE remote_publication_attempts; DROP TABLE remote_integration_merge_metadata; DROP TABLE remote_resolution_ref_log_artifacts; DROP TABLE remote_resolution_index_artifacts; DROP TABLE remote_resolution_paths; DROP TABLE remote_resolution_attempts; DROP TABLE remote_identity_confirmations; DROP TABLE remote_integration_steps; DROP TABLE remote_integration_windows;")
        .unwrap();
    connection.execute_batch(schema).unwrap();
    drop(connection);
    // Empty legacy evidence has no unobserved attempt to preserve, so it is
    // rebuilt with the immutable preflight and commit-time columns.
    let _reopened = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let columns = connection
        .prepare("PRAGMA table_info(remote_resolution_attempts)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for added in ["preflight_digest", "commit_time", "commit_offset_minutes"] {
        assert!(columns.iter().any(|column| column == added), "{added}");
    }

    let (data, root, service) = fixture();
    let operation = crate::repository::OperationId::new();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan());
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
    connection.execute_batch(schema).unwrap();
    connection.execute(
        "INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,conflict_digest,phase) VALUES(?1,?2,0,0,'primary',?3,?4,?5,zeroblob(32),zeroblob(32),'conflict_pending')",
        params![operation_id,generation,ADVERTISED,TRACKING,ADVERTISED],
    ).unwrap();
    connection
        .execute(
            populated,
            params![
                crate::repository::OperationId::new().to_string(),
                operation_id,
                generation
            ],
        )
        .unwrap();
    drop(connection);
    // A populated old attempt has no authentic observation of the added
    // evidence and must fail closed rather than receive a manufactured value.
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
            "recorded commit time immutability",
            MERGE_EVIDENCE_SCHEMA.replace(
                ",commit_time,commit_offset_minutes ON remote_resolution_attempts",
                " ON remote_resolution_attempts",
            ),
        ),
        (
            "recorded commit time check",
            MERGE_EVIDENCE_SCHEMA.replace(" AND commit_time > 0)", ")"),
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)
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
    let polling_columns = connection
        .prepare("PRAGMA table_info(remote_polling_state)")
        .unwrap()
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for name in ["latest_attempt_at", "next_eligible_at"] {
        assert!(polling_columns.contains(&(name.to_owned(), "INTEGER".to_owned())));
    }
    for (name, _) in polling_columns {
        for forbidden in ["body", "credential", "server", "path", "request", "session"] {
            assert!(
                !name.contains(forbidden),
                "persisted forbidden column {name}"
            );
        }
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
    connection.execute("INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,phase,commit_time,commit_offset_minutes) VALUES(?1,?2,1,?3,0,zeroblob(32),zeroblob(32),zeroblob(32),'paths_applying',1700000000,0)",params![attempt.to_string(),parent,generation]).unwrap();
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
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
        INSERT INTO remote_resolution_attempts(id,attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,identity_confirmation_id,phase,commit_time,commit_offset_minutes) VALUES(51,'01ARZ3NDEKTSV4RRFFQ69G5FAZ',1,41,1,0,zeroblob(32),zeroblob(32),zeroblob(32),61,'paths_applying',1700000000,0);
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
        "UPDATE remote_resolution_attempts SET commit_time=commit_time+1 WHERE id=51",
        "UPDATE remote_resolution_attempts SET commit_offset_minutes=60 WHERE id=51",
        "UPDATE remote_resolution_index_artifacts SET inode=99 WHERE attempt_id=51",
        "UPDATE remote_resolution_ref_log_artifacts SET digest=randomblob(32) WHERE attempt_id=51",
    ] {
        assert!(connection.execute(mutation, []).is_err());
    }
}

fn window_fixture(service: &RepositoryService, root: &Path) -> crate::repository::OperationId {
    let operation = crate::repository::OperationId::new();
    with_transaction(service, root, |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
        tx.execute("INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,candidate_oid,checkpoint_oid,phase,commit_time,commit_offset_minutes) VALUES(?1,?2,(SELECT id FROM remote_integration_steps WHERE operation_record_id=?2 AND window_number=0 AND ordinal=1),?3,0,zeroblob(32),zeroblob(32),zeroblob(32),?4,?4,'applied',1700000000,0)", params![crate::repository::OperationId::new().to_string(), record.id, record.generation, context.to_string()]).unwrap();
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
            commit_time: (1_700_000_000, 0),
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
    connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO remote_resolution_attempts(id,attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,phase,commit_time,commit_offset_minutes) VALUES(51,'01ARZ3NDEKTSV4RRFFQ69G5FAZ',999,41,0,0,zeroblob(32),zeroblob(32),zeroblob(32),'prepared',1700000000,0); PRAGMA foreign_keys=ON;").unwrap();
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
        let generation = configure(tx, id, Some(&next_plan), ConfigurationChange::Plan)?;
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
            configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
                        commit_time: (1_700_000_000, 0),
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
            configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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

/// Closing an attempt that never began its paths leaves the step as it is,
/// though another attempt of the step has begun its own: only an attempt
/// that began its paths put the step at `resolution_prepared`, and only its
/// closing puts it back (final review, minor 3).
#[test]
fn closing_an_unbegun_attempt_keeps_the_step_another_attempt_began() {
    let (data, root, service) = fixture();
    let operation = crate::repository::OperationId::new();
    with_transaction(&service, root.path(), |tx, id| {
        configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
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
    let (parent, generation): (i64, i64) = connection
        .query_row(
            "SELECT id,configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    connection.execute("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,conflict_digest,phase) VALUES(?1,?2,0,0,'primary',?3,?4,?5,zeroblob(32),zeroblob(32),'resolution_prepared')",params![parent,generation,ADVERTISED,TRACKING,ADVERTISED]).unwrap();
    let begun = crate::repository::OperationId::new();
    let unbegun = crate::repository::OperationId::new();
    for (attempt, phase) in [(begun, "paths_applying"), (unbegun, "prepared")] {
        connection.execute("INSERT INTO remote_resolution_attempts(attempt_ulid,operation_record_id,integration_step_id,configuration_generation,owner_epoch,observation_digest,input_digest,preflight_digest,phase,commit_time,commit_offset_minutes) VALUES(?1,?2,1,?3,0,zeroblob(32),zeroblob(32),zeroblob(32),?4,1700000000,0)",params![attempt.to_string(),parent,generation,phase]).unwrap();
    }
    drop(connection);
    let closed = with_transaction(&service, root.path(), |tx, id| {
        let record = read_operation(tx, id, operation)?.ok_or_else(recovery_required)?;
        close_unwritten_resolution_attempt(tx, &record, unbegun)
    })
    .unwrap();
    assert!(closed);
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let step: String = connection
        .query_row("SELECT phase FROM remote_integration_steps", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(step, "resolution_prepared");
    let attempts: Vec<String> = connection
        .prepare("SELECT attempt_ulid FROM remote_resolution_attempts")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(attempts, [begun.to_string()]);
}

#[test]
fn credentials_fence_carries_batch_and_rows() {
    let document_ref = format!("refs/heads/manyhands/document/{ITEM}");
    let document = || {
        RemoteRefObservation::from_advertisement(
            &plan(),
            &document_ref,
            Oid::from_str(TRACKING).unwrap(),
            None,
        )
        .unwrap()
    };
    for pinned in [false, true] {
        let (data, root, service) = fixture();
        let operation = window_fixture(&service, root.path());
        let generation = with_transaction(&service, root.path(), |tx, id| {
            let generation = read_policy(tx, id)?.generation;
            complete_batch(
                tx,
                id,
                &plan(),
                generation,
                &[document(), advertised()],
                123,
            )?;
            Ok(generation)
        })
        .unwrap();
        // The second batch no longer advertises the document: its row becomes
        // `remotely_deleted` next to the observed ticket row.
        let old_batch = if pinned {
            let intent = pinned_window(
                &service,
                root.path(),
                operation,
                Some(Oid::from_str(ADVERTISED).unwrap()),
            );
            with_transaction(&service, root.path(), |tx, id| {
                let record = read_operation(tx, id, operation)?.unwrap();
                prepare_integration_window(tx, &record, 1, &intent)?;
                Ok(intent.observation_batch_id)
            })
            .unwrap()
        } else {
            with_transaction(&service, root.path(), |tx, id| {
                complete_batch(tx, id, &plan(), generation, &[advertised()], 124)
            })
            .unwrap()
        };
        let before = service.remote_snapshot(root.path()).unwrap();
        let carried = with_transaction(&service, root.path(), |tx, id| {
            let after = configure(tx, id, Some(&plan()), ConfigurationChange::Credentials)?;
            // The startup audit reads every batch, pinned historical ones too.
            read_snapshot_rows(tx, id, true)?;
            Ok(after)
        })
        .unwrap();
        assert_eq!(carried, generation + 1, "pinned={pinned}");
        let after = service.remote_snapshot(root.path()).unwrap();
        assert_eq!(
            format!("{:?}", after.observations()),
            format!("{:?}", before.observations()),
            "pinned={pinned}"
        );
        assert_eq!(
            format!("{:?}", after.contexts()),
            format!("{:?}", before.contexts()),
            "pinned={pinned}"
        );
        assert_eq!(after.contexts().len(), 2);
        assert!(
            after
                .contexts()
                .iter()
                .any(|context| context.state() == RemoteContextState::RemotelyDeleted)
        );
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        let (current_generation, current_rows): (i64, i64) = db
            .query_row(
                "SELECT configuration_generation,(SELECT COUNT(*) FROM remote_ref_observations WHERE batch_id=remote_observation_batches.id) FROM remote_observation_batches WHERE is_current=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(current_generation, carried);
        assert_eq!(current_rows as usize, before.observations().len());
        let old_kept: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM remote_observation_batches WHERE id=?1 AND is_current=0)",
                [old_batch],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old_kept, pinned, "pinned={pinned}");
        // An unavailable plan keeps the evidence dormant; its return carries
        // it again, and a fresh observation at that generation still works.
        with_transaction(&service, root.path(), |tx, id| {
            configure(tx, id, None, ConfigurationChange::Unavailable)?;
            read_snapshot_rows(tx, id, true)?;
            assert!(read_snapshot(tx, id)?.observations().is_empty());
            let back = configure(tx, id, Some(&plan()), ConfigurationChange::Plan)?;
            read_snapshot_rows(tx, id, true)?;
            assert_eq!(
                format!("{:?}", read_snapshot(tx, id)?.contexts()),
                format!("{:?}", before.contexts())
            );
            complete_batch(tx, id, &plan(), back, &[advertised()], 125)?;
            read_snapshot_rows(tx, id, true)?;
            Ok(())
        })
        .unwrap();
    }
}

#[derive(Clone, Copy)]
enum Call {
    Plan(&'static str, &'static str),
    Endpoints(&'static str, &'static str, u8),
    Credentials,
    Unavailable,
}

fn apply_call(service: &RepositoryService, root: &Path, call: Call) -> i64 {
    with_transaction(service, root, |tx, id| match call {
        Call::Plan(name, branch) => {
            let plan = RemoteRefPlan::from_configuration(name, branch).unwrap();
            configure(tx, id, Some(&plan), ConfigurationChange::Plan)
        }
        Call::Endpoints(name, branch, digest) => {
            let plan = RemoteRefPlan::from_configuration(name, branch).unwrap();
            configure_endpoints(tx, id, &plan, &[digest; 32])
        }
        Call::Credentials => {
            let plan = read_policy(tx, id)?.plan;
            configure(tx, id, plan.as_ref(), ConfigurationChange::Credentials)
        }
        Call::Unavailable => configure(tx, id, None, ConfigurationChange::Unavailable),
    })
    .unwrap()
}

#[test]
fn configure_reasons_fence_and_forget() {
    use Call::*;
    let a = Plan("origin", "main");
    // (setup, flag set before the call, call, bumps, forgets, flag after,
    // known name, known branch, known digest byte)
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        Vec<Call>,
        bool,
        Call,
        bool,
        bool,
        i64,
        Option<&str>,
        Option<&str>,
        Option<u8>,
    )> = vec![
        // first plan
        (
            vec![],
            false,
            a,
            true,
            false,
            0,
            Some("origin"),
            Some("main"),
            None,
        ),
        // credentials, unavailable
        (
            vec![a],
            false,
            Credentials,
            true,
            false,
            0,
            Some("origin"),
            Some("main"),
            None,
        ),
        (
            vec![a],
            false,
            Unavailable,
            true,
            false,
            0,
            Some("origin"),
            Some("main"),
            None,
        ),
        // the same plan returning after unavailable
        (
            vec![a, Unavailable],
            false,
            a,
            true,
            false,
            0,
            Some("origin"),
            Some("main"),
            None,
        ),
        // a different plan after unavailable
        (
            vec![a, Unavailable],
            false,
            Plan("upstream", "main"),
            true,
            true,
            0,
            Some("upstream"),
            Some("main"),
            None,
        ),
        // a new primary branch
        (
            vec![a],
            false,
            Plan("origin", "trunk"),
            true,
            true,
            0,
            Some("origin"),
            Some("trunk"),
            None,
        ),
        // digest changed, then unchanged
        (
            vec![Endpoints("origin", "main", 1)],
            false,
            Endpoints("origin", "main", 2),
            true,
            true,
            0,
            Some("origin"),
            Some("main"),
            Some(2),
        ),
        (
            vec![Endpoints("origin", "main", 1)],
            false,
            Endpoints("origin", "main", 1),
            false,
            false,
            0,
            Some("origin"),
            Some("main"),
            Some(1),
        ),
        // unavailable, then the same digest returning: known digest kept
        (
            vec![Endpoints("origin", "main", 1), Unavailable],
            false,
            Endpoints("origin", "main", 1),
            true,
            false,
            0,
            Some("origin"),
            Some("main"),
            Some(1),
        ),
        // the cache-loss flag stays set, and nothing here sets it
        (
            vec![a],
            true,
            Credentials,
            true,
            false,
            1,
            Some("origin"),
            Some("main"),
            None,
        ),
    ];
    for (n, (setup, flag, call, bumps, forgets, flag_after, name, branch, digest)) in
        rows.into_iter().enumerate()
    {
        let (data, root, service) = fixture();
        let mut generation = 0;
        let seeded = !setup.is_empty();
        Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,'/active',?2)", params![format!("manyhands/ticket/{ITEM}"), ITEM]).unwrap();
        for (step_n, step) in setup.into_iter().enumerate() {
            generation = apply_call(&service, root.path(), step);
            if step_n == 0 {
                // Evidence for the first remote: a current batch and a row.
                with_transaction(&service, root.path(), |tx, id| {
                    complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
                })
                .unwrap();
            }
        }
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        if flag {
            db.execute("UPDATE remote_polling_state SET history_unknown=1", [])
                .unwrap();
        }
        let after = apply_call(&service, root.path(), call);
        assert_eq!(after != generation, bumps, "row {n} generation");
        let (unknown, known_name, known_branch, known_digest): (
            i64,
            Option<String>,
            Option<String>,
            Option<Vec<u8>>,
        ) = db
            .query_row(
                "SELECT history_unknown,known_remote_name,known_primary_branch,known_endpoint_digest FROM remote_polling_state",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        if seeded {
            // Forgetting rewrites every active context to an unknown row;
            // anything else keeps the observed row and carries the batch.
            let (rows, evidence, state, batch_generation): (i64, String, String, Option<i64>) = db
                .query_row(
                    "SELECT (SELECT COUNT(*) FROM remote_context_states),(SELECT publication_evidence FROM remote_context_states WHERE item_id=?1),(SELECT state FROM remote_context_states WHERE item_id=?1),(SELECT configuration_generation FROM remote_observation_batches WHERE is_current=1)",
                    [ITEM],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .unwrap();
            assert_eq!(rows, 1, "row {n} one row per active context");
            let (want_evidence, want_state) = if forgets {
                ("history_unknown", "history_unknown")
            } else {
                ("observed_published", "unmaterialized")
            };
            assert_eq!(
                (evidence.as_str(), state.as_str()),
                (want_evidence, want_state),
                "row {n} rows"
            );
            assert_eq!(
                batch_generation,
                (!forgets).then_some(after),
                "row {n} carried batch"
            );
            // A context registered afterwards has no row: never published.
            let later = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
            db.execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,'/later',?2)", params![format!("manyhands/ticket/{later}"), later]).unwrap();
            assert_eq!(
                read_snapshot(&db, 1)
                    .unwrap()
                    .fetch_publication_evidence_for(AuthoringKind::Ticket, &later.parse().unwrap()),
                if flag_after == 0 {
                    RemotePublicationEvidence::NeverPublished
                } else {
                    RemotePublicationEvidence::HistoryUnknown
                },
                "row {n} later context"
            );
        }
        assert_eq!(unknown, flag_after, "row {n} flag");
        assert_eq!(known_name.as_deref(), name, "row {n} known name");
        assert_eq!(known_branch.as_deref(), branch, "row {n} known branch");
        assert_eq!(
            known_digest,
            digest.map(|byte| vec![byte; 32]),
            "row {n} known digest"
        );
    }
}

/// A forgetting fence writes an unknown row for each active context except
/// one whose item is retired (R1): that history was consumed, and its new
/// context keeps reading as never published.
#[test]
fn a_forgetting_fence_writes_no_unknown_row_for_a_retired_item() {
    const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
    let (data, root, service) = fixture();
    apply_call(&service, root.path(), Call::Plan("origin", "main"));
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    for item in [ITEM, OTHER] {
        db.execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,?3,?2)", params![format!("manyhands/ticket/{item}"), item, format!("/{item}")]).unwrap();
    }
    seed_retirement(&db, AuthoringKind::Ticket, ITEM);
    apply_call(&service, root.path(), Call::Plan("upstream", "main"));
    let items: Vec<String> = db
        .prepare("SELECT item_id FROM remote_context_states")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(items, [OTHER]);
}

/// What still reads unknown under the cache-loss flag: an unknown row counts
/// only while its context is active, and an active context with no row counts
/// too. A ghost row (its context released or removed by hand) is neither
/// listed nor blocking.
#[test]
fn unresolved_contexts_count_only_active_contexts() {
    const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
    // (active contexts, rows by item, items listed, flag cleared by resolving)
    #[allow(clippy::type_complexity)]
    let rows: [(&[&str], &[&str], &[&str], bool); 5] = [
        (&[ITEM], &[ITEM], &[ITEM], false),
        (&[], &[ITEM], &[], true),
        (&[OTHER], &[ITEM], &[OTHER], false),
        (&[ITEM, OTHER], &[ITEM], &[ITEM, OTHER], false),
        (&[], &[], &[], true),
    ];
    for (n, (active, with_rows, listed, clears)) in rows.into_iter().enumerate() {
        let (data, root, service) = fixture();
        apply_call(&service, root.path(), Call::Plan("origin", "main"));
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        for item in with_rows {
            let target = plan().context(AuthoringKind::Ticket, &item.parse().unwrap());
            db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,'history_unknown','history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),item]).unwrap();
        }
        for item in active {
            db.execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,?3,?2)", params![format!("manyhands/ticket/{item}"), item, format!("/{item}")]).unwrap();
        }
        db.execute("UPDATE remote_polling_state SET history_unknown=1", [])
            .unwrap();
        let (flag, unknown) = with_transaction(&service, root.path(), unresolved_contexts).unwrap();
        assert!(flag, "row {n} flag");
        let unknown: Vec<String> = unknown.iter().map(|(_, id)| id.to_string()).collect();
        assert_eq!(unknown, listed, "row {n} listed");
        let cleared = with_transaction(&service, root.path(), |tx, id| {
            clear_cache_loss_when_resolved(tx, id)
        })
        .unwrap();
        assert_eq!(cleared, clears, "row {n} cleared");
    }
}

#[test]
fn null_known_columns_fall_back_to_the_current_plan() {
    let (data, root, service) = fixture();
    let generation = apply_call(&service, root.path(), Call::Plan("origin", "main"));
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    // A registry from before the known columns: no batch, a stale row.
    db.execute_batch(
        "UPDATE remote_polling_state SET known_remote_name=NULL,known_primary_branch=NULL;",
    )
    .unwrap();
    db.execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,'/active',?2)", params![format!("manyhands/ticket/{ITEM}"), ITEM]).unwrap();
    let target = RemoteRefPlan::from_configuration("origin", "main")
        .unwrap()
        .context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
    db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,'never_published','history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),ITEM]).unwrap();
    assert_ne!(
        apply_call(&service, root.path(), Call::Plan("upstream", "main")),
        generation
    );
    let (evidence, remote_ref): (String, String) = db
        .query_row(
            "SELECT publication_evidence,remote_ref FROM remote_context_states",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(evidence, "history_unknown");
    let upstream = RemoteRefPlan::from_configuration("upstream", "main")
        .unwrap()
        .context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
    assert_eq!(remote_ref, upstream.remote_ref());
}

#[test]
fn null_known_and_no_plan_rows_follow_the_new_plan() {
    // (the remote configured afterwards, whether the kept rows are forgotten)
    for (remote, forgets) in [("upstream", true), ("origin", false)] {
        let (data, root, service) = fixture();
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        // No known identity and no plan, with a row under the old remote.
        db.execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,'/active',?2)", params![format!("manyhands/ticket/{ITEM}"), ITEM]).unwrap();
        let target = plan().context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
        db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,'never_published','history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),ITEM]).unwrap();
        apply_call(&service, root.path(), Call::Plan(remote, "main"));
        let (evidence, remote_ref): (String, String) = db
            .query_row(
                "SELECT publication_evidence,remote_ref FROM remote_context_states",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let expected = RemoteRefPlan::from_configuration(remote, "main")
            .unwrap()
            .context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
        assert_eq!(remote_ref, expected.remote_ref(), "{remote} refs");
        assert_eq!(
            evidence,
            if forgets {
                "history_unknown"
            } else {
                "never_published"
            },
            "{remote} evidence"
        );
        read_snapshot(&db, 1).unwrap_or_else(|_| panic!("{remote} readable"));
    }
}

#[test]
fn history_per_context_migration() {
    const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
    // (flag, marker file, a row for the second context, migrations run)
    for (n, (flag, marker, with_row, runs)) in [
        (1, false, false, 1),
        (1, true, false, 1),
        (0, false, false, 1),
        (1, false, true, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let (data, root, service) = fixture();
        apply_call(&service, root.path(), Call::Plan("origin", "main"));
        drop(service);
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        for id in [ITEM, OTHER] {
            db.execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,?2,?3)", params![format!("manyhands/ticket/{id}"), format!("/{id}"), id]).unwrap();
        }
        let target = plan().context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
        db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,'history_unknown','history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),ITEM]).unwrap();
        if with_row {
            let target = plan().context(AuthoringKind::Ticket, &OTHER.parse().unwrap());
            db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,'never_published','history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),OTHER]).unwrap();
        }
        db.execute("UPDATE remote_polling_state SET history_unknown=?1,endpoint_digest=zeroblob(32),known_remote_name=NULL,known_primary_branch=NULL,known_endpoint_digest=NULL", [flag]).unwrap();
        db.execute(
            "DELETE FROM registry_migrations WHERE name='remote_history_per_context_v1'",
            [],
        )
        .unwrap();
        if marker {
            std::fs::write(
                data.path().join("remote-history-recovery-required"),
                b"manyhands remote history recovery required v1\n",
            )
            .unwrap();
        }
        let unmigrated = |db: &Connection| -> Vec<(String, String)> {
            db.prepare(
                "SELECT item_id,publication_evidence FROM remote_context_states ORDER BY item_id",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
        };
        let before = unmigrated(&db);
        let mut after = Vec::new();
        for _ in 0..runs {
            drop(RepositoryService::open_at(data.path()).unwrap());
            after = unmigrated(&db);
        }
        let migrates = flag == 1 && !marker;
        let (unknown, name, branch, digest): (i64, Option<String>, Option<String>, Option<Vec<u8>>) = db
            .query_row(
                "SELECT history_unknown,known_remote_name,known_primary_branch,known_endpoint_digest FROM remote_polling_state",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(unknown, i64::from(flag == 1 && marker), "row {n} flag");
        if migrates {
            let mut want = vec![
                (ITEM.to_owned(), "history_unknown".to_owned()),
                (
                    OTHER.to_owned(),
                    if with_row {
                        "never_published"
                    } else {
                        "history_unknown"
                    }
                    .to_owned(),
                ),
            ];
            want.sort();
            assert_eq!(after, want, "row {n} rows");
        } else {
            assert_eq!(after, before, "row {n} unchanged");
        }
        assert_eq!(name.as_deref(), Some("origin"), "row {n} known name");
        assert_eq!(branch.as_deref(), Some("main"), "row {n} known branch");
        assert_eq!(digest, Some(vec![0; 32]), "row {n} known digest");
        read_snapshot(&db, 1).unwrap_or_else(|_| panic!("row {n} readable"));
        let marked: i64 = db.query_row("SELECT count(*) FROM registry_migrations WHERE name='remote_history_per_context_v1'", [], |r| r.get(0)).unwrap();
        assert_eq!(marked, 1, "row {n} marker");
    }
}

#[test]
fn evidence_defaults_by_flag_and_rows() {
    let ticket = AuthoringKind::Ticket;
    let item: ItemId = ITEM.parse().unwrap();
    for flag in [false, true] {
        for row in [
            None,
            Some("never_published"),
            Some("history_unknown"),
            Some("observed_published"),
        ] {
            for retired in [false, true] {
                let (data, root, service) = fixture();
                let generation = apply_call(&service, root.path(), Call::Plan("origin", "main"));
                let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
                if row == Some("observed_published") {
                    with_transaction(&service, root.path(), |tx, id| {
                        complete_batch(tx, id, &plan(), generation, &[advertised()], 123)
                    })
                    .unwrap();
                }
                if let Some(evidence @ ("never_published" | "history_unknown")) = row {
                    let target = plan().context(ticket, &item);
                    db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,?4,'history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),ITEM,evidence]).unwrap();
                }
                if flag {
                    db.execute("UPDATE remote_polling_state SET history_unknown=1", [])
                        .unwrap();
                }
                if retired {
                    seed_retirement(&db, ticket, ITEM);
                }
                let case = format!("flag={flag} row={row:?} retired={retired}");
                // A `never_published` row is the owner's or the creation's
                // explicit resolution (`item resolve-history`), so it is
                // accepted under the cache-loss flag too; only contexts with
                // no row read unknown under the flag.
                let snapshot = read_snapshot(&db, 1);
                assert!(snapshot.is_ok(), "{case} accepted");
                if let Ok(snapshot) = snapshot {
                    let expected = match row {
                        Some("never_published") => RemotePublicationEvidence::NeverPublished,
                        Some("history_unknown") => RemotePublicationEvidence::HistoryUnknown,
                        Some(_) => RemotePublicationEvidence::ObservedPublished,
                        None if flag && !retired => RemotePublicationEvidence::HistoryUnknown,
                        None => RemotePublicationEvidence::NeverPublished,
                    };
                    assert_eq!(
                        snapshot.fetch_publication_evidence_for(ticket, &item),
                        expected,
                        "{case}"
                    );
                }
            }
        }
    }
}

#[test]
fn corrupt_dormant_context_rows_are_rejected_while_the_plan_is_unavailable() {
    // (column assignment, a row the CHECKs allow but the evidence/state pairing does not)
    for update in [
        "last_advertised_oid='1111111111111111111111111111111111111111'",
        "publication_evidence='observed_published'",
        "state='observed'",
        "item_id='not-a-ulid'",
    ] {
        let (data, root, service) = fixture();
        apply_call(&service, root.path(), Call::Plan("origin", "main"));
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        let target = plan().context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
        db.execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(1,?1,?2,'ticket',?3,'never_published','history_unknown',123)",params![target.remote_ref(),target.tracking_ref(),ITEM]).unwrap();
        apply_call(&service, root.path(), Call::Unavailable);
        assert!(read_snapshot_rows(&db, 1, true).is_ok(), "{update} before");
        db.execute(&format!("UPDATE remote_context_states SET {update}"), [])
            .unwrap();
        assert!(read_snapshot_rows(&db, 1, true).is_err(), "{update} audit");
        assert!(read_snapshot_rows(&db, 1, false).is_err(), "{update} read");
    }
}

#[test]
fn poll_dispositions_unpublished_migration_preserves_rows_and_is_idempotent() {
    let (data, _, service, _) = review_completed_state();
    drop(service);
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    install_historical_dispositions(&db, 15);
    let insert = format!(
        "INSERT INTO remote_poll_dispositions SELECT repository_id,operation_ulid,1,'context','refs/heads/manyhands/ticket/{ITEM}','ticket','{ITEM}','unpublished',NULL,NULL,observed_at FROM remote_poll_dispositions WHERE ordinal=0"
    );
    assert!(db.execute(&insert, []).is_err());
    let rows = |db: &Connection| -> Vec<(i64, String, Option<String>)> {
        db.prepare(
            "SELECT ordinal,disposition,after_oid FROM remote_poll_dispositions ORDER BY ordinal",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    };
    let before = rows(&db);
    assert_eq!(before.len(), 1);
    drop(db);
    for _ in 0..2 {
        drop(RepositoryService::open_at(data.path()).unwrap());
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        assert_eq!(rows(&db), before);
        let marked: i64 = db.query_row("SELECT count(*) FROM registry_migrations WHERE name='remote_poll_dispositions_unpublished_v1'", [], |r| r.get(0)).unwrap();
        assert_eq!(marked, 1);
        let triggers: i64 = db.query_row("SELECT count(*) FROM sqlite_master WHERE name='remote_poll_disposition_owner_immutable'", [], |r| r.get(0)).unwrap();
        assert_eq!(triggers, 1);
    }
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute(&insert, []).unwrap();
    assert_eq!(rows(&db).len(), 2);
}

/// A forgetting `Plan` fence must not leave the old remote's endpoint digest
/// as the known one: the first endpoint registration under the new remote
/// would read as a digest change and forget the owner's resolution again.
#[test]
fn a_forgetting_plan_fence_does_not_forget_again_at_the_next_endpoint_registration() {
    let (data, root, service) = fixture();
    apply_call(&service, root.path(), Call::Endpoints("origin", "main", 1));
    Connection::open(data.path().join(REGISTRY_FILE)).unwrap().execute("INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active',?1,'/active',?2)", params![format!("manyhands/ticket/{ITEM}"), ITEM]).unwrap();
    apply_call(&service, root.path(), Call::Plan("upstream", "main"));
    let upstream = RemoteRefPlan::from_configuration("upstream", "main").unwrap();
    let item: ItemId = ITEM.parse().unwrap();
    with_transaction(&service, root.path(), |tx, id| {
        assert_eq!(
            read_snapshot(tx, id)?.fetch_publication_evidence_for(AuthoringKind::Ticket, &item),
            RemotePublicationEvidence::HistoryUnknown
        );
        record_context_resolved(tx, id, &upstream, AuthoringKind::Ticket, &item)
    })
    .unwrap();
    apply_call(
        &service,
        root.path(),
        Call::Endpoints("upstream", "main", 2),
    );
    with_transaction(&service, root.path(), |tx, id| {
        assert_eq!(
            read_snapshot(tx, id)?.fetch_publication_evidence_for(AuthoringKind::Ticket, &item),
            RemotePublicationEvidence::NeverPublished
        );
        Ok(())
    })
    .unwrap();
}

/// A registry created while the cache-loss marker was on disk bakes `DEFAULT 1`
/// into `remote_polling_state`. Once the marker is retired, reopening rebuilds
/// the table with defaults 0, keeping every row and the registration trigger,
/// so a repository registered afterwards starts clear and unsuspended.
#[test]
fn polling_defaults_are_rebuilt_once_the_marker_is_retired() {
    let data = tempfile::tempdir().unwrap();
    let marker = data.path().join("remote-history-recovery-required");
    std::fs::write(&marker, b"manyhands remote history recovery required v1\n").unwrap();
    drop(RepositoryService::open_at(data.path()).unwrap());
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let register = |db: &Connection, root: &str| {
        db.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)", [root]).unwrap();
    };
    let flags = |db: &Connection, id: i64| -> (i64, i64) {
        db.query_row(
            "SELECT history_unknown,recovery_suspended FROM remote_polling_state WHERE repository_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    };
    register(&db, "/first");
    assert_eq!(flags(&db, 1), (1, 1), "defaults while the marker stands");
    // Distinguishable values in every kind of column.
    db.execute("UPDATE remote_polling_state SET paused=1,interval_seconds=600,automatic_backoff_seconds=120,remote_name='origin',primary_branch='main',endpoint_digest=zeroblob(32),known_remote_name='origin',known_primary_branch='main',known_endpoint_digest=zeroblob(32),latest_outcome='completed',latest_attempt_at=5,next_eligible_at=9,configuration_generation=3 WHERE repository_id=1", []).unwrap();
    let dump = |db: &Connection| -> String {
        db.query_row("SELECT quote(repository_id)||quote(enabled)||quote(paused)||quote(interval_seconds)||quote(automatic_backoff_seconds)||quote(recovery_suspended)||quote(history_unknown)||quote(configuration_generation)||quote(endpoint_digest)||quote(remote_name)||quote(primary_branch)||quote(latest_outcome)||quote(latest_attempt_at)||quote(next_eligible_at)||quote(known_remote_name)||quote(known_primary_branch)||quote(known_endpoint_digest) FROM remote_polling_state WHERE repository_id=1", [], |r| r.get(0)).unwrap()
    };
    let columns = |db: &Connection| -> Vec<String> {
        db.prepare("SELECT name FROM pragma_table_info('remote_polling_state') ORDER BY cid")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let (row, order) = (dump(&db), columns(&db));
    drop(db);
    // With the marker still on disk the table is left alone.
    drop(RepositoryService::open_at(data.path()).unwrap());
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    register(&db, "/second");
    assert_eq!(flags(&db, 2), (1, 1), "marker present: defaults stay");
    drop(db);
    std::fs::remove_file(&marker).unwrap();
    for _ in 0..2 {
        drop(RepositoryService::open_at(data.path()).unwrap());
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        register_if_missing(&db);
        assert_eq!(dump(&db), row, "existing row exactly as it was");
        assert_eq!(columns(&db), order, "column order");
        assert_eq!(flags(&db, 2), (1, 1), "existing rows keep their values");
        assert_eq!(flags(&db, 3), (0, 0), "a new repository starts clear");
        let triggers: i64 = db.query_row("SELECT count(*) FROM sqlite_master WHERE type='trigger' AND name='remote_policy_on_registration'", [], |r| r.get(0)).unwrap();
        assert_eq!(triggers, 1);
    }

    fn register_if_missing(db: &Connection) {
        db.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) SELECT '/third',123,'accessible',0 WHERE NOT EXISTS(SELECT 1 FROM repositories WHERE root_path='/third')", []).unwrap();
    }
}
