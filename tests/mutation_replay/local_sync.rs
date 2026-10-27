//! Acceptance-to-link request authority, through the existing request store API.
#[path = "local_sync_inspection_spec.rs"]
mod inspection_spec;
#[path = "local_sync_live_prerequisites.rs"]
mod live_prerequisites;
#[path = "local_sync_quality.rs"]
mod quality;
#[path = "local_sync_spec.rs"]
mod spec;
use super::support::conformance::{Ask, Row};
use super::support::{items, mutation};
use super::support::{operations, records};
use manyhands::repository::request_store::InsertRequestOutcome;
use mutation::Previewed;

pub(crate) const CONFORMANCE: [Row; 2] = [
    Row::new("context sync-local", false, "invalid_input", |rule| {
        let world = mutation::World::new();
        let id = items::TICKET_A;
        world.execute(mutation::REQUEST_1, world.save(id, &world.token(id)));
        let observation = world
            .service
            .observe_local_context_sync(
                &world.resolved(),
                AuthoringKind::Ticket,
                &items::item_id(id),
            )
            .unwrap();
        mutation::drive_plain(
            &world.service,
            world.data.path(),
            &|ask| {
                Mutation::LocalContextSync(LocalContextSyncInput {
                    root: world.root.clone(),
                    kind: AuthoringKind::Ticket,
                    id: items::item_id(id),
                    observation: if matches!(ask, Ask::Changed | Ask::Refused) {
                        "malformed".into()
                    } else {
                        observation.observation.clone()
                    },
                })
            },
            rule,
        );
        Ok(())
    }),
    Row::new("context resolve-local", false, "invalid_input", |rule| {
        let f = ConflictWorld::new();
        let input = f.input();
        mutation::drive_plain(
            &f.world.service,
            f.world.data.path(),
            &|ask| {
                let mut input = copy_input(&input);
                if matches!(ask, Ask::Changed | Ask::Refused) {
                    input.observation = "malformed".into();
                }
                Mutation::LocalContextResolve(input)
            },
            rule,
        );
        Ok(())
    }),
];

#[test]
fn local_sync_request_child() {
    let Ok(payload) = std::env::var("MANYHANDS_LOCAL_REQUEST_CHILD") else {
        return;
    };
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let data = std::path::Path::new(json["data"].as_str().unwrap());
    let point = match json["point"].as_str().unwrap() {
        "path" => Some(FailurePoint::LocalSyncAfterResolutionPath),
        "write" => Some(FailurePoint::LocalSyncAfterResolutionWriteBeforeObservation),
        "ack" => Some(FailurePoint::LocalSyncAfterCancellationAcknowledgement),
        "accepted" => None,
        _ => panic!("unknown child boundary"),
    };
    let service = match point {
        Some(p) => RepositoryService::open_at_with_exit_point_for_testing(data, p).unwrap(),
        None => RepositoryService::open_at(data).unwrap(),
    };
    if point.is_none() {
        service.set_request_hook_for_testing(|| {
            std::process::exit(manyhands::repository::FAILURE_POINT_EXIT_STATUS)
        });
    }
    let input = LocalContextResolveInput {
        root: json["root"].as_str().unwrap().into(),
        operation_id: OperationId::parse(json["operation"].as_str().unwrap()).unwrap(),
        observation: json["observation"].as_str().unwrap().into(),
        resolutions: json["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| LocalConflictResolutionInput {
                token: s["token"].as_str().unwrap().into(),
                source: s["source"].as_str().unwrap().into(),
            })
            .collect(),
    };
    mutation::execute_with(
        &service,
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(input),
    );
    panic!("child must exit at boundary");
}
fn killed_resolution(f: &ConflictWorld, input: &LocalContextResolveInput, point: &str) {
    let payload = serde_json::json!({"root":f.world.root,"data":f.world.data.path(),"operation":f.operation.to_string(),"observation":input.observation,"sources":input.resolutions.iter().map(|s|serde_json::json!({"token":s.token,"source":s.source})).collect::<Vec<_>>(),"point":point});
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "local_sync::local_sync_request_child"])
        .env(
            "MANYHANDS_LOCAL_REQUEST_CHILD",
            serde_json::to_string(&payload).unwrap(),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(manyhands::repository::FAILURE_POINT_EXIT_STATUS)
    );
}
#[test]
fn local_sync_no_drop_acceptance_handover_and_acknowledgement_preserve_requests() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    killed_resolution(&f, &input, "accepted");
    assert_eq!(f.world.record(mutation::REQUEST_3).unwrap().attempt, 1);
    killed_resolution(&f, &input, "path");
    assert_eq!(f.world.record(mutation::REQUEST_3).unwrap().attempt, 2);
    f.world
        .service
        .cancel_request(RequestId::parse(mutation::REQUEST_3).unwrap());
    killed_resolution(&f, &input, "accepted");
    assert_eq!(f.world.record(mutation::REQUEST_3).unwrap().attempt, 3);
    killed_resolution(&f, &input, "ack");
    f.world.reopen();
    for id in [mutation::REQUEST_2, mutation::REQUEST_3] {
        assert_eq!(f.world.record(id).unwrap().state, RequestState::Accepted);
    }
    let request = f.world.record(mutation::REQUEST_3).unwrap();
    assert_eq!(request.attempt, 4);
    assert!(!request.cancel_requested);
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
}
use manyhands::{
    repository::*,
    results::{ResultCode, WriteEffect},
};

struct ConflictWorld {
    world: mutation::World,
    operation: OperationId,
    start_observation: String,
}
impl ConflictWorld {
    fn new() -> Self {
        Self::with_incoming("")
    }
    fn with_incoming(extra: &str) -> Self {
        let world = mutation::World::new();
        let id = items::TICKET_A;
        assert_eq!(
            world
                .execute(mutation::REQUEST_1, world.save(id, &world.token(id)))
                .code,
            ResultCode::Ok
        );
        let path = items::ticket_path(id);
        items::write(
            &world.root,
            &path,
            &items::ticket_source(id, "Incoming conflict", extra),
        );
        items::commit(&world.fixture, &[&path], items::COMMITTED_AT);
        let observed = world
            .service
            .observe_local_context_sync(
                &world.resolved(),
                AuthoringKind::Ticket,
                &items::item_id(id),
            )
            .unwrap();
        let result = world.execute(
            mutation::REQUEST_2,
            Mutation::LocalContextSync(LocalContextSyncInput {
                root: world.root.clone(),
                kind: AuthoringKind::Ticket,
                id: items::item_id(id),
                observation: observed.observation.clone(),
            }),
        );
        assert_eq!(result.code, ResultCode::RecoveryRequired);
        let operation = OperationId::parse(result.operation_id.as_deref().unwrap()).unwrap();
        Self {
            world,
            operation,
            start_observation: observed.observation,
        }
    }
    fn input(&self) -> LocalContextResolveInput {
        let view = self
            .world
            .service
            .inspect_local_context_sync(&self.world.resolved(), self.operation)
            .unwrap();
        assert!(view.can_start_resolution, "{view:?}");
        LocalContextResolveInput {
            root: self.world.root.clone(),
            operation_id: self.operation,
            observation: view.observation.unwrap(),
            resolutions: view
                .paths
                .into_iter()
                .map(|p| LocalConflictResolutionInput {
                    token: p.token,
                    source: items::ticket_source(items::TICKET_A, "Resolved", ""),
                })
                .collect(),
        }
    }
    fn start(&self) -> Mutation {
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: self.world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(items::TICKET_A),
            observation: self.start_observation.clone(),
        })
    }
}
fn copy_input(input: &LocalContextResolveInput) -> LocalContextResolveInput {
    LocalContextResolveInput {
        root: input.root.clone(),
        operation_id: input.operation_id,
        observation: input.observation.clone(),
        resolutions: input
            .resolutions
            .iter()
            .map(|s| LocalConflictResolutionInput {
                token: s.token.clone(),
                source: s.source.clone(),
            })
            .collect(),
    }
}

