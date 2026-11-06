//! Inert A2 producer projections, synthetic advertisements/real ODB and real
//! owned poll SQL/refresh. NOT Task 4 selection, enrollment or coordination.
use super::*;
#[path = "source_handoff_specfix_tests.rs"]
mod specfix;

fn a2_projections(db: &Connection) {
    db.execute_batch("CREATE TABLE poll_retirement_freeze_inputs (
      operation_ulid TEXT NOT NULL, owner_epoch INTEGER NOT NULL,
      request_attempt INTEGER NOT NULL, source_owner_epoch INTEGER NOT NULL,
      configuration_generation INTEGER NOT NULL, candidate_count INTEGER NOT NULL,
      image TEXT NOT NULL, PRIMARY KEY(operation_ulid,owner_epoch));
      CREATE TABLE poll_retirement_freezes (
      operation_ulid TEXT NOT NULL, owner_epoch INTEGER NOT NULL,
      provenance_id INTEGER NOT NULL UNIQUE, binding_id INTEGER NOT NULL UNIQUE,
      capture_nonce TEXT NOT NULL UNIQUE, candidate_count INTEGER NOT NULL,
      image TEXT NOT NULL, original_report TEXT NOT NULL, digest BLOB NOT NULL,
      eligibility TEXT NOT NULL CHECK(eligibility IN ('unconfirmed','eligible','ineligible')),
      final_code TEXT, PRIMARY KEY(operation_ulid,owner_epoch));
      CREATE TRIGGER inert_frozen_identity BEFORE UPDATE OF operation_ulid,owner_epoch,provenance_id,binding_id,capture_nonce,candidate_count,image,original_report,digest ON poll_retirement_freezes BEGIN SELECT RAISE(ABORT,'immutable frozen image'); END;
      CREATE TRIGGER inert_frozen_decision BEFORE UPDATE ON poll_retirement_freezes WHEN OLD.eligibility!='unconfirmed' BEGIN SELECT RAISE(ABORT,'immutable handoff'); END;
      INSERT INTO registry_migrations(name) VALUES('poll_retirement_handoff_contract_v1');").unwrap();
}

fn input(s: &SourceFixture, image: &str, count: i64) {
    s.f.db().execute("INSERT INTO poll_retirement_freeze_inputs SELECT a.operation_ulid,a.owner_epoch,a.request_attempt,a.source_owner_epoch,b.configuration_generation,?2,?3 FROM poll_retirement_attempts a JOIN poll_retirement_batches b USING(operation_ulid) WHERE a.operation_ulid=?1",params![s.operation.to_string(),count,image]).unwrap();
}

#[allow(clippy::result_large_err)] // Mirrors the existing poll seam's typed error.
fn run(s: &SourceFixture) -> Result<RemotePollResult, RemotePollError> {
    let root = s.f.root.path();
    let git = git2::Repository::open(root).unwrap();
    let tip = git.head().unwrap().target().unwrap();
    git.reference(
        "refs/remotes/origin/main",
        tip,
        true,
        "inert fetched primary",
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
    s.f.service.poll_after_fetch(
        root,
        s.owner.as_ref().unwrap(),
        &s.plan,
        &configuration,
        vec![("refs/heads/main".into(), tip)],
    )
}

fn candidate(s: &SourceFixture, present: bool) -> serde_json::Value {
    candidate_id(s, present, ITEM)
}
fn candidate_id(s: &SourceFixture, present: bool, item: &str) -> serde_json::Value {
    let item_id = item;
    let root = s.f.root.path();
    let git = git2::Repository::open(root).unwrap();
    let path = root.join(format!(".manyhands/tickets/{item}/ticket.md"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,format!("---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {item}\ntitle: Closed\ntype: task\nstatus: closed\nclosed_at: '2026-11-04T00:00:00Z'\nclosed_by: Fixture\n---\nBody\n")).unwrap();
    let mut index = git.index().unwrap();
    index.add_path(path.strip_prefix(root).unwrap()).unwrap();
    index.write().unwrap();
    let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = git.head().unwrap().peel_to_commit().unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let tip = git
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "closed fixture",
            &tree,
            &[&parent],
        )
        .unwrap();
    let branch = git
        .branch(
            &format!("manyhands/ticket/{item}"),
            &git.find_commit(tip).unwrap(),
            false,
        )
        .unwrap()
        .into_reference();
    let old: Vec<_> = git
        .branches(Some(git2::BranchType::Local))
        .unwrap()
        .map(|b| b.unwrap().0.into_reference())
        .filter_map(|r| {
            RemoteRefObservation::from_advertisement(
                &s.plan,
                r.name().unwrap(),
                r.target().unwrap(),
                r.target(),
            )
        })
        .collect();
    state::with_transaction(&s.f.service, root, |tx, id| {
        state::persist_complete_advertisement(tx, id, &s.plan, state::generation(tx, id)?, &old, 42)
    })
    .unwrap();
    let nonce = OperationId::new();
    if present {
        let worktree = root.join(".manyhands/worktrees").join(item);
        std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&branch));
        git.worktree(item, &worktree, Some(&options)).unwrap();
        // A pre-enrolled original instance supplied by the inert producer. This
        // test does NOT run Task 4's future enrollment loop.
        std::fs::write(
            git.commondir()
                .join("worktrees")
                .join(item)
                .join("manyhands-retirement-instance"),
            format!("manyhands-retirement-instance-v1\nticket\n{item}\n{nonce}\n"),
        )
        .unwrap();
    }
    serde_json::json!({"ordinal":0,"kind":"ticket","item_id":item_id,"initial_resources":if present{"present"}else{"branch_only"},"expected_tip":tip.to_string(),"selection_epoch":1,"instance_nonce":present.then(||nonce.to_string()),"instance_verified":present,"child_operation_id":OperationId::new().to_string(),"materialization_capture":"absent","materialization_operation_id":null,"materialization_oid":null,"materialization_step":null})
}

