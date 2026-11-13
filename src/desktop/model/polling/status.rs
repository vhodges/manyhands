//! The words of a Settings row (D6 design A4's text column, C2, C4, L-2):
//! pure, from a repository's schedule and a few facts around it.
//!
//! The status line comes from the schedule's policy first (off, held,
//! paused) and then from its phase and the last attempt, never from a guess
//! about another process (C4): "Checking…" is said only while this process's
//! worker runs a poll. A repository the scheduler holds no schedule for has
//! no shared copy to check, or has not been read yet.
//!
//! No text here names an item, a comment or a key: only the repository's
//! name, which the Settings pages already show.

use manyhands::repository::PollingOutcome;
use time::OffsetDateTime;

use super::{Answer, AttemptSummary, Phase, Reason, RepoSchedule, TransportFailure, Why};
use crate::desktop::services::scheduler::Holding;

pub(crate) const NO_SHARED_COPY: &str = "No shared copy, nothing to check";
pub(crate) const CHECKING_IS_OFF: &str = "Checking is off";
pub(crate) const PAUSED: &str = "Paused";
pub(crate) const HELD: &str = "Checking is held until recovery";
pub(crate) const CHECKING: &str = "Checking\u{2026}";
pub(crate) const CHECKING_SOON: &str = "Checking soon";
pub(crate) const WAITING_FOR_OTHER: &str = "Waiting for another operation to finish";
pub(crate) const WAITING_FOR_UNFINISHED: &str = "Waiting for an unfinished operation";
pub(crate) const WAITING_FOR_CONFLICT: &str = "Waiting for the conflict to be resolved";
pub(crate) const CANT_CHECK: &str = "Can't check";
pub(crate) const NEEDS_UNLOCKING: &str = "Needs unlocking";
pub(crate) const CHOOSE_A_KEY: &str = "Choose a key";
pub(crate) const NEEDS_HOST_APPROVAL: &str = "Needs host approval";
pub(crate) const NOT_CHECKED_YET: &str = "Not checked yet";
pub(crate) const RETIRING: &str = "Removing finished copies\u{2026}";
pub(crate) const READING: &str = "Reading the remotes";

/// The separator between a status line's parts.
const THEN: &str = " \u{b7} ";

/// Every interval the select offers, in seconds and in words: the library's
/// 60 to 3600 s (C2). Nothing else is offered.
pub(crate) const INTERVALS: [(u64, &str); 6] = [
    (60, "Every 1 minute"),
    (300, "Every 5 minutes"),
    (600, "Every 10 minutes"),
    (900, "Every 15 minutes"),
    (1800, "Every 30 minutes"),
    (3600, "Every 60 minutes"),
];

/// A stored interval in words: the offered ones as offered, any other as
/// the library holds it.
pub(crate) fn interval_words(seconds: u64) -> String {
    if let Some((_, words)) = INTERVALS.iter().find(|(offered, _)| *offered == seconds) {
        return (*words).to_owned();
    }
    if seconds.is_multiple_of(60) {
        format!("Every {} minutes", seconds / 60)
    } else {
        format!("Every {seconds} seconds")
    }
}

/// "Can't reach {repository}. Your work is safe."
pub(crate) fn cant_reach(name: &str) -> String {
    format!("Can't reach {name}. Your work is safe.")
}

/// L-2's first sentence, which every cause continues.
const INTERRUPTED: &str = "An interrupted check is holding this repository.";

/// Why the scheduler has no schedule for a repository yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Unplaced {
    /// Its remotes are being read.
    Reading,
    /// The read failed; it is read again on window activation.
    Unreadable,
}

/// What a row is made from.
pub(crate) struct Facts<'a> {
    /// The repository's name, for "Can't reach {repository}".
    pub name: &'a str,
    /// `None`: the scheduler holds none, because the repository has no SSH
    /// publication remote (or `unplaced` says why not yet).
    pub schedule: Option<&'a RepoSchedule>,
    pub unplaced: Option<Unplaced>,
    /// Why a stale row the in-flight record holds is not taken over (L-2).
    pub holding: Option<Holding>,
    /// This process's own Check now of the repository is queued or running.
    pub checking_now: bool,
    /// How the library's latest check of the repository ended, as stored
    /// before this process ran (the scheduler's side map). It speaks only
    /// where [`stored_applies`] says.
    pub stored: Option<PollingOutcome>,
    pub now: OffsetDateTime,
}

/// The one predicate that decides whether a stored outcome counts (R-32):
/// checking is on, the repository is not held or paused and polls, this
/// process has seen no attempt, nothing is checking, the phase is `Idle`,
/// `Stopped` or waiting for a deadline, and no conflict is pending. It is
/// exactly where Settings' row reaches [`since_library`], so the row and
/// the panel say the same.
pub(crate) fn stored_applies(facts: &Facts) -> Option<PollingOutcome> {
    stored_when(facts, true)
}

