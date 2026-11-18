//! Q1: a request-map reservation precedes the retirement envelope. These inert
//! canonical F2 fixtures exercise public refresh and the real private protocol;
//! they do not introduce Task 3 bindings or bypass CHECK/foreign-key constraints.
use super::*;

fn refresh(
    service: &RepositoryService,
    root: &Path,
    operation_id: OperationId,
) -> Result<RefreshOutcome, RepositoryError> {
    service.refresh_repository(RefreshRepositoryRequest {
        root: root.into(),
        operation_id,
    })
}

#[test]
fn retirement_f2_qualityfix_preclaim_refresh_refusal_preserves_same_current_execution() {
    let mut failures = Vec::new();
    for case in [
        "no_generic_control",
        "same_service",
        "cross_service",
        "after_reopen",
        "equivalent_root",
        "new_current",
    ] {
        let f = Fixture::new();
        let r = request(&f);
        let a = accepted(&f.service, &r);
        let other = RepositoryService::open_at(f.data.path()).unwrap();
        let service = if matches!(case, "cross_service" | "after_reopen") {
            &other
        } else {
            &f.service
        };
        let before = registry_image(&f.db());
        let disk = disk_image(f.root.path());
        assert!(
            !f.db()
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM operation_records WHERE operation_ulid=?1)",
                    [r.operation_id.to_string()],
                    |row| row.get::<_, bool>(0)
                )
                .unwrap()
        );
        if case != "no_generic_control" {
            let root = if case == "equivalent_root" {
                r.root.join(".")
            } else {
                r.root.clone()
            };
            assert!(
                matches!(refresh(service,&root,r.operation_id),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
            );
            if before != registry_image(&f.db()) {
                failures.push((case, "refusal changed full registry/cache/schema image"));
            }
            assert_eq!(disk, disk_image(f.root.path()));
        }
        let current = if case == "new_current" {
            f.service.enter_request(a.request_id).unwrap().unwrap();
            duplicate(&f.service, a.request_id)
        } else {
            a
        };
        // Always run this, including on RED after detecting the durable poison.
        // Advancing F2 is a separate control, not a prerequisite for unblocking.
        let reopened = if case == "after_reopen" {
            Some(RepositoryService::open_at(f.data.path()).unwrap())
        } else {
            None
        };
        let result = run(reopened.as_ref().unwrap_or(&f.service), &r, &current);
        if !matches!(result,Ok(ContextRetirementOutcome::Final(ref report)) if report.outcome==RetirementDisposition::AlreadyAbsent)
        {
            failures.push((case, "valid private retirement did not complete"));
        }
        assert_eq!(disk, disk_image(f.root.path()));
    }
    assert!(
        failures.is_empty(),
        "preclaim reservation failures: {failures:?}"
    );
}

