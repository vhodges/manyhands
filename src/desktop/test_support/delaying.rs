//! The test backend: a real service over a fixture, whose reads and jobs a
//! test can hold.
//!
//! It only delays. It never fails or fabricates a call; failures in tests
//! come from the library's own failure points and hooks.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use futures::channel::oneshot;
use manyhands::repository::{RepositoryService, keys::KeyStore};

use crate::desktop::services::{Backend, JobKind, KeyStoreUnavailable, ReadGate, ReadKind};

pub(crate) struct DelayingBackend {
    service: Arc<RepositoryService>,
    home: PathBuf,
    reads: Mutex<HashMap<ReadKind, VecDeque<ReadGate>>>,
    jobs: Mutex<HashMap<JobKind, VecDeque<Arc<Gate>>>>,
    /// Every job kind the worker reached `before_job` with, in order.
    seen: Mutex<Vec<JobKind>>,
}

impl DelayingBackend {
    /// A backend over `service`, with its key store in `home`.
    pub(crate) fn new(service: Arc<RepositoryService>, home: PathBuf) -> Self {
        Self {
            service,
            home,
            reads: Mutex::default(),
            jobs: Mutex::default(),
            seen: Mutex::default(),
        }
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

    /// The job kinds the worker has asked to run, in order. A job that
    /// was removed or not sent never appears.
    pub(crate) fn jobs_seen(&self) -> Vec<JobKind> {
        self.seen.lock().unwrap().clone()
    }
}

impl Backend for DelayingBackend {
    fn service(&self) -> &RepositoryService {
        &self.service
    }

    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        KeyStore::for_home(&self.home).map_err(|_| KeyStoreUnavailable)
    }

    fn read_gate(&self, kind: ReadKind) -> Option<ReadGate> {
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
}

#[derive(Default)]
struct GateState {
    reached: bool,
    released: bool,
}

#[derive(Default)]
struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
}

impl Gate {
    /// The worker side: says it is here, then waits for the release.
    fn wait(&self) {
        let mut state = self.state.lock().unwrap();
        state.reached = true;
        self.changed.notify_all();
        while !state.released {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn release(&self) {
        self.state.lock().unwrap().released = true;
        self.changed.notify_all();
    }

    /// Blocks until the worker is at the gate, for at most `limit`.
    fn wait_reached(&self, limit: Duration) -> bool {
        let state = self.state.lock().unwrap();
        let (state, _) = self
            .changed
            .wait_timeout_while(state, limit, |state| !state.reached)
            .unwrap();
        state.reached
    }
}

/// A held job. The worker thread blocks before the job's library call
/// until `release` is called or the gate is dropped, so a failed test
/// cannot leave the worker waiting.
#[must_use = "dropping the handle releases the hold"]
pub(crate) struct JobGate(Arc<Gate>);

impl JobGate {
    pub(crate) fn release(&self) {
        self.0.release();
    }

    /// Blocks the test until the worker thread has reached this gate, so
    /// the job is running and not yet accepted. Panics after ten seconds,
    /// inside the scheduler's fifteen.
    pub(crate) fn wait_reached(&self) {
        assert!(
            self.0.wait_reached(Duration::from_secs(10)),
            "the worker did not reach the held job"
        );
    }
}

impl Drop for JobGate {
    fn drop(&mut self) {
        self.0.release();
    }
}
