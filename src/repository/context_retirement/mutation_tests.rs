//! Real shared-boundary acceptance/reentry/settlement over isolated local Git.
//! No SSH evidence is claimed. Transport results and crash/finalizer faults are synthetic.
use super::*;
use crate::repository::keys::*;
use crate::results::{DiscoveryEffect, ResultCode};

struct NoCredentials;
impl SessionCredentialProvider for NoCredentials {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("must not implicitly unlock")
    }
}
fn retire(f: &Fixture) -> Mutation {
    Mutation::ContextRetire(ContextRetireInput {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
    })
}
fn execute(
    service: &RepositoryService,
    id: RequestId,
    mutation: Mutation,
) -> crate::results::Envelope<MutationDataDto> {
    service.execute(
        MutationCall {
            request_id: id,
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

fn branch(f: &Fixture) -> String {
    let git = git2::Repository::open(f.root.path()).unwrap();
    git.remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
        .unwrap();
    let config = f.root.path().join(".manyhands/config.toml");
    let text = std::fs::read_to_string(&config).unwrap() + "publication_remote = \"origin\"\n";
    std::fs::write(config, text).unwrap();
    let branch = format!("refs/heads/manyhands/ticket/{ITEM}");
    git.reference(
        &branch,
        git.head().unwrap().target().unwrap(),
        false,
        "fixture",
    )
    .unwrap();
    branch
}

fn present(f: &Fixture) -> String {
    let branch = branch(f);
    let git = git2::Repository::open(f.root.path()).unwrap();
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    std::fs::create_dir_all(f.root.path().join(&path).parent().unwrap()).unwrap();
    std::fs::write(f.root.path().join(&path),format!("---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\ntitle: Private fixture title\ntype: task\nstatus: closed\nclosed_at: '2026-11-04T00:00:00Z'\nclosed_by: 'Owner <owner@example.invalid>'\n---\nPrivate fixture body\n")).unwrap();
    let mut index = git.index().unwrap();
    index.add_path(Path::new(&path)).unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
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
    git.reference(&branch, tip, true, "fixture").unwrap();
    let reference = git.find_reference(&branch).unwrap();
    let path = f.root.path().join(".manyhands/worktrees").join(ITEM);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    git.worktree(
        ITEM,
        &path,
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    )
    .unwrap();
    // Synthetic earlier successful Fetch history, not a network claim. The
    // injected empty new advertisement still flows through real O/receipt SQL.
    let plan = crate::repository::remote::observation::observation_plan(f.root.path()).unwrap();
    let config = f
        .service
        .observation_configuration(f.root.path(), &plan)
        .unwrap();
    crate::repository::remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
        use crate::repository::remote::state;
        state::configure_endpoints(tx, id, &plan, &config.endpoint_digest())?;
        let prior = RemoteRefObservation::from_advertisement(
            &plan,
            plan.context(AuthoringKind::Ticket, &ITEM.parse().unwrap())
                .remote_ref(),
            tip,
            Some(tip),
        )
        .unwrap();
        state::persist_complete_advertisement(
            tx,
            id,
            &plan,
            state::generation(tx, id)?,
            &[prior],
            1,
        )?;
        Ok(())
    })
    .unwrap();
    branch
}

#[test]
fn retirement_batch_live_fences_rejoin_at_each_ordinary_effect() {
    for before_branch in [false, true] {
        for fault in [
            "endpoint",
            "observation",
            "parent_f2",
            "child_f2",
            "materialization",
        ] {
            let f = Fixture::new();
            let branch = present(&f);
            f.materialization("complete");
            let git = git2::Repository::open(f.root.path()).unwrap();
            let primary = git.head().unwrap().target().unwrap();
            git.reference(
                "refs/remotes/origin/main",
                primary,
                false,
                "synthetic fetch",
            )
            .unwrap();
            let root = f.root.path().to_owned();
            let data = f.data.path().to_owned();
            let change = move || {
                let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
                match fault {
                    "endpoint" => git2::Repository::open(&root)
                        .unwrap()
                        .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/replaced")
                        .unwrap(),
                    "parent_f2" => {
                        db.execute("UPDATE request_records SET attempt=attempt+1 WHERE command='poll once' AND state='accepted'",[]).unwrap();
                    }
                    "child_f2" => {
                        db.execute("UPDATE request_records SET attempt=attempt+1 WHERE command='context retire' AND state='accepted'",[]).unwrap();
                    }
                    "materialization" => {
                        db.execute("DELETE FROM remote_poll_materializations", [])
                            .unwrap();
                        db.execute("INSERT INTO remote_poll_materializations SELECT repository_id,'ticket',?1,operation_ulid,?2,'complete' FROM poll_retirement_passes",params![ITEM,primary.to_string()]).unwrap();
                    }
                    "observation" => {
                        let service = RepositoryService::open_at(&data).unwrap();
                        let plan = crate::repository::remote::observation::observation_plan(&root)
                            .unwrap();
                        crate::repository::remote::state::with_transaction(
                            &service,
                            &root,
                            |tx, id| {
                                crate::repository::remote::state::persist_complete_advertisement(
                                    tx,
                                    id,
                                    &plan,
                                    crate::repository::remote::state::generation(tx, id)?,
                                    &[],
                                    3,
                                )?;
                                Ok(())
                            },
                        )
                        .unwrap();
                    }
                    _ => unreachable!(),
                }
            };
            if before_branch {
                RepositoryService::set_retirement_intent_hook_for_testing(change);
            } else {
                RepositoryService::set_retirement_worktree_hook_for_testing(change);
            }
            let ri = RequestId::new();
            let mutation = || {
                Mutation::PollOnceRetiring(PollOnceInput {
                    root: f.root.path().into(),
                })
            };
            crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
            let mut result = execute(&f.service, ri, mutation());
            if f.service.request_record(ri).unwrap().unwrap().state == RequestState::Accepted {
                result = execute(&f.service, ri, mutation());
            }
            assert!(
                git.find_reference(&branch).is_ok(),
                "{fault}/{before_branch}: no stale B effect"
            );
            assert_eq!(
                f.root
                    .path()
                    .join(".manyhands/worktrees")
                    .join(ITEM)
                    .exists(),
                !before_branch,
                "{fault}: truthful W state"
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM request_records WHERE command='context retire'",
                        [],
                        |r| r.get::<_, usize>(0)
                    )
                    .unwrap(),
                1
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM remote_operation_records WHERE poll_kind='observe'",
                        [],
                        |r| r.get::<_, usize>(0)
                    )
                    .unwrap(),
                0,
                "no re-observation in initial executor"
            );
            let image = serde_json::to_value(&result.data).unwrap();
            if before_branch {
                assert_eq!(image["completion"], "stopped", "{fault}: {result:?}");
                assert_eq!(image["retirements"][0]["state"], "pending");
            }
            assert_eq!(
                serde_json::to_value(execute(&f.service, ri, mutation()).data).unwrap(),
                image
            );
        }
    }
    for fault in [
        "last_token",
        "last_flag",
        "empty_token",
        "empty_flag",
        "child_only",
    ] {
        let f = Fixture::new();
        let empty = fault.starts_with("empty_");
        let branch = if empty {
            let b = branch(&f);
            git2::Repository::open(f.root.path())
                .unwrap()
                .find_reference(&b)
                .unwrap()
                .delete()
                .unwrap();
            b
        } else {
            let b = present(&f);
            f.materialization("complete");
            b
        };
        let git = git2::Repository::open(f.root.path()).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        let id = RequestId::new();
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let data = f.data.path().join(REGISTRY_FILE);
        if empty {
            super::super::handoff::observe_next_settle_for_testing(move || {
                if fault == "empty_token" {
                    token.cancel();
                } else {
                    Connection::open(data)
                        .unwrap()
                        .execute(
                            "UPDATE request_records SET cancel_requested=1 WHERE request_ulid=?1",
                            [id.to_string()],
                        )
                        .unwrap();
                }
            });
        } else if fault == "last_token" {
            RepositoryService::set_retirement_prune_entry_hook_for_testing(move || token.cancel());
        } else {
            RepositoryService::set_retirement_worktree_hook_for_testing(move || {
                let db = Connection::open(data).unwrap();
                if fault == "child_only" {
                    db.execute("UPDATE request_records SET cancel_requested=1 WHERE command='context retire' AND state='accepted'",[]).unwrap();
                } else {
                    db.execute(
                        "UPDATE request_records SET cancel_requested=1 WHERE request_ulid=?1",
                        [id.to_string()],
                    )
                    .unwrap();
                }
            });
        }
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        let mutation = || {
            Mutation::PollOnceRetiring(PollOnceInput {
                root: f.root.path().into(),
            })
        };
        let result = f.service.execute(
            MutationCall {
                request_id: id,
                confirmation: None,
                mutation: mutation(),
            },
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &cancel,
        );
        let expected = if fault == "child_only" {
            ResultCode::Ok
        } else {
            ResultCode::Cancelled
        };
        assert_eq!(
            result.code, expected,
            "S3 parent cancellation must survive the last child/empty image, but independent child refusal is still settled: {fault}: {result:?}"
        );
        let image = serde_json::to_value(&result.data).unwrap();
        assert_eq!(
            image["completion"],
            if fault == "child_only" {
                "completed"
            } else {
                "stopped"
            }
        );
        assert_eq!(
            f.service.request_record(id).unwrap().unwrap().state,
            RequestState::Finished
        );
        if !empty {
            assert!(git.find_reference(&branch).is_ok());
            assert!(
                f.root
                    .path()
                    .join(".manyhands/worktrees")
                    .join(ITEM)
                    .exists()
            );
            assert_eq!(image["retirements"][0]["state"], "refused");
            assert_eq!(
                image["retirements"][0]["result"]["worktree_removal"],
                "none"
            );
            assert_eq!(image["retirements"][0]["result"]["branch_removal"], "none");
            if fault == "last_token" || fault == "child_only" {
                assert_eq!(image["retirements"][0]["code"], "cancelled");
            }
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM remote_poll_materializations",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                1
            );
        } else {
            assert_eq!(image["retirements"].as_array().unwrap().len(), 0);
        }
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT count(*) FROM retirement_observation_pins",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            serde_json::to_value(execute(&f.service, id, mutation()).data).unwrap(),
            image,
            "immutable first cut"
        );
    }
}

