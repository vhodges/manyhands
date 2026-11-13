//! "Retiring…" in the item's tabs (D6 Task 11b, decision 19): while the
//! item's local copy is being retired every tab of it is read-only, as it is
//! while its close or merge is unfinished, with its own words. Keyed by
//! (root, kind, id) and counted per request, over the real worker, the
//! request model and the in-flight record, held at the start of the job so
//! nothing reaches a remote (decision 9, `answer_job`).

use std::path::Path;

use gpui_kit::{
    Entity, TestAppContext, VisualTestContext, WindowHandle, component::Root, test::TestWindowExt,
};
use manyhands::{
    canonical::ItemId,
    repository::{
        ContextRetirementState as State, ItemDtoKind, RequestId, RetirementRemoval as Removal,
    },
    results::ResultCode as Code,
};

use super::{
    lock_tests::{LOCKED_NOTE, assert_edits_live, assert_edits_wait},
    tests::{DOCUMENT, TICKET, open, saved_ticket, shell, ticket},
};
use crate::desktop::{
    model::{lifecycle_lock::LockSource, lifecycle_lock::LockState, polling::TargetKind},
    services::{
        JobKind, Services,
        requests::{DesktopRequestId, Lane},
    },
    test_support::{drafts::store_draft, fixtures::Fixture, retire::answer},
    view::{
        drafts::row_id,
        harness::{Shell, key},
        item::{RETIRING, locked_text},
        workspace::Workspace,
    },
};

fn services(shell: &Shell, cx: &mut TestAppContext) -> Services {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone())
}

fn item() -> ItemId {
    TICKET.parse().unwrap()
}

/// Where the ticket of `root` stands under every lock.
fn lock(
    shell: &Shell,
    root: &Path,
    kind: ItemDtoKind,
    id: &str,
    cx: &mut TestAppContext,
) -> LockState {
    let services = services(shell, cx);
    let root = key(root);
    cx.read(|cx| services.lock(&root, kind, id, cx))
}

fn retiring() -> LockState {
    LockState::Locked(LockSource::Retiring)
}

/// Draws frames until the ticket of `root` stands as `expected`.
async fn until_lock(shell: &Shell, root: &Path, expected: LockState, cx: &mut TestAppContext) {
    let root = root.to_owned();
    shell
        .until_drawn(cx, &format!("the lock {expected:?}"), move |shell, cx| {
            lock(shell, &root, ItemDtoKind::Ticket, TICKET, cx) == expected
        })
        .await;
}

/// A retirement's answer that ends its entry and leaves nothing pending.
fn refused() -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    answer(State::Refused, Code::ExternalChange, Removal::None)
}

fn pending() -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    answer(State::Pending, Code::RecoveryRequired, Removal::None)
}

/// Sends a retirement of the ticket the way each path does.
fn send(
    shell: &Shell,
    root: &Path,
    lane: Lane,
    request: Option<RequestId>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let root = key(root);
    services(shell, cx)
        .scheduler()
        .update(cx, |scheduler, _| match request {
            Some(request) => {
                scheduler.resend_retirement(request, &root, TargetKind::Ticket, &item(), lane)
            }
            None => scheduler
                .retire(&root, TargetKind::Ticket, &item(), lane)
                .expect("not quitting"),
        })
}

/// The label of `id` in `window`, after a frame.
fn label_in(window: WindowHandle<Root>, id: &str, cx: &mut TestAppContext) -> Option<String> {
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    let id = gpui_kit::ElementId::from(id.to_owned());
    visual.update(|window, cx| {
        window.render_frame(cx);
        window
            .try_find(id)
            .and_then(|element| element.label().map(str::to_owned))
    })
}

