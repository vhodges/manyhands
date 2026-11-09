//! The scheduler entity (D6 Task 5): shared by clones, behind the polling
//! switch, the launch poll, the timer, and what a poll brings in. The clock
//! is injected; the double stands in for the transport, which no desktop
//! test reaches.

use std::time::Duration;

use gpui_kit::TestAppContext;
use manyhands::repository::OperationId;

use super::support::World;
use crate::desktop::{
    model::polling::{Answer, AttemptTarget, Clock, Disposition, Phase, PollAttempt, TargetKind},
    services::{
        ReadKind,
        requests::{Intent, Lane},
    },
    test_support::fixtures::{Fixture, item_id},
    view::harness::{Shell, key},
};

fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// A poll submitted through a `Services` clone is seen by the original's
/// scheduler: one entity, shared by every clone and window (A17).
#[gpui_kit::test]
async fn scheduler_shared_by_clones(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world.backend.answer_poll(world.completed());
    let clone = world.services.clone();
    assert!(clone.scheduler() == world.services.scheduler());

    cx.update(|cx| clone.repositories_listed(std::slice::from_ref(&root), cx));
    World::settle(cx, "the clone's launch poll to be applied", |cx| {
        world.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .repo(&root)
                .is_some_and(|repo| repo.last.is_some())
        })
    })
    .await;

    assert_eq!(world.backend.polled_roots(), std::slice::from_ref(&root));
    let last = world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler.schedule().repo(&root).unwrap().last.clone()
    });
    assert_eq!(last.unwrap().answer, Answer::Completed);
}

/// Polling is off unless switched on (R-27): `Services::start` alone makes
/// no call and starts no poll, however the list loads. Switched on, the
/// same list starts one. Separate from `launch_refresh`.
#[gpui_kit::test]
async fn polling_switch_off_in_tests(cx: &mut TestAppContext) {
    let world = World::new(false, cx);
    let root = world.ssh_repo("a");
    world.backend.answer_poll(world.completed());
    assert!(!world.services.polling());

    world.list(std::slice::from_ref(&root), cx);
    world.activate(cx);
    World::quiet(cx).await;
    assert!(
        world.backend.calls().is_empty(),
        "{:?}",
        world.backend.calls()
    );
    assert_eq!(world.backend.remote_reads(), 0);
    assert!(world.backend.polled_roots().is_empty());
    assert!(!world.scheduled(&root, cx));

    // The switch is shared by every clone.
    let switched = world.services.clone().with_polling();
    assert!(switched.polling() && world.services.polling());
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the poll to start", |_| {
        !world.backend.polled_roots().is_empty()
    })
    .await;
    assert_eq!(world.backend.polled_roots(), [root]);
}

/// After the list loads, each enabled, unpaused repository with an SSH
/// publication remote and a due deadline gets a launch poll, fairest first
/// and one at a time; one without a remote, or paused, is never polled; and
/// remotes are read again on window activation, so a remote added meanwhile
/// is seen (U-2, B6, A11).
#[gpui_kit::test]
async fn launch_polls_due_ssh_repos(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let first = world.ssh_repo("a1");
    let second = world.ssh_repo("a2");
    let plain = world.plain_repo("plain");
    let paused = world.ssh_repo("paused");
    world.fixture.pause_polling(std::path::Path::new(&paused));
    let roots = [first.clone(), second.clone(), plain.clone(), paused.clone()];

    // One at a time: the first stays in flight until it is released.
    let gate = world.backend.hold_poll();
    world.backend.answer_poll(world.completed());
    world.backend.answer_poll(world.completed());
    world.list(&roots, cx);
    World::settle(cx, "the first launch poll", |_| {
        !world.backend.polled_roots().is_empty()
    })
    .await;
    World::quiet(cx).await;
    assert_eq!(world.backend.polled_roots(), std::slice::from_ref(&first));
    drop(gate);
    World::settle(cx, "the second launch poll", |_| {
        world.backend.polled_roots().len() == 2
    })
    .await;
    World::quiet(cx).await;
    assert_eq!(world.backend.polled_roots(), [first, second]);
    // The repository without a remote has no schedule at all, and the
    // paused one is scheduled but not polled.
    assert!(!world.scheduled(&plain, cx));
    assert!(world.scheduled(&paused, cx));
    assert_eq!(world.backend.remote_reads(), 4);

    // A remote added behind the desktop's back is seen on activation.
    world.fixture.publish_over_ssh(std::path::Path::new(&plain));
    world.activate(cx);
    World::settle(cx, "the remote to be read again", |_| {
        world.backend.polled_roots().contains(&plain)
    })
    .await;
    assert_eq!(world.backend.remote_reads(), 8);
    assert!(world.scheduled(&plain, cx));
}

