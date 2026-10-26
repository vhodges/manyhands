use super::{
    record::LocalSyncState,
    record_tests::{Fixture, commit},
    run::{self, Reason},
};
use crate::{repository::*, results::*};
use std::{fs, path::Path};

fn primary(f: &Fixture) -> git2::Repository {
    git2::Repository::open(f.root.path()).unwrap()
}
fn tip(g: &git2::Repository) -> git2::Oid {
    g.head().unwrap().target().unwrap()
}
fn graph(f: &Fixture, shape: &str) -> (git2::Oid, git2::Oid) {
    let p = primary(f);
    let c = tip(&f.context());
    if matches!(shape, "equal" | "fast_forward") {
        p.reference("refs/heads/main", c, true, "fixture").unwrap();
        p.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
    }
    if matches!(shape, "fast_forward" | "merge") {
        fs::write(f.root.path().join("incoming"), "committed primary\n").unwrap();
        commit(&p);
    }
    let incoming = tip(&p);
    p.reference("refs/remotes/origin/main", incoming, true, "fixture")
        .unwrap();
    (c, incoming)
}
fn execute(f: &Fixture, id: OperationId, observation: &str) -> run::Report {
    run::execute(&f.service, &f.repo, f.kind, &f.item, id, observation)
}
fn faults(f: &Fixture, id: OperationId, observation: &str, point: FailurePoint) -> run::Report {
    let service =
        RepositoryService::open_at_with_failure_point_for_testing(f.data.path(), point).unwrap();
    run::execute(&service, &f.repo, f.kind, &f.item, id, observation)
}
fn no_owner(f: &Fixture, id: OperationId) {
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
fn refs_unchanged(f: &Fixture, incoming: git2::Oid) {
    let p = primary(f);
    assert_eq!(tip(&p), incoming);
    assert_eq!(
        p.refname_to_id("refs/remotes/origin/main").unwrap(),
        incoming
    );
}
fn reflog_len(f: &Fixture) -> usize {
    let c = f.context();
    c.reflog(c.head().unwrap().name().unwrap()).unwrap().len()
}

#[test]
fn local_sync_required_outcomes_graphs_both_kinds() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["equal", "contained", "fast_forward", "merge"] {
            let f = Fixture::new(kind);
            let (context, incoming) = graph(&f, shape);
            let before = f.image();
            let logs = reflog_len(&f);
            let observation = f.observe();
            let id = OperationId::new();
            let result = execute(&f, id, &observation.observation);
            let noop = matches!(shape, "equal" | "contained");
            assert_eq!(
                result.code,
                if noop {
                    ResultCode::AlreadyApplied
                } else {
                    ResultCode::Ok
                },
                "{shape}: {result:?}"
            );
            assert_eq!(result.state, Some(LocalSyncState::Complete));
            assert_eq!(result.effects.publication, PublicationEffect::NotRequested);
            assert_eq!(result.effects.cleanup, CleanupEffect::NotRequested);
            assert_eq!(result.effects.discovery, DiscoveryEffect::Current);
            if noop {
                assert_eq!(before, f.image());
                assert_eq!(result.effects.write, WriteEffect::Unchanged);
                assert_eq!(result.effects.integration, IntegrationEffect::NotRequested);
                assert_eq!(result.effects.commit_oid, None);
            } else {
                assert_eq!(result.effects.write, WriteEffect::Written);
                assert_eq!(result.effects.integration, IntegrationEffect::Complete);
                assert_eq!(
                    result.effects.commit_oid,
                    Some(tip(&f.context()).to_string())
                );
                assert_eq!(
                    fs::read(f.context().workdir().unwrap().join("incoming")).unwrap(),
                    b"committed primary\n"
                );
            }
            assert_eq!(
                result.effects.checkpoint,
                if shape == "merge" {
                    CheckpointEffect::Committed
                } else {
                    CheckpointEffect::Unchanged
                }
            );
            if shape == "merge" {
                let c = f.context();
                let result = c.find_commit(tip(&c)).unwrap();
                assert_eq!(result.parent_ids().collect::<Vec<_>>(), [context, incoming]);
            }
            if shape == "fast_forward" {
                assert_eq!(tip(&f.context()), incoming);
            }
            assert_eq!(reflog_len(&f), logs + usize::from(!noop));
            let after = f.image();
            assert_eq!(execute(&f, id, &observation.observation), result);
            assert_eq!(after, f.image());
            assert_eq!(reflog_len(&f), logs + usize::from(!noop));
            refs_unchanged(&f, incoming);
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            assert!(row.integration_proven && row.artifacts_released && row.index_complete);
            assert_eq!(
                (row.context_parent, row.primary_parent),
                (context, incoming)
            );
        }
    }
}

#[test]
fn local_sync_required_outcomes_stale_and_frozen_parent_retry() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let (_, incoming) = graph(&f, "merge");
        let observation = f.observe();
        let id = OperationId::new();
        let stopped = faults(
            &f,
            id,
            &observation.observation,
            FailurePoint::LocalSyncAfterIntent,
        );
        assert_ne!(stopped.code, ResultCode::Ok);
        fs::write(f.root.path().join("later"), "newer primary").unwrap();
        commit(&primary(&f));
        let fresh_id = OperationId::new();
        let before = f.image();
        assert_eq!(
            execute(&f, fresh_id, &observation.observation).code,
            ResultCode::ExternalChange
        );
        no_owner(&f, fresh_id);
        assert_eq!(before, f.image());
        let resumed = execute(&f, id, &observation.observation);
        assert_eq!(resumed.code, ResultCode::Ok, "{resumed:?}");
        let c = f.context();
        let merged = c.find_commit(tip(&c)).unwrap();
        assert_eq!(merged.parent_id(1).unwrap(), incoming);
        assert!(!c.workdir().unwrap().join("later").exists());
    }
}

