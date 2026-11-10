//! A recording double for the automatic poll's calls (D6 Task 3).
//!
//! It wraps the delaying backend, so every other `Backend` method is the
//! fixture's. Each poll method records its name and then makes the
//! library's own call. It fabricates nothing, with two exceptions that
//! stand in for the SSH transport, which no desktop test reaches:
//!
//! - `hold_poll` holds `poll_automatic` before its library call until the
//!   gate is released or dropped, or until `cancel_poll` is called: the
//!   cancel releases it, as the library's cancel makes a running poll stop
//!   at its next safe point.
//! - `answer_poll` queues a constructed `Library` answer that stands in place
//!   of the next poll's (`Backend::after_poll`), since the library builds a
//!   `RemotePollReport` only inside itself and no desktop test reaches the
//!   SSH transport that would give a `Cancelled` or an `Interrupted` one. The
//!   poll's call itself does not run, so nothing is stored.
//! - `script_cancels` scripts what the next `cancel_poll` calls do, in
//!   place of the library's own cancel: nothing happens (an `Ok` on a row
//!   that is not active, Fact 30), the library's real error for a row that
//!   does not exist, or an effective cancel that releases the held poll.
//! - `ask_unlock` makes `poll_automatic` ask the session for a passphrase
//!   as the transport would, with a public `UnlockRequest`, and answer the
//!   library's locked-key failure when the session refuses. Nothing else
//!   reaches the provider without SSH.
//! - `ask_in_check_now` makes a Check now, the user lane's `poll once`, and
//!   a retirement, `context retire`, ask a provider for a passphrase before
//!   its library call, as the transport would: the worker's own provider,
//!   sharing its declined keys, so a prompt reaches the UI, a declined key
//!   is answered at once, and a background job is refused (B5).
//! - `select_key` sets the key `selected_key` answers, the registry's
//!   selection a retirement's key attention falls back to (E3).
//! - `resume_polling` records each Resume checking and then makes the
//!   library's own call.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use manyhands::{
    drafts::DraftStore,
    repository::{
        AutomaticPollEligibility, ConflictDto, Mutation, MutationDataDto, OperationId,
        PollPublicationRemoteRequest, PollingStatusDto, PrepareDataDto, ProgressStage, ReadError,
        RemoteListDto, RemotePollError, RemotePollResult, RemoteSnapshot, RepositoryError,
        RepositoryService, ResolvedRepository, SharedKeyId,
        keys::{
            KeySourceToken, KeyStore, PassphraseResponse, SessionCredentialProvider,
            SessionCredentials, SessionUnlockFailure, UnlockReason, UnlockRequest,
        },
        transport::{SshAuthority, SshDirection, SshTransportError, SshTransportErrorKind},
    },
    results::Envelope,
};

use super::delaying::{DelayingBackend, Gate, JobGate};
use crate::desktop::services::{
    Backend, DraftStoreUnavailable, JobKind, KeyStoreUnavailable, ReadGate, ReadKind,
    layout::{LayoutStore, LayoutStoreError},
    poll_convert::Library,
    prompts::DesktopProvider,
    resolutions::{ResolutionStore, ResolutionStoreError},
};

/// One call the double heard, in the order made.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PollCall {
    Resolve,
    PollingStatus,
    PollEligibility,
    PollAutomatic,
    RemoteSnapshot,
    CancelPoll,
}

/// What a scripted `cancel_poll` does (P-1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CancelStep {
    /// The library's real cancel, for an operation it has no row for: an
    /// error (`recovery_required`), a poll that has not reserved yet. The
    /// held poll stays held.
    Missing,
    /// `Ok(())` and nothing else: the row exists but is not active, such as
    /// an `interrupted` row between a same-ID restart's attempts. The held
    /// poll stays held.
    Ineffective,
    /// `Ok(())` and the held poll is released, as a cancel that reached an
    /// active row stops the poll at its next safe point.
    Effective,
}

/// One `cancel_poll` the double heard, and what it answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CancelSeen {
    pub op: OperationId,
    pub answered_ok: bool,
}

