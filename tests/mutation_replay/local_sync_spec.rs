//! Task 7 SPEC regressions through the production public boundaries.
use super::*;
use manyhands::results::{Effects, Outcome};

fn authority_image(world: &mutation::World) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    let db = items::index(world.data.path());
    [
        "request_records",
        "request_operations",
        "operation_records",
        "local_context_sync_request_control",
        "local_context_sync_records",
        "local_context_sync_resolutions",
        "local_context_sync_steps",
        "local_context_sync_execution",
        "local_context_sync_conflicts",
        "local_context_sync_resolution_cancellation",
    ]
    .into_iter()
    .map(|table| {
        let mut query = db
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = query.column_count();
        query
            .query_map([], |row| (0..columns).map(|c| row.get(c)).collect())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    })
    .collect()
}

fn partial(point: FailurePoint) -> (ConflictWorld, LocalContextResolveInput) {
    let mut f = ConflictWorld::new();
    let input = f.input();
    f.world.service = super::super::support::FailOnce::at(point).open_service(f.world.data.path());
    assert_ne!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
    (f, input)
}

#[test]
fn spec_cancel_reply_does_not_acknowledge_a_new_generation() {
    let (f, input) = partial(FailurePoint::LocalSyncAfterResolutionPath);
    let id = RequestId::parse(mutation::REQUEST_3).unwrap();
    assert!(
        !f.world
            .service
            .cancel_request(id)
            .data
            .unwrap()
            .acknowledged
    );
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Cancelled
    );
    let second = f.world.service.cancel_request(id).data.unwrap();
    assert!(second.requested);
    assert!(!second.acknowledged);
    let third = f.world.service.cancel_request(id).data.unwrap();
    assert!(third.requested);
    assert!(!third.acknowledged);
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Cancelled
    );
    let view = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    assert!(view.cancel_acknowledged);
    assert!(!view.cancel_requested);
}

#[test]
fn spec_resume_aliases_use_canonical_local_scope() {
    for needs_source in [false, true] {
        for alias in 0..3 {
            if alias != 0 && !cfg!(unix) {
                continue;
            }
            let (f, _) = partial(if needs_source {
                FailurePoint::LocalSyncAfterResolutionPath
            } else {
                FailurePoint::LocalSyncAfterCandidate
            });
            let root = match alias {
                0 => f.world.root.join("."),
                1 => f.world.root.clone(),
                _ => {
                    let link = f.world.data.path().join("root-alias");
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(&f.world.root, &link).unwrap();
                    link
                }
            };
            // A genuinely relative path can be expressed without changing the
            // process cwd (tests run concurrently).
            let root = if alias == 1 {
                let cwd = std::env::current_dir().unwrap();
                let mut relative = std::path::PathBuf::new();
                for c in cwd.components() {
                    if matches!(c, std::path::Component::Normal(_)) {
                        relative.push("..");
                    }
                }
                relative.push(f.world.root.strip_prefix("/").unwrap());
                relative
            } else {
                root
            };
            let answer = f.world.execute(
                super::super::REQUEST_A,
                Mutation::OperationResume(OperationResumeInput {
                    root,
                    operation_id: f.operation,
                }),
            );
            let canonical = f.world.execute(
                super::super::REQUEST_A,
                Mutation::OperationResume(OperationResumeInput {
                    root: f.world.root.clone(),
                    operation_id: f.operation,
                }),
            );
            assert_eq!(
                serde_json::to_value(&answer).unwrap(),
                serde_json::to_value(&canonical).unwrap()
            );
            if needs_source {
                assert_eq!(answer.code, ResultCode::OriginalRequestRequired);
                let Some(MutationDataDto::LocalContextSync(data)) = answer.data else {
                    panic!("local resume data")
                };
                assert_eq!(
                    data.required_request_id.as_deref(),
                    Some(mutation::REQUEST_3)
                );
                assert!(f.world.record(super::super::REQUEST_A).is_none());
            } else {
                assert_eq!(answer.code, ResultCode::Ok);
            }
        }
    }
}

