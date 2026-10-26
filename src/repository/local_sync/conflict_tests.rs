use super::{
    record::LocalSyncState,
    record_tests::{Fixture, commit},
    run,
};
use crate::{repository::*, results::*};
use std::{fs, path::PathBuf};

pub(super) fn start(f: &Fixture) -> OperationId {
    let id = OperationId::new();
    let result = run::execute(
        &f.service,
        &f.repo,
        f.kind,
        &f.item,
        id,
        &f.observe().observation,
    );
    assert_eq!(
        result.state,
        Some(LocalSyncState::ConflictPending),
        "{result:?}"
    );
    id
}

#[test]
fn immutable_tokens_reopen_and_complete_resolution_preserves_exact_source() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let (f, paths) = fixture(kind);
        let id = start(&f);
        let inspection = super::conflict::inspect(&f.service, &f.repo, id).unwrap();
        assert!(inspection.can_start_resolution);
        assert_eq!(
            inspection.eligibility,
            integration::validate::ConflictEligibility::EligibleCanonical
        );
        assert_eq!(inspection.paths.len(), 2);
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        let mut sources = Vec::new();
        for entry in &inspection.paths {
            let token: String =
                serde_json::from_str(&serde_json::to_string(&entry.token).unwrap()).unwrap();
            let sides = super::conflict::read(&reopened, &f.repo, id, &token).unwrap();
            assert!(!sides.historical);
            assert!(sides.base.as_ref().unwrap().ends_with("\n"));
            assert!(sides.incoming.as_ref().unwrap().ends_with("incoming\n"));
            sources.push(super::resolve::Source {
                token,
                source: sides.local.unwrap().replace('\n', "\r\n"),
            });
        }
        let request = OperationId::new();
        let result = super::resolve::execute(
            &reopened,
            &f.repo,
            id,
            request,
            &inspection.observation,
            &sources,
        );
        assert_eq!(result.code, ResultCode::Ok, "{result:?}");
        assert_eq!(result.state, Some(LocalSyncState::Complete));
        let git = f.context();
        let commit = git.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(commit.parent_count(), 2);
        for path in paths {
            let bytes = fs::read(git.workdir().unwrap().join(&path)).unwrap();
            assert!(bytes.windows(2).any(|b| b == b"\r\n"));
            let oid = commit.tree().unwrap().get_path(&path).unwrap().id();
            assert_eq!(git.find_blob(oid).unwrap().content(), bytes);
        }
        let image = f.image();
        assert_eq!(
            super::resolve::execute(
                &reopened,
                &f.repo,
                id,
                request,
                &inspection.observation,
                &sources
            ),
            result
        );
        assert_eq!(image, f.image());
        assert!(super::conflict::read(&reopened, &f.repo, id, &sources[0].token).is_err());
    }
}

pub(super) fn input(f: &Fixture, id: OperationId) -> (String, Vec<super::resolve::Source>) {
    let inspection = super::conflict::inspect(&f.service, &f.repo, id).unwrap();
    let sources = inspection
        .paths
        .iter()
        .map(|e| super::resolve::Source {
            token: e.token.clone(),
            source: super::conflict::read(&f.service, &f.repo, id, &e.token)
                .unwrap()
                .local
                .unwrap(),
        })
        .collect();
    (inspection.observation, sources)
}

#[test]
fn resolution_authority_loss_and_missing_companion_never_authorize_writes() {
    for corrupt in ["owner", "manifest", "control", "marker"] {
        let (f, paths) = fixture(AuthoringKind::Document);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        if corrupt == "owner" {
            let dbpath = f.data.path().join(REGISTRY_FILE);
            super::resolve::HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(move |point| {
                    if point == super::resolve::Point::Path(0) {
                        rusqlite::Connection::open(&dbpath)
                            .unwrap()
                            .execute(
                                "UPDATE local_context_sync_records SET owner_epoch=owner_epoch+1",
                                [],
                            )
                            .unwrap();
                    }
                    Ok(())
                }))
            });
        } else {
            if corrupt == "control" {
                super::resolve::HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(|point| {
                        if point == super::resolve::Point::Accepted {
                            Err(std::io::Error::other("fixture"))
                        } else {
                            Ok(())
                        }
                    }))
                });
                super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
                super::resolve::HOOK.with(|h| h.borrow_mut().take());
            }
            let sql = match corrupt {
                "manifest" => "DELETE FROM local_context_sync_conflicts",
                "control" => "DELETE FROM local_context_sync_resolution_cancellation",
                "marker" => {
                    "DELETE FROM registry_migrations WHERE name='local_context_sync_conflicts_v7'"
                }
                _ => unreachable!(),
            };
            f.db().execute(sql, []).unwrap();
        }
        let before = paths
            .iter()
            .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
            .collect::<Vec<_>>();
        let result =
            super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
        super::resolve::HOOK.with(|h| h.borrow_mut().take());
        assert_ne!(result.code, ResultCode::Ok, "{corrupt}");
        assert_eq!(
            before,
            paths
                .iter()
                .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
                .collect::<Vec<_>>(),
            "{corrupt}"
        );
    }
}

