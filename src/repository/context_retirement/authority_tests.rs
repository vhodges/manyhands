use super::*;
use crate::repository::context_retirement::execution_tests::registry_image;
use crate::repository::keys::*;
use crate::repository::*;

struct Fixture {
    _root: tempfile::TempDir,
    _data: tempfile::TempDir,
    service: RepositoryService,
    request: RetireContextRequest,
    owner: Box<RemoteReservation>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let git = git2::Repository::init(root.path()).unwrap();
        git.set_head("refs/heads/main").unwrap();
        let tree = git
            .find_tree(git.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        git.commit(Some("HEAD"), &sig, &sig, "fixture", &tree, &[])
            .unwrap();
        git.remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
            .unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let config = root.path().join(".manyhands/config.toml");
        let mut text = std::fs::read_to_string(&config).unwrap();
        text.push_str("publication_remote = \"origin\"\n");
        std::fs::write(config, text).unwrap();
        let item = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
        let request = RetireContextRequest {
            root: root.path().canonicalize().unwrap(),
            kind: AuthoringKind::Ticket,
            item_id: item,
            operation_id: OperationId::new(),
        };
        let tip = git.head().unwrap().target().unwrap();
        git.reference(
            &format!("refs/heads/manyhands/ticket/{}", request.item_id),
            tip,
            false,
            "fixture",
        )
        .unwrap();
        let plan = crate::repository::remote::observation::observation_plan(&request.root).unwrap();
        let configuration = service
            .observation_configuration(&request.root, &plan)
            .unwrap();
        let lease =
            repository_lease(&git, &request.root, RepositoryOperation::RetireContext).unwrap();
        let BranchClaim::Execute(owner) =
            claim_branch(&service, &request, tip, plan, configuration).unwrap()
        else {
            panic!("valid claim")
        };
        drop(lease);
        Self {
            _root: root,
            _data: data,
            service,
            request,
            owner,
        }
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self._data.path().join(REGISTRY_FILE)).unwrap()
    }
    fn before_batch(&self) {
        for point in [
            RemoteOperationSafePoint::BeforeTransport,
            RemoteOperationSafePoint::AfterAdvertisement,
            RemoteOperationSafePoint::BeforeBatchCommit,
        ] {
            assert_eq!(
                self.service
                    .remote_safe_point(&self.request.root, &self.owner, point)
                    .unwrap(),
                RemoteSafePointOutcome::Continue
            );
        }
    }

    // Metadata audit fixture: real claim and owned returned-batch receipt,
    // followed by CHECK/trigger-valid intent/handoff mutations, not Git evidence.
    fn branch_intent(&self) {
        self.before_batch();
        reservation::commit_observation_batch(
            &self.service,
            &self.request.root,
            &self.owner,
            self.owner.retirement_execution().unwrap().plan(),
            &[],
            2,
        )
        .unwrap();
        self.service
            .finish_remote_operation(
                &self.request.root,
                &self.owner,
                RemoteOutcomeCategory::Completed,
            )
            .unwrap();
        let db = self.db();
        let oid = git2::Repository::open(&self.request.root)
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap()
            .to_string();
        db.execute("UPDATE context_retirement_records SET initial_primary_oid=?1,phase='branch_deleting',branch_removal='possible'",[&oid]).unwrap();
        db.execute("INSERT INTO context_retirement_effect_intents(operation_ulid,owner_epoch,resource,receipt_id,primary_oid,proof_kind) SELECT c.operation_ulid,c.owner_epoch,'branch',b.id,?1,'ancestor' FROM context_retirement_records c JOIN retirement_observation_receipts b ON b.consumer_operation_ulid=c.operation_ulid",[oid]).unwrap();
        super::super::audit_connection(&db).unwrap();
    }
}
struct Provider;
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

