//! `poll configure`, `poll pause` and `poll resume`: a `c5_poll_` case per row of the design's policy table, and the
//! rows of last commit wins. They need no server: each world is one local
//! repository.
//!
//! `poll once`: a `c5_poll_once_` case per row of the design's frozen
//! table, on a two-clone world, in the last part of this file.

use std::path::Path;

use super::*;
use lifecycle_world::*;
use repository::request_store::RequestRecord;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    ("c5_poll_pause_success", pause_success),
    ("c5_poll_pause_already_so", pause_already_so),
    ("c5_poll_resume_success", resume_success),
    (
        "c5_poll_resume_not_paused_with_a_future_deadline",
        resume_not_paused_with_a_future_deadline,
    ),
    ("c5_poll_configure_success", configure_success),
    ("c5_poll_configure_same_value", configure_same_value),
    ("c5_poll_configure_bounds", configure_bounds),
    ("c5_poll_configure_out_of_range", configure_out_of_range),
    ("c5_poll_unregistered", unregistered),
    (
        "c5_poll_unregistered_after_the_check",
        unregistered_after_the_check,
    ),
    ("c5_poll_unenabled", unenabled),
    ("c5_poll_unavailable_index", unavailable_index),
    (
        "c5_poll_pending_local_sync_does_not_refuse",
        pending_local_sync_does_not_refuse,
    ),
    (
        "c5_poll_pause_survives_the_remote_s_removal",
        pause_survives_the_remote_s_removal,
    ),
    (
        "c5_poll_stale_whole_tuple_control",
        stale_whole_tuple_control,
    ),
    (
        "c5_poll_stale_pause_keeps_the_newer_interval",
        stale_pause_keeps_the_newer_interval,
    ),
    (
        "c5_poll_stale_resume_keeps_the_newer_interval",
        stale_resume_keeps_the_newer_interval,
    ),
    (
        "c5_poll_crash_before_the_transaction",
        crash_before_the_transaction,
    ),
    (
        "c5_poll_stop_inside_the_transaction",
        stop_inside_the_transaction,
    ),
    (
        "c5_poll_committed_reply_lost_replays_and_the_pause_stands",
        committed_reply_lost_replays_and_the_pause_stands,
    ),
    (
        "c5_poll_never_committed_resend_is_a_new_write",
        never_committed_resend_is_a_new_write,
    ),
    ("c5_poll_the_same_request_races", the_same_request_races),
    (
        "c5_poll_the_race_is_lost_to_another_verb",
        the_race_is_lost_to_another_verb,
    ),
    ("c5_poll_replay_of_each_verb", replay_of_each_verb),
    (
        "c5_poll_request_show_and_cancel_of_a_policy_record",
        request_show_and_cancel_of_a_policy_record,
    ),
    ("c5_poll_pause_leaves_the_rest", pause_leaves_the_rest),
    ("c5_poll_resume_leaves_the_rest", resume_leaves_the_rest),
    (
        "c5_poll_configure_leaves_the_rest",
        configure_leaves_the_rest,
    ),
    ("c5_poll_resume_makes_a_poll_due", resume_makes_a_poll_due),
    (
        "c5_poll_resume_does_not_lift_the_suspension",
        resume_does_not_lift_the_suspension,
    ),
    (
        "c5_poll_policy_row_gone_inside_the_path",
        policy_row_gone_inside_the_path,
    ),
    (
        "c5_poll_policy_row_unreadable_inside_the_path",
        policy_row_unreadable_inside_the_path,
    ),
    (
        "c5_poll_an_already_applied_replay_stays_so",
        an_already_applied_replay_stays_so,
    ),
    ("c5_poll_request_id_reused", request_id_reused),
    (
        "c5_poll_cancelled_before_it_writes",
        cancelled_before_it_writes,
    ),
    ("c5_poll_no_verb_sets_enabled", no_verb_sets_enabled),
    ("c5_poll_once_ok", once_ok),
    ("c5_poll_once_ok_replays", once_ok_replays),
    ("c5_poll_once_dirty_is_data", once_dirty_is_data),
    ("c5_poll_once_malformed_is_data", once_malformed_is_data),
    ("c5_poll_once_discovery_pending", once_discovery_pending),
    ("c5_poll_once_no_remote", once_no_remote),
    (
        "c5_poll_once_a_pause_does_not_refuse",
        once_a_pause_does_not_refuse,
    ),
    (
        "c5_poll_once_cancelled_by_the_token",
        once_cancelled_by_the_token,
    ),
    (
        "c5_poll_once_cancelled_by_the_flag",
        once_cancelled_by_the_flag,
    ),
    (
        "c5_poll_once_cancelled_by_request_cancel_while_running",
        once_cancelled_by_request_cancel_while_running,
    ),
    (
        "c5_poll_once_cancelled_at_the_last_safe_point",
        once_cancelled_at_the_last_safe_point,
    ),
    ("c5_poll_once_flag_unreadable", once_flag_unreadable),
    ("c5_poll_once_interrupted", once_interrupted),
    (
        "c5_poll_once_interrupted_after_a_target_moved",
        once_interrupted_after_a_target_moved,
    ),
    ("c5_poll_once_err_interrupted", once_err_interrupted),
    ("c5_poll_once_held_then_cancelled", once_held_then_cancelled),
    ("c5_poll_once_fenced_is_final", once_fenced_is_final),
    (
        "c5_poll_once_stored_configuration_required",
        once_stored_configuration_required,
    ),
    (
        "c5_poll_once_stored_selected_key_unavailable",
        once_stored_selected_key_unavailable,
    ),
    (
        "c5_poll_once_stored_unlock_required",
        once_stored_unlock_required,
    ),
    (
        "c5_poll_once_stored_host_approval_required",
        once_stored_host_approval_required,
    ),
    (
        "c5_poll_once_stored_transport_unavailable",
        once_stored_transport_unavailable,
    ),
    (
        "c5_poll_once_stored_protocol_rejected",
        once_stored_protocol_rejected,
    ),
    (
        "c5_poll_once_stored_repository_unavailable",
        once_stored_repository_unavailable,
    ),
    (
        "c5_poll_once_transport_stop_is_final",
        once_transport_stop_is_final,
    ),
    ("c5_poll_once_key_rejected", once_key_rejected),
    (
        "c5_poll_once_key_rejected_reentered",
        once_key_rejected_reentered,
    ),
    ("c5_poll_once_no_selected_key", once_no_selected_key),
    ("c5_poll_once_unlock_required", once_unlock_required),
    (
        "c5_poll_once_host_approval_required",
        once_host_approval_required,
    ),
    (
        "c5_poll_once_transport_with_no_row",
        once_transport_with_no_row,
    ),
    ("c5_poll_once_invalid_remote", once_invalid_remote),
    (
        "c5_poll_once_repository_error_held",
        once_repository_error_held,
    ),
    (
        "c5_poll_once_repository_error_after_completion",
        once_repository_error_after_completion,
    ),
    (
        "c5_poll_once_killed_resent_restarts",
        once_killed_resent_restarts,
    ),
    ("c5_poll_once_busy", once_busy),
    (
        "c5_poll_once_busy_on_a_held_lease",
        once_busy_on_a_held_lease,
    ),
    ("c5_poll_once_poll_yielding", once_poll_yielding),
    ("c5_poll_once_conflict_pending", once_conflict_pending),
    ("c5_poll_once_pending_local_sync", once_pending_local_sync),
    (
        "c5_poll_once_resume_names_a_held_poll",
        once_resume_names_a_held_poll,
    ),
    (
        "c5_poll_once_resume_of_a_failed_poll",
        once_resume_of_a_failed_poll,
    ),
    (
        "c5_poll_once_pause_during_a_running_poll",
        once_pause_during_a_running_poll,
    ),
    ("c5_poll_once_conformance", once_conformance),
    (
        "c5_poll_once_token_stops_a_transfer",
        once_token_stops_a_transfer,
    ),
    (
        "c5_poll_once_repository_error_over_a_cancelled_row",
        once_repository_error_over_a_cancelled_row,
    ),
    (
        "c5_poll_once_resend_during_a_materialization",
        once_resend_during_a_materialization,
    ),
    (
        "c5_poll_once_interrupted_then_one_worktree",
        once_interrupted_then_one_worktree,
    ),
    (
        "c5_poll_once_request_cancel_of_a_failed_fenced_row",
        once_request_cancel_of_a_failed_fenced_row,
    ),
    (
        "c5_poll_once_killed_then_the_remote_removed",
        once_killed_then_the_remote_removed,
    ),
    (
        "c5_poll_once_busy_beside_a_lifecycle_child",
        once_busy_beside_a_lifecycle_child,
    ),
    (
        "c5_poll_once_fences_a_lifecycle_child",
        once_fences_a_lifecycle_child,
    ),
    (
        "c5_poll_once_a_closed_ticket_s_older_branch_is_current",
        once_a_closed_ticket_s_older_branch_is_current,
    ),
    (
        "c5_poll_once_a_branch_primary_lacks_still_materializes",
        once_a_branch_primary_lacks_still_materializes,
    ),
];

const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP1";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP2";
const THIRD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP3";
const POLICY: &str = "poll_policy_mutation.schema.json";
/// 2030-01-01T00:00:00Z, later than any clock these rows run under.
const FUTURE: i64 = 1_893_456_000;
const FUTURE_TEXT: &str = "2030-01-01T00:00:00Z";

fn request(id: &str) -> RequestId {
    RequestId::parse(id).expect("a request ID")
}

fn called(service: &RepositoryService, id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    envelope
}

fn pause(root: &Path) -> Mutation {
    Mutation::PollPause(PollPauseInput {
        root: root.to_owned(),
    })
}

fn resume(root: &Path) -> Mutation {
    Mutation::PollResume(PollResumeInput {
        root: root.to_owned(),
    })
}

fn configure(root: &Path, interval_seconds: u64) -> Mutation {
    Mutation::PollConfigure(PollConfigureInput {
        root: root.to_owned(),
        interval_seconds,
    })
}

fn status(local: &Local) -> Result<PollingStatusDto, FixtureError> {
    let repository = fixed(local.service.resolve_repository(&local.root))?;
    fixed(local.service.polling_status(&repository))
}

fn policy(envelope: &Envelope<MutationDataDto>) -> &PollPolicyMutationDto {
    match envelope.data.as_ref() {
        Some(MutationDataDto::PollPolicy(data)) => data,
        other => panic!("{:?}: {other:?}", envelope.code),
    }
}

/// Stages the columns no verb sets, as earlier polls would have left them.
fn stage(local: &Local, assignments: &str) -> Result<(), FixtureError> {
    fixed(
        local
            .db()?
            .execute_batch(&format!("UPDATE remote_polling_state SET {assignments}")),
    )
}

fn request_rows(local: &Local) -> Result<i64, FixtureError> {
    fixed(
        local
            .db()?
            .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0)),
    )
}

fn finished_record(local: &Local, id: &str) -> Result<RequestRecord, FixtureError> {
    let record = fixed(local.service.request_record(request(id)))?.ok_or(FixtureError)?;
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.attempt, 1, "a policy request is entered once");
    assert!(record.operations.is_empty(), "no operation is recorded");
    Ok(record)
}

/// A golden of the answer, with its data checked against `POLICY`.
fn golden(root: &Path, name: &str, envelope: &Envelope<MutationDataDto>) {
    let root = root.canonicalize().expect("the root");
    mutation_golden::assert_contract_at(
        name,
        root.to_str().expect("a UTF-8 root"),
        envelope.data.as_ref().map(|_| POLICY),
        &[],
        envelope,
    );
}

/// Nothing is requested of Git, the index or a remote.
fn no_effects(envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.operation_id, None);
    assert!(envelope.recovery.is_empty(), "{:?}", envelope.recovery);
}

fn pause_success() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let done = called(&local.service, FIRST, pause(&local.root));
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.outcome, Outcome::Success);
    no_effects(&done);
    assert!(policy(&done).paused);
    assert!(status(&local)?.paused);
    finished_record(&local, FIRST)?;
    golden(&local.root, "poll_pause", &done);
    Ok(())
}

