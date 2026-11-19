//! How long quitting has waited for a job (D6 design Part D, item 5).
//!
//! Quitting that waits changes its words once, after ten seconds: "Still
//! stopping" (`QuitPhase::Finishing`). Nothing is cleared or reclaimed then;
//! only the text changes. The wait is a value here, not a constant in the
//! view, so a test injects it.

use std::time::Duration;

use super::Operations;

/// How long quitting waits for a job before it says it is still stopping.
pub(crate) const STILL_STOPPING_AFTER: Duration = Duration::from_secs(10);

#[derive(Default)]
pub(super) struct QuitWait {
    /// An injected wait, in place of [`STILL_STOPPING_AFTER`].
    after: Option<Duration>,
    /// Quitting has waited that long for the job it is finishing.
    long: bool,
}

impl Operations {
    /// How long quitting waits before it says it is still stopping.
    pub(crate) fn still_stopping_after(&self) -> Duration {
        self.quit_wait.after.unwrap_or(STILL_STOPPING_AFTER)
    }

    /// Whether quitting has waited that long for its job.
    pub(crate) fn still_stopping(&self) -> bool {
        self.quit_wait.long
    }

    pub(crate) fn set_still_stopping(&mut self, long: bool) {
        self.quit_wait.long = long;
    }

    #[cfg(test)]
    pub(crate) fn set_still_stopping_after(&mut self, after: Duration) {
        self.quit_wait.after = Some(after);
    }
}
