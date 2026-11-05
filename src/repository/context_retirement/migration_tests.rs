//! Exact published-schema fixtures; these are registry history, not Git authority.
use super::*;
use crate::repository::AuthoringKind;
use crate::repository::context_retirement::{
    ContextRetirementOutcome, RetireContextRequest, execution_tests::registry_image, record,
};

fn index_v2(data: &std::path::Path) -> Connection {
    let (mut db, _) = legacy(data, "released");
    record::migrate(&mut db).unwrap();
    // Build the exact pre-index v2 catalog, retaining every old history row.
    // Fresh fixture only; these objects/rows never exist in published v2.
    if recovery::has_table(&db, "context_retirement_index_claims").unwrap() {
        db.execute_batch("DROP TRIGGER retirement_index_claim_immutable; DROP TRIGGER retirement_index_claim_delete; DROP TABLE context_retirement_index_claims;").unwrap();
    }
    db.execute(
        "DELETE FROM registry_migrations WHERE name='context_retirement_index_v1'",
        [],
    )
    .unwrap();
    db
}

#[test]
fn retirement_f2_index_migration_is_additive_preserves_v2_history_and_reopens() {
    let data = tempfile::tempdir().unwrap();
    let mut db = index_v2(data.path());
    let mut before = specfix_r1_full_image(&db);
    before.remove("sqlite_master");
    before.remove("schema_version");
    before.remove("registry_migrations");
    before.remove("rowids/registry_migrations");
    record::migrate(&mut db).unwrap();
    assert!(
        recovery::has_table(&db, "context_retirement_index_claims").unwrap(),
        "durable single-use F2 claims require the additive index envelope"
    );
    let after = specfix_r1_full_image(&db);
    for (table, rows) in before {
        assert_eq!(
            after.get(&table),
            Some(&rows),
            "{table}: old values/rowids must be unchanged"
        );
    }
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_index_claims",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    record::audit_connection(&db).unwrap();
    let before = specfix_r1_full_image(&db);
    drop(RepositoryService::open_at(data.path()).unwrap());
    assert_eq!(before, specfix_r1_full_image(&db));
}

#[test]
fn retirement_f2_index_migration_each_new_boundary_rolls_back_exact_v2() {
    for point in 8..=11 {
        let data = tempfile::tempdir().unwrap();
        let mut db = index_v2(data.path());
        let before = specfix_r1_full_image(&db);
        record::fail_migration_at(point);
        assert!(
            record::migrate(&mut db).is_err(),
            "point {point}: missing index migration fault boundary"
        );
        assert_eq!(
            before,
            specfix_r1_full_image(&db),
            "point {point}: no partial schema/marker/data/rowid changes"
        );
        assert!(RepositoryService::open_at(data.path()).is_ok());
    }
}

#[test]
fn retirement_f2_index_migration_unknown_partial_and_dependency_envelopes_preserve_evidence() {
    for case in [
        "unmarked_table",
        "unknown_marker",
        "old_schema_new_marker",
        "foreign_child",
        "foreign_trigger",
        "missing_table",
        "missing_marker",
        "substituted_trigger",
    ] {
        let data = tempfile::tempdir().unwrap();
        let mut db = index_v2(data.path());
        if matches!(
            case,
            "foreign_trigger" | "missing_table" | "missing_marker" | "substituted_trigger"
        ) {
            record::migrate(&mut db).unwrap();
        }
        match case {
            "unmarked_table"=>db.execute_batch("CREATE TABLE context_retirement_index_claims(payload TEXT); INSERT INTO context_retirement_index_claims VALUES('must-keep');").unwrap(),
            "unknown_marker"=>{db.execute("INSERT INTO registry_migrations VALUES('context_retirement_index_v99')",[]).unwrap();},
            "old_schema_new_marker"=>{db.execute("INSERT INTO registry_migrations VALUES('context_retirement_index_v1')",[]).unwrap();},
            "foreign_child"=>db.execute_batch("CREATE TABLE foreign_claim_history(id INTEGER PRIMARY KEY,op TEXT,attempt INTEGER,payload TEXT,FOREIGN KEY(op,attempt) REFERENCES context_retirement_index_claims(operation_ulid,request_attempt) ON DELETE CASCADE); CREATE TABLE foreign_claim_payload(id INTEGER PRIMARY KEY,payload TEXT); INSERT INTO foreign_claim_payload VALUES(91,'must-keep');").unwrap(),
            "foreign_trigger"=>db.execute_batch("CREATE TRIGGER foreign_claim_guard BEFORE DELETE ON context_retirement_index_claims BEGIN SELECT 1; END;").unwrap(),
            "missing_table"=>db.execute_batch("DROP TABLE context_retirement_index_claims;").unwrap(),
            "missing_marker"=>{db.execute("DELETE FROM registry_migrations WHERE name='context_retirement_index_v1'",[]).unwrap();},
            _=>db.execute_batch("DROP TRIGGER retirement_index_claim_immutable; CREATE TRIGGER retirement_index_claim_immutable BEFORE UPDATE ON context_retirement_index_claims BEGIN SELECT 1; END;").unwrap(),
        }
        let before = specfix_r1_full_image(&db);
        assert!(RepositoryService::open_at(data.path()).is_err(), "{case}");
        assert_eq!(
            before,
            specfix_r1_full_image(&db),
            "{case}: no widening/copy/evidence loss"
        );
    }
}