fn pause_already_so() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(&local, "paused=1,next_eligible_at=1893456000")?;
    let before = status(&local)?;
    let done = called(&local.service, FIRST, pause(&local.root));
    assert_eq!(done.code, ResultCode::AlreadyApplied, "{:?}", done.data);
    assert_eq!(done.outcome, Outcome::Noop);
    no_effects(&done);
    assert_eq!(status(&local)?, before);
    // Nothing is written to the policy; the finished record is stored.
    finished_record(&local, FIRST)?;
    golden(&local.root, "poll_pause_noop", &done);
    Ok(())
}

/// Clears the pause and the deadline, and leaves the suspension, `enabled` and
/// the backoff value alone, read through `polling_status`.
fn resume_success() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(
        &local,
        "paused=1,enabled=0,recovery_suspended=1,automatic_backoff_seconds=240,\
         next_eligible_at=1893456000,interval_seconds=900",
    )?;
    let done = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    no_effects(&done);
    let after = status(&local)?;
    assert!(!after.paused);
    assert_eq!(after.next_eligible_at, None);
    assert!(after.recovery_suspended, "the suspension stays");
    assert!(!after.enabled, "enabled stays");
    assert_eq!(after.backoff_seconds, Some(240), "the backoff value stays");
    assert_eq!(after.interval_seconds, 900);
    let data = policy(&done);
    assert!(!data.paused);
    assert!(data.recovery_suspended, "the answer names the suspension");
    assert_eq!(data.next_eligible_at, None);
    golden(&local.root, "poll_resume", &done);
    Ok(())
}

/// Not paused: nothing is written, so the deadline stands.
fn resume_not_paused_with_a_future_deadline() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(
        &local,
        "automatic_backoff_seconds=120,next_eligible_at=1893456000",
    )?;
    let before = status(&local)?;
    let done = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(done.code, ResultCode::AlreadyApplied, "{:?}", done.data);
    assert_eq!(done.outcome, Outcome::Noop);
    no_effects(&done);
    assert_eq!(status(&local)?, before);
    assert_eq!(
        status(&local)?.next_eligible_at.as_deref(),
        Some(FUTURE_TEXT)
    );
    assert_eq!(policy(&done).next_eligible_at.as_deref(), Some(FUTURE_TEXT));
    finished_record(&local, FIRST)?;
    golden(&local.root, "poll_resume_noop", &done);
    Ok(())
}

/// Leaves the stored due time. `configure_leaves_the_rest` pins every other field.
fn configure_success() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(&local, &format!("paused=1,next_eligible_at={FUTURE}"))?;
    let done = called(&local.service, FIRST, configure(&local.root, 120));
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    no_effects(&done);
    let after = status(&local)?;
    assert_eq!(after.interval_seconds, 120);
    assert!(after.paused);
    assert_eq!(after.next_eligible_at.as_deref(), Some(FUTURE_TEXT));
    assert_eq!(policy(&done).interval_seconds, 120);
    golden(&local.root, "poll_configure", &done);
    Ok(())
}

fn configure_same_value() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let before = status(&local)?;
    let done = called(
        &local.service,
        FIRST,
        configure(&local.root, before.interval_seconds),
    );
    assert_eq!(done.code, ResultCode::AlreadyApplied, "{:?}", done.data);
    assert_eq!(status(&local)?, before);
    finished_record(&local, FIRST)?;
    golden(&local.root, "poll_configure_noop", &done);
    Ok(())
}

fn configure_bounds() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    for (id, seconds) in [(FIRST, 60), (SECOND, 3600)] {
        let done = called(&local.service, id, configure(&local.root, seconds));
        assert_eq!(done.code, ResultCode::Ok, "{seconds}: {:?}", done.data);
        assert_eq!(status(&local)?.interval_seconds, seconds);
    }
    Ok(())
}

fn configure_out_of_range() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let before = status(&local)?;
    for seconds in [0, 59, 3601, u64::MAX] {
        let refused = called(&local.service, FIRST, configure(&local.root, seconds));
        assert_eq!(refused.code, ResultCode::InvalidInput, "{seconds}");
        assert_eq!(refused.data, None);
        assert_eq!(status(&local)?, before);
        assert_eq!(request_rows(&local)?, 0, "{seconds}: no record");
        golden(&local.root, "poll_configure_invalid", &refused);
    }
    Ok(())
}

/// A repository another index holds: enabled, and not registered here.
fn unregistered() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let elsewhere = fixed(tempfile::tempdir())?;
    let service = fixed(RepositoryService::open_at(elsewhere.path()))?;
    for (id, mutation) in [
        (FIRST, pause(&local.root)),
        (SECOND, resume(&local.root)),
        (THIRD, configure(&local.root, 120)),
    ] {
        let refused = called(&service, id, mutation);
        assert_eq!(refused.code, ResultCode::RepositoryNotRegistered);
        assert_eq!(refused.data, None);
        assert_eq!(fixed(service.request_record(request(id)))?, None);
    }
    let refused = called(&service, FIRST, pause(&local.root));
    golden(&local.root, "poll_pause_not_registered", &refused);
    Ok(())
}

/// The registration goes after `prepare` read it. The transaction reads it
/// again, so the answer is what is true at the commit, and no record is left.
fn unregistered_after_the_check() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let registry = local.directory.path().join("data").join(REGISTRY_FILE);
    local.service.set_acceptance_hook_for_testing(move || {
        rusqlite::Connection::open(registry)
            .expect("the registry")
            .execute("DELETE FROM repositories", [])
            .expect("the registration is removed");
    });
    let refused = called(&local.service, FIRST, pause(&local.root));
    assert_eq!(refused.code, ResultCode::RepositoryNotRegistered);
    assert_eq!(refused.data, None);
    assert_eq!(request_rows(&local)?, 0);
    Ok(())
}

fn unenabled() -> Result<(), FixtureError> {
    let directory = fixed(tempfile::tempdir())?;
    let root = directory.path().join("plain");
    initialized(&root)?;
    let service = fixed(RepositoryService::open_at(&directory.path().join("data")))?;
    for (id, mutation) in [
        (FIRST, pause(&root)),
        (SECOND, resume(&root)),
        (THIRD, configure(&root, 120)),
    ] {
        let refused = called(&service, id, mutation);
        assert_eq!(refused.code, ResultCode::RepositoryNotEnabled);
        assert_eq!(fixed(service.request_record(request(id)))?, None);
    }
    let refused = called(&service, FIRST, pause(&root));
    golden(&root, "poll_pause_not_enabled", &refused);
    Ok(())
}

fn unavailable_index() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let data = local.directory.path().join("data");
    for entry in fixed(std::fs::read_dir(&data))? {
        let path = fixed(entry)?.path();
        let name = path.file_name().ok_or(FixtureError)?.to_string_lossy();
        if name.starts_with(REGISTRY_FILE) && !name.ends_with(".lock") {
            fixed(std::fs::remove_file(&path))?;
        }
    }
    fixed(std::fs::write(data.join(REGISTRY_FILE), b"not sqlite"))?;
    let service = fixed(RepositoryService::open_at(&data))?;
    for (id, mutation) in [
        (FIRST, pause(&local.root)),
        (SECOND, resume(&local.root)),
        (THIRD, configure(&local.root, 120)),
    ] {
        let refused = called(&service, id, mutation);
        assert_eq!(refused.code, ResultCode::IndexUnavailable);
        assert_eq!(refused.data, None);
        assert!(
            refused
                .recovery
                .iter()
                .any(|action| action.action.as_str() == "index.rebuild"),
            "{:?}",
            refused.recovery
        );
        assert_eq!(
            fixed(std::fs::read(data.join(REGISTRY_FILE)))?,
            b"not sqlite",
            "a policy verb doesn't replace the index"
        );
        if id == FIRST {
            // The recovery tables' row for the three, which share a binding.
            golden(&local.root, "poll_pause_index_unavailable", &refused);
        }
    }
    Ok(())
}

/// No Git work happens, so a pending local synchronization refuses nothing.
fn pending_local_sync_does_not_refuse() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\n\
         title: Primary\ntype: task\nstatus: open\n---\nBody of Primary.\n"
    );
    commit_in(
        &local.root,
        MAIN,
        &format!(".manyhands/tickets/{ITEM}/ticket.md"),
        source.as_bytes(),
    )?;
    let id: canonical::ItemId = fixed(ITEM.parse())?;
    let observed = fixed(local.service.observe_local_context_sync(
        &fixed(local.service.resolve_repository(&local.root))?,
        AuthoringKind::Ticket,
        &id,
    ))?;
    let stopped = called(
        &local.service,
        FIRST,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: local.root.clone(),
            kind: AuthoringKind::Ticket,
            id,
            observation: observed.observation,
        }),
    );
    assert_eq!(stopped.code, ResultCode::RecoveryRequired, "the sync pends");
    let paused = called(&local.service, SECOND, pause(&local.root));
    assert_eq!(paused.code, ResultCode::Ok);
    let resumed = called(&local.service, THIRD, resume(&local.root));
    assert_eq!(resumed.code, ResultCode::Ok);
    Ok(())
}

fn confirmed(
    service: &RepositoryService,
    id: &str,
    mutation: impl Fn() -> Mutation,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    let envelope = service.prepare(&mutation());
    let preview = envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?;
    let confirmation = fixed(ConfirmationId::parse(&preview.confirmation_id))?;
    Ok(service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: Some(confirmation),
            mutation: mutation(),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    ))
}

/// A pause outlives the remote it was polling, and re-adding the remote does
/// not silently resume it (runtime RFC).
fn pause_survives_the_remote_s_removal() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let add = || {
        Mutation::RemoteAdd(RemoteAddInput {
            root: local.root.clone(),
            name: "origin".to_owned(),
            url: "git@example.invalid:team/project.git".to_owned(),
        })
    };
    let select = |name: Option<&str>| {
        Mutation::RemoteSelect(RemoteSelectInput {
            root: local.root.clone(),
            name: name.map(str::to_owned),
        })
    };
    let ok = |envelope: Envelope<MutationDataDto>| {
        assert_eq!(envelope.outcome, Outcome::Success, "{:?}", envelope.code);
    };
    ok(called(&local.service, "01ARZ3NDEKTSV4RRFFQ69G5FQ1", add()));
    ok(called(
        &local.service,
        "01ARZ3NDEKTSV4RRFFQ69G5FQ2",
        select(Some("origin")),
    ));
    ok(called(&local.service, FIRST, pause(&local.root)));
    ok(called(
        &local.service,
        "01ARZ3NDEKTSV4RRFFQ69G5FQ3",
        select(None),
    ));
    ok(confirmed(
        &local.service,
        "01ARZ3NDEKTSV4RRFFQ69G5FQ4",
        || {
            Mutation::RemoteRemove(RemoteRemoveInput {
                root: local.root.clone(),
                name: "origin".to_owned(),
            })
        },
    )?);
    assert!(status(&local)?.paused, "after the removal");
    ok(called(&local.service, "01ARZ3NDEKTSV4RRFFQ69G5FQ5", add()));
    ok(called(
        &local.service,
        "01ARZ3NDEKTSV4RRFFQ69G5FQ6",
        select(Some("origin")),
    ));
    assert!(status(&local)?.paused, "after re-adding it");
    Ok(())
}

/// Review Focus 6, the control. A caller that read the policy, then writes the
/// whole tuple after someone else's `poll configure`, loses the newer interval.
/// This is what the next two rows are against: were this row to stop failing
/// the interval, they would prove nothing of `update_remote_polling`.
fn stale_whole_tuple_control() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let stale = status(&local)?;
    assert_eq!(stale.interval_seconds, 300);
    let newer = called(&local.service, FIRST, configure(&local.root, 120));
    assert_eq!(newer.code, ResultCode::Ok);
    fixed(local.service.set_remote_polling(
        &local.root,
        stale.enabled,
        true,
        fixed(PollingInterval::from_seconds(stale.interval_seconds))?,
    ))?;
    let after = status(&local)?;
    assert!(after.paused);
    assert_eq!(after.interval_seconds, 300, "the newer interval is lost");
    Ok(())
}

/// Review Focus 6, the same interleaving through the verb.
fn stale_pause_keeps_the_newer_interval() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let stale = status(&local)?;
    assert_eq!(stale.interval_seconds, 300);
    let newer = called(&local.service, FIRST, configure(&local.root, 120));
    assert_eq!(newer.code, ResultCode::Ok);
    let paused = called(&local.service, SECOND, pause(&local.root));
    assert_eq!(paused.code, ResultCode::Ok);
    let after = status(&local)?;
    assert!(after.paused);
    assert_eq!(after.interval_seconds, 120, "the newer interval stands");
    assert_eq!(policy(&paused).interval_seconds, 120);
    Ok(())
}

