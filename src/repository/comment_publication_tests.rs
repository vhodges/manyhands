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
    f.db().execute_batch("CREATE TRIGGER stop_receipt BEFORE UPDATE ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
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
