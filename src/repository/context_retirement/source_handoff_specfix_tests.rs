//! Review regressions: inert A0/A2 projections and synthetic selected-key files
//! in isolated Fixture data, real poll processing/owned SQL/original refresh.
//! No transport, real keys/profile, Task 4 producer/enrollment or consumption.
use super::*;
use crate::repository::{context_retirement::handoff, keys::*};
use std::sync::{Arc, Mutex};

#[test]
fn retirement_source_handoff_specfix_r1_original_process_configuration_survives_a3() {
    let mut wrongly_eligible = Vec::new();
    for change in [
        "unchanged",
        "rewritten",
        "replaced",
        "missing",
        #[cfg(unix)]
        "retargeted",
    ] {
        let mut s = SourceFixture::with_setup(|f| {
            let key = f.data.path().join("specfix-selected-source");
            std::fs::write(&key, b"synthetic source metadata, not a private key").unwrap();
            let RegisterSharedKeyOutcome::Registered(first) = f
                .service
                .register_shared_key(RegisterSharedKeyRequest {
                    label: "specfix original".into(),
                    ownership: SharedKeyOwnership::Imported,
                    private_key_path: key,
                    public_key_path: None,
                })
                .unwrap()
            else {
                panic!("isolated synthetic source registration");
            };
            f.service.select_shared_key(first.id).unwrap();
            let second = f.data.path().join("specfix-other-source");
            std::fs::write(&second, b"second synthetic source metadata").unwrap();
            assert!(matches!(
                f.service
                    .register_shared_key(RegisterSharedKeyRequest {
                        label: "specfix replacement".into(),
                        ownership: SharedKeyOwnership::Imported,
                        private_key_path: second,
                        public_key_path: None
                    })
                    .unwrap(),
                RegisterSharedKeyOutcome::Registered(_)
            ));
        });
        a2_projections(&s.f.db());
        let image = serde_json::to_string(&vec![candidate(&s, true)]).unwrap();
        input(&s, &image, 1);
        s.seal().unwrap();
        let original =
            s.f.service
                .observation_configuration(s.f.root.path(), &s.plan)
                .unwrap();
        let source = s.f.data.path().join("specfix-selected-source");
        let token = KeySourceToken::observe(&source).unwrap();
        let generation: i64 =
            s.f.db()
                .query_row(
                    "SELECT configuration_generation FROM remote_polling_state",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
        let data = s.f.data.path().to_path_buf();
        let root = s.f.root.path().to_path_buf();
        let plan = s.plan.clone();
        let reached = Arc::new(Mutex::new(false));
        let hit = Arc::clone(&reached);
        s.f.service
            .set_poll_boundary_hook_for_testing(move |point, _| {
                if point == FailurePoint::PollAfterCompletion {
                    let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
                    assert_eq!(counts(&db), (1, 2, 0));
                    assert_eq!(decision(&db), ("unconfirmed".into(), None));
                    let service = RepositoryService::open_at(&data).unwrap();
                    match change {
                        "rewritten" => std::fs::write(
                            &source,
                            b"externally rewritten, different length synthetic source",
                        )
                        .unwrap(),
                        "replaced" => {
                            let replacement = data.join("specfix-replacement-file");
                            std::fs::write(&replacement, std::fs::read(&source).unwrap()).unwrap();
                            std::fs::rename(replacement, &source).unwrap();
                        }
                        "missing" => std::fs::remove_file(&source).unwrap(),
                        #[cfg(unix)]
                        "retargeted" => {
                            std::fs::remove_file(&source).unwrap();
                            std::os::unix::fs::symlink(data.join("specfix-other-source"), &source)
                                .unwrap();
                        }
                        "unchanged" => {}
                        _ => unreachable!(),
                    }
                    let selected = service
                        .list_shared_keys()
                        .unwrap()
                        .into_iter()
                        .find(|key| key.selected)
                        .unwrap();
                    let current_token = KeySourceToken::observe(&selected.private_key_path).ok();
                    assert_eq!(
                        current_token.as_ref() != Some(&token),
                        change != "unchanged"
                    );
                    let current = service.observation_configuration(&root, &plan).unwrap();
                    assert_eq!(current != original, change != "unchanged");
                    assert_eq!(current.endpoint_digest(), original.endpoint_digest());
                    assert_eq!(
                        db.query_row(
                            "SELECT configuration_generation FROM remote_polling_state",
                            [],
                            |r| r.get::<_, i64>(0)
                        )
                        .unwrap(),
                        generation
                    );
                    *hit.lock().unwrap() = true;
                }
            });
        let refreshes_before:i64=s.f.db().query_row("SELECT count(*) FROM operation_records WHERE action='refresh' AND state='completed'",[],|r|r.get(0)).unwrap();
        let snapshot = Arc::new(Mutex::new(None));
        let observed = Arc::clone(&snapshot);
        let data = s.f.data.path().to_path_buf();
        let root = s.f.root.path().to_path_buf();
        handoff::observe_next_settle_for_testing(move || {
            let db = Connection::open(data.join(REGISTRY_FILE)).unwrap();
            assert_eq!(
                db.query_row("SELECT refresh_required FROM repositories", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0,
                "original refresh really succeeded before A3"
            );
            assert_eq!(db.query_row("SELECT count(*) FROM operation_records WHERE action='refresh' AND state='completed'",[],|r|r.get::<_,i64>(0)).unwrap(),refreshes_before+1);
            *observed.lock().unwrap() =
                Some((registry_image(&db), disk_image(&root), terminal(&db)));
        });
        let result = run(&s);
        assert!(*reached.lock().unwrap());
        let db = s.f.db();
        let (before, disk, public) = snapshot
            .lock()
            .unwrap()
            .take()
            .expect("actual A3 boundary reached");
        if change == "unchanged" {
            assert!(matches!(result, Ok(RemotePollResult::Complete(_))));
            assert_eq!(decision(&db), ("eligible".into(), None));
            assert_eq!(counts(&db), (1, 1, 1));
        } else if decision(&db).0 == "eligible" {
            wrongly_eligible.push(change);
        } else {
            assert!(
                matches!(result,Err(RemotePollError::Repository(ref e)) if e.kind==RepositoryErrorKind::RecoveryRequired)
            );
            assert_eq!(
                before,
                registry_image(&db),
                "{change}: A3 refusal must not mutate ANY registry row"
            );
            assert_eq!(disk, disk_image(s.f.root.path()));
            assert_eq!(counts(&db), (1, 2, 0));
            assert_eq!(decision(&db), ("unconfirmed".into(), None));
            recover(&s);
            assert_eq!(
                decision(&db),
                ("ineligible".into(), Some("discovery_pending".into()))
            );
            assert_eq!(counts(&db), (1, 0, 2));
            let releases:Vec<(String,String)>=db.prepare("SELECT pin_kind,reason FROM retirement_observation_pin_releases ORDER BY pin_kind").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
            assert_eq!(
                releases,
                vec![
                    ("consumer_attempt".into(), "consumer_terminal".into()),
                    ("source_attempt".into(), "handoff_decided".into())
                ]
            );
            let settled = registry_image(&db);
            recover(&s);
            assert_eq!(settled, registry_image(&db));
        }
        assert_eq!(public, terminal(&db));
        assert_eq!(
            db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        record::audit_connection(&db).unwrap();
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
    assert!(
        wrongly_eligible.is_empty(),
        "changed original process configuration granted eligibility: {wrongly_eligible:?}"
    );
}

#[allow(clippy::result_large_err)] // Existing public poll error, not a new API.
fn call(
    service: &RepositoryService,
    root: &Path,
    operation_id: OperationId,
    invocation: RemotePollInvocation,
    restart: bool,
) -> Result<RemotePollResult, RemotePollError> {
    service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: root.into(),
            operation_id,
            invocation,
            approval: None,
            restart,
        },
        &mut SessionCredentials::new(NoCredentials),
    )
}

fn assert_exact_recovery(
    s: &SourceFixture,
    mut before: BTreeMap<String, Vec<Vec<rusqlite::types::Value>>>,
) {
    use rusqlite::types::Value;
    let db = s.f.db();
    for (table, column, value) in [
        ("poll_retirement_freezes", "eligibility", "ineligible"),
        ("poll_retirement_freezes", "final_code", "discovery_pending"),
        ("poll_retirement_attempts", "phase", "ended"),
    ] {
        let columns: Vec<String> = db
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let rows = before.get_mut(table).unwrap();
        assert_eq!(rows.len(), 1);
        rows[0][columns.iter().position(|name| name == column).unwrap()] =
            Value::Text(value.into());
    }
    let pins = before.get_mut("retirement_observation_pins").unwrap();
    assert_eq!(pins.len(), 2);
    let releases: Vec<_> = pins
        .drain(..)
        .map(|mut pin| {
            let reason = match &pin[1] {
                Value::Text(kind) if kind == "source_attempt" => "handoff_decided",
                Value::Text(kind) if kind == "consumer_attempt" => "consumer_terminal",
                _ => panic!("exact bound hold"),
            };
            pin.push(Value::Text(reason.into()));
            pin
        })
        .collect();
    assert!(before["retirement_observation_pin_releases"].is_empty());
    before.insert("retirement_observation_pin_releases".into(), releases);
    assert_eq!(
        before,
        registry_image(&db),
        "only exact conservative decision/end/Z may write; no reservation, refresh, recapture or repin"
    );
}

fn ordinary_poll_replaces_latest(s: &SourceFixture, service: &RepositoryService) -> OperationId {
    let operation = OperationId::new();
    let RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Reserved(owner)) =
        service
            .reserve_remote_poll(
                s.f.root.path(),
                operation,
                &RemoteOperationTarget::for_poll(&s.plan),
                RemoteOperationPriority::Manual,
            )
            .unwrap()
    else {
        panic!("fresh ordinary poll");
    };
    assert_eq!(
        service
            .remote_safe_point(
                s.f.root.path(),
                &owner,
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    let configuration = service
        .observation_configuration(s.f.root.path(), &s.plan)
        .unwrap();
    let tip = git2::Repository::open(s.f.root.path())
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap();
    assert!(matches!(
        service
            .poll_after_fetch(
                s.f.root.path(),
                &owner,
                &s.plan,
                &configuration,
                vec![("refs/heads/main".into(), tip)]
            )
            .unwrap(),
        RemotePollResult::Complete(_)
    ));
    assert_eq!(
        s.f.db()
            .query_row(
                "SELECT DISTINCT operation_ulid FROM remote_poll_dispositions",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        operation.to_string()
    );
    operation
}

#[test]
fn retirement_source_handoff_specfix_r2_completed_restart_flags_settle_and_replay_exact_report() {
    let mut blocked = Vec::new();
    let mut replay_blocked = Vec::new();
    for restart in [false, true] {
        for repair_before in [false, true] {
            let s = prepared(true);
            *s.f.service.failure_point.lock().unwrap() = Some(FailurePoint::PollAfterCompletion);
            assert!(run(&s).is_err());
            let db = s.f.db();
            assert_eq!(decision(&db), ("unconfirmed".into(), None));
            assert_eq!(counts(&db), (1, 2, 0));
            let public = terminal(&db);
            let report_bytes: String = db
                .query_row(
                    "SELECT original_report FROM poll_retirement_freezes",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let original = RemotePollReport::decode_frozen(&report_bytes, &s.plan).unwrap();
            let service = RepositoryService::open_at(s.f.data.path()).unwrap();
            if repair_before {
                assert!(matches!(
                    service
                        .refresh_repository(RefreshRepositoryRequest {
                            root: s.f.root.path().into(),
                            operation_id: OperationId::new()
                        })
                        .unwrap(),
                    RefreshOutcome::Refreshed { .. }
                ));
            }
            let before = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            let result = call(
                &service,
                s.f.root.path(),
                s.operation,
                RemotePollInvocation::Explicit,
                restart,
            );
            if let Ok(result) = result {
                let (report, pending) = match result {
                    RemotePollResult::Complete(RemotePollOutcome::Polled(report)) => {
                        (report, false)
                    }
                    RemotePollResult::IndexPending(p) => {
                        let RemotePollOutcome::Polled(report) = p.authoritative else {
                            panic!("original P report");
                        };
                        (report, true)
                    }
                    _ => panic!("original P report"),
                };
                assert_eq!(report, original);
                assert_eq!(pending, !repair_before);
                assert_exact_recovery(&s, before);
                assert_eq!(disk, disk_image(s.f.root.path()));
                assert_eq!(terminal(&db), public);
                assert_eq!(
                    decision(&db),
                    ("ineligible".into(), Some("discovery_pending".into()))
                );
                assert_eq!(counts(&db), (1, 0, 2));
                if !repair_before {
                    assert!(matches!(
                        service
                            .refresh_repository(RefreshRepositoryRequest {
                                root: s.f.root.path().into(),
                                operation_id: OperationId::new()
                            })
                            .unwrap(),
                        RefreshOutcome::Refreshed { .. }
                    ));
                }
                ordinary_poll_replaces_latest(&s, &service);
                let after_other = registry_image(&db);
                let disk = disk_image(s.f.root.path());
                for replay_restart in [false, true] {
                    let replay = call(
                        &service,
                        s.f.root.path(),
                        s.operation,
                        RemotePollInvocation::Explicit,
                        replay_restart,
                    );
                    if let Ok(replay) = replay {
                        assert_eq!(
                            replay,
                            RemotePollResult::Complete(RemotePollOutcome::Polled(original.clone()))
                        );
                    } else {
                        replay_blocked.push((restart, repair_before, replay_restart));
                    }
                    assert_eq!(after_other, registry_image(&db));
                    assert_eq!(disk, disk_image(s.f.root.path()));
                    assert_eq!(terminal(&db), public);
                }
                assert_eq!(
                    db.query_row(
                        "SELECT original_report FROM poll_retirement_freezes",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                    report_bytes
                );
            } else {
                assert!(
                    matches!(result,Err(RemotePollError::Repository(ref e)) if e.kind==RepositoryErrorKind::RecoveryRequired)
                );
                assert_eq!(before, registry_image(&db));
                assert_eq!(disk, disk_image(s.f.root.path()));
                blocked.push((restart, repair_before));
            }
            assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
        }
    }
    assert!(
        blocked.is_empty(),
        "completed original P skipped conservative recovery: {blocked:?}"
    );
    assert!(
        replay_blocked.is_empty(),
        "settled completed P refused immutable replay: {replay_blocked:?}"
    );
}

#[test]
fn retirement_source_handoff_specfix_r2_active_and_automatic_never_acquire_recovery_authority() {
    for stage in ["reserved", "captured", "completed"] {
        let s = prepared(true);
        match stage {
            "captured" => {
                s.before_batch();
                s.capture().unwrap();
            }
            "completed" => {
                *s.f.service.failure_point.lock().unwrap() =
                    Some(FailurePoint::PollAfterCompletion);
                assert!(run(&s).is_err());
            }
            _ => {}
        }
        let service = RepositoryService::open_at(s.f.data.path()).unwrap();
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        for restart in [false, true] {
            for invocation in [
                RemotePollInvocation::Explicit,
                RemotePollInvocation::Automatic,
            ] {
                if stage == "completed" && invocation == RemotePollInvocation::Explicit {
                    continue;
                }
                assert!(
                    matches!(call(&service,s.f.root.path(),s.operation,invocation,restart),Err(RemotePollError::Repository(ref e)) if e.kind==RepositoryErrorKind::RecoveryRequired),
                    "{stage}/{restart}/{invocation:?}"
                );
                assert_eq!(before, registry_image(&db));
                assert_eq!(disk, disk_image(s.f.root.path()));
            }
        }
        if stage == "completed" {
            assert_eq!(decision(&db), ("unconfirmed".into(), None));
            assert_eq!(counts(&db), (1, 2, 0));
        }
    }
}

#[test]
fn retirement_source_handoff_specfix_r2_foreign_root_and_child_source_identity_refuse_without_writes()
 {
    let s = prepared(true);
    *s.f.service.failure_point.lock().unwrap() = Some(FailurePoint::PollAfterCompletion);
    assert!(run(&s).is_err());
    let other = tempfile::tempdir().unwrap();
    let git = git2::Repository::init(other.path()).unwrap();
    git.set_head("refs/heads/main").unwrap();
    let tree = git
        .find_tree(git.treebuilder(None).unwrap().write().unwrap())
        .unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    git.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "other isolated root",
        &tree,
        &[],
    )
    .unwrap();
    s.f.service
        .enable(EnableRepositoryRequest {
            root: other.path().into(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    let db = s.f.db();
    let image: String = db
        .query_row("SELECT image FROM poll_retirement_freezes", [], |r| {
            r.get(0)
        })
        .unwrap();
    let image: serde_json::Value = serde_json::from_str(&image).unwrap();
    let child = OperationId::parse(image[0]["child_operation_id"].as_str().unwrap()).unwrap();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    let other_disk = disk_image(other.path());
    for restart in [false, true] {
        for (root, id) in [(other.path(), s.operation), (s.f.root.path(), child)] {
            assert!(
                matches!(call(&s.f.service,root,id,RemotePollInvocation::Explicit,restart),Err(RemotePollError::Repository(ref e)) if e.kind==RepositoryErrorKind::RecoveryRequired)
            );
            assert_eq!(before, registry_image(&db));
            assert_eq!(disk, disk_image(s.f.root.path()));
            assert_eq!(other_disk, disk_image(other.path()));
        }
    }
    assert_eq!(decision(&db), ("unconfirmed".into(), None));
    assert_eq!(counts(&db), (1, 2, 0));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}
