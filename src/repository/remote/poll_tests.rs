use super::super::{
    AutomaticBackoff, PollingInterval, RemoteOutcomeCategory, RemotePollInvocation,
    RemotePollingConfiguration,
};
use super::super::{RemoteRefPlan, SynchronizationError, SynchronizationTarget};
use super::*;
use crate::repository::AuthoringKind;
use crate::repository::{REGISTRY_FILE, RepositoryService};
use rusqlite::{Connection, params};
use std::{collections::BTreeMap, fs, path::PathBuf};
use time::OffsetDateTime;

struct PanicCredentials;

fn review_restart_gate(reason: PollIneligibility) {
    for interrupted in [false, true] {
        let f = PollFixture::new();
        if interrupted {
            f.db()
                .execute("UPDATE remote_operation_records SET yield_requested=1", [])
                .unwrap();
            assert_eq!(
                f.local
                    .service
                    .remote_safe_point(
                        &f.local.root,
                        &f.owner,
                        super::super::RemoteOperationSafePoint::BeforeTransport
                    )
                    .unwrap(),
                super::super::RemoteSafePointOutcome::Interrupted
            );
        }
        let (enabled, paused, suspended, next) = match reason {
            PollIneligibility::Disabled => (false, false, false, None),
            PollIneligibility::Paused => (true, true, false, None),
            PollIneligibility::RecoverySuspended => (true, false, true, None),
            PollIneligibility::NotDue => (
                true,
                false,
                false,
                Some(OffsetDateTime::now_utc().unix_timestamp() + 60),
            ),
        };
        f.db().execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=?4,automatic_backoff_seconds=120,latest_outcome='completed',latest_attempt_at=123",params![enabled,paused,suspended,next]).unwrap();
        let before = f.local.service.remote_snapshot(&f.local.root).unwrap();
        let owned_before: (String, i64, String) = f
            .db()
            .query_row(
                "SELECT phase,owner_epoch,coalesce(outcome,'') FROM remote_operation_records",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        f.db().execute_batch("CREATE TABLE review_writes(n INTEGER);INSERT INTO review_writes VALUES(0);CREATE TRIGGER review_operation_write AFTER UPDATE ON remote_operation_records BEGIN UPDATE review_writes SET n=n+1;END;CREATE TRIGGER review_policy_write AFTER UPDATE ON remote_polling_state BEGIN UPDATE review_writes SET n=n+1;END;").unwrap();
        let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
        let result = f.local.service.poll_publication_remote(
            PollPublicationRemoteRequest {
                root: f.local.root.clone(),
                operation_id: f.owner.operation_id(),
                invocation: RemotePollInvocation::Automatic,
                approval: None,
                restart: true,
            },
            &mut session,
        );
        assert!(
            matches!(result,Ok(RemotePollResult::Complete(RemotePollOutcome::NotEligible{reason:r,next_eligible_at})) if r==reason && next_eligible_at==next.map(at)),
            "{interrupted}: {result:?}"
        );
        assert_eq!(
            f.local.service.remote_snapshot(&f.local.root).unwrap(),
            before
        );
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT phase,owner_epoch,coalesce(outcome,'') FROM remote_operation_records",
                    [],
                    |r| Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?
                    ))
                )
                .unwrap(),
            owned_before
        );
        assert_eq!(
            f.db()
                .query_row("SELECT n FROM review_writes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn review_restart_auto_gate_disabled() {
    review_restart_gate(PollIneligibility::Disabled);
}
#[test]
fn review_restart_auto_gate_paused() {
    review_restart_gate(PollIneligibility::Paused);
}
#[test]
fn review_restart_auto_gate_recovery_suspended() {
    review_restart_gate(PollIneligibility::RecoverySuspended);
}
#[test]
fn review_restart_auto_gate_not_due() {
    review_restart_gate(PollIneligibility::NotDue);
}

#[test]
fn review_terminal_replay_precedes_removed_or_changed_live_configuration() {
    for change in [
        "publication_removed",
        "configuration_removed",
        "remote_changed",
        "primary_changed",
    ] {
        let f = PollFixture::new();
        f.local
            .tracking(&SynchronizationTarget::Primary, f.local.old);
        let (original, _) = report(
            f.run(vec![("refs/heads/main".into(), f.local.old)])
                .unwrap(),
        );
        let config = f.local.root.join(".manyhands/config.toml");
        match change {
            "publication_removed" => {
                fs::write(&config, "format_version = 1\nprimary_branch = \"main\"\n").unwrap()
            }
            "configuration_removed" => fs::remove_file(&config).unwrap(),
            "remote_changed" => fs::write(
                &config,
                "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"other\"\n",
            )
            .unwrap(),
            "primary_changed" => fs::write(
                &config,
                "format_version = 1\nprimary_branch = \"other\"\npublication_remote = \"origin\"\n",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        f.db().execute_batch("CREATE TABLE review_writes(n INTEGER);INSERT INTO review_writes VALUES(0);CREATE TRIGGER review_operation_write AFTER UPDATE ON remote_operation_records BEGIN UPDATE review_writes SET n=n+1;END;CREATE TRIGGER review_policy_write AFTER UPDATE ON remote_polling_state BEGIN UPDATE review_writes SET n=n+1;END;").unwrap();
        let before = state::with_transaction(&f.local.service, &f.local.root, |tx, id| {
            state::read_snapshot(tx, id)
        })
        .unwrap();
        let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
        for restart in [false, true] {
            let (replay, pending) = report(
                f.local
                    .service
                    .poll_publication_remote(
                        PollPublicationRemoteRequest {
                            root: f.local.root.clone(),
                            operation_id: f.owner.operation_id(),
                            invocation: RemotePollInvocation::Automatic,
                            approval: None,
                            restart,
                        },
                        &mut session,
                    )
                    .unwrap(),
            );
            assert_eq!(replay, original, "{change}");
            assert!(!pending);
        }
        assert_eq!(
            state::with_transaction(&f.local.service, &f.local.root, |tx, id| {
                state::read_snapshot(tx, id)
            })
            .unwrap(),
            before
        );
        assert_eq!(
            f.db()
                .query_row("SELECT n FROM review_writes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(
            matches!(f.local.service.poll_publication_remote(PollPublicationRemoteRequest{root:f.local.root.clone(),operation_id:f.owner.operation_id(),invocation:RemotePollInvocation::Explicit,approval:None,restart:false},&mut session),Err(RemotePollError::Repository(e)) if e.kind==crate::repository::RepositoryErrorKind::OperationMismatch)
        );
    }
}

#[test]
fn review_original_report_survives_second_poll_before_refresh_return() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    for failure in [
        None,
        Some(crate::repository::FailurePoint::BeforeIndexTransactionCommit),
    ] {
        let f = PollFixture::with_failure(failure);
        let incoming = f.local.incoming(f.local.old, None);
        f.local.tracking(&SynchronizationTarget::Primary, incoming);
        let root = f.local.root.clone();
        let data = f.local._data.path().to_owned();
        let plan = f.local.plan.clone();
        let called = Arc::new(AtomicBool::new(false));
        let fired = called.clone();
        f.local
            .service
            .set_poll_boundary_hook_for_testing(move |point, _| {
                if point == crate::repository::FailurePoint::PollAfterCompletion
                    && !fired.swap(true, Ordering::SeqCst)
                {
                    let service = RepositoryService::open_at(&data).unwrap();
                    let configuration = service.observation_configuration(&root, &plan).unwrap();
                    let owner = match service
                        .reserve_remote_poll(
                            &root,
                            crate::repository::OperationId::new(),
                            &super::super::RemoteOperationTarget::for_poll(&plan),
                            super::super::RemoteOperationPriority::Manual,
                        )
                        .unwrap()
                    {
                        super::super::reservation::RemotePollReservationOutcome::Reservation(
                            super::super::RemoteReservationOutcome::Reserved(o),
                        ) => o,
                        other => panic!("{other:?}"),
                    };
                    service
                        .remote_safe_point(
                            &root,
                            &owner,
                            super::super::RemoteOperationSafePoint::BeforeTransport,
                        )
                        .unwrap();
                    let (second, pending) = report(
                        service
                            .poll_after_fetch(
                                &root,
                                &owner,
                                &plan,
                                &configuration,
                                vec![("refs/heads/main".into(), incoming)],
                            )
                            .unwrap(),
                    );
                    assert!(!pending);
                    assert_eq!(second.targets()[0].disposition(), PollDisposition::Current);
                }
            });
        let (original, pending) =
            report(f.run(vec![("refs/heads/main".into(), incoming)]).unwrap());
        assert!(
            called.load(Ordering::SeqCst),
            "post-completion fixture boundary not reached"
        );
        assert_eq!(pending, failure.is_some());
        assert_eq!(
            original.targets().len(),
            1,
            "original response lost its targets"
        );
        assert_eq!(
            original.targets()[0].disposition(),
            PollDisposition::FastForwarded
        );
        assert_eq!(original.targets()[0].before_oid(), Some(f.local.old));
        assert_eq!(original.targets()[0].after_oid(), Some(incoming));
        let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
        let (older, _) = report(
            f.local
                .service
                .poll_publication_remote(
                    PollPublicationRemoteRequest {
                        root: f.local.root.clone(),
                        operation_id: f.owner.operation_id(),
                        invocation: RemotePollInvocation::Automatic,
                        approval: None,
                        restart: false,
                    },
                    &mut session,
                )
                .unwrap(),
        );
        assert!(older.targets().is_empty());
        assert_eq!(older.completed_at(), original.completed_at());
        assert_eq!(older.next_eligible_at(), original.next_eligible_at());
    }
}

#[test]
fn review_original_stop_report_survives_later_poll_and_index_repair() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    for cancelled in [false, true] {
        let f = PollFixture::new();
        let (target, _) = f.local.context();
        let incoming = f.local.incoming(f.local.old, None);
        f.local.tracking(&SynchronizationTarget::Primary, incoming);
        f.local.tracking(&target, incoming);
        let context = f
            .local
            .plan
            .context(AuthoringKind::Document, &POLL_ID.parse().unwrap());
        let advertised = vec![
            ("refs/heads/main".into(), incoming),
            (context.remote_ref().into(), incoming),
        ];
        f.db().execute_batch(&format!("CREATE TRIGGER review_stop AFTER UPDATE ON remote_poll_dispositions WHEN NEW.ordinal=0 AND NEW.disposition='fast_forwarded' BEGIN UPDATE remote_operation_records SET {}=1 WHERE operation_ulid=NEW.operation_ulid;END;",if cancelled{"cancel_requested"}else{"yield_requested"})).unwrap();
        let root = f.local.root.clone();
        let data = f.local._data.path().to_owned();
        let plan = f.local.plan.clone();
        let second_advertised = advertised.clone();
        let called = Arc::new(AtomicBool::new(false));
        let fired = called.clone();
        f.local
            .service
            .set_poll_boundary_hook_for_testing(move |point, _| {
                if point == crate::repository::FailurePoint::PollAfterCompletion
                    && !fired.swap(true, Ordering::SeqCst)
                {
                    let service = RepositoryService::open_at(&data).unwrap();
                    let configuration = service.observation_configuration(&root, &plan).unwrap();
                    let owner = match service
                        .reserve_remote_poll(
                            &root,
                            crate::repository::OperationId::new(),
                            &super::super::RemoteOperationTarget::for_poll(&plan),
                            super::super::RemoteOperationPriority::Manual,
                        )
                        .unwrap()
                    {
                        super::super::reservation::RemotePollReservationOutcome::Reservation(
                            super::super::RemoteReservationOutcome::Reserved(o),
                        ) => o,
                        other => panic!("{other:?}"),
                    };
                    service
                        .remote_safe_point(
                            &root,
                            &owner,
                            super::super::RemoteOperationSafePoint::BeforeTransport,
                        )
                        .unwrap();
                    let (second, pending) = report(
                        service
                            .poll_after_fetch(
                                &root,
                                &owner,
                                &plan,
                                &configuration,
                                second_advertised.clone(),
                            )
                            .unwrap(),
                    );
                    assert!(!pending);
                    assert_eq!(second.targets()[0].disposition(), PollDisposition::Current);
                    assert_eq!(
                        second.targets()[1].disposition(),
                        PollDisposition::FastForwarded
                    );
                }
            });
        let (original, pending) = report(f.run(advertised).unwrap());
        assert!(called.load(Ordering::SeqCst));
        assert!(pending);
        assert_eq!(
            original.completion(),
            if cancelled {
                PollCompletion::Cancelled
            } else {
                PollCompletion::Interrupted
            }
        );
        assert_eq!(
            original
                .targets()
                .iter()
                .map(|t| t.disposition())
                .collect::<Vec<_>>(),
            [PollDisposition::FastForwarded, PollDisposition::NotReached]
        );
        assert_eq!(original.targets()[0].before_oid(), Some(f.local.old));
        assert_eq!(original.targets()[0].after_oid(), Some(incoming));
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM operation_records", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1,
            "only the later poll scans"
        );
        assert!(
            !f.db()
                .query_row("SELECT refresh_required FROM repositories", [], |r| r
                    .get::<_, bool>(0))
                .unwrap(),
            "later poll repaired the index but original stop remains IndexPending"
        );
    }
}

#[test]
fn review_original_fenced_stop_keeps_pending_after_later_poll_repairs_index() {
    let f = PollFixture::new();
    let (target, _) = f.local.context();
    let incoming = f.local.incoming(f.local.old, None);
    f.local.tracking(&SynchronizationTarget::Primary, incoming);
    f.local.tracking(&target, incoming);
    let context = f
        .local
        .plan
        .context(AuthoringKind::Document, &POLL_ID.parse().unwrap());
    let advertised = vec![
        ("refs/heads/main".into(), incoming),
        (context.remote_ref().into(), incoming),
    ];
    let root = f.local.root.clone();
    let data = f.local._data.path().to_owned();
    let plan = f.local.plan.clone();
    let second_advertised = advertised.clone();
    f.local
        .service
        .set_poll_boundary_hook_for_testing(move |point, ordinal| {
            if point == crate::repository::FailurePoint::PollBeforeTarget && ordinal == Some(1) {
                let service = RepositoryService::open_at(&data).unwrap();
                // A real configuration-generation fence ends the first owner before
                // its next safe point. A later explicit local poll may now proceed.
                state::with_transaction(&service, &root, |tx, id| {
                    state::configure(tx, id, Some(&plan), true)
                })
                .unwrap();
                let configuration = service.observation_configuration(&root, &plan).unwrap();
                let owner = match service
                    .reserve_remote_poll(
                        &root,
                        crate::repository::OperationId::new(),
                        &super::super::RemoteOperationTarget::for_poll(&plan),
                        super::super::RemoteOperationPriority::Manual,
                    )
                    .unwrap()
                {
                    super::super::reservation::RemotePollReservationOutcome::Reservation(
                        super::super::RemoteReservationOutcome::Reserved(o),
                    ) => o,
                    other => panic!("{other:?}"),
                };
                service
                    .remote_safe_point(
                        &root,
                        &owner,
                        super::super::RemoteOperationSafePoint::BeforeTransport,
                    )
                    .unwrap();
                let (second, pending) = report(
                    service
                        .poll_after_fetch(
                            &root,
                            &owner,
                            &plan,
                            &configuration,
                            second_advertised.clone(),
                        )
                        .unwrap(),
                );
                assert!(!pending);
                assert_eq!(
                    second.targets()[1].disposition(),
                    PollDisposition::FastForwarded
                );
            }
        });
    let (original, pending) = report(f.run(advertised).unwrap());
    assert_eq!(original.completion(), PollCompletion::Interrupted);
    assert_eq!(
        original
            .targets()
            .iter()
            .map(|t| t.disposition())
            .collect::<Vec<_>>(),
        [PollDisposition::FastForwarded, PollDisposition::NotReached]
    );
    assert!(
        pending,
        "original fenced stop must retain its own index handoff state"
    );
    assert!(
        !f.db()
            .query_row("SELECT refresh_required FROM repositories", [], |r| r
                .get::<_, bool>(0))
            .unwrap()
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM operation_records", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1,
        "only the later poll scanned"
    );
}

#[test]
fn review_explicit_restart_preserves_manual_identity_and_bypasses_policy_gates() {
    let f = PollFixture::new();
    end_fixture_reservation(&f);
    let op = crate::repository::OperationId::new();
    let owner = match f
        .local
        .service
        .reserve_remote_poll(
            &f.local.root,
            op,
            &super::super::RemoteOperationTarget::for_poll(&f.local.plan),
            super::super::RemoteOperationPriority::Manual,
        )
        .unwrap()
    {
        super::super::reservation::RemotePollReservationOutcome::Reservation(
            super::super::RemoteReservationOutcome::Reserved(o),
        ) => o,
        other => panic!("{other:?}"),
    };
    f.local
        .service
        .remote_safe_point(
            &f.local.root,
            &owner,
            super::super::RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    f.db().execute_batch("UPDATE remote_polling_state SET enabled=0,paused=1,recovery_suspended=1,next_eligible_at=12345,automatic_backoff_seconds=120").unwrap();
    let before = f
        .local
        .service
        .remote_snapshot(&f.local.root)
        .unwrap()
        .polling()
        .clone();
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    assert!(matches!(
        f.local.service.poll_publication_remote(
            PollPublicationRemoteRequest {
                root: f.local.root.clone(),
                operation_id: op,
                invocation: RemotePollInvocation::Explicit,
                approval: None,
                restart: true
            },
            &mut session
        ),
        Err(RemotePollError::Transport(_))
    ));
    let after = f.local.service.remote_snapshot(&f.local.root).unwrap();
    assert_eq!(after.polling(), &before);
    assert_eq!(after.next_eligible_at(), Some(at(12345)));
    assert_eq!(
        f.db()
            .query_row(
                "SELECT owner_epoch FROM remote_operation_records WHERE operation_ulid=?1",
                [op.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn review_terminal_other_kind_rejects_before_missing_live_configuration() {
    let f = PollFixture::new();
    end_fixture_reservation(&f);
    let op = crate::repository::OperationId::new();
    let owner = match f
        .local
        .service
        .reserve_remote_operation(
            &f.local.root,
            op,
            &super::super::RemoteOperationTarget::for_poll(&f.local.plan),
        )
        .unwrap()
    {
        super::super::RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    f.local
        .service
        .remote_safe_point(
            &f.local.root,
            &owner,
            super::super::RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    f.local
        .service
        .finish_remote_operation(
            &f.local.root,
            &owner,
            RemoteOutcomeCategory::TransportUnavailable,
        )
        .unwrap();
    fs::remove_file(f.local.root.join(".manyhands/config.toml")).unwrap();
    let before = state::with_transaction(&f.local.service, &f.local.root, |tx, id| {
        state::read_snapshot(tx, id)
    })
    .unwrap();
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    assert!(
        matches!(f.local.service.poll_publication_remote(PollPublicationRemoteRequest{root:f.local.root.clone(),operation_id:op,invocation:RemotePollInvocation::Automatic,approval:None,restart:true},&mut session),Err(RemotePollError::Repository(e)) if e.kind==crate::repository::RepositoryErrorKind::OperationMismatch)
    );
    assert_eq!(
        state::with_transaction(&f.local.service, &f.local.root, |tx, id| {
            state::read_snapshot(tx, id)
        })
        .unwrap(),
        before
    );
}
impl crate::repository::keys::SessionCredentialProvider for PanicCredentials {
    fn request_passphrase(
        &mut self,
        _: &crate::repository::keys::UnlockRequest,
    ) -> crate::repository::keys::PassphraseResponse {
        panic!("gate or replay must not request credentials")
    }
}

struct PollFixture {
    local: LocalFixture,
    owner: super::super::RemoteReservation,
    configuration: super::super::observation::ObservationConfiguration,
}
impl PollFixture {
    fn new() -> Self {
        Self::with_failure(None)
    }
    fn with_failure(failure: Option<crate::repository::FailurePoint>) -> Self {
        let mut local = LocalFixture::new();
        if let Some(failure) = failure {
            local.service = RepositoryService::open_at_with_failure_point_for_testing(
                local._data.path(),
                failure,
            )
            .unwrap();
        }
        let db = Connection::open(local._data.path().join(REGISTRY_FILE)).unwrap();
        db.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)", [local.root.to_str().unwrap()]).unwrap();
        let configuration = local
            .service
            .observation_configuration(&local.root, &local.plan)
            .unwrap();
        state::with_transaction(&local.service, &local.root, |tx, id| {
            state::configure_endpoints(tx, id, &local.plan, &configuration.endpoint_digest())
        })
        .unwrap();
        let owner = match local
            .service
            .reserve_remote_poll(
                &local.root,
                crate::repository::OperationId::new(),
                &super::super::RemoteOperationTarget::for_poll(&local.plan),
                super::super::RemoteOperationPriority::Poll,
            )
            .unwrap()
        {
            super::super::reservation::RemotePollReservationOutcome::Reservation(
                super::super::RemoteReservationOutcome::Reserved(o),
            ) => o,
            other => panic!("{other:?}"),
        };
        local
            .service
            .remote_safe_point(
                &local.root,
                &owner,
                super::super::RemoteOperationSafePoint::BeforeTransport,
            )
            .unwrap();
        Self {
            local,
            owner,
            configuration,
        }
    }
    fn db(&self) -> Connection {
        Connection::open(self.local._data.path().join(REGISTRY_FILE)).unwrap()
    }
    fn run(
        &self,
        advertised: Vec<(String, git2::Oid)>,
    ) -> Result<RemotePollResult, RemotePollError> {
        self.local.service.poll_after_fetch(
            &self.local.root,
            &self.owner,
            &self.local.plan,
            &self.configuration,
            advertised,
        )
    }
}
fn report(result: RemotePollResult) -> (RemotePollReport, bool) {
    let (outcome, pending) = match result {
        RemotePollResult::Complete(o) => (o, false),
        RemotePollResult::IndexPending(p) => (p.authoritative, true),
    };
    match outcome {
        RemotePollOutcome::Polled(r) => (r, pending),
        other => panic!("{other:?}"),
    }
}

#[test]
fn task6_atomic_batch_extension_rolls_back_and_rejects_foreign_owner() {
    let f = PollFixture::new();
    let s = &f.local.service;
    for p in [
        super::super::RemoteOperationSafePoint::AfterAdvertisement,
        super::super::RemoteOperationSafePoint::BeforeBatchCommit,
    ] {
        s.remote_safe_point(&f.local.root, &f.owner, p).unwrap();
    }
    let obs = super::super::RemoteRefObservation::from_advertisement(
        &f.local.plan,
        "refs/heads/main",
        f.local.old,
        Some(f.local.old),
    )
    .unwrap();
    let mut reached = false;
    let error = super::super::reservation::commit_observation_batch_with(
        s,
        &f.local.root,
        &f.owner,
        &f.local.plan,
        std::slice::from_ref(&obs),
        123,
        |tx, id| {
            reached = true;
            assert_eq!(
                tx.query_row("SELECT count(*) FROM remote_ref_observations", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            tx.execute(
                "UPDATE repositories SET refresh_required=1 WHERE id=?1",
                [id],
            )
            .unwrap();
            Err(state::recovery_required())
        },
    )
    .unwrap_err();
    assert!(reached);
    assert_eq!(
        error.kind,
        crate::repository::RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(
        !f.db()
            .query_row("SELECT refresh_required FROM repositories", [], |r| r
                .get::<_, bool>(0))
            .unwrap()
    );
    assert_eq!(
        s.active_remote_operation(&f.local.root)
            .unwrap()
            .unwrap()
            .completed_step(),
        Some(super::super::RemoteOperationSafePoint::BeforeBatchCommit)
    );
    let other = RepositoryService::open_at(f.local._data.path()).unwrap();
    assert!(
        super::super::reservation::commit_observation_batch_with(
            &other,
            &f.local.root,
            &f.owner,
            &f.local.plan,
            &[obs],
            124,
            |_, _| panic!("foreign owner callback")
        )
        .is_err()
    );
}

#[test]
fn task6_postfetch_advances_primary_and_replays_without_transport() {
    let f = PollFixture::new();
    let incoming = f.local.incoming(f.local.old, None);
    f.local.tracking(&SynchronizationTarget::Primary, incoming);
    let (r, pending) = report(f.run(vec![("refs/heads/main".into(), incoming)]).unwrap());
    assert!(!pending);
    assert_eq!(r.completion(), PollCompletion::Completed);
    assert_eq!(r.targets()[0].disposition(), PollDisposition::FastForwarded);
    assert_eq!(r.targets()[0].before_oid(), Some(f.local.old));
    assert_eq!(r.targets()[0].after_oid(), Some(incoming));
    assert_eq!(f.local.repository.head().unwrap().target(), Some(incoming));
    assert_eq!(fs::read(f.local.root.join("file")).unwrap(), b"new\n");
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    let (replayed, _) = report(
        f.local
            .service
            .poll_publication_remote(
                PollPublicationRemoteRequest {
                    root: f.local.root.clone(),
                    operation_id: f.owner.operation_id(),
                    invocation: RemotePollInvocation::Automatic,
                    approval: None,
                    restart: false,
                },
                &mut session,
            )
            .unwrap(),
    );
    assert_eq!(replayed, r);
}

#[test]
fn task6_automatic_gates_do_not_reserve_or_change_outcome() {
    for (enabled, paused, suspended, next, reason) in [
        (false, false, false, None, PollIneligibility::Disabled),
        (true, true, false, None, PollIneligibility::Paused),
        (
            true,
            false,
            true,
            None,
            PollIneligibility::RecoverySuspended,
        ),
        (
            true,
            false,
            false,
            Some(OffsetDateTime::now_utc().unix_timestamp() + 60),
            PollIneligibility::NotDue,
        ),
    ] {
        let (_data, root, s, db) = fixture();
        fs::create_dir(root.path().join(".manyhands")).unwrap();
        fs::write(
            root.path().join(".manyhands/config.toml"),
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
        )
        .unwrap();
        git2::Repository::open(root.path())
            .unwrap()
            .remote("origin", "ssh://fixture@example.invalid/unreachable")
            .unwrap();
        db.execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=?4",params![enabled,paused,suspended,next]).unwrap();
        db.execute("UPDATE eligibility_writes SET n=0", []).unwrap();
        let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
        let result = s
            .poll_publication_remote(
                PollPublicationRemoteRequest {
                    root: root.path().into(),
                    operation_id: crate::repository::OperationId::new(),
                    invocation: RemotePollInvocation::Automatic,
                    approval: None,
                    restart: false,
                },
                &mut session,
            )
            .unwrap();
        assert!(
            matches!(result,RemotePollResult::Complete(RemotePollOutcome::NotEligible{reason:r,..}) if r==reason)
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT n FROM eligibility_writes", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn task6_postfetch_configuration_fence_preserves_git_and_previous_batch() {
    let f = PollFixture::new();
    let incoming = f.local.incoming(f.local.old, None);
    f.local.tracking(&SynchronizationTarget::Primary, incoming);
    f.local
        .repository
        .remote_set_url("origin", "ssh://different@example.invalid/repo")
        .unwrap();
    let before = snapshot(&f.local.root, &f.local.repository);
    assert!(matches!(
        f.run(vec![("refs/heads/main".into(), incoming)]),
        Err(RemotePollError::Interrupted)
    ));
    assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn task6_cancel_after_batch_returns_report_without_scan() {
    let f = PollFixture::new();
    // A registry trigger requests cancellation in the atomic batch transaction.
    f.db().execute_batch("CREATE TRIGGER stop_poll AFTER INSERT ON remote_ref_observations BEGIN UPDATE remote_operation_records SET cancel_requested=1 WHERE action='poll'; END;").unwrap();
    f.local
        .tracking(&SynchronizationTarget::Primary, f.local.old);
    let (r, pending) = report(
        f.run(vec![("refs/heads/main".into(), f.local.old)])
            .unwrap(),
    );
    assert!(!pending);
    assert_eq!(r.completion(), PollCompletion::Cancelled);
    assert_eq!(r.targets()[0].disposition(), PollDisposition::NotReached);
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM operation_records", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn task6_stop_matrix_at_each_boundary_preserves_completed_targets_and_deadlines() {
    for cancelled in [false, true] {
        for point in [
            "after_advertisement",
            "between_observations",
            "before_batch_commit",
            "after_batch_commit",
            "before_local_mutation",
            "before_discovery",
        ] {
            let f = PollFixture::new();
            let incoming = f.local.incoming(f.local.old, None);
            f.local.tracking(&SynchronizationTarget::Primary, incoming);
            f.db().execute_batch(&format!("UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=12345;CREATE TRIGGER request_stop AFTER UPDATE OF completed_step ON remote_operation_records WHEN NEW.completed_step='{point}' BEGIN UPDATE remote_operation_records SET {}=1 WHERE id=NEW.id; END;",if cancelled{"cancel_requested"}else{"yield_requested"})).unwrap();
            let result = f.run(vec![("refs/heads/main".into(), incoming)]);
            let has_batch = matches!(
                point,
                "after_batch_commit" | "before_local_mutation" | "before_discovery"
            );
            if !has_batch && !cancelled {
                assert!(
                    matches!(result, Err(RemotePollError::Interrupted)),
                    "{point}: {result:?}"
                );
            } else {
                let (r, pending) = report(result.unwrap());
                assert_eq!(
                    r.completion(),
                    if cancelled {
                        PollCompletion::Cancelled
                    } else {
                        PollCompletion::Interrupted
                    },
                    "{point}"
                );
                if has_batch {
                    assert_eq!(r.targets().len(), 1);
                    assert_eq!(
                        r.targets()[0].disposition(),
                        if point == "before_discovery" {
                            PollDisposition::FastForwarded
                        } else {
                            PollDisposition::NotReached
                        }
                    );
                }
                assert_eq!(pending, point == "before_discovery");
            }
            assert_eq!(
                f.local.repository.head().unwrap().target(),
                Some(if point == "before_discovery" {
                    incoming
                } else {
                    f.local.old
                })
            );
            assert_eq!(
                f.db()
                    .query_row("SELECT count(*) FROM operation_records", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(f.db().query_row("SELECT automatic_backoff_seconds,next_eligible_at FROM remote_polling_state",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?))).unwrap(),(120,12345));
        }
    }
}

#[test]
fn task6_ordered_existing_new_malformed_targets_materialize_and_snapshot_after_refresh() {
    let f = PollFixture::new();
    let privacy_db = f.db();
    assert_eq!(
        privacy_db
            .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    let (existing, linked) = f.local.context();
    let incoming = f.local.incoming(f.local.old, None);
    f.local.tracking(&existing, incoming);
    let id: canonical::ItemId = OTHER_ID.parse().unwrap();
    let mut source = item_source(AuthoringKind::Ticket, OTHER_ID);
    source.extend_from_slice(b"\nPRIVATE_POLL_BODY_SENTINEL\n");
    let path = format!(".manyhands/tickets/{OTHER_ID}/ticket.md");
    let tree = f
        .local
        .repository
        .find_tree(fixture_tree(
            &f.local.repository,
            &[(&path, source, 0o100644)],
        ))
        .unwrap();
    let sig =
        git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(1, 0)).unwrap();
    let oid = f
        .local
        .repository
        .commit(
            None,
            &sig,
            &sig,
            "ticket",
            &tree,
            &[&f.local.repository.find_commit(f.local.old).unwrap()],
        )
        .unwrap();
    let target = f.local.plan.context(AuthoringKind::Ticket, &id);
    f.local
        .repository
        .reference(target.tracking_ref(), oid, false, "installed tracking")
        .unwrap();
    f.local.tracking(&SynchronizationTarget::Primary, incoming);
    let malformed = "refs/heads/manyhands/document/private-marker/nested";
    let (r, pending) = report(
        f.run(vec![
            (target.remote_ref().into(), oid),
            (malformed.into(), oid),
            ("refs/heads/unrelated".into(), oid),
            ("refs/heads/main".into(), incoming),
            (
                f.local
                    .plan
                    .context(AuthoringKind::Document, &POLL_ID.parse().unwrap())
                    .remote_ref()
                    .into(),
                incoming,
            ),
        ])
        .unwrap(),
    );
    assert!(!pending);
    assert_eq!(
        r.targets()
            .iter()
            .map(|t| t.disposition())
            .collect::<Vec<_>>(),
        [
            PollDisposition::FastForwarded,
            PollDisposition::FastForwarded,
            PollDisposition::Materialized,
            PollDisposition::Malformed
        ]
    );
    assert_eq!(linked.head().unwrap().target(), Some(incoming));
    let snapshot = f.local.service.remote_snapshot(&f.local.root).unwrap();
    assert_eq!(
        snapshot.primary_disposition(),
        Some(PollDisposition::FastForwarded)
    );
    let ticket = snapshot
        .contexts()
        .iter()
        .find(|c| c.item_id() == Some(&id))
        .unwrap();
    assert_eq!(ticket.state(), super::super::RemoteContextState::Observed);
    assert_eq!(
        ticket.latest_disposition(),
        Some(PollDisposition::Materialized)
    );
    assert_eq!(snapshot.latest_attempt_at(), Some(r.completed_at()));
    assert_eq!(snapshot.next_eligible_at(), r.next_eligible_at());
    assert!(!format!("{r:?}").contains("private-marker"));
    let backup = f.local._data.path().join("b.db");
    privacy_db
        .execute("VACUUM INTO ?1", [backup.to_str().unwrap()])
        .unwrap();
    assert!(
        f.local
            ._data
            .path()
            .join(format!("{REGISTRY_FILE}-wal"))
            .exists()
    );
    for name in [REGISTRY_FILE, "manyhands.sqlite3-wal", "b.db"] {
        if let Ok(bytes) = fs::read(f.local._data.path().join(name)) {
            for probe in [
                b"PRIVATE_POLL_BODY_SENTINEL".as_slice(),
                b"private-marker".as_slice(),
                b"ssh://fixture@example.invalid/repo".as_slice(),
            ] {
                assert!(
                    !bytes.windows(probe.len()).any(|w| w == probe),
                    "private poll payload in {name}"
                );
            }
        }
    }
    assert!(RepositoryService::open_at(f.local._data.path()).is_ok());
}

#[test]
fn task6_yield_between_targets_keeps_changed_primary_and_restart_is_current() {
    let f = PollFixture::new();
    let (target, linked) = f.local.context();
    let incoming = f.local.incoming(f.local.old, None);
    f.local.tracking(&SynchronizationTarget::Primary, incoming);
    f.local.tracking(&target, incoming);
    f.db().execute_batch("CREATE TRIGGER yield_after_primary AFTER UPDATE ON remote_poll_dispositions WHEN NEW.ordinal=0 AND NEW.disposition='fast_forwarded' BEGIN UPDATE remote_operation_records SET yield_requested=1 WHERE operation_ulid=NEW.operation_ulid; END;").unwrap();
    let context = f
        .local
        .plan
        .context(AuthoringKind::Document, &POLL_ID.parse().unwrap());
    let advertised = vec![
        ("refs/heads/main".into(), incoming),
        (context.remote_ref().into(), incoming),
    ];
    let (r, pending) = report(f.run(advertised.clone()).unwrap());
    assert!(pending);
    assert_eq!(r.completion(), PollCompletion::Interrupted);
    assert_eq!(
        r.targets()
            .iter()
            .map(|t| t.disposition())
            .collect::<Vec<_>>(),
        [PollDisposition::FastForwarded, PollDisposition::NotReached]
    );
    assert_eq!(linked.head().unwrap().target(), Some(f.local.old));
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM operation_records", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    assert!(matches!(
        f.local.service.poll_publication_remote(
            PollPublicationRemoteRequest {
                root: f.local.root.clone(),
                operation_id: f.owner.operation_id(),
                invocation: RemotePollInvocation::Automatic,
                approval: None,
                restart: false
            },
            &mut session
        ),
        Err(RemotePollError::Interrupted)
    ));
    f.db()
        .execute_batch("DROP TRIGGER yield_after_primary")
        .unwrap();
    let owner = match f
        .local
        .service
        .restart_remote_poll(
            &f.local.root,
            f.owner.operation_id(),
            &super::super::RemoteOperationTarget::for_poll(&f.local.plan),
        )
        .unwrap()
    {
        super::super::RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    f.local
        .service
        .remote_safe_point(
            &f.local.root,
            &owner,
            super::super::RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    let (r, pending) = report(
        f.local
            .service
            .poll_after_fetch(
                &f.local.root,
                &owner,
                &f.local.plan,
                &f.configuration,
                advertised,
            )
            .unwrap(),
    );
    assert!(!pending);
    assert_eq!(r.targets()[0].disposition(), PollDisposition::Current);
    assert_eq!(r.targets()[1].disposition(), PollDisposition::FastForwarded);
}

#[test]
fn task6_index_failure_and_crash_leave_repairable_flag_without_poll_id_reuse() {
    for point in [
        crate::repository::FailurePoint::BeforeIndexTransactionCommit,
        crate::repository::FailurePoint::PollAfterCompletion,
    ] {
        let f = PollFixture::with_failure(Some(point));
        let incoming = f.local.incoming(f.local.old, None);
        f.local.tracking(&SynchronizationTarget::Primary, incoming);
        let result = f.run(vec![("refs/heads/main".into(), incoming)]);
        if point == crate::repository::FailurePoint::PollAfterCompletion {
            assert!(
                matches!(result,Err(RemotePollError::Repository(e)) if e.kind==crate::repository::RepositoryErrorKind::InjectedFailure)
            );
        } else {
            assert!(report(result.unwrap()).1);
        }
        assert!(
            f.db()
                .query_row("SELECT refresh_required FROM repositories", [], |r| r
                    .get::<_, bool>(0))
                .unwrap()
        );
        assert_eq!(f.local.repository.head().unwrap().target(), Some(incoming));
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                    [f.owner.operation_id().to_string()],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        let reopened = RepositoryService::open_at(f.local._data.path()).unwrap();
        let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
        let (before, pending) = report(
            reopened
                .poll_publication_remote(
                    PollPublicationRemoteRequest {
                        root: f.local.root.clone(),
                        operation_id: f.owner.operation_id(),
                        invocation: RemotePollInvocation::Automatic,
                        approval: None,
                        restart: false,
                    },
                    &mut session,
                )
                .unwrap(),
        );
        assert!(pending);
        use rusqlite::OptionalExtension;
        // Index-only recovery resumes its ordinary local journal ID, never the poll ID.
        let refresh_id=f.db().query_row("SELECT operation_ulid FROM operation_records WHERE action='refresh' ORDER BY id DESC LIMIT 1",[],|r|r.get::<_,String>(0)).optional().unwrap().map(|id|crate::repository::OperationId::parse(&id).unwrap()).unwrap_or_default();
        assert_ne!(refresh_id, f.owner.operation_id());
        assert!(matches!(
            reopened
                .refresh_repository(crate::repository::RefreshRepositoryRequest {
                    root: f.local.root.clone(),
                    operation_id: refresh_id
                })
                .unwrap(),
            crate::repository::RefreshOutcome::Refreshed { .. }
        ));
        let (after, pending) = report(
            reopened
                .poll_publication_remote(
                    PollPublicationRemoteRequest {
                        root: f.local.root.clone(),
                        operation_id: f.owner.operation_id(),
                        invocation: RemotePollInvocation::Automatic,
                        approval: None,
                        restart: false,
                    },
                    &mut session,
                )
                .unwrap(),
        );
        assert!(!pending);
        assert_eq!(after, before);
    }
}

#[test]
fn task6_batch_completion_once_and_older_replay_keeps_its_times_but_no_targets() {
    let f = PollFixture::new();
    f.local
        .tracking(&SynchronizationTarget::Primary, f.local.old);
    f.db().execute_batch("UPDATE remote_polling_state SET automatic_backoff_seconds=120;CREATE TABLE outcome_count(n INTEGER);INSERT INTO outcome_count VALUES(0);CREATE TRIGGER outcome_once AFTER UPDATE OF latest_outcome ON remote_polling_state BEGIN UPDATE outcome_count SET n=n+1;END;").unwrap();
    let (first, _) = report(
        f.run(vec![("refs/heads/main".into(), f.local.old)])
            .unwrap(),
    );
    assert_eq!(
        f.db()
            .query_row("SELECT n FROM outcome_count", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        first.next_eligible_at(),
        Some(first.completed_at() + time::Duration::seconds(300))
    );
    assert_eq!(
        f.local
            .service
            .remote_snapshot(&f.local.root)
            .unwrap()
            .polling()
            .automatic_backoff(),
        None
    );
    let owner = match f
        .local
        .service
        .reserve_remote_poll(
            &f.local.root,
            crate::repository::OperationId::new(),
            &super::super::RemoteOperationTarget::for_poll(&f.local.plan),
            super::super::RemoteOperationPriority::Poll,
        )
        .unwrap()
    {
        super::super::reservation::RemotePollReservationOutcome::Reservation(
            super::super::RemoteReservationOutcome::Reserved(o),
        ) => o,
        other => panic!("{other:?}"),
    };
    f.local
        .service
        .remote_safe_point(
            &f.local.root,
            &owner,
            super::super::RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    f.local
        .service
        .poll_after_fetch(
            &f.local.root,
            &owner,
            &f.local.plan,
            &f.configuration,
            vec![("refs/heads/main".into(), f.local.old)],
        )
        .unwrap();
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    let (old, _) = report(
        f.local
            .service
            .poll_publication_remote(
                PollPublicationRemoteRequest {
                    root: f.local.root.clone(),
                    operation_id: f.owner.operation_id(),
                    invocation: RemotePollInvocation::Automatic,
                    approval: None,
                    restart: true,
                },
                &mut session,
            )
            .unwrap(),
    );
    assert!(old.targets().is_empty());
    assert_eq!(old.completed_at(), first.completed_at());
    assert_eq!(old.next_eligible_at(), first.next_eligible_at());
    assert_eq!(old.outcome(), Some(RemoteOutcomeCategory::Completed));
}

fn poll_request(f: &PollFixture, invocation: RemotePollInvocation) -> PollPublicationRemoteRequest {
    PollPublicationRemoteRequest {
        root: f.local.root.clone(),
        operation_id: crate::repository::OperationId::new(),
        invocation,
        approval: None,
        restart: false,
    }
}
fn end_fixture_reservation(f: &PollFixture) {
    f.local
        .service
        .cancel_remote_operation(&f.local.root, f.owner.operation_id())
        .unwrap();
    f.local
        .service
        .remote_safe_point(
            &f.local.root,
            &f.owner,
            super::super::RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
}

#[test]
fn task6_missing_remote_and_explicit_bypass_have_no_credentials_or_policy_reset() {
    let f = PollFixture::new();
    end_fixture_reservation(&f);
    fs::write(
        f.local.root.join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\n",
    )
    .unwrap();
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    assert!(
        matches!(f.local.service.poll_publication_remote(poll_request(&f,RemotePollInvocation::Explicit),&mut session),Err(RemotePollError::Transport(e)) if e.kind==crate::repository::transport::SshTransportErrorKind::PublicationRemoteMissing)
    );
    assert_eq!(
        f.local
            .service
            .remote_snapshot(&f.local.root)
            .unwrap()
            .latest_outcome(),
        Some(RemoteOutcomeCategory::ConfigurationRequired)
    );
    for (enabled, paused, suspended, next) in [
        (false, false, false, 12345),
        (true, true, false, 12345),
        (true, false, true, 12345),
        (
            true,
            false,
            false,
            OffsetDateTime::now_utc().unix_timestamp() + 60,
        ),
    ] {
        let f = PollFixture::new();
        end_fixture_reservation(&f);
        f.db().execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=?4,automatic_backoff_seconds=120",params![enabled,paused,suspended,next]).unwrap();
        let before = f
            .local
            .service
            .remote_snapshot(&f.local.root)
            .unwrap()
            .polling()
            .clone();
        let error = f
            .local
            .service
            .poll_publication_remote(
                poll_request(&f, RemotePollInvocation::Explicit),
                &mut session,
            )
            .unwrap_err();
        assert!(matches!(error, RemotePollError::Transport(_)), "{error:?}");
        let after = f.local.service.remote_snapshot(&f.local.root).unwrap();
        assert_eq!(after.polling(), &before);
        assert_eq!(after.next_eligible_at(), Some(at(next)));
        assert!(after.latest_attempt_at().is_some());
    }
}

#[test]
fn task6_busy_yield_local_pending_and_kind_mismatch_gate_before_transport() {
    let f = PollFixture::new();
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    assert!(matches!(
        f.local.service.poll_publication_remote(
            poll_request(&f, RemotePollInvocation::Automatic),
            &mut session
        ),
        Err(RemotePollError::Busy)
    ));
    assert!(matches!(
        f.local.service.poll_publication_remote(
            poll_request(&f, RemotePollInvocation::Explicit),
            &mut session
        ),
        Err(RemotePollError::PollYielding)
    ));
    assert!(
        f.local
            .service
            .active_remote_operation(&f.local.root)
            .unwrap()
            .unwrap()
            .yield_requested()
    );
    assert!(matches!(f.run(vec![]), Err(RemotePollError::Interrupted)));
    let observe_id = crate::repository::OperationId::new();
    let owner = match f
        .local
        .service
        .reserve_remote_operation(
            &f.local.root,
            observe_id,
            &super::super::RemoteOperationTarget::for_poll(&f.local.plan),
        )
        .unwrap()
    {
        super::super::RemoteReservationOutcome::Reserved(o) => o,
        _ => panic!("observe reservation"),
    };
    let mut request = poll_request(&f, RemotePollInvocation::Automatic);
    request.operation_id = observe_id;
    request.restart = true;
    assert!(
        matches!(f.local.service.poll_publication_remote(request,&mut session),Err(RemotePollError::Repository(e)) if e.kind==crate::repository::RepositoryErrorKind::OperationMismatch)
    );
    assert_eq!(
        f.local
            .service
            .active_remote_operation(&f.local.root)
            .unwrap()
            .unwrap()
            .phase(),
        super::super::RemoteOperationPhase::Reserved
    );
    f.local
        .service
        .cancel_remote_operation(&f.local.root, owner.operation_id())
        .unwrap();
    f.local
        .service
        .remote_safe_point(
            &f.local.root,
            &owner,
            super::super::RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    let mut db = f.db();
    let local_id = crate::repository::OperationId::new();
    crate::repository::recovery::begin_or_reconcile_operation(
        &mut db,
        &f.local.root,
        RepositoryOperation::SaveDocument,
        local_id,
        "fixture",
    )
    .unwrap();
    for invocation in [
        RemotePollInvocation::Automatic,
        RemotePollInvocation::Explicit,
    ] {
        // The previous cancelled observation preserves no deadline on this fixture.
        f.db()
            .execute("UPDATE remote_polling_state SET next_eligible_at=NULL", [])
            .unwrap();
        assert!(
            matches!(f.local.service.poll_publication_remote(poll_request(&f,invocation),&mut session),Err(RemotePollError::Repository(e)) if e.kind==crate::repository::RepositoryErrorKind::RecoveryRequired)
        );
    }
    assert_eq!(
        db.query_row(
            "SELECT state FROM operation_records WHERE operation_ulid=?1",
            [local_id.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "created"
    );
}

#[test]
fn task6_pending_conflict_blocks_both_invocations_without_outcome_change() {
    let f = PollFixture::new();
    end_fixture_reservation(&f);
    let target = SynchronizationTarget::Primary.operation_target(&f.local.plan);
    let owner = match f
        .local
        .service
        .reserve_remote_operation(
            &f.local.root,
            crate::repository::OperationId::new(),
            &target,
        )
        .unwrap()
    {
        super::super::RemoteReservationOutcome::Reserved(o) => o,
        other => panic!("{other:?}"),
    };
    let commit = f.local.repository.find_commit(f.local.old).unwrap();
    f.local
        .service
        .prepare_synchronization_integration(
            &f.local.root,
            &owner,
            &state::IntegrationStepIntent {
                ordinal: 0,
                stage: super::super::merge::IntegrationStage::Primary,
                local_oid: commit.id(),
                incoming_oid: commit.id(),
                baseline_tree_oid: commit.tree_id(),
                baseline_index_digest: [0; 32],
            },
        )
        .unwrap();
    f.local
        .service
        .begin_synchronization_integration_effect(&f.local.root, &owner, 0, None)
        .unwrap();
    f.local
        .service
        .release_synchronization_conflict(&f.local.root, &owner, 0, [1; 32])
        .unwrap();
    let before = f.local.service.remote_snapshot(&f.local.root).unwrap();
    let count = f
        .db()
        .query_row("SELECT count(*) FROM remote_operation_records", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    let mut session = crate::repository::keys::SessionCredentials::new(PanicCredentials);
    for invocation in [
        RemotePollInvocation::Automatic,
        RemotePollInvocation::Explicit,
    ] {
        assert!(matches!(
            f.local
                .service
                .poll_publication_remote(poll_request(&f, invocation), &mut session),
            Err(RemotePollError::ConflictPending)
        ));
    }
    assert_eq!(
        f.local.service.remote_snapshot(&f.local.root).unwrap(),
        before
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        count
    );
}

#[test]
fn task6_disposition_insert_fault_rolls_back_advertisement_and_old_report() {
    let f = PollFixture::new();
    f.local
        .tracking(&SynchronizationTarget::Primary, f.local.old);
    f.db().execute_batch("CREATE TRIGGER fail_disposition BEFORE INSERT ON remote_poll_dispositions BEGIN SELECT RAISE(ABORT,'fixture');END;").unwrap();
    assert!(
        f.run(vec![("refs/heads/main".into(), f.local.old)])
            .is_err()
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_poll_dispositions", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    assert_eq!(
        f.local
            .service
            .active_remote_operation(&f.local.root)
            .unwrap()
            .unwrap()
            .completed_step(),
        Some(super::super::RemoteOperationSafePoint::BeforeBatchCommit)
    );
}

#[test]
fn task6_absent_known_context_preserves_tracking_and_local_files() {
    let f = PollFixture::new();
    let (target, linked) = f.local.context();
    f.local.tracking(&target, f.local.old);
    f.local
        .tracking(&SynchronizationTarget::Primary, f.local.old);
    let context = f
        .local
        .plan
        .context(AuthoringKind::Document, &POLL_ID.parse().unwrap());
    state::with_transaction(&f.local.service, &f.local.root, |tx, id| {
        state::persist_complete_advertisement(
            tx,
            id,
            &f.local.plan,
            state::generation(tx, id)?,
            &[super::super::RemoteRefObservation::from_advertisement(
                &f.local.plan,
                context.remote_ref(),
                f.local.old,
                Some(f.local.old),
            )
            .unwrap()],
            123,
        )
        .map(|_| ())
    })
    .unwrap();
    fs::write(linked.workdir().unwrap().join("local"), b"local work").unwrap();
    let before = snapshot(linked.workdir().unwrap(), &linked);
    let (r, _) = report(
        f.run(vec![("refs/heads/main".into(), f.local.old)])
            .unwrap(),
    );
    assert_eq!(
        r.targets()[1].disposition(),
        PollDisposition::RemotelyDeleted
    );
    assert_eq!(snapshot(linked.workdir().unwrap(), &linked), before);
    assert_eq!(
        f.local
            .repository
            .refname_to_id(context.tracking_ref())
            .unwrap(),
        f.local.old
    );
}

#[test]
fn task6_config_and_selected_key_source_after_fetch_fence_before_batch() {
    use crate::repository::keys::*;
    for change in ["configuration", "key_source"] {
        let f = PollFixture::new();
        end_fixture_reservation(&f);
        let key = f.local._data.path().join("k");
        fs::write(&key, b"fixture source identity only").unwrap();
        let registration = match f
            .local
            .service
            .register_shared_key(RegisterSharedKeyRequest {
                label: "fixture".into(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: key.clone(),
                public_key_path: None,
            })
            .unwrap()
        {
            RegisterSharedKeyOutcome::Registered(r) => r,
            _ => panic!("registration"),
        };
        f.local.service.select_shared_key(registration.id).unwrap();
        let configuration = f
            .local
            .service
            .observation_configuration(&f.local.root, &f.local.plan)
            .unwrap();
        state::with_transaction(&f.local.service, &f.local.root, |tx, id| {
            state::configure_endpoints(tx, id, &f.local.plan, &configuration.endpoint_digest())
        })
        .unwrap();
        let owner = match f
            .local
            .service
            .reserve_remote_poll(
                &f.local.root,
                crate::repository::OperationId::new(),
                &super::super::RemoteOperationTarget::for_poll(&f.local.plan),
                super::super::RemoteOperationPriority::Poll,
            )
            .unwrap()
        {
            super::super::reservation::RemotePollReservationOutcome::Reservation(
                super::super::RemoteReservationOutcome::Reserved(o),
            ) => o,
            other => panic!("{other:?}"),
        };
        f.local
            .service
            .remote_safe_point(
                &f.local.root,
                &owner,
                super::super::RemoteOperationSafePoint::BeforeTransport,
            )
            .unwrap();
        f.local
            .tracking(&SynchronizationTarget::Primary, f.local.old);
        if change == "configuration" {
            fs::write(
                f.local.root.join(".manyhands/config.toml"),
                "format_version = 1\nprimary_branch = \"other\"\npublication_remote = \"origin\"\n",
            )
            .unwrap();
        } else {
            fs::write(&key, b"externally replaced longer fixture source").unwrap();
        }
        let before = snapshot(&f.local.root, &f.local.repository);
        assert!(matches!(
            f.local.service.poll_after_fetch(
                &f.local.root,
                &owner,
                &f.local.plan,
                &configuration,
                vec![("refs/heads/main".into(), f.local.old)]
            ),
            Err(RemotePollError::Interrupted)
        ));
        assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn task6_tracking_mismatch_after_fetch_interrupts_before_local_change() {
    let f = PollFixture::new();
    let incoming = f.local.incoming(f.local.old, None);
    f.local
        .tracking(&SynchronizationTarget::Primary, f.local.old);
    let before = snapshot(&f.local.root, &f.local.repository);
    let (r, pending) = report(f.run(vec![("refs/heads/main".into(), incoming)]).unwrap());
    assert_eq!(r.completion(), PollCompletion::Interrupted);
    assert!(!pending);
    assert_eq!(r.targets()[0].disposition(), PollDisposition::NotReached);
    assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
}

#[test]
fn task6_atomic_extension_cancellation_and_stale_epoch_never_run_callback() {
    use super::super::{RemoteOperationSafePoint as S, reservation::commit_observation_batch_with};
    for stale in [false, true] {
        let f = PollFixture::new();
        for point in [S::AfterAdvertisement, S::BeforeBatchCommit] {
            f.local
                .service
                .remote_safe_point(&f.local.root, &f.owner, point)
                .unwrap();
        }
        if stale {
            f.local
                .service
                .restart_remote_poll(
                    &f.local.root,
                    f.owner.operation_id(),
                    &super::super::RemoteOperationTarget::for_poll(&f.local.plan),
                )
                .unwrap();
        } else {
            f.local
                .service
                .cancel_remote_operation(&f.local.root, f.owner.operation_id())
                .unwrap();
        }
        let result = commit_observation_batch_with(
            &f.local.service,
            &f.local.root,
            &f.owner,
            &f.local.plan,
            &[],
            123,
            |_, _| panic!("stopped/stale callback"),
        );
        if stale {
            assert!(result.is_err());
        } else {
            assert_eq!(
                result.unwrap(),
                super::super::RemoteSafePointOutcome::Cancelled
            );
        }
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn task6_external_local_or_tracking_change_before_target_interrupts_remaining_work() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    for change in ["local", "tracking", "endpoint"] {
        let f = PollFixture::new();
        let (target, linked) = f.local.context();
        let incoming = f.local.incoming(f.local.old, None);
        f.local.tracking(&target, incoming);
        f.local.tracking(&SynchronizationTarget::Primary, incoming);
        let other = f.local.incoming(incoming, Some("external"));
        let context = f
            .local
            .plan
            .context(AuthoringKind::Document, &POLL_ID.parse().unwrap());
        let root = f.local.root.clone();
        let reference = context.clone();
        let called = Arc::new(AtomicBool::new(false));
        let hook_called = called.clone();
        f.local
            .service
            .set_poll_boundary_hook_for_testing(move |point, ordinal| {
                if point == crate::repository::FailurePoint::PollBeforeTarget && ordinal == Some(1)
                {
                    let repo = git2::Repository::open(&root).unwrap();
                    match change {
                        "local" => {
                            repo.reference(
                                reference.remote_ref(),
                                other,
                                true,
                                "external local change",
                            )
                            .unwrap();
                        }
                        "tracking" => {
                            repo.reference(
                                reference.tracking_ref(),
                                other,
                                true,
                                "external tracking change",
                            )
                            .unwrap();
                        }
                        "endpoint" => repo
                            .remote_set_url("origin", "ssh://other@example.invalid/repo")
                            .unwrap(),
                        _ => unreachable!(),
                    }
                    hook_called.store(true, Ordering::SeqCst);
                }
            });
        let files = files_at(linked.workdir().unwrap());
        let index = fs::read(linked.path().join("index")).unwrap();
        let (r, pending) = report(
            f.run(vec![
                ("refs/heads/main".into(), incoming),
                (context.remote_ref().into(), incoming),
            ])
            .unwrap(),
        );
        assert!(called.load(Ordering::SeqCst), "public boundary not reached");
        assert!(pending);
        assert_eq!(r.completion(), PollCompletion::Interrupted);
        assert_eq!(
            r.targets()
                .iter()
                .map(|t| t.disposition())
                .collect::<Vec<_>>(),
            [PollDisposition::FastForwarded, PollDisposition::NotReached]
        );
        assert_eq!(files_at(linked.workdir().unwrap()), files);
        assert_eq!(fs::read(linked.path().join("index")).unwrap(), index);
        assert_eq!(f.local.repository.head().unwrap().target(), Some(incoming));
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM operation_records", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn task6_final_endpoint_fence_stops_before_discovery_and_policy_completion() {
    let f = PollFixture::new();
    let incoming = f.local.incoming(f.local.old, None);
    f.local.tracking(&SynchronizationTarget::Primary, incoming);
    let root = f.local.root.clone();
    f.local
        .service
        .set_poll_boundary_hook_for_testing(move |point, _| {
            if point == crate::repository::FailurePoint::PollBeforeDiscovery {
                git2::Repository::open(&root)
                    .unwrap()
                    .remote_set_url("origin", "ssh://other@example.invalid/repo")
                    .unwrap();
            }
        });
    let (r, pending) = report(f.run(vec![("refs/heads/main".into(), incoming)]).unwrap());
    assert!(pending);
    assert_eq!(r.completion(), PollCompletion::Interrupted);
    assert_eq!(r.targets()[0].disposition(), PollDisposition::FastForwarded);
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM operation_records", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        f.local
            .service
            .remote_snapshot(&f.local.root)
            .unwrap()
            .latest_outcome(),
        None
    );
}

#[test]
fn task6_registered_inaccessible_context_is_classified_without_materialization_adoption() {
    let f = PollFixture::new();
    let (target, linked) = f.local.context();
    f.local
        .repository
        .branch(
            "foreign",
            &f.local.repository.find_commit(f.local.old).unwrap(),
            false,
        )
        .unwrap();
    linked.set_head("refs/heads/foreign").unwrap();
    let incoming = f.local.incoming(f.local.old, None);
    f.local.tracking(&target, incoming);
    f.local
        .tracking(&SynchronizationTarget::Primary, f.local.old);
    let context = f
        .local
        .plan
        .context(AuthoringKind::Document, &POLL_ID.parse().unwrap());
    let before = snapshot(linked.workdir().unwrap(), &linked);
    let (r, _) = report(
        f.run(vec![
            ("refs/heads/main".into(), f.local.old),
            (context.remote_ref().into(), incoming),
        ])
        .unwrap(),
    );
    assert_eq!(r.targets()[1].disposition(), PollDisposition::Inaccessible);
    assert_eq!(snapshot(linked.workdir().unwrap(), &linked), before);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn task6_remote_only_history_does_not_precede_existing_local_contexts() {
    let f = PollFixture::new();
    let (target, _) = f.local.context();
    f.local.tracking(&target, f.local.old);
    f.local
        .tracking(&SynchronizationTarget::Primary, f.local.old);
    let id: canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FA0".parse().unwrap();
    let remote = f.local.plan.context(AuthoringKind::Document, &id);
    state::with_transaction(&f.local.service, &f.local.root, |tx, repo| {
        state::persist_complete_advertisement(
            tx,
            repo,
            &f.local.plan,
            state::generation(tx, repo)?,
            &[super::super::RemoteRefObservation::from_advertisement(
                &f.local.plan,
                remote.remote_ref(),
                f.local.old,
                Some(f.local.old),
            )
            .unwrap()],
            123,
        )
        .map(|_| ())
    })
    .unwrap();
    f.local
        .repository
        .reference(
            remote.tracking_ref(),
            f.local.old,
            false,
            "test installed tracking",
        )
        .unwrap();
    let local = f
        .local
        .plan
        .context(AuthoringKind::Document, &POLL_ID.parse().unwrap());
    let (r, _) = report(
        f.run(vec![
            ("refs/heads/main".into(), f.local.old),
            (remote.remote_ref().into(), f.local.old),
            (local.remote_ref().into(), f.local.old),
        ])
        .unwrap(),
    );
    assert_eq!(r.targets()[1].item_id(), Some(&POLL_ID.parse().unwrap()));
    assert_eq!(r.targets()[2].item_id(), Some(&id));
}

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

fn policy(
    enabled: bool,
    paused: bool,
    suspended: bool,
    interval: u64,
) -> RemotePollingConfiguration {
    RemotePollingConfiguration::new(
        enabled,
        paused,
        PollingInterval::from_seconds(interval).unwrap(),
        None,
        suspended,
    )
}

#[test]
fn eligibility_policy_precedence_wins_over_due_and_future_deadlines() {
    for next in [None, Some(at(999)), Some(at(100_000))] {
        for (policy, reason) in [
            (policy(false, true, true, 300), PollIneligibility::Disabled),
            (policy(true, true, true, 300), PollIneligibility::Paused),
            (
                policy(true, false, true, 300),
                PollIneligibility::RecoverySuspended,
            ),
        ] {
            assert_eq!(
                eligibility(&policy, next, at(1000)),
                AutomaticPollEligibility::NotEligible {
                    reason,
                    next_eligible_at: next
                }
            );
        }
    }
}

#[test]
fn empty_past_and_equal_deadlines_are_due_and_future_is_not_due() {
    let policy = RemotePollingConfiguration::default();
    for next in [None, Some(at(999)), Some(at(1000))] {
        assert_eq!(
            eligibility(&policy, next, at(1000)),
            AutomaticPollEligibility::Eligible
        );
    }
    assert_eq!(
        eligibility(&policy, Some(at(1001)), at(1000)),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1001))
        }
    );
}

#[test]
fn rollback_clamp_uses_larger_of_interval_and_maximum_backoff() {
    for (interval, bound) in [(60, 1900), (300, 1900), (900, 1900), (3600, 4600)] {
        let policy = policy(true, false, false, interval);
        assert_eq!(
            eligibility(&policy, Some(at(100_000)), at(1000)),
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(at(bound))
            }
        );
        assert_eq!(
            eligibility(&policy, Some(at(bound)), at(1100)),
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(at(bound))
            }
        );
        assert_eq!(
            eligibility(&policy, Some(at(bound)), at(bound)),
            AutomaticPollEligibility::Eligible
        );
    }
}

#[test]
fn next_eligibility_result_table_covers_every_category_and_invocation() {
    let policy = RemotePollingConfiguration::default()
        .with_automatic_backoff(AutomaticBackoff::from_seconds(120).unwrap());
    let cases = [
        (
            RemoteOutcomeCategory::Completed,
            Some(at(1300)),
            Some(at(1300)),
        ),
        (
            RemoteOutcomeCategory::TransportUnavailable,
            Some(at(1240)),
            None,
        ),
        (
            RemoteOutcomeCategory::ProtocolRejected,
            Some(at(1240)),
            None,
        ),
        (
            RemoteOutcomeCategory::SelectedKeyUnavailable,
            Some(at(1300)),
            None,
        ),
        (RemoteOutcomeCategory::UnlockRequired, Some(at(1300)), None),
        (
            RemoteOutcomeCategory::HostApprovalRequired,
            Some(at(1300)),
            None,
        ),
        (
            RemoteOutcomeCategory::ConfigurationRequired,
            Some(at(1300)),
            None,
        ),
        (
            RemoteOutcomeCategory::RepositoryUnavailable,
            Some(at(1300)),
            None,
        ),
        (RemoteOutcomeCategory::Cancelled, None, None),
    ];
    for (category, automatic, explicit) in cases {
        for (invocation, expected) in [
            (RemotePollInvocation::Automatic, automatic),
            (RemotePollInvocation::Explicit, explicit),
        ] {
            assert_eq!(
                next_eligible(category, invocation, at(1000), &policy),
                expected,
                "{category:?} {invocation:?}"
            );
        }
    }
}

#[test]
fn network_retry_deadlines_start_at_sixty_double_and_cap_at_nine_hundred() {
    for category in [
        RemoteOutcomeCategory::TransportUnavailable,
        RemoteOutcomeCategory::ProtocolRejected,
    ] {
        let mut policy = RemotePollingConfiguration::default();
        for delay in [60, 120, 240, 480, 900, 900] {
            assert_eq!(
                next_eligible(category, RemotePollInvocation::Automatic, at(1000), &policy),
                Some(at(1000 + delay))
            );
            assert_eq!(
                next_eligible(category, RemotePollInvocation::Explicit, at(1000), &policy),
                None
            );
            policy = policy
                .with_automatic_backoff(AutomaticBackoff::from_seconds(delay as u64).unwrap());
        }
    }
}

fn fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    Connection,
) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    git2::Repository::init(root.path()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)", [root.path().canonicalize().unwrap().to_str().unwrap()]).unwrap();
    connection.execute_batch("CREATE TABLE eligibility_writes(n INTEGER NOT NULL); INSERT INTO eligibility_writes VALUES(0); CREATE TRIGGER count_eligibility_writes AFTER UPDATE ON remote_polling_state BEGIN UPDATE eligibility_writes SET n=n+1; END;").unwrap();
    (data, root, service, connection)
}

#[test]
fn stored_clamp_is_written_once_and_counts_down_without_reserving() {
    let (_data, root, service, connection) = fixture();
    connection.execute_batch("UPDATE remote_polling_state SET next_eligible_at=100000; UPDATE eligibility_writes SET n=0").unwrap();
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1000)).unwrap(),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1900))
        }
    );
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1100)).unwrap(),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1900))
        }
    );
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1900)).unwrap(),
        AutomaticPollEligibility::Eligible
    );
    assert_eq!(
        connection
            .query_row("SELECT n FROM eligibility_writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM remote_operation_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(snapshot.latest_attempt_at(), None);
    assert_eq!(snapshot.next_eligible_at(), Some(at(1900)));
}

#[test]
fn ineligible_policy_writes_nothing_even_when_deadline_needs_clamping() {
    let (_data, root, service, connection) = fixture();
    for (enabled, paused, suspended, reason) in [
        (false, true, true, PollIneligibility::Disabled),
        (true, true, true, PollIneligibility::Paused),
        (true, false, true, PollIneligibility::RecoverySuspended),
    ] {
        connection.execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=100000", params![enabled, paused, suspended]).unwrap();
        connection
            .execute_batch("UPDATE eligibility_writes SET n=0")
            .unwrap();
        assert_eq!(
            automatic_poll_eligibility_at(&service, root.path(), at(1000)).unwrap(),
            AutomaticPollEligibility::NotEligible {
                reason,
                next_eligible_at: Some(at(100000))
            }
        );
        assert_eq!(
            connection
                .query_row("SELECT n FROM eligibility_writes", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn public_eligibility_is_due_without_an_endpoint_and_does_not_write() {
    let (_data, root, service, connection) = fixture();
    assert_eq!(
        service.automatic_poll_eligibility(root.path()).unwrap(),
        AutomaticPollEligibility::Eligible
    );
    assert_eq!(
        connection
            .query_row("SELECT n FROM eligibility_writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

const POLL_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

#[test]
fn poll_dispositions_have_only_fixed_guidance() {
    for (disposition, guidance) in [
        (PollDisposition::Current, "Local state is current."),
        (
            PollDisposition::FastForwarded,
            "Clean local state was fast-forwarded.",
        ),
        (
            PollDisposition::LocalAhead,
            "Publish local commits with deliberate synchronization.",
        ),
        (
            PollDisposition::Diverged,
            "Resolve divergent history with deliberate synchronization.",
        ),
        (
            PollDisposition::Dirty,
            "Save or otherwise handle local changes before polling again.",
        ),
        (
            PollDisposition::Conflicted,
            "Resolve the conflict or in-progress Git operation before polling again.",
        ),
        (
            PollDisposition::Inaccessible,
            "Restore the expected checked-out branch and registered worktree before polling again.",
        ),
        (
            PollDisposition::Materialized,
            "The remote context was materialized locally.",
        ),
        (
            PollDisposition::Nonconforming,
            "Repair the remote context's canonical tree before materialization.",
        ),
        (
            PollDisposition::BlockedByLocalState,
            "Inspect the existing local branch or path before materialization.",
        ),
        (
            PollDisposition::MaterializationIncomplete,
            "Inspect and repair the retained partial materialization.",
        ),
        (
            PollDisposition::Malformed,
            "Repair the remote context's branch name.",
        ),
        (
            PollDisposition::RemotelyDeleted,
            "The remote context is absent; local state was preserved.",
        ),
        (
            PollDisposition::HistoryUnknown,
            "Publication history is unknown; inspect remote state before proceeding.",
        ),
        (
            PollDisposition::NotReached,
            "This target was not reached; retry the poll when ready.",
        ),
    ] {
        assert_eq!(disposition.guidance(), guidance);
    }
}

#[test]
fn poll_relation_equal_behind_ahead_and_diverged_are_pure() {
    let a = git2::Oid::from_bytes(&[1; 20]).unwrap();
    let b = git2::Oid::from_bytes(&[2; 20]).unwrap();
    assert_eq!(
        classify_relation(a, a, |_, _| -> Result<bool, ()> {
            panic!("equal needs no ancestry")
        }),
        Ok(PollRelation::Current)
    );
    for (answers, expected) in [
        ([true, false], PollRelation::Behind),
        ([false, true], PollRelation::LocalAhead),
        ([false, false], PollRelation::Diverged),
    ] {
        let mut calls = Vec::new();
        assert_eq!(
            classify_relation(a, b, |x, y| -> Result<bool, ()> {
                calls.push((x, y));
                Ok(if (x, y) == (a, b) {
                    answers[0]
                } else {
                    assert_eq!((x, y), (b, a));
                    answers[1]
                })
            }),
            Ok(expected)
        );
        assert_eq!(calls.len(), if answers[0] { 1 } else { 2 });
    }
}

#[test]
fn poll_relation_surfaces_both_ancestry_errors_without_guessing() {
    let a = git2::Oid::from_bytes(&[1; 20]).unwrap();
    let b = git2::Oid::from_bytes(&[2; 20]).unwrap();
    for failing_call in [1, 2] {
        let mut calls = 0;
        assert_eq!(
            classify_relation(a, b, |_, _| {
                calls += 1;
                if calls == failing_call {
                    Err("ancestry unavailable")
                } else {
                    Ok(false)
                }
            }),
            Err("ancestry unavailable")
        );
        assert_eq!(calls, failing_call);
    }
}

struct LocalFixture {
    _root: tempfile::TempDir,
    _data: tempfile::TempDir,
    root: PathBuf,
    service: RepositoryService,
    plan: RemoteRefPlan,
    repository: git2::Repository,
    old: git2::Oid,
}

struct MaterializationFixture {
    local: LocalFixture,
    request: PollMaterializationRequest,
    configuration: super::super::observation::ObservationConfiguration,
}

impl MaterializationFixture {
    fn new(kind: AuthoringKind, failed_checkout: bool) -> Self {
        let local = LocalFixture::new();
        let db =
            rusqlite::Connection::open(local._data.path().join(crate::repository::REGISTRY_FILE))
                .unwrap();
        db.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)", [local.root.to_str().unwrap()]).unwrap();
        state::with_transaction(&local.service, &local.root, |tx, id| {
            state::configure(tx, id, Some(&local.plan), false)
        })
        .unwrap();
        let item_id: canonical::ItemId = POLL_ID.parse().unwrap();
        let path = match kind {
            AuthoringKind::Document => "docs/item.md".to_owned(),
            AuthoringKind::Ticket => format!(".manyhands/tickets/{POLL_ID}/ticket.md"),
        };
        let mut entries = vec![(path.as_str(), item_source(kind, POLL_ID), 0o100644)];
        // Task 0's real Linux checkout fault: ordinary path written before an
        // overlong symlink target fails. No synthetic worktree transport/fault.
        if failed_checkout {
            entries.push(("a", b"retained".to_vec(), 0o100644));
            entries.push(("z", vec![b'x'; 5000], 0o120000));
        }
        let tree = local
            .repository
            .find_tree(fixture_tree(&local.repository, &entries))
            .unwrap();
        let sig =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(1, 0))
                .unwrap();
        let oid = local
            .repository
            .commit(
                None,
                &sig,
                &sig,
                "context",
                &tree,
                &[&local.repository.find_commit(local.old).unwrap()],
            )
            .unwrap();
        drop(tree);
        let target = SynchronizationTarget::Context {
            kind,
            item_id: item_id.clone(),
        };
        local.tracking(&target, oid);
        let operation_id = crate::repository::OperationId::new();
        let operation_target = super::super::RemoteOperationTarget::for_poll(&local.plan);
        assert!(matches!(
            local
                .service
                .reserve_remote_poll(
                    &local.root,
                    operation_id,
                    &operation_target,
                    super::super::RemoteOperationPriority::Poll
                )
                .unwrap(),
            super::super::reservation::RemotePollReservationOutcome::Reservation(
                super::super::RemoteReservationOutcome::Reserved(_)
            )
        ));
        let configuration = local
            .service
            .observation_configuration(&local.root, &local.plan)
            .unwrap();
        Self {
            local,
            request: PollMaterializationRequest {
                operation_id,
                kind,
                item_id,
                oid,
            },
            configuration,
        }
    }

    fn run(
        &self,
        service: &RepositoryService,
    ) -> Result<PollMaterializationOutcome, SynchronizationError> {
        service.materialize_context(
            &self.local.root,
            &self.local.plan,
            &self.configuration,
            &self.request,
        )
    }

    fn path(&self) -> PathBuf {
        self.local.root.join(".manyhands/worktrees").join(POLL_ID)
    }
    fn branch(&self) -> String {
        format!(
            "manyhands/{}/{POLL_ID}",
            match self.request.kind {
                AuthoringKind::Document => "document",
                AuthoringKind::Ticket => "ticket",
            }
        )
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(
            self.local
                ._data
                .path()
                .join(crate::repository::REGISTRY_FILE),
        )
        .unwrap()
    }
    fn intent(&self) -> Option<(String, String, String)> {
        use rusqlite::OptionalExtension;
        self.db()
            .query_row(
                "SELECT operation_ulid,target_oid,step FROM remote_poll_materializations",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .unwrap()
    }
    fn assert_complete(&self) {
        let r = &self.local.repository;
        assert_eq!(
            r.find_branch(&self.branch(), git2::BranchType::Local)
                .unwrap()
                .get()
                .target(),
            Some(self.request.oid)
        );
        assert_eq!(r.worktrees().unwrap().len(), 1);
        assert_eq!(
            r.find_worktree(POLL_ID)
                .unwrap()
                .path()
                .canonicalize()
                .unwrap(),
            self.path().canonicalize().unwrap()
        );
        let linked = git2::Repository::open(self.path()).unwrap();
        assert_eq!(
            linked.head().unwrap().name(),
            Some(format!("refs/heads/{}", self.branch()).as_str())
        );
        assert_eq!(linked.head().unwrap().target(), Some(self.request.oid));
        assert!(linked.statuses(None).unwrap().is_empty());
        assert_eq!(
            linked.index().unwrap().write_tree().unwrap(),
            linked.find_commit(self.request.oid).unwrap().tree_id()
        );
        for suffix in ["remote", "merge"] {
            assert!(
                r.config()
                    .unwrap()
                    .get_string(&format!("branch.{}.{suffix}", self.branch()))
                    .is_err()
            );
        }
        let context = crate::repository::ItemContext {
            root: self.local.root.clone(),
            worktree: self.path(),
            branch: self.branch(),
            kind: self.request.kind,
            item_id: self.request.item_id.clone(),
        };
        crate::repository::validate_context_worktree(
            r,
            &context,
            true,
            RepositoryOperation::RepositorySnapshot,
        )
        .unwrap();
        assert_eq!(
            self.intent(),
            Some((
                self.request.operation_id.to_string(),
                self.request.oid.to_string(),
                "complete".into()
            ))
        );
    }
}

#[test]
fn poll_materialization_document_and_ticket_once_reopen_and_later_poll() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = MaterializationFixture::new(kind, false);
        assert_eq!(
            f.run(&f.local.service).unwrap(),
            PollMaterializationOutcome::Materialized
        );
        f.assert_complete();
        let before = snapshot(&f.local.root, &f.local.repository);
        let config_before = fs::read(f.local.repository.commondir().join("config")).unwrap();
        let later = RepositoryService::open_at(f.local._data.path()).unwrap();
        assert_eq!(f.run(&later).unwrap(), PollMaterializationOutcome::Present);
        // A subsequent poll's operation ID must not rewrite the original owner.
        let mut request = f.request.clone();
        request.operation_id = crate::repository::OperationId::new();
        assert_eq!(
            later
                .materialize_context(&f.local.root, &f.local.plan, &f.configuration, &request)
                .unwrap(),
            PollMaterializationOutcome::Present
        );
        assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
        assert_eq!(
            fs::read(f.local.repository.commondir().join("config")).unwrap(),
            config_before
        );
        f.assert_complete();
        assert_eq!(
            crate::repository::discovery::observe_root(&f.local.repository, &f.local.root)
                .active_contexts
                .len(),
            1
        );
    }
}

fn interrupted_owned_worktree(kind: AuthoringKind) -> (MaterializationFixture, git2::Repository) {
    let f = MaterializationFixture::new(kind, false);
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        f.local._data.path(),
        crate::repository::FailurePoint::PollAfterMaterializationWorktree,
    )
    .unwrap();
    assert!(
        matches!(f.run(&service), Err(SynchronizationError::Repository(error)) if error.kind == crate::repository::RepositoryErrorKind::InjectedFailure)
    );
    assert_eq!(
        f.intent(),
        Some((
            f.request.operation_id.to_string(),
            f.request.oid.to_string(),
            "branch_created".into()
        ))
    );
    let linked = poll_materialized_target(
        &f.local.root,
        f.local.plan.primary_branch(),
        &SynchronizationTarget::Context {
            kind,
            item_id: f.request.item_id.clone(),
        },
    )
    .unwrap();
    assert_eq!(linked.head().unwrap().target(), Some(f.request.oid));
    assert!(linked.statuses(None).unwrap().is_empty());
    assert!(linked.path().join("index").is_file());
    (f, linked)
}

fn assert_owned_completion_preserves_git_and_classifies(
    f: &MaterializationFixture,
    linked: &git2::Repository,
    expected: PollDisposition,
) {
    let target = SynchronizationTarget::Context {
        kind: f.request.kind,
        item_id: f.request.item_id.clone(),
    };
    let local = linked.head().unwrap().target().unwrap();
    let before = snapshot(&f.local.root, &f.local.repository);
    let target_before = snapshot(&f.path(), linked);
    let config_before = fs::read(f.local.repository.commondir().join("config")).unwrap();
    let service = RepositoryService::open_at(f.local._data.path()).unwrap();
    let mut request = f.request.clone();
    request.operation_id = crate::repository::OperationId::new();
    assert_eq!(
        service
            .materialize_context(&f.local.root, &f.local.plan, &f.configuration, &request)
            .unwrap(),
        PollMaterializationOutcome::Present,
    );
    assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
    assert_eq!(snapshot(&f.path(), linked), target_before);
    assert_eq!(
        fs::read(f.local.repository.commondir().join("config")).unwrap(),
        config_before
    );
    assert_eq!(
        f.intent(),
        Some((
            f.request.operation_id.to_string(),
            f.request.oid.to_string(),
            "complete".into()
        )),
    );
    assert_eq!(f.local.repository.worktrees().unwrap().len(), 1);
    assert_eq!(
        classify_local_target(
            &f.local.root,
            f.local.plan.primary_branch(),
            &target,
            PollTrackingState::Present(f.request.oid)
        )
        .unwrap(),
        LocalPollClassification::Unchanged {
            disposition: expected,
            local_oid: Some(local)
        },
    );
    assert_eq!(
        f.run(&service).unwrap(),
        PollMaterializationOutcome::Present
    );
    assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
    assert_eq!(snapshot(&f.path(), linked), target_before);
    assert_eq!(f.intent().unwrap().2, "complete");
}

#[test]
fn poll_materialization_owned_unfinished_current_or_untracked_is_present_then_classified() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for untracked in [false, true] {
            let (f, linked) = interrupted_owned_worktree(kind);
            if untracked {
                fs::write(f.path().join("local.txt"), b"retained local work").unwrap();
            }
            assert_owned_completion_preserves_git_and_classifies(
                &f,
                &linked,
                if untracked {
                    PollDisposition::Dirty
                } else {
                    PollDisposition::Current
                },
            );
        }
    }
}

#[test]
fn poll_materialization_owned_unfinished_valid_canonical_edit_is_present_then_dirty() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let (f, linked) = interrupted_owned_worktree(kind);
        let path = match kind {
            AuthoringKind::Document => f.path().join("docs/item.md"),
            AuthoringKind::Ticket => f
                .path()
                .join(format!(".manyhands/tickets/{POLL_ID}/ticket.md")),
        };
        let mut source = fs::read(&path).unwrap();
        source.extend_from_slice(b"\nRetained canonical body edit.\n");
        fs::write(path, source).unwrap();
        assert_owned_completion_preserves_git_and_classifies(&f, &linked, PollDisposition::Dirty);
    }
}

#[test]
fn poll_materialization_owned_unfinished_local_commit_is_present_then_local_ahead() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let (f, linked) = interrupted_owned_worktree(kind);
        let path = match kind {
            AuthoringKind::Document => PathBuf::from("docs/item.md"),
            AuthoringKind::Ticket => {
                PathBuf::from(format!(".manyhands/tickets/{POLL_ID}/ticket.md"))
            }
        };
        let mut source = fs::read(f.path().join(&path)).unwrap();
        source.extend_from_slice(b"\nRetained local commit.\n");
        fs::write(f.path().join(&path), source).unwrap();
        let mut index = linked.index().unwrap();
        index.add_path(&path).unwrap();
        index.write().unwrap();
        let tree = linked.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = linked.find_commit(f.request.oid).unwrap();
        let sig =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0))
                .unwrap();
        let advanced = linked
            .commit(Some("HEAD"), &sig, &sig, "local work", &tree, &[&parent])
            .unwrap();
        assert_ne!(advanced, f.request.oid);
        assert!(linked.statuses(None).unwrap().is_empty());
        assert_owned_completion_preserves_git_and_classifies(
            &f,
            &linked,
            PollDisposition::LocalAhead,
        );
    }
}

