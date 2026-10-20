//! The worker over a real fixture, from a plain test thread. Holds come
//! from the delaying backend and the library's own test hooks.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use manyhands::{
    repository::{
        ConfirmationId, MutationDataDto, PrepareDataDto, ProgressEvent, ProgressStage,
        ResolvedRepository, keys::SecretPassphrase,
    },
    results::{Envelope, Outcome, ResultCode},
};

use super::{CancelResult, NotSent, QuitState, WorkerEvent, WorkerHandle};
use crate::desktop::{
    model::operations::Operations,
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

/// Long enough for a refresh of a fixture on a loaded machine.
const WAIT: Duration = Duration::from_secs(10);

/// What the worker said, as a test reads it back.
enum Seen {
    Queued(DesktopRequest),
    Started(DesktopRequestId),
    Progress(DesktopRequestId, ProgressEvent),
    Finished(DesktopRequestId, Box<Envelope<MutationDataDto>>),
    HostObserved(DesktopRequestId, ResultCode, Option<String>),
    Removed(DesktopRequestId),
    NotSent(DesktopRequestId, NotSent),
    Prompt,
}

struct Harness {
    fixture: Fixture,
    backend: Arc<DelayingBackend>,
    worker: WorkerHandle,
    events: UnboundedReceiver<WorkerEvent>,
    root: PathBuf,
    /// Every event so far, applied as the UI would.
    operations: Operations,
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
        }
    }

    fn refresh(&self) -> Intent {
        Intent::IndexRefresh {
            root: self.root.clone(),
        }
    }

    /// The next event, applied to the operations model, or a panic after
    /// `WAIT`.
    fn next(&mut self) -> Seen {
        let deadline = Instant::now() + WAIT;
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    let seen = seen(&event);
                    self.operations.apply(event);
                    return seen;
                }
                Err(TryRecvError::Closed) => panic!("the worker hung up"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "no worker event in {WAIT:?}");
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    /// Every event up to and including `id`'s end.
    fn until_end_of(&mut self, id: DesktopRequestId) -> Vec<Seen> {
        let mut seen = Vec::new();
        loop {
            let event = self.next();
            let end = match &event {
                Seen::Finished(finished, _)
                | Seen::HostObserved(finished, _, _)
                | Seen::Removed(finished)
                | Seen::NotSent(finished, _) => *finished == id,
                _ => false,
            };
            seen.push(event);
            if end {
                return seen;
            }
        }
    }

    fn state(&self, id: DesktopRequestId) -> &RequestState {
        &self
            .operations
            .request(id)
            .expect("a recorded request")
            .state
    }

    fn resolved(&self) -> ResolvedRepository {
        self.fixture
            .service()
            .resolve_repository(&self.root)
            .unwrap()
    }

    /// The operations of the repository that still have work to do.
    fn pending_operations(&self) -> usize {
        let repository = self.resolved();
        let operations = self.fixture.service().list_operations(&repository).unwrap();
        operations
            .items
            .iter()
            .filter(|operation| operation.next_action.is_some())
            .count()
    }
}

fn seen(event: &WorkerEvent) -> Seen {
    match event {
        WorkerEvent::Queued(request) => Seen::Queued(DesktopRequest {
            id: request.id,
            request_id: request.request_id,
            intent: request.intent.clone(),
            confirmation: request.confirmation,
            lane: request.lane,
            attempt: request.attempt,
            state: RequestState::Queued,
        }),
        WorkerEvent::Started(id) => Seen::Started(*id),
        WorkerEvent::Progress(id, progress) => Seen::Progress(*id, progress.clone()),
        WorkerEvent::Finished(id, envelope) => Seen::Finished(*id, envelope.clone()),
        WorkerEvent::HostObserved(id, envelope) => {
            Seen::HostObserved(*id, envelope.code, envelope.scope.repository.clone())
        }
        WorkerEvent::Removed(id) => Seen::Removed(*id),
        WorkerEvent::NotSent(id, why) => Seen::NotSent(*id, *why),
        WorkerEvent::Prompt(_) => Seen::Prompt,
    }
}

fn started(seen: &[Seen]) -> Vec<DesktopRequestId> {
    seen.iter()
        .filter_map(|event| match event {
            Seen::Started(id) => Some(*id),
            _ => None,
        })
        .collect()
}

fn finished(seen: &[Seen], id: DesktopRequestId) -> &Envelope<MutationDataDto> {
    seen.iter()
        .find_map(|event| match event {
            Seen::Finished(finished, envelope) if *finished == id => Some(&**envelope),
            _ => None,
        })
        .expect("the request finished")
}