fn accept_without_link(world: &mutation::World, id: &str, input: Mutation) {
    world
        .service
        .set_request_hook_for_testing(|| panic!("stop after acceptance"));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| world.execute(id, input)))
            .is_err()
    );
    assert_eq!(world.record(id).unwrap().state, RequestState::Accepted);
}
#[test]
fn artifact_free_abandon_settles_all_associated_requests_atomically() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    accept_without_link(
        &f.world,
        super::REQUEST_A,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    accept_without_link(
        &f.world,
        super::REQUEST_B,
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        }),
    );
    f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncBeforeIndex)
        .open_service(f.world.data.path());
    let stopped = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(stopped.code, ResultCode::DiscoveryPending);
    let mutation = Mutation::OperationAbandon(OperationAbandonInput {
        root: f.world.root.clone(),
        operation_id: f.operation,
    });
    let preview = f.world.service.prepare(&mutation).previewed();
    assert_eq!(preview.code, ResultCode::Ok);
    let consent = preview.data.unwrap().confirmation_id;
    let result =
        mutation::execute_confirmed(&f.world.service, super::REQUEST_ABSENT, &consent, mutation);
    assert_eq!(result.code, ResultCode::Ok);
    for id in [
        mutation::REQUEST_2,
        mutation::REQUEST_3,
        super::REQUEST_A,
        super::REQUEST_B,
    ] {
        let record = f.world.record(id).unwrap();
        assert_eq!(record.state, RequestState::Finished, "{id}");
        let result = record.result.unwrap();
        assert_eq!(result.code, ResultCode::Cancelled);
        assert_eq!(result.data.unwrap()["state"], "abandoned");
    }
    let replay = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(replay.code, ResultCode::Cancelled);
    let mut changed = copy_input(&input);
    changed.observation.push('0');
    assert_eq!(
        f.world
            .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(changed))
            .code,
        ResultCode::RequestMismatch
    );
}

#[test]
fn unlinked_accepted_resolution_cannot_write_after_attempt_loss() {
    let f = ConflictWorld::new();
    let input = f.input();
    let service = RepositoryService::open_at(f.world.data.path()).unwrap();
    f.world.service.set_request_hook_for_testing(move || {
        assert_eq!(
            service
                .enter_request(RequestId::parse(mutation::REQUEST_3).unwrap())
                .unwrap(),
            Some(2)
        );
    });
    let before = std::fs::read(
        f.world
            .worktree(items::TICKET_A)
            .join(items::ticket_path(items::TICKET_A)),
    )
    .unwrap();
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_ne!(result.code, ResultCode::Ok);
    assert_eq!(
        result.effects.write,
        manyhands::results::WriteEffect::NotRequested
    );
    assert_eq!(
        std::fs::read(
            f.world
                .worktree(items::TICKET_A)
                .join(items::ticket_path(items::TICKET_A))
        )
        .unwrap(),
        before
    );
    assert_eq!(f.world.record(mutation::REQUEST_3).unwrap().attempt, 2);
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
}

#[test]
fn request_attempt_handover_preserves_resolution_and_cancellation() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
        .open_service(f.world.data.path());
    f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    let old = f.world.record(mutation::REQUEST_3).unwrap();
    f.world.service.cancel_request(old.request_id);
    accept_without_link(
        &f.world,
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    let db = items::index(f.world.data.path());
    let counters:(i64,i64,i64)=db.query_row("SELECT execution_attempt,cancel_issued,cancel_acknowledged FROM local_context_sync_request_control WHERE request_ulid=?1",[mutation::REQUEST_3],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(counters, (2, 1, 0));
    drop(db);
    f.world.reopen();
    let cancelled = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.effects.write, WriteEffect::Written);
    assert!(
        !f.world
            .service
            .finish_request(
                old.request_id,
                old.attempt,
                &manyhands::repository::request_store::RequestResult {
                    outcome: manyhands::results::Outcome::Success,
                    code: ResultCode::Ok,
                    effects: manyhands::results::Effects::not_requested(),
                    data: None
                }
            )
            .unwrap()
    );
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
    assert_eq!(
        items::index(f.world.data.path())
            .query_row(
                "SELECT count(*) FROM local_context_sync_resolutions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn unlinked_contender_observes_parent_completion_or_abandonment() {
    let f = ConflictWorld::new();
    let input = f.input();
    accept_without_link(
        &f.world,
        super::REQUEST_A,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    accept_without_link(
        &f.world,
        super::REQUEST_B,
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        }),
    );
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
    let r = f.world.execute(
        super::REQUEST_A,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(r.code, ResultCode::ExternalChange);
    assert_eq!(r.effects, manyhands::results::Effects::not_requested());
    assert!(f.world.record(super::REQUEST_A).is_none());
    let u = f.world.execute(
        super::REQUEST_B,
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        }),
    );
    assert_eq!(u.code, ResultCode::ExternalChange);
    assert!(f.world.record(super::REQUEST_B).is_none());
    artifact_free_abandon_settles_all_associated_requests_atomically();
}

#[test]
fn new_cancellation_survives_acknowledgement_and_resume() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncAfterCandidate)
        .open_service(f.world.data.path());
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_ne!(result.code, ResultCode::Ok);
    let start = RequestId::parse(mutation::REQUEST_2).unwrap();
    f.world.service.cancel_request(start);
    let concurrent = RepositoryService::open_at(f.world.data.path()).unwrap();
    f.world.service.set_observation_hook_for_testing(move || {
        assert_eq!(concurrent.cancel_request(start).code, ResultCode::Ok);
    });
    let resume = || {
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        })
    };
    let first = f.world.execute(super::REQUEST_A, resume());
    assert_eq!(first.code, ResultCode::Cancelled);
    let counters:(i64,i64)=items::index(f.world.data.path()).query_row("SELECT cancel_issued,cancel_acknowledged FROM local_context_sync_request_control WHERE request_ulid=?1",[mutation::REQUEST_2],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(counters, (2, 1));
    let second = f.world.execute(super::REQUEST_A, resume());
    assert_eq!(second.code, ResultCode::Cancelled);
    assert_eq!(
        f.world.execute(super::REQUEST_A, resume()).code,
        ResultCode::Ok
    );
}

#[test]
fn identity_missing_then_set_retries_the_same_local_intent() {
    for boundary in [
        None,
        Some(FailurePoint::LocalSyncAfterIntent),
        Some(FailurePoint::LocalSyncAfterCandidate),
    ] {
        let mut world = mutation::World::new();
        let id = items::TICKET_A;
        world.execute(mutation::REQUEST_1, world.save(id, &world.token(id)));
        items::write(
            &world.root,
            mutation::DOCUMENT_PATH,
            &items::document_source(mutation::DOCUMENT, "Incoming clean", ""),
        );
        items::commit(
            &world.fixture,
            &[mutation::DOCUMENT_PATH],
            items::COMMITTED_AT,
        );
        let observed = world
            .service
            .observe_local_context_sync(
                &world.resolved(),
                AuthoringKind::Ticket,
                &items::item_id(id),
            )
            .unwrap();
        let input = |root: std::path::PathBuf| {
            Mutation::LocalContextSync(LocalContextSyncInput {
                root,
                kind: AuthoringKind::Ticket,
                id: items::item_id(id),
                observation: observed.observation.clone(),
            })
        };
        let original_name = world
            .fixture
            .repository
            .config()
            .unwrap()
            .get_string("user.name")
            .unwrap();
        if let Some(point) = boundary {
            world.service = super::support::FailOnce::at(point).open_service(world.data.path());
            assert_ne!(
                world
                    .execute(mutation::REQUEST_2, input(world.root.clone()))
                    .code,
                ResultCode::Ok
            );
            assert_eq!(
                world.record(mutation::REQUEST_2).unwrap().state,
                RequestState::Accepted
            );
        }
        world
            .fixture
            .repository
            .config()
            .unwrap()
            .set_str("user.name", "")
            .unwrap();
        if boundary != Some(FailurePoint::LocalSyncAfterCandidate) {
            let refused = world.execute(mutation::REQUEST_2, input(world.root.clone()));
            assert_eq!(refused.code, ResultCode::IdentityRequired);
            if boundary.is_none() {
                world.assert_golden_with_data(
                    "local_sync_identity_required",
                    "local_context_sync_mutation.schema.json",
                    &refused,
                );
            }
            assert_eq!(
                world.record(mutation::REQUEST_2).is_some(),
                boundary.is_some()
            );
        }
        let set = Mutation::RepoIdentitySet(RepoIdentitySetInput {
            root: world.root.clone(),
            name: "Restored identity".into(),
            email: "restored@example.invalid".into(),
            initials: InitialsChange::Unchanged,
        });
        let preview = world.service.prepare(&set).previewed();
        assert_eq!(preview.code, ResultCode::Ok);
        let consent = preview.data.unwrap().confirmation_id;
        assert_eq!(
            mutation::execute_confirmed(&world.service, super::REQUEST_A, &consent, set).code,
            ResultCode::Ok
        );
        let result = world.execute(mutation::REQUEST_2, input(world.root.clone()));
        assert_eq!(result.code, ResultCode::Ok);
        let commit = world
            .fixture
            .repository
            .find_commit(
                git2::Oid::from_str(result.effects.commit_oid.as_deref().unwrap()).unwrap(),
            )
            .unwrap();
        assert_eq!(
            commit.author().name(),
            Some(if boundary == Some(FailurePoint::LocalSyncAfterCandidate) {
                original_name.as_str()
            } else {
                "Restored identity"
            })
        );
    }
}

