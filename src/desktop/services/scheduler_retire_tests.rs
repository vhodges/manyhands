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
        retire::{ITEM, OPERATION, answer, empty_list, pending_list},
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

/// Lets the world run, answering Not now to any prompt, until `done` holds
/// or a bounded time has passed (a failure then shows as the caller's
/// assertion, not a long wait).
async fn answering_prompts(
    world: &World,
    cx: &mut TestAppContext,
    done: impl Fn(&mut TestAppContext) -> bool,
) {
    use crate::desktop::services::prompts::PromptReply;
    for _ in 0..600 {
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        let prompt = world
            .services
            .operations()
            .update(cx, |operations, _| operations.take_prompt());
        if let Some(prompt) = prompt {
            prompt.answer(PromptReply::Cancelled);
        }
        cx.executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    assert!(done(cx), "the world did not settle");
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
    assert!(
        world.services.worker().is_declined(key.key_id),
        "Not now declined the key"
    );
    cx.update(|cx| world.services.resume(&root, &operation.to_string(), cx));
    // Resume itself cleared the mark, before anything was sent.
    assert!(
        !world.services.worker().is_declined(key.key_id),
        "Resume clears the request's declined keys"
    );
    // The resend asks for the key again, and may prompt; the wait is bounded.
    answering_prompts(&world, cx, |cx| {
        retirements_sent(&world, cx).len() == 2 && entered(&world, cx).is_empty()
    })
    .await;
    let sent = retirements_sent(&world, cx);
    assert_eq!(sent[1].0, Lane::User);
    assert_eq!(sent[1].1, original[0], "under the named request ID");
    // I-B: the Resume that was answered `original_request_required` is
    // dismissed, so it does not count as needing attention: only the pending
    // row we listed does.
    let resumes = world.services.operations().read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(|request| matches!(request.intent, Intent::OperationResume { .. }))
            .map(|request| matches!(request.state, RequestState::Dismissed))
            .collect::<Vec<_>>()
    });
    assert_eq!(resumes, [true], "the Resume is dismissed");
    let counted = world.services.scheduler().read_with(cx, |scheduler, cx| {
        world.services.operations().read_with(cx, |operations, cx| {
            world.services.drafts().read_with(cx, |drafts, _| {
                crate::desktop::model::attention::needs_attention(
                    operations,
                    drafts,
                    scheduler.schedule(),
                )
            })
        })
    });
    assert_eq!(counted, 1, "the listed row alone");
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
    // E-32: the library still lists the pending retirement's row.
    world
        .backend
        .inner()
        .answer_pending(Some(pending_list(&operation.to_string())));
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
    // Not now had declined the key; Unlock… clears the mark before it
    // resends.
    world.services.worker().decline_for_testing(key.key_id);
    assert!(world.services.worker().is_declined(key.key_id));
    assert!(super::attention_tests::unlock(&world, key.key_id, cx));
    assert!(
        !world.services.worker().is_declined(key.key_id),
        "Unlock… clears the key's declined mark"
    );
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
    // E-35: Unlock… cleared both holds, so both repositories poll again;
    // `raise` leaves a repository whose poll is queued or running alone, so
    // the holds are raised once neither poll is in flight, not after a
    // bounded wait that a loaded machine overruns.
    World::settle(cx, "both polls to answer", |cx| {
        [&a, &b].into_iter().all(|root| {
            !matches!(
                world.phase(root, cx),
                Some(Phase::Queued | Phase::Running { .. })
            )
        })
    })
    .await;
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

