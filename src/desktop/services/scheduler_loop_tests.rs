//! The retirement loop at the scheduler (D6 Task 11b, design E1, RT-1 to
//! RT-11): after a completed attempt, one background `ContextRetire` per
//! reported item, one at a time. The library's answers are canned and its
//! call is not made (decision 9, `answer_job`); the poll's report is
//! canned too, since no desktop test reaches the SSH transport. The pure
//! rules are `retire_loop_table`'s; these tests check that the scheduler
//! reads the facts, marks, submits and ends the loop as they say.

use gpui_kit::TestAppContext;
use manyhands::{
    drafts::{DraftFields, DraftTarget, ItemKind},
    repository::{
        AuthoringKind, ContextRetirementState as State, MutationDataDto, PollCompletion,
        PollDisposition, RetirementRemoval as Removal,
    },
    results::{Envelope, ResultCode as Code},
};

use super::{
    reconfigure::{Heard, Moment, Retiring, Scope},
    support::World,
};
use crate::desktop::{
    model::{
        polling::{
            AttemptTarget, Clock as _, Disposition, Phase, Root, TargetKind, entry::RetireState,
        },
        tabs::TabKey,
    },
    services::{
        JobKind, ReadKind,
        drafts::Fresh,
        edit_tabs::TabEditing,
        poll_convert::TargetParts,
        requests::{Intent, RequestState},
    },
    test_support::retire::answer,
};

const IDS: [&str; 4] = [
    "01ARZ3NDEKTSV4RRFFQ69G5FA1",
    "01ARZ3NDEKTSV4RRFFQ69G5FA2",
    "01ARZ3NDEKTSV4RRFFQ69G5FA3",
    "01ARZ3NDEKTSV4RRFFQ69G5FA4",
];

fn retired() -> Envelope<MutationDataDto> {
    answer(State::Retired, Code::Ok, Removal::Observed)
}

/// A refusal the loop goes on after (RT-7).
fn refused() -> Envelope<MutationDataDto> {
    answer(State::Refused, Code::WorktreeNotClean, Removal::None)
}

/// A report of closed tickets in the order given.
fn closed(reported: &[usize]) -> Vec<TargetParts> {
    reported
        .iter()
        .map(|&n| TargetParts {
            item_id: Some(IDS[n].parse().unwrap()),
            kind: Some(AuthoringKind::Ticket),
            disposition: PollDisposition::Closed,
        })
        .collect()
}

/// The same items as an attempt's targets, for a Check now.
fn targets(reported: &[usize]) -> Vec<AttemptTarget> {
    reported
        .iter()
        .map(|&n| AttemptTarget {
            item_id: Some(IDS[n].parse().unwrap()),
            kind: Some(TargetKind::Ticket),
            disposition: Disposition::Closed,
        })
        .collect()
}

fn index(id: &str) -> usize {
    IDS.iter().position(|known| *known == id).expect("an item")
}

/// The items retirements were sent for, oldest first.
fn sent(world: &World, cx: &mut TestAppContext) -> Vec<usize> {
    world.services.operations().read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .rev()
            .filter_map(|request| match &request.intent {
                Intent::ContextRetire { id, .. } => Some(index(&id.to_string())),
                _ => None,
            })
            .collect()
    })
}

/// Whether every retirement sent has ended.
fn all_ended(world: &World, cx: &mut TestAppContext) -> bool {
    world.services.operations().read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(|request| matches!(request.intent, Intent::ContextRetire { .. }))
            .all(|request| {
                !matches!(
                    request.state,
                    RequestState::Queued | RequestState::Running { .. }
                )
            })
    })
}

/// What the attempt's entry says of each retirement, in the order they
/// began (E3).
fn entry_lines(world: &World, cx: &mut TestAppContext) -> Vec<RetireState> {
    world.services.scheduler().read_with(cx, |scheduler, _| {
        scheduler
            .schedule()
            .attempts()
            .flat_map(|(_, _, lines)| lines.iter().map(|line| line.state))
            .collect()
    })
}

fn consistent(world: &World, cx: &mut TestAppContext) -> bool {
    world
        .services
        .scheduler()
        .read_with(cx, |scheduler, cx| scheduler.loops_consistent(cx))
}

/// Stores a draft of the ticket `n` of `root`, claimed by no one.
fn store_draft(world: &World, root: &str, n: usize) {
    let target = DraftTarget {
        repository: root.to_owned(),
        item_kind: ItemKind::Ticket,
        item_id: IDS[n].parse().unwrap(),
        comment: None,
    };
    let current = DraftFields::Ticket {
        title: "T".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: "Words".to_owned(),
    };
    world
        .fixture
        .draft_store()
        .create(target, None, current)
        .unwrap_or_else(|error| panic!("the draft must be stored: {error}"));
}