fn position(seen: &[Seen], predicate: impl Fn(&Seen) -> bool) -> usize {
    seen.iter().position(predicate).expect("the event was seen")
}

/// Whether `path`, as an event names it, is `root` or inside it.
fn within(path: &str, root: &Path) -> bool {
    Path::new(path).starts_with(root)
}

#[test]
fn a_user_job_runs_before_a_queued_background_job_and_none_is_preempted() {
    let mut harness = Harness::new();
    let held = harness.backend.hold_job(JobKind::IndexRefresh);
    let first = harness
        .worker
        .submit(harness.refresh(), Lane::Background, None, None);
    held.wait_reached();
    let background = harness.worker.submit(
        Intent::IndexRebuild {
            root: harness.root.clone(),
        },
        Lane::Background,
        None,
        None,
    );
    let user = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    held.release();

    let seen = harness.until_end_of(background);
    assert_eq!(started(&seen), [first, user, background]);
    // The running job was not preempted: it finished before the user's
    // job started.
    let first_finished = position(
        &seen,
        |event| matches!(event, Seen::Finished(id, _) if *id == first),
    );
    let user_started = position(
        &seen,
        |event| matches!(event, Seen::Started(id) if *id == user),
    );
    assert!(first_finished < user_started);
    for id in [first, user, background] {
        assert_eq!(finished(&seen, id).code, ResultCode::Ok);
    }
}

#[test]
fn a_refresh_through_the_worker_succeeds_with_its_progress_inside_the_root() {
    let mut harness = Harness::new();
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let seen = harness.until_end_of(id);

    let envelope = finished(&seen, id);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.code, ResultCode::Ok);
    assert!(matches!(
        harness.state(id),
        RequestState::Finished(Outcome::Success, ResultCode::Ok, ..)
    ));

    let started_at = position(&seen, |event| matches!(event, Seen::Started(_)));
    let finished_at = position(&seen, |event| matches!(event, Seen::Finished(..)));
    let progress: Vec<(usize, &ProgressEvent)> = seen
        .iter()
        .enumerate()
        .filter_map(|(at, event)| match event {
            Seen::Progress(of, progress) if *of == id => Some((at, progress)),
            _ => None,
        })
        .collect();
    assert!(!progress.is_empty(), "a refresh reports its progress");
    // In order: accepted first, once, and all between start and finish.
    assert_eq!(progress[0].1.stage, ProgressStage::Accepted);
    assert_eq!(
        progress
            .iter()
            .filter(|(_, event)| event.stage == ProgressStage::Accepted)
            .count(),
        1
    );
    for (at, event) in &progress {
        assert!(started_at < *at && *at < finished_at);
        assert_eq!(Some(&event.request_id), envelope.request_id.as_ref());
        assert_eq!(event.command, "index refresh");
        let scope = &event.scope;
        for path in [&scope.repository, &scope.worktree].into_iter().flatten() {
            assert!(within(path, &harness.root), "a path outside the root");
        }
    }
}

#[test]
fn cancelling_a_queued_job_removes_it_before_the_library_sees_it() {
    let mut harness = Harness::new();
    let held = harness.backend.hold_job(JobKind::IndexRefresh);
    let first = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    held.wait_reached();
    let queued = harness.worker.submit(
        Intent::IndexRebuild {
            root: harness.root.clone(),
        },
        Lane::User,
        None,
        None,
    );
    assert_eq!(harness.worker.cancel(queued), CancelResult::Removed);
    // A second cancel finds nothing to stop.
    assert_eq!(harness.worker.cancel(queued), CancelResult::TooLate);
    held.release();

    let mut seen = harness.until_end_of(first);
    // The worker goes on to the next job; the removed one never ran.
    let after = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    seen.extend(harness.until_end_of(after));
    assert_eq!(started(&seen), [first, after]);
    assert!(
        seen.iter()
            .any(|event| matches!(event, Seen::Removed(id) if *id == queued))
    );
    assert!(!harness.backend.jobs_seen().contains(&JobKind::IndexRebuild));
    assert!(matches!(harness.state(queued), RequestState::Removed));
}