#[test]
fn spec_inspection_requires_an_imported_candidate_and_refuses_foreign_locks() {
    let (f, _) = partial(FailurePoint::LocalSyncBeforeObjectImport);
    let _lease = RepositoryService::hold_lease_for_testing(
        &f.world.root,
        f.world.data.path(),
        LeaseKind::Repository,
    )
    .unwrap();
    let before = authority_image(&f.world);
    let view = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    assert_eq!(
        view.next_action,
        LocalContextSyncNextAction::RetryResolution
    );
    assert_eq!(
        view.required_request_id.as_deref(),
        Some(mutation::REQUEST_3)
    );
    assert!(!view.can_start_resolution);
    assert!(authority_image(&f.world) == before);
}
#[test]
fn spec_inspection_stable_foreign_locks_are_external_only() {
    for lock in ["index.lock", "HEAD.lock"] {
        let f = ConflictWorld::new();
        let git = git2::Repository::open(f.world.worktree(items::TICKET_A)).unwrap();
        std::fs::write(git.path().join(lock), b"foreign lock").unwrap();
        let before = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
        let authority = authority_image(&f.world);
        let _lease = RepositoryService::hold_lease_for_testing(
            &f.world.root,
            f.world.data.path(),
            LeaseKind::Repository,
        )
        .unwrap();
        let view = f
            .world
            .service
            .inspect_local_context_sync(&f.world.resolved(), f.operation)
            .unwrap();
        assert_eq!(
            view.next_action,
            LocalContextSyncNextAction::ExternalRepair,
            "{lock}"
        );
        assert!(!view.can_start_resolution);
        assert!(
            before == super::super::support::repository_and_worktree_snapshot(&f.world.fixture)
        );
        assert!(authority_image(&f.world) == authority);
    }
}

#[cfg(unix)]
#[test]
fn spec_actual_conflict_path_permission_denial_is_repository_inaccessible() {
    use std::os::unix::fs::PermissionsExt;
    // A root process cannot enforce this permission control; the portable
    // recorded-blob injection remains a separate existing test.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let f = ConflictWorld::new();
    let input = f.input();
    let path = f
        .world
        .worktree(items::TICKET_A)
        .join(items::ticket_path(items::TICKET_A));
    let before = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
    let requests = f.world.request_rows();
    let authority = authority_image(&f.world);
    let permissions = std::fs::metadata(&path).unwrap().permissions();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let read = f.world.service.read_local_context_conflict(
        &f.world.resolved(),
        f.operation,
        &input.resolutions[0].token,
    );
    std::fs::set_permissions(&path, permissions).unwrap();
    let code = match read {
        Ok(_) => panic!("permission denial returned side bytes"),
        Err(e) => e.code(),
    };
    assert_eq!(code, ResultCode::RepositoryInaccessible);
    assert!(before == super::super::support::repository_and_worktree_snapshot(&f.world.fixture));
    assert_eq!(f.world.request_rows(), requests);
    assert!(authority_image(&f.world) == authority);
}

struct AtStage<F> {
    stage: ProgressStage,
    skip: usize,
    action: Option<F>,
}
impl<F: FnOnce()> ProgressSink for AtStage<F> {
    fn event(&mut self, e: &ProgressEvent) {
        if e.stage == self.stage {
            if self.skip > 0 {
                self.skip -= 1;
            } else if let Some(f) = self.action.take() {
                f();
            }
        }
    }
}
fn execute_at(
    service: &RepositoryService,
    id: &str,
    input: Mutation,
    stage: ProgressStage,
    action: impl FnOnce(),
) -> manyhands::results::Envelope<MutationDataDto> {
    service.execute(
        MutationCall {
            request_id: RequestId::parse(id).unwrap(),
            confirmation: None,
            mutation: input,
        },
        &mut keys::SessionCredentials::new(mutation::NoPassphrase),
        &mut AtStage {
            stage,
            skip: 0,
            action: Some(action),
        },
        &CancellationToken::new(),
    )
}

