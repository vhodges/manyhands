//! Cancelling a setup (design Part H, P10): the remote branch is deleted only
//! at its recorded commit, the local branch, the private ref and the record
//! are removed, a first-time setup is unregistered, and both requests are
//! finished in one transaction. Local repositories on disk and a stubbed
//! channel over a bare repository; no network, no waits.

use std::time::Duration;

use git2::{Oid, Repository};
use rusqlite::{OptionalExtension, params};

use super::*;
use crate::{
    repository::{
        OperationFamily, OperationId, RequestState,
        request_store::{DigestSalt, IntentDigest, NewRequest, RequestOperation, ScopeKey},
        setup::{
            commit,
            record::{self, SetupKind},
            test_world::{CURRENT, OTHER, SETUP, Stub, World, identity},
        },
        transport::{
            DeleteAttempt, SetupAdvertisement, SetupPush, SetupScope, SshDirection,
            SshTransportErrorKind,
        },
    },
    results::{Effects, ResultCode},
};

/// What the server does with the guarded delete it is sent.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Delete {
    /// The branch is deleted while it is at the commit.
    Honest,
    /// The server refuses: nothing is deleted, and the connection says so.
    Refused,
    /// The delete is lost: nothing is deleted, and the connection says it
    /// was sent.
    Lost,
    /// A peer moves the branch to another commit as the delete arrives:
    /// nothing is deleted.
    Moved,
    /// The delete lands, and the connection fails after it.
    LandsThenFails(SshTransportErrorKind),
}

/// The world's stub for the fetch, and the remote's answers for the rest.
/// It counts every connection, in order, and sends nothing but a delete.
struct CancelStub {
    inner: Stub,
    delete: Delete,
    calls: Vec<&'static str>,
    observe_fails: Option<SshTransportErrorKind>,
}

impl CancelStub {
    fn new(world: &World, delete: Delete) -> Self {
        Self {
            inner: world.stub(),
            delete,
            calls: Vec::new(),
            observe_fails: None,
        }
    }

    fn error(
        &self,
        scope: &SetupScope,
        direction: SshDirection,
        kind: SshTransportErrorKind,
    ) -> SshTransportError {
        SshTransportError {
            root: self.inner.local.clone(),
            remote_name: scope.remote_name.clone(),
            direction,
            selected_key_id: None,
            authority: None,
            kind,
        }
    }

    fn deletes(&self) -> usize {
        self.calls.iter().filter(|call| **call == "delete").count()
    }
}

impl SetupChannel for CancelStub {
    fn fetch_base(&mut self, scope: &SetupScope) -> Result<SetupAdvertisement, SshTransportError> {
        self.calls.push("fetch");
        self.inner.fetch_base(scope)
    }

    fn push_branch(&mut self, _: &SetupScope, _: Oid) -> Result<SetupPush, SshTransportError> {
        panic!("a cancel never pushes");
    }

    fn delete_branch(
        &mut self,
        scope: &SetupScope,
        expected: Oid,
    ) -> Result<DeleteAttempt, SshTransportError> {
        self.calls.push("delete");
        let remote = Repository::open(&self.inner.remote).unwrap();
        let at = remote
            .find_reference(&scope.branch_ref())
            .ok()
            .and_then(|reference| reference.target());
        let remove = || {
            remote
                .find_reference(&scope.branch_ref())
                .unwrap()
                .delete()
                .unwrap()
        };
        match self.delete.clone() {
            // The guard: the server deletes only a branch that is at the
            // commit the command names.
            Delete::Honest => match at {
                None => Ok(DeleteAttempt::Absent),
                Some(oid) if oid == expected => {
                    remove();
                    Ok(DeleteAttempt::Sent)
                }
                Some(_) => Err(self.error(
                    scope,
                    SshDirection::Delete,
                    SshTransportErrorKind::PushRejected,
                )),
            },
            Delete::Refused => Err(self.error(
                scope,
                SshDirection::Delete,
                SshTransportErrorKind::PushRejected,
            )),
            Delete::Lost => Ok(DeleteAttempt::Sent),
            Delete::Moved => {
                let other =
                    commit::build(&remote, expected, b"a peer's file", "A peer's", &identity())
                        .unwrap();
                remote
                    .reference(&scope.branch_ref(), other, true, "a peer")
                    .unwrap();
                Ok(DeleteAttempt::Sent)
            }
            Delete::LandsThenFails(kind) => {
                if at == Some(expected) {
                    remove();
                }
                Err(self.error(scope, SshDirection::Delete, kind))
            }
        }
    }

