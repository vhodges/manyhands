//! Standalone domain journeys, not mutation/CLI/batch retirement behavior.
use super::*;
use std::{fs, path::Path};

pub(crate) const CASES: &[ssh_harness::Case] = &[
    (
        "retirement_reentry_intact_prior_worktree_intent_uses_new_proof_and_preserves_history",
        reentry_intact_intent,
    ),
    (
        "retirement_reentry_interrupted_prune_intact_instance_continues",
        reentry_intact_prune,
    ),
    (
        "retirement_reentry_prior_intent_mismatched_dirty_mixed_recreated_instance_preserves",
        reentry_intact_negative,
    ),
    (
        "retirement_reentry_resumed_worktree_reappeared_remote_retains_prior_uncertainty",
        reentry_intact_remote_stop,
    ),
    (
        "retirement_reentry_resumed_worktree_late_identity_stop_settles_only_new_uninvoked_intent",
        reentry_intact_late_stop,
    ),
    (
        "retirement_reentry_resumed_prune_sql_gap_owned_absence_preserves_all_old_uncertainty",
        reentry_resumed_gap_m,
    ),
    (
        "retirement_reentry_public_provisional_bootstrap_matrix",
        reentry_bootstrap,
    ),
    (
        "retirement_reentry_missing_verified_marker_cannot_select_new_identity",
        reentry_missing_verified,
    ),
    (
        "retirement_reentry_replacement_transaction_failure_rolls_back_every_write",
        reentry_rollback,
    ),
    (
        "retirement_reentry_d14_mismatch_rolls_back_replacement_and_preserves_new_intent",
        reentry_d14_mismatch,
    ),
    (
        "retirement_reentry_active_observer_return_cannot_write_after_replacement",
        reentry_active,
    ),
    (
        "retirement_reentry_completed_old_executor_cannot_write_after_replacement",
        reentry_stale_completed,
    ),
    (
        "retirement_reentry_selected_generation_change_is_final_without_network",
        reentry_generation,
    ),
    (
        "retirement_reentry_same_oid_branch_recreation_after_sql_gap_is_owed",
        reentry_recreated_branch,
    ),
    (
        "retirement_reentry_owned_absence_after_known_uninvoked_branch_keeps_branch_none",
        reentry_uninvoked_branch,
    ),
    (
        "retirement_reentry_present_replaces_completed_observer_and_retires",
        reentry_present,
    ),
    (
        "retirement_reentry_branch_only_never_reconstructs_old_lifetime",
        reentry_branch_only,
    ),
    (
        "retirement_reentry_owned_all_absent_is_metadata_only",
        reentry_all_absent,
    ),
    (
        "retirement_reentry_worktree_intent_cannot_invent_branch_removal",
        reentry_worktree_only,
    ),
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
    (
        "retirement_present_p1_initial_git_locks_preserve_resources",
        p1_initial_locks,
    ),
    (
        "retirement_present_p1_final_git_locks_are_no_effect",
        p1_final_locks,
    ),
    #[cfg(unix)]
    (
        "retirement_present_p1_redirected_lock_slots_are_preserved",
        p1_redirected_locks,
    ),
    (
        "retirement_present_p2_initial_malformed_other_head_refuses",
        p2_initial_malformed,
    ),
    (
        "retirement_present_p2_final_malformed_other_head_refuses",
        p2_final_malformed,
    ),
    (
        "retirement_present_p2_initial_indirect_target_head_refuses",
        p2_initial_indirect,
    ),
    (
        "retirement_present_p2_final_indirect_target_head_refuses",
        p2_final_indirect,
    ),
    (
        "retirement_present_p2_valid_other_checkout_allows_prune",
        p2_valid_other,
    ),
    (
        "retirement_present_p1_case_initial_git_locks_preserve_resources",
        p1_case_initial_locks,
    ),
    (
        "retirement_present_p1_case_final_git_locks_are_no_effect",
        p1_case_final_locks,
    ),
    (
        "retirement_present_p2_namespace_initial_masked_alias_refuses",
        p2_namespace_initial,
    ),
    (
        "retirement_present_p2_namespace_final_masked_alias_is_no_effect",
        p2_namespace_final,
    ),
    (
        "retirement_present_p2_namespace_valid_detached_checkout_allows_prune",
        p2_namespace_detached,
    ),
    (
        "retirement_p2_chain_present_late_masked_alias_preserves_branch_debt",
        p2_chain_present_alias,
    ),
    (
        "retirement_p2_chain_branch_only_late_masked_alias_is_no_effect",
        p2_chain_branch_alias,
    ),
    (
        "retirement_p2_chain_initial_case_equivalent_checkout_refuses",
        p2_chain_initial_case,
    ),
    (
        "retirement_p2_chain_preworktree_case_equivalent_checkout_is_no_effect",
        p2_chain_preworktree_case,
    ),
    (
        "retirement_p2_chain_present_late_case_equivalent_checkout_preserves_branch_debt",
        p2_chain_present_case,
    ),
    (
        "retirement_p2_chain_branch_only_late_case_equivalent_checkout_is_no_effect",
        p2_chain_branch_case,
    ),
    (
        "retirement_quality_unicode_initial_head_is_unsupported",
        quality_unicode_initial,
    ),
    (
        "retirement_quality_unicode_preworktree_head_is_no_effect",
        quality_unicode_preworktree,
    ),
    (
        "retirement_quality_unicode_late_branch_head_preserves_worktree_debt",
        quality_unicode_late,
    ),
    (
        "retirement_quality_verified_history_survives_observer_completion_clear_attempt",
        quality_verified_completion,
    ),
    (
        "retirement_quality_unicode_admin_initial_lock_spelling_refuses",
        quality_unicode_admin_initial_lock,
    ),
    (
        "retirement_quality_unicode_admin_final_lock_spelling_is_no_effect",
        quality_unicode_admin_final_lock,
    ),
    (
        "retirement_quality_unicode_admin_initial_directory_component_refuses",
        quality_unicode_admin_initial_directory,
    ),
    (
        "retirement_quality_unicode_admin_final_directory_component_is_no_effect",
        quality_unicode_admin_final_directory,
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

fn reentry_call(
    w: &World,
    request: RetireContextRequest,
    epoch: i64,
) -> Result<ContextRetirementOutcome, FixtureError> {
    fixed(w.service.reenter_context_retirement_authenticated(
        ReenterContextRetirementRequest {
            retirement: request,
            expected_owner_epoch: epoch,
        },
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))
}

fn interrupted_worktree_intent(w: &World, request: &RetireContextRequest, inside_prune: bool) {
    if inside_prune {
        RepositoryService::set_retirement_prune_entry_hook_for_testing(|| {
            std::panic::resume_unwind(Box::new(()))
        });
    } else {
        RepositoryService::set_retirement_worktree_hook_for_testing(|| {
            std::panic::resume_unwind(Box::new(()))
        });
    }
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = w.service.retire_context_authenticated(
                request.clone(),
                w.primary().approval,
                &mut SessionCredentials::new(Provider),
            );
        }))
        .is_err()
    );
}
fn reentry_intact_intent() -> Result<(), FixtureError> {
    reentry_intact(false)
}
fn reentry_intact_prune() -> Result<(), FixtureError> {
    reentry_intact(true)
}
fn reentry_intact(inside_prune: bool) -> Result<(), FixtureError> {
    let w = setup()?;
    let path = add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    interrupted_worktree_intent(&w, &request, inside_prune);
    let old=fixed(w.db()?.query_row("SELECT owner_epoch,receipt_id,primary_oid,proof_kind,observation FROM context_retirement_effect_intents WHERE resource='worktree'",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?))))?;
    assert!(old.4.is_none());
    let fixed_inputs=fixed(w.db()?.query_row("SELECT expected_tip,instance_nonce,initial_primary_oid,materialization_capture FROM context_retirement_records",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))))?;
    let rows = registry_image(&w.db()?);
    assert!(
        w.service
            .retire_context_authenticated(
                request.clone(),
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert!(rows == registry_image(&w.db()?));
    if !inside_prune {
        advance(&w.repo()?, "refs/heads/main", "new-proof-primary")?;
    }
    let result = reentry_call(&w, request.clone(), 1)?;
    assert!(
        matches!(&result,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired && r.worktree_removal==RetirementRemoval::Observed && r.branch_removal==RetirementRemoval::Observed)
    );
    assert!(!path.exists());
    assert!(w.repo()?.find_reference(CONTEXT).is_err());
    assert_eq!(fixed(w.db()?.query_row("SELECT owner_epoch,receipt_id,primary_oid,proof_kind,observation FROM context_retirement_effect_intents WHERE resource='worktree' AND owner_epoch=1",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?))))?,old);
    assert_eq!(fixed(w.db()?.query_row("SELECT expected_tip,instance_nonce,initial_primary_oid,materialization_capture FROM context_retirement_records",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))))?,fixed_inputs);
    assert_eq!(fixed(w.db()?.query_row("SELECT count(*) FROM context_retirement_effect_intents WHERE owner_epoch=2 AND observation='removed'",[],|r|r.get::<_,i64>(0)))?,2);
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        2
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
    assert_eq!(reentry_call(&w, request, 1)?, result);
    w.privacy()
}
fn reentry_intact_negative() -> Result<(), FixtureError> {
    for case in ["missing", "different", "dirty", "mixed", "recreated"] {
        let w = setup()?;
        let path = add_present(&w)?;
        history(&w, true)?;
        let request = request(&w);
        interrupted_worktree_intent(&w, &request, false);
        let marker = w
            .repo()?
            .commondir()
            .join(format!("worktrees/{ITEM}/manyhands-retirement-instance"));
        match case {
            "missing" => fixed(fs::remove_file(marker))?,
            "different" => fixed(fs::write(
                marker,
                format!(
                    "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
                    OperationId::new()
                ),
            ))?,
            "dirty" => fixed(fs::write(path.join("pending-user-bytes"), b"preserve"))?,
            "mixed" => fixed(fs::rename(&path, w.root.join("fixture-moved-worktree")))?,
            _ => {
                fixed(w.repo()?.find_worktree(ITEM).and_then(|w| {
                    w.prune(Some(
                        git2::WorktreePruneOptions::new()
                            .valid(true)
                            .working_tree(true),
                    ))
                }))?;
                add_present(&w)?;
            }
        }
        let resources = resource_image(&w.root);
        let commands = w.server.commands();
        assert_eq!(
            reentry_call(&w, request, 1)?,
            ContextRetirementOutcome::CleanupPending {
                worktree_removal: RetirementRemoval::Possible,
                branch_removal: RetirementRemoval::None
            }
        );
        assert!(resources == resource_image(&w.root));
        assert_eq!(commands, w.server.commands());
        assert!(RepositoryService::open_at(&w.data()).is_ok());
        w.privacy()?;
    }
    Ok(())
}
fn reentry_intact_remote_stop() -> Result<(), FixtureError> {
    reentry_intact_stop(false)
}
fn reentry_intact_late_stop() -> Result<(), FixtureError> {
    reentry_intact_stop(true)
}
fn reentry_intact_stop(late: bool) -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    interrupted_worktree_intent(&w, &request, false);
    let marker = w
        .repo()?
        .commondir()
        .join(format!("worktrees/{ITEM}/manyhands-retirement-instance"));
    let bytes = fixed(fs::read(&marker))?;
    if late {
        let marker = marker.clone();
        RepositoryService::set_retirement_worktree_hook_for_testing(move || {
            fs::write(marker, b"foreign-marker").unwrap();
        });
    } else {
        let bare = w.bare()?;
        let oid = fixed(bare.refname_to_id("refs/heads/main"))?;
        fixed(bare.reference(CONTEXT, oid, false, "reappeared"))?;
    }
    let result = reentry_call(&w, request.clone(), 1)?;
    assert_eq!(
        result,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Possible,
            branch_removal: RetirementRemoval::None
        }
    );
    assert_eq!(fixed(w.db()?.query_row("SELECT observation FROM context_retirement_effect_intents WHERE owner_epoch=1 AND resource='worktree'",[],|r|r.get::<_,Option<String>>(0)))?,None);
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    if late {
        assert_eq!(fixed(w.db()?.query_row("SELECT observation FROM context_retirement_effect_intents WHERE owner_epoch=2 AND resource='worktree'",[],|r|r.get::<_,String>(0)))?,"changed");
        fixed(fs::write(marker, bytes))?;
    } else {
        fixed(
            w.bare()?
                .find_reference(CONTEXT)
                .and_then(|mut r| r.delete()),
        )?;
    }
    assert!(
        matches!(reentry_call(&w,request,2)?,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired)
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn reentry_resumed_gap_m() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    interrupted_worktree_intent(&w, &request, false);
    let marker = w
        .repo()?
        .commondir()
        .join(format!("worktrees/{ITEM}/manyhands-retirement-instance"));
    let bytes = fixed(fs::read(&marker))?;
    let changed = marker.clone();
    RepositoryService::set_retirement_worktree_hook_for_testing(move || {
        fs::write(changed, b"changed").unwrap();
    });
    assert!(matches!(
        reentry_call(&w, request.clone(), 1)?,
        ContextRetirementOutcome::CleanupPending { .. }
    ));
    fixed(fs::write(marker, bytes))?;
    fixed(w.db()?.execute_batch("CREATE TRIGGER resumed_gap BEFORE UPDATE OF observation ON context_retirement_effect_intents WHEN OLD.owner_epoch=3 AND OLD.resource='worktree' BEGIN SELECT RAISE(ABORT,'fixture'); END;"))?;
    assert!(matches!(
        reentry_call(&w, request.clone(), 2)?,
        ContextRetirementOutcome::CleanupPending { .. }
    ));
    fixed(w.db()?.execute_batch("DROP TRIGGER resumed_gap"))?;
    fixed(
        w.repo()?
            .find_reference(CONTEXT)
            .and_then(|mut r| r.delete()),
    )?;
    let commands = w.server.commands();
    let result = reentry_call(&w, request, 3)?;
    assert!(
        matches!(result,ContextRetirementOutcome::Final(r) if r.worktree_removal==RetirementRemoval::Possible && r.branch_removal==RetirementRemoval::None)
    );
    assert_eq!(commands, w.server.commands());
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn reentry_bootstrap() -> Result<(), FixtureError> {
    for case in [
        "missing",
        "missing_recreated",
        "installed",
        "partial",
        "collision",
    ] {
        let w = setup()?;
        let path = add_present(&w)?;
        history(&w, true)?;
        let request = request(&w);
        RepositoryService::set_retirement_bootstrap_hook_for_testing(
            !case.starts_with("missing"),
            || std::panic::resume_unwind(Box::new(())),
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = w.service.retire_context_authenticated(
                    request.clone(),
                    w.primary().approval,
                    &mut SessionCredentials::new(Provider),
                );
            }))
            .is_err()
        );
        let marker = w
            .repo()?
            .commondir()
            .join(format!("worktrees/{ITEM}/manyhands-retirement-instance"));
        assert_eq!(
            fixed(w.db()?.query_row(
                "SELECT instance_verified FROM context_retirement_records",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            0
        );
        if case.starts_with("missing") {
            assert!(!marker.exists());
        }
        if case == "missing_recreated" {
            fixed(w.repo()?.find_worktree(ITEM).and_then(|w| {
                w.prune(Some(
                    git2::WorktreePruneOptions::new()
                        .valid(true)
                        .working_tree(true),
                ))
            }))?;
            add_present(&w)?;
        }
        if case == "partial" {
            fixed(fs::write(
                &marker,
                b"manyhands-retirement-instance-v1\nticket\n",
            ))?;
        }
        if case == "collision" {
            fixed(fs::write(
                &marker,
                format!(
                    "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
                    OperationId::new()
                ),
            ))?;
        }
        let resources = resource_image(&w.root);
        let commands = w.server.commands();
        let result = reentry_call(&w, request.clone(), 1)?;
        if case == "installed" {
            assert!(
                matches!(&result,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired)
            );
            assert!(!path.exists());
        } else {
            assert!(
                matches!(&result,ContextRetirementOutcome::Final(r) if r.code==ResultCode::ExternalChange && r.worktree_removal==RetirementRemoval::None)
            );
            assert!(resources == resource_image(&w.root));
            assert_eq!(commands, w.server.commands());
        }
        assert_eq!(reentry_call(&w, request, 1)?, result);
        assert!(RepositoryService::open_at(&w.data()).is_ok());
        w.privacy()?;
    }
    Ok(())
}
fn reentry_missing_verified() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    interrupt_selected(&w, &request, false);
    let marker = w
        .repo()?
        .commondir()
        .join(format!("worktrees/{ITEM}/manyhands-retirement-instance"));
    fixed(fs::remove_file(marker))?;
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    let result = reentry_call(&w, request, 1)?;
    assert!(
        matches!(result,ContextRetirementOutcome::Final(r) if r.code==ResultCode::ExternalChange)
    );
    assert!(resources == resource_image(&w.root));
    assert_eq!(commands, w.server.commands());
    w.privacy()
}
fn reentry_rollback() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    interrupt_selected(&w, &request, true);
    for injection in [
        "BEFORE UPDATE OF phase ON context_retirement_attempts WHEN NEW.phase='fenced'",
        "BEFORE INSERT ON context_retirement_reentries",
        "BEFORE INSERT ON retirement_observation_pin_releases",
        "BEFORE INSERT ON remote_operation_records WHEN NEW.action='poll'",
    ] {
        fixed(w.db()?.execute_batch(&format!(
            "CREATE TRIGGER reentry_fail {injection} BEGIN SELECT RAISE(ABORT,'fixture'); END;"
        )))?;
        let rows = registry_image(&w.db()?);
        let resources = resource_image(&w.root);
        let commands = w.server.commands();
        assert!(
            w.service
                .reenter_context_retirement_authenticated(
                    ReenterContextRetirementRequest {
                        retirement: request.clone(),
                        expected_owner_epoch: 1
                    },
                    w.primary().approval,
                    &mut SessionCredentials::new(Provider)
                )
                .is_err()
        );
        assert!(rows == registry_image(&w.db()?));
        assert!(resources == resource_image(&w.root));
        assert_eq!(commands, w.server.commands());
        fixed(w.db()?.execute_batch("DROP TRIGGER reentry_fail"))?;
        assert!(RepositoryService::open_at(&w.data()).is_ok());
    }
    assert!(
        matches!(reentry_call(&w,request,1)?,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired)
    );
    w.privacy()
}
fn reentry_d14_mismatch() -> Result<(), FixtureError> {
    for missing in [false, true] {
        let w = setup()?;
        add_present(&w)?;
        history(&w, true)?;
        let old = complete_materialization(&w)?;
        let request = request(&w);
        fixed(w.db()?.execute_batch("CREATE TRIGGER reentry_fail_d14 BEFORE DELETE ON remote_poll_materializations BEGIN SELECT RAISE(IGNORE); END;"))?;
        assert!(matches!(
            fixed(w.service.retire_context_authenticated(
                request.clone(),
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            ))?,
            ContextRetirementOutcome::CleanupPending { .. }
        ));
        fixed(w.db()?.execute_batch(
            "DROP TRIGGER reentry_fail_d14; DELETE FROM remote_poll_materializations;",
        ))?;
        if !missing {
            let new = OperationId::new().to_string();
            fixed(w.db()?.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind,poll_completed_at) SELECT repository_id,?1,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,completed_step,outcome,created_at,updated_at,poll_kind,poll_completed_at FROM remote_operation_records WHERE operation_ulid=?2",rusqlite::params![new,old.0]))?;
            fixed(w.db()?.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,?2,?3,'complete' FROM repositories",rusqlite::params![ITEM,new,old.1]))?;
        }
        let rows = registry_image(&w.db()?);
        let resources = resource_image(&w.root);
        let commands = w.server.commands();
        assert!(
            w.service
                .reenter_context_retirement_authenticated(
                    ReenterContextRetirementRequest {
                        retirement: request,
                        expected_owner_epoch: 1
                    },
                    None,
                    &mut SessionCredentials::new(Provider)
                )
                .is_err()
        );
        assert!(rows == registry_image(&w.db()?));
        assert!(resources == resource_image(&w.root));
        assert_eq!(commands, w.server.commands());
        assert!(RepositoryService::open_at(&w.data()).is_ok());
        w.privacy()?;
    }
    Ok(())
}
fn reentry_active() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    let hold = w.server.hold_advertisement()?;
    let data = w.data();
    let request2 = request.clone();
    let stale = std::thread::scope(|scope| {
        let controller = scope.spawn(move || {
            hold.wait_until_held().unwrap();
            let service = RepositoryService::open_at(&data).unwrap();
            RepositoryService::set_retirement_selected_hook_for_testing(|| {
                std::panic::resume_unwind(Box::new(()))
            });
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = service.reenter_context_retirement_authenticated(
                        ReenterContextRetirementRequest {
                            retirement: request2,
                            expected_owner_epoch: 1,
                        },
                        None,
                        &mut SessionCredentials::new(Provider),
                    );
                }))
                .is_err()
            );
            let rows =
                registry_image(&rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap());
            hold.release().unwrap();
            rows
        });
        let stale = w.service.retire_context_authenticated(
            request.clone(),
            w.primary().approval,
            &mut SessionCredentials::new(Provider),
        );
        let rows = controller.join().unwrap();
        assert!(rows == registry_image(&w.db().unwrap()));
        stale
    });
    assert!(stale.is_err());
    assert!(w.repo()?.find_reference(CONTEXT).is_ok());
    assert!(
        matches!(reentry_call(&w,request,2)?,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired)
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn reentry_stale_completed() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    let data = w.data();
    let second = request.clone();
    let image = std::rc::Rc::new(std::cell::RefCell::new(None));
    let after = image.clone();
    RepositoryService::set_retirement_observed_hook_for_testing(move || {
        let service = RepositoryService::open_at(&data).unwrap();
        let result = service
            .reenter_context_retirement_authenticated(
                ReenterContextRetirementRequest {
                    retirement: second,
                    expected_owner_epoch: 1,
                },
                None,
                &mut SessionCredentials::new(Provider),
            )
            .unwrap();
        assert!(matches!(result, ContextRetirementOutcome::Final(_)));
        *after.borrow_mut() = Some(registry_image(
            &rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap(),
        ));
    });
    assert!(
        w.service
            .retire_context_authenticated(
                request,
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert!(image.borrow().as_ref() == Some(&registry_image(&w.db()?)));
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn reentry_generation() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    interrupt_selected(&w, &request, true);
    fixed(fs::write(
        w.root.join(".manyhands/config.toml"),
        b"invalid = [",
    ))?;
    assert!(
        w.service
            .observe_publication_remote(
                ObservePublicationRemoteRequest {
                    root: w.root.clone(),
                    operation_id: OperationId::new(),
                    invocation: RemotePollInvocation::Explicit,
                    approval: None,
                    restart: false
                },
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    let result = reentry_call(&w, request, 1)?;
    assert!(
        matches!(result,ContextRetirementOutcome::Final(r) if r.code==ResultCode::ExternalChange)
    );
    assert!(resources == resource_image(&w.root));
    assert_eq!(commands, w.server.commands());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn reentry_recreated_branch() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let old = complete_materialization(&w)?;
    let request = request(&w);
    fixed(w.db()?.execute_batch("CREATE TRIGGER reentry_fail_d14 BEFORE DELETE ON remote_poll_materializations BEGIN SELECT RAISE(IGNORE); END;"))?;
    let _ = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    fixed(w.db()?.execute_batch("DROP TRIGGER reentry_fail_d14"))?;
    fixed(
        w.repo()?
            .reference(CONTEXT, fixed(old.1.parse())?, false, "new branch lifetime"),
    )?;
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    assert_eq!(
        reentry_call(&w, request, 1)?,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Observed,
            branch_removal: RetirementRemoval::Possible
        }
    );
    assert!(resources == resource_image(&w.root));
    assert_eq!(commands, w.server.commands());
    assert_materialization(&w, &old)?;
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn reentry_uninvoked_branch() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    let root = w.root.clone();
    RepositoryService::set_retirement_intent_hook_for_testing(move || {
        let git = git2::Repository::open(root).unwrap();
        git.find_reference(CONTEXT).unwrap().delete().unwrap();
    });
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request.clone(),
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Observed,
            branch_removal: RetirementRemoval::None
        }
    );
    let commands = w.server.commands();
    let result = reentry_call(&w, request, 1)?;
    assert!(
        matches!(result,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired && r.worktree_removal==RetirementRemoval::Observed && r.branch_removal==RetirementRemoval::None)
    );
    assert_eq!(commands, w.server.commands());
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn interrupt_selected(w: &World, request: &RetireContextRequest, observed: bool) {
    if observed {
        RepositoryService::set_retirement_observed_hook_for_testing(|| {
            std::panic::resume_unwind(Box::new(()))
        });
    } else {
        RepositoryService::set_retirement_selected_hook_for_testing(|| {
            std::panic::resume_unwind(Box::new(()))
        });
    }
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = w.service.retire_context_authenticated(
                request.clone(),
                w.primary().approval,
                &mut SessionCredentials::new(Provider),
            );
        }))
        .is_err()
    );
}
fn reentry_present() -> Result<(), FixtureError> {
    let w = setup()?;
    let path = add_present(&w)?;
    history(&w, true)?;
    let request = request(&w);
    interrupt_selected(&w, &request, true);
    let rows = registry_image(&w.db()?);
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    assert!(
        w.service
            .retire_context_authenticated(
                request.clone(),
                w.primary().approval,
                &mut SessionCredentials::new(Provider)
            )
            .is_err()
    );
    assert!(rows == registry_image(&w.db()?));
    assert!(resources == resource_image(&w.root));
    assert_eq!(commands, w.server.commands());
    let result = reentry_call(&w, request.clone(), 1)?;
    assert!(
        matches!(&result,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired && r.worktree_removal==RetirementRemoval::Observed && r.branch_removal==RetirementRemoval::Observed)
    );
    assert!(!path.exists());
    assert!(w.repo()?.find_reference(CONTEXT).is_err());
    let db = w.db()?;
    assert_eq!(
        fixed(db.query_row(
            "SELECT owner_epoch FROM context_retirement_records",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        2
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM context_retirement_attempts WHERE phase='fenced'",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        2
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    assert_eq!(reentry_call(&w, request, 1)?, result);
    w.privacy()
}
fn reentry_branch_only() -> Result<(), FixtureError> {
    let w = setup()?;
    history(&w, true)?;
    let request = request(&w);
    interrupt_selected(&w, &request, false);
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    let result = reentry_call(&w, request.clone(), 1)?;
    assert!(
        matches!(&result,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Refused && r.code==ResultCode::ExternalChange && r.worktree_removal==RetirementRemoval::None && r.branch_removal==RetirementRemoval::None)
    );
    assert!(resources == resource_image(&w.root));
    assert_eq!(commands, w.server.commands());
    assert_eq!(reentry_call(&w, request, 1)?, result);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn reentry_all_absent() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let old = complete_materialization(&w)?;
    let request = request(&w);
    fixed(w.db()?.execute_batch("CREATE TRIGGER reentry_ignore_d14 BEFORE DELETE ON remote_poll_materializations BEGIN SELECT RAISE(IGNORE); END;"))?;
    let partial = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    assert_eq!(
        partial,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Observed,
            branch_removal: RetirementRemoval::Possible
        }
    );
    fixed(w.db()?.execute_batch("DROP TRIGGER reentry_ignore_d14"))?;
    fixed(fs::write(
        w.root.join(".manyhands/config.toml"),
        b"invalid configuration = [",
    ))?;
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    let result = reentry_call(&w, request.clone(), 1)?;
    assert!(
        matches!(&result,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Retired && r.worktree_removal==RetirementRemoval::Observed && r.branch_removal==RetirementRemoval::Possible && r.materialization_settled)
    );
    assert!(resources == resource_image(&w.root));
    assert_eq!(commands, w.server.commands());
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT count(*) FROM remote_poll_materializations",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    fixed(w.repo()?.reference(
        CONTEXT,
        fixed(old.1.parse())?,
        false,
        "same-OID new lifetime",
    ))?;
    add_present(&w)?;
    let resources = resource_image(&w.root);
    let rows = registry_image(&w.db()?);
    assert_eq!(reentry_call(&w, request, 1)?, result);
    assert!(resources == resource_image(&w.root));
    assert!(rows == registry_image(&w.db()?));
    w.privacy()
}
fn reentry_worktree_only() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let old = complete_materialization(&w)?;
    let request = request(&w);
    fixed(w.db()?.execute_batch("CREATE TRIGGER reentry_ignore_w_observation BEFORE UPDATE OF observation ON context_retirement_effect_intents WHEN OLD.resource='worktree' BEGIN SELECT RAISE(ABORT,'fixture'); END;"))?;
    let _ = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    fixed(
        w.db()?
            .execute_batch("DROP TRIGGER reentry_ignore_w_observation"),
    )?;
    let resources = resource_image(&w.root);
    let commands = w.server.commands();
    assert_eq!(
        reentry_call(&w, request.clone(), 1)?,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Possible,
            branch_removal: RetirementRemoval::None
        }
    );
    assert!(resources == resource_image(&w.root));
    assert_eq!(commands, w.server.commands());
    assert_materialization(&w, &old)?;
    fixed(
        w.repo()?
            .find_reference(CONTEXT)
            .and_then(|mut r| r.delete()),
    )?;
    fixed(fs::write(
        w.root.join(".manyhands/config.toml"),
        b"invalid = [",
    ))?;
    let result = reentry_call(&w, request, 2)?;
    assert!(
        matches!(result,ContextRetirementOutcome::Final(r) if r.worktree_removal==RetirementRemoval::Possible && r.branch_removal==RetirementRemoval::None && r.materialization_settled)
    );
    assert_eq!(
        fixed(w.db()?.query_row(
            "SELECT completed_step FROM operation_records WHERE action='retire_context'",
            [],
            |r| r.get::<_, String>(0)
        ))?,
        "context_branch_released"
    );
    assert_eq!(fixed(w.db()?.query_row("SELECT count(*) FROM context_retirement_absence_receipts m JOIN context_retirement_records c ON c.operation_ulid=m.operation_ulid AND c.owner_epoch=m.owner_epoch WHERE m.worktree_removal='possible' AND m.branch_removal='none' AND c.absence_basis='owned_intent'",[],|r|r.get::<_,i64>(0)))?,1);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
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
        // Isolated corruption injection, not an ordinary producer UPDATE: the
        // forward-only guard now prevents clearing verified history. Restore
        // its exact original DDL before snapshot/audit; never weaken assertions.
        let sql: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='retirement_instance_verified_forward'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        db.execute_batch("DROP TRIGGER retirement_instance_verified_forward")
            .unwrap();
        db.execute(
            "UPDATE context_retirement_records SET instance_verified=0",
            [],
        )
        .unwrap();
        db.execute_batch(&sql).unwrap();
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

