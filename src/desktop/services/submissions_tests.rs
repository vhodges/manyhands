//! The session's comment submissions: one post of a comment at a time,
//! its request ID the comment ID (R-16), a retry under it, a restart
//! under a new one, and an answer that lands with no tab open (P9).

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    drafts::DraftId,
    repository::{AuthoringKind, MutationDataDto, RequestId},
    results::{Envelope, ResultCode, Scope},
};

use super::{Posted, SubmissionState, Submissions};
use crate::desktop::{
    model::publication::CommentOutcome,
    services::{
        JobKind, Services,
        requests::{CommentBody, DesktopRequest, DesktopRequestId, Intent, Lane},
        worker::{CancelResult, RecordState, WorkerEvent, WorkerHandle},
    },
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
    view::harness::wait_until,
};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG2";

/// Long enough for a comment on a loaded machine.
const WAIT: Duration = Duration::from_secs(10);

fn comment(root: &Path, comment_id: &str) -> Intent {
    Intent::CommentAdd {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
        comment_id: comment_id.parse().unwrap(),
        parent_id: None,
        body: CommentBody::new("A comment.".to_owned()),
    }
}

fn item_id(id: &str) -> ItemId {
    id.parse().unwrap()
}

/// A registry over a worker of its own, with no remote: a post is saved
/// here and its record finished.
struct Harness {
    fixture: Fixture,
    backend: Arc<DelayingBackend>,
    root: PathBuf,
    events: UnboundedReceiver<WorkerEvent>,
    submissions: Submissions,
    /// What the registry left for the drain at the last end.
    posted: Option<Posted>,
}

impl Harness {
    fn new() -> Self {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        fixture.create_ticket(&root, ITEM, "One", "open");
        let backend = fixture.delaying_backend();
        let (worker, events) = WorkerHandle::spawn(backend.clone());
        Self {
            fixture,
            backend,
            root,
            events,
            submissions: Submissions::new(worker),
            posted: None,
        }
    }

    /// The events up to `id`'s end: what was queued, and its answer,
    /// handed to the registry as the services drain hands it.
    fn until_end_of(&mut self, id: DesktopRequestId) -> (Vec<DesktopRequest>, ResultCode) {
        let deadline = Instant::now() + WAIT;
        let mut queued = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(WorkerEvent::Queued(request)) => queued.push(request),
                Ok(WorkerEvent::Finished(of, envelope)) if of == id => {
                    let code = envelope.code;
                    self.posted = self.submissions.finished(id, envelope);
                    assert!(self.posted.is_some());
                    return (queued, code);
                }
                Ok(_) => {}
                Err(TryRecvError::Closed) => panic!("the worker hung up"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "no end in {WAIT:?}");
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    /// The last answer of the post of `comment_id`, as it was classified.
    fn outcome(&self, comment_id: &str) -> (CommentOutcome, bool) {
        let Some(SubmissionState::Finished { result, .. }) = self.state(comment_id) else {
            panic!("the post finished");
        };
        (result.outcome.clone(), result.attempt.resend)
    }

    fn state(&self, comment_id: &str) -> Option<&SubmissionState> {
        self.submissions
            .get(&self.root, &item_id(comment_id))
            .map(|submission| &submission.state)
    }
}

/// P10: while a comment is posting, a second Submit of it is refused.
#[test]
fn a_second_submit_while_posting_is_refused() {
    let mut harness = Harness::new();
    let held = harness.backend.hold_job(JobKind::Comment);
    let id = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), None, false)
        .expect("the first post is sent");
    held.wait_reached();
    assert!(matches!(
        harness.state(COMMENT),
        Some(SubmissionState::Posting)
    ));
    assert!(
        harness
            .submissions
            .submit(comment(&harness.root, COMMENT), None, false)
            .is_none()
    );
    held.release();
    let (queued, code) = harness.until_end_of(id);
    assert_eq!(queued.len(), 1);
    assert_eq!(code, ResultCode::Ok);
}

/// R-16: a post's library request ID is its comment ID.
#[test]
fn a_posts_request_id_is_its_comment_id() {
    let mut harness = Harness::new();
    let id = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), None, false)
        .unwrap();
    assert_eq!(
        harness
            .submissions
            .get(&harness.root, &item_id(COMMENT))
            .unwrap()
            .request,
        id
    );
    let (queued, _) = harness.until_end_of(id);
    assert_eq!(queued[0].request_id, RequestId::parse(COMMENT).unwrap());
    let Some(SubmissionState::Finished { envelope, result }) = harness.state(COMMENT) else {
        panic!("the post finished");
    };
    assert_eq!(envelope.request_id.as_deref(), Some(COMMENT));
    assert_eq!(result.attempt.record, RecordState::Finished);
}

