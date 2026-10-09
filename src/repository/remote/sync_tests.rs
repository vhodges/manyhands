use super::*;
#[cfg(unix)]
use crate::repository::OwnedPathBoundary;
use crate::repository::remote::reservation::commit_observation_batch;
use crate::repository::{EnableRepositoryRequest, FailurePoint, REGISTRY_FILE};
use std::fs;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
struct NoPrompt;
impl SessionCredentialProvider for NoPrompt {
    fn request_passphrase(
        &mut self,
        _: &crate::repository::keys::UnlockRequest,
    ) -> crate::repository::keys::PassphraseResponse {
        panic!("unexpected prompt")
    }
}
fn fixture_tempdir() -> tempfile::TempDir {
    fixture_tempdir_in(&std::env::temp_dir())
}

fn fixture_tempdir_in(parent: &Path) -> tempfile::TempDir {
    // macOS temporary parents may be spelled /var rather than /private/var.
    // Canonicalize only fixture setup, keeping roots and lexical hook keys aligned
    // with libgit2 without relaxing production descriptor no-follow traversal.
    #[cfg(unix)]
    let parent = parent.canonicalize().unwrap();
    tempfile::tempdir_in(parent).unwrap()
}

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    fixture_in(&std::env::temp_dir())
}

fn fixture_in(parent: &Path) -> (tempfile::TempDir, tempfile::TempDir, RepositoryService) {
    let root = fixture_tempdir_in(parent);
    let data = fixture_tempdir_in(parent);
    let repo = git2::Repository::init(root.path()).unwrap();
    // Pin byte-exact fixtures before any staging. Production checkout continues
    // to honor Git filters (characterized separately with local autocrlf=true).
    // CI proved CRLF checkout, but did not log inherited config provenance.
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
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
    applying_candidate_restart_fixture(true);
}

#[test]
fn candidate_old_head_restart_preserves_applying_effect_recovery() {
    applying_candidate_restart_fixture(false);
}

