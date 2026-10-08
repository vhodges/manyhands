use super::*;
#[cfg(unix)]
use crate::repository::OwnedPathBoundary;
use crate::repository::remote::reservation::commit_observation_batch;
use crate::repository::{EnableRepositoryRequest, FailurePoint, REGISTRY_FILE};
#[cfg(target_os = "linux")]
use sha1::{Digest, Sha1};
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
    {
        // Simulate process death after fully initialized acquisition but before
        // persistence: forget closes neither descriptor nor pathname locally,
        // while the fresh service below must recognize and retire its image.
        let repository = git2::Repository::open(root.path()).unwrap();
        let lock = ResolutionIndexLock::acquire(&repository).unwrap();
        std::mem::forget(lock);
        assert!(repository.path().join("index.lock").exists());
    }
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
        "persisted exchange sentinel survives the injected ref-transition failure"
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
        !git2::Repository::open(root.path())
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
fn scratch_pathname_replacement_before_descriptor_parse_is_ignored() {
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
        // This runs after memfd serialization and immediately before libgit2
        // parses it. A persistent filesystem scratch-name replacement cannot
        // affect descriptor-bound serialization or the lock copy.
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
    let checksum = Sha1::digest(&index);
    index.extend_from_slice(&checksum);
    index
}

#[cfg(target_os = "linux")]
#[test]
fn semantic_index_extensions_are_rejected_before_index_lock_creation() {
    for extension in [b"REUC", b"NAME", b"link", b"abcd", b"ABCD"] {
        let (root, _data, _service) = fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        let index_path = repository.path().join("index");
        let extension_index = index_with_extension(fs::read(&index_path).unwrap(), extension);
        fs::write(&index_path, &extension_index).unwrap();
        assert!(matches!(
            ResolutionIndexLock::acquire(&repository),
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
fn substituted_lock_between_retire_precheck_and_exchange_is_preserved() {
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
fn substituted_lock_after_exchange_is_retained_without_cleanup() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let local = base.replace("base", "local");
    let remote = base.replace("base", "remote");
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("docs/document.md", base, local.as_str(), remote.as_str())]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let external_lock = b"external lock after index exchange\n".to_vec();
    let gitdir = repository.path().to_owned();
    let hook_lock = external_lock.clone();
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        let replacement = gitdir.join("external-post-exchange-lock");
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
