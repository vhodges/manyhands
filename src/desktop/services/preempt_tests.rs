//! Preemption (D6 Task 4, design B2): P-1 to P-7 in one table, over the real
//! worker and the recording double. Nothing sleeps: the cancel task's
//! schedule is injected and its pauses are recorded, not waited.

use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use gpui_kit::TestAppContext;
use manyhands::repository::{OperationId, PollCompletion, RequestId};
use time::OffsetDateTime;

use super::cancel_until_answered;
use crate::desktop::{
    model::polling::{
        Answer, CancelCause, Clock, Phase, Policy, PollAttempt, PollSchedule, TestClock,
    },
    services::{
        JobKind, Services,
        inflight::{ActiveRow, InflightStore},
        poll_convert::{Library, ReportParts},
        requests::{DesktopRequestId, Intent, Lane},
        scheduler,
        worker::{WorkerEvent, WorkerHandle},
    },
    test_support::{
        WAIT,
        fixtures::Fixture,
        polling::{CancelStep, PollCall, PollingBackend},
    },
};

/// The injected schedule: three distinct steps and a repeat, so a test sees
/// which delay each resend waited.
const DELAYS: [Duration; 3] = [
    Duration::from_millis(1),
    Duration::from_millis(2),
    Duration::from_millis(4),
];

#[derive(Debug, PartialEq)]
enum Heard {
    Queued(DesktopRequestId),
    Started(DesktopRequestId),
    Finished(DesktopRequestId),
    Host(DesktopRequestId),
    Polled(PollAttempt),
    Preempt(OperationId),
}

/// One cancel task as the test ran it: the pauses it asked for.
struct Task {
    ran: JoinHandle<Vec<Duration>>,
}

struct Harness {
    _fixture: Fixture,
    backend: Arc<PollingBackend>,
    worker: WorkerHandle,
    events: UnboundedReceiver<WorkerEvent>,
    root: String,
    log: Vec<Heard>,
    tasks: Vec<Task>,
}

fn cancelled() -> Library {
    polled(PollCompletion::Cancelled)
}

fn polled(completion: PollCompletion) -> Library {
    Library::Polled {
        parts: ReportParts {
            completion,
            completed_at: OffsetDateTime::now_utc(),
            next_eligible_at: None,
            targets: Vec::new(),
            outcome: None,
        },
        index_pending: false,
    }
}

impl Harness {
    fn new() -> Self {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a").to_str().unwrap().to_owned();
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let (worker, events) = WorkerHandle::spawn(backend.clone());
        Self {
            _fixture: fixture,
            backend,
            worker,
            events,
            root,
            log: Vec::new(),
            tasks: Vec::new(),
        }
    }

    fn refresh(&self) -> Intent {
        Intent::IndexRefresh {
            root: self.root.clone().into(),
        }
    }