#[test]
fn retirement_batch_history_audits_parent_intent_and_materialization_owner() {
    for case in ["parent_digest", "materialization_owner"] {
        let f = Fixture::new();
        present(&f);
        if case == "materialization_owner" {
            let (owner, oid) = f.materialization("complete");
            let db = f.db();
            db.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
            db.execute("DELETE FROM remote_poll_materializations", [])
                .unwrap();
            db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",params![ITEM,OperationId::new().to_string(),oid]).unwrap();
            let c = super::super::pass::Candidate {
                poll_ordinal: 0,
                kind: "ticket".into(),
                item_id: ITEM.into(),
                resources: "invalid".into(),
                tip: None,
                nonce: None,
                materialization: None,
                refusal: Some("recovery_required".into()),
                child_request: RequestId::new().to_string(),
            };
            assert!(
                super::super::pass::materialization(&db, 1, &c).is_err(),
                "foreign/missing M owner {owner} is not a complete captured intent"
            );
            continue;
        }
        let git = git2::Repository::open(f.root.path()).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        let id = RequestId::new();
        let result = execute(
            &f.service,
            id,
            Mutation::PollOnceRetiring(PollOnceInput {
                root: f.root.path().into(),
            }),
        );
        assert_eq!(result.code, ResultCode::Ok);
        use crate::repository::mutation::request_store::{DigestSalt, IntentDigest, ScopeKey};
        let ordinary = IntentDigest::builder(
            DigestSalt::Request(id),
            "poll once",
            &ScopeKey::for_repository(f.root.path()).unwrap(),
            "",
        )
        .finish();
        f.db()
            .execute(
                "UPDATE request_records SET intent_digest=?2 WHERE request_ulid=?1",
                params![id.to_string(), ordinary.to_string()],
            )
            .unwrap();
        assert!(
            super::super::record::audit_connection(&f.db()).is_err(),
            "captured poll parent must retain opted intent, not merely its IDs"
        );
    }
}

#[test]
fn retirement_batch_initial_history_requires_exact_membership() {
    for case in [
        "missing",
        "parent_request",
        "ordinal",
        "receipt",
        "candidate",
    ] {
        let f = Fixture::new();
        let branch = present(&f);
        let git = git2::Repository::open(f.root.path()).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        RepositoryService::set_retirement_selected_hook_for_testing(|| {
            panic!("loss after initial ordinary selection")
        });
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        let id = RequestId::new();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(
                &f.service,
                id,
                Mutation::PollOnceRetiring(PollOnceInput {
                    root: f.root.path().into()
                })
            )))
            .is_err()
        );
        let db = f.db();
        super::super::record::audit_connection(&db).unwrap();
        let (ri, ki): (String, String) = db
            .query_row(
                "SELECT child_request_ulid,child_operation_ulid FROM poll_retirement_child_sources",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let trigger = if case == "missing" {
            "poll_retirement_membership_retained"
        } else {
            "poll_retirement_membership_immutable"
        };
        let sql: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name=?1",
                [trigger],
                |r| r.get(0),
            )
            .unwrap();
        db.execute_batch(&format!("DROP TRIGGER {trigger}"))
            .unwrap();
        match case {
            "missing" => {
                db.execute("DELETE FROM poll_retirement_child_sources", [])
                    .unwrap();
            }
            "parent_request" => {
                db.execute("UPDATE poll_retirement_child_sources SET parent_request_ulid=child_request_ulid", []).unwrap();
            }
            "ordinal" => {
                db.execute(
                    "UPDATE poll_retirement_child_sources SET ordinal=ordinal+1",
                    [],
                )
                .unwrap();
            }
            "receipt" => {
                db.execute("UPDATE poll_retirement_child_sources SET initial_receipt_id=(SELECT binding_id FROM poll_retirement_passes)", []).unwrap();
            }
            _ => {
                db.execute("UPDATE poll_retirement_child_sources SET candidate=json_set(candidate,'$.item_id','01ARZ3NDEKTSV4RRFFQ69G5FAW')", []).unwrap();
            }
        }
        db.execute_batch(&sql).unwrap();
        let before = registry_image(&db);
        let disk = disk_image(f.root.path());
        assert!(
            super::super::record::audit_connection(&db).is_err(),
            "{case}: NULL observer alone is not initial lineage"
        );
        assert!(
            RepositoryService::open_at(f.data.path()).is_err(),
            "{case}: startup cannot reconstruct membership"
        );
        assert_eq!(before, registry_image(&db));
        let refused = execute(&f.service, RequestId::parse(&ri).unwrap(), retire(&f));
        assert_eq!(
            refused.code,
            ResultCode::RecoveryRequired,
            "{case}: {refused:?}, K={ki}"
        );
        assert_eq!(disk, disk_image(f.root.path()));
        assert!(git.find_reference(&branch).is_ok());
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_operation_records WHERE poll_kind='observe'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
}

