use crate::repository::{
    AbandonOperationRequest, OperationId, REGISTRY_FILE, RemoteOperationTarget, RemoteRefPlan,
    RepositoryErrorKind, RepositoryOperation, RepositoryService, recovery,
};
use rusqlite::{Connection, params};

const REFUSAL: &str = r#"{"version":1,"code":"external_change","outcome":"refused","worktree_removal":"none","branch_removal":"none","materialization_settled":false}"#;

const TABLES: [&str; 7] = [
    "context_retirement_records",
    "context_retirement_attempts",
    "context_retirement_effect_intents",
    "retirement_advertisement_provenance",
    "retirement_observation_receipts",
    "retirement_observation_pins",
    "retirement_observation_pin_releases",
];

#[test]
fn retirement_quality_q2_partial_worktree_step_requires_correlated_effect() {
    let mut accepted = Vec::new();
    for case in [
        "valid",
        "foreign_step",
        "missing_step",
        "possible_only",
        "missing_observation",
    ] {
        let data = tempfile::tempdir().unwrap();
        drop(RepositoryService::open_at(data.path()).unwrap());
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        let operation = OperationId::new();
        db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,?1,1,'accessible',0)", [data.path().to_str()]).unwrap();
        db.execute("INSERT INTO operation_records(id,repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(1,1,?2,?1,'retire_context','authoring-context-v1/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV/Retire','created',1)", params![operation.to_string(),data.path().to_str()]).unwrap();
        db.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,instance_nonce,instance_verified,remote_name,primary_branch,configuration_generation,endpoint_digest,selection_epoch,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) VALUES(?1,1,1,1,'ticket','01ARZ3NDEKTSV4RRFFQ69G5FAV','present',?2,?3,1,'origin','main',0,zeroblob(32),1,'absent',0,1,'selected','none','none','none')", params![operation.to_string(),"1".repeat(40),OperationId::new().to_string()]).unwrap();
        db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) VALUES(?1,1,1,'active')", [operation.to_string()]).unwrap();
        bound_observation(&db, operation);
        assert!(RepositoryService::open_at(data.path()).is_ok());
        db.execute("UPDATE context_retirement_records SET phase='worktree_removing',worktree_removal='possible'", []).unwrap();
        db.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) VALUES(?1,1,'worktree',1,?2,'ancestor')", params![operation.to_string(),"1".repeat(40)]).unwrap();
        assert!(
            RepositoryService::open_at(data.path()).is_ok(),
            "a recorded intent alone remains possible, not a completed worktree release"
        );
        if case != "missing_observation" {
            db.execute(
                "UPDATE context_retirement_effect_intents SET observation='removed'",
                [],
            )
            .unwrap();
        }
        db.execute_batch("UPDATE context_retirement_records SET phase='worktree_removed',worktree_removal='observed'; UPDATE operation_records SET state='context_worktree_released',completed_step='context_worktree_released';").unwrap();
        match case {
            "foreign_step" => {
                db.execute(
                    "UPDATE operation_records SET completed_step='publication_committed'",
                    [],
                )
                .unwrap();
            }
            "missing_step" => {
                db.execute("UPDATE operation_records SET completed_step=NULL", [])
                    .unwrap();
            }
            "possible_only" => {
                db.execute(
                    "UPDATE context_retirement_records SET worktree_removal='possible'",
                    [],
                )
                .unwrap();
            }
            _ => {}
        }
        let before = super::execution_tests::registry_image(&db);
        let opened = RepositoryService::open_at(data.path());
        if case == "valid" {
            assert!(
                opened.is_ok(),
                "a truthful partial removal must retain its noncheckpoint step"
            );
        } else if !matches!(opened, Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired) {
            accepted.push(case);
        }
        assert_eq!(
            super::execution_tests::registry_image(&db),
            before,
            "{case}: no repair or schema rewrite"
        );
    }
    assert!(
        accepted.is_empty(),
        "uncorrelated partial authority accepted: {accepted:?}"
    );
}

#[test]
fn retirement_registry_installs_the_complete_envelope_once() {
    let data = tempfile::tempdir().unwrap();
    drop(RepositoryService::open_at(data.path()).unwrap());
    let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let installed: Vec<_> = TABLES
        .into_iter()
        .filter(|name| {
            db.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [name],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
        })
        .collect();
    assert_eq!(
        installed, TABLES,
        "retirement authority must be a complete envelope"
    );
    let version: i64 = db
        .query_row("PRAGMA schema_version", [], |r| r.get(0))
        .unwrap();
    drop(RepositoryService::open_at(data.path()).unwrap());
    assert_eq!(
        db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        version
    );
}

