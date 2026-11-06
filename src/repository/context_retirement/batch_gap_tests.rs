//! Inert Task4 selection/cursor projections; real accepted-request reentry/original
//! handoff, gap claims, common owned writes, advertisement/receipt/pin and fences.
//! Synthetic advertisements, local Git only; no transport or resumed consumption.
use super::*;
use crate::repository::{context_retirement::handoff::gap, remote::reservation};

struct FixedClock;
impl crate::repository::mutation::Clock for FixedClock {
    fn now(&self) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(44).unwrap()
    }
}

fn accepted(s: &SourceFixture, attempt: u64) -> crate::repository::mutation::Accepted {
    while s
        .f
        .service
        .request_record(s.request_id)
        .unwrap()
        .unwrap()
        .attempt
        < attempt
    {
        s.f.service.enter_request(s.request_id).unwrap().unwrap();
    }
    let row = s.f.service.request_record(s.request_id).unwrap().unwrap();
    assert_eq!(row.attempt, attempt);
    RetirementAccepted::of_record(&row, row.attempt).unwrap()
}

fn gap_fixture() -> SourceFixture {
    let mut s = prepared_batch();
    let live = run_original(&s);
    change_first(&s);
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .refuse_if_changed(&s.f.service)
        .unwrap();
    drop(live); // No original branch lifetime permit crosses this new attempt.
    s.f.service.clock = std::sync::Arc::new(FixedClock);
    s
}

#[test]
fn retirement_batch_gap_history_audit_rejects_unproduced_or_substituted_authority() {
    for kind in [
        "fenced_without_end",
        "sync_point",
        "remote_oid",
        "wrong_outcome",
        "request",
        "digest",
        "cursor",
        "missing_pin",
    ] {
        let s = gap_fixture();
        let current = accepted(&s, 4);
        let owner = gap::claim(&s.f.service, s.f.root.path(), &current, 1).unwrap();
        before_batch(&s, &owner);
        let db = s.f.db();
        if !matches!(kind, "sync_point" | "remote_oid") {
            capture(&s, &owner).unwrap();
            s.f.service
                .finish_remote_operation(s.f.root.path(), &owner, RemoteOutcomeCategory::Completed)
                .unwrap();
        }
        match kind {
            "fenced_without_end" => {
                let next = accepted(&s, 7);
                gap::claim(&s.f.service, s.f.root.path(), &next, 2).unwrap();
                corrupt(
                    &db,
                    "retirement_gap_end",
                    "UPDATE retirement_batch_gap_attempts SET ended_kind=NULL WHERE owner_epoch=2",
                );
            }
            "sync_point" => {
                db.execute("UPDATE remote_operation_records SET completed_step='before_fetch' WHERE operation_ulid=?1",[owner.operation_id().to_string()]).unwrap();
            }
            "remote_oid" => {
                let oid = git2::Repository::open(s.f.root.path())
                    .unwrap()
                    .head()
                    .unwrap()
                    .target()
                    .unwrap();
                db.execute(
                    "UPDATE remote_operation_records SET tracking_oid=?2 WHERE operation_ulid=?1",
                    params![owner.operation_id().to_string(), oid.to_string()],
                )
                .unwrap();
            }
            "wrong_outcome" => {
                db.execute("UPDATE remote_operation_records SET outcome='transport_unavailable' WHERE operation_ulid=?1",[owner.operation_id().to_string()]).unwrap();
            }
            "request" => corrupt(
                &db,
                "retirement_gap_identity",
                "UPDATE retirement_batch_gap_attempts SET request_attempt=3",
            ),
            "digest" => corrupt(
                &db,
                "retirement_gap_identity",
                "UPDATE retirement_batch_gap_attempts SET frozen_worklist_digest=zeroblob(32)",
            ),
            "cursor" => corrupt(
                &db,
                "retirement_gap_identity",
                "UPDATE retirement_batch_gap_attempts SET next_ordinal=0",
            ),
            "missing_pin" => corrupt(
                &db,
                "retirement_pin_release_required",
                "DELETE FROM retirement_observation_pins",
            ),
            _ => unreachable!(),
        }
        let before = registry_image(&db);
        assert!(
            record::audit_connection(&db).is_err(),
            "{kind}: a supported enum/foreign-key shape is not enough to prove produced history"
        );
        assert!(
            RepositoryService::open_at(s.f.data.path()).is_err(),
            "{kind}"
        );
        assert_eq!(
            before,
            registry_image(&db),
            "{kind}: never repair a missing manifest/binding/history into authority"
        );
    }
}

