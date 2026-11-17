//! Part B's rows 1 to 13, each alone.

use super::*;

pub(super) fn alone() -> Vec<Case> {
    use StateKind::*;
    let two = ToShare(2);
    let to_two = to_share(2);
    vec![
        case(
            "kept row",
            |s| s.kept = true,
            CantOpen,
            Some(CANT_OPEN),
            Act::Retry,
        ),
        case(
            "unavailable",
            |s| s.unavailable(),
            CantOpen,
            Some(CANT_OPEN),
            Act::Retry,
        ),
        case(
            "failed",
            |s| s.failed(),
            CantOpen,
            Some(CANT_OPEN),
            Act::Retry,
        ),
        // Row 2.
        case(
            "busy",
            |s| s.resolution(Resolution::Busy),
            BusyOpening,
            None,
            Act::Retry,
        ),
        // Row 3.
        case(
            "invalid configuration",
            |s| {
                s.invalid(
                    ConfigurationState::Invalid,
                    vec![problem(ProblemCode::InvalidPath)],
                )
            },
            NeedsFixing,
            Some(NEEDS_FIXING),
            Act::Settings,
        ),
        case(
            "missing configuration",
            |s| s.invalid(ConfigurationState::Missing, Vec::new()),
            NeedsFixing,
            Some(NEEDS_FIXING),
            Act::Settings,
        ),
        // Row 4.
        case(
            "checking now",
            |s| s.checking_now = true,
            Checking,
            Some(CHECKING),
            Act::None,
        ),
        case(
            "queued",
            |s| s.schedule().phase = Phase::Queued,
            Checking,
            Some(CHECKING),
            Act::None,
        ),
        // Row 5.
        case(
            "live locked",
            |s| s.schedule().phase = attention(Reason::KeyLocked(key())),
            Locked,
            Some(LOCKED),
            Act::Unlock,
        ),
        case(
            "stored unlock required",
            |s| s.stored = Some(PollingOutcome::UnlockRequired),
            Locked,
            Some(LOCKED),
            Act::CheckNow,
        ),
        // Row 6.
        case(
            "key problem",
            |s| s.schedule().phase = attention(Reason::KeyProblem(key())),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Attend(Action::ChooseKey),
        ),
        case(
            "no key selected",
            |s| s.schedule().phase = attention(Reason::NoKeySelected(ROOT.to_owned())),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Attend(Action::ChooseKey),
        ),
        case(
            "host unapproved",
            |s| s.schedule().phase = attention(Reason::HostUnapproved(authority())),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Attend(Action::ApproveHost),
        ),
        case(
            "configuration required",
            |s| s.schedule().phase = attention(Reason::ConfigurationRequired(ROOT.to_owned())),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Attend(Action::OpenRemoteSettings),
        ),
        case(
            "held by the phase",
            |s| s.schedule().phase = attention(Reason::RecoveryHeld(ROOT.to_owned())),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Attend(Action::ResumeChecking),
        ),
        case(
            "held by the policy",
            |s| s.schedule().policy.recovery_suspended = true,
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Attend(Action::ResumeChecking),
        ),
        case(
            "pending conflict",
            |s| s.schedule().conflict_pending = true,
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::None,
        ),
        case(
            "failed wait",
            |s| s.schedule().phase = waiting(Why::Failed),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::RetryNow,
        ),
        case(
            "unreadable",
            |s| {
                s.schedule = None;
                s.unplaced = Some(Unplaced::Unreadable);
            },
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Reread,
        ),
        case(
            "last answer: transport unavailable",
            |s| s.schedule().last = Some(attempt(Answer::Transport(TransportFailure::Unavailable))),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::RetryNow,
        ),
        case(
            "last answer: configuration required",
            |s| {
                s.schedule().last = Some(attempt(Answer::Transport(
                    TransportFailure::ConfigurationRequired,
                )));
            },
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::RetryNow,
        ),
        case(
            "stored: selected key unavailable",
            |s| s.stored = Some(PollingOutcome::SelectedKeyUnavailable),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::CheckNow,
        ),
        case(
            "stored: host approval required",
            |s| s.stored = Some(PollingOutcome::HostApprovalRequired),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::CheckNow,
        ),
        case(
            "stored: configuration required",
            |s| s.stored = Some(PollingOutcome::ConfigurationRequired),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::RetryNow,
        ),
        case(
            "stored: repository unavailable",
            |s| s.stored = Some(PollingOutcome::RepositoryUnavailable),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::RetryNow,
        ),
        case(
            "problems",
            |s| s.checked(Vec::new(), vec![dirty()]),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::None,
        ),
        // Row 7.
        case(
            "last answer: unreachable",
            |s| s.schedule().last = Some(attempt(unreachable())),
            CantReach,
            Some(CANT_REACH),
            Act::RetryNow,
        ),
        case(
            "stored: transport unavailable",
            |s| s.stored = Some(PollingOutcome::TransportUnavailable),
            CantReach,
            Some(CANT_REACH),
            Act::RetryNow,
        ),
        case(
            "stored: protocol rejected",
            |s| s.stored = Some(PollingOutcome::ProtocolRejected),
            CantReach,
            Some(CANT_REACH),
            Act::RetryNow,
        ),
        // Row 8.
        case(
            "items to share, main one of them",
            |s| {
                s.checked(
                    vec![item(1, Disposition::LocalAhead), main_ahead()],
                    Vec::new(),
                )
            },
            two,
            Some(&to_two),
            Act::None,
        ),
        // Row 9.
        case(
            "paused",
            |s| s.schedule().policy.paused = true,
            Paused,
            Some(PAUSED),
            Act::CheckNow,
        ),
        // Row 10.
        case(
            "up to date",
            |s| s.checked(Vec::new(), Vec::new()),
            UpToDate,
            Some(UP_TO_DATE),
            Act::None,
        ),
        // Rows 11 and 12.
        case(
            "no shared copy",
            |s| s.unscheduled(None),
            ThisComputer,
            Some(THIS_COMPUTER),
            Act::SetUp,
        ),
        case(
            "not ssh",
            |s| s.unscheduled(Some("origin")),
            NotSsh,
            None,
            Act::SetUp,
        ),
        // Row 13.
        case("never checked", |_| {}, Unknown, None, Act::None),
        case(
            "remotes being read",
            |s| {
                s.unscheduled(Some("origin"));
                s.unplaced = Some(Unplaced::Reading);
            },
            Unknown,
            None,
            Act::None,
        ),
        case(
            "polling off",
            |s| {
                s.unscheduled(Some("origin"));
                s.polling = false;
            },
            Unknown,
            None,
            Act::None,
        ),
        case(
            "checking soon",
            |s| s.schedule().phase = waiting(Why::Busy),
            Unknown,
            None,
            Act::None,
        ),
        case(
            "interrupted wait",
            |s| s.schedule().phase = waiting(Why::Interrupted),
            Unknown,
            None,
            Act::None,
        ),
        case(
            "unfinished operation",
            |s| s.schedule().phase = waiting(Why::Unfinished),
            Unknown,
            None,
            Act::None,
        ),
        case(
            "checking off",
            |s| s.schedule().policy.enabled = false,
            Unknown,
            None,
            Act::None,
        ),
        case(
            "stored: completed",
            |s| s.stored = Some(PollingOutcome::Completed),
            Unknown,
            None,
            Act::None,
        ),
        case(
            "stored: cancelled",
            |s| s.stored = Some(PollingOutcome::Cancelled),
            Unknown,
            None,
            Act::None,
        ),
    ]
}
