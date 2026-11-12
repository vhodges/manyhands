//! Part B's order: two facts at once, the first row that holds wins.

use super::*;

pub(super) fn pairs() -> Vec<Case> {
    use StateKind::*;
    let to_one = to_share(1);
    vec![
        case(
            "can't open over locked",
            |s| {
                s.unavailable();
                s.schedule().phase = attention(Reason::KeyLocked(key()));
            },
            CantOpen,
            Some(CANT_OPEN),
            Act::Retry,
        ),
        case(
            "can't open over unreadable, and its Retry is the resolution's",
            |s| {
                s.failed();
                s.schedule = None;
                s.unplaced = Some(Unplaced::Unreadable);
            },
            CantOpen,
            Some(CANT_OPEN),
            Act::Retry,
        ),
        case(
            "the kept row over a busy resolution",
            |s| {
                s.kept = true;
                s.resolution(Resolution::Busy);
            },
            CantOpen,
            Some(CANT_OPEN),
            Act::Retry,
        ),
        case(
            "busy over needs fixing",
            |s| {
                s.resolution(Resolution::Busy);
                s.invalid(ConfigurationState::Missing, Vec::new());
            },
            BusyOpening,
            None,
            Act::Retry,
        ),
        case(
            "needs fixing over to share",
            |s| {
                s.invalid(ConfigurationState::Invalid, Vec::new());
                s.checked(vec![main_ahead()], Vec::new());
            },
            NeedsFixing,
            Some(NEEDS_FIXING),
            Act::Settings,
        ),
        case(
            "checking over a stored unlock required",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.checking_now = true;
            },
            Checking,
            Some(CHECKING),
            Act::None,
        ),
        case(
            "checking over a live locked key",
            |s| {
                s.schedule().phase = attention(Reason::KeyLocked(key()));
                s.checking_now = true;
            },
            Checking,
            Some(CHECKING),
            Act::None,
        ),
        case(
            "checking over can't reach",
            |s| {
                s.schedule().last = Some(attempt(unreachable()));
                s.checking_now = true;
            },
            Checking,
            Some(CHECKING),
            Act::None,
        ),
        case(
            "to share with problems",
            |s| s.checked(vec![main_ahead()], vec![dirty()]),
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::None,
        ),
        case(
            "to share with paused",
            |s| {
                s.checked(vec![main_ahead()], Vec::new());
                s.schedule().policy.paused = true;
            },
            ToShare(1),
            Some(&to_one),
            Act::None,
        ),
        case(
            "to share while an unfinished operation waits",
            |s| {
                s.checked(vec![main_ahead()], Vec::new());
                s.schedule().phase = waiting(Why::Unfinished);
            },
            Unknown,
            None,
            Act::None,
        ),
        case(
            "paused over up to date",
            |s| {
                s.checked(Vec::new(), Vec::new());
                s.schedule().policy.paused = true;
            },
            Paused,
            Some(PAUSED),
            Act::CheckNow,
        ),
        case(
            "a paused repository's stored outcome does not apply",
            |s| {
                s.stored = Some(PollingOutcome::TransportUnavailable);
                s.schedule().policy.paused = true;
            },
            Paused,
            Some(PAUSED),
            Act::CheckNow,
        ),
        case(
            "an off repository's stored outcome does not apply",
            |s| {
                s.stored = Some(PollingOutcome::TransportUnavailable);
                s.schedule().policy.enabled = false;
            },
            Unknown,
            None,
            Act::None,
        ),
        case(
            "a held repository reads needs attention, not can't reach",
            |s| {
                s.stored = Some(PollingOutcome::TransportUnavailable);
                s.schedule().policy.recovery_suspended = true;
            },
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::Attend(Action::ResumeChecking),
        ),
        case(
            "a live attempt over a stored outcome",
            |s| {
                s.stored = Some(PollingOutcome::UnlockRequired);
                s.checked(Vec::new(), Vec::new());
                s.schedule().last = Some(attempt(Answer::Completed));
            },
            UpToDate,
            Some(UP_TO_DATE),
            Act::None,
        ),
        case(
            "the unshared list kept after a failed attempt",
            |s| {
                s.checked(vec![main_ahead()], Vec::new());
                s.schedule().last = Some(attempt(Answer::Interrupted));
                s.schedule().phase = waiting(Why::Interrupted);
            },
            ToShare(1),
            Some(&to_one),
            Act::None,
        ),
        case(
            "a stored outcome with a busy wait does not apply",
            |s| {
                s.stored = Some(PollingOutcome::TransportUnavailable);
                s.schedule().phase = waiting(Why::Busy);
            },
            Unknown,
            None,
            Act::None,
        ),
        case(
            "a stored outcome with an unfinished wait does not apply",
            |s| {
                s.stored = Some(PollingOutcome::TransportUnavailable);
                s.schedule().phase = waiting(Why::Unfinished);
            },
            Unknown,
            None,
            Act::None,
        ),
        case(
            "a stored outcome with a pending conflict does not apply",
            |s| {
                s.stored = Some(PollingOutcome::TransportUnavailable);
                s.schedule().conflict_pending = true;
            },
            NeedsAttention,
            Some(NEEDS_ATTENTION),
            Act::None,
        ),
    ]
}
