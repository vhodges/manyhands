use super::*;
use keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest};

struct NoPrompt;
impl SessionCredentialProvider for NoPrompt {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("local comment must not prompt")
    }
}

struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    item: canonical::ItemId,
    context: ItemContext,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(root.path(), &options).unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Comment fixture").unwrap();
        config
            .set_str("user.email", "comment@example.invalid")
            .unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        // Construct a born canonical fixture before any service operation. No
        // index repair follows an authoring save or a comment checkpoint.
        let item = canonical::ItemId::generate();
        std::fs::create_dir(root.path().join(".manyhands")).unwrap();
        std::fs::create_dir(root.path().join("docs")).unwrap();
        std::fs::write(
            root.path().join(".manyhands/config.toml"),
            canonical::serialize_repository_config(&canonical::RepositoryConfig {
                primary_branch: "main".into(),
                publication_remote: None,
                unknown: toml::Table::new(),
            })
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("docs/a.md"),
            canonical::serialize_item(&canonical::CanonicalItem::Document(canonical::Document {
                id: item.clone(),
                title: "Fixture".into(),
                body: "original document\n".into(),
                unknown: serde_yaml::Mapping::new(),
            }))
            .unwrap(),
        )
        .unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new(".manyhands/config.toml")).unwrap();
        index.add_path(Path::new("docs/a.md")).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = repository.signature().unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "Fixture", &tree, &[])
            .unwrap();
        index.write().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let outcome = service
            .prepare_context(AuthoringTarget {
                root: root.path().into(),
                kind: AuthoringKind::Document,
                item_id: item.clone(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let context = match outcome {
            ContextProvisionOutcome::Created(context)
            | ContextProvisionOutcome::Reused(context) => context,
            _ => panic!("fixture checkpoint"),
        };
        Self {
            root,
            data,
            service,
            item,
            context,
        }
    }
    fn request(&self) -> PublishCommentRequest {
        PublishCommentRequest {
            comment: SubmitCommentRequest {
                target: AuthoringTarget {
                    root: self.root.path().into(),
                    kind: AuthoringKind::Document,
                    item_id: self.item.clone(),
                    intent: ContextIntent::Edit,
                    operation_id: OperationId::new(),
                },
                comment_id: canonical::ItemId::generate(),
                parent_id: None,
                body: "private-comment-sentinel\n".into(),
                expected_destination: ExpectedPathObservation::Missing,
            },
            approval: None,
            confirmed_identity: None,
        }
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }
    fn configure_remote(&self) {
        self.service
            .add_remote(AddRemoteRequest {
                root: self.root.path().into(),
                name: "origin".into(),
                url: "ssh://fixture@localhost/repository".into(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        self.service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: self.root.path().into(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            })
            .unwrap();
    }
}
fn saved(
    value: CommentSubmissionOutcome,
) -> (
    CommentReceipt,
    CommentPublicationState,
    CommentIndexingState,
) {
    match value {
        CommentSubmissionOutcome::Saved {
            receipt,
            publication,
            indexing,
            ..
        } => (*receipt, publication, indexing),
        _ => panic!("expected saved receipt"),
    }
}

#[test]
fn comment_publication_local_only_receipt_reuses_original_checkpoint_and_child() {
    let f = Fixture::new();
    let request = f.request();
    let mut session = SessionCredentials::new(NoPrompt);
    let (receipt, publication, indexing) = saved(
        f.service
            .submit_comment(request.clone(), &mut session)
            .unwrap(),
    );
    assert!(matches!(
        publication,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::NoPublicationRemote
        }
    ));
    assert!(!indexing.local_pending && !indexing.remote_pending);
    assert_eq!(receipt.operation_id, request.comment.target.operation_id);
    assert_ne!(receipt.synchronization_id, receipt.operation_id);
    assert_eq!(receipt.comment_id, request.comment.comment_id);
    let repository = Repository::open(&f.context.worktree).unwrap();
    assert_eq!(
        repository.head().unwrap().target(),
        Some(receipt.checkpoint_oid)
    );
    let before = std::fs::read(f.context.worktree.join(&receipt.comment_path)).unwrap();
    let (again, _, _) = saved(
        f.service
            .retry_comment_publication(
                RetryCommentPublicationRequest {
                    root: f.root.path().into(),
                    operation_id: receipt.operation_id,
                    approval: None,
                    confirmed_identity: None,
                    restart: false,
                },
                &mut session,
            )
            .unwrap(),
    );
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    assert_eq!(
        std::fs::read(f.context.worktree.join(&receipt.comment_path)).unwrap(),
        before
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        f.db()
            .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
}

#[test]
fn comment_publication_identity_required_never_writes_a_comment() {
    let f = Fixture::new();
    let request = f.request();
    let repository = Repository::open(&f.context.worktree).unwrap();
    let mut config = repository.config().unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    let effective = Config::new().unwrap();
    let result = f
        .service
        .submit_comment_with_identity_config_for_testing(
            request.clone(),
            &effective,
            &mut SessionCredentials::new(NoPrompt),
        )
        .unwrap();
    assert!(matches!(
        result,
        CommentSubmissionOutcome::IdentityRequired { .. }
    ));
    assert!(
        !f.context
            .worktree
            .join(format!(
                ".manyhands/comments/{}/{}.md",
                f.item, request.comment.comment_id
            ))
            .exists()
    );
}

#[test]
fn comment_publication_commit_then_journal_failure_still_returns_saved() {
    let f = Fixture::new();
    let request = f.request();
    f.db().execute_batch("CREATE TRIGGER stop_receipt BEFORE UPDATE ON operation_records WHEN NEW.completed_step='authoring_checkpoint_observed' BEGIN SELECT RAISE(ABORT, 'fixed failure'); END;").unwrap();
    let (receipt, state, index) = saved(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    assert!(index.local_pending);
    assert!(matches!(
        state,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::LocalRecoveryRequired
        }
    ));
    assert_eq!(
        Repository::open(&f.context.worktree)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        Some(receipt.checkpoint_oid)
    );
    f.db().execute_batch("DROP TRIGGER stop_receipt;").unwrap();
    let (again, _, repaired) = saved(
        f.service
            .retry_comment_publication(
                RetryCommentPublicationRequest {
                    root: f.root.path().into(),
                    operation_id: receipt.operation_id,
                    approval: None,
                    confirmed_identity: None,
                    restart: false,
                },
                &mut SessionCredentials::new(NoPrompt),
            )
            .unwrap(),
    );
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(
        !repaired.local_pending,
        "proved checkpoint must repair its unfinished journal/discovery handoff"
    );
}

#[test]
fn comment_publication_diagnostics_redact_request_and_receipt_paths() {
    let f = Fixture::new();
    let request = f.request();
    let text = format!("{request:?}");
    assert!(!text.contains("private-comment-sentinel"));
    assert!(!text.contains(f.root.path().to_str().unwrap()));
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(request, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    assert!(!format!("{receipt:?}").contains(f.root.path().to_str().unwrap()));
}

#[test]
fn comment_publication_rejected_row_refresh_and_reopen_keep_one_binding() {
    let f = Fixture::new();
    let mut request = f.request();
    let parent = canonical::ItemId::generate();
    request.comment.parent_id = Some(parent.clone());
    assert!(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .is_err()
    );
    let child: String = f
        .db()
        .query_row(
            "SELECT synchronization_ulid FROM comment_publication_bindings",
            [],
            |r| r.get(0),
        )
        .unwrap();
    f.service
        .refresh_repository(RefreshRepositoryRequest {
            root: f.root.path().into(),
            operation_id: request.comment.target.operation_id,
        })
        .unwrap();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    // A rejected call may run again; immutable parent identity still cannot change.
    request.comment.parent_id = None;
    assert!(
        matches!(reopened.submit_comment(request, &mut SessionCredentials::new(NoPrompt)), Err(error) if error.kind == RepositoryErrorKind::OperationMismatch)
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT synchronization_ulid FROM comment_publication_bindings",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        child
    );
}

#[test]
fn comment_publication_receiptless_identity_retry_refuses_changed_pre_checkpoint_before_write() {
    let f = Fixture::new();
    let request = f.request();
    let repository = Repository::open(&f.context.worktree).unwrap();
    let mut config = repository.config().unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    assert!(matches!(
        f.service
            .submit_comment_with_identity_config_for_testing(
                request.clone(),
                &Config::new().unwrap(),
                &mut SessionCredentials::new(NoPrompt)
            )
            .unwrap(),
        CommentSubmissionOutcome::IdentityRequired { .. }
    ));
    config.set_str("user.name", "Comment fixture").unwrap();
    config
        .set_str("user.email", "comment@example.invalid")
        .unwrap();
    let path = f.context.worktree.join("docs/a.md");
    f.service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: f.root.path().into(),
                kind: AuthoringKind::Document,
                item_id: f.item.clone(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            source_path: Some("docs/a.md".into()),
            destination_path: "docs/a.md".into(),
            draft: DocumentDraft {
                title: "Fixture".into(),
                body: "intervening checkpoint\n".into(),
            },
            expected_source: Some(ExpectedPathObservation::from_bytes(
                &std::fs::read(&path).unwrap(),
            )),
            expected_destination: ExpectedPathObservation::from_bytes(
                &std::fs::read(path).unwrap(),
            ),
        })
        .unwrap();
    let head = repository.head().unwrap().target();
    assert!(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .is_err()
    );
    assert_eq!(
        repository.head().unwrap().target(),
        head,
        "refusal must precede a new comment checkpoint"
    );
    assert!(
        !f.context
            .worktree
            .join(format!(
                ".manyhands/comments/{}/{}.md",
                f.item, request.comment.comment_id
            ))
            .exists()
    );
}

#[test]
fn comment_publication_receiptless_replay_never_recreates_an_externally_deleted_checkpoint() {
    let f = Fixture::new();
    let request = f.request();
    f.db().execute_batch("CREATE TRIGGER stop_receipt BEFORE UPDATE OF checkpoint_oid ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    f.db().execute_batch("DROP TRIGGER stop_receipt;").unwrap();
    let repository = Repository::open(&f.context.worktree).unwrap();
    std::fs::remove_file(f.context.worktree.join(&receipt.comment_path)).unwrap();
    let mut index = repository.index().unwrap();
    index.remove_path(&receipt.comment_path).unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repository.find_commit(receipt.checkpoint_oid).unwrap();
    let signature = repository.signature().unwrap();
    let removed = repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "External removal",
            &tree,
            &[&parent],
        )
        .unwrap();
    index.write().unwrap();
    let (again, state, _) = saved(
        f.service
            .submit_comment(request, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(matches!(state, CommentPublicationState::Pending {
        reason: CommentPublicationPendingReason::Synchronization(error)
    } if matches!(*error, SynchronizationError::RecoveryRequired)));
    assert_eq!(repository.head().unwrap().target(), Some(removed));
    assert!(!f.context.worktree.join(&receipt.comment_path).exists());
}

#[test]
fn comment_publication_bound_child_cannot_be_consumed_by_local_context_or_plain_refresh() {
    for refresh in [false, true] {
        let f = Fixture::new();
        let (receipt, _, _) = saved(
            f.service
                .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        let kind = if refresh {
            f.service
                .refresh_repository(RefreshRepositoryRequest {
                    root: f.root.path().into(),
                    operation_id: receipt.synchronization_id,
                })
                .err()
                .map(|e| e.kind)
        } else {
            f.service
                .prepare_context(AuthoringTarget {
                    root: f.root.path().into(),
                    kind: AuthoringKind::Document,
                    item_id: f.item.clone(),
                    intent: ContextIntent::Edit,
                    operation_id: receipt.synchronization_id,
                })
                .err()
                .map(|e| e.kind)
        };
        assert_eq!(kind, Some(RepositoryErrorKind::OperationMismatch));
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT COUNT(*) FROM operation_records WHERE operation_ulid=?1",
                    [receipt.synchronization_id.to_string()],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert!(RepositoryService::open_at(f.data.path()).is_ok());
    }
}

#[test]
fn comment_publication_bound_child_refuses_other_remote_actions_and_targets() {
    for action in [0, 1, 2] {
        let f = Fixture::new();
        let (receipt, _, _) = saved(
            f.service
                .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = match action {
            0 => RemoteOperationTarget::for_primary_synchronization(&plan),
            1 => RemoteOperationTarget::for_poll(&plan),
            _ => RemoteOperationTarget::for_context(
                &plan,
                RemoteOperationAction::SynchronizeContext,
                AuthoringKind::Document,
                canonical::ItemId::generate(),
            )
            .unwrap(),
        };
        assert!(
            matches!(f.service.reserve_remote_operation(f.root.path(), receipt.synchronization_id, &target), Err(e) if e.kind == RepositoryErrorKind::OperationMismatch)
        );
        assert_eq!(
            f.db()
                .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
    }
}

#[test]
fn comment_publication_startup_refuses_substituted_schema_root_and_item_identity() {
    for corruption in [0, 1, 2] {
        let f = Fixture::new();
        saved(
            f.service
                .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        match corruption {
            0 => f.db().execute_batch("PRAGMA foreign_keys=OFF; ALTER TABLE comment_publication_bindings RENAME TO old_bindings;
                CREATE TABLE comment_publication_bindings AS SELECT * FROM old_bindings; DROP TABLE old_bindings;").unwrap(),
            1 => { f.db().execute("UPDATE comment_publication_bindings SET root_digest=zeroblob(32)", []).unwrap(); },
            _ => { f.db().execute("UPDATE comment_publication_bindings SET item_ulid=?1", [canonical::ItemId::generate().to_string()]).unwrap(); },
        }
        assert!(
            RepositoryService::open_at(f.data.path()).is_err(),
            "substituted registry must fail closed; category={corruption}"
        );
    }
}

#[test]
fn comment_publication_unreachable_receiptless_checkpoint_never_generates_a_replacement() {
    let f = Fixture::new();
    let request = f.request();
    f.db().execute_batch("CREATE TRIGGER stop_receipt BEFORE UPDATE OF checkpoint_oid ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    f.db().execute_batch("DROP TRIGGER stop_receipt;").unwrap();
    let repository = Repository::open(&f.context.worktree).unwrap();
    let pre = repository
        .find_commit(receipt.checkpoint_oid)
        .unwrap()
        .parent_id(0)
        .unwrap();
    repository
        .find_reference(&format!("refs/heads/{}", receipt.context_branch))
        .unwrap()
        .set_target(pre, "external reset")
        .unwrap();
    repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    assert!(!f.context.worktree.join(&receipt.comment_path).exists());
    assert!(
        f.service
            .submit_comment(request, &mut SessionCredentials::new(NoPrompt))
            .is_err()
    );
    assert_eq!(repository.head().unwrap().target(), Some(pre));
    assert!(!f.context.worktree.join(&receipt.comment_path).exists());
}

#[test]
fn comment_publication_binding_and_file_faults_reopen_without_duplicate_effects() {
    for fault in [0, 1, 2, 3] {
        let f = Fixture::new();
        let request = f.request();
        let path = f.context.worktree.join(format!(
            ".manyhands/comments/{}/{}.md",
            f.item, request.comment.comment_id
        ));
        let service = match fault {
            1 => RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::BeforeItemWrite,
            )
            .unwrap(),
            2 => RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::BeforeCheckpointCommit,
            )
            .unwrap(),
            3 => RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::BeforeRegistryWrite,
            )
            .unwrap(),
            _ => {
                f.db().execute_batch("CREATE TRIGGER stop_binding BEFORE INSERT ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
                RepositoryService::open_at(f.data.path()).unwrap()
            }
        };
        let first = service.submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt));
        let retained = std::fs::read(&path).ok();
        let child: Option<String> = f
            .db()
            .query_row(
                "SELECT synchronization_ulid FROM comment_publication_bindings",
                [],
                |r| r.get(0),
            )
            .optional()
            .unwrap();
        if fault == 3 {
            assert!(first.is_ok());
        } else {
            assert!(first.is_err());
        }
        if fault == 0 {
            f.db().execute_batch("DROP TRIGGER stop_binding;").unwrap();
        }
        drop(service);
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        let (receipt, state, indexing) = saved(
            reopened
                .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        assert_eq!(receipt.operation_id, request.comment.target.operation_id);
        if let Some(child) = child {
            assert_eq!(receipt.synchronization_id.to_string(), child);
        }
        if let Some(bytes) = retained {
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        assert!(matches!(
            state,
            CommentPublicationState::Pending {
                reason: CommentPublicationPendingReason::NoPublicationRemote
            }
        ));
        assert!(!indexing.local_pending);
        let repository = Repository::open(&f.context.worktree).unwrap();
        assert_eq!(
            repository.head().unwrap().target(),
            Some(receipt.checkpoint_oid)
        );
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT COUNT(*) FROM comment_publication_bindings",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }
}

#[test]
fn comment_publication_binding_mismatch_and_changed_body_never_write_again() {
    let f = Fixture::new();
    let request = f.request();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    let repository = Repository::open(&f.context.worktree).unwrap();
    let bytes = std::fs::read(f.context.worktree.join(&receipt.comment_path)).unwrap();
    for change in [0, 1, 2, 3, 4] {
        let mut altered = request.clone();
        match change {
            0 => altered.comment.parent_id = Some(canonical::ItemId::generate()),
            1 => altered.comment.target.kind = AuthoringKind::Ticket,
            2 => altered.comment.comment_id = canonical::ItemId::generate(),
            3 => altered.comment.body = "different-body-sentinel\n".into(),
            _ => altered.comment.target.operation_id = OperationId::new(),
        }
        assert!(
            f.service
                .submit_comment(altered, &mut SessionCredentials::new(NoPrompt))
                .is_err()
        );
        assert_eq!(
            repository.head().unwrap().target(),
            Some(receipt.checkpoint_oid)
        );
        assert_eq!(
            std::fs::read(f.context.worktree.join(&receipt.comment_path)).unwrap(),
            bytes
        );
    }
    assert_eq!(
        f.db()
            .query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn comment_publication_legacy_rows_migrate_without_inferred_correlation_or_publication() {
    let f = Fixture::new();
    let request = f.request();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(request, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    let original_step: String = f
        .db()
        .query_row(
            "SELECT completed_step FROM operation_records WHERE operation_ulid=?1",
            [receipt.operation_id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    f.db().execute_batch("DROP TABLE comment_publication_bindings; DELETE FROM registry_migrations WHERE name='cycle_07_comment_publication_bindings';").unwrap();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        f.db()
            .query_row(
                "SELECT completed_step FROM operation_records WHERE operation_ulid=?1",
                [receipt.operation_id.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        original_step
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert!(
        matches!(reopened.retry_comment_publication(RetryCommentPublicationRequest { root: f.root.path().into(), operation_id: receipt.operation_id,
        approval: None, confirmed_identity: None, restart: false }, &mut SessionCredentials::new(NoPrompt)), Err(e) if e.kind == RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        Repository::open(&f.context.worktree)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        Some(receipt.checkpoint_oid)
    );
    assert_eq!(
        f.db()
            .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
}

#[test]
fn comment_publication_persistence_excludes_bodies_and_body_fingerprints() {
    let f = Fixture::new();
    let db = f.db();
    db.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    let mut request = f.request();
    request.comment.body = "body-private-canary ssh://endpoint-private-canary passphrase-private-canary server-private-canary\n".into();
    let body_hash = blake3::hash(request.comment.body.as_bytes())
        .to_hex()
        .to_string();
    saved(
        f.service
            .submit_comment(request, &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    for entry in std::fs::read_dir(f.data.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let bytes = std::fs::read(entry.path()).unwrap();
            for forbidden in [
                "body-private-canary",
                "endpoint-private-canary",
                "passphrase-private-canary",
                "server-private-canary",
                &body_hash,
            ] {
                assert!(
                    !bytes
                        .windows(forbidden.len())
                        .any(|window| window == forbidden.as_bytes()),
                    "persistence privacy category"
                );
            }
        }
    }
}

#[test]
fn comment_publication_correct_child_target_is_reserved_once() {
    let f = Fixture::new();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    f.service
        .add_remote(AddRemoteRequest {
            root: f.root.path().into(),
            name: "origin".into(),
            url: "ssh://fixture@localhost/repository".into(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    f.service
        .set_publication_remote(SetPublicationRemoteRequest {
            root: f.root.path().into(),
            name: Some("origin".into()),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        receipt.kind,
        receipt.item_id.clone(),
    )
    .unwrap();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root.path(), receipt.synchronization_id, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root.path(), receipt.synchronization_id, &target)
            .unwrap(),
        RemoteReservationOutcome::Replay(_)
    ));
    assert_eq!(
        f.db()
            .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        1
    );
}

#[cfg(unix)]
#[test]
fn comment_publication_simultaneous_services_keep_one_binding_and_checkpoint() {
    let f = Fixture::new();
    let request = f.request();
    let competing = RepositoryService::open_at(f.data.path()).unwrap();
    let db = f.db();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    f.service.set_owned_path_hook_for_root_for_testing(
        f.context.worktree.clone(),
        format!(
            ".manyhands/comments/{}/{}.md",
            f.item, request.comment.comment_id
        )
        .into(),
        OwnedPathBoundary::Replace,
        move || {
            ready_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
        },
    );
    let first_request = request.clone();
    let service = f.service;
    let first = std::thread::spawn(move || {
        service.submit_comment(first_request, &mut SessionCredentials::new(NoPrompt))
    });
    ready_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let child: String = db
        .query_row(
            "SELECT synchronization_ulid FROM comment_publication_bindings",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let second = competing.submit_comment(request, &mut SessionCredentials::new(NoPrompt));
    release_tx.send(()).unwrap();
    assert!(matches!(second, Err(e) if e.kind == RepositoryErrorKind::RepositoryBusy));
    let (receipt, _, _) = saved(first.join().unwrap().unwrap());
    assert_eq!(receipt.synchronization_id.to_string(), child);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM comment_publication_bindings",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        Repository::open(&f.context.worktree)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        Some(receipt.checkpoint_oid)
    );
}

#[test]
fn comment_publication_original_action_cannot_move_to_another_registered_root() {
    let f = Fixture::new();
    let other = Fixture::new();
    let request = f.request();
    saved(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    f.service
        .enable(EnableRepositoryRequest {
            root: other.root.path().into(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    let mut foreign = other.request();
    foreign.comment.target.operation_id = request.comment.target.operation_id;
    assert!(
        matches!(f.service.submit_comment(foreign, &mut SessionCredentials::new(NoPrompt)), Err(e) if e.kind == RepositoryErrorKind::OperationMismatch)
    );
}

#[test]
fn comment_publication_binding_transaction_rechecks_the_parent_record_identity() {
    let f = Fixture::new();
    let request = f.request();
    let unrelated = f
        .db()
        .query_row(
            "SELECT id FROM operation_records WHERE action='prepare_context' LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    let repository = Repository::open(f.root.path()).unwrap();
    assert!(
        matches!(f.service.bind_comment_publication(&repository, f.root.path(), &request.comment,
        recovery::RecoveryRecord { id: unrelated, is_new: true, is_pending: false, completed_step: None }), Err(e) if e.kind == RepositoryErrorKind::OperationMismatch)
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn comment_publication_missing_receipt_authority_never_recreates_a_comment_or_checkpoint() {
    for missing in [0, 1, 2, 3] {
        let f = Fixture::new();
        let request = f.request();
        f.db().execute_batch("CREATE TRIGGER stop_receipt BEFORE UPDATE OF checkpoint_oid ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
        let (receipt, _, _) = saved(
            f.service
                .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        f.db().execute_batch("DROP TRIGGER stop_receipt;").unwrap();
        let repository = Repository::open(&f.context.worktree).unwrap();
        let commit = repository.find_commit(receipt.checkpoint_oid).unwrap();
        let pre = commit.parent_id(0).unwrap();
        let tree = commit.tree().unwrap();
        let missing_oid = match missing {
            0 => receipt.checkpoint_oid,
            1 => tree.id(),
            _ => tree.get_path(&receipt.comment_path).unwrap().id(),
        };
        let original_bytes = std::fs::read(f.context.worktree.join(&receipt.comment_path)).unwrap();
        repository
            .find_reference(&format!("refs/heads/{}", receipt.context_branch))
            .unwrap()
            .set_target(pre, "external reset")
            .unwrap();
        if missing != 3 {
            repository
                .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                .unwrap();
        }
        let oid = missing_oid.to_string();
        std::fs::remove_file(
            repository
                .commondir()
                .join("objects")
                .join(&oid[..2])
                .join(&oid[2..]),
        )
        .unwrap();
        drop(tree);
        drop(commit);
        drop(repository);
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert!(
            matches!(reopened.submit_comment(request, &mut SessionCredentials::new(NoPrompt)), Err(e) if e.kind == RepositoryErrorKind::RecoveryRequired),
            "missing authority category={missing}"
        );
        assert_eq!(
            Repository::open(&f.context.worktree)
                .unwrap()
                .head()
                .unwrap()
                .target(),
            Some(pre)
        );
        if missing == 3 {
            assert_eq!(
                std::fs::read(f.context.worktree.join(&receipt.comment_path)).unwrap(),
                original_bytes
            );
        } else {
            assert!(!f.context.worktree.join(&receipt.comment_path).exists());
        }
    }
}

fn metadata_rows(
    connection: &rusqlite::Connection,
    table: &str,
) -> Vec<Vec<rusqlite::types::Value>> {
    let mut statement = connection
        .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
        .unwrap();
    let count = statement.column_count();
    statement
        .query_map([], |r| {
            (0..count)
                .map(|i| r.get(i))
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

#[test]
fn comment_publication_migration_preserves_pending_comment_and_interrupted_remote_rows() {
    let f = Fixture::new();
    f.configure_remote();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let operation = OperationId::new();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        canonical::ItemId::generate(),
    )
    .unwrap();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root.path(), operation, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    f.db()
        .execute(
            "UPDATE remote_operation_records SET phase='interrupted' WHERE operation_ulid=?1",
            [operation.to_string()],
        )
        .unwrap();
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeCheckpointCommit,
    )
    .unwrap();
    assert!(
        failing
            .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
            .is_err()
    );
    let local = metadata_rows(&f.db(), "operation_records");
    let remote = metadata_rows(&f.db(), "remote_operation_records");
    let head = Repository::open(&f.context.worktree)
        .unwrap()
        .head()
        .unwrap()
        .target();
    f.db().execute_batch("DROP TABLE comment_publication_bindings; DELETE FROM registry_migrations WHERE name='cycle_07_comment_publication_bindings';").unwrap();
    RepositoryService::open_at(f.data.path()).unwrap();
    assert!(
        metadata_rows(&f.db(), "operation_records") == local,
        "legacy local rows changed during migration"
    );
    assert!(
        metadata_rows(&f.db(), "remote_operation_records") == remote,
        "legacy remote rows changed during migration"
    );
    assert_eq!(
        Repository::open(&f.context.worktree)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        head
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn comment_publication_migration_failure_after_table_creation_rolls_back_table_and_marker() {
    let f = Fixture::new();
    f.db().execute_batch("DROP TABLE comment_publication_bindings; DELETE FROM registry_migrations WHERE name='cycle_07_comment_publication_bindings';
        CREATE TRIGGER stop_migration BEFORE INSERT ON registry_migrations WHEN NEW.name='cycle_07_comment_publication_bindings' BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
    assert!(RepositoryService::open_at(f.data.path()).is_err());
    assert!(!f.db().query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='comment_publication_bindings')", [], |r| r.get::<_, bool>(0)).unwrap());
    assert!(!f.db().query_row("SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name='cycle_07_comment_publication_bindings')", [], |r| r.get::<_, bool>(0)).unwrap());
    f.db()
        .execute_batch("DROP TRIGGER stop_migration;")
        .unwrap();
    RepositoryService::open_at(f.data.path()).unwrap();
}

#[test]
fn comment_publication_competing_plausible_checkpoints_require_recovery() {
    let f = Fixture::new();
    let request = f.request();
    f.db().execute_batch("CREATE TRIGGER stop_receipt BEFORE UPDATE OF checkpoint_oid ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    f.db().execute_batch("DROP TRIGGER stop_receipt;").unwrap();
    let repository = Repository::open(&f.context.worktree).unwrap();
    let original = repository.find_commit(receipt.checkpoint_oid).unwrap();
    let parent = original.parent(0).unwrap();
    let signature = Signature::new(
        "other",
        "other@example.invalid",
        &git2::Time::new(original.time().seconds() + 1, 0),
    )
    .unwrap();
    repository
        .commit(
            None,
            &signature,
            &signature,
            original.message().unwrap(),
            &original.tree().unwrap(),
            &[&parent],
        )
        .unwrap();
    assert!(
        matches!(f.service.retry_comment_publication(RetryCommentPublicationRequest { root: f.root.path().into(), operation_id: receipt.operation_id, approval: None, confirmed_identity: None, restart: false }, &mut SessionCredentials::new(NoPrompt)), Err(e) if e.kind == RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        repository.head().unwrap().target(),
        Some(receipt.checkpoint_oid)
    );
}

#[test]
fn comment_publication_authoritative_child_allows_only_its_plain_refresh_and_reopen() {
    // Durable-authority shape is arranged here; actual SSH authority is Task 5.
    let f = Fixture::new();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    f.configure_remote();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let target = RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        receipt.kind,
        receipt.item_id.clone(),
    )
    .unwrap();
    f.service
        .reserve_remote_operation(f.root.path(), receipt.synchronization_id, &target)
        .unwrap();
    f.db().execute("UPDATE remote_operation_records SET phase='completed',sync_checkpoint='discovery_pending',completed_step='before_discovery',expected_oid=?2,local_oid=?2,primary_tracking_oid=?2,push_oid=?2,push_advertised_oid=?2,authoritative_kind='already_current',authoritative_oid=?2,index_pending=1,outcome='completed' WHERE operation_ulid=?1",
        params![receipt.synchronization_id.to_string(),receipt.checkpoint_oid.to_string()]).unwrap();
    assert!(matches!(
        f.service
            .refresh_repository(RefreshRepositoryRequest {
                root: f.root.path().into(),
                operation_id: receipt.synchronization_id
            })
            .unwrap(),
        RefreshOutcome::Refreshed { .. }
    ));
    RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(f.db().query_row("SELECT COUNT(*) FROM operation_records WHERE operation_ulid=?1 AND action='refresh' AND target=''", [receipt.synchronization_id.to_string()], |r| r.get::<_, i64>(0)).unwrap(), 1);
}

#[test]
fn comment_publication_stops_in_pre_effect_intent_windows_require_recovery_without_replacement() {
    for point in [
        FailurePoint::CommentAfterDestinationPrepared,
        FailurePoint::CommentAfterCheckpointIntent,
    ] {
        let f = Fixture::new();
        let request = f.request();
        let failing =
            RepositoryService::open_at_with_failure_point_for_testing(f.data.path(), point)
                .unwrap();
        assert!(
            matches!(failing.submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt)), Err(e) if e.kind == RepositoryErrorKind::InjectedFailure)
        );
        let path = f.context.worktree.join(format!(
            ".manyhands/comments/{}/{}.md",
            f.item, request.comment.comment_id
        ));
        let before = std::fs::read(&path).ok();
        let head = Repository::open(&f.context.worktree)
            .unwrap()
            .head()
            .unwrap()
            .target();
        let time: String = f
            .db()
            .query_row(
                "SELECT created_at FROM comment_publication_bindings",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert!(
            matches!(reopened.submit_comment(request, &mut SessionCredentials::new(NoPrompt)), Err(e) if e.kind == RepositoryErrorKind::RecoveryRequired)
        );
        assert_eq!(std::fs::read(&path).ok(), before);
        assert_eq!(
            Repository::open(&f.context.worktree)
                .unwrap()
                .head()
                .unwrap()
                .target(),
            head
        );
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT created_at FROM comment_publication_bindings",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            time
        );
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT COUNT(*) FROM comment_publication_bindings",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }
}

#[test]
fn comment_publication_missing_registration_is_index_pending_and_repairs_before_publication() {
    let f = Fixture::new();
    f.service
        .remove_registration(RemoveRegistrationRequest {
            root: f.root.path().into(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let (receipt, state, indexing) = saved(
        f.service
            .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    assert!(indexing.local_pending);
    assert!(matches!(
        state,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::LocalRecoveryRequired
        }
    ));
    assert_eq!(
        f.db()
            .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    f.service
        .enable(EnableRepositoryRequest {
            root: f.root.path().into(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert!(f.service.recovery_inspection(f.root.path()).unwrap().iter().any(|entry| matches!(entry, RecoveryInspection::Pending { operation_id, .. } if *operation_id==receipt.operation_id)));
    let repository_id = f
        .db()
        .query_row("SELECT id FROM repositories", [], |r| r.get::<_, i64>(0))
        .unwrap();
    assert!(recovery::require_no_pending_local(&f.db(), repository_id).is_err());
    let (again, state, indexing) = saved(
        f.service
            .retry_comment_publication(
                RetryCommentPublicationRequest {
                    root: f.root.path().into(),
                    operation_id: receipt.operation_id,
                    approval: None,
                    confirmed_identity: None,
                    restart: false,
                },
                &mut SessionCredentials::new(NoPrompt),
            )
            .unwrap(),
    );
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(!indexing.local_pending);
    assert!(matches!(
        state,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::NoPublicationRemote
        }
    ));
    assert!(
        !f.service
            .repository_snapshot(f.root.path())
            .unwrap()
            .refresh_required
    );
    assert!(recovery::require_no_pending_local(&f.db(), repository_id).is_ok());
}

#[test]
fn comment_publication_live_frontmatter_failure_does_not_mask_the_child_recovery() {
    let f = Fixture::new();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    f.configure_remote();
    let conflict = b"<<<<<<< retained front matter conflict\n=======\n>>>>>>> incoming\n";
    std::fs::write(f.context.worktree.join("docs/a.md"), conflict).unwrap();
    let (again, state, _) = saved(
        f.service
            .retry_comment_publication(
                RetryCommentPublicationRequest {
                    root: f.root.path().into(),
                    operation_id: receipt.operation_id,
                    approval: None,
                    confirmed_identity: None,
                    restart: false,
                },
                &mut SessionCredentials::new(NoPrompt),
            )
            .unwrap(),
    );
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(
        matches!(state, CommentPublicationState::Pending { reason: CommentPublicationPendingReason::Synchronization(error) } if matches!(*error, SynchronizationError::TargetNotMaterialized))
    );
    assert_eq!(
        std::fs::read(f.context.worktree.join("docs/a.md")).unwrap(),
        conflict
    );
}

#[test]
fn comment_publication_pending_mapping_preserves_named_recoveries_and_redacts_repository_errors() {
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Document,
        item_id: canonical::ItemId::generate(),
    };
    let operation_id = OperationId::new();
    let errors = vec![
        SynchronizationError::Busy,
        SynchronizationError::PollYielding,
        SynchronizationError::Interrupted,
        SynchronizationError::WorktreeNotClean {
            target: target.clone(),
        },
        SynchronizationError::WorktreeConflicted {
            target: target.clone(),
        },
        SynchronizationError::ConflictPending {
            target: target.clone(),
            operation_id,
            stage: SynchronizationStage::Context,
        },
        SynchronizationError::ExternalResolutionRequired {
            target: target.clone(),
            operation_id,
        },
        SynchronizationError::PushRejected,
        SynchronizationError::RemoteContextDeleted,
        SynchronizationError::ExternalChange,
        SynchronizationError::RecoveryRequired,
    ];
    for error in errors {
        let expected = std::mem::discriminant(&error);
        let CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::Synchronization(mapped),
        } = comment_publication::pending_synchronization(error)
        else {
            panic!("saved pending mapping");
        };
        assert_eq!(std::mem::discriminant(&*mapped), expected);
    }
    let error = SynchronizationError::Repository(RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        Some("private-error-path-canary".into()),
        RepositoryErrorKind::Git,
        "private-server-message-canary",
    ));
    let result = comment_publication::pending_synchronization(error);
    assert!(!format!("{result:?}").contains("private-error-path-canary"));
    let CommentPublicationState::Pending {
        reason: CommentPublicationPendingReason::Synchronization(mapped),
    } = result
    else {
        panic!("repository mapping");
    };
    assert!(!format!("{mapped:?}").contains("private-server-message-canary"));
}

#[test]
fn comment_publication_registration_parking_recovers_after_metadata_interruptions() {
    for interruption in [0, 1] {
        let f = Fixture::new();
        f.service
            .remove_registration(RemoveRegistrationRequest {
                root: f.root.path().into(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        f.db().execute_batch(if interruption==0 {
            "CREATE TRIGGER stop_handoff BEFORE UPDATE OF checkpoint_oid ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;"
        } else {
            "CREATE TRIGGER stop_handoff BEFORE UPDATE ON operation_records WHEN NEW.completed_step='comment_registration_pending' BEGIN SELECT RAISE(ABORT,'fixed failure'); END;"
        }).unwrap();
        let (receipt, _, index) = saved(
            f.service
                .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        assert!(index.local_pending);
        f.db().execute_batch("DROP TRIGGER stop_handoff;").unwrap();
        let request = RetryCommentPublicationRequest {
            root: f.root.path().into(),
            operation_id: receipt.operation_id,
            approval: None,
            confirmed_identity: None,
            restart: false,
        };
        let (again, state, index) = saved(
            f.service
                .retry_comment_publication(request.clone(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
        assert!(index.local_pending);
        assert!(matches!(
            state,
            CommentPublicationState::Pending {
                reason: CommentPublicationPendingReason::LocalRecoveryRequired
            }
        ));
        f.service
            .enable(EnableRepositoryRequest {
                root: f.root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let (again, _, index) = saved(
            f.service
                .retry_comment_publication(request, &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
        assert!(!index.local_pending);
    }
}

#[test]
fn comment_publication_discovery_releases_leases_and_retry_preserves_active_index_owner() {
    let f = Fixture::new();
    let request = f.request();
    let competing = RepositoryService::open_at(f.data.path()).unwrap();
    let root = f.root.path().to_path_buf();
    let data = f.data.path().to_path_buf();
    let db = f.db();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    f.service.set_observation_hook_for_testing(move || {
        let lease =
            RepositoryService::hold_lease_for_testing(&root, &data, LeaseKind::Repository).unwrap();
        drop(lease);
        let lease =
            RepositoryService::hold_lease_for_testing(&root, &data, LeaseKind::CacheWrite).unwrap();
        drop(lease);
        ready_tx.send(()).unwrap();
        release_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
    });
    let id = request.comment.target.operation_id;
    let first = f.service;
    let worker = std::thread::spawn(move || {
        first.submit_comment(request, &mut SessionCredentials::new(NoPrompt))
    });
    ready_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let row = || {
        db.query_row(
            "SELECT state,index_owner_epoch FROM operation_records WHERE operation_ulid=?1",
            [id.to_string()],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap()
    };
    let before = row();
    assert_eq!(before.0, "indexing");
    let (_, _, index) = saved(
        competing
            .retry_comment_publication(
                RetryCommentPublicationRequest {
                    root: f.root.path().into(),
                    operation_id: id,
                    approval: None,
                    confirmed_identity: None,
                    restart: false,
                },
                &mut SessionCredentials::new(NoPrompt),
            )
            .unwrap(),
    );
    assert!(index.local_pending);
    assert_eq!(row(), before);
    release_tx.send(()).unwrap();
    let (_, _, index) = saved(worker.join().unwrap().unwrap());
    assert!(!index.local_pending);
}

#[test]
fn comment_publication_retry_without_a_checkpoint_or_binding_never_accepts_a_replacement() {
    let f = Fixture::new();
    let request = f.request();
    let id = request.comment.target.operation_id;
    let retry = RetryCommentPublicationRequest {
        root: f.root.path().into(),
        operation_id: id,
        approval: None,
        confirmed_identity: None,
        restart: false,
    };
    assert!(
        matches!(f.service.retry_comment_publication(retry.clone(),&mut SessionCredentials::new(NoPrompt)),Err(e) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeCheckpointCommit,
    )
    .unwrap();
    assert!(
        failing
            .submit_comment(request.clone(), &mut SessionCredentials::new(NoPrompt))
            .is_err()
    );
    let path = f.context.worktree.join(format!(
        ".manyhands/comments/{}/{}.md",
        f.item, request.comment.comment_id
    ));
    let bytes = std::fs::read(&path).unwrap();
    let head = Repository::open(&f.context.worktree)
        .unwrap()
        .head()
        .unwrap()
        .target();
    assert!(
        matches!(f.service.retry_comment_publication(retry,&mut SessionCredentials::new(NoPrompt)),Err(e) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert_eq!(
        Repository::open(&f.context.worktree)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        head
    );
}

#[test]
fn comment_publication_cancel_before_unused_child_has_no_effect() {
    let f = Fixture::new();
    let (receipt, _, _) = saved(
        f.service
            .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
            .unwrap(),
    );
    assert!(
        !f.service
            .cancel_comment_publication(f.root.path(), receipt.operation_id)
            .unwrap()
    );
    assert_eq!(
        f.db()
            .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    assert_eq!(
        Repository::open(&f.context.worktree)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        Some(receipt.checkpoint_oid)
    );
}

#[test]
fn comment_publication_recorded_receipt_refuses_reset_removed_or_replaced_comment_without_transport()
 {
    for change in [0, 1, 2] {
        let f = Fixture::new();
        let (receipt, _, _) = saved(
            f.service
                .submit_comment(f.request(), &mut SessionCredentials::new(NoPrompt))
                .unwrap(),
        );
        let repository = Repository::open(&f.context.worktree).unwrap();
        let original = repository.find_commit(receipt.checkpoint_oid).unwrap();
        let changed = if change == 0 {
            original.parent_id(0).unwrap()
        } else {
            let mut index = git2::Index::new().unwrap();
            index.read_tree(&original.tree().unwrap()).unwrap();
            if change == 1 {
                index.remove_path(&receipt.comment_path).unwrap();
            } else {
                let mut comment = match canonical::parse_item(
                    &receipt.comment_path,
                    &std::fs::read_to_string(f.context.worktree.join(&receipt.comment_path))
                        .unwrap(),
                )
                .unwrap()
                {
                    canonical::CanonicalItem::Comment(c) => c,
                    _ => panic!("original comment"),
                };
                comment.body = "replacement-body-canary\n".into();
                let bytes =
                    canonical::serialize_item(&canonical::CanonicalItem::Comment(comment)).unwrap();
                index
                    .add(&git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode: 0o100644,
                        uid: 0,
                        gid: 0,
                        file_size: bytes.len() as u32,
                        id: repository.blob(bytes.as_bytes()).unwrap(),
                        flags: 0,
                        flags_extended: 0,
                        path: receipt.comment_path.to_str().unwrap().as_bytes().to_vec(),
                    })
                    .unwrap();
            }
            let tree = repository
                .find_tree(index.write_tree_to(&repository).unwrap())
                .unwrap();
            let signature = repository.signature().unwrap();
            repository
                .commit(
                    None,
                    &signature,
                    &signature,
                    "external change",
                    &tree,
                    &[&original],
                )
                .unwrap()
        };
        repository
            .find_reference(&format!("refs/heads/{}", receipt.context_branch))
            .unwrap()
            .set_target(changed, "external change")
            .unwrap();
        repository
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        let (again, state, _) = saved(
            f.service
                .retry_comment_publication(
                    RetryCommentPublicationRequest {
                        root: f.root.path().into(),
                        operation_id: receipt.operation_id,
                        approval: None,
                        confirmed_identity: None,
                        restart: false,
                    },
                    &mut SessionCredentials::new(NoPrompt),
                )
                .unwrap(),
        );
        assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
        assert!(
            matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::RecoveryRequired))
        );
        assert_eq!(repository.head().unwrap().target(), Some(changed));
        assert_eq!(
            f.db()
                .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
    }
}
