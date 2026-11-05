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
//! A set is deleted once nothing can finish with it (R-12, R-25): after
//! its answer (`Services::start`'s drain), when an answer proves it, and,
//! at launch, by the sweep, which asks `show_request` of every kept set.
//! A finished request's set goes. One the library has no record of goes
//! when it is local; a remote one is kept while its conflict still names
//! its request as the unfinished attempt's (`ResolutionStore::settle`). A
//! set kept under a request ID is never replaced by a different one.
//!
//! Nothing here derives `Debug` or `Display` over a set: it holds item text.
//! The store's errors carry no path and no text, and say "resolutions
//! folder", never the drafts' words.

use std::{
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use manyhands::{
    canonical::ItemId,
    drafts::{DraftError, DraftErrorKind, folder::Folder},
    repository::{OperationId, RequestId, RequestState},
    results::{CheckpointEffect, ResultCode},
};
use serde::{Deserialize, Serialize};

use super::{
    Backend,
    conflicts::ConflictSource,
    requests::{Intent, PathResolution},
};

/// The folder's name in the data directory.
const RESOLUTIONS: &str = "resolutions";

/// A kept file's extension.
const EXTENSION: &str = ".json";

/// The format a kept file is written in.
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
    /// A different set is already kept under the request ID: a request is
    /// only ever sent again with the set it was first sent with.
    Differs,
    /// A kept file was written by a newer build, in a format this one
    /// does not read.
    Newer,
    /// A kept file is not a set this version reads.
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
            Self::Differs => "A different resolution is already kept for this request.",
            Self::Newer => "A kept resolution was saved by a newer version of Manyhands.",
            Self::Damaged => "A kept resolution could not be read.",
            Self::Io => "The resolutions folder could not be read or written.",
        })
    }
}

