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
    results::ResultCode,
};

use super::{SubmissionState, Submissions};
use crate::desktop::{
    services::{
        JobKind, Services,
        requests::{CommentBody, DesktopRequest, DesktopRequestId, Intent},
        worker::{RecordState, WorkerEvent, WorkerHandle},
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
    _fixture: Fixture,
    backend: Arc<DelayingBackend>,
    root: PathBuf,
    events: UnboundedReceiver<WorkerEvent>,
    submissions: Submissions,
}

impl Harness {
    fn new() -> Self {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        fixture.create_ticket(&root, ITEM, "One", "open");
        let backend = fixture.delaying_backend();
        let (worker, events) = WorkerHandle::spawn(backend.clone());
        Self {
            _fixture: fixture,
            backend,
            root,
            events,
            submissions: Submissions::new(worker),
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
                    assert!(self.submissions.finished(id, envelope));
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
        .submit(comment(&harness.root, COMMENT), None)
        .expect("the first post is sent");
    held.wait_reached();
    assert!(matches!(
        harness.state(COMMENT),
        Some(SubmissionState::Posting)
    ));
    assert!(
        harness
            .submissions
            .submit(comment(&harness.root, COMMENT), None)
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
        .submit(comment(&harness.root, COMMENT), None)
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
    let Some(SubmissionState::Finished { envelope, record }) = harness.state(COMMENT) else {
        panic!("the post finished");
    };
    assert_eq!(envelope.request_id.as_deref(), Some(COMMENT));
    assert_eq!(*record, RecordState::Finished);
}

/// Retry resubmits the same request: same worker request, same request
/// ID.
#[test]
fn retry_keeps_the_request_id() {
    let mut harness = Harness::new();
    let id = harness
        .submissions
        .submit(comment(&harness.root, COMMENT), None)
        .unwrap();
    harness.until_end_of(id);

    assert!(harness.submissions.retry(&item_id(COMMENT)));
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
        .submit(comment(&harness.root, COMMENT), Some(DraftId::generate()))
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
            submissions.submit(comment(&root, COMMENT), None)
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
        let SubmissionState::Finished { envelope, record } = &submission.state else {
            unreachable!();
        };
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert!(matches!(envelope.data, Some(MutationDataDto::Comment(_))));
        assert_eq!(*record, RecordState::Finished);
    });
    // No longer posting: the same comment may be sent again.
    let again = services.submissions().update(cx, |submissions, _| {
        submissions.submit(comment(&root, COMMENT), None)
    });
    assert!(again.is_some());
    wait_until(cx, services.submissions(), |submissions, _| {
        submissions
            .get(&root, &comment_id)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Finished { .. }))
    })
    .await;
}
