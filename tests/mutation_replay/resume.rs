//! `operation resume`, for the operations that need no server: the index
//! hand-off a local operation still owes is run, and anything that needs
//! the input of the request that began it is left exactly as it was, for
//! that request. A synchronization's resume is in the test target
//! `mutation_remote`.

use manyhands::{
    repository::{
        AbandonOperationOutcome, AbandonOperationRequest, CancellationToken, FailurePoint,
        Mutation, MutationCall, MutationDataDto, OperationId, OperationResumeInput, ProgressEvent,
        ProgressSink, ProgressStage, RemoteAddInput, RequestId, RequestState,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        request_store::{FinalKind, JournalRow},
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, FailureClass, Outcome, ResultCode,
        WriteEffect,
    },
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{self, KeyWorld, REQUEST_1, REQUEST_2, REQUEST_3, TITLE, World, recovery_actions},
    operations::{self, OPERATION_A, OPERATION_B, OPERATION_C},
};

/// An operation ID no journal holds.
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZZ";

fn resume_in(root: &std::path::Path, operation_id: &str) -> Mutation {
    Mutation::OperationResume(OperationResumeInput {
        root: root.to_owned(),
        operation_id: OperationId::parse(operation_id).unwrap(),
    })
}

fn resume(world: &World, operation_id: &str) -> Mutation {
    resume_in(&world.root, operation_id)
}

/// A local row as the journal stores it: its state, its step, when it
/// was last touched and how often its hand-off was claimed.
type Stored = (String, Option<String>, i64, i64);

