//! QUALITY ses_e8613a466ffe3Dlw50236N05X9: effects at continuation boundaries.
use super::super::{record::LocalSyncState, run};
use super::*;
use std::{cell::RefCell, rc::Rc};

struct Repair {
    head: git2::Oid,
    branch: String,
    logs: [Vec<u8>; 2],
    leaf: Option<(PathBuf, Option<Vec<u8>>)>,
}
fn change(git: &git2::Repository, what: &str, parent: git2::Oid) -> Repair {
    let branch = git.head().unwrap().name().unwrap().to_owned();
    let head = git.head().unwrap().target().unwrap();
    let mut repair = Repair {
        head,
        branch: branch.clone(),
        logs: [
            fs::read(git.commondir().join("logs").join(&branch)).unwrap(),
            fs::read(git.path().join("logs/HEAD")).unwrap(),
        ],
        leaf: None,
    };
    match what {
        "ref" => {
            let foreign = if parent != head {
                parent
            } else {
                let old = git.find_commit(head).unwrap();
                let sig = git.signature().unwrap();
                git.commit(
                    None,
                    &sig,
                    &sig,
                    "foreign empty checkpoint",
                    &old.tree().unwrap(),
                    &[&old],
                )
                .unwrap()
            };
            git.reference(&branch, foreign, true, "foreign context change")
                .unwrap();
        }
        "branch_log" | "head_log" => introduce_foreign(git, what),
        "lock" | "MERGE_MSG" | "MERGE_MODE" => {
            let path = git
                .path()
                .join(if what == "lock" { "index.lock" } else { what });
            repair.leaf = Some((path.clone(), fs::read(&path).ok()));
            fs::write(path, b"FOREIGN").unwrap();
        }
        _ => unreachable!(),
    }
    repair
}
impl Repair {
    fn restore(self, git: &git2::Repository) {
        if git.refname_to_id(&self.branch).unwrap() != self.head {
            git.reference(&self.branch, self.head, true, "restore exact proof")
                .unwrap();
        }
        fs::write(
            git.commondir().join("logs").join(&self.branch),
            &self.logs[0],
        )
        .unwrap();
        fs::write(git.path().join("logs/HEAD"), &self.logs[1]).unwrap();
        if let Some((path, bytes)) = self.leaf {
            if let Some(bytes) = bytes {
                fs::write(path, bytes).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }
    }
}
fn pending(shape: &str) -> (Fixture, OperationId) {
    if shape != "resolution" {
        return index_pending(AuthoringKind::Document, shape);
    }
    let (f, _) = fixture(AuthoringKind::Ticket);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let failed = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::LocalSyncBeforeIndex,
    )
    .unwrap();
    let result = resolve::execute(
        &failed,
        &f.repo,
        id,
        OperationId::new(),
        &observation,
        &sources,
    );
    assert_eq!(result.code, ResultCode::DiscoveryPending);
    (f, id)
}
fn retry(f: &Fixture, id: OperationId, shape: &str) -> run::Report {
    if matches!(shape, "resolution" | "external") {
        resolve::resume(&f.service, &f.repo, id)
    } else {
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        run::execute(&f.service, &f.repo, f.kind, &f.item, id, &row.observation)
    }
}

#[test]
fn quality_completion_revalidates_all_dispositions_after_index_boundaries() {
    for point in [
        FailurePoint::LocalSyncBeforeIndex,
        FailurePoint::BeforeIndexTransactionCommit,
    ] {
        for shape in [
            "resolution",
            "external",
            "equal",
            "contained",
            "fast_forward",
            "merge",
        ] {
            for foreign in ["ref", "branch_log", "head_log", "lock"] {
                let (f, id) = pending(shape);
                let f = Rc::new(f);
                let row = f
                    .service
                    .read_local_context_sync_record(&f.repo, id)
                    .unwrap();
                let captured = Rc::new(RefCell::new(None));
                let repair = Rc::new(RefCell::new(None));
                let hf = f.clone();
                let hc = captured.clone();
                let hr = repair.clone();
                let parent = row.context_parent;
                run::BOUNDARY_HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(move |p| {
                        if p == point {
                            *hr.borrow_mut() = Some(change(&hf.context(), foreign, parent));
                            *hc.borrow_mut() = Some(image(&hf));
                        }
                    }))
                });
                let result = retry(&f, id, shape);
                run::BOUNDARY_HOOK.with(|h| h.borrow_mut().take());
                assert!(
                    !matches!(result.code, ResultCode::Ok | ResultCode::AlreadyApplied),
                    "{point:?} {shape} {foreign}: {result:?}"
                );
                let after = f
                    .service
                    .read_local_context_sync_record(&f.repo, id)
                    .unwrap();
                assert_eq!(after.state, LocalSyncState::IndexPending);
                assert!(!after.index_complete);
                assert_eq!(after.result, row.result);
                assert_ne!(
                    f.db()
                        .query_row(
                            "SELECT state FROM operation_records WHERE id=?1",
                            [row.local_record_id],
                            |r| r.get::<_, String>(0)
                        )
                        .unwrap(),
                    "completed"
                );
                if let Some(attempt) = row.active_resolution {
                    assert_ne!(f.db().query_row("SELECT state FROM local_context_sync_resolutions WHERE attempt_ulid=?1",[attempt.to_string()],|r|r.get::<_,String>(0)).unwrap(),"complete");
                }
                assert_eq!(
                    image(&f),
                    captured.borrow_mut().take().unwrap(),
                    "{point:?} {shape} {foreign}"
                );
                repair.borrow_mut().take().unwrap().restore(&f.context());
                let restored = image(&f);
                let resumed = retry(&f, id, shape);
                assert!(
                    matches!(resumed.code, ResultCode::Ok | ResultCode::AlreadyApplied),
                    "{point:?} {shape} {foreign}: {resumed:?}"
                );
                assert_eq!(image(&f), restored);
                assert_eq!(
                    f.service
                        .read_local_context_sync_record(&f.repo, id)
                        .unwrap()
                        .state,
                    LocalSyncState::Complete
                );
            }
        }
    }
}