#[test]
fn poll_materialization_owned_unfinished_revalidation_refuses_malformed_or_foreign_contexts() {
    for case in ["malformed", "foreign", "missing_index"] {
        let (f, linked) = interrupted_owned_worktree(AuthoringKind::Document);
        match case {
            "malformed" => fs::write(
                f.path().join("docs/item.md"),
                b"---\nmanyhands_managed: true\nmanyhands_kind: document\nid: [\n---\n",
            )
            .unwrap(),
            "foreign" => {
                drop(linked);
                fs::rename(f.path(), f.local.root.join("retained_owned")).unwrap();
                let foreign = git2::Repository::init(f.path()).unwrap();
                copy_fixture_objects(&f.local.repository, &foreign);
                foreign
                    .reference(
                        &format!("refs/heads/{}", f.branch()),
                        f.request.oid,
                        false,
                        "fixture",
                    )
                    .unwrap();
                foreign
                    .set_head(&format!("refs/heads/{}", f.branch()))
                    .unwrap();
                foreign.checkout_head(None).unwrap();
            }
            "missing_index" => fs::remove_file(linked.path().join("index")).unwrap(),
            _ => unreachable!(),
        }
        let before = snapshot(&f.local.root, &f.local.repository);
        let intent = f.intent();
        let actual = git2::Repository::open(f.path()).unwrap();
        let target_before = snapshot(&f.path(), &actual);
        let service = RepositoryService::open_at(f.local._data.path()).unwrap();
        for _ in 0..2 {
            assert_eq!(
                f.run(&service).unwrap(),
                PollMaterializationOutcome::Refused(PollDisposition::MaterializationIncomplete),
                "{case}"
            );
            assert_eq!(
                snapshot(&f.local.root, &f.local.repository),
                before,
                "{case}"
            );
            assert_eq!(snapshot(&f.path(), &actual), target_before, "{case}");
            assert_eq!(f.intent(), intent, "{case}");
        }
    }
}

