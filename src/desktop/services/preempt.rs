//! The cancel task of a poll the worker was asked to stop (D6 design B2).
//!
//! A user job admitted while an automatic poll runs, a reconfiguration and
//! quit all ask the same thing of the worker (`WorkerHandle::ask_poll_cancel`):
//! set the poll's flag and send `WorkerEvent::PreemptPoll` once. The
//! `Services` drain answers it by spawning [`cancel_until_answered`] on a
//! background executor, never on the UI thread, because the library's cancel
//! is a write: a shared cache lock for up to 250 ms and an immediate
//! transaction (Fact 30, U-5).
//!
//! One cancel is not enough. The library answers `Ok` without effect when
//! the row exists but is not active, such as an `interrupted` row between a
//! same-ID restart's attempts, and an error when the poll has not reserved
//! its row yet (it does so only after eligibility, plan and configuration).
//! So the task sends the cancel again on a short schedule, whatever the
//! answer, until the worker says the poll is no longer the job it runs. The
//! flag the worker checks before every `poll_automatic` (P-2) closes the
//! window the other way round: a poll asked to stop never starts.

use std::{future::Future, path::Path, time::Duration};

use manyhands::repository::OperationId;

use super::{Backend, worker::PollWatch};
use crate::desktop::model::polling::Root;

/// The waits between one cancel and the next: short at first, since a poll
/// usually reaches its row within a moment, then once a second while a
/// blocked call (a connect, a handshake) holds it (K-1). The last repeats.
pub(crate) const RESEND_DELAYS: [Duration; 5] = [
    Duration::from_millis(50),
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
];

/// Calls `cancel_poll(root, op)` and calls it again after each of `delays`
/// (the last repeating) until the poll `op` is no longer running. No answer
/// ends it: not an `Ok`, which may have changed nothing, and not an error,
/// which is a row that does not exist yet or a busy repository (P-1). The
/// poll's own answer ends it.
///
/// `pause` waits out a delay: a timer on the executor in production, and
/// nothing in a test, which injects the schedule instead of sleeping.
pub(crate) async fn cancel_until_answered<Pause, Wait>(
    backend: &dyn Backend,
    watch: &PollWatch,
    root: &Root,
    op: OperationId,
    delays: &[Duration],
    pause: Pause,
) where
    Pause: Fn(Duration) -> Wait,
    Wait: Future<Output = ()>,
{
    let mut step = 0;
    while watch.running(op) {
        // Whatever it answers, the poll's end is what stops this.
        let _ = backend.cancel_poll(Path::new(root), op);
        let delay = delays
            .get(step)
            .or_else(|| delays.last())
            .copied()
            .unwrap_or_default();
        step += 1;
        pause(delay).await;
    }
}

#[cfg(test)]
#[path = "preempt_tests.rs"]
mod tests;
