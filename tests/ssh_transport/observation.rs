use crate::{
    repository::{transport::*, *},
    session::*,
    ssh_remote::*,
};

pub const CASES: &[crate::ssh_harness::Case] = &[
    ("observe_complete_and_replay", complete_and_replay),
    ("observe_unlock_session", unlock_session),
    ("observe_host_and_key_failures", host_and_key_failures),
    ("observe_safe_points", safe_points),
    ("observe_database_recovery", database_recovery),
    ("observe_transport_failure", transport_failure),
    ("observe_cache_invalidation", cache_invalidation),
    ("observe_missing_remote", missing_remote),
    ("observe_malformed_redaction", malformed_redaction),
    ("observe_manual_priority", manual_priority),
    ("observe_automatic_backoff", automatic_backoff),
    ("observe_block_reset", block_reset),
    ("observe_configuration_race", configuration_race),
    ("observe_endpoint_race", endpoint_race),
    ("observe_selection_race", selection_race),
    ("observe_selection_lifecycle", selection_lifecycle),
    ("observe_selection_replacement", selection_replacement),
    (
        "observe_shared_block_config_failure",
        shared_block_config_failure,
    ),
    (
        "observe_shared_block_preflight_failure",
        shared_block_preflight_failure,
    ),
    ("observe_explicit_cancel_backoff", explicit_cancel_backoff),
    ("observe_automatic_cancel_backoff", automatic_cancel_backoff),
];

fn setup(encrypted: bool) -> Result<Case, FixtureError> {
    let case = Case::new(encrypted)?;
    crate::failures::seed(&case)?;
    let db = fixed(rusqlite::Connection::open(
        case.directory.path().join("data").join(REGISTRY_FILE),
    ))?;
    fixed(db.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)", [case.root.to_str().unwrap()]))?;
    Ok(case)
}
fn request(case: &Case) -> ObservePublicationRemoteRequest {
    ObservePublicationRemoteRequest {
        root: case.root.clone(),
        operation_id: OperationId::new(),
        invocation: RemotePollInvocation::Explicit,
        approval: case.request().approval,
        restart: false,
    }
}

// Catches partial publication, unintended Git mutations, and replay re-advertising.
fn complete_and_replay() -> Result<(), FixtureError> {
    let case = setup(false)?;
    let remote = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let oid = fixed(remote.refname_to_id("refs/heads/main"))?;
    fixed(remote.reference(
        "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV",
        oid,
        true,
        "fixture",
    ))?;
    let local = fixed(git2::Repository::open(&case.root))?;
    let local_oid = fixed(local.head())?.target().unwrap();
    fixed(local.reference("refs/remotes/origin/main", local_oid, true, "fixture"))?;
    let before = crate::failures::Preservation::capture(&case)?;
    let (mut session, requests) = session(vec![]);
    let req = request(&case);
    let result = fixed(
        case.service
            .observe_publication_remote(req.clone(), &mut session),
    )?;
    assert_eq!(result.category(), RemoteOutcomeCategory::Completed);
    assert_eq!(result.snapshot().observations().len(), 2);
    let primary = result
        .snapshot()
        .observations()
        .iter()
        .find(|r| r.remote_ref() == Some("refs/heads/main"))
        .unwrap();
    assert_eq!(primary.advertised_oid(), oid);
    assert_eq!(primary.tracking_oid(), Some(local_oid));
    assert_eq!(
        result.snapshot().contexts()[0].state(),
        RemoteContextState::Unmaterialized
    );
    assert!(requests.borrow().is_empty());
    let helpers = case.fixture.helper_invocations();
    fixed(remote.find_reference("refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV"))?
        .delete()
        .unwrap();
    let replay = fixed(case.service.observe_publication_remote(req, &mut session))?;
    assert_eq!(replay.snapshot(), result.snapshot());
    assert_eq!(case.fixture.helper_invocations(), helpers);
    before.check(&case)
}

