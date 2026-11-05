//! Standalone domain journeys, not mutation/CLI/batch retirement behavior.
use super::*;
use std::{fs, path::Path};

pub(crate) const CASES: &[ssh_harness::Case] = &[
    (
        "retirement_branch_only_fresh_fetch_retires_closed_ticket",
        fresh_absence,
    ),
    (
        "retirement_branch_only_unknown_fetch_history_is_final",
        unknown_history,
    ),
    (
        "retirement_branch_only_remote_reappearance_is_final",
        reappeared,
    ),
    (
        "retirement_branch_only_dirty_primary_item_is_final",
        dirty_primary,
    ),
    (
        "retirement_branch_only_d14_ignored_delete_stays_owed_and_recreation_survives",
        d14_ignored,
    ),
    (
        "retirement_branch_only_d14_consumes_exact_old_intent_and_keeps_new_document_intent",
        d14_complete,
    ),
    (
        "retirement_branch_only_fresh_document_keeps_other_context",
        fresh_document,
    ),
    (
        "retirement_branch_only_host_stop_is_final_for_old_id",
        host_stop,
    ),
    (
        "retirement_branch_only_cancel_during_live_advertisement_is_final",
        held_cancel,
    ),
    (
        "retirement_branch_only_stale_parent_returning_from_transport_cannot_write",
        held_fence,
    ),
    (
        "retirement_spec_s2_ref_move_before_git_is_final_no_effect",
        spec_s2_ref_move,
    ),
    (
        "retirement_spec_s2_primary_move_before_git_is_final_no_effect",
        spec_s2_primary_move,
    ),
    (
        "retirement_spec_s2_fenced_before_git_cannot_finalize_no_effect",
        spec_s2_fenced,
    ),
    (
        "retirement_spec_s1_live_intended_only_handoff_cannot_finalize",
        spec_s1_live_handoff,
    ),
    (
        "retirement_spec_s4_plain_packed_primary_remains_eligible",
        spec_s4_packed_primary,
    ),
    #[cfg(unix)]
    (
        "retirement_spec_s4_initial_primary_ref_symlink_is_refused",
        spec_s4_initial_target,
    ),
    #[cfg(unix)]
    (
        "retirement_spec_s4_final_primary_ref_symlink_is_refused",
        spec_s4_final_target,
    ),
    #[cfg(unix)]
    (
        "retirement_spec_s4_initial_primary_ancestor_symlink_is_refused",
        spec_s4_initial_ancestor,
    ),
    #[cfg(unix)]
    (
        "retirement_spec_s4_final_primary_ancestor_symlink_is_refused",
        spec_s4_final_ancestor,
    ),
    #[cfg(unix)]
    (
        "retirement_spec_s4_initial_primary_lock_slots_are_refused",
        spec_s4_initial_locks,
    ),
    #[cfg(unix)]
    (
        "retirement_spec_s4_final_primary_lock_slots_are_refused",
        spec_s4_final_locks,
    ),
];

fn setup() -> Result<World, FixtureError> {
    let w = World::plain_key()?;
    let repo = w.repo()?;
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    fixed(fs::create_dir_all(
        w.root.join(Path::new(&path).parent().unwrap()),
    ))?;
    fixed(fs::write(
        w.root.join(&path),
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\ntitle: Completed fixture\ntype: task\nstatus: open\nclosed_at: '2026-11-04T00:00:00Z'\nclosed_by: 'Fixture <fixture@example.invalid>'\n---\nRetirement fixture\n"
        ),
    ))?;
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(Path::new(&path)))?;
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let parent = fixed(fixed(repo.head())?.peel_to_commit())?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let tip = fixed(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "completed primary fixture",
        &tree,
        &[&parent],
    ))?;
    fixed(repo.reference(CONTEXT, tip, false, "branch-only fixture"))?;
    Ok(w)
}

