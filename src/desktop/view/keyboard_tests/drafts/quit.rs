//! Quitting with drafts, by keyboard alone: with a draft's write held,
//! Quit now focused; and with a draft that cannot be saved, Cancel by
//! Enter and by the first Tab stop, the focus on the list at the draft's
//! Flush row (Retry pressed by Enter, or by Enter or Space on its button),
//! and Quit anyway by Tab.

use gpui_kit::{TestAppContext, test::TestWindowExt};

use super::Walk;
use crate::desktop::{
    model::activity::rows::RowKey,
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::{
        activity_support::{button_focus, highlighted, list_focused},
        harness::Shell,
        quit::{QUIT_NOW, QuitPhase, SAVING_DRAFTS},
        workspace::Region,
    },
};

/// `secondary-q` with a draft's newest text unsaved and its write held:
/// "Saving drafts for recovery", with Quit now holding focus and drawing
/// its ring; Enter on it quits at once.
#[gpui_kit::test]
async fn quit_with_a_held_write_by_keyboard(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    // No editor yet: the session's unsaved text is made as D3's editor
    // will make it, through the registry.
    shell.unsaved_draft(cx).await;
    let gate = shell.backend.hold_read(ReadKind::DraftWrite);
    let a = shell.fixture.data_dir().to_owned();
    let walk = Walk::over(shell, a);

    walk.keys("secondary-q", cx);
    let phase = |cx: &mut TestAppContext| {
        walk.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase())
    };
    assert_eq!(phase(cx), QuitPhase::SavingDrafts);
    assert_eq!(
        walk.shell.label("quit-finishing", cx).as_deref(),
        Some(SAVING_DRAFTS)
    );
    assert_eq!(walk.shell.label("quit-now", cx).as_deref(), Some(QUIT_NOW));
    walk.shell.frame(cx, |window, _| {
        let quit_now = window.find("quit-now");
        assert_eq!(quit_now.focused(), Some(true), "Quit now holds focus");
        assert!(quit_now.visible(), "Quit now is on screen");
        assert!(window.find("quit-now-focus-ring").visible());
    });
    walk.enter(cx);
    assert_eq!(phase(cx), QuitPhase::Quit);
    drop(gate);
}

/// `secondary-q` when a draft cannot be saved: the dialog holds focus;
/// Enter is Cancel, which highlights the draft's Flush row and focuses the
/// list (C3); Enter on the list presses its Retry, and Tab reaches Retry's
/// button, where Enter and Space each press it again;
/// Quit anyway, reached by Tab and pressed by Enter, quits.
#[gpui_kit::test]
async fn a_draft_not_saved_on_quit_by_keyboard(cx: &mut TestAppContext) {
    use manyhands::drafts::{DraftErrorKind, FailureAction, FailurePoint};

    let shell = Shell::open(Fixture::new(), cx);
    let id = shell.unsaved_draft(cx).await;
    // Clones share the failure point: the window's store fails too.
    let _failing = shell.fixture.draft_store().with_failure_point_for_testing(
        FailurePoint::CreateTemp,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );
    let a = shell.fixture.data_dir().to_owned();
    let walk = Walk::over(shell, a);
    let phase = |cx: &mut TestAppContext| {
        walk.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase())
    };
    let flush = RowKey::Flush(id);
    // The list holds focus, on the draft's Flush row.
    let on_the_row = |cx: &mut TestAppContext| {
        walk.region(Region::Operations, cx);
        assert!(list_focused(&walk.shell, cx), "the list holds focus");
        assert_eq!(highlighted(&walk.shell, cx), Some(flush.clone()));
    };

    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("drafts-not-saved-dialog", cx));
    assert_eq!(phase(cx), QuitPhase::Running);
    on_the_row(cx);
    // Enter on the list presses the row's button, Retry, which writes
    // again; Tab reaches the button, where Enter and Space each press it.
    let writes = |walk: &Walk| walk.shell.backend.reads_seen(ReadKind::DraftWrite);
    let before = writes(&walk);
    walk.shell.press("enter", cx);
    walk.shell.run(cx);
    assert!(writes(&walk) > before, "Enter on the list presses Retry");
    walk.keys("tab", cx);
    walk.on_handle(&button_focus(&walk.shell, cx), Region::Operations, cx);
    for key in ["enter", "space"] {
        let before = writes(&walk);
        walk.shell.press(key, cx);
        walk.shell.run(cx);
        assert!(writes(&walk) > before, "{key} presses Retry");
    }

    // One Tab, Enter: the first Tab stop is Cancel; it does not quit.
    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.keys("tab", cx);
    walk.in_dialog("drafts-not-saved-cancel-button", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("drafts-not-saved-dialog", cx));
    assert_eq!(phase(cx), QuitPhase::Running);
    on_the_row(cx);

    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.tab_in_dialog("drafts-not-saved-quit-button", cx);
    walk.enter(cx);
    assert_eq!(phase(cx), QuitPhase::Quit);
}
