//! The mutation worker: one desktop-owned thread that runs every
//! mutation and owns the process's only `SessionCredentials`.
//!
//! Jobs wait in two lanes. The worker always takes the next user job
//! before any background job, and never preempts a running one. The queue
//! is shared state behind a lock, so the UI can remove a queued job and
//! learn at once whether cancelling reached it. The worker tells the UI
//! what happened over an unbounded `futures` channel, which a foreground
//! task drains into the request model.
//!
//! A job answered `busy` is taken again after 100, 200 and 400 ms, in its
//! own lane and with no new event (rulings R-T7a and R-B3). A user job
//! waiting out that backoff keeps the head of the user lane, so no job is
//! taken before it. An attempt that carried a secret reports `busy` at
//! once: the secret went into its `Mutation` and is not kept for another.
//!
//! Cancellation follows the library: a queued job is removed and never
//! runs; a running job's token is cancelled, which every command honors
//! until it is accepted. After acceptance a command that stops at safe
//! points (a comment, a share, `operation resume`) still has its token
//! cancelled and stops at the next one (R-1); a local command cannot be
//! stopped, and a host observation has no token at all. Any progress
//! stage marks a job accepted, since a same-ID re-entry never reports
//! `Accepted` (R-14).
//!
//! After each finished attempt of a command that stops at safe points,
//! the worker asks `show_request` where its record stands and keeps the
//! answer (`record_state`) before the UI hears of the attempt. Give up
//! (decision 8) is a job of its own over the same request: an accepted
//! record is cancelled through `cancel_request` and the same request sent
//! again, whose answer is the attempt's.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError},
    thread,
    time::Instant,
};

use futures::channel::mpsc;
use manyhands::{
    repository::{
        CancellationToken, ConfirmationId, HostObservationDto, MutationCall, MutationDataDto,
        ProgressEvent, ProgressSink, RepositoryService, RequestId, RequestState as RecordDtoState,
        keys::{SecretPassphrase, SessionCredentials},
        transport::SshAuthority,
    },
    results::{Envelope, ResultCode},
};

use super::{
    BUSY_RETRY_DELAYS, Backend, JobKind,
    prompts::{DeclinedKeys, DesktopProvider, PromptRequest},
    requests::{DesktopRequest, DesktopRequestId, Intent, Lane, Refusals, RequestState},
};

pub(crate) use super::requests::NotSent;

/// A finished job's envelope, boxed: it is large and moves through a
/// channel.
pub(crate) type FinishedEnvelope = Box<Envelope<MutationDataDto>>;

/// What the worker tells the UI.
pub(crate) enum WorkerEvent {
    /// A request was submitted, or submitted again for a retry under the
    /// same ID.
    Queued(DesktopRequest),
    Started(DesktopRequestId),
    Progress(DesktopRequestId, ProgressEvent),
    Finished(DesktopRequestId, FinishedEnvelope),
    HostObserved(DesktopRequestId, Envelope<HostObservationDto>),
    /// A queued job was removed before it ran: cancelled, or emptied from
    /// the queue on quit.
    Removed(DesktopRequestId),
    /// The job could not be built, and nothing was asked of the library.
    NotSent(DesktopRequestId, NotSent),
    /// The running job's library call asks for a passphrase, and the
    /// worker waits for the reply (`prompts`).
    Prompt(PromptRequest),
}

/// What cancelling a request did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CancelResult {
    /// It was queued, and was removed: it never runs.
    Removed,
    /// It is running, and not yet accepted or accepted by a command that
    /// stops at safe points: its token is cancelled (R-1).
    Signalled,
    /// It is an accepted local command, has no token, or is no longer
    /// queued or running: nothing can stop it now.
    TooLate,
}

/// Where a request's library record stood when its last attempt
/// finished, as `show_request` answered (Give up, decision 8).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecordState {
    /// The request is still open: the same request takes it up again.
    Accepted,
    /// Its result is stored, and every retry replays it. A request with
    /// no record (`show_request` answered `request_not_found`), refused
    /// before it was accepted, stands here too: nothing of it is open. So
    /// this state never tells "no record" apart: P16's check that no
    /// record exists calls `show_request` itself (the composer's Check
    /// again, `composer/place/check.rs`, through
    /// `publication::record::check_again`).
    Finished,
    /// `show_request` failed: where the record stands is not known (P17).
    Unknown,
}