fn observe(w: &World) -> Result<(), FixtureError> {
    fixed(w.service.observe_publication_remote(
        ObservePublicationRemoteRequest {
            root: w.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: w.primary().approval,
            restart: false,
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    Ok(())
}

fn history(w: &World, remove: bool) -> Result<(), FixtureError> {
    let bare = w.bare()?;
    let oid = fixed(bare.refname_to_id("refs/heads/main"))?;
    fixed(bare.reference(CONTEXT, oid, false, "published fixture"))?;
    observe(w)?;
    if remove {
        fixed(fixed(bare.find_reference(CONTEXT))?.delete())?;
    }
    Ok(())
}

fn request(w: &World) -> RetireContextRequest {
    RetireContextRequest {
        root: w.root.clone(),
        kind: AuthoringKind::Ticket,
        item_id: ITEM.parse().unwrap(),
        operation_id: OperationId::new(),
    }
}

fn retire(
    w: &World,
    request: RetireContextRequest,
) -> Result<ContextRetirementReport, FixtureError> {
    let result = w.service.retire_context_authenticated(
        request,
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    );
    ssh_harness::observation(&[
        1,
        result.is_ok() as u128,
        matches!(result, Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired) as u128,
    ]);
    match fixed(result)? {
        ContextRetirementOutcome::Final(report) => Ok(report),
        _ => Err(FixtureError),
    }
}

fn fresh_absence() -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let repo = w.repo()?;
    let tracking = fixed(repo.refname_to_id("refs/remotes/origin/main"))?;
    let server_primary = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let request = request(&w);
    let mut objects_before = Vec::new();
    fixed(fixed(repo.odb())?.foreach(|oid| {
        objects_before.push(*oid);
        true
    }))?;
    objects_before.sort();
    let receives_before = w.server.receive_updates().len();
    let first = retire(&w, request.clone())?;
    assert_eq!(first.outcome, RetirementDisposition::Retired);
    assert_eq!(first.branch_removal, RetirementRemoval::Observed);
    assert_eq!(first.worktree_removal, RetirementRemoval::None);
    assert!(first.materialization_settled);
    assert!(repo.find_reference(CONTEXT).is_err());
    assert_eq!(
        fixed(repo.refname_to_id("refs/remotes/origin/main"))?,
        tracking
    );
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        server_primary
    );
    let mut objects_after = Vec::new();
    fixed(fixed(repo.odb())?.foreach(|oid| {
        objects_after.push(*oid);
        true
    }))?;
    objects_after.sort();
    assert_eq!(objects_after, objects_before);
    assert_eq!(w.server.receive_updates().len(), receives_before);
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    // Same-OID recreation and new materialization belong to a new lifetime.
    fixed(repo.reference(
        CONTEXT,
        fixed(repo.refname_to_id("refs/heads/main"))?,
        false,
        "new fixture lifetime",
    ))?;
    assert_eq!(retire(&w, request)?, first);
    assert!(repo.find_reference(CONTEXT).is_ok());
    w.privacy()
}

fn refusal(w: &World) -> Result<(), FixtureError> {
    let request = request(w);
    let first = retire(w, request.clone())?;
    assert_eq!(first.outcome, RetirementDisposition::Refused);
    assert_eq!(first.worktree_removal, RetirementRemoval::None);
    assert_eq!(first.branch_removal, RetirementRemoval::None);
    assert!(!first.materialization_settled);
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert_eq!(retire(w, request)?, first);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn unknown_history() -> Result<(), FixtureError> {
    refusal(&setup()?)
}
fn reappeared() -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, false)?;
    refusal(&w)
}
fn dirty_primary() -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    fixed(fs::write(
        w.root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
        b"uncommitted fixture edit",
    ))?;
    refusal(&w)
}