#[test]
fn complete_preflight_refuses_every_invalid_set_before_any_path_write() {
    for invalid in [
        "missing",
        "duplicate",
        "foreign",
        "tampered",
        "canonical",
        "stale",
    ] {
        let (f, paths) = fixture(AuthoringKind::Ticket);
        let id = start(&f);
        let (observation, mut sources) = input(&f, id);
        match invalid {
            "missing" => {
                sources.pop();
            }
            "duplicate" => {
                sources[1].token = sources[0].token.clone();
            }
            "foreign" => {
                sources[1].token = format!("local-conflict-v1:{}", "a".repeat(64));
            }
            "tampered" => {
                sources[1].token = "../../ordinary".into();
            }
            "canonical" => {
                sources[1].source = "not canonical".into();
            }
            "stale" => {
                fs::write(
                    f.context().workdir().unwrap().join(&paths[0]),
                    "caller edit",
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = f.image();
        let files = paths
            .iter()
            .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
            .collect::<Vec<_>>();
        let result = super::resolve::execute(
            &f.service,
            &f.repo,
            id,
            OperationId::new(),
            &observation,
            &sources,
        );
        assert_ne!(result.code, ResultCode::Ok, "{invalid}");
        if invalid == "tampered" {
            assert_eq!(result.code, ResultCode::InvalidInput);
        }
        assert_eq!(before, f.image(), "{invalid}");
        assert_eq!(
            files,
            paths
                .iter()
                .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
                .collect::<Vec<_>>(),
            "{invalid}"
        );
        assert!(
            f.service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap()
                .active_resolution
                .is_none(),
            "{invalid}"
        );
    }
}

#[test]
fn mixed_conflict_requires_exact_ordered_clean_external_repair() {
    for wrong in [false, true] {
        let (f, paths) = fixture(AuthoringKind::Ticket);
        for (git, source) in [
            (f.context(), "local code"),
            (
                git2::Repository::open(f.root.path()).unwrap(),
                "incoming code",
            ),
        ] {
            fs::write(git.workdir().unwrap().join("ordinary"), source).unwrap();
            commit(&git);
        }
        let id = start(&f);
        let inspection = super::conflict::inspect(&f.service, &f.repo, id).unwrap();
        assert_eq!(
            inspection.eligibility,
            integration::validate::ConflictEligibility::ExternalResolutionRequired
        );
        assert!(!inspection.can_start_resolution);
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        let git = f.context();
        for path in &paths {
            let text = git
                .find_commit(row.context_parent)
                .unwrap()
                .tree()
                .unwrap()
                .get_path(path)
                .unwrap()
                .id();
            fs::write(
                git.workdir().unwrap().join(path),
                git.find_blob(text).unwrap().content(),
            )
            .unwrap();
        }
        fs::write(git.workdir().unwrap().join("ordinary"), "external repair").unwrap();
        let mut index = git.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git.signature().unwrap();
        let c = git.find_commit(row.context_parent).unwrap();
        let p = git.find_commit(row.primary_parent).unwrap();
        let parents = if wrong { vec![&p, &c] } else { vec![&c, &p] };
        let oid = git
            .commit(None, &signature, &signature, "external", &tree, &parents)
            .unwrap();
        git.reference(&row.context_ref, oid, true, "external")
            .unwrap();
        git.cleanup_state().unwrap();
        let before = f.image();
        let result = super::resolve::resume(&f.service, &f.repo, id);
        if wrong {
            assert_ne!(result.code, ResultCode::Ok);
            assert_eq!(before, f.image());
        } else {
            assert_eq!(result.code, ResultCode::Ok, "{result:?}");
            assert_eq!(result.effects.checkpoint, CheckpointEffect::Unchanged);
            assert_eq!(result.effects.commit_oid, Some(oid.to_string()));
            assert_eq!(result.effects.integration, IntegrationEffect::Complete);
        }
    }
}

#[test]
fn local_resolution_retirement_child() {
    let Ok(root) = std::env::var("MANYHANDS_TASK6_ROOT") else {
        return;
    };
    let service = RepositoryService::open_at(
        PathBuf::from(std::env::var("MANYHANDS_TASK6_DATA").unwrap()).as_path(),
    )
    .unwrap();
    let repo = service
        .resolve_repository(PathBuf::from(root).as_path())
        .unwrap();
    let id = OperationId::parse(&std::env::var("MANYHANDS_TASK6_OPERATION").unwrap()).unwrap();
    let install = std::env::var("MANYHANDS_TASK6_POINT").is_ok_and(|p| p == "install");
    super::resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move |p| {
            if p == if install {
                super::resolve::Point::InstallReturned
            } else {
                super::resolve::Point::Retired(0)
            } {
                std::process::exit(76);
            }
            Ok(())
        }))
    });
    let result = super::resolve::resume(&service, &repo, id);
    panic!("did not exit at owned retirement: {result:?}");
}

