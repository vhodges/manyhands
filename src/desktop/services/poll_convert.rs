//! From the library's answers to the desktop's [`PollAttempt`] (D6 design
//! B1 and A4).
//!
//! An automatic poll answers `Result<RemotePollResult, RemotePollError>`.
//! [`classify`] reads that answer into [`Library`], the parts the desktop
//! needs, and [`attempt_of`] turns those into A4's rows. The split exists
//! because the library builds a `RemotePollReport` and a `RepositoryError`
//! only inside itself: a test builds `Library` for them, and the real types
//! for every other answer.
//!
//! A live failure is an error, not a completion: the library stores the
//! category and then answers `Err(Transport(e))` (Fact 6). The desktop
//! sorts `e.kind` itself (R-29), because the library's own sort
//! (`transport_category`) is crate-private and A4 draws some rows
//! differently: key source, selection, endpoint and host trust changes are
//! `Changed`, not key, configuration or host problems (Task 2 ruling).
//!
//! Nothing here holds a title, body, comment or key material: ids, kinds,
//! counts, times and dispositions only.

use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationId, PollCompletion, PollDisposition, PollIneligibility,
        RemoteOutcomeCategory, RemotePollError, RemotePollOutcome, RemotePollReport,
        RemotePollResult, RepositoryErrorKind, SharedKeyId,
        transport::{SshAuthority, SshTransportErrorKind},
    },
};
use time::OffsetDateTime;

use crate::desktop::model::polling::{
    Answer, AttemptTarget, Authority, CancelCause, Disposition, NotEligible, PollAttempt, Root,
    TargetKind, TransportFailure,
};

/// What the conversion reads of one reported item.
pub(crate) struct TargetParts {
    pub item_id: Option<ItemId>,
    pub kind: Option<AuthoringKind>,
    pub disposition: PollDisposition,
}

/// What the conversion reads of a `RemotePollReport`.
pub(crate) struct ReportParts {
    pub completion: PollCompletion,
    pub completed_at: OffsetDateTime,
    pub next_eligible_at: Option<OffsetDateTime>,
    pub targets: Vec<TargetParts>,
    /// The stored category. A live pass has `Completed` or none; a replayed
    /// terminal row can carry a failure's (Fact 6).
    pub outcome: Option<RemoteOutcomeCategory>,
}

impl ReportParts {
    fn of(report: &RemotePollReport) -> Self {
        Self {
            completion: report.completion(),
            completed_at: report.completed_at(),
            next_eligible_at: report.next_eligible_at(),
            targets: report
                .targets()
                .iter()
                .map(|target| TargetParts {
                    item_id: target.item_id().cloned(),
                    kind: target.kind(),
                    disposition: target.disposition(),
                })
                .collect(),
            outcome: report.outcome(),
        }
    }
}

/// A poll's answer, reduced to what A4 needs.
pub(crate) enum Library {
    /// `Ok(Complete(Polled(..)))`, or `Ok(IndexPending(Polled(..)))`.
    Polled {
        parts: ReportParts,
        index_pending: bool,
    },
    /// `Ok(Complete(NotEligible{..}))`, or `Ok(IndexPending(NotEligible{..}))`.
    NotEligible {
        reason: PollIneligibility,
        next_eligible_at: Option<OffsetDateTime>,
        index_pending: bool,
    },
    /// `Err(Transport(e))`, by `e.kind`, `e.selected_key_id` and
    /// `e.authority` (R-29).
    Transport {
        kind: SshTransportErrorKind,
        key: Option<SharedKeyId>,
        authority: Option<SshAuthority>,
    },
    Busy,
    /// `Err(PollYielding)`: an automatic poll never answers it (only a
    /// Manual reservation meets a Poll row, Fact 29). It is read
    /// defensively as `Busy`: another operation holds the repository.
    PollYielding,
    Interrupted,
    ConflictPending,
    /// `Err(Repository(e))`, or a read that failed before the poll.
    /// `recovery_required` is a local operation still pending (Fact 32).
    Repository {
        recovery_required: bool,
    },
    /// `Err(Repository(e))` of kind `OperationMismatch` (U-13): the row under
    /// the operation is another target's, priority's or kind's.
    Mismatch,
}

/// Reads `result` into [`Library`].
pub(crate) fn classify(result: &Result<RemotePollResult, RemotePollError>) -> Library {
    let (outcome, index_pending) = match result {
        Ok(RemotePollResult::Complete(outcome)) => (outcome, false),
        Ok(RemotePollResult::IndexPending(pending)) => (&pending.authoritative, true),
        Err(RemotePollError::Transport(error)) => {
            return Library::Transport {
                kind: error.kind.clone(),
                key: error.selected_key_id,
                authority: error.authority.clone(),
            };
        }
        Err(RemotePollError::Busy) => return Library::Busy,
        Err(RemotePollError::PollYielding) => return Library::PollYielding,
        Err(RemotePollError::Interrupted) => return Library::Interrupted,
        Err(RemotePollError::ConflictPending) => return Library::ConflictPending,
        Err(RemotePollError::Repository(error)) => {
            return match error.kind {
                RepositoryErrorKind::OperationMismatch => Library::Mismatch,
                kind => Library::Repository {
                    recovery_required: kind == RepositoryErrorKind::RecoveryRequired,
                },
            };
        }
    };
    match outcome {
        RemotePollOutcome::NotEligible {
            reason,
            next_eligible_at,
        } => Library::NotEligible {
            reason: *reason,
            next_eligible_at: *next_eligible_at,
            index_pending,
        },
        RemotePollOutcome::Polled(report) => Library::Polled {
            parts: ReportParts::of(report),
            index_pending,
        },
    }
}

