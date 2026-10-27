//! Read-only recovery advice must match the executor's full image proof.
use super::spec::authority_image;
use super::*;

fn commit_paths(git: &git2::Repository, paths: &[&str]) {
    let mut index = git.index().unwrap();
    index.read(true).unwrap();
    for path in paths {
        index.add_path(std::path::Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = git.head().unwrap().peel_to_commit().unwrap();
    let signature = git.signature().unwrap();
    git.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "fixture context edit",
        &tree,
        &[&parent],
    )
    .unwrap();
}

fn two_conflicts() -> (ConflictWorld, LocalContextResolveInput, Vec<Vec<u8>>) {
    let world = mutation::World::new();
    let id = items::TICKET_A;
    assert_eq!(
        world
            .execute(mutation::REQUEST_1, world.save(id, &world.token(id)))
            .code,
        ResultCode::Ok
    );
    let context = world.worktree(id);
    let git = git2::Repository::open(&context).unwrap();
    std::fs::write(
        context.join(mutation::DOCUMENT_PATH),
        items::document_source(mutation::DOCUMENT, "Local guide", ""),
    )
    .unwrap();
    commit_paths(&git, &[mutation::DOCUMENT_PATH]);
    let ticket = items::ticket_path(id);
    items::write(
        &world.root,
        &ticket,
        &items::ticket_source(id, "Incoming ticket", ""),
    );
    items::write(
        &world.root,
        mutation::DOCUMENT_PATH,
        &items::document_source(mutation::DOCUMENT, "Incoming guide", ""),
    );
    items::commit(
        &world.fixture,
        &[&ticket, mutation::DOCUMENT_PATH],
        items::COMMITTED_AT,
    );
    let observation = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(id),
        )
        .unwrap();
    let result = world.execute(
        mutation::REQUEST_2,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(id),
            observation: observation.observation.clone(),
        }),
    );
    assert_eq!(result.code, ResultCode::RecoveryRequired);
    let operation = OperationId::parse(result.operation_id.as_deref().unwrap()).unwrap();
    let view = world
        .service
        .inspect_local_context_sync(&world.resolved(), operation)
        .unwrap();
    assert!(view.can_start_resolution);
    assert_eq!(view.paths.len(), 2);
    assert_eq!(view.paths[0].display_path, ticket);
    assert_eq!(view.paths[1].display_path, mutation::DOCUMENT_PATH);
    let resolutions = view
        .paths
        .into_iter()
        .map(|entry| {
            let content = world
                .service
                .read_local_context_conflict(&world.resolved(), operation, &entry.token)
                .unwrap();
            LocalConflictResolutionInput {
                token: entry.token,
                source: content.local.unwrap(),
            }
        })
        .collect();
    let before = [ticket.as_str(), mutation::DOCUMENT_PATH]
        .into_iter()
        .map(|path| std::fs::read(context.join(path)).unwrap())
        .collect();
    let input = LocalContextResolveInput {
        root: world.root.clone(),
        operation_id: operation,
        observation: view.observation.unwrap(),
        resolutions,
    };
    (
        ConflictWorld {
            world,
            operation,
            start_observation: observation.observation,
        },
        input,
        before,
    )
}

fn images(world: &mutation::World) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
    items::index(world.data.path())
        .prepare(
            "SELECT manifest,baseline_v2 FROM local_context_sync_images ORDER BY operation_ulid",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
#[allow(clippy::result_large_err)] // Preserve the public read's by-value error contract.
fn inspect_unchanged(f: &ConflictWorld) -> Result<LocalContextSyncInspectionDto, ReadError> {
    let _lease = RepositoryService::hold_lease_for_testing(
        &f.world.root,
        f.world.data.path(),
        LeaseKind::Repository,
    )
    .unwrap();
    let git = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
    let git_files = super::super::support::repository_git_file_bytes(&f.world.fixture);
    let sql = authority_image(&f.world);
    let proof = images(&f.world);
    let result = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation);
    assert!(git == super::super::support::repository_and_worktree_snapshot(&f.world.fixture));
    assert!(git_files == super::super::support::repository_git_file_bytes(&f.world.fixture));
    assert!(sql == authority_image(&f.world));
    assert!(proof == images(&f.world));
    result
}
fn assert_retry(f: &ConflictWorld) {
    let view = inspect_unchanged(f).unwrap();
    assert_eq!(
        view.next_action,
        LocalContextSyncNextAction::RetryResolution
    );
    assert_eq!(
        view.required_request_id.as_deref(),
        Some(mutation::REQUEST_3)
    );
    assert!(!view.can_start_resolution);
}
struct CancelAt<'a> {
    stage: ProgressStage,
    token: &'a CancellationToken,
}
impl ProgressSink for CancelAt<'_> {
    fn event(&mut self, e: &ProgressEvent) {
        if e.stage == self.stage {
            self.token.cancel();
        }
    }
}

