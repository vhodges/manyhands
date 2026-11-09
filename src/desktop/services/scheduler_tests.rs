//! R-4's restart schedule and the stale rows L-1 to L-6 (D6 Task 4), over the
//! real worker, the recording double and a real in-flight record. The clock
//! is moved by hand; nothing sleeps.

use std::{
    fs,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use manyhands::repository::{OperationId, PollCompletion, RemotePollError};
use tempfile::TempDir;
use time::OffsetDateTime;

use super::{Holding, Planned, apply, holding, plan, start};
use crate::desktop::{
    model::polling::{
        Answer, AttemptSummary, Clock, NotEligible, Phase, Policy, PollAttempt, PollSchedule,
        TestClock, Why,
    },
    services::{
        inflight::{Acquired, ActiveRow, InflightStore},
        poll_convert::{Library, ReportParts},
        worker::{WorkerEvent, WorkerHandle},
    },
    test_support::{WAIT, fixtures::Fixture, polling::PollingBackend},
};

fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn open_policy() -> Policy {
    Policy {
        enabled: true,
        paused: false,
        interval: secs(300),
        backoff: None,
        recovery_suspended: false,
        next_eligible_at: None,
        active_operation: None,
    }
}

fn report(completion: PollCompletion, completed_at: OffsetDateTime) -> Library {
    Library::Polled {
        parts: ReportParts {
            completion,
            completed_at,
            next_eligible_at: None,
            targets: Vec::new(),
            outcome: None,
        },
        index_pending: false,
    }
}

/// A repository, its schedule and record, a worker over the double, and a
/// launch time.
struct World {
    _fixture: Fixture,
    _data: TempDir,
    clock: TestClock,
    schedule: PollSchedule,
    store: InflightStore,
    backend: Arc<PollingBackend>,
    worker: WorkerHandle,
    events: UnboundedReceiver<WorkerEvent>,
    root: String,
    launched_at: OffsetDateTime,
}

impl World {
    /// A launch whose record, left by a process that died, holds a poll
    /// entry for the repository under `seed`, if any. This instance takes
    /// the lock.
    fn launch(seed: Option<OperationId>) -> Self {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a").to_str().unwrap().to_owned();
        let data = tempfile::tempdir().unwrap();
        if let Some(operation) = seed {
            let mut previous = InflightStore::open(data.path());
            assert_eq!(previous.try_acquire(), Acquired::Now);
            previous.poll_submitting(&root, operation);
        }
        let mut store = InflightStore::open(data.path());
        assert_eq!(store.try_acquire(), Acquired::Now);
        Self::over(fixture, data, store, root)
    }

    fn over(fixture: Fixture, data: TempDir, store: InflightStore, root: String) -> Self {
        let backend = Arc::new(PollingBackend::new(fixture.delaying_backend()));
        let (worker, events) = WorkerHandle::spawn(backend.clone());
        let clock = TestClock::new();
        let mut schedule = PollSchedule::default();
        schedule.add(root.clone(), open_policy(), None);
        let launched_at = clock.utc();
        Self {
            _fixture: fixture,
            _data: data,
            clock,
            schedule,
            store,
            backend,
            worker,
            events,
            root,
            launched_at,
        }
    }

    fn due(&self) -> bool {
        self.schedule.due(&self.clock.now()) == [self.root.clone()]
    }

    fn plan(&self) -> Planned {
        plan(&self.schedule, &self.store, &self.root)
    }

    fn holding(&self) -> Option<Holding> {
        holding(&self.schedule, &self.store, &self.root, &self.clock.now())
    }

    /// Sends the poll of the due repository and waits for its attempt.
    fn poll(&mut self) -> PollAttempt {
        let (worker, root) = (&self.worker, self.root.clone());
        let record = self._data.path().join("inflight").join("record.json");
        let holds_lock = self.store.holds_lock();
        let sent = start(
            &mut self.schedule,
            &mut self.store,
            &self.root,
            |op, restart| {
                // R-31: the entry is on disk before the poll is submitted.
                if holds_lock {
                    let written = fs::read_to_string(&record).unwrap();
                    assert!(
                        written.contains(&op.to_string()),
                        "the entry was not written before the submit"
                    );
                }
                worker.submit_poll(root, op, restart)
            },
        );
        assert!(sent.is_some());
        let deadline = Instant::now() + WAIT;
        loop {
            match self.events.try_recv() {
                Ok(WorkerEvent::Polled(_, attempt)) => return attempt,
                Ok(_) => {}
                Err(TryRecvError::Closed) => panic!("the worker hung up"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "no poll attempt in {WAIT:?}");
                    thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }

    fn apply(&mut self, attempt: &PollAttempt, active: ActiveRow) -> Option<AttemptSummary> {
        apply(
            &mut self.schedule,
            &mut self.store,
            &self.clock.now(),
            attempt,
            active,
            self.launched_at,
        )
    }

    fn phase(&self) -> Phase {
        self.schedule.repo(&self.root).unwrap().phase.clone()
    }

    fn entry(&self) -> Option<OperationId> {
        self.store.poll_entry(&self.root)
    }
}

/// How the first attempt of a kind is answered, for R-4's schedule.
enum Fails {
    /// `Err(Busy)`, with another operation active.
    Busy,
    /// `Err(Interrupted)`.
    Interrupted,
    /// `Ok(Polled{Interrupted})`, which only a constructed report can be.
    PolledInterrupted,
}

/// `Err(Busy)`, `Err(Interrupted)` and `Ok(Polled{Interrupted})` resend the
/// same operation with `restart=true` after 5, 15 and 60 s of the injected
/// clock, then wait the interval and use a new operation, with no failure
/// backoff (R-4, O-5). `Busy` for an operation the record does not hold only
/// waits (L-5).
#[test]
fn restart_schedule_same_op() {
    for (name, fails) in [
        ("Err(Busy)", Fails::Busy),
        ("Err(Interrupted)", Fails::Interrupted),
        ("Ok(Polled{Interrupted})", Fails::PolledInterrupted),
    ] {
        let mut world = World::launch(None);
        let foreign = OperationId::new();
        let (answer, active) = match fails {
            Fails::Busy => (Answer::Busy, ActiveRow::Operation(foreign)),
            Fails::Interrupted | Fails::PolledInterrupted => {
                (Answer::Interrupted, ActiveRow::Nothing)
            }
        };
        let fail = |world: &World| match fails {
            Fails::Busy => world.backend.fail_poll(RemotePollError::Busy),
            Fails::Interrupted => world.backend.fail_poll(RemotePollError::Interrupted),
            Fails::PolledInterrupted => world
                .backend
                .answer_poll(report(PollCompletion::Interrupted, world.clock.utc())),
        };

        // The first attempt is new; each of the next three waits its delay
        // and resends it.
        let mut operation = None;
        for (step, wait) in [5, 15, 60].into_iter().enumerate() {
            assert!(world.due(), "{name}: step {step}");
            fail(&world);
            let attempt = world.poll();
            let first = *operation.get_or_insert(attempt.operation);
            assert_eq!(attempt.operation, first, "{name}: step {step}");
            assert_eq!(attempt.answer, answer, "{name}");
            world.apply(&attempt, active);
            assert_eq!(world.entry(), None, "{name}: no one else's row is held");
            assert!(!world.due(), "{name}: waiting");
            world.clock.advance(secs(wait) - Duration::from_millis(1));
            assert!(!world.due(), "{name}: not before {wait} s");
            world.clock.advance(Duration::from_millis(1));
            assert!(world.due(), "{name}: after {wait} s");
        }
        // The fourth failure waits the interval and then uses a new
        // operation.
        fail(&world);
        let attempt = world.poll();
        assert_eq!(attempt.operation, operation.unwrap());
        world.apply(&attempt, active);
        world.clock.advance(secs(299));
        assert!(!world.due(), "{name}: the interval is 300 s");
        world.clock.advance(secs(1));
        assert!(world.due());
        assert!(matches!(world.phase(), Phase::Waiting { op: None, .. }));
        world
            .backend
            .answer_poll(report(PollCompletion::Completed, world.clock.utc()));
        let fresh = world.poll();
        assert_ne!(Some(fresh.operation), operation, "{name}: a new operation");

        // What the library was sent: the same operation three times with
        // `restart=true` after the first, then the new one.
        let sent = world.backend.requests();
        let first = operation.unwrap();
        assert_eq!(
            sent,
            [
                (first, false),
                (first, true),
                (first, true),
                (first, true),
                (fresh.operation, false),
            ],
            "{name}"
        );
    }
}

/// `Err(ConflictPending)` waits the interval, not R-4's steps and not the
/// stored deadline (A4).
#[test]
fn conflict_pending_waits() {
    let mut world = World::launch(None);
    world.backend.fail_poll(RemotePollError::ConflictPending);
    let attempt = world.poll();
    assert_eq!(attempt.answer, Answer::ConflictPending);
    world.apply(&attempt, ActiveRow::Nothing);
    assert_eq!(world.entry(), None);
    assert_eq!(world.phase(), Phase::Idle);
    world.clock.advance(secs(299));
    assert!(!world.due(), "not before the interval");
    world.clock.advance(secs(1));
    assert!(world.due());
    // A new operation, not a restart.
    assert!(!world.plan().restart);
    assert_ne!(world.plan().operation, attempt.operation);
}
#[path = "scheduler_rows_tests.rs"]
mod rows;

/// L-1 to L-6 (design B3): see `rows::stale_rows`.
#[test]
fn stale_rows_table() {
    rows::stale_rows();
}
