use super::{
    record::LocalSyncState,
    record_tests::{Fixture, commit},
    run::{self, Reason},
};
use crate::{repository::*, results::*};
use std::{fs, path::Path};

#[cfg(unix)]
#[path = "checkout_write_tests.rs"]
mod checkout_write_tests;

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
    let child_point = std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_POINT").unwrap();
    let point = match child_point.as_str() {
        "checkout" => FailurePoint::LocalSyncDuringCheckout,
        "reference" => FailurePoint::LocalSyncInsideRefWindow,
        "ref_logs" => FailurePoint::LocalSyncAfterRef,
        "objects" => FailurePoint::LocalSyncAfterObjectImport,
        "mode" => FailurePoint::LocalSyncDuringCheckout,
        _ => panic!("unknown child boundary"),
    };
    let service = if child_point == "mode" {
        RepositoryService::open_at(Path::new(&data)).unwrap()
    } else {
        RepositoryService::open_at_with_exit_point_for_testing(Path::new(&data), point).unwrap()
    };
    let repo = service.resolve_repository(Path::new(&root)).unwrap();
    let kind = if std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_KIND").unwrap() == "ticket" {
        AuthoringKind::Ticket
    } else {
        AuthoringKind::Document
    };
    let item: crate::canonical::ItemId = std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_ITEM")
        .unwrap()
        .parse()
        .unwrap();
    let id = OperationId::parse(&std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_ID").unwrap()).unwrap();
    let observation = std::env::var("MANYHANDS_LOCAL_SYNC_CHILD_OBSERVATION").unwrap();
    #[cfg(unix)]
    if child_point == "mode" {
        use std::os::unix::fs::PermissionsExt;
        let path = repo
            .root()
            .join(".manyhands/worktrees")
            .join(item.to_string())
            .join("mode-only");
        run::BOUNDARY_HOOK.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move |point| {
                if point == FailurePoint::LocalSyncDuringCheckout
                    && fs::metadata(&path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
                {
                    std::process::exit(FAILURE_POINT_EXIT_STATUS);
                }
            }))
        });
    }
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

#[test]
fn quality_filtered_checkout_bytes_are_proven_for_lf_and_crlf_baselines() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge"] {
            for attributes in [false, true] {
                for crlf_baseline in [false, true] {
                    let f = Fixture::new(kind);
                    let c = f.context();
                    if attributes {
                        fs::write(
                            c.workdir().unwrap().join(".gitattributes"),
                            "* text eol=crlf\n",
                        )
                        .unwrap();
                        commit(&c);
                    }
                    graph(&f, "equal");
                    if shape == "merge" {
                        fs::write(c.workdir().unwrap().join("context-only"), "local\n").unwrap();
                        commit(&c);
                    }
                    let (context, incoming) = graph(&f, shape);
                    if !attributes {
                        c.config().unwrap().set_bool("core.autocrlf", true).unwrap();
                    }
                    if crlf_baseline {
                        fs::remove_file(c.workdir().unwrap().join("ordinary")).unwrap();
                        c.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                            .unwrap();
                        assert_eq!(
                            fs::read(c.workdir().unwrap().join("ordinary")).unwrap(),
                            b"base\r\n"
                        );
                    }
                    let configuration = fs::read(c.commondir().join("config")).unwrap();
                    let observation = f.observe();
                    let id = OperationId::new();
                    let logs = reflog_len(&f);
                    let stopped = faults(
                        &f,
                        id,
                        &observation.observation,
                        FailurePoint::LocalSyncAfterCheckout,
                    );
                    assert_eq!(
                        stopped.effects.write,
                        WriteEffect::Written,
                        "{kind:?} {shape} attr={attributes} crlf={crlf_baseline}: {stopped:?}"
                    );
                    assert_eq!(
                        fs::read(c.workdir().unwrap().join("incoming")).unwrap(),
                        b"committed primary\r\n"
                    );
                    let result = execute(&f, id, &observation.observation);
                    assert_eq!(result.code, ResultCode::Ok, "{result:?}");
                    assert_eq!(reflog_len(&f), logs + 1);
                    refs_unchanged(&f, incoming);
                    if shape == "merge" {
                        assert_eq!(
                            c.find_commit(tip(&c))
                                .unwrap()
                                .parent_ids()
                                .collect::<Vec<_>>(),
                            [context, incoming]
                        );
                    }
                    assert_eq!(
                        fs::read(c.commondir().join("config")).unwrap(),
                        configuration
                    );
                }
            }
        }
    }
}

