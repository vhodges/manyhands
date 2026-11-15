//! Resolve conflict as a sheet the workspace keeps (Wave 04 Cycle 05b
//! design B1, B2; P5), written before its code (Task 5).
//!
//! Every way in opens it on Root's one sheet layer, owned by the
//! workspace (`SheetOwner::Resolve`), and `sync_sheet` keeps it frame
//! after frame; every way out clears the owner. The lifecycle dialog's ways
//! in go on only while focus is inside it (R-24): a press made in the frame
//! a prompt opens over it does nothing, and Repair locally…'s sheet waits
//! under the prompt until the dialog draws with focus back inside it
//! (U-10).
//!
//! A remote conflict is canned (D5 decision 9), as
//! `conflict/entry_tests.rs` cans it; the pending row's local conflict and
//! Repair locally…'s are real.

use gpui_kit::{
    App, InputEvent, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, TestAppContext, Window,
    component::{WindowExt, dialog::Dialog},
    test::TestWindowExt,
};
use manyhands::repository::{AuthoringKind, ConflictReadNextAction, OperationId, RequestId};

use super::super::settings_sheet::SheetOwnerKind;
use crate::desktop::{
    model::activity::rows::RowKey,
    services::{JobKind, requests::Intent},
    test_support::fixtures::item_id,
    view::{
        activity_support::{act, until_offered},
        conflict::{
            REVIEW_CHANGES,
            answer_tests::applied,
            entry_tests::{
                SYNCHRONIZATION, child_conflict, fabricate, row_of, share_conflict, ticket_shell,
            },
            tests::{
                TICKET, canned, canned_path, code, local_shell, opened, until_long, until_present,
            },
        },
        harness::Shell,
        item::LIFECYCLE_ACTION,
        needs_attention::tests::actions::press_from_popover,
        operations::OPEN_LIFECYCLE,
    },
};

/// The close a canned child conflict names (`child_conflict`).
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";

fn owner(shell: &Shell, cx: &mut TestAppContext) -> Option<SheetOwnerKind> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.sheet_owner())
}

/// After a frame: whether a sheet is active, and whether a dialog is.
fn layers(shell: &Shell, cx: &mut TestAppContext) -> (bool, bool) {
    shell.frame(cx, |window, cx| {
        (window.has_active_sheet(cx), window.has_active_dialog(cx))
    })
}

/// B1 case 1, opened by `way`: a sheet is active, no dialog is, and the
/// workspace keeps it as Resolve conflict's, frame after frame.
fn kept(shell: &Shell, way: &str, cx: &mut TestAppContext) {
    for _ in 0..3 {
        shell.run(cx);
        assert_eq!(
            layers(shell, cx),
            (true, false),
            "{way}: a sheet, no dialog"
        );
        assert_eq!(
            owner(shell, cx),
            Some(SheetOwnerKind::Resolve),
            "{way}: the workspace keeps it"
        );
    }
    assert!(shell.present("conflict-sheet", cx), "{way}: it is drawn");
}

/// B1 case 2, closed by `way`: no sheet is active, and the owner is
/// cleared.
fn cleared(shell: &Shell, way: &str, cx: &mut TestAppContext) {
    shell.run(cx);
    assert_eq!(layers(shell, cx), (false, false), "{way}: it closed");
    assert_eq!(owner(shell, cx), None, "{way}: the owner is cleared");
    assert!(!shell.present("conflict-sheet", cx), "{way}: not drawn");
}

/// A prompt over the lifecycle dialog, as a key's passphrase prompt opens
/// over it: another dialog, on top, which takes focus.
fn open_prompt(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, |dialog: Dialog, _, _| dialog.title("A prompt"));
}

/// A left click at `at`, dispatched to the frame drawn last. In the update
/// that opens a prompt, the prompt is not drawn yet, so the click reaches
/// the dialog under it, as a click made in the frame a prompt opens does.
fn press_at(window: &mut Window, at: Point<Pixels>, cx: &mut App) {
    let (button, modifiers) = (MouseButton::Left, Modifiers::default());
    let moved = MouseMoveEvent {
        position: at,
        pressed_button: None,
        modifiers,
    };
    let down = MouseDownEvent {
        button,
        position: at,
        modifiers,
        click_count: 1,
        first_mouse: false,
    };
    let up = MouseUpEvent {
        button,
        position: at,
        modifiers,
        click_count: 1,
    };
    window.dispatch_event(moved.to_platform_input(), cx);
    window.dispatch_event(down.to_platform_input(), cx);
    window.dispatch_event(up.to_platform_input(), cx);
}