#[test]
fn retirement_batch_original_refresh_gap_and_background_poll_cannot_mint_authority() {
    for case in ["ordinary", "automatic", "endpoint_gap", "advertisement_gap"] {
        let f = Fixture::new();
        let branch = present(&f);
        let git = git2::Repository::open(f.root.path()).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        let marker = git
            .commondir()
            .join("worktrees")
            .join(ITEM)
            .join(super::super::instance::MARKER);
        if case.ends_with("_gap") {
            let root = f.root.path().to_owned();
            let data = f.data.path().to_owned();
            super::super::handoff::observe_next_settle_for_testing(move || {
                if case == "endpoint_gap" {
                    git2::Repository::open(&root)
                        .unwrap()
                        .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/changed")
                        .unwrap();
                } else {
                    let service = RepositoryService::open_at(&data).unwrap();
                    let plan =
                        crate::repository::remote::observation::observation_plan(&root).unwrap();
                    crate::repository::remote::state::with_transaction(
                        &service,
                        &root,
                        |tx, id| {
                            crate::repository::remote::state::persist_complete_advertisement(
                                tx,
                                id,
                                &plan,
                                crate::repository::remote::state::generation(tx, id)?,
                                &[],
                                9,
                            )?;
                            Ok(())
                        },
                    )
                    .unwrap();
                }
            });
        }
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        if case == "automatic" {
            f.db().execute("UPDATE remote_polling_state SET enabled=1,paused=0,recovery_suspended=0,next_eligible_at=NULL",[]).unwrap();
            let result = f
                .service
                .poll_publication_remote(
                    PollPublicationRemoteRequest {
                        root: f.root.path().into(),
                        operation_id: OperationId::new(),
                        invocation: RemotePollInvocation::Automatic,
                        approval: None,
                        restart: false,
                    },
                    &mut SessionCredentials::new(NoCredentials),
                )
                .unwrap();
            assert!(
                matches!(
                    result,
                    RemotePollResult::Complete(RemotePollOutcome::Polled(_))
                ),
                "real eligible automatic poll: {result:?}"
            );
        } else {
            let opted = case != "ordinary";
            let id = RequestId::new();
            let mutation = || {
                if opted {
                    Mutation::PollOnceRetiring(PollOnceInput {
                        root: f.root.path().into(),
                    })
                } else {
                    Mutation::PollOnce(PollOnceInput {
                        root: f.root.path().into(),
                    })
                }
            };
            let result = execute(&f.service, id, mutation());
            assert_eq!(
                result.code,
                if opted {
                    ResultCode::DiscoveryPending
                } else {
                    ResultCode::Ok
                },
                "{case}: {result:?}"
            );
            if opted {
                let image = serde_json::to_value(&result.data).unwrap();
                f.service
                    .refresh_repository(RefreshRepositoryRequest {
                        root: f.root.path().into(),
                        operation_id: OperationId::new(),
                    })
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(execute(&f.service, id, mutation()).data).unwrap(),
                    image,
                    "later repair does not promote original wrapper"
                );
            }
        }
        assert!(git.find_reference(&branch).is_ok());
        assert_eq!(
            marker.exists(),
            case.ends_with("_gap"),
            "only opted original capture enrolls"
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT count(*) FROM retirement_observation_pins",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}

#[test]
fn retirement_batch_original_handoff_and_crash_table_never_restarts_cleanup() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    for case in [
        "empty",
        "failed",
        "cancelled",
        "index_pending",
        "capture_lost",
        "parent_accepted_crash",
        "parent_companion_lost",
        "preselection_restart",
        "preselection_fenced",
        "unconfirmed",
        "eligible_crash",
        "accepted_crash",
        "selected_crash",
        "nonce_replaced",
        "refused_copy_lost",
        "refused_copy_missing",
        "request_cancel_cut",
        "all_absent",
        "owned_absence_crash",
        "conflict_gap",
    ] {
        let f = Fixture::new();
        let b = if case == "empty" {
            let b = branch(&f);
            git2::Repository::open(f.root.path())
                .unwrap()
                .find_reference(&b)
                .unwrap()
                .delete()
                .unwrap();
            b
        } else {
            present(&f)
        };
        let git = git2::Repository::open(f.root.path()).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        if case == "all_absent" {
            f.service
                .refresh_repository(RefreshRepositoryRequest {
                    root: f.root.path().into(),
                    operation_id: OperationId::new(),
                })
                .unwrap();
            // External disappearance in an ISOLATED fixture, not owned removal.
            let path = f.root.path().join(".manyhands/worktrees").join(ITEM);
            std::fs::remove_dir_all(path).unwrap();
            git.find_worktree(ITEM)
                .unwrap()
                .prune(Some(git2::WorktreePruneOptions::new().valid(true)))
                .unwrap();
            git.find_reference(&b).unwrap().delete().unwrap();
            f.materialization("complete");
        }
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        let id = RequestId::new();
        let mutation = || {
            Mutation::PollOnceRetiring(PollOnceInput {
                root: f.root.path().into(),
            })
        };
        let producer = match case {
            "parent_accepted_crash" | "parent_companion_lost" => {
                RepositoryService::open_at_with_failure_point_for_testing(
                    f.data.path(),
                    FailurePoint::BeforeRequestDomainCall,
                )
                .unwrap()
            }
            "index_pending" => RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::BeforeIndexTransactionCommit,
            )
            .unwrap(),
            "preselection_restart" | "preselection_fenced" => {
                RepositoryService::open_at_with_failure_point_for_testing(
                    f.data.path(),
                    FailurePoint::PollBeforeDiscovery,
                )
                .unwrap()
            }
            "unconfirmed" => RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::PollAfterCompletion,
            )
            .unwrap(),
            _ => RepositoryService::open_at(f.data.path()).unwrap(),
        };
        if case == "capture_lost" {
            super::super::source::fail_capture();
        }
        if case == "eligible_crash" {
            super::super::handoff::observe_eligible(|_| {
                panic!("simulated process loss after durable eligibility")
            });
        }
        if case == "accepted_crash" {
            super::super::handoff::observe_eligible(|s| {
                s.set_request_hook_for_testing(|| {
                    panic!("simulated accepted child loss before local claim")
                })
            });
        }
        if case == "selected_crash" {
            RepositoryService::set_retirement_selected_hook_for_testing(|| {
                panic!("simulated child loss after ordinary local claim before Git")
            });
        }
        if case == "owned_absence_crash" {
            RepositoryService::set_retirement_after_branch_hook_for_testing(|| {
                panic!("simulated SQLite rollback after actual Git absence")
            });
            f.materialization("complete");
        }
        if case == "conflict_gap" {
            let root = f.root.path().to_owned();
            super::super::handoff::observe_eligible(move |s| {
                let plan = crate::repository::remote::observation::observation_plan(&root).unwrap();
                let RemoteReservationOutcome::Reserved(owner) = s
                    .reserve_remote_operation(
                        &root,
                        OperationId::new(),
                        &RemoteOperationTarget::for_primary_synchronization(&plan),
                    )
                    .unwrap()
                else {
                    panic!("owned foreign integration reserves");
                };
                s.retirement_test_conflict(&root, &owner, 1).unwrap();
            });
        }
        if case == "request_cancel_cut" {
            let data = f.data.path().join(REGISTRY_FILE);
            super::super::handoff::observe_eligible(move |s| {
                let response = s.cancel_request(id);
                assert_eq!(response.code, ResultCode::Ok);
                let cut: Option<String> = Connection::open(data)
                    .unwrap()
                    .query_row(
                        "SELECT summary FROM poll_retirement_passes WHERE request_ulid=?1",
                        [id.to_string()],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert!(
                    cut.is_some(),
                    "cancellation must revoke starts and persist its exact cut before returning"
                );
            });
        }
        if matches!(
            case,
            "nonce_replaced" | "refused_copy_lost" | "refused_copy_missing"
        ) {
            let marker = git
                .commondir()
                .join("worktrees")
                .join(ITEM)
                .join(super::super::instance::MARKER);
            super::super::handoff::observe_eligible(move |s| {
                std::fs::write(
                    marker,
                    format!(
                        "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
                        OperationId::new()
                    ),
                )
                .unwrap();
                if case.starts_with("refused_copy_") {
                    *s.failure_point.lock().unwrap() = Some(FailurePoint::BeforeRequestSettlement);
                }
            });
        }
        if case.starts_with("refused_copy_") {
            f.materialization("complete");
            super::super::handoff::observe_children(|_| {
                panic!(
                    "loss after committed refusal/ordinary copy fault, BEFORE parent reconciliation"
                )
            });
        }
        let cancel = CancellationToken::new();
        if case == "cancelled" {
            let token = cancel.clone();
            producer.set_request_hook_for_testing(move || token.cancel());
        }
        if !matches!(
            case,
            "failed" | "cancelled" | "parent_accepted_crash" | "parent_companion_lost"
        ) {
            crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        }
        let first = catch_unwind(AssertUnwindSafe(|| {
            producer.execute(
                MutationCall {
                    request_id: id,
                    confirmation: None,
                    mutation: mutation(),
                },
                &mut SessionCredentials::new(NoCredentials),
                &mut NoProgress,
                &cancel,
            )
        }));
        let response = if case.starts_with("refused_copy_") {
            assert!(first.is_err());
            let db = f.db();
            let (ri,ki):(String,String)=db.query_row("SELECT child_request_ulid,child_operation_ulid FROM poll_retirement_targets WHERE admission='accepted'",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
            assert_eq!(
                db.query_row(
                    "SELECT phase FROM context_retirement_records WHERE operation_ulid=?1",
                    [&ki],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "refused",
                "actual durable refusal precedes both copy losses"
            );
            assert_eq!(
                f.service.request_record(id).unwrap().unwrap().state,
                RequestState::Accepted
            );
            assert!(
                db.query_row(
                    "SELECT summary IS NULL FROM poll_retirement_passes",
                    [],
                    |r| r.get::<_, bool>(0)
                )
                .unwrap()
            );
            if case == "refused_copy_missing" {
                db.execute(
                    "DELETE FROM context_retirement_records WHERE operation_ulid=?1",
                    [&ki],
                )
                .unwrap();
            }
            let disk = disk_image(f.root.path());
            let tables = [
                "poll_retirement_passes",
                "poll_retirement_targets",
                "remote_poll_materializations",
                "retirement_observation_receipts",
                "retirement_observation_pins",
            ];
            let before: Vec<_> = tables.iter().map(|t| row_image(&db, t)).collect();
            let child = execute(&f.service, RequestId::parse(&ri).unwrap(), retire(&f));
            if case == "refused_copy_missing" {
                assert_eq!(
                    child.code,
                    ResultCode::RecoveryRequired,
                    "S4 missing terminal companion is not never-claimed proof"
                );
                assert_eq!(
                    f.service
                        .request_record(RequestId::parse(&ri).unwrap())
                        .unwrap()
                        .unwrap()
                        .state,
                    RequestState::Accepted
                );
                assert_eq!(disk, disk_image(f.root.path()));
                assert_eq!(
                    before,
                    tables.iter().map(|t| row_image(&db, t)).collect::<Vec<_>>()
                );
                assert_eq!(
                    db.query_row(
                        "SELECT count(*) FROM remote_operation_records WHERE poll_kind='observe'",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                    0
                );
                continue;
            }
            assert_eq!(
                child.code,
                ResultCode::ExternalChange,
                "S4 Ri-only retry must read its exact durable terminal refusal before never-claimed logic: {child:?}"
            );
            assert_eq!(
                f.service
                    .request_record(RequestId::parse(&ri).unwrap())
                    .unwrap()
                    .unwrap()
                    .state,
                RequestState::Finished
            );
            assert_eq!(
                serde_json::to_value(&child.data).unwrap()["state"],
                "refused"
            );
            assert_eq!(
                child,
                execute(&f.service, RequestId::parse(&ri).unwrap(), retire(&f))
            );
            assert_eq!(
                before,
                tables.iter().map(|t| row_image(&db, t)).collect::<Vec<_>>(),
                "child settlement cannot reconcile or depend on its parent"
            );
            assert_eq!(disk, disk_image(f.root.path()));
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM remote_operation_records WHERE poll_kind='observe'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM poll_retirement_child_sources",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0,
                "preselection refusal needs no fabricated receipt/member"
            );
            execute(&f.service, id, mutation())
        } else if case == "preselection_fenced" {
            assert!(first.unwrap().code != ResultCode::Ok);
            let db = f.db();
            git.remote_set_url("origin", "ssh://fixture@127.0.0.1:2/fenced")
                .unwrap();
            let plan =
                crate::repository::remote::observation::observation_plan(f.root.path()).unwrap();
            let config = f
                .service
                .observation_configuration(f.root.path(), &plan)
                .unwrap();
            crate::repository::remote::state::with_transaction(
                &f.service,
                f.root.path(),
                |tx, repo| {
                    crate::repository::remote::state::configure_endpoints(
                        tx,
                        repo,
                        &plan,
                        &config.endpoint_digest(),
                    )
                },
            )
            .unwrap();
            let result = execute(&f.service, id, mutation());
            assert_eq!(
                f.service.request_record(id).unwrap().unwrap().state,
                RequestState::Finished,
                "a fenced old P retains normal finality, not a new observation/loop: {result:?}"
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM retirement_observation_pins",
                    [],
                    |r| r.get::<_, usize>(0)
                )
                .unwrap(),
                0,
                "old source holds end with P, no aggregate debt"
            );
            result
        } else if matches!(case, "parent_accepted_crash" | "parent_companion_lost") {
            assert_eq!(first.unwrap().code, ResultCode::InternalError);
            assert_eq!(
                f.service.request_record(id).unwrap().unwrap().state,
                RequestState::Accepted
            );
            assert_eq!(
                f.db()
                    .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                        .get::<_, i64>(
                        0
                    ))
                    .unwrap(),
                0,
                "actual acceptance loss is before P begins"
            );
            if case == "parent_companion_lost" {
                f.db()
                    .execute("DELETE FROM poll_retirement_passes", [])
                    .unwrap();
                let refused = execute(&f.service, id, mutation());
                assert_eq!(
                    refused.code,
                    ResultCode::RecoveryRequired,
                    "S1 missing atomic companion is not a new preselection invocation"
                );
                assert_eq!(
                    f.db()
                        .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                            .get::<_, i64>(
                            0
                        ))
                        .unwrap(),
                    0,
                    "no ordinary P is started from lost initializer evidence"
                );
                assert_eq!(
                    f.db()
                        .query_row("SELECT count(*) FROM poll_retirement_passes", [], |r| r
                            .get::<_, i64>(0))
                        .unwrap(),
                    0,
                    "never recreate the missing companion on reentry"
                );
                assert_eq!(
                    f.db()
                        .query_row(
                            "SELECT count(*) FROM request_records WHERE command='context retire'",
                            [],
                            |r| r.get::<_, i64>(0)
                        )
                        .unwrap(),
                    0
                );
                assert!(git.find_reference(&b).is_ok());
                continue;
            }
            crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
            let result = execute(&f.service, id, mutation());
            assert_eq!(
                f.service.request_record(id).unwrap().unwrap().state,
                RequestState::Finished,
                "S1: accepted-before-initializer recovery must settle ordinary P, not remain Owed: {result:?}"
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM retirement_advertisement_provenance",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                0
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM retirement_observation_pins",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                0
            );
            assert!(
                !git.commondir()
                    .join("worktrees")
                    .join(ITEM)
                    .join(super::super::instance::MARKER)
                    .exists(),
                "retry mints no instance nonce"
            );
            result
        } else if matches!(case, "capture_lost" | "preselection_restart") {
            assert!(first.unwrap().code != ResultCode::Ok);
            crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
            execute(&f.service, id, mutation())
        } else if matches!(
            case,
            "eligible_crash" | "accepted_crash" | "selected_crash" | "owned_absence_crash"
        ) {
            assert!(first.is_err());
            execute(&f.service, id, mutation())
        } else {
            first.unwrap()
        };
        let data = serde_json::to_value(&response.data).unwrap();
        assert_eq!(data["type"], "poll_retirement", "{case}: {response:?}");
        if matches!(
            case,
            "empty" | "nonce_replaced" | "refused_copy_lost" | "all_absent"
        ) {
            assert_eq!(response.code, ResultCode::Ok, "{case}: {response:?}");
            assert_eq!(data["completion"], "completed");
            if matches!(case, "nonce_replaced" | "refused_copy_lost") {
                assert_eq!(data["retirements"][0]["state"], "refused");
            }
            if case == "all_absent" {
                assert_eq!(data["retirements"][0]["state"], "already_absent");
                assert_eq!(
                    data["poll"]["targets"][1]["after_oid"],
                    serde_json::Value::Null
                );
            }
        } else {
            assert!(
                !matches!(response.code, ResultCode::Ok | ResultCode::AlreadyApplied),
                "{case}: {response:?}"
            );
            assert_eq!(data["completion"], "stopped");
        }
        if !matches!(
            case,
            "nonce_replaced"
                | "refused_copy_lost"
                | "accepted_crash"
                | "selected_crash"
                | "all_absent"
                | "owned_absence_crash"
        ) {
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM request_records WHERE command='context retire'",
                        [],
                        |r| r.get::<_, usize>(0)
                    )
                    .unwrap(),
                0,
                "{case}: no child acceptance from lost/failed wrapper"
            );
        }
        if !matches!(case, "empty" | "all_absent" | "owned_absence_crash") {
            assert!(git.find_reference(&b).is_ok(), "{case}: context preserved");
        }
        assert_eq!(
            serde_json::to_value(execute(&f.service, id, mutation()).data).unwrap(),
            data,
            "{case}: zero-network stable old pass"
        );
        if case == "accepted_crash" {
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM operation_records WHERE action='retire_context'",
                        [],
                        |r| r.get::<_, usize>(0)
                    )
                    .unwrap(),
                0,
                "metadata-only no-effect settlement, not run-as-new"
            );
            assert_eq!(data["retirements"][0]["state"], "refused");
        }
        if case == "selected_crash" {
            let ri =
                RequestId::parse(data["retirements"][0]["request_id"].as_str().unwrap()).unwrap();
            assert_eq!(data["retirements"][0]["state"], "pending");
            super::super::run::synthetic_advertisement(Ok(Vec::new()));
            let child = execute(&f.service, ri, retire(&f));
            assert_eq!(
                child.code,
                ResultCode::Ok,
                "initial NULL O ordinary fresh-O recovery after parent Final: {child:?}"
            );
            assert_eq!(
                serde_json::to_value(execute(&f.service, id, mutation()).data).unwrap(),
                data
            );
        }
        if case == "owned_absence_crash" {
            let ri =
                RequestId::parse(data["retirements"][0]["request_id"].as_str().unwrap()).unwrap();
            assert_eq!(data["retirements"][0]["state"], "pending");
            assert_eq!(
                execute(&f.service, ri, retire(&f)).code,
                ResultCode::Ok,
                "M/I-only independent child recovery uses no credentials/advertisement"
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM context_retirement_absence_receipts",
                        [],
                        |r| r.get::<_, usize>(0)
                    )
                    .unwrap(),
                1
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM remote_poll_materializations",
                        [],
                        |r| r.get::<_, usize>(0)
                    )
                    .unwrap(),
                0
            );
            assert_eq!(
                serde_json::to_value(execute(&f.service, id, mutation()).data).unwrap(),
                data
            );
        }
        assert_eq!(
            execute(
                &f.service,
                id,
                Mutation::PollOnce(PollOnceInput {
                    root: f.root.path().into()
                })
            )
            .code,
            ResultCode::RequestMismatch
        );
    }
    // Terminal transport failures flow through the real public opted binding,
    // domain failure settlement, metadata cut and finished-request replay.
    use crate::repository::transport::{
        HostKeyIdentity, SshAuthority, SshDirection, SshTransportError, SshTransportErrorKind as E,
    };
    use crate::results::RecoveryActionKind as A;
    for case in [
        "key",
        "unlock",
        "host_replace",
        "host_verify",
        "host_approve",
        "lost_key",
        "lost_host",
    ] {
        let f = Fixture::new();
        let branch = present(&f);
        let id = RequestId::new();
        let lost = case.starts_with("lost_");
        let key = || HostKeyIdentity {
            algorithm: "ssh-ed25519".into(),
            sha256: "SHA256:safe-presented".into(),
        };
        let expected = HostKeyIdentity {
            algorithm: "ssh-ed25519".into(),
            sha256: "SHA256:safe-pinned".into(),
        };
        let (kind, code, category) = match case {
            "key" => (
                E::KeyRejected,
                ResultCode::KeyRejected,
                "selected_key_unavailable",
            ),
            "lost_key" => (
                E::KeyRejected,
                ResultCode::NoSelectedKey,
                "selected_key_unavailable",
            ),
            "unlock" => (E::UnlockFailed, ResultCode::UnlockFailed, "unlock_required"),
            "host_verify" => (
                E::HostVerificationUnavailable,
                ResultCode::RecoveryRequired,
                "host_approval_required",
            ),
            "host_approve" => (
                E::HostApprovalRequired { presented: key() },
                ResultCode::HostApprovalRequired,
                "host_approval_required",
            ),
            "lost_host" => (
                E::HostReplacementRequired {
                    expected,
                    presented: key(),
                },
                ResultCode::HostApprovalRequired,
                "host_approval_required",
            ),
            _ => (
                E::HostReplacementRequired {
                    expected,
                    presented: key(),
                },
                ResultCode::HostReplacementRequired,
                "host_approval_required",
            ),
        };
        let selected = SharedKeyId::new();
        let error = SshTransportError {
            root: f.root.path().into(),
            remote_name: "private-backend-remote-user-secret".into(),
            direction: SshDirection::Fetch,
            selected_key_id: Some(selected),
            authority: Some(SshAuthority {
                host: "safe-host.example".into(),
                port: 2222,
            }),
            kind,
        };
        let disk = disk_image(f.root.path());
        let producer = RepositoryService::open_at(f.data.path()).unwrap();
        if lost {
            let root = f.root.path().to_owned();
            let data = f.data.path().to_owned();
            producer.set_request_hook_for_testing(move || {
                let service = RepositoryService::open_at(&data).unwrap();
                let operation =
                    service.request_record(id).unwrap().unwrap().operations[0].operation_id;
                crate::repository::remote::synthetic_fetch_error(error);
                assert!(matches!(
                    service.poll_publication_remote(
                        PollPublicationRemoteRequest {
                            root,
                            operation_id: operation,
                            invocation: RemotePollInvocation::Explicit,
                            approval: None,
                            restart: false
                        },
                        &mut SessionCredentials::new(NoCredentials)
                    ),
                    Err(RemotePollError::Transport(_))
                ));
                panic!("lost original transport answer BEFORE the opted wrapper can compose a cut");
            });
        } else {
            crate::repository::remote::synthetic_fetch_error(error);
        }
        let mutation = || {
            Mutation::PollOnceRetiring(PollOnceInput {
                root: f.root.path().into(),
            })
        };
        let first = if lost {
            assert!(catch_unwind(AssertUnwindSafe(|| execute(&producer, id, mutation()))).is_err());
            assert!(
                f.db()
                    .query_row(
                        "SELECT summary IS NULL FROM poll_retirement_passes",
                        [],
                        |r| r.get::<_, bool>(0)
                    )
                    .unwrap()
            );
            execute(&f.service, id, mutation())
        } else {
            execute(&producer, id, mutation())
        };
        assert_eq!(
            first.code, code,
            "I1 precise original terminal transport answer, coarse only when actually lost: {case}: {first:?}"
        );
        assert_eq!(
            first.effects,
            crate::results::Effects::not_requested(),
            "I1 live stop must not claim a refresh: {case}"
        );
        assert_eq!(
            f.service.request_record(id).unwrap().unwrap().state,
            RequestState::Finished
        );
        let host = matches!(case, "host_replace" | "host_approve");
        let data = serde_json::to_value(&first.data).unwrap();
        if host {
            assert!(
                matches!(first.data, Some(MutationDataDto::Synchronization(_))),
                "reuse the existing poll host schema family"
            );
            assert_eq!(
                data.as_object().unwrap().len(),
                6,
                "existing closed synchronization host root only"
            );
            assert_eq!(data["host"], "safe-host.example");
            assert_eq!(data["port"], 2222);
            assert_eq!(data["presented_fingerprint"], "SHA256:safe-presented");
            assert_eq!(
                data["pinned_fingerprint"],
                if case == "host_replace" {
                    serde_json::json!("SHA256:safe-pinned")
                } else {
                    serde_json::Value::Null
                }
            );
            let action = first
                .recovery
                .iter()
                .find(|r| {
                    r.action
                        == if case == "host_replace" {
                            A::HostReplace
                        } else {
                            A::HostApprove
                        }
                })
                .expect("I1 safe host recovery route survives composition");
            assert_eq!(
                action.arguments.get("root"),
                Some(&serde_json::json!(f.root.path().to_str().unwrap()))
            );
            assert_eq!(
                action.arguments.get("authority"),
                Some(&serde_json::json!("safe-host.example:2222"))
            );
        } else {
            assert_eq!(data["type"], "poll_retirement");
            assert_eq!(data["stop_code"], code.as_str());
            assert!(
                !first
                    .recovery
                    .iter()
                    .any(|r| matches!(r.action, A::HostApprove | A::HostReplace)),
                "lost-original category cannot invent host authority"
            );
        }
        let op = first.operation_id.as_deref().unwrap();
        let db = f.db();
        let (summary, image, live): (String, Option<String>, bool) = db
            .query_row(
                "SELECT summary,image,live FROM poll_retirement_passes WHERE operation_ulid=?1",
                [op],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        let cut: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(cut["stop_code"], code.as_str());
        assert_eq!(cut["completion"], "stopped");
        assert_eq!(cut["retirements"], serde_json::json!([]));
        assert!(image.is_none() && !live);
        let (phase, outcome): (String, String) = db
            .query_row(
                "SELECT phase,outcome FROM remote_operation_records WHERE operation_ulid=?1",
                [op],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            (phase.as_str(), outcome.as_str()),
            ("failed", category),
            "real domain persists only the coarse category"
        );
        for table in [
            "poll_retirement_targets",
            "poll_retirement_child_sources",
            "context_retirement_records",
            "retirement_advertisement_provenance",
            "retirement_observation_receipts",
            "retirement_observation_pins",
            "ssh_host_pins",
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0,
                "{case}: no retirement/implicit approval: {table}"
            );
        }
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM request_records WHERE command='context retire'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert!(
            git2::Repository::open(f.root.path())
                .unwrap()
                .find_reference(&branch)
                .is_ok()
        );
        assert_eq!(
            disk,
            disk_image(f.root.path()),
            "no nonce, selection, cleanup, transport or approval disk effects"
        );
        let serialized = serde_json::to_string(&first).unwrap();
        for private in [
            "private-backend-remote-user-secret",
            "Private fixture title",
            "Private fixture body",
            &selected.to_string(),
        ] {
            assert!(
                !serialized.contains(private),
                "{case}: private transport evidence escaped"
            );
        }
        let decoded = if host {
            SynchronizationMutationDto::from_stored(&data).map(MutationDataDto::Synchronization)
        } else {
            PollRetirementMutationDto::from_stored(&data).map(MutationDataDto::PollRetirement)
        };
        assert_eq!(
            decoded, first.data,
            "existing stored family decoder round trip"
        );
        let registry = registry_image(&db);
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert_eq!(
            first,
            execute(&reopened, id, mutation()),
            "exact finished code/data/effects/recovery, not category fallback: {case}"
        );
        assert_eq!(registry, registry_image(&db));
        assert_eq!(disk, disk_image(f.root.path()));
        if host {
            for invalid in ["backend_error", "pinned_fingerprint"] {
                let mut bad = data.clone();
                bad[invalid] = if invalid == "backend_error" {
                    "private-backend-error".into()
                } else if case == "host_replace" {
                    serde_json::Value::Null
                } else {
                    "invented-pin".into()
                };
                db.execute(
                    "UPDATE request_records SET result_data=?2 WHERE request_ulid=?1",
                    params![id.to_string(), bad.to_string()],
                )
                .unwrap();
                assert!(
                    super::super::record::audit_connection(&db).is_err(),
                    "host-cut exception must stay closed and code/pin correlated"
                );
                db.execute(
                    "UPDATE request_records SET result_data=?2 WHERE request_ulid=?1",
                    params![id.to_string(), data.to_string()],
                )
                .unwrap();
            }
            super::super::record::audit_connection(&db).unwrap();
        }
    }
}

