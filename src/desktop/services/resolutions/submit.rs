//! Submitting a kept set, the launch sweep, what a resolution's end means
//! for its set (R-12, R-25), and what follows an applied resolution of a
//! lifecycle child (R-10; Task 11b review I-3).

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use gpui_kit::{App, AsyncApp, Task};
use manyhands::{
    repository::{OperationId, RequestId},
    results::{CheckpointEffect, ResultCode},
};

use super::{KeptSet, ResolutionStore, ResolutionStoreError};
use crate::desktop::{
    model::operations::Operations,
    services::{
        Backend, Services,
        conflicts::ConflictSource,
        requests::{DesktopRequestId, Intent, Lane},
        resends::send_lifecycle_on,
        worker::WorkerHandle,
    },
};

impl Services {
    /// The kept sets' store, or why it did not open.
    pub(crate) fn resolutions(&self) -> Result<&ResolutionStore, ResolutionStoreError> {
        self.backend.resolutions()
    }

    /// Keeps `set`, then submits it under its own request ID, on the
    /// background executor: nothing is sent when it could not be kept.
    /// Dropping the task before it ends may leave the set kept and unsent;
    /// the launch sweep deletes it, as the library has no record of it.
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

    /// The kept set a sheet sends again, looked up on the background
    /// executor, off the store's lock on the UI thread (Task 16, fix 2):
    /// the one kept under `named`, when it names a request and one is kept;
    /// else the newest one kept for `operation_id` of `root` under a request
    /// not in `except`. `None` when none is, or the store did not open.
    pub(crate) fn find_kept(
        &self,
        named: Option<RequestId>,
        root: PathBuf,
        operation_id: OperationId,
        except: Vec<RequestId>,
        cx: &App,
    ) -> Task<Option<KeptSet>> {
        let backend = self.backend.clone();
        cx.background_executor().spawn(async move {
            let store = backend.resolutions().ok()?;
            named
                .and_then(|named| store.kept(named).ok().flatten())
                .or_else(|| store.kept_for(&root, operation_id, &except).ok().flatten())
        })
    }

    /// The set kept under `named`, looked up on the background executor
    /// (Task 16 re-review, M-2). `None` when it names none, none is kept,
    /// or the store did not open.
    pub(crate) fn find_kept_named(
        &self,
        named: Option<RequestId>,
        cx: &App,
    ) -> Task<Option<KeptSet>> {
        let backend = self.backend.clone();
        cx.background_executor()
            .spawn(async move { backend.resolutions().ok()?.kept(named?).ok().flatten() })
    }
}

/// The launch sweep, on the background executor: `ResolutionStore::sweep`
/// over the backend's store, when it opened.
pub(in crate::desktop::services) fn sweep_at_launch(backend: &Arc<dyn Backend>, cx: &App) {
    let backend = backend.clone();
    cx.background_executor()
        .spawn(async move {
            if let Ok(store) = backend.resolutions() {
                store.sweep(backend.as_ref());
            }
        })
        .detach();
}

/// A resolution's request that has ended in the worker.
pub(in crate::desktop::services) struct Ended {
    pub request: RequestId,
    pub source: ConflictSource,
    /// The answer's code and checkpoint effect; `None` when the request
    /// never reached the library (removed from the queue, or not sent).
    pub answer: Option<(ResultCode, CheckpointEffect)>,
}

/// What each of `ended` means for its kept set, on the background
/// executor. A set deleted takes its follow-up with it: nothing can apply
/// that resolution any more (review M-a).
pub(in crate::desktop::services) fn settle_finished(
    backend: &Arc<dyn Backend>,
    followups: &Followups,
    ended: Vec<Ended>,
    cx: &AsyncApp,
) {
    let backend = backend.clone();
    let followups = followups.clone();
    cx.background_executor()
        .spawn(async move {
            if let Ok(store) = backend.resolutions() {
                for ended in &ended {
                    // A set that could not be deleted stays kept, with its
                    // follow-up: the launch sweep settles it. Nothing is
                    // logged.
                    if store.ended(backend.as_ref(), ended).is_err() {
                        continue;
                    }
                    if matches!(store.kept(ended.request), Ok(None)) {
                        followups.forget(ended.request);
                    }
                }
            }
        })
        .detach();
}