fn applying_candidate_restart_fixture(already_at_candidate: bool) {
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
    if already_at_candidate {
        fast_forward(&repo, "refs/heads/main", local, candidate).unwrap();
    }
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
    if already_at_candidate {
        assert_eq!(repo.head().unwrap().target(), Some(head));
        assert_eq!(fs::read(repo.path().join("index")).unwrap(), index);
    } else {
        assert_eq!(head, local);
        assert_eq!(repo.head().unwrap().target(), Some(candidate));
    }
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
fn fixture_pins_autocrlf_locally_before_initial_staging() {
    let (root, _data, _service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let local = repo
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    assert_eq!(local.get_bool("core.autocrlf").ok(), Some(false));
    let tree = repo.head().unwrap().peel_to_tree().unwrap();
    let blob = repo
        .find_blob(tree.get_name("fixture.txt").unwrap().id())
        .unwrap();
    assert!(blob.content() == b"original\n");
}

#[test]
fn fast_forward_respects_local_autocrlf_checkout_policy() {
    let (root, _data, _service) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", true)
        .unwrap();
    let old = repo.head().unwrap().target().unwrap();
    let new = child(&repo, old, b"advanced\n");
    fast_forward(&repo, "refs/heads/main", old, new).unwrap();
    assert_eq!(repo.head().unwrap().target(), Some(new));
    assert!(fs::read(root.path().join("fixture.txt")).unwrap() == b"advanced\r\n");
    local_target(root.path(), "main", &SynchronizationTarget::Primary).unwrap();
}

#[test]
fn collected_native_document_and_ticket_paths_are_canonical() {
    let root = fixture_tempdir();
    let document = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbody\n";
    let ticket = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Ticket\"\ntype: \"task\"\nstatus: \"open\"\n---\n\nbody\n";
    fs::create_dir_all(root.path().join("docs/nested")).unwrap();
    fs::create_dir_all(
        root.path()
            .join(".manyhands/tickets/01BX5ZZKBKACTAV9WEVGEMMVRZ"),
    )
    .unwrap();
    fs::write(root.path().join("docs/nested/document.md"), document).unwrap();
    fs::write(
        root.path()
            .join(".manyhands/tickets/01BX5ZZKBKACTAV9WEVGEMMVRZ/ticket.md"),
        ticket,
    )
    .unwrap();
    let mut sources = Vec::new();
    crate::repository::collect_canonical_sources(
        root.path(),
        &mut sources,
        RepositoryOperation::RepositorySnapshot,
    )
    .unwrap();
    assert_eq!(sources.len(), 2);
    assert!(
        sources
            .iter()
            .all(|(path, _)| !path.to_str().unwrap().contains('\\'))
    );
    let validated = canonical::validate_context(sources);
    assert_eq!(validated.items.len(), 2);
    assert!(validated.problems.is_empty());
}

#[cfg(unix)]
#[test]
fn collected_literal_backslash_filename_stays_invalid() {
    let root = fixture_tempdir();
    fs::create_dir(root.path().join("docs")).unwrap();
    fs::write(root.path().join("docs/literal\\name.md"), "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbody\n").unwrap();
    let mut sources = Vec::new();
    crate::repository::collect_canonical_sources(
        root.path(),
        &mut sources,
        RepositoryOperation::RepositorySnapshot,
    )
    .unwrap();
    assert_eq!(sources.len(), 1);
    let validated = canonical::validate_context(sources);
    assert!(validated.items.is_empty());
    assert!(
        validated
            .problems
            .iter()
            .any(|problem| problem.code == canonical::ValidationCode::InvalidPath)
    );
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
            let foreign = fixture_tempdir();
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

#[test]
fn review_resolution_preserves_unrelated_existing_context_diagnostics() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let duplicate = base.replace("01ARZ3NDEKTSV4RRFFQ69G5FAV", "01BX5ZZKBKACTAV9WEVGEMMVRZ");
    let orphan = "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"01CRZ3NDEKTSV4RRFFQ69G5FAV\"\nitem_id: \"01DRZ3NDEKTSV4RRFFQ69G5FAV\"\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n\nExisting orphan\n";
    let orphan_path =
        ".manyhands/comments/01DRZ3NDEKTSV4RRFFQ69G5FAV/01CRZ3NDEKTSV4RRFFQ69G5FAV.md";
    let files = [
        (
            "docs/document.md",
            base,
            base.replace("base", "local"),
            base.replace("base", "incoming"),
        ),
        (
            "docs/duplicate-one.md",
            duplicate.as_str(),
            duplicate.clone(),
            duplicate.clone(),
        ),
        (
            "docs/duplicate-two.md",
            duplicate.as_str(),
            duplicate.clone(),
            duplicate.clone(),
        ),
        (orphan_path, orphan, orphan.into(), orphan.into()),
    ];
    let files = files
        .iter()
        .map(|(path, base, local, incoming)| (*path, *base, local.as_str(), incoming.as_str()))
        .collect::<Vec<_>>();
    let (root, _data, service, operation, _, _) = resolution_fixture(&files);
    let diagnostics = canonical::validate_context(
        files
            .iter()
            .map(|(path, source, _, _)| (PathBuf::from(path), source.to_string())),
    )
    .problems;
    assert_eq!(diagnostics.len(), 3);
    assert!(matches!(
        resolve_fixture(root.path(), &service, operation, &[base]),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    let after = canonical::validate_context(files.iter().map(|(path, _, _, _)| {
        (
            PathBuf::from(path),
            fs::read_to_string(root.path().join(path)).unwrap(),
        )
    }));
    assert_eq!(after.problems, diagnostics);
    for (path, contents, _, _) in &files[1..] {
        assert_eq!(
            fs::read(root.path().join(path)).unwrap(),
            contents.as_bytes()
        );
    }
}

#[test]
fn review_prospective_context_rejects_existing_touched_duplicate_and_affected_parent() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let (root, _data, service, operation, local, _) = resolution_fixture(&[
        (
            "docs/document.md",
            base,
            &base.replace("base", "local"),
            &base.replace("base", "incoming"),
        ),
        ("docs/duplicate.md", base, base, base),
    ]);
    let repository = git2::Repository::open(root.path()).unwrap();
    let index = fs::read(repository.path().join("index")).unwrap();
    let bytes = fs::read(root.path().join("docs/document.md")).unwrap();
    assert_eq!(
        resolve_fixture(root.path(), &service, operation, &[base]),
        ResolveSynchronizationOutcome::ValidationFailed
    );
    assert_eq!(repository.head().unwrap().target(), Some(local));
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
    assert_eq!(
        fs::read(root.path().join("docs/document.md")).unwrap(),
        bytes
    );

    let (parent_root, _parent_data, _parent_service) = fixture();
    let repository = git2::Repository::open(parent_root.path()).unwrap();
    fs::create_dir_all(parent_root.path().join("docs")).unwrap();
    fs::write(parent_root.path().join("docs/one.md"), base).unwrap();
    fs::write(parent_root.path().join("docs/two.md"), base).unwrap();
    commit_all(&repository);
    let observation = merge::ConflictObservation::for_testing([42; 32]);
    let comment_path =
        b".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01BX5ZZKBKACTAV9WEVGEMMVRZ.md";
    let comment = b"---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\nitem_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n\nComment\n";
    let token = merge::ConflictPathToken {
        observation,
        ordinal: 0,
        path: comment_path.to_vec(),
        base: None,
        base_mode: None,
        local: None,
        local_mode: None,
        incoming: None,
        incoming_mode: None,
    };
    let result = RedactedConflictBytes::from_bytes(comment.to_vec());
    assert!(
        !RepositoryService::validates_prospective_context(
            &repository,
            &std::collections::BTreeMap::from([(0, (&token, &result))])
        )
        .unwrap()
    );
}

#[test]
fn review_prospective_context_rejects_touched_comment_cycle() {
    let (root, _data, _service) = fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let document = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nBody\n";
    fs::create_dir_all(root.path().join("docs")).unwrap();
    fs::write(root.path().join("docs/document.md"), document).unwrap();
    commit_all(&repository);
    let first_id = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
    let second_id = "01CRZ3NDEKTSV4RRFFQ69G5FAV";
    let comment = |id, parent| {
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"{id}\"\nitem_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\nparent_id: \"{parent}\"\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n\nReply\n"
        )
    };
    let first = merge::ConflictPathToken {
        observation: merge::ConflictObservation::for_testing([42; 32]),
        ordinal: 0,
        path: format!(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/{first_id}.md").into_bytes(),
        base: None,
        base_mode: None,
        local: None,
        local_mode: None,
        incoming: None,
        incoming_mode: None,
    };
    let second = merge::ConflictPathToken {
        ordinal: 1,
        path: format!(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/{second_id}.md").into_bytes(),
        ..first.clone()
    };
    let first_bytes = RedactedConflictBytes::from_bytes(comment(first_id, second_id).into_bytes());
    let second_bytes = RedactedConflictBytes::from_bytes(comment(second_id, first_id).into_bytes());
    assert!(
        !RepositoryService::validates_prospective_context(
            &repository,
            &std::collections::BTreeMap::from([
                (0, (&first, &first_bytes)),
                (1, (&second, &second_bytes))
            ])
        )
        .unwrap()
    );
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
    resolution_fixture_with_incoming_files(files, &[])
}

fn resolution_fixture_with_incoming_files(
    files: &[(&str, &str, &str, &str)],
    new_incoming: &[(&str, &str)],
) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    OperationId,
    git2::Oid,
    git2::Oid,
) {
    resolution_fixture_with_baseline_bytes(files, new_incoming, &[])
}

fn resolution_fixture_with_baseline_bytes(
    files: &[(&str, &str, &str, &str)],
    new_incoming: &[(&str, &str)],
    baseline_bytes: &[(&str, &[u8])],
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
    for (path, bytes) in baseline_bytes {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
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
    for (path, contents) in new_incoming {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
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
    fs::write(
        root.path().join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
    )
    .unwrap();
    commit_all(&git2::Repository::open(root.path()).unwrap());
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
fn review_resolution_preserves_clean_merge_entries_in_initial_and_candidate_recovery() {
    for recover in [false, true] {
        let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
        let local = base.replace("base", "local");
        let incoming = base.replace("base", "incoming");
        let result = base.replace("base", "resolved");
        let (root, data, service, operation, local_oid, incoming_oid) =
            resolution_fixture_with_incoming_files(
                &[
                    ("docs/document.md", base, &local, &incoming),
                    ("src/changed.rs", "base\n", "base\n", "incoming\n"),
                    ("src/local.rs", "base\n", "local\n", "base\n"),
                ],
                &[("src/new.rs", "new incoming\n")],
            );
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let request = ResolveSynchronizationRequest::new(
            root.path().into(),
            operation,
            OperationId::new(),
            inspection.observation,
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(result.into_bytes()),
            )],
            None,
        );
        if recover {
            *service.failure_point.lock().unwrap() =
                Some(FailurePoint::ResolutionAfterCandidatePrepared);
            assert!(
                matches!(service.resolve_synchronization(request.clone()), Err(SynchronizationError::Repository(error)) if error.kind == RepositoryErrorKind::InjectedFailure)
            );
            *service.failure_point.lock().unwrap() = None;
        }
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
            restarted.resolve_synchronization(request).unwrap()
        else {
            panic!("clean merge entries must be accepted")
        };
        let repository = git2::Repository::open(root.path()).unwrap();
        let commit = repository.find_commit(commit_oid).unwrap();
        assert_eq!(
            [commit.parent_id(0).unwrap(), commit.parent_id(1).unwrap()],
            [local_oid, incoming_oid]
        );
        for (path, expected) in [
            ("src/changed.rs", "incoming\n"),
            ("src/local.rs", "local\n"),
            ("src/new.rs", "new incoming\n"),
        ] {
            let blob = repository
                .find_blob(
                    commit
                        .tree()
                        .unwrap()
                        .get_path(Path::new(path))
                        .unwrap()
                        .id(),
                )
                .unwrap();
            assert_eq!(blob.content(), expected.as_bytes());
            assert_eq!(
                fs::read(root.path().join(path)).unwrap(),
                expected.as_bytes()
            );
        }
    }
}

#[test]
fn explicit_resolution_preserves_lf_and_crlf_under_local_autocrlf() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    for crlf in [false, true] {
        let local = base.replace("base", "local");
        let incoming = base.replace("base", "incoming");
        let (root, _data, service, operation, _, _) =
            resolution_fixture(&[("docs/document.md", base, &local, &incoming)]);
        let repo = git2::Repository::open(root.path()).unwrap();
        repo.config()
            .unwrap()
            .set_bool("core.autocrlf", true)
            .unwrap();
        let result = base.replace("base", "resolved");
        let result = if crlf {
            result.replace('\n', "\r\n")
        } else {
            result
        };
        let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
            resolve_fixture(root.path(), &service, operation, &[&result])
        else {
            panic!("expected local checkpoint");
        };
        assert!(fs::read(root.path().join("docs/document.md")).unwrap() == result.as_bytes());
        let tree = repo.find_commit(commit_oid).unwrap().tree().unwrap();
        let blob = repo
            .find_blob(tree.get_path(Path::new("docs/document.md")).unwrap().id())
            .unwrap();
        assert!(blob.content() == result.as_bytes());
    }
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
fn review_resolution_durably_requests_refresh() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let (root, data, service, operation, _, _) = resolution_fixture(&[(
        "docs/document.md",
        base,
        &base.replace("base", "local"),
        &base.replace("base", "incoming"),
    )]);
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute("UPDATE repositories SET refresh_required=0", [])
        .unwrap();
    assert!(matches!(
        resolve_fixture(root.path(), &service, operation, &[base]),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    assert!(
        db.query_row("SELECT refresh_required FROM repositories", [], |row| row
            .get::<_, bool>(
            0
        ))
        .unwrap()
    );
}

#[test]
fn review_primary_resolution_releases_whole_repository_authoring_without_restart() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let (root, _data, service, operation, _, _) = resolution_fixture(&[(
        "docs/document.md",
        base,
        &base.replace("base", "local"),
        &base.replace("base", "incoming"),
    )]);
    let item_id: canonical::ItemId = "01BX5ZZKBKACTAV9WEVGEMMVRZ".parse().unwrap();
    let save = || SaveDocumentRequest {
        target: AuthoringTarget {
            root: root.path().into(),
            kind: AuthoringKind::Document,
            item_id: item_id.clone(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        },
        source_path: None,
        destination_path: "docs/new.md".into(),
        draft: DocumentDraft {
            title: "new document".into(),
            body: "editable".into(),
        },
        expected_source: None,
        expected_destination: ExpectedPathObservation::Missing,
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionBeforeMetadataRetirement);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let request = ResolveSynchronizationRequest::new(
        root.path().into(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(base.as_bytes().to_vec()),
        )],
        None,
    );
    assert!(service.resolve_synchronization(request.clone()).is_err());
    assert!(
        matches!(service.save_document(save()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
    );
    *service.failure_point.lock().unwrap() = None;
    assert!(matches!(
        service.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    assert!(matches!(
        service.save_document(save()).unwrap(),
        SaveOutcome::Saved { .. } | SaveOutcome::IndexPending { .. }
    ));
    let submitted = service.submit_comment(SubmitCommentRequest {
        target: AuthoringTarget {
            root: root.path().into(),
            kind: AuthoringKind::Document,
            item_id,
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        },
        comment_id: "01CRZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        parent_id: None,
        body: "available after local completion".into(),
        expected_destination: ExpectedPathObservation::Missing,
    });
    assert!(
        submitted.is_ok(),
        "comment authoring after primary release; category={:?}",
        submitted.as_ref().err().map(|error| error.kind)
    );
}

#[test]
fn review_resolution_releases_parent_after_verified_cleanup_and_replays_original_attempt() {
    for recover in [false, true] {
        let (root, data, service, operation, worktree, _, _, ticket_id) =
            context_primary_conflict_fixture();
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let result = service
            .read_synchronization_conflict(&inspection.paths[0].token)
            .unwrap()
            .local
            .unwrap();
        let request = ResolveSynchronizationRequest::new(
            root.path().into(),
            operation,
            OperationId::new(),
            inspection.observation,
            vec![(inspection.paths[0].token.clone(), result)],
            None,
        );
        let ticket_path = worktree.join(format!(".manyhands/tickets/{ticket_id}/ticket.md"));
        let edit = || SaveTicketRequest {
            target: AuthoringTarget {
                root: root.path().into(),
                kind: AuthoringKind::Ticket,
                item_id: ticket_id.clone(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "after resolution".into(),
                body: "editable".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
            },
            expected_path: ExpectedPathObservation::from_bytes(&fs::read(&ticket_path).unwrap()),
        };
        let comment = || SubmitCommentRequest {
            target: AuthoringTarget {
                root: root.path().into(),
                kind: AuthoringKind::Ticket,
                item_id: ticket_id.clone(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            comment_id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap(),
            parent_id: None,
            body: "after resolution".into(),
            expected_destination: ExpectedPathObservation::Missing,
        };
        assert!(
            matches!(service.save_ticket(edit()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
        );
        assert!(
            matches!(service.submit_comment(comment()), Err(error) if error.kind == RepositoryErrorKind::RecoveryRequired)
        );
        if recover {
            *service.failure_point.lock().unwrap() =
                Some(FailurePoint::ResolutionBeforeMetadataRetirement);
            assert!(service.resolve_synchronization(request.clone()).is_err());
            assert!(service.save_ticket(edit()).is_err());
            assert!(service.submit_comment(comment()).is_err());
            *service.failure_point.lock().unwrap() = None;
        }
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
            restarted.resolve_synchronization(request.clone()).unwrap()
        else {
            panic!("completion")
        };
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [operation.to_string()],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "interrupted"
        );
        let repository = git2::Repository::open(&worktree).unwrap();
        assert_eq!(
            repository.find_commit(commit_oid).unwrap().summary(),
            Some(format!("Resolve synchronization ticket {ticket_id}").as_str())
        );
        let logs = (
            fs::read(repository.path().join("logs/HEAD")).unwrap(),
            fs::read(repository.commondir().join(format!(
                "logs/{}",
                repository.head().unwrap().name().unwrap()
            )))
            .unwrap(),
        );
        assert!(
            matches!(restarted.resolve_synchronization(request).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid: replay } if replay == commit_oid)
        );
        assert_eq!(
            logs,
            (
                fs::read(repository.path().join("logs/HEAD")).unwrap(),
                fs::read(repository.commondir().join(format!(
                    "logs/{}",
                    repository.head().unwrap().name().unwrap()
                )))
                .unwrap()
            )
        );
        let saved = restarted.save_ticket(edit());
        assert!(
            saved.is_ok(),
            "ticket authoring after parent release; category={:?}",
            saved.as_ref().err().map(|error| error.kind)
        );
        assert!(matches!(
            saved.unwrap(),
            SaveOutcome::Saved { .. } | SaveOutcome::IndexPending { .. }
        ));
        let submitted = restarted.submit_comment(comment());
        assert!(
            submitted.is_ok(),
            "comment authoring after parent release; category={:?}",
            submitted.as_ref().err().map(|error| error.kind)
        );
    }
}

#[test]
fn review_resolution_finalizer_and_refresh_sql_faults_retry_without_new_git_effects() {
    for fault in ["refresh", "finalize"] {
        let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
        let (root, data, service, operation, _, _) = resolution_fixture(&[(
            "docs/document.md",
            base,
            &base.replace("base", "local"),
            &base.replace("base", "incoming"),
        )]);
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        let request = ResolveSynchronizationRequest::new(
            root.path().into(),
            operation,
            OperationId::new(),
            inspection.observation,
            vec![(
                inspection.paths[0].token.clone(),
                RedactedConflictBytes::from_bytes(base.as_bytes().to_vec()),
            )],
            None,
        );
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute("UPDATE repositories SET refresh_required=0", [])
            .unwrap();
        db.execute_batch(if fault == "refresh" {
            "CREATE TRIGGER review_fault BEFORE UPDATE OF refresh_required ON repositories WHEN NEW.refresh_required=1 BEGIN SELECT RAISE(ABORT,'test fault'); END;"
        } else {
            "CREATE TRIGGER review_fault BEFORE UPDATE OF phase ON remote_operation_records WHEN OLD.phase='reconciling' AND NEW.phase='interrupted' BEGIN SELECT RAISE(ABORT,'test fault'); END;"
        }).unwrap();
        assert!(
            service.resolve_synchronization(request.clone()).is_err(),
            "{fault} must retain recovery"
        );
        let repository = git2::Repository::open(root.path()).unwrap();
        let candidate = repository.head().unwrap().target().unwrap();
        let logs = (
            fs::read(repository.path().join("logs/HEAD")).unwrap(),
            fs::read(repository.path().join("logs/refs/heads/main")).unwrap(),
        );
        assert_eq!(
            db.query_row("SELECT phase FROM remote_operation_records", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
            "reconciling"
        );
        if fault == "refresh" {
            assert_eq!(
                db.query_row("SELECT phase FROM remote_resolution_attempts", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
                "candidate_prepared"
            );
            assert!(
                !db.query_row("SELECT refresh_required FROM repositories", [], |row| row
                    .get::<_, bool>(
                    0
                ))
                .unwrap()
            );
        } else {
            assert!(!repository.path().join("index.lock").exists());
            assert_eq!(
                db.query_row(
                    "SELECT phase FROM remote_resolution_index_artifacts",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
                "released"
            );
        }
        db.execute_batch("DROP TRIGGER review_fault").unwrap();
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        assert!(
            matches!(restarted.resolve_synchronization(request).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
        );
        assert_eq!(
            logs,
            (
                fs::read(repository.path().join("logs/HEAD")).unwrap(),
                fs::read(repository.path().join("logs/refs/heads/main")).unwrap()
            )
        );
        assert!(
            db.query_row("SELECT refresh_required FROM repositories", [], |row| row
                .get::<_, bool>(
                0
            ))
            .unwrap()
        );
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let RemoteReservationOutcome::Reserved(owner) = restarted
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("original sync restart")
        };
        let mut evidence = state::with_transaction(&restarted, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let pending = reconcile_pending_candidate(
            &restarted,
            root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &owner,
            &mut evidence,
        )
        .unwrap()
        .expect("released resolution candidate");
        assert_eq!(pending.oid, candidate);
        restarted
            .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
            .unwrap();
        let observation = RemoteRefObservation::from_advertisement(
            &plan,
            "refs/heads/main",
            evidence.primary_tracking_oid.unwrap(),
            None,
        )
        .unwrap();
        commit_observation_batch(&restarted, root.path(), &owner, &plan, &[observation], 2)
            .unwrap();
        finalize_reconciled_candidate(&restarted, root.path(), &owner, pending, &evidence).unwrap();
        let record = state::with_transaction(&restarted, root.path(), |tx, id| {
            state::read_operation(tx, id, operation)
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            record.sync_checkpoint,
            Some(state::SynchronizationCheckpoint::LocalFastForwarded)
        );
        assert_eq!(record.sync_evidence.local_oid, Some(candidate));
        assert!(!record.reconciliation_required);
        assert!(record.authority.is_none());
        assert_eq!(repository.head().unwrap().target(), Some(candidate));
        assert_eq!(
            logs,
            (
                fs::read(repository.path().join("logs/HEAD")).unwrap(),
                fs::read(repository.path().join("logs/refs/heads/main")).unwrap()
            )
        );
    }
}

#[test]
fn review_resolution_uses_target_specific_subject_for_new_candidate() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let (root, _data, service, operation, _, _) = resolution_fixture(&[(
        "docs/document.md",
        base,
        &base.replace("base", "local"),
        &base.replace("base", "incoming"),
    )]);
    let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
        resolve_fixture(root.path(), &service, operation, &[base])
    else {
        panic!("completion")
    };
    let repository = git2::Repository::open(root.path()).unwrap();
    assert_eq!(
        repository.find_commit(commit_oid).unwrap().summary(),
        Some("Resolve synchronization primary")
    );
}

#[test]
fn review_released_resolution_restart_refuses_old_or_third_head_without_git_effects() {
    for restore_old in [true, false] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let operation = request.synchronization_id;
        let local = request.observation.head;
        assert!(matches!(
            service.resolve_synchronization(request).unwrap(),
            ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
        ));
        assert!(
            service
                .active_remote_operation(root.path())
                .unwrap()
                .is_none()
        );
        let repository = git2::Repository::open(root.path()).unwrap();
        let log_image = || {
            (
                *blake3::hash(&fs::read(repository.path().join("logs/HEAD")).unwrap()).as_bytes(),
                *blake3::hash(&fs::read(repository.path().join("logs/refs/heads/main")).unwrap())
                    .as_bytes(),
            )
        };
        let completed_logs = log_image();
        let restored = if restore_old {
            local
        } else {
            let parent = repository.find_commit(local).unwrap();
            let signature = repository.signature().unwrap();
            repository
                .commit(
                    None,
                    &signature,
                    &signature,
                    "external third commit",
                    &parent.tree().unwrap(),
                    &[&parent],
                )
                .unwrap()
        };
        // External mutation after completed release: restore a clean checkout
        // without erasing or appending the completed resolution's log history.
        fs::write(
            repository.path().join("refs/heads/main"),
            format!("{restored}\n"),
        )
        .unwrap();
        let restored_repository = git2::Repository::open(root.path()).unwrap();
        restored_repository
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        assert_eq!(restored_repository.head().unwrap().target(), Some(restored));
        assert!(restored_repository.statuses(None).unwrap().is_empty());
        assert_eq!(log_image(), completed_logs);
        let before = local_binding_image(root.path());
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let RemoteReservationOutcome::Reserved(owner) = restarted
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("original sync restart")
        };
        let mut evidence = state::with_transaction(&restarted, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let result = reconcile_pending_candidate(
            &restarted,
            root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &owner,
            &mut evidence,
        );
        assert!(
            matches!(result, Err(SynchronizationError::RecoveryRequired)),
            "released resolution handoff must be observation-only"
        );
        assert_eq!(
            local_binding_image(root.path()),
            before,
            "restart must preserve ref/index/worktree"
        );
        assert_eq!(
            log_image(),
            completed_logs,
            "restart must not append either reflog"
        );
    }
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
fn pending_conflict_restart_is_offline_and_preserves_git_evidence() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let (root, data, service, operation, local, _) = resolution_fixture(&[(
        "docs/document.md",
        base,
        &base.replace("base", "local"),
        &base.replace("base", "incoming"),
    )]);
    let before = local_binding_image(root.path());
    let repository = git2::Repository::open(root.path()).unwrap();
    let metadata = fs::read(repository.path().join("MERGE_HEAD")).unwrap();
    let mut retry = request(root.path());
    retry.operation_id = operation;
    retry.restart = true;
    // This fixture has no transport remote. Reaching transport/configuration
    // preflight rather than the owned local conflict is a regression.
    for service in [service, RepositoryService::open_at(data.path()).unwrap()] {
        assert!(matches!(
            service.synchronize_remote(retry.clone(), &mut SessionCredentials::new(NoPrompt)),
            Err(SynchronizationError::ConflictPending { operation_id, .. }) if operation_id == operation
        ));
        assert_eq!(local_binding_image(root.path()), before);
        assert_eq!(repository.head().unwrap().target(), Some(local));
        assert_eq!(
            fs::read(repository.path().join("MERGE_HEAD")).unwrap(),
            metadata
        );
        assert!(
            service
                .inspect_synchronization_recovery(root.path(), operation)
                .is_ok()
        );
    }
    fs::write(root.path().join("fixture.txt"), b"unrelated dirty\n").unwrap();
    let before = local_binding_image(root.path());
    assert!(matches!(
        RepositoryService::open_at(data.path())
            .unwrap()
            .synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(local_binding_image(root.path()), before);
}

#[test]
fn new_local_reconciliation_fences_cancel_and_stale_service_owner() {
    for cancel in [false, true] {
        let (root, data, service, operation, _, _) =
            resolution_fixture(&[("src/foreign.rs", "base\n", "local\n", "incoming\n")]);
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let RemoteReservationOutcome::Reserved(owner) = service
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("original owner")
        };
        let other = RepositoryService::open_at(data.path()).unwrap();
        if cancel {
            other
                .cancel_remote_operation(root.path(), operation)
                .unwrap();
        } else {
            assert!(matches!(
                other
                    .restart_remote_synchronization(root.path(), operation, &target)
                    .unwrap(),
                RemoteReservationOutcome::Reserved(_)
            ));
        }
        let before = local_binding_image(root.path());
        let mut evidence = state::with_transaction(&other, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let result = reconcile_pending_candidate(
            &service,
            root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &owner,
            &mut evidence,
        );
        if cancel {
            assert!(matches!(result, Err(SynchronizationError::Interrupted)));
        } else {
            assert!(matches!(
                result,
                Err(SynchronizationError::Repository(RepositoryError {
                    kind: RepositoryErrorKind::RecoveryRequired,
                    ..
                }))
            ));
        }
        assert_eq!(local_binding_image(root.path()), before);
        assert!(
            git2::Repository::open(root.path())
                .unwrap()
                .index()
                .unwrap()
                .has_conflicts()
        );
        if cancel {
            // The cancellation stopped that owner without ending the
            // operation: its pending conflict is still restartable.
            let mut retry = request(root.path());
            retry.operation_id = operation;
            retry.restart = true;
            assert!(matches!(
                other.synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
                Err(SynchronizationError::ConflictPending { operation_id, .. })
                    if operation_id == operation
            ));
            assert_eq!(local_binding_image(root.path()), before);
        }
    }
}

#[test]
fn unfinished_merge_observation_restart_records_the_installed_conflict_offline() {
    let (root, data, service, operation, _, _) =
        resolution_fixture(&[("src/foreign.rs", "base\n", "local\n", "incoming\n")]);
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute(
        "UPDATE remote_integration_steps SET phase='applying',conflict_digest=NULL",
        [],
    )
    .unwrap();
    let before = local_binding_image(root.path());
    let mut retry = request(root.path());
    retry.operation_id = operation;
    retry.restart = true;
    assert!(matches!(
        service.synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    assert_eq!(local_binding_image(root.path()), before);
    assert!(
        service
            .inspect_synchronization_recovery(root.path(), operation)
            .is_ok()
    );
    assert_eq!(
        db.query_row("SELECT phase FROM remote_integration_steps", [], |row| {
            row.get::<_, String>(0)
        })
        .unwrap(),
        "conflict_pending"
    );
}

#[test]
fn conflict_inspection_rejects_same_tree_head_movement_during_preparation() {
    for unfinished in [false, true] {
        let (root, data, service, operation, local, _) =
            resolution_fixture(&[("src/foreign.rs", "base\n", "local\n", "incoming\n")]);
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        if unfinished {
            db.execute(
                "UPDATE remote_integration_steps SET phase='applying',conflict_digest=NULL",
                [],
            )
            .unwrap();
        }
        let step = |db: &rusqlite::Connection| {
            db.query_row(
                "SELECT phase,conflict_digest FROM remote_integration_steps",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<Vec<u8>>>(1)?)),
            )
            .unwrap()
        };
        let recorded = step(&db);
        let fired = Arc::new(AtomicBool::new(false));
        let observed = fired.clone();
        let hook_root = root.path().to_owned();
        set_local_reconciliation_prepared_hook(root.path().to_owned(), move || {
            observed.store(true, Ordering::SeqCst);
            let repository = git2::Repository::open(hook_root).unwrap();
            let commit = repository.find_commit(local).unwrap();
            let signature = git2::Signature::now("External", "external@example.invalid").unwrap();
            let foreign = repository
                .commit(
                    None,
                    &signature,
                    &signature,
                    "External same-tree commit",
                    &commit.tree().unwrap(),
                    &[&commit],
                )
                .unwrap();
            assert_ne!(foreign, local);
            // Fixture-only direct mutation models stale state at the revalidation
            // boundary; index, worktree and merge metadata stay byte-identical.
            fs::write(
                repository.path().join("refs/heads/main"),
                format!("{foreign}\n"),
            )
            .unwrap();
        });
        let mut retry = request(root.path());
        retry.operation_id = operation;
        retry.restart = true;
        assert!(matches!(
            service.synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
            Err(SynchronizationError::RecoveryRequired)
        ));
        assert!(fired.load(Ordering::SeqCst));
        let repository = git2::Repository::open(root.path()).unwrap();
        let head = repository.head().unwrap().target().unwrap();
        assert_ne!(head, local);
        assert_eq!(
            repository.find_commit(head).unwrap().parent_id(0).unwrap(),
            local
        );
        assert!(repository.index().unwrap().has_conflicts());
        assert!(repository.path().join("MERGE_HEAD").exists());
        // No conflict observation is recorded or released for a parent that
        // is no longer the attached target's HEAD.
        assert_eq!(step(&db), recorded);
    }
}

#[test]
fn external_repair_exact_ordered_parent_clean_only() {
    for variant in [
        "exact",
        "reversed",
        "one_parent",
        "unrelated",
        "dirty",
        "detached",
        "invalid",
        "invalid_new",
        "partial_metadata",
    ] {
        let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
        let (root, data, service, operation, local, incoming) = resolution_fixture(&[
            (
                "docs/document.md",
                base,
                &base.replace("base", "local"),
                &base.replace("base", "incoming"),
            ),
            ("src/foreign.rs", "base\n", "local\n", "incoming\n"),
        ]);
        let repository = git2::Repository::open(root.path()).unwrap();
        let tree = if matches!(variant, "invalid" | "invalid_new") {
            let old = repository.find_commit(local).unwrap().tree().unwrap();
            let mut index = git2::Index::new().unwrap();
            index.read_tree(&old).unwrap();
            let mut entry = index.get_path(Path::new("docs/document.md"), 0).unwrap();
            let bytes: &[u8] = if variant == "invalid_new" {
                entry.path = b"docs/new-invalid.md".to_vec();
                b"---\nmanyhands_managed: true\nmanyhands_kind: document\ntitle: Missing identity\n---\n\ninvalid\n"
            } else {
                b"invalid canonical result\n"
            };
            entry.id = repository.blob(bytes).unwrap();
            index.add(&entry).unwrap();
            repository
                .find_tree(index.write_tree_to(&repository).unwrap())
                .unwrap()
        } else {
            repository.find_commit(local).unwrap().tree().unwrap()
        };
        let local_parent = repository.find_commit(local).unwrap();
        let incoming_parent = repository.find_commit(incoming).unwrap();
        let signature = repository.signature().unwrap();
        let unrelated_parent = (variant == "unrelated").then(|| {
            let oid = repository
                .commit(
                    None,
                    &signature,
                    &signature,
                    "unrelated fixture",
                    &tree,
                    &[],
                )
                .unwrap();
            repository.find_commit(oid).unwrap()
        });
        let parents = match variant {
            "reversed" => vec![&incoming_parent, &local_parent],
            "one_parent" => vec![&local_parent],
            "unrelated" => vec![&local_parent, unrelated_parent.as_ref().unwrap()],
            _ => vec![&local_parent, &incoming_parent],
        };
        // libgit2 refuses a HEAD update whose first parent is not the tip, so
        // false repairs are installed as an external tool would leave them.
        let repaired = repository
            .commit(
                None,
                &signature,
                &signature,
                "external repair",
                &tree,
                &parents,
            )
            .unwrap();
        repository
            .reference("refs/heads/main", repaired, true, "external repair")
            .unwrap();
        repository.cleanup_state().unwrap();
        repository
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        if variant == "dirty" {
            fs::write(root.path().join("fixture.txt"), b"unrelated dirty\n").unwrap();
        }
        if variant == "detached" {
            repository.set_head_detached(repaired).unwrap();
        }
        if variant == "partial_metadata" {
            fs::write(
                repository.path().join("MERGE_MSG"),
                b"uncertain external remnant\n",
            )
            .unwrap();
        }
        let before = local_binding_image(root.path());
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        let RemoteReservationOutcome::Reserved(owner) = restarted
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("same-operation local reconciliation")
        };
        let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let result = reconcile_pending_candidate(
            &restarted,
            root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &owner,
            &mut evidence,
        );
        if variant == "exact" {
            assert_eq!(result.unwrap().unwrap().oid, repaired);
            let step = state::with_transaction(&restarted, root.path(), |tx, id| {
                state::integration_step(
                    tx,
                    state::read_operation(tx, id, operation)?.unwrap().id,
                    0,
                )
            })
            .unwrap()
            .unwrap();
            assert_eq!(step.phase, state::IntegrationStepPhase::Applied);
            assert_eq!(step.result_oid, Some(repaired));
            assert_eq!(step.intent.local_oid, local);
            assert_eq!(step.intent.incoming_oid, incoming);
        } else {
            assert!(
                matches!(result, Err(SynchronizationError::RecoveryRequired)),
                "{variant}"
            );
        }
        assert_eq!(local_binding_image(root.path()), before, "{variant}");
    }
}

fn inspection_odb_inventory(
    repository: &git2::Repository,
) -> std::collections::BTreeSet<git2::Oid> {
    let mut objects = std::collections::BTreeSet::new();
    repository
        .odb()
        .unwrap()
        .foreach(|oid| {
            objects.insert(*oid);
            true
        })
        .unwrap();
    objects
}

fn inspection_git_image(root: &Path) -> [u8; 32] {
    let repository = git2::Repository::open(root).unwrap();
    let mut digest = blake3::Hasher::new();
    digest.update(&local_binding_image(root));
    for name in [
        "logs/HEAD",
        "logs/refs/heads/main",
        "MERGE_HEAD",
        "MERGE_MSG",
        "MERGE_MODE",
    ] {
        match fs::read(repository.path().join(name)) {
            Ok(bytes) => {
                digest.update(&[1]);
                digest.update(&bytes);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                digest.update(&[0]);
            }
            Err(error) => panic!("fixture metadata read category: {:?}", error.kind()),
        }
    }
    *digest.finalize().as_bytes()
}

fn inspection_tree_insert(
    repository: &git2::Repository,
    tree: &git2::Tree<'_>,
    path: &str,
    oid: git2::Oid,
    mode: i32,
) -> git2::Oid {
    let mut builder = repository.treebuilder(Some(tree)).unwrap();
    if let Some((directory, tail)) = path.split_once('/') {
        let child = tree
            .get_name(directory)
            .filter(|entry| entry.kind() == Some(git2::ObjectType::Tree))
            .map(|entry| entry.id())
            .unwrap_or_else(|| repository.treebuilder(None).unwrap().write().unwrap());
        let child = repository.find_tree(child).unwrap();
        let updated = inspection_tree_insert(repository, &child, tail, oid, mode);
        builder.insert(directory, updated, 0o040000).unwrap();
    } else {
        builder.insert(path, oid, mode).unwrap();
    }
    builder.write().unwrap()
}

#[test]
fn external_repair_new_canonical_encoding_and_shape_gate_preserves_invalid_baseline() {
    let document = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let new_id = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    let item_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let cases = [
        ("docs/new.md".to_owned(), document.replace(item_id, new_id)),
        (
            format!(".manyhands/tickets/{new_id}/ticket.md"),
            format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"{new_id}\"\ntitle: New\ntype: task\nstatus: open\n---\n\nnew\n"
            ),
        ),
        (
            format!(".manyhands/comments/{item_id}/{new_id}.md"),
            format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"{new_id}\"\nitem_id: \"{item_id}\"\ncreated_at: 2026-09-30T12:00:00Z\n---\n\nnew\n"
            ),
        ),
    ];
    for (path, valid) in cases {
        for shape in [
            "invalid_utf8",
            "symlink",
            "tree",
            "executable",
            "valid_code_control",
        ] {
            let (root, data, service, operation, local, incoming) =
                resolution_fixture_with_baseline_bytes(
                    &[("src/foreign.rs", "base\n", "local\n", "incoming\n")],
                    &[],
                    &[
                        ("docs/item.md", document.as_bytes()),
                        ("docs/unchanged-invalid.md", b"\xff\xfe"),
                        (
                            "docs/unchanged-malformed.md",
                            b"---\nmanyhands_managed: true\nmanyhands_kind: document\n---\n\nlegacy\n",
                        ),
                        (
                            ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAX/ticket.md",
                            b"\xff\xfe",
                        ),
                        (
                            ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAY.md",
                            b"\xff\xfe",
                        ),
                    ],
                );
            let repository = git2::Repository::open(root.path()).unwrap();
            let local_parent = repository.find_commit(local).unwrap();
            let incoming_parent = repository.find_commit(incoming).unwrap();
            let tree = local_parent.tree().unwrap();
            let (entry_oid, mode) = match shape {
                "invalid_utf8" => (repository.blob(b"\xff\xfe").unwrap(), 0o100644),
                "tree" => (
                    repository.treebuilder(None).unwrap().write().unwrap(),
                    0o040000,
                ),
                "symlink" => (repository.blob(valid.as_bytes()).unwrap(), 0o120000),
                "executable" => (repository.blob(valid.as_bytes()).unwrap(), 0o100755),
                _ => (
                    repository.blob(b"externally repaired code\n").unwrap(),
                    0o100644,
                ),
            };
            let inserted = if shape == "valid_code_control" {
                "src/foreign.rs"
            } else {
                &path
            };
            let candidate_tree =
                inspection_tree_insert(&repository, &tree, inserted, entry_oid, mode);
            let tree = repository.find_tree(candidate_tree).unwrap();
            let signature = repository.signature().unwrap();
            let candidate = repository
                .commit(
                    None,
                    &signature,
                    &signature,
                    "external repair",
                    &tree,
                    &[&local_parent, &incoming_parent],
                )
                .unwrap();
            let step = state::with_transaction(&service, root.path(), |tx, id| {
                let record = state::read_operation(tx, id, operation)?.unwrap();
                state::integration_step(tx, record.id, 0)
            })
            .unwrap()
            .unwrap();
            let objects = inspection_odb_inventory(&repository);
            let before = inspection_git_image(root.path());
            assert_eq!(
                RepositoryService::validates_external_integration(&repository, &step, candidate)
                    .unwrap(),
                shape == "valid_code_control",
                "{shape}"
            );
            assert_eq!(inspection_odb_inventory(&repository), objects, "{shape}");
            assert_eq!(inspection_git_image(root.path()), before, "{shape}");
            // Regular invalid bytes and the positive control also exercise the
            // adoption boundary without OS-dependent symlink checkout setup.
            if matches!(shape, "invalid_utf8" | "valid_code_control") {
                repository
                    .reference(
                        "refs/heads/main",
                        candidate,
                        true,
                        "fixture external repair",
                    )
                    .unwrap();
                repository.cleanup_state().unwrap();
                repository
                    .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                    .unwrap();
                let before = inspection_git_image(root.path());
                let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
                let target = RemoteOperationTarget::for_primary_synchronization(&plan);
                let RemoteReservationOutcome::Reserved(owner) = service
                    .restart_remote_synchronization(root.path(), operation, &target)
                    .unwrap()
                else {
                    panic!("fenced external inspection")
                };
                let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
                    Ok(state::read_operation(tx, id, operation)?
                        .unwrap()
                        .sync_evidence)
                })
                .unwrap();
                let result = reconcile_pending_candidate(
                    &service,
                    root.path(),
                    "main",
                    &SynchronizationTarget::Primary,
                    &owner,
                    &mut evidence,
                );
                if shape == "valid_code_control" {
                    assert_eq!(result.unwrap().unwrap().oid, candidate);
                } else {
                    assert!(matches!(
                        result,
                        Err(SynchronizationError::RecoveryRequired)
                    ));
                    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
                    assert_eq!(
                        db.query_row("SELECT phase FROM remote_integration_steps", [], |row| {
                            row.get::<_, String>(0)
                        })
                        .unwrap(),
                        "conflict_pending"
                    );
                }
                assert_eq!(inspection_odb_inventory(&repository), objects, "{shape}");
                assert_eq!(inspection_git_image(root.path()), before, "{shape}");
            }
        }
    }
}

#[test]
fn rejected_recorded_merge_inspection_never_imports_generated_clean_blobs() {
    for phase in [
        "applying",
        "conflict_pending",
        "stale_owner",
        "index_mismatch",
    ] {
        let clean_base = "base first\nmiddle one\nmiddle two\nmiddle three\nbase last\n";
        let clean_local = "local first\nmiddle one\nmiddle two\nmiddle three\nbase last\n";
        let clean_incoming = "base first\nmiddle one\nmiddle two\nmiddle three\nincoming last\n";
        let merged = b"local first\nmiddle one\nmiddle two\nmiddle three\nincoming last\n";
        let (root, data, service, operation, local, incoming) = resolution_fixture(&[
            ("src/foreign.rs", "base\n", "local\n", "incoming\n"),
            ("clean.txt", clean_base, clean_local, clean_incoming),
        ]);
        let repository = git2::Repository::open(root.path()).unwrap();
        // The fixture initially installed the real merge. Remove ONLY this
        // generated loose test blob to model an interrupted pre-install pass
        // with foreign merge metadata, before inventorying rejection effects.
        let canary = git2::Oid::hash_object(git2::ObjectType::Blob, merged).unwrap();
        assert!(repository.odb().unwrap().exists(canary));
        let text = canary.to_string();
        fs::remove_file(
            repository
                .commondir()
                .join("objects")
                .join(&text[..2])
                .join(&text[2..]),
        )
        .unwrap();
        assert!(!repository.odb().unwrap().exists(canary));
        let other = repository.find_commit(local).unwrap().parent_id(0).unwrap();
        fs::write(repository.path().join("MERGE_HEAD"), format!("{other}\n")).unwrap();
        if phase == "index_mismatch" {
            let mut index = repository.index().unwrap();
            let mut entry = index.get_path(Path::new("fixture.txt"), 0).unwrap();
            entry.id = repository.blob(b"unrelated staged fixture\n").unwrap();
            index.add(&entry).unwrap();
            index.write().unwrap();
        }
        if phase == "applying" {
            let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
            db.execute(
                "UPDATE remote_integration_steps SET phase='applying',conflict_digest=NULL",
                [],
            )
            .unwrap();
        }
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let RemoteReservationOutcome::Reserved(owner) = service
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("inspection owner")
        };
        if phase == "stale_owner" {
            let other_service = RepositoryService::open_at(data.path()).unwrap();
            assert!(matches!(
                other_service
                    .restart_remote_synchronization(root.path(), operation, &target)
                    .unwrap(),
                RemoteReservationOutcome::Reserved(_)
            ));
        }
        let objects = inspection_odb_inventory(&repository);
        let before = inspection_git_image(root.path());
        let step_before = state::with_transaction(&service, root.path(), |tx, id| {
            let record = state::read_operation(tx, id, operation)?.unwrap();
            state::integration_step(tx, record.id, 0)
        })
        .unwrap()
        .unwrap();
        let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        assert!(
            reconcile_pending_candidate(
                &service,
                root.path(),
                "main",
                &SynchronizationTarget::Primary,
                &owner,
                &mut evidence,
            )
            .is_err()
        );
        assert!(!repository.odb().unwrap().exists(canary), "{phase}");
        assert_eq!(inspection_odb_inventory(&repository), objects, "{phase}");
        assert_eq!(inspection_git_image(root.path()), before, "{phase}");
        let step_after = state::with_transaction(&service, root.path(), |tx, id| {
            let record = state::read_operation(tx, id, operation)?.unwrap();
            state::integration_step(tx, record.id, 0)
        })
        .unwrap()
        .unwrap();
        assert_eq!(step_after, step_before, "{phase}");
        // The reusable read-only comparison must also keep its generated
        // stage-zero blob solely in its private overlay, even on mismatch.
        let expected = prepare_recorded_merge_index(&repository, local, incoming).unwrap();
        assert_eq!(
            expected.get_path(Path::new("clean.txt"), 0).unwrap().id,
            canary
        );
        assert_eq!(
            recorded_merge_index_matches(&repository, local, incoming).unwrap(),
            phase != "index_mismatch"
        );
        assert!(!repository.odb().unwrap().exists(canary));
        assert_eq!(inspection_odb_inventory(&repository), objects);
    }
}

#[test]
fn context_done_primary_pending_restart_is_local_only_without_stage_replay() {
    let (root, data, service, operation, worktree, local, _, item_id) =
        context_primary_conflict_fixture();
    let before = local_binding_image(&worktree);
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let context_oid: String = db
        .query_row(
            "SELECT result_oid FROM remote_integration_steps WHERE stage='context'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut retry = request(root.path());
    retry.operation_id = operation;
    retry.target = SynchronizationTarget::Context {
        kind: crate::repository::AuthoringKind::Ticket,
        item_id,
    };
    retry.restart = true;
    assert!(matches!(
        service.synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::ConflictPending {
            stage: SynchronizationStage::Primary,
            ..
        })
    ));
    assert_eq!(local_binding_image(&worktree), before);
    assert_eq!(
        git2::Repository::open(&worktree)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        Some(local)
    );
    assert_eq!(
        db.query_row(
            "SELECT result_oid FROM remote_integration_steps WHERE stage='context'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        context_oid
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_integration_steps", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        2
    );
}

#[test]
fn conflict_tokens_bind_windows_without_rebinding_legacy_attempts() {
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("src/foreign.rs", "base\n", "local\n", "incoming\n")]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let original = inspection.paths[0].token.clone();
    let mut later = original.clone();
    later.observation.window_number = 1;
    assert_ne!(original, later);
    assert_ne!(
        bind_conflict_digest(
            original.observation.fingerprint,
            1,
            original.observation.ordinal
        ),
        original.observation.fingerprint
    );
    assert_ne!(
        conflict_token_digest(&original),
        conflict_token_digest(&later)
    );
    assert!(matches!(
        service.read_synchronization_conflict(&later),
        Err(SynchronizationError::ExternalChange)
    ));
}

#[test]
fn local_effect_restart_before_clean_preflight_and_transport() {
    for fast_forwarded in [false, true] {
        let (root, data, service) = fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        fs::write(
            root.path().join(".manyhands/config.toml"),
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
        )
        .unwrap();
        let local = commit_all(&repository);
        let incoming = if fast_forwarded {
            child(&repository, local, b"incoming\n")
        } else {
            local
        };
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&plan), false)
        })
        .unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let operation = OperationId::new();
        let RemoteReservationOutcome::Reserved(owner) = service
            .reserve_remote_operation(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("reservation")
        };
        let evidence = state::SynchronizationEvidence {
            expected_oid: Some(local),
            local_oid: Some(local),
            primary_tracking_oid: Some(incoming),
            tracking_oid: Some(incoming),
            ..Default::default()
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
        let tree = repository.find_commit(local).unwrap().tree_id();
        service
            .prepare_synchronization_integration(
                root.path(),
                &owner,
                &state::IntegrationStepIntent {
                    ordinal: 0,
                    stage: merge::IntegrationStage::Primary,
                    local_oid: local,
                    incoming_oid: incoming,
                    baseline_tree_oid: tree,
                    baseline_index_digest: index_digest(tree),
                },
            )
            .unwrap();
        service
            .begin_synchronization_integration_effect(root.path(), &owner, 0, None)
            .unwrap();
        if fast_forwarded {
            fast_forward(&repository, "refs/heads/main", local, incoming).unwrap();
        }
        let before = local_binding_image(root.path());
        let mut retry = request(root.path());
        retry.operation_id = operation;
        retry.restart = true;
        // Endpoint discovery will fail; it must happen after observing the
        // already-completed local effect, without repeating checkout/ref/logs.
        assert!(
            RepositoryService::open_at(data.path())
                .unwrap()
                .synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt))
                .is_err()
        );
        let step = state::with_transaction(&service, root.path(), |tx, id| {
            state::integration_step(tx, state::read_operation(tx, id, operation)?.unwrap().id, 0)
        })
        .unwrap()
        .unwrap();
        assert_eq!(step.phase, state::IntegrationStepPhase::Applied);
        assert_eq!(step.result_oid, Some(incoming));
        assert_eq!(local_binding_image(root.path()), before);
    }
}

#[test]
fn external_repair_refuses_marker_removal_and_staged_code_without_commit() {
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("src/foreign.rs", "base\n", "local\n", "incoming\n")]);
    let repository = git2::Repository::open(root.path()).unwrap();
    fs::write(root.path().join("src/foreign.rs"), b"resolved\n").unwrap();
    let mut index = repository.index().unwrap();
    for stage in 1..=3 {
        index.remove(Path::new("src/foreign.rs"), stage).unwrap();
    }
    index.add_path(Path::new("src/foreign.rs")).unwrap();
    index.write().unwrap();
    let before = local_binding_image(root.path());
    let metadata = fs::read(repository.path().join("MERGE_HEAD")).unwrap();
    let mut retry = request(root.path());
    retry.operation_id = operation;
    retry.restart = true;
    assert!(matches!(
        service.synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(local_binding_image(root.path()), before);
    assert_eq!(
        fs::read(repository.path().join("MERGE_HEAD")).unwrap(),
        metadata
    );
}

#[test]
fn released_checkpoint_then_normal_save_continuation_preserves_original_candidate() {
    let (root, data, service, operation, worktree, _, _, ticket_id) =
        context_primary_conflict_fixture();
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let result = service
        .read_synchronization_conflict(&inspection.paths[0].token)
        .unwrap()
        .local
        .unwrap();
    let resolution = ResolveSynchronizationRequest::new(
        root.path().into(),
        operation,
        OperationId::new(),
        inspection.observation,
        vec![(inspection.paths[0].token.clone(), result)],
        None,
    );
    let ResolveSynchronizationOutcome::LocalCheckpointComplete {
        commit_oid: checkpoint,
    } = service.resolve_synchronization(resolution).unwrap()
    else {
        panic!("released checkpoint")
    };
    let ticket_path = worktree.join(format!(".manyhands/tickets/{ticket_id}/ticket.md"));
    assert!(matches!(
        service
            .save_ticket(SaveTicketRequest {
                target: AuthoringTarget {
                    root: root.path().into(),
                    kind: AuthoringKind::Ticket,
                    item_id: ticket_id.clone(),
                    intent: ContextIntent::Edit,
                    operation_id: OperationId::new(),
                },
                draft: TicketDraft {
                    title: "normal continuation".into(),
                    body: "later saved checkpoint".into(),
                    ticket_type: "task".into(),
                    status: "open".into(),
                    project: None,
                    team: None,
                },
                expected_path: ExpectedPathObservation::from_bytes(&fs::read(ticket_path).unwrap()),
            })
            .unwrap(),
        SaveOutcome::Saved { .. } | SaveOutcome::IndexPending { .. }
    ));
    let repository = git2::Repository::open(&worktree).unwrap();
    let descendant = repository.head().unwrap().target().unwrap();
    assert!(
        repository
            .graph_descendant_of(descendant, checkpoint)
            .unwrap()
    );
    // A save commits its owned paths without rewriting the on-disk index.
    // Continuation needs a clean target, so refresh it as the public
    // synchronization fixtures do after authoring.
    let mut index = repository.index().unwrap();
    index
        .read_tree(&repository.head().unwrap().peel_to_tree().unwrap())
        .unwrap();
    index.write().unwrap();
    let before = local_binding_image(&worktree);
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        ticket_id.clone(),
    )
    .unwrap();
    let RemoteReservationOutcome::Reserved(owner) = restarted
        .restart_remote_synchronization(root.path(), operation, &target)
        .unwrap()
    else {
        panic!("original operation")
    };
    let mut evidence = state::with_transaction(&restarted, root.path(), |tx, id| {
        Ok(state::read_operation(tx, id, operation)?
            .unwrap()
            .sync_evidence)
    })
    .unwrap();
    let sync_target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ticket_id,
    };
    let observed = reconcile_pending_candidate(
        &restarted,
        root.path(),
        "main",
        &sync_target,
        &owner,
        &mut evidence,
    )
    .unwrap()
    .unwrap();
    assert_eq!(observed.oid, checkpoint);
    assert_eq!(evidence.local_oid, Some(descendant));
    let step = state::with_transaction(&restarted, root.path(), |tx, id| {
        state::integration_step(tx, state::read_operation(tx, id, operation)?.unwrap().id, 1)
    })
    .unwrap()
    .unwrap();
    assert_eq!(step.candidate_oid, Some(checkpoint));
    assert_eq!(step.result_oid, Some(checkpoint));
    assert_eq!(local_binding_image(&worktree), before);
}

#[test]
fn synchronization_handoff_refuses_unreleased_resolution_artifacts() {
    let (root, data, service, resolution) = protocol_resolution_fixture();
    let operation = resolution.synchronization_id;
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionBeforeMetadataRetirement);
    assert!(service.resolve_synchronization(resolution).is_err());
    let before = local_binding_image(root.path());
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let phase: String = db
        .query_row(
            "SELECT phase FROM remote_resolution_index_artifacts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_ne!(phase, "released");
    let mut retry = request(root.path());
    retry.operation_id = operation;
    retry.restart = true;
    assert!(
        RepositoryService::open_at(data.path())
            .unwrap()
            .synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt))
            .is_err()
    );
    assert_eq!(local_binding_image(root.path()), before);
    assert_eq!(
        db.query_row(
            "SELECT phase FROM remote_resolution_index_artifacts",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        phase
    );
}

#[test]
fn recorded_candidate_fault_table_preserves_partial_checkout_and_observes_ref_once() {
    for point in [
        "before_checkout",
        "after_checkout",
        "after_ref",
        "observation_fault",
        "operator_lock",
    ] {
        let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
        let (root, data, service, operation, local, incoming) = resolution_fixture(&[(
            "docs/document.md",
            base,
            &base.replace("base", "local"),
            &base.replace("base", "incoming"),
        )]);
        let repository = git2::Repository::open(root.path()).unwrap();
        repository.cleanup_state().unwrap();
        repository
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        let signature = repository.signature().unwrap();
        let tree = repository.find_commit(incoming).unwrap().tree().unwrap();
        let candidate = repository
            .commit(
                None,
                &signature,
                &signature,
                "recorded clean candidate",
                &tree,
                &[
                    &repository.find_commit(local).unwrap(),
                    &repository.find_commit(incoming).unwrap(),
                ],
            )
            .unwrap();
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        // Restore the durable applying seam without regenerating its intent.
        db.execute(
            "UPDATE remote_integration_steps SET phase='applying',candidate_oid=?1,conflict_digest=NULL",
            [candidate.to_string()],
        )
        .unwrap();
        match point {
            "after_checkout" => repository
                .checkout_tree(
                    tree.as_object(),
                    Some(git2::build::CheckoutBuilder::new().force()),
                )
                .unwrap(),
            "after_ref" | "observation_fault" | "operator_lock" => {
                fast_forward(&repository, "refs/heads/main", local, candidate).unwrap();
            }
            _ => {}
        }
        if point == "observation_fault" {
            db.execute_batch("CREATE TRIGGER observation_fault BEFORE UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        }
        if point == "operator_lock" {
            fs::write(
                repository.path().join("index.lock"),
                b"operator-owned lock\n",
            )
            .unwrap();
        }
        let before = local_binding_image(root.path());
        let logs = || {
            let mut digest = blake3::Hasher::new();
            for name in ["logs/HEAD", "logs/refs/heads/main"] {
                digest.update(&fs::read(repository.path().join(name)).unwrap());
            }
            *digest.finalize().as_bytes()
        };
        let before_logs = logs();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let RemoteReservationOutcome::Reserved(owner) = service
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("fenced restart")
        };
        let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let reconcile = |evidence: &mut state::SynchronizationEvidence| {
            reconcile_pending_candidate(
                &service,
                root.path(),
                "main",
                &SynchronizationTarget::Primary,
                &owner,
                evidence,
            )
        };
        let result = reconcile(&mut evidence);
        if matches!(point, "after_checkout" | "operator_lock") {
            assert!(matches!(
                result,
                Err(SynchronizationError::RecoveryRequired)
            ));
            assert_eq!(
                repository.head().unwrap().target(),
                Some(if point == "after_checkout" {
                    local
                } else {
                    candidate
                })
            );
            assert_eq!(local_binding_image(root.path()), before);
            assert_eq!(logs(), before_logs);
            if point == "operator_lock" {
                assert!(repository.path().join("index.lock").exists());
            }
        } else if point == "observation_fault" {
            assert!(result.is_err());
            assert_eq!(local_binding_image(root.path()), before);
            assert_eq!(logs(), before_logs);
            db.execute_batch("DROP TRIGGER observation_fault").unwrap();
            assert_eq!(reconcile(&mut evidence).unwrap().unwrap().oid, candidate);
            assert_eq!(logs(), before_logs);
        } else {
            assert_eq!(result.unwrap().unwrap().oid, candidate);
            assert_eq!(repository.head().unwrap().target(), Some(candidate));
            let completed_logs = logs();
            assert_eq!(reconcile(&mut evidence).unwrap().unwrap().oid, candidate);
            assert_eq!(logs(), completed_logs);
            if point == "after_ref" {
                assert_eq!(completed_logs, before_logs);
                assert_eq!(local_binding_image(root.path()), before);
            }
        }
    }
}

fn child_file(repo: &git2::Repository, parent: git2::Oid, name: &str, bytes: &[u8]) -> git2::Oid {
    let parent = repo.find_commit(parent).unwrap();
    let mut builder = repo.treebuilder(Some(&parent.tree().unwrap())).unwrap();
    builder
        .insert(name, repo.blob(bytes).unwrap(), 0o100644)
        .unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(None, &sig, &sig, "fixture", &tree, &[&parent])
        .unwrap()
}

/// number, previous intent, previous advertisement, disposition, phase, candidate.
type PassAttemptRow = (i64, String, Option<String>, String, String, Option<String>);
/// MERGE_HEAD, MERGE_MSG and MERGE_MODE digests with the journal phase.
type RecordedMergeMetadata = (Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>, String);

/// A reserved primary synchronization whose local branch holds one real
/// commit (`local.txt` or a changed `fixture.txt`) above the shared base.
struct PassFixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    plan: RemoteRefPlan,
    target: RemoteOperationTarget,
    operation: OperationId,
    owner: RemoteReservation,
    base: git2::Oid,
    local: git2::Oid,
}

impl PassFixture {
    fn new(local_file: &str, local_bytes: &[u8]) -> Self {
        Self::build(local_file, local_bytes, false)
    }

    /// As `new`, with the publication remote also selected in the committed
    /// configuration and its endpoints bound, so the public entry point
    /// proceeds past local reconciliation to the transport boundary.
    fn published(local_file: &str, local_bytes: &[u8]) -> Self {
        Self::build(local_file, local_bytes, true)
    }

    fn build(local_file: &str, local_bytes: &[u8], published: bool) -> Self {
        let (root, data, service) = fixture();
        let repository = git2::Repository::open(root.path()).unwrap();
        repository
            .remote("origin", "ssh://example.invalid/fixture.git")
            .unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&plan), false)
        })
        .unwrap();
        if published {
            fs::write(
                root.path().join(".manyhands/config.toml"),
                "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
            )
            .unwrap();
            commit_all(&repository);
            let configuration = service
                .observation_configuration(root.path(), &plan)
                .unwrap();
            state::with_transaction(&service, root.path(), |tx, id| {
                state::configure_endpoints(tx, id, &plan, &configuration.endpoint_digest())
            })
            .unwrap();
        }
        let base = repository.head().unwrap().target().unwrap();
        fs::write(root.path().join(local_file), local_bytes).unwrap();
        let local = commit_all(&repository);
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let operation = OperationId::new();
        let RemoteReservationOutcome::Reserved(owner) = service
            .reserve_remote_operation(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("reservation")
        };
        service
            .checkpoint_synchronization(
                root.path(),
                &owner,
                state::SynchronizationCheckpoint::FetchPrepared,
                &state::SynchronizationEvidence {
                    expected_oid: Some(local),
                    local_oid: Some(local),
                    ..Default::default()
                },
            )
            .unwrap();
        Self {
            root,
            data,
            service,
            plan,
            target,
            operation,
            owner,
            base,
            local,
        }
    }

    fn repository(&self) -> git2::Repository {
        git2::Repository::open(self.root.path()).unwrap()
    }

    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    fn record(&self) -> state::StoredRemoteOperation {
        state::with_transaction(&self.service, self.root.path(), |tx, id| {
            state::read_operation(tx, id, self.operation)
        })
        .unwrap()
        .unwrap()
    }

    fn restart(&mut self) {
        let RemoteReservationOutcome::Reserved(owner) = self
            .service
            .restart_remote_synchronization(self.root.path(), self.operation, &self.target)
            .unwrap()
        else {
            panic!("explicit restart")
        };
        self.owner = owner;
    }

    /// Restart and hand back the fenced token of the previous owner epoch.
    fn restart_superseding(&mut self) -> RemoteReservation {
        let RemoteReservationOutcome::Reserved(owner) = self
            .service
            .restart_remote_synchronization(self.root.path(), self.operation, &self.target)
            .unwrap()
        else {
            panic!("explicit restart")
        };
        std::mem::replace(&mut self.owner, owner)
    }

    fn intent(&self) -> PublicationIntent {
        self.service
            .synchronization_publication_intent(self.root.path(), &self.owner)
            .unwrap()
    }

    fn settle(
        &self,
        settlement: &PublicationSettlement,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        self.service
            .settle_synchronization_publication(self.root.path(), &self.owner, settlement)
    }

    fn point(&self, point: RemoteOperationSafePoint) {
        self.service
            .remote_safe_point(self.root.path(), &self.owner, point)
            .unwrap();
    }

    fn prepare(
        &self,
        candidate: git2::Oid,
        advertised: Option<git2::Oid>,
        verified: bool,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        self.service.prepare_synchronization_publication(
            self.root.path(),
            &self.owner,
            candidate,
            advertised,
            verified,
        )
    }

    fn advance(
        &self,
        phase: state::PublicationPhase,
        advertised: Option<git2::Oid>,
    ) -> Result<RemoteSafePointOutcome, RepositoryError> {
        self.service.advance_synchronization_publication(
            self.root.path(),
            &self.owner,
            phase,
            advertised,
        )
    }

    fn authority(&self) -> state::SynchronizationAuthority {
        self.service
            .synchronization_publication_authority(self.root.path(), &self.owner)
            .unwrap()
    }

    /// One completed Fetch whose exact tracking ref equals the advertisement.
    fn fetch(&self, incoming: git2::Oid, observed_at: i64) {
        self.repository()
            .reference(
                self.plan.primary().tracking_ref(),
                incoming,
                true,
                "fixture observation",
            )
            .unwrap();
        self.service
            .remote_safe_point(
                self.root.path(),
                &self.owner,
                RemoteOperationSafePoint::BeforeFetch,
            )
            .unwrap();
        let observation = RemoteRefObservation::from_advertisement(
            &self.plan,
            "refs/heads/main",
            incoming,
            Some(incoming),
        )
        .unwrap();
        commit_observation_batch(
            &self.service,
            self.root.path(),
            &self.owner,
            &self.plan,
            &[observation],
            observed_at,
        )
        .unwrap();
    }

    fn append_window(
        &self,
        number: u32,
        incoming: git2::Oid,
    ) -> Result<state::IntegrationWindowEvidence, RepositoryError> {
        let batch = state::with_transaction(&self.service, self.root.path(), |tx, id| {
            tx.query_row(
                "SELECT id FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1",
                [id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| state::recovery_required())
        })
        .unwrap();
        self.service.prepare_synchronization_window(
            self.root.path(),
            &self.owner,
            number,
            &state::IntegrationWindowIntent {
                observation_batch_id: batch,
                local_oid: self.repository().head().unwrap().target().unwrap(),
                primary_oid: incoming,
                context_oid: None,
            },
        )
    }

    /// The production ordered-integration pass for the newest window.
    fn integrate(&self, incoming: git2::Oid) -> Result<git2::Oid, SynchronizationError> {
        let configuration = self
            .service
            .observation_configuration(self.root.path(), &self.plan)
            .unwrap();
        let mut req = request(self.root.path());
        req.operation_id = self.operation;
        integrate_divergence(
            &self.service,
            DivergenceInputs {
                root: self.root.path(),
                primary_branch: "main",
                target: &SynchronizationTarget::Primary,
                request: &req,
                owner: &self.owner,
                plan: &self.plan,
                configuration: &configuration,
                selected: self.plan.primary(),
                primary_tracking: Some(incoming),
                selected_tracking: Some(incoming),
                context: None,
                primary: incoming,
            },
        )
    }

    fn reconcile(
        &self,
        evidence: &mut state::SynchronizationEvidence,
    ) -> Result<Option<ReconciledCandidate>, SynchronizationError> {
        reconcile_pending_candidate(
            &self.service,
            self.root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &self.owner,
            evidence,
        )
    }

    fn attempts(&self) -> Vec<PassAttemptRow> {
        let db = self.db();
        let mut statement = db
            .prepare("SELECT number,previous_oid,previous_advertised_oid,previous_disposition,phase,candidate_oid FROM remote_publication_attempts ORDER BY number")
            .unwrap();
        statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    /// The legacy envelope's push-direction columns, which later publication
    /// attempts must never rewrite.
    fn legacy_push(&self) -> (String, Option<String>, Option<String>, Option<String>) {
        self.db()
            .query_row(
                "SELECT sync_checkpoint,local_oid,push_oid,push_advertised_oid FROM remote_operation_records WHERE operation_ulid=?1",
                [self.operation.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
    }
}

fn reflog_image(repository: &git2::Repository) -> [u8; 32] {
    let mut digest = blake3::Hasher::new();
    for name in ["logs/HEAD", "logs/refs/heads/main"] {
        digest.update(&fs::read(repository.path().join(name)).unwrap());
    }
    *digest.finalize().as_bytes()
}

/// D: a production conflict records this operation's own merge metadata. An
/// external whole-merge commit that leaves exactly that metadata behind is
/// accepted and the remnant retired; anything altered, unrecorded, foreign,
/// linked or locked is preserved byte-for-byte and stays Recovery.
#[test]
fn external_repair_retires_only_own_recorded_merge_metadata() {
    for variant in [
        "own_all",
        "own_after_partial_retirement",
        "altered_message",
        "unrecorded",
        "foreign_state",
        "operator_lock",
        "wrong_parents",
        "linked_member",
        "recreated_after_retirement",
        "hard_linked_member",
        "retire_intent_all_present",
        "retire_intent_none_present",
        "altered_before_lease",
    ] {
        let mut fixture = PassFixture::new("fixture.txt", b"local\n");
        let repository = fixture.repository();
        let local = fixture.local;
        let incoming = child(&repository, fixture.base, b"incoming\n");
        fixture.fetch(incoming, 1);
        fixture.append_window(1, incoming).unwrap();
        // The real ordered integration installs the conflict and, under the
        // same lease, records the digests of the metadata it just wrote.
        assert!(matches!(
            fixture.integrate(incoming),
            Err(SynchronizationError::ConflictPending { .. })
        ));
        let gitdir = repository.path().to_owned();
        let recorded: RecordedMergeMetadata = fixture
            .db()
            .query_row(
                "SELECT merge_head_digest,merge_msg_digest,merge_mode_digest,phase FROM remote_integration_merge_metadata",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(recorded.3, "recorded", "{variant}");
        assert_eq!(
            recorded.0,
            blake3::hash(&fs::read(gitdir.join("MERGE_HEAD")).unwrap()).as_bytes()
        );
        assert_eq!(
            recorded.1.as_deref(),
            Some(
                blake3::hash(&fs::read(gitdir.join("MERGE_MSG")).unwrap())
                    .as_bytes()
                    .as_slice()
            )
        );
        // An external tool commits the whole merge with the exact ordered
        // parents and a clean index/worktree, but never cleans up merge state.
        let signature = repository.signature().unwrap();
        let local_parent = repository.find_commit(local).unwrap();
        let incoming_parent = repository.find_commit(incoming).unwrap();
        let parents = if variant == "wrong_parents" {
            vec![&incoming_parent, &local_parent]
        } else {
            vec![&local_parent, &incoming_parent]
        };
        let repaired = repository
            .commit(
                None,
                &signature,
                &signature,
                "external repair",
                &local_parent.tree().unwrap(),
                &parents,
            )
            .unwrap();
        repository
            .reference("refs/heads/main", repaired, true, "external repair")
            .unwrap();
        repository
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        for member in RESOLUTION_MERGE_MEMBERS {
            assert!(gitdir.join(member).exists(), "{variant} {member}");
        }
        match variant {
            "own_after_partial_retirement" => {
                // A process that died between unlinks: absence is completion.
                fs::remove_file(gitdir.join("MERGE_HEAD")).unwrap();
            }
            "altered_message" => {
                fs::write(gitdir.join("MERGE_MSG"), b"edited by another tool\n").unwrap();
            }
            "unrecorded" => {
                // A conflict installed before this evidence existed.
                fixture
                    .db()
                    .execute("DELETE FROM remote_integration_merge_metadata", [])
                    .unwrap();
            }
            "foreign_state" => {
                fs::write(gitdir.join("CHERRY_PICK_HEAD"), format!("{incoming}\n")).unwrap();
            }
            "operator_lock" => {
                fs::write(gitdir.join("index.lock"), b"operator-owned lock\n").unwrap();
            }
            "linked_member" => {
                #[cfg(unix)]
                {
                    let bytes = fs::read(gitdir.join("MERGE_MSG")).unwrap();
                    let elsewhere = fixture.data.path().join("same-bytes-elsewhere");
                    fs::write(&elsewhere, bytes).unwrap();
                    fs::remove_file(gitdir.join("MERGE_MSG")).unwrap();
                    std::os::unix::fs::symlink(&elsewhere, gitdir.join("MERGE_MSG")).unwrap();
                }
                #[cfg(not(unix))]
                fs::write(gitdir.join("MERGE_MSG"), b"edited by another tool\n").unwrap();
            }
            "recreated_after_retirement" => {
                fixture
                    .db()
                    .execute(
                        "UPDATE remote_integration_merge_metadata SET phase='retired'",
                        [],
                    )
                    .unwrap();
            }
            "hard_linked_member" => {
                // Same inode reachable through a name this operation never
                // wrote: not owned, even though the bytes match.
                #[cfg(unix)]
                fs::hard_link(
                    gitdir.join("MERGE_MSG"),
                    fixture.data.path().join("second-link"),
                )
                .unwrap();
                #[cfg(not(unix))]
                fs::write(gitdir.join("MERGE_MSG"), b"edited by another tool\n").unwrap();
            }
            "retire_intent_all_present" | "retire_intent_none_present" => {
                // A holder that journaled its intent and died before, or
                // after, every unlink.
                fixture
                    .db()
                    .execute(
                        "UPDATE remote_integration_merge_metadata SET phase='retire_intent'",
                        [],
                    )
                    .unwrap();
                if variant == "retire_intent_none_present" {
                    for member in RESOLUTION_MERGE_MEMBERS {
                        fs::remove_file(gitdir.join(member)).unwrap();
                    }
                }
            }
            _ => {}
        }
        let members = || {
            RESOLUTION_MERGE_MEMBERS
                .iter()
                .chain(["CHERRY_PICK_HEAD", "index.lock"].iter())
                .map(|member| {
                    fs::symlink_metadata(gitdir.join(member))
                        .ok()
                        .map(|metadata| {
                            (
                                metadata.file_type().is_symlink(),
                                fs::read(gitdir.join(member)).unwrap(),
                            )
                        })
                })
                .collect::<Vec<_>>()
        };
        let mut before_members = members();
        let before = local_binding_image(fixture.root.path());
        let before_logs = reflog_image(&repository);
        let before_objects = inspection_odb_inventory(&repository);
        if variant == "altered_before_lease" {
            // Changed after the outside observation accepted the remnant and
            // before the lease: ownership is decided again under the lease.
            let member = gitdir.join("MERGE_MSG");
            set_merge_metadata_observed_hook(fixture.root.path().to_path_buf(), move || {
                fs::write(member, b"edited during validation\n").unwrap();
            });
            before_members[1] = Some((false, b"edited during validation\n".to_vec()));
        }
        fixture.restart();
        let mut evidence = fixture.record().sync_evidence;
        let result = fixture.reconcile(&mut evidence);
        let step = state::with_transaction(&fixture.service, fixture.root.path(), |tx, id| {
            state::integration_step_in_window(
                tx,
                state::read_operation(tx, id, fixture.operation)?
                    .unwrap()
                    .id,
                1,
                0,
            )
        })
        .unwrap()
        .unwrap();
        let phase = fixture
            .db()
            .query_row(
                "SELECT phase FROM remote_integration_merge_metadata",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();
        // Never a ref, ref-log, index, worktree or object effect either way.
        assert_eq!(
            local_binding_image(fixture.root.path()),
            before,
            "{variant}"
        );
        assert_eq!(reflog_image(&repository), before_logs, "{variant}");
        assert_eq!(
            inspection_odb_inventory(&repository),
            before_objects,
            "{variant}"
        );
        assert_eq!(repository.head().unwrap().target(), Some(repaired));
        if matches!(
            variant,
            "own_all"
                | "own_after_partial_retirement"
                | "retire_intent_all_present"
                | "retire_intent_none_present"
        ) {
            assert_eq!(result.unwrap().unwrap().oid, repaired, "{variant}");
            for member in RESOLUTION_MERGE_MEMBERS {
                assert!(
                    fs::symlink_metadata(gitdir.join(member)).is_err(),
                    "{variant} {member}"
                );
            }
            assert_eq!(phase.as_deref(), Some("retired"), "{variant}");
            assert_eq!(step.phase, state::IntegrationStepPhase::Applied);
            assert_eq!(step.result_oid, Some(repaired));
            assert_eq!(step.intent.local_oid, local);
            assert_eq!(step.intent.incoming_oid, incoming);
            assert_eq!(
                git2::Repository::open(fixture.root.path()).unwrap().state(),
                git2::RepositoryState::Clean
            );
            // The journal itself refuses moving a retirement backwards.
            for earlier in ["recorded", "retire_intent"] {
                assert!(
                    fixture
                        .db()
                        .execute(
                            "UPDATE remote_integration_merge_metadata SET phase=?1",
                            [earlier]
                        )
                        .is_err(),
                    "{variant} {earlier}"
                );
            }
            // Observation is idempotent: no second retirement or ref effect.
            assert_eq!(
                fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
                repaired
            );
            assert_eq!(reflog_image(&repository), before_logs);
        } else {
            assert!(
                matches!(result, Err(SynchronizationError::RecoveryRequired)),
                "{variant}"
            );
            assert_eq!(members(), before_members, "{variant}");
            assert_eq!(
                step.phase,
                state::IntegrationStepPhase::ConflictPending,
                "{variant}"
            );
            assert_eq!(step.result_oid, None, "{variant}");
            assert_eq!(
                phase.as_deref(),
                match variant {
                    "unrecorded" => None,
                    "recreated_after_retirement" => Some("retired"),
                    _ => Some("recorded"),
                },
                "{variant}"
            );
        }
    }
}

/// E: a merge candidate whose Push intent lost a remote race. The old intent
/// is reconciled first from Push-direction evidence and stays immutable; the
/// continuation integrates one new window and publishes through an appended
/// attempt without overwriting `push_oid` or replaying the legacy checkpoints.
#[test]
fn displaced_push_intent_continues_through_an_appended_publication_attempt() {
    let mut fixture = PassFixture::new("local.txt", b"local\n");
    let repository = fixture.repository();
    let first = child_file(&repository, fixture.base, "remote-one.txt", b"one\n");
    fixture.fetch(first, 1);
    fixture.append_window(1, first).unwrap();
    let merged = fixture.integrate(first).unwrap();
    let merge = repository.find_commit(merged).unwrap();
    assert_eq!(
        [merge.parent_id(0).unwrap(), merge.parent_id(1).unwrap()],
        [fixture.local, first]
    );
    let mut evidence = state::SynchronizationEvidence {
        expected_oid: Some(fixture.local),
        local_oid: Some(merged),
        tracking_oid: Some(first),
        primary_tracking_oid: Some(first),
        ..Default::default()
    };
    fixture
        .service
        .checkpoint_synchronization_merge_applied(fixture.root.path(), &fixture.owner, &evidence)
        .unwrap();
    // The original durable Push intent for the merge candidate.
    evidence.push_oid = Some(merged);
    evidence.push_advertised_oid = Some(first);
    fixture
        .service
        .checkpoint_synchronization(
            fixture.root.path(),
            &fixture.owner,
            state::SynchronizationCheckpoint::PushPrepared,
            &evidence,
        )
        .unwrap();
    fixture
        .service
        .remote_safe_point(
            fixture.root.path(),
            &fixture.owner,
            RemoteOperationSafePoint::BeforePush,
        )
        .unwrap();
    let legacy = fixture.legacy_push();
    assert_eq!(
        legacy,
        (
            "push_prepared".into(),
            Some(merged.to_string()),
            Some(merged.to_string()),
            Some(first.to_string())
        )
    );
    // The receiver rejected that push: another writer advanced the remote.
    let second = child_file(&repository, first, "remote-two.txt", b"two\n");
    fixture.restart();
    let mut restarted = fixture.record().sync_evidence;
    let observed = fixture.reconcile(&mut restarted).unwrap().unwrap();
    assert_eq!(observed.oid, merged);
    fixture.fetch(second, 2);
    // Local handoff can never clear the recorded intent, and no new pass may
    // start before the old push is reconciled.
    assert!(
        finalize_reconciled_candidate(
            &fixture.service,
            fixture.root.path(),
            &fixture.owner,
            observed,
            &restarted
        )
        .is_err()
    );
    assert!(fixture.append_window(2, second).is_err());
    assert_eq!(
        fixture
            .service
            .synchronization_publication_intent(fixture.root.path(), &fixture.owner)
            .unwrap(),
        PublicationIntent::Legacy(merged)
    );
    let relation = push_intent_relation(&repository, merged, Some(second)).unwrap();
    assert_eq!(relation, PushIntentRelation::Diverged);
    let settlement = PublicationSettlement {
        intent: PublicationIntent::Legacy(merged),
        local_oid: merged,
        continuation: false,
        advertised_oid: Some(second),
        relation: Some(relation),
    };
    // A stale intent, an unproved continuation or a mismatched relation is refused.
    for refused in [
        PublicationSettlement {
            intent: PublicationIntent::Legacy(first),
            ..settlement
        },
        PublicationSettlement {
            continuation: true,
            ..settlement
        },
        PublicationSettlement {
            local_oid: second,
            ..settlement
        },
        PublicationSettlement {
            relation: Some(PushIntentRelation::Absent),
            ..settlement
        },
        PublicationSettlement {
            relation: Some(PushIntentRelation::Equal),
            ..settlement
        },
    ] {
        assert!(
            fixture
                .service
                .settle_synchronization_publication(fixture.root.path(), &fixture.owner, &refused)
                .is_err()
        );
        assert!(fixture.attempts().is_empty());
    }
    assert_eq!(
        fixture
            .service
            .settle_synchronization_publication(fixture.root.path(), &fixture.owner, &settlement)
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(fixture.legacy_push(), legacy);
    assert_eq!(
        fixture.attempts(),
        vec![(
            1,
            merged.to_string(),
            Some(second.to_string()),
            "displaced".into(),
            "open".into(),
            None
        )]
    );
    let record = fixture.record();
    assert_eq!(record.phase, RemoteOperationPhase::PushPrepared);
    assert!(!record.reconciliation_required);
    // No push boundary and no legacy checkpoint is available to an open attempt.
    assert!(
        fixture
            .service
            .remote_safe_point(
                fixture.root.path(),
                &fixture.owner,
                RemoteOperationSafePoint::BeforePush
            )
            .is_err()
    );
    let mut overwritten = record.sync_evidence.clone();
    overwritten.push_oid = Some(second);
    overwritten.local_oid = Some(second);
    for checkpoint in [
        state::SynchronizationCheckpoint::LocalPrepared,
        state::SynchronizationCheckpoint::PushPrepared,
        state::SynchronizationCheckpoint::PushVerified,
    ] {
        assert!(
            fixture
                .service
                .checkpoint_synchronization(
                    fixture.root.path(),
                    &fixture.owner,
                    checkpoint,
                    &overwritten
                )
                .is_err()
        );
    }
    // Freeze: even the UNCHANGED recorded evidence, or the old intent now
    // observed as advertised, can no longer move a legacy checkpoint.
    let frozen = record.sync_evidence.clone();
    let mut reverified = frozen.clone();
    reverified.push_advertised_oid = reverified.push_oid;
    for (checkpoint, evidence) in [
        (state::SynchronizationCheckpoint::PushPrepared, &frozen),
        (state::SynchronizationCheckpoint::PushReturned, &frozen),
        (state::SynchronizationCheckpoint::PushVerified, &frozen),
        (state::SynchronizationCheckpoint::PushVerified, &reverified),
    ] {
        assert!(
            fixture
                .service
                .checkpoint_synchronization(
                    fixture.root.path(),
                    &fixture.owner,
                    checkpoint,
                    evidence
                )
                .is_err()
        );
    }
    assert!(
        fixture
            .service
            .reconcile_synchronization(
                fixture.root.path(),
                &fixture.owner,
                merged,
                merged,
                Some(second),
                false
            )
            .is_err()
    );
    assert_eq!(fixture.legacy_push(), legacy);
    // One new window: the earlier merge is the first parent, never regenerated.
    fixture.append_window(2, second).unwrap();
    let continued = fixture.integrate(second).unwrap();
    let continuation = repository.find_commit(continued).unwrap();
    assert_eq!(
        [
            continuation.parent_id(0).unwrap(),
            continuation.parent_id(1).unwrap()
        ],
        [merged, second]
    );
    let mut working = fixture.record().sync_evidence;
    working.local_oid = Some(continued);
    assert_eq!(
        fixture
            .service
            .checkpoint_synchronization_merge_applied(fixture.root.path(), &fixture.owner, &working)
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(fixture.legacy_push(), legacy);
    // The candidate must be the newest applied window, and a verified claim
    // needs an equal advertisement.
    for (candidate, advertised, verified) in [
        (merged, Some(second), false),
        (continued, Some(second), true),
        (continued, Some(continued), false),
    ] {
        assert!(
            fixture
                .service
                .prepare_synchronization_publication(
                    fixture.root.path(),
                    &fixture.owner,
                    candidate,
                    advertised,
                    verified
                )
                .is_err()
        );
    }
    assert_eq!(
        fixture
            .service
            .prepare_synchronization_publication(
                fixture.root.path(),
                &fixture.owner,
                continued,
                Some(second),
                false
            )
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    // Verification cannot skip the returned push or name another OID.
    assert!(
        fixture
            .service
            .advance_synchronization_publication(
                fixture.root.path(),
                &fixture.owner,
                state::PublicationPhase::Returned,
                None
            )
            .is_err()
    );
    fixture
        .service
        .remote_safe_point(
            fixture.root.path(),
            &fixture.owner,
            RemoteOperationSafePoint::BeforePush,
        )
        .unwrap();
    fixture
        .service
        .advance_synchronization_publication(
            fixture.root.path(),
            &fixture.owner,
            state::PublicationPhase::Returned,
            None,
        )
        .unwrap();
    fixture
        .service
        .remote_safe_point(
            fixture.root.path(),
            &fixture.owner,
            RemoteOperationSafePoint::AfterPushReturn,
        )
        .unwrap();
    assert!(
        fixture
            .service
            .advance_synchronization_publication(
                fixture.root.path(),
                &fixture.owner,
                state::PublicationPhase::Verified,
                Some(second)
            )
            .is_err()
    );
    assert!(
        fixture
            .service
            .classify_synchronization(
                fixture.root.path(),
                &fixture.owner,
                state::SynchronizationAuthority::Published(continued)
            )
            .is_err()
    );
    fixture
        .service
        .advance_synchronization_publication(
            fixture.root.path(),
            &fixture.owner,
            state::PublicationPhase::Verified,
            Some(continued),
        )
        .unwrap();
    fixture
        .service
        .remote_safe_point(
            fixture.root.path(),
            &fixture.owner,
            RemoteOperationSafePoint::AfterPushVerification,
        )
        .unwrap();
    // The superseded legacy intent can never become the authority.
    assert!(
        fixture
            .service
            .classify_synchronization(
                fixture.root.path(),
                &fixture.owner,
                state::SynchronizationAuthority::Published(merged)
            )
            .is_err()
    );
    fixture
        .service
        .classify_synchronization(
            fixture.root.path(),
            &fixture.owner,
            state::SynchronizationAuthority::Published(continued),
        )
        .unwrap();
    let record = fixture.record();
    assert_eq!(
        record.authority,
        Some(state::SynchronizationAuthority::Published(continued))
    );
    assert_eq!(record.sync_evidence.push_oid, Some(merged));
    assert_eq!(record.sync_evidence.push_advertised_oid, Some(first));
    assert_eq!(record.sync_evidence.local_oid, Some(merged));
    assert_eq!(
        fixture.attempts(),
        vec![(
            1,
            merged.to_string(),
            Some(second.to_string()),
            "displaced".into(),
            "verified".into(),
            Some(continued.to_string())
        )]
    );
    // Append-only: SQLite itself refuses rewriting an intent or moving back.
    let db = fixture.db();
    for statement in [
        "UPDATE remote_publication_attempts SET candidate_oid=previous_oid",
        "UPDATE remote_publication_attempts SET previous_oid=candidate_oid",
        "UPDATE remote_publication_attempts SET previous_disposition='accepted'",
        "UPDATE remote_publication_attempts SET phase='returned',advertised_oid=NULL",
        "UPDATE remote_publication_attempts SET window_number=1",
    ] {
        assert!(db.execute(statement, []).is_err(), "{statement}");
    }
    // Losing the attempt leaves an authority the frozen legacy columns cannot
    // prove: the envelope fails closed instead of being reinterpreted.
    db.execute("DELETE FROM remote_publication_attempts", [])
        .unwrap();
    assert!(
        state::with_transaction(&fixture.service, fixture.root.path(), |tx, id| {
            state::read_operation(tx, id, fixture.operation)
        })
        .is_err()
    );
    assert_eq!(repository.head().unwrap().target(), Some(continued));
    assert!(repository.statuses(None).unwrap().is_empty());
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_integration_steps", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        2
    );
}

/// E: a released local checkpoint already has a Push intent, and a clean
/// descendant was committed on top of it afterwards. The old intent is
/// reconciled as accepted from Push-direction evidence and kept verbatim; the
/// descendant publishes through an appended attempt bound to a new window,
/// while the original checkpoint candidate is never changed or replayed.
#[test]
fn released_checkpoint_descendant_after_old_push_intent_opens_a_continuation_attempt() {
    let (root, data, service, operation, worktree, _, incoming, ticket_id) =
        context_primary_conflict_fixture();
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let result = service
        .read_synchronization_conflict(&inspection.paths[0].token)
        .unwrap()
        .local
        .unwrap();
    let ResolveSynchronizationOutcome::LocalCheckpointComplete {
        commit_oid: checkpoint,
    } = service
        .resolve_synchronization(ResolveSynchronizationRequest::new(
            root.path().into(),
            operation,
            OperationId::new(),
            inspection.observation,
            vec![(inspection.paths[0].token.clone(), result)],
            None,
        ))
        .unwrap()
    else {
        panic!("released checkpoint")
    };
    let repository = git2::Repository::open(&worktree).unwrap();
    git2::Repository::open(root.path())
        .unwrap()
        .remote("origin", "ssh://example.invalid/fixture.git")
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    repository
        .reference(plan.primary().tracking_ref(), incoming, true, "fixture")
        .unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        ticket_id.clone(),
    )
    .unwrap();
    let sync_target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ticket_id,
    };
    let service = RepositoryService::open_at(data.path()).unwrap();
    let restart = || {
        let RemoteReservationOutcome::Reserved(owner) = service
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("original operation")
        };
        owner
    };
    let record = || {
        state::with_transaction(&service, root.path(), |tx, id| {
            state::read_operation(tx, id, operation)
        })
        .unwrap()
        .unwrap()
    };
    let fetch = |owner: &RemoteReservation, observed_at: i64| {
        service
            .remote_safe_point(root.path(), owner, RemoteOperationSafePoint::BeforeFetch)
            .unwrap();
        let observation = RemoteRefObservation::from_advertisement(
            &plan,
            "refs/heads/main",
            incoming,
            Some(incoming),
        )
        .unwrap();
        commit_observation_batch(
            &service,
            root.path(),
            owner,
            &plan,
            &[observation],
            observed_at,
        )
        .unwrap();
    };
    let reconcile = |owner: &RemoteReservation, evidence: &mut state::SynchronizationEvidence| {
        reconcile_pending_candidate(&service, root.path(), "main", &sync_target, owner, evidence)
    };
    // First deliberate invocation: hand off the released checkpoint and
    // durably record its Push intent, whose outcome is then unknown.
    let owner = restart();
    let mut evidence = record().sync_evidence;
    let observed = reconcile(&owner, &mut evidence).unwrap().unwrap();
    assert_eq!(observed.oid, checkpoint);
    fetch(&owner, 2);
    evidence.primary_tracking_oid = Some(incoming);
    evidence.tracking_oid = None;
    finalize_reconciled_candidate(&service, root.path(), &owner, observed, &evidence).unwrap();
    evidence.push_oid = Some(checkpoint);
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::PushPrepared,
            &evidence,
        )
        .unwrap();
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
        .unwrap();
    let legacy = |db: &rusqlite::Connection| {
        db.query_row(
            "SELECT sync_checkpoint,local_oid,push_oid,push_advertised_oid FROM remote_operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .unwrap()
    };
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let frozen = legacy(&db);
    assert_eq!(
        frozen,
        (
            "push_prepared".into(),
            Some(checkpoint.to_string()),
            Some(checkpoint.to_string()),
            None
        )
    );
    // A later clean commit continues from the released checkpoint.
    fs::write(worktree.join("notes.txt"), b"later ordinary work\n").unwrap();
    let descendant = commit_all(&repository);
    assert!(
        repository
            .graph_descendant_of(descendant, checkpoint)
            .unwrap()
    );
    let before = local_binding_image(&worktree);
    // Second deliberate invocation.
    let owner = restart();
    let mut evidence = record().sync_evidence;
    let observed = reconcile(&owner, &mut evidence).unwrap().unwrap();
    assert_eq!(observed.oid, checkpoint);
    assert_eq!(evidence.local_oid, Some(descendant));
    fetch(&owner, 3);
    assert!(
        finalize_reconciled_candidate(&service, root.path(), &owner, observed, &evidence).is_err()
    );
    assert_eq!(
        service
            .synchronization_publication_intent(root.path(), &owner)
            .unwrap(),
        PublicationIntent::Legacy(checkpoint)
    );
    // The receiver had accepted the checkpoint: fresh Push evidence equals it.
    let settlement = PublicationSettlement {
        intent: PublicationIntent::Legacy(checkpoint),
        local_oid: descendant,
        continuation: true,
        advertised_oid: Some(checkpoint),
        relation: Some(push_intent_relation(&repository, checkpoint, Some(checkpoint)).unwrap()),
    };
    // A moved HEAD without the validated released-descendant proof is refused.
    assert!(
        service
            .settle_synchronization_publication(
                root.path(),
                &owner,
                &PublicationSettlement {
                    continuation: false,
                    ..settlement
                }
            )
            .is_err()
    );
    assert_eq!(
        service
            .settle_synchronization_publication(root.path(), &owner, &settlement)
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(legacy(&db), frozen);
    let attempt = || {
        db.query_row(
            "SELECT number,previous_oid,previous_advertised_oid,previous_disposition,local_oid,window_number,candidate_oid,phase FROM remote_publication_attempts",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .unwrap()
    };
    assert_eq!(
        attempt(),
        (
            1,
            checkpoint.to_string(),
            Some(checkpoint.to_string()),
            "accepted".into(),
            descendant.to_string(),
            None,
            None,
            "open".into()
        )
    );
    // One new window starts at the descendant; the released stage is untouched.
    let batch = state::with_transaction(&service, root.path(), |tx, id| {
        tx.query_row(
            "SELECT id FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1",
            [id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| state::recovery_required())
    })
    .unwrap();
    service
        .prepare_synchronization_window(
            root.path(),
            &owner,
            1,
            &state::IntegrationWindowIntent {
                observation_batch_id: batch,
                local_oid: descendant,
                primary_oid: incoming,
                context_oid: None,
            },
        )
        .unwrap();
    let configuration = service
        .observation_configuration(root.path(), &plan)
        .unwrap();
    let mut req = request(root.path());
    req.operation_id = operation;
    req.target = sync_target.clone();
    let selected = target_ref(&plan, &sync_target);
    let integrated = integrate_divergence(
        &service,
        DivergenceInputs {
            root: root.path(),
            primary_branch: "main",
            target: &sync_target,
            request: &req,
            owner: &owner,
            plan: &plan,
            configuration: &configuration,
            selected: &selected,
            primary_tracking: Some(incoming),
            selected_tracking: None,
            context: None,
            primary: incoming,
        },
    )
    .unwrap();
    assert_eq!(integrated, descendant);
    let mut working = record().sync_evidence;
    working.local_oid = Some(descendant);
    service
        .checkpoint_synchronization_merge_applied(root.path(), &owner, &working)
        .unwrap();
    // The superseded checkpoint itself can no longer be bound as a candidate.
    assert!(
        service
            .prepare_synchronization_publication(
                root.path(),
                &owner,
                checkpoint,
                Some(checkpoint),
                true
            )
            .is_err()
    );
    assert_eq!(
        service
            .prepare_synchronization_publication(
                root.path(),
                &owner,
                descendant,
                Some(checkpoint),
                false
            )
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    assert_eq!(
        attempt(),
        (
            1,
            checkpoint.to_string(),
            Some(checkpoint.to_string()),
            "accepted".into(),
            descendant.to_string(),
            Some(1),
            Some(descendant.to_string()),
            "prepared".into()
        )
    );
    assert_eq!(legacy(&db), frozen);
    let (released, continued) = state::with_transaction(&service, root.path(), |tx, id| {
        let record = state::read_operation(tx, id, operation)?.unwrap();
        Ok((
            state::integration_step_in_window(tx, record.id, 0, 1)?.unwrap(),
            state::integration_step_in_window(tx, record.id, 1, 1)?.unwrap(),
        ))
    })
    .unwrap();
    assert_eq!(released.candidate_oid, Some(checkpoint));
    assert_eq!(released.result_oid, Some(checkpoint));
    assert_eq!(continued.intent.local_oid, descendant);
    assert_eq!(continued.result_oid, Some(descendant));
    assert_eq!(continued.candidate_oid, None);
    assert_eq!(local_binding_image(&worktree), before);
    assert_eq!(repository.head().unwrap().target(), Some(descendant));
    // F: while the newest attempt is unverified, an absent remote context is
    // ambiguous; once it is verified, absence is deletion, exactly as after a
    // legacy PushVerified checkpoint.
    let boundary = || {
        service
            .synchronization_push_absence_boundary(root.path(), &owner, true)
            .unwrap()
    };
    assert!(matches!(boundary(), Some(PushAbsenceBoundary::Ambiguous)));
    service
        .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforePush)
        .unwrap();
    service
        .advance_synchronization_publication(
            root.path(),
            &owner,
            state::PublicationPhase::Returned,
            None,
        )
        .unwrap();
    assert!(matches!(boundary(), Some(PushAbsenceBoundary::Ambiguous)));
    service
        .remote_safe_point(
            root.path(),
            &owner,
            RemoteOperationSafePoint::AfterPushReturn,
        )
        .unwrap();
    service
        .advance_synchronization_publication(
            root.path(),
            &owner,
            state::PublicationPhase::Verified,
            Some(descendant),
        )
        .unwrap();
    assert!(matches!(boundary(), Some(PushAbsenceBoundary::Deleted)));
    assert!(matches!(
        boundary().unwrap().error(),
        SynchronizationError::RemoteContextDeleted
    ));
    assert_eq!(
        service
            .synchronization_publication_authority(root.path(), &owner)
            .unwrap(),
        state::SynchronizationAuthority::Published(descendant)
    );
    assert_eq!(legacy(&db), frozen);
}

/// D: a context target's own gitdir is the linked worktree's. Merge metadata
/// in the shared common directory belongs to another worktree and is never
/// tolerated, retired or adopted, even beside this operation's own remnant.
#[test]
fn external_repair_in_linked_worktree_never_claims_commondir_merge_metadata() {
    for foreign_commondir_merge in [false, true] {
        let (root, data, service, operation, worktree, local, incoming, ticket_id) =
            context_primary_conflict_fixture();
        let repository = git2::Repository::open(&worktree).unwrap();
        let gitdir = repository.path().to_owned();
        let commondir = repository.commondir().to_owned();
        assert_ne!(gitdir, commondir);
        // The shared fixture installs its conflict without the production
        // pass, so journal the digests of the metadata that merge wrote.
        let digest = |member: &str| {
            fs::read(gitdir.join(member))
                .ok()
                .map(|bytes| blake3::hash(&bytes).as_bytes().to_vec())
        };
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute(
            "INSERT INTO remote_integration_merge_metadata(integration_step_id,merge_head_digest,merge_msg_digest,merge_mode_digest,phase) SELECT id,?1,?2,?3,'recorded' FROM remote_integration_steps WHERE stage='primary'",
            rusqlite::params![
                digest("MERGE_HEAD").unwrap(),
                digest("MERGE_MSG"),
                digest("MERGE_MODE")
            ],
        )
        .unwrap();
        let signature = repository.signature().unwrap();
        let local_parent = repository.find_commit(local).unwrap();
        let repaired = repository
            .commit(
                None,
                &signature,
                &signature,
                "external repair",
                &local_parent.tree().unwrap(),
                &[&local_parent, &repository.find_commit(incoming).unwrap()],
            )
            .unwrap();
        let branch = repository
            .find_reference("HEAD")
            .unwrap()
            .symbolic_target()
            .unwrap()
            .to_owned();
        repository
            .reference(&branch, repaired, true, "external repair")
            .unwrap();
        repository
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        assert!(gitdir.join("MERGE_HEAD").exists());
        if foreign_commondir_merge {
            // Byte-identical to this operation's own MERGE_HEAD, but it lives
            // in the primary worktree's gitdir.
            fs::copy(gitdir.join("MERGE_HEAD"), commondir.join("MERGE_HEAD")).unwrap();
        }
        let images = || {
            [&gitdir, &commondir].map(|directory| {
                RESOLUTION_MERGE_MEMBERS.map(|member| fs::read(directory.join(member)).ok())
            })
        };
        let before_members = images();
        let before = local_binding_image(&worktree);
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_context(
            &plan,
            RemoteOperationAction::SynchronizeContext,
            AuthoringKind::Ticket,
            ticket_id.clone(),
        )
        .unwrap();
        let RemoteReservationOutcome::Reserved(owner) = service
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("original operation")
        };
        let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let result = reconcile_pending_candidate(
            &service,
            root.path(),
            "main",
            &SynchronizationTarget::Context {
                kind: AuthoringKind::Ticket,
                item_id: ticket_id,
            },
            &owner,
            &mut evidence,
        );
        let phase: String = db
            .query_row(
                "SELECT phase FROM remote_integration_merge_metadata",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(local_binding_image(&worktree), before);
        if foreign_commondir_merge {
            assert!(matches!(
                result,
                Err(SynchronizationError::RecoveryRequired)
            ));
            assert_eq!(images(), before_members);
            assert_eq!(phase, "recorded");
        } else {
            assert_eq!(result.unwrap().unwrap().oid, repaired);
            assert_eq!(images(), [[None, None, None], [None, None, None]]);
            assert_eq!(phase, "retired");
        }
    }
}

/// An open publication attempt whose newest window is applied, ready to bind
/// `candidate`. With `advertised`, it is reached through the real settlement
/// of a displaced legacy intent and `tip` is the Push/Fetch tip merged into
/// the candidate. Without, the legacy intent was never advertised anywhere.
fn open_attempt_fixture(advertised: bool) -> (PassFixture, git2::Oid, git2::Oid) {
    let mut fixture = PassFixture::new("local.txt", b"local\n");
    let repository = fixture.repository();
    let (base, local) = (fixture.base, fixture.local);
    fixture.fetch(base, 1);
    let evidence = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(local),
        tracking_oid: Some(base),
        primary_tracking_oid: Some(base),
        push_oid: Some(local),
        push_advertised_oid: advertised.then_some(base),
    };
    fixture
        .service
        .checkpoint_synchronization(
            fixture.root.path(),
            &fixture.owner,
            state::SynchronizationCheckpoint::PushPrepared,
            &evidence,
        )
        .unwrap();
    fixture.restart();
    let tip = if advertised {
        let tip = child_file(&repository, base, "remote-other.txt", b"other\n");
        fixture.fetch(tip, 2);
        fixture
            .settle(&PublicationSettlement {
                intent: PublicationIntent::Legacy(local),
                local_oid: local,
                continuation: false,
                advertised_oid: Some(tip),
                relation: Some(PushIntentRelation::Diverged),
            })
            .unwrap();
        tip
    } else {
        fixture.fetch(base, 2);
        // No reservation transition opens an attempt for a never-advertised
        // intent that is still HEAD (that stays on the exact-retry route; only
        // a validated released descendant gets here). Journal that state
        // directly: not accepted, with no advertisement anywhere.
        state::with_transaction(&fixture.service, fixture.root.path(), |tx, id| {
            let record = state::read_operation(tx, id, fixture.operation)?.unwrap();
            state::open_publication_attempt(
                tx,
                &record,
                local,
                None,
                state::PublicationDisposition::NotAccepted,
                local,
            )?;
            tx.execute(
                "UPDATE remote_operation_records SET phase='push_prepared',reconciliation_required=0 WHERE id=?1",
                [record.id],
            )
            .map_err(|_| state::recovery_required())?;
            Ok(())
        })
        .unwrap();
        base
    };
    fixture.append_window(1, tip).unwrap();
    let candidate = fixture.integrate(tip).unwrap();
    let mut working = fixture.record().sync_evidence;
    working.local_oid = Some(candidate);
    fixture
        .service
        .checkpoint_synchronization_merge_applied(fixture.root.path(), &fixture.owner, &working)
        .unwrap();
    assert_eq!(fixture.intent(), PublicationIntent::Open);
    (fixture, candidate, tip)
}

/// E/F: reconciliation of an intent that already lives in a publication
/// attempt, and of a verified attempt interrupted before classification.
#[test]
fn attempt_intent_reconciliation_table() {
    for variant in [
        "equal_verifies_same_attempt",
        "behind_reopens_then_publishes",
        "verified_restart_before_classification",
        "absent_after_recorded_advertisement",
        "absent_never_advertised",
    ] {
        let advertised = variant != "absent_never_advertised";
        let (mut fixture, candidate, tip) = open_attempt_fixture(advertised);
        let repository = fixture.repository();
        let legacy = fixture.legacy_push();
        let directly_verified = variant == "verified_restart_before_classification";
        if directly_verified {
            // The endpoint already advertised the candidate: no push intent.
            assert_eq!(
                fixture.prepare(candidate, Some(candidate), true).unwrap(),
                RemoteSafePointOutcome::Continue
            );
            assert_eq!(
                fixture.authority(),
                state::SynchronizationAuthority::AlreadyCurrent(candidate)
            );
        } else {
            let before_push = advertised.then_some(tip);
            assert_eq!(
                fixture.prepare(candidate, before_push, false).unwrap(),
                RemoteSafePointOutcome::Continue
            );
            // SQLite itself refuses a verified row without an advertisement.
            assert!(
                fixture
                    .db()
                    .execute(
                        "UPDATE remote_publication_attempts SET phase='verified',advertised_oid=NULL",
                        []
                    )
                    .is_err()
            );
            fixture.point(RemoteOperationSafePoint::BeforePush);
            if variant == "equal_verifies_same_attempt" {
                fixture
                    .advance(state::PublicationPhase::Returned, None)
                    .unwrap();
            }
        }
        let rows = fixture.attempts();
        assert_eq!(rows.len(), 1, "{variant}");
        let before = local_binding_image(fixture.root.path());
        // The process stops here; a later deliberate invocation restarts.
        fixture.restart();
        let mut evidence = fixture.record().sync_evidence;
        assert_eq!(
            fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
            candidate
        );
        fixture.fetch(tip, 3);
        let stored = if directly_verified {
            PublicationIntent::Verified(candidate)
        } else {
            PublicationIntent::Attempt(candidate)
        };
        assert_eq!(fixture.intent(), stored, "{variant}");
        let settlement = |advertised_oid: Option<git2::Oid>| PublicationSettlement {
            intent: stored,
            local_oid: candidate,
            continuation: false,
            advertised_oid,
            relation: Some(push_intent_relation(&repository, candidate, advertised_oid).unwrap()),
        };
        match variant {
            "equal_verifies_same_attempt" => {
                // The receiver had accepted the push whose status was lost.
                assert_eq!(
                    fixture.settle(&settlement(Some(candidate))).unwrap(),
                    RemoteSafePointOutcome::Continue
                );
                let after = fixture.attempts();
                assert_eq!(after.len(), 1);
                assert_eq!(after[0].4, "verified");
                assert_eq!(after[0].5, Some(candidate.to_string()));
                assert_eq!(fixture.intent(), PublicationIntent::Verified(candidate));
                assert_eq!(
                    fixture.authority(),
                    state::SynchronizationAuthority::Published(candidate)
                );
            }
            "behind_reopens_then_publishes" => {
                assert_eq!(
                    push_intent_relation(&repository, candidate, Some(tip)).unwrap(),
                    PushIntentRelation::Behind
                );
                assert_eq!(
                    fixture.settle(&settlement(Some(tip))).unwrap(),
                    RemoteSafePointOutcome::Continue
                );
                let after = fixture.attempts();
                assert_eq!(after[0], rows[0]);
                assert_eq!(
                    after[1],
                    (
                        2,
                        candidate.to_string(),
                        Some(tip.to_string()),
                        "not_accepted".into(),
                        "open".into(),
                        None
                    )
                );
                // The endpoint advertised this target before; if it now
                // advertises nothing it was deleted, and is not recreated.
                assert!(fixture.prepare(candidate, None, false).is_err());
                assert_eq!(fixture.attempts(), after);
                assert_eq!(
                    fixture.prepare(candidate, Some(tip), false).unwrap(),
                    RemoteSafePointOutcome::Continue
                );
                fixture.point(RemoteOperationSafePoint::BeforePush);
                fixture
                    .advance(state::PublicationPhase::Returned, None)
                    .unwrap();
                fixture.point(RemoteOperationSafePoint::AfterPushReturn);
                fixture
                    .advance(state::PublicationPhase::Verified, Some(candidate))
                    .unwrap();
                assert_eq!(
                    fixture.authority(),
                    state::SynchronizationAuthority::Published(candidate)
                );
                assert_eq!(fixture.attempts()[0], rows[0]);
            }
            "verified_restart_before_classification" => {
                // A proven attempt whose endpoint later moved, rewound or
                // vanished is permanent Recovery; nothing is appended.
                let ahead = child_file(&repository, candidate, "remote-ahead.txt", b"ahead\n");
                let diverged = child_file(&repository, fixture.base, "remote-third.txt", b"x\n");
                for moved in [Some(tip), Some(ahead), Some(diverged), None] {
                    assert!(fixture.settle(&settlement(moved)).is_err(), "{moved:?}");
                    assert_eq!(fixture.attempts(), rows);
                }
                assert_eq!(
                    fixture.settle(&settlement(Some(candidate))).unwrap(),
                    RemoteSafePointOutcome::Continue
                );
                assert_eq!(fixture.attempts(), rows);
                // The reported kind does not depend on where the crash fell.
                assert_eq!(
                    fixture.authority(),
                    state::SynchronizationAuthority::AlreadyCurrent(candidate)
                );
            }
            "absent_after_recorded_advertisement" => {
                assert!(fixture.settle(&settlement(None)).is_err());
                assert_eq!(fixture.attempts(), rows);
                assert_eq!(fixture.intent(), stored);
            }
            _ => {
                // Nothing was ever advertised for this operation, so absence
                // is "not created yet": the same candidate may be retried.
                assert_eq!(
                    fixture.settle(&settlement(None)).unwrap(),
                    RemoteSafePointOutcome::Continue
                );
                let after = fixture.attempts();
                assert_eq!(after[0], rows[0]);
                assert_eq!(
                    after[1],
                    (
                        2,
                        candidate.to_string(),
                        None,
                        "not_accepted".into(),
                        "open".into(),
                        None
                    )
                );
                assert_eq!(
                    fixture.prepare(candidate, None, false).unwrap(),
                    RemoteSafePointOutcome::Continue
                );
            }
        }
        assert_eq!(fixture.legacy_push(), legacy, "{variant}");
        assert_eq!(
            local_binding_image(fixture.root.path()),
            before,
            "{variant}"
        );
        if fixture.intent() == PublicationIntent::Verified(candidate) {
            let authority = fixture.authority();
            fixture.point(RemoteOperationSafePoint::AfterPushVerification);
            fixture
                .service
                .classify_synchronization(fixture.root.path(), &fixture.owner, authority)
                .unwrap();
            let record = fixture.record();
            assert_eq!(record.authority, Some(authority), "{variant}");
            assert_eq!(record.sync_evidence.push_oid, Some(fixture.local));
        }
    }
}

/// E/F: every Push-direction relation of an old intent that is still HEAD.
/// Equal and strictly-behind evidence stay on the unchanged exact-retry route;
/// contained and displaced intents open an attempt; deletion after any
/// recorded advertisement, cancellation and stale owners write nothing.
#[test]
fn publication_settlement_table_preserves_the_old_push_intent() {
    for variant in [
        "equal",
        "behind",
        "ahead",
        "diverged",
        "deleted",
        "never_advertised",
        "cancelled",
        "stale_owner",
        "foreign_service",
        "verified_then_moved",
    ] {
        let mut fixture = PassFixture::new("local.txt", b"local\n");
        let repository = fixture.repository();
        let base = fixture.base;
        let local = fixture.local;
        fixture.fetch(base, 1);
        let mut evidence = state::SynchronizationEvidence {
            expected_oid: Some(local),
            local_oid: Some(local),
            tracking_oid: Some(base),
            primary_tracking_oid: Some(base),
            push_oid: Some(local),
            push_advertised_oid: (variant != "never_advertised").then_some(base),
        };
        fixture
            .service
            .checkpoint_synchronization(
                fixture.root.path(),
                &fixture.owner,
                state::SynchronizationCheckpoint::PushPrepared,
                &evidence,
            )
            .unwrap();
        if variant == "verified_then_moved" {
            evidence.push_advertised_oid = Some(local);
            fixture
                .service
                .checkpoint_synchronization(
                    fixture.root.path(),
                    &fixture.owner,
                    state::SynchronizationCheckpoint::PushVerified,
                    &evidence,
                )
                .unwrap();
        }
        let legacy = fixture.legacy_push();
        let before = local_binding_image(fixture.root.path());
        let ahead = child_file(&repository, local, "remote-ahead.txt", b"ahead\n");
        let diverged = child_file(&repository, base, "remote-other.txt", b"other\n");
        let advertised = match variant {
            "equal" => Some(local),
            "behind" | "cancelled" | "stale_owner" | "foreign_service" => Some(base),
            "ahead" => Some(ahead),
            "diverged" | "verified_then_moved" => Some(diverged),
            _ => None,
        };
        fixture.restart();
        fixture.fetch(base, 2);
        let relation = push_intent_relation(&repository, local, advertised).unwrap();
        assert_eq!(
            relation,
            match variant {
                "equal" => PushIntentRelation::Equal,
                "behind" | "cancelled" | "stale_owner" | "foreign_service" => {
                    PushIntentRelation::Behind
                }
                "ahead" => PushIntentRelation::Ahead,
                "diverged" | "verified_then_moved" => PushIntentRelation::Diverged,
                _ => PushIntentRelation::Absent,
            },
            "{variant}"
        );
        let mut settlement = PublicationSettlement {
            intent: PublicationIntent::Legacy(local),
            local_oid: local,
            continuation: false,
            advertised_oid: advertised,
            relation: Some(relation),
        };
        let stale = RepositoryService::open_at(fixture.data.path()).unwrap();
        let result = match variant {
            "cancelled" => {
                // Force the envelope route, then honor the durable request.
                settlement.advertised_oid = Some(diverged);
                settlement.relation = Some(PushIntentRelation::Diverged);
                fixture
                    .service
                    .cancel_remote_operation(fixture.root.path(), fixture.operation)
                    .unwrap();
                fixture.service.settle_synchronization_publication(
                    fixture.root.path(),
                    &fixture.owner,
                    &settlement,
                )
            }
            "stale_owner" => {
                settlement.advertised_oid = Some(diverged);
                settlement.relation = Some(PushIntentRelation::Diverged);
                // A later explicit restart fenced this token's owner epoch.
                // The new owner stands at the identical boundary, so only
                // the epoch distinguishes the refused call.
                let superseded = fixture.restart_superseding();
                fixture.fetch(base, 3);
                fixture.service.settle_synchronization_publication(
                    fixture.root.path(),
                    &superseded,
                    &settlement,
                )
            }
            "foreign_service" => {
                settlement.advertised_oid = Some(diverged);
                settlement.relation = Some(PushIntentRelation::Diverged);
                // Another service instance never inherits this owner's token.
                stale.settle_synchronization_publication(
                    fixture.root.path(),
                    &fixture.owner,
                    &settlement,
                )
            }
            _ => fixture.service.settle_synchronization_publication(
                fixture.root.path(),
                &fixture.owner,
                &settlement,
            ),
        };
        assert_eq!(fixture.legacy_push(), legacy, "{variant}");
        assert_eq!(
            local_binding_image(fixture.root.path()),
            before,
            "{variant}"
        );
        match variant {
            "ahead" | "diverged" => {
                assert_eq!(
                    result.unwrap(),
                    RemoteSafePointOutcome::Continue,
                    "{variant}"
                );
                assert_eq!(
                    fixture.attempts(),
                    vec![(
                        1,
                        local.to_string(),
                        advertised.map(|oid| oid.to_string()),
                        if variant == "ahead" {
                            "contained"
                        } else {
                            "displaced"
                        }
                        .into(),
                        "open".into(),
                        None
                    )],
                    "{variant}"
                );
                assert_eq!(
                    fixture
                        .service
                        .synchronization_publication_intent(fixture.root.path(), &fixture.owner)
                        .unwrap(),
                    PublicationIntent::Open
                );
                // A second settlement of the same intent cannot append again.
                assert!(
                    fixture
                        .service
                        .settle_synchronization_publication(
                            fixture.root.path(),
                            &fixture.owner,
                            &settlement
                        )
                        .is_err()
                );
                assert_eq!(fixture.attempts().len(), 1);
                // A later invocation resumes the open attempt offline; it
                // neither needs nor accepts another disposition.
                fixture.restart();
                fixture.fetch(base, 3);
                // The frozen legacy envelope is no longer reconciled by the
                // legacy transition, even from evidence it would accept.
                assert!(
                    fixture
                        .service
                        .reconcile_synchronization(
                            fixture.root.path(),
                            &fixture.owner,
                            local,
                            local,
                            Some(base),
                            true
                        )
                        .is_err(),
                    "{variant}"
                );
                assert_eq!(fixture.legacy_push(), legacy, "{variant}");
                assert!(
                    fixture
                        .service
                        .settle_synchronization_publication(
                            fixture.root.path(),
                            &fixture.owner,
                            &PublicationSettlement {
                                intent: PublicationIntent::Open,
                                ..settlement
                            }
                        )
                        .is_err()
                );
                assert_eq!(
                    fixture
                        .service
                        .settle_synchronization_publication(
                            fixture.root.path(),
                            &fixture.owner,
                            &PublicationSettlement {
                                intent: PublicationIntent::Open,
                                advertised_oid: None,
                                relation: None,
                                ..settlement
                            }
                        )
                        .unwrap(),
                    RemoteSafePointOutcome::Continue
                );
                assert_eq!(fixture.attempts().len(), 1);
                assert_eq!(fixture.legacy_push(), legacy, "{variant}");
            }
            "cancelled" => {
                assert_eq!(result.unwrap(), RemoteSafePointOutcome::Cancelled);
                assert!(fixture.attempts().is_empty());
            }
            _ => {
                // Equal and behind belong to the unchanged exact-retry route;
                // deletion, a moved verified intent and a stale owner are
                // typed Recovery with no appended evidence.
                assert!(result.is_err(), "{variant}");
                assert!(fixture.attempts().is_empty(), "{variant}");
            }
        }
        if variant == "stale_owner" {
            // The identical settlement under the current epoch is accepted.
            assert_eq!(
                fixture.settle(&settlement).unwrap(),
                RemoteSafePointOutcome::Continue
            );
            assert_eq!(fixture.attempts().len(), 1);
            assert_eq!(fixture.legacy_push(), legacy);
        }
        if matches!(variant, "equal" | "behind" | "never_advertised") {
            let actual = repository.head().unwrap().target().unwrap();
            assert_eq!(
                fixture
                    .service
                    .reconcile_synchronization(
                        fixture.root.path(),
                        &fixture.owner,
                        actual,
                        actual,
                        advertised,
                        variant == "behind"
                    )
                    .unwrap(),
                RemoteSafePointOutcome::Continue
            );
            assert_eq!(
                fixture.record().sync_checkpoint,
                Some(if variant == "equal" {
                    state::SynchronizationCheckpoint::PushVerified
                } else {
                    state::SynchronizationCheckpoint::PushPrepared
                })
            );
            assert!(fixture.attempts().is_empty());
        }
    }
}

#[test]
fn new_refs_append_window_immutable_evidence() {
    let (root, data, service) = fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    repository
        .remote("origin", "ssh://example.invalid/fixture.git")
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&service, root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), false)
    })
    .unwrap();
    let local = repository.head().unwrap().target().unwrap();
    let first = child(&repository, local, b"first pass\n");
    let second = child(&repository, first, b"second pass\n");
    let target = RemoteOperationTarget::for_primary_synchronization(&plan);
    let operation = OperationId::new();
    let RemoteReservationOutcome::Reserved(mut owner) = service
        .reserve_remote_operation(root.path(), operation, &target)
        .unwrap()
    else {
        panic!("reservation")
    };
    let mut evidence = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(local),
        ..Default::default()
    };
    service
        .checkpoint_synchronization(
            root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &evidence,
        )
        .unwrap();
    let mut original = None;
    for (number, incoming) in [(1, first), (2, second)] {
        let pending = if number == 2 {
            let RemoteReservationOutcome::Reserved(restarted) = service
                .restart_remote_synchronization(root.path(), operation, &target)
                .unwrap()
            else {
                panic!("new invocation")
            };
            owner = restarted;
            reconcile_pending_candidate(
                &service,
                root.path(),
                "main",
                &SynchronizationTarget::Primary,
                &owner,
                &mut evidence,
            )
            .unwrap()
        } else {
            None
        };
        repository
            .reference(
                plan.primary().tracking_ref(),
                incoming,
                true,
                "fixture observation",
            )
            .unwrap();
        service
            .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
            .unwrap();
        let observation = RemoteRefObservation::from_advertisement(
            &plan,
            "refs/heads/main",
            incoming,
            Some(incoming),
        )
        .unwrap();
        commit_observation_batch(
            &service,
            root.path(),
            &owner,
            &plan,
            &[observation],
            i64::from(number),
        )
        .unwrap();
        evidence.primary_tracking_oid = Some(incoming);
        evidence.tracking_oid = Some(incoming);
        if let Some(pending) = pending {
            finalize_reconciled_candidate(&service, root.path(), &owner, pending, &evidence)
                .unwrap();
        }
        let batch = state::with_transaction(&service, root.path(), |tx, id| {
            tx.query_row(
                "SELECT id FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1",
                [id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| state::recovery_required())
        })
        .unwrap();
        service
            .prepare_synchronization_window(
                root.path(),
                &owner,
                number,
                &state::IntegrationWindowIntent {
                    observation_batch_id: batch,
                    local_oid: repository.head().unwrap().target().unwrap(),
                    primary_oid: incoming,
                    context_oid: None,
                },
            )
            .unwrap();
        let configuration = service
            .observation_configuration(root.path(), &plan)
            .unwrap();
        let req = request(root.path());
        let merged = integrate_divergence(
            &service,
            DivergenceInputs {
                root: root.path(),
                primary_branch: "main",
                target: &SynchronizationTarget::Primary,
                request: &req,
                owner: &owner,
                plan: &plan,
                configuration: &configuration,
                selected: plan.primary(),
                primary_tracking: Some(incoming),
                selected_tracking: Some(incoming),
                context: None,
                primary: incoming,
            },
        )
        .unwrap();
        assert_eq!(merged, incoming);
        evidence.local_oid = Some(merged);
        service
            .checkpoint_synchronization_merge_applied(root.path(), &owner, &evidence)
            .unwrap();
        let step = state::with_transaction(&service, root.path(), |tx, id| {
            let record = state::read_operation(tx, id, operation)?.unwrap();
            assert!(state::integration_step(tx, record.id, 0)?.is_none());
            let old = state::integration_step_in_window(tx, record.id, 1, 0)?.unwrap();
            if let Some(original) = &original {
                assert_eq!(&old, original);
            }
            state::integration_step_in_window(tx, record.id, number, 0)
        })
        .unwrap()
        .unwrap();
        assert_eq!(step.phase, state::IntegrationStepPhase::Applied);
        if number == 1 {
            original = Some(step);
        }
    }
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_integration_steps", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        2
    );
    assert_eq!(repository.head().unwrap().target(), Some(second));
    assert!(repository.statuses(None).unwrap().is_empty());
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
    #[cfg(unix)]
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
    #[cfg(unix)]
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
    #[cfg(unix)]
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
    #[cfg(unix)]
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
    #[cfg(unix)]
    let races = ["staged", "untracked", "bytes", "symlink"];
    #[cfg(not(unix))]
    let races = ["staged", "untracked", "bytes", "directory"];
    for race in races {
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
            #[cfg(unix)]
            "symlink" => {
                fs::remove_file(root.path().join("docs/document.md")).unwrap();
                std::os::unix::fs::symlink("../fixture.txt", root.path().join("docs/document.md"))
                    .unwrap();
            }
            "directory" => {
                fs::remove_file(root.path().join("docs/document.md")).unwrap();
                fs::create_dir(root.path().join("docs/document.md")).unwrap();
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
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_lock_hook(root.path().to_owned(), move || {
        observed.store(true, Ordering::SeqCst);
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
    let outcome = service.resolve_synchronization(request).unwrap();
    assert!(
        fired.load(Ordering::SeqCst),
        "target-change lock hook fired"
    );
    assert!(matches!(
        outcome,
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

#[cfg(unix)]
#[test]
fn alias_registered_lock_hook_fires_at_canonical_dispatch_and_preserves_external_change() {
    let (root, _data, service, request) = protocol_resolution_fixture();
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("root");
    std::os::unix::fs::symlink(root.path(), &alias).unwrap();
    assert!(
        alias != root.path().canonicalize().unwrap(),
        "hook registration deliberately uses an alias"
    );
    let target = root.path().join("docs/document.md");
    let changed = b"external change at alias-registered lock boundary";
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_lock_hook(alias, move || {
        observed.store(true, Ordering::SeqCst);
        fs::write(target, changed).unwrap();
    });
    let outcome = service.resolve_synchronization(request).unwrap();
    assert!(
        fired.load(Ordering::SeqCst),
        "alias-registered lock callback fired"
    );
    assert!(matches!(
        outcome,
        ResolveSynchronizationOutcome::StaleObservation
    ));
    assert_eq!(
        fs::read(root.path().join("docs/document.md")).unwrap(),
        changed
    );
}

#[cfg(unix)]
#[test]
fn alias_registered_install_hook_panics_only_in_its_own_worktree() {
    let (root, _data, service, request) = protocol_resolution_fixture();
    let (other, _other_data, other_service, other_request) = protocol_resolution_fixture();
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("root");
    std::os::unix::fs::symlink(root.path(), &alias).unwrap();
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_install_hook(alias, move || {
        observed.store(true, Ordering::SeqCst);
        panic!("alias-registered installation fault");
    });
    assert!(matches!(
        other_service
            .resolve_synchronization(other_request)
            .unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    assert!(
        !fired.load(Ordering::SeqCst),
        "other worktree cannot consume callback"
    );
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        service.resolve_synchronization(request)
    }));
    assert!(
        fired.load(Ordering::SeqCst),
        "alias-registered install callback fired"
    );
    assert!(
        stopped.is_err(),
        "installation boundary propagated injected panic"
    );
    assert!(
        git2::Repository::open(root.path())
            .unwrap()
            .path()
            .join("index.lock")
            .exists()
    );
    assert!(
        !git2::Repository::open(other.path())
            .unwrap()
            .path()
            .join("index.lock")
            .exists()
    );
}

#[test]
fn represented_root_hooks_all_fire_at_real_resolution_boundaries() {
    type Register = fn(PathBuf, Box<dyn FnOnce() + Send>);
    let (root, _data, service, request) = protocol_resolution_fixture();
    let marker = root.path().join("hook-spelling");
    fs::create_dir(&marker).unwrap();
    let represented = marker.join("..");
    let registrations: [(&str, Register); 7] = [
        ("lock", set_resolution_index_lock_hook),
        ("scratch", set_resolution_index_scratch_hook),
        ("persist", set_resolution_index_persist_hook),
        ("install", set_resolution_index_install_hook),
        ("effect", set_resolution_index_effect_hook),
        ("refresh", set_resolution_ref_refresh_hook),
        ("retire", set_resolution_index_retire_hook),
    ];
    let fired = Arc::new(std::sync::Mutex::new(Vec::new()));
    for (stage, register) in registrations {
        let observed = fired.clone();
        register(
            represented.clone(),
            Box::new(move || {
                observed.lock().unwrap().push(stage);
            }),
        );
    }
    assert!(matches!(
        service.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    assert_eq!(
        *fired.lock().unwrap(),
        [
            "lock", "scratch", "persist", "install", "effect", "refresh", "retire"
        ],
        "every registered callback must fire at its real effect boundary"
    );
}

#[test]
fn absent_test_hook_root_keeps_its_exact_fallback_key() {
    let parent = fixture_tempdir();
    let absent = parent.path().join("absent");
    assert!(!absent.exists());
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_lock_hook(absent.clone(), move || {
        observed.store(true, Ordering::SeqCst);
    });
    run_resolution_index_lock_hook(parent.path());
    assert!(!fired.load(Ordering::SeqCst));
    run_resolution_index_lock_hook(&absent);
    assert!(fired.load(Ordering::SeqCst));
    assert!(
        !absent.exists(),
        "test-only lookup cannot create a missing root"
    );
}

fn registered_worktree_gitdir_text(worktree: &Path) -> String {
    let gitdir = worktree.join(".git");
    let text = gitdir.to_str().expect("fixture Git path must be UTF-8");
    // libgit2 reads this field verbatim, then dirname scans only '/'. Encode
    // Git metadata, not native path display; Unix backslashes are literal names.
    #[cfg(windows)]
    let text = {
        use std::path::{Component, Prefix};
        let text = match gitdir.components().next() {
            // Match Git's ordinary drive/UNC presentation, so '..' remains
            // meaningful without discarding the UNC server/share root.
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::VerbatimDisk(_) => text[4..].to_owned(),
                Prefix::VerbatimUNC(_, _) => format!(r"\\{}", &text[8..]),
                _ => text.to_owned(),
            },
            _ => text.to_owned(),
        };
        text.replace('\\', "/")
    };
    format!("{text}\n")
}

#[test]
fn own_authoring_registered_path_spelling_is_not_another_worktree() {
    let (root, _data, service) = fixture();
    let item_id: canonical::ItemId = "01BX5ZZKBKACTAV9WEVGEMMVRZ".parse().unwrap();
    let target = || AuthoringTarget {
        root: root.path().into(),
        kind: AuthoringKind::Document,
        item_id: item_id.clone(),
        intent: ContextIntent::Create,
        operation_id: OperationId::new(),
    };
    let context = match service.prepare_context(target()).unwrap() {
        ContextProvisionOutcome::Created(context) | ContextProvisionOutcome::Reused(context) => {
            context
        }
        ContextProvisionOutcome::IndexPending { context } => context,
    };
    let repository = git2::Repository::open(root.path()).unwrap();
    // Exercise literal Unix backslashes through the real metadata parser too.
    #[cfg(unix)]
    let marker = root.path().join(r"registered\spelling");
    #[cfg(not(unix))]
    let marker = root.path().join("registered-spelling");
    fs::create_dir(&marker).unwrap();
    // Existing, real directory components give libgit2 a noncanonical spelling
    // of the very same deterministic worktree (no symlink authorization).
    let registered = marker
        .join("..")
        .join(".manyhands/worktrees")
        .join(item_id.to_string());
    fs::write(
        repository
            .path()
            .join("worktrees")
            .join(item_id.to_string())
            .join("gitdir"),
        registered_worktree_gitdir_text(&registered),
    )
    .unwrap();
    let metadata = repository.find_worktree(&item_id.to_string()).unwrap();
    assert!(
        metadata.path() != context.worktree.canonicalize().unwrap(),
        "registered spelling is noncanonical"
    );
    assert!(
        metadata.path().canonicalize().unwrap() == context.worktree.canonicalize().unwrap(),
        "registered spelling names the same physical worktree"
    );
    let reused = service.prepare_context(target());
    assert!(
        reused.is_ok(),
        "same physical authoring worktree must be reusable; category={:?}",
        reused.as_ref().err().map(|error| error.kind)
    );
}

#[cfg(unix)]
#[test]
fn own_authoring_registered_symlink_alias_does_not_authorize_reuse() {
    let (root, _data, service) = fixture();
    let item_id: canonical::ItemId = "01BX5ZZKBKACTAV9WEVGEMMVRZ".parse().unwrap();
    let target = || AuthoringTarget {
        root: root.path().into(),
        kind: AuthoringKind::Document,
        item_id: item_id.clone(),
        intent: ContextIntent::Create,
        operation_id: OperationId::new(),
    };
    let context = match service.prepare_context(target()).unwrap() {
        ContextProvisionOutcome::Created(context) | ContextProvisionOutcome::Reused(context) => {
            context
        }
        ContextProvisionOutcome::IndexPending { context } => context,
    };
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("untrusted-root");
    std::os::unix::fs::symlink(&context.root, &alias).unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let registered = alias.join(".manyhands/worktrees").join(item_id.to_string());
    fs::write(
        repository
            .path()
            .join("worktrees")
            .join(item_id.to_string())
            .join("gitdir"),
        registered_worktree_gitdir_text(&registered),
    )
    .unwrap();
    let linked = git2::Repository::open(&context.worktree).unwrap();
    let before = (
        linked.head().unwrap().target(),
        fs::read(linked.path().join("index")).unwrap(),
    );
    assert!(
        matches!(service.prepare_context(target()),
        Err(error) if error.kind == RepositoryErrorKind::MismatchedAuthoringContext),
        "an untrusted registered root alias must remain refused"
    );
    assert_eq!(
        (
            linked.head().unwrap().target(),
            fs::read(linked.path().join("index")).unwrap()
        ),
        before
    );
}

#[test]
fn own_authoring_registered_path_guard_refuses_different_and_unavailable_locations() {
    for case in [
        "different_root",
        "missing_registered",
        "missing_intended",
        "both_missing",
    ] {
        let (root, _data, service) = fixture();
        let (other_root, _other_data, _other_service) = fixture();
        let item_id: canonical::ItemId = "01BX5ZZKBKACTAV9WEVGEMMVRZ".parse().unwrap();
        let target = || AuthoringTarget {
            root: root.path().into(),
            kind: AuthoringKind::Document,
            item_id: item_id.clone(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        };
        let context = match service.prepare_context(target()).unwrap() {
            ContextProvisionOutcome::Created(context)
            | ContextProvisionOutcome::Reused(context) => context,
            ContextProvisionOutcome::IndexPending { context } => context,
        };
        let repository = git2::Repository::open(root.path()).unwrap();
        let other = git2::Repository::open(other_root.path()).unwrap();
        let head = other.head().unwrap().peel_to_commit().unwrap();
        other.branch(&context.branch, &head, false).unwrap();
        other
            .set_head(&format!("refs/heads/{}", context.branch))
            .unwrap();
        let registered = match case {
            "different_root" => other_root.path().to_owned(),
            "missing_registered" => other_root.path().join("absent"),
            "both_missing" => {
                fs::rename(&context.worktree, root.path().join("displaced")).unwrap();
                assert!(!context.worktree.exists());
                other_root.path().join("absent")
            }
            "missing_intended" => {
                let moved = root.path().join("displaced");
                fs::rename(&context.worktree, &moved).unwrap();
                assert!(!context.worktree.exists());
                moved
            }
            _ => unreachable!(),
        };
        fs::write(
            repository
                .path()
                .join("worktrees")
                .join(item_id.to_string())
                .join("gitdir"),
            registered_worktree_gitdir_text(&registered),
        )
        .unwrap();
        // Attest the fixture before invoking the guard: it must describe the
        // foreign/missing location, not a directory misparsed from native text.
        let metadata = repository.find_worktree(&item_id.to_string()).unwrap();
        if registered.exists() {
            assert!(
                metadata.path().canonicalize().unwrap() == registered.canonicalize().unwrap(),
                "registered metadata names the intended physical fixture; case={case}"
            );
        } else {
            assert!(
                !metadata.path().exists(),
                "registered metadata names a missing fixture; case={case}"
            );
        }
        let before = (
            repository.head().unwrap().target(),
            fs::read(repository.path().join("index")).unwrap(),
            other.head().unwrap().target(),
            fs::read(other.path().join("index")).unwrap(),
        );
        let refused = service.prepare_context(target());
        assert!(
            matches!(&refused,
            Err(error) if error.kind == RepositoryErrorKind::MismatchedAuthoringContext),
            "different or unavailable registered/intended locations must refuse; case={case}; category={:?}",
            refused.as_ref().err().map(|error| error.kind)
        );
        assert_eq!(
            (
                repository.head().unwrap().target(),
                fs::read(repository.path().join("index")).unwrap(),
                other.head().unwrap().target(),
                fs::read(other.path().join("index")).unwrap()
            ),
            before
        );
    }
}

#[cfg(not(any(unix, windows)))]
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

#[cfg(windows)]
#[test]
fn windows_owned_resolution_helpers_preserve_exact_bytes_and_prewrite_guard() {
    let (root, _data, _service) = fixture();
    let relative = Path::new("docs/windows-image.md");
    let bytes = b"caller\r\nbytes\0\xff";
    let operation = RepositoryOperation::RepositorySnapshot;
    let missing = crate::repository::owned_prewrite_digest(None);
    crate::repository::write_owned_document_if_prewrite_digest(
        root.path(),
        relative,
        bytes,
        missing,
        operation,
        root.path(),
    )
    .unwrap();
    assert_eq!(
        owned_resolution_file_bytes(root.path(), relative, operation, root.path()).unwrap(),
        Some(bytes.to_vec())
    );
    assert!(
        crate::repository::write_owned_document_if_prewrite_digest(
            root.path(),
            relative,
            b"must not replace",
            missing,
            operation,
            root.path(),
        )
        .is_err()
    );
    assert_eq!(std::fs::read(root.path().join(relative)).unwrap(), bytes);
}

#[cfg(windows)]
#[test]
fn windows_public_resolution_preserves_foreign_index_lock() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let (root, _data, service, operation, _, _) = resolution_fixture(&[(
        "docs/document.md",
        base,
        &base.replace("base", "local"),
        &base.replace("base", "remote"),
    )]);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let repository = git2::Repository::open(root.path()).unwrap();
    let index = std::fs::read(repository.path().join("index")).unwrap();
    let foreign = b"unowned index lock";
    std::fs::write(repository.path().join("index.lock"), foreign).unwrap();
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
    assert_eq!(
        std::fs::read(repository.path().join("index.lock")).unwrap(),
        foreign
    );
    assert_eq!(
        std::fs::read(repository.path().join("index")).unwrap(),
        index
    );
}

#[cfg(windows)]
fn windows_image_identity(path: &Path) -> [u64; 2] {
    native_resolution::Directory::open(path.parent().unwrap())
        .unwrap()
        .image(path.file_name().unwrap().to_str().unwrap())
        .unwrap()
        .unwrap()
        .stamp
        .identity
}

#[cfg(windows)]
#[test]
fn windows_public_resolution_path_observation_fault_reuses_exact_result_inode() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let database = data.path().join(REGISTRY_FILE);
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_path_observation BEFORE UPDATE OF applied ON remote_resolution_paths WHEN NEW.applied=1 BEGIN SELECT RAISE(ABORT,'test path observation fault'); END;").unwrap();
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let path = root.path().join("docs/document.md");
    assert!(fs::read(&path).unwrap() == request.resolutions[0].1.bytes());
    let identity = windows_image_identity(&path);
    connection
        .execute_batch("DROP TRIGGER fail_path_observation;")
        .unwrap();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request.clone()).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    assert_eq!(windows_image_identity(&path), identity);
    assert!(fs::read(path).unwrap() == request.resolutions[0].1.bytes());
}

#[cfg(windows)]
#[test]
fn windows_public_resolution_checkpoint_observation_fault_never_reappends_logs() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_checkpoint_observation BEFORE UPDATE OF phase ON remote_resolution_attempts WHEN NEW.phase='applied' BEGIN SELECT RAISE(ABORT,'test checkpoint observation fault'); END;").unwrap();
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let repository = git2::Repository::open(root.path()).unwrap();
    let candidate = repository.refname_to_id("HEAD").unwrap();
    assert_ne!(candidate, request.observation.head);
    let commit = repository.find_commit(candidate).unwrap();
    assert_eq!(commit.parent_count(), 2);
    assert_eq!(commit.parent_id(0).unwrap(), request.observation.head);
    let logs = [
        repository.commondir().join("logs/refs/heads/main"),
        repository.path().join("logs/HEAD"),
    ];
    let images = logs.each_ref().map(|path| fs::read(path).unwrap());
    connection
        .execute_batch("DROP TRIGGER fail_checkpoint_observation;")
        .unwrap();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    for _ in 0..2 {
        assert!(
            matches!(restarted.resolve_synchronization(request.clone()).unwrap(), ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } if commit_oid == candidate)
        );
        assert!(logs.each_ref().map(|path| fs::read(path).unwrap()) == images);
        assert!(!repository.path().join("index.lock").exists());
    }
}

#[cfg(windows)]
#[test]
fn windows_public_resolution_preserves_same_bytes_foreign_sentinel_anchor() {
    let (root, data, service, request) = protocol_resolution_fixture();
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterCandidatePrepared);
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let repository = git2::Repository::open(root.path()).unwrap();
    let sentinel = repository.path().join(format!(
        ".manyhands-resolution-{}/sentinel",
        request.attempt_id
    ));
    let bytes = fs::read(&sentinel).unwrap();
    fs::rename(&sentinel, sentinel.with_file_name("saved-sentinel")).unwrap();
    fs::write(&sentinel, &bytes).unwrap();
    let foreign = windows_image_identity(&sentinel);
    let index = fs::read(repository.path().join("index")).unwrap();
    let old = repository.refname_to_id("HEAD").unwrap();
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(restarted.resolve_synchronization(request).is_err());
    assert_eq!(windows_image_identity(&sentinel), foreign);
    assert!(fs::read(sentinel).unwrap() == bytes);
    assert!(fs::read(repository.path().join("index")).unwrap() == index);
    assert_eq!(repository.refname_to_id("HEAD").unwrap(), old);
    assert!(repository.path().join("index.lock").exists());
}

#[cfg(windows)]
#[test]
fn windows_public_resolution_rejects_unowned_or_missing_immutable_ref_proof() {
    for adverse in ["missing", "same-bytes-foreign", "tampered"] {
        let (root, data, service, request) = protocol_resolution_fixture();
        *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterRefTransition);
        assert!(service.resolve_synchronization(request.clone()).is_err());
        let repository = git2::Repository::open(root.path()).unwrap();
        let candidate = repository.refname_to_id("HEAD").unwrap();
        let logs = [
            repository.commondir().join("logs/refs/heads/main"),
            repository.path().join("logs/HEAD"),
        ];
        let log_images = logs.each_ref().map(|path| fs::read(path).unwrap());
        let staging = repository
            .path()
            .join(format!(".manyhands-resolution-{}", request.attempt_id));
        let manifest = staging.join("ref-log-transition");
        let bytes = fs::read(&manifest).unwrap();
        match adverse {
            "missing" => fs::remove_file(&manifest).unwrap(),
            "same-bytes-foreign" => {
                fs::rename(&manifest, staging.join("saved-transition")).unwrap();
                fs::write(&manifest, &bytes).unwrap();
            }
            "tampered" => fs::write(&manifest, b"unproved manifest").unwrap(),
            _ => unreachable!(),
        }
        let preserved = fs::read(&manifest).ok();
        let restarted = RepositoryService::open_at(data.path()).unwrap();
        assert!(matches!(
            restarted.resolve_synchronization(request),
            Err(SynchronizationError::RecoveryRequired)
        ));
        assert!(fs::read(manifest).ok() == preserved);
        assert!(logs.each_ref().map(|path| fs::read(path).unwrap()) == log_images);
        assert_eq!(repository.refname_to_id("HEAD").unwrap(), candidate);
        assert!(repository.path().join("index.lock").exists());
    }
}

#[cfg(windows)]
#[test]
fn windows_public_resolution_final_proof_rejects_changed_branch_and_exact_oid() {
    for reconcile in [false, true] {
        for third_commit in [false, true] {
            let (root, data, service, request) = protocol_resolution_fixture();
            let repository = git2::Repository::open(root.path()).unwrap();
            repository
                .config()
                .unwrap()
                .set_bool("core.logallrefupdates", false)
                .unwrap();
            for log in ["logs/refs/heads/main", "logs/HEAD"] {
                fs::remove_file(repository.path().join(log)).unwrap();
            }
            if reconcile {
                *service.failure_point.lock().unwrap() =
                    Some(FailurePoint::ResolutionAfterCandidatePrepared);
                assert!(service.resolve_synchronization(request.clone()).is_err());
            }
            let hook_root = root.path().to_owned();
            let fired = Arc::new(AtomicBool::new(false));
            let observed = fired.clone();
            set_resolution_ref_refresh_hook(root.path().to_owned(), move || {
                observed.store(true, Ordering::SeqCst);
                let repository = git2::Repository::open(hook_root).unwrap();
                let candidate = repository.refname_to_id("HEAD").unwrap();
                if third_commit {
                    let commit = repository.find_commit(candidate).unwrap();
                    let signature =
                        git2::Signature::now("External", "external@example.invalid").unwrap();
                    let foreign = repository
                        .commit(
                            None,
                            &signature,
                            &signature,
                            "same tree",
                            &commit.tree().unwrap(),
                            &[&commit],
                        )
                        .unwrap();
                    fs::write(
                        repository.path().join("refs/heads/main"),
                        format!("{foreign}\n"),
                    )
                    .unwrap();
                } else {
                    fs::write(
                        repository.path().join("refs/heads/substitute"),
                        format!("{candidate}\n"),
                    )
                    .unwrap();
                    fs::write(
                        repository.path().join("HEAD"),
                        b"ref: refs/heads/substitute\n",
                    )
                    .unwrap();
                }
            });
            let restarted = RepositoryService::open_at(data.path()).unwrap();
            let result = if reconcile {
                restarted.resolve_synchronization(request.clone())
            } else {
                service.resolve_synchronization(request.clone())
            };
            assert!(
                fired.load(Ordering::SeqCst),
                "native final-proof ref-refresh fault fired"
            );
            assert!(matches!(
                result,
                Err(SynchronizationError::RecoveryRequired)
            ));
            let live = repository.refname_to_id("HEAD").unwrap();
            let metadata =
                RESOLUTION_MERGE_MEMBERS.map(|role| fs::read(repository.path().join(role)).ok());
            assert!(restarted.resolve_synchronization(request).is_err());
            assert_eq!(repository.refname_to_id("HEAD").unwrap(), live);
            assert!(
                RESOLUTION_MERGE_MEMBERS.map(|role| fs::read(repository.path().join(role)).ok())
                    == metadata
            );
            assert!(repository.path().join("index.lock").exists());
            let checkpoints: i64 = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap().query_row("SELECT count(*) FROM remote_resolution_attempts WHERE checkpoint_oid IS NOT NULL", [], |row| row.get(0)).unwrap();
            assert_eq!(checkpoints, 0);
        }
    }
}

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn native_fixture_symlink_parent_runs_workdir_hook_and_reads_owned_file() {
    use std::os::unix::fs::symlink;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    let temporary = tempfile::tempdir().unwrap();
    let parent = temporary.path().canonicalize().unwrap();
    let real = parent.join("real");
    let alias = parent.join("alias");
    fs::create_dir(&real).unwrap();
    symlink(&real, &alias).unwrap();
    let (root, _data, _service) = fixture_in(&alias);
    let repository = git2::Repository::open(root.path()).unwrap();
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_lock_hook(root.path().to_owned(), move || {
        observed.store(true, Ordering::SeqCst);
    });
    // This is the exact workdir-keyed dispatch used by native lock acquisition.
    run_resolution_index_lock_hook(repository.workdir().unwrap());
    let fired_at_workdir = fired.load(Ordering::SeqCst);
    // Also clear an unmatched registration on failure so it cannot leak.
    run_resolution_index_lock_hook(root.path());
    assert!(
        fired_at_workdir,
        "fixture root must match canonical workdir hook lookup"
    );
    assert_eq!(root.path(), repository.workdir().unwrap());
    assert_eq!(
        owned_resolution_file_bytes(
            root.path(),
            Path::new("fixture.txt"),
            RepositoryOperation::RepositorySnapshot,
            root.path(),
        )
        .unwrap(),
        Some(b"original\n".to_vec())
    );

    // Reintroducing the deliberate ancestor alias must still be refused; the
    // fixture fix must not weaken the production pinned no-follow helper.
    let aliased_root = alias.join(root.path().file_name().unwrap());
    assert!(
        owned_resolution_file_bytes(
            &aliased_root,
            Path::new("fixture.txt"),
            RepositoryOperation::RepositorySnapshot,
            &aliased_root,
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn native_serialization_uses_the_validated_private_namespace() {
    let (root, _data, service, request) = protocol_resolution_fixture();
    *service.failure_point.lock().unwrap() = Some(FailurePoint::ResolutionAfterCandidatePrepared);
    assert!(service.resolve_synchronization(request.clone()).is_err());
    let record = state::with_transaction(&service, root.path(), |tx, id| {
        state::read_operation(tx, id, request.synchronization_id)
    })
    .unwrap()
    .unwrap();
    let RemoteReservationOutcome::Reserved(owner) = service
        .reacquire_synchronization_conflict(
            root.path(),
            request.synchronization_id,
            &record.target,
            request.observation.ordinal,
            request.observation.fingerprint,
        )
        .unwrap()
    else {
        panic!("reservation must resume")
    };
    let repository = git2::Repository::open(root.path()).unwrap();
    let held = ResolutionIndexLock::acquire(
        &repository,
        &service,
        root.path(),
        &owner,
        request.attempt_id,
    )
    .unwrap();
    let index = held.authoritative_index().unwrap();
    assert_eq!(
        index.path(),
        Some(
            repository
                .path()
                .join(format!(
                    ".manyhands-resolution-{}/index",
                    request.attempt_id
                ))
                .as_path()
        )
    );
}

#[cfg(unix)]
#[test]
fn native_candidate_storage_refuses_redirected_objects_before_journaling() {
    use std::os::unix::fs::symlink;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let (root, data, service, request) = protocol_resolution_fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let old = repository.head().unwrap().target();
    let index = fs::read(repository.path().join("index")).unwrap();
    let gitdir = repository.path().to_owned();
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_lock_hook(root.path().to_owned(), move || {
        observed.store(true, Ordering::SeqCst);
        fs::rename(gitdir.join("objects"), gitdir.join("retained-objects")).unwrap();
        symlink(gitdir.join("retained-objects"), gitdir.join("objects")).unwrap();
    });
    let result = service.resolve_synchronization(request);
    assert!(
        fired.load(Ordering::SeqCst),
        "object redirection hook must run"
    );
    assert!(result.is_err());
    assert_eq!(repository.head().unwrap().target(), old);
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert!(
        connection
            .query_row(
                "SELECT candidate_oid IS NULL FROM remote_resolution_attempts",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap()
    );
}

#[cfg(unix)]
#[test]
fn native_index_image_refuses_executable_files() {
    use std::os::unix::fs::PermissionsExt;
    let directory = fixture_tempdir();
    let path = directory.path().join("index");
    fs::write(&path, b"not an index").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let parent = fs::File::open(directory.path()).unwrap();
    assert!(matches!(
        index_file_image_at(&parent, &fixed_index_name("index")),
        Err(SynchronizationError::ExternalChange)
    ));
}

#[cfg(unix)]
#[test]
fn native_owned_resolution_read_refuses_symlinked_root_ancestor() {
    use std::os::unix::fs::symlink;
    let directory = fixture_tempdir();
    fs::create_dir(directory.path().join("real")).unwrap();
    fs::create_dir(directory.path().join("real/root")).unwrap();
    fs::write(directory.path().join("real/root/document"), b"body").unwrap();
    let real_root = directory.path().join("real/root");
    assert_eq!(
        owned_resolution_file_bytes(
            &real_root,
            Path::new("document"),
            RepositoryOperation::RepositorySnapshot,
            &real_root,
        )
        .unwrap(),
        Some(b"body".to_vec())
    );
    symlink(
        directory.path().join("real"),
        directory.path().join("alias"),
    )
    .unwrap();
    let root = directory.path().join("alias/root");
    assert!(
        owned_resolution_file_bytes(
            &root,
            Path::new("document"),
            RepositoryOperation::RepositorySnapshot,
            &root,
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn native_backend_lock_scan_refuses_symlinked_gitdir_ancestor() {
    use std::os::unix::fs::symlink;
    let directory = fixture_tempdir();
    let real = directory.path().join("real");
    fs::create_dir(&real).unwrap();
    let repository = git2::Repository::init(real.join("root")).unwrap();
    let signature = git2::Signature::now("Native", "native@example.com").unwrap();
    let tree = repository.index().unwrap().write_tree().unwrap();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "initial",
            &repository.find_tree(tree).unwrap(),
            &[],
        )
        .unwrap();
    // Opening an alias first lets libgit2 canonicalize it. Replace an ancestor
    // only after opening the repository so the helper sees the stale namespace.
    refuse_ambiguous_resolution_backend_locks(&repository).unwrap();
    let moved = directory.path().join("moved");
    fs::rename(&real, &moved).unwrap();
    symlink(&moved, &real).unwrap();
    assert!(refuse_ambiguous_resolution_backend_locks(&repository).is_err());
}

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn advisory_index_extensions_are_explicitly_rebuildable() {
    let (root, _data, _service) = fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let index = index_with_extension(fs::read(repository.path().join("index")).unwrap(), b"TREE");
    assert!(approved_index_extensions(&index).is_ok());
}

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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
    let first_fired = Arc::new(AtomicBool::new(false));
    let second_fired = Arc::new(AtomicBool::new(false));
    let first_observed = first_fired.clone();
    let second_observed = second_fired.clone();
    set_resolution_index_lock_hook(hook_root.path().to_owned(), move || {
        first_observed.store(true, Ordering::SeqCst);
        fs::write(hook_target, first_hook_external).unwrap()
    });
    set_resolution_index_lock_hook(other_root.path().to_owned(), move || {
        second_observed.store(true, Ordering::SeqCst);
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
    assert!(
        second_fired.load(Ordering::SeqCst),
        "second worktree lock hook fired"
    );
    assert!(
        !first_fired.load(Ordering::SeqCst),
        "first worktree hook remains independent"
    );
    assert!(matches!(
        hook_service
            .resolve_synchronization(request(hook_root.path(), hook_operation, &hook_inspection))
            .unwrap(),
        ResolveSynchronizationOutcome::StaleObservation
    ));
    assert!(
        first_fired.load(Ordering::SeqCst),
        "first worktree lock hook fired"
    );
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(any(unix, windows))]
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

#[cfg(unix)]
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

#[cfg(any(unix, windows))]
#[test]
fn release_observation_transaction_failure_recovers_absence_and_refuses_foreign_replacement() {
    for foreign in [false, true] {
        let (root, data, service, request) = protocol_resolution_fixture();
        let database = data.path().join(REGISTRY_FILE);
        let hook_database = database.clone();
        let fired = Arc::new(AtomicBool::new(false));
        let observed = fired.clone();
        set_resolution_index_retire_hook(root.path().to_owned(), move || {
            observed.store(true, Ordering::SeqCst);
            rusqlite::Connection::open(hook_database).unwrap().execute_batch("CREATE TRIGGER fail_release_observation BEFORE UPDATE OF phase ON remote_resolution_index_artifacts WHEN NEW.phase='released' BEGIN SELECT RAISE(ABORT,'test observation fault'); END;").unwrap();
        });
        let outcome = service.resolve_synchronization(request.clone());
        assert!(
            fired.load(Ordering::SeqCst),
            "release-observation retire hook fired"
        );
        assert!(outcome.is_err());
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

#[cfg(any(unix, windows))]
#[test]
fn pre_ref_installed_index_old_head_reuses_candidate_and_preserves_foreign_backend_lock() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    let old_head = repository.head().unwrap().target().unwrap();
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        observed.store(true, Ordering::SeqCst);
        panic!("test termination after installation")
    });
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        service.resolve_synchronization(request.clone())
    }));
    assert!(
        fired.load(Ordering::SeqCst),
        "pre-ref installation hook fired"
    );
    assert!(stopped.is_err());
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

#[cfg(any(unix, windows))]
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

#[cfg(any(unix, windows))]
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
    let private = fixture_tempdir();
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

#[cfg(unix)]
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

#[cfg(unix)]
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
fn comment_resolution_created_by_changes_fail_validation_before_effects() {
    let alice = Some("Alice <alice@example.invalid>");
    let mallory = Some("Mallory <mallory@example.invalid>");
    for (case, base_creator, local_creator, incoming_creator, result_creator) in [
        ("replacement", alice, alice, alice, mallory),
        ("removal", alice, alice, alice, None),
        ("invented legacy creator", None, None, None, alice),
        ("base disagreement", mallory, alice, alice, alice),
        ("local disagreement", alice, mallory, alice, alice),
        ("incoming disagreement", alice, alice, mallory, alice),
        ("base cannot be outvoted", alice, mallory, mallory, mallory),
        ("local absence", alice, None, alice, alice),
        ("incoming absence", alice, alice, None, alice),
    ] {
        let base = resolution_creator_comment(base_creator, false, "base");
        let local = resolution_creator_comment(local_creator, false, "local");
        let incoming = resolution_creator_comment(incoming_creator, false, "incoming");
        let result = resolution_creator_comment(result_creator, false, "resolved");
        let path = ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01CRZ3NDEKTSV4RRFFQ69G5FAV.md";
        let (root, data, service, operation, head, _) = resolution_fixture(&[
            ("docs/item.md", M2_DOCUMENT, M2_DOCUMENT, M2_DOCUMENT),
            (path, &base, &local, &incoming),
        ]);
        let inspection = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        assert_eq!(inspection.paths.len(), 1);
        assert_eq!(
            inspection.paths[0].eligibility,
            merge::ConflictEligibility::EligibleCanonical
        );
        assert!(inspection.paths[0].token.path == path.as_bytes());
        let attempt = OperationId::new();
        let request =
            resolution_request_for_bodies(root.path(), operation, attempt, &inspection, &[&result]);
        let repository = git2::Repository::open(root.path()).unwrap();
        let (token, bytes) = &request.resolutions[0];
        let replacements = std::collections::BTreeMap::from([(token.ordinal, (token, bytes))]);
        assert!(
            RepositoryService::validates_prospective_context(&repository, &replacements).unwrap(),
            "{case}: creator evidence must be the only invalid invariant"
        );
        // Compare digests, so a failed assertion never dumps canonical bodies
        // or raw ref/log images into fixture output.
        let images = || {
            let mut paths = vec![root.path().join(path), root.path().join("docs/item.md")];
            paths.extend(
                [
                    "HEAD",
                    "refs/heads/main",
                    "logs/HEAD",
                    "logs/refs/heads/main",
                    "index",
                    "ORIG_HEAD",
                    "MERGE_HEAD",
                    "MERGE_MSG",
                    "MERGE_MODE",
                ]
                .map(|name| repository.path().join(name)),
            );
            paths
                .iter()
                .map(|path| match fs::read(path) {
                    Ok(bytes) => Some(*blake3::hash(&bytes).as_bytes()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(_) => panic!("resolution image unavailable"),
                })
                .collect::<Vec<_>>()
        };
        let before = images();
        let staging = repository
            .path()
            .join(format!(".manyhands-resolution-{attempt}"));
        assert!(!staging.exists());
        assert!(!repository.path().join("index.lock").exists());
        assert_eq!(
            service.resolve_synchronization(request).unwrap(),
            ResolveSynchronizationOutcome::ValidationFailed,
            "{case}"
        );
        assert_eq!(repository.head().unwrap().target(), Some(head), "{case}");
        assert_eq!(images(), before, "{case}");
        assert!(repository.index().unwrap().has_conflicts());
        assert!(!staging.exists());
        assert!(!repository.path().join("index.lock").exists());
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        for table in [
            "remote_resolution_attempts",
            "remote_resolution_paths",
            "remote_resolution_index_artifacts",
            "remote_resolution_ref_log_artifacts",
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
                0,
                "{case}/{table}"
            );
        }
    }
}

fn resolution_creator_comment(creator: Option<&str>, reply: bool, body: &str) -> String {
    let parent = if reply {
        "parent_id: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\n"
    } else {
        ""
    };
    let creator = creator
        .map(|creator| format!("created_by: \"{creator}\"\n"))
        .unwrap_or_default();
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"01CRZ3NDEKTSV4RRFFQ69G5FAV\"\nitem_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\n{parent}created_at: \"2026-01-01T00:00:00Z\"\nauthor: \"legacy author\"\n{creator}---\n\n{body}\n"
    )
}

#[test]
fn comment_resolution_preserves_created_by_and_exact_bytes_for_roots_and_replies() {
    for creator in [Some("Alice <alice@example.invalid>"), None] {
        for reply in [false, true] {
            let base = resolution_creator_comment(creator, reply, "base");
            let local = resolution_creator_comment(creator, reply, "local");
            let incoming = resolution_creator_comment(creator, reply, "incoming");
            let result = resolution_creator_comment(creator, reply, "resolved\n\n  exact spacing")
                .replace("---\n\n", "custom: [keep, unknown]\n---\n\n");
            let path =
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01CRZ3NDEKTSV4RRFFQ69G5FAV.md";
            let parent_path =
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01BX5ZZKBKACTAV9WEVGEMMVRZ.md";
            let parent = resolution_creator_comment(None, false, "parent")
                .replace("01CRZ3NDEKTSV4RRFFQ69G5FAV", "01BX5ZZKBKACTAV9WEVGEMMVRZ");
            let (root, _data, service, operation, local_oid, incoming_oid) = resolution_fixture(&[
                ("docs/item.md", M2_DOCUMENT, M2_DOCUMENT, M2_DOCUMENT),
                (parent_path, &parent, &parent, &parent),
                (path, &base, &local, &incoming),
            ]);
            let inspection = service
                .inspect_synchronization_recovery(root.path(), operation)
                .unwrap();
            assert_eq!(inspection.paths.len(), 1);
            assert_eq!(
                inspection.paths[0].eligibility,
                merge::ConflictEligibility::EligibleCanonical
            );
            let request = resolution_request_for_bodies(
                root.path(),
                operation,
                OperationId::new(),
                &inspection,
                &[&result],
            );
            let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
                service.resolve_synchronization(request).unwrap()
            else {
                panic!("unchanged creator must permit exact caller resolution");
            };
            let repository = git2::Repository::open(root.path()).unwrap();
            let commit = repository.find_commit(commit_oid).unwrap();
            assert_eq!(commit.parent_count(), 2);
            assert_eq!(
                [commit.parent_id(0).unwrap(), commit.parent_id(1).unwrap()],
                [local_oid, incoming_oid]
            );
            assert_eq!(repository.head().unwrap().target(), Some(commit_oid));
            assert!(!repository.index().unwrap().has_conflicts());
            assert!(fs::read(root.path().join(path)).unwrap() == result.as_bytes());
            assert!(fs::read(root.path().join(parent_path)).unwrap() == parent.as_bytes());
            let blob = repository
                .find_blob(
                    commit
                        .tree()
                        .unwrap()
                        .get_path(Path::new(path))
                        .unwrap()
                        .id(),
                )
                .unwrap();
            assert!(blob.content() == result.as_bytes());
            let canonical::CanonicalItem::Comment(comment) =
                canonical::parse_item(Path::new(path), &result).unwrap()
            else {
                panic!("expected comment");
            };
            let expected_creator = creator.map(|value| serde_yaml::Value::String(value.to_owned()));
            assert!(comment.unknown.get("created_by") == expected_creator.as_ref());
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
#[cfg(unix)]
fn ref_effect_api_fixture() -> (tempfile::TempDir, git2::Repository, git2::Oid, git2::Oid) {
    let root = fixture_tempdir();
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

#[cfg(unix)]
fn ref_effect_entry(old: git2::Oid, new: git2::Oid) -> Vec<u8> {
    format!("{old} {new} Frozen Identity <frozen@example.invalid> 5678 +0000\towned effect\n")
        .into_bytes()
}

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(any(unix, windows))]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn ref_log_final_proof_initial_rejects_same_candidate_branch_substitution() {
    assert_ref_log_final_proof_rejects_stale_target(false, false);
}

#[cfg(unix)]
#[test]
fn ref_log_final_proof_reconcile_rejects_same_candidate_branch_substitution() {
    assert_ref_log_final_proof_rejects_stale_target(true, false);
}

#[cfg(unix)]
#[test]
fn ref_log_final_proof_reconcile_rejects_third_same_tree_commit() {
    assert_ref_log_final_proof_rejects_stale_target(true, true);
}

#[cfg(unix)]
fn assert_ref_log_final_proof_rejects_stale_target(reconcile: bool, third_commit: bool) {
    use std::os::unix::fs::MetadataExt;

    let (root, data, service, request) = protocol_resolution_fixture();
    let repository = git2::Repository::open(root.path()).unwrap();
    repository
        .config()
        .unwrap()
        .set_bool("core.logallrefupdates", false)
        .unwrap();
    let logs = [
        repository.path().join("logs/refs/heads/main"),
        repository.path().join("logs/HEAD"),
        repository.path().join("logs/refs/heads/substitute"),
    ];
    for log in &logs[..2] {
        fs::remove_file(log).unwrap();
    }
    let old = repository.refname_to_id("HEAD").unwrap();
    let merge_images =
        RESOLUTION_MERGE_MEMBERS.map(|member| fs::read(repository.path().join(member)).ok());
    assert!(merge_images[0].is_some());
    if reconcile {
        *service.failure_point.lock().unwrap() =
            Some(FailurePoint::ResolutionAfterCandidatePrepared);
        assert!(service.resolve_synchronization(request.clone()).is_err());
        assert_eq!(repository.refname_to_id("HEAD").unwrap(), old);
    }

    struct EffectSnapshot {
        candidate: git2::Oid,
        live: git2::Oid,
        files: Vec<(PathBuf, Vec<u8>, [u64; 2])>,
    }
    let snapshot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let hook_snapshot = snapshot.clone();
    let hook_root = root.path().to_owned();
    set_resolution_ref_refresh_hook(root.path().to_owned(), move || {
        let repository = git2::Repository::open(hook_root).unwrap();
        let candidate = repository.refname_to_id("HEAD").unwrap();
        assert_ne!(candidate, old);
        let commit = repository.find_commit(candidate).unwrap();
        assert_eq!(commit.parent_count(), 2);
        assert_eq!(commit.parent_id(0).unwrap(), old);
        let mut index = repository.index().unwrap();
        index.read(true).unwrap();
        assert!(!index.has_conflicts());
        assert_eq!(index.write_tree_to(&repository).unwrap(), commit.tree_id());
        let live = if third_commit {
            let signer = git2::Signature::now("External", "external@example.invalid").unwrap();
            let foreign = repository
                .commit(
                    None,
                    &signer,
                    &signer,
                    "External same-tree commit",
                    &commit.tree().unwrap(),
                    &[&commit],
                )
                .unwrap();
            assert_ne!(foreign, candidate);
            // Fixture-only direct mutation models stale state at the final proof
            // checkpoint; it does not promise namespace CAS for uncoordinated writers.
            fs::write(
                repository.path().join("refs/heads/main"),
                format!("{foreign}\n"),
            )
            .unwrap();
            foreign
        } else {
            fs::write(
                repository.path().join("refs/heads/substitute"),
                format!("{candidate}\n"),
            )
            .unwrap();
            fs::write(
                repository.path().join("HEAD"),
                b"ref: refs/heads/substitute\n",
            )
            .unwrap();
            candidate
        };
        let mut files = vec![
            repository.path().join("HEAD"),
            repository.path().join("refs/heads/main"),
            repository.path().join("index"),
            repository.path().join("index.lock"),
        ];
        if !third_commit {
            files.push(repository.path().join("refs/heads/substitute"));
        }
        for member in RESOLUTION_MERGE_MEMBERS {
            let path = repository.path().join(member);
            if path.exists() {
                files.push(path);
            }
        }
        let files = files
            .into_iter()
            .map(|path| {
                let meta = fs::metadata(&path).unwrap();
                let bytes = fs::read(&path).unwrap();
                (path, bytes, [meta.dev(), meta.ino()])
            })
            .collect();
        *hook_snapshot.lock().unwrap() = Some(EffectSnapshot {
            candidate,
            live,
            files,
        });
    });

    let restarted = RepositoryService::open_at(data.path()).unwrap();
    let result = if reconcile {
        restarted.resolve_synchronization(request.clone())
    } else {
        service.resolve_synchronization(request.clone())
    };
    let snapshot = snapshot
        .lock()
        .unwrap()
        .take()
        .expect("post-backend hook ran");
    assert!(
        matches!(result, Err(SynchronizationError::RecoveryRequired)),
        "stale final proof must reject completion"
    );
    let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    // A second attempt must preserve the observed foreign state without repeating
    // the native transition, observing a checkpoint, or retiring owned evidence.
    for retry in [false, true] {
        if retry {
            assert!(restarted.resolve_synchronization(request.clone()).is_err());
        }
        assert_eq!(repository.refname_to_id("HEAD").unwrap(), snapshot.live);
        assert_eq!(
            repository.refname_to_id("refs/heads/main").unwrap(),
            if third_commit {
                snapshot.live
            } else {
                snapshot.candidate
            }
        );
        assert_eq!(
            repository.find_reference("HEAD").unwrap().symbolic_target(),
            Some(if third_commit {
                "refs/heads/main"
            } else {
                "refs/heads/substitute"
            })
        );
        for (path, bytes, identity) in &snapshot.files {
            assert!(fs::read(path).unwrap() == *bytes, "effect image preserved");
            let meta = fs::metadata(path).unwrap();
            assert_eq!([meta.dev(), meta.ino()], *identity);
        }
        for log in &logs {
            assert!(!log.exists(), "disabled absent logs stay absent");
        }
        assert!(
            RESOLUTION_MERGE_MEMBERS.map(|member| fs::read(repository.path().join(member)).ok())
                == merge_images
        );
        assert!(
            fs::read(root.path().join("docs/document.md")).unwrap()
                == request.resolutions[0].1.bytes()
        );
        let progress: (String, String, String, Option<String>, String, Option<String>, String) =
            connection.query_row(
                "SELECT artifact.phase,artifact.ref_phase,attempt.phase,attempt.checkpoint_oid,step.phase,step.result_oid,attempt.candidate_oid FROM remote_resolution_index_artifacts artifact JOIN remote_resolution_attempts attempt ON attempt.id=artifact.attempt_id JOIN remote_integration_steps step ON step.id=attempt.integration_step_id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            ).unwrap();
        assert_eq!(
            progress,
            (
                "published".into(),
                "intent".into(),
                "candidate_prepared".into(),
                None,
                "commit_prepared".into(),
                None,
                snapshot.candidate.to_string(),
            )
        );
    }
}

#[cfg(any(unix, windows))]
#[test]
fn ref_log_proof_no_effect_intent_restarts_with_original_images() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let database = data.path().join(REGISTRY_FILE);
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        observed.store(true, Ordering::SeqCst);
        rusqlite::Connection::open(database)
            .unwrap()
            .execute_batch("UPDATE remote_resolution_index_artifacts SET ref_phase='intent';")
            .unwrap();
        panic!("fixture stop after intent, before backend");
    });
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        service.resolve_synchronization(request.clone())
    }));
    assert!(
        fired.load(Ordering::SeqCst),
        "no-effect intent installation hook fired"
    );
    assert!(stopped.is_err());
    let restarted = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        restarted.resolve_synchronization(request).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

#[cfg(any(unix, windows))]
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

#[cfg(unix)]
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

#[cfg(any(unix, windows))]
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

#[cfg(any(unix, windows))]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(any(unix, windows))]
#[test]
fn ref_log_proof_legacy_missing_all_evidence_refuses_even_old_no_effect_state() {
    let (root, data, service, request) = protocol_resolution_fixture();
    let fired = Arc::new(AtomicBool::new(false));
    let observed = fired.clone();
    set_resolution_index_install_hook(root.path().to_owned(), move || {
        observed.store(true, Ordering::SeqCst);
        panic!("fixture old HEAD with candidate")
    });
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        service.resolve_synchronization(request.clone())
    }));
    assert!(
        fired.load(Ordering::SeqCst),
        "legacy-evidence installation hook fired"
    );
    assert!(stopped.is_err());
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

#[cfg(any(unix, windows))]
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

#[cfg(unix)]
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

#[cfg(any(unix, windows))]
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

// Task 5 remainder A, B, C and G: injected stops at the durable transitions of
// integration, metadata retirement and publication; partial ref/log effects on
// the generic path; composition of offline restarts; and observers at the
// safe points added with publication attempts and metadata retirement.

/// phase, completed step, checkpoint, authority, index-pending flag, epoch.
type EnvelopeRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    i64,
);

impl PassFixture {
    /// A later process: a new service instance explicitly restarts the same
    /// operation, so nothing in memory survives the stop.
    fn restart_in_new_process(&mut self) {
        self.service = RepositoryService::open_at(self.data.path()).unwrap();
        self.restart();
    }

    fn step(&self, window: u32) -> Option<state::IntegrationStepEvidence> {
        state::with_transaction(&self.service, self.root.path(), |tx, id| {
            state::integration_step_in_window(
                tx,
                state::read_operation(tx, id, self.operation)?.unwrap().id,
                window,
                0,
            )
        })
        .unwrap()
    }

    fn step_count(&self) -> i64 {
        self.db()
            .query_row("SELECT count(*) FROM remote_integration_steps", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn merge_metadata_phases(&self) -> Vec<String> {
        let db = self.db();
        let mut statement = db
            .prepare(
                "SELECT phase FROM remote_integration_merge_metadata ORDER BY integration_step_id",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn envelope(&self) -> EnvelopeRow {
        self.db()
            .query_row(
                "SELECT phase,completed_step,sync_checkpoint,authoritative_oid,index_pending,owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",
                [self.operation.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap()
    }
}

/// Bytes of each merge-metadata member of one gitdir, None when absent.
fn merge_member_images(gitdir: &Path) -> Vec<Option<Vec<u8>>> {
    RESOLUTION_MERGE_MEMBERS
        .iter()
        .map(|member| match fs::read(gitdir.join(member)) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("fixture metadata read category: {:?}", error.kind()),
        })
        .collect()
}

/// HEAD and primary-branch ref logs, in that order.
fn ref_log_bytes(repository: &git2::Repository) -> [Vec<u8>; 2] {
    ["logs/HEAD", "logs/refs/heads/main"]
        .map(|name| fs::read(repository.path().join(name)).unwrap())
}

fn ref_log_lines(log: &[u8]) -> usize {
    log.iter().filter(|byte| **byte == b'\n').count()
}

fn reachable_merge_commits(repository: &git2::Repository) -> usize {
    let mut walk = repository.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.filter(|oid| {
        repository
            .find_commit(*oid.as_ref().unwrap())
            .unwrap()
            .parent_count()
            == 2
    })
    .count()
}

/// An external tool commits the whole merge with the exact ordered parents and
/// a clean index and worktree, but never cleans up merge state. With `logged`
/// its ref update also appended ref logs, as stock Git does.
fn commit_external_repair(
    repository: &git2::Repository,
    local: git2::Oid,
    incoming: git2::Oid,
    logged: bool,
) -> git2::Oid {
    let signature = repository.signature().unwrap();
    let local_parent = repository.find_commit(local).unwrap();
    let incoming_parent = repository.find_commit(incoming).unwrap();
    let repaired = repository
        .commit(
            None,
            &signature,
            &signature,
            "external repair",
            &local_parent.tree().unwrap(),
            &[&local_parent, &incoming_parent],
        )
        .unwrap();
    if logged {
        repository
            .reference("refs/heads/main", repaired, true, "external repair")
            .unwrap();
    } else {
        fs::write(
            repository.path().join("refs/heads/main"),
            format!("{repaired}\n"),
        )
        .unwrap();
    }
    repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    repaired
}

/// Every durable synchronization evidence row. Only the ownership epoch and
/// the update timestamp, which each explicit restart advances, are left out.
fn durable_evidence(db: &rusqlite::Connection) -> Vec<String> {
    let mut rows = Vec::new();
    for table in [
        "remote_operation_records",
        "remote_integration_windows",
        "remote_integration_steps",
        "remote_integration_merge_metadata",
        "remote_publication_attempts",
        "remote_observation_batches",
        "remote_ref_observations",
    ] {
        let mut statement = db
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
            .unwrap();
        let names = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mut query = statement.query([]).unwrap();
        while let Some(row) = query.next().unwrap() {
            let mut line = table.to_owned();
            for (column, name) in names.iter().enumerate() {
                if table == "remote_operation_records"
                    && matches!(name.as_str(), "owner_epoch" | "updated_at")
                {
                    continue;
                }
                line.push_str(&format!(" {name}={:?}", row.get_ref(column).unwrap()));
            }
            rows.push(line);
        }
    }
    rows
}

const DROP_INJECTED_STOP: &str = "DROP TRIGGER IF EXISTS injected_stop";

fn stop_before(db: &rusqlite::Connection, event: &str) {
    db.execute_batch(&format!(
        "CREATE TRIGGER injected_stop BEFORE {event} BEGIN SELECT RAISE(ABORT,'fixture'); END;"
    ))
    .unwrap();
}

/// A: the clean-merge stage of the real ordered integration, stopped at each
/// of its durable writes (stage intent, effect intent after the candidate
/// object exists, effect observation after the ref moved). A later process
/// resumes the frozen stage and the branch moves exactly once to one merge of
/// the exact recorded parents; nothing is regenerated behind a moved ref.
#[test]
fn clean_merge_integration_fault_matrix_resumes_each_durable_transition_once() {
    for point in ["stage_intent", "effect_intent", "effect_observation"] {
        let mut fixture = PassFixture::new("local.txt", b"local\n");
        let repository = fixture.repository();
        let local = fixture.local;
        let incoming = child_file(&repository, fixture.base, "remote.txt", b"remote\n");
        fixture.fetch(incoming, 1);
        fixture.append_window(1, incoming).unwrap();
        let before = local_binding_image(fixture.root.path());
        let before_logs = ref_log_bytes(&repository);
        let db = fixture.db();
        stop_before(
            &db,
            match point {
                "stage_intent" => "INSERT ON remote_integration_steps",
                "effect_intent" => {
                    "UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applying'"
                }
                _ => "UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied'",
            },
        );
        assert!(journal_stop(&fixture.integrate(incoming)), "{point}");
        db.execute_batch(DROP_INJECTED_STOP).unwrap();
        let stopped_head = repository.head().unwrap().target().unwrap();
        let stopped_logs = ref_log_bytes(&repository);
        let stopped = fixture.step(1);
        if point == "effect_observation" {
            // The ref effect happened; only its observation was lost.
            let stopped = stopped.unwrap();
            assert_eq!(stopped.phase, state::IntegrationStepPhase::Applying);
            assert_eq!(stopped.candidate_oid, Some(stopped_head));
            assert_eq!(stopped.result_oid, None);
            assert_ne!(stopped_head, local);
        } else {
            assert_eq!(stopped_head, local, "{point}");
            assert_eq!(local_binding_image(fixture.root.path()), before, "{point}");
            assert_eq!(stopped_logs, before_logs, "{point}");
            assert_eq!(
                stopped.map(|step| (step.phase, step.candidate_oid)),
                (point == "effect_intent").then_some((state::IntegrationStepPhase::Prepared, None)),
                "{point}"
            );
        }
        fixture.restart_in_new_process();
        let mut evidence = fixture.record().sync_evidence;
        let merged = match fixture.reconcile(&mut evidence).unwrap() {
            Some(observed) => {
                // Observation only: no second checkout, ref move or log line.
                assert_eq!(point, "effect_observation");
                assert_eq!(ref_log_bytes(&repository), stopped_logs);
                observed.oid
            }
            None => {
                // No effect was proven, so the frozen stage itself resumes.
                assert_ne!(point, "effect_observation");
                fixture.integrate(incoming).unwrap()
            }
        };
        assert_eq!(repository.head().unwrap().target(), Some(merged), "{point}");
        let merge = repository.find_commit(merged).unwrap();
        assert_eq!(
            merge.parent_ids().collect::<Vec<_>>(),
            [local, incoming],
            "{point}"
        );
        assert_eq!(reachable_merge_commits(&repository), 1, "{point}");
        assert!(repository.statuses(None).unwrap().is_empty(), "{point}");
        let step = fixture.step(1).unwrap();
        assert_eq!(step.phase, state::IntegrationStepPhase::Applied, "{point}");
        assert_eq!(step.candidate_oid, Some(merged), "{point}");
        assert_eq!(step.result_oid, Some(merged), "{point}");
        assert_eq!(
            (step.intent.local_oid, step.intent.incoming_oid),
            (local, incoming)
        );
        assert_eq!(fixture.step_count(), 1, "{point}");
        // Exactly one ref transition in total, appended to both logs.
        let logs = ref_log_bytes(&repository);
        for (log, original) in logs.iter().zip(&before_logs) {
            assert!(log.starts_with(original), "{point}");
            assert_eq!(ref_log_lines(log), ref_log_lines(original) + 1, "{point}");
        }
        // A further restart only observes the completed stage.
        let completed = local_binding_image(fixture.root.path());
        fixture.restart_in_new_process();
        let mut evidence = fixture.record().sync_evidence;
        assert_eq!(
            fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
            merged
        );
        assert_eq!(ref_log_bytes(&repository), logs, "{point}");
        assert_eq!(local_binding_image(fixture.root.path()), completed);
        assert_eq!(fixture.step_count(), 1, "{point}");
    }
}

/// A: the clean stage is applied and observed, but the write that carries it
/// into the synchronization envelope (the merge-applied checkpoint) is not
/// durable. A later process observes the applied stage, and after its Fetch
/// commits that one candidate into the envelope without another merge, ref
/// move or stage row.
#[test]
fn merge_applied_checkpoint_stop_is_reconciled_from_the_applied_stage() {
    let mut fixture = PassFixture::new("local.txt", b"local\n");
    let repository = fixture.repository();
    let local = fixture.local;
    let incoming = child_file(&repository, fixture.base, "remote.txt", b"remote\n");
    fixture.fetch(incoming, 1);
    fixture.append_window(1, incoming).unwrap();
    let merged = fixture.integrate(incoming).unwrap();
    let applied = state::SynchronizationEvidence {
        expected_oid: Some(local),
        local_oid: Some(merged),
        tracking_oid: Some(incoming),
        primary_tracking_oid: Some(incoming),
        ..Default::default()
    };
    let db = fixture.db();
    stop_before(
        &db,
        "UPDATE OF sync_checkpoint ON remote_operation_records WHEN NEW.sync_checkpoint='local_fast_forwarded'",
    );
    assert!(matches!(
        fixture.service.checkpoint_synchronization_merge_applied(
            fixture.root.path(),
            &fixture.owner,
            &applied
        ),
        Err(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        })
    ));
    db.execute_batch(DROP_INJECTED_STOP).unwrap();
    let stopped = fixture.record();
    assert_eq!(
        stopped.sync_checkpoint,
        Some(state::SynchronizationCheckpoint::FetchObserved)
    );
    assert_eq!(stopped.sync_evidence.local_oid, Some(local));
    let image = inspection_git_image(fixture.root.path());
    fixture.restart_in_new_process();
    let mut evidence = fixture.record().sync_evidence;
    let observed = fixture.reconcile(&mut evidence).unwrap().unwrap();
    assert_eq!(observed.oid, merged);
    assert_eq!(evidence.local_oid, Some(merged));
    fixture.fetch(incoming, 2);
    evidence.tracking_oid = Some(incoming);
    evidence.primary_tracking_oid = Some(incoming);
    finalize_reconciled_candidate(
        &fixture.service,
        fixture.root.path(),
        &fixture.owner,
        observed,
        &evidence,
    )
    .unwrap();
    let record = fixture.record();
    assert_eq!(
        record.sync_checkpoint,
        Some(state::SynchronizationCheckpoint::LocalFastForwarded)
    );
    assert_eq!(record.sync_evidence.local_oid, Some(merged));
    assert_eq!(record.sync_evidence.push_oid, None);
    assert!(!record.reconciliation_required);
    assert_eq!(inspection_git_image(fixture.root.path()), image);
    assert_eq!(repository.head().unwrap().target(), Some(merged));
    assert_eq!(reachable_merge_commits(&repository), 1);
    assert_eq!(fixture.step_count(), 1);
}

/// The typed failure of a durable journal write that could not commit.
fn journal_stop<T>(result: &Result<T, SynchronizationError>) -> bool {
    matches!(
        result,
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        }))
    )
}

/// A reserved context synchronization whose remote context and remote primary
/// each gained one commit that merges cleanly into the local context branch.
/// One pinned window is recorded and no stage has started. The publication
/// remote is configured, so the public entry point reaches transport.
struct ContextPass {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    plan: RemoteRefPlan,
    target: SynchronizationTarget,
    operation_target: RemoteOperationTarget,
    operation: OperationId,
    owner: RemoteReservation,
    /// The context worktree.
    repository: git2::Repository,
    selected: RemoteRefTarget,
    main: git2::Oid,
    local: git2::Oid,
    context_incoming: git2::Oid,
    primary_incoming: git2::Oid,
}

impl ContextPass {
    fn new() -> Self {
        let (root, data, service) = fixture();
        let primary_repository = git2::Repository::open(root.path()).unwrap();
        primary_repository
            .remote("origin", "ssh://example.invalid/fixture.git")
            .unwrap();
        fs::write(
            root.path().join(".manyhands/config.toml"),
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
        )
        .unwrap();
        let main = commit_all(&primary_repository);
        let target = materialize_local_context(&service, root.path());
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let configuration = service
            .observation_configuration(root.path(), &plan)
            .unwrap();
        state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&plan), false)?;
            state::configure_endpoints(tx, id, &plan, &configuration.endpoint_digest())
        })
        .unwrap();
        let repository = materialized_target(root.path(), "main", &target).unwrap();
        let local = repository.head().unwrap().target().unwrap();
        let context_incoming = child_file(&repository, main, "remote-context.txt", b"context\n");
        let primary_incoming = child_file(&repository, main, "remote-primary.txt", b"primary\n");
        let selected = target_ref(&plan, &target);
        for (reference, oid) in [
            (plan.primary().tracking_ref(), primary_incoming),
            (selected.tracking_ref(), context_incoming),
        ] {
            repository
                .reference(reference, oid, true, "fixture observation")
                .unwrap();
        }
        let operation_target = target.operation_target(&plan);
        let operation = OperationId::new();
        let RemoteReservationOutcome::Reserved(owner) = service
            .reserve_remote_operation(root.path(), operation, &operation_target)
            .unwrap()
        else {
            panic!("reservation")
        };
        service
            .checkpoint_synchronization(
                root.path(),
                &owner,
                state::SynchronizationCheckpoint::FetchPrepared,
                &state::SynchronizationEvidence {
                    expected_oid: Some(local),
                    local_oid: Some(local),
                    ..Default::default()
                },
            )
            .unwrap();
        service
            .remote_safe_point(root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
            .unwrap();
        let observations = [
            RemoteRefObservation::from_advertisement(
                &plan,
                "refs/heads/main",
                primary_incoming,
                Some(primary_incoming),
            )
            .unwrap(),
            RemoteRefObservation::from_advertisement(
                &plan,
                selected.remote_ref(),
                context_incoming,
                Some(context_incoming),
            )
            .unwrap(),
        ];
        commit_observation_batch(&service, root.path(), &owner, &plan, &observations, 1).unwrap();
        let pass = Self {
            root,
            data,
            service,
            plan,
            target,
            operation_target,
            operation,
            owner,
            repository,
            selected,
            main,
            local,
            context_incoming,
            primary_incoming,
        };
        let batch = pass
            .db()
            .query_row(
                "SELECT id FROM remote_observation_batches WHERE is_current=1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        pass.service
            .prepare_synchronization_window(
                pass.root.path(),
                &pass.owner,
                1,
                &state::IntegrationWindowIntent {
                    observation_batch_id: batch,
                    local_oid: local,
                    primary_oid: primary_incoming,
                    context_oid: Some(context_incoming),
                },
            )
            .unwrap();
        pass
    }

    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    /// The production ordered pass: fetched context first, then primary.
    fn integrate(&self) -> Result<git2::Oid, SynchronizationError> {
        let configuration = self
            .service
            .observation_configuration(self.root.path(), &self.plan)
            .unwrap();
        let mut req = request(self.root.path());
        req.operation_id = self.operation;
        req.target = self.target.clone();
        integrate_divergence(
            &self.service,
            DivergenceInputs {
                root: self.root.path(),
                primary_branch: "main",
                target: &self.target,
                request: &req,
                owner: &self.owner,
                plan: &self.plan,
                configuration: &configuration,
                selected: &self.selected,
                primary_tracking: Some(self.primary_incoming),
                selected_tracking: Some(self.context_incoming),
                context: Some(self.context_incoming),
                primary: self.primary_incoming,
            },
        )
    }

    fn stage(&self, ordinal: u8) -> Option<state::IntegrationStepEvidence> {
        state::with_transaction(&self.service, self.root.path(), |tx, id| {
            state::integration_step_in_window(
                tx,
                state::read_operation(tx, id, self.operation)?.unwrap().id,
                1,
                ordinal,
            )
        })
        .unwrap()
    }

    fn head(&self) -> git2::Oid {
        self.repository.head().unwrap().target().unwrap()
    }

    fn parents(&self, oid: git2::Oid) -> Vec<git2::Oid> {
        self.repository
            .find_commit(oid)
            .unwrap()
            .parent_ids()
            .collect()
    }

    fn head_log(&self) -> Vec<u8> {
        fs::read(self.repository.path().join("logs/HEAD")).unwrap()
    }
}