fn race_resolution_after_resume_read(
    f: &ConflictWorld,
    input: LocalContextResolveInput,
    failure: Option<FailurePoint>,
) -> (manyhands::results::Envelope<MutationDataDto>, ResultCode) {
    let (start, ready) = std::sync::mpsc::channel();
    let (done, finished) = std::sync::mpsc::channel();
    let data = f.world.data.path().to_owned();
    let worker = std::thread::spawn(move || {
        ready
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap();
        let service = match failure {
            Some(point) => super::super::support::FailOnce::at(point).open_service(&data),
            None => RepositoryService::open_at(&data).unwrap(),
        };
        let result = mutation::execute_with(
            &service,
            mutation::REQUEST_3,
            Mutation::LocalContextResolve(input),
        );
        done.send(()).unwrap();
        result.code
    });
    let mut barrier = AtStage {
        stage: ProgressStage::Observing,
        skip: 1,
        action: Some(|| {
            start.send(()).unwrap();
            finished
                .recv_timeout(std::time::Duration::from_secs(60))
                .unwrap();
        }),
    };
    let result = f.world.service.execute(
        MutationCall {
            request_id: RequestId::parse(super::super::REQUEST_A).unwrap(),
            confirmation: None,
            mutation: Mutation::OperationResume(OperationResumeInput {
                root: f.world.root.clone(),
                operation_id: f.operation,
            }),
        },
        &mut keys::SessionCredentials::new(mutation::NoPassphrase),
        &mut barrier,
        &CancellationToken::new(),
    );
    assert!(
        barrier.action.is_none(),
        "resume read barrier was not reached"
    );
    (result, worker.join().unwrap())
}
#[test]
fn spec_missing_start_or_active_resolution_control_fences_the_live_executor() {
    for through_resume in [false, true] {
        let (f, input) = if through_resume {
            partial(FailurePoint::LocalSyncAfterCandidate)
        } else {
            let f = ConflictWorld::new();
            let input = f.input();
            (f, input)
        };
        let missing = if through_resume {
            mutation::REQUEST_3
        } else {
            mutation::REQUEST_2
        };
        let before = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
        let db = items::index(f.world.data.path());
        let input = if through_resume {
            Mutation::OperationResume(OperationResumeInput {
                root: f.world.root.clone(),
                operation_id: f.operation,
            })
        } else {
            Mutation::LocalContextResolve(input)
        };
        let id = if through_resume {
            super::super::REQUEST_A
        } else {
            mutation::REQUEST_3
        };
        let retained = std::cell::RefCell::new(None);
        let answer = execute_at(
            &f.world.service,
            id,
            input,
            if through_resume {
                ProgressStage::UpdatingLocal
            } else {
                ProgressStage::Writing
            },
            || {
                assert_eq!(
                    db.execute(
                        "DELETE FROM local_context_sync_request_control WHERE request_ulid=?1",
                        [missing]
                    )
                    .unwrap(),
                    1
                );
                *retained.borrow_mut() = Some(authority_image(&f.world));
            },
        );
        assert_ne!(answer.code, ResultCode::Ok);
        assert!(
            before == super::super::support::repository_and_worktree_snapshot(&f.world.fixture),
            "effect after loss of required control"
        );
        assert_eq!(f.world.record(id).unwrap().state, RequestState::Accepted);
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM local_context_sync_request_control WHERE request_ulid=?1",
                [missing],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert!(retained.borrow().as_ref() == Some(&authority_image(&f.world)));
    }
}

