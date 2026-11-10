//! Needs attention at the scheduler (D6 Task 8, design C3, A3, B5, K-23,
//! K-26): what a locked key, a host answer or a missing key becomes, what
//! Unlock… does and does not touch, and what clears an entry. The clock is
//! injected; the transport is never reached. The poll's passphrase request
//! goes through the real provider and a real session; a Check now's goes
//! through the worker's own provider, sharing its declined keys.

use std::{cell::RefCell, path::Path, thread};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        RemotePollError, SharedKeyId,
        keys::{SecretPassphrase, SessionCredentials, SessionUnlockFailure, UnlockRequest},
        transport::SshTransportErrorKind,
    },
    results::{Envelope, Outcome, ResultCode, Scope},
};

use super::{
    reconfigure::{Heard, Moment, Scope as Change},
    support::World,
};
use crate::desktop::{
    model::polling::{Phase, Reason},
    services::{
        JobKind,
        prompts::PromptReply,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::polling::{Asked, locked, transport_error, unlock_request},
};

pub(super) fn send(world: &World, intent: Intent) -> DesktopRequestId {
    world
        .services
        .worker()
        .submit(intent, Lane::User, None, None)
}

/// The scheduler's entries as `(reason, roots)`, roots sorted.
pub(super) fn entries(world: &World, cx: &mut TestAppContext) -> Vec<(Reason, Vec<String>)> {
    world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler
            .schedule()
            .attention()
            .into_iter()
            .map(|entry| {
                let mut roots = entry.roots;
                roots.sort();
                (entry.reason, roots)
            })
            .collect()
    })
}

fn sorted(mut roots: Vec<String>) -> Vec<String> {
    roots.sort();
    roots
}

fn paused(world: &World, root: &str, cx: &mut TestAppContext) -> bool {
    world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler
            .schedule()
            .repo(&root.to_owned())
            .unwrap()
            .policy
            .paused
    })
}

pub(super) fn unlock(world: &World, key: SharedKeyId, cx: &mut TestAppContext) -> bool {
    world
        .services
        .scheduler()
        .update(cx, |scheduler, cx| scheduler.unlock(key, cx))
}

fn answered_ok() -> Envelope<manyhands::repository::MutationDataDto> {
    Envelope::mutation(
        "poll once",
        Scope::default(),
        Outcome::Success,
        ResultCode::Ok,
    )
}

pub(super) fn refused(code: ResultCode) -> Envelope<manyhands::repository::MutationDataDto> {
    Envelope::failure("poll once", Scope::default(), code, vec![])
}