/// The stored outcome as Check all asks for it (owner ruling 2026-11-12,
/// decision 16): every condition of [`stored_applies`] except that the
/// repository is not paused and that its phase is quiet, so a paused
/// repository, or one in a busy, interrupted or failed wait, whose stored
/// outcome waits on a key or a host is still known to, and Check all never
/// raises a prompt for it.
pub(crate) fn stored_for_check_all(facts: &Facts) -> Option<PollingOutcome> {
    stored_when(facts, false)
}

/// `stored_applies`' conditions; `strict` adds "not paused" and "quiet".
fn stored_when(facts: &Facts, strict: bool) -> Option<PollingOutcome> {
    let stored = facts.stored?;
    let repo = facts.schedule?;
    let held = repo.policy.recovery_suspended
        || matches!(&repo.phase, Phase::Attention(Reason::RecoveryHeld(_)));
    let quiet = matches!(
        &repo.phase,
        Phase::Idle
            | Phase::Stopped
            | Phase::Waiting {
                why: Why::NotDue,
                ..
            }
    );
    (repo.policy.enabled
        && !held
        && (!strict || (!repo.policy.paused && quiet))
        && repo.last.is_none()
        && !facts.checking_now
        && !repo.in_flight()
        && !repo.conflict_pending)
        .then_some(stored)
}

/// What the row offers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Shape {
    /// "No shared copy": Set up sharing….
    NoSharedCopy,
    /// Status only: not read yet, or unreadable.
    Pending,
    /// `enabled` is off: no verb sets it, so no control (K-5).
    Off,
    /// Held until recovery; Resume checking is the attention row's (C3).
    Held,
    /// The interval, Check now and the switch.
    Polling(Controls),
}

/// The controls of a repository that polls.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Controls {
    /// The switch is off.
    pub paused: bool,
    pub interval_seconds: u64,
    /// The action is Retry now: the status says a check failed.
    pub retry: bool,
    /// This process's own Check now is queued or running: the only time
    /// Check now is disabled (C1).
    pub checking: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Row {
    pub status: String,
    /// L-2's sentence, when an interrupted check holds the repository.
    pub detail: Option<String>,
    /// The status waits for an unfinished operation, whose pending row
    /// Activity lists: the row links to it (A4).
    pub pending: bool,
    pub shape: Shape,
}

/// How long ago `then` was, as of `now`.
pub(crate) fn ago(then: OffsetDateTime, now: OffsetDateTime) -> String {
    let minutes = (now - then).whole_minutes().max(0);
    let (count, unit) = match minutes {
        0 => return "just now".to_owned(),
        1..=59 => (minutes, "minute"),
        60..=1439 => (minutes / 60, "hour"),
        _ => (minutes / 1440, "day"),
    };
    format!("{count} {unit}{} ago", if count == 1 { "" } else { "s" })
}

pub(super) fn changes(count: u32) -> String {
    format!(
        "Brought in {count} change{}",
        if count == 1 { "" } else { "s" }
    )
}

/// "Checked {ago} \u{b7} up to date" or "\u{b7} Brought in N changes". What
/// is not shared yet is said by every line alike (`counted`).
fn checked(summary: &AttemptSummary, repo: &RepoSchedule, now: OffsetDateTime) -> String {
    let when = summary.completed_at.or(repo.last_completed);
    let mut line = match when {
        Some(when) => format!("Checked {}", ago(when, now)),
        None => "Checked".to_owned(),
    };
    line.push_str(THEN);
    if summary.brought_in > 0 {
        line.push_str(&changes(summary.brought_in));
    } else {
        line.push_str("up to date");
    }
    line
}

/// `status`, and "\u{b7} N not shared yet" from the unshared list when it
/// holds anything, whatever the status says and before any completed check
/// (decision 24).
fn counted(status: String, repo: &RepoSchedule) -> String {
    match repo.unshared.items.len() {
        0 => status,
        count => format!("{status}{THEN}{count} not shared yet"),
    }
}

/// The status of a repository the schedule is idle for, from the last
/// recorded attempt.
fn after_last(repo: &RepoSchedule, facts: &Facts) -> (String, bool) {
    if repo.conflict_pending {
        return (WAITING_FOR_CONFLICT.to_owned(), false);
    }
    let Some(last) = &repo.last else {
        return since_library(repo, facts);
    };
    match &last.answer {
        Answer::Completed => (checked(last, repo, facts.now), false),
        Answer::Transport(TransportFailure::Unreachable) => (cant_reach(facts.name), true),
        Answer::Transport(TransportFailure::ConfigurationRequired)
        | Answer::Transport(TransportFailure::Unavailable) => (CANT_CHECK.to_owned(), true),
        // Attention was cleared, or the key, host or endpoint changed: the
        // next check tries again.
        Answer::Transport(_) | Answer::Interrupted | Answer::Cancelled(_) => {
            (CHECKING_SOON.to_owned(), false)
        }
        _ => since_library(repo, facts),
    }
}