#[test]
fn quality_introduced_symlink_and_gitlink_refuse_before_acceptance() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge"] {
            for mode in [0o120000, 0o160000] {
                let f = Fixture::new(kind);
                graph(&f, shape);
                let p = primary(&f);
                let parent = p.find_commit(tip(&p)).unwrap();
                let mut tree = p.treebuilder(Some(&parent.tree().unwrap())).unwrap();
                let object = if mode == 0o120000 {
                    p.blob(b"ordinary").unwrap()
                } else {
                    parent.id()
                };
                tree.insert("unsupported-input", object, mode).unwrap();
                let tree = p.find_tree(tree.write().unwrap()).unwrap();
                let sig = p.signature().unwrap();
                let incoming = p
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        "unsupported prospective mode",
                        &tree,
                        &[&parent],
                    )
                    .unwrap();
                p.reference("refs/remotes/origin/main", incoming, true, "fixture")
                    .unwrap();
                let observation = f.observe();
                let id = OperationId::new();
                let image = f.image();
                let logs = reflog_len(&f);
                let result = execute(&f, id, &observation.observation);
                no_owner(&f, id);
                assert_eq!(result.code, ResultCode::RecoveryRequired);
                assert_eq!(result.reason, Reason::UnsupportedConflict);
                assert!(!result.effects.is_durable());
                assert_eq!(image, f.image());
                assert_eq!(logs, reflog_len(&f));
                refs_unchanged(&f, incoming);
                assert!(
                    !f.context()
                        .workdir()
                        .unwrap()
                        .join("unsupported-input")
                        .exists()
                );
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
        }
    }
}

#[test]
fn quality_disabled_reflogs_refuse_before_effects_and_recheck_accepted_work() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge"] {
            for stop in [
                None,
                Some(FailurePoint::LocalSyncAfterIntent),
                Some(FailurePoint::LocalSyncAfterCheckout),
            ] {
                let f = Fixture::new(kind);
                let (_, incoming) = graph(&f, shape);
                let c = f.context();
                let observation = f.observe();
                let id = OperationId::new();
                if let Some(point) = stop {
                    faults(&f, id, &observation.observation, point);
                }
                c.config()
                    .unwrap()
                    .set_bool("core.logallrefupdates", false)
                    .unwrap();
                let configuration = fs::read(c.commondir().join("config")).unwrap();
                let image = f.image();
                let logs = reflog_len(&f);
                let result = execute(&f, id, &observation.observation);
                if stop.is_none() {
                    no_owner(&f, id);
                }
                assert_eq!(result.code, ResultCode::RecoveryRequired);
                assert_eq!(result.reason, Reason::AuthorityMissing, "{result:?}");
                assert_eq!(image, f.image());
                assert_eq!(logs, reflog_len(&f));
                refs_unchanged(&f, incoming);
                assert_eq!(
                    fs::read(c.commondir().join("config")).unwrap(),
                    configuration
                );
                if stop == Some(FailurePoint::LocalSyncAfterCheckout) {
                    assert_eq!(result.effects.write, WriteEffect::Written);
                } else {
                    assert!(!result.effects.is_durable());
                }
                // The caller changes configuration explicitly; the executor never does.
                c.config()
                    .unwrap()
                    .set_bool("core.logallrefupdates", true)
                    .unwrap();
                assert_eq!(
                    execute(&f, id, &observation.observation).code,
                    ResultCode::Ok
                );
                assert_eq!(reflog_len(&f), logs + 1);
            }
        }
        for shape in ["equal", "contained"] {
            let f = Fixture::new(kind);
            graph(&f, shape);
            f.context()
                .config()
                .unwrap()
                .set_bool("core.logallrefupdates", false)
                .unwrap();
            let observation = f.observe();
            let image = f.image();
            let logs = reflog_len(&f);
            assert_eq!(
                execute(&f, OperationId::new(), &observation.observation).code,
                ResultCode::AlreadyApplied
            );
            assert_eq!(image, f.image());
            assert_eq!(logs, reflog_len(&f));
        }
    }
}

#[test]
fn quality_filtered_proof_rejects_lossy_clean_filter_equivalence() {
    let f = Fixture::new(AuthoringKind::Document);
    let c = f.context();
    let path = c.workdir().unwrap().join("expanded.txt");
    fs::write(
        c.workdir().unwrap().join(".gitattributes"),
        "expanded.txt ident\n",
    )
    .unwrap();
    fs::write(&path, "$Id$\n").unwrap();
    commit(&c);
    graph(&f, "equal");
    fs::remove_file(&path).unwrap();
    c.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let valid = fs::read(&path).unwrap();
    assert!(valid.starts_with(b"$Id: "));
    let (_, incoming) = graph(&f, "fast_forward");
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncAfterCheckout,
    );
    fs::write(&path, "$Id: foreign expansion $\n").unwrap();
    // Positive control: an actual clean-filter roundtrip collapses the foreign
    // expansion to the original blob. Keep its output in a memory ODB.
    {
        use std::io::Write;
        let worker = git2::Repository::open(c.path()).unwrap();
        let odb = worker.odb().unwrap();
        let _memory = odb.add_new_mempack_backend(1000).unwrap();
        let mut writer = worker.blob_writer(Some(Path::new("expanded.txt"))).unwrap();
        writer.write_all(b"$Id: foreign expansion $\n").unwrap();
        assert_eq!(
            writer.commit().unwrap(),
            c.head()
                .unwrap()
                .peel_to_tree()
                .unwrap()
                .get_path(Path::new("expanded.txt"))
                .unwrap()
                .id()
        );
    }
    let image = f.image();
    let result = execute(&f, id, &observation.observation);
    assert_eq!(result.reason, Reason::AmbiguousBackend);
    assert_eq!(image, f.image());
    assert_eq!(fs::read(&path).unwrap(), b"$Id: foreign expansion $\n");
    refs_unchanged(&f, incoming);
    fs::write(&path, valid).unwrap();
    assert_eq!(
        execute(&f, id, &observation.observation).code,
        ResultCode::Ok
    );
}

