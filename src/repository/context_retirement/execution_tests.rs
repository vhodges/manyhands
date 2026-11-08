use crate::repository::*;
use rusqlite::{Connection, params};
#[path = "index_claim_tests.rs"]
mod index_claim_tests;
#[path = "mutation_tests.rs"]
mod mutation_tests;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

#[derive(Debug, PartialEq, Eq)]
enum DiskNode {
    Directory,
    File(Vec<u8>),
    Link(PathBuf),
}

fn disk_image(root: &Path) -> BTreeMap<PathBuf, DiskNode> {
    fn visit(root: &Path, at: &Path, image: &mut BTreeMap<PathBuf, DiskNode>) {
        let metadata = std::fs::symlink_metadata(at).unwrap();
        let node = if metadata.file_type().is_symlink() {
            DiskNode::Link(std::fs::read_link(at).unwrap())
        } else if metadata.is_dir() {
            DiskNode::Directory
        } else {
            assert!(metadata.is_file());
            DiskNode::File(std::fs::read(at).unwrap())
        };
        let directory = node == DiskNode::Directory;
        image.insert(at.strip_prefix(root).unwrap().into(), node);
        if directory {
            for entry in std::fs::read_dir(at).unwrap() {
                visit(root, &entry.unwrap().path(), image);
            }
        }
    }
    let mut image = BTreeMap::new();
    visit(root, root, &mut image);
    image
}

fn row_image(db: &Connection, table: &str) -> Vec<Vec<rusqlite::types::Value>> {
    db.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
        .unwrap()
        .query_map([], |row| {
            (0..row.as_ref().column_count())
                .map(|i| row.get(i))
                .collect()
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

pub(super) fn registry_image(
    db: &Connection,
) -> BTreeMap<String, Vec<Vec<rusqlite::types::Value>>> {
    let tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut image = BTreeMap::new();
    image.insert("sqlite_master".into(), row_image(db, "sqlite_master"));
    for table in tables {
        image.insert(table.clone(), row_image(db, &table));
    }
    image.insert(
        "schema_version".into(),
        vec![vec![rusqlite::types::Value::Integer(
            db.query_row("PRAGMA schema_version", [], |r| r.get(0))
                .unwrap(),
        )]],
    );
    image
}

fn assert_inventory_refusal(
    f: &Fixture,
    request: &RetireContextRequest,
    result: &ContextRetirementOutcome,
) {
    assert!(
        matches!(result, ContextRetirementOutcome::Final(report)
        if report.code==crate::results::ResultCode::RecoveryRequired
        && report.outcome==RetirementDisposition::Refused
        && report.worktree_removal==RetirementRemoval::None
        && report.branch_removal==RetirementRemoval::None
        && !report.materialization_settled),
        "expected stable no-effect inventory refusal, got {result:?}"
    );
    let flags: Vec<rusqlite::types::Value> = f.db().query_row(
        "SELECT c.initial_resources,c.phase,c.absence_basis,c.materialization_settled,c.worktree_removal,c.branch_removal,o.state,o.completed_step FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id WHERE c.operation_ulid=?1",
        [request.operation_id.to_string()], |row| (0..row.as_ref().column_count()).map(|i| row.get(i)).collect()).unwrap();
    use rusqlite::types::Value::{Integer, Null, Text};
    assert_eq!(
        flags,
        vec![
            Text("invalid".into()),
            Text("refused".into()),
            Text("none".into()),
            Integer(0),
            Text("none".into()),
            Text("none".into()),
            Text("completed".into()),
            Null
        ]
    );
    assert_eq!(
        f.service.retire_context(request.clone()).unwrap(),
        *result,
        "accepted no-effect refusal must replay stably"
    );
}

struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
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
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        Self {
            root,
            data,
            service,
        }
    }

    fn db(&self) -> Connection {
        Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }

    /// Inert registry setup for exercising the REAL claimed local refresh seam;
    /// this is not evidence that selection/D14/producer authority ran.
    fn inert_absent_handoff(&self, operation: OperationId) {
        let db = self.db();
        db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at) SELECT id,root_path,?1,'retire_context',?2,'authoritative_observed','context_retirement_absent',1 FROM repositories", params![operation.to_string(),format!("authoring-context-v1/ticket/{ITEM}/Retire")]).unwrap();
        db.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) SELECT operation_ulid,id,repository_id,1,'ticket',?2,'absent',1,0,'absent',1,1,'index_pending','none','none','initial' FROM operation_records WHERE operation_ulid=?1", params![operation.to_string(),ITEM]).unwrap();
        db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) SELECT operation_ulid,1,repository_id,'active' FROM context_retirement_records WHERE operation_ulid=?1", [operation.to_string()]).unwrap();
    }

    fn inert_selected(&self, operation: OperationId) {
        let db = self.db();
        db.execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT id,root_path,?1,'retire_context',?2,'created',1 FROM repositories", params![operation.to_string(),format!("authoring-context-v1/ticket/{ITEM}/Retire")]).unwrap();
        db.execute("INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) SELECT operation_ulid,id,repository_id,1,'ticket',?2,'absent',1,0,'absent',0,1,'selected','none','none','none' FROM operation_records WHERE operation_ulid=?1", params![operation.to_string(),ITEM]).unwrap();
        db.execute("INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) SELECT operation_ulid,1,repository_id,'active' FROM context_retirement_records WHERE operation_ulid=?1", [operation.to_string()]).unwrap();
    }

    fn materialization(&self, step: &str) -> (String, String) {
        let owner = OperationId::new().to_string();
        let oid = git2::Repository::open(self.root.path())
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap()
            .to_string();
        let db = self.db();
        db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind,poll_completed_at) SELECT id,?1,0,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','manual','completed','before_discovery','completed',1,1,'update',1 FROM repositories", [&owner]).unwrap();
        db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,?4 FROM repositories", params![ITEM,owner,oid,step]).unwrap();
        (owner, oid)
    }
}

