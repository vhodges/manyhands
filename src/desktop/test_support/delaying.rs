//! The test backend: a real service over a fixture, whose reads and jobs a
//! test can hold.
//!
//! It only delays. It never fails or fabricates a call; failures in tests
//! come from the library's own failure points and hooks. The exceptions
//! are the canned answers (D5 decision 9): `hold_job_return_answering`, a
//! canned prepare (`answer_next_prepare`) and a canned conflict read
//! (`answer_next_conflict_read`). Each lets the library's real call run,
//! then puts a test's answer, built from library types, in place of its
//! answer, for an answer no fixture can reach. A test that uses one says
//! so in its doc comment, names the library test that produces the real
//! answer, and asserts nothing about repository state.
//!
//! `hold_at_stage` holds a job on the worker thread once it has reported
//! a named progress stage: a real interruption point, not an answer.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, OnceLock},
    time::Duration,
};

use futures::channel::oneshot;
use manyhands::{
    drafts::{DraftError, DraftStore},
    repository::{
        ConflictDto, Mutation, MutationDataDto, OperationId, OperationListDto, PrepareDataDto,
        ProgressStage, RepositoryService, keys::KeyStore,
    },
    results::Envelope,
};

use crate::desktop::services::{
    Backend, DraftStoreUnavailable, JobKind, KeyStoreUnavailable, ReadGate, ReadKind,
    layout::{LayoutStore, LayoutStoreError},
    resolutions::{ResolutionStore, ResolutionStoreError},
};

/// How many times a test can put another real service in place.
const REPLACEMENTS: usize = 8;

/// The holds a job takes when it reports a stage, by kind and stage.
type StageHolds = HashMap<(JobKind, ProgressStage), VecDeque<Arc<Gate>>>;

pub(crate) struct DelayingBackend {
    service: Arc<RepositoryService>,
    /// Real services a test put in place of the first, in order: the last
    /// one set is the one in use. Each stays open for as long as the
    /// backend, so a call already running on an earlier one finishes on
    /// it.
    replacements: [OnceLock<RepositoryService>; REPLACEMENTS],
    home: PathBuf,
    /// The real store over the fixture's data directory, or why it did
    /// not open.
    drafts: Result<DraftStore, DraftStoreUnavailable>,
    /// The real resolutions store over the fixture's data directory, or
    /// why it did not open; unavailable until `with_resolutions`.
    resolutions: Result<ResolutionStore, ResolutionStoreError>,
    /// The real layout store over the fixture's data directory, or why it
    /// did not open; unavailable until `with_layout`.
    layout: Result<LayoutStore, LayoutStoreError>,
    reads: Mutex<HashMap<ReadKind, VecDeque<ReadGate>>>,
    jobs: Mutex<HashMap<JobKind, VecDeque<Arc<Gate>>>>,
    /// Holds taken after a job's library call returns, each with the
    /// answer, if any, that stands in place of the library's.
    returned: Mutex<HashMap<JobKind, VecDeque<Returned>>>,
    /// Canned answers that stand in place of the next jobs' library calls,
    /// by kind: the call is not made.
    instead: Mutex<HashMap<JobKind, VecDeque<Envelope<MutationDataDto>>>>,
    /// Every read kind a read or prepare asked its gate for, in order:
    /// only observed.
    reads_seen: Mutex<Vec<ReadKind>>,
    /// Every job kind the worker reached `before_job` with, in order.
    seen: Mutex<Vec<JobKind>>,
    /// Holds taken when a job of the kind reports the stage.
    stages: Mutex<StageHolds>,
    /// Canned prepare answers, each by the command whose next prepare it
    /// stands in place of.
    prepares: Mutex<Vec<(&'static str, Envelope<PrepareDataDto>)>>,
    /// Canned conflicts, each standing in place of the next conflict read
    /// of its own operation ID.
    conflicts: Mutex<Vec<ConflictDto>>,
    /// A list that stands in place of the library's pending operations of
    /// every repository (`Some(None)`: the read fails).
    pending: Mutex<Option<Option<OperationListDto>>>,
}

impl DelayingBackend {
    /// A backend over `service` and the draft store's open result, with
    /// its key store in `home`.
    pub(crate) fn new(
        service: Arc<RepositoryService>,
        home: PathBuf,
        drafts: Result<DraftStore, DraftError>,
    ) -> Self {
        Self {
            service,
            replacements: Default::default(),
            home,
            drafts: drafts.map_err(DraftStoreUnavailable::from),
            resolutions: Err(ResolutionStoreError::Unavailable),
            layout: Err(LayoutStoreError::Unavailable),
            reads: Mutex::default(),
            jobs: Mutex::default(),
            returned: Mutex::default(),
            instead: Mutex::default(),
            reads_seen: Mutex::default(),
            seen: Mutex::default(),
            stages: Mutex::default(),
            prepares: Mutex::default(),
            conflicts: Mutex::default(),
            pending: Mutex::default(),
        }
    }