/// The attempt for `result`: [`classify`] then [`attempt_of`].
///
/// `cancel` says why the poll was cancelled. The answer does not: the
/// caller does, from its own flags (a user job that preempted, quit, a
/// configuration change, a launch takeover). It is called lazily, only for
/// an answer that is `Cancelled` and at most once, so a closure may consume
/// the flags.
pub(crate) fn poll_attempt(
    root: &Root,
    operation: OperationId,
    result: &Result<RemotePollResult, RemotePollError>,
    cancel: impl FnOnce() -> CancelCause,
) -> PollAttempt {
    attempt_of(root, operation, classify(result), cancel)
}

/// A repository failure before the poll reserved anything (a read that
/// failed): no row was recorded, so there is no operation to resume.
pub(crate) fn repository_failed(
    root: &Root,
    operation: OperationId,
    recovery_required: bool,
) -> PollAttempt {
    let library = Library::Repository { recovery_required };
    attempt_of(root, operation, library, || CancelCause::Preemption)
}

/// A4's row for `library`. The attempt names the deadline its answer names
/// (`next_eligible_at`), and the caller adds the stored one when there is
/// none.
pub(crate) fn attempt_of(
    root: &Root,
    operation: OperationId,
    library: Library,
    cancel: impl FnOnce() -> CancelCause,
) -> PollAttempt {
    let mut attempt = PollAttempt {
        root: root.clone(),
        operation,
        answer: Answer::Completed,
        completed_at: None,
        next_eligible_at: None,
        targets: Vec::new(),
        index_pending: false,
    };
    match library {
        Library::Polled {
            parts,
            index_pending,
        } => {
            attempt.answer = polled_answer(&parts, cancel);
            // Every answer a report gave has its time, a replayed failure's
            // too, so L-1 can age a replayed terminal row. The model moves
            // `last_completed` only for `Answer::Completed`.
            //
            // The library replays a terminal row under the same operation
            // ID before its eligibility check (`poll_probed`), so a same-ID
            // restart of a terminal row on a repository that is now
            // ineligible gets `NotEligible` from the desktop's own
            // eligibility skip, not the stored result. That is acceptable:
            // no takeover while not eligible (L-2).
            attempt.completed_at = Some(parts.completed_at);
            attempt.next_eligible_at = parts.next_eligible_at;
            attempt.targets = parts.targets.iter().map(target).collect();
            attempt.index_pending = index_pending;
        }
        Library::NotEligible {
            reason,
            next_eligible_at,
            index_pending,
        } => {
            attempt.answer = Answer::NotEligible(match reason {
                PollIneligibility::Disabled => NotEligible::Disabled,
                PollIneligibility::Paused => NotEligible::Paused,
                PollIneligibility::RecoverySuspended => NotEligible::RecoverySuspended,
                PollIneligibility::NotDue => NotEligible::NotDue,
            });
            attempt.next_eligible_at = next_eligible_at;
            attempt.index_pending = index_pending;
        }
        Library::Transport {
            kind,
            key,
            authority,
        } => {
            attempt.answer = Answer::Transport(transport_failure(&kind, key, authority.as_ref()));
        }
        // Another operation holds the repository (see `PollYielding`).
        Library::Busy | Library::PollYielding => attempt.answer = Answer::Busy,
        Library::Interrupted => attempt.answer = Answer::Interrupted,
        Library::ConflictPending => attempt.answer = Answer::ConflictPending,
        Library::Repository { recovery_required } => {
            attempt.answer = Answer::Repository { recovery_required };
        }
        Library::Mismatch => attempt.answer = Answer::OperationMismatch,
    }
    attempt
}

/// A report's answer: its completion, and for `Completed` the category a
/// replayed terminal row carries.
fn polled_answer(parts: &ReportParts, cancel: impl FnOnce() -> CancelCause) -> Answer {
    match parts.completion {
        // `Ok(Polled{Interrupted})` and `Err(Interrupted)` are one row (A4).
        PollCompletion::Interrupted => Answer::Interrupted,
        PollCompletion::Cancelled => Answer::Cancelled(cancel()),
        PollCompletion::Completed => match parts.outcome {
            None | Some(RemoteOutcomeCategory::Completed) => Answer::Completed,
            Some(category) => category_answer(category, cancel),
        },
    }
}

