//! `poll configure`, `poll pause` and `poll resume` (Task 5a, decisions 5, 11
//! and 15): a `c5_poll_` case per row of the design's policy table, and the
//! rows of last commit wins. They need no server: each world is one local
//! repository.

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
    ("c5_poll_replay_of_each_verb", replay_of_each_verb),
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

/// Leaves the stored due time, and every other field.
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
    let before = status(&local)?;
    for (mutation, name) in [
        (pause(&local.root), "pause"),
        (configure(&local.root, 120), "configure"),
    ] {
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
    let local = Local::empty()?;
    stage(&local, "paused=1")?;
    let dying = at_failure_point(&local, FailurePoint::BeforeRequestDomainCall)?;
    let died = called(&dying, FIRST, resume(&local.root));
    assert_eq!(died.code, ResultCode::InternalError);
    assert!(status(&local)?.paused, "R1 wrote nothing");
    let paused = called(&local.service, SECOND, pause(&local.root));
    assert_eq!(
        paused.code,
        ResultCode::AlreadyApplied,
        "R2 finds it paused"
    );
    let resent = called(&local.service, FIRST, resume(&local.root));
    assert_eq!(resent.code, ResultCode::Ok, "a new write");
    assert!(!policy(&resent).paused, "the answer says so");
    assert!(!status(&local)?.paused, "the pause is cleared");
    finished_record(&local, FIRST)?;
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