pub(crate) struct PollingBackend {
    inner: Arc<DelayingBackend>,
    calls: Mutex<Vec<PollCall>>,
    /// Holds for the next `poll_automatic`, each taken in turn.
    holds: Mutex<VecDeque<Arc<Gate>>>,
    /// The hold `poll_automatic` is parked at, if any.
    parked: Mutex<Option<Arc<Gate>>>,
    /// The unlock `poll_automatic` asks the session for.
    unlock: Mutex<Option<UnlockRequest>>,
    /// Answers that stand in place of the next polls', each taken in turn.
    canned: Mutex<VecDeque<Library>>,
    /// Errors that stand in place of the next polls' results (`fail_poll`).
    failures: Mutex<VecDeque<RemotePollError>>,
    /// What the next `cancel_poll` calls do; unscripted, one releases the
    /// held poll and asks the library.
    cancel_script: Mutex<VecDeque<CancelStep>>,
    cancels: Mutex<Vec<CancelSeen>>,
    /// Each `poll_automatic` request's operation and `restart`.
    requests: Mutex<Vec<(OperationId, bool)>>,
    /// The root of each `poll_automatic` request, in the same order.
    roots: Mutex<Vec<String>>,
    /// How many times the scheduler read a repository's remotes
    /// (`publication_remotes`). Not a `PollCall`: it is the scheduler's
    /// call, not part of an automatic poll's own list (B4).
    remote_reads: Mutex<usize>,
    /// The root of each of those reads, in order.
    remote_read_roots: Mutex<Vec<String>>,
    /// The provider and request a Check now asks before its library call.
    check_asks: Mutex<Option<(DesktopProvider, UnlockRequest)>>,
    /// What each of those asks was answered.
    asked: Mutex<Vec<Asked>>,
    /// The root of every `resume_polling` made, in order.
    resumes: Mutex<Vec<String>>,
    /// The key `selected_key` answers.
    selected: Mutex<Option<SharedKeyId>>,
}

/// What a Check now's passphrase request was answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Asked {
    Supplied,
    Cancelled,
    Unavailable,
}

impl PollingBackend {
    pub(crate) fn new(inner: Arc<DelayingBackend>) -> Self {
        Self {
            inner,
            calls: Mutex::default(),
            holds: Mutex::default(),
            parked: Mutex::default(),
            unlock: Mutex::default(),
            canned: Mutex::default(),
            failures: Mutex::default(),
            cancel_script: Mutex::default(),
            cancels: Mutex::default(),
            requests: Mutex::default(),
            roots: Mutex::default(),
            remote_reads: Mutex::default(),
            remote_read_roots: Mutex::default(),
            check_asks: Mutex::default(),
            asked: Mutex::default(),
            resumes: Mutex::default(),
            selected: Mutex::default(),
        }
    }

    /// The fixture's delaying backend, for a test's other holds.
    pub(crate) fn inner(&self) -> &DelayingBackend {
        &self.inner
    }

    /// The same, shared.
    pub(crate) fn inner_shared(&self) -> Arc<DelayingBackend> {
        self.inner.clone()
    }

    /// The poll calls so far, in order.
    pub(crate) fn calls(&self) -> Vec<PollCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Forgets the calls so far.
    pub(crate) fn clear_calls(&self) {
        self.calls.lock().unwrap().clear();
    }

    /// Holds the next `poll_automatic` before its library call until the
    /// gate is released or dropped, or until an effective `cancel_poll`
    /// (`script_cancels`; unscripted, any call is effective).
    #[must_use = "dropping the handle releases the hold"]
    pub(crate) fn hold_poll(&self) -> JobGate {
        let gate = Arc::new(Gate::default());
        self.holds.lock().unwrap().push_back(gate.clone());
        JobGate(gate)
    }

    /// The next poll's answer is `library` (`Backend::after_poll`); its
    /// `poll_automatic` call is not made. Answers queue: each poll takes the
    /// oldest.
    pub(crate) fn answer_poll(&self, library: Library) {
        self.canned.lock().unwrap().push_back(library);
    }

    /// The next poll's `poll_automatic` answers `error` without asking the
    /// library, for the errors a test can build (`Busy`, `Interrupted`,
    /// `ConflictPending`, `Transport`). Errors queue.
    pub(crate) fn fail_poll(&self, error: RemotePollError) {
        self.failures.lock().unwrap().push_back(error);
    }

    /// What the next `cancel_poll` calls do, one step each. Once the script
    /// is used up, a call is effective.
    pub(crate) fn script_cancels(&self, steps: impl IntoIterator<Item = CancelStep>) {
        self.cancel_script.lock().unwrap().extend(steps);
    }

    /// Every `cancel_poll` heard so far, with what it answered.
    pub(crate) fn cancels(&self) -> Vec<CancelSeen> {
        self.cancels.lock().unwrap().clone()
    }

    /// The operation and `restart` of every `poll_automatic` made so far.
    pub(crate) fn requests(&self) -> Vec<(OperationId, bool)> {
        self.requests.lock().unwrap().clone()
    }

    /// The root of every `poll_automatic` made so far, in order.
    pub(crate) fn polled_roots(&self) -> Vec<String> {
        self.roots.lock().unwrap().clone()
    }

