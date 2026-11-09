//! The schedule's decisions (A3): what is due, when to wake, a clock event,
//! reconfiguration, and attention.

use super::*;

#[test]
fn due_order_and_launch() {
    // At launch the stored deadline governs, whatever set it (decision 23,
    // A11): an interval and a backoff deadline both hold.
    struct Case {
        name: &'static str,
        deadline: Option<i64>,
        change: fn(&mut Policy),
        due_after: Option<u64>,
    }
    let case = |name, deadline, due_after| Case {
        name,
        deadline,
        change: |_| {},
        due_after,
    };
    let cases = [
        case("no stored deadline", None, Some(0)),
        case("a past deadline", Some(-5), Some(0)),
        case("an interval deadline ahead", Some(240), Some(240)),
        case("a backoff deadline ahead", Some(60), Some(60)),
        Case {
            change: |policy| policy.enabled = false,
            ..case("checking is off", None, None)
        },
        Case {
            change: |policy| policy.paused = true,
            ..case("paused", None, None)
        },
        Case {
            change: |policy| policy.recovery_suspended = true,
            ..case("held for recovery", None, None)
        },
    ];
    for case in cases {
        let mut fx = Fx::new();
        let mut policy = policy(case.deadline.map(|seconds| fx.at(seconds)));
        (case.change)(&mut policy);
        fx.add("r", policy);
        match case.due_after {
            None => {
                fx.clock.advance(secs(10_000));
                assert!(fx.due().is_empty(), "{}", case.name);
            }
            Some(0) => assert_eq!(fx.due(), vec![root("r")], "{}", case.name),
            Some(after) => {
                fx.clock.advance(secs(after - 1));
                assert!(fx.due().is_empty(), "{}: a second early", case.name);
                fx.clock.advance(secs(1));
                assert_eq!(fx.due(), vec![root("r")], "{}: on time", case.name);
            }
        }
    }

    // Earliest last completion first; never completed before all; one
    // repository at a time.
    let mut fx = Fx::new();
    let (old, recent) = (Some(fx.at(-3 * 3600)), Some(fx.at(-3600)));
    for (name, last) in [("a-recent", recent), ("c-never", None), ("b-old", old)] {
        fx.schedule.add(root(name), policy(None), last);
    }
    fx.add(
        "held",
        Policy {
            paused: true,
            ..policy(None)
        },
    );
    let order = |fx: &Fx| fx.due();
    assert_eq!(
        order(&fx),
        vec![root("c-never"), root("b-old"), root("a-recent")]
    );

    fx.schedule.mark_queued(&root("c-never"));
    assert!(fx.due().is_empty(), "one repository at a time");
    fx.answer("c-never", Answer::Completed, Some(fx.at(300)));
    assert_eq!(order(&fx), vec![root("b-old"), root("a-recent")]);

    // No poll of a repository whose retirement loop runs (R-32); none with
    // attention.
    assert!(fx.schedule.begin_retiring(&root("b-old")));
    assert_eq!(order(&fx), vec![root("a-recent")]);
    fx.answer(
        "a-recent",
        Answer::Transport(TransportFailure::ConfigurationRequired),
        None,
    );
    assert!(fx.due().is_empty());
    fx.schedule.end_retiring(&root("b-old"));
    assert_eq!(order(&fx), vec![root("b-old")]);
}

