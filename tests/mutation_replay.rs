//! Request identity and records: the request and confirmation IDs, the
//! intent digest, the three request tables, the journal lookup and
//! `show_request`.

use std::{path::Path, sync::Arc};

use manyhands::{
    repository::{
        AddRemoteRequest, ConfirmationId, ExpectedPathObservation, LeaseKind, OperationFamily,
        OperationId, RemoteOperationPhase, RemoveRegistrationOutcome, RemoveRegistrationRequest,
        RepositoryService, RequestDto, RequestId, RequestOperationDto, RequestResultDto,
        RequestState,
        keys::KeyMaterialPhase,
        request_store::{
            ConfirmationRecord, DigestSalt, FieldValue, FinalKind, InsertRequestOutcome,
            IntentDigest, JournalRow, NewConfirmation, NewRequest, PendingOperation,
            RequestOperation, RequestRecord, RequestResult, ScopeKey,
        },
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Outcome, RecoveryActionKind, ResultCode, Scope,
        WriteEffect,
    },
};
use rusqlite::{Connection, types::Value as SqlValue};
use serde_json::json;

mod support;

use support::{
    EnabledRepository, TestRepository,
    golden::{self, ContractCase},
    items,
    mutation::{self, TestClock},
    operations::{self, OPERATION_A, OPERATION_B, OPERATION_C},
    schema,
};

const REQUEST_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ0";
const REQUEST_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ1";
const REQUEST_ABSENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ9";
const CONFIRMATION_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN0";
const CONFIRMATION_ABSENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN9";

fn request_id(id: &str) -> RequestId {
    RequestId::parse(id).unwrap()
}

fn confirmation_id(id: &str) -> ConfirmationId {
    ConfirmationId::parse(id).unwrap()
}

#[test]
fn request_and_confirmation_ids_are_canonical_ulids() {
    assert_eq!(request_id(REQUEST_A).to_string(), REQUEST_A);
    assert_eq!(confirmation_id(CONFIRMATION_A).to_string(), CONFIRMATION_A);
    assert_ne!(RequestId::new(), RequestId::new());
    assert_ne!(ConfirmationId::new(), ConfirmationId::new());
    for malformed in ["", "01arz3ndektsv4rrffq69g5fq0", "not-a-ulid", " "] {
        assert!(RequestId::parse(malformed).is_err(), "{malformed:?}");
        assert!(ConfirmationId::parse(malformed).is_err(), "{malformed:?}");
    }
}

/// What a ticket save would feed the digest, each part replaceable.
#[derive(Clone)]
struct Intent<'a> {
    salt: DigestSalt,
    command: &'a str,
    repository: ScopeKey,
    target: &'a str,
    title: FieldValue<'a>,
    body: FieldValue<'a>,
    status: FieldValue<'a>,
    deps: FieldValue<'a>,
    parent: FieldValue<'a>,
    observation: FieldValue<'a>,
}

impl Intent<'_> {
    fn digest(&self) -> IntentDigest {
        IntentDigest::builder(self.salt, self.command, &self.repository, self.target)
            .field("title", self.title)
            .field("body", self.body)
            .field("status", self.status)
            .field("deps", self.deps)
            .field("parent", self.parent)
            .field("observation", self.observation)
            .finish()
    }
}

const DEPS: &[&str] = &["01ARZ3NDEKTSV4RRFFQ69G5FC1"];

fn intent() -> Intent<'static> {
    Intent {
        salt: DigestSalt::Request(request_id(REQUEST_A)),
        command: "ticket save",
        repository: ScopeKey::from_stored("/projects/example"),
        target: "01ARZ3NDEKTSV4RRFFQ69G5FC0",
        title: FieldValue::Text("Title"),
        body: FieldValue::Text("Body.\n"),
        status: FieldValue::Text("open"),
        deps: FieldValue::List(DEPS),
        parent: FieldValue::Null,
        observation: FieldValue::Text("token-1"),
    }
}

#[test]
fn the_digest_is_stable_for_equal_input() {
    assert_eq!(intent().digest(), intent().digest());
    let text = intent().digest().to_string();
    assert_eq!(text.len(), 64);
    assert!(
        text.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert_eq!(IntentDigest::from_stored(&text), Some(intent().digest()));
    assert_eq!(IntentDigest::from_stored("not a digest"), None);
}

#[test]
fn the_digest_differs_for_each_changed_part() {
    let base = intent();
    let changed: Vec<(&str, Intent<'_>)> = vec![
        (
            "title",
            Intent {
                title: FieldValue::Text("Other"),
                ..base.clone()
            },
        ),
        (
            "body",
            Intent {
                body: FieldValue::Text("Body!\n"),
                ..base.clone()
            },
        ),
        (
            "status",
            Intent {
                status: FieldValue::Text("closed"),
                ..base.clone()
            },
        ),
        (
            "deps",
            Intent {
                deps: FieldValue::List(&["01ARZ3NDEKTSV4RRFFQ69G5FC2"]),
                ..base.clone()
            },
        ),
        (
            "parent",
            Intent {
                parent: FieldValue::Text("01ARZ3NDEKTSV4RRFFQ69G5FC2"),
                ..base.clone()
            },
        ),
        (
            "observation token",
            Intent {
                observation: FieldValue::Text("token-2"),
                ..base.clone()
            },
        ),
        (
            "target",
            Intent {
                target: "01ARZ3NDEKTSV4RRFFQ69G5FC9",
                ..base.clone()
            },
        ),
        (
            "command",
            Intent {
                command: "document save",
                ..base.clone()
            },
        ),
        (
            "repository",
            Intent {
                repository: ScopeKey::from_stored("/projects/other"),
                ..base.clone()
            },
        ),
        (
            "repository against application",
            Intent {
                repository: ScopeKey::application(),
                ..base.clone()
            },
        ),
        (
            "salt",
            Intent {
                salt: DigestSalt::Request(request_id(REQUEST_B)),
                ..base.clone()
            },
        ),
        (
            "salt kind",
            Intent {
                salt: DigestSalt::Confirmation(confirmation_id(REQUEST_A)),
                ..base.clone()
            },
        ),
    ];

    let mut seen = vec![base.digest()];
    for (part, intent) in changed {
        let digest = intent.digest();
        assert!(!seen.contains(&digest), "a changed {part} kept a digest");
        seen.push(digest);
    }
}

#[test]
fn absent_null_and_empty_are_three_different_inputs() {
    let with = |parent| Intent { parent, ..intent() }.digest();
    let values = [
        with(FieldValue::Absent),
        with(FieldValue::Null),
        with(FieldValue::Text("")),
        with(FieldValue::Bytes(b"")),
        with(FieldValue::List(&[])),
        with(FieldValue::List(&[""])),
        with(FieldValue::Flag(false)),
        with(FieldValue::Flag(true)),
    ];
    for (index, value) in values.iter().enumerate() {
        for other in &values[index + 1..] {
            assert_ne!(value, other);
        }
    }
}

#[test]
fn field_boundaries_cannot_be_shifted() {
    let fields = |title, body| {
        Intent {
            title: FieldValue::Text(title),
            body: FieldValue::Text(body),
            ..intent()
        }
        .digest()
    };
    assert_ne!(fields("ab", "c"), fields("a", "bc"));

    let header = |command, target| {
        Intent {
            command,
            target,
            ..intent()
        }
        .digest()
    };
    assert_ne!(header("ab", "c"), header("a", "bc"));

    let list = |deps| {
        Intent {
            deps: FieldValue::List(deps),
            ..intent()
        }
        .digest()
    };
    assert_ne!(list(&["ab", "c"]), list(&["a", "bc"]));
    assert_ne!(list(&["abc"]), list(&["ab", "c"]));

    // A field's name is part of it: the same values under other names.
    let scope = ScopeKey::from_stored("/projects/example");
    let named = |first, second| {
        IntentDigest::builder(DigestSalt::Request(request_id(REQUEST_A)), "c", &scope, "t")
            .field(first, FieldValue::Text("x"))
            .field(second, FieldValue::Text("x"))
            .finish()
    };
    assert_ne!(named("ab", "c"), named("a", "bc"));
}

fn create_digest(root: &Path) -> IntentDigest {
    let scope = ScopeKey::for_new_repository(root).unwrap();
    IntentDigest::builder(
        DigestSalt::Request(request_id(REQUEST_A)),
        "repo create",
        &scope,
        scope.as_str(),
    )
    .field("primary_branch", FieldValue::Text("main"))
    .finish()
}

#[test]
fn the_digest_for_repo_create_is_the_same_before_and_after_the_root_exists() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("new-repository");

    let before = ScopeKey::for_new_repository(&root).unwrap();
    let digest_before = create_digest(&root);
    std::fs::create_dir(&root).unwrap();

    assert_eq!(ScopeKey::for_new_repository(&root).unwrap(), before);
    assert_eq!(ScopeKey::for_repository(&root).unwrap(), before);
    assert_eq!(create_digest(&root), digest_before);
    let scope = ScopeKey::for_repository(&root).unwrap();
    assert_eq!(
        IntentDigest::builder(
            DigestSalt::Request(request_id(REQUEST_A)),
            "repo create",
            &scope,
            scope.as_str(),
        )
        .field("primary_branch", FieldValue::Text("main"))
        .finish(),
        digest_before
    );
    // The parent is resolved, so another spelling of it is the same key.
    assert_eq!(
        ScopeKey::for_new_repository(&parent.path().join(".").join("new-repository")).unwrap(),
        before
    );
    assert_eq!(ScopeKey::application().as_str(), "application");
    assert!(ScopeKey::for_new_repository(&parent.path().join("absent").join("new")).is_none());
    assert!(ScopeKey::for_repository(&parent.path().join("absent")).is_none());
}