fn stale_resume_keeps_the_newer_interval() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(&local, "paused=1")?;
    let newer = called(&local.service, FIRST, configure(&local.root, 120));
    assert_eq!(newer.code, ResultCode::Ok);
    let resumed = called(&local.service, SECOND, resume(&local.root));
    assert_eq!(resumed.code, ResultCode::Ok);
    assert_eq!(status(&local)?.interval_seconds, 120);
    Ok(())
}

fn at_failure_point(local: &Local, point: FailurePoint) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &local.directory.path().join("data"),
        point,
    ))
}

/// A process that dies before the transaction leaves no record and no write.
fn crash_before_the_transaction() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let before = status(&local)?;
    let dying = at_failure_point(&local, FailurePoint::BeforeRequestDomainCall)?;
    let died = called(&dying, FIRST, pause(&local.root));
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(status(&local)?, before);
    assert_eq!(request_rows(&local)?, 0);
    Ok(())
}

/// The field is written and the transaction does not commit: neither stands.
/// The two are one write, so there is no accepted record to find.
fn stop_inside_the_transaction() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    for (mutation, name, assignment) in [
        (pause(&local.root), "pause", "paused=0"),
        (
            resume(&local.root),
            "resume",
            "paused=1,next_eligible_at=1893456000",
        ),
        (configure(&local.root, 120), "configure", "paused=0"),
    ] {
        stage(&local, assignment)?;
        let before = status(&local)?;
        let dying = at_failure_point(&local, FailurePoint::BeforeRequestSettlement)?;
        let died = called(&dying, FIRST, mutation);
        assert_eq!(died.code, ResultCode::InternalError, "{name}");
        assert_eq!(status(&local)?, before, "{name}: the field is rolled back");
        assert_eq!(request_rows(&local)?, 0, "{name}: so is the record");
    }
    Ok(())
}

/// R1 committed and its reply was lost. R2 pauses. R1 resent replays, and
/// changes nothing: the pause stands.
fn committed_reply_lost_replays_and_the_pause_stands() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(&local, "paused=1")?;
    let lost = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(lost.code, ResultCode::Ok);
    assert!(!status(&local)?.paused);
    let paused = called(&local.service, SECOND, pause(&local.root));
    assert_eq!(paused.code, ResultCode::Ok);
    let resent = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(whole(&resent), whole(&lost), "the stored result, again");
    assert!(status(&local)?.paused, "the later pause stands");
    Ok(())
}

/// R1 stopped before it committed, so it can't be told from one never sent.
/// R2 pauses. R1 resent is a new write and clears the pause: last commit wins,
/// and its answer says what it did (owner question 6).
fn never_committed_resend_is_a_new_write() -> Result<(), FixtureError> {
    for point in [
        FailurePoint::BeforeRequestDomainCall,
        FailurePoint::BeforeRequestSettlement,
    ] {
        let local = Local::empty()?;
        stage(&local, "paused=1")?;
        let dying = at_failure_point(&local, point)?;
        let died = called(&dying, FIRST, resume(&local.root));
        assert_eq!(died.code, ResultCode::InternalError, "{point:?}");
        assert!(status(&local)?.paused, "{point:?}: R1 wrote nothing");
        let paused = called(&local.service, SECOND, pause(&local.root));
        assert_eq!(
            paused.code,
            ResultCode::AlreadyApplied,
            "{point:?}: R2 finds it paused"
        );
        let resent = called(&local.service, FIRST, resume(&local.root));
        assert_eq!(resent.code, ResultCode::Ok, "{point:?}: a new write");
        assert!(!policy(&resent).paused, "{point:?}: the answer says so");
        assert!(!status(&local)?.paused, "{point:?}: the pause is cleared");
        finished_record(&local, FIRST)?;
    }
    Ok(())
}

/// Another call of the request commits between this one's look and its
/// transaction. This call writes nothing and answers from that record.
fn the_same_request_races() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let data = local.directory.path().join("data");
    let root = local.root.clone();
    local.service.set_acceptance_hook_for_testing(move || {
        let other = RepositoryService::open_at(&data).expect("another call's service");
        let won = called(&other, FIRST, resume(&root));
        assert_eq!(won.code, ResultCode::AlreadyApplied);
        rusqlite::Connection::open(data.join(REGISTRY_FILE))
            .expect("the registry")
            .execute("UPDATE remote_polling_state SET paused=1", [])
            .expect("the policy moves on");
    });
    let lost = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(lost.code, ResultCode::AlreadyApplied, "the stored answer");
    assert!(!policy(&lost).paused);
    assert!(status(&local)?.paused, "this call wrote nothing");
    assert_eq!(request_rows(&local)?, 1);
    finished_record(&local, FIRST)?;
    Ok(())
}

/// As above, with the winner of another verb under the same ID: the loser is
/// another request, refused, and leaves the winner's record and write.
fn the_race_is_lost_to_another_verb() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let data = local.directory.path().join("data");
    let root = local.root.clone();
    local.service.set_acceptance_hook_for_testing(move || {
        let other = RepositoryService::open_at(&data).expect("another call's service");
        let won = called(&other, FIRST, pause(&root));
        assert_eq!(won.code, ResultCode::Ok);
    });
    let lost = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(lost.code, ResultCode::RequestMismatch);
    assert!(status(&local)?.paused, "the winner's write stands");
    let record = finished_record(&local, FIRST)?;
    assert_eq!(record.command, "poll pause");
    assert_eq!(request_rows(&local)?, 1);
    Ok(())
}

/// Every column of the policy row, by name, as text.
fn policy_columns(local: &Local) -> Result<Vec<(String, String)>, FixtureError> {
    let db = local.db()?;
    let mut statement = fixed(db.prepare("SELECT * FROM remote_polling_state"))?;
    let names: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    fixed(statement.query_row([], |row| {
        names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let value: rusqlite::types::Value = row.get(index)?;
                Ok((name.clone(), format!("{value:?}")))
            })
            .collect()
    }))
}

/// Every column a verb must leave, set to a value other than its default, and
/// a current observation batch. Only `active_operation_id` stays null: it reads
/// the remote journal, not this row.
fn stage_every_column(local: &Local, paused: i64) -> Result<(), FixtureError> {
    stage(
        local,
        &format!(
            "enabled=0,paused={paused},interval_seconds=900,automatic_backoff_seconds=240,\
             recovery_suspended=1,history_unknown=1,configuration_generation=3,\
             endpoint_digest=zeroblob(32),remote_name='origin',primary_branch='main',\
             latest_outcome='transport_unavailable',latest_attempt_at=1000,\
             next_eligible_at={FUTURE}"
        ),
    )?;
    fixed(local.db()?.execute_batch(
        "INSERT INTO remote_observation_batches
            (repository_id,remote_name,primary_branch,configuration_generation,observed_at,is_current)
         SELECT id,'origin','main',3,1000,1 FROM repositories",
    ))
}

/// The verb changes exactly `named`: the whole status equals `expected`, and
/// every other column of the row is as it was.
fn leaves_the_rest(
    local: &Local,
    mutation: Mutation,
    named: &[&str],
    change: impl FnOnce(&mut PollingStatusDto),
) -> Result<(), FixtureError> {
    let mut expected = status(local)?;
    let columns = policy_columns(local)?;
    change(&mut expected);
    let done = called(&local.service, FIRST, mutation);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(status(local)?, expected);
    let unnamed = |columns: Vec<(String, String)>| -> Vec<(String, String)> {
        columns
            .into_iter()
            .filter(|(name, _)| !named.contains(&name.as_str()))
            .collect()
    };
    assert_eq!(unnamed(policy_columns(local)?), unnamed(columns));
    Ok(())
}

fn pause_leaves_the_rest() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage_every_column(&local, 0)?;
    leaves_the_rest(&local, pause(&local.root), &["paused"], |expected| {
        expected.paused = true;
    })
}

fn resume_leaves_the_rest() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage_every_column(&local, 1)?;
    leaves_the_rest(
        &local,
        resume(&local.root),
        &["paused", "next_eligible_at"],
        |expected| {
            expected.paused = false;
            expected.next_eligible_at = None;
        },
    )
}

fn configure_leaves_the_rest() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage_every_column(&local, 1)?;
    leaves_the_rest(
        &local,
        configure(&local.root, 120),
        &["interval_seconds"],
        |expected| expected.interval_seconds = 120,
    )
}

/// Decision 11: a backoff deadline is cleared, so an eligible poll is due, with
/// the backoff value left for the next failure.
fn resume_makes_a_poll_due() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(
        &local,
        &format!(
            "enabled=1,recovery_suspended=0,paused=1,automatic_backoff_seconds=240,\
             next_eligible_at={FUTURE}"
        ),
    )?;
    let done = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(done.code, ResultCode::Ok);
    assert_eq!(
        fixed(local.service.automatic_poll_eligibility(&local.root))?,
        AutomaticPollEligibility::Eligible
    );
    assert_eq!(status(&local)?.backoff_seconds, Some(240));
    Ok(())
}

/// Decision 5: the suspension still blocks automatic polls after a resume.
fn resume_does_not_lift_the_suspension() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(
        &local,
        &format!(
            "enabled=1,recovery_suspended=1,paused=1,automatic_backoff_seconds=240,\
             next_eligible_at={FUTURE}"
        ),
    )?;
    let done = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(done.code, ResultCode::Ok);
    assert!(matches!(
        fixed(local.service.automatic_poll_eligibility(&local.root))?,
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::RecoverySuspended,
            ..
        }
    ));
    Ok(())
}

/// The policy row goes after `prepare`, and neither a record nor a write stands.
/// `read_policy` isn't reached: opening the index refuses a registration with no
/// policy row (`state::migrate`), and `write_records` converts that as a read.
///
/// It answers `internal_error`, by the controller's ruling: a registration with
/// no policy row is refused at open, and `write_records` keeps its conversion.
/// `policy_row_unreadable_inside_the_path` pins `recovery_required` where `write`
/// is reached.
fn policy_row_gone_inside_the_path() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let registry = local.directory.path().join("data").join(REGISTRY_FILE);
    local.service.set_acceptance_hook_for_testing(move || {
        rusqlite::Connection::open(registry)
            .expect("the registry")
            .execute("DELETE FROM remote_polling_state", [])
            .expect("the policy row is removed");
    });
    let refused = called(&local.service, FIRST, pause(&local.root));
    assert_eq!(refused.code, ResultCode::InternalError);
    assert_eq!(refused.data, None);
    assert_eq!(request_rows(&local)?, 0);
    Ok(())
}

/// A row `read_policy` itself refuses, and opening the index doesn't: a remote
/// name no ref plan can be made from. The repository error keeps its code.
fn policy_row_unreadable_inside_the_path() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    stage(&local, "remote_name='bad name',primary_branch='main'")?;
    let before = policy_columns(&local)?;
    let refused = called(&local.service, FIRST, pause(&local.root));
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(refused.data, None);
    assert_eq!(request_rows(&local)?, 0);
    assert_eq!(policy_columns(&local)?, before);
    Ok(())
}

/// `request show` reads a policy record as finished, with its result. `request
/// cancel` finds nothing to cancel and sets no flag: nothing is left accepted.
fn request_show_and_cancel_of_a_policy_record() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let done = called(&local.service, FIRST, pause(&local.root));
    assert_eq!(done.code, ResultCode::Ok);
    let record = finished_record(&local, FIRST)?;
    let shown = local.service.show_request(request(FIRST));
    assert_eq!(shown.code, ResultCode::Ok);
    let shown = whole_of(&shown);
    assert_eq!(shown["data"]["state"], "finished", "{shown}");
    assert_eq!(
        shown["data"]["operations"],
        serde_json::json!([]),
        "{shown}"
    );
    assert_eq!(shown["data"]["result"]["code"], "ok", "{shown}");
    assert_eq!(shown["data"]["result"]["data"]["paused"], true, "{shown}");

    let cancelled = whole_of(&local.service.cancel_request(request(FIRST)));
    assert_eq!(cancelled["data"]["state"], "finished", "{cancelled}");
    assert_eq!(cancelled["data"]["requested"], false, "{cancelled}");
    let after = finished_record(&local, FIRST)?;
    assert_eq!(after, record, "cancel changed the record");
    assert!(!after.cancel_requested);
    Ok(())
}

