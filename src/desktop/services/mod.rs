//! The service layer views read and act through, and the `Backend` seam
//! beneath it.
//!
//! Production uses `LibraryBackend`, whose hooks do nothing. Tests use
//! `test_support::delaying::DelayingBackend`, which only delays: it never
//! fails or fabricates a call. Failures in tests come from the library's
//! own failure points and hooks.

mod reads;

use std::sync::Arc;

use manyhands::repository::{RepositoryService, keys::KeyStore};

pub(crate) use reads::{ReadFailure, ReadResult};

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
#[allow(dead_code)] // The worker that runs jobs arrives with Task 3.
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
#[allow(dead_code)] // The Keys page arrives with Task 15.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct KeyStoreUnavailable;

#[allow(dead_code)] // The Keys page arrives with Task 15.
pub(crate) const KEY_STORE_UNAVAILABLE: &str = "Manyhands could not find your SSH key store.";

/// What the desktop calls the library through.
pub(crate) trait Backend: Send + Sync + 'static {
    fn service(&self) -> &RepositoryService;

    /// The current user's key store in production; the fixture home's in
    /// tests.
    #[allow(dead_code)] // The Keys page arrives with Task 15.
    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable>;

    /// Awaited inside the read's future, before the blocking library call.
    fn read_gate(&self, _kind: ReadKind) -> Option<ReadGate> {
        None
    }

    /// Called on the worker thread before a job's library call.
    #[allow(dead_code)] // The worker arrives with Task 3.
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

/// The handle views hold. Cloning it shares the backend.
#[derive(Clone)]
pub(crate) struct Services {
    backend: Arc<dyn Backend>,
}

impl Services {
    pub(crate) fn new(backend: Arc<dyn Backend>) -> Self {
        Self { backend }
    }

    #[cfg(test)]
    pub(crate) fn backend(&self) -> &Arc<dyn Backend> {
        &self.backend
    }
}