/// Which kind of set `intent` submits, if it submits one.
pub(in crate::desktop::services) fn source_of(intent: &Intent) -> Option<ConflictSource> {
    match intent {
        Intent::ConflictResolve { .. } => Some(ConflictSource::Remote),
        Intent::LocalContextResolve { .. } => Some(ConflictSource::Local),
        _ => None,
    }
}

/// What follows an applied resolution of a lifecycle child (R-10): the
/// lifecycle's request sent again when this session holds it, or else, and
/// when it cannot be sent again, `operation resume` of the operation the
/// conflict was read by (its lifecycle row's after a restart).
#[derive(Clone)]
pub(crate) struct Followup {
    pub lifecycle: Option<DesktopRequestId>,
    pub root: PathBuf,
    pub operation_id: OperationId,
}

/// What the drain did for an applied lifecycle child's resolution, for its
/// sheet to show (review M-b).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Followed {
    /// The lifecycle's request was sent again, or is still running.
    Resent,
    /// It could not be sent again: the operation was resumed instead, by
    /// this request.
    Resumed(DesktopRequestId),
}

#[derive(Default)]
struct Registry {
    waiting: HashMap<RequestId, Followup>,
    /// What the drain did, for an open sheet to take.
    followed: HashMap<RequestId, Followed>,
    /// How many open sheets watch each request: what the drain did is
    /// recorded only while one does, and goes when the last closes (Task
    /// 16, fix 3).
    watched: HashMap<RequestId, usize>,
}

/// The follow-ups of resolutions sent, by request ID. The drain runs one
/// when its resolution is applied, whether or not a sheet is still open
/// (I-3). One stays while its resolution's kept set does, so a Retry from
/// the operation area still sends the lifecycle on (review M-a); it goes
/// when the resolution is applied or its set is deleted.
#[derive(Clone, Default)]
pub(crate) struct Followups(Arc<Mutex<Registry>>);

impl Followups {
    /// `followup` follows the resolution `request` once it is applied.
    pub(crate) fn register(&self, request: RequestId, followup: Followup) {
        self.lock().waiting.insert(request, followup);
    }

    /// An open sheet watches `request`: what the drain does once it is
    /// applied is recorded for it to take.
    pub(crate) fn watch(&self, request: RequestId) {
        *self.lock().watched.entry(request).or_default() += 1;
    }

    /// A sheet that watched `request` closed: once none does, what the
    /// drain did for it goes, and nothing more is recorded.
    pub(crate) fn unwatch(&self, request: RequestId) {
        let mut registry = self.lock();
        let Some(count) = registry.watched.get_mut(&request) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count == 0 {
            registry.watched.remove(&request);
            registry.followed.remove(&request);
        }
    }

    /// The resolution `request` will not be applied: its set is gone, or
    /// was never kept.
    pub(crate) fn forget(&self, request: RequestId) {
        self.lock().waiting.remove(&request);
    }

    /// What the drain did once `request` was applied, taken once.
    pub(crate) fn take_followed(&self, request: RequestId) -> Option<Followed> {
        self.lock().followed.remove(&request)
    }

    /// Whether a follow-up waits on `request`.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn waits_on(&self, request: RequestId) -> bool {
        self.lock().waiting.contains_key(&request)
    }

    /// Whether what the drain did for `request` is recorded, untaken.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn records(&self, request: RequestId) -> bool {
        self.lock().followed.contains_key(&request)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Registry> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The resolution `request` ended with `code`. Applied, its follow-up
    /// runs and is recorded: the lifecycle's request sent again, or left
    /// while it is queued or running, as it goes on by itself; one that
    /// cannot be sent again resumes the operation instead (M-3). What was
    /// done is recorded only while a sheet watches `request`. Any other
    /// answer leaves it waiting while the set is kept.
    pub(in crate::desktop::services) fn ended(
        &self,
        request: RequestId,
        code: ResultCode,
        operations: &Operations,
        worker: &WorkerHandle,
    ) {
        if !matches!(code, ResultCode::Ok | ResultCode::AlreadyApplied) {
            return;
        }
        let Some(followup) = self.lock().waiting.remove(&request) else {
            return;
        };
        let resume = Some((followup.root, followup.operation_id));
        let followed = send_lifecycle_on(followup.lifecycle, resume, operations, worker);
        let mut registry = self.lock();
        if let Some(followed) = followed
            && registry.watched.contains_key(&request)
        {
            registry.followed.insert(request, followed);
        }
    }
}