#[test]
fn local_sync_required_outcomes_fault_matrix() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge"] {
            for point in [
                FailurePoint::LocalSyncAfterIntent,
                FailurePoint::LocalSyncAfterCandidate,
                FailurePoint::LocalSyncAfterCheckout,
                FailurePoint::LocalSyncAfterRef,
                FailurePoint::LocalSyncBeforeIndex,
                FailurePoint::LocalSyncAfterIndex,
            ] {
                let f = Fixture::new(kind);
                let (context, incoming) = graph(&f, shape);
                let observation = f.observe();
                let id = OperationId::new();
                let logs = reflog_len(&f);
                let stopped = faults(&f, id, &observation.observation, point);
                assert_ne!(stopped.code, ResultCode::Ok, "{point:?}");
                if point == FailurePoint::LocalSyncBeforeIndex {
                    assert_eq!(stopped.code, ResultCode::DiscoveryPending);
                    assert_eq!(stopped.reason, Reason::IndexPending);
                    assert_eq!(stopped.effects.integration, IntegrationEffect::Complete);
                    assert_eq!(stopped.outcome(), Outcome::Partial);
                }
                let retained = f
                    .service
                    .read_local_context_sync_record(&f.repo, id)
                    .unwrap();
                let reopened = RepositoryService::open_at(f.data.path()).unwrap();
                let result = run::execute(
                    &reopened,
                    &f.repo,
                    kind,
                    &f.item,
                    id,
                    &observation.observation,
                );
                assert_eq!(result.code, ResultCode::Ok, "{shape} {point:?}: {result:?}");
                if let Some(candidate) = retained.candidate {
                    assert_eq!(tip(&f.context()), candidate);
                }
                assert_eq!(reflog_len(&f), logs + 1);
                let row = reopened
                    .read_local_context_sync_record(&f.repo, id)
                    .unwrap();
                assert_eq!(
                    (row.context_parent, row.primary_parent),
                    (context, incoming)
                );
                refs_unchanged(&f, incoming);
                let after = f.image();
                assert_eq!(
                    execute(&f, id, &observation.observation).code,
                    ResultCode::Ok
                );
                assert_eq!(after, f.image());
            }
        }
    }
}

#[test]
fn unrelated_histories_refuse_before_acceptance() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let p = primary(&f);
        let tree = p.find_commit(tip(&p)).unwrap().tree().unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        let orphan = p.commit(None, &sig, &sig, "orphan", &tree, &[]).unwrap();
        p.reference("refs/heads/main", orphan, true, "fixture")
            .unwrap();
        let observation = f.observe();
        let id = OperationId::new();
        let before = f.image();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.code, ResultCode::RecoveryRequired);
        assert_eq!(result.reason, Reason::UnrelatedHistories);
        assert!(!result.effects.is_durable());
        no_owner(&f, id);
        assert_eq!(before, f.image());
    }
}

#[test]
fn identity_missing_then_set_retries_the_same_local_intent() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for boundary in [
            None,
            Some(FailurePoint::LocalSyncAfterIntent),
            Some(FailurePoint::LocalSyncAfterCandidate),
        ] {
            let f = Fixture::new(kind);
            graph(&f, "merge");
            let observation = f.observe();
            let id = OperationId::new();
            if let Some(point) = boundary {
                faults(&f, id, &observation.observation, point);
            }
            let c = f.context();
            let mut config = c.config().unwrap();
            config.set_str("user.name", "").unwrap();
            config.set_str("user.email", "").unwrap();
            let result = execute(&f, id, &observation.observation);
            if boundary == Some(FailurePoint::LocalSyncAfterCandidate) {
                assert_eq!(result.code, ResultCode::Ok, "{result:?}");
                assert_eq!(
                    c.find_commit(tip(&c)).unwrap().author().name(),
                    Some("Fixture")
                );
            } else {
                assert_eq!(result.code, ResultCode::IdentityRequired, "{result:?}");
                assert_eq!(result.reason, Reason::IdentityRequired);
                if boundary.is_none() {
                    no_owner(&f, id);
                } else {
                    assert_eq!(
                        f.service
                            .read_local_context_sync_record(&f.repo, id)
                            .unwrap()
                            .state,
                        LocalSyncState::Prepared
                    );
                }
                config.set_str("user.name", "Replacement").unwrap();
                config
                    .set_str("user.email", "replacement@example.invalid")
                    .unwrap();
                assert_eq!(
                    execute(&f, id, &observation.observation).code,
                    ResultCode::Ok
                );
                assert_eq!(
                    c.find_commit(tip(&c)).unwrap().author().name(),
                    Some("Replacement")
                );
            }
        }
    }
}

#[test]
fn immutable_closure_disagreement_never_fences_the_repository() {
    let f = Fixture::new(AuthoringKind::Ticket);
    graph(&f, "equal");
    let path = format!(".manyhands/tickets/{}/ticket.md", f.item);
    for (g, who) in [(f.context(), "One"), (primary(&f), "Two")] {
        let file = g.workdir().unwrap().join(&path);
        let source = fs::read_to_string(&file).unwrap();
        let source = source.replacen(
            "---\n",
            &format!("---\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: {who}\n"),
            1,
        );
        crate::canonical::parse_item(Path::new(&path), &source).unwrap();
        fs::write(file, source).unwrap();
        commit(&g);
    }
    let observation = f.observe();
    let id = OperationId::new();
    let before = f.image();
    let result = execute(&f, id, &observation.observation);
    assert_eq!(result.code, ResultCode::RecoveryRequired);
    assert_eq!(result.reason, Reason::ImmutableClosureDisagreement);
    no_owner(&f, id);
    assert_eq!(before, f.image());
    assert!(matches!(
        f.service
            .refresh_repository(RefreshRepositoryRequest {
                root: f.root.path().into(),
                operation_id: OperationId::new()
            })
            .unwrap(),
        RefreshOutcome::Refreshed { .. }
    ));
}

#[test]
fn local_sync_required_outcomes_foreign_change_preserves_historical_effects() {
    for point in [
        FailurePoint::LocalSyncAfterIntent,
        FailurePoint::LocalSyncAfterCheckout,
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let observation = f.observe();
        let id = OperationId::new();
        faults(&f, id, &observation.observation, point);
        fs::write(
            f.context().workdir().unwrap().join("ordinary"),
            "foreign edit",
        )
        .unwrap();
        let before = f.image();
        let result = execute(&f, id, &observation.observation);
        assert!(matches!(
            result.code,
            ResultCode::ExternalChange | ResultCode::RecoveryRequired
        ));
        assert_eq!(before, f.image());
        assert_eq!(
            result.effects.is_durable(),
            point == FailurePoint::LocalSyncAfterCheckout
        );
        if result.effects.is_durable() {
            assert_eq!(result.outcome(), Outcome::Partial);
        }
        assert!(
            f.service
                .read_local_context_sync_record(&f.repo, id)
                .is_ok()
        );
    }
}