/// The Check now requests this session sent, as roots, oldest first.
pub(super) fn checks(world: &World, cx: &mut TestAppContext) -> Vec<String> {
    world.services.operations().read_with(cx, |operations, _| {
        let mut sent: Vec<String> = operations
            .requests()
            .iter()
            .filter_map(|request| match &request.intent {
                Intent::PollOnce { root } => Some(root.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        sent.reverse();
        sent
    })
}

pub(super) fn polls_of(world: &World, root: &str) -> usize {
    world
        .backend
        .polled_roots()
        .iter()
        .filter(|polled| *polled == root)
        .count()
}

/// Waits for a prompt to reach the request model and takes it.
pub(super) async fn prompt(
    world: &World,
    cx: &mut TestAppContext,
) -> crate::desktop::services::prompts::PromptRequest {
    let taken = RefCell::new(None);
    World::settle(cx, "a prompt", |cx| {
        let prompt = world
            .services
            .operations()
            .update(cx, |operations, _| operations.take_prompt());
        *taken.borrow_mut() = prompt;
        taken.borrow().is_some()
    })
    .await;
    taken.into_inner().unwrap()
}

fn has_prompt(world: &World, cx: &mut TestAppContext) -> bool {
    world
        .services
        .operations()
        .update(cx, |operations, _| operations.take_prompt())
        .is_some()
}

/// A locked key met by a poll, for each of `roots`: the poll asks the
/// session as the transport would, and the provider refuses a background
/// job (B5).
pub(super) async fn locked_pair(
    world: &World,
    key: &UnlockRequest,
    roots: &[String],
    cx: &mut TestAppContext,
) {
    world.backend.ask_unlock(key.clone());
    world.list(roots, cx);
    let wanted = roots.to_vec();
    World::settle(cx, "the key to hold its repositories back", |cx| {
        entries(world, cx) == [(Reason::KeyLocked(key.key_id), sorted(wanted.clone()))]
    })
    .await;
}

fn error_of(root: &str, kind: SshTransportErrorKind, key: Option<SharedKeyId>) -> RemotePollError {
    transport_error(root, kind, key, None)
}

/// A3: pause and the unlock block are independent, in both directions.
/// Pausing a held repository leaves its entry; Unlock… sends a Check now and
/// nothing that touches the pause; resuming leaves the block. Through the
/// real service for the pause and the resume.
#[gpui_kit::test]
async fn attention_pause_independent(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    let root = world.ssh_repo("a");
    locked_pair(&world, &key, std::slice::from_ref(&root), cx).await;
    let held = vec![(Reason::KeyLocked(key.key_id), vec![root.clone()])];
    assert!(!paused(&world, &root, cx));

    send(
        &world,
        Intent::PollPause {
            root: root.clone().into(),
        },
    );
    World::settle(cx, "the pause to be read", |cx| paused(&world, &root, cx)).await;
    assert_eq!(entries(&world, cx), held, "pausing leaves the block");

    // Unlock… sends a Check now, and no verb of the pause's.
    assert!(unlock(&world, key.key_id, cx));
    World::settle(cx, "the Check now to be sent", |cx| {
        checks(&world, cx).len() == 1
    })
    .await;
    World::quiet(cx).await;
    assert!(paused(&world, &root, cx), "Unlock… leaves the pause");
    let verbs: Vec<&'static str> = world.services.operations().read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .rev()
            .map(|request| request.intent.label())
            .collect()
    });
    assert_eq!(verbs, ["Pause checking", "Check now"]);

    // Resuming does not clear the block: the repository is still held back.
    send(
        &world,
        Intent::PollResume {
            root: root.clone().into(),
        },
    );
    World::settle(cx, "the resume to be read", |cx| !paused(&world, &root, cx)).await;
    World::quiet(cx).await;
    assert_eq!(entries(&world, cx), held, "resuming leaves the block");
    assert_eq!(
        world.phase(&root, cx),
        Some(Phase::Attention(Reason::KeyLocked(key.key_id)))
    );
}

/// Fact 15, C3: Check now's cancelled prompt marks the key declined, so
/// nothing prompts for it again; Unlock… clears the mark by key (the new
/// production clear) and submits Check now, which prompts.
#[gpui_kit::test]
async fn unlock_after_cancelled_prompt(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    let root = world.ssh_repo("a");
    // A Check now asks as the transport would, through the worker's own
    // provider.
    world
        .backend
        .ask_in_check_now(world.services.worker().provider(), key.clone());

    // The person's own Check now prompts, and the person says Not now.
    send(
        &world,
        Intent::PollOnce {
            root: root.clone().into(),
        },
    );
    prompt(&world, cx).await.answer(PromptReply::Cancelled);
    World::settle(cx, "the first Check now to end", |_| {
        world.backend.asked() == [Asked::Cancelled]
    })
    .await;

    // A poll finds the key locked and does not prompt; so does a second
    // Check now, which the declined mark answers at once.
    locked_pair(&world, &key, std::slice::from_ref(&root), cx).await;
    assert!(!has_prompt(&world, cx), "a poll never prompts");
    send(
        &world,
        Intent::PollOnce {
            root: root.clone().into(),
        },
    );
    World::settle(cx, "the second Check now to ask", |_| {
        world.backend.asked().len() == 2
    })
    .await;
    assert_eq!(world.backend.asked()[1], Asked::Cancelled);
    assert!(!has_prompt(&world, cx), "declined: no second prompt");

    // Unlock… clears the mark and submits Check now, which prompts.
    World::settle(cx, "the worker to be idle", |_| {
        !world.services.worker().in_flight(|_| true)
    })
    .await;
    assert!(unlock(&world, key.key_id, cx));
    let again = prompt(&world, cx).await;
    assert_eq!(again.key_id, key.key_id);
    again.answer(PromptReply::Cancelled);
    World::settle(cx, "the Check now to end", |_| {
        world.backend.asked().len() == 3
    })
    .await;
    assert_eq!(world.backend.asked()[2], Asked::Cancelled);
    assert_eq!(checks(&world, cx).len(), 3, "Unlock… sent the third");
}

/// K-26, C3: an accepted unlock clears the key's attention for every
/// repository using it; the proof is the Check now's own success. A failed
/// unlock, and a repository that cannot be reached, leave it. Nothing is
/// cleared before the answer.
#[gpui_kit::test]
async fn unlock_clears_for_all_repos(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    let (a, b) = (world.ssh_repo("a"), world.ssh_repo("b"));
    locked_pair(&world, &key, &[a.clone(), b.clone()], cx).await;
    let held = vec![(
        Reason::KeyLocked(key.key_id),
        sorted(vec![a.clone(), b.clone()]),
    )];
    let (polls_a, polls_b) = (polls_of(&world, &a), polls_of(&world, &b));

    // A failed unlock and an unreachable repository leave the entry.
    for (n, code) in [ResultCode::UnlockRequired, ResultCode::TransportUnavailable]
        .into_iter()
        .enumerate()
    {
        let gate = world
            .backend
            .inner()
            .hold_job_return_answering(JobKind::PollOnce, refused(code));
        assert!(unlock(&world, key.key_id, cx));
        // The first repository of the entry is the one asked.
        World::settle(cx, "the Check now", |cx| checks(&world, cx).len() == n + 1).await;
        gate.release();
        World::settle(cx, "the Check now to end", |cx| {
            world.services.operations().read_with(cx, |operations, _| {
                operations.requests().iter().all(|request| {
                    matches!(
                        request.state,
                        crate::desktop::services::requests::RequestState::Finished(..)
                    )
                })
            })
        })
        .await;
        World::quiet(cx).await;
        assert_eq!(entries(&world, cx), held, "{code:?} leaves the attention");
        // Nothing moved the repositories: they were not polled again.
        assert_eq!(
            (polls_of(&world, &a), polls_of(&world, &b)),
            (polls_a, polls_b)
        );
        assert_eq!(checks(&world, cx)[n], a.clone().min(b.clone()));
    }

    // An accepted unlock clears it for both, and they poll again at their
    // stored deadlines (none stored: due).
    world.backend.stop_asking_unlock();
    world.backend.answer_poll(world.completed());
    world.backend.answer_poll(world.completed());
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::PollOnce, answered_ok());
    assert!(unlock(&world, key.key_id, cx));
    gate.wait_reached();
    assert_eq!(entries(&world, cx), held, "nothing clears before the proof");
    gate.release();
    World::settle(cx, "the attention to clear", |cx| {
        entries(&world, cx).is_empty()
    })
    .await;
    World::settle(cx, "both to poll again", |_| {
        polls_of(&world, &a) > polls_a && polls_of(&world, &b) > polls_b
    })
    .await;
}

