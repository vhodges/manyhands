//! Quit at the worker (D6 design Part D).

use std::sync::PoisonError;

use super::{
    DesktopRequestId, Job, QuitState, Work, WorkerEvent, WorkerHandle, ask_poll_cancel, poll,
};
use crate::desktop::model::polling::CancelCause;

impl WorkerHandle {
    /// Stops admitting jobs, empties the queue and cancels the running
    /// job's token. Quitting may go ahead when no job is running;
    /// otherwise once that job returns.
    ///
    /// A running automatic poll has no token, so it is asked to stop
    /// through the cancel task (`ask_poll_cancel`, P-7): the first ask sends
    /// `PreemptPoll` and later asks, this being called on every `Operations`
    /// notify while quitting waits, send nothing. A queued poll is answered
    /// as cancelled by quit, which it was never run to see, so the
    /// scheduler's record of it settles by the ordinary answer; any other
    /// queued job is removed.
    pub(crate) fn quit(&self) -> QuitState {
        let shared = self.shared();
        let mut guard = shared.lock();
        let state = &mut *guard;
        state.closed = true;
        let queued: Vec<Job> = state
            .user
            .drain(..)
            .chain(state.background.drain(..))
            .collect();
        for job in queued {
            shared.send(match job.work {
                Work::Poll {
                    root, operation, ..
                } => {
                    let attempt = poll::stopped(&root, operation, CancelCause::Quit, None);
                    WorkerEvent::Polled(root, attempt)
                }
                _ => WorkerEvent::Removed(job.id),
            });
        }
        ask_poll_cancel(shared, state, CancelCause::Quit);
        // TODO(Task 11a): a running background `ContextRetire` is left to
        // finish (decision 28, R-38): its token is not cancelled here, or
        // the item would be left pending and hold the repository's polls at
        // the next launch (Fact 32). Every other job's token is.
        match &state.running {
            Some(running) => {
                if let Some(cancel) = &running.cancel {
                    cancel.cancel();
                }
                QuitState::Finishing(running.id)
            }
            None => QuitState::Idle,
        }
    }

    /// Whether the job `id` is the automatic poll the worker is running: a
    /// poll is no request, so the request model cannot say what quitting
    /// waits for.
    pub(crate) fn runs_poll(&self, id: DesktopRequestId) -> bool {
        let state = self.shared().lock();
        state
            .running
            .as_ref()
            .is_some_and(|running| running.id == id && running.poll.is_some())
    }

    /// Ends the worker thread and waits for it. A graceful exit calls it
    /// once `quit()` has answered `Idle` (A9, O-7): nothing is running, so
    /// the thread, woken, returns at once and drops its credential session.
    /// Quit now does not: it leaves the thread, and the library's journals
    /// recover what it was doing.
    ///
    /// The join blocks the caller, not the executor: the thread wakes
    /// nothing on its way out (the events channel stays open until the last
    /// handle goes, see `Owner`), so a test scheduler that forbids parking
    /// is not asked to park. Called again, or after a panic of the thread,
    /// it does nothing more.
    pub(crate) fn shut_down_and_join(&self) {
        let shared = self.shared();
        shared.lock().shut_down = true;
        shared.changed.notify_all();
        let thread = shared
            .thread
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(thread) = thread {
            // A thread that panicked has ended all the same.
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
#[path = "quit_tests.rs"]
mod tests;
