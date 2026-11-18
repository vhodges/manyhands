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
    // A remote that is not SSH, and an SSH one not selected for publication,
    // are no more polled than none (U-2): both halves of the gate.
    let local = world.fixture.create_repository("local");
    // (Selected while it was SSH, then pointed elsewhere: the library
    // refuses to select a remote it cannot publish to.)
    world.fixture.publish_over_ssh(&local);
    world
        .fixture
        .set_remote_url(&local, "origin", "file:///srv/elsewhere.git");
    let local = local.to_str().unwrap().to_owned();
    let unselected = world.fixture.create_repository("unselected");
    world.fixture.add_remote(
        &unselected,
        "origin",
        "ssh://git@example.invalid/team/repo.git",
    );
    let unselected = unselected.to_str().unwrap().to_owned();
    let roots = [
        first.clone(),
        second.clone(),
        plain.clone(),
        paused.clone(),
        local.clone(),
        unselected.clone(),
    ];

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
    assert!(!world.scheduled(&local, cx));
    assert!(!world.scheduled(&unselected, cx));
    assert!(world.scheduled(&paused, cx));
    assert_eq!(world.backend.remote_reads(), 6);

    // A remote added behind the desktop's back is seen on activation.
    world.fixture.publish_over_ssh(std::path::Path::new(&plain));
    world.activate(cx);
    World::settle(cx, "the remote to be read again", |_| {
        world.backend.polled_roots().contains(&plain)
    })
    .await;
    assert_eq!(world.backend.remote_reads(), 12);
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
        main: false,
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

/// Review 4 (B-, P- rules, R-7): at most one automatic poll is in flight.
/// A repository forgotten while its poll runs (the list drops it, or a read
/// says it is no longer polled) leaves the poll running, so nothing else is
/// submitted until that poll answers; its answer then lets the next go.
#[gpui_kit::test]
async fn a_repository_forgotten_mid_poll_still_holds_the_single_poll(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let roots = [world.ssh_repo("a1"), world.ssh_repo("a2")];
    let gate = world.backend.hold_poll();
    world.backend.answer_poll(world.completed());
    world.backend.answer_poll(world.completed());
    world.list(&roots, cx);
    World::settle(cx, "the first launch poll", |_| {
        !world.backend.polled_roots().is_empty()
    })
    .await;
    let running = world.backend.polled_roots()[0].clone();
    let other = roots.iter().find(|root| **root != running).unwrap().clone();
    assert_eq!(world.phase(&other, cx), Some(Phase::Idle));

    // The running poll's repository leaves the list.
    world.list(std::slice::from_ref(&other), cx);
    World::quiet(cx).await;
    assert_eq!(
        world.phase(&other, cx),
        Some(Phase::Idle),
        "no second poll while the first still runs"
    );

    // Its answer ends it, and the other goes.
    drop(gate);
    World::settle(cx, "the other repository's poll", |_| {
        world.backend.polled_roots().len() == 2
    })
    .await;
    assert_eq!(world.backend.polled_roots(), [running, other]);
}

/// Review 6 (R-34, B2): a preempted poll returns when the user lane is
/// empty, whatever the background lane holds. A background `IndexRefresh`
/// running behind the person's Check now does not keep the poll waiting.
#[gpui_kit::test]
async fn a_preempted_poll_returns_while_a_background_job_runs(cx: &mut TestAppContext) {
    use manyhands::repository::PollCompletion;
    use manyhands::results::{Outcome, ResultCode, Scope};

    use crate::desktop::services::JobKind;
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    let gate = world.backend.hold_poll();
    // The held poll ends `Cancelled`, as the library ends one it is asked to
    // stop; the poll that comes back completes.
    world
        .backend
        .answer_poll(world.report(PollCompletion::Cancelled, Vec::new(), false));
    world.backend.answer_poll(world.completed());
    world.backend.inner().answer_job(
        JobKind::PollOnce,
        manyhands::results::Envelope::mutation(
            "poll once",
            Scope::default(),
            Outcome::Success,
            ResultCode::Ok,
        ),
    );
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the poll to be in flight", |_| {
        !world.backend.requests().is_empty()
    })
    .await;
    // A user job preempts it; a background job waits behind that, and is
    // held at its start.
    let background = world.backend.inner().hold_job(JobKind::IndexRefresh);
    world.services.worker().submit(
        Intent::PollOnce {
            root: root.clone().into(),
        },
        Lane::User,
        None,
        None,
    );
    world.services.worker().submit(
        Intent::IndexRefresh {
            root: root.clone().into(),
        },
        Lane::Background,
        None,
        None,
    );
    drop(gate);
    background.wait_reached();
    World::quiet(cx).await;
    assert_eq!(
        world.phase(&root, cx),
        Some(Phase::Queued),
        "the poll is sent again behind the background job"
    );
    background.release();
}

