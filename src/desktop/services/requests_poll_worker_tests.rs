//! The poll intents through the worker and the request model (D6 Task 6):
//! what Cancel and Give up find, what a policy verb's `writing`-only run
//! looks like, and a policy verb sent again. A real service and worker; no
//! SSH, since none of these reaches a transport.

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use manyhands::{
    repository::{MutationDataDto, ProgressStage},
    results::{Envelope, Outcome, ResultCode},
};

use super::Intent;
use crate::desktop::{
    model::operations::{Operations, StopOffer},
    services::{
        JobKind,
        requests::{DesktopRequestId, Lane, RequestState},
        worker::{CancelResult, WorkerEvent, WorkerHandle},
    },
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

/// Long enough for a job on a loaded machine.
const WAIT: Duration = Duration::from_secs(10);

struct Harness {
    fixture: Fixture,
    backend: Arc<DelayingBackend>,
    worker: WorkerHandle,
    events: UnboundedReceiver<WorkerEvent>,
    root: PathBuf,
    /// Every event so far, applied as the UI would.
    operations: Operations,
    /// The stage of every progress event so far.
    stages: Vec<ProgressStage>,
}

impl Harness {
    fn new() -> Self {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        let backend = fixture.delaying_backend();
        let (worker, events) = WorkerHandle::spawn(backend.clone());
        Self {
            fixture,
            backend,
            worker,
            events,
            root,
            operations: Operations::default(),
            stages: Vec::new(),
        }
    }

    /// Applies the next event, or panics after `WAIT`. The envelope when
    /// the event is an end.
    fn step(&mut self) -> Option<(DesktopRequestId, Envelope<MutationDataDto>)> {
        let deadline = Instant::now() + WAIT;
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    let ended = match &event {
                        WorkerEvent::Progress(_, progress) => {
                            self.stages.push(progress.stage);
                            None
                        }
                        WorkerEvent::Finished(id, envelope) => Some((*id, (**envelope).clone())),
                        _ => None,
                    };
                    self.operations.apply(event);
                    return ended;
                }
                Err(TryRecvError::Closed) => panic!("the worker hung up"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "no worker event in {WAIT:?}");
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }

    /// Applies events until `id` finishes, and gives its answer.
    fn until_finished(&mut self, id: DesktopRequestId) -> Envelope<MutationDataDto> {
        loop {
            if let Some((ended, envelope)) = self.step()
                && ended == id
            {
                return envelope;
            }
        }
    }

    /// Applies events until a progress event of `stage` is heard.
    fn until_stage(&mut self, stage: ProgressStage) {
        let before = self.stages.iter().filter(|seen| **seen == stage).count();
        while self.stages.iter().filter(|seen| **seen == stage).count() == before {
            self.step();
        }
    }

    fn send(&self, intent: Intent) -> DesktopRequestId {
        self.worker.submit(intent, Lane::User, None, None)
    }

    /// The stored policy, as a read finds it.
    fn paused(&self) -> bool {
        let repository = self
            .fixture
            .service()
            .resolve_repository(&self.root)
            .unwrap();
        self.fixture
            .service()
            .polling_status(&repository)
            .unwrap()
            .paused
    }
}

