//! S1/S2 LIMITED SPEC reproductions: original A3 and real cleanup/index;
//! synthetic ads/inert producer projections, not a live SSH producer journey.
use super::*;

fn new_b_lifetime(s: &SourceFixture) -> (String, String) {
    let git = git2::Repository::open(s.f.root.path()).unwrap();
    let tip = git.head().unwrap().target().unwrap();
    let branch = format!("refs/heads/manyhands/ticket/{ITEM}");
    git.reference(&branch, tip, false, "new post-D14 B lifetime")
        .unwrap();
    s.f.db().execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,s.operation.to_string(),tip.to_string()]).unwrap();
    (branch, tip.to_string())
}

#[test]
fn retirement_batch_specfix_s1_unspent_index_ignores_changed_deletion_configuration() {
    for changed in [false, true] {
        let s = branches();
        let live = run_original(&s);
        batch::observe(batch::Boundary::BeforeClaim, |db| {
            db.execute_batch("CREATE TRIGGER fail_specfix_claim BEFORE UPDATE OF state ON operation_records WHEN NEW.state='indexing' BEGIN SELECT RAISE(ABORT,'unspent claim fault'); END;").unwrap()
        });
        assert_eq!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap(),
            ContextRetirementOutcome::IndexPending
        );
        let (branch, tip) = new_b_lifetime(&s);
        let db = s.f.db();
        let m = row_image(&db, "remote_poll_materializations");
        let intents = row_image(&db, "context_retirement_effect_intents");
        if changed {
            git2::Repository::open(s.f.root.path())
                .unwrap()
                .config()
                .unwrap()
                .set_str(
                    "remote.origin.pushurl",
                    "ssh://specfix.invalid/new-endpoint",
                )
                .unwrap();
        }
        let result = live.continue_index(&s.f.service, s.f.root.path());
        assert!(
            matches!(result, Ok(ContextRetirementOutcome::Final(_))),
            "changed={changed}: {result:?}"
        );
        assert_eq!(m, row_image(&db, "remote_poll_materializations"));
        assert_eq!(intents, row_image(&db, "context_retirement_effect_intents"));
        assert_eq!(
            git2::Repository::open(s.f.root.path())
                .unwrap()
                .refname_to_id(&branch)
                .unwrap()
                .to_string(),
            tip
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
        assert_eq!(counts(&db), (1, 1, 1));
        if changed {
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        }
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_batch_specfix_s1_inflight_index_ignores_endpoint_and_current_cancel() {
    for cancel in [false, true] {
        let s = branches();
        let live = run_original(&s);
        let root = s.f.root.path().to_owned();
        let data = s.f.data.path().to_owned();
        let operation = s.operation;
        s.f.service.set_refresh_persist_hook_for_testing(move||{
            let git=git2::Repository::open(&root).unwrap();let tip=git.head().unwrap().target().unwrap();
            git.reference(&format!("refs/heads/manyhands/ticket/{ITEM}"),tip,false,"new post-cleanup B lifetime").unwrap();
            let db=Connection::open(data.join(REGISTRY_FILE)).unwrap();
            db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,operation.to_string(),tip.to_string()]).unwrap();
            if cancel {db.execute("UPDATE request_records SET cancel_requested=1",[]).unwrap();}
            else {git.config().unwrap().set_str("remote.origin.pushurl","ssh://specfix.invalid/inflight-endpoint").unwrap();}
        });
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service);
        assert!(
            matches!(result, Ok(ContextRetirementOutcome::Final(_))),
            "cancel={cancel}: {result:?}"
        );
        let db = s.f.db();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT cancel_requested FROM request_records", [], |r| r
                .get::<_, bool>(
                0
            ))
            .unwrap(),
            cancel
        );
        assert_eq!(counts(&db), (1, 1, 1));
        assert!(live.next(&s.f.service, s.f.root.path()).is_err());
        assert_eq!(
            db.query_row(
                "SELECT next_ordinal FROM poll_retirement_cursors",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert!(
            git2::Repository::open(s.f.root.path())
                .unwrap()
                .find_reference(&format!("refs/heads/manyhands/ticket/{ITEM}"))
                .is_ok()
        );
    }
}