#[test]
fn local_sync_dirty_targets_preserve_tracked_untracked_and_ignored_files() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for dirty in ["ordinary", "untracked", "incoming"] {
            let f = Fixture::new(kind);
            if dirty == "incoming" {
                fs::write(
                    f.context().workdir().unwrap().join(".gitignore"),
                    "incoming\n",
                )
                .unwrap();
                commit(&f.context());
            }
            graph(&f, "merge");
            let observation = f.observe();
            let id = OperationId::new();
            let path = f.context().workdir().unwrap().join(dirty);
            fs::write(&path, "private untouched bytes").unwrap();
            let before = f.image();
            let result = execute(&f, id, &observation.observation);
            assert_eq!(
                result.code,
                ResultCode::WorktreeNotClean,
                "{dirty}: {result:?}"
            );
            no_owner(&f, id);
            assert_eq!(before, f.image());
            assert_eq!(fs::read(&path).unwrap(), b"private untouched bytes");
        }
    }
}

#[cfg(unix)]
#[test]
fn local_sync_symlink_substitution_never_writes_through_the_target() {
    use std::os::unix::fs::symlink;
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        graph(&f, "merge");
        let observation = f.observe();
        let id = OperationId::new();
        let outside = f.data.path().join("outside");
        fs::write(&outside, "outside private").unwrap();
        let target = f.context().workdir().unwrap().join("ordinary");
        fs::remove_file(&target).unwrap();
        symlink(&outside, &target).unwrap();
        let before = fs::read(f.context().path().join("index")).unwrap();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.code, ResultCode::WorktreeNotClean);
        no_owner(&f, id);
        assert_eq!(fs::read(&outside).unwrap(), b"outside private");
        assert_eq!(fs::read_link(&target).unwrap(), outside);
        assert_eq!(before, fs::read(f.context().path().join("index")).unwrap());
    }
}

#[test]
fn local_sync_committed_primary_with_unreachable_remote_and_live_transport_spy() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let spy = calls.clone();
    // Unique fixture-only protocol; the callback refuses before a socket exists.
    unsafe {
        git2::transport::register("manyhands-local-sync-spy", move |_| {
            spy.fetch_add(1, Ordering::SeqCst);
            Err(git2::Error::from_str("fixture transport refuses"))
        })
        .unwrap();
    }
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let p = primary(&f);
        p.remote(
            "origin",
            "manyhands-local-sync-spy://unreachable.invalid/repository",
        )
        .unwrap();
        let path = f.root.path().join(".manyhands/config.toml");
        let config = fs::read_to_string(&path).unwrap();
        fs::write(
            &path,
            format!("{config}\npublication_remote = \"origin\"\n"),
        )
        .unwrap();
        commit(&p);
        f.service
            .refresh_repository(RefreshRepositoryRequest {
                root: f.root.path().into(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        let (_, incoming) = graph(&f, "merge");
        let observation = f.observe();
        let id = OperationId::new();
        fs::write(
            f.root.path().join("incoming"),
            "private uncommitted primary",
        )
        .unwrap();
        let before = calls.load(Ordering::SeqCst);
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.code, ResultCode::Ok, "{result:?}");
        assert_eq!(calls.load(Ordering::SeqCst), before);
        assert_eq!(
            fs::read(f.context().workdir().unwrap().join("incoming")).unwrap(),
            b"committed primary\n"
        );
        assert_eq!(
            fs::read(f.root.path().join("incoming")).unwrap(),
            b"private uncommitted primary"
        );
        refs_unchanged(&f, incoming);
        assert!(
            p.find_remote("origin")
                .unwrap()
                .fetch(&["main"], None, None)
                .is_err()
        );
        assert!(
            calls.load(Ordering::SeqCst) > before,
            "negative control must exercise the spy"
        );
    }
}

#[test]
fn local_sync_no_drop_child() {
    let Ok(root) = std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_ROOT") else {
        return;
    };
    let data = std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_DATA").unwrap();
    let point = match std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_POINT")
        .unwrap()
        .as_str()
    {
        "checkout" => FailurePoint::LocalSyncDuringCheckout,
        "reference" => FailurePoint::LocalSyncInsideRefWindow,
        "ref_logs" => FailurePoint::LocalSyncAfterRef,
        "objects" => FailurePoint::LocalSyncAfterObjectImport,
        _ => panic!("unknown child boundary"),
    };
    let service =
        RepositoryService::open_at_with_exit_point_for_testing(Path::new(&data), point).unwrap();
    let repo = service.resolve_repository(Path::new(&root)).unwrap();
    let kind = if std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_KIND").unwrap() == "ticket" {
        AuthoringKind::Ticket
    } else {
        AuthoringKind::Document
    };
    let item = std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_ITEM")
        .unwrap()
        .parse()
        .unwrap();
    let id = OperationId::parse(&std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_ID").unwrap()).unwrap();
    let observation = std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_OBSERVATION").unwrap();
    let _ = run::execute(&service, &repo, kind, &item, id, &observation);
    panic!("child did not exit at its real backend boundary");
}