#[test]
fn spec_parent_completion_between_resume_read_and_link_releases_unlinked_u() {
    let (f, input) = partial(FailurePoint::LocalSyncAfterCandidate);
    let (result, resolution) = race_resolution_after_resume_read(&f, input, None);
    assert_eq!(resolution, ResultCode::Ok);
    assert_eq!(result.code, ResultCode::ExternalChange);
    assert_eq!(result.effects, Effects::not_requested());
    assert!(f.world.record(super::super::REQUEST_A).is_none());
    assert_eq!(
        f.world.record(mutation::REQUEST_3).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(
        items::index(f.world.data.path())
            .query_row(
                "SELECT count(*) FROM local_context_sync_resolutions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn spec_partial_resolution_between_resume_read_and_link_requires_r_without_linking_u() {
    let f = ConflictWorld::new();
    let input = f.input();
    let (result, resolution) = race_resolution_after_resume_read(
        &f,
        input,
        Some(FailurePoint::LocalSyncAfterResolutionPath),
    );
    assert_ne!(resolution, ResultCode::Ok);
    assert_eq!(result.code, ResultCode::OriginalRequestRequired);
    assert_eq!(result.effects, Effects::not_requested());
    assert!(f.world.record(super::super::REQUEST_A).is_none());
    let Some(MutationDataDto::LocalContextSync(data)) = result.data else {
        panic!("local result")
    };
    assert_eq!(
        data.required_request_id.as_deref(),
        Some(mutation::REQUEST_3)
    );
}

#[test]
fn spec_linked_resume_keeps_its_receipt_after_another_request_completes() {
    let (mut f, input) = partial(FailurePoint::LocalSyncAfterCandidate);
    f.world.service = super::super::support::FailOnce::at(FailurePoint::LocalSyncBeforeIndex)
        .open_service(f.world.data.path());
    let resume = || {
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        })
    };
    assert_eq!(
        f.world.execute(super::super::REQUEST_A, resume()).code,
        ResultCode::DiscoveryPending
    );
    assert_eq!(
        f.world
            .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(input))
            .code,
        ResultCode::Ok
    );
    let first = f.world.execute(super::super::REQUEST_A, resume());
    assert_eq!(first.code, ResultCode::Ok);
    let again = f.world.execute(super::super::REQUEST_A, resume());
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(again).unwrap()
    );
}

#[test]
fn spec_no_drop_written_unobserved_path_has_read_only_mismatch_history() {
    let f = ConflictWorld::new();
    let input = f.input();
    killed_resolution(&f, &input, "write");
    let db = items::index(f.world.data.path());
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM local_context_sync_steps WHERE kind='path' AND phase='observed'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT effect_write FROM local_context_sync_request_control WHERE request_ulid=?1",
            [mutation::REQUEST_3],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "unchanged"
    );
    let before = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
    let record = f.world.record(mutation::REQUEST_3).unwrap();
    let authority = authority_image(&f.world);
    let mut changed = copy_input(&input);
    changed.observation.push('0');
    let mismatch = f
        .world
        .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(changed));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.effects.write, WriteEffect::Written);
    assert_eq!(mismatch.outcome, Outcome::Partial);
    assert_eq!(record, f.world.record(mutation::REQUEST_3).unwrap());
    assert!(authority_image(&f.world) == authority);
    assert!(before == super::super::support::repository_and_worktree_snapshot(&f.world.fixture));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM local_context_sync_steps WHERE kind='path' AND phase='observed'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT effect_write FROM local_context_sync_request_control WHERE request_ulid=?1",
            [mutation::REQUEST_3],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "unchanged"
    );
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(result.code, ResultCode::Ok);
    let replay = f
        .world
        .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(input));
    assert_eq!(replay.effects, result.effects);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM local_context_sync_resolutions",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn spec_inspection_foreign_images_and_candidate_identity_are_read_only() {
    for phase in 0..3 {
        let (f, token) = if phase == 0 {
            let f = ConflictWorld::new();
            let token = f.input().resolutions[0].token.clone();
            (f, token)
        } else {
            let (f, input) = partial(if phase == 1 {
                FailurePoint::LocalSyncAfterResolutionPath
            } else {
                FailurePoint::LocalSyncAfterCandidate
            });
            (f, input.resolutions[0].token.clone())
        };
        let path = f
            .world
            .worktree(items::TICKET_A)
            .join(items::ticket_path(items::TICKET_A));
        let mut source = std::fs::read(&path).unwrap();
        source.extend_from_slice(b"foreign bytes");
        std::fs::write(&path, source).unwrap();
        let authority = authority_image(&f.world);
        let before = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
        let _lease = RepositoryService::hold_lease_for_testing(
            &f.world.root,
            f.world.data.path(),
            LeaseKind::Repository,
        )
        .unwrap();
        let view = f
            .world
            .service
            .inspect_local_context_sync(&f.world.resolved(), f.operation)
            .unwrap();
        assert_eq!(view.next_action, LocalContextSyncNextAction::ExternalRepair);
        assert!(!view.can_start_resolution);
        assert!(view.required_request_id.is_none());
        if phase != 0 {
            assert!(
                f.world
                    .service
                    .read_local_context_conflict(&f.world.resolved(), f.operation, &token)
                    .unwrap()
                    .historical
            );
        }
        assert!(authority_image(&f.world) == authority);
        assert!(
            before == super::super::support::repository_and_worktree_snapshot(&f.world.fixture)
        );
    }
    let (f, _) = partial(FailurePoint::LocalSyncAfterCandidate);
    items::index(f.world.data.path())
        .execute(
            "UPDATE local_context_sync_execution SET commit_name='foreign signature'",
            [],
        )
        .unwrap();
    let authority = authority_image(&f.world);
    let view = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    assert_eq!(view.next_action, LocalContextSyncNextAction::ExternalRepair);
    assert!(!view.can_start_resolution);
    assert!(authority_image(&f.world) == authority);
}