fn stored(world: &World, operation_id: &str) -> Stored {
    items::index(world.data.path())
        .query_row(
            "SELECT state, completed_step, observed_at, index_owner_epoch
               FROM operation_records WHERE operation_ulid = ?1",
            [operation_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap()
}

/// Sets the state and the time of a local row, as a call that stopped
/// there would have left them.
fn set_state(world: &World, operation_id: &str, state: &str, observed_at: i64) {
    let changed = items::index(world.data.path())
        .execute(
            "UPDATE operation_records SET state = ?2, observed_at = ?3 WHERE operation_ulid = ?1",
            rusqlite::params![operation_id, state, observed_at],
        )
        .unwrap();
    assert_eq!(changed, 1);
}

/// The index brought up to date and nothing else: what a resume that ran
/// a hand-off reports.
fn current() -> Effects {
    Effects {
        discovery: DiscoveryEffect::Current,
        ..Effects::not_requested()
    }
}

/// A world in which `REQUEST_1` saved `TICKET_A`, committed, and failed
/// its index hand-off: the save's envelope, with its operation pending
/// and its record accepted.
fn save_owing_its_hand_off() -> (World, String, Envelope<MutationDataDto>) {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let token = world.token(TICKET_A);
    let owed = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(owed.code, ResultCode::DiscoveryPending);
    assert_eq!(world.pending_journal_rows(), 1);
    (world, token, owed)
}

fn operation_of(envelope: &Envelope<MutationDataDto>) -> String {
    envelope.operation_id.clone().expect("an operation ID")
}

/// Asserts that a resume was refused with nothing recorded of it.
fn assert_refused(
    world: &World,
    request_id: &str,
    envelope: &Envelope<MutationDataDto>,
    outcome: Outcome,
    code: ResultCode,
) {
    assert_eq!(envelope.command, "operation resume");
    assert_eq!((envelope.outcome, envelope.code), (outcome, code));
    assert_eq!(envelope.effects, Effects::not_requested());
    assert_eq!(envelope.operation_id, None);
    assert_eq!(envelope.data, None);
    assert!(world.record(request_id).is_none(), "a request record");
}

#[test]
fn a_resume_runs_the_hand_off_a_save_owes_and_the_save_s_request_is_then_finished() {
    let (world, token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    let commit = world.branch_tip(TICKET_A).unwrap();
    // The action the save's result offered is this request.
    assert_eq!(owed.recovery[0].action.as_str(), "operation.resume");
    assert_eq!(owed.recovery[0].operation_id, owed.operation_id);

    let resumed = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(resumed.command, "operation resume");
    assert_eq!(
        (resumed.outcome, resumed.code, resumed.failure_class()),
        (Outcome::Success, ResultCode::Ok, None)
    );
    // The hand-off is what it did: the write and the commit are the
    // save's, and are not claimed.
    assert_eq!(resumed.effects, current());
    assert_eq!(resumed.request_id.as_deref(), Some(REQUEST_2));
    // The operation is another request's, and is this one's target.
    assert_eq!(resumed.operation_id, None);
    assert_eq!(resumed.data, None);
    assert!(resumed.recovery.is_empty());
    assert_eq!(resumed.scope.repository.as_deref(), world.root.to_str());
    assert_eq!(resumed.scope.item_id, None);

    let record = world.record(REQUEST_2).expect("the resume's record");
    assert_eq!(record.state, RequestState::Finished);
    assert_eq!(record.target, operation_id);
    assert!(record.operations.is_empty());
    assert_eq!(
        world.journal_row(&operation_id),
        JournalRow::Final {
            kind: FinalKind::Completed,
            owes_work: false,
            checkpointed: true,
        }
    );
    assert_eq!(world.pending_journal_rows(), 0);

    // The save's own request, sent again, is finished from what its
    // operation did: nothing is written or committed a second time.
    let source = world.worktree_source(TICKET_A);
    let finished = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (finished.outcome, finished.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(finished.effects.write, WriteEffect::Written);
    assert_eq!(finished.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(finished.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(finished.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(finished.operation_id.as_deref(), Some(&*operation_id));
    assert_eq!(world.branch_tip(TICKET_A), Some(commit));
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(world.worktree_source(TICKET_A), source);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_resume_of_a_save_interrupted_before_its_write_needs_the_original_request() {
    let world =
        World::with_service(|data| FailOnce::at(FailurePoint::BeforeItemWrite).open_service(data));
    let token = world.token(TICKET_A);
    let interrupted = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(interrupted.code, ResultCode::InternalError);
    let operation_id = operation_of(&interrupted);
    assert!(!world.worktree_source(TICKET_A).contains(TITLE));
    let before = stored(&world, &operation_id);
    let record = world.record(REQUEST_1);

    let refused = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &refused,
        Outcome::Blocked,
        ResultCode::OriginalRequestRequired,
    );
    assert_eq!(refused.failure_class(), Some(FailureClass::Blocked));
    // The request to send is the one that began the operation.
    assert_eq!(recovery_actions(&refused), ["request.retry"]);
    assert_eq!(refused.recovery[0].arguments["request_id"], REQUEST_1);
    assert_eq!(refused.recovery[0].operation_id, None);

    // The row is exactly as it was: no step was recorded on it, it was
    // not claimed and it was not touched. The write is not marked done.
    assert_eq!(stored(&world, &operation_id), before);
    assert_eq!(world.record(REQUEST_1), record);
    assert!(!world.worktree_source(TICKET_A).contains(TITLE));
    assert_eq!(world.branch_commits(TICKET_A), 0);

    // And so the original request still does the write, once.
    let finished = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(
        (finished.outcome, finished.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(finished.effects.write, WriteEffect::Written);
    assert!(world.worktree_source(TICKET_A).contains(TITLE));
    assert_eq!(world.branch_commits(TICKET_A), 1);
}

#[test]
fn a_resume_of_a_save_that_wrote_and_did_not_commit_needs_the_original_request() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence).open_service(data)
    });
    let token = world.token(TICKET_A);
    let partial = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(partial.outcome, Outcome::Partial);
    let operation_id = operation_of(&partial);
    let before = stored(&world, &operation_id);

    let refused = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &refused,
        Outcome::Blocked,
        ResultCode::OriginalRequestRequired,
    );
    assert_eq!(refused.recovery[0].arguments["request_id"], REQUEST_1);
    assert_eq!(stored(&world, &operation_id), before);
    assert_eq!(world.branch_commits(TICKET_A), 0);
}

#[test]
fn a_resume_of_a_row_whose_work_is_recorded_and_whose_hand_off_has_not_begun_is_left_alone() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    // As a call that died between recording its checkpoint and handing
    // off would leave the row.
    let (_, step, observed_at, _) = stored(&world, &operation_id);
    let step = step.expect("the checkpoint step");
    assert_eq!(step, "authoring_checkpoint_observed");
    set_state(&world, &operation_id, &step, observed_at);
    let before = stored(&world, &operation_id);

    let refused = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &refused,
        Outcome::Blocked,
        ResultCode::OriginalRequestRequired,
    );
    assert_eq!(refused.recovery[0].arguments["request_id"], REQUEST_1);
    assert_eq!(stored(&world, &operation_id), before);
}

#[test]
fn a_resume_of_an_interrupted_key_generation_needs_the_original_request() {
    let world = KeyWorld::with_service(|data| {
        FailOnce::at(FailurePoint::GenerationAfterPublicWrite).open_service(data)
    });
    let interrupted = world.execute(
        REQUEST_1,
        world.generate("Work laptop", Some(mutation::PASSPHRASE)),
    );
    assert_eq!(interrupted.outcome, Outcome::Partial);
    let operation_id = operation_of(&interrupted);
    let phase = || -> String {
        items::index(world.data.path())
            .query_row(
                "SELECT phase FROM key_material_operations WHERE operation_id = ?1",
                [&operation_id],
                |row| row.get(0),
            )
            .unwrap()
    };
    let before = phase();

    // A key operation is the application's: it is found whatever
    // directory the resume names.
    let refused = world.execute(REQUEST_2, resume_in(world.home.path(), &operation_id));
    assert_eq!(
        (refused.outcome, refused.code),
        (Outcome::Blocked, ResultCode::OriginalRequestRequired)
    );
    assert_eq!(refused.effects, Effects::not_requested());
    assert_eq!(recovery_actions(&refused), ["request.retry"]);
    assert_eq!(refused.recovery[0].arguments["request_id"], REQUEST_1);
    assert!(world.record(REQUEST_2).is_none());
    assert_eq!(phase(), before);
    assert!(world.keys().is_empty());

    // The original request registers the key: one key.
    let finished = world.execute(
        REQUEST_1,
        world.generate("Work laptop", Some(mutation::PASSPHRASE)),
    );
    assert_eq!(finished.outcome, Outcome::Success);
    assert_eq!(world.keys().len(), 1);
    assert_eq!(world.private_key_files().len(), 1);

    // Nothing is owed by a generation that has ended.
    let ended = world.execute(REQUEST_2, resume_in(world.home.path(), &operation_id));
    assert_eq!(
        (ended.outcome, ended.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert!(world.record(REQUEST_2).is_none());
}

#[test]
fn a_resume_of_an_operation_no_journal_holds_is_operation_not_found() {
    let world = World::new();
    let unknown = world.execute(REQUEST_1, resume(&world, UNKNOWN));
    assert_refused(
        &world,
        REQUEST_1,
        &unknown,
        Outcome::Error,
        ResultCode::OperationNotFound,
    );
    assert_eq!(unknown.failure_class(), Some(FailureClass::Input));
    assert!(unknown.recovery.is_empty());
    assert_eq!(world.request_rows(), 0);
    assert_eq!(world.pending_journal_rows(), 0);
}

#[test]
fn a_resume_is_replayed_under_its_request_id_and_refused_for_another_operation() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    let resumed = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(resumed.outcome, Outcome::Success);
    let record = world.record(REQUEST_2);
    let row = stored(&world, &operation_id);

    // The same request again: the stored result, and nothing runs.
    let again = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(again, resumed);
    assert_eq!(world.record(REQUEST_2), record);
    assert_eq!(stored(&world, &operation_id), row);

    // The same request ID for another operation is another request.
    let mismatch = world.execute(REQUEST_2, resume(&world, UNKNOWN));
    assert_eq!(
        (mismatch.outcome, mismatch.code, mismatch.failure_class()),
        (
            Outcome::Error,
            ResultCode::RequestMismatch,
            Some(FailureClass::Input)
        )
    );
    assert_eq!(mismatch.effects, Effects::not_requested());
    assert_eq!(world.record(REQUEST_2), record);
    assert_eq!(world.request_rows(), 2);
}

#[test]
fn a_resume_of_an_abandoned_operation_is_refused_and_the_row_stays_abandoned() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    // Its hand-off was owed, which a resume would have run.
    let abandoned = world
        .service
        .abandon_operation(AbandonOperationRequest {
            root: world.root.clone(),
            operation_id: OperationId::parse(&operation_id).unwrap(),
            expected: None,
        })
        .unwrap();
    assert!(matches!(abandoned, AbandonOperationOutcome::Abandoned(_)));
    let before = stored(&world, &operation_id);
    assert_eq!(
        (before.0.as_str(), before.1.as_deref()),
        ("completed", Some("abandoned"))
    );

    // It is not resumed, and is not taken to be done.
    let refused = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &refused,
        Outcome::Blocked,
        ResultCode::ExternalChange,
    );
    assert!(refused.recovery.is_empty());
    assert_eq!(stored(&world, &operation_id), before);
}

#[test]
fn a_resume_of_an_operation_that_has_ended_is_a_no_op() {
    let world = World::new();
    let token = world.token(TICKET_A);
    let saved = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(saved.outcome, Outcome::Success);
    let operation_id = operation_of(&saved);
    let before = stored(&world, &operation_id);

    let noop = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &noop,
        Outcome::Noop,
        ResultCode::AlreadyApplied,
    );
    assert_eq!(noop.failure_class(), None);
    assert!(noop.recovery.is_empty());
    assert_eq!(stored(&world, &operation_id), before);
    assert_eq!(world.branch_commits(TICKET_A), 1);
}

