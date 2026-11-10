//! A retirement at the scheduler (D6 Task 11a, design E2, E3, R-31, B5):
//! its in-flight entry, and what its answer becomes. The library's answers
//! are canned (decision 9), since no desktop test reaches the SSH transport
//! a real retirement observes through.

use std::fs;

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    repository::{
        ContextRetirementState as State, OperationId, RequestId, RetirementRemoval as Removal,
        SharedKeyId,
    },
    results::ResultCode as Code,
};

use super::{
    attention_tests::entries,
    begin_retirement,
    reconfigure::{Heard, Retiring},
    support::World,
};
use crate::desktop::{
    model::polling::{Phase, Reason, TargetKind},
    services::{
        JobKind,
        inflight::{Acquired, InflightStore, RetirementEnd},
        requests::{DesktopRequestId, Lane, RequestState},
        worker::CancelResult,
    },
    test_support::{
        polling::{Asked, unlock_request},
        retire::{ITEM, answer},
    },
};

fn item() -> ItemId {
    ITEM.parse().unwrap()
}

/// Sends a retirement of the item of `root`, as the loop or Retire… does.
fn retire(world: &World, root: &str, lane: Lane, cx: &mut TestAppContext) -> DesktopRequestId {
    let root = root.to_owned();
    world.services.scheduler().update(cx, |scheduler, _| {
        scheduler.retire(&root, TargetKind::Ticket, &item(), lane)
    })
}

/// The request IDs of the entries the in-flight record holds, oldest first.
fn entered(world: &World, cx: &mut TestAppContext) -> Vec<RequestId> {
    world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler
            .store()
            .retirements()
            .iter()
            .filter_map(|entry| entry.request())
            .collect()
    })
}

/// The library's request ID of the request `id`, once the model has heard
/// it queued.
async fn request_id(world: &World, id: DesktopRequestId, cx: &mut TestAppContext) -> RequestId {
    World::settle(cx, "the request to be queued", |cx| {
        world
            .services
            .operations()
            .read_with(cx, |operations, _| operations.request(id).is_some())
    })
    .await;
    world.services.operations().read_with(cx, |operations, _| {
        operations.request(id).expect("the request").request_id
    })
}

/// Waits until the request `id` has ended, whatever became of it.
async fn ended(world: &World, id: DesktopRequestId, cx: &mut TestAppContext) {
    World::settle(cx, "the retirement to end", |cx| {
        world.services.operations().read_with(cx, |operations, _| {
            operations.request(id).is_some_and(|request| {
                !matches!(
                    request.state,
                    RequestState::Queued | RequestState::Running { .. }
                )
            })
        })
    })
    .await;
    World::quiet(cx).await;
}

/// A repository `a` that has had its launch poll.
async fn polled(cx: &mut TestAppContext) -> (World, String) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world.backend.answer_poll(world.completed());
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the launch poll", |_| {
        world.backend.polled_roots().len() == 1
    })
    .await;
    World::quiet(cx).await;
    (world, root)
}