#[test]
fn retirement_claimed_refresh_cannot_release_blocker_without_final_result() {
    let f = Fixture::new();
    let operation = OperationId::new();
    f.inert_absent_handoff(operation);
    let refreshed = f
        .service
        .refresh_repository(RefreshRepositoryRequest {
            root: f.root.path().into(),
            operation_id: operation,
        })
        .unwrap();
    assert!(matches!(refreshed, RefreshOutcome::Refreshed { .. }));
    let (state, phase, result): (String, String, Option<String>) = f.db().query_row("SELECT o.state,c.phase,c.final_result FROM operation_records o JOIN context_retirement_records c ON c.local_record_id=o.id WHERE o.operation_ulid=?1", [operation.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(state, "completed");
    assert_eq!(
        phase, "complete",
        "claimed refresh must settle the companion in the blocker-release transaction"
    );
    assert!(
        result.is_some(),
        "completed journal must never outrun the durable retirement result"
    );
    assert!(RepositoryService::open_at(f.data.path()).is_ok());
}

#[test]
fn retirement_all_absent_is_local_only_and_completed_replay_does_not_inspect_recreation() {
    let f = Fixture::new();
    let operation = OperationId::new();
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: operation,
    };
    let first = f.service.retire_context(request.clone()).unwrap();
    let ContextRetirementOutcome::Final(ref report) = first else {
        panic!("absence should complete: {first:?}");
    };
    assert_eq!(report.code, crate::results::ResultCode::AlreadyApplied);
    assert_eq!(report.outcome, RetirementDisposition::AlreadyAbsent);
    assert_eq!(report.worktree_removal, RetirementRemoval::None);
    assert_eq!(report.branch_removal, RetirementRemoval::None);
    let db = f.db();
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT completed_step FROM operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "context_retirement_absent"
    );
    assert!(
        !LocalStep::parse("context_retirement_absent")
            .unwrap()
            .is_checkpoint()
    );
    // A hostile new path at the deterministic location must not be inspected by
    // a completed replay, let alone repaired, adopted or removed.
    let worktree = f.root.path().join(".manyhands/worktrees").join(ITEM);
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::write(worktree.join("user-content"), b"new instance").unwrap();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(reopened.retire_context(request.clone()).unwrap(), first);
    assert_eq!(
        std::fs::read(worktree.join("user-content")).unwrap(),
        b"new instance"
    );
    let mut mismatch = request;
    mismatch.kind = AuthoringKind::Document;
    assert!(
        reopened.retire_context(mismatch).is_err(),
        "terminal results remain bound to exact kind/item/root"
    );
}

#[test]
fn retirement_absence_index_retry_does_not_reselect_or_touch_new_worktree() {
    for event in [
        "UPDATE OF final_result ON context_retirement_records",
        "UPDATE OF phase ON context_retirement_attempts WHEN NEW.phase='ended'",
        "UPDATE OF state ON operation_records WHEN NEW.state='completed' AND OLD.action='retire_context'",
        "UPDATE OF refresh_required ON repositories WHEN NEW.refresh_required=0",
    ] {
        let f = Fixture::new();
        let operation = OperationId::new();
        let request = RetireContextRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            operation_id: operation,
        };
        f.db().execute_batch(&format!("CREATE TRIGGER fail_retirement_result BEFORE {event} BEGIN SELECT RAISE(ABORT,'injected finalization failure'); END;")).unwrap();
        assert_eq!(
            f.service.retire_context(request.clone()).unwrap(),
            ContextRetirementOutcome::IndexPending
        );
        let db = f.db();
        let state: (String,String,Option<String>) = db.query_row("SELECT o.state,c.phase,c.final_result FROM operation_records o JOIN context_retirement_records c ON c.local_record_id=o.id WHERE o.operation_ulid=?1", [operation.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_ne!(state.0, "completed");
        assert_eq!(state.1, "index_pending");
        assert!(state.2.is_none());
        assert!(
            db.query_row("SELECT refresh_required FROM repositories", [], |r| r
                .get::<_, bool>(0))
                .unwrap()
        );
        assert_eq!(
            db.query_row("SELECT phase FROM context_retirement_attempts", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "active"
        );
        db.execute_batch("DROP TRIGGER fail_retirement_result")
            .unwrap();
        let git = git2::Repository::open(f.root.path()).unwrap();
        let tip = git.head().unwrap().peel_to_commit().unwrap();
        let branch = format!("manyhands/ticket/{ITEM}");
        git.branch(&branch, &tip, false).unwrap();
        let worktree = f.root.path().join(".manyhands/worktrees").join(ITEM);
        std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
        let mut opts = git2::WorktreeAddOptions::new();
        let reference = git.find_reference(&format!("refs/heads/{branch}")).unwrap();
        opts.reference(Some(&reference));
        git.worktree(ITEM, &worktree, Some(&opts)).unwrap();
        std::fs::write(worktree.join("user-content"), b"recreated").unwrap();
        let new_intent = f.materialization("complete");
        let resumed = f.service.retire_context(request).unwrap();
        assert!(
            matches!(resumed, ContextRetirementOutcome::Final(_)),
            "index-only retry: {resumed:?}"
        );
        assert!(git.find_reference(&format!("refs/heads/{branch}")).is_ok());
        assert_eq!(
            std::fs::read(worktree.join("user-content")).unwrap(),
            b"recreated"
        );
        assert_eq!(
            db.query_row(
                "SELECT operation_ulid,target_oid FROM remote_poll_materializations",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            )
            .unwrap(),
            new_intent,
            "index-only continuation must not consume newly provisioned bookkeeping either"
        );
    }
}

#[test]
fn retirement_d14_exact_completed_intent_settles_atomically_and_incomplete_refusal_is_stable() {
    for step in ["complete", "intended", "branch_created"] {
        let f = Fixture::new();
        let (owner, oid) = f.materialization(step);
        let request = RetireContextRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            operation_id: OperationId::new(),
        };
        let first = f.service.retire_context(request.clone()).unwrap();
        let ContextRetirementOutcome::Final(ref report) = first else {
            panic!("{step}: {first:?}");
        };
        let db = f.db();
        if step == "complete" {
            assert_eq!(report.outcome, RetirementDisposition::AlreadyAbsent);
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM remote_poll_materializations",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            let captured: (String,String,String,bool) = db.query_row("SELECT materialization_capture,materialization_operation_ulid,materialization_oid,materialization_settled FROM context_retirement_records", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
            assert_eq!(captured, ("complete".into(), owner, oid, true));
        } else {
            assert_eq!(report.code, crate::results::ResultCode::RecoveryRequired);
            assert_eq!(report.outcome, RetirementDisposition::Refused);
            assert_eq!(
                db.query_row("SELECT step FROM remote_poll_materializations", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
                step
            );
            db.execute(
                "UPDATE remote_poll_materializations SET step='complete'",
                [],
            )
            .unwrap();
            assert_eq!(
                f.service.retire_context(request).unwrap(),
                first,
                "a repaired intent needs a new operation; the old refusal never reselects"
            );
        }
        assert!(RepositoryService::open_at(f.data.path()).is_ok());
    }
}

#[test]
fn retirement_d14_receipt_failure_rolls_back_claim_and_consumption_together() {
    let f = Fixture::new();
    let (owner, oid) = f.materialization("complete");
    let db = f.db();
    db.execute_batch("CREATE TRIGGER fail_absence_settlement BEFORE UPDATE OF materialization_settled ON context_retirement_records BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END;").unwrap();
    let operation = OperationId::new();
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: operation,
    };
    assert!(f.service.retire_context(request.clone()).is_err());
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT operation_ulid,target_oid FROM remote_poll_materializations",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        )
        .unwrap(),
        (owner, oid)
    );
    db.execute_batch("DROP TRIGGER fail_absence_settlement")
        .unwrap();
    assert!(matches!(
        f.service.retire_context(request).unwrap(),
        ContextRetirementOutcome::Final(_)
    ));
}

