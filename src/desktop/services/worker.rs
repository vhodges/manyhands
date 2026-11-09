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
        OperationId, ProgressEvent, ProgressSink, RepositoryService, RequestId,
        RequestState as RecordDtoState,
        keys::{SecretPassphrase, SessionCredentials},
        transport::SshAuthority,
    },
    results::{Envelope, ResultCode},
};

use crate::desktop::model::polling::{CancelCause, PollAttempt, Root};

use super::{
    BUSY_RETRY_DELAYS, Backend, JobKind, poll,
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
    /// An automatic poll ended (D6 B1), with the desktop's account of its
    /// answer. Sent after the worker clears its running job, as `Finished`
    /// is, so a quit that hears it finds the worker idle. A poll sends no
    /// `Queued`, `Started` or `Progress`: it is not a request.
    Polled(Root, PollAttempt),
    /// A user job was admitted while this poll runs, or quit or a
    /// reconfiguration asked to stop it (D6 B2, P-1, P-7). The drain starts
    /// the task that calls `cancel_poll` until the poll answers. Sent once
    /// per running poll, however many asks follow.
    PreemptPoll {
        root: Root,
        op: OperationId,
    },
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

/// Why a request was, or was not, queued again (`WorkerHandle::retry_as`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Retried {
    /// Queued again.
    Requeued,
    /// The worker holds it queued or running already.
    AlreadyOut,
    /// Quitting closed the queue.
    Closed,
    /// This worker never took it.
    NotHeld,
    /// Held and finished, and still left alone (R-78, or a Give up of a
    /// command that cannot be given up).
    Refused,
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
    /// An automatic poll of the repository at `root` (D6 B1). Not an
    /// `Intent`: it does not go through the mutation boundary. `restart`
    /// resends `operation` after `Busy`, an interruption or a launch
    /// takeover (R-4); otherwise `operation` is new.
    Poll {
        root: Root,
        operation: OperationId,
        restart: bool,
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
    /// Whether its token was cancelled while it waited.
    fn cancelled(&self) -> bool {
        match &self.work {
            Work::Execute { cancel, .. } | Work::GiveUp { cancel, .. } => cancel.is_cancelled(),
            Work::ObserveHost { .. } | Work::Poll { .. } => false,
        }
    }

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
    /// `None` for a host observation, which cannot be cancelled, and for a
    /// poll, which has no token: `quit()` does not stop a running poll. A
    /// poll is stopped through `cancel_poll` by the cancel task (P-1) that
    /// `ask_poll_cancel` starts; Task 10 asks it for quit.
    cancel: Option<CancellationToken>,
    accepted: bool,
    /// Whether its command honours the token after acceptance (R-1).
    stops_at_safe_points: bool,
    /// The poll this is, and whether a stop was asked of it (P-1, P-2).
    poll: Option<PollRun>,
}

/// A running automatic poll, and the stop asked of it. The flag is the
/// worker's: the library's cancel can be lost (Fact 30), so the worker also
/// checks this before every `poll_automatic` (P-2).
struct PollRun {
    root: Root,
    op: OperationId,
    asked: Option<CancelCause>,
}

/// How firmly a cause ends a poll: quit, then a configuration change, then a
/// takeover's stored cancel, then a user job's preemption. The strongest ask
/// names the answer.
fn firmness(cause: CancelCause) -> u8 {
    match cause {
        CancelCause::Quit => 3,
        CancelCause::Reconfiguration => 2,
        CancelCause::Takeover => 1,
        CancelCause::Preemption => 0,
    }
}

/// Asks the running poll, if there is one, to stop for `cause`. The first ask
/// of a poll sends `PreemptPoll`; later asks only name a firmer cause, so
/// quit, a reconfiguration and any number of user jobs share one cancel task
/// (P-7). False when no poll is running.
fn ask_poll_cancel(shared: &Shared, state: &mut State, cause: CancelCause) -> bool {
    let Some(poll) = state
        .running
        .as_mut()
        .and_then(|running| running.poll.as_mut())
    else {
        return false;
    };
    let first = poll.asked.is_none();
    if poll
        .asked
        .is_none_or(|asked| firmness(cause) > firmness(asked))
    {
        poll.asked = Some(cause);
    }
    if first {
        shared.send(WorkerEvent::PreemptPoll {
            root: poll.root.clone(),
            op: poll.op,
        });
    }
    true
}

/// Whether the poll `op` is the job the worker is running now: the cancel
/// task's reason to go on (P-1). It holds the shared state, not a
/// `WorkerHandle`, so it never ends the worker by being the last to let go.
#[derive(Clone)]
pub(crate) struct PollWatch(Arc<Shared>);

impl PollWatch {
    pub(crate) fn running(&self, op: OperationId) -> bool {
        let state = self.0.lock();
        // A worker that is gone (shut down, or ended by a panic) answers no
        // poll, so the cancel task must not wait for one.
        !state.shut_down
            && state
                .running
                .as_ref()
                .and_then(|running| running.poll.as_ref())
                .is_some_and(|poll| poll.op == op)
    }
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

/// Marks the worker gone when its thread ends, a panic included: nothing is
/// running then, and the cancel task of a poll (`PollWatch`) stops instead of
/// resending for ever.
struct WorkerEnd(Arc<Shared>);

impl Drop for WorkerEnd {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        state.shut_down = true;
        state.running = None;
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
            .spawn(move || {
                let _ends = WorkerEnd(worker.clone());
                run(&worker, backend.as_ref());
            })
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
        self.requeue(id, None, secret, false).is_ok()
    }

    /// `retry`, saying why it queued the request `id` again or did not
    /// (Task 16 re-review, I-1).
    pub(crate) fn retry_as(&self, id: DesktopRequestId) -> Retried {
        match self.requeue(id, None, None, false) {
            Ok(_) => Retried::Requeued,
            Err(why) => why,
        }
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
        self.requeue(id, None, None, true).is_ok()
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
        self.requeue(id, Some(confirmation), None, false).ok()
    }

    fn requeue(
        &self,
        id: DesktopRequestId,
        confirmation: Option<ConfirmationId>,
        secret: Option<SecretPassphrase>,
        give_up: bool,
    ) -> Result<u32, Retried> {
        let shared = self.shared();
        let mut state = shared.lock();
        if state.closed {
            return Err(Retried::Closed);
        }
        if state.queued(id)
            || state
                .running
                .as_ref()
                .is_some_and(|running| running.id == id)
        {
            return Err(Retried::AlreadyOut);
        }
        // R-78, as the owner narrowed it (2026-10-28): a request is not sent
        // again once a later attempt of its item would be undone by it,
        // unless that later attempt was a release refused with nothing
        // touched (`Intent::kept_from_again_by`).
        let asked = state.submitted.get(&id).ok_or(Retried::NotHeld)?;
        if give_up && !asked.intent.can_be_given_up() {
            return Err(Retried::Refused);
        }
        if state.submitted.values().any(|later| {
            later.sent > asked.sent
                && asked
                    .intent
                    .kept_from_again_by(&later.intent, later.answers.all_refused())
        }) {
            return Err(Retried::Refused);
        }
        let sent = state.next_send();
        let submitted = state.submitted.get_mut(&id).ok_or(Retried::NotHeld)?;
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
        Ok(attempt)
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
        self.shared().declined.attempting(id, Lane::User);
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
                ..
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

    /// Asks the running automatic poll, if any, to stop for `cause`: sets its
    /// flag and, the first time, sends `PreemptPoll` for the drain's cancel
    /// task (P-1, P-7). A user job admitted while a poll runs does this by
    /// itself, with `Preemption`; quit (Task 10) asks with `Quit` and a
    /// reconfiguration (the scheduler) with `Reconfiguration`. True when a poll was
    /// running.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 10.
    pub(crate) fn ask_poll_cancel(&self, cause: CancelCause) -> bool {
        let shared = self.shared();
        ask_poll_cancel(shared, &mut shared.lock(), cause)
    }

    /// What the cancel task asks whether its poll is still running.
    pub(crate) fn poll_watch(&self) -> PollWatch {
        PollWatch(self.owner.0.clone())
    }

    /// The keys the background job `id`'s last attempt was refused a prompt
    /// for (B5): not declined, and what a retirement's `unlock_required`
    /// answer, which names no key, is held under.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 11a.
    pub(crate) fn refused_keys(
        &self,
        id: DesktopRequestId,
    ) -> Vec<manyhands::repository::SharedKeyId> {
        self.shared().declined.refused_for(id)
    }

    /// Queues an automatic poll of the repository at `root` on the
    /// background lane, under `operation` (D6 B1): a new ID from
    /// `OperationId::new()`, or the stored one for a `restart`. The UI
    /// hears `WorkerEvent::Polled` when it ends. `None` once quitting has
    /// closed the queue: nothing is queued.
    pub(crate) fn submit_poll(
        &self,
        root: Root,
        operation: OperationId,
        restart: bool,
    ) -> Option<DesktopRequestId> {
        let shared = self.shared();
        let mut state = shared.lock();
        if state.closed {
            return None;
        }
        let id = next_id(&mut state);
        let work = Work::Poll {
            root,
            operation,
            restart,
        };
        admit(shared, &mut state, Job::new(id, work, Lane::Background));
        Some(id)
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
    // The same resume, or the same lifecycle request, is never out twice
    // (V16, M1): the one in flight stands for this call.
    if let Some(out) = in_flight_twin(&state, &intent, request_id) {
        return out;
    }
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

/// The request in flight that `intent`, under `request_id`, would only
/// duplicate: a resume of the same operation of the same repository, or a
/// close or merge under the same library request ID. Queued, waiting out
/// `busy`, or running; a finished one is no twin.
fn in_flight_twin(
    state: &State,
    intent: &Intent,
    request_id: RequestId,
) -> Option<DesktopRequestId> {
    let twin = |submitted: &Submitted| match (&submitted.intent, intent) {
        (
            Intent::OperationResume {
                root: held,
                operation_id: held_operation,
            },
            Intent::OperationResume { root, operation_id },
        ) => held == root && held_operation == operation_id,
        (
            Intent::TicketClose { .. } | Intent::DocumentPromote { .. },
            Intent::TicketClose { .. } | Intent::DocumentPromote { .. },
        ) => submitted.request_id == request_id,
        _ => false,
    };
    // One Stop was pressed on is ending: a call sent after the Stop is a
    // new request, not that one.
    let running = state
        .running
        .as_ref()
        .filter(|running| {
            !running
                .cancel
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled)
        })
        .map(|running| running.id);
    running
        .into_iter()
        .chain(
            state
                .user
                .iter()
                .chain(&state.background)
                .filter(|job| !job.cancelled())
                .map(|job| job.id),
        )
        .find(|id| state.submitted.get(id).is_some_and(twin))
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
    // Every way a user job enters the worker comes through here (`submit`,
    // `submit_with_request_id`, `requeue`, `observe_host`; Fact 36), so a
    // running poll hears of it once, whichever it was (P-1). A background
    // job, a retirement among them, asks nothing (P-5, P-6).
    if job.lane == Lane::User {
        ask_poll_cancel(shared, state, CancelCause::Preemption);
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
        let attempt = matches!(
            work,
            Work::Execute { .. } | Work::GiveUp { .. } | Work::Poll { .. }
        )
        .then_some(id);
        // The provider learns the lane, so a background job never prompts
        // (B5).
        shared.declined.attempting(attempt, lane);
        if matches!(work, Work::Poll { .. }) {
            shared.declined.polling(id);
        }
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
            Work::Poll {
                root,
                operation,
                restart,
            } => {
                backend.before_job(JobKind::Poll);
                let attempt = poll::run(backend, &mut session, &root, operation, restart, || {
                    asked_cause(shared)
                });
                // The job is over before the UI hears so (Fact 36).
                finish(shared);
                shared.send(WorkerEvent::Polled(root, attempt));
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
                Work::ObserveHost { .. } | Work::Poll { .. } => (None, false),
            };
            let poll = match &job.work {
                Work::Poll {
                    root, operation, ..
                } => Some(PollRun {
                    root: root.clone(),
                    op: *operation,
                    asked: None,
                }),
                _ => None,
            };
            state.running = Some(Running {
                id: job.id,
                cancel,
                accepted: false,
                stops_at_safe_points,
                poll,
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

/// Why the running poll was asked to stop, if it was: quit closed the queue,
/// or a user job, a reconfiguration or quit asked (`ask_poll_cancel`). The
/// poll reads it before every `poll_automatic` (P-2) and, for a `Cancelled`
/// answer, to name the cause (R-3).
fn asked_cause(shared: &Shared) -> Option<CancelCause> {
    let state = shared.lock();
    if state.closed {
        return Some(CancelCause::Quit);
    }
    state.running.as_ref()?.poll.as_ref()?.asked
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