#[test]
fn quality_checkout_baseline_upgrade_preserves_history_and_rolls_back() {
    for fail in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let observation = f.observe();
        let id = OperationId::new();
        faults(
            &f,
            id,
            &observation.observation,
            FailurePoint::LocalSyncAfterCheckout,
        );
        let before = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        let image = f.image();
        f.db().execute_batch("ALTER TABLE local_context_sync_execution DROP COLUMN baseline_worktree; DELETE FROM registry_migrations WHERE name='local_context_sync_checkout_image_v5'").unwrap();
        if fail {
            f.db().execute_batch("CREATE TRIGGER stop_checkout_image_upgrade BEFORE INSERT ON registry_migrations WHEN NEW.name='local_context_sync_checkout_image_v5' BEGIN SELECT RAISE(ABORT,'fixture'); END").unwrap();
            let rows = local_authority_rows(&f);
            assert!(RepositoryService::open_at(f.data.path()).is_err());
            assert_eq!(rows, local_authority_rows(&f));
            assert_eq!(image, f.image());
            f.db()
                .execute_batch("DROP TRIGGER stop_checkout_image_upgrade")
                .unwrap();
        }
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert_eq!(
            before,
            reopened
                .read_local_context_sync_record(&f.repo, id)
                .unwrap()
        );
        assert!(
            super::recover::Execution::read(&f.db(), id)
                .unwrap()
                .checkout_baseline
                .is_none(),
            "migration must not invent a pre-write snapshot"
        );
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::Ok
        );
    }
}

#[test]
fn quality_checkout_baseline_corruption_is_not_recovery_authority() {
    for value in ["x'00'", "'not a digest'", "42"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let observation = f.observe();
        let id = OperationId::new();
        faults(
            &f,
            id,
            &observation.observation,
            FailurePoint::LocalSyncAfterCheckout,
        );
        let image = f.image();
        f.db().execute_batch(&format!("PRAGMA ignore_check_constraints=ON; UPDATE local_context_sync_execution SET baseline_worktree={value}")).unwrap();
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::RecoveryRequired
        );
        assert_eq!(image, f.image());
    }
}

#[test]
fn quality_logging_default_always_and_in_call_disable_are_observed() {
    for setting in ["default", "always", "disable_during_checkout"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let c = f.context();
        if setting == "default" {
            c.config().unwrap().remove("core.logallrefupdates").unwrap();
        }
        if setting == "always" {
            c.config()
                .unwrap()
                .set_str("core.logallrefupdates", "always")
                .unwrap();
        }
        let observation = f.observe();
        let id = OperationId::new();
        let before = tip(&c);
        let logs = reflog_len(&f);
        if setting == "disable_during_checkout" {
            let root = f.root.path().to_owned();
            run::BOUNDARY_HOOK.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(move |point| {
                    if point == FailurePoint::LocalSyncAfterCheckout {
                        git2::Repository::open(&root)
                            .unwrap()
                            .config()
                            .unwrap()
                            .set_bool("core.logallrefupdates", false)
                            .unwrap();
                    }
                }))
            });
        }
        let result = execute(&f, id, &observation.observation);
        run::BOUNDARY_HOOK.with(|slot| slot.borrow_mut().take());
        if setting == "disable_during_checkout" {
            assert_eq!(result.reason, Reason::AuthorityMissing);
            assert_eq!(result.effects.write, WriteEffect::Written);
            assert_eq!(tip(&c), before);
            assert_eq!(reflog_len(&f), logs);
            // The hook wrote through another handle. Reopen rather than asking
            // a stale config writer to set the value it already caches as true.
            primary(&f)
                .config()
                .unwrap()
                .set_bool("core.logallrefupdates", true)
                .unwrap();
            let retried = execute(&f, id, &observation.observation);
            assert_eq!(retried.code, ResultCode::Ok, "{retried:?}");
        } else {
            assert_eq!(result.code, ResultCode::Ok);
        }
        assert_eq!(reflog_len(&f), logs + 1);
    }
}