#[test]
fn retirement_d14_replacement_before_absence_receipt_is_not_adopted() {
    let f = Fixture::new();
    let original = f.materialization("complete");
    let db = f.db();
    db.execute_batch("CREATE TRIGGER replace_materialization_after_consume AFTER DELETE ON remote_poll_materializations BEGIN
      INSERT INTO remote_poll_materializations(repository_id,kind,item_id,operation_ulid,target_oid,step) VALUES(OLD.repository_id,OLD.kind,OLD.item_id,OLD.operation_ulid,OLD.target_oid,'intended'); END;").unwrap();
    let operation = OperationId::new();
    let result = f.service.retire_context(RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: operation,
    });
    assert!(
        result.is_err(),
        "replaced materialization must refuse before claiming the absence receipt: {result:?}"
    );
    assert_eq!(
        db.query_row(
            "SELECT operation_ulid,target_oid,step FROM remote_poll_materializations",
            [],
            |r| Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?
            ))
        )
        .unwrap(),
        (original.0, original.1, "complete".into())
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn retirement_mixed_initial_resources_refuse_stably_without_adopting_user_content() {
    let f = Fixture::new();
    let worktree = f.root.path().join(".manyhands/worktrees").join(ITEM);
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::write(worktree.join("user-content"), b"keep").unwrap();
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request.clone());
    assert!(
        matches!(result, Ok(ContextRetirementOutcome::Final(ref report)) if report.code==crate::results::ResultCode::RecoveryRequired && report.outcome==RetirementDisposition::Refused),
        "mixed state needs a stable no-effect refusal, not a retryable unbound error: {result:?}"
    );
    std::fs::remove_file(worktree.join("user-content")).unwrap();
    std::fs::remove_dir(&worktree).unwrap();
    assert_eq!(
        f.service.retire_context(request.clone()).unwrap(),
        result.unwrap()
    );
    let new = RetireContextRequest {
        operation_id: OperationId::new(),
        ..request
    };
    assert!(
        matches!(f.service.retire_context(new).unwrap(), ContextRetirementOutcome::Final(ref report) if report.outcome==RetirementDisposition::AlreadyAbsent)
    );
}

#[test]
fn retirement_ambiguous_materialization_cannot_be_consumed_by_kind_filter() {
    let f = Fixture::new();
    f.materialization("complete");
    f.db().execute("INSERT INTO remote_poll_materializations SELECT repository_id,'document',item_id,operation_ulid,target_oid,step FROM remote_poll_materializations WHERE kind='ticket'", []).unwrap();
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request);
    assert!(
        matches!(result, Ok(ContextRetirementOutcome::Final(ref report)) if report.outcome==RetirementDisposition::Refused),
        "ambiguous old provisioning is a stable refusal: {result:?}"
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
}