#[test]
fn poll_materialization_two_services_race_under_root_lease() {
    let f = MaterializationFixture::new(AuthoringKind::Document, false);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let service = RepositoryService::open_at(f.local._data.path()).unwrap();
        let (root, plan, configuration, request, barrier) = (
            f.local.root.clone(),
            f.local.plan.clone(),
            f.configuration.clone(),
            f.request.clone(),
            barrier.clone(),
        );
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            service.materialize_context(&root, &plan, &configuration, &request)
        }));
    }
    barrier.wait();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Ok(PollMaterializationOutcome::Materialized)))
            .count(),
        1
    );
    for result in results {
        assert!(matches!(
            result,
            Ok(PollMaterializationOutcome::Materialized | PollMaterializationOutcome::Present)
                | Err(SynchronizationError::Repository(
                    crate::repository::RepositoryError {
                        kind: crate::repository::RepositoryErrorKind::RepositoryBusy,
                        ..
                    }
                ))
        ));
    }
    // The lease is deliberately retryable, not a queued operation. Both racing
    // calls have returned before this retry; no delay or arbitrary sleep.
    let later = RepositoryService::open_at(f.local._data.path()).unwrap();
    assert_eq!(f.run(&later).unwrap(), PollMaterializationOutcome::Present);
    f.assert_complete();
}