#[test]
fn local_sync_mutation_schemas_and_goldens() {
    let f = ConflictWorld::new();
    let input = f.input();
    let partial = f.world.execute(mutation::REQUEST_2, f.start());
    f.world.assert_golden_with_data(
        "local_sync_partial",
        "local_context_sync_mutation.schema.json",
        &partial,
    );
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    let operation = f.operation.to_string();
    mutation::assert_contract_at(
        "local_resolve_success",
        f.world.root.to_str().unwrap(),
        Some("local_context_sync_mutation.schema.json"),
        &[(&operation, "<operation>")],
        &result,
    );
    let refused = f.world.execute(
        super::REQUEST_A,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: f.world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(items::TICKET_A),
            observation: "malformed".into(),
        }),
    );
    f.world.assert_golden("local_sync_refused", &refused);
    let side = serde_json::json!({"operation_id":operation,"token":"token","base":null,"local":"private","incoming":null,"historical":true});
    assert!(
        super::support::schema::check_published("local_context_sync_mutation.schema.json", &side)
            .is_err()
    );
}

#[test]
fn local_sync_lost_resolution_settlement_and_start_receipts_are_independent() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    f.world.service = super::support::FailOnce::at(FailurePoint::BeforeRequestSettlementWrite)
        .open_service(f.world.data.path());
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(result.code, ResultCode::Ok);
    assert_eq!(
        f.world.record(mutation::REQUEST_3).unwrap().state,
        RequestState::Accepted
    );
    let tip = f.world.branch_tip(items::TICKET_A);
    f.world.reopen();
    assert_eq!(
        f.world.execute(mutation::REQUEST_2, f.start()).code,
        ResultCode::Ok
    );
    let repeated = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(repeated.code, ResultCode::Ok);
    assert_eq!(repeated.effects, result.effects);
    assert_eq!(f.world.branch_tip(items::TICKET_A), tip);
}

#[test]
fn local_sync_public_resolution_never_stores_submitted_source() {
    let f = ConflictWorld::new();
    let mut input = f.input();
    const PRIVATE: &str = "LOCAL_SOURCE_PRIVATE_9740ac";
    input.resolutions[0].source.push_str(PRIVATE);
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(result.code, ResultCode::Ok);
    assert!(!serde_json::to_string(&result).unwrap().contains(PRIVATE));
    let db = items::index(f.world.data.path());
    let tables:Vec<String>=db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'local_context_sync_%'").unwrap().query_map([],|r|r.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    assert!(tables.len() >= 7);
    for table in tables {
        let mut statement = db.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let count = statement.column_count();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for column in 0..count {
                match row.get_ref(column).unwrap() {
                    rusqlite::types::ValueRef::Text(bytes)
                    | rusqlite::types::ValueRef::Blob(bytes) => assert!(
                        !bytes
                            .windows(PRIVATE.len())
                            .any(|p| p == PRIVATE.as_bytes()),
                        "{table}"
                    ),
                    _ => {}
                }
            }
        }
    }
    mutation::assert_request_stores_exclude(f.world.data.path(), &[PRIVATE]);
}

#[test]
fn local_sync_unimported_candidate_requires_exact_resolution_before_resume_takeover() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    let name = f
        .world
        .fixture
        .repository
        .config()
        .unwrap()
        .get_string("user.name")
        .unwrap();
    f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncBeforeObjectImport)
        .open_service(f.world.data.path());
    assert_ne!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
    let epoch = || {
        items::index(f.world.data.path())
            .query_row(
                "SELECT owner_epoch FROM local_context_sync_records WHERE operation_ulid=?1",
                [f.operation.to_string()],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
    };
    let before = epoch();
    let resume = f.world.execute(
        super::REQUEST_A,
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        }),
    );
    assert_eq!(resume.code, ResultCode::OriginalRequestRequired);
    assert_eq!(epoch(), before);
    assert!(f.world.record(super::REQUEST_A).is_none());
    f.world
        .fixture
        .repository
        .config()
        .unwrap()
        .set_str("user.name", "Changed after candidate")
        .unwrap();
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(result.code, ResultCode::Ok);
    let commit = f
        .world
        .fixture
        .repository
        .find_commit(git2::Oid::from_str(result.effects.commit_oid.as_deref().unwrap()).unwrap())
        .unwrap();
    assert_eq!(commit.author().name(), Some(name.as_str()));
}

#[test]
fn local_sync_missing_resume_control_refuses_atomic_handover() {
    let f = ConflictWorld::new();
    let input = || {
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        })
    };
    accept_without_link(&f.world, super::REQUEST_A, input());
    items::index(f.world.data.path())
        .execute(
            "DELETE FROM local_context_sync_request_control WHERE request_ulid=?1",
            [super::REQUEST_A],
        )
        .unwrap();
    let result = f.world.execute(super::REQUEST_A, input());
    assert_ne!(result.code, ResultCode::Ok);
    assert_eq!(f.world.record(super::REQUEST_A).unwrap().attempt, 1);
}

#[test]
fn local_sync_cancellation_preserves_external_only_recovery_choice() {
    let f = ConflictWorld::with_incoming("invalid yaml line\n");
    let before = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    assert_eq!(
        before.next_action,
        LocalContextSyncNextAction::ExternalRepair
    );
    f.world
        .service
        .cancel_request(RequestId::parse(mutation::REQUEST_2).unwrap());
    let stopped = f.world.execute(mutation::REQUEST_2, f.start());
    assert_eq!(stopped.code, ResultCode::Cancelled);
    let Some(MutationDataDto::LocalContextSync(data)) = stopped.data else {
        panic!("local data")
    };
    assert_eq!(data.next_action, LocalContextSyncNextAction::ExternalRepair);
    assert_eq!(
        data.eligibility,
        Some(LocalContextSyncEligibility::ExternalResolutionRequired)
    );
}

#[test]
fn local_sync_data_tag_cannot_bypass_legacy_replay_reachability() {
    let world = mutation::World::new();
    let id = items::TICKET_A;
    let token = world.token(id);
    let before = world.fixture.repository.head().unwrap().target().unwrap();
    assert_eq!(
        world
            .execute(mutation::REQUEST_1, world.save(id, &token))
            .code,
        ResultCode::Ok
    );
    items::index(world.data.path()).execute("UPDATE request_records SET result_data=json_set(result_data,'$.type','local_context_sync') WHERE request_ulid=?1",[mutation::REQUEST_1]).unwrap();
    world
        .fixture
        .repository
        .find_reference(&format!("refs/heads/{}", mutation::World::branch(id)))
        .unwrap()
        .set_target(before, "fixture foreign reset")
        .unwrap();
    assert_eq!(
        world
            .execute(mutation::REQUEST_1, world.save(id, &token))
            .code,
        ResultCode::RecoveryRequired
    );
}

#[test]
fn local_sync_public_resolution_and_start_settle_independently() {
    let f = ConflictWorld::new();
    let input = f.input();
    let sides = f
        .world
        .service
        .read_local_context_conflict(
            &f.world.resolved(),
            f.operation,
            &input.resolutions[0].token,
        )
        .unwrap();
    assert!(sides.local.unwrap().contains(mutation::TITLE));
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(result.code, ResultCode::Ok);
    assert_eq!(
        f.world.record(mutation::REQUEST_2).unwrap().state,
        RequestState::Accepted
    );
    let start = f.world.execute(mutation::REQUEST_2, f.start());
    assert_eq!(start.code, ResultCode::Ok);
    assert_eq!(result.effects.commit_oid, start.effects.commit_oid);
    let replay = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(replay.effects, result.effects);
    assert!(
        f.world
            .service
            .read_local_context_conflict(
                &f.world.resolved(),
                f.operation,
                &input.resolutions[0].token
            )
            .is_err()
    );
}

