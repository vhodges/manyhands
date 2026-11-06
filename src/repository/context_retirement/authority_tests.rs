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
        Self::with_request(false)
    }
    fn with_request(bound: bool) -> Self {
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
        let claimed = if bound {
            // INERT canonical accepted-F2 projection, not a mutation binding.
            use crate::repository::mutation::{RequestId, request_store::*};
            let id = RequestId::new();
            let scope = ScopeKey::for_repository(&request.root).unwrap();
            let digest = IntentDigest::builder(
                DigestSalt::Request(id),
                "context retire",
                &scope,
                &request.item_id.to_string(),
            )
            .field("kind", FieldValue::Text("ticket"))
            .finish();
            service
                .insert_request(&NewRequest {
                    request_id: id,
                    scope,
                    command: "context retire".into(),
                    target: request.item_id.to_string(),
                    intent_digest: digest,
                    confirmation: None,
                    base_ref: None,
                    base_oid: None,
                    expected_digest: None,
                    operations: vec![RequestOperation {
                        family: OperationFamily::Local,
                        operation_id: request.operation_id,
                    }],
                })
                .unwrap();
            let row = service.request_record(id).unwrap().unwrap();
            let accepted =
                crate::repository::RetirementAccepted::of_record(&row, row.attempt).unwrap();
            let scope =
                super::super::super::request_scope::Scope::capture(&service, &request, &accepted)
                    .unwrap();
            claim_scoped(&service, &request, tip, plan, configuration, None, scope)
        } else {
            claim_branch(&service, &request, tip, plan, configuration)
        };
        let BranchClaim::Execute(owner) = claimed.unwrap() else {
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
    fn index_handoff(&self) -> i64 {
        // Inert effect/handoff metadata over a REAL owned observer receipt;
        // this tests index SQL authority, not a destructive Git journey.
        self.branch_intent();
        let db = self.db();
        db.execute_batch("UPDATE context_retirement_effect_intents SET observation='removed'; UPDATE context_retirement_records SET phase='index_pending',absence_basis='owned_intent',materialization_settled=1,branch_removal='observed'; UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released' WHERE action='retire_context';").unwrap();
        super::super::audit_connection(&db).unwrap();
        db.query_row(
            "SELECT local_record_id FROM context_retirement_records",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }
    fn index_owner(&self) -> recovery::IndexOwner {
        self.index_handoff();
        let fence = self
            .owner
            .retirement_execution()
            .unwrap()
            .request_scope()
            .fence()
            .unwrap();
        self.service
            .set_refresh_claim_hook_for_testing(|| std::panic::resume_unwind(Box::new(())));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = self
                    .service
                    .refresh_retirement_request(&self.request, fence);
            }))
            .is_err()
        );
        let (record_id,epoch)=self.db().query_row("SELECT local_record_id,index_owner_epoch FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        recovery::IndexOwner {
            record_id,
            epoch,
            retirement_request: Some(fence),
            retirement_batch: None,
        }
    }
}

#[test]
fn retirement_f2_index_envelope_is_checked_before_every_owned_write() {
    let mut failures = Vec::new();
    for sql in [
        "UPDATE operation_records SET action='refresh' WHERE action='retire_context'",
        "UPDATE operation_records SET target='authoring-context-v1/document/01ARZ3NDEKTSV4RRFFQ69G5FAV/Retire' WHERE action='retire_context'",
        "UPDATE operation_records SET root_path='/foreign' WHERE action='retire_context'",
        "UPDATE operation_records SET repository_id=NULL WHERE action='retire_context'",
        "UPDATE operation_records SET completed_step='context_retirement_absent' WHERE action='retire_context'",
    ] {
        let f = Fixture::with_request(true);
        let owner = f.index_owner();
        let db = f.db();
        assert!(recovery::owns_indexing(&db, owner.clone()).unwrap());
        db.execute_batch(sql).unwrap();
        let before = registry_image(&db);
        if recovery::owns_indexing(&db, owner.clone()).is_ok() {
            failures.push((sql, "ownership"));
        }
        if recovery::touch_indexing(&db, owner.clone()).is_ok() {
            failures.push((sql, "heartbeat"));
        }
        if recovery::record_owned_persisted_context(&db, owner.clone(), &f.request.root).is_ok() {
            failures.push((sql, "persistence"));
        }
        if recovery::transition_indexing(&db, owner.clone(), "failed", None).is_ok() {
            failures.push((sql, "error"));
        }
        if before != registry_image(&db) {
            failures.push((sql, "wrote before refusal"));
        }
    }
    assert!(
        failures.is_empty(),
        "mismatched envelope authorized index writes: {failures:?}"
    );
}