#[test]
fn no_drop_backend_ambiguity_preserves_until_operator_proof() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for point in ["checkout", "reference"] {
            let f = Fixture::new(kind);
            let (context, _) = graph(&f, "merge");
            for n in 0..4 {
                fs::write(
                    f.root.path().join(format!("incoming-{n}")),
                    format!("primary {n}\n"),
                )
                .unwrap();
            }
            let incoming = commit(&primary(&f));
            let observation = f.observe();
            let id = OperationId::new();
            let logs = reflog_len(&f);
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "repository::local_sync::run_tests::local_sync_no_drop_child",
                    "--nocapture",
                ])
                .env("MANYHANDS_LOCAL_SYNC_CHILD_ROOT", f.root.path())
                .env("MANYHANDS_LOCAL_SYNC_CHILD_DATA", f.data.path())
                .env("MANYHANDS_LOCAL_SYNC_CHILD_POINT", point)
                .env(
                    "MANYHANDS_LOCAL_SYNC_CHILD_KIND",
                    authoring_kind_segment(&kind),
                )
                .env("MANYHANDS_LOCAL_SYNC_CHILD_ITEM", f.item.to_string())
                .env("MANYHANDS_LOCAL_SYNC_CHILD_ID", id.to_string())
                .env(
                    "MANYHANDS_LOCAL_SYNC_CHILD_OBSERVATION",
                    &observation.observation,
                )
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(FAILURE_POINT_EXIT_STATUS),
                "{point}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            let candidate = row.candidate.unwrap();
            let c = f.context();
            let lock = c.commondir().join(format!("{}.lock", row.context_ref));
            if point == "reference" {
                assert!(lock.exists(), "actual backend ref lock survives no Drop");
            }
            let interrupted = execute(&f, id, &observation.observation);
            assert_eq!(
                interrupted.reason,
                Reason::AmbiguousBackend,
                "{point}: {interrupted:?}"
            );
            assert_eq!(interrupted.effects.write, WriteEffect::Written);
            let private = c.workdir().unwrap().join("incoming");
            fs::write(&private, "operator incomplete repair").unwrap();
            let before = f.image();
            let refused = execute(&f, id, &observation.observation);
            assert_eq!(refused.reason, Reason::AmbiguousBackend, "{refused:?}");
            assert_eq!(before, f.image());
            assert_eq!(
                refused.effects.write,
                WriteEffect::Written,
                "partial checkout really wrote files"
            );
            assert_eq!(fs::read(&private).unwrap(), b"operator incomplete repair");
            if point == "reference" {
                assert!(lock.exists());
                // Operator explicitly settles the lock and establishes the fully
                // journaled result, including its exact branch and HEAD log append.
                fs::remove_file(lock).unwrap();
                c.checkout_tree(
                    c.find_commit(candidate).unwrap().as_object(),
                    Some(git2::build::CheckoutBuilder::new().force()),
                )
                .unwrap();
                let e = super::recover::Execution::read(&f.db(), id).unwrap();
                let signature = git2::Signature::new(
                    "Manyhands",
                    "local-sync@manyhands.invalid",
                    &git2::Time::new(e.ref_time, 0),
                )
                .unwrap();
                let mut tx = c.transaction().unwrap();
                tx.lock_ref(&row.context_ref).unwrap();
                tx.set_target(
                    &row.context_ref,
                    candidate,
                    Some(&signature),
                    &format!("manyhands local sync {id}"),
                )
                .unwrap();
                tx.commit().unwrap();
            } else {
                // Operator restores the exact clean baseline; the executor itself
                // has no reset, forced checkout or path deletion recovery.
                c.checkout_head(Some(
                    git2::build::CheckoutBuilder::new()
                        .force()
                        .remove_untracked(true),
                ))
                .unwrap();
            }
            let result = execute(&f, id, &observation.observation);
            assert_eq!(result.code, ResultCode::Ok, "{point}: {result:?}");
            assert_eq!(tip(&c), candidate);
            assert_eq!(reflog_len(&f), logs + 1);
            assert_eq!(
                c.find_commit(candidate)
                    .unwrap()
                    .parent_ids()
                    .collect::<Vec<_>>(),
                [context, incoming]
            );
        }
    }
}

#[test]
fn local_sync_owner_loss_before_checkout_preserves_candidate_and_historical_effects() {
    for lose in ["epoch", "lease"] {
        let f = Fixture::new(AuthoringKind::Document);
        graph(&f, "merge");
        let observation = f.observe();
        let id = OperationId::new();
        let db = f.data.path().join(REGISTRY_FILE);
        let common = f.context().commondir().to_owned();
        run::BOUNDARY_HOOK.with(|slot| *slot.borrow_mut()=Some(Box::new(move |point| {
            if point==FailurePoint::LocalSyncAfterCandidate {
                if lose=="epoch" {rusqlite::Connection::open(&db).unwrap().execute("UPDATE local_context_sync_records SET owner_epoch=owner_epoch+1 WHERE operation_ulid=?1",[id.to_string()]).unwrap();}
                else {let path=common.join("manyhands-operation.lock");fs::remove_file(&path).unwrap();fs::write(path,[]).unwrap();}
            }
        })));
        let before = f.image();
        let result = execute(&f, id, &observation.observation);
        run::BOUNDARY_HOOK.with(|slot| slot.borrow_mut().take());
        assert_eq!(
            result.code,
            ResultCode::ExternalChange,
            "{lose}: {result:?}"
        );
        assert_eq!(before, f.image());
        assert_eq!(result.effects.checkpoint, CheckpointEffect::Committed);
        assert_eq!(result.outcome(), Outcome::Partial);
        let candidate = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap()
            .candidate
            .unwrap();
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::Ok
        );
        assert_eq!(tip(&f.context()), candidate);
    }
}

#[test]
fn local_sync_ref_log_disagreement_is_not_repaired_or_replayed_as_a_second_commit() {
    for member in ["branch", "HEAD"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "merge");
        let observation = f.observe();
        let id = OperationId::new();
        faults(
            &f,
            id,
            &observation.observation,
            FailurePoint::LocalSyncAfterRef,
        );
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        let c = f.context();
        let path = if member == "branch" {
            c.commondir().join("logs").join(&row.context_ref)
        } else {
            c.path().join("logs/HEAD")
        };
        let valid = fs::read(&path).unwrap();
        let mut damaged = valid.clone();
        damaged.extend_from_slice(b"foreign backend metadata\n");
        fs::write(&path, &damaged).unwrap();
        let before = f.image();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.reason, Reason::AmbiguousBackend);
        assert_eq!(before, f.image());
        assert_eq!(fs::read(&path).unwrap(), damaged);
        fs::write(&path, valid).unwrap();
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::Ok
        );
        assert_eq!(tip(&c), row.candidate.unwrap());
    }
}