#[cfg(unix)]
#[test]
fn a_resume_of_an_operation_that_was_rejected_is_a_no_op() {
    use manyhands::repository::OwnedPathBoundary;

    let _hook_guard = mutation::owned_path_hook_test_lock();
    let world = World::new();
    let token = world.token(TICKET_A);
    let worktree = world.worktree(TICKET_A);
    let file = worktree.join(items::ticket_path(TICKET_A));
    let other = items::ticket_source(TICKET_A, "Changed elsewhere", "");
    world.service.set_owned_path_hook_for_root_for_testing(
        worktree,
        items::ticket_path(TICKET_A).into(),
        OwnedPathBoundary::Read,
        move || std::fs::write(&file, &other).unwrap(),
    );
    // The domain refused the save and closed its row as rejected.
    let rejected = world.execute(REQUEST_1, world.save(TICKET_A, &token));
    assert_eq!(rejected.code, ResultCode::ExternalChange);
    let operation_id = operation_of(&rejected);
    let before = stored(&world, &operation_id);
    assert_eq!(
        (before.0.as_str(), before.1.as_deref()),
        ("completed", Some("rejected"))
    );

    let noop = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &noop,
        Outcome::Noop,
        ResultCode::AlreadyApplied,
    );
    assert_eq!(stored(&world, &operation_id), before);
}