#[test]
fn no_drop_conflict_backend_window_preserves_until_exact_operator_baseline() {
    let (f, _) = fixture(AuthoringKind::Ticket);
    let id = OperationId::new();
    let observation = f.observe();
    let failed = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::LocalSyncAfterIntent,
    )
    .unwrap();
    super::run::execute(
        &failed,
        &f.repo,
        f.kind,
        &f.item,
        id,
        &observation.observation,
    );
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "repository::local_sync::conflict_tests::local_resolution_retirement_child",
            "--nocapture",
        ])
        .env("MANYHANDS_TASK6_ROOT", f.root.path())
        .env("MANYHANDS_TASK6_DATA", f.data.path())
        .env("MANYHANDS_TASK6_OPERATION", id.to_string())
        .env("MANYHANDS_TASK6_POINT", "install")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(76),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let git = f.context();
    assert!(git.index().unwrap().has_conflicts());
    let before = f.image();
    let result = super::resolve::resume(&f.service, &f.repo, id);
    assert_eq!(result.reason, super::run::Reason::AmbiguousBackend);
    assert_eq!(result.effects.write, WriteEffect::Written);
    assert_eq!(result.outcome(), Outcome::Partial);
    assert_eq!(before, f.image());
    assert!(super::control::abandon(&f.service, &f.repo, id).is_err());
    // Operator preserves their edits and restores the recorded baseline; the
    // implementation itself never performs this reset or cleanup.
    git.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    git.cleanup_state().unwrap();
    let result = super::resolve::resume(&f.service, &f.repo, id);
    assert_eq!(
        result.state,
        Some(LocalSyncState::ConflictPending),
        "{result:?}"
    );
    let (observation, sources) = input(&f, id);
    assert_eq!(
        super::resolve::execute(
            &f.service,
            &f.repo,
            id,
            OperationId::new(),
            &observation,
            &sources
        )
        .code,
        ResultCode::Ok
    );
}

#[test]
fn no_drop_owned_metadata_retirement_finishes_automatically() {
    for foreign in [false, true] {
        let (f, _) = fixture(AuthoringKind::Document);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        super::resolve::HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(|p| {
                if p == super::resolve::Point::Ref {
                    Err(std::io::Error::other("fixture"))
                } else {
                    Ok(())
                }
            }))
        });
        super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
        super::resolve::HOOK.with(|h| h.borrow_mut().take());
        let git = f.context();
        let oid = git.head().unwrap().target().unwrap();
        let logs = git
            .reflog(git.head().unwrap().name().unwrap())
            .unwrap()
            .len();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "repository::local_sync::conflict_tests::local_resolution_retirement_child",
                "--nocapture",
            ])
            .env("MANYHANDS_TASK6_ROOT", f.root.path())
            .env("MANYHANDS_TASK6_DATA", f.data.path())
            .env("MANYHANDS_TASK6_OPERATION", id.to_string())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(76),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(!git.path().join("MERGE_HEAD").exists());
        assert!(git.path().join("MERGE_MSG").exists());
        if foreign {
            fs::write(git.path().join("MERGE_MSG"), "FOREIGN").unwrap();
        }
        let result = super::resolve::resume(&f.service, &f.repo, id);
        if foreign {
            assert_ne!(result.code, ResultCode::Ok);
            assert_eq!(fs::read(git.path().join("MERGE_MSG")).unwrap(), b"FOREIGN");
        } else {
            assert_eq!(result.code, ResultCode::Ok, "{result:?}");
            assert!(!git.path().join("MERGE_MSG").exists());
            assert!(!git.path().join("MERGE_MODE").exists());
        }
        assert_eq!(git.head().unwrap().target().unwrap(), oid);
        assert_eq!(
            git.reflog(git.head().unwrap().name().unwrap())
                .unwrap()
                .len(),
            logs
        );
    }
}

#[test]
fn permission_loss_after_one_resolution_path_preserves_recovery() {
    use super::resolve::{self, Point};
    let (f, _) = fixture(AuthoringKind::Ticket);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    resolve::WRITE_ATTEMPTS.with(|n| n.set(0));
    resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|p| {
            if p == Point::Path(1) {
                resolve::WRITE_FAULT.with(|f| f.set(Some(std::io::ErrorKind::PermissionDenied)));
            }
            Ok(())
        }))
    });
    let partial = resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    resolve::HOOK.with(|h| h.borrow_mut().take());
    assert_eq!(
        partial.code,
        ResultCode::RepositoryInaccessible,
        "{partial:?}"
    );
    assert_eq!(partial.outcome(), Outcome::Partial);
    assert_eq!(resolve::WRITE_ATTEMPTS.with(|n| n.get()), 2);
    assert_eq!(f.db().query_row("SELECT count(*) FROM local_context_sync_steps WHERE kind='path' AND phase='observed'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    let row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    let image = f.image();
    let resume = resolve::resume(&f.service, &f.repo, id);
    assert_eq!(resume.code, ResultCode::OriginalRequestRequired);
    assert_eq!(resume.required_request, Some(request));
    assert_eq!(image, f.image());
    let historical = super::conflict::read(&f.service, &f.repo, id, &sources[0].token).unwrap();
    assert!(historical.historical);
    assert_eq!(historical.local.unwrap(), sources[0].source);
    let mismatch = resolve::execute(
        &f.service,
        &f.repo,
        id,
        request,
        "changed observation",
        &sources,
    );
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.effects, partial.effects);
    assert_eq!(image, f.image());
    assert_eq!(
        f.service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap()
            .active_resolution,
        row.active_resolution
    );
    let result = resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    assert_eq!(result.code, ResultCode::Ok, "{result:?}");
    assert_eq!(
        resolve::execute(&f.service, &f.repo, id, request, &observation, &sources),
        result
    );
}