/// Retry resubmits the same request: same worker request, same request
/// ID.
#[test]
fn retry_keeps_the_request_id() {
    let mut harness = Harness::new();
    let id = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), None, false)
        .unwrap();
    harness.until_end_of(id);

    assert!(harness.submissions.retry(&harness.root, &item_id(COMMENT)));
    // Another repository's comment of the same ID is not this post.
    assert!(
        !harness
            .submissions
            .retry(Path::new("/elsewhere"), &item_id(COMMENT))
    );
    assert!(matches!(
        harness.state(COMMENT),
        Some(SubmissionState::Posting)
    ));
    let (queued, code) = harness.until_end_of(id);
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].id, id);
    assert_eq!(queued[0].attempt, 2);
    assert_eq!(queued[0].request_id, RequestId::parse(COMMENT).unwrap());
    assert_eq!(code, ResultCode::Ok);
}

/// Restart forgets the old comment and posts under the new comment ID,
/// and so a new request ID.
#[test]
fn restart_moves_to_a_new_comment_id() {
    let mut harness = Harness::new();
    let first = harness
        .submissions
        .submit(
            comment(&harness.root, COMMENT),
            Some(DraftId::generate()),
            false,
        )
        .unwrap();
    harness.until_end_of(first);

    // The new comment ID must be the intent's.
    assert!(
        harness
            .submissions
            .restart(
                &item_id(COMMENT),
                &item_id(OTHER),
                comment(&harness.root, COMMENT),
                None
            )
            .is_none()
    );
    // The draft re-targeted to the new comment ID goes with it (P13).
    let draft = DraftId::generate();
    let second = harness
        .submissions
        .restart(
            &item_id(COMMENT),
            &item_id(OTHER),
            comment(&harness.root, OTHER),
            Some(draft),
        )
        .expect("the restart is sent");
    assert_ne!(second, first);
    assert!(harness.state(COMMENT).is_none());
    let restarted = harness
        .submissions
        .get(&harness.root, &item_id(OTHER))
        .unwrap();
    assert_eq!(restarted.request, second);
    assert_eq!(restarted.draft, Some(draft));
    assert!(matches!(
        harness.state(OTHER),
        Some(SubmissionState::Posting)
    ));
    let (queued, _) = harness.until_end_of(second);
    assert_eq!(queued[0].request_id, RequestId::parse(OTHER).unwrap());
}

/// P9: the services drain, not a tab, applies a post's answer, so it
/// lands with no tab open and a later Submit is not refused.
#[gpui_kit::test]
async fn a_finished_event_with_no_tab_open_updates_the_submission(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, ITEM, "One", "open");
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx).without_launch_refresh());

    let id = services
        .submissions()
        .update(cx, |submissions, _| {
            submissions.submit(comment(&root, COMMENT), None, false)
        })
        .expect("the post is sent");
    let comment_id = item_id(COMMENT);
    wait_until(cx, services.submissions(), |submissions, _| {
        submissions
            .get(&root, &comment_id)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Finished { .. }))
    })
    .await;

    services.submissions().read_with(cx, |submissions, _| {
        let submission = submissions.get(&root, &comment_id).unwrap();
        assert_eq!(submission.request, id);
        let SubmissionState::Finished { envelope, result } = &submission.state else {
            unreachable!();
        };
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert!(matches!(envelope.data, Some(MutationDataDto::Comment(_))));
        assert_eq!(result.attempt.record, RecordState::Finished);
    });
    // No longer posting: the same comment may be sent again.
    let again = services.submissions().update(cx, |submissions, _| {
        submissions.submit(comment(&root, COMMENT), None, false)
    });
    assert!(again.is_some());
    wait_until(cx, services.submissions(), |submissions, _| {
        submissions
            .get(&root, &comment_id)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Finished { .. }))
    })
    .await;
}

