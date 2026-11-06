//! Synthetic advertisements/inert pre-enrollment and producer projections.
//! Real original A3, nonce verification, guarded W/B, D14, discovery/index/result.
use super::*;

fn present_batch() -> SourceFixture {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let mut first = candidate_id(&s, true, ITEM);
    let mut second = candidate_id(&s, true, SECOND);
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

#[test]
fn retirement_batch_present_original_nonce_w_b_d14_real_index_then_c_same_receipt() {
    let s = present_batch();
    let live = run_original(&s);
    let db = s.f.db();
    let root = s.f.root.path();
    let git = git2::Repository::open(root).unwrap();
    let ad = row_image(&db, "remote_observation_batches");
    let receipt = row_image(&db, "retirement_observation_receipts");
    let p = row_image(&db, "remote_operation_records");
    let tracking = git.refname_to_id("refs/remotes/origin/main").unwrap();
    let primary = std::fs::read(root.join(format!(".manyhands/tickets/{ITEM}/ticket.md"))).unwrap();
    let sentinel = s.f.data.path().join("inert-draft-sentinel");
    std::fs::write(&sentinel, b"untouched draft projection").unwrap();
    for (ordinal, item) in [ITEM, SECOND].into_iter().enumerate() {
        assert_eq!(counts(&db), (1, 1, 1));
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM operation_records WHERE state!='completed'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        if ordinal == 1 {
            let row:(String,String,i64,String)=db.query_row("SELECT o.state,c.phase,o.persisted_context_count,t.handoff FROM operation_records o JOIN context_retirement_records c ON c.local_record_id=o.id JOIN poll_retirement_targets t ON t.ordinal=c.batch_ordinal WHERE c.item_id=?1",[ITEM],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
            assert_eq!(
                (row.0.as_str(), row.1.as_str(), row.3.as_str()),
                ("completed", "complete", "final")
            );
            assert!(row.2 > 0);
        }
        let result = live
            .next(&s.f.service, root)
            .unwrap()
            .execute_branch_only(&s.f.service)
            .expect("real present child cleanup");
        let ContextRetirementOutcome::Final(report) = result else {
            panic!("ordinary index/result must finish: {result:?}")
        };
        assert_eq!(
            report,
            crate::repository::context_retirement::ContextRetirementReport::retired(
                RetirementRemoval::Observed,
                RetirementRemoval::Observed
            )
        );
        assert!(!root.join(".manyhands/worktrees").join(item).exists());
        assert!(!git.commondir().join("worktrees").join(item).exists());
        assert!(
            git.find_reference(&format!("refs/heads/manyhands/ticket/{item}"))
                .is_err()
        );
        assert_eq!(db.query_row("SELECT count(*) FROM context_retirement_effect_intents e JOIN context_retirement_records c USING(operation_ulid) WHERE c.item_id=?1 AND e.observation='removed' AND e.receipt_id=(SELECT binding_id FROM poll_retirement_freezes)",[item],|r|r.get::<_,i64>(0)).unwrap(),2);
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert!(db.query_row("SELECT EXISTS(SELECT 1 FROM discovered_items i JOIN contexts c ON c.id=i.context_id WHERE i.item_id=?1 AND i.status='closed' AND i.closed_at IS NOT NULL AND c.worktree_path=?2)",params![item,root.to_str()],|r|r.get::<_,bool>(0)).unwrap());
        assert_eq!(
            db.query_row(
                "SELECT next_ordinal FROM poll_retirement_cursors",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            ordinal as i64 + 1
        );
        assert_eq!(p, row_image(&db, "remote_operation_records"));
        assert_eq!(ad, row_image(&db, "remote_observation_batches"));
        assert_eq!(receipt, row_image(&db, "retirement_observation_receipts"));
        assert_eq!(
            tracking,
            git.refname_to_id("refs/remotes/origin/main").unwrap()
        );
    }
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(
        primary,
        std::fs::read(root.join(format!(".manyhands/tickets/{ITEM}/ticket.md"))).unwrap()
    );
    assert_eq!(
        std::fs::read(sentinel).unwrap(),
        b"untouched draft projection"
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_index_claims",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    record::audit_connection(&db).unwrap();
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    assert!(live.next(&s.f.service, root).is_err());
}

fn assert_present_debt(
    s: &SourceFixture,
    w: RetirementRemoval,
    b: RetirementRemoval,
    outcome: ContextRetirementOutcome,
) {
    assert_eq!(
        outcome,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: w,
            branch_removal: b
        }
    );
    let db = s.f.db();
    assert_eq!(counts(&db), (1, 1, 1));
    assert_eq!(decision(&db), ("eligible".into(), None));
    assert_eq!(
        db.query_row(
            "SELECT next_ordinal FROM poll_retirement_cursors",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
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
            "SELECT count(*) FROM context_retirement_records WHERE final_result IS NOT NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn retirement_batch_present_immediate_registration_locks_and_hidden_flags_are_not_authority() {
    for case in [
        "missing",
        "partial",
        "foreign",
        "admin_lock",
        "ascii_lock",
        "locked",
        "ignored",
        "hidden",
        "other_checkout",
        #[cfg(unix)]
        "symlink",
        "partial_w",
        "missing_git",
        "submodule",
    ] {
        let s = present_batch();
        let live = run_original(&s);
        let root = s.f.root.path().to_owned();
        let snapshot = std::sync::Arc::new(std::sync::Mutex::new(None));
        let saved = std::sync::Arc::clone(&snapshot);
        batch::observe(batch::Boundary::ImmediatePrune, move |_| {
            let admin = root.join(".git/worktrees").join(ITEM);
            let w = root.join(".manyhands/worktrees").join(ITEM);
            match case {
                "missing" => {
                    std::fs::remove_file(admin.join("manyhands-retirement-instance")).unwrap()
                }
                "partial" => std::fs::write(
                    admin.join("manyhands-retirement-instance"),
                    b"manyhands-retirement-instance-v1\n",
                )
                .unwrap(),
                "foreign" => std::fs::write(
                    admin.join("manyhands-retirement-instance"),
                    format!(
                        "manyhands-retirement-instance-v1\ndocument\n{ITEM}\n{}\n",
                        OperationId::new()
                    ),
                )
                .unwrap(),
                "admin_lock" | "ascii_lock" => {
                    std::fs::create_dir_all(admin.join("refs/heads")).unwrap();
                    std::fs::write(
                        admin.join(if case == "admin_lock" {
                            "refs/heads/foreign.lock"
                        } else {
                            "refs/heads/foreign.LoCK"
                        }),
                        b"foreign",
                    )
                    .unwrap();
                }
                "locked" => std::fs::write(admin.join("locked"), b"foreign checkout lock").unwrap(),
                "partial_w" => {
                    std::fs::remove_file(w.join(format!(".manyhands/tickets/{ITEM}/ticket.md")))
                        .unwrap()
                }
                "missing_git" => std::fs::remove_file(w.join(".git")).unwrap(),
                "submodule" => {
                    let git = git2::Repository::open(&w).unwrap();
                    let mut index = git.index().unwrap();
                    let mut e = index
                        .get_path(
                            Path::new(&format!(".manyhands/tickets/{ITEM}/ticket.md")),
                            0,
                        )
                        .unwrap();
                    e.path = b"nested-module".to_vec();
                    e.mode = 0o160000;
                    e.id = git.head().unwrap().target().unwrap();
                    index.add(&e).unwrap();
                    index.write().unwrap();
                }
                "ignored" => {
                    std::fs::write(w.join(".gitignore"), "ignored-secret\n").unwrap();
                    std::fs::write(w.join("ignored-secret"), b"do not remove").unwrap();
                }
                "hidden" => {
                    let git = git2::Repository::open(&w).unwrap();
                    let mut index = git.index().unwrap();
                    let mut e = index
                        .get_path(
                            Path::new(&format!(".manyhands/tickets/{ITEM}/ticket.md")),
                            0,
                        )
                        .unwrap();
                    e.flags |= 0x8000;
                    index.add(&e).unwrap();
                    index.write().unwrap();
                }
                "other_checkout" => {
                    let other = root.join(".git/worktrees").join(SECOND);
                    std::fs::write(
                        other.join("HEAD"),
                        format!("ref: refs/heads/manyhands/ticket/{ITEM}\n"),
                    )
                    .unwrap();
                }
                #[cfg(unix)]
                "symlink" => {
                    std::fs::remove_file(w.join(".git")).unwrap();
                    std::os::unix::fs::symlink(admin.join("gitdir"), w.join(".git")).unwrap();
                }
                _ => unreachable!(),
            }
            *saved.lock().unwrap() = Some(disk_image(&root));
        });
        let outcome = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        assert_present_debt(
            &s,
            RetirementRemoval::None,
            RetirementRemoval::None,
            outcome,
        );
        assert_eq!(
            *snapshot.lock().unwrap(),
            Some(disk_image(s.f.root.path())),
            "{case}"
        );
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        record::audit_connection(&s.f.db()).unwrap();
    }
}

#[test]
fn retirement_batch_present_known_nonce_or_primary_change_before_w_uses_original_noeffect_witness()
{
    for point in [
        batch::Boundary::WorktreeIntentCommitted,
        batch::Boundary::ImmediatePrune,
    ] {
        for case in ["nonce", "primary", "bytes"] {
            let s = present_batch();
            let live = run_original(&s);
            let root = s.f.root.path().to_owned();
            batch::observe(point, move |_| match case {
                "nonce" => std::fs::write(
                    root.join(format!(
                        ".git/worktrees/{ITEM}/manyhands-retirement-instance"
                    )),
                    format!(
                        "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
                        OperationId::new()
                    ),
                )
                .unwrap(),
                "primary" => {
                    let git = git2::Repository::open(&root).unwrap();
                    let old = git.head().unwrap().peel_to_commit().unwrap();
                    let tree = old.tree().unwrap();
                    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
                    git.commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        "same-tree primary advancement",
                        &tree,
                        &[&old],
                    )
                    .unwrap();
                }
                "bytes" => std::fs::write(
                    root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
                    b"dirty primary bytes",
                )
                .unwrap(),
                _ => unreachable!(),
            });
            let m = row_image(&s.f.db(), "remote_poll_materializations");
            let outcome = live
                .next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap();
            assert!(
                matches!(outcome,ContextRetirementOutcome::Final(ref r) if r.code==crate::results::ResultCode::ExternalChange && r.worktree_removal==RetirementRemoval::None && r.branch_removal==RetirementRemoval::None),
                "{point:?}/{case}: {outcome:?}"
            );
            let db = s.f.db();
            assert_eq!(m, row_image(&db, "remote_poll_materializations"));
            assert_eq!(counts(&db), (1, 1, 1));
            assert!(
                s.f.root
                    .path()
                    .join(".manyhands/worktrees")
                    .join(ITEM)
                    .exists()
            );
            assert_eq!(
                db.query_row(
                    "SELECT observation FROM context_retirement_effect_intents",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "changed"
            );
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
        }
    }
}

#[test]
fn retirement_batch_present_after_w_change_is_partial_never_noeffect_final() {
    for point in [
        batch::Boundary::WorktreeCommitted,
        batch::Boundary::IntentCommitted,
        batch::Boundary::LockedBranch,
    ] {
        let s = present_batch();
        let live = run_original(&s);
        let root = s.f.root.path().to_owned();
        batch::observe(point, move |_| {
            std::fs::write(
                root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
                b"dirty after W",
            )
            .unwrap();
        });
        let m = row_image(&s.f.db(), "remote_poll_materializations");
        let outcome = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        assert_present_debt(
            &s,
            RetirementRemoval::Observed,
            RetirementRemoval::None,
            outcome,
        );
        let db = s.f.db();
        assert_eq!(m, row_image(&db, "remote_poll_materializations"));
        assert!(
            !s.f.root
                .path()
                .join(".manyhands/worktrees")
                .join(ITEM)
                .exists()
        );
        assert!(
            git2::Repository::open(s.f.root.path())
                .unwrap()
                .find_reference(&format!("refs/heads/manyhands/ticket/{ITEM}"))
                .is_ok()
        );
        if point != batch::Boundary::WorktreeCommitted {
            assert_eq!(db.query_row("SELECT observation FROM context_retirement_effect_intents WHERE resource='branch'",[],|r|r.get::<_,String>(0)).unwrap(),"changed");
        }
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        record::audit_connection(&db).unwrap();
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_batch_present_prune_sql_gap_loses_permit_and_cannot_reconstruct_surviving_b() {
    for sql_gap in [false, true] {
        let s = present_batch();
        let live = run_original(&s);
        if sql_gap {
            batch::observe(batch::Boundary::Pruned, |db| {
                db.execute_batch("CREATE TRIGGER fail_w_observation BEFORE UPDATE OF observation ON context_retirement_effect_intents BEGIN SELECT RAISE(ABORT,'W SQL gap'); END;").unwrap();
            });
        } else {
            batch::observe(batch::Boundary::Pruned, |_| {
                std::panic::resume_unwind(Box::new(()))
            });
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
        }));
        if sql_gap {
            assert_present_debt(
                &s,
                RetirementRemoval::Possible,
                RetirementRemoval::None,
                result.unwrap().unwrap(),
            );
        } else {
            assert!(result.is_err());
        }
        let db = s.f.db();
        assert_eq!(
            db.query_row("SELECT phase FROM context_retirement_records", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "worktree_removing"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM context_retirement_effect_intents WHERE observation IS NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(
            !s.f.root
                .path()
                .join(".manyhands/worktrees")
                .join(ITEM)
                .exists()
        );
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
        assert!(
            live.continue_no_effect(&s.f.service, s.f.root.path())
                .is_err()
        );
        let other = RepositoryService::open_at(s.f.data.path()).unwrap();
        assert!(
            other
                .retire_context(RetireContextRequest {
                    root: s.f.root.path().into(),
                    kind: AuthoringKind::Ticket,
                    item_id: ITEM.parse().unwrap(),
                    operation_id: OperationId::new()
                })
                .is_err()
        );
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
    }
}

#[test]
fn retirement_batch_present_exact_ownership_fences_every_cleanup_boundary() {
    for point in [
        batch::Boundary::ImmediatePrune,
        batch::Boundary::Pruned,
        batch::Boundary::LockedBranch,
        batch::Boundary::Deleted,
    ] {
        for sql in [
            "UPDATE request_records SET attempt=attempt+1,cancel_requested=1",
            "UPDATE request_operations SET family='local'",
            "UPDATE poll_retirement_cursors SET next_ordinal=1",
            "UPDATE poll_retirement_attempts SET request_attempt=request_attempt+1",
            "UPDATE remote_polling_state SET configuration_generation=configuration_generation+1",
            "UPDATE remote_observation_batches SET is_current=0",
            "INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,completed_step,observed_at) SELECT id,root_path,'01ARZ3NDEKTSV4RRFFQ69G5FAX','refresh','completed','comment_registration_pending',1 FROM repositories",
        ] {
            let s = present_batch();
            let live = run_original(&s);
            let db = s.f.db();
            let m = row_image(&db, "remote_poll_materializations");
            batch::observe(point, move |db| {
                db.execute_batch(sql).unwrap();
            });
            let outcome = live
                .next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap();
            let (w, b) = match point {
                batch::Boundary::ImmediatePrune => {
                    (RetirementRemoval::None, RetirementRemoval::None)
                }
                batch::Boundary::Pruned => (RetirementRemoval::Possible, RetirementRemoval::None),
                batch::Boundary::LockedBranch => {
                    (RetirementRemoval::Observed, RetirementRemoval::None)
                }
                batch::Boundary::Deleted => {
                    (RetirementRemoval::Observed, RetirementRemoval::Possible)
                }
                _ => unreachable!(),
            };
            assert_present_debt(&s, w, b, outcome);
            assert_eq!(m, row_image(&db, "remote_poll_materializations"));
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
            assert!(
                live.continue_no_effect(&s.f.service, s.f.root.path())
                    .is_err()
            );
            record::audit_connection(&db).unwrap();
        }
    }
}

#[test]
fn retirement_batch_present_original_configuration_source_fences_cleanup_but_not_index() {
    for point in [
        Some(batch::Boundary::ImmediatePrune),
        Some(batch::Boundary::WorktreeCommitted),
        Some(batch::Boundary::LockedBranch),
        None,
    ] {
        let s = prepared_with_setup(|f| {
            let path = f.data.path().join("inert-original-key-source");
            std::fs::write(&path, b"synthetic key source").unwrap();
            let crate::repository::keys::RegisterSharedKeyOutcome::Registered(key) = f
                .service
                .register_shared_key(crate::repository::keys::RegisterSharedKeyRequest {
                    label: "fixture".into(),
                    ownership: crate::repository::keys::SharedKeyOwnership::Imported,
                    private_key_path: path,
                    public_key_path: None,
                })
                .unwrap()
            else {
                panic!("key registration")
            };
            f.service.select_shared_key(key.id).unwrap();
        });
        let live = run_original(&s);
        let path = s.f.data.path().join("inert-original-key-source");
        let replace = move || std::fs::write(path, b"replacement source identity").unwrap();
        if let Some(point) = point {
            batch::observe(point, move |_| replace());
        } else {
            s.f.service.set_refresh_persist_hook_for_testing(replace);
        }
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        match point {
            Some(batch::Boundary::ImmediatePrune) => {
                assert_present_debt(&s, RetirementRemoval::None, RetirementRemoval::None, result)
            }
            Some(_) => assert_present_debt(
                &s,
                RetirementRemoval::Observed,
                RetirementRemoval::None,
                result,
            ),
            None => assert!(
                matches!(result,ContextRetirementOutcome::Final(ref r) if r.worktree_removal==RetirementRemoval::Observed)
            ),
        }
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        record::audit_connection(&s.f.db()).unwrap();
    }
}

#[test]
fn retirement_batch_present_w_and_partial_b_witness_retry_is_metadata_only() {
    for partial in [false, true] {
        let s = present_batch();
        let live = run_original(&s);
        let root = s.f.root.path().to_owned();
        batch::observe(
            if partial {
                batch::Boundary::LockedBranch
            } else {
                batch::Boundary::ImmediatePrune
            },
            move |_| {
                std::fs::write(
                    root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
                    b"known primary change",
                )
                .unwrap();
                batch::observe(batch::Boundary::NoEffect, |db| {
                    db.execute_batch("CREATE TRIGGER fail_known_noeffect BEFORE UPDATE OF observation ON context_retirement_effect_intents BEGIN SELECT RAISE(ABORT,'metadata rollback'); END;").unwrap();
                });
            },
        );
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        assert_present_debt(
            &s,
            if partial {
                RetirementRemoval::Observed
            } else {
                RetirementRemoval::None
            },
            RetirementRemoval::None,
            result,
        );
        // The saved witness, not current proof/nonce availability, authorizes
        // only this metadata settlement. It cannot prune, consume M or start C.
        std::fs::remove_file(
            s.f.root
                .path()
                .join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
        )
        .unwrap();
        let db = s.f.db();
        let disk = disk_image(s.f.root.path());
        let m = row_image(&db, "remote_poll_materializations");
        let ad = row_image(&db, "remote_observation_batches");
        let result = live
            .continue_no_effect(&s.f.service, s.f.root.path())
            .unwrap();
        if partial {
            assert_present_debt(
                &s,
                RetirementRemoval::Observed,
                RetirementRemoval::None,
                result,
            );
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        } else {
            assert!(matches!(result, ContextRetirementOutcome::Final(_)));
        }
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_eq!(m, row_image(&db, "remote_poll_materializations"));
        assert_eq!(ad, row_image(&db, "remote_observation_batches"));
        assert!(
            live.continue_no_effect(&s.f.service, s.f.root.path())
                .is_err()
        );
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_batch_present_d14_replacement_and_sql_fault_preserve_exact_m_and_partial_truth() {
    for replace in [false, true] {
        let s = present_batch();
        let live = run_original(&s);
        let old = row_image(&s.f.db(), "remote_poll_materializations");
        let replacement_owner = s.operation.to_string();
        let replaced = std::sync::Arc::new(std::sync::Mutex::new(None));
        let saved = std::sync::Arc::clone(&replaced);
        batch::observe(batch::Boundary::IntentCommitted, move |db| {
            // Install a durable fault AFTER W, before the B transaction. D14
            // must roll back its exact deletion and no authoritative step leaks.
            if replace {
                db.execute("DELETE FROM remote_poll_materializations", [])
                    .unwrap();
                db.execute("INSERT INTO remote_poll_materializations SELECT repository_id,kind,item_id,?1,materialization_oid,'complete' FROM context_retirement_records",[replacement_owner]).unwrap();
                *saved.lock().unwrap() = Some(row_image(db, "remote_poll_materializations"));
            } else {
                db.execute_batch("CREATE TRIGGER fail_present_d14 BEFORE DELETE ON remote_poll_materializations BEGIN SELECT RAISE(ABORT,'D14 failed'); END;").unwrap();
            }
        });
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        assert_present_debt(
            &s,
            RetirementRemoval::Observed,
            if replace {
                RetirementRemoval::None
            } else {
                RetirementRemoval::Possible
            },
            result,
        );
        let db = s.f.db();
        assert_eq!(
            row_image(&db, "remote_poll_materializations"),
            if replace {
                replaced.lock().unwrap().take().unwrap()
            } else {
                old
            }
        );
        assert_eq!(
            db.query_row(
                "SELECT completed_step FROM operation_records WHERE action='retire_context'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "context_worktree_released"
        );
        assert!(
            !db.query_row(
                "SELECT materialization_settled FROM context_retirement_records",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
        );
        assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_batch_present_index_only_continuation_and_spent_claim_never_repeat_cleanup() {
    for spent in [false, true] {
        let s = present_batch();
        let live = run_original(&s);
        if spent {
            s.f.service
                .set_refresh_persist_hook_for_testing(|| std::panic::resume_unwind(Box::new(())));
        } else {
            batch::observe(batch::Boundary::BeforeClaim, |db| {
                db.execute_batch("CREATE TRIGGER fail_present_claim BEFORE INSERT ON context_retirement_index_claims BEGIN SELECT RAISE(ABORT,'claim rollback'); END;").unwrap();
            });
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
        }));
        if spent {
            assert!(result.is_err());
        } else {
            assert_eq!(
                result.unwrap().unwrap(),
                ContextRetirementOutcome::IndexPending
            );
        }
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
        let effects = row_image(&db, "context_retirement_effect_intents");
        let disk = disk_image(s.f.root.path());
        let result = live.continue_index(&s.f.service, s.f.root.path()).unwrap();
        if spent {
            assert_eq!(result, ContextRetirementOutcome::IndexPending);
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        } else {
            assert!(
                matches!(result,ContextRetirementOutcome::Final(ref r) if r.worktree_removal==RetirementRemoval::Observed)
            );
            assert_eq!(
                db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_eq!(effects, row_image(&db, "context_retirement_effect_intents"));
        assert_eq!(counts(&db), (1, 1, 1));
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_batch_present_index_result_cursor_and_last_z_are_atomic() {
    for last in [false, true] {
        let s = present_batch();
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
        let before = std::sync::Arc::new(std::sync::Mutex::new(None));
        let saved = std::sync::Arc::clone(&before);
        batch::observe(batch::Boundary::Finalizer, move |db| {
            *saved.lock().unwrap() = Some(row_image(db, "poll_retirement_targets"));
            if last {
                record::set_release_failure(true);
            } else {
                db.execute_batch("CREATE TRIGGER fail_present_cursor BEFORE UPDATE OF next_ordinal ON poll_retirement_cursors BEGIN SELECT RAISE(ABORT,'cursor rollback'); END;").unwrap();
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
        assert_eq!(
            row_image(&db, "poll_retirement_targets"),
            before.lock().unwrap().take().unwrap()
        );
        assert_eq!(counts(&db), (1, 1, 1));
        assert_eq!(
            db.query_row(
                "SELECT next_ordinal FROM poll_retirement_cursors",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            i64::from(last)
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM context_retirement_records WHERE phase='complete'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            i64::from(last)
        );
        assert_eq!(
            live.continue_index(&s.f.service, s.f.root.path()).unwrap(),
            ContextRetirementOutcome::IndexPending
        );
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_batch_present_final_readonly_replay_preserves_recreated_w_b_nonce_m() {
    let s = present_batch();
    let live = run_original(&s);
    let result = live
        .next(&s.f.service, s.f.root.path())
        .unwrap()
        .execute_branch_only(&s.f.service)
        .unwrap();
    let ContextRetirementOutcome::Final(report) = result else {
        panic!("complete")
    };
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let tip = git.head().unwrap().peel_to_commit().unwrap();
    let branch = git
        .branch(&format!("manyhands/ticket/{ITEM}"), &tip, false)
        .unwrap()
        .into_reference();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&branch));
    git.worktree(
        ITEM,
        &s.f.root.path().join(".manyhands/worktrees").join(ITEM),
        Some(&options),
    )
    .unwrap();
    std::fs::write(
        git.commondir()
            .join("worktrees")
            .join(ITEM)
            .join("manyhands-retirement-instance"),
        format!(
            "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
            OperationId::new()
        ),
    )
    .unwrap();
    let db = s.f.db();
    db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,s.operation.to_string(),tip.id().to_string()]).unwrap();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert_eq!(
        batch::read_final(&s.f.service, s.f.root.path(), s.operation, 0).unwrap(),
        Some(report)
    );
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
    record::audit_connection(&db).unwrap();
}

#[test]
fn retirement_batch_present_same_oid_recreated_admin_is_a_new_nonce_lifetime() {
    let s = present_batch();
    let live = run_original(&s);
    let root = s.f.root.path().to_owned();
    let snapshot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let saved = std::sync::Arc::clone(&snapshot);
    batch::observe(batch::Boundary::WorktreeIntentCommitted, move |_| {
        let git = git2::Repository::open(&root).unwrap();
        let branch = git
            .find_reference(&format!("refs/heads/manyhands/ticket/{ITEM}"))
            .unwrap();
        let tip = branch.target().unwrap();
        git.find_worktree(ITEM)
            .unwrap()
            .prune(Some(
                git2::WorktreePruneOptions::new()
                    .valid(true)
                    .working_tree(true),
            ))
            .unwrap();
        let w = root.join(".manyhands/worktrees").join(ITEM);
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&branch));
        git.worktree(ITEM, &w, Some(&options)).unwrap();
        assert_eq!(
            git2::Repository::open(&w).unwrap().head().unwrap().target(),
            Some(tip)
        );
        std::fs::write(
            git.commondir()
                .join("worktrees")
                .join(ITEM)
                .join("manyhands-retirement-instance"),
            format!(
                "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
                OperationId::new()
            ),
        )
        .unwrap();
        *saved.lock().unwrap() = Some(disk_image(&root));
    });
    let result = live
        .next(&s.f.service, s.f.root.path())
        .unwrap()
        .execute_branch_only(&s.f.service)
        .unwrap();
    assert!(
        matches!(result,ContextRetirementOutcome::Final(ref r) if r.code==crate::results::ResultCode::ExternalChange && r.worktree_removal==RetirementRemoval::None)
    );
    assert_eq!(*snapshot.lock().unwrap(), Some(disk_image(s.f.root.path())));
    record::audit_connection(&s.f.db()).unwrap();
}

#[test]
fn retirement_batch_present_ticket_then_promoted_document_reuses_same_receipt() {
    let mut s = SourceFixture::new();
    a2_projections(&s.f.db());
    let first = candidate_id(&s, true, ITEM);
    let root = s.f.root.path().to_owned();
    let git = git2::Repository::open(&root).unwrap();
    let path = root.join("docs/promoted.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {SECOND}\ntitle: Promoted\n---\nBody\n")).unwrap();
    let bytes = std::fs::read(&path).unwrap();
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
            "promoted fixture",
            &tree,
            &[&parent],
        )
        .unwrap();
    let branch = git
        .reference(
            &format!("refs/heads/manyhands/document/{SECOND}"),
            tip,
            false,
            "selected document",
        )
        .unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&branch));
    git.worktree(
        SECOND,
        &root.join(".manyhands/worktrees").join(SECOND),
        Some(&options),
    )
    .unwrap();
    let nonce = OperationId::new();
    std::fs::write(
        git.commondir()
            .join("worktrees")
            .join(SECOND)
            .join("manyhands-retirement-instance"),
        format!("manyhands-retirement-instance-v1\ndocument\n{SECOND}\n{nonce}\n"),
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
    let second = serde_json::json!({"ordinal":1,"kind":"document","item_id":SECOND,"initial_resources":"present","expected_tip":tip.to_string(),"selection_epoch":1,"instance_nonce":nonce.to_string(),"instance_verified":true,"child_operation_id":OperationId::new().to_string(),"materialization_capture":"absent","materialization_operation_id":null,"materialization_oid":null,"materialization_step":null});
    let entries = vec![first, second];
    input(&s, &serde_json::to_string(&entries).unwrap(), 2);
    projections(&s, &entries);
    s.seal().unwrap();
    let live = run_original(&s);
    let db = s.f.db();
    let receipt = row_image(&db, "retirement_observation_receipts");
    for item in [ITEM, SECOND] {
        assert!(
            matches!(live.next(&s.f.service,&root).unwrap().execute_branch_only(&s.f.service).unwrap(),ContextRetirementOutcome::Final(ref r) if r.worktree_removal==RetirementRemoval::Observed && r.branch_removal==RetirementRemoval::Observed)
        );
        assert!(!root.join(".manyhands/worktrees").join(item).exists());
        assert!(!git.commondir().join("worktrees").join(item).exists());
        assert_eq!(receipt, row_image(&db, "retirement_observation_receipts"));
    }
    assert_eq!(bytes, std::fs::read(path).unwrap());
    assert_eq!(counts(&db), (1, 0, 2));
    record::audit_connection(&db).unwrap();
}

// Inert original source projection at F2 > 1, followed by REAL present cleanup,
// D14 and index claim. F2 is not the independent first index executor epoch.
fn present_claim_history(stage: &str, original_attempt: i64) -> (SourceFixture, batch::Live) {
    let mut s = SourceFixture::new();
    for _ in 1..original_attempt {
        s.f.service.enter_request(s.request_id).unwrap().unwrap();
    }
    s.f.db()
        .execute(
            "UPDATE poll_retirement_attempts SET request_attempt=?1",
            [original_attempt],
        )
        .unwrap();
    a2_projections(&s.f.db());
    let entry = candidate_id(&s, true, ITEM);
    input(&s, &serde_json::to_string(&vec![entry.clone()]).unwrap(), 1);
    projections(&s, &[entry]);
    s.seal().unwrap();
    let live = run_original(&s);
    if stage == "complete" {
        assert!(matches!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap(),
            ContextRetirementOutcome::Final(_)
        ));
    } else {
        let saved = std::sync::Arc::new(std::sync::Mutex::new(None));
        let slot = std::sync::Arc::clone(&saved);
        // Stop after the real committed claim, outside the service hook mutex.
        batch::observe_claim(move |owner| {
            *slot.lock().unwrap() = Some(owner);
            std::panic::resume_unwind(Box::new(()));
        });
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                live.next(&s.f.service, s.f.root.path())
                    .unwrap()
                    .execute_branch_only(&s.f.service)
            }))
            .is_err()
        );
        let owner = saved.lock().unwrap().take().unwrap();
        if stage == "index_invalidated" {
            state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                crate::repository::recovery::invalidate_index_observations(tx, id)
            })
            .unwrap();
        } else if stage != "indexing" {
            // Existing guarded journal states; not a claim that the interrupted
            // fresh index emitted every one of these states by itself.
            assert!(
                crate::repository::recovery::transition_indexing(&s.f.db(), owner, stage, None)
                    .unwrap()
            );
        }
    }
    let db = s.f.db();
    let counters: (i64, i64, i64, i64) = db.query_row(
        "SELECT a.request_attempt,i.request_attempt,i.index_owner_epoch,o.index_owner_epoch FROM context_retirement_records c JOIN context_retirement_attempts a USING(operation_ulid) JOIN context_retirement_index_claims i USING(operation_ulid) JOIN operation_records o ON o.id=c.local_record_id",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).unwrap();
    assert_eq!(
        counters,
        (
            original_attempt,
            original_attempt,
            1,
            if stage == "index_invalidated" { 2 } else { 1 }
        )
    );
    record::audit_connection(&db).unwrap();
    (s, live)
}

#[test]
fn retirement_batch_present_claim_history_wrong_original_attempt_is_not_spend_evidence() {
    let mut accepted = Vec::new();
    for stage in [
        "complete",
        "indexing",
        "observed",
        "persisted",
        "failed",
        "retry",
        "index_invalidated",
    ] {
        let (s, _live) = present_claim_history(stage, 2);
        let db = s.f.db();
        let claim: (String, i64, String, i64) = db.query_row(
            "SELECT operation_ulid,repository_id,request_ulid,index_owner_epoch FROM context_retirement_index_claims",
            [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).unwrap();
        let delete_guard: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='retirement_index_claim_delete'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // Completed spends are deletable under the real catalog. Pending cases
        // are explicit corruption fixtures: restore the EXACT canonical trigger
        // before auditing, with FK/CHECK constraints continuously enabled.
        if stage != "complete" {
            db.execute_batch("DROP TRIGGER retirement_index_claim_delete")
                .unwrap();
        }
        assert_eq!(
            db.execute("DELETE FROM context_retirement_index_claims", [])
                .unwrap(),
            1
        );
        if stage != "complete" {
            db.execute_batch(&delete_guard).unwrap();
        }
        db.execute(
            "INSERT INTO context_retirement_index_claims VALUES(?1,?2,?3,1,?4)",
            params![claim.0, claim.1, claim.2, claim.3],
        )
        .unwrap();
        assert_eq!(
            db.query_row(
                "SELECT sql FROM sqlite_master WHERE name='retirement_index_claim_delete'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            delete_guard
        );
        assert!(
            !db.prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap()
        );
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        if record::audit_connection(&db).is_ok() {
            accepted.push(stage);
        }
        if stage == "complete" {
            assert!(
                batch::read_final(&s.f.service, s.f.root.path(), s.operation, 0).is_err(),
                "wrong original-attempt claim must not authorize historical replay"
            );
        }
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
    }
    assert!(
        accepted.is_empty(),
        "wrong-attempt original present spends accepted: {accepted:?}"
    );
}

#[test]
fn retirement_batch_present_claim_history_replay_uses_original_not_current_attempt_or_epoch() {
    for original_attempt in [2, 3] {
        for stage in ["complete", "index_invalidated"] {
            let (s, live) = present_claim_history(stage, original_attempt);
            let old = batch::read_final(&s.f.service, s.f.root.path(), s.operation, 0).unwrap();
            s.f.service.enter_request(s.request_id).unwrap().unwrap();
            let db = s.f.db();
            assert_eq!(
                db.query_row("SELECT attempt FROM request_records", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                original_attempt + 1
            );
            let before = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            record::audit_connection(&db).unwrap();
            assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
            assert_eq!(
                batch::read_final(&s.f.service, s.f.root.path(), s.operation, 0).unwrap(),
                old
            );
            assert_eq!(old.is_some(), stage == "complete");
            // Historical acceptance cannot restore the old executor's authority.
            assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
            assert_eq!(before, registry_image(&db));
            assert_eq!(disk, disk_image(s.f.root.path()));
        }
    }
}