#[test]
fn retirement_registry_refuses_partial_unmarked_and_orphan_authority() {
    for case in ["partial", "orphan"] {
        let data = tempfile::tempdir().unwrap();
        drop(RepositoryService::open_at(data.path()).unwrap());
        let db = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        match case {
            "partial" => {
                // Simulate an unmarked substituted envelope, not a genuine old registry.
                db.execute_batch("DROP TABLE IF EXISTS context_retirement_records; DELETE FROM registry_migrations WHERE name='context_retirement_v1'; CREATE TABLE context_retirement_records(substituted TEXT);").unwrap();
            }
            "orphan" => {
                db.execute("INSERT INTO operation_records(root_path,operation_ulid,action,target,state,observed_at) VALUES('/retirement-fixture',?1,'retire_context','authoring-context-v1/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV/Retire','created',1)", [OperationId::new().to_string()]).unwrap();
            }
            _ => unreachable!(),
        }
        let before: i64 = db
            .query_row("PRAGMA schema_version", [], |r| r.get(0))
            .unwrap();
        let opened = RepositoryService::open_at(data.path());
        assert!(
            matches!(opened, Err(ref error) if error.kind == RepositoryErrorKind::RecoveryRequired),
            "{case} must not be accepted/reconstructed"
        );
        assert_eq!(
            db.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            before,
            "{case}: refusal must roll back"
        );
    }
}

fn selected(data: &std::path::Path, operation: &str, item: &str) -> Connection {
    selected_for(data, operation, item, false)
}

fn selected_for(
    data: &std::path::Path,
    operation: &str,
    item: &str,
    observing: bool,
) -> Connection {
    drop(RepositoryService::open_at(data).unwrap());
    let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
    db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,?1,1,'accessible',0)", [data.to_str().unwrap()]).unwrap();
    db.execute("INSERT INTO operation_records(id,repository_id,root_path,operation_ulid,action,target,state,observed_at) VALUES(1,1,?3,?1,'retire_context',?2,'created',1)", params![operation,format!("authoring-context-v1/ticket/{item}/Retire"),data.to_str().unwrap()]).unwrap();
    db.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,expected_tip,remote_name,primary_branch,configuration_generation,endpoint_digest,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) VALUES(?1,1,1,1,'ticket',?2,CASE WHEN ?3 THEN 'branch_only' ELSE 'absent' END,?4,CASE WHEN ?3 THEN 'origin' END,CASE WHEN ?3 THEN 'main' END,CASE WHEN ?3 THEN 0 END,CASE WHEN ?3 THEN zeroblob(32) END,1,0,'absent',0,1,'selected','none','none','none')", params![operation,item,observing,observing.then(|| "1".repeat(40))]).unwrap();
    db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) VALUES(?1,1,1,'active')", [operation]).unwrap();
    db
}

#[test]
fn retirement_registry_audits_identity_executor_and_authoritative_phase() {
    let mut accepted = Vec::new();
    for case in [
        "valid",
        "operation_id",
        "item_id",
        "zero_oid",
        "lost_executor",
        "premature_handoff",
        "terminal_pending",
    ] {
        let data = tempfile::tempdir().unwrap();
        let op = OperationId::new().to_string();
        let db = selected(data.path(), &op, "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        // A corrupt fixture writer alone bypasses constraints so it can replace
        // a parent identity before updating its matching child. Otherwise the
        // immediate FK refuses the injection before startup can audit anything.
        // Service open is a separate connection with normal enforcement.
        db.execute_batch("PRAGMA foreign_keys=OFF; PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        match case {
            "operation_id" | "item_id" => {
                // Drop only the immutable-input trigger to inject existing data
                // corruption, then restore the exact original envelope.
                let sql: String = db
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name='retirement_record_identity'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                db.execute_batch("DROP TRIGGER retirement_record_identity")
                    .unwrap();
                if case == "operation_id" {
                    db.execute(
                        "UPDATE context_retirement_records SET operation_ulid='invalid'",
                        [],
                    )
                    .unwrap();
                    db.execute("UPDATE operation_records SET operation_ulid='invalid'", [])
                        .unwrap();
                    db.execute("DELETE FROM context_retirement_attempts", [])
                        .unwrap();
                    db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) VALUES('invalid',1,1,'active')", []).unwrap();
                } else {
                    db.execute(
                        "UPDATE context_retirement_records SET item_id='invalid'",
                        [],
                    )
                    .unwrap();
                    db.execute("UPDATE operation_records SET target='authoring-context-v1/ticket/invalid/Retire'", []).unwrap();
                }
                db.execute_batch(&sql).unwrap();
            }
            "zero_oid" => {
                db.execute(
                    "UPDATE context_retirement_records SET initial_primary_oid=?1",
                    ["0".repeat(40)],
                )
                .unwrap();
            }
            "lost_executor" => {
                db.execute("DELETE FROM context_retirement_attempts", [])
                    .unwrap();
            }
            "premature_handoff" => {
                db.execute("UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released'", []).unwrap();
            }
            "terminal_pending" => {
                db.execute("UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=?1", [REFUSAL]).unwrap();
            }
            "valid" => {}
            _ => unreachable!(),
        }
        db.execute_batch("PRAGMA ignore_check_constraints=OFF; PRAGMA foreign_keys=ON;")
            .unwrap();
        let result = RepositoryService::open_at(data.path());
        if case == "valid" {
            assert!(result.is_ok(), "valid pending selection remains readable");
        } else if !matches!(result, Err(ref e) if e.kind == RepositoryErrorKind::RecoveryRequired) {
            accepted.push(case);
        }
    }
    assert!(
        accepted.is_empty(),
        "corrupt retirement authority accepted: {accepted:?}"
    );
}