#[test]
fn local_sync_request_mismatch_retains_evidence_and_journals_contain_no_source() {
    let sentinel = "PRIVATE-SOURCE-LOCAL-SYNC-MUST-NOT-ENTER-JOURNALS";
    let f = Fixture::new(AuthoringKind::Ticket);
    graph(&f, "merge");
    fs::write(f.root.path().join("incoming"), sentinel).unwrap();
    commit(&primary(&f));
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncAfterCheckout,
    );
    let before = f.image();
    let row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    let changed = format!("local-sync-v1:{}", "f".repeat(64));
    let result = execute(&f, id, &changed);
    assert_eq!(result.code, ResultCode::RequestMismatch);
    assert_eq!(result.outcome(), Outcome::Partial);
    assert_eq!(before, f.image());
    assert_eq!(
        row,
        f.service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap()
    );
    assert!(!format!("{result:?}").contains(sentinel));
    assert_eq!(
        fs::read(f.context().workdir().unwrap().join("incoming")).unwrap(),
        sentinel.as_bytes()
    );
    let db = f.db();
    for table in [
        "local_context_sync_records",
        "local_context_sync_execution",
        "local_context_sync_steps",
        "local_context_sync_resolutions",
        "operation_records",
        "request_records",
    ] {
        let mut statement = db.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns = statement.column_count();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for column in 0..columns {
                match row.get_ref(column).unwrap() {
                    rusqlite::types::ValueRef::Text(bytes)
                    | rusqlite::types::ValueRef::Blob(bytes) => assert!(
                        !bytes
                            .windows(sentinel.len())
                            .any(|w| w == sentinel.as_bytes())
                    ),
                    _ => {}
                }
            }
        }
    }
}

#[test]
fn local_sync_execution_evidence_schema_is_versioned_and_loss_refuses_without_effects() {
    for corrupt in [
        "DROP TABLE local_context_sync_execution",
        "ALTER TABLE local_context_sync_execution ADD COLUMN unexpected TEXT",
        "PRAGMA ignore_check_constraints=ON; UPDATE local_context_sync_execution SET routing_digest=x'00'",
        "DELETE FROM local_context_sync_execution",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "merge");
        let observation = f.observe();
        let id = OperationId::new();
        faults(
            &f,
            id,
            &observation.observation,
            FailurePoint::LocalSyncAfterCheckout,
        );
        assert_eq!(f.db().query_row("SELECT count(*) FROM registry_migrations WHERE name='local_context_sync_execution_v3'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        let before = f.image();
        f.db().execute_batch(corrupt).unwrap();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(
            result.code,
            ResultCode::RecoveryRequired,
            "{corrupt}: {result:?}"
        );
        assert_eq!(before, f.image());
    }
}

#[test]
fn local_sync_object_intent_retries_the_exact_candidate_across_identity_changes() {
    for point in [
        FailurePoint::LocalSyncBeforeObjectImport,
        FailurePoint::LocalSyncAfterObjectImport,
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "merge");
        let observation = f.observe();
        let id = OperationId::new();
        let stopped = faults(&f, id, &observation.observation, point);
        assert_ne!(stopped.code, ResultCode::Ok);
        let intended:String=f.db().query_row("SELECT expected_new_oid FROM local_context_sync_steps WHERE operation_ulid=?1 AND kind='objects'",[id.to_string()],|r|r.get(0)).unwrap();
        let intended = git2::Oid::from_str(&intended).unwrap();
        let c = f.context();
        c.config()
            .unwrap()
            .set_str("user.name", "Changed after durable object intent")
            .unwrap();
        c.config()
            .unwrap()
            .set_str("user.email", "changed@example.invalid")
            .unwrap();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.code, ResultCode::Ok, "{point:?}: {result:?}");
        assert_eq!(
            tip(&c),
            intended,
            "a journaled object intent freezes candidate identity"
        );
        assert_eq!(
            c.find_commit(intended).unwrap().author().name(),
            Some("Fixture")
        );
    }
}

#[test]
fn local_sync_valid_oid_corruption_cannot_substitute_an_unrelated_candidate() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let (context, primary) = graph(&f, "merge");
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncAfterCandidate,
    );
    f.db()
        .execute(
            "UPDATE local_context_sync_records SET candidate_oid=?2 WHERE operation_ulid=?1",
            rusqlite::params![id.to_string(), primary.to_string()],
        )
        .unwrap();
    let before = f.image();
    let result = execute(&f, id, &observation.observation);
    assert_eq!(result.code, ResultCode::RecoveryRequired, "{result:?}");
    assert_eq!(before, f.image());
    assert_eq!(tip(&f.context()), context);
}

#[test]
fn local_sync_ignored_epoch_or_intent_update_never_authorizes_git_effects() {
    for table in ["local_context_sync_records", "local_context_sync_execution"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let observation = f.observe();
        let id = OperationId::new();
        faults(
            &f,
            id,
            &observation.observation,
            FailurePoint::LocalSyncAfterIntent,
        );
        let column = if table == "local_context_sync_records" {
            "owner_epoch"
        } else {
            "checkout_started"
        };
        f.db().execute_batch(&format!("CREATE TRIGGER refuse_effect_intent BEFORE UPDATE OF {column} ON {table} BEGIN SELECT RAISE(IGNORE); END")).unwrap();
        let before = f.image();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(
            result.code,
            ResultCode::RecoveryRequired,
            "{table}: {result:?}"
        );
        assert_eq!(before, f.image());
        f.db()
            .execute_batch("DROP TRIGGER refuse_effect_intent")
            .unwrap();
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::Ok
        );
    }
}

#[test]
fn local_sync_index_handoff_cannot_be_settled_by_generic_refresh() {
    let f = Fixture::new(AuthoringKind::Document);
    graph(&f, "merge");
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncBeforeIndex,
    );
    let row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    let before = f.image();
    assert!(
        f.service
            .refresh_repository(RefreshRepositoryRequest {
                root: f.root.path().into(),
                operation_id: id
            })
            .is_err()
    );
    assert_eq!(
        row,
        f.service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap()
    );
    assert_eq!(before, f.image());
    assert_eq!(
        execute(&f, id, &observation.observation).code,
        ResultCode::Ok
    );
}