// Catches prompting again on an automatic retry and persisting session locks as pause.
fn unlock_session() -> Result<(), FixtureError> {
    use crate::repository::keys::PassphraseResponse;
    for response in [
        PassphraseResponse::Cancelled,
        PassphraseResponse::Unavailable,
    ] {
        let case = setup(true)?;
        fixed(case.service.set_remote_polling(
            &case.root,
            true,
            true,
            PollingInterval::from_seconds(60).unwrap(),
        ))?;
        fixed(rusqlite::Connection::open(case.directory.path().join("data").join(REGISTRY_FILE)))?.execute("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main',automatic_backoff_seconds=60", []).unwrap();
        let (mut session, requests) = session(vec![response, secret(PASSWORD)]);
        let error = case
            .service
            .observe_publication_remote(request(&case), &mut session)
            .unwrap_err();
        assert!(
            matches!(error, RemoteObservationError::Transport(ref e) if matches!(e.kind, SshTransportErrorKind::UnlockCancelled | SshTransportErrorKind::ProviderUnavailable))
        );
        let mut auto = request(&case);
        auto.invocation = RemotePollInvocation::Automatic;
        assert!(matches!(
            case.service.observe_publication_remote(auto, &mut session),
            Err(RemoteObservationError::Transport(_))
        ));
        assert_eq!(requests.borrow().len(), 1);
        let snapshot = fixed(case.service.remote_snapshot(&case.root))?;
        assert!(snapshot.polling().paused());
        assert_eq!(
            snapshot.polling().automatic_backoff(),
            Some(std::time::Duration::from_secs(60))
        );
        assert_eq!(
            snapshot.latest_outcome(),
            Some(RemoteOutcomeCategory::UnlockRequired)
        );
        fixed(
            case.service
                .observe_publication_remote(request(&case), &mut session),
        )?;
        fixed(
            case.service
                .observe_publication_remote(request(&case), &mut session),
        )?;
        assert_eq!(requests.borrow().len(), 2);
    }
    Ok(())
}

// Catches host trust and selected-key errors being flattened or treated as a batch.
fn host_and_key_failures() -> Result<(), FixtureError> {
    let case = setup(false)?;
    let before = crate::failures::Preservation::capture(&case)?;
    let (mut session, requests) = session(vec![]);
    let mut req = request(&case);
    req.approval = None;
    assert!(
        matches!(case.service.observe_publication_remote(req, &mut session), Err(RemoteObservationError::Transport(e)) if matches!(e.kind, SshTransportErrorKind::HostApprovalRequired { .. }))
    );
    assert_eq!(
        fixed(case.service.remote_snapshot(&case.root))?.latest_outcome(),
        Some(RemoteOutcomeCategory::HostApprovalRequired)
    );
    assert!(requests.borrow().is_empty());
    before.check(&case)?;
    fixed(case.service.clear_shared_key_selection())?;
    assert!(
        matches!(case.service.observe_publication_remote(request(&case), &mut session), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::NoSelectedKey)
    );
    assert!(
        fixed(case.service.remote_snapshot(&case.root))?
            .observations()
            .is_empty()
    );
    Ok(())
}