#[test]
fn accepted_resolution_observation_is_part_of_request_identity() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
        .open_service(f.world.data.path());
    let stopped = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(stopped.effects.write, WriteEffect::Written);
    assert_eq!(
        f.world.record(mutation::REQUEST_3).unwrap().state,
        RequestState::Accepted
    );
    let before = std::fs::read(
        f.world
            .worktree(items::TICKET_A)
            .join(items::ticket_path(items::TICKET_A)),
    )
    .unwrap();
    let mut changed = copy_input(&input);
    changed.observation.push('0');
    let mismatch = f
        .world
        .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(changed));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.effects.write, WriteEffect::Written);
    assert_eq!(
        std::fs::read(
            f.world
                .worktree(items::TICKET_A)
                .join(items::ticket_path(items::TICKET_A))
        )
        .unwrap(),
        before
    );
    assert_eq!(f.world.record(mutation::REQUEST_3).unwrap().attempt, 1);
    let resumed = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(resumed.code, ResultCode::Ok);
}

#[test]
fn cancelled_partial_resolution_remains_accepted_and_resendable() {
    let mut f = ConflictWorld::new();
    let input = f.input();
    f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
        .open_service(f.world.data.path());
    let stopped = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(stopped.effects.write, WriteEffect::Written);
    f.world
        .service
        .cancel_request(RequestId::parse(mutation::REQUEST_3).unwrap());
    let cancelled = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    f.world.reopen();
    for id in [mutation::REQUEST_2, mutation::REQUEST_3] {
        assert_eq!(f.world.record(id).unwrap().state, RequestState::Accepted);
    }
    let resume = f.world.execute(
        super::REQUEST_ABSENT,
        Mutation::OperationResume(OperationResumeInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        }),
    );
    assert_eq!(resume.code, ResultCode::OriginalRequestRequired);
    let Some(MutationDataDto::LocalContextSync(data)) = resume.data else {
        panic!("local data")
    };
    assert_eq!(
        data.required_request_id.as_deref(),
        Some(mutation::REQUEST_3)
    );
    assert!(f.world.record(super::REQUEST_ABSENT).is_none());
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
}

#[test]
fn local_sync_public_contained_primary_finishes_and_replays() {
    use super::support::{items, mutation};
    use manyhands::{
        repository::{AuthoringKind, LocalContextSyncInput, Mutation, RequestState},
        results::ResultCode,
    };
    let world = mutation::World::new();
    let id = items::TICKET_A;
    assert_eq!(
        world
            .execute(mutation::REQUEST_1, world.save(id, &world.token(id)))
            .code,
        ResultCode::Ok
    );
    let observation = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(id),
        )
        .unwrap();
    let input = || {
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(id),
            observation: observation.observation.clone(),
        })
    };
    let before = world.branch_tip(id);
    let result = world.execute(mutation::REQUEST_2, input());
    assert_eq!(result.code, ResultCode::AlreadyApplied);
    world.assert_golden_with_data(
        "local_sync_noop",
        "local_context_sync_mutation.schema.json",
        &result,
    );
    assert_eq!(
        world.record(mutation::REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    let replay = world.execute(mutation::REQUEST_2, input());
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    assert_eq!(world.branch_tip(id), before);
}

#[test]
fn local_sync_acceptance_records_intended_parent_before_domain_link() {
    let world = records::world();
    for (id, command, role) in [
        (super::REQUEST_A, "context sync-local", "start"),
        (super::REQUEST_B, "context resolve-local", "resolve"),
    ] {
        let mut request = world.request(id);
        request.command = command.into();
        if role == "resolve" {
            request.operations.clear();
            request.target = operations::OPERATION_A.into();
        }
        assert_eq!(
            world.service.insert_request(&request).unwrap(),
            InsertRequestOutcome::Inserted
        );
        let row = world.index().query_row(
            "SELECT intended_parent,scope_key,role,execution_attempt,parent_operation,resolution_attempt,cancel_issued,cancel_acknowledged FROM local_context_sync_request_control WHERE request_ulid=?1",
            [id], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?,r.get::<_, String>(2)?,r.get::<_, i64>(3)?,r.get::<_, Option<String>>(4)?,r.get::<_, Option<String>>(5)?,r.get::<_, i64>(6)?,r.get::<_, i64>(7)?)),
        ).unwrap();
        assert_eq!(
            row,
            (
                operations::OPERATION_A.into(),
                world.scope.as_str().into(),
                role.into(),
                1,
                None,
                None,
                0,
                0
            )
        );
    }
}

#[test]
fn local_sync_attempt_handover_preserves_unlinked_identity_counters_and_effects() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context resolve-local".into();
    request.target = operations::OPERATION_A.into();
    request.operations.clear();
    world.service.insert_request(&request).unwrap();
    world.index().execute(
        "UPDATE local_context_sync_request_control SET cancel_issued=2,cancel_acknowledged=1,effect_write='written' WHERE request_ulid=?1",
        [super::REQUEST_A],
    ).unwrap();
    for expected in [2, 3] {
        assert_eq!(
            world.service.enter_request(request.request_id).unwrap(),
            Some(expected)
        );
        let row = world.index().query_row(
            "SELECT execution_attempt,intended_parent,cancel_issued,cancel_acknowledged,effect_write FROM local_context_sync_request_control WHERE request_ulid=?1",
            [super::REQUEST_A],|r| Ok((r.get::<_,u64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?)),
        ).unwrap();
        assert_eq!(
            row,
            (
                expected,
                operations::OPERATION_A.into(),
                2,
                1,
                "written".into()
            )
        );
    }
}

#[test]
fn local_sync_missing_control_refuses_handover_without_advancing_request() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    world.service.insert_request(&request).unwrap();
    world
        .index()
        .execute("DELETE FROM local_context_sync_request_control", [])
        .unwrap();
    assert!(world.service.enter_request(request.request_id).is_err());
    assert_eq!(world.record(super::REQUEST_A).unwrap().attempt, 1);
}

#[test]
fn local_sync_ignored_control_insert_rolls_back_acceptance() {
    let world = records::world();
    world.index().execute_batch("CREATE TRIGGER refuse_local_control BEFORE INSERT ON local_context_sync_request_control BEGIN SELECT RAISE(IGNORE); END").unwrap();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    assert!(world.service.insert_request(&request).is_err());
    assert!(world.record(super::REQUEST_A).is_none());
    assert_eq!(world.count("SELECT count(*) FROM request_operations"), 0);
}

#[test]
fn local_sync_stale_control_attempt_cannot_be_repaired_by_handover() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    world.service.insert_request(&request).unwrap();
    world
        .index()
        .execute("UPDATE request_records SET attempt=2", [])
        .unwrap();
    assert!(world.service.enter_request(request.request_id).is_err());
    assert_eq!(world.record(super::REQUEST_A).unwrap().attempt, 2);
    assert_eq!(
        world.count("SELECT execution_attempt FROM local_context_sync_request_control"),
        1
    );
}

#[test]
fn local_sync_ignored_control_handover_rolls_back_both_attempt_fences() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context sync-local".into();
    world.service.insert_request(&request).unwrap();
    world.index().execute_batch("CREATE TRIGGER refuse_control_handover BEFORE UPDATE OF execution_attempt ON local_context_sync_request_control BEGIN SELECT RAISE(IGNORE); END").unwrap();
    assert!(world.service.enter_request(request.request_id).is_err());
    assert_eq!(world.record(super::REQUEST_A).unwrap().attempt, 1);
    assert_eq!(
        world.count("SELECT execution_attempt FROM local_context_sync_request_control"),
        1
    );
}

#[test]
fn local_sync_control_loss_is_not_recreated_on_reopen() {
    for lose_table in [false, true] {
        let world = records::world();
        if lose_table {
            world
                .index()
                .execute_batch("DROP TABLE local_context_sync_request_control")
                .unwrap();
        } else {
            world.index().execute("DELETE FROM registry_migrations WHERE name='local_context_sync_request_control_v1'",[]).unwrap();
        }
        let data = world.data.path().to_owned();
        drop(world.service);
        assert!(manyhands::repository::RepositoryService::open_at(&data).is_err());
    }
}

