//! Task 6 SPEC compositions, from review ses_e864b2dfbffemfrKBcgnvISO55.
use super::{
    conflict_tests::{fixture, fixture_with, input, start},
    record_tests::{Fixture, commit},
    resolve,
};
use crate::{repository::*, results::*};
use std::{collections::BTreeSet, fs, path::PathBuf};

#[derive(PartialEq, Eq)]
struct GitImage {
    head: git2::Oid,
    index: Vec<u8>,
    files: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    objects: BTreeSet<git2::Oid>,
    metadata: Vec<Option<Vec<u8>>>,
    logs: Vec<Vec<u8>>,
}
impl std::fmt::Debug for GitImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitImage")
            .field("head", &self.head)
            .field("index_digest", &blake3::hash(&self.index))
            .field("file_count", &self.files.len())
            .field("object_count", &self.objects.len())
            .finish_non_exhaustive()
    }
}
fn image(f: &Fixture) -> GitImage {
    let git = f.context();
    let branch = git.head().unwrap().name().unwrap().to_owned();
    let paths = git
        .index()
        .unwrap()
        .iter()
        .map(|e| e.path)
        .collect::<BTreeSet<_>>();
    let mut objects = BTreeSet::new();
    git.odb()
        .unwrap()
        .foreach(|oid| {
            objects.insert(*oid);
            true
        })
        .unwrap();
    GitImage {
        head: git.head().unwrap().target().unwrap(),
        index: fs::read(git.path().join("index")).unwrap(),
        files: paths
            .into_iter()
            .map(|p| {
                let bytes = fs::read(
                    git.workdir()
                        .unwrap()
                        .join(std::str::from_utf8(&p).unwrap()),
                )
                .ok();
                (p, bytes)
            })
            .collect(),
        objects,
        metadata: [
            "MERGE_HEAD",
            "MERGE_MSG",
            "MERGE_MODE",
            "index.lock",
            "HEAD.lock",
        ]
        .iter()
        .map(|p| fs::read(git.path().join(p)).ok())
        .collect(),
        logs: vec![
            fs::read(git.commondir().join("logs").join(branch)).unwrap(),
            fs::read(git.path().join("logs/HEAD")).unwrap(),
        ],
    }
}

