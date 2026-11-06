//! `Resends`: what waits on a republish, how long, and what a resume's
//! answer sends (review I-2, M4; orchestrator rulings (a), (b); re-review 1,
//! 2, 3, 5). Each branch of `ended` runs against a request model the test
//! fills and a worker over a fixture's repository.

use std::path::{Path, PathBuf};

use manyhands::{
    repository::{
        MutationDataDto, OperationAction, OperationDto, OperationFamily, OperationId,
        OperationListDto, OperationNextAction, OperationOwner, RequestId,
    },
    results::{Effects, Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use super::{AfterRepublish, Resends};
use crate::desktop::{
    model::operations::Operations,
    services::{
        ReadResult,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{WorkerEvent, WorkerHandle},
    },
    test_support::fixtures::{Fixture, item_id},
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KS0";

/// A worker over a fixture's own backend; its events are not read.
fn worker(fixture: &Fixture) -> WorkerHandle {
    WorkerHandle::spawn(fixture.delaying_backend()).0
}

/// The request `n` of `intent`, under `request_id`, finished in the model.
fn finished(
    operations: &mut Operations,
    n: u64,
    intent: Intent,
    request_id: RequestId,
) -> DesktopRequestId {
    let id = DesktopRequestId(5_000_000 + n);
    operations.apply(WorkerEvent::Queued(DesktopRequest {
        id,
        request_id,
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    }));
    id
}

fn answer(
    command: &str,
    code: ResultCode,
    recovery: Vec<RecoveryAction>,
) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        command,
        Scope::default(),
        false,
        code,
        Effects::not_requested(),
        false,
    );
    if code == ResultCode::Ok {
        envelope.outcome = Outcome::Success;
    }
    envelope.recovery = recovery;
    envelope
}

fn republish(root: &Path) -> Intent {
    Intent::ItemRepublish {
        root: root.to_owned(),
        kind: manyhands::repository::AuthoringKind::Ticket,
        id: item_id(TICKET),
    }
}

fn resume_after(shown: DesktopRequestId) -> AfterRepublish {
    AfterRepublish::Resume {
        shown,
        root: PathBuf::from("/repo"),
        operation_id: OperationId::new(),
    }
}

/// Ruling (a), M4: a republish waits for its run until it, the republish,
/// is dismissed; dismissing the run keeps it (ruling (b)).
#[test]
fn a_republish_waits_until_its_own_entry_is_dismissed() {
    let resends = Resends::default();
    let (run, republish) = (DesktopRequestId(1), DesktopRequestId(2));
    assert!(!resends.pending_for(run));
    resends.after_republish(
        republish,
        AfterRepublish::Retry(run),
        PathBuf::from("/repo"),
        item_id(TICKET),
    );
    assert!(resends.pending_for(run));
    assert!(resends.waiting_for(Path::new("/repo"), &item_id(TICKET)));
    assert!(!resends.waiting_for(Path::new("/other"), &item_id(TICKET)));
    resends.forget(run);
    assert!(resends.pending_for(run), "the run's dismissal keeps it");
    resends.forget(republish);
    assert!(!resends.pending_for(run));
    assert!(!resends.waiting_for(Path::new("/repo"), &item_id(TICKET)));
}

/// Ruling (a), re-review 1, 2: an answer offering `request.retry` or
/// `request.prepare` keeps the republish waiting; any other failure, and
/// one removed or not sent, ends it; success sends the lifecycle on, its
/// resume then following the run.
#[test]
fn a_republishs_answer_keeps_or_ends_what_waits_on_it() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let worker = worker(&fixture);
    let mut operations = Operations::default();
    let resends = Resends::default();
    let run = DesktopRequestId(9);
    let cases = [
        (RecoveryActionKind::RequestRetry, true),
        (RecoveryActionKind::RequestPrepare, true),
        (RecoveryActionKind::RepoInspect, false),
    ];
    for (n, (kind, kept)) in cases.into_iter().enumerate() {
        let id = finished(
            &mut operations,
            n as u64,
            republish(&root),
            RequestId::new(),
        );
        resends.after_republish(id, resume_after(run), root.clone(), item_id(TICKET));
        let refused = answer(
            "item republish",
            ResultCode::ExternalChange,
            vec![RecoveryAction::new(kind, [])],
        );
        resends.ended(id, &refused, &operations, &worker);
        assert_eq!(resends.pending_for(run), kept, "{kind:?}");
        resends.forget(id);
    }

    let removed = finished(&mut operations, 10, republish(&root), RequestId::new());
    resends.after_republish(removed, resume_after(run), root.clone(), item_id(TICKET));
    resends.not_run(removed);
    assert!(!resends.pending_for(run), "removed or not sent");

    let done = finished(&mut operations, 11, republish(&root), RequestId::new());
    resends.after_republish(done, resume_after(run), root.clone(), item_id(TICKET));
    resends.ended(
        done,
        &answer("item republish", ResultCode::Ok, Vec::new()),
        &operations,
        &worker,
    );
    assert!(!resends.pending_for(run), "done");
    assert!(
        resends.follow(run).is_some(),
        "the resume sent follows the run"
    );
    assert_eq!(resends.follow(run), resends.follow(run), "read, not taken");
    // Last review, minor 1: the run queued again leads on afresh.
    resends.requeued(run);
    assert_eq!(resends.follow(run), None, "queued again");

    // Dismissing the run drops its link (review M4).
    let again = finished(&mut operations, 12, republish(&root), RequestId::new());
    resends.after_republish(again, resume_after(run), root.clone(), item_id(TICKET));
    resends.ended(
        again,
        &answer("item republish", ResultCode::Ok, Vec::new()),
        &operations,
        &worker,
    );
    assert!(resends.follow(run).is_some());
    resends.forget(run);
    assert_eq!(resends.follow(run), None, "dismissed");

    // Task 16, fix 1: a republish done whose lifecycle cannot be sent
    // again (a run this worker never took) resumes the lifecycle's
    // operation, which the rows name, instead; that resume follows the run.
    rows(&mut operations, &root, OperationId::new());
    let refused = finished(&mut operations, 13, republish(&root), RequestId::new());
    resends.after_republish(
        refused,
        AfterRepublish::Retry(run),
        root.clone(),
        item_id(TICKET),
    );
    resends.ended(
        refused,
        &answer("item republish", ResultCode::Ok, Vec::new()),
        &operations,
        &worker,
    );
    assert!(!resends.pending_for(run), "done");
    assert!(
        resends.follow(run).is_some(),
        "a refused retry resumes the operation instead"
    );
}