/// Review I-1: `reconcile` is guarded by `entry_epoch` alone once a read
/// no longer moves it, so every event that begins, answers or settles an
/// entry must move it. Each of the three sites, singly.
#[gpui_kit::test]
async fn retire_entry_events_move_the_epoch(cx: &mut TestAppContext) {
    let epoch = |world: &World, cx: &mut TestAppContext| {
        world
            .services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.entry_epoch)
    };

    // A retirement submitted (`resend_retirement`).
    {
        let (world, root) = polled(cx).await;
        world.backend.inner().answer_job(
            JobKind::ContextRetire,
            answer(State::Retired, Code::Ok, Removal::Observed),
        );
        let before = epoch(&world, cx);
        retire(&world, &root, Lane::User, cx);
        assert!(epoch(&world, cx) > before, "a submit moves the epoch");
    }

    // A pending retirement ended by an Abandon or a Resume (`pending_ended`).
    {
        let (world, root, _, operation) = left_pending(cx, None).await;
        let before = epoch(&world, cx);
        world.heard(
            vec![Heard::PendingEnded {
                root,
                operation,
                finished: true,
            }],
            cx,
        );
        assert!(epoch(&world, cx) > before, "an ended row moves the epoch");
        assert!(entered(&world, cx).is_empty());
    }

    // A retirement's progress (`retiring`), and the stronger claim: a list
    // read before it, complete and without the entry's row, ends nothing.
    {
        let (world, root, request, _) = left_pending(cx, None).await;
        let before = epoch(&world, cx);
        let operation = OperationId::new();
        world.heard(
            vec![Heard::Retirement(Retiring::Operation {
                request,
                operation,
            })],
            cx,
        );
        assert!(epoch(&world, cx) > before, "progress moves the epoch");
        world.services.scheduler().update(cx, |scheduler, cx| {
            scheduler.reconciled(&root, before, Some(empty_list()), cx)
        });
        assert_eq!(
            entered(&world, cx),
            [request],
            "a read that began before the change ends nothing"
        );
        assert!(retiring_now(&world, &root, cx));
    }
}

/// Review 7: "a retirement of the item is queued or running" is keyed by
/// repository, kind and item ID, wherever it is asked. A queued retirement
/// of a document that has the pending ticket's ID does not hold the ticket:
/// Unlock… still resends the ticket's retirement.
#[gpui_kit::test]
async fn retirement_in_flight_is_keyed_by_kind(cx: &mut TestAppContext) {
    use manyhands::repository::{AuthoringKind, ItemDtoKind};

    use crate::desktop::services::scheduler::attention::UnlockTarget;
    let world = World::new(true, cx);
    let directory = tempfile::tempdir().unwrap();
    let key = unlock_request(directory.path(), SharedKeyId::new());
    let a = world.ssh_repo("a");
    let (request, operation) = (RequestId::new(), OperationId::new());
    {
        let mut previous = InflightStore::open(world.fixture.data_dir());
        assert_eq!(previous.try_acquire(), Acquired::Now);
        previous.retirement_submitting(RetirementEntry::new(
            request,
            &a,
            TargetKind::Ticket,
            &item(),
        ));
        previous.retirement_operation(request, operation);
        previous.retirement_ended(request, RetirementEnd::Accepted);
    }
    world
        .backend
        .inner()
        .answer_pending(Some(pending_list(&operation.to_string())));
    super::attention_tests::locked_pair(&world, &key, std::slice::from_ref(&a), cx).await;
    // A retirement of a document with the ticket's ID runs (held).
    let running = world.backend.inner().hold_job(JobKind::ContextRetire);
    world.backend.inner().answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, Code::Ok, Removal::Observed),
    );
    world.services.worker().submit(
        Intent::ContextRetire {
            root: std::path::PathBuf::from(&a),
            kind: AuthoringKind::Document,
            id: item(),
        },
        Lane::Background,
        None,
        None,
    );
    running.wait_reached();
    let worker = world.services.worker();
    assert!(worker.retirement_in_flight(&a, ItemDtoKind::Document, ITEM));
    assert!(!worker.retirement_in_flight(&a, ItemDtoKind::Ticket, ITEM));
    let target = world.services.scheduler().read_with(cx, |scheduler, cx| {
        world.services.operations().read_with(cx, |operations, _| {
            scheduler.unlock_target(key.key_id, operations)
        })
    });
    assert_eq!(
        target,
        Some(UnlockTarget::Retirement {
            root: a.clone(),
            request,
            kind: TargetKind::Ticket,
            item: item(),
        }),
        "the document does not hold the ticket"
    );
    running.release();
    World::quiet(cx).await;
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