#[test]
fn a_resume_runs_the_hand_off_a_remote_that_was_added_owes() {
    let world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let add = || {
        Mutation::RemoteAdd(RemoteAddInput {
            root: world.root.clone(),
            name: "origin".to_owned(),
            url: "ssh://git@example.invalid/team/project.git".to_owned(),
        })
    };
    let owed = world.execute(REQUEST_1, add());
    assert_eq!(owed.code, ResultCode::DiscoveryPending);
    let operation_id = operation_of(&owed);
    // A remote commits nothing: its step is not a checkpoint's.
    assert_eq!(
        stored(&world, &operation_id).1.as_deref(),
        Some("remote_changed")
    );

    let resumed = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(
        (resumed.outcome, resumed.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(resumed.effects, current());
    assert_eq!(world.pending_journal_rows(), 0);

    let finished = world.execute(REQUEST_1, add());
    assert_eq!(
        (finished.outcome, finished.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn a_resume_of_a_row_another_call_is_indexing_under_is_busy_and_leaves_it() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    // An index owner claimed the hand-off a moment ago and is at work.
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    set_state(&world, &operation_id, "indexing", now);
    let before = stored(&world, &operation_id);

    let busy = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(&world, REQUEST_2, &busy, Outcome::Error, ResultCode::Busy);
    assert_eq!(busy.failure_class(), Some(FailureClass::Transient));
    // The same resume is what to send again.
    assert_eq!(recovery_actions(&busy), ["request.retry"]);
    assert_eq!(busy.recovery[0].arguments["request_id"], REQUEST_2);
    assert_eq!(stored(&world, &operation_id), before);
}

#[test]
fn a_resume_does_nothing_on_a_journal_row_it_cannot_read() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    let (_, _, observed_at, _) = stored(&world, &operation_id);
    // No state of a local operation has this form.
    set_state(&world, &operation_id, "Not A State", observed_at);
    let before = stored(&world, &operation_id);

    let unreadable = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &unreadable,
        Outcome::Error,
        ResultCode::InternalError,
    );
    assert!(unreadable.recovery.is_empty());
    assert_eq!(stored(&world, &operation_id), before);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    // Put back, so that the world's own scans read the journal.
    set_state(&world, &operation_id, "failed", observed_at);
}

#[test]
fn a_resume_that_died_before_its_hand_off_is_taken_up_by_the_same_request() {
    let (mut world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(world.data.path());
    let before = stored(&world, &operation_id);

    let died = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(died.code, ResultCode::InternalError);
    assert_eq!(stored(&world, &operation_id), before);
    let record = world.record(REQUEST_2).expect("the record stays");
    assert_eq!((record.state, record.attempt), (RequestState::Accepted, 1));

    world.reopen();
    let resumed = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(
        (resumed.outcome, resumed.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(resumed.effects, current());
    assert_eq!(world.pending_journal_rows(), 0);
    let record = world.record(REQUEST_2).unwrap();
    assert_eq!((record.state, record.attempt), (RequestState::Finished, 2));
}

#[test]
fn a_resume_whose_answer_was_lost_finds_the_operation_ended_and_is_a_no_op() {
    let (mut world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestSettlement).open_service(world.data.path());

    let lost = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(lost.code, ResultCode::InternalError);
    // The hand-off ran, and nothing was recorded of the request's end.
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Accepted
    );
    let row = stored(&world, &operation_id);

    world.reopen();
    // The row does not say whose call completed it: nothing is claimed.
    let found = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(
        (found.outcome, found.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_eq!(found.effects, Effects::not_requested());
    assert_eq!(stored(&world, &operation_id), row);
    assert_eq!(
        world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(
        world.execute(REQUEST_2, resume(&world, &operation_id)),
        found
    );
}

#[test]
fn a_resume_entered_again_on_an_operation_it_cannot_resume_leaves_no_record() {
    let (mut world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    world.service =
        FailOnce::at(FailurePoint::BeforeRequestDomainCall).open_service(world.data.path());
    let died = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_eq!(died.code, ResultCode::InternalError);
    assert!(world.record(REQUEST_2).is_some());
    world.reopen();
    // The operation was abandoned meanwhile.
    world
        .service
        .abandon_operation(AbandonOperationRequest {
            root: world.root.clone(),
            operation_id: OperationId::parse(&operation_id).unwrap(),
            expected: None,
        })
        .unwrap();
    let before = stored(&world, &operation_id);

    let refused = world.execute(REQUEST_2, resume(&world, &operation_id));
    assert_refused(
        &world,
        REQUEST_2,
        &refused,
        Outcome::Blocked,
        ResultCode::ExternalChange,
    );
    assert_eq!(stored(&world, &operation_id), before);
    // Its ID is free: the same ID is a new request.
    let unknown = world.execute(REQUEST_2, resume(&world, UNKNOWN));
    assert_eq!(unknown.code, ResultCode::OperationNotFound);
}

struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _request: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

/// A sink that keeps what it is sent.
#[derive(Default)]
struct Events(Vec<ProgressEvent>);

impl ProgressSink for Events {
    fn event(&mut self, event: &ProgressEvent) {
        self.0.push(event.clone());
    }
}

#[test]
fn a_resume_reports_that_it_looks_at_the_operation_and_then_indexes() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    let mut events = Events::default();
    let resumed = world.service.execute(
        MutationCall {
            request_id: RequestId::parse(REQUEST_2).unwrap(),
            confirmation: None,
            mutation: resume(&world, &operation_id),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut events,
        &CancellationToken::new(),
    );
    assert_eq!(resumed.outcome, Outcome::Success);
    assert_eq!(
        events.0.iter().map(|event| event.stage).collect::<Vec<_>>(),
        [
            ProgressStage::Accepted,
            ProgressStage::Observing,
            ProgressStage::Indexing
        ]
    );
    for event in &events.0 {
        assert_eq!(event.request_id, REQUEST_2);
        assert_eq!(event.command, "operation resume");
        // The request has no operation of its own.
        assert_eq!(event.operation_id, None);
        assert_eq!(event.scope, resumed.scope);
        assert_eq!(event.effects, Effects::not_requested());
    }

    // Refused before it is accepted: nothing is reported.
    let mut events = Events::default();
    let unknown = world.service.execute(
        MutationCall {
            request_id: RequestId::parse(REQUEST_3).unwrap(),
            confirmation: None,
            mutation: resume(&world, UNKNOWN),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut events,
        &CancellationToken::new(),
    );
    assert_eq!(unknown.code, ResultCode::OperationNotFound);
    assert!(events.0.is_empty());
}

#[test]
fn a_resume_cancelled_before_it_is_accepted_does_nothing() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    let before = stored(&world, &operation_id);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let cancelled = mutation::execute_cancellable(
        &world.service,
        REQUEST_2,
        resume(&world, &operation_id),
        &cancel,
    );
    assert_refused(
        &world,
        REQUEST_2,
        &cancelled,
        Outcome::Cancelled,
        ResultCode::Cancelled,
    );
    assert_eq!(stored(&world, &operation_id), before);
}

#[test]
fn a_resume_presenting_a_confirmation_is_not_confirmable() {
    let (world, _token, owed) = save_owing_its_hand_off();
    let operation_id = operation_of(&owed);
    let before = stored(&world, &operation_id);
    let refused = mutation::execute_confirmed(
        &world.service,
        REQUEST_2,
        "01ARZ3NDEKTSV4RRFFQ69G5FN1",
        resume(&world, &operation_id),
    );
    assert_refused(
        &world,
        REQUEST_2,
        &refused,
        Outcome::Error,
        ResultCode::NotConfirmable,
    );
    assert_eq!(stored(&world, &operation_id), before);
    // And `prepare` gives no preview of it.
    let prepared = world.service.prepare(&resume(&world, &operation_id));
    assert_eq!(prepared.code, ResultCode::NotConfirmable);
}

// The remote journal, with rows no call here can leave: what a resume
// makes of each without a server. A synchronization it takes up is in
// the test target `mutation_remote`.

/// A remote row as the journal stores what a call changes of it.
type StoredRemote = (String, i64, i64, i64, Option<String>);

fn stored_remote(world: &World, operation_id: &str) -> StoredRemote {
    items::index(world.data.path())
        .query_row(
            "SELECT phase, updated_at, owner_epoch, cancel_requested, sync_checkpoint
               FROM remote_operation_records WHERE operation_ulid = ?1",
            [operation_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap()
}

fn set_remote(world: &World, operation_id: &str, assignments: &str) {
    items::index(world.data.path())
        .execute_batch(&format!(
            "UPDATE remote_operation_records SET {assignments}
              WHERE operation_ulid = '{operation_id}'"
        ))
        .unwrap();
}

/// A synchronization of the primary branch in `phase`.
fn insert_synchronization(world: &World, operation_id: &str, phase: &str) {
    operations::insert_remote_synchronization(
        world.data.path(),
        operation_id,
        None,
        phase,
        Some("before_fetch"),
        None,
    );
}

fn remote_world() -> World {
    let world = World::new();
    operations::configure_remote(world.data.path());
    world
}

#[test]
fn a_resume_of_a_remote_operation_that_has_ended_is_a_no_op() {
    let world = remote_world();
    insert_synchronization(&world, OPERATION_A, "completed");
    insert_synchronization(&world, OPERATION_B, "cancelled");
    operations::insert_remote_poll(
        world.data.path(),
        OPERATION_C,
        "completed",
        Some("after_batch_commit"),
        Some("completed"),
    );
    for operation_id in [OPERATION_A, OPERATION_B, OPERATION_C] {
        let before = stored_remote(&world, operation_id);
        let noop = world.execute(REQUEST_1, resume(&world, operation_id));
        assert_refused(
            &world,
            REQUEST_1,
            &noop,
            Outcome::Noop,
            ResultCode::AlreadyApplied,
        );
        assert!(noop.recovery.is_empty(), "{operation_id}");
        assert_eq!(
            stored_remote(&world, operation_id),
            before,
            "{operation_id}"
        );
    }
}

#[test]
fn a_resume_of_a_cancelled_synchronization_that_left_work_says_to_look_at_the_repository() {
    let world = remote_world();
    insert_synchronization(&world, OPERATION_A, "cancelled");
    set_remote(&world, OPERATION_A, "reconciliation_required = 1");
    let before = stored_remote(&world, OPERATION_A);

    let left = world.execute(REQUEST_1, resume(&world, OPERATION_A));
    assert_refused(
        &world,
        REQUEST_1,
        &left,
        Outcome::Blocked,
        ResultCode::RecoveryRequired,
    );
    assert_eq!(recovery_actions(&left), ["repo.inspect"]);
    assert_eq!(stored_remote(&world, OPERATION_A), before);
}

/// The record of the synchronization that holds `operation_id`,
/// accepted and not finished.
fn accept_synchronization(world: &World, request_id: &str, operation_id: &str) {
    use manyhands::repository::{
        OperationFamily,
        request_store::{DigestSalt, IntentDigest, NewRequest, RequestOperation, ScopeKey},
    };
    let scope = ScopeKey::for_repository(&world.root).unwrap();
    let request_id = RequestId::parse(request_id).unwrap();
    world
        .service
        .insert_request(&NewRequest {
            request_id,
            intent_digest: IntentDigest::builder(
                DigestSalt::Request(request_id),
                "repo sync",
                &scope,
                "",
            )
            .finish(),
            scope,
            command: "repo sync".to_owned(),
            target: String::new(),
            confirmation: None,
            base_ref: Some("refs/heads/main".to_owned()),
            base_oid: Some(world.primary_head()),
            expected_digest: None,
            operations: vec![RequestOperation {
                family: OperationFamily::Remote,
                operation_id: OperationId::parse(operation_id).unwrap(),
            }],
        })
        .unwrap();
}

#[test]
fn a_resume_of_a_fenced_synchronization_is_not_taken_up_and_names_its_request() {
    let world = remote_world();
    insert_synchronization(&world, OPERATION_A, "interrupted");
    // The remote's endpoints or the selected key changed since it began.
    items::index(world.data.path())
        .execute_batch(
            "UPDATE remote_polling_state
                SET configuration_generation = configuration_generation + 1",
        )
        .unwrap();
    let before = stored_remote(&world, OPERATION_A);

    // No record holds it: there is no request to name.
    let unowned = world.execute(REQUEST_2, resume(&world, OPERATION_A));
    assert_refused(
        &world,
        REQUEST_2,
        &unowned,
        Outcome::Blocked,
        ResultCode::OriginalRequestRequired,
    );
    assert!(unowned.recovery.is_empty());
    assert_eq!(stored_remote(&world, OPERATION_A), before);

    // Its own request, sent again, is what is given its final answer.
    accept_synchronization(&world, REQUEST_1, OPERATION_A);
    let refused = world.execute(REQUEST_2, resume(&world, OPERATION_A));
    assert_refused(
        &world,
        REQUEST_2,
        &refused,
        Outcome::Blocked,
        ResultCode::OriginalRequestRequired,
    );
    assert_eq!(recovery_actions(&refused), ["request.retry"]);
    assert_eq!(refused.recovery[0].arguments["request_id"], REQUEST_1);
    assert_eq!(stored_remote(&world, OPERATION_A), before);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );
}

#[test]
fn a_resume_of_a_poll_that_has_not_ended_is_not_taken_up() {
    let world = remote_world();
    operations::insert_remote_poll(world.data.path(), OPERATION_A, "interrupted", None, None);
    let before = stored_remote(&world, OPERATION_A);

    let refused = world.execute(REQUEST_1, resume(&world, OPERATION_A));
    assert_refused(
        &world,
        REQUEST_1,
        &refused,
        Outcome::Blocked,
        ResultCode::OriginalRequestRequired,
    );
    // No request begins a poll.
    assert!(refused.recovery.is_empty());
    assert_eq!(stored_remote(&world, OPERATION_A), before);
}

#[test]
fn a_resume_does_nothing_on_a_remote_row_it_cannot_read() {
    let world = remote_world();
    // No item has this ID.
    operations::insert_remote_synchronization(
        world.data.path(),
        OPERATION_A,
        Some(("ticket", "not-an-item")),
        "interrupted",
        Some("before_fetch"),
        None,
    );
    let before = stored_remote(&world, OPERATION_A);

    let unreadable = world.execute(REQUEST_1, resume(&world, OPERATION_A));
    assert_refused(
        &world,
        REQUEST_1,
        &unreadable,
        Outcome::Error,
        ResultCode::InternalError,
    );
    assert!(unreadable.recovery.is_empty());
    assert_eq!(stored_remote(&world, OPERATION_A), before);
}

#[test]
fn a_resume_of_another_repository_s_operation_is_not_found() {
    let world = remote_world();
    insert_synchronization(&world, OPERATION_A, "interrupted");
    let before = stored_remote(&world, OPERATION_A);
    // A directory that is no repository of the journal's.
    let elsewhere = tempfile::tempdir().unwrap();

    let unknown = world.execute(REQUEST_1, resume_in(elsewhere.path(), OPERATION_A));
    assert_eq!(
        (unknown.outcome, unknown.code),
        (Outcome::Error, ResultCode::OperationNotFound)
    );
    assert!(world.record(REQUEST_1).is_none());
    assert_eq!(stored_remote(&world, OPERATION_A), before);
}
