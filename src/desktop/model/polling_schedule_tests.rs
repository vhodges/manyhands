//! The summary counts, R-4's restart schedule and R-34.

use std::time::Duration;

use manyhands::{canonical::ItemId, repository::OperationId};

use super::*;

#[test]
fn attempt_summary_counts() {
    let target = |disposition| AttemptTarget {
        item_id: Some(ItemId::generate()),
        kind: Some(match disposition {
            Disposition::Promoted => TargetKind::Document,
            _ => TargetKind::Ticket,
        }),
        disposition,
        main: false,
    };
    use Disposition::*;
    let every = [
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
    ];
    // (answer, brought in, up to date, to share, problems, listed, to
    // retire): the fixture lists every disposition once, so three are to
    // share (`LocalAhead`, `Unpublished`, `Diverged`) and ten need a look.
    let cases = [
        (Answer::Completed, 2, 2, 3, 10, 12, 2),
        (Answer::Interrupted, 2, 2, 3, 10, 14, 0),
        (Answer::Cancelled(CancelCause::Quit), 2, 2, 3, 10, 14, 0),
    ];
    for (answer, brought_in, up_to_date, to_share, problems, listed, retire) in cases {
        let mut fx = Fx::new();
        fx.add("r", policy(None));
        let attempt = PollAttempt {
            root: root("r"),
            operation: OperationId::new(),
            answer: answer.clone(),
            completed_at: Some(fx.at(0)),
            next_eligible_at: Some(fx.at(300)),
            targets: every
                .iter()
                .map(|&disposition| target(disposition))
                .collect(),
            index_pending: true,
        };
        let summary = fx
            .schedule
            .after(&fx.now(), &attempt)
            .expect("a repository");
        let counts = (
            summary.brought_in,
            summary.up_to_date,
            summary.to_share.len() as u32,
            summary.problems.len() as u32,
            summary.listed.len() as u32,
            summary.retire.len() as u32,
        );
        assert_eq!(
            counts,
            (brought_in, up_to_date, to_share, problems, listed, retire),
            "{answer:?}"
        );
        assert!(
            summary.changed(),
            "{answer:?}: brought in marks the repository changed"
        );
        assert!(summary.index_pending);
        assert!(fx.repo("r").last.is_some(), "{answer:?}: recorded");
        let completed = (answer == Answer::Completed).then(|| fx.at(0));
        assert_eq!(
            fx.repo("r").last_completed,
            completed,
            "{answer:?}: only a completion counts"
        );
    }

    // A cancel is an attempt only when it brought something in or listed
    // something; a cancel that saw only up-to-date items is not.
    let cancels = [
        (vec![], false),
        (vec![Current, LocalAhead], false),
        (vec![Current, FastForwarded], true),
        (vec![Dirty], true),
    ];
    for (dispositions, recorded) in cancels {
        let mut fx = Fx::new();
        fx.add("r", policy(None));
        let attempt = PollAttempt {
            root: root("r"),
            operation: OperationId::new(),
            answer: Answer::Cancelled(CancelCause::Preemption),
            completed_at: None,
            next_eligible_at: None,
            targets: dispositions.iter().map(|&d| target(d)).collect(),
            index_pending: false,
        };
        let summary = fx
            .schedule
            .after(&fx.now(), &attempt)
            .expect("a repository");
        assert_eq!(fx.repo("r").last.is_some(), recorded, "{dispositions:?}");
        assert_eq!(summary.is_recorded(), recorded, "{dispositions:?}");
    }

    // An item the answer names no ID or kind for cannot be retired: listed.
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    let nameless = AttemptTarget {
        item_id: None,
        kind: None,
        disposition: Closed,
        main: false,
    };
    let attempt = PollAttempt {
        root: root("r"),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: None,
        next_eligible_at: None,
        targets: vec![nameless],
        index_pending: false,
    };
    let summary = fx
        .schedule
        .after(&fx.now(), &attempt)
        .expect("a repository");
    assert_eq!((summary.listed.len(), summary.retire.len()), (1, 0));
    assert!(!summary.changed());
}