    /// The root of every remotes read so far, in order.
    pub(crate) fn remote_read_roots(&self) -> Vec<String> {
        self.remote_read_roots.lock().unwrap().clone()
    }

    /// How many times the scheduler has read a repository's remotes.
    pub(crate) fn remote_reads(&self) -> usize {
        *self.remote_reads.lock().unwrap()
    }

    /// Makes every `poll_automatic` ask the session for `request`'s
    /// passphrase, as the transport does for a protected key.
    pub(crate) fn ask_unlock(&self, request: UnlockRequest) {
        *self.unlock.lock().unwrap() = Some(request);
    }

    /// Makes every Check now ask `provider` for `request`'s passphrase
    /// before its library call, as the transport does for a protected key.
    pub(crate) fn ask_in_check_now(&self, provider: DesktopProvider, request: UnlockRequest) {
        *self.check_asks.lock().unwrap() = Some((provider, request));
    }

    /// Sets the key `selected_key` answers (E3).
    pub(crate) fn select_key(&self, key: Option<SharedKeyId>) {
        *self.selected.lock().unwrap() = key;
    }

    /// What each of those requests was answered, in order.
    pub(crate) fn asked(&self) -> Vec<Asked> {
        self.asked.lock().unwrap().clone()
    }

    /// The root of every Resume checking made, in order.
    pub(crate) fn resumed(&self) -> Vec<String> {
        self.resumes.lock().unwrap().clone()
    }

    /// Polls stop asking the session for a passphrase.
    pub(crate) fn stop_asking_unlock(&self) {
        *self.unlock.lock().unwrap() = None;
    }

    fn record(&self, call: PollCall) {
        self.calls.lock().unwrap().push(call);
    }
}

/// An unlock request for the key `key`, whose private file is written
/// under `directory`.
pub(crate) fn unlock_request(directory: &Path, key: SharedKeyId) -> UnlockRequest {
    let path = directory.join(key.to_string());
    std::fs::write(&path, b"opaque fixture bytes").unwrap();
    UnlockRequest {
        key_id: key,
        label: "primary".to_owned(),
        source: KeySourceToken::observe(&path).unwrap(),
        reason: UnlockReason::ProtectedKey,
    }
}

/// The transport's error of `kind` for the repository at `root`.
pub(crate) fn transport_error(
    root: &str,
    kind: SshTransportErrorKind,
    key: Option<SharedKeyId>,
    authority: Option<&SshAuthority>,
) -> RemotePollError {
    RemotePollError::Transport(SshTransportError {
        root: PathBuf::from(root),
        remote_name: "origin".to_owned(),
        direction: SshDirection::Fetch,
        selected_key_id: key,
        authority: authority.cloned(),
        kind,
    })
}

/// The transport's error for a refused unlock.
pub(crate) fn locked(
    request: &UnlockRequest,
    failure: SessionUnlockFailure,
    root: &Path,
) -> RemotePollError {
    let kind = match failure {
        SessionUnlockFailure::Cancelled => SshTransportErrorKind::UnlockCancelled,
        SessionUnlockFailure::ProviderUnavailable => SshTransportErrorKind::ProviderUnavailable,
        _ => SshTransportErrorKind::UnlockFailed,
    };
    RemotePollError::Transport(SshTransportError {
        root: PathBuf::from(root),
        remote_name: "origin".to_owned(),
        direction: SshDirection::Fetch,
        selected_key_id: Some(request.key_id),
        authority: None,
        kind,
    })
}