#[test]
fn cancelling_before_acceptance_ends_cancelled_with_no_pending_row() {
    let mut harness = Harness::new();
    let pending_before = harness.pending_operations();
    let held = harness.backend.hold_job(JobKind::IndexRefresh);
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    held.wait_reached();
    assert_eq!(harness.worker.cancel(id), CancelResult::Signalled);
    held.release();

    let seen = harness.until_end_of(id);
    let envelope = finished(&seen, id);
    assert_eq!(envelope.code, ResultCode::Cancelled);
    assert_eq!(envelope.outcome, Outcome::Cancelled);
    assert!(
        !seen.iter().any(
            |event| matches!(event, Seen::Progress(_, p) if p.stage == ProgressStage::Accepted)
        )
    );
    assert_eq!(harness.pending_operations(), pending_before);
    assert_eq!(pending_before, 0);
}

#[test]
fn cancelling_after_acceptance_is_too_late_and_shows_accepted() {
    let mut harness = Harness::new();
    let (reached_tx, reached) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    // Runs on the worker thread, after the refresh was accepted and
    // before it persists.
    harness
        .fixture
        .service()
        .set_refresh_persist_hook_for_testing(move || {
            reached_tx.send(()).unwrap();
            let _ = released.recv_timeout(WAIT);
        });
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    reached
        .recv_timeout(WAIT)
        .expect("the refresh reached its persist");
    assert_eq!(harness.worker.cancel(id), CancelResult::TooLate);

    // Everything sent before the hold, applied: the request is accepted.
    loop {
        match harness.next() {
            Seen::Progress(of, event) if of == id && event.stage == ProgressStage::Accepted => {
                break;
            }
            Seen::Finished(..) => panic!("finished while held"),
            _ => {}
        }
    }
    assert!(matches!(
        harness.state(id),
        RequestState::Running { accepted: true, .. }
    ));

    release.send(()).unwrap();
    let seen = harness.until_end_of(id);
    assert_eq!(finished(&seen, id).code, ResultCode::Ok);
}

#[test]
fn a_retry_uses_the_same_request_id_and_is_answered_from_its_record() {
    let mut harness = Harness::new();
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let seen = harness.until_end_of(id);
    let Seen::Queued(request) = &seen[0] else {
        panic!("a submitted request is queued first");
    };
    let request_id = request.request_id;
    let first = finished(&seen, id).clone();
    assert_eq!(first.request_id, Some(request_id.to_string()));
    let refreshed_at = harness
        .fixture
        .service()
        .index_status(&harness.resolved())
        .unwrap()
        .refreshed_at;

    // A second refresh would persist again and run the hook.
    let persisted = Arc::new(AtomicBool::new(false));
    let flag = persisted.clone();
    harness
        .fixture
        .service()
        .set_refresh_persist_hook_for_testing(move || flag.store(true, Ordering::SeqCst));
    harness.worker.retry(id, None);
    let seen = harness.until_end_of(id);
    let Seen::Queued(retried) = &seen[0] else {
        panic!("a retried request is queued again");
    };
    assert_eq!(retried.id, id);
    assert_eq!(retried.request_id, request_id);
    let second = finished(&seen, id);
    assert_eq!(second.request_id, first.request_id);
    assert_eq!(second.code, ResultCode::Ok);
    assert_eq!(
        harness.operations.request(id).unwrap().request_id,
        request_id
    );

    assert!(!persisted.load(Ordering::SeqCst), "no second refresh");
    let now = harness
        .fixture
        .service()
        .index_status(&harness.resolved())
        .unwrap()
        .refreshed_at;
    assert_eq!(now, refreshed_at);
}

#[test]
fn a_protected_key_generation_without_a_secret_never_reaches_the_library() {
    let mut harness = Harness::new();
    let id = harness.worker.submit(
        Intent::KeyGenerate {
            label: "laptop".to_owned(),
            protected: true,
        },
        Lane::User,
        None,
        None,
    );
    let seen = harness.until_end_of(id);
    assert!(
        seen.iter()
            .any(|event| matches!(event, Seen::NotSent(of, NotSent::SecretRequired) if *of == id))
    );
    assert!(started(&seen).is_empty());
    assert!(matches!(
        harness.state(id),
        RequestState::NotSent(NotSent::SecretRequired)
    ));
    assert!(!harness.backend.jobs_seen().contains(&JobKind::KeyGenerate));
    let keys = harness.fixture.service().list_keys().unwrap();
    assert!(keys.items.is_empty());
    assert!(!harness.fixture.home().join(".ssh").exists());
}

