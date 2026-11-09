//! The desktop's own account of one automatic poll (D6 design B1, A4).
//!
//! A [`PollAttempt`] holds ids, kinds, counts, times and dispositions, and
//! nothing a person wrote: no title, body, comment or key material. Task 3
//! fills it from the library's answers; the model only reads it. The
//! classification of the answer is the desktop's, so the table in `table.rs`
//! needs no library error type.

use manyhands::{canonical::ItemId, repository::OperationId, repository::SharedKeyId};
use time::OffsetDateTime;

use crate::desktop::model::stamp::RepoKey;

/// A repository, by its registration root.
pub(crate) type Root = RepoKey;
/// A shared SSH key.
pub(crate) type KeyId = SharedKeyId;

/// A remote's host and port: what a host approval is for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Authority {
    pub host: String,
    pub port: u16,
}

/// What an `Err(Transport)` answer comes to (A4's rows; Task 3 sorts the
/// library's `SshTransportErrorKind` into them). Every kind lands in
/// exactly one.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TransportFailure {
    /// Transport unavailable, remote unavailable, protocol failure or push
    /// rejected: retry at the stored backoff deadline.
    Unreachable,
    /// Unlock cancelled, provider unavailable or unlock failed.
    KeyLocked { key: Option<KeyId> },
    /// No selected key, or one missing, unreadable, invalid or rejected.
    KeyProblem { key: Option<KeyId> },
    /// Host approval or replacement required, or host verification
    /// unavailable.
    HostUnapproved { authority: Option<Authority> },
    /// Configuration invalid, username required or publication remote
    /// missing.
    ConfigurationRequired,
    /// Key source, selection, endpoint or host trust changed.
    Changed,
    /// Registry unavailable or runtime uninitialized.
    Unavailable,
}

/// Why a poll answered `Cancelled`.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CancelCause {
    /// A user job asked for the worker (R-3).
    Preemption,
    /// Quit.
    Quit,
    /// A configuration change (R-11).
    Reconfiguration,
    /// A launch takeover's stored cancel (L-1).
    Takeover,
}

/// Why the library would not poll (`PollIneligibility`), with the deadline
/// it names.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotEligible {
    Paused,
    Disabled,
    RecoverySuspended,
    NotDue,
}

/// The answer, classified into A4's rows.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// `Ok(Polled{Completed})`, or a replayed terminal row's category
    /// already sorted by Task 3.
    Completed,
    /// `Err(Transport(e))`.
    Transport(TransportFailure),
    /// `Ok(Polled{Cancelled})`.
    Cancelled(CancelCause),
    /// `Ok(Polled{Interrupted})` or `Err(Interrupted)`: the same row of A4.
    Interrupted,
    /// `Err(Busy)`.
    Busy,
    /// `Err(Repository(_))`; `recovery_required` is a local operation still
    /// pending (Fact 32).
    Repository { recovery_required: bool },
    /// `Err(ConflictPending)`.
    ConflictPending,
    /// `Complete(NotEligible{reason, _})`.
    NotEligible(NotEligible),
}

/// What a poll saw of one item (`PollDisposition`).
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Disposition {
    Current,
    FastForwarded,
    LocalAhead,
    Diverged,
    Dirty,
    Conflicted,
    Inaccessible,
    Materialized,
    Nonconforming,
    BlockedByLocalState,
    MaterializationIncomplete,
    Malformed,
    RemotelyDeleted,
    Closed,
    Promoted,
    HistoryUnknown,
    Unpublished,
    NotReached,
}

/// A ticket or a document.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetKind {
    Ticket,
    Document,
}

/// One item a poll reported: its ID, its kind and what happened to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttemptTarget {
    pub item_id: Option<ItemId>,
    pub kind: Option<TargetKind>,
    pub disposition: Disposition,
}

/// One automatic poll, as the desktop understands it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PollAttempt {
    pub root: Root,
    pub operation: OperationId,
    pub answer: Answer,
    /// The library's `completed_at`; only a `Completed` answer has one.
    pub completed_at: Option<OffsetDateTime>,
    /// The deadline the answer names or the stored one, when it is known.
    pub next_eligible_at: Option<OffsetDateTime>,
    pub targets: Vec<AttemptTarget>,
    /// The poll completed but its index refresh did not (R-10).
    pub index_pending: bool,
}

/// What the model keeps of an attempt (A2 `last`), and hands back to its
/// caller to act on (counts, the repository changed, what Part E retires).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttemptSummary {
    pub answer: Answer,
    pub completed_at: Option<OffsetDateTime>,
    /// `FastForwarded` and `Materialized`.
    pub brought_in: u32,
    /// `Current` and `LocalAhead`.
    pub up_to_date: u32,
    /// `LocalAhead`: "N not shared yet".
    pub not_shared: u32,
    /// Listed under the attempt entry with their text; none is an error.
    pub listed: Vec<AttemptTarget>,
    /// `Closed` and `Promoted`, from a `Completed` attempt only (E1).
    pub retire: Vec<AttemptTarget>,
    pub index_pending: bool,
}

impl AttemptSummary {
    /// The repository changed: something was brought in, even by a
    /// cancelled or interrupted attempt.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
    pub fn changed(&self) -> bool {
        self.brought_in > 0
    }

    pub fn of(attempt: &PollAttempt) -> Self {
        let completed = attempt.answer == Answer::Completed;
        let mut summary = Self {
            answer: attempt.answer.clone(),
            completed_at: completed.then_some(attempt.completed_at).flatten(),
            brought_in: 0,
            up_to_date: 0,
            not_shared: 0,
            listed: Vec::new(),
            retire: Vec::new(),
            index_pending: attempt.index_pending,
        };
        for target in &attempt.targets {
            use Disposition::*;
            match target.disposition {
                FastForwarded | Materialized => summary.brought_in += 1,
                Current => summary.up_to_date += 1,
                LocalAhead => {
                    summary.up_to_date += 1;
                    summary.not_shared += 1;
                }
                Closed | Promoted
                    if completed && target.item_id.is_some() && target.kind.is_some() =>
                {
                    summary.retire.push(target.clone());
                }
                _ => summary.listed.push(target.clone()),
            }
        }
        summary
    }

    /// Whether an answer that ended an attempt replaces `last`: one that
    /// reached the remote and said how it went, or a cancelled one that
    /// brought something in or listed something. A busy or not-eligible
    /// answer, a repository error and an empty cancel are not attempts.
    pub fn is_recorded(&self) -> bool {
        match self.answer {
            Answer::Completed | Answer::Transport(_) | Answer::Interrupted => true,
            Answer::Cancelled(_) => self.brought_in > 0 || !self.listed.is_empty(),
            _ => false,
        }
    }
}