#[test]
fn spec_whole_result_immutable_preflight_precedes_attempt_and_path_writes() {
    let ticket = crate::canonical::ItemId::generate();
    let path = PathBuf::from(format!(".manyhands/tickets/{ticket}/ticket.md"));
    let (f, _) = fixture_with(AuthoringKind::Document, |_, git| {
        let file = git.workdir().unwrap().join(&path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file,format!("---\nid: {ticket}\ntitle: Closed\ntype: task\nstatus: closed\nmanyhands_managed: true\nmanyhands_kind: ticket\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: Original\n---\nclosed body\n")).unwrap();
    });
    let primary = git2::Repository::open(f.root.path()).unwrap();
    let file = f.root.path().join(&path);
    let text = fs::read_to_string(&file).unwrap();
    fs::write(
        file,
        text.replace("closed_by: Original", "closed_by: Changed"),
    )
    .unwrap();
    commit(&primary);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let inspection = super::conflict::inspect(&f.service, &f.repo, id).unwrap();
    assert!(inspection.can_start_resolution);
    assert_eq!(
        inspection.eligibility,
        integration::validate::ConflictEligibility::EligibleCanonical
    );
    assert!(f.context().index().unwrap().get_path(&path, 0).is_some());
    let before = image(&f);
    let row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    resolve::WRITE_ATTEMPTS.with(|n| n.set(0));
    let result = resolve::execute(
        &f.service,
        &f.repo,
        id,
        OperationId::new(),
        &observation,
        &sources,
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM local_context_sync_resolutions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(resolve::WRITE_ATTEMPTS.with(|n| n.get()), 0);
    assert_eq!(result.code, ResultCode::InvalidInput);
    assert_eq!(image(&f), before);
    assert_eq!(
        f.service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap(),
        row
    );
}

fn foreign_path(git: &git2::Repository, kind: &str) -> PathBuf {
    let branch = git.head().unwrap().name().unwrap().to_owned();
    match kind {
        "index_lock" => git.path().join("index.lock"),
        "head_lock" => git.path().join("HEAD.lock"),
        "branch_lock" => git.commondir().join(format!("{branch}.lock")),
        "branch_log_lock" => git.commondir().join(format!("logs/{branch}.lock")),
        "packed_lock" => git.commondir().join("packed-refs.lock"),
        "branch_log" => git.commondir().join("logs").join(branch),
        "head_log" => git.path().join("logs/HEAD"),
        _ => unreachable!(),
    }
}
fn introduce_foreign(git: &git2::Repository, kind: &str) {
    if kind.ends_with("_log") {
        let name = if kind == "head_log" {
            "HEAD".into()
        } else {
            git.head().unwrap().name().unwrap().to_owned()
        };
        let mut log = git.reflog(&name).unwrap();
        log.append(
            git.head().unwrap().target().unwrap(),
            &git.signature().unwrap(),
            Some("foreign same-oid entry"),
        )
        .unwrap();
        log.write().unwrap();
    } else {
        fs::write(foreign_path(git, kind), b"FOREIGN-LOCK").unwrap();
    }
}

#[test]
fn spec_foreign_backend_evidence_precedes_first_and_partial_resolution_effects() {
    for partial in [false, true] {
        for kind in [
            "index_lock",
            "head_lock",
            "branch_lock",
            "branch_log_lock",
            "packed_lock",
            "branch_log",
            "head_log",
        ] {
            let (f, _) = fixture(AuthoringKind::Document);
            let id = start(&f);
            let (observation, sources) = input(&f, id);
            let request = OperationId::new();
            if partial {
                resolve::HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(|p| {
                        if p == resolve::Point::Path(1) {
                            Err(std::io::Error::other("fixture"))
                        } else {
                            Ok(())
                        }
                    }))
                });
                resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
                resolve::HOOK.with(|h| h.borrow_mut().take());
                assert_eq!(f.db().query_row("SELECT count(*) FROM local_context_sync_steps WHERE kind='path' AND phase='observed'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
            }
            let git = f.context();
            let foreign = foreign_path(&git, kind);
            let old = fs::read(&foreign).ok();
            introduce_foreign(&git, kind);
            let before = image(&f);
            let foreign_bytes = fs::read(&foreign).unwrap();
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            let result = resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
            assert_ne!(result.code, ResultCode::Ok, "{partial} {kind}");
            assert_eq!(image(&f), before, "{partial} {kind}");
            assert_eq!(fs::read(&foreign).unwrap(), foreign_bytes);
            assert_eq!(
                f.service
                    .read_local_context_sync_record(&f.repo, id)
                    .unwrap()
                    .active_resolution,
                row.active_resolution
            );
            if !partial {
                assert_eq!(
                    f.db()
                        .query_row(
                            "SELECT count(*) FROM local_context_sync_resolutions",
                            [],
                            |r| r.get::<_, i64>(0)
                        )
                        .unwrap(),
                    0
                );
            }
            if let Some(bytes) = old {
                fs::write(foreign, bytes).unwrap();
            } else {
                fs::remove_file(foreign).unwrap();
            }
            assert_eq!(
                resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
                ResultCode::Ok
            );
        }
    }
}

