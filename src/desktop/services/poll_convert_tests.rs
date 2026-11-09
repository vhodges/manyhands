//! The conversion from library answers to `PollAttempt` (D6 B1, A4, R-29).
//!
//! The library builds a report and a repository error only inside itself,
//! so those rows are built as `Library` values (decision 9); every other
//! answer is the library's own type.

use manyhands::repository::{
    AuthoringKind, IndexPending, OperationId, PollCompletion, PollDisposition, PollIneligibility,
    RemoteOutcomeCategory, RemotePollError, RemotePollOutcome, RemotePollResult,
    RepositoryErrorKind, SharedKeyId,
    transport::{
        HostKeyIdentity, SshAuthority, SshDirection, SshTransportError, SshTransportErrorKind,
    },
};
use time::{Duration, OffsetDateTime};

use super::{Library, ReportParts, TargetParts, attempt_of, poll_attempt};
use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, Authority, CancelCause, Disposition, NotEligible, TargetKind,
        TransportFailure,
    },
    test_support::fixtures::item_id,
};

const TICKET: &str = "01M6KHVD6D0DB8KYT6VGQP10Y0";
const DOCUMENT: &str = "01M4CC0VN3VPVVQ3RNWJKXWEVX";

fn root() -> String {
    "/repo".to_owned()
}

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH + Duration::seconds(seconds)
}

fn identity() -> HostKeyIdentity {
    HostKeyIdentity {
        algorithm: "ssh-ed25519".to_owned(),
        sha256: "SHA256:fixture".to_owned(),
    }
}

#[allow(clippy::result_large_err)] // The library's `RemotePollError`.
fn error(
    kind: SshTransportErrorKind,
    key: Option<SharedKeyId>,
    authority: Option<&SshAuthority>,
) -> Result<RemotePollResult, RemotePollError> {
    Err(RemotePollError::Transport(SshTransportError {
        root: "/repo".into(),
        remote_name: "origin".to_owned(),
        direction: SshDirection::Fetch,
        selected_key_id: key,
        authority: authority.cloned(),
        kind,
    }))
}

fn parts(completion: PollCompletion, outcome: Option<RemoteOutcomeCategory>) -> ReportParts {
    ReportParts {
        completion,
        completed_at: at(100),
        next_eligible_at: Some(at(400)),
        targets: Vec::new(),
        outcome,
    }
}

fn polled(parts: ReportParts, index_pending: bool) -> Library {
    Library::Polled {
        parts,
        index_pending,
    }
}

fn answer_of(library: Library) -> Answer {
    attempt_of(
        &root(),
        OperationId::new(),
        library,
        CancelCause::Preemption,
    )
    .answer
}

