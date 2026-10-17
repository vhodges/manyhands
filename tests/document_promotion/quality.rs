//! Quality recheck regressions use the real public service and deterministic gates.
use super::*;

fn image_probes() -> Result<Vec<Vec<u8>>, FixtureError> {
    let probes = vec![b"image-control-private-sentinel".to_vec()];
    ssh_privacy::save(&probes)?;
    Ok(probes)
}
pub(super) fn image_boundaries() -> Result<(), FixtureError> {
    let probes = image_probes()?;
    let a = fixed(tempfile::tempdir())?;
    let b = fixed(tempfile::tempdir())?;
    fixed(std::fs::write(a.path().join("a"), b"bc"))?;
    fixed(std::fs::write(b.path().join("ab"), b"c"))?;
    // Same unframed byte stream, different path/payload boundaries.
    assert_ne!(image(a.path())?, image(b.path())?);
    fixed(std::fs::remove_file(b.path().join("ab")))?;
    fixed(std::fs::write(b.path().join("a"), b"bc"))?;
    assert_eq!(image(a.path())?, image(b.path())?);
    ssh_privacy::scan(a.path(), &probes)?;
    ssh_privacy::scan(b.path(), &probes)?;
    Ok(())
}
#[cfg(unix)]
pub(super) fn image_types() -> Result<(), FixtureError> {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let probes = image_probes()?;
    let directory = fixed(tempfile::tempdir())?;
    let file = directory.path().join("entry");
    fixed(std::fs::write(&file, b"payload"))?;
    fixed(std::fs::set_permissions(
        &file,
        std::fs::Permissions::from_mode(0o644),
    ))?;
    let regular = image(directory.path())?;
    fixed(std::fs::remove_file(&file))?;
    fixed(symlink("payload", &file))?;
    assert_ne!(image(directory.path())?, regular);
    fixed(std::fs::remove_file(&file))?;
    fixed(std::fs::write(&file, b"payload"))?;
    fixed(std::fs::set_permissions(
        &file,
        std::fs::Permissions::from_mode(0o644),
    ))?;
    assert_eq!(image(directory.path())?, regular);
    fixed(std::fs::set_permissions(
        &file,
        std::fs::Permissions::from_mode(0o755),
    ))?;
    assert_ne!(image(directory.path())?, regular);
    fixed(std::fs::set_permissions(
        &file,
        std::fs::Permissions::from_mode(0o644),
    ))?;
    assert_eq!(image(directory.path())?, regular);
    ssh_privacy::scan(directory.path(), &probes)?;
    Ok(())
}