#[test]
fn retirement_f2_qualityfix_malformed_retirement_mappings_still_reserve_the_operation() {
    let mut changed = Vec::new();
    for case in [
        "remote_family",
        "key_family",
        "ordinal",
        "duplicate",
        "extra_operation",
        "scope",
        "target",
        "digest",
        "cancelled",
    ] {
        let f = Fixture::new();
        let r = request(&f);
        accepted(&f.service, &r);
        let db = f.db();
        // Explicit corrupt/inert mapping projections, all within canonical SQL
        // constraints. Reservation is not permission to adopt or repair them.
        db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        match case {
            "remote_family" => {
                db.execute("UPDATE request_operations SET family='remote'", [])
                    .unwrap();
            }
            "key_family" => {
                db.execute("UPDATE request_operations SET family='key_material'", [])
                    .unwrap();
            }
            "ordinal" => {
                db.execute("UPDATE request_operations SET ordinal=1", [])
                    .unwrap();
            }
            "duplicate" => {
                db.execute("INSERT INTO request_operations SELECT request_ulid,1,family,operation_ulid FROM request_operations",[]).unwrap();
            }
            "extra_operation" => {
                db.execute("INSERT INTO request_operations SELECT request_ulid,1,family,?1 FROM request_operations",[OperationId::new().to_string()]).unwrap();
            }
            "scope" => {
                db.execute("UPDATE request_records SET scope_key='/foreign-root'", [])
                    .unwrap();
            }
            "target" => {
                db.execute("UPDATE request_records SET target='foreign-target'", [])
                    .unwrap();
            }
            "digest" => {
                db.execute(
                    "UPDATE request_records SET intent_digest=?1",
                    ["0".repeat(64)],
                )
                .unwrap();
            }
            "cancelled" => {
                db.execute("UPDATE request_records SET cancel_requested=1", [])
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            !db.prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap()
        );
        let before = registry_image(&db);
        let disk = disk_image(f.root.path());
        let preflight = recovery::require_refresh_can_begin(
            &db,
            &r.root,
            RepositoryOperation::RefreshRepository,
            r.operation_id,
        );
        if !matches!(preflight,Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired) {
            changed.push((case, "preflight accepted"));
        }
        assert!(
            matches!(refresh(&f.service,&r.root,r.operation_id),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        );
        if before != registry_image(&db) {
            changed.push((case, "public refusal changed full image"));
        }
        assert_eq!(disk, disk_image(f.root.path()));
    }
    assert!(
        changed.is_empty(),
        "malformed reservation failures: {changed:?}"
    );
}

#[test]
fn retirement_f2_qualityfix_ordinary_and_other_request_refreshes_remain_supported() {
    for case in ["ordinary", "index refresh", "ticket save"] {
        let f = Fixture::new();
        let r = request(&f);
        if case != "ordinary" {
            let id = RequestId::new();
            let scope = ScopeKey::for_repository(&r.root).unwrap();
            let target = if case == "ticket save" { ITEM } else { "" };
            let digest =
                IntentDigest::builder(DigestSalt::Request(id), case, &scope, target).finish();
            f.service
                .insert_request(&NewRequest {
                    request_id: id,
                    scope,
                    command: case.into(),
                    target: target.into(),
                    intent_digest: digest,
                    confirmation: None,
                    base_ref: None,
                    base_oid: None,
                    expected_digest: None,
                    operations: vec![RequestOperation {
                        family: OperationFamily::Local,
                        operation_id: r.operation_id,
                    }],
                })
                .unwrap();
            if case == "ticket save" {
                // Inert ordinary authoritative lifecycle handoff, NOT a claim
                // to have performed authoring. Refresh must keep its identity.
                f.db().execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at) SELECT id,root_path,?1,'save_ticket',?2,'authoritative_observed','authoritative_observed',1 FROM repositories",params![r.operation_id.to_string(),ITEM]).unwrap();
            }
        }
        let db = f.db();
        let requests = row_image(&db, "request_records");
        let mappings = row_image(&db, "request_operations");
        recovery::require_refresh_can_begin(
            &db,
            &r.root,
            RepositoryOperation::RefreshRepository,
            r.operation_id,
        )
        .unwrap();
        assert!(
            matches!(
                refresh(&f.service, &r.root, r.operation_id),
                Ok(RefreshOutcome::Refreshed { .. })
            ),
            "{case}"
        );
        assert_eq!(requests, row_image(&db, "request_records"));
        assert_eq!(mappings, row_image(&db, "request_operations"));
        assert_eq!(
            db.query_row(
                "SELECT action,state FROM operation_records WHERE operation_ulid=?1",
                [r.operation_id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            )
            .unwrap(),
            (
                if case == "ticket save" {
                    "save_ticket"
                } else {
                    "refresh"
                }
                .into(),
                "completed".into()
            )
        );
    }
}

#[test]
fn retirement_f2_qualityfix_reservation_is_operation_not_item_identity() {
    let f = Fixture::new();
    let r = request(&f);
    let a = accepted(&f.service, &r);
    assert_ne!(r.operation_id.to_string(), ITEM);
    assert!(matches!(
        refresh(&f.service, &r.root, OperationId::parse(ITEM).unwrap()),
        Ok(RefreshOutcome::Refreshed { .. })
    ));
    assert!(
        matches!(run(&f.service,&r,&a),Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::AlreadyAbsent)
    );
}

#[test]
fn retirement_f2_qualityfix_pre_request_schema_stays_valid_but_partial_pair_refuses() {
    let f = Fixture::new();
    let source = f.db();
    for installed in ["neither", "records", "mappings", "both"] {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE repositories(id INTEGER PRIMARY KEY,root_path TEXT);")
            .unwrap();
        recovery::migrate_operation_records(&mut db).unwrap();
        for (table, include) in [
            ("request_records", matches!(installed, "records" | "both")),
            (
                "request_operations",
                matches!(installed, "mappings" | "both"),
            ),
        ] {
            if include {
                let ddl: String = source
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name=?1",
                        [table],
                        |row| row.get(0),
                    )
                    .unwrap();
                db.execute_batch(&ddl).unwrap();
            }
        }
        let before = registry_image(&db);
        let result = record::refresh_allowed(
            &db,
            &OperationId::new().to_string(),
            Path::new("/repository"),
        );
        if matches!(installed, "neither" | "both") {
            assert!(result.unwrap(), "{installed}");
        } else {
            assert!(
                matches!(result,Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired),
                "partial {installed}"
            );
        }
        assert_eq!(before, registry_image(&db));
    }
}