#[test]
fn retirement_f2_index_guard_keeps_ordinary_nullable_target_claim_semantics() {
    let f = Fixture::new();
    let db = f.db();
    db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT id,root_path,?1,'refresh',NULL,'authoritative_observed',1 FROM repositories",[OperationId::new().to_string()]).unwrap();
    let id = db.last_insert_rowid();
    let owner = recovery::claim_indexing(&db, id, 0)
        .expect("ordinary refresh may have a NULL target")
        .unwrap();
    assert!(owner.retirement_request.is_none());
    assert!(recovery::owns_indexing(&db, owner.clone()).unwrap());
    assert!(recovery::touch_indexing(&db, owner).unwrap());
}

#[test]
fn retirement_f2_index_guard_preserves_domain_only_explicit_invalidation_reclaim() {
    let f = Fixture::new();
    let local = f.index_handoff();
    let db = f.db();
    let old = recovery::claim_indexing(&db, local, 0).unwrap().unwrap();
    state::with_transaction(&f.service, &f.request.root, |tx, id| {
        recovery::invalidate_index_observations(tx, id)
    })
    .unwrap();
    let new =
        recovery::reclaim_invalidated_indexing(&db, old.clone(), f.request.root.to_str().unwrap())
            .expect("existing domain-only reclaim remains valid")
            .unwrap();
    assert_eq!(new.epoch, old.epoch + 2);
    assert!(new.retirement_request.is_none());
    assert!(recovery::owns_indexing(&db, new).unwrap());
    assert!(recovery::touch_indexing(&db, old).is_err());
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_index_claims",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn retirement_f2_unscoped_index_claim_cannot_write_request_bound_retirement() {
    let f = Fixture::with_request(true);
    let local = f.index_handoff();
    let db = f.db();
    let before = registry_image(&db);
    assert!(
        recovery::claim_indexing(&db, local, i64::MAX).is_err(),
        "generic claim must not mint an unscoped owner for a bound retirement"
    );
    assert_eq!(before, registry_image(&db));
}

#[test]
fn retirement_f2_index_sql_rejoins_current_request_at_every_write_and_finalizer() {
    use crate::repository::mutation::{RequestId, request_store::*};
    for case in [
        "advance",
        "finished",
        "missing_mapping",
        "wrong_ordinal",
        "wrong_family",
        "foreign_request",
        "wrong_request_owner",
        "invalidated",
    ] {
        let f = Fixture::with_request(true);
        let mut owner = f.index_owner();
        let db = f.db();
        assert!(recovery::owns_indexing(&db, owner.clone()).unwrap());
        let id: String = db
            .query_row("SELECT request_ulid FROM request_records", [], |r| r.get(0))
            .unwrap();
        let id = RequestId::parse(&id).unwrap();
        match case {
            "invalidated" => {
                state::with_transaction(&f.service, &f.request.root, |tx, id| {
                    recovery::invalidate_index_observations(tx, id)
                })
                .unwrap();
                f.service.enter_request(id).unwrap().unwrap();
            }
            "advance" => {
                f.service.enter_request(id).unwrap().unwrap();
            }
            "finished" => {
                assert!(
                    f.service
                        .finish_request(
                            id,
                            1,
                            &RequestResult {
                                outcome: crate::results::Outcome::Cancelled,
                                code: crate::results::ResultCode::Cancelled,
                                effects: crate::results::Effects::not_requested(),
                                data: None
                            }
                        )
                        .unwrap()
                );
            }
            "missing_mapping" => {
                db.execute("DELETE FROM request_operations", []).unwrap();
            }
            "wrong_ordinal" => {
                db.execute("UPDATE request_operations SET ordinal=1", [])
                    .unwrap();
            }
            "wrong_family" => {
                db.execute("UPDATE request_operations SET family='remote'", [])
                    .unwrap();
            }
            "foreign_request" => {
                db.execute("INSERT INTO request_records(request_ulid,attempt,scope_key,command,target,intent_digest,state,accepted_at) SELECT ?1,attempt,scope_key,command,target,intent_digest,state,accepted_at FROM request_records",[RequestId::new().to_string()]).unwrap();
                db.execute("UPDATE request_operations SET request_ulid=(SELECT request_ulid FROM request_records WHERE request_ulid!=?1)",[id.to_string()]).unwrap();
            }
            _ => {
                let foreign = Fixture::with_request(true);
                owner.retirement_request = foreign
                    .owner
                    .retirement_execution()
                    .unwrap()
                    .request_scope()
                    .fence();
            }
        }
        let before = registry_image(&db);
        assert!(
            recovery::owns_indexing(&db, owner.clone()).is_err(),
            "{case}"
        );
        assert!(
            recovery::touch_indexing(&db, owner.clone()).is_err(),
            "{case}"
        );
        assert!(
            recovery::transition_indexing(&db, owner.clone(), "failed", None).is_err(),
            "{case}"
        );
        assert!(
            recovery::record_owned_persisted_context(&db, owner.clone(), &f.request.root).is_err(),
            "{case}"
        );
        assert!(
            recovery::reclaim_invalidated_indexing(
                &db,
                owner.clone(),
                f.request.root.to_str().unwrap()
            )
            .is_err(),
            "{case}"
        );
        assert!(
            state::with_transaction(&f.service, &f.request.root, |tx, id| {
                super::super::finalize_index(tx, owner.clone(), id, &f.request.root)
            })
            .is_err(),
            "{case}"
        );
        assert_eq!(
            before,
            registry_image(&db),
            "{case}: no stale touch/error/persistence/completion/Z"
        );
    }
}

