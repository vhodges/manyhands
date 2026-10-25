//! Reads and previews on the background executor, returned to the asking
//! entity with the stamp they were asked under.
//!
//! A busy read or prepare is retried after 100, 200 and 400 ms on the
//! executor's timer, then delivered as busy for the view to offer Retry. The
//! returned `Task` belongs in the asking model's slot, so replacing a
//! request drops the previous one; `apply` still checks the stamp, because
//! a dropped task's blocking library call cannot be interrupted.
//!
//! A read job (`read_job`) makes several calls one after another under one
//! stamp, as a tab does when it loads: each call awaits its own gate when
//! the job reaches it, and each is retried on `busy` on its own.

use std::{future::Future, sync::Arc};

use gpui_kit::{BackgroundExecutor, Context, Task};
use manyhands::{
    repository::{Mutation, PrepareDataDto, ReadError, RepositoryService},
    results::{Envelope, RecoveryAction, ResultCode},
};

use super::{BUSY_RETRY_DELAYS, Backend, ReadKind, Services};
use crate::desktop::model::stamp::Stamp;

/// What a read gave the view.
#[derive(Debug)]
pub(crate) enum ReadResult<T> {
    Ok(T),
    /// Still busy after every retry.
    Busy,
    Failed(ReadFailure),
}

impl<T> ReadResult<T> {
    /// The same answer for another read that was not made because this
    /// one gave nothing: `None` when this one gave its value.
    pub(crate) fn stopped<U>(&self) -> Option<ReadResult<U>> {
        match self {
            Self::Ok(_) => None,
            Self::Busy => Some(ReadResult::Busy),
            Self::Failed(failure) => Some(ReadResult::Failed(failure.clone())),
        }
    }
}

/// A failed read, by code and recovery only. The error's text and source
/// are never kept, so they cannot reach the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadFailure {
    pub code: ResultCode,
    pub recovery: Vec<RecoveryAction>,
}

impl From<ReadError> for ReadFailure {
    fn from(error: ReadError) -> Self {
        Self {
            code: error.code(),
            recovery: error.recovery,
        }
    }
}

/// Calls `f` until it answers something other than `busy`, waiting each
/// of `BUSY_RETRY_DELAYS` between the calls; busy after the last.
async fn retrying<T>(
    service: &RepositoryService,
    timers: &BackgroundExecutor,
    f: impl Fn(&RepositoryService) -> Result<T, ReadError>,
) -> ReadResult<T> {
    let mut delays = BUSY_RETRY_DELAYS.iter();
    loop {
        match f(service) {
            Ok(value) => return ReadResult::Ok(value),
            Err(error) if error.code() == ResultCode::Busy => match delays.next() {
                Some(delay) => timers.timer(*delay).await,
                None => return ReadResult::Busy,
            },
            Err(error) => return ReadResult::Failed(error.into()),
        }
    }
}

/// The calls of one read job, made one after another on the background
/// executor.
pub(crate) struct Reader {
    backend: Arc<dyn Backend>,
    timers: BackgroundExecutor,
}

impl Reader {
    /// A reader of its own, outside a read job: the draft registry's
    /// re-read after a save.
    pub(super) fn new(backend: Arc<dyn Backend>, timers: BackgroundExecutor) -> Self {
        Self { backend, timers }
    }

    /// One call of the job: awaits the gate of `kind`, asked for now that
    /// the job has reached it, then calls `f`, retrying it on `busy` after
    /// each of `BUSY_RETRY_DELAYS`.
    pub(crate) async fn call<T>(
        &self,
        kind: ReadKind,
        f: impl Fn(&RepositoryService) -> Result<T, ReadError>,
    ) -> ReadResult<T> {
        if let Some(gate) = self.backend.read_gate(kind) {
            // A dropped sender releases the call as a send does.
            let _ = gate.await;
        }
        retrying(self.backend.service(), &self.timers, f).await
    }
}

impl Services {
    /// Runs `f` against the service on the background executor and hands
    /// its result to `apply` on the asking entity, with `stamp`. `apply`
    /// is not called if the entity is gone.
    pub(crate) fn read<V: 'static, T: Send + 'static>(
        &self,
        kind: ReadKind,
        stamp: Stamp,
        f: impl Fn(&RepositoryService) -> Result<T, ReadError> + Send + Sync + 'static,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, ReadResult<T>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        let backend = self.backend.clone();
        // Taken now, in the order requests are made; awaited in the read.
        let gate = backend.read_gate(kind);
        let executor = cx.background_executor().clone();
        let timers = executor.clone();
        let read = executor.spawn(async move {
            if let Some(gate) = gate {
                // A dropped sender releases the read as a send does.
                let _ = gate.await;
            }
            retrying(backend.service(), &timers, f).await
        });
        cx.spawn(async move |view, cx| {
            let result = read.await;
            // The entity may be gone; then nobody wants the result.
            let _ = view.update(cx, |view, cx| apply(view, stamp, result, cx));
        })
    }

    /// Runs the job `job` makes on the background executor, its calls one
    /// after another through the `Reader` it is given, and hands what it
    /// gives to `apply` on the asking entity, with `stamp`. Each call is
    /// retried on `busy` by itself, so a busy call does not make the
    /// calls before it again. `apply` is not called if the entity is gone.
    pub(crate) fn read_job<V: 'static, T: Send + 'static, F>(
        &self,
        stamp: Stamp,
        job: impl FnOnce(Reader) -> F,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, T, &mut Context<V>) + 'static,
    ) -> Task<()>
    where
        F: Future<Output = T> + Send + 'static,
    {
        let executor = cx.background_executor().clone();
        let reader = Reader {
            backend: self.backend.clone(),
            timers: executor.clone(),
        };
        let read = executor.spawn(job(reader));
        cx.spawn(async move |view, cx| {
            let result = read.await;
            let _ = view.update(cx, |view, cx| apply(view, stamp, result, cx));
        })
    }

    /// Prepares `mutation` on the background executor and hands its
    /// envelope to `apply` on the asking entity, with `stamp`. A prepare
    /// answers with an envelope, not a `ReadError`, so it is not a `read`;
    /// like a read, one answered `busy` is retried after each of
    /// `BUSY_RETRY_DELAYS`, and the last answer, `busy` included, is
    /// delivered (ruling R-A1).
    pub(crate) fn prepare<V: 'static>(
        &self,
        stamp: Stamp,
        mutation: Mutation,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, Envelope<PrepareDataDto>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        let backend = self.backend.clone();
        let gate = backend.read_gate(ReadKind::Prepare);
        let executor = cx.background_executor().clone();
        let timers = executor.clone();
        let prepare = executor.spawn(async move {
            if let Some(gate) = gate {
                let _ = gate.await;
            }
            let mut delays = BUSY_RETRY_DELAYS.iter();
            loop {
                let envelope = backend.service().prepare(&mutation);
                match delays.next() {
                    Some(delay) if envelope.code == ResultCode::Busy => timers.timer(*delay).await,
                    _ => return envelope,
                }
            }
        });
        cx.spawn(async move |view, cx| {
            let envelope = prepare.await;
            let _ = view.update(cx, |view, cx| apply(view, stamp, envelope, cx));
        })
    }
}

#[cfg(test)]
#[path = "reads_tests.rs"]
mod tests;
