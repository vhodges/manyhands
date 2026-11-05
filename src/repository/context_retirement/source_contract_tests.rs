//! INERT canonical A0/A1 projection fixtures, NOT Task 4 producer/migration or
//! coordinator evidence. No worklist/eligibility/child permission exists here.
use super::*;
use crate::repository::remote::reservation::RemotePollReservationOutcome;
use crate::repository::{
    context_retirement::record,
    mutation::{RequestId, request_store::*},
    remote::{observation::observation_plan, state},
};

// The private SQL projection contract is deliberately installed ONLY by tests.
// Production must refuse absent future producer records/marker, never create them.
fn projections(db: &Connection) {
    db.execute_batch("CREATE TABLE poll_retirement_batches (
      operation_ulid TEXT PRIMARY KEY NOT NULL, repository_id INTEGER NOT NULL,
      request_ulid TEXT NOT NULL, mode TEXT NOT NULL CHECK(mode='retire_completed'),
      configuration_generation INTEGER NOT NULL, remote_name TEXT NOT NULL,
      primary_branch TEXT NOT NULL, endpoint_digest BLOB NOT NULL CHECK(length(endpoint_digest)=32),
      base_oid TEXT NOT NULL, owner_epoch INTEGER NOT NULL CHECK(owner_epoch>0),
      stage TEXT NOT NULL CHECK(stage='unplanned'));
      CREATE TABLE poll_retirement_attempts (
      operation_ulid TEXT NOT NULL REFERENCES poll_retirement_batches(operation_ulid),
      owner_epoch INTEGER NOT NULL CHECK(owner_epoch>0), request_attempt INTEGER NOT NULL CHECK(request_attempt>0),
      source_owner_epoch INTEGER NOT NULL CHECK(source_owner_epoch>=0),
      phase TEXT NOT NULL CHECK(phase IN ('active','ended','fenced')),
      active_provenance_id INTEGER REFERENCES retirement_advertisement_provenance(id),
      PRIMARY KEY(operation_ulid,owner_epoch));
      INSERT INTO registry_migrations(name) VALUES('poll_retirement_source_contract_v1');").unwrap();
}