#[test]
fn spec_accepted_two_path_resolution_cancelled_before_first_write_is_retryable() {
    let (f, input, before) = two_conflicts();
    let token = CancellationToken::new();
    let answer = f.world.service.execute(
        MutationCall {
            request_id: RequestId::parse(mutation::REQUEST_3).unwrap(),
            confirmation: None,
            mutation: Mutation::LocalContextResolve(input),
        },
        &mut keys::SessionCredentials::new(mutation::NoPassphrase),
        &mut CancelAt {
            stage: ProgressStage::Writing,
            token: &token,
        },
        &token,
    );
    assert_eq!(answer.code, ResultCode::Cancelled);
    assert_eq!(
        f.world.record(mutation::REQUEST_3).unwrap().state,
        RequestState::Accepted
    );
    assert_eq!(items::index(f.world.data.path()).query_row("SELECT count(*) FROM local_context_sync_steps WHERE kind='path' AND phase='observed'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    for (path, bytes) in [
        items::ticket_path(items::TICKET_A),
        mutation::DOCUMENT_PATH.into(),
    ]
    .iter()
    .zip(before)
    {
        assert!(std::fs::read(f.world.worktree(items::TICKET_A).join(path)).unwrap() == bytes);
    }
    assert_retry(&f);
}

#[test]
fn spec_partial_two_path_resolution_keeps_untouched_preimages_retryable() {
    for observed in [true, false] {
        let (mut f, input, before) = two_conflicts();
        if observed {
            f.world.service =
                super::super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
                    .open_service(f.world.data.path());
            assert_ne!(
                f.world
                    .execute(
                        mutation::REQUEST_3,
                        Mutation::LocalContextResolve(copy_input(&input))
                    )
                    .code,
                ResultCode::Ok
            );
        } else {
            killed_resolution(&f, &input, "write");
        }
        assert_eq!(items::index(f.world.data.path()).query_row("SELECT count(*) FROM local_context_sync_steps WHERE kind='path' AND phase='observed'",[],|r|r.get::<_,i64>(0)).unwrap(),i64::from(observed));
        assert!(
            std::fs::read(
                f.world
                    .worktree(items::TICKET_A)
                    .join(mutation::DOCUMENT_PATH)
            )
            .unwrap()
                == before[1]
        );
        assert_retry(&f);
        assert_eq!(
            f.world
                .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(input))
                .code,
            ResultCode::Ok
        );
    }
}

#[test]
fn spec_observed_resolution_path_cannot_return_to_its_preimage() {
    let (mut f, input, before) = two_conflicts();
    f.world.service =
        super::super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
            .open_service(f.world.data.path());
    f.world
        .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(input));
    std::fs::write(
        f.world
            .worktree(items::TICKET_A)
            .join(items::ticket_path(items::TICKET_A)),
        &before[0],
    )
    .unwrap();
    let view = inspect_unchanged(&f).unwrap();
    assert_eq!(view.next_action, LocalContextSyncNextAction::ExternalRepair);
    assert!(!view.can_start_resolution);
}

fn ordinary(disposition: &str, phase: &str, filtered: bool) -> ConflictWorld {
    let mut world = mutation::World::new();
    let id = items::TICKET_A;
    assert_eq!(
        world
            .execute(mutation::REQUEST_1, world.save(id, &world.token(id)))
            .code,
        ResultCode::Ok
    );
    if filtered {
        let git = git2::Repository::open(world.worktree(id)).unwrap();
        git.config()
            .unwrap()
            .set_bool("core.autocrlf", true)
            .unwrap();
        std::fs::remove_file(world.worktree(id).join(mutation::DOCUMENT_PATH)).unwrap();
        git.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
    }
    if disposition == "fast_forward" {
        world
            .fixture
            .repository
            .find_reference("refs/heads/main")
            .unwrap()
            .set_target(
                world.branch_tip(id).unwrap(),
                "fixture primary contains context",
            )
            .unwrap();
    }
    if disposition != "contained" {
        items::write(
            &world.root,
            mutation::DOCUMENT_PATH,
            &items::document_source(mutation::DOCUMENT, "Incoming clean guide", ""),
        );
        items::commit(
            &world.fixture,
            &[mutation::DOCUMENT_PATH],
            items::COMMITTED_AT,
        );
    }
    let observation = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(id),
        )
        .unwrap();
    let input = Mutation::LocalContextSync(LocalContextSyncInput {
        root: world.root.clone(),
        kind: AuthoringKind::Ticket,
        id: items::item_id(id),
        observation: observation.observation.clone(),
    });
    let answer = if phase == "integrated" {
        let token = CancellationToken::new();
        world.service.execute(
            MutationCall {
                request_id: RequestId::parse(mutation::REQUEST_2).unwrap(),
                confirmation: None,
                mutation: input,
            },
            &mut keys::SessionCredentials::new(mutation::NoPassphrase),
            &mut CancelAt {
                stage: ProgressStage::Indexing,
                token: &token,
            },
            &token,
        )
    } else {
        let point = match phase {
            "prepared" => FailurePoint::LocalSyncAfterIntent,
            "candidate" => FailurePoint::LocalSyncAfterCandidate,
            "checkout" => FailurePoint::LocalSyncAfterCheckout,
            "reference" => FailurePoint::LocalSyncAfterRef,
            "index_pending" => FailurePoint::LocalSyncBeforeIndex,
            _ => panic!("unknown fixture phase"),
        };
        world.service = super::super::support::FailOnce::at(point).open_service(world.data.path());
        world.execute(mutation::REQUEST_2, input)
    };
    assert_ne!(answer.code, ResultCode::Ok);
    assert_ne!(answer.code, ResultCode::AlreadyApplied);
    let operation = OperationId::parse(answer.operation_id.as_deref().unwrap()).unwrap();
    let state: String = items::index(world.data.path())
        .query_row(
            "SELECT state FROM local_context_sync_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    if matches!(phase, "prepared" | "integrated" | "index_pending") {
        assert_eq!(state, phase);
    }
    if filtered {
        assert!(
            std::fs::read(world.worktree(id).join(mutation::DOCUMENT_PATH))
                .unwrap()
                .windows(2)
                .any(|b| b == b"\r\n")
        );
    }
    ConflictWorld {
        world,
        operation,
        start_observation: observation.observation,
    }
}