#[test]
fn retirement_batch_specfix_s2_known_dirty_primary_noeffect_must_settle_current_child() {
    for point in [
        None,
        Some(batch::Boundary::Begin),
        Some(batch::Boundary::IntentCommitted),
        Some(batch::Boundary::LockedBranch),
    ] {
        let s = branches();
        let live = run_original(&s);
        let stale = live.next(&s.f.service, s.f.root.path()).unwrap();
        let path =
            s.f.root
                .path()
                .join(format!(".manyhands/tickets/{ITEM}/ticket.md"));
        let bytes = std::fs::read(&path).unwrap();
        let dirty = path.clone();
        if let Some(point) = point {
            batch::observe(point, move |_| {
                std::fs::write(dirty, b"positively inspected dirty primary item bytes").unwrap()
            });
        } else {
            std::fs::write(dirty, b"positively inspected dirty primary item bytes").unwrap();
        }
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        assert!(
            matches!(result,ContextRetirementOutcome::Final(ref report) if report.code==crate::results::ResultCode::ExternalChange && report.worktree_removal==RetirementRemoval::None && report.branch_removal==RetirementRemoval::None && !report.materialization_settled),
            "{point:?}: {result:?}"
        );
        let db = s.f.db();
        assert_eq!(
            db.query_row(
                "SELECT next_ordinal FROM poll_retirement_cursors",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
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
        assert_eq!(counts(&db), (1, 1, 1));
        record::audit_connection(&db).unwrap();
        assert!(
            git2::Repository::open(s.f.root.path())
                .unwrap()
                .find_reference(&format!("refs/heads/manyhands/ticket/{ITEM}"))
                .is_ok()
        );
        std::fs::write(path, bytes).unwrap();
        assert!(stale.execute_branch_only(&s.f.service).is_err());
        assert!(matches!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap(),
            ContextRetirementOutcome::Final(_)
        ));
        assert_eq!(counts(&db), (1, 0, 2));
    }
}

#[test]
fn retirement_batch_specfix_s1_supersession_and_generation_allow_index_and_exact_old_z() {
    for generation in [false, true] {
        for last in [false, true] {
            let s = branches();
            let live = run_original(&s);
            if last {
                live.next(&s.f.service, s.f.root.path())
                    .unwrap()
                    .execute_branch_only(&s.f.service)
                    .unwrap();
            }
            let db = s.f.db();
            let old:i64=db.query_row("SELECT observation_batch_id FROM retirement_observation_pins WHERE pin_kind='consumer_attempt'",[],|r|r.get(0)).unwrap();
            let root = s.f.root.path().to_owned();
            let data = s.f.data.path().to_owned();
            let plan = s.plan.clone();
            let operation = s.operation;
            s.f.service.set_refresh_persist_hook_for_testing(move||{
                let git=git2::Repository::open(&root).unwrap();let tip=git.head().unwrap().target().unwrap();
                git.reference(&format!("refs/heads/manyhands/ticket/{ITEM}"),tip,false,"new post-cleanup B lifetime").unwrap();
                let db=Connection::open(data.join(REGISTRY_FILE)).unwrap();
                db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,operation.to_string(),tip.to_string()]).unwrap();
                // Real shared journal producers, not another child observation.
                let service=RepositoryService::open_at(&data).unwrap();
                state::with_transaction(&service,&root,|tx,id|{
                    if generation {state::configure(tx,id,Some(&plan),true)?;}
                    else {
                        let rows=vec![RemoteRefObservation::from_advertisement(&plan,"refs/heads/main",tip,Some(tip)).unwrap()];
                        state::persist_complete_advertisement(tx,id,&plan,state::generation(tx,id)?,&rows,999)?;
                    }
                    Ok(())
                }).unwrap();
            });
            let result = live
                .next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap();
            assert!(
                matches!(result, ContextRetirementOutcome::Final(_)),
                "generation={generation},last={last}: {result:?}"
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM remote_poll_materializations",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM retirement_advertisement_provenance",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM remote_observation_batches WHERE id=?1",
                    [old],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                i64::from(!last)
            );
            assert_eq!(counts(&db), if last { (1, 0, 2) } else { (1, 1, 1) });
            if !last {
                assert_eq!(db.query_row("SELECT observation_batch_id FROM retirement_observation_pins WHERE pin_kind='consumer_attempt'",[],|r|r.get::<_,i64>(0)).unwrap(),old);
            }
            assert!(live.next(&s.f.service, s.f.root.path()).is_err());
            record::audit_connection(&db).unwrap();
            assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
        }
    }
}