pub(super) struct SourceFixture {
    pub(super) f: Fixture,
    pub(super) operation: OperationId,
    pub(super) request_id: RequestId,
    pub(super) plan: RemoteRefPlan,
    pub(super) owner: Option<RemoteReservation>,
}
impl SourceFixture {
    pub(super) fn new() -> Self {
        let f = Fixture::new();
        let root = f.root.path().canonicalize().unwrap();
        git2::Repository::open(&root)
            .unwrap()
            .remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
            .unwrap();
        let path = root.join(".manyhands/config.toml");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("publication_remote = \"origin\"\n");
        std::fs::write(&path, text).unwrap();
        let plan = observation_plan(&root).unwrap();
        let config = f.service.observation_configuration(&root, &plan).unwrap();
        state::with_transaction(&f.service, &root, |tx, id| {
            state::configure_endpoints(tx, id, &plan, &config.endpoint_digest())
        })
        .unwrap();
        let operation = OperationId::new();
        let request_id = RequestId::new();
        let scope = ScopeKey::for_repository(&root).unwrap();
        let digest =
            IntentDigest::builder(DigestSalt::Request(request_id), "poll once", &scope, "")
                .field("retire_completed", FieldValue::Flag(true))
                .finish();
        let oid = git2::Repository::open(&root)
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap();
        f.service
            .insert_request(&NewRequest {
                request_id,
                scope,
                command: "poll once".into(),
                target: "".into(),
                intent_digest: digest,
                confirmation: None,
                base_ref: Some("refs/heads/main".into()),
                base_oid: Some(oid),
                expected_digest: None,
                operations: vec![RequestOperation {
                    family: OperationFamily::Remote,
                    operation_id: operation,
                }],
            })
            .unwrap();
        let RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Reserved(owner)) =
            f.service
                .reserve_remote_poll(
                    &root,
                    operation,
                    &RemoteOperationTarget::for_poll(&plan),
                    RemoteOperationPriority::Manual,
                )
                .unwrap()
        else {
            panic!("original P reserved")
        };
        let db = f.db();
        projections(&db);
        db.execute("INSERT INTO poll_retirement_batches SELECT ?1,repository_id,?2,'retire_completed',configuration_generation,remote_name,primary_branch,?3,?4,1,'unplanned' FROM remote_polling_state",params![operation.to_string(),request_id.to_string(),config.endpoint_digest().as_slice(),oid.to_string()]).unwrap();
        db.execute("INSERT INTO poll_retirement_attempts(operation_ulid,owner_epoch,request_attempt,source_owner_epoch,phase) SELECT operation_ulid,1,1,owner_epoch,'active' FROM remote_operation_records WHERE operation_ulid=?1",[operation.to_string()]).unwrap();
        Self {
            f,
            operation,
            request_id,
            plan,
            owner: Some(owner),
        }
    }
    fn seal(&mut self) -> Result<(), RepositoryError> {
        let row = self
            .f
            .service
            .request_record(self.request_id)
            .unwrap()
            .unwrap();
        let accepted = RetirementAccepted::of_record(&row, row.attempt).unwrap();
        self.owner = Some(self.f.service.seal_original_poll_source(
            self.f.root.path(),
            self.owner.take().unwrap(),
            &accepted,
        )?);
        Ok(())
    }
    fn before_batch(&self) {
        for point in [
            RemoteOperationSafePoint::BeforeTransport,
            RemoteOperationSafePoint::AfterAdvertisement,
            RemoteOperationSafePoint::BeforeBatchCommit,
        ] {
            assert_eq!(
                self.f
                    .service
                    .remote_safe_point(self.f.root.path(), self.owner.as_ref().unwrap(), point)
                    .unwrap(),
                RemoteSafePointOutcome::Continue
            );
        }
    }
    fn capture(&self) -> Result<RemoteSafePointOutcome, RepositoryError> {
        crate::repository::remote::reservation::commit_observation_batch(
            &self.f.service,
            self.f.root.path(),
            self.owner.as_ref().unwrap(),
            &self.plan,
            &[],
            43,
        )
    }
    fn project_capture(&self) -> (i64, i64) {
        // Inert A1 history used to drive audit RED before adding the real owned
        // capture seam. Exact returned SQL batch, never a public snapshot lookup.
        state::with_transaction(&self.f.service,self.f.root.path(),|tx,id|{
            let config=crate::repository::remote::observation::ObservationConfiguration::capture_owned(&self.f.service,tx,self.f.root.path(),&self.plan)?.endpoint_digest();
            let batch=state::persist_complete_advertisement(tx,id,&self.plan,state::generation(tx,id)?,&[],42)?;
            let remote=state::read_operation(tx,id,self.operation)?.unwrap();
            tx.execute("INSERT INTO retirement_advertisement_provenance(repository_id,source_operation_ulid,source_owner_epoch,observation_batch_id,observed_at,configuration_generation,remote_name,primary_branch,endpoint_digest,execution_kind,execution_parent_ulid,execution_parent_epoch,execution_request_ulid,execution_request_attempt,execution_batch_operation_ulid,execution_batch_attempt_epoch) VALUES(?1,?2,?3,?4,42,?5,?6,?7,?8,'poll_parent',?2,?3,?9,1,?2,1)",params![id,self.operation.to_string(),remote.owner_epoch,batch,remote.generation,self.plan.remote_name(),self.plan.primary_branch(),config.as_slice(),self.request_id.to_string()]).map_err(RepositoryError::sqlite)?;
            let provenance=tx.last_insert_rowid();
            tx.execute("INSERT INTO retirement_observation_pins(provenance_id,pin_kind,holder_operation_ulid,holder_epoch,repository_id,observation_batch_id) VALUES(?1,'source_attempt',?2,?3,?4,?5)",params![provenance,self.operation.to_string(),remote.owner_epoch,id,batch]).map_err(RepositoryError::sqlite)?;
            tx.execute("UPDATE poll_retirement_attempts SET active_provenance_id=?2 WHERE operation_ulid=?1",params![self.operation.to_string(),provenance]).map_err(RepositoryError::sqlite)?;
            Ok((provenance,batch))
        }).unwrap()
    }
}