#[test]
fn local_sync_ordinary_requests_keep_their_existing_acceptance_and_handover() {
    let world = records::world();
    let request = world.request(super::REQUEST_A);
    world.service.insert_request(&request).unwrap();
    assert_eq!(
        world.service.enter_request(request.request_id).unwrap(),
        Some(2)
    );
    assert_eq!(
        world.count("SELECT count(*) FROM local_context_sync_request_control"),
        0
    );
}

#[test]
fn local_sync_unlinked_request_keeps_its_control_after_reopen() {
    let world = records::world();
    let mut request = world.request(super::REQUEST_A);
    request.command = "context resolve-local".into();
    request.target = operations::OPERATION_A.into();
    request.operations.clear();
    world.service.insert_request(&request).unwrap();
    let service = manyhands::repository::RepositoryService::open_at(world.data.path()).unwrap();
    assert_eq!(service.enter_request(request.request_id).unwrap(), Some(2));
    assert_eq!(
        world.count("SELECT execution_attempt FROM local_context_sync_request_control"),
        2
    );
    assert_eq!(world.count("SELECT count(*) FROM local_context_sync_request_control WHERE parent_operation IS NULL AND resolution_attempt IS NULL"),1);
}

type AuthorityImage = Vec<(String, Vec<Vec<rusqlite::types::Value>>)>;

