//! Focus in the resolve conflict sheet when a control goes by itself
//! (A21): Check again, pressed, waits disabled for its read and then goes
//! once the set settles, and focus stays in the sheet, where Tab reaches
//! Resume. The Result editor's tab stop (R-31): Tab reaches it after the
//! choices, Enter goes in, Escape comes back, and the edited Result is
//! saved. And Ctrl+Enter in the Result editor, with a part not chosen,
//! saves nothing and leaves focus in the editor.

use gpui_kit::{FocusHandle, Focusable, TestAppContext};
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

/// The Result's tab stop of the first body (R-31).
const RESULT: (&str, usize) = ("conflict-result", 0);

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

/// A local conflict whose title and body both sides changed, real, after a
/// restart: `secondary-4`, Tab and Enter on its pending row's Review
/// changes open the sheet, with nothing chosen. Gives the walk and the
/// Result editor's own focus.
async fn body_sheet(cx: &mut TestAppContext) -> (Walk, FocusHandle) {
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
    (walk, handle)
}

/// Whether focus is in the editor whose focus is `handle`.
fn in_editor(walk: &Walk, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
    walk.shell
        .frame(cx, |window, cx| handle.contains_focused(window, cx))
}

/// R-31: Tab reaches the Result's tab stop, which holds focus while no
/// region does; Enter on it puts focus in the editor, the sheet open.
fn into_the_result(walk: &Walk, handle: &FocusHandle, cx: &mut TestAppContext) {
    walk.tab_in_sheet(RESULT.into(), cx);
    assert!(!in_editor(walk, handle, cx), "the stop, not the editor");
    walk.enter(cx);
    assert!(
        in_editor(walk, handle, cx),
        "Enter puts focus in the editor"
    );
    assert!(walk.shell.present("conflict-sheet", cx));
}

/// R-31, B3, real local: by keyboard alone, the title's Keep mine and the
/// body's Keep both are chosen by Tab and Enter; Tab then reaches the
/// Result's tab stop, after the choices, and Enter goes into the editor,
/// where typing edits the Result. Escape comes back to the stop and leaves
/// the sheet open; Tab goes on to Save resolution, and Enter saves the
/// edited Result, which applies.
#[gpui_kit::test]
async fn edit_the_result_by_keyboard(cx: &mut TestAppContext) {
    let (walk, handle) = body_sheet(cx).await;
    walk.tab_in_sheet(("conflict-keep-mine", 0usize).into(), cx);
    walk.enter(cx);
    walk.tab_in_sheet(("conflict-body-both", 0usize).into(), cx);
    walk.enter(cx);
    assert_eq!(
        walk.shell.label("conflict-progress", cx).as_deref(),
        Some("2 of 2 chosen")
    );

    into_the_result(&walk, &handle, cx);
    walk.type_text("Added by keyboard", cx);
    walk.keys("escape", cx);
    assert!(walk.shell.present("conflict-sheet", cx), "the sheet stays");
    walk.in_dialog(RESULT, cx);

    walk.keys("tab", cx);
    walk.in_dialog("conflict-save", cx);
    walk.enter(cx);
    let sent = walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::LocalContextResolve { .. })
    });
    assert_eq!(sent.len(), 1, "Enter saved");
    let operations = walk.shell.operations(cx);
    let intent = operations.read_with(cx, |operations, _| {
        operations.request(sent[0]).unwrap().intent.clone()
    });
    let Intent::LocalContextResolve { resolutions, .. } = intent else {
        panic!("a local resolution");
    };
    assert!(
        resolutions[0].source.contains("Added by keyboard"),
        "the edited Result is saved"
    );
    walk.until_the_sheet_closes(cx).await;
    walk.region(Region::Operations, cx);
}

/// Review I-2, B3, real local: Ctrl+Enter in the Result editor, reached by
/// Tab and Enter (R-31), while the title is not chosen saves nothing, and
/// focus stays in the editor.
#[gpui_kit::test]
async fn ctrl_enter_with_a_part_unchosen_keeps_focus_in_the_editor(cx: &mut TestAppContext) {
    let (walk, handle) = body_sheet(cx).await;
    into_the_result(&walk, &handle, cx);

    walk.keys("ctrl-enter", cx);
    let sent = walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::LocalContextResolve { .. })
    });
    assert!(sent.is_empty(), "nothing is saved");
    assert!(in_editor(&walk, &handle, cx), "focus stays in the editor");
}
