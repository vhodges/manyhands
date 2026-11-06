//! Synthetic advertisements and inert preselected target/cursor projections.
//! Actual original A3, guarded branch deletion, discovery and index finalization;
//! NOT live SSH advertisement or Task 4 producer/coordinator evidence.
use super::*;
#[path = "batch_present_tests.rs"]
mod present;
#[path = "batch_positive_primary_tests.rs"]
mod primary;
#[path = "batch_positive_specfix_tests.rs"]
mod specfix;

const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

#[test]
fn retirement_batch_positive_live_index_owner_cannot_write_an_identical_copied_registry() {
    let s = branches();
    let live = run_original(&s);
    let saved = std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot = std::sync::Arc::clone(&saved);
    batch::observe_claim(move |owner| {
        *slot.lock().unwrap() = Some(owner);
    });
    s.f.service
        .set_refresh_claim_hook_for_testing(|| std::panic::resume_unwind(Box::new(())));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)))
        .is_err()
    );
    let owner = saved.lock().unwrap().take().unwrap();
    let db = s.f.db();
    assert!(crate::repository::recovery::owns_indexing(&db, owner.clone()).unwrap());
    let copied = tempfile::tempdir().unwrap();
    let path = copied.path().join("copied-registry.db");
    db.execute("VACUUM INTO ?1", [path.to_str()]).unwrap();
    let other = Connection::open(path).unwrap();
    let before = registry_image(&other);
    assert!(
        crate::repository::recovery::touch_indexing(&other, owner).is_err(),
        "current process index authority must not travel to an identical copied registry"
    );
    assert_eq!(before, registry_image(&other));
}

fn branches() -> SourceFixture {
    branches_with_setup(|_| {})
}
fn branches_with_setup(setup: impl FnOnce(&Fixture)) -> SourceFixture {
    let mut s = SourceFixture::with_setup(setup);
    a2_projections(&s.f.db());
    let first = candidate_id(&s, false, ITEM);
    let mut second = candidate_id(&s, false, SECOND);
    second["ordinal"] = 1.into();
    let entries = vec![first, second];
    input(&s, &serde_json::to_string(&entries).unwrap(), 2);
    projections(&s, &entries);
    s.seal().unwrap();
    s
}

#[test]
fn retirement_batch_positive_old_b_index_authority_cannot_touch_after_completion() {
    let s = branches();
    let live = run_original(&s);
    let saved = std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot = std::sync::Arc::clone(&saved);
    batch::observe_claim(move |owner| {
        *slot.lock().unwrap() = Some(owner);
    });
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .execute_branch_only(&s.f.service)
        .unwrap();
    let old = saved.lock().unwrap().take().unwrap();
    let db = s.f.db();
    let before = registry_image(&db);
    use crate::repository::recovery;
    assert!(recovery::owns_indexing(&db, old.clone()).is_err());
    assert!(recovery::touch_indexing(&db, old.clone()).is_err());
    assert!(recovery::transition_indexing(&db, old.clone(), "failed", None).is_err());
    assert!(recovery::record_owned_persisted_context(&db, old.clone(), s.f.root.path()).is_err());
    assert!(
        recovery::reclaim_invalidated_indexing(&db, old.clone(), s.f.root.path().to_str().unwrap())
            .is_err()
    );
    assert!(
        state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
            record::finalize_index(tx, old.clone(), id, s.f.root.path())
        })
        .is_err()
    );
    assert_eq!(before, registry_image(&db));
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .execute_branch_only(&s.f.service)
        .unwrap();
    assert_eq!(counts(&db), (1, 0, 2));
}