fn task8_authority_image(world: &mutation::World) -> AuthorityImage {
    let db = items::index(world.data.path());
    let tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND (name LIKE 'local_context_sync_%' OR name IN ('request_records','request_operations','operation_records','repositories','contexts','configuration_observations')) ORDER BY name").unwrap().query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        tables
            .iter()
            .filter(|t| t.starts_with("local_context_sync_"))
            .count(),
        8,
        "every local companion including images and request control"
    );
    tables
        .into_iter()
        .map(|table| {
            let mut query = db
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let columns = query.column_count();
            let rows = query
                .query_map([], |r| (0..columns).map(|c| r.get(c)).collect())
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}

#[test]
fn lost_journal_distinguishes_conflict_checkout_and_completed_state() {
    for shape in ["conflict", "dirty_checkout", "private_lock", "complete"] {
        let mut f = ConflictWorld::new();
        let input = f.input();
        let context = git2::Repository::open(f.world.worktree(items::TICKET_A)).unwrap();
        if shape != "conflict" {
            assert_eq!(
                f.world
                    .execute(
                        mutation::REQUEST_3,
                        Mutation::LocalContextResolve(copy_input(&input))
                    )
                    .code,
                ResultCode::Ok
            );
        }
        if shape == "dirty_checkout" {
            for path in ["incoming-a", "incoming-b", "incoming-c"] {
                items::write(&f.world.root, path, "private partial checkout bytes");
            }
            items::commit(
                &f.world.fixture,
                &["incoming-a", "incoming-b", "incoming-c"],
                items::COMMITTED_AT,
            );
            let observation = f
                .world
                .service
                .observe_local_context_sync(
                    &f.world.resolved(),
                    AuthoringKind::Ticket,
                    &items::item_id(items::TICKET_A),
                )
                .unwrap();
            f.start_observation = observation.observation;
            f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncDuringCheckout)
                .open_service(f.world.data.path());
            let stopped = f.world.execute(super::REQUEST_A, f.start());
            assert_ne!(stopped.code, ResultCode::Ok);
            assert_eq!(stopped.effects.write, WriteEffect::Written);
            f.operation = OperationId::parse(stopped.operation_id.as_deref().unwrap()).unwrap();
            assert!(!context.path().join("MERGE_HEAD").exists());
            assert!(!context.index().unwrap().has_conflicts());
            assert!(!context.statuses(None).unwrap().is_empty());
        }
        if shape == "private_lock" {
            std::fs::write(
                context.path().join("index.lock"),
                "private interrupted backend lock",
            )
            .unwrap();
        }
        let git = super::support::repository_and_worktree_snapshot(&f.world.fixture);
        let artifacts = super::support::repository_git_file_bytes(&f.world.fixture);
        f.world.lose_cache();
        assert!(f.world.record(mutation::REQUEST_2).is_none());
        let before_rebuild = f.world.execute(mutation::REQUEST_2, f.start());
        assert_eq!(before_rebuild.code, ResultCode::RepositoryNotRegistered);
        let _ = f
            .world
            .service
            .rebuild_repository(RebuildRepositoryRequest {
                root: f.world.root.clone(),
                operation_id: OperationId::new(),
            });
        let resolved = f.world.resolved();
        let rows = task8_authority_image(&f.world);
        assert_eq!(
            f.world
                .service
                .inspect_local_context_sync(&resolved, f.operation)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired
        );
        assert_eq!(
            f.world
                .service
                .read_local_context_conflict(&resolved, f.operation, &input.resolutions[0].token)
                .err()
                .unwrap()
                .code(),
            ResultCode::RecoveryRequired
        );
        for table in [
            "local_context_sync_records",
            "local_context_sync_request_control",
        ] {
            assert_eq!(
                items::index(f.world.data.path())
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        let observation = f.world.service.observe_local_context_sync(
            &resolved,
            AuthoringKind::Ticket,
            &items::item_id(items::TICKET_A),
        );
        if shape == "complete" {
            let observation = observation.unwrap();
            assert_eq!(
                observation.context_head,
                context.head().unwrap().target().unwrap().to_string()
            );
        } else {
            let error = observation.unwrap_err();
            // Discovery cannot index conflict-marked/invalid canonical source.
            // Missing item authority is also a refusal, never Git adoption.
            assert_eq!(
                error.code(),
                match shape {
                    "conflict" => ResultCode::ItemNotFound,
                    "dirty_checkout" => ResultCode::WorktreeNotClean,
                    "private_lock" => ResultCode::RecoveryRequired,
                    _ => unreachable!(),
                },
                "{shape}: {error:?}"
            );
        }
        assert!(
            f.world.record(mutation::REQUEST_2).is_none(),
            "no invented starting request"
        );
        assert!(f.world.record(mutation::REQUEST_3).is_none());
        assert_eq!(rows, task8_authority_image(&f.world));
        assert_eq!(
            git,
            super::support::repository_and_worktree_snapshot(&f.world.fixture),
            "{shape}"
        );
        assert_eq!(
            artifacts,
            super::support::repository_git_file_bytes(&f.world.fixture),
            "{shape}"
        );
    }
}

#[test]
fn local_sync_healthy_rebuild_preserves_public_conflict_and_partial_resolution() {
    for partial in [false, true] {
        let mut f = ConflictWorld::new();
        let input = f.input();
        if partial {
            f.world.service =
                super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
                    .open_service(f.world.data.path());
            assert_ne!(
                f.world
                    .execute(
                        mutation::REQUEST_3,
                        Mutation::LocalContextResolve(copy_input(&input))
                    )
                    .code,
                ResultCode::Ok
            );
            f.world.reopen();
        }
        let before = super::support::repository_and_worktree_snapshot(&f.world.fixture);
        let artifacts = super::support::repository_git_file_bytes(&f.world.fixture);
        let rows = task8_authority_image(&f.world);
        let view = f
            .world
            .service
            .inspect_local_context_sync(&f.world.resolved(), f.operation)
            .unwrap();
        assert!(
            f.world
                .service
                .rebuild_repository(RebuildRepositoryRequest {
                    root: f.world.root.clone(),
                    operation_id: OperationId::new()
                })
                .is_err()
        );
        assert_eq!(rows, task8_authority_image(&f.world));
        assert_eq!(
            before,
            super::support::repository_and_worktree_snapshot(&f.world.fixture)
        );
        assert_eq!(
            artifacts,
            super::support::repository_git_file_bytes(&f.world.fixture)
        );
        assert_eq!(
            view,
            f.world
                .service
                .inspect_local_context_sync(&f.world.resolved(), f.operation)
                .unwrap()
        );
        assert_eq!(
            f.world
                .execute(
                    mutation::REQUEST_3,
                    Mutation::LocalContextResolve(copy_input(&input))
                )
                .code,
            ResultCode::Ok
        );
    }
}

#[test]
fn local_sync_full_public_output_excludes_positive_source_and_progress_sentinels() {
    const PRIVATE: &str = "TASK8-RESOLUTION-SOURCE-3f27d1";
    let mut f = ConflictWorld::new();
    let mut input = f.input();
    input.resolutions[0].source.push_str(PRIVATE);
    let mut events = mutation::Events::default();
    f.world.service = super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
        .open_service(f.world.data.path());
    let result = f.world.service.execute(
        MutationCall {
            request_id: RequestId::parse(mutation::REQUEST_3).unwrap(),
            confirmation: None,
            mutation: Mutation::LocalContextResolve(copy_input(&input)),
        },
        &mut keys::SessionCredentials::new(mutation::NoPassphrase),
        &mut events,
        &CancellationToken::new(),
    );
    assert_ne!(result.code, ResultCode::Ok);
    assert!(
        f.world.worktree_source(items::TICKET_A).contains(PRIVATE),
        "positive owned-file control"
    );
    let partial_rows = task8_authority_image(&f.world);
    mutation::assert_request_stores_exclude(
        f.world.data.path(),
        &[PRIVATE, mutation::SENTINELS[0], mutation::SENTINELS[1]],
    );
    f.world.reopen();
    let view = f
        .world
        .service
        .inspect_local_context_sync(&f.world.resolved(), f.operation)
        .unwrap();
    let explicit = f
        .world
        .service
        .read_local_context_conflict(
            &f.world.resolved(),
            f.operation,
            &input.resolutions[0].token,
        )
        .unwrap();
    assert!(
        explicit.local.unwrap().contains(mutation::SENTINELS[1]),
        "positive committed-side control"
    );
    let read_error = f
        .world
        .service
        .read_local_context_conflict(&f.world.resolved(), f.operation, "invalid-token")
        .err()
        .unwrap();
    let mut changed = copy_input(&input);
    changed.resolutions[0].source.push_str("changed");
    let mismatch = f
        .world
        .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(changed));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    let completed = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    assert_eq!(completed.code, ResultCode::Ok);
    let git = git2::Repository::open(f.world.worktree(items::TICKET_A)).unwrap();
    let tree = git.head().unwrap().peel_to_tree().unwrap();
    let entry = tree
        .get_path(std::path::Path::new(&items::ticket_path(items::TICKET_A)))
        .unwrap();
    assert!(
        std::str::from_utf8(git.find_blob(entry.id()).unwrap().content())
            .unwrap()
            .contains(PRIVATE)
    );
    let replay = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    let shown = f
        .world
        .service
        .show_request(RequestId::parse(mutation::REQUEST_3).unwrap());
    let output = format!(
        "{} {read_error:?} {read_error}",
        serde_json::json!([result, view, mismatch, completed, replay, events.0, shown])
    );
    assert!(!events.0.is_empty());
    for sentinel in [PRIVATE, mutation::SENTINELS[0], mutation::SENTINELS[1]] {
        assert!(!output.contains(sentinel));
    }
    mutation::assert_request_stores_exclude(
        f.world.data.path(),
        &[PRIVATE, mutation::SENTINELS[0], mutation::SENTINELS[1]],
    );
    // Discovery legitimately indexes titles; scan the eight local request /
    // journal companions, including the image manifest, rather than discovery.
    for (table, rows) in task8_authority_image(&f.world)
        .into_iter()
        .chain(partial_rows)
    {
        if !table.starts_with("local_context_sync_") {
            continue;
        }
        for value in rows.into_iter().flatten() {
            let bytes = match value {
                rusqlite::types::Value::Text(text) => text.into_bytes(),
                rusqlite::types::Value::Blob(bytes) => bytes,
                _ => continue,
            };
            for sentinel in [PRIVATE, mutation::SENTINELS[0], mutation::SENTINELS[1]] {
                assert!(
                    !bytes
                        .windows(sentinel.len())
                        .any(|w| w == sentinel.as_bytes()),
                    "{table}"
                );
            }
        }
    }
}

#[test]
fn local_sync_process_owner_child() {
    use std::io::{Read, Write};
    let Ok(payload) = std::env::var("MANYHANDS_TASK8_OWNER_CHILD") else {
        return;
    };
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    if let Some(fault) = json["harness_fault"].as_str() {
        println!("TASK8-FAULT-STDOUT");
        eprintln!("TASK8-FAULT-STDERR");
        std::io::stdout().flush().unwrap();
        if fault == "early_exit" {
            std::process::exit(23);
        }
        if fault == "completion" {
            println!("TASK8-ACCEPTED");
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_exact(&mut [0]).unwrap();
        }
        // Deliberately never progress. Only this fixture's supervisor may end
        // this child; no sleep is used to guess whether a boundary was reached.
        let (_hold, blocked) = std::sync::mpsc::channel::<()>();
        blocked.recv().unwrap();
        unreachable!();
    }
    if json["mode"] == "lease" {
        let git = git2::Repository::open(json["root"].as_str().unwrap()).unwrap();
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(git.commondir().join("manyhands-operation.lock"))
            .unwrap();
        fs4::fs_std::FileExt::lock_exclusive(&lock).unwrap();
        println!("TASK8-ACCEPTED");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_exact(&mut [0]).unwrap();
        fs4::fs_std::FileExt::unlock(&lock).unwrap();
        return;
    }
    let service =
        RepositoryService::open_at(std::path::Path::new(json["data"].as_str().unwrap())).unwrap();
    service.set_request_hook_for_testing(|| {
        println!("TASK8-ACCEPTED");
        std::io::stdout().flush().unwrap();
        let mut release = [0];
        std::io::stdin().read_exact(&mut release).unwrap();
    });
    let result = mutation::execute_with(
        &service,
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(LocalContextResolveInput {
            root: json["root"].as_str().unwrap().into(),
            operation_id: OperationId::parse(json["operation"].as_str().unwrap()).unwrap(),
            observation: json["observation"].as_str().unwrap().into(),
            resolutions: vec![LocalConflictResolutionInput {
                token: json["token"].as_str().unwrap().into(),
                source: json["source"].as_str().unwrap().into(),
            }],
        }),
    );
    // A terminal winner's immutable receipt is returned by F2 settlement;
    // the stale executor itself cannot write or settle it again.
    assert_eq!(result.code, ResultCode::Ok);
}

enum LocalChildCommand {
    Release,
    Abort,
}

struct LocalChildReport {
    status: std::process::ExitStatus,
    termination: &'static str,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl LocalChildReport {
    fn diagnostics(&self) -> String {
        format!(
            "{} ({})\nstdout:\n{}\nstderr:\n{}",
            self.status,
            self.termination,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

// Only the supervisor owns Child: a blocked main-thread library call cannot
// prevent the watchdog from killing/reaping this particular fixture process.
struct LocalChildProcess {
    child: std::process::Child,
    reaped: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for LocalChildProcess {
    fn drop(&mut self) {
        if !self.reaped.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = self.child.kill();
            if self.child.wait().is_ok() {
                self.reaped.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
}

struct GuardedLocalChild {
    commands: std::sync::mpsc::Sender<LocalChildCommand>,
    ready: std::sync::mpsc::Receiver<()>,
    worker: Option<std::thread::JoinHandle<LocalChildReport>>,
    report: Option<LocalChildReport>,
    reaped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    deadline: std::time::Instant,
}

impl GuardedLocalChild {
    fn spawn(payload: serde_json::Value, timeout: std::time::Duration) -> Self {
        use std::io::{BufRead, Read, Write};
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        };
        let reaped = Arc::new(AtomicBool::new(false));
        let mut process = LocalChildProcess {
            child: std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "local_sync::local_sync_process_owner_child",
                    "--nocapture",
                ])
                .env("MANYHANDS_TASK8_OWNER_CHILD", payload.to_string())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
            reaped: reaped.clone(),
        };
        let deadline = std::time::Instant::now() + timeout;
        let (ready_tx, ready) = mpsc::channel();
        let stdout = process.child.stdout.take().unwrap();
        let stderr = process.child.stderr.take().unwrap();
        let (commands, receive) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            // Drain both pipes concurrently so diagnostics cannot fill a pipe
            // and prevent either the explicit barrier or process completion.
            let out = std::thread::spawn(move || {
                let mut reader = std::io::BufReader::new(stdout);
                let mut bytes = Vec::new();
                loop {
                    let mut line = Vec::new();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            if line == b"TASK8-ACCEPTED\n" || line == b"TASK8-ACCEPTED\r\n" {
                                let _ = ready_tx.send(());
                            }
                            bytes.extend(line);
                        }
                        Err(error) => {
                            bytes.extend(format!("stdout capture failed: {error}").as_bytes());
                            break;
                        }
                    }
                }
                bytes
            });
            let err = std::thread::spawn(move || {
                let mut bytes = Vec::new();
                if let Err(error) = std::io::BufReader::new(stderr).read_to_end(&mut bytes) {
                    bytes.extend(format!("stderr capture failed: {error}").as_bytes());
                }
                bytes
            });
            let (status, termination) = loop {
                if let Some(status) = process.child.try_wait().unwrap() {
                    break (status, "exited");
                }
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                let termination = if remaining.is_zero() {
                    "deadline"
                } else {
                    // This timeout only polls liveness/deadline; ordering proof
                    // is the child's flushed pipe barrier, never a guessed delay.
                    match receive.recv_timeout(remaining.min(std::time::Duration::from_millis(10)))
                    {
                        Ok(LocalChildCommand::Release) => {
                            if let Some(mut stdin) = process.child.stdin.take() {
                                let _ = stdin.write_all(b"x");
                            }
                            continue;
                        }
                        Ok(LocalChildCommand::Abort)
                        | Err(mpsc::RecvTimeoutError::Disconnected) => "aborted",
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    }
                };
                let _ = process.child.kill();
                break (process.child.wait().unwrap(), termination);
            };
            process.reaped.store(true, Ordering::SeqCst);
            LocalChildReport {
                status,
                termination,
                stdout: out.join().unwrap(),
                stderr: err.join().unwrap(),
            }
        });
        Self {
            commands,
            ready,
            worker: Some(worker),
            report: None,
            reaped,
            deadline,
        }
    }

    fn collect(&mut self) -> &LocalChildReport {
        if self.report.is_none() {
            self.report = Some(
                self.worker
                    .take()
                    .unwrap()
                    .join()
                    .expect("child supervisor panicked"),
            );
        }
        self.report.as_ref().unwrap()
    }

    fn wait_ready(&mut self) -> Result<(), String> {
        match self.ready.recv_timeout(
            self.deadline
                .saturating_duration_since(std::time::Instant::now()),
        ) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = self.commands.send(LocalChildCommand::Abort);
                Err(format!(
                    "child readiness failed: {error}\n{}",
                    self.collect().diagnostics()
                ))
            }
        }
    }

    fn finish(&mut self) -> &LocalChildReport {
        let _ = self.commands.send(LocalChildCommand::Release);
        self.collect()
    }
}