/// What a Give up found.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GaveUp {
    /// The record was accepted, or could not be read: the library was
    /// asked to cancel it, and the same request was sent again. Its answer
    /// is the attempt's, whatever it is (fact 14d).
    Resent,
    /// The record was already finished: nothing was cancelled, and the
    /// attempt replays its stored answer.
    AlreadyFinished,
}

/// The running job, as a test reads it.
#[cfg(test)]
pub(crate) struct RunningJob {
    pub id: DesktopRequestId,
    pub accepted: bool,
    /// Whether its token is cancelled.
    pub cancelled: bool,
}

/// Whether quitting can go ahead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum QuitState {
    Idle,
    /// This job is still running; quit when it returns.
    Finishing(DesktopRequestId),
}

/// What a job does.
enum Work {
    Execute {
        request_id: RequestId,
        /// Boxed: a ticket save's fields make an intent large.
        intent: Box<Intent>,
        confirmation: Option<ConfirmationId>,
        secret: Option<SecretPassphrase>,
        cancel: CancellationToken,
    },
    /// Give up (decision 8): where the request's record stands decides
    /// what is asked before the same request is sent again.
    GiveUp {
        request_id: RequestId,
        intent: Box<Intent>,
        confirmation: Option<ConfirmationId>,
        cancel: CancellationToken,
    },
    ObserveHost {
        root: PathBuf,
        authority: Option<SshAuthority>,
    },
}

struct Job {
    id: DesktopRequestId,
    work: Work,
    lane: Lane,
    /// How many times a job answered `busy` has been admitted again.
    busy_retries: usize,
    /// A job admitted again after `busy` is not taken before this.
    not_before: Option<Instant>,
}

impl Job {
    fn new(id: DesktopRequestId, work: Work, lane: Lane) -> Self {
        Self {
            id,
            work,
            lane,
            busy_retries: 0,
            not_before: None,
        }
    }
}

/// What a retry rebuilds the job from.
struct Submitted {
    request_id: RequestId,
    intent: Intent,
    confirmation: Option<ConfirmationId>,
    lane: Lane,
    /// The attempt the last `Queued` event announced.
    attempt: u32,
    /// When that attempt was queued, in the order of every attempt (R-78).
    sent: SendMark,
    /// What its attempts were answered, for R-78's exemption.
    answers: Refusals,
    /// Where its record stood after its last finished attempt, for a
    /// command that stops at safe points.
    record: Option<RecordState>,
    /// What its last Give up found, until a retry.
    gave_up: Option<GaveUp>,
}

/// Where an attempt stands in the order every attempt was queued in: a
/// request sent again moves on (R-78).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct SendMark(u64);

/// The job the worker is running.
struct Running {
    id: DesktopRequestId,
    /// `None` for a host observation, which cannot be cancelled.
    cancel: Option<CancellationToken>,
    accepted: bool,
    /// Whether its command honours the token after acceptance (R-1).
    stops_at_safe_points: bool,
}

#[derive(Default)]
struct State {
    next_id: u64,
    user: VecDeque<Job>,
    background: VecDeque<Job>,
    running: Option<Running>,
    /// Set by `quit`: nothing more is admitted.
    closed: bool,
    /// Set when the last handle is dropped: the thread ends once the
    /// running job returns.
    shut_down: bool,
    submitted: HashMap<DesktopRequestId, Submitted>,
    /// How many attempts have been queued.
    sends: u64,
}

impl State {
    /// The mark of an attempt queued now.
    fn next_send(&mut self) -> SendMark {
        self.sends += 1;
        SendMark(self.sends)
    }

    fn queued(&self, id: DesktopRequestId) -> bool {
        self.user
            .iter()
            .chain(&self.background)
            .any(|job| job.id == id)
    }

    fn remove_queued(&mut self, id: DesktopRequestId) -> bool {
        for lane in [&mut self.user, &mut self.background] {
            if let Some(at) = lane.iter().position(|job| job.id == id) {
                lane.remove(at);
                return true;
            }
        }
        false
    }
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    events: mpsc::UnboundedSender<WorkerEvent>,
    /// The keys declined with Not now, shared with the provider (R-13).
    declined: DeclinedKeys,
}