    /// The next event, starting the cancel task for a `PreemptPoll` as the
    /// `Services` drain does.
    fn pump(&mut self) {
        let deadline = Instant::now() + WAIT;
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    let heard = match event {
                        WorkerEvent::Queued(request) => Heard::Queued(request.id),
                        WorkerEvent::Started(id) => Heard::Started(id),
                        WorkerEvent::Finished(id, _) => Heard::Finished(id),
                        WorkerEvent::HostObserved(id, _) => Heard::Host(id),
                        WorkerEvent::Polled(_, attempt) => Heard::Polled(attempt),
                        WorkerEvent::PreemptPoll { root, op, .. } => {
                            self.start_task(root, op);
                            Heard::Preempt(op)
                        }
                        WorkerEvent::Progress(..)
                        | WorkerEvent::Removed(_)
                        | WorkerEvent::NotSent(..) => return,
                        WorkerEvent::Prompt(_) => panic!("a prompt"),
                    };
                    self.log.push(heard);
                    return;
                }
                Err(TryRecvError::Closed) => panic!("the worker hung up"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "no worker event in {WAIT:?}");
                    thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }

    fn start_task(&mut self, root: String, op: OperationId) {
        let backend = self.backend.clone();
        let watch = self.worker.poll_watch();
        let ran = thread::spawn(move || {
            let waited = Mutex::new(Vec::new());
            futures::executor::block_on(cancel_until_answered(
                backend.as_ref(),
                &watch,
                &root,
                op,
                &DELAYS,
                |delay| {
                    waited.lock().unwrap().push(delay);
                    thread::yield_now();
                    std::future::ready(())
                },
            ));
            waited.into_inner().unwrap()
        });
        self.tasks.push(Task { ran });
    }

    /// Hears events until `done` holds of the log.
    fn until(&mut self, done: impl Fn(&[Heard]) -> bool) {
        while !done(&self.log) {
            self.pump();
        }
    }

    fn until_polled(&mut self) -> PollAttempt {
        let polled = |log: &[Heard]| {
            log.iter().find_map(|heard| match heard {
                Heard::Polled(attempt) => Some(attempt.clone()),
                _ => None,
            })
        };
        self.until(|log| polled(log).is_some());
        polled(&self.log).expect("polled")
    }

    fn finished(&mut self, id: DesktopRequestId) {
        self.until(|log| {
            log.iter()
                .any(|h| matches!(h, Heard::Finished(f) | Heard::Host(f) if *f == id))
        });
    }

    /// Hears events until `id` finishes after the poll was heard: a
    /// request queued again has finished before.
    fn finished_after_poll(&mut self, id: DesktopRequestId) {
        self.until(|log| {
            let Some(at) = log.iter().position(|h| matches!(h, Heard::Polled(_))) else {
                return false;
            };
            log[at..]
                .iter()
                .any(|h| matches!(h, Heard::Finished(f) | Heard::Host(f) if *f == id))
        });
    }

    fn position(&self, wanted: impl Fn(&Heard) -> bool) -> usize {
        self.log.iter().position(wanted).expect("heard")
    }

    fn preempts(&self) -> usize {
        self.log
            .iter()
            .filter(|h| matches!(h, Heard::Preempt(_)))
            .count()
    }

    /// Every cancel task ended, and the pauses each asked for.
    fn join_tasks(&mut self) -> Vec<Vec<Duration>> {
        self.tasks
            .drain(..)
            .map(|task| task.ran.join().expect("the cancel task ended"))
            .collect()
    }

    /// A poll of the repository, held before its library call.
    fn hold_poll(
        &mut self,
        op: OperationId,
        restart: bool,
    ) -> crate::desktop::test_support::delaying::JobGate {
        let gate = self.backend.hold_poll();
        self.worker
            .submit_poll(self.root.clone(), op, restart)
            .expect("the queue is open");
        gate.wait_reached();
        gate
    }
}

#[path = "preempt_rows_tests.rs"]
mod rows;
use rows::{
    Entry, background_job_cancels_nothing, flag_checked_before_every_poll,
    preempted_polls_are_rescheduled, quit_and_reconfiguration_share_the_task,
    running_background_job_is_left_alone, user_job_preempts,
};

/// The required outcomes P-1 to P-7 (design B2).
#[test]
fn preemption_table() {
    let rows: [(&str, fn()); 11] = [
        ("P-1 submit", || user_job_preempts(Entry::Submit)),
        ("P-1 submit_with_request_id", || {
            user_job_preempts(Entry::SubmitWithRequestId)
        }),
        ("P-1 requeue", || user_job_preempts(Entry::Requeue)),
        ("P-1 observe_host", || user_job_preempts(Entry::ObserveHost)),
        ("P-2 flag before a first attempt", || {
            flag_checked_before_every_poll(false)
        }),
        ("P-2 flag before a same-ID restart", || {
            flag_checked_before_every_poll(true)
        }),
        ("P-3 P-4 rescheduled", preempted_polls_are_rescheduled),
        ("P-5 background job", background_job_cancels_nothing),
        (
            "P-6 running background job",
            running_background_job_is_left_alone,
        ),
        (
            "P-7 quit and reconfiguration",
            quit_and_reconfiguration_share_the_task,
        ),
        (
            "the cancel task stops with the worker",
            rows::cancel_task_stops_with_the_worker,
        ),
    ];
    for (name, row) in rows {
        eprintln!("{name}");
        row();
    }
}