#[test]
fn resolution_candidate_ref_index_and_release_interruptions_finish_once() {
    use super::resolve::{self, Point};
    for point in [
        Point::Accepted,
        Point::Path(1),
        Point::Candidate,
        Point::Index,
        Point::Ref,
        Point::Retired(0),
    ] {
        let (f, _) = fixture(AuthoringKind::Document);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        let git = f.context();
        let branch = git.head().unwrap().name().unwrap().to_owned();
        let logs = git.reflog(&branch).unwrap().len();
        resolve::HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |p| {
                if p == point {
                    Err(std::io::Error::other("fixture"))
                } else {
                    Ok(())
                }
            }))
        });
        assert_ne!(
            resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
            ResultCode::Ok
        );
        resolve::HOOK.with(|h| h.borrow_mut().take());
        let result = if matches!(point, Point::Accepted | Point::Path(_)) {
            assert_eq!(
                resolve::resume(&f.service, &f.repo, id).code,
                ResultCode::OriginalRequestRequired
            );
            resolve::execute(&f.service, &f.repo, id, request, &observation, &sources)
        } else {
            resolve::resume(&f.service, &f.repo, id)
        };
        assert_eq!(result.code, ResultCode::Ok, "{point:?}: {result:?}");
        assert_eq!(git.reflog(&branch).unwrap().len(), logs + 1);
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        assert!(row.artifacts_released && row.index_complete);
        assert_eq!(
            resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).effects,
            result.effects
        );
    }
}

#[test]
fn candidate_intent_rebuilds_same_identity_and_foreign_release_precedes_no_write() {
    use super::resolve::{self, Point};
    let (f, _) = fixture(AuthoringKind::Ticket);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|p| {
            if p == Point::CandidateIntent {
                Err(std::io::Error::other("fixture"))
            } else {
                Ok(())
            }
        }))
    });
    let partial = resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    resolve::HOOK.with(|h| h.borrow_mut().take());
    assert_ne!(partial.code, ResultCode::Ok);
    let candidate = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap()
        .candidate
        .unwrap();
    assert!(f.context().find_commit(candidate).is_err());
    assert_eq!(
        resolve::resume(&f.service, &f.repo, id).code,
        ResultCode::OriginalRequestRequired
    );
    f.context()
        .config()
        .unwrap()
        .set_str("user.name", "Changed identity")
        .unwrap();
    assert_eq!(
        resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
        ResultCode::Ok
    );
    assert_eq!(f.context().head().unwrap().target(), Some(candidate));
    let (f, _) = fixture(AuthoringKind::Document);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|p| {
            if p == Point::Candidate {
                Err(std::io::Error::other("fixture"))
            } else {
                Ok(())
            }
        }))
    });
    resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    resolve::HOOK.with(|h| h.borrow_mut().take());
    fs::write(f.context().path().join("MERGE_MSG"), "FOREIGN").unwrap();
    let before = f.image();
    assert_ne!(
        resolve::resume(&f.service, &f.repo, id).code,
        ResultCode::Ok
    );
    assert_eq!(before, f.image());
}

#[test]
fn source_identity_and_historical_reads_do_not_trust_mutable_paths() {
    use super::resolve::{self, Point};
    let (f, paths) = fixture(AuthoringKind::Ticket);
    let id = start(&f);
    let (observation, mut sources) = input(&f, id);
    let request = OperationId::new();
    resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|p| {
            if p == Point::Path(1) {
                Err(std::io::Error::other("fixture"))
            } else {
                Ok(())
            }
        }))
    });
    resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    resolve::HOOK.with(|h| h.borrow_mut().take());
    let before = f.image();
    let original = sources[0].source.clone();
    sources[0]
        .source
        .push_str("private-resolution-source-sentinel-824718\n");
    let mismatch = resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(before, f.image());
    sources[0].source = original;
    for path in paths {
        fs::write(
            f.context().workdir().unwrap().join(path),
            "MUTABLE-SUBSTITUTE",
        )
        .unwrap();
    }
    for source in &sources {
        let sides = super::conflict::read(&f.service, &f.repo, id, &source.token).unwrap();
        assert!(sides.historical);
        assert_eq!(sides.local.unwrap(), source.source);
    }
    assert!(!format!("{mismatch:?}").contains("private-resolution-source"));
    let db = fs::read(f.data.path().join(REGISTRY_FILE)).unwrap();
    assert!(
        !db.windows(b"private-resolution-source".len())
            .any(|b| b == b"private-resolution-source")
    );
}

