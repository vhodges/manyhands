//! The scheduler over a real service (D6 Task 5): a second library actor
//! holding the remote reservation, a pending local operation, and the
//! reconfigurations. The transport is never reached: `Busy` and
//! `recovery_required` come before it (decision 26), and a poll that does
//! reach it finds no selected key, which the library answers without a
//! connection. The clock is injected.

use std::{path::Path, path::PathBuf, time::Duration};

use gpui_kit::TestAppContext;
use manyhands::repository::{
    OperationId, PollCompletion, PollPublicationRemoteRequest, RemoteOperationTarget,
    RemotePollError, RemotePollInvocation, RemoteRefPlan, RemoteReservationOutcome,
    RepositoryService, SharedKeyId,
    keys::SessionCredentials,
    transport::{SshAuthority, SshTransportError, SshTransportErrorKind},
};

use super::{
    reconfigure::{Heard, Moment, Scope, scope_of},
    support::World,
};
use crate::desktop::{
    model::polling::{Phase, Reason, Why},
    services::requests::Intent,
    test_support::fixtures::NoPassphrase,
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ0";

fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// The polls made so far: their operation and `restart`.
fn sent(world: &World) -> Vec<(OperationId, bool)> {
    world.backend.requests()
}

/// Moves the clock by `by`, ticks, and waits for the poll that follows (the
/// `n`th), which must come.
async fn due_after(world: &World, root: &str, by: Duration, n: usize, cx: &mut TestAppContext) {
    world.advance(by);
    world.tick(cx);
    World::settle(cx, "the next poll", |_| sent(world).len() >= n).await;
    // Its answer is applied before the next step is measured.
    World::settle(cx, "its answer", |cx| {
        matches!(world.phase(root, cx), Some(Phase::Waiting { .. }))
    })
    .await;
}

/// Nothing is sent over `by` more of the clock.
async fn nothing_for(world: &World, by: Duration, cx: &mut TestAppContext) {
    let before = sent(world).len();
    world.advance(by);
    world.tick(cx);
    World::quiet(cx).await;
    assert_eq!(sent(world).len(), before, "a poll before its step");
}

/// A second library actor over the same registry holds the remote
/// reservation with a manual operation (decision 26: the same process,
/// where track rule 4 asked for another). The automatic poll answers `Busy`
/// before any transport, resends the same operation with `restart` after 5,
/// 15 and 60 seconds, and then waits the interval and uses a new one. It
/// never sends the other actor's operation (A4, B3, L-5).
#[gpui_kit::test]
async fn second_actor_busy(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    // The repository's polling endpoints are recorded by a first poll, and a
    // change of them would fence the other actor's row (Fact 29). An
    // explicit poll does it and moves no deadline; with the transport
    // runtime uninitialized, as in a test, it fails closed before any
    // connection.
    let configured = world.fixture.service().poll_publication_remote(
        PollPublicationRemoteRequest {
            root: PathBuf::from(&root),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: None,
            restart: false,
        },
        &mut SessionCredentials::new(NoPassphrase),
    );
    assert!(matches!(configured, Err(RemotePollError::Transport(_))));
    let other = RepositoryService::open_at(world.fixture.data_dir()).unwrap();
    let target = RemoteOperationTarget::for_primary_synchronization(
        &RemoteRefPlan::from_configuration("origin", "main").unwrap(),
    );
    let held = OperationId::new();
    let reserved = other
        .reserve_remote_operation(Path::new(&root), held, &target)
        .unwrap();
    assert!(matches!(reserved, RemoteReservationOutcome::Reserved(_)));

    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the first poll's Busy", |cx| {
        sent(&world).len() == 1 && matches!(world.phase(&root, cx), Some(Phase::Waiting { .. }))
    })
    .await;
    let (first, restarted) = sent(&world)[0];
    assert!(!restarted);
    assert_ne!(first, held);
    assert!(matches!(
        world.phase(&root, cx),
        Some(Phase::Waiting {
            why: Why::Busy,
            op: Some(op),
            ..
        }) if op == first
    ));
    // The other actor's row is the active one, and the record keeps no
    // entry for it: it is not ours to take over (L-5).
    let status = other
        .polling_status(&other.resolve_repository(Path::new(&root)).unwrap())
        .unwrap();
    assert_eq!(status.active_operation_id, Some(held.to_string()));
    let entry = world
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| scheduler.store().poll_entry(&root));
    assert_eq!(entry, None);

    // 5 s, 15 s, 60 s: the same operation, restarting.
    let mut polls = 1;
    for step in [5, 15, 60] {
        nothing_for(&world, secs(step - 1), cx).await;
        polls += 1;
        due_after(&world, &root, secs(1), polls, cx).await;
        assert_eq!(*sent(&world).last().unwrap(), (first, true), "{step} s");
    }
    // Then the interval, and a new operation.
    let interval = world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler.schedule().repo(&root).unwrap().policy.interval
    });
    nothing_for(&world, interval - secs(1), cx).await;
    due_after(&world, &root, secs(1), polls + 1, cx).await;
    let (last, restarted) = *sent(&world).last().unwrap();
    assert!(!restarted);
    assert_ne!(last, first);
    assert!(sent(&world).iter().all(|(op, _)| *op != held));
}

