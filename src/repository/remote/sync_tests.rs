use super::*;
#[cfg(unix)]
use crate::repository::OwnedPathBoundary;
use crate::repository::remote::reservation::commit_observation_batch;
use crate::repository::{EnableRepositoryRequest, FailurePoint, REGISTRY_FILE};
#[cfg(target_os = "linux")]
use std::fs;
struct NoPrompt;
impl SessionCredentialProvider for NoPrompt {
    fn request_passphrase(
        &mut self,
        _: &crate::repository::keys::UnlockRequest,
    ) -> crate::repository::keys::PassphraseResponse {
        panic!("unexpected prompt")
    }
}
fn fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(root.path()).unwrap();
    repo.set_head("refs/heads/main").unwrap();
    repo.config()
        .unwrap()
        .set_str("user.name", "Fixture")
        .unwrap();
    repo.config()
        .unwrap()
        .set_str("user.email", "fixture@example.invalid")
        .unwrap();
    fs::write(root.path().join("fixture.txt"), b"original\n").unwrap();
    commit_all(&repo);
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new()
            })
            .unwrap(),
        EnableRepositoryOutcome::Enabled { .. }
    ));
    commit_all(&repo);
    (root, data, service)
}
fn commit_all(repo: &git2::Repository) -> git2::Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}
fn request(root: &Path) -> SynchronizeRemoteRequest {
    SynchronizeRemoteRequest {
        root: root.into(),
        operation_id: OperationId::new(),
        target: SynchronizationTarget::Primary,
        approval: None,
        confirmed_identity: None,
        restart: false,
    }
}
#[test]
fn no_remote_refreshes_same_id_without_remote_envelope_or_git_mutation() {
    let (root, data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let oid = repo.head().unwrap().target().unwrap();
    let index = fs::read(repo.path().join("index")).unwrap();
    let req = request(root.path());
    for _ in 0..2 {
        assert_eq!(
            service
                .synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
            SynchronizationResult::Complete(SynchronizationOutcome::PublishPending {
                target: req.target.clone(),
                local_oid: oid,
                reason: PublishPendingReason::NoPublicationRemote
            })
        );
    }
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE operation_ulid=?1 AND action='refresh'",
            [req.operation_id.to_string()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(repo.head().unwrap().target(), Some(oid));
    assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
}
#[test]
fn dirty_preflight_preserves_bytes_index_and_refs() {
    let (root, _data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let oid = repo.head().unwrap().target();
    let index = fs::read(repo.path().join("index")).unwrap();
    fs::write(root.path().join("fixture.txt"), b"dirty sentinel").unwrap();
    assert!(matches!(
        service.synchronize_remote(request(root.path()), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::WorktreeNotClean { .. })
    ));
    assert_eq!(
        fs::read(root.path().join("fixture.txt")).unwrap(),
        b"dirty sentinel"
    );
    assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(repo.head().unwrap().target(), oid);
}
#[test]
fn wrong_branch_and_unmaterialized_context_are_nonmutating() {
    let (root, _data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let oid = repo.head().unwrap().target().unwrap();
    repo.reference("refs/heads/wrong", oid, false, "fixture")
        .unwrap();
    repo.set_head("refs/heads/wrong").unwrap();
    assert!(matches!(
        service.synchronize_remote(request(root.path()), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::TargetNotMaterialized)
    ));
    repo.set_head("refs/heads/main").unwrap();
    let mut req = request(root.path());
    req.target = SynchronizationTarget::Context {
        kind: crate::repository::AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    };
    assert!(matches!(
        service.synchronize_remote(req, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::TargetNotMaterialized)
    ));
    assert_eq!(repo.head().unwrap().target(), Some(oid));
}
#[test]
fn foreign_merge_rebase_and_cherry_pick_metadata_are_refused_without_adoption() {
    let (root, _data, _service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let head = repo.head().unwrap().target().unwrap().to_string();
    for metadata in ["MERGE_HEAD", "REBASE_HEAD", "CHERRY_PICK_HEAD"] {
        fs::write(repo.path().join(metadata), format!("{head}\n")).unwrap();
        assert!(matches!(
            local_target(root.path(), "main", &SynchronizationTarget::Primary),
            Err(SynchronizationError::WorktreeConflicted { .. })
        ));
        fs::remove_file(repo.path().join(metadata)).unwrap();
    }
    assert!(local_target(root.path(), "main", &SynchronizationTarget::Primary).is_ok());
}

#[test]
fn candidate_head_restart_observes_the_recorded_merge_without_another_ref_transition() {
    let (root, _data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), false)
    })
    .unwrap();
    let base = repo.head().unwrap().target().unwrap();
    fs::write(root.path().join("local.txt"), b"local\n").unwrap();
    let local = commit_all(&repo);
    repo.set_head_detached(base).unwrap();
    repo.checkout_tree(&repo.find_object(base, None).unwrap(), None)
        .unwrap();
    fs::write(root.path().join("incoming.txt"), b"incoming\n").unwrap();
    let incoming = commit_all(&repo);
    repo.set_head("refs/heads/main").unwrap();
    repo.reference("refs/heads/main", local, true, "fixture")
        .unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let tree = repo.find_commit(local).unwrap().tree().unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let candidate = repo
        .commit(
            None,
            &signature,
            &signature,
            "merge",
            &tree,
            &[
                &repo.find_commit(local).unwrap(),
                &repo.find_commit(incoming).unwrap(),
            ],
        )
        .unwrap();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan);
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("reservation"),
    };
    let evidence = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(local),
        tracking_oid: Some(incoming),
        primary_tracking_oid: Some(incoming),
        ..state::SynchronizationEvidence::default()
    };
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &evidence,
        )
        .unwrap();
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    let observation =
        RemoteRefObservation::from_advertisement(&plan, "refs/heads/main", incoming, None).unwrap();
    commit_observation_batch(
        &service,
        root.path(),
        &owner,
        &plan,
        std::slice::from_ref(&observation),
        1,
    )
    .unwrap();
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: merge::IntegrationStage::Primary,
                local_oid: local,
                incoming_oid: incoming,
                baseline_tree_oid: tree.id(),
                baseline_index_digest: index_digest(tree.id()),
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 0, Some(candidate))
        .unwrap();
    fast_forward(&repo, "refs/heads/main", local, candidate).unwrap();
    let head = repo.head().unwrap().target().unwrap();
    let index = fs::read(repo.path().join("index")).unwrap();
    let worktree = fs::read(root.path().join("local.txt")).unwrap();
    let resumed = match service
        .restart_remote_synchronization(root.path(), owner.operation_id(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("restart"),
    };
    let mut resume_evidence = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap()
    .sync_evidence;
    let pending = reconcile_pending_candidate(
        &service,
        root.path(),
        "main",
        &SynchronizationTarget::Primary,
        &resumed,
        &mut resume_evidence,
    )
    .unwrap()
    .expect("candidate evidence");
    assert_eq!(pending.oid, candidate);
    // The outer envelope remains reconciling until a fresh Fetch batch commits.
    service
        .remote_safe_point(root.path(), &resumed, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    commit_observation_batch(&service, root.path(), &resumed, &plan, &[observation], 2).unwrap();
    finalize_reconciled_candidate(&service, root.path(), &resumed, pending, &resume_evidence)
        .unwrap();
    assert_eq!(repo.head().unwrap().target(), Some(head));
    assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(fs::read(root.path().join("local.txt")).unwrap(), worktree);
    let commit = repo.find_commit(candidate).unwrap();
    assert_eq!(
        [commit.parent_id(0).unwrap(), commit.parent_id(1).unwrap()],
        [local, incoming]
    );
    let record = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    assert_eq!(
        record.sync_checkpoint,
        Some(state::SynchronizationCheckpoint::LocalFastForwarded)
    );
    assert!(!record.reconciliation_required);
}

#[test]
fn candidate_third_head_restart_preserves_applying_evidence() {
    let (root, _data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), false)
    })
    .unwrap();
    let base = repo.head().unwrap().target().unwrap();
    fs::write(root.path().join("local.txt"), b"local\n").unwrap();
    let local = commit_all(&repo);
    repo.set_head_detached(base).unwrap();
    repo.checkout_tree(&repo.find_object(base, None).unwrap(), None)
        .unwrap();
    fs::write(root.path().join("incoming.txt"), b"incoming\n").unwrap();
    let incoming = commit_all(&repo);
    repo.set_head("refs/heads/main").unwrap();
    repo.reference("refs/heads/main", local, true, "fixture")
        .unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let tree = repo.find_commit(local).unwrap().tree().unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let candidate = repo
        .commit(
            None,
            &signature,
            &signature,
            "merge",
            &tree,
            &[
                &repo.find_commit(local).unwrap(),
                &repo.find_commit(incoming).unwrap(),
            ],
        )
        .unwrap();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan);
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("reservation"),
    };
    let evidence = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(local),
        tracking_oid: Some(incoming),
        primary_tracking_oid: Some(incoming),
        ..state::SynchronizationEvidence::default()
    };
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &evidence,
        )
        .unwrap();
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    let observation =
        RemoteRefObservation::from_advertisement(&plan, "refs/heads/main", incoming, None).unwrap();
    commit_observation_batch(&service, root.path(), &owner, &plan, &[observation], 1).unwrap();
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: merge::IntegrationStage::Primary,
                local_oid: local,
                incoming_oid: incoming,
                baseline_tree_oid: tree.id(),
                baseline_index_digest: index_digest(tree.id()),
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 0, Some(candidate))
        .unwrap();
    let third = base;
    repo.reference("refs/heads/main", third, true, "third head")
        .unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let head = repo.head().unwrap().target();
    let index = fs::read(repo.path().join("index")).unwrap();
    let worktree = fs::read(root.path().join("fixture.txt")).unwrap();
    let resumed = match service
        .restart_remote_synchronization(root.path(), owner.operation_id(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("restart"),
    };
    let before = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    let mut resume_evidence = before.sync_evidence.clone();
    assert!(matches!(
        reconcile_pending_candidate(
            &service,
            root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &resumed,
            &mut resume_evidence
        ),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(repo.head().unwrap().target(), head);
    assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(fs::read(root.path().join("fixture.txt")).unwrap(), worktree);
    let after = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, owner.operation_id())
    })
    .unwrap()
    .unwrap();
    assert_eq!(after.sync_checkpoint, before.sync_checkpoint);
    assert_eq!(after.sync_evidence, before.sync_evidence);
    assert!(after.reconciliation_required);
}

#[test]
fn divergence_recheck_refuses_configuration_and_tracking_changes_before_mutation() {
    for mutate_configuration in [true, false] {
        let (root, _data, service) = fixture();
        let repo = git2::Repository::open(root.path()).unwrap();
        repo.remote("origin", "ssh://example.invalid/one.git")
            .unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&plan), false)
        })
        .unwrap();
        let base = repo.head().unwrap().target().unwrap();
        let local = child(&repo, base, b"local\n");
        let incoming = child(&repo, base, b"incoming\n");
        repo.reference("refs/heads/main", local, true, "fixture")
            .unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        repo.reference(
            plan.primary().tracking_ref(),
            incoming,
            true,
            "fixture tracking",
        )
        .unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let owner = match service
            .reserve_remote_operation(root.path(), OperationId::new(), &target)
            .unwrap()
        {
            RemoteReservationOutcome::Reserved(owner) => owner,
            _ => panic!("reservation"),
        };
        let evidence = state::SynchronizationEvidence {
            expected_oid: Some(local),
            local_oid: Some(local),
            tracking_oid: Some(incoming),
            primary_tracking_oid: Some(incoming),
            ..state::SynchronizationEvidence::default()
        };
        service
            .checkpoint_synchronization(
                root.path(),
                &owner,
                state::SynchronizationCheckpoint::FetchPrepared,
                &evidence,
            )
            .unwrap();
        service
            .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
            .unwrap();
        let observation =
            RemoteRefObservation::from_advertisement(&plan, "refs/heads/main", incoming, None)
                .unwrap();
        commit_observation_batch(&service, root.path(), &owner, &plan, &[observation], 1).unwrap();
        let configuration = service
            .observation_configuration(root.path(), &plan)
            .unwrap();
        let hook_root = root.path().to_owned();
        let tracking = plan.primary().tracking_ref().to_owned();
        let hook = super::super::observation_tests::install_hook(move |point| {
            if point != RemoteOperationSafePoint::BeforeLocalMutation {
                return;
            }
            let repository = git2::Repository::open(&hook_root).unwrap();
            if mutate_configuration {
                repository
                    .remote_set_url("origin", "ssh://example.invalid/two.git")
                    .unwrap();
            } else {
                repository
                    .reference(&tracking, local, true, "tracking race")
                    .unwrap();
            }
        });
        let request = request(root.path());
        let result = integrate_divergence(
            &service,
            DivergenceInputs {
                root: root.path(),
                primary_branch: "main",
                target: &SynchronizationTarget::Primary,
                request: &request,
                owner: &owner,
                plan: &plan,
                configuration: &configuration,
                selected: plan.primary(),
                primary_tracking: Some(incoming),
                selected_tracking: Some(incoming),
                context: None,
                primary: incoming,
            },
        );
        assert!(
            matches!(result, Err(SynchronizationError::ExternalChange)),
            "{result:?}"
        );
        drop(hook);
        assert_eq!(repo.head().unwrap().target(), Some(local));
        assert_eq!(repo.refname_to_id("refs/heads/main").unwrap(), local);
    }
}

#[test]
fn no_remote_index_pending_retains_exact_outcome_and_replays_refresh_only() {
    let (root, _data, service) = fixture();
    let req = request(root.path());
    *service.failure_point.lock().unwrap() = Some(FailurePoint::BeforeIndexTransactionCommit);
    let result = service
        .synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt))
        .unwrap();
    let SynchronizationResult::IndexPending(pending) = result else {
        panic!("missing pending result")
    };
    *service.failure_point.lock().unwrap() = None;
    assert_eq!(
        service
            .synchronize_remote(req, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        SynchronizationResult::Complete(pending.authoritative)
    );
}

