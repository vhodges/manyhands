//! Focus in the resolve conflict sheet when a control goes by itself
//! (A21): Check again, pressed, waits disabled for its read and then goes
//! once the set settles, and focus stays in the sheet, where Tab reaches
//! Resume. And Ctrl+Enter in the Result editor, with a part not chosen,
//! saves nothing and leaves focus in the editor.

use gpui_kit::{Focusable, TestAppContext};
use manyhands::repository::OperationId;

use super::{
    PENDING_REVIEW, SYNCHRONIZATION, TICKET, Walk,
    canned::{external_read, repaired_read},
    shared_banner,
};
use crate::desktop::{
    services::requests::Intent,
    test_support::fixtures::Fixture,
    view::{conflict::OpenedSheet, harness::Shell, workspace::Region},
};

/// B4, A21, canned (decision 9): the sheet opened by keyboard from the
/// banner (`shared_banner`) reads a set that goes outside, as
/// `external_read`. Tab and Enter on Check again read it again, as
/// `repaired_read`: Check again goes, and focus stays in the sheet, where
/// Tab reaches Resume and Enter resumes the synchronization. The sheet
/// closes and focus is back in the content region.
#[gpui_kit::test]
async fn check_again_by_keyboard_keeps_focus_in_the_sheet(cx: &mut TestAppContext) {
    let walk = shared_banner(cx).await;
    walk.shell
        .backend
        .answer_next_conflict_read(external_read());
    walk.enter(cx);
    walk.sheet_drawn("conflict-check-again", cx).await;
    assert!(!walk.shell.present("conflict-save", cx), "no Save outside");
    walk.tab_in_sheet("conflict-check-again".into(), cx);

    walk.shell
        .backend
        .answer_next_conflict_read(repaired_read());
    walk.enter(cx);
    walk.sheet_drawn("conflict-resume", cx).await;
    assert!(!walk.shell.present("conflict-check-again", cx));
    walk.in_dialog("conflict-sheet", cx);
    walk.tab_in_sheet("conflict-resume".into(), cx);
    walk.enter(cx);
    let operation = OperationId::parse(SYNCHRONIZATION).unwrap();
    let resumes = walk.shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::OperationResume { operation_id, .. } if operation_id == operation
        )
    });
    assert_eq!(resumes.len(), 1, "Enter resumed the synchronization");
    walk.until_the_sheet_closes(cx).await;
    walk.region(Region::Content, cx);
}

/// Review I-2, B3, real local: Ctrl+Enter in the Result editor while the
/// title is not chosen saves nothing, and focus stays in the editor.
#[gpui_kit::test]
async fn ctrl_enter_with_a_part_unchosen_keeps_focus_in_the_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.conflict_locally_with_body(&a, TICKET, "Mine", Some("Mine body.\n"), "Theirs");
    let walk = Walk::over(Shell::open(fixture, cx), a);
    walk.keys("secondary-4", cx);
    walk.until_drawn((PENDING_REVIEW, 0usize), cx).await;
    walk.tab_to("tab", (PENDING_REVIEW, 0usize), cx);
    walk.enter(cx);
    walk.sheet_drawn("conflict-save", cx).await;
    assert_eq!(
        walk.shell.label("conflict-progress", cx).as_deref(),
        Some("0 of 2 chosen")
    );
    let editor = cx
        .update(|cx| {
            cx.try_global::<OpenedSheet>()
                .and_then(|opened| opened.0.upgrade())
        })
        .and_then(|sheet| sheet.read_with(cx, |sheet, _| sheet.result_editor(0)))
        .expect("the Result editor");
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    let in_editor = |walk: &Walk, cx: &mut TestAppContext| {
        walk.shell
            .frame(cx, |window, cx| handle.contains_focused(window, cx))
    };
    // Tab does not reach the Result editor (reported to the orchestrator
    // with Task 15a's review fixes), so focus is put there as
    // `conflict_tests.rs` `ctrl_enter_saves_and_enter_is_a_newline` does.
    walk.shell.frame(cx, |window, cx| window.focus(&handle, cx));
    assert!(in_editor(&walk, cx));

    walk.keys("ctrl-enter", cx);
    let sent = walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::LocalContextResolve { .. })
    });
    assert!(sent.is_empty(), "nothing is saved");
    assert!(in_editor(&walk, cx), "focus stays in the editor");
}