#[test]
fn spec_non_resolution_foreign_file_or_index_requires_external_repair() {
    for disposition in ["merge", "fast_forward", "contained"] {
        for phase in ["prepared", "integrated", "index_pending"] {
            for filtered in [false, true] {
                for index_change in [false, true] {
                    let f = ordinary(disposition, phase, filtered);
                    let valid = inspect_unchanged(&f).unwrap();
                    assert_eq!(valid.next_action, LocalContextSyncNextAction::Resume);
                    assert_eq!(
                        valid.required_request_id.as_deref(),
                        Some(mutation::REQUEST_2)
                    );
                    let git = git2::Repository::open(f.world.worktree(items::TICKET_A)).unwrap();
                    let path = if index_change {
                        git.path().join("index")
                    } else {
                        f.world
                            .worktree(items::TICKET_A)
                            .join(mutation::DOCUMENT_PATH)
                    };
                    let original = std::fs::read(&path).unwrap();
                    if index_change {
                        let mut index = git.index().unwrap();
                        index
                            .remove_path(std::path::Path::new(mutation::DOCUMENT_PATH))
                            .unwrap();
                        index.write().unwrap();
                    } else {
                        let mut foreign = original.clone();
                        foreign.extend_from_slice(b"foreign edit");
                        std::fs::write(&path, foreign).unwrap();
                    }
                    let view = inspect_unchanged(&f).unwrap();
                    assert_eq!(
                        view.next_action,
                        LocalContextSyncNextAction::ExternalRepair,
                        "{disposition}/{phase}/filtered={filtered}/index={index_change}"
                    );
                    assert!(!view.can_start_resolution);
                    std::fs::write(path, original).unwrap();
                    assert_eq!(
                        inspect_unchanged(&f).unwrap().next_action,
                        LocalContextSyncNextAction::Resume
                    );
                }
            }
        }
    }
}