/// A: a context synchronization stopped between its ordered stages, after the
/// fetched context was merged and before the primary stage has any durable
/// intent. Deliberate restarts through the public entry point, with no
/// transport, keep that context merge exactly as recorded, resume only the
/// primary stage on top of it once, and then change nothing further.
#[test]
fn stop_between_context_and_primary_stages_resumes_only_the_primary_stage() {
    let pass = ContextPass::new();
    let db = pass.db();
    let original_log = pass.head_log();
    stop_before(&db, "INSERT ON remote_integration_steps WHEN NEW.ordinal=1");
    assert!(journal_stop(&pass.integrate()));
    db.execute_batch(DROP_INJECTED_STOP).unwrap();
    let context_merge = pass.head();
    assert_eq!(
        pass.parents(context_merge),
        [pass.local, pass.context_incoming]
    );
    let context_stage = pass.stage(0).unwrap();
    assert_eq!(context_stage.phase, state::IntegrationStepPhase::Applied);
    assert_eq!(context_stage.result_oid, Some(context_merge));
    assert!(pass.stage(1).is_none());
    let stopped_log = pass.head_log();
    assert_eq!(
        ref_log_lines(&stopped_log),
        ref_log_lines(&original_log) + 1
    );
    let mut retry = request(pass.root.path());
    retry.operation_id = pass.operation;
    retry.target = pass.target.clone();
    retry.restart = true;
    let mut merged = None;
    for _ in 0..2 {
        // A later process: the applied context stage is only observed and the
        // frozen pass resumes at the primary stage before any transport.
        assert!(matches!(
            RepositoryService::open_at(pass.data.path())
                .unwrap()
                .synchronize_remote(retry.clone(), &mut SessionCredentials::new(NoPrompt)),
            Err(SynchronizationError::Transport(_))
        ));
        let head = pass.head();
        assert_eq!(*merged.get_or_insert(head), head);
        assert_eq!(pass.parents(head), [context_merge, pass.primary_incoming]);
        assert_eq!(pass.stage(0).unwrap(), context_stage);
        let primary_stage = pass.stage(1).unwrap();
        assert_eq!(primary_stage.phase, state::IntegrationStepPhase::Applied);
        assert_eq!(primary_stage.result_oid, Some(head));
        assert_eq!(
            (
                primary_stage.intent.local_oid,
                primary_stage.intent.incoming_oid
            ),
            (context_merge, pass.primary_incoming)
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_integration_steps", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            2
        );
        assert_eq!(reachable_merge_commits(&pass.repository), 2);
        let log = pass.head_log();
        assert!(log.starts_with(&stopped_log));
        assert_eq!(ref_log_lines(&log), ref_log_lines(&stopped_log) + 1);
        assert!(pass.repository.statuses(None).unwrap().is_empty());
    }
    // Only the context branch moved; the primary checkout is untouched.
    assert_eq!(
        git2::Repository::open(pass.root.path())
            .unwrap()
            .refname_to_id("refs/heads/main")
            .unwrap(),
        pass.main
    );
}

