//! The service layer views read and act through, and the `Backend` seam
//! beneath it.
//!
//! Production uses `LibraryBackend`, whose hooks do nothing. Tests use
//! `test_support::delaying::DelayingBackend`, which only delays: it never
//! fails or fabricates a call. Failures in tests come from the library's
//! own failure points and hooks.

mod reads;
pub(crate) mod requests;
pub(crate) mod worker;

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use gpui_kit::{App, AppContext, Entity};
use manyhands::repository::{RepositoryService, keys::KeyStore};

use crate::desktop::model::operations::Operations;
use worker::WorkerHandle;

pub(crate) use reads::{ReadFailure, ReadResult};

/// The waits before each retry of a call answered `busy`: a read, a
/// prepare, or a worker job of either lane (rulings R6, R-T7a, R-B3).
pub(crate) const BUSY_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
];

/// A read's delay: awaited inside the read's future, before the blocking
/// library call. It must never block: under `#[gpui_kit::test]` the
/// background executor runs on the test thread.
pub(crate) type ReadGate = futures::channel::oneshot::Receiver<()>;

/// What a read asks for, so the seam can hold one kind and not another.
#[allow(dead_code)] // Most kinds are read by the views of later tasks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ReadKind {
    Repositories,
    Resolve,
    Documents,
    Folders,
    Tickets,
    Item,
    Comments,
    Dependencies,
    Children,
    FindSlug,
    Operations,
    IndexStatus,
    Remotes,
    Identity,
    Keys,
    Hosts,
    Prepare,
}

/// What a worker job does, so the seam can hold one kind and not another.
/// One per desktop intent, and the host observation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum JobKind {
    RepoCreate,
    RepoEnable,
    RepoRemove,
    RepoIdentitySet,
    RemoteAdd,
    RemoteRemove,
    RemoteSelect,
    IndexRefresh,
    IndexRebuild,
    KeyGenerate,
    KeyImport,
    KeySelect,
    KeyClear,
    KeyRemove,
    KeyDelete,
    HostApprove,
    HostReplace,
    OperationResume,
    OperationAbandon,
    ObserveHost,
}

/// The key store could not be found. The Keys page shows
/// `KEY_STORE_UNAVAILABLE`; the error itself is never shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct KeyStoreUnavailable;

pub(crate) const KEY_STORE_UNAVAILABLE: &str = "Manyhands could not find your SSH key store.";

/// What the desktop calls the library through.
pub(crate) trait Backend: Send + Sync + 'static {
    fn service(&self) -> &RepositoryService;

    /// The current user's key store in production; the fixture home's in
    /// tests.
    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable>;

    /// Awaited inside the read's future, before the blocking library call.
    fn read_gate(&self, _kind: ReadKind) -> Option<ReadGate> {
        None
    }

    /// Called on the worker thread before a job's library call.
    fn before_job(&self, _kind: JobKind) {}
}

/// The production backend: the library, with no delays.
pub(crate) struct LibraryBackend {
    service: RepositoryService,
}

impl LibraryBackend {
    pub(crate) fn new(service: RepositoryService) -> Self {
        Self { service }
    }
}

impl Backend for LibraryBackend {
    fn service(&self) -> &RepositoryService {
        &self.service
    }

    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        KeyStore::for_current_user().map_err(|_| KeyStoreUnavailable)
    }
}

/// The handle views hold. Cloning it shares the backend, the worker
/// and the request model.
#[derive(Clone)]
pub(crate) struct Services {
    backend: Arc<dyn Backend>,
    worker: WorkerHandle,
    operations: Entity<Operations>,
    /// Whether the workspace refreshes every listed repository once its
    /// list first loads. Tests that do not wait for the worker turn it
    /// off.
    launch_refresh: bool,
}

impl Services {
    /// Starts the mutation worker over `backend`, and the foreground task
    /// that applies its events to the request model. Every event waiting
    /// is applied in one update with one notify, so the views read what a
    /// batch changed once, not once per event. The task ends when the
    /// worker does, after the last handle is dropped.
    pub(crate) fn start(backend: Arc<dyn Backend>, cx: &mut App) -> Self {
        let (worker, mut events) = WorkerHandle::spawn(backend.clone());
        let operations = cx.new(|_| Operations::default());
        let model = operations.downgrade();
        cx.spawn(async move |cx| {
            while let Some(event) = events.next().await {
                let mut batch = vec![event];
                while let Ok(event) = events.try_recv() {
                    batch.push(event);
                }
                let applied = model.update(cx, |operations, cx| {
                    for event in batch {
                        operations.apply(event);
                    }
                    cx.notify();
                });
                if applied.is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            backend,
            worker,
            operations,
            launch_refresh: true,
        }
    }

    /// The same services, with no refresh at launch: for a test that
    /// does not wait for the worker.
    #[cfg(test)]
    pub(crate) fn without_launch_refresh(mut self) -> Self {
        self.launch_refresh = false;
        self
    }

    pub(crate) fn launch_refresh(&self) -> bool {
        self.launch_refresh
    }

    /// The key store a key command is built with.
    pub(crate) fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        self.backend.key_store()
    }

    pub(crate) fn worker(&self) -> &WorkerHandle {
        &self.worker
    }

    /// This session's requests, as the worker's events left them.
    pub(crate) fn operations(&self) -> &Entity<Operations> {
        &self.operations
    }

    #[cfg(test)]
    pub(crate) fn backend(&self) -> &Arc<dyn Backend> {
        &self.backend
    }
}