fn p1_initial_locks() -> Result<(), FixtureError> {
    p1_locks(false)
}
fn p1_final_locks() -> Result<(), FixtureError> {
    p1_locks(true)
}
fn p1_locks(final_boundary: bool) -> Result<(), FixtureError> {
    for member in [
        "index.lock",
        "HEAD.lock",
        "config.worktree.lock",
        "ORIG_HEAD.lock",
        "FETCH_HEAD.lock",
        "refs/worktree/private.lock",
        "logs/HEAD.lock",
    ] {
        pre_prune_refusal(
            final_boundary,
            move |w| {
                let path = w
                    .repo()
                    .unwrap()
                    .commondir()
                    .join("worktrees")
                    .join(ITEM)
                    .join(member);
                Box::new(move || {
                    fs::create_dir_all(path.parent().unwrap()).unwrap();
                    fs::write(path, b"pending Git writer fixture bytes").unwrap();
                })
            },
            move |w| {
                let path = w
                    .repo()
                    .unwrap()
                    .commondir()
                    .join("worktrees")
                    .join(ITEM)
                    .join(member);
                assert_eq!(
                    fs::read(&path).unwrap(),
                    b"pending Git writer fixture bytes"
                );
                fs::remove_file(path).unwrap();
            },
        )?;
    }
    Ok(())
}
fn p1_case_initial_locks() -> Result<(), FixtureError> {
    p1_case_locks(false)
}
fn p1_case_final_locks() -> Result<(), FixtureError> {
    p1_case_locks(true)
}
fn p1_case_locks(final_boundary: bool) -> Result<(), FixtureError> {
    // Deliberately runs on case-sensitive Linux too: preserve these slots
    // conservatively instead of guessing the filesystem's case-folding policy.
    for member in [
        "index.LOCK",
        "HEAD.LoCk",
        "config.worktree.lOcK",
        "ORIG_HEAD.LOCK",
        "FETCH_HEAD.LoCk",
        "refs/worktree/private.LoCk",
        "logs/HEAD.LOCK",
    ] {
        pre_prune_refusal(
            final_boundary,
            move |w| {
                let path = w
                    .repo()
                    .unwrap()
                    .commondir()
                    .join("worktrees")
                    .join(ITEM)
                    .join(member);
                Box::new(move || {
                    fs::create_dir_all(path.parent().unwrap()).unwrap();
                    fs::write(path, b"case-equivalent pending Git writer bytes").unwrap();
                })
            },
            move |w| {
                let path = w
                    .repo()
                    .unwrap()
                    .commondir()
                    .join("worktrees")
                    .join(ITEM)
                    .join(member);
                assert!(fs::symlink_metadata(&path).unwrap().file_type().is_file());
                assert_eq!(
                    fs::read(&path).unwrap(),
                    b"case-equivalent pending Git writer bytes"
                );
                fs::remove_file(path).unwrap();
            },
        )?;
    }
    Ok(())
}
#[cfg(unix)]
fn p1_redirected_locks() -> Result<(), FixtureError> {
    for final_boundary in [false, true] {
        for directory in [false, true] {
            pre_prune_refusal(
                final_boundary,
                move |w| {
                    let path = w
                        .repo()
                        .unwrap()
                        .commondir()
                        .join("worktrees")
                        .join(ITEM)
                        .join("index.lock");
                    let backing = w.data().join("foreign-lock-fixture");
                    fs::write(&backing, b"preserved foreign writer bytes").unwrap();
                    Box::new(move || {
                        if directory {
                            fs::create_dir(path).unwrap();
                        } else {
                            std::os::unix::fs::symlink(backing, path).unwrap();
                        }
                    })
                },
                move |w| {
                    let path = w
                        .repo()
                        .unwrap()
                        .commondir()
                        .join("worktrees")
                        .join(ITEM)
                        .join("index.lock");
                    assert_eq!(
                        fs::read(w.data().join("foreign-lock-fixture")).unwrap(),
                        b"preserved foreign writer bytes"
                    );
                    if directory {
                        fs::remove_dir(path).unwrap();
                    } else {
                        assert!(
                            fs::symlink_metadata(&path)
                                .unwrap()
                                .file_type()
                                .is_symlink()
                        );
                        fs::remove_file(path).unwrap();
                    }
                },
            )?;
        }
    }
    Ok(())
}
const OTHER_BRANCH: &str = "refs/heads/retirement-other";
fn add_other_checkout(w: &World) -> (std::path::PathBuf, std::path::PathBuf) {
    let repo = w.repo().unwrap();
    let tip = repo.head().unwrap().target().unwrap();
    let reference = repo
        .reference(OTHER_BRANCH, tip, false, "unrelated fixture checkout")
        .unwrap();
    let path = w.root.join(".manyhands/worktrees/unrelated-checkout");
    repo.worktree(
        "retirement-other",
        &path,
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    )
    .unwrap();
    (path, repo.commondir().join("worktrees/retirement-other"))
}
fn p2_initial_malformed() -> Result<(), FixtureError> {
    p2_invalid_head(false, false)
}
fn p2_final_malformed() -> Result<(), FixtureError> {
    p2_invalid_head(true, false)
}
fn p2_initial_indirect() -> Result<(), FixtureError> {
    p2_invalid_head(false, true)
}
fn p2_final_indirect() -> Result<(), FixtureError> {
    p2_invalid_head(true, true)
}
fn p2_invalid_head(final_boundary: bool, indirect: bool) -> Result<(), FixtureError> {
    pre_prune_refusal(
        final_boundary,
        move |w| {
            let (_, admin) = add_other_checkout(w);
            let head = admin.join("HEAD");
            let root = w.root.clone();
            Box::new(move || {
                let data = if indirect {
                    let git = git2::Repository::open(root).unwrap();
                    let alias = git
                        .reference_symbolic(
                            "refs/heads/retirement-alias",
                            CONTEXT,
                            false,
                            "target alias fixture",
                        )
                        .unwrap();
                    assert_eq!(alias.symbolic_target(), Some(CONTEXT));
                    b"ref: refs/heads/retirement-alias\n".as_slice()
                } else {
                    b"invalid\n".as_slice()
                };
                fs::write(head, data).unwrap();
            })
        },
        move |w| {
            fs::write(
                w.repo()
                    .unwrap()
                    .commondir()
                    .join("worktrees/retirement-other/HEAD"),
                format!("ref: {OTHER_BRANCH}\n"),
            )
            .unwrap();
        },
    )
}
fn p2_valid_other() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    let (other, admin) = add_other_checkout(&w);
    history(&w, true)?;
    let other_before = resource_image(&other);
    let admin_before = resource_image(&admin);
    let tip = fixed(w.repo()?.refname_to_id(OTHER_BRANCH))?;
    let report = retire(&w, request(&w))?;
    assert_eq!(report.outcome, RetirementDisposition::Retired);
    assert_eq!(report.worktree_removal, RetirementRemoval::Observed);
    assert_eq!(report.branch_removal, RetirementRemoval::Observed);
    assert!(resource_image(&other) == other_before);
    assert!(resource_image(&admin) == admin_before);
    assert_eq!(fixed(w.repo()?.refname_to_id(OTHER_BRANCH))?, tip);
    w.privacy()
}