/// P5 (B1, B2, B3's ways out, R-24, U-10): through every way in, Resolve
/// conflict is a sheet the workspace keeps; Escape, the close button,
/// Cancel, Resume and a save each clear the owner; the lifecycle's way in
/// does nothing under a prompt, and Repair locally…'s sheet opens only
/// once the prompt has closed, for the dialog to open again on.
#[gpui_kit::test]
async fn resolve_is_a_sheet_the_workspace_keeps(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = ticket_shell(cx);
    share_ways_in(&shell, &root, cx).await;
    lifecycle_way_in(&shell, &root, cx).await;
    pending_way_in(cx).await;
    repair_way_in(cx).await;
}

/// Canned: a share of the ticket stopped on a conflict, whose banner,
/// Activity entry and Needs attention row each offer Review changes.
/// Escape, the close button, Cancel and a save that applies close it.
async fn share_ways_in(shell: &Shell, root: &std::path::Path, cx: &mut TestAppContext) {
    let operation = OperationId::parse(SYNCHRONIZATION).unwrap();
    let read = || canned(operation, vec![canned_path(TICKET, '1')]);
    let share = fabricate(
        shell,
        1,
        Intent::ItemSync {
            root: root.to_owned(),
            kind: AuthoringKind::Ticket,
            id: item_id(TICKET),
        },
        share_conflict(),
        cx,
    );
    let entry = RowKey::Request(share);

    // The banner's, through the tab's route.
    until_present(shell, "item-review-changes", cx).await;
    shell.backend.answer_next_conflict_read(read());
    shell.click("item-review-changes", cx);
    until_present(shell, "conflict-save", cx).await;
    kept(shell, "the banner", cx);
    shell.keys("escape", cx);
    cleared(shell, "Escape", cx);

    // Activity's entry.
    until_offered(shell, &entry, REVIEW_CHANGES, cx).await;
    shell.backend.answer_next_conflict_read(read());
    act(shell, &entry, REVIEW_CHANGES, cx);
    until_present(shell, "conflict-save", cx).await;
    kept(shell, "an Activity row", cx);
    shell.frame(cx, |window, cx| {
        window.within("sheet-content").click("close", cx)
    });
    cleared(shell, "the close button", cx);

    // The Needs attention popover's row.
    shell.backend.answer_next_conflict_read(read());
    press_from_popover(shell, &entry, REVIEW_CHANGES, cx);
    until_present(shell, "conflict-save", cx).await;
    kept(shell, "a popover row", cx);
    shell.click("conflict-cancel", cx);
    cleared(shell, "Cancel", cx);

    // The banner again, and a save the library applies.
    shell.backend.answer_next_conflict_read(read());
    shell.click("item-review-changes", cx);
    until_present(shell, "conflict-save", cx).await;
    kept(shell, "the banner again", cx);
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    shell.click("conflict-save", cx);
    let sent = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ConflictResolve { .. })
    });
    assert_eq!(sent.len(), 1, "Save resolution sent one resolve");
    shell.until_ended(sent[0], cx).await;
    cleared(shell, "a save", cx);
}

