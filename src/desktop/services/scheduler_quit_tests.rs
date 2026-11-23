//! Quit at the scheduler (D6 Task 10, R-33, A7): nothing the person did
//! earlier starts once quitting is asked (an armed Check now, Unlock…,
//! Resume checking), a poll quit drops from the queue settles its record,
//! and a relaunch goes by the stored deadline. The clock is injected; the
//! transport is never reached.

use std::time::Duration;

use gpui_kit::TestAppContext;
use manyhands::repository::{
    REGISTRY_FILE, RepositoryService, SharedKeyId, transport::SshTransportErrorKind,
};

use super::{
    attention_tests::{checks, entries, unlock},
    reconfigure::{Heard, Moment, Scope as Change},
    support::World,
};
use crate::desktop::{
    model::polling::{Clock, Phase, Reason},
    services::{
        JobKind, Services,
        requests::{Intent, Lane},
    },
    test_support::polling::{PollCall, PollingBackend, transport_error},
};

fn quit(world: &World, quitting: bool, cx: &mut TestAppContext) {
    world
        .services
        .scheduler()
        .update(cx, |scheduler, cx| scheduler.set_quitting(quitting, cx));
}

/// Registry access beside the service's own connection, for state a test
/// cannot make through the library: a polling hold and a stored deadline.
fn registry(world: &World) -> rusqlite::Connection {
    rusqlite::Connection::open(world.fixture.data_dir().join(REGISTRY_FILE)).unwrap()
}

/// C3, R-33: Choose a key… armed a Check now for when the change finishes.
/// The change finishing after quitting was asked does not send it, and it is
/// not sent after Cancel either (the arm goes with the ask): the entry stays
/// until the next poll or the person's own Check now.
#[gpui_kit::test]
async fn quit_drops_an_armed_check_now(cx: &mut TestAppContext) {
    use SshTransportErrorKind::NoSelectedKey;
    let world = World::new(true, cx);
    let a = world.ssh_repo("a");
    world
        .backend
        .fail_poll(transport_error(&a, NoSelectedKey, None, None));
    world.list(std::slice::from_ref(&a), cx);
    World::settle(cx, "the entry", |cx| !entries(&world, cx).is_empty()).await;
    world
        .services
        .scheduler()
        .update(cx, |scheduler, _| scheduler.check_once_changed(a.clone()));

    quit(&world, true, cx);
    let scope = Change::Repository {
        root: a.clone(),
        gone: false,
    };
    world.heard(vec![Heard::Reconfiguration(Moment::Succeeded, scope)], cx);
    World::quiet(cx).await;
    assert!(checks(&world, cx).is_empty(), "no Check now while quitting");

    quit(&world, false, cx);
    World::quiet(cx).await;
    assert!(checks(&world, cx).is_empty(), "the arm went with the ask");
}

/// C3, R-33: Unlock… submits a Check now. It does nothing once quitting is
/// asked, and works again after Cancel.
#[gpui_kit::test]
async fn quit_stops_unlock(cx: &mut TestAppContext) {
    use SshTransportErrorKind::UnlockCancelled;
    let world = World::new(true, cx);
    let a = world.ssh_repo("a");
    let key = SharedKeyId::new();
    world
        .backend
        .fail_poll(transport_error(&a, UnlockCancelled, Some(key), None));
    world.list(std::slice::from_ref(&a), cx);
    World::settle(cx, "the locked key", |cx| {
        entries(&world, cx).first().map(|entry| entry.0.clone()) == Some(Reason::KeyLocked(key))
    })
    .await;

    quit(&world, true, cx);
    assert!(!unlock(&world, key, cx), "nothing starts while quitting");
    World::quiet(cx).await;
    assert!(checks(&world, cx).is_empty());

    quit(&world, false, cx);
    assert!(unlock(&world, key, cx), "Cancel lets it go on");
    World::settle(cx, "the Check now", |cx| checks(&world, cx) == [a.clone()]).await;
}