#[test]
fn retirement_batch_positive_late_materialization_replacement_and_proof_change_preserve_b() {
    for replace_m in [true, false] {
        let s = branches();
        let live = run_original(&s);
        let root = s.f.root.path().to_owned();
        let op = s.operation;
        batch::observe(batch::Boundary::IntentCommitted, move |db| {
            let git = git2::Repository::open(&root).unwrap();
            if replace_m {
                let tip = git.head().unwrap().target().unwrap();
                db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,op.to_string(),tip.to_string()]).unwrap();
            } else {
                std::fs::write(
                    root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
                    b"external dirty primary bytes",
                )
                .unwrap();
            }
        });
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        if replace_m {
            assert_eq!(
                result,
                ContextRetirementOutcome::CleanupPending {
                    worktree_removal: RetirementRemoval::None,
                    branch_removal: RetirementRemoval::None
                }
            );
            assert_owed(&s, 0, false);
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        } else {
            assert!(
                matches!(result,ContextRetirementOutcome::Final(ref report) if report.code==crate::results::ResultCode::ExternalChange)
            );
            assert_eq!(
                s.f.db()
                    .query_row(
                        "SELECT next_ordinal FROM poll_retirement_cursors",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                1
            );
            assert!(live.next(&s.f.service, s.f.root.path()).is_ok());
        }
        assert_eq!(
            s.f.db()
                .query_row(
                    "SELECT count(*) FROM remote_poll_materializations",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            i64::from(replace_m)
        );
    }
}

#[test]
fn retirement_batch_positive_selected_key_source_fences_cleanup_not_authorized_index() {
    for locked in [true, false] {
        let s = branches_with_setup(|f| {
            let path = f.data.path().join("positive-synthetic-key-source");
            std::fs::write(&path, b"inert metadata source").unwrap();
            let crate::repository::keys::RegisterSharedKeyOutcome::Registered(key) = f
                .service
                .register_shared_key(crate::repository::keys::RegisterSharedKeyRequest {
                    label: "positive fixture".into(),
                    ownership: crate::repository::keys::SharedKeyOwnership::Imported,
                    private_key_path: path,
                    public_key_path: None,
                })
                .unwrap()
            else {
                panic!("registration");
            };
            f.service.select_shared_key(key.id).unwrap();
        });
        let live = run_original(&s);
        let path = s.f.data.path().join("positive-synthetic-key-source");
        let change = move || {
            std::fs::write(
                path,
                b"external replacement of the original process key metadata source",
            )
            .unwrap();
        };
        if locked {
            batch::observe(batch::Boundary::LockedBranch, move |_| change());
        } else {
            s.f.service.set_refresh_persist_hook_for_testing(change);
        }
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service);
        if locked {
            assert!(!matches!(result, Ok(ContextRetirementOutcome::Final(_))));
            assert_owed(&s, 0, false);
        } else {
            assert!(matches!(result, Ok(ContextRetirementOutcome::Final(_))));
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        }
    }
}

#[test]
fn retirement_batch_positive_missing_companion_claim_or_map_never_reconstructs_index_evidence() {
    for sql in [
        "DELETE FROM context_retirement_records",
        "DELETE FROM request_operations",
        "DELETE FROM context_retirement_attempts",
    ] {
        let s = branches();
        let live = run_original(&s);
        s.f.service.set_refresh_persist_hook_for_testing({
            let data = s.f.data.path().to_owned();
            move || {
                let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
                let ddl: Option<String> = if sql == "DELETE FROM context_retirement_records" {
                    Some(
                        db.query_row(
                            "SELECT sql FROM sqlite_master WHERE name='retirement_record_delete'",
                            [],
                            |r| r.get(0),
                        )
                        .unwrap(),
                    )
                } else {
                    None
                };
                if ddl.is_some() {
                    db.execute_batch("DROP TRIGGER retirement_record_delete")
                        .unwrap();
                }
                db.execute_batch(sql).unwrap();
                if let Some(ddl) = ddl {
                    db.execute_batch(&ddl).unwrap();
                }
            }
        });
        assert_eq!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap(),
            ContextRetirementOutcome::IndexPending
        );
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
    }
    // Claim loss is explicitly corruption; canonical immutability is restored
    // before invoking an actual persist boundary, not a normal producer path.
    let s = branches();
    let live = run_original(&s);
    let data = s.f.data.path().to_owned();
    s.f.service.set_refresh_persist_hook_for_testing(move||{
        let db=Connection::open(data.join(REGISTRY_FILE)).unwrap();
        let ddl:String=db.query_row("SELECT sql FROM sqlite_master WHERE name='retirement_index_claim_delete'",[],|r|r.get(0)).unwrap();
        db.execute_batch("DROP TRIGGER retirement_index_claim_delete;DELETE FROM context_retirement_index_claims;").unwrap();db.execute_batch(&ddl).unwrap();
    });
    assert_eq!(
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap(),
        ContextRetirementOutcome::IndexPending
    );
    let db = s.f.db();
    let before = registry_image(&db);
    assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
    assert_eq!(before, registry_image(&db));
    assert!(RepositoryService::open_at(s.f.data.path()).is_err());
}