#[test]
fn spec_attribute_changes_preserve_trusted_unchanged_images_and_filter_changed_blobs() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge"] {
            for from in ["crlf", "lf"] {
                for changed_blob in [false, true] {
                    let f = Fixture::new(kind);
                    let c = f.context();
                    let root = c.workdir().unwrap();
                    fs::write(
                        root.join(".gitattributes"),
                        format!("ordinary text eol={from}\n"),
                    )
                    .unwrap();
                    commit(&c);
                    graph(&f, "equal");
                    fs::remove_file(root.join("ordinary")).unwrap();
                    c.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                        .unwrap();
                    let baseline = fs::read(root.join("ordinary")).unwrap();
                    assert_eq!(
                        baseline,
                        if from == "crlf" {
                            b"base\r\n".as_slice()
                        } else {
                            b"base\n"
                        }
                    );
                    if shape == "merge" {
                        fs::write(root.join("context-only"), "local\n").unwrap();
                        commit(&c);
                    }
                    let context = tip(&c);
                    let to = if from == "crlf" { "lf" } else { "crlf" };
                    fs::write(
                        f.root.path().join(".gitattributes"),
                        format!("ordinary text eol={to}\n"),
                    )
                    .unwrap();
                    if changed_blob {
                        fs::write(f.root.path().join("ordinary"), "new primary\n").unwrap();
                    }
                    let p = primary(&f);
                    let incoming = commit(&p);
                    p.reference("refs/remotes/origin/main", incoming, true, "fixture")
                        .unwrap();
                    let observation = f.observe();
                    let id = OperationId::new();
                    let logs = reflog_len(&f);
                    faults(
                        &f,
                        id,
                        &observation.observation,
                        FailurePoint::LocalSyncAfterCheckout,
                    );
                    let expected = if changed_blob {
                        if to == "crlf" {
                            b"new primary\r\n".to_vec()
                        } else {
                            b"new primary\n".to_vec()
                        }
                    } else {
                        baseline
                    };
                    assert_eq!(
                        fs::read(root.join("ordinary")).unwrap(),
                        expected,
                        "actual safe checkout image"
                    );
                    let result = execute(&f, id, &observation.observation);
                    assert_eq!(
                        result.code,
                        ResultCode::Ok,
                        "{kind:?} {shape} {from}->{to} changed={changed_blob}: {result:?}"
                    );
                    assert_eq!(fs::read(root.join("ordinary")).unwrap(), expected);
                    assert_eq!(reflog_len(&f), logs + 1);
                    refs_unchanged(&f, incoming);
                    if shape == "merge" {
                        assert_eq!(
                            c.find_commit(tip(&c))
                                .unwrap()
                                .parent_ids()
                                .collect::<Vec<_>>(),
                            [context, incoming]
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn spec_retained_image_does_not_accept_a_forged_ident_after_attribute_change() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let c = f.context();
    let root = c.workdir().unwrap();
    fs::write(root.join(".gitattributes"), "ordinary ident\n").unwrap();
    fs::write(root.join("ordinary"), "$Id$\n").unwrap();
    commit(&c);
    graph(&f, "equal");
    fs::remove_file(root.join("ordinary")).unwrap();
    c.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let baseline = fs::read(root.join("ordinary")).unwrap();
    fs::write(f.root.path().join(".gitattributes"), "ordinary -ident\n").unwrap();
    commit(&primary(&f));
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncAfterCheckout,
    );
    fs::write(root.join("ordinary"), "$Id: forged $\n").unwrap();
    let before = f.image();
    assert_ne!(
        execute(&f, id, &observation.observation).code,
        ResultCode::Ok
    );
    assert_eq!(before, f.image());
    fs::write(root.join("ordinary"), baseline).unwrap();
    assert_eq!(
        execute(&f, id, &observation.observation).code,
        ResultCode::Ok
    );
}

#[test]
fn spec_frozen_filter_inputs_do_not_follow_fresh_foreign_attributes() {
    for external in [false, true] {
        let f = Fixture::new(AuthoringKind::Document);
        let c = f.context();
        let root = c.workdir().unwrap();
        fs::write(root.join(".gitattributes"), "ordinary text eol=lf\n").unwrap();
        commit(&c);
        graph(&f, "equal");
        fs::write(f.root.path().join("ordinary"), "new primary\n").unwrap();
        let incoming = commit(&primary(&f));
        let observation = f.observe();
        let id = OperationId::new();
        faults(
            &f,
            id,
            &observation.observation,
            FailurePoint::LocalSyncAfterCandidate,
        );
        let path = if external {
            fs::create_dir_all(c.commondir().join("info")).unwrap();
            c.commondir().join("info/attributes")
        } else {
            root.join(".gitattributes")
        };
        let previous = fs::read(&path).ok();
        fs::write(&path, "ordinary text eol=crlf\n").unwrap();
        let before = f.image();
        let logs = reflog_len(&f);
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.code, ResultCode::ExternalChange, "{result:?}");
        assert_eq!(before, f.image());
        assert_eq!(reflog_len(&f), logs);
        assert_eq!(fs::read(&path).unwrap(), b"ordinary text eol=crlf\n");
        if let Some(bytes) = previous {
            fs::write(&path, bytes).unwrap();
        } else {
            fs::remove_file(&path).unwrap();
        }
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::Ok
        );
        assert_eq!(tip(&c), incoming);
        assert_eq!(reflog_len(&f), logs + 1);
    }
}