/// C2, decision 10, R-33: Resume checking releases the library's hold on a
/// background task. Asked while quitting, it starts nothing; one already out
/// when quitting is asked finishes, but the repository it frees is not
/// polled until Cancel.
#[gpui_kit::test]
async fn quit_stops_resume_checking(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let a = world.ssh_repo("a");
    let held = registry(&world)
        .execute(
            "UPDATE remote_polling_state SET recovery_suspended=1 WHERE repository_id IN \
             (SELECT id FROM repositories WHERE root_path LIKE ?1)",
            [format!("%/{}", a.rsplit('/').next().unwrap())],
        )
        .unwrap();
    assert_eq!(held, 1, "the repository has a polling row");
    world.list(std::slice::from_ref(&a), cx);
    World::settle(cx, "the hold", |cx| {
        entries(&world, cx).first().map(|entry| entry.0.clone())
            == Some(Reason::RecoveryHeld(a.clone()))
    })
    .await;
    let resume = |cx: &mut TestAppContext| {
        world
            .services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.resume_checking(a.clone(), cx))
    };

    // Asked while quitting: not started.
    quit(&world, true, cx);
    resume(cx);
    World::quiet(cx).await;
    assert!(world.backend.resumed().is_empty());
    assert!(world.backend.polled_roots().is_empty());

    // Out before quitting was asked: the hold is released, and the
    // repository is polled once Cancel lets the scheduler go on.
    quit(&world, false, cx);
    world.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.resume_checking(a.clone(), cx);
        scheduler.set_quitting(true, cx);
    });
    World::settle(cx, "the hold to be released", |_| {
        world.backend.resumed() == std::slice::from_ref(&a)
    })
    .await;
    World::quiet(cx).await;
    assert!(world.backend.polled_roots().is_empty(), "frozen");
    quit(&world, false, cx);
    World::settle(cx, "the poll", |_| !world.backend.polled_roots().is_empty()).await;
}

/// R-31, Fact 36: a poll still queued when quit closes the queue never runs.
/// Its in-flight entry is settled by the answer quit gives it (cancelled, no
/// completion time), not left for the next launch to resend, and the
/// repository stops (`Stopped`, R-33).
#[gpui_kit::test]
async fn quit_withdraws_queued_poll(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let a = world.ssh_repo("a");
    // A user job holds the worker, so the poll waits in the queue.
    let held = world.backend.inner().hold_job(JobKind::IndexRefresh);
    world.services.worker().submit(
        Intent::IndexRefresh {
            root: a.clone().into(),
        },
        Lane::User,
        None,
        None,
    );
    held.wait_reached();
    world.list(std::slice::from_ref(&a), cx);
    World::settle(cx, "the poll to be queued", |cx| {
        world.phase(&a, cx) == Some(Phase::Queued)
    })
    .await;
    let entry = |cx: &mut TestAppContext| {
        world
            .services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.store().poll_entry(&a))
    };
    assert!(entry(cx).is_some(), "written before the poll is submitted");

    world.services.worker().quit();
    World::settle(cx, "the poll's answer", |cx| {
        world.phase(&a, cx) == Some(Phase::Stopped)
    })
    .await;
    assert_eq!(entry(cx), None, "settled by the answer");
    assert!(world.backend.polled_roots().is_empty(), "it never ran");
    held.release();
}

/// Whether a poll has started: `poll_eligibility` is the worker's own call,
/// the first the scheduler's reads never make.
fn started(backend: &PollingBackend) -> bool {
    backend.calls().contains(&PollCall::PollEligibility)
}

/// A7, decision 23: a fresh `Services` over the same data directory, with
/// its own reopened library service, polls by the deadline the first stored
/// and by nothing else: not at launch, not because the first session's poll
/// was cut short, and not before that time.
#[gpui_kit::test]
async fn relaunch_keeps_deadline(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let a = world.ssh_repo("a");
    // The first session's poll completed; the library stored the interval
    // after it (this stands for it: the transport is never reached).
    let due = world.clock.utc() + time::Duration::seconds(200);
    let stored = registry(&world)
        .execute(
            "UPDATE remote_polling_state SET next_eligible_at=?1 WHERE repository_id IN \
             (SELECT id FROM repositories WHERE root_path LIKE ?2)",
            rusqlite::params![
                due.unix_timestamp(),
                format!("%/{}", a.rsplit('/').next().unwrap())
            ],
        )
        .unwrap();
    assert_eq!(stored, 1);

    // The relaunch: a new service over the data directory, a new backend, a
    // new `Services` and worker.
    let reopened = RepositoryService::open_at(world.fixture.data_dir()).unwrap();
    let delaying = world.fixture.delaying_backend();
    delaying.replace_service(reopened);
    let backend = std::sync::Arc::new(PollingBackend::new(delaying));
    let services = cx.update(|cx| {
        Services::start(backend.clone(), cx)
            .without_launch_refresh()
            .with_clock(world.clock.clone(), cx)
            .with_polling()
    });
    cx.update(|cx| services.repositories_listed(std::slice::from_ref(&a), cx));
    World::settle(cx, "the policy to be read", |cx| {
        services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.schedule().repo(&a).is_some())
    })
    .await;
    let tick = |cx: &mut TestAppContext| {
        services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.tick(cx))
    };

    tick(cx);
    World::quiet(cx).await;
    assert!(!started(&backend), "not due at launch");
    world.advance(Duration::from_secs(199));
    tick(cx);
    World::quiet(cx).await;
    assert!(!started(&backend), "not before the deadline");
    world.advance(Duration::from_secs(1));
    tick(cx);
    // The poll started: the library, whose clock is the real one, finds it
    // not due and says so, which is all the test needs of it.
    World::settle(cx, "the poll", |_| started(&backend)).await;
}