/// An open tab of the ticket `n` that says it is being edited.
fn open_tab(world: &World, root: &str, n: usize, cx: &mut TestAppContext) {
    let tab = TabKey::Item {
        repo: root.to_owned(),
        kind: manyhands::repository::ItemDtoKind::Ticket,
        id: IDS[n].to_owned(),
    };
    let editing = TabEditing {
        editing: true,
        ..TabEditing::default()
    };
    world.services.edit_tabs().update(cx, |tabs, _| {
        tabs.watch(tab, Box::new(move |_| Some(editing)))
    });
}

/// What a row of the world table is: which items the poll reports, which
/// are edited or have a draft, how each submitted item is answered, and
/// which are expected to be submitted.
struct Row {
    what: &'static str,
    reported: Vec<usize>,
    edited: Vec<usize>,
    drafted: Vec<usize>,
    completion: PollCompletion,
    answers: Vec<Envelope<MutationDataDto>>,
    submitted: Vec<usize>,
    /// What the attempt's entry says of each retirement afterwards, in the
    /// order they began (E3); `None` where the row does not check it.
    lines: Option<Vec<RetireState>>,
}

fn row(
    what: &'static str,
    reported: &[usize],
    answers: Vec<Envelope<MutationDataDto>>,
    submitted: &[usize],
) -> Row {
    Row {
        what,
        reported: reported.to_vec(),
        edited: Vec::new(),
        drafted: Vec::new(),
        completion: PollCompletion::Completed,
        answers,
        submitted: submitted.to_vec(),
        lines: None,
    }
}

/// A repository `a` whose launch poll reports `reported` (canned), with the
/// draft listing landed first, and the retirements it will be asked for
/// answered by `answers`. Returns once the poll has been applied.
async fn polled(row: &Row, cx: &mut TestAppContext) -> (World, Root) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    for &n in &row.drafted {
        store_draft(&world, &root, n);
    }
    // The launch listing lands before the poll completes.
    World::quiet(cx).await;
    for &n in &row.edited {
        open_tab(&world, &root, n, cx);
    }
    for answer in &row.answers {
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, answer.clone());
    }
    let report = world.report(row.completion, closed(&row.reported), false);
    world.backend.answer_poll(report);
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the launch poll", |_| {
        world.backend.polled_roots().len() == 1
    })
    .await;
    // Paused, so the policy an answer reads again does not make the
    // repository due: the fake poll stores no deadline, and the next
    // attempt would replace the entry's lines.
    world.fixture.pause_polling(std::path::Path::new(&root));
    (world, root)
}

/// Waits for the loop to end: every retirement it sent has answered and the
/// repository is polled again.
async fn finished(world: &World, root: &str, cx: &mut TestAppContext) {
    World::settle(cx, "the loop to end", |cx| {
        all_ended(world, cx) && world.phase(root, cx) == Some(Phase::Idle)
    })
    .await;
    World::quiet(cx).await;
}

/// The wiring of the loop through the scheduler: it reads the open tabs and
/// the draft store, submits in report order under a new request each, says
/// what came of each in the attempt's entry, and ends in the phase that lets
/// the repository be polled again; a cancelled attempt starts none. The
/// rules (RT-7 to RT-9, R-37, duplicates, an interrupted attempt) are
/// `retire_loop_table`'s, `retire_answer_table`'s and the model's.
#[gpui_kit::test]
async fn retire_loop_world(cx: &mut TestAppContext) {
    let rows = vec![
        Row {
            lines: Some(vec![RetireState::Retired; 3]),
            ..row(
                "every item retired, in report order",
                &[2, 0, 1],
                vec![retired(), retired(), retired()],
                &[2, 0, 1],
            )
        },
        Row {
            edited: vec![0],
            drafted: vec![1],
            ..row(
                "RT-3 and RT-4: one edited, one with a draft: only the third",
                &[0, 1, 2],
                vec![retired()],
                &[2],
            )
        },
        Row {
            completion: PollCompletion::Cancelled,
            ..row("a cancelled attempt starts no loop", &[0, 1], vec![], &[])
        },
    ];
    for row in rows {
        let (world, root) = polled(&row, cx).await;
        if row.submitted.is_empty() {
            World::quiet(cx).await;
        } else {
            finished(&world, &root, cx).await;
        }
        assert_eq!(sent(&world, cx), row.submitted, "{}", row.what);
        assert!(
            consistent(&world, cx),
            "{}: a loop and its phase agree",
            row.what
        );
        if let Some(expected) = &row.lines {
            let seen = entry_lines(&world, cx);
            assert_eq!(
                &seen, expected,
                "{}: the entry's retirement lines",
                row.what
            );
        }
        if !row.submitted.is_empty() {
            assert_ne!(
                world.phase(&root, cx),
                Some(Phase::Retiring),
                "{}: the repository is polled again",
                row.what
            );
        }
    }
}