    /// The same backend over the resolutions store's open result.
    pub(crate) fn with_resolutions(
        mut self,
        resolutions: Result<ResolutionStore, ResolutionStoreError>,
    ) -> Self {
        self.resolutions = resolutions;
        self
    }

    /// The same backend over the layout store's open result.
    pub(crate) fn with_layout(mut self, layout: Result<LayoutStore, LayoutStoreError>) -> Self {
        self.layout = layout;
        self
    }

    /// Holds the next read of `kind` at its gate until the returned sender
    /// sends, or is dropped. Holds stack: each read takes the oldest.
    #[must_use = "dropping the handle releases the hold"]
    pub(crate) fn hold_read(&self, kind: ReadKind) -> oneshot::Sender<()> {
        let (sender, receiver) = oneshot::channel();
        self.reads
            .lock()
            .unwrap()
            .entry(kind)
            .or_default()
            .push_back(receiver);
        sender
    }

    /// Holds the next job of `kind` on the worker thread, before its
    /// library call, until the gate is released or dropped.
    #[must_use = "dropping the handle releases the hold"]
    pub(crate) fn hold_job(&self, kind: JobKind) -> JobGate {
        let gate = Arc::new(Gate::default());
        self.jobs
            .lock()
            .unwrap()
            .entry(kind)
            .or_default()
            .push_back(gate.clone());
        JobGate(gate)
    }

    /// Holds the next job of `kind` on the worker thread once its library
    /// call has returned, before the worker acts on the answer, until the
    /// gate is released or dropped.
    #[must_use = "dropping the handle releases the hold"]
    pub(crate) fn hold_job_return(&self, kind: JobKind) -> JobGate {
        self.hold_return(kind, None)
    }

    /// Holds the next job of `kind` as `hold_job_return` does, then gives
    /// the worker `answer` in place of the library's: the call itself
    /// still runs.
    #[must_use = "dropping the handle releases the hold"]
    pub(crate) fn hold_job_return_answering(
        &self,
        kind: JobKind,
        answer: Envelope<MutationDataDto>,
    ) -> JobGate {
        self.hold_return(kind, Some(answer))
    }

    /// Makes every repository's pending operations read answer `list` in
    /// place of the library's (`None`: the read fails), as the scheduler
    /// matches its in-flight entries against it.
    pub(crate) fn answer_pending(&self, list: Option<OperationListDto>) {
        *self.pending.lock().unwrap() = Some(list);
    }

    /// Answers the next job of `kind` with `answer` and makes no library
    /// call: for a job that would reach a remote no desktop test has, or
    /// take seconds to fail. Answers for one kind stack: each job takes the
    /// oldest.
    pub(crate) fn answer_job(&self, kind: JobKind, answer: Envelope<MutationDataDto>) {
        self.instead
            .lock()
            .unwrap()
            .entry(kind)
            .or_default()
            .push_back(answer);
    }

    fn hold_return(&self, kind: JobKind, answer: Option<Envelope<MutationDataDto>>) -> JobGate {
        let gate = Arc::new(Gate::default());
        self.returned
            .lock()
            .unwrap()
            .entry(kind)
            .or_default()
            .push_back(Returned {
                gate: gate.clone(),
                answer,
            });
        JobGate(gate)
    }

