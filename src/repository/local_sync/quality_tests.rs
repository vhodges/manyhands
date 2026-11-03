//! Task 4 QUALITY ses_e881a2165ffeEIJWyTo9j3HVR3 regressions.
use super::{record::LocalContextSyncRecord, record_tests::Fixture};
use crate::{repository::*, results::ResultCode};
use rusqlite::{params, types::Value};

fn children(f: &Fixture, row: &LocalContextSyncRecord) {
    let db = f.db();
    db.execute("INSERT INTO local_context_sync_steps(operation_ulid,ordinal,generation,owner_epoch,kind,phase,baseline_index_digest,baseline_metadata_digest) VALUES(?1,0,1,1,'objects','prepared',zeroblob(32),zeroblob(32))",[row.operation_id.to_string()]).unwrap();
    db.execute("INSERT INTO local_context_sync_resolutions(attempt_ulid,operation_ulid,request_ulid,generation,owner_epoch,observation,input_digest,state) VALUES(?1,?2,?3,1,1,?4,zeroblob(32),'prepared')",params![OperationId::new().to_string(),row.operation_id.to_string(),OperationId::new().to_string(),row.observation]).unwrap();
}

fn abandoned_before_effects(f: &Fixture, row: &LocalContextSyncRecord) {
    let mut db = f.db();
    let tx = db.transaction().unwrap();
    tx.execute(
        "UPDATE local_context_sync_resolutions SET state='abandoned'",
        [],
    )
    .unwrap();
    tx.execute("UPDATE local_context_sync_records SET state='abandoned',owner_epoch=owner_epoch+1,semantic_revision=semantic_revision+1",[]).unwrap();
    tx.execute("UPDATE operation_records SET state='completed',completed_step='abandoned' WHERE operation_ulid=?1",[row.operation_id.to_string()]).unwrap();
    tx.commit().unwrap();
    // Historical terminal fixture only: creation/admission is always the real
    // store. This does not implement Task 6/7 abandonment or request settlement.
    f.service
        .read_local_context_sync_record(&f.repo, row.operation_id)
        .unwrap();
}

fn assert_removed(f: &Fixture) {
    let before = f.image();
    let result = f.service.remove_registration(RemoveRegistrationRequest {
        root: f.root.path().into(),
        operation_id: OperationId::new(),
    });
    assert!(
        matches!(result, Ok(RemoveRegistrationOutcome::Removed)),
        "{result:?}"
    );
    for table in [
        "repositories",
        "operation_records",
        "local_context_sync_records",
        "local_context_sync_steps",
        "local_context_sync_resolutions",
    ] {
        assert_eq!(
            f.db()
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0,
            "{table}"
        );
    }
    assert_eq!(before, f.image());
}

#[test]
fn public_registration_removal_cascades_terminal_local_history_atomically() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    children(&f, &row);
    abandoned_before_effects(&f, &row);
    assert_removed(&f);
}

#[test]
fn public_registration_removal_preserves_pending_local_children() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    children(&f, &row);
    let before = f.image();
    let removal = OperationId::new();
    assert!(
        f.service
            .remove_registration(RemoveRegistrationRequest {
                root: f.root.path().into(),
                operation_id: removal
            })
            .is_err()
    );
    assert_eq!(
        f.service
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap(),
        row
    );
    for table in [
        "repositories",
        "local_context_sync_records",
        "local_context_sync_steps",
        "local_context_sync_resolutions",
    ] {
        assert_eq!(
            f.db()
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1,
            "{table}"
        );
    }
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [removal.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(before, f.image());
}