#[test]
fn spec_non_resolution_images_require_coarse_intent_and_retained_authority() {
    for filtered in [false, true] {
        for phase in ["checkout", "reference"] {
            let f = ordinary("merge", phase, filtered);
            assert_eq!(
                inspect_unchanged(&f).unwrap().next_action,
                LocalContextSyncNextAction::Resume
            );
        }
        let f = ordinary("merge", "candidate", filtered);
        let candidate: String = items::index(f.world.data.path())
            .query_row(
                "SELECT candidate_oid FROM local_context_sync_records WHERE operation_ulid=?1",
                [f.operation.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        let git = git2::Repository::open(f.world.worktree(items::TICKET_A)).unwrap();
        let commit = git
            .find_commit(git2::Oid::from_str(&candidate).unwrap())
            .unwrap();
        git.checkout_tree(
            commit.as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )
        .unwrap();
        assert_eq!(
            inspect_unchanged(&f).unwrap().next_action,
            LocalContextSyncNextAction::ExternalRepair
        );
    }
    for remove in [true, false] {
        let f = ordinary("merge", "prepared", false);
        let db = items::index(f.world.data.path());
        if remove {
            db.execute("DELETE FROM local_context_sync_images", [])
                .unwrap();
        } else {
            db.execute(
                "UPDATE local_context_sync_images SET manifest=?1",
                [b"corrupt proof".as_slice()],
            )
            .unwrap();
        }
        assert_eq!(
            inspect_unchanged(&f).unwrap_err().code(),
            ResultCode::RecoveryRequired
        );
    }
}

#[test]
fn spec_non_resolution_filter_change_never_rebaselines_the_retained_image() {
    for phase in ["prepared", "candidate", "checkout", "index_pending"] {
        let f = ordinary("merge", phase, false);
        f.world.set_local_config("core.autocrlf", "true");
        let view = inspect_unchanged(&f).unwrap();
        assert_eq!(
            view.next_action,
            if matches!(phase, "prepared" | "candidate") {
                LocalContextSyncNextAction::ExternalRepair
            } else {
                LocalContextSyncNextAction::Resume
            },
            "{phase}"
        );
    }
}

#[test]
fn spec_non_resolution_read_rechecks_filter_and_image_authority_after_session_b() {
    for lose_proof in [false, true] {
        let f = ordinary("merge", "prepared", false);
        let root = f.world.root.clone();
        let data = f.world.data.path().to_owned();
        let sql = authority_image(&f.world);
        let proof = images(&f.world);
        f.world.service.set_observation_hook_for_testing(move || {
            if lose_proof {
                items::index(&data)
                    .execute("DELETE FROM local_context_sync_images", [])
                    .unwrap();
            } else {
                git2::Repository::open(&root)
                    .unwrap()
                    .config()
                    .unwrap()
                    .set_bool("core.autocrlf", true)
                    .unwrap();
            }
        });
        let _lease = RepositoryService::hold_lease_for_testing(
            &f.world.root,
            f.world.data.path(),
            LeaseKind::Repository,
        )
        .unwrap();
        let result = f
            .world
            .service
            .inspect_local_context_sync(&f.world.resolved(), f.operation);
        assert_eq!(result.unwrap_err().code(), ResultCode::ExternalChange);
        assert!(sql == authority_image(&f.world));
        if lose_proof {
            assert!(images(&f.world).is_empty());
        } else {
            assert!(proof == images(&f.world));
        }
    }
}

#[test]
fn spec_unobserved_conflict_progress_is_not_resume_authority() {
    let f = ConflictWorld::new();
    let db = items::index(f.world.data.path());
    // Lose the install observation while retaining its durable intent and Git
    // effects, the same ambiguity the coarse backend boundary preserves.
    db.execute_batch("BEGIN; UPDATE local_context_sync_records SET state='applying'; UPDATE local_context_sync_conflicts SET manifest=NULL; UPDATE local_context_sync_steps SET phase='prepared' WHERE kind='conflict_install'; COMMIT;").unwrap();
    let view = inspect_unchanged(&f).unwrap();
    assert_eq!(view.next_action, LocalContextSyncNextAction::ExternalRepair);
    assert!(!view.can_start_resolution);
}

#[test]
fn spec_non_resolution_integrated_state_requires_recorded_release_proof() {
    for disposition in ["contained", "merge"] {
        let f = ordinary(disposition, "index_pending", false);
        items::index(f.world.data.path()).execute("UPDATE local_context_sync_records SET artifacts_released=0 WHERE operation_ulid=?1",[f.operation.to_string()]).unwrap();
        assert_eq!(
            inspect_unchanged(&f).unwrap().next_action,
            LocalContextSyncNextAction::ExternalRepair
        );
    }
}

#[test]
fn spec_external_index_handoff_requires_recorded_release_proof() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    let view = inspect_unchanged(&f).unwrap();
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
    git.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "fixture external repair",
        &tree,
        &[&parents[0], &parents[1]],
    )
    .unwrap();
    git.cleanup_state().unwrap();
    f.world.service = super::super::support::FailOnce::at(FailurePoint::LocalSyncBeforeIndex)
        .open_service(f.world.data.path());
    assert_eq!(
        f.world
            .execute(
                super::super::REQUEST_A,
                Mutation::OperationResume(OperationResumeInput {
                    root: f.world.root.clone(),
                    operation_id: f.operation
                })
            )
            .code,
        ResultCode::DiscoveryPending
    );
    let db = items::index(f.world.data.path());
    db.execute(
        "UPDATE local_context_sync_records SET artifacts_released=0 WHERE operation_ulid=?1",
        [f.operation.to_string()],
    )
    .unwrap();
    assert_eq!(
        inspect_unchanged(&f).unwrap().next_action,
        LocalContextSyncNextAction::ExternalRepair
    );
    db.execute(
        "UPDATE local_context_sync_records SET artifacts_released=1 WHERE operation_ulid=?1",
        [f.operation.to_string()],
    )
    .unwrap();
    assert_eq!(
        inspect_unchanged(&f).unwrap().next_action,
        LocalContextSyncNextAction::Resume
    );
}