#[test]
fn local_only_replay_freezes_oid_and_rejects_target_or_plain_refresh_reuse() {
    let (root, _data, service) = fixture();
    let req = request(root.path());
    let original = service
        .synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt))
        .unwrap();
    let repo = git2::Repository::open(root.path()).unwrap();
    fs::write(root.path().join("fixture.txt"), b"later\n").unwrap();
    commit_all(&repo);
    // Even newly configured publication must not turn the old ID into a push.
    fs::write(
        root.path().join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
    )
    .unwrap();
    assert_eq!(
        service
            .synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        original
    );
    let mut wrong = req;
    wrong.target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    };
    assert!(matches!(
        service.synchronize_remote(wrong, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::OperationMismatch,
            ..
        }))
    ));
    let other = request(root.path());
    service
        .refresh_repository(RefreshRepositoryRequest {
            root: root.path().into(),
            operation_id: other.operation_id,
        })
        .unwrap();
    assert!(matches!(
        service.synchronize_remote(other, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::OperationMismatch,
            ..
        }))
    ));
}
fn child(repo: &git2::Repository, parent: git2::Oid, bytes: &[u8]) -> git2::Oid {
    let parent = repo.find_commit(parent).unwrap();
    let mut builder = repo.treebuilder(Some(&parent.tree().unwrap())).unwrap();
    builder
        .insert("fixture.txt", repo.blob(bytes).unwrap(), 0o100644)
        .unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(None, &sig, &sig, "fixture", &tree, &[&parent])
        .unwrap()
}
#[test]
fn actual_commit_graph_rejections_preserve_ref_index_and_worktree_bytes() {
    let (root, _data, _service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let local = repo.head().unwrap().target().unwrap();
    let remote = child(&repo, local, b"remote\n");
    let divergent = child(&repo, local, b"divergent\n");
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
    };
    let index = fs::read(repo.path().join("index")).unwrap();
    let bytes = fs::read(root.path().join("fixture.txt")).unwrap();
    let cases = [
        (
            None,
            Some(remote),
            RemotePublicationEvidence::NeverPublished,
        ),
        (
            Some(local),
            None,
            RemotePublicationEvidence::ObservedPublished,
        ),
        (Some(local), None, RemotePublicationEvidence::HistoryUnknown),
        (
            Some(divergent),
            Some(remote),
            RemotePublicationEvidence::NeverPublished,
        ),
    ];
    for (primary, context, publication) in cases {
        let result = graph_plan(&repo, &target, local, primary, context, publication);
        assert!(result.is_err());
        assert_eq!(repo.head().unwrap().target(), Some(local));
        assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
        assert_eq!(fs::read(root.path().join("fixture.txt")).unwrap(), bytes);
    }
    assert!(matches!(
        graph_plan(
            &repo,
            &SynchronizationTarget::Primary,
            remote,
            Some(divergent),
            None,
            RemotePublicationEvidence::NeverPublished
        ),
        Err(SynchronizationError::MergeRequired { .. })
    ));
}
#[test]
fn one_expected_old_safe_fast_forward_updates_worktree_and_ref() {
    let (root, _data, _service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let old = repo.head().unwrap().target().unwrap();
    let new = child(&repo, old, b"advanced\n");
    fast_forward(&repo, "refs/heads/main", old, new).unwrap();
    assert_eq!(repo.head().unwrap().target(), Some(new));
    assert_eq!(
        fs::read(root.path().join("fixture.txt")).unwrap(),
        b"advanced\n"
    );
    local_target(root.path(), "main", &SynchronizationTarget::Primary).unwrap();
    assert!(matches!(
        fast_forward(&repo, "refs/heads/main", old, new),
        Err(SynchronizationError::ExternalChange)
    ));
}
#[test]
fn checkout_failure_preserves_old_ref_and_user_content_without_rollback() {
    let (root, _data, _service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let old = repo.head().unwrap().target().unwrap();
    let new = child(&repo, old, b"advanced\n");
    fs::write(root.path().join("fixture.txt"), b"external writer").unwrap();
    assert!(matches!(
        fast_forward(&repo, "refs/heads/main", old, new),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(repo.head().unwrap().target(), Some(old));
    assert_eq!(
        fs::read(root.path().join("fixture.txt")).unwrap(),
        b"external writer"
    );
}
#[test]
fn missing_primary_is_typed_and_preserves_index_and_worktree() {
    let (root, _data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let index = fs::read(repo.path().join("index")).unwrap();
    repo.find_reference("refs/heads/main")
        .unwrap()
        .delete()
        .unwrap();
    assert!(matches!(
        service.synchronize_remote(request(root.path()), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::PrimaryMissing)
    ));
    assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(
        fs::read(root.path().join("fixture.txt")).unwrap(),
        b"original\n"
    );
}
#[test]
fn materialized_context_uses_existing_authoring_identity_and_checks_its_own_worktree() {
    let (root, _data, service) = fixture();
    let item_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    let target = AuthoringTarget {
        root: root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id,
        intent: ContextIntent::Create,
        operation_id: OperationId::new(),
    };
    let saved = service.save_ticket(SaveTicketRequest {
        target: target.clone(),
        draft: TicketDraft {
            title: "Fixture".into(),
            body: "fixture".into(),
            ticket_type: "task".into(),
            status: "open".into(),
            project: None,
            team: None,
        },
        expected_path: ExpectedPathObservation::Missing,
    });
    let context = match saved.unwrap() {
        SaveOutcome::Saved { context, .. } | SaveOutcome::IndexPending { context, .. } => context,
        _ => panic!("missing context"),
    };
    let linked = git2::Repository::open(&context.worktree).unwrap();
    let mut index = linked.index().unwrap();
    index
        .read_tree(&linked.head().unwrap().peel_to_tree().unwrap())
        .unwrap();
    index.write().unwrap();
    let mut req = request(root.path());
    req.target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: target.item_id,
    };
    // Context synchronization does not require the unrelated primary to be clean.
    fs::write(root.path().join("fixture.txt"), b"primary dirty").unwrap();
    assert!(matches!(
        service
            .synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        SynchronizationResult::Complete(SynchronizationOutcome::PublishPending { .. })
    ));
    fs::write(context.worktree.join("fixture.txt"), b"context dirty").unwrap();
    req.operation_id = OperationId::new();
    assert!(matches!(
        service.synchronize_remote(req, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::WorktreeNotClean { .. })
    ));
    assert_eq!(
        fs::read(context.worktree.join("fixture.txt")).unwrap(),
        b"context dirty"
    );
}
#[test]
fn ref_lock_rejects_before_checkout_and_post_checkout_mismatch_is_not_rolled_back() {
    let (root, _data, _service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let old = repo.head().unwrap().target().unwrap();
    let new = child(&repo, old, b"advanced\n");
    let mut lock = repo.transaction().unwrap();
    lock.lock_ref("refs/heads/main").unwrap();
    assert!(matches!(
        fast_forward(&repo, "refs/heads/main", old, new),
        Err(SynchronizationError::ExternalChange)
    ));
    assert_eq!(
        fs::read(root.path().join("fixture.txt")).unwrap(),
        b"original\n"
    );
    drop(lock);
    let path = root.path().to_owned();
    let hook = super::super::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeLocalMutation {
            git2::Repository::open(&path)
                .unwrap()
                .set_head_detached(old)
                .unwrap();
        }
    });
    assert!(matches!(
        fast_forward(&repo, "refs/heads/main", old, new),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(hook);
    assert_eq!(repo.refname_to_id("refs/heads/main").unwrap(), old);
    assert_eq!(
        fs::read(root.path().join("fixture.txt")).unwrap(),
        b"advanced\n"
    );
}
#[test]
fn conflicted_preflight_preserves_the_real_merge_index_and_content() {
    let (root, _data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let base = repo.head().unwrap().target().unwrap();
    let ours = child(&repo, base, b"ours\n");
    let theirs = child(&repo, base, b"theirs\n");
    fast_forward(&repo, "refs/heads/main", base, ours).unwrap();
    repo.merge(&[&repo.find_annotated_commit(theirs).unwrap()], None, None)
        .unwrap();
    assert!(repo.index().unwrap().has_conflicts());
    let index = fs::read(repo.path().join("index")).unwrap();
    let bytes = fs::read(root.path().join("fixture.txt")).unwrap();
    assert!(matches!(
        service.synchronize_remote(request(root.path()), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::WorktreeConflicted { .. })
    ));
    assert_eq!(repo.head().unwrap().target(), Some(ours));
    assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(fs::read(root.path().join("fixture.txt")).unwrap(), bytes);
}
#[test]
fn local_binding_sql_failure_rolls_back_and_does_not_claim_index_pending() {
    let (root, data, service) = fixture();
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("CREATE TRIGGER fail_bind BEFORE INSERT ON operation_records WHEN NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END").unwrap();
    let req = request(root.path());
    assert!(
        service
            .synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt))
            .is_err()
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
            [req.operation_id.to_string()],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER fail_bind").unwrap();
    let result = service
        .synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt))
        .unwrap();
    let reopened = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        reopened
            .synchronize_remote(req, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        result
    );
}

#[test]
fn local_synchronization_binding_rejects_remote_collision_after_none_inspection_in_every_phase() {
    // Structural phase fixtures exercise the insertion policy, not effect proof.
    // Each fresh database prevents unrelated active rows from masking the guard.
    for phase in [
        "completed",
        "interrupted",
        "cancelled",
        "failed",
        "reserved",
        "advertising",
        "persisting",
        "fetch_prepared",
        "fetch_observed",
        "local_prepared",
        "local_fast_forwarded",
        "push_prepared",
        "push_returned",
        "push_verified",
        "reconciling",
    ] {
        let (root, data, service) = fixture();
        let req = request(root.path());
        assert!(
            state::with_transaction(&service, root.path(), |tx, id| state::read_operation(
                tx,
                id,
                req.operation_id
            ))
            .unwrap()
            .is_none()
        );
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        // Simulate the remote insertion AFTER service inspection returned None,
        // before local binding. The binder must inspect again in its transaction.
        state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&plan), false)?;
            state::insert_operation(
                tx,
                id,
                req.operation_id,
                &RemoteOperationTarget::for_primary_synchronization(&plan),
                RemoteOperationPriority::Manual,
                0,
            )?;
            tx.execute(
                "UPDATE remote_operation_records SET phase=?1 WHERE operation_ulid=?2",
                rusqlite::params![phase, req.operation_id.to_string()],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
        let repo = git2::Repository::open(root.path()).unwrap();
        let oid = repo.head().unwrap().target();
        let index = fs::read(repo.path().join("index")).unwrap();
        let bytes = fs::read(root.path().join("fixture.txt")).unwrap();
        assert!(
            matches!(
                service.bind_local_synchronization(&req, "main"),
                Err(SynchronizationError::RecoveryRequired)
            ),
            "phase {phase}"
        );
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [req.operation_id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(repo.head().unwrap().target(), oid);
        assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
        assert_eq!(fs::read(root.path().join("fixture.txt")).unwrap(), bytes);
    }
}

#[test]
fn existing_remote_id_rejects_other_root_before_invalid_live_configuration() {
    let (root, _data, service) = fixture();
    let (other, _other_data, _other_service) = fixture();
    service
        .enable(EnableRepositoryRequest {
            root: other.path().into(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    let req = request(root.path());
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), false)?;
        state::insert_operation(
            tx,
            id,
            req.operation_id,
            &RemoteOperationTarget::for_primary_synchronization(&plan),
            RemoteOperationPriority::Manual,
            0,
        )?;
        Ok(())
    })
    .unwrap();
    fs::write(
        other.path().join(".manyhands/config.toml"),
        b"invalid configuration",
    )
    .unwrap();
    let mut wrong = req;
    wrong.root = other.path().into();
    wrong.restart = true;
    assert!(matches!(
        service.synchronize_remote(wrong, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::OperationMismatch,
            ..
        }))
    ));
}

type LocalBindingHook = (OperationId, Box<dyn FnOnce()>);
thread_local! {
    static LOCAL_BINDING_HOOK: std::cell::RefCell<Option<LocalBindingHook>> = const { std::cell::RefCell::new(None) };
}
struct LocalBindingHookGuard;
impl Drop for LocalBindingHookGuard {
    fn drop(&mut self) {
        LOCAL_BINDING_HOOK.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
impl RepositoryService {
    // Runs after None inspection, before Git/cache leases. Removed before invocation
    // so a competing service call can deterministically finish its real binding.
    pub(crate) fn set_local_binding_hook_for_testing(
        &self,
        id: OperationId,
        hook: impl FnOnce() + 'static,
    ) -> impl Drop {
        LOCAL_BINDING_HOOK
            .with(|slot| assert!(slot.borrow_mut().replace((id, Box::new(hook))).is_none()));
        LocalBindingHookGuard
    }
}
pub(super) fn run_local_binding_hook(id: OperationId) {
    let hook = LOCAL_BINDING_HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|(expected, _)| *expected == id) {
            slot.take()
        } else {
            None
        }
    });
    if let Some((_, hook)) = hook {
        hook();
    }
}
fn materialize_local_context(service: &RepositoryService, root: &Path) -> SynchronizationTarget {
    let item_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    let result = service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: root.into(),
                kind: AuthoringKind::Ticket,
                item_id,
                intent: ContextIntent::Create,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "Fixture".into(),
                body: "fixture".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
            },
            expected_path: ExpectedPathObservation::Missing,
        })
        .unwrap();
    let context = match result {
        SaveOutcome::Saved { context, .. } | SaveOutcome::IndexPending { context, .. } => context,
        _ => panic!("fixture context"),
    };
    let linked = git2::Repository::open(&context.worktree).unwrap();
    let mut index = linked.index().unwrap();
    index
        .read_tree(&linked.head().unwrap().peel_to_tree().unwrap())
        .unwrap();
    index.write().unwrap();
    SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: context.item_id,
    }
}
fn local_binding_image(root: &Path) -> [u8; 32] {
    let repo = git2::Repository::open(root).unwrap();
    let mut hash = blake3::Hasher::new();
    let mut refs = repo
        .references_glob("refs/heads/*")
        .unwrap()
        .map(|r| {
            let r = r.unwrap();
            format!("{} {:?}", r.name().unwrap(), r.target())
        })
        .collect::<Vec<_>>();
    refs.sort();
    for r in refs {
        hash.update(r.as_bytes());
    }
    let mut worktrees = vec![root.to_owned()];
    for name in repo.worktrees().unwrap().iter().flatten() {
        worktrees.push(repo.find_worktree(name).unwrap().path().to_owned());
    }
    for path in worktrees {
        let linked = git2::Repository::open(&path).unwrap();
        for name in ["HEAD", "index"] {
            hash.update(&fs::read(linked.path().join(name)).unwrap());
        }
        for entry in linked.index().unwrap().iter() {
            hash.update(&entry.path);
            hash.update(&fs::read(path.join(std::str::from_utf8(&entry.path).unwrap())).unwrap());
        }
    }
    *hash.finalize().as_bytes()
}
#[test]
fn local_binding_after_none_rejects_actual_other_target_service_authority() {
    let (root, data, service) = fixture();
    let target = materialize_local_context(&service, root.path());
    let service = std::sync::Arc::new(service);
    let primary = request(root.path());
    let mut context = primary.clone();
    context.target = target;
    let original = std::sync::Arc::new(std::sync::Mutex::new(None));
    let first = original.clone();
    let competing = service.clone();
    let first_request = primary.clone();
    let path = root.path().to_owned();
    let image = std::sync::Arc::new(std::sync::Mutex::new(None));
    let snapshot = image.clone();
    let scans = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = scans.clone();
    let hook = service.set_local_binding_hook_for_testing(context.operation_id, move || {
        *first.lock().unwrap() = Some(
            competing
                .synchronize_remote(first_request, &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        *snapshot.lock().unwrap() = Some(local_binding_image(&path));
        competing.set_observation_hook_for_testing(move || {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
    });
    assert!(matches!(
        service.synchronize_remote(context.clone(), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(hook);
    assert_eq!(
        local_binding_image(root.path()),
        image.lock().unwrap().unwrap()
    );
    assert_eq!(scans.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        service
            .synchronize_remote(primary, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        original.lock().unwrap().clone().unwrap()
    );
    assert!(matches!(
        service.synchronize_remote(context, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::OperationMismatch,
            ..
        }))
    ));
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE target LIKE 'synchronization-local-v1/%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
#[test]
fn local_binding_after_none_rejects_competing_oid_then_replays_frozen_first_outcome() {
    let (root, _data, service) = fixture();
    let service = std::sync::Arc::new(service);
    let req = request(root.path());
    let first_request = req.clone();
    let competing = service.clone();
    let path = root.path().to_owned();
    let original = std::sync::Arc::new(std::sync::Mutex::new(None));
    let first = original.clone();
    let image = std::sync::Arc::new(std::sync::Mutex::new(None));
    let snapshot = image.clone();
    let scans = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = scans.clone();
    let hook = service.set_local_binding_hook_for_testing(req.operation_id, move || {
        *first.lock().unwrap() = Some(
            competing
                .synchronize_remote(first_request, &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        let repo = git2::Repository::open(&path).unwrap();
        fs::write(path.join("fixture.txt"), b"later committed fixture").unwrap();
        commit_all(&repo);
        // Baseline AFTER deliberate fixture commit, before the losing binder.
        *snapshot.lock().unwrap() = Some(local_binding_image(&path));
        competing.set_observation_hook_for_testing(move || {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
    });
    assert!(matches!(
        service.synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(hook);
    assert_eq!(
        local_binding_image(root.path()),
        image.lock().unwrap().unwrap()
    );
    assert_eq!(scans.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        service
            .synchronize_remote(req, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        original.lock().unwrap().clone().unwrap()
    );
}

#[test]
fn local_binding_after_none_rejects_actual_plain_refresh_before_discovery() {
    let (root, data, service) = fixture();
    let service = std::sync::Arc::new(service);
    let req = request(root.path());
    let first_request = req.clone();
    let competing = service.clone();
    let scans = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = scans.clone();
    let hook = service.set_local_binding_hook_for_testing(req.operation_id, move || {
        // Ordinary plain refresh is still legitimate; only tagged adoption fails.
        assert!(matches!(
            competing
                .refresh_repository(RefreshRepositoryRequest {
                    root: first_request.root,
                    operation_id: first_request.operation_id
                })
                .unwrap(),
            RefreshOutcome::Refreshed { .. }
        ));
        competing.set_observation_hook_for_testing(move || {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
    });
    let repo = git2::Repository::open(root.path()).unwrap();
    let oid = repo.head().unwrap().target();
    let index = fs::read(repo.path().join("index")).unwrap();
    assert!(matches!(
        service.synchronize_remote(req.clone(), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(hook);
    assert_eq!(scans.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(repo.head().unwrap().target(), oid);
    assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT target FROM operation_records WHERE operation_ulid=?1",
            [req.operation_id.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        ""
    );
    assert!(matches!(
        service.synchronize_remote(req, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::OperationMismatch,
            ..
        }))
    ));
}

#[test]
fn local_binding_transaction_rejects_incompatible_existing_and_pending_rows() {
    // Structural row fixtures prove binding policy only, not effect/discovery.
    for status in ["completed", "created", "observed", "error", "indexing"] {
        for collision in [
            "target",
            "oid",
            "malformed",
            "plain",
            "action",
            "root",
            "pending_id",
            "pending_alias",
        ] {
            if status == "completed" && matches!(collision, "pending_id" | "pending_alias") {
                continue;
            }
            let (root, data, service) = fixture();
            let req = request(root.path());
            assert!(
                service
                    .local_synchronization_replay(&req)
                    .unwrap()
                    .is_none()
            );
            let repo = git2::Repository::open(root.path()).unwrap();
            let oid = repo.head().unwrap().target().unwrap();
            let matcher = format!("{}/{oid}", local_refresh_identity(&req.target));
            let foreign = tempfile::tempdir().unwrap();
            let stored_root = if collision == "root" {
                foreign.path()
            } else {
                root.path()
            };
            let stored_id = match collision {
                "pending_id" => Some(OperationId::new().to_string()),
                "pending_alias" => None,
                _ => Some(req.operation_id.to_string()),
            };
            let stored_action = if collision == "action" {
                "enable"
            } else {
                "refresh"
            };
            let stored_target = match collision {
                "target" => {
                    format!("synchronization-local-v1/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV/{oid}")
                }
                "oid" => format!("synchronization-local-v1/primary/{}", git2::Oid::zero()),
                "malformed" => "synchronization-local-v1/primary/not-an-oid".into(),
                "plain" => String::new(),
                _ => matcher.clone(),
            };
            let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            db.execute("INSERT INTO operation_records(root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,?3,?4,?5,0)",rusqlite::params![stored_root.to_str().unwrap(),stored_id,stored_action,stored_target,status]).unwrap();
            let count: i64 = db
                .query_row("SELECT count(*) FROM operation_records", [], |r| r.get(0))
                .unwrap();
            let index = fs::read(repo.path().join("index")).unwrap();
            let bytes = fs::read(root.path().join("fixture.txt")).unwrap();
            assert!(
                matches!(
                    service.bind_local_synchronization(&req, "main"),
                    Err(SynchronizationError::RecoveryRequired)
                ),
                "{collision}/{status}"
            );
            assert_eq!(
                db.query_row("SELECT count(*) FROM operation_records", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                count
            );
            assert_eq!(db.query_row("SELECT root_path,action,target,state FROM operation_records ORDER BY id DESC LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).unwrap(),(stored_root.to_str().unwrap().into(),stored_action.into(),stored_target,status.into()));
            assert_eq!(repo.head().unwrap().target(), Some(oid));
            assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
            assert_eq!(fs::read(root.path().join("fixture.txt")).unwrap(), bytes);
        }
    }
}
#[test]
fn local_binding_transaction_accepts_only_identical_complete_tag_without_rewriting() {
    for status in ["completed", "created", "observed", "error", "indexing"] {
        let (root, data, service) = fixture();
        let req = request(root.path());
        let oid = git2::Repository::open(root.path())
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap();
        let matcher = format!("{}/{oid}", local_refresh_identity(&req.target));
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute("INSERT INTO operation_records(root_path,operation_ulid,action,target,state,observed_at) VALUES(?1,?2,'refresh',?3,?4,0)",rusqlite::params![root.path().to_str().unwrap(),req.operation_id.to_string(),matcher,status]).unwrap();
        let count: i64 = db
            .query_row("SELECT count(*) FROM operation_records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            service.bind_local_synchronization(&req, "main").unwrap(),
            oid
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM operation_records", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            count
        );
        assert_eq!(
            db.query_row(
                "SELECT target,state FROM operation_records WHERE operation_ulid=?1",
                [req.operation_id.to_string()],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            )
            .unwrap(),
            (matcher, status.into())
        );
    }
}

#[test]
fn prospective_resolution_context_rejects_duplicate_ids_and_invalid_comment_threads() {
    let (root, _data, _service) = fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let observation = merge::ConflictObservation::for_testing([42; 32]);
    let duplicate = b"---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Title\"\n---\n\nBody\n";
    let first = merge::ConflictPathToken {
        observation: observation.clone(),
        ordinal: 0,
        path: b"docs/one.md".to_vec(),
        base: None,
        base_mode: None,
        local: None,
        local_mode: None,
        incoming: None,
        incoming_mode: None,
    };
    let second = merge::ConflictPathToken {
        path: b"docs/two.md".to_vec(),
        ordinal: 1,
        ..first.clone()
    };
    let first_bytes = merge::RedactedConflictBytes::from_bytes(duplicate.to_vec());
    let second_bytes = merge::RedactedConflictBytes::from_bytes(duplicate.to_vec());
    let replacements = std::collections::BTreeMap::from([
        (0, (&first, &first_bytes)),
        (1, (&second, &second_bytes)),
    ]);
    assert!(!RepositoryService::validates_prospective_context(&repository, &replacements).unwrap());

    let comment = b"---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\nitem_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\nparent_id: \"01CRZ3NDEKTSV4RRFFQ69G5FAV\"\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n\nReply\n";
    let comment_token = merge::ConflictPathToken {
        observation,
        ordinal: 0,
        path: b".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01BX5ZZKBKACTAV9WEVGEMMVRZ.md"
            .to_vec(),
        base: None,
        base_mode: None,
        local: None,
        local_mode: None,
        incoming: None,
        incoming_mode: None,
    };
    let comment_bytes = merge::RedactedConflictBytes::from_bytes(comment.to_vec());
    let replacements = std::collections::BTreeMap::from([(0, (&comment_token, &comment_bytes))]);
    assert!(!RepositoryService::validates_prospective_context(&repository, &replacements).unwrap());
}

fn resolution_fixture(
    files: &[(&str, &str, &str, &str)],
) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    OperationId,
    git2::Oid,
    git2::Oid,
) {
    let (root, data, service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    fs::write(
        root.path().join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
    )
    .unwrap();
    for (path, base, _, _) in files {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, base).unwrap();
    }
    let base = commit_all(&repo);
    for (path, _, local, _) in files {
        fs::write(root.path().join(path), local).unwrap();
    }
    let local = commit_all(&repo);
    repo.set_head_detached(base).unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    for (path, _, _, incoming) in files {
        fs::write(root.path().join(path), incoming).unwrap();
    }
    let incoming = commit_all(&repo);
    repo.set_head("refs/heads/main").unwrap();
    repo.reference("refs/heads/main", local, true, "fixture")
        .unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), false)
    })
    .unwrap();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan);
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("reservation"),
    };
    let evidence = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(local),
        tracking_oid: Some(incoming),
        primary_tracking_oid: Some(incoming),
        ..state::SynchronizationEvidence::default()
    };
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &evidence,
        )
        .unwrap();
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    let observation =
        RemoteRefObservation::from_advertisement(&plan, "refs/heads/main", incoming, None).unwrap();
    commit_observation_batch(&service, root.path(), &owner, &plan, &[observation], 1).unwrap();
    let tree = repo.find_commit(local).unwrap().tree().unwrap();
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: merge::IntegrationStage::Primary,
                local_oid: local,
                incoming_oid: incoming,
                baseline_tree_oid: tree.id(),
                baseline_index_digest: index_digest(tree.id()),
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 0, None)
        .unwrap();
    repo.merge(
        &[&repo.find_annotated_commit(incoming).unwrap()],
        None,
        None,
    )
    .unwrap();
    let fingerprint = conflict_digest(&mut git2::Repository::open(root.path()).unwrap()).unwrap();
    service
        .release_synchronization_conflict(root.path(), &owner, 0, fingerprint)
        .unwrap();
    (root, data, service, owner.operation_id(), local, incoming)
}

fn context_primary_conflict_fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    OperationId,
    std::path::PathBuf,
    git2::Oid,
    git2::Oid,
    crate::canonical::ItemId,
) {
    let (root, data, service) = fixture();
    let ticket_id: crate::canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    let saved = service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: root.path().into(),
                kind: AuthoringKind::Ticket,
                item_id: ticket_id.clone(),
                intent: ContextIntent::Create,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "context".into(),
                body: "context".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
            },
            expected_path: ExpectedPathObservation::Missing,
        })
        .unwrap();
    let context = match saved {
        SaveOutcome::Saved { context, .. } | SaveOutcome::IndexPending { context, .. } => context,
        _ => panic!("context"),
    };
    fs::write(
        root.path().join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
    )
    .unwrap();
    let repository = git2::Repository::open(&context.worktree).unwrap();
    let base_body = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Primary stage\"\n---\n\nbase\n";
    fs::create_dir_all(context.worktree.join("docs")).unwrap();
    fs::write(context.worktree.join("docs/primary.md"), base_body).unwrap();
    let base = commit_all(&repository);
    fs::write(
        context.worktree.join("docs/primary.md"),
        base_body.replace("base", "local"),
    )
    .unwrap();
    let local = commit_all(&repository);
    repository.set_head_detached(base).unwrap();
    repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    fs::write(
        context.worktree.join("docs/primary.md"),
        base_body.replace("base", "incoming"),
    )
    .unwrap();
    let incoming = commit_all(&repository);
    let branch_ref = format!("refs/heads/{}", context.branch);
    repository.set_head(&branch_ref).unwrap();
    repository
        .reference(&branch_ref, local, true, "fixture")
        .unwrap();
    repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), false)
    })
    .unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        ticket_id.clone(),
    )
    .unwrap();
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("reservation"),
    };
    let evidence = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(local),
        tracking_oid: Some(incoming),
        primary_tracking_oid: Some(incoming),
        ..state::SynchronizationEvidence::default()
    };
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &evidence,
        )
        .unwrap();
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    let observation =
        RemoteRefObservation::from_advertisement(&plan, "refs/heads/main", incoming, None).unwrap();
    commit_observation_batch(&service, root.path(), &owner, &plan, &[observation], 1).unwrap();
    let local_tree = repository.find_commit(local).unwrap().tree().unwrap();
    // Context stage ordinal zero has already completed before the primary
    // stage encounters this real merge conflict.
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: merge::IntegrationStage::Context,
                local_oid: local,
                incoming_oid: local,
                baseline_tree_oid: local_tree.id(),
                baseline_index_digest: index_digest(local_tree.id()),
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 0, None)
        .unwrap();
    service
        .observe_synchronization_integration_effect(root.path(), &owner, 0, local, local_tree.id())
        .unwrap();
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 1,
                stage: merge::IntegrationStage::Primary,
                local_oid: local,
                incoming_oid: incoming,
                baseline_tree_oid: local_tree.id(),
                baseline_index_digest: index_digest(local_tree.id()),
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 1, None)
        .unwrap();
    repository
        .merge(
            &[&repository.find_annotated_commit(incoming).unwrap()],
            None,
            None,
        )
        .unwrap();
    let fingerprint =
        conflict_digest(&mut git2::Repository::open(&context.worktree).unwrap()).unwrap();
    service
        .release_synchronization_conflict(root.path(), &owner, 1, fingerprint)
        .unwrap();
    (
        root,
        data,
        service,
        owner.operation_id(),
        context.worktree,
        local,
        incoming,
        ticket_id,
    )
}