fn whole_of(envelope: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(envelope).expect("an envelope serializes")
}

fn whole(envelope: &Envelope<MutationDataDto>) -> serde_json::Value {
    serde_json::to_value(envelope).expect("an envelope serializes")
}

/// Byte for byte, with the policy having moved since and the record unchanged.
fn replay_of_each_verb() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let pauses = || pause(&local.root);
    let resumes = || resume(&local.root);
    let configures = || configure(&local.root, 120);
    let verbs: [(&str, &dyn Fn() -> Mutation); 3] =
        [(FIRST, &pauses), (SECOND, &resumes), (THIRD, &configures)];
    for (id, mutation) in verbs {
        let first = called(&local.service, id, mutation());
        assert_eq!(first.code, ResultCode::Ok, "{id}");
        let record = finished_record(&local, id)?;
        // The policy moves on.
        stage(&local, "interval_seconds=interval_seconds+1")?;
        let moved = status(&local)?;
        let again = called(&local.service, id, mutation());
        assert_eq!(
            fixed(serde_json::to_vec(&again))?,
            fixed(serde_json::to_vec(&first))?,
            "{id}"
        );
        assert_eq!(status(&local)?, moved, "{id}: a replay writes nothing");
        assert_eq!(finished_record(&local, id)?, record, "{id}");
    }
    Ok(())
}

fn an_already_applied_replay_stays_so() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let first = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(first.code, ResultCode::AlreadyApplied);
    stage(&local, "paused=1")?;
    let again = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(whole(&again), whole(&first));
    assert!(status(&local)?.paused, "a replay writes nothing");
    Ok(())
}

/// The same ID with another verb or value is refused, and changes nothing.
fn request_id_reused() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let first = called(&local.service, FIRST, configure(&local.root, 120));
    assert_eq!(first.code, ResultCode::Ok);
    let record = finished_record(&local, FIRST)?;
    for mutation in [
        configure(&local.root, 240),
        pause(&local.root),
        resume(&local.root),
    ] {
        let refused = called(&local.service, FIRST, mutation);
        assert_eq!(refused.code, ResultCode::RequestMismatch);
    }
    assert_eq!(finished_record(&local, FIRST)?, record);
    assert_eq!(status(&local)?.interval_seconds, 120);
    assert!(!status(&local)?.paused);
    Ok(())
}

fn cancelled_before_it_writes() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let before = status(&local)?;
    let token = CancellationToken::new();
    token.cancel();
    let refused = local.service.execute(
        MutationCall {
            request_id: request(FIRST),
            confirmation: None,
            mutation: pause(&local.root),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &token,
    );
    assert_eq!(refused.code, ResultCode::Cancelled);
    assert_eq!(status(&local)?, before);
    assert_eq!(request_rows(&local)?, 0);
    Ok(())
}

fn no_verb_sets_enabled() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    for (enabled, flag) in [(1, true), (0, false)] {
        stage(&local, &format!("enabled={enabled}"))?;
        for mutation in [
            pause(&local.root),
            resume(&local.root),
            configure(&local.root, 120 + u64::from(flag)),
        ] {
            let id = RequestId::new().to_string();
            let done = called(&local.service, &id, mutation);
            assert_eq!(done.outcome, Outcome::Success, "{:?}", done.code);
            assert_eq!(status(&local)?.enabled, flag);
        }
    }
    Ok(())
}

// `poll once`: the design's frozen table, each row on its own world.

const REPORT: &str = "poll_mutation.schema.json";
const PEER_FILE: &str = "peer-advance";

fn once(root: &Path) -> Mutation {
    Mutation::PollOnce(PollOnceInput {
        root: root.to_owned(),
    })
}

fn execute_once(w: &World, id: &str) -> Envelope<MutationDataDto> {
    execute(w, id, once(&w.root))
}

/// A world whose host is trusted, with primary observed as the remote holds it.
fn observed() -> Result<World, FixtureError> {
    observed_in(World::plain_key()?)
}

/// `observed`, in the world given.
fn observed_in(w: World) -> Result<World, FixtureError> {
    trust(&w)?;
    fixed(w.sync(w.primary()))?;
    Ok(w)
}

/// `observed`, with a commit the peer pushed to primary that the world's clone
/// does not hold.
fn behind() -> Result<(World, git2::Oid), FixtureError> {
    let w = observed()?;
    let incoming = advance(&w.peer, MAIN, PEER_FILE)?;
    push_peer(&w.peer, &w.server, MAIN)?;
    Ok((w, incoming))
}

/// `behind`, with a second target: a ticket the clone saved and never published.
fn behind_with_a_context() -> Result<(World, git2::Oid), FixtureError> {
    let (mut w, incoming) = behind()?;
    w.context()?;
    Ok((w, incoming))
}

fn report(envelope: &Envelope<MutationDataDto>) -> &PollMutationDto {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Poll(data)) => data,
        other => panic!("{:?}: {other:?}", envelope.code),
    }
}

fn primary_target(report: &PollMutationDto) -> &PollTargetDto {
    report
        .targets
        .iter()
        .find(|target| target.target.as_deref() == Some(MAIN))
        .expect("primary is a target")
}

fn local_primary(w: &World) -> Result<git2::Oid, FixtureError> {
    fixed(w.repo()?.refname_to_id(MAIN))
}

/// A golden of a poll's answer, with what differs from run to run replaced.
fn golden_once(w: &World, name: &str, envelope: &Envelope<MutationDataDto>) {
    let mut replaced: Vec<(String, &str)> = Vec::new();
    let schema = match envelope.data.as_ref() {
        Some(MutationDataDto::Poll(data)) => {
            for target in &data.targets {
                for oid in [&target.before_oid, &target.after_oid]
                    .into_iter()
                    .flatten()
                {
                    replaced.push((oid.clone(), "<oid>"));
                }
            }
            replaced.push((data.completed_at.clone(), "<completed_at>"));
            if let Some(next) = &data.next_eligible_at {
                replaced.push((next.clone(), "<next_eligible_at>"));
            }
            Some(REPORT)
        }
        Some(_) => panic!("{name}: use the host's golden"),
        None => None,
    };
    let replaced: Vec<(&str, &str)> = replaced
        .iter()
        .map(|(found, placeholder)| (found.as_str(), *placeholder))
        .collect();
    let root = w.root.canonicalize().expect("the root");
    mutation_golden::assert_contract_at(
        name,
        root.to_str().expect("a UTF-8 root"),
        schema,
        &replaced,
        envelope,
    );
}

fn operation_of(envelope: &Envelope<MutationDataDto>) -> Result<OperationId, FixtureError> {
    fixed(OperationId::parse(
        envelope.operation_id.as_deref().ok_or(FixtureError)?,
    ))
}

fn row_of(w: &World, operation: &str) -> Result<(String, Option<String>), FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT phase, outcome FROM remote_operation_records WHERE operation_ulid = ?1",
        [operation],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))
}

fn poll_rows(w: &World) -> Result<i64, FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT COUNT(*) FROM remote_operation_records WHERE action = 'poll'",
        [],
        |row| row.get(0),
    ))
}

fn finished(w: &World, id: &str) -> Result<(), FixtureError> {
    let held = record(w, id).ok_or(FixtureError)?;
    assert_eq!(held.state, RequestState::Finished, "{id}");
    Ok(())
}

fn left_accepted(w: &World, id: &str) -> Result<(), FixtureError> {
    let held = record(w, id).ok_or(FixtureError)?;
    assert_eq!(held.state, RequestState::Accepted, "{id}");
    Ok(())
}

/// A new request polls: the stop held nothing.
fn a_new_request_polls(w: &World, id: &str) -> Result<(), FixtureError> {
    assert!(!reserved(w), "the reservation is free");
    let again = execute_once(w, id);
    assert_eq!(again.code, ResultCode::Ok, "{:?}", again.recovery);
    assert_ne!(again.operation_id, None);
    Ok(())
}

fn second_service(w: &World) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at(&w.data()))
}

fn dying_at(w: &World, point: FailurePoint) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        point,
    ))
}

fn once_ok() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let pushes = accepted(&w.server);
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert!(envelope.recovery.is_empty());
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(
        envelope.effects.publication,
        PublicationEffect::NotRequested
    );
    assert_eq!(envelope.scope.branch.as_deref(), Some("main"));
    let data = report(&envelope);
    assert_eq!(data.completion, PollMutationCompletion::Completed);
    assert_eq!(
        Some(data.operation_id.as_str()),
        envelope.operation_id.as_deref()
    );
    let primary = primary_target(data);
    assert_eq!(primary.disposition, PollTargetDisposition::FastForwarded);
    assert_eq!(primary.after_oid, Some(incoming.to_string()));
    assert_eq!(primary.kind, None);
    assert_eq!(local_primary(&w)?, incoming);
    assert_eq!(accepted(&w.server), pushes, "a poll never pushes");
    finished(&w, FIRST)?;
    golden_once(&w, "poll_once", &envelope);
    Ok(())
}

fn once_ok_replays() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::Ok);
    let opened = connections(&w);
    let rows = poll_rows(&w)?;
    let again = execute_once(&w, FIRST);
    assert_eq!(again, first);
    assert_eq!(connections(&w), opened, "a replay has no transport");
    assert_eq!(poll_rows(&w)?, rows);
    Ok(())
}

/// R-16: a target the poll left alone is data, and the answer is `ok`.
fn once_dirty_is_data() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let before = local_primary(&w)?;
    dirty_primary(&w.root)?;
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(
        primary_target(report(&envelope)).disposition,
        PollTargetDisposition::Dirty
    );
    assert_eq!(local_primary(&w)?, before);
    Ok(())
}

/// A malformed remote ref is a target with nothing but its disposition, and its
/// name is nowhere in the result.
fn once_malformed_is_data() -> Result<(), FixtureError> {
    const NAME: &str = "refs/heads/manyhands/ticket/not-a-ulid-sentinel";
    let (w, incoming) = behind()?;
    let bare = w.bare()?;
    fixed(bare.reference(NAME, incoming, false, "malformed"))?;
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::Ok);
    let malformed = report(&envelope)
        .targets
        .iter()
        .find(|target| target.disposition == PollTargetDisposition::Malformed)
        .ok_or(FixtureError)?;
    assert_eq!(
        (&malformed.target, &malformed.kind, &malformed.item_id),
        (&None, &None, &None)
    );
    assert_eq!(
        (&malformed.before_oid, &malformed.after_oid),
        (&None, &None)
    );
    assert!(!fixed(serde_json::to_string(&envelope))?.contains("not-a-ulid-sentinel"));
    Ok(())
}

/// The authoritative report first, and index pending second.
fn once_discovery_pending() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let dying = dying_at(&w, FailurePoint::BeforeIndexTransactionCommit)?;
    let envelope = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(envelope.code, ResultCode::DiscoveryPending);
    assert_eq!(envelope.outcome, Outcome::Partial);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(actions(&envelope), [RecoveryActionKind::IndexRefresh]);
    assert_eq!(
        report(&envelope).completion,
        PollMutationCompletion::Completed
    );
    assert_eq!(local_primary(&w)?, incoming);
    finished(&w, FIRST)?;
    golden_once(&w, "poll_once_discovery_pending", &envelope);
    assert_eq!(execute_once(&w, FIRST), envelope, "a replay is the same");
    Ok(())
}

/// Refused before the domain, with nothing written.
fn once_no_remote() -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let policy_before = status(&local)?;
    let envelope = called(&local.service, FIRST, once(&local.root));
    assert_eq!(envelope.code, ResultCode::PublicationRemoteRequired);
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.operation_id, None);
    assert!(envelope.recovery.is_empty());
    assert_eq!(request_rows(&local)?, 0);
    assert_eq!(status(&local)?, policy_before, "nothing is written");
    let rows: i64 = fixed(local.db()?.query_row(
        "SELECT COUNT(*) FROM remote_operation_records",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(rows, 0);
    let root = local.root.canonicalize().expect("the root");
    mutation_golden::assert_contract_at(
        "poll_once_no_remote",
        root.to_str().expect("a UTF-8 root"),
        None,
        &[],
        &envelope,
    );
    Ok(())
}

/// Explicit: a pause and a disabled policy each refuse nothing.
fn once_a_pause_does_not_refuse() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    fixed(
        w.db()?
            .execute_batch("UPDATE remote_polling_state SET paused = 1, enabled = 0"),
    )?;
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(local_primary(&w)?, incoming);
    let flags: (bool, bool) = fixed(w.db()?.query_row(
        "SELECT paused, enabled FROM remote_polling_state",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))?;
    assert_eq!(flags, (true, false), "the poll leaves the policy");
    Ok(())
}