#[test]
fn retirement_batch_preselection_poll_recovery_does_not_free_its_id_for_other_journals() {
    let f = Fixture::new();
    present(&f);
    let git = git2::Repository::open(f.root.path()).unwrap();
    let primary = git.head().unwrap().target().unwrap();
    git.reference(
        "refs/remotes/origin/main",
        primary,
        false,
        "synthetic fetch",
    )
    .unwrap();
    let producer = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::PollBeforeDiscovery,
    )
    .unwrap();
    let id = RequestId::new();
    let mutation = || {
        Mutation::PollOnceRetiring(PollOnceInput {
            root: f.root.path().into(),
        })
    };
    crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
    let interrupted = execute(&producer, id, mutation());
    assert_ne!(interrupted.code, ResultCode::Ok);
    let p = OperationId::parse(interrupted.operation_id.as_deref().unwrap()).unwrap();
    assert!(super::super::record::owned_identity(&f.db(), &p.to_string()).unwrap());
    assert!(
        super::super::record::require_foreign_id_available(&f.db(), p).is_err(),
        "normal old-P recovery is not a generic foreign-journal ID exemption"
    );
    let config = f.root.path().join(".manyhands/config.toml");
    let text = std::fs::read(&config).unwrap();
    std::fs::write(&config, "invalid = [").unwrap();
    let before = registry_image(&f.db());
    assert!(
        f.service
            .observe_publication_remote(
                ObservePublicationRemoteRequest {
                    root: f.root.path().into(),
                    operation_id: p,
                    invocation: RemotePollInvocation::Explicit,
                    approval: None,
                    restart: true
                },
                &mut SessionCredentials::new(NoCredentials)
            )
            .is_err()
    );
    assert_eq!(
        before,
        registry_image(&f.db()),
        "public observer refuses before configuration writes"
    );
    std::fs::write(config, text).unwrap();
    // A second REGISTERED repository in the SAME application registry cannot
    // borrow A's preselection P, even before B's invalid-config attention writes.
    let other = Fixture::new();
    present(&other);
    f.service
        .enable(EnableRepositoryRequest {
            root: other.root.path().into(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    let b_id = f
        .service
        .resolve_repository(other.root.path())
        .unwrap()
        .registration_id();
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM repositories", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    let bconfig = other.root.path().join(".manyhands/config.toml");
    let btext = std::fs::read(&bconfig).unwrap();
    for valid in [false, true] {
        if !valid {
            std::fs::write(&bconfig, "invalid = [").unwrap();
        }
        let registry = registry_image(&f.db());
        let other_registry = registry_image(&other.db());
        let a_disk = disk_image(f.root.path());
        let b_disk = disk_image(other.root.path());
        let foreign = f.service.poll_publication_remote(
            PollPublicationRemoteRequest {
                root: other.root.path().into(),
                operation_id: p,
                invocation: RemotePollInvocation::Explicit,
                approval: None,
                restart: true,
            },
            &mut SessionCredentials::new(NoCredentials),
        );
        assert!(
            foreign.is_err(),
            "S2 foreign P is not B's UPDATE-P authority: {foreign:?}"
        );
        assert_eq!(
            registry,
            registry_image(&f.db()),
            "S2 foreign P must refuse BEFORE all B config/attention/reservation writes, valid={valid}"
        );
        assert_eq!(other_registry, registry_image(&other.db()));
        assert_eq!(a_disk, disk_image(f.root.path()));
        assert_eq!(b_disk, disk_image(other.root.path()));
        if valid {
            let plan = crate::repository::remote::observation::observation_plan(other.root.path())
                .unwrap();
            let target = RemoteOperationTarget::for_poll(&plan);
            assert!(
                f.service
                    .reserve_remote_poll(
                        other.root.path(),
                        p,
                        &target,
                        RemoteOperationPriority::Manual
                    )
                    .is_err(),
                "S2 valid B cannot reserve A's P"
            );
            assert!(
                crate::repository::promotion::record::validate_remote_identity(
                    &f.db(),
                    b_id,
                    p,
                    &target,
                    RemoteOperationPriority::Manual
                )
                .is_err(),
                "S2 common identity dispatcher also requires incoming scope/kind"
            );
            let a_id = f
                .service
                .resolve_repository(f.root.path())
                .unwrap()
                .registration_id();
            for repo in [a_id, b_id] {
                for kind in [
                    None,
                    Some(crate::repository::remote::state::PollKind::Observe),
                    Some(crate::repository::remote::state::PollKind::Update),
                ] {
                    for priority in [
                        RemoteOperationPriority::Manual,
                        RemoteOperationPriority::Poll,
                    ] {
                        let allowed = crate::repository::promotion::record::validate_poll_identity(
                            &f.db(),
                            repo,
                            p,
                            &target,
                            priority,
                            kind,
                        )
                        .is_ok();
                        assert_eq!(
                            allowed,
                            repo == a_id
                                && kind == Some(crate::repository::remote::state::PollKind::Update)
                                && priority == RemoteOperationPriority::Manual,
                            "S2 BOTH identity checks use exact incoming repository, UPDATE kind and manual priority"
                        );
                    }
                }
            }
            for root in [f.root.path(), other.root.path()] {
                assert!(
                    f.service
                        .poll_publication_remote(
                            PollPublicationRemoteRequest {
                                root: root.into(),
                                operation_id: p,
                                invocation: RemotePollInvocation::Automatic,
                                approval: None,
                                restart: true
                            },
                            &mut SessionCredentials::new(NoCredentials)
                        )
                        .is_err(),
                    "S2 automatic calls cannot borrow a manual preselection P"
                );
            }
            assert_eq!(registry, registry_image(&f.db()));
        }
        std::fs::write(&bconfig, &btext).unwrap();
    }
    crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
    let finished = execute(&f.service, id, mutation());
    assert_eq!(
        f.service.request_record(id).unwrap().unwrap().state,
        RequestState::Finished,
        "normal update-P reentry remains available: {finished:?}"
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    // The original report replay shortcut must use the incoming repository too,
    // rather than bypass the scoped ID checks once a later pass has captured.
    crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
    let captured = execute(&f.service, RequestId::new(), mutation());
    assert_eq!(
        captured.code,
        ResultCode::Ok,
        "own original-live pass remains valid: {captured:?}"
    );
    let captured_p = OperationId::parse(captured.operation_id.as_deref().unwrap()).unwrap();
    let registry = registry_image(&f.db());
    let a_disk = disk_image(f.root.path());
    let b_disk = disk_image(other.root.path());
    assert!(
        f.service
            .poll_publication_remote(
                PollPublicationRemoteRequest {
                    root: other.root.path().into(),
                    operation_id: captured_p,
                    invocation: RemotePollInvocation::Explicit,
                    approval: None,
                    restart: true
                },
                &mut SessionCredentials::new(NoCredentials)
            )
            .is_err(),
        "S2 foreign root cannot bypass scoped identity dispatch through captured-report replay"
    );
    assert_eq!(registry, registry_image(&f.db()));
    assert_eq!(a_disk, disk_image(f.root.path()));
    assert_eq!(b_disk, disk_image(other.root.path()));
}

#[test]
fn retirement_batch_reserved_request_ids_cannot_be_spent_by_public_or_atomic_bindings() {
    for policy in [false, true] {
        let f = Fixture::new();
        present(&f);
        let git = git2::Repository::open(f.root.path()).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        let root = f.root.path().to_owned();
        let data = f.data.path().join(REGISTRY_FILE);
        super::super::handoff::observe_eligible(move |s| {
            let db = Connection::open(data).unwrap();
            let ri: String = db
                .query_row(
                    "SELECT child_request_ulid FROM poll_retirement_targets",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let before = registry_image(&db);
            let mutation = if policy {
                Mutation::PollPause(PollPauseInput { root })
            } else {
                Mutation::ContextRetire(ContextRetireInput {
                    root,
                    kind: AuthoringKind::Ticket,
                    id: ITEM.parse().unwrap(),
                })
            };
            let refused = execute(s, RequestId::parse(&ri).unwrap(), mutation);
            assert_eq!(
                refused.code,
                ResultCode::RecoveryRequired,
                "reserved Ri requires the sealed ordinary initial binding, including atomic policy writes: {refused:?}"
            );
            assert_eq!(
                before,
                registry_image(&db),
                "public caller cannot poison a reserved child ID"
            );
        });
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        let result = execute(
            &f.service,
            RequestId::new(),
            Mutation::PollOnceRetiring(PollOnceInput {
                root: f.root.path().into(),
            }),
        );
        assert_eq!(
            result.code,
            ResultCode::Ok,
            "legitimate original child still accepts/settles: {result:?}"
        );
        let policy_id = RequestId::new();
        let policy = || {
            Mutation::PollPause(PollPauseInput {
                root: f.root.path().into(),
            })
        };
        let paused = execute(&f.service, policy_id, policy());
        assert_eq!(
            paused.code,
            ResultCode::Ok,
            "ordinary atomic bindings still work with unreserved IDs"
        );
        assert_eq!(execute(&f.service, policy_id, policy()), paused);
    }
}

#[test]
fn retirement_batch_shared_source_ordinary_children_settle_before_next_admission() {
    const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    for case in [
        "two_present",
        "branch_only",
        "dirty_first",
        "unmatched_first",
        "ordinary",
        "index_pending",
        "cancelled",
        "lost_all",
        "lost_prefix",
        "lost_all_mixed",
        "cancel_at_admission",
    ] {
        let f = Fixture::new();
        let first_branch = present(&f);
        let git = git2::Repository::open(f.root.path()).unwrap();
        let doc = format!("docs/{SECOND}.md");
        std::fs::create_dir_all(f.root.path().join("docs")).unwrap();
        std::fs::write(f.root.path().join(&doc),format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {SECOND}\ntitle: Second private fixture\n---\nPreserved content\n")).unwrap();
        let mut idx = git.index().unwrap();
        idx.add_path(Path::new(&doc)).unwrap();
        idx.write().unwrap();
        let tree = git.find_tree(idx.write_tree().unwrap()).unwrap();
        let parent = git.head().unwrap().peel_to_commit().unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        let primary = git
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "second primary",
                &tree,
                &[&parent],
            )
            .unwrap();
        let second_branch = format!("refs/heads/manyhands/document/{SECOND}");
        git.reference(&second_branch, primary, false, "fixture")
            .unwrap();
        if case != "branch_only" {
            let reference = git.find_reference(&second_branch).unwrap();
            git.worktree(
                SECOND,
                &f.root.path().join(".manyhands/worktrees").join(SECOND),
                Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
            )
            .unwrap();
        }
        // Sorted ordinary poll order is document first, then ticket.
        if matches!(case, "dirty_first" | "lost_all_mixed") {
            std::fs::write(
                f.root
                    .path()
                    .join(".manyhands/worktrees")
                    .join(SECOND)
                    .join("private dirt"),
                "never delete",
            )
            .unwrap();
        }
        if case == "unmatched_first" {
            let linked =
                git2::Repository::open(f.root.path().join(".manyhands/worktrees").join(ITEM))
                    .unwrap();
            std::fs::write(
                linked.workdir().unwrap().join("unique-unpublished"),
                "not on primary",
            )
            .unwrap();
            let mut i = linked.index().unwrap();
            i.add_path(Path::new("unique-unpublished")).unwrap();
            let t = linked.find_tree(i.write_tree().unwrap()).unwrap();
            let p = linked.head().unwrap().peel_to_commit().unwrap();
            linked
                .commit(Some("HEAD"), &sig, &sig, "unmatched", &t, &[&p])
                .unwrap();
        }
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        let plan = crate::repository::remote::observation::observation_plan(f.root.path()).unwrap();
        crate::repository::remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
            let refs = [
                (
                    first_branch.as_str(),
                    git.refname_to_id(&first_branch).unwrap(),
                ),
                (second_branch.as_str(), primary),
            ];
            let observations: Vec<_> = refs
                .into_iter()
                .map(|(name, oid)| {
                    RemoteRefObservation::from_advertisement(&plan, name, oid, Some(oid)).unwrap()
                })
                .collect();
            crate::repository::remote::state::persist_complete_advertisement(
                tx,
                id,
                &plan,
                crate::repository::remote::state::generation(tx, id)?,
                &observations,
                2,
            )?;
            Ok(())
        })
        .unwrap();
        let (materialization, _) = f.materialization("complete");
        f.db().execute("INSERT INTO remote_poll_materializations(repository_id,kind,item_id,operation_ulid,target_oid,step) SELECT id,'document',?1,?2,?3,'complete' FROM repositories",params![SECOND,materialization,primary.to_string()]).unwrap();
        struct Order {
            db: PathBuf,
            seen: usize,
        }
        impl ProgressSink for Order {
            fn event(&mut self, e: &ProgressEvent) {
                if e.command == "context retire" && e.stage == ProgressStage::Accepted {
                    let db = Connection::open(&self.db).unwrap();
                    assert_eq!(db.query_row("SELECT count(*) FROM context_retirement_records WHERE phase IN ('complete','refused')",[],|r|r.get::<_,usize>(0)).unwrap(),self.seen,"D12: previous child must settle before next acceptance");
                    assert_eq!(db.query_row("SELECT count(*) FROM request_records WHERE command='context retire' AND state='finished'",[],|r|r.get::<_,usize>(0)).unwrap(),self.seen);
                    self.seen += 1;
                }
            }
        }
        let mut order = Order {
            db: f.data.path().join(REGISTRY_FILE),
            seen: 0,
        };
        let request_id = RequestId::new();
        if case == "cancel_at_admission" {
            let data = f.data.path().to_owned();
            super::super::handoff::observe_eligible(move |s| {
                s.set_acceptance_hook_for_testing(move || {
                    assert_eq!(
                        RepositoryService::open_at(&data)
                            .unwrap()
                            .cancel_request(request_id)
                            .code,
                        ResultCode::Ok
                    );
                })
            });
        }
        if matches!(case, "lost_all" | "lost_prefix" | "lost_all_mixed") {
            let after = if case == "lost_prefix" { 0 } else { 1 };
            super::super::handoff::observe_children(move |ordinal| {
                if ordinal == after {
                    panic!("simulated loss after settled ordinary child, before parent copy");
                }
            });
        }
        if case == "index_pending" {
            super::super::record::fail_next_child_index();
        }
        if case == "cancelled" {
            let path = f.data.path().join(REGISTRY_FILE);
            RepositoryService::set_retirement_intent_hook_for_testing(move || {
                Connection::open(path)
                    .unwrap()
                    .execute(
                        "UPDATE request_records SET cancel_requested=1 WHERE request_ulid=?1",
                        [request_id.to_string()],
                    )
                    .unwrap();
            });
        }
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        let opted = case != "ordinary";
        let mutation = || {
            if opted {
                Mutation::PollOnceRetiring(PollOnceInput {
                    root: f.root.path().into(),
                })
            } else {
                Mutation::PollOnce(PollOnceInput {
                    root: f.root.path().into(),
                })
            }
        };
        let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f.service.execute(
                MutationCall {
                    request_id,
                    confirmation: None,
                    mutation: mutation(),
                },
                &mut SessionCredentials::new(NoCredentials),
                &mut order,
                &CancellationToken::new(),
            )
        }));
        let response = if matches!(case, "lost_all" | "lost_prefix" | "lost_all_mixed") {
            assert!(response.is_err());
            execute(&f.service, request_id, mutation())
        } else {
            response.unwrap()
        };
        if case == "cancel_at_admission" {
            assert_eq!(response.code, ResultCode::Cancelled, "{response:?}");
            assert_eq!(order.seen, 0);
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM poll_retirement_targets WHERE admission='reserved'",
                        [],
                        |r| r.get::<_, usize>(0)
                    )
                    .unwrap(),
                2,
                "stale coordinator must not rewrite exact not-reached membership after stop cut"
            );
            continue;
        }
        if case == "lost_prefix" {
            assert_eq!(
                response.code,
                ResultCode::RecoveryRequired,
                "settled prefix is not all selected"
            );
            let data = serde_json::to_value(&response.data).unwrap();
            assert_eq!(data["retirements"][0]["state"], "retired");
            assert_eq!(data["retirements"][1]["state"], "not_reached");
            assert_eq!(order.seen, 1);
            assert!(git.find_reference(&first_branch).is_ok());
            assert_eq!(
                serde_json::to_value(execute(&f.service, request_id, mutation()).data).unwrap(),
                data
            );
            continue;
        }
        if matches!(case, "index_pending" | "cancelled") {
            assert!(
                !matches!(response.code, ResultCode::Ok | ResultCode::AlreadyApplied),
                "{case}: {response:?}"
            );
            assert_eq!(
                response.outcome,
                crate::results::Outcome::Partial,
                "owned removals may not be hidden by parent cut: {response:?}"
            );
            let data = serde_json::to_value(&response.data).unwrap();
            assert_eq!(data["completion"], "stopped");
            assert_eq!(data["retirements"][0]["state"], "pending");
            assert_eq!(data["retirements"][1]["state"], "not_reached");
            assert_eq!(order.seen, 1);
            let ri =
                RequestId::parse(data["retirements"][0]["request_id"].as_str().unwrap()).unwrap();
            let ki = OperationId::parse(data["retirements"][0]["operation_id"].as_str().unwrap())
                .unwrap();
            for action in &response.recovery {
                assert_ne!(
                    action.operation_id.as_deref(),
                    response.operation_id.as_deref(),
                    "stopped parent owns no cleanup/index continuation"
                );
                assert!(
                    action.arguments.get("request_id").and_then(|v| v.as_str())
                        != Some(request_id.to_string().as_str()),
                    "old parent replay is not recovery work"
                );
            }
            if case == "index_pending" {
                assert!(
                    response.recovery.iter().any(|a| a.action
                        == crate::results::RecoveryActionKind::OperationResume
                        && a.operation_id.as_deref() == Some(ki.to_string().as_str())),
                    "offer the actual current child's I recovery: {response:?}"
                );
            }
            assert_eq!(
                f.service.request_record(request_id).unwrap().unwrap().state,
                RequestState::Finished
            );
            assert_eq!(
                f.service.request_record(ri).unwrap().unwrap().state,
                RequestState::Accepted
            );
            assert_eq!(
                serde_json::to_value(execute(&f.service, request_id, mutation()).data).unwrap(),
                data
            );
            if case == "index_pending" {
                let resumed = execute(
                    &f.service,
                    RequestId::new(),
                    Mutation::OperationResume(OperationResumeInput {
                        root: f.root.path().into(),
                        operation_id: ki,
                    }),
                );
                assert_eq!(
                    resumed.code,
                    ResultCode::Ok,
                    "independent child I after parent Final: {resumed:?}"
                );
                assert_eq!(
                    serde_json::to_value(execute(&f.service, request_id, mutation()).data).unwrap(),
                    data,
                    "old cut is immutable after independent recovery"
                );
            } else {
                let abandon = Mutation::OperationAbandon(OperationAbandonInput {
                    root: f.root.path().into(),
                    operation_id: ki,
                });
                let preview = f
                    .service
                    .prepare(&abandon)
                    .data
                    .unwrap()
                    .into_preview()
                    .unwrap();
                let kept = f.service.execute(
                    MutationCall {
                        request_id: RequestId::new(),
                        confirmation: Some(
                            ConfirmationId::parse(&preview.confirmation_id).unwrap(),
                        ),
                        mutation: abandon,
                    },
                    &mut SessionCredentials::new(NoCredentials),
                    &mut NoProgress,
                    &CancellationToken::new(),
                );
                assert_eq!(
                    kept.code,
                    ResultCode::Ok,
                    "independent child preserving exit after parent Final: {kept:?}"
                );
            }
            let replay = execute(&f.service, request_id, mutation());
            assert_eq!(serde_json::to_value(&replay.data).unwrap(), data);
            assert!(
                !replay.recovery.iter().any(|a| a.operation_id.as_deref()
                    == Some(ki.to_string().as_str())
                    || a.arguments.get("request_id").and_then(|v| v.as_str())
                        == Some(ri.to_string().as_str())),
                "settled child no longer offers recovery, frozen report is unchanged: {replay:?}"
            );
            assert!(
                git.find_reference(&first_branch).is_ok(),
                "later ticket was not reached"
            );
            crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
            let next = execute(&f.service, RequestId::new(), mutation());
            assert_eq!(
                next.code,
                ResultCode::Ok,
                "new explicit pass selects current remainder: {next:?}"
            );
            assert!(git.find_reference(&first_branch).is_err());
            assert_eq!(
                serde_json::to_value(execute(&f.service, request_id, mutation()).data).unwrap(),
                data
            );
            continue;
        }
        assert_eq!(response.code, ResultCode::Ok, "{case}: {response:?}");
        if !opted {
            assert_eq!(order.seen, 0);
            assert_eq!(
                f.db()
                    .query_row("SELECT count(*) FROM poll_retirement_passes", [], |r| r
                        .get::<_, usize>(
                        0
                    ))
                    .unwrap(),
                0
            );
            continue;
        }
        let data = serde_json::to_value(response.data).unwrap();
        assert_eq!(data["completion"], "completed", "{case}: {data}");
        assert_eq!(
            data["retirements"].as_array().unwrap().len(),
            2,
            "{case}: {data}"
        );
        let refusals = data["retirements"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["state"] == "refused")
            .count();
        assert_eq!(
            refusals,
            usize::from(matches!(
                case,
                "dirty_first" | "unmatched_first" | "lost_all_mixed"
            )),
            "{case}: {data}"
        );
        assert_eq!(order.seen, 2);
        let db = f.db();
        assert_eq!(db.query_row("SELECT count(*) FROM retirement_advertisement_provenance WHERE execution_kind='poll_parent'",[],|r|r.get::<_,usize>(0)).unwrap(),1,"exactly one source, not one per child");
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_operation_records WHERE poll_kind='observe'",
                [],
                |r| r.get::<_, usize>(0)
            )
            .unwrap(),
            0,
            "no child O or network observation"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM retirement_observation_pins",
                [],
                |r| r.get::<_, usize>(0)
            )
            .unwrap(),
            0
        );
        if refusals == 0 {
            assert_eq!(db.query_row("SELECT count(*) FROM retirement_observation_receipts WHERE consumer_kind='single'",[],|r|r.get::<_,usize>(0)).unwrap(),2,"K1 immutable receipt survives K2 binding same provenance");
        }
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, usize>(0)
            )
            .unwrap(),
            refusals,
            "D14 settles only each successful child's exact old M"
        );
        assert_eq!(git.head().unwrap().target(), Some(primary));
        assert_eq!(
            git.refname_to_id("refs/remotes/origin/main").unwrap(),
            primary
        );
        assert_eq!(
            serde_json::to_value(execute(&f.service, request_id, mutation()).data).unwrap(),
            data
        );
    }
}