/// R-31: a retirement request's entry (request ID, root, kind, item and
/// the operation of its first progress event) is written before the request
/// is submitted, kept while the request is Owed, and removed when it
/// finishes, is removed from the queue, or was never sent.
#[gpui_kit::test]
async fn retire_inflight_entry(cx: &mut TestAppContext) {
    // Written before it is submitted: the file holds it when `submit` runs.
    let data = tempfile::tempdir().unwrap();
    let mut store = InflightStore::open(data.path());
    assert_eq!(store.try_acquire(), Acquired::Now);
    let record = data.path().join("inflight").join("record.json");
    let first = RequestId::new();
    let sent = begin_retirement(
        &mut store,
        first,
        &"/repo".to_owned(),
        TargetKind::Ticket,
        &item(),
        || {
            let written = fs::read_to_string(&record).unwrap();
            assert!(
                written.contains(&first.to_string()),
                "the entry was not written before the submit"
            );
            DesktopRequestId(1)
        },
    );
    assert_eq!(sent, DesktopRequestId(1));
    // A resend of the same request keeps the entry it has.
    let operation = OperationId::new();
    store.retirement_operation(first, operation);
    begin_retirement(
        &mut store,
        first,
        &"/repo".to_owned(),
        TargetKind::Ticket,
        &item(),
        || DesktopRequestId(2),
    );
    assert_eq!(store.retirements().len(), 1);
    assert_eq!(store.retirements()[0].operation(), Some(operation));

    // Through the worker.
    let (world, root) = polled(cx).await;
    let pending = answer(State::Pending, Code::RecoveryRequired, Removal::None);
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::ContextRetire, pending);
    let one = retire(&world, &root, Lane::Background, cx);
    gate.wait_reached();
    let one_request = request_id(&world, one, cx).await;
    assert_eq!(entered(&world, cx), [one_request], "written when sent");
    World::settle(cx, "the first progress event", |cx| {
        world.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler.store().retirements()[0].operation().is_some()
        })
    })
    .await;
    let learned = world.services.scheduler().read_with(cx, |scheduler, _| {
        let entry = &scheduler.store().retirements()[0];
        assert_eq!(entry.root(), &root);
        assert_eq!(entry.item(), Some((TargetKind::Ticket, item())));
        entry.operation()
    });
    // Only the first progress event names it: a later one changes nothing.
    world.heard(
        vec![Heard::Retirement(Retiring::Operation {
            request: one_request,
            operation: OperationId::new(),
        })],
        cx,
    );
    world.services.scheduler().read_with(cx, |scheduler, _| {
        assert_eq!(scheduler.store().retirements()[0].operation(), learned);
    });
    // Owed: pending keeps the entry, for Resume.
    gate.release();
    ended(&world, one, cx).await;
    assert_eq!(entered(&world, cx), [one_request], "kept while Owed");

    // Finished: a final answer removes its own entry and leaves the other.
    let refused = answer(State::Refused, Code::WorktreeNotClean, Removal::None);
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::ContextRetire, refused);
    let two = retire(&world, &root, Lane::Background, cx);
    gate.wait_reached();
    assert_eq!(entered(&world, cx).len(), 2);
    gate.release();
    ended(&world, two, cx).await;
    assert_eq!(entered(&world, cx), [one_request], "removed on Finished");

    // Removed from the queue before it ran.
    let running = world.backend.inner().hold_job(JobKind::ContextRetire);
    let three = retire(&world, &root, Lane::Background, cx);
    running.wait_reached();
    let four = retire(&world, &root, Lane::Background, cx);
    assert_eq!(entered(&world, cx).len(), 3);
    assert_eq!(world.services.worker().cancel(four), CancelResult::Removed);
    World::settle(cx, "the removal to be heard", |cx| {
        entered(&world, cx).len() == 2
    })
    .await;
    running.release();
    ended(&world, three, cx).await;
    assert_eq!(entered(&world, cx), [one_request], "removed on Removed");

    // Never sent.
    world.heard(
        vec![Heard::Retirement(Retiring::Gone {
            request: one_request,
            end: RetirementEnd::NotSent,
        })],
        cx,
    );
    assert!(entered(&world, cx).is_empty(), "removed on NotSent");
}

/// B5, A13, U-12: a background retirement through the real provider, its
/// key locked, never prompts. The key is refused a prompt and not declined
/// (Not now was not the person's answer), the library's `unlock_required`
/// is the answer, and it becomes the key's attention: the request is
/// dismissed, the item is not in the refused set, and the repository waits
/// for the person.
#[gpui_kit::test]
async fn background_retire_never_prompts(cx: &mut TestAppContext) {
    let (world, root) = polled(cx).await;
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    world
        .backend
        .ask_in_check_now(world.services.worker().provider(), key.clone());
    let locked = answer(State::Refused, Code::UnlockRequired, Removal::None);
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::ContextRetire, locked);
    let id = retire(&world, &root, Lane::Background, cx);
    gate.wait_reached();
    gate.release();
    ended(&world, id, cx).await;

    // The session refused the passphrase request at once.
    assert_eq!(world.backend.asked(), [Asked::Unavailable]);
    assert!(
        !world
            .services
            .operations()
            .update(cx, |operations, _| operations.has_prompt()),
        "a background retirement prompted"
    );
    assert!(!world.services.worker().clear_declined(key.key_id));
    // The answer is the key's attention, not the item's.
    assert_eq!(
        entries(&world, cx),
        [(Reason::KeyLocked(key.key_id), vec![root.clone()])]
    );
    world.services.operations().read_with(cx, |operations, _| {
        let request = operations.request(id).expect("the request");
        assert!(matches!(request.state, RequestState::Dismissed));
    });
    world.services.scheduler().read_with(cx, |scheduler, _| {
        assert!(!scheduler.schedule().retirement_refused(&root, &item()));
        // Attention parks the repository: no poll until the person acts.
        assert!(matches!(
            scheduler.schedule().repo(&root).map(|repo| &repo.phase),
            Some(Phase::Attention(_))
        ));
    });
}

/// Sends a retirement that the canned `answer` ends, and waits for it.
async fn answered(
    world: &World,
    root: &str,
    lane: Lane,
    answer: manyhands::results::Envelope<manyhands::repository::MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::ContextRetire, answer);
    let id = retire(world, root, lane, cx);
    gate.wait_reached();
    gate.release();
    ended(world, id, cx).await;
    id
}