/// The timer wakes at the model's next wake, and submits one poll at a
/// time however many repositories are due (R-7).
#[gpui_kit::test]
async fn timer_submits_one_at_a_time(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let first = world.ssh_repo("a1");
    let second = world.ssh_repo("a2");
    let roots = [first, second];
    world.backend.answer_poll(world.completed());
    world.backend.answer_poll(world.completed());
    world.list(&roots, cx);
    World::settle(cx, "both launch polls", |_| {
        world.backend.polled_roots().len() == 2
    })
    .await;
    World::settle(cx, "the second answer", |cx| {
        roots
            .iter()
            .all(|root| matches!(world.phase(root, cx), Some(Phase::Idle)))
            && world
                .services
                .scheduler()
                .read_with(cx, |scheduler, _| scheduler.armed_for().is_some())
    })
    .await;

    // Both are stored for five minutes ahead; the timer is armed no
    // further than its longest sleep.
    let armed = world
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| scheduler.armed_for())
        .unwrap();
    let begun = world.clock.mono();
    assert!(armed > begun && armed <= begun + secs(30));

    // Time passes: both come due together. The wake submits one, which is
    // held in flight, and nothing more.
    let gate = world.backend.hold_poll();
    world.advance(secs(301));
    world.backend.answer_poll(world.completed());
    world.backend.answer_poll(world.completed());
    cx.executor().advance_clock(secs(30));
    World::settle(cx, "the timer's poll", |_| {
        world.backend.polled_roots().len() == 3
    })
    .await;
    World::quiet(cx).await;
    world.tick(cx);
    World::quiet(cx).await;
    assert_eq!(world.backend.polled_roots().len(), 3, "two polls at once");
    // Nor is the other one waiting in the worker's queue: the scheduler
    // has one repository in flight, whatever the worker would run.
    let in_flight = roots
        .iter()
        .filter(|root| {
            matches!(
                world.phase(root, cx),
                Some(Phase::Queued | Phase::Running { .. })
            )
        })
        .count();
    assert_eq!(in_flight, 1, "two polls submitted at once");

    drop(gate);
    World::settle(cx, "the next poll once the first answered", |_| {
        world.backend.polled_roots().len() == 4
    })
    .await;
    let polled = world.backend.polled_roots();
    assert_ne!(polled[2], polled[3], "fairness: the other one went next");
}

const ONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ0";
const TWO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ1";