impl Backend for PollingBackend {
    fn service(&self) -> &RepositoryService {
        self.inner.service()
    }

    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        self.inner.key_store()
    }

    fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable> {
        self.inner.drafts()
    }

    fn resolutions(&self) -> Result<&ResolutionStore, ResolutionStoreError> {
        self.inner.resolutions()
    }

    fn layout(&self) -> Result<&LayoutStore, LayoutStoreError> {
        self.inner.layout()
    }

    fn read_gate(&self, kind: ReadKind) -> Option<ReadGate> {
        self.inner.read_gate(kind)
    }

    fn before_job(&self, kind: JobKind) {
        if matches!(kind, JobKind::PollOnce | JobKind::ContextRetire) {
            let mut asks = self.check_asks.lock().unwrap();
            if let Some((provider, request)) = asks.as_mut() {
                let answer = match provider.request_passphrase(request) {
                    PassphraseResponse::Supplied(_) => Asked::Supplied,
                    PassphraseResponse::Cancelled => Asked::Cancelled,
                    PassphraseResponse::Unavailable => Asked::Unavailable,
                };
                self.asked.lock().unwrap().push(answer);
            }
        }
        self.inner.before_job(kind);
    }

    fn instead_of_job(&self, kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        self.inner.instead_of_job(kind)
    }

    fn after_job(&self, kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        self.inner.after_job(kind)
    }

    fn on_progress(&self, kind: JobKind, stage: ProgressStage) {
        self.inner.on_progress(kind, stage);
    }

    fn after_prepare(&self, mutation: &Mutation) -> Option<Envelope<PrepareDataDto>> {
        self.inner.after_prepare(mutation)
    }

    fn after_conflict_read(
        &self,
        kind: ReadKind,
        operation_id: &OperationId,
    ) -> Option<ConflictDto> {
        self.inner.after_conflict_read(kind, operation_id)
    }

    fn resolve(&self, root: &Path) -> Result<ResolvedRepository, ReadError> {
        self.record(PollCall::Resolve);
        self.inner.resolve(root)
    }

    fn polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError> {
        self.record(PollCall::PollingStatus);
        self.inner.polling_status(repo)
    }

    fn publication_remotes(&self, repo: &ResolvedRepository) -> Result<RemoteListDto, ReadError> {
        *self.remote_reads.lock().unwrap() += 1;
        self.remote_read_roots
            .lock()
            .unwrap()
            .push(repo.root().to_string_lossy().into_owned());
        self.inner.publication_remotes(repo)
    }

    fn poll_eligibility(&self, root: &Path) -> Result<AutomaticPollEligibility, RepositoryError> {
        self.record(PollCall::PollEligibility);
        self.inner.poll_eligibility(root)
    }

    fn poll_automatic(
        &self,
        request: PollPublicationRemoteRequest,
        session: &mut SessionCredentials<DesktopProvider>,
    ) -> Result<RemotePollResult, RemotePollError> {
        self.record(PollCall::PollAutomatic);
        self.requests
            .lock()
            .unwrap()
            .push((request.operation_id, request.restart));
        self.roots
            .lock()
            .unwrap()
            .push(request.root.to_string_lossy().into_owned());
        let hold = self.holds.lock().unwrap().pop_front();
        if let Some(hold) = hold {
            *self.parked.lock().unwrap() = Some(hold.clone());
            hold.wait();
            *self.parked.lock().unwrap() = None;
        }
        if let Some(error) = self.failures.lock().unwrap().pop_front() {
            return Err(error);
        }
        // A constructed answer stands in: the library is not asked, and
        // `after_poll` gives the answer.
        if !self.canned.lock().unwrap().is_empty() {
            return Err(RemotePollError::Busy);
        }
        let unlock = self.unlock.lock().unwrap().clone();
        if let Some(unlock) = unlock {
            return match session.with_passphrase(unlock.clone(), |_| Ok(())) {
                Ok(()) => self.inner.poll_automatic(request, session),
                Err(failure) => Err(locked(&unlock, failure, &request.root)),
            };
        }
        self.inner.poll_automatic(request, session)
    }

    fn remote_snapshot(&self, root: &Path) -> Result<RemoteSnapshot, RepositoryError> {
        self.record(PollCall::RemoteSnapshot);
        self.inner.remote_snapshot(root)
    }

    fn after_poll(&self, _operation: OperationId) -> Option<Library> {
        self.canned.lock().unwrap().pop_front()
    }

    fn selected_key(&self) -> Option<SharedKeyId> {
        *self.selected.lock().unwrap()
    }

    fn resume_polling(&self, root: &Path) -> Result<(), RepositoryError> {
        self.resumes
            .lock()
            .unwrap()
            .push(root.to_string_lossy().into_owned());
        self.inner.resume_polling(root)
    }

    fn cancel_poll(&self, root: &Path, op: OperationId) -> Result<(), RepositoryError> {
        self.record(PollCall::CancelPoll);
        let step = self.cancel_script.lock().unwrap().pop_front();
        let release = || {
            if let Some(parked) = self.parked.lock().unwrap().clone() {
                parked.release();
            }
        };
        let answered = match step {
            // The library's own answer, for a row it does not have.
            Some(CancelStep::Missing) => self.inner.cancel_poll(root, op),
            Some(CancelStep::Ineffective) => Ok(()),
            Some(CancelStep::Effective) => {
                release();
                Ok(())
            }
            None => {
                release();
                self.inner.cancel_poll(root, op)
            }
        };
        self.cancels.lock().unwrap().push(CancelSeen {
            op,
            answered_ok: answered.is_ok(),
        });
        answered
    }
}