type OwnerRow = (i64, String, String, i64, Option<String>);
fn indexing_row(side: &Side, operation: OperationId) -> Result<OwnerRow, FixtureError> {
    assert_eq!(
        fixed(side.db()?.query_row(
            "SELECT COUNT(*) FROM operation_records WHERE state='indexing' AND operation_ulid=?1",
            [operation.to_string()],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    fixed(side.db()?.query_row("SELECT id,operation_ulid,state,index_owner_epoch,completed_step FROM operation_records WHERE state='indexing' AND operation_ulid=?1", [operation.to_string()],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))))
}
fn owner_row(side: &Side, id: i64) -> Result<OwnerRow, FixtureError> {
    fixed(side.db()?.query_row("SELECT id,operation_ulid,state,index_owner_epoch,completed_step FROM operation_records WHERE id=?1", [id],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))))
}
pub(super) fn refresh_f11(compound: bool) -> Result<(), FixtureError> {
    refresh_f11_inner(compound, false)
}
pub(super) fn refresh_takeover() -> Result<(), FixtureError> {
    refresh_f11_inner(false, true)
}
fn refresh_f11_inner(compound: bool, takeover: bool) -> Result<(), FixtureError> {
    use std::sync::mpsc;
    use std::time::Duration;
    let p = Pair::new()?;
    p.save(&p.a)?;
    p.promote()?;
    poll(&p, &p.a)?;
    fixed(p.a.service.prepare_context(p.a.target()))?;
    assert_eq!(
        p.preview()?.integration(),
        PlannedIntegration::AlreadyContained
    );
    let request = p.request()?;
    stopped(
        &p,
        &request,
        if compound {
            FailurePoint::AfterIntegrationRef
        } else {
            FailurePoint::AfterPublication
        },
    )?;
    let older = fixed(RepositoryService::open_at(&p.a.data))?;
    // The foreign root deliberately shares A's registry: an unscoped fence
    // would invalidate it too, even though its own filesystem is independent.
    let foreign = Side {
        service: fixed(RepositoryService::open_at(&p.a.data))?,
        root: p.b.root.clone(),
        data: p.a.data.clone(),
    };
    fixed(foreign.service.enable(EnableRepositoryRequest {
        root: foreign.root.clone(),
        primary_branch: "main".into(),
        identity: None,
        operation_id: OperationId::new(),
    }))?;
    let ordinary_id = OperationId::new();
    let foreign_id = OperationId::new();
    let (observed, held) = mpsc::sync_channel(0);
    let (release, resume) = mpsc::sync_channel(0);
    older.set_refresh_persist_hook_for_testing(move || {
        observed.send(()).unwrap();
        resume.recv_timeout(Duration::from_secs(30)).unwrap();
    });
    let (b_observed, b_held) = mpsc::sync_channel(0);
    let (b_release, b_resume) = mpsc::sync_channel(0);
    foreign
        .service
        .set_refresh_persist_hook_for_testing(move || {
            b_observed.send(()).unwrap();
            b_resume.recv_timeout(Duration::from_secs(30)).unwrap();
        });
    let receipts = p.server.receive_updates();
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let worker=scope.spawn(|| -> Result<bool,FixtureError> {
            if compound {
                let r=fixed(p.execute(&older,request.clone()))?;
                Ok(matches!(r,DocumentPromotionResult::Complete(ref o) if o.cleanup()==PromotionCleanup::Complete))
            } else {
                Ok(matches!(fixed(older.refresh_repository(RefreshRepositoryRequest{root:p.a.root.clone(),operation_id:ordinary_id}))?,RefreshOutcome::Refreshed{..}))
            }
        });
        let b_worker = scope.spawn(|| {
            foreign
                .service
                .refresh_repository(RefreshRepositoryRequest {
                    root: foreign.root.clone(),
                    operation_id: foreign_id,
                })
        });
        let captures = (|| -> Result<(OwnerRow, OwnerRow), FixtureError> {
            fixed(held.recv_timeout(Duration::from_secs(30)))?;
            fixed(b_held.recv_timeout(Duration::from_secs(30)))?;
            let id = if compound {
                let child: String = fixed(p.a.db()?.query_row(
                    "SELECT primary_sync_id FROM document_promotions WHERE operation_id=?1",
                    [request.operation_id.to_string()],
                    |r| r.get(0),
                ))?;
                fixed(OperationId::parse(&child))?
            } else {
                ordinary_id
            };
            Ok((indexing_row(&p.a, id)?, indexing_row(&foreign, foreign_id)?))
        })();
        let completed = (|| -> Result<(OwnerRow, OwnerRow, i64), FixtureError> {
            let (a, b) = captures.as_ref().map_err(|_| FixtureError)?;
            exits::launch(&p, &request, 10)?; // Actual F11 process exit after SQL completion.
            assert_eq!(step(&p.a, request.operation_id)?, "complete");
            let captured=(owner_row(&p.a,a.0)?,owner_row(&foreign,b.0)?,fixed(p.a.db()?.query_row("SELECT refresh_required FROM repositories WHERE root_path=(SELECT root_path FROM operation_records WHERE id=?1)",[a.0],|r|r.get(0)))?);
            if takeover {
                // Another executor of this ORIGINAL refresh may claim the fence
                // first. The paused token must not adopt its completed record.
                let reclaimer = fixed(RepositoryService::open_at(&p.a.data))?;
                assert!(matches!(
                    fixed(reclaimer.refresh_repository(RefreshRepositoryRequest {
                        root: p.a.root.clone(),
                        operation_id: fixed(OperationId::parse(&a.1))?,
                    }))?,
                    RefreshOutcome::Refreshed { .. }
                ));
            }
            Ok(captured)
        })();
        // Always release/join both genuine owners before examining assertions.
        let _ = release.send(());
        let _ = b_release.send(());
        let old = fixed(worker.join())?;
        let other = fixed(b_worker.join())?;
        let (before, before_b) = captures?;
        let (fenced, untouched, stale) = completed?;
        assert_eq!(old?, !takeover);
        assert!(matches!(fixed(other)?, RefreshOutcome::Refreshed { .. }));
        assert_eq!(stale, 1);
        let after = owner_row(&p.a, before.0)?;
        ssh_harness::observation(&[
            2100,
            compound as u128,
            before.3 as u128,
            fenced.3 as u128,
            after.3 as u128,
        ]);
        // The stale owner must not commit its obsolete two-context observations.
        discovery_primary_once(&p)?;
        assert_eq!(fenced.2, "index_invalidated");
        assert_eq!(fenced.3, before.3 + 1);
        assert_eq!(
            (fenced.0, fenced.1, fenced.4),
            (before.0, before.1.clone(), before.4.clone())
        );
        assert_eq!(after.1, before.1);
        assert_eq!(after.2, "completed");
        assert_eq!(after.3, before.3 + 2);
        assert_eq!(untouched, before_b);
        assert_eq!(owner_row(&foreign, before_b.0)?.3, before_b.3);
        if compound {
            assert_eq!(
                fixed(p.a.db()?.query_row(
                    "SELECT index_pending FROM remote_operation_records WHERE operation_ulid=?1",
                    [after.1],
                    |r| r.get::<_, i64>(0)
                ))?,
                0
            );
        }
        Ok(())
    })?;
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    discovery_primary_once(&p)?;
    assert!(matches!(
        fixed(p.a.service.refresh_repository(RefreshRepositoryRequest {
            root: p.a.root.clone(),
            operation_id: OperationId::new()
        }))?,
        RefreshOutcome::Refreshed { .. }
    ));
    p.server.wait_for_helper_audits()?;
    assert_eq!(p.server.receive_updates(), receipts);
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}

