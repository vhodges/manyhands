//! The item header's lifecycle action (D5 design, A1, A2): it opens the
//! dialog where the item has an ID and the tab can act, gives focus back to
//! itself when the dialog closes, and is disabled, saying why, where the
//! item has no ID (Task 5 review Minor 1, R-18).

use gpui_kit::TestAppContext;

use super::tests::{NEW_TICKET, TICKET, open, pane, shell, ticket, until_present};
use crate::desktop::{
    model::{item::CLOSE_TICKET, navigation::OpenTarget},
    services::{JobKind, ReadKind},
    test_support::fixtures::ticket_path,
    view::{
        harness::key,
        item::{LIFECYCLE_ACTION, LIFECYCLE_NOTE, NO_ID, NOT_CREATED},
    },
};

/// Carried from Task 5 (review Minor 1): a nonconforming file, and a new
/// item not created yet, have no ID, so the lifecycle action stays, with
/// its label, but is disabled, saying why; pressed, it opens nothing.
#[gpui_kit::test]
async fn a_tab_with_no_id_opens_no_lifecycle(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_ticket(root, TICKET);
    });
    let target = OpenTarget::Inspect {
        repo: key(&root),
        worktree: key(&root),
        path: ticket_path(TICKET),
    };
    open(&shell, target, cx);
    assert_eq!(
        shell.label(LIFECYCLE_ACTION, cx).as_deref(),
        Some(CLOSE_TICKET)
    );
    assert_eq!(shell.label(LIFECYCLE_NOTE, cx).as_deref(), Some(NO_ID));
    // Disabled: Tab passes it by (review O-4).
    assert!(!shell.tab_reaches(LIFECYCLE_ACTION, 12, cx));
    shell.click(LIFECYCLE_ACTION, cx);
    assert!(!shell.present("lifecycle-dialog", cx));

    // A real not-created tab: the new item's draft, continued.
    shell
        .edit_new_ticket(&root, NEW_TICKET, "Not yet", cx)
        .await;
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Not yet"));
    assert_eq!(
        shell.label(LIFECYCLE_ACTION, cx).as_deref(),
        Some(CLOSE_TICKET)
    );
    assert_eq!(
        shell.label(LIFECYCLE_NOTE, cx).as_deref(),
        Some(NOT_CREATED)
    );
    shell.click(LIFECYCLE_ACTION, cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0);
}

/// Review M-6: a conforming file read by its path has its ID from the
/// read: its action says nothing of an ID, and pressed, it opens the dialog.
#[gpui_kit::test]
async fn a_conforming_inspect_tab_opens_the_dialog(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Readable", "");
    });
    let target = OpenTarget::Inspect {
        repo: key(&root),
        worktree: key(&root),
        path: ticket_path(TICKET),
    };
    open(&shell, target, cx);
    assert!(!shell.present(LIFECYCLE_NOTE, cx));
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-title", cx).await;
    assert!(shell.present("lifecycle-dialog", cx));
}

/// Review M-2 (A2): closing the dialog gives focus back to the header's
/// action.
#[gpui_kit::test]
async fn focus_returns_to_the_action_when_the_dialog_closes(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Focused", "");
    });
    open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-dialog", cx).await;
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    let focus = pane(&shell, cx).read_with(cx, |pane, _| pane.lifecycle_focus());
    assert!(shell.frame(cx, |window, _| focus.is_focused(window)));
}

/// Review M-5: while the tab cannot act on a press (here its Edit prepares
/// the item's working copy), the action is drawn disabled, and a press
/// opens nothing.
#[gpui_kit::test]
async fn the_action_waits_while_the_tab_is_busy(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Busy", "");
    });
    open(&shell, ticket(&root), cx);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    assert!(shell.inert_button(LIFECYCLE_ACTION, cx));
    assert!(!shell.tab_reaches(LIFECYCLE_ACTION, 12, cx));
    shell.click(LIFECYCLE_ACTION, cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    held.release();
    shell.until_editing(cx).await;
    assert_eq!(shell.backend.reads_seen(ReadKind::Prepare), 0);
}