#[test]
fn quit_with_no_job_is_idle_and_admits_nothing_more() {
    let mut harness = Harness::new();
    let before = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    harness.until_end_of(before);

    assert!(matches!(harness.worker.quit(), QuitState::Idle));
    let late = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let seen = harness.until_end_of(late);
    assert!(started(&seen).is_empty());
    assert!(matches!(harness.state(late), RequestState::Removed));
}

/// A retry after quitting is ignored: the finished request keeps its
/// result rather than becoming Removed.
#[test]
fn a_retry_after_quitting_leaves_the_finished_request_alone() {
    let mut harness = Harness::new();
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    harness.until_end_of(id);
    assert!(matches!(harness.state(id), RequestState::Finished(..)));

    assert!(matches!(harness.worker.quit(), QuitState::Idle));
    harness.worker.retry(id, None);
    // A retry sends its events before it returns, so none is coming.
    assert!(matches!(
        harness.events.try_recv(),
        Err(TryRecvError::Empty)
    ));
    assert!(matches!(harness.state(id), RequestState::Finished(..)));
}

#[test]
fn quit_while_a_job_is_held_is_finishing_then_idle() {
    let mut harness = Harness::new();
    let held = harness.backend.hold_job(JobKind::IndexRefresh);
    let running = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    held.wait_reached();
    let queued = harness
        .worker
        .submit(harness.refresh(), Lane::Background, None, None);

    let QuitState::Finishing(finishing) = harness.worker.quit() else {
        panic!("a held job is finishing");
    };
    assert_eq!(finishing, running);
    // Nothing more is admitted.
    let late = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    assert!(matches!(harness.worker.quit(), QuitState::Finishing(id) if id == running));

    held.release();
    let seen = harness.until_end_of(running);
    // The running job's token was cancelled before it was accepted.
    assert_eq!(finished(&seen, running).code, ResultCode::Cancelled);
    assert_eq!(started(&seen), [running]);
    for id in [queued, late] {
        assert!(
            seen.iter()
                .any(|event| matches!(event, Seen::Removed(of) if *of == id))
        );
    }
    assert!(matches!(harness.worker.quit(), QuitState::Idle));
}

/// The observation runs on the worker, under its session. The fixture
/// has no remote, so the library answers before any connection.
#[test]
fn a_host_observation_runs_on_the_worker_and_connects_nowhere() {
    let mut harness = Harness::new();
    let id = harness.worker.observe_host(harness.root.clone(), None);
    let seen = harness.until_end_of(id);
    assert_eq!(started(&seen), [id]);
    let (code, repository) = seen
        .iter()
        .find_map(|event| match event {
            Seen::HostObserved(of, code, repository) if *of == id => {
                Some((*code, repository.clone()))
            }
            _ => None,
        })
        .unwrap();
    assert_ne!(code, ResultCode::Ok);
    assert_eq!(
        repository.as_deref().map(Path::new),
        Some(harness.root.as_path())
    );
    assert_eq!(harness.backend.jobs_seen(), [JobKind::ObserveHost]);
    // An observation has no token: it cannot be cancelled.
    assert_eq!(harness.worker.cancel(id), CancelResult::TooLate);
}

/// A preview prepared again goes with the same request: the request is
/// queued once more under its own request ID with the new confirmation,
/// which the library accepts as a first call, since the stale attempt
/// left no record.
#[test]
fn a_request_confirmed_again_keeps_its_request_id_and_runs_with_the_new_confirmation() {
    let mut harness = Harness::new();
    let intent = Intent::RepoRemove {
        root: harness.root.clone(),
    };
    let stale = ConfirmationId::new();
    let id = harness
        .worker
        .submit(intent.clone(), Lane::User, Some(stale), None);
    let seen = harness.until_end_of(id);
    assert_eq!(
        finished(&seen, id).code,
        ResultCode::ConfirmationNotFound,
        "{}",
        finished(&seen, id).message
    );
    let request_id = harness.operations.request(id).unwrap().request_id;

    let prepared = harness
        .fixture
        .service()
        .prepare(&intent.build(None, || unreachable!()).unwrap());
    let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
        panic!("repo remove gave no preview: {}", prepared.message);
    };
    let fresh = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    assert_eq!(harness.worker.reconfirm(id, fresh), Some(2));
    let seen = harness.until_end_of(id);
    let Seen::Queued(queued) = &seen[0] else {
        panic!("queued again first");
    };
    assert_eq!(queued.id, id);
    assert_eq!(queued.request_id, request_id);
    assert_eq!(queued.confirmation, Some(fresh));
    assert_eq!(queued.attempt, 2);
    assert_eq!(finished(&seen, id).code, ResultCode::Ok);
    assert_eq!(
        finished(&seen, id).request_id.as_deref(),
        Some(request_id.to_string().as_str())
    );
    // A request that is not this worker's is left alone.
    assert_eq!(harness.worker.reconfirm(DesktopRequestId(999), fresh), None);
}