#[test]
fn retirement_f2_index_claim_is_durable_single_use_even_after_invalidation() {
    use crate::repository::mutation::RequestId;
    let f = Fixture::with_request(true);
    let old = f.index_owner();
    let db = f.db();
    state::with_transaction(&f.service, &f.request.root, |tx, id| {
        recovery::invalidate_index_observations(tx, id)
    })
    .unwrap();
    let before = registry_image(&db);
    assert!(matches!(
        f.service
            .refresh_retirement_request(&f.request, old.retirement_request.unwrap()),
        Ok(RefreshOutcome::IndexPending { .. })
    ));
    assert_eq!(
        before,
        registry_image(&db),
        "invalidation does not unspend this accepted F2 call"
    );
    assert!(
        db.execute(
            "UPDATE context_retirement_index_claims SET index_owner_epoch=index_owner_epoch+1",
            []
        )
        .is_err()
    );
    assert!(
        db.execute("DELETE FROM context_retirement_index_claims", [])
            .is_err()
    );
    let id: String = db
        .query_row("SELECT request_ulid FROM request_records", [], |r| r.get(0))
        .unwrap();
    let id = RequestId::parse(&id).unwrap();
    assert_eq!(f.service.enter_request(id).unwrap(), Some(2));
    assert!(
        RepositoryService::open_at(f._data.path()).is_ok(),
        "historical claims remain history after F2 moves"
    );
    assert!(recovery::touch_indexing(&db, old).is_err());
    assert_eq!(
        before.get("context_retirement_index_claims"),
        registry_image(&db).get("context_retirement_index_claims")
    );
}