/// 2023-11-14T22:13:20Z.
const T0: i64 = 1_700_000_000;
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
/// Planted in what no store may keep.
const BODY_SENTINEL: &str = "SENTINEL-body-7c1d";

/// One enabled repository, and a service whose records `clock` times.
struct World {
    fixture: TestRepository,
    data: tempfile::TempDir,
    service: RepositoryService,
    clock: Arc<TestClock>,
    scope: ScopeKey,
}

fn world() -> World {
    let fixture = support::born_repository();
    let EnabledRepository {
        service,
        data_directory,
    } = support::enabled_repository(&fixture);
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
    fn index(&self) -> Connection {
        items::index(self.data.path())
    }

    fn count(&self, sql: &str) -> i64 {
        self.index().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn digest(&self, salt: DigestSalt) -> IntentDigest {
        IntentDigest::builder(salt, "document save", &self.scope, items::DOCUMENT_A)
            .field("title", FieldValue::Text("Title"))
            .field("body", FieldValue::Bytes(BODY_SENTINEL.as_bytes()))
            .finish()
    }

    fn request(&self, id: &str) -> NewRequest {
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

    fn confirmation(&self, id: &str, expires_at: i64) -> NewConfirmation {
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

    fn record(&self, id: &str) -> Option<RequestRecord> {
        self.service.request_record(request_id(id)).unwrap()
    }

    fn confirmation_record(&self, id: &str) -> Option<ConfirmationRecord> {
        self.service
            .confirmation_record(confirmation_id(id))
            .unwrap()
    }

    /// A domain call that takes the index lock itself.
    fn add_remote(&self, name: &str) {
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

fn committed() -> RequestResult {
    RequestResult {
        outcome: Outcome::Success,
        code: ResultCode::Ok,
        effects: Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Committed,
            discovery: DiscoveryEffect::Current,
            commit_oid: Some(COMMIT.to_owned()),
            ..Effects::not_requested()
        },
        data: Some(json!({"id": items::DOCUMENT_A, "path": "docs/a.md"})),
    }
}

fn already_applied() -> RequestResult {
    RequestResult {
        outcome: Outcome::Noop,
        code: ResultCode::AlreadyApplied,
        effects: Effects::not_requested(),
        data: None,
    }
}

const REQUEST_TABLES: [&str; 3] = [
    "confirmation_records",
    "request_operations",
    "request_records",
];

/// Every object of the schema and every row of every table, as text.
fn dump(connection: &Connection) -> Vec<(String, String, Vec<Vec<SqlValue>>)> {
    let objects: Vec<(String, String, String)> = connection
        .prepare(
            "SELECT type, name, sql FROM sqlite_master
              WHERE sql IS NOT NULL ORDER BY type, name",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    objects
        .into_iter()
        .map(|(kind, name, sql)| {
            let rows = if kind == "table" {
                let mut statement = connection
                    .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
                    .unwrap();
                let columns = statement.column_count();
                statement
                    .query_map([], |row| {
                        (0..columns).map(|column| row.get(column)).collect()
                    })
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap()
            } else {
                Vec::new()
            };
            (name, sql, rows)
        })
        .collect()
}

fn columns(connection: &Connection, table: &str) -> Vec<String> {
    connection
        .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
        .unwrap()
        .query_map([table], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else() {
    let world = world();
    operations::configure_remote(world.data.path());
    operations::insert_local(
        world.data.path(),
        Some(OPERATION_A),
        "save_document",
        "created",
        None,
    );
    operations::insert_remote_poll(world.data.path(), OPERATION_B, "completed", None, None);
    operations::insert_key_material(world.data.path(), OPERATION_C, "generate", "reserved", None);
    let World { data, service, .. } = world;
    drop(service);
    // What a build from before the request tables left behind.
    let connection = items::index(data.path());
    for table in [
        "request_operations",
        "request_records",
        "confirmation_records",
    ] {
        connection
            .execute_batch(&format!("DROP TABLE {table}"))
            .unwrap();
    }
    let before = dump(&connection);
    assert!(
        before
            .iter()
            .any(|(name, _, rows)| name == "repositories" && rows.len() == 1)
    );
    assert!(!before.iter().any(|(name, _, _)| name.contains("request")));
    drop(connection);

    RepositoryService::open_at(data.path()).unwrap();

    let connection = items::index(data.path());
    let after = dump(&connection);
    let added: Vec<&str> = after
        .iter()
        .filter(|object| !before.contains(object))
        .map(|(name, _, _)| name.as_str())
        .collect();
    assert_eq!(added, REQUEST_TABLES);
    let kept: Vec<_> = after
        .iter()
        .filter(|(name, _, _)| !REQUEST_TABLES.contains(&name.as_str()))
        .cloned()
        .collect();
    assert_eq!(kept, before);

    assert_eq!(
        columns(&connection, "request_records"),
        [
            "request_ulid",
            "attempt",
            "scope_key",
            "command",
            "target",
            "intent_digest",
            "confirmation_ulid",
            "base_ref",
            "base_oid",
            "expected_digest",
            "cancel_requested",
            "state",
            "outcome",
            "code",
            "effect_write",
            "effect_checkpoint",
            "effect_discovery",
            "effect_publication",
            "effect_integration",
            "effect_cleanup",
            "commit_oid",
            "result_data",
            "accepted_at",
            "finished_at",
        ]
    );
    assert_eq!(
        columns(&connection, "request_operations"),
        ["request_ulid", "ordinal", "family", "operation_ulid"]
    );
    assert_eq!(
        columns(&connection, "confirmation_records"),
        [
            "confirmation_ulid",
            "scope_key",
            "command",
            "target",
            "intent_digest",
            "observation_digest",
            "created_at",
            "expires_at",
            "accepted_by_request",
            "accepted_at",
        ]
    );
    // The journal of local operations gained nothing.
    support::assert_operation_records_hold_no_content(data.path());
}

#[test]
fn opening_a_database_that_has_the_request_tables_keeps_their_rows() {
    let world = world();
    assert_eq!(
        world
            .service
            .insert_request(&world.request(REQUEST_A))
            .unwrap(),
        InsertRequestOutcome::Inserted
    );
    assert!(
        world
            .service
            .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
            .unwrap()
    );
    let before = dump(&world.index());

    RepositoryService::open_at(world.data.path()).unwrap();

    assert_eq!(dump(&world.index()), before);
}

#[test]
fn an_inserted_request_is_read_back_as_it_was_accepted() {
    let world = world();
    let mut request = world.request(REQUEST_A);
    request.operations.push(RequestOperation {
        family: OperationFamily::Remote,
        operation_id: operations::operation_id(OPERATION_B),
    });
    request.operations.push(RequestOperation {
        family: OperationFamily::KeyMaterial,
        operation_id: operations::operation_id(OPERATION_C),
    });

    assert_eq!(
        world.service.insert_request(&request).unwrap(),
        InsertRequestOutcome::Inserted
    );

    assert_eq!(
        world.record(REQUEST_A),
        Some(RequestRecord {
            request_id: request.request_id,
            attempt: 1,
            scope: world.scope.clone(),
            command: "document save".to_owned(),
            target: items::DOCUMENT_A.to_owned(),
            intent_digest: request.intent_digest,
            confirmation: None,
            base_ref: Some("refs/heads/main".to_owned()),
            base_oid: Some(git2::Oid::from_str(COMMIT).unwrap()),
            expected_digest: Some(ExpectedPathObservation::from_bytes(b"before")),
            cancel_requested: false,
            state: RequestState::Accepted,
            result: None,
            accepted_at: T0,
            finished_at: None,
            operations: request.operations.clone(),
        })
    );
    assert_eq!(world.record(REQUEST_ABSENT), None);

    // What a request with no branch, no expectation and no operation holds,
    // and one that expects its path to be missing.
    let bare = NewRequest {
        scope: ScopeKey::application(),
        base_ref: None,
        base_oid: None,
        expected_digest: None,
        operations: Vec::new(),
        ..world.request(REQUEST_B)
    };
    assert_eq!(
        world.service.insert_request(&bare).unwrap(),
        InsertRequestOutcome::Inserted
    );
    let stored = world.record(REQUEST_B).unwrap();
    assert_eq!(stored.scope, ScopeKey::application());
    assert_eq!(
        (stored.base_ref, stored.base_oid, stored.expected_digest),
        (None, None, None)
    );
    assert!(stored.operations.is_empty());
    let missing = NewRequest {
        expected_digest: Some(ExpectedPathObservation::Missing),
        ..world.request(REQUEST_ABSENT)
    };
    world.service.insert_request(&missing).unwrap();
    assert_eq!(
        world.record(REQUEST_ABSENT).unwrap().expected_digest,
        Some(ExpectedPathObservation::Missing)
    );
}

#[test]
fn insert_is_refused_for_an_existing_request_id() {
    let world = world();
    let first = world.request(REQUEST_A);
    assert_eq!(
        world.service.insert_request(&first).unwrap(),
        InsertRequestOutcome::Inserted
    );
    let stored = world.record(REQUEST_A);

    let second = NewRequest {
        command: "ticket save".to_owned(),
        target: items::TICKET_A.to_owned(),
        operations: vec![RequestOperation {
            family: OperationFamily::Local,
            operation_id: operations::operation_id(OPERATION_B),
        }],
        ..world.request(REQUEST_A)
    };
    world.clock.set(T0 + 5);
    assert_eq!(
        world.service.insert_request(&second).unwrap(),
        InsertRequestOutcome::RequestExists
    );

    assert_eq!(world.record(REQUEST_A), stored);
    assert_eq!(world.count("SELECT count(*) FROM request_records"), 1);
    assert_eq!(world.count("SELECT count(*) FROM request_operations"), 1);
}

#[test]
fn finish_succeeds_once_on_an_accepted_record_and_changes_nothing_on_a_finished_one() {
    let world = world();
    let id = request_id(REQUEST_A);
    world
        .service
        .insert_request(&world.request(REQUEST_A))
        .unwrap();

    world.clock.set(T0 + 7);
    assert!(world.service.finish_request(id, 1, &committed()).unwrap());

    let finished = world.record(REQUEST_A).unwrap();
    assert_eq!(finished.state, RequestState::Finished);
    assert_eq!(finished.result, Some(committed()));
    assert_eq!(
        (finished.accepted_at, finished.finished_at),
        (T0, Some(T0 + 7))
    );
    assert_eq!(finished.attempt, 1);
    assert_eq!(finished.operations, world.request(REQUEST_A).operations);

    // Nothing changes a finished record: not another finish, a delete or
    // an entry.
    world.clock.set(T0 + 9);
    assert!(
        !world
            .service
            .finish_request(id, 1, &already_applied())
            .unwrap()
    );
    assert!(!world.service.delete_request(id, 1).unwrap());
    assert_eq!(world.service.enter_request(id).unwrap(), None);
    assert_eq!(world.record(REQUEST_A), Some(finished));

    // A result with no data and no commit is stored as such.
    world
        .service
        .insert_request(&world.request(REQUEST_B))
        .unwrap();
    assert!(
        world
            .service
            .finish_request(request_id(REQUEST_B), 1, &already_applied())
            .unwrap()
    );
    assert_eq!(
        world.record(REQUEST_B).unwrap().result,
        Some(already_applied())
    );

    // A request no record holds is not finished, deleted or entered.
    let absent = request_id(REQUEST_ABSENT);
    assert!(
        !world
            .service
            .finish_request(absent, 1, &committed())
            .unwrap()
    );
    assert!(!world.service.delete_request(absent, 1).unwrap());
    assert_eq!(world.service.enter_request(absent).unwrap(), None);
}

#[test]
fn a_call_whose_attempt_is_no_longer_current_settles_nothing() {
    let world = world();
    for id in [REQUEST_A, REQUEST_B] {
        let id = request_id(id);
        world
            .service
            .insert_request(&world.request(&id.to_string()))
            .unwrap();
        // Attempt 1 was entered by the insert; a second call enters as 2.
        assert_eq!(world.service.enter_request(id).unwrap(), Some(2));
        let entered = world.record(&id.to_string()).unwrap();
        assert_eq!(
            (entered.attempt, entered.state),
            (2, RequestState::Accepted)
        );

        assert!(!world.service.finish_request(id, 1, &committed()).unwrap());
        assert!(!world.service.delete_request(id, 1).unwrap());
        // Nor does an attempt that was never entered.
        assert!(!world.service.finish_request(id, 3, &committed()).unwrap());
        assert!(!world.service.delete_request(id, 3).unwrap());
        assert_eq!(world.record(&id.to_string()), Some(entered));
    }

    assert!(
        world
            .service
            .finish_request(request_id(REQUEST_A), 2, &committed())
            .unwrap()
    );
    assert_eq!(
        world.record(REQUEST_A).unwrap().state,
        RequestState::Finished
    );

    assert!(
        world
            .service
            .delete_request(request_id(REQUEST_B), 2)
            .unwrap()
    );
    assert_eq!(world.record(REQUEST_B), None);
    assert_eq!(
        world.count(&format!(
            "SELECT count(*) FROM request_operations WHERE request_ulid = '{REQUEST_B}'"
        )),
        0
    );
    // The ID is free again.
    assert_eq!(
        world
            .service
            .insert_request(&world.request(REQUEST_B))
            .unwrap(),
        InsertRequestOutcome::Inserted
    );
    assert_eq!(world.record(REQUEST_B).unwrap().attempt, 1);
}

#[test]
fn deleting_a_record_releases_the_confirmation_it_accepted() {
    let world = world();
    let confirmation = confirmation_id(CONFIRMATION_A);
    assert!(
        world
            .service
            .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
            .unwrap()
    );
    // A confirmation ID is recorded once.
    assert!(
        !world
            .service
            .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 900))
            .unwrap()
    );
    let created = world.confirmation_record(CONFIRMATION_A).unwrap();
    assert_eq!(
        (
            created.created_at,
            created.expires_at,
            created.accepted_by,
            created.accepted_at
        ),
        (T0, T0 + 600, None, None)
    );
    assert_eq!(created.scope, world.scope);
    assert_eq!(
        created.intent_digest,
        world.digest(DigestSalt::Confirmation(confirmation))
    );
    assert_eq!(world.confirmation_record(CONFIRMATION_ABSENT), None);

    world.clock.set(T0 + 30);
    let accepting = NewRequest {
        confirmation: Some(confirmation),
        ..world.request(REQUEST_A)
    };
    assert_eq!(
        world.service.insert_request(&accepting).unwrap(),
        InsertRequestOutcome::Inserted
    );
    assert_eq!(
        world.record(REQUEST_A).unwrap().confirmation,
        Some(confirmation)
    );
    let accepted = world.confirmation_record(CONFIRMATION_A).unwrap();
    assert_eq!(
        (accepted.accepted_by, accepted.accepted_at),
        (Some(request_id(REQUEST_A)), Some(T0 + 30))
    );

    // A confirmation is accepted once, and only one that exists.
    let second = NewRequest {
        confirmation: Some(confirmation),
        ..world.request(REQUEST_B)
    };
    assert_eq!(
        world.service.insert_request(&second).unwrap(),
        InsertRequestOutcome::ConfirmationUnavailable
    );
    let unknown = NewRequest {
        confirmation: Some(confirmation_id(CONFIRMATION_ABSENT)),
        ..world.request(REQUEST_B)
    };
    assert_eq!(
        world.service.insert_request(&unknown).unwrap(),
        InsertRequestOutcome::ConfirmationUnavailable
    );
    assert_eq!(world.record(REQUEST_B), None);
    assert_eq!(world.count("SELECT count(*) FROM request_operations"), 1);

    // A delete that fails part of the way through releases nothing: the
    // release and the deletion are one transaction.
    world
        .index()
        .execute_batch(
            "CREATE TRIGGER refuse_request_deletion BEFORE DELETE ON request_records
             BEGIN SELECT RAISE(ABORT, 'refused by the test'); END",
        )
        .unwrap();
    world.clock.set(T0 + 60);
    let refused = world
        .service
        .delete_request(request_id(REQUEST_A), 1)
        .unwrap_err();
    assert_eq!(refused.code(), ResultCode::InternalError);
    assert_eq!(world.confirmation_record(CONFIRMATION_A), Some(accepted));
    assert!(world.record(REQUEST_A).is_some());
    assert_eq!(world.count("SELECT count(*) FROM request_operations"), 1);
    world
        .index()
        .execute_batch("DROP TRIGGER refuse_request_deletion")
        .unwrap();

    assert!(
        world
            .service
            .delete_request(request_id(REQUEST_A), 1)
            .unwrap()
    );

    assert_eq!(world.record(REQUEST_A), None);
    // Released, with its expiry still running from its creation.
    assert_eq!(world.confirmation_record(CONFIRMATION_A), Some(created));
    assert_eq!(
        world.service.insert_request(&second).unwrap(),
        InsertRequestOutcome::Inserted
    );
    assert_eq!(
        world
            .confirmation_record(CONFIRMATION_A)
            .unwrap()
            .accepted_by,
        Some(request_id(REQUEST_B))
    );
}

#[test]
fn finishing_a_record_keeps_the_confirmation_it_accepted() {
    let world = world();
    world
        .service
        .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
        .unwrap();
    let accepting = NewRequest {
        confirmation: Some(confirmation_id(CONFIRMATION_A)),
        ..world.request(REQUEST_A)
    };
    world.service.insert_request(&accepting).unwrap();

    assert!(
        world
            .service
            .finish_request(request_id(REQUEST_A), 1, &committed())
            .unwrap()
    );

    assert_eq!(
        world
            .confirmation_record(CONFIRMATION_A)
            .unwrap()
            .accepted_by,
        Some(request_id(REQUEST_A))
    );
}

#[test]
fn removing_a_registration_leaves_request_and_confirmation_rows() {
    let world = world();
    world
        .service
        .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
        .unwrap();
    let accepting = NewRequest {
        confirmation: Some(confirmation_id(CONFIRMATION_A)),
        ..world.request(REQUEST_A)
    };
    world.service.insert_request(&accepting).unwrap();
    world
        .service
        .insert_request(&world.request(REQUEST_B))
        .unwrap();
    world
        .service
        .finish_request(request_id(REQUEST_B), 1, &committed())
        .unwrap();
    let records = [world.record(REQUEST_A), world.record(REQUEST_B)];
    let confirmation = world.confirmation_record(CONFIRMATION_A);
    assert!(world.count("SELECT count(*) FROM operation_records") > 0);
    assert_eq!(world.count("SELECT count(*) FROM repositories"), 1);

    assert_eq!(
        world
            .service
            .remove_registration(RemoveRegistrationRequest {
                root: world.fixture.root.clone(),
                operation_id: OperationId::new(),
            })
            .unwrap(),
        RemoveRegistrationOutcome::Removed
    );

    // The registration and its operation records went, as before.
    assert_eq!(world.count("SELECT count(*) FROM repositories"), 0);
    assert_eq!(world.count("SELECT count(*) FROM operation_records"), 0);
    assert_eq!([world.record(REQUEST_A), world.record(REQUEST_B)], records);
    assert_eq!(world.confirmation_record(CONFIRMATION_A), confirmation);
    assert_eq!(world.count("SELECT count(*) FROM request_operations"), 2);
}

/// A world whose index cannot be read.
fn degraded_world() -> World {
    let fixture = support::born_repository();
    let scope = ScopeKey::for_repository(&fixture.root).unwrap();
    let (data, service) = items::degraded_service(support::enabled_repository(&fixture));
    World {
        fixture,
        data,
        service,
        clock: TestClock::at(T0),
        scope,
    }
}

#[test]
fn record_access_reports_an_unavailable_database() {
    let world = degraded_world();
    let id = request_id(REQUEST_A);

    let codes = [
        world
            .service
            .insert_request(&world.request(REQUEST_A))
            .unwrap_err()
            .code(),
        world.service.request_record(id).unwrap_err().code(),
        world.service.enter_request(id).unwrap_err().code(),
        world
            .service
            .finish_request(id, 1, &committed())
            .unwrap_err()
            .code(),
        world.service.delete_request(id, 1).unwrap_err().code(),
        world
            .service
            .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
            .unwrap_err()
            .code(),
        world
            .service
            .confirmation_record(confirmation_id(CONFIRMATION_A))
            .unwrap_err()
            .code(),
    ];

    assert_eq!(codes, [ResultCode::IndexUnavailable; 7]);
}

#[test]
fn record_access_reports_a_database_another_process_holds() {
    let world = world();
    let id = request_id(REQUEST_A);
    world
        .service
        .insert_request(&world.request(REQUEST_A))
        .unwrap();
    let held = RepositoryService::hold_lease_for_testing(
        &world.fixture.root,
        world.data.path(),
        LeaseKind::CacheWrite,
    )
    .unwrap();

    let codes = [
        world
            .service
            .insert_request(&world.request(REQUEST_B))
            .unwrap_err()
            .code(),
        world.service.request_record(id).unwrap_err().code(),
        world.service.enter_request(id).unwrap_err().code(),
        world
            .service
            .finish_request(id, 1, &committed())
            .unwrap_err()
            .code(),
        world.service.delete_request(id, 1).unwrap_err().code(),
    ];

    assert_eq!(codes, [ResultCode::Busy; 5]);
    drop(held);
    // Nothing was changed by a call that could not take the lock.
    let record = world.record(REQUEST_A).unwrap();
    assert_eq!((record.attempt, record.state), (1, RequestState::Accepted));
    assert_eq!(world.record(REQUEST_B), None);
}

#[test]
fn the_index_lock_is_released_when_a_record_function_returns() {
    let world = world();
    let id = request_id(REQUEST_A);
    // Taking the exclusive lock at once shows that nothing still holds it;
    // the domain call after it takes the lock itself.
    let released = |step: &str| {
        drop(
            RepositoryService::hold_lease_for_testing(
                &world.fixture.root,
                world.data.path(),
                LeaseKind::CacheWrite,
            )
            .unwrap_or_else(|_| panic!("the index lock is still held after {step}")),
        );
        world.add_remote(step);
    };

    world
        .service
        .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
        .unwrap();
    released("insert_confirmation");
    world.confirmation_record(CONFIRMATION_A).unwrap();
    released("confirmation_record");
    world
        .service
        .insert_request(&world.request(REQUEST_A))
        .unwrap();
    released("insert_request");
    assert_eq!(
        world
            .service
            .insert_request(&world.request(REQUEST_A))
            .unwrap(),
        InsertRequestOutcome::RequestExists
    );
    released("refused_insert");
    world.record(REQUEST_A).unwrap();
    released("request_record");
    world.service.enter_request(id).unwrap();
    released("enter_request");
    assert!(!world.service.finish_request(id, 1, &committed()).unwrap());
    released("stale_finish");
    assert!(world.service.finish_request(id, 2, &committed()).unwrap());
    released("finish_request");
    world
        .service
        .insert_request(&world.request(REQUEST_B))
        .unwrap();
    assert!(
        world
            .service
            .delete_request(request_id(REQUEST_B), 1)
            .unwrap()
    );
    released("delete_request");
}

#[test]
fn the_request_stores_hold_none_of_a_request_s_content() {
    let world = world();
    world
        .service
        .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
        .unwrap();
    let accepting = NewRequest {
        confirmation: Some(confirmation_id(CONFIRMATION_A)),
        ..world.request(REQUEST_A)
    };
    world.service.insert_request(&accepting).unwrap();
    world
        .service
        .finish_request(request_id(REQUEST_A), 1, &committed())
        .unwrap();

    // The body was fed to both digests and is stored nowhere.
    mutation::assert_request_stores_exclude(world.data.path(), &[BODY_SENTINEL]);
    support::assert_operation_records_hold_no_content(world.data.path());
}

#[test]
fn the_privacy_scan_finds_a_sentinel_in_every_request_store() {
    const PLANTED: &str = "SENTINEL-planted-\"3b9e\\";
    let world = world();
    let data = world.data.path();
    world
        .service
        .insert_confirmation(&world.confirmation(CONFIRMATION_A, T0 + 600))
        .unwrap();
    world
        .service
        .insert_request(&world.request(REQUEST_A))
        .unwrap();
    world
        .service
        .finish_request(request_id(REQUEST_A), 1, &committed())
        .unwrap();
    operations::configure_remote(data);
    operations::insert_remote_poll(data, OPERATION_B, "completed", None, None);
    operations::insert_key_material(data, OPERATION_C, "delete", "prepared", None);
    assert_eq!(mutation::find_stored_sentinel(data, &[PLANTED]), None);

    // Each write below is one a record function never makes: the sentinel
    // goes straight into one column of one store, and is taken out again.
    let found = |plant: &str, restore: &str| {
        let connection = world.index();
        connection.execute(plant, [PLANTED]).unwrap();
        let found = mutation::find_stored_sentinel(data, &["absent", PLANTED]);
        assert!(
            std::panic::catch_unwind(|| mutation::assert_request_stores_exclude(data, &[PLANTED]))
                .is_err(),
            "{plant}"
        );
        connection.execute_batch(restore).unwrap();
        assert_eq!(mutation::find_stored_sentinel(data, &[PLANTED]), None);
        let found = found.unwrap_or_else(|| panic!("not found after: {plant}"));
        assert_eq!(found.sentinel, PLANTED);
        (found.table, found.column)
    };

    assert_eq!(
        found(
            "UPDATE request_records SET result_data = json_object('body', ?1)",
            "UPDATE request_records SET result_data = NULL",
        ),
        ("request_records", "result_data".to_owned())
    );
    assert_eq!(
        found(
            "UPDATE request_records SET target = 'before ' || ?1 || ' after'",
            "UPDATE request_records SET target = 'target'",
        ),
        ("request_records", "target".to_owned())
    );
    assert_eq!(
        found(
            "UPDATE request_operations SET operation_ulid = ?1",
            &format!("UPDATE request_operations SET operation_ulid = '{OPERATION_A}'"),
        ),
        ("request_operations", "operation_ulid".to_owned())
    );
    assert_eq!(
        found(
            "UPDATE confirmation_records SET command = ?1",
            "UPDATE confirmation_records SET command = 'document save'",
        ),
        ("confirmation_records", "command".to_owned())
    );
    assert_eq!(
        found(
            "UPDATE operation_records SET redacted_error = ?1",
            "UPDATE operation_records SET redacted_error = NULL",
        ),
        ("operation_records", "redacted_error".to_owned())
    );
    assert_eq!(
        found(
            "INSERT INTO remote_operation_records (
                repository_id, operation_ulid, configuration_generation, remote_name,
                primary_branch, primary_ref, primary_tracking_ref, action, priority, phase,
                created_at, updated_at
             ) SELECT id, '01ARZ3NDEKTSV4RRFFQ69G5FA7', 0, ?1, 'main', 'refs/heads/main',
                      'refs/remotes/origin/main', 'poll', 'poll', 'completed', 1, 1
                 FROM repositories",
            "DELETE FROM remote_operation_records
              WHERE operation_ulid = '01ARZ3NDEKTSV4RRFFQ69G5FA7'",
        ),
        ("remote_operation_records", "remote_name".to_owned())
    );
    // A column of bytes is read as well as one of text.
    assert_eq!(
        found(
            "UPDATE key_material_operations SET private_file_identity = CAST(?1 AS BLOB)",
            "UPDATE key_material_operations SET private_file_identity = NULL",
        ),
        (
            "key_material_operations",
            "private_file_identity".to_owned()
        )
    );
    // The fixture's own key paths hold a sentinel of theirs.
    assert_eq!(
        mutation::find_stored_sentinel(data, &[operations::KEY_MATERIAL_SENTINEL])
            .map(|found| found.table),
        Some("key_material_operations")
    );
}

fn pending_local(state: &str, step: Option<&str>) -> JournalRow {
    JournalRow::Pending(PendingOperation::Local {
        state: state.to_owned(),
        step: step.map(str::to_owned),
    })
}

fn pending_remote(phase: RemoteOperationPhase) -> JournalRow {
    JournalRow::Pending(PendingOperation::Remote { phase })
}

fn pending_key(phase: KeyMaterialPhase) -> JournalRow {
    JournalRow::Pending(PendingOperation::KeyMaterial { phase })
}

/// A row that has ended and owes nothing.
fn ended(kind: FinalKind) -> JournalRow {
    JournalRow::Final {
        kind,
        owes_work: false,
    }
}

/// A row that has ended and still owes its index hand-off or a
/// reconciliation.
fn owing(kind: FinalKind) -> JournalRow {
    JournalRow::Final {
        kind,
        owes_work: true,
    }
}

impl World {
    fn journal_row(&self, family: OperationFamily, id: &str) -> JournalRow {
        self.service
            .journal_row(family, &self.scope, operations::operation_id(id))
            .unwrap()
    }

    fn set_local(&self, id: &str, state: &str, step: Option<&str>) {
        self.index()
            .execute(
                "UPDATE operation_records SET state = ?2, completed_step = ?3
                  WHERE operation_ulid = ?1",
                rusqlite::params![id, state, step],
            )
            .unwrap();
    }
}

#[test]
fn settlement_asks_one_question_of_a_journal_row() {
    assert!(!JournalRow::Absent.in_flight());
    assert!(pending_local("created", None).in_flight());
    assert!(pending_remote(RemoteOperationPhase::Interrupted).in_flight());
    assert!(pending_key(KeyMaterialPhase::Reserved).in_flight());
    for kind in [
        FinalKind::Completed,
        FinalKind::Cancelled,
        FinalKind::RetainedForInspection,
    ] {
        assert!(!ended(kind).in_flight(), "{kind:?}");
        assert!(owing(kind).in_flight(), "{kind:?}");
        // What a row came to is told apart from whether it owes work.
        assert_ne!(owing(kind), ended(kind));
        assert!(matches!(owing(kind), JournalRow::Final { kind: found, .. } if found == kind));
    }
}

#[test]
fn the_journal_lookup_reads_a_local_operation() {
    let world = world();
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        JournalRow::Absent
    );

    operations::insert_local(
        world.data.path(),
        Some(OPERATION_A),
        "save_document",
        "created",
        None,
    );
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        pending_local("created", None)
    );

    world.set_local(
        OPERATION_A,
        "authoring",
        Some("authoring_destination_observed"),
    );
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        pending_local("authoring", Some("authoring_destination_observed"))
    );

    world.set_local(OPERATION_A, "completed", Some("authoritative_observed"));
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        ended(FinalKind::Completed)
    );
    world.set_local(OPERATION_A, "completed", None);
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        ended(FinalKind::Completed)
    );

    // Each journal answers for its own rows only.
    assert_eq!(
        world.journal_row(OperationFamily::Remote, OPERATION_A),
        JournalRow::Absent
    );
    assert_eq!(
        world.journal_row(OperationFamily::KeyMaterial, OPERATION_A),
        JournalRow::Absent
    );
    assert_eq!(
        world.journal_row(OperationFamily::Local, operations::OPERATION_ABSENT),
        JournalRow::Absent
    );
    // And a root answers for its own operations only.
    assert_eq!(
        world
            .service
            .journal_row(
                OperationFamily::Local,
                &ScopeKey::from_stored("/projects/other"),
                operations::operation_id(OPERATION_A),
            )
            .unwrap(),
        JournalRow::Absent
    );
}