#[test]
fn retirement_batch_public_live_pass_retires_real_context_and_replays_without_transport() {
    for lost in [false, true] {
        let f = Fixture::new();
        let branch = present(&f);
        let git = git2::Repository::open(f.root.path()).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        git.reference(
            "refs/remotes/origin/main",
            primary,
            false,
            "synthetic fetch",
        )
        .unwrap();
        let mutation = || {
            Mutation::PollOnceRetiring(PollOnceInput {
                root: f.root.path().into(),
            })
        };
        let id = RequestId::new();
        if lost {
            super::super::handoff::observe_children(|_| {
                panic!("loss after last child before parent copy")
            });
        }
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute(&f.service, id, mutation())
        }));
        if lost {
            assert!(first.is_err());
        } else {
            assert_eq!(first.unwrap().code, ResultCode::Ok);
        }
        // Replace the latest ordinary poll image before recovering the parent copy.
        // Its report comes from original owned capture, not today's projection.
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
        let later = execute(
            &f.service,
            RequestId::new(),
            Mutation::PollOnce(PollOnceInput {
                root: f.root.path().into(),
            }),
        );
        assert_eq!(later.code, ResultCode::Ok, "{later:?}");
        let first = execute(&f.service, id, mutation());
        assert_eq!(first.code, ResultCode::Ok, "{first:?}");
        let data = serde_json::to_value(&first.data).unwrap();
        assert_eq!(
            data["type"], "poll_retirement",
            "the opted public producer must not fall back to ordinary poll data"
        );
        assert_eq!(data["completion"], "completed");
        assert_eq!(data["retirements"][0]["state"], "retired");
        assert!(git.find_reference(&branch).is_err());
        assert_ne!(first.operation_id, later.operation_id);
        assert_eq!(
            data["poll"]["operation_id"],
            first.operation_id.as_deref().unwrap()
        );
        // Recreated same-OID resources and a later complete M are a NEW lifetime.
        git.reference(&branch, primary, false, "new lifetime")
            .unwrap();
        let reference = git.find_reference(&branch).unwrap();
        git.worktree(
            ITEM,
            &f.root.path().join(".manyhands/worktrees").join(ITEM),
            Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
        )
        .unwrap();
        f.materialization("complete");
        let disk = disk_image(f.root.path());
        let materializations = row_image(&f.db(), "remote_poll_materializations");
        assert_eq!(
            serde_json::to_value(execute(&f.service, id, mutation()).data).unwrap(),
            data
        );
        assert_eq!(disk, disk_image(f.root.path()));
        assert_eq!(
            materializations,
            row_image(&f.db(), "remote_poll_materializations")
        );
        let requests = row_image(&f.db(), "request_records");
        let maps = row_image(&f.db(), "request_operations");
        assert_eq!(
            f.service
                .remove_registration(RemoveRegistrationRequest {
                    root: f.root.path().into(),
                    operation_id: OperationId::new()
                })
                .unwrap(),
            RemoveRegistrationOutcome::Removed
        );
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert_eq!(
            serde_json::to_value(execute(&reopened, id, mutation()).data).unwrap(),
            data,
            "finished report remains readable after terminal deregistration"
        );
        assert_eq!(requests, row_image(&f.db(), "request_records"));
        assert_eq!(maps, row_image(&f.db(), "request_operations"));
        assert_eq!(disk, disk_image(f.root.path()));
        assert!(
            !f.db()
                .prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap()
        );
    }
}