#[test]
fn closure_side_is_irreversible_and_accepted_source_stays_out_of_authority() {
    let (f, paths) = fixture(AuthoringKind::Ticket);
    let git = f.context();
    let ticket = git.workdir().unwrap().join(&paths[0]);
    let text = fs::read_to_string(&ticket).unwrap().replacen(
        "---\n",
        "---\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: Original\n",
        1,
    );
    fs::write(&ticket, text).unwrap();
    commit(&git);
    let id = start(&f);
    let (observation, mut sources) = input(&f, id);
    let ticket = sources
        .iter_mut()
        .find(|s| s.source.contains("closed_by:"))
        .unwrap();
    let valid = ticket.source.clone();
    ticket.source = ticket
        .source
        .replace("closed_at: 2026-10-25T12:00:00Z\n", "")
        .replace("closed_by: Original\n", "");
    let before = f.image();
    assert_eq!(
        super::resolve::execute(
            &f.service,
            &f.repo,
            id,
            OperationId::new(),
            &observation,
            &sources
        )
        .code,
        ResultCode::InvalidInput
    );
    assert_eq!(before, f.image());
    let ticket = sources
        .iter_mut()
        .find(|s| s.source.contains("manyhands_kind: ticket"))
        .unwrap();
    ticket.source = valid + "ACCEPTED-SOURCE-BODY-PRIVATE-837915\n";
    let result = super::resolve::execute(
        &f.service,
        &f.repo,
        id,
        OperationId::new(),
        &observation,
        &sources,
    );
    assert_eq!(result.code, ResultCode::Ok, "{result:?}");
    assert!(
        fs::read_to_string(git.workdir().unwrap().join(&paths[0]))
            .unwrap()
            .contains("ACCEPTED-SOURCE-BODY-PRIVATE-837915")
    );
    let db = fs::read(f.data.path().join(REGISTRY_FILE)).unwrap();
    assert!(
        !db.windows(b"ACCEPTED-SOURCE-BODY-PRIVATE-837915".len())
            .any(|b| b == b"ACCEPTED-SOURCE-BODY-PRIVATE-837915")
    );
    assert!(!format!("{result:?}").contains("ACCEPTED-SOURCE-BODY"));
}

#[test]
fn ancestor_only_closure_constrains_the_complete_local_resolution() {
    let (f, paths) = fixture(AuthoringKind::Ticket);
    let c = f.context();
    let p = git2::Repository::open(f.root.path()).unwrap();
    let local = c.head().unwrap().peel_to_commit().unwrap();
    let incoming = p.head().unwrap().peel_to_commit().unwrap();
    let base = c
        .find_commit(c.merge_base(local.id(), incoming.id()).unwrap())
        .unwrap();
    let mut index = git2::Index::new().unwrap();
    index.read_tree(&base.tree().unwrap()).unwrap();
    let entry = index.get_path(&paths[0], 0).unwrap();
    let source = String::from_utf8(c.find_blob(entry.id).unwrap().content().to_vec())
        .unwrap()
        .replacen(
            "---\n",
            "---\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: Ancestor\n",
            1,
        );
    let mut entry = entry;
    entry.id = c.blob(source.as_bytes()).unwrap();
    index.add(&entry).unwrap();
    let tree = c.find_tree(index.write_tree_to(&c).unwrap()).unwrap();
    let signature = c.signature().unwrap();
    let closed = c
        .commit(
            None,
            &signature,
            &signature,
            "closed ancestor",
            &tree,
            &[&base],
        )
        .unwrap();
    let closed = c.find_commit(closed).unwrap();
    for (git, old) in [(&c, &local), (&p, &incoming)] {
        let parent = git.find_commit(closed.id()).unwrap();
        let oid = git
            .commit(
                None,
                &signature,
                &signature,
                "removed closure on side",
                &old.tree().unwrap(),
                &[&parent],
            )
            .unwrap();
        assert_eq!(git.find_commit(oid).unwrap().parent_count(), 1);
        git.reference(git.head().unwrap().name().unwrap(), oid, true, "fixture")
            .unwrap();
    }
    let current = f.context();
    let ctip = current.head().unwrap().target().unwrap();
    let ptip = git2::Repository::open(f.root.path())
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap();
    assert_eq!(
        current.find_commit(ctip).unwrap().parent_id(0).unwrap(),
        closed.id()
    );
    assert_eq!(
        current.find_commit(ptip).unwrap().parent_id(0).unwrap(),
        closed.id()
    );
    assert_eq!(current.merge_base(ctip, ptip).unwrap(), closed.id());
    let id = start(&f);
    let (observation, mut sources) = input(&f, id);
    let before = f.image();
    assert_eq!(
        super::resolve::execute(
            &f.service,
            &f.repo,
            id,
            OperationId::new(),
            &observation,
            &sources
        )
        .code,
        ResultCode::InvalidInput
    );
    assert_eq!(before, f.image());
    let ticket = sources
        .iter_mut()
        .find(|s| s.source.contains("manyhands_kind: ticket"))
        .unwrap();
    ticket.source = ticket.source.replacen(
        "---\n",
        "---\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: Ancestor\n",
        1,
    );
    assert_eq!(
        super::resolve::execute(
            &f.service,
            &f.repo,
            id,
            OperationId::new(),
            &observation,
            &sources
        )
        .code,
        ResultCode::Ok
    );
}