#[test]
fn retirement_batch_positive_missing_present_nonce_stays_closed_without_any_new_rows_or_effects() {
    let s = prepared_batch();
    let live = run_original(&s);
    std::fs::remove_file(s.f.root.path().join(format!(
        ".git/worktrees/{ITEM}/manyhands-retirement-instance"
    )))
    .unwrap();
    let before = registry_image(&s.f.db());
    let disk = disk_image(s.f.root.path());
    assert!(
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .is_err()
    );
    assert_eq!(before, registry_image(&s.f.db()));
    assert_eq!(disk, disk_image(s.f.root.path()));
}

#[test]
fn retirement_batch_positive_real_d14_complete_m_and_final_replay_preserve_new_lifetime() {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let mut k = candidate_id(&s, false, ITEM);
    let (owner, oid) = s.f.materialization("complete");
    k["materialization_capture"] = "complete".into();
    k["materialization_operation_id"] = owner.into();
    k["materialization_oid"] = oid.into();
    k["materialization_step"] = "complete".into();
    input(&s, &serde_json::to_string(&vec![k.clone()]).unwrap(), 1);
    projections(&s, &[k]);
    s.seal().unwrap();
    let live = run_original(&s);
    let ContextRetirementOutcome::Final(report) = live
        .next(&s.f.service, s.f.root.path())
        .unwrap()
        .execute_branch_only(&s.f.service)
        .unwrap()
    else {
        panic!("real cleanup/index Final");
    };
    let db = s.f.db();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_poll_materializations",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(counts(&db), (1, 0, 2));
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let tip = git.head().unwrap().target().unwrap();
    git.reference(
        &format!("refs/heads/manyhands/ticket/{ITEM}"),
        tip,
        false,
        "new tokenless lifetime after Final",
    )
    .unwrap();
    db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,s.operation.to_string(),tip.to_string()]).unwrap();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert_eq!(
        batch::read_final(&s.f.service, s.f.root.path(), s.operation, 0).unwrap(),
        Some(report)
    );
    assert!(live.next(&s.f.service, s.f.root.path()).is_err());
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
}