    /// Holds the next job of `kind` on the worker thread once it has
    /// reported `stage`, which the UI has then been sent, until the gate
    /// is released or dropped. The library goes on only after: Stop
    /// pressed meanwhile lands at its next safe point. The library may
    /// hold the repository lease while the job is held (L3 reports
    /// `integrating` under it), so a test does not open a second
    /// repository operation until it releases the gate.
    #[must_use = "dropping the handle releases the hold"]
    pub(crate) fn hold_at_stage(&self, kind: JobKind, stage: ProgressStage) -> JobGate {
        let gate = Arc::new(Gate::default());
        self.stages
            .lock()
            .unwrap()
            .entry((kind, stage))
            .or_default()
            .push_back(gate.clone());
        JobGate(gate)
    }

    /// A canned prepare answer for `command` (`Mutation::command`, as
    /// "ticket close"): the next prepare of that command still calls the
    /// library, and `answer` stands in place of its answer. A prepare of
    /// any other command takes none. Canned answers for one command
    /// stack: each prepare takes the oldest.
    pub(crate) fn answer_next_prepare(
        &self,
        command: &'static str,
        answer: Envelope<PrepareDataDto>,
    ) {
        self.prepares.lock().unwrap().push((command, answer));
    }

    /// A canned conflict read: the next conflict read of
    /// `conflict.operation_id` still calls the library, and `conflict`
    /// stands in place of its answer.
    pub(crate) fn answer_next_conflict_read(&self, conflict: ConflictDto) {
        self.conflicts.lock().unwrap().push(conflict);
    }

    /// Puts `service`, a real service the test opened over the same data
    /// directory, in place of the one in use: what the library tests do
    /// when they open a service with a failure point, and reopen it with
    /// `open_at` before a retry. Call it while no job runs.
    pub(crate) fn replace_service(&self, service: RepositoryService) {
        let slot = self
            .replacements
            .iter()
            .find(|slot| slot.get().is_none())
            .expect("too many service replacements in one test");
        assert!(slot.set(service).is_ok());
    }

    /// How many reads of `kind` were started: each read and prepare asks
    /// its gate once, before its first library call; its busy retries do
    /// not ask again.
    pub(crate) fn reads_seen(&self, kind: ReadKind) -> usize {
        self.reads_seen
            .lock()
            .unwrap()
            .iter()
            .filter(|seen| **seen == kind)
            .count()
    }

    /// Every read kind asked for so far, in the order the reads reached
    /// their gates: only observed.
    pub(crate) fn reads_in_order(&self) -> Vec<ReadKind> {
        self.reads_seen.lock().unwrap().clone()
    }

    /// The job kinds the worker has asked to run, in order. A job that
    /// was removed or not sent never appears.
    pub(crate) fn jobs_seen(&self) -> Vec<JobKind> {
        self.seen.lock().unwrap().clone()
    }
}

impl Backend for DelayingBackend {
    fn service(&self) -> &RepositoryService {
        self.replacements
            .iter()
            .rev()
            .find_map(OnceLock::get)
            .unwrap_or(&self.service)
    }

    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        KeyStore::for_home(&self.home).map_err(|_| KeyStoreUnavailable)
    }

    fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable> {
        self.drafts.as_ref().map_err(|unavailable| *unavailable)
    }

    fn resolutions(&self) -> Result<&ResolutionStore, ResolutionStoreError> {
        self.resolutions.as_ref().map_err(|error| *error)
    }

    fn layout(&self) -> Result<&LayoutStore, LayoutStoreError> {
        self.layout.as_ref().map_err(|error| *error)
    }

    fn read_gate(&self, kind: ReadKind) -> Option<ReadGate> {
        self.reads_seen.lock().unwrap().push(kind);
        self.reads.lock().unwrap().get_mut(&kind)?.pop_front()
    }

    fn before_job(&self, kind: JobKind) {
        self.seen.lock().unwrap().push(kind);
        let gate = self
            .jobs
            .lock()
            .unwrap()
            .get_mut(&kind)
            .and_then(VecDeque::pop_front);
        if let Some(gate) = gate {
            gate.wait();
        }
    }

    fn instead_of_job(&self, kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        self.instead
            .lock()
            .unwrap()
            .get_mut(&kind)
            .and_then(VecDeque::pop_front)
    }