/// A Check now's completed attempt over `reported`, as the drain hands it.
fn checked(world: &World, root: &str, reported: &[usize]) -> Heard {
    use crate::desktop::model::polling::{Answer, PollAttempt};
    Heard::CheckedNow(
        root.to_owned(),
        Some(PollAttempt {
            root: root.to_owned(),
            operation: manyhands::repository::OperationId::new(),
            answer: Answer::Completed,
            completed_at: Some(world.clock.utc()),
            next_eligible_at: Some(world.clock.utc() + time::Duration::seconds(300)),
            targets: targets(reported),
            index_pending: false,
        }),
    )
}

/// A repository `a` that has had a launch poll reporting nothing.
async fn idle(cx: &mut TestAppContext) -> (World, Root) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    world.backend.answer_poll(world.completed());
    World::quiet(cx).await;
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the launch poll", |_| {
        world.backend.polled_roots().len() == 1
    })
    .await;
    World::quiet(cx).await;
    // Paused, so a Check now's policy read does not make it due again: the
    // fake poll stores no deadline.
    world.fixture.pause_polling(std::path::Path::new(&root));
    (world, root)
}

/// The retirement requests of the loop, newest first, with their state.
fn retire_requests(
    world: &World,
    cx: &mut TestAppContext,
) -> Vec<(
    crate::desktop::services::requests::DesktopRequestId,
    usize,
    bool,
)> {
    world.services.operations().read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter_map(|request| match &request.intent {
                Intent::ContextRetire { id, .. } => Some((
                    request.id,
                    index(&id.to_string()),
                    matches!(request.state, RequestState::Queued),
                )),
                _ => None,
            })
            .collect()
    })
}

/// Waits until a retirement is running (held at its start), letting the
/// loop's own steps on the UI thread run meanwhile.
async fn running_item(world: &World, cx: &mut TestAppContext) {
    World::settle(cx, "a retirement to run", |cx| {
        world.services.operations().read_with(cx, |operations, _| {
            operations.requests().iter().any(|request| {
                matches!(request.intent, Intent::ContextRetire { .. })
                    && matches!(request.state, RequestState::Running { .. })
            })
        })
    })
    .await;
}

fn retiring(world: &World, root: &str, n: usize, cx: &mut TestAppContext) -> bool {
    world.services.operations().read_with(cx, |operations, _| {
        operations.is_retiring(root, manyhands::repository::ItemDtoKind::Ticket, IDS[n])
    })
}

/// RT-5: a launch poll that completes before the draft listing lands
/// retires nothing, whatever the store holds; the items wait for the next
/// completed attempt, which retires them once the listing is known.
#[gpui_kit::test]
async fn retire_listing_not_landed(cx: &mut TestAppContext) {
    let mut held = None;
    let world = World::new_with(true, cx, |backend| {
        held = Some(backend.inner().hold_read(ReadKind::Drafts));
    });
    let root = world.ssh_repo("a");
    for _ in 0..2 {
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
    }
    world
        .backend
        .answer_poll(world.report(PollCompletion::Completed, closed(&[0, 1]), false));
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the launch poll", |_| {
        world.backend.polled_roots().len() == 1
    })
    .await;
    World::quiet(cx).await;
    assert!(sent(&world, cx).is_empty(), "the listing had not landed");
    assert_eq!(world.phase(&root, cx), Some(Phase::Idle), "polled again");
    assert!(consistent(&world, cx));

    // The listing lands; the next completed attempt retires them.
    held.take().unwrap().send(()).unwrap();
    World::quiet(cx).await;
    world.heard(vec![checked(&world, &root, &[0, 1])], cx);
    finished(&world, &root, cx).await;
    assert_eq!(sent(&world, cx), [0, 1]);
}

/// RT-6: the item is marked "Retiring…" as the loop reaches it, the draft
/// store is listed again, and what is stored or begun between the check and
/// the fresh listing stops that item and only that item.
#[gpui_kit::test]
async fn retire_relist_before_submit(cx: &mut TestAppContext) {
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    World::quiet(cx).await;
    for _ in 0..2 {
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
    }
    // Held at the fresh listing the first item asks for.
    let gate = world.backend.inner().hold_read(ReadKind::Drafts);
    world
        .backend
        .answer_poll(world.report(PollCompletion::Completed, closed(&[0, 1, 2]), false));
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the first item to be marked", |cx| {
        retiring(&world, &root, 0, cx)
    })
    .await;
    World::quiet(cx).await;
    assert!(
        !retiring(&world, &root, 1, cx),
        "one item at a time is marked"
    );
    assert_eq!(world.phase(&root, cx), Some(Phase::Retiring));
    assert!(sent(&world, cx).is_empty(), "held at the listing");

    // A draft is stored, and the second item is opened for editing, before
    // the fresh listing lands.
    store_draft(&world, &root, 0);
    gate.send(()).unwrap();
    World::settle(cx, "the loop to end", |cx| {
        all_ended(&world, cx) && world.phase(&root, cx) == Some(Phase::Idle)
    })
    .await;
    World::quiet(cx).await;
    assert_eq!(
        sent(&world, cx),
        [1, 2],
        "only the drafted item was stopped"
    );
    assert!(!retiring(&world, &root, 0, cx), "its mark is cleared");
    assert!(consistent(&world, &mut *cx));

    // An edit begun while the fresh listing is out stops its item too.
    let (world, root) = idle(cx).await;
    let gate = world.backend.inner().hold_read(ReadKind::Drafts);
    world
        .backend
        .inner()
        .answer_job(JobKind::ContextRetire, retired());
    world.heard(vec![checked(&world, &root, &[0, 3])], cx);
    World::settle(cx, "the first item to be marked", |cx| {
        retiring(&world, &root, 0, cx)
    })
    .await;
    open_tab(&world, &root, 0, cx);
    gate.send(()).unwrap();
    finished(&world, &root, cx).await;
    assert_eq!(sent(&world, cx), [3]);
}