/// A retry removed before it ran (Stop while queued, or quitting) brings
/// back the answer the post already had: the post is not forgotten, so no
/// second comment is offered (P2, P7, P15).
#[test]
fn a_removed_retry_restores_the_previous_finished_state() {
    let mut harness = Harness::new();
    let id = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), None, false)
        .unwrap();
    harness.until_end_of(id);

    let held = harness.backend.hold_job(JobKind::IndexRefresh);
    let worker = harness.submissions.worker.clone();
    worker.submit(
        Intent::IndexRefresh {
            root: harness.root.clone(),
        },
        Lane::User,
        None,
        None,
    );
    held.wait_reached();
    assert!(harness.submissions.retry(&harness.root, &item_id(COMMENT)));
    assert!(matches!(
        harness.state(COMMENT),
        Some(SubmissionState::Posting)
    ));
    assert_eq!(worker.cancel(id), CancelResult::Removed);
    held.release();
    // The drain hands the registry the removal.
    assert!(harness.submissions.ended(id));
    let Some(SubmissionState::Finished { envelope, result }) = harness.state(COMMENT) else {
        panic!("the earlier answer is back");
    };
    assert_eq!(envelope.request_id.as_deref(), Some(COMMENT));
    assert_eq!(result.attempt.record, RecordState::Finished);

    // A post that never answered is forgotten.
    let held = harness.backend.hold_job(JobKind::IndexRefresh);
    worker.submit(
        Intent::IndexRefresh {
            root: harness.root.clone(),
        },
        Lane::User,
        None,
        None,
    );
    held.wait_reached();
    let other = harness
        .submissions
        .submit(comment(&harness.root, OTHER), None, false)
        .unwrap();
    assert_eq!(worker.cancel(other), CancelResult::Removed);
    held.release();
    assert!(harness.submissions.ended(other));
    assert!(harness.state(OTHER).is_none());
}

/// Give up of a post runs as posting: a Submit of the comment meanwhile is
/// refused (P10), and the post is posting again until it answers.
#[test]
fn a_second_submit_while_giving_up_is_refused() {
    let mut harness = Harness::new();
    let id = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), None, false)
        .unwrap();
    harness.until_end_of(id);

    let held = harness.backend.hold_job(JobKind::Comment);
    assert!(
        harness
            .submissions
            .give_up(&harness.root, &item_id(COMMENT))
    );
    held.wait_reached();
    assert!(matches!(
        harness.state(COMMENT),
        Some(SubmissionState::Posting)
    ));
    assert!(
        harness
            .submissions
            .submit(comment(&harness.root, COMMENT), None, false)
            .is_none()
    );
    // A second Give up, or a retry, is refused while it runs.
    assert!(
        !harness
            .submissions
            .give_up(&harness.root, &item_id(COMMENT))
    );
    assert!(!harness.submissions.retry(&harness.root, &item_id(COMMENT)));
    held.release();
    let (queued, code) = harness.until_end_of(id);
    assert_eq!(queued.len(), 1);
    assert_eq!(code, ResultCode::Ok);
    assert!(matches!(
        harness.state(COMMENT),
        Some(SubmissionState::Finished { .. })
    ));
}

/// A constructed answer of `internal_error` with nothing written: what a
/// re-sent comment ID whose binding is found but not its record answers
/// (P15). The library's own call still runs before it is put in place.
fn internal_error() -> Envelope<MutationDataDto> {
    Envelope::failure(
        "comment add",
        Scope::default(),
        ResultCode::InternalError,
        vec![],
    )
}

/// P15: an answer is classified under its attempt. A first post's
/// `internal_error` with its record finished is not saved; the same
/// comment ID submitted again, or retried, is a re-send, and the same
/// answer may be saved. What the drain is left names the draft and
/// whether the comment is saved.
#[test]
fn an_answer_is_classified_as_a_resend_after_the_first_attempt() {
    let mut harness = Harness::new();
    let draft = DraftId::generate();
    let returned = harness
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    let id = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), Some(draft), false)
        .unwrap();
    returned.release();
    harness.until_end_of(id);
    assert_eq!(
        harness.outcome(COMMENT),
        (
            CommentOutcome::NotSaved {
                code: ResultCode::InternalError
            },
            false
        )
    );
    assert_eq!(
        harness.posted,
        Some(Posted {
            draft: Some(draft),
            saved: false
        })
    );

    // A Submit of a comment this session posted is a re-send.
    let returned = harness
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    let again = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), Some(draft), false)
        .unwrap();
    returned.release();
    harness.until_end_of(again);
    assert_eq!(harness.outcome(COMMENT), (CommentOutcome::MaybeSaved, true));

    // A retry is one too; its replayed answer is saved here (no remote).
    assert!(harness.submissions.retry(&harness.root, &item_id(COMMENT)));
    harness.until_end_of(again);
    assert_eq!(harness.outcome(COMMENT), (CommentOutcome::SavedHere, true));
    assert_eq!(
        harness.posted,
        Some(Posted {
            draft: Some(draft),
            saved: true
        })
    );
}