/// R-11, C3: a key or remote change clears the entries it reaches, and a
/// completed Check now clears its own repository's entry however the cause
/// was fixed; the other entries stay.
#[gpui_kit::test]
async fn attention_cleared_by_change(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let key = SharedKeyId::new();
    let roots: Vec<String> = ["a", "b", "c", "d"].map(|name| world.ssh_repo(name)).into();
    let (a, b, c, d) = (&roots[0], &roots[1], &roots[2], &roots[3]);
    // a: no key chosen; b: a host that names none; c and d: the same locked
    // key. The polls go in the roots' order.
    use SshTransportErrorKind as Kind;
    world
        .backend
        .fail_poll(error_of(a, Kind::NoSelectedKey, None));
    world
        .backend
        .fail_poll(error_of(b, Kind::HostVerificationUnavailable, None));
    world
        .backend
        .fail_poll(error_of(c, Kind::UnlockCancelled, Some(key)));
    world
        .backend
        .fail_poll(error_of(d, Kind::UnlockCancelled, Some(key)));
    world.list(&roots, cx);
    let all = vec![
        (Reason::NoKeySelected(a.clone()), vec![a.clone()]),
        (Reason::HostUnidentified(b.clone()), vec![b.clone()]),
        (Reason::KeyLocked(key), sorted(vec![c.clone(), d.clone()])),
    ];
    World::settle(cx, "the three entries", |cx| entries(&world, cx) == all).await;

    // A completed Check now of b clears b's entry, whoever fixed the cause.
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::PollOnce, answered_ok());
    for _ in 0..6 {
        world.backend.answer_poll(world.completed());
    }
    send(
        &world,
        Intent::PollOnce {
            root: b.clone().into(),
        },
    );
    gate.release();
    World::settle(cx, "b's entry to clear", |cx| {
        entries(&world, cx).len() == 2
    })
    .await;
    assert_eq!(entries(&world, cx), [all[0].clone(), all[2].clone()]);

    // A remote change of a reaches a alone; a key change reaches all.
    world.heard(
        vec![Heard::Reconfiguration(
            Moment::Finished,
            Change::Repository {
                root: a.clone(),
                gone: false,
            },
        )],
        cx,
    );
    World::settle(cx, "a's entry to clear", |cx| {
        entries(&world, cx).len() == 1
    })
    .await;
    assert_eq!(entries(&world, cx), [all[2].clone()]);
    world.heard(
        vec![Heard::Reconfiguration(Moment::Finished, Change::Keys)],
        cx,
    );
    World::settle(cx, "the key's entry to clear", |cx| {
        entries(&world, cx).is_empty()
    })
    .await;
}