/// A: one injected stop at every durable transition of owned merge-metadata
/// retirement: the digest record, `retire_intent`, each unlink, `retired` and
/// the stage observation that follows. The stop loses nothing (no ref, log,
/// index, worktree or object effect; members not yet unlinked keep their
/// bytes) and a later process converges on exactly one retirement and one
/// observed stage. A conflict whose digest record was lost stays typed
/// Recovery after external repair, with its remnant preserved (accepted limit).
#[test]
fn merge_metadata_retirement_fault_matrix_converges_once_without_loss() {
    for point in [
        "record",
        "retire_intent",
        "unlink_1",
        "unlink_2",
        "unlink_3",
        "retired",
        "observation",
    ] {
        let mut fixture = PassFixture::new("fixture.txt", b"local\n");
        let repository = fixture.repository();
        let gitdir = repository.path().to_owned();
        let root = fixture.root.path().to_owned();
        let local = fixture.local;
        let incoming = child(&repository, fixture.base, b"incoming\n");
        fixture.fetch(incoming, 1);
        fixture.append_window(1, incoming).unwrap();
        let db = fixture.db();
        if point == "record" {
            stop_before(&db, "INSERT ON remote_integration_merge_metadata");
            // The conflict is installed, but the process stops before its
            // metadata digests and its conflict observation are durable.
            assert!(matches!(
                fixture.integrate(incoming),
                Err(error) if !matches!(error, SynchronizationError::ConflictPending { .. })
            ));
            db.execute_batch(DROP_INJECTED_STOP).unwrap();
            let installed = inspection_git_image(&root);
            assert_eq!(
                fixture.step(1).unwrap().phase,
                state::IntegrationStepPhase::Applying
            );
            assert!(repository.index().unwrap().has_conflicts());
            fixture.restart_in_new_process();
            let mut evidence = fixture.record().sync_evidence;
            assert!(matches!(
                fixture.reconcile(&mut evidence),
                Err(SynchronizationError::ConflictPending { .. })
            ));
            assert_eq!(inspection_git_image(&root), installed);
            assert_eq!(
                fixture.step(1).unwrap().phase,
                state::IntegrationStepPhase::ConflictPending
            );
            assert!(fixture.merge_metadata_phases().is_empty());
            // Nothing recorded these members as this operation's own, so an
            // external repair that leaves them behind is never cleaned up.
            let repaired = commit_external_repair(&repository, local, incoming, true);
            let members = merge_member_images(&gitdir);
            assert!(members.iter().all(Option::is_some));
            let before = inspection_git_image(&root);
            fixture.restart_in_new_process();
            let mut evidence = fixture.record().sync_evidence;
            assert!(matches!(
                fixture.reconcile(&mut evidence),
                Err(SynchronizationError::RecoveryRequired)
            ));
            assert_eq!(inspection_git_image(&root), before);
            assert_eq!(merge_member_images(&gitdir), members);
            assert_eq!(repository.head().unwrap().target(), Some(repaired));
            let step = fixture.step(1).unwrap();
            assert_eq!(step.phase, state::IntegrationStepPhase::ConflictPending);
            assert_eq!(step.result_oid, None);
            continue;
        }
        assert!(matches!(
            fixture.integrate(incoming),
            Err(SynchronizationError::ConflictPending { .. })
        ));
        let repaired = commit_external_repair(&repository, local, incoming, true);
        let members = merge_member_images(&gitdir);
        assert!(members.iter().all(Option::is_some), "{point}");
        let before = local_binding_image(&root);
        let before_logs = reflog_image(&repository);
        let before_objects = inspection_odb_inventory(&repository);
        let event = match point {
            "retire_intent" => Some(
                "UPDATE OF phase ON remote_integration_merge_metadata WHEN NEW.phase='retire_intent'",
            ),
            "retired" => Some(
                "UPDATE OF phase ON remote_integration_merge_metadata WHEN NEW.phase='retired'",
            ),
            "observation" => {
                Some("UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied'")
            }
            _ => None,
        };
        let unlinked = match point {
            "retire_intent" => 0,
            "unlink_1" => 1,
            "unlink_2" => 2,
            _ => 3,
        };
        if let Some(event) = event {
            stop_before(&db, event);
        } else {
            // The process dies directly after the chosen unlink.
            for _ in 1..unlinked {
                set_merge_metadata_unlinked_hook(root.clone(), || {});
            }
            set_merge_metadata_unlinked_hook(root.clone(), || {
                panic!("fixture process stop after an unlink")
            });
        }
        fixture.restart();
        let mut evidence = fixture.record().sync_evidence;
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            fixture.reconcile(&mut evidence)
        }));
        match (event, stopped) {
            (Some(_), Ok(result)) => assert!(journal_stop(&result), "{point}"),
            (None, Err(_)) => {}
            _ => panic!("{point}: the injected stop did not fire"),
        }
        db.execute_batch(DROP_INJECTED_STOP).unwrap();
        let after_stop = merge_member_images(&gitdir);
        for (ordinal, (image, original)) in after_stop.iter().zip(&members).enumerate() {
            if ordinal < unlinked {
                assert_eq!(image, &None, "{point} {ordinal}");
            } else {
                assert_eq!(image, original, "{point} {ordinal}");
            }
        }
        assert_eq!(
            fixture.merge_metadata_phases(),
            [match point {
                "retire_intent" => "recorded",
                "observation" => "retired",
                _ => "retire_intent",
            }],
            "{point}"
        );
        let step = fixture.step(1).unwrap();
        assert_eq!(
            step.phase,
            state::IntegrationStepPhase::ConflictPending,
            "{point}"
        );
        assert_eq!(step.result_oid, None, "{point}");
        assert_eq!(local_binding_image(&root), before, "{point}");
        assert_eq!(reflog_image(&repository), before_logs, "{point}");
        assert_eq!(
            inspection_odb_inventory(&repository),
            before_objects,
            "{point}"
        );
        // A later process finishes the same retirement and observes the stage.
        fixture.restart_in_new_process();
        let mut evidence = fixture.record().sync_evidence;
        assert_eq!(
            fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
            repaired,
            "{point}"
        );
        assert_eq!(merge_member_images(&gitdir), [None, None, None], "{point}");
        assert_eq!(fixture.merge_metadata_phases(), ["retired"], "{point}");
        let step = fixture.step(1).unwrap();
        assert_eq!(step.phase, state::IntegrationStepPhase::Applied, "{point}");
        assert_eq!(step.result_oid, Some(repaired), "{point}");
        assert_eq!(
            (step.intent.local_oid, step.intent.incoming_oid),
            (local, incoming)
        );
        assert_eq!(fixture.step_count(), 1, "{point}");
        assert_eq!(repository.head().unwrap().target(), Some(repaired));
        assert_eq!(
            git2::Repository::open(&root).unwrap().state(),
            git2::RepositoryState::Clean,
            "{point}"
        );
        // Convergence is a fixed point: nothing further is retired or moved.
        fixture.restart_in_new_process();
        let mut evidence = fixture.record().sync_evidence;
        assert_eq!(
            fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
            repaired
        );
        assert_eq!(fixture.merge_metadata_phases(), ["retired"], "{point}");
        assert_eq!(local_binding_image(&root), before, "{point}");
        assert_eq!(reflog_image(&repository), before_logs, "{point}");
        assert_eq!(
            inspection_odb_inventory(&repository),
            before_objects,
            "{point}"
        );
    }
}

