//! INERT F2 projections, REAL retirement protocol. Not Task 3 mutation journeys.
use super::*;
use repository::{RetirementAccepted, request_store::*};

fn projected(w: &World, r: &RetireContextRequest) -> Result<RetirementAccepted, FixtureError> {
    let id = RequestId::new();
    let scope = ScopeKey::for_repository(&r.root).ok_or(FixtureError)?;
    let digest = IntentDigest::builder(
        DigestSalt::Request(id),
        "context retire",
        &scope,
        &r.item_id.to_string(),
    )
    .field(
        "kind",
        FieldValue::Text(match r.kind {
            AuthoringKind::Ticket => "ticket",
            AuthoringKind::Document => "document",
        }),
    )
    .finish();
    fixed(w.service.insert_request(&NewRequest {
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
    }))?;
    let record = fixed(w.service.request_record(id))?.ok_or(FixtureError)?;
    RetirementAccepted::of_record(&record, record.attempt).ok_or(FixtureError)
}
pub(super) fn fresh() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    let rows = registry_image(&w.db()?);
    assert!(
        w.service
            .retire_context_authenticated(
                r.clone(),
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert_eq!(rows, registry_image(&w.db()?));
    let result = w.service.retire_context_for_request(
        r.clone(),
        &accepted,
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    );
    assert!(
        matches!(&result, Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::Retired && report.worktree_removal==RetirementRemoval::Observed && report.branch_removal==RetirementRemoval::Observed),
        "valid current F2 projection must execute the real protocol"
    );
    assert!(w.repo()?.find_reference(CONTEXT).is_err());
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
pub(super) fn identity() -> Result<(), FixtureError> {
    for sql in [
        "DELETE FROM request_operations",
        "UPDATE request_operations SET ordinal=1",
        "UPDATE request_operations SET family='remote'",
        "INSERT INTO request_operations SELECT request_ulid,1,family,operation_ulid FROM request_operations",
        "UPDATE request_records SET attempt=attempt+1",
        "UPDATE request_records SET scope_key='/foreign'",
        "UPDATE request_records SET command='context release'",
        "UPDATE request_records SET target='01ARZ3NDEKTSV4RRFFQ69G5FAV'",
        "UPDATE request_records SET intent_digest=printf('%064d',0)",
    ] {
        let w = setup()?;
        add_present(&w)?;
        history(&w, true)?;
        let r = request(&w);
        let accepted = projected(&w, &r)?;
        fixed(w.db()?.execute_batch(sql))?;
        let rows = registry_image(&w.db()?);
        let resources = resource_image(&w.root);
        let commands = w.server.commands();
        assert!(
            w.service
                .retire_context_for_request(
                    r,
                    &accepted,
                    w.primary().approval,
                    &mut SessionCredentials::new(Provider)
                )
                .is_err()
        );
        assert_eq!(rows, registry_image(&w.db()?));
        assert!(resources == resource_image(&w.root));
        assert_eq!(commands, w.server.commands());
        w.privacy()?;
    }
    for case in [
        "foreign_owner",
        "duplicate_owner",
        "wrong_kind",
        "wrong_item",
        "wrong_root",
        "wrong_mode_digest",
        "missing_request",
        "finished_request",
    ] {
        let w = setup()?;
        add_present(&w)?;
        history(&w, true)?;
        let mut r = request(&w);
        let accepted = projected(&w, &r)?;
        match case {
            "foreign_owner" | "duplicate_owner" => {
                let _foreign = projected(&w, &r)?;
                if case == "foreign_owner" {
                    fixed(w.db()?.execute(
                        "DELETE FROM request_operations WHERE request_ulid=?1",
                        [accepted.request_id.to_string()],
                    ))?;
                }
            }
            "wrong_kind" => r.kind = AuthoringKind::Document,
            "wrong_item" => r.item_id = fixed("01ARZ3NDEKTSV4RRFFQ69G5FAV".parse())?,
            "wrong_root" => r.root = w.root.join(".manyhands"),
            "wrong_mode_digest" => {
                let scope = ScopeKey::for_repository(&r.root).ok_or(FixtureError)?;
                let digest = IntentDigest::builder(
                    DigestSalt::Request(accepted.request_id),
                    "context retire",
                    &scope,
                    &r.item_id.to_string(),
                )
                .field("kind", FieldValue::Text("ticket"))
                .field("mode", FieldValue::Text("release"))
                .finish();
                fixed(w.db()?.execute(
                    "UPDATE request_records SET intent_digest=?1",
                    [digest.to_string()],
                ))?;
            }
            "missing_request" => {
                fixed(w.db()?.execute_batch(
                    "DELETE FROM request_operations; DELETE FROM request_records;",
                ))?;
            }
            _ => {
                assert!(fixed(w.service.finish_request(
                    accepted.request_id,
                    accepted.attempt,
                    &RequestResult {
                        outcome: Outcome::Cancelled,
                        code: ResultCode::Cancelled,
                        effects: Effects::not_requested(),
                        data: None
                    }
                ))?);
            }
        }
        let rows = registry_image(&w.db()?);
        let resources = resource_image(&w.root);
        let commands = w.server.commands();
        assert!(
            w.service
                .retire_context_for_request(
                    r,
                    &accepted,
                    None,
                    &mut SessionCredentials::new(SpecfixNoCredentials)
                )
                .is_err(),
            "{case}"
        );
        assert_eq!(rows, registry_image(&w.db()?), "{case}");
        assert!(resources == resource_image(&w.root), "{case}");
        assert_eq!(commands, w.server.commands(), "{case}");
        w.privacy()?;
    }
    Ok(())
}

fn advanced(service: &RepositoryService, old: &RetirementAccepted) -> RetirementAccepted {
    let attempt = service.enter_request(old.request_id).unwrap().unwrap();
    let row = service.request_record(old.request_id).unwrap().unwrap();
    RetirementAccepted::of_record(&row, attempt).unwrap()
}
pub(super) fn network() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    let hold = w.server.hold_advertisement()?;
    let data = w.data();
    let r2 = r.clone();
    std::thread::scope(|threads| {
        let old = &accepted;
        let controller = threads.spawn(move || {
            hold.wait_until_held().unwrap();
            let service = RepositoryService::open_at(&data).unwrap();
            let new = advanced(&service, old);
            let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
            let before = registry_image(&db);
            assert!(
                service
                    .reenter_context_for_request(
                        ReenterContextRetirementRequest {
                            retirement: r2.clone(),
                            expected_owner_epoch: 1
                        },
                        old,
                        None,
                        &mut SessionCredentials::new(Provider)
                    )
                    .is_err()
            );
            assert_eq!(before, registry_image(&db));
            RepositoryService::set_retirement_selected_hook_for_testing(|| {
                std::panic::resume_unwind(Box::new(()))
            });
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = service.reenter_context_for_request(
                        ReenterContextRetirementRequest {
                            retirement: r2,
                            expected_owner_epoch: 1,
                        },
                        &new,
                        None,
                        &mut SessionCredentials::new(Provider),
                    );
                }))
                .is_err(),
                "new current F2 must atomically replace and reach the real executor"
            );
            assert_eq!(
                db.query_row(
                    "SELECT owner_epoch FROM context_retirement_records",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                2
            );
            let rows = registry_image(&db);
            hold.release().unwrap();
            rows
        });
        assert!(
            w.service
                .retire_context_for_request(
                    r.clone(),
                    &accepted,
                    w.primary().approval,
                    &mut SessionCredentials::new(Provider)
                )
                .is_err()
        );
        assert_eq!(controller.join().unwrap(), registry_image(&w.db().unwrap()));
    });
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    let new = advanced(&w.service, &accepted);
    let result = w.service.reenter_context_for_request(
        ReenterContextRetirementRequest {
            retirement: r,
            expected_owner_epoch: 2,
        },
        &new,
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    );
    assert!(
        matches!(result,Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::Retired)
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
pub(super) fn index() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    let data = w.data();
    let id = accepted.request_id;
    w.service.set_refresh_claim_hook_for_testing(move || {
        let service = RepositoryService::open_at(&data).unwrap();
        service.enter_request(id).unwrap().unwrap();
    });
    let _ = w.service.retire_context_for_request(
        r.clone(),
        &accepted,
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    );
    assert_eq!(
        fixed(
            w.db()?
                .query_row("SELECT phase FROM context_retirement_records", [], |r| {
                    r.get::<_, String>(0)
                })
        )?,
        "index_pending"
    );
    let before = registry_image(&w.db()?);
    assert!(
        w.service
            .retire_context_for_request(
                r.clone(),
                &accepted,
                None,
                &mut SessionCredentials::new(SpecfixNoCredentials)
            )
            .is_err()
    );
    assert!(
        w.service
            .refresh_repository(RefreshRepositoryRequest {
                root: w.root.clone(),
                operation_id: r.operation_id
            })
            .is_err()
    );
    assert_eq!(before, registry_image(&w.db()?));
    let row = fixed(w.service.request_record(id))?.ok_or(FixtureError)?;
    let new = RetirementAccepted::of_record(&row, row.attempt).ok_or(FixtureError)?;
    let tip: String = fixed(w.db()?.query_row(
        "SELECT expected_tip FROM context_retirement_records",
        [],
        |r| r.get(0),
    ))?;
    fixed(w.repo()?.reference(
        CONTEXT,
        fixed(tip.parse())?,
        false,
        "new lifetime before I-only",
    ))?;
    add_present(&w)?;
    let new_intent = complete_materialization(&w)?;
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    let result = w.service.retire_context_for_request(
        r,
        &new,
        None,
        &mut SessionCredentials::new(SpecfixNoCredentials),
    );
    assert!(
        matches!(result,Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::Retired),
        "new current request must finish I-only, not adopt stale index authority"
    );
    assert!(resources == resource_image(&w.root));
    assert_materialization(&w, &new_intent)?;
    assert_eq!(commands, w.server.commands());
    w.privacy()
}
pub(super) fn cancel() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    let data = w.data();
    RepositoryService::set_retirement_selected_hook_for_testing(move || {
        rusqlite::Connection::open(data.join(REGISTRY_FILE))
            .unwrap()
            .execute("UPDATE request_records SET cancel_requested=1", [])
            .unwrap();
    });
    let result = w.service.retire_context_for_request(
        r,
        &accepted,
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    );
    assert!(
        matches!(result,Ok(ContextRetirementOutcome::Final(report)) if report.code==ResultCode::Cancelled && report.worktree_removal==RetirementRemoval::None && report.branch_removal==RetirementRemoval::None),
        "only current cancellation owner may finalize before effect"
    );
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert!(!fixed(w.db()?.query_row(
        "SELECT cancel_requested FROM request_records",
        [],
        |r| r.get::<_, bool>(0)
    ))?);
    w.privacy()
}
pub(super) fn effect_boundaries() -> Result<(), FixtureError> {
    for (branch, change) in [
        (false, "advance"),
        (true, "advance"),
        (false, "mapping"),
        (true, "mapping"),
        (false, "finished"),
        (true, "finished"),
        (true, "cancel"),
    ] {
        let w = setup()?;
        add_present(&w)?;
        history(&w, true)?;
        let r = request(&w);
        let accepted = projected(&w, &r)?;
        let data = w.data();
        let id = accepted.request_id;
        let hook = move || {
            let service = RepositoryService::open_at(&data).unwrap();
            match change {
                "advance" => {
                    service.enter_request(id).unwrap().unwrap();
                }
                "mapping" => {
                    rusqlite::Connection::open(data.join(REGISTRY_FILE))
                        .unwrap()
                        .execute("DELETE FROM request_operations", [])
                        .unwrap();
                }
                "cancel" => {
                    rusqlite::Connection::open(data.join(REGISTRY_FILE))
                        .unwrap()
                        .execute("UPDATE request_records SET cancel_requested=1", [])
                        .unwrap();
                }
                _ => {
                    assert!(
                        service
                            .finish_request(
                                id,
                                1,
                                &RequestResult {
                                    outcome: Outcome::Cancelled,
                                    code: ResultCode::Cancelled,
                                    effects: Effects::not_requested(),
                                    data: None
                                }
                            )
                            .unwrap()
                    );
                }
            }
        };
        if branch {
            RepositoryService::set_retirement_intent_hook_for_testing(hook);
        } else {
            RepositoryService::set_retirement_worktree_hook_for_testing(hook);
        }
        let result = w.service.retire_context_for_request(
            r,
            &accepted,
            w.primary().approval,
            &mut SessionCredentials::new(Provider),
        );
        assert!(w.repo()?.find_reference(CONTEXT).is_ok());
        assert_eq!(
            fixed(w.db()?.query_row(
                "SELECT worktree_removal,branch_removal FROM context_retirement_records",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            ))?,
            (
                if branch { "observed" } else { "possible" }.into(),
                if branch && change != "cancel" {
                    "possible"
                } else {
                    "none"
                }
                .into()
            )
        );
        if change == "cancel" {
            assert!(matches!(
                result,
                Ok(ContextRetirementOutcome::CleanupPending {
                    worktree_removal: RetirementRemoval::Observed,
                    branch_removal: RetirementRemoval::None
                })
            ));
            assert!(fixed(w.db()?.query_row(
                "SELECT cancel_requested FROM request_records",
                [],
                |r| r.get::<_, bool>(0)
            ))?);
        }
        assert_eq!(
            RepositoryService::open_at(&w.data()).is_ok(),
            change != "mapping"
        );
        w.privacy()?;
    }
    Ok(())
}
pub(super) fn metadata() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    complete_materialization(&w)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    RepositoryService::set_retirement_worktree_hook_for_testing(|| {
        std::panic::resume_unwind(Box::new(()))
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = w.service.retire_context_for_request(
                r.clone(),
                &accepted,
                w.primary().approval,
                &mut SessionCredentials::new(Provider),
            );
        }))
        .is_err()
    );
    fixed(w.repo()?.find_worktree(ITEM).and_then(|tree| {
        tree.prune(Some(
            git2::WorktreePruneOptions::new()
                .valid(true)
                .working_tree(true),
        ))
    }))?;
    fixed(
        w.repo()?
            .find_reference(CONTEXT)
            .and_then(|mut r| r.delete()),
    )?;
    let new = advanced(&w.service, &accepted);
    let commands = w.server.commands();
    let result = w.service.reenter_context_for_request(
        ReenterContextRetirementRequest {
            retirement: r.clone(),
            expected_owner_epoch: 1,
        },
        &new,
        None,
        &mut SessionCredentials::new(SpecfixNoCredentials),
    );
    assert!(
        matches!(&result,Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::Retired && report.worktree_removal==RetirementRemoval::Possible && report.branch_removal==RetirementRemoval::None && report.materialization_settled)
    );
    let result = fixed(result)?;
    let tip = fixed(w.db()?.query_row(
        "SELECT expected_tip FROM context_retirement_records",
        [],
        |r| r.get::<_, String>(0),
    ))?;
    fixed(
        w.repo()?
            .reference(CONTEXT, fixed(tip.parse())?, false, "new lifetime"),
    )?;
    add_present(&w)?;
    let new_intent = complete_materialization(&w)?;
    let mut effects = Effects::not_requested();
    effects.cleanup = results::CleanupEffect::Complete;
    assert!(fixed(w.service.finish_request(
        new.request_id,
        new.attempt,
        &RequestResult {
            outcome: Outcome::Success,
            code: ResultCode::Ok,
            effects,
            data: None
        }
    ))?);
    let rows = registry_image(&w.db()?);
    let resources = resource_image(&w.root);
    let replay = w.service.retire_context_for_request(
        r.clone(),
        &new,
        None,
        &mut SessionCredentials::new(SpecfixNoCredentials),
    );
    assert!(
        replay.is_ok(),
        "finished request replay must remain available without reexecution"
    );
    assert_eq!(
        fixed(replay)?,
        result,
        "finished request replay is read-only, not executed as new"
    );
    assert!(w.service.retire_context(r).is_err());
    assert_eq!(rows, registry_image(&w.db()?));
    assert!(resources == resource_image(&w.root));
    assert_materialization(&w, &new_intent)?;
    assert_eq!(commands, w.server.commands());
    w.privacy()
}
pub(super) fn initial() -> Result<(), FixtureError> {
    for cancelled in [false, true] {
        let w = setup()?;
        history(&w, true)?;
        if !cancelled {
            fixed(
                w.repo()?
                    .find_reference(CONTEXT)
                    .and_then(|mut r| r.delete()),
            )?;
        }
        let r = request(&w);
        let accepted = projected(&w, &r)?;
        if cancelled {
            fixed(
                w.db()?
                    .execute("UPDATE request_records SET cancel_requested=1", []),
            )?;
        }
        let commands = w.server.commands();
        let result = w.service.retire_context_for_request(
            r,
            &accepted,
            None,
            &mut SessionCredentials::new(SpecfixNoCredentials),
        );
        assert!(
            matches!(result,Ok(ContextRetirementOutcome::Final(report)) if if cancelled {report.code==ResultCode::Cancelled}else{report.outcome==RetirementDisposition::AlreadyAbsent}),
            "current F2 initial absence/cancel must have stable owned result"
        );
        assert_eq!(commands, w.server.commands());
        assert!(RepositoryService::open_at(&w.data()).is_ok());
        w.privacy()?;
    }
    Ok(())
}
pub(super) fn rollback() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    RepositoryService::set_retirement_selected_hook_for_testing(|| {
        std::panic::resume_unwind(Box::new(()))
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = w.service.retire_context_for_request(
                r.clone(),
                &accepted,
                w.primary().approval,
                &mut SessionCredentials::new(Provider),
            );
        }))
        .is_err()
    );
    let new = advanced(&w.service, &accepted);
    fixed(w.db()?.execute_batch("CREATE TRIGGER f2_replace_failure BEFORE INSERT ON context_retirement_reentries BEGIN SELECT RAISE(ABORT,'fixture'); END;"))?;
    let rows = registry_image(&w.db()?);
    let resources = resource_image(&w.root);
    assert!(
        w.service
            .reenter_context_for_request(
                ReenterContextRetirementRequest {
                    retirement: r,
                    expected_owner_epoch: 1
                },
                &new,
                None,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert_eq!(rows, registry_image(&w.db()?));
    assert!(resources == resource_image(&w.root));
    w.privacy()
}

pub(super) fn reentry_cancel() -> Result<(), FixtureError> {
    for partial in [false, true] {
        let w = setup()?;
        add_present(&w)?;
        history(&w, true)?;
        let r = request(&w);
        let accepted = projected(&w, &r)?;
        let crash = || std::panic::resume_unwind(Box::new(()));
        if partial {
            RepositoryService::set_retirement_worktree_hook_for_testing(crash);
        } else {
            RepositoryService::set_retirement_selected_hook_for_testing(crash);
        }
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = w.service.retire_context_for_request(
                    r.clone(),
                    &accepted,
                    w.primary().approval,
                    &mut SessionCredentials::new(Provider),
                );
            }))
            .is_err()
        );
        let new = advanced(&w.service, &accepted);
        fixed(
            w.db()?
                .execute("UPDATE request_records SET cancel_requested=1", []),
        )?;
        let resources = resource_image(&w.root);
        let commands = w.server.commands();
        let result = w.service.reenter_context_for_request(
            ReenterContextRetirementRequest {
                retirement: r,
                expected_owner_epoch: 1,
            },
            &new,
            None,
            &mut SessionCredentials::new(SpecfixNoCredentials),
        );
        if partial {
            assert!(
                matches!(
                    result,
                    Ok(ContextRetirementOutcome::CleanupPending {
                        worktree_removal: RetirementRemoval::Possible,
                        branch_removal: RetirementRemoval::None
                    })
                ),
                "current cancellation retains old uncertainty as Owed"
            );
        } else {
            assert!(
                matches!(result,Ok(ContextRetirementOutcome::Final(report)) if report.code==ResultCode::Cancelled && report.worktree_removal==RetirementRemoval::None && report.branch_removal==RetirementRemoval::None),
                "new current caller can acknowledge no-effect cancellation without observing"
            );
        }
        assert_eq!(
            fixed(
                w.db()?
                    .query_row("SELECT cancel_requested FROM request_records", [], |r| r
                        .get::<_, bool>(
                        0
                    ))
            )?,
            partial
        );
        assert!(resources == resource_image(&w.root));
        assert_eq!(commands, w.server.commands());
        assert!(RepositoryService::open_at(&w.data()).is_ok());
        w.privacy()?;
    }
    Ok(())
}