#[test]
fn next_wake_table() {
    type Setup = fn(&mut Fx);
    // Each case ends in the wake it expects, in seconds from now.
    let cases: [(&str, Setup, Option<u64>); 12] = [
        ("nothing", |_| {}, None),
        (
            "a deadline ahead",
            |fx| fx.add("a", policy(Some(fx.at(90)))),
            Some(90),
        ),
        ("no deadline: now", |fx| fx.add("a", policy(None)), Some(0)),
        (
            "the earliest of several",
            |fx| {
                fx.add("a", policy(Some(fx.at(90))));
                fx.add("b", policy(Some(fx.at(40))));
            },
            Some(40),
        ),
        (
            "a restart wait before a later deadline",
            |fx| {
                fx.add("a", policy(Some(fx.at(90))));
                fx.add("b", policy(Some(fx.at(-5))));
                fx.answer("b", Answer::Busy, None);
            },
            Some(5),
        ),
        (
            "a not-due wait is the library's deadline",
            |fx| {
                fx.add("a", policy(Some(fx.at(-5))));
                fx.answer(
                    "a",
                    Answer::NotEligible(NotEligible::NotDue),
                    Some(fx.at(120)),
                );
            },
            Some(120),
        ),
        (
            "a hold outlasts a past stored deadline",
            |fx| {
                fx.add("a", policy(Some(fx.at(-5))));
                fx.answer("a", Answer::ConflictPending, None);
            },
            Some(300),
        ),
        (
            "paused",
            |fx| {
                fx.add(
                    "a",
                    Policy {
                        paused: true,
                        ..policy(None)
                    },
                )
            },
            None,
        ),
        (
            "attention",
            |fx| {
                fx.add("a", policy(None));
                fx.answer(
                    "a",
                    Answer::Transport(TransportFailure::ConfigurationRequired),
                    None,
                );
            },
            None,
        ),
        (
            "a stale policy waits to be read",
            |fx| {
                fx.add("a", policy(None));
                fx.answer("a", Answer::Transport(TransportFailure::Changed), None);
            },
            None,
        ),
        (
            "preempted: no time",
            |fx| {
                fx.add("a", policy(None));
                fx.answer("a", Answer::Cancelled(CancelCause::Preemption), None);
            },
            None,
        ),
        (
            "a poll in flight",
            |fx| {
                fx.add("a", policy(Some(fx.at(10))));
                fx.add("b", policy(None));
                fx.schedule.mark_queued(&root("b"));
            },
            None,
        ),
    ];
    for (name, setup, wake) in cases {
        let mut fx = Fx::new();
        setup(&mut fx);
        let now = fx.now();
        let wake = wake.map(|seconds| now.after(secs(seconds)));
        assert_eq!(fx.schedule.next_wake(&now), wake, "{name}");
    }
}

#[test]
fn clock_event_once() {
    let mut fx = Fx::new();
    for (name, deadline) in [("a", -1), ("b", -1), ("c", 3600), ("d", -1)] {
        fx.add(name, policy(Some(fx.at(deadline))));
    }
    // a: waiting, not due for 10 minutes. b: a restart's first wait.
    fx.answer(
        "a",
        Answer::NotEligible(NotEligible::NotDue),
        Some(fx.at(600)),
    );
    let (operation, _) = fx.answer("b", Answer::Busy, None);
    let before = fx.phase("b");

    // Two clocks agreeing to within a minute are not an event.
    assert_eq!(
        fx.schedule
            .clock_event(time::Duration::seconds(60), secs(0)),
        ClockEvent::Steady
    );
    assert!(fx.schedule.stale().is_empty());

    // A suspend: UTC moved an hour and a half, the monotonic clock 10 s.
    let (utc, mono) = (time::Duration::seconds(5400), secs(10));
    fx.clock.skew(utc, mono);
    assert_eq!(fx.schedule.clock_event(utc, mono), ClockEvent::Jumped);
    assert_eq!(fx.phase("a"), Phase::Idle, "a not-due wait ends");
    assert_eq!(
        fx.phase("b"),
        before,
        "a restart keeps its place and its operation"
    );
    assert_eq!(fx.schedule.restart_of(&root("b")), Some(operation));
    assert_eq!(fx.repo("b").streak, 1);
    assert_eq!(fx.schedule.stale().len(), 4, "every policy is read again");
    assert!(fx.due().is_empty(), "nothing polls on an unread policy");

    // The library's answers, clamped: every deadline is past. Each
    // repository is polled once, one at a time, and then nothing, however
    // long the sleep was.
    for name in ["a", "b", "c", "d"] {
        fx.schedule
            .set_policy(&root(name), policy(Some(fx.at(-10))));
    }
    let mut polled = Vec::new();
    while let Some(next) = fx.due().first().cloned() {
        assert_eq!(fx.due().len(), 4 - polled.len());
        let name = next.trim_start_matches('/').to_owned();
        polled.push(next);
        fx.answer(&name, Answer::Completed, Some(fx.at(300)));
        assert!(polled.len() <= 4, "no catch-up loop");
    }
    assert_eq!(polled.len(), 4);
    assert!(fx.due().is_empty());
    fx.clock.advance(secs(299));
    assert!(
        fx.due().is_empty(),
        "the next deadline is a full interval away"
    );

    // A clock set back is an event too.
    let back = fx
        .schedule
        .clock_event(time::Duration::seconds(-3600), secs(1));
    assert_eq!(back, ClockEvent::Jumped);
}