// Ruling R-T7a: a background job answered `busy` is taken again after
// 100, 200 and 400 ms, behind user jobs and with no new `Queued` event.

/// Holds `root`'s repository lease in this process, as another call
/// would: a refresh of it answers `busy`.
fn hold_lease(harness: &Harness, root: &Path) -> manyhands::repository::LeaseHolderForTesting {
    manyhands::repository::RepositoryService::hold_lease_for_testing(
        root,
        harness.fixture.data_dir(),
        manyhands::repository::LeaseKind::Repository,
    )
    .unwrap()
}

fn count(seen: &[Seen], pick: impl Fn(&Seen) -> bool) -> usize {
    seen.iter().filter(|event| pick(event)).count()
}

/// Waits until the worker has a job backing off after `busy`.
fn until_backing_off(harness: &mut Harness, seen: &mut Vec<Seen>) {
    let deadline = Instant::now() + WAIT;
    while harness.worker.backing_off() == 0 {
        assert!(Instant::now() < deadline, "no job backed off");
        if let Ok(event) = harness.events.try_recv() {
            seen.push(self::seen(&event));
            harness.operations.apply(event);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn a_busy_background_job_is_taken_again_and_finishes_once_the_lease_is_free() {
    let mut harness = Harness::new();
    let lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::Background, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    // Free before the second retry is due.
    drop(lease);
    seen.extend(harness.until_end_of(id));
    assert_eq!(finished(&seen, id).code, ResultCode::Ok);
    assert!(count(&seen, |event| matches!(event, Seen::Started(_))) >= 2);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Queued(_))), 1);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Finished(..))), 1);
}

#[test]
fn a_background_job_busy_through_every_retry_reports_busy_once() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::Background, None, None);
    let seen = harness.until_end_of(id);
    assert_eq!(finished(&seen, id).code, ResultCode::Busy);
    // The first attempt and three retries.
    assert_eq!(count(&seen, |event| matches!(event, Seen::Started(_))), 4);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Queued(_))), 1);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Finished(..))), 1);
    assert_eq!(harness.backend.jobs_seen().len(), 4);
}

// Ruling R-B3: a user job answered `busy` is taken again the same way,
// ahead of every background job, unless its attempt carried a secret.

#[test]
fn a_busy_user_job_is_taken_again_and_finishes_once_the_lease_is_free() {
    let mut harness = Harness::new();
    let lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    // Free before the second retry is due.
    drop(lease);
    seen.extend(harness.until_end_of(id));
    assert_eq!(finished(&seen, id).code, ResultCode::Ok);
    assert!(count(&seen, |event| matches!(event, Seen::Started(_))) >= 2);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Queued(_))), 1);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Finished(..))), 1);
}

#[test]
fn a_user_job_busy_through_every_retry_reports_busy_once() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let seen = harness.until_end_of(id);
    assert_eq!(finished(&seen, id).code, ResultCode::Busy);
    // The first attempt and three retries.
    assert_eq!(count(&seen, |event| matches!(event, Seen::Started(_))), 4);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Queued(_))), 1);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Finished(..))), 1);
}

/// An attempt built with a secret gave it to its `Mutation`, so nothing
/// is left to build another from: it reports `busy` at once.
#[test]
fn a_user_job_with_a_secret_reports_busy_at_once() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, Some(secret));
    let seen = harness.until_end_of(id);
    assert_eq!(finished(&seen, id).code, ResultCode::Busy);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Started(_))), 1);
    assert_eq!(harness.worker.backing_off(), 0);
}

/// A user job backing off after `busy` keeps the user lane's place: a
/// background job submitted meanwhile, for a repository that is free,
/// waits until the user job has finished.
#[test]
fn a_backing_off_user_job_still_runs_before_a_background_job() {
    let mut harness = Harness::new();
    let other = harness.fixture.create_repository("b");
    let lease = hold_lease(&harness, &harness.root.clone());
    let user = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    let background = harness.worker.submit(
        Intent::IndexRefresh { root: other },
        Lane::Background,
        None,
        None,
    );
    // Long enough for the background job to have run, were it taken.
    std::thread::sleep(Duration::from_millis(300));
    drop(lease);
    seen.extend(harness.until_end_of(user));
    seen.extend(harness.until_end_of(background));
    let user_finished = position(
        &seen,
        |event| matches!(event, Seen::Finished(id, _) if *id == user),
    );
    let background_started = position(
        &seen,
        |event| matches!(event, Seen::Started(id) if *id == background),
    );
    assert!(user_finished < background_started);
    assert_eq!(finished(&seen, user).code, ResultCode::Ok);
    assert_eq!(finished(&seen, background).code, ResultCode::Ok);
}