fn before_batch(s: &SourceFixture, owner: &RemoteReservation) {
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
        RemoteOperationSafePoint::BeforeBatchCommit,
    ] {
        assert_eq!(
            s.f.service
                .remote_safe_point(s.f.root.path(), owner, point)
                .unwrap(),
            RemoteSafePointOutcome::Continue
        );
    }
}

fn capture(
    s: &SourceFixture,
    owner: &RemoteReservation,
) -> Result<RemoteSafePointOutcome, RepositoryError> {
    reservation::commit_observation_batch(&s.f.service, s.f.root.path(), owner, &s.plan, &[], 44)
}

#[test]
fn retirement_batch_gap_owned_capture_replacement_preserves_original_selection() {
    let s = gap_fixture();
    let first = accepted(&s, 4); // F2, batch epoch and observer epoch are independent.
    let db = s.f.db();
    let original = row_image(&db, "poll_retirement_freezes");
    let targets = row_image(&db, "poll_retirement_targets");
    let cursor = row_image(&db, "poll_retirement_cursors");
    let local = row_image(&db, "operation_records");
    let p = terminal(&db);
    let disk = disk_image(s.f.root.path());
    let owner = gap::claim(&s.f.service, s.f.root.path(), &first, 1)
        .expect("a later current accepted attempt can reserve O from a no-child gap");
    before_batch(&s, &owner);
    assert_eq!(
        capture(&s, &owner).unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(
        s.f.service
            .finish_remote_operation(s.f.root.path(), &owner, RemoteOutcomeCategory::Completed)
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    let scope:(String,i64,i64,Vec<u8>,String,Option<String>)=db.query_row("SELECT b.consumer_operation_ulid,b.consumer_request_attempt,b.consumer_attempt_epoch,b.frozen_worklist_digest,p.execution_kind,p.execution_parent_ulid FROM retirement_observation_receipts b JOIN retirement_advertisement_provenance p ON p.id=b.provenance_id WHERE p.source_operation_ulid=?1",[owner.operation_id().to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).unwrap();
    assert_eq!(
        (scope.0, scope.1, scope.2, scope.4, scope.5),
        (s.operation.to_string(), 4, 2, "batch_gap".into(), None)
    );
    assert_eq!(
        scope.3,
        db.query_row("SELECT digest FROM poll_retirement_freezes", [], |r| r
            .get::<_, Vec<u8>>(0))
            .unwrap()
    );
    assert_eq!(original, row_image(&db, "poll_retirement_freezes"));
    assert_eq!(targets, row_image(&db, "poll_retirement_targets"));
    assert_eq!(cursor, row_image(&db, "poll_retirement_cursors"));
    assert_eq!(local, row_image(&db, "operation_records"));
    assert_eq!(
        disk,
        disk_image(s.f.root.path()),
        "advertisement-only: no refs, objects, sources, nonce, W/B or M effects"
    );
    let before = registry_image(&db);
    assert!(
        gap::claim(&s.f.service, s.f.root.path(), &first, 2).is_err(),
        "same F2 cannot spend a second observer"
    );
    assert!(
        s.f.service
            .restart_remote_observation(
                s.f.root.path(),
                owner.operation_id(),
                &RemoteOperationTarget::for_poll(&s.plan)
            )
            .is_err()
    );
    assert!(
        capture(&s, &owner).is_err(),
        "terminal public O is not fresh capture authority"
    );
    assert!(
        matches!(replay(&s.f.service, &s), RemotePollResult::Complete(_)),
        "public original-P replay remains historical only"
    );
    assert!(
        s.f.service
            .observe_publication_remote(
                ObservePublicationRemoteRequest {
                    root: s.f.root.path().into(),
                    operation_id: owner.operation_id(),
                    invocation: RemotePollInvocation::Explicit,
                    approval: None,
                    restart: false
                },
                &mut crate::repository::keys::SessionCredentials::new(NoCredentials)
            )
            .is_err(),
        "public O replay cannot adopt the private gap scope"
    );
    assert_eq!(before, registry_image(&db));
    // New service/process authority does not require or reconstruct original Live.
    let next = accepted(&s, 7);
    let reopened = RepositoryService::open_at(s.f.data.path()).unwrap();
    let replacement = gap::claim(&reopened, s.f.root.path(), &next, 2).unwrap();
    let old:(String,String,i64)=db.query_row("SELECT phase,outcome,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",[owner.operation_id().to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(
        old,
        ("completed".into(), "completed".into(), 1),
        "terminal outcome preserved while epoch is raised"
    );
    assert!(capture(&s, &owner).is_err());
    assert_eq!(original, row_image(&db, "poll_retirement_freezes"));
    let current_p:Vec<rusqlite::types::Value>=db.query_row("SELECT phase,outcome,poll_completed_at,poll_next_eligible_at,owner_epoch,cancel_requested,yield_requested FROM remote_operation_records WHERE operation_ulid=?1",[s.operation.to_string()],|r|(0..r.as_ref().column_count()).map(|i|r.get(i)).collect()).unwrap();
    assert_eq!(
        p, current_p,
        "P is never fenced/reopened as though it were O"
    );
    assert_ne!(owner.operation_id(), replacement.operation_id());
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0,
        "both superseded consumer holds released through exact Z"
    );
    record::audit_connection(&db).unwrap();
}

#[test]
fn retirement_batch_gap_stops_release_only_the_exact_failed_attempt() {
    for kind in ["error", "cancel", "configuration", "generation"] {
        for captured in [false, true] {
            let s = gap_fixture();
            let first = accepted(&s, 4);
            let owner = gap::claim(&s.f.service, s.f.root.path(), &first, 1).unwrap();
            before_batch(&s, &owner);
            if captured {
                capture(&s, &owner).unwrap();
            }
            let db = s.f.db();
            let immutable = row_image(&db, "retirement_advertisement_provenance");
            match kind {
                "error" => {
                    s.f.service
                        .finish_remote_operation(
                            s.f.root.path(),
                            &owner,
                            RemoteOutcomeCategory::TransportUnavailable,
                        )
                        .unwrap();
                }
                "cancel" => {
                    db.execute(
                        "UPDATE request_records SET cancel_requested=1 WHERE request_ulid=?1",
                        [s.request_id.to_string()],
                    )
                    .unwrap();
                    assert_eq!(
                        s.f.service
                            .remote_safe_point(
                                s.f.root.path(),
                                &owner,
                                if captured {
                                    RemoteOperationSafePoint::AfterBatchCommit
                                } else {
                                    RemoteOperationSafePoint::BeforeBatchCommit
                                }
                            )
                            .unwrap(),
                        RemoteSafePointOutcome::Cancelled
                    );
                    assert!(
                        !db.query_row("SELECT cancel_requested FROM request_records", [], |r| r
                            .get::<_, bool>(
                            0
                        ))
                        .unwrap()
                    );
                }
                "configuration" => {
                    // An unregistered endpoint change does not rebind the old
                    // generation. Only exact owned STOP may close this attempt.
                    git2::Repository::open(s.f.root.path())
                        .unwrap()
                        .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/changed")
                        .unwrap();
                    let before = registry_image(&db);
                    assert!(capture(&s, &owner).is_err());
                    assert_eq!(before, registry_image(&db));
                    gap::stop_configuration(&s.f.service, s.f.root.path(), &owner)
                        .expect("known current config mismatch permits only exact owned stop");
                }
                "generation" => {
                    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                        state::configure(tx, id, Some(&s.plan), true)
                    })
                    .unwrap();
                    assert!(
                        s.f.service
                            .finish_remote_operation(
                                s.f.root.path(),
                                &owner,
                                RemoteOutcomeCategory::Completed
                            )
                            .is_err()
                    );
                    gap::stop_configuration(&s.f.service,s.f.root.path(),&owner).expect("exact owned STOP survives a registered configuration fence, never continuation/rebinding");
                }
                _ => unreachable!(),
            }
            assert_eq!(
                immutable,
                row_image(&db, "retirement_advertisement_provenance")
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM retirement_observation_pins",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            assert_eq!(db.query_row("SELECT count(*) FROM retirement_observation_pin_releases WHERE reason='consumer_terminal'",[],|r|r.get::<_,i64>(0)).unwrap(),i64::from(captured));
            record::audit_connection(&db).unwrap();
            if !matches!(kind, "configuration" | "generation") {
                if kind == "error" && captured {
                    let old:i64=db.query_row("SELECT observation_batch_id FROM retirement_advertisement_provenance WHERE execution_kind='batch_gap'",[],|r|r.get(0)).unwrap();
                    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                        state::persist_complete_advertisement(
                            tx,
                            id,
                            &s.plan,
                            state::generation(tx, id)?,
                            &[],
                            45,
                        )
                    })
                    .unwrap();
                    db.execute(
                        "DELETE FROM remote_observation_batches WHERE id=?1 AND is_current=0",
                        [old],
                    )
                    .unwrap();
                    // Explicit synthetic scalar-ID reuse, not a second owned ad.
                    db.execute("INSERT INTO remote_observation_batches(id,repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current) SELECT ?1,repository_id,remote_name,primary_branch,configuration_generation,99,0 FROM remote_polling_state",[old]).unwrap();
                }
                let next = accepted(&s, 7);
                let releases = row_image(&db, "retirement_observation_pin_releases");
                let batches = row_image(&db, "remote_observation_batches");
                gap::claim(&s.f.service,s.f.root.path(),&next,2).expect("an ended failed observation with released receipt may be replaced without resolving/relabeling its historical batch ID");
                assert_eq!(
                    releases,
                    row_image(&db, "retirement_observation_pin_releases")
                );
                assert_eq!(
                    batches,
                    row_image(&db, "remote_observation_batches"),
                    "already-released historical scalar IDs are not resolved or collected again"
                );
            }
            let before = registry_image(&db);
            assert!(
                s.f.service
                    .finish_remote_operation(
                        s.f.root.path(),
                        &owner,
                        RemoteOutcomeCategory::Completed
                    )
                    .is_err()
            );
            assert!(gap::stop_configuration(&s.f.service, s.f.root.path(), &owner).is_err());
            assert_eq!(before, registry_image(&db));
            assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
        }
    }
}

#[test]
#[allow(clippy::result_large_err)] // The injected seam uses the real SSH error type.
fn retirement_batch_gap_network_seam_rechecks_this_attempt_configuration_and_f2() {
    use crate::repository::keys::*;
    for kind in ["unchanged", "key_source", "endpoint", "f2"] {
        let mut s = prepared_with_setup(|f| {
            let path = f.data.path().join("gap-synthetic-selected-source");
            std::fs::write(&path, b"synthetic metadata, not private key material").unwrap();
            let RegisterSharedKeyOutcome::Registered(key) = f
                .service
                .register_shared_key(RegisterSharedKeyRequest {
                    label: "isolated gap fixture".into(),
                    ownership: SharedKeyOwnership::Imported,
                    private_key_path: path,
                    public_key_path: None,
                })
                .unwrap()
            else {
                panic!("synthetic registration")
            };
            f.service.select_shared_key(key.id).unwrap();
        });
        drop(run_original(&s));
        s.f.service.clock = std::sync::Arc::new(FixedClock);
        let key = s.f.data.path().join("gap-synthetic-selected-source");
        // Explicit reentry captures this NEW source observation, never pretends
        // it equals an opaque observation retained by the original executor.
        std::fs::write(&key, b"new explicit attempt synthetic source observation").unwrap();
        let current = accepted(&s, 4);
        let owner = gap::claim(&s.f.service, s.f.root.path(), &current, 1).unwrap();
        let db = s.f.db();
        let frozen = row_image(&db, "poll_retirement_freezes");
        let disk = disk_image(s.f.root.path());
        let mut called = false;
        let result = gap::observe_with(&s.f.service, s.f.root.path(), &owner, |_expected| {
            called = true;
            let git = git2::Repository::open(s.f.root.path()).unwrap();
            let _lease =
                repository_lease(&git, s.f.root.path(), RepositoryOperation::RetireContext)
                    .expect("no Git lease spans authentication/network");
            match kind {
                "key_source" => std::fs::write(
                    &key,
                    b"changed during network: original attempt may not adopt it",
                )
                .unwrap(),
                "endpoint" => git
                    .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/changed")
                    .unwrap(),
                "f2" => {
                    db.execute(
                        "UPDATE request_records SET attempt=attempt+1,cancel_requested=1",
                        [],
                    )
                    .unwrap();
                }
                _ => {}
            }
            Ok(vec![(
                "refs/heads/main".into(),
                git.head().unwrap().target().unwrap(),
            )])
        });
        assert!(
            called,
            "real owned path must reach the injected advertisement seam"
        );
        if kind == "unchanged" {
            assert_eq!(result.unwrap(), RemoteSafePointOutcome::Continue);
            assert_eq!(db.query_row("SELECT observed_at FROM retirement_advertisement_provenance WHERE execution_kind='batch_gap'",[],|r|r.get::<_,i64>(0)).unwrap(),44);
            assert_eq!(disk, disk_image(s.f.root.path()));
        } else {
            assert!(result.is_err(), "{kind}");
            assert_eq!(db.query_row("SELECT count(*) FROM retirement_advertisement_provenance WHERE execution_kind='batch_gap'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
            assert_eq!(
                db.query_row("SELECT phase FROM retirement_batch_gap_attempts", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                if kind == "f2" { "active" } else { "ended" }
            );
            if kind == "f2" {
                assert!(
                    db.query_row("SELECT cancel_requested FROM request_records", [], |r| r
                        .get::<_, bool>(
                        0
                    ))
                    .unwrap(),
                    "stale owner cannot acknowledge a newer request's STOP"
                );
            }
        }
        assert_eq!(frozen, row_image(&db, "poll_retirement_freezes"));
        record::audit_connection(&db).unwrap();
    }
}

fn corrupt(db: &Connection, trigger: &str, sql: &str) {
    let ddl: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name=?1",
            [trigger],
            |r| r.get(0),
        )
        .unwrap();
    db.execute_batch(&format!("DROP TRIGGER {trigger}"))
        .unwrap();
    db.execute_batch(sql).unwrap();
    db.execute_batch(&ddl).unwrap(); // Real catalog restored before audit/claim.
}

#[test]
fn retirement_batch_gap_exclusion_and_original_contract_are_required_before_claim() {
    for kind in [
        "local",
        "foreign_root",
        "comment",
        "readonly",
        "mutating",
        "integration",
        "resolution",
        "cleanup",
        "index",
        "same_f2",
        "mapping",
        "generation",
        "endpoint",
        "cursor",
        "digest",
        "manifest",
        "ineligible",
        "unconfirmed",
    ] {
        let s = if matches!(kind, "cleanup" | "index") {
            prepared_batch()
        } else {
            gap_fixture()
        };
        if matches!(kind, "cleanup" | "index") {
            let live = run_original(&s);
            if kind == "cleanup" {
                batch::observe(batch::Boundary::WorktreeIntentCommitted, |_| {
                    std::panic::resume_unwind(Box::new(()))
                });
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| live
                        .next(&s.f.service, s.f.root.path())
                        .unwrap()
                        .execute_branch_only(&s.f.service)))
                    .is_err()
                );
                assert_eq!(
                    s.f.db()
                        .query_row("SELECT phase FROM context_retirement_records", [], |r| {
                            r.get::<_, String>(0)
                        })
                        .unwrap(),
                    "worktree_removing"
                );
            } else {
                *s.f.service.failure_point.lock().unwrap() =
                    Some(FailurePoint::BeforeIndexTransactionCommit);
                assert!(matches!(
                    live.next(&s.f.service, s.f.root.path())
                        .unwrap()
                        .execute_branch_only(&s.f.service)
                        .unwrap(),
                    ContextRetirementOutcome::IndexPending
                ));
                assert_eq!(
                    s.f.db()
                        .query_row("SELECT phase FROM context_retirement_records", [], |r| {
                            r.get::<_, String>(0)
                        })
                        .unwrap(),
                    "index_pending"
                );
            }
        }
        let current = accepted(&s, if kind == "same_f2" { 1 } else { 4 });
        let db = s.f.db();
        match kind {
            "local" | "foreign_root" | "comment" => {
                db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,completed_step,observed_at) SELECT id,CASE WHEN ?2='foreign_root' THEN root_path||'/foreign' ELSE root_path END,?1,'refresh',?3,?4,1 FROM repositories",params![OperationId::new().to_string(),kind,if kind=="comment" {"completed"}else{"created"},(kind=="comment").then_some("comment_registration_pending")]).unwrap();
            }
            "readonly" | "mutating" => {
                let target = if kind == "readonly" {
                    RemoteOperationTarget::for_poll(&s.plan)
                } else {
                    RemoteOperationTarget::for_primary_synchronization(&s.plan)
                };
                assert!(matches!(
                    s.f.service
                        .reserve_remote_operation_with_priority(
                            s.f.root.path(),
                            OperationId::new(),
                            &target,
                            RemoteOperationPriority::Manual
                        )
                        .unwrap(),
                    RemoteReservationOutcome::Reserved(_)
                ));
            }
            "integration" => {
                debt::foreign_conflict(&s, 0);
            }
            "resolution" => {
                let op = debt::foreign_conflict(&s, 0);
                s.f.service.retirement_test_resolution(s.f.root.path(),op,&s.plan,0,crate::repository::remote::retirement_batch_test_support::ResolutionStage::Applied).unwrap();
                debt::project_inactive_resolution_envelope(&s, op);
            }
            "mapping" => {
                db.execute("UPDATE request_operations SET family='local'", [])
                    .unwrap();
            }
            "generation" => {
                state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                    state::configure(tx, id, Some(&s.plan), true)
                })
                .unwrap();
            }
            "endpoint" => git2::Repository::open(s.f.root.path())
                .unwrap()
                .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/changed")
                .unwrap(),
            "cursor" => {
                db.execute("UPDATE poll_retirement_cursors SET next_ordinal=0", [])
                    .unwrap();
            }
            "digest" => corrupt(
                &db,
                "inert_frozen_identity",
                "UPDATE poll_retirement_freezes SET digest=zeroblob(32)",
            ),
            "manifest" => {
                db.execute("DELETE FROM poll_retirement_targets WHERE ordinal=1", [])
                    .unwrap();
            }
            "ineligible" | "unconfirmed" => corrupt(
                &db,
                "inert_frozen_decision",
                &format!("UPDATE poll_retirement_freezes SET eligibility='{kind}'"),
            ),
            _ => {}
        }
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(
            gap::claim(&s.f.service, s.f.root.path(), &current, 1).is_err(),
            "{kind}: no invented parent exemption or current-snapshot authority"
        );
        assert_eq!(before, registry_image(&db), "{kind}");
        assert_eq!(disk, disk_image(s.f.root.path()), "{kind}");
    }
}

