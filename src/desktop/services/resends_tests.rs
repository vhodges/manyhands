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

/// Activity's `resend_waits` (R-47): `waits_on` is true exactly when
/// `forget` would drop something, a chain link from or to the request, or a
/// republish waiting under its ID, and not for the run a republish sends on.
#[test]
fn waits_on_says_when_forgetting_would_drop_something() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let worker = worker(&fixture);
    let mut operations = Operations::default();
    let resends = Resends::default();
    let operation = OperationId::new();
    let resume = Intent::OperationResume {
        root: root.clone(),
        operation_id: operation,
    };
    rows(&mut operations, &root, operation);
    let shown = finished(&mut operations, 1, resume, RequestId::new());
    assert!(!resends.waits_on(shown), "nothing is registered");
    resends.ended(
        shown,
        &original_request(RequestId::new()),
        &operations,
        &worker,
    );
    let follow = resends.follow(shown).expect("a request under it");
    assert!(resends.waits_on(shown), "a link from it");
    assert!(resends.waits_on(follow), "a link to it");
    assert!(!resends.waits_on(DesktopRequestId(99)), "another request");
    resends.forget(shown);
    assert!(!resends.waits_on(shown) && !resends.waits_on(follow));

    let (run, republish) = (DesktopRequestId(10), DesktopRequestId(11));
    resends.after_republish(
        republish,
        AfterRepublish::Retry(run),
        PathBuf::from("/repo"),
        item_id(TICKET),
    );
    assert!(
        resends.waits_on(republish),
        "a republish waits under its ID"
    );
    assert!(!resends.waits_on(run), "its run's dismissal keeps it");
    resends.forget(republish);
    assert!(!resends.waits_on(republish));
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

/// V16, M1: one operation is resumed once, whoever asks. A row Resume (not
/// gated while a conflict resolution of the item is out) and a drain
/// follow-up both submit `operation resume`; while the first is queued or
/// running the second is the first, and a close sent again under a library
/// request ID already out is that close. Once the first is answered, a new
/// resume is a new request.
#[test]
fn a_resume_or_close_already_out_is_not_queued_twice() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let backend = fixture.delaying_backend();
    let held = backend.hold_job(crate::desktop::services::JobKind::OperationResume);
    let (worker, _events) = WorkerHandle::spawn(backend);
    let resume = |operation: &OperationId| Intent::OperationResume {
        root: root.clone(),
        operation_id: *operation,
    };
    let (this, other) = (OperationId::new(), OperationId::new());

    let first = worker.submit(resume(&this), Lane::User, None, None);
    held.wait_reached();
    let mark = worker.newest();
    assert_eq!(
        worker.submit(resume(&this), Lane::User, None, None),
        first,
        "running"
    );
    let another = worker.submit(resume(&other), Lane::User, None, None);
    assert_ne!(another, first, "another operation is another resume");
    let queued = worker.newest();
    assert_eq!(
        worker.submit(resume(&other), Lane::User, None, None),
        another,
        "queued"
    );
    assert!(worker.sent_since(mark, |_| true), "the other one was sent");
    assert_eq!(worker.newest(), queued, "and nothing was sent twice");

    let close = |id| Intent::TicketClose {
        root: root.clone(),
        id,
    };
    let named = RequestId::new();
    let sent = worker.submit_with_request_id(close(item_id(TICKET)), Lane::User, named);
    assert_eq!(
        worker.submit_with_request_id(close(item_id(TICKET)), Lane::User, named),
        sent,
        "the same library request"
    );
    assert_ne!(
        worker.submit_with_request_id(close(item_id(TICKET)), Lane::User, RequestId::new()),
        sent,
        "another request"
    );
    held.release();
}