/// With a local operation pending, the poll answers `recovery_required`
/// before any transport. The repository waits for an unfinished operation,
/// and is polled at most once per R-4 step (5 s, 15 s, 60 s) and then once
/// per interval, never at the stored deadline, which no attempt recorded and
/// so is long past (R-35, A23).
#[gpui_kit::test]
async fn unfinished_operation_waits(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world
        .fixture
        .create_ticket(Path::new(&root), TICKET, "One", "open");
    world
        .fixture
        .strand_ticket_save(Path::new(&root), TICKET, "Renamed");

    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the first poll's refusal", |cx| {
        sent(&world).len() == 1 && matches!(world.phase(&root, cx), Some(Phase::Waiting { .. }))
    })
    .await;
    assert!(matches!(
        world.phase(&root, cx),
        Some(Phase::Waiting {
            why: Why::Unfinished,
            op: None,
            ..
        })
    ));
    let mut polls = 1;
    for step in [5, 15, 60] {
        nothing_for(&world, secs(step - 1), cx).await;
        polls += 1;
        due_after(&world, &root, secs(1), polls, cx).await;
        assert!(matches!(
            world.phase(&root, cx),
            Some(Phase::Waiting {
                why: Why::Unfinished,
                ..
            })
        ));
    }
    let interval = world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler.schedule().repo(&root).unwrap().policy.interval
    });
    nothing_for(&world, interval - secs(1), cx).await;
    due_after(&world, &root, secs(1), polls + 1, cx).await;
    // Every attempt was a new operation: nothing was recorded to restart.
    let operations: Vec<_> = sent(&world).iter().map(|(op, _)| *op).collect();
    let unique: std::collections::HashSet<_> = operations.iter().collect();
    assert_eq!(unique.len(), operations.len());
    assert!(sent(&world).iter().all(|(_, restart)| !restart));
}

/// The next poll fails as the transport does when no key is selected. The
/// library answers it without a connection, but not in a test, where the
/// transport runtime is uninitialized and fails first.
fn needs_a_key(world: &World, root: &str) {
    world
        .backend
        .fail_poll(RemotePollError::Transport(SshTransportError {
            root: PathBuf::from(root),
            remote_name: "origin".to_owned(),
            direction: manyhands::repository::transport::SshDirection::Fetch,
            selected_key_id: None,
            authority: None,
            kind: SshTransportErrorKind::NoSelectedKey,
        }));
}