#[test]
fn retirement_live_public_observer_cannot_configure_a_reserved_private_id() {
    let f = Fixture::new();
    std::fs::write(
        f.request.root.join(".manyhands/config.toml"),
        b"invalid = [",
    )
    .unwrap();
    let before = registry_image(&f.db());
    let state = || {
        f.db().query_row("SELECT p.configuration_generation,o.phase FROM remote_polling_state p JOIN remote_operation_records o ON o.repository_id=p.repository_id WHERE o.operation_ulid=?1",[f.owner.operation_id().to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).unwrap()
    };
    let original = state();
    let result = f.service.observe_publication_remote(
        ObservePublicationRemoteRequest {
            root: f.request.root.clone(),
            operation_id: f.owner.operation_id(),
            invocation: RemotePollInvocation::Explicit,
            approval: None,
            restart: true,
        },
        &mut SessionCredentials::new(Provider),
    );
    assert_eq!(
        state(),
        original,
        "public reserved-ID misuse changed the frozen generation/observer"
    );
    assert!(
        matches!(result,Err(RemoteObservationError::Repository(ref e)) if e.kind==RepositoryErrorKind::RecoveryRequired),
        "public private-ID misuse must refuse before configuration writes"
    );
    assert_eq!(registry_image(&f.db()), before);
}

#[test]
fn retirement_live_owned_observer_checks_every_write_boundary() {
    for fence in [
        "epoch",
        "parent_terminal",
        "foreign_local",
        "configuration",
        "observer_epoch",
    ] {
        let f = Fixture::new();
        f.before_batch();
        let db = f.db();
        match fence {
            "epoch" => {
                db.execute("UPDATE context_retirement_attempts SET phase='fenced'", [])
                    .unwrap();
                db.execute("UPDATE context_retirement_records SET owner_epoch=2", [])
                    .unwrap();
                db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) SELECT operation_ulid,2,repository_id,'active' FROM context_retirement_records",[]).unwrap();
            }
            "parent_terminal" => {
                let report =
                    ContextRetirementReport::refused(crate::results::ResultCode::ExternalChange);
                db.execute("UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=?1",[report.encode().unwrap()]).unwrap();
                db.execute("UPDATE context_retirement_attempts SET phase='ended'", [])
                    .unwrap();
                db.execute(
                    "UPDATE operation_records SET state='completed' WHERE action='retire_context'",
                    [],
                )
                .unwrap();
            }
            "foreign_local" => {
                db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,observed_at) SELECT id,root_path,?1,'refresh','created',1 FROM repositories",[OperationId::new().to_string()]).unwrap();
            }
            "configuration" => {
                let path = f.request.root.join(".manyhands/config.toml");
                let text = std::fs::read_to_string(&path).unwrap().replace(
                    "publication_remote = \"origin\"",
                    "publication_remote = \"foreign\"",
                );
                std::fs::write(path, text).unwrap();
            }
            "observer_epoch" => {
                db.execute("UPDATE remote_operation_records SET owner_epoch=owner_epoch+1 WHERE operation_ulid=?1",[f.owner.operation_id().to_string()]).unwrap();
            }
            _ => unreachable!(),
        }
        let before = registry_image(&db);
        assert!(
            f.service
                .remote_safe_point(
                    &f.request.root,
                    &f.owner,
                    RemoteOperationSafePoint::BeforeBatchCommit
                )
                .is_err(),
            "{fence}: safe point"
        );
        assert!(
            f.service
                .finish_remote_operation(
                    &f.request.root,
                    &f.owner,
                    RemoteOutcomeCategory::TransportUnavailable
                )
                .is_err(),
            "{fence}: error/completion"
        );
        assert!(
            reservation::commit_observation_batch(
                &f.service,
                &f.request.root,
                &f.owner,
                f.owner.retirement_execution().unwrap().plan(),
                &[],
                1
            )
            .is_err(),
            "{fence}: advertisement"
        );
        assert!(
            state::with_transaction(&f.service, &f.request.root, |tx, id| {
                reservation::owned(&f.service, tx, id, &f.owner)?;
                state::configure(tx, id, None, false)
            })
            .is_err(),
            "{fence}: configuration write"
        );
        assert_eq!(
            registry_image(&db),
            before,
            "{fence}: no write or receipt by stale execution"
        );
    }
}

#[test]
fn retirement_live_advertisement_captures_exact_batch_and_public_replay_cannot_mint() {
    let f = Fixture::new();
    f.before_batch();
    let plan = f.owner.retirement_execution().unwrap().plan();
    reservation::commit_observation_batch(&f.service, &f.request.root, &f.owner, plan, &[], 2)
        .unwrap();
    let db = f.db();
    let exact:bool=db.query_row("SELECT p.observation_batch_id=b.id AND p.source_operation_ulid=?1 AND pin.binding_id=r.id FROM retirement_advertisement_provenance p JOIN remote_observation_batches b ON b.id=p.observation_batch_id JOIN retirement_observation_receipts r ON r.provenance_id=p.id JOIN retirement_observation_pins pin ON pin.provenance_id=p.id WHERE b.is_current=1",[f.owner.operation_id().to_string()],|r|r.get(0)).unwrap();
    assert!(exact);
    f.service
        .finish_remote_operation(&f.request.root, &f.owner, RemoteOutcomeCategory::Completed)
        .unwrap();
    let before = registry_image(&db);
    assert!(
        f.service
            .reserve_remote_operation(
                &f.request.root,
                f.owner.operation_id(),
                &RemoteOperationTarget::for_poll(plan)
            )
            .is_err()
    );
    assert!(
        f.service
            .restart_remote_observation(
                &f.request.root,
                f.owner.operation_id(),
                &RemoteOperationTarget::for_poll(plan)
            )
            .is_err()
    );
    assert!(
        f.service
            .remote_safe_point(
                &f.request.root,
                &f.owner,
                RemoteOperationSafePoint::AfterBatchCommit
            )
            .is_err()
    );
    assert_eq!(registry_image(&db), before);
    assert!(RepositoryService::open_at(f._data.path()).is_ok());
}