#[test]
fn poll_materialization_local_obstruction_matrix_is_byte_exact_unchanged() {
    for case in [
        "same_branch",
        "different_branch",
        "checked_out_elsewhere",
        "file",
        "empty_dir",
        "nonempty_dir",
        "registration_elsewhere",
        "symlink",
        "redirect_manyhands",
        "redirect_base",
    ] {
        #[cfg(not(unix))]
        if matches!(case, "symlink" | "redirect_manyhands" | "redirect_base") {
            continue;
        }
        let f = MaterializationFixture::new(AuthoringKind::Document, false);
        let r = &f.local.repository;
        fs::create_dir(f.path().parent().unwrap()).unwrap();
        match case {
            "same_branch"
            | "different_branch"
            | "checked_out_elsewhere"
            | "registration_elsewhere" => {
                let oid = if case == "different_branch" {
                    f.local.old
                } else {
                    f.request.oid
                };
                let reference = r
                    .branch(&f.branch(), &r.find_commit(oid).unwrap(), false)
                    .unwrap()
                    .into_reference();
                if matches!(case, "checked_out_elsewhere" | "registration_elsewhere") {
                    let mut options = git2::WorktreeAddOptions::new();
                    options.reference(Some(&reference));
                    r.worktree(
                        if case == "registration_elsewhere" {
                            POLL_ID
                        } else {
                            "other"
                        },
                        &f.local.root.join("elsewhere"),
                        Some(&options),
                    )
                    .unwrap();
                }
            }
            "file" => fs::write(f.path(), b"sentinel").unwrap(),
            "empty_dir" | "nonempty_dir" => {
                fs::create_dir(f.path()).unwrap();
                if case == "nonempty_dir" {
                    fs::write(f.path().join("sentinel"), b"local").unwrap();
                }
            }
            #[cfg(unix)]
            "symlink" => std::os::unix::fs::symlink("missing", f.path()).unwrap(),
            #[cfg(unix)]
            "redirect_manyhands" | "redirect_base" => {
                let path = if case == "redirect_manyhands" {
                    f.local.root.join(".manyhands")
                } else {
                    f.path().parent().unwrap().to_owned()
                };
                let moved = f.local.root.join("redirected");
                fs::rename(&path, &moved).unwrap();
                std::os::unix::fs::symlink(moved, path).unwrap();
            }
            _ => unreachable!(),
        }
        let before = snapshot(&f.local.root, r);
        let config = fs::read(r.commondir().join("config")).unwrap();
        assert_eq!(
            f.run(&f.local.service).unwrap(),
            PollMaterializationOutcome::Refused(PollDisposition::BlockedByLocalState),
            "{case}"
        );
        assert_eq!(snapshot(&f.local.root, r), before, "{case}");
        assert_eq!(fs::read(r.commondir().join("config")).unwrap(), config);
        assert_eq!(f.intent(), None);
    }
}

