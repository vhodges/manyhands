//! The polling rows of `counts_what_awaits_the_person`: one for each entry
//! the schedule lists, however many repositories it holds back (decision
//! 11, R-14), and none for what waits for itself or is the person's own
//! choice.

use std::time::Duration;

use manyhands::repository::OperationId;

use crate::desktop::model::polling::{
    Answer, Authority, CancelCause, Clock, KeyId, NotEligible, Policy, PollAttempt, PollSchedule,
    Root, TestClock, TransportFailure,
};

/// Enabled and unpaused, as the library reports a repository nobody has
/// touched.
fn policy() -> Policy {
    Policy {
        enabled: true,
        paused: false,
        interval: Duration::from_secs(300),
        backoff: None,
        recovery_suspended: false,
        next_eligible_at: None,
        active_operation: None,
    }
}

fn root(name: &str) -> Root {
    format!("/{name}")
}

/// A schedule with one repository per row (`/a`, `/b`, ...), each having
/// answered a poll as given.
pub(super) fn polled(answers: &[(&str, Answer)]) -> PollSchedule {
    let clock = TestClock::new();
    let mut schedule = PollSchedule::default();
    for (name, answer) in answers {
        let (root, operation) = (root(name), OperationId::new());
        schedule.add(root.clone(), policy(), None);
        schedule.mark_running(&root, operation);
        let attempt = PollAttempt {
            root,
            operation,
            completed_at: None,
            answer: answer.clone(),
            next_eligible_at: None,
            targets: Vec::new(),
            index_pending: false,
        };
        schedule
            .after(&clock.now(), &attempt)
            .expect("a repository");
    }
    schedule
}

/// One repository that has not answered a poll yet, under `policy`.
fn registered(policy: Policy) -> PollSchedule {
    let mut schedule = PollSchedule::default();
    schedule.add(root("a"), policy, None);
    schedule
}

pub(super) fn locked(key: KeyId) -> Answer {
    Answer::Transport(TransportFailure::KeyLocked { key: Some(key) })
}

pub(super) fn host(authority: Option<&str>) -> Answer {
    Answer::Transport(TransportFailure::HostUnapproved {
        authority: authority.map(|host| Authority {
            host: host.to_owned(),
            port: 22,
        }),
    })
}

pub(super) fn unconfigured() -> Answer {
    Answer::Transport(TransportFailure::ConfigurationRequired)
}

fn problem(key: Option<KeyId>) -> Answer {
    Answer::Transport(TransportFailure::KeyProblem { key })
}

fn held() -> Answer {
    Answer::NotEligible(NotEligible::RecoverySuspended)
}

/// Each row: what it is, the count it adds, and the schedule.
pub(super) fn rows() -> Vec<(&'static str, usize, PollSchedule)> {
    let (one, two) = (KeyId::new(), KeyId::new());
    let mut rows = Vec::new();
    for (what, expected, answers) in [
        (
            "a locked key held back three repositories",
            1,
            vec![("a", locked(one)), ("b", locked(one)), ("c", locked(one))],
        ),
        (
            "two locked keys, one each",
            2,
            vec![("a", locked(one)), ("b", locked(two))],
        ),
        (
            "a key problem held back two repositories",
            1,
            vec![("a", problem(Some(one))), ("b", problem(Some(one)))],
        ),
        (
            "no key selected, one per repository",
            2,
            vec![
                ("a", problem(None)),
                (
                    "b",
                    Answer::Transport(TransportFailure::KeyLocked { key: None }),
                ),
            ],
        ),
        (
            "an unapproved host held back two repositories",
            1,
            vec![("a", host(Some("x"))), ("b", host(Some("x")))],
        ),
        (
            "a host answer naming no host, one per repository",
            2,
            vec![("a", host(None)), ("b", host(None))],
        ),
        (
            "configuration required, one per repository",
            2,
            vec![("a", unconfigured()), ("b", unconfigured())],
        ),
        (
            "held until recovery, one per repository",
            2,
            vec![("a", held()), ("b", held())],
        ),
        (
            "every kind once",
            7,
            vec![
                ("a", locked(one)),
                ("b", problem(Some(two))),
                ("c", problem(None)),
                ("d", host(Some("x"))),
                ("e", host(None)),
                ("f", unconfigured()),
                ("g", held()),
            ],
        ),
        // What waits for itself, or is the person's own choice, is not
        // counted. A Retire... offer is no term (Task 12).
        (
            "paused, off and not due answers",
            0,
            vec![
                ("a", Answer::NotEligible(NotEligible::Paused)),
                ("b", Answer::NotEligible(NotEligible::Disabled)),
                ("c", Answer::NotEligible(NotEligible::NotDue)),
            ],
        ),
        (
            "unreachable, waiting and quiet attempts",
            0,
            vec![
                ("a", Answer::Transport(TransportFailure::Unreachable)),
                ("b", Answer::Transport(TransportFailure::Changed)),
                ("c", Answer::Transport(TransportFailure::Unavailable)),
                ("d", Answer::Busy),
                ("e", Answer::Interrupted),
                (
                    "f",
                    Answer::Repository {
                        recovery_required: true,
                    },
                ),
                (
                    "g",
                    Answer::Repository {
                        recovery_required: false,
                    },
                ),
                ("h", Answer::ConflictPending),
                ("i", Answer::Cancelled(CancelCause::Preemption)),
                ("j", Answer::Cancelled(CancelCause::Quit)),
                ("k", Answer::Completed),
            ],
        ),
    ] {
        rows.push((what, expected, polled(&answers)));
    }
    rows.push(("nothing registered", 0, PollSchedule::default()));
    // States the person set, before any poll answered.
    let paused = Policy {
        paused: true,
        ..policy()
    };
    rows.push(("a paused policy", 0, registered(paused)));
    let off = Policy {
        enabled: false,
        ..policy()
    };
    rows.push(("a policy that is off", 0, registered(off)));
    let mut retiring = registered(policy());
    assert!(retiring.begin_retiring(&root("a")));
    rows.push(("a retirement loop running", 0, retiring));
    // A repository the library holds back until recovery counts once, as
    // the policy says so and again when the policy is read again (decision
    // 10).
    let suspended = Policy {
        recovery_suspended: true,
        ..policy()
    };
    let mut library_held = registered(suspended.clone());
    library_held.sync_recovery(&root("a"));
    library_held.set_policy(&root("a"), suspended);
    library_held.sync_recovery(&root("a"));
    rows.push(("a library-held repository", 1, library_held));
    rows
}
