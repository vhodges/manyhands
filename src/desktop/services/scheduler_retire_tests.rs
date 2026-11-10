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
    model::polling::{Phase, Reason, TargetKind, retire::Verdict},
    services::{
        JobKind,
        inflight::{Acquired, InflightStore, RetirementEnd, RetirementEntry},
        requests::{DesktopRequestId, Intent, Lane, RequestState},
        worker::{CancelResult, RecordState},
    },
    test_support::{
        polling::{Asked, unlock_request},
        retire::{ITEM, OPERATION, answer},
    },
};

fn item() -> ItemId {
    ITEM.parse().unwrap()
}

/// Sends a retirement of the item of `root`, as the loop or Retire… does.
fn retire(world: &World, root: &str, lane: Lane, cx: &mut TestAppContext) -> DesktopRequestId {
    let root = root.to_owned();
    world.services.scheduler().update(cx, |scheduler, _| {
        scheduler
            .retire(&root, TargetKind::Ticket, &item(), lane)
            .expect("not quitting")
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

    // Never sent: this one is Owed, so the library holds it pending and the
    // entry stays (review I-3, E-29: this used to remove it); a request that
    // never reached the library is `a_removed_resend_keeps_a_pending_entry`'s.
    let gone = |end| {
        vec![Heard::Retirement(Retiring::Gone {
            id: one,
            request: one_request,
            end,
        })]
    };
    world.heard(gone(RetirementEnd::NotSent), cx);
    assert_eq!(entered(&world, cx), [one_request], "kept on NotSent");
    world.heard(gone(RetirementEnd::Finished), cx);
    assert!(entered(&world, cx).is_empty(), "removed on Finished");
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

/// A20, R-31 through the real worker and drain: a request whose record is
/// left `Accepted` is Owed whatever it was answered. The answer is a retry
/// code, which a finished record dismisses and retries next attempt, and the
/// record is forced `Accepted` (a canned answer cannot leave a real one open).
/// The request stays and counts, its entry stays for Resume, and the item is
/// no refused-set entry.
#[gpui_kit::test]
async fn retire_left_accepted_is_owed(cx: &mut TestAppContext) {
    let (world, root) = polled(cx).await;
    let retry = answer(State::Refused, Code::IndexUnavailable, Removal::None);
    world
        .services
        .worker()
        .force_record_for_testing(Some(RecordState::Accepted));
    let id = answered(&world, &root, Lane::Background, retry, cx).await;
    world.services.operations().read_with(cx, |operations, _| {
        let request = operations.request(id).expect("the request");
        assert!(matches!(request.state, RequestState::Finished(..)));
        let answer = operations.retirement(id).expect("sorted");
        assert!(matches!(answer.verdict, Verdict::Owed { .. }), "{answer:?}");
    });
    assert_eq!(entered(&world, cx).len(), 1, "kept for Resume");
    world.services.scheduler().read_with(cx, |scheduler, _| {
        assert!(!scheduler.schedule().retirement_refused(&root, &item()));
    });
    // A finished record is a retry: dismissed, entry ended.
    world
        .services
        .worker()
        .force_record_for_testing(Some(RecordState::Finished));
    let again = answer(State::Refused, Code::IndexUnavailable, Removal::None);
    let id = answered(&world, &root, Lane::Background, again, cx).await;
    world.services.operations().read_with(cx, |operations, _| {
        let request = operations.request(id).expect("the request");
        assert!(matches!(request.state, RequestState::Dismissed));
    });
    assert_eq!(entered(&world, cx).len(), 1, "only the first remains");
}

/// E3, R-31: the drain tells the request model where the worker's record of
/// a retirement stood before it applies `Finished`, which carries none. A
/// canned answer cannot leave a real record Accepted (a cancel before the
/// claim is final, and the claim needs the transport), so the worker's
/// answer is a stub: `Accepted` makes even a retry code Owed, which the
/// record state first reading (A20) is for.
#[test]
fn retire_record_state_reaches_the_answer() {
    use crate::desktop::{
        model::{operations::Operations, polling::retire::Verdict},
        services::{
            requests::{DesktopRequest, Intent},
            scheduler::note_record,
            worker::{RecordState, WorkerEvent},
        },
    };
    let refused = || answer(State::Refused, Code::IndexUnavailable, Removal::None);
    let intents = |retiring: bool| {
        if retiring {
            Intent::ContextRetire {
                root: "/repo".into(),
                kind: manyhands::repository::AuthoringKind::Ticket,
                id: item(),
            }
        } else {
            Intent::IndexRefresh {
                root: "/repo".into(),
            }
        }
    };
    let cases = [
        ("accepted", true, Some(RecordState::Accepted), true),
        ("finished", true, Some(RecordState::Finished), false),
        ("unknown", true, Some(RecordState::Unknown), true),
        // The worker never took the request: it is finished.
        ("none", true, None, false),
        // Only a retirement's answer is sorted by its record.
        ("another request", false, Some(RecordState::Accepted), false),
    ];
    for (what, retiring, record, owed) in cases {
        let mut operations = Operations::default();
        let id = DesktopRequestId(1);
        operations.apply(WorkerEvent::Queued(DesktopRequest {
            id,
            request_id: RequestId::new(),
            intent: intents(retiring),
            confirmation: None,
            lane: Lane::Background,
            attempt: 1,
            state: RequestState::Queued,
        }));
        let event = WorkerEvent::Finished(id, Box::new(refused()));
        note_record(&event, &mut operations, |asked| {
            assert_eq!(asked, id);
            record
        });
        operations.apply(event);
        let seen = operations.retirement(id).map(|answer| answer.verdict);
        if retiring {
            let owed_now = matches!(seen, Some(Verdict::Owed { .. }));
            assert_eq!(owed_now, owed, "{what}: {seen:?}");
        } else {
            assert_eq!(seen, None, "{what}");
        }
    }
}

/// Seeds the pending row of `OPERATION` in `root` into the request model, as
/// a read of the repository's pending operations would list it.
fn list_pending(world: &World, root: &str, cx: &mut TestAppContext) {
    use crate::desktop::{services::ReadResult, test_support::retire::pending_row};
    world.services.operations().update(cx, |operations, cx| {
        let stamp = operations.load_pending(root.to_owned());
        operations.apply_pending(
            stamp,
            ReadResult::Ok(manyhands::repository::OperationListDto {
                items: vec![pending_row(OPERATION, "created", None)],
                complete: true,
            }),
        );
        cx.notify();
    });
}

/// The retirements sent, as `(lane, library request ID, desktop ID)`,
/// oldest first.
fn retirements_sent(
    world: &World,
    cx: &mut TestAppContext,
) -> Vec<(Lane, RequestId, DesktopRequestId)> {
    world.services.operations().read_with(cx, |operations, _| {
        let mut sent: Vec<_> = operations
            .requests()
            .iter()
            .filter(|request| matches!(request.intent, Intent::ContextRetire { .. }))
            .map(|request| (request.lane, request.request_id, request.id))
            .collect();
        sent.reverse();
        sent
    })
}

/// Fact 26, A20, R-31, E4: a retirement left pending offers Resume (its
/// entry names the operation) and Abandon. Resume first clears the declined
/// keys of the request's last attempt, so the resend can prompt; the
/// library answers `original_request_required`, and the desktop rebuilds
/// `Intent::ContextRetire` from the entry under the request ID the answer
/// names and sends it as a new user-lane request. The finished resend ends
/// the entry. A resend taken from the queue before it ran leaves the entry
/// (review I-3). An operation no entry names is not resent as a retirement.
#[gpui_kit::test]
async fn retire_resume_resends_original(cx: &mut TestAppContext) {
    use crate::desktop::{
        model::operations::{ResumeOffer, abandon_offered, resume_offer_among},
        services::prompts::PromptReply,
        test_support::retire::original_request,
    };
    let (world, root) = polled(cx).await;
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    world
        .backend
        .ask_in_check_now(world.services.worker().provider(), key.clone());

    // The person's own retirement: the prompt is answered Not now (the key
    // is declined), and the library has the retirement pending.
    let owed = answer(State::Pending, Code::UnlockRequired, Removal::None);
    world
        .backend
        .inner()
        .answer_job(JobKind::ContextRetire, owed);
    let first = retire(&world, &root, Lane::User, cx);
    super::attention_tests::prompt(&world, cx)
        .await
        .answer(PromptReply::Cancelled);
    ended(&world, first, cx).await;
    let original = entered(&world, cx);
    assert_eq!(original.len(), 1, "Owed keeps its entry");
    let operation = OperationId::parse(OPERATION).unwrap();
    list_pending(&world, &root, cx);

    // Resume and Abandon are both offered.
    let row = world.services.operations().read_with(cx, |operations, _| {
        let own = operations.automatic_polls().clone();
        let rows = operations.pending_shown();
        let row = rows
            .iter()
            .find_map(|(shown, state)| (*shown == root).then_some(state.rows()?.items[0].clone()))
            .expect("the pending row");
        assert_eq!(
            resume_offer_among(&row, &own, &operations.entered_operations()),
            ResumeOffer::Offered
        );
        row
    });
    assert!(abandon_offered(&row));

    // Resume: the declined key is cleared, and the resume is answered
    // `original_request_required` naming the entry's request.
    world
        .backend
        .inner()
        .answer_job(JobKind::OperationResume, original_request(original[0]));
    world.backend.inner().answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, Code::Ok, Removal::Observed),
    );
    cx.update(|cx| world.services.resume(&root, &operation.to_string(), cx));
    // The resend asks for the key again: cleared, it prompts, not declined.
    super::attention_tests::prompt(&world, cx)
        .await
        .answer(PromptReply::Cancelled);
    World::settle(cx, "the resend", |cx| {
        retirements_sent(&world, cx).len() == 2
    })
    .await;
    World::settle(cx, "the resend to end", |cx| entered(&world, cx).is_empty()).await;
    let sent = retirements_sent(&world, cx);
    assert_eq!(sent[1].0, Lane::User);
    assert_eq!(sent[1].1, original[0], "under the named request ID");
    assert_ne!(sent[1].2, sent[0].2, "a new desktop request");
    world.services.operations().read_with(cx, |operations, _| {
        match &operations.request(sent[1].2).expect("the resend").intent {
            Intent::ContextRetire {
                root: sent,
                kind,
                id,
            } => {
                assert_eq!(sent.to_str(), Some(root.as_str()));
                assert_eq!(*kind, manyhands::repository::AuthoringKind::Ticket);
                assert_eq!(*id, item());
            }
            _ => panic!("the resend is not a retirement"),
        }
    });
}

/// C3, E4: Unlock… picks the first of the key's repositories whose next step
/// reaches authentication. A pending retirement in the in-flight record comes
/// first, since a poll there would answer `recovery_required` before it
/// authenticates (Fact 32): it is resent from the record, on the user lane,
/// under its own request ID; the retirement that got past its observation
/// clears the key's attention for every repository. With none, the first
/// repository with no pending local operation and no hold gets a Check now.
/// With neither, Unlock… is disabled.
#[gpui_kit::test]
async fn unlock_picks_reachable(cx: &mut TestAppContext) {
    use crate::desktop::services::scheduler::attention::UnlockTarget;
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    let (a, b) = (world.ssh_repo("a"), world.ssh_repo("b"));
    // The previous launch left a retirement of `b` pending.
    let request = RequestId::new();
    let operation = OperationId::new();
    {
        let mut previous = InflightStore::open(world.fixture.data_dir());
        assert_eq!(previous.try_acquire(), Acquired::Now);
        previous.retirement_submitting(RetirementEntry::new(
            request,
            &b,
            TargetKind::Ticket,
            &item(),
        ));
        previous.retirement_operation(request, operation);
        previous.retirement_ended(request, RetirementEnd::Accepted);
    }
    super::attention_tests::locked_pair(&world, &key, &[a.clone(), b.clone()], cx).await;
    let held = vec![(Reason::KeyLocked(key.key_id), {
        let mut roots = vec![a.clone(), b.clone()];
        roots.sort();
        roots
    })];
    let target = |world: &World, cx: &mut TestAppContext| {
        world.services.scheduler().read_with(cx, |scheduler, cx| {
            world.services.operations().read_with(cx, |operations, _| {
                scheduler.unlock_target(key.key_id, operations)
            })
        })
    };

    // The retirement is picked, though `a` is reachable and first.
    assert!(a < b);
    assert_eq!(
        target(&world, cx),
        Some(UnlockTarget::Retirement {
            root: b.clone(),
            request,
            kind: TargetKind::Ticket,
            item: item(),
        })
    );
    // Unlock… resends it on the user lane under its own request ID; it got
    // past its observation, so the key's attention clears everywhere.
    world.backend.stop_asking_unlock();
    world.backend.inner().answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, Code::Ok, Removal::Observed),
    );
    assert_eq!(entries(&world, cx), held);
    assert!(super::attention_tests::unlock(&world, key.key_id, cx));
    World::settle(cx, "the attention to clear", |cx| {
        entries(&world, cx).is_empty()
    })
    .await;
    World::quiet(cx).await;
    let sent = retirements_sent(&world, cx);
    assert_eq!(sent.len(), 1);
    assert_eq!((sent[0].0, sent[0].1), (Lane::User, request));
    assert!(
        super::attention_tests::checks(&world, cx).is_empty(),
        "no Check now was needed"
    );
    assert!(entered(&world, cx).is_empty(), "it finished");

    // No retirement left: the first repository with no pending local
    // operation and no hold gets a Check now.
    for root in [&a, &b] {
        world.services.scheduler().update(cx, |scheduler, cx| {
            scheduler
                .schedule_mut()
                .raise(root, Reason::KeyLocked(key.key_id));
            cx.notify();
        });
    }
    assert_eq!(entries(&world, cx), held);
    assert_eq!(target(&world, cx), Some(UnlockTarget::CheckNow(a.clone())));
    list_pending(&world, &a, cx);
    assert_eq!(target(&world, cx), Some(UnlockTarget::CheckNow(b.clone())));

    // Neither: both wait on a pending operation. Disabled, and nothing sent.
    list_pending(&world, &b, cx);
    assert_eq!(target(&world, cx), None);
    assert!(!super::attention_tests::unlock(&world, key.key_id, cx));
    assert!(super::attention_tests::checks(&world, cx).is_empty());
    assert_eq!(retirements_sent(&world, cx).len(), 1);
}