/// B: the generic integration stage has no owned ref-log manifest. After its
/// real libgit2 ref transaction, a restart observes the exact recorded
/// candidate from the ref alone and never repairs, rewrites or re-appends a
/// partial ref log; a log entry without the ref move, and every ambiguous
/// files-backend lock role, is preserved byte-for-byte as typed Recovery
/// instead of being replayed.
#[test]
fn generic_candidate_restart_preserves_partial_ref_and_log_effects() {
    for variant in [
        "complete",
        "ref_without_branch_log",
        "ref_without_head_log",
        "ref_without_logs",
        "logs_without_ref",
        "branch_ref_lock",
        "branch_log_lock",
        "head_lock",
        "head_log_lock",
        "packed_refs_lock",
    ] {
        let mut fixture = PassFixture::new("local.txt", b"local\n");
        let repository = fixture.repository();
        let gitdir = repository.path().to_owned();
        let local = fixture.local;
        let incoming = child_file(&repository, fixture.base, "remote.txt", b"remote\n");
        fixture.fetch(incoming, 1);
        fixture.append_window(1, incoming).unwrap();
        let original_logs = ref_log_bytes(&repository);
        let db = fixture.db();
        stop_before(
            &db,
            "UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied'",
        );
        assert!(journal_stop(&fixture.integrate(incoming)), "{variant}");
        db.execute_batch(DROP_INJECTED_STOP).unwrap();
        let candidate = repository.head().unwrap().target().unwrap();
        assert_ne!(candidate, local);
        let complete_logs = ref_log_bytes(&repository);
        for (complete, original) in complete_logs.iter().zip(&original_logs) {
            assert!(complete.starts_with(original) && complete.len() > original.len());
        }
        let branch = gitdir.join("refs/heads/main");
        let lock = match variant {
            "branch_ref_lock" => Some("refs/heads/main.lock"),
            "branch_log_lock" => Some("logs/refs/heads/main.lock"),
            "head_lock" => Some("HEAD.lock"),
            "head_log_lock" => Some("logs/HEAD.lock"),
            "packed_refs_lock" => Some("packed-refs.lock"),
            _ => None,
        };
        match variant {
            "ref_without_branch_log" => {
                fs::write(gitdir.join("logs/refs/heads/main"), &original_logs[1]).unwrap();
            }
            "ref_without_head_log" => {
                fs::write(gitdir.join("logs/HEAD"), &original_logs[0]).unwrap();
            }
            "ref_without_logs" => {
                fs::write(gitdir.join("logs/HEAD"), &original_logs[0]).unwrap();
                fs::write(gitdir.join("logs/refs/heads/main"), &original_logs[1]).unwrap();
            }
            "logs_without_ref" => {
                // Checkout and both log appends happened; the loose ref did not.
                fs::write(&branch, format!("{local}\n")).unwrap();
            }
            _ => {}
        }
        if let Some(lock) = lock {
            fs::write(gitdir.join(lock), b"operator-owned lock\n").unwrap();
        }
        let partial_logs = ref_log_bytes(&repository);
        let partial_ref = fs::read(&branch).unwrap();
        let before = local_binding_image(fixture.root.path());
        let before_objects = inspection_odb_inventory(&repository);
        fixture.restart_in_new_process();
        let mut evidence = fixture.record().sync_evidence;
        let result = fixture.reconcile(&mut evidence);
        // In every variant: no log repair or append, no second ref
        // transition, no index, worktree or object effect.
        assert_eq!(ref_log_bytes(&repository), partial_logs, "{variant}");
        assert_eq!(fs::read(&branch).unwrap(), partial_ref, "{variant}");
        assert_eq!(
            local_binding_image(fixture.root.path()),
            before,
            "{variant}"
        );
        assert_eq!(
            inspection_odb_inventory(&repository),
            before_objects,
            "{variant}"
        );
        let step = fixture.step(1).unwrap();
        assert_eq!(step.candidate_oid, Some(candidate), "{variant}");
        if lock.is_none() && variant != "logs_without_ref" {
            assert_eq!(result.unwrap().unwrap().oid, candidate, "{variant}");
            assert_eq!(
                step.phase,
                state::IntegrationStepPhase::Applied,
                "{variant}"
            );
            assert_eq!(step.result_oid, Some(candidate), "{variant}");
        } else {
            assert!(
                matches!(result, Err(SynchronizationError::RecoveryRequired)),
                "{variant}"
            );
            assert_eq!(
                step.phase,
                state::IntegrationStepPhase::Applying,
                "{variant}"
            );
            assert_eq!(step.result_oid, None, "{variant}");
            if let Some(lock) = lock {
                assert_eq!(
                    fs::read(gitdir.join(lock)).unwrap(),
                    b"operator-owned lock\n",
                    "{variant}"
                );
            }
        }
        assert_eq!(fixture.step_count(), 1, "{variant}");
    }
}