#[test]
fn retirement_batch_specfix_s1_wrong_metadata_scope_refuses_before_actual_cache_writes() {
    for sql in [
        "UPDATE request_records SET attempt=attempt+1",
        "UPDATE request_operations SET family='local'",
        "UPDATE poll_retirement_batches SET owner_epoch=owner_epoch+1",
        "UPDATE poll_retirement_cursors SET next_ordinal=1",
        "UPDATE operation_records SET index_owner_epoch=index_owner_epoch+1 WHERE action='retire_context'",
        "UPDATE operation_records SET completed_step=NULL WHERE action='retire_context'",
    ] {
        let s = branches();
        let live = run_original(&s);
        let data = s.f.data.path().to_owned();
        let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
        let capture = std::sync::Arc::clone(&observed);
        s.f.service.set_refresh_persist_hook_for_testing(move || {
            let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
            db.execute_batch(sql).unwrap();
            *capture.lock().unwrap() = Some(registry_image(&db));
        });
        assert_eq!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap(),
            ContextRetirementOutcome::IndexPending,
            "{sql}"
        );
        assert_eq!(
            *observed.lock().unwrap(),
            Some(registry_image(&s.f.db())),
            "{sql}"
        );
        assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
    }
}

#[test]
fn retirement_batch_specfix_s2_noeffect_all_settlement_faults_retry_only_original_witness() {
    for (last, table, column, condition) in [
        (
            false,
            "context_retirement_effect_intents",
            "observation",
            "NEW.observation='changed'",
        ),
        (
            false,
            "context_retirement_records",
            "phase",
            "NEW.phase='refused'",
        ),
        (
            false,
            "context_retirement_attempts",
            "phase",
            "NEW.phase='ended'",
        ),
        (false, "operation_records", "state", "NEW.state='completed'"),
        (
            false,
            "poll_retirement_targets",
            "state",
            "NEW.state='refused'",
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
        (true, "Z", "", ""),
    ] {
        let s = branches();
        let live = run_original(&s);
        if last {
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap();
        }
        let item = if last { SECOND } else { ITEM };
        let root = s.f.root.path().to_owned();
        // A retained original witness is created by typed successful inspection,
        // then the real result transaction fails. No saved-row reconstruction.
        let snapshot = std::sync::Arc::new(std::sync::Mutex::new(None));
        let capture = std::sync::Arc::clone(&snapshot);
        batch::observe(batch::Boundary::IntentCommitted, move |_| {
            std::fs::write(
                root.join(format!(".manyhands/tickets/{item}/ticket.md")),
                b"known dirty bytes before invocation",
            )
            .unwrap();
            batch::observe(batch::Boundary::NoEffect, move |db| {
                *capture.lock().unwrap() = Some((registry_image(db), disk_image(&root)));
                if table == "Z" {
                    record::set_release_failure(true);
                } else {
                    db.execute_batch(&format!("CREATE TRIGGER specfix_noeffect_fault BEFORE UPDATE OF {column} ON {table} WHEN {condition} BEGIN SELECT RAISE(ABORT,'noeffect settlement fault'); END;")).unwrap();
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
            },
            "{table}"
        );
        let db = s.f.db();
        let before = snapshot
            .lock()
            .unwrap()
            .take()
            .expect("actual no-effect transaction reached");
        assert_eq!(before.0, registry_image(&db), "{table}");
        assert_eq!(before.1, disk_image(s.f.root.path()), "{table}");
        assert_owed(&s, i64::from(last), false);
        let git = git2::Repository::open(s.f.root.path()).unwrap();
        let tip = git.head().unwrap().target().unwrap();
        git.reference(
            &format!("refs/heads/manyhands/ticket/{item}"),
            tip,
            true,
            "new lifetime while original known-noeffect witness retained",
        )
        .unwrap();
        db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![item,s.operation.to_string(),tip.to_string()]).unwrap();
        let disk = disk_image(s.f.root.path());
        let m = row_image(&db, "remote_poll_materializations");
        let ads = row_image(&db, "remote_observation_batches");
        let other = RepositoryService::open_at(s.f.data.path()).unwrap();
        assert!(live.continue_no_effect(&other, s.f.root.path()).is_err());
        let ContextRetirementOutcome::Final(report) = live
            .continue_no_effect(&s.f.service, s.f.root.path())
            .unwrap()
        else {
            panic!("original metadata witness retry {table}");
        };
        assert_eq!(report.branch_removal, RetirementRemoval::None);
        assert_eq!(report.worktree_removal, RetirementRemoval::None);
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_eq!(m, row_image(&db, "remote_poll_materializations"));
        assert_eq!(ads, row_image(&db, "remote_observation_batches"));
        let image = registry_image(&db);
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
        assert_eq!(counts(&db), if last { (1, 0, 2) } else { (1, 1, 1) });
        record::audit_connection(&db).unwrap();
    }
}

#[test]
fn retirement_batch_specfix_s2_unknown_inspection_and_stale_owner_never_authorize_noeffect_final() {
    for point in [
        batch::Boundary::Begin,
        batch::Boundary::IntentCommitted,
        batch::Boundary::LockedBranch,
    ] {
        for stale in [false, true] {
            let s = branches();
            let live = run_original(&s);
            let root = s.f.root.path().to_owned();
            batch::observe(point, move |db| {
                if stale {
                    db.execute("UPDATE request_records SET attempt=attempt+1", [])
                        .unwrap();
                }
                std::fs::remove_file(root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")))
                    .unwrap();
            });
            let result = live
                .next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap();
            assert!(
                !matches!(result, ContextRetirementOutcome::Final(_)),
                "{point:?},stale={stale}"
            );
            assert!(
                live.continue_no_effect(&s.f.service, s.f.root.path())
                    .is_err(),
                "inspection errors cannot mint a known-change witness"
            );
            assert_owed(&s, 0, false);
        }
    }
}

#[test]
fn retirement_batch_specfix_s2_retained_witness_requires_current_f2_map_and_no_reconstruction() {
    for lost in ["f2", "map", "process"] {
        let s = branches();
        let live = run_original(&s);
        let root = s.f.root.path().to_owned();
        batch::observe(batch::Boundary::IntentCommitted, move |_| {
            std::fs::write(
                root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
                b"known dirty current primary",
            )
            .unwrap();
            batch::observe(batch::Boundary::NoEffect, |db| {
                db.execute_batch("CREATE TRIGGER fail_specfix_noeffect BEFORE UPDATE OF phase ON context_retirement_records WHEN NEW.phase='refused' BEGIN SELECT RAISE(ABORT,'known-noeffect fault'); END;").unwrap()
            });
        });
        assert!(matches!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .execute_branch_only(&s.f.service)
                .unwrap(),
            ContextRetirementOutcome::CleanupPending { .. }
        ));
        let db = s.f.db();
        let child: String = db
            .query_row(
                "SELECT operation_ulid FROM context_retirement_records",
                [],
                |r| r.get(0),
            )
            .unwrap();
        if lost == "f2" {
            s.f.service.enter_request(s.request_id).unwrap().unwrap();
            db.execute("UPDATE request_records SET cancel_requested=1", [])
                .unwrap();
        }
        if lost == "map" {
            db.execute("UPDATE request_operations SET family='local'", [])
                .unwrap();
        }
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        if lost == "process" {
            drop(live);
            let other = RepositoryService::open_at(s.f.data.path()).unwrap();
            assert!(
                other
                    .retire_context(RetireContextRequest {
                        root: s.f.root.path().into(),
                        kind: AuthoringKind::Ticket,
                        item_id: ITEM.parse().unwrap(),
                        operation_id: OperationId::parse(&child).unwrap()
                    })
                    .is_err()
            );
            assert!(
                other
                    .refresh_repository(RefreshRepositoryRequest {
                        root: s.f.root.path().into(),
                        operation_id: OperationId::parse(&child).unwrap()
                    })
                    .is_err()
            );
        } else {
            assert!(!matches!(
                live.continue_no_effect(&s.f.service, s.f.root.path()),
                Ok(ContextRetirementOutcome::Final(_))
            ));
            assert!(live.continue_index(&s.f.service, s.f.root.path()).is_err());
        }
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
    }
}

#[test]
fn retirement_batch_specfix_s2_last_noeffect_metadata_can_release_old_g_holder_without_reproof() {
    let s = branches();
    let live = run_original(&s);
    live.next(&s.f.service, s.f.root.path())
        .unwrap()
        .execute_branch_only(&s.f.service)
        .unwrap();
    let root = s.f.root.path().to_owned();
    batch::observe(batch::Boundary::IntentCommitted, move |_| {
        std::fs::write(
            root.join(format!(".manyhands/tickets/{SECOND}/ticket.md")),
            b"known dirty last child primary",
        )
        .unwrap();
        batch::observe(batch::Boundary::NoEffect, |_| {
            record::set_release_failure(true);
        });
    });
    assert!(matches!(
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap(),
        ContextRetirementOutcome::CleanupPending { .. }
    ));
    let db = s.f.db();
    let old:i64=db.query_row("SELECT observation_batch_id FROM retirement_observation_pins WHERE pin_kind='consumer_attempt'",[],|r|r.get(0)).unwrap();
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        state::configure(tx, id, Some(&s.plan), true)?;
        tx.execute("UPDATE request_records SET cancel_requested=1", [])
            .map_err(RepositoryError::sqlite)?;
        Ok(())
    })
    .unwrap();
    let disk = disk_image(s.f.root.path());
    let result = live
        .continue_no_effect(&s.f.service, s.f.root.path())
        .unwrap();
    assert!(
        matches!(result,ContextRetirementOutcome::Final(ref r) if r.code==crate::results::ResultCode::ExternalChange && r.branch_removal==RetirementRemoval::None)
    );
    assert_eq!(disk, disk_image(s.f.root.path()));
    assert_eq!(counts(&db), (1, 0, 2));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_observation_batches WHERE id=?1",
            [old],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(
        db.query_row("SELECT cancel_requested FROM request_records", [], |r| r
            .get::<_, bool>(
            0
        ))
        .unwrap()
    );
    assert!(live.next(&s.f.service, s.f.root.path()).is_err());
    record::audit_connection(&db).unwrap();
}

#[test]
fn retirement_batch_specfix_s2_noeffect_final_keeps_exact_captured_complete_materialization() {
    for after_intent in [false, true] {
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
        let path =
            s.f.root
                .path()
                .join(format!(".manyhands/tickets/{ITEM}/ticket.md"));
        if after_intent {
            batch::observe(batch::Boundary::IntentCommitted, move |_| {
                std::fs::write(path, b"known primary item mismatch").unwrap()
            });
        } else {
            std::fs::write(path, b"known primary item mismatch").unwrap();
        }
        let result = live
            .next(&s.f.service, s.f.root.path())
            .unwrap()
            .execute_branch_only(&s.f.service)
            .unwrap();
        assert!(
            matches!(result,ContextRetirementOutcome::Final(ref r) if r.code==crate::results::ResultCode::ExternalChange && !r.materialization_settled && r.branch_removal==RetirementRemoval::None)
        );
        assert_eq!(m, row_image(&db, "remote_poll_materializations"));
        assert_eq!(counts(&db), (1, 0, 2));
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
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}
