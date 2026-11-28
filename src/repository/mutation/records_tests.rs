use super::*;
use crate::repository::{REGISTRY_FILE, open_registry};
use crate::results::{Effects, ResultCode};
use rusqlite::{Connection, params};

use super::super::{
    bind::Answer,
    identity::{DigestSalt, IntentDigest, ScopeKey},
};

struct Fixture {
    service: RepositoryService,
    data: tempfile::TempDir,
    /// Two registered repositories: the setup's and another.
    roots: [tempfile::TempDir; 2],
}

impl Fixture {
    fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let roots = [tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap()];
        let fixture = Self {
            service,
            data,
            roots,
        };
        let db = fixture.db();
        for root in &fixture.roots {
            db.execute(
                "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required)
                 VALUES(?1,1,'accessible',0)",
                [root.path().canonicalize().unwrap().to_str().unwrap()],
            )
            .unwrap();
        }
        drop(db);
        fixture
    }

    fn db(&self) -> Connection {
        open_registry(&self.data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap()
    }

    fn root(&self, index: usize) -> String {
        self.roots[index]
            .path()
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    fn repository_id(&self, index: usize) -> i64 {
        self.db()
            .query_row(
                "SELECT id FROM repositories WHERE root_path = ?1",
                [self.root(index)],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// Accepts a request of `command` in the repository `scope`, whose own
    /// local operation row is in `row`: its `(state, completed_step)`.
    fn accept(&self, command: &str, scope: usize, row: Option<(&str, &str)>) -> RequestId {
        let request_id = RequestId::new();
        let operation_id = OperationId::new();
        let scope_key = ScopeKey::for_repository(self.roots[scope].path()).unwrap();
        let digest =
            IntentDigest::builder(DigestSalt::Request(request_id), command, &scope_key, "")
                .finish();
        self.service
            .insert_request(&NewRequest {
                request_id,
                scope: scope_key,
                command: command.into(),
                target: String::new(),
                intent_digest: digest,
                confirmation: None,
                base_ref: None,
                base_oid: None,
                expected_digest: None,
                operations: vec![RequestOperation {
                    family: OperationFamily::Local,
                    operation_id,
                }],
            })
            .unwrap();
        if let Some((state, step)) = row {
            self.db()
                .execute(
                    "INSERT INTO operation_records
                        (repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at)
                     VALUES (?1,?2,?3,'enable','',?4,?5,1)",
                    params![
                        self.repository_id(scope),
                        self.root(scope),
                        operation_id.to_string(),
                        state,
                        step
                    ],
                )
                .unwrap();
        }
        request_id
    }

    /// Runs `finish_accepted` for the repository `repository` in its own
    /// transaction.
    fn finish(&self, repository: usize, request_id: RequestId, code: ResultCode) -> bool {
        let mut db = self.db();
        let tx = db.transaction().unwrap();
        let finished = finish_accepted(
            &tx,
            self.repository_id(repository),
            request_id,
            &Answer::stopped(code),
            1_700_000_000,
        )
        .unwrap();
        tx.commit().unwrap();
        finished
    }
}

#[test]
fn finish_accepted_finishes_only_a_setup_request_of_its_repository() {
    let f = Fixture::new();
    let recorded = ("completed", "setup_recorded");
    // (name, command, scope, local row, whether it finishes)
    let cases = vec![
        ("repo enable", "repo enable", 0, Some(recorded), true),
        ("remote select", "remote select", 0, Some(recorded), true),
        ("another command", "ticket save", 0, Some(recorded), false),
        ("another scope", "repo enable", 1, Some(recorded), false),
        (
            "a row at another step",
            "repo enable",
            0,
            Some(("completed", "initialization_committed")),
            false,
        ),
        (
            "a row that is not completed",
            "repo enable",
            0,
            Some(("created", "setup_recorded")),
            false,
        ),
        ("no local row", "repo enable", 0, None, false),
    ];
    for (name, command, scope, row, finishes) in cases {
        let id = f.accept(command, scope, row);
        assert_eq!(
            f.finish(0, id, ResultCode::RemoteAlreadySetUp),
            finishes,
            "{name}"
        );
        let record = f.service.request_record(id).unwrap().unwrap();
        if finishes {
            assert_eq!(record.state, RequestState::Finished, "{name}");
            let result = record.result.unwrap();
            assert_eq!(result.code, ResultCode::RemoteAlreadySetUp, "{name}");
            assert_eq!(result.effects, Effects::not_requested(), "{name}");
            assert_eq!(result.data, None, "{name}");
        } else {
            assert_eq!(record.state, RequestState::Accepted, "{name}");
            assert_eq!(record.result, None, "{name}");
        }
    }

    // A request that finished is not finished again, whatever the answer.
    let id = f.accept("repo enable", 0, Some(recorded));
    assert!(f.finish(0, id, ResultCode::Cancelled));
    assert!(!f.finish(0, id, ResultCode::ExternalChange));
    let result = f
        .service
        .request_record(id)
        .unwrap()
        .unwrap()
        .result
        .unwrap();
    assert_eq!(result.code, ResultCode::Cancelled);

    // It finishes at the request's current attempt: a call that entered the
    // request again since is not the one whose ending counts.
    let id = f.accept("remote select", 0, Some(recorded));
    assert_eq!(f.service.enter_request(id).unwrap(), Some(2));
    assert_eq!(f.service.enter_request(id).unwrap(), Some(3));
    assert!(f.finish(0, id, ResultCode::ExternalChange));
    let record = f.service.request_record(id).unwrap().unwrap();
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.attempt, 3);

    // A request nobody accepted is none.
    assert!(!f.finish(0, RequestId::new(), ResultCode::Cancelled));
}