#[test]
fn failed_public_registration_removal_rolls_back_terminal_child_deletion() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    children(&f, &row);
    abandoned_before_effects(&f, &row);
    let terminal = f
        .service
        .read_local_context_sync_record(&f.repo, row.operation_id)
        .unwrap();
    let before = f.image();
    f.db().execute_batch("CREATE TRIGGER refuse_registration_delete BEFORE DELETE ON repositories BEGIN SELECT RAISE(ABORT,'fixture'); END").unwrap();
    let removal = OperationId::new();
    assert!(
        f.service
            .remove_registration(RemoveRegistrationRequest {
                root: f.root.path().into(),
                operation_id: removal
            })
            .is_err()
    );
    assert_eq!(
        terminal,
        f.service
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap()
    );
    for table in [
        "repositories",
        "local_context_sync_records",
        "local_context_sync_steps",
        "local_context_sync_resolutions",
    ] {
        assert_eq!(
            f.db()
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1,
            "{table}"
        );
    }
    f.db()
        .execute_batch("DROP TRIGGER refuse_registration_delete")
        .unwrap();
    assert!(matches!(
        f.service.remove_registration(RemoveRegistrationRequest {
            root: f.root.path().into(),
            operation_id: removal
        }),
        Ok(RemoveRegistrationOutcome::Removed)
    ));
    assert_eq!(before, f.image());
}

#[test]
fn child_integer_authority_corruption_refuses_read_and_takeover() {
    for (table, column) in [
        ("local_context_sync_steps", "ordinal"),
        ("local_context_sync_steps", "owner_epoch"),
        ("local_context_sync_steps", "generation"),
        ("local_context_sync_resolutions", "owner_epoch"),
        ("local_context_sync_resolutions", "generation"),
    ] {
        let mut values = vec![
            Value::Text("not-an-integer".into()),
            Value::Real(0.5),
            Value::Real(1.0e30),
            Value::Integer(-1),
        ];
        if column != "ordinal" {
            values.extend([
                Value::Integer(0),
                Value::Integer(2),
                Value::Integer(i64::MAX),
            ]);
        }
        for value in values {
            let f = Fixture::new(AuthoringKind::Ticket);
            let row = f.begin(OperationId::new()).unwrap();
            children(&f, &row);
            let db = f.db();
            // Simulate a corrupt writer even for values SQL CHECKs reject.
            db.execute_batch("PRAGMA ignore_check_constraints=ON")
                .unwrap();
            db.execute(&format!("UPDATE {table} SET {column}=?1"), [&value])
                .unwrap();
            let before = f.image();
            assert_eq!(
                f.service
                    .read_local_context_sync_record(&f.repo, row.operation_id)
                    .unwrap_err()
                    .code(),
                ResultCode::RecoveryRequired,
                "{table}.{column} {value:?}"
            );
            assert_eq!(
                f.service
                    .take_over_local_context_sync(&f.repo, &row)
                    .unwrap_err()
                    .code(),
                ResultCode::RecoveryRequired,
                "{table}.{column} {value:?}"
            );
            let authority:(i64,i64,i64)=db.query_row("SELECT owner_epoch,generation,semantic_revision FROM local_context_sync_records",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
            assert_eq!(
                authority,
                (row.owner_epoch, row.generation, row.semantic_revision)
            );
            assert_eq!(before, f.image());
        }
    }
}

#[test]
fn child_integer_boundaries_and_older_owner_epoch_are_valid() {
    for ordinal in [0, i64::MAX] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let row = f.begin(OperationId::new()).unwrap();
        children(&f, &row);
        f.db()
            .execute("UPDATE local_context_sync_steps SET ordinal=?1", [ordinal])
            .unwrap();
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap(),
            row
        );
        let owner = f
            .service
            .take_over_local_context_sync(&f.repo, &row)
            .unwrap();
        assert_eq!(owner.owner_epoch, 2);
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap(),
            owner
        );
    }
}

#[test]
fn corrupt_closure_checkpoint_refuses_read_and_takeover() {
    for spelling in ["zero", "uppercase", "short", "invalid"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        super::admission_tests::closure_owner_with(&f, true, "valid");
        let row = f.begin(OperationId::new()).unwrap();
        let checkpoint = row.closure_checkpoint.unwrap().to_string();
        let corrupt = match spelling {
            "zero" => "0".repeat(40),
            "uppercase" => checkpoint.to_uppercase(),
            "short" => checkpoint[..12].into(),
            _ => "not-an-oid".into(),
        };
        assert_ne!(corrupt, checkpoint);
        f.db()
            .execute(
                "UPDATE local_context_sync_records SET closure_checkpoint=?1",
                [corrupt],
            )
            .unwrap();
        let before = f.image();
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired,
            "{spelling}"
        );
        assert_eq!(
            f.service
                .take_over_local_context_sync(&f.repo, &row)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired,
            "{spelling}"
        );
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT owner_epoch FROM local_context_sync_records",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            row.owner_epoch
        );
        assert_eq!(before, f.image());
    }
}