#[test]
fn local_sync_same_mismatch_code_uses_historical_effects_for_its_failure_class() {
    let mut classes = Vec::new();
    for point in [
        FailurePoint::LocalSyncAfterIntent,
        FailurePoint::LocalSyncAfterCheckout,
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let observation = f.observe();
        let id = OperationId::new();
        faults(&f, id, &observation.observation, point);
        let result = execute(&f, id, &format!("local-sync-v1:{}", "f".repeat(64)));
        assert_eq!(result.code, ResultCode::RequestMismatch);
        let envelope = Envelope::<()>::classified_mutation(
            "context sync-local",
            Scope::default(),
            false,
            result.code,
            result.effects,
            false,
        );
        classes.push(envelope.failure_class());
    }
    assert_eq!(classes[0], ResultCode::RequestMismatch.failure_class());
    assert_eq!(classes[1], Some(FailureClass::Incomplete));
    assert_ne!(classes[0], classes[1]);
}

#[test]
fn local_sync_lease_contender_cannot_erase_historical_effects() {
    use std::{cell::RefCell, rc::Rc};
    let f = Fixture::new(AuthoringKind::Ticket);
    graph(&f, "merge");
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncAfterCandidate,
    );
    let lease = Rc::new(RefCell::new(None));
    let held = lease.clone();
    let root = f.root.path().to_owned();
    run::BOUNDARY_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |point| {
            if point == FailurePoint::LocalSyncBeforeExecuteLease {
                let git = git2::Repository::open(&root).unwrap();
                *held.borrow_mut() = Some(
                    repository_lease(&git, &root, RepositoryOperation::ContextSyncLocal).unwrap(),
                );
            }
        }))
    });
    let before = f.image();
    let result = execute(&f, id, &observation.observation);
    run::BOUNDARY_HOOK.with(|slot| slot.borrow_mut().take());
    lease.borrow_mut().take();
    assert_eq!(result.code, ResultCode::Busy);
    assert_eq!(result.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(result.outcome(), Outcome::Partial);
    assert_eq!(before, f.image());
    assert_eq!(
        execute(&f, id, &observation.observation).code,
        ResultCode::Ok
    );
}

#[test]
fn local_sync_no_drop_candidate_and_ref_logs_reconcile_exactly_once() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for point in ["objects", "ref_logs"] {
            let f = Fixture::new(kind);
            let (context, incoming) = graph(&f, "merge");
            let observation = f.observe();
            let id = OperationId::new();
            let logs = reflog_len(&f);
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "repository::local_sync::run_tests::local_sync_no_drop_child",
                    "--nocapture",
                ])
                .env("MANYHANDS_LOCAL_SYNC_CHILD_ROOT", f.root.path())
                .env("MANYHANDS_LOCAL_SYNC_CHILD_DATA", f.data.path())
                .env("MANYHANDS_LOCAL_SYNC_CHILD_POINT", point)
                .env(
                    "MANYHANDS_LOCAL_SYNC_CHILD_KIND",
                    authoring_kind_segment(&kind),
                )
                .env("MANYHANDS_LOCAL_SYNC_CHILD_ITEM", f.item.to_string())
                .env("MANYHANDS_LOCAL_SYNC_CHILD_ID", id.to_string())
                .env(
                    "MANYHANDS_LOCAL_SYNC_CHILD_OBSERVATION",
                    &observation.observation,
                )
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(FAILURE_POINT_EXIT_STATUS),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            let candidate = row.candidate.unwrap();
            let c = f.context();
            assert!(c.find_commit(candidate).is_ok());
            assert_eq!(
                tip(&c),
                if point == "objects" {
                    context
                } else {
                    candidate
                }
            );
            assert_eq!(reflog_len(&f), logs + usize::from(point == "ref_logs"));
            c.config()
                .unwrap()
                .set_str("user.name", "Changed after child exit")
                .unwrap();
            assert_eq!(
                execute(&f, id, &observation.observation).code,
                ResultCode::Ok
            );
            assert_eq!(tip(&c), candidate);
            assert_eq!(reflog_len(&f), logs + 1);
            let commit = c.find_commit(candidate).unwrap();
            assert_eq!(commit.parent_ids().collect::<Vec<_>>(), [context, incoming]);
            assert_eq!(commit.author().name(), Some("Fixture"));
            refs_unchanged(&f, incoming);
        }
    }
}

#[test]
fn spec_same_tree_integrations_transition_once_without_claiming_file_writes() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["empty_fast_forward", "reverted_fast_forward", "merge"] {
            for stop in [None, Some(FailurePoint::LocalSyncAfterRef)] {
                let f = Fixture::new(kind);
                graph(&f, "equal");
                let p = primary(&f);
                if shape == "merge" {
                    commit(&f.context());
                }
                if shape == "reverted_fast_forward" {
                    fs::write(f.root.path().join("ordinary"), "temporary primary change").unwrap();
                    commit(&p);
                    fs::write(f.root.path().join("ordinary"), "base\n").unwrap();
                }
                // A distinct subject ensures genuinely divergent equal-tree
                // commits rather than accidentally identical fixture objects.
                let parent = p.find_commit(tip(&p)).unwrap();
                let mut index = p.index().unwrap();
                index
                    .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
                    .unwrap();
                index.write().unwrap();
                let tree = p.find_tree(index.write_tree().unwrap()).unwrap();
                let sig = p.signature().unwrap();
                let incoming = p
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        "same tree primary",
                        &tree,
                        &[&parent],
                    )
                    .unwrap();
                p.reference("refs/remotes/origin/main", incoming, true, "fixture")
                    .unwrap();
                let context = tip(&f.context());
                let image = f.image();
                let logs = reflog_len(&f);
                assert_eq!(
                    f.context().find_commit(context).unwrap().tree_id(),
                    tree.id()
                );
                let observation = f.observe();
                let id = OperationId::new();
                if let Some(point) = stop {
                    assert_ne!(
                        faults(&f, id, &observation.observation, point).code,
                        ResultCode::Ok
                    );
                }
                let result = execute(&f, id, &observation.observation);
                assert_eq!(
                    result.code,
                    ResultCode::Ok,
                    "{kind:?} {shape} {stop:?}: {result:?}"
                );
                assert_eq!(result.effects.write, WriteEffect::Unchanged);
                assert_eq!(
                    result.effects.checkpoint,
                    if shape == "merge" {
                        CheckpointEffect::Committed
                    } else {
                        CheckpointEffect::Unchanged
                    }
                );
                assert_eq!(result.effects.integration, IntegrationEffect::Complete);
                assert_eq!(f.image().1, image.1);
                assert_eq!(f.image().2, image.2);
                assert_eq!(reflog_len(&f), logs + 1);
                refs_unchanged(&f, incoming);
                if shape == "merge" {
                    assert_eq!(
                        f.context()
                            .find_commit(tip(&f.context()))
                            .unwrap()
                            .parent_ids()
                            .collect::<Vec<_>>(),
                        [context, incoming]
                    );
                } else {
                    assert_eq!(tip(&f.context()), incoming);
                }
                let e = super::recover::Execution::read(&f.db(), id).unwrap();
                assert!(!e.checkout_started && !e.checkout_observed);
                assert_eq!(f.db().query_row("SELECT count(*) FROM local_context_sync_steps WHERE operation_ulid=?1 AND kind='checkout'",[id.to_string()],|r|r.get::<_,i64>(0)).unwrap(),0);
                assert_eq!(execute(&f, id, &observation.observation), result);
                assert_eq!(reflog_len(&f), logs + 1);
            }
        }
    }
}