fn resolve_fixture(
    root: &Path,
    service: &RepositoryService,
    operation: OperationId,
    results: &[&str],
) -> ResolveSynchronizationOutcome {
    let inspection = service
        .inspect_synchronization_recovery(root, operation)
        .unwrap();
    let resolutions = inspection
        .paths
        .iter()
        .map(|path| {
            let path_text = std::str::from_utf8(&path.token.path).unwrap();
            let result = if path_text.starts_with("docs/") {
                results[0]
            } else if path_text.contains("/tickets/") {
                results[1]
            } else {
                results[2]
            };
            (
                path.token.clone(),
                RedactedConflictBytes::from_bytes(result.as_bytes().to_vec()),
            )
        })
        .collect();
    let request = ResolveSynchronizationRequest::new(
        root.to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        resolutions,
        None,
    );
    service.resolve_synchronization(request).unwrap()
}

#[test]
fn public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally() {
    let document_base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let document_local = document_base.replace("base", "local");
    let document_remote = document_base.replace("base", "remote");
    let ticket_base = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Ticket\"\ntype: \"cycle\"\nstatus: \"open\"\n---\n\nbase\n";
    let ticket_local = ticket_base.replace("base", "local");
    let ticket_remote = ticket_base.replace("base", "remote");
    let comment_base = "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"01CRZ3NDEKTSV4RRFFQ69G5FAV\"\nitem_id: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n\nbase\n";
    let comment_local = comment_base.replace("base", "local");
    let comment_remote = comment_base.replace("base", "remote");
    let files = [
        (
            "docs/document.md",
            document_base,
            document_local.as_str(),
            document_remote.as_str(),
        ),
        (
            ".manyhands/tickets/01BX5ZZKBKACTAV9WEVGEMMVRZ/ticket.md",
            ticket_base,
            ticket_local.as_str(),
            ticket_remote.as_str(),
        ),
        (
            ".manyhands/comments/01BX5ZZKBKACTAV9WEVGEMMVRZ/01CRZ3NDEKTSV4RRFFQ69G5FAV.md",
            comment_base,
            comment_local.as_str(),
            comment_remote.as_str(),
        ),
    ];
    let (root, _data, service, operation, local, incoming) = resolution_fixture(&files);
    let result_document = document_base.replace("base", "resolved document");
    let result_ticket = ticket_base.replace("base", "resolved ticket");
    let result_comment = comment_base.replace("base", "resolved comment");
    let outcome = resolve_fixture(
        root.path(),
        &service,
        operation,
        &[&result_document, &result_ticket, &result_comment],
    );
    let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } = outcome else {
        let evidence = state::with_transaction(&service, root.path(), |tx, id| {
            let record = state::read_operation(tx, id, operation)?.unwrap();
            let step = state::integration_step(tx, record.id, 0)?;
            let paths: Vec<(i64, bool)> = tx
                .prepare("SELECT ordinal,applied FROM remote_resolution_paths")
                .unwrap()
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            Ok((step, paths))
        })
        .unwrap();
        panic!("local completion: {outcome:?}; evidence={evidence:?}")
    };
    let mut repository = git2::Repository::open(root.path()).unwrap();
    let commit = repository.find_commit(commit_oid).unwrap();
    assert_eq!(
        [commit.parent_id(0).unwrap(), commit.parent_id(1).unwrap()],
        [local, incoming]
    );
    drop(commit);
    assert_eq!(repository.head().unwrap().target(), Some(commit_oid));
    assert!(!repository.index().unwrap().has_conflicts());
    assert!(repository.mergehead_foreach(|_| true).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("docs/document.md")).unwrap(),
        result_document
    );
    assert_eq!(
        fs::read_to_string(
            root.path()
                .join(".manyhands/tickets/01BX5ZZKBKACTAV9WEVGEMMVRZ/ticket.md")
        )
        .unwrap(),
        result_ticket
    );
    assert_eq!(
        fs::read_to_string(
            root.path().join(
                ".manyhands/comments/01BX5ZZKBKACTAV9WEVGEMMVRZ/01CRZ3NDEKTSV4RRFFQ69G5FAV.md"
            )
        )
        .unwrap(),
        result_comment
    );
}

#[test]
fn mixed_canonical_and_code_conflict_is_whole_merge_external_only_without_writes() {
    let document_base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local_document = document_base.replace("base", "local");
    let remote_document = document_base.replace("base", "remote");
    let (root, _data, service, operation, local, _incoming) = resolution_fixture(&[
        (
            "docs/document.md",
            document_base,
            local_document.as_str(),
            remote_document.as_str(),
        ),
        ("src/foreign.rs", "base\n", "local\n", "remote\n"),
    ]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    assert!(
        inspection
            .paths
            .iter()
            .all(|path| path.eligibility == merge::ConflictEligibility::ExternalResolutionRequired)
    );
    let before_head = git2::Repository::open(root.path())
        .unwrap()
        .head()
        .unwrap()
        .target();
    let before_index = fs::read(
        git2::Repository::open(root.path())
            .unwrap()
            .path()
            .join("index"),
    )
    .unwrap();
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        Vec::new(),
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::StaleObservation
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(repository.head().unwrap().target(), before_head);
    assert_eq!(
        fs::read(repository.path().join("index")).unwrap(),
        before_index
    );
    assert_eq!(repository.head().unwrap().target(), Some(local));
    assert!(repository.index().unwrap().has_conflicts());
}

#[test]
fn resolution_path_write_failure_reacquires_identical_attempt_and_rejects_altered_input() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let resolved = base.replace("base", "resolved");
    let altered = base.replace("base", "altered");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request = |body: &str| {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(body.as_bytes().to_vec()),
            )],
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterPathWrite);
    assert!(
        matches!(service.resolve_synchronization(request(&resolved)), Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure)
    );
    *service.failure_point.lock().unwrap() = None;
    assert!(
        matches!(service.resolve_synchronization(request(&altered)), Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        fs::read_to_string(root.path().join("docs/document.md")).unwrap(),
        resolved
    );
    assert!(matches!(
        service.resolve_synchronization(request(&resolved)).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

#[test]
fn resumed_applied_path_external_edit_is_rejected_without_overwrite() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let resolved = base.replace("base", "resolved");
    let external = base.replace("base", "external edit after owned write");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request = || {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(resolved.as_bytes().to_vec()),
            )],
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterPathWrite);
    assert!(matches!(
        service.resolve_synchronization(request()),
        Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure
    ));
    *service.failure_point.lock().unwrap() = None;
    let repository = git2::Repository::open(root.path()).unwrap();
    let image = (
        repository.head().unwrap().target(),
        fs::read(repository.path().join("index")).unwrap(),
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT attempt.phase,path.applied FROM remote_resolution_attempts attempt JOIN remote_resolution_paths path ON path.attempt_id=attempt.id",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
            )
            .unwrap(),
    );
    fs::write(root.path().join("docs/document.md"), &external).unwrap();
    assert!(matches!(
        service.resolve_synchronization(request()),
        Err(SynchronizationError::ExternalChange)
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(repository.head().unwrap().target(), image.0);
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), image.1);
    assert_eq!(
        fs::read_to_string(root.path().join("docs/document.md")).unwrap(),
        external
    );
    assert_eq!(
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT attempt.phase,path.applied FROM remote_resolution_attempts attempt JOIN remote_resolution_paths path ON path.attempt_id=attempt.id",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
            )
            .unwrap(),
        image.2
    );
}

#[test]
fn resolution_candidate_failure_reuses_the_same_candidate_on_retry() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let resolved = base.replace("base", "resolved");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request = || {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(resolved.as_bytes().to_vec()),
            )],
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterCandidatePrepared);
    assert!(
        matches!(service.resolve_synchronization(request()), Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure)
    );
    let candidate: String = state::with_transaction(&service, root.path(), |tx, _| {
        tx.query_row(
            "SELECT candidate_oid FROM remote_resolution_attempts",
            [],
            |row| row.get(0),
        )
        .map_err(|_| state::recovery_required())
    })
    .unwrap();
    *service.failure_point.lock().unwrap() = None;
    #[cfg(target_os = "linux")]
    assert!(
        git2::Repository::open(root.path())
            .unwrap()
            .path()
            .join("index.lock")
            .exists(),
        "failure retains the journaled sentinel without Drop cleanup"
    );
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
        restarted.resolve_synchronization(request()).unwrap()
    else {
        panic!("completion")
    };
    assert_eq!(commit_oid.to_string(), candidate);
    #[cfg(target_os = "linux")]
    assert!(
        !git2::Repository::open(root.path())
            .unwrap()
            .path()
            .join("index.lock")
            .exists()
    );
}

#[test]
fn resolution_ref_transition_failure_reconciles_candidate_without_second_ref_move() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let resolved = base.replace("base", "resolved");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request = || {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(resolved.as_bytes().to_vec()),
            )],
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(
        matches!(service.resolve_synchronization(request()), Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure)
    );
    #[cfg(target_os = "linux")]
    assert!(
        git2::Repository::open(root.path())
            .unwrap()
            .path()
            .join("index.lock")
            .exists(),
        "stable anchored sentinel survives the injected ref-transition failure"
    );
    let candidate = git2::Repository::open(root.path())
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap();
    *service.failure_point.lock().unwrap() = None;
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
        restarted.resolve_synchronization(request()).unwrap()
    else {
        panic!("completion")
    };
    assert_eq!(commit_oid, candidate);
    let mut repository = git2::Repository::open(root.path()).unwrap();
    assert!(repository.mergehead_foreach(|_| true).is_err());
    #[cfg(target_os = "linux")]
    assert!(
        !repository.path().join("index.lock").exists(),
        "durable retry retires only the recognized recovery sentinel"
    );
}

#[test]
fn resolution_checkpoint_and_metadata_retirement_failures_reconcile_without_duplicate_commit() {
    for hook in [
        FailurePoint::ResolutionAfterCheckpointObservation,
        FailurePoint::ResolutionBeforeMetadataRetirement,
        FailurePoint::ResolutionAfterIndexLockRetirement,
        FailurePoint::ResolutionAfterMetadataCleanup,
    ] {
        let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
        let local = base.replace("base", "local");
        let remote = base.replace("base", "remote");
        let resolved = base.replace("base", "resolved");
        let (root, _data, service, operation, _, _) =
            resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let attempt = OperationId::new();
        let request = || {
            ResolveSynchronizationRequest::new(
                root.path().to_owned(),
                operation,
                attempt,
                inspection.observation.clone(),
                vec![(
                    inspection.paths[0].token.clone(),
                    RedactedConflictBytes::from_bytes(resolved.as_bytes().to_vec()),
                )],
                None,
            )
        };
        *service.failure_point.lock().unwrap() = Some(hook);
        assert!(
            matches!(service.resolve_synchronization(request()), Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure)
        );
        let candidate = git2::Repository::open(root.path())
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap();
        *service.failure_point.lock().unwrap() = None;
        let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
            service.resolve_synchronization(request()).unwrap()
        else {
            panic!("completion")
        };
        assert_eq!(commit_oid, candidate);
        let mut repository = git2::Repository::open(root.path()).unwrap();
        assert!(repository.mergehead_foreach(|_| true).is_err());
    }
}

#[test]
fn pending_context_synchronization_blocks_only_its_public_authoring_context() {
    let (root, _data, service) = fixture();
    let ticket_id: crate::canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    let initial = service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: root.path().into(),
                kind: AuthoringKind::Ticket,
                item_id: ticket_id.clone(),
                intent: ContextIntent::Create,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "before".into(),
                body: "before".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
            },
            expected_path: ExpectedPathObservation::Missing,
        })
        .unwrap();
    let context = match initial {
        SaveOutcome::Saved { context, .. } | SaveOutcome::IndexPending { context, .. } => context,
        _ => panic!("ticket context"),
    };
    let repo = git2::Repository::open(&context.worktree).unwrap();
    let head = repo.head().unwrap().target().unwrap();
    let tree = repo.find_commit(head).unwrap().tree().unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), false)
    })
    .unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        ticket_id.clone(),
    )
    .unwrap();
    let owner = match service
        .reserve_remote_operation(root.path(), OperationId::new(), &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("owner"),
    };
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &state::SynchronizationEvidence {
                expected_oid: Some(head),
                local_oid: Some(head),
                tracking_oid: Some(head),
                primary_tracking_oid: Some(head),
                ..state::SynchronizationEvidence::default()
            },
        )
        .unwrap();
    service
        .prepare_synchronization_integration(
            root.path(),
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: merge::IntegrationStage::Context,
                local_oid: head,
                incoming_oid: head,
                baseline_tree_oid: tree.id(),
                baseline_index_digest: index_digest(tree.id()),
            },
        )
        .unwrap();
    service
        .begin_synchronization_integration_effect(root.path(), &owner, 0, None)
        .unwrap();
    service
        .release_synchronization_conflict(root.path(), &owner, 0, [7; 32])
        .unwrap();
    let ticket_path = context
        .worktree
        .join(format!(".manyhands/tickets/{ticket_id}/ticket.md"));
    let before = (
        local_binding_image(root.path()),
        fs::read(&ticket_path).unwrap(),
        fs::read(repo.path().join("index")).unwrap(),
    );
    let edit = SaveTicketRequest {
        target: AuthoringTarget {
            root: root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ticket_id.clone(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        },
        draft: TicketDraft {
            title: "blocked".into(),
            body: "blocked".into(),
            ticket_type: "task".into(),
            status: "open".into(),
            project: None,
            team: None,
        },
        expected_path: ExpectedPathObservation::from_bytes(&before.1),
    };
    assert!(
        matches!(service.save_ticket(edit), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
    );
    let comment_id: crate::canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap();
    assert!(
        matches!(service.submit_comment(SubmitCommentRequest { target: AuthoringTarget { root: root.path().into(), kind: AuthoringKind::Ticket, item_id: ticket_id.clone(), intent: ContextIntent::Edit, operation_id: OperationId::new() }, comment_id, parent_id: None, body: "blocked".into(), expected_destination: ExpectedPathObservation::Missing }), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        (
            local_binding_image(root.path()),
            fs::read(&ticket_path).unwrap(),
            fs::read(repo.path().join("index")).unwrap()
        ),
        before
    );
    assert!(service.inspect(root.path()).is_ok());
    let document_id: crate::canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAX".parse().unwrap();
    assert!(matches!(
        service
            .save_document(SaveDocumentRequest {
                target: AuthoringTarget {
                    root: root.path().into(),
                    kind: AuthoringKind::Document,
                    item_id: document_id,
                    intent: ContextIntent::Create,
                    operation_id: OperationId::new()
                },
                source_path: None,
                destination_path: "docs/unrelated.md".into(),
                draft: DocumentDraft {
                    title: "unrelated".into(),
                    body: "ok".into()
                },
                expected_source: None,
                expected_destination: ExpectedPathObservation::Missing
            })
            .unwrap(),
        SaveOutcome::Saved { .. } | SaveOutcome::IndexPending { .. }
    ));
}

#[test]
fn resolution_preflight_real_index_detects_index_status_bytes_and_symlink_races() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    for race in ["staged", "untracked", "bytes", "symlink"] {
        let (root, data, service, operation, _, _) =
            resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let resolutions = vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )];
        let repository = git2::Repository::open(root.path()).unwrap();
        let before = resolution_preflight(&repository, &resolutions).unwrap();
        let before_head = repository.head().unwrap().target();
        let before_index = fs::read(repository.path().join("index")).unwrap();
        match race {
            "staged" => {
                fs::write(root.path().join("fixture.txt"), b"clean staged race\n").unwrap();
                let mut index = repository.index().unwrap();
                index.add_path(Path::new("fixture.txt")).unwrap();
                index.write().unwrap();
            }
            "untracked" => fs::write(root.path().join("noncolliding-untracked"), b"race").unwrap(),
            "bytes" => fs::write(root.path().join("docs/document.md"), b"external bytes").unwrap(),
            "symlink" => {
                fs::remove_file(root.path().join("docs/document.md")).unwrap();
                std::os::unix::fs::symlink("../fixture.txt", root.path().join("docs/document.md"))
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                resolution_preflight(&repository, &resolutions),
                Ok(after) if after != before
            ) || matches!(
                resolution_preflight(&repository, &resolutions),
                Err(SynchronizationError::ExternalChange)
            )
        );
        // The seam itself is observation-only: it cannot move refs, modify the
        // index, write a candidate, or create an attempt row.
        assert_eq!(repository.head().unwrap().target(), before_head);
        if race != "staged" {
            assert_eq!(
                fs::read(repository.path().join("index")).unwrap(),
                before_index
            );
        }
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_resolution_attempts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
}