#[test]
fn spec_read_and_resume_share_external_result_proof() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    let view = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    let git = git2::Repository::open(f.world.worktree(items::TICKET_A)).unwrap();
    let path = items::ticket_path(items::TICKET_A);
    std::fs::write(
        git.workdir().unwrap().join(&path),
        &input.resolutions[0].source,
    )
    .unwrap();
    let mut index = git.index().unwrap();
    index.add_path(std::path::Path::new(&path)).unwrap();
    index.write().unwrap();
    let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git.signature().unwrap();
    let parents = [
        git.find_commit(git2::Oid::from_str(&view.context_parent).unwrap())
            .unwrap(),
        git.find_commit(git2::Oid::from_str(&view.primary_parent).unwrap())
            .unwrap(),
    ];
    let oid = git
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "explicit external repair",
            &tree,
            &[&parents[0], &parents[1]],
        )
        .unwrap();
    git.cleanup_state().unwrap();
    git.config().unwrap().set_str("user.name", "").unwrap();
    let before = authority_image(&f.world);
    let view = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    assert_eq!(view.next_action, LocalContextSyncNextAction::Resume);
    assert!(!view.can_start_resolution);
    assert_eq!(view.reason, LocalContextSyncReason::None);
    assert!(authority_image(&f.world) == before);
    f.world.service = super::super::support::FailOnce::at(FailurePoint::LocalSyncBeforeIndex)
        .open_service(f.world.data.path());
    let resume = || {
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        })
    };
    assert_eq!(
        f.world.execute(super::super::REQUEST_A, resume()).code,
        ResultCode::DiscoveryPending
    );
    let view = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    assert_eq!(view.next_action, LocalContextSyncNextAction::Resume);
    assert_eq!(view.reason, LocalContextSyncReason::IndexPending);
    let result = f.world.execute(super::super::REQUEST_A, resume());
    assert_eq!(result.code, ResultCode::Ok);
    assert_eq!(
        result.effects.commit_oid.as_deref(),
        Some(oid.to_string().as_str())
    );
    assert_eq!(
        result.effects.checkpoint,
        manyhands::results::CheckpointEffect::Unchanged
    );
}