#[test]
fn ordinary_retirement_partial_cancel_has_truthful_flags_and_preserving_escape() {
    let f = Fixture::new();
    let branch = present(&f);
    let id = RequestId::new();
    let cancel = CancellationToken::new();
    let other = cancel.clone();
    RepositoryService::set_retirement_intent_hook_for_testing(move || other.cancel());
    super::super::run::synthetic_advertisement(Ok(Vec::new()));
    let stopped = f.service.execute(
        MutationCall {
            request_id: id,
            confirmation: None,
            mutation: retire(&f),
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &cancel,
    );
    assert_eq!(stopped.code, ResultCode::Cancelled, "{stopped:?}");
    assert_eq!(stopped.outcome, crate::results::Outcome::Partial);
    let data = serde_json::to_value(&stopped.data).unwrap();
    assert_eq!(data["worktree_removal"], "observed");
    assert_eq!(data["branch_removal"], "none");
    assert!(
        !f.root
            .path()
            .join(".manyhands/worktrees")
            .join(ITEM)
            .exists()
    );
    let operation = OperationId::parse(stopped.operation_id.as_deref().unwrap()).unwrap();
    let original = execute(
        &f.service,
        RequestId::new(),
        Mutation::OperationResume(OperationResumeInput {
            root: f.root.path().into(),
            operation_id: operation,
        }),
    );
    assert_eq!(original.code, ResultCode::OriginalRequestRequired);
    let mutation = || {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: f.root.path().into(),
            operation_id: operation,
        })
    };
    let preview = f
        .service
        .prepare(&mutation())
        .data
        .unwrap()
        .into_preview()
        .unwrap();
    let kept = f.service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: Some(ConfirmationId::parse(&preview.confirmation_id).unwrap()),
            mutation: mutation(),
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(kept.code, ResultCode::Ok, "{kept:?}");
    assert!(
        git2::Repository::open(f.root.path())
            .unwrap()
            .find_reference(&branch)
            .is_ok()
    );
    assert_eq!(
        execute(&f.service, id, retire(&f)).code,
        ResultCode::ExternalChange
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM retirement_observation_pins",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(f.db().query_row("SELECT count(*) FROM retirement_observation_pin_releases WHERE reason='consumer_terminal'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
}

