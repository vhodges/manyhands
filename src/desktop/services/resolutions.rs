//! Kept resolution sets (D5 design, B3; decision 19; R-12).
//!
//! Before a Save resolution is sent, the set it submits is written to
//! `<data_dir>/resolutions`, one file per library request ID, through the
//! library's protected folder by name (L4, decision 22): the drafts'
//! protection, owner-only files, and an atomic replace. A set is kept until
//! its request finishes, so a Retry after an `original_request_required`
//! answer, after a restart too, sends the same set under the same request
//! ID (P24).
//!
//! A set is deleted when its request is seen finished: after its answer
//! (`Services::start`'s drain) and, at launch, by the sweep, which asks
//! `show_request` of every kept set and deletes the finished ones and
//! those the library has no record of.
//!
//! Nothing here derives `Debug` or `Display` over a set: it holds item text.
//! The store's errors carry no path and no text, and say "resolutions
//! folder", never the drafts' words.

use std::{
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use gpui_kit::{App, AsyncApp, Task};
use manyhands::{
    canonical::ItemId,
    drafts::{DraftError, DraftErrorKind, folder::Folder},
    repository::{OperationId, RepositoryService, RequestId, RequestState},
    results::ResultCode,
};
use serde::{Deserialize, Serialize};

use super::{
    Backend, Services,
    conflicts::ConflictSource,
    requests::{DesktopRequestId, Intent, Lane, PathResolution},
};

/// The folder's name in the data directory.
const RESOLUTIONS: &str = "resolutions";

/// A kept file's extension.
const EXTENSION: &str = ".json";

/// The format a kept file is written in.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
const VERSION: u32 = 1;

/// Why the resolutions store failed. It carries no path and no text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ResolutionStoreError {
    /// The platform gave no application data directory.
    Unavailable,
    /// A link, a reparse point, another owner or an other-writable parent.
    ProtectionUnavailable,
    /// Permission was denied.
    Access,
    /// The disk or the quota is full.
    NoSpace,
    /// A kept file is not a set this version reads.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
    Damaged,
    /// Anything else.
    Io,
}

impl fmt::Display for ResolutionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "No application data directory is available for kept resolutions.",
            Self::ProtectionUnavailable => "The resolutions folder could not be protected.",
            Self::Access => "Access to the resolutions folder was denied.",
            Self::NoSpace => "There is no space left to keep the resolution.",
            Self::Damaged => "A kept resolution could not be read.",
            Self::Io => "The resolutions folder could not be read or written.",
        })
    }
}

/// The set a Save resolution submits, kept by its request ID: which
/// synchronization it resolves, and every path's result. It derives
/// neither `Debug` nor `Display`.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
#[derive(Clone)]
pub(crate) struct KeptSet {
    pub request_id: RequestId,
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub item: ItemId,
    /// Remote or local: which intent the set is sent with.
    pub source: ConflictSource,
    /// The read's observation the set was chosen against.
    pub observation: String,
    pub resolutions: Vec<PathResolution>,
}

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
impl KeptSet {
    /// The intent that submits this set: `ConflictResolve` for a remote
    /// conflict, `LocalContextResolve` for a local one.
    pub(crate) fn intent(&self) -> Intent {
        let root = self.root.clone();
        let operation_id = self.operation_id;
        let observation = self.observation.clone();
        let resolutions = self.resolutions.clone();
        match self.source {
            ConflictSource::Remote => Intent::ConflictResolve {
                root,
                operation_id,
                observation,
                resolutions,
            },
            ConflictSource::Local => Intent::LocalContextResolve {
                root,
                operation_id,
                observation,
                resolutions,
            },
        }
    }
}

/// A kept set as it is written: a set of this file's version, its IDs as
/// their canonical text.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    version: u32,
    request_id: String,
    root: PathBuf,
    operation_id: String,
    item: String,
    kind: StoredKind,
    observation: String,
    paths: Vec<StoredPath>,
}

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredKind {
    Remote,
    Local,
}

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPath {
    token: String,
    source: String,
}

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
impl Stored {
    fn of(set: &KeptSet) -> Self {
        Self {
            version: VERSION,
            request_id: set.request_id.to_string(),
            root: set.root.clone(),
            operation_id: set.operation_id.to_string(),
            item: set.item.to_string(),
            kind: match set.source {
                ConflictSource::Remote => StoredKind::Remote,
                ConflictSource::Local => StoredKind::Local,
            },
            observation: set.observation.clone(),
            paths: set
                .resolutions
                .iter()
                .map(|path| StoredPath {
                    token: path.token.clone(),
                    source: path.source.clone(),
                })
                .collect(),
        }
    }

    /// The set, when this is one of this version's kept under `request`.
    fn into_set(self, request: RequestId) -> Option<KeptSet> {
        if self.version != VERSION || self.request_id != request.to_string() {
            return None;
        }
        Some(KeptSet {
            request_id: request,
            root: self.root,
            operation_id: OperationId::parse(&self.operation_id).ok()?,
            item: self.item.parse().ok()?,
            source: match self.kind {
                StoredKind::Remote => ConflictSource::Remote,
                StoredKind::Local => ConflictSource::Local,
            },
            observation: self.observation,
            resolutions: self
                .paths
                .into_iter()
                .map(|path| PathResolution {
                    token: path.token,
                    source: path.source,
                })
                .collect(),
        })
    }
}

/// The kept sets, in `<data_dir>/resolutions`. Cloning it shares the open
/// folder.
#[derive(Clone)]
pub(crate) struct ResolutionStore {
    folder: Arc<Mutex<Folder>>,
}