#[test]
fn the_journal_lookup_reads_a_local_row_closed_as_rejected_as_absent() {
    let world = world();
    let rejected = operations::operation_id(OPERATION_A);

    // A real rejection: the name is refused before anything is written.
    assert!(
        world
            .service
            .add_remote(AddRemoteRequest {
                root: world.fixture.root.clone(),
                name: "not a name".to_owned(),
                url: "https://example.invalid/origin.git".to_owned(),
                operation_id: rejected,
            })
            .is_err()
    );

    // The row is kept, closed.
    let stored: (String, Option<String>) = world
        .index()
        .query_row(
            "SELECT state, completed_step FROM operation_records WHERE operation_ulid = ?1",
            [OPERATION_A],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        stored,
        ("completed".to_owned(), Some("rejected".to_owned()))
    );
    let row = world.journal_row(OperationFamily::Local, OPERATION_A);
    assert_eq!(row, JournalRow::Absent);
    assert!(!row.in_flight());

    // Only `completed` with that step is a closed row.
    world.set_local(OPERATION_A, "created", Some("rejected"));
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        pending_local("created", Some("rejected"))
    );
}

#[test]
fn the_journal_lookup_reads_a_synchronization_bound_locally() {
    let world = world();
    // What a synchronization with no remote to publish to leaves: a
    // refresh in the local journal, under the synchronization's ID.
    world
        .index()
        .execute(
            "INSERT INTO operation_records (
                repository_id, root_path, operation_ulid, action, target, state, observed_at
             ) SELECT id, root_path, ?1, 'refresh', ?2, 'indexing', 1 FROM repositories",
            [
                OPERATION_A,
                &format!("synchronization-local-v1/primary/{COMMIT}"),
            ],
        )
        .unwrap();

    assert_eq!(
        world.journal_row(OperationFamily::Remote, OPERATION_A),
        JournalRow::Absent
    );
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        pending_local("indexing", None)
    );

    world.set_local(OPERATION_A, "completed", None);
    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        ended(FinalKind::Completed)
    );
}

