//! The rows of `stale_rows_table` (D6 Task 4, L-1 to L-6), over the world of
//! the parent module.

use super::*;

/// Each row builds a launch, may run polls, and asserts L-1 to L-6.
fn row(name: &str, run: impl FnOnce()) {
    eprintln!("{name}");
    run();
}

/// L-1 to L-6 (design B3), with a seeded in-flight record and canned library
/// answers (A23).
pub(super) fn stale_rows() {
    // L-1: the entry's operation is the next poll, sent with restart=true
    // when the repository is due at launch; a completion this launch makes
    // counts, ends the entry, and the next poll is new.
    row("L-1 at launch", || {
        let held = OperationId::new();
        let mut world = World::launch(Some(held));
        assert!(world.due());
        assert_eq!(world.holding(), None);
        world
            .backend
            .answer_poll(report(PollCompletion::Completed, world.launched_at));
        let attempt = world.poll();
        assert_eq!(world.backend.requests(), [(held, true)]);
        let summary = world.apply(&attempt, ActiveRow::Nothing);
        assert!(summary.is_some(), "it counts");
        assert!(world.schedule.repo(&world.root).unwrap().last.is_some());
        assert_eq!(world.entry(), None, "a terminal answer ends it");
        let next = world.plan();
        assert!(!next.restart, "the next poll is new");
        assert_ne!(next.operation, held);
    });

    // L-1 later in the session: not due at launch, taken over when due.
    row("L-1 later in the session", || {
        let held = OperationId::new();
        let mut world = World::launch(Some(held));
        let later = world.clock.utc() + time::Duration::seconds(600);
        let mut policy = open_policy();
        policy.next_eligible_at = Some(later);
        world.schedule.set_policy(&world.root, policy);
        assert!(!world.due());
        assert_eq!(world.holding(), Some(Holding::NotDue));
        assert_eq!(world.entry(), Some(held), "the entry stays");
        world.clock.advance(secs(600));
        assert!(world.due());
        assert_eq!(world.holding(), None);
        world
            .backend
            .answer_poll(report(PollCompletion::Completed, world.clock.utc()));
        world.poll();
        assert_eq!(world.backend.requests(), [(held, true)]);
    });

    // L-1: a replayed terminal answer older than this launch counts as no
    // attempt: no status line, no loop, and the entry is gone.
    row("L-1 replay older than the launch", || {
        let held = OperationId::new();
        let mut world = World::launch(Some(held));
        let before = world.launched_at - time::Duration::hours(1);
        world
            .backend
            .answer_poll(report(PollCompletion::Completed, before));
        let attempt = world.poll();
        assert_eq!(attempt.answer, Answer::Completed);
        assert_eq!(world.apply(&attempt, ActiveRow::Nothing), None);
        let repo = world.schedule.repo(&world.root).unwrap();
        assert!(repo.last.is_none(), "no status line");
        assert!(repo.last_completed.is_none());
        assert_eq!(repo.phase, Phase::Idle);
        assert!(repo.stale, "the policy is read again");
        assert_eq!(world.entry(), None);
    });

    // L-2: not eligible, no takeover; the row says why; the entry stays;
    // turning checking on takes it over at runtime.
    for (cause, expect) in [
        ("off", Holding::Off),
        ("paused", Holding::Paused),
        ("held", Holding::Held),
        ("not due", Holding::NotDue),
    ] {
        row(&format!("L-2 {cause}"), || {
            let held = OperationId::new();
            let mut world = World::launch(Some(held));
            let mut policy = open_policy();
            match expect {
                Holding::Off => policy.enabled = false,
                Holding::Paused => policy.paused = true,
                Holding::Held => policy.recovery_suspended = true,
                _ => {
                    policy.next_eligible_at =
                        Some(world.clock.utc() + time::Duration::seconds(120));
                }
            }
            world.schedule.set_policy(&world.root, policy);
            assert_eq!(world.holding(), Some(expect));
            assert!(!world.due(), "no takeover while not eligible");
            assert_eq!(world.entry(), Some(held));
            assert_eq!(world.backend.requests(), []);

            // Checking is turned on (or the deadline arrives): L-1 applies.
            world.schedule.set_policy(&world.root, open_policy());
            assert_eq!(world.holding(), None);
            assert!(world.due(), "{cause}");
            world
                .backend
                .answer_poll(report(PollCompletion::Completed, world.clock.utc()));
            world.poll();
            assert_eq!(world.backend.requests(), [(held, true)], "{cause}");
        });
    }
    // L-2 learned from the library: the poll answers NotEligible while the
    // row is still the entry's. The entry stays; the row says why.
    row("L-2 the library says paused", || {
        let held = OperationId::new();
        let mut world = World::launch(Some(held));
        world.backend.answer_poll(Library::NotEligible {
            reason: manyhands::repository::PollIneligibility::Paused,
            next_eligible_at: None,
            index_pending: false,
        });
        let attempt = world.poll();
        assert_eq!(attempt.answer, Answer::NotEligible(NotEligible::Paused));
        world.apply(&attempt, ActiveRow::Operation(held));
        assert_eq!(world.entry(), Some(held));
        assert_eq!(world.holding(), Some(Holding::Paused));
    });

    // L-3: not ours any more, judged by the active operation, not the code.
    row("L-3 operation mismatch", || {
        let held = OperationId::new();
        let mut world = World::launch(Some(held));
        world.backend.answer_poll(Library::Mismatch);
        let attempt = world.poll();
        assert_eq!(attempt.answer, Answer::OperationMismatch);
        world.apply(&attempt, ActiveRow::Operation(held));
        assert_eq!(world.entry(), None, "dropped though the row looks ours");
        world.clock.advance(secs(5));
        assert!(world.due());
        let next = world.plan();
        assert!(!next.restart && next.operation != held, "a new operation");
    });
    row("L-3 fenced row", || {
        let held = OperationId::new();
        let mut world = World::launch(Some(held));
        world.backend.answer_poll(Library::Repository {
            recovery_required: true,
        });
        let attempt = world.poll();
        world.apply(&attempt, ActiveRow::Operation(OperationId::new()));
        assert_eq!(world.entry(), None, "another operation is active: dropped");
        world.clock.advance(secs(5));
        let next = world.plan();
        assert!(!next.restart && next.operation != held, "a new operation");
    });
    row("L-3 pending local work is not a fence", || {
        let held = OperationId::new();
        let mut world = World::launch(Some(held));
        world.backend.answer_poll(Library::Repository {
            recovery_required: true,
        });
        let attempt = world.poll();
        world.apply(&attempt, ActiveRow::Operation(held));
        assert_eq!(world.entry(), Some(held), "the row is still ours");
        assert_eq!(
            world.phase(),
            Phase::Waiting {
                until: world.clock.mono() + secs(5),
                why: Why::Unfinished,
                op: None
            }
        );
    });

    // L-5: Busy from an operation the record does not hold waits on R-4's
    // schedule and never takes the row over.
    row("L-5 busy from another operation", || {
        let mut world = World::launch(None);
        let other = OperationId::new();
        world.backend.fail_poll(RemotePollError::Busy);
        let attempt = world.poll();
        world.apply(&attempt, ActiveRow::Operation(other));
        assert_eq!(world.entry(), None, "the other row is not in the record");
        assert!(world.store.polls().is_empty());
        world.clock.advance(secs(5));
        let next = world.plan();
        assert_eq!(next.operation, attempt.operation, "our own, resent");
        assert_ne!(next.operation, other);
    });

    // L-6: a second instance takes nothing over, writes nothing and runs no
    // retirement loop.
    row("L-6 second instance", || {
        let held = OperationId::new();
        let fixture = Fixture::new();
        let root = fixture.create_repository("a").to_str().unwrap().to_owned();
        let data = tempfile::tempdir().unwrap();
        let mut first = InflightStore::open(data.path());
        assert_eq!(first.try_acquire(), Acquired::Now);
        first.poll_submitting(&root, held);
        let record = data.path().join("inflight").join("record.json");
        let before = fs::read(&record).unwrap();

        let mut second = InflightStore::open(data.path());
        assert_eq!(second.try_acquire(), Acquired::Elsewhere);
        let mut world = World::over(fixture, data, second, root);
        assert!(world.due());
        assert_eq!(world.holding(), None);
        assert!(!world.store.runs_retirement_loop());
        world
            .backend
            .answer_poll(report(PollCompletion::Completed, world.clock.utc()));
        let attempt = world.poll();
        let sent = world.backend.requests();
        assert_eq!(sent.len(), 1);
        assert_ne!(sent[0].0, held, "it never takes the row over");
        assert!(!sent[0].1);
        world.apply(&attempt, ActiveRow::Nothing);
        assert_eq!(fs::read(&record).unwrap(), before, "it wrote nothing");

        // The first instance goes; the lock is tried again on activation,
        // and L-1 runs for the entry it left.
        drop(first);
        assert_eq!(world.store.try_acquire(), Acquired::Now);
        assert_eq!(
            world.plan(),
            Planned {
                operation: held,
                restart: true
            }
        );
        assert!(world.store.runs_retirement_loop());
    });
}
