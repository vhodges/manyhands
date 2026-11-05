//! Real private initial-absence/index execution under INERT canonical F2 records.
//! Not Task 3 mutation journeys. Missing receipts are explicit corruption tests;
//! canonical delete-guard DDL is restored BEFORE every audit/recovery assertion.
use super::*;
use crate::repository::{
    context_retirement::{record, request_scope::Scope},
    keys::*,
    request_store::*,
};

struct NoCredentials;
impl SessionCredentialProvider for NoCredentials {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("local absence/index recovery must not ask for credentials")
    }
}

fn request(f: &Fixture) -> RetireContextRequest {
    RetireContextRequest {
        root: f.root.path().canonicalize().unwrap(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    }
}
fn accepted(service: &RepositoryService, r: &RetireContextRequest) -> RetirementAccepted {
    let id = RequestId::new();
    let scope = ScopeKey::for_repository(&r.root).unwrap();
    let digest = IntentDigest::builder(
        DigestSalt::Request(id),
        "context retire",
        &scope,
        &r.item_id.to_string(),
    )
    .field("kind", FieldValue::Text("ticket"))
    .finish();
    service
        .insert_request(&NewRequest {
            request_id: id,
            scope,
            command: "context retire".into(),
            target: r.item_id.to_string(),
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
    duplicate(service, id)
}
fn duplicate(service: &RepositoryService, id: RequestId) -> RetirementAccepted {
    let row = service.request_record(id).unwrap().unwrap();
    RetirementAccepted::of_record(&row, row.attempt).unwrap()
}
fn run(
    service: &RepositoryService,
    r: &RetireContextRequest,
    a: &RetirementAccepted,
) -> Result<ContextRetirementOutcome, RepositoryError> {
    service.retire_context_for_request(
        r.clone(),
        a,
        None,
        &mut SessionCredentials::new(NoCredentials),
    )
}
fn claimed(f: &Fixture, stage: &str) -> (RetireContextRequest, RetirementAccepted) {
    let r = request(f);
    let a = accepted(&f.service, &r);
    // Never reuse a service whose hook unwound while holding its hook mutex.
    // Both f.service and the separate recovery service below stay unpoisoned.
    let producer = RepositoryService::open_at(f.data.path()).unwrap();
    if matches!(stage, "complete" | "finished_complete") {
        assert!(
            matches!(run(&producer,&r,&a),Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::AlreadyAbsent)
        );
        if stage == "finished_complete" {
            assert!(
                f.service
                    .finish_request(
                        a.request_id,
                        a.attempt,
                        &RequestResult {
                            outcome: crate::results::Outcome::Noop,
                            code: crate::results::ResultCode::AlreadyApplied,
                            effects: crate::results::Effects::not_requested(),
                            data: None
                        }
                    )
                    .unwrap()
            );
        }
    } else if stage == "persisted_cache" {
        let db = f.db();
        // A finalizer SQL failure leaves REAL earlier cache persistence owed.
        db.execute_batch("CREATE TRIGGER index_claim_finalizer_fault BEFORE UPDATE OF phase ON context_retirement_records WHEN NEW.phase='complete' BEGIN SELECT RAISE(ABORT,'fixture finalizer failure'); END;").unwrap();
        assert!(matches!(
            run(&producer, &r, &a),
            Ok(ContextRetirementOutcome::IndexPending)
        ));
        db.execute_batch("DROP TRIGGER index_claim_finalizer_fault;")
            .unwrap();
        assert!(
            db.query_row(
                "SELECT persisted_context_count>0 FROM operation_records WHERE operation_ulid=?1",
                [r.operation_id.to_string()],
                |row| row.get::<_, bool>(0)
            )
            .unwrap()
        );
    } else {
        producer.set_refresh_claim_hook_for_testing(|| std::panic::resume_unwind(Box::new(())));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&producer, &r, &a)))
                .is_err()
        );
        let fence = Scope::capture(&f.service, &r, &a).unwrap().fence().unwrap();
        let db = f.db();
        let local = db
            .query_row(
                "SELECT local_record_id FROM context_retirement_records WHERE operation_ulid=?1",
                [r.operation_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        let owner = recovery::IndexOwner {
            record_id: local,
            epoch: 1,
            retirement_request: Some(fence),
        };
        if stage == "index_invalidated" {
            crate::repository::remote::state::with_transaction(&f.service, &r.root, |tx, id| {
                recovery::invalidate_index_observations(tx, id)
            })
            .unwrap();
        } else if stage != "indexing" {
            // Guarded shared-journal states explicitly accepted by the private
            // I-only recovery claim. Not a claim that fresh execution emits all.
            assert!(recovery::transition_indexing(&db, owner, stage, None).unwrap());
        }
    }
    record::audit_connection(&f.db()).unwrap();
    assert!(
        RepositoryService::open_at(f.data.path()).is_ok(),
        "intact {stage}"
    );
    (r, a)
}
fn remove_claim(f: &Fixture, epoch: Option<i64>) {
    let db = f.db();
    db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let ddl: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='retirement_index_claim_delete'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    db.execute_batch("DROP TRIGGER retirement_index_claim_delete;")
        .unwrap();
    assert!(
        db.execute(
            "DELETE FROM context_retirement_index_claims WHERE ?1 IS NULL OR index_owner_epoch=?1",
            [epoch]
        )
        .unwrap()
            > 0
    );
    db.execute_batch(&ddl).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT sql FROM sqlite_master WHERE name='retirement_index_claim_delete'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        ddl
    );
    assert_eq!(
        db.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert!(
        !db.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );
}

#[test]
fn retirement_f2_specfix_missing_spend_refuses_startup_in_every_claimed_state() {
    let mut allowed = Vec::new();
    for stage in [
        "indexing",
        "persisted_cache",
        "failed",
        "observed",
        "persisted",
        "retry",
        "index_invalidated",
        "complete",
        "finished_complete",
    ] {
        let f = Fixture::new();
        let (_r, _a) = claimed(&f, stage);
        remove_claim(&f, None);
        let before = registry_image(&f.db());
        let disk = disk_image(f.root.path());
        if !matches!(RepositoryService::open_at(f.data.path()),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        {
            allowed.push(stage);
        }
        assert_eq!(before, registry_image(&f.db()));
        assert_eq!(disk, disk_image(f.root.path()));
    }
    assert!(
        allowed.is_empty(),
        "missing spend accepted at startup: {allowed:?}"
    );
}

#[test]
fn retirement_f2_specfix_missing_spend_cannot_be_reminted_by_open_or_cross_service() {
    let mut allowed = Vec::new();
    for stage in [
        "indexing",
        "persisted_cache",
        "failed",
        "observed",
        "persisted",
        "retry",
        "index_invalidated",
        "complete",
        "finished_complete",
    ] {
        let f = Fixture::new();
        let (r, a) = claimed(&f, stage);
        let other = RepositoryService::open_at(f.data.path()).unwrap();
        let same = duplicate(&other, a.request_id);
        assert_eq!(same.attempt, a.attempt);
        remove_claim(&f, None);
        let before = registry_image(&f.db());
        let disk = disk_image(f.root.path());
        // Exercise BOTH unpoisoned already-open services even when startup
        // refuses. Do not make recovery assertions depend on successful reopen.
        for (name, service) in [("cross-service", &other), ("already-open", &f.service)] {
            if !matches!(run(service,&r,&same),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
            {
                allowed.push((stage, name));
            }
        }
        if before != registry_image(&f.db()) {
            allowed.push((stage, "registry/cache/claim replacement"));
        }
        assert_eq!(disk, disk_image(f.root.path()));
    }
    assert!(
        allowed.is_empty(),
        "missing spend was treated as never claimed: {allowed:?}"
    );
}

#[test]
fn retirement_f2_specfix_intact_spends_keep_duplicates_nonowning_and_new_f2_takeover_valid() {
    for stage in [
        "indexing",
        "persisted_cache",
        "failed",
        "observed",
        "persisted",
        "retry",
        "index_invalidated",
        "complete",
        "finished_complete",
    ] {
        let f = Fixture::new();
        let (r, a) = claimed(&f, stage);
        let other = RepositoryService::open_at(f.data.path()).unwrap();
        let same = duplicate(&other, a.request_id);
        let before = registry_image(&f.db());
        let disk = disk_image(f.root.path());
        let result = run(&other, &r, &same).unwrap();
        assert!(if matches!(stage, "complete" | "finished_complete") {
            matches!(result, ContextRetirementOutcome::Final(_))
        } else {
            result == ContextRetirementOutcome::IndexPending
        });
        assert_eq!(before, registry_image(&f.db()));
        assert_eq!(disk, disk_image(f.root.path()));
        if !matches!(stage, "complete" | "finished_complete") {
            other.enter_request(a.request_id).unwrap().unwrap();
            let new = duplicate(&other, a.request_id);
            assert!(
                matches!(run(&other,&r,&new),Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::AlreadyAbsent)
            );
            let claims:Vec<(i64,i64)>=f.db().prepare("SELECT request_attempt,index_owner_epoch FROM context_retirement_index_claims ORDER BY request_attempt").unwrap().query_map([],|row|Ok((row.get(0)?,row.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
            assert_eq!(
                claims,
                vec![
                    (1, 1),
                    (2, if stage == "index_invalidated" { 3 } else { 2 })
                ]
            );
            assert!(RepositoryService::open_at(f.data.path()).is_ok());
            assert_eq!(disk, disk_image(f.root.path()));
        }
    }
}

fn take_over(
    f: &Fixture,
    r: &RetireContextRequest,
    id: RequestId,
    invalidate: bool,
) -> RetirementAccepted {
    if invalidate {
        crate::repository::remote::state::with_transaction(&f.service, &r.root, |tx, repo| {
            recovery::invalidate_index_observations(tx, repo)
        })
        .unwrap();
    }
    f.service.enter_request(id).unwrap().unwrap();
    let next = duplicate(&f.service, id);
    let producer = RepositoryService::open_at(f.data.path()).unwrap();
    producer.set_refresh_claim_hook_for_testing(|| std::panic::resume_unwind(Box::new(())));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&producer, r, &next)))
            .is_err()
    );
    next
}

#[test]
fn retirement_f2_specfix_missing_provable_older_spends_cannot_authorize_new_takeover() {
    let mut allowed = Vec::new();
    for case in [
        "first_live",
        "first_after_tombstone",
        "middle_after_tombstones",
        "last_live",
        "last_tombstone",
    ] {
        let f = Fixture::new();
        let (r, first) = claimed(&f, "indexing");
        take_over(&f, &r, first.request_id, case != "first_live");
        if case == "middle_after_tombstones" {
            take_over(&f, &r, first.request_id, true);
        }
        if case == "last_tombstone" {
            crate::repository::remote::state::with_transaction(&f.service, &r.root, |tx, repo| {
                recovery::invalidate_index_observations(tx, repo)
            })
            .unwrap();
        }
        let epochs:Vec<i64>=f.db().prepare("SELECT index_owner_epoch FROM context_retirement_index_claims ORDER BY index_owner_epoch").unwrap().query_map([],|row|row.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(
            epochs,
            match case {
                "first_live" => vec![1, 2],
                "middle_after_tombstones" => vec![1, 3, 5],
                _ => vec![1, 3],
            }
        );
        let other = RepositoryService::open_at(f.data.path()).unwrap();
        // Raise F2 BEFORE corruption, so refusal cannot be attributed to a stale
        // caller. The intact control for these same chains is above/below.
        other.enter_request(first.request_id).unwrap().unwrap();
        let current = duplicate(&other, first.request_id);
        remove_claim(
            &f,
            Some(match case {
                "first_live" | "first_after_tombstone" => 1,
                _ => 3,
            }),
        );
        let before = registry_image(&f.db());
        let disk = disk_image(f.root.path());
        if RepositoryService::open_at(f.data.path()).is_ok() {
            allowed.push((case, "startup"));
        }
        if !matches!(run(&other,&r,&current),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        {
            allowed.push((case, "new-current takeover"));
        }
        if before != registry_image(&f.db()) {
            allowed.push((case, "rewrote evidence"));
        }
        assert_eq!(disk, disk_image(f.root.path()));
    }
    assert!(
        allowed.is_empty(),
        "provable spend history loss was accepted: {allowed:?}"
    );
}

#[test]
fn retirement_f2_specfix_never_claimed_handoff_and_domain_history_need_no_spend() {
    let f = Fixture::new();
    let r = request(&f);
    let a = accepted(&f.service, &r);
    let db = f.db();
    // INERT never-claimed authoritative handoff: the real I-only protocol below
    // must claim it once. This projection is not an initial-absence producer.
    db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at) SELECT id,root_path,?1,'retire_context',?2,'authoritative_observed','context_retirement_absent',1 FROM repositories",params![r.operation_id.to_string(),format!("authoring-context-v1/ticket/{ITEM}/Retire")]).unwrap();
    db.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis,request_ulid) SELECT operation_ulid,id,repository_id,1,'ticket',?2,'absent',1,0,'absent',1,1,'index_pending','none','none','initial',?3 FROM operation_records WHERE operation_ulid=?1",params![r.operation_id.to_string(),ITEM,a.request_id.to_string()]).unwrap();
    db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,request_attempt,phase) VALUES(?1,1,1,1,'active')",[r.operation_id.to_string()]).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT index_owner_epoch FROM operation_records WHERE operation_ulid=?1",
            [r.operation_id.to_string()],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    record::audit_connection(&db).unwrap();
    let service = RepositoryService::open_at(f.data.path()).unwrap();
    assert!(
        matches!(run(&service,&r,&a),Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::AlreadyAbsent)
    );

    // Published DOMAIN history has no request spend ledger and independent old
    // index counters. Existing v1/v2 catalog/history migration tests also remain.
    let f = Fixture::new();
    let r = request(&f);
    f.inert_absent_handoff(r.operation_id);
    f.db().execute("UPDATE operation_records SET state='failed',index_owner_epoch=41 WHERE operation_ulid=?1",[r.operation_id.to_string()]).unwrap();
    let service = RepositoryService::open_at(f.data.path()).unwrap();
    assert!(matches!(
        service.retire_context(r),
        Ok(ContextRetirementOutcome::Final(_))
    ));
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM context_retirement_index_claims",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