#[test]
fn retirement_batch_gap_each_owned_boundary_rechecks_exact_scope() {
    for kind in [
        "f2",
        "mapping",
        "batch_epoch",
        "cursor",
        "root",
        "generation",
        "digest",
        "local",
        "comment",
        "advertisement",
    ] {
        let s = gap_fixture();
        let current = accepted(&s, 4);
        let owner = gap::claim(&s.f.service, s.f.root.path(), &current, 1).unwrap();
        before_batch(&s, &owner);
        capture(&s, &owner).unwrap();
        let db = s.f.db();
        match kind {
            "f2" => {
                db.execute(
                    "UPDATE request_records SET attempt=attempt+1,cancel_requested=1",
                    [],
                )
                .unwrap();
            }
            "mapping" => {
                db.execute("UPDATE request_operations SET family='local'", [])
                    .unwrap();
            }
            "batch_epoch" => {
                db.execute(
                    "UPDATE poll_retirement_batches SET owner_epoch=owner_epoch+1",
                    [],
                )
                .unwrap();
            }
            "cursor" => {
                db.execute("UPDATE poll_retirement_cursors SET next_ordinal=0", [])
                    .unwrap();
            }
            "root" => {
                db.execute(
                    "UPDATE repositories SET root_path=root_path||'/foreign'",
                    [],
                )
                .unwrap();
            }
            "generation" => {
                state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                    state::configure(tx, id, Some(&s.plan), true)
                })
                .unwrap();
            }
            "digest" => corrupt(
                &db,
                "inert_frozen_identity",
                "UPDATE poll_retirement_freezes SET digest=zeroblob(32)",
            ),
            "local" | "comment" => {
                db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,completed_step,observed_at) SELECT id,root_path,?1,'refresh',?2,?3,1 FROM repositories",params![OperationId::new().to_string(),if kind=="comment" {"completed"}else{"created"},(kind=="comment").then_some("comment_registration_pending")]).unwrap();
            }
            "advertisement" => {
                state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                    state::persist_complete_advertisement(
                        tx,
                        id,
                        &s.plan,
                        state::generation(tx, id)?,
                        &[],
                        45,
                    )
                })
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = registry_image(&db);
        assert!(
            state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| reservation::owned(
                &s.f.service,
                tx,
                id,
                &owner
            ))
            .is_err(),
            "{kind}: common owned error/config/write gate"
        );
        assert!(
            s.f.service
                .remote_safe_point(
                    s.f.root.path(),
                    &owner,
                    RemoteOperationSafePoint::AfterBatchCommit
                )
                .is_err(),
            "{kind}: safe point"
        );
        assert!(capture(&s, &owner).is_err(), "{kind}: ad capture");
        assert!(
            s.f.service
                .finish_remote_operation(
                    s.f.root.path(),
                    &owner,
                    RemoteOutcomeCategory::TransportUnavailable
                )
                .is_err(),
            "{kind}: error write"
        );
        assert!(
            s.f.service
                .finish_remote_operation(s.f.root.path(), &owner, RemoteOutcomeCategory::Completed)
                .is_err(),
            "{kind}: completion"
        );
        assert!(
            s.f.service
                .restart_remote_observation(
                    s.f.root.path(),
                    owner.operation_id(),
                    &RemoteOperationTarget::for_poll(&s.plan)
                )
                .is_err(),
            "{kind}: public restart"
        );
        assert_eq!(
            before,
            registry_image(&db),
            "{kind}: stale owners never acknowledge current cancellation or write a projection"
        );
    }
}