// Catches ignored cancellation/yield, early reservation release, and lost postcommit evidence.
fn safe_points() -> Result<(), FixtureError> {
    use crate::repository::observation_tests::install_hook;
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::AfterAdvertisement,
        RemoteOperationSafePoint::BetweenObservations,
        RemoteOperationSafePoint::BeforeBatchCommit,
        RemoteOperationSafePoint::AfterBatchCommit,
    ] {
        for cancel in [true, false] {
            let case = setup(false)?;
            let before = crate::failures::Preservation::capture(&case)?;
            let mut req = request(&case);
            req.invocation = RemotePollInvocation::Automatic;
            let id = req.operation_id;
            let data = case.directory.path().join("data");
            let root = case.root.clone();
            let mut fired = false;
            let guard = install_hook(move |at| {
                if fired || at != point {
                    return;
                }
                fired = true;
                let other = RepositoryService::open_at(&data).unwrap();
                if cancel {
                    other.cancel_remote_operation(&root, id).unwrap();
                } else {
                    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
                    assert!(matches!(
                        other
                            .reserve_remote_operation_with_priority(
                                &root,
                                OperationId::new(),
                                &RemoteOperationTarget::for_poll(&plan),
                                RemoteOperationPriority::Manual
                            )
                            .unwrap(),
                        RemoteReservationOutcome::PollYielding
                    ));
                }
            });
            let (mut session, _) = session(vec![]);
            let result = case
                .service
                .observe_publication_remote(req.clone(), &mut session);
            drop(guard);
            if cancel {
                assert_eq!(fixed(result)?.category(), RemoteOutcomeCategory::Cancelled);
            } else {
                assert!(matches!(result, Err(RemoteObservationError::Interrupted)));
            }
            assert!(fixed(case.service.active_remote_operation(&case.root))?.is_none());
            assert_eq!(
                !fixed(case.service.remote_snapshot(&case.root))?
                    .observations()
                    .is_empty(),
                point == RemoteOperationSafePoint::AfterBatchCommit
            );
            if point == RemoteOperationSafePoint::BeforeTransport {
                assert_eq!(case.fixture.helper_invocations(), 0);
            }
            if !cancel {
                let helpers = case.fixture.helper_invocations();
                req.restart = true;
                assert_eq!(
                    fixed(case.service.observe_publication_remote(req, &mut session))?.category(),
                    RemoteOutcomeCategory::Completed
                );
                assert_eq!(
                    case.fixture.helper_invocations(),
                    helpers + usize::from(point != RemoteOperationSafePoint::AfterBatchCommit)
                );
            }
            before.check(&case)?;
        }
    }
    Ok(())
}

// Catches partial SQLite commit and recovery repeating a successfully committed advertisement.
fn database_recovery() -> Result<(), FixtureError> {
    use crate::repository::observation_tests::install_hook;
    for after_commit in [false, true] {
        let case = setup(false)?;
        let (mut session, _) = session(vec![]);
        fixed(
            case.service
                .observe_publication_remote(request(&case), &mut session),
        )?;
        let prior = fixed(case.service.remote_snapshot(&case.root))?;
        let path = case.directory.path().join("data").join(REGISTRY_FILE);
        let trigger_path = path.clone();
        let guard = install_hook(move |point| {
            if point
                == if after_commit {
                    RemoteOperationSafePoint::AfterBatchCommit
                } else {
                    RemoteOperationSafePoint::BeforeBatchCommit
                }
            {
                rusqlite::Connection::open(&trigger_path).unwrap().execute_batch(if after_commit {
                    "CREATE TRIGGER fail_observation BEFORE UPDATE ON remote_operation_records BEGIN SELECT RAISE(ABORT,'fixture'); END;"
                } else {
                    "CREATE TRIGGER fail_observation BEFORE INSERT ON remote_ref_observations BEGIN SELECT RAISE(ABORT,'fixture'); END;"
                }).unwrap();
            }
        });
        let mut req = request(&case);
        assert!(matches!(
            case.service
                .observe_publication_remote(req.clone(), &mut session),
            Err(RemoteObservationError::Repository(_))
        ));
        drop(guard);
        let db = fixed(rusqlite::Connection::open(path))?;
        fixed(db.execute_batch("DROP TRIGGER fail_observation"))?;
        assert_eq!(
            fixed(case.service.remote_snapshot(&case.root))?.observations(),
            prior.observations()
        );
        let helpers = case.fixture.helper_invocations();
        let before = crate::failures::Preservation::capture(&case)?;
        req.restart = true;
        fixed(case.service.observe_publication_remote(req, &mut session))?;
        assert_eq!(
            case.fixture.helper_invocations(),
            helpers + usize::from(!after_commit)
        );
        before.check(&case)?;
    }
    Ok(())
}

fn transport_failure() -> Result<(), FixtureError> {
    let case = setup(false)?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut session),
    )?;
    let prior = fixed(case.service.remote_snapshot(&case.root))?;
    let before = crate::failures::Preservation::capture(&case)?;
    case.fixture.disconnect_at(FixtureBoundary::Advertisement);
    assert!(
        matches!(case.service.observe_publication_remote(request(&case), &mut session), Err(RemoteObservationError::Transport(e)) if matches!(e.kind, SshTransportErrorKind::TransportUnavailable | SshTransportErrorKind::RemoteUnavailable | SshTransportErrorKind::ProtocolFailure))
    );
    assert_eq!(
        fixed(case.service.remote_snapshot(&case.root))?.observations(),
        prior.observations()
    );
    before.check(&case)
}