pub(super) fn network_finished() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    let hold = w.server.hold_advertisement()?;
    let data = w.data();
    let id = accepted.request_id;
    let root = w.root.clone();
    std::thread::scope(|threads| {
        let controller = threads.spawn(move || {
            hold.wait_until_held().unwrap();
            // Bootstrap is legitimately complete before transport. Compare the
            // exact resource image at request finish, not before marker install.
            let resources = resource_image(&root);
            let service = RepositoryService::open_at(&data).unwrap();
            assert!(
                service
                    .finish_request(
                        id,
                        1,
                        &RequestResult {
                            outcome: Outcome::Cancelled,
                            code: ResultCode::Cancelled,
                            effects: Effects::not_requested(),
                            data: None
                        }
                    )
                    .unwrap()
            );
            let rows =
                registry_image(&rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap());
            hold.release().unwrap();
            (rows, resources)
        });
        assert!(
            w.service
                .retire_context_for_request(
                    r.clone(),
                    &accepted,
                    w.primary().approval,
                    &mut SessionCredentials::new(Provider)
                )
                .is_err()
        );
        let (rows, resources) = controller.join().unwrap();
        assert_eq!(rows, registry_image(&w.db().unwrap()));
        assert!(resources == resource_image(&w.root));
    });
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

pub(super) fn persistence() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    let data = w.data();
    let (sender, receiver) = std::sync::mpsc::channel();
    w.service.set_refresh_persist_hook_for_testing(move || {
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        db.execute("DELETE FROM request_operations", []).unwrap();
        sender.send(registry_image(&db)).unwrap();
    });
    assert!(
        w.service
            .retire_context_for_request(
                r.clone(),
                &accepted,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert_eq!(
        fixed(receiver.recv())?,
        registry_image(&w.db()?),
        "mapping lost after scan cannot authorize persistence, error or finalization"
    );
    fixed(w.db()?.execute(
        "INSERT INTO request_operations VALUES(?1,0,'local',?2)",
        rusqlite::params![accepted.request_id.to_string(), r.operation_id.to_string()],
    ))?;
    let new = advanced(&w.service, &accepted);
    let commands = w.server.commands();
    assert!(
        matches!(w.service.retire_context_for_request(r,&new,None,&mut SessionCredentials::new(SpecfixNoCredentials)),Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::Retired)
    );
    assert_eq!(commands, w.server.commands());
    w.privacy()
}

pub(super) fn invalidation() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let r = request(&w);
    let accepted = projected(&w, &r)?;
    let data = w.data();
    let id = accepted.request_id;
    w.service.set_refresh_claim_hook_for_testing(move || {
        let db=rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        // Exact generic invalidation projection, not a promotion journey.
        db.execute("UPDATE operation_records SET state='index_invalidated',index_owner_epoch=index_owner_epoch+1 WHERE action='retire_context' AND state='indexing'",[]).unwrap();
        RepositoryService::open_at(&data).unwrap().enter_request(id).unwrap().unwrap();
    });
    assert!(
        w.service
            .retire_context_for_request(
                r.clone(),
                &accepted,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    let row = fixed(w.service.request_record(id))?.ok_or(FixtureError)?;
    let current = RetirementAccepted::of_record(&row, row.attempt).ok_or(FixtureError)?;
    let tombstone: i64 = fixed(w.db()?.query_row(
        "SELECT index_owner_epoch FROM operation_records WHERE action='retire_context'",
        [],
        |r| r.get(0),
    ))?;
    assert_eq!(
        tombstone, current.attempt as i64,
        "regression prerequisite: distinct counters happen to coincide"
    );
    let tip: String = fixed(w.db()?.query_row(
        "SELECT expected_tip FROM context_retirement_records",
        [],
        |r| r.get(0),
    ))?;
    fixed(
        w.repo()?
            .reference(CONTEXT, fixed(tip.parse())?, false, "new lifetime"),
    )?;
    add_present(&w)?;
    let new_intent = complete_materialization(&w)?;
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    let data = w.data();
    let r2 = r.clone();
    w.service.set_refresh_claim_hook_for_testing(move || {
        let service = RepositoryService::open_at(&data).unwrap();
        let row = service.request_record(id).unwrap().unwrap();
        let same = RetirementAccepted::of_record(&row, row.attempt).unwrap();
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        let before = registry_image(&db);
        assert!(
            matches!(
                service.retire_context_for_request(
                    r2,
                    &same,
                    None,
                    &mut SessionCredentials::new(SpecfixNoCredentials)
                ),
                Ok(ContextRetirementOutcome::IndexPending)
            ),
            "duplicate current F2 cannot steal the live index executor"
        );
        assert_eq!(before, registry_image(&db));
    });
    let result = w.service.retire_context_for_request(
        r,
        &current,
        None,
        &mut SessionCredentials::new(SpecfixNoCredentials),
    );
    assert!(
        matches!(result,Ok(ContextRetirementOutcome::Final(report)) if report.outcome==RetirementDisposition::Retired),
        "immediately next current F2 must finish an invalidated I-only handoff"
    );
    let final_epoch: i64 = fixed(w.db()?.query_row(
        "SELECT index_owner_epoch FROM operation_records WHERE action='retire_context'",
        [],
        |r| r.get(0),
    ))?;
    assert!(
        final_epoch > tombstone,
        "index executor epoch must strictly advance independently of F2"
    );
    assert!(resources == resource_image(&w.root));
    assert_materialization(&w, &new_intent)?;
    assert_eq!(commands, w.server.commands());
    w.privacy()
}

pub(super) fn envelope_persistence() -> Result<(), FixtureError> {
    for sql in [
        "UPDATE operation_records SET action='refresh' WHERE action='retire_context'",
        "UPDATE operation_records SET target='wrong-mode' WHERE action='retire_context'",
        "UPDATE operation_records SET root_path='/foreign' WHERE action='retire_context'",
        "UPDATE operation_records SET repository_id=NULL WHERE action='retire_context'",
        "UPDATE operation_records SET completed_step='context_retirement_absent' WHERE action='retire_context'",
    ] {
        let w = setup()?;
        add_present(&w)?;
        history(&w, true)?;
        complete_materialization(&w)?;
        let r = request(&w);
        let accepted = projected(&w, &r)?;
        let data = w.data();
        let (sender, receiver) = std::sync::mpsc::channel();
        w.service.set_refresh_persist_hook_for_testing(move || {
            let db=rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
            db.execute("INSERT INTO remote_poll_materializations SELECT repository_id,kind,item_id,materialization_operation_ulid,'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb','complete' FROM context_retirement_records",[]).unwrap();
            db.execute_batch(sql).unwrap(); sender.send(registry_image(&db)).unwrap();
        });
        let result = w.service.retire_context_for_request(
            r,
            &accepted,
            w.primary().approval,
            &mut SessionCredentials::new(Provider),
        );
        assert!(
            result.is_err(),
            "{sql}: corrupt bound envelope must not become an ordinary refresh or Final"
        );
        assert_eq!(
            fixed(receiver.recv())?,
            registry_image(&w.db()?),
            "{sql}: exact post-change image preserved BEFORE index mutation/error/stale clear"
        );
        assert_eq!(
            fixed(w.db()?.query_row(
                "SELECT phase,final_result IS NULL FROM context_retirement_records",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?))
            ))?,
            ("index_pending".into(), true)
        );
        assert!(fixed(w.db()?.query_row(
            "SELECT refresh_required FROM repositories",
            [],
            |r| r.get::<_, bool>(0)
        ))?);
        w.privacy()?;
    }
    Ok(())
}