#[test]
fn accepted_resolution_refuses_changed_mode_and_unrelated_untracked_before_more_effects() {
    for foreign in ["untracked", "metadata", "lock"] {
        let (f, _) = fixture(AuthoringKind::Document);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        super::resolve::HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(|p| {
                if p == super::resolve::Point::Candidate {
                    Err(std::io::Error::other("fixture"))
                } else {
                    Ok(())
                }
            }))
        });
        super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
        super::resolve::HOOK.with(|h| h.borrow_mut().take());
        let git = f.context();
        let path = match foreign {
            "untracked" => git.workdir().unwrap().join("foreign"),
            "metadata" => git.path().join("MERGE_MSG"),
            _ => git.path().join("index.lock"),
        };
        fs::write(&path, "FOREIGN").unwrap();
        let before = f.image();
        assert_ne!(
            super::resolve::resume(&f.service, &f.repo, id).code,
            ResultCode::Ok,
            "{foreign}"
        );
        assert_eq!(before, f.image(), "{foreign}");
        assert_eq!(fs::read(path).unwrap(), b"FOREIGN");
    }
}

#[cfg(unix)]
#[test]
fn partial_resolution_mode_change_is_refused_before_any_more_path_write() {
    use std::os::unix::fs::PermissionsExt;
    let (f, paths) = fixture(AuthoringKind::Document);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    super::resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|p| {
            if p == super::resolve::Point::Accepted {
                Err(std::io::Error::other("fixture"))
            } else {
                Ok(())
            }
        }))
    });
    super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    super::resolve::HOOK.with(|h| h.borrow_mut().take());
    let target = f.context().workdir().unwrap().join(&paths[1]);
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
    let before = paths
        .iter()
        .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
        .collect::<Vec<_>>();
    assert_ne!(
        super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
        ResultCode::Ok
    );
    assert_eq!(
        before,
        paths
            .iter()
            .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn opaque_tokens_refuse_wrong_operation_generation_mode_and_malformed_input() {
    let (f, _) = fixture(AuthoringKind::Document);
    let id = start(&f);
    let inspection = super::conflict::inspect(&f.service, &f.repo, id).unwrap();
    assert_eq!(
        super::conflict::read(&f.service, &f.repo, id, "docs/local.md")
            .err()
            .unwrap()
            .code,
        ResultCode::InvalidInput
    );
    let token = inspection.paths[0].token.clone();
    let (other, _) = fixture(AuthoringKind::Document);
    let other_id = start(&other);
    assert_eq!(
        super::conflict::read(&other.service, &other.repo, other_id, &token)
            .err()
            .unwrap()
            .code,
        ResultCode::ExternalChange
    );
    let mut row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    row.generation += 1;
    let (next, _) = super::conflict::enumerate(&f.repo, &f.context(), &row).unwrap();
    assert!(next.iter().all(|e| e.token != token));
    let before = f.image();
    assert_eq!(
        super::conflict::read(&f.service, &f.repo, id, &next[0].token)
            .err()
            .unwrap()
            .code,
        ResultCode::ExternalChange
    );
    assert_eq!(before, f.image());
}

#[test]
fn conflict_preparation_keeps_frozen_attribute_source_guards() {
    use std::io::Write;
    let (f, _) = fixture(AuthoringKind::Document);
    let git = f.context();
    std::fs::OpenOptions::new()
        .append(true)
        .open(git.commondir().join("info/exclude"))
        .unwrap()
        .write_all(b"\n.gitattributes\n")
        .unwrap();
    fs::write(
        git.workdir().unwrap().join(".gitattributes"),
        "*.md text eol=crlf\n",
    )
    .unwrap();
    let observation = f.observe();
    let before = f.image();
    let id = OperationId::new();
    let result = super::run::execute(
        &f.service,
        &f.repo,
        f.kind,
        &f.item,
        id,
        &observation.observation,
    );
    assert_eq!(result.state, None, "{result:?}");
    assert_eq!(before, f.image());
}

#[test]
fn resolution_index_transaction_failure_preserves_integration_and_retries_only_discovery() {
    let (f, _) = fixture(AuthoringKind::Document);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    let failed = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeIndexTransactionCommit,
    )
    .unwrap();
    let result = super::resolve::execute(&failed, &f.repo, id, request, &observation, &sources);
    assert_eq!(result.code, ResultCode::DiscoveryPending, "{result:?}");
    assert_eq!(result.effects.integration, IntegrationEffect::Complete);
    assert_eq!(result.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(result.state, Some(LocalSyncState::IndexPending));
    let git = f.context();
    let before = f.image();
    let logs = git
        .reflog(git.head().unwrap().name().unwrap())
        .unwrap()
        .len();
    let result = super::resolve::resume(&f.service, &f.repo, id);
    assert_eq!(result.code, ResultCode::Ok, "{result:?}");
    assert_eq!(before, f.image());
    assert_eq!(
        git.reflog(git.head().unwrap().name().unwrap())
            .unwrap()
            .len(),
        logs
    );
}

#[test]
fn conflict_read_rechecks_resolution_counters_in_a_fresh_session() {
    let (f, _) = fixture(AuthoringKind::Document);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    super::resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|p| {
            if p == super::resolve::Point::Accepted {
                Err(std::io::Error::other("fixture"))
            } else {
                Ok(())
            }
        }))
    });
    super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    super::resolve::HOOK.with(|h| h.borrow_mut().take());
    let data = f.data.path().to_owned();
    let root = f.root.path().to_owned();
    super::conflict::INSPECT_HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move || {
            let service = RepositoryService::open_at(&data).unwrap();
            let repo = service.resolve_repository(&root).unwrap();
            super::control::issue(&service, &repo, id, Some(request)).unwrap();
        }))
    });
    let result = super::conflict::inspect(&f.service, &f.repo, id);
    super::conflict::INSPECT_HOOK.with(|h| h.borrow_mut().take());
    assert_eq!(result.err().unwrap().code, ResultCode::ExternalChange);
    assert!(
        super::conflict::read(&f.service, &f.repo, id, &sources[0].token)
            .unwrap()
            .historical
    );
}