#[test]
fn cancelling_a_user_job_during_its_backoff_removes_it() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    assert_eq!(harness.worker.cancel(id), CancelResult::Removed);
    seen.extend(harness.until_end_of(id));
    assert_eq!(count(&seen, |event| matches!(event, Seen::Finished(..))), 0);
    assert!(matches!(harness.state(id), RequestState::Removed));
    assert_eq!(harness.worker.backing_off(), 0);
}

#[test]
fn quitting_during_a_user_backoff_removes_the_job() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    assert_eq!(harness.worker.quit(), QuitState::Idle);
    seen.extend(harness.until_end_of(id));
    assert!(matches!(harness.state(id), RequestState::Removed));
}

#[test]
fn a_user_job_submitted_during_the_backoff_runs_first() {
    let mut harness = Harness::new();
    let other = harness.fixture.create_repository("b");
    let lease = hold_lease(&harness, &harness.root.clone());
    let background = harness
        .worker
        .submit(harness.refresh(), Lane::Background, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    // The user job is held at its start, so the backing-off job is due
    // meanwhile and must still wait behind it.
    let held = harness.backend.hold_job(JobKind::IndexRefresh);
    let user = harness
        .worker
        .submit(Intent::IndexRefresh { root: other }, Lane::User, None, None);
    held.wait_reached();
    std::thread::sleep(Duration::from_millis(500));
    drop(lease);
    held.release();
    seen.extend(harness.until_end_of(user));
    seen.extend(harness.until_end_of(background));
    let user_started = position(
        &seen,
        |event| matches!(event, Seen::Started(id) if *id == user),
    );
    let retried = seen
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event, Seen::Started(id) if *id == background))
        .nth(1)
        .map(|(at, _)| at)
        .expect("the background job was taken again");
    assert!(user_started < retried);
    assert_eq!(finished(&seen, user).code, ResultCode::Ok);
    assert_eq!(finished(&seen, background).code, ResultCode::Ok);
}

#[test]
fn cancelling_a_job_during_its_backoff_removes_it() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::Background, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    assert_eq!(harness.worker.cancel(id), CancelResult::Removed);
    seen.extend(harness.until_end_of(id));
    assert!(
        seen.iter()
            .any(|event| matches!(event, Seen::Removed(removed) if *removed == id))
    );
    assert_eq!(count(&seen, |event| matches!(event, Seen::Finished(..))), 0);
    assert!(matches!(harness.state(id), RequestState::Removed));
    assert_eq!(harness.worker.backing_off(), 0);
}

#[test]
fn quitting_during_a_backoff_removes_the_job() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::Background, None, None);
    let mut seen = Vec::new();
    until_backing_off(&mut harness, &mut seen);
    assert_eq!(harness.worker.quit(), QuitState::Idle);
    seen.extend(harness.until_end_of(id));
    assert!(matches!(harness.state(id), RequestState::Removed));
}

/// Quitting cancels the running job's token whether or not it has been
/// accepted. A job answered `busy` after that is not taken again, nor
/// reported removed as if it never ran: it finishes with its answer.
#[test]
fn a_job_answered_busy_after_quit_finishes_with_its_answer() {
    let mut harness = Harness::new();
    let _lease = hold_lease(&harness, &harness.root.clone());
    let returned = harness.backend.hold_job_return(JobKind::IndexRefresh);
    let id = harness
        .worker
        .submit(harness.refresh(), Lane::User, None, None);
    returned.wait_reached();
    assert_eq!(harness.worker.quit(), QuitState::Finishing(id));
    returned.release();

    let seen = harness.until_end_of(id);
    assert_eq!(finished(&seen, id).code, ResultCode::Busy);
    assert_eq!(count(&seen, |event| matches!(event, Seen::Started(_))), 1);
    assert!(
        !seen
            .iter()
            .any(|event| matches!(event, Seen::Removed(removed) if *removed == id))
    );
    assert_eq!(harness.worker.quit(), QuitState::Idle);
}