impl Drop for GuardedLocalChild {
    fn drop(&mut self) {
        let _ = self.commands.send(LocalChildCommand::Abort);
        if let Some(worker) = self.worker.take() {
            // Join before the surrounding repository/data fixture can be removed.
            // Do not cause a second panic while unwinding a failed assertion.
            let _ = worker.join();
        }
    }
}

#[test]
fn local_sync_child_harness_bounds_readiness_and_captures_early_exit() {
    for fault in ["readiness", "early_exit"] {
        let mut child = GuardedLocalChild::spawn(
            serde_json::json!({"harness_fault": fault}),
            std::time::Duration::from_secs(2),
        );
        let error = child.wait_ready().unwrap_err();
        assert!(error.contains("TASK8-FAULT-STDOUT"), "{error}");
        assert!(error.contains("TASK8-FAULT-STDERR"), "{error}");
        let report = child.report.as_ref().unwrap();
        if fault == "early_exit" {
            assert_eq!(report.status.code(), Some(23));
        } else {
            assert!(!report.status.success());
        }
        assert!(child.reaped.load(std::sync::atomic::Ordering::SeqCst));
    }
}

#[test]
fn local_sync_child_harness_bounds_completion_after_release() {
    let mut child = GuardedLocalChild::spawn(
        serde_json::json!({"harness_fault": "completion"}),
        std::time::Duration::from_secs(2),
    );
    child.wait_ready().unwrap();
    let report = child.finish();
    assert_eq!(report.termination, "deadline", "{}", report.diagnostics());
    assert!(!report.status.success());
    assert!(report.diagnostics().contains("TASK8-FAULT-STDERR"));
    assert!(child.reaped.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn local_sync_child_harness_reaps_on_unwind_before_fixture_cleanup() {
    let f = ConflictWorld::new();
    let before = super::support::repository_git_file_bytes(&f.world.fixture);
    let mut child = GuardedLocalChild::spawn(
        serde_json::json!({"mode":"lease", "root": f.world.root}),
        std::time::Duration::from_secs(30),
    );
    child.wait_ready().unwrap();
    let reaped = child.reaped.clone();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _guard = child;
            panic!("fixture assertion before release");
        }))
        .is_err()
    );
    assert!(reaped.load(std::sync::atomic::Ordering::SeqCst));
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(
            f.world
                .fixture
                .repository
                .commondir()
                .join("manyhands-operation.lock"),
        )
        .unwrap();
    assert!(fs4::fs_std::FileExt::try_lock_exclusive(&lock).unwrap());
    fs4::fs_std::FileExt::unlock(&lock).unwrap();
    assert_eq!(
        before,
        super::support::repository_git_file_bytes(&f.world.fixture)
    );
}

#[test]
fn local_sync_child_harness_watchdog_releases_lease_while_caller_is_blocked() {
    let f = ConflictWorld::new();
    let before = super::support::repository_git_file_bytes(&f.world.fixture);
    let mut child = GuardedLocalChild::spawn(
        serde_json::json!({"mode":"lease", "root": f.world.root}),
        std::time::Duration::from_secs(2),
    );
    child.wait_ready().unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(
            f.world
                .fixture
                .repository
                .commondir()
                .join("manyhands-operation.lock"),
        )
        .unwrap();
    // A real blocking caller, with no release command. The independently owned
    // watchdog must terminate/reap the child and thereby release its OS lease.
    assert!(
        !fs4::fs_std::FileExt::try_lock_exclusive(&lock).unwrap(),
        "child still holds the lease before the blocking call"
    );
    fs4::fs_std::FileExt::lock_exclusive(&lock).unwrap();
    fs4::fs_std::FileExt::unlock(&lock).unwrap();
    let report = child.finish();
    assert_eq!(report.termination, "deadline", "{}", report.diagnostics());
    assert!(child.reaped.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(
        before,
        super::support::repository_git_file_bytes(&f.world.fixture)
    );
}

#[test]
fn local_sync_cross_process_attempt_loss_cannot_write_or_settle_after_winner() {
    let f = ConflictWorld::new();
    let input = f.input();
    let payload = serde_json::json!({"data":f.world.data.path(),"root":f.world.root,"operation":f.operation.to_string(),"observation":input.observation,"token":input.resolutions[0].token,"source":input.resolutions[0].source});
    let mut child = GuardedLocalChild::spawn(payload, std::time::Duration::from_secs(60));
    child.wait_ready().unwrap();
    assert_eq!(f.world.record(mutation::REQUEST_3).unwrap().attempt, 1);
    assert_eq!(
        f.world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextResolve(copy_input(&input))
            )
            .code,
        ResultCode::Ok
    );
    let rows = task8_authority_image(&f.world);
    let before = super::support::repository_and_worktree_snapshot(&f.world.fixture);
    let artifacts = super::support::repository_git_file_bytes(&f.world.fixture);
    let result = child.finish();
    assert_eq!(result.termination, "exited", "{}", result.diagnostics());
    assert!(result.status.success(), "{}", result.diagnostics());
    assert_eq!(rows, task8_authority_image(&f.world));
    assert_eq!(
        before,
        super::support::repository_and_worktree_snapshot(&f.world.fixture)
    );
    assert_eq!(
        artifacts,
        super::support::repository_git_file_bytes(&f.world.fixture)
    );
}

#[test]
fn local_sync_cross_process_lease_refuses_resolution_without_second_authority() {
    let f = ConflictWorld::new();
    let input = f.input();
    let payload = serde_json::json!({"mode":"lease","root":f.world.root});
    // Includes the exact lease-file bytes. Windows mandatory locks forbid
    // reading them while the child owns the lease, so snapshot before spawn.
    let rows = task8_authority_image(&f.world);
    let before = super::support::repository_and_worktree_snapshot(&f.world.fixture);
    let artifacts = super::support::repository_git_file_bytes(&f.world.fixture);
    let mut child = GuardedLocalChild::spawn(payload, std::time::Duration::from_secs(60));
    child.wait_ready().unwrap();
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    let completed = child.finish();
    assert_eq!(
        completed.termination,
        "exited",
        "{}",
        completed.diagnostics()
    );
    assert!(completed.status.success(), "{}", completed.diagnostics());
    assert_eq!(result.code, ResultCode::Busy);
    assert_eq!(rows, task8_authority_image(&f.world));
    assert_eq!(
        before,
        super::support::repository_and_worktree_snapshot(&f.world.fixture)
    );
    assert_eq!(
        artifacts,
        super::support::repository_git_file_bytes(&f.world.fixture)
    );
    assert_eq!(
        f.world
            .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(input))
            .code,
        ResultCode::Ok
    );
}