fn edit_image_manifest(f: &Fixture, id: OperationId, edit: impl FnOnce(&mut serde_json::Value)) {
    let bytes: Vec<u8> = f
        .db()
        .query_row(
            "SELECT manifest FROM local_context_sync_images WHERE operation_ulid=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    edit(&mut value);
    f.db()
        .execute(
            "UPDATE local_context_sync_images SET manifest=?2 WHERE operation_ulid=?1",
            rusqlite::params![id.to_string(), serde_json::to_vec(&value).unwrap()],
        )
        .unwrap();
}

#[test]
fn spec_image_manifest_loss_corruption_and_version_mismatch_refuse() {
    for corrupt in [
        "missing",
        "version",
        "coarse_version",
        "missing_path",
        "baseline_v2_missing",
        "legacy_version_with_v2",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let observation = f.observe();
        let id = OperationId::new();
        let stop = if matches!(corrupt, "baseline_v2_missing" | "legacy_version_with_v2") {
            FailurePoint::LocalSyncAfterCheckout
        } else {
            FailurePoint::LocalSyncAfterCandidate
        };
        faults(&f, id, &observation.observation, stop);
        let before = f.image();
        let logs = reflog_len(&f);
        match corrupt {
            "missing" => {
                f.db()
                    .execute(
                        "DELETE FROM local_context_sync_images WHERE operation_ulid=?1",
                        [id.to_string()],
                    )
                    .unwrap();
            }
            "baseline_v2_missing" => {
                f.db().execute("UPDATE local_context_sync_images SET baseline_v2=NULL WHERE operation_ulid=?1",[id.to_string()]).unwrap();
            }
            "legacy_version_with_v2" => {
                edit_image_manifest(&f, id, |v| v["coarse_version"] = 1.into())
            }
            "missing_path" => edit_image_manifest(&f, id, |v| {
                v["paths"].as_array_mut().unwrap().remove(0);
            }),
            field => edit_image_manifest(&f, id, |v| v[field] = 99.into()),
        }
        let result = execute(&f, id, &observation.observation);
        assert_eq!(
            result.code,
            ResultCode::RecoveryRequired,
            "{corrupt}: {result:?}"
        );
        assert_eq!(before, f.image());
        assert_eq!(logs, reflog_len(&f));
    }
}

#[test]
fn spec_image_manifest_stores_metadata_and_never_rebuilds_lost_authority() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let c = f.context();
    let secret = "PRIVATE-FROZEN-IMAGE-SOURCE-NEVER-IN-JOURNAL";
    fs::write(c.workdir().unwrap().join("ordinary"), secret).unwrap();
    commit(&c);
    graph(&f, "fast_forward");
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncAfterIntent,
    );
    let bytes: Vec<u8> = f
        .db()
        .query_row(
            "SELECT manifest FROM local_context_sync_images WHERE operation_ulid=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!bytes.windows(secret.len()).any(|b| b == secret.as_bytes()));
    assert_eq!(
        fs::read(c.workdir().unwrap().join("ordinary")).unwrap(),
        secret.as_bytes()
    );
    f.db()
        .execute(
            "DELETE FROM local_context_sync_images WHERE operation_ulid=?1",
            [id.to_string()],
        )
        .unwrap();
    let before = f.image();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        run::execute(
            &reopened,
            &f.repo,
            f.kind,
            &f.item,
            id,
            &observation.observation
        )
        .code,
        ResultCode::RecoveryRequired
    );
    assert_eq!(before, f.image());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM local_context_sync_images WHERE operation_ulid=?1",
                [id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn spec_coarse_versions_are_separate_and_valid_legacy_bytes_are_not_reinterpreted() {
    let f = Fixture::new(AuthoringKind::Ticket);
    graph(&f, "fast_forward");
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncBeforeCheckoutBackend,
    );
    let evidence = super::recover::Execution::read(&f.db(), id).unwrap();
    assert_ne!(
        evidence.checkout_baseline,
        evidence.images.as_ref().unwrap().baseline_v2
    );
    edit_image_manifest(&f, id, |v| v["coarse_version"] = 1.into());
    f.db()
        .execute(
            "UPDATE local_context_sync_images SET baseline_v2=NULL WHERE operation_ulid=?1",
            [id.to_string()],
        )
        .unwrap();
    let row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    let evidence = super::recover::Execution::read(&f.db(), id).unwrap();
    let git = f.context();
    {
        let owner = super::recover::Owner {
            request: None,
            service: &f.service,
            repo: &f.repo,
            lease: repository_lease(&git, f.root.path(), RepositoryOperation::ContextSyncLocal)
                .unwrap(),
            git: f.context(),
            row,
            known_execution: Some(evidence),
            performed: super::recover::Performed::default(),
        };
        assert!(
            !super::recover::checkout_progress(&owner).unwrap(),
            "do not compare a legacy v1 hash with a v2 image"
        );
    }
    assert_eq!(
        execute(&f, id, &observation.observation).code,
        ResultCode::Ok
    );
}

