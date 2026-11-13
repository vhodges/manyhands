//! The poll intents and the scheduler (D6 Task 6): what a finished Check
//! now or policy verb moves, over the real drain and a real service. The
//! clock is injected; the transport is never reached (a Check now of a
//! repository with no selected key fails closed before any connection).

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use gpui_kit::TestAppContext;

use super::support::World;
use crate::desktop::{
    model::polling::{Phase, Policy},
    services::requests::{Intent, Lane},
};

fn policy(world: &World, root: &str, cx: &mut TestAppContext) -> Policy {
    world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler
            .schedule()
            .repo(&root.to_owned())
            .unwrap()
            .policy
            .clone()
    })
}

fn send(world: &World, intent: Intent) {
    world
        .services
        .worker()
        .submit(intent, Lane::User, None, None);
}

/// A finished policy verb moves the scheduler to take the policy the
/// library now stores (the answer's `PollPolicy` is that policy), and a
/// finished Check now to read the status again, for the deadline and the
/// row it left. Each reads once and moves exactly what it should: the
/// interval, the pause, neither. None cancels anything or clears
/// attention: they only read (R-11), and only Resume, which makes the
/// repository due, lets the scheduler start a poll.
#[gpui_kit::test]
async fn poll_once_rereads_status(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    // Paused, so the scheduler starts no poll of its own.
    world.fixture.pause_polling(Path::new(&root));
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the repository to be scheduled", |cx| {
        world.scheduled(&root, cx)
    })
    .await;
    World::quiet(cx).await;
    assert!(policy(&world, &root, cx).paused);
    let path = PathBuf::from(&root);

    // What a Resume starts is held in flight while its reads are counted,
    // then answered, so no user job finds it running (B2 would cancel it).
    let gate = world.backend.hold_poll();
    world.backend.answer_poll(world.completed());
    // Each verb, and Check now, in turn: the policy after it.
    let rows = [
        (
            Intent::PollConfigure {
                root: path.clone(),
                interval_seconds: 1800,
            },
            true,
            1800,
        ),
        // Check now moves neither, and starts no automatic poll.
        (Intent::PollOnce { root: path.clone() }, true, 1800),
        (Intent::PollResume { root: path.clone() }, false, 1800),
        (Intent::PollPause { root: path.clone() }, true, 1800),
    ];
    let mut reads = world.backend.remote_reads();
    for (intent, paused, interval) in rows {
        let label = intent.label();
        let resume = matches!(intent, Intent::PollResume { .. });
        if resume {
            assert!(world.backend.polled_roots().is_empty(), "{label}");
        }
        send(&world, intent);
        reads += 1;
        World::settle(cx, label, |_| world.backend.remote_reads() >= reads).await;
        World::settle(cx, label, |cx| {
            let now = policy(&world, &root, cx);
            now.paused == paused && now.interval == Duration::from_secs(interval)
        })
        .await;
        World::quiet(cx).await;
        assert_eq!(world.backend.remote_reads(), reads, "{label}: one read");
        let now = policy(&world, &root, cx);
        assert_eq!(now.paused, paused, "{label}");
        assert_eq!(now.interval, Duration::from_secs(interval), "{label}");
        if resume {
            gate.release();
            World::settle(cx, "the poll to be answered", |cx| {
                !world.backend.polled_roots().is_empty()
                    && matches!(
                        world.phase(&root, cx),
                        Some(Phase::Idle | Phase::Waiting { .. })
                    )
            })
            .await;
            World::quiet(cx).await;
        }
    }

    // Nothing was cancelled by any of them.
    assert!(world.backend.cancels().is_empty());
    assert_eq!(world.backend.polled_roots(), std::slice::from_ref(&root));
    assert!(!matches!(
        world.phase(&root, cx),
        Some(Phase::Attention(_) | Phase::Stopped)
    ));
}
