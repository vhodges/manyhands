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
        requests::{DesktopRequestId, Intent, Lane, RequestState},
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
/// executor.
pub(in crate::desktop::services) fn settle_finished(
    backend: &Arc<dyn Backend>,
    ended: Vec<Ended>,
    cx: &AsyncApp,
) {
    let backend = backend.clone();
    cx.background_executor()
        .spawn(async move {
            if let Ok(store) = backend.resolutions() {
                for ended in &ended {
                    store.ended(backend.as_ref(), ended);
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

/// The follow-ups of resolutions sent, by request ID. The drain takes one
/// when its resolution ends, so it follows whether or not a sheet is still
/// open (I-3).
#[derive(Clone, Default)]
pub(crate) struct Followups(Arc<Mutex<HashMap<RequestId, Followup>>>);

impl Followups {
    /// `followup` follows the resolution `request` once it is applied.
    pub(crate) fn register(&self, request: RequestId, followup: Followup) {
        self.lock().insert(request, followup);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<RequestId, Followup>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The resolution `request` ended with `code`: its follow-up runs when
    /// it was applied, and is forgotten either way (a Retry registers it
    /// again). While the lifecycle is queued or running nothing is sent:
    /// it goes on by itself. One that cannot be sent again resumes the
    /// operation instead (M-3).
    pub(in crate::desktop::services) fn ended(
        &self,
        request: RequestId,
        code: ResultCode,
        operations: &Operations,
        worker: &WorkerHandle,
    ) {
        let Some(followup) = self.lock().remove(&request) else {
            return;
        };
        if !matches!(code, ResultCode::Ok | ResultCode::AlreadyApplied) {
            return;
        }
        if let Some(lifecycle) = followup.lifecycle {
            let running = operations.request(lifecycle).is_some_and(|request| {
                matches!(
                    request.state,
                    RequestState::Queued | RequestState::Running { .. }
                )
            });
            if running || worker.retry(lifecycle, None) {
                return;
            }
        }
        worker.submit(
            Intent::OperationResume {
                root: followup.root,
                operation_id: followup.operation_id,
            },
            Lane::User,
            None,
            None,
        );
    }
}