#[test]
fn retirement_source_handoff_nonempty_keeps_consumer_hold_and_never_begins_children() {
    for present in [false, true] {
        let mut s = SourceFixture::new();
        a2_projections(&s.f.db());
        let entry = candidate(&s, present);
        let image = serde_json::to_string(&vec![entry.clone()]).unwrap();
        input(&s, &image, 1);
        s.seal().unwrap();
        let before = disk_image(s.f.root.path());
        let result = run(&s).expect("complete nonempty frozen image");
        assert!(
            matches!(result,RemotePollResult::Complete(RemotePollOutcome::Polled(ref p)) if p.completion()==PollCompletion::Completed)
        );
        let db = s.f.db();
        assert_eq!(
            db.query_row("SELECT eligibility FROM poll_retirement_freezes", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "eligible"
        );
        assert_eq!(
            db.query_row("SELECT image FROM poll_retirement_freezes", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            image
        );
        assert_eq!(
            db.query_row(
                "SELECT pin_kind FROM retirement_observation_pins",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "consumer_attempt"
        );
        assert_eq!(
            db.query_row(
                "SELECT reason FROM retirement_observation_pin_releases",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "handoff_decided"
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        // No removal/enrollment. The poll may write only its tracking/ref logs.
        for (path, node) in before {
            if path.starts_with(format!(".manyhands/worktrees/{ITEM}")) {
                assert_eq!(disk_image(s.f.root.path()).get(&path), Some(&node));
            }
        }
        let child = OperationId::parse(entry["child_operation_id"].as_str().unwrap()).unwrap();
        let frozen = registry_image(&db);
        assert!(
            s.f.service
                .refresh_repository(RefreshRepositoryRequest {
                    root: s.f.root.path().into(),
                    operation_id: child
                })
                .is_err(),
            "frozen child IDs remain globally reserved, not generic refresh authority"
        );
        assert!(
            s.f.service
                .retire_context(RetireContextRequest {
                    root: s.f.root.path().into(),
                    operation_id: child,
                    kind: AuthoringKind::Ticket,
                    item_id: ITEM.parse().unwrap()
                })
                .is_err(),
            "batch child begin remains closed even after eligible handoff"
        );
        assert_eq!(frozen, registry_image(&db));
        record::audit_connection(&db).unwrap();
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

fn prepared(nonempty: bool) -> SourceFixture {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let image = if nonempty {
        vec![candidate(&s, true)]
    } else {
        vec![]
    };
    input(
        &s,
        &serde_json::to_string(&image).unwrap(),
        image.len() as i64,
    );
    s.seal().unwrap();
    s
}
fn counts(db: &Connection) -> (i64, i64, i64) {
    db.query_row("SELECT (SELECT count(*) FROM poll_retirement_freezes),(SELECT count(*) FROM retirement_observation_pins),(SELECT count(*) FROM retirement_observation_pin_releases)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap()
}
fn decision(db: &Connection) -> (String, Option<String>) {
    db.query_row(
        "SELECT eligibility,final_code FROM poll_retirement_freezes",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}
fn terminal(db: &Connection) -> Vec<rusqlite::types::Value> {
    db.query_row("SELECT phase,outcome,poll_completed_at,poll_next_eligible_at,owner_epoch,cancel_requested,yield_requested FROM remote_operation_records",[],|r|(0..r.as_ref().column_count()).map(|i|r.get(i)).collect()).unwrap()
}
fn recover(s: &SourceFixture) {
    crate::repository::context_retirement::handoff::recover_unconfirmed(
        &s.f.service,
        s.f.root.path(),
        s.operation,
    )
    .unwrap();
}
fn after_completion(s: &SourceFixture, sql: String) {
    let path = s.f.data.path().join(REGISTRY_FILE);
    s.f.service
        .set_poll_boundary_hook_for_testing(move |point, _| {
            if point == FailurePoint::PollAfterCompletion {
                let db = Connection::open(&path).unwrap();
                assert_eq!(decision(&db), ("unconfirmed".into(), None));
                assert_eq!(counts(&db), (1, 2, 0));
                assert_eq!(
                    terminal(&db)[0],
                    rusqlite::types::Value::Text("completed".into())
                );
                db.execute_batch(&sql).unwrap();
            }
        });
}

struct NoCredentials;
impl crate::repository::keys::SessionCredentialProvider for NoCredentials {
    fn request_passphrase(
        &mut self,
        _: &crate::repository::keys::UnlockRequest,
    ) -> crate::repository::keys::PassphraseResponse {
        panic!("replay cannot request transport credentials")
    }
}
fn replay(service: &RepositoryService, s: &SourceFixture) -> RemotePollResult {
    service
        .poll_publication_remote(
            PollPublicationRemoteRequest {
                root: s.f.root.path().into(),
                operation_id: s.operation,
                invocation: RemotePollInvocation::Explicit,
                approval: None,
                restart: false,
            },
            &mut crate::repository::keys::SessionCredentials::new(NoCredentials),
        )
        .unwrap()
}

#[test]
fn retirement_source_handoff_original_index_failure_closes_both_and_repair_cannot_upgrade() {
    let s = prepared(true);
    *s.f.service.failure_point.lock().unwrap() = Some(FailurePoint::BeforeIndexTransactionCommit);
    let result = run(&s).expect("failed original refresh is a settled no-retirement return");
    assert!(matches!(result, RemotePollResult::IndexPending(_)));
    let db = s.f.db();
    assert_eq!(
        decision(&db),
        ("ineligible".into(), Some("discovery_pending".into()))
    );
    assert_eq!(counts(&db), (1, 0, 2));
    let public = terminal(&db);
    let refresh:String=db.query_row("SELECT operation_ulid FROM operation_records WHERE action='refresh' AND state!='completed'",[],|r|r.get(0)).unwrap();
    let repaired = RepositoryService::open_at(s.f.data.path()).unwrap();
    assert!(matches!(
        repaired
            .refresh_repository(RefreshRepositoryRequest {
                root: s.f.root.path().into(),
                operation_id: OperationId::parse(&refresh).unwrap()
            })
            .unwrap(),
        RefreshOutcome::Refreshed { .. }
    ));
    assert!(
        matches!(replay(&repaired, &s), RemotePollResult::Complete(_)),
        "ordinary public replay may reflect index repair, never old retirement eligibility"
    );
    assert_eq!(
        decision(&db),
        ("ineligible".into(), Some("discovery_pending".into()))
    );
    assert_eq!(public, terminal(&db));
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(
        db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_source_handoff_lost_capture_after_completed_p_is_ineligible_on_public_replay() {
    let s = prepared(true);
    *s.f.service.failure_point.lock().unwrap() = Some(FailurePoint::PollAfterCompletion);
    assert!(run(&s).is_err());
    let db = s.f.db();
    assert_eq!(decision(&db), ("unconfirmed".into(), None));
    assert_eq!(counts(&db), (1, 2, 0));
    let public = terminal(&db);
    let reopened = RepositoryService::open_at(s.f.data.path()).unwrap();
    reopened
        .refresh_repository(RefreshRepositoryRequest {
            root: s.f.root.path().into(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert!(matches!(
        replay(&reopened, &s),
        RemotePollResult::Complete(_)
    ));
    assert_eq!(
        decision(&db),
        ("ineligible".into(), Some("discovery_pending".into()))
    );
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(public, terminal(&db));
    let before = registry_image(&db);
    replay(&reopened, &s);
    assert_eq!(before, registry_image(&db));
}

#[test]
fn retirement_source_handoff_preterminal_binding_and_finish_rollback_as_one_image() {
    for (name, sql) in [
        (
            "binding",
            "CREATE TRIGGER fail_a2 BEFORE INSERT ON retirement_observation_receipts BEGIN SELECT RAISE(ABORT,'a2 binding'); END;",
        ),
        (
            "consumer_pin",
            "CREATE TRIGGER fail_a2 BEFORE INSERT ON retirement_observation_pins WHEN NEW.pin_kind='consumer_attempt' BEGIN SELECT RAISE(ABORT,'a2 pin'); END;",
        ),
        (
            "image",
            "CREATE TRIGGER fail_a2 BEFORE INSERT ON poll_retirement_freezes BEGIN SELECT RAISE(ABORT,'a2 image'); END;",
        ),
        (
            "stage",
            "CREATE TRIGGER fail_a2 BEFORE UPDATE OF stage ON poll_retirement_batches WHEN NEW.stage='frozen' BEGIN SELECT RAISE(ABORT,'a2 stage'); END;",
        ),
        (
            "finish",
            "CREATE TRIGGER fail_a2 BEFORE UPDATE OF phase ON remote_operation_records WHEN NEW.phase='completed' BEGIN SELECT CASE WHEN (SELECT count(*) FROM retirement_observation_pins)!=2 OR (SELECT eligibility FROM poll_retirement_freezes)!='unconfirmed' OR OLD.phase='completed' THEN RAISE(ABORT,'wrong preterminal lifetime') END; SELECT RAISE(ABORT,'a2 finish'); END;",
        ),
    ] {
        let s = prepared(false);
        s.f.db().execute_batch(sql).unwrap();
        assert!(run(&s).is_err(), "{name}");
        let db = s.f.db();
        assert_eq!(counts(&db), (0, 1, 0), "{name}");
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM retirement_observation_receipts",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_ne!(
            terminal(&db)[0],
            rusqlite::types::Value::Text("completed".into())
        );
        assert_eq!(
            db.query_row("SELECT stage FROM poll_retirement_batches", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "unplanned"
        );
        record::audit_connection(&db).unwrap();
        db.execute_batch("DROP TRIGGER fail_a2").unwrap();
        s.f.service
            .cancel_remote_operation(s.f.root.path(), s.operation)
            .expect(name);
        // BeforeDiscovery is already acknowledged; use the supported finish,
        // not a new permit to repeat the same action checkpoint.
        s.f.service
            .finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::Cancelled,
            )
            .expect(name);
        assert_eq!(counts(&db), (0, 0, 1));
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_source_handoff_invalid_complete_images_never_freeze_or_finish_p() {
    for case in [
        "truncated",
        "duplicate",
        "ordinal",
        "nonce",
        "tip",
        "child_parent",
        "child_foreign",
        "unknown",
        "materialization",
        "missing_projection",
    ] {
        let mut s = SourceFixture::new();
        a2_projections(&s.f.db());
        let mut k = candidate(&s, true);
        let mut entries = vec![k.clone()];
        let mut count = 1;
        match case {
            "truncated" => count = 2,
            "duplicate" => {
                entries.push(k.clone());
                count = 2;
            }
            "ordinal" => {
                k["ordinal"] = 1.into();
                entries = vec![k];
            }
            "nonce" => {
                k["instance_nonce"] = OperationId::new().to_string().into();
                entries = vec![k];
            }
            "tip" => {
                k["expected_tip"] = "1".repeat(40).into();
                entries = vec![k];
            }
            "child_parent" => {
                k["child_operation_id"] = s.operation.to_string().into();
                entries = vec![k];
            }
            "child_foreign" => {
                let child = k["child_operation_id"].as_str().unwrap();
                s.f.db().execute("INSERT INTO key_registry_changes(operation_id,action,changed,recorded_at) VALUES(?1,'clear',0,1)",[child]).unwrap();
            }
            "unknown" => {
                k["invented_flag"] = true.into();
                entries = vec![k];
            }
            "materialization" => {
                k["materialization_capture"] = "complete".into();
                entries = vec![k];
            }
            "missing_projection" => {}
            _ => unreachable!(),
        }
        input(&s, &serde_json::to_string(&entries).unwrap(), count);
        if case == "missing_projection" {
            s.f.db()
                .execute_batch("DROP TABLE poll_retirement_freeze_inputs")
                .unwrap();
        }
        s.seal().unwrap();
        let disk = disk_image(s.f.root.path());
        assert!(run(&s).is_err(), "{case}");
        let db = s.f.db();
        assert_eq!(counts(&db), (0, 1, 0), "{case}");
        assert_ne!(
            terminal(&db)[0],
            rusqlite::types::Value::Text("completed".into())
        );
        for (path, node) in disk {
            if path.starts_with(format!(".manyhands/worktrees/{ITEM}")) {
                assert_eq!(disk_image(s.f.root.path()).get(&path), Some(&node));
            }
        }
    }
}

#[test]
fn retirement_source_handoff_current_cancel_closes_but_stale_request_cannot_ack() {
    for stale in [false, true] {
        let s = prepared(true);
        let sql = if stale {
            "UPDATE request_records SET attempt=attempt+1,cancel_requested=1;"
        } else {
            "UPDATE request_records SET cancel_requested=1;"
        };
        after_completion(&s, sql.into());
        let result = run(&s);
        let db = s.f.db();
        let public = terminal(&db);
        if stale {
            assert!(result.is_err());
            assert_eq!(counts(&db), (1, 2, 0));
            assert_eq!(decision(&db), ("unconfirmed".into(), None));
            recover(&s);
        } else {
            assert!(result.is_ok());
            assert_eq!(
                decision(&db),
                ("ineligible".into(), Some("cancelled".into()))
            );
        }
        assert_eq!(counts(&db), (1, 0, 2));
        assert_eq!(terminal(&db), public);
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_source_handoff_ended_p_cannot_write_and_duplicate_freeze_is_immutable() {
    let s = prepared(true);
    run(&s).unwrap();
    let db = s.f.db();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert!(s.capture().is_err());
    assert!(
        s.f.service
            .remote_safe_point(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOperationSafePoint::BeforeDiscovery
            )
            .is_err()
    );
    assert!(
        s.f.service
            .finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::Completed
            )
            .is_err()
    );
    assert!(
        s.f.service
            .finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::Cancelled
            )
            .is_err()
    );
    assert!(
        db.execute("UPDATE poll_retirement_freezes SET image='[]'", [])
            .is_err()
    );
    assert!(
        db.execute(
            "INSERT INTO poll_retirement_freezes SELECT * FROM poll_retirement_freezes",
            []
        )
        .is_err()
    );
    assert!(
        db.execute(
            "UPDATE retirement_observation_receipts SET frozen_worklist_digest=zeroblob(32)",
            []
        )
        .is_err()
    );
    let (provenance, binding): (i64, i64) = db
        .query_row(
            "SELECT provenance_id,binding_id FROM poll_retirement_freezes",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        state::with_transaction(&s.f.service, s.f.root.path(), |tx, _| {
            record::release_bound_source(tx, provenance, binding, true)
        })
        .is_err(),
        "active eligible consumer is not terminal source release authority"
    );
    replay(&s.f.service, &s);
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
}

#[test]
fn retirement_source_handoff_whole_ordered_two_candidate_image_and_report_survive_latest_replacement()
 {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let first = candidate_id(&s, true, ITEM);
    let mut second = candidate_id(&s, false, "01ARZ3NDEKTSV4RRFFQ69G5FAW");
    second["ordinal"] = 1.into();
    let entries = vec![first, second];
    let image = serde_json::to_string(&entries).unwrap();
    input(&s, &image, 2);
    s.seal().unwrap();
    let original = run(&s).unwrap();
    let db = s.f.db();
    let original_image: String = db
        .query_row(
            "SELECT original_report FROM poll_retirement_freezes",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let old: i64 = db
        .query_row(
            "SELECT observation_batch_id FROM retirement_observation_pins",
            [],
            |r| r.get(0),
        )
        .unwrap();
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        state::persist_complete_advertisement(tx, id, &s.plan, state::generation(tx, id)?, &[], 999)
    })
    .unwrap();
    assert_eq!(replay(&s.f.service, &s), original);
    assert_eq!(
        db.query_row("SELECT image FROM poll_retirement_freezes", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        image
    );
    assert_eq!(
        db.query_row(
            "SELECT original_report FROM poll_retirement_freezes",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        original_image
    );
    assert!(
        db.query_row(
            "SELECT EXISTS(SELECT 1 FROM remote_observation_batches WHERE id=?1 AND is_current=0)",
            [old],
            |r| r.get::<_, bool>(0)
        )
        .unwrap(),
        "consumer hold retains superseded advertisement"
    );
    record::audit_connection(&db).unwrap();
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_source_handoff_postterminal_failures_and_z_rollback_preserve_unconfirmed_for_recovery()
 {
    for release in [false, true] {
        let s = prepared(true);
        let path = s.f.data.path().join(REGISTRY_FILE);
        s.f.service.set_poll_boundary_hook_for_testing(move |point,_|if point==FailurePoint::PollAfterCompletion {
            if release {record::set_release_failure(true);}else{Connection::open(&path).unwrap().execute_batch("CREATE TRIGGER fail_handoff BEFORE UPDATE OF eligibility ON poll_retirement_freezes BEGIN SELECT RAISE(ABORT,'handoff fail'); END;").unwrap();}
        });
        assert!(run(&s).is_err());
        let db = s.f.db();
        let public = terminal(&db);
        assert_eq!(decision(&db), ("unconfirmed".into(), None));
        assert_eq!(counts(&db), (1, 2, 0));
        if release {
            assert!(
                !record::set_release_failure(false),
                "must reach actual Z failure boundary"
            );
        } else {
            db.execute_batch("DROP TRIGGER fail_handoff").unwrap();
        }
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
        recover(&s);
        assert_eq!(counts(&db), (1, 0, 2));
        assert_eq!(public, terminal(&db));
        let before = registry_image(&db);
        recover(&s);
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_source_handoff_current_batch_config_and_instance_must_survive_refresh_gap() {
    for case in [
        "batch",
        "generation",
        "endpoint",
        "nonce",
        "tip",
        "finished_request",
        "other_remote",
    ] {
        let s = prepared(true);
        let data = s.f.data.path().to_path_buf();
        let root = s.f.root.path().to_path_buf();
        let plan = s.plan.clone();
        s.f.service
            .set_poll_boundary_hook_for_testing(move |point, _| {
                if point == FailurePoint::PollAfterCompletion {
                    let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
                    assert_eq!(counts(&db), (1, 2, 0));
                    match case {
                        "batch" => {
                            let service = RepositoryService::open_at(&data).unwrap();
                            state::with_transaction(&service, &root, |tx, id| {
                                state::persist_complete_advertisement(
                                    tx,
                                    id,
                                    &plan,
                                    state::generation(tx, id)?,
                                    &[],
                                    999,
                                )
                            })
                            .unwrap();
                        }
                        "generation" => {
                            let service = RepositoryService::open_at(&data).unwrap();
                            state::with_transaction(&service, &root, |tx, id| {
                                state::configure(tx, id, None, true)
                            })
                            .unwrap();
                        }
                        "endpoint" => {
                            git2::Repository::open(&root)
                                .unwrap()
                                .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/changed")
                                .unwrap();
                        }
                        "nonce" => {
                            std::fs::write(
                                root.join(format!(
                                    ".git/worktrees/{ITEM}/manyhands-retirement-instance"
                                )),
                                format!(
                                    "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
                                    OperationId::new()
                                ),
                            )
                            .unwrap();
                        }
                        "tip" => {
                            let git = git2::Repository::open(&root).unwrap();
                            let old = git
                                .head()
                                .unwrap()
                                .peel_to_commit()
                                .unwrap()
                                .parent_id(0)
                                .unwrap();
                            git.reference(
                                &format!("refs/heads/manyhands/ticket/{ITEM}"),
                                old,
                                true,
                                "changed fixture",
                            )
                            .unwrap();
                        }
                        "finished_request" => {
                            let request: String = db
                                .query_row("SELECT request_ulid FROM request_records", [], |r| {
                                    r.get(0)
                                })
                                .unwrap();
                            let service = RepositoryService::open_at(&data).unwrap();
                            assert!(
                                service
                                    .finish_request(
                                        RequestId::parse(&request).unwrap(),
                                        1,
                                        &RequestResult {
                                            outcome: crate::results::Outcome::Success,
                                            code: crate::results::ResultCode::Ok,
                                            effects: crate::results::Effects::not_requested(),
                                            data: None
                                        }
                                    )
                                    .unwrap()
                            );
                        }
                        "other_remote" => {
                            let service = RepositoryService::open_at(&data).unwrap();
                            service
                                .reserve_remote_operation_with_priority(
                                    &root,
                                    OperationId::new(),
                                    &RemoteOperationTarget::for_poll(&plan),
                                    RemoteOperationPriority::Manual,
                                )
                                .unwrap();
                        }
                        _ => unreachable!(),
                    }
                }
            });
        let result = run(&s);
        let db = s.f.db();
        assert_ne!(
            decision(&db).0,
            "eligible",
            "{case} must not grant eligibility"
        );
        if case == "other_remote" {
            assert!(
                matches!(result, Ok(RemotePollResult::IndexPending(_))),
                "the other slot stops original refresh; negative metadata handoff remains permitted"
            );
            assert_eq!(
                decision(&db),
                ("ineligible".into(), Some("discovery_pending".into()))
            );
        } else {
            assert!(result.is_err(), "{case}");
            assert_eq!(decision(&db), ("unconfirmed".into(), None), "{case}");
            assert_eq!(counts(&db), (1, 2, 0));
        }
        let public = terminal(&db);
        recover(&s);
        assert_eq!(
            decision(&db),
            ("ineligible".into(), Some("discovery_pending".into()))
        );
        assert_eq!(counts(&db), (1, 0, 2));
        assert_eq!(public, terminal(&db));
        assert!(
            RepositoryService::open_at(s.f.data.path()).is_ok(),
            "{case}"
        );
    }
}

#[test]
fn retirement_source_handoff_failed_cancelled_interrupted_full_pass_does_not_freeze() {
    for case in ["failure", "request_cancel", "remote_cancel", "yield"] {
        let s = prepared(true);
        let path = s.f.data.path().join(REGISTRY_FILE);
        if case == "failure" {
            *s.f.service.failure_point.lock().unwrap() = Some(FailurePoint::PollBeforeTarget);
        } else {
            s.f.service
                .set_poll_boundary_hook_for_testing(move |point, _| {
                    if point == FailurePoint::PollBeforeDiscovery {
                        Connection::open(&path)
                            .unwrap()
                            .execute_batch(match case {
                                "request_cancel" => "UPDATE request_records SET cancel_requested=1",
                                "remote_cancel" => {
                                    "UPDATE remote_operation_records SET cancel_requested=1"
                                }
                                _ => "UPDATE remote_operation_records SET yield_requested=1",
                            })
                            .unwrap();
                    }
                });
        }
        let result = run(&s);
        let db = s.f.db();
        assert_eq!(counts(&db).0, 0);
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM retirement_observation_receipts",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        if case == "failure" {
            assert!(result.is_err());
            assert_eq!(counts(&db), (0, 1, 0));
        } else {
            assert!(result.is_ok());
            assert_eq!(counts(&db), (0, 0, 1));
        }
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_source_handoff_bound_last_release_collects_once_and_replay_ignores_reused_id() {
    let s = prepared(false);
    *s.f.service.failure_point.lock().unwrap() = Some(FailurePoint::PollAfterCompletion);
    assert!(run(&s).is_err());
    let db = s.f.db();
    let old: i64 = db
        .query_row(
            "SELECT observation_batch_id FROM retirement_observation_pins LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        state::configure(tx, id, None, true)
    })
    .unwrap();
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    let public = terminal(&db);
    record::set_release_failure(true);
    assert!(
        crate::repository::context_retirement::handoff::recover_unconfirmed(
            &s.f.service,
            s.f.root.path(),
            s.operation
        )
        .is_err()
    );
    assert!(!record::set_release_failure(false));
    assert_eq!(counts(&db), (1, 2, 0));
    recover(&s);
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(public, terminal(&db));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_observation_batches WHERE id=?1",
            [old],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        let configuration =
            crate::repository::remote::observation::ObservationConfiguration::capture_owned(
                &s.f.service,
                tx,
                s.f.root.path(),
                &s.plan,
            )?;
        state::configure_endpoints(tx, id, &s.plan, &configuration.endpoint_digest())?;
        let new = state::persist_complete_advertisement(
            tx,
            id,
            &s.plan,
            state::generation(tx, id)?,
            &[],
            999,
        )?;
        assert_eq!(new, old, "exercise exact SQLite ID reuse");
        Ok(())
    })
    .unwrap();
    let before = registry_image(&db);
    recover(&s);
    replay(&s.f.service, &s);
    assert_eq!(before, registry_image(&db));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_source_handoff_audit_rejects_orphan_freeze_and_wrong_bound_release_reason() {
    for case in ["orphan", "source_reason", "consumer_reason"] {
        let s = prepared(false);
        run(&s).unwrap();
        let db = s.f.db();
        if case == "orphan" {
            db.execute("INSERT INTO poll_retirement_freezes SELECT ?1,owner_epoch,provenance_id+100,binding_id+100,?2,candidate_count,image,original_report,digest,eligibility,final_code FROM poll_retirement_freezes",params![OperationId::new().to_string(),OperationId::new().to_string()]).unwrap();
        } else {
            let ddl: String = db
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE name='retirement_release_immutable'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            db.execute_batch("DROP TRIGGER retirement_release_immutable")
                .unwrap();
            db.execute(
                "UPDATE retirement_observation_pin_releases SET reason=?1 WHERE pin_kind=?2",
                params![
                    if case == "source_reason" {
                        "source_terminal"
                    } else {
                        "consumer_fenced"
                    },
                    if case == "source_reason" {
                        "source_attempt"
                    } else {
                        "consumer_attempt"
                    }
                ],
            )
            .unwrap();
            db.execute_batch(&ddl).unwrap();
        }
        let before = registry_image(&db);
        assert!(record::audit_connection(&db).is_err(), "{case}");
        assert_eq!(before, registry_image(&db));
        assert!(
            RepositoryService::open_at(s.f.data.path()).is_err(),
            "{case}"
        );
    }
}

#[test]
fn retirement_source_handoff_capture_nonce_and_current_mapping_cannot_be_reconstructed() {
    for case in ["nonce", "mapping", "batch_attempt", "request_attempt"] {
        let s = prepared(false);
        let sql = match case {
            "nonce" => format!(
                "DROP TRIGGER inert_frozen_identity; UPDATE poll_retirement_freezes SET capture_nonce='{}'; CREATE TRIGGER inert_frozen_identity BEFORE UPDATE OF operation_ulid,owner_epoch,provenance_id,binding_id,capture_nonce,candidate_count,image,original_report,digest ON poll_retirement_freezes BEGIN SELECT RAISE(ABORT,'immutable frozen image'); END;",
                OperationId::new()
            ),
            "mapping" => "UPDATE request_operations SET family='local'".into(),
            "batch_attempt" => {
                "UPDATE poll_retirement_batches SET owner_epoch=owner_epoch+1".into()
            }
            "request_attempt" => "UPDATE request_records SET attempt=attempt+1".into(),
            _ => unreachable!(),
        };
        after_completion(&s, sql);
        assert!(run(&s).is_err(), "{case}");
        let db = s.f.db();
        assert_eq!(counts(&db), (1, 2, 0));
        assert_eq!(decision(&db), ("unconfirmed".into(), None));
        let public = terminal(&db);
        // Restore only deliberately corrupt fixture mapping, never a production
        // repair. Nonce/epoch changes cannot create a fresh process token.
        if case == "mapping" {
            db.execute("UPDATE request_operations SET family='remote'", [])
                .unwrap();
        }
        recover(&s);
        assert_eq!(counts(&db), (1, 0, 2));
        assert_eq!(public, terminal(&db));
        assert_eq!(
            decision(&db),
            ("ineligible".into(), Some("discovery_pending".into()))
        );
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_source_handoff_empty_full_pass_freezes_before_finish_and_settles_original() {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    input(&s, "[]", 0);
    s.seal().unwrap();
    let result = run(&s).expect("A2/A3 successful original full pass");
    assert!(
        matches!(result,RemotePollResult::Complete(RemotePollOutcome::Polled(ref p)) if p.completion()==PollCompletion::Completed)
    );
    let db = s.f.db();
    let row: (String, String, i64, String) = db
        .query_row(
            "SELECT eligibility,image,candidate_count,final_code FROM poll_retirement_freezes",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(row, ("eligible".into(), "[]".into(), 0, "ok".into()));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM retirement_observation_receipts WHERE consumer_kind='batch'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0,
        "explicit empty list settles both holders without children"
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM retirement_observation_pin_releases",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    record::audit_connection(&db).unwrap();
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}
