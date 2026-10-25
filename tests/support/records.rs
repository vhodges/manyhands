//! The world of the tests of the request stores themselves: one enabled
//! repository, and a service whose records a clock the test sets times.

use std::sync::Arc;

use manyhands::repository::{
    AddRemoteRequest, ConfirmationId, ExpectedPathObservation, OperationFamily, OperationId,
    RepositoryService, RequestId,
    request_store::{
        ConfirmationRecord, DigestSalt, FieldValue, IntentDigest, JournalRow, NewConfirmation,
        NewRequest, RequestOperation, RequestRecord, ScopeKey,
    },
};
use rusqlite::Connection;

use super::{
    EnabledRepository, TestRepository, items,
    mutation::{self, TestClock},
    operations::{self, OPERATION_A, OPERATION_B},
};

pub fn request_id(id: &str) -> RequestId {
    RequestId::parse(id).unwrap()
}

pub fn confirmation_id(id: &str) -> ConfirmationId {
    ConfirmationId::parse(id).unwrap()
}

/// 2023-11-14T22:13:20Z.
pub const T0: i64 = 1_700_000_000;

pub const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// Planted in what no store may keep.
pub const BODY_SENTINEL: &str = "SENTINEL-body-7c1d";

/// One enabled repository, and a service whose records `clock` times.
pub struct World {
    pub fixture: TestRepository,
    pub data: tempfile::TempDir,
    pub service: RepositoryService,
    pub clock: Arc<TestClock>,
    pub scope: ScopeKey,
}

pub fn world() -> World {
    let fixture = super::born_repository();
    let EnabledRepository {
        service,
        data_directory,
    } = super::enabled_repository(&fixture);
    drop(service);
    let clock = TestClock::at(T0);
    let service = mutation::service_with_clock(data_directory.path(), &clock);
    let scope = ScopeKey::for_repository(&fixture.root).unwrap();
    World {
        fixture,
        data: data_directory,
        service,
        clock,
        scope,
    }
}

impl World {
    pub fn index(&self) -> Connection {
        items::index(self.data.path())
    }

    pub fn count(&self, sql: &str) -> i64 {
        self.index().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    pub fn digest(&self, salt: DigestSalt) -> IntentDigest {
        IntentDigest::builder(salt, "document save", &self.scope, items::DOCUMENT_A)
            .field("title", FieldValue::Text("Title"))
            .field("body", FieldValue::Bytes(BODY_SENTINEL.as_bytes()))
            .finish()
    }

    pub fn request(&self, id: &str) -> NewRequest {
        let request_id = request_id(id);
        NewRequest {
            request_id,
            scope: self.scope.clone(),
            command: "document save".to_owned(),
            target: items::DOCUMENT_A.to_owned(),
            intent_digest: self.digest(DigestSalt::Request(request_id)),
            confirmation: None,
            base_ref: Some("refs/heads/main".to_owned()),
            base_oid: Some(git2::Oid::from_str(COMMIT).unwrap()),
            expected_digest: Some(ExpectedPathObservation::from_bytes(b"before")),
            operations: vec![RequestOperation {
                family: OperationFamily::Local,
                operation_id: operations::operation_id(OPERATION_A),
            }],
        }
    }

    pub fn confirmation(&self, id: &str, expires_at: i64) -> NewConfirmation {
        let confirmation_id = confirmation_id(id);
        NewConfirmation {
            confirmation_id,
            scope: self.scope.clone(),
            command: "document save".to_owned(),
            target: items::DOCUMENT_A.to_owned(),
            intent_digest: self.digest(DigestSalt::Confirmation(confirmation_id)),
            observation_digest: None,
            expires_at,
        }
    }

    pub fn record(&self, id: &str) -> Option<RequestRecord> {
        self.service.request_record(request_id(id)).unwrap()
    }

    pub fn confirmation_record(&self, id: &str) -> Option<ConfirmationRecord> {
        self.service
            .confirmation_record(confirmation_id(id))
            .unwrap()
    }

    /// A domain call that takes the index lock itself.
    pub fn add_remote(&self, name: &str) {
        self.service
            .add_remote(AddRemoteRequest {
                root: self.fixture.root.clone(),
                name: name.to_owned(),
                url: "https://example.invalid/origin.git".to_owned(),
                operation_id: OperationId::new(),
            })
            .unwrap();
    }
}

impl World {
    pub fn journal_row(&self, family: OperationFamily, id: &str) -> JournalRow {
        self.service
            .journal_row(family, &self.scope, operations::operation_id(id))
            .unwrap()
    }

    pub fn set_local(&self, id: &str, state: &str, step: Option<&str>) {
        self.index()
            .execute(
                "UPDATE operation_records SET state = ?2, completed_step = ?3
                  WHERE operation_ulid = ?1",
                rusqlite::params![id, state, step],
            )
            .unwrap();
    }
}

impl World {
    /// A request that began a local and a remote operation.
    pub fn insert_two_operation_request(&self, id: &str) {
        let mut request = self.request(id);
        request.operations.push(RequestOperation {
            family: OperationFamily::Remote,
            operation_id: operations::operation_id(OPERATION_B),
        });
        self.service.insert_request(&request).unwrap();
    }
}

/// A world whose index cannot be read.
pub fn degraded_world() -> World {
    let fixture = super::born_repository();
    let scope = ScopeKey::for_repository(&fixture.root).unwrap();
    let (data, service) = items::degraded_service(super::enabled_repository(&fixture));
    World {
        fixture,
        data,
        service,
        clock: TestClock::at(T0),
        scope,
    }
}