#[test]
fn ordinary_retirement_local_q1_matrix_is_stable_private_and_never_implicitly_approves() {
    use crate::repository::transport::HostKeyIdentity;
    use crate::repository::transport::SshTransportErrorKind as E;
    for case in ["retired", "dirty", "before_w", "key", "host"] {
        let f = Fixture::new();
        let branch = present(&f);
        let id = RequestId::new();
        let cancel = CancellationToken::new();
        let expected = match case {
            "dirty" => {
                std::fs::write(
                    f.root
                        .path()
                        .join(".manyhands/worktrees")
                        .join(ITEM)
                        .join("private-untracked"),
                    "private dirty source",
                )
                .unwrap();
                ResultCode::WorktreeNotClean
            }
            "before_w" => {
                let other = cancel.clone();
                RepositoryService::set_retirement_worktree_hook_for_testing(move || other.cancel());
                ResultCode::Cancelled
            }
            "key" => ResultCode::NoSelectedKey,
            "host" => ResultCode::HostApprovalRequired,
            _ => ResultCode::Ok,
        };
        let advertisement = match case {
            "key" => Err(E::NoSelectedKey),
            "host" => Err(E::HostApprovalRequired {
                presented: HostKeyIdentity {
                    algorithm: "synthetic-private-algorithm".into(),
                    sha256: "synthetic-private-fingerprint".into(),
                },
            }),
            _ => Ok(Vec::new()),
        };
        if case != "dirty" {
            super::super::run::synthetic_advertisement(advertisement);
        }
        let first = f.service.execute(
            MutationCall {
                request_id: id,
                confirmation: None,
                mutation: retire(&f),
            },
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &cancel,
        );
        assert_eq!(first.code, expected, "{case}: {first:?}");
        assert_eq!(
            f.service.show_request(id).data.unwrap().state,
            RequestState::Finished,
            "{case}"
        );
        let data = serde_json::to_value(&first.data).unwrap();
        assert_eq!(
            data["state"],
            if case == "retired" {
                "retired"
            } else {
                "refused"
            }
        );
        assert_eq!(
            data["worktree_removal"],
            if case == "retired" {
                "observed"
            } else {
                "none"
            }
        );
        assert_eq!(
            data["branch_removal"],
            if case == "retired" {
                "observed"
            } else {
                "none"
            }
        );
        let json = serde_json::to_string(&first).unwrap();
        for private in [
            "Private fixture title",
            "Private fixture body",
            "private dirty source",
            "synthetic-private-algorithm",
            "synthetic-private-fingerprint",
        ] {
            assert!(!json.contains(private), "{case}: private evidence escaped");
        }
        let git = git2::Repository::open(f.root.path()).unwrap();
        if case == "retired" {
            assert!(git.find_reference(&branch).is_err());
            git.reference(
                &branch,
                git.head().unwrap().target().unwrap(),
                false,
                "new context",
            )
            .unwrap();
        } else {
            assert!(git.find_reference(&branch).is_ok());
        }
        let before = disk_image(f.root.path());
        let replay = execute(&f.service, id, retire(&f));
        assert_eq!(replay.code, expected);
        assert_eq!(first.data, replay.data);
        assert_eq!(
            before,
            disk_image(f.root.path()),
            "{case}: final replay cannot inspect/reselect/delete"
        );
    }
}

#[test]
fn ordinary_retirement_cancellation_after_acceptance_stops_before_auth_and_is_final() {
    for token in [true, false] {
        let f = Fixture::new();
        let branch = branch(&f);
        let id = RequestId::new();
        let cancel = CancellationToken::new();
        let other = cancel.clone();
        let data = f.data.path().to_owned();
        RepositoryService::set_retirement_selected_hook_for_testing(move || {
            if token {
                other.cancel();
            } else {
                let result = RepositoryService::open_at(&data)
                    .unwrap()
                    .cancel_request(id);
                assert!(result.data.unwrap().requested);
            }
        });
        let stopped = f.service.execute(
            MutationCall {
                request_id: id,
                confirmation: None,
                mutation: retire(&f),
            },
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &cancel,
        );
        assert_eq!(stopped.code, ResultCode::Cancelled, "{stopped:?}");
        assert_eq!(
            f.service.show_request(id).data.unwrap().state,
            RequestState::Finished
        );
        assert!(
            git2::Repository::open(f.root.path())
                .unwrap()
                .find_reference(&branch)
                .is_ok()
        );
        assert_eq!(
            execute(&f.service, id, retire(&f)).code,
            ResultCode::Cancelled
        );
    }
}