impl Shared {
    /// The state. A panic elsewhere while it was held leaves nothing
    /// half-written that matters here, so a poisoned lock is taken.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sends to the UI. A UI that is gone wants nothing more.
    fn send(&self, event: WorkerEvent) {
        let _ = self.events.unbounded_send(event);
    }
}

/// Ends the worker when the last handle goes.
struct Owner(Arc<Shared>);

impl Drop for Owner {
    fn drop(&mut self) {
        self.0.lock().shut_down = true;
        self.0.changed.notify_all();
        // Closed here, on the UI's thread, so the worker's own sender,
        // dropped later on its thread, wakes nothing: a test scheduler
        // that forbids parking refuses a wake from another thread.
        self.0.events.close_channel();
    }
}

/// The UI's handle on the worker. Cloning it shares the worker.
#[derive(Clone)]
pub(crate) struct WorkerHandle {
    owner: Arc<Owner>,
}

impl WorkerHandle {
    /// Starts the worker thread over `backend`, and gives the receiver of
    /// its events.
    pub(crate) fn spawn(backend: Arc<dyn Backend>) -> (Self, mpsc::UnboundedReceiver<WorkerEvent>) {
        let (events, receiver) = mpsc::unbounded();
        let shared = Arc::new(Shared {
            state: Mutex::default(),
            changed: Condvar::new(),
            events,
            declined: DeclinedKeys::default(),
        });
        let worker = shared.clone();
        thread::Builder::new()
            .name("manyhands-worker".to_owned())
            .spawn(move || run(&worker, backend.as_ref()))
            .expect("failed to start the Manyhands worker thread");
        let handle = Self {
            owner: Arc::new(Owner(shared)),
        };
        (handle, receiver)
    }

    fn shared(&self) -> &Shared {
        &self.owner.0
    }

    /// Queues `intent` under a new request ID. `secret` is the passphrase
    /// of a protected key generation; it is given to this attempt only.
    pub(crate) fn submit(
        &self,
        intent: Intent,
        lane: Lane,
        confirmation: Option<ConfirmationId>,
        secret: Option<SecretPassphrase>,
    ) -> DesktopRequestId {
        submit(
            self.shared(),
            intent,
            lane,
            confirmation,
            secret,
            RequestId::new(),
        )
    }

    /// Queues `intent` under the caller's library request ID: a post's is
    /// its comment ID, so the first post after a relaunch is the same
    /// request (R-16).
    pub(crate) fn submit_with_request_id(
        &self,
        intent: Intent,
        lane: Lane,
        request_id: RequestId,
    ) -> DesktopRequestId {
        submit(self.shared(), intent, lane, None, None, request_id)
    }

    /// Queues the request `id` again, under its own request ID, so the
    /// library answers a finished one from its record. A request that is
    /// queued or running, or that this worker never took, is left alone,
    /// and so is every request once quitting has closed the queue: its
    /// last result stands. So is a context request that a later attempt of
    /// its item would be undone by (R-78, `Intent::kept_from_again_by`): a
    /// release once a prepare of the item was sent after it, a prepare once
    /// a release was, unless the library refused that release, touching
    /// nothing (owner, 2026-10-28). "After" is the order attempts were
    /// queued in. True when it was queued again.
    pub(crate) fn retry(&self, id: DesktopRequestId, secret: Option<SecretPassphrase>) -> bool {
        self.requeue(id, None, secret, false).is_some()
    }

    /// Gives up the request `id` (decision 8): queues a job, on its lane
    /// and under its own request ID, that asks where its record stands; an
    /// accepted one is cancelled through `cancel_request` and the same
    /// request is sent again, and the `Finished` it ends with is that
    /// request's answer. A finished one is only replayed, and `gave_up`
    /// says it was already finished. Refused, as a retry is, while the
    /// request is queued or running, its Give up or its retry, and for a
    /// command that cannot be given up (`Intent::can_be_given_up`). True
    /// when queued.
    pub(crate) fn give_up(&self, id: DesktopRequestId) -> bool {
        self.requeue(id, None, None, true).is_some()
    }

    /// Where the request `id`'s record stood after its last finished
    /// attempt; `None` for a command that does not stop at safe points,
    /// or before its first attempt finished. Kept before the UI hears of
    /// the attempt, so a `Finished` event's reader finds it.
    pub(crate) fn record_state(&self, id: DesktopRequestId) -> Option<RecordState> {
        self.shared().lock().submitted.get(&id)?.record
    }