#[test]
fn poll_materialization_interruptions_reconcile_only_approved_boundary() {
    use crate::repository::FailurePoint;
    for (point, step, branch, worktree) in [
        (
            FailurePoint::PollAfterMaterializationIntent,
            "intended",
            false,
            false,
        ),
        (
            FailurePoint::PollAfterMaterializationBranch,
            "branch_created",
            true,
            false,
        ),
        (
            FailurePoint::PollAfterMaterializationWorktree,
            "branch_created",
            true,
            true,
        ),
    ] {
        let f = MaterializationFixture::new(AuthoringKind::Ticket, false);
        let service =
            RepositoryService::open_at_with_failure_point_for_testing(f.local._data.path(), point)
                .unwrap();
        assert!(f.run(&service).is_err());
        assert_eq!(f.intent().unwrap().2, step);
        assert_eq!(
            f.local
                .repository
                .find_branch(&f.branch(), git2::BranchType::Local)
                .is_ok(),
            branch
        );
        assert_eq!(f.path().exists(), worktree);
        assert_eq!(f.local.repository.find_worktree(POLL_ID).is_ok(), worktree);
        let before = snapshot(&f.local.root, &f.local.repository);
        let reopened = RepositoryService::open_at(f.local._data.path()).unwrap();
        assert_eq!(
            f.run(&reopened).unwrap(),
            if worktree {
                PollMaterializationOutcome::Present
            } else {
                PollMaterializationOutcome::Materialized
            }
        );
        f.assert_complete();
        if worktree {
            assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
        }
        let complete = snapshot(&f.local.root, &f.local.repository);
        assert_eq!(
            f.run(&reopened).unwrap(),
            PollMaterializationOutcome::Present
        );
        assert_eq!(snapshot(&f.local.root, &f.local.repository), complete);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn poll_materialization_failed_git_checkout_retains_every_partial_artifact() {
    let f = MaterializationFixture::new(AuthoringKind::Document, true);
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        f.local._data.path(),
        crate::repository::FailurePoint::PollAfterFailedMaterializationWorktree,
    )
    .unwrap();
    assert!(f.run(&service).is_err());
    assert_eq!(f.intent().unwrap().2, "branch_created");
    assert_eq!(fs::read(f.path().join("a")).unwrap(), b"retained");
    assert!(f.path().join(".git").is_file());
    assert!(fs::symlink_metadata(f.path().join("z")).is_err());
    let admin = f
        .local
        .repository
        .commondir()
        .join("worktrees")
        .join(POLL_ID);
    for name in ["HEAD", "gitdir", "commondir"] {
        assert!(admin.join(name).is_file());
    }
    assert!(!admin.join("index").exists());
    assert!(f.local.repository.find_worktree(POLL_ID).is_ok());
    assert_eq!(
        f.local
            .repository
            .refname_to_id(&format!("refs/heads/{}", f.branch()))
            .unwrap(),
        f.request.oid
    );
    let before = snapshot(&f.local.root, &f.local.repository);
    let reopened = RepositoryService::open_at(f.local._data.path()).unwrap();
    for _ in 0..2 {
        assert_eq!(
            f.run(&reopened).unwrap(),
            PollMaterializationOutcome::Refused(PollDisposition::MaterializationIncomplete)
        );
        assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
    }
}

#[test]
fn poll_materialization_ambiguous_intents_and_partial_paths_never_retry_or_remove() {
    for case in [
        "intended_branch",
        "moved_branch",
        "partial_file",
        "partial_directory",
        "admin_only",
        "wrong_kind",
        "foreign_owner",
    ] {
        let f = MaterializationFixture::new(AuthoringKind::Document, false);
        let point = if case == "intended_branch" {
            crate::repository::FailurePoint::PollAfterMaterializationIntent
        } else {
            crate::repository::FailurePoint::PollAfterMaterializationBranch
        };
        let service =
            RepositoryService::open_at_with_failure_point_for_testing(f.local._data.path(), point)
                .unwrap();
        assert!(f.run(&service).is_err());
        match case {
            "intended_branch" => {
                f.local
                    .repository
                    .branch(
                        &f.branch(),
                        &f.local.repository.find_commit(f.request.oid).unwrap(),
                        false,
                    )
                    .unwrap();
            }
            "moved_branch" => {
                f.local
                    .repository
                    .reference(
                        &format!("refs/heads/{}", f.branch()),
                        f.local.old,
                        true,
                        "fixture external move",
                    )
                    .unwrap();
            }
            "partial_file" => fs::write(f.path(), b"local").unwrap(),
            "partial_directory" => {
                fs::create_dir(f.path()).unwrap();
                fs::write(f.path().join("local"), b"retained").unwrap();
            }
            "admin_only" => {
                fs::create_dir_all(
                    f.local
                        .repository
                        .commondir()
                        .join("worktrees")
                        .join(POLL_ID),
                )
                .unwrap();
            }
            "wrong_kind" => {
                let mut request = f.request.clone();
                request.kind = AuthoringKind::Ticket;
                f.local.tracking(
                    &SynchronizationTarget::Context {
                        kind: request.kind,
                        item_id: request.item_id.clone(),
                    },
                    request.oid,
                );
                let before = snapshot(&f.local.root, &f.local.repository);
                assert_eq!(
                    service
                        .materialize_context(
                            &f.local.root,
                            &f.local.plan,
                            &f.configuration,
                            &request
                        )
                        .unwrap(),
                    PollMaterializationOutcome::Refused(PollDisposition::BlockedByLocalState)
                );
                assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
                continue;
            }
            "foreign_owner" => {
                let reference = f
                    .local
                    .repository
                    .find_reference(&format!("refs/heads/{}", f.branch()))
                    .unwrap();
                let mut options = git2::WorktreeAddOptions::new();
                options.reference(Some(&reference));
                f.local
                    .repository
                    .worktree(POLL_ID, &f.path(), Some(&options))
                    .unwrap();
                fs::rename(f.path(), f.local.root.join("retained")).unwrap();
                let foreign = git2::Repository::init(f.path()).unwrap();
                copy_fixture_objects(&f.local.repository, &foreign);
                foreign
                    .reference(
                        &format!("refs/heads/{}", f.branch()),
                        f.request.oid,
                        false,
                        "fixture",
                    )
                    .unwrap();
                foreign
                    .set_head(&format!("refs/heads/{}", f.branch()))
                    .unwrap();
                foreign.checkout_head(None).unwrap();
            }
            _ => unreachable!(),
        }
        let before = snapshot(&f.local.root, &f.local.repository);
        let intent = f.intent();
        for _ in 0..2 {
            assert_eq!(
                f.run(&service).unwrap(),
                PollMaterializationOutcome::Refused(PollDisposition::MaterializationIncomplete),
                "{case}"
            );
            assert_eq!(
                snapshot(&f.local.root, &f.local.repository),
                before,
                "{case}"
            );
            assert_eq!(f.intent(), intent);
        }
    }
}

#[test]
fn poll_materialization_reobserves_tracking_configuration_and_endpoint() {
    for case in ["tracking", "config", "endpoint"] {
        let f = MaterializationFixture::new(AuthoringKind::Document, false);
        match case {
            "tracking" => f.local.tracking(
                &SynchronizationTarget::Context {
                    kind: f.request.kind,
                    item_id: f.request.item_id.clone(),
                },
                f.local.old,
            ),
            "config" => fs::write(
                f.local.root.join(".manyhands/config.toml"),
                "format_version=1\nprimary_branch=\"other\"\npublication_remote=\"origin\"\n",
            )
            .unwrap(),
            "endpoint" => f
                .local
                .repository
                .remote_set_url("origin", "ssh://other@example.invalid/repo")
                .unwrap(),
            _ => unreachable!(),
        }
        let before = snapshot(&f.local.root, &f.local.repository);
        assert!(
            matches!(
                f.run(&f.local.service),
                Err(SynchronizationError::ExternalChange)
            ),
            "{case}"
        );
        assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
        assert_eq!(f.intent(), None);
    }
}

#[test]
fn poll_materialization_git_absence_wins_over_any_cached_step() {
    for step in ["intended", "branch_created", "complete"] {
        let f = MaterializationFixture::new(AuthoringKind::Document, false);
        let service = RepositoryService::open_at_with_failure_point_for_testing(
            f.local._data.path(),
            crate::repository::FailurePoint::PollAfterMaterializationIntent,
        )
        .unwrap();
        assert!(f.run(&service).is_err());
        f.db()
            .execute("UPDATE remote_poll_materializations SET step=?1", [step])
            .unwrap();
        assert_eq!(
            f.run(&service).unwrap(),
            PollMaterializationOutcome::Materialized,
            "{step}"
        );
        f.assert_complete();
    }
}

#[test]
fn poll_materialization_nonconforming_tree_or_unrelated_operation_creates_nothing() {
    for case in ["nonconforming", "unrelated_operation"] {
        let f = MaterializationFixture::new(AuthoringKind::Document, false);
        let mut request = f.request.clone();
        if case == "nonconforming" {
            request.oid = f.local.old;
            f.local.tracking(
                &SynchronizationTarget::Context {
                    kind: request.kind,
                    item_id: request.item_id.clone(),
                },
                request.oid,
            );
        } else {
            request.operation_id = crate::repository::OperationId::new();
        }
        let before = snapshot(&f.local.root, &f.local.repository);
        let result = f.local.service.materialize_context(
            &f.local.root,
            &f.local.plan,
            &f.configuration,
            &request,
        );
        if case == "nonconforming" {
            assert_eq!(
                result.unwrap(),
                PollMaterializationOutcome::Refused(PollDisposition::Nonconforming)
            );
        } else {
            assert!(result.is_err());
        }
        assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
        assert_eq!(f.intent(), None);
    }
}

#[test]
fn poll_materialization_step_write_failure_does_not_infer_branch_ownership() {
    let f = MaterializationFixture::new(AuthoringKind::Document, false);
    f.db().execute_batch("CREATE TRIGGER fail_branch_step BEFORE UPDATE OF step ON remote_poll_materializations WHEN NEW.step='branch_created' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(f.run(&f.local.service).is_err());
    assert_eq!(f.intent().unwrap().2, "intended");
    assert_eq!(
        f.local
            .repository
            .refname_to_id(&format!("refs/heads/{}", f.branch()))
            .unwrap(),
        f.request.oid
    );
    assert!(!f.path().exists());
    assert!(f.local.repository.find_worktree(POLL_ID).is_err());
    f.db()
        .execute_batch("DROP TRIGGER fail_branch_step")
        .unwrap();
    let before = snapshot(&f.local.root, &f.local.repository);
    assert_eq!(
        f.run(&f.local.service).unwrap(),
        PollMaterializationOutcome::Refused(PollDisposition::MaterializationIncomplete)
    );
    assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
}

#[test]
fn poll_materialization_completed_intent_cannot_adopt_a_symbolic_branch_replacement() {
    let f = MaterializationFixture::new(AuthoringKind::Document, false);
    assert_eq!(
        f.run(&f.local.service).unwrap(),
        PollMaterializationOutcome::Materialized
    );
    f.local
        .repository
        .branch(
            "foreign",
            &f.local.repository.find_commit(f.request.oid).unwrap(),
            false,
        )
        .unwrap();
    f.local
        .repository
        .reference_symbolic(
            &format!("refs/heads/{}", f.branch()),
            "refs/heads/foreign",
            true,
            "external replacement",
        )
        .unwrap();
    let before = snapshot(&f.local.root, &f.local.repository);
    assert_eq!(
        f.run(&f.local.service).unwrap(),
        PollMaterializationOutcome::Refused(PollDisposition::MaterializationIncomplete)
    );
    assert_eq!(snapshot(&f.local.root, &f.local.repository), before);
}

impl LocalFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let data = tempfile::tempdir().unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = git2::Repository::init_opts(&root, &options).unwrap();
        repository
            .config()
            .unwrap()
            .set_bool("core.autocrlf", false)
            .unwrap();
        fs::create_dir(root.join(".manyhands")).unwrap();
        fs::write(
            root.join(".manyhands/config.toml"),
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
        )
        .unwrap();
        fs::write(root.join(".gitignore"), ".manyhands/worktrees/\nignored\n").unwrap();
        fs::write(root.join("file"), b"old\n").unwrap();
        let mut index = repository.index().unwrap();
        for path in ["file", ".gitignore", ".manyhands/config.toml"] {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(0, 0))
                .unwrap();
        let old = repository
            .commit(Some("HEAD"), &signature, &signature, "old", &tree, &[])
            .unwrap();
        drop(tree);
        repository
            .remote("origin", "ssh://fixture@example.invalid/repo")
            .unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        Self {
            _root: directory,
            _data: data,
            root,
            service,
            plan: RemoteRefPlan::from_configuration("origin", "main").unwrap(),
            repository,
            old,
        }
    }

    fn incoming(&self, parent: git2::Oid, extra: Option<&str>) -> git2::Oid {
        let parent = self.repository.find_commit(parent).unwrap();
        let mut builder = self
            .repository
            .treebuilder(Some(&parent.tree().unwrap()))
            .unwrap();
        builder
            .insert("file", self.repository.blob(b"new\n").unwrap(), 0o100644)
            .unwrap();
        if let Some(extra) = extra {
            builder
                .insert(
                    extra,
                    self.repository.blob(b"incoming\n").unwrap(),
                    0o100644,
                )
                .unwrap();
        }
        let tree = self.repository.find_tree(builder.write().unwrap()).unwrap();
        let signature =
            git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(1, 0))
                .unwrap();
        self.repository
            .commit(None, &signature, &signature, "new", &tree, &[&parent])
            .unwrap()
    }

    fn context(&self) -> (SynchronizationTarget, git2::Repository) {
        let target = SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: POLL_ID.parse().unwrap(),
        };
        let reference = self
            .repository
            .branch(
                &format!("manyhands/document/{POLL_ID}"),
                &self.repository.find_commit(self.old).unwrap(),
                false,
            )
            .unwrap()
            .into_reference();
        let path = self.root.join(".manyhands/worktrees").join(POLL_ID);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&reference));
        self.repository
            .worktree(POLL_ID, &path, Some(&options))
            .unwrap();
        (target, git2::Repository::open(path).unwrap())
    }

    fn tracking(&self, target: &SynchronizationTarget, oid: git2::Oid) {
        let name = match target {
            SynchronizationTarget::Primary => self.plan.primary().tracking_ref().to_owned(),
            SynchronizationTarget::Context { kind, item_id } => {
                self.plan.context(*kind, item_id).tracking_ref().to_owned()
            }
        };
        self.repository
            .reference(&name, oid, true, "fixture observation")
            .unwrap();
    }
}