fn cache_invalidation() -> Result<(), FixtureError> {
    let case = setup(true)?;
    let (mut credentials, requests) = session(vec![
        secret(PASSWORD),
        secret(PASSWORD),
        secret(PASSWORD),
        secret(PASSWORD),
        secret(PASSWORD),
    ]);
    fixed(case.verify(&mut credentials))?;
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut credentials),
    )?;
    assert_eq!(requests.borrow().len(), 1);
    credentials.clear();
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut credentials),
    )?;
    assert_eq!(requests.borrow().len(), 2);
    credentials.invalidate(case.registration.id);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut credentials),
    )?;
    assert_eq!(requests.borrow().len(), 3);
    let bytes = fixed(std::fs::read(case.fixture.client_key_path()))?;
    fixed(std::fs::write(case.fixture.client_key_path(), bytes))?;
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut credentials),
    )?;
    assert_eq!(requests.borrow().len(), 4);
    let new_path = case.directory.path().join("other-private");
    fixed(std::fs::copy(case.fixture.client_key_path(), &new_path))?;
    let crate::repository::keys::RegisterSharedKeyOutcome::Registered(other) = fixed(
        case.service
            .register_shared_key(crate::repository::keys::RegisterSharedKeyRequest {
                label: "other".into(),
                ownership: crate::repository::keys::SharedKeyOwnership::Imported,
                private_key_path: new_path,
                public_key_path: None,
            }),
    )?
    else {
        return Err(FixtureError);
    };
    fixed(case.service.select_shared_key(other.id))?;
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut credentials),
    )?;
    assert_eq!(requests.borrow().len(), 5);
    let (mut replacement, replacement_requests) = session(vec![secret(PASSWORD)]);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut replacement),
    )?;
    assert_eq!(replacement_requests.borrow().len(), 1);
    Ok(())
}

fn missing_remote() -> Result<(), FixtureError> {
    for missing_configuration in [false, true] {
        let case = setup(false)?;
        if missing_configuration {
            fixed(std::fs::write(
                case.root.join(".manyhands/config.toml"),
                "format_version = 1\nprimary_branch = \"main\"\n",
            ))?;
        } else {
            fixed(git2::Repository::open(&case.root))?
                .remote_delete("origin")
                .unwrap();
        }
        let before = crate::failures::Preservation::capture(&case)?;
        let (mut session, requests) = session(vec![]);
        assert!(
            matches!(case.service.observe_publication_remote(request(&case), &mut session), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::PublicationRemoteMissing)
        );
        assert_eq!(
            fixed(case.service.remote_snapshot(&case.root))?.latest_outcome(),
            Some(RemoteOutcomeCategory::ConfigurationRequired)
        );
        assert_eq!(case.fixture.helper_invocations(), 0);
        assert!(requests.borrow().is_empty());
        before.check(&case)?;
    }
    Ok(())
}

fn malformed_redaction() -> Result<(), FixtureError> {
    let case = setup(false)?;
    let remote = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(remote.reference(
        "refs/heads/manyhands/ticket/private-response-marker",
        case.fixture.commit_id(),
        true,
        "fixture",
    ))?;
    let (mut session, _) = session(vec![]);
    let outcome = fixed(
        case.service
            .observe_publication_remote(request(&case), &mut session),
    )?;
    let malformed = outcome
        .snapshot()
        .observations()
        .iter()
        .find(|o| o.classification() == &RemoteRefClassification::MalformedContext)
        .unwrap();
    assert!(malformed.remote_ref().is_none());
    assert!(!format!("{outcome:?}").contains("private-response-marker"));
    for entry in fixed(std::fs::read_dir(case.directory.path().join("data")))? {
        let path = fixed(entry)?.path();
        if path.is_file() {
            let bytes = fixed(std::fs::read(path))?;
            assert!(
                !bytes
                    .windows(b"private-response-marker".len())
                    .any(|w| w == b"private-response-marker")
            );
        }
    }
    Ok(())
}

