//! The poll intents and the scheduler (D6 Task 6): what a finished Check
//! now or policy verb moves, over the real drain and a real service. The
//! clock is injected; the transport is never reached (a Check now of a
//! repository with no selected key fails closed before any connection).

use std::{path::Path, time::Duration};

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
/// row it left. Neither cancels anything or clears attention: they only
/// read (R-11), and Check now starts no automatic poll.
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
    let before = policy(&world, &root, cx);
    assert!(before.paused);
    let reads = world.backend.remote_reads();

    // A policy verb: the interval it stored is the scheduler's.
    send(
        &world,
        Intent::PollConfigure {
            root: root.clone().into(),
            interval_seconds: 1800,
        },
    );
    World::settle(cx, "the interval to be taken", |cx| {
        policy(&world, &root, cx).interval == Duration::from_secs(1800)
    })
    .await;
    World::quiet(cx).await;
    assert_eq!(world.backend.remote_reads(), reads + 1);
    assert!(policy(&world, &root, cx).paused, "only the interval moved");

    // Check now: the status is read again.
    send(
        &world,
        Intent::PollOnce {
            root: root.clone().into(),
        },
    );
    World::settle(cx, "the status to be read again", |_| {
        world.backend.remote_reads() == reads + 2
    })
    .await;
    World::quiet(cx).await;
    assert_eq!(world.backend.remote_reads(), reads + 2);

    // Neither cancelled or polled anything on its own.
    assert!(world.backend.cancels().is_empty());
    assert!(world.backend.polled_roots().is_empty());
    assert!(matches!(
        world.phase(&root, cx),
        Some(Phase::Idle | Phase::Waiting { .. })
    ));
}
