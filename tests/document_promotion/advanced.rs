use super::*;

struct HookProvider<'a>(Option<Box<dyn FnOnce() + 'a>>);
impl SessionCredentialProvider for HookProvider<'_> {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        if let Some(hook) = self.0.take() {
            hook();
        }
        PassphraseResponse::Supplied(SecretPassphrase::new(remote_world::PASSWORD.into()).unwrap())
    }
}
fn child(
    side: &Side,
    request: &PromoteDocumentRequest,
    primary: bool,
) -> Result<(OperationId, i64), FixtureError> {
    let sql = if primary {
        "SELECT primary_sync_id,primary_attempt FROM document_promotions WHERE operation_id=?1"
    } else {
        "SELECT context_sync_id,context_attempt FROM document_promotions WHERE operation_id=?1"
    };
    let (id, attempt): (String, i64) = fixed(side.db()?.query_row(
        sql,
        [request.operation_id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    Ok((fixed(OperationId::parse(&id))?, attempt))
}
pub(super) fn cancelled(primary: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    p.save(&p.a)?;
    let request = p.request()?;
    if primary {
        stopped(&p, &request, FailurePoint::AfterIntegrationRef)?;
    }
    let old = std::cell::Cell::new(None);
    let hook = || {
        let (id, attempt) = child(&p.a, &request, primary).unwrap();
        assert_eq!(attempt, 1);
        old.set(Some(id));
        p.a.service.cancel_remote_operation(&p.a.root, id).unwrap();
    };
    let result = p.a.service.promote_document(
        request.clone(),
        &mut SessionCredentials::new(HookProvider(Some(Box::new(hook)))),
    );
    p.diagnostics(&format!("{result:?}"))?;
    let old = old.get().ok_or(FixtureError)?;
    if primary {
        let outcome = authoritative(fixed(result)?);
        assert_eq!(
            outcome.publication(),
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Cancelled
            }
        );
    } else {
        assert!(
            matches!(result,Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::Interrupted))
        );
    }
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    let (new, attempt) = child(&p.a, &request, primary)?;
    assert_ne!(new, old);
    assert_eq!(attempt, 2);
    let updates = p.server.receive_updates();
    let accepted: Vec<_> = updates[cursor..].iter().filter(|u| u.accepted).collect();
    assert_eq!(accepted.len(), 3);
    assert_eq!(accepted.iter().filter(|u| u.reference == MAIN).count(), 1);
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
pub(super) fn retained(primary: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = if primary {
        let request = p.request()?;
        stopped(&p, &request, FailurePoint::AfterIntegrationRef)?;
        let source = fixed(canonical::serialize_item(
            &canonical::CanonicalItem::Document(canonical::Document {
                id: DOC.parse().unwrap(),
                title: TITLE.into(),
                body: BODY.to_owned() + "peer primary\n",
                unknown: serde_yaml::Mapping::new(),
            }),
        ))?;
        commit_files(&p.b.repo()?, &[(DOCUMENT_PATH, source.as_bytes())])?;
        fixed(p.sync_target(&p.b, SynchronizationTarget::Primary))?;
        let o = run(&p, &request)?;
        assert_eq!(
            o.publication(),
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Synchronization(
                    PromotionSynchronizationCategory::Conflict
                )
            }
        );
        request
    } else {
        fixed(p.sync(&p.a))?;
        poll(&p, &p.b)?;
        p.save_body(&p.a, &(BODY.to_owned() + "owned local\n"))?;
        p.save_body(&p.b, &(BODY.to_owned() + "owned peer\n"))?;
        fixed(p.sync(&p.b))?;
        let request = p.request()?;
        assert!(
            matches!(p.execute(&p.a.service,request.clone()),Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::ConflictPending{..}))
        );
        request
    };
    let (id, attempt) = child(&p.a, &request, primary)?;
    assert_eq!(attempt, 1);
    let worktree = if primary {
        p.a.root.clone()
    } else {
        p.a.worktree()
    };
    let before = image(&worktree)?;
    // Cancel at the reacquired child's durable epoch boundary, after the early
    // replay checks. This is the actual T0-1 SQL-trigger fault seam, not an
    // injected synchronization result or fabricated integration record.
    fixed(p.a.db()?.execute_batch(&format!("CREATE TABLE remote_cancel_marks(value INTEGER); CREATE TRIGGER cancel_retained AFTER UPDATE OF owner_epoch ON remote_operation_records WHEN NEW.operation_ulid='{id}' AND NEW.owner_epoch>OLD.owner_epoch BEGIN INSERT INTO remote_cancel_marks VALUES(1); UPDATE remote_operation_records SET cancel_requested=1 WHERE id=NEW.id; END;")))?;
    let interrupted = p.execute(&p.a.service, request.clone());
    p.diagnostics(&format!("{interrupted:?}"))?;
    fixed(p.a.db()?.execute_batch("DROP TRIGGER cancel_retained"))?;
    assert_eq!(count(&p.a, "remote_cancel_marks")?, 1);
    if primary {
        assert!(matches!(
            fixed(interrupted)?.clone(),
            DocumentPromotionResult::Complete(_) | DocumentPromotionResult::IndexPending(_)
        ));
    } else {
        assert!(
            matches!(interrupted,Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::Interrupted))
        );
    }
    assert_eq!(child(&p.a, &request, primary)?, (id, 1));
    assert_eq!(
        fixed(p.a.db()?.query_row(
            "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0)
        ))?,
        "interrupted"
    );
    // Worktree/index/merge evidence survives the actual cancelled child; the
    // primary case's root also contains unrelated administrative refresh writes.
    if !primary {
        assert_eq!(image(&worktree)?, before);
    }
    resolve_local(&p, id)?;
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(child(&p.a, &request, primary)?, (id, 1));
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
fn poll_request(p: &Pair) -> PollPublicationRemoteRequest {
    PollPublicationRemoteRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
        invocation: RemotePollInvocation::Automatic,
        approval: Some(p.approval()),
        restart: false,
    }
}
pub(super) fn priority() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    fixed(p.a.service.set_remote_polling(
        &p.a.root,
        true,
        false,
        PollingInterval::from_seconds(120).unwrap(),
    ))?;
    let mut session = SessionCredentials::new(remote_world::Provider);
    fixed(p.a.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: p.a.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(p.approval()),
        },
        &mut session,
    ))?;
    let second = fixed(RepositoryService::open_at(&p.a.data))?;
    let gate = p.server.hold_advertisement()?;
    let req = poll_request(&p);
    let cursor = p.server.receive_updates().len();
    let primary = oid(&p.a, MAIN)?;
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let worker = scope.spawn(|| p.a.service.poll_publication_remote(req, &mut session));
        gate.wait_until_held()?;
        let result = p.execute(&second, request.clone());
        assert!(
            matches!(result,Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::PollYielding))
        );
        assert_eq!(oid(&p.a, MAIN)?, primary);
        gate.release()?;
        let result = fixed(worker.join())?;
        p.diagnostics(&format!("{result:?}"))?;
        assert!(
            matches!(result, Err(RemotePollError::Interrupted))
                || matches!(result,Ok(RemotePollResult::Complete(RemotePollOutcome::Polled(ref r))) if r.completion()==PollCompletion::Interrupted)
        );
        Ok(())
    })?;
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(
        p.server.receive_updates()[cursor..]
            .iter()
            .filter(|u| u.accepted)
            .count(),
        3
    );
    p.privacy()
}
pub(super) fn between_publication_and_f6() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    stopped(&p, &request, FailurePoint::AfterPublication)?;
    let context = image(&p.a.worktree())?;
    let primary = oid(&p.a, MAIN)?;
    let updates = p.server.receive_updates();
    poll(&p, &p.a)?;
    assert_eq!(image(&p.a.worktree())?, context);
    assert_eq!(oid(&p.a, MAIN)?, primary);
    assert_eq!(p.server.receive_updates(), updates);
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
pub(super) fn busy(point: FailurePoint) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    fixed(p.a.service.set_remote_polling(
        &p.a.root,
        true,
        false,
        PollingInterval::from_seconds(120).unwrap(),
    ))?;
    stopped(&p, &request, point)?;
    let before = all_images(&p)?;
    let updates = p.server.receive_updates();
    let connections = p.server.accepted_sockets();
    let result = p.a.service.poll_publication_remote(
        poll_request(&p),
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{result:?}"))?;
    assert!(matches!(result, Err(RemotePollError::Busy)));
    assert_eq!(p.server.accepted_sockets(), connections);
    assert_eq!(all_images(&p)?, before);
    assert_eq!(p.server.receive_updates(), updates);
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    let r = poll(&p, &p.a)?;
    assert!(
        !r.targets()
            .iter()
            .any(|r| r.item_id().is_some_and(|id| id.to_string() == DOC))
    );
    assert!(!p.a.worktree().exists());
    p.privacy()
}
pub(super) fn configuration(integrated: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    stopped(
        &p,
        &request,
        if integrated {
            FailurePoint::AfterPublication
        } else {
            FailurePoint::AfterContextSync
        },
    )?;
    let repo = p.a.repo()?;
    fixed(repo.remote_rename("origin", "renamed"))?;
    let source = fixed(canonical::serialize_repository_config(
        &canonical::RepositoryConfig {
            primary_branch: "main".into(),
            publication_remote: Some("renamed".into()),
            unknown: toml::Table::new(),
        },
    ))?;
    commit_files(&repo, &[(".manyhands/config.toml", source.as_bytes())])?;
    let before = all_images(&p)?;
    let updates = p.server.receive_updates();
    let connections = p.server.accepted_sockets();
    let result = p.execute(&p.a.service, request);
    if integrated {
        let o = authoritative(fixed(result)?);
        assert_eq!(
            o.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RecoveryRequired
            }
        );
    } else {
        assert!(matches!(result, Err(DocumentPromotionError::Interrupted)));
    }
    assert_eq!(p.server.accepted_sockets(), connections);
    assert_eq!(p.server.receive_updates(), updates);
    assert_eq!(all_images(&p)?, before);
    p.privacy()
}