/// The rows of `root`: one closure parent of `operation`, of the ticket.
fn rows(operations: &mut Operations, root: &Path, operation: OperationId) {
    let row = OperationDto {
        operation_id: Some(operation.to_string()),
        family: OperationFamily::Remote,
        owner: OperationOwner::Repository,
        action: OperationAction::Close,
        state: "closing".to_owned(),
        completed_step: None,
        next_action: Some(OperationNextAction::Resume),
        item_id: Some(TICKET.to_owned()),
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    };
    operations.locks_mut().rows_read(
        root.to_str().unwrap(),
        &ReadResult::Ok(OperationListDto {
            items: vec![row],
            complete: true,
        }),
    );
}

fn original_request(named: RequestId) -> Envelope<MutationDataDto> {
    answer(
        "operation resume",
        ResultCode::OriginalRequestRequired,
        vec![RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [("request_id", named.to_string().into())],
        )],
    )
}

/// Decision 10, review M1, the re-review's minor: a resume answered
/// `original_request_required` sends the named close again when the rows
/// name its item, as a new request when this session holds none; when this
/// session's request cannot be queued again, nothing is sent and nothing
/// follows; when the rows name no item, nothing either.
#[test]
fn a_resumes_answer_sends_the_named_request_or_nothing() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let worker = worker(&fixture);
    let mut operations = Operations::default();
    let resends = Resends::default();
    let operation = OperationId::new();
    let resume = |root: &Path| Intent::OperationResume {
        root: root.to_owned(),
        operation_id: operation,
    };

    let unknown = finished(&mut operations, 1, resume(&root), RequestId::new());
    resends.ended(
        unknown,
        &original_request(RequestId::new()),
        &operations,
        &worker,
    );
    assert_eq!(resends.follow(unknown), None, "no row names the item");

    rows(&mut operations, &root, operation);
    let named = RequestId::new();
    let sent = finished(&mut operations, 2, resume(&root), RequestId::new());
    resends.ended(sent, &original_request(named), &operations, &worker);
    assert!(resends.follow(sent).is_some(), "a new request under it");

    // This session holds the named close, but its worker never took it: it
    // cannot be queued again.
    let held_id = RequestId::new();
    let close = Intent::TicketClose {
        root: root.clone(),
        id: item_id(TICKET),
    };
    let held = finished(&mut operations, 3, close, held_id);
    operations.apply(WorkerEvent::Removed(held));
    let refused = finished(&mut operations, 4, resume(&root), RequestId::new());
    resends.ended(refused, &original_request(held_id), &operations, &worker);
    assert_eq!(
        resends.follow(refused),
        None,
        "the resume's answer stays shown"
    );
}

/// Task 16 re-review, I-1: sending a lifecycle on never sends it twice. A
/// run the worker holds queued or running, though the request model has
/// not heard of it yet, counts as sent, and no resume goes with it; once
/// quitting has closed the queue, nothing is sent at all, not even a
/// resume.
#[test]
fn a_lifecycle_already_out_or_a_closed_queue_sends_no_resume() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let backend = fixture.delaying_backend();
    let held = backend.hold_job(crate::desktop::services::JobKind::TicketClose);
    let (worker, _events) = WorkerHandle::spawn(backend);
    let operations = Operations::default();
    let resume = || Some((root.clone(), OperationId::new()));
    let is_resume = |intent: &Intent| matches!(intent, Intent::OperationResume { .. });

    let close = worker.submit(
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        Lane::User,
        None,
        None,
    );
    let mark = worker.newest();
    assert_eq!(
        super::send_lifecycle_on(Some(close), resume(), &operations, &worker),
        Some(super::Followed::Resent),
        "already out"
    );
    assert!(!worker.sent_since(mark, is_resume), "no resume with it");

    worker.quit();
    held.release();
    let mark = worker.newest();
    let unknown = DesktopRequestId(77);
    assert_eq!(
        super::send_lifecycle_on(Some(unknown), resume(), &operations, &worker),
        None
    );
    assert_eq!(
        super::send_lifecycle_on(None, resume(), &operations, &worker),
        None
    );
    assert!(!worker.sent_since(mark, is_resume), "nothing once closed");
}
