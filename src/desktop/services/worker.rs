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
//! Cancellation follows the library: a queued job is removed and never
//! runs; a running job's token is cancelled, which every command honors
//! until it is accepted; after acceptance a local command cannot be
//! stopped, and a host observation has no token at all.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError},
    thread,
    time::Instant,
};

use futures::channel::{mpsc, oneshot};
use manyhands::{
    repository::{
        CancellationToken, ConfirmationId, HostObservationDto, MutationCall, MutationDataDto,
        ProgressEvent, ProgressSink, ProgressStage, RequestId, SharedKeyId,
        keys::{
            PassphraseResponse, SecretPassphrase, SessionCredentialProvider, SessionCredentials,
            UnlockReason, UnlockRequest,
        },
        transport::SshAuthority,
    },
    results::{Envelope, ResultCode},
};

use super::{
    BUSY_RETRY_DELAYS, Backend, JobKind,
    requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
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
    #[allow(dead_code)] // The prompt bridge sends prompts (Task 11).
    Prompt(PromptRequest),
}

/// A passphrase the library asks for while a job runs. The worker waits
/// for `reply`; a dropped sender reads as `Unavailable`.
///
/// No `Debug`: the reply carries a passphrase.
#[allow(dead_code)] // The prompt bridge sends prompts (Task 11).
pub(crate) struct PromptRequest {
    pub key_id: SharedKeyId,
    pub label: String,
    pub reason: UnlockReason,
    pub reply: oneshot::Sender<PromptReply>,
}

/// The answer to a prompt. No `Debug`: it may carry the passphrase.
#[allow(dead_code)] // The prompt bridge sends replies (Task 11).
pub(crate) enum PromptReply {
    Supplied(SecretPassphrase),
    Cancelled,
    Unavailable,
}

/// What cancelling a request did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CancelResult {
    /// It was queued, and was removed: it never runs.
    Removed,
    /// It is running and not yet accepted: its token is cancelled.
    Signalled,
    /// It is accepted, has no token, or is no longer queued or running:
    /// nothing can stop it now.
    TooLate,
}

/// Whether quitting can go ahead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum QuitState {
    Idle,
    /// This job is still running; quit when it returns.
    Finishing(DesktopRequestId),
}

/// The credential provider of the worker's session. Until the prompt
/// bridge arrives (Task 11) nobody is there to ask, so every unlock is
/// `Unavailable` and no prompt is sent.
struct DesktopProvider;

impl SessionCredentialProvider for DesktopProvider {
    fn request_passphrase(&mut self, _request: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

/// What a job does.
enum Work {
    Execute {
        request_id: RequestId,
        intent: Intent,
        confirmation: Option<ConfirmationId>,
        secret: Option<SecretPassphrase>,
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
    /// How many times a background job answered `busy` has been
    /// admitted again.
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
}

/// The job the worker is running.
struct Running {
    id: DesktopRequestId,
    /// `None` for a host observation, which cannot be cancelled.
    cancel: Option<CancellationToken>,
    accepted: bool,
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
}

impl State {
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
        let shared = self.shared();
        let mut state = shared.lock();
        let id = next_id(&mut state);
        let request_id = RequestId::new();
        state.submitted.insert(
            id,
            Submitted {
                request_id,
                intent: intent.clone(),
                confirmation,
                lane,
                attempt: 1,
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
            intent,
            confirmation,
            secret,
            cancel: CancellationToken::new(),
        };
        admit(shared, &mut state, Job::new(id, work, lane));
        id
    }

    /// Queues the request `id` again, under its own request ID, so the
    /// library answers a finished one from its record. A request that is
    /// queued or running, or that this worker never took, is left alone,
    /// and so is every request once quitting has closed the queue: its
    /// last result stands.
    pub(crate) fn retry(&self, id: DesktopRequestId, secret: Option<SecretPassphrase>) {
        self.requeue(id, None, secret);
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
        self.requeue(id, Some(confirmation), None)
    }

    fn requeue(
        &self,
        id: DesktopRequestId,
        confirmation: Option<ConfirmationId>,
        secret: Option<SecretPassphrase>,
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
        let submitted = state.submitted.get_mut(&id)?;
        submitted.attempt += 1;
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
        let work = Work::Execute {
            request_id,
            intent,
            confirmation,
            secret,
            cancel: CancellationToken::new(),
        };
        admit(shared, &mut state, Job::new(id, work, lane));
        Some(attempt)
    }

    /// How many background jobs wait to be taken again after `busy`.
    #[cfg(test)]
    pub(crate) fn backing_off(&self) -> usize {
        self.shared()
            .lock()
            .background
            .iter()
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
                accepted: false,
            }) if *running == id => {
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

/// The worker thread: takes the next job, user lane first, until the last
/// handle is dropped.
fn run(shared: &Shared, backend: &dyn Backend) {
    let mut session = SessionCredentials::new(DesktopProvider);
    while let Some(job) = take(shared) {
        let Job {
            id,
            work,
            lane,
            busy_retries,
            ..
        } = job;
        match work {
            Work::Execute {
                request_id,
                intent,
                confirmation,
                secret,
                cancel,
            } => {
                let mutation = match intent.build(secret, || backend.key_store()) {
                    Ok(mutation) => mutation,
                    Err(not_built) => {
                        finish(shared);
                        shared.send(WorkerEvent::NotSent(id, not_built.into()));
                        continue;
                    }
                };
                shared.send(WorkerEvent::Started(id));
                backend.before_job(intent.job_kind());
                let mut progress = Forward { id, shared };
                let envelope = backend.service().execute(
                    MutationCall {
                        request_id,
                        confirmation,
                        mutation,
                    },
                    &mut session,
                    &mut progress,
                    &cancel,
                );
                // A background job answered `busy` waits and is taken again,
                // behind every user job, with no event: the UI sees one
                // request that finishes once.
                if lane == Lane::Background
                    && envelope.code == ResultCode::Busy
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
                    admit(
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
                    continue;
                }
                // The job is over before the UI hears so, so a quit that
                // hears it finds the worker idle.
                finish(shared);
                shared.send(WorkerEvent::Finished(id, Box::new(envelope)));
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

/// The next job, marked running, or `None` once the worker is to end.
fn take(shared: &Shared) -> Option<Job> {
    let mut state = shared.lock();
    loop {
        if state.shut_down {
            return None;
        }
        let now = Instant::now();
        let ready = state
            .background
            .iter()
            .position(|job| job.not_before.is_none_or(|at| at <= now));
        if let Some(job) = state
            .user
            .pop_front()
            .or_else(|| ready.and_then(|at| state.background.remove(at)))
        {
            let cancel = match &job.work {
                Work::Execute { cancel, .. } => Some(cancel.clone()),
                Work::ObserveHost { .. } => None,
            };
            state.running = Some(Running {
                id: job.id,
                cancel,
                accepted: false,
            });
            return Some(job);
        }
        // Nothing is ready: wait for a change, or for the first job
        // waiting after `busy` to be due.
        let due = state
            .background
            .iter()
            .filter_map(|job| job.not_before)
            .min();
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
/// forwards a copy: it never calls back into the library.
struct Forward<'a> {
    id: DesktopRequestId,
    shared: &'a Shared,
}

impl ProgressSink for Forward<'_> {
    fn event(&mut self, event: &ProgressEvent) {
        if event.stage == ProgressStage::Accepted
            && let Some(running) = self.shared.lock().running.as_mut()
        {
            running.accepted = true;
        }
        self.shared
            .send(WorkerEvent::Progress(self.id, event.clone()));
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
