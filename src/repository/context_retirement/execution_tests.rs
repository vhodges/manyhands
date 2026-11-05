use crate::repository::*;
use rusqlite::{Connection, params};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

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