/// RT-9, RT-10 and the other ends of a loop: a request removed from the
/// queue, one the person stopped, a reconfiguration submitted or finished,
/// a quit, a failed listing and a repository forgotten each end the loop
/// whole, leave the repository in no stuck phase, and let the item that was
/// running answer on its own.
#[gpui_kit::test]
async fn retire_loop_exits(cx: &mut TestAppContext) {
    let own = |root: &str| Scope::Repository {
        root: root.to_owned(),
        gone: false,
    };

    // RT-9: queued behind a held refresh and removed by Cancel.
    {
        let (world, root) = idle(cx).await;
        let refresh = world.backend.inner().hold_job(JobKind::IndexRefresh);
        // The Check now's attempt is index-pending, so a refresh goes first.
        let mut pending = checked(&world, &root, &[0, 1]);
        if let Heard::CheckedNow(_, Some(attempt)) = &mut pending {
            attempt.index_pending = true;
        }
        world.heard(vec![pending], cx);
        World::settle(cx, "the item to be queued", |cx| {
            retire_requests(&world, cx)
                .iter()
                .any(|(_, _, queued)| *queued)
        })
        .await;
        let (queued, ..) = retire_requests(&world, cx)[0];
        assert_eq!(
            world.services.worker().cancel(queued),
            crate::desktop::services::worker::CancelResult::Removed
        );
        refresh.release();
        finished(&world, &root, cx).await;
        assert_eq!(sent(&world, cx), [0], "RT-9: the second was never sent");
        assert!(consistent(&world, cx));
    }

    // RT-10: a reconfiguration submitted removes the item queued behind a
    // held refresh, and ends its loop.
    {
        let (world, root) = idle(cx).await;
        let refresh = world.backend.inner().hold_job(JobKind::IndexRefresh);
        let mut pending = checked(&world, &root, &[0, 1]);
        if let Heard::CheckedNow(_, Some(attempt)) = &mut pending {
            attempt.index_pending = true;
        }
        world.heard(vec![pending], cx);
        World::settle(cx, "the item to be queued", |cx| {
            retire_requests(&world, cx)
                .iter()
                .any(|(_, _, queued)| *queued)
        })
        .await;
        world.heard(
            vec![Heard::Reconfiguration(Moment::Submitted, own(&root))],
            cx,
        );
        refresh.release();
        World::quiet(cx).await;
        let states = world.services.operations().read_with(cx, |operations, _| {
            operations
                .requests()
                .iter()
                .filter(|request| matches!(request.intent, Intent::ContextRetire { .. }))
                .map(|request| matches!(request.state, RequestState::Removed))
                .collect::<Vec<_>>()
        });
        assert_eq!(states, [true], "the queued item was removed");
        assert_eq!(world.phase(&root, cx), Some(Phase::Idle));
        assert!(consistent(&world, cx));
        let entries = world
            .services
            .scheduler()
            .read_with(cx, |scheduler, _| scheduler.store().retirements().len());
        assert_eq!(entries, 0, "its entry ended with it");
    }

    // The person pressed Stop on the running item: the loop stops, though a
    // cancelled retirement is a retry code (RT-7) otherwise.
    {
        let (world, root) = idle(cx).await;
        let running = world.backend.inner().hold_job(JobKind::ContextRetire);
        world.backend.inner().answer_job(
            JobKind::ContextRetire,
            answer(State::Refused, Code::Cancelled, Removal::None),
        );
        world.heard(vec![checked(&world, &root, &[0, 1])], cx);
        running_item(&world, cx).await;
        assert_eq!(
            entry_lines(&world, cx),
            [RetireState::Retiring],
            "the entry says it is being retired"
        );
        let (id, ..) = retire_requests(&world, cx)[0];
        world
            .services
            .operations()
            .update(cx, |operations, _| operations.stopping(id));
        running.release();
        finished(&world, &root, cx).await;
        assert_eq!(sent(&world, cx), [0], "Stop ends the loop");
        assert!(consistent(&world, cx));
    }

    // RT-10: a reconfiguration submitted drops the loop at once and the
    // queued item with it; the running one finishes and finds no loop.
    for (what, moment) in [
        ("submitted", Moment::Submitted),
        ("finished", Moment::Finished),
    ] {
        let (world, root) = idle(cx).await;
        let running = world.backend.inner().hold_job(JobKind::ContextRetire);
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
        world.heard(vec![checked(&world, &root, &[0, 1])], cx);
        running_item(&world, cx).await;
        world.heard(vec![Heard::Reconfiguration(moment, own(&root))], cx);
        assert_eq!(world.phase(&root, cx), Some(Phase::Idle), "{what}");
        assert!(consistent(&world, cx), "{what}");
        running.release();
        World::quiet(cx).await;
        assert_eq!(sent(&world, cx), [0], "{what}: the second was dropped");
        assert_eq!(
            world.phase(&root, cx),
            Some(Phase::Idle),
            "{what}: and stays so"
        );
    }

    // RT-10: none starts while a reconfiguring request is queued or running.
    {
        let (world, root) = idle(cx).await;
        let clearing = world.backend.inner().hold_job(JobKind::KeyClear);
        world.services.worker().submit(
            Intent::KeyClear,
            crate::desktop::services::requests::Lane::User,
            None,
            None,
        );
        clearing.wait_reached();
        // The drain has heard the change queued before the loop could start.
        World::quiet(cx).await;
        world.heard(vec![checked(&world, &root, &[0, 1])], cx);
        World::quiet(cx).await;
        assert!(sent(&world, cx).is_empty(), "no loop beside a key change");
        assert_eq!(world.phase(&root, cx), Some(Phase::Idle));
        clearing.release();
        World::quiet(cx).await;
    }

    // RT-11: a Check now's attempt replaces the loop's remaining items.
    {
        let (world, root) = idle(cx).await;
        let running = world.backend.inner().hold_job(JobKind::ContextRetire);
        for _ in 0..3 {
            world
                .backend
                .inner()
                .answer_job(JobKind::ContextRetire, retired());
        }
        world.heard(vec![checked(&world, &root, &[0, 1, 2])], cx);
        running_item(&world, cx).await;
        world.heard(vec![checked(&world, &root, &[3, 2])], cx);
        running.release();
        finished(&world, &root, cx).await;
        assert_eq!(sent(&world, cx), [0, 3, 2], "RT-11: never the replaced 1");
        assert!(consistent(&world, cx));
    }

    // A quit asked drops the loop; Cancel leaves no stuck phase.
    {
        let (world, root) = idle(cx).await;
        let running = world.backend.inner().hold_job(JobKind::ContextRetire);
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
        world.heard(vec![checked(&world, &root, &[0, 1])], cx);
        running_item(&world, cx).await;
        assert_eq!(world.phase(&root, cx), Some(Phase::Retiring));
        let quit = |quitting: bool, cx: &mut TestAppContext| {
            world
                .services
                .scheduler()
                .update(cx, |scheduler, cx| scheduler.set_quitting(quitting, cx));
        };
        quit(true, cx);
        assert_eq!(world.phase(&root, cx), Some(Phase::Idle));
        assert!(consistent(&world, cx));
        running.release();
        World::quiet(cx).await;
        quit(false, cx);
        assert_eq!(world.phase(&root, cx), Some(Phase::Idle), "not stuck");
        assert_eq!(sent(&world, cx), [0]);
        assert!(consistent(&world, cx));
    }

    // The fresh listing failed: the loop ends, the item is not marked.
    {
        let (world, root) = idle(cx).await;
        let gate = world.backend.inner().hold_read(ReadKind::Drafts);
        world.heard(vec![checked(&world, &root, &[0, 1])], cx);
        World::settle(cx, "the item to be marked", |cx| {
            retiring(&world, &root, 0, cx)
        })
        .await;
        world.services.scheduler().update(cx, |scheduler, cx| {
            let generation = scheduler.loops.get(&root).unwrap().generation();
            scheduler.relisted(root.clone(), generation, Fresh::Failed, cx);
        });
        assert!(!retiring(&world, &root, 0, cx), "no longer marked");
        assert_eq!(world.phase(&root, cx), Some(Phase::Idle));
        assert!(consistent(&world, cx));
        let _ = gate.send(());
        World::quiet(cx).await;
        assert!(sent(&world, cx).is_empty());
    }

    // A repository the list no longer shows.
    {
        let (world, root) = idle(cx).await;
        let running = world.backend.inner().hold_job(JobKind::ContextRetire);
        world.heard(vec![checked(&world, &root, &[0, 1])], cx);
        running_item(&world, cx).await;
        world.list(&[], cx);
        assert!(!world.scheduled(&root, cx), "forgotten");
        assert!(consistent(&world, cx));
        running.release();
        World::quiet(cx).await;
        assert_eq!(sent(&world, cx), [0]);
        assert!(consistent(&world, cx));
    }
}