#[cfg(unix)]
#[test]
fn unix_directory_permission_loss_keeps_partial_resolution_resendable() {
    use std::os::unix::fs::PermissionsExt;
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let (f, _) = fixture(AuthoringKind::Ticket);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    let directory = f.context().workdir().unwrap().join("docs");
    let denied = directory.clone();
    super::resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move |point| {
            if point == super::resolve::Point::Path(1) {
                fs::set_permissions(&denied, fs::Permissions::from_mode(0o500)).unwrap();
            }
            Ok(())
        }))
    });
    let result = super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    super::resolve::HOOK.with(|h| h.borrow_mut().take());
    fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        result.code,
        ResultCode::RepositoryInaccessible,
        "{result:?}"
    );
    assert_eq!(result.outcome(), Outcome::Partial);
    assert_eq!(
        super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
        ResultCode::Ok
    );
}

#[test]
fn resolution_journal_access_loss_retains_last_proven_effects() {
    for point in [
        super::resolve::Point::Path(1),
        super::resolve::Point::Candidate,
    ] {
        let (f, _) = fixture(AuthoringKind::Ticket);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        super::resolve::HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |p| {
                if p == point {
                    super::record::ACCESS_FAULT
                        .with(|f| f.set(Some(std::io::ErrorKind::PermissionDenied)));
                }
                Ok(())
            }))
        });
        let result =
            super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
        super::resolve::HOOK.with(|h| h.borrow_mut().take());
        super::record::ACCESS_FAULT.with(|f| f.set(None));
        assert_eq!(result.code, ResultCode::IndexUnavailable, "{result:?}");
        assert_eq!(result.effects.write, WriteEffect::Written, "{result:?}");
        assert_eq!(result.outcome(), Outcome::Partial);
        if point == super::resolve::Point::Candidate {
            assert_eq!(result.effects.checkpoint, CheckpointEffect::Committed);
        }
        assert_eq!(
            super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
            ResultCode::Ok
        );
    }
}

pub(super) fn fixture(kind: AuthoringKind) -> (Fixture, Vec<PathBuf>) {
    fixture_with(kind, |_, _| {})
}
fn fixture_with(
    kind: AuthoringKind,
    setup: impl FnOnce(&Fixture, &git2::Repository),
) -> (Fixture, Vec<PathBuf>) {
    let f = Fixture::new(kind);
    let c = f.context();
    let p = git2::Repository::open(f.root.path()).unwrap();
    let path = match kind {
        AuthoringKind::Document => PathBuf::from("docs/local.md"),
        AuthoringKind::Ticket => PathBuf::from(format!(".manyhands/tickets/{}/ticket.md", f.item)),
    };
    let other = PathBuf::from("docs/other.md");
    fs::create_dir_all(c.workdir().unwrap().join("docs")).unwrap();
    fs::write(c.workdir().unwrap().join(&other), format!("---\ntitle: Other\nmanyhands_managed: true\nmanyhands_kind: document\nid: {}\n---\nbase\n", crate::canonical::ItemId::generate())).unwrap();
    setup(&f, &c);
    let base = commit(&c);
    p.reference("refs/heads/main", base, true, "fixture")
        .unwrap();
    p.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    for (git, side) in [(&c, "local"), (&p, "incoming")] {
        for path in [&path, &other] {
            let file = git.workdir().unwrap().join(path);
            let text = fs::read_to_string(&file).unwrap();
            fs::write(file, format!("{text}{side}\n")).unwrap();
        }
        commit(git);
    }
    (f, vec![path, other])
}