    fn observe_branch(&mut self, scope: &SetupScope) -> Result<Option<Oid>, SshTransportError> {
        self.calls.push("observe");
        if let Some(kind) = self.observe_fails.clone() {
            return Err(self.error(scope, SshDirection::Fetch, kind));
        }
        Ok(Repository::open(&self.inner.remote)
            .unwrap()
            .find_reference(&scope.branch_ref())
            .ok()
            .and_then(|reference| reference.target()))
    }
}

/// The cancel's own request, accepted, with its operation, as the binding
/// leaves them before the domain call.
fn own_request(world: &World) -> CancelSetupRequest {
    let request_id = RequestId::new();
    let operation_id = OperationId::new();
    let scope = ScopeKey::for_repository(&world.root).unwrap();
    let digest = IntentDigest::builder(
        DigestSalt::Request(request_id),
        "repo setup-cancel",
        &scope,
        "",
    )
    .finish();
    world
        .service
        .insert_request(&NewRequest {
            request_id,
            scope,
            command: "repo setup-cancel".into(),
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
    CancelSetupRequest {
        root: world.root.clone(),
        operation_id,
        request_id,
    }
}

fn cancel(
    world: &World,
    request: &CancelSetupRequest,
    stub: &mut CancelStub,
) -> Result<SetupCancelReport, SetupCancelError> {
    world
        .service
        .cancel_setup_with(request, stub, Some(Duration::from_millis(1)))
}

fn other_commit(world: &World) -> Oid {
    commit::build(
        &world.remote(),
        world.c0,
        b"a peer's file",
        "A peer's",
        &identity(),
    )
    .unwrap()
}

/// The cancel's own row: its action, state and last step.
fn own_row(
    world: &World,
    request: &CancelSetupRequest,
) -> Option<(String, String, Option<String>)> {
    world
        .db()
        .query_row(
            "SELECT action, state, completed_step FROM operation_records WHERE operation_ulid = ?1",
            [request.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .unwrap()
}

fn recorded_row() -> Option<(String, String, Option<String>)> {
    Some((
        "cancel_setup".to_owned(),
        "completed".to_owned(),
        Some("setup_cancel_recorded".to_owned()),
    ))
}

/// Every local ref but the private base ref, which every fetch rewrites.
fn refs_without_the_base(world: &World) -> std::collections::BTreeMap<String, Option<Oid>> {
    let mut refs = world.refs();
    refs.remove(&world.base_ref());
    refs
}

fn registered(world: &World) -> bool {
    world.count(&format!(
        "SELECT COUNT(*) FROM repositories WHERE root_path = '{}'",
        world.root.to_str().unwrap()
    )) == 1
}

fn operation_rows(world: &World) -> i64 {
    world.count(&format!(
        "SELECT COUNT(*) FROM operation_records WHERE root_path = '{}'",
        world.root.to_str().unwrap()
    ))
}

/// What a case expects of a cancel.
enum Outcome {
    /// The setup is gone, and both requests are finished.
    Cancelled {
        remote_branch: RemoteBranch,
        local_branch: LocalBranch,
        /// The remote still has a setup branch (another commit's).
        remote_ref_left: bool,
    },
    /// The cancel stopped with `code`; the record, the branches, the
    /// registration and the setup's request are as they were. `own_row` is
    /// whether the cancel's own row had been recorded.
    Stopped { code: ResultCode, own_row: bool },
}

struct Case {
    what: &'static str,
    kind: SetupKind,
    published: bool,
    arrange: fn(&World),
    delete: Delete,
    fetch_fails: Option<SshTransportErrorKind>,
    calls: &'static [&'static str],
    outcome: Outcome,
}

fn case(what: &'static str, outcome: Outcome) -> Case {
    Case {
        what,
        kind: SetupKind::Enable,
        published: true,
        arrange: |_| {},
        delete: Delete::Honest,
        fetch_fails: None,
        calls: &["fetch", "delete", "observe"],
        outcome,
    }
}

fn deleted(remote_branch: RemoteBranch, local_branch: LocalBranch, left: bool) -> Outcome {
    Outcome::Cancelled {
        remote_branch,
        local_branch,
        remote_ref_left: left,
    }
}

fn the_merged_file(world: &World) -> Vec<u8> {
    world.record().unwrap().configuration
}

fn cases() -> Vec<Case> {
    let in_the_way = |world: &World| {
        world
            .db()
            .execute(
                "INSERT INTO operation_records
                    (repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at)
                 VALUES (?1,?2,?3,'enable','','created',NULL,1)",
                params![
                    world.repository_id,
                    world.root.to_str().unwrap(),
                    OperationId::new().to_string()
                ],
            )
            .unwrap();
    };
    vec![
        Case {
            published: false,
            calls: &[],
            ..case(
                "no commit: no connection, and a first-time setup is unregistered",
                deleted(RemoteBranch::NotPublished, LocalBranch::Absent, false),
            )
        },
        Case {
            published: false,
            kind: SetupKind::Change,
            calls: &[],
            ..case(
                "a change that was never published keeps the registration",
                deleted(RemoteBranch::NotPublished, LocalBranch::Absent, false),
            )
        },
        case(
            "published and unmoved: the remote ref is deleted at the recorded commit",
            deleted(RemoteBranch::Deleted, LocalBranch::Deleted, false),
        ),
        Case {
            delete: Delete::LandsThenFails(SshTransportErrorKind::TransportUnavailable),
            ..case(
                "a delete that landed and then failed is seen by the observation",
                deleted(RemoteBranch::Deleted, LocalBranch::Deleted, false),
            )
        },
        Case {
            // The advertisement of the delete's own connection had the branch
            // at the commit, and a peer moved it as the delete arrived.
            delete: Delete::Moved,
            ..case(
                "the observation finds another commit (Changed): kept, said so",
                deleted(RemoteBranch::Kept, LocalBranch::Deleted, true),
            )
        },
        Case {
            arrange: |world| {
                let other = other_commit(world);
                world
                    .remote()
                    .reference(&world.branch_ref(), other, true, "a peer")
                    .unwrap();
            },
            calls: &["fetch"],
            ..case(
                "the branch is at another commit before any delete: kept, no delete sent",
                deleted(RemoteBranch::Kept, LocalBranch::Deleted, true),
            )
        },
        Case {
            delete: Delete::Refused,
            ..case(
                "a refused delete is push_rejected, and nothing more changes",
                Outcome::Stopped {
                    code: ResultCode::PushRejected,
                    own_row: true,
                },
            )
        },
        Case {
            delete: Delete::Lost,
            ..case(
                "a lost delete is push_rejected, and nothing more changes",
                Outcome::Stopped {
                    code: ResultCode::PushRejected,
                    own_row: true,
                },
            )
        },
        Case {
            arrange: |world| world.delete_remote_branch(),
            calls: &["fetch"],
            ..case(
                "absent: it goes on",
                deleted(RemoteBranch::Absent, LocalBranch::Deleted, false),
            )
        },
        Case {
            // The push landed and its record did not (S7): the branch is
            // there at the commit, and the record says it is not published.
            arrange: |world| {
                world
                    .db()
                    .execute("UPDATE repository_setups SET published_oid = NULL", [])
                    .unwrap();
            },
            ..case(
                "a pushed but unrecorded ref is found and deleted",
                deleted(RemoteBranch::Deleted, LocalBranch::Deleted, false),
            )
        },
        Case {
            arrange: |world| {
                world.fast_forward();
            },
            calls: &["fetch"],
            ..case(
                "already merged by a fast-forward: recovery_required, nothing changed",
                Outcome::Stopped {
                    code: ResultCode::RecoveryRequired,
                    own_row: true,
                },
            )
        },
        Case {
            // A squash, the forge deleted the branch afterwards, and the
            // root has pulled: the base holds the setup, whatever the
            // checkout and the branch.
            arrange: |world| {
                let tip = world.squash(&the_merged_file(world));
                world.delete_remote_branch();
                world.pull(tip);
            },
            calls: &["fetch"],
            ..case(
                "already merged, branch gone, pulled: recovery_required",
                Outcome::Stopped {
                    code: ResultCode::RecoveryRequired,
                    own_row: true,
                },
            )
        },
        Case {
            kind: SetupKind::Change,
            arrange: |world| {
                world.merge_commit(&the_merged_file(world));
            },
            calls: &["fetch"],
            ..case(
                "a change already merged by a merge commit: recovery_required",
                Outcome::Stopped {
                    code: ResultCode::RecoveryRequired,
                    own_row: true,
                },
            )
        },
        Case {
            kind: SetupKind::Change,
            ..case(
                "a change keeps the registration",
                deleted(RemoteBranch::Deleted, LocalBranch::Deleted, false),
            )
        },
        Case {
            // A person checked the setup branch out: deleting it would leave
            // HEAD naming a branch that is not there.
            arrange: |world| {
                world.git().set_head(&world.branch_ref()).unwrap();
            },
            ..case(
                "the setup branch is checked out: kept, and the rest goes on",
                deleted(RemoteBranch::Deleted, LocalBranch::Kept, false),
            )
        },
        Case {
            arrange: |world| {
                let other = world.c0;
                world
                    .git()
                    .reference(&world.branch_ref(), other, true, "moved")
                    .unwrap();
            },
            ..case(
                "a local branch at another commit is kept",
                deleted(RemoteBranch::Deleted, LocalBranch::Kept, false),
            )
        },
        Case {
            arrange: |world| {
                let mut db = world.db();
                let tx = db.transaction().unwrap();
                record::delete(&tx, world.repository_id).unwrap();
                tx.commit().unwrap();
            },
            calls: &[],
            ..case(
                "no record is already_applied",
                Outcome::Stopped {
                    code: ResultCode::AlreadyApplied,
                    own_row: false,
                },
            )
        },
        Case {
            arrange: in_the_way,
            calls: &[],
            ..case(
                "another pending operation row is in the way: recovery_required, nothing sent",
                Outcome::Stopped {
                    code: ResultCode::RecoveryRequired,
                    own_row: false,
                },
            )
        },
        Case {
            fetch_fails: Some(SshTransportErrorKind::NoSelectedKey),
            calls: &["fetch"],
            ..case(
                "a key stop changes nothing and leaves the request accepted",
                Outcome::Stopped {
                    code: ResultCode::NoSelectedKey,
                    own_row: true,
                },
            )
        },
    ]
}

#[test]
fn cancel_outcomes() {
    for case in cases() {
        let what = case.what;
        let world = World::new(case.kind, OTHER, case.published);
        (case.arrange)(&world);
        let before = world.record();
        let refs_before = refs_without_the_base(&world);
        let remote_before = world.remote_ref(&world.branch_ref());
        let rows_before = operation_rows(&world);
        let request = own_request(&world);
        let mut stub = CancelStub::new(&world, case.delete.clone());
        stub.inner.fails = case.fetch_fails.clone();

        let answer = cancel(&world, &request, &mut stub);

        assert_eq!(stub.calls, case.calls, "{what}: the connections, in order");
        let own = world
            .service
            .request_record(request.request_id)
            .unwrap()
            .unwrap();
        match case.outcome {
            Outcome::Cancelled {
                remote_branch,
                local_branch,
                remote_ref_left,
            } => {
                let report = answer.unwrap_or_else(|error| panic!("{what}: {error:?}"));
                let unregistered = case.kind == SetupKind::Enable;
                assert_eq!(report.setup_id.to_string(), SETUP, "{what}");
                assert_eq!(report.kind, case.kind, "{what}");
                assert_eq!(report.remote_branch, remote_branch, "{what}");
                assert_eq!(report.local_branch, local_branch, "{what}");
                assert_eq!(report.unregistered, unregistered, "{what}");
                // The record, the private ref and (unless kept) the local
                // branch are gone; the registration with it for a first-time
                // setup, and not for a change.
                assert_eq!(world.record(), None, "{what}");
                assert_eq!(world.local_ref(&world.base_ref()), None, "{what}");
                assert_eq!(
                    world.local_ref(&world.branch_ref()).is_some(),
                    local_branch == LocalBranch::Kept,
                    "{what}"
                );
                assert_eq!(registered(&world), !unregistered, "{what}");
                assert_eq!(
                    world.remote_ref(&world.branch_ref()).is_some(),
                    remote_ref_left,
                    "{what}"
                );
                // The setup's request is finished as cancelled, in the same
                // transaction, with the setup's data and no effect.
                let setup_request = world.request();
                assert_eq!(setup_request.state, RequestState::Finished, "{what}");
                let stored = setup_request.result.expect("a stored result");
                assert_eq!(stored.code, ResultCode::Cancelled, "{what}");
                assert_eq!(stored.effects, Effects::not_requested(), "{what}");
                assert_eq!(stored.data.unwrap()["type"], "setup", "{what}");
                // The cancel's own row is kept, and its request is finished
                // with its answer.
                assert_eq!(own_row(&world, &request), recorded_row(), "{what}");
                assert_eq!(own.state, RequestState::Finished, "{what}");
                let stored = own.result.expect("the cancel's stored result");
                assert_eq!(stored.code, ResultCode::Ok, "{what}");
                let data = stored.data.unwrap();
                assert_eq!(data["type"], "setup_cancel", "{what}");
                assert_eq!(data["setup_id"], SETUP, "{what}");
                assert_eq!(data["remote_branch"], remote_branch.as_str(), "{what}");
                assert_eq!(data["local_branch"], local_branch.as_str(), "{what}");
                assert_eq!(data["unregistered"], unregistered, "{what}");
                // The setup's own row goes with a first-time setup's other
                // rows; a change's stays.
                assert_eq!(
                    operation_rows(&world),
                    if unregistered { 1 } else { rows_before + 1 },
                    "{what}"
                );
            }
            Outcome::Stopped {
                code,
                own_row: recorded,
            } => {
                let error = answer.expect_err(what);
                assert_eq!(error.code(), code, "{what}: {error:?}");
                // Nothing more changed: the record, every local ref, the
                // registration and the setup's request are as they were.
                assert_eq!(world.record(), before, "{what}");
                assert_eq!(refs_without_the_base(&world), refs_before, "{what}");
                assert!(registered(&world), "{what}");
                if before.is_some() {
                    assert_eq!(world.request().state, RequestState::Accepted, "{what}");
                }
                assert_eq!(
                    world.remote_ref(&world.branch_ref()),
                    remote_before,
                    "{what}"
                );
                // The cancel's own request stays accepted, to be sent again.
                assert_eq!(own.state, RequestState::Accepted, "{what}");
                assert_eq!(
                    own_row(&world, &request),
                    if recorded { recorded_row() } else { None },
                    "{what}"
                );
            }
        }
    }
}

/// A preview binds the record's state: a publish between `prepare` and the
/// confirm changes what it observes, so the confirm is `external_change`.
#[test]
fn a_publish_between_prepare_and_confirm_changes_the_state_the_preview_binds() {
    let recorded = World::new(SetupKind::Enable, OTHER, false);
    let published = World::new(SetupKind::Enable, OTHER, true);
    let a = state_key(recorded.record().as_ref());
    let b = state_key(published.record().as_ref());
    assert_ne!(a, b);
    assert_eq!(a, state_key(recorded.record().as_ref()));
    assert_ne!(a, state_key(None));
    // Another remote is another setup.
    let elsewhere = World::new(SetupKind::Enable, CURRENT, true);
    assert_ne!(b, state_key(elsewhere.record().as_ref()));
}

/// The same request finishes a cancel that stopped after any of its steps,
/// with no second delete and one answer.
#[test]
fn a_cancel_interrupted_between_steps_is_finished_by_the_same_request() {
    for step in [Step::Recorded, Step::Remote, Step::Local, Step::Transaction] {
        let world = World::new(SetupKind::Enable, OTHER, true);
        let request = own_request(&world);
        let mut stub = CancelStub::new(&world, Delete::Honest);

        STOP_AFTER.with(|stop| stop.set(Some(step)));
        let first = cancel(&world, &request, &mut stub);

        let error = first.expect_err("the call stopped");
        assert_eq!(error.code(), ResultCode::RecoveryRequired, "{step:?}");
        assert_eq!(own_row(&world, &request), recorded_row(), "{step:?}");
        let gone_remote = !matches!(step, Step::Recorded);
        assert_eq!(
            world.remote_ref(&world.branch_ref()).is_none(),
            gone_remote,
            "{step:?}"
        );
        let gone_local = matches!(step, Step::Local | Step::Transaction);
        assert_eq!(
            world.local_ref(&world.branch_ref()).is_none(),
            gone_local,
            "{step:?}"
        );
        assert_eq!(
            world.local_ref(&world.base_ref()).is_none(),
            gone_local,
            "{step:?}"
        );
        let transaction = step == Step::Transaction;
        assert_eq!(world.record().is_none(), transaction, "{step:?}");
        assert_eq!(
            world.request().state,
            if transaction {
                RequestState::Finished
            } else {
                RequestState::Accepted
            },
            "{step:?}"
        );

        // The same request again.
        let second = cancel(&world, &request, &mut stub);

        if transaction {
            // Finished in the transaction: its stored result is the answer.
            match second {
                Err(SetupCancelError::Ended(stored)) => {
                    assert_eq!(stored.code, ResultCode::Ok, "{step:?}");
                }
                other => panic!("{step:?}: {other:?}"),
            }
        } else {
            let report = second.unwrap_or_else(|error| panic!("{step:?}: {error:?}"));
            // What the first call already did is found done.
            assert_eq!(
                report.remote_branch,
                if gone_remote {
                    RemoteBranch::Absent
                } else {
                    RemoteBranch::Deleted
                },
                "{step:?}"
            );
            assert_eq!(
                report.local_branch,
                if gone_local {
                    LocalBranch::Absent
                } else {
                    LocalBranch::Deleted
                },
                "{step:?}"
            );
        }
        assert_eq!(stub.deletes(), 1, "{step:?}: one delete, ever");
        assert_eq!(world.record(), None, "{step:?}");
        assert!(!registered(&world), "{step:?}");
        assert_eq!(world.request().state, RequestState::Finished, "{step:?}");
        assert_eq!(
            world.request().result.unwrap().code,
            ResultCode::Cancelled,
            "{step:?}"
        );
        let own = world
            .service
            .request_record(request.request_id)
            .unwrap()
            .unwrap();
        assert_eq!(own.state, RequestState::Finished, "{step:?}");
        assert_eq!(own.result.unwrap().code, ResultCode::Ok, "{step:?}");
        assert_eq!(own_row(&world, &request), recorded_row(), "{step:?}");
    }
}

/// A row that another operation begins after the remote step stops the
/// transaction before it writes: the remote branch is gone, the record stays,
/// and the same request finishes once the row has ended.
#[test]
fn an_operation_that_begins_after_the_delete_stops_the_transaction_and_the_retry_finishes() {
    let world = World::new(SetupKind::Enable, OTHER, true);
    let request = own_request(&world);
    let mut stub = CancelStub::new(&world, Delete::Honest);
    let rows = world.db();
    let (repository_id, root) = (world.repository_id, world.root.clone());
    BEFORE_TRANSACTION.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            rows.execute(
                "INSERT INTO operation_records
                    (repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at)
                 VALUES (?1,?2,?3,'enable','','created',NULL,1)",
                params![repository_id, root.to_str().unwrap(), OperationId::new().to_string()],
            )
            .unwrap();
        }));
    });

    let stopped = cancel(&world, &request, &mut stub).expect_err("a row is in the way");

    assert_eq!(stopped.code(), ResultCode::RecoveryRequired);
    assert!(world.record().is_some());
    assert_eq!(world.request().state, RequestState::Accepted);
    assert_eq!(world.remote_ref(&world.branch_ref()), None);
    assert!(registered(&world));

    // The operation ends; the same request finishes, sending nothing more.
    world
        .db()
        .execute(
            "UPDATE operation_records SET state = 'completed' WHERE state = 'created'",
            [],
        )
        .unwrap();
    let report = cancel(&world, &request, &mut stub).unwrap();
    assert_eq!(report.remote_branch, RemoteBranch::Absent);
    assert_eq!(stub.deletes(), 1);
    assert_eq!(world.record(), None);
}