    /// What the request `id`'s last Give up found, until a retry.
    pub(crate) fn gave_up(&self, id: DesktopRequestId) -> Option<GaveUp> {
        self.shared().lock().submitted.get(&id)?.gave_up
    }

    /// Unlock…: clears the Not now mark of `key_id`, so its next request
    /// prompts again (R-3, R-13). True when it was declined. Unlock… uses
    /// `clear_declined_for`; this per-key form serves the tests.
    #[cfg(test)]
    pub(crate) fn clear_declined(&self, key_id: manyhands::repository::SharedKeyId) -> bool {
        self.shared().declined.clear(key_id)
    }

    /// Unlock… on the outcome of the request `id`, before it is retried,
    /// and Share after a finished `NeedsUnlock`, before the new request:
    /// clears the Not now marks of the keys `id`'s last attempt was
    /// answered `Cancelled` for, so the next request prompts for them
    /// again (R-3, R-13, R-17). True when any was still declined.
    pub(crate) fn clear_declined_for(&self, id: DesktopRequestId) -> bool {
        self.shared().declined.clear_for(id)
    }

    /// Queues the request `id` again, under its own request ID, with the
    /// confirmation of a preview prepared again: the attempt that met a
    /// stale confirmation left no record, so the library takes this one
    /// as the request's first call. Gives the attempt the new `Queued`
    /// event announces; `None`, and nothing queued, when a retry would be
    /// left alone.
    pub(crate) fn reconfirm(
        &self,
        id: DesktopRequestId,
        confirmation: ConfirmationId,
    ) -> Option<u32> {
        self.requeue(id, Some(confirmation), None, false)
    }

    fn requeue(
        &self,
        id: DesktopRequestId,
        confirmation: Option<ConfirmationId>,
        secret: Option<SecretPassphrase>,
        give_up: bool,
    ) -> Option<u32> {
        let shared = self.shared();
        let mut state = shared.lock();
        if state.closed
            || state.queued(id)
            || state
                .running
                .as_ref()
                .is_some_and(|running| running.id == id)
        {
            return None;
        }
        // R-78, as the owner narrowed it (2026-10-28): a request is not sent
        // again once a later attempt of its item would be undone by it,
        // unless that later attempt was a release refused with nothing
        // touched (`Intent::kept_from_again_by`).
        let asked = state.submitted.get(&id)?;
        if give_up && !asked.intent.can_be_given_up() {
            return None;
        }
        if state.submitted.values().any(|later| {
            later.sent > asked.sent
                && asked
                    .intent
                    .kept_from_again_by(&later.intent, later.answers.all_refused())
        }) {
            return None;
        }
        let sent = state.next_send();
        let submitted = state.submitted.get_mut(&id)?;
        submitted.attempt += 1;
        submitted.sent = sent;
        submitted.answers.queued();
        submitted.gave_up = None;
        if confirmation.is_some() {
            submitted.confirmation = confirmation;
        }
        let (request_id, intent, confirmation, lane, attempt) = (
            submitted.request_id,
            submitted.intent.clone(),
            submitted.confirmation,
            submitted.lane,
            submitted.attempt,
        );
        shared.send(WorkerEvent::Queued(DesktopRequest {
            id,
            request_id,
            intent: intent.clone(),
            confirmation,
            lane,
            attempt,
            state: RequestState::Queued,
        }));
        let (intent, cancel) = (Box::new(intent), CancellationToken::new());
        let work = if give_up {
            Work::GiveUp {
                request_id,
                intent,
                confirmation,
                cancel,
            }
        } else {
            Work::Execute {
                request_id,
                intent,
                confirmation,
                secret,
                cancel,
            }
        };
        admit(shared, &mut state, Job::new(id, work, lane));
        Some(attempt)
    }

    /// The mark of the newest attempt queued so far: one `sent_since`
    /// counts from.
    pub(crate) fn newest(&self) -> SendMark {
        SendMark(self.shared().lock().sends)
    }

    /// The mark of the request `id`'s last attempt.
    pub(crate) fn sent_mark(&self, id: DesktopRequestId) -> Option<SendMark> {
        Some(self.shared().lock().submitted.get(&id)?.sent)
    }

