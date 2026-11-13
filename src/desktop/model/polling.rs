//! The polling schedule (D6 design Part A): pure, synchronous and
//! clock-injected, so the policy is table-tested without a worker or a
//! thread.
//!
//! The model holds one [`RepoSchedule`] per repository. Whoever drives it
//! (the scheduler) reads a [`Clock`] and passes the reading in; the
//! model never reads a clock, starts a thread or does I/O. It decides what is
//! due and when to wake ([`PollSchedule::due`], [`PollSchedule::next_wake`]),
//! moves a repository's phase from an attempt ([`PollSchedule::after`], the
//! table of A4), and keeps what needs the person's attention, one entry per
//! key, host or repository ([`PollSchedule::attention`]). Attention never
//! changes a repository's policy, and pausing never clears attention (A2).

mod attempt;
pub(crate) mod attention;
mod clock;
pub(crate) mod entry;
pub(crate) mod retire;
pub(crate) mod retire_loop;
mod state;
pub(crate) mod status;
mod table;

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

pub(crate) use attempt::{
    Answer, AttemptSummary, AttemptTarget, Authority, CancelCause, Disposition, KeyId, NotEligible,
    PollAttempt, Root, TargetKind, TransportFailure,
};
#[cfg(test)]
pub(crate) use clock::TestClock;
pub(crate) use clock::{Clock, Now};
// The tests use `TestClock`, so only the production clock is unused there.
pub(crate) use clock::SystemClock;
use entry::{RetireLine, RetireState};
use manyhands::{canonical::ItemId, repository::OperationId};
use state::PREEMPTIONS_BEFORE_INTERVAL;
pub(crate) use state::{Phase, Policy, Reason, RepoSchedule, Unshared, Why};
use table::Change;
use time::OffsetDateTime;

/// How far the two clocks may drift apart before it is a suspend or a jump
/// (A3).
const CLOCK_TOLERANCE: time::Duration = time::Duration::seconds(60);

/// One reason and the repositories it holds back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttentionEntry {
    pub reason: Reason,
    pub roots: Vec<Root>,
}

/// A check that brought something in (Wave 04 Cycle 03b design A5): an
/// automatic one or a Check now, kept for the session and never pruned. Its
/// row is Activity's S2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Brought {
    /// The order they were applied in, from this schedule's counter.
    pub seq: u64,
    pub root: Root,
    /// How many items it brought in (`AttemptSummary::brought_in`), never 0.
    pub count: u32,
    /// The time the check was applied (`Now::utc`).
    pub at: OffsetDateTime,
    /// An automatic check, not a Check now.
    pub automatic: bool,
}

/// What a clock event came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClockEvent {
    /// The clocks agree: nothing changes.
    Steady,
    /// A suspend or a jump: policies are stale, and so are the remotes the
    /// caller holds (R-6).
    Jumped,
}

#[derive(Debug, Default)]
pub(crate) struct PollSchedule {
    repos: BTreeMap<Root, RepoSchedule>,
    /// The items the loop skips for the session (R-21).
    refused: retire::RefusedSet,
    /// The kind of each item in the refused set, which Retire… needs to
    /// send its retirement (E4).
    refused_kinds: BTreeMap<(Root, ItemId), TargetKind>,
    /// Repositories removed while a poll of theirs was queued or running:
    /// the poll goes on, so it still holds the one-at-a-time slot until it
    /// answers (R-7). Nothing else of the repository is kept.
    orphans: BTreeSet<Root>,
    /// This session's finished shares and saves, per repository, each with
    /// the time the scheduler heard it, in the order heard (rule 1 of the
    /// unshared list). A completed check that landed later replays those
    /// heard after it completed.
    changes: BTreeMap<Root, Vec<(Change, OffsetDateTime)>>,
    /// The checks that brought something in, for the session (A5, K-4).
    brought: Vec<Brought>,
    /// How many completed attempts were applied to each repository, for the
    /// session (`completed_attempts`). Kept when a repository is removed, so
    /// a count only grows.
    completed: BTreeMap<Root, u64>,
}