#[test]
fn spec_missing_after_only_image_refuses_before_checkout() {
    let f = Fixture::new(AuthoringKind::Ticket);
    graph(&f, "fast_forward");
    let observation = f.observe();
    let id = OperationId::new();
    faults(
        &f,
        id,
        &observation.observation,
        FailurePoint::LocalSyncAfterCandidate,
    );
    edit_image_manifest(&f, id, |v| {
        v["paths"]
            .as_array_mut()
            .unwrap()
            .retain(|p| p["path"] != "incoming")
    });
    let image = f.image();
    let logs = reflog_len(&f);
    let result = execute(&f, id, &observation.observation);
    assert_ne!(result.code, ResultCode::Ok);
    assert!(!result.effects.is_durable(), "{result:?}");
    assert_eq!(image, f.image());
    assert_eq!(logs, reflog_len(&f));
    assert!(!f.context().workdir().unwrap().join("incoming").exists());
}

#[test]
fn spec_both_manifest_sides_bind_before_reconciliation_or_owner_change() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for corrupt in ["before_oid", "before_mode", "before_path", "after_path"] {
            let f = Fixture::new(kind);
            graph(&f, "equal");
            let c = f.context();
            let context = tip(&c);
            let old_bytes = fs::read(c.workdir().unwrap().join("ordinary")).unwrap();
            fs::write(f.root.path().join("ordinary"), "new primary bytes\n").unwrap();
            let incoming = commit(&primary(&f));
            primary(&f)
                .reference("refs/remotes/origin/main", incoming, true, "fixture")
                .unwrap();
            let observation = f.observe();
            let id = OperationId::new();
            let stop = if corrupt == "after_path" {
                FailurePoint::LocalSyncAfterIntent
            } else {
                FailurePoint::LocalSyncAfterCheckout
            };
            faults(&f, id, &observation.observation, stop);
            if corrupt != "after_path" {
                // Real owned checkout: candidate index, still the recorded parent
                // ref. The composition below changes ONLY before.oid in evidence.
                assert_eq!(
                    c.index()
                        .unwrap()
                        .get_path(Path::new("ordinary"), 0)
                        .unwrap()
                        .id,
                    primary(&f)
                        .head()
                        .unwrap()
                        .peel_to_tree()
                        .unwrap()
                        .get_path(Path::new("ordinary"))
                        .unwrap()
                        .id()
                );
            }
            edit_image_manifest(&f, id, |value| {
                let path = value["paths"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["path"] == "ordinary")
                    .unwrap();
                match corrupt {
                    "before_oid" => path["before"]["oid"] = path["after"]["oid"].clone(),
                    "before_mode" => path["before"]["mode"] = 0o100755.into(),
                    "before_path" => path["before"] = serde_json::Value::Null,
                    "after_path" => path["after"] = serde_json::Value::Null,
                    _ => unreachable!(),
                }
            });
            if corrupt == "before_oid" {
                fs::write(c.workdir().unwrap().join("ordinary"), &old_bytes).unwrap();
            }
            let rows = local_authority_rows(&f);
            let image = f.image();
            let logs = reflog_len(&f);
            let manifest: Vec<u8> = f
                .db()
                .query_row(
                    "SELECT manifest FROM local_context_sync_images WHERE operation_ulid=?1",
                    [id.to_string()],
                    |r| r.get(0),
                )
                .unwrap();
            let result = execute(&f, id, &observation.observation);
            assert_eq!(
                result.code,
                ResultCode::RecoveryRequired,
                "{kind:?} {corrupt}: {result:?}"
            );
            assert_eq!(
                rows,
                local_authority_rows(&f),
                "no takeover, new intent or settlement on invalid bindings"
            );
            assert_eq!(
                manifest,
                f.db()
                    .query_row(
                        "SELECT manifest FROM local_context_sync_images WHERE operation_ulid=?1",
                        [id.to_string()],
                        |r| r.get::<_, Vec<u8>>(0)
                    )
                    .unwrap()
            );
            assert_eq!(image, f.image());
            assert_eq!(tip(&c), context);
            assert_eq!(reflog_len(&f), logs);
            refs_unchanged(&f, incoming);
        }
    }
}

#[test]
fn spec_complete_bindings_match_git_directory_and_file_ordering() {
    let f = Fixture::new(AuthoringKind::Document);
    let c = f.context();
    let root = c.workdir().unwrap();
    fs::create_dir_all(root.join("a/sub")).unwrap();
    for path in ["a.c", "a0", "a/sub.c", "a/sub/file", "a/sub0", "a/é"] {
        fs::write(root.join(path), format!("{path}\n")).unwrap();
    }
    commit(&c);
    let (_, incoming) = graph(&f, "fast_forward");
    let observation = f.observe();
    let id = OperationId::new();
    let result = execute(&f, id, &observation.observation);
    assert_eq!(result.code, ResultCode::Ok, "{result:?}");
    assert_eq!(tip(&c), incoming);
    assert_eq!(execute(&f, id, &observation.observation), result);
}

