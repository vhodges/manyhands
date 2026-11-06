//! Remaining S2 invariant: successfully proved but different intent pair.
//! Real primary commits/cleanup/index, synthetic ads and inert batch projections.
use super::*;
use crate::repository::context_retirement::{
    proof::PreservationProof,
    run::{self, LocalProofComparison},
};

fn materialized_branches() -> SourceFixture {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let mut first = candidate_id(&s, false, ITEM);
    let mut second = candidate_id(&s, false, SECOND);
    second["ordinal"] = 1.into();
    let (owner, oid) = s.f.materialization("complete");
    first["materialization_capture"] = "complete".into();
    first["materialization_operation_id"] = owner.into();
    first["materialization_oid"] = oid.into();
    first["materialization_step"] = "complete".into();
    let entries = vec![first, second];
    input(&s, &serde_json::to_string(&entries).unwrap(), 2);
    projections(&s, &entries);
    s.seal().unwrap();
    s
}

// Independently establish successful Preserved(new, Ancestor), not a generic
// inspection error or dirty-file comparison, while retaining the original pair.
fn advance_and_prove(root: &Path, db: &Connection, item: &str) -> (String, String, String, i64) {
    let git = git2::Repository::open(root).unwrap();
    let old = git.head().unwrap().peel_to_commit().unwrap();
    let tree = old.tree().unwrap();
    let sig = git2::Signature::now("Primary invariant fixture", "fixture@example.invalid").unwrap();
    let new = git
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "same-tree primary advancement",
            &tree,
            &[&old],
        )
        .unwrap();
    assert_ne!(old.id(), new);
    assert_eq!(tree.id(), git.find_commit(new).unwrap().tree_id());
    let (operation,tip,primary,kind,receipt):(String,String,String,String,i64)=db.query_row("SELECT c.operation_ulid,c.expected_tip,e.primary_oid,e.proof_kind,e.receipt_id FROM context_retirement_records c JOIN context_retirement_effect_intents e ON e.operation_ulid=c.operation_ulid WHERE c.item_id=?1 AND e.resource='branch'",[item],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
    assert_eq!(primary, old.id().to_string());
    let request = RetireContextRequest {
        root: root.into(),
        kind: AuthoringKind::Ticket,
        item_id: item.parse().unwrap(),
        operation_id: OperationId::parse(&operation).unwrap(),
    };
    match run::compare_local_proof(&git, &request, git2::Oid::from_str(&tip).unwrap(), "main")
        .unwrap()
    {
        LocalProofComparison::Preserved(now, proof) => {
            assert_eq!(now, new);
            assert_eq!(proof, PreservationProof::Ancestor);
        }
        LocalProofComparison::Changed => panic!("primary bytes/mode must be unchanged"),
    }
    (primary, new.to_string(), kind, receipt)
}