fn p2_namespace_initial() -> Result<(), FixtureError> {
    p2_namespace_alias(false)
}
fn p2_namespace_final() -> Result<(), FixtureError> {
    p2_namespace_alias(true)
}
fn p2_namespace_alias(final_boundary: bool) -> Result<(), FixtureError> {
    for shadow in [
        "refs/worktree/shadow",
        "refs/bisect/shadow",
        "refs/rewritten/shadow",
        "SHADOW_HEAD",
    ] {
        pre_prune_refusal(
            final_boundary,
            move |w| {
                let (path, admin) = add_other_checkout(w);
                let root = w.root.clone();
                Box::new(move || {
                    let git = git2::Repository::open(root).unwrap();
                    let tip = git.refname_to_id(CONTEXT).unwrap();
                    let mask = git.commondir().join(shadow);
                    fs::create_dir_all(mask.parent().unwrap()).unwrap();
                    fs::write(mask, format!("{tip}\n")).unwrap();
                    let actual = admin.join(shadow);
                    fs::create_dir_all(actual.parent().unwrap()).unwrap();
                    fs::write(actual, format!("ref: {CONTEXT}\n")).unwrap();
                    fs::write(admin.join("HEAD"), format!("ref: {shadow}\n")).unwrap();

                    // Prove the counterexample's actual checkout namespace, not
                    // just HEAD text: primary sees a direct mask, linked sees
                    // an alias which resolves to the selected target context.
                    let mask = git.find_reference(shadow).unwrap();
                    assert_eq!(mask.symbolic_target(), None);
                    assert_eq!(mask.target(), Some(tip));
                    let linked = git2::Repository::open(path).unwrap();
                    assert_eq!(
                        linked.find_reference("HEAD").unwrap().symbolic_target(),
                        Some(shadow)
                    );
                    assert_eq!(
                        linked.find_reference(shadow).unwrap().symbolic_target(),
                        Some(CONTEXT)
                    );
                    let resolved = linked.head().unwrap();
                    assert_eq!(resolved.name(), Some(CONTEXT));
                    assert_eq!(resolved.target(), Some(tip));
                })
            },
            move |w| {
                fs::write(
                    w.repo()
                        .unwrap()
                        .commondir()
                        .join("worktrees/retirement-other/HEAD"),
                    format!("ref: {OTHER_BRANCH}\n"),
                )
                .unwrap();
            },
        )?;
    }
    Ok(())
}
fn p2_namespace_detached() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    let (other, admin) = add_other_checkout(&w);
    let tip = fixed(w.repo()?.refname_to_id(OTHER_BRANCH))?;
    fixed(git2::Repository::open(&other).and_then(|repo| repo.set_head_detached(tip)))?;
    assert!(
        fixed(git2::Repository::open(&other))?
            .head_detached()
            .unwrap()
    );
    history(&w, true)?;
    let other_before = resource_image(&other);
    let admin_before = resource_image(&admin);
    let report = retire(&w, request(&w))?;
    assert_eq!(report.outcome, RetirementDisposition::Retired);
    assert_eq!(report.worktree_removal, RetirementRemoval::Observed);
    assert_eq!(report.branch_removal, RetirementRemoval::Observed);
    assert!(resource_image(&other) == other_before);
    assert!(resource_image(&admin) == admin_before);
    assert_eq!(fixed(w.repo()?.refname_to_id(OTHER_BRANCH))?, tip);
    w.privacy()
}