/// The drain starts the cancel task on a background executor when the worker
/// sends `PreemptPoll`: with a poll held, a user job admitted through the
/// real `Services` gets the poll cancelled, and the poll's attempt reaches
/// the inbox as preempted.
#[gpui_kit::test]
async fn the_drain_cancels_a_preempted_poll(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a").to_str().unwrap().to_owned();
    let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
    let services = cx.update(|cx| Services::start(backend.clone(), cx).without_launch_refresh());

    backend.answer_poll(cancelled());
    let gate = backend.hold_poll();
    let op = OperationId::new();
    services
        .worker()
        .submit_poll(root.clone(), op, false)
        .expect("the queue is open");
    gate.wait_reached();
    services.worker().submit(
        Intent::IndexRefresh {
            root: root.clone().into(),
        },
        Lane::User,
        None,
        None,
    );
    let deadline = Instant::now() + WAIT;
    let mut heard = Vec::new();
    while heard.is_empty() {
        cx.run_until_parked();
        heard = services.polls().take();
        assert!(Instant::now() < deadline, "the poll was never cancelled");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
    let [(_, attempt)] = heard.as_slice() else {
        panic!("one poll");
    };
    assert_eq!(attempt.operation, op);
    assert_eq!(attempt.answer, Answer::Cancelled(CancelCause::Preemption));
    assert!(!backend.cancels().is_empty());
}

/// Review 5: a cancel task is about one run of a poll. A task still in its
/// pause when its run ended does not cancel a later run of the same
/// operation (a restart after `busy` or an interruption is another job).
/// The pause is held by the test, not slept.
#[test]
fn a_cancel_task_ignores_a_restart_of_its_operation() {
    use std::sync::mpsc;

    use futures::StreamExt;
    use futures::executor::block_on;

    let mut h = Harness::new();
    let op = OperationId::new();
    h.backend.answer_poll(cancelled());
    h.backend.answer_poll(polled(PollCompletion::Completed));
    let first = h.hold_poll(op, false);
    assert!(h.worker.ask_poll_cancel(CancelCause::Preemption));
    let id = loop {
        if let WorkerEvent::PreemptPoll { op: asked, id, .. } =
            block_on(h.events.next()).expect("the worker hung up")
        {
            assert_eq!(asked, op);
            break id;
        }
    };
    // The task for the first run, held in its first pause.
    let (paused, in_pause) = mpsc::channel::<()>();
    let (resume, resumed) = mpsc::channel::<()>();
    let task = {
        let backend = h.backend.clone();
        let watch = h.worker.poll_watch().for_run(id);
        let root = h.root.clone();
        let pauses = std::sync::atomic::AtomicUsize::new(0);
        thread::spawn(move || {
            block_on(cancel_until_answered(
                backend.as_ref(),
                &watch,
                &root,
                op,
                &DELAYS,
                |_| {
                    if pauses.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                        paused.send(()).unwrap();
                        resumed.recv().unwrap();
                    }
                    std::future::ready(())
                },
            ));
        })
    };
    in_pause.recv().expect("the task reached its pause");
    let sent = h.backend.cancels().len();
    assert!(sent > 0);

    // The first run ends; the same operation runs again as another job.
    first.release();
    let attempt = h.until_polled();
    assert_eq!(attempt.answer, Answer::Cancelled(CancelCause::Preemption));
    let second = h.hold_poll(op, true);

    // The task wakes: its run is over, so it ends without a cancel. (With
    // the run matched by operation alone it cancels the second run; the
    // second run is released then, so the task still ends.)
    resume.send(()).unwrap();
    let deadline = Instant::now() + WAIT;
    while !task.is_finished() && h.backend.cancels().len() == sent {
        assert!(
            Instant::now() < deadline,
            "the task neither ended nor cancelled"
        );
        thread::yield_now();
    }
    let cancelled_again = h.backend.cancels().len() > sent;
    second.release();
    task.join().expect("the cancel task ended");
    assert!(
        !cancelled_again,
        "a later run of the operation was cancelled"
    );
}