/// The previous launch left a retirement of the ticket pending (`request`,
/// journal row `operation`), in repository `a`; a launch poll has landed.
async fn left_pending(
    cx: &mut TestAppContext,
    list: Option<manyhands::repository::OperationListDto>,
) -> (World, String, RequestId, OperationId) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    let (request, operation) = (RequestId::new(), OperationId::new());
    {
        let mut previous = InflightStore::open(world.fixture.data_dir());
        assert_eq!(previous.try_acquire(), Acquired::Now);
        previous.retirement_submitting(RetirementEntry::new(
            request,
            &root,
            TargetKind::Ticket,
            &item(),
        ));
        previous.retirement_operation(request, operation);
        previous.retirement_ended(request, RetirementEnd::Accepted);
    }
    world.backend.inner().answer_pending(list);
    world.backend.answer_poll(world.completed());
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the launch poll", |_| {
        world.backend.polled_roots().len() == 1
    })
    .await;
    World::quiet(cx).await;
    (world, root, request, operation)
}

fn retiring_now(world: &World, root: &str, cx: &mut TestAppContext) -> bool {
    world.services.operations().read_with(cx, |operations, _| {
        operations.is_retiring(root, manyhands::repository::ItemDtoKind::Ticket, ITEM)
    })
}

/// R-31, review I-C: a pending retirement whose row was settled outside this
/// process (the CLI, a second instance) no longer holds its item "Retiring…"
/// and read-only: at launch the entry is ended against a complete, fresh
/// pending list that does not name its row, and the item is offered Retire…
/// and taken up by the loop again.
#[gpui_kit::test]
async fn retire_entry_ends_when_its_row_was_settled_elsewhere(cx: &mut TestAppContext) {
    let (world, root, _, _) = left_pending(cx, Some(empty_list())).await;
    assert!(entered(&world, cx).is_empty(), "the entry ended");
    assert!(!retiring_now(&world, &root, cx), "no longer Retiring…");

    // Offered Retire… again, and taken up by the loop at the next attempt.
    world.backend.inner().answer_job(
        JobKind::ContextRetire,
        answer(State::Refused, Code::WorktreeNotClean, Removal::None),
    );
    world.heard(
        vec![Heard::CheckedNow(
            root.clone(),
            Some(crate::desktop::model::polling::PollAttempt {
                root: root.clone(),
                operation: OperationId::new(),
                answer: crate::desktop::model::polling::Answer::Completed,
                completed_at: Some(crate::desktop::model::polling::Clock::utc(&*world.clock)),
                next_eligible_at: Some(
                    crate::desktop::model::polling::Clock::utc(&*world.clock)
                        + time::Duration::seconds(300),
                ),
                targets: vec![crate::desktop::model::polling::AttemptTarget {
                    item_id: Some(item()),
                    kind: Some(TargetKind::Ticket),
                    disposition: crate::desktop::model::polling::Disposition::Closed,
                    main: false,
                }],
                index_pending: false,
            }),
        )],
        cx,
    );
    World::settle(cx, "the loop to take it up", |cx| {
        retirements_sent(&world, cx).len() == 1
    })
    .await;
    World::quiet(cx).await;
    world.services.scheduler().read_with(cx, |scheduler, _| {
        let offers = scheduler.retire_offers(&root);
        assert_eq!(offers.len(), 1, "refused: offered Retire…");
        assert_eq!(offers[0].item, item());
    });
}

