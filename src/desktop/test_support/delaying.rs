//! The test backend: a real service over a fixture, whose reads and jobs a
//! test can hold.
//!
//! It only delays. It never fails or fabricates a call; failures in tests
//! come from the library's own failure points and hooks.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
};

use futures::channel::oneshot;
use manyhands::repository::{RepositoryService, keys::KeyStore};

use crate::desktop::services::{Backend, JobKind, KeyStoreUnavailable, ReadGate, ReadKind};

pub(crate) struct DelayingBackend {
    service: Arc<RepositoryService>,
    home: PathBuf,
    reads: Mutex<HashMap<ReadKind, VecDeque<ReadGate>>>,
    jobs: Mutex<HashMap<JobKind, VecDeque<Arc<Gate>>>>,
}

impl DelayingBackend {
    /// A backend over `service`, with its key store in `home`.
    pub(crate) fn new(service: Arc<RepositoryService>, home: PathBuf) -> Self {
        Self {
            service,
            home,
            reads: Mutex::default(),
            jobs: Mutex::default(),
        }
    }

    /// Holds the next read of `kind` at its gate until the returned sender
    /// sends, or is dropped. Holds stack: each read takes the oldest.
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
    #[allow(dead_code)] // The worker arrives with Task 3.
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
struct Gate {
    released: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn wait(&self) {
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.changed.wait(released).unwrap();
        }
    }

    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

/// A held job. The worker thread blocks before the job's library call
/// until `release` is called or the gate is dropped, so a failed test
/// cannot leave the worker waiting.
pub(crate) struct JobGate(Arc<Gate>);

impl JobGate {
    #[allow(dead_code)] // The worker arrives with Task 3.
    pub(crate) fn release(&self) {
        self.0.release();
    }
}

impl Drop for JobGate {
    fn drop(&mut self) {
        self.0.release();
    }
}