/// P4, P9, P11, P15: an adopted draft's first post after a relaunch is a
/// re-send of its comment ID. Its `internal_error`, landing through the
/// services drain with no tab open, may be saved: never not saved, which
/// would offer Edit text under a new comment ID.
#[gpui_kit::test]
async fn an_adopted_resend_answering_internal_error_with_no_tab_open_is_maybe_saved(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, ITEM, "One", "open");
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx).without_launch_refresh());

    let returned = backend.hold_job_return_answering(JobKind::Comment, internal_error());
    services
        .submissions()
        .update(cx, |submissions, _| {
            submissions.submit(comment(&root, COMMENT), Some(DraftId::generate()), true)
        })
        .expect("the post is sent");
    returned.release();
    let comment_id = item_id(COMMENT);
    wait_until(cx, services.submissions(), |submissions, _| {
        submissions
            .get(&root, &comment_id)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Finished { .. }))
    })
    .await;

    services.submissions().read_with(cx, |submissions, _| {
        let SubmissionState::Finished { envelope, result } =
            &submissions.get(&root, &comment_id).unwrap().state
        else {
            unreachable!();
        };
        assert_eq!(envelope.code, ResultCode::InternalError);
        assert!(result.attempt.resend);
        assert_eq!(result.attempt.record, RecordState::Finished);
        assert_eq!(result.outcome, CommentOutcome::MaybeSaved);
        assert!(!matches!(result.outcome, CommentOutcome::NotSaved { .. }));
    });
}

/// P10: a retry of a post sent from the operation area, straight to the
/// worker, goes through `Submissions`: the drain marks the post posting
/// again as a re-send, so a Submit of it is refused while the retry runs.
#[gpui_kit::test]
async fn a_retry_from_the_operation_area_posts_through_submissions(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, ITEM, "One", "open");
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx).without_launch_refresh());
    let comment_id = item_id(COMMENT);
    let finished = |submissions: &Submissions, _: &gpui_kit::App| {
        submissions
            .get(&root, &comment_id)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Finished { .. }))
    };

    let id = services
        .submissions()
        .update(cx, |submissions, _| {
            submissions.submit(comment(&root, COMMENT), None, false)
        })
        .expect("the post is sent");
    wait_until(cx, services.submissions(), finished).await;

    let held = backend.hold_job(JobKind::Comment);
    assert!(services.worker().retry(id, None));
    wait_until(cx, services.submissions(), |submissions, _| {
        submissions
            .get(&root, &comment_id)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Posting))
    })
    .await;
    held.wait_reached();
    let refused = services.submissions().update(cx, |submissions, _| {
        submissions.submit(comment(&root, COMMENT), None, false)
    });
    assert!(refused.is_none());
    held.release();
    wait_until(cx, services.submissions(), finished).await;

    services.submissions().read_with(cx, |submissions, _| {
        let submission = submissions.get(&root, &comment_id).unwrap();
        assert_eq!(submission.request, id);
        let SubmissionState::Finished { result, .. } = &submission.state else {
            unreachable!();
        };
        assert!(result.attempt.resend);
        assert_eq!(result.outcome, CommentOutcome::SavedHere);
    });
}

/// Restart forgets the old comment of the repository it posts to, by
/// repository and comment ID: another repository's comment of the same
/// ID stays.
#[test]
fn restart_forgets_the_old_comment_of_its_own_repository_only() {
    let mut harness = Harness::new();
    let elsewhere = harness.fixture.create_repository("b");
    harness
        .fixture
        .create_ticket(&elsewhere, ITEM, "One", "open");
    let here = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), None, false)
        .unwrap();
    harness.until_end_of(here);
    let there = harness
        .submissions
        .submit(comment(&elsewhere, COMMENT), None, false)
        .unwrap();
    harness.until_end_of(there);

    let restarted = harness
        .submissions
        .restart(
            &item_id(COMMENT),
            &item_id(OTHER),
            comment(&elsewhere, OTHER),
            None,
        )
        .expect("the restart is sent");
    assert!(
        harness
            .submissions
            .get(&elsewhere, &item_id(COMMENT))
            .is_none()
    );
    assert_eq!(
        harness
            .submissions
            .get(&harness.root, &item_id(COMMENT))
            .map(|submission| submission.request),
        Some(here)
    );
    harness.until_end_of(restarted);
}