#[test]
fn retirement_batch_gap_replacement_fences_only_its_exact_readonly_observer() {
    for stage in [
        "reserved",
        "before_commit",
        "after_commit",
        "foreign_observer",
        "mutating_observer",
        "reserved_child",
    ] {
        let s = gap_fixture();
        let current = accepted(&s, 4);
        let owner = gap::claim(&s.f.service, s.f.root.path(), &current, 1).unwrap();
        if stage != "reserved" {
            before_batch(&s, &owner);
        }
        if stage == "after_commit" {
            capture(&s, &owner).unwrap();
        }
        let db = s.f.db();
        if matches!(
            stage,
            "foreign_observer" | "mutating_observer" | "reserved_child"
        ) {
            s.f.service
                .finish_remote_operation(
                    s.f.root.path(),
                    &owner,
                    RemoteOutcomeCategory::TransportUnavailable,
                )
                .unwrap();
            let next = accepted(&s, 7);
            let id = if stage == "reserved_child" {
                let image: String = db
                    .query_row("SELECT image FROM poll_retirement_freezes", [], |r| {
                        r.get(0)
                    })
                    .unwrap();
                let image: serde_json::Value = serde_json::from_str(&image).unwrap();
                OperationId::parse(image[1]["child_operation_id"].as_str().unwrap()).unwrap()
            } else {
                OperationId::new()
            };
            let target = if stage == "mutating_observer" {
                RemoteOperationTarget::for_primary_synchronization(&s.plan)
            } else {
                RemoteOperationTarget::for_poll(&s.plan)
            };
            state::with_transaction(&s.f.service, s.f.root.path(), |tx, id_repo| {
                state::insert_operation(
                    tx,
                    id_repo,
                    id,
                    &target,
                    RemoteOperationPriority::Manual,
                    44,
                )
            })
            .unwrap();
            corrupt(
                &db,
                "retirement_gap_identity",
                &format!("UPDATE retirement_batch_gap_attempts SET observer_operation_ulid='{id}'"),
            );
            let before = registry_image(&db);
            assert!(
                gap::claim(&s.f.service, s.f.root.path(), &next, 2).is_err(),
                "{stage}"
            );
            assert_eq!(
                before,
                registry_image(&db),
                "{stage}: no unrelated/mutating fence or new binding"
            );
        } else {
            let next = accepted(&s, 7);
            let replacement = gap::claim(&s.f.service, s.f.root.path(), &next, 2).unwrap();
            assert_ne!(owner.operation_id(), replacement.operation_id());
            let old:(String,i64)=db.query_row("SELECT phase,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",[owner.operation_id().to_string()],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
            assert_eq!(old, ("interrupted".into(), 1));
            let before = registry_image(&db);
            assert!(capture(&s, &owner).is_err());
            assert!(
                s.f.service
                    .remote_safe_point(
                        s.f.root.path(),
                        &owner,
                        RemoteOperationSafePoint::BeforeBatchCommit
                    )
                    .is_err()
            );
            assert!(
                s.f.service
                    .finish_remote_operation(
                        s.f.root.path(),
                        &owner,
                        RemoteOutcomeCategory::TransportUnavailable
                    )
                    .is_err()
            );
            assert!(gap::stop_configuration(&s.f.service, s.f.root.path(), &owner).is_err());
            assert_eq!(before, registry_image(&db));
            record::audit_connection(&db).unwrap();
        }
    }
}

#[test]
fn retirement_batch_gap_claim_capture_and_stop_faults_roll_back_the_whole_projection() {
    use gap::Fault;
    for point in [
        Fault::Fence,
        Fault::Epoch,
        Fault::Observer,
        Fault::Bind,
        Fault::Release,
        Fault::Provenance,
        Fault::Receipt,
        Fault::Pin,
        Fault::Capture,
        Fault::End,
    ] {
        let s = gap_fixture();
        let first = accepted(&s, 4);
        let owner = gap::claim(&s.f.service, s.f.root.path(), &first, 1).unwrap();
        let db = s.f.db();
        before_batch(&s, &owner);
        let claim_fault = matches!(
            point,
            Fault::Fence | Fault::Epoch | Fault::Observer | Fault::Bind | Fault::Release
        );
        let next = if claim_fault {
            capture(&s, &owner).unwrap();
            s.f.service
                .finish_remote_operation(s.f.root.path(), &owner, RemoteOutcomeCategory::Completed)
                .unwrap();
            // Explicit synthetic second advertisement makes the old held batch
            // obsolete: Z's conditional collection must roll back as well.
            state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                state::persist_complete_advertisement(
                    tx,
                    id,
                    &s.plan,
                    state::generation(tx, id)?,
                    &[],
                    45,
                )
            })
            .unwrap();
            Some(accepted(&s, 7))
        } else {
            if point == Fault::End {
                capture(&s, &owner).unwrap();
            }
            None
        };
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert_eq!(gap::set_failure(Some(point)), None);
        let failed = if let Some(next) = next {
            gap::claim(&s.f.service, s.f.root.path(), &next, 2).is_err()
        } else if point == Fault::End {
            s.f.service
                .finish_remote_operation(
                    s.f.root.path(),
                    &owner,
                    RemoteOutcomeCategory::TransportUnavailable,
                )
                .is_err()
        } else {
            capture(&s, &owner).is_err()
        };
        let unconsumed = gap::set_failure(None);
        assert!(failed, "{point:?}: missing failure boundary");
        assert_eq!(
            unconsumed, None,
            "{point:?}: the real write path must reach the injected fault"
        );
        assert_eq!(
            before,
            registry_image(&db),
            "{point:?}: claim/fence/ad/provenance/receipt/pin/Z/collection/outcome commit together"
        );
        assert_eq!(
            disk,
            disk_image(s.f.root.path()),
            "{point:?}: no Git/resource/source effects"
        );
        record::audit_connection(&db).unwrap();
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}