fn p2_chain_present_alias() -> Result<(), FixtureError> {
    p2_chain_late(true, false)
}
fn p2_chain_branch_alias() -> Result<(), FixtureError> {
    p2_chain_late(false, false)
}
fn p2_chain_present_case() -> Result<(), FixtureError> {
    p2_chain_late(true, true)
}
fn p2_chain_branch_case() -> Result<(), FixtureError> {
    p2_chain_late(false, true)
}
fn p2_chain_initial_case() -> Result<(), FixtureError> {
    p2_chain_preworktree_case_at(false)
}
fn p2_chain_preworktree_case() -> Result<(), FixtureError> {
    p2_chain_preworktree_case_at(true)
}
fn p2_chain_case_reference(upper: bool) -> String {
    format!(
        "refs/heads/{}/{ITEM}",
        if upper {
            "MANYHANDS/TICKET"
        } else {
            "Manyhands/ticket"
        }
    )
}
fn p2_chain_case_fault(root: &Path, other: &Path, admin: &Path, reference: &str) {
    let git = git2::Repository::open(root).unwrap();
    let tip = git.refname_to_id(CONTEXT).unwrap();
    assert_ne!(reference, CONTEXT);
    assert!(reference.eq_ignore_ascii_case(CONTEXT));
    let slot = git.commondir().join(reference);
    fs::create_dir_all(slot.parent().unwrap()).unwrap();
    fs::write(slot, format!("{tip}\n")).unwrap();
    fs::write(admin.join("HEAD"), format!("ref: {reference}\n")).unwrap();
    let linked = git2::Repository::open(other).unwrap();
    assert_eq!(
        linked.find_reference("HEAD").unwrap().symbolic_target(),
        Some(reference)
    );
    assert_eq!(linked.head().unwrap().target(), Some(tip));
    // Linux has distinct slots here; refusal must be conservative on Linux too.
    assert_eq!(git.refname_to_id(reference).unwrap(), tip);
}
fn p2_chain_alias_fault(root: &Path, other: &Path, admin: &Path, shadow: &str) {
    let git = git2::Repository::open(root).unwrap();
    let tip = git.refname_to_id(CONTEXT).unwrap();
    let other_tip = git.find_commit(tip).unwrap().parent_id(0).unwrap();
    assert_ne!(other_tip, tip);
    let slot = git.commondir().join(shadow);
    fs::create_dir_all(slot.parent().unwrap()).unwrap();
    fs::write(slot, format!("{other_tip}\n")).unwrap();
    let slot = admin.join(shadow);
    fs::create_dir_all(slot.parent().unwrap()).unwrap();
    fs::write(slot, format!("ref: {CONTEXT}\n")).unwrap();
    fs::write(admin.join("HEAD"), format!("ref: {shadow}\n")).unwrap();
    assert_eq!(
        git.find_reference(shadow).unwrap().target(),
        Some(other_tip)
    );
    let linked = git2::Repository::open(other).unwrap();
    assert_eq!(
        linked.find_reference(shadow).unwrap().symbolic_target(),
        Some(CONTEXT)
    );
    let actual = linked.head().unwrap();
    assert_eq!(actual.name(), Some(CONTEXT));
    assert_eq!(actual.target(), Some(tip));
}
fn p2_chain_preworktree_case_at(final_boundary: bool) -> Result<(), FixtureError> {
    for upper in [false, true] {
        pre_prune_refusal(
            final_boundary,
            move |w| {
                let (other, admin) = add_other_checkout(w);
                let root = w.root.clone();
                Box::new(move || {
                    p2_chain_case_fault(&root, &other, &admin, &p2_chain_case_reference(upper))
                })
            },
            |w| {
                fs::write(
                    w.repo()
                        .unwrap()
                        .commondir()
                        .join("worktrees/retirement-other/HEAD"),
                    format!("ref: {OTHER_BRANCH}\n"),
                )
                .unwrap();
            },
        )?;
    }
    Ok(())
}
fn p2_chain_late(present: bool, case_equivalent: bool) -> Result<(), FixtureError> {
    let variants = if case_equivalent {
        vec![
            p2_chain_case_reference(false),
            p2_chain_case_reference(true),
        ]
    } else {
        [
            "refs/worktree/shadow",
            "refs/bisect/shadow",
            "refs/rewritten/shadow",
            "SHADOW_HEAD",
        ]
        .map(str::to_owned)
        .to_vec()
    };
    for variant in variants {
        let w = setup()?;
        if present {
            add_present(&w)?;
        } else {
            fixed(fs::create_dir_all(w.root.join(".manyhands/worktrees")))?;
        }
        let (other, admin) = add_other_checkout(&w);
        history(&w, true)?;
        let materialization = complete_materialization(&w)?;
        let request = request(&w);
        let tip = fixed(w.repo()?.refname_to_id(CONTEXT))?;
        let root = w.root.clone();
        let data = w.data();
        let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
        let snapshot = captured.clone();
        RepositoryService::set_retirement_intent_hook_for_testing(move || {
            let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
            let (wflag,bstep):(String,Option<String>)=db.query_row("SELECT c.worktree_removal,o.completed_step FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
            assert_eq!(wflag, if present { "observed" } else { "none" });
            assert_eq!(
                bstep.as_deref(),
                if present {
                    Some("context_worktree_released")
                } else {
                    None
                }
            );
            assert!(!root.join(".manyhands/worktrees").join(ITEM).exists());
            assert_eq!(db.query_row("SELECT count(*) FROM context_retirement_effect_intents WHERE resource='branch' AND observation IS NULL",[],|r|r.get::<_,i64>(0)).unwrap(),1);
            if case_equivalent {
                p2_chain_case_fault(&root, &other, &admin, &variant);
            } else {
                p2_chain_alias_fault(&root, &other, &admin, &variant);
            }
            *snapshot.borrow_mut() = Some((resource_image(&root), registry_image(&db)));
        });
        let result = fixed(w.service.retire_context_authenticated(
            request.clone(),
            w.primary().approval,
            &mut SessionCredentials::new(Provider),
        ))?;
        let snapshot = captured.borrow();
        let (resources, registry) = snapshot.as_ref().unwrap();
        let expected = ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Observed,
            branch_removal: RetirementRemoval::None,
        };
        ssh_harness::observation(&[
            601,
            present as u128,
            case_equivalent as u128,
            (result == expected) as u128,
            (resource_image(&w.root) == *resources) as u128,
        ]);
        if present {
            assert_eq!(result, expected);
        } else {
            assert!(
                matches!(&result,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Refused && r.code==ResultCode::ExternalChange && r.worktree_removal==RetirementRemoval::None && r.branch_removal==RetirementRemoval::None)
            );
        }
        assert!(resource_image(&w.root) == *resources);
        assert_eq!(fixed(w.repo()?.refname_to_id(CONTEXT))?, tip);
        assert_materialization(&w, &materialization)?;
        let db = w.db()?;
        let after = registry_image(&db);
        for (table, rows) in registry {
            if ![
                "operation_records",
                "context_retirement_records",
                "context_retirement_effect_intents",
                "context_retirement_attempts",
                "retirement_observation_pins",
                "retirement_observation_pin_releases",
            ]
            .contains(&table.as_str())
            {
                assert!(
                    after.get(table) == Some(rows),
                    "unrelated row changed after late checkout refusal"
                );
            }
        }
        assert_eq!(fixed(db.query_row("SELECT count(*) FROM context_retirement_effect_intents WHERE resource='branch' AND observation='changed'",[],|r|r.get::<_,i64>(0)))?,1);
        assert_eq!(fixed(db.query_row("SELECT count(*) FROM context_retirement_effect_intents WHERE resource='branch' AND observation='removed'",[],|r|r.get::<_,i64>(0)))?,0);
        assert_eq!(
            fixed(db.query_row(
                "SELECT count(*) FROM retirement_observation_pins",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            i64::from(present)
        );
        if present {
            let row:(String,String,String,String)=fixed(db.query_row("SELECT c.phase,c.worktree_removal,c.branch_removal,o.completed_step FROM context_retirement_records c JOIN operation_records o ON o.id=c.local_record_id",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))))?;
            assert_eq!(
                row,
                (
                    "worktree_removed".into(),
                    "observed".into(),
                    "none".into(),
                    "context_worktree_released".into()
                )
            );
        }
        // Repair removes the current eligibility blocker, never restores the
        // spent same-live B permit. Owed replay must not advertise or delete.
        fixed(fs::write(
            w.repo()?
                .commondir()
                .join("worktrees/retirement-other/HEAD"),
            format!("ref: {OTHER_BRANCH}\n"),
        ))?;
        let resources = resource_image(&w.root);
        let rows = registry_image(&db);
        let commands = w.server.commands();
        let replay = w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider),
        );
        if present {
            assert!(matches!(replay,Err(e) if e.kind==RepositoryErrorKind::RecoveryRequired));
        } else {
            assert_eq!(fixed(replay)?, result);
        }
        assert!(resource_image(&w.root) == resources);
        assert!(registry_image(&db) == rows);
        assert_eq!(w.server.commands(), commands);
        assert!(RepositoryService::open_at(&w.data()).is_ok());
        w.privacy()?;
    }
    Ok(())
}