struct StopAt(RemoteOperationSafePoint);
impl SynchronizationObserver for StopAt {
    fn safe_point(&mut self, point: RemoteOperationSafePoint) -> SafePointRequest {
        if point == self.0 {
            SafePointRequest::Stop
        } else {
            SafePointRequest::Continue
        }
    }
}
pub(super) fn retired_push_guard(verified: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    p.promote()?;
    p.save_body(&p.a, &(BODY.to_owned() + "new retirement generation\n"))?;
    let mut request = SynchronizeRemoteRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
        target: document_target(),
        restart: false,
        confirmed_identity: None,
        approval: Some(p.approval()),
    };
    let point = if verified {
        RemoteOperationSafePoint::AfterPushVerification
    } else {
        RemoteOperationSafePoint::BeforePush
    };
    let result = p.a.service.synchronize_remote_observed(
        request.clone(),
        &mut SessionCredentials::new(remote_world::Provider),
        SynchronizationSettings::default(),
        &mut StopAt(point),
    );
    p.diagnostics(&format!("{result:?}"))?;
    assert!(matches!(
        result,
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(count(&p.a, "retired_contexts")?, i64::from(!verified));
    if verified {
        peer_delete(&p)?;
    }
    fixed(
        p.a.service
            .cancel_remote_operation(&p.a.root, request.operation_id),
    )?;
    request.restart = true;
    let ack = p.a.service.synchronize_remote(
        request,
        &mut SessionCredentials::new(remote_world::Provider),
    );
    assert!(matches!(
        ack,
        Err(SynchronizationError::Interrupted | SynchronizationError::RecoveryRequired)
    ));
    let receives = p.server.receive_updates();
    let result = p.sync(&p.a);
    if verified {
        assert!(matches!(
            result,
            Err(SynchronizationError::RemoteContextDeleted)
        ));
    } else {
        assert!(matches!(
            result,
            Err(SynchronizationError::RecoveryRequired)
        ));
    }
    assert_eq!(p.server.receive_updates(), receives);
    p.privacy()
}