/// What the lookup says of a remote operation in a phase, when the row
/// owes nothing: pending in that phase, or ended in some way.
fn remote_standing(phase: RemoteOperationPhase) -> (&'static str, Result<(), FinalKind>) {
    match phase {
        RemoteOperationPhase::Reserved => ("reserved", Ok(())),
        RemoteOperationPhase::Advertising => ("advertising", Ok(())),
        RemoteOperationPhase::Persisting => ("persisting", Ok(())),
        RemoteOperationPhase::FetchPrepared => ("fetch_prepared", Ok(())),
        RemoteOperationPhase::FetchObserved => ("fetch_observed", Ok(())),
        RemoteOperationPhase::LocalPrepared => ("local_prepared", Ok(())),
        RemoteOperationPhase::LocalFastForwarded => ("local_fast_forwarded", Ok(())),
        RemoteOperationPhase::PushPrepared => ("push_prepared", Ok(())),
        RemoteOperationPhase::PushReturned => ("push_returned", Ok(())),
        RemoteOperationPhase::PushVerified => ("push_verified", Ok(())),
        RemoteOperationPhase::Reconciling => ("reconciling", Ok(())),
        // Stopped, with work only this operation ID can take up again.
        RemoteOperationPhase::Interrupted => ("interrupted", Ok(())),
        RemoteOperationPhase::Failed => ("failed", Ok(())),
        RemoteOperationPhase::Completed => ("completed", Err(FinalKind::Completed)),
        RemoteOperationPhase::Cancelled => ("cancelled", Err(FinalKind::Cancelled)),
    }
}

