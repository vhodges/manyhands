//! The checks that brought something in, kept for the session (Wave 04 Cycle
//! 03b design A5; P8, U-5).

use std::time::Duration;

use manyhands::{canonical::ItemId, repository::OperationId};

use super::*;

fn target(disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: Some(ItemId::generate()),
        kind: Some(TargetKind::Ticket),
        disposition,
        main: false,
    }
}

/// An attempt that answered `answer` with one target per disposition.
fn attempt(fx: &Fx, name: &str, answer: Answer, seen: &[Disposition]) -> PollAttempt {
    PollAttempt {
        root: root(name),
        operation: OperationId::new(),
        completed_at: (answer == Answer::Completed).then(|| fx.clock.utc()),
        answer,
        next_eligible_at: None,
        targets: seen.iter().map(|&seen| target(seen)).collect(),
        index_pending: false,
    }
}

fn brought(seq: u64, name: &str, count: u32, at: OffsetDateTime, automatic: bool) -> Brought {
    Brought {
        seq,
        root: root(name),
        count,
        at,
        automatic,
    }
}

/// An automatic check and a Check now that brought changes in are each kept,
/// with the time they were applied; ones that brought nothing in, and a
/// failed one, add nothing; the list is never pruned, and a Check now that
/// lands while a poll is in flight (U-5) is kept too.
#[test]
fn brought_in_is_kept_for_the_session() {
    use Disposition::*;
    let mut fx = Fx::new();
    fx.add("a", policy(None));
    fx.add("b", policy(None));
    assert!(fx.schedule.brought().is_empty());

    // An automatic check that fast-forwarded one item and materialized one.
    let first = fx.at(0);
    let poll = attempt(
        &fx,
        "a",
        Answer::Completed,
        &[FastForwarded, Materialized, Current],
    );
    fx.schedule.mark_queued(&root("a"));
    fx.schedule.after(&fx.now(), &poll).expect("a repository");
    assert_eq!(fx.schedule.brought(), [brought(0, "a", 2, first, true)]);

    // Quiet checks, automatic and Check now, and a failed one: nothing.
    fx.clock.advance(Duration::from_secs(10));
    let quiet = attempt(&fx, "a", Answer::Completed, &[Current, LocalAhead]);
    fx.schedule.after(&fx.now(), &quiet).expect("a repository");
    fx.schedule
        .checked_now(&fx.now(), &quiet)
        .expect("a repository");
    let failed = attempt(
        &fx,
        "a",
        Answer::Transport(TransportFailure::Unreachable),
        &[],
    );
    fx.schedule.after(&fx.now(), &failed).expect("a repository");
    fx.schedule
        .checked_now(&fx.now(), &failed)
        .expect("a repository");
    assert_eq!(fx.schedule.brought().len(), 1);

    // A Check now that brought one change in, with no poll in flight.
    let second = fx.at(0);
    let now_check = attempt(&fx, "b", Answer::Completed, &[FastForwarded]);
    fx.schedule
        .checked_now(&fx.now(), &now_check)
        .expect("a repository");
    assert_eq!(
        fx.schedule.brought(),
        [
            brought(0, "a", 2, first, true),
            brought(1, "b", 1, second, false)
        ]
    );

    // A Check now answered while a poll of the repository is queued: the
    // in-flight branch returns its summary, so it is kept as well.
    fx.clock.advance(Duration::from_secs(10));
    let third = fx.at(0);
    fx.schedule.mark_queued(&root("a"));
    let in_flight = attempt(
        &fx,
        "a",
        Answer::Completed,
        &[Materialized, Materialized, Materialized],
    );
    fx.schedule
        .checked_now(&fx.now(), &in_flight)
        .expect("a repository");
    assert_eq!(fx.schedule.brought().len(), 3);
    assert_eq!(fx.schedule.brought()[2], brought(2, "a", 3, third, false));

    // An unscheduled repository answers nothing, so nothing is kept.
    let missing = attempt(&fx, "missing", Answer::Completed, &[FastForwarded]);
    assert!(fx.schedule.after(&fx.now(), &missing).is_none());
    assert!(fx.schedule.checked_now(&fx.now(), &missing).is_none());
    assert_eq!(fx.schedule.brought().len(), 3);

    // Removing the repository, or checking again, prunes nothing (K-4), and
    // the sequence numbers keep counting.
    fx.schedule.remove(&root("a"));
    let again = attempt(&fx, "b", Answer::Completed, &[FastForwarded]);
    fx.schedule.after(&fx.now(), &again).expect("a repository");
    let seqs: Vec<u64> = fx.schedule.brought().iter().map(|kept| kept.seq).collect();
    assert_eq!(seqs, [0, 1, 2, 3]);
    assert_eq!(fx.schedule.brought()[0], brought(0, "a", 2, first, true));
}

/// Each repository counts the completed attempts applied to it, automatic or
/// Check now (in flight or not), and no other answer (N22 clause 4).
#[test]
fn completed_attempts_are_counted_per_root() {
    use Disposition::*;
    let mut fx = Fx::new();
    fx.add("a", policy(None));
    fx.add("b", policy(None));
    let count = |fx: &Fx, name: &str| fx.schedule.completed_attempts(&root(name));
    assert_eq!(
        (count(&fx, "a"), count(&fx, "b"), count(&fx, "x")),
        (0, 0, 0)
    );

    let done = attempt(&fx, "a", Answer::Completed, &[Current]);
    fx.schedule.after(&fx.now(), &done).expect("a repository");
    assert_eq!(count(&fx, "a"), 1, "an automatic completed attempt");

    let failed = attempt(
        &fx,
        "a",
        Answer::Transport(TransportFailure::Unreachable),
        &[],
    );
    fx.schedule.after(&fx.now(), &failed).expect("a repository");
    fx.schedule
        .checked_now(&fx.now(), &failed)
        .expect("a repository");
    assert_eq!(count(&fx, "a"), 1, "a failed attempt is not counted");

    fx.schedule
        .checked_now(&fx.now(), &done)
        .expect("a repository");
    assert_eq!(count(&fx, "a"), 2, "a completed Check now");

    fx.schedule.mark_queued(&root("a"));
    fx.schedule
        .checked_now(&fx.now(), &done)
        .expect("a repository");
    assert_eq!(
        count(&fx, "a"),
        3,
        "a completed Check now while a poll is in flight"
    );

    let missing = attempt(&fx, "x", Answer::Completed, &[Current]);
    assert!(fx.schedule.after(&fx.now(), &missing).is_none());
    assert_eq!(
        (count(&fx, "b"), count(&fx, "x")),
        (0, 0),
        "no other repository moves"
    );
}
