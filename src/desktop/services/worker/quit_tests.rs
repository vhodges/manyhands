//! Quit at the worker (D6 Task 10, design Part D): a running poll is
//! cancelled through the off-thread cancel, started once however often
//! `quit()` is called (it is called on every `Operations` notify while
//! quitting waits), and the queue is emptied. Over the real worker and the
//! recording double; the drain's cancel task is not run, only counted.

use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use futures::{StreamExt, channel::mpsc::UnboundedReceiver, executor::block_on};
use manyhands::repository::{OperationId, PollCompletion};
use time::OffsetDateTime;

use crate::desktop::{
    model::polling::{Answer, CancelCause, PollAttempt},
    services::{
        poll_convert::{Library, ReportParts},
        preempt::cancel_until_answered,
        requests::{DesktopRequestId, Intent, Lane},
        worker::{QuitState, WorkerEvent, WorkerHandle},
    },
    test_support::{fixtures::Fixture, polling::PollingBackend},
};

/// What the worker said that these tests ask about.
#[derive(Debug, PartialEq)]
enum Heard {
    Removed(DesktopRequestId),
    Polled(String, Answer, bool),
    Preempt,
    Other,
}

struct Harness {
    _fixture: Fixture,
    backend: Arc<PollingBackend>,
    worker: WorkerHandle,
    events: UnboundedReceiver<WorkerEvent>,
    first: String,
    second: String,
}

impl Harness {
    fn new() -> Self {
        let fixture = Fixture::new();
        let first = fixture.create_repository("a").to_str().unwrap().to_owned();
        let second = fixture.create_repository("b").to_str().unwrap().to_owned();
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let (worker, events) = WorkerHandle::spawn(backend.clone());
        Self {
            _fixture: fixture,
            backend,
            worker,
            events,
            first,
            second,
        }
    }

    /// The next event, blocking: the worker thread sends it.
    fn next(&mut self) -> Heard {
        match block_on(self.events.next()).expect("the worker hung up") {
            WorkerEvent::Removed(id) => Heard::Removed(id),
            WorkerEvent::Polled(
                root,
                PollAttempt {
                    answer,
                    completed_at,
                    ..
                },
            ) => Heard::Polled(root, answer, completed_at.is_some()),
            WorkerEvent::PreemptPoll { .. } => Heard::Preempt,
            _ => Heard::Other,
        }
    }

    /// Every event the worker has sent so far, without waiting.
    fn heard(&mut self) -> Vec<Heard> {
        let mut heard = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            heard.push(match event {
                WorkerEvent::Removed(id) => Heard::Removed(id),
                WorkerEvent::Polled(root, attempt) => {
                    Heard::Polled(root, attempt.answer, attempt.completed_at.is_some())
                }
                WorkerEvent::PreemptPoll { .. } => Heard::Preempt,
                _ => Heard::Other,
            });
        }
        heard
    }
}

fn cancelled() -> Library {
    Library::Polled {
        parts: ReportParts {
            completion: PollCompletion::Cancelled,
            completed_at: OffsetDateTime::now_utc(),
            next_eligible_at: None,
            targets: Vec::new(),
            outcome: None,
        },
        index_pending: false,
    }
}

/// `quit()` stops a running automatic poll, which has no token, through the
/// cancel `WorkerEvent::PreemptPoll` starts; and it is called again for
/// every `Operations` notify while quitting waits, so the cancel is started
/// once per operation, not once per call (design Part D, item 2). What is
/// queued is dropped: a queued poll is answered as cancelled by quit, which
/// never ran (the scheduler's record is settled by the ordinary answer), and
/// any other queued job is removed. This covers queued jobs of any kind;
/// queued retirements are Task 11b's, and take this same path.
#[test]
fn quit_cancels_poll() {
    let mut h = Harness::new();
    h.backend.answer_poll(cancelled());
    let gate = h.backend.hold_poll();
    let op = OperationId::new();
    h.worker
        .submit_poll(h.first.clone(), op, false)
        .expect("the queue is open");
    gate.wait_reached();
    // Queued behind the running poll: a poll of another repository, and a
    // background job.
    let queued_poll = OperationId::new();
    h.worker
        .submit_poll(h.second.clone(), queued_poll, false)
        .expect("the queue is open");
    let queued_job = h.worker.submit(
        Intent::IndexRefresh {
            root: h.second.clone().into(),
        },
        Lane::Background,
        None,
        None,
    );

    let running = match h.worker.quit() {
        QuitState::Finishing(id) => id,
        QuitState::Idle => panic!("a poll is running"),
    };
    // The notify that follows, again and again.
    for _ in 0..3 {
        assert_eq!(h.worker.quit(), QuitState::Finishing(running));
    }

    let mut heard = h.heard();
    heard.retain(|heard| *heard != Heard::Other);
    assert_eq!(
        heard.iter().filter(|h| **h == Heard::Preempt).count(),
        1,
        "the cancel is started once: {heard:?}"
    );
    assert!(
        heard.contains(&Heard::Removed(queued_job)),
        "a queued job is dropped: {heard:?}"
    );
    assert!(
        heard.contains(&Heard::Polled(
            h.second.clone(),
            Answer::Cancelled(CancelCause::Quit),
            false
        )),
        "a queued poll is answered, never run: {heard:?}"
    );
    assert!(
        h.backend.requests().iter().all(|(seen, _)| *seen == op),
        "the queued poll made no library call"
    );

    // The library's cancel, sent by the drain's task, stops the held poll
    // at its next safe point; its answer is the cause quit named.
    let cancelling = {
        let backend = h.backend.clone();
        let watch = h.worker.poll_watch();
        let root = h.first.clone();
        thread::spawn(move || {
            block_on(cancel_until_answered(
                backend.as_ref(),
                &watch,
                &root,
                op,
                &[Duration::ZERO],
                |_| std::future::ready(()),
            ));
        })
    };
    loop {
        match h.next() {
            Heard::Polled(root, answer, _) => {
                assert_eq!(root, h.first);
                assert_eq!(answer, Answer::Cancelled(CancelCause::Quit));
                break;
            }
            Heard::Preempt => panic!("a second cancel task"),
            _ => {}
        }
    }
    join(cancelling);
    assert_eq!(h.worker.quit(), QuitState::Idle, "the poll's end is seen");
    assert!(h.heard().iter().all(|heard| *heard != Heard::Preempt));
    drop(gate);
}

fn join(task: JoinHandle<()>) {
    task.join().expect("the cancel task ended");
}