const REMOTE_PHASES: [RemoteOperationPhase; 15] = [
    RemoteOperationPhase::Reserved,
    RemoteOperationPhase::Advertising,
    RemoteOperationPhase::Persisting,
    RemoteOperationPhase::Completed,
    RemoteOperationPhase::Interrupted,
    RemoteOperationPhase::Cancelled,
    RemoteOperationPhase::Failed,
    RemoteOperationPhase::FetchPrepared,
    RemoteOperationPhase::FetchObserved,
    RemoteOperationPhase::LocalPrepared,
    RemoteOperationPhase::LocalFastForwarded,
    RemoteOperationPhase::PushPrepared,
    RemoteOperationPhase::PushReturned,
    RemoteOperationPhase::PushVerified,
    RemoteOperationPhase::Reconciling,
];

#[test]
fn the_journal_lookup_reads_a_synchronization_in_the_remote_journal() {
    let world = world();
    let data = world.data.path();
    operations::configure_remote(data);
    assert_eq!(
        world.journal_row(OperationFamily::Remote, OPERATION_A),
        JournalRow::Absent
    );
    operations::insert_remote_synchronization(
        data,
        OPERATION_A,
        Some(("ticket", items::TICKET_A)),
        "reserved",
        None,
        None,
    );
    let set = |assignments: &str| {
        world
            .index()
            .execute_batch(&format!(
                "UPDATE remote_operation_records SET {assignments}
                  WHERE operation_ulid = '{OPERATION_A}'"
            ))
            .unwrap();
    };

    // Every phase, owing nothing and then owing a reconciliation.
    for phase in REMOTE_PHASES {
        let (stored, standing) = remote_standing(phase);
        for owes in [false, true] {
            set(&format!(
                "phase = '{stored}', completed_step = 'after_fetch',
                 reconciliation_required = {}",
                i32::from(owes)
            ));
            let row = world.journal_row(OperationFamily::Remote, OPERATION_A);
            let expected = match standing {
                // A pending row is pending whatever else it owes.
                Ok(()) => pending_remote(phase),
                Err(kind) if owes => owing(kind),
                Err(kind) => ended(kind),
            };
            assert_eq!(row, expected, "{stored}, owes: {owes}");
            assert_eq!(row.in_flight(), standing.is_ok() || owes, "{stored}");
        }
    }

    // A cancelled row that owes work is still a cancelled row, and is
    // told apart from a pending one.
    set("phase = 'cancelled', reconciliation_required = 1");
    let cancelled = world.journal_row(OperationFamily::Remote, OPERATION_A);
    assert_eq!(cancelled, owing(FinalKind::Cancelled));
    assert!(!matches!(cancelled, JournalRow::Pending(_)));
    set("phase = 'cancelled', reconciliation_required = 0");
    assert_eq!(
        world.journal_row(OperationFamily::Remote, OPERATION_A),
        ended(FinalKind::Cancelled)
    );

    // A synchronization that completed and has not yet handed what it
    // published to the index owes that hand-off.
    operations::insert_remote_index_pending(data, OPERATION_B);
    assert_eq!(
        world.journal_row(OperationFamily::Remote, OPERATION_B),
        owing(FinalKind::Completed)
    );

    assert_eq!(
        world.journal_row(OperationFamily::Local, OPERATION_A),
        JournalRow::Absent
    );
    assert_eq!(
        world.journal_row(OperationFamily::Remote, operations::OPERATION_ABSENT),
        JournalRow::Absent
    );
    // And a root answers for its own operations only.
    assert_eq!(
        world
            .service
            .journal_row(
                OperationFamily::Remote,
                &ScopeKey::from_stored("/projects/other"),
                operations::operation_id(OPERATION_A),
            )
            .unwrap(),
        JournalRow::Absent
    );
}

