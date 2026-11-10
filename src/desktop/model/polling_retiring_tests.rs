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

/// The attempt entry's retirement lines (E3, "what the person sees"): a
/// retirement that is sent says "Retiring…", one that retired says so, one
/// left pending says it did not finish; any other answer is attention, a
/// retry or a refusal, which have their own rows, so the line goes. The
/// next recorded attempt replaces them; an answer that is no attempt does
/// not.
#[test]
fn retirement_lines_follow_the_requests() {
    use super::retire::Verdict;
    use crate::desktop::model::polling::entry::{RetireLine, RetireState, attempt_entry};

    let other: ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FA2".parse().unwrap();
    let item: ItemId = ITEM.parse().unwrap();
    let lines = |fx: &Fx| -> Vec<(ItemId, RetireState)> {
        fx.schedule
            .attempts()
            .find(|(attempted, ..)| **attempted == root("r"))
            .map(|(_, _, lines)| {
                lines
                    .iter()
                    .map(|line| (line.item.clone(), line.state))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    let attempt = closed(&fx, "r");
    fx.schedule.checked_now(&fx.now(), &attempt);

    // Sent: Retiring…, in the order they began; sending again replaces.
    let r = root("r");
    fx.schedule.retire_started(&r, TargetKind::Ticket, &item);
    fx.schedule.retire_started(&r, TargetKind::Ticket, &other);
    fx.schedule.retire_started(&r, TargetKind::Ticket, &item);
    assert_eq!(
        lines(&fx),
        [
            (other.clone(), RetireState::Retiring),
            (item.clone(), RetireState::Retiring)
        ]
    );

    // Answered.
    let owed = |catching_up| Verdict::Owed {
        attention: None,
        catching_up,
    };
    let cases: Vec<(&str, Verdict, Option<RetireState>)> = vec![
        ("retired", Verdict::Retired, Some(RetireState::Retired)),
        (
            "pending",
            owed(false),
            Some(RetireState::Unfinished { catching_up: false }),
        ),
        (
            "pending, the index behind",
            owed(true),
            Some(RetireState::Unfinished { catching_up: true }),
        ),
        (
            "attention",
            Verdict::Attention(super::retire::Need::Host),
            None,
        ),
        ("unreachable", Verdict::Unreachable, None),
        (
            "a retry",
            Verdict::Retry {
                recovery_required: false,
            },
            None,
        ),
        ("a refusal", Verdict::Refused, None),
        ("no data", Verdict::Failed, None),
    ];
    for (what, verdict, line) in cases {
        fx.schedule.retire_started(&r, TargetKind::Ticket, &item);
        fx.schedule
            .retire_settled(&r, TargetKind::Ticket, &item, &verdict);
        let expected: Vec<_> = [Some((other.clone(), RetireState::Retiring))]
            .into_iter()
            .chain([line.map(|state| (item.clone(), state))])
            .flatten()
            .collect();
        assert_eq!(lines(&fx), expected, "{what}");
    }

    // The entry shows them though the attempt itself had nothing to say.
    fx.schedule
        .retire_settled(&r, TargetKind::Ticket, &item, &Verdict::Retired);
    let (_, summary, shown) = fx.schedule.attempts().next().unwrap();
    assert_eq!(
        attempt_entry(summary, &[]),
        None,
        "a retirement alone was nothing to say"
    );
    let entry = attempt_entry(summary, shown).expect("with its retirements");
    assert_eq!(
        entry.retirements,
        vec![
            RetireLine {
                kind: TargetKind::Ticket,
                item: other.clone(),
                state: RetireState::Retiring
            },
            RetireLine {
                kind: TargetKind::Ticket,
                item: item.clone(),
                state: RetireState::Retired
            },
        ]
    );

    // An answer that is no attempt leaves them; the next attempt replaces.
    fx.schedule.mark_queued(&r);
    let busy = PollAttempt {
        answer: Answer::Busy,
        ..closed(&fx, "r")
    };
    fx.schedule.after(&fx.now(), &busy);
    assert_eq!(lines(&fx).len(), 2, "busy is not an attempt");
    let attempt = closed(&fx, "r");
    fx.schedule.checked_now(&fx.now(), &attempt);
    assert!(lines(&fx).is_empty(), "the next attempt replaces them");

    // A repository that is not scheduled has none.
    fx.schedule
        .retire_started(&root("x"), TargetKind::Ticket, &item);
    assert!(fx.schedule.repo(&root("x")).is_none());
}

/// E4, A19: Retire… is offered for every reported `Closed` or `Promoted`
/// item that the loop did not retire (skipped, refused, preserved, failed
/// before any effect, not reached), in the poll's order, and for an item in
/// the refused set that the latest attempt no longer reports. Not for one
/// that is retiring, pending or retired. Choosing it removes the item from
/// the refused set; the attempt still reports it, so it stays offered.
#[test]
fn retire_offers_follow_the_attempt_and_the_refused_set() {
    use super::entry::RetireOffer;
    use super::retire::Verdict;

    let ids: Vec<ItemId> = (1..=7)
        .map(|n| format!("01ARZ3NDEKTSV4RRFFQ69G5FB{n}").parse().unwrap())
        .collect();
    let (retired, refused, skipped, pending, retiring, promoted, gone) = (
        &ids[0], &ids[1], &ids[2], &ids[3], &ids[4], &ids[5], &ids[6],
    );
    let mut fx = Fx::new();
    fx.add("r", policy(None));
    let r = root("r");
    let target = |item: &ItemId, kind, disposition| AttemptTarget {
        item_id: Some(item.clone()),
        kind: Some(kind),
        disposition,
    };
    let attempt = PollAttempt {
        targets: vec![
            target(retired, TargetKind::Ticket, Disposition::Closed),
            target(refused, TargetKind::Ticket, Disposition::Closed),
            target(skipped, TargetKind::Ticket, Disposition::Closed),
            target(pending, TargetKind::Ticket, Disposition::Closed),
            target(retiring, TargetKind::Ticket, Disposition::Closed),
            target(promoted, TargetKind::Document, Disposition::Promoted),
        ],
        ..closed(&fx, "r")
    };
    fx.schedule.checked_now(&fx.now(), &attempt);
    let owed = Verdict::Owed {
        attention: None,
        catching_up: false,
    };
    for item in [retired, pending, retiring] {
        fx.schedule.retire_started(&r, TargetKind::Ticket, item);
    }
    fx.schedule
        .retire_settled(&r, TargetKind::Ticket, retired, &Verdict::Retired);
    fx.schedule
        .retire_settled(&r, TargetKind::Ticket, pending, &owed);
    fx.schedule
        .retirement_answered(&r, TargetKind::Ticket, refused, &Verdict::Refused);
    // A refused item the latest attempt does not report any more.
    fx.schedule
        .retirement_answered(&r, TargetKind::Document, gone, &Verdict::Refused);
    // A retry is not the refused set.
    fx.schedule.retirement_answered(
        &r,
        TargetKind::Ticket,
        skipped,
        &Verdict::Retry {
            recovery_required: false,
        },
    );

    let offer = |kind, item: &ItemId| RetireOffer {
        kind,
        item: item.clone(),
    };
    let expected = vec![
        offer(TargetKind::Ticket, refused),
        offer(TargetKind::Ticket, skipped),
        offer(TargetKind::Document, promoted),
        offer(TargetKind::Document, gone),
    ];
    assert_eq!(fx.schedule.retire_offers(&r), expected);
    assert!(fx.schedule.retirement_refused(&r, refused));

    // Choosing Retire… forgets the refusal; the item is still reported, the
    // one the attempt no longer reports is not offered again.
    fx.schedule.retire_chosen(&r, refused);
    fx.schedule.retire_chosen(&r, gone);
    assert!(!fx.schedule.retirement_refused(&r, refused));
    assert_eq!(fx.schedule.retire_offers(&r), expected[..3]);

    // Another repository, and one the schedule does not hold, offer none.
    fx.add("s", policy(None));
    assert!(fx.schedule.retire_offers(&root("s")).is_empty());
    assert!(fx.schedule.retire_offers(&root("x")).is_empty());
}