/// E4 step 1: Retire… checks RT-1 again where it submits. A retirement of the
/// item that the in-flight record holds pending, or the worker holds, sends
/// nothing; one that is not held is sent on the user lane, and its item
/// leaves the refused set (A19). Nothing is sent once quitting is asked.
#[gpui_kit::test]
async fn retire_chosen_checks_rt1_again(cx: &mut TestAppContext) {
    let (world, root) = polled(cx).await;
    let other: ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FG9".parse().unwrap();
    let choose = |world: &World, item: &ItemId, cx: &mut TestAppContext| {
        let (root, item) = (root.clone(), item.clone());
        world.services.scheduler().update(cx, |scheduler, cx| {
            scheduler.retire_chosen(&root, TargetKind::Ticket, &item, cx)
        })
    };
    // Held pending by the record: nothing.
    world.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.hold_retirement(
            RequestId::new(),
            &root,
            TargetKind::Ticket,
            &item(),
            OperationId::new(),
            cx,
        )
    });
    assert_eq!(choose(&world, &item(), cx), None);
    assert!(retirements_sent(&world, cx).is_empty());

    // Not held: sent, and the refusal is forgotten.
    world.services.scheduler().update(cx, |scheduler, _| {
        scheduler.schedule_mut().retirement_answered(
            &root,
            TargetKind::Ticket,
            &other,
            &Verdict::Refused,
        );
        assert!(scheduler.schedule().retirement_refused(&root, &other));
    });
    world.backend.inner().answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, Code::Ok, Removal::Observed),
    );
    let sent = choose(&world, &other, cx).expect("sent");
    ended(&world, sent, cx).await;
    assert_eq!(retirements_sent(&world, cx)[0].0, Lane::User);
    world.services.scheduler().read_with(cx, |scheduler, _| {
        assert!(!scheduler.schedule().retirement_refused(&root, &other));
    });

    // Quitting: nothing.
    world
        .services
        .scheduler()
        .update(cx, |scheduler, cx| scheduler.set_quitting(true, cx));
    assert_eq!(choose(&world, &other, cx), None);
}