/// A repository `b` sits in attention (no key is selected); another, `a`, has a poll in
/// flight, held. The change `make` builds for `a` finishes: the poll of
/// every repository it reaches is cancelled, its attention cleared, and its
/// policy and remotes read again, so it polls again; a removal drops the
/// schedule instead. A repository it does not reach is untouched (R-11, A6).
async fn finished(
    make: fn(PathBuf, SharedKeyId, SshAuthority) -> Intent,
    reaches_other: bool,
    cx: &mut TestAppContext,
) {
    let world = World::new(true, cx);
    let a = world.ssh_repo("a");
    let b = world.ssh_repo("b");
    let intent = make(
        PathBuf::from(&a),
        SharedKeyId::new(),
        SshAuthority {
            host: "example.invalid".to_owned(),
            port: 22,
        },
    );
    let removal = matches!(intent, Intent::RepoRemove { .. });

    needs_a_key(&world, &b);
    world.list(std::slice::from_ref(&b), cx);
    World::settle(cx, "b's attention", |cx| {
        matches!(world.phase(&b, cx), Some(Phase::Attention(_)))
    })
    .await;
    assert_eq!(world.backend.polled_roots(), std::slice::from_ref(&b));

    // `a`'s poll is in flight, and the library would end it `Cancelled`.
    let gate = world.backend.hold_poll();
    world
        .backend
        .answer_poll(world.report(PollCompletion::Cancelled, Vec::new(), false));
    world.list(&[a.clone(), b.clone()], cx);
    World::settle(cx, "a's poll in flight", |_| {
        world.backend.polled_roots() == [b.clone(), a.clone()]
    })
    .await;
    let reads = world.backend.remote_reads();

    let scope = scope_of(&intent, true).expect("a reconfiguring intent");
    world.heard(vec![Heard::Reconfiguration(Moment::Finished, scope)], cx);
    // Its poll is asked to stop; the hold stands for the safe point.
    World::settle(cx, "the cancel", |_| !world.backend.cancels().is_empty()).await;
    drop(gate);
    let polled = |root: &str| {
        world
            .backend
            .polled_roots()
            .iter()
            .filter(|polled| polled.as_str() == root)
            .count()
    };
    if removal {
        World::settle(cx, "a's schedule to go", |cx| !world.scheduled(&a, cx)).await;
        assert_eq!(polled(&a), 1);
    } else {
        World::settle(cx, "a to poll again", |_| polled(&a) == 2).await;
    }
    if reaches_other {
        World::settle(cx, "b to poll again", |_| polled(&b) == 2).await;
    }
    World::quiet(cx).await;
    if !reaches_other {
        assert_eq!(polled(&b), 1);
        assert!(matches!(world.phase(&b, cx), Some(Phase::Attention(_))));
    }
    // Policies and remotes were read again for exactly those it reached.
    let reached = 1 + usize::from(reaches_other) - usize::from(removal);
    assert_eq!(world.backend.remote_reads() - reads, reached);
}

type Make = fn(PathBuf, SharedKeyId, SshAuthority) -> Intent;