fn local_authority_rows(f: &Fixture) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    let db = f.db();
    [
        "operation_records",
        "local_context_sync_records",
        "local_context_sync_execution",
        "local_context_sync_steps",
        "repositories",
        "contexts",
    ]
    .into_iter()
    .map(|table| {
        let mut statement = db
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = statement.column_count();
        statement
            .query_map([], |row| {
                (0..columns)
                    .map(|i| row.get(i))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    })
    .collect()
}

#[test]
fn spec_missing_execution_authority_refuses_public_refresh_without_settlement() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for stop in [
            FailurePoint::LocalSyncBeforeIndex,
            FailurePoint::LocalSyncAfterIntent,
        ] {
            let f = Fixture::new(kind);
            graph(&f, "merge");
            let observation = f.observe();
            let id = OperationId::new();
            faults(&f, id, &observation.observation, stop);
            f.db()
                .execute(
                    "DELETE FROM local_context_sync_execution WHERE operation_ulid=?1",
                    [id.to_string()],
                )
                .unwrap();
            let rows = local_authority_rows(&f);
            let image = f.image();
            let logs = reflog_len(&f);
            assert!(
                f.service
                    .refresh_repository(RefreshRepositoryRequest {
                        root: f.root.path().into(),
                        operation_id: id
                    })
                    .is_err(),
                "missing execution must never grant a generic handoff"
            );
            assert_eq!(rows, local_authority_rows(&f));
            assert_eq!(image, f.image());
            assert_eq!(logs, reflog_len(&f));
            assert_eq!(
                f.service
                    .read_local_context_sync_record(&f.repo, id)
                    .unwrap_err()
                    .code(),
                ResultCode::RecoveryRequired
            );
            assert!(RepositoryService::open_at(f.data.path()).is_err());
            assert_eq!(rows, local_authority_rows(&f));
        }
    }
}

#[test]
fn spec_execution_authority_upgrade_preserves_store_only_and_executor_records() {
    for executor in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let id = OperationId::new();
        if executor {
            graph(&f, "merge");
            faults(
                &f,
                id,
                &f.observe().observation,
                FailurePoint::LocalSyncBeforeIndex,
            );
        } else {
            f.begin(id).unwrap();
        }
        let before = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        f.db().execute_batch("ALTER TABLE local_context_sync_records DROP COLUMN execution_required; DELETE FROM registry_migrations WHERE name='local_context_sync_execution_authority_v4'").unwrap();
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert_eq!(
            before,
            reopened
                .read_local_context_sync_record(&f.repo, id)
                .unwrap()
        );
        assert_eq!(f.db().query_row("SELECT execution_required FROM local_context_sync_records WHERE operation_ulid=?1",[id.to_string()],|r|r.get::<_,bool>(0)).unwrap(),executor);
    }
}

#[test]
fn spec_execution_authority_upgrade_rolls_back_and_refuses_lost_legacy_progress() {
    for lose in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "merge");
        let id = OperationId::new();
        faults(
            &f,
            id,
            &f.observe().observation,
            FailurePoint::LocalSyncBeforeIndex,
        );
        f.db().execute_batch("ALTER TABLE local_context_sync_records DROP COLUMN execution_required; DELETE FROM registry_migrations WHERE name='local_context_sync_execution_authority_v4'").unwrap();
        if lose {
            f.db()
                .execute(
                    "DELETE FROM local_context_sync_execution WHERE operation_ulid=?1",
                    [id.to_string()],
                )
                .unwrap();
        } else {
            f.db().execute_batch("CREATE TRIGGER stop_authority_upgrade BEFORE INSERT ON registry_migrations WHEN NEW.name='local_context_sync_execution_authority_v4' BEGIN SELECT RAISE(ABORT,'fixture'); END").unwrap();
        }
        let rows = local_authority_rows(&f);
        let image = f.image();
        assert!(RepositoryService::open_at(f.data.path()).is_err());
        assert_eq!(rows, local_authority_rows(&f));
        assert_eq!(image, f.image());
        assert_eq!(f.db().query_row("SELECT count(*) FROM registry_migrations WHERE name='local_context_sync_execution_authority_v4'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        if !lose {
            f.db()
                .execute_batch("DROP TRIGGER stop_authority_upgrade")
                .unwrap();
            assert!(RepositoryService::open_at(f.data.path()).is_ok());
        }
    }
}