#[test]
fn initial_resolution_rejects_unrelated_staged_clean_merge_entry_without_writes() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let repository = git2::Repository::open(root.path()).unwrap();
    let blob = repository.blob(b"unrelated staged clean entry\n").unwrap();
    let mut index = repository.index().unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 29,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: b"clean-staged-race.txt".to_vec(),
        })
        .unwrap();
    index.write().unwrap();
    let image = (
        repository.head().unwrap().target(),
        fs::read(repository.path().join("index")).unwrap(),
        fs::read(root.path().join("docs/document.md")).unwrap(),
    );
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::StaleObservation
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(repository.head().unwrap().target(), image.0);
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), image.1);
    assert_eq!(
        fs::read(root.path().join("docs/document.md")).unwrap(),
        image.2
    );
    assert_eq!(
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT count(*) FROM remote_resolution_attempts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn index_lock_revalidates_target_change_after_preflight_without_clobbering_it() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let external = b"external target change while index.lock is held\n".to_vec();
    let target = root.path().join("docs/document.md");
    let hook_external = external.clone();
    set_resolution_index_lock_hook(root.path().to_owned(), move || {
        fs::write(target, &hook_external).unwrap()
    });
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::StaleObservation
    ));
    assert_eq!(
        fs::read(root.path().join("docs/document.md")).unwrap(),
        external
    );
    assert!(
        git2::Repository::open(root.path())
            .unwrap()
            .path()
            .join("index.lock")
            .exists()
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_index_lock_refuses_before_creating_index_lock() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request),
        Err(SynchronizationError::ExternalChange)
    ));
    assert!(!repository.path().join("index.lock").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn index_lock_pathname_substitution_before_persist_is_refused() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let index = fs::read(repository.path().join("index")).unwrap();
    let external_lock = b"external substituted index lock\n".to_vec();
    let gitdir = repository.path().to_owned();
    let hook_lock = external_lock.clone();
    set_resolution_index_persist_hook(root.path().to_owned(), move || {
        let replacement = gitdir.join("external-index-lock");
        fs::write(&replacement, hook_lock).unwrap();
        fs::rename(replacement, gitdir.join("index.lock")).unwrap();
    });
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request),
        Err(SynchronizationError::ExternalChange)
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
    assert_eq!(
        fs::read(repository.path().join("index.lock")).unwrap(),
        external_lock
    );
}

#[cfg(target_os = "linux")]
#[test]
fn unrelated_scratch_leaf_is_never_a_serialization_source() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let original_index = fs::read(repository.path().join("index")).unwrap();
    let gitdir = repository.path().to_owned();
    set_resolution_index_scratch_hook(root.path().to_owned(), move || {
        // The backend has serialized the operation-private index. An unrelated
        // leaf is never selected as the serialization/install source.
        let replacement = gitdir.join(".manyhands-index-scratch-race");
        fs::copy(gitdir.join("index"), replacement).unwrap();
    });
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_ne!(
        fs::read(repository.path().join("index")).unwrap(),
        original_index
    );
    assert_eq!(
        fs::read(repository.path().join(".manyhands-index-scratch-race")).unwrap(),
        original_index
    );
}

#[cfg(target_os = "linux")]
fn index_with_extension(mut index: Vec<u8>, signature: &[u8; 4]) -> Vec<u8> {
    index.truncate(index.len() - 20);
    index.extend_from_slice(signature);
    index.extend_from_slice(&0_u32.to_be_bytes());
    // The extension policy scanner does not implement a checksum. Actual
    // serialization/reads are validated by libgit2; these policy-only images
    // deliberately have a dummy checksum.
    index.extend_from_slice(&[0; 20]);
    index
}

#[cfg(target_os = "linux")]
#[test]
fn unsupported_index_extensions_are_rejected_before_index_lock_creation() {
    for extension in [b"link", b"abcd", b"ABCD"] {
        let (root, _data, _service) = fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        let index_path = repository.path().join("index");
        let extension_index = index_with_extension(fs::read(&index_path).unwrap(), extension);
        fs::write(&index_path, &extension_index).unwrap();
        assert!(matches!(
            approved_index_extensions(&extension_index),
            Err(SynchronizationError::ExternalChange)
        ));
        assert_eq!(fs::read(&index_path).unwrap(), extension_index);
        assert!(!repository.path().join("index.lock").exists());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn advisory_index_extensions_are_explicitly_rebuildable() {
    let (root, _data, _service) = fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let index = index_with_extension(fs::read(repository.path().join("index")).unwrap(), b"TREE");
    assert!(approved_index_extensions(&index).is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn external_index_substitution_after_persist_rejects_before_ref_transition() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let head = repository.head().unwrap().target();
    let external_index = b"external index after resolution persist\n".to_vec();
    let gitdir = repository.path().to_owned();
    let hook_index = external_index.clone();
    set_resolution_index_effect_hook(root.path().to_owned(), move || {
        let replacement = gitdir.join("external-index-after-persist");
        fs::write(&replacement, hook_index).unwrap();
        fs::rename(replacement, gitdir.join("index")).unwrap();
    });
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request),
        Err(SynchronizationError::ExternalChange)
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(repository.head().unwrap().target(), head);
    assert_eq!(
        fs::read(repository.path().join("index")).unwrap(),
        external_index
    );
    assert!(repository.path().join("index.lock").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn substituted_lock_before_verified_release_is_preserved() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let external_lock = b"external lock during guarded retirement\n".to_vec();
    let gitdir = repository.path().to_owned();
    let hook_lock = external_lock.clone();
    set_resolution_index_retire_hook(root.path().to_owned(), move || {
        let replacement = gitdir.join("external-retire-lock");
        fs::write(&replacement, hook_lock).unwrap();
        fs::rename(replacement, gitdir.join("index.lock")).unwrap();
    });
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request),
        Err(SynchronizationError::ExternalChange)
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(
        fs::read(repository.path().join("index.lock")).unwrap(),
        external_lock
    );
}

#[cfg(target_os = "linux")]
#[test]
fn substituted_lock_after_install_is_retained_without_cleanup() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let external_lock = b"external lock after index installation\n".to_vec();
    let gitdir = repository.path().to_owned();
    let hook_lock = external_lock.clone();
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        let replacement = gitdir.join("external-post-install-lock");
        fs::write(&replacement, hook_lock).unwrap();
        fs::rename(replacement, gitdir.join("index.lock")).unwrap();
    });
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request),
        Err(SynchronizationError::ExternalChange)
    ));
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(
        fs::read(repository.path().join("index.lock")).unwrap(),
        external_lock
    );
}

#[test]
fn index_lock_hooks_are_independent_across_worktrees() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (hook_root, _hook_data, hook_service, hook_operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let (other_root, _other_data, other_service, other_operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let hook_inspection = hook_service
        .inspect_synchronization_recovery(hook_root.path(), hook_operation)
        .unwrap();
    let other_inspection = other_service
        .inspect_synchronization_recovery(other_root.path(), other_operation)
        .unwrap();
    let hook_external = b"external first root-specific index hook\n".to_vec();
    let other_external = b"external second root-specific index hook\n".to_vec();
    let hook_target = hook_root.path().join("docs/document.md");
    let other_target = other_root.path().join("docs/document.md");
    let first_hook_external = hook_external.clone();
    let second_hook_external = other_external.clone();
    set_resolution_index_lock_hook(hook_root.path().to_owned(), move || {
        fs::write(hook_target, first_hook_external).unwrap()
    });
    set_resolution_index_lock_hook(other_root.path().to_owned(), move || {
        fs::write(other_target, second_hook_external).unwrap()
    });
    let request = |root: &std::path::Path,
                   operation,
                   inspection: &SynchronizationConflictInspection| {
        ResolveSynchronizationRequest::new(
            root.to_owned(),
            operation,
            OperationId::new(),
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
            )],
            None,
        )
    };
    assert!(matches!(
        other_service
            .resolve_synchronization(request(
                other_root.path(),
                other_operation,
                &other_inspection
            ))
            .unwrap(),
        ResolveSynchronizationOutcome::StaleObservation
    ));
    assert!(matches!(
        hook_service
            .resolve_synchronization(request(hook_root.path(), hook_operation, &hook_inspection))
            .unwrap(),
        ResolveSynchronizationOutcome::StaleObservation
    ));
    assert_eq!(
        fs::read(other_root.path().join("docs/document.md")).unwrap(),
        other_external
    );
    assert_eq!(
        fs::read(hook_root.path().join("docs/document.md")).unwrap(),
        hook_external
    );
}

#[cfg(unix)]
#[test]
fn resolution_preflight_no_follow_read_rejects_deterministic_symlink_swap() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let resolutions = vec![(
        inspection.paths[0].token.clone(),
        RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
    )];
    let repository = git2::Repository::open(root.path()).unwrap();
    let head = repository.head().unwrap().target();
    let index = fs::read(repository.path().join("index")).unwrap();
    let victim = root.path().join("docs/document.md");
    service.set_owned_path_hook_for_root_for_testing(
        root.path().to_owned(),
        "docs/document.md".into(),
        OwnedPathBoundary::Read,
        move || {
            fs::remove_file(&victim).unwrap();
            std::os::unix::fs::symlink("../fixture.txt", &victim).unwrap();
        },
    );
    assert!(matches!(
        resolution_preflight(&repository, &resolutions),
        Err(SynchronizationError::ExternalChange)
    ));
    // The descriptor read rejects the swapped link before any resolution
    // mutation or durable attempt record can be created.
    assert_eq!(repository.head().unwrap().target(), head);
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
    assert_eq!(
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT count(*) FROM remote_resolution_attempts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn ignored_file_does_not_break_exact_post_ref_retry() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let resolved = base.replace("base", "resolved");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let repository = git2::Repository::open(root.path()).unwrap();
    fs::write(repository.path().join("info/exclude"), b"task4-ignored\n").unwrap();
    fs::write(root.path().join("task4-ignored"), b"ignored unchanged\n").unwrap();
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request = || {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(resolved.as_bytes().to_vec()),
            )],
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(matches!(
        service.resolve_synchronization(request()),
        Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure
    ));
    *service.failure_point.lock().unwrap() = None;
    assert!(matches!(
        service.resolve_synchronization(request()).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    assert_eq!(
        fs::read(root.path().join("task4-ignored")).unwrap(),
        b"ignored unchanged\n"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn writer_boundary_rejects_regular_target_replacement_without_advancing_path() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let external = base.replace("base", "external writer-boundary replacement");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let image = (
        repository.head().unwrap().target(),
        fs::read(repository.path().join("index")).unwrap(),
    );
    let target = root.path().join("docs/document.md");
    let replacement = root.path().join("docs/external-replacement.md");
    service.set_owned_path_hook_for_root_for_testing(
        root.path().to_owned(),
        "docs/document.md".into(),
        OwnedPathBoundary::Replace,
        move || {
            fs::write(&replacement, &external).unwrap();
            fs::rename(replacement, target).unwrap();
        },
    );
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    let error = service.resolve_synchronization(request).unwrap_err();
    assert!(
        matches!(error, SynchronizationError::Repository(ref error) if error.kind == RepositoryErrorKind::ExternalChange),
        "writer boundary error: {error:?}"
    );
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(repository.head().unwrap().target(), image.0);
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), image.1);
    assert_eq!(
        fs::read_to_string(root.path().join("docs/document.md")).unwrap(),
        base.replace("base", "external writer-boundary replacement")
    );
    assert!(
        !rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row("SELECT applied FROM remote_resolution_paths", [], |row| row
                .get::<_, bool>(0))
            .unwrap()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn writer_temp_source_substitution_is_rejected_without_applying_resolution() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let external = base.replace("base", "external temporary source replacement");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let image = (
        repository.head().unwrap().target(),
        fs::read(repository.path().join("index")).unwrap(),
    );
    let gitdir = repository.path().to_owned();
    service.set_owned_path_hook_for_root_for_testing(
        root.path().to_owned(),
        "docs/document.md".into(),
        OwnedPathBoundary::TempWritten,
        move || {
            let temp = fs::read_dir(&gitdir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with(".manyhands-write-"))
                })
                .expect("guarded writer created its temporary source");
            fs::write(temp, &external).unwrap();
        },
    );
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    let error = service.resolve_synchronization(request).unwrap_err();
    assert!(
        matches!(error, SynchronizationError::Repository(ref error) if error.kind == RepositoryErrorKind::ExternalChange),
        "temporary source substitution error: {error:?}"
    );
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(repository.head().unwrap().target(), image.0);
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), image.1);
    assert_eq!(
        fs::read_to_string(root.path().join("docs/document.md")).unwrap(),
        base.replace("base", "external temporary source replacement")
    );
    assert!(
        !rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row("SELECT applied FROM remote_resolution_paths", [], |row| row
                .get::<_, bool>(0))
            .unwrap()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn verified_guarded_temp_replacement_is_retained_without_pathname_cleanup() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let external = b"external post-verification temporary replacement\n".to_vec();
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let gitdir = repository.path().to_owned();
    let hook_gitdir = gitdir.clone();
    let hook_external = external.clone();
    service.set_owned_path_hook_for_root_for_testing(
        root.path().to_owned(),
        "docs/document.md".into(),
        OwnedPathBoundary::TempVerified,
        move || {
            let temp = fs::read_dir(&hook_gitdir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with(".manyhands-write-"))
                })
                .expect("guarded exchange left its private temporary leaf");
            let replacement = hook_gitdir.join("external-verified-temp");
            fs::write(&replacement, &hook_external).unwrap();
            fs::rename(replacement, temp).unwrap();
        },
    );
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    assert!(matches!(
        service.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    let retained = fs::read_dir(gitdir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(".manyhands-write-"))
        })
        .expect("post-verification replacement was not pathname-unlinked");
    assert_eq!(fs::read(retained).unwrap(), external);
}

#[test]
fn candidate_recovery_refuses_and_preserves_forged_merge_rebase_and_cherry_metadata() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    for (metadata, value) in [
        ("MERGE_HEAD", format!("{}\n", git2::Oid::zero())),
        ("REBASE_HEAD", format!("{}\n", git2::Oid::zero())),
        ("CHERRY_PICK_HEAD", format!("{}\n", git2::Oid::zero())),
    ] {
        let (root, _data, service, operation, _, _) =
            resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let attempt = OperationId::new();
        let request = || {
            ResolveSynchronizationRequest::new(
                root.path().to_owned(),
                operation,
                attempt,
                inspection.observation.clone(),
                vec![(
                    inspection.paths[0].token.clone(),
                    RedactedConflictBytes::from_bytes(
                        base.replace("base", "resolved").into_bytes(),
                    ),
                )],
                None,
            )
        };
        *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
        assert!(matches!(
            service.resolve_synchronization(request()),
            Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure
        ));
        *service.failure_point.lock().unwrap() = None;
        let repository = git2::Repository::open(root.path()).unwrap();
        let image = (
            repository.head().unwrap().target(),
            fs::read(repository.path().join("index")).unwrap(),
            fs::read(root.path().join("docs/document.md")).unwrap(),
        );
        let metadata_path = repository.path().join(metadata);
        fs::write(&metadata_path, &value).unwrap();
        assert!(matches!(
            service.resolve_synchronization(request()),
            Err(SynchronizationError::ExternalChange)
        ));
        let repository = git2::Repository::open(root.path()).unwrap();
        assert_eq!(repository.head().unwrap().target(), image.0, "{metadata}");
        assert_eq!(
            fs::read(repository.path().join("index")).unwrap(),
            image.1,
            "{metadata}"
        );
        assert_eq!(
            fs::read(root.path().join("docs/document.md")).unwrap(),
            image.2,
            "{metadata}"
        );
        assert_eq!(
            fs::read(metadata_path).unwrap(),
            value.as_bytes(),
            "{metadata}"
        );
    }
}