fn manual_priority() -> Result<(), FixtureError> {
    use crate::repository::observation_tests::install_hook;
    let case = setup(false)?;
    let data = case.directory.path().join("data");
    let root = case.root.clone();
    let guard = install_hook(move |point| {
        if point != RemoteOperationSafePoint::BeforeTransport {
            return;
        }
        let other = RepositoryService::open_at(&data).unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        assert!(matches!(
            other
                .reserve_remote_operation_with_priority(
                    &root,
                    OperationId::new(),
                    &RemoteOperationTarget::for_poll(&plan),
                    RemoteOperationPriority::Manual
                )
                .unwrap(),
            RemoteReservationOutcome::Busy
        ));
    });
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut session),
    )?;
    drop(guard);
    Ok(())
}

// Catches delaying explicit calls, counting attention blocks as failures, and uncapped retry state.
fn automatic_backoff() -> Result<(), FixtureError> {
    let case = setup(false)?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut session),
    )?;
    case.fixture.disconnect_at(FixtureBoundary::Advertisement);
    for seconds in [60, 120, 240, 480, 900, 900] {
        let mut auto = request(&case);
        auto.invocation = RemotePollInvocation::Automatic;
        assert!(matches!(
            case.service.observe_publication_remote(auto, &mut session),
            Err(RemoteObservationError::Transport(_))
        ));
        let snapshot = fixed(case.service.remote_snapshot(&case.root))?;
        assert_eq!(
            snapshot.polling().automatic_backoff(),
            Some(std::time::Duration::from_secs(seconds))
        );
        assert_eq!(
            snapshot.polling().delay_for(RemotePollInvocation::Explicit),
            None
        );
    }
    assert!(matches!(
        case.service
            .observe_publication_remote(request(&case), &mut session),
        Err(RemoteObservationError::Transport(_))
    ));
    assert_eq!(
        fixed(case.service.remote_snapshot(&case.root))?
            .polling()
            .automatic_backoff(),
        Some(std::time::Duration::from_secs(900))
    );
    fixed(case.service.clear_shared_key_selection())?;
    let mut auto = request(&case);
    auto.invocation = RemotePollInvocation::Automatic;
    assert!(
        matches!(case.service.observe_publication_remote(auto, &mut session), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::NoSelectedKey)
    );
    assert_eq!(
        fixed(case.service.remote_snapshot(&case.root))?
            .polling()
            .automatic_backoff(),
        None
    );
    Ok(())
}

fn block_reset() -> Result<(), FixtureError> {
    use crate::repository::keys::PassphraseResponse;
    for reset in 0..4 {
        let case = setup(true)?;
        let (mut credentials, requests) =
            session(vec![PassphraseResponse::Cancelled, secret(PASSWORD)]);
        let mut req = request(&case);
        req.invocation = RemotePollInvocation::Automatic;
        assert!(
            matches!(case.service.observe_publication_remote(req, &mut credentials), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::UnlockCancelled)
        );
        match reset {
            0 => credentials.clear(),
            1 => credentials.invalidate(case.registration.id),
            2 => {
                let bytes = fixed(std::fs::read(case.fixture.client_key_path()))?;
                fixed(std::fs::write(case.fixture.client_key_path(), bytes))?;
            }
            _ => {
                fixed(case.service.clear_shared_key_selection())?;
                assert!(
                    matches!(case.service.observe_publication_remote(request(&case), &mut credentials), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::NoSelectedKey)
                );
                fixed(case.service.select_shared_key(case.registration.id))?;
            }
        }
        let mut req = request(&case);
        req.invocation = RemotePollInvocation::Automatic;
        fixed(
            case.service
                .observe_publication_remote(req, &mut credentials),
        )?;
        assert_eq!(requests.borrow().len(), 2);
    }
    Ok(())
}