#[test]
fn spec_foreign_evidence_at_path_import_and_index_boundaries_has_no_next_effect() {
    for point in [
        resolve::Point::Path(0),
        resolve::Point::CandidateIntent,
        resolve::Point::Candidate,
    ] {
        for kind in ["index_lock", "branch_log", "head_log"] {
            let (f, _) = fixture(AuthoringKind::Ticket);
            let id = start(&f);
            let (observation, sources) = input(&f, id);
            let context = f.context().workdir().unwrap().to_owned();
            let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
            let hook_capture = captured.clone();
            resolve::HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(move |p| {
                    if p == point {
                        let git = git2::Repository::open(&context).unwrap();
                        introduce_foreign(&git, kind);
                        let mut objects = BTreeSet::new();
                        git.odb()
                            .unwrap()
                            .foreach(|oid| {
                                objects.insert(*oid);
                                true
                            })
                            .unwrap();
                        let files = git
                            .index()
                            .unwrap()
                            .iter()
                            .map(|e| e.path)
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .map(|p| {
                                let bytes =
                                    fs::read(context.join(std::str::from_utf8(&p).unwrap())).ok();
                                (p, bytes)
                            })
                            .collect::<Vec<_>>();
                        *hook_capture.borrow_mut() =
                            Some((objects, fs::read(git.path().join("index")).unwrap(), files));
                    }
                    Ok(())
                }))
            });
            let result = resolve::execute(
                &f.service,
                &f.repo,
                id,
                OperationId::new(),
                &observation,
                &sources,
            );
            resolve::HOOK.with(|h| h.borrow_mut().take());
            assert_ne!(result.code, ResultCode::Ok, "{point:?} {kind}");
            let actual = image(&f);
            let expected = captured.borrow();
            let (objects, index, files) = expected.as_ref().unwrap();
            assert_eq!(&actual.objects, objects, "{point:?} {kind}");
            assert_eq!(&actual.index, index, "{point:?} {kind}");
            assert_eq!(&actual.files, files, "{point:?} {kind}");
        }
    }
}

#[test]
fn spec_import_observation_child() {
    let Ok(root) = std::env::var("MANYHANDS_TASK6_SPEC_ROOT") else {
        return;
    };
    let service = RepositoryService::open_at(&PathBuf::from(
        std::env::var("MANYHANDS_TASK6_SPEC_DATA").unwrap(),
    ))
    .unwrap();
    let repo = service.resolve_repository(&PathBuf::from(root)).unwrap();
    let id = OperationId::parse(&std::env::var("MANYHANDS_TASK6_SPEC_OPERATION").unwrap()).unwrap();
    let request =
        OperationId::parse(&std::env::var("MANYHANDS_TASK6_SPEC_REQUEST").unwrap()).unwrap();
    let observation = std::env::var("MANYHANDS_TASK6_SPEC_OBSERVATION").unwrap();
    // Fixture caller explicitly resends its original choice of the local sides.
    // No production source-free resume reconstructs submitted source this way.
    let inspection = super::conflict::inspect(&service, &repo, id).unwrap();
    let sources = inspection
        .paths
        .iter()
        .map(|entry| resolve::Source {
            token: entry.token.clone(),
            source: super::conflict::read(&service, &repo, id, &entry.token)
                .unwrap()
                .local
                .unwrap(),
        })
        .collect::<Vec<_>>();
    resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(|point| {
            if point == resolve::Point::CandidateImported {
                std::process::exit(79);
            }
            Ok(())
        }))
    });
    let result = resolve::execute(&service, &repo, id, request, &observation, &sources);
    panic!("child did not reach post-import/pre-observation boundary: {result:?}");
}