#[test]
fn retirement_absence_does_not_consume_a_materialization_with_a_foreign_owner() {
    let f = Fixture::new();
    let db = f.db();
    let owner = OperationId::new().to_string();
    db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind) SELECT id,?1,0,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','manual','completed','after_batch_commit','completed',1,1,'observe' FROM repositories", [&owner]).unwrap();
    db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories", params![ITEM,owner,"1".repeat(40)]).unwrap();
    let result = f.service.retire_context(RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    });
    assert!(
        matches!(result, Ok(ContextRetirementOutcome::Final(ref report)) if report.outcome==RetirementDisposition::Refused),
        "CHECK/FK-valid but wrong-owner intent must not become absence authority: {result:?}"
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_poll_materializations",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn retirement_requires_registration_before_claiming_any_absence() {
    let f = Fixture::new();
    f.db().execute("DELETE FROM repositories", []).unwrap();
    let result = f.service.retire_context(RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    });
    assert!(
        matches!(result, Err(ref error) if error.kind==RepositoryErrorKind::RepositoryNotRegistered)
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn retirement_absent_claim_cannot_bypass_another_operations_index_debt() {
    let f = Fixture::new();
    let pending = OperationId::new();
    f.inert_absent_handoff(pending);
    let foreign = OperationId::new();
    let result = f.service.retire_context(RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: foreign,
    });
    assert!(matches!(result, Err(ref error) if error.kind==RepositoryErrorKind::RecoveryRequired));
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [foreign.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT state FROM operation_records WHERE operation_ulid=?1",
                [pending.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "authoritative_observed"
    );
}

#[cfg(unix)]
#[test]
fn retirement_dangling_or_redirected_ancestor_is_not_initial_absence() {
    for dangling in [false, true] {
        let f = Fixture::new();
        let external = tempfile::tempdir().unwrap();
        let target = if dangling {
            external.path().join("missing")
        } else {
            external.path().into()
        };
        std::os::unix::fs::symlink(target, f.root.path().join(".manyhands/worktrees")).unwrap();
        let request = RetireContextRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            operation_id: OperationId::new(),
        };
        let result = f.service.retire_context(request).unwrap();
        assert!(
            matches!(result, ContextRetirementOutcome::Final(ref report) if report.outcome==RetirementDisposition::Refused)
        );
        assert_eq!(std::fs::read_dir(external.path()).unwrap().count(), 0);
        assert!(
            std::fs::symlink_metadata(f.root.path().join(".manyhands/worktrees"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}

#[test]
fn retirement_specfix_s1_alternate_registration_with_deleted_branch_is_not_absent() {
    let f = Fixture::new();
    f.materialization("complete");
    let git = git2::Repository::open(f.root.path()).unwrap();
    let branch = format!("manyhands/ticket/{ITEM}");
    let tip = git.head().unwrap().peel_to_commit().unwrap();
    let reference = git.branch(&branch, &tip, false).unwrap().into_reference();
    let external = tempfile::tempdir().unwrap();
    let worktree = external.path().join("actual-context");
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    git.worktree("alternate-admin-name", &worktree, Some(&options))
        .unwrap();
    std::fs::write(worktree.join("user-content"), b"preserve external context").unwrap();
    std::fs::remove_file(git.commondir().join("refs/heads").join(&branch)).unwrap();
    assert!(
        !f.root
            .path()
            .join(".manyhands/worktrees")
            .join(ITEM)
            .exists()
    );
    assert!(!git.commondir().join("worktrees").join(ITEM).exists());
    let admin_before = disk_image(&git.commondir().join("worktrees"));
    let worktree_before = disk_image(&worktree);
    let db = f.db();
    let materialization_before = row_image(&db, "remote_poll_materializations");
    let repository_before = row_image(&db, "repositories");
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request.clone()).unwrap();
    let materialization_after = row_image(&db, "remote_poll_materializations");
    assert_eq!(
        materialization_after, materialization_before,
        "alternate checkout must not consume D14; outcome={result:?}"
    );
    assert_eq!(disk_image(&git.commondir().join("worktrees")), admin_before);
    assert_eq!(disk_image(&worktree), worktree_before);
    assert_eq!(row_image(&db, "repositories"), repository_before);
    assert_inventory_refusal(&f, &request, &result);
    git.reference(
        &format!("refs/heads/{branch}"),
        tip.id(),
        false,
        "fixture repair",
    )
    .unwrap();
    assert_eq!(f.service.retire_context(request).unwrap(), result);
}

#[test]
fn retirement_specfix_s1_incomplete_unreadable_or_foreign_inventory_refuses() {
    for defect in ["incomplete", "unreadable_head", "foreign_commondir"] {
        let f = Fixture::new();
        f.materialization("complete");
        let git = git2::Repository::open(f.root.path()).unwrap();
        let admin = git.commondir().join("worktrees/unlisted");
        std::fs::create_dir_all(&admin).unwrap();
        let external = tempfile::tempdir().unwrap();
        if defect != "incomplete" {
            let reference = git
                .branch(
                    "unrelated",
                    &git.head().unwrap().peel_to_commit().unwrap(),
                    false,
                )
                .unwrap()
                .into_reference();
            // Remove just this empty fixture directory before libgit2 installs
            // an actual differently named registration there.
            std::fs::remove_dir(&admin).unwrap();
            let mut options = git2::WorktreeAddOptions::new();
            options.reference(Some(&reference));
            git.worktree(
                "unlisted",
                &external.path().join("checkout"),
                Some(&options),
            )
            .unwrap();
            if defect == "unreadable_head" {
                std::fs::write(admin.join("HEAD"), [0xff]).unwrap();
            } else {
                std::fs::write(admin.join("commondir"), external.path().to_str().unwrap()).unwrap();
            }
        }
        let admin_before = disk_image(&admin);
        let external_before = disk_image(external.path());
        let db = f.db();
        let materialization_before = row_image(&db, "remote_poll_materializations");
        let repository_before = row_image(&db, "repositories");
        let request = RetireContextRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            operation_id: OperationId::new(),
        };
        let result = f.service.retire_context(request.clone()).unwrap();
        assert_eq!(
            row_image(&db, "remote_poll_materializations"),
            materialization_before,
            "{defect}: {result:?}"
        );
        assert_eq!(row_image(&db, "repositories"), repository_before);
        assert_eq!(disk_image(&admin), admin_before);
        assert_eq!(disk_image(external.path()), external_before);
        assert_inventory_refusal(&f, &request, &result);
    }
}

#[test]
fn retirement_specfix_s1_complete_unrelated_registration_does_not_block_absence() {
    let f = Fixture::new();
    let git = git2::Repository::open(f.root.path()).unwrap();
    let reference = git
        .branch(
            "unrelated",
            &git.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap()
        .into_reference();
    let external = tempfile::tempdir().unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    git.worktree(
        "unrelated-registration",
        &external.path().join("checkout"),
        Some(&options),
    )
    .unwrap();
    let admin_before = disk_image(&git.commondir().join("worktrees"));
    let external_before = disk_image(external.path());
    let result = f
        .service
        .retire_context(RetireContextRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert!(
        matches!(result, ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::AlreadyAbsent)
    );
    assert_eq!(disk_image(&git.commondir().join("worktrees")), admin_before);
    assert_eq!(disk_image(external.path()), external_before);
}

fn assert_backlink_refusal(corrupt: impl FnOnce(&git2::Repository, &Path, &Path, &Path)) {
    let f = Fixture::new();
    f.materialization("complete");
    let git = git2::Repository::open(f.root.path()).unwrap();
    let reference = git
        .branch(
            "unrelated",
            &git.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap()
        .into_reference();
    let external = tempfile::tempdir().unwrap();
    let worktree = external.path().join("checkout");
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    git.worktree("unrelated-registration", &worktree, Some(&options))
        .unwrap();
    let admin = git.commondir().join("worktrees/unrelated-registration");
    let original_gitfile = std::fs::read(worktree.join(".git")).unwrap();
    let original_commondir = std::fs::read(admin.join("commondir")).unwrap();
    let original_gitdir = std::fs::read(admin.join("gitdir")).unwrap();
    std::fs::write(worktree.join("sentinel"), b"preserve unrelated checkout\n").unwrap();
    corrupt(&git, &admin, &worktree, external.path());
    assert!(!git.commondir().join("worktrees").join(ITEM).exists());
    assert!(
        !f.root
            .path()
            .join(".manyhands/worktrees")
            .join(ITEM)
            .exists()
    );
    assert!(
        git.find_reference(&format!("refs/heads/manyhands/ticket/{ITEM}"))
            .is_err()
    );
    // The old inventory proves only equal canonical paths, not the backlink.
    assert!(
        !crate::repository::cleanup::branch_is_checked_out(
            &git,
            &format!("refs/heads/manyhands/ticket/{ITEM}")
        )
        .unwrap()
    );
    let admin_before = disk_image(git.commondir());
    let external_before = disk_image(external.path());
    let db = f.db();
    let intent_before = row_image(&db, "remote_poll_materializations");
    let repository_before = row_image(&db, "repositories");
    let remote_before = row_image(&db, "remote_operation_records");
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request.clone()).unwrap();
    assert_eq!(
        row_image(&db, "remote_poll_materializations"),
        intent_before,
        "invalid unrelated checkout backlink must not consume D14: {result:?}"
    );
    assert_inventory_refusal(&f, &request, &result);
    let local_before_replay = row_image(&db, "operation_records");
    let retirement_before_replay = row_image(&db, "context_retirement_records");
    let attempts_before_replay = row_image(&db, "context_retirement_attempts");
    assert_eq!(f.service.retire_context(request.clone()).unwrap(), result);
    assert_eq!(row_image(&db, "operation_records"), local_before_replay);
    assert_eq!(
        row_image(&db, "context_retirement_records"),
        retirement_before_replay
    );
    assert_eq!(
        row_image(&db, "context_retirement_attempts"),
        attempts_before_replay
    );
    assert_eq!(
        row_image(&db, "remote_poll_materializations"),
        intent_before
    );
    assert_eq!(row_image(&db, "repositories"), repository_before);
    assert_eq!(row_image(&db, "remote_operation_records"), remote_before);
    assert_eq!(disk_image(git.commondir()), admin_before);
    assert_eq!(disk_image(external.path()), external_before);
    if std::fs::symlink_metadata(worktree.join(".git"))
        .unwrap()
        .is_file()
    {
        // Fixture-only repair after proving preservation. A stored refusal must
        // remain stable even when the gitfile and admin links become valid.
        std::fs::write(worktree.join(".git"), original_gitfile).unwrap();
        std::fs::write(admin.join("commondir"), original_commondir).unwrap();
        std::fs::write(admin.join("gitdir"), original_gitdir).unwrap();
        assert_eq!(f.service.retire_context(request).unwrap(), result);
        assert_eq!(
            row_image(&db, "remote_poll_materializations"),
            intent_before
        );
        assert_eq!(
            row_image(&db, "context_retirement_records"),
            retirement_before_replay
        );
        assert_eq!(row_image(&db, "operation_records"), local_before_replay);
        assert_eq!(row_image(&db, "repositories"), repository_before);
    }
}

#[test]
fn retirement_specfix_s1_backlink_foreign_gitfile_contents_refuse() {
    for same_common in [false, true] {
        assert_backlink_refusal(|git, _, worktree, external| {
            let foreign = if same_common {
                let reference = git
                    .branch(
                        "another",
                        &git.head().unwrap().peel_to_commit().unwrap(),
                        false,
                    )
                    .unwrap()
                    .into_reference();
                let mut options = git2::WorktreeAddOptions::new();
                options.reference(Some(&reference));
                git.worktree(
                    "another-registration",
                    &external.join("another"),
                    Some(&options),
                )
                .unwrap();
                git.commondir().join("worktrees/another-registration")
            } else {
                let other = git2::Repository::init(external.join("foreign")).unwrap();
                other.path().to_owned()
            };
            std::fs::write(
                worktree.join(".git"),
                format!("gitdir: {}", remote::git_metadata_link_text(&foreign)),
            )
            .unwrap();
        });
    }
}

#[test]
fn retirement_specfix_s1_backlink_malformed_gitfile_refuses() {
    for defect in [
        "prefix",
        "empty",
        "extra_line",
        "nul",
        "invalid_utf8",
        "directory",
    ] {
        assert_backlink_refusal(|_, admin, worktree, _| {
            let path = worktree.join(".git");
            let target = remote::git_metadata_link_text(admin);
            let bytes = match defect {
                "prefix" => format!("not-gitdir: {target}").into_bytes(),
                "empty" => b"gitdir: \n".to_vec(),
                "extra_line" => format!("gitdir: {target}gitdir: {target}").into_bytes(),
                "nul" => format!("gitdir: {}\0\n", target.trim_end()).into_bytes(),
                "invalid_utf8" => vec![0xff],
                "directory" => {
                    std::fs::remove_file(&path).unwrap();
                    std::fs::create_dir(&path).unwrap();
                    return;
                }
                _ => unreachable!(),
            };
            std::fs::write(path, bytes).unwrap();
        });
    }
}

#[cfg(unix)]
#[test]
fn retirement_specfix_s1_backlink_redirected_gitfile_refuses() {
    assert_backlink_refusal(|_, _, worktree, external| {
        let path = worktree.join(".git");
        let saved = external.join("saved-gitfile");
        std::fs::rename(&path, &saved).unwrap();
        std::os::unix::fs::symlink(&saved, &path).unwrap();
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_link(&path).unwrap(), saved);
    });
}

#[cfg(any(unix, windows))]
#[test]
fn retirement_specfix_s1_backlink_redirected_gitfile_ancestor_refuses() {
    assert_backlink_refusal(|_, admin, worktree, external| {
        let alias = external.join("checkout-alias");
        #[cfg(unix)]
        std::os::unix::fs::symlink(worktree, &alias).unwrap();
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert!(
                std::process::Command::new("cmd.exe")
                    .args(["/C", "mklink", "/J"])
                    .arg(&alias)
                    .arg(worktree)
                    .status()
                    .unwrap()
                    .success()
            );
            assert_ne!(
                std::fs::symlink_metadata(&alias).unwrap().file_attributes() & 0x400,
                0
            );
        }
        std::fs::write(
            admin.join("gitdir"),
            remote::git_metadata_link_text(&alias.join(".git")),
        )
        .unwrap();
        assert!(std::fs::metadata(alias.join(".git")).unwrap().is_file());
    });
}

#[cfg(windows)]
#[test]
fn retirement_specfix_s1_backlink_gitfile_junction_refuses() {
    use std::os::windows::fs::MetadataExt;
    assert_backlink_refusal(|_, _, worktree, external| {
        let path = worktree.join(".git");
        let destination = external.join("junction-destination");
        std::fs::create_dir(&destination).unwrap();
        std::fs::rename(&path, destination.join("saved-gitfile")).unwrap();
        assert!(
            std::process::Command::new("cmd.exe")
                .args(["/C", "mklink", "/J"])
                .arg(&path)
                .arg(&destination)
                .status()
                .unwrap()
                .success()
        );
        assert_ne!(
            std::fs::symlink_metadata(&path).unwrap().file_attributes() & 0x400,
            0
        );
    });
}

#[test]
fn retirement_specfix_s1_backlink_relative_gitfile_allows_absence() {
    let f = Fixture::new();
    f.materialization("complete");
    let git = git2::Repository::open(f.root.path()).unwrap();
    let reference = git
        .branch(
            "unrelated",
            &git.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap()
        .into_reference();
    let worktree = f.root.path().join("unrelated-checkout");
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    git.worktree("unrelated-registration", &worktree, Some(&options))
        .unwrap();
    std::fs::write(
        worktree.join(".git"),
        b"gitdir: ../.git/worktrees/unrelated-registration\r\n",
    )
    .unwrap();
    let admin_before = disk_image(git.commondir());
    let worktree_before = disk_image(&worktree);
    let result = f
        .service
        .retire_context(RetireContextRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert!(
        matches!(result, ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::AlreadyAbsent && report.materialization_settled)
    );
    assert!(row_image(&f.db(), "remote_poll_materializations").is_empty());
    assert_eq!(disk_image(git.commondir()), admin_before);
    assert_eq!(disk_image(&worktree), worktree_before);
}

#[cfg(any(unix, windows))]
fn assert_redirected_commondir_refusal(ancestor: bool) {
    assert_backlink_refusal(|git, admin, _, external| {
        #[cfg(unix)]
        let alias = admin.join("common-alias");
        // Keep a Windows junction outside the admin image to avoid traversing a
        // recursive common-dir junction in snapshot implementations.
        #[cfg(windows)]
        let alias = external.join("common-alias");
        #[cfg(unix)]
        let _ = external;
        let common = git.commondir();
        let destination = if ancestor {
            common.parent().unwrap()
        } else {
            common
        };
        #[cfg(unix)]
        std::os::unix::fs::symlink(destination, &alias).unwrap();
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert!(
                std::process::Command::new("cmd.exe")
                    .args(["/C", "mklink", "/J"])
                    .arg(&alias)
                    .arg(destination)
                    .status()
                    .unwrap()
                    .success()
            );
            assert_ne!(
                std::fs::symlink_metadata(&alias).unwrap().file_attributes() & 0x400,
                0
            );
        }
        #[cfg(unix)]
        let target = PathBuf::from("common-alias");
        #[cfg(windows)]
        let target = alias.clone();
        let target = if ancestor {
            target.join(common.file_name().unwrap())
        } else {
            target
        };
        std::fs::write(
            admin.join("commondir"),
            remote::git_metadata_link_text(&target),
        )
        .unwrap();
        assert_eq!(
            admin.join(&target).canonicalize().unwrap(),
            common.canonicalize().unwrap()
        );
        assert_eq!(
            std::fs::read_link(&alias).unwrap().canonicalize().unwrap(),
            destination.canonicalize().unwrap()
        );
    });
}

#[cfg(any(unix, windows))]
#[test]
fn retirement_quality_q1_redirected_commondir_target_refuses() {
    assert_redirected_commondir_refusal(false);
}

#[cfg(any(unix, windows))]
#[test]
fn retirement_quality_q1_redirected_commondir_ancestor_refuses() {
    assert_redirected_commondir_refusal(true);
}

#[test]
fn retirement_quality_q1_plain_relative_commondir_allows_absence() {
    let f = Fixture::new();
    f.materialization("complete");
    let git = git2::Repository::open(f.root.path()).unwrap();
    let reference = git
        .branch(
            "unrelated",
            &git.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap()
        .into_reference();
    let external = tempfile::tempdir().unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    git.worktree(
        "unrelated-registration",
        &external.path().join("checkout"),
        Some(&options),
    )
    .unwrap();
    std::fs::write(
        git.commondir()
            .join("worktrees/unrelated-registration/commondir"),
        b"../../worktrees/..\n",
    )
    .unwrap();
    let admin_before = disk_image(git.commondir());
    let external_before = disk_image(external.path());
    let result = f
        .service
        .retire_context(RetireContextRequest {
            root: f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert!(
        matches!(result, ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::AlreadyAbsent && report.materialization_settled)
    );
    assert!(row_image(&f.db(), "remote_poll_materializations").is_empty());
    assert_eq!(disk_image(git.commondir()), admin_before);
    assert_eq!(disk_image(external.path()), external_before);
}

fn assert_foreign_step_matrix(
    f: &Fixture,
    operation: OperationId,
    replay: Option<RetireContextRequest>,
) {
    assert!(
        RepositoryService::open_at(f.data.path()).is_ok(),
        "uncorrupted fixture must pass startup audit"
    );
    let db = f.db();
    let steps = LocalStep::ALL.into_iter().map(|s| s.as_str()).chain([
        "context_retirement_preserved",
        "unregistered_foreign_step",
        "",
    ]);
    let mut accepted = Vec::new();
    for step in steps {
        // Ordinary SQL only: no constraint, trigger or schema bypass.
        assert_eq!(
            db.execute(
                "UPDATE operation_records SET completed_step=?1 WHERE operation_ulid=?2",
                params![step, operation.to_string()]
            )
            .unwrap(),
            1
        );
        let before = registry_image(&db);
        if !matches!(RepositoryService::open_at(f.data.path()), Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        {
            accepted.push(format!("startup:{step}"));
        }
        if let Some(request) = &replay
            && !matches!(f.service.retire_context(request.clone()), Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        {
            accepted.push(format!("replay:{step}"));
        }
        assert_eq!(
            registry_image(&db),
            before,
            "startup/replay must retain every row and schema object for {step:?}"
        );
    }
    assert!(
        accepted.is_empty(),
        "contradictory retirement step authority accepted: {accepted:?}"
    );
}

#[test]
fn retirement_quality_q2_no_effect_refusal_rejects_other_step_authority() {
    let f = Fixture::new();
    f.materialization("complete");
    std::fs::create_dir_all(f.root.path().join(".manyhands/worktrees").join(ITEM)).unwrap();
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request.clone()).unwrap();
    assert_inventory_refusal(&f, &request, &result);
    assert_foreign_step_matrix(&f, request.operation_id, Some(request));
}

#[test]
fn retirement_quality_q2_selected_rejects_other_step_authority() {
    let f = Fixture::new();
    let operation = OperationId::new();
    f.inert_selected(operation);
    assert_foreign_step_matrix(&f, operation, None);
}

#[test]
fn retirement_quality_q2_absent_handoff_and_complete_keep_noncheckpoint_step() {
    for complete in [false, true] {
        let f = Fixture::new();
        let operation = OperationId::new();
        f.inert_absent_handoff(operation);
        if complete {
            assert!(matches!(
                f.service
                    .retire_context(RetireContextRequest {
                        root: f.root.path().into(),
                        kind: AuthoringKind::Ticket,
                        item_id: ITEM.parse().unwrap(),
                        operation_id: operation
                    })
                    .unwrap(),
                ContextRetirementOutcome::Final(_)
            ));
        }
        assert!(RepositoryService::open_at(f.data.path()).is_ok());
        let db = f.db();
        let step: String = db
            .query_row(
                "SELECT completed_step FROM operation_records WHERE operation_ulid=?1",
                [operation.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(step, "context_retirement_absent");
        assert!(!LocalStep::ContextRetirementAbsent.is_checkpoint());
        assert!(!LocalStep::ContextBranchReleased.is_checkpoint());
        let before = registry_image(&db);
        assert!(RepositoryService::open_at(f.data.path()).is_ok());
        assert_eq!(registry_image(&db), before);
    }
}

#[test]
fn retirement_quality_q2_preserving_placeholder_requires_unimplemented_correlation() {
    let mut accepted = Vec::new();
    for terminal in [false, true] {
        let f = Fixture::new();
        let operation = OperationId::new();
        f.inert_selected(operation);
        let db = f.db();
        assert!(RepositoryService::open_at(f.data.path()).is_ok());
        db.execute("UPDATE operation_records SET completed_step='context_retirement_preserved' WHERE operation_ulid=?1", [operation.to_string()]).unwrap();
        if terminal {
            db.execute_batch("UPDATE context_retirement_records SET phase='abandoned',final_code='external_change',final_result=json_object('version',1,'code','external_change','outcome','preserved','worktree_removal','none','branch_removal','none','materialization_settled',json('false')); UPDATE context_retirement_attempts SET phase='ended'; UPDATE operation_records SET state='completed' WHERE action='retire_context';").unwrap();
        } else {
            db.execute(
                "UPDATE context_retirement_records SET phase='preserving'",
                [],
            )
            .unwrap();
        }
        let before = registry_image(&db);
        if !matches!(RepositoryService::open_at(f.data.path()), Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
        {
            accepted.push(terminal);
        }
        assert_eq!(
            registry_image(&db),
            before,
            "incomplete preservation authority must not be repaired or erased"
        );
    }
    assert!(
        accepted.is_empty(),
        "placeholder preserving authority accepted (terminal): {accepted:?}"
    );
}

#[cfg(unix)]
#[test]
fn retirement_specfix_s2_dangling_loose_ref_slot_is_not_absence() {
    let f = Fixture::new();
    f.materialization("complete");
    let git = git2::Repository::open(f.root.path()).unwrap();
    let slot = git
        .commondir()
        .join(format!("refs/heads/manyhands/ticket/{ITEM}"));
    std::fs::create_dir_all(slot.parent().unwrap()).unwrap();
    let target = PathBuf::from("missing-reference-target");
    std::os::unix::fs::symlink(&target, &slot).unwrap();
    match git.find_reference(&format!("refs/heads/manyhands/ticket/{ITEM}")) {
        Err(error) => assert_eq!(error.code(), git2::ErrorCode::NotFound),
        Ok(_) => panic!("fixture must reproduce libgit2's dangling-ref NotFound"),
    }
    let admin_before = disk_image(git.commondir());
    let db = f.db();
    let materialization_before = row_image(&db, "remote_poll_materializations");
    let repository_before = row_image(&db, "repositories");
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request.clone()).unwrap();
    assert_eq!(
        row_image(&db, "remote_poll_materializations"),
        materialization_before,
        "dangling branch slot must not consume D14: {result:?}"
    );
    assert_eq!(disk_image(git.commondir()), admin_before);
    assert_eq!(row_image(&db, "repositories"), repository_before);
    assert!(
        std::fs::symlink_metadata(&slot)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read_link(&slot).unwrap(), target);
    assert_inventory_refusal(&f, &request, &result);
}

#[cfg(unix)]
#[test]
fn retirement_specfix_s2_redirected_packed_storage_is_not_absence() {
    let f = Fixture::new();
    f.materialization("complete");
    let git = git2::Repository::open(f.root.path()).unwrap();
    let slot = git.commondir().join("packed-refs");
    std::os::unix::fs::symlink("missing-packed-reference-target", &slot).unwrap();
    let admin_before = disk_image(git.commondir());
    let db = f.db();
    let materialization_before = row_image(&db, "remote_poll_materializations");
    let repository_before = row_image(&db, "repositories");
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request.clone()).unwrap();
    assert_eq!(
        row_image(&db, "remote_poll_materializations"),
        materialization_before,
        "redirected packed storage must not consume D14: {result:?}"
    );
    assert_eq!(disk_image(git.commondir()), admin_before);
    assert_eq!(row_image(&db, "repositories"), repository_before);
    assert_inventory_refusal(&f, &request, &result);
}

#[test]
fn retirement_specfix_s2_nonregular_loose_ref_slot_is_not_absence() {
    let f = Fixture::new();
    f.materialization("complete");
    let git = git2::Repository::open(f.root.path()).unwrap();
    std::fs::create_dir_all(
        git.commondir()
            .join(format!("refs/heads/manyhands/ticket/{ITEM}")),
    )
    .unwrap();
    let admin_before = disk_image(git.commondir());
    let db = f.db();
    let materialization_before = row_image(&db, "remote_poll_materializations");
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    let result = f.service.retire_context(request.clone()).unwrap();
    assert_eq!(
        row_image(&db, "remote_poll_materializations"),
        materialization_before,
        "nonregular branch slot must not consume D14: {result:?}"
    );
    assert_eq!(disk_image(git.commondir()), admin_before);
    assert_inventory_refusal(&f, &request, &result);
}

fn assert_retirement_preserves_remote_debt(reconciliation: bool) {
    use crate::repository::remote::{reservation::commit_observation_batch, state};
    let f = Fixture::new();
    f.materialization("complete");
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&f.service, f.root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    let target = RemoteOperationTarget::for_primary_synchronization(&plan);
    let operation = OperationId::new();
    let owner = match f
        .service
        .reserve_remote_operation(f.root.path(), operation, &target)
        .unwrap()
    {
        RemoteReservationOutcome::Reserved(owner) => owner,
        other => panic!("fixture reservation: {other:?}"),
    };
    let oid = git2::Repository::open(f.root.path())
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap();
    f.service
        .checkpoint_synchronization(
            f.root.path(),
            &owner,
            state::SynchronizationCheckpoint::FetchPrepared,
            &state::SynchronizationEvidence::default(),
        )
        .unwrap();
    f.service
        .remote_safe_point(f.root.path(), &owner, RemoteOperationSafePoint::BeforeFetch)
        .unwrap();
    let observation =
        RemoteRefObservation::from_advertisement(&plan, "refs/heads/main", oid, Some(oid)).unwrap();
    // Metadata-only synchronization fixture using the real owned checkpoint,
    // complete-batch, classification/restart/cancellation paths, not a corrupt
    // row with hand-edited index/reconciliation flags. No transport is claimed.
    commit_observation_batch(&f.service, f.root.path(), &owner, &plan, &[observation], 1).unwrap();
    let evidence = state::SynchronizationEvidence {
        expected_oid: Some(oid),
        local_oid: Some(oid),
        tracking_oid: Some(oid),
        primary_tracking_oid: Some(oid),
        push_oid: Some(oid),
        push_advertised_oid: (!reconciliation).then_some(oid),
    };
    if reconciliation {
        f.service
            .checkpoint_synchronization(
                f.root.path(),
                &owner,
                state::SynchronizationCheckpoint::PushPrepared,
                &evidence,
            )
            .unwrap();
        let restarted = match f
            .service
            .restart_remote_synchronization(f.root.path(), operation, &target)
            .unwrap()
        {
            RemoteReservationOutcome::Reserved(owner) => owner,
            other => panic!("fixture restart: {other:?}"),
        };
        f.service
            .cancel_remote_operation(f.root.path(), operation)
            .unwrap();
        assert_eq!(
            f.service
                .remote_safe_point(
                    f.root.path(),
                    &restarted,
                    RemoteOperationSafePoint::BeforeFetch
                )
                .unwrap(),
            RemoteSafePointOutcome::Cancelled
        );
    } else {
        f.service
            .checkpoint_synchronization(
                f.root.path(),
                &owner,
                state::SynchronizationCheckpoint::PushVerified,
                &evidence,
            )
            .unwrap();
        f.service
            .classify_synchronization(
                f.root.path(),
                &owner,
                state::SynchronizationAuthority::AlreadyCurrent(oid),
            )
            .unwrap();
    }
    let db = f.db();
    let flags: (String,bool,bool) = db.query_row("SELECT phase,index_pending,reconciliation_required FROM remote_operation_records WHERE operation_ulid=?1", [operation.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(
        flags,
        (
            if reconciliation {
                "cancelled"
            } else {
                "completed"
            }
            .into(),
            !reconciliation,
            reconciliation
        )
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE state!='completed'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0,
        "exercise the real remote-completed/local-not-yet-created gap"
    );
    assert!(
        RepositoryService::open_at(f.data.path()).is_ok(),
        "fixture must pass the complete startup audit"
    );
    db.execute("UPDATE repositories SET refresh_required=1", [])
        .unwrap();
    let remote_before = row_image(&db, "remote_operation_records");
    let intent_before = row_image(&db, "remote_poll_materializations");
    let repository_before = row_image(&db, "repositories");
    let request = RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    };
    for _ in 0..2 {
        let result = f.service.retire_context(request.clone());
        assert!(
            matches!(result, Err(ref error) if error.kind==RepositoryErrorKind::RecoveryRequired),
            "unrelated retained debt must block before claim, flags={flags:?}, got {result:?}; intent before={intent_before:?}, after={:?}",
            row_image(&db, "remote_poll_materializations")
        );
        assert_eq!(row_image(&db, "remote_operation_records"), remote_before);
        assert_eq!(
            row_image(&db, "remote_poll_materializations"),
            intent_before
        );
        assert_eq!(
            row_image(&db, "repositories"),
            repository_before,
            "retirement must not clear unrelated stale/index debt"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM operation_records WHERE operation_ulid=?1",
                [request.operation_id.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn retirement_specfix_s3_completed_sync_index_debt_blocks_before_claim() {
    assert_retirement_preserves_remote_debt(false);
}

#[test]
fn retirement_specfix_s3_terminal_reconciliation_debt_blocks_before_claim() {
    assert_retirement_preserves_remote_debt(true);
}

#[test]
fn retirement_specfix_s3_active_observer_still_blocks_before_claim() {
    use crate::repository::remote::state;
    let f = Fixture::new();
    f.materialization("complete");
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    state::with_transaction(&f.service, f.root.path(), |tx, id| {
        state::configure(tx, id, Some(&plan), state::ConfigurationChange::Plan)
    })
    .unwrap();
    assert!(matches!(
        f.service
            .reserve_remote_operation_with_priority(
                f.root.path(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan),
                RemoteOperationPriority::Manual
            )
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    let db = f.db();
    let remote_before = row_image(&db, "remote_operation_records");
    let intent_before = row_image(&db, "remote_poll_materializations");
    let repository_before = row_image(&db, "repositories");
    let result = f.service.retire_context(RetireContextRequest {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    });
    assert!(matches!(result, Err(ref error) if error.kind==RepositoryErrorKind::RecoveryRequired));
    assert_eq!(row_image(&db, "remote_operation_records"), remote_before);
    assert_eq!(
        row_image(&db, "remote_poll_materializations"),
        intent_before
    );
    assert_eq!(row_image(&db, "repositories"), repository_before);
    assert_eq!(
        db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