    /// Whether an attempt queued after `mark`, a request's first or a
    /// retry, asked for what `pick` picks.
    pub(crate) fn sent_since(&self, mark: SendMark, pick: impl Fn(&Intent) -> bool) -> bool {
        self.shared()
            .lock()
            .submitted
            .values()
            .any(|submitted| submitted.sent > mark && pick(&submitted.intent))
    }

    /// Whether a request that asks for what `pick` picks is queued, waiting
    /// out `busy`, or running.
    pub(crate) fn in_flight(&self, pick: impl Fn(&Intent) -> bool) -> bool {
        let state = self.shared().lock();
        let picked = |id: &DesktopRequestId| {
            state
                .submitted
                .get(id)
                .is_some_and(|submitted| pick(&submitted.intent))
        };
        state
            .running
            .as_ref()
            .is_some_and(|running| picked(&running.id))
            || state
                .user
                .iter()
                .chain(&state.background)
                .any(|job| picked(&job.id))
    }

    /// A provider that asks this worker's UI, as the worker's own does,
    /// and shares its declined keys: for a test that raises a prompt from
    /// a plain thread.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn provider(&self) -> DesktopProvider {
        let shared = self.shared();
        DesktopProvider::with_declined(shared.events.clone(), shared.declined.clone())
    }

    /// Marks the request `id`'s attempt running for the declined keys, as
    /// the worker does before it runs one: for a test that raises a prompt
    /// from a plain thread through `provider`.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn attempting_for_testing(&self, id: Option<DesktopRequestId>) {
        self.shared().declined.attempting(id);
    }

    /// The running job, if any.
    #[cfg(test)]
    pub(crate) fn running_job(&self) -> Option<RunningJob> {
        let state = self.shared().lock();
        let running = state.running.as_ref()?;
        Some(RunningJob {
            id: running.id,
            accepted: running.accepted,
            cancelled: running
                .cancel
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled),
        })
    }

    /// Sends `prompt` to the UI over the worker's event channel, as the
    /// provider does, without waiting for its reply: for a test that must
    /// place a prompt in one drain batch with other events.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn send_prompt(&self, prompt: PromptRequest) {
        self.shared().send(WorkerEvent::Prompt(prompt));
    }

    /// How many jobs, of either lane, wait to be taken again after `busy`.
    #[cfg(test)]
    pub(crate) fn backing_off(&self) -> usize {
        let state = self.shared().lock();
        state
            .user
            .iter()
            .chain(&state.background)
            .filter(|job| job.not_before.is_some())
            .count()
    }

    /// Stops the request `id` if it still can be.
    pub(crate) fn cancel(&self, id: DesktopRequestId) -> CancelResult {
        let shared = self.shared();
        let mut state = shared.lock();
        if state.remove_queued(id) {
            shared.send(WorkerEvent::Removed(id));
            return CancelResult::Removed;
        }
        match &state.running {
            Some(Running {
                id: running,
                cancel: Some(cancel),
                accepted,
                stops_at_safe_points,
            }) if *running == id && (!accepted || *stops_at_safe_points) => {
                cancel.cancel();
                CancelResult::Signalled
            }
            _ => CancelResult::TooLate,
        }
    }

    /// Asks what the host `authority` of the repository at `root`
    /// presents, on the worker, under its session. It cannot be
    /// cancelled.
    pub(crate) fn observe_host(
        &self,
        root: PathBuf,
        authority: Option<SshAuthority>,
    ) -> DesktopRequestId {
        let shared = self.shared();
        let mut state = shared.lock();
        let id = next_id(&mut state);
        let work = Work::ObserveHost { root, authority };
        admit(shared, &mut state, Job::new(id, work, Lane::User));
        id
    }

    /// Whether quitting has begun: no job is admitted any more.
    pub(crate) fn quitting(&self) -> bool {
        self.shared().lock().closed
    }

    /// Stops admitting jobs, empties the queue and cancels the running
    /// job's token. Quitting may go ahead when no job is running;
    /// otherwise once that job returns.
    pub(crate) fn quit(&self) -> QuitState {
        let shared = self.shared();
        let mut guard = shared.lock();
        let state = &mut *guard;
        state.closed = true;
        let removed: Vec<DesktopRequestId> = state
            .user
            .drain(..)
            .chain(state.background.drain(..))
            .map(|job| job.id)
            .collect();
        for id in removed {
            shared.send(WorkerEvent::Removed(id));
        }
        match &state.running {
            Some(running) => {
                if let Some(cancel) = &running.cancel {
                    cancel.cancel();
                }
                QuitState::Finishing(running.id)
            }
            None => QuitState::Idle,
        }
    }
}