/// What a poll brought in marks its repository changed through the new
/// `Operations` entry point, so every tab of it reads again (R-10, Fact
/// 17); only `FastForwarded` and `Materialized` do, whatever the poll's
/// end; an `IndexPending` answer queues one background `IndexRefresh`.
#[gpui_kit::test]
async fn result_marks_repository_changed(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, ONE, "One", "");
    fixture.commit_ticket(&root, TWO, "Two", "");
    fixture.refresh(&root);
    fixture.publish_over_ssh(&root);
    // Paused: scheduled, but its launch poll is not what is under test.
    fixture.pause_polling(&root);
    let shell = Shell::open(fixture, cx);
    let clock = std::sync::Arc::new(crate::desktop::model::polling::TestClock::new());
    let services = shell.workspace.read_with(cx, |w, _| w.services().clone());
    let services = cx.update(|cx| services.with_polling().with_clock(clock.clone(), cx));
    let key_root = key(&root);
    cx.update(|cx| services.repositories_listed(std::slice::from_ref(&key_root), cx));
    World::settle(cx, "the repository to be scheduled", |cx| {
        services.scheduler().read_with(cx, |scheduler, _| {
            scheduler.schedule().repo(&key_root).is_some()
        })
    })
    .await;

    for id in [ONE, TWO] {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.open_item(
                crate::desktop::model::navigation::OpenTarget::Item {
                    repo: key(&root),
                    kind: manyhands::repository::ItemDtoKind::Ticket,
                    id: item_id(id),
                },
                cx,
            )
        });
        shell.run(cx);
    }
    let item_reads = || shell.backend.reads_seen(ReadKind::Item);
    let answer = |targets: Vec<AttemptTarget>, index_pending: bool| PollAttempt {
        root: key_root.clone(),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(clock.utc()),
        next_eligible_at: Some(clock.utc() + time::Duration::seconds(300)),
        targets,
        index_pending,
    };
    let target = |disposition| AttemptTarget {
        item_id: Some(item_id(ONE)),
        kind: Some(TargetKind::Ticket),
        disposition,
    };
    let hear = |attempt: PollAttempt, cx: &mut TestAppContext| {
        services.polls().push(key_root.clone(), attempt);
        services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.heard(Vec::new(), cx));
    };
    let index_refreshes = |cx: &mut TestAppContext| {
        services.operations().read_with(cx, |operations, _| {
            operations
                .requests()
                .iter()
                .filter(|request| {
                    matches!(request.intent, Intent::IndexRefresh { .. })
                        && request.lane == Lane::Background
                })
                .count()
        })
    };

    // Up to date and listed items bring nothing in: nothing reads again.
    let before = item_reads();
    hear(
        answer(
            vec![
                target(Disposition::Current),
                target(Disposition::LocalAhead),
                target(Disposition::Diverged),
            ],
            false,
        ),
        cx,
    );
    World::quiet(cx).await;
    shell.run(cx);
    assert_eq!(item_reads(), before);
    assert_eq!(index_refreshes(cx), 0);

    // A fast-forward and a materialization: both tabs read again, through
    // the new entry point alone, and no index refresh is queued.
    hear(
        answer(
            vec![
                target(Disposition::FastForwarded),
                target(Disposition::Materialized),
            ],
            false,
        ),
        cx,
    );
    World::settle(cx, "every tab of the repository to read again", |cx| {
        shell.run(cx);
        item_reads() >= before + 2
    })
    .await;
    assert_eq!(index_refreshes(cx), 0);

    // A completed poll whose index refresh did not: one background
    // `IndexRefresh`.
    hear(answer(Vec::new(), true), cx);
    World::quiet(cx).await;
    assert_eq!(index_refreshes(cx), 1);
}

/// A pending row of a poll that holds the reservation.
fn poll_row(operation: OperationId) -> manyhands::repository::OperationDto {
    use manyhands::repository::{OperationAction, OperationDto, OperationFamily, OperationOwner};
    OperationDto {
        operation_id: Some(operation.to_string()),
        family: OperationFamily::Remote,
        owner: OperationOwner::Repository,
        action: OperationAction::Poll,
        state: "advertising".to_owned(),
        completed_step: None,
        next_action: None,
        item_id: None,
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    }
}

/// The desktop's own automatic poll is not offered the Resume that can only
/// fail; a row it does not know still is (Task 4 review I-2). The
/// scheduler feeds the request model the operations of its record and its
/// own sends.
#[gpui_kit::test]
async fn own_polls_are_not_offered_resume(cx: &mut TestAppContext) {
    use crate::desktop::model::operations::{ResumeOffer, resume_offer_among};
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    let gate = world.backend.hold_poll();
    world.backend.answer_poll(world.completed());
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the poll to be in flight", |_| {
        !world.backend.requests().is_empty()
    })
    .await;
    let (own, _) = world.backend.requests()[0];
    let known = |cx: &mut TestAppContext| {
        world
            .services
            .operations()
            .read_with(cx, |operations, _| operations.automatic_polls().clone())
    };
    World::settle(cx, "the request model to hear of it", |cx| {
        known(cx).contains(&own)
    })
    .await;
    let polls = known(cx);
    assert_eq!(
        resume_offer_among(&poll_row(own), &polls),
        ResumeOffer::NotOffered
    );
    assert_eq!(
        resume_offer_among(&poll_row(OperationId::new()), &polls),
        ResumeOffer::Offered,
        "a row the desktop did not send still offers Resume"
    );
    // Once it ends, the record forgets it.
    drop(gate);
    World::settle(cx, "the record to forget it", |cx| {
        !known(cx).contains(&own)
    })
    .await;
}

