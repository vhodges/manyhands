//! A Check now's answer and the Check now an action arms (D6 Task 8 review
//! fixes, design C3, A4): what a failed Check now becomes, and when the one
//! an action armed is sent. Same world as `attention_tests`.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        MutationDataDto, SharedKeyId, SynchronizationMutationDto, keys::SecretPassphrase,
    },
    results::{Envelope, ResultCode, Scope},
};

use super::{
    attention_tests::{checks, entries, locked_pair, prompt, refused, send, unlock},
    reconfigure::{Heard, Moment, Scope as Change},
    support::World,
};
use crate::desktop::{
    model::polling::{Authority, Phase, Reason},
    services::{JobKind, prompts::PromptReply, requests::Intent},
    test_support::polling::unlock_request,
};

fn host_refused(authority: &Authority) -> Envelope<MutationDataDto> {
    Envelope::failure(
        "poll once",
        Scope::default(),
        ResultCode::HostApprovalRequired,
        vec![],
    )
    .with_data(MutationDataDto::Synchronization(
        SynchronizationMutationDto {
            host: authority.host.clone(),
            port: authority.port,
            presented_algorithm: "ssh-ed25519".to_owned(),
            presented_fingerprint: "SHA256:x".to_owned(),
            pinned_algorithm: None,
            pinned_fingerprint: None,
        },
    ))
}

/// Sends a Check now of `root` answered `answer` after its work, and waits
/// for the request to end.
async fn checked_with(
    world: &World,
    root: &str,
    answer: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) {
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::PollOnce, answer);
    let before = checks(world, cx).len();
    send(world, Intent::PollOnce { root: root.into() });
    World::settle(cx, "the Check now to be sent", |cx| {
        checks(world, cx).len() == before + 1
    })
    .await;
    gate.release();
    World::settle(cx, "the Check now to end", |cx| {
        !world.services.worker().in_flight(|_| true)
            && world.services.operations().read_with(cx, |operations, _| {
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
}

/// C3, A4: a Check now that fails is an answer like a poll's, and becomes
/// the same attention. A plain unreachable one is only a backoff: no entry,
/// the repository `Idle`. A cancelled prompt holds the key as a locked-key
/// entry. A host answer after the key was supplied replaces that entry, for
/// the repository, with the host's.
#[gpui_kit::test]
async fn check_now_failure_becomes_attention(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    let (a, b) = (world.ssh_repo("a"), world.ssh_repo("b"));
    for _ in 0..2 {
        world.backend.answer_poll(world.completed());
    }
    world.list(&[a.clone(), b.clone()], cx);
    World::settle(cx, "both to be polled", |_| {
        world.backend.polled_roots().len() == 2
    })
    .await;
    World::quiet(cx).await;
    assert!(entries(&world, cx).is_empty());

    // Unreachable: a backoff, not an entry.
    checked_with(&world, &b, refused(ResultCode::TransportUnavailable), cx).await;
    assert!(entries(&world, cx).is_empty());
    assert_eq!(world.phase(&b, cx), Some(Phase::Idle));

    // The person says Not now to Check now's prompt: the key is declined
    // and the answer holds the repository for it.
    world
        .backend
        .ask_in_check_now(world.services.worker().provider(), key.clone());
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::PollOnce, refused(ResultCode::UnlockRequired));
    send(
        &world,
        Intent::PollOnce {
            root: a.clone().into(),
        },
    );
    prompt(&world, cx).await.answer(PromptReply::Cancelled);
    gate.release();
    World::settle(cx, "the key's entry", |cx| {
        entries(&world, cx) == [(Reason::KeyLocked(key.key_id), vec![a.clone()])]
    })
    .await;

    // Unlock… supplies the key, and the host is the next thing: its entry
    // replaces the key's for the repository.
    World::settle(cx, "the worker to be idle", |_| {
        !world.services.worker().in_flight(|_| true)
    })
    .await;
    let host = Authority {
        host: "example.invalid".to_owned(),
        port: 22,
    };
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::PollOnce, host_refused(&host));
    assert!(unlock(&world, key.key_id, cx));
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    prompt(&world, cx)
        .await
        .answer(PromptReply::Supplied(secret));
    gate.release();
    World::settle(cx, "the host's entry", |cx| {
        entries(&world, cx) == [(Reason::HostUnapproved(host.clone()), vec![a.clone()])]
    })
    .await;
}

/// C3: the Check now an action arms follows a change that went through, once,
/// for the repository it was armed for. A change that failed sends nothing
/// and drops the arm with the entry it cleared; a repository removed and
/// created again does not inherit an old one.
#[gpui_kit::test]
async fn awaited_check_only_after_success(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    let root = world.ssh_repo("a");
    let held = [(Reason::KeyLocked(key.key_id), vec![root.clone()])];
    locked_pair(&world, &key, std::slice::from_ref(&root), cx).await;
    let arm = |world: &World, cx: &mut TestAppContext| {
        world.services.scheduler().update(cx, |scheduler, _| {
            scheduler.check_once_changed(root.clone())
        });
    };
    let change = Change::Repository {
        root: root.clone(),
        gone: false,
    };
    let hear = |world: &World, moment, cx: &mut TestAppContext| {
        world.heard(vec![Heard::Reconfiguration(moment, change.clone())], cx);
    };
    // The key is locked again after each change: the poll it clears finds it.
    let held_again = |cx: &mut TestAppContext| entries(&world, cx) == held;

    // A change that failed sends nothing, and the arm goes with the entry.
    arm(&world, cx);
    hear(&world, Moment::Finished, cx);
    World::settle(cx, "the key to hold it again", |cx| held_again(cx)).await;
    World::quiet(cx).await;
    assert!(
        checks(&world, cx).is_empty(),
        "a failed change sends nothing"
    );
    hear(&world, Moment::Succeeded, cx);
    World::settle(cx, "the key to hold it again", |cx| held_again(cx)).await;
    World::quiet(cx).await;
    assert!(checks(&world, cx).is_empty(), "the arm went with the entry");

    // A change that went through sends the one Check now.
    arm(&world, cx);
    hear(&world, Moment::Succeeded, cx);
    World::settle(cx, "the Check now", |cx| checks(&world, cx).len() == 1).await;
    World::settle(cx, "the key to hold it again", |cx| held_again(cx)).await;
    World::quiet(cx).await;
    assert_eq!(checks(&world, cx), std::slice::from_ref(&root));

    // A repository removed and created again starts with no arm.
    arm(&world, cx);
    world.list(&[], cx);
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the key to hold it again", |cx| held_again(cx)).await;
    hear(&world, Moment::Succeeded, cx);
    World::quiet(cx).await;
    assert_eq!(checks(&world, cx).len(), 1, "no Check now for the old arm");
}
