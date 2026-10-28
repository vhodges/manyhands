//! The service layer views read and act through, and the `Backend` seam
//! beneath it.
//!
//! Production uses `LibraryBackend`, whose hooks do nothing. Tests use
//! `test_support::delaying::DelayingBackend`, which only delays: it never
//! fails or fabricates a call. Failures in tests come from the library's
//! own failure points and hooks.

pub(crate) mod drafts;
pub(crate) mod images;
pub(crate) mod links;
pub(crate) mod prompts;
mod reads;
pub(crate) mod requests;
pub(crate) mod worker;

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use gpui_kit::{App, AppContext, Entity};
use manyhands::{
    drafts::{DraftError, DraftStore},
    repository::{MutationDataDto, RepositoryService, keys::KeyStore},
    results::{Envelope, Outcome},
};

use crate::desktop::model::operations::Operations;
use drafts::DraftRegistry;
use worker::{WorkerEvent, WorkerHandle};

pub(crate) use drafts::DraftStoreUnavailable;

pub(crate) use reads::{ReadFailure, ReadResult, Reader};

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
    /// `inspect_repository`, of a folder the Repositories page checks.
    Inspect,
    Remotes,
    Identity,
    Keys,
    Hosts,
    Prepare,
    /// The draft store's listing and claims.
    Drafts,
    /// A draft's revision written to the store.
    DraftWrite,
    /// The registry's re-read of an item after a save, apart from `Item`,
    /// which the repository's tabs reload under at the same time.
    DraftItem,
    /// `observe_path`: the path a clicked link names, and any other path
    /// the desktop asks the library about.
    ObservePath,
    /// `new_id`, for a new item's ID.
    #[allow(dead_code)] // Called from Task 22; no test builds it.
    NewId,
    /// The check of a relationship a draft sets, before it is saved.
    #[allow(dead_code)] // Called from Task 21; no test builds it.
    RelationshipCheck,
    /// A local image an editor shows, read from the tab's trees and
    /// decoded (`images`): no library call, but held like one.
    Image,
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
    DocumentSave,
    TicketSave,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 14.
    ContextPrepare,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 17.
    ContextRelease,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 22.
    DocumentCreate,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 22.
    TicketCreate,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 24.
    DocumentMove,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 24.
    FolderCreate,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 25.
    DocumentRepair,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 25.
    TicketRepair,
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

    /// The draft store, opened in the application data directory in
    /// production and in the fixture's in tests; otherwise why it did not
    /// open.
    fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable>;

    /// Awaited inside the read's future, before the blocking library call.
    fn read_gate(&self, _kind: ReadKind) -> Option<ReadGate> {
        None
    }

    /// Called on the worker thread before a job's library call.
    fn before_job(&self, _kind: JobKind) {}

    /// Called on the worker thread after a job's library call returns,
    /// before the worker decides what follows it.
    fn after_job(&self, _kind: JobKind) {}
}

/// The production backend: the library, with no delays.
pub(crate) struct LibraryBackend {
    service: RepositoryService,
    drafts: Result<DraftStore, DraftStoreUnavailable>,
}

impl LibraryBackend {
    /// A backend over `service` and the draft store's open result: the
    /// desktop starts whether or not the store opened.
    pub(crate) fn new(service: RepositoryService, drafts: Result<DraftStore, DraftError>) -> Self {
        Self {
            service,
            drafts: drafts.map_err(DraftStoreUnavailable::from),
        }
    }
}

impl Backend for LibraryBackend {
    fn service(&self) -> &RepositoryService {
        &self.service
    }

    fn key_store(&self) -> Result<KeyStore, KeyStoreUnavailable> {
        KeyStore::for_current_user().map_err(|_| KeyStoreUnavailable)
    }

    fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable> {
        self.drafts.as_ref().map_err(|unavailable| *unavailable)
    }
}

/// The handle views hold. Cloning it shares the backend, the worker
/// and the request model.
#[derive(Clone)]
pub(crate) struct Services {
    backend: Arc<dyn Backend>,
    worker: WorkerHandle,
    operations: Entity<Operations>,
    /// The drafts this session knows of, shared by every view.
    drafts: Entity<DraftRegistry>,
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
    /// worker does, after the last handle is dropped. The draft store is
    /// listed once, at launch.
    ///
    /// Each finished request is then passed to the draft registry, after
    /// the request model has it: a save's answer applies to its draft, and
    /// an abandoned operation ends the pending save it held. The registry
    /// outlives tabs, so a save that finishes after its tab closed still
    /// retires its revision.
    pub(crate) fn start(backend: Arc<dyn Backend>, cx: &mut App) -> Self {
        let (worker, mut events) = WorkerHandle::spawn(backend.clone());
        let operations = cx.new(|_| Operations::default());
        let model = operations.downgrade();
        let drafts = cx.new(|_| DraftRegistry::new(backend.clone(), worker.clone()));
        let registry = drafts.downgrade();
        cx.spawn(async move |cx| {
            while let Some(event) = events.next().await {
                let mut batch = vec![event];
                while let Ok(event) = events.try_recv() {
                    batch.push(event);
                }
                let applied = model.update(cx, |operations, cx| {
                    let mut finished = Vec::new();
                    for event in batch {
                        if let WorkerEvent::Finished(id, envelope) = &event
                            && let Some(forward) = for_drafts(operations, *id, envelope)
                        {
                            finished.push(forward);
                        }
                        operations.apply(event);
                    }
                    cx.notify();
                    finished
                });
                let Ok(finished) = applied else {
                    break;
                };
                if finished.is_empty() {
                    continue;
                }
                // The registry lives as long as the request model.
                let _ = registry.update(cx, |registry, cx| {
                    for forward in finished {
                        match forward {
                            ForDrafts::Save(id, envelope) => registry.finished(id, &envelope, cx),
                            ForDrafts::Abandoned(operation) => {
                                registry.operation_abandoned(&operation, cx)
                            }
                        }
                    }
                });
            }
        })
        .detach();
        let services = Self {
            backend,
            worker,
            operations,
            drafts: drafts.clone(),
            launch_refresh: true,
        };
        drafts.update(cx, |registry, cx| {
            registry.reload_listing(cx);
        });
        services
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

    /// The drafts this session knows of.
    pub(crate) fn drafts(&self) -> &Entity<DraftRegistry> {
        &self.drafts
    }

    #[cfg(test)]
    pub(crate) fn backend(&self) -> &Arc<dyn Backend> {
        &self.backend
    }
}

/// What a finished request means to the draft registry.
enum ForDrafts {
    /// A save finished: its answer applies to its draft.
    Save(requests::DesktopRequestId, Box<Envelope<MutationDataDto>>),
    /// An `operation abandon` ended this operation: a pending save that
    /// held it is over (ruling R22).
    Abandoned(String),
}

fn for_drafts(
    operations: &Operations,
    id: requests::DesktopRequestId,
    envelope: &Envelope<MutationDataDto>,
) -> Option<ForDrafts> {
    match &operations.request(id)?.intent {
        requests::Intent::DocumentSave { .. } | requests::Intent::TicketSave { .. } => {
            Some(ForDrafts::Save(id, Box::new(envelope.clone())))
        }
        requests::Intent::OperationAbandon { operation_id, .. }
            if matches!(envelope.outcome, Outcome::Success | Outcome::Noop) =>
        {
            Some(ForDrafts::Abandoned(operation_id.to_string()))
        }
        _ => None,
    }
}