    fn pending_operations(&self, root: &std::path::Path) -> Option<OperationListDto> {
        let canned = self.pending.lock().unwrap().clone();
        match canned {
            Some(list) => list,
            None => {
                let repo = self.service().resolve_repository(root).ok()?;
                self.service().list_operations(&repo).ok()
            }
        }
    }

    fn after_job(&self, kind: JobKind) -> Option<Envelope<MutationDataDto>> {
        let returned = self
            .returned
            .lock()
            .unwrap()
            .get_mut(&kind)
            .and_then(VecDeque::pop_front)?;
        returned.gate.wait();
        returned.answer
    }

    fn on_progress(&self, kind: JobKind, stage: ProgressStage) {
        let gate = self
            .stages
            .lock()
            .unwrap()
            .get_mut(&(kind, stage))
            .and_then(VecDeque::pop_front);
        if let Some(gate) = gate {
            gate.wait();
        }
    }

    fn after_prepare(&self, mutation: &Mutation) -> Option<Envelope<PrepareDataDto>> {
        let mut prepares = self.prepares.lock().unwrap();
        let command = mutation.command();
        let at = prepares.iter().position(|(of, _)| *of == command)?;
        Some(prepares.remove(at).1)
    }

    /// A conflict read takes the canned conflict; the pending rows' probe
    /// only looks at it, so the sheet that follows still reads it (Task 16
    /// re-review, M-3).
    fn after_conflict_read(
        &self,
        kind: ReadKind,
        operation_id: &OperationId,
    ) -> Option<ConflictDto> {
        let mut conflicts = self.conflicts.lock().unwrap();
        let wanted = operation_id.to_string();
        let at = conflicts
            .iter()
            .position(|conflict| conflict.operation_id == wanted)?;
        match kind {
            ReadKind::PendingConflicts => Some(conflicts[at].clone()),
            _ => Some(conflicts.remove(at)),
        }
    }
}

/// A hold after a job's library call, and the answer that stands in place
/// of the library's, if any.
struct Returned {
    gate: Arc<Gate>,
    answer: Option<Envelope<MutationDataDto>>,
}

#[derive(Default)]
struct GateState {
    reached: bool,
    released: bool,
}

#[derive(Default)]
pub(super) struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
}

impl Gate {
    /// The worker side: says it is here, then waits for the release.
    pub(super) fn wait(&self) {
        let mut state = self.state.lock().unwrap();
        state.reached = true;
        self.changed.notify_all();
        while !state.released {
            state = self.changed.wait(state).unwrap();
        }
    }

    pub(super) fn release(&self) {
        self.state.lock().unwrap().released = true;
        self.changed.notify_all();
    }

    /// Blocks until the worker is at the gate, for at most `limit`.
    pub(super) fn wait_reached(&self, limit: Duration) -> bool {
        let state = self.state.lock().unwrap();
        let (state, _) = self
            .changed
            .wait_timeout_while(state, limit, |state| !state.reached)
            .unwrap();
        state.reached
    }
}

/// A held job. The worker thread blocks at the gate until `release` is
/// called or the gate is dropped, so a failed test cannot leave the worker
/// waiting: before the job's library call for `hold_job`, after it returns
/// for `hold_job_return`, and once the named stage has been sent to the UI
/// for `hold_at_stage`.
#[must_use = "dropping the handle releases the hold"]
pub(crate) struct JobGate(pub(super) Arc<Gate>);

impl JobGate {
    pub(crate) fn release(&self) {
        self.0.release();
    }

    /// Blocks the test until the worker thread has reached this gate:
    /// for `hold_job` the job is running and not yet accepted; for
    /// `hold_at_stage` it is past acceptance, its stage already sent; for
    /// `hold_job_return` its library call has returned. Panics after the
    /// shared finite worker wait; GPUI's per-park limit is independent.
    pub(crate) fn wait_reached(&self) {
        assert!(
            self.0.wait_reached(super::WAIT),
            "the worker did not reach the held job"
        );
    }
}

impl Drop for JobGate {
    fn drop(&mut self) {
        self.0.release();
    }
}