/// V16: a resume Stop was pressed on is ending, and a resume of the same
/// operation sent after the Stop is a new request that runs, not the
/// stopping one. Once a resume is answered it is no twin either.
#[test]
fn a_stopped_or_finished_resume_is_not_a_twin() {
    use crate::desktop::services::worker::{CancelResult, WorkerEvent};
    use futures::StreamExt;
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let backend = fixture.delaying_backend();
    let held = backend.hold_job(crate::desktop::services::JobKind::OperationResume);
    let (worker, mut events) = WorkerHandle::spawn(backend);
    let operation = OperationId::new();
    let resume = || Intent::OperationResume {
        root: root.clone(),
        operation_id: operation,
    };

    let first = worker.submit(resume(), Lane::User, None, None);
    held.wait_reached();
    assert_eq!(worker.cancel(first), CancelResult::Signalled, "Stop");
    let again = worker.submit(resume(), Lane::User, None, None);
    assert_ne!(again, first, "sent after the Stop: a new request");
    assert_eq!(
        worker.submit(resume(), Lane::User, None, None),
        again,
        "that one is a twin of the next"
    );

    held.release();
    for want in [first, again] {
        while let Some(event) = futures::executor::block_on(events.next()) {
            if matches!(event, WorkerEvent::Finished(id, _) if id == want) {
                break;
            }
        }
    }
    let after = worker.submit(resume(), Lane::User, None, None);
    assert!(![first, again].contains(&after), "answered: not a twin");
}

/// L-4, U-15: a Check now left active by Quit now or a crash is listed with
/// no next action, and its pending row offers Resume; Resume answered
/// `original_request_required` sends `PollOnce{root}` under the request the
/// answer names, on the user lane. A row that is not a poll's, or that does
/// not hold the reservation, is not offered it and is not resent as one.
#[test]
fn resume_dead_check_now() {
    use crate::desktop::model::operations::{ResumeOffer, resume_offer};
    use futures::StreamExt;

    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (worker, mut events) = WorkerHandle::spawn(fixture.delaying_backend());
    let mut operations = Operations::default();
    let resends = Resends::default();
    let operation = OperationId::new();
    let poll_row = |state: &str| OperationDto {
        operation_id: Some(operation.to_string()),
        family: OperationFamily::Remote,
        owner: OperationOwner::Repository,
        action: OperationAction::Poll,
        state: state.to_owned(),
        completed_step: None,
        next_action: None,
        item_id: None,
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    };

    // Every phase that holds the reservation offers Resume; a poll that
    // ended, and rows of other kinds, do not.
    for state in ["reserved", "advertising", "fetch_prepared", "reconciling"] {
        assert_eq!(
            resume_offer(&poll_row(state)),
            ResumeOffer::Offered,
            "{state}"
        );
    }
    for state in ["completed", "interrupted", "cancelled", "failed"] {
        assert_eq!(
            resume_offer(&poll_row(state)),
            ResumeOffer::NotOffered,
            "{state}"
        );
    }
    let mut sync = poll_row("advertising");
    sync.action = OperationAction::SynchronizePrimary;
    assert_eq!(resume_offer(&sync), ResumeOffer::NotOffered);
    let mut unnamed = poll_row("advertising");
    unnamed.operation_id = None;
    assert_eq!(resume_offer(&unnamed), ResumeOffer::NotOffered);

    // The resume is answered with the request that holds the row. Until the
    // rows list the operation as a poll's, nothing is resent for it.
    let named = RequestId::new();
    let resume = |operations: &mut Operations, n: u64| {
        finished(
            operations,
            n,
            Intent::OperationResume {
                root: root.clone(),
                operation_id: operation,
            },
            RequestId::new(),
        )
    };
    let unlisted = resume(&mut operations, 1);
    resends.ended(unlisted, &original_request(named), &operations, &worker);
    assert_eq!(resends.follow(unlisted), None, "no row names a poll");

    let stamp = operations.load_pending(root.to_str().unwrap().to_owned());
    operations.apply_pending(
        stamp,
        ReadResult::Ok(OperationListDto {
            items: vec![poll_row("advertising")],
            complete: true,
        }),
    );
    let sent = resume(&mut operations, 2);
    resends.ended(sent, &original_request(named), &operations, &worker);
    let followed = resends
        .follow(sent)
        .expect("the poll once follows the resume");
    let queued = loop {
        match futures::executor::block_on(events.next()) {
            Some(WorkerEvent::Queued(request)) if request.id == followed => break request,
            Some(_) => {}
            None => panic!("the worker hung up"),
        }
    };
    assert!(matches!(&queued.intent, Intent::PollOnce { root: asked } if *asked == root));
    assert_eq!(queued.request_id, named, "under the request that holds it");
    assert_eq!(queued.lane, Lane::User);
    assert!(queued.intent.stops_at_safe_points());
}