/// Queues `intent` under `request_id`: `WorkerHandle::submit` and
/// `submit_with_request_id`, and the worker's own refresh after a
/// repository is created or enabled.
fn submit(
    shared: &Shared,
    intent: Intent,
    lane: Lane,
    confirmation: Option<ConfirmationId>,
    secret: Option<SecretPassphrase>,
    request_id: RequestId,
) -> DesktopRequestId {
    let mut state = shared.lock();
    let id = next_id(&mut state);
    let sent = state.next_send();
    state.submitted.insert(
        id,
        Submitted {
            request_id,
            intent: intent.clone(),
            confirmation,
            lane,
            attempt: 1,
            sent,
            answers: Refusals::default(),
            record: None,
            gave_up: None,
        },
    );
    shared.send(WorkerEvent::Queued(DesktopRequest {
        id,
        request_id,
        intent: intent.clone(),
        confirmation,
        lane,
        attempt: 1,
        state: RequestState::Queued,
    }));
    let work = Work::Execute {
        request_id,
        intent: Box::new(intent),
        confirmation,
        secret,
        cancel: CancellationToken::new(),
    };
    admit(shared, &mut state, Job::new(id, work, lane));
    id
}

fn next_id(state: &mut State) -> DesktopRequestId {
    state.next_id += 1;
    DesktopRequestId(state.next_id)
}

/// Queues `job`, unless quitting has closed the queue: then it is removed
/// at once.
fn admit(shared: &Shared, state: &mut State, job: Job) {
    if state.closed {
        shared.send(WorkerEvent::Removed(job.id));
        return;
    }
    match job.lane {
        Lane::User => state.user.push_back(job),
        Lane::Background => state.background.push_back(job),
    }
    shared.changed.notify_all();
}

/// Queues a job answered `busy` again, unless quitting has closed the
/// queue. A background job goes to the back of its lane; a user job to the
/// front of its own, so it stays ahead of the user jobs submitted after it
/// and of every background job while it waits.
fn readmit(shared: &Shared, state: &mut State, job: Job) {
    if state.closed {
        shared.send(WorkerEvent::Removed(job.id));
        return;
    }
    match job.lane {
        Lane::User => state.user.push_front(job),
        Lane::Background => state.background.push_back(job),
    }
    shared.changed.notify_all();
}

/// The worker thread: takes the next job, user lane first, until the last
/// handle is dropped.
fn run(shared: &Shared, backend: &dyn Backend) {
    // Its provider asks the UI over the channel every other event takes,
    // and shares the declined keys with the handle (R-13).
    let provider = DesktopProvider::with_declined(shared.events.clone(), shared.declined.clone());
    let mut session = SessionCredentials::new(provider);
    while let Some(job) = take(shared) {
        let Job {
            id,
            work,
            lane,
            busy_retries,
            ..
        } = job;
        // A key the provider answers `Cancelled` is recorded against the
        // request whose attempt asked for it (R-13).
        let attempt = matches!(work, Work::Execute { .. } | Work::GiveUp { .. }).then_some(id);
        shared.declined.attempting(attempt);
        match work {
            Work::Execute {
                request_id,
                intent,
                confirmation,
                secret,
                cancel,
            } => {
                let call = Call {
                    id,
                    lane,
                    busy_retries,
                    request_id,
                    intent,
                    confirmation,
                    cancel,
                };
                execute(shared, backend, &mut session, call, secret, false);
            }
            Work::GiveUp {
                request_id,
                intent,
                confirmation,
                cancel,
            } => {
                let call = Call {
                    id,
                    lane,
                    busy_retries,
                    request_id,
                    intent,
                    confirmation,
                    cancel,
                };
                execute(shared, backend, &mut session, call, None, true);
            }
            Work::ObserveHost { root, authority } => {
                shared.send(WorkerEvent::Started(id));
                backend.before_job(JobKind::ObserveHost);
                let envelope =
                    backend
                        .service()
                        .observe_host(&root, authority.as_ref(), &mut session);
                finish(shared);
                shared.send(WorkerEvent::HostObserved(id, envelope));
            }
        }
    }
}