fn legacy(data: &std::path::Path, case: &str) -> (Connection, OperationId) {
    drop(RepositoryService::open_at(data).unwrap());
    git2::Repository::init(data).unwrap();
    let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
    // Empty fixture installation, with normal FK/CHECK enforcement throughout.
    let triggers = db
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='trigger' AND (name LIKE 'retirement_%')",
        )
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for name in triggers {
        db.execute_batch(&format!("DROP TRIGGER {name}")).unwrap();
    }
    for name in [
        "context_retirement_index_claims",
        "context_retirement_absence_receipts",
        "context_retirement_worktree_resumptions",
        "retirement_observation_pin_releases",
        "retirement_observation_pins",
        "context_retirement_effect_intents",
        "retirement_observation_receipts",
        "retirement_advertisement_provenance",
        "context_retirement_reentries",
        "context_retirement_attempts",
        "context_retirement_records",
    ] {
        db.execute_batch(&format!("DROP TABLE {name}")).unwrap();
    }
    db.execute(
        "DELETE FROM registry_migrations WHERE name LIKE 'context_retirement_%'",
        [],
    )
    .unwrap();
    for (_, _, sql) in record::published_v1_objects().unwrap() {
        db.execute_batch(&sql).unwrap();
    }
    db.execute(
        "INSERT INTO registry_migrations VALUES('context_retirement_v1')",
        [],
    )
    .unwrap();
    let operation = OperationId::new();
    let item = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,?1,1,'accessible',1)",[data.to_str()]).unwrap();
    db.execute("INSERT INTO operation_records(id,repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(17,1,?1,?2,'retire_context',?3,'created',123)",params![data.to_str(),operation.to_string(),format!("authoring-context-v1/ticket/{item}/Retire")]).unwrap();
    let observing = matches!(case, "pinned" | "possible" | "branch_complete" | "released");
    db.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,remote_name,primary_branch,configuration_generation,endpoint_digest,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) VALUES(?1,17,1,1,'ticket',?2,CASE WHEN ?3 THEN 'branch_only' ELSE 'absent' END,?4,CASE WHEN ?3 THEN 'origin' END,CASE WHEN ?3 THEN 'main' END,CASE WHEN ?3 THEN 0 END,CASE WHEN ?3 THEN zeroblob(32) END,1,0,'absent',0,1,'selected','none','none','none')",params![operation.to_string(),item,observing,observing.then(||"f".repeat(40))]).unwrap();
    db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) VALUES(?1,1,1,'active')",[operation.to_string()]).unwrap();
    if observing {
        bound_observation(&db, operation);
    }
    if matches!(case, "possible" | "branch_complete") {
        db.execute("UPDATE context_retirement_records SET initial_primary_oid=?1,phase='branch_deleting',branch_removal='possible'",["e".repeat(40)]).unwrap();
        db.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) VALUES(?1,1,'branch',1,?2,'folded')",params![operation.to_string(),"e".repeat(40)]).unwrap();
    }
    if matches!(case, "complete" | "index" | "branch_complete") {
        let branch = case == "branch_complete";
        if branch {
            db.execute(
                "UPDATE context_retirement_effect_intents SET observation='removed'",
                [],
            )
            .unwrap();
        }
        db.execute("UPDATE context_retirement_records SET phase='index_pending',absence_basis=?1,materialization_settled=1,branch_removal=?2",params![if branch{"owned_intent"}else{"initial"},if branch{"observed"}else{"none"}]).unwrap();
        db.execute(
            "UPDATE operation_records SET state='authoritative_observed',completed_step=?1",
            [if branch {
                "context_branch_released"
            } else {
                "context_retirement_absent"
            }],
        )
        .unwrap();
        if case != "index" {
            let report = if branch {
                crate::repository::ContextRetirementReport::retired(
                    crate::repository::RetirementRemoval::None,
                    crate::repository::RetirementRemoval::Observed,
                )
            } else {
                crate::repository::ContextRetirementReport::absent()
            };
            db.execute("UPDATE context_retirement_records SET phase='complete',final_code=?1,final_result=?2",params![report.code.as_str(),report.encode().unwrap()]).unwrap();
        }
    }
    if matches!(case, "refused" | "released") {
        db.execute("UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=?1",[REFUSAL]).unwrap();
    }
    if matches!(
        case,
        "complete" | "refused" | "branch_complete" | "released"
    ) {
        if observing {
            db.execute_batch("INSERT INTO retirement_observation_pin_releases(rowid,provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id,reason) SELECT 71,provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id,'consumer_terminal' FROM retirement_observation_pins; DELETE FROM retirement_observation_pins;").unwrap();
        }
        db.execute_batch("UPDATE context_retirement_attempts SET phase='ended'; UPDATE operation_records SET state='completed' WHERE action='retire_context';").unwrap();
    }
    (db, operation)
}