#[test]
fn post_ref_candidate_recovery_requires_exact_multi_path_evidence() {
    let base_one = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"One\"\n---\n\nbase\n";
    let base_two = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Two\"\n---\n\nbase\n";
    let local_one = base_one.replace("base", "local");
    let remote_one = base_one.replace("base", "remote");
    let local_two = base_two.replace("base", "local");
    let remote_two = base_two.replace("base", "remote");
    let (root, data, service, operation, _, _) = resolution_fixture(&[
        (
            "docs/one.md",
            base_one,
            local_one.as_str(),
            remote_one.as_str(),
        ),
        (
            "docs/two.md",
            base_two,
            local_two.as_str(),
            remote_two.as_str(),
        ),
    ]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let correct_values = || {
        vec![
            (
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(
                    base_one.replace("base", "resolved one").into_bytes(),
                ),
            ),
            (
                inspection.paths[1].token.clone(),
                RedactedConflictBytes::from_bytes(
                    base_two.replace("base", "resolved two").into_bytes(),
                ),
            ),
        ]
    };
    let correct = || {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            correct_values(),
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(matches!(
        service.resolve_synchronization(correct()),
        Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure
    ));
    *service.failure_point.lock().unwrap() = None;
    let repository = git2::Repository::open(root.path()).unwrap();
    let image = (
        repository.head().unwrap().target(),
        fs::read(repository.path().join("index")).unwrap(),
        fs::read(root.path().join("docs/one.md")).unwrap(),
        fs::read(root.path().join("docs/two.md")).unwrap(),
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT phase,candidate_oid FROM remote_resolution_attempts",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .unwrap(),
    );
    let malformed = [
        // Subset, extra duplicate, and changed result all fail before a ref,
        // index, worktree, candidate, or metadata transition.
        {
            let mut values = correct_values();
            vec![values.remove(0)]
        },
        {
            let mut values = correct_values();
            values.push(values[0].clone());
            values
        },
        {
            let mut values = correct_values();
            vec![
                values.remove(0),
                (
                    inspection.paths[1].token.clone(),
                    RedactedConflictBytes::from_bytes(
                        base_two.replace("base", "altered").into_bytes(),
                    ),
                ),
            ]
        },
        {
            let mut values = correct_values();
            // Same ordinal/path/result but a substituted immutable conflict
            // side must not recover a post-ref candidate.
            values[0].0.base = Some(git2::Oid::zero());
            values
        },
    ];
    for resolutions in malformed {
        let request = ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            resolutions,
            None,
        );
        assert!(matches!(
            service.resolve_synchronization(request),
            Err(SynchronizationError::RecoveryRequired)
        ));
        let repository = git2::Repository::open(root.path()).unwrap();
        assert_eq!(repository.head().unwrap().target(), image.0);
        assert_eq!(fs::read(repository.path().join("index")).unwrap(), image.1);
        assert_eq!(fs::read(root.path().join("docs/one.md")).unwrap(), image.2);
        assert_eq!(fs::read(root.path().join("docs/two.md")).unwrap(), image.3);
        assert_eq!(
            rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
                .unwrap()
                .query_row(
                    "SELECT phase,candidate_oid FROM remote_resolution_attempts",
                    [],
                    |row| { Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)) }
                )
                .unwrap(),
            image.4
        );
    }
    assert!(matches!(
        service.resolve_synchronization(correct()).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

#[test]
fn context_primary_ordinal_one_candidate_retry_uses_attempt_foreign_key() {
    let (root, data, service, operation, worktree, local, incoming, ticket_id) =
        context_primary_conflict_fixture();
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    assert_eq!(inspection.stage, SynchronizationStage::Primary);
    assert_eq!(inspection.observation.ordinal, 1);
    let attempt = OperationId::new();
    let request = || {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(
                    "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Primary stage\"\n---\n\nresolved\n"
                        .as_bytes()
                        .to_vec(),
                ),
            )],
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(matches!(
        service.resolve_synchronization(request()),
        Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure
    ));
    *service.failure_point.lock().unwrap() = None;
    let repository = git2::Repository::open(&worktree).unwrap();
    let candidate = repository.head().unwrap().target().unwrap();
    let commit = repository.find_commit(candidate).unwrap();
    assert_eq!(
        [commit.parent_id(0).unwrap(), commit.parent_id(1).unwrap()],
        [local, incoming]
    );
    let image = (
        fs::read(repository.path().join("index")).unwrap(),
        fs::read(worktree.join("docs/primary.md")).unwrap(),
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .prepare("SELECT ordinal,phase,result_oid,candidate_oid FROM remote_integration_steps ORDER BY ordinal")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        image.2[0],
        (0, "applied".into(), Some(local.to_string()), None)
    );
    assert_eq!(
        image.2[1],
        (
            1,
            "commit_prepared".into(),
            None,
            Some(candidate.to_string())
        )
    );
    assert!(matches!(
        service.resolve_synchronization(request()).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate
    ));
    let repository = git2::Repository::open(&worktree).unwrap();
    assert_eq!(repository.head().unwrap().target(), Some(candidate));
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), image.0);
    assert_eq!(fs::read(worktree.join("docs/primary.md")).unwrap(), image.1);
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_resolution_attempts",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT ordinal FROM remote_resolution_attempts attempt JOIN remote_integration_steps step ON step.id=attempt.integration_step_id", [], |row| row.get::<_, i64>(0)).unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_integration_steps WHERE ordinal=0 AND phase='applied' AND result_oid=?1", [local.to_string()], |row| row.get::<_, i64>(0)).unwrap(),
        1
    );
    let _ = ticket_id; // Documents the public context identity used by the fixture.
}

#[test]
fn confirmed_identity_post_ref_retry_requires_the_exact_confirmation() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, data, service, operation, local_parent, incoming) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let repository = git2::Repository::open(root.path()).unwrap();
    let mut config = repository.config().unwrap();
    // Empty repository-local values prevent libgit2 from falling through to a
    // developer's global identity while exercising the public confirmation.
    config.set_str("user.name", "").unwrap();
    config.set_str("user.email", "").unwrap();
    config.set_bool("user.useConfigOnly", true).unwrap();
    drop(config);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let confirmation = ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration: inspection.observation.configuration,
    };
    let request = |identity| {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
            )],
            identity,
        )
    };
    assert!(matches!(
        service.resolve_synchronization(request(None)).unwrap(),
        ResolveSynchronizationOutcome::IdentityRequired
    ));
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(matches!(
        service.resolve_synchronization(request(Some(confirmation.clone()))),
        Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure
    ));
    *service.failure_point.lock().unwrap() = None;
    let repository = git2::Repository::open(root.path()).unwrap();
    let candidate = repository.head().unwrap().target().unwrap();
    let commit = repository.find_commit(candidate).unwrap();
    assert_eq!(
        [commit.parent_id(0).unwrap(), commit.parent_id(1).unwrap()],
        [local_parent, incoming]
    );
    let image = (
        fs::read(repository.path().join("index")).unwrap(),
        fs::read(root.path().join("docs/document.md")).unwrap(),
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT attempt.phase,attempt.checkpoint_oid,confirmation.phase FROM remote_resolution_attempts attempt JOIN remote_identity_confirmations confirmation ON confirmation.id=attempt.identity_confirmation_id",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, String>(2)?)),
            )
            .unwrap(),
    );
    let changed = ConfirmedCommitIdentity {
        identity: CommitIdentity {
            name: "Changed".into(),
            email: "confirmed@example.invalid".into(),
        },
        ..confirmation.clone()
    };
    for identity in [None, Some(changed)] {
        assert!(matches!(
            service.resolve_synchronization(request(identity)),
            Err(SynchronizationError::RecoveryRequired)
        ));
        let repository = git2::Repository::open(root.path()).unwrap();
        assert_eq!(repository.head().unwrap().target(), Some(candidate));
        assert_eq!(fs::read(repository.path().join("index")).unwrap(), image.0);
        assert_eq!(
            fs::read(root.path().join("docs/document.md")).unwrap(),
            image.1
        );
        assert_eq!(
            rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
                .unwrap()
                .query_row(
                    "SELECT attempt.phase,attempt.checkpoint_oid,confirmation.phase FROM remote_resolution_attempts attempt JOIN remote_identity_confirmations confirmation ON confirmation.id=attempt.identity_confirmation_id",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, String>(2)?)),
                )
                .unwrap(),
            image.2
        );
    }
    assert!(matches!(
        service.resolve_synchronization(request(Some(confirmation))).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate
    ));
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_identity_confirmations",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT checkpoint_oid FROM remote_resolution_attempts",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        candidate.to_string()
    );
}

#[test]
fn effective_repository_identity_ignores_caller_confirmation_and_retry() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let supplied = ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Ignored caller".into(),
            email: "ignored@example.invalid".into(),
        },
        expected_configuration: inspection.observation.configuration,
    };
    let request = |identity| {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
            )],
            identity,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(matches!(
        service.resolve_synchronization(request(Some(supplied))),
        Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure
    ));
    *service.failure_point.lock().unwrap() = None;
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_identity_confirmations",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(
        db.query_row(
            "SELECT identity_confirmation_id IS NULL FROM remote_resolution_attempts",
            [],
            |row| row.get::<_, bool>(0)
        )
        .unwrap()
    );
    drop(db);
    // The exact candidate resumes with no caller confirmation because the
    // repository's effective signature, not the ignored request field, bound
    // the attempt input.
    assert!(matches!(
        service.resolve_synchronization(request(None)).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

#[test]
fn document_ticket_comment_kind_change_is_external_only_without_effects() {
    let document = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let ticket = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Ticket\"\ntype: \"task\"\nstatus: \"open\"\n---\n\nlocal\n";
    let comment = "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\nitem_id: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n\nremote\n";
    let (root, data, service, operation, local, _) =
        resolution_fixture(&[("docs/kind-change.md", document, ticket, comment)]);
    let repository = git2::Repository::open(root.path()).unwrap();
    let index = fs::read(repository.path().join("index")).unwrap();
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    assert!(inspection.paths.iter().all(|path| {
        path.eligibility == merge::ConflictEligibility::ExternalResolutionRequired
    }));
    assert_eq!(repository.head().unwrap().target(), Some(local));
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
    assert_eq!(
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT count(*) FROM remote_resolution_attempts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[cfg(target_os = "linux")]
#[test]
fn copied_content_foreign_lock_is_never_recovered_as_owned() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, &local, &remote)]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request = || {
        ResolveSynchronizationRequest::new(
            root.path().to_owned(),
            operation,
            attempt,
            inspection.observation.clone(),
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
            )],
            None,
        )
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterCandidatePrepared);
    assert!(service.resolve_synchronization(request()).is_err());
    let repository = git2::Repository::open(root.path()).unwrap();
    let lock = repository.path().join("index.lock");
    let bytes = fs::read(&lock).unwrap();
    // Exact sentinel bytes, different filesystem identity; content is not provenance.
    fs::remove_file(&lock).ok();
    fs::write(&lock, &bytes).unwrap();
    let identity = fs::metadata(&lock).unwrap().ino();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request()),
        Err(SynchronizationError::ExternalChange)
    ));
    assert_eq!(fs::read(&lock).unwrap(), bytes);
    assert_eq!(fs::metadata(&lock).unwrap().ino(), identity);
}

#[cfg(target_os = "linux")]
#[test]
fn sentinel_identity_and_bytes_are_stable_across_index_persistence() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, &local, &remote)]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let before = std::sync::Arc::new(std::sync::Mutex::new(None));
    let captured = before.clone();
    let repository = git2::Repository::open(root.path()).unwrap();
    let lock = repository.path().join("index.lock");
    let acquired = lock.clone();
    set_resolution_index_lock_hook(root.path().to_owned(), move || {
        let metadata = fs::metadata(&acquired).unwrap();
        *captured.lock().unwrap() =
            Some((metadata.dev(), metadata.ino(), fs::read(&acquired).unwrap()));
    });
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        let metadata = fs::metadata(&lock).unwrap();
        assert_eq!(
            Some((metadata.dev(), metadata.ino(), fs::read(&lock).unwrap())),
            *before.lock().unwrap()
        );
    });
    assert!(matches!(
        service
            .resolve_synchronization(ResolveSynchronizationRequest::new(
                root.path().to_owned(),
                operation,
                OperationId::new(),
                inspection.observation,
                vec![(
                    inspection.paths[0].token.clone(),
                    RedactedConflictBytes::from_bytes(
                        base.replace("base", "resolved").into_bytes()
                    )
                )],
                None
            ))
            .unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

#[cfg(target_os = "linux")]
fn protocol_resolution_fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    ResolveSynchronizationRequest,
) {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\nfuture_metadata: retained\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, &local, &remote)]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let request = ResolveSynchronizationRequest::new(
        root.path().to_owned(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.replace("base", "resolved").into_bytes()),
        )],
        None,
    );
    (root, data, service, request)
}

