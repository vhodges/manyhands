//! Inert producer/target/cursor projections, synthetic ads and pre-enrolled
//! identities. Actual owned poll, A3, C decisions/results/Z; positive submodule
//! additionally runs branch cleanup/index. NOT source SSH, Task 4 selection,
//! enrollment or coordinator evidence.
use super::*;
use crate::repository::context_retirement::handoff::batch;

#[path = "batch_debt_tests.rs"]
mod debt;
#[path = "batch_inspection_tests.rs"]
mod inspection;
#[path = "batch_positive_tests.rs"]
mod positive;

fn projections(s: &SourceFixture, entries: &[serde_json::Value]) {
    let db = s.f.db();
    db.execute_batch("DROP TRIGGER inert_frozen_decision;
      CREATE TRIGGER inert_frozen_decision BEFORE UPDATE OF eligibility ON poll_retirement_freezes WHEN OLD.eligibility!='unconfirmed' BEGIN SELECT RAISE(ABORT,'immutable handoff'); END;
      CREATE TRIGGER inert_batch_final BEFORE UPDATE OF final_code ON poll_retirement_freezes WHEN OLD.final_code IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable batch final'); END;
      CREATE TABLE poll_retirement_cursors(operation_ulid TEXT NOT NULL,owner_epoch INTEGER NOT NULL,next_ordinal INTEGER NOT NULL CHECK(next_ordinal>=0),stage TEXT NOT NULL CHECK(stage IN ('active','complete')),PRIMARY KEY(operation_ulid,owner_epoch));
      CREATE TABLE poll_retirement_targets(operation_ulid TEXT NOT NULL,owner_epoch INTEGER NOT NULL,ordinal INTEGER NOT NULL CHECK(ordinal>=0),candidate TEXT NOT NULL CHECK(json_valid(candidate)),state TEXT NOT NULL CHECK(state IN ('waiting','running','complete','refused')),final_result TEXT CHECK(final_result IS NULL OR json_valid(final_result)),handoff TEXT NOT NULL CHECK(handoff IN ('not_started','cleanup','index_pending','final')),PRIMARY KEY(operation_ulid,owner_epoch,ordinal),CHECK((state='waiting' AND final_result IS NULL AND handoff='not_started') OR (state='running' AND final_result IS NULL AND handoff IN ('cleanup','index_pending')) OR (state IN ('refused','complete') AND final_result IS NOT NULL AND handoff='final')));
      CREATE TRIGGER inert_target_identity BEFORE UPDATE OF operation_ulid,owner_epoch,ordinal,candidate ON poll_retirement_targets BEGIN SELECT RAISE(ABORT,'immutable target'); END;
      CREATE TRIGGER inert_target_final BEFORE UPDATE ON poll_retirement_targets WHEN OLD.final_result IS NOT NULL BEGIN SELECT RAISE(ABORT,'immutable target final'); END;
      INSERT INTO registry_migrations(name) VALUES('poll_retirement_consumption_contract_v1');").unwrap();
    db.execute(
        "INSERT INTO poll_retirement_cursors VALUES(?1,1,0,'active')",
        [s.operation.to_string()],
    )
    .unwrap();
    for (ordinal, entry) in entries.iter().enumerate() {
        db.execute(
            "INSERT INTO poll_retirement_targets VALUES(?1,1,?2,?3,'waiting',NULL,'not_started')",
            params![s.operation.to_string(), ordinal as i64, entry.to_string()],
        )
        .unwrap();
    }
}

fn prepared_batch() -> SourceFixture {
    prepared_with_setup(|_| {})
}
fn prepared_with_setup(setup: impl FnOnce(&Fixture)) -> SourceFixture {
    let mut s = SourceFixture::with_setup(setup);
    a2_projections(&s.f.db());
    let first = candidate_id(&s, true, ITEM);
    let mut second = candidate_id(&s, false, "01ARZ3NDEKTSV4RRFFQ69G5FAW");
    second["ordinal"] = 1.into();
    let entries = vec![first, second];
    input(&s, &serde_json::to_string(&entries).unwrap(), 2);
    projections(&s, &entries);
    s.seal().unwrap();
    s
}

#[allow(clippy::result_large_err)]
fn run_completion(
    s: &SourceFixture,
) -> Result<(RemotePollResult, Option<batch::Live>), RemotePollError> {
    let root = s.f.root.path();
    let git = git2::Repository::open(root).unwrap();
    let tip = git.head().unwrap().target().unwrap();
    git.reference(
        "refs/remotes/origin/main",
        tip,
        true,
        "synthetic fetched primary",
    )
    .unwrap();
    let configuration =
        s.f.service
            .observation_configuration(root, &s.plan)
            .unwrap();
    assert_eq!(
        s.f.service
            .remote_safe_point(
                root,
                s.owner.as_ref().unwrap(),
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    s.f.service.poll_after_fetch_with_handoff(
        root,
        s.owner.as_ref().unwrap(),
        &s.plan,
        &configuration,
        vec![("refs/heads/main".into(), tip)],
    )
}
fn run_original(s: &SourceFixture) -> batch::Live {
    let (result, live) = run_completion(s).unwrap();
    assert!(matches!(result, RemotePollResult::Complete(_)));
    live.expect("same original successful A3 must return live C continuity, never from replay")
}

#[test]
fn retirement_batch_original_live_refusals_advance_exactly_and_last_hold_settles() {
    let s = prepared_batch();
    let live = run_original(&s);
    let db = s.f.db();
    assert_eq!(decision(&db), ("eligible".into(), None));
    assert_eq!(counts(&db), (1, 1, 1));
    let p = terminal(&db);
    let first = live.next(&s.f.service, s.f.root.path()).unwrap();
    let stale = live.next(&s.f.service, s.f.root.path()).unwrap();
    std::fs::write(
        s.f.root.path().join(format!(
            ".git/worktrees/{ITEM}/manyhands-retirement-instance"
        )),
        format!(
            "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
            OperationId::new()
        ),
    )
    .unwrap();
    let disk = disk_image(s.f.root.path());
    let report = first
        .refuse_if_changed(&s.f.service)
        .expect("actual guarded before-effect refusal");
    assert_eq!(report.code, crate::results::ResultCode::ExternalChange);
    assert_eq!(report.worktree_removal, RetirementRemoval::None);
    assert_eq!(report.branch_removal, RetirementRemoval::None);
    assert_eq!(
        counts(&db),
        (1, 1, 1),
        "B Final cannot release C's shared hold"
    );
    assert_eq!(
        db.query_row(
            "SELECT next_ordinal FROM poll_retirement_cursors",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(disk, disk_image(s.f.root.path()));
    let before = registry_image(&db);
    assert!(stale.refuse_if_changed(&s.f.service).is_err());
    assert_eq!(before, registry_image(&db));
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let earlier = git
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .parent_id(0)
        .unwrap();
    git.reference(
        "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW",
        earlier,
        true,
        "changed fixed tip",
    )
    .unwrap();
    let disk = disk_image(s.f.root.path());
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .refuse_if_changed(&s.f.service)
        .unwrap();
    assert_eq!(disk, disk_image(s.f.root.path()));
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(decision(&db), ("eligible".into(), Some("ok".into())));
    assert_eq!(terminal(&db), p);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_records WHERE phase='refused'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE state!='completed'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM retirement_observation_receipts",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    record::audit_connection(&db).unwrap();
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    assert!(live.next(&s.f.service, s.f.root.path()).is_err());
    assert!(s.capture().is_err());
    assert!(
        s.f.service
            .finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::Completed
            )
            .is_err()
    );
}

fn change_first(s: &SourceFixture) {
    std::fs::write(
        s.f.root.path().join(format!(
            ".git/worktrees/{ITEM}/manyhands-retirement-instance"
        )),
        format!(
            "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
            OperationId::new()
        ),
    )
    .unwrap();
}

#[test]
fn retirement_batch_final_replay_never_reconsumes_new_instance_or_materialization() {
    let s = prepared_batch();
    let live = run_original(&s);
    change_first(&s);
    let report = live
        .next(&s.f.service, s.f.root.path())
        .unwrap()
        .refuse_if_changed(&s.f.service)
        .unwrap();
    let db = s.f.db();
    let tip = git2::Repository::open(s.f.root.path())
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap()
        .to_string();
    db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,s.operation.to_string(),tip]).unwrap();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert_eq!(
        batch::read_refusal(&s.f.service, s.f.root.path(), s.operation, 0).unwrap(),
        Some(report)
    );
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
    assert_eq!(counts(&db), (1, 1, 1));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_batch_audit_refuses_an_unfrozen_extra_child_even_with_valid_copied_final() {
    let s = prepared_batch();
    let live = run_original(&s);
    change_first(&s);
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .refuse_if_changed(&s.f.service)
        .unwrap();
    let db = s.f.db();
    let extra = OperationId::new().to_string();
    db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT repository_id,root_path,?1,action,target,state,observed_at FROM operation_records WHERE action='retire_context'",[&extra]).unwrap();
    let local = db.last_insert_rowid();
    for table in ["context_retirement_records", "context_retirement_attempts"] {
        let columns: Vec<String> = db
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let expressions = columns
            .iter()
            .map(|name| match name.as_str() {
                "operation_ulid" => "?1".into(),
                "local_record_id" => "?2".into(),
                _ => name.clone(),
            })
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("INSERT INTO {table} SELECT {expressions} FROM {table} LIMIT 1");
        if table == "context_retirement_records" {
            db.execute(&sql, params![extra, local]).unwrap();
        } else {
            db.execute(&sql, [&extra]).unwrap();
        }
    }
    let before = registry_image(&db);
    assert!(
        record::audit_connection(&db).is_err(),
        "a matching ordinal/result is not authority for another child ID"
    );
    assert_eq!(before, registry_image(&db));
    assert!(RepositoryService::open_at(s.f.data.path()).is_err());
}

#[test]
fn retirement_batch_unchanged_cleanup_and_all_unowned_scopes_remain_closed() {
    let s = prepared_batch();
    let live = run_original(&s);
    let db = s.f.db();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert!(
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .refuse_if_changed(&s.f.service)
            .is_err(),
        "unchanged candidates cannot acquire destructive authority from this refusal seam"
    );
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
    let other = RepositoryService::open_at(s.f.data.path()).unwrap();
    assert!(live.next(&other, s.f.root.path()).is_err());
    assert_eq!(before, registry_image(&db));
    assert!(
        batch::read_refusal(&s.f.service, s.f.root.path(), s.operation, 0)
            .unwrap()
            .is_none()
    );
}

#[test]
fn retirement_batch_every_refusal_rejoins_current_f2_scope_and_complete_projection() {
    for case in [
        "request_attempt",
        "batch_attempt",
        "mapping",
        "digest",
        "cursor",
        "sibling",
        "binding",
        "marker",
        "target_missing",
        "unconfirmed",
        "ineligible",
        "local_debt",
        "comment_registration",
        "remote_slot",
        "finished",
        "cancelled",
        "source_epoch",
    ] {
        let s = prepared_batch();
        let live = run_original(&s);
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        change_first(&s);
        let db = s.f.db();
        match case {
            "request_attempt" => {
                s.f.service.enter_request(s.request_id).unwrap().unwrap();
            }
            "batch_attempt" => {
                db.execute(
                    "UPDATE poll_retirement_batches SET owner_epoch=owner_epoch+1",
                    [],
                )
                .unwrap();
            }
            "mapping" => {
                db.execute("UPDATE request_operations SET family='local'", [])
                    .unwrap();
            }
            "digest" | "binding" => {
                let ddl: String = db
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name='inert_frozen_identity'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                db.execute_batch("DROP TRIGGER inert_frozen_identity")
                    .unwrap();
                db.execute_batch(if case == "digest" {
                    "UPDATE poll_retirement_freezes SET digest=zeroblob(32)"
                } else {
                    "UPDATE poll_retirement_freezes SET binding_id=binding_id+1"
                })
                .unwrap();
                db.execute_batch(&ddl).unwrap();
            }
            "cursor" => {
                db.execute("UPDATE poll_retirement_cursors SET next_ordinal=1", [])
                    .unwrap();
            }
            "sibling" => {
                let ddl: String = db
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name='inert_target_identity'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                db.execute_batch("DROP TRIGGER inert_target_identity;UPDATE poll_retirement_targets SET candidate=(SELECT candidate FROM poll_retirement_targets WHERE ordinal=0) WHERE ordinal=1").unwrap();
                db.execute_batch(&ddl).unwrap();
            }
            "marker" => {
                db.execute("DELETE FROM registry_migrations WHERE name='poll_retirement_consumption_contract_v1'",[]).unwrap();
            }
            "target_missing" => {
                db.execute("DELETE FROM poll_retirement_targets WHERE ordinal=1", [])
                    .unwrap();
            }
            "unconfirmed" | "ineligible" => {
                let ddl: String = db
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name='inert_frozen_decision'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                db.execute_batch("DROP TRIGGER inert_frozen_decision")
                    .unwrap();
                db.execute("UPDATE poll_retirement_freezes SET eligibility=?1", [case])
                    .unwrap();
                db.execute_batch(&ddl).unwrap();
            }
            "local_debt" | "comment_registration" => {
                db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,completed_step,observed_at) SELECT id,root_path,?1,'refresh',?2,?3,1 FROM repositories",params![OperationId::new().to_string(),if case=="local_debt"{"authoritative_observed"}else{"completed"},(case=="comment_registration").then_some("comment_registration_pending")]).unwrap();
            }
            "remote_slot" => {
                s.f.service
                    .reserve_remote_operation_with_priority(
                        s.f.root.path(),
                        OperationId::new(),
                        &RemoteOperationTarget::for_poll(&s.plan),
                        RemoteOperationPriority::Manual,
                    )
                    .unwrap();
            }
            "finished" => {
                s.f.service
                    .finish_request(
                        s.request_id,
                        1,
                        &RequestResult {
                            outcome: crate::results::Outcome::Success,
                            code: crate::results::ResultCode::Ok,
                            effects: crate::results::Effects::not_requested(),
                            data: None,
                        },
                    )
                    .unwrap();
            }
            "cancelled" => {
                db.execute("UPDATE request_records SET cancel_requested=1", [])
                    .unwrap();
            }
            "source_epoch" => {
                db.execute(
                    "UPDATE remote_operation_records SET owner_epoch=owner_epoch+1",
                    [],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(child.refuse_if_changed(&s.f.service).is_err(), "{case}");
        assert_eq!(before, registry_image(&db), "{case}");
        assert_eq!(disk, disk_image(s.f.root.path()), "{case}");
        assert_eq!(counts(&db), (1, 1, 1));
    }
}

#[test]
fn retirement_batch_current_advertisement_generation_and_original_process_identity_are_required() {
    for case in ["advertisement", "generation", "process_configuration"] {
        let s = prepared_with_setup(|f| {
            let path = f.data.path().join("batch-synthetic-source");
            std::fs::write(&path, b"synthetic metadata only").unwrap();
            let crate::repository::keys::RegisterSharedKeyOutcome::Registered(key) = f
                .service
                .register_shared_key(crate::repository::keys::RegisterSharedKeyRequest {
                    label: "batch fixture".into(),
                    ownership: crate::repository::keys::SharedKeyOwnership::Imported,
                    private_key_path: path,
                    public_key_path: None,
                })
                .unwrap()
            else {
                panic!("synthetic registration");
            };
            f.service.select_shared_key(key.id).unwrap();
        });
        let live = run_original(&s);
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        change_first(&s);
        match case {
            "advertisement" => {
                state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                    state::persist_complete_advertisement(
                        tx,
                        id,
                        &s.plan,
                        state::generation(tx, id)?,
                        &[],
                        999,
                    )
                })
                .unwrap();
            }
            "generation" => {
                state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                    state::configure(tx, id, None, true)
                })
                .unwrap();
            }
            "process_configuration" => {
                let original =
                    s.f.service
                        .observation_configuration(s.f.root.path(), &s.plan)
                        .unwrap();
                let generation: i64 =
                    s.f.db()
                        .query_row(
                            "SELECT configuration_generation FROM remote_polling_state",
                            [],
                            |r| r.get(0),
                        )
                        .unwrap();
                std::fs::write(
                    s.f.data.path().join("batch-synthetic-source"),
                    b"externally replaced longer synthetic metadata source",
                )
                .unwrap();
                let current =
                    s.f.service
                        .observation_configuration(s.f.root.path(), &s.plan)
                        .unwrap();
                assert!(current != original);
                assert_eq!(current.endpoint_digest(), original.endpoint_digest());
                assert_eq!(
                    s.f.db()
                        .query_row(
                            "SELECT configuration_generation FROM remote_polling_state",
                            [],
                            |r| r.get::<_, i64>(0)
                        )
                        .unwrap(),
                    generation
                );
            }
            _ => unreachable!(),
        }
        let before = registry_image(&s.f.db());
        let disk = disk_image(s.f.root.path());
        assert!(child.refuse_if_changed(&s.f.service).is_err(), "{case}");
        assert_eq!(before, registry_image(&s.f.db()));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_batch_each_refusal_write_and_last_release_roll_back_together() {
    for boundary in [
        "envelope",
        "companion",
        "attempt",
        "target",
        "cursor",
        "batch_final",
        "last_release",
    ] {
        let s = prepared_batch();
        let live = run_original(&s);
        change_first(&s);
        let last = matches!(boundary, "batch_final" | "last_release");
        if last {
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .refuse_if_changed(&s.f.service)
                .unwrap();
            let git = git2::Repository::open(s.f.root.path()).unwrap();
            let tip = git
                .head()
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .parent_id(0)
                .unwrap();
            git.reference(
                "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW",
                tip,
                true,
                "changed fixed tip",
            )
            .unwrap();
        }
        let db = s.f.db();
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        if boundary == "last_release" {
            record::set_release_failure(true);
        } else {
            let (table, event) = match boundary {
                "envelope" => ("operation_records", "INSERT"),
                "companion" => ("context_retirement_records", "INSERT"),
                "attempt" => ("context_retirement_attempts", "INSERT"),
                "target" => ("poll_retirement_targets", "UPDATE"),
                "cursor" => ("poll_retirement_cursors", "UPDATE"),
                "batch_final" => ("poll_retirement_freezes", "UPDATE"),
                _ => unreachable!(),
            };
            db.execute_batch(&format!("CREATE TRIGGER fail_batch BEFORE {event} ON {table} BEGIN SELECT RAISE(ABORT,'batch checkpoint'); END")).unwrap();
        }
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(child.refuse_if_changed(&s.f.service).is_err(), "{boundary}");
        assert_eq!(before, registry_image(&db), "{boundary}");
        assert_eq!(disk, disk_image(s.f.root.path()));
        if boundary == "last_release" {
            assert!(
                !record::set_release_failure(false),
                "actual consumer Z boundary must consume the fault"
            );
        } else {
            db.execute_batch("DROP TRIGGER fail_batch").unwrap();
        }
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .refuse_if_changed(&s.f.service)
            .unwrap();
        assert_eq!(counts(&db), if last { (1, 0, 2) } else { (1, 1, 1) });
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_batch_terminal_release_replay_never_resolves_a_reused_observation_id() {
    let s = prepared_batch();
    let live = run_original(&s);
    change_first(&s);
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .refuse_if_changed(&s.f.service)
        .unwrap();
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let tip = git
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .parent_id(0)
        .unwrap();
    git.reference(
        "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW",
        tip,
        true,
        "changed fixed tip",
    )
    .unwrap();
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .refuse_if_changed(&s.f.service)
        .unwrap();
    let db = s.f.db();
    let (provenance,binding,old):(i64,i64,i64)=db.query_row("SELECT f.provenance_id,f.binding_id,p.observation_batch_id FROM poll_retirement_freezes f JOIN retirement_advertisement_provenance p ON p.id=f.provenance_id",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        state::configure(tx, id, None, true)
    })
    .unwrap();
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        let config =
            crate::repository::remote::observation::ObservationConfiguration::capture_owned(
                &s.f.service,
                tx,
                s.f.root.path(),
                &s.plan,
            )?;
        state::configure_endpoints(tx, id, &s.plan, &config.endpoint_digest())?;
        // Three bounded real store writes exercise the released ID (the original
        // full poll followed two fixture/synthetic previous-history batches).
        let _ = state::persist_complete_advertisement(
            tx,
            id,
            &s.plan,
            state::generation(tx, id)?,
            &[],
            997,
        )?;
        let _ = state::persist_complete_advertisement(
            tx,
            id,
            &s.plan,
            state::generation(tx, id)?,
            &[],
            998,
        )?;
        let new = state::persist_complete_advertisement(
            tx,
            id,
            &s.plan,
            state::generation(tx, id)?,
            &[],
            999,
        )?;
        assert_eq!(new, old);
        Ok(())
    })
    .unwrap();
    let before = registry_image(&db);
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, _| {
        record::release_bound_source(tx, provenance, binding, true)
    })
    .unwrap();
    assert_eq!(before, registry_image(&db));
    assert!(
        batch::read_refusal(&s.f.service, s.f.root.path(), s.operation, 0)
            .unwrap()
            .is_some()
    );
    assert_eq!(before, registry_image(&db));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_batch_empty_original_with_consumption_projection_still_settles_without_child() {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    input(&s, "[]", 0);
    projections(&s, &[]);
    s.seal().unwrap();
    let (result, live) =
        run_completion(&s).expect("explicit frozen empty remains a successful original handoff");
    assert!(matches!(result, RemotePollResult::Complete(_)));
    assert!(live.is_none());
    let db = s.f.db();
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(decision(&db), ("eligible".into(), Some("ok".into())));
    assert_eq!(
        db.query_row("SELECT stage FROM poll_retirement_cursors", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "complete"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_batch_empty_cursor_settlement_is_atomic_and_recovery_never_mints_c() {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    input(&s, "[]", 0);
    projections(&s, &[]);
    s.seal().unwrap();
    let db = s.f.db();
    db.execute_batch("CREATE TRIGGER fail_empty_cursor BEFORE UPDATE ON poll_retirement_cursors BEGIN SELECT RAISE(ABORT,'empty cursor checkpoint'); END").unwrap();
    assert!(run_completion(&s).is_err());
    assert_eq!(decision(&db), ("unconfirmed".into(), None));
    assert_eq!(counts(&db), (1, 2, 0));
    assert_eq!(
        db.query_row("SELECT stage FROM poll_retirement_cursors", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "active"
    );
    db.execute_batch("DROP TRIGGER fail_empty_cursor").unwrap();
    recover(&s);
    assert_eq!(
        decision(&db),
        ("ineligible".into(), Some("discovery_pending".into()))
    );
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(
        db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    let before = registry_image(&db);
    recover(&s);
    assert_eq!(before, registry_image(&db));
}