fn assert_cancelled(w: &World, envelope: &Envelope<MutationDataDto>) -> Result<(), FixtureError> {
    assert_eq!(envelope.code, ResultCode::Cancelled);
    assert_eq!(envelope.outcome, Outcome::Cancelled);
    assert!(envelope.recovery.is_empty(), "{:?}", envelope.recovery);
    assert_eq!(
        report(envelope).completion,
        PollMutationCompletion::Cancelled
    );
    let operation = envelope.operation_id.as_deref().ok_or(FixtureError)?;
    assert_eq!(row_of(w, operation)?.0, "cancelled");
    Ok(())
}

/// The call's token, at the first safe point: no connection is opened.
fn once_cancelled_by_the_token() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let opened = connections(&w);
    let token = CancellationToken::new();
    let asked = token.clone();
    w.service
        .set_request_hook_for_testing(move || asked.cancel());
    let envelope = call(
        &w,
        &w.service,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &token,
    );
    assert_cancelled(&w, &envelope)?;
    assert_eq!(connections(&w), opened);
    finished(&w, FIRST)?;
    golden_once(&w, "poll_once_cancelled", &envelope);
    assert_eq!(execute_once(&w, FIRST), envelope, "a replay is the same");
    a_new_request_polls(&w, SECOND)
}

/// `request cancel` with no row yet: only the record's flag says so.
fn once_cancelled_by_the_flag() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let opened = connections(&w);
    let other = second_service(&w)?;
    w.service.set_request_hook_for_testing(move || {
        let asked = other.cancel_request(request(FIRST));
        assert!(asked.data.expect("the request").requested);
    });
    let envelope = execute_once(&w, FIRST);
    assert_cancelled(&w, &envelope)?;
    assert_eq!(connections(&w), opened);
    assert!(!record(&w, FIRST).ok_or(FixtureError)?.cancel_requested);
    a_new_request_polls(&w, SECOND)
}

/// `request cancel` while the poll runs, between two targets: what it moved
/// stands, and the data names it.
fn once_cancelled_by_request_cancel_while_running() -> Result<(), FixtureError> {
    let (w, incoming) = behind_with_a_context()?;
    let other = second_service(&w)?;
    let mut armed = true;
    w.service
        .set_poll_boundary_hook_for_testing(move |point, ordinal| {
            if armed && point == FailurePoint::PollBeforeTarget && ordinal == Some(1) {
                armed = false;
                let asked = other.cancel_request(request(FIRST));
                assert!(asked.data.expect("the request").requested);
            }
        });
    let envelope = execute_once(&w, FIRST);
    assert_cancelled(&w, &envelope)?;
    let primary = primary_target(report(&envelope));
    assert_eq!(primary.disposition, PollTargetDisposition::FastForwarded);
    assert_eq!(local_primary(&w)?, incoming);
    finished(&w, FIRST)?;
    a_new_request_polls(&w, SECOND)
}

/// The token, after the last target: the last safe point reads it, with no
/// row flag to go by.
fn once_cancelled_at_the_last_safe_point() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let token = CancellationToken::new();
    let asked = token.clone();
    w.service
        .set_poll_boundary_hook_for_testing(move |point, _| {
            if point == FailurePoint::PollBeforeDiscovery {
                asked.cancel();
            }
        });
    let envelope = call(
        &w,
        &w.service,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &token,
    );
    assert_cancelled(&w, &envelope)?;
    assert_eq!(local_primary(&w)?, incoming);
    a_new_request_polls(&w, SECOND)
}

/// A flag that cannot be read is not taken for "no": the poll stops at the safe
/// point, and the same request goes on.
fn once_flag_unreadable() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let before = local_primary(&w)?;
    let dying = dying_at(&w, FailurePoint::BeforeRequestCancellationRead)?;
    let stopped = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(local_primary(&w)?, before);
    left_accepted(&w, FIRST)?;
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    let finished_after = execute_once(&w, FIRST);
    assert_eq!(finished_after.code, ResultCode::Ok);
    assert_eq!(local_primary(&w)?, incoming);
    Ok(())
}

/// Asks the poll to give way before the target `at`, as a manual request asks an
/// automatic poll: the row ends `interrupted`, with its generation.
fn interrupted_before_target(w: &World, at: usize) {
    let registry = w.data().join(REGISTRY_FILE);
    let mut armed = true;
    w.service
        .set_poll_boundary_hook_for_testing(move |point, ordinal| {
            if armed && point == FailurePoint::PollBeforeTarget && ordinal == Some(at) {
                armed = false;
                rusqlite::Connection::open(&registry)
                    .expect("the registry")
                    .execute(
                        "UPDATE remote_operation_records SET yield_requested = 1",
                        [],
                    )
                    .expect("the row is asked");
            }
        });
}

fn once_interrupted() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let before = local_primary(&w)?;
    interrupted_before_target(&w, 0);
    let stopped = execute_once(&w, FIRST);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Blocked, "nothing moved: exit 3");
    assert_eq!(
        report(&stopped).completion,
        PollMutationCompletion::Interrupted
    );
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(local_primary(&w)?, before);
    left_accepted(&w, FIRST)?;
    golden_once(&w, "poll_once_interrupted", &stopped);
    let again = execute_once(&w, FIRST);
    assert_eq!(again.code, ResultCode::Ok);
    assert_eq!(again.operation_id, stopped.operation_id, "the same row");
    assert_eq!(local_primary(&w)?, incoming);
    finished(&w, FIRST)
}

fn once_interrupted_after_a_target_moved() -> Result<(), FixtureError> {
    let (w, incoming) = behind_with_a_context()?;
    interrupted_before_target(&w, 1);
    let stopped = execute_once(&w, FIRST);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Partial, "a target moved: exit 4");
    assert_eq!(
        primary_target(report(&stopped)).disposition,
        PollTargetDisposition::FastForwarded
    );
    assert_eq!(local_primary(&w)?, incoming);
    left_accepted(&w, FIRST)?;
    // A restart reports what its own pass saw (known limit).
    let again = execute_once(&w, FIRST);
    assert_eq!(again.code, ResultCode::Ok);
    assert_eq!(
        primary_target(report(&again)).disposition,
        PollTargetDisposition::Current
    );
    Ok(())
}

/// The endpoint changes while the transport is held, and is put back: the
/// domain interrupts the row and returns an error, with no report.
fn once_err_interrupted() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let url = w.server.url();
    let hold = w.server.hold_advertisement()?;
    let data = w.data();
    let root = w.root.clone();
    let running = std::thread::spawn(move || {
        let service = RepositoryService::open_at(&data).expect("a service");
        service.execute(
            MutationCall {
                request_id: request(FIRST),
                confirmation: None,
                mutation: once(&root),
            },
            &mut SessionCredentials::new(Provider),
            &mut NoProgress,
            &CancellationToken::new(),
        )
    });
    hold.wait_until_held()?;
    let repo = w.repo()?;
    fixed(repo.remote_set_pushurl("origin", Some(&format!("{url}-changed"))))?;
    hold.release()?;
    let stopped = joined(&w, running)?;
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.data, None);
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    left_accepted(&w, FIRST)?;
    let operation = stopped.operation_id.as_deref().ok_or(FixtureError)?;
    assert_eq!(row_of(&w, operation)?.0, "interrupted");
    fixed(repo.remote_set_pushurl("origin", None))?;
    let again = execute_once(&w, FIRST);
    assert_eq!(again.code, ResultCode::Ok);
    assert_eq!(local_primary(&w)?, incoming);
    Ok(())
}

/// A held poll, `request cancel`, and the same request: `cancelled`, final.
fn once_held_then_cancelled() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    interrupted_before_target(&w, 0);
    let stopped = execute_once(&w, FIRST);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    let asked = w.service.cancel_request(request(FIRST));
    assert!(asked.data.ok_or(FixtureError)?.requested);
    let opened = connections(&w);
    let ended = execute_once(&w, FIRST);
    assert_cancelled(&w, &ended)?;
    assert_eq!(connections(&w), opened);
    finished(&w, FIRST)?;
    a_new_request_polls(&w, SECOND)
}

/// A held poll whose endpoints have since changed. The call that finds the change
/// raises the generation and the restart it then asks for is refused, so the
/// record stays once. From then on the row is fenced and a restart would refuse
/// it for good: the next answer is final, as `request cancel` would finish it.
fn once_fenced_is_final() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    interrupted_before_target(&w, 0);
    let stopped = execute_once(&w, FIRST);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    let url = w.server.url();
    fixed(
        w.repo()?
            .remote_set_pushurl("origin", Some(&format!("{url}-changed"))),
    )?;
    let finding = execute_once(&w, FIRST);
    assert_eq!(finding.code, ResultCode::RecoveryRequired);
    left_accepted(&w, FIRST)?;
    let ended = execute_once(&w, FIRST);
    assert_eq!(ended.code, ResultCode::ExternalChange);
    assert!(ended.recovery.is_empty(), "{:?}", ended.recovery);
    finished(&w, FIRST)?;
    a_new_request_polls(&w, SECOND)?;
    assert_eq!(local_primary(&w)?, incoming);
    Ok(())
}

/// The category as a later call reads it: a request left accepted over a row that
/// ended with `category`. The row's outcome is staged, and the domain reads it.
fn stored(category: &str, code: ResultCode) -> Result<World, FixtureError> {
    let (w, _) = behind()?;
    let dying = dying_at(&w, FailurePoint::BeforeRequestSettlement)?;
    let lost = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(lost.code, ResultCode::InternalError, "the answer is lost");
    left_accepted(&w, FIRST)?;
    fixed(w.db()?.execute(
        "UPDATE remote_operation_records SET phase = 'failed', outcome = ?1 WHERE action = 'poll'",
        [category],
    ))?;
    let opened = connections(&w);
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, code, "{category}");
    assert_eq!(actions(&envelope), [], "{category}: a final answer");
    assert_eq!(
        report(&envelope).completion,
        PollMutationCompletion::Completed
    );
    assert_eq!(connections(&w), opened, "no transport");
    finished(&w, FIRST)?;
    let replayed = execute_once(&w, FIRST);
    assert_eq!(replayed, envelope, "a replay is the same");
    a_new_request_polls(&w, SECOND)?;
    Ok(w)
}

fn once_stored_configuration_required() -> Result<(), FixtureError> {
    stored(
        "configuration_required",
        ResultCode::PublicationRemoteRequired,
    )
    .map(|_| ())
}

fn once_stored_selected_key_unavailable() -> Result<(), FixtureError> {
    stored("selected_key_unavailable", ResultCode::NoSelectedKey).map(|_| ())
}

fn once_stored_unlock_required() -> Result<(), FixtureError> {
    stored("unlock_required", ResultCode::UnlockRequired).map(|_| ())
}

fn once_stored_host_approval_required() -> Result<(), FixtureError> {
    stored("host_approval_required", ResultCode::HostApprovalRequired).map(|_| ())
}

fn once_stored_transport_unavailable() -> Result<(), FixtureError> {
    let w = stored("transport_unavailable", ResultCode::TransportUnavailable)?;
    let replayed = execute_once(&w, FIRST);
    assert!(replayed.recovery.is_empty(), "{:?}", replayed.recovery);
    Ok(())
}

fn once_stored_protocol_rejected() -> Result<(), FixtureError> {
    let w = stored("protocol_rejected", ResultCode::RemoteUnavailable)?;
    let replayed = execute_once(&w, FIRST);
    assert!(replayed.recovery.is_empty(), "{:?}", replayed.recovery);
    Ok(())
}

fn once_stored_repository_unavailable() -> Result<(), FixtureError> {
    let w = stored("repository_unavailable", ResultCode::RepositoryInaccessible)?;
    // The recovery tables' row: the stored answer, read again with no transport.
    let replayed = execute_once(&w, FIRST);
    golden_once(&w, "poll_once_repository_inaccessible", &replayed);
    Ok(())
}