/// Part A: the scheduler keeps each scheduled repository's stored outcome and
/// its shared copy from the same read that places it; a later read replaces
/// them; a read that fails leaves them; `reread` reads one repository again,
/// including one whose read failed, and ignores a root it never listed; and
/// the repository leaving the list drops both (P3, P26, P42).
#[gpui_kit::test]
async fn the_stored_outcome_and_shared_copy_are_kept(cx: &mut TestAppContext) {
    use manyhands::repository::PollingOutcome;

    let world = World::new(true, cx);
    let a = world.ssh_repo("a");
    world
        .backend
        .store_outcome(&a, PollingOutcome::TransportUnavailable);
    world.backend.answer_poll(world.completed());
    world.list(std::slice::from_ref(&a), cx);
    let stored = |cx: &mut TestAppContext| {
        world
            .services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.stored(&a))
    };
    World::settle(cx, "the registration to be read", |cx| stored(cx).is_some()).await;
    World::quiet(cx).await;
    assert_eq!(stored(cx), Some(PollingOutcome::TransportUnavailable));
    let copy = world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler
            .shared_copy(&a)
            .map(|copy| (copy.name.clone(), copy.location.clone()))
    });
    let (name, location) = copy.expect("the publication remote");
    assert_eq!(name, "origin");
    assert!(
        location.contains("example.invalid/team/repo.git"),
        "{location}"
    );

    // A later read replaces the stored outcome.
    world.backend.store_outcome(&a, PollingOutcome::Completed);
    world.activate(cx);
    World::settle(cx, "the outcome to be replaced", |cx| {
        stored(cx) == Some(PollingOutcome::Completed)
    })
    .await;
    World::quiet(cx).await;

    // The folder is renamed away: the read fails and leaves what was kept.
    let away = format!("{a}.away");
    std::fs::rename(&a, &away).unwrap();
    world.activate(cx);
    let failed = |cx: &mut TestAppContext| {
        world
            .services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.failed_reads.contains(&a))
    };
    World::settle(cx, "the read to fail", |cx| failed(cx)).await;
    World::quiet(cx).await;
    assert_eq!(stored(cx), Some(PollingOutcome::Completed), "kept");
    let kept = world
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| scheduler.shared_copy(&a).is_some());
    assert!(kept, "the shared copy is kept");

    // Renamed back, it is read again by `reread` alone: one more read of
    // its remotes, and it is no longer a failed read.
    std::fs::rename(&away, &a).unwrap();
    let reads = world
        .backend
        .remote_read_roots()
        .iter()
        .filter(|root| **root == a)
        .count();
    // The row's facts changed as Retry was chosen, so the scheduler tells
    // its observers at once, before the read lands.
    let notified = std::rc::Rc::new(std::cell::Cell::new(0_u32));
    let observing = cx.update(|cx| {
        let notified = notified.clone();
        cx.observe(world.services.scheduler(), move |_, _| {
            notified.set(notified.get() + 1)
        })
    });
    world.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.reread(a.clone(), cx);
    });
    assert_eq!(notified.get(), 1, "Retry is felt at once");
    drop(observing);
    World::settle(cx, "the read to land", |cx| !failed(cx)).await;
    World::quiet(cx).await;
    let after = world
        .backend
        .remote_read_roots()
        .iter()
        .filter(|root| **root == a)
        .count();
    assert_eq!(after, reads + 1, "read again");

    // A root the scheduler never listed is not read.
    let never = world.ssh_repo("never");
    let (reads, calls) = (world.backend.remote_reads(), world.backend.calls().len());
    world.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.reread(never.clone(), cx);
    });
    World::quiet(cx).await;
    assert_eq!(world.backend.remote_reads(), reads, "no read");
    assert_eq!(world.backend.calls().len(), calls, "no call");

    // The repository leaves the list: both are gone.
    world.list(&[], cx);
    World::quiet(cx).await;
    let gone = world.services.scheduler().read_with(cx, |scheduler, _| {
        (scheduler.stored(&a), scheduler.shared_copy(&a).is_none())
    });
    assert_eq!(gone, (None, true));
}