#[test]
fn retirement_f2_orphaned_companion_never_becomes_unfenced_generic_index_authority() {
    // Explicit metadata corruption fixture, not a supported deletion journey.
    // Drop only the target refusal trigger, then restore canonical DDL before
    // exercising ANY owner/refresh/audit. No CHECK constraint is disabled.
    let f = Fixture::with_request(true);
    let owner = f.index_owner();
    let db = f.db();
    let trigger: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='retirement_record_delete'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    db.execute_batch(
        "DROP TRIGGER retirement_record_delete; DELETE FROM context_retirement_records;",
    )
    .unwrap();
    db.execute_batch(&trigger).unwrap();
    for disguise in [false, true] {
        if disguise {
            db.execute(
                "UPDATE operation_records SET action='refresh' WHERE id=?1",
                [owner.record_id],
            )
            .unwrap();
            db.execute("DELETE FROM request_operations", []).unwrap();
        }
        let before = registry_image(&db);
        assert!(recovery::owns_indexing(&db, owner.clone()).is_err());
        assert!(recovery::touch_indexing(&db, owner.clone()).is_err());
        assert!(recovery::claim_indexing(&db, owner.record_id, i64::MAX).is_err());
        assert!(
            f.service
                .refresh_repository(RefreshRepositoryRequest {
                    root: f.request.root.clone(),
                    operation_id: f.request.operation_id
                })
                .is_err()
        );
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_f2_finished_replay_rejects_zero_attempt_without_writes() {
    use crate::repository::mutation::{RequestId, request_store::RequestResult};
    let f = Fixture::with_request(true);
    let id: String = f
        .db()
        .query_row("SELECT request_ulid FROM request_records", [], |r| r.get(0))
        .unwrap();
    let id = RequestId::parse(&id).unwrap();
    let row = f.service.request_record(id).unwrap().unwrap();
    let mut accepted = RetirementAccepted::of_record(&row, row.attempt).unwrap();
    let report = refuse_execution(
        &f.service,
        &f.request.root,
        &f.owner,
        crate::results::ResultCode::Cancelled,
    )
    .unwrap();
    assert!(
        f.service
            .finish_request(
                id,
                1,
                &RequestResult {
                    outcome: crate::results::Outcome::Cancelled,
                    code: report.code,
                    effects: crate::results::Effects::not_requested(),
                    data: None
                }
            )
            .unwrap()
    );
    let before = registry_image(&f.db());
    assert!(
        matches!(f.service.retire_context_for_request(f.request.clone(),&accepted,None,&mut SessionCredentials::new(Provider)),Ok(ContextRetirementOutcome::Final(r)) if r==report)
    );
    accepted.attempt = 0;
    assert!(
        f.service
            .retire_context_for_request(
                f.request.clone(),
                &accepted,
                None,
                &mut SessionCredentials::new(Provider)
            )
            .is_err(),
        "zero was never an accepted execution attempt"
    );
    assert_eq!(before, registry_image(&f.db()));
}

#[test]
fn retirement_f2_history_and_finished_replay_reject_extra_immutable_metadata() {
    use crate::repository::mutation::{RequestId, request_store::RequestResult};
    let mut allowed = Vec::new();
    for sql in [
        "UPDATE request_records SET confirmation_ulid='01ARZ3NDEKTSV4RRFFQ69G5FAV'",
        "UPDATE request_records SET base_ref='refs/heads/main'",
        "UPDATE request_records SET base_oid='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'",
        "UPDATE request_records SET expected_digest='missing'",
    ] {
        let f = Fixture::with_request(true);
        let db = f.db();
        let id: String = db
            .query_row("SELECT request_ulid FROM request_records", [], |r| r.get(0))
            .unwrap();
        let id = RequestId::parse(&id).unwrap();
        let row = f.service.request_record(id).unwrap().unwrap();
        let accepted = RetirementAccepted::of_record(&row, row.attempt).unwrap();
        let report = refuse_execution(
            &f.service,
            &f.request.root,
            &f.owner,
            crate::results::ResultCode::Cancelled,
        )
        .unwrap();
        assert!(
            f.service
                .finish_request(
                    id,
                    1,
                    &RequestResult {
                        outcome: crate::results::Outcome::Cancelled,
                        code: report.code,
                        effects: crate::results::Effects::not_requested(),
                        data: None
                    }
                )
                .unwrap()
        );
        db.execute_batch(sql).unwrap();
        let before = registry_image(&db);
        if super::super::audit_connection(&db).is_ok() {
            allowed.push((sql, "history"));
        }
        if f.service
            .retire_context_for_request(
                f.request.clone(),
                &accepted,
                None,
                &mut SessionCredentials::new(Provider),
            )
            .is_ok()
        {
            allowed.push((sql, "finished replay"));
        }
        assert_eq!(before, registry_image(&db));
    }
    assert!(
        allowed.is_empty(),
        "unexpected immutable request fields accepted: {allowed:?}"
    );
}

#[test]
fn retirement_f2_all_old_remote_and_local_write_boundaries_require_actual_current_scope() {
    use crate::repository::mutation::{RequestId, request_store::RequestResult};
    for case in [
        "advance",
        "finished",
        "mapping_missing",
        "wrong_ordinal",
        "wrong_family",
        "duplicate",
        "foreign_request",
        "foreign_command",
        "wrong_scope",
        "wrong_target",
        "wrong_digest",
        "extra_metadata",
    ] {
        let f = Fixture::with_request(true);
        f.before_batch();
        let db = f.db();
        let id: String = db
            .query_row("SELECT request_ulid FROM request_records", [], |r| r.get(0))
            .unwrap();
        let id = RequestId::parse(&id).unwrap();
        state::with_transaction(&f.service, &f.request.root, |tx, id| {
            f.owner
                .retirement_execution()?
                .validate_local(&f.service, tx, id)
        })
        .unwrap();
        match case {
            "advance" => {
                f.service.enter_request(id).unwrap().unwrap();
                db.execute("UPDATE request_records SET cancel_requested=1", [])
                    .unwrap();
            }
            "finished" => {
                assert!(
                    f.service
                        .finish_request(
                            id,
                            1,
                            &RequestResult {
                                outcome: crate::results::Outcome::Cancelled,
                                code: crate::results::ResultCode::Cancelled,
                                effects: crate::results::Effects::not_requested(),
                                data: None
                            }
                        )
                        .unwrap()
                );
            }
            "mapping_missing" => {
                db.execute("DELETE FROM request_operations", []).unwrap();
            }
            "wrong_ordinal" => {
                db.execute("UPDATE request_operations SET ordinal=1", [])
                    .unwrap();
            }
            "wrong_family" => {
                db.execute("UPDATE request_operations SET family='remote'", [])
                    .unwrap();
            }
            "duplicate" => {
                db.execute("INSERT INTO request_operations SELECT request_ulid,1,family,operation_ulid FROM request_operations",[]).unwrap();
            }
            "foreign_request" => {
                db.execute("INSERT INTO request_records(request_ulid,attempt,scope_key,command,target,intent_digest,state,accepted_at) SELECT ?1,attempt,scope_key,command,target,intent_digest,state,accepted_at FROM request_records",[RequestId::new().to_string()]).unwrap();
                db.execute("UPDATE request_operations SET request_ulid=(SELECT request_ulid FROM request_records WHERE request_ulid!=?1)",[id.to_string()]).unwrap();
            }
            "foreign_command" => {
                db.execute("UPDATE request_records SET command='context release'", [])
                    .unwrap();
            }
            "wrong_scope" => {
                db.execute("UPDATE request_records SET scope_key='/foreign'", [])
                    .unwrap();
            }
            "wrong_target" => {
                db.execute(
                    "UPDATE request_records SET target='01ARZ3NDEKTSV4RRFFQ69G5FAW'",
                    [],
                )
                .unwrap();
            }
            "extra_metadata" => {
                db.execute("UPDATE request_records SET expected_digest='missing'", [])
                    .unwrap();
            }
            _ => {
                db.execute(
                    "UPDATE request_records SET intent_digest=?1",
                    ["0".repeat(64)],
                )
                .unwrap();
            }
        }
        let before = registry_image(&db);
        assert!(
            state::with_transaction(&f.service, &f.request.root, |tx, id| f
                .owner
                .retirement_execution()?
                .validate_install(&f.service, tx, id))
            .is_err(),
            "{case}: token installation/verification"
        );
        for point in [
            RemoteOperationSafePoint::BeforeTransport,
            RemoteOperationSafePoint::AfterAdvertisement,
            RemoteOperationSafePoint::BeforeBatchCommit,
            RemoteOperationSafePoint::AfterBatchCommit,
        ] {
            assert!(
                f.service
                    .remote_safe_point(&f.request.root, &f.owner, point)
                    .is_err(),
                "{case}"
            );
        }
        for outcome in [
            RemoteOutcomeCategory::Completed,
            RemoteOutcomeCategory::TransportUnavailable,
        ] {
            assert!(
                f.service
                    .finish_remote_operation(&f.request.root, &f.owner, outcome)
                    .is_err(),
                "{case}"
            );
        }
        assert!(
            reservation::commit_observation_batch(
                &f.service,
                &f.request.root,
                &f.owner,
                f.owner.retirement_execution().unwrap().plan(),
                &[],
                2
            )
            .is_err(),
            "{case}"
        );
        assert!(
            refuse_execution(
                &f.service,
                &f.request.root,
                &f.owner,
                crate::results::ResultCode::ExternalChange
            )
            .is_err(),
            "{case}: stale cancellation/refusal must not acknowledge"
        );
        assert!(
            state::with_transaction(&f.service, &f.request.root, |tx, id| {
                reservation::owned(&f.service, tx, id, &f.owner)?;
                state::configure(tx, id, None, false)
            })
            .is_err(),
            "{case}"
        );
        assert_eq!(
            before,
            registry_image(&db),
            "{case}: stale owner must not write configuration, receipts, errors, cancellation ack or completion"
        );
    }
}
struct Provider;
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

fn specfix_r3_observer(stage: &str) -> Fixture {
    let f = Fixture::new();
    if stage == "reserved" {
        return f;
    }
    f.service
        .remote_safe_point(
            &f.request.root,
            &f.owner,
            RemoteOperationSafePoint::BeforeTransport,
        )
        .unwrap();
    if stage == "advertising" {
        return f;
    }
    f.service
        .remote_safe_point(
            &f.request.root,
            &f.owner,
            RemoteOperationSafePoint::AfterAdvertisement,
        )
        .unwrap();
    f.service
        .remote_safe_point(
            &f.request.root,
            &f.owner,
            RemoteOperationSafePoint::BeforeBatchCommit,
        )
        .unwrap();
    if stage == "persisting" {
        return f;
    }
    reservation::commit_observation_batch(
        &f.service,
        &f.request.root,
        &f.owner,
        f.owner.retirement_execution().unwrap().plan(),
        &[],
        2,
    )
    .unwrap();
    match stage {
        "interrupted" => {
            f.db()
                .execute("UPDATE remote_operation_records SET yield_requested=1", [])
                .unwrap();
            assert_eq!(
                f.service
                    .remote_safe_point(
                        &f.request.root,
                        &f.owner,
                        RemoteOperationSafePoint::AfterBatchCommit
                    )
                    .unwrap(),
                RemoteSafePointOutcome::Interrupted
            );
        }
        "interrupted_unavailable" => {
            // Valid durable stop projection, not a fabricated CHECK-bypassed pair.
            f.db().execute("UPDATE remote_operation_records SET phase='interrupted',outcome='repository_unavailable'", []).unwrap();
        }
        _ => {
            f.service
                .finish_remote_operation(
                    &f.request.root,
                    &f.owner,
                    match stage {
                        "completed" => RemoteOutcomeCategory::Completed,
                        "cancelled" => RemoteOutcomeCategory::Cancelled,
                        _ => RemoteOutcomeCategory::TransportUnavailable,
                    },
                )
                .unwrap();
        }
    }
    f
}

#[test]
fn retirement_specfix_r3_terminal_history_preserved_while_every_old_owner_is_fenced() {
    let mut relabeled = Vec::new();
    for stage in [
        "reserved",
        "advertising",
        "persisting",
        "completed",
        "failed",
        "cancelled",
        "interrupted",
        "interrupted_unavailable",
    ] {
        let f = specfix_r3_observer(stage);
        let db = f.db();
        assert!(
            RepositoryService::open_at(f._data.path()).is_ok(),
            "valid {stage} fixture"
        );
        let reenter = || {
            f.service.reenter_context_retirement_authenticated(
                ReenterContextRetirementRequest {
                    retirement: f.request.clone(),
                    expected_owner_epoch: 1,
                },
                None,
                &mut SessionCredentials::new(Provider),
            )
        };
        // Failure after the O update must undo the epoch, phase, parent and Z.
        db.execute_batch("CREATE TRIGGER specfix_replacement_rollback BEFORE INSERT ON context_retirement_reentries BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        let before = registry_image(&db);
        assert!(reenter().is_err());
        assert_eq!(
            registry_image(&db),
            before,
            "{stage}: atomic replacement rollback"
        );
        db.execute_batch("DROP TRIGGER specfix_replacement_rollback")
            .unwrap();
        let before = registry_image(&db);
        let id = db
            .query_row("SELECT id FROM repositories", [], |r| r.get::<_, i64>(0))
            .unwrap();
        let old = state::read_operation(&db, id, f.owner.operation_id())
            .unwrap()
            .unwrap();
        let old_epoch = old.owner_epoch;
        let old_report = RemoteOperationInspection::from(old);
        let result = reenter().unwrap();
        assert!(matches!(result, ContextRetirementOutcome::Final(_)));
        assert_eq!(
            db.query_row(
                "SELECT phase FROM context_retirement_attempts WHERE owner_epoch=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "fenced"
        );
        let new = state::read_operation(&db, id, f.owner.operation_id())
            .unwrap()
            .unwrap();
        assert_eq!(
            new.owner_epoch,
            old_epoch + 1,
            "{stage}: fence even terminal O"
        );
        let report = RemoteOperationInspection::from(new);
        let active = matches!(stage, "reserved" | "advertising" | "persisting");
        if active {
            assert_eq!(report.phase(), RemoteOperationPhase::Interrupted);
            assert_eq!(
                report.outcome(),
                Some(RemoteOutcomeCategory::RepositoryUnavailable)
            );
        } else if report.phase() != old_report.phase() || report.outcome() != old_report.outcome() {
            relabeled.push(stage);
        }
        assert_eq!(report.completed_step(), old_report.completed_step());
        assert_eq!(report.yield_requested(), old_report.yield_requested());
        assert_eq!(report.cancel_requested(), old_report.cancel_requested());
        // Compare every O column, not just its public outcome projection.
        let columns = db
            .prepare("SELECT * FROM remote_operation_records")
            .unwrap()
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let after = registry_image(&db);
        for (column, (old, new)) in columns.iter().zip(
            before["remote_operation_records"][0]
                .iter()
                .zip(&after["remote_operation_records"][0]),
        ) {
            if column != "owner_epoch"
                && !matches!((active, column.as_str()), (true, "phase" | "outcome"))
                && old != new
            {
                relabeled.push(stage);
            }
        }
        assert_eq!(
            before["remote_polling_state"],
            after["remote_polling_state"]
        );
        for point in [
            RemoteOperationSafePoint::BeforeTransport,
            RemoteOperationSafePoint::AfterAdvertisement,
            RemoteOperationSafePoint::BeforeBatchCommit,
            RemoteOperationSafePoint::AfterBatchCommit,
        ] {
            assert!(
                f.service
                    .remote_safe_point(&f.request.root, &f.owner, point)
                    .is_err()
            );
        }
        assert!(
            f.service
                .finish_remote_operation(
                    &f.request.root,
                    &f.owner,
                    RemoteOutcomeCategory::Completed
                )
                .is_err()
        );
        assert!(
            reservation::commit_observation_batch(
                &f.service,
                &f.request.root,
                &f.owner,
                f.owner.retirement_execution().unwrap().plan(),
                &[],
                3
            )
            .is_err()
        );
        assert!(
            refuse_execution(
                &f.service,
                &f.request.root,
                &f.owner,
                crate::results::ResultCode::ExternalChange
            )
            .is_err()
        );
        assert_eq!(
            registry_image(&db),
            after,
            "{stage}: stale calls cannot write"
        );
        assert!(RepositoryService::open_at(f._data.path()).is_ok());
    }
    assert!(
        relabeled.is_empty(),
        "terminal observer history changed: {relabeled:?}"
    );
}

#[test]
fn retirement_reentry_audit_rejects_unfenced_observer_and_false_purposes() {
    for case in [
        "old_observer_not_fenced",
        "preserve_without_debt",
        "observe_branch_only",
    ] {
        let f = Fixture::new();
        let db = f.db();
        db.execute("UPDATE context_retirement_attempts SET phase='fenced'", [])
            .unwrap();
        db.execute("UPDATE context_retirement_records SET owner_epoch=2", [])
            .unwrap();
        db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) SELECT operation_ulid,2,repository_id,'active' FROM context_retirement_records",[]).unwrap();
        let purpose = match case {
            "old_observer_not_fenced" => "no_effect",
            "preserve_without_debt" => "preserve",
            _ => "observe",
        };
        db.execute("INSERT INTO context_retirement_reentries(operation_ulid,owner_epoch,previous_epoch,purpose) VALUES(?1,2,1,?2)",params![f.request.operation_id.to_string(),purpose]).unwrap();
        if case == "old_observer_not_fenced" {
            let report =
                ContextRetirementReport::refused(crate::results::ResultCode::ExternalChange);
            db.execute("UPDATE context_retirement_records SET phase='refused',final_code='external_change',final_result=?1",[report.encode().unwrap()]).unwrap();
            db.execute_batch("UPDATE context_retirement_attempts SET phase='ended' WHERE owner_epoch=2; UPDATE operation_records SET state='completed' WHERE action='retire_context';").unwrap();
        } else {
            db.execute("UPDATE remote_operation_records SET owner_epoch=owner_epoch+1,phase='interrupted',outcome='repository_unavailable' WHERE operation_ulid=?1",[f.owner.operation_id().to_string()]).unwrap();
        }
        if case == "observe_branch_only" {
            let op = OperationId::new();
            state::with_transaction(&f.service,&f.request.root,|tx,id|{
                state::insert_operation(tx,id,op,&RemoteOperationTarget::for_poll(f.owner.retirement_execution()?.plan()),RemoteOperationPriority::Manual,1)?;
                tx.execute("UPDATE context_retirement_attempts SET observer_operation_ulid=?1,observer_owner_epoch=0 WHERE owner_epoch=2",[op.to_string()]).map_err(RepositoryError::sqlite)?;Ok(())
            }).unwrap();
        }
        let before = registry_image(&db);
        assert!(super::super::audit_connection(&db).is_err(), "{case}");
        assert!(
            matches!(RepositoryService::open_at(f._data.path()),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired),
            "{case}"
        );
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_reentry_explicit_replacement_fences_all_old_write_boundaries() {
    for stage in ["reserved", "advertised", "batch", "completed"] {
        let f = Fixture::new();
        if stage != "reserved" {
            f.service
                .remote_safe_point(
                    &f.request.root,
                    &f.owner,
                    RemoteOperationSafePoint::BeforeTransport,
                )
                .unwrap();
            f.service
                .remote_safe_point(
                    &f.request.root,
                    &f.owner,
                    RemoteOperationSafePoint::AfterAdvertisement,
                )
                .unwrap();
        }
        if matches!(stage, "batch" | "completed") {
            f.service
                .remote_safe_point(
                    &f.request.root,
                    &f.owner,
                    RemoteOperationSafePoint::BeforeBatchCommit,
                )
                .unwrap();
            reservation::commit_observation_batch(
                &f.service,
                &f.request.root,
                &f.owner,
                f.owner.retirement_execution().unwrap().plan(),
                &[],
                1,
            )
            .unwrap();
        }
        if stage == "completed" {
            f.service
                .finish_remote_operation(
                    &f.request.root,
                    &f.owner,
                    RemoteOutcomeCategory::Completed,
                )
                .unwrap();
        }
        let result = f
            .service
            .reenter_context_retirement_authenticated(
                ReenterContextRetirementRequest {
                    retirement: f.request.clone(),
                    expected_owner_epoch: 1,
                },
                None,
                &mut SessionCredentials::new(Provider),
            )
            .unwrap();
        assert!(
            matches!(result,ContextRetirementOutcome::Final(ref r) if r.code==crate::results::ResultCode::ExternalChange)
        );
        let before = registry_image(&f.db());
        for point in [
            RemoteOperationSafePoint::BeforeTransport,
            RemoteOperationSafePoint::AfterAdvertisement,
            RemoteOperationSafePoint::BeforeBatchCommit,
            RemoteOperationSafePoint::AfterBatchCommit,
        ] {
            assert!(
                f.service
                    .remote_safe_point(&f.request.root, &f.owner, point)
                    .is_err()
            );
        }
        assert!(
            f.service
                .finish_remote_operation(
                    &f.request.root,
                    &f.owner,
                    RemoteOutcomeCategory::TransportUnavailable
                )
                .is_err()
        );
        assert!(
            reservation::commit_observation_batch(
                &f.service,
                &f.request.root,
                &f.owner,
                f.owner.retirement_execution().unwrap().plan(),
                &[],
                2
            )
            .is_err()
        );
        assert!(
            refuse_execution(
                &f.service,
                &f.request.root,
                &f.owner,
                crate::results::ResultCode::ExternalChange
            )
            .is_err()
        );
        assert!(
            state::with_transaction(&f.service, &f.request.root, |tx, id| {
                reservation::owned(&f.service, tx, id, &f.owner)?;
                state::configure(tx, id, None, false)
            })
            .is_err()
        );
        assert_eq!(before, registry_image(&f.db()));
        assert!(RepositoryService::open_at(f._data.path()).is_ok());
    }
}

#[test]
fn retirement_reentry_exclusions_and_fixed_inputs_cannot_replace_an_owner() {
    for case in [
        "local",
        "comment",
        "observe",
        "update",
        "request_mapping",
        "wrong_epoch",
        "wrong_target",
    ] {
        let f = Fixture::new();
        let db = f.db();
        let mut request = f.request.clone();
        if matches!(case, "observe" | "update") {
            f.before_batch();
            reservation::commit_observation_batch(
                &f.service,
                &f.request.root,
                &f.owner,
                f.owner.retirement_execution().unwrap().plan(),
                &[],
                1,
            )
            .unwrap();
            f.service
                .finish_remote_operation(
                    &f.request.root,
                    &f.owner,
                    RemoteOutcomeCategory::Completed,
                )
                .unwrap();
        }
        match case {
            "local" | "comment" => {
                db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,completed_step,observed_at) SELECT id,root_path,?1,'refresh',?2,?3,1 FROM repositories",params![OperationId::new().to_string(),if case=="local"{"created"}else{"completed"},(case=="comment").then_some("comment_registration_pending")]).unwrap();
            }
            "observe" | "update" => {
                db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,created_at,updated_at,poll_kind) SELECT repository_id,?1,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,'poll','manual','reserved',1,1,?2 FROM remote_operation_records WHERE operation_ulid=?3",params![OperationId::new().to_string(),if case=="observe"{"observe"}else{"update"},f.owner.operation_id().to_string()]).unwrap();
            }
            "request_mapping" => {
                let r = OperationId::new().to_string();
                db.execute("INSERT INTO request_records(request_ulid,attempt,scope_key,command,target,intent_digest,state,accepted_at) VALUES(?1,1,?2,'context retire',?3,?4,'accepted',1)",params![r,f.request.root.to_str(),f.request.item_id.to_string(),"1".repeat(64)]).unwrap();
                db.execute(
                    "INSERT INTO request_operations VALUES(?1,0,'local',?2)",
                    params![r, f.request.operation_id.to_string()],
                )
                .unwrap();
            }
            "wrong_target" => request.kind = AuthoringKind::Document,
            _ => {}
        }
        let before = registry_image(&db);
        assert!(
            f.service
                .reenter_context_retirement_authenticated(
                    ReenterContextRetirementRequest {
                        retirement: request,
                        expected_owner_epoch: if case == "wrong_epoch" { 2 } else { 1 }
                    },
                    None,
                    &mut SessionCredentials::new(Provider)
                )
                .is_err(),
            "{case}"
        );
        assert_eq!(registry_image(&db), before, "{case}");
    }
}

#[test]
fn retirement_reentry_owned_absence_requires_exact_receipt_not_purpose_or_initial_step() {
    for case in ["missing_receipt", "initial_step", "receipt_flags"] {
        // Real domain claim/advertisement capability, inert durable effect
        // projection as in branch_intent(); this is an audit, not a Git proof.
        let f = Fixture::new();
        f.branch_intent();
        let git = git2::Repository::open(&f.request.root).unwrap();
        git.find_reference(&format!(
            "refs/heads/manyhands/ticket/{}",
            f.request.item_id
        ))
        .unwrap()
        .delete()
        .unwrap();
        let result = f
            .service
            .reenter_context_retirement_authenticated(
                ReenterContextRetirementRequest {
                    retirement: f.request.clone(),
                    expected_owner_epoch: 1,
                },
                None,
                &mut SessionCredentials::new(Provider),
            )
            .unwrap();
        assert!(matches!(result, ContextRetirementOutcome::Final(_)));
        let db = f.db();
        assert!(
            db.execute(
                "UPDATE context_retirement_absence_receipts SET branch_removal='observed'",
                []
            )
            .is_err(),
            "immutable receipt"
        );
        match case {
            "missing_receipt" => {
                db.execute("DELETE FROM context_retirement_absence_receipts", [])
                    .unwrap();
            }
            "initial_step" => {
                db.execute("UPDATE operation_records SET completed_step='context_retirement_absent' WHERE action='retire_context'",[]).unwrap();
            }
            _ => {
                let sql:String=db.query_row("SELECT sql FROM sqlite_master WHERE name='retirement_absence_receipt_immutable'",[],|r|r.get(0)).unwrap();
                db.execute_batch("DROP TRIGGER retirement_absence_receipt_immutable; UPDATE context_retirement_absence_receipts SET branch_removal='observed';").unwrap();
                db.execute_batch(&sql).unwrap();
            }
        }
        let before = registry_image(&db);
        assert!(super::super::audit_connection(&db).is_err(), "{case}");
        assert!(
            matches!(RepositoryService::open_at(f._data.path()),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired),
            "{case}"
        );
        assert_eq!(registry_image(&db), before);
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