#[test]
fn spec_no_drop_import_before_observation_reconciles_owned_checkpoint_on_resume_and_replay() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for source_free in [false, true] {
            let (f, _) = fixture(kind);
            let id = start(&f);
            let (observation, sources) = input(&f, id);
            let request = OperationId::new();
            resolve::HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(|p| {
                    if p == resolve::Point::CandidateIntent {
                        Err(std::io::Error::other("fixture"))
                    } else {
                        Ok(())
                    }
                }))
            });
            resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
            resolve::HOOK.with(|h| h.borrow_mut().take());
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            let candidate = row.candidate.unwrap();
            let git = f.context();
            assert!(git.find_commit(candidate).is_err());
            let branch = git.head().unwrap().name().unwrap().to_owned();
            let logs = git.reflog(&branch).unwrap().len();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "repository::local_sync::spec_tests::spec_import_observation_child",
                    "--nocapture",
                ])
                .env("MANYHANDS_TASK6_SPEC_ROOT", f.root.path())
                .env("MANYHANDS_TASK6_SPEC_DATA", f.data.path())
                .env("MANYHANDS_TASK6_SPEC_OPERATION", id.to_string())
                .env("MANYHANDS_TASK6_SPEC_REQUEST", request.to_string())
                .env("MANYHANDS_TASK6_SPEC_OBSERVATION", &observation)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(79),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            let commit = git.find_commit(candidate).unwrap();
            assert_eq!(
                commit.parent_ids().collect::<Vec<_>>(),
                [row.context_parent, row.primary_parent]
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT phase FROM local_context_sync_steps WHERE ordinal=0",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                "prepared"
            );
            assert_eq!(git.head().unwrap().target(), Some(row.context_parent));
            let objects = image(&f).objects;
            let result = if source_free {
                resolve::resume(&f.service, &f.repo, id)
            } else {
                resolve::execute(&f.service, &f.repo, id, request, &observation, &sources)
            };
            assert_eq!(result.code, ResultCode::Ok, "{result:?}");
            assert_eq!(
                result.effects.checkpoint,
                CheckpointEffect::Committed,
                "{result:?}"
            );
            assert_eq!(result.effects.commit_oid, Some(candidate.to_string()));
            assert_eq!(git.reflog(&branch).unwrap().len(), logs + 1);
            assert_eq!(image(&f).objects, objects);
            let before = image(&f);
            assert_eq!(
                resolve::execute(&f.service, &f.repo, id, request, &observation, &sources)
                    .effects
                    .checkpoint,
                CheckpointEffect::Committed
            );
            assert_eq!(
                super::run::execute(&f.service, &f.repo, kind, &f.item, id, &row.observation)
                    .effects
                    .checkpoint,
                CheckpointEffect::Committed
            );
            assert_eq!(image(&f), before);
        }
    }
}

#[test]
fn spec_existing_external_object_cannot_be_observed_as_the_owned_candidate() {
    for broken in ["signature", "attempt"] {
        let (f, _) = fixture(AuthoringKind::Ticket);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        resolve::HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(|p| {
                if p == resolve::Point::CandidateImported {
                    Err(std::io::Error::other("fixture"))
                } else {
                    Ok(())
                }
            }))
        });
        resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
        resolve::HOOK.with(|h| h.borrow_mut().take());
        let git = f.context();
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        if broken == "signature" {
            let sig = git2::Signature::now("Foreign", "foreign@example.invalid").unwrap();
            let commit = git.find_commit(row.candidate.unwrap()).unwrap();
            let tree = commit.tree().unwrap();
            let oid = git
                .commit(
                    None,
                    &sig,
                    &sig,
                    "external repair",
                    &tree,
                    &[
                        &git.find_commit(row.context_parent).unwrap(),
                        &git.find_commit(row.primary_parent).unwrap(),
                    ],
                )
                .unwrap();
            let db = f.db();
            db.execute(
                "UPDATE local_context_sync_records SET candidate_oid=?1",
                [oid.to_string()],
            )
            .unwrap();
            db.execute(
                "UPDATE local_context_sync_resolutions SET candidate_oid=?1",
                [oid.to_string()],
            )
            .unwrap();
            db.execute(
                "UPDATE local_context_sync_steps SET expected_new_oid=?1 WHERE ordinal=0",
                [oid.to_string()],
            )
            .unwrap();
        } else {
            f.db()
                .execute(
                    "UPDATE local_context_sync_resolutions SET candidate_oid=NULL",
                    [],
                )
                .unwrap();
        }
        let before = image(&f);
        assert_ne!(
            resolve::resume(&f.service, &f.repo, id).code,
            ResultCode::Ok
        );
        assert_eq!(image(&f), before);
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT phase FROM local_context_sync_steps WHERE ordinal=0",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "prepared"
        );
    }
}