/// Canned: a close stopped on its child's conflict, whose dialog offers
/// Review changes…. Pressed in the frame a prompt opens over the dialog,
/// it does nothing, then or once the prompt closes (R-24). Pressed with
/// focus inside the dialog, the sheet takes the dialog's place; its read
/// offers Resume, which closes it.
async fn lifecycle_way_in(shell: &Shell, root: &std::path::Path, cx: &mut TestAppContext) {
    let close = fabricate(
        shell,
        2,
        Intent::TicketClose {
            root: root.to_owned(),
            id: item_id(TICKET),
        },
        child_conflict(root, PARENT, RequestId::new()),
        cx,
    );
    act(shell, &RowKey::Request(close), OPEN_LIFECYCLE, cx);
    until_present(shell, "lifecycle-resolve-conflict", cx).await;

    let at = shell.frame(cx, |window, _| {
        window.find("lifecycle-resolve-conflict").bounds().center()
    });
    shell.visual(cx).update(|window, cx| {
        open_prompt(window, cx);
        press_at(window, at, cx);
    });
    shell.run(cx);
    assert_eq!(
        layers(shell, cx),
        (false, true),
        "under the prompt: no sheet"
    );
    assert!(shell.present("lifecycle-dialog", cx), "the dialog stays");
    assert_ne!(owner(shell, cx), Some(SheetOwnerKind::Resolve));
    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);
    assert_eq!(layers(shell, cx), (false, true), "nothing once it closed");
    assert!(shell.present("lifecycle-dialog", cx), "the dialog is back");

    let operation = OperationId::parse(PARENT).unwrap();
    let mut resumable = canned(operation, vec![canned_path(TICKET, '1')]);
    resumable.next_action = ConflictReadNextAction::Resume;
    shell.backend.answer_next_conflict_read(resumable);
    shell
        .visual(cx)
        .update(|window, cx| press_at(window, at, cx));
    until_present(shell, "conflict-resume", cx).await;
    kept(shell, "the lifecycle's Review changes…", cx);
    assert!(
        !shell.present("lifecycle-dialog", cx),
        "in the dialog's place"
    );
    shell.click("conflict-resume", cx);
    cleared(shell, "Resume", cx);
}

/// Real: after a restart, a local synchronization's pending row offers
/// Review changes.
async fn pending_way_in(cx: &mut TestAppContext) {
    let (shell, root, _) = local_shell(cx, |fixture, root| {
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    let pending = row_of(&shell, &root, "context_sync_local", cx).await;
    until_offered(&shell, &pending, REVIEW_CHANGES, cx).await;
    act(&shell, &pending, REVIEW_CHANGES, cx);
    until_present(&shell, "conflict-save", cx).await;
    kept(&shell, "a pending row", cx);
    shell.keys("escape", cx);
    cleared(&shell, "Escape from a pending row's", cx);
}

/// Real: a blocked prepare's Repair locally…, whose synchronization
/// stops on a conflict while a prompt is over the dialog. The sheet waits
/// on the dialog under the prompt, and opens in the dialog's place at the
/// first frame with focus back inside it (U-10), holding the dialog to
/// open again once its resolution applies (`reopen_after`).
async fn repair_way_in(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    shell.fixture.commit_ticket(&root, TICKET, "Moved", "");
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-repair-locally", cx).await;
    let held = shell.backend.hold_job(JobKind::LocalContextSync);
    shell.click("lifecycle-repair-locally", cx);
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::LocalContextSync { .. }))
        })
        .await;
    held.wait_reached();
    shell.visual(cx).update(open_prompt);
    held.release();
    let sync = shell.requests(cx, |request| {
        matches!(request.intent, Intent::LocalContextSync { .. })
    })[0];
    until_long(&shell, "the synchronization's answer", cx, |shell, cx| {
        code(shell, sync, cx).is_some()
    })
    .await;
    for _ in 0..3 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    assert_eq!(
        layers(&shell, cx),
        (false, true),
        "under the prompt: no sheet"
    );
    assert!(
        shell.present("lifecycle-dialog", cx),
        "it waits on the dialog"
    );

    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.frame(cx, |_, _| ());
    let opened_now = shell
        .visual(cx)
        .update(|window, cx| window.has_active_sheet(cx));
    assert!(
        opened_now,
        "the first frame with focus back inside opens it"
    );
    kept(&shell, "Repair locally…", cx);
    assert!(
        !shell.present("lifecycle-dialog", cx),
        "in the dialog's place"
    );
    let sheet = opened(cx).expect("the sheet is open");
    assert!(
        sheet.read_with(cx, |sheet, _| sheet.reopens()),
        "the dialog opens again once it applies"
    );
    shell.keys("escape", cx);
    cleared(&shell, "Escape from Repair locally…'s", cx);
}