/// The set a Save resolution submits, kept by its request ID: which
/// synchronization it resolves, and every path's result. It derives
/// neither `Debug` nor `Display`.
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredKind {
    Remote,
    Local,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPath {
    token: String,
    source: String,
}

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

    /// Writes `set` under its request ID. A request is only ever sent
    /// again with the set it was first sent with (the library answers
    /// another input under the same ID `request_mismatch`), so keeping the
    /// same set again changes nothing, and a different one under the same
    /// ID is refused with `Differs`, the kept one left as it was. A kept
    /// file that does not read as a set is refused with its own error
    /// (`Damaged` or `Newer`), and left as it was too. The
    /// folder is held from the look to the write: no two keeps of this
    /// process interleave.
    pub(crate) fn keep(&self, set: &KeptSet) -> Result<(), ResolutionStoreError> {
        let bytes = serde_json::to_vec(&Stored::of(set)).map_err(|_| ResolutionStoreError::Io)?;
        let name = file_name(set.request_id);
        let mut folder = self.folder();
        match folder.read(&name) {
            Ok(kept) if kept == bytes => Ok(()),
            // A file this build cannot read as a set is named as such.
            Ok(kept) => decode(&kept, set.request_id).and(Err(ResolutionStoreError::Differs)),
            Err(error) if error.kind == DraftErrorKind::NotFound => {
                folder.replace(&name, &bytes)?;
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    /// The set kept under `request`, if any.
    pub(crate) fn kept(&self, request: RequestId) -> Result<Option<KeptSet>, ResolutionStoreError> {
        let bytes = match self.folder().read(&file_name(request)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind == DraftErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        decode(&bytes, request).map(Some)
    }

    /// The newest set kept for `operation` in `root` under a request not in
    /// `except`, if any: one a Save resolution of that conflict sends
    /// again, under its own request ID (R-12). A file that does not read as
    /// a set is passed by.
    pub(crate) fn kept_for(
        &self,
        root: &std::path::Path,
        operation: OperationId,
        except: &[RequestId],
    ) -> Result<Option<KeptSet>, ResolutionStoreError> {
        Ok(self
            .requests()?
            .into_iter()
            .rev()
            .filter(|request| !except.contains(request))
            .filter_map(|request| self.kept(request).ok().flatten())
            .find(|set| set.operation_id == operation && set.root == root))
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
    /// request is finished, or when the library has no record of it and
    /// nothing else can finish with the set:
    ///
    /// - a local set: a local resolution that stops owing keeps its record
    ///   accepted, so no record means nothing stands;
    /// - a remote set (R-25): a `conflict resolve` keeps no record on any
    ///   stop, while its attempt, which only the same input finishes, may
    ///   stand. Its set is kept while `show_conflict` of its operation names
    ///   this request as the unfinished attempt's, and deleted once it does
    ///   not;
    /// - a set that does not parse (`Damaged`): no request can be sent with
    ///   it. One a newer build wrote (`Newer`) is kept: that build may
    ///   still send it.
    ///
    /// Any failed read keeps the set. True when the set was deleted.
    pub(crate) fn settle(&self, backend: &dyn Backend, request: RequestId) -> bool {
        let shown = backend.service().show_request(request);
        let over = match &shown.data {
            Some(record) => record.state == RequestState::Finished,
            None if shown.code == ResultCode::RequestNotFound => match self.kept(request) {
                Ok(Some(set)) => match set.source {
                    ConflictSource::Local => true,
                    ConflictSource::Remote => !attempt_stands(backend, &set),
                },
                Ok(None) | Err(ResolutionStoreError::Damaged) => true,
                Err(_) => false,
            },
            None => false,
        };
        over && self.forget(request).is_ok()
    }

    /// The launch sweep: `settle` of every kept set.
    pub(crate) fn sweep(&self, backend: &dyn Backend) {
        for request in self.requests().unwrap_or_default() {
            self.settle(backend, request);
        }
    }

    /// What a resolution's end means for its kept set (R-12, R-25).
    ///
    /// A remote answer is read as it is, since the request keeps no record
    /// on any stop (`bind/conflict.rs` `call`): the set goes on an answer
    /// that ends the attempt or proves this request has none, is settled
    /// on a `cancelled` before acceptance, and is kept on any other, for
    /// the launch sweep to settle (`remote_answer`). A local answer, and a
    /// request that never reached the library, are settled from the
    /// library's state.
    fn ended(&self, backend: &dyn Backend, ended: &Ended) {
        let remote = match (ended.source, ended.answer) {
            (ConflictSource::Remote, Some((code, checkpoint))) => remote_answer(code, checkpoint),
            _ => RemoteAnswer::Settle,
        };
        match remote {
            RemoteAnswer::Ends => {
                let _ = self.forget(ended.request);
            }
            RemoteAnswer::Settle => {
                self.settle(backend, ended.request);
            }
            RemoteAnswer::Keep => {}
        }
    }
}

/// What a remote resolution's answer means for its set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RemoteAnswer {
    /// No attempt is left that the set could still finish: it goes.
    Ends,
    /// The answer proves nothing of the attempt, and nothing ran: the set
    /// is settled as one the library has no record of (`settle`).
    Settle,
    /// The attempt may stand: the set is kept for the launch sweep.
    Keep,
}

/// What a remote resolution answered `code`, with the checkpoint effect
/// `checkpoint`, means for its set (R-25 as refined; `bind/conflict.rs`):
///
/// - `ok` and `already_applied`: the resolution is checkpointed and
///   published (`continued`). Ends.
/// - `cancelled` with the checkpoint `committed`: the call's token stopped
///   it after the checkpoint, the synchronization left resolved for
///   `operation resume` (`continued`). Ends.
/// - `cancelled` with nothing committed: the token was signalled before
///   acceptance (the requests panel's Cancel, or quitting), and
///   `execute.rs` stopped it with nothing run. It proves nothing of an
///   attempt an earlier call left: Settle.
/// - `original_request_required`: another request's attempt stands and
///   this one has none. It is answered before this request owns an
///   attempt (`fresh`, step 3), or after a refused supersede, once the
///   request is disowned of the attempt the domain never recorded. Ends.
///
/// Every other code is Keep: `external_change`, `invalid_input`,
/// `identity_required` and `recovery_required` are also answered on a
/// retry whose attempt has written paths and stands (`resolve`, the
/// `PathsApplying` and `RecoveryRequired` phases, through `reported`),
/// and a failure's effects show only what that call moved (`effects_of`),
/// so the answer cannot tell them from a refusal before any write.
fn remote_answer(code: ResultCode, checkpoint: CheckpointEffect) -> RemoteAnswer {
    match code {
        ResultCode::Ok | ResultCode::AlreadyApplied | ResultCode::OriginalRequestRequired => {
            RemoteAnswer::Ends
        }
        ResultCode::Cancelled if checkpoint == CheckpointEffect::Committed => RemoteAnswer::Ends,
        ResultCode::Cancelled => RemoteAnswer::Settle,
        _ => RemoteAnswer::Keep,
    }
}

/// Whether `show_conflict` of the set's operation names the set's request
/// as the owner of its unfinished attempt. A read that fails, or a
/// repository that no longer resolves, counts as standing. A test's canned
/// conflict of that operation stands in place of the library's, as for the
/// sheet's reads.
fn attempt_stands(backend: &dyn Backend, set: &KeptSet) -> bool {
    let service = backend.service();
    let Ok(repository) = service.resolve_repository(&set.root) else {
        return true;
    };
    let read = service.show_conflict(&repository, set.operation_id);
    let read = match backend.after_conflict_read(&set.operation_id) {
        Some(conflict) => Ok(conflict),
        None => read.map_err(|_| ()),
    };
    match read {
        Ok(conflict) => conflict.request_id.as_deref() == Some(&set.request_id.to_string()),
        Err(()) => true,
    }
}

/// Only a kept file's format version, read first so a newer format is told
/// from a damaged file.
#[derive(Deserialize)]
struct StoredVersion {
    version: u32,
}

/// The set a kept file holds for `request`: `Newer` when a newer build
/// wrote it, `Damaged` when it is not a set this build reads.
fn decode(bytes: &[u8], request: RequestId) -> Result<KeptSet, ResolutionStoreError> {
    match serde_json::from_slice::<StoredVersion>(bytes) {
        Ok(stored) if stored.version > VERSION => return Err(ResolutionStoreError::Newer),
        Ok(_) => {}
        Err(_) => return Err(ResolutionStoreError::Damaged),
    }
    serde_json::from_slice::<Stored>(bytes)
        .ok()
        .and_then(|stored| stored.into_set(request))
        .ok_or(ResolutionStoreError::Damaged)
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

mod submit;

pub(super) use submit::{Ended, settle_finished, source_of, sweep_at_launch};
pub(crate) use submit::{Followed, Followup, Followups};

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resolutions_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resolutions_answers_tests.rs"]
mod answers_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resolutions_format_tests.rs"]
mod format_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resolutions_lookup_tests.rs"]
mod lookup_tests;