#[cfg(target_os = "linux")]
#[test]
fn acquisition_observation_transaction_failure_recovers_before_candidate_without_drop_cleanup() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let database = data.path().join(REGISTRY_FILE);
    let hook_database = database.clone();
    set_resolution_index_lock_hook(root.path().to_owned(), move || {
        rusqlite::Connection::open(hook_database).unwrap().execute_batch("CREATE TRIGGER fail_publication_observation BEFORE UPDATE OF phase ON remote_resolution_index_artifacts WHEN NEW.phase='published' BEGIN SELECT RAISE(ABORT,'test observation fault'); END;").unwrap();
    });
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let repository = git2::Repository::open(root.path()).unwrap();
    let identity = fs::metadata(repository.path().join("index.lock"))
        .unwrap()
        .ino();
    let connection = rusqlite::Connection::open(&database).unwrap();
    let progress: (String, String, Option<String>) = connection.query_row("SELECT artifact.phase,attempt.phase,attempt.candidate_oid FROM remote_resolution_index_artifacts artifact JOIN remote_resolution_attempts attempt ON attempt.id=artifact.attempt_id", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(progress, ("intent".into(), "paths_applying".into(), None));
    connection
        .execute_batch("DROP TRIGGER fail_publication_observation;")
        .unwrap();
    let check = repository.path().join("index.lock");
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        assert_eq!(fs::metadata(check).unwrap().ino(), identity)
    });
    // No Drop cleanup exists; a newly opened service recognizes only the anchor.
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn release_observation_transaction_failure_recovers_absence_and_refuses_foreign_replacement() {
    for foreign in [false, true] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let database = data.path().join(REGISTRY_FILE);
        let hook_database = database.clone();
        set_resolution_index_retire_hook(root.path().to_owned(), move || {
            rusqlite::Connection::open(hook_database).unwrap().execute_batch("CREATE TRIGGER fail_release_observation BEFORE UPDATE OF phase ON remote_resolution_index_artifacts WHEN NEW.phase='released' BEGIN SELECT RAISE(ABORT,'test observation fault'); END;").unwrap();
        });
        assert!(service.resolve_synchronization(request.clone()).is_err());
        let repository = git2::Repository::open(root.path()).unwrap();
        let candidate = repository.head().unwrap().target().unwrap();
        let lock = repository.path().join("index.lock");
        assert!(!lock.exists());
        let connection = rusqlite::Connection::open(&database).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT phase FROM remote_resolution_index_artifacts",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "release_intent"
        );
        connection
            .execute_batch("DROP TRIGGER fail_release_observation;")
            .unwrap();
        if foreign {
            fs::write(&lock, b"foreign release lock").unwrap();
        }
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        let outcome = restarted.resolve_synchronization(request);
        if foreign {
            assert!(matches!(outcome, Err(SynchronizationError::ExternalChange)));
            assert_eq!(fs::read(&lock).unwrap(), b"foreign release lock");
        } else {
            assert!(
                matches!(outcome.unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
            );
            assert_eq!(
                connection
                    .query_row(
                        "SELECT phase FROM remote_resolution_index_artifacts",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                "released"
            );
        }
        assert_eq!(repository.head().unwrap().target(), Some(candidate));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn pre_ref_installed_index_old_head_reuses_candidate_and_preserves_foreign_backend_lock() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let old_head = repository.head().unwrap().target().unwrap();
    set_resolution_index_install_hook(root.path().to_owned(), || {
        panic!("test termination after installation")
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || service.resolve_synchronization(request.clone())
        ))
        .is_err()
    );
    assert_eq!(repository.head().unwrap().target(), Some(old_head));
    assert!(repository.path().join("index.lock").exists());
    let backend_lock = repository.path().join("refs/heads/main.lock");
    fs::write(&backend_lock, b"ambiguous backend lock").unwrap();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(fs::read(&backend_lock).unwrap(), b"ambiguous backend lock");
    assert_eq!(repository.head().unwrap().target(), Some(old_head));
    // Simulated quiesced operator action on this disposable fixture only.
    fs::remove_file(&backend_lock).unwrap();
    let candidate: String = rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .query_row(
            "SELECT candidate_oid FROM remote_resolution_attempts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        matches!(restarted.resolve_synchronization(request).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid.to_string() == candidate)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn metadata_members_reconcile_independent_absence_and_preserve_foreign_remnants() {
    for missing in RESOLUTION_MERGE_MEMBERS {
        for foreign in [false, true] {
            let (root, data, service, request) = protocol_resolution_fixture();
            *service.failure_point.lock().unwrap() =
                Some(FailurePoint::ResolutionAfterCheckpointObservation);
            assert!(service.resolve_synchronization(request.clone()).is_err());
            let repository = git2::Repository::open(root.path()).unwrap();
            let candidate = repository.head().unwrap().target();
            fs::remove_file(repository.path().join(missing)).unwrap();
            let remnant = repository.path().join(if missing == "MERGE_MSG" {
                "MERGE_MODE"
            } else {
                "MERGE_MSG"
            });
            if foreign {
                fs::write(&remnant, b"foreign merge metadata").unwrap();
            }
            let restarted = RepositoryService::open_at(data.path()).unwrap();
            let outcome = restarted.resolve_synchronization(request);
            if foreign {
                assert!(matches!(outcome, Err(SynchronizationError::ExternalChange)));
                assert_eq!(fs::read(remnant).unwrap(), b"foreign merge metadata");
                assert!(repository.path().join("index.lock").exists());
            } else {
                assert!(matches!(
                    outcome.unwrap(),
                    ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
                ));
                assert!(!repository.path().join("index.lock").exists());
                assert!(
                    RESOLUTION_MERGE_MEMBERS
                        .iter()
                        .all(|member| !repository.path().join(member).exists())
                );
            }
            assert_eq!(repository.head().unwrap().target(), candidate);
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn authoritative_libgit2_serialization_preserves_reuc_and_name_semantics() {
    // Valid v2 index with one REUC and NAME record. A static checksum fixture,
    // not a production or test reimplementation of the index serializer.
    let hex = "444952430000000200000000524555430000005a72657461696e6564003130303634340031303036343400313030363434000101010101010101010101010101010101010101020202020202020202020202020202020202020203030303030303030303030303030303030303034e414d4500000015616e636573746f72006f7572730074686569727300121d6a29fa7fac253bec9f4f1e91de4aba61867c";
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&hex[offset..offset + 2], 16).unwrap())
        .collect::<Vec<_>>();
    assert!(approved_index_extensions(&bytes).is_ok());
    let private = tempfile::tempdir().unwrap();
    let path = private.path().join("index");
    fs::write(&path, &bytes).unwrap();
    let mut authoritative = git2::Index::open(&path).unwrap();
    authoritative
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: git2::Oid::from_bytes(&[4; 20]).unwrap(),
            flags: 0,
            flags_extended: 0,
            path: b"resolved.md".to_vec(),
        })
        .unwrap();
    authoritative.write().unwrap();
    let serialized = fs::read(&path).unwrap();
    for (start, end) in [(12, 110), (110, 139)] {
        let extension = &bytes[start..end];
        assert!(
            serialized
                .windows(extension.len())
                .any(|window| window == extension),
            "semantic extension was lost or changed"
        );
    }
    let reopened = git2::Index::open(&path).unwrap();
    assert_eq!(reopened.len(), 1);
    assert!(!reopened.has_conflicts());
    assert!(approved_index_extensions(&serialized).is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn same_image_index_identity_substitution_is_refused_before_canonical_writes() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let original = fs::read(root.path().join("docs/document.md")).unwrap();
    let gitdir = repository.path().to_owned();
    set_resolution_index_lock_hook(root.path().to_owned(), move || {
        let replacement = gitdir.join("same-image-index");
        fs::copy(gitdir.join("index"), &replacement).unwrap();
        fs::rename(replacement, gitdir.join("index")).unwrap();
    });
    assert!(matches!(
        service.resolve_synchronization(request),
        Err(SynchronizationError::ExternalChange)
    ));
    assert_eq!(
        fs::read(root.path().join("docs/document.md")).unwrap(),
        original
    );
    assert_eq!(
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT count(*) FROM remote_resolution_paths WHERE applied=1",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[cfg(target_os = "linux")]
#[test]
fn uncertain_ref_intent_preserves_candidate_and_both_logs_after_operator_lock_handling() {
    let (root, data, service, request) = protocol_resolution_fixture();
    set_resolution_index_install_hook(root.path().to_owned(), || {
        panic!("test termination before ref invocation")
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || service.resolve_synchronization(request.clone())
        ))
        .is_err()
    );
    let repository = git2::Repository::open(root.path()).unwrap();
    let old_head = repository.head().unwrap().target();
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    // Simulate the actual backend crash window: intent is durable, the backend
    // may already have appended logs, but the loose ref is still old.
    connection
        .execute_batch("UPDATE remote_resolution_index_artifacts SET ref_phase='intent';")
        .unwrap();
    let candidate: String = connection
        .query_row(
            "SELECT candidate_oid FROM remote_resolution_attempts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let logs = [
        repository.path().join("logs/HEAD"),
        repository.path().join("logs/refs/heads/main"),
    ];
    for log in &logs {
        std::fs::OpenOptions::new()
            .append(true)
            .open(log)
            .unwrap()
            .write_all(b"test uncertain partial reflog append\n")
            .unwrap();
    }
    let before = logs
        .iter()
        .map(|log| fs::read(log).unwrap())
        .collect::<Vec<_>>();
    let backend_lock = repository.path().join("refs/heads/main.lock");
    fs::write(&backend_lock, b"ambiguous backend lock").unwrap();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    // Disposable fixture's simulated operator handles only the ambiguous lock.
    fs::remove_file(&backend_lock).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(
        logs.iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(repository.head().unwrap().target(), old_head);
    assert_eq!(
        connection
            .query_row(
                "SELECT candidate_oid FROM remote_resolution_attempts",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        candidate
    );
    assert!(repository.path().join("index.lock").exists());
}

fn resolution_request_for_bodies(
    root: &Path,
    operation: OperationId,
    attempt: OperationId,
    inspection: &SynchronizationConflictInspection,
    bodies: &[&str],
) -> ResolveSynchronizationRequest {
    ResolveSynchronizationRequest::new(
        root.to_owned(),
        operation,
        attempt,
        inspection.observation.clone(),
        inspection
            .paths
            .iter()
            .zip(bodies)
            .map(|(path, body)| {
                (
                    path.token.clone(),
                    RedactedConflictBytes::from_bytes(body.as_bytes().to_vec()),
                )
            })
            .collect(),
        None,
    )
}

const M2_DOCUMENT: &str = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\nunknown: {keep: [1, 2]}\n---\n\nbase\n";

#[cfg(unix)]
#[test]
fn missing_path_observation_replays_exact_result_without_second_replacement() {
    use std::os::unix::fs::MetadataExt;
    for (count, failed_ordinal) in [(1, 0), (2, 0), (2, 1)] {
        let second =
            M2_DOCUMENT.replace("01ARZ3NDEKTSV4RRFFQ69G5FAV", "01BX5ZZKBKACTAV9WEVGEMMVRZ");
        let bases = [M2_DOCUMENT.to_owned(), second];
        let locals = bases
            .iter()
            .map(|body| body.replace("base", "local"))
            .collect::<Vec<_>>();
        let incoming = bases
            .iter()
            .map(|body| body.replace("base", "incoming"))
            .collect::<Vec<_>>();
        let results = bases
            .iter()
            .map(|body| body.replace("base", "caller\n  exact bytes"))
            .collect::<Vec<_>>();
        let paths = ["docs/a.md", "docs/b.md"];
        let files = (0..count)
            .map(|i| {
                (
                    paths[i],
                    bases[i].as_str(),
                    locals[i].as_str(),
                    incoming[i].as_str(),
                )
            })
            .collect::<Vec<_>>();
        let (root, data, service, operation, head, _) = resolution_fixture(&files);
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let attempt = OperationId::new();
        let bodies = results[..count]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let request =
            || resolution_request_for_bodies(root.path(), operation, attempt, &inspection, &bodies);
        let repository = git2::Repository::open(root.path()).unwrap();
        let index = fs::read(repository.path().join("index")).unwrap();
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute_batch(&format!("CREATE TRIGGER fail_path_observation BEFORE UPDATE OF applied ON remote_resolution_paths WHEN NEW.ordinal={failed_ordinal} BEGIN SELECT RAISE(ABORT,'test observation failure'); END;")).unwrap();
        assert!(service.resolve_synchronization(request()).is_err());
        assert_eq!(repository.head().unwrap().target(), Some(head));
        assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
        assert!(
            !db.query_row(
                "SELECT applied FROM remote_resolution_paths WHERE ordinal=?1",
                [failed_ordinal],
                |row| row.get::<_, bool>(0)
            )
            .unwrap()
        );
        db.execute_batch("DROP TRIGGER fail_path_observation;")
            .unwrap();
        let altered = results[0].replace("caller", "altered");
        let mut altered_bodies = bodies.clone();
        altered_bodies[0] = &altered;
        assert!(
            service
                .resolve_synchronization(resolution_request_for_bodies(
                    root.path(),
                    operation,
                    attempt,
                    &inspection,
                    &altered_bodies
                ))
                .is_err()
        );
        let written_images = paths
            .iter()
            .take(failed_ordinal as usize + 1)
            .map(|path| {
                let metadata = fs::metadata(root.path().join(path)).unwrap();
                (
                    (*path).to_owned(),
                    (
                        metadata.dev(),
                        metadata.ino(),
                        metadata.mtime(),
                        metadata.mtime_nsec(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        for path in paths.iter().take(failed_ordinal as usize + 1) {
            service.set_owned_path_hook_for_root_for_testing(
                root.path().to_owned(),
                (*path).into(),
                OwnedPathBoundary::Replace,
                || panic!("bound result must not be replaced again"),
            );
        }
        let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
            service.resolve_synchronization(request()).unwrap()
        else {
            panic!("exact retry must complete")
        };
        for (path, image) in written_images {
            let metadata = fs::metadata(root.path().join(path)).unwrap();
            assert_eq!(
                (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.mtime(),
                    metadata.mtime_nsec()
                ),
                image,
                "no second actual write, including checkout"
            );
        }
        for i in 0..count {
            assert_eq!(
                fs::read(root.path().join(paths[i])).unwrap(),
                results[i].as_bytes()
            );
            assert_eq!(
                repository
                    .find_commit(commit_oid)
                    .unwrap()
                    .tree()
                    .unwrap()
                    .get_path(Path::new(paths[i]))
                    .unwrap()
                    .id(),
                repository.blob(results[i].as_bytes()).unwrap()
            );
        }
        assert!(
            matches!(service.resolve_synchronization(request()).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid: replay } if replay == commit_oid)
        );
    }
}

#[cfg(unix)]
#[test]
fn missing_path_observation_preserves_third_values_and_rejects_stale_or_unsafe_state() {
    use std::os::unix::fs::PermissionsExt;
    for change in [
        "third",
        "executable",
        "symlink",
        "index",
        "head",
        "metadata",
        "unrelated",
    ] {
        let local = M2_DOCUMENT.replace("base", "local");
        let incoming = M2_DOCUMENT.replace("base", "incoming");
        let result = M2_DOCUMENT.replace("base", "caller");
        let (root, data, service, operation, _, _) =
            resolution_fixture(&[("docs/a.md", M2_DOCUMENT, &local, &incoming)]);
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let attempt = OperationId::new();
        let request = || {
            resolution_request_for_bodies(root.path(), operation, attempt, &inspection, &[&result])
        };
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute_batch("CREATE TRIGGER fail_path_observation BEFORE UPDATE OF applied ON remote_resolution_paths BEGIN SELECT RAISE(ABORT,'test observation failure'); END;").unwrap();
        assert!(service.resolve_synchronization(request()).is_err());
        db.execute_batch("DROP TRIGGER fail_path_observation;")
            .unwrap();
        let repository = git2::Repository::open(root.path()).unwrap();
        let path = root.path().join("docs/a.md");
        match change {
            "third" => fs::write(&path, b"third value must survive\n").unwrap(),
            "executable" => fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap(),
            "symlink" => {
                fs::remove_file(&path).unwrap();
                std::os::unix::fs::symlink("../fixture.txt", &path).unwrap();
            }
            "index" => fs::write(repository.path().join("index"), b"external index image").unwrap(),
            "head" => {
                repository
                    .reference(
                        "refs/heads/main",
                        repository
                            .head()
                            .unwrap()
                            .peel_to_commit()
                            .unwrap()
                            .parent_id(0)
                            .unwrap(),
                        true,
                        "test",
                    )
                    .unwrap();
            }
            "metadata" => {
                fs::write(repository.path().join("MERGE_MSG"), b"foreign metadata\n").unwrap()
            }
            "unrelated" => fs::write(root.path().join("fixture.txt"), b"unrelated edit\n").unwrap(),
            _ => unreachable!(),
        }
        let head = repository.head().unwrap().target();
        let index = fs::read(repository.path().join("index")).unwrap();
        let bytes = fs::read(&path).unwrap();
        let metadata = fs::read(repository.path().join("MERGE_MSG")).unwrap();
        assert!(
            !matches!(
                service.resolve_synchronization(request()),
                Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete { .. })
            ),
            "{change}"
        );
        assert_eq!(repository.head().unwrap().target(), head, "{change}");
        assert_eq!(
            fs::read(repository.path().join("index")).unwrap(),
            index,
            "{change}"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes, "{change}");
        assert_eq!(
            fs::read(repository.path().join("MERGE_MSG")).unwrap(),
            metadata,
            "{change}"
        );
        assert!(
            !db.query_row("SELECT applied FROM remote_resolution_paths", [], |row| row
                .get::<_, bool>(0))
                .unwrap(),
            "{change}"
        );
    }
}

#[test]
fn all_recorded_ticket_sides_constrain_retained_closure_before_writes() {
    let open = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Ticket\"\ntype: \"cycle\"\nstatus: \"open\"\n---\n\nbase\n";
    let closed = open.replace(
        "status: \"open\"",
        "status: \"closed\"\nclosed_at: \"2026-01-01T00:00:00Z\"\nclosed_by: \"original author\"",
    );
    let other = closed.replace("original author", "different author");
    for (base, local, incoming, result, valid) in [
        (open, closed.as_str(), open, open, false),
        (open, open, closed.as_str(), open, false),
        (
            open,
            closed.as_str(),
            other.as_str(),
            closed.as_str(),
            false,
        ),
        (
            open,
            closed.as_str(),
            closed.as_str(),
            other.as_str(),
            false,
        ),
        (open, closed.as_str(), open, closed.as_str(), true),
        (open, open, closed.as_str(), closed.as_str(), true),
        (
            open,
            closed.as_str(),
            closed.as_str(),
            closed.as_str(),
            true,
        ),
        (
            closed.as_str(),
            closed.as_str(),
            closed.as_str(),
            closed.as_str(),
            true,
        ),
    ] {
        let local = local.replace("base", "local");
        let incoming = incoming.replace("base", "incoming");
        let result = result
            .replace("base", "exact caller closure\n")
            .replace("---\n\n", "custom: {preserve: true}\n---\n\n");
        let path = ".manyhands/tickets/01BX5ZZKBKACTAV9WEVGEMMVRZ/ticket.md";
        let (root, data, service, operation, head, _) =
            resolution_fixture(&[(path, base, &local, &incoming)]);
        let repository = git2::Repository::open(root.path()).unwrap();
        let index = fs::read(repository.path().join("index")).unwrap();
        let bytes = fs::read(root.path().join(path)).unwrap();
        let outcome = resolve_fixture(
            root.path(),
            &service,
            operation,
            &[&result, &result, &result],
        );
        if valid {
            assert!(matches!(
                outcome,
                ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
            ));
            assert_eq!(fs::read(root.path().join(path)).unwrap(), result.as_bytes());
        } else {
            assert_eq!(outcome, ResolveSynchronizationOutcome::ValidationFailed);
            assert_eq!(repository.head().unwrap().target(), Some(head));
            assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
            assert_eq!(fs::read(root.path().join(path)).unwrap(), bytes);
            assert_eq!(
                rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
                    .unwrap()
                    .query_row(
                        "SELECT count(*) FROM remote_resolution_attempts",
                        [],
                        |row| row.get::<_, i64>(0)
                    )
                    .unwrap(),
                0
            );
        }
    }
}

#[test]
fn all_recorded_comment_sides_constrain_immutable_fields_before_writes() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"01CRZ3NDEKTSV4RRFFQ69G5FAV\"\nitem_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ncreated_at: \"2026-01-01T00:00:00Z\"\nauthor: \"original author\"\n---\n\nbase\n";
    let path = ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01CRZ3NDEKTSV4RRFFQ69G5FAV.md";
    for field in ["author", "created", "parent", "item", "consistent"] {
        for changed_side in ["local", "incoming", "result"] {
            let changed = match field {
                "item" => base.replace(
                    "item_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"",
                    "item_id: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"",
                ),
                "parent" => base.replace(
                    "created_at:",
                    "parent_id: \"01DRZ3NDEKTSV4RRFFQ69G5FAV\"\ncreated_at:",
                ),
                "created" => base.replace("2026-01-01", "2026-01-02"),
                "author" => base.replace("original author", "different author"),
                "consistent" => base.to_owned(),
                _ => unreachable!(),
            };
            let local = if changed_side == "local" {
                &changed
            } else {
                base
            }
            .replace("base", "local");
            let incoming = if changed_side == "incoming" {
                &changed
            } else {
                base
            }
            .replace("base", "incoming");
            let result = if changed_side == "result" {
                &changed
            } else {
                base
            }
            .replace("base", "exact caller comment")
            .replace("---\n\n", "custom: [keep, unknown]\n---\n\n");
            let (root, data, service, operation, head, _) = resolution_fixture(&[
                ("docs/item.md", M2_DOCUMENT, M2_DOCUMENT, M2_DOCUMENT),
                (path, base, &local, &incoming),
            ]);
            let repository = git2::Repository::open(root.path()).unwrap();
            let index = fs::read(repository.path().join("index")).unwrap();
            let bytes = fs::read(root.path().join(path)).unwrap();
            let outcome = resolve_fixture(
                root.path(),
                &service,
                operation,
                &[&result, &result, &result],
            );
            if field == "consistent" {
                assert!(matches!(
                    outcome,
                    ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
                ));
                assert_eq!(fs::read(root.path().join(path)).unwrap(), result.as_bytes());
            } else {
                assert!(
                    matches!(
                        outcome,
                        ResolveSynchronizationOutcome::ValidationFailed
                            | ResolveSynchronizationOutcome::StaleObservation
                    ),
                    "{field}/{changed_side}: {outcome:?}"
                );
                assert_eq!(repository.head().unwrap().target(), Some(head));
                assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
                assert_eq!(fs::read(root.path().join(path)).unwrap(), bytes);
                assert_eq!(
                    rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
                        .unwrap()
                        .query_row(
                            "SELECT count(*) FROM remote_resolution_attempts",
                            [],
                            |row| row.get::<_, i64>(0)
                        )
                        .unwrap(),
                    0
                );
            }
        }
    }
}

#[test]
fn missing_path_observation_checks_all_images_before_completing_remaining_writes() {
    let second = M2_DOCUMENT.replace("01ARZ3NDEKTSV4RRFFQ69G5FAV", "01BX5ZZKBKACTAV9WEVGEMMVRZ");
    let local = M2_DOCUMENT.replace("base", "local");
    let incoming = M2_DOCUMENT.replace("base", "incoming");
    let second_local = second.replace("base", "local");
    let second_incoming = second.replace("base", "incoming");
    let result = M2_DOCUMENT.replace("base", "caller");
    let second_result = second.replace("base", "caller");
    let (root, data, service, operation, _, _) = resolution_fixture(&[
        ("docs/a.md", M2_DOCUMENT, &local, &incoming),
        ("docs/b.md", &second, &second_local, &second_incoming),
    ]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request = || {
        resolution_request_for_bodies(
            root.path(),
            operation,
            attempt,
            &inspection,
            &[&result, &second_result],
        )
    };
    let old = fs::read(root.path().join("docs/a.md")).unwrap();
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("CREATE TRIGGER fail_path_observation BEFORE UPDATE OF applied ON remote_resolution_paths WHEN NEW.ordinal=0 BEGIN SELECT RAISE(ABORT,'test observation failure'); END;").unwrap();
    assert!(service.resolve_synchronization(request()).is_err());
    db.execute_batch("DROP TRIGGER fail_path_observation;")
        .unwrap();
    // Even a still-authorized prewrite image must remain untouched when a
    // later member is a third value. No partial retry effect is needed here.
    fs::write(root.path().join("docs/a.md"), &old).unwrap();
    fs::write(root.path().join("docs/b.md"), b"third value\n").unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let head = repository.head().unwrap().target();
    let index = fs::read(repository.path().join("index")).unwrap();
    assert!(matches!(
        service.resolve_synchronization(request()),
        Err(SynchronizationError::ExternalChange)
    ));
    assert_eq!(fs::read(root.path().join("docs/a.md")).unwrap(), old);
    assert_eq!(
        fs::read(root.path().join("docs/b.md")).unwrap(),
        b"third value\n"
    );
    assert_eq!(repository.head().unwrap().target(), head);
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
    assert_eq!(
        db.query_row(
            "SELECT sum(applied) FROM remote_resolution_paths",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn retained_candidate_cannot_bypass_all_side_closure_validation() {
    // Construct evidence as an older writer could: its base-only validation
    // allowed an open result even though the local side retained closure.
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Ticket\"\ntype: \"cycle\"\nstatus: \"open\"\n---\n\nbase\n";
    let local = base.replace("status: \"open\"", "status: \"closed\"\nclosed_at: \"2026-01-01T00:00:00Z\"\nclosed_by: \"original author\"").replace("base", "local");
    let incoming = base.replace("base", "incoming");
    let result = base.replace("base", "legacy reopening");
    let path = ".manyhands/tickets/01BX5ZZKBKACTAV9WEVGEMMVRZ/ticket.md";
    let (root, data, service, operation, local_oid, incoming_oid) =
        resolution_fixture(&[(path, base, &local, &incoming)]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let attempt = OperationId::new();
    let request =
        || resolution_request_for_bodies(root.path(), operation, attempt, &inspection, &[&result]);
    let repository = git2::Repository::open(root.path()).unwrap();
    let token = &inspection.paths[0].token;
    let mut input = blake3::Hasher::new();
    input.update(b"manyhands-resolution-v1\0");
    input.update(&token.ordinal.to_be_bytes());
    input.update(&token.path);
    input.update(&conflict_token_digest(token));
    input.update(result.as_bytes());
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let owner = match service
        .reacquire_synchronization_conflict(
            root.path(),
            operation,
            &inspection.target.operation_target(&plan),
            0,
            inspection.observation.fingerprint,
        )
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        _ => panic!("reacquire"),
    };
    service
        .prepare_synchronization_resolution_attempt(
            root.path(),
            &owner,
            &state::ResolutionAttemptIntent {
                attempt_id: attempt,
                step_ordinal: 0,
                observation_digest: inspection.observation.fingerprint,
                input_digest: *input.finalize().as_bytes(),
                preflight_digest: resolution_preflight(&repository, &request().resolutions)
                    .unwrap(),
                identity_confirmation_id: None,
            },
            &[state::ResolutionPathIntent {
                ordinal: token.ordinal,
                path_digest: *blake3::hash(&token.path).as_bytes(),
                expected_digest: conflict_token_digest(token),
                result_digest: *blake3::hash(result.as_bytes()).as_bytes(),
                prewrite_digest: conflict_worktree_digest(root.path(), Path::new(path)).unwrap(),
                base_blob_oid: token.base,
                local_blob_oid: token.local,
                incoming_blob_oid: token.incoming,
                mode: 0o100644,
            }],
        )
        .unwrap();
    service
        .begin_synchronization_resolution_path_effects(root.path(), &owner, attempt)
        .unwrap();
    fs::write(root.path().join(path), &result).unwrap();
    service
        .observe_synchronization_resolution_path_effect(root.path(), &owner, attempt, token.ordinal)
        .unwrap();
    let mut index = repository.index().unwrap();
    for stage in 1..=3 {
        index.remove(Path::new(path), stage).unwrap();
    }
    let blob = repository.blob(result.as_bytes()).unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: result.len() as u32,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: token.path.clone(),
        })
        .unwrap();
    let tree = repository
        .find_tree(index.write_tree_to(&repository).unwrap())
        .unwrap();
    let signature = repository.signature().unwrap();
    let candidate = repository
        .commit(
            None,
            &signature,
            &signature,
            "legacy candidate",
            &tree,
            &[
                &repository.find_commit(local_oid).unwrap(),
                &repository.find_commit(incoming_oid).unwrap(),
            ],
        )
        .unwrap();
    service
        .prepare_synchronization_resolution_candidate(root.path(), &owner, attempt, candidate)
        .unwrap();
    let index_image = fs::read(repository.path().join("index")).unwrap();
    assert_eq!(
        service.resolve_synchronization(request()).unwrap(),
        ResolveSynchronizationOutcome::ValidationFailed
    );
    assert_eq!(repository.head().unwrap().target(), Some(local_oid));
    assert_eq!(
        fs::read(repository.path().join("index")).unwrap(),
        index_image
    );
    assert_eq!(fs::read(root.path().join(path)).unwrap(), result.as_bytes());
    assert!(!repository.path().join("index.lock").exists());
    assert_eq!(
        rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT count(*) FROM remote_resolution_index_artifacts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

// Locked public-API characterization only. None of these fixture transitions
// establishes process-death behavior or authorizes production reflog repair.
#[cfg(target_os = "linux")]
fn ref_effect_api_fixture() -> (tempfile::TempDir, git2::Repository, git2::Oid, git2::Oid) {
    let root = tempfile::tempdir().unwrap();
    let repository = git2::Repository::init(root.path()).unwrap();
    repository.set_head("refs/heads/main").unwrap();
    let mut config = repository.config().unwrap();
    config.set_str("user.name", "Config Identity").unwrap();
    config
        .set_str("user.email", "config@example.invalid")
        .unwrap();
    config.set_bool("core.logallrefupdates", true).unwrap();
    let signature = git2::Signature::new(
        "Frozen Identity",
        "frozen@example.invalid",
        &git2::Time::new(1234, 0),
    )
    .unwrap();
    let tree = repository
        .find_tree(repository.index().unwrap().write_tree().unwrap())
        .unwrap();
    let old = repository
        .commit(Some("HEAD"), &signature, &signature, "baseline", &tree, &[])
        .unwrap();
    let parent = repository.find_commit(old).unwrap();
    let candidate = repository
        .commit(None, &signature, &signature, "candidate", &tree, &[&parent])
        .unwrap();
    drop(parent);
    drop(tree);
    (root, repository, old, candidate)
}

#[cfg(target_os = "linux")]
fn ref_effect_entry(old: git2::Oid, new: git2::Oid) -> Vec<u8> {
    format!("{old} {new} Frozen Identity <frozen@example.invalid> 5678 +0000\towned effect\n")
        .into_bytes()
}

#[cfg(target_os = "linux")]
#[test]
fn stock_ref_effect_transaction_appends_both_logs_with_explicit_signature_and_absence() {
    for absent in [false, true] {
        let (_root, repository, old, candidate) = ref_effect_api_fixture();
        let logs = [
            repository.path().join("logs/refs/heads/main"),
            repository.path().join("logs/HEAD"),
        ];
        let baseline = logs
            .iter()
            .map(|log| {
                if absent {
                    fs::remove_file(log).unwrap();
                    Vec::new()
                } else {
                    fs::read(log).unwrap()
                }
            })
            .collect::<Vec<_>>();
        let signature = git2::Signature::new(
            "Frozen Identity",
            "frozen@example.invalid",
            &git2::Time::new(5678, 0),
        )
        .unwrap();
        let mut transaction = repository.transaction().unwrap();
        transaction.lock_ref("refs/heads/main").unwrap();
        transaction
            .set_target(
                "refs/heads/main",
                candidate,
                Some(&signature),
                "owned effect",
            )
            .unwrap();
        transaction.commit().unwrap();
        assert_eq!(repository.refname_to_id("HEAD").unwrap(), candidate);
        for (log, mut expected) in logs.iter().zip(baseline) {
            expected.extend(ref_effect_entry(old, candidate));
            assert_eq!(fs::read(log).unwrap(), expected);
        }
        // The same-value fast path does not append, but cannot finish a missing
        // HEAD effect: it returns before both logging and loose-ref installation.
        let before = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        let mut transaction = repository.transaction().unwrap();
        transaction.lock_ref("refs/heads/main").unwrap();
        transaction
            .set_target(
                "refs/heads/main",
                candidate,
                Some(&signature),
                "owned effect",
            )
            .unwrap();
        transaction.commit().unwrap();
        assert_eq!(
            logs.iter()
                .map(|log| fs::read(log).unwrap())
                .collect::<Vec<_>>(),
            before
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn stock_ref_effect_set_reflog_suppresses_head_and_replaces_branch_even_without_target() {
    for absent in [false, true] {
        let (_root, repository, old, candidate) = ref_effect_api_fixture();
        let branch_log = repository.path().join("logs/refs/heads/main");
        let head_log = repository.path().join("logs/HEAD");
        if absent {
            fs::remove_file(&branch_log).unwrap();
            fs::remove_file(&head_log).unwrap();
        }
        let before_head = fs::read(&head_log).ok();
        let before_branch = fs::read(&branch_log).ok();
        let log = repository.reflog("refs/heads/main").unwrap();
        // PUBLIC read creates an absent file; therefore it is not a read-only
        // actual-state probe. Never use this to infer baseline absence.
        assert_eq!(
            fs::read(&branch_log).unwrap(),
            before_branch.unwrap_or_default()
        );
        use std::os::unix::fs::MetadataExt;
        let anchor = repository.path().join("branch-log-anchor");
        fs::hard_link(&branch_log, &anchor).unwrap();
        let inode = fs::metadata(&anchor).unwrap().ino();
        let mut transaction = repository.transaction().unwrap();
        transaction.lock_ref("refs/heads/main").unwrap();
        transaction
            .set_target("refs/heads/main", candidate, None, "ignored message")
            .unwrap();
        transaction.set_reflog("refs/heads/main", log).unwrap();
        transaction.commit().unwrap();
        assert_eq!(repository.refname_to_id("HEAD").unwrap(), candidate);
        assert_eq!(fs::read(&head_log).ok(), before_head); // No implicit HEAD append.
        assert_eq!(fs::read(&branch_log).unwrap(), fs::read(&anchor).unwrap());
        assert_ne!(fs::metadata(&branch_log).unwrap().ino(), inode); // Full replacement, not no-op.
        assert_eq!(
            repository.reflog("refs/heads/main").unwrap().len(),
            usize::from(!absent)
        );
        // Log-only transactions can install the missing HEAD entry, but still
        // replace the full log rather than appending just the missing bytes.
        let signature = git2::Signature::new(
            "Frozen Identity",
            "frozen@example.invalid",
            &git2::Time::new(5678, 0),
        )
        .unwrap();
        let mut log = repository.reflog("HEAD").unwrap();
        log.append(candidate, &signature, Some("owned effect"))
            .unwrap();
        let mut transaction = repository.transaction().unwrap();
        transaction.lock_ref("HEAD").unwrap();
        transaction.set_reflog("HEAD", log).unwrap();
        transaction.commit().unwrap();
        let mut expected = before_head.unwrap_or_default();
        // In-memory append derives old OID from the last log entry, not the ref.
        expected.extend(ref_effect_entry(
            if absent { git2::Oid::zero() } else { old },
            candidate,
        ));
        assert_eq!(fs::read(&head_log).unwrap(), expected);
        assert_eq!(
            repository.find_reference("HEAD").unwrap().symbolic_target(),
            Some("refs/heads/main")
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn stock_ref_effect_reflog_write_normalizes_prior_bytes_and_discards_partial_lines() {
    let (_root, repository, old, _candidate) = ref_effect_api_fixture();
    let path = repository.path().join("logs/refs/heads/main");
    let normalized =
        format!("{old} {old} Frozen Identity <frozen@example.invalid> 5678 +0000\thistoric\n");
    let image = format!(
        "{old} {old} Frozen Identity <frozen@example.invalid> 5678 +0000\thistoric   \n{old} "
    );
    fs::write(&path, image.as_bytes()).unwrap();
    let mut log = repository.reflog("refs/heads/main").unwrap();
    assert_eq!(log.len(), 1); // Invalid partial line silently skipped.
    assert_eq!(fs::read(&path).unwrap(), image.as_bytes()); // Existing read alone preserves bytes.
    let lock = path.with_extension("lock");
    fs::write(&lock, b"foreign reflog lock").unwrap();
    // This locked version surfaces GenericError, not ErrorCode::Locked.
    assert_eq!(
        log.write().unwrap_err().code(),
        git2::ErrorCode::GenericError
    );
    assert_eq!(fs::read(&lock).unwrap(), b"foreign reflog lock");
    assert_eq!(fs::read(&path).unwrap(), image.as_bytes());
    // Disposable fixture simulates operator handling; this does NOT authorize
    // production repair of the remaining partial log image.
    fs::remove_file(&lock).unwrap();
    log.write().unwrap();
    assert_eq!(fs::read(&path).unwrap(), normalized.as_bytes());
    assert_eq!(repository.refname_to_id("HEAD").unwrap(), old);
}

#[cfg(target_os = "linux")]
#[test]
fn stock_ref_effect_linked_worktree_updates_common_branch_and_only_target_head_log() {
    let (root, repository, old, candidate) = ref_effect_api_fixture();
    let branch = repository
        .reference("refs/heads/linked", old, false, "linked baseline")
        .unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&branch));
    let linked_path = root.path().join("linked");
    repository
        .worktree("linked", &linked_path, Some(&options))
        .unwrap();
    let linked = git2::Repository::open(&linked_path).unwrap();
    let primary_head = fs::read(repository.path().join("logs/HEAD")).unwrap();
    let branch_log = repository.path().join("logs/refs/heads/linked");
    let target_head_log = linked.path().join("logs/HEAD");
    let mut expected_branch = fs::read(&branch_log).unwrap();
    assert!(!target_head_log.exists()); // Worktree creation does not seed its HEAD log.
    let mut expected_head = Vec::new();
    let signature = git2::Signature::new(
        "Frozen Identity",
        "frozen@example.invalid",
        &git2::Time::new(5678, 0),
    )
    .unwrap();
    let mut transaction = linked.transaction().unwrap();
    transaction.lock_ref("refs/heads/linked").unwrap();
    transaction
        .set_target(
            "refs/heads/linked",
            candidate,
            Some(&signature),
            "owned effect",
        )
        .unwrap();
    transaction.commit().unwrap();
    expected_branch.extend(ref_effect_entry(old, candidate));
    expected_head.extend(ref_effect_entry(old, candidate));
    assert_eq!(fs::read(&branch_log).unwrap(), expected_branch);
    assert_eq!(fs::read(&target_head_log).unwrap(), expected_head);
    assert_eq!(
        fs::read(repository.path().join("logs/HEAD")).unwrap(),
        primary_head
    );
    assert_eq!(repository.refname_to_id("HEAD").unwrap(), old);
    assert_eq!(linked.refname_to_id("HEAD").unwrap(), candidate);
}

#[cfg(target_os = "linux")]
#[test]
fn uncertain_ref_effect_matrix_preserves_log_images_after_operator_lock_handling() {
    // Explicitly simulated images, not process death inside libgit2. HEAD-only
    // is an adversarial state, not the normal branch-then-HEAD backend ordering.
    for absent in [false, true] {
        for image in [
            "none",
            "branch",
            "head",
            "both",
            "partial_branch",
            "partial_head",
            "changed",
            "third_ref",
        ] {
            let (root, data, service, request) = protocol_resolution_fixture();
            if absent {
                let repository = git2::Repository::open(root.path()).unwrap();
                for role in ["logs/refs/heads/main", "logs/HEAD"] {
                    fs::remove_file(repository.path().join(role)).unwrap();
                }
            }
            set_resolution_index_install_hook(root.path().to_owned(), || {
                panic!("fixture stop before ref invocation")
            });
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || service.resolve_synchronization(request.clone())
                ))
                .is_err()
            );
            let repository = git2::Repository::open(root.path()).unwrap();
            let old = repository.refname_to_id("HEAD").unwrap();
            let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            connection
                .execute_batch("UPDATE remote_resolution_index_artifacts SET ref_phase='intent';")
                .unwrap();
            let candidate: String = connection
                .query_row(
                    "SELECT candidate_oid FROM remote_resolution_attempts",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let candidate_oid = git2::Oid::from_str(&candidate).unwrap();
            let logs = [
                repository.path().join("logs/refs/heads/main"),
                repository.path().join("logs/HEAD"),
            ];
            for (index, log) in logs.iter().enumerate() {
                let mut bytes = if absent {
                    Vec::new()
                } else {
                    fs::read(log).unwrap()
                };
                match (image, index) {
                    ("branch", 0) | ("head", 1) | ("both", _) | ("partial_head", 0) => {
                        bytes.extend(ref_effect_entry(old, candidate_oid))
                    }
                    ("partial_branch", 0) | ("partial_head", 1) => {
                        bytes.extend(&ref_effect_entry(old, candidate_oid)[..45])
                    }
                    ("changed", _) => bytes = b"foreign changed log image\n".to_vec(),
                    _ => {}
                }
                if absent && bytes.is_empty() {
                    assert!(!log.exists());
                } else {
                    fs::write(log, bytes).unwrap();
                }
            }
            if image == "third_ref" {
                // Incoming is a real third OID, but never an authorized candidate.
                let third = repository
                    .find_commit(candidate_oid)
                    .unwrap()
                    .parent_id(1)
                    .unwrap();
                assert_ne!(third, old);
                assert_ne!(third, candidate_oid);
                fs::write(
                    repository.path().join("refs/heads/main"),
                    format!("{third}\n"),
                )
                .unwrap();
            }
            let before_head = repository.refname_to_id("HEAD").unwrap();
            let before_logs = logs
                .iter()
                .map(|log| fs::read(log).ok())
                .collect::<Vec<_>>();
            let before_index = fs::read(repository.path().join("index")).unwrap();
            let backend_lock = repository.path().join("refs/heads/main.lock");
            // Matching candidate bytes are still not backend lock provenance.
            let lock_bytes = format!("{candidate}\n");
            fs::write(&backend_lock, &lock_bytes).unwrap();
            let restarted = RepositoryService::open_at(data.path()).unwrap();
            assert!(matches!(
                restarted.resolve_synchronization(request.clone()),
                Err(SynchronizationError::RecoveryRequired)
            ));
            assert_eq!(fs::read(&backend_lock).unwrap(), lock_bytes.as_bytes());
            fs::remove_file(&backend_lock).unwrap(); // Quiesced fixture operator only.
            let result = restarted.resolve_synchronization(request);
            if image == "none" {
                assert!(
                    matches!(result.unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate_oid)
                );
                assert!(!repository.path().join("index.lock").exists());
                continue;
            }
            if image == "third_ref" {
                assert!(matches!(result, Err(SynchronizationError::ExternalChange)));
            } else {
                assert!(matches!(
                    result,
                    Err(SynchronizationError::RecoveryRequired)
                ));
            }
            assert_eq!(
                logs.iter()
                    .map(|log| fs::read(log).ok())
                    .collect::<Vec<_>>(),
                before_logs
            );
            assert_eq!(repository.refname_to_id("HEAD").unwrap(), before_head);
            assert_eq!(
                fs::read(repository.path().join("index")).unwrap(),
                before_index
            );
            assert!(repository.path().join("index.lock").exists());
            assert_eq!(
                connection
                    .query_row(
                        "SELECT candidate_oid FROM remote_resolution_attempts",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                candidate
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn candidate_ref_effect_before_checkpoint_observation_replays_without_another_append() {
    let (root, data, service, request) = protocol_resolution_fixture();
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let repository = git2::Repository::open(root.path()).unwrap();
    let candidate = repository.refname_to_id("HEAD").unwrap();
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT phase FROM remote_resolution_attempts", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
        "candidate_prepared"
    );
    let logs = [
        repository.path().join("logs/refs/heads/main"),
        repository.path().join("logs/HEAD"),
    ];
    let before = logs
        .iter()
        .map(|log| fs::read(log).unwrap())
        .collect::<Vec<_>>();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        matches!(restarted.resolve_synchronization(request).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
    );
    assert_eq!(
        logs.iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>(),
        before
    );
    // Actual successful backend return followed by an injected service failure;
    // not death in the backend, nor proof of candidate-HEAD log authentication.
}

#[cfg(target_os = "linux")]
#[test]
fn stock_ref_effect_head_append_failure_leaves_branch_append_and_replay_duplicates_it() {
    let (_root, repository, old, candidate) = ref_effect_api_fixture();
    let branch_log = repository.path().join("logs/refs/heads/main");
    let head_log = repository.path().join("logs/HEAD");
    let baseline_branch = fs::read(&branch_log).unwrap();
    let baseline_head = fs::read(&head_log).unwrap();
    // A real PUBLIC backend error, not a simulated successful append or death.
    // A nonempty directory makes HEAD appending fail after branch appending.
    fs::remove_file(&head_log).unwrap();
    fs::create_dir(&head_log).unwrap();
    let foreign = head_log.join("foreign-canary");
    fs::write(&foreign, b"preserve foreign obstruction").unwrap();
    let signature = git2::Signature::new(
        "Frozen Identity",
        "frozen@example.invalid",
        &git2::Time::new(5678, 0),
    )
    .unwrap();
    let apply = || {
        let mut transaction = repository.transaction().unwrap();
        transaction.lock_ref("refs/heads/main").unwrap();
        transaction
            .set_target(
                "refs/heads/main",
                candidate,
                Some(&signature),
                "owned effect",
            )
            .unwrap();
        transaction.commit()
    };
    assert!(apply().is_err());
    assert_eq!(repository.refname_to_id("HEAD").unwrap(), old);
    let mut branch_once = baseline_branch;
    branch_once.extend(ref_effect_entry(old, candidate));
    assert_eq!(fs::read(&branch_log).unwrap(), branch_once);
    assert_eq!(fs::read(&foreign).unwrap(), b"preserve foreign obstruction");
    // This is fixture restoration ONLY, not operator permission for production
    // reflog effects. The second stock invocation demonstrates duplicate append.
    fs::remove_file(foreign).unwrap();
    fs::remove_dir(&head_log).unwrap();
    fs::write(&head_log, &baseline_head).unwrap();
    apply().unwrap();
    branch_once.extend(ref_effect_entry(old, candidate));
    assert_eq!(fs::read(&branch_log).unwrap(), branch_once);
    let mut head_once = baseline_head;
    head_once.extend(ref_effect_entry(old, candidate));
    assert_eq!(fs::read(&head_log).unwrap(), head_once);
    assert_eq!(repository.refname_to_id("HEAD").unwrap(), candidate);
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_no_effect_intent_restarts_with_original_images() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let database = data.path().join(REGISTRY_FILE);
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        rusqlite::Connection::open(database)
            .unwrap()
            .execute_batch("UPDATE remote_resolution_index_artifacts SET ref_phase='intent';")
            .unwrap();
        panic!("fixture stop after intent, before backend");
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || service.resolve_synchronization(request.clone())
        ))
        .is_err()
    );
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_candidate_requires_exact_logs_and_operator_restoration() {
    let (root, data, service, request) = protocol_resolution_fixture();
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let repository = git2::Repository::open(root.path()).unwrap();
    let candidate = repository.head().unwrap().target().unwrap();
    let path = repository.path().join("logs/HEAD");
    let intended = fs::read(&path).unwrap();
    let partial = intended[..intended.len() - 5].to_vec();
    fs::write(&path, &partial).unwrap();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(fs::read(&path).unwrap(), partial);
    assert!(repository.path().join("index.lock").exists());
    // Disposable fixture operator restores complete intended backend metadata.
    fs::write(&path, &intended).unwrap();
    assert!(
        matches!(restarted.resolve_synchronization(request).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
    );
    assert_eq!(fs::read(&path).unwrap(), intended);
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_static_unsafe_metadata_and_policy_refuse_before_canonical_writes() {
    for adverse in [
        "symlink",
        "executable",
        "directory",
        "ancestor",
        "fifo",
        "ref_symlink",
        "policy",
    ] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        let canonical = root.path().join("docs/document.md");
        let original = fs::read(&canonical).unwrap();
        let log = repository.path().join("logs/HEAD");
        match adverse {
            "symlink" => {
                fs::remove_file(&log).unwrap();
                std::os::unix::fs::symlink(&canonical, &log).unwrap();
            }
            "executable" => {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&log, fs::Permissions::from_mode(0o700)).unwrap();
            }
            "directory" => {
                fs::remove_file(&log).unwrap();
                fs::create_dir(&log).unwrap();
            }
            "ancestor" => {
                let logs = repository.path().join("logs");
                let saved = repository.path().join("saved-logs");
                fs::rename(&logs, &saved).unwrap();
                std::os::unix::fs::symlink(saved, logs).unwrap();
            }
            "fifo" => {
                fs::remove_file(&log).unwrap();
                let path = CString::new(log.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "ref_symlink" => {
                let reference = repository.path().join("refs/heads/main");
                let saved = repository.path().join("saved-main-ref");
                fs::rename(&reference, &saved).unwrap();
                std::os::unix::fs::symlink(saved, reference).unwrap();
            }
            _ => repository
                .config()
                .unwrap()
                .set_str("core.logallrefupdates", "unsupported-policy")
                .unwrap(),
        }
        assert!(service.resolve_synchronization(request).is_err());
        assert_eq!(fs::read(&canonical).unwrap(), original);
        assert_eq!(
            rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM remote_resolution_attempts",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}

#[cfg(target_os = "linux")]
fn frozen_resolution_log_entry(
    repository: &git2::Repository,
    old: git2::Oid,
    candidate: git2::Oid,
) -> Vec<u8> {
    let commit = repository.find_commit(candidate).unwrap();
    let signer = commit.committer();
    format!(
        "{old} {candidate} {} <{}> 0 +0000\tmanyhands resolution\n",
        signer.name().unwrap(),
        signer.email().unwrap()
    )
    .into_bytes()
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_policy_opaque_history_and_frozen_signer_are_private_and_exact() {
    for policy in [None, Some("false"), Some("true"), Some("always")] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        let mut config = repository.config().unwrap();
        match policy {
            Some(value) => config.set_str("core.logallrefupdates", value).unwrap(),
            None => {
                let _ = config.remove("core.logallrefupdates");
            }
        }
        const SIGNER: &str = "PRIVATE-REF-SIGNER-CANARY";
        const EMAIL: &str = "PRIVATE-REF-EMAIL-CANARY@example.invalid";
        const HISTORY: &[u8] = b"PRIVATE-REF-HISTORY-CANARY\xff opaque spaces  \nunterminated";
        config.set_str("user.name", SIGNER).unwrap();
        config.set_str("user.email", EMAIL).unwrap();
        let logs = [
            repository.path().join("logs/refs/heads/main"),
            repository.path().join("logs/HEAD"),
        ];
        for log in &logs {
            fs::write(log, HISTORY).unwrap();
        }
        let old = repository.head().unwrap().target().unwrap();
        *service.failure_point.lock().unwrap() =
            Some(FailurePoint::ResolutionAfterCandidatePrepared);
        assert!(service.resolve_synchronization(request.clone()).is_err());
        let database = data.path().join(REGISTRY_FILE);
        let connection = rusqlite::Connection::open(&database).unwrap();
        let candidate: String = connection
            .query_row(
                "SELECT candidate_oid FROM remote_resolution_attempts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let candidate = git2::Oid::from_str(&candidate).unwrap();
        let commit = repository.find_commit(candidate).unwrap();
        assert!(commit.committer().name() == Some(SIGNER));
        assert_eq!(commit.committer().when().seconds(), 0);
        // A changed configured identity cannot regenerate the candidate or signer.
        config.set_str("user.name", "Changed Identity").unwrap();
        config
            .set_str("user.email", "changed@example.invalid")
            .unwrap();
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        assert!(
            matches!(restarted.resolve_synchronization(request.clone()).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
        );
        let mut expected = HISTORY.to_vec();
        if policy != Some("false") {
            expected.extend(frozen_resolution_log_entry(&repository, old, candidate));
        }
        for log in &logs {
            assert!(fs::read(log).unwrap() == expected);
        }
        // Completed replay rechecks both exact images and never appends again.
        assert!(
            matches!(restarted.resolve_synchronization(request.clone()).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
        );
        for log in &logs {
            assert!(fs::read(log).unwrap() == expected);
        }
        let backup = data.path().join("ref-evidence-backup.sqlite");
        connection
            .execute("VACUUM INTO ?1", [backup.to_str().unwrap()])
            .unwrap();
        let staging = repository
            .path()
            .join(format!(".manyhands-resolution-{}", request.attempt_id));
        let mut private_files = vec![
            database,
            data.path().join(format!("{REGISTRY_FILE}-wal")),
            backup,
        ];
        for role in ["baseline", "transition"] {
            private_files.push(staging.join(format!("ref-log-{role}")));
            private_files.push(staging.join(format!("ref-log-{role}-anchor")));
        }
        for path in private_files {
            if let Ok(bytes) = fs::read(path) {
                for canary in [
                    SIGNER.as_bytes(),
                    EMAIL.as_bytes(),
                    b"PRIVATE-REF-HISTORY-CANARY",
                ] {
                    assert!(
                        !bytes.windows(canary.len()).any(|window| window == canary),
                        "private ref evidence leaked raw authority"
                    );
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_old_ref_effects_preserve_then_accept_operator_restored_original_images() {
    for shape in [
        "branch",
        "head",
        "both",
        "partial_branch",
        "partial_head",
        "foreign",
    ] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        let logs = [
            repository.path().join("logs/refs/heads/main"),
            repository.path().join("logs/HEAD"),
        ];
        let originals = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        let old = repository.head().unwrap().target().unwrap();
        let database = data.path().join(REGISTRY_FILE);
        set_resolution_index_install_hook(root.path().to_owned(), move || {
            rusqlite::Connection::open(database)
                .unwrap()
                .execute_batch("UPDATE remote_resolution_index_artifacts SET ref_phase='intent';")
                .unwrap();
            panic!("simulated old-ref backend interruption");
        });
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || service.resolve_synchronization(request.clone())
            ))
            .is_err()
        );
        let candidate: String = rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .query_row(
                "SELECT candidate_oid FROM remote_resolution_attempts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let candidate = git2::Oid::from_str(&candidate).unwrap();
        let entry = frozen_resolution_log_entry(&repository, old, candidate);
        for (index, log) in logs.iter().enumerate() {
            let mut image = originals[index].clone();
            match (shape, index) {
                ("branch", 0) | ("head", 1) | ("both", _) | ("partial_head", 0) => {
                    image.extend(&entry)
                }
                ("partial_branch", 0) | ("partial_head", 1) => image.extend(&entry[..45]),
                ("foreign", _) => image = b"foreign backend metadata".to_vec(),
                _ => {}
            }
            fs::write(log, image).unwrap();
        }
        let interrupted = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        assert!(matches!(
            restarted.resolve_synchronization(request.clone()),
            Err(SynchronizationError::RecoveryRequired)
        ));
        assert_eq!(repository.head().unwrap().target(), Some(old));
        assert_eq!(
            logs.iter()
                .map(|log| fs::read(log).unwrap())
                .collect::<Vec<_>>(),
            interrupted
        );
        // Operator restores both original images, not merely an absent lock.
        for (log, image) in logs.iter().zip(&originals) {
            fs::write(log, image).unwrap();
        }
        let mut altered = request.clone();
        altered.resolutions[0].1 = RedactedConflictBytes::from_bytes(b"altered same ID".to_vec());
        assert!(restarted.resolve_synchronization(altered).is_err());
        assert!(
            matches!(restarted.resolve_synchronization(request).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
        );
        for (log, mut expected) in logs.iter().zip(originals) {
            expected.extend(&entry);
            assert_eq!(fs::read(log).unwrap(), expected);
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_unproved_candidate_states_and_policy_changes_preserve_effects() {
    for shape in [
        "original",
        "branch_only",
        "head_only",
        "partial_branch",
        "partial_head",
        "foreign",
        "policy",
    ] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        let logs = [
            repository.path().join("logs/refs/heads/main"),
            repository.path().join("logs/HEAD"),
        ];
        let originals = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        let original_policy = repository
            .config()
            .unwrap()
            .get_string("core.logallrefupdates")
            .ok();
        *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
        assert!(service.resolve_synchronization(request.clone()).is_err());
        let candidate = repository.head().unwrap().target().unwrap();
        let intended = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        for (index, log) in logs.iter().enumerate() {
            let image = match (shape, index) {
                ("original", _) | ("branch_only", 1) | ("head_only", 0) => originals[index].clone(),
                ("partial_branch", 0) | ("partial_head", 1) => {
                    intended[index][..intended[index].len() - 5].to_vec()
                }
                ("foreign", _) => b"foreign candidate log".to_vec(),
                _ => intended[index].clone(),
            };
            fs::write(log, image).unwrap();
        }
        if shape == "policy" {
            repository
                .config()
                .unwrap()
                .set_bool("core.logallrefupdates", false)
                .unwrap();
        }
        let interrupted = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        assert!(matches!(
            restarted.resolve_synchronization(request.clone()),
            Err(SynchronizationError::RecoveryRequired)
        ));
        assert_eq!(repository.head().unwrap().target(), Some(candidate));
        assert!(repository.path().join("index.lock").exists());
        assert_eq!(
            logs.iter()
                .map(|log| fs::read(log).unwrap())
                .collect::<Vec<_>>(),
            interrupted
        );
        // Fixture operator restores the COMPLETE candidate images and policy.
        if shape == "policy" {
            let mut config = repository.config().unwrap();
            if let Some(policy) = original_policy {
                config.set_str("core.logallrefupdates", &policy).unwrap();
            } else {
                config.remove("core.logallrefupdates").unwrap();
            }
        }
        for (log, image) in logs.iter().zip(&intended) {
            fs::write(log, image).unwrap();
        }
        assert!(
            matches!(restarted.resolve_synchronization(request).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
        );
        assert_eq!(
            logs.iter()
                .map(|log| fs::read(log).unwrap())
                .collect::<Vec<_>>(),
            intended
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_missing_partial_tampered_or_substituted_evidence_never_authenticates() {
    for corruption in [
        "missing_baseline",
        "missing_transition",
        "partial_manifest",
        "same_image_substitution",
        "digest",
        "symlink",
        "missing_anchor",
    ] {
        let (root, data, service, request) = protocol_resolution_fixture();
        *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
        assert!(service.resolve_synchronization(request.clone()).is_err());
        let repository = git2::Repository::open(root.path()).unwrap();
        let candidate = repository.head().unwrap().target();
        let staging = repository
            .path()
            .join(format!(".manyhands-resolution-{}", request.attempt_id));
        let manifest = staging.join("ref-log-transition");
        let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        match corruption {
            "missing_baseline" => {
                connection
                    .execute(
                        "DELETE FROM remote_resolution_ref_log_artifacts WHERE role='baseline'",
                        [],
                    )
                    .unwrap();
            }
            "missing_transition" => {
                connection
                    .execute(
                        "DELETE FROM remote_resolution_ref_log_artifacts WHERE role='transition'",
                        [],
                    )
                    .unwrap();
            }
            "partial_manifest" => fs::write(&manifest, b"version: 1\npartial:").unwrap(),
            "same_image_substitution" => {
                let bytes = fs::read(&manifest).unwrap();
                fs::remove_file(&manifest).unwrap();
                fs::write(&manifest, bytes).unwrap();
            }
            "digest" => {
                connection.execute_batch("DROP TRIGGER remote_resolution_ref_log_artifact_immutable; UPDATE remote_resolution_ref_log_artifacts SET digest=zeroblob(32); CREATE TRIGGER remote_resolution_ref_log_artifact_immutable BEFORE UPDATE ON remote_resolution_ref_log_artifacts BEGIN SELECT RAISE(ABORT,'immutable ref log evidence'); END;").unwrap();
            }
            "symlink" => {
                fs::remove_file(&manifest).unwrap();
                std::os::unix::fs::symlink(staging.join("ref-log-baseline"), &manifest).unwrap();
            }
            _ => fs::remove_file(staging.join("ref-log-transition-anchor")).unwrap(),
        }
        let logs = [
            repository.path().join("logs/refs/heads/main"),
            repository.path().join("logs/HEAD"),
        ];
        let before = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        if let Ok(restarted) = RepositoryService::open_at(data.path()) {
            assert!(restarted.resolve_synchronization(request).is_err());
        }
        assert_eq!(repository.head().unwrap().target(), candidate);
        assert_eq!(
            logs.iter()
                .map(|log| fs::read(log).unwrap())
                .collect::<Vec<_>>(),
            before
        );
        assert!(repository.path().join("index.lock").exists());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_manifest_observation_failure_recovers_only_private_preparation() {
    for role in ["baseline", "transition"] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let database = data.path().join(REGISTRY_FILE);
        let hook_database = database.clone();
        set_resolution_index_lock_hook(root.path().to_owned(), move || {
            let sql = format!(
                "CREATE TRIGGER fail_ref_manifest BEFORE INSERT ON remote_resolution_ref_log_artifacts WHEN NEW.role='{role}' BEGIN SELECT RAISE(ABORT,'test manifest observation fault'); END;"
            );
            rusqlite::Connection::open(hook_database)
                .unwrap()
                .execute_batch(&sql)
                .unwrap();
        });
        let repository = git2::Repository::open(root.path()).unwrap();
        let old = repository.head().unwrap().target();
        let logs = [
            repository.path().join("logs/refs/heads/main"),
            repository.path().join("logs/HEAD"),
        ];
        let before = logs
            .iter()
            .map(|log| fs::read(log).unwrap())
            .collect::<Vec<_>>();
        assert!(service.resolve_synchronization(request.clone()).is_err());
        assert_eq!(repository.head().unwrap().target(), old);
        assert_eq!(
            logs.iter()
                .map(|log| fs::read(log).unwrap())
                .collect::<Vec<_>>(),
            before
        );
        let connection = rusqlite::Connection::open(database).unwrap();
        connection
            .execute_batch("DROP TRIGGER fail_ref_manifest;")
            .unwrap();
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        assert!(matches!(
            restarted.resolve_synchronization(request).unwrap(),
            ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
        ));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_legacy_missing_all_evidence_refuses_even_old_no_effect_state() {
    let (root, data, service, request) = protocol_resolution_fixture();
    set_resolution_index_install_hook(root.path().to_owned(), || {
        panic!("fixture old HEAD with candidate")
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || service.resolve_synchronization(request.clone())
        ))
        .is_err()
    );
    let repository = git2::Repository::open(root.path()).unwrap();
    let old = repository.head().unwrap().target();
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection
        .execute("DELETE FROM remote_resolution_ref_log_artifacts", [])
        .unwrap();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(repository.head().unwrap().target(), old);
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM remote_resolution_ref_log_artifacts",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_preserves_every_relevant_foreign_backend_lock_before_writes() {
    for role in [
        "HEAD.lock",
        "logs/HEAD.lock",
        "packed-refs.lock",
        "refs/heads/main.lock",
        "logs/refs/heads/main.lock",
    ] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        let canonical = root.path().join("docs/document.md");
        let before = fs::read(&canonical).unwrap();
        let path = repository.path().join(role);
        fs::write(&path, b"foreign backend lock identity not inferred").unwrap();
        assert!(matches!(
            service.resolve_synchronization(request),
            Err(SynchronizationError::RecoveryRequired)
        ));
        assert_eq!(
            fs::read(path).unwrap(),
            b"foreign backend lock identity not inferred"
        );
        assert_eq!(fs::read(canonical).unwrap(), before);
        assert_eq!(
            rusqlite::Connection::open(data.path().join(REGISTRY_FILE))
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM remote_resolution_attempts",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_snapshot_uses_common_branch_and_target_worktree_head_roles() {
    let (root, repository, old, candidate) = ref_effect_api_fixture();
    let reference = repository
        .reference("refs/heads/log-proof-linked", old, false, "fixture")
        .unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    let workdir = root.path().join("log-proof-linked");
    repository
        .worktree("log-proof-linked", &workdir, Some(&options))
        .unwrap();
    let target = git2::Repository::open(workdir).unwrap();
    let primary_head = fs::read(repository.path().join("logs/HEAD")).unwrap();
    let snapshot = RefLogSnapshot::read(&target).unwrap();
    assert!(snapshot.logs[0].image.present);
    assert!(!snapshot.logs[1].image.present);
    snapshot.revalidate(&target).unwrap();
    let signature = git2::Signature::new(
        "Frozen Identity",
        "frozen@example.invalid",
        &git2::Time::new(5678, 0),
    )
    .unwrap();
    let entry = ref_effect_entry(old, candidate);
    let intended = snapshot
        .logs
        .each_ref()
        .map(|log| log.intended(Some(&entry)));
    let mut transaction = target.transaction().unwrap();
    transaction.lock_ref("refs/heads/log-proof-linked").unwrap();
    transaction
        .set_target(
            "refs/heads/log-proof-linked",
            candidate,
            Some(&signature),
            "owned effect",
        )
        .unwrap();
    transaction.commit().unwrap();
    let result = RefLogSnapshot::read(&target).unwrap();
    result.revalidate(&target).unwrap();
    assert!(result.matches(&intended));
    assert_eq!(
        fs::read(repository.path().join("logs/HEAD")).unwrap(),
        primary_head
    );
    assert_eq!(repository.head().unwrap().target(), Some(old));
    // Observer/role fidelity only, not full public context resolution/death proof.
}

#[cfg(target_os = "linux")]
#[test]
fn ref_log_proof_old_operator_restoration_after_failed_checkpoint_keeps_progress_monotonic() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let old = repository.head().unwrap().target().unwrap();
    let logs = [
        repository.path().join("logs/refs/heads/main"),
        repository.path().join("logs/HEAD"),
    ];
    let original = logs
        .iter()
        .map(|log| fs::read(log).unwrap())
        .collect::<Vec<_>>();
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_checkpoint_observation BEFORE UPDATE OF phase ON remote_resolution_attempts WHEN NEW.phase='applied' BEGIN SELECT RAISE(ABORT,'test checkpoint observation fault'); END;").unwrap();
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let candidate = repository.head().unwrap().target().unwrap();
    assert_ne!(candidate, old);
    assert_eq!(
        connection
            .query_row(
                "SELECT ref_phase FROM remote_resolution_index_artifacts",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "observed"
    );
    connection
        .execute_batch("DROP TRIGGER fail_checkpoint_observation;")
        .unwrap();
    // Fixture operator restores the original old-ref/log images after the failed
    // checkpoint observation. Ref progress must not rewind or regenerate proof.
    fs::write(
        repository.path().join("refs/heads/main"),
        format!("{old}\n"),
    )
    .unwrap();
    for (log, image) in logs.iter().zip(&original) {
        fs::write(log, image).unwrap();
    }
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        matches!(restarted.resolve_synchronization(request).unwrap(),ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid==candidate)
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT ref_phase FROM remote_resolution_index_artifacts",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "observed"
    );
}