/// One attempt of a request, as the worker runs it.
struct Call {
    id: DesktopRequestId,
    lane: Lane,
    busy_retries: usize,
    request_id: RequestId,
    intent: Box<Intent>,
    confirmation: Option<ConfirmationId>,
    cancel: CancellationToken,
}

/// Runs one attempt of a request and tells the UI what came of it. A Give
/// up first settles what it asks of the record (`give_up_first`).
fn execute(
    shared: &Shared,
    backend: &dyn Backend,
    session: &mut SessionCredentials<DesktopProvider>,
    call: Call,
    secret: Option<SecretPassphrase>,
    give_up: bool,
) {
    let Call {
        id,
        lane,
        busy_retries,
        request_id,
        intent,
        confirmation,
        cancel,
    } = call;
    // The secret goes into this attempt's `Mutation` and is gone with it:
    // an attempt that had one is never retried.
    let had_secret = secret.is_some();
    let mutation = match intent.build(secret, || backend.key_store()) {
        Ok(mutation) => mutation,
        Err(not_built) => {
            finish(shared);
            shared.send(WorkerEvent::NotSent(id, not_built.into()));
            return;
        }
    };
    shared.send(WorkerEvent::Started(id));
    backend.before_job(intent.job_kind());
    if give_up {
        give_up_first(shared, backend.service(), id, request_id, &cancel);
    }
    let mut progress = Forward {
        id,
        shared,
        backend,
        kind: intent.job_kind(),
    };
    let envelope = backend.service().execute(
        MutationCall {
            request_id,
            confirmation,
            mutation,
        },
        session,
        &mut progress,
        &cancel,
    );
    // A test may put a constructed answer in place of the library's.
    let envelope = backend.after_job(intent.job_kind()).unwrap_or(envelope);
    // A job answered `busy` waits and is taken again in its own lane, with
    // no event: the UI sees one request that finishes once. A background
    // job waits behind every user job (ruling R-T7a); a user job keeps the
    // head of the user lane, ahead of every background job (ruling R-B3).
    // One whose token was cancelled meanwhile, by Cancel or by quitting, is
    // not taken again: it ends with its answer, as a cancelled job ends. A
    // Give up's re-send is taken again as a plain one: what it asked of the
    // record is done.
    if envelope.code == ResultCode::Busy
        && !had_secret
        && !cancel.is_cancelled()
        && let Some(delay) = BUSY_RETRY_DELAYS.get(busy_retries)
    {
        let mut state = shared.lock();
        state.running = None;
        let work = Work::Execute {
            request_id,
            intent,
            confirmation,
            secret: None,
            cancel,
        };
        readmit(
            shared,
            &mut state,
            Job {
                id,
                work,
                lane,
                busy_retries: busy_retries + 1,
                not_before: Some(Instant::now() + *delay),
            },
        );
        return;
    }
    // Where the record now stands, for an attempt of a command that stops
    // at safe points (decision 8), and whether it was refused, touching
    // nothing (R-78, as narrowed), are kept before the UI hears of the
    // answer.
    let record = intent
        .stops_at_safe_points()
        .then(|| record_state(backend.service(), request_id));
    let refused = Intent::refused_answer(envelope.outcome, envelope.code, &envelope.recovery);
    if let Some(submitted) = shared.lock().submitted.get_mut(&id) {
        submitted.answers.finished(refused);
        if record.is_some() {
            submitted.record = record;
        }
    }
    // The job is over before the UI hears so, so a quit that hears it
    // finds the worker idle.
    finish(shared);
    // The root as the library registered it, else as asked.
    let onboarded = (envelope.code == ResultCode::Ok)
        .then(|| intent.onboarded_root())
        .flatten()
        .map(|asked| {
            envelope
                .scope
                .repository
                .clone()
                .map_or(asked, PathBuf::from)
        });
    shared.send(WorkerEvent::Finished(id, Box::new(envelope)));
    // A repository just created or enabled is refreshed once, in the
    // background (design, Index Maintenance, decision 4). Quitting refuses
    // it as it refuses every job.
    if let Some(root) = onboarded {
        submit(
            shared,
            Intent::IndexRefresh { root },
            Lane::Background,
            None,
            None,
            RequestId::new(),
        );
    }
}