#[derive(Debug, PartialEq, Eq)]
struct LocalSnapshot {
    files: BTreeMap<PathBuf, Vec<u8>>,
    registrations: BTreeMap<PathBuf, Vec<u8>>,
    index: Option<Vec<u8>>,
    head: Vec<u8>,
    refs: Vec<(String, String)>,
    statuses: Vec<(Vec<u8>, u32)>,
}

fn files_at(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return;
        };
        let relative = path.strip_prefix(root).unwrap().to_owned();
        if metadata.file_type().is_symlink() {
            files.insert(
                relative,
                fs::read_link(path)
                    .unwrap()
                    .as_os_str()
                    .as_encoded_bytes()
                    .to_vec(),
            );
        } else if metadata.is_file() {
            files.insert(relative, fs::read(path).unwrap());
        } else if metadata.is_dir() {
            files.insert(relative, Vec::new());
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                if entry.file_name() != ".git" || !entry.file_type().unwrap().is_dir() {
                    visit(root, &entry.path(), files);
                }
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(path, path, &mut files);
    files
}

fn snapshot(root: &Path, repository: &git2::Repository) -> LocalSnapshot {
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repository
        .statuses(Some(&mut options))
        .unwrap()
        .iter()
        .map(|e| (e.path_bytes().to_vec(), e.status().bits()))
        .collect();
    let mut refs = repository
        .references()
        .unwrap()
        .map(|r| {
            let r = r.unwrap();
            (
                r.name().unwrap().to_owned(),
                r.symbolic_target()
                    .map(str::to_owned)
                    .unwrap_or_else(|| r.target().unwrap().to_string()),
            )
        })
        .collect::<Vec<_>>();
    refs.sort();
    LocalSnapshot {
        files: files_at(root),
        registrations: files_at(&repository.commondir().join("worktrees")),
        index: fs::read(repository.path().join("index")).ok(),
        head: fs::read(repository.path().join("HEAD")).unwrap(),
        refs,
        statuses,
    }
}

#[test]
fn poll_classification_dirty_and_conflicted_states_preserve_full_snapshot() {
    for (case, expected) in [
        ("modified", PollDisposition::Dirty),
        ("staged", PollDisposition::Dirty),
        ("untracked", PollDisposition::Dirty),
        ("directory", PollDisposition::Dirty),
        ("merge", PollDisposition::Conflicted),
        ("conflict", PollDisposition::Conflicted),
        ("index_tree", PollDisposition::Dirty),
    ] {
        let f = LocalFixture::new();
        let r = &f.repository;
        match case {
            "modified" | "staged" => {
                fs::write(f.root.join("file"), b"dirty").unwrap();
                if case == "staged" {
                    let mut i = r.index().unwrap();
                    i.add_path(Path::new("file")).unwrap();
                    i.write().unwrap();
                }
            }
            "untracked" => fs::write(f.root.join("untracked"), b"untracked").unwrap(),
            "directory" => {
                fs::create_dir(f.root.join("dir")).unwrap();
                fs::write(f.root.join("dir/untracked"), b"untracked").unwrap();
            }
            "merge" => fs::write(r.path().join("MERGE_HEAD"), format!("{}\n", f.old)).unwrap(),
            "conflict" => {
                let mut i = r.index().unwrap();
                let mut e = i.get_path(Path::new("file"), 0).unwrap();
                i.remove_path(Path::new("file")).unwrap();
                e.flags = (e.flags & !0x3000) | 0x1000;
                i.add(&e).unwrap();
                i.write().unwrap();
                assert!(i.has_conflicts());
            }
            "index_tree" => {
                let mut i = r.index().unwrap();
                let mut e = i.get_path(Path::new("file"), 0).unwrap();
                e.id = r.blob(b"different index tree").unwrap();
                i.add(&e).unwrap();
                i.write().unwrap();
                assert_ne!(
                    i.write_tree().unwrap(),
                    r.find_commit(f.old).unwrap().tree_id()
                );
            }
            _ => unreachable!(),
        }
        let before = snapshot(&f.root, r);
        let result = classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(f.old),
        )
        .unwrap();
        assert_eq!(
            result,
            LocalPollClassification::Unchanged {
                disposition: expected,
                local_oid: Some(f.old)
            },
            "{case}"
        );
        assert_eq!(snapshot(&f.root, r), before, "{case}");
    }
}