#[test]
fn ordinary_retirement_final_and_lost_result_replay_preserve_new_context_and_mismatches() {
    for lost in [false, true] {
        let f = Fixture::new();
        let id = RequestId::new();
        if lost {
            // End after the local result exists but before boundary settlement.
            f.service.set_request_hook_for_testing(|| {});
            let producer = RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::BeforeRequestSettlement,
            )
            .unwrap();
            assert_eq!(
                execute(&producer, id, retire(&f)).code,
                ResultCode::InternalError
            );
        } else {
            assert_eq!(
                execute(&f.service, id, retire(&f)).code,
                ResultCode::AlreadyApplied
            );
        }
        let git = git2::Repository::open(f.root.path()).unwrap();
        let tip = git.head().unwrap().target().unwrap();
        let branch = format!("refs/heads/manyhands/ticket/{ITEM}");
        git.reference(&branch, tip, false, "new lifetime").unwrap();
        let before = disk_image(f.root.path());
        let replay = execute(&f.service, id, retire(&f));
        assert_eq!(replay.code, ResultCode::AlreadyApplied, "{replay:?}");
        assert_eq!(before, disk_image(f.root.path()));
        assert_eq!(git.refname_to_id(&branch).unwrap(), tip);
        for (kind, item) in [
            (AuthoringKind::Document, ITEM),
            (AuthoringKind::Ticket, "01ARZ3NDEKTSV4RRFFQ69G5FAW"),
        ] {
            assert_eq!(
                execute(
                    &f.service,
                    id,
                    Mutation::ContextRetire(ContextRetireInput {
                        root: f.root.path().into(),
                        kind,
                        id: item.parse().unwrap()
                    })
                )
                .code,
                ResultCode::RequestMismatch
            );
        }
        assert_eq!(
            execute(
                &f.service,
                id,
                Mutation::ContextRelease(ContextReleaseInput {
                    root: f.root.path().into(),
                    kind: AuthoringKind::Ticket,
                    id: ITEM.parse().unwrap()
                })
            )
            .code,
            ResultCode::RequestMismatch
        );
        assert_eq!(f.db().query_row("SELECT count(*) FROM context_retirement_records WHERE batch_operation_ulid IS NOT NULL",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}

#[test]
fn ordinary_retirement_accepted_but_never_claimed_settles_without_selection_or_observation() {
    for cancelled in [false, true] {
        let f = Fixture::new();
        let id = RequestId::new();
        let producer = RepositoryService::open_at_with_failure_point_for_testing(
            f.data.path(),
            FailurePoint::BeforeRequestDomainCall,
        )
        .unwrap();
        assert_eq!(
            execute(&producer, id, retire(&f)).code,
            ResultCode::InternalError
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let branch = present(&f); // Today's context is NOT that old request's selection.
        if cancelled {
            assert!(f.service.cancel_request(id).data.unwrap().requested);
        }
        let before = disk_image(f.root.path());
        let replay = execute(&f.service, id, retire(&f));
        assert_eq!(
            replay.code,
            if cancelled {
                ResultCode::Cancelled
            } else {
                ResultCode::ExternalChange
            },
            "{replay:?}"
        );
        assert_eq!(
            replay.outcome,
            if cancelled {
                crate::results::Outcome::Cancelled
            } else {
                crate::results::Outcome::Blocked
            }
        );
        assert_eq!(before, disk_image(f.root.path()));
        assert!(
            git2::Repository::open(f.root.path())
                .unwrap()
                .find_reference(&branch)
                .is_ok()
        );
        assert_eq!(
            f.service.show_request(id).data.unwrap().state,
            RequestState::Finished
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
        assert_eq!(f.db().query_row("SELECT count(*) FROM context_retirement_records WHERE initial_resources='invalid' AND materialization_capture='unverified' AND phase='refused'", [], |r|r.get::<_, i64>(0)).unwrap(), 1);
    }
}

#[test]
fn ordinary_retirement_index_only_resume_spends_own_request_and_preserves_recreation() {
    let f = Fixture::new();
    let id = RequestId::new();
    f.db().execute_batch("CREATE TRIGGER retirement_fixture_fail_final BEFORE UPDATE OF phase ON context_retirement_records WHEN NEW.phase='complete' BEGIN SELECT RAISE(ABORT,'isolated fault'); END").unwrap();
    let first = execute(&f.service, id, retire(&f));
    assert_eq!(first.code, ResultCode::DiscoveryPending, "{first:?}");
    let operation = OperationId::parse(first.operation_id.as_deref().unwrap()).unwrap();
    f.db()
        .execute_batch("DROP TRIGGER retirement_fixture_fail_final")
        .unwrap();
    let git = git2::Repository::open(f.root.path()).unwrap();
    let tip = git.head().unwrap().target().unwrap();
    let branch = format!("refs/heads/manyhands/ticket/{ITEM}");
    git.reference(&branch, tip, false, "new lifetime").unwrap();
    let result = execute(
        &f.service,
        RequestId::new(),
        Mutation::OperationResume(OperationResumeInput {
            root: f.root.path().into(),
            operation_id: operation,
        }),
    );
    assert_eq!(
        result.code,
        ResultCode::Ok,
        "ordinary I recovery must not use generic unfenced refresh: {result:?}"
    );
    assert_eq!(result.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(git.refname_to_id(&branch).unwrap(), tip);
    let old = execute(&f.service, id, retire(&f));
    assert_eq!(old.code, ResultCode::AlreadyApplied);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM context_retirement_index_claims WHERE operation_ulid=?1",
                [operation.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );

    // The other supported exit from initial-absence I is confirmed preserving
    // abandonment. Its terminal local metadata must not prevent deregistration.
    let f = Fixture::new();
    let id = RequestId::new();
    let producer = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeIndexTransactionCommit,
    )
    .unwrap();
    let pending = execute(&producer, id, retire(&f));
    assert_eq!(pending.code, ResultCode::DiscoveryPending, "{pending:?}");
    let operation = OperationId::parse(pending.operation_id.as_deref().unwrap()).unwrap();
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    let abandon = RequestId::new();
    // The public boundary must keep accepting a retirement root alias while
    // binding metadata/ownership to the already-canonical scope.
    let mutation = || {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: f.root.path().join("."),
            operation_id: operation,
        })
    };
    let preview = f
        .service
        .prepare(&mutation())
        .data
        .unwrap()
        .into_preview()
        .unwrap();
    let call = || MutationCall {
        request_id: abandon,
        confirmation: Some(ConfirmationId::parse(&preview.confirmation_id).unwrap()),
        mutation: mutation(),
    };
    f.db().execute_batch("CREATE TRIGGER retirement_fixture_fail_preserved BEFORE UPDATE OF phase ON context_retirement_records WHEN NEW.phase='abandoned' BEGIN SELECT RAISE(ABORT,'isolated finalizer fault'); END").unwrap();
    let pending = f.service.execute(
        call(),
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(pending.code, ResultCode::DiscoveryPending, "{pending:?}");
    let row = f.service.request_record(abandon).unwrap().unwrap();
    let accepted = mutation::Accepted::of_record(&row, row.attempt).unwrap();
    assert!(
        context_retirement::preserve::owns(&f.service, f.root.path(), operation, &accepted)
            .unwrap()
    );
    let wrong_root = f.root.path().join("unrelated");
    assert_eq!(
        context_retirement::preserve::owns(&f.service, &wrong_root, operation, &accepted)
            .unwrap_err()
            .code(),
        ResultCode::RecoveryRequired
    );
    let stale = mutation::Accepted::of_record(&row, row.attempt + 1).unwrap();
    assert_eq!(
        context_retirement::preserve::owns(&f.service, f.root.path(), operation, &stale)
            .unwrap_err()
            .code(),
        ResultCode::RecoveryRequired
    );
    let before = registry_image(&f.db());
    let refused = f
        .service
        .remove_registration(RemoveRegistrationRequest {
            root: f.root.path().into(),
            operation_id: OperationId::new(),
        })
        .unwrap_err();
    assert_eq!(refused.kind, RepositoryErrorKind::RecoveryRequired);
    for table in [
        "context_retirement_preservations",
        "context_retirement_preservation_claims",
    ] {
        assert!(
            f.db().execute(&format!("DELETE FROM {table}"), []).is_err(),
            "pending {table} is retained"
        );
        assert!(
            f.db()
                .execute(
                    &format!("UPDATE {table} SET operation_ulid=operation_ulid"),
                    []
                )
                .is_err(),
            "pending {table} stays immutable"
        );
    }
    assert_eq!(
        before,
        registry_image(&f.db()),
        "pending preserving debt blocks removal unchanged"
    );
    f.db()
        .execute_batch("DROP TRIGGER retirement_fixture_fail_preserved")
        .unwrap();
    let abandoned = f.service.execute(
        call(),
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(abandoned.code, ResultCode::Ok, "{abandoned:?}");
    let retired = execute(&f.service, id, retire(&f));
    assert_eq!(retired.code, ResultCode::ExternalChange);
    for table in [
        "context_retirement_preservations",
        "context_retirement_preservation_claims",
    ] {
        assert!(
            f.db()
                .execute(
                    &format!("UPDATE {table} SET operation_ulid=operation_ulid"),
                    []
                )
                .is_err(),
            "terminal {table} stays immutable until cleanup"
        );
    }
    let requests = row_image(&f.db(), "request_records");
    let mappings = row_image(&f.db(), "request_operations");
    let disk = disk_image(f.root.path());
    assert_eq!(
        f.service
            .remove_registration(RemoveRegistrationRequest {
                root: f.root.path().into(),
                operation_id: OperationId::new()
            })
            .unwrap(),
        RemoveRegistrationOutcome::Removed
    );
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(execute(&reopened, id, retire(&f)), retired);
    assert_eq!(
        reopened.execute(
            call(),
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &CancellationToken::new()
        ),
        abandoned
    );
    assert_eq!(requests, row_image(&f.db(), "request_records"));
    assert_eq!(mappings, row_image(&f.db(), "request_operations"));
    assert_eq!(disk, disk_image(f.root.path()));
    for table in [
        "repositories",
        "operation_records",
        "context_retirement_records",
        "context_retirement_preservations",
        "context_retirement_preservation_claims",
    ] {
        assert_eq!(
            f.db()
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0,
            "terminal {table} follows registration cleanup"
        );
    }
    assert!(
        !f.db()
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );
}