#[test]
fn retirement_batch_positive_ticket_then_document_branch_cleanup_uses_same_original_receipt() {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let first = candidate_id(&s, false, ITEM);
    let root = s.f.root.path().to_owned();
    let git = git2::Repository::open(&root).unwrap();
    let path = root.join("docs/promoted.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {SECOND}\ntitle: Promoted\n---\nBody\n")).unwrap();
    let mut index = git.index().unwrap();
    index.add_path(Path::new("docs/promoted.md")).unwrap();
    index.write().unwrap();
    let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = git.head().unwrap().peel_to_commit().unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let tip = git
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "promoted document",
            &tree,
            &[&parent],
        )
        .unwrap();
    git.reference(
        &format!("refs/heads/manyhands/document/{SECOND}"),
        tip,
        false,
        "synthetic selected document branch",
    )
    .unwrap();
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
    state::with_transaction(&s.f.service, &root, |tx, id| {
        state::persist_complete_advertisement(tx, id, &s.plan, state::generation(tx, id)?, &old, 42)
    })
    .unwrap();
    let second = serde_json::json!({"ordinal":1,"kind":"document","item_id":SECOND,"initial_resources":"branch_only","expected_tip":tip.to_string(),"selection_epoch":1,"instance_nonce":null,"instance_verified":false,"child_operation_id":OperationId::new().to_string(),"materialization_capture":"absent","materialization_operation_id":null,"materialization_oid":null,"materialization_step":null});
    let entries = vec![first, second];
    input(&s, &serde_json::to_string(&entries).unwrap(), 2);
    projections(&s, &entries);
    s.seal().unwrap();
    let live = run_original(&s);
    let db = s.f.db();
    let receipt = row_image(&db, "retirement_observation_receipts");
    let before = std::fs::read(path).unwrap();
    for (kind, item) in [("ticket", ITEM), ("document", SECOND)] {
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        assert!(matches!(result, ContextRetirementOutcome::Final(_)));
        assert!(
            git.find_reference(&format!("refs/heads/manyhands/{kind}/{item}"))
                .is_err()
        );
    }
    assert_eq!(receipt, row_image(&db, "retirement_observation_receipts"));
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(
        before,
        std::fs::read(root.join("docs/promoted.md")).unwrap()
    );
    record::audit_connection(&db).unwrap();
}