struct Stop(RemoteOperationSafePoint);
impl SynchronizationObserver for Stop {
    fn safe_point(&mut self, point: RemoteOperationSafePoint) -> SafePointRequest {
        if point == self.0 {
            SafePointRequest::Stop
        } else {
            SafePointRequest::Continue
        }
    }
}
fn stop_sync(
    p: &Pair,
    request: &SynchronizeRemoteRequest,
    point: RemoteOperationSafePoint,
) -> Result<(), FixtureError> {
    let result = p.a.service.synchronize_remote_observed(
        request.clone(),
        &mut SessionCredentials::new(remote_world::Provider),
        SynchronizationSettings::default(),
        &mut Stop(point),
    );
    p.diagnostics(&format!("{result:?}"))?;
    assert!(matches!(
        result,
        Err(SynchronizationError::RecoveryRequired)
    ));
    Ok(())
}
fn callbacks<'a>(p: &'a Pair, server: &'a SshRemoteFixture) -> git2::RemoteCallbacks<'a> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_, user, _| {
        git2::Cred::ssh_key(
            user.unwrap_or("fixture"),
            None,
            p.server.client_key_path(),
            Some(remote_world::PASSWORD),
        )
    });
    let key = server.host_public_key().to_bytes().unwrap();
    callbacks.certificate_check(move |certificate, _| {
        if certificate.as_hostkey().and_then(|h| h.hostkey()) == Some(key.as_slice()) {
            Ok(git2::CertificateCheckStatus::CertificateOk)
        } else {
            Err(git2::Error::from_str("fixture host mismatch"))
        }
    });
    callbacks
}
fn push_peer_to(
    p: &Pair,
    server: &SshRemoteFixture,
    oid: git2::Oid,
    destination: &str,
) -> Result<(), FixtureError> {
    let repo = p.a.repo()?;
    fixed(repo.reference("refs/heads/quality-peer", oid, true, "owned fixture peer"))?;
    let receiver = fixed(git2::Repository::open_bare(server.repository_path()))?;
    let old = match receiver.refname_to_id(destination) {
        Ok(id) => id,
        Err(e) if e.code() == git2::ErrorCode::NotFound => git2::Oid::zero(),
        Err(_) => return Err(FixtureError),
    };
    let cursor = server.receive_updates().len();
    let mut remote = fixed(repo.remote_anonymous(&server.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks(p, server));
    fixed(remote.push(
        &[&format!("refs/heads/quality-peer:{destination}")],
        Some(&mut options),
    ))?;
    server.fixture_push_transport_returned()?;
    server.wait_for_receive_update(
        cursor,
        &ssh_remote::ReceiveUpdate {
            reference: destination.into(),
            old_oid: old,
            new_oid: oid,
            accepted: true,
        },
    )
}
pub(super) fn verified_routes(mode: usize) -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    p.save(&p.a)?;
    fixed(p.sync(&p.a))?;
    lose_registry(&mut p)?;
    unknown(&p.a)?;
    fixed(p.sync(&p.a))?;
    p.promote()?;
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    unknown(&p.a)?;
    let push = SshRemoteFixture::start()?;
    push.allow_client_public_key(fixed(russh::keys::ssh_key::PublicKey::from_bytes(
        &p.server.allowed_client_public_key(),
    ))?);
    p.probes.push(push.url().into_bytes());
    p.probes.push(fixed(std::fs::read(push.client_key_path()))?);
    ssh_privacy::save(&p.probes)?;
    fixed(p.a.repo()?.remote_set_pushurl("origin", Some(&push.url())))?;
    fixed(p.a.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: p.a.root.clone(),
            direction: SshDirection::Push,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: push.address().port(),
                },
                expected: None,
                presented: push.host_identity(),
            }),
        },
        &mut SessionCredentials::new(remote_world::Provider),
    ))?;
    let candidate = p.save_body(&p.a, &(BODY.to_owned() + "quality verified route\n"))?;
    let mut request = SynchronizeRemoteRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
        target: document_target(),
        restart: false,
        confirmed_identity: None,
        approval: None,
    };
    stop_sync(
        &p,
        &request,
        if mode == 0 {
            RemoteOperationSafePoint::AfterPushReturn
        } else {
            RemoteOperationSafePoint::BeforePush
        },
    )?;
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    assert!(server_oid(&p, CONTEXT).is_err()); // Fetch cannot consume the permission.
    request.restart = true;
    let final_candidate = if mode == 0 {
        candidate
    } else {
        let repo = p.a.repo()?;
        let peer = remote_world::commit(&repo, candidate, "quality-peer", b"peer\n")?;
        push_peer_to(&p, &push, peer, CONTEXT)?;
        let incoming = if mode == 1 {
            peer
        } else {
            remote_world::commit(&repo, peer, "quality-primary", b"primary\n")?
        };
        push_peer_to(&p, &p.server, incoming, MAIN)?;
        if mode == 2 {
            stop_sync(&p, &request, RemoteOperationSafePoint::AfterPushReturn)?;
            assert_eq!(count(&p.a, "retired_contexts")?, 1);
            assert_eq!(
                fixed(p.a.db()?.query_row(
                    "SELECT phase FROM remote_publication_attempts ORDER BY number DESC LIMIT 1",
                    [],
                    |r| r.get::<_, String>(0)
                ))?,
                "returned"
            );
        }
        incoming
    };
    let receipts = push.receive_updates();
    // Reject permission consumption to prove verification and consumption share
    // one transaction: no verified checkpoint/attempt may commit on rollback.
    fixed(p.a.db()?.execute_batch("CREATE TRIGGER quality_consumption_failure BEFORE DELETE ON retired_contexts BEGIN SELECT RAISE(ABORT,'fixed verification rollback'); END;"))?;
    let failed = p.a.service.synchronize_remote_observed(
        request.clone(),
        &mut SessionCredentials::new(remote_world::Provider),
        SynchronizationSettings::default(),
        &mut Stop(RemoteOperationSafePoint::AfterPushVerification),
    );
    p.diagnostics(&format!("{failed:?}"))?;
    assert!(
        matches!(failed,Err(SynchronizationError::Repository(ref e)) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    if mode == 0 {
        assert_eq!(
            fixed(p.a.db()?.query_row(
                "SELECT sync_checkpoint FROM remote_operation_records WHERE operation_ulid=?1",
                [request.operation_id.to_string()],
                |r| r.get::<_, String>(0)
            ))?,
            "push_returned"
        );
    } else {
        assert_eq!(
            fixed(p.a.db()?.query_row(
                "SELECT phase FROM remote_publication_attempts ORDER BY number DESC LIMIT 1",
                [],
                |r| r.get::<_, String>(0)
            ))?,
            if mode == 1 { "open" } else { "returned" }
        );
    }
    fixed(
        p.a.db()?
            .execute_batch("DROP TRIGGER quality_consumption_failure"),
    )?;
    stop_sync(
        &p,
        &request,
        RemoteOperationSafePoint::AfterPushVerification,
    )?;
    assert_eq!(push.receive_updates(), receipts); // Reconciliation/already-current never push again.
    assert_eq!(
        fixed(git2::Repository::open_bare(push.repository_path()))?
            .refname_to_id(CONTEXT)
            .ok(),
        Some(final_candidate)
    );
    assert_eq!(fixed(p.a.db()?.query_row("SELECT COUNT(*) FROM remote_operation_records WHERE operation_ulid=?1 AND authoritative_kind IS NOT NULL",[request.operation_id.to_string()],|r|r.get::<_,i64>(0)))?,0);
    assert_eq!(count(&p.a, "retired_contexts")?, 0);
    let snapshot = fixed(p.a.service.remote_snapshot(&p.a.root))?;
    assert_eq!(
        snapshot.publication_evidence_for(AuthoringKind::Document, &DOC.parse().unwrap()),
        RemotePublicationEvidence::ObservedPublished
    );
    let projected = snapshot
        .contexts()
        .iter()
        .find(|c| c.item_id().is_some_and(|id| id.to_string() == DOC))
        .ok_or(FixtureError)?;
    assert_eq!(
        projected.publication_evidence(),
        RemotePublicationEvidence::ObservedPublished
    );
    assert_eq!(projected.advertised_oid(), None);
    assert_eq!(projected.tracking_oid(), None);
    assert_eq!(projected.state(), RemoteContextState::HistoryUnknown);
    assert!(server_oid(&p, CONTEXT).is_err());
    unknown(&p.a)?;
    if mode > 0 {
        assert_eq!(
            fixed(p.a.db()?.query_row(
                "SELECT phase FROM remote_publication_attempts ORDER BY number DESC LIMIT 1",
                [],
                |r| r.get::<_, String>(0)
            ))?,
            "verified"
        );
    }
    let result = p.a.service.synchronize_remote(
        request,
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{result:?}"))?;
    assert!(matches!(fixed(result)?, SynchronizationResult::Complete(_)));
    assert_eq!(push.receive_updates(), receipts);
    p.privacy()
}

pub(super) fn projection_index() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let before = (
        count(&p.a, "operation_records")?,
        count(&p.a, "remote_operation_records")?,
        oid(&p.a, MAIN)?,
    );
    // Simulate the immediately preceding schema, preserving all business rows.
    fixed(p.a.db()?.execute_batch("DROP INDEX remote_context_publication_evidence; DELETE FROM registry_migrations WHERE name='cycle_09_context_publication_evidence';"))?;
    fixed(RepositoryService::open_at(&p.a.data))?;
    fixed(RepositoryService::open_at(&p.a.data))?;
    assert_eq!(
        (
            count(&p.a, "operation_records")?,
            count(&p.a, "remote_operation_records")?,
            oid(&p.a, MAIN)?
        ),
        before
    );
    assert_eq!(fixed(p.a.db()?.query_row("SELECT COUNT(*) FROM registry_migrations WHERE name='cycle_09_context_publication_evidence'",[],|r|r.get::<_,i64>(0)))?,1);
    fixed(
        p.a.db()?
            .execute_batch("DROP INDEX remote_context_publication_evidence"),
    )?;
    let error = match RepositoryService::open_at(&p.a.data) {
        Err(error) => error,
        Ok(_) => panic!("marked index loss must require recovery"),
    };
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    p.diagnostics(&format!("{error:?} {error}"))?;
    assert_eq!(
        (
            count(&p.a, "operation_records")?,
            count(&p.a, "remote_operation_records")?
        ),
        (before.0, before.1)
    );
    p.privacy()
}

struct HoldVerified {
    held: std::sync::mpsc::SyncSender<()>,
    resume: std::sync::mpsc::Receiver<()>,
}
impl SynchronizationObserver for HoldVerified {
    fn safe_point(&mut self, point: RemoteOperationSafePoint) -> SafePointRequest {
        if point == RemoteOperationSafePoint::AfterPushVerification {
            self.held.send(()).unwrap();
            self.resume
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap();
        }
        SafePointRequest::Continue
    }
}
pub(super) fn late_classification() -> Result<(), FixtureError> {
    use std::sync::mpsc;
    use std::time::Duration;
    let p = Pair::new()?;
    p.save(&p.a)?;
    p.promote()?;
    poll(&p, &p.a)?;
    fixed(p.a.service.prepare_context(p.a.target()))?;
    let cleanup = p.request()?;
    stopped(&p, &cleanup, FailurePoint::AfterPublication)?;
    let old = fixed(RepositoryService::open_at(&p.a.data))?;
    let synchronization = SynchronizeRemoteRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
        target: document_target(),
        restart: false,
        approval: Some(p.approval()),
        confirmed_identity: None,
    };
    let (signal, held) = mpsc::sync_channel(0);
    let (release, resume) = mpsc::sync_channel(0);
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let worker = scope.spawn(|| {
            old.synchronize_remote_observed(
                synchronization,
                &mut SessionCredentials::new(remote_world::Provider),
                SynchronizationSettings::default(),
                &mut HoldVerified {
                    held: signal,
                    resume,
                },
            )
        });
        let completed = (|| -> Result<(), FixtureError> {
            fixed(held.recv_timeout(Duration::from_secs(30)))?;
            assert_eq!(count(&p.a, "retired_contexts")?, 0); // consumed by the OLD verification.
            peer_delete(&p)?;
            exits::launch(&p, &cleanup, 10)?; // NEW confirmed cleanup earns a new row.
            assert_eq!(count(&p.a, "retired_contexts")?, 1);
            Ok(())
        })();
        let _ = release.send(());
        let result = fixed(worker.join())?;
        p.diagnostics(&format!("{result:?}"))?;
        completed?;
        assert!(matches!(fixed(result)?, SynchronizationResult::Complete(_)));
        // Classification is NOT a new publication observation. It must not
        // consume retirement earned after its already-committed verification.
        assert_eq!(count(&p.a, "retired_contexts")?, 1);
        assert_eq!(
            fixed(
                p.a.db()?
                    .query_row("SELECT operation_id FROM retired_contexts", [], |r| r
                        .get::<_, String>(0))
            )?,
            cleanup.operation_id.to_string()
        );
        Ok(())
    })?;
    assert_eq!(run(&p, &cleanup)?.cleanup(), PromotionCleanup::Complete);
    let next = p.save_body(&p.a, &(BODY.to_owned() + "new earned permission\n"))?;
    assert_eq!(published(fixed(p.sync(&p.a))?)?, next);
    assert_eq!(count(&p.a, "retired_contexts")?, 0);
    p.privacy()
}
