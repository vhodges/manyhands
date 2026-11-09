//! One repository's schedule (D6 design A2).

use std::time::{Duration, Instant};

use manyhands::repository::OperationId;
use time::OffsetDateTime;

use super::{
    attempt::{AttemptSummary, Authority, KeyId, Root},
    clock::Now,
};

/// The last `polling_status` read. The library owns the arithmetic; the
/// model only follows it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) struct Policy {
    pub enabled: bool,
    pub paused: bool,
    pub interval: Duration,
    pub backoff: Option<Duration>,
    pub recovery_suspended: bool,
    /// The stored deadline. None, or a past one, is due.
    pub next_eligible_at: Option<OffsetDateTime>,
    pub active_operation: Option<OperationId>,
}

impl Policy {
    /// Enabled, unpaused and unsuspended: the person's and the library's
    /// say-so, before any deadline.
    pub fn allows(&self) -> bool {
        self.enabled && !self.paused && !self.recovery_suspended
    }
}

/// Why a repository is waiting on the monotonic clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) enum Why {
    Busy,
    Interrupted,
    /// A local operation is pending (`recovery_required`).
    Unfinished,
    /// Any other repository error (R-35).
    Failed,
    NotDue,
}

/// What holds a repository back until the person acts. Per key, per host,
/// or per repository (R-8, decision 11): a locked key or an unapproved host
/// names every repository it holds back; the rest are one repository's.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 8.
pub(crate) enum Reason {
    KeyLocked(KeyId),
    /// No key selected (`None`), or one missing, unreadable, invalid or
    /// rejected. Per repository, since Choose a key… is its setting.
    KeyProblem(Option<KeyId>, Root),
    HostUnapproved(Authority),
    /// A host answer that names no host: per repository.
    HostUnidentified(Root),
    ConfigurationRequired(Root),
    RecoveryHeld(Root),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) enum Phase {
    Idle,
    Queued,
    Running {
        op: OperationId,
    },
    /// R-4's schedule and the not-due wait. `op` is the operation a restart
    /// resends (`restart=true`); `None` means a new one.
    Waiting {
        until: Instant,
        why: Why,
        op: Option<OperationId>,
    },
    /// Until the user lane empties; no time (B2).
    Preempted,
    Attention(Reason),
    /// A retirement loop runs (R-32).
    Retiring,
    Stopped,
}

/// R-4: 5 s, 15 s, 60 s, then the interval.
pub(super) const RESTART_DELAYS: [Duration; 3] = [
    Duration::from_secs(5),
    Duration::from_secs(15),
    Duration::from_secs(60),
];

/// R-34.
pub(super) const PREEMPTIONS_BEFORE_INTERVAL: u8 = 3;

#[derive(Clone, Debug)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) struct RepoSchedule {
    pub policy: Policy,
    pub phase: Phase,
    pub last: Option<AttemptSummary>,
    /// The last completed attempt's time, for fairness (R-7). Only a
    /// `Completed` answer moves it (R-9).
    pub last_completed: Option<OffsetDateTime>,
    /// Preempted this many times in a row (R-34).
    pub preempted: u8,
    /// Steps taken on R-4's schedule in a row.
    pub(super) streak: u8,
    /// Do not poll before this, whatever the stored deadline says: the
    /// interval after an answer the library stores no deadline for (R-35).
    pub(super) hold_until: Option<Instant>,
    /// The policy may be out of date; it is read again before this
    /// repository is polled.
    pub stale: bool,
}

impl RepoSchedule {
    pub fn new(policy: Policy, last_completed: Option<OffsetDateTime>) -> Self {
        Self {
            policy,
            phase: Phase::Idle,
            last: None,
            last_completed,
            preempted: 0,
            streak: 0,
            hold_until: None,
            stale: false,
        }
    }

    pub fn in_flight(&self) -> bool {
        matches!(self.phase, Phase::Queued | Phase::Running { .. })
    }

    /// When this repository may next be polled, on the monotonic clock, or
    /// `None` when something else must happen first: it is off, paused or
    /// held, its policy is stale, or its phase is not a waiting one.
    pub fn wake_at(&self, now: &Now) -> Option<Instant> {
        if self.stale || !self.policy.allows() {
            return None;
        }
        match &self.phase {
            Phase::Idle => {
                let deadline = self
                    .policy
                    .next_eligible_at
                    .map_or(now.mono, |at| now.mono_at(at));
                Some(self.hold_until.map_or(deadline, |hold| hold.max(deadline)))
            }
            Phase::Waiting { until, .. } => Some(*until),
            _ => None,
        }
    }

    pub fn ready(&self, now: &Now) -> bool {
        self.wake_at(now).is_some_and(|at| at <= now.mono)
    }

    /// Back to a plain `Idle`, forgetting every wait.
    pub(super) fn rearm(&mut self) {
        self.phase = Phase::Idle;
        self.streak = 0;
        self.hold_until = None;
    }
}