/// B: an external repair whose ref update left no ref-log entry is observed
/// from the exact two-parent commit alone. Retiring the owned merge metadata
/// never creates, appends or rewrites a ref log on the tool's behalf.
#[test]
fn external_repair_without_a_ref_log_entry_is_observed_without_writing_logs() {
    let mut fixture = PassFixture::new("fixture.txt", b"local\n");
    let repository = fixture.repository();
    let local = fixture.local;
    let incoming = child(&repository, fixture.base, b"incoming\n");
    fixture.fetch(incoming, 1);
    fixture.append_window(1, incoming).unwrap();
    assert!(matches!(
        fixture.integrate(incoming),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let conflict_logs = ref_log_bytes(&repository);
    let repaired = commit_external_repair(&repository, local, incoming, false);
    assert_eq!(ref_log_bytes(&repository), conflict_logs);
    let before = local_binding_image(fixture.root.path());
    fixture.restart_in_new_process();
    let mut evidence = fixture.record().sync_evidence;
    assert_eq!(
        fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
        repaired
    );
    assert_eq!(ref_log_bytes(&repository), conflict_logs);
    assert_eq!(local_binding_image(fixture.root.path()), before);
    assert_eq!(merge_member_images(repository.path()), [None, None, None]);
    assert_eq!(fixture.merge_metadata_phases(), ["retired"]);
    let step = fixture.step(1).unwrap();
    assert_eq!(step.phase, state::IntegrationStepPhase::Applied);
    assert_eq!(step.result_oid, Some(repaired));
}

/// C: repeated deliberate restarts through the public entry point with no
/// usable transport. The first may complete a recorded local stage (that is
/// local-first reconciliation, not new evidence); every later one reaches the
/// same durable rows, refs, logs, index, worktree and object set, and no
/// restart appends a window, stage, merge, attempt or observation batch.
#[test]
fn offline_public_restarts_reach_a_fixed_point_in_each_recorded_local_state() {
    for recorded in [
        "pending_conflict",
        "external_repair",
        "prepared_stage",
        "unobserved_candidate",
        "applied_candidate",
    ] {
        let conflict = matches!(recorded, "pending_conflict" | "external_repair");
        let fixture = PassFixture::published(
            if conflict { "fixture.txt" } else { "local.txt" },
            b"local\n",
        );
        let repository = fixture.repository();
        let root = fixture.root.path();
        let local = fixture.local;
        let incoming = if conflict {
            child(&repository, fixture.base, b"incoming\n")
        } else {
            child_file(&repository, fixture.base, "remote.txt", b"remote\n")
        };
        fixture.fetch(incoming, 1);
        fixture.append_window(1, incoming).unwrap();
        let original_logs = ref_log_bytes(&repository);
        let db = fixture.db();
        match recorded {
            "prepared_stage" => stop_before(
                &db,
                "UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applying'",
            ),
            "unobserved_candidate" => stop_before(
                &db,
                "UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied'",
            ),
            _ => {}
        }
        let integrated = fixture.integrate(incoming);
        db.execute_batch(DROP_INJECTED_STOP).unwrap();
        match recorded {
            "pending_conflict" | "external_repair" => assert!(matches!(
                integrated,
                Err(SynchronizationError::ConflictPending { .. })
            )),
            "applied_candidate" => {
                integrated.unwrap();
            }
            _ => assert!(journal_stop(&integrated), "{recorded}"),
        }
        let repaired = (recorded == "external_repair")
            .then(|| commit_external_repair(&repository, local, incoming, true));
        let recorded_image = inspection_git_image(root);
        let recorded_windows: i64 = db
            .query_row(
                "SELECT count(*) FROM remote_integration_windows",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut retry = request(root);
        retry.operation_id = fixture.operation;
        retry.restart = true;
        let mut observed = Vec::new();
        for _ in 0..3 {
            // Each restart is a new process; none may prompt for a credential.
            let error = RepositoryService::open_at(fixture.data.path())
                .unwrap()
                .synchronize_remote(retry.clone(), &mut SessionCredentials::new(NoPrompt))
                .expect_err("no transport is available");
            observed.push((
                error.to_string(),
                durable_evidence(&db),
                inspection_git_image(root),
                inspection_odb_inventory(&repository),
            ));
        }
        assert_eq!(observed[1], observed[0], "{recorded}");
        assert_eq!(observed[2], observed[0], "{recorded}");
        assert_eq!(fixture.step_count(), 1, "{recorded}");
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_integration_windows",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            recorded_windows,
            "{recorded}"
        );
        assert!(fixture.attempts().is_empty(), "{recorded}");
        let step = fixture.step(1).unwrap();
        let head = repository.head().unwrap().target().unwrap();
        match recorded {
            "pending_conflict" => {
                assert_eq!(observed[0].0, "synchronization conflict pending");
                assert_eq!(observed[0].2, recorded_image);
                assert_eq!(step.phase, state::IntegrationStepPhase::ConflictPending);
                assert_eq!(head, local);
            }
            "external_repair" => {
                assert_eq!(observed[0].0, "synchronization transport unavailable");
                assert_eq!(step.phase, state::IntegrationStepPhase::Applied);
                assert_eq!(step.result_oid, repaired);
                assert_eq!(Some(head), repaired);
                assert_eq!(fixture.merge_metadata_phases(), ["retired"]);
                assert_eq!(merge_member_images(repository.path()), [None, None, None]);
                // Only the external tool's own ref update was ever logged.
                for (log, original) in ref_log_bytes(&repository).iter().zip(&original_logs) {
                    assert!(log.starts_with(original));
                    assert!(ref_log_lines(log) <= ref_log_lines(original) + 1);
                }
            }
            _ => {
                assert_eq!(
                    observed[0].0, "synchronization transport unavailable",
                    "{recorded}"
                );
                if recorded != "prepared_stage" {
                    // The local effect predates the restarts: Git is untouched.
                    assert_eq!(observed[0].2, recorded_image, "{recorded}");
                }
                assert_eq!(
                    step.phase,
                    state::IntegrationStepPhase::Applied,
                    "{recorded}"
                );
                assert_eq!(step.result_oid, Some(head), "{recorded}");
                assert_eq!(
                    repository
                        .find_commit(head)
                        .unwrap()
                        .parent_ids()
                        .collect::<Vec<_>>(),
                    [local, incoming],
                    "{recorded}"
                );
                assert_eq!(reachable_merge_commits(&repository), 1, "{recorded}");
                for (log, original) in ref_log_bytes(&repository).iter().zip(&original_logs) {
                    assert!(log.starts_with(original), "{recorded}");
                    assert_eq!(
                        ref_log_lines(log),
                        ref_log_lines(original) + 1,
                        "{recorded}"
                    );
                }
            }
        }
    }
}

/// G: a superseded owner epoch and another service instance are refused at
/// every transition of a publication attempt, with no row changed, while the
/// current owner standing at the identical boundary is accepted. A durable
/// cancellation request is honored at each attempt write without advancing it.
#[test]
fn publication_transitions_fence_stale_owner_foreign_service_and_cancellation() {
    use RemoteOperationSafePoint as Point;
    type Transition<'a> = (
        &'static str,
        Box<
            dyn Fn(
                    &RepositoryService,
                    &RemoteReservation,
                ) -> Result<RemoteSafePointOutcome, RepositoryError>
                + 'a,
        >,
    );
    let refused = |result: Result<RemoteSafePointOutcome, RepositoryError>| {
        matches!(
            result,
            Err(RepositoryError {
                kind: RepositoryErrorKind::RecoveryRequired,
                ..
            })
        )
    };
    let (mut fixture, candidate, tip) = open_attempt_fixture(true);
    // A later explicit restart supersedes the owner that opened the attempt.
    let stale = fixture.restart_superseding();
    let mut evidence = fixture.record().sync_evidence;
    assert_eq!(
        fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
        candidate
    );
    fixture.fetch(tip, 3);
    let resume = PublicationSettlement {
        intent: PublicationIntent::Open,
        local_oid: candidate,
        continuation: false,
        advertised_oid: None,
        relation: None,
    };
    let foreign = RepositoryService::open_at(fixture.data.path()).unwrap();
    let root = fixture.root.path();
    let authority = state::SynchronizationAuthority::Published(candidate);
    let transitions: Vec<Transition<'_>> = vec![
        (
            "settle",
            Box::new(|service, owner| {
                service.settle_synchronization_publication(root, owner, &resume)
            }),
        ),
        (
            "prepare",
            Box::new(|service, owner| {
                service.prepare_synchronization_publication(
                    root,
                    owner,
                    candidate,
                    Some(tip),
                    false,
                )
            }),
        ),
        (
            "before_push",
            Box::new(|service, owner| service.remote_safe_point(root, owner, Point::BeforePush)),
        ),
        (
            "returned",
            Box::new(|service, owner| {
                service.advance_synchronization_publication(
                    root,
                    owner,
                    state::PublicationPhase::Returned,
                    None,
                )
            }),
        ),
        (
            "after_push_return",
            Box::new(|service, owner| {
                service.remote_safe_point(root, owner, Point::AfterPushReturn)
            }),
        ),
        (
            "verified",
            Box::new(|service, owner| {
                service.advance_synchronization_publication(
                    root,
                    owner,
                    state::PublicationPhase::Verified,
                    Some(candidate),
                )
            }),
        ),
        (
            "authority",
            Box::new(|service, owner| {
                service
                    .synchronization_publication_authority(root, owner)
                    .map(|observed| {
                        assert_eq!(observed, authority);
                        RemoteSafePointOutcome::Continue
                    })
            }),
        ),
        (
            "after_push_verification",
            Box::new(|service, owner| {
                service.remote_safe_point(root, owner, Point::AfterPushVerification)
            }),
        ),
        (
            "classify",
            Box::new(|service, owner| service.classify_synchronization(root, owner, authority)),
        ),
    ];
    for (label, transition) in &transitions {
        let before = (
            fixture.attempts(),
            fixture.legacy_push(),
            fixture.envelope(),
        );
        assert!(
            refused(transition(&fixture.service, &stale)),
            "{label}: superseded owner"
        );
        assert!(
            refused(transition(&foreign, &fixture.owner)),
            "{label}: foreign service"
        );
        assert!(
            refused(transition(&foreign, &stale)),
            "{label}: foreign service with a superseded token"
        );
        assert_eq!(
            (
                fixture.attempts(),
                fixture.legacy_push(),
                fixture.envelope()
            ),
            before,
            "{label}"
        );
        assert_eq!(
            transition(&fixture.service, &fixture.owner).unwrap(),
            RemoteSafePointOutcome::Continue,
            "{label}"
        );
    }
    assert_eq!(fixture.record().authority, Some(authority));
    let rows = fixture.attempts();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].4, "verified");
    assert_eq!(rows[0].5, Some(candidate.to_string()));

    for at in ["prepare", "returned", "verified"] {
        let (fixture, candidate, tip) = open_attempt_fixture(true);
        if at != "prepare" {
            fixture.prepare(candidate, Some(tip), false).unwrap();
            fixture.point(Point::BeforePush);
        }
        if at == "verified" {
            fixture
                .advance(state::PublicationPhase::Returned, None)
                .unwrap();
            fixture.point(Point::AfterPushReturn);
        }
        let rows = fixture.attempts();
        let legacy = fixture.legacy_push();
        // The request arrives from another service instance.
        RepositoryService::open_at(fixture.data.path())
            .unwrap()
            .cancel_remote_operation(fixture.root.path(), fixture.operation)
            .unwrap();
        let outcome = match at {
            "prepare" => fixture.prepare(candidate, Some(tip), false),
            "returned" => fixture.advance(state::PublicationPhase::Returned, None),
            _ => fixture.advance(state::PublicationPhase::Verified, Some(candidate)),
        };
        assert_eq!(outcome.unwrap(), RemoteSafePointOutcome::Cancelled, "{at}");
        assert_eq!(fixture.attempts(), rows, "{at}");
        assert_eq!(fixture.legacy_push(), legacy, "{at}");
        assert_eq!(fixture.record().phase, RemoteOperationPhase::Cancelled);
        // A cancelled operation is replayed, never handed a new owner.
        assert!(
            matches!(
                fixture
                    .service
                    .restart_remote_synchronization(
                        fixture.root.path(),
                        fixture.operation,
                        &fixture.target
                    )
                    .unwrap(),
                RemoteReservationOutcome::Replay(_)
            ),
            "{at}"
        );
        assert_eq!(fixture.attempts(), rows, "{at}");
    }
}