/// What the lookup says of a key-material operation in a phase.
fn key_standing(phase: KeyMaterialPhase) -> (&'static str, &'static str, Result<(), FinalKind>) {
    match phase {
        KeyMaterialPhase::Reserved => ("generate", "reserved", Ok(())),
        KeyMaterialPhase::PrivateWritten => ("generate", "private-written", Ok(())),
        KeyMaterialPhase::PairWritten => ("generate", "pair-written", Ok(())),
        KeyMaterialPhase::Prepared => ("delete", "prepared", Ok(())),
        KeyMaterialPhase::PrivateRemoved => ("delete", "private-removed", Ok(())),
        KeyMaterialPhase::FilesRemoved => ("delete", "files-removed", Ok(())),
        KeyMaterialPhase::Completed => ("generate", "completed", Err(FinalKind::Completed)),
        KeyMaterialPhase::RetainedForInspection => (
            "generate",
            "retained-for-inspection",
            Err(FinalKind::RetainedForInspection),
        ),
    }
}

#[test]
fn the_journal_lookup_reads_a_key_operation() {
    let world = world();
    let data = world.data.path();
    assert_eq!(
        world.journal_row(OperationFamily::KeyMaterial, OPERATION_A),
        JournalRow::Absent
    );
    operations::insert_key_material(data, OPERATION_A, "generate", "reserved", None);
    operations::insert_key_material(data, OPERATION_B, "delete", "prepared", None);
    let set = |id: &str, phase: &str, failure: Option<&str>| {
        world
            .index()
            .execute(
                "UPDATE key_material_operations SET phase = ?2, failure_code = ?3
                  WHERE operation_id = ?1",
                rusqlite::params![id, phase, failure],
            )
            .unwrap();
    };

    for phase in [
        KeyMaterialPhase::Reserved,
        KeyMaterialPhase::PrivateWritten,
        KeyMaterialPhase::PairWritten,
        KeyMaterialPhase::Prepared,
        KeyMaterialPhase::PrivateRemoved,
        KeyMaterialPhase::FilesRemoved,
        KeyMaterialPhase::Completed,
        KeyMaterialPhase::RetainedForInspection,
    ] {
        let (action, stored, standing) = key_standing(phase);
        let id = if action == "generate" {
            OPERATION_A
        } else {
            OPERATION_B
        };
        // Whether the operation failed changes nothing.
        for failure in [None, Some("storage-unavailable")] {
            set(id, stored, failure);
            let row = world.journal_row(OperationFamily::KeyMaterial, id);
            let expected = match standing {
                Ok(()) => pending_key(phase),
                Err(kind) => ended(kind),
            };
            assert_eq!(row, expected, "{stored}");
            assert_eq!(row.in_flight(), standing.is_ok(), "{stored}");
        }
    }

    // A deletion ends in the same two ways.
    for (stored, kind) in [
        ("completed", FinalKind::Completed),
        ("retained-for-inspection", FinalKind::RetainedForInspection),
    ] {
        set(OPERATION_B, stored, None);
        assert_eq!(
            world.journal_row(OperationFamily::KeyMaterial, OPERATION_B),
            ended(kind)
        );
    }

    // A key operation belongs to the application, whatever scope asks.
    assert_eq!(
        world
            .service
            .journal_row(
                OperationFamily::KeyMaterial,
                &ScopeKey::application(),
                operations::operation_id(OPERATION_A),
            )
            .unwrap(),
        ended(FinalKind::RetainedForInspection)
    );
}