/// Registration and reads.
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
        if self.repos.get(root).is_some_and(RepoSchedule::in_flight) {
            self.orphans.insert(root.clone());
        }
        self.repos.remove(root);
        self.changes.remove(root);
        self.refused.forget_root(root);
        self.refused_kinds.retain(|(held, _), _| held != root);
    }

    pub fn repo(&self, root: &Root) -> Option<&RepoSchedule> {
        self.repos.get(root)
    }

    /// How many completed attempts, automatic or Check now, were applied to
    /// `root` this session: what a failed Check now compares to see that a
    /// later check completed (Activity N22 clause 4).
    pub fn completed_attempts(&self, root: &Root) -> u64 {
        self.completed.get(root).copied().unwrap_or_default()
    }

    /// The checks that brought something in, in the order they were applied.
    /// Never pruned within a session, whatever becomes of the repository
    /// (A5, K-4).
    pub fn brought(&self) -> &[Brought] {
        &self.brought
    }

    /// Each repository's last recorded attempt and the retirements since,
    /// in the order of the roots: what the attempt entries of the operation
    /// area are made from (R-23, E3).
    pub fn attempts(&self) -> impl Iterator<Item = (&Root, &AttemptSummary, &[RetireLine])> {
        self.repos.iter().filter_map(|(root, repo)| {
            Some((root, repo.last.as_ref()?, repo.retirements.as_slice()))
        })
    }

    /// A policy just read. The phase, attention and waits are untouched:
    /// pause and attention are independent in both directions (A3).
    pub fn set_policy(&mut self, root: &Root, policy: Policy) {
        if let Some(repo) = self.repos.get_mut(root) {
            repo.policy = policy;
            repo.stale = false;
        }
    }

    /// A policy just read says whether the library holds the repository
    /// back until recovery. A repository that is not in flight and not held
    /// for another reason then waits as `RecoveryHeld` attention, so it is
    /// listed and counted like any other (decision 10, 11); one that is no
    /// longer held (resumed through the CLI) polls again. Called after
    /// `add` and `set_policy`, not by them: they leave the phase alone.
    pub fn sync_recovery(&mut self, root: &Root) {
        let Some(repo) = self.repos.get_mut(root) else {
            return;
        };
        let held = Phase::Attention(Reason::RecoveryHeld(root.clone()));
        if repo.policy.recovery_suspended {
            if matches!(
                repo.phase,
                Phase::Idle | Phase::Waiting { .. } | Phase::Preempted | Phase::Stopped
            ) {
                repo.phase = held;
            }
        } else if repo.phase == held {
            repo.rearm();
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
impl PollSchedule {
    fn busy(&self) -> bool {
        !self.orphans.is_empty() || self.repos.values().any(RepoSchedule::in_flight)
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

/// What is not shared yet (Part A, rules 1 to 5). Pure and synchronous, like
/// the rest: the caller passes the time it heard each change.
impl PollSchedule {
    /// A share finished and was heard at `at`: its item leaves the unshared
    /// list and the last attempt's listed items at once, and main with it
    /// when `main` (rules 1 and 2). True when one went. A root with no
    /// schedule has no list and keeps nothing.
    pub fn shared(
        &mut self,
        root: &Root,
        item: Option<&ItemId>,
        main: bool,
        at: OffsetDateTime,
    ) -> bool {
        let Some(repo) = self.repos.get_mut(root) else {
            return false;
        };
        let change = Change::Shared {
            item: item.cloned(),
            main,
        };
        let mut gone = change.apply_to(&mut repo.unshared.items);
        if let Some(last) = repo.last.as_mut() {
            gone |= change.remove_from(&mut last.listed);
        }
        self.changes
            .entry(root.clone())
            .or_default()
            .push((change, at));
        gone
    }

    /// A save finished and was heard at `at`: its item joins the unshared
    /// list at once, `Unpublished` when `new` and else `LocalAhead`, unless
    /// it is there, also before any completed check (rules 1 and 2). True
    /// when it joined. False for a root with no schedule: nothing is kept.
    pub fn saved(
        &mut self,
        root: &Root,
        item: ItemId,
        kind: Option<TargetKind>,
        new: bool,
        at: OffsetDateTime,
    ) -> bool {
        self.record(root, Change::Saved { item, kind, new }, at)
    }

    /// A merge that did not publish was heard at `at`: main joins the
    /// unshared list as `LocalAhead`, since it is ahead of the shared copy.
    pub fn main_ahead(&mut self, root: &Root, at: OffsetDateTime) -> bool {
        self.record(root, Change::MainAhead, at)
    }

    /// Logs `change` and applies it to the list at once.
    fn record(&mut self, root: &Root, change: Change, at: OffsetDateTime) -> bool {
        let Some(repo) = self.repos.get_mut(root) else {
            return false;
        };
        let changed = change.apply_to(&mut repo.unshared.items);
        self.changes
            .entry(root.clone())
            .or_default()
            .push((change, at));
        changed
    }

    /// An attempt was applied (rules 3 to 5). A completed attempt that
    /// carries targets and is not older than the list sets it from its own
    /// targets, then replays the changes heard after it completed, in the
    /// order heard, so the last share or save of an item decides. An attempt
    /// with no `completed_at` counts as older than every change heard, and
    /// than a list that has an `as_of`. An older one leaves the list, and its
    /// listed items (it is the last attempt) lose what was shared after it.
    /// Anything else (a failure, an interruption, a cancel, a replay with no
    /// targets) leaves the list.
    fn unshared_after(&mut self, root: &Root, summary: &AttemptSummary) {
        if summary.answer != Answer::Completed || !summary.carries_targets() {
            return;
        }
        let Some(repo) = self.repos.get_mut(root) else {
            return;
        };
        let at = summary.completed_at;
        let older = match (at, repo.unshared.as_of) {
            (Some(at), Some(held)) => at < held,
            (at, held) => at.is_none() && held.is_some(),
        };
        if !older {
            repo.unshared = Unshared {
                items: summary.to_share.clone(),
                problems: summary.problems.clone(),
                as_of: at,
            };
        }
        let heard = self.changes.get(root).map_or(&[][..], Vec::as_slice);
        for (change, when) in heard {
            if at.is_none_or(|at| *when > at) {
                if !older {
                    change.apply_to(&mut repo.unshared.items);
                }
                if let Some(last) = repo.last.as_mut() {
                    change.remove_from(&mut last.listed);
                }
            }
        }
    }
}

/// A poll's life.
impl PollSchedule {
    /// The scheduler submitted a poll.
    pub fn mark_queued(&mut self, root: &Root) {
        if let Some(repo) = self.repos.get_mut(root) {
            repo.phase = Phase::Queued;
        }
    }

    /// The poll of `root` answered (or will not): a repository removed
    /// while it ran no longer holds the one-at-a-time slot. Only a poll's
    /// end calls this; a Check now's `after` is not a poll ending.
    pub fn poll_ended(&mut self, root: &Root) {
        self.orphans.remove(root);
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

    /// The worker started the poll under this operation. Nothing calls it:
    /// the worker sends no `Started` for a poll (Task 3), so `Queued` stands
    /// for queued and running alike, and `in_flight` covers both.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn mark_running(&mut self, root: &Root, op: OperationId) {
        if let Some(repo) = self.repos.get_mut(root) {
            repo.phase = Phase::Running { op };
        }
    }

    /// The poll's answer (A4). Returns what it summed to; the caller marks
    /// the repository changed and starts Part E's loop from it.
    pub fn after(&mut self, now: &Now, attempt: &PollAttempt) -> Option<AttemptSummary> {
        let repo = self.repos.get_mut(&attempt.root)?;
        let summary = repo.apply(&attempt.root, now, attempt);
        self.unshared_after(&attempt.root, &summary);
        self.note_brought(&attempt.root, &summary, now, true);
        self.note_completed(attempt);
        Some(summary)
    }

    /// A Check now completed (C1, E1): its answer is the repository's latest
    /// attempt, and, unless a poll of it is queued or running, moves the
    /// phase as a poll's `Completed` does. Returns what it summed to;
    /// `None` for a repository that is not scheduled.
    pub fn checked_now(&mut self, now: &Now, attempt: &PollAttempt) -> Option<AttemptSummary> {
        let repo = self.repos.get_mut(&attempt.root)?;
        let summary = if repo.in_flight() {
            let summary = AttemptSummary::of(attempt);
            repo.last_completed = Some(attempt.completed_at.unwrap_or(now.utc));
            if summary.is_recorded() {
                repo.last = Some(summary.clone());
                repo.retirements.clear();
            }
            summary
        } else {
            repo.apply(&attempt.root, now, attempt)
        };
        // Both branches follow the same rules for the unshared list (rule 5).
        self.unshared_after(&attempt.root, &summary);
        self.note_brought(&attempt.root, &summary, now, false);
        self.note_completed(attempt);
        Some(summary)
    }

    /// Counts an applied attempt that completed.
    fn note_completed(&mut self, attempt: &PollAttempt) {
        if attempt.answer == Answer::Completed {
            *self.completed.entry(attempt.root.clone()).or_default() += 1;
        }
    }

    /// Keeps a check that brought something in (A5), at the reading its
    /// caller applied it with. The list is never pruned (K-4), so the next
    /// sequence number is its length.
    fn note_brought(&mut self, root: &Root, summary: &AttemptSummary, now: &Now, automatic: bool) {
        if summary.brought_in == 0 {
            return;
        }
        self.brought.push(Brought {
            seq: self.brought.len() as u64,
            root: root.clone(),
            count: summary.brought_in,
            at: now.utc,
            automatic,
        });
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

    /// Part E's loop for this repository starts, or, where one runs, starts
    /// again over a newer attempt's items (RT-11). No poll of it meanwhile
    /// (R-32). Returns false when the repository is not scheduled, when a
    /// poll of it is queued or running, or when it waits for the person
    /// (attention) or for a quit's Cancel (`Stopped`). A wait on R-4's
    /// schedule or a preempted poll would give way, but the callers reach
    /// this after the attempt that began the loop has been applied, which
    /// leaves the repository `Idle` (or, for a Check now beside a running
    /// loop, `Retiring`): in practice only those two are seen. The wider
    /// match is the owner's ruling for a Check now to start or replace a
    /// loop from any such phase.
    pub fn begin_retiring(&mut self, root: &Root) -> bool {
        match self.repos.get_mut(root) {
            Some(repo)
                if matches!(
                    repo.phase,
                    Phase::Idle | Phase::Preempted | Phase::Waiting { .. } | Phase::Retiring
                ) =>
            {
                repo.rearm();
                repo.phase = Phase::Retiring;
                true
            }
            _ => false,
        }
    }

    /// A retirement's answer needs the person (E3): the repository waits
    /// for `reason` like a poll's answer would make it (A4), replacing any
    /// entry it had, and the retirement loop, if one runs, ends (RT-8). A
    /// poll in flight is left to its own answer, which meets the same wall.
    /// Returns whether it was set.
    pub fn raise(&mut self, root: &Root, reason: Reason) -> bool {
        match self.repos.get_mut(root) {
            Some(repo) if !repo.in_flight() => {
                repo.rearm();
                repo.phase = Phase::Attention(reason);
                true
            }
            _ => false,
        }
    }

    /// Records the answer to a retirement of `item`: a refusal the table
    /// does not retry joins the session's refused set (R-21).
    pub fn retirement_answered(
        &mut self,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        verdict: &retire::Verdict,
    ) {
        self.refused.answered(root, item, verdict);
        if self.refused.contains(root, item) {
            self.refused_kinds
                .insert((root.clone(), item.clone()), kind);
        }
    }

    /// A retirement of `item` was submitted: its entry says "Retiring…".
    pub fn retire_started(&mut self, root: &Root, kind: TargetKind, item: &ItemId) {
        self.set_retire_line(root, kind, item, Some(RetireState::Retiring), true);
    }

    /// The retirement of `item` answered with `verdict`: its entry says
    /// what came of it (E3), or nothing when the answer is attention, a
    /// retry or a refusal, which have their own rows.
    pub fn retire_settled(
        &mut self,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        verdict: &retire::Verdict,
    ) {
        let state = match verdict {
            retire::Verdict::Retired => Some(RetireState::Retired),
            retire::Verdict::Owed { catching_up, .. } => Some(RetireState::Unfinished {
                catching_up: *catching_up,
            }),
            _ => None,
        };
        self.set_retire_line(root, kind, item, state, false);
    }

    /// The pending retirement of `item` was settled by an Abandon or a
    /// Resume (E4): after a Resume that finished the row the entry says it
    /// finished retiring; after an Abandon, or a row settled elsewhere, it
    /// says nothing, as it claims nothing was removed.
    pub fn retire_ended(&mut self, root: &Root, kind: TargetKind, item: &ItemId, finished: bool) {
        let state = finished.then_some(RetireState::Finished);
        self.set_retire_line(root, kind, item, state, false);
    }

    /// Whether the attempt's entry says `item` was retired since the attempt.
    pub fn retire_line_retired(&self, root: &Root, kind: TargetKind, item: &ItemId) -> bool {
        self.repos.get(root).is_some_and(|repo| {
            repo.retirements.iter().any(|line| {
                line.kind == kind && line.item == *item && line.state == RetireState::Retired
            })
        })
    }

    /// The item's line is `state`, or gone, with `None`. A retirement that
    /// begins puts its line last; its answer keeps the place it began in.
    fn set_retire_line(
        &mut self,
        root: &Root,
        kind: TargetKind,
        item: &ItemId,
        state: Option<RetireState>,
        begins: bool,
    ) {
        let Some(repo) = self.repos.get_mut(root) else {
            return;
        };
        let held = repo
            .retirements
            .iter()
            .position(|line| line.kind == kind && line.item == *item);
        let line = state.map(|state| RetireLine {
            kind,
            item: item.clone(),
            state,
        });
        match (held, line) {
            (Some(at), Some(line)) if !begins => repo.retirements[at] = line,
            (held, line) => {
                if let Some(at) = held {
                    repo.retirements.remove(at);
                }
                repo.retirements.extend(line);
            }
        }
    }

    /// Whether the loop skips the item (R-21).
    pub fn retirement_refused(&self, root: &Root, item: &ItemId) -> bool {
        self.refused.contains(root, item)
    }

    /// Retire… was chosen for the item: it leaves the refused set (A19).
    pub fn retire_chosen(&mut self, root: &Root, item: &ItemId) {
        self.refused.forget(root, item);
        self.refused_kinds.remove(&(root.clone(), item.clone()));
    }

    /// The items Retire… is offered for in `root` (E4): every `Closed` or
    /// `Promoted` item its latest completed attempt reported that has no
    /// retirement queued, running, pending or done since, in the poll's
    /// order, then the items in the refused set that attempt no longer
    /// reports. Skipped, refused, preserved, failed before any effect and
    /// not reached all land here. The worker and the in-flight record hold
    /// the state a line cannot (a retirement sent before a relaunch): the
    /// caller leaves those out.
    pub fn retire_offers(&self, root: &Root) -> Vec<entry::RetireOffer> {
        let Some(repo) = self.repos.get(root) else {
            return Vec::new();
        };
        let reported = repo
            .last
            .iter()
            .flat_map(|last| &last.retire)
            .filter_map(|target| Some((target.kind?, target.item_id.clone()?)));
        let refused = self
            .refused_kinds
            .iter()
            .filter(|((held, _), _)| held == root)
            .map(|((_, item), kind)| (*kind, item.clone()));
        let mut offers: Vec<entry::RetireOffer> = Vec::new();
        for (kind, item) in reported.chain(refused) {
            let begun = repo
                .retirements
                .iter()
                .any(|line| line.kind == kind && line.item == item);
            let listed = offers
                .iter()
                .any(|offer| offer.kind == kind && offer.item == item);
            if !begun && !listed {
                offers.push(entry::RetireOffer { kind, item });
            }
        }
        offers
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