fn quality_unicode_initial() -> Result<(), FixtureError> {
    quality_unicode_before_worktree(false)
}
fn quality_unicode_preworktree() -> Result<(), FixtureError> {
    quality_unicode_before_worktree(true)
}
fn quality_unicode_reference() -> String {
    format!("refs/heads/manyhand\u{017f}/ticket/{ITEM}")
}
fn quality_unicode_fault(root: &Path, other: &Path, admin: &Path) {
    let reference = quality_unicode_reference();
    assert!(git2::Reference::is_valid_name(&reference));
    assert!(!reference.eq_ignore_ascii_case(CONTEXT));
    let git = git2::Repository::open(root).unwrap();
    let tip = git.refname_to_id(CONTEXT).unwrap();
    let slot = git.commondir().join(&reference);
    fs::create_dir_all(slot.parent().unwrap()).unwrap();
    fs::write(slot, format!("{tip}\n")).unwrap();
    fs::write(admin.join("HEAD"), format!("ref: {reference}\n")).unwrap();
    let linked = git2::Repository::open(other).unwrap();
    assert_eq!(
        linked.find_reference("HEAD").unwrap().symbolic_target(),
        Some(reference.as_str())
    );
    assert_eq!(linked.head().unwrap().target(), Some(tip));
    assert_eq!(
        git.find_reference(&reference).unwrap().symbolic_target(),
        None
    );
    // This fixture proves a Git-valid non-ASCII direct binding on Linux. It
    // does NOT enable casefold or claim these two slots alias on this volume.
}
fn quality_unicode_before_worktree(final_boundary: bool) -> Result<(), FixtureError> {
    pre_prune_refusal(
        final_boundary,
        |w| {
            let (other, admin) = add_other_checkout(w);
            let root = w.root.clone();
            Box::new(move || quality_unicode_fault(&root, &other, &admin))
        },
        |w| {
            fs::write(
                w.repo()
                    .unwrap()
                    .commondir()
                    .join("worktrees/retirement-other/HEAD"),
                format!("ref: {OTHER_BRANCH}\n"),
            )
            .unwrap();
        },
    )
}
fn quality_unicode_late() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    let (other, admin) = add_other_checkout(&w);
    history(&w, true)?;
    let materialization = complete_materialization(&w)?;
    let request = request(&w);
    let root = w.root.clone();
    let data = w.data();
    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let snapshot = captured.clone();
    RepositoryService::set_retirement_intent_hook_for_testing(move || {
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT worktree_removal FROM context_retirement_records",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "observed"
        );
        quality_unicode_fault(&root, &other, &admin);
        *snapshot.borrow_mut() = Some((resource_image(&root), registry_image(&db)));
    });
    let result = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    let snapshot = captured.borrow();
    let (resources, rows) = snapshot.as_ref().unwrap();
    assert_eq!(
        result,
        ContextRetirementOutcome::CleanupPending {
            worktree_removal: RetirementRemoval::Observed,
            branch_removal: RetirementRemoval::None
        }
    );
    assert!(resource_image(&w.root) == *resources);
    assert_materialization(&w, &materialization)?;
    let db = w.db()?;
    let after = registry_image(&db);
    for (table, image) in rows {
        if ![
            "context_retirement_records",
            "context_retirement_effect_intents",
        ]
        .contains(&table.as_str())
        {
            assert!(
                after.get(table) == Some(image),
                "only uninvoked B intent may settle after W"
            );
        }
    }
    assert_eq!(
        fixed(db.query_row(
            "SELECT observation FROM context_retirement_effect_intents WHERE resource='branch'",
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
        1
    );
    fixed(fs::write(
        w.repo()?
            .commondir()
            .join("worktrees/retirement-other/HEAD"),
        format!("ref: {OTHER_BRANCH}\n"),
    ))?;
    let resources = resource_image(&w.root);
    let rows = registry_image(&db);
    let commands = w.server.commands();
    assert!(
        matches!(w.service.retire_context_authenticated(request,w.primary().approval,&mut SessionCredentials::new(Provider)),Err(e) if e.kind==RepositoryErrorKind::RecoveryRequired)
    );
    assert!(resource_image(&w.root) == resources);
    assert!(registry_image(&db) == rows);
    assert_eq!(w.server.commands(), commands);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}
fn quality_verified_completion() -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let materialization = complete_materialization(&w)?;
    fixed(w.db()?.execute_batch("CREATE TRIGGER quality_clear_verified_at_bound_completion AFTER UPDATE OF phase ON remote_operation_records WHEN NEW.phase='completed' AND NEW.poll_kind='observe' AND EXISTS(SELECT 1 FROM context_retirement_attempts a WHERE a.observer_operation_ulid=NEW.operation_ulid) BEGIN UPDATE context_retirement_records SET instance_verified=0 WHERE operation_ulid IN (SELECT operation_ulid FROM context_retirement_attempts WHERE observer_operation_ulid=NEW.operation_ulid); END;"))?;
    let resources = resource_image(&w.root);
    let request = request(&w);
    let report = retire(&w, request.clone())?;
    assert_eq!(report.outcome, RetirementDisposition::Refused);
    assert_eq!(report.code, ResultCode::ExternalChange);
    assert_eq!(report.worktree_removal, RetirementRemoval::None);
    assert_eq!(report.branch_removal, RetirementRemoval::None);
    let db = w.db()?;
    assert_eq!(
        fixed(db.query_row(
            "SELECT instance_verified FROM context_retirement_records",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1,
        "completed observer cannot erase verified history"
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM retirement_advertisement_provenance",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM context_retirement_effect_intents",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert_materialization(&w, &materialization)?;
    // Fresh installation is the only authorized filesystem delta before this
    // failed completion: retain its full valid marker rather than roll it back.
    let marker = marker_path(&w)?;
    let nonce: String = fixed(db.query_row(
        "SELECT instance_nonce FROM context_retirement_records",
        [],
        |r| r.get(0),
    ))?;
    assert_eq!(
        fixed(fs::read(&marker))?,
        format!("manyhands-retirement-instance-v1\nticket\n{ITEM}\n{nonce}\n").into_bytes()
    );
    let mut after = resource_image(&w.root);
    assert!(
        after
            .remove(marker.strip_prefix(&w.root).unwrap())
            .is_some()
    );
    assert!(after == resources);
    let resources = resource_image(&w.root);
    let rows = registry_image(&db);
    let commands = w.server.commands();
    assert_eq!(retire(&w, request)?, report);
    assert!(resource_image(&w.root) == resources);
    assert!(registry_image(&db) == rows);
    assert_eq!(w.server.commands(), commands);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
}

fn quality_unicode_admin_initial_lock() -> Result<(), FixtureError> {
    quality_unicode_admin(false, false)
}
fn quality_unicode_admin_final_lock() -> Result<(), FixtureError> {
    quality_unicode_admin(true, false)
}
fn quality_unicode_admin_initial_directory() -> Result<(), FixtureError> {
    quality_unicode_admin(false, true)
}
fn quality_unicode_admin_final_directory() -> Result<(), FixtureError> {
    quality_unicode_admin(true, true)
}
fn quality_unicode_admin(final_boundary: bool, directory: bool) -> Result<(), FixtureError> {
    let member = if directory {
        "refs/cafe\u{0301}/pending"
    } else {
        "index.loc\u{212a}"
    };
    // Kelvin lock spelling / decomposed directory component on ordinary Linux:
    // this proves conservative unsupported-name refusal, NOT enabled casefold.
    pre_prune_refusal(
        final_boundary,
        move |w| {
            let path = w
                .repo()
                .unwrap()
                .commondir()
                .join("worktrees")
                .join(ITEM)
                .join(member);
            Box::new(move || {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, b"Unicode-spelled pending writer fixture bytes").unwrap();
            })
        },
        move |w| {
            let path = w
                .repo()
                .unwrap()
                .commondir()
                .join("worktrees")
                .join(ITEM)
                .join(member);
            assert!(fs::symlink_metadata(&path).unwrap().file_type().is_file());
            assert_eq!(
                fs::read(&path).unwrap(),
                b"Unicode-spelled pending writer fixture bytes"
            );
            fs::remove_file(&path).unwrap();
            if directory {
                assert!(
                    fs::symlink_metadata(path.parent().unwrap())
                        .unwrap()
                        .is_dir()
                );
                fs::remove_dir(path.parent().unwrap()).unwrap();
            }
        },
    )
}

// Shared negative journey: capture all resource and registry rows at the actual
// initial/final boundary, allowing only the exact metadata-only TF/Z changes.
fn pre_prune_refusal(
    final_boundary: bool,
    make_fault: impl FnOnce(&World) -> Box<dyn FnOnce()>,
    repair: impl FnOnce(&World),
) -> Result<(), FixtureError> {
    let w = setup()?;
    add_present(&w)?;
    history(&w, true)?;
    let materialization = complete_materialization(&w)?;
    let request = request(&w);
    let root = w.root.clone();
    let data = w.data();
    let fault = make_fault(&w);
    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let snapshot = captured.clone();
    let fault = move || {
        fault();
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        *snapshot.borrow_mut() = Some((resource_image(&root), registry_image(&db)));
    };
    if final_boundary {
        RepositoryService::set_retirement_worktree_hook_for_testing(fault);
    } else {
        fault();
    }
    let outcome = fixed(w.service.retire_context_authenticated(
        request.clone(),
        w.primary().approval,
        &mut SessionCredentials::new(Provider),
    ))?;
    let captured = captured.borrow();
    let (resources, registry) = captured.as_ref().unwrap();
    ssh_harness::observation(&[
        501,
        final_boundary as u128,
        matches!(&outcome,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Refused)
            as u128,
        (resource_image(&w.root) == *resources) as u128,
    ]);
    let expected = if final_boundary {
        ResultCode::ExternalChange
    } else {
        ResultCode::RecoveryRequired
    };
    assert!(
        matches!(&outcome,ContextRetirementOutcome::Final(r) if r.outcome==RetirementDisposition::Refused && r.code==expected && r.worktree_removal==RetirementRemoval::None && r.branch_removal==RetirementRemoval::None && !r.materialization_settled)
    );
    assert!(
        resource_image(&w.root) == *resources,
        "no prune, ref or marker rewrite at a refused boundary"
    );
    let after = registry_image(&w.db()?);
    for (table, rows) in registry {
        if ![
            "operation_records",
            "context_retirement_records",
            "context_retirement_attempts",
            "context_retirement_effect_intents",
            "retirement_observation_pins",
            "retirement_observation_pin_releases",
        ]
        .contains(&table.as_str())
        {
            assert!(
                after.get(table) == Some(rows),
                "unrelated row image must remain unchanged"
            );
        }
    }
    let db = w.db()?;
    assert_eq!(fixed(db.query_row("SELECT count(*) FROM context_retirement_effect_intents WHERE resource='worktree' AND observation='changed'",[],|r|r.get::<_,i64>(0)))?,i64::from(final_boundary));
    assert_eq!(fixed(db.query_row("SELECT count(*) FROM context_retirement_effect_intents WHERE observation='removed' OR resource='branch'",[],|r|r.get::<_,i64>(0)))?,0);
    assert_eq!(
        fixed(db.query_row(
            "SELECT count(*) FROM retirement_observation_pins",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        0
    );
    assert_materialization(&w, &materialization)?;
    repair(&w);
    let resources = resource_image(&w.root);
    let registry = registry_image(&db);
    let commands = w.server.commands();
    assert_eq!(
        fixed(w.service.retire_context_authenticated(
            request,
            w.primary().approval,
            &mut SessionCredentials::new(Provider)
        ))?,
        outcome
    );
    assert!(resource_image(&w.root) == resources);
    assert!(registry_image(&db) == registry);
    assert_eq!(w.server.commands(), commands);
    assert!(RepositoryService::open_at(&w.data()).is_ok());
    w.privacy()
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