/// G: another service instance takes the operation over, or cancels it, while
/// the first holder stands at a merge-metadata retirement safe point. Before
/// the lease the fenced or stopped holder unlinks nothing and journals
/// nothing. Between unlinks it can neither journal `retired` nor observe the
/// stage. The new owner, or the same operation restarted after the
/// cancellation, completes that same retirement exactly once.
#[test]
fn merge_metadata_retirement_fences_a_superseding_service_and_cancellation() {
    type Takeover = Arc<std::sync::Mutex<Option<(RepositoryService, RemoteReservation)>>>;
    for variant in [
        "superseded_before_lease",
        "cancelled_before_lease",
        "superseded_between_unlinks",
    ] {
        let mut fixture = PassFixture::new("fixture.txt", b"local\n");
        let repository = fixture.repository();
        let gitdir = repository.path().to_owned();
        let root = fixture.root.path().to_owned();
        let local = fixture.local;
        let incoming = child(&repository, fixture.base, b"incoming\n");
        fixture.fetch(incoming, 1);
        fixture.append_window(1, incoming).unwrap();
        assert!(matches!(
            fixture.integrate(incoming),
            Err(SynchronizationError::ConflictPending { .. })
        ));
        let repaired = commit_external_repair(&repository, local, incoming, true);
        let members = merge_member_images(&gitdir);
        let before = local_binding_image(&root);
        let before_logs = reflog_image(&repository);
        fixture.restart();
        let takeover: Takeover = Arc::new(std::sync::Mutex::new(None));
        let observer = {
            let slot = takeover.clone();
            let data = fixture.data.path().to_owned();
            let root = root.clone();
            let operation = fixture.operation;
            let target = fixture.target.clone();
            let cancel = variant == "cancelled_before_lease";
            move || {
                let other = RepositoryService::open_at(&data).unwrap();
                if cancel {
                    other.cancel_remote_operation(&root, operation).unwrap();
                    return;
                }
                let RemoteReservationOutcome::Reserved(owner) = other
                    .restart_remote_synchronization(&root, operation, &target)
                    .unwrap()
                else {
                    panic!("superseding restart")
                };
                *slot.lock().unwrap() = Some((other, owner));
            }
        };
        if variant == "superseded_between_unlinks" {
            set_merge_metadata_unlinked_hook(root.clone(), observer);
        } else {
            set_merge_metadata_observed_hook(root.clone(), observer);
        }
        let mut evidence = fixture.record().sync_evidence;
        let stopped = fixture.reconcile(&mut evidence);
        if variant == "cancelled_before_lease" {
            assert!(matches!(stopped, Err(SynchronizationError::Interrupted)));
        } else {
            assert!(journal_stop(&stopped), "{variant}");
        }
        let after = merge_member_images(&gitdir);
        if variant == "superseded_between_unlinks" {
            // Under its lease the fenced holder may finish unlinking members
            // that were proven its own; it alters none and journals nothing.
            assert_eq!(after[0], None);
            for (image, original) in after.iter().zip(&members) {
                assert!(image.is_none() || image == original);
            }
            assert_eq!(fixture.merge_metadata_phases(), ["retire_intent"]);
        } else {
            assert_eq!(after, members, "{variant}");
            assert_eq!(fixture.merge_metadata_phases(), ["recorded"], "{variant}");
        }
        let step = fixture.step(1).unwrap();
        assert_eq!(
            step.phase,
            state::IntegrationStepPhase::ConflictPending,
            "{variant}"
        );
        assert_eq!(step.result_oid, None, "{variant}");
        assert_eq!(local_binding_image(&root), before, "{variant}");
        assert_eq!(reflog_image(&repository), before_logs, "{variant}");
        let taken = takeover.lock().unwrap().take();
        let (other, owner) = if variant == "cancelled_before_lease" {
            // The pending conflict made the cancellation a recoverable stop:
            // a later process restarts the same operation into ownership.
            assert!(taken.is_none());
            assert_eq!(
                cancellation_state(&fixture.db(), fixture.operation),
                ("interrupted".into(), 0)
            );
            let other = RepositoryService::open_at(fixture.data.path()).unwrap();
            let RemoteReservationOutcome::Reserved(owner) = other
                .restart_remote_synchronization(&root, fixture.operation, &fixture.target)
                .unwrap()
            else {
                panic!("restart after a recoverable stop")
            };
            (other, owner)
        } else {
            taken.expect("the observer took the operation over")
        };
        let mut evidence = fixture.record().sync_evidence;
        assert_eq!(
            reconcile_pending_candidate(
                &other,
                &root,
                "main",
                &SynchronizationTarget::Primary,
                &owner,
                &mut evidence,
            )
            .unwrap()
            .unwrap()
            .oid,
            repaired,
            "{variant}"
        );
        assert_eq!(
            merge_member_images(&gitdir),
            [None, None, None],
            "{variant}"
        );
        assert_eq!(fixture.merge_metadata_phases(), ["retired"], "{variant}");
        let step = fixture.step(1).unwrap();
        assert_eq!(
            step.phase,
            state::IntegrationStepPhase::Applied,
            "{variant}"
        );
        assert_eq!(step.result_oid, Some(repaired), "{variant}");
        assert_eq!(fixture.step_count(), 1, "{variant}");
        assert_eq!(local_binding_image(&root), before, "{variant}");
        assert_eq!(reflog_image(&repository), before_logs, "{variant}");
    }
}

/// G: discovery composes with a pending conflict without owning it. An
/// ordinary refresh and a rebuild neither clear the installed Git conflict nor
/// touch synchronization evidence, and the conflict stays directly inspectable
/// and readable while the index cannot commit.
#[test]
fn pending_conflict_survives_refresh_and_rebuild_and_stays_inspectable_without_the_index() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    let (root, data, service, operation, local, _) = resolution_fixture(&[(
        "docs/document.md",
        base,
        &base.replace("base", "local"),
        &base.replace("base", "incoming"),
    )]);
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let image = inspection_git_image(root.path());
    let evidence = durable_evidence(&db);
    let inspection = service
        .inspect_synchronization_recovery(root.path(), operation)
        .unwrap();
    let inspectable = |service: &RepositoryService| {
        let again = service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap();
        assert!(again.observation == inspection.observation);
        assert_eq!(again.paths.len(), inspection.paths.len());
        let sides = service
            .read_synchronization_conflict(&again.paths[0].token)
            .unwrap();
        assert!(sides.local.is_some() && sides.incoming.is_some());
        assert_eq!(inspection_git_image(root.path()), image);
        assert_eq!(durable_evidence(&db), evidence);
        let repository = git2::Repository::open(root.path()).unwrap();
        assert!(repository.index().unwrap().has_conflicts());
        assert_eq!(repository.state(), git2::RepositoryState::Merge);
        assert_eq!(repository.head().unwrap().target(), Some(local));
    };
    // An ordinary refresh and a rebuild both complete over the conflicted
    // worktree; each is read-only towards Git and the synchronization journal.
    assert!(matches!(
        service.refresh_repository(RefreshRepositoryRequest {
            root: root.path().into(),
            operation_id: OperationId::new(),
        }),
        Ok(RefreshOutcome::Refreshed { .. })
    ));
    inspectable(&service);
    let rebuilder = RepositoryService::open_at(data.path()).unwrap();
    rebuilder
        .rebuild_repository(crate::repository::RebuildRepositoryRequest {
            root: root.path().into(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    inspectable(&rebuilder);
    inspectable(&service);
    // Indexing unavailable: the discovery transaction cannot commit.
    let interrupted = RefreshRepositoryRequest {
        root: root.path().into(),
        operation_id: OperationId::new(),
    };
    *service.failure_point.lock().unwrap() = Some(FailurePoint::BeforeIndexTransactionCommit);
    assert!(matches!(
        service.refresh_repository(interrupted.clone()),
        Err(RepositoryError {
            kind: RepositoryErrorKind::InjectedFailure,
            ..
        })
    ));
    *service.failure_point.lock().unwrap() = None;
    inspectable(&service);
    inspectable(&rebuilder);
    let mut retry = request(root.path());
    retry.operation_id = operation;
    retry.restart = true;
    // The unfinished index operation fences a new owner, as it does for every
    // remote operation; the refusal itself changes nothing.
    assert!(matches!(
        rebuilder.synchronize_remote(retry.clone(), &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        }))
    ));
    inspectable(&service);
    // Resuming that same refresh restores the identical offline restart.
    assert!(matches!(
        service.refresh_repository(interrupted),
        Ok(RefreshOutcome::Refreshed { .. })
    ));
    inspectable(&service);
    assert!(matches!(
        rebuilder.synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
        Err(SynchronizationError::ConflictPending { operation_id, .. }) if operation_id == operation
    ));
    assert_eq!(inspection_git_image(root.path()), image);
    assert!(
        service
            .inspect_synchronization_recovery(root.path(), operation)
            .unwrap()
            .observation
            == inspection.observation
    );
}