#[test]
fn retirement_migration_published_v1_histories_upgrade_once_without_rewriting_rows() {
    for case in [
        "refused",
        "complete",
        "index",
        "pinned",
        "possible",
        "branch_complete",
        "released",
    ] {
        let data = tempfile::tempdir().unwrap();
        let (db, operation) = legacy(data.path(), case);
        let mut before = registry_image(&db);
        before.remove("sqlite_master");
        before.remove("schema_version");
        before.remove("registry_migrations");
        let release_rows: Vec<(i64, String)> = db
            .prepare("SELECT rowid,reason FROM retirement_observation_pin_releases")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM registry_migrations WHERE name='context_retirement_v2'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1,
            "{case}: versioned upgrade required"
        );
        let after = registry_image(&db);
        for (table, rows) in before {
            assert_eq!(after.get(&table), Some(&rows), "{case}/{table}");
        }
        for table in [
            "context_retirement_reentries",
            "context_retirement_absence_receipts",
            "context_retirement_worktree_resumptions",
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        assert_eq!(
            db.prepare("SELECT rowid,reason FROM retirement_observation_pin_releases")
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<Result<Vec<(i64, String)>, _>>()
                .unwrap(),
            release_rows
        );
        let current = registry_image(&db);
        drop(RepositoryService::open_at(data.path()).unwrap());
        assert_eq!(registry_image(&db), current, "{case}: idempotent");
        if matches!(
            case,
            "refused" | "complete" | "branch_complete" | "released"
        ) {
            let request = RetireContextRequest {
                root: data.path().into(),
                kind: AuthoringKind::Ticket,
                item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
                operation_id: operation,
            };
            assert!(matches!(
                service.retire_context(request),
                Ok(ContextRetirementOutcome::Final(_))
            ));
            assert_eq!(registry_image(&db), current);
        }
    }
}