#[test]
fn retirement_spec_s1_branch_handoff_and_terminal_require_settled_matching_effect() {
    let mut accepted = Vec::new();
    for terminal in [false, true] {
        for observation in [
            None,
            Some("removed"),
            Some("already_absent"),
            Some("incomplete"),
            Some("changed"),
            Some("not_clean"),
        ] {
            for flag in ["observed", "possible"] {
                let f = Fixture::new();
                f.branch_intent();
                let db = f.db();
                if let Some(observation) = observation {
                    db.execute(
                        "UPDATE context_retirement_effect_intents SET observation=?1",
                        [observation],
                    )
                    .unwrap();
                }
                db.execute("UPDATE context_retirement_records SET phase='index_pending',absence_basis='owned_intent',materialization_settled=1,branch_removal=?1",[flag]).unwrap();
                db.execute("UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released' WHERE action='retire_context'",[]).unwrap();
                if terminal {
                    let removal = if flag == "observed" {
                        RetirementRemoval::Observed
                    } else {
                        RetirementRemoval::Possible
                    };
                    let report = ContextRetirementReport::retired(RetirementRemoval::None, removal);
                    db.execute("UPDATE context_retirement_records SET phase='complete',final_code='ok',final_result=?1",[report.encode().unwrap()]).unwrap();
                    db.execute("UPDATE context_retirement_attempts SET phase='ended'", [])
                        .unwrap();
                    db.execute("UPDATE operation_records SET state='completed' WHERE action='retire_context'",[]).unwrap();
                }
                let valid = matches!(
                    (observation, flag),
                    (Some("removed"), "observed")
                        | (Some("already_absent" | "incomplete"), "possible")
                );
                let before = registry_image(&db);
                let audited = super::super::audit_connection(&db);
                let opened = RepositoryService::open_at(f._data.path());
                if valid {
                    assert!(
                        audited.is_ok() && opened.is_ok(),
                        "valid {terminal}/{observation:?}/{flag}"
                    );
                } else {
                    if audited.is_ok()
                        || !matches!(opened,Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
                    {
                        accepted.push((terminal, observation, flag));
                    }
                    let retry = f.service.retire_context(f.request.clone());
                    if !matches!(retry,Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
                    {
                        accepted.push((terminal, observation, flag));
                    }
                }
                assert_eq!(
                    registry_image(&db),
                    before,
                    "audit/startup/index retry must retain corrupt history"
                );
                assert!(
                    git2::Repository::open(&f.request.root)
                        .unwrap()
                        .find_reference(&format!(
                            "refs/heads/manyhands/ticket/{}",
                            f.request.item_id
                        ))
                        .is_ok()
                );
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "unsettled/mismatching effect authorized handoff or terminal: {accepted:?}"
    );
}

#[test]
fn retirement_spec_s3_public_update_rejects_parent_and_observer_before_configuration_writes() {
    let mut failures = Vec::new();
    for terminal in [false, true] {
        for observer in [false, true] {
            for valid_config in [false, true] {
                let f = Fixture::new();
                if terminal {
                    refuse_execution(
                        &f.service,
                        &f.request.root,
                        &f.owner,
                        crate::results::ResultCode::ExternalChange,
                    )
                    .unwrap();
                }
                let git = git2::Repository::open(&f.request.root).unwrap();
                git.remote("other", "ssh://fixture@127.0.0.1:1/fixture")
                    .unwrap();
                let path = f.request.root.join(".manyhands/config.toml");
                let text = if valid_config {
                    std::fs::read_to_string(&path).unwrap().replace(
                        "publication_remote = \"origin\"",
                        "publication_remote = \"other\"",
                    )
                } else {
                    "invalid = [".into()
                };
                std::fs::write(path, text).unwrap();
                let before = registry_image(&f.db());
                let result = f.service.poll_publication_remote(
                    PollPublicationRemoteRequest {
                        root: f.request.root.clone(),
                        operation_id: if observer {
                            f.owner.operation_id()
                        } else {
                            f.request.operation_id
                        },
                        invocation: RemotePollInvocation::Explicit,
                        approval: None,
                        restart: true,
                    },
                    &mut SessionCredentials::new(Provider),
                );
                if !matches!(result,Err(RemotePollError::Repository(ref e)) if e.kind==RepositoryErrorKind::RecoveryRequired)
                    || registry_image(&f.db()) != before
                {
                    failures.push((terminal, observer, valid_config));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "reserved update-poll ID wrote configuration/metadata or failed to refuse (terminal,observer,valid): {failures:?}"
    );
}