#[test]
fn the_journal_lookup_works_for_a_root_that_no_longer_exists() {
    let world = world();
    let data = world.data.path();
    operations::configure_remote(data);
    operations::insert_local(data, Some(OPERATION_A), "save_ticket", "authoring", None);
    operations::insert_remote_synchronization(
        data,
        OPERATION_B,
        None,
        "interrupted",
        None,
        Some("transport_unavailable"),
    );
    // An operation on a root that was never registered, as creating a
    // repository leaves one.
    let unregistered = ScopeKey::from_stored("/projects/never-registered");
    world
        .index()
        .execute(
            "INSERT INTO operation_records (root_path, operation_ulid, action, target, state,
                                            completed_step, observed_at)
             VALUES (?1, ?2, 'create_and_enable', '', 'created', 'initialization_committed', 1)",
            [unregistered.as_str(), OPERATION_C],
        )
        .unwrap();
    let World {
        fixture,
        data,
        service,
        scope,
        ..
    } = world;
    let root = fixture.root.clone();
    drop(fixture);
    assert!(!root.exists());
    assert!(ScopeKey::for_repository(&root).is_none());
    let lookup = |family, scope: &ScopeKey, id: &str| {
        service
            .journal_row(family, scope, operations::operation_id(id))
            .unwrap()
    };

    assert_eq!(
        lookup(OperationFamily::Local, &scope, OPERATION_A),
        pending_local("authoring", None)
    );
    assert_eq!(
        lookup(OperationFamily::Remote, &scope, OPERATION_B),
        pending_remote(RemoteOperationPhase::Interrupted)
    );
    assert_eq!(
        lookup(OperationFamily::Local, &unregistered, OPERATION_C),
        pending_local("created", Some("initialization_committed"))
    );
    assert_eq!(
        lookup(OperationFamily::Remote, &unregistered, OPERATION_C),
        JournalRow::Absent
    );
    drop(data);
}

#[test]
fn the_journal_lookup_reports_what_it_cannot_read() {
    let degraded = degraded_world();
    for family in OperationFamily::ALL {
        assert_eq!(
            degraded
                .service
                .journal_row(
                    family,
                    &degraded.scope,
                    operations::operation_id(OPERATION_A)
                )
                .unwrap_err()
                .code(),
            ResultCode::IndexUnavailable
        );
    }

    // A state no operation writes is not passed on.
    let world = world();
    operations::insert_local(
        world.data.path(),
        Some(OPERATION_A),
        "save_document",
        "Not A State",
        None,
    );
    let lookup = || {
        world
            .service
            .journal_row(
                OperationFamily::Local,
                &world.scope,
                operations::operation_id(OPERATION_A),
            )
            .map_err(|error| error.code())
    };
    assert_eq!(lookup(), Err(ResultCode::InternalError));
    world.set_local(OPERATION_A, "authoring", Some("Not A Step"));
    assert_eq!(lookup(), Err(ResultCode::InternalError));

    // The lookup holds no lock once it has answered.
    world.set_local(OPERATION_A, "authoring", None);
    assert_eq!(lookup(), Ok(pending_local("authoring", None)));
    drop(
        RepositoryService::hold_lease_for_testing(
            &world.fixture.root,
            world.data.path(),
            LeaseKind::CacheWrite,
        )
        .unwrap(),
    );
}

const SHOW: &str = "request show";

impl World {
    /// A request that began a local and a remote operation.
    fn insert_two_operation_request(&self, id: &str) {
        let mut request = self.request(id);
        request.operations.push(RequestOperation {
            family: OperationFamily::Remote,
            operation_id: operations::operation_id(OPERATION_B),
        });
        self.service.insert_request(&request).unwrap();
    }
}