/// Decision 19: a retirement of the item makes every tab of it read-only,
/// saying "Retiring…". It is keyed by (root, kind, id); it holds for a loop
/// item, a Retire…, a Resume resend and an Unlock… resend, and while the
/// retirement is pending; it covers a tab opened meanwhile and Restore from
/// the Drafts page; and two requests for one item do not clear each other's
/// share.
#[gpui_kit::test]
async fn retire_tabs_read_only(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut draft = None;
    let (shell, root) = shell(cx, |fixture: &Fixture, root| {
        saved_ticket(fixture, root, "Retire", "");
        draft = Some(store_draft(
            fixture,
            root,
            ItemDtoKind::Ticket,
            TICKET,
            "Mine",
        ));
    });
    let draft = draft.unwrap();
    // The scheduler hears the drain only with polling on (R-27); the record
    // of a pending retirement is its.
    let _ = services(&shell, cx).with_polling();
    open(&shell, ticket(&root), cx);
    assert_eq!(
        lock(&shell, &root, ItemDtoKind::Ticket, TICKET, cx),
        LockState::Open
    );
    assert_edits_live(&shell, cx);

    // A loop item (background), a Retire… (user lane) and an Unlock…
    // resend (user lane, a new request): the worker holds each, whatever
    // the lane. Neither another kind nor another item is held.
    for (what, lane) in [
        ("a loop item", Lane::Background),
        ("a Retire…", Lane::User),
        ("an Unlock… resend", Lane::User),
    ] {
        let held = shell.backend.hold_job(JobKind::ContextRetire);
        shell.backend.answer_job(JobKind::ContextRetire, refused());
        let request = (what == "an Unlock… resend").then(RequestId::new);
        send(&shell, &root, lane, request, cx);
        until_lock(&shell, &root, retiring(), cx).await;
        assert_edits_wait(&shell, RETIRING, cx);
        assert_eq!(
            lock(&shell, &root, ItemDtoKind::Document, TICKET, cx),
            LockState::Open,
            "{what}: another kind of the same ID"
        );
        assert_eq!(
            lock(&shell, &root, ItemDtoKind::Ticket, DOCUMENT, cx),
            LockState::Open,
            "{what}: another item"
        );
        // Its own words, not the lifecycle's.
        assert_eq!(locked_text(retiring(), ItemDtoKind::Ticket), RETIRING);
        assert_eq!(locked_text(retiring(), ItemDtoKind::Document), RETIRING);
        held.release();
        until_lock(&shell, &root, LockState::Open, cx).await;
        assert_edits_live(&shell, cx);
    }

    // Pending: the request has ended and the worker holds nothing, but the
    // in-flight record keeps the entry, so the tab stays read-only, as it
    // does after a relaunch. A Resume resend of the same request settles it.
    shell.backend.answer_job(JobKind::ContextRetire, pending());
    let first = send(&shell, &root, Lane::Background, None, cx);
    shell.until_ended(first, cx).await;
    shell.run(cx);
    assert!(!services(&shell, cx).worker().in_flight(|_| true));
    assert_eq!(
        lock(&shell, &root, ItemDtoKind::Ticket, TICKET, cx),
        retiring(),
        "pending: held by the record alone"
    );
    assert_edits_wait(&shell, RETIRING, cx);

    // A tab of the item opened in a second window during it says so too,
    // and Restore from the Drafts page is refused with the same words.
    let handle = services(&shell, cx);
    let second = cx.update(|cx| crate::desktop::open_main_window(handle, cx));
    let other: Entity<Workspace> = second
        .update(cx, |root, _, _| root.view().clone().downcast().unwrap())
        .unwrap();
    other.update(cx, |workspace, cx| workspace.open_item(ticket(&root), cx));
    VisualTestContext::from_window(second.into(), cx).run_until_parked();
    assert_eq!(
        label_in(second, LOCKED_NOTE, cx).as_deref(),
        Some(RETIRING),
        "a tab opened during it"
    );
    assert_eq!(label_in(second, "item-pill", cx).as_deref(), Some(RETIRING));
    shell.keys("secondary-shift-d", cx);
    shell.click(row_id(draft, "restore"), cx);
    shell.run(cx);
    assert_eq!(
        shell.label(row_id(draft, "notice"), cx).as_deref(),
        Some(RETIRING),
        "Restore is refused"
    );

    let entry = services(&shell, cx)
        .scheduler()
        .read_with(cx, |scheduler, _| {
            scheduler.store().retirements()[0].request().unwrap()
        });
    shell.backend.answer_job(JobKind::ContextRetire, refused());
    let resume = send(&shell, &root, Lane::User, Some(entry), cx);
    shell.until_ended(resume, cx).await;
    until_lock(&shell, &root, LockState::Open, cx).await;
    assert_eq!(label_in(second, LOCKED_NOTE, cx), None, "and lifts there");

    // Two requests for one item: one ending clears nothing of the other's.
    let (one, two) = (
        shell.backend.hold_job(JobKind::ContextRetire),
        shell.backend.hold_job(JobKind::ContextRetire),
    );
    for _ in 0..2 {
        shell.backend.answer_job(JobKind::ContextRetire, refused());
    }
    let sent = [
        send(&shell, &root, Lane::Background, None, cx),
        send(&shell, &root, Lane::User, None, cx),
    ];
    until_lock(&shell, &root, retiring(), cx).await;
    // The user lane runs first; whichever runs, the other still holds.
    let ran_first = if services(&shell, cx)
        .operations()
        .read_with(cx, |operations, _| {
            operations
                .running()
                .is_some_and(|request| request.id == sent[1])
        }) {
        sent[1]
    } else {
        sent[0]
    };
    one.release();
    shell.until_ended(ran_first, cx).await;
    shell.run(cx);
    assert_eq!(
        lock(&shell, &root, ItemDtoKind::Ticket, TICKET, cx),
        retiring(),
        "the other request still holds it"
    );
    two.release();
    until_lock(&shell, &root, LockState::Open, cx).await;

    // A mark the loop (or a Retire…) made before the request is out: two
    // marks of one item, one cleared.
    let item = (key(&root), ItemDtoKind::Ticket, TICKET.to_owned());
    let operations = services(&shell, cx).operations().clone();
    let (a, b) = operations.update(cx, |operations, _| {
        (
            operations.mark_retiring(item.clone()),
            operations.mark_retiring(item.clone()),
        )
    });
    until_lock(&shell, &root, retiring(), cx).await;
    operations.update(cx, |operations, _| assert!(operations.unmark_retiring(a)));
    shell.run(cx);
    assert_eq!(
        lock(&shell, &root, ItemDtoKind::Ticket, TICKET, cx),
        retiring(),
        "one mark ended, the other holds"
    );
    operations.update(cx, |operations, _| assert!(operations.unmark_retiring(b)));
    until_lock(&shell, &root, LockState::Open, cx).await;
}