impl ResolutionStore {
    /// `<data>/resolutions`, made or checked and held open: for tests,
    /// which give the fixture's data directory, as the draft store's
    /// `open_in`.
    #[cfg(test)]
    pub(crate) fn open_in(data: &std::path::Path) -> Result<Self, ResolutionStoreError> {
        Ok(Self::over(Folder::open_named(data, RESOLUTIONS)?))
    }

    /// `resolutions` in the platform's application data directory, beside
    /// the draft store's `drafts`.
    pub(crate) fn open_default() -> Result<Self, ResolutionStoreError> {
        Ok(Self::over(Folder::open_named_default(RESOLUTIONS)?))
    }

    fn over(folder: Folder) -> Self {
        Self {
            folder: Arc::new(Mutex::new(folder)),
        }
    }

    fn folder(&self) -> std::sync::MutexGuard<'_, Folder> {
        // A panic while holding the folder leaves it as usable as before.
        self.folder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Writes `set` under its request ID, replacing any set kept under it.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
    pub(crate) fn keep(&self, set: &KeptSet) -> Result<(), ResolutionStoreError> {
        let bytes = serde_json::to_vec(&Stored::of(set)).map_err(|_| ResolutionStoreError::Io)?;
        self.folder().replace(&file_name(set.request_id), &bytes)?;
        Ok(())
    }

    /// The set kept under `request`, if any.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
    pub(crate) fn kept(&self, request: RequestId) -> Result<Option<KeptSet>, ResolutionStoreError> {
        let bytes = match self.folder().read(&file_name(request)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind == DraftErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        serde_json::from_slice::<Stored>(&bytes)
            .ok()
            .and_then(|stored| stored.into_set(request))
            .map(Some)
            .ok_or(ResolutionStoreError::Damaged)
    }

    /// The request IDs of every kept set, in order. A file of any other
    /// name is left out.
    pub(crate) fn requests(&self) -> Result<Vec<RequestId>, ResolutionStoreError> {
        Ok(self
            .folder()
            .list()?
            .iter()
            .filter_map(|name| RequestId::parse(name.strip_suffix(EXTENSION)?).ok())
            .collect())
    }

    /// Deletes the set kept under `request`; none kept is not a failure.
    pub(crate) fn forget(&self, request: RequestId) -> Result<(), ResolutionStoreError> {
        match self.folder().remove(&file_name(request)) {
            Err(error) if error.kind != DraftErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// Asks `show_request` of `request` and deletes its set when the
    /// request is finished or the library has no record of it. A failed
    /// read keeps the set. True when the set was deleted.
    pub(crate) fn settle(&self, service: &RepositoryService, request: RequestId) -> bool {
        let shown = service.show_request(request);
        let over = match &shown.data {
            Some(record) => record.state == RequestState::Finished,
            None => shown.code == ResultCode::RequestNotFound,
        };
        over && self.forget(request).is_ok()
    }

    /// The launch sweep: `settle` of every kept set.
    pub(crate) fn sweep(&self, service: &RepositoryService) {
        for request in self.requests().unwrap_or_default() {
            self.settle(service, request);
        }
    }
}

/// A kept set's file name.
fn file_name(request: RequestId) -> String {
    format!("{request}{EXTENSION}")
}

impl From<DraftError> for ResolutionStoreError {
    fn from(error: DraftError) -> Self {
        match error.kind {
            DraftErrorKind::Unavailable => Self::Unavailable,
            DraftErrorKind::ProtectionUnavailable => Self::ProtectionUnavailable,
            DraftErrorKind::Access => Self::Access,
            DraftErrorKind::NoSpace => Self::NoSpace,
            DraftErrorKind::OpenElsewhere
            | DraftErrorKind::HeldHere
            | DraftErrorKind::NotFound
            | DraftErrorKind::Io => Self::Io,
        }
    }
}

impl Services {
    /// Keeps `set`, then submits it under its own request ID, on the
    /// background executor: nothing is sent when it could not be kept.
    /// Dropping the task before it ends may leave the set kept and unsent;
    /// the launch sweep deletes it, as the library has no record of it.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 11b.
    pub(crate) fn submit_resolution(
        &self,
        set: KeptSet,
        cx: &mut App,
    ) -> Task<Result<DesktopRequestId, ResolutionStoreError>> {
        let backend = self.backend.clone();
        let worker = self.worker.clone();
        cx.background_executor().spawn(async move {
            backend.resolutions()?.keep(&set)?;
            Ok(worker.submit_with_request_id(set.intent(), Lane::User, set.request_id))
        })
    }
}

/// The launch sweep, on the background executor: `ResolutionStore::sweep`
/// over the backend's store, when it opened.
pub(super) fn sweep_at_launch(backend: &Arc<dyn Backend>, cx: &App) {
    let backend = backend.clone();
    cx.background_executor()
        .spawn(async move {
            if let Ok(store) = backend.resolutions() {
                store.sweep(backend.service());
            }
        })
        .detach();
}

/// `ResolutionStore::settle` of each of `requests`, whose resolutions
/// have just answered, on the background executor.
pub(super) fn settle_finished(backend: &Arc<dyn Backend>, requests: Vec<RequestId>, cx: &AsyncApp) {
    let backend = backend.clone();
    cx.background_executor()
        .spawn(async move {
            if let Ok(store) = backend.resolutions() {
                for request in requests {
                    store.settle(backend.service(), request);
                }
            }
        })
        .detach();
}

/// Whether `intent` submits a kept set.
pub(super) fn submits_a_set(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::ConflictResolve { .. } | Intent::LocalContextResolve { .. }
    )
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resolutions_tests.rs"]
mod tests;