/// L-6: a second instance runs no loop. Another process holds the lock.
#[gpui_kit::test]
async fn a_second_instance_runs_no_loop(cx: &mut TestAppContext) {
    use crate::desktop::services::inflight::{Acquired, InflightStore};
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    let mut other = InflightStore::open(world.fixture.data_dir());
    assert_eq!(other.try_acquire(), Acquired::Now);
    world
        .backend
        .inner()
        .answer_job(JobKind::ContextRetire, retired());
    world
        .backend
        .answer_poll(world.report(PollCompletion::Completed, closed(&[0]), false));
    world.list(std::slice::from_ref(&root), cx);
    World::settle(cx, "the launch poll", |_| {
        world.backend.polled_roots().len() == 1
    })
    .await;
    World::quiet(cx).await;
    assert!(sent(&world, cx).is_empty());
    assert_eq!(world.phase(&root, cx), Some(Phase::Idle));
}

/// A Check now's answer reaches the scheduler with its attempt (E1, owner
/// ruling of 2026-11-10): a success, and `discovery_pending` with the index
/// still behind, carry the report's items in report order; one that did
/// not complete, or has no data, carries none; any other failure is a
/// failure.
#[test]
fn check_now_is_heard_with_its_attempt() {
    use manyhands::{
        repository::{OperationId, PollMutationCompletion, PollMutationDto, PollTargetDto},
        results::{Outcome, Scope as ResultScope},
    };

    use crate::desktop::{
        model::operations::Operations,
        services::{
            requests::{DesktopRequest, DesktopRequestId},
            scheduler::heard,
            worker::WorkerEvent,
        },
    };

    let data = |completion| {
        MutationDataDto::Poll(PollMutationDto {
            operation_id: OperationId::new().to_string(),
            completion,
            completed_at: "2026-11-10T12:00:00Z".to_owned(),
            next_eligible_at: None,
            targets: [2, 0]
                .iter()
                .map(|&n| PollTargetDto {
                    target: None,
                    kind: Some(manyhands::repository::ItemDtoKind::Ticket),
                    item_id: Some(IDS[n].to_owned()),
                    disposition: manyhands::repository::PollTargetDisposition::Closed,
                    before_oid: None,
                    after_oid: None,
                })
                .collect(),
        })
    };
    let envelope = |outcome, code, data: Option<MutationDataDto>| {
        let envelope = Envelope::mutation("poll once", ResultScope::default(), outcome, code);
        match data {
            Some(data) => envelope.with_data(data),
            None => envelope,
        }
    };
    let completed = PollMutationCompletion::Completed;
    // (what, the answer, what it was heard as: `None` a failure, else the
    // attempt's last item and `index_pending`, if it carried an attempt)
    type Heardas = Option<Option<(usize, bool)>>;
    let cases: Vec<(&str, Envelope<MutationDataDto>, Heardas)> = vec![
        (
            "success",
            envelope(Outcome::Success, Code::Ok, Some(data(completed))),
            Some(Some((2, false))),
        ),
        (
            "discovery pending: the index is still behind",
            envelope(
                Outcome::Partial,
                Code::DiscoveryPending,
                Some(data(completed)),
            ),
            Some(Some((2, true))),
        ),
        (
            "success with no data",
            envelope(Outcome::Success, Code::Ok, None),
            Some(None),
        ),
        (
            "success that did not complete",
            envelope(
                Outcome::Success,
                Code::Ok,
                Some(data(PollMutationCompletion::Cancelled)),
            ),
            Some(None),
        ),
        (
            "a failure",
            envelope(
                Outcome::Blocked,
                Code::TransportUnavailable,
                Some(data(completed)),
            ),
            None,
        ),
    ];
    let root = "/repo";
    for (what, answer, expected) in cases {
        let mut operations = Operations::default();
        let id = DesktopRequestId(1);
        operations.apply(WorkerEvent::Queued(DesktopRequest {
            id,
            request_id: manyhands::repository::RequestId::new(),
            intent: Intent::PollOnce { root: root.into() },
            confirmation: None,
            lane: crate::desktop::services::requests::Lane::User,
            attempt: 1,
            state: RequestState::Queued,
        }));
        let event = WorkerEvent::Finished(id, Box::new(answer));
        let seen = heard(&event, &operations);
        match (seen, expected) {
            (Some(Heard::CheckedNow(heard_root, attempt)), Some(expected)) => {
                assert_eq!(heard_root, root, "{what}");
                let seen = attempt.map(|attempt| {
                    let ids: Vec<usize> = attempt
                        .targets
                        .iter()
                        .map(|target| index(&target.item_id.as_ref().unwrap().to_string()))
                        .collect();
                    assert_eq!(ids, [2, 0], "{what}: report order");
                    (2, attempt.index_pending)
                });
                assert_eq!(seen, expected, "{what}");
            }
            (Some(Heard::CheckFailed { .. }), None) => {}
            (seen, _) => panic!("{what}: heard {seen:?}"),
        }
    }
}

