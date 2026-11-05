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
    (
        "retirement_quality_cleanup_lease_contention_is_final_external_change",
        quality_cleanup_lease_contended,
    ),
    (
        "retirement_quality_cleanup_lease_uncontended_control_retires",
        quality_cleanup_lease_uncontended,
    ),
    (
        "retirement_quality_cleanup_lease_preselection_busy_remains_retryable",
        quality_preselection_busy,
    ),
    (
        "retirement_present_fresh_prunes_then_retires_and_replay_preserves_recreation",
        present_fresh,
    ),
    (
        "retirement_present_dirty_worktree_refuses_before_marker",
        present_dirty,
    ),
    (
        "retirement_present_foreign_marker_is_preserved",
        present_foreign_marker,
    ),
    (
        "retirement_present_marker_changed_after_intent_is_final_no_effect",
        present_marker_after_intent,
    ),
    (
        "retirement_present_after_prune_branch_refusal_remains_owed",
        present_branch_stop,
    ),
    (
        "retirement_present_prune_sql_gap_preserves_surviving_branch_on_retry",
        present_prune_sql_gap,
    ),
    (
        "retirement_present_branch_sql_gap_preserves_same_oid_recreation",
        present_branch_sql_gap,
    ),
    (
        "retirement_present_installed_identity_is_checked_after_network",
        present_marker_after_network,
    ),
    (
        "retirement_present_cleanup_hidden_and_ignored_matrix",
        present_cleanup_matrix,
    ),
    (
        "retirement_present_document_prunes_then_retires",
        present_document,
    ),
    (
        "retirement_present_unverified_effect_evidence_is_preserved_as_corruption",
        present_unverified_effect,
    ),
    (
        "retirement_present_incomplete_or_ambiguous_materialization_refuses_before_prune",
        present_materialization_refusal,
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

fn quality_cleanup_lease_contended() -> Result<(), FixtureError> {
    quality_cleanup_lease(true)
}
fn quality_cleanup_lease_uncontended() -> Result<(), FixtureError> {
    quality_cleanup_lease(false)
}
fn quality_cleanup_lease(contended: bool) -> Result<(), FixtureError> {
    use fs4::fs_std::FileExt;
    let w = setup()?;
    history(&w, true)?;
    let materialization = complete_materialization(&w)?;
    let request = request(&w);
    let before = resource_image(&w.root);
    let remote_before = resource_image(w.bare()?.path());
    let hold = w.server.hold_advertisement()?;
    let data = w.data();
    let running_request = request.clone();
    let approval = w.primary().approval;
    let running = std::thread::spawn(move || {
        let service = RepositoryService::open_at(&data).unwrap();
        service.retire_context_authenticated(
            running_request,
            approval,
            &mut SessionCredentials::new(Provider),
        )
    });
    hold.wait_until_held()?;
    // Same actual common-lease file and fs4 protocol as the existing C5 SSH
    // contention controls; no synthetic acquire failure or new test-only API.
    let lease = fixed(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(w.repo()?.commondir().join("manyhands-operation.lock")),
    )?;
    if contended {
        assert!(fixed(FileExt::try_lock_exclusive(&lease))?);
    }
    hold.release()?;
    let result = fixed(running.join())?;
    let db = w.db()?;
    let selected = fixed(db.query_row(
        "SELECT count(*) FROM context_retirement_records WHERE phase='selected'",
        [],
        |r| r.get::<_, i64>(0),
    ))?;
    let pins = fixed(db.query_row(
        "SELECT count(*) FROM retirement_observation_pins",
        [],
        |r| r.get::<_, i64>(0),
    ))?;
    let intents = fixed(db.query_row(
        "SELECT count(*) FROM context_retirement_effect_intents",
        [],
        |r| r.get::<_, i64>(0),
    ))?;
    ssh_harness::observation(&[
        301,
        contended as u128,
        matches!(&result, Ok(ContextRetirementOutcome::Final(_))) as u128,
        matches!(&result,Err(e) if e.kind==RepositoryErrorKind::RepositoryBusy) as u128,
        selected as u128,
        pins as u128,
        intents as u128,
    ]);
    assert!(
        matches!(&result,Ok(ContextRetirementOutcome::Final(report)) if if contended {
        report.code==ResultCode::ExternalChange && report.outcome==RetirementDisposition::Refused && report.worktree_removal==RetirementRemoval::None && report.branch_removal==RetirementRemoval::None && !report.materialization_settled
    }else{report.code==ResultCode::Ok && report.outcome==RetirementDisposition::Retired && report.branch_removal==RetirementRemoval::Observed && report.materialization_settled})
    );
    if contended {
        assert!(
            resource_image(&w.root) == before,
            "contention must preserve the entire local resource image"
        );
        assert_materialization(&w, &materialization)?;
        assert_eq!(intents, 0);
    } else {
        assert!(w.repo()?.find_reference(CONTEXT).is_err());
        assert_eq!(intents, 1);
    }
    assert!(
        resource_image(w.bare()?.path()) == remote_before,
        "retirement never modifies remote resources"
    );
    assert_eq!(pins, 0);
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM context_retirement_attempts WHERE phase='active'",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert_eq!(fixed(db.query_row("SELECT count(*) FROM operation_records WHERE action='retire_context' AND state!='completed'",[],|r|r.get::<_,i64>(0)))?,0);
    assert_eq!(fixed(db.query_row("SELECT count(*) FROM remote_operation_records WHERE operation_ulid IN (SELECT observer_operation_ulid FROM context_retirement_attempts) AND phase!='completed'",[],|r|r.get::<_,i64>(0)))?,0);
    assert_eq!(fixed(db.query_row("SELECT count(*) FROM retirement_observation_pin_releases WHERE reason='consumer_terminal'",[],|r|r.get::<_,i64>(0)))?,1);
    // Keep contention until AFTER the retiring call and its valid Final result;
    // metadata-only TF/Z must not depend on obtaining this Git lease later.
    if contended {
        fixed(FileExt::unlock(&lease))?;
    }
    let result = fixed(result)?;
    let first_replay_registry = registry_image(&db);
    let first_replay_resources = resource_image(&w.root);
    w.server.wait_for_helper_audits()?;
    let first_commands = w.server.commands();
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request.clone(),
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        result
    );
    w.server.wait_for_helper_audits()?;
    assert!(
        registry_image(&db) == first_replay_registry,
        "releasing the lease must not reopen or settle the old request again"
    );
    assert!(resource_image(&w.root) == first_replay_resources);
    assert_eq!(
        w.server.commands(),
        first_commands,
        "retry after lease release must not advertise again"
    );
    if !contended {
        fixed(w.repo()?.reference(
            CONTEXT,
            fixed(materialization.1.parse())?,
            false,
            "new same-OID lifetime",
        ))?;
    }
    fixed(db.execute("INSERT INTO remote_poll_materializations SELECT id,'document',?1,?2,?3,'complete' FROM repositories",rusqlite::params![ITEM,materialization.0,materialization.1]))?;
    let reopened = fixed(RepositoryService::open_at(&w.data()))?;
    let replay_registry = registry_image(&db);
    let replay_resources = resource_image(&w.root);
    w.server.wait_for_helper_audits()?;
    let commands = w.server.commands();
    assert_eq!(
        fixed(reopened.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        result
    );
    w.server.wait_for_helper_audits()?;
    assert!(
        registry_image(&db) == replay_registry,
        "old request must not consume new same-ID materialization"
    );
    assert!(
        resource_image(&w.root) == replay_resources,
        "replay must not delete the surviving/recreated branch"
    );
    assert_eq!(
        w.server.commands(),
        commands,
        "replay must not advertise again"
    );
    w.privacy()
}

fn quality_preselection_busy() -> Result<(), FixtureError> {
    use fs4::fs_std::FileExt;
    let w = setup()?;
    history(&w, true)?;
    let request = request(&w);
    let before = registry_image(&w.db()?);
    let resources = resource_image(&w.root);
    let lease = fixed(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(w.repo()?.commondir().join("manyhands-operation.lock")),
    )?;
    assert!(fixed(FileExt::try_lock_exclusive(&lease))?);
    let commands = w.server.commands();
    assert!(
        matches!(w.service.retire_context_authenticated(request.clone(),w.primary().approval,&mut SessionCredentials::new(Provider)),Err(ref e) if e.kind==RepositoryErrorKind::RepositoryBusy)
    );
    assert!(registry_image(&w.db()?) == before);
    assert!(resource_image(&w.root) == resources);
    assert_eq!(w.server.commands(), commands);
    fixed(FileExt::unlock(&lease))?;
    assert_eq!(retire(&w, request)?.outcome, RetirementDisposition::Retired);
    w.privacy()
}

#[derive(PartialEq, Eq)]
enum ResourceNode {
    Directory(u32),
    File(u32, Vec<u8>),
    Link(std::path::PathBuf),
}

const INSTANCE_MARKER: &str = "manyhands-retirement-instance";
fn add_present(w: &World) -> Result<std::path::PathBuf, FixtureError> {
    let path = w.root.join(".manyhands/worktrees").join(ITEM);
    fixed(fs::create_dir_all(fixed(
        path.parent().ok_or(FixtureError),
    )?))?;
    let repo = w.repo()?;
    let reference = fixed(repo.find_reference(CONTEXT))?;
    fixed(repo.worktree(
        ITEM,
        &path,
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    ))?;
    Ok(path)
}
fn marker_path(w: &World) -> Result<std::path::PathBuf, FixtureError> {
    Ok(w.repo()?
        .commondir()
        .join("worktrees")
        .join(ITEM)
        .join(INSTANCE_MARKER))
}
fn present_fresh() -> Result<(), FixtureError> {
    let w = setup()?;
    let path = add_present(&w)?;
    fixed(w.service.refresh_repository(RefreshRepositoryRequest {
        root: w.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM contexts WHERE item_id=?1",
            [ITEM],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    history(&w, true)?;
    let materialization = complete_materialization(&w)?;
    let tracking = fixed(w.repo()?.refname_to_id("refs/remotes/origin/main"))?;
    let draft = w.data().join("draft-sentinel");
    fixed(fs::write(&draft, b"fixture unsaved draft"))?;
    assert!(!marker_path(&w)?.exists());
    let objects = resource_image(&w.repo()?.commondir().join("objects"));
    let server = resource_image(w.bare()?.path());
    let request = request(&w);
    let report = retire(&w, request.clone())?;
    assert_eq!(report.outcome, RetirementDisposition::Retired);
    assert_eq!(report.worktree_removal, RetirementRemoval::Observed);
    assert_eq!(report.branch_removal, RetirementRemoval::Observed);
    assert!(report.materialization_settled);
    assert!(!path.exists());
    assert!(!marker_path(&w)?.parent().unwrap().exists());
    assert!(w.repo()?.find_reference(CONTEXT).is_err());
    assert_eq!(
        fixed(w.repo()?.refname_to_id("refs/remotes/origin/main"))?,
        tracking
    );
    assert_eq!(fixed(fs::read(&draft))?, b"fixture unsaved draft");
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM remote_poll_materializations",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM contexts WHERE item_id=?1",
            [ITEM],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert!(resource_image(&w.repo()?.commondir().join("objects")) == objects);
    assert!(resource_image(w.bare()?.path()) == server);
    fixed(w.repo()?.reference(
        CONTEXT,
        fixed(materialization.1.parse())?,
        false,
        "new fixture lifetime",
    ))?;
    add_present(&w)?;
    fixed(w.db()?.execute("INSERT INTO remote_poll_materializations SELECT id,'document',?1,?2,?3,'complete' FROM repositories",rusqlite::params![ITEM,materialization.0,materialization.1]))?;
    let resources = resource_image(&w.root);
    let db = registry_image(&w.db()?);
    let commands = w.server.commands();
    assert_eq!(retire(&w, request)?, report);
    assert!(resource_image(&w.root) == resources);
    assert!(registry_image(&w.db()?) == db);
    assert_eq!(w.server.commands(), commands);
    assert!(!marker_path(&w)?.exists());
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn present_dirty() -> Result<(), FixtureError> {
    let w = setup()?;
    let path = add_present(&w)?;
    history(&w, true)?;
    fixed(fs::write(
        path.join("untracked-fixture"),
        b"unsaved fixture work",
    ))?;
    let before = resource_image(&w.root);
    let report = retire(&w, request(&w))?;
    assert_eq!(report.outcome, RetirementDisposition::Refused);
    assert_eq!(report.code, ResultCode::WorktreeNotClean);
    assert_eq!(report.worktree_removal, RetirementRemoval::None);
    assert!(resource_image(&w.root) == before);
    assert!(!marker_path(&w)?.exists());
    w.privacy()
}
fn present_foreign_marker() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    fixed(fs::write(
        marker_path(&w)?,
        b"foreign/partial fixture token",
    ))?;
    let before = resource_image(&w.root);
    let request = request(&w);
    let report = retire(&w, request.clone())?;
    assert_eq!(report.outcome, RetirementDisposition::Refused);
    assert_eq!(report.worktree_removal, RetirementRemoval::None);
    assert!(resource_image(&w.root) == before);
    assert_eq!(retire(&w, request)?, report);
    w.privacy()
}
fn present_marker_after_intent() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let marker = marker_path(&w)?;
    let changed = marker.clone();
    RepositoryService::set_retirement_worktree_hook_for_testing(move || {
        fs::write(changed, b"changed fixture marker").unwrap();
    });
    let request = request(&w);
    let result = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    ssh_harness::observation(&[
        401,
        matches!(&result, ContextRetirementOutcome::Final(_)) as u128,
        matches!(&result, ContextRetirementOutcome::CleanupPending { .. }) as u128,
    ]);
    assert!(
        matches!(&result,ContextRetirementOutcome::Final(r) if r.code==ResultCode::ExternalChange && r.outcome==RetirementDisposition::Refused && r.worktree_removal==RetirementRemoval::None && r.branch_removal==RetirementRemoval::None)
    );
    assert_eq!(fixed(fs::read(marker))?, b"changed fixture marker");
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT observation FROM context_retirement_effect_intents",
            [],
            |r| r.get::<_, String>(0)
        ))?,
        "changed"
    );
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        result
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn present_branch_stop() -> Result<(), FixtureError> {
    let w = setup()?;
    let path = add_present(&w)?;
    history(&w, true)?;
    let repo = w.repo()?;
    let old = fixed(repo.refname_to_id(CONTEXT))?;
    let parent = fixed(repo.find_commit(old))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let next = fixed(repo.commit(
        None,
        &sig,
        &sig,
        "moved primary after prune",
        &fixed(parent.tree())?,
        &[&parent],
    ))?;
    let root = w.root.clone();
    RepositoryService::set_retirement_intent_hook_for_testing(move || {
        git2::Repository::open(root)
            .unwrap()
            .reference("refs/heads/main", next, true, "external primary move")
            .unwrap();
    });
    let request = request(&w);
    let result = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    assert_eq!(
        result,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Observed,
            branch_removal: RetirementRemoval::None
        }
    );
    assert!(!path.exists());
    assert_eq!(fixed(repo.refname_to_id(CONTEXT))?, old);
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT completed_step FROM operation_records WHERE operation_ulid=?1",
            [request.operation_id.to_string()],
            |r| r.get::<_, String>(0)
        ))?,
        "context_worktree_released"
    );
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT observation FROM context_retirement_effect_intents WHERE resource='branch'",
            [],
            |r| r.get::<_, String>(0)
        ))?,
        "changed"
    );
    fixed(repo.reference("refs/heads/main", old, true, "repair not new authority"))?;
    assert!(
        w.service
            .retire_context_authenticated(
                request,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert_eq!(fixed(repo.refname_to_id(CONTEXT))?, old);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn present_prune_sql_gap() -> Result<(), FixtureError> {
    present_sql_gap(false)
}
fn present_branch_sql_gap() -> Result<(), FixtureError> {
    present_sql_gap(true)
}
fn present_sql_gap(branch: bool) -> Result<(), FixtureError> {
    let w = setup()?;
    let path = add_present(&w)?;
    history(&w, true)?;
    let tuple = complete_materialization(&w)?;
    if branch {
        fixed(w.db()?.execute_batch("CREATE TRIGGER retirement_present_ignore_d14 BEFORE DELETE ON remote_poll_materializations BEGIN SELECT RAISE(IGNORE); END;"))?;
    } else {
        fixed(w.db()?.execute_batch("CREATE TRIGGER retirement_present_prune_receipt BEFORE UPDATE OF observation ON context_retirement_effect_intents WHEN OLD.resource='worktree' BEGIN SELECT RAISE(ABORT,'fixture'); END;"))?;
    }
    let request = request(&w);
    let result = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    assert_eq!(
        result,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Observed,
            branch_removal: if branch {
                RetirementRemoval::Possible
            } else {
                RetirementRemoval::None
            }
        }
    );
    assert!(!path.exists());
    assert_materialization(&w, &tuple)?;
    if branch {
        fixed(w.repo()?.reference(
            CONTEXT,
            fixed(tuple.1.parse())?,
            false,
            "same-OID fixture recreation",
        ))?;
    }
    add_present(&w)?;
    let resources = resource_image(&w.root);
    let registry = registry_image(&w.db()?);
    assert!(
        w.service
            .retire_context_authenticated(
                request,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert!(resource_image(&w.root) == resources);
    assert!(registry_image(&w.db()?) == registry);
    assert!(!marker_path(&w)?.exists());
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn present_marker_after_network() -> Result<(), FixtureError> {
    use fs4::fs_std::FileExt;
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    let hold = w.server.hold_advertisement()?;
    let root = w.root.clone();
    let data = w.data();
    let marker = marker_path(&w)?;
    let changed = marker.clone();
    let result = std::thread::scope(|scope| {
        let controller = scope.spawn(move || {
            hold.wait_until_held().unwrap();
            let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
            let (nonce, verified): (String, bool) = db
                .query_row(
                    "SELECT instance_nonce,instance_verified FROM context_retirement_records",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert!(verified);
            assert_eq!(
                fs::read(&changed).unwrap(),
                format!("manyhands-retirement-instance-v1\nticket\n{ITEM}\n{nonce}\n").into_bytes()
            );
            let lease = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(
                    git2::Repository::open(root)
                        .unwrap()
                        .commondir()
                        .join("manyhands-operation.lock"),
                )
                .unwrap();
            assert!(FileExt::try_lock_exclusive(&lease).unwrap());
            FileExt::unlock(&lease).unwrap();
            fs::write(
                &changed,
                format!(
                    "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
                    OperationId::new()
                ),
            )
            .unwrap();
            hold.release().unwrap();
        });
        let result = w.service.retire_context_authenticated(
            request.clone(),
            w.primary().approval,
            &mut SessionCredentials::new(Provider),
        );
        controller.join().unwrap();
        result
    });
    assert!(
        matches!(&result,Ok(ContextRetirementOutcome::Final(r)) if r.outcome==RetirementDisposition::Refused && r.worktree_removal==RetirementRemoval::None)
    );
    let resources = resource_image(&w.root);
    let registry = registry_image(&w.db()?);
    let commands = w.server.commands();
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        fixed(result)?
    );
    assert!(resource_image(&w.root) == resources);
    assert!(registry_image(&w.db()?) == registry);
    assert_eq!(w.server.commands(), commands);
    w.privacy()
}
fn present_cleanup_matrix() -> Result<(), FixtureError> {
    for variant in [
        "dirty",
        "staged",
        "ignored",
        "--assume-unchanged",
        "--skip-worktree",
    ] {
        let w = setup()?;
        let path = add_present(&w)?;
        history(&w, true)?;
        let relative = format!(".manyhands/tickets/{ITEM}/ticket.md");
        if variant == "ignored" {
            let exclude = w.repo()?.commondir().join("info/exclude");
            let mut bytes = fixed(fs::read(&exclude))?;
            bytes.extend_from_slice(b"\nignored-retirement-fixture\n");
            fixed(fs::write(exclude, bytes))?;
            fixed(fs::write(
                path.join("ignored-retirement-fixture"),
                b"unsaved ignored fixture",
            ))?;
        } else {
            if variant.starts_with("--") {
                let output = fixed(
                    std::process::Command::new("git")
                        .arg("-C")
                        .arg(&path)
                        .args(["update-index", variant, &relative])
                        .output(),
                )?;
                assert!(output.status.success());
            }
            fixed(fs::write(path.join(&relative), b"unsaved hidden fixture"))?;
            if variant == "staged" {
                let linked = fixed(git2::Repository::open(&path))?;
                let mut index = fixed(linked.index())?;
                fixed(index.add_path(Path::new(&relative)))?;
                fixed(index.write())?;
            }
        }
        let before = resource_image(&w.root);
        let report = retire(&w, request(&w))?;
        assert_eq!(report.code, ResultCode::WorktreeNotClean);
        assert!(!marker_path(&w)?.exists());
        assert!(resource_image(&w.root) == before);
        w.privacy()?;
    }
    Ok(())
}
fn present_document() -> Result<(), FixtureError> {
    const DOC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
    let w = setup()?;
    let repo = w.repo()?;
    fixed(fs::create_dir_all(w.root.join("docs")))?;
    fixed(fs::write(
        w.root.join("docs/retirement.md"),
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOC}\ntitle: Fixture\n---\nPreserved document\n"
        ),
    ))?;
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(Path::new("docs/retirement.md")))?;
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let parent = fixed(repo.head().and_then(|h| h.peel_to_commit()))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let tip = fixed(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "document fixture",
        &tree,
        &[&parent],
    ))?;
    let name = format!("refs/heads/manyhands/document/{DOC}");
    let reference = fixed(repo.reference(&name, tip, false, "document fixture"))?;
    let path = w.root.join(".manyhands/worktrees").join(DOC);
    fixed(fs::create_dir_all(path.parent().unwrap()))?;
    fixed(repo.worktree(
        DOC,
        &path,
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    ))?;
    let bare = w.bare()?;
    fixed(bare.reference(
        &name,
        fixed(bare.refname_to_id("refs/heads/main"))?,
        false,
        "published fixture",
    ))?;
    observe(&w)?;
    fixed(bare.find_reference(&name).and_then(|mut r| r.delete()))?;
    let outcome = fixed(w.service.retire_context_authenticated(
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
        matches!(outcome,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired && r.worktree_removal==RetirementRemoval::Observed && r.branch_removal==RetirementRemoval::Observed)
    );
    assert!(!path.exists());
    assert!(repo.find_reference(CONTEXT).is_ok());
    w.privacy()
}
fn present_unverified_effect() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let root = w.root.clone();
    let data = w.data();
    let snapshot = std::rc::Rc::new(std::cell::RefCell::new(None));
    let captured = snapshot.clone();
    RepositoryService::set_retirement_worktree_hook_for_testing(move || {
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        db.execute(
            "UPDATE context_retirement_records SET instance_verified=0",
            [],
        )
        .unwrap();
        *captured.borrow_mut() = Some((resource_image(&root), registry_image(&db)));
    });
    let result = w.service.retire_context_authenticated(
        request(&w),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    );
    let before = snapshot.borrow();
    let (resources, registry) = before.as_ref().unwrap();
    ssh_harness::observation(&[
        402,
        matches!(&result,Err(e) if e.kind==RepositoryErrorKind::RecoveryRequired) as u128,
        (resource_image(&w.root) == *resources) as u128,
    ]);
    assert!(matches!(result,Err(ref e) if e.kind==RepositoryErrorKind::RecoveryRequired));
    assert!(
        resource_image(&w.root) == *resources,
        "unverified durable effect evidence must not authorize pruning"
    );
    assert!(registry_image(&w.db()?) == *registry);
    w.privacy()
}
fn present_materialization_refusal() -> Result<(), FixtureError> {
    for ambiguous in [false, true] {
        let w = setup()?;
        add_present(&w)?;
        history(&w, true)?;
        let tuple = complete_materialization(&w)?;
        let db = w.db()?;
        if ambiguous {
            fixed(db.execute("INSERT INTO remote_poll_materializations SELECT id,'document',?1,?2,?3,'complete' FROM repositories",rusqlite::params![ITEM,tuple.0,tuple.1]))?;
        } else {
            fixed(db.execute(
                "UPDATE remote_poll_materializations SET step='intended'",
                [],
            ))?;
        }
        let before = resource_image(&w.root);
        let commands = w.server.commands();
        let report = retire(&w, request(&w))?;
        assert_eq!(report.outcome, RetirementDisposition::Refused);
        assert_eq!(report.worktree_removal, RetirementRemoval::None);
        assert!(!report.materialization_settled);
        assert!(resource_image(&w.root) == before);
        assert!(!marker_path(&w)?.exists());
        assert_eq!(w.server.commands(), commands);
        assert_eq!(
            fixed(db.query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            if ambiguous { 2 } else { 1 }
        );
        w.privacy()?;
    }
    Ok(())
}
fn resource_image(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, ResourceNode> {
    fn visit(
        root: &Path,
        path: &Path,
        image: &mut std::collections::BTreeMap<std::path::PathBuf, ResourceNode>,
    ) {
        let metadata = fs::symlink_metadata(path).unwrap();
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        #[cfg(not(unix))]
        let mode = u32::from(metadata.permissions().readonly());
        let node = if metadata.file_type().is_symlink() {
            ResourceNode::Link(fs::read_link(path).unwrap())
        } else if metadata.is_dir() {
            ResourceNode::Directory(mode)
        } else {
            assert!(metadata.is_file());
            // Preserve the exact lease-file entry and its empty content, using
            // metadata while locked (Windows mandatory byte-range lock safety).
            let bytes = if path == root.join(".git/manyhands-operation.lock") {
                assert_eq!(metadata.len(), 0);
                Vec::new()
            } else {
                fs::read(path).unwrap()
            };
            ResourceNode::File(mode, bytes)
        };
        image.insert(path.strip_prefix(root).unwrap().into(), node);
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), image);
            }
        }
    }
    let mut image = std::collections::BTreeMap::new();
    visit(root, root, &mut image);
    image
}
