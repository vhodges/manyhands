use super::*;
use crate::repository::{EnableRepositoryRequest, FailurePoint, REGISTRY_FILE};
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