/// A replayed terminal row's category, sorted to the nearest row. The row
/// kept the category, not the error: `e.selected_key_id` and `e.authority`
/// are gone, so a key or host row is per repository. A key source,
/// selection, endpoint or host trust change was sorted by the library into
/// the key, configuration or host category, so a replay shows it as one
/// (the live answer was `Changed`).
fn category_answer(
    category: RemoteOutcomeCategory,
    cancel: impl FnOnce() -> CancelCause,
) -> Answer {
    use RemoteOutcomeCategory::*;
    Answer::Transport(match category {
        // Not a failure: the caller took it above.
        Completed => return Answer::Completed,
        Cancelled => return Answer::Cancelled(cancel()),
        // Transport unavailable, protocol rejected: retry at the backoff.
        TransportUnavailable | ProtocolRejected => TransportFailure::Unreachable,
        // Unlock cancelled, provider unavailable or unlock failed.
        UnlockRequired => TransportFailure::KeyLocked { key: None },
        // No selected key, or one missing, unreadable, invalid or rejected.
        SelectedKeyUnavailable => TransportFailure::KeyProblem { key: None },
        // Host approval or replacement required, or verification unavailable.
        HostApprovalRequired => TransportFailure::HostUnapproved { authority: None },
        ConfigurationRequired => TransportFailure::ConfigurationRequired,
        // The registry was unavailable.
        RepositoryUnavailable => TransportFailure::Unavailable,
    })
}

/// A4's `Err(Transport(e))` rows. Exhaustive over every kind, with no
/// wildcard, so a kind the library adds does not build until it is sorted.
fn transport_failure(
    kind: &SshTransportErrorKind,
    key: Option<SharedKeyId>,
    authority: Option<&SshAuthority>,
) -> TransportFailure {
    use SshTransportErrorKind::*;
    match kind {
        // Transport unavailable, remote unavailable, protocol failure or
        // push rejected.
        TransportUnavailable | RemoteUnavailable | ProtocolFailure | PushRejected => {
            TransportFailure::Unreachable
        }
        // Unlock cancelled, provider unavailable or unlock failed.
        UnlockCancelled | ProviderUnavailable | UnlockFailed => TransportFailure::KeyLocked { key },
        // No selected key, or a key missing, unreadable, invalid or
        // rejected.
        NoSelectedKey | KeyMissing | KeyUnreadable | KeyInvalidOrUnsupported | KeyRejected => {
            TransportFailure::KeyProblem { key }
        }
        // Host approval or replacement required, or host verification
        // unavailable.
        HostApprovalRequired { .. }
        | HostReplacementRequired { .. }
        | HostVerificationUnavailable => TransportFailure::HostUnapproved {
            authority: authority.map(|authority| Authority {
                host: authority.host.clone(),
                port: authority.port,
            }),
        },
        // Configuration invalid, username required or publication remote
        // missing.
        ConfigurationInvalid | UsernameRequired | PublicationRemoteMissing => {
            TransportFailure::ConfigurationRequired
        }
        // Key source, selection, endpoint or host trust changed (R-11):
        // A4 draws these as `Changed`, not as the library's categories.
        KeySourceChanged | SelectionChanged | EndpointChanged | HostTrustChanged => {
            TransportFailure::Changed
        }
        // Registry unavailable or runtime uninitialized.
        RegistryUnavailable | RuntimeUninitialized => TransportFailure::Unavailable,
    }
}

fn target(parts: &TargetParts) -> AttemptTarget {
    AttemptTarget {
        item_id: parts.item_id.clone(),
        kind: parts.kind.map(|kind| match kind {
            AuthoringKind::Ticket => TargetKind::Ticket,
            AuthoringKind::Document => TargetKind::Document,
        }),
        disposition: disposition(parts.disposition),
    }
}

/// Every `PollDisposition`, one to one.
fn disposition(disposition: PollDisposition) -> Disposition {
    match disposition {
        PollDisposition::Current => Disposition::Current,
        PollDisposition::FastForwarded => Disposition::FastForwarded,
        PollDisposition::LocalAhead => Disposition::LocalAhead,
        PollDisposition::Diverged => Disposition::Diverged,
        PollDisposition::Dirty => Disposition::Dirty,
        PollDisposition::Conflicted => Disposition::Conflicted,
        PollDisposition::Inaccessible => Disposition::Inaccessible,
        PollDisposition::Materialized => Disposition::Materialized,
        PollDisposition::Nonconforming => Disposition::Nonconforming,
        PollDisposition::BlockedByLocalState => Disposition::BlockedByLocalState,
        PollDisposition::MaterializationIncomplete => Disposition::MaterializationIncomplete,
        PollDisposition::Malformed => Disposition::Malformed,
        PollDisposition::RemotelyDeleted => Disposition::RemotelyDeleted,
        PollDisposition::Closed => Disposition::Closed,
        PollDisposition::Promoted => Disposition::Promoted,
        PollDisposition::HistoryUnknown => Disposition::HistoryUnknown,
        PollDisposition::Unpublished => Disposition::Unpublished,
        PollDisposition::NotReached => Disposition::NotReached,
    }
}

#[cfg(test)]
#[path = "poll_convert_tests.rs"]
mod tests;
