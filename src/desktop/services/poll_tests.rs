//! The automatic poll on the worker's background lane (D6 Task 3), over a
//! real fixture and the recording double.

use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use gpui_kit::TestAppContext;
use manyhands::repository::{
    OperationId, SharedKeyId,
    keys::{KeySourceToken, UnlockReason, UnlockRequest},
};

use crate::desktop::{
    model::polling::{Answer, NotEligible, PollAttempt, TransportFailure},
    services::{
        JobKind, Services,
        requests::{DesktopRequestId, Intent, Lane},
        worker::{WorkerEvent, WorkerHandle},
    },
    test_support::{
        fixtures::Fixture,
        polling::{PollCall, PollingBackend},
    },
};

/// Long enough for a poll of a fixture on a loaded machine.
const WAIT: Duration = Duration::from_secs(10);

struct Harness {
    fixture: Fixture,
    backend: Arc<PollingBackend>,
    worker: WorkerHandle,
    events: UnboundedReceiver<WorkerEvent>,
    root: String,
}

/// What the worker said, reduced to what these tests ask.
#[derive(Debug, PartialEq)]
enum Heard {
    /// A request event of the request `.1`: queued, started, progress,
    /// finished, removed.
    Request(&'static str, Option<DesktopRequestId>),
    Prompt,
    Polled(String),
}

impl Harness {
    fn new() -> Self {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a").to_str().unwrap().to_owned();
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let (worker, events) = WorkerHandle::spawn(backend.clone());
        Self {
            fixture,
            backend,
            worker,
            events,
            root,
        }
    }

    /// The next event, or a panic after `WAIT`. A prompt is a failure of
    /// the tests that call this, and is returned so they can say so.
    fn next(&mut self) -> (Heard, Option<PollAttempt>) {
        let deadline = Instant::now() + WAIT;
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    return match event {
                        WorkerEvent::Queued(request) => {
                            (Heard::Request("queued", Some(request.id)), None)
                        }
                        WorkerEvent::Started(id) => (Heard::Request("started", Some(id)), None),
                        WorkerEvent::Progress(id, _) => {
                            (Heard::Request("progress", Some(id)), None)
                        }
                        WorkerEvent::Finished(id, _) => {
                            (Heard::Request("finished", Some(id)), None)
                        }
                        WorkerEvent::Removed(id) => (Heard::Request("removed", Some(id)), None),
                        WorkerEvent::NotSent(id, _) => (Heard::Request("not sent", Some(id)), None),
                        WorkerEvent::HostObserved(id, _) => {
                            (Heard::Request("host", Some(id)), None)
                        }
                        WorkerEvent::Prompt(_) => (Heard::Prompt, None),
                        WorkerEvent::Polled(root, attempt) => (Heard::Polled(root), Some(attempt)),
                    };
                }
                Err(TryRecvError::Closed) => panic!("the worker hung up"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "no worker event in {WAIT:?}");
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    /// Every event up to and including the poll's.
    fn until_polled(&mut self) -> (Vec<Heard>, PollAttempt) {
        let mut heard = Vec::new();
        loop {
            let (event, attempt) = self.next();
            heard.push(event);
            if let Some(attempt) = attempt {
                return (heard, attempt);
            }
        }
    }