const CASCADE_MARKER: &str = "local_context_sync_child_cascade_v2";

fn legacy_children(f: &Fixture) {
    let mut db = f.db();
    let tx = db.transaction().unwrap();
    for table in ["local_context_sync_steps", "local_context_sync_resolutions"] {
        let sql: String = tx
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name=?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        let legacy = sql.replace(" ON DELETE CASCADE", "");
        tx.execute_batch(&format!("ALTER TABLE {table} RENAME TO fixture_child_backup; {legacy}; INSERT INTO {table} SELECT * FROM fixture_child_backup; DROP TABLE fixture_child_backup;")).unwrap();
    }
    tx.execute(
        "DELETE FROM registry_migrations WHERE name=?1",
        [CASCADE_MARKER],
    )
    .unwrap();
    tx.commit().unwrap();
}

fn assert_child_cascade(f: &Fixture, expected: &str) {
    for table in ["local_context_sync_steps", "local_context_sync_resolutions"] {
        let action: String = f
            .db()
            .query_row(
                &format!("SELECT on_delete FROM pragma_foreign_key_list('{table}')"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(action, expected, "{table}");
    }
}

#[test]
fn legacy_child_schema_upgrade_preserves_authority_and_enables_terminal_removal() {
    for terminal in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let row = f.begin(OperationId::new()).unwrap();
        children(&f, &row);
        if terminal {
            abandoned_before_effects(&f, &row);
        }
        let before = f
            .service
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap();
        legacy_children(&f);
        assert_child_cascade(&f, "NO ACTION");
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert_eq!(
            before,
            reopened
                .read_local_context_sync_record(&f.repo, row.operation_id)
                .unwrap()
        );
        assert_child_cascade(&f, "CASCADE");
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT count(*) FROM registry_migrations WHERE name=?1",
                    [CASCADE_MARKER],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        if terminal {
            assert_removed(&f);
        }
    }
}

#[test]
fn child_schema_upgrade_rolls_back_both_tables_and_marker_on_failure() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    children(&f, &row);
    legacy_children(&f);
    f.db().execute_batch(&format!("CREATE TRIGGER fail_child_upgrade BEFORE INSERT ON registry_migrations WHEN NEW.name='{CASCADE_MARKER}' BEGIN SELECT RAISE(ABORT,'fixture'); END")).unwrap();
    assert!(RepositoryService::open_at(f.data.path()).is_err());
    assert_child_cascade(&f, "NO ACTION");
    let db = f.db();
    for table in [
        "local_context_sync_records",
        "local_context_sync_steps",
        "local_context_sync_resolutions",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM registry_migrations WHERE name=?1",
            [CASCADE_MARKER],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER fail_child_upgrade").unwrap();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        row,
        reopened
            .read_local_context_sync_record(&f.repo, row.operation_id)
            .unwrap()
    );
    assert_child_cascade(&f, "CASCADE");
}

#[test]
fn child_schema_upgrade_refuses_invalid_legacy_authority_without_repair() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let row = f.begin(OperationId::new()).unwrap();
    children(&f, &row);
    legacy_children(&f);
    f.db()
        .execute(
            "UPDATE local_context_sync_steps SET ordinal='not-an-ordinal'",
            [],
        )
        .unwrap();
    assert!(RepositoryService::open_at(f.data.path()).is_err());
    assert_child_cascade(&f, "NO ACTION");
    assert_eq!(
        f.db()
            .query_row("SELECT ordinal FROM local_context_sync_steps", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "not-an-ordinal"
    );
}