/// Every library answer reaches the A4 row the design draws for it.
#[test]
fn poll_attempt_conversion() {
    let operation = OperationId::new();
    let key = SharedKeyId::new();
    let host = SshAuthority {
        host: "example.test".to_owned(),
        port: 2222,
    };
    let authority = Authority {
        host: "example.test".to_owned(),
        port: 2222,
    };
    let convert = |result: &Result<RemotePollResult, RemotePollError>| {
        poll_attempt(&root(), operation, result, CancelCause::Quit)
    };

    // Err(Transport(e)): every one of the 24 kinds, keyed by
    // `selected_key_id` and `authority` (R-29).
    use SshTransportErrorKind::*;
    use TransportFailure as F;
    let locked = F::KeyLocked { key: Some(key) };
    let problem = F::KeyProblem { key: Some(key) };
    let hosts = F::HostUnapproved {
        authority: Some(authority.clone()),
    };
    let kinds = [
        (TransportUnavailable, F::Unreachable),
        (RemoteUnavailable, F::Unreachable),
        (ProtocolFailure, F::Unreachable),
        (PushRejected, F::Unreachable),
        (UnlockCancelled, locked.clone()),
        (ProviderUnavailable, locked.clone()),
        (UnlockFailed, locked),
        (NoSelectedKey, problem.clone()),
        (KeyMissing, problem.clone()),
        (KeyUnreadable, problem.clone()),
        (KeyInvalidOrUnsupported, problem.clone()),
        (KeyRejected, problem),
        (
            HostApprovalRequired {
                presented: identity(),
            },
            hosts.clone(),
        ),
        (
            HostReplacementRequired {
                expected: identity(),
                presented: identity(),
            },
            hosts.clone(),
        ),
        (HostVerificationUnavailable, hosts),
        (ConfigurationInvalid, F::ConfigurationRequired),
        (UsernameRequired, F::ConfigurationRequired),
        (PublicationRemoteMissing, F::ConfigurationRequired),
        (KeySourceChanged, F::Changed),
        (SelectionChanged, F::Changed),
        (EndpointChanged, F::Changed),
        (HostTrustChanged, F::Changed),
        (RegistryUnavailable, F::Unavailable),
        (RuntimeUninitialized, F::Unavailable),
    ];
    assert_eq!(
        kinds.len(),
        24,
        "every SshTransportErrorKind is in the table"
    );
    for (kind, expected) in kinds {
        let attempt = convert(&error(kind.clone(), Some(key), Some(&host)));
        assert_eq!(attempt.answer, Answer::Transport(expected), "{kind:?}");
        assert_eq!(attempt.root, root());
        assert_eq!(attempt.operation, operation);
        assert_eq!(attempt.completed_at, None);
        assert!(attempt.targets.is_empty());
    }
    // No key and no host: the row is per repository (A4).
    let bare = convert(&error(UnlockFailed, None, None));
    assert_eq!(bare.answer, Answer::Transport(F::KeyLocked { key: None }));
    let bare = convert(&error(HostTrustChanged, None, None));
    assert_eq!(bare.answer, Answer::Transport(F::Changed));
    let bare = convert(&error(HostVerificationUnavailable, Some(key), None));
    assert_eq!(
        bare.answer,
        Answer::Transport(F::HostUnapproved { authority: None })
    );

    // The other errors. An automatic poll never answers PollYielding: it
    // reads as Busy.
    for (result, expected) in [
        (Err(RemotePollError::Busy), Answer::Busy),
        (Err(RemotePollError::PollYielding), Answer::Busy),
        (Err(RemotePollError::Interrupted), Answer::Interrupted),
        (
            Err(RemotePollError::ConflictPending),
            Answer::ConflictPending,
        ),
    ] {
        assert_eq!(convert(&result).answer, expected);
    }
    // Err(Repository(_)): recovery_required against any other code.
    for (kind, recovery_required) in [
        (RepositoryErrorKind::RecoveryRequired, true),
        (RepositoryErrorKind::IndexUnavailable, false),
        (RepositoryErrorKind::NotRepository, false),
    ] {
        assert_eq!(
            answer_of(Library::Repository(kind)),
            Answer::Repository { recovery_required },
            "{kind:?}"
        );
    }

    // Complete(NotEligible{reason, next}), also under IndexPending.
    for (reason, expected) in [
        (PollIneligibility::Disabled, NotEligible::Disabled),
        (PollIneligibility::Paused, NotEligible::Paused),
        (
            PollIneligibility::RecoverySuspended,
            NotEligible::RecoverySuspended,
        ),
        (PollIneligibility::NotDue, NotEligible::NotDue),
    ] {
        let outcome = RemotePollOutcome::NotEligible {
            reason,
            next_eligible_at: Some(at(900)),
        };
        let attempt = convert(&Ok(RemotePollResult::Complete(outcome.clone())));
        assert_eq!(attempt.answer, Answer::NotEligible(expected));
        assert_eq!(attempt.next_eligible_at, Some(at(900)));
        assert!(!attempt.index_pending);
        let pending = convert(&Ok(RemotePollResult::IndexPending(IndexPending::new(
            outcome,
        ))));
        assert_eq!(pending.answer, Answer::NotEligible(expected));
        assert!(pending.index_pending);
    }

    // A report: its completion, with the cancel cause the caller names.
    for (completion, expected) in [
        (PollCompletion::Completed, Answer::Completed),
        (PollCompletion::Interrupted, Answer::Interrupted),
        (
            PollCompletion::Cancelled,
            Answer::Cancelled(CancelCause::Preemption),
        ),
    ] {
        let attempt = attempt_of(
            &root(),
            operation,
            polled(
                parts(completion, Some(RemoteOutcomeCategory::Completed)),
                false,
            ),
            CancelCause::Preemption,
        );
        assert_eq!(attempt.answer, expected);
        assert_eq!(attempt.next_eligible_at, Some(at(400)));
        // Only a completion has a time.
        let completed = expected == Answer::Completed;
        assert_eq!(attempt.completed_at, completed.then_some(at(100)));
    }
    assert_eq!(
        attempt_of(
            &root(),
            operation,
            polled(parts(PollCompletion::Cancelled, None), true),
            CancelCause::Takeover,
        )
        .answer,
        Answer::Cancelled(CancelCause::Takeover)
    );
    // The index refresh that did not finish.
    let attempt = attempt_of(
        &root(),
        operation,
        polled(parts(PollCompletion::Completed, None), true),
        CancelCause::Quit,
    );
    assert_eq!(
        (attempt.answer, attempt.index_pending),
        (Answer::Completed, true)
    );

    // A replayed terminal row: Completed with a category (Fact 6).
    use RemoteOutcomeCategory as C;
    for (category, expected) in [
        (C::Completed, Answer::Completed),
        (C::TransportUnavailable, Answer::Transport(F::Unreachable)),
        (C::ProtocolRejected, Answer::Transport(F::Unreachable)),
        (
            C::UnlockRequired,
            Answer::Transport(F::KeyLocked { key: None }),
        ),
        (
            C::SelectedKeyUnavailable,
            Answer::Transport(F::KeyProblem { key: None }),
        ),
        (
            C::HostApprovalRequired,
            Answer::Transport(F::HostUnapproved { authority: None }),
        ),
        (
            C::ConfigurationRequired,
            Answer::Transport(F::ConfigurationRequired),
        ),
        (C::RepositoryUnavailable, Answer::Transport(F::Unavailable)),
        (
            C::Cancelled,
            Answer::Cancelled(CancelCause::Reconfiguration),
        ),
    ] {
        let attempt = attempt_of(
            &root(),
            operation,
            polled(parts(PollCompletion::Completed, Some(category)), false),
            CancelCause::Reconfiguration,
        );
        assert_eq!(attempt.answer, expected, "{category:?}");
        assert_eq!(
            attempt.completed_at.is_some(),
            expected == Answer::Completed,
            "{category:?}"
        );
    }

    // Targets: all 18 dispositions, with the item and kind when given.
    use Disposition as D;
    use PollDisposition as P;
    let all = [
        (P::Current, D::Current),
        (P::FastForwarded, D::FastForwarded),
        (P::LocalAhead, D::LocalAhead),
        (P::Diverged, D::Diverged),
        (P::Dirty, D::Dirty),
        (P::Conflicted, D::Conflicted),
        (P::Inaccessible, D::Inaccessible),
        (P::Materialized, D::Materialized),
        (P::Nonconforming, D::Nonconforming),
        (P::BlockedByLocalState, D::BlockedByLocalState),
        (P::MaterializationIncomplete, D::MaterializationIncomplete),
        (P::Malformed, D::Malformed),
        (P::RemotelyDeleted, D::RemotelyDeleted),
        (P::Closed, D::Closed),
        (P::Promoted, D::Promoted),
        (P::HistoryUnknown, D::HistoryUnknown),
        (P::Unpublished, D::Unpublished),
        (P::NotReached, D::NotReached),
    ];
    assert_eq!(all.len(), 18);
    let mut report = parts(PollCompletion::Completed, Some(C::Completed));
    for (at, (disposition, _)) in all.iter().enumerate() {
        let (item, kind) = match at % 3 {
            0 => (Some(TICKET), Some(AuthoringKind::Ticket)),
            1 => (Some(DOCUMENT), Some(AuthoringKind::Document)),
            _ => (None, None),
        };
        report.targets.push(TargetParts {
            item_id: item.map(item_id),
            kind,
            disposition: *disposition,
        });
    }
    let attempt = attempt_of(&root(), operation, polled(report, false), CancelCause::Quit);
    let expected: Vec<AttemptTarget> = all
        .iter()
        .enumerate()
        .map(|(at, (_, disposition))| AttemptTarget {
            item_id: match at % 3 {
                0 => Some(item_id(TICKET)),
                1 => Some(item_id(DOCUMENT)),
                _ => None,
            },
            kind: match at % 3 {
                0 => Some(TargetKind::Ticket),
                1 => Some(TargetKind::Document),
                _ => None,
            },
            disposition: *disposition,
        })
        .collect();
    assert_eq!(attempt.targets, expected);
}