fn ignored_attribute_fixture(f: &Fixture, nested: bool) -> (String, std::path::PathBuf) {
    let c = f.context();
    fs::create_dir_all(c.commondir().join("info")).unwrap();
    let exclude = c.commondir().join("info/exclude");
    let mut contents = fs::read_to_string(&exclude).unwrap_or_default();
    contents.push_str("\n.gitattributes\nignored.txt\nscratch/\n");
    fs::write(exclude, contents).unwrap();
    let incoming = if nested {
        "nested/deeper/incoming"
    } else {
        "incoming"
    }
    .to_owned();
    let relative = if nested {
        "nested/.gitattributes"
    } else {
        ".gitattributes"
    };
    let attrs = c.workdir().unwrap().join(relative);
    fs::create_dir_all(attrs.parent().unwrap()).unwrap();
    (incoming, attrs)
}

#[test]
fn quality_ignored_worktree_attributes_refuse_before_acceptance() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge"] {
            for nested in [false, true] {
                let f = Fixture::new(kind);
                graph(&f, shape);
                let (incoming_path, attrs) = ignored_attribute_fixture(&f, nested);
                let file = f.root.path().join(&incoming_path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(&file, "committed primary\n").unwrap();
                let p = primary(&f);
                let incoming = commit(&p);
                p.reference("refs/remotes/origin/main", incoming, true, "fixture")
                    .unwrap();
                fs::write(&attrs, "* text eol=crlf\n").unwrap();
                let c = f.context();
                let blob = p
                    .find_commit(incoming)
                    .unwrap()
                    .tree()
                    .unwrap()
                    .get_path(Path::new(&incoming_path))
                    .unwrap()
                    .id();
                assert_eq!(p.find_blob(blob).unwrap().content(), b"committed primary\n");
                assert_eq!(
                    integration::filtered::checkout_bytes(&c, incoming, blob, &incoming_path)
                        .unwrap(),
                    b"committed primary\r\n",
                    "the real worktree source changes filtering"
                );
                let observation = f.observe();
                let id = OperationId::new();
                let image = f.image();
                let logs = reflog_len(&f);
                let result = execute(&f, id, &observation.observation);
                no_owner(&f, id);
                assert_eq!(result.code, ResultCode::RecoveryRequired);
                assert_eq!(result.reason, Reason::AuthorityMissing);
                assert!(!result.effects.is_durable());
                assert_eq!(image, f.image());
                assert_eq!(logs, reflog_len(&f));
                refs_unchanged(&f, incoming);
                assert_eq!(fs::read(&attrs).unwrap(), b"* text eol=crlf\n");
                assert!(!c.workdir().unwrap().join(&incoming_path).exists());
                assert!(matches!(
                    f.service
                        .refresh_repository(RefreshRepositoryRequest {
                            root: f.root.path().into(),
                            operation_id: OperationId::new()
                        })
                        .unwrap(),
                    RefreshOutcome::Refreshed { .. }
                ));
                // Independent positive control AFTER all refusal assertions:
                // actual safe checkout reads the ignored worktree attributes.
                c.checkout_tree(
                    c.find_commit(incoming).unwrap().as_object(),
                    Some(git2::build::CheckoutBuilder::new().safe()),
                )
                .unwrap();
                assert_eq!(
                    fs::read(c.workdir().unwrap().join(&incoming_path)).unwrap(),
                    b"committed primary\r\n"
                );
            }
        }
    }
}

#[test]
fn quality_new_worktree_attribute_sources_cannot_change_accepted_execution() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for nested in [false, true] {
            for stop in [
                FailurePoint::LocalSyncAfterIntent,
                FailurePoint::LocalSyncAfterCheckout,
            ] {
                let f = Fixture::new(kind);
                graph(&f, "fast_forward");
                let (incoming_path, attrs) = ignored_attribute_fixture(&f, nested);
                let file = f.root.path().join(&incoming_path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(&file, "committed primary\n").unwrap();
                let incoming = commit(&primary(&f));
                let observation = f.observe();
                let id = OperationId::new();
                faults(&f, id, &observation.observation, stop);
                fs::write(&attrs, "* text eol=crlf\n").unwrap();
                let image = f.image();
                let logs = reflog_len(&f);
                let result = execute(&f, id, &observation.observation);
                assert_eq!(
                    result.code,
                    ResultCode::RecoveryRequired,
                    "{stop:?}: {result:?}"
                );
                assert_eq!(result.reason, Reason::AuthorityMissing);
                assert_eq!(image, f.image());
                assert_eq!(reflog_len(&f), logs);
                assert_eq!(fs::read(&attrs).unwrap(), b"* text eol=crlf\n");
                assert_eq!(
                    result.effects.write,
                    if stop == FailurePoint::LocalSyncAfterCheckout {
                        WriteEffect::Written
                    } else {
                        WriteEffect::Unchanged
                    }
                );
                fs::remove_file(&attrs).unwrap();
                assert_eq!(
                    execute(&f, id, &observation.observation).code,
                    ResultCode::Ok
                );
                assert_eq!(tip(&f.context()), incoming);
                assert_eq!(reflog_len(&f), logs + 1);
            }
        }
    }
}