/// R-11 and A6. A change that is submitted drops the queued retirements of
/// the repositories it reaches; one that finishes cancels their polls,
/// clears their attention and reads their policies and remotes again so they
/// re-arm; a request that only moves the stored policy re-reads it and
/// cancels nothing.
#[gpui_kit::test]
async fn reconfiguration_cancels_and_rearms(cx: &mut TestAppContext) {
    // Submitted: the loop of every repository it reaches is dropped.
    {
        let world = World::new(true, cx);
        let x = world.ssh_repo("x");
        let y = world.ssh_repo("y");
        world.fixture.pause_polling(Path::new(&x));
        world.fixture.pause_polling(Path::new(&y));
        world.list(&[x.clone(), y.clone()], cx);
        World::settle(cx, "both to be scheduled", |cx| {
            world.scheduled(&x, cx) && world.scheduled(&y, cx)
        })
        .await;
        let retiring = |cx: &mut TestAppContext| {
            [&x, &y].map(|root| matches!(world.phase(root, cx), Some(Phase::Retiring)))
        };
        let begin = |cx: &mut TestAppContext| {
            world.services.scheduler().update(cx, |scheduler, _| {
                assert!(scheduler.schedule_mut().begin_retiring(&x));
                assert!(scheduler.schedule_mut().begin_retiring(&y));
            });
        };
        begin(cx);
        assert_eq!(retiring(cx), [true, true]);
        let own = |root: &String| Scope::Repository {
            root: root.clone(),
            gone: false,
        };
        world.heard(vec![Heard::Reconfiguration(Moment::Submitted, own(&x))], cx);
        assert_eq!(retiring(cx), [false, true]);
        world.heard(
            vec![Heard::Reconfiguration(Moment::Submitted, Scope::Keys)],
            cx,
        );
        assert_eq!(retiring(cx), [false, false]);
    }

    // R-11's twelve intents. The scheduler reacts to the scope an intent
    // reaches, not to the intent, so each kind of scope is run once with
    // the first intent that has it, and every other intent must map to a
    // scope of a kind already run.
    let cases: [(&str, Make); 12] = [
        ("RemoteAdd", |root, _, _| Intent::RemoteAdd {
            root,
            name: "origin".to_owned(),
            url: "ssh://git@example.invalid/a.git".to_owned(),
        }),
        ("RemoteRemove", |root, _, _| Intent::RemoteRemove {
            root,
            name: "origin".to_owned(),
        }),
        ("RemoteSelect", |root, _, _| Intent::RemoteSelect {
            root,
            name: None,
        }),
        ("KeySelect", |_, id, _| Intent::KeySelect { id }),
        ("KeyClear", |_, _, _| Intent::KeyClear),
        ("KeyRemove", |_, id, _| Intent::KeyRemove { id }),
        ("KeyDelete", |_, id, _| Intent::KeyDelete { id }),
        ("HostApprove", |root, _, authority| Intent::HostApprove {
            root,
            authority,
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint: "SHA256:x".to_owned(),
        }),
        ("HostReplace", |root, _, authority| Intent::HostReplace {
            root,
            authority,
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint: "SHA256:x".to_owned(),
            old_fingerprint: "SHA256:y".to_owned(),
        }),
        ("RepoCreate", |root, _, _| Intent::RepoCreate {
            root,
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: None,
        }),
        ("RepoEnable", |root, _, _| Intent::RepoEnable {
            root,
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: None,
        }),
        ("RepoRemove", |root, _, _| Intent::RepoRemove { root }),
    ];
    let mut run = std::collections::HashSet::new();
    for (name, make) in cases {
        let intent = make(
            PathBuf::from("/a"),
            SharedKeyId::new(),
            SshAuthority {
                host: "example.invalid".to_owned(),
                port: 22,
            },
        );
        let kind = match scope_of(&intent, true).unwrap_or_else(|| panic!("{name} reconfigures")) {
            Scope::Repository { gone: false, .. } => "repository",
            Scope::Repository { gone: true, .. } => "removal",
            Scope::Host { .. } => "host",
            Scope::Keys => "keys",
        };
        // A removal that went through is the only one that drops a schedule.
        assert_eq!(kind == "removal", name == "RepoRemove", "{name}");
        if run.insert(kind) {
            // `b` is on the same host as `a`, so a host change reaches it.
            finished(make, kind == "keys" || kind == "host", cx).await;
        }
    }
    assert_eq!(run.len(), 4);

    // A policy request only reads the policy again: nothing is cancelled,
    // and the attention it is not about stays.
    let world = World::new(true, cx);
    let b = world.ssh_repo("b");
    needs_a_key(&world, &b);
    world.list(std::slice::from_ref(&b), cx);
    World::settle(cx, "b's attention", |cx| {
        matches!(world.phase(&b, cx), Some(Phase::Attention(_)))
    })
    .await;
    let reads = world.backend.remote_reads();
    world.heard(vec![Heard::PolicyMoved(b.clone())], cx);
    World::settle(cx, "the policy to be read", |_| {
        world.backend.remote_reads() == reads + 1
    })
    .await;
    World::quiet(cx).await;
    assert!(world.backend.cancels().is_empty());
    assert_eq!(world.backend.polled_roots(), std::slice::from_ref(&b));
    assert!(matches!(
        world.phase(&b, cx),
        Some(Phase::Attention(Reason::NoKeySelected(_)))
    ));
}