/// An answer that left the model's policy wrong takes the policy read
/// after it at once, not a read of its own later: the library says the
/// repository is paused, and the model learns it from the answer's read
/// (the Task 2 hand-off: `next_wake` is `None` while stale).
#[gpui_kit::test]
async fn stale_answer_rereads_policy(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world.backend.answer_poll(world.completed());
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the launch poll", |cx| {
        world.backend.polled_roots().len() == 1
            && matches!(world.phase(&root, cx), Some(Phase::Idle))
    })
    .await;

    // Paused behind the desktop's back; the next poll finds out.
    world.fixture.pause_polling(std::path::Path::new(&root));
    let reads = world.backend.remote_reads();
    world.advance(secs(301));
    world.tick(cx);
    World::settle(cx, "the answer to be applied", |cx| {
        world.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .repo(&root)
                .is_some_and(|repo| repo.policy.paused)
        })
    })
    .await;
    World::quiet(cx).await;
    let (stale, polled) = world.services.scheduler().read_with(cx, |scheduler, _| {
        (
            scheduler.schedule().stale(),
            world.backend.polled_roots().len(),
        )
    });
    assert!(stale.is_empty(), "{stale:?}");
    // No poll was made (the library's policy said no), and no other read
    // was needed.
    assert_eq!(polled, 1);
    assert_eq!(world.backend.remote_reads(), reads);
}

/// Window activation tries the in-flight lock again, and once it is held
/// the previous holder's stale poll is the lock holder's to finish (L-1,
/// L-2): its operation is known as ours and the row says why it waits.
#[gpui_kit::test]
async fn activation_takes_the_lock_for_a_stale_poll(cx: &mut TestAppContext) {
    use crate::desktop::services::{
        inflight::{Acquired, InflightStore},
        scheduler::Holding,
    };
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world.fixture.pause_polling(std::path::Path::new(&root));
    // Another instance holds the lock, and left a poll it did not finish.
    let mut other = InflightStore::open(world.fixture.data_dir());
    assert_eq!(other.try_acquire(), Acquired::Now);
    let stale = OperationId::new();
    other.poll_submitting(&root, stale);

    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the repository to be scheduled", |cx| {
        world.scheduled(&root, cx)
    })
    .await;
    let held = |cx: &mut TestAppContext| {
        world.services.scheduler().read_with(cx, |scheduler, _| {
            (
                scheduler.store().holds_lock(),
                scheduler.store().takeover_of(&root),
                scheduler.holding(&root),
            )
        })
    };
    assert_eq!(
        held(cx),
        (false, None, None),
        "a second instance takes nothing over"
    );

    drop(other);
    world.activate(cx);
    assert_eq!(held(cx), (true, Some(stale), Some(Holding::Paused)));
    let own = world
        .services
        .operations()
        .read_with(cx, |operations, _| operations.automatic_polls().clone());
    assert!(own.contains(&stale));
}

/// A suspend or a jump (the two clocks disagree) reads every policy and
/// remote again, once, so a remote added meanwhile to a repository that had
/// none is seen too (A3, R-6).
#[gpui_kit::test]
async fn clock_event_reads_policies_and_remotes(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    let plain = world.plain_repo("plain");
    world.fixture.pause_polling(std::path::Path::new(&root));
    world.list(&[root.clone(), plain.clone()], cx);
    World::settle(cx, "the repositories to be read", |cx| {
        world.scheduled(&root, cx) && world.backend.remote_reads() == 2
    })
    .await;
    world.tick(cx);
    assert!(!world.scheduled(&plain, cx));

    // A remote is added to the one that had none. The wall clock jumps an
    // hour; the monotonic clock a second.
    world.fixture.publish_over_ssh(std::path::Path::new(&plain));
    world.fixture.pause_polling(std::path::Path::new(&plain));
    world.clock.skew(time::Duration::hours(1), secs(1));
    world.tick(cx);
    World::settle(cx, "the remotes to be read again", |cx| {
        world.backend.remote_reads() == 4 && world.scheduled(&plain, cx)
    })
    .await;
    world.tick(cx);
    World::quiet(cx).await;
    assert_eq!(world.backend.remote_reads(), 4, "one event, one read");
}

/// From the entry to quit the scheduler starts nothing and arms no timer;
/// Cancel resumes it (R-33).
#[gpui_kit::test]
async fn quitting_starts_nothing(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world.backend.answer_poll(world.completed());
    let quit = |quitting, cx: &mut TestAppContext| {
        world
            .services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.set_quitting(quitting, cx))
    };
    quit(true, cx);
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the repository to be scheduled", |cx| {
        world.scheduled(&root, cx)
    })
    .await;
    World::quiet(cx).await;
    assert!(world.backend.polled_roots().is_empty());
    let armed = world
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| scheduler.armed_for());
    assert_eq!(armed, None);

    quit(false, cx);
    World::settle(cx, "the poll to start", |_| {
        !world.backend.polled_roots().is_empty()
    })
    .await;
}