#[test]
fn quality_ordinary_ignored_files_and_irrelevant_attribute_directories_remain_allowed() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["fast_forward", "merge"] {
            let f = Fixture::new(kind);
            graph(&f, shape);
            ignored_attribute_fixture(&f, false);
            let c = f.context();
            let root = c.workdir().unwrap();
            fs::write(root.join("ignored.txt"), "private ignored bytes").unwrap();
            fs::create_dir_all(root.join("scratch")).unwrap();
            fs::write(root.join("scratch/.gitattributes"), "* text eol=crlf\n").unwrap();
            let observation = f.observe();
            let result = execute(&f, OperationId::new(), &observation.observation);
            assert_eq!(result.code, ResultCode::Ok, "{result:?}");
            assert_eq!(
                fs::read(root.join("ignored.txt")).unwrap(),
                b"private ignored bytes"
            );
            assert_eq!(
                fs::read(root.join("scratch/.gitattributes")).unwrap(),
                b"* text eol=crlf\n"
            );
            assert_eq!(
                fs::read(root.join("incoming")).unwrap(),
                b"committed primary\n"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn quality_ignored_symlink_attribute_sources_are_preserved_and_refused() {
    use std::os::unix::fs::symlink;
    for nested in [false, true] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, "fast_forward");
        let (incoming_path, attrs) = ignored_attribute_fixture(&f, nested);
        let file = f.root.path().join(&incoming_path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "committed primary\n").unwrap();
        commit(&primary(&f));
        let observation = f.observe();
        let outside = f.data.path().join("outside-attributes");
        fs::write(&outside, "* text eol=crlf\n").unwrap();
        symlink(&outside, &attrs).unwrap();
        let id = OperationId::new();
        let image = f.image();
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.code, ResultCode::RecoveryRequired, "{result:?}");
        no_owner(&f, id);
        assert_eq!(image, f.image());
        assert_eq!(fs::read_link(&attrs).unwrap(), outside);
        assert_eq!(fs::read(outside).unwrap(), b"* text eol=crlf\n");
    }
}

#[test]
fn quality_worktree_attribute_sources_are_rechecked_at_git_effect_boundaries() {
    for (shape, point) in [
        ("merge", FailurePoint::LocalSyncBeforeObjectImport),
        ("fast_forward", FailurePoint::LocalSyncAfterCandidate),
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, shape);
        let (_, attrs) = ignored_attribute_fixture(&f, false);
        let observation = f.observe();
        let id = OperationId::new();
        let created = attrs.clone();
        run::BOUNDARY_HOOK.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move |current| {
                if current == point {
                    fs::write(&created, "* text eol=crlf\n").unwrap();
                }
            }))
        });
        let image = f.image();
        let logs = reflog_len(&f);
        let result = execute(&f, id, &observation.observation);
        run::BOUNDARY_HOOK.with(|slot| slot.borrow_mut().take());
        assert_eq!(
            result.code,
            ResultCode::RecoveryRequired,
            "{point:?}: {result:?}"
        );
        assert_eq!(result.reason, Reason::AuthorityMissing);
        assert!(!result.effects.is_durable());
        assert_eq!(image, f.image());
        assert_eq!(logs, reflog_len(&f));
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        if shape == "merge" {
            assert!(
                f.context().find_commit(row.candidate.unwrap()).is_err(),
                "no candidate object may be imported after the source changes"
            );
        }
        fs::remove_file(attrs).unwrap();
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::Ok
        );
        assert_eq!(reflog_len(&f), logs + 1);
    }
}

#[test]
fn quality_untracked_attribute_sources_remain_dirty_and_noop_needs_no_checkout_source() {
    let f = Fixture::new(AuthoringKind::Document);
    graph(&f, "fast_forward");
    let observation = f.observe();
    let id = OperationId::new();
    let c = f.context();
    fs::write(
        c.workdir().unwrap().join(".gitattributes"),
        "* text eol=crlf\n",
    )
    .unwrap();
    let image = f.image();
    assert_eq!(
        execute(&f, id, &observation.observation).code,
        ResultCode::WorktreeNotClean
    );
    no_owner(&f, id);
    assert_eq!(image, f.image());
    for shape in ["equal", "contained"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        graph(&f, shape);
        let (_, attrs) = ignored_attribute_fixture(&f, false);
        fs::write(&attrs, "* text eol=crlf\n").unwrap();
        let observation = f.observe();
        let image = f.image();
        let logs = reflog_len(&f);
        assert_eq!(
            execute(&f, OperationId::new(), &observation.observation).code,
            ResultCode::AlreadyApplied
        );
        assert_eq!(image, f.image());
        assert_eq!(logs, reflog_len(&f));
        assert_eq!(fs::read(attrs).unwrap(), b"* text eol=crlf\n");
    }
}