/// The first call's `Err(Transport)` over the row the domain ended: `Final`, on a
/// row the journal reads as pending. A replay offers nothing, holds no
/// reservation, and a new request polls (R-18).
fn once_transport_stop_is_final() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    // Longer than the transport waits for a server.
    w.server.stall_at(
        FixtureBoundary::Advertisement,
        std::time::Duration::from_secs(45),
    );
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::TransportUnavailable);
    assert_eq!(first.outcome, Outcome::Error);
    assert!(first.recovery.is_empty(), "{:?}", first.recovery);
    assert_eq!(first.data, None);
    golden_once(&w, "poll_once_transport_unavailable", &first);
    let operation = first.operation_id.as_deref().ok_or(FixtureError)?;
    assert_eq!(
        row_of(&w, operation)?,
        (
            "failed".to_owned(),
            Some("transport_unavailable".to_owned())
        )
    );
    // The row the record was finished over is one the journal calls pending.
    let scope = request_store::ScopeKey::for_repository(&w.root).ok_or(FixtureError)?;
    let journal = fixed(w.service.journal_row(
        OperationFamily::Remote,
        &scope,
        operation_of(&first)?,
    ))?;
    assert!(journal.in_flight(), "{journal:?}");
    finished(&w, FIRST)?;
    assert!(!reserved(&w));
    w.server.clear_fault();
    let opened = connections(&w);
    let replayed = execute_once(&w, FIRST);
    assert_eq!(replayed, first, "stored, and offering nothing");
    assert_eq!(connections(&w), opened);
    a_new_request_polls(&w, SECOND)?;
    assert_eq!(local_primary(&w)?, incoming);
    Ok(())
}

/// A row the first call's error named by its own code.
fn once_key_rejected() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    w.server.accept_anonymous();
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::KeyRejected);
    assert_eq!(first.outcome, Outcome::Blocked);
    assert!(first.recovery.is_empty(), "{:?}", first.recovery);
    finished(&w, FIRST)?;
    golden_once(&w, "poll_once_key_rejected", &first);
    assert!(!reserved(&w));
    Ok(())
}

/// The same stop, with the first answer lost: the later call reads the stored
/// category's code, which names it differently.
fn once_key_rejected_reentered() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    w.server.accept_anonymous();
    let dying = dying_at(&w, FailurePoint::BeforeRequestSettlement)?;
    let lost = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(lost.code, ResultCode::InternalError);
    left_accepted(&w, FIRST)?;
    let later = execute_once(&w, FIRST);
    assert_eq!(later.code, ResultCode::NoSelectedKey);
    assert!(later.recovery.is_empty(), "{:?}", later.recovery);
    assert_eq!(report(&later).completion, PollMutationCompletion::Completed);
    finished(&w, FIRST)?;
    golden_once(&w, "poll_once_key_rejected_reentered", &later);
    Ok(())
}

fn once_no_selected_key() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let cleared = execute(&w, THIRD, Mutation::KeyClear(KeyClearInput {}));
    assert_eq!(cleared.code, ResultCode::Ok);
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::NoSelectedKey);
    assert!(first.recovery.is_empty(), "{:?}", first.recovery);
    finished(&w, FIRST)?;
    assert!(!reserved(&w));
    Ok(())
}

/// A locked key with nothing to unlock it: final, and a new request at a terminal
/// polls.
fn once_unlock_required() -> Result<(), FixtureError> {
    let w = observed_in(World::new()?)?;
    advance(&w.peer, MAIN, PEER_FILE)?;
    push_peer(&w.peer, &w.server, MAIN)?;
    let first = call(
        &w,
        &w.service,
        FIRST,
        once(&w.root),
        NoPassphrase,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(first.code, ResultCode::UnlockRequired);
    assert!(first.recovery.is_empty(), "{:?}", first.recovery);
    finished(&w, FIRST)?;
    golden_once(&w, "poll_once_unlock_required", &first);
    let replayed = execute_once(&w, FIRST);
    assert_eq!(
        replayed, first,
        "with a passphrase to hand, still the stored stop"
    );
    a_new_request_polls(&w, SECOND)
}

/// A host that isn't trusted: the first answer names it and offers its approval;
/// the new request then polls.
fn once_host_approval_required() -> Result<(), FixtureError> {
    let w = World::plain_key()?;
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::HostApprovalRequired);
    assert_eq!(actions(&first), [RecoveryActionKind::HostApprove]);
    finished(&w, FIRST)?;
    assert_golden(
        &w,
        "poll_once_host_approval_required",
        Some(SYNCHRONIZATION_SCHEMA),
        &first,
    );
    let replayed = execute_once(&w, FIRST);
    assert_eq!(replayed.code, first.code);
    assert_eq!(replayed.data, first.data);
    assert!(replayed.recovery.is_empty(), "no action is stored");
    approve(&w)?;
    a_new_request_polls(&w, SECOND)
}

/// The publication remote goes between the checks and the domain's call: the
/// plan can't be made, so there is no row, and the record goes. The domain's
/// stored state is written (known limit).
fn once_transport_with_no_row() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let config = w.root.join(".manyhands/config.toml");
    let source = fixed(std::fs::read_to_string(&config))?;
    let without = source.replace("publication_remote = \"origin\"\n", "");
    assert_ne!(without, source);
    let removed = config.clone();
    w.service
        .set_request_hook_for_testing(move || std::fs::write(removed, without).expect("written"));
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::PublicationRemoteRequired);
    assert!(first.recovery.is_empty(), "{:?}", first.recovery);
    assert!(record(&w, FIRST).is_none(), "deleted");
    assert_eq!(poll_rows(&w)?, 0);
    golden_once(&w, "poll_once_transport_with_no_row", &first);
    fixed(std::fs::write(&config, source))?;
    a_new_request_polls(&w, FIRST)
}

/// A remote whose location the transport can't use is found once the poll has
/// reserved: a row after the call, so `Final`, under the error's own code.
fn once_invalid_remote() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let repo = w.repo()?;
    fixed(repo.remote_set_url("origin", "https://example.invalid/not-ssh.git"))?;
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::InvalidRemote);
    assert!(first.recovery.is_empty(), "{:?}", first.recovery);
    finished(&w, FIRST)?;
    assert_eq!(poll_rows(&w)?, 1);
    golden_once(&w, "poll_once_invalid_remote", &first);
    Ok(())
}

/// `Err(Repository)` over a row in flight: the error's code, and the record stays.
fn once_repository_error_held() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let dying = dying_at(&w, FailurePoint::PollBeforeTarget)?;
    let stopped = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(stopped.code, ResultCode::InternalError);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    left_accepted(&w, FIRST)?;
    assert!(reserved(&w), "the row holds the reservation");
    let again = execute_once(&w, FIRST);
    assert_eq!(again.code, ResultCode::Ok);
    assert_eq!(local_primary(&w)?, incoming);
    assert!(!reserved(&w));
    Ok(())
}

/// `Err(Repository)` over a row that completed: the record stays, and the next
/// call reads what is stored, with no transport.
fn once_repository_error_after_completion() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let dying = dying_at(&w, FailurePoint::PollAfterCompletion)?;
    let stopped = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(stopped.code, ResultCode::InternalError);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    left_accepted(&w, FIRST)?;
    assert_eq!(local_primary(&w)?, incoming);
    let opened = connections(&w);
    // No refresh ran, so what is stored reads as the same report with its index
    // behind: the table's `IndexPending` row.
    let read = execute_once(&w, FIRST);
    assert_eq!(read.code, ResultCode::DiscoveryPending);
    assert_eq!(actions(&read), [RecoveryActionKind::IndexRefresh]);
    assert_eq!(read.operation_id, stopped.operation_id);
    assert_eq!(connections(&w), opened);
    finished(&w, FIRST)
}

/// A poll killed mid-pass: its service is gone and its row is left in flight,
/// holding the reservation. A resend under its ID takes the same row up with a
/// raised epoch, and the request finishes. A kill itself is for the CLI's interruption cases, through
/// the binary; a stop at the same boundary leaves the same row.
fn once_killed_resent_restarts() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let before = local_primary(&w)?;
    let dying = dying_at(&w, FailurePoint::PollBeforeTarget)?;
    let stopped = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    drop(dying);
    let operation = stopped.operation_id.clone().ok_or(FixtureError)?;
    let epoch = |w: &World| -> Result<i64, FixtureError> {
        fixed(w.db()?.query_row(
            "SELECT owner_epoch FROM remote_operation_records WHERE operation_ulid = ?1",
            [operation.as_str()],
            |row| row.get(0),
        ))
    };
    let held = epoch(&w)?;
    let (phase, outcome) = row_of(&w, &operation)?;
    assert!(
        !matches!(
            phase.as_str(),
            "completed" | "cancelled" | "failed" | "interrupted"
        ),
        "{phase}"
    );
    assert_eq!(outcome, None);
    assert!(reserved(&w));
    assert_eq!(local_primary(&w)?, before);
    left_accepted(&w, FIRST)?;
    let again = execute_once(&w, FIRST);
    assert_eq!(again.code, ResultCode::Ok, "{:?}", again.recovery);
    assert_eq!(again.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(poll_rows(&w)?, 1);
    assert!(epoch(&w)? > held, "the row was restarted");
    assert_eq!(local_primary(&w)?, incoming);
    assert!(!reserved(&w));
    finished(&w, FIRST)
}

fn reserve_another(w: &World, priority: RemoteOperationPriority) -> Result<(), FixtureError> {
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    let held = fixed(w.service.reserve_remote_operation_with_priority(
        &w.root,
        OperationId::new(),
        &RemoteOperationTarget::for_poll(&plan),
        priority,
    ))?;
    assert!(matches!(held, RemoteReservationOutcome::Reserved(_)));
    Ok(())
}

fn once_busy() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    reserve_another(&w, RemoteOperationPriority::Manual)?;
    let opened = connections(&w);
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::Busy);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestRetry]);
    assert!(record(&w, FIRST).is_none());
    assert_eq!(connections(&w), opened);
    golden_once(&w, "poll_once_busy", &envelope);
    Ok(())
}

/// Decision 24: the repository's lease held when a fetch comes to move its
/// tracking refs is another's short local work, and no protocol failure. Nothing
/// is stored as this poll's result, so no failure replays and no backoff grows,
/// and the same request polls once the lease is free.
fn once_busy_on_a_held_lease() -> Result<(), FixtureError> {
    use fs4::fs_std::FileExt;
    const TRACKING: &str = "refs/remotes/origin/main";
    let (w, incoming) = behind()?;
    let repo = w.repo()?;
    let tracked = fixed(repo.refname_to_id(TRACKING))?;
    let hold = w.server.hold_transfer_after(1)?;
    let (data, root) = (w.data(), w.root.clone());
    let running = std::thread::spawn(move || {
        let service = RepositoryService::open_at(&data).expect("a service");
        service.execute(
            MutationCall {
                request_id: request(FIRST),
                confirmation: None,
                mutation: once(&root),
            },
            &mut SessionCredentials::new(Provider),
            &mut NoProgress,
            &CancellationToken::new(),
        )
    });
    hold.wait_until_held()?;
    let lease = fixed(
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(repo.commondir().join("manyhands-operation.lock")),
    )?;
    assert!(fixed(FileExt::try_lock_exclusive(&lease))?);
    hold.release()?;
    let envelope = joined(&w, running)?;
    assert_eq!(envelope.code, ResultCode::Busy, "{:?}", envelope.data);
    assert_eq!(envelope.outcome, Outcome::Error, "a transient code");
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestRetry]);
    assert_eq!(fixed(repo.refname_to_id(TRACKING))?, tracked);
    left_accepted(&w, FIRST)?;
    let operation = envelope.operation_id.as_deref().ok_or(FixtureError)?;
    let stored: Option<String> = fixed(w.db()?.query_row(
        "SELECT outcome FROM remote_operation_records WHERE operation_ulid = ?1",
        [operation],
        |row| row.get(0),
    ))?;
    assert_eq!(stored, None, "nothing is stored as the poll's result");
    fixed(FileExt::unlock(&lease))?;
    let polled = execute_once(&w, FIRST);
    assert_eq!(polled.code, ResultCode::Ok, "{:?}", polled.data);
    assert_eq!(polled.operation_id.as_deref(), Some(operation));
    assert_eq!(fixed(repo.refname_to_id(TRACKING))?, incoming);
    finished(&w, FIRST)?;
    assert_eq!(execute_once(&w, FIRST), polled, "a replay is the same");
    Ok(())
}

