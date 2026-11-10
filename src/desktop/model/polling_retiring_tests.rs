//! The schedule's side of the retirement loop (D6 Task 11b): where a loop
//! can begin (R-32, RT-11) and what a Check now's completed answer does.

use manyhands::canonical::ItemId;

use super::*;

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA1";

/// A completed attempt that reported one closed ticket.
fn closed(fx: &Fx, name: &str) -> PollAttempt {
    PollAttempt {
        root: root(name),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(fx.clock.utc()),
        next_eligible_at: Some(fx.at(300)),
        targets: vec![AttemptTarget {
            item_id: Some(ITEM.parse::<ItemId>().unwrap()),
            kind: Some(TargetKind::Ticket),
            disposition: Disposition::Closed,
        }],
        index_pending: false,
    }
}

/// A loop begins from a repository that waits on a schedule or a preempted
/// poll as well as from an idle one, and a running loop is begun again by a
/// Check now (RT-11). Not from one that is in flight, held for attention or
/// stopped by a quit.
#[test]
fn a_loop_begins_where_a_check_now_can_replace_it() {
    let attention = Phase::Attention(Reason::ConfigurationRequired(root("r")));
    let waiting = Phase::Waiting {
        until: Fx::new().now().mono,
        why: Why::Failed,
        op: None,
    };
    let rows = [
        ("idle", Phase::Idle, true),
        ("preempted", Phase::Preempted, true),
        ("waiting", waiting, true),
        ("retiring", Phase::Retiring, true),
        ("queued", Phase::Queued, false),
        ("attention", attention, false),
        ("stopped", Phase::Stopped, false),
    ];
    for (what, phase, begins) in rows {
        let mut fx = Fx::new();
        fx.add("r", policy(None));
        fx.schedule.repos.get_mut(&root("r")).unwrap().phase = phase.clone();
        assert_eq!(fx.schedule.begin_retiring(&root("r")), begins, "{what}");
        let want = if begins { Phase::Retiring } else { phase };
        assert_eq!(fx.phase("r"), want, "{what}");
    }
    let mut fx = Fx::new();
    assert!(!fx.schedule.begin_retiring(&root("missing")));
    fx.add("r", policy(None));
    fx.schedule.mark_running(&root("r"), OperationId::new());
    assert!(!fx.schedule.begin_retiring(&root("r")), "running");
}

/// A Check now's completed answer is the repository's latest attempt, in
/// report order (C1, E1), and settles the phase as a poll's does, whether
/// the repository was idle, preempted, waiting or running a loop (RT-11).
/// While a poll of it is queued or running, the attempt is recorded and the
/// phase is left to the poll's own answer.
#[test]
fn a_check_now_is_the_latest_attempt() {
    for from in [Phase::Idle, Phase::Preempted, Phase::Retiring] {
        let mut fx = Fx::new();
        fx.add("r", policy(Some(fx.at(-5))));
        fx.schedule.repos.get_mut(&root("r")).unwrap().phase = from.clone();
        let attempt = closed(&fx, "r");
        let summary = fx
            .schedule
            .checked_now(&fx.now(), &attempt)
            .expect("a scheduled repository");
        assert_eq!(summary.retire, attempt.targets, "{from:?}: report order");
        assert_eq!(fx.phase("r"), Phase::Idle, "{from:?}: settled");
        let last = fx.repo("r").last.as_ref().expect("the latest attempt");
        assert_eq!(last.retire, attempt.targets, "{from:?}: recorded");
        assert_eq!(
            fx.repo("r").last_completed,
            Some(fx.clock.utc()),
            "{from:?}: completed now"
        );
        assert_eq!(
            fx.repo("r").policy.next_eligible_at,
            Some(fx.at(300)),
            "{from:?}: the deadline it names"
        );
    }

    // A poll of it is in flight: recorded, phase untouched.
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    fx.schedule.mark_queued(&root("r"));
    let attempt = closed(&fx, "r");
    assert!(fx.schedule.checked_now(&fx.now(), &attempt).is_some());
    assert_eq!(fx.phase("r"), Phase::Queued);
    assert!(fx.repo("r").last.is_some());

    // A repository the schedule does not hold has no attempt.
    assert!(
        fx.schedule
            .checked_now(&fx.now(), &closed(&fx, "x"))
            .is_none()
    );
}