fn index_pending(kind: AuthoringKind, shape: &str) -> (Fixture, OperationId) {
    if shape == "external" {
        let (f, paths) = fixture(kind);
        let id = start(&f);
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        let git = f.context();
        let local = git.find_commit(row.context_parent).unwrap();
        let incoming = git.find_commit(row.primary_parent).unwrap();
        let tree = local.tree().unwrap();
        for path in paths {
            fs::write(
                git.workdir().unwrap().join(&path),
                git.find_blob(tree.get_path(&path).unwrap().id())
                    .unwrap()
                    .content(),
            )
            .unwrap();
        }
        let mut index = git.index().unwrap();
        index.read_tree(&tree).unwrap();
        index.write().unwrap();
        let sig = git.signature().unwrap();
        let oid = git
            .commit(
                None,
                &sig,
                &sig,
                "external repair",
                &tree,
                &[&local, &incoming],
            )
            .unwrap();
        git.reference(&row.context_ref, oid, true, "external repair")
            .unwrap();
        git.cleanup_state().unwrap();
        let failed = RepositoryService::open_at_with_failure_point_for_testing(
            f.data.path(),
            FailurePoint::LocalSyncBeforeIndex,
        )
        .unwrap();
        let result = resolve::resume(&failed, &f.repo, id);
        assert_eq!(result.code, ResultCode::DiscoveryPending, "{result:?}");
        assert_eq!(result.effects.checkpoint, CheckpointEffect::Unchanged);
        return (f, id);
    }
    let f = Fixture::new(kind);
    let primary = git2::Repository::open(f.root.path()).unwrap();
    if matches!(shape, "equal" | "fast_forward") {
        primary
            .reference(
                "refs/heads/main",
                f.context().head().unwrap().target().unwrap(),
                true,
                "fixture",
            )
            .unwrap();
        primary
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
    }
    if matches!(shape, "fast_forward" | "merge") {
        fs::write(f.root.path().join("incoming"), "incoming\n").unwrap();
        commit(&primary);
    }
    let id = OperationId::new();
    let observation = f.observe();
    let failed = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::LocalSyncBeforeIndex,
    )
    .unwrap();
    let result = super::run::execute(
        &failed,
        &f.repo,
        kind,
        &f.item,
        id,
        &observation.observation,
    );
    assert_eq!(
        result.code,
        ResultCode::DiscoveryPending,
        "{shape}: {result:?}"
    );
    (f, id)
}

#[test]
fn spec_artifact_free_abandon_uses_each_integration_disposition_and_provenance() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for shape in ["equal", "contained", "fast_forward", "merge", "external"] {
            let (f, id) = index_pending(kind, shape);
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            assert_eq!(row.state, super::record::LocalSyncState::IndexPending);
            assert!(row.integration_proven && row.artifacts_released);
            let before = image(&f);
            let result = super::control::abandon(&f.service, &f.repo, id);
            assert!(result.is_ok(), "{kind:?} {shape}: {result:?}");
            assert_eq!(image(&f), before);
            let after = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            assert_eq!(after.state, super::record::LocalSyncState::Abandoned);
            assert!(after.owner_epoch > row.owner_epoch);
            assert_eq!(after.result, row.result);
            assert_eq!(
                resolve::resume(&f.service, &f.repo, id).code,
                ResultCode::Cancelled
            );
            assert_eq!(image(&f), before);
        }
    }
}

#[test]
fn spec_disposition_abandon_preserves_foreign_files_and_backend_artifacts() {
    for shape in ["equal", "contained", "fast_forward", "merge", "external"] {
        for foreign in ["file", "lock"] {
            let (f, id) = index_pending(AuthoringKind::Document, shape);
            let git = f.context();
            let path = if foreign == "file" {
                git.workdir().unwrap().join("ordinary")
            } else {
                git.path().join("index.lock")
            };
            let original = fs::read(&path).ok();
            fs::write(&path, "FOREIGN").unwrap();
            let row = f
                .service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap();
            let before = image(&f);
            assert!(
                super::control::abandon(&f.service, &f.repo, id).is_err(),
                "{shape} {foreign}"
            );
            assert_eq!(
                f.service
                    .read_local_context_sync_record(&f.repo, id)
                    .unwrap(),
                row
            );
            assert_eq!(image(&f), before);
            if let Some(bytes) = original {
                fs::write(path, bytes).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
            assert!(
                super::control::abandon(&f.service, &f.repo, id).is_ok(),
                "{shape} {foreign} restored"
            );
        }
    }
}
