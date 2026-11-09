//! More of the scheduler entity's behaviour (D6 Task 5): the own-poll
//! exclusion, stale rows, the lock on activation, the clock, quit and a
//! read that keeps failing. The clock is injected.

use std::time::Duration;

use gpui_kit::TestAppContext;
use manyhands::repository::OperationId;

use super::support::World;
use crate::desktop::model::polling::Phase;

fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
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

/// A policy read that keeps failing is not retried in a loop: the old
/// policy stays, and the repository is read again only at the next window
/// activation, clock event or reconfiguration. (A stale repository is read
/// again at once, so a read that fails must not leave it stale and
/// unexcused.)
#[gpui_kit::test]
async fn failing_read_waits_for_activation(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world.fixture.pause_polling(std::path::Path::new(&root));
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the repository to be scheduled", |cx| {
        world.scheduled(&root, cx)
    })
    .await;
    let resolves = || {
        world
            .backend
            .calls()
            .iter()
            .filter(|call| **call == crate::desktop::test_support::polling::PollCall::Resolve)
            .count()
    };

    // The repository goes, and the clock jumps: its read now fails.
    std::fs::remove_dir_all(&root).unwrap();
    world.clock.skew(time::Duration::hours(1), secs(1));
    world.tick(cx);
    // This must park: no read follows the failed one on its own.
    cx.run_until_parked();
    let after_failure = resolves();
    for _ in 0..5 {
        world.tick(cx);
        cx.run_until_parked();
    }
    assert_eq!(resolves(), after_failure, "a failed read was retried");
    assert!(world.scheduled(&root, cx), "the old policy is kept");

    // The next activation reads it again, once.
    world.activate(cx);
    cx.run_until_parked();
    assert_eq!(resolves(), after_failure + 1);
    cx.run_until_parked();
    assert_eq!(resolves(), after_failure + 1);
}

/// A host approval or replacement reaches every scheduled repository whose
/// publication remote is reached at that host (R-11): the one it names, an
/// idle one on the same host, one held for the host, and not one on
/// another host.
#[gpui_kit::test]
async fn host_change_reaches_repos_on_the_host(cx: &mut TestAppContext) {
    use super::reconfigure::{Heard, Moment, Scope};
    use crate::desktop::model::polling::Authority;
    use manyhands::repository::{
        RemotePollError,
        transport::{
            HostKeyIdentity, SshAuthority, SshDirection, SshTransportError, SshTransportErrorKind,
        },
    };
    use std::path::PathBuf;

    let world = World::new(true, cx);
    let at = |name: &str, url: &str, paused: bool| {
        let root = world.fixture.create_repository(name);
        world.fixture.publish_at(&root, url);
        if paused {
            world.fixture.pause_polling(&root);
        }
        root.to_str().unwrap().to_owned()
    };
    let same = "ssh://git@example.invalid/team/repo.git";
    let named = at("named", same, true);
    let idle = at("idle", same, true);
    let held = at("held", same, false);
    let other = at("other", "ssh://git@other.invalid/team/repo.git", true);

    // `held` polls once and waits for the host to be approved.
    let host = SshAuthority {
        host: "example.invalid".to_owned(),
        port: 22,
    };
    world
        .backend
        .fail_poll(RemotePollError::Transport(SshTransportError {
            root: PathBuf::from(&held),
            remote_name: "origin".to_owned(),
            direction: SshDirection::Fetch,
            selected_key_id: None,
            authority: Some(host.clone()),
            kind: SshTransportErrorKind::HostApprovalRequired {
                presented: HostKeyIdentity {
                    algorithm: "ssh-ed25519".to_owned(),
                    sha256: "SHA256:x".to_owned(),
                },
            },
        }));
    world.list(std::slice::from_ref(&held), cx);
    World::settle(cx, "the host attention", |cx| {
        matches!(world.phase(&held, cx), Some(Phase::Attention(_)))
    })
    .await;
    world.list(
        &[named.clone(), idle.clone(), held.clone(), other.clone()],
        cx,
    );
    World::settle(cx, "the rest to be scheduled", |cx| {
        [&named, &idle, &other]
            .iter()
            .all(|root| world.scheduled(root, cx))
    })
    .await;
    let before = world.backend.remote_read_roots().len();

    world.heard(
        vec![Heard::Reconfiguration(
            Moment::Finished,
            Scope::Host {
                root: named.clone(),
                authority: Authority {
                    host: host.host.clone(),
                    port: host.port,
                },
            },
        )],
        cx,
    );
    World::settle(cx, "the policies to be read again", |_| {
        world.backend.remote_read_roots().len() >= before + 3
    })
    .await;
    World::quiet(cx).await;
    let mut read: Vec<String> = world.backend.remote_read_roots()[before..].to_vec();
    read.sort();
    let mut expected = vec![named, idle, held.clone()];
    expected.sort();
    assert_eq!(read, expected);
    // The attention was cleared: the one held for the host polls again.
    let held_polls = world
        .backend
        .polled_roots()
        .iter()
        .filter(|polled| **polled == held)
        .count();
    assert_eq!(held_polls, 2);
}
