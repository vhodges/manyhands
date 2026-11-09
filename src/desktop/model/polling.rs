//! The polling schedule (D6 design Part A): pure, synchronous and
//! clock-injected, so the policy is table-tested without a worker or a
//! thread.
//!
//! The model holds one [`RepoSchedule`] per repository. Whoever drives it
//! (the scheduler, Task 5) reads a [`Clock`] and passes the reading in; the
//! model never reads a clock, starts a thread or does I/O. It decides what is
//! due and when to wake ([`PollSchedule::due`], [`PollSchedule::next_wake`]),
//! moves a repository's phase from an attempt ([`PollSchedule::after`], the
//! table of A4), and keeps what needs the person's attention, one entry per
//! key, host or repository ([`PollSchedule::attention`]). Attention never
//! changes a repository's policy, and pausing never clears attention (A2).

mod attempt;
mod clock;
mod state;
mod table;

use std::{collections::BTreeMap, time::Duration};

#[cfg_attr(not(test), allow(unused_imports))] // Called from Task 3.
pub(crate) use attempt::{
    Answer, AttemptSummary, AttemptTarget, Authority, CancelCause, Disposition, KeyId, NotEligible,
    PollAttempt, Root, TargetKind, TransportFailure,
};
#[cfg_attr(not(test), allow(unused_imports))] // Called from Task 5.
pub(crate) use clock::{Clock, Now};
// The tests use `TestClock`, so only the production clock is unused there.
#[allow(unused_imports)] // Called from Task 5.
pub(crate) use clock::SystemClock;
use manyhands::repository::OperationId;
use state::PREEMPTIONS_BEFORE_INTERVAL;
pub(crate) use state::{Phase, Policy, Reason, RepoSchedule, Why};
use time::OffsetDateTime;

/// How far the two clocks may drift apart before it is a suspend or a jump
/// (A3).
const CLOCK_TOLERANCE: time::Duration = time::Duration::seconds(60);

/// One reason and the repositories it holds back.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 8.
pub(crate) struct AttentionEntry {
    pub reason: Reason,
    pub roots: Vec<Root>,
}

/// What a clock event came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) enum ClockEvent {
    /// The clocks agree: nothing changes.
    Steady,
    /// A suspend or a jump: policies are stale, and so are the remotes the
    /// caller holds (R-6).
    Jumped,
}

#[derive(Debug, Default)]
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) struct PollSchedule {
    repos: BTreeMap<Root, RepoSchedule>,
}

/// Registration and reads.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
impl PollSchedule {
    /// A repository, with the policy just read and its last completion
    /// (restored from `remote_snapshot` at launch). Replaces any schedule it
    /// had.
    pub fn add(&mut self, root: Root, policy: Policy, last_completed: Option<OffsetDateTime>) {
        self.repos
            .insert(root, RepoSchedule::new(policy, last_completed));
    }

    /// Removal drops the schedule (R-11).
    pub fn remove(&mut self, root: &Root) {
        self.repos.remove(root);
    }

    pub fn repo(&self, root: &Root) -> Option<&RepoSchedule> {
        self.repos.get(root)
    }

    /// A policy just read. The phase, attention and waits are untouched:
    /// pause and attention are independent in both directions (A3).
    pub fn set_policy(&mut self, root: &Root, policy: Policy) {
        if let Some(repo) = self.repos.get_mut(root) {
            repo.policy = policy;
            repo.stale = false;
        }
    }

    /// The repositories whose policy must be read again before they are
    /// polled.
    pub fn stale(&self) -> Vec<Root> {
        self.repos
            .iter()
            .filter(|(_, repo)| repo.stale)
            .map(|(root, _)| root.clone())
            .collect()
    }

    /// The operation a restart resends (`restart=true`) when this repository
    /// is polled next, or `None` for a new one (R-4). Read it before
    /// `mark_queued`, which discards the wait and its operation.
    pub fn restart_of(&self, root: &Root) -> Option<OperationId> {
        match self.repos.get(root)?.phase {
            Phase::Waiting { op, .. } => op,
            _ => None,
        }
    }
}

/// What to poll and when (A3).
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
impl PollSchedule {
    fn busy(&self) -> bool {
        self.repos.values().any(RepoSchedule::in_flight)
    }

    /// The repositories to poll now, fairest first: enabled, unpaused,
    /// unsuspended, past their deadline, with no attention and no retirement
    /// loop (R-32), earliest last completion first, never-completed first
    /// of all (R-7). Nothing is due while a poll is queued or running: one
    /// repository at a time. At launch the stored deadline governs, whether
    /// the interval or a failure backoff set it (decision 23).
    pub fn due(&self, now: &Now) -> Vec<Root> {
        if self.busy() {
            return Vec::new();
        }
        let mut due: Vec<_> = self
            .repos
            .iter()
            .filter(|(_, repo)| repo.ready(now))
            .collect();
        due.sort_by_key(|(_, repo)| repo.last_completed);
        due.into_iter().map(|(root, _)| root.clone()).collect()
    }

    /// The earliest time something becomes due, on the monotonic clock, as
    /// of this reading. `None` while a poll is in flight (its answer wakes
    /// the scheduler) or when nothing waits on a time.
    pub fn next_wake(&self, now: &Now) -> Option<std::time::Instant> {
        if self.busy() {
            return None;
        }
        self.repos
            .values()
            .filter_map(|repo| repo.wake_at(now))
            .min()
    }