#[test]
fn restart_schedule() {
    let mut fx = Fx::new();
    // The stored deadline is long past: R-35 means it never governs here.
    fx.add("r", policy(Some(fx.at(-1000))));
    let waits = |fx: &Fx| match fx.phase("r") {
        Phase::Waiting { until, why, op } => (until, why, op),
        other => panic!("not waiting: {other:?}"),
    };

    // Busy, an interruption and a repository error share one schedule:
    // 5 s, 15 s, 60 s, resending the same operation, then the interval.
    let steps = [
        (Answer::Busy, Why::Busy, 5, true),
        (Answer::Interrupted, Why::Interrupted, 15, true),
        (Answer::Busy, Why::Busy, 60, true),
        (Answer::Busy, Why::Busy, 300, false),
        (Answer::Interrupted, Why::Interrupted, 5, true),
    ];
    for (answer, why, delay, same) in steps {
        let now = fx.now();
        let (operation, _) = fx.answer("r", answer, None);
        let (until, got_why, op) = waits(&fx);
        assert_eq!(
            (until, got_why, op),
            (now.after(secs(delay)), why, same.then_some(operation))
        );
        assert_eq!(
            fx.schedule.restart_of(&root("r")),
            same.then_some(operation)
        );

        // Not due a moment early, due on the dot, whatever the stored deadline.
        fx.clock.advance(secs(delay) - Duration::from_millis(1));
        assert!(fx.due().is_empty(), "{delay} s: early");
        fx.clock.advance(Duration::from_millis(1));
        assert_eq!(fx.due(), vec![root("r")], "{delay} s: on time");
    }

    // A repository error is on the same schedule (R-35) and its place in it
    // carries on from the steps before.
    fx.answer("r", Answer::Completed, Some(fx.at(300)));
    assert_eq!(
        fx.repo("r").streak,
        0,
        "a completion starts the schedule again"
    );
    let delays = [(true, 5), (false, 15), (true, 60), (false, 300)];
    for (recovery_required, delay) in delays {
        let now = fx.now();
        fx.answer("r", Answer::Repository { recovery_required }, None);
        let (until, _, op) = waits(&fx);
        assert_eq!((until, op), (now.after(secs(delay)), None), "{delay} s");
        fx.clock.advance(secs(delay));
    }

    // Anything that is not on the schedule ends it.
    fx.answer("r", Answer::Busy, None);
    assert_eq!(fx.repo("r").streak, 1);
    fx.answer("r", Answer::ConflictPending, None);
    assert_eq!(fx.repo("r").streak, 0);
}

#[test]
fn preempted_three_times() {
    let mut fx = Fx::new();
    fx.add("r", policy(Some(fx.at(-10))));
    let preempt = |fx: &mut Fx| fx.answer("r", Answer::Cancelled(CancelCause::Preemption), None);

    // Preempted once or twice: no time, no failure backoff, polled again.
    for count in 1..=2 {
        preempt(&mut fx);
        assert_eq!(
            (fx.phase("r"), fx.repo("r").preempted),
            (Phase::Preempted, count)
        );
        assert!(fx.due().is_empty(), "preempted waits for the user lane");
        fx.schedule.user_lane_empty(&fx.now());
        assert_eq!(fx.due(), vec![root("r")]);
    }
    // Any other answer in between starts the count again.
    fx.answer("r", Answer::Busy, None);
    assert_eq!(fx.repo("r").preempted, 0);

    // The third time in a row, it waits for the interval.
    for _ in 0..3 {
        preempt(&mut fx);
    }
    let now = fx.now();
    fx.schedule.user_lane_empty(&now);
    assert_eq!(fx.phase("r"), Phase::Idle);
    assert!(fx.due().is_empty());
    assert_eq!(fx.schedule.next_wake(&now), Some(now.after(secs(300))));
    fx.clock.advance(secs(300));
    assert_eq!(fx.due(), vec![root("r")]);
    assert_eq!(fx.repo("r").preempted, 0);
}

