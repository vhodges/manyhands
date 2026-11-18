//! The schedule model's tests: the fixture and A4's table. The clock is
//! moved by hand; nothing sleeps. The summary counts and R-4's schedule are
//! in `polling_schedule_tests`, the decisions in `polling_decisions_tests`
//! and `polling_attention_tests`.

use std::time::Duration;

use manyhands::repository::{OperationId, SharedKeyId};
use time::OffsetDateTime;

use super::{clock::TestClock, *};

#[path = "polling_attention_tests.rs"]
mod attention;
#[path = "polling_brought_tests.rs"]
mod brought;
#[path = "polling_decisions_tests.rs"]
mod decisions;
#[path = "polling_retiring_tests.rs"]
mod retiring;
#[path = "polling_schedule_tests.rs"]
mod schedule;

pub(super) struct Fx {
    pub clock: TestClock,
    pub schedule: PollSchedule,
}

pub(super) fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

pub(super) fn root(name: &str) -> Root {
    format!("/{name}")
}

/// Enabled, unpaused, a 300 s interval and the given stored deadline.
pub(super) fn policy(next: Option<OffsetDateTime>) -> Policy {
    Policy {
        enabled: true,
        paused: false,
        interval: secs(300),
        backoff: None,
        recovery_suspended: false,
        next_eligible_at: next,
        active_operation: None,
    }
}

impl Fx {
    pub fn new() -> Self {
        Self {
            clock: TestClock::new(),
            schedule: PollSchedule::default(),
        }
    }

    pub fn now(&self) -> Now {
        self.clock.now()
    }

    /// The UTC time `seconds` from now, negative for the past.
    pub fn at(&self, seconds: i64) -> OffsetDateTime {
        self.clock.utc() + time::Duration::seconds(seconds)
    }

    pub fn add(&mut self, name: &str, policy: Policy) {
        self.schedule.add(root(name), policy, None);
    }

    pub fn repo(&self, name: &str) -> &RepoSchedule {
        self.schedule.repo(&root(name)).expect("a repository")
    }

    pub fn phase(&self, name: &str) -> Phase {
        self.repo(name).phase.clone()
    }

    pub fn due(&self) -> Vec<Root> {
        self.schedule.due(&self.now())
    }

    /// The repository's poll runs under a fresh operation and answers.
    pub fn answer(
        &mut self,
        name: &str,
        answer: Answer,
        next: Option<OffsetDateTime>,
    ) -> (OperationId, AttemptSummary) {
        let operation = OperationId::new();
        self.schedule.mark_running(&root(name), operation);
        let attempt = PollAttempt {
            root: root(name),
            operation,
            completed_at: (answer == Answer::Completed).then(|| self.clock.utc()),
            answer,
            next_eligible_at: next,
            targets: Vec::new(),
            index_pending: false,
        };
        let summary = self
            .schedule
            .after(&self.now(), &attempt)
            .expect("a repository");
        (operation, summary)
    }
}

enum Want {
    Idle,
    /// `Idle`, and not before this many seconds whatever the stored deadline.
    Hold(u64),
    /// `Waiting` for this many seconds, resending the attempt's operation or
    /// a new one.
    Waiting(Why, u64, bool),
    /// `Waiting{NotDue}` until the answer's deadline.
    NotDue,
    Preempted,
    Attention(Reason),
    Stopped,
}

struct Row {
    name: &'static str,
    answer: Answer,
    /// The deadline the answer carries, in seconds from now.
    next: Option<i64>,
    want: Want,
    /// The stored deadline afterwards, in seconds from now. `None`: the
    /// one the repository had (100 s ago).
    deadline: Option<i64>,
    stale: bool,
    policy: fn(&Policy) -> bool,
}

fn row(name: &'static str, answer: Answer, next: Option<i64>, want: Want) -> Row {
    Row {
        name,
        answer,
        next,
        want,
        deadline: None,
        stale: false,
        policy: |_| true,
    }
}

impl Row {
    fn deadline(mut self, seconds: i64) -> Self {
        self.deadline = Some(seconds);
        self
    }

