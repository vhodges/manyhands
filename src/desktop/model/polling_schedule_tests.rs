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
    // (answer, brought in, up to date, not shared, listed, to retire)
    let cases = [
        (Answer::Completed, 2, 2, 1, 12, 2),
        (Answer::Interrupted, 2, 2, 1, 14, 0),
        (Answer::Cancelled(CancelCause::Quit), 2, 2, 1, 14, 0),
    ];
    for (answer, brought_in, up_to_date, not_shared, listed, retire) in cases {
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
            summary.not_shared,
            summary.listed.len() as u32,
            summary.retire.len() as u32,
        );
        assert_eq!(
            counts,
            (brought_in, up_to_date, not_shared, listed, retire),
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
