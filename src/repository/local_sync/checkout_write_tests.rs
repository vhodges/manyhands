//! A real OS write failure inside libgit2's FIRST file, not a synthetic partial
//! checkout or a completed-path callback. All process-global limits/signals are
//! confined to an isolated child executable (Unix; Linux execution evidence).
use super::*;
use std::cell::Cell;

extern "C" fn exit_in_write(_: libc::c_int) {
    // SAFETY: _exit is async-signal-safe and runs no Rust/C destructors.
    unsafe {
        libc::_exit(FAILURE_POINT_EXIT_STATUS);
    }
}

#[test]
fn first_write_child() {
    let Ok(root) = std::env::var("MANYHANDS_FIRST_WRITE_ROOT") else {
        return;
    };
    let data = std::env::var("MANYHANDS_FIRST_WRITE_DATA").unwrap();
    let kind = if std::env::var("MANYHANDS_FIRST_WRITE_KIND").unwrap() == "ticket" {
        AuthoringKind::Ticket
    } else {
        AuthoringKind::Document
    };
    let item = std::env::var("MANYHANDS_FIRST_WRITE_ITEM")
        .unwrap()
        .parse()
        .unwrap();
    let id = OperationId::parse(&std::env::var("MANYHANDS_FIRST_WRITE_ID").unwrap()).unwrap();
    let observation = std::env::var("MANYHANDS_FIRST_WRITE_OBSERVATION").unwrap();
    let size: libc::rlim_t = std::env::var("MANYHANDS_FIRST_WRITE_LIMIT")
        .unwrap()
        .parse()
        .unwrap();
    let exit = std::env::var("MANYHANDS_FIRST_WRITE_EXIT").unwrap() == "true";
    let service = RepositoryService::open_at(Path::new(&data)).unwrap();
    let repo = service.resolve_repository(Path::new(&root)).unwrap();
    let original = Cell::new(None);
    run::BOUNDARY_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |point| {
            if point == FailurePoint::LocalSyncBeforeCheckoutBackend {
                let mut old = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                // SAFETY: valid pointers; this child runs one selected test. Arm only
                // after all SQLite intents/authority checks and just before checkout.
                unsafe {
                    assert_eq!(libc::getrlimit(libc::RLIMIT_FSIZE, &mut old), 0);
                    let signal = libc::signal(
                        libc::SIGXFSZ,
                        if exit {
                            exit_in_write as *const () as libc::sighandler_t
                        } else {
                            libc::SIG_IGN
                        },
                    );
                    assert_ne!(signal, libc::SIG_ERR);
                    original.set(Some((old.rlim_cur, old.rlim_max, signal)));
                    assert_eq!(
                        libc::setrlimit(
                            libc::RLIMIT_FSIZE,
                            &libc::rlimit {
                                rlim_cur: size,
                                rlim_max: old.rlim_max
                            }
                        ),
                        0
                    );
                }
            } else if point == FailurePoint::LocalSyncAfterCheckoutBackend {
                let (cur, max, signal) = original.take().expect("backend was entered");
                // SAFETY: restore the child-only limit/handler before any SQL write.
                unsafe {
                    assert_eq!(
                        libc::setrlimit(
                            libc::RLIMIT_FSIZE,
                            &libc::rlimit {
                                rlim_cur: cur,
                                rlim_max: max
                            }
                        ),
                        0
                    );
                    libc::signal(libc::SIGXFSZ, signal);
                }
            }
        }))
    });
    let result = run::execute(&service, &repo, kind, &item, id, &observation);
    run::BOUNDARY_HOOK.with(|slot| slot.borrow_mut().take());
    assert!(
        !exit,
        "no-Drop child must exit from the actual failing write"
    );
    assert_eq!(result.reason, Reason::AmbiguousBackend);
    assert_eq!(result.effects.write, WriteEffect::Written, "{result:?}");
    assert_eq!(result.effects.checkpoint, CheckpointEffect::Unchanged);
    assert_eq!(result.outcome(), Outcome::Partial);
}