fn once_poll_yielding() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    reserve_another(&w, RemoteOperationPriority::Poll)?;
    let opened = connections(&w);
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::PollYielding);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestRetry]);
    assert!(record(&w, FIRST).is_none());
    assert_eq!(connections(&w), opened);
    golden_once(&w, "poll_once_poll_yielding", &envelope);
    Ok(())
}

fn once_conflict_pending() -> Result<(), FixtureError> {
    let w = observed()?;
    let peer_root = w.peer.workdir().ok_or(FixtureError)?.to_owned();
    commit_in(&peer_root, MAIN, "conflict", b"incoming")?;
    push_peer(&w.peer, &w.server, MAIN)?;
    commit_in(&w.root, MAIN, "conflict", b"local")?;
    let synced = execute(&w, SECOND, repo_sync(&w));
    assert_eq!(
        synced.code,
        ResultCode::RecoveryRequired,
        "the merge conflicts"
    );
    let opened = connections(&w);
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(envelope.data, None);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RepoInspect]);
    assert!(record(&w, FIRST).is_none());
    assert_eq!(connections(&w), opened);
    Ok(())
}

/// The domain's answer to a pending local synchronization, pinned.
fn once_pending_local_sync() -> Result<(), FixtureError> {
    let w = remote(Verb::Close)?;
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\n\
         title: Primary\ntype: task\nstatus: open\n---\nBody of Primary.\n"
    );
    commit_in(
        &w.root,
        MAIN,
        &format!(".manyhands/tickets/{ITEM}/ticket.md"),
        source.as_bytes(),
    )?;
    let id: canonical::ItemId = fixed(ITEM.parse())?;
    let observed = fixed(w.service.observe_local_context_sync(
        &fixed(w.service.resolve_repository(&w.root))?,
        AuthoringKind::Ticket,
        &id,
    ))?;
    let stopped = execute(
        &w,
        SECOND,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: w.root.clone(),
            kind: AuthoringKind::Ticket,
            id,
            observation: observed.observation,
        }),
    );
    assert_eq!(stopped.code, ResultCode::RecoveryRequired, "the sync pends");
    let opened = connections(&w);
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert!(
        actions(&envelope).contains(&RecoveryActionKind::RepoInspect),
        "{:?}",
        envelope.recovery
    );
    assert!(record(&w, FIRST).is_none());
    assert_eq!(connections(&w), opened);
    assert_eq!(poll_rows(&w)?, 0);
    Ok(())
}

/// A poll the boundary began and that was left held: `operation resume` names its
/// request, and that request is what goes on.
fn once_resume_names_a_held_poll() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let dying = dying_at(&w, FailurePoint::PollBeforeTarget)?;
    let stopped = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    left_accepted(&w, FIRST)?;
    let operation = operation_of(&stopped)?;
    let resumed = execute(
        &w,
        SECOND,
        Mutation::OperationResume(OperationResumeInput {
            root: w.root.clone(),
            operation_id: operation,
        }),
    );
    assert_eq!(resumed.code, ResultCode::OriginalRequestRequired);
    assert_eq!(actions(&resumed), [RecoveryActionKind::RequestRetry]);
    assert!(
        resumed.recovery[0]
            .arguments
            .values()
            .any(|value| value == FIRST),
        "names the request: {:?}",
        resumed.recovery
    );
    assert_eq!(execute_once(&w, FIRST).code, ResultCode::Ok);
    Ok(())
}

/// A poll whose stop was final holds nothing a resume could take up.
fn once_resume_of_a_failed_poll() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    w.server.accept_anonymous();
    let first = execute_once(&w, FIRST);
    assert_eq!(first.code, ResultCode::KeyRejected);
    let resumed = execute(
        &w,
        SECOND,
        Mutation::OperationResume(OperationResumeInput {
            root: w.root.clone(),
            operation_id: operation_of(&first)?,
        }),
    );
    assert_eq!(resumed.code, ResultCode::InvalidInput);
    assert!(resumed.recovery.is_empty());
    Ok(())
}

/// A pause during a running poll doesn't stop that poll, and
/// applies to the next automatic one.
fn once_pause_during_a_running_poll() -> Result<(), FixtureError> {
    let (w, incoming) = behind()?;
    let other = second_service(&w)?;
    let root = w.root.clone();
    w.service
        .set_poll_boundary_hook_for_testing(move |point, ordinal| {
            if point == FailurePoint::PollBeforeTarget && ordinal == Some(0) {
                let paused = other.execute(
                    MutationCall {
                        request_id: request(SECOND),
                        confirmation: None,
                        mutation: pause(&root),
                    },
                    &mut SessionCredentials::new(Provider),
                    &mut NoProgress,
                    &CancellationToken::new(),
                );
                assert_eq!(paused.code, ResultCode::Ok);
            }
        });
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(
        primary_target(report(&envelope)).disposition,
        PollTargetDisposition::FastForwarded
    );
    assert_eq!(local_primary(&w)?, incoming);
    let repository = fixed(w.service.resolve_repository(&w.root))?;
    assert!(fixed(w.service.polling_status(&repository))?.paused);
    let automatic = fixed(w.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: w.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Automatic,
            approval: None,
            restart: false,
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    assert!(
        matches!(
            automatic,
            RemotePollResult::Complete(RemotePollOutcome::NotEligible {
                reason: PollIneligibility::Paused,
                ..
            })
        ),
        "{automatic:?}"
    );
    Ok(())
}

pub(super) fn conforming_once(
    rule: &mut dyn FnMut(&conformance::Subject<'_>),
) -> Result<(), String> {
    conforming(
        |w, ask| match ask {
            conformance::Ask::Same | conformance::Ask::Unconsented => once(&w.root),
            // The command has no input but its repository: another command.
            conformance::Ask::Changed => repo_sync(w),
            // A directory that is no repository.
            conformance::Ask::Refused => once(&w.data()),
        },
        rule,
    )
}

fn once_conformance() -> Result<(), FixtureError> {
    conformance_row("poll once")
}

/// A press during the transfer stops the transfer, not only the next safe point:
/// what the fetch was bringing isn't kept. The registry isn't read in that
/// callback, so the token is what it looks at, and the row's flag `poll_progress`.
fn once_token_stops_a_transfer() -> Result<(), FixtureError> {
    const TRACKING: &str = "refs/remotes/origin/main";
    let (w, _) = behind()?;
    let tracked = fixed(w.repo()?.refname_to_id(TRACKING))?;
    let hold = w.server.hold_transfer_after(1)?;
    let token = CancellationToken::new();
    let asked = token.clone();
    let (data, root) = (w.data(), w.root.clone());
    let running = std::thread::spawn(move || {
        let service = RepositoryService::open_at(&data).expect("a service");
        service.execute(
            MutationCall {
                request_id: request(FIRST),
                confirmation: None,
                mutation: once(&root),
            },
            &mut SessionCredentials::new(Provider),
            &mut NoProgress,
            &asked,
        )
    });
    hold.wait_until_held()?;
    token.cancel();
    // Past `POLL_PROGRESS_THROTTLE`, 250 ms, which the callback's first call may
    // have begun.
    std::thread::sleep(std::time::Duration::from_millis(300));
    hold.release()?;
    let envelope = joined(&w, running)?;
    assert_cancelled(&w, &envelope)?;
    assert_eq!(fixed(w.repo()?.refname_to_id(TRACKING))?, tracked);
    Ok(())
}

/// `Err(Repository)` over a row that was cancelled: not stored as the request's
/// answer, which would replay for good. The next call reads `cancelled`.
fn once_repository_error_over_a_cancelled_row() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let dying = dying_at(&w, FailurePoint::PollAfterCompletion)?;
    let token = CancellationToken::new();
    let asked = token.clone();
    dying.set_request_hook_for_testing(move || asked.cancel());
    let stopped = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &token,
    );
    assert_eq!(stopped.code, ResultCode::InternalError);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    left_accepted(&w, FIRST)?;
    let operation = stopped.operation_id.as_deref().ok_or(FixtureError)?;
    assert_eq!(row_of(&w, operation)?.0, "cancelled");
    let read = execute_once(&w, FIRST);
    assert_cancelled(&w, &read)?;
    finished(&w, FIRST)?;
    assert_eq!(execute_once(&w, FIRST), read, "a replay is the same");
    a_new_request_polls(&w, SECOND)
}

/// A ticket the peer published that the world's clone doesn't hold.
fn published_by_the_peer(w: &World) -> Result<git2::Oid, FixtureError> {
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\n\
         title: Peer\ntype: task\nstatus: open\n---\nBody of Peer.\n"
    );
    let tip = fixed(w.peer.refname_to_id(MAIN))?;
    let new = commit_path(
        &w.peer,
        tip,
        &format!(".manyhands/tickets/{ITEM}/ticket.md"),
        source.as_bytes(),
    )?;
    fixed(
        w.peer
            .reference(TICKET_BRANCH, new, true, "the peer's ticket"),
    )?;
    push_peer(&w.peer, &w.server, TICKET_BRANCH)?;
    Ok(new)
}

fn one_worktree(w: &World, tip: git2::Oid) -> Result<(), FixtureError> {
    let repo = w.repo()?;
    let worktrees = fixed(repo.worktrees())?;
    let names: Vec<_> = worktrees.iter().flatten().collect();
    assert_eq!(names, [ITEM], "one worktree");
    assert_eq!(fixed(repo.refname_to_id(TICKET_BRANCH))?, tip);
    assert!(Verb::Close.worktree(&w.root).join(".manyhands").exists());
    Ok(())
}

/// The resend that matters: A has passed its `owned` check and holds the lease,
/// inside a materialization. B raises the epoch and resets the phase, and its own
/// pass can't move a tracking reference or reach a target while A holds the lease,
/// so it stops. A finishes its effect, and is refused its row. A new request finds
/// the worktree there. Git holds one branch and one worktree: takeover matches a
/// kill.
fn once_resend_during_a_materialization() -> Result<(), FixtureError> {
    let w = observed()?;
    let tip = published_by_the_peer(&w)?;
    let mut other = Some(second_service(&w)?);
    let root = w.root.clone();
    let registry = w.data().join(REGISTRY_FILE);
    let mut armed = true;
    let during = std::sync::Arc::new(std::sync::Mutex::new(None));
    let seen = during.clone();
    w.service
        .set_poll_boundary_hook_for_testing(move |point, _| {
            if armed && point == FailurePoint::PollAfterMaterializationIntent {
                armed = false;
                let epoch = |db: &rusqlite::Connection| -> i64 {
                    db.query_row(
                        "SELECT owner_epoch FROM remote_operation_records WHERE action = 'poll'",
                        [],
                        |row| row.get(0),
                    )
                    .expect("the row")
                };
                let db = rusqlite::Connection::open(&registry).expect("the registry");
                let before = epoch(&db);
                // On a thread of its own, as a second process is.
                let (service, again) = (other.take().expect("one call"), root.clone());
                let resent = std::thread::spawn(move || {
                    service.execute(
                        MutationCall {
                            request_id: request(FIRST),
                            confirmation: None,
                            mutation: once(&again),
                        },
                        &mut SessionCredentials::new(Provider),
                        &mut NoProgress,
                        &CancellationToken::new(),
                    )
                })
                .join()
                .expect("the resend");
                *seen.lock().expect("the answer") = Some((resent.code, before, epoch(&db)));
            }
        });
    let a = execute_once(&w, FIRST);
    let (code, before, after) = during
        .lock()
        .expect("the answer")
        .take()
        .ok_or(FixtureError)?;
    // B's fetch needs the lease to move tracking references, and A holds it:
    // `busy` (decision 24), with nothing of B's at the target, nothing stored
    // as a failure, and the request held for its retry.
    assert_eq!(code, ResultCode::Busy);
    assert!(after > before, "B raised the epoch");
    assert_eq!(a.code, ResultCode::RecoveryRequired, "A lost its row");
    one_worktree(&w, tip)?;
    left_accepted(&w, FIRST)?;
    let retried = execute_once(&w, FIRST);
    assert_eq!(retried.code, ResultCode::Ok, "{:?}", retried.data);
    finished(&w, FIRST)?;
    assert_eq!(poll_rows(&w)?, 1);
    one_worktree(&w, tip)
}