    fn stale(mut self) -> Self {
        self.stale = true;
        self
    }

    fn policy(mut self, check: fn(&Policy) -> bool) -> Self {
        self.policy = check;
        self
    }
}

fn transport(failure: TransportFailure) -> Answer {
    Answer::Transport(failure)
}

/// The rows that replace `last`: an attempt that reached the remote and said
/// how it went.
const RECORDED: [&str; 14] = [
    "completed",
    "completed, no deadline named",
    "unreachable, backoff deadline named",
    "unreachable, stored deadline to read",
    "key locked",
    "key locked, no key named",
    "key problem",
    "no key selected",
    "host unapproved",
    "host unapproved, no host named",
    "configuration required",
    "key, host or endpoint changed",
    "registry unavailable or runtime uninitialized",
    "interrupted",
];

#[test]
fn after_attempt_table() {
    let key = SharedKeyId::new();
    let host = Authority {
        host: "example.test".to_owned(),
        port: 22,
    };
    let repo = root("r");
    let cancelled = |cause| Answer::Cancelled(cause);
    let eligible = |reason| Answer::NotEligible(reason);
    let rows = [
        row("completed", Answer::Completed, Some(300), Want::Idle).deadline(300),
        row(
            "completed, no deadline named",
            Answer::Completed,
            None,
            Want::Idle,
        )
        .stale(),
        row(
            "unreachable, backoff deadline named",
            transport(TransportFailure::Unreachable),
            Some(60),
            Want::Idle,
        )
        .deadline(60),
        row(
            "unreachable, stored deadline to read",
            transport(TransportFailure::Unreachable),
            None,
            Want::Idle,
        )
        .stale(),
        row(
            "key locked",
            transport(TransportFailure::KeyLocked { key: Some(key) }),
            None,
            Want::Attention(Reason::KeyLocked(key)),
        ),
        row(
            "key locked, no key named",
            transport(TransportFailure::KeyLocked { key: None }),
            None,
            Want::Attention(Reason::NoKeySelected(repo.clone())),
        ),
        row(
            "key problem",
            transport(TransportFailure::KeyProblem { key: Some(key) }),
            None,
            Want::Attention(Reason::KeyProblem(key)),
        ),
        row(
            "no key selected",
            transport(TransportFailure::KeyProblem { key: None }),
            None,
            Want::Attention(Reason::NoKeySelected(repo.clone())),
        ),
        row(
            "host unapproved",
            transport(TransportFailure::HostUnapproved {
                authority: Some(host.clone()),
            }),
            None,
            Want::Attention(Reason::HostUnapproved(host)),
        ),
        row(
            "host unapproved, no host named",
            transport(TransportFailure::HostUnapproved { authority: None }),
            None,
            Want::Attention(Reason::HostUnidentified(repo.clone())),
        ),
        row(
            "configuration required",
            transport(TransportFailure::ConfigurationRequired),
            None,
            Want::Attention(Reason::ConfigurationRequired(repo.clone())),
        ),
        row(
            "key, host or endpoint changed",
            transport(TransportFailure::Changed),
            None,
            Want::Idle,
        )
        .stale(),
        row(
            "registry unavailable or runtime uninitialized",
            transport(TransportFailure::Unavailable),
            None,
            Want::Hold(300),
        ),
        row(
            "cancelled by preemption",
            cancelled(CancelCause::Preemption),
            None,
            Want::Preempted,
        ),
        row(
            "cancelled by quit",
            cancelled(CancelCause::Quit),
            None,
            Want::Stopped,
        ),
        row(
            "cancelled by reconfiguration",
            cancelled(CancelCause::Reconfiguration),
            None,
            Want::Idle,
        )
        .stale(),
        row(
            "cancelled by a takeover",
            cancelled(CancelCause::Takeover),
            None,
            Want::Idle,
        )
        .stale(),
        row(
            "interrupted",
            Answer::Interrupted,
            None,
            Want::Waiting(Why::Interrupted, 5, true),
        ),
        row(
            "busy",
            Answer::Busy,
            None,
            Want::Waiting(Why::Busy, 5, true),
        ),
        row(
            "repository, recovery required",
            Answer::Repository {
                recovery_required: true,
            },
            Some(-50),
            Want::Waiting(Why::Unfinished, 5, false),
        ),
        row(
            "repository, any other code",
            Answer::Repository {
                recovery_required: false,
            },
            Some(-50),
            Want::Waiting(Why::Failed, 5, false),
        ),
        row(
            "conflict pending",
            Answer::ConflictPending,
            None,
            Want::Hold(300),
        ),
        row(
            "not eligible, paused",
            eligible(NotEligible::Paused),
            Some(30),
            Want::Idle,
        )
        .deadline(30)
        .stale()
        .policy(|policy| policy.paused),
        row(
            "not eligible, disabled",
            eligible(NotEligible::Disabled),
            Some(30),
            Want::Idle,
        )
        .deadline(30)
        .stale()
        .policy(|policy| !policy.enabled),
        row(
            "not eligible, recovery suspended",
            eligible(NotEligible::RecoverySuspended),
            None,
            Want::Attention(Reason::RecoveryHeld(repo.clone())),
        )
        .stale()
        .policy(|policy| policy.recovery_suspended),
        row(
            "not eligible, not due",
            eligible(NotEligible::NotDue),
            Some(120),
            Want::NotDue,
        )
        .deadline(120),
        // The library's clock and ours differ: a deadline that has passed
        // for us, yet "not due" for it. No floor would poll again at once.
        row(
            "not eligible, not due, deadline already past",
            eligible(NotEligible::NotDue),
            Some(-10),
            Want::Hold(5),
        )
        .stale(),
        row(
            "not eligible, not due, no deadline",
            eligible(NotEligible::NotDue),
            None,
            Want::Idle,
        )
        .stale(),
    ];

    for row in rows {
        let mut fx = Fx::new();
        fx.add("r", policy(Some(fx.at(-100))));
        // An earlier completed attempt, so a row that must not touch `last`
        // can be told from one that replaces it.
        fx.answer("r", Answer::Completed, Some(fx.at(-100)));
        let now = fx.now();
        let next = row.next.map(|seconds| fx.at(seconds));
        let (operation, _) = fx.answer("r", row.answer.clone(), next);
        let repo = fx.repo("r");

        let (phase, hold) = match &row.want {
            Want::Idle => (Phase::Idle, None),
            Want::Hold(seconds) => (Phase::Idle, Some(now.after(secs(*seconds)))),
            Want::Waiting(why, seconds, same) => (
                Phase::Waiting {
                    until: now.after(secs(*seconds)),
                    why: *why,
                    op: same.then_some(operation),
                },
                None,
            ),
            Want::NotDue => (
                Phase::Waiting {
                    until: now.mono_at(next.expect("a deadline")),
                    why: Why::NotDue,
                    op: None,
                },
                None,
            ),
            Want::Preempted => (Phase::Preempted, None),
            Want::Attention(reason) => (Phase::Attention(reason.clone()), None),
            Want::Stopped => (Phase::Stopped, None),
        };
        let name = row.name;
        assert_eq!(repo.phase, phase, "{name}: phase");
        assert_eq!(repo.hold_until, hold, "{name}: hold");
        assert_eq!(repo.stale, row.stale, "{name}: stale");
        let deadline = fx.at(row.deadline.unwrap_or(-100));
        assert_eq!(
            repo.policy.next_eligible_at,
            Some(deadline),
            "{name}: deadline"
        );
        assert!((row.policy)(&repo.policy), "{name}: policy");
        let last = repo.last.as_ref().map(|last| &last.answer);
        let want = if RECORDED.contains(&name) {
            &row.answer
        } else {
            &Answer::Completed
        };
        assert_eq!(last, Some(want), "{name}: last");
        let preempted = u8::from(matches!(row.want, Want::Preempted));
        assert_eq!(repo.preempted, preempted, "{name}: preempted");
        let streak = u8::from(matches!(row.want, Want::Waiting(..)));
        assert_eq!(repo.streak, streak, "{name}: streak");
    }
}