fn d14_ignored() -> Result<(), FixtureError> {
    d14_settlement(true)
}
fn d14_complete() -> Result<(), FixtureError> {
    d14_settlement(false)
}
fn d14_settlement(ignore: bool) -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let db = w.db()?;
    let owner = OperationId::new().to_string();
    let tip = fixed(w.repo()?.refname_to_id(CONTEXT))?;
    fixed(db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind,poll_completed_at) SELECT repository_id,?1,configuration_generation,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','manual','completed','before_discovery','completed',1,1,'update',1 FROM remote_polling_state",[&owner]))?;
    fixed(db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",rusqlite::params![ITEM,owner,tip.to_string()]))?;
    if ignore {
        fixed(db.execute_batch("CREATE TRIGGER retirement_test_ignore_d14 BEFORE DELETE ON remote_poll_materializations BEGIN SELECT RAISE(IGNORE); END;"))?;
    }
    let request = request(&w);
    let result = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    if !ignore {
        assert!(
            matches!(&result,ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::Retired && report.materialization_settled)
        );
        assert_eq!(
            fixed(db.query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            0
        );
        let next = OperationId::new().to_string();
        fixed(db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind,poll_completed_at) SELECT repository_id,?1,configuration_generation,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','manual','completed','before_discovery','completed',1,1,'update',1 FROM remote_polling_state",[&next]))?;
        fixed(db.execute("INSERT INTO remote_poll_materializations SELECT id,'document',?1,?2,?3,'complete' FROM repositories",rusqlite::params![ITEM,next,tip.to_string()]))?;
        fixed(
            w.repo()?
                .reference(CONTEXT, tip, false, "new fixture context lifetime"),
        )?;
        assert_eq!(
            fixed(w.service.retire_context_authenticated(
                request,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            ))?,
            result
        );
        assert_eq!(fixed(db.query_row("SELECT count(*) FROM remote_poll_materializations WHERE kind='document' AND operation_ulid=?1",[next],|r|r.get::<_,i64>(0)))?,1);
        assert_eq!(fixed(w.repo()?.refname_to_id(CONTEXT))?, tip);
        assert!(RepositoryService::open_at(&w.data()).is_ok());
        return w.privacy();
    }
    ssh_harness::observation(&[
        2,
        matches!(result, ContextRetirementOutcome::CleanupPending { .. }) as u128,
    ]);
    assert!(matches!(
        result,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::None,
            branch_removal: RetirementRemoval::Possible
        }
    ));
    assert!(w.repo()?.find_reference(CONTEXT).is_err());
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM remote_poll_materializations WHERE operation_ulid=?1",
            [owner],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    assert_eq!(fixed(db.query_row("SELECT phase,materialization_settled FROM context_retirement_records WHERE operation_ulid=?1",[request.operation_id.to_string()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,bool>(1)?))))?,("branch_deleting".into(),false));
    fixed(
        w.repo()?
            .reference(CONTEXT, tip, false, "new same-tip fixture lifetime"),
    )?;
    assert!(
        w.service
            .retire_context_authenticated(
                request,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert_eq!(fixed(w.repo()?.refname_to_id(CONTEXT))?, tip);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

fn fresh_document() -> Result<(), FixtureError> {
    const DOC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
    let w = setup()?;
    let repo = w.repo()?;
    fixed(fs::create_dir_all(w.root.join("docs")))?;
    fixed(fs::write(
        w.root.join("docs/retirement.md"),
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOC}\ntitle: Fixture document\n---\nDocument preserved on primary\n"
        ),
    ))?;
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(Path::new("docs/retirement.md")))?;
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let parent = fixed(fixed(repo.head())?.peel_to_commit())?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let tip = fixed(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "document primary fixture",
        &tree,
        &[&parent],
    ))?;
    let reference = format!("refs/heads/manyhands/document/{DOC}");
    fixed(repo.reference(&reference, tip, false, "document branch fixture"))?;
    let bare = w.bare()?;
    fixed(bare.reference(
        &reference,
        fixed(bare.refname_to_id("refs/heads/main"))?,
        false,
        "document published fixture",
    ))?;
    observe(&w)?;
    fixed(fixed(bare.find_reference(&reference))?.delete())?;
    let result = fixed(w.service.retire_context_authenticated(
        RetireContextRequest {
            root: w.root.clone(),
            kind: AuthoringKind::Document,
            item_id: fixed(DOC.parse())?,
            operation_id: OperationId::new(),
        },
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    assert!(
        matches!(result,ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::Retired && report.worktree_removal==RetirementRemoval::None && report.branch_removal==RetirementRemoval::Observed)
    );
    assert!(repo.find_reference(&reference).is_err());
    assert!(repo.find_reference(CONTEXT).is_ok());
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

fn host_stop() -> Result<(), FixtureError> {
    let w = setup()?;
    let request = request(&w);
    let first = fixed(w.service.retire_context_authenticated(
        request.clone(),
        None,
        &mut SessionCredentials::new(Provider),
    ))?;
    assert!(
        matches!(&first,ContextRetirementOutcome::Final(report) if report.code==ResultCode::HostApprovalRequired && report.outcome==RetirementDisposition::Refused)
    );
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        first
    );
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

fn held_stop(fence: bool) -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let request = request(&w);
    let hold = w.server.hold_advertisement()?;
    let data = w.data();
    let root = w.root.clone();
    let operation = request.operation_id;
    let result = std::thread::scope(|scope| {
        let controller=scope.spawn(move||{
            hold.wait_until_held().unwrap();
            let service=RepositoryService::open_at(&data).unwrap();
            let observer=service.active_remote_operation(&root).unwrap().unwrap().operation_id();
            let db=rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
            assert!(db.query_row("SELECT poll_kind='observe' AND priority='manual' FROM remote_operation_records WHERE operation_ulid=?1",[observer.to_string()],|r|r.get::<_,bool>(0)).unwrap());
            if fence{
                db.execute_batch("BEGIN IMMEDIATE; UPDATE context_retirement_attempts SET phase='fenced' WHERE phase='active'; UPDATE context_retirement_records SET owner_epoch=owner_epoch+1 WHERE phase='selected'; INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) SELECT operation_ulid,owner_epoch,repository_id,'active' FROM context_retirement_records WHERE phase='selected'; COMMIT;").unwrap();
            }else{service.cancel_remote_operation(&root,observer).unwrap();}
            // This read/write controller can progress while transport is held:
            // the retiring executor has retained no SQLite/cache guard.
            assert!(db.query_row("SELECT count(*) FROM context_retirement_records WHERE operation_ulid=?1",[operation.to_string()],|r|r.get::<_,i64>(0)).unwrap()==1);
            hold.release().unwrap();
        });
        let result = w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider),
        );
        controller.join().unwrap();
        result
    });
    if fence {
        assert!(matches!(result,Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired));
    } else {
        assert!(
            matches!(result,Ok(ContextRetirementOutcome::Final(report)) if report.code==ResultCode::Cancelled && report.branch_removal==RetirementRemoval::None)
        );
    }
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn held_cancel() -> Result<(), FixtureError> {
    held_stop(false)
}
fn held_fence() -> Result<(), FixtureError> {
    held_stop(true)
}