#[test]
fn quality_metadata_retirement_rechecks_remaining_evidence_before_each_unlink() {
    for foreign in [
        "lock",
        "ref",
        "branch_log",
        "head_log",
        "MERGE_MODE",
        "MERGE_MSG",
    ] {
        let (f, _) = fixture(AuthoringKind::Ticket);
        let f = Rc::new(f);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        let captured = Rc::new(RefCell::new(None));
        let repair = Rc::new(RefCell::new(None));
        let retired = Rc::new(RefCell::new(Vec::new()));
        let hf = f.clone();
        let hc = captured.clone();
        let hr = repair.clone();
        let he = retired.clone();
        let parent = row.context_parent;
        resolve::HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |point| {
                if let resolve::Point::Retired(i) = point {
                    he.borrow_mut().push(i);
                    if i == 0 {
                        *hr.borrow_mut() = Some(change(&hf.context(), foreign, parent));
                        *hc.borrow_mut() = Some(image(&hf));
                    }
                }
                // Deliberately return normally: the implementation must revalidate,
                // rather than relying on an injected error to stop the release loop.
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
        assert_ne!(result.code, ResultCode::Ok, "{foreign}: {result:?}");
        assert_eq!(*retired.borrow(), vec![0], "{foreign}");
        assert_eq!(
            image(&f),
            captured.borrow_mut().take().unwrap(),
            "{foreign}"
        );
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        assert_eq!(row.state, LocalSyncState::Integrated);
        assert!(!row.artifacts_released && !row.index_complete);
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT phase FROM local_context_sync_steps WHERE ordinal=12",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "prepared"
        );
        assert!(!f.context().path().join("MERGE_HEAD").exists());
        assert!(f.context().path().join("MERGE_MSG").exists());
        assert!(f.context().path().join("MERGE_MODE").exists());
        repair.borrow_mut().take().unwrap().restore(&f.context());
        let restored = image(&f);
        let git = f.context();
        let branch = git.head().unwrap().name().unwrap().to_owned();
        let logs = git.reflog(&branch).unwrap().len();
        let resumed = resolve::resume(&f.service, &f.repo, id);
        assert_eq!(resumed.code, ResultCode::Ok, "{foreign}: {resumed:?}");
        assert_eq!(
            resumed.effects.commit_oid,
            row.result.map(|oid| oid.to_string())
        );
        assert_eq!(git.reflog(&branch).unwrap().len(), logs);
        assert_eq!(image(&f).objects, restored.objects);
        assert!(!git.path().join("MERGE_MSG").exists());
        assert!(!git.path().join("MERGE_MODE").exists());
    }
}

#[test]
fn quality_external_log_authority_is_not_recreated_from_current_git() {
    for corrupt in ["missing", "changed", "partially_lost_prepared"] {
        let (f, id) = pending("external");
        if corrupt == "missing" {
            f.db().execute("UPDATE local_context_sync_steps SET expected_metadata_digest=NULL WHERE ordinal=13",[]).unwrap();
        } else if corrupt == "changed" {
            f.db().execute("UPDATE local_context_sync_steps SET baseline_metadata_digest=zeroblob(32) WHERE ordinal=13",[]).unwrap();
        } else {
            // Retain the bound branch digest in the pre-observation adoption
            // state, but lose the HEAD digest. It is not a new unbound intent.
            f.db().execute_batch("UPDATE local_context_sync_records SET state='conflict_pending',result_oid=NULL,integration_proven=0,artifacts_released=0; UPDATE local_context_sync_steps SET phase='prepared',expected_index_digest=NULL,expected_metadata_digest=NULL WHERE ordinal=13;").unwrap();
            introduce_foreign(&f.context(), "branch_log");
        }
        let row = f
            .service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap();
        let before = image(&f);
        for _ in 0..2 {
            assert_ne!(retry(&f, id, "external").code, ResultCode::Ok, "{corrupt}");
        }
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap(),
            row
        );
        assert_eq!(image(&f), before);
        assert!(super::super::control::abandon(&f.service, &f.repo, id).is_err());
    }
}