/// `PollOnce` stops at safe points in the library's command table, and
/// the desktop arm says so: once the library has accepted it, Cancel
/// signals the token, the row offers Stop at a safe point, and Give up is
/// open. A policy verb does not: its Cancel finds it too late, and it
/// offers neither (Fact 14).
#[test]
fn intent_poll_stoppable() {
    // The flags the worker reads when it takes a job.
    let root = PathBuf::from("/repo");
    assert!(Intent::PollOnce { root: root.clone() }.stops_at_safe_points());
    assert!(Intent::PollOnce { root: root.clone() }.can_be_given_up());
    for verb in [
        Intent::PollConfigure {
            root: root.clone(),
            interval_seconds: 600,
        },
        Intent::PollPause { root: root.clone() },
        Intent::PollResume { root },
    ] {
        assert!(!verb.stops_at_safe_points(), "{}", verb.label());
        assert!(!verb.can_be_given_up(), "{}", verb.label());
    }

    // A Check now past acceptance, held, in a repository with an SSH
    // publication remote.
    let mut harness = Harness::new();
    harness.fixture.publish_over_ssh(&harness.root);
    let held = harness
        .backend
        .hold_at_stage(JobKind::PollOnce, ProgressStage::Accepted);
    let id = harness.send(Intent::PollOnce {
        root: harness.root.clone(),
    });
    held.wait_reached();
    harness.until_stage(ProgressStage::Accepted);
    assert!(matches!(
        harness.operations.request(id).unwrap().state,
        RequestState::Running { accepted: true, .. }
    ));
    assert_eq!(harness.operations.stop_offer(id, true), StopOffer::Offered);
    assert_eq!(harness.worker.cancel(id), CancelResult::Signalled);
    held.release();
    let envelope = harness.until_finished(id);
    // Stopped at the first safe point, or refused for the missing key
    // before one: either way it is no success.
    assert_ne!(envelope.outcome, Outcome::Success);
}

/// A policy verb reports `writing` and never `accepted` (Fact 2): its
/// record is inserted and finished in the one write. Any stage marks a job
/// accepted (R-14), so the request model reads it as accepted from
/// `writing`, offers neither Cancel nor Stop, and the worker answers
/// Cancel with "too late". It finishes `ok`, and the policy is stored.
#[test]
fn policy_verb_never_accepted() {
    let mut harness = Harness::new();
    let held = harness
        .backend
        .hold_at_stage(JobKind::PollPause, ProgressStage::Writing);
    let id = harness.send(Intent::PollPause {
        root: harness.root.clone(),
    });
    held.wait_reached();
    harness.until_stage(ProgressStage::Writing);
    assert!(!harness.stages.contains(&ProgressStage::Accepted));
    assert!(matches!(
        harness.operations.request(id).unwrap().state,
        RequestState::Running {
            accepted: true,
            stage: Some(ProgressStage::Writing)
        }
    ));
    assert!(harness.worker.running_job().unwrap().accepted);
    assert_eq!(
        harness.operations.stop_offer(id, true),
        StopOffer::NotOffered
    );
    assert_eq!(harness.worker.cancel(id), CancelResult::TooLate);
    held.release();

    let envelope = harness.until_finished(id);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert!(!harness.stages.contains(&ProgressStage::Accepted));
    assert!(matches!(
        harness.operations.request(id).unwrap().state,
        RequestState::Finished(Outcome::Success, ResultCode::Ok, ..)
    ));
    assert!(harness.paused());
}

/// A policy verb sent again is last commit wins (C5 decision 15, A22). A
/// request that committed replays its stored answer and writes nothing; a
/// request that never committed, stopped before its write, is a new write
/// when it is sent again, so it undoes a later verb that did commit: the
/// person sent it, and its answer says so.
#[test]
fn policy_verb_resent() {
    let mut harness = Harness::new();
    assert!(!harness.paused());

    // R1, Resume, is stopped before the library writes it.
    let held = harness.backend.hold_job(JobKind::PollResume);
    let resume = harness.send(Intent::PollResume {
        root: harness.root.clone(),
    });
    held.wait_reached();
    assert_eq!(harness.worker.cancel(resume), CancelResult::Signalled);
    held.release();
    let stopped = harness.until_finished(resume);
    assert_eq!(stopped.code, ResultCode::Cancelled);

    // R2, Pause, commits.
    let pause = harness.send(Intent::PollPause {
        root: harness.root.clone(),
    });
    assert_eq!(harness.until_finished(pause).code, ResultCode::Ok);
    assert!(harness.paused());

    // R1 sent again writes: the pause is cleared.
    assert!(harness.worker.retry(resume, None));
    assert_eq!(harness.until_finished(resume).code, ResultCode::Ok);
    assert!(!harness.paused());

    // R2 sent again replays: it does not pause a second time.
    assert!(harness.worker.retry(pause, None));
    let replayed = harness.until_finished(pause);
    assert_eq!(replayed.code, ResultCode::Ok);
    assert!(!harness.paused());
}