#[test]
fn quality_first_file_partial_write_is_partial_without_granting_resume_authority() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for exit in [true, false] {
            for size in [0, 3] {
                let f = Fixture::new(kind);
                let c = f.context();
                let root = c.workdir().unwrap();
                fs::write(root.join("000-first"), "old first contents\n").unwrap();
                fs::write(root.join("zzz-second"), "old second contents\n").unwrap();
                commit(&c);
                graph(&f, "equal");
                let context = tip(&c);
                let p = primary(&f);
                fs::write(f.root.path().join("000-first"), "new first long contents\n").unwrap();
                fs::write(f.root.path().join("zzz-second"), "new second contents\n").unwrap();
                let incoming = commit(&p);
                p.reference("refs/remotes/origin/main", incoming, true, "fixture")
                    .unwrap();
                let observation = f.observe();
                let id = OperationId::new();
                let image = f.image();
                let logs = reflog_len(&f);
                let output=std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact","repository::local_sync::run_tests::checkout_write_tests::first_write_child","--nocapture"])
                    .env("MANYHANDS_FIRST_WRITE_ROOT",f.root.path()).env("MANYHANDS_FIRST_WRITE_DATA",f.data.path())
                    .env("MANYHANDS_FIRST_WRITE_KIND",authoring_kind_segment(&kind)).env("MANYHANDS_FIRST_WRITE_ITEM",f.item.to_string())
                    .env("MANYHANDS_FIRST_WRITE_ID",id.to_string()).env("MANYHANDS_FIRST_WRITE_OBSERVATION",&observation.observation)
                    .env("MANYHANDS_FIRST_WRITE_LIMIT",size.to_string()).env("MANYHANDS_FIRST_WRITE_EXIT",exit.to_string()).output().unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(if exit { FAILURE_POINT_EXIT_STATUS } else { 0 }),
                    "{}",
                    String::from_utf8_lossy(&output.stdout)
                );
                // These snapshots prove truncation/prefix bytes in the FIRST
                // file, with no completed path, no index update and no ref move.
                let partial = fs::read(root.join("000-first")).unwrap();
                assert_eq!(partial, &b"new first long contents\n"[..size]);
                assert_eq!(
                    fs::read(root.join("zzz-second")).unwrap(),
                    b"old second contents\n"
                );
                assert_eq!(f.image(), image);
                assert_eq!(tip(&c), context);
                assert_eq!(reflog_len(&f), logs);
                let refused = execute(&f, id, &observation.observation);
                assert_eq!(refused.reason, Reason::AmbiguousBackend);
                assert_eq!(refused.effects.write, WriteEffect::Written, "{refused:?}");
                assert_eq!(refused.effects.checkpoint, CheckpointEffect::Unchanged);
                assert_eq!(refused.outcome(), Outcome::Partial);
                assert_eq!(fs::read(root.join("000-first")).unwrap(), partial);
                assert_eq!(f.image(), image);
                // Explicit operator baseline restoration, not automatic repair.
                fs::write(root.join("000-first"), "old first contents\n").unwrap();
                assert_eq!(
                    execute(&f, id, &observation.observation).code,
                    ResultCode::Ok
                );
                assert_eq!(tip(&c), incoming);
                assert_eq!(reflog_len(&f), logs + 1);
                refs_unchanged(&f, incoming);
            }
        }
    }
}

#[test]
fn spec_mode_only_no_drop_is_partial_and_requires_complete_image_proof() {
    use std::os::unix::fs::PermissionsExt;
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let c = f.context();
        let path = c.workdir().unwrap().join("mode-only");
        c.config().unwrap().set_bool("core.filemode", true).unwrap();
        fs::write(&path, "unchanged bytes\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        commit(&c);
        graph(&f, "equal");
        let context = tip(&c);
        let p = primary(&f);
        let parent = p.find_commit(tip(&p)).unwrap();
        let mut index = p.index().unwrap();
        let mut entry = index.get_path(Path::new("mode-only"), 0).unwrap();
        entry.mode = 0o100755;
        index.add(&entry).unwrap();
        index.write().unwrap();
        let tree = p.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = p.signature().unwrap();
        let incoming = p
            .commit(Some("HEAD"), &sig, &sig, "mode only", &tree, &[&parent])
            .unwrap();
        p.reference("refs/remotes/origin/main", incoming, true, "fixture")
            .unwrap();
        let observation = f.observe();
        let id = OperationId::new();
        let image = f.image();
        let logs = reflog_len(&f);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "repository::local_sync::run_tests::local_sync_no_drop_child",
                "--nocapture",
            ])
            .env("MANYHANDS_LOCAL_SYNC_CHILD_ROOT", f.root.path())
            .env("MANYHANDS_LOCAL_SYNC_CHILD_DATA", f.data.path())
            .env("MANYHANDS_LOCAL_SYNC_CHILD_POINT", "mode")
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
        assert_ne!(fs::metadata(&path).unwrap().permissions().mode() & 0o111, 0);
        assert_eq!(fs::read(&path).unwrap(), b"unchanged bytes\n");
        assert_eq!(image, f.image());
        assert_eq!(tip(&c), context);
        assert_eq!(reflog_len(&f), logs);
        {
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            let evidence = super::super::recover::Execution::read(&f.db(), id).unwrap();
            let owner = super::super::recover::Owner {
                request: None,
                service: &f.service,
                repo: &f.repo,
                git: f.context(),
                lease: repository_lease(&c, f.root.path(), RepositoryOperation::ContextSyncLocal)
                    .unwrap(),
                row,
                known_execution: Some(evidence),
                performed: super::super::recover::Performed::default(),
            };
            assert!(
                super::super::recover::checkout_progress(&owner).unwrap(),
                "actual mode-only filesystem effect must be observed"
            );
        }
        let result = execute(&f, id, &observation.observation);
        assert_eq!(result.reason, Reason::AmbiguousBackend);
        assert_eq!(result.effects.write, WriteEffect::Written);
        assert_eq!(result.effects.checkpoint, CheckpointEffect::Unchanged);
        assert_eq!(result.outcome(), Outcome::Partial);
        assert_eq!(image, f.image());
        assert_ne!(fs::metadata(&path).unwrap().permissions().mode() & 0o111, 0);
        // Explicit operator restores the baseline mode; no executor chmod/reset.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            execute(&f, id, &observation.observation).code,
            ResultCode::Ok
        );
        assert_eq!(tip(&c), incoming);
        assert_eq!(reflog_len(&f), logs + 1);
        refs_unchanged(&f, incoming);
    }
}
