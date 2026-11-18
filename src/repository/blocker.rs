//! What stands in a writer's way when another operation's row does, and
//! how long the writer waits for it.
//!
//! A call that would begin an operation is refused while another
//! operation of the repository has a row that is not finished; see
//! `recovery::in_the_way`. The refusal names that operation: a `Blocker`.
//! Whether its holder still works is judged as an operation read judges
//! it, from the row the read gives (`RepositoryService::blocking_operation`,
//! `liveness::holder_liveness`), with the service's process probe.
//!
//! A writer that begins its row with `begin_lifecycle` waits for a live
//! holder: it lets go of the repository lease, and its registry
//! transaction has ended, before it sleeps, and takes the lease again
//! before it looks again. So it never holds anything a peer needs to
//! finish. Still refused once `WaitPolicy::budget` has passed, it answers
//! `busy`, naming the holder. A stale holder is not waited for: the
//! refusal stays `recovery_required` and names it. Neither leaves a row.

use std::{path::Path, sync::Arc, time::Duration};

use time::OffsetDateTime;

use super::{
    OperationFamily, OperationId, RepositoryError, RepositoryErrorKind, RepositoryService,
};

/// The operation whose unfinished row a call was refused for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocker {
    /// The journal whose row it is: `Local` or `Remote`.
    pub family: OperationFamily,
    pub operation_id: OperationId,
    /// Whether its holder still worked when the call that was refused
    /// judged it. `None` until a call judges it with the service's probe.
    pub live: Option<bool>,
}

impl Blocker {
    /// The local row of `operation_id`, as its journal stores the ID. A
    /// row recorded before operations had IDs names no operation, and none
    /// is named for it.
    pub(crate) fn local(operation_id: Option<&str>) -> Option<Self> {
        Self::of(OperationFamily::Local, operation_id?)
    }

    /// The remote row of `operation_id`, as its journal stores the ID.
    pub(crate) fn remote(operation_id: &str) -> Option<Self> {
        Self::of(OperationFamily::Remote, operation_id)
    }

    fn of(family: OperationFamily, operation_id: &str) -> Option<Self> {
        Some(Self {
            family,
            operation_id: OperationId::parse(operation_id).ok()?,
            live: None,
        })
    }

    fn judged(self, live: bool) -> Self {
        Self {
            live: Some(live),
            ..self
        }
    }
}

/// How long a writer waits for a live holder, and how long it sleeps
/// between two looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitPolicy {
    pub budget: Duration,
    pub poll: Duration,
}

impl Default for WaitPolicy {
    fn default() -> Self {
        Self {
            budget: Duration::from_secs(5),
            poll: Duration::from_millis(50),
        }
    }
}

/// What a waiting writer lets time pass with. The service sleeps the
/// thread; a test passes one that moves its own clock instead.
pub trait Sleeper: Send + Sync {
    fn sleep(&self, duration: Duration);
}

/// The thread's own sleep.
pub(crate) struct SystemSleeper;

impl Sleeper for SystemSleeper {
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// One writer's wait: when it began by the service's clock, and how many
/// times it has slept. It ends at the budget by either measure, so a clock
/// that does not move cannot make it endless.
pub(crate) struct Wait {
    started: OffsetDateTime,
    polls: u128,
}

/// What a writer that was refused does next.
pub(crate) enum Waited {
    /// It slept with nothing held, and looks again.
    Again,
    /// It is answered with this.
    Refused(RepositoryError),
}

impl RepositoryService {
    /// The same service, waiting for a live holder as `policy` says.
    #[doc(hidden)]
    pub fn with_wait_policy_for_testing(mut self, policy: WaitPolicy) -> Self {
        self.wait_policy = policy;
        self
    }

    /// The same service, letting the time of a wait pass with `sleeper`.
    #[doc(hidden)]
    pub fn with_sleeper_for_testing(mut self, sleeper: Arc<dyn Sleeper>) -> Self {
        self.sleeper = sleeper;
        self
    }

    /// A wait that begins now.
    pub(crate) fn start_wait(&self) -> Wait {
        Wait {
            started: self.clock.now(),
            polls: 0,
        }
    }

    /// Whether the holder of `blocker`'s row still works, as the read of
    /// it at `root` judges it. A row that has finished since the call was
    /// refused is as good as live: the next look finds it out of the way.
    /// One that cannot be read, or names no holder, is not live.
    pub(crate) fn blocker_is_live(&self, root: &Path, blocker: &Blocker) -> bool {
        match self.blocking_operation(root, blocker) {
            Some((_, false)) => true,
            Some((operation, true)) => operation.attribution.live == Some(true),
            None => false,
        }
    }

    /// What a writer that was refused with `error` does next. The caller
    /// holds nothing: its lease is let go and its transaction ended.
    ///
    /// An error that names no blocker, or that is not `recovery_required`,
    /// is answered as it is. A blocker whose holder still works is slept
    /// on, `WaitPolicy::poll` at a time, until the budget is spent; then
    /// the answer is `busy`, naming it, live. Any other blocker is named,
    /// stale, and the answer stays `recovery_required`.
    pub(crate) fn waited(&self, root: &Path, wait: &mut Wait, error: RepositoryError) -> Waited {
        let Some(blocker) = error.blocker().cloned() else {
            return Waited::Refused(error);
        };
        if error.kind != RepositoryErrorKind::RecoveryRequired {
            return Waited::Refused(error);
        }
        if !self.blocker_is_live(root, &blocker) {
            return Waited::Refused(error.with_blocker(Some(blocker.judged(false))));
        }
        let policy = self.wait_policy;
        let poll = policy.poll.max(Duration::from_nanos(1));
        let polls = policy.budget.as_nanos() / poll.as_nanos();
        let elapsed = (self.clock.now() - wait.started).whole_nanoseconds();
        if wait.polls >= polls
            || elapsed >= i128::try_from(policy.budget.as_nanos()).unwrap_or(i128::MAX)
        {
            return Waited::Refused(busy(error, blocker.judged(true)));
        }
        self.sleeper.sleep(poll);
        wait.polls += 1;
        Waited::Again
    }
}

/// `busy`, for the operation and root `error` names, naming `blocker`.
fn busy(error: RepositoryError, blocker: Blocker) -> RepositoryError {
    RepositoryError::new(
        error.operation,
        error.root.clone(),
        RepositoryErrorKind::RepositoryBusy,
        "another operation of the repository is still at work; retry",
    )
    .with_blocker(Some(blocker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_writer_waits_five_seconds_looking_again_every_fifty_milliseconds() {
        assert_eq!(
            WaitPolicy::default(),
            WaitPolicy {
                budget: Duration::from_secs(5),
                poll: Duration::from_millis(50),
            }
        );
    }
}