fn spec_s2_ref_move() -> Result<(), FixtureError> {
    spec_s2_move(false)
}
fn spec_s2_primary_move() -> Result<(), FixtureError> {
    spec_s2_move(true)
}
fn spec_s2_move(primary: bool) -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let materialization = complete_materialization(&w)?;
    let request = request(&w);
    let repo = w.repo()?;
    let old = fixed(repo.refname_to_id(CONTEXT))?;
    let commit = fixed(repo.find_commit(old))?;
    let tree = fixed(commit.tree())?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let next = fixed(repo.commit(
        None,
        &sig,
        &sig,
        "new same-tree fixture lifetime",
        &tree,
        &[&commit],
    ))?;
    let item = w.root.join(format!(".manyhands/tickets/{ITEM}/ticket.md"));
    let bytes = fixed(fs::read(&item))?;
    let path = w.root.clone();
    let data = w.data();
    RepositoryService::set_retirement_intent_hook_for_testing(move || {
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM context_retirement_effect_intents WHERE observation IS NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        let git = git2::Repository::open(path).unwrap();
        git.reference(
            if primary { "refs/heads/main" } else { CONTEXT },
            next,
            true,
            "external move after durable intent",
        )
        .unwrap();
        ssh_harness::observation(&[201, 1]);
    });
    let result = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    ssh_harness::observation(&[
        202,
        matches!(&result, ContextRetirementOutcome::Final(_)) as u128,
        matches!(&result, ContextRetirementOutcome::CleanupPending { .. }) as u128,
    ]);
    assert!(
        matches!(&result,ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::Refused && report.code==ResultCode::ExternalChange && report.branch_removal==RetirementRemoval::None && report.worktree_removal==RetirementRemoval::None && !report.materialization_settled)
    );
    assert_eq!(
        fixed(repo.refname_to_id(CONTEXT))?,
        if primary { old } else { next }
    );
    assert_eq!(fixed(fs::read(&item))?, bytes);
    let db = w.db()?;
    assert_eq!(
        fixed(db.query_row(
            "SELECT observation FROM context_retirement_effect_intents",
            [],
            |r| r.get::<_, String>(0)
        ))?,
        "changed"
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM context_retirement_attempts WHERE phase='ended'",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT state FROM operation_records WHERE operation_ulid=?1",
            [request.operation_id.to_string()],
            |r| r.get::<_, String>(0)
        ))?,
        "completed"
    );
    assert_materialization(&w, &materialization)?;
    let before_replay = registry_image(&db);
    fixed(repo.reference(
        if primary { "refs/heads/main" } else { CONTEXT },
        old,
        true,
        "repair does not reopen old request",
    ))?;
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        result
    );
    assert_eq!(registry_image(&db), before_replay);
    assert_eq!(fixed(repo.refname_to_id(CONTEXT))?, old);
    assert_materialization(&w, &materialization)?;
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