/// Review I-C: only a complete list that was read ends an entry. One that
/// failed, one that is not complete and one that names the row end nothing.
#[gpui_kit::test]
async fn retire_entries_reconcile_only_from_a_complete_list(cx: &mut TestAppContext) {
    use manyhands::repository::OperationListDto;
    let (world, root, _, operation) = left_pending(cx, None).await;
    assert_eq!(entered(&world, cx).len(), 1, "a failed read ends nothing");
    let reconcile = |world: &World, list: Option<OperationListDto>, cx: &mut TestAppContext| {
        world.backend.inner().answer_pending(list);
        world
            .services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.reconcile_entries(cx));
    };
    let incomplete = OperationListDto {
        items: Vec::new(),
        complete: false,
    };
    for (what, list) in [
        ("an incomplete list", Some(incomplete)),
        (
            "a list that names the row",
            Some(pending_list(&operation.to_string())),
        ),
    ] {
        reconcile(&world, list, cx);
        World::quiet(cx).await;
        assert_eq!(entered(&world, cx).len(), 1, "{what} ends nothing");
        assert!(retiring_now(&world, &root, cx), "{what}");
    }
    // A list read before the entries last changed may predate the change.
    let stale = world
        .services
        .scheduler()
        .read_with(cx, |scheduler, _| scheduler.entry_epoch + 1);
    world.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.reconciled(&root, stale, Some(empty_list()), cx)
    });
    assert_eq!(entered(&world, cx).len(), 1, "a stale read ends nothing");
    reconcile(&world, Some(empty_list()), cx);
    World::settle(cx, "the entry to end", |cx| entered(&world, cx).is_empty()).await;
    assert!(!retiring_now(&world, &root, cx));
}

/// Review 1 (Task 12a): the reads of every repository's pending operations,
/// taken at one activation, are all applied. The first read that ends an
/// entry must not discard the others'. Two repositories each left a pending
/// retirement whose row was settled elsewhere; one launch ends both.
#[gpui_kit::test]
async fn retire_one_activation_ends_the_stuck_entries_of_every_repository(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let roots = [world.ssh_repo("a"), world.ssh_repo("b")];
    {
        let mut previous = InflightStore::open(world.fixture.data_dir());
        assert_eq!(previous.try_acquire(), Acquired::Now);
        for root in &roots {
            let request = RequestId::new();
            previous.retirement_submitting(RetirementEntry::new(
                request,
                root,
                TargetKind::Ticket,
                &item(),
            ));
            previous.retirement_operation(request, OperationId::new());
            previous.retirement_ended(request, RetirementEnd::Accepted);
        }
    }
    world.backend.inner().answer_pending(Some(empty_list()));
    world.backend.answer_poll(world.completed());
    world.list(&roots, cx);
    World::settle(cx, "the launch polls", |_| {
        world.backend.polled_roots().len() == 2
    })
    .await;
    World::quiet(cx).await;
    assert_eq!(
        entered(&world, cx),
        [],
        "every repository's stuck entry ended"
    );
}

/// Review I-C: an entry whose retirement the worker holds is never
/// reconciled, whatever the list says: the resend running now will answer.
#[gpui_kit::test]
async fn retire_entry_the_worker_holds_is_not_reconciled(cx: &mut TestAppContext) {
    // The row is listed at launch, so the entry stays.
    let (world, root, request, operation) = left_pending(cx, None).await;
    world
        .backend
        .inner()
        .answer_pending(Some(pending_list(&operation.to_string())));
    // A resend of it is running.
    let running = world.backend.inner().hold_job(JobKind::ContextRetire);
    world.backend.inner().answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, Code::Ok, Removal::Observed),
    );
    world.services.scheduler().update(cx, |scheduler, _| {
        scheduler.resend_retirement(request, &root, TargetKind::Ticket, &item(), Lane::User);
    });
    running.wait_reached();
    // The list no longer names its row, but the worker holds the request.
    world.backend.inner().answer_pending(Some(empty_list()));
    world
        .services
        .scheduler()
        .update(cx, |scheduler, cx| scheduler.reconcile_entries(cx));
    World::quiet(cx).await;
    assert_eq!(entered(&world, cx), [request], "held by the worker");
    running.release();
    World::settle(cx, "the resend to end", |cx| entered(&world, cx).is_empty()).await;
}

/// Review M-1: window activation reads the pending list for a lock the
/// scheduler already held (taking the lock reads it itself, once): a row
/// settled while the window was away ends its entry then.
#[gpui_kit::test]
async fn retire_activation_reconciles_a_lock_already_held(cx: &mut TestAppContext) {
    // The launch read fails, so the entry stays.
    let (world, root, request, _) = left_pending(cx, None).await;
    assert_eq!(entered(&world, cx), [request]);
    assert!(retiring_now(&world, &root, cx));
    // Away from the window, the row was settled elsewhere.
    world.backend.inner().answer_pending(Some(empty_list()));
    world.activate(cx);
    World::settle(cx, "the entry to end", |cx| entered(&world, cx).is_empty()).await;
    assert!(!retiring_now(&world, &root, cx));
}