#[test]
fn poll_ignored_only_is_clean_and_remote_absence_wins_over_dirty() {
    let f = LocalFixture::new();
    fs::write(f.root.join("ignored"), b"local ignored").unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(f.old)
        )
        .unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::Current,
            local_oid: Some(f.old)
        }
    );
    assert_eq!(snapshot(&f.root, &f.repository), before);
    let (target, context) = f.context();
    fs::write(context.workdir().unwrap().join("file"), b"dirty").unwrap();
    for (state, disposition) in [
        (
            PollTrackingState::RemotelyDeleted,
            PollDisposition::RemotelyDeleted,
        ),
        (
            PollTrackingState::HistoryUnknown,
            PollDisposition::HistoryUnknown,
        ),
    ] {
        let before = snapshot(&f.root, &context);
        assert_eq!(
            classify_local_target(&f.root, "main", &target, state).unwrap(),
            LocalPollClassification::Unchanged {
                disposition,
                local_oid: None
            }
        );
        assert_eq!(snapshot(&f.root, &context), before);
    }
}

#[test]
fn poll_wrong_or_detached_head_and_primary_not_at_root_are_inaccessible() {
    for case in ["detached", "wrong", "primary_elsewhere"] {
        let f = LocalFixture::new();
        if case == "detached" {
            f.repository.set_head_detached(f.old).unwrap();
        } else {
            f.repository
                .branch("wrong", &f.repository.find_commit(f.old).unwrap(), false)
                .unwrap();
            f.repository.set_head("refs/heads/wrong").unwrap();
        }
        if case == "primary_elsewhere" {
            let reference = f.repository.find_reference("refs/heads/main").unwrap();
            let path = f.root.join(".manyhands/worktrees/primary");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut options = git2::WorktreeAddOptions::new();
            options.reference(Some(&reference));
            f.repository
                .worktree("primary", &path, Some(&options))
                .unwrap();
        }
        let before = snapshot(&f.root, &f.repository);
        assert_eq!(
            classify_local_target(
                &f.root,
                "main",
                &SynchronizationTarget::Primary,
                PollTrackingState::Present(f.old)
            )
            .unwrap(),
            LocalPollClassification::Unchanged {
                disposition: PollDisposition::Inaccessible,
                local_oid: None
            }
        );
        assert_eq!(snapshot(&f.root, &f.repository), before);
    }
}

#[test]
fn poll_missing_and_unregistered_contexts_are_inaccessible_and_unchanged() {
    for case in ["missing", "unregistered", "wrong", "detached"] {
        let f = LocalFixture::new();
        let (target, context) = f.context();
        let path = context.workdir().unwrap().to_owned();
        if case == "missing" {
            fs::rename(&path, f.root.join("kept")).unwrap();
        }
        if case == "unregistered" {
            fs::rename(
                f.repository.path().join("worktrees").join(POLL_ID),
                f.repository.path().join("kept-registration"),
            )
            .unwrap();
        }
        if case == "wrong" {
            f.repository
                .branch("wrong", &f.repository.find_commit(f.old).unwrap(), false)
                .unwrap();
            context.set_head("refs/heads/wrong").unwrap();
        }
        if case == "detached" {
            context.set_head_detached(f.old).unwrap();
        }
        let before = snapshot(&f.root, &f.repository);
        assert_eq!(
            classify_local_target(&f.root, "main", &target, PollTrackingState::Present(f.old))
                .unwrap(),
            LocalPollClassification::Unchanged {
                disposition: PollDisposition::Inaccessible,
                local_oid: None
            },
            "{case}"
        );
        assert_eq!(snapshot(&f.root, &f.repository), before, "{case}");
    }
}

#[cfg(unix)]
#[test]
fn poll_symlink_deterministic_path_is_inaccessible_without_following_it() {
    let f = LocalFixture::new();
    let (target, _context) = f.context();
    let path = f.root.join(".manyhands/worktrees").join(POLL_ID);
    let kept = f.root.join("kept");
    fs::rename(&path, &kept).unwrap();
    std::os::unix::fs::symlink(&kept, &path).unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        classify_local_target(&f.root, "main", &target, PollTrackingState::Present(f.old)).unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::Inaccessible,
            local_oid: None
        }
    );
    assert_eq!(snapshot(&f.root, &f.repository), before);
    assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn poll_fast_forward_primary_and_context_advance_files_index_branch_then_are_current() {
    for is_context in [false, true] {
        let f = LocalFixture::new();
        let (target, linked) = if is_context {
            f.context()
        } else {
            (
                SynchronizationTarget::Primary,
                git2::Repository::open(&f.root).unwrap(),
            )
        };
        let incoming = f.incoming(f.old, None);
        f.tracking(&target, incoming);
        let expected = f
            .service
            .observation_configuration(&f.root, &f.plan)
            .unwrap();
        assert_eq!(
            classify_local_target(
                &f.root,
                "main",
                &target,
                PollTrackingState::Present(incoming)
            )
            .unwrap(),
            LocalPollClassification::FastForward {
                local_oid: f.old,
                tracking_oid: incoming
            }
        );
        let update = PollFastForward {
            target: target.clone(),
            before: f.old,
            after: incoming,
        };
        assert_eq!(
            f.service
                .poll_fast_forward(&f.root, &f.plan, &expected, &update)
                .unwrap(),
            PollDisposition::FastForwarded
        );
        assert_eq!(linked.head().unwrap().target(), Some(incoming));
        assert_eq!(
            linked.index().unwrap().write_tree().unwrap(),
            linked.find_commit(incoming).unwrap().tree_id()
        );
        assert_eq!(
            fs::read(linked.workdir().unwrap().join("file")).unwrap(),
            b"new\n"
        );
        let before = snapshot(&f.root, &linked);
        let update = PollFastForward {
            target: target.clone(),
            before: incoming,
            after: incoming,
        };
        assert_eq!(
            f.service
                .poll_fast_forward(&f.root, &f.plan, &expected, &update)
                .unwrap(),
            PollDisposition::Current
        );
        assert_eq!(snapshot(&f.root, &linked), before);
        if is_context {
            assert_eq!(f.repository.head().unwrap().target(), Some(f.old));
        }
    }
}

#[test]
fn poll_fast_forward_reobserves_ref_tracking_configuration_and_endpoint_before_mutation() {
    for case in ["local", "tracking", "configuration", "endpoint", "dirty"] {
        let f = LocalFixture::new();
        let target = SynchronizationTarget::Primary;
        let incoming = f.incoming(f.old, None);
        f.tracking(&target, incoming);
        let expected = f
            .service
            .observation_configuration(&f.root, &f.plan)
            .unwrap();
        assert!(matches!(
            classify_local_target(
                &f.root,
                "main",
                &target,
                PollTrackingState::Present(incoming)
            )
            .unwrap(),
            LocalPollClassification::FastForward { .. }
        ));
        match case {
            "local" => {
                f.repository
                    .reference("refs/heads/main", incoming, true, "external change")
                    .unwrap();
            }
            "tracking" => f.tracking(&target, f.old),
            "configuration" => fs::write(
                f.root.join(".manyhands/config.toml"),
                "format_version = 1\nprimary_branch = \"other\"\npublication_remote = \"origin\"\n",
            )
            .unwrap(),
            "endpoint" => f
                .repository
                .remote_set_url("origin", "ssh://other@example.invalid/repo")
                .unwrap(),
            "dirty" => fs::write(f.root.join("file"), b"dirty").unwrap(),
            _ => unreachable!(),
        }
        let before = snapshot(&f.root, &f.repository);
        let result = f.service.poll_fast_forward(
            &f.root,
            &f.plan,
            &expected,
            &PollFastForward {
                target,
                before: f.old,
                after: incoming,
            },
        );
        if case == "dirty" {
            assert_eq!(result.unwrap(), PollDisposition::Dirty);
        } else {
            assert!(
                matches!(result, Err(SynchronizationError::ExternalChange)),
                "{case}: {result:?}"
            );
        }
        assert_eq!(snapshot(&f.root, &f.repository), before, "{case}");
    }
}

#[test]
fn poll_ignored_incoming_collision_is_dirty_and_byte_exact_unchanged() {
    let f = LocalFixture::new();
    let incoming = f.incoming(f.old, Some("ignored"));
    let target = SynchronizationTarget::Primary;
    f.tracking(&target, incoming);
    fs::write(f.root.join("ignored"), b"ignored sentinel").unwrap();
    let expected = f
        .service
        .observation_configuration(&f.root, &f.plan)
        .unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        f.service
            .poll_fast_forward(
                &f.root,
                &f.plan,
                &expected,
                &PollFastForward {
                    target,
                    before: f.old,
                    after: incoming
                }
            )
            .unwrap(),
        PollDisposition::Dirty
    );
    assert_eq!(snapshot(&f.root, &f.repository), before);
}

fn item_source(kind: AuthoringKind, id: &str) -> Vec<u8> {
    let (kind, extra) = match kind {
        AuthoringKind::Document => ("document", ""),
        AuthoringKind::Ticket => ("ticket", "type: task\nstatus: open\n"),
    };
    format!("---\nmanyhands_managed: true\nmanyhands_kind: {kind}\nid: {id}\ntitle: Fixture\n{extra}---\n").into_bytes()
}

// Build nested trees entirely in the ODB, including modes not produced by staging.
type FixtureTreeEntries<'a> = Vec<(Vec<&'a str>, git2::Oid, i32)>;

fn fixture_tree(repository: &git2::Repository, entries: &[(&str, Vec<u8>, i32)]) -> git2::Oid {
    fn build(repository: &git2::Repository, entries: &[(Vec<&str>, git2::Oid, i32)]) -> git2::Oid {
        let mut builder = repository.treebuilder(None).unwrap();
        let mut dirs: BTreeMap<&str, FixtureTreeEntries<'_>> = BTreeMap::new();
        for (path, oid, mode) in entries {
            if path.len() == 1 {
                builder.insert(path[0], *oid, *mode).unwrap();
            } else {
                dirs.entry(path[0])
                    .or_default()
                    .push((path[1..].to_vec(), *oid, *mode));
            }
        }
        for (name, entries) in dirs {
            if name == ".git" {
                // TreeBuilder protects this name. A fetched hostile tree can
                // still hold it; create that test object directly in the ODB.
                assert!(builder.is_empty());
                let mut bytes = b"40000 .git\0".to_vec();
                bytes.extend_from_slice(build(repository, &entries).as_bytes());
                return repository
                    .odb()
                    .unwrap()
                    .write(git2::ObjectType::Tree, &bytes)
                    .unwrap();
            }
            builder
                .insert(name, build(repository, &entries), 0o040000)
                .unwrap();
        }
        builder.write().unwrap()
    }
    let entries = entries
        .iter()
        .map(|(path, bytes, mode)| {
            let oid = if *mode == 0o160000 {
                repository.head().unwrap().target().unwrap()
            } else {
                repository.blob(bytes).unwrap()
            };
            (path.split('/').collect(), oid, *mode)
        })
        .collect::<Vec<_>>();
    build(repository, &entries)
}

#[test]
fn poll_tree_conformity_matrix_rejects_without_creation_and_all_accepted_are_discoverable() {
    for case in [
        "document",
        "ticket",
        "invalid_comment",
        "duplicate",
        "ticket_id",
        "kind",
        "missing",
        "frontmatter",
        "utf8",
        "executable",
        "symlink",
        "gitlink",
        "worktrees",
        "worktrees_blob",
    ] {
        let f = LocalFixture::new();
        let kind = if case == "ticket" || case == "ticket_id" {
            AuthoringKind::Ticket
        } else {
            AuthoringKind::Document
        };
        let path = if kind == AuthoringKind::Ticket {
            format!(".manyhands/tickets/{POLL_ID}/ticket.md")
        } else {
            "docs/item.md".into()
        };
        let source = match case {
            "ticket_id" => item_source(kind, OTHER_ID),
            "kind" => item_source(AuthoringKind::Ticket, POLL_ID),
            "frontmatter" => b"---\n[broken\n---\n".to_vec(),
            "utf8" => vec![255],
            "symlink" => b"../other".to_vec(),
            _ => item_source(kind, POLL_ID),
        };
        let mode = match case {
            "executable" => 0o100755,
            "symlink" => 0o120000,
            "gitlink" => 0o160000,
            _ => 0o100644,
        };
        let mut entries = if case == "missing" {
            Vec::new()
        } else {
            vec![(path.as_str(), source, mode)]
        };
        match case {
            "duplicate" => entries.push(("docs/second.md", item_source(kind, POLL_ID), 0o100644)),
            "worktrees" => entries.push((
                ".manyhands/worktrees/forbidden/file",
                b"forbidden".to_vec(),
                0o100644,
            )),
            "worktrees_blob" => {
                entries.push((".manyhands/worktrees", b"forbidden".to_vec(), 0o100644))
            }
            "invalid_comment" => entries.push((
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAW.md",
                b"---\n[broken\n---\n".to_vec(),
                0o100644,
            )),
            _ => {}
        }
        let tree_id = fixture_tree(&f.repository, &entries);
        let tree = f.repository.find_tree(tree_id).unwrap();
        let expected = matches!(case, "document" | "ticket" | "invalid_comment");
        let before = snapshot(&f.root, &f.repository);
        assert_eq!(
            conforming_context_tree(&f.repository, &tree, kind, &POLL_ID.parse().unwrap()).unwrap(),
            expected,
            "{case}"
        );
        assert_eq!(
            snapshot(&f.root, &f.repository),
            before,
            "validation creates nothing: {case}"
        );
        assert!(
            f.repository
                .find_branch(
                    &format!(
                        "manyhands/{}/{POLL_ID}",
                        if kind == AuthoringKind::Document {
                            "document"
                        } else {
                            "ticket"
                        }
                    ),
                    git2::BranchType::Local
                )
                .is_err()
        );
        assert!(f.repository.find_worktree(POLL_ID).is_err());
        assert!(!f.root.join(".manyhands/worktrees").join(POLL_ID).exists());
        if expected {
            let signature =
                git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(1, 0))
                    .unwrap();
            let oid = f
                .repository
                .commit(None, &signature, &signature, "incoming", &tree, &[])
                .unwrap();
            let commit = f.repository.find_commit(oid).unwrap();
            let branch = format!(
                "manyhands/{}/{POLL_ID}",
                if kind == AuthoringKind::Document {
                    "document"
                } else {
                    "ticket"
                }
            );
            let reference = f
                .repository
                .branch(&branch, &commit, false)
                .unwrap()
                .into_reference();
            let path = f.root.join(".manyhands/worktrees").join(POLL_ID);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut options = git2::WorktreeAddOptions::new();
            options.reference(Some(&reference));
            f.repository
                .worktree(POLL_ID, &path, Some(&options))
                .unwrap();
            let observation = crate::repository::discovery::observe_root(&f.repository, &f.root);
            assert_eq!(observation.active_contexts.len(), 1, "{case}");
            assert_eq!(
                observation.active_contexts[0].context.branch.as_deref(),
                Some(branch.as_str())
            );
            if case == "invalid_comment" {
                assert!(
                    !observation.active_contexts[0]
                        .validation
                        .problems
                        .is_empty()
                );
            }
        }
    }
}

#[test]
fn poll_conformity_cannot_hide_executable_duplicate_or_discovery_excluded_item() {
    let f = LocalFixture::new();
    for entries in [
        vec![
            (
                "docs/item.md",
                item_source(AuthoringKind::Document, POLL_ID),
                0o100644,
            ),
            (
                "docs/duplicate.md",
                item_source(AuthoringKind::Document, POLL_ID),
                0o100755,
            ),
        ],
        vec![(
            "docs/.git/item.md",
            item_source(AuthoringKind::Document, POLL_ID),
            0o100644,
        )],
    ] {
        let tree = f
            .repository
            .find_tree(fixture_tree(&f.repository, &entries))
            .unwrap();
        let before = snapshot(&f.root, &f.repository);
        assert!(
            !conforming_context_tree(
                &f.repository,
                &tree,
                AuthoringKind::Document,
                &POLL_ID.parse().unwrap()
            )
            .unwrap()
        );
        assert_eq!(snapshot(&f.root, &f.repository), before);
    }
}

#[test]
fn poll_classifies_actual_ahead_diverged_and_missing_ancestry_without_mutation() {
    let f = LocalFixture::new();
    let child = f.incoming(f.old, None);
    f.repository
        .reference("refs/heads/main", child, true, "fixture local commit")
        .unwrap();
    f.repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let before = snapshot(&f.root, &f.repository);
    assert_eq!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(f.old)
        )
        .unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::LocalAhead,
            local_oid: Some(child)
        }
    );
    let mut builder = f
        .repository
        .treebuilder(Some(
            &f.repository.find_commit(f.old).unwrap().tree().unwrap(),
        ))
        .unwrap();
    builder
        .insert("peer", f.repository.blob(b"peer").unwrap(), 0o100644)
        .unwrap();
    let tree = f.repository.find_tree(builder.write().unwrap()).unwrap();
    let sig =
        git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0)).unwrap();
    let peer = f
        .repository
        .commit(
            None,
            &sig,
            &sig,
            "peer",
            &tree,
            &[&f.repository.find_commit(f.old).unwrap()],
        )
        .unwrap();
    assert_eq!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(peer)
        )
        .unwrap(),
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::Diverged,
            local_oid: Some(child)
        }
    );
    assert!(matches!(
        classify_local_target(
            &f.root,
            "main",
            &SynchronizationTarget::Primary,
            PollTrackingState::Present(git2::Oid::from_bytes(&[99; 20]).unwrap())
        ),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(snapshot(&f.root, &f.repository), before);
}

#[test]
fn poll_fast_forward_allows_clean_tracked_file_to_directory_transition() {
    let f = LocalFixture::new();
    let parent = f.repository.find_commit(f.old).unwrap();
    let mut directory = f.repository.treebuilder(None).unwrap();
    directory
        .insert("nested", f.repository.blob(b"nested\n").unwrap(), 0o100644)
        .unwrap();
    let mut builder = f
        .repository
        .treebuilder(Some(&parent.tree().unwrap()))
        .unwrap();
    builder.remove("file").unwrap();
    builder
        .insert("file", directory.write().unwrap(), 0o040000)
        .unwrap();
    let tree = f.repository.find_tree(builder.write().unwrap()).unwrap();
    let sig =
        git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0)).unwrap();
    let incoming = f
        .repository
        .commit(None, &sig, &sig, "file to directory", &tree, &[&parent])
        .unwrap();
    f.tracking(&SynchronizationTarget::Primary, incoming);
    let expected = f
        .service
        .observation_configuration(&f.root, &f.plan)
        .unwrap();
    assert_eq!(
        f.service
            .poll_fast_forward(
                &f.root,
                &f.plan,
                &expected,
                &PollFastForward {
                    target: SynchronizationTarget::Primary,
                    before: f.old,
                    after: incoming
                }
            )
            .unwrap(),
        PollDisposition::FastForwarded
    );
    assert_eq!(fs::read(f.root.join("file/nested")).unwrap(), b"nested\n");
    assert_eq!(f.repository.head().unwrap().target(), Some(incoming));
    let mut index = f.repository.index().unwrap();
    index.read(true).unwrap(); // Reobserve disk after the updater's separate handle.
    assert_eq!(index.write_tree().unwrap(), tree.id());
}

