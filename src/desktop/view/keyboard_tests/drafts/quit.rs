//! Quitting with drafts, by keyboard alone: with a draft's write held,
//! Quit now focused; and with a draft that cannot be saved, Cancel by
//! Enter and by the first Tab stop, the focus on the draft's Retry
//! wrapper (ringed, pressed by Enter or Space), and Quit anyway by Tab.

use gpui_kit::{TestAppContext, test::TestWindowExt};

use super::{Walk, kit_button};
use crate::desktop::{
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::{
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
/// Enter is Cancel, which focuses the draft's Retry wrapper, ringed and
/// pressed by Enter or Space;
/// Quit anyway, reached by Tab and pressed by Enter, quits.
#[gpui_kit::test]
async fn a_draft_not_saved_on_quit_by_keyboard(cx: &mut TestAppContext) {
    use manyhands::drafts::{DraftErrorKind, FailureAction, FailurePoint};

    use crate::desktop::view::operations::draft_flush_id;

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

    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("drafts-not-saved-dialog", cx));
    assert_eq!(phase(cx), QuitPhase::Running);
    walk.on(draft_flush_id(id, "retry"), Region::Operations, cx);
    // The focus is Retry's wrapper: its ring shows, and Enter and Space
    // each press Retry, which writes again.
    let retry_ring = format!("{}-focus-ring", draft_flush_id(id, "retry"));
    walk.ringed(retry_ring.clone(), cx);
    for key in ["enter", "space"] {
        let writes = walk.shell.backend.reads_seen(ReadKind::DraftWrite);
        walk.shell.press(key, cx);
        walk.shell.run(cx);
        assert!(
            walk.shell.backend.reads_seen(ReadKind::DraftWrite) > writes,
            "{key} presses Retry"
        );
        walk.ringed(retry_ring.clone(), cx);
    }
    walk.ring_goes_for_the_button(
        retry_ring.clone(),
        kit_button(draft_flush_id(id, "retry")),
        Region::Operations,
        cx,
    );

    // One Tab, Enter: the first Tab stop is Cancel; it does not quit.
    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.keys("tab", cx);
    walk.in_dialog("drafts-not-saved-cancel-button", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("drafts-not-saved-dialog", cx));
    assert_eq!(phase(cx), QuitPhase::Running);
    walk.on(draft_flush_id(id, "retry"), Region::Operations, cx);
    walk.ringed(retry_ring, cx);

    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.tab_in_dialog("drafts-not-saved-quit-button", cx);
    walk.enter(cx);
    assert_eq!(phase(cx), QuitPhase::Quit);
}