#[test]
fn retirement_generic_abandon_preserves_pending_and_terminal_identity() {
    for terminal in [false, true] {
        let data = tempfile::tempdir().unwrap();
        let operation = OperationId::new();
        let db = selected(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        if terminal {
            db.execute_batch("BEGIN IMMEDIATE;
              UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=json_object('version',1,'code','external_change','outcome','refused','worktree_removal','none','branch_removal','none','materialization_settled',json('false'));
              UPDATE context_retirement_attempts SET phase='ended';
              UPDATE operation_records SET state='completed';
              COMMIT;").unwrap();
        }
        let service = RepositoryService::open_at(data.path()).unwrap();
        let before: (String, Option<String>) = db
            .query_row(
                "SELECT state,completed_step FROM operation_records WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let outcome = service.abandon_operation(AbandonOperationRequest {
            root: data.path().into(),
            operation_id: operation,
            expected: None,
        });
        assert!(
            matches!(outcome, Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired),
            "terminal={terminal}: generic abandon must not release or relabel retirement identity: {outcome:?}"
        );
        let after = db
            .query_row(
                "SELECT state,completed_step FROM operation_records WHERE id=1",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .unwrap();
        assert_eq!(before, after);
        assert!(
            service
                .abandonable_operation(data.path(), operation)
                .is_err(),
            "retirement needs its guarded preview, not ordinary completed/no-op handling"
        );
    }
}

fn bound_observation(db: &Connection, operation: OperationId) {
    let observer = OperationId::new().to_string();
    db.execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main',endpoint_digest=zeroblob(32) WHERE repository_id=1;
      INSERT INTO remote_observation_batches(id,repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current) VALUES(1,1,'origin','main',0,1,1);").unwrap();
    db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind) VALUES(1,?1,0,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','manual','completed','after_batch_commit','completed',1,1,'observe')", [&observer]).unwrap();
    db.execute(
        "UPDATE context_retirement_attempts SET observer_operation_ulid=?1,observer_owner_epoch=0",
        [&observer],
    )
    .unwrap();
    db.execute("INSERT INTO retirement_advertisement_provenance(id,repository_id,source_operation_ulid,source_owner_epoch,observation_batch_id,observed_at,configuration_generation,remote_name,primary_branch,endpoint_digest,execution_kind,execution_parent_ulid,execution_parent_epoch) VALUES(1,1,?1,0,1,1,0,'origin','main',zeroblob(32),'local_parent',?2,1)", params![observer,operation.to_string()]).unwrap();
    db.execute("INSERT INTO retirement_observation_receipts(id,provenance_id,repository_id,consumer_kind,consumer_operation_ulid,consumer_attempt_epoch,configuration_generation) VALUES(1,1,1,'single',?1,1,0)", [operation.to_string()]).unwrap();
    db.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id) VALUES(1,'consumer_attempt',?1,1,1,1,1)", [operation.to_string()]).unwrap();
}

#[test]
fn retirement_registry_correlates_provenance_binding_and_pin_instead_of_current_guess() {
    let mut accepted = Vec::new();
    for case in [
        "valid",
        "source_generation",
        "pin_batch",
        "consumer_identity",
        "missing_pin",
    ] {
        let data = tempfile::tempdir().unwrap();
        let operation = OperationId::new();
        let db = selected_for(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            true,
        );
        bound_observation(&db, operation);
        if case != "valid" {
            let trigger = match case {
                "source_generation" => "retirement_provenance_immutable",
                "pin_batch" => "retirement_pin_immutable",
                "consumer_identity" => "retirement_binding_immutable",
                "missing_pin" => "retirement_pin_release_required",
                _ => unreachable!(),
            };
            let sql: String = db
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE name=?1",
                    [trigger],
                    |r| r.get(0),
                )
                .unwrap();
            db.execute_batch(&format!("DROP TRIGGER {trigger}"))
                .unwrap();
            match case {
                "source_generation" => {
                    db.execute(
                        "UPDATE retirement_advertisement_provenance SET configuration_generation=1",
                        [],
                    )
                    .unwrap();
                }
                "pin_batch" => {
                    db.execute_batch("INSERT INTO remote_observation_batches(id,repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current) VALUES(2,1,'origin','main',0,2,0); UPDATE retirement_observation_pins SET observation_batch_id=2;").unwrap();
                }
                "consumer_identity" => {
                    db.execute(
                        "UPDATE retirement_observation_receipts SET consumer_operation_ulid=?1",
                        [OperationId::new().to_string()],
                    )
                    .unwrap();
                }
                "missing_pin" => {
                    db.execute("DELETE FROM retirement_observation_pins", [])
                        .unwrap();
                }
                _ => unreachable!(),
            }
            db.execute_batch(&sql).unwrap();
        }
        let result = RepositoryService::open_at(data.path());
        if case == "valid" {
            assert!(
                result.is_ok(),
                "a completed observer can retain a bound consumer: {:?}",
                result.as_ref().err()
            );
        } else if !matches!(result, Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired) {
            accepted.push(case);
        }
    }
    assert!(
        accepted.is_empty(),
        "uncorrelated observation authority accepted: {accepted:?}"
    );
}