#[test]
fn prospective_context_checks_comment_dependencies_before_accepting_sources() {
    let parent = crate::canonical::ItemId::generate();
    let comment = crate::canonical::ItemId::generate();
    let (f, _) = fixture_with(AuthoringKind::Ticket, |f, git| {
        let target = &f.item;
        let directory = git
            .workdir()
            .unwrap()
            .join(format!(".manyhands/comments/{target}"));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(format!("{parent}.md")),format!("---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {parent}\nitem_id: {target}\ncreated_at: 2026-10-25T12:00:00Z\n---\nparent\n")).unwrap();
        fs::write(directory.join(format!("{comment}.md")),format!("---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {comment}\nitem_id: {target}\nparent_id: {parent}\ncreated_at: 2026-10-25T12:00:00Z\n---\ncomment\n")).unwrap();
    });
    let primary = git2::Repository::open(f.root.path()).unwrap();
    fs::remove_file(
        f.root
            .path()
            .join(format!(".manyhands/comments/{}/{parent}.md", f.item)),
    )
    .unwrap();
    commit(&primary);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let before = f.image();
    let result = super::resolve::execute(
        &f.service,
        &f.repo,
        id,
        OperationId::new(),
        &observation,
        &sources,
    );
    assert_eq!(result.code, ResultCode::InvalidInput, "{result:?}");
    assert_eq!(before, f.image());
    assert!(
        f.service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap()
            .active_resolution
            .is_none()
    );
}

#[test]
fn rebuild_and_lost_registry_preserve_real_conflict_and_partial_resolution_evidence() {
    for partial in [false, true] {
        let (f, _) = fixture(AuthoringKind::Document);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        if partial {
            super::resolve::HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(|p| {
                    if p == super::resolve::Point::Path(1) {
                        Err(std::io::Error::other("fixture"))
                    } else {
                        Ok(())
                    }
                }))
            });
            super::resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
            super::resolve::HOOK.with(|h| h.borrow_mut().take());
        }
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        let before = f.image();
        assert!(
            f.service
                .rebuild_repository(RebuildRepositoryRequest {
                    root: f.root.path().into(),
                    operation_id: OperationId::new()
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
        // No database handles survive this replacement; Git artifacts stay put.
        fs::remove_file(f.data.path().join(REGISTRY_FILE)).unwrap();
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert_ne!(
            super::resolve::resume(&reopened, &f.repo, id).code,
            ResultCode::Ok
        );
        assert!(super::conflict::read(&reopened, &f.repo, id, &sources[0].token).is_err());
        assert_eq!(before, f.image());
        assert!(f.context().path().join("MERGE_HEAD").exists());
    }
}

#[test]
fn ignored_resolution_takeover_does_not_authorize_a_path_write() {
    let (f, paths) = fixture(AuthoringKind::Document);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    f.db().execute_batch("CREATE TRIGGER refuse_resolution_epoch BEFORE UPDATE OF owner_epoch ON local_context_sync_records WHEN NEW.active_resolution IS NOT NULL BEGIN SELECT RAISE(IGNORE); END").unwrap();
    let before = paths
        .iter()
        .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
        .collect::<Vec<_>>();
    let result = super::resolve::execute(
        &f.service,
        &f.repo,
        id,
        OperationId::new(),
        &observation,
        &sources,
    );
    assert_ne!(result.code, ResultCode::Ok);
    assert_eq!(
        before,
        paths
            .iter()
            .map(|p| fs::read(f.context().workdir().unwrap().join(p)).unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn conflict_installation_is_owned_partial_for_both_context_kinds() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let (f, _) = fixture(kind);
        let observation = f.observe();
        let id = OperationId::new();
        let result = run::execute(
            &f.service,
            &f.repo,
            kind,
            &f.item,
            id,
            &observation.observation,
        );
        assert_eq!(
            result.state,
            Some(LocalSyncState::ConflictPending),
            "{result:?}"
        );
        assert_eq!(result.effects.write, WriteEffect::Written);
        assert_eq!(result.effects.checkpoint, CheckpointEffect::Pending);
        assert_eq!(result.effects.integration, IntegrationEffect::Pending);
        assert_eq!(result.outcome(), Outcome::Partial);
        assert_eq!(f.context().index().unwrap().conflicts().unwrap().count(), 2);
        let image = f.image();
        assert_eq!(
            run::execute(
                &f.service,
                &f.repo,
                kind,
                &f.item,
                id,
                &observation.observation
            ),
            result
        );
        assert_eq!(image, f.image());
    }
}