#[test]
fn retirement_batch_positive_before_begin_stale_and_unknown_scope_has_no_writes() {
    for case in [
        "request_attempt",
        "cancel",
        "mapping",
        "cursor",
        "current",
        "generation",
        "target",
        "local",
        "comment",
        "tip",
        "ref_lock",
        "configuration",
        "retained_conflict",
    ] {
        let s = branches();
        let live = run_original(&s);
        let held = live.next(&s.f.service, s.f.root.path()).unwrap();
        let db = s.f.db();
        match case {
            "request_attempt" => {
                s.f.service.enter_request(s.request_id).unwrap().unwrap();
            }
            "cancel" => {
                db.execute("UPDATE request_records SET cancel_requested=1", [])
                    .unwrap();
            }
            "mapping" => {
                db.execute("UPDATE request_operations SET family='local'", [])
                    .unwrap();
            }
            "cursor" => {
                db.execute("UPDATE poll_retirement_cursors SET next_ordinal=1", [])
                    .unwrap();
            }
            "current" => {
                db.execute("UPDATE remote_observation_batches SET is_current=0", [])
                    .unwrap();
            }
            "generation" => {
                db.execute("UPDATE remote_polling_state SET configuration_generation=configuration_generation+1",[]).unwrap();
            }
            "target" => {
                db.execute("DELETE FROM poll_retirement_targets WHERE ordinal=1", [])
                    .unwrap();
            }
            "local" | "comment" => {
                db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,completed_step,observed_at) SELECT id,root_path,?1,'refresh',?2,?3,1 FROM repositories",params![OperationId::new().to_string(),if case=="local"{"authoritative_observed"}else{"completed"},(case=="comment").then_some("comment_registration_pending")]).unwrap();
            }
            "tip" => {
                let git = git2::Repository::open(s.f.root.path()).unwrap();
                let different = git.head().unwrap().target().unwrap();
                git.reference(
                    &format!("refs/heads/manyhands/ticket/{ITEM}"),
                    different,
                    true,
                    "changed frozen tip",
                )
                .unwrap();
            }
            "ref_lock" => {
                std::fs::write(
                    s.f.root.path().join(".git/packed-refs.lock"),
                    b"foreign ref lock",
                )
                .unwrap();
            }
            "configuration" => {
                git2::Repository::open(s.f.root.path())
                    .unwrap()
                    .config()
                    .unwrap()
                    .set_str("remote.origin.pushurl", "ssh://other.invalid/project")
                    .unwrap();
            }
            "retained_conflict" => {
                let RemoteReservationOutcome::Reserved(owner) =
                    s.f.service
                        .reserve_remote_operation(
                            s.f.root.path(),
                            OperationId::new(),
                            &RemoteOperationTarget::for_primary_synchronization(&s.plan),
                        )
                        .unwrap()
                else {
                    panic!("reservation");
                };
                s.f.service
                    .retirement_test_conflict(s.f.root.path(), &owner, 1)
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(held.execute_branch_only(&s.f.service).is_err(), "{case}");
        assert_eq!(before, registry_image(&db), "{case}");
        assert_eq!(disk, disk_image(s.f.root.path()), "{case}");
    }
}

#[test]
fn retirement_batch_positive_original_branch_b_index_then_c_same_receipt() {
    let s = branches();
    let live = run_original(&s);
    let db = s.f.db();
    let p = terminal(&db);
    let ad = row_image(&db, "remote_observation_batches");
    let receipt = row_image(&db, "retirement_observation_receipts");
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let tracking = git.refname_to_id("refs/remotes/origin/main").unwrap();
    let old = live.next(&s.f.service, s.f.root.path()).unwrap();
    for (ordinal, item) in [ITEM, SECOND].into_iter().enumerate() {
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM operation_records WHERE state!='completed'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(counts(&db), (1, 1, 1), "shared hold outlives B");
        assert_eq!(
            db.query_row(
                "SELECT next_ordinal FROM poll_retirement_cursors",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            ordinal as i64
        );
        if ordinal == 1 {
            let row: (String,String,i64,bool) = db.query_row("SELECT o.state,c.phase,o.persisted_context_count,c.final_result IS NOT NULL FROM operation_records o JOIN context_retirement_records c ON c.local_record_id=o.id WHERE c.item_id=?1", [ITEM], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
            assert_eq!(
                (row.0.as_str(), row.1.as_str(), row.3),
                ("completed", "complete", true)
            );
            assert!(
                row.2 > 0,
                "ordinary discovery actually persisted the primary context"
            );
        }
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .expect("original current branch child must execute real cleanup and local index");
        let ContextRetirementOutcome::Final(report) = result else {
            panic!("index and result must complete: {result:?}");
        };
        assert_eq!(report.code, crate::results::ResultCode::Ok);
        assert_eq!(report.branch_removal, RetirementRemoval::Observed);
        assert_eq!(report.worktree_removal, RetirementRemoval::None);
        assert!(report.materialization_settled);
        assert!(
            git.find_reference(&format!("refs/heads/manyhands/ticket/{item}"))
                .is_err()
        );
        assert_eq!(terminal(&db), p, "ended P stays byte-for-byte ended");
        assert_eq!(receipt, row_image(&db, "retirement_observation_receipts"));
        assert_eq!(ad, row_image(&db, "remote_observation_batches"));
        assert_eq!(
            tracking,
            git.refname_to_id("refs/remotes/origin/main").unwrap()
        );
    }
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(decision(&db), ("eligible".into(), Some("ok".into())));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_index_claims",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    let before = registry_image(&db);
    assert!(old.execute_branch_only(&s.f.service).is_err());
    assert!(live.next(&s.f.service, s.f.root.path()).is_err());
    assert_eq!(before, registry_image(&db));
    record::audit_connection(&db).unwrap();
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

fn assert_owed(s: &SourceFixture, cursor: i64, deleted: bool) {
    let db = s.f.db();
    assert_eq!(
        db.query_row(
            "SELECT next_ordinal FROM poll_retirement_cursors",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        cursor
    );
    assert_eq!(decision(&db), ("eligible".into(), None));
    assert_eq!(counts(&db), (1, 1, 1));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_records WHERE phase='complete'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        cursor
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE state!='completed'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT state FROM poll_retirement_targets WHERE ordinal=?1",
            [cursor],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "running"
    );
    let item = if cursor == 0 { ITEM } else { SECOND };
    assert_eq!(
        git2::Repository::open(s.f.root.path())
            .unwrap()
            .find_reference(&format!("refs/heads/manyhands/ticket/{item}"))
            .is_err(),
        deleted
    );
    record::audit_connection(&db).unwrap();
}

#[test]
fn retirement_batch_positive_index_failure_keeps_b_owed_and_same_f2_spent() {
    let s = branches();
    let live = run_original(&s);
    let held = live.next(&s.f.service, s.f.root.path()).unwrap();
    *s.f.service.failure_point.lock().unwrap() = Some(FailurePoint::BeforeIndexTransactionCommit);
    assert_eq!(
        held.execute_branch_only(&s.f.service).unwrap(),
        ContextRetirementOutcome::IndexPending
    );
    assert_owed(&s, 0, true);
    assert!(live.next(&s.f.service, s.f.root.path()).is_err());
    let db = s.f.db();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert_eq!(
        live.continue_index(&s.f.service, s.f.root.path()).unwrap(),
        ContextRetirementOutcome::IndexPending
    );
    let child: String = db
        .query_row(
            "SELECT operation_ulid FROM context_retirement_records",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let local: i64 = db
        .query_row(
            "SELECT local_record_id FROM context_retirement_records",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(crate::repository::recovery::claim_indexing(&db, local, i64::MAX).is_err());
    assert!(
        s.f.service
            .refresh_repository(RefreshRepositoryRequest {
                root: s.f.root.path().into(),
                operation_id: OperationId::parse(&child).unwrap()
            })
            .is_err()
    );
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_batch_positive_index_only_unspent_continuation_never_reconsumes_or_starts_c() {
    let s = branches();
    let live = run_original(&s);
    batch::observe(batch::Boundary::BeforeClaim, |db| {
        db.execute_batch("CREATE TRIGGER fail_positive_claim BEFORE UPDATE OF state ON operation_records WHEN NEW.state='indexing' BEGIN SELECT RAISE(ABORT,'claim fault'); END;").unwrap();
    });
    assert_eq!(
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap(),
        ContextRetirementOutcome::IndexPending,
        "a failed unspent claim after real cleanup must report index debt, not erase partial truth in a generic error"
    );
    assert_owed(&s, 0, true);
    let db = s.f.db();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_index_claims",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let tip = git.head().unwrap().target().unwrap();
    git.reference(
        &format!("refs/heads/manyhands/ticket/{ITEM}"),
        tip,
        false,
        "new lifetime after accepted D14",
    )
    .unwrap();
    db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,s.operation.to_string(),tip.to_string()]).unwrap();
    let m = row_image(&db, "remote_poll_materializations");
    let intents = row_image(&db, "context_retirement_effect_intents");
    let ads = row_image(&db, "retirement_advertisement_provenance");
    let report = live.continue_index(&s.f.service, s.f.root.path()).unwrap();
    assert!(matches!(report, ContextRetirementOutcome::Final(_)));
    assert_eq!(m, row_image(&db, "remote_poll_materializations"));
    assert_eq!(intents, row_image(&db, "context_retirement_effect_intents"));
    assert_eq!(ads, row_image(&db, "retirement_advertisement_provenance"));
    assert_eq!(
        git.refname_to_id(&format!("refs/heads/manyhands/ticket/{ITEM}"))
            .unwrap(),
        tip
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(counts(&db), (1, 1, 1));
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert!(
        live.continue_index(&s.f.service, s.f.root.path()).is_err(),
        "continuation must not begin next target"
    );
    let ContextRetirementOutcome::Final(expected) = report else {
        unreachable!()
    };
    assert_eq!(
        batch::read_final(&s.f.service, s.f.root.path(), s.operation, 0).unwrap(),
        Some(expected)
    );
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
}

#[test]
fn retirement_batch_positive_actual_finalizer_faults_rollback_result_cursor_staleclear_and_z() {
    for (last, table, column, condition) in [
        (
            false,
            "context_retirement_records",
            "phase",
            "NEW.phase='complete'",
        ),
        (
            false,
            "operation_records",
            "state",
            "NEW.state='completed' AND OLD.state='indexing'",
        ),
        (
            false,
            "poll_retirement_targets",
            "handoff",
            "NEW.handoff='final'",
        ),
        (
            false,
            "poll_retirement_cursors",
            "next_ordinal",
            "NEW.next_ordinal>OLD.next_ordinal",
        ),
        (
            true,
            "poll_retirement_freezes",
            "final_code",
            "NEW.final_code='ok'",
        ),
        (
            true,
            "poll_retirement_attempts",
            "phase",
            "NEW.phase='ended'",
        ),
        (
            true,
            "repositories",
            "refresh_required",
            "NEW.refresh_required=0",
        ),
        (true, "retirement_observation_pin_releases", "", ""),
    ] {
        let s = branches();
        let live = run_original(&s);
        if last {
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap();
        }
        let db = s.f.db();
        if column.is_empty() {
            record::set_release_failure(true);
        } else {
            db.execute_batch(&format!("CREATE TRIGGER fail_positive_settlement BEFORE UPDATE OF {column} ON {table} WHEN {condition} BEGIN SELECT RAISE(ABORT,'finalizer fault'); END;")).unwrap();
        }
        assert_eq!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap(),
            ContextRetirementOutcome::IndexPending,
            "{table}"
        );
        assert_owed(&s, i64::from(last), true);
        assert_eq!(
            db.query_row("SELECT refresh_required FROM repositories", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
    }
}

#[test]
fn retirement_batch_positive_d14_exact_m_atomic_rollback_preserves_partial_truth() {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let mut k = candidate_id(&s, false, ITEM);
    let (owner, oid) = s.f.materialization("complete");
    k["materialization_capture"] = "complete".into();
    k["materialization_operation_id"] = owner.into();
    k["materialization_oid"] = oid.into();
    k["materialization_step"] = "complete".into();
    input(&s, &serde_json::to_string(&vec![k.clone()]).unwrap(), 1);
    projections(&s, &[k]);
    s.seal().unwrap();
    let live = run_original(&s);
    let db = s.f.db();
    let m = row_image(&db, "remote_poll_materializations");
    db.execute_batch("CREATE TRIGGER fail_positive_d14 BEFORE UPDATE OF completed_step ON operation_records WHEN NEW.completed_step='context_branch_released' BEGIN SELECT RAISE(ABORT,'D14 step fault'); END;").unwrap();
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
    assert_owed(&s, 0, true);
    assert_eq!(m, row_image(&db, "remote_poll_materializations"));
    assert_eq!(
        db.query_row(
            "SELECT materialization_settled FROM context_retirement_records",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(
        db.query_row(
            "SELECT observation IS NULL FROM context_retirement_effect_intents",
            [],
            |r| r.get::<_, bool>(0)
        )
        .unwrap()
    );
    assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_batch_positive_each_actual_write_boundary_rejoins_full_current_scope() {
    // Deliberate in-transaction stale/corrupt projections at actual boundaries,
    // not supported request producers. SQL mutation rolls back with refusal.
    for point in [
        batch::Boundary::Begin,
        batch::Boundary::Intent,
        batch::Boundary::LockedBranch,
        batch::Boundary::Deleted,
        batch::Boundary::BeforeClaim,
        batch::Boundary::Finalizer,
    ] {
        for sql in [
            "UPDATE request_records SET cancel_requested=1",
            "UPDATE request_records SET attempt=attempt+1",
            "UPDATE request_operations SET family='local'",
            "UPDATE poll_retirement_batches SET owner_epoch=owner_epoch+1",
            "UPDATE poll_retirement_cursors SET next_ordinal=1",
            "UPDATE remote_polling_state SET configuration_generation=configuration_generation+1,history_unknown=1; UPDATE remote_observation_batches SET is_current=0; DELETE FROM remote_context_states;",
            "UPDATE remote_observation_batches SET is_current=0",
            "UPDATE remote_operation_records SET owner_epoch=owner_epoch+1 WHERE poll_kind='update'",
        ] {
            let s = branches();
            let live = run_original(&s);
            let p = terminal(&s.f.db());
            batch::observe(point, move |db| {
                db.execute_batch(sql).unwrap();
            });
            let result = live
                .next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service);
            let index_stage = matches!(
                point,
                batch::Boundary::BeforeClaim | batch::Boundary::Finalizer
            );
            // The G case is an explicitly compatible in-transaction projection
            // including invalidation, not a bare generation edit that leaves a
            // current batch at the wrong G. Real configure runs in specfix tests.
            let deletion_only = sql.contains("cancel_requested")
                || sql.contains("UPDATE remote_observation_batches");
            if index_stage && deletion_only {
                assert!(
                    matches!(result, Ok(ContextRetirementOutcome::Final(_))),
                    "{point:?}: {sql}: {result:?}"
                );
                assert_eq!(
                    s.f.db()
                        .query_row(
                            "SELECT next_ordinal FROM poll_retirement_cursors",
                            [],
                            |r| r.get::<_, i64>(0)
                        )
                        .unwrap(),
                    1
                );
                assert_eq!(counts(&s.f.db()), (1, 1, 1));
                assert!(live.next(&s.f.service, s.f.root.path()).is_err());
            } else {
                assert!(
                    !matches!(result, Ok(ContextRetirementOutcome::Final(_))),
                    "{point:?}: {sql}"
                );
                assert_owed(
                    &s,
                    0,
                    matches!(
                        point,
                        batch::Boundary::Deleted
                            | batch::Boundary::BeforeClaim
                            | batch::Boundary::Finalizer
                    ),
                );
            }
            assert_eq!(terminal(&s.f.db()), p);
        }
    }
}

#[test]
fn retirement_batch_positive_config_fences_locked_b_not_authorized_index() {
    for locked in [true, false] {
        let s = branches();
        let live = run_original(&s);
        let root = s.f.root.path().to_owned();
        if locked {
            batch::observe(batch::Boundary::LockedBranch, move |_| {
                let git = git2::Repository::open(root).unwrap();
                git.config()
                    .unwrap()
                    .set_str("remote.origin.pushurl", "ssh://other.invalid/project")
                    .unwrap();
            });
        } else {
            s.f.service.set_refresh_persist_hook_for_testing(move || {
                let git = git2::Repository::open(root).unwrap();
                git.config()
                    .unwrap()
                    .set_str("remote.origin.pushurl", "ssh://other.invalid/project")
                    .unwrap();
            });
        }
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service);
        if locked {
            assert!(!matches!(result, Ok(ContextRetirementOutcome::Final(_))));
            assert_owed(&s, 0, false);
        } else {
            assert!(matches!(result, Ok(ContextRetirementOutcome::Final(_))));
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        }
    }
}

#[test]
fn retirement_batch_positive_surviving_tokenless_b_cannot_be_adopted_by_restart_or_fresh_executor()
{
    let s = branches();
    let live = run_original(&s);
    let other = RepositoryService::open_at(s.f.data.path()).unwrap();
    let held = live.next(&s.f.service, s.f.root.path()).unwrap();
    let db = s.f.db();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert!(held.execute_branch_only(&other).is_err());
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
    drop(live);
    let image: String = db
        .query_row("SELECT image FROM poll_retirement_freezes", [], |r| {
            r.get(0)
        })
        .unwrap();
    let entries: serde_json::Value = serde_json::from_str(&image).unwrap();
    let child = OperationId::parse(entries[0]["child_operation_id"].as_str().unwrap()).unwrap();
    assert!(
        other
            .retire_context(RetireContextRequest {
                root: s.f.root.path().into(),
                kind: AuthoringKind::Ticket,
                item_id: ITEM.parse().unwrap(),
                operation_id: child
            })
            .is_err()
    );
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
}