/// Interrupted past a materialization, then restarted: a target it made reads as
/// partial, and the restart reports what its own pass saw over one worktree.
fn once_interrupted_then_one_worktree() -> Result<(), FixtureError> {
    let w = observed()?;
    let tip = published_by_the_peer(&w)?;
    let registry = w.data().join(REGISTRY_FILE);
    let mut armed = true;
    w.service
        .set_poll_boundary_hook_for_testing(move |point, _| {
            if armed && point == FailurePoint::PollBeforeDiscovery {
                armed = false;
                rusqlite::Connection::open(&registry)
                    .expect("the registry")
                    .execute(
                        "UPDATE remote_operation_records SET yield_requested = 1",
                        [],
                    )
                    .expect("the row is asked");
            }
        });
    let stopped = execute_once(&w, FIRST);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert!(
        report(&stopped)
            .targets
            .iter()
            .any(|target| target.disposition == PollTargetDisposition::Materialized)
    );
    one_worktree(&w, tip)?;
    let again = execute_once(&w, FIRST);
    assert_eq!(again.code, ResultCode::Ok);
    assert!(
        report(&again)
            .targets
            .iter()
            .all(|target| target.disposition != PollTargetDisposition::Materialized),
        "its own pass made nothing"
    );
    one_worktree(&w, tip)
}

/// `request cancel` of a request whose row ended `failed`, with the generation
/// moved since: it gives what its own next call gives, the stored category.
fn once_request_cancel_of_a_failed_fenced_row() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let dying = dying_at(&w, FailurePoint::BeforeRequestSettlement)?;
    w.server.accept_anonymous();
    let lost = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(lost.code, ResultCode::InternalError);
    let phase: String = fixed(w.db()?.query_row(
        "SELECT phase FROM remote_operation_records WHERE action = 'poll'",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(phase, "failed");
    fixed(w.db()?.execute_batch(
        "UPDATE remote_polling_state SET configuration_generation = configuration_generation + 1",
    ))?;
    let asked = w.service.cancel_request(request(FIRST));
    let asked = asked.data.ok_or(FixtureError)?;
    assert_eq!(asked.state, RequestState::Accepted, "not finished for it");
    left_accepted(&w, FIRST)?;
    let next = execute_once(&w, FIRST);
    assert_eq!(next.code, ResultCode::NoSelectedKey);
    assert_eq!(actions(&next), []);
    finished(&w, FIRST)
}

/// A kill leaves the row in flight, and the remote goes without the generation
/// moving. The resend's plan fails, and `state::configure` fences the row to
/// `interrupted`: no row ended by a transport stop, but a row after the call, so
/// `Final`. Interrupted holds no reservation, and a new request polls.
fn once_killed_then_the_remote_removed() -> Result<(), FixtureError> {
    let (w, _) = behind()?;
    let dying = dying_at(&w, FailurePoint::PollBeforeTarget)?;
    let stopped = call(
        &w,
        &dying,
        FIRST,
        once(&w.root),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    let operation = stopped.operation_id.clone().ok_or(FixtureError)?;
    drop(dying);
    assert!(reserved(&w));
    let config = w.root.join(".manyhands/config.toml");
    let source = fixed(std::fs::read_to_string(&config))?;
    let without = source.replace("publication_remote = \"origin\"\n", "");
    assert_ne!(without, source);
    fixed(std::fs::write(&config, without))?;
    let ended = execute_once(&w, FIRST);
    assert_eq!(ended.code, ResultCode::PublicationRemoteRequired);
    assert_eq!(actions(&ended), []);
    assert_eq!(
        row_of(&w, &operation)?,
        (
            "interrupted".to_owned(),
            Some("configuration_required".to_owned())
        )
    );
    finished(&w, FIRST)?;
    assert!(!reserved(&w));
    fixed(std::fs::write(&config, source))?;
    a_new_request_polls(&w, SECOND)
}

/// A close stopped with its child holding the reservation: `busy`, nothing done.
/// The child's row is left as it was when the endpoints haven't changed.
fn child_holding_the_reservation() -> Result<(World, String), FixtureError> {
    let w = remote(Verb::Close)?;
    fixed(w.sync(w.request(SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: fixed(ITEM.parse())?,
    })))?;
    let preview = w.service.prepare(&Verb::Close.mutation(&w.root, ITEM));
    let confirmation = preview
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?
        .confirmation_id
        .clone();
    fixed(fixed(w.bare()?.find_reference(TICKET_BRANCH))?.delete())?;
    let stopped = presenting(
        &w,
        SECOND,
        Some(&confirmation),
        Verb::Close.mutation(&w.root, ITEM),
    );
    assert_eq!(stopped.code, ResultCode::RemoteBranchDeleted);
    assert!(reserved(&w));
    let child = fixed(w.service.active_remote_operation(&w.root))?
        .ok_or(FixtureError)?
        .operation_id()
        .to_string();
    Ok((w, child))
}

fn once_busy_beside_a_lifecycle_child() -> Result<(), FixtureError> {
    let (w, child) = child_holding_the_reservation()?;
    let phase = row_of(&w, &child)?;
    let opened = connections(&w);
    let envelope = execute_once(&w, FIRST);
    assert_eq!(envelope.code, ResultCode::Busy);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestRetry]);
    assert!(record(&w, FIRST).is_none());
    assert_eq!(connections(&w), opened);
    assert_eq!(row_of(&w, &child)?, phase, "the child is left");
    Ok(())
}

/// Known limit (S2): a poll configures its endpoints before it asks for the
/// reservation. With the endpoints changed, that raises the generation and fences
/// whatever is running, a lifecycle's child among them, and the poll then polls.
fn once_fences_a_lifecycle_child() -> Result<(), FixtureError> {
    let (w, child) = child_holding_the_reservation()?;
    let before_close = fixed(w.repo()?.refname_to_id(MAIN))?;
    let url = w.server.url();
    fixed(
        w.repo()?
            .remote_set_pushurl("origin", Some(&format!("{url}-changed"))),
    )?;
    let envelope = execute_once(&w, FIRST);
    assert_eq!(row_of(&w, &child)?.0, "interrupted", "fenced");
    assert_eq!(envelope.code, ResultCode::Ok);
    // A child with nothing retained is replaced, so the close survives the fence
    // once its branch and its URL are back. `c5_cancel_fenced_` has the ones
    // that don't.
    fixed(w.repo()?.remote_set_pushurl("origin", None))?;
    let tip = fixed(w.repo()?.refname_to_id(TICKET_BRANCH))?;
    fixed(
        w.bare()?
            .reference(TICKET_BRANCH, tip, true, "owned fixture"),
    )?;
    let resent = w.service.execute(
        MutationCall {
            request_id: request(SECOND),
            confirmation: None,
            mutation: Verb::Close.mutation(&w.root, ITEM),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(resent.code, ResultCode::Ok, "{:?}", resent.data);
    let primary = w.repo()?;
    let merge = fixed(primary.find_commit(fixed(primary.refname_to_id(MAIN))?))?;
    assert_eq!(merge.parent_count(), 2, "one merge");
    assert_eq!(
        fixed(merge.parent_id(0))?,
        before_close,
        "and only that one"
    );
    Ok(())
}

/// Another clone of the world's server, registered with the world's service,
/// as a person's second checkout would be.
fn another_clone(w: &World) -> Result<std::path::PathBuf, FixtureError> {
    let root = w.directory.path().join("clone-c");
    let repo = clone_owned(&w.server, &root)?;
    fixed(fixed(repo.config())?.set_str("user.name", "Another"))?;
    fixed(fixed(repo.config())?.set_str("user.email", "another@example.invalid"))?;
    fixed(w.service.enable(EnableRepositoryRequest {
        root: root.clone(),
        primary_branch: "main".into(),
        identity: None,
        operation_id: OperationId::new(),
    }))?;
    fixed(root.canonicalize())
}

fn context_disposition(envelope: &Envelope<MutationDataDto>) -> Option<PollTargetDisposition> {
    report(envelope)
        .targets
        .iter()
        .find(|target| target.target.as_deref() == Some(TICKET_BRANCH))
        .map(|target| target.disposition)
}

/// Decision 28: the latest copy is authoritative. A close whose remote deletion
/// was refused leaves the branch it published, which primary now contains. A
/// clone that never had the context gets no branch or worktree of it, and its
/// primary keeps the closure.
fn once_a_closed_ticket_s_older_branch_is_current() -> Result<(), FixtureError> {
    let w = remote(Verb::Close)?;
    fixed(w.sync(w.request(context_target(Verb::Close)?)))?;
    let prepared = w.service.prepare(&Verb::Close.mutation(&w.root, ITEM));
    let confirmation = prepared
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?
        .confirmation_id
        .clone();
    w.server.reject_context_deletion(true)?;
    // The fixture's native refusal applies to the receiver's current branch.
    fixed(fixed(w.bare())?.set_head(TICKET_BRANCH))?;
    let closed = w.service.execute(
        MutationCall {
            request_id: request(FIRST),
            confirmation: Some(fixed(ConfirmationId::parse(&confirmation))?),
            mutation: Verb::Close.mutation(&w.root, ITEM),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(closed.code, ResultCode::PushRejected, "{:?}", closed.data);
    assert_eq!(closed.effects.cleanup, results::CleanupEffect::Pending);
    let bare = w.bare()?;
    let main = fixed(bare.refname_to_id(MAIN))?;
    let stale = fixed(bare.refname_to_id(TICKET_BRANCH))?;
    assert!(
        stale == main || fixed(bare.graph_descendant_of(main, stale))?,
        "primary holds the branch's tip"
    );
    fixed(bare.set_head(MAIN))?;
    let other = another_clone(&w)?;
    let pushes = accepted(&w.server);

    let envelope = execute(&w, SECOND, once(&other));
    assert_eq!(envelope.code, ResultCode::Ok, "{:?}", envelope.data);
    assert_eq!(
        context_disposition(&envelope),
        Some(PollTargetDisposition::Current)
    );
    let repo = fixed(git2::Repository::open(&other))?;
    assert!(repo.refname_to_id(TICKET_BRANCH).is_err(), "no branch");
    assert!(!other.join(".manyhands/worktrees").join(ITEM).exists());
    assert_eq!(fixed(repo.refname_to_id(MAIN))?, main);
    let source = fixed(std::fs::read_to_string(
        other.join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
    ))?;
    assert!(
        source.contains("\nclosed_at: "),
        "primary keeps the closure"
    );
    assert_eq!(accepted(&w.server), pushes, "a poll never pushes");
    assert_eq!(fixed(w.bare()?.refname_to_id(TICKET_BRANCH))?, stale);

    // And again: still current, still nothing.
    let again = execute(&w, THIRD, once(&other));
    assert_eq!(
        context_disposition(&again),
        Some(PollTargetDisposition::Current)
    );
    assert!(repo.refname_to_id(TICKET_BRANCH).is_err());
    Ok(())
}

/// The control: a published branch with a commit primary doesn't hold
/// materializes in another clone, as before.
fn once_a_branch_primary_lacks_still_materializes() -> Result<(), FixtureError> {
    let w = remote(Verb::Close)?;
    fixed(w.sync(w.request(context_target(Verb::Close)?)))?;
    let bare = w.bare()?;
    let tip = fixed(bare.refname_to_id(TICKET_BRANCH))?;
    let main = fixed(bare.refname_to_id(MAIN))?;
    assert!(!fixed(bare.graph_descendant_of(main, tip))?);
    let other = another_clone(&w)?;
    let envelope = execute(&w, FIRST, once(&other));
    assert_eq!(envelope.code, ResultCode::Ok, "{:?}", envelope.data);
    assert_eq!(
        context_disposition(&envelope),
        Some(PollTargetDisposition::Materialized)
    );
    let repo = fixed(git2::Repository::open(&other))?;
    assert_eq!(fixed(repo.refname_to_id(TICKET_BRANCH))?, tip);
    assert!(other.join(".manyhands/worktrees").join(ITEM).is_dir());
    Ok(())
}