/// K-23, decision 25: the library's session caches one passphrase, so a
/// background job using key B evicts key A's, and A, refused a prompt,
/// comes back locked: its poll answers the locked-key failure and the
/// entry returns. Through the worker's real provider and a real session.
#[gpui_kit::test]
async fn two_keys_evict(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let (a, b) = (
        unlock_request(directory.path(), SharedKeyId::new()),
        unlock_request(directory.path(), SharedKeyId::new()),
    );
    let root = world.ssh_repo("a");
    let worker = world.services.worker().clone();
    let (asked_a, asked_b) = (a.clone(), b.clone());
    let session = thread::spawn(move || {
        let mut session = SessionCredentials::new(worker.provider());
        // The person unlocks A on the user lane.
        worker.attempting_in_for_testing(Some(DesktopRequestId(901)), Lane::User);
        session
            .with_passphrase(asked_a.clone(), |_| Ok(()))
            .unwrap();
        let cached = session.has_cached_passphrase(&asked_a);
        // A background job using B: refused, and the cache is B's to fill.
        worker.attempting_in_for_testing(Some(DesktopRequestId(902)), Lane::Background);
        let refused_b = session.with_passphrase(asked_b, |_| Ok(()));
        let evicted = !session.has_cached_passphrase(&asked_a);
        // A's next background poll finds it locked.
        let refused_a = session.with_passphrase(asked_a, |_| Ok(())).unwrap_err();
        (cached, refused_b.is_err(), evicted, refused_a)
    });
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    prompt(&world, cx)
        .await
        .answer(PromptReply::Supplied(secret));
    World::settle(cx, "the session thread", |_| session.is_finished()).await;
    let (cached, refused_b, evicted, refused_a) = session.join().unwrap();
    assert!(cached, "A was unlocked");
    assert!(refused_b, "a background job is never asked for B");
    assert!(evicted, "B's job evicted A's passphrase");
    assert_eq!(refused_a, SessionUnlockFailure::ProviderUnavailable);
    assert!(!has_prompt(&world, cx), "no job but the first prompted");

    // The poll that found A locked answers so, and A's entry returns.
    world
        .backend
        .fail_poll(locked(&a, refused_a, Path::new(&root)));
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "A's entry to return", |cx| {
        entries(&world, cx) == [(Reason::KeyLocked(a.key_id), vec![root.clone()])]
    })
    .await;
}