fn incoming_entry(f: &Fixture, path: Vec<u8>, mode: u32, bytes: &[u8]) {
    let git = git2::Repository::open(f.root.path()).unwrap();
    let parent = git.head().unwrap().peel_to_commit().unwrap();
    let mut index = git2::Index::new().unwrap();
    index.read_tree(&parent.tree().unwrap()).unwrap();
    let oid = if mode == 0o160000 {
        parent.id()
    } else {
        git.blob(bytes).unwrap()
    };
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode,
            uid: 0,
            gid: 0,
            file_size: bytes.len() as u32,
            id: oid,
            flags: 0,
            flags_extended: 0,
            path,
        })
        .unwrap();
    let tree = git.find_tree(index.write_tree_to(&git).unwrap()).unwrap();
    let sig = git.signature().unwrap();
    let oid = git
        .commit(
            None,
            &sig,
            &sig,
            "incoming structural entry",
            &tree,
            &[&parent],
        )
        .unwrap();
    git.reference("refs/heads/main", oid, true, "incoming structural entry")
        .unwrap();
}
fn assert_owner_free_refusal(f: &Fixture) {
    let before = image(f);
    let observation = f.observe();
    let id = OperationId::new();
    let result = run::execute(
        &f.service,
        &f.repo,
        f.kind,
        &f.item,
        id,
        &observation.observation,
    );
    assert_eq!(result.state, None, "{result:?}");
    assert!(!result.effects.is_durable());
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
    assert_eq!(image(f), before);
}

#[test]
fn quality_conflict_structural_preflight_covers_nonconflict_and_conflict_stages() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for mode in [0o120000, 0o160000] {
            for conflicted in [false, true] {
                let (f, paths) = fixture(kind);
                let path = if conflicted {
                    paths[0].to_str().unwrap().as_bytes().to_vec()
                } else {
                    b"incoming-unsupported".to_vec()
                };
                incoming_entry(&f, path, mode, b"outside-target");
                assert_owner_free_refusal(&f);
                assert!(
                    !f.context()
                        .workdir()
                        .unwrap()
                        .join("incoming-unsupported")
                        .exists()
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn quality_conflict_with_non_utf8_incoming_path_refuses_before_acceptance() {
    use std::os::unix::ffi::OsStrExt;
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let (f, _) = fixture(kind);
        let path = b"incoming-\xff";
        incoming_entry(&f, path.to_vec(), 0o100644, b"incoming bytes\n");
        assert_owner_free_refusal(&f);
        assert!(
            !f.context()
                .workdir()
                .unwrap()
                .join(std::ffi::OsStr::from_bytes(path))
                .exists()
        );
    }
}

#[test]
fn quality_representable_unsupported_conflicts_remain_external_only() {
    for variant in ["code", "executable", "binary"] {
        let (f, paths) = fixture(AuthoringKind::Document);
        if variant == "code" {
            for (git, bytes) in [
                (f.context(), "local code\n"),
                (
                    git2::Repository::open(f.root.path()).unwrap(),
                    "incoming code\n",
                ),
            ] {
                fs::write(git.workdir().unwrap().join("ordinary"), bytes).unwrap();
                commit(&git);
            }
        } else {
            let git = git2::Repository::open(f.root.path()).unwrap();
            let source = if variant == "binary" {
                b"\0\xffbinary".to_vec()
            } else {
                let tree = git
                    .head()
                    .unwrap()
                    .peel_to_commit()
                    .unwrap()
                    .tree()
                    .unwrap();
                git.find_blob(tree.get_path(&paths[0]).unwrap().id())
                    .unwrap()
                    .content()
                    .to_vec()
            };
            incoming_entry(
                &f,
                paths[0].to_str().unwrap().as_bytes().to_vec(),
                if variant == "executable" {
                    0o100755
                } else {
                    0o100644
                },
                &source,
            );
        }
        let id = start(&f);
        let inspection = super::super::conflict::inspect(&f.service, &f.repo, id).unwrap();
        assert_eq!(
            inspection.eligibility,
            integration::validate::ConflictEligibility::ExternalResolutionRequired,
            "{variant}"
        );
        assert!(!inspection.can_start_resolution);
    }
}