    /// A suspend or a jump: if the two clocks moved apart by more than 60 s,
    /// every not-due wait ends (the stored deadline decides again), a busy,
    /// interrupted or unfinished wait keeps its operation and its place in
    /// R-4's schedule, and every policy is read again. Each repository is
    /// then due at most once, one at a time: no catch-up (R-6).
    pub fn clock_event(&mut self, utc_delta: time::Duration, mono_delta: Duration) -> ClockEvent {
        let drift = match time::Duration::try_from(mono_delta) {
            Ok(mono) => utc_delta.saturating_sub(mono).abs(),
            Err(_) => time::Duration::MAX,
        };
        if drift <= CLOCK_TOLERANCE {
            return ClockEvent::Steady;
        }
        for repo in self.repos.values_mut() {
            repo.stale = true;
            if matches!(
                repo.phase,
                Phase::Waiting {
                    why: Why::NotDue,
                    ..
                }
            ) {
                repo.phase = Phase::Idle;
            }
        }
        ClockEvent::Jumped
    }
}

/// A poll's life.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
impl PollSchedule {
    /// The scheduler submitted a poll.
    pub fn mark_queued(&mut self, root: &Root) {
        if let Some(repo) = self.repos.get_mut(root) {
            repo.phase = Phase::Queued;
        }
    }

    /// A queued or running poll will not answer (it was dropped before it
    /// ran, the worker panicked or its channel closed): the repository goes
    /// back to `Idle` with its policy read again, so the one-at-a-time rule
    /// cannot silence polling for good. Returns whether it was in flight.
    pub fn abandon(&mut self, root: &Root) -> bool {
        match self.repos.get_mut(root) {
            Some(repo) if repo.in_flight() => {
                repo.rearm();
                repo.stale = true;
                true
            }
            _ => false,
        }
    }

    /// The worker started the poll under this operation.
    pub fn mark_running(&mut self, root: &Root, op: OperationId) {
        if let Some(repo) = self.repos.get_mut(root) {
            repo.phase = Phase::Running { op };
        }
    }

    /// The poll's answer (A4). Returns what it summed to; the caller marks
    /// the repository changed and starts Part E's loop from it.
    pub fn after(&mut self, now: &Now, attempt: &PollAttempt) -> Option<AttemptSummary> {
        let repo = self.repos.get_mut(&attempt.root)?;
        Some(repo.apply(&attempt.root, now, attempt))
    }

    /// The user lane is empty: every preempted poll goes again with no
    /// failure backoff, except one preempted three times in a row, which
    /// waits for the interval (R-34).
    pub fn user_lane_empty(&mut self, now: &Now) {
        for repo in self.repos.values_mut() {
            if repo.phase != Phase::Preempted {
                continue;
            }
            repo.phase = Phase::Idle;
            if repo.preempted >= PREEMPTIONS_BEFORE_INTERVAL {
                repo.preempted = 0;
                repo.hold_until = Some(now.after(repo.policy.interval));
            }
        }
    }

    /// Quit's Cancel resumes polling (R-33).
    pub fn resume_stopped(&mut self) {
        for repo in self.repos.values_mut() {
            if repo.phase == Phase::Stopped {
                repo.rearm();
            }
        }
    }
}

/// Attention, reconfiguration and the retirement loop.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 8.
impl PollSchedule {
    /// One entry per key, host or repository, and nothing else (decision
    /// 11, W-1): a key locked for three repositories is one. Paused, off,
    /// unreachable, waiting and not-due repositories are not here. The
    /// entries come in the order of their first repository's root; sort in
    /// the view if a different stable order is wanted.
    pub fn attention(&self) -> Vec<AttentionEntry> {
        let mut entries: Vec<AttentionEntry> = Vec::new();
        for (root, repo) in &self.repos {
            let Phase::Attention(reason) = &repo.phase else {
                continue;
            };
            match entries.iter_mut().find(|entry| &entry.reason == reason) {
                Some(entry) => entry.roots.push(root.clone()),
                None => entries.push(AttentionEntry {
                    reason: reason.clone(),
                    roots: vec![root.clone()],
                }),
            }
        }
        entries
    }

    /// The person acted: every repository held for this reason polls again.
    /// Returns them. Pause is untouched (A3). Resume checking must call this
    /// with `RecoveryHeld(root)` and also `set_policy` with the policy read
    /// again, since the hold is both attention and the policy's flag.
    pub fn clear_attention(&mut self, reason: &Reason) -> Vec<Root> {
        let mut cleared = Vec::new();
        for (root, repo) in &mut self.repos {
            if repo.phase == Phase::Attention(reason.clone()) {
                repo.rearm();
                cleared.push(root.clone());
            }
        }
        cleared
    }

    /// A key, host-trust, remote or registration change reached this
    /// repository (R-11): attention and every wait are cleared, and the
    /// policy is read again. A stored pause is not touched, so a re-added
    /// remote leaves it alone. A poll in flight is cancelled by the caller
    /// and answers `Cancelled(Reconfiguration)`.
    pub fn reconfigure(&mut self, root: &Root) {
        let Some(repo) = self.repos.get_mut(root) else {
            return;
        };
        repo.stale = true;
        if matches!(repo.phase, Phase::Attention(_) | Phase::Waiting { .. }) {
            repo.rearm();
        } else {
            repo.streak = 0;
            repo.hold_until = None;
        }
    }

    /// Part E's loop for this repository starts. No poll of it meanwhile
    /// (R-32). Returns false when it is not idle.
    pub fn begin_retiring(&mut self, root: &Root) -> bool {
        match self.repos.get_mut(root) {
            Some(repo) if repo.phase == Phase::Idle => {
                repo.phase = Phase::Retiring;
                true
            }
            _ => false,
        }
    }

    pub fn end_retiring(&mut self, root: &Root) {
        if let Some(repo) = self.repos.get_mut(root)
            && repo.phase == Phase::Retiring
        {
            repo.phase = Phase::Idle;
        }
    }
}

#[cfg(test)]
#[path = "polling_tests.rs"]
mod tests;