fn configuration_race() -> Result<(), FixtureError> {
    use crate::repository::observation_tests::install_hook;
    let case = setup(false)?;
    let config = case.root.join(".manyhands/config.toml");
    let guard = install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeBatchCommit {
            std::fs::write(&config, "format_version = 1\nprimary_branch = \"changed\"\npublication_remote = \"origin\"\n").unwrap();
        }
    });
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service
            .observe_publication_remote(request(&case), &mut session),
        Err(RemoteObservationError::Interrupted)
    ));
    drop(guard);
    assert!(
        fixed(case.service.remote_snapshot(&case.root))?
            .observations()
            .is_empty()
    );
    Ok(())
}

fn endpoint_race() -> Result<(), FixtureError> {
    configuration_identity_race(false)
}

fn selection_race() -> Result<(), FixtureError> {
    configuration_identity_race(true)
}

// Catches a late same-name endpoint or selected-key edit publishing obsolete evidence.
fn configuration_identity_race(selection: bool) -> Result<(), FixtureError> {
    use crate::repository::observation_tests::install_hook;
    let case = setup(false)?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut session),
    )?;
    let root = case.root.clone();
    let data = case.directory.path().join("data");
    let guard = install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeBatchCommit {
            if selection {
                RepositoryService::open_at(&data)
                    .unwrap()
                    .clear_shared_key_selection()
                    .unwrap();
            } else {
                git2::Repository::open(&root)
                    .unwrap()
                    .remote_set_url("origin", "ssh://git@127.0.0.1:1/changed")
                    .unwrap();
            }
        }
    });
    assert!(
        case.service
            .observe_publication_remote(request(&case), &mut session)
            .is_err()
    );
    drop(guard);
    assert!(
        fixed(case.service.remote_snapshot(&case.root))?
            .observations()
            .is_empty()
    );
    assert!(fixed(case.service.active_remote_operation(&case.root))?.is_none());
    Ok(())
}

// Catches selection edits leaving old observations and live ownership behind.
fn selection_lifecycle() -> Result<(), FixtureError> {
    selection_transition(false)
}

fn selection_replacement() -> Result<(), FixtureError> {
    selection_transition(true)
}

fn selection_transition(replacement: bool) -> Result<(), FixtureError> {
    let case = setup(false)?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut session),
    )?;
    let before = fixed(case.service.remote_snapshot(&case.root))?;
    fixed(case.service.select_shared_key(case.registration.id))?;
    assert_eq!(fixed(case.service.remote_snapshot(&case.root))?, before);
    fixed(case.service.set_remote_polling(
        &case.root,
        true,
        true,
        PollingInterval::from_seconds(120).unwrap(),
    ))?;
    let target = RemoteOperationTarget::for_poll(
        &RemoteRefPlan::from_configuration("origin", "main").unwrap(),
    );
    let RemoteReservationOutcome::Reserved(owner) = fixed(case.service.reserve_remote_operation(
        &case.root,
        OperationId::new(),
        &target,
    ))?
    else {
        panic!("reservation missing")
    };
    let other = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    if replacement {
        let path = case.directory.path().join("replacement key");
        fixed(std::fs::copy(&case.registration.private_key_path, &path))?;
        let keys::RegisterSharedKeyOutcome::Registered(key) =
            fixed(other.register_shared_key(keys::RegisterSharedKeyRequest {
                label: "replacement".into(),
                ownership: keys::SharedKeyOwnership::Imported,
                private_key_path: path,
                public_key_path: None,
            }))?
        else {
            panic!("replacement registration missing")
        };
        fixed(other.select_shared_key(key.id))?;
    } else {
        fixed(other.clear_shared_key_selection())?;
    }
    let snapshot = fixed(other.remote_snapshot(&case.root))?;
    assert!(snapshot.observations().is_empty());
    assert!(snapshot.polling().paused());
    assert_eq!(snapshot.polling().interval().as_secs(), 120);
    assert!(
        case.service
            .remote_safe_point(
                &case.root,
                &owner,
                RemoteOperationSafePoint::BeforeTransport
            )
            .is_err()
    );
    assert!(fixed(other.active_remote_operation(&case.root))?.is_none());
    Ok(())
}