fn case<'a>(
    name: &'a str,
    data_schema: Option<&'a str>,
    placeholders: &'a [(&'a str, &'a str)],
    sentinels: &'a [&'a str],
) -> ContractCase<'a> {
    ContractCase {
        name,
        data_schema,
        placeholders,
        sentinels,
    }
}

#[test]
fn show_request_returns_an_accepted_request() {
    let world = world();
    world.insert_two_operation_request(REQUEST_A);
    world.clock.set(T0 + 60);
    assert_eq!(
        world.service.enter_request(request_id(REQUEST_A)).unwrap(),
        Some(2)
    );

    let envelope = world.service.show_request(request_id(REQUEST_A));

    assert_eq!(
        (envelope.outcome, envelope.code, envelope.command.as_str()),
        (Outcome::Success, ResultCode::Ok, SHOW)
    );
    assert_eq!(
        envelope.scope.repository.as_deref(),
        Some(world.scope.as_str())
    );
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(
        envelope.data,
        Some(RequestDto {
            request_id: REQUEST_A.to_owned(),
            state: RequestState::Accepted,
            command: "document save".to_owned(),
            accepted_at: operations::STORED_AT_TEXT.to_owned(),
            finished_at: None,
            operations: vec![
                RequestOperationDto {
                    family: OperationFamily::Local,
                    operation_id: OPERATION_A.to_owned(),
                },
                RequestOperationDto {
                    family: OperationFamily::Remote,
                    operation_id: OPERATION_B.to_owned(),
                },
            ],
            result: None,
        })
    );
    let data_directory = world.data.path().to_str().unwrap();
    mutation::assert_contract(
        &case(
            "request_show_accepted",
            Some("request.schema.json"),
            &[(world.scope.as_str(), "<repository>")],
            &[BODY_SENTINEL, data_directory],
        ),
        &envelope,
    );
}

#[test]
fn show_request_returns_a_finished_request_with_its_stored_result() {
    let world = world();
    world.insert_two_operation_request(REQUEST_A);
    world.clock.set(T0 + 100);
    world
        .service
        .finish_request(request_id(REQUEST_A), 1, &committed())
        .unwrap();

    let envelope = world.service.show_request(request_id(REQUEST_A));

    // The read succeeded; what the request came to is in its data.
    assert_eq!(
        (envelope.outcome, envelope.code),
        (Outcome::Success, ResultCode::Ok)
    );
    let data = envelope.data.clone().unwrap();
    assert_eq!(data.state, RequestState::Finished);
    assert_eq!(data.accepted_at, operations::STORED_AT_TEXT);
    assert_eq!(data.finished_at.as_deref(), Some("2023-11-14T22:15:00Z"));
    assert_eq!(data.operations.len(), 2);
    assert_eq!(
        data.result,
        Some(RequestResultDto {
            outcome: Outcome::Success,
            code: ResultCode::Ok,
            message: ResultCode::Ok.message().to_owned(),
            effects: committed().effects,
            data: committed().data,
        })
    );
    let data_directory = world.data.path().to_str().unwrap();
    mutation::assert_contract(
        &case(
            "request_show_finished",
            Some("request.schema.json"),
            &[(world.scope.as_str(), "<repository>")],
            &[BODY_SENTINEL, data_directory],
        ),
        &envelope,
    );

    // A result that carried no data shows none.
    world
        .service
        .insert_request(&world.request(REQUEST_B))
        .unwrap();
    world
        .service
        .finish_request(request_id(REQUEST_B), 1, &already_applied())
        .unwrap();
    let noop = world
        .service
        .show_request(request_id(REQUEST_B))
        .data
        .unwrap()
        .result
        .unwrap();
    assert_eq!(
        (noop.outcome, noop.code, noop.data),
        (Outcome::Noop, ResultCode::AlreadyApplied, None)
    );
}

#[test]
fn show_request_of_an_application_request_names_no_repository() {
    let world = world();
    let request = NewRequest {
        scope: ScopeKey::application(),
        ..world.request(REQUEST_A)
    };
    world.service.insert_request(&request).unwrap();

    let envelope = world.service.show_request(request_id(REQUEST_A));

    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(envelope.scope, Scope::default());
}

#[test]
fn show_request_reports_a_request_no_record_holds() {
    let world = world();
    world
        .service
        .insert_request(&world.request(REQUEST_A))
        .unwrap();

    let envelope = world.service.show_request(request_id(REQUEST_ABSENT));

    assert_eq!(envelope.code, ResultCode::RequestNotFound);
    assert_eq!(envelope.outcome, Outcome::Error);
    assert_eq!(envelope.data, None);
    assert_eq!(envelope.scope, Scope::default());
    mutation::assert_contract(
        &case("failure_request_not_found", None, &[], &[BODY_SENTINEL]),
        &envelope,
    );

    // A deleted record is no longer shown.
    assert!(
        world
            .service
            .delete_request(request_id(REQUEST_A), 1)
            .unwrap()
    );
    assert_eq!(
        world.service.show_request(request_id(REQUEST_A)).code,
        ResultCode::RequestNotFound
    );
}

#[test]
fn show_request_reports_an_unavailable_or_unreadable_database() {
    let degraded = degraded_world();
    let envelope = degraded.service.show_request(request_id(REQUEST_A));
    assert_eq!(envelope.code, ResultCode::IndexUnavailable);
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(envelope.data, None);
    assert_eq!(
        envelope
            .recovery
            .iter()
            .map(|action| action.action)
            .collect::<Vec<_>>(),
        [RecoveryActionKind::IndexRebuild]
    );

    let world = world();
    world
        .service
        .insert_request(&world.request(REQUEST_A))
        .unwrap();
    let held = RepositoryService::hold_lease_for_testing(
        &world.fixture.root,
        world.data.path(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    assert_eq!(
        world.service.show_request(request_id(REQUEST_A)).code,
        ResultCode::Busy
    );
    drop(held);

    // A row no record function wrote is not passed on.
    world
        .index()
        .execute_batch("UPDATE request_operations SET operation_ulid = 'not an operation ID'")
        .unwrap();
    let envelope = world.service.show_request(request_id(REQUEST_A));
    assert_eq!(envelope.code, ResultCode::InternalError);
    assert_eq!(envelope.data, None);
}

#[test]
fn the_mutation_goldens_are_exactly_the_registered_cases() {
    let mut fixtures: Vec<_> = std::fs::read_dir(mutation::fixture_directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    fixtures.sort();
    let mut cases: Vec<_> = mutation::CASES
        .iter()
        .map(|case| format!("{case}.json"))
        .collect();
    cases.sort();
    assert_eq!(fixtures, cases);

    for case in mutation::CASES {
        let path = mutation::fixture_directory().join(format!("{case}.json"));
        let text = std::fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(golden::render(&value, &[]), text, "{path:?}");
        schema::check_published(golden::ENVELOPE_SCHEMA, &value).unwrap();
    }
}

#[path = "mutation_replay/execute.rs"]
mod execute;

#[path = "mutation_replay/cache_loss.rs"]
mod cache_loss;

#[path = "mutation_replay/reenter.rs"]
mod reenter;

/// The process `support::hold_lease_in_child` starts: it holds the lease
/// it is told to until it is released.
#[test]
fn common_git_lease_child() {
    let Ok(root) = std::env::var("MANYHANDS_LEASE_ROOT") else {
        return;
    };
    let variable = |name: &str| std::path::PathBuf::from(std::env::var(name).unwrap());
    let kind = LeaseKind::parse(&std::env::var("MANYHANDS_LEASE_KIND").unwrap()).unwrap();
    let _holder = RepositoryService::hold_lease_for_testing(
        Path::new(&root),
        &variable("MANYHANDS_LEASE_DATA_DIRECTORY"),
        kind,
    )
    .unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(variable("MANYHANDS_LEASE_READY"))
        .unwrap();
    let release = variable("MANYHANDS_LEASE_RELEASE");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !release.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for lease release"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