/// The person acts on `reason`: the repository is due again, and its poll,
/// answered as completed, parks it until its next interval.
async fn cleared(world: &World, reason: Reason, cx: &mut TestAppContext) {
    world.backend.answer_poll(world.completed());
    let before = world.backend.polled_roots().len();
    world
        .services
        .scheduler()
        .update(cx, |scheduler, cx| scheduler.clear_attention(&reason, cx));
    World::settle(cx, "the poll that follows", |_| {
        world.backend.polled_roots().len() > before
    })
    .await;
    World::quiet(cx).await;
}

/// E3: what a retirement's answer becomes. A locked key is keyed by the key
/// a user-lane request asked for, else the repository's selected key, else
/// the repository is asked to choose one; a host answer by the host the
/// repository is reached at; both are attention, not the item's, so the item
/// is not in the refused set. A refusal of the item's own joins the refused
/// set, and shows no attention, until Retire… removes it (R-21, A19). A
/// retirement that retired clears the repository's attention (C3).
#[gpui_kit::test]
async fn retire_answers_become_attention(cx: &mut TestAppContext) {
    let (world, root) = polled(cx).await;
    // No prompt was asked, no key is recorded: the selected key. (With none
    // selected the repository is the entry: `retire_answer_table`.)
    let locked = || answer(State::Refused, Code::UnlockRequired, Removal::None);
    let selected = SharedKeyId::new();
    world.backend.select_key(Some(selected));
    answered(&world, &root, Lane::User, locked(), cx).await;
    World::quiet(cx).await;
    assert_eq!(
        entries(&world, cx),
        [(Reason::KeyLocked(selected), vec![root.clone()])]
    );
    cleared(&world, Reason::KeyLocked(selected), cx).await;
    // A host answer: the host the repository is reached at.
    let host = world
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| scheduler.authorities[&root].clone())
        .expect("the remote names its host");
    let unapproved = answer(State::Refused, Code::HostApprovalRequired, Removal::None);
    answered(&world, &root, Lane::Background, unapproved, cx).await;
    World::quiet(cx).await;
    assert_eq!(
        entries(&world, cx),
        [(Reason::HostUnapproved(host.clone()), vec![root.clone()])]
    );
    cleared(&world, Reason::HostUnapproved(host), cx).await;
    world.services.scheduler().read_with(cx, |scheduler, _| {
        assert!(!scheduler.schedule().retirement_refused(&root, &item()));
    });

    // The item's own refusal: the refused set, no attention, a row that
    // stays, until Retire… removes the item.
    let refused = answer(State::Refused, Code::WorktreeNotClean, Removal::None);
    let id = answered(&world, &root, Lane::Background, refused, cx).await;
    assert!(entries(&world, cx).is_empty());
    world.services.operations().read_with(cx, |operations, _| {
        let request = operations.request(id).expect("the request");
        assert!(matches!(request.state, RequestState::Finished(..)));
    });
    let refused = |world: &World, cx: &mut TestAppContext| {
        world.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler.schedule().retirement_refused(&root, &item())
        })
    };
    assert!(refused(&world, cx));
    world.services.scheduler().update(cx, |scheduler, _| {
        scheduler.schedule_mut().retire_chosen(&root, &item())
    });
    assert!(!refused(&world, cx));

    // Not now to a user-lane prompt: the key it asked for.
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    world
        .backend
        .ask_in_check_now(world.services.worker().provider(), key.clone());
    let locked = answer(State::Refused, Code::UnlockRequired, Removal::None);
    let gate = world
        .backend
        .inner()
        .hold_job_return_answering(JobKind::ContextRetire, locked);
    let id = retire(&world, &root, Lane::User, cx);
    super::attention_tests::prompt(&world, cx)
        .await
        .answer(crate::desktop::services::prompts::PromptReply::Cancelled);
    gate.wait_reached();
    gate.release();
    ended(&world, id, cx).await;
    assert_eq!(world.backend.asked(), [Asked::Cancelled]);
    assert_eq!(
        entries(&world, cx),
        [(Reason::KeyLocked(key.key_id), vec![root.clone()])]
    );
    // Dismissed, though the person sent it.
    world.services.operations().read_with(cx, |operations, _| {
        let request = operations.request(id).expect("the request");
        assert!(matches!(request.state, RequestState::Dismissed));
    });

    // An answer that got past authentication proves the key: its entry
    // goes, and the repository is polled again (C3).
    world.backend.answer_poll(world.completed());
    let before = world.backend.polled_roots().len();
    let retired = answer(State::Retired, Code::Ok, Removal::Observed);
    answered(&world, &root, Lane::Background, retired, cx).await;
    World::settle(cx, "the poll that follows", |_| {
        world.backend.polled_roots().len() > before
    })
    .await;
    assert!(entries(&world, cx).is_empty(), "the key's entry stays");
}
