//! Reads and previews on the background executor, returned to the asking
//! entity with the stamp they were asked under.
//!
//! A busy read is retried after 100, 200 and 400 ms on the executor's
//! timer, then delivered as `Busy` for the view to offer Retry. The
//! returned `Task` belongs in the asking model's slot, so replacing a
//! request drops the previous one; `apply` still checks the stamp, because
//! a dropped task's blocking library call cannot be interrupted.

use std::time::Duration;

use gpui_kit::{Context, Task};
use manyhands::{
    repository::{Mutation, PrepareDataDto, ReadError, RepositoryService},
    results::{Envelope, RecoveryAction, ResultCode},
};

use super::{ReadKind, Services};
use crate::desktop::model::stamp::Stamp;

/// The waits before each retry of a busy read.
const BUSY_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
];

/// What a read gave the view.
#[derive(Debug)]
pub(crate) enum ReadResult<T> {
    Ok(T),
    /// Still busy after every retry.
    Busy,
    Failed(ReadFailure),
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
            let mut delays = BUSY_RETRY_DELAYS.iter();
            loop {
                match f(backend.service()) {
                    Ok(value) => return ReadResult::Ok(value),
                    Err(error) if error.code() == ResultCode::Busy => match delays.next() {
                        Some(delay) => timers.timer(*delay).await,
                        None => return ReadResult::Busy,
                    },
                    Err(error) => return ReadResult::Failed(error.into()),
                }
            }
        });
        cx.spawn(async move |view, cx| {
            let result = read.await;
            // The entity may be gone; then nobody wants the result.
            let _ = view.update(cx, |view, cx| apply(view, stamp, result, cx));
        })
    }

    /// Prepares `mutation` on the background executor and hands its
    /// envelope to `apply` on the asking entity, with `stamp`. A prepare
    /// answers with an envelope, not a `ReadError`, so it is not a `read`
    /// and is not retried; the envelope's code says what happened.
    pub(crate) fn prepare<V: 'static>(
        &self,
        stamp: Stamp,
        mutation: Mutation,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, Envelope<PrepareDataDto>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        let backend = self.backend.clone();
        let gate = backend.read_gate(ReadKind::Prepare);
        let prepare = cx.background_executor().spawn(async move {
            if let Some(gate) = gate {
                let _ = gate.await;
            }
            backend.service().prepare(&mutation)
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