#[test]
fn retirement_migration_rejects_mixed_partial_substituted_and_laundered_old_release() {
    for case in [
        "new_marker_old_schema",
        "unknown_marker",
        "missing_table",
        "missing_index",
        "extra_column",
        "missing_trigger",
        "substituted_trigger",
        "mixed_table",
        "forged_release",
        "bad_row",
    ] {
        let data = tempfile::tempdir().unwrap();
        let (db, _) = legacy(data.path(), "released");
        match case {
            "new_marker_old_schema" => {
                db.execute(
                    "INSERT INTO registry_migrations VALUES('context_retirement_v2')",
                    [],
                )
                .unwrap();
            }
            "unknown_marker" => {
                db.execute(
                    "INSERT INTO registry_migrations VALUES('context_retirement_v3')",
                    [],
                )
                .unwrap();
            }
            "missing_table" => {
                db.execute_batch("DROP TABLE context_retirement_effect_intents")
                    .unwrap();
            }
            "missing_index" => {
                db.execute_batch("DROP INDEX retirement_pinned_batch")
                    .unwrap();
            }
            "extra_column" => {
                db.execute_batch(
                    "ALTER TABLE context_retirement_records ADD COLUMN substituted TEXT",
                )
                .unwrap();
            }
            "missing_trigger" => {
                db.execute_batch("DROP TRIGGER retirement_release_immutable")
                    .unwrap();
            }
            "substituted_trigger" => {
                db.execute_batch("DROP TRIGGER retirement_release_immutable; CREATE TRIGGER retirement_release_immutable BEFORE UPDATE ON retirement_observation_pin_releases BEGIN SELECT 1; END;").unwrap();
            }
            "mixed_table" => {
                db.execute_batch("CREATE TABLE context_retirement_reentries(substituted TEXT)")
                    .unwrap();
            }
            "forged_release" => {
                let sql: String = db
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name='retirement_release_immutable'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                db.execute_batch("DROP TRIGGER retirement_release_immutable; PRAGMA ignore_check_constraints=ON; UPDATE retirement_observation_pin_releases SET reason='consumer_fenced'; PRAGMA ignore_check_constraints=OFF;").unwrap();
                db.execute_batch(&sql).unwrap();
            }
            _ => {
                db.execute("UPDATE operation_records SET completed_step='context_branch_released' WHERE action='retire_context'",[]).unwrap();
            }
        }
        let before = registry_image(&db);
        assert!(
            matches!(RepositoryService::open_at(data.path()),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired),
            "{case}"
        );
        assert_eq!(
            registry_image(&db),
            before,
            "{case}: schema_version, rows and schema must all roll back"
        );
    }
}

#[test]
fn retirement_migration_each_upgrade_boundary_rolls_back_schema_rows_and_rowids() {
    for point in 1..=7 {
        let data = tempfile::tempdir().unwrap();
        let (mut db, _) = legacy(data.path(), "released");
        let before = registry_image(&db);
        record::fail_migration_at(point);
        assert!(record::migrate(&mut db).is_err(), "point {point}");
        assert_eq!(
            registry_image(&db),
            before,
            "point {point}: rollback includes schema_version"
        );
        assert!(
            RepositoryService::open_at(data.path()).is_ok(),
            "point {point}: clean retry"
        );
    }
}

#[test]
fn retirement_migration_current_envelope_missing_marker_is_not_legacy_authority() {
    for marker in ["context_retirement_v1", "context_retirement_v2"] {
        let data = tempfile::tempdir().unwrap();
        drop(RepositoryService::open_at(data.path()).unwrap());
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute("DELETE FROM registry_migrations WHERE name=?1", [marker])
            .unwrap();
        let before = registry_image(&db);
        assert!(record::audit_connection(&db).is_err(), "missing {marker}");
        assert!(RepositoryService::open_at(data.path()).is_err());
        assert_eq!(registry_image(&db), before);
    }
}