#[test]
fn retirement_schema_refuses_noncanonical_capture_before_startup_audit() {
    let data = tempfile::tempdir().unwrap();
    let operation = OperationId::new();
    let db = selected(
        data.path(),
        &operation.to_string(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
    );
    assert!(
        db.execute(
            "UPDATE context_retirement_records SET initial_primary_oid=?1",
            ["0".repeat(40)]
        )
        .is_err(),
        "zero OID must fail the stored envelope itself"
    );
    db.execute("INSERT INTO operation_records(id,repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT 2,repository_id,root_path,'invalid',action,target,state,observed_at FROM operation_records WHERE id=1", []).unwrap();
    assert!(db.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) SELECT 'invalid',2,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis FROM context_retirement_records WHERE local_record_id=1", []).is_err(), "a malformed immutable operation ID must never enter the retirement table");
}

#[test]
fn retirement_current_envelope_migration_does_not_take_a_write_lock() {
    let data = tempfile::tempdir().unwrap();
    drop(RepositoryService::open_at(data.path()).unwrap());
    let holder = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    let mut reader = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    reader.busy_timeout(std::time::Duration::ZERO).unwrap();
    let migration = super::record::migrate(&mut reader);
    holder.execute_batch("ROLLBACK").unwrap();
    assert!(
        migration.is_ok(),
        "current envelope validation is read-only; full retained-row audit belongs to startup: {migration:?}"
    );
}

#[test]
fn retirement_selected_identity_cannot_be_borrowed_for_refresh() {
    let data = tempfile::tempdir().unwrap();
    let operation = OperationId::new();
    let mut db = selected(
        data.path(),
        &operation.to_string(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
    );
    let preflight = recovery::require_refresh_can_begin(
        &db,
        data.path(),
        RepositoryOperation::RefreshRepository,
        operation,
    );
    assert!(
        preflight.is_err(),
        "selected retirement is not an authoritative index handoff"
    );
    let begin = recovery::begin_or_reconcile_operation(
        &mut db,
        data.path(),
        RepositoryOperation::RefreshRepository,
        operation,
        "",
    );
    assert!(
        matches!(begin, Err(ref e) if e.kind == RepositoryErrorKind::RecoveryRequired),
        "generic refresh cannot bypass the same gate"
    );
    let state: String = db
        .query_row("SELECT state FROM operation_records WHERE id=1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(state, "created");
}

#[test]
fn retirement_reserved_observer_cannot_be_replayed_restarted_or_used_as_local_id() {
    let data = tempfile::tempdir().unwrap();
    let operation = OperationId::new();
    let mut db = selected_for(
        data.path(),
        &operation.to_string(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        true,
    );
    bound_observation(&db, operation);
    let observer: String = db
        .query_row(
            "SELECT observer_operation_ulid FROM context_retirement_attempts",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let observer = OperationId::parse(&observer).unwrap();
    // Finish only the inert fixture's single consumer so unrelated-pending
    // exclusion cannot accidentally make these reserved-ID assertions pass.
    db.execute_batch("BEGIN IMMEDIATE;
      INSERT INTO retirement_observation_pin_releases SELECT provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id,'consumer_terminal' FROM retirement_observation_pins;
      DELETE FROM retirement_observation_pins;
      UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=json_object('version',1,'code','external_change','outcome','refused','worktree_removal','none','branch_removal','none','materialization_settled',json('false'));
      UPDATE context_retirement_attempts SET phase='ended';
      UPDATE operation_records SET state='completed'; COMMIT;").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let target = RemoteOperationTarget::for_poll(&plan);
    let reserve = service.reserve_remote_operation_with_priority(
        data.path(),
        observer,
        &target,
        crate::repository::RemoteOperationPriority::Manual,
    );
    assert!(
        reserve.is_err(),
        "public completed-observation replay must not adopt a reserved retirement observer"
    );
    let restart = service.restart_remote_observation(data.path(), observer, &target);
    assert!(
        restart.is_err(),
        "public restart must not steal a retirement observer"
    );
    let local = recovery::begin_or_reconcile_operation(
        &mut db,
        data.path(),
        RepositoryOperation::SaveTicket,
        observer,
        "unrelated",
    );
    assert!(
        local.is_err(),
        "observer identity stays reserved after its parent ends"
    );
}

#[test]
fn retirement_refresh_requires_exact_due_handoff_and_intact_companion() {
    for case in ["due", "wrong_root", "wrong_step", "missing_companion"] {
        let data = tempfile::tempdir().unwrap();
        let operation = OperationId::new();
        let mut db = selected(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        db.execute_batch("BEGIN IMMEDIATE;
          UPDATE context_retirement_records SET phase='index_pending',absence_basis='initial',materialization_settled=1;
          UPDATE operation_records SET state='authoritative_observed',completed_step='context_retirement_absent'; COMMIT;").unwrap();
        match case {
            "wrong_step" => {
                db.execute(
                    "UPDATE operation_records SET completed_step='authoritative_observed'",
                    [],
                )
                .unwrap();
            }
            "missing_companion" => {
                let trigger: String = db
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name='retirement_record_delete'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                db.execute_batch("PRAGMA foreign_keys=OFF; DROP TRIGGER retirement_record_delete; DELETE FROM context_retirement_records;").unwrap();
                db.execute_batch(&trigger).unwrap();
                db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
            }
            _ => {}
        }
        let root = if case == "wrong_root" {
            data.path().join("another-root")
        } else {
            data.path().to_owned()
        };
        let preflight = recovery::require_refresh_can_begin(
            &db,
            &root,
            RepositoryOperation::RefreshRepository,
            operation,
        );
        let begin = recovery::begin_or_reconcile_operation(
            &mut db,
            &root,
            RepositoryOperation::RefreshRepository,
            operation,
            "",
        );
        assert_eq!(preflight.is_ok(), case == "due", "{case}: preflight");
        assert_eq!(begin.is_ok(), case == "due", "{case}: transactional begin");
    }
}

#[test]
fn retirement_old_generation_pin_retains_exact_batch_and_children_through_reopen() {
    let data = tempfile::tempdir().unwrap();
    let operation = OperationId::new();
    let mut db = selected_for(
        data.path(),
        &operation.to_string(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        true,
    );
    bound_observation(&db, operation);
    db.execute("INSERT INTO remote_ref_observations(batch_id,ordinal,remote_ref,tracking_ref,classification,advertised_oid) VALUES(1,0,'refs/heads/main','refs/remotes/origin/main','primary',?1)", ["1".repeat(40)]).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let tx = db.transaction().unwrap();
    let fenced = crate::repository::remote::state::configure(&tx, 1, Some(&plan), true);
    assert!(
        fenced.is_ok(),
        "configuration must retain live retirement evidence, not fail deleting its pinned parent: {fenced:?}"
    );
    let committed = tx.commit();
    assert!(
        committed.is_ok(),
        "configuration must commit with pinned batch and children intact: {committed:?}"
    );
    assert_eq!(
        db.query_row(
            "SELECT is_current FROM remote_observation_batches WHERE id=1",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_ref_observations WHERE batch_id=1",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert!(
        RepositoryService::open_at(data.path()).is_ok(),
        "historical audit must accept exact live pin, not require a current-generation guess"
    );
}

#[test]
fn retirement_release_history_cannot_excuse_live_or_unsettled_debt() {
    let mut accepted = Vec::new();
    for case in [
        "pin_and_release",
        "pending_consumer",
        "wrong_batch",
        "wrong_repository",
        "source_reason",
    ] {
        let data = tempfile::tempdir().unwrap();
        let operation = OperationId::new();
        let db = selected_for(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            true,
        );
        bound_observation(&db, operation);
        db.execute("INSERT INTO retirement_observation_pin_releases SELECT provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id,binding_id,'consumer_terminal' FROM retirement_observation_pins", []).unwrap();
        if case != "pin_and_release" {
            db.execute("DELETE FROM retirement_observation_pins", [])
                .unwrap();
        }
        if case != "pending_consumer" && case != "pin_and_release" {
            db.execute("UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=?1", [REFUSAL]).unwrap();
            db.execute_batch("UPDATE context_retirement_attempts SET phase='ended'; UPDATE operation_records SET state='completed';").unwrap();
            let sql: String = db
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE name='retirement_release_immutable'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            db.execute_batch("DROP TRIGGER retirement_release_immutable")
                .unwrap();
            let assignment = match case {
                "wrong_batch" => "observation_batch_id=2",
                "wrong_repository" => "repository_id=2",
                "source_reason" => "reason='source_terminal'",
                _ => unreachable!(),
            };
            db.execute(
                &format!("UPDATE retirement_observation_pin_releases SET {assignment}"),
                [],
            )
            .unwrap();
            db.execute_batch(&sql).unwrap();
        }
        if RepositoryService::open_at(data.path()).is_ok() {
            accepted.push(case);
        }
    }
    assert!(
        accepted.is_empty(),
        "contradictory release authority accepted: {accepted:?}"
    );
}

#[test]
fn retirement_terminal_result_is_closed_and_correlated_not_merely_json() {
    let mut accepted = Vec::new();
    for result in [
        "{}",
        r#"{"version":1,"code":"invented","outcome":"refused","worktree_removal":"none","branch_removal":"none","materialization_settled":false}"#,
        r#"{"version":1,"code":"external_change","outcome":"refused","worktree_removal":"observed","branch_removal":"none","materialization_settled":false}"#,
    ] {
        let data = tempfile::tempdir().unwrap();
        let operation = OperationId::new();
        let db = selected(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        db.execute("UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=?1", [result]).unwrap();
        db.execute_batch("UPDATE context_retirement_attempts SET phase='ended'; UPDATE operation_records SET state='completed';").unwrap();
        if RepositoryService::open_at(data.path()).is_ok() {
            accepted.push(result);
        }
    }
    assert!(
        accepted.is_empty(),
        "untyped/inconsistent final results accepted: {accepted:?}"
    );
}

#[test]
fn retirement_audit_checks_reverse_handoff_and_domain_request_correlations() {
    let mut accepted = Vec::new();
    for case in [
        "missing_step",
        "unmapped_request",
        "domain_borrowed_by_request",
        "observer_local_collision",
        "source_fence_without_attempt_end",
    ] {
        let data = tempfile::tempdir().unwrap();
        let operation = OperationId::new();
        let db = selected_for(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            case.starts_with("observer") || case.starts_with("source"),
        );
        match case {
            "missing_step" => {
                db.execute_batch("UPDATE context_retirement_records SET phase='index_pending',absence_basis='initial',materialization_settled=1; UPDATE operation_records SET state='authoritative_observed';").unwrap();
            }
            "unmapped_request" | "domain_borrowed_by_request" => {
                let request = OperationId::new().to_string();
                db.execute("INSERT INTO request_records(request_ulid,attempt,scope_key,command,target,intent_digest,state,accepted_at) VALUES(?1,1,?2,'context retire',?3,?4,'accepted',1)", params![request,data.path().to_str(),"01ARZ3NDEKTSV4RRFFQ69G5FAV","1".repeat(64)]).unwrap();
                if case == "unmapped_request" {
                    let trigger: String = db
                        .query_row(
                            "SELECT sql FROM sqlite_master WHERE name='retirement_record_identity'",
                            [],
                            |r| r.get(0),
                        )
                        .unwrap();
                    db.execute_batch("DROP TRIGGER retirement_record_identity")
                        .unwrap();
                    db.execute(
                        "UPDATE context_retirement_records SET request_ulid=?1",
                        [&request],
                    )
                    .unwrap();
                    db.execute_batch(&trigger).unwrap();
                } else {
                    db.execute(
                        "INSERT INTO request_operations VALUES(?1,0,'local',?2)",
                        params![request, operation.to_string()],
                    )
                    .unwrap();
                }
            }
            "observer_local_collision" | "source_fence_without_attempt_end" => {
                bound_observation(&db, operation);
                if case == "observer_local_collision" {
                    db.execute("INSERT INTO operation_records(root_path,operation_ulid,action,state,observed_at) SELECT r.root_path,a.observer_operation_ulid,'refresh','completed',1 FROM context_retirement_attempts a JOIN repositories r ON r.id=a.repository_id", []).unwrap();
                } else {
                    db.execute(
                        "UPDATE remote_operation_records SET owner_epoch=owner_epoch+1",
                        [],
                    )
                    .unwrap();
                }
            }
            _ => unreachable!(),
        }
        if RepositoryService::open_at(data.path()).is_ok() {
            accepted.push(case);
        }
    }
    assert!(
        accepted.is_empty(),
        "inconsistent handoff/owner/identity accepted: {accepted:?}"
    );
}

#[test]
fn retirement_absence_cannot_invent_effect_or_materialization_authority() {
    let mut accepted = Vec::new();
    for case in [
        "owned_absence_without_intent",
        "effect_on_branch_only_worktree",
        "settled_unverified_capture",
    ] {
        let data = tempfile::tempdir().unwrap();
        let operation = OperationId::new();
        let db = selected_for(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            true,
        );
        if case == "owned_absence_without_intent" {
            db.execute_batch("UPDATE context_retirement_records SET phase='index_pending',absence_basis='owned_intent',materialization_settled=1,branch_removal='possible'; UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released';").unwrap();
        } else if case == "effect_on_branch_only_worktree" {
            bound_observation(&db, operation);
            db.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) VALUES(?1,1,'worktree',1,?2,'ancestor')", params![operation.to_string(),"1".repeat(40)]).unwrap();
        } else {
            // This CHECK-valid combination is semantically impossible: a
            // before-effect refusal cannot also claim a settled old intent.
            db.execute_batch("UPDATE context_retirement_records SET phase='refused',absence_basis='owned_intent',materialization_settled=1,final_code='external_change',final_result=json_object('version',1,'code','external_change','outcome','refused','worktree_removal','none','branch_removal','none','materialization_settled',json('true')); UPDATE context_retirement_attempts SET phase='ended'; UPDATE operation_records SET state='completed';").unwrap();
        }
        if RepositoryService::open_at(data.path()).is_ok() {
            accepted.push(case);
        }
    }
    assert!(
        accepted.is_empty(),
        "fabricated effect/absence authority accepted: {accepted:?}"
    );
}

#[test]
fn retirement_pending_absence_receipt_cannot_be_unsettled() {
    let data = tempfile::tempdir().unwrap();
    let operation = OperationId::new();
    let db = selected(
        data.path(),
        &operation.to_string(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
    );
    db.execute_batch("UPDATE context_retirement_records SET phase='index_pending',absence_basis='initial',materialization_settled=1; UPDATE operation_records SET state='authoritative_observed',completed_step='context_retirement_absent';").unwrap();
    assert!(
        db.execute(
            "UPDATE context_retirement_records SET materialization_settled=0",
            []
        )
        .is_err(),
        "a settled D14 receipt must be forward-only even while discovery is still owed"
    );
}

#[test]
fn retirement_z_keeps_a_valid_integration_window_and_its_batch() {
    // Inert ownership/evidence setup; actual standalone metadata refusal and Z.
    let data = tempfile::tempdir().unwrap();
    git2::Repository::init(data.path()).unwrap();
    let operation = OperationId::new();
    let mut db = selected_for(
        data.path(),
        &operation.to_string(),
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        true,
    );
    bound_observation(&db, operation);
    db.execute("INSERT INTO remote_ref_observations(batch_id,ordinal,remote_ref,tracking_ref,classification,advertised_oid,tracking_oid) VALUES(1,0,'refs/heads/main','refs/remotes/origin/main','primary',?1,?1)", ["1".repeat(40)]).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let sync = OperationId::new();
    let tx = db.transaction().unwrap();
    crate::repository::remote::state::insert_operation(
        &tx,
        1,
        sync,
        &RemoteOperationTarget::for_primary_synchronization(&plan),
        crate::repository::RemoteOperationPriority::Manual,
        1,
    )
    .unwrap();
    tx.execute("UPDATE remote_operation_records SET phase='completed',outcome='completed' WHERE operation_ulid=?1", [sync.to_string()]).unwrap();
    tx.execute("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind,observation_batch_id,local_oid,primary_oid) SELECT id,1,0,0,'pinned',1,?2,?3 FROM remote_operation_records WHERE operation_ulid=?1", params![sync.to_string(),"2".repeat(40),"1".repeat(40)]).unwrap();
    crate::repository::remote::state::configure(&tx, 1, Some(&plan), true).unwrap();
    tx.commit().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let result = service
        .retire_context(crate::repository::RetireContextRequest {
            root: data.path().into(),
            kind: crate::repository::AuthoringKind::Ticket,
            item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
            operation_id: operation,
        })
        .unwrap();
    assert!(matches!(
        result,
        crate::repository::ContextRetirementOutcome::Final(_)
    ));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_ref_observations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_integration_windows WHERE number=1",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert!(
        RepositoryService::open_at(data.path()).is_ok(),
        "the independent valid window remains historical retention authority after final retirement unpin"
    );
}

#[test]
fn retirement_last_pin_release_is_atomic_with_refusal_collection_and_reopen() {
    // Inert preselected consumer; exercises real domain metadata refusal/Z, not
    // a live advertisement, selection, marker or destructive producer journey.
    for failure in [
        None,
        Some("result"),
        Some("receipt"),
        Some("pin"),
        Some("batch"),
        Some("journal"),
    ] {
        let data = tempfile::tempdir().unwrap();
        git2::Repository::init(data.path()).unwrap();
        let operation = OperationId::new();
        let mut db = selected_for(
            data.path(),
            &operation.to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            true,
        );
        bound_observation(&db, operation);
        db.execute("INSERT INTO remote_ref_observations(batch_id,ordinal,remote_ref,tracking_ref,classification,advertised_oid) VALUES(1,0,'refs/heads/main','refs/remotes/origin/main','primary',?1)", ["1".repeat(40)]).unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let tx = db.transaction().unwrap();
        crate::repository::remote::state::configure(&tx, 1, Some(&plan), true).unwrap();
        tx.commit().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        if let Some(failure) = failure {
            let event = match failure {
                "result" => "UPDATE OF final_result ON context_retirement_records",
                "receipt" => "INSERT ON retirement_observation_pin_releases",
                "pin" => "DELETE ON retirement_observation_pins",
                "batch" => "DELETE ON remote_observation_batches",
                "journal" => "UPDATE OF state ON operation_records",
                _ => unreachable!(),
            };
            db.execute_batch(&format!("CREATE TRIGGER fail_z BEFORE {event} BEGIN SELECT RAISE(ABORT,'injected Z failure'); END;")).unwrap();
        }
        let request = crate::repository::RetireContextRequest {
            root: data.path().into(),
            kind: crate::repository::AuthoringKind::Ticket,
            item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
            operation_id: operation,
        };
        if failure.is_some() {
            assert!(service.retire_context(request.clone()).is_err());
            assert_eq!(
                db.query_row("SELECT phase FROM context_retirement_records", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
                "selected"
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM retirement_observation_pin_releases",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM retirement_observation_pins",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
            assert_eq!(
                db.query_row("SELECT count(*) FROM remote_ref_observations", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert!(RepositoryService::open_at(data.path()).is_ok());
            db.execute_batch("DROP TRIGGER fail_z").unwrap();
        }
        let result = service.retire_context(request.clone());
        assert!(
            matches!(result, Ok(crate::repository::ContextRetirementOutcome::Final(ref report)) if report.code == crate::results::ResultCode::ExternalChange),
            "old generation before-effect refusal must atomically settle its hold: {result:?}"
        );
        for table in [
            "retirement_observation_pins",
            "remote_observation_batches",
            "remote_ref_observations",
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0,
                "{table}"
            );
        }
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM retirement_observation_pin_releases",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert!(
            RepositoryService::open_at(data.path()).is_ok(),
            "reopen AFTER final unpin/collection must succeed"
        );
        // SQLite may reuse this scalar ID. Replay must not look it up or GC it.
        db.execute_batch("INSERT INTO remote_observation_batches(id,repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current) VALUES(1,1,'origin','main',1,2,1)").unwrap();
        let replay = service.retire_context(request).unwrap();
        assert_eq!(replay, result.unwrap());
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(RepositoryService::open_at(data.path()).is_ok());
    }
}