/// G: the authority of a continuation publication attempt composes with
/// index-only discovery exactly like a legacy one. A failed discovery leaves
/// exact index-pending authority; an unrelated rebuild neither consumes that
/// handoff nor replays any mutation; replay then only refreshes, with no
/// credential prompt, transport, integration or push, and never scans again.
#[test]
fn continuation_authority_index_pending_replays_refresh_only_across_rebuild() {
    use RemoteOperationSafePoint as Point;
    let (fixture, candidate, tip) = open_attempt_fixture(true);
    fixture.prepare(candidate, Some(tip), false).unwrap();
    fixture.point(Point::BeforePush);
    fixture
        .advance(state::PublicationPhase::Returned, None)
        .unwrap();
    fixture.point(Point::AfterPushReturn);
    fixture
        .advance(state::PublicationPhase::Verified, Some(candidate))
        .unwrap();
    fixture.point(Point::AfterPushVerification);
    let authority = fixture.authority();
    assert_eq!(
        authority,
        state::SynchronizationAuthority::Published(candidate)
    );
    fixture
        .service
        .classify_synchronization(fixture.root.path(), &fixture.owner, authority)
        .unwrap();
    let root = fixture.root.path();
    let repository = fixture.repository();
    let attempts = fixture.attempts();
    let legacy = fixture.legacy_push();
    let image = inspection_git_image(root);
    let objects = inspection_odb_inventory(&repository);
    let handoff = |pending: i64| {
        let envelope = fixture.envelope();
        assert_eq!(envelope.0, "completed");
        assert_eq!(envelope.3, Some(candidate.to_string()));
        assert_eq!(envelope.4, pending);
        assert_eq!(fixture.attempts(), attempts);
        assert_eq!(fixture.legacy_push(), legacy);
        assert_eq!(inspection_git_image(root), image);
        assert_eq!(inspection_odb_inventory(&repository), objects);
    };
    handoff(1);
    let expected = SynchronizationOutcome::Published {
        target: SynchronizationTarget::Primary,
        oid: candidate,
    };
    let mut replay = request(root);
    replay.operation_id = fixture.operation;
    // A later process whose discovery transaction cannot commit.
    let service = RepositoryService::open_at(fixture.data.path()).unwrap();
    *service.failure_point.lock().unwrap() = Some(FailurePoint::BeforeIndexTransactionCommit);
    assert_eq!(
        service
            .synchronize_remote(replay.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        SynchronizationResult::IndexPending(IndexPending::new(expected.clone()))
    );
    *service.failure_point.lock().unwrap() = None;
    handoff(1);
    // An ordinary rebuild under its own operation is not this handoff: it is
    // refused until the pending refresh is resumed, and consumes nothing.
    let rebuild = || {
        service.rebuild_repository(crate::repository::RebuildRepositoryRequest {
            root: root.into(),
            operation_id: OperationId::new(),
        })
    };
    assert!(matches!(
        rebuild(),
        Err(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        })
    ));
    handoff(1);
    let mut restart = replay.clone();
    restart.restart = true;
    assert_eq!(
        service
            .synchronize_remote(restart, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        SynchronizationResult::Complete(expected.clone())
    );
    handoff(0);
    // Once handed off, a rebuild of the derived index replays no mutation and
    // leaves the recorded authority as it is.
    rebuild().unwrap();
    handoff(0);
    service.set_observation_hook_for_testing(|| panic!("completed handoff must not scan"));
    assert_eq!(
        service
            .synchronize_remote(replay, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
        SynchronizationResult::Complete(expected)
    );
    handoff(0);
}

/// phase and cancel flag of one operation's envelope.
fn cancellation_state(db: &rusqlite::Connection, operation: OperationId) -> (String, i64) {
    db.query_row(
        "SELECT phase,cancel_requested FROM remote_operation_records WHERE operation_ulid=?1",
        [operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap()
}

/// A cancellation honoured while the operation holds a pending conflict stops
/// that owner and preserves the recoverable transition: the operation is not
/// terminal, the request is consumed, and the same operation still restarts,
/// reacquires and resolves its conflict, or retires its own merge metadata
/// after external repair. Covered at the reconciliation entry boundary, the
/// under-lease conflict inspection boundary and the retirement boundary.
#[test]
fn cancellation_with_a_pending_conflict_is_a_recoverable_stop() {
    let base = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\ntitle: \"Document\"\n---\n\nbase\n";
    for boundary in [
        "reconciliation_entry",
        "under_lease_inspection",
        "restart_acknowledgement",
    ] {
        let (root, data, service, operation, local, incoming) = resolution_fixture(&[(
            "docs/document.md",
            base,
            &base.replace("base", "local"),
            &base.replace("base", "incoming"),
        )]);
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let RemoteReservationOutcome::Reserved(owner) = service
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("explicit restart")
        };
        let cancel = {
            let (data, root) = (data.path().to_owned(), root.path().to_owned());
            move || {
                RepositoryService::open_at(&data)
                    .unwrap()
                    .cancel_remote_operation(&root, operation)
                    .unwrap();
            }
        };
        if boundary == "under_lease_inspection" {
            set_local_reconciliation_prepared_hook(root.path().to_owned(), cancel);
        } else {
            cancel();
        }
        let image = inspection_git_image(root.path());
        let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let stopped = if boundary == "restart_acknowledgement" {
            // The cancelled owner never ran again; the next deliberate
            // restart through the public entry point honours the request.
            let mut retry = request(root.path());
            retry.operation_id = operation;
            retry.restart = true;
            RepositoryService::open_at(data.path())
                .unwrap()
                .synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt))
                .map(|_| None)
        } else {
            reconcile_pending_candidate(
                &service,
                root.path(),
                "main",
                &SynchronizationTarget::Primary,
                &owner,
                &mut evidence,
            )
        };
        assert!(
            matches!(stopped, Err(SynchronizationError::Interrupted)),
            "{boundary}"
        );
        assert_eq!(
            cancellation_state(&db, operation),
            ("interrupted".into(), 0),
            "{boundary}"
        );
        assert_eq!(
            db.query_row("SELECT phase FROM remote_integration_steps", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
            "conflict_pending",
            "{boundary}"
        );
        assert_eq!(inspection_git_image(root.path()), image, "{boundary}");
        // The stopped owner is fenced, but the operation is not terminal: a
        // later process restarts the same ID into the same offline conflict.
        let later = RepositoryService::open_at(data.path()).unwrap();
        let mut retry = request(root.path());
        retry.operation_id = operation;
        retry.restart = true;
        assert!(
            matches!(
                later.synchronize_remote(retry, &mut SessionCredentials::new(NoPrompt)),
                Err(SynchronizationError::ConflictPending { operation_id, .. })
                    if operation_id == operation
            ),
            "{boundary}"
        );
        assert_eq!(inspection_git_image(root.path()), image, "{boundary}");
        // And that same operation reacquires and resolves it.
        let resolved = base.replace("base", "resolved");
        let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
            resolve_fixture(root.path(), &later, operation, &[resolved.as_str()])
        else {
            panic!("{boundary}: resolution after a cancelled owner")
        };
        let repository = git2::Repository::open(root.path()).unwrap();
        assert_eq!(repository.head().unwrap().target(), Some(commit_oid));
        assert_eq!(
            repository
                .find_commit(commit_oid)
                .unwrap()
                .parent_ids()
                .collect::<Vec<_>>(),
            [local, incoming],
            "{boundary}"
        );
        assert!(!repository.index().unwrap().has_conflicts());
        assert_eq!(repository.state(), git2::RepositoryState::Clean);
    }

    // The retirement boundary: the conflict was repaired externally and the
    // cancellation lands after validation, before the lease.
    let mut fixture = PassFixture::new("fixture.txt", b"local\n");
    let repository = fixture.repository();
    let gitdir = repository.path().to_owned();
    let root = fixture.root.path().to_owned();
    let incoming = child(&repository, fixture.base, b"incoming\n");
    fixture.fetch(incoming, 1);
    fixture.append_window(1, incoming).unwrap();
    assert!(matches!(
        fixture.integrate(incoming),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let repaired = commit_external_repair(&repository, fixture.local, incoming, true);
    let members = merge_member_images(&gitdir);
    let before = local_binding_image(&root);
    let before_logs = reflog_image(&repository);
    fixture.restart();
    {
        let (data, root, operation) = (
            fixture.data.path().to_owned(),
            root.clone(),
            fixture.operation,
        );
        set_merge_metadata_observed_hook(root.clone(), move || {
            RepositoryService::open_at(&data)
                .unwrap()
                .cancel_remote_operation(&root, operation)
                .unwrap();
        });
    }
    let mut evidence = fixture.record().sync_evidence;
    assert!(matches!(
        fixture.reconcile(&mut evidence),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(
        cancellation_state(&fixture.db(), fixture.operation),
        ("interrupted".into(), 0)
    );
    assert_eq!(merge_member_images(&gitdir), members);
    assert_eq!(fixture.merge_metadata_phases(), ["recorded"]);
    assert_eq!(
        fixture.step(1).unwrap().phase,
        state::IntegrationStepPhase::ConflictPending
    );
    assert_eq!(local_binding_image(&root), before);
    // A later process restarts the same operation and completes retirement.
    fixture.restart_in_new_process();
    let mut evidence = fixture.record().sync_evidence;
    assert_eq!(
        fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
        repaired
    );
    assert_eq!(merge_member_images(&gitdir), [None, None, None]);
    assert_eq!(fixture.merge_metadata_phases(), ["retired"]);
    let step = fixture.step(1).unwrap();
    assert_eq!(step.phase, state::IntegrationStepPhase::Applied);
    assert_eq!(step.result_oid, Some(repaired));
    assert_eq!(local_binding_image(&root), before);
    assert_eq!(reflog_image(&repository), before_logs);
}

/// A cancellation requested while the first conflicted merge is being
/// installed (after its boundary, before the conflict is released) does not
/// survive the release as a latent request: the conflict stays reacquirable
/// by its own operation and a later restart still converges.
#[test]
fn cancellation_requested_during_the_first_conflicted_merge_keeps_it_reacquirable() {
    let mut fixture = PassFixture::new("fixture.txt", b"local\n");
    let repository = fixture.repository();
    let incoming = child(&repository, fixture.base, b"incoming\n");
    fixture.fetch(incoming, 1);
    fixture.append_window(1, incoming).unwrap();
    let db = fixture.db();
    // The request becomes durable together with the effect intent, which is
    // written after the last boundary that could have honoured it.
    db.execute_batch(
        "CREATE TRIGGER injected_stop AFTER UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applying' BEGIN UPDATE remote_operation_records SET cancel_requested=1; END;",
    )
    .unwrap();
    assert!(matches!(
        fixture.integrate(incoming),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    db.execute_batch(DROP_INJECTED_STOP).unwrap();
    assert_eq!(
        cancellation_state(&db, fixture.operation),
        ("interrupted".into(), 0)
    );
    let step = fixture.step(1).unwrap();
    assert_eq!(step.phase, state::IntegrationStepPhase::ConflictPending);
    assert!(repository.index().unwrap().has_conflicts());
    assert_eq!(fixture.merge_metadata_phases(), ["recorded"]);
    // Explicit reacquisition by the same operation, as resolution performs it.
    assert!(matches!(
        fixture
            .service
            .reacquire_synchronization_conflict_in_window(
                fixture.root.path(),
                fixture.operation,
                &fixture.target,
                1,
                0,
                step.conflict_digest.unwrap(),
            )
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    // External repair followed by a deliberate restart converges as well.
    let repaired = commit_external_repair(&repository, fixture.local, incoming, true);
    fixture.restart_in_new_process();
    let mut evidence = fixture.record().sync_evidence;
    assert_eq!(
        fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
        repaired
    );
    assert_eq!(fixture.merge_metadata_phases(), ["retired"]);
    assert_eq!(
        fixture.step(1).unwrap().phase,
        state::IntegrationStepPhase::Applied
    );
}

/// Cycle 05 cancellation is unchanged for an operation without a pending
/// conflict or owned resolution: it is terminal, with or without a completed
/// clean stage, and the same ID only ever replays the interruption.
#[test]
fn cancellation_without_a_pending_conflict_stays_terminal() {
    for applied in [false, true] {
        let fixture = PassFixture::new("local.txt", b"local\n");
        let repository = fixture.repository();
        if applied {
            let incoming = child_file(&repository, fixture.base, "remote.txt", b"remote\n");
            fixture.fetch(incoming, 1);
            fixture.append_window(1, incoming).unwrap();
            fixture.integrate(incoming).unwrap();
        }
        let image = inspection_git_image(fixture.root.path());
        RepositoryService::open_at(fixture.data.path())
            .unwrap()
            .cancel_remote_operation(fixture.root.path(), fixture.operation)
            .unwrap();
        assert_eq!(
            fixture
                .service
                .check_synchronization_requests(fixture.root.path(), &fixture.owner)
                .unwrap(),
            RemoteSafePointOutcome::Cancelled,
            "{applied}"
        );
        assert_eq!(
            fixture.record().phase,
            RemoteOperationPhase::Cancelled,
            "{applied}"
        );
        assert!(matches!(
            fixture
                .service
                .restart_remote_synchronization(
                    fixture.root.path(),
                    fixture.operation,
                    &fixture.target
                )
                .unwrap(),
            RemoteReservationOutcome::Replay(_)
        ));
        let mut retry = request(fixture.root.path());
        retry.operation_id = fixture.operation;
        retry.restart = true;
        for _ in 0..2 {
            assert!(matches!(
                RepositoryService::open_at(fixture.data.path())
                    .unwrap()
                    .synchronize_remote(retry.clone(), &mut SessionCredentials::new(NoPrompt)),
                Err(SynchronizationError::Interrupted)
            ));
        }
        assert_eq!(
            fixture.record().phase,
            RemoteOperationPhase::Cancelled,
            "{applied}"
        );
        assert_eq!(inspection_git_image(fixture.root.path()), image);
    }
}

/// Whether the common Git lease of `root` is held right now.
fn lease_refusal(root: &Path) -> Option<RepositoryErrorKind> {
    let repository = git2::Repository::open(root).unwrap();
    crate::repository::repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)
        .err()
        .map(|error| error.kind)
}

/// G: validation and isolated preparation run outside the short common Git
/// lease, and owned merge metadata is only ever unlinked under it. An
/// observer at each safe point sees the lease free before conflict
/// inspection, before retirement and before the generic candidate
/// observation, and held after every unlink.
#[test]
fn local_reconciliation_validates_outside_the_lease_and_retires_under_it() {
    type Seen = Arc<std::sync::Mutex<Vec<(&'static str, Option<RepositoryErrorKind>)>>>;
    let observer = |seen: &Seen, label: &'static str, root: &Path| {
        let (seen, root) = (seen.clone(), root.to_owned());
        move || {
            let refusal = lease_refusal(&root);
            seen.lock().unwrap().push((label, refusal));
        }
    };
    // Retirement after external repair.
    let seen: Seen = Arc::default();
    let mut fixture = PassFixture::new("fixture.txt", b"local\n");
    let repository = fixture.repository();
    let root = fixture.root.path().to_owned();
    let incoming = child(&repository, fixture.base, b"incoming\n");
    fixture.fetch(incoming, 1);
    fixture.append_window(1, incoming).unwrap();
    assert!(matches!(
        fixture.integrate(incoming),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let repaired = commit_external_repair(&repository, fixture.local, incoming, true);
    set_merge_metadata_observed_hook(root.clone(), observer(&seen, "validated", &root));
    for _ in 0..3 {
        set_merge_metadata_unlinked_hook(root.clone(), observer(&seen, "unlinked", &root));
    }
    fixture.restart();
    let mut evidence = fixture.record().sync_evidence;
    assert_eq!(
        fixture.reconcile(&mut evidence).unwrap().unwrap().oid,
        repaired
    );
    assert_eq!(
        *seen.lock().unwrap(),
        [
            ("validated", None),
            ("unlinked", Some(RepositoryErrorKind::RepositoryBusy)),
            ("unlinked", Some(RepositoryErrorKind::RepositoryBusy)),
            ("unlinked", Some(RepositoryErrorKind::RepositoryBusy)),
        ]
    );
    assert_eq!(lease_refusal(&root), None);

    // Isolated preparation of a pending conflict's inspection.
    let seen: Seen = Arc::default();
    let (root, _data, service, operation, _, _) =
        resolution_fixture(&[("src/foreign.rs", "base\n", "local\n", "incoming\n")]);
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan);
    let RemoteReservationOutcome::Reserved(owner) = service
        .restart_remote_synchronization(root.path(), operation, &target)
        .unwrap()
    else {
        panic!("explicit restart")
    };
    set_local_reconciliation_prepared_hook(
        root.path().to_owned(),
        observer(&seen, "prepared", root.path()),
    );
    let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
        Ok(state::read_operation(tx, id, operation)?
            .unwrap()
            .sync_evidence)
    })
    .unwrap();
    assert!(matches!(
        reconcile_pending_candidate(
            &service,
            root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &owner,
            &mut evidence,
        ),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    assert_eq!(*seen.lock().unwrap(), [("prepared", None)]);

    // Observation of a recorded clean candidate.
    let seen: Seen = Arc::default();
    let mut fixture = PassFixture::new("local.txt", b"local\n");
    let repository = fixture.repository();
    let root = fixture.root.path().to_owned();
    let incoming = child_file(&repository, fixture.base, "remote.txt", b"remote\n");
    fixture.fetch(incoming, 1);
    fixture.append_window(1, incoming).unwrap();
    let db = fixture.db();
    stop_before(
        &db,
        "UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied'",
    );
    assert!(journal_stop(&fixture.integrate(incoming)));
    db.execute_batch(DROP_INJECTED_STOP).unwrap();
    set_candidate_reconciliation_observed_hook(root.clone(), observer(&seen, "candidate", &root));
    fixture.restart();
    let mut evidence = fixture.record().sync_evidence;
    assert!(fixture.reconcile(&mut evidence).unwrap().is_some());
    assert_eq!(*seen.lock().unwrap(), [("candidate", None)]);
}

/// G: a takeover by another service and a cancellation at the remaining local
/// safe points: the under-lease conflict inspection, the observation of a
/// recorded clean candidate, and the boundary between the context and primary
/// stages. The fenced or stopped holder records and moves nothing further.
#[test]
fn local_safe_points_fence_takeover_and_cancellation() {
    let superseded = |result: &Result<Option<ReconciledCandidate>, SynchronizationError>| {
        matches!(
            result,
            Err(SynchronizationError::Repository(RepositoryError {
                kind: RepositoryErrorKind::RecoveryRequired,
                ..
            }))
        )
    };
    for cancel in [false, true] {
        let intervene =
            |data: &Path, root: &Path, operation: OperationId, target: &RemoteOperationTarget| {
                let (data, root, target) = (data.to_owned(), root.to_owned(), target.clone());
                move || {
                    let other = RepositoryService::open_at(&data).unwrap();
                    if cancel {
                        other.cancel_remote_operation(&root, operation).unwrap();
                    } else {
                        assert!(matches!(
                            other
                                .restart_remote_synchronization(&root, operation, &target)
                                .unwrap(),
                            RemoteReservationOutcome::Reserved(_)
                        ));
                    }
                }
            };

        // Under-lease inspection of a pending conflict.
        let (root, data, service, operation, _, _) =
            resolution_fixture(&[("src/foreign.rs", "base\n", "local\n", "incoming\n")]);
        let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        db.execute(
            "UPDATE remote_integration_steps SET phase='applying',conflict_digest=NULL",
            [],
        )
        .unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_primary_synchronization(&plan);
        let RemoteReservationOutcome::Reserved(owner) = service
            .restart_remote_synchronization(root.path(), operation, &target)
            .unwrap()
        else {
            panic!("explicit restart")
        };
        set_local_reconciliation_prepared_hook(
            root.path().to_owned(),
            intervene(data.path(), root.path(), operation, &target),
        );
        let image = inspection_git_image(root.path());
        let mut evidence = state::with_transaction(&service, root.path(), |tx, id| {
            Ok(state::read_operation(tx, id, operation)?
                .unwrap()
                .sync_evidence)
        })
        .unwrap();
        let result = reconcile_pending_candidate(
            &service,
            root.path(),
            "main",
            &SynchronizationTarget::Primary,
            &owner,
            &mut evidence,
        );
        if cancel {
            assert!(matches!(result, Err(SynchronizationError::Interrupted)));
        } else {
            assert!(superseded(&result));
        }
        // The installed conflict was not recorded by the fenced holder.
        assert_eq!(
            db.query_row(
                "SELECT phase,conflict_digest FROM remote_integration_steps",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<Vec<u8>>>(1)?))
            )
            .unwrap(),
            ("applying".into(), None),
            "{cancel}"
        );
        assert_eq!(inspection_git_image(root.path()), image, "{cancel}");

        // Observation of a recorded clean candidate whose ref already moved.
        let mut fixture = PassFixture::new("local.txt", b"local\n");
        let repository = fixture.repository();
        let incoming = child_file(&repository, fixture.base, "remote.txt", b"remote\n");
        fixture.fetch(incoming, 1);
        fixture.append_window(1, incoming).unwrap();
        let db = fixture.db();
        stop_before(
            &db,
            "UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied'",
        );
        assert!(journal_stop(&fixture.integrate(incoming)));
        db.execute_batch(DROP_INJECTED_STOP).unwrap();
        let candidate = repository.head().unwrap().target().unwrap();
        fixture.restart();
        set_candidate_reconciliation_observed_hook(
            fixture.root.path().to_owned(),
            intervene(
                fixture.data.path(),
                fixture.root.path(),
                fixture.operation,
                &fixture.target,
            ),
        );
        let image = inspection_git_image(fixture.root.path());
        let mut evidence = fixture.record().sync_evidence;
        let result = fixture.reconcile(&mut evidence);
        if cancel {
            assert!(matches!(result, Err(SynchronizationError::Interrupted)));
            // No conflict or owned resolution is pending: terminal as before.
            assert_eq!(fixture.record().phase, RemoteOperationPhase::Cancelled);
        } else {
            assert!(superseded(&result));
        }
        let step = fixture.step(1).unwrap();
        assert_eq!(
            step.phase,
            state::IntegrationStepPhase::Applying,
            "{cancel}"
        );
        assert_eq!(step.candidate_oid, Some(candidate), "{cancel}");
        assert_eq!(step.result_oid, None, "{cancel}");
        assert_eq!(inspection_git_image(fixture.root.path()), image, "{cancel}");

        // Between the context and primary stages. A trigger cannot run a
        // second service, so the takeover is modelled by its durable effect:
        // the owner epoch advances in the transaction that observes the
        // context stage, exactly where an explicit restart could land.
        let pass = ContextPass::new();
        let db = pass.db();
        db.execute_batch(&format!(
            "CREATE TRIGGER injected_stop AFTER UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied' AND NEW.ordinal=0 BEGIN UPDATE remote_operation_records SET {}; END;",
            if cancel {
                "cancel_requested=1"
            } else {
                "owner_epoch=owner_epoch+1"
            }
        ))
        .unwrap();
        let original_log = pass.head_log();
        let result = pass.integrate();
        db.execute_batch(DROP_INJECTED_STOP).unwrap();
        if cancel {
            assert!(matches!(result, Err(SynchronizationError::Interrupted)));
        } else {
            assert!(journal_stop(&result));
        }
        // The context merge stands; the primary stage never got an intent.
        let context_merge = pass.head();
        assert_eq!(
            pass.parents(context_merge),
            [pass.local, pass.context_incoming],
            "{cancel}"
        );
        let context_stage = pass.stage(0).unwrap();
        assert_eq!(context_stage.phase, state::IntegrationStepPhase::Applied);
        assert_eq!(context_stage.result_oid, Some(context_merge));
        assert!(pass.stage(1).is_none(), "{cancel}");
        assert_eq!(
            ref_log_lines(&pass.head_log()),
            ref_log_lines(&original_log) + 1,
            "{cancel}"
        );
        if !cancel {
            // The current owner epoch resumes the primary stage only.
            assert!(matches!(
                pass.service
                    .restart_remote_synchronization(
                        pass.root.path(),
                        pass.operation,
                        &pass.operation_target
                    )
                    .unwrap(),
                RemoteReservationOutcome::Reserved(_)
            ));
        }
    }
}

// Real SIGKILL tests, deliberately separate from synthetic partial-log and
// transaction-fault tests above. The parent owns every disposable fixture and
// the immutable request; no production helper process or journal format exists.
#[cfg(target_os = "linux")]
pub(super) fn process_death_boundary(root: &Path, point: &str) {
    process_death::boundary(root, point);
}

// Native process-death evidence remains Linux-only until exercised elsewhere.
#[cfg(not(target_os = "linux"))]
pub(super) fn process_death_boundary(_root: &Path, _point: &str) {}

#[cfg(target_os = "linux")]
mod process_death {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    const CHILD_TEST: &str =
        "repository::remote::sync::tests::process_death::resolution_process_death_child";
    const TIMEOUT: Duration = Duration::from_secs(30);
    const CHILD_TIMEOUT_EXIT: i32 = 86;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct FixtureRequest {
        operation: String,
        attempt: String,
        ordinal: u8,
        fingerprint: [u8; 32],
        configuration: [u8; 32],
        head: String,
        path_ordinal: u32,
        sides: [Option<String>; 3],
        modes: [Option<u32>; 3],
        result: Vec<u8>,
    }

    impl FixtureRequest {
        fn from_request(request: &ResolveSynchronizationRequest) -> Self {
            assert!(request.identity.is_none());
            assert_eq!(request.resolutions.len(), 1);
            let (token, result) = &request.resolutions[0];
            assert!(token.path == b"docs/document.md");
            Self {
                operation: request.synchronization_id.to_string(),
                attempt: request.attempt_id.to_string(),
                ordinal: request.observation.ordinal,
                fingerprint: request.observation.fingerprint,
                configuration: request.observation.configuration,
                head: request.observation.head.to_string(),
                path_ordinal: token.ordinal,
                sides: [token.base, token.local, token.incoming]
                    .map(|oid| oid.map(|oid| oid.to_string())),
                modes: [token.base_mode, token.local_mode, token.incoming_mode],
                result: result.bytes().to_vec(),
            }
        }

        fn into_request(self, root: PathBuf) -> ResolveSynchronizationRequest {
            let operation = OperationId::parse(&self.operation).expect("fixture operation");
            let observation = merge::ConflictObservation {
                window_number: 0,
                operation_id: operation,
                ordinal: self.ordinal,
                fingerprint: self.fingerprint,
                head: self.head.parse().expect("fixture head"),
                configuration: self.configuration,
                root: root.clone(),
            };
            let [base, local, incoming] = self
                .sides
                .map(|oid| oid.map(|oid| oid.parse().expect("fixture side")));
            let token = merge::ConflictPathToken {
                observation: observation.clone(),
                ordinal: self.path_ordinal,
                path: b"docs/document.md".to_vec(),
                base,
                local,
                incoming,
                base_mode: self.modes[0],
                local_mode: self.modes[1],
                incoming_mode: self.modes[2],
            };
            ResolveSynchronizationRequest::new(
                root,
                operation,
                OperationId::parse(&self.attempt).expect("fixture attempt"),
                observation,
                vec![(token, RedactedConflictBytes::from_bytes(self.result))],
                None,
            )
        }
    }

    pub(super) fn boundary(root: &Path, point: &str) {
        if std::env::var("MANYHANDS_RESOLUTION_DEATH_POINT")
            .ok()
            .as_deref()
            != Some(point)
            || std::env::var_os("MANYHANDS_RESOLUTION_DEATH_ROOT")
                .is_none_or(|expected| Path::new(&expected) != root)
        {
            return;
        }
        let ipc = PathBuf::from(std::env::var_os("MANYHANDS_RESOLUTION_DEATH_IPC").unwrap());
        let child_timeout = std::env::var("MANYHANDS_RESOLUTION_DEATH_TIMEOUT_MS")
            .map(|value| Duration::from_millis(value.parse().expect("child timeout milliseconds")))
            .unwrap_or(TIMEOUT);
        let deadline = Instant::now() + child_timeout;
        fs::write(ipc.join("ready"), b"ready").expect("death child readiness");
        // Neither unwind nor Drop is the crash mechanism. The parent kills and
        // reaps this process while the real backend/lease handles are still live.
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        // A delayed parent must never let this hook unwind through the live
        // transaction/lease guards. This distinct failure exit is not SIGKILL
        // and therefore cannot count as a successful boundary-death proof.
        unsafe { libc::_exit(CHILD_TIMEOUT_EXIT) }
    }

    struct DropCanary(PathBuf);
    impl Drop for DropCanary {
        fn drop(&mut self) {
            fs::write(&self.0, b"drop ran").expect("death child drop canary");
        }
    }

    #[test]
    fn resolution_process_death_child() {
        let Some(ipc) = std::env::var_os("MANYHANDS_RESOLUTION_DEATH_IPC") else {
            return;
        };
        let ipc = PathBuf::from(ipc);
        let root = PathBuf::from(std::env::var_os("MANYHANDS_RESOLUTION_DEATH_ROOT").unwrap());
        let data = PathBuf::from(std::env::var_os("MANYHANDS_RESOLUTION_DEATH_DATA").unwrap());
        let dto: FixtureRequest = serde_yaml::from_slice(&fs::read(ipc.join("request")).unwrap())
            .expect("fixture request decoding");
        let request = dto.into_request(root.clone());
        let service = RepositoryService::open_at(&data).expect("death child service");
        if std::env::var_os("MANYHANDS_RESOLUTION_DEATH_POINT").is_some() {
            let _drop_canary = DropCanary(ipc.join("drop-ran"));
            for (point, setter) in [
                (
                    "sentinel-published",
                    set_resolution_index_lock_hook as fn(PathBuf, fn()),
                ),
                (
                    "scratch-prepared",
                    set_resolution_index_scratch_hook as fn(PathBuf, fn()),
                ),
                (
                    "output-recorded",
                    set_resolution_index_persist_hook as fn(PathBuf, fn()),
                ),
                (
                    "index-installed",
                    set_resolution_index_install_hook as fn(PathBuf, fn()),
                ),
                (
                    "release-intent",
                    set_resolution_index_retire_hook as fn(PathBuf, fn()),
                ),
            ] {
                if std::env::var("MANYHANDS_RESOLUTION_DEATH_POINT").unwrap() == point {
                    setter(root.clone(), selected_boundary);
                }
            }
            if std::env::var("MANYHANDS_RESOLUTION_DEATH_POINT").unwrap() == "canonical-installed" {
                service.set_owned_path_hook_for_root_for_testing(
                    root,
                    "docs/document.md".into(),
                    OwnedPathBoundary::TempVerified,
                    selected_boundary,
                );
            }
            let _ = service.resolve_synchronization(request);
            panic!("death child missed selected boundary");
        }
        let result = service.resolve_synchronization(request);
        match std::env::var("MANYHANDS_RESOLUTION_DEATH_REFUSED")
            .ok()
            .as_deref()
        {
            Some("required") => {
                assert!(
                    matches!(result, Err(SynchronizationError::RecoveryRequired)),
                    "unproved recovery was not refused"
                );
                assert!(result.err().unwrap().to_string() == "synchronization recovery required");
            }
            Some("external") => {
                assert!(
                    matches!(result, Err(SynchronizationError::ExternalChange)),
                    "foreign effect was not refused"
                );
                assert!(result.err().unwrap().to_string() == "external change");
            }
            None => {
                assert!(
                    matches!(
                        result,
                        Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete { .. })
                    ),
                    "death recovery did not complete"
                );
            }
            _ => panic!("invalid recovery expectation"),
        }
    }

    fn selected_boundary() {
        let root = PathBuf::from(std::env::var_os("MANYHANDS_RESOLUTION_DEATH_ROOT").unwrap());
        boundary(
            &root,
            &std::env::var("MANYHANDS_RESOLUTION_DEATH_POINT").unwrap(),
        );
    }

    struct ReapedChild(Child);
    impl Drop for ReapedChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn spawn(
        root: &Path,
        data: &Path,
        ipc: &Path,
        point: Option<&str>,
        refused: Option<&str>,
        child_timeout: Option<Duration>,
    ) -> ReapedChild {
        let mut command = Command::new(std::env::current_exe().expect("death test executable"));
        command
            .args(["--exact", CHILD_TEST, "--test-threads=1"])
            .env("MANYHANDS_RESOLUTION_DEATH_ROOT", root)
            .env("MANYHANDS_RESOLUTION_DEATH_DATA", data)
            .env("MANYHANDS_RESOLUTION_DEATH_IPC", ipc)
            .env_remove("MANYHANDS_RESOLUTION_DEATH_POINT")
            .env_remove("MANYHANDS_RESOLUTION_DEATH_REFUSED")
            .env_remove("MANYHANDS_RESOLUTION_DEATH_TIMEOUT_MS")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(point) = point {
            command.env("MANYHANDS_RESOLUTION_DEATH_POINT", point);
        }
        if let Some(refused) = refused {
            command.env("MANYHANDS_RESOLUTION_DEATH_REFUSED", refused);
        }
        if let Some(timeout) = child_timeout {
            command.env(
                "MANYHANDS_RESOLUTION_DEATH_TIMEOUT_MS",
                timeout.as_millis().to_string(),
            );
        }
        ReapedChild(command.spawn().expect("death child spawn"))
    }

    fn kill_at(root: &Path, data: &Path, ipc: &Path, point: &str) {
        let mut child = spawn(root, data, ipc, Some(point), None, None);
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if ipc.join("ready").exists() {
                break;
            }
            assert!(
                child.0.try_wait().expect("death child status").is_none(),
                "death child exited before readiness at {point}"
            );
            assert!(
                Instant::now() < deadline,
                "death child readiness timed out at {point}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let repository = git2::Repository::open(root).unwrap();
        assert!(
            matches!(repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot), Err(error) if error.kind == RepositoryErrorKind::RepositoryBusy),
            "cooperating writer lease was not fenced at {point}"
        );
        child.0.kill().expect("death child kill");
        let status = child.0.wait().expect("death child reap");
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        assert!(
            !ipc.join("drop-ran").exists(),
            "death child ran a destructor"
        );
        drop(
            repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)
                .expect("death did not release OS lease"),
        );
    }

    fn recover(root: &Path, data: &Path, ipc: &Path, refused: Option<&str>) {
        let mut child = spawn(root, data, ipc, None, refused, None);
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = child.0.try_wait().expect("recovery child status") {
                assert!(status.success(), "recovery child failed");
                break;
            }
            assert!(Instant::now() < deadline, "recovery child timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    type FileImage = (u64, u64, Vec<u8>);

    fn image(path: &Path) -> FileImage {
        let metadata = fs::metadata(path).expect("fixture image metadata");
        (
            metadata.dev(),
            metadata.ino(),
            fs::read(path).expect("fixture image bytes"),
        )
    }

    fn optional_image(path: &Path) -> Option<FileImage> {
        match fs::symlink_metadata(path) {
            Ok(_) => Some(image(path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => panic!("refusal snapshot metadata unavailable"),
        }
    }

    // Deliberately no Debug: failed comparisons must not dump fixture bodies or
    // signer/history bytes. Presence plus identity and bytes cover all remnants.
    #[derive(PartialEq, Eq)]
    struct RefusalSnapshot {
        canonical: FileImage,
        index: FileImage,
        head: Option<git2::Oid>,
        logs: [FileImage; 2],
        metadata: [Option<FileImage>; 3],
        sentinel: Option<FileImage>,
        anchor: FileImage,
        backend_locks: [Option<FileImage>; 5],
    }

    fn refusal_snapshot(
        repository: &git2::Repository,
        root: &Path,
        attempt: OperationId,
    ) -> RefusalSnapshot {
        let gitdir = repository.path();
        RefusalSnapshot {
            canonical: image(&root.join("docs/document.md")),
            index: image(&gitdir.join("index")),
            head: repository.head().unwrap().target(),
            logs: ["logs/refs/heads/main", "logs/HEAD"].map(|role| image(&gitdir.join(role))),
            metadata: RESOLUTION_MERGE_MEMBERS.map(|member| optional_image(&gitdir.join(member))),
            sentinel: optional_image(&gitdir.join("index.lock")),
            anchor: image(&gitdir.join(format!(".manyhands-resolution-{attempt}/sentinel"))),
            backend_locks: [
                "HEAD.lock",
                "logs/HEAD.lock",
                "packed-refs.lock",
                "refs/heads/main.lock",
                "logs/refs/heads/main.lock",
            ]
            .map(|role| optional_image(&gitdir.join(role))),
        }
    }

    #[test]
    fn resolution_process_death_timeout_never_unwinds_or_counts_as_boundary_kill() {
        let (root, data, service, request) = protocol_resolution_fixture();
        let ipc = tempfile::tempdir().unwrap();
        fs::write(
            ipc.path().join("request"),
            serde_yaml::to_string(&FixtureRequest::from_request(&request)).unwrap(),
        )
        .unwrap();
        drop(service);
        let mut child = spawn(
            root.path(),
            data.path(),
            ipc.path(),
            Some("ref-locked"),
            None,
            Some(Duration::from_millis(20)),
        );
        // Deliberately delay the parent's kill until the shortened child timeout
        // expires. Polling for exit makes the adverse schedule deterministic.
        let deadline = Instant::now() + TIMEOUT;
        let status = loop {
            if let Some(status) = child.0.try_wait().expect("timeout child status") {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "shortened child timeout did not exit"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(
            ipc.path().join("ready").exists(),
            "timeout child did not reach boundary"
        );
        assert!(
            !ipc.path().join("drop-ran").exists(),
            "child timeout ran a destructor"
        );
        assert_eq!(status.code(), Some(CHILD_TIMEOUT_EXIT));
        // An eventual kill/wait must retain the timeout failure status, rather
        // than qualify this fixture as a successful SIGKILL boundary proof.
        let _ = child.0.kill();
        let reaped = child.0.wait().expect("timeout child reap");
        use std::os::unix::process::ExitStatusExt;
        assert_ne!(reaped.signal(), Some(libc::SIGKILL));
        assert_eq!(reaped.code(), Some(CHILD_TIMEOUT_EXIT));
        let repository = git2::Repository::open(root.path()).unwrap();
        assert!(
            repository.path().join("refs/heads/main.lock").exists(),
            "timeout dropped the backend transaction"
        );
    }

    fn prove(point: &str) {
        let (root, data, service, mut request) = protocol_resolution_fixture();
        const BODY: &[u8] = b"PRIVATE-DEATH-BODY-CANARY";
        const SIGNER: &str = "PRIVATE-DEATH-SIGNER-CANARY";
        const EMAIL: &str = "PRIVATE-DEATH-EMAIL-CANARY@example.invalid";
        const HISTORY: &[u8] = b"PRIVATE-DEATH-HISTORY-CANARY\xff opaque tail";
        let mut result = request.resolutions[0].1.bytes().to_vec();
        result.extend_from_slice(b"\nPRIVATE-DEATH-BODY-CANARY\n  exact caller spacing\n");
        request.resolutions[0].1 = RedactedConflictBytes::from_bytes(result);
        let ipc = tempfile::tempdir().unwrap();
        // Test IPC alone may contain fixture-only bodies. It is outside the
        // repository and registry, never part of production evidence or logs.
        fs::write(
            ipc.path().join("request"),
            serde_yaml::to_string(&FixtureRequest::from_request(&request)).unwrap(),
        )
        .unwrap();
        drop(service);
        let repository = git2::Repository::open(root.path()).unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", SIGNER).unwrap();
        config.set_str("user.email", EMAIL).unwrap();
        let gitdir = repository.path();
        let old = repository.head().unwrap().target().unwrap();
        let baseline_index = image(&gitdir.join("index"));
        let logs = [
            gitdir.join("logs/refs/heads/main"),
            gitdir.join("logs/HEAD"),
        ];
        for log in &logs {
            fs::write(log, HISTORY).unwrap();
        }
        let baseline_logs = logs.each_ref().map(|path| fs::read(path).unwrap());
        kill_at(root.path(), data.path(), ipc.path(), point);
        let canonical = root.path().join("docs/document.md");
        let installed_path = image(&canonical);
        let installed_index = image(&gitdir.join("index"));
        let connection = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
        let candidate: Option<String> = connection
            .query_row(
                "SELECT candidate_oid FROM remote_resolution_attempts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let killed_logs = logs.each_ref().map(|path| fs::read(path).unwrap());
        let sentinel = gitdir.join("index.lock");
        let released = matches!(point, "sentinel-unlinked" | "sentinel-release-barrier");
        assert_eq!(sentinel.exists(), !released);
        if !released {
            let anchor = gitdir.join(format!(
                ".manyhands-resolution-{}/sentinel",
                request.attempt_id
            ));
            assert!(
                image(&sentinel) == image(&anchor),
                "sentinel lost anchored identity"
            );
        }
        let committed = point == "ref-committed"
            || point.starts_with("metadata-")
            || point.starts_with("release-")
            || released;
        if let Some(member) = point
            .strip_prefix("metadata-")
            .and_then(|point| point.split('-').next())
        {
            let retired = RESOLUTION_MERGE_MEMBERS
                .iter()
                .position(|name| *name == member)
                .unwrap();
            for (ordinal, name) in RESOLUTION_MERGE_MEMBERS.iter().enumerate() {
                assert_eq!(
                    gitdir.join(name).exists(),
                    ordinal > retired,
                    "metadata retirement order changed"
                );
            }
        }
        let (artifact_phase, ref_phase): (String, String) = connection
            .query_row(
                "SELECT phase,ref_phase FROM remote_resolution_index_artifacts",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(
            artifact_phase
                == if released || point == "release-intent" {
                    "release_intent"
                } else if point.starts_with("sentinel-") {
                    "intent"
                } else {
                    "published"
                },
            "unexpected sentinel observation boundary"
        );
        if matches!(point, "ref-locked" | "ref-intent" | "ref-committed") {
            assert!(
                ref_phase
                    == if point == "ref-locked" {
                        "not_started"
                    } else {
                        "intent"
                    },
                "unexpected ref observation boundary"
            );
        }
        if committed {
            assert!(
                candidate.as_deref()
                    == Some(
                        repository
                            .head()
                            .unwrap()
                            .target()
                            .unwrap()
                            .to_string()
                            .as_str()
                    ),
                "candidate ref changed"
            );
            assert!(
                killed_logs
                    .iter()
                    .zip(&baseline_logs)
                    .all(|(result, baseline)| result.starts_with(baseline)
                        && result.len() > baseline.len()),
                "expected log append absent"
            );
        } else {
            assert_eq!(repository.head().unwrap().target(), Some(old));
            assert!(killed_logs == baseline_logs, "precommit logs changed");
        }
        if matches!(point, "ref-locked" | "ref-intent") {
            let backend_lock = gitdir.join("refs/heads/main.lock");
            let ambiguous = image(&backend_lock);
            let refused_before = refusal_snapshot(&repository, root.path(), request.attempt_id);
            recover(root.path(), data.path(), ipc.path(), Some("required"));
            assert!(
                refusal_snapshot(&repository, root.path(), request.attempt_id) == refused_before,
                "backend-lock refusal changed protected images"
            );
            assert!(
                image(&backend_lock) == ambiguous,
                "ambiguous backend lock changed"
            );
            assert!(
                logs.each_ref().map(|path| fs::read(path).unwrap()) == baseline_logs,
                "refused recovery changed logs"
            );
            assert_eq!(repository.head().unwrap().target(), Some(old));
            // Simulated operator ONLY on the parent-owned disposable fixture:
            // all children reaped; exact old ref/logs and lock identity verified.
            fs::remove_file(backend_lock).unwrap();
        }
        if candidate.is_some() {
            // Frozen ODB signer is authoritative after candidate creation even
            // when effective configuration changes between real processes.
            config.set_str("user.name", "Changed Identity").unwrap();
            config
                .set_str("user.email", "changed@example.invalid")
                .unwrap();
            let mut altered = request.clone();
            altered.resolutions[0].1 =
                RedactedConflictBytes::from_bytes(b"altered same-ID bytes".to_vec());
            fs::write(
                ipc.path().join("request"),
                serde_yaml::to_string(&FixtureRequest::from_request(&altered)).unwrap(),
            )
            .unwrap();
            let refused_before = refusal_snapshot(&repository, root.path(), request.attempt_id);
            recover(root.path(), data.path(), ipc.path(), Some("required"));
            assert!(
                refusal_snapshot(&repository, root.path(), request.attempt_id) == refused_before,
                "altered-request refusal changed protected images"
            );
            assert!(
                image(&canonical) == installed_path
                    && image(&gitdir.join("index")) == installed_index,
                "altered retry changed owned effects"
            );
            assert!(
                logs.each_ref().map(|path| fs::read(path).unwrap()) == killed_logs,
                "altered retry changed logs"
            );
            fs::write(
                ipc.path().join("request"),
                serde_yaml::to_string(&FixtureRequest::from_request(&request)).unwrap(),
            )
            .unwrap();
        }
        recover(root.path(), data.path(), ipc.path(), None);
        let final_candidate = repository.head().unwrap().target().unwrap();
        assert_ne!(final_candidate, old);
        if let Some(candidate) = candidate {
            assert!(
                final_candidate.to_string() == candidate,
                "frozen candidate was replaced"
            );
        }
        assert!(
            fs::read(&canonical).unwrap() == request.resolutions[0].1.bytes(),
            "caller bytes changed"
        );
        if !matches!(point, "sentinel-linked" | "sentinel-published") {
            assert!(
                image(&canonical) == installed_path,
                "exact installed result was rewritten"
            );
        }
        if installed_index != baseline_index {
            assert!(
                image(&gitdir.join("index")) == installed_index,
                "installed index was rewritten"
            );
        }
        let final_logs = logs.each_ref().map(|path| fs::read(path).unwrap());
        if committed {
            assert!(
                final_logs == killed_logs,
                "completed candidate recovery appended again"
            );
        }
        for (result, baseline) in final_logs.iter().zip(&baseline_logs) {
            let commit = repository.find_commit(final_candidate).unwrap();
            let signer = commit.committer();
            let entry = format!(
                "{old} {final_candidate} {} <{}> 0 +0000\t{RESOLUTION_REF_MESSAGE}\n",
                signer.name().unwrap(),
                signer.email().unwrap()
            );
            let mut expected = baseline.clone();
            expected.extend_from_slice(entry.as_bytes());
            assert!(
                *result == expected,
                "ref transition did not append exactly once"
            );
        }
        assert!(!sentinel.exists());
        assert!(
            RESOLUTION_MERGE_MEMBERS
                .iter()
                .all(|member| !gitdir.join(member).exists())
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM remote_resolution_attempts",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert!(
            connection
                .query_row(
                    "SELECT phase FROM remote_resolution_index_artifacts",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap()
                == "released"
        );
        let commit = repository.find_commit(final_candidate).unwrap();
        assert!(
            commit.committer().name() == Some(SIGNER) && commit.committer().email() == Some(EMAIL),
            "frozen signer changed"
        );
        let staging = gitdir.join(format!(".manyhands-resolution-{}", request.attempt_id));
        let mut evidence_files = vec![
            data.path().join(REGISTRY_FILE),
            data.path().join(format!("{REGISTRY_FILE}-wal")),
        ];
        for role in ["baseline", "transition"] {
            evidence_files.push(staging.join(format!("ref-log-{role}")));
            evidence_files.push(staging.join(format!("ref-log-{role}-anchor")));
        }
        for path in evidence_files {
            if let Ok(bytes) = fs::read(path) {
                for canary in [BODY, SIGNER.as_bytes(), EMAIL.as_bytes(), HISTORY] {
                    assert!(
                        !bytes.windows(canary.len()).any(|window| window == canary),
                        "death recovery evidence leaked raw authority"
                    );
                }
            }
        }
        drop(
            repository_lease(
                &repository,
                root.path(),
                RepositoryOperation::RepositorySnapshot,
            )
            .expect("completed recovery retained writer lease"),
        );
    }

    #[test]
    fn resolution_process_death_owned_publication_path_and_index() {
        for point in [
            "sentinel-linked",
            "sentinel-published",
            "canonical-installed",
            "scratch-serialized",
            "scratch-prepared",
            "output-recorded",
            "index-install-linked",
            "index-renamed",
            "index-installed",
        ] {
            prove(point);
        }
    }

    #[test]
    fn resolution_process_death_backend_lock_intent_and_commit() {
        for point in ["ref-locked", "ref-intent", "ref-committed"] {
            prove(point);
        }
    }

    #[test]
    fn resolution_process_death_each_metadata_retirement_and_release() {
        for point in [
            "metadata-MERGE_HEAD-unlinked",
            "metadata-MERGE_HEAD-barrier",
            "metadata-MERGE_MSG-unlinked",
            "metadata-MERGE_MSG-barrier",
            "metadata-MERGE_MODE-unlinked",
            "metadata-MERGE_MODE-barrier",
            "release-intent",
            "sentinel-unlinked",
            "sentinel-release-barrier",
        ] {
            prove(point);
        }
    }

    #[test]
    fn resolution_process_death_foreign_identity_and_metadata_stay_fenced() {
        for point in [
            "output-recorded",
            "metadata-MERGE_HEAD-unlinked",
            "sentinel-unlinked",
        ] {
            let (root, data, service, request) = protocol_resolution_fixture();
            let ipc = tempfile::tempdir().unwrap();
            fs::write(
                ipc.path().join("request"),
                serde_yaml::to_string(&FixtureRequest::from_request(&request)).unwrap(),
            )
            .unwrap();
            drop(service);
            kill_at(root.path(), data.path(), ipc.path(), point);
            let repository = git2::Repository::open(root.path()).unwrap();
            let gitdir = repository.path();
            let sentinel = gitdir.join("index.lock");
            let anchor = gitdir.join(format!(
                ".manyhands-resolution-{}/sentinel",
                request.attempt_id
            ));
            let foreign = if point.starts_with("metadata-") {
                let member = gitdir.join("MERGE_MSG");
                fs::write(&member, b"foreign merge metadata canary").unwrap();
                member
            } else {
                // Byte-identical content is never authority. Atomic substitution
                // keeps the original anchored inode alive for identity comparison.
                let replacement = gitdir.join("fixture-foreign-sentinel");
                fs::write(&replacement, fs::read(&anchor).unwrap()).unwrap();
                fs::rename(replacement, &sentinel).unwrap();
                assert!(
                    image(&sentinel).0 != image(&anchor).0
                        || image(&sentinel).1 != image(&anchor).1
                );
                sentinel.clone()
            };
            assert!(foreign.exists());
            let refused_before = refusal_snapshot(&repository, root.path(), request.attempt_id);
            recover(root.path(), data.path(), ipc.path(), Some("external"));
            assert!(
                refusal_snapshot(&repository, root.path(), request.attempt_id) == refused_before,
                "foreign-effect refusal changed protected images"
            );
            assert!(
                sentinel.exists(),
                "refused recovery released fencing sentinel"
            );
        }
    }
}