fn shared_block_config_failure() -> Result<(), FixtureError> {
    shared_block_across_repositories(true)
}

fn shared_block_preflight_failure() -> Result<(), FixtureError> {
    shared_block_across_repositories(false)
}

// A failure in repository B must not authorize another automatic prompt for A.
fn shared_block_across_repositories(invalid_config: bool) -> Result<(), FixtureError> {
    use crate::repository::keys::PassphraseResponse;
    let case = setup(true)?;
    let other_root = case.directory.path().join("other-repo");
    fixed(git2::Repository::init(&other_root))?;
    fixed(std::fs::create_dir(other_root.join(".manyhands")))?;
    fixed(std::fs::write(
        other_root.join(".manyhands/config.toml"),
        if invalid_config {
            "format_version = 1\nprimary_branch = \"main\"\n"
        } else {
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n"
        },
    ))?;
    let db = fixed(rusqlite::Connection::open(
        case.directory.path().join("data").join(REGISTRY_FILE),
    ))?;
    fixed(db.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES (?1,123,'accessible',0)", [other_root.to_str().unwrap()]))?;
    let before = crate::failures::Preservation::capture(&case)?;
    let (mut session, requests) = session(vec![PassphraseResponse::Cancelled, secret(PASSWORD)]);
    assert!(
        matches!(case.service.observe_publication_remote(request(&case), &mut session), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::UnlockCancelled)
    );
    let mut other = request(&case);
    other.root = other_root;
    other.invocation = RemotePollInvocation::Automatic;
    assert!(
        matches!(case.service.observe_publication_remote(other, &mut session), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::PublicationRemoteMissing)
    );
    let mut automatic = request(&case);
    automatic.invocation = RemotePollInvocation::Automatic;
    assert!(
        matches!(case.service.observe_publication_remote(automatic, &mut session), Err(RemoteObservationError::Transport(e)) if e.kind == SshTransportErrorKind::UnlockCancelled)
    );
    assert_eq!(requests.borrow().len(), 1);
    fixed(
        case.service
            .observe_publication_remote(request(&case), &mut session),
    )?;
    assert_eq!(requests.borrow().len(), 2);
    before.check(&case)
}

fn explicit_cancel_backoff() -> Result<(), FixtureError> {
    cancel_preserves_backoff(RemotePollInvocation::Explicit)
}

fn automatic_cancel_backoff() -> Result<(), FixtureError> {
    cancel_preserves_backoff(RemotePollInvocation::Automatic)
}

// Cancellation is not a successful batch or a new network failure.
fn cancel_preserves_backoff(invocation: RemotePollInvocation) -> Result<(), FixtureError> {
    use crate::repository::observation_tests::install_hook;
    for point in [
        RemoteOperationSafePoint::BeforeTransport,
        RemoteOperationSafePoint::BeforeBatchCommit,
        RemoteOperationSafePoint::AfterBatchCommit,
    ] {
        let case = setup(false)?;
        let db = fixed(rusqlite::Connection::open(
            case.directory.path().join("data").join(REGISTRY_FILE),
        ))?;
        fixed(db.execute("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main',automatic_backoff_seconds=240,paused=1", []))?;
        let mut req = request(&case);
        req.invocation = invocation;
        let operation_id = req.operation_id;
        let data = case.directory.path().join("data");
        let root = case.root.clone();
        let guard = install_hook(move |at| {
            if at == point {
                RepositoryService::open_at(&data)
                    .unwrap()
                    .cancel_remote_operation(&root, operation_id)
                    .unwrap();
            }
        });
        let (mut session, _) = session(vec![]);
        let result = fixed(case.service.observe_publication_remote(req, &mut session))?;
        drop(guard);
        assert_eq!(result.category(), RemoteOutcomeCategory::Cancelled);
        assert!(result.snapshot().polling().paused());
        assert_eq!(
            result.snapshot().polling().automatic_backoff(),
            if point == RemoteOperationSafePoint::AfterBatchCommit {
                None
            } else {
                Some(std::time::Duration::from_secs(240))
            }
        );
    }
    Ok(())
}