/// Decision 19, R-31: a retirement the last launch left pending is still
/// "Retiring…" after a relaunch, from the in-flight record, and is not once
/// the record lets it go; the loop skips it (RT-1) while it is.
#[gpui_kit::test]
async fn a_pending_retirement_survives_a_relaunch(cx: &mut TestAppContext) {
    use crate::desktop::services::inflight::{
        Acquired, InflightStore, RetirementEnd, RetirementEntry,
    };
    let world = World::new(true, cx);
    let root = world.ssh_repo("a");
    // The previous launch: an entry it kept, and gone.
    let request = manyhands::repository::RequestId::new();
    {
        let mut previous = InflightStore::open(world.fixture.data_dir());
        assert_eq!(previous.try_acquire(), Acquired::Now);
        previous.retirement_submitting(RetirementEntry::new(
            request,
            &root,
            TargetKind::Ticket,
            &IDS[0].parse().unwrap(),
        ));
        previous.retirement_ended(request, RetirementEnd::Accepted);
    }
    for _ in 0..2 {
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
    }
    world
        .backend
        .answer_poll(world.report(PollCompletion::Completed, closed(&[0, 1]), false));
    world.list(std::slice::from_ref(&root), cx);
    finished(&world, &root, cx).await;
    assert_eq!(
        sent(&world, cx),
        [1],
        "RT-1: the pending one is not sent again"
    );
    assert!(retiring(&world, &root, 0, cx), "held by the record");
    assert!(!retiring(&world, &root, 1, cx));

    // The record lets it go (Abandon, or Resume finishing it).
    world.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.heard(
            vec![Heard::Retirement(Retiring::Gone {
                id: crate::desktop::services::requests::DesktopRequestId(9_999),
                request,
                end: RetirementEnd::Removed,
            })],
            cx,
        )
    });
    assert!(!retiring(&world, &root, 0, cx), "and then it is not");
}