    fn poll(&self, operation: OperationId) -> DesktopRequestId {
        self.worker
            .submit_poll(self.root.clone(), operation, false)
            .expect("the queue is open")
    }
}

/// A poll goes to the background lane and runs behind a user job; the UI
/// hears its attempt and nothing else of it: no `Queued`, `Started` or
/// `Progress`, because a poll is not a request.
#[test]
fn worker_poll_background() {
    let mut harness = Harness::new();
    // A user job holds the worker while the poll waits, and a second user
    // job is submitted after the poll was.
    let held = harness.backend.inner().hold_job(JobKind::IndexRefresh);
    let refresh = || Intent::IndexRefresh {
        root: harness.root.clone().into(),
    };
    let first = harness.worker.submit(refresh(), Lane::User, None, None);
    held.wait_reached();
    let operation = OperationId::new();
    let polled = harness.poll(operation);
    let second = harness.worker.submit(refresh(), Lane::User, None, None);
    drop(held);

    let (heard, attempt) = harness.until_polled();
    // Both user jobs finished before the poll was heard, though the poll
    // was submitted between them: it waited in the background lane.
    let finished = |id| {
        heard
            .iter()
            .position(|event| *event == Heard::Request("finished", Some(id)))
            .expect("the user job finished")
    };
    assert!(finished(first) < finished(second));
    assert_eq!(heard.last(), Some(&Heard::Polled(harness.root.clone())));
    assert_eq!(
        heard
            .iter()
            .filter(|event| matches!(event, Heard::Polled(_)))
            .count(),
        1
    );
    // A poll emits no request event of its own, no progress and no prompt.
    assert!(!heard.contains(&Heard::Prompt));
    assert!(
        heard
            .iter()
            .all(|event| !matches!(event, Heard::Request(_, Some(id)) if *id == polled)),
        "{heard:?}"
    );

    assert_eq!(attempt.root, harness.root);
    assert_eq!(attempt.operation, operation);
    assert_eq!(
        harness.backend.inner().jobs_seen(),
        [JobKind::IndexRefresh, JobKind::IndexRefresh, JobKind::Poll]
    );
    let _ = &harness.fixture;
}

/// The automatic path makes exactly five calls, in order; `cancel_poll`
/// is not one of them (B4, R-30).
#[test]
fn poll_call_list() {
    let mut harness = Harness::new();
    harness.poll(OperationId::new());
    let (_, attempt) = harness.until_polled();
    // A repository with no publication remote: the library stores the
    // configuration failure and answers it (Fact 31).
    assert_eq!(
        attempt.answer,
        Answer::Transport(TransportFailure::ConfigurationRequired)
    );
    assert_eq!(
        harness.backend.calls(),
        [
            PollCall::Resolve,
            PollCall::PollingStatus,
            PollCall::PollEligibility,
            PollCall::PollAutomatic,
            PollCall::RemoteSnapshot,
        ]
    );
    // The stored backoff deadline is read back after the poll.
    assert!(attempt.next_eligible_at.is_some());

    // A policy that says no is asked once and not argued with: the poll is
    // not made, so there is nothing to read back (the library would answer
    // the same `NotEligible` without contacting the remote).
    harness.fixture.pause_polling(Path::new(&harness.root));
    harness.backend.clear_calls();
    harness.poll(OperationId::new());
    let (_, attempt) = harness.until_polled();
    assert_eq!(attempt.answer, Answer::NotEligible(NotEligible::Paused));
    assert_eq!(
        harness.backend.calls(),
        [
            PollCall::Resolve,
            PollCall::PollingStatus,
            PollCall::PollEligibility,
        ]
    );
}

/// A background poll asks nobody for a passphrase: no prompt, the key is
/// not declined, it is recorded against the running request, and the poll
/// answers a locked key (B5, R-26). Through the real provider and a fresh
/// session; the double asks the session as the transport would.
#[test]
fn background_poll_never_prompts() {
    let mut harness = Harness::new();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("key");
    fs::write(&path, b"opaque fixture bytes").unwrap();
    let key = SharedKeyId::new();
    harness.backend.ask_unlock(UnlockRequest {
        key_id: key,
        label: "primary".to_owned(),
        source: KeySourceToken::observe(&path).unwrap(),
        reason: UnlockReason::ProtectedKey,
    });
    let id = harness.poll(OperationId::new());

    let (heard, attempt) = harness.until_polled();
    assert!(
        !heard.contains(&Heard::Prompt),
        "a background poll prompted: {heard:?}"
    );
    assert_eq!(
        attempt.answer,
        Answer::Transport(TransportFailure::KeyLocked { key: Some(key) })
    );
    // Refused, not declined: Not now was not the person's answer.
    assert!(!harness.worker.clear_declined(key));
    assert_eq!(harness.worker.refused_keys(id), [key]);
}

/// The services drain hands a poll's attempt to the inbox the scheduler
/// takes from, and the request model hears nothing of it: a poll is not a
/// request.
#[gpui_kit::test]
async fn the_drain_hands_a_poll_to_the_inbox(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a").to_str().unwrap().to_owned();
    let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
    let services = cx.update(|cx| Services::start(backend, cx).without_launch_refresh());

    let operation = OperationId::new();
    let id = services
        .worker()
        .submit_poll(root.clone(), operation, false)
        .expect("the queue is open");
    let deadline = Instant::now() + crate::desktop::test_support::WAIT;
    let mut heard = Vec::new();
    while heard.is_empty() {
        cx.run_until_parked();
        heard = services.polls().take();
        assert!(Instant::now() < deadline, "no poll attempt arrived");
        cx.executor().timer(Duration::from_millis(10)).await;
    }

    let [(heard_root, attempt)] = heard.as_slice() else {
        panic!("one poll was submitted");
    };
    assert_eq!((heard_root, &attempt.root), (&root, &root));
    assert_eq!(attempt.operation, operation);
    services.operations().read_with(cx, |operations, _| {
        assert!(operations.request(id).is_none());
        assert!(operations.running().is_none());
    });
}