fn retiring_directory_fixture(
    nested: bool,
    incoming_mode: i32,
) -> (LocalFixture, PathBuf, git2::Oid) {
    let mut f = LocalFixture::new();
    let child_path = if nested { "dir/sub/child" } else { "dir/child" };
    let deep_child = if nested {
        "dir/sub/tracked/deep/child"
    } else {
        "dir/tracked/deep/child"
    };
    let dirs = f
        .repository
        .find_tree(fixture_tree(
            &f.repository,
            &[
                (child_path, b"tracked child\n".to_vec(), 0o100644),
                (deep_child, b"deep tracked child\n".to_vec(), 0o100644),
            ],
        ))
        .unwrap();
    let parent = f.repository.find_commit(f.old).unwrap();
    let mut builder = f
        .repository
        .treebuilder(Some(&parent.tree().unwrap()))
        .unwrap();
    builder
        .insert("dir", dirs.get_name("dir").unwrap().id(), 0o040000)
        .unwrap();
    builder
        .insert(
            "afile",
            f.repository.blob(b"old afile\n").unwrap(),
            0o100644,
        )
        .unwrap();
    builder
        .insert(
            ".gitignore",
            f.repository
                .blob(b".manyhands/worktrees/\nignored\n*.ignored\n")
                .unwrap(),
            0o100644,
        )
        .unwrap();
    let tree = f.repository.find_tree(builder.write().unwrap()).unwrap();
    let sig =
        git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0)).unwrap();
    f.old = f
        .repository
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "tracked directory",
            &tree,
            &[&parent],
        )
        .unwrap();
    drop(builder);
    f.repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let incoming = f
        .repository
        .blob(if incoming_mode == 0o120000 {
            b"file"
        } else {
            b"incoming file\n"
        })
        .unwrap();
    let mut builder = f.repository.treebuilder(Some(&tree)).unwrap();
    if nested {
        let old_dir = f
            .repository
            .find_tree(tree.get_name("dir").unwrap().id())
            .unwrap();
        let mut dir = f.repository.treebuilder(Some(&old_dir)).unwrap();
        dir.insert("sub", incoming, incoming_mode).unwrap();
        builder
            .insert("dir", dir.write().unwrap(), 0o040000)
            .unwrap();
    } else {
        builder.insert("dir", incoming, incoming_mode).unwrap();
    }
    builder
        .insert(
            "afile",
            f.repository.blob(b"changed afile\n").unwrap(),
            0o100644,
        )
        .unwrap();
    let incoming_tree = f.repository.find_tree(builder.write().unwrap()).unwrap();
    let incoming = f
        .repository
        .commit(
            None,
            &sig,
            &sig,
            "directory transition",
            &incoming_tree,
            &[&f.repository.find_commit(f.old).unwrap()],
        )
        .unwrap();
    f.tracking(&SynchronizationTarget::Primary, incoming);
    let retiring = f.root.join(if nested { "dir/sub" } else { "dir" });
    drop((builder, incoming_tree, tree, parent, dirs));
    (f, retiring, incoming)
}

#[test]
fn poll_retiring_directory_ignored_descendants_are_dirty_before_any_checkout() {
    let modes: &[i32] = if cfg!(unix) {
        &[0o100644, 0o120000]
    } else {
        &[0o100644]
    };
    let obstructions: &[&str] = if cfg!(unix) {
        &[
            "file",
            "nested_file",
            "tracked_descendant",
            "empty_directory",
            "symlink",
        ]
    } else {
        &[
            "file",
            "nested_file",
            "tracked_descendant",
            "empty_directory",
        ]
    };
    for nested in [false, true] {
        for &mode in modes {
            for &obstruction in obstructions {
                let (f, retiring, incoming) = retiring_directory_fixture(nested, mode);
                let outside = f._data.path().join("outside-sentinel");
                fs::write(&outside, b"outside sentinel\n").unwrap();
                let ignored = retiring.join("local.ignored");
                match obstruction {
                    "file" => fs::write(&ignored, b"ignored sentinel\n").unwrap(),
                    "nested_file" => {
                        fs::create_dir(&ignored).unwrap();
                        fs::write(ignored.join("sentinel"), b"ignored nested sentinel\n").unwrap();
                    }
                    "tracked_descendant" => fs::write(
                        retiring.join("tracked/deep/local.ignored"),
                        b"deep ignored sentinel\n",
                    )
                    .unwrap(),
                    "empty_directory" => fs::create_dir(&ignored).unwrap(),
                    #[cfg(unix)]
                    "symlink" => std::os::unix::fs::symlink(&outside, &ignored).unwrap(),
                    _ => unreachable!(),
                }
                let expected = f
                    .service
                    .observation_configuration(&f.root, &f.plan)
                    .unwrap();
                let before = snapshot(&f.root, &f.repository);
                assert!(before.statuses.is_empty());
                let result = f.service.poll_fast_forward(
                    &f.root,
                    &f.plan,
                    &expected,
                    &PollFastForward {
                        target: SynchronizationTarget::Primary,
                        before: f.old,
                        after: incoming,
                    },
                );
                assert_eq!(
                    snapshot(&f.root, &f.repository),
                    before,
                    "checkout mutated state: nested={nested}, mode={mode:o}, obstruction={obstruction}, result={result:?}"
                );
                assert_eq!(result.unwrap(), PollDisposition::Dirty);
                assert_eq!(fs::read(&outside).unwrap(), b"outside sentinel\n");
            }
        }
    }
}

#[test]
fn poll_clean_retiring_directories_still_fast_forward_with_noncolliding_ignored_state() {
    let modes: &[i32] = if cfg!(unix) {
        &[0o100644, 0o120000]
    } else {
        &[0o100644]
    };
    for nested in [false, true] {
        for &mode in modes {
            let (f, retiring, incoming) = retiring_directory_fixture(nested, mode);
            fs::write(f.root.join("ignored"), b"noncolliding ignored\n").unwrap();
            let expected = f
                .service
                .observation_configuration(&f.root, &f.plan)
                .unwrap();
            assert_eq!(
                f.service
                    .poll_fast_forward(
                        &f.root,
                        &f.plan,
                        &expected,
                        &PollFastForward {
                            target: SynchronizationTarget::Primary,
                            before: f.old,
                            after: incoming
                        }
                    )
                    .unwrap(),
                PollDisposition::FastForwarded
            );
            assert_eq!(f.repository.head().unwrap().target(), Some(incoming));
            assert_eq!(fs::read(f.root.join("afile")).unwrap(), b"changed afile\n");
            assert_eq!(
                fs::read(f.root.join("ignored")).unwrap(),
                b"noncolliding ignored\n"
            );
            assert!(!retiring.join("child").exists());
            if mode == 0o100644 {
                assert_eq!(fs::read(&retiring).unwrap(), b"incoming file\n");
            }
            #[cfg(unix)]
            if mode == 0o120000 {
                assert_eq!(fs::read_link(&retiring).unwrap(), Path::new("file"));
            }
            let mut index = f.repository.index().unwrap();
            index.read(true).unwrap();
            assert_eq!(
                index.write_tree().unwrap(),
                f.repository.find_commit(incoming).unwrap().tree_id()
            );
        }
    }
}

fn copy_fixture_objects(source: &git2::Repository, destination: &git2::Repository) {
    let source = source.odb().unwrap();
    let destination = destination.odb().unwrap();
    source
        .foreach(|oid| {
            let object = source.read(*oid).unwrap();
            assert_eq!(
                destination.write(object.kind(), object.data()).unwrap(),
                *oid
            );
            true
        })
        .unwrap();
}

fn substituted_context_is_inaccessible(case: &str) {
    let f = LocalFixture::new();
    f.repository
        .config()
        .unwrap()
        .set_str("user.name", "Fixture")
        .unwrap();
    f.repository
        .config()
        .unwrap()
        .set_str("user.email", "fixture@example.invalid")
        .unwrap();
    let (target, original) = f.context();
    let incoming = f.incoming(f.old, None);
    f.tracking(&target, incoming);
    let path = f.root.join(".manyhands/worktrees").join(POLL_ID);
    let branch = format!("manyhands/document/{POLL_ID}");
    let foreign_root = tempfile::tempdir().unwrap();
    let foreign_root_path = foreign_root.path().canonicalize().unwrap();
    let mut init = git2::RepositoryInitOptions::new();
    init.initial_head("main");
    let foreign_parent = git2::Repository::init_opts(&foreign_root_path, &init).unwrap();
    foreign_parent
        .config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    foreign_parent
        .config()
        .unwrap()
        .set_str("user.name", "Fixture")
        .unwrap();
    foreign_parent
        .config()
        .unwrap()
        .set_str("user.email", "fixture@example.invalid")
        .unwrap();
    copy_fixture_objects(&f.repository, &foreign_parent);
    foreign_parent
        .reference("refs/heads/main", f.old, false, "fixture")
        .unwrap();
    foreign_parent
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    drop(original);
    let linked = if case == "backlink" {
        let gitfile = path.join(".git");
        assert!(gitfile.is_file());
        fs::write(path.join("ignored"), b"ignored backlink sentinel\n").unwrap();
        fs::write(
            f.repository
                .commondir()
                .join("worktrees")
                .join(POLL_ID)
                .join("gitdir"),
            format!("{}\n", path.join("ignored").display()),
        )
        .unwrap();
        git2::Repository::open(&path).unwrap()
    } else {
        fs::rename(&path, path.parent().unwrap().join("kept")).unwrap();
        match case {
            "standalone" => {
                let mut init = git2::RepositoryInitOptions::new();
                init.initial_head(&branch);
                let repo = git2::Repository::init_opts(&path, &init).unwrap();
                repo.config()
                    .unwrap()
                    .set_bool("core.autocrlf", false)
                    .unwrap();
                repo.config()
                    .unwrap()
                    .set_str("user.name", "Fixture")
                    .unwrap();
                repo.config()
                    .unwrap()
                    .set_str("user.email", "fixture@example.invalid")
                    .unwrap();
                copy_fixture_objects(&f.repository, &repo);
                repo.reference(&format!("refs/heads/{branch}"), f.old, false, "fixture")
                    .unwrap();
                repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                    .unwrap();
                assert!(!repo.is_worktree());
                repo
            }
            "foreign_linked" => {
                let reference = foreign_parent
                    .branch(&branch, &foreign_parent.find_commit(f.old).unwrap(), false)
                    .unwrap()
                    .into_reference();
                let mut options = git2::WorktreeAddOptions::new();
                options.reference(Some(&reference));
                foreign_parent
                    .worktree("foreign", &path, Some(&options))
                    .unwrap();
                let repo = git2::Repository::open(&path).unwrap();
                assert!(repo.is_worktree());
                assert_ne!(
                    repo.commondir().canonicalize().unwrap(),
                    f.repository.commondir().canonicalize().unwrap()
                );
                repo
            }
            "same_common_admin" => {
                let other_path = path.parent().unwrap().join(OTHER_ID);
                let reference = f
                    .repository
                    .branch("other", &f.repository.find_commit(f.old).unwrap(), false)
                    .unwrap()
                    .into_reference();
                let mut options = git2::WorktreeAddOptions::new();
                options.reference(Some(&reference));
                f.repository
                    .worktree(OTHER_ID, &other_path, Some(&options))
                    .unwrap();
                let other = git2::Repository::open(&other_path).unwrap();
                other.set_head(&format!("refs/heads/{branch}")).unwrap();
                fs::create_dir(&path).unwrap();
                fs::write(
                    path.join(".git"),
                    fs::read(other_path.join(".git")).unwrap(),
                )
                .unwrap();
                fs::write(
                    other.path().join("gitdir"),
                    format!("{}\n", path.join(".git").display()),
                )
                .unwrap();
                let repo = git2::Repository::open(&path).unwrap();
                repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                    .unwrap();
                assert_eq!(
                    repo.commondir().canonicalize().unwrap(),
                    f.repository.commondir().canonicalize().unwrap()
                );
                assert_ne!(
                    repo.path().canonicalize().unwrap(),
                    f.repository
                        .commondir()
                        .join("worktrees")
                        .join(POLL_ID)
                        .canonicalize()
                        .unwrap()
                );
                repo
            }
            _ => unreachable!(),
        }
    };
    assert_eq!(
        f.repository
            .find_worktree(POLL_ID)
            .unwrap()
            .path()
            .canonicalize()
            .unwrap(),
        path.canonicalize().unwrap(),
        "original root registration kept"
    );
    assert_eq!(
        linked.workdir().unwrap().canonicalize().unwrap(),
        path.canonicalize().unwrap()
    );
    assert_eq!(
        linked.find_reference("HEAD").unwrap().symbolic_target(),
        Some(format!("refs/heads/{branch}").as_str())
    );
    assert_eq!(linked.head().unwrap().target(), Some(f.old));
    assert!(
        linked.find_commit(incoming).is_ok(),
        "matching incoming objects exist"
    );
    let expected = f
        .service
        .observation_configuration(&f.root, &f.plan)
        .unwrap();
    let root_before = snapshot(&f.root, &f.repository);
    let foreign_before = snapshot(&path, &linked);
    let parent_before = snapshot(&foreign_root_path, &foreign_parent);
    let classification = classify_local_target(
        &f.root,
        "main",
        &target,
        PollTrackingState::Present(incoming),
    )
    .unwrap();
    let result = f.service.poll_fast_forward(
        &f.root,
        &f.plan,
        &expected,
        &PollFastForward {
            target,
            before: f.old,
            after: incoming,
        },
    );
    assert_eq!(
        snapshot(&f.root, &f.repository),
        root_before,
        "root changed: {case}, result={result:?}"
    );
    assert_eq!(
        snapshot(&path, &linked),
        foreign_before,
        "foreign target changed: {case}"
    );
    assert_eq!(
        snapshot(&foreign_root_path, &foreign_parent),
        parent_before,
        "foreign owner changed: {case}"
    );
    assert_eq!(
        classification,
        LocalPollClassification::Unchanged {
            disposition: PollDisposition::Inaccessible,
            local_oid: None
        }
    );
    assert_eq!(result.unwrap(), PollDisposition::Inaccessible);
}

#[test]
fn poll_context_identity_rejects_standalone_substitution_without_mutating_either_repo() {
    substituted_context_is_inaccessible("standalone");
}

#[test]
fn poll_context_identity_rejects_foreign_linked_substitution_without_mutating_either_repo() {
    substituted_context_is_inaccessible("foreign_linked");
}

#[test]
fn poll_context_identity_requires_exact_registered_admin_and_gitfile_backlink() {
    for case in ["same_common_admin", "backlink"] {
        substituted_context_is_inaccessible(case);
    }
}

#[test]
fn poll_document_selection_matches_discovery_extensions_including_dotfiles() {
    for (path, extra, expected) in [
        ("docs/.md", None, false),
        ("docs/nested/.md", None, false),
        ("docs/.hidden.md", None, true),
        ("docs/nested/.hidden.md", None, true),
        ("docs/visible.md", Some("docs/.md"), true),
        ("docs/visible.md", Some("docs/nested/.md"), true),
    ] {
        let f = LocalFixture::new();
        let mut entries = vec![(
            path,
            item_source(AuthoringKind::Document, POLL_ID),
            0o100644,
        )];
        if let Some(extra) = extra {
            entries.push((
                extra,
                item_source(AuthoringKind::Document, POLL_ID),
                0o100644,
            ));
        }
        let tree = f
            .repository
            .find_tree(fixture_tree(&f.repository, &entries))
            .unwrap();
        let before = snapshot(&f.root, &f.repository);
        let accepted = conforming_context_tree(
            &f.repository,
            &tree,
            AuthoringKind::Document,
            &POLL_ID.parse().unwrap(),
        )
        .unwrap();
        assert_eq!(snapshot(&f.root, &f.repository), before);
        assert_eq!(accepted, expected, "{path}, extra={extra:?}");
        assert!(f.repository.find_worktree(POLL_ID).is_err());
        assert!(!f.root.join(".manyhands/worktrees").join(POLL_ID).exists());
        if accepted {
            let sig =
                git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(2, 0))
                    .unwrap();
            let oid = f
                .repository
                .commit(None, &sig, &sig, "document selection", &tree, &[])
                .unwrap();
            let reference = f
                .repository
                .branch(
                    &format!("manyhands/document/{POLL_ID}"),
                    &f.repository.find_commit(oid).unwrap(),
                    false,
                )
                .unwrap()
                .into_reference();
            let path = f.root.join(".manyhands/worktrees").join(POLL_ID);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut options = git2::WorktreeAddOptions::new();
            options.reference(Some(&reference));
            f.repository
                .worktree(POLL_ID, &path, Some(&options))
                .unwrap();
            let observed = crate::repository::discovery::observe_root(&f.repository, &f.root);
            assert_eq!(observed.active_contexts.len(), 1);
            assert_eq!(
                observed.active_contexts[0].context.item_id,
                Some(POLL_ID.parse().unwrap())
            );
        }
    }
}