#[test]
fn retirement_source_unplanned_provenance_audits_without_manifest_or_eligibility() {
    let s = SourceFixture::new();
    s.project_capture();
    let before = registry_image(&s.f.db());
    record::audit_connection(&s.f.db())
        .expect("valid unbound A1 source history, not deletion authority");
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    assert_eq!(before, registry_image(&s.f.db()));
    assert_eq!(
        s.f.db()
            .query_row(
                "SELECT count(*) FROM retirement_observation_receipts",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn retirement_source_missing_projection_or_pin_is_not_authority() {
    for case in ["projection", "pin"] {
        let s = SourceFixture::new();
        s.project_capture();
        let db = s.f.db();
        if case == "projection" {
            db.execute("DELETE FROM poll_retirement_attempts", [])
                .unwrap();
        } else {
            // Explicit corruption; exact canonical guard restored before audit.
            let ddl: String = db
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE name='retirement_pin_release_required'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            db.execute_batch("DROP TRIGGER retirement_pin_release_required;")
                .unwrap();
            db.execute("DELETE FROM retirement_observation_pins", [])
                .unwrap();
            db.execute_batch(&ddl).unwrap();
        }
        let before = registry_image(&db);
        assert!(
            matches!(record::audit_connection(&db),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        );
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_source_live_owned_capture_is_unbound_and_public_poll_remains_capture_free() {
    for explicit in [false, true] {
        let mut s = SourceFixture::new();
        if explicit {
            s.seal().unwrap();
        } else {
            // Ordinary-path control has no Task 4 source projection/producer.
            s.f.db().execute_batch("DROP TABLE poll_retirement_attempts; DROP TABLE poll_retirement_batches; DELETE FROM registry_migrations WHERE name='poll_retirement_source_contract_v1';").unwrap();
        }
        s.before_batch();
        assert_eq!(s.capture().unwrap(), RemoteSafePointOutcome::Continue);
        let db = s.f.db();
        let sources: i64 = db
            .query_row(
                "SELECT count(*) FROM retirement_advertisement_provenance",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            sources,
            i64::from(explicit),
            "explicit process-only capture, not the ordinary poll path"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM retirement_observation_receipts",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        if explicit {
            assert!(db.query_row("SELECT EXISTS(SELECT 1 FROM retirement_advertisement_provenance p JOIN remote_observation_batches b ON b.id=p.observation_batch_id JOIN poll_retirement_attempts a ON a.active_provenance_id=p.id JOIN retirement_observation_pins pin ON pin.provenance_id=p.id WHERE b.is_current=1 AND b.observed_at=43 AND pin.pin_kind='source_attempt' AND a.phase='active')",[],|r|r.get::<_,bool>(0)).unwrap());
            let before = registry_image(&db);
            assert!(
                s.f.service
                    .finish_remote_operation(
                        s.f.root.path(),
                        s.owner.as_ref().unwrap(),
                        RemoteOutcomeCategory::Completed
                    )
                    .is_err(),
                "A1 alone cannot finalize or mint A2/eligibility"
            );
            assert_eq!(before, registry_image(&db));
        }
        record::audit_connection(&db).unwrap();
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[test]
fn retirement_source_seal_requires_exact_current_request_projection_and_no_local_debt() {
    for case in [
        "marker",
        "projection",
        "family",
        "duplicate",
        "duplicate_source",
        "digest",
        "scope",
        "attempt",
        "base",
        "generation",
        "endpoint",
        "local_debt",
    ] {
        let mut s = SourceFixture::new();
        let db = s.f.db();
        match case {
            "marker" => {
                db.execute("DELETE FROM registry_migrations WHERE name='poll_retirement_source_contract_v1'",[]).unwrap();
            }
            "projection" => {
                db.execute("DELETE FROM poll_retirement_attempts", [])
                    .unwrap();
            }
            "family" => {
                db.execute("UPDATE request_operations SET family='local'", [])
                    .unwrap();
            }
            "duplicate" => {
                db.execute("INSERT INTO request_operations SELECT request_ulid,1,family,operation_ulid FROM request_operations",[]).unwrap();
            }
            "duplicate_source" => {
                db.execute("INSERT INTO poll_retirement_attempts SELECT operation_ulid,2,request_attempt,source_owner_epoch,phase,active_provenance_id FROM poll_retirement_attempts",[]).unwrap();
            }
            "digest" => {
                db.execute(
                    "UPDATE request_records SET intent_digest=?1",
                    ["0".repeat(64)],
                )
                .unwrap();
            }
            "scope" => {
                db.execute("UPDATE request_records SET scope_key='/foreign-root'", [])
                    .unwrap();
            }
            "attempt" => {
                s.f.service.enter_request(s.request_id).unwrap().unwrap();
            }
            "base" => {
                db.execute(
                    "UPDATE poll_retirement_batches SET base_oid=?1",
                    ["1".repeat(40)],
                )
                .unwrap();
            }
            "generation" => {
                db.execute("UPDATE poll_retirement_batches SET configuration_generation=configuration_generation+1",[]).unwrap();
            }
            "endpoint" => {
                db.execute(
                    "UPDATE poll_retirement_batches SET endpoint_digest=zeroblob(32)",
                    [],
                )
                .unwrap();
            }
            "local_debt" => {
                db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,state,observed_at) SELECT id,root_path,?1,'refresh','authoritative_observed',1 FROM repositories",[OperationId::new().to_string()]).unwrap();
            }
            _ => unreachable!(),
        }
        let before = registry_image(&db);
        assert!(s.seal().is_err(), "{case}");
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_source_every_owned_write_rejoins_f2_and_source_scope() {
    for case in ["attempt", "mapping", "generation", "batch_epoch"] {
        let mut s = SourceFixture::new();
        s.seal().unwrap();
        s.before_batch();
        let db = s.f.db();
        match case {
            "attempt" => {
                s.f.service.enter_request(s.request_id).unwrap().unwrap();
            }
            "mapping" => {
                db.execute("UPDATE request_operations SET family='local'", [])
                    .unwrap();
            }
            "generation" => {
                state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
                    state::configure(tx, id, None, true)
                })
                .unwrap();
            }
            "batch_epoch" => {
                db.execute(
                    "UPDATE poll_retirement_batches SET owner_epoch=owner_epoch+1",
                    [],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = registry_image(&db);
        assert!(s.capture().is_err(), "{case}");
        assert!(
            s.f.service
                .remote_safe_point(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    RemoteOperationSafePoint::BeforeBatchCommit
                )
                .is_err()
        );
        assert!(
            s.f.service
                .finish_remote_operation(
                    s.f.root.path(),
                    s.owner.as_ref().unwrap(),
                    RemoteOutcomeCategory::TransportUnavailable
                )
                .is_err()
        );
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_source_capture_and_stop_release_are_atomic() {
    let mut s = SourceFixture::new();
    s.seal().unwrap();
    s.before_batch();
    let db = s.f.db();
    let before = registry_image(&db);
    crate::repository::context_retirement::source::fail_capture();
    let result = s.capture();
    assert!(result.is_err());
    assert_eq!(before, registry_image(&db));
    s.capture().unwrap();
    let before = registry_image(&db);
    record::set_release_failure(true);
    assert!(
        s.f.service
            .finish_remote_operation(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOutcomeCategory::TransportUnavailable
            )
            .is_err()
    );
    assert_eq!(before, registry_image(&db));
    s.f.service
        .finish_remote_operation(
            s.f.root.path(),
            s.owner.as_ref().unwrap(),
            RemoteOutcomeCategory::TransportUnavailable,
        )
        .unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT reason FROM retirement_observation_pin_releases",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "source_terminal"
    );
    let before = registry_image(&db);
    assert!(s.capture().is_err());
    assert!(
        s.f.service
            .remote_safe_point(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOperationSafePoint::BeforeTransport
            )
            .is_err()
    );
    assert_eq!(before, registry_image(&db));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_source_last_pin_collection_and_replay_do_not_resolve_reused_batch_ids() {
    let mut s = SourceFixture::new();
    s.seal().unwrap();
    s.before_batch();
    s.capture().unwrap();
    let db = s.f.db();
    let old: i64 = db
        .query_row(
            "SELECT observation_batch_id FROM retirement_advertisement_provenance",
            [],
            |r| r.get(0),
        )
        .unwrap();
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        state::configure(tx, id, None, true)
    })
    .unwrap();
    assert!(
        RepositoryService::open_at(s.f.data.path()).is_ok(),
        "pinned unbound history survives G fence"
    );
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        let stopped = state::read_operation(tx, id, s.operation)?.unwrap();
        crate::repository::context_retirement::source::close_stopped(tx, &stopped)
    })
    .unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_observation_batches WHERE id=?1",
            [old],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(
        RepositoryService::open_at(s.f.data.path()).is_ok(),
        "last release collected obsolete batch"
    );
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        let config =
            crate::repository::remote::observation::ObservationConfiguration::capture_owned(
                &s.f.service,
                tx,
                s.f.root.path(),
                &s.plan,
            )?;
        state::configure_endpoints(tx, id, &s.plan, &config.endpoint_digest())?;
        let new = state::persist_complete_advertisement(
            tx,
            id,
            &s.plan,
            state::generation(tx, id)?,
            &[],
            44,
        )?;
        assert_eq!(new, old, "exercise SQLite integer-ID reuse");
        Ok(())
    })
    .unwrap();
    let before = registry_image(&db);
    state::with_transaction(&s.f.service, s.f.root.path(), |tx, id| {
        let stopped = state::read_operation(tx, id, s.operation)?.unwrap();
        crate::repository::context_retirement::source::close_stopped(tx, &stopped)
    })
    .unwrap();
    assert_eq!(before, registry_image(&db));
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    assert!(
        !db.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );
}

#[test]
fn retirement_source_unbound_cancel_before_capture_closes_without_debt_or_pins() {
    let mut s = SourceFixture::new();
    s.seal().unwrap();
    s.f.service
        .cancel_remote_operation(s.f.root.path(), s.operation)
        .unwrap();
    assert_eq!(
        s.f.service
            .remote_safe_point(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOperationSafePoint::BeforeTransport
            )
            .unwrap(),
        RemoteSafePointOutcome::Cancelled
    );
    let db = s.f.db();
    assert_eq!(
        db.query_row("SELECT phase FROM poll_retirement_attempts", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "ended"
    );
    for table in [
        "retirement_advertisement_provenance",
        "retirement_observation_pins",
        "context_retirement_records",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
}

#[test]
fn retirement_source_unbound_terminal_or_unexplained_pointer_is_not_a_planning_hold() {
    for case in ["terminal", "pointer"] {
        let s = SourceFixture::new();
        s.project_capture();
        let db = s.f.db();
        if case == "terminal" {
            db.execute("UPDATE remote_operation_records SET phase='failed',outcome='transport_unavailable' WHERE operation_ulid=?1",[s.operation.to_string()]).unwrap();
        } else {
            db.execute(
                "UPDATE poll_retirement_attempts SET active_provenance_id=NULL",
                [],
            )
            .unwrap();
        }
        let before = registry_image(&db);
        assert!(record::audit_connection(&db).is_err(), "{case}");
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_source_unsealed_public_owner_cannot_borrow_opted_in_projection() {
    let s = SourceFixture::new();
    let db = s.f.db();
    let before = registry_image(&db);
    assert!(
        s.f.service
            .remote_safe_point(
                s.f.root.path(),
                s.owner.as_ref().unwrap(),
                RemoteOperationSafePoint::BeforeTransport
            )
            .is_err()
    );
    assert_eq!(before, registry_image(&db));
}