#[test]
fn spec_journal_access_loss_preserves_known_effects_and_original_drive_error() {
    for shape in ["fast_forward", "merge"] {
        for point in [
            FailurePoint::LocalSyncAfterCheckout,
            FailurePoint::LocalSyncAfterRef,
            FailurePoint::LocalSyncBeforeIndex,
            FailurePoint::LocalSyncAfterIntent,
            FailurePoint::LocalSyncBeforeObjectImport,
            FailurePoint::LocalSyncAfterObjectImport,
        ] {
            if shape == "fast_forward"
                && matches!(
                    point,
                    FailurePoint::LocalSyncBeforeObjectImport
                        | FailurePoint::LocalSyncAfterObjectImport
                )
            {
                continue;
            }
            let f = Fixture::new(AuthoringKind::Ticket);
            let (context, incoming) = graph(&f, shape);
            let observation = f.observe();
            let id = OperationId::new();
            super::record::ACCESS_FAILURES.with(|counts| counts.set((0, 0)));
            run::BOUNDARY_HOOK.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(move |p| {
                    if p == point {
                        super::record::ACCESS_FAULT
                            .with(|fault| fault.set(Some(std::io::ErrorKind::PermissionDenied)));
                    }
                }))
            });
            let result = execute(&f, id, &observation.observation);
            let failures = super::record::ACCESS_FAILURES.with(|counts| counts.get());
            super::record::ACCESS_FAULT.with(|fault| fault.set(None));
            run::BOUNDARY_HOOK.with(|slot| slot.borrow_mut().take());
            assert_eq!(
                failures.0, 1,
                "the final evidence READ must actually fail at the store boundary"
            );
            assert!(failures.1 > 0, "journal WRITES must also be unavailable");
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            if matches!(
                point,
                FailurePoint::LocalSyncAfterCheckout
                    | FailurePoint::LocalSyncAfterRef
                    | FailurePoint::LocalSyncBeforeIndex
            ) {
                assert_eq!(
                    fs::read(f.context().workdir().unwrap().join("incoming")).unwrap(),
                    b"committed primary\n"
                );
            }
            assert_eq!(
                result.state,
                Some(row.state),
                "{shape} {point:?}: retain known phase"
            );
            let index = point == FailurePoint::LocalSyncBeforeIndex;
            assert_eq!(
                result.code,
                if index {
                    ResultCode::DiscoveryPending
                } else {
                    ResultCode::IndexUnavailable
                },
                "{shape} {point:?}: preserve drive error"
            );
            if index {
                assert_eq!(result.reason, Reason::IndexPending);
            }
            let wrote = matches!(
                point,
                FailurePoint::LocalSyncAfterCheckout
                    | FailurePoint::LocalSyncAfterRef
                    | FailurePoint::LocalSyncBeforeIndex
            );
            let made_commit = shape == "merge"
                && !matches!(
                    point,
                    FailurePoint::LocalSyncAfterIntent | FailurePoint::LocalSyncBeforeObjectImport
                );
            let integrated = matches!(
                point,
                FailurePoint::LocalSyncAfterRef | FailurePoint::LocalSyncBeforeIndex
            );
            assert_eq!(
                result.effects.write,
                if wrote {
                    WriteEffect::Written
                } else {
                    WriteEffect::Unchanged
                },
                "{shape} {point:?}"
            );
            assert_eq!(
                result.effects.checkpoint,
                if made_commit {
                    CheckpointEffect::Committed
                } else {
                    CheckpointEffect::Unchanged
                },
                "{shape} {point:?}"
            );
            assert_eq!(
                result.effects.integration,
                if integrated {
                    IntegrationEffect::Complete
                } else {
                    IntegrationEffect::Pending
                },
                "{shape} {point:?}"
            );
            if made_commit || wrote {
                assert_eq!(
                    result.effects.commit_oid,
                    row.candidate.map(|o| o.to_string())
                );
                assert_eq!(result.outcome(), Outcome::Partial);
            } else {
                assert_eq!(result.effects.commit_oid, None);
                assert!(!result.effects.is_durable());
                assert_ne!(result.outcome(), Outcome::Partial);
            }
            assert_eq!(
                tip(&f.context()),
                if integrated {
                    row.candidate.unwrap()
                } else {
                    context
                }
            );
            refs_unchanged(&f, incoming);
            assert!(!format!("{result:?}").contains("PermissionDenied"));
            assert_eq!(
                execute(&f, id, &observation.observation).code,
                ResultCode::Ok
            );
        }
    }
}

#[test]
fn spec_corrupt_dispositions_refuse_before_any_git_effect_or_settlement() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge", "equal", "contained"] {
            for corrupt in ["noop", "fast_forward", "merge"] {
                let actual = match shape {
                    "equal" | "contained" => "noop",
                    other => other,
                };
                if actual == corrupt {
                    continue;
                }
                let f = Fixture::new(kind);
                graph(&f, shape);
                let observation = f.observe();
                let id = OperationId::new();
                faults(
                    &f,
                    id,
                    &observation.observation,
                    FailurePoint::LocalSyncAfterIntent,
                );
                f.db().execute("UPDATE local_context_sync_execution SET disposition=?2 WHERE operation_ulid=?1",rusqlite::params![id.to_string(),corrupt]).unwrap();
                let rows = local_authority_rows(&f);
                let image = f.image();
                let logs = reflog_len(&f);
                let result = execute(&f, id, &observation.observation);
                assert_eq!(
                    result.code,
                    ResultCode::RecoveryRequired,
                    "{kind:?} {shape}->{corrupt}: {result:?}"
                );
                assert_eq!(rows, local_authority_rows(&f));
                assert_eq!(image, f.image());
                assert_eq!(logs, reflog_len(&f));
                assert!(!result.effects.is_durable());
            }
        }
    }
}

#[test]
fn spec_corrupt_completed_noop_cannot_replay_fabricated_integration_effects() {
    for corrupt in ["fast_forward", "merge"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "equal");
        let observation = f.observe();
        let id = OperationId::new();
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::AlreadyApplied
        );
        f.db()
            .execute(
                "UPDATE local_context_sync_execution SET disposition=?2 WHERE operation_ulid=?1",
                rusqlite::params![id.to_string(), corrupt],
            )
            .unwrap();
        let rows = local_authority_rows(&f);
        let image = f.image();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.code, ResultCode::RecoveryRequired);
        assert!(!result.effects.is_durable());
        assert_eq!(rows, local_authority_rows(&f));
        assert_eq!(image, f.image());
    }
}