/// Decision 20, K-17: a user job sent while a background retirement runs
/// waits for it, does not cancel it, and runs before the rest of the loop;
/// the loop's next item is queued behind it.
#[gpui_kit::test]
async fn user_job_waits_for_retirement(cx: &mut TestAppContext) {
    let (world, root) = idle(cx).await;
    let running = world.backend.inner().hold_job(JobKind::ContextRetire);
    for _ in 0..3 {
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
    }
    world.heard(vec![checked(&world, &root, &[0, 1, 2])], cx);
    running_item(&world, cx).await;
    let before = world.backend.inner().jobs_seen().len();

    // The person's job, on the user lane, while the first item runs.
    let user = world.services.worker().submit(
        Intent::IndexRefresh {
            root: std::path::PathBuf::from(&root),
        },
        crate::desktop::services::requests::Lane::User,
        None,
        None,
    );
    World::quiet(cx).await;
    let job = world
        .services
        .worker()
        .running_job()
        .expect("still running");
    assert!(!job.cancelled, "the retirement is not cancelled");
    assert_ne!(job.id, user, "the user job waits for it");
    assert_eq!(
        world.backend.inner().jobs_seen().len(),
        before,
        "nothing else ran"
    );

    running.release();
    finished(&world, &root, cx).await;
    let seen = world.backend.inner().jobs_seen();
    let order: Vec<JobKind> = seen[before - 1..].to_vec();
    assert_eq!(
        order,
        [
            JobKind::ContextRetire,
            JobKind::IndexRefresh,
            JobKind::ContextRetire,
            JobKind::ContextRetire
        ],
        "the user job runs before the rest of the loop"
    );
    assert_eq!(sent(&world, cx), [0, 1, 2]);
}