#[test]
fn retirement_specfix_r1_rejects_unknown_release_attachments_without_evidence_loss() {
    let mut violations = Vec::new();
    for current in [false, true] {
        for case in ["trigger", "index", "both", "case_trigger", "case_index"] {
            let data = tempfile::tempdir().unwrap();
            let (db, _) = legacy(data.path(), "released");
            if current {
                drop(RepositoryService::open_at(data.path()).unwrap());
            }
            let parent = if case.starts_with("case_") {
                "RETIREMENT_OBSERVATION_PIN_RELEASES"
            } else {
                "retirement_observation_pin_releases"
            };
            if matches!(case, "trigger" | "both" | "case_trigger") {
                db.execute_batch(&format!("CREATE TRIGGER ForeignReleaseGuard BEFORE DELETE ON \"{parent}\" BEGIN SELECT 1; END;")).unwrap();
            }
            if matches!(case, "index" | "both" | "case_index") {
                db.execute_batch(&format!(
                    "CREATE INDEX ForeignReleaseIndex ON \"{parent}\"(reason);"
                ))
                .unwrap();
            }
            let before = specfix_r1_full_image(&db);
            let opened = RepositoryService::open_at(data.path());
            if !matches!(opened, Err(ref e) if e.kind == RepositoryErrorKind::RecoveryRequired)
                || specfix_r1_full_image(&db) != before
            {
                violations.push((current, case));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "accepted or changed attached evidence: {violations:?}"
    );
}

#[test]
fn retirement_specfix_r1_rejects_unknown_incoming_release_fks_before_cascade() {
    let mut violations = Vec::new();
    for current in [false, true] {
        for action in ["CASCADE", "NO ACTION", "RESTRICT"] {
            for populated in [false, true] {
                let data = tempfile::tempdir().unwrap();
                let (db, _) = legacy(data.path(), "released");
                if current {
                    drop(RepositoryService::open_at(data.path()).unwrap());
                }
                db.execute_batch(&format!("CREATE TABLE foreign_release_history(id INTEGER PRIMARY KEY, provenance_id INTEGER, pin_kind TEXT, holder_operation_ulid TEXT, holder_epoch INTEGER, payload TEXT, FOREIGN KEY(provenance_id,pin_kind,holder_operation_ulid,holder_epoch) REFERENCES \"RETIREMENT_OBSERVATION_PIN_RELEASES\"(provenance_id,pin_kind,holder_operation_ulid,holder_epoch) ON DELETE {action});")).unwrap();
                if populated {
                    db.execute_batch("INSERT INTO foreign_release_history SELECT 91,provenance_id,pin_kind,holder_operation_ulid,holder_epoch,'foreign-row-91-sentinel' FROM retirement_observation_pin_releases;").unwrap();
                }
                let before = specfix_r1_full_image(&db);
                let opened = RepositoryService::open_at(data.path());
                if !matches!(opened, Err(ref e) if e.kind == RepositoryErrorKind::RecoveryRequired)
                    || specfix_r1_full_image(&db) != before
                {
                    violations.push((current, action, populated));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "accepted or changed incoming dependencies: {violations:?}"
    );
}

fn specfix_r1_full_image(
    db: &Connection,
) -> std::collections::BTreeMap<String, Vec<Vec<rusqlite::types::Value>>> {
    let mut image = registry_image(db);
    let tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for table in tables {
        let quoted = table.replace('"', "\"\"");
        let rows = db
            .prepare(&format!("SELECT rowid,* FROM \"{quoted}\" ORDER BY rowid"))
            .unwrap()
            .query_map([], |r| {
                (0..r.as_ref().column_count()).map(|i| r.get(i)).collect()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        image.insert(format!("rowids/{table}"), rows);
    }
    image
}

#[test]
fn retirement_specfix_r1_unrelated_schema_is_not_part_of_release_closure() {
    let data = tempfile::tempdir().unwrap();
    let (db, _) = legacy(data.path(), "released");
    db.execute_batch("CREATE TABLE unrelated_history(id INTEGER PRIMARY KEY,payload TEXT); INSERT INTO unrelated_history VALUES(91,'keep'); CREATE INDEX unrelated_index ON unrelated_history(payload); CREATE TRIGGER unrelated_guard BEFORE DELETE ON unrelated_history BEGIN SELECT RAISE(ABORT,'keep'); END;").unwrap();
    drop(RepositoryService::open_at(data.path()).unwrap());
    let before = registry_image(&db);
    drop(RepositoryService::open_at(data.path()).unwrap());
    assert_eq!(registry_image(&db), before);
}