/// Review minor 1: a repository removed while its poll runs still holds the
/// one-at-a-time slot until that poll ends. An `after` that is not a poll's
/// end (a failed Check now's) does not free it; `poll_ended` does.
#[test]
fn a_removed_repositorys_poll_holds_the_slot_until_it_ends() {
    let mut fx = Fx::new();
    fx.add("a", policy(Some(fx.at(-10))));
    fx.add("b", policy(Some(fx.at(-10))));
    fx.schedule.mark_queued(&root("a"));
    assert!(fx.due().is_empty(), "one at a time");
    fx.schedule.remove(&root("a"));
    assert!(fx.due().is_empty(), "its poll still runs");

    // A Check now's failure is applied through `after`: not a poll's end.
    let attempt = PollAttempt {
        root: root("a"),
        operation: OperationId::new(),
        completed_at: None,
        answer: Answer::Busy,
        next_eligible_at: None,
        targets: Vec::new(),
        index_pending: false,
    };
    assert!(fx.schedule.after(&fx.now(), &attempt).is_none());
    assert!(fx.due().is_empty(), "still held");

    fx.schedule.poll_ended(&root("a"));
    assert_eq!(fx.due(), vec![root("b")]);
}

/// One step of a row of the unshared list's table: an attempt applied the
/// way the scheduler applies it, or a finished share or save heard at a
/// time, in the order the scheduler hears them.
enum Step {
    /// A poll's answer through `after`: a completed check, an older one
    /// applied late, a failed, interrupted or cancelled one, a replay.
    Poll(Answer, Option<i64>, Vec<(&'static str, Disposition)>),
    /// A Check now's answer through `checked_now`, with no poll of the
    /// repository in flight.
    CheckNow(i64, Vec<(&'static str, Disposition)>),
    /// The same with a poll of the repository queued or running (fact 64).
    CheckNowInFlight(i64, Vec<(&'static str, Disposition)>),
    /// A finished share, heard at the time: the item, whether main went too,
    /// and whether something left the list.
    Shared(Option<&'static str>, bool, i64, bool),
    /// A finished save, heard at the time: the item and whether it is new.
    Saved(&'static str, bool, i64),
    /// A close or approval that merged and did not publish, heard at the time.
    MainAhead(i64),
}

/// What the list holds after a step: its items, its problems (by name and
/// disposition) and `as_of`, in seconds from the fixture's start.
struct Want {
    items: Vec<(&'static str, Disposition)>,
    problems: Vec<(&'static str, Disposition)>,
    as_of: Option<i64>,
}

fn want(
    items: &[(&'static str, Disposition)],
    problems: &[(&'static str, Disposition)],
    as_of: Option<i64>,
) -> Want {
    Want {
        items: items.to_vec(),
        problems: problems.to_vec(),
        as_of,
    }
}

/// A completed check at `at`, applied when the scheduler gets to it.
fn check(at: i64, targets: &[(&'static str, Disposition)]) -> Step {
    Step::Poll(Answer::Completed, Some(at), targets.to_vec())
}

/// The design's table U1 to U15 (Part A, rules 1 to 5), one row each, and the
/// cases the table implies. Each row is a fresh repository and a sequence of
/// steps; the list and its `as_of` are asserted after every step. Times are
/// seconds from the fixture's start: an attempt's `completed_at` and the time
/// a share or save was heard. This is Task 1's design checkpoint: the table
/// is written from the design, not from the code, and is not to be edited to
/// fit it.
#[test]
fn unshared_follows_attempts_shares_and_saves() {
    use Disposition::*;
    const X: &str = "X";
    const Y: &str = "Y";
    const MAIN: &str = "main";
    const P: &str = "P";
    const Z: &str = "Z";

    // (the row, its steps with what the list holds after each, and the line
    // Settings says at the end when the row pins it)
    #[allow(clippy::type_complexity)]
    let rows: Vec<(&str, Vec<(Step, Want)>, Option<&str>)> = vec![
        (
            "U1: a check lists X",
            vec![(
                check(10, &[(X, LocalAhead), (P, Dirty)]),
                want(&[(X, LocalAhead)], &[(P, Dirty)], Some(10)),
            )],
            None,
        ),
        (
            "U2: then X is shared",
            vec![
                (
                    check(10, &[(X, LocalAhead)]),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                (
                    Step::Shared(Some(X), false, 20, true),
                    want(&[], &[], Some(10)),
                ),
            ],
            None,
        ),
        (
            "U3: then X is saved",
            vec![
                (
                    check(10, &[(X, LocalAhead)]),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                (
                    Step::Shared(Some(X), false, 20, true),
                    want(&[], &[], Some(10)),
                ),
                (
                    Step::Saved(X, false, 30),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
            ],
            None,
        ),
        (
            "U4: a check that completed before the save lists X as Current",
            vec![
                (check(10, &[(X, Current)]), want(&[], &[], Some(10))),
                (
                    Step::Saved(X, false, 30),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                // Applied late: it completed at 20, the save was heard at 30.
                (
                    check(20, &[(X, Current)]),
                    want(&[(X, LocalAhead)], &[], Some(20)),
                ),
            ],
            None,
        ),
        (
            "U5: a check that completed after the share lists X as LocalAhead",
            vec![
                (
                    check(10, &[(X, LocalAhead)]),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                (
                    Step::Shared(Some(X), false, 20, true),
                    want(&[], &[], Some(10)),
                ),
                (
                    check(30, &[(X, LocalAhead)]),
                    want(&[(X, LocalAhead)], &[], Some(30)),
                ),
            ],
            None,
        ),
        (
            "U6: X saved, then shared, both heard before an older check is applied",
            vec![
                (check(10, &[(X, Current)]), want(&[], &[], Some(10))),
                (
                    Step::Saved(X, false, 20),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                (
                    Step::Shared(Some(X), false, 25, true),
                    want(&[], &[], Some(10)),
                ),
                // The older check (15) saw X ahead; the share (25) is last.
                (check(15, &[(X, LocalAhead)]), want(&[], &[], Some(15))),
            ],
            None,
        ),
        (
            "U7: X shared, then saved, both heard before an older check is applied",
            vec![
                (
                    check(10, &[(X, LocalAhead)]),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                (
                    Step::Shared(Some(X), false, 20, true),
                    want(&[], &[], Some(10)),
                ),
                (
                    Step::Saved(X, false, 25),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                // The older check (15) saw X current; the save (25) is last.
                (
                    check(15, &[(X, Current)]),
                    want(&[(X, LocalAhead)], &[], Some(15)),
                ),
            ],
            None,
        ),
        (
            "U8: a failed, interrupted or cancelled check leaves the list",
            {
                let kept = || want(&[(X, LocalAhead)], &[(P, Dirty)], Some(10));
                vec![
                    (check(10, &[(X, LocalAhead), (P, Dirty)]), kept()),
                    (
                        Step::Poll(
                            Answer::Transport(TransportFailure::Unreachable),
                            Some(20),
                            vec![],
                        ),
                        kept(),
                    ),
                    (
                        Step::Poll(Answer::Interrupted, Some(30), vec![(Y, LocalAhead)]),
                        kept(),
                    ),
                    (
                        Step::Poll(
                            Answer::Cancelled(CancelCause::Preemption),
                            Some(40),
                            vec![(Z, Dirty)],
                        ),
                        kept(),
                    ),
                ]
            },
            None,
        ),
        (
            "U9: a replayed Completed with no targets leaves the list",
            vec![
                (
                    check(10, &[(X, LocalAhead), (P, Dirty)]),
                    want(&[(X, LocalAhead)], &[(P, Dirty)], Some(10)),
                ),
                (
                    check(20, &[]),
                    want(&[(X, LocalAhead)], &[(P, Dirty)], Some(10)),
                ),
            ],
            None,
        ),
        (
            "U10: an attempt older than as_of is ignored",
            vec![
                (
                    check(20, &[(X, LocalAhead), (P, Dirty)]),
                    want(&[(X, LocalAhead)], &[(P, Dirty)], Some(20)),
                ),
                (
                    check(10, &[(Y, LocalAhead)]),
                    want(&[(X, LocalAhead)], &[(P, Dirty)], Some(20)),
                ),
            ],
            None,
        ),
        (
            "U11: a close or approval that published takes the item and main",
            vec![
                (
                    check(10, &[(X, LocalAhead), (MAIN, LocalAhead)]),
                    want(&[(X, LocalAhead), (MAIN, LocalAhead)], &[], Some(10)),
                ),
                (
                    Step::Shared(Some(X), true, 20, true),
                    want(&[], &[], Some(10)),
                ),
            ],
            None,
        ),
        (
            "U12: a close or approval that merged without publishing adds main",
            vec![
                (
                    check(10, &[(X, Current), (MAIN, Current)]),
                    want(&[], &[], Some(10)),
                ),
                (
                    Step::MainAhead(20),
                    want(&[(MAIN, LocalAhead)], &[], Some(10)),
                ),
            ],
            None,
        ),
        (
            "U13: a Check now answered before an older automatic answer is applied",
            vec![
                // The Check now (20) is handled when heard; the older
                // automatic answer (10) waited for its status read.
                (
                    Step::CheckNow(20, vec![(Y, LocalAhead)]),
                    want(&[(Y, LocalAhead)], &[], Some(20)),
                ),
                (
                    check(10, &[(X, LocalAhead), (P, Dirty)]),
                    want(&[(Y, LocalAhead)], &[], Some(20)),
                ),
            ],
            None,
        ),
        (
            "U14: a save before the first completed check",
            vec![(
                Step::Saved(X, false, 5),
                want(&[(X, LocalAhead)], &[], None),
            )],
            Some("Not checked yet \u{b7} 1 not shared yet"),
        ),
        (
            "U15: a Check now completed while a poll is in flight",
            vec![
                (
                    Step::CheckNowInFlight(10, vec![(X, LocalAhead), (P, Dirty)]),
                    want(&[(X, LocalAhead)], &[(P, Dirty)], Some(10)),
                ),
                (
                    Step::Shared(Some(X), false, 20, true),
                    want(&[], &[(P, Dirty)], Some(10)),
                ),
                // Rule 3: completed at 15, the share was heard at 20.
                (
                    Step::CheckNowInFlight(15, vec![(X, LocalAhead), (P, Dirty)]),
                    want(&[], &[(P, Dirty)], Some(15)),
                ),
                // Rule 4: older than as_of.
                (
                    Step::CheckNowInFlight(5, vec![(Z, LocalAhead)]),
                    want(&[], &[(P, Dirty)], Some(15)),
                ),
            ],
            None,
        ),
        (
            "a Check now with no poll in flight follows rules 3 and 4 too",
            vec![
                (
                    Step::CheckNow(10, vec![(X, LocalAhead)]),
                    want(&[(X, LocalAhead)], &[], Some(10)),
                ),
                (
                    Step::Shared(Some(X), false, 20, true),
                    want(&[], &[], Some(10)),
                ),
                (
                    Step::CheckNow(15, vec![(X, LocalAhead)]),
                    want(&[], &[], Some(15)),
                ),
                (
                    Step::CheckNow(5, vec![(Z, LocalAhead)]),
                    want(&[], &[], Some(15)),
                ),
            ],
            None,
        ),
        (
            "a save of an item already listed counts it once",
            vec![
                (
                    check(10, &[(X, Diverged)]),
                    want(&[(X, Diverged)], &[], Some(10)),
                ),
                (
                    Step::Saved(X, false, 20),
                    want(&[(X, Diverged)], &[], Some(10)),
                ),
                (
                    Step::Saved(X, true, 21),
                    want(&[(X, Diverged)], &[], Some(10)),
                ),
            ],
            None,
        ),
        (
            "a save of a new item is Unpublished, and a share that went on another item leaves it",
            vec![
                (
                    Step::Saved(Y, true, 5),
                    want(&[(Y, Unpublished)], &[], None),
                ),
                (
                    Step::Shared(Some(X), false, 6, false),
                    want(&[(Y, Unpublished)], &[], None),
                ),
            ],
            Some("Not checked yet \u{b7} 1 not shared yet"),
        ),
        (
            "the first completed check sets the list whole, a save heard before it replayed",
            vec![
                (
                    Step::Saved(X, false, 5),
                    want(&[(X, LocalAhead)], &[], None),
                ),
                (
                    Step::Saved(Y, false, 15),
                    want(&[(X, LocalAhead), (Y, LocalAhead)], &[], None),
                ),
                // Completed at 10: the save of X (5) is before it, so the
                // check decides X; the save of Y (15) is after it.
                (
                    check(10, &[(MAIN, Current)]),
                    want(&[(Y, LocalAhead)], &[], Some(10)),
                ),
            ],
            None,
        ),
        (
            "a share of an item no list holds is heard, and a check that saw it ahead earlier loses",
            vec![
                (
                    Step::Shared(Some(X), false, 20, false),
                    want(&[], &[], None),
                ),
                (check(10, &[(X, LocalAhead)]), want(&[], &[], Some(10))),
            ],
            None,
        ),
    ];

    for (label, steps, settings) in rows {
        let mut fx = Fx::new();
        fx.add("r", policy(None));
        let ids: std::cell::RefCell<Vec<(&str, ItemId)>> = Default::default();
        let id_of = |name: &'static str| -> ItemId {
            let mut ids = ids.borrow_mut();
            if let Some((_, id)) = ids.iter().find(|(held, _)| *held == name) {
                return id.clone();
            }
            let id = ItemId::generate();
            ids.push((name, id.clone()));
            id
        };
        let target = |(name, disposition): (&'static str, Disposition)| {
            if name == MAIN {
                AttemptTarget {
                    item_id: None,
                    kind: None,
                    disposition,
                    main: true,
                }
            } else {
                AttemptTarget {
                    item_id: Some(id_of(name)),
                    kind: Some(TargetKind::Ticket),
                    disposition,
                    main: false,
                }
            }
        };
        for (number, (step, expected)) in steps.into_iter().enumerate() {
            let step_label = format!("{label}, step {}", number + 1);
            let attempt = |fx: &Fx,
                           answer: Answer,
                           at: Option<i64>,
                           targets: Vec<AttemptTarget>| PollAttempt {
                root: root("r"),
                operation: OperationId::new(),
                answer,
                completed_at: at.map(|at| fx.at(at)),
                next_eligible_at: Some(fx.at(300)),
                targets,
                index_pending: false,
            };
            match step {
                Step::Poll(answer, at, targets) => {
                    let targets = targets.into_iter().map(target).collect();
                    let attempt = attempt(&fx, answer, at, targets);
                    fx.schedule.after(&fx.now(), &attempt);
                }
                Step::CheckNow(at, targets) => {
                    let targets = targets.into_iter().map(target).collect();
                    let attempt = attempt(&fx, Answer::Completed, Some(at), targets);
                    fx.schedule.checked_now(&fx.now(), &attempt);
                }
                Step::CheckNowInFlight(at, targets) => {
                    fx.schedule.mark_queued(&root("r"));
                    assert!(fx.repo("r").in_flight(), "{step_label}");
                    let targets = targets.into_iter().map(target).collect();
                    let attempt = attempt(&fx, Answer::Completed, Some(at), targets);
                    fx.schedule.checked_now(&fx.now(), &attempt);
                }
                Step::Shared(item, main, at, went) => {
                    let item = item.map(id_of);
                    let gone = fx
                        .schedule
                        .shared(&root("r"), item.as_ref(), main, fx.at(at));
                    assert_eq!(gone, went, "{step_label}: something left");
                }
                Step::Saved(item, new, at) => {
                    let kind = Some(TargetKind::Ticket);
                    fx.schedule
                        .saved(&root("r"), id_of(item), kind, new, fx.at(at));
                }
                Step::MainAhead(at) => {
                    assert!(
                        fx.schedule.main_ahead(&root("r"), fx.at(at)),
                        "{step_label}"
                    );
                }
            }
            let list = &fx.repo("r").unshared;
            let items: Vec<_> = expected.items.iter().map(|&row| target(row)).collect();
            let problems: Vec<_> = expected.problems.iter().map(|&row| target(row)).collect();
            assert_eq!(list.items, items, "{step_label}: the items");
            assert_eq!(list.problems, problems, "{step_label}: the problems");
            assert_eq!(
                list.as_of,
                expected.as_of.map(|at| fx.at(at)),
                "{step_label}: as_of"
            );
        }
        if let Some(line) = settings {
            let repo = fx.repo("r");
            let said = status::row(&status::Facts {
                name: "r",
                schedule: Some(repo),
                unplaced: None,
                holding: None,
                checking_now: false,
                stored: None,
                now: fx.now().utc,
            });
            assert_eq!(said.status, line, "{label}: Settings");
        }
    }

    // A save, a share and a main-ahead on a root with no schedule do
    // nothing, and are not kept: the root is scheduled afterwards and a
    // check that completed before them finds nothing of them.
    let mut fx = Fx::new();
    let item = ItemId::generate();
    let kind = Some(TargetKind::Ticket);
    assert!(
        !fx.schedule
            .saved(&root("none"), item.clone(), kind, false, fx.at(20))
    );
    assert!(
        !fx.schedule
            .shared(&root("none"), Some(&item), true, fx.at(21))
    );
    assert!(!fx.schedule.main_ahead(&root("none"), fx.at(22)));
    assert!(
        fx.schedule.repo(&root("none")).is_none(),
        "no schedule is made"
    );
    fx.add("none", policy(None));
    let attempt = PollAttempt {
        root: root("none"),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(fx.at(10)),
        next_eligible_at: Some(fx.at(300)),
        targets: vec![AttemptTarget {
            item_id: Some(item.clone()),
            kind,
            disposition: LocalAhead,
            main: false,
        }],
        index_pending: false,
    };
    fx.schedule.after(&fx.now(), &attempt);
    let list = &fx.repo("none").unshared;
    assert_eq!(
        list.items.len(),
        1,
        "nothing was logged for the unscheduled root"
    );
    assert_eq!(list.items[0].item_id, Some(item));
}

/// An attempt's counts of what is to share and what needs a look, and a
/// finished share's effect on the unshared list and on the last attempt's
/// listed items (P4, P5, P19).
#[test]
fn to_share_counts_and_finished_shares() {
    use Disposition::*;
    let (first, document, second, dirty) = (
        ItemId::generate(),
        ItemId::generate(),
        ItemId::generate(),
        ItemId::generate(),
    );
    let item = |id: &ItemId, kind, disposition| AttemptTarget {
        item_id: Some(id.clone()),
        kind: Some(kind),
        disposition,
        main: false,
    };
    let main = |disposition| AttemptTarget {
        item_id: None,
        kind: None,
        disposition,
        main: true,
    };
    // An item whose ID did not parse is not main, and is not found by an ID.
    let nameless = AttemptTarget {
        item_id: None,
        kind: Some(TargetKind::Ticket),
        disposition: LocalAhead,
        main: false,
    };
    let completed = |fx: &Fx, targets: Vec<AttemptTarget>| PollAttempt {
        root: root("r"),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(fx.at(0)),
        next_eligible_at: Some(fx.at(300)),
        targets,
        index_pending: false,
    };
    let listed = |fx: &Fx| {
        fx.repo("r")
            .last
            .as_ref()
            .expect("an attempt")
            .listed
            .clone()
    };
    let held = |fx: &Fx| fx.repo("r").unshared.items.len();

    let mut fx = Fx::new();
    fx.add("r", policy(None));
    let attempt = completed(
        &fx,
        vec![
            item(&first, TargetKind::Ticket, LocalAhead),
            item(&document, TargetKind::Document, Unpublished),
            item(&second, TargetKind::Ticket, Diverged),
            item(&dirty, TargetKind::Ticket, Dirty),
            main(LocalAhead),
            nameless.clone(),
            item(&ItemId::generate(), TargetKind::Ticket, Current),
        ],
    );
    let summary = fx
        .schedule
        .after(&fx.now(), &attempt)
        .expect("a repository");
    assert_eq!(
        (summary.to_share.len(), summary.problems.len()),
        (5, 1),
        "main counts as one item, and a nameless one is still to share"
    );
    assert_eq!(held(&fx), 5);
    assert_eq!(fx.repo("r").unshared.problems.len(), 1);
    // `listed` keeps what it listed before: the unpublished and diverged
    // items and the one needing a look, not the one only ahead.
    assert_eq!(listed(&fx).len(), 3);

    let heard = fx.at(10);
    // A share of the first ticket, which is only ahead.
    assert!(fx.schedule.shared(&root("r"), Some(&first), false, heard));
    assert_eq!(held(&fx), 4);
    assert!(
        !fx.repo("r")
            .unshared
            .items
            .iter()
            .any(|target| target.item_id.as_ref() == Some(&first))
    );
    // Again, and an ID nothing holds: nothing goes.
    assert!(!fx.schedule.shared(&root("r"), Some(&first), false, heard));
    assert!(
        !fx.schedule
            .shared(&root("r"), Some(&ItemId::generate()), false, heard)
    );
    assert_eq!(held(&fx), 4);

    // The diverged ticket and main: both go, from the list and from `listed`.
    assert!(fx.schedule.shared(&root("r"), Some(&second), true, heard));
    assert_eq!(held(&fx), 2, "the document and the nameless item are left");
    let rest = listed(&fx);
    assert!(
        !rest
            .iter()
            .any(|target| target.item_id.as_ref() == Some(&second)),
        "{rest:?}"
    );
    assert_eq!(rest.len(), 2, "the document and the one needing a look");
    // A share by ID never takes the nameless item, and main only goes by `main`.
    assert!(!fx.schedule.shared(&root("r"), None, false, heard));
    assert_eq!(held(&fx), 2);

    // Main, when a check listed it unpublished or diverged, leaves `listed`.
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    let attempt = completed(
        &fx,
        vec![main(Diverged), item(&first, TargetKind::Ticket, Dirty)],
    );
    fx.schedule.after(&fx.now(), &attempt);
    assert_eq!((held(&fx), listed(&fx).len()), (1, 2));
    assert!(fx.schedule.shared(&root("r"), None, true, fx.at(10)));
    assert_eq!((held(&fx), listed(&fx).len()), (0, 1));
    assert!(!listed(&fx).iter().any(|target| target.main));

    // A share heard after a check completed, applied when that check lands
    // later, takes the item off the list and off the attempt's listed items.
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    assert!(
        !fx.schedule
            .shared(&root("r"), Some(&second), false, fx.at(10))
    );
    let attempt = completed(
        &fx,
        vec![
            item(&second, TargetKind::Ticket, Diverged),
            item(&document, TargetKind::Document, Unpublished),
        ],
    );
    let attempt = PollAttempt {
        completed_at: Some(fx.at(5)),
        ..attempt
    };
    fx.schedule.after(&fx.now(), &attempt);
    assert_eq!((held(&fx), listed(&fx).len()), (1, 1));
    assert_eq!(fx.repo("r").unshared.items[0].item_id, Some(document));

    // A root with no schedule: nothing to remove.
    assert!(
        !fx.schedule
            .shared(&root("none"), Some(&first), true, fx.at(10))
    );
}