fn complete_materialization(w: &World) -> Result<(String, String), FixtureError> {
    let db = w.db()?;
    let owner = OperationId::new().to_string();
    let tip = fixed(w.repo()?.refname_to_id(CONTEXT))?.to_string();
    fixed(db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind,poll_completed_at) SELECT repository_id,?1,configuration_generation,'origin','main','refs/heads/main','refs/remotes/origin/main','poll','manual','completed','before_discovery','completed',1,1,'update',1 FROM remote_polling_state",[&owner]))?;
    fixed(db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",rusqlite::params![ITEM,owner,tip]))?;
    Ok((owner, tip))
}
fn assert_materialization(w: &World, tuple: &(String, String)) -> Result<(), FixtureError> {
    assert_eq!(fixed(w.db()?.query_row("SELECT operation_ulid,target_oid,step FROM remote_poll_materializations WHERE kind='ticket' AND item_id=?1",[ITEM],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))))?,(tuple.0.clone(),tuple.1.clone(),"complete".into()));
    Ok(())
}
fn spec_s2_fenced() -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let request = request(&w);
    let data = w.data();
    RepositoryService::set_retirement_intent_hook_for_testing(move || {
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        db.execute_batch("BEGIN IMMEDIATE; UPDATE context_retirement_attempts SET phase='fenced' WHERE phase='active'; UPDATE context_retirement_records SET owner_epoch=owner_epoch+1; INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) SELECT operation_ulid,owner_epoch,repository_id,'active' FROM context_retirement_records; COMMIT;").unwrap();
    });
    assert!(
        matches!(w.service.retire_context_authenticated(request.clone(),w.primary().approval,&mut SessionCredentials::new(Provider)),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT observation FROM context_retirement_effect_intents",
            [],
            |r| r.get::<_, Option<String>>(0)
        ))?,
        None
    );
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    // A new call has no no-effect witness and cannot revive branch deletion.
    assert!(
        w.service
            .retire_context_authenticated(
                request,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

fn spec_s1_live_handoff() -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let request = request(&w);
    let data = w.data();
    let root = w.root.clone();
    let snapshot = std::rc::Rc::new(std::cell::RefCell::new(None));
    let captured = snapshot.clone();
    RepositoryService::set_retirement_intent_hook_for_testing(move || {
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT observation FROM context_retirement_effect_intents",
                [],
                |r| r.get::<_, Option<String>>(0)
            )
            .unwrap(),
            None
        );
        assert!(
            git2::Repository::open(root)
                .unwrap()
                .find_reference(CONTEXT)
                .is_ok()
        );
        // Ordinary SQL only: no disabled checks, trigger removal or schema edits.
        db.execute_batch("BEGIN IMMEDIATE; UPDATE context_retirement_records SET phase='index_pending',absence_basis='owned_intent',materialization_settled=1,branch_removal='observed'; UPDATE operation_records SET state='authoritative_observed',completed_step='context_branch_released' WHERE action='retire_context'; COMMIT;").unwrap();
        *captured.borrow_mut() = Some(registry_image(&db));
    });
    assert!(
        matches!(w.service.retire_context_authenticated(request.clone(),w.primary().approval,&mut SessionCredentials::new(Provider)),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    assert!(
        matches!(RepositoryService::open_at(&w.data()),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    assert!(
        matches!(w.service.retire_context(request),Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(
        registry_image(&w.db()?),
        snapshot.borrow().as_ref().unwrap().clone()
    );
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    w.privacy()
}

fn registry_image(
    db: &rusqlite::Connection,
) -> std::collections::BTreeMap<String, Vec<Vec<rusqlite::types::Value>>> {
    let mut tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables.push("sqlite_master".into());
    let mut image = std::collections::BTreeMap::new();
    for table in tables {
        let rows = db
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap()
            .query_map([], |r| {
                (0..r.as_ref().column_count()).map(|i| r.get(i)).collect()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        image.insert(table, rows);
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

fn spec_s4_packed_primary() -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let repo = w.repo()?;
    let tip = fixed(repo.refname_to_id("refs/heads/main"))?;
    let packed = repo.commondir().join("packed-refs");
    let old = fs::read_to_string(&packed).unwrap_or_default();
    let mut refs = std::collections::BTreeMap::new();
    for line in old.lines().filter(|line| !line.starts_with(['#', '^'])) {
        if let Some((oid, name)) = line.split_once(' ') {
            refs.insert(name.to_string(), oid.to_string());
        }
    }
    refs.insert("refs/heads/main".into(), tip.to_string());
    let mut bytes = String::from("# pack-refs with: sorted\n");
    for (name, oid) in refs {
        bytes.push_str(&format!("{oid} {name}\n"));
    }
    fixed(fs::write(&packed, &bytes))?;
    fixed(fs::remove_file(repo.commondir().join("refs/heads/main")))?;
    let report = retire(&w, request(&w))?;
    assert_eq!(report.outcome, RetirementDisposition::Retired);
    assert_eq!(report.branch_removal, RetirementRemoval::Observed);
    assert_eq!(fixed(fs::read_to_string(&packed))?, bytes);
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, tip);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

#[cfg(unix)]
fn spec_s4_initial_target() -> Result<(), FixtureError> {
    spec_s4_redirect(false, "target")
}
#[cfg(unix)]
fn spec_s4_final_target() -> Result<(), FixtureError> {
    spec_s4_redirect(true, "target")
}
#[cfg(unix)]
fn spec_s4_initial_ancestor() -> Result<(), FixtureError> {
    spec_s4_redirect(false, "ancestor")
}
#[cfg(unix)]
fn spec_s4_final_ancestor() -> Result<(), FixtureError> {
    spec_s4_redirect(true, "ancestor")
}
#[cfg(unix)]
fn spec_s4_initial_locks() -> Result<(), FixtureError> {
    for name in ["primary_lock", "packed_lock", "head_lock"] {
        spec_s4_redirect(false, name)?;
    }
    Ok(())
}
#[cfg(unix)]
fn spec_s4_final_locks() -> Result<(), FixtureError> {
    for name in ["primary_lock", "packed_lock", "head_lock"] {
        spec_s4_redirect(true, name)?;
    }
    Ok(())
}

#[cfg(unix)]
fn spec_s4_redirect(final_boundary: bool, variant: &'static str) -> Result<(), FixtureError> {
    use std::os::unix::fs::symlink;
    let w = setup()?;
    let repo = w.repo()?;
    let old = fixed(repo.refname_to_id(CONTEXT))?;
    fixed(repo.reference(
        "refs/heads/review/main",
        old,
        false,
        "nested primary fixture",
    ))?;
    fixed(repo.set_head("refs/heads/review/main"))?;
    let config = w.root.join(".manyhands/config.toml");
    fixed(fs::write(
        &config,
        fixed(fs::read_to_string(&config))?.replace(
            "primary_branch = \"main\"",
            "primary_branch = \"review/main\"",
        ),
    ))?;
    let bare = w.bare()?;
    fixed(bare.reference(
        "refs/heads/review/main",
        fixed(bare.refname_to_id("refs/heads/main"))?,
        false,
        "nested primary fixture",
    ))?;
    history(&w, true)?;
    let materialization = complete_materialization(&w)?;
    let common = repo.commondir().to_path_buf();
    let slot = common.join(match variant {
        "target" => "refs/heads/review/main",
        "ancestor" => "refs/heads/review",
        "primary_lock" => "refs/heads/review/main.lock",
        "packed_lock" => "packed-refs.lock",
        "head_lock" => "HEAD.lock",
        _ => unreachable!(),
    });
    let backing = common.join("fixture-primary-storage");
    let corrupt = {
        let slot = slot.clone();
        let backing = backing.clone();
        move || {
            if matches!(variant, "target" | "ancestor") {
                fs::rename(&slot, &backing).unwrap();
            } else {
                fs::write(&backing, b"private fixture lock").unwrap();
            }
            symlink(&backing, &slot).unwrap();
        }
    };
    if final_boundary {
        RepositoryService::set_retirement_intent_hook_for_testing(corrupt);
    } else {
        corrupt();
    }
    let request = request(&w);
    let bytes = fixed(fs::read(
        w.root.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
    ))?;
    let result = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    ssh_harness::observation(&[
        204,
        final_boundary as u128,
        matches!(&result,ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::Refused)
            as u128,
    ]);
    assert!(
        matches!(&result,ContextRetirementOutcome::Final(report) if report.outcome==RetirementDisposition::Refused && report.worktree_removal==RetirementRemoval::None && report.branch_removal==RetirementRemoval::None && !report.materialization_settled)
    );
    assert_eq!(fixed(repo.refname_to_id(CONTEXT))?, old);
    assert_eq!(fixed(fs::read_link(&slot))?, backing);
    assert_eq!(fixed(repo.refname_to_id("refs/heads/review/main"))?, old);
    if !matches!(variant, "target" | "ancestor") {
        assert_eq!(fixed(fs::read(&backing))?, b"private fixture lock");
    }
    assert_eq!(
        fixed(fs::read(
            w.root.join(format!(".manyhands/tickets/{ITEM}/ticket.md"))
        ))?,
        bytes
    );
    assert_materialization(&w, &materialization)?;
    let db = w.db()?;
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM context_retirement_effect_intents",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        i64::from(final_boundary)
    );
    if final_boundary {
        assert_eq!(
            fixed(db.query_row(
                "SELECT observation FROM context_retirement_effect_intents",
                [],
                |r| r.get::<_, String>(0)
            ))?,
            "changed"
        );
    }
    let before_replay = registry_image(&db);
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request.clone(),
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        result
    );
    assert_eq!(registry_image(&db), before_replay);
    fixed(fs::remove_file(&slot))?;
    if matches!(variant, "target" | "ancestor") {
        fixed(fs::rename(&backing, &slot))?;
    }
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        result
    );
    assert_eq!(registry_image(&db), before_replay);
    assert_eq!(fixed(repo.refname_to_id(CONTEXT))?, old);
    assert_materialization(&w, &materialization)?;
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