/// R-32: a Check now that fails while a loop runs is an attempt that sets the
/// repository's phase, so the loop ends first, with its mark: it must not go
/// on submitting into a repository that waits for the person.
#[gpui_kit::test]
async fn a_failed_check_now_ends_the_loop(cx: &mut TestAppContext) {
    let (world, root) = idle(cx).await;
    // The first item is marked and waits at the fresh listing.
    let gate = world.backend.inner().hold_read(ReadKind::Drafts);
    world.heard(vec![checked(&world, &root, &[0, 1])], cx);
    World::settle(cx, "the item to be marked", |cx| {
        retiring(&world, &root, 0, cx)
    })
    .await;
    assert_eq!(world.phase(&root, cx), Some(Phase::Retiring));
    assert!(consistent(&world, cx));

    world.heard(
        vec![Heard::CheckFailed {
            root: root.clone(),
            request: crate::desktop::services::requests::DesktopRequestId(9_000),
            code: Code::UnlockRequired,
            authority: None,
        }],
        cx,
    );
    assert!(
        matches!(world.phase(&root, cx), Some(Phase::Attention(_))),
        "{:?}",
        world.phase(&root, cx)
    );
    assert!(!retiring(&world, &root, 0, cx), "its mark is cleared");
    assert!(
        consistent(&world, cx),
        "no loop, no mark, no Retiring phase"
    );
    let _ = gate.send(());
    World::quiet(cx).await;
    assert!(sent(&world, cx).is_empty(), "nothing is submitted after it");
    assert!(matches!(world.phase(&root, cx), Some(Phase::Attention(_))));
}

/// RT-6: a fresh listing that a newer one overtook (any draft begun,
/// discarded or restored elsewhere asks for one) is not a failure. The item
/// it was for is skipped, its mark cleared, and the loop goes on: a draft
/// stored meanwhile is not lost to it.
#[gpui_kit::test]
async fn retire_relist_overtaken(cx: &mut TestAppContext) {
    let (world, root) = idle(cx).await;
    for _ in 0..2 {
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
    }
    let gate = world.backend.inner().hold_read(ReadKind::Drafts);
    world.heard(vec![checked(&world, &root, &[0, 1])], cx);
    World::settle(cx, "the first item to be marked", |cx| {
        retiring(&world, &root, 0, cx)
    })
    .await;
    // A draft of that item is stored, and another listing is asked for and
    // lands first, so the held one is overtaken.
    store_draft(&world, &root, 0);
    world.services.drafts().update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    World::quiet(cx).await;
    let _ = gate.send(());
    finished(&world, &root, cx).await;
    assert_eq!(
        sent(&world, cx),
        [1],
        "the drafted item is skipped and the loop goes on to the next"
    );
    assert!(!retiring(&world, &root, 0, cx), "its mark is cleared");
    assert!(consistent(&world, cx));
}

/// RT-1 and RT-2 as the scheduler reads them. An attempt that reports an
/// item whose retirement the worker already holds, running or queued, with
/// no entry in the in-flight record, submits nothing for it (RT-1); an item
/// whose retirement was refused joins the refused set, and the next attempt
/// does not submit it again (RT-2).
#[gpui_kit::test]
async fn retire_loop_leaves_what_the_worker_and_the_refused_set_hold(cx: &mut TestAppContext) {
    use crate::desktop::services::requests::Lane;
    let direct = |world: &World, n: usize, root: &str| {
        world.services.worker().submit(
            Intent::ContextRetire {
                root: std::path::PathBuf::from(root),
                kind: AuthoringKind::Ticket,
                id: IDS[n].parse().unwrap(),
            },
            Lane::Background,
            None,
            None,
        )
    };
    let (world, root) = idle(cx).await;
    for _ in 0..4 {
        world
            .backend
            .inner()
            .answer_job(JobKind::ContextRetire, retired());
    }

    // Running in the worker.
    let running = world.backend.inner().hold_job(JobKind::ContextRetire);
    direct(&world, 0, &root);
    running_item(&world, cx).await;
    world.heard(vec![checked(&world, &root, &[0, 1])], cx);
    World::quiet(cx).await;
    running.release();
    finished(&world, &root, cx).await;
    assert_eq!(sent(&world, cx), [0, 1], "RT-1: not 0 again while it runs");

    // Queued in the worker, behind a job that is held.
    let refreshing = world.backend.inner().hold_job(JobKind::IndexRefresh);
    world.services.worker().submit(
        Intent::IndexRefresh {
            root: std::path::PathBuf::from(&root),
        },
        Lane::Background,
        None,
        None,
    );
    refreshing.wait_reached();
    direct(&world, 2, &root);
    world.heard(vec![checked(&world, &root, &[2, 3])], cx);
    World::quiet(cx).await;
    refreshing.release();
    finished(&world, &root, cx).await;
    assert_eq!(
        sent(&world, cx),
        [0, 1, 2, 3],
        "RT-1: not 2 again while it waits"
    );

    // Refused: joins the set; the next attempt leaves it.
    world
        .backend
        .inner()
        .answer_job(JobKind::ContextRetire, refused());
    world.heard(vec![checked(&world, &root, &[0])], cx);
    finished(&world, &root, cx).await;
    world.heard(vec![checked(&world, &root, &[0, 1])], cx);
    finished(&world, &root, cx).await;
    assert_eq!(
        sent(&world, cx),
        [0, 1, 2, 3, 0, 1],
        "RT-2: the refused item is not submitted again"
    );
    assert!(consistent(&world, cx));
}