fn assert_original_intent(
    db: &Connection,
    item: &str,
    observed: &(String, String, String, i64),
    observation: Option<&str>,
) {
    let row:(String,String,i64,Option<String>)=db.query_row("SELECT e.primary_oid,e.proof_kind,e.receipt_id,e.observation FROM context_retirement_effect_intents e JOIN context_retirement_records c ON c.operation_ulid=e.operation_ulid WHERE c.item_id=?1 AND e.resource='branch'",[item],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
    assert_eq!(
        row,
        (
            observed.0.clone(),
            observed.2.clone(),
            observed.3,
            observation.map(str::to_owned)
        )
    );
}

#[test]
fn retirement_batch_primaryfix_successfully_different_pair_is_known_noeffect_at_both_boundaries() {
    let mut failures = Vec::new();
    for point in [
        batch::Boundary::IntentCommitted,
        batch::Boundary::LockedBranch,
    ] {
        for last in [false, true] {
            let s = materialized_branches();
            let live = run_original(&s);
            if last {
                assert!(matches!(
                    live.next(&s.f.service, s.f.root.path())
                        .unwrap()
                        .execute_branch_only(&s.f.service)
                        .unwrap(),
                    ContextRetirementOutcome::Final(_)
                ));
            }
            let item = if last { SECOND } else { ITEM };
            let held = live.next(&s.f.service, s.f.root.path()).unwrap();
            let stale = live.next(&s.f.service, s.f.root.path()).unwrap();
            let db = s.f.db();
            let m = row_image(&db, "remote_poll_materializations");
            let ads = row_image(&db, "remote_observation_batches");
            let p = row_image(&db, "remote_operation_records");
            let expected_tip:String=db.query_row("SELECT json_extract(candidate,'$.expected_tip') FROM poll_retirement_targets WHERE ordinal=?1",[i64::from(last)],|r|r.get(0)).unwrap();
            let root = s.f.root.path().to_owned();
            let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
            let capture = std::sync::Arc::clone(&observed);
            batch::observe(point, move |db| {
                *capture.lock().unwrap() = Some(advance_and_prove(&root, db, item));
            });
            let result = held.execute_branch_only(&s.f.service).unwrap();
            let pair = observed.lock().unwrap().take().unwrap();
            let git = git2::Repository::open(s.f.root.path()).unwrap();
            assert_eq!(git.head().unwrap().target().unwrap().to_string(), pair.1);
            assert_eq!(
                git.refname_to_id(&format!("refs/heads/manyhands/ticket/{item}"))
                    .unwrap()
                    .to_string(),
                expected_tip
            );
            assert_eq!(m, row_image(&db, "remote_poll_materializations"));
            assert_eq!(ads, row_image(&db, "remote_observation_batches"));
            assert_eq!(p, row_image(&db, "remote_operation_records"));
            let ContextRetirementOutcome::Final(report) = result else {
                assert_original_intent(&db, item, &pair, None);
                failures.push(format!("{point:?},last={last}: {result:?}"));
                continue;
            };
            assert_eq!(report.code, crate::results::ResultCode::ExternalChange);
            assert_eq!(report.worktree_removal, RetirementRemoval::None);
            assert_eq!(report.branch_removal, RetirementRemoval::None);
            assert!(!report.materialization_settled);
            assert_original_intent(&db, item, &pair, Some("changed"));
            assert_eq!(db.query_row("SELECT count(*) FROM context_retirement_index_claims i JOIN context_retirement_records c USING(operation_ulid) WHERE c.item_id=?1",[item],|r|r.get::<_,i64>(0)).unwrap(),0);
            assert_eq!(
                db.query_row(
                    "SELECT next_ordinal FROM poll_retirement_cursors",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1 + i64::from(last)
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
            assert_eq!(counts(&db), if last { (1, 0, 2) } else { (1, 1, 1) });
            let image = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            assert!(stale.execute_branch_only(&s.f.service).is_err());
            assert!(
                live.continue_no_effect(&s.f.service, s.f.root.path())
                    .is_err()
            );
            assert_eq!(
                batch::read_final(
                    &s.f.service,
                    s.f.root.path(),
                    s.operation,
                    usize::from(last)
                )
                .unwrap(),
                Some(report)
            );
            assert_eq!(image, registry_image(&db));
            assert_eq!(disk, disk_image(s.f.root.path()));
            if !last {
                assert!(matches!(
                    live.next(&s.f.service, s.f.root.path())
                        .unwrap()
                        .execute_branch_only(&s.f.service)
                        .unwrap(),
                    ContextRetirementOutcome::Final(_)
                ));
            }
            assert_eq!(counts(&db), (1, 0, 2));
            assert_eq!(ads, row_image(&db, "remote_observation_batches"));
            record::audit_connection(&db).unwrap();
        }
    }
    assert!(
        failures.is_empty(),
        "successfully different pairs were stranded instead of settled: {failures:?}"
    );
}

#[test]
fn retirement_batch_primaryfix_pair_change_settlement_fault_retains_original_metadata_witness() {
    for point in [
        batch::Boundary::IntentCommitted,
        batch::Boundary::LockedBranch,
    ] {
        for last in [false, true] {
            let s = materialized_branches();
            let live = run_original(&s);
            if last {
                live.next(&s.f.service, s.f.root.path())
                    .unwrap()
                    .execute_branch_only(&s.f.service)
                    .unwrap();
            }
            let item = if last { SECOND } else { ITEM };
            let root = s.f.root.path().to_owned();
            let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
            let capture = std::sync::Arc::clone(&observed);
            let rollback = std::sync::Arc::new(std::sync::Mutex::new(None));
            let snapshot = std::sync::Arc::clone(&rollback);
            batch::observe(point, move |db| {
                *capture.lock().unwrap() = Some(advance_and_prove(&root, db, item));
                batch::observe(batch::Boundary::NoEffect, move |db| {
                    *snapshot.lock().unwrap() = Some((registry_image(db), disk_image(&root)));
                    if last {
                        record::set_release_failure(true);
                    } else {
                        db.execute_batch("CREATE TRIGGER fail_primary_pair_cursor BEFORE UPDATE OF next_ordinal ON poll_retirement_cursors WHEN NEW.next_ordinal>OLD.next_ordinal BEGIN SELECT RAISE(ABORT,'primary pair settlement fault'); END;").unwrap();
                    }
                });
            });
            assert_eq!(
                live.next(&s.f.service, s.f.root.path())
                    .unwrap()
                    .execute_branch_only(&s.f.service)
                    .unwrap(),
                ContextRetirementOutcome::CleanupPending {
                    worktree_removal: RetirementRemoval::None,
                    branch_removal: RetirementRemoval::None
                }
            );
            let db = s.f.db();
            let pair = observed.lock().unwrap().take().unwrap();
            let before = rollback
                .lock()
                .unwrap()
                .take()
                .expect("known pair must reach actual no-effect settlement");
            assert_eq!(before.0, registry_image(&db));
            assert_eq!(before.1, disk_image(s.f.root.path()));
            assert_original_intent(&db, item, &pair, None);
            assert_owed(&s, i64::from(last), false);
            // Retry must be metadata-only even if proof inspection would now
            // fail. Its witness still binds the OLD pair, never the new primary.
            std::fs::remove_file(
                s.f.root
                    .path()
                    .join(format!(".manyhands/tickets/{item}/ticket.md")),
            )
            .unwrap();
            let m = row_image(&db, "remote_poll_materializations");
            let disk = disk_image(s.f.root.path());
            let ads = row_image(&db, "remote_observation_batches");
            let result = live
                .continue_no_effect(&s.f.service, s.f.root.path())
                .unwrap();
            assert!(
                matches!(result,ContextRetirementOutcome::Final(ref report) if report.code==crate::results::ResultCode::ExternalChange && report.branch_removal==RetirementRemoval::None && report.worktree_removal==RetirementRemoval::None)
            );
            assert_original_intent(&db, item, &pair, Some("changed"));
            assert_eq!(m, row_image(&db, "remote_poll_materializations"));
            assert_eq!(disk, disk_image(s.f.root.path()));
            assert_eq!(ads, row_image(&db, "remote_observation_batches"));
            assert_eq!(counts(&db), if last { (1, 0, 2) } else { (1, 1, 1) });
            assert_eq!(db.query_row("SELECT count(*) FROM context_retirement_index_claims c JOIN context_retirement_records r USING(operation_ulid) WHERE r.item_id=?1",[item],|r|r.get::<_,i64>(0)).unwrap(),0);
            assert!(
                live.continue_no_effect(&s.f.service, s.f.root.path())
                    .is_err()
            );
            record::audit_connection(&db).unwrap();
        }
    }
}

#[test]
fn retirement_batch_primaryfix_known_pair_cannot_override_stale_f2_map_or_cursor() {
    for point in [
        batch::Boundary::IntentCommitted,
        batch::Boundary::LockedBranch,
    ] {
        for sql in [
            "UPDATE request_records SET attempt=attempt+1,cancel_requested=1",
            "UPDATE request_operations SET family='local'",
            "UPDATE poll_retirement_cursors SET next_ordinal=1",
        ] {
            let s = materialized_branches();
            let live = run_original(&s);
            let root = s.f.root.path().to_owned();
            let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
            let capture = std::sync::Arc::clone(&observed);
            let rollback = std::sync::Arc::new(std::sync::Mutex::new(None));
            let snapshot = std::sync::Arc::clone(&rollback);
            batch::observe(point, move |db| {
                *capture.lock().unwrap() = Some(advance_and_prove(&root, db, ITEM));
                // Deliberate in-transaction stale/corrupt ownership, not a
                // supported F2/map producer. It must roll back without a Final.
                batch::observe(batch::Boundary::NoEffect, move |db| {
                    *snapshot.lock().unwrap() = Some(registry_image(db));
                    db.execute_batch(sql).unwrap();
                });
            });
            assert_eq!(
                live.next(&s.f.service, s.f.root.path())
                    .unwrap()
                    .execute_branch_only(&s.f.service)
                    .unwrap(),
                ContextRetirementOutcome::CleanupPending {
                    worktree_removal: RetirementRemoval::None,
                    branch_removal: RetirementRemoval::None
                }
            );
            let db = s.f.db();
            let pair = observed.lock().unwrap().take().unwrap();
            assert_eq!(*rollback.lock().unwrap(), Some(registry_image(&db)));
            assert_original_intent(&db, ITEM, &pair, None);
            assert_owed(&s, 0, false);
            // Real durable F2 advancement now also blocks the retained witness;
            // known difference cannot authorize writes or cancel acknowledgment.
            s.f.service.enter_request(s.request_id).unwrap().unwrap();
            db.execute("UPDATE request_records SET cancel_requested=1", [])
                .unwrap();
            let before = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            assert!(!matches!(
                live.continue_no_effect(&s.f.service, s.f.root.path()),
                Ok(ContextRetirementOutcome::Final(_))
            ));
            assert_eq!(before, registry_image(&db));
            assert_eq!(disk, disk_image(s.f.root.path()));
            assert!(
                db.query_row("SELECT cancel_requested FROM request_records", [], |r| r
                    .get::<_, bool>(
                    0
                ))
                .unwrap()
            );
        }
    }
}

#[test]
fn retirement_batch_primaryfix_pair_change_after_actual_git_stays_possible_owed() {
    let s = materialized_branches();
    let live = run_original(&s);
    let root = s.f.root.path().to_owned();
    let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
    let capture = std::sync::Arc::clone(&observed);
    batch::observe(batch::Boundary::Deleted, move |db| {
        *capture.lock().unwrap() = Some(advance_and_prove(&root, db, ITEM));
    });
    let db = s.f.db();
    let m = row_image(&db, "remote_poll_materializations");
    assert_eq!(
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap(),
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::None,
            branch_removal: RetirementRemoval::Possible
        }
    );
    let pair = observed.lock().unwrap().take().unwrap();
    assert_original_intent(&db, ITEM, &pair, None);
    assert_owed(&s, 0, true);
    assert_eq!(m, row_image(&db, "remote_poll_materializations"));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_index_claims",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert!(
        live.continue_no_effect(&s.f.service, s.f.root.path())
            .is_err()
    );
    assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
    assert!(live.next(&s.f.service, s.f.root.path()).is_err());
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}