/// Review M-6: the worker's own hold on the item is a check of its own, not
/// only the request model's. A resend has just been submitted, so the worker
/// holds the item, and the drain has not yet told the request model it is
/// queued (no request of that ID shows as queued or running); the list lacks
/// the row. The entry stays.
#[gpui_kit::test]
async fn retire_entry_the_worker_holds_is_kept_before_the_model_shows_it(cx: &mut TestAppContext) {
    // The launch read fails, so the entry (with its operation) stays.
    let (world, root, request, _) = left_pending(cx, None).await;
    assert_eq!(entered(&world, cx), [request]);
    // Held, so that the worker still has the request when the list is read.
    let running = world.backend.inner().hold_job(JobKind::ContextRetire);
    world.backend.inner().answer_job(
        JobKind::ContextRetire,
        answer(State::Retired, Code::Ok, Removal::Observed),
    );
    let operations = world.services.operations().clone();
    world.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.resend_retirement(request, &root, TargetKind::Ticket, &item(), Lane::User);
        // One synchronous turn: the model has heard nothing of the resend.
        assert!(
            operations
                .read(cx)
                .requests()
                .iter()
                .all(|held| held.request_id != request),
            "the request model does not show the resend yet"
        );
        let epoch = scheduler.entry_epoch;
        // The list was read after the resend and is complete without the row.
        scheduler.reconciled(&root, epoch, Some(empty_list()), cx);
    });
    assert_eq!(entered(&world, cx), [request], "held by the worker");
    running.release();
    World::settle(cx, "the resend to end", |cx| entered(&world, cx).is_empty()).await;
}

/// Review minor 1: what the attempt's entry says when a pending retirement's
/// row is settled by an Abandon or a Resume. A Resume that finished the row
/// says "Finished retiring {item}."; nothing is claimed otherwise, and never
/// "Retired the copy…", which only a retirement that removed it says.
#[gpui_kit::test]
async fn retire_pending_ended_says_only_what_happened(cx: &mut TestAppContext) {
    use crate::desktop::model::polling::entry::{
        RetireLine, RetireState, finished_text, short_code,
    };
    let (world, root, request, operation) =
        left_pending(cx, Some(pending_list(&OperationId::new().to_string()))).await;
    let lines = |world: &World, cx: &mut TestAppContext| -> Vec<RetireLine> {
        world.services.scheduler().read_with(cx, |scheduler, _| {
            scheduler
                .schedule()
                .attempts()
                .flat_map(|(_, _, lines)| lines.to_vec())
                .collect()
        })
    };
    let owed = Verdict::Owed {
        attention: None,
        catching_up: false,
    };
    for (finished, expected) in [(true, vec![RetireState::Finished]), (false, vec![])] {
        // Pending again, with its line saying it did not finish.
        world.services.scheduler().update(cx, |scheduler, cx| {
            scheduler.hold_retirement(request, &root, TargetKind::Ticket, &item(), operation, cx);
            let schedule = scheduler.schedule_mut();
            schedule.retire_started(&root, TargetKind::Ticket, &item());
            schedule.retire_settled(&root, TargetKind::Ticket, &item(), &owed);
        });
        world.heard(
            vec![Heard::PendingEnded {
                root: root.clone(),
                operation,
                finished,
            }],
            cx,
        );
        let said: Vec<RetireState> = lines(&world, cx).iter().map(|line| line.state).collect();
        assert_eq!(said, expected, "finished: {finished}");
        assert!(entered(&world, cx).is_empty());
        for line in lines(&world, cx) {
            assert_eq!(line.text(), finished_text(&short_code(&item())));
            assert!(!line.text().contains("Retired the copy"));
        }
    }
}