fn frozen_pair_public(remote: bool) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let mut world = mutation::World::new();
    assert_eq!(
        world
            .execute(
                mutation::REQUEST_1,
                world.save(items::TICKET_A, &world.token(items::TICKET_A))
            )
            .code,
        ResultCode::Ok
    );
    if remote {
        let spy = calls.clone();
        unsafe {
            git2::transport::register("manyhands-task8-public", move |_| {
                spy.fetch_add(1, Ordering::SeqCst);
                Err(git2::Error::from_str("fixture transport refuses"))
            })
            .unwrap();
        }
        // Fixture-only protocol, as in the domain transport-spy control. The
        // public remote setter correctly requires SSH; local sync must not
        // consult transport even when this directly configured endpoint exists.
        world
            .fixture
            .repository
            .remote(
                "origin",
                "manyhands-task8-public://unreachable.invalid/repository",
            )
            .unwrap();
        let path = world.root.join(".manyhands/config.toml");
        let config = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, format!("{config}\npublication_remote = \"origin\"\n")).unwrap();
        items::commit(
            &world.fixture,
            &[".manyhands/config.toml"],
            items::COMMITTED_AT,
        );
        items::refresh_completely(&world.service, &world.root);
    }
    items::write(&world.root, "incoming", "frozen incoming");
    items::commit(&world.fixture, &["incoming"], items::COMMITTED_AT);
    let observation = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(items::TICKET_A),
        )
        .unwrap();
    let start = || {
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(items::TICKET_A),
            observation: observation.observation.clone(),
        })
    };
    let failing = super::support::FailOnce::at(FailurePoint::LocalSyncAfterIntent)
        .open_service(world.data.path());
    assert_ne!(
        mutation::execute_with(&failing, mutation::REQUEST_2, start()).code,
        ResultCode::Ok
    );
    items::write(&world.root, "later", "later committed primary");
    items::commit(&world.fixture, &["later"], items::COMMITTED_AT);
    let primary = world.fixture.repository.head().unwrap().target().unwrap();
    assert_eq!(
        world.execute(mutation::REQUEST_2, start()).code,
        ResultCode::Ok
    );
    let context = git2::Repository::open(world.worktree(items::TICKET_A)).unwrap();
    let first = context.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(
        first.parent_id(1).unwrap().to_string(),
        observation.primary_head
    );
    assert!(!context.workdir().unwrap().join("later").exists());
    assert_eq!(
        world.fixture.repository.head().unwrap().target(),
        Some(primary)
    );
    let before = super::support::repository_and_worktree_snapshot(&world.fixture);
    let artifacts = super::support::repository_git_file_bytes(&world.fixture);
    let rows = task8_authority_image(&world);
    assert_eq!(
        world.execute(mutation::REQUEST_2, start()).code,
        ResultCode::Ok
    );
    assert_eq!(rows, task8_authority_image(&world));
    assert_eq!(
        before,
        super::support::repository_and_worktree_snapshot(&world.fixture)
    );
    assert_eq!(
        artifacts,
        super::support::repository_git_file_bytes(&world.fixture)
    );
    let fresh = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(items::TICKET_A),
        )
        .unwrap();
    assert_eq!(
        world
            .execute(
                mutation::REQUEST_3,
                Mutation::LocalContextSync(LocalContextSyncInput {
                    root: world.root.clone(),
                    kind: AuthoringKind::Ticket,
                    id: items::item_id(items::TICKET_A),
                    observation: fresh.observation
                })
            )
            .code,
        ResultCode::Ok
    );
    assert!(context.workdir().unwrap().join("later").is_file());
    assert_eq!(
        world.fixture.repository.head().unwrap().target(),
        Some(primary)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    if remote {
        assert!(
            world
                .fixture
                .repository
                .find_remote("origin")
                .unwrap()
                .fetch(&["main"], None, None)
                .is_err()
        );
        assert!(
            calls.load(Ordering::SeqCst) > 0,
            "live transport negative control"
        );
    }
    world.reopen();
}

#[test]
fn local_sync_public_frozen_pair_without_any_remote() {
    frozen_pair_public(false);
}

#[test]
fn local_sync_public_frozen_pair_with_unreachable_configured_remote() {
    frozen_pair_public(true);
}

#[test]
fn local_sync_public_malformed_companion_refuses_without_repair_or_acceptance() {
    let f = ConflictWorld::new();
    let input = f.input();
    items::index(f.world.data.path())
        .execute(
            "ALTER TABLE local_context_sync_records ADD COLUMN unexpected TEXT",
            [],
        )
        .unwrap();
    let rows = task8_authority_image(&f.world);
    let before = super::support::repository_and_worktree_snapshot(&f.world.fixture);
    let artifacts = super::support::repository_git_file_bytes(&f.world.fixture);
    assert!(
        f.world
            .service
            .inspect_local_context_sync(&f.world.resolved(), f.operation)
            .is_err()
    );
    let result = f
        .world
        .execute(mutation::REQUEST_3, Mutation::LocalContextResolve(input));
    assert_eq!(result.code, ResultCode::RecoveryRequired);
    assert!(f.world.record(mutation::REQUEST_3).is_none());
    assert_eq!(rows, task8_authority_image(&f.world));
    assert_eq!(
        before,
        super::support::repository_and_worktree_snapshot(&f.world.fixture)
    );
    assert_eq!(
        artifacts,
        super::support::repository_git_file_bytes(&f.world.fixture)
    );
}

#[test]
fn local_sync_public_immutable_closure_race_leaves_no_owner_and_allows_other_work() {
    let world = mutation::World::new();
    assert_eq!(
        world
            .execute(
                mutation::REQUEST_1,
                world.save(items::TICKET_A, &world.token(items::TICKET_A))
            )
            .code,
        ResultCode::Ok
    );
    let path = items::ticket_path(items::TICKET_A);
    let context = git2::Repository::open(world.worktree(items::TICKET_A)).unwrap();
    for (git, who) in [(&context, "One"), (&world.fixture.repository, "Two")] {
        let file = git.workdir().unwrap().join(&path);
        let source = std::fs::read_to_string(&file).unwrap().replacen(
            "---\n",
            &format!("---\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: {who}\n"),
            1,
        );
        manyhands::canonical::parse_item(std::path::Path::new(&path), &source).unwrap();
        std::fs::write(file, source).unwrap();
        let mut index = git.index().unwrap();
        index.add_path(std::path::Path::new(&path)).unwrap();
        index.write().unwrap();
        let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = git.head().unwrap().peel_to_commit().unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        git.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "independent closure",
            &tree,
            &[&parent],
        )
        .unwrap();
    }
    let observation = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(items::TICKET_A),
        )
        .unwrap();
    let rows = task8_authority_image(&world);
    let before = super::support::repository_and_worktree_snapshot(&world.fixture);
    let artifacts = super::support::repository_git_file_bytes(&world.fixture);
    let result = world.execute(
        mutation::REQUEST_2,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(items::TICKET_A),
            observation: observation.observation,
        }),
    );
    assert_eq!(result.code, ResultCode::RecoveryRequired);
    let MutationDataDto::LocalContextSync(dto) = result.data.unwrap() else {
        panic!("local result");
    };
    assert_eq!(
        dto.reason,
        LocalContextSyncReason::ImmutableClosureDisagreement
    );
    assert!(world.record(mutation::REQUEST_2).is_none());
    assert_eq!(rows, task8_authority_image(&world));
    assert_eq!(
        before,
        super::support::repository_and_worktree_snapshot(&world.fixture)
    );
    assert_eq!(
        artifacts,
        super::support::repository_git_file_bytes(&world.fixture)
    );
    assert!(
        world
            .service
            .refresh_repository(RefreshRepositoryRequest {
                root: world.root.clone(),
                operation_id: OperationId::new()
            })
            .is_ok()
    );
}