/// Where the record of `request_id` stands. A request with no record was
/// refused before acceptance: nothing of it is open, so it stands as a
/// finished one.
fn record_state(service: &RepositoryService, request_id: RequestId) -> RecordState {
    let shown = service.show_request(request_id);
    match shown.data {
        Some(request) => match request.state {
            RecordDtoState::Accepted => RecordState::Accepted,
            RecordDtoState::Finished => RecordState::Finished,
        },
        None if shown.code == ResultCode::RequestNotFound => RecordState::Finished,
        None => RecordState::Unknown,
    }
}

/// What a Give up asks of the record before the same request is sent
/// again (decision 8). An accepted record is asked to stop through
/// `cancel_request`, and the re-sent request ends it at its first safe
/// point, or answers otherwise (fact 14d); when that ask fails, the
/// re-send's token is cancelled too, so a Give up never completes the
/// share. A finished one is not touched:
/// the re-send, with its token cancelled, only replays its stored answer,
/// and one with no record stops before it could be accepted. One whose
/// record cannot be read is asked to stop and re-sent with its token
/// cancelled, which ends it whichever it is.
fn give_up_first(
    shared: &Shared,
    service: &RepositoryService,
    id: DesktopRequestId,
    request_id: RequestId,
    cancel: &CancellationToken,
) {
    let gave_up = match record_state(service, request_id) {
        RecordState::Accepted => {
            if service.cancel_request(request_id).code != ResultCode::Ok {
                cancel.cancel();
            }
            GaveUp::Resent
        }
        RecordState::Finished => {
            cancel.cancel();
            GaveUp::AlreadyFinished
        }
        RecordState::Unknown => {
            let _ = service.cancel_request(request_id);
            cancel.cancel();
            GaveUp::Resent
        }
    };
    if let Some(submitted) = shared.lock().submitted.get_mut(&id) {
        submitted.gave_up = Some(gave_up);
    }
}

/// The next job, marked running, or `None` once the worker is to end.
///
/// The user lane is taken in order: while its first job waits out a
/// `busy` backoff, nothing is taken, background jobs included, so user
/// priority holds across the wait.
fn take(shared: &Shared) -> Option<Job> {
    let mut state = shared.lock();
    loop {
        if state.shut_down {
            return None;
        }
        let now = Instant::now();
        let due = |job: &Job| job.not_before.is_none_or(|at| at <= now);
        let next = match state.user.front() {
            Some(job) if due(job) => state.user.pop_front(),
            Some(_) => None,
            None => state
                .background
                .iter()
                .position(due)
                .and_then(|at| state.background.remove(at)),
        };
        if let Some(job) = next {
            let (cancel, stops_at_safe_points) = match &job.work {
                Work::Execute { cancel, intent, .. } | Work::GiveUp { cancel, intent, .. } => {
                    (Some(cancel.clone()), intent.stops_at_safe_points())
                }
                Work::ObserveHost { .. } => (None, false),
            };
            state.running = Some(Running {
                id: job.id,
                cancel,
                accepted: false,
                stops_at_safe_points,
            });
            return Some(job);
        }
        // Nothing is ready: wait for a change, or for the first job
        // waiting after `busy` to be due.
        let due = match state.user.front() {
            Some(job) => job.not_before,
            None => state
                .background
                .iter()
                .filter_map(|job| job.not_before)
                .min(),
        };
        state = match due {
            Some(due) => {
                shared
                    .changed
                    .wait_timeout(state, due.saturating_duration_since(now))
                    .unwrap_or_else(PoisonError::into_inner)
                    .0
            }
            None => shared
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner),
        };
    }
}

fn finish(shared: &Shared) {
    shared.lock().running = None;
}

/// The running job's progress sink. It only records acceptance and
/// forwards a copy: it never calls back into the library. Then the
/// backend hears of the stage (`Backend::on_progress`), where a test may
/// hold the job.
///
/// Every stage is reported after acceptance, and a same-ID re-entry
/// reports later stages only (fact 10), so any stage marks the job
/// accepted (R-14).
struct Forward<'a> {
    id: DesktopRequestId,
    shared: &'a Shared,
    backend: &'a dyn Backend,
    kind: JobKind,
}

impl ProgressSink for Forward<'_> {
    fn event(&mut self, event: &ProgressEvent) {
        if let Some(running) = self.shared.lock().running.as_mut() {
            running.accepted = true;
        }
        self.shared
            .send(WorkerEvent::Progress(self.id, event.clone()));
        self.backend.on_progress(self.kind, event.stage);
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
