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