/// What the library stored says, when this process has seen no attempt of
/// its own: the outcome [`stored_applies`] gives, in Settings' words and
/// with whether the action is Retry now; else the stored time.
fn since_library(repo: &RepoSchedule, facts: &Facts) -> (String, bool) {
    use PollingOutcome::*;
    match stored_applies(facts) {
        Some(TransportUnavailable | ProtocolRejected) => return (cant_reach(facts.name), true),
        Some(UnlockRequired) => return (NEEDS_UNLOCKING.to_owned(), false),
        Some(SelectedKeyUnavailable) => return (CHOOSE_A_KEY.to_owned(), false),
        Some(HostApprovalRequired) => return (NEEDS_HOST_APPROVAL.to_owned(), false),
        Some(ConfigurationRequired | RepositoryUnavailable) => {
            return (CANT_CHECK.to_owned(), true);
        }
        // Nothing stored of these that today's words do not say.
        Some(Completed | Cancelled) | None => {}
    }
    let words = match repo.last_completed {
        Some(when) => format!("Checked {}", ago(when, facts.now)),
        None => NOT_CHECKED_YET.to_owned(),
    };
    (words, false)
}

/// What needs the person, in words. Its action is the attention row's.
fn attention(reason: &Reason) -> (&'static str, bool) {
    match reason {
        Reason::KeyLocked(_) => (NEEDS_UNLOCKING, false),
        Reason::KeyProblem(_) | Reason::NoKeySelected(_) => (CHOOSE_A_KEY, false),
        Reason::HostUnapproved(_) | Reason::HostUnidentified(_) => (NEEDS_HOST_APPROVAL, false),
        Reason::ConfigurationRequired(_) => (CANT_CHECK, true),
        Reason::RecoveryHeld(_) => (HELD, false),
    }
}

fn phase_status(repo: &RepoSchedule, facts: &Facts) -> (String, bool) {
    let say = |words: &str| (words.to_owned(), false);
    match &repo.phase {
        Phase::Queued | Phase::Running { .. } => say(CHECKING),
        Phase::Retiring => say(RETIRING),
        Phase::Preempted => say(CHECKING_SOON),
        Phase::Attention(reason) => {
            let (words, failed) = attention(reason);
            (words.to_owned(), failed)
        }
        Phase::Waiting { why, op, .. } => match why {
            // The restart steps of R-4 resend the same operation; once they
            // are used up the wait is the interval, with a new one.
            Why::Busy if op.is_none() => say(WAITING_FOR_OTHER),
            Why::Busy | Why::Interrupted => say(CHECKING_SOON),
            Why::Unfinished => say(WAITING_FOR_UNFINISHED),
            Why::Failed => (CANT_CHECK.to_owned(), true),
            Why::NotDue => since_library(repo, facts),
        },
        Phase::Idle | Phase::Stopped => after_last(repo, facts),
    }
}

fn detail(holding: Holding) -> String {
    let rest = match holding {
        Holding::Off => "It finishes once checking is on.",
        Holding::Paused => "Turn checking on to finish it.",
        Holding::Held => "Resume checking to finish it.",
        Holding::NotDue => "It finishes at the next check.",
        Holding::Attention => "It finishes once what needs your attention is dealt with.",
    };
    format!("{INTERRUPTED} {rest}")
}

/// The row for `facts`.
pub(crate) fn row(facts: &Facts) -> Row {
    let pending = |words: &str| Row {
        status: words.to_owned(),
        detail: None,
        pending: false,
        shape: Shape::Pending,
    };
    let Some(repo) = facts.schedule else {
        return match facts.unplaced {
            Some(Unplaced::Reading) => pending(READING),
            Some(Unplaced::Unreadable) => pending(CANT_CHECK),
            None => Row {
                status: NO_SHARED_COPY.to_owned(),
                detail: None,
                pending: false,
                shape: Shape::NoSharedCopy,
            },
        };
    };
    let detail = facts.holding.map(detail);
    let held = repo.policy.recovery_suspended
        || matches!(&repo.phase, Phase::Attention(Reason::RecoveryHeld(_)));
    if !repo.policy.enabled {
        return Row {
            status: counted(CHECKING_IS_OFF.to_owned(), repo),
            detail,
            pending: false,
            shape: Shape::Off,
        };
    }
    if held {
        return Row {
            status: counted(HELD.to_owned(), repo),
            detail,
            pending: false,
            shape: Shape::Held,
        };
    }
    let (mut status, mut retry) = if facts.checking_now {
        (CHECKING.to_owned(), false)
    } else {
        phase_status(repo, facts)
    };
    if repo.policy.paused && !facts.checking_now {
        // Pause and attention are independent (A3): both are said.
        status = match repo.phase {
            Phase::Attention(_) => format!("{PAUSED}{THEN}{status}"),
            _ => PAUSED.to_owned(),
        };
        retry = false;
    }
    Row {
        status: counted(status, repo),
        detail,
        pending: !facts.checking_now
            && matches!(
                repo.phase,
                Phase::Waiting {
                    why: Why::Unfinished,
                    ..
                }
            ),
        shape: Shape::Polling(Controls {
            paused: repo.policy.paused,
            interval_seconds: repo.policy.interval.as_secs(),
            retry,
            checking: facts.checking_now,
        }),
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
