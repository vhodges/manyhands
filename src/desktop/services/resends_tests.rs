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
    resends.after_republish(republish, AfterRepublish::Retry(run));
    assert!(resends.pending_for(run));
    resends.forget(run);
    assert!(resends.pending_for(run), "the run's dismissal keeps it");
    resends.forget(republish);
    assert!(!resends.pending_for(run));
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
        resends.after_republish(id, resume_after(run));
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
    resends.after_republish(removed, resume_after(run));
    resends.not_run(removed);
    assert!(!resends.pending_for(run), "removed or not sent");

    let done = finished(&mut operations, 11, republish(&root), RequestId::new());
    resends.after_republish(done, resume_after(run));
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
