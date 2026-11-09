//! After an attempt (D6 design A4): one match arm per row of the table.

use super::{
    attempt::{
        Answer, AttemptSummary, CancelCause, NotEligible, PollAttempt, Root, TransportFailure,
    },
    clock::Now,
    state::{Phase, RESTART_DELAYS, Reason, RepoSchedule, Why},
};

impl RepoSchedule {
    /// Moves the phase from the attempt and returns what the attempt
    /// summed to. Policy flags an answer proves (paused, off, held) are
    /// set at once; the policy is also marked stale wherever the model's
    /// copy was wrong, so it is read again before the next poll and the
    /// scheduler cannot loop on a wrong copy.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
    pub(super) fn apply(
        &mut self,
        root: &Root,
        now: &Now,
        attempt: &PollAttempt,
    ) -> AttemptSummary {
        let summary = AttemptSummary::of(attempt);
        let on_schedule = matches!(
            attempt.answer,
            Answer::Busy
                | Answer::Interrupted
                | Answer::Repository { .. }
                | Answer::OperationMismatch
        );
        let preempted = attempt.answer == Answer::Cancelled(CancelCause::Preemption);
        self.preempted = if preempted {
            self.preempted.saturating_add(1)
        } else {
            0
        };
        if !on_schedule {
            self.streak = 0;
        }
        self.hold_until = None;

        match &attempt.answer {
            Answer::Completed => {
                self.last_completed = Some(attempt.completed_at.unwrap_or(now.utc));
                self.idle_at_stored_deadline(attempt);
            }
            Answer::Transport(failure) => self.transport(root, now, attempt, failure),
            Answer::Cancelled(CancelCause::Preemption) => self.phase = Phase::Preempted,
            Answer::Cancelled(CancelCause::Quit) => self.phase = Phase::Stopped,
            Answer::Cancelled(CancelCause::Reconfiguration | CancelCause::Takeover) => {
                self.phase = Phase::Idle;
                self.stale = true;
            }
            Answer::Interrupted => self.wait(now, Why::Interrupted, Some(attempt.operation)),
            Answer::Busy => self.wait(now, Why::Busy, Some(attempt.operation)),
            Answer::Repository {
                recovery_required: true,
            } => self.wait(now, Why::Unfinished, None),
            Answer::Repository { .. } | Answer::OperationMismatch => {
                self.wait(now, Why::Failed, None)
            }
            Answer::ConflictPending => self.idle_for_interval(now),
            Answer::NotEligible(reason) => self.not_eligible(root, now, attempt, *reason),
        }
        if summary.is_recorded() {
            self.last = Some(summary.clone());
        }
        summary
    }

    /// `Idle`, due at the stored deadline: the one the attempt carries, or
    /// the one the library stored, read again.
    fn idle_at_stored_deadline(&mut self, attempt: &PollAttempt) {
        self.phase = Phase::Idle;
        match attempt.next_eligible_at {
            Some(next) => self.policy.next_eligible_at = Some(next),
            None => self.stale = true,
        }
    }

    /// `Idle` at the interval, never at the stored deadline (R-35).
    fn idle_for_interval(&mut self, now: &Now) {
        self.phase = Phase::Idle;
        self.hold_until = Some(now.after(self.policy.interval));
    }

    /// R-4: wait 5 s, 15 s, 60 s, resending the same operation, then wait
    /// the interval and use a new one. The count starts again after the
    /// interval.
    fn wait(&mut self, now: &Now, why: Why, op: Option<manyhands::repository::OperationId>) {
        self.phase = match RESTART_DELAYS.get(usize::from(self.streak)) {
            Some(delay) => {
                self.streak = self.streak.saturating_add(1);
                Phase::Waiting {
                    until: now.after(*delay),
                    why,
                    op,
                }
            }
            None => {
                self.streak = 0;
                Phase::Waiting {
                    until: now.after(self.policy.interval),
                    why,
                    op: None,
                }
            }
        };
    }

    fn transport(
        &mut self,
        root: &Root,
        now: &Now,
        attempt: &PollAttempt,
        failure: &TransportFailure,
    ) {
        let attention = match failure {
            TransportFailure::Unreachable => return self.idle_at_stored_deadline(attempt),
            TransportFailure::Changed => {
                self.phase = Phase::Idle;
                self.stale = true;
                return;
            }
            TransportFailure::Unavailable => return self.idle_for_interval(now),
            TransportFailure::KeyLocked { key: Some(key) } => Reason::KeyLocked(*key),
            TransportFailure::KeyLocked { key: None }
            | TransportFailure::KeyProblem { key: None } => Reason::NoKeySelected(root.clone()),
            TransportFailure::KeyProblem { key: Some(key) } => Reason::KeyProblem(*key),
            TransportFailure::HostUnapproved {
                authority: Some(authority),
            } => Reason::HostUnapproved(authority.clone()),
            TransportFailure::HostUnapproved { authority: None } => {
                Reason::HostUnidentified(root.clone())
            }
            TransportFailure::ConfigurationRequired => Reason::ConfigurationRequired(root.clone()),
        };
        self.phase = Phase::Attention(attention);
    }

    fn not_eligible(&mut self, root: &Root, now: &Now, attempt: &PollAttempt, reason: NotEligible) {
        match reason {
            NotEligible::Paused => self.policy.paused = true,
            NotEligible::Disabled => self.policy.enabled = false,
            NotEligible::RecoverySuspended => self.policy.recovery_suspended = true,
            NotEligible::NotDue => {}
        }
        if reason == NotEligible::NotDue
            && let Some(next) = attempt.next_eligible_at
        {
            self.policy.next_eligible_at = Some(next);
            self.phase = Phase::Waiting {
                until: now.mono_at(next),
                why: Why::NotDue,
                op: None,
            };
            return;
        }
        if reason == NotEligible::RecoverySuspended {
            self.phase = Phase::Attention(Reason::RecoveryHeld(root.clone()));
        } else {
            self.idle_at_stored_deadline(attempt);
        }
        // The copy of the policy was wrong, or the deadline is unknown.
        self.stale = true;
    }
}
