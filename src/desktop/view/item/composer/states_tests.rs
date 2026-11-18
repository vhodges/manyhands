//! The composer's events and states (design, Lane C; Task 11b): Submit,
//! Cancel and the discard prompt, the read-only `Posting` and `Failed`
//! states and the failed actions, the keys that stay in the editor and
//! the ones that leave it, Ctrl K's link field, and a click on a math
//! block. Each test that types sets the cursor, focuses the editor and
//! draws a frame first, then types through the window (D3's editor-test
//! rules). Texts are compared with `assert!`, never printed.

use gpui_kit::{ClipboardItem, TestAppContext, test::TestWindowExt};

use super::{
    ComposerState, FailedAction, FailedActions,
    test_bench::{Bench, Seen, Setup},
    toolbar::MiniControl,
};
use crate::desktop::keys::InsertLink;

/// One line of text.
const BODY: &str = "One two three";
/// A failed post's message.
const MESSAGE: &str = "The comment could not be saved.";
/// The failed row's controls, with what offers each.
const FAILED_CONTROLS: [&str; 7] = [
    "composer-retry",
    "composer-edit-text",
    "composer-set-identity",
    "composer-discard",
    "composer-check-again",
    "composer-copy-text",
    "composer-post-as-new",
];

fn with_body() -> Setup {
    Setup {
        restored: BODY.to_owned(),
        ..Setup::new()
    }
}

fn failed(actions: FailedActions, discard_note: Option<&str>) -> ComposerState {
    ComposerState::Failed {
        message: MESSAGE.to_owned(),
        actions,
        discard_note: discard_note.map(str::to_owned),
    }
}

/// Whether `actions` offers the control `id`.
fn offers(actions: FailedActions, id: &str) -> bool {
    match id {
        "composer-retry" => actions.retry,
        "composer-edit-text" => actions.edit_text,
        "composer-set-identity" => actions.set_identity,
        "composer-discard" => actions.discard,
        "composer-check-again" => actions.check_again,
        "composer-copy-text" => actions.copy_text,
        "composer-post-as-new" => actions.post_as_new,
        _ => unreachable!(),
    }
}

/// Ctrl Enter and the post button emit `Submit` for a text, in a new
/// composer and a reply, and change nothing of the text; an empty or
/// blank composer submits nothing.
#[gpui_kit::test]
fn ctrl_enter_submits_and_an_empty_composer_does_not(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    bench.place(0, cx);
    bench.keys("secondary-enter", cx);
    bench.click("composer-post", cx);
    assert_eq!(bench.events(), [], "an empty composer submits nothing");
    bench.input("   ", cx);
    bench.keys("secondary-enter", cx);
    assert_eq!(bench.events(), [], "nor a blank one");

    for setup in [with_body(), with_body().reply()] {
        let case = format!("reply {}", setup.reply);
        bench.reset(setup, cx);
        bench.place(BODY.len(), cx);
        bench.keys("secondary-enter", cx);
        assert_eq!(bench.events(), [Seen::Submit], "{case}: Ctrl Enter");
        assert!(bench.text(cx) == BODY, "{case}: no line was added");
        assert!(bench.editor_focused(cx), "{case}: focus stays");
        bench.forget();
        bench.click("composer-post", cx);
        assert_eq!(bench.events(), [Seen::Submit], "{case}: the button");
        assert!(!bench.read_only(cx), "{case}: the owner sets Posting");
    }
}

/// `Posting` is read-only: typing, deleting, pasting, undo, formatting
/// and Ctrl K change nothing, and nothing is emitted. Its button says
/// "Posting…"; Submit is refused with "Already posting" (P10), and Escape
/// and the reply's Cancel do nothing. The caret still moves. `Idle` again,
/// the text is editable.
#[gpui_kit::test]
fn posting_is_read_only_and_submits_nothing(cx: &mut TestAppContext) {
    let bench = Bench::open(with_body().reply(), cx);
    bench.set_state(ComposerState::Posting, cx);
    assert!(bench.read_only(cx));
    assert_eq!(
        bench.label("composer-post", cx).as_deref(),
        Some("Posting\u{2026}")
    );
    assert!(!bench.present("composer-cancel", cx), "no Cancel");
    assert!(!bench.present("composer-refused", cx));

    cx.write_to_clipboard(ClipboardItem::new_string("pasted".to_owned()));
    bench.place(3, cx);
    bench.input("x", cx);
    for keys in [
        "backspace",
        "delete",
        "enter",
        "ctrl-v",
        "ctrl-x",
        "ctrl-z",
        "ctrl-b",
        "tab",
    ] {
        bench.place(3, cx);
        bench.keys(keys, cx);
        assert!(bench.text(cx) == BODY, "{keys} changed nothing");
    }
    bench.select(4, 7, cx);
    for control in [MiniControl::Bold, MiniControl::Bullets, MiniControl::Link] {
        bench.click(control.id(), cx);
        assert!(bench.text(cx) == BODY, "{control:?} changed nothing");
    }
    assert!(!bench.present("composer-link-field", cx), "no link field");
    bench.place(3, cx);
    bench.dispatch(&InsertLink, cx);
    assert!(!bench.present("composer-link-field", cx), "nor from Ctrl K");
    assert_eq!(bench.changes(), 0, "no change was emitted");

    bench.place(3, cx);
    bench.keys("right", cx);
    assert_eq!(bench.cursor(cx), 4, "the caret moves");

    bench.keys("secondary-enter", cx);
    bench.click("composer-post", cx);
    assert_eq!(bench.events(), [], "no Submit while posting");
    assert_eq!(
        bench.label("composer-refused", cx).as_deref(),
        Some("Already posting")
    );
    bench.place(3, cx);
    bench.keys("escape", cx);
    assert_eq!(bench.events(), [], "Escape does nothing");
    assert!(!bench.present("composer-discard-dialog", cx));

    bench.set_state(ComposerState::Idle, cx);
    assert!(!bench.present("composer-refused", cx), "said once");
    assert!(bench.present("composer-cancel", cx));
    bench.place(0, cx);
    bench.input("x", cx);
    assert!(bench.text(cx) == format!("x{BODY}"), "editable again");
    assert_eq!(bench.changes(), 1);
}

/// `Failed` shows the owner's message in red and exactly the actions it
/// offers, no post button, and is read-only. Retry, Set identity, Check
/// again and Post as a new comment are told to the owner, the state
/// unchanged; Copy text puts
/// the text on the clipboard; Discard asks, with the owner's note; Edit
/// text is `Idle` again, focused and editable. Escape asks to discard
/// only when Discard is offered. Tab from the read-only editor reaches the
/// first action, and Shift-Tab goes back to the toolbar's stop.
#[gpui_kit::test]
fn failed_offers_exactly_the_actions_its_owner_sets(cx: &mut TestAppContext) {
    let bench = Bench::open(with_body(), cx);
    let sets = [
        FailedActions {
            edit_text: true,
            ..FailedActions::default()
        },
        FailedActions {
            retry: true,
            discard: true,
            copy_text: true,
            ..FailedActions::default()
        },
        FailedActions {
            set_identity: true,
            retry: true,
            ..FailedActions::default()
        },
        FailedActions {
            check_again: true,
            post_as_new: true,
            ..FailedActions::default()
        },
        FailedActions {
            discard: true,
            ..FailedActions::default()
        },
    ];
    for actions in sets {
        bench.set_state(failed(actions, None), cx);
        assert_eq!(bench.label("composer-failed", cx).as_deref(), Some(MESSAGE));
        for id in FAILED_CONTROLS {
            assert_eq!(
                bench.present(id, cx),
                offers(actions, id),
                "{actions:?}: {id}"
            );
        }
        assert!(!bench.present("composer-post", cx), "{actions:?}: no post");
        bench.place(0, cx);
        bench.input("x", cx);
        assert!(bench.text(cx) == BODY, "{actions:?}: read-only");
    }

    let all = FailedActions {
        retry: true,
        edit_text: true,
        set_identity: true,
        discard: true,
        check_again: true,
        copy_text: true,
        post_as_new: true,
    };
    bench.set_state(failed(all, None), cx);
    for (id, action) in [
        ("composer-retry", FailedAction::Retry),
        ("composer-set-identity", FailedAction::SetIdentity),
        ("composer-check-again", FailedAction::CheckAgain),
        ("composer-post-as-new", FailedAction::PostAsNew),
    ] {
        bench.forget();
        bench.click(id, cx);
        assert_eq!(bench.events(), [Seen::Failed(action)], "{id}");
        assert!(bench.read_only(cx), "{id}: still failed");
    }

    // Tab and Shift-Tab leave the read-only editor, which they cannot
    // indent: on to the first action, back to the toolbar's stop.
    bench.place(3, cx);
    bench.keys("tab", cx);
    assert!(
        bench.focused_id("composer-retry-button", cx),
        "Tab reached Retry"
    );
    bench.place(3, cx);
    bench.keys("shift-tab", cx);
    assert!(
        bench.focused(MiniControl::Bold, cx),
        "Shift-Tab: the toolbar"
    );
    assert!(bench.text(cx) == BODY, "nothing was indented");

    bench.forget();
    bench.click("composer-copy-text", cx);
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert!(copied.as_deref() == Some(BODY), "the text was copied");
    assert_eq!(bench.events(), [], "Copy text tells no one");

    let note = "This comment may still be saved later if it was written.";
    bench.set_state(failed(all, Some(note)), cx);
    bench.click("composer-discard", cx);
    assert_eq!(bench.events(), [], "Discard asks first");
    assert_eq!(
        bench.label("composer-discard-note", cx).as_deref(),
        Some(note)
    );
    bench.click("composer-discard-confirm", cx);
    assert_eq!(bench.events(), [Seen::Discard]);
    assert!(!bench.present("composer-discard-dialog", cx));

    // Escape asks only when Discard is offered.
    bench.set_state(failed(all, None), cx);
    bench.place(0, cx);
    bench.forget();
    bench.keys("escape", cx);
    assert!(bench.present("composer-discard-dialog", cx));
    bench.click("composer-discard-keep", cx);
    let retry = FailedActions {
        retry: true,
        ..FailedActions::default()
    };
    bench.set_state(failed(retry, None), cx);
    bench.place(0, cx);
    bench.keys("escape", cx);
    assert!(!bench.present("composer-discard-dialog", cx));
    assert_eq!(bench.events(), []);

    bench.set_state(failed(all, None), cx);
    bench.click("composer-edit-text", cx);
    assert_eq!(bench.events(), [Seen::Failed(FailedAction::EditText)]);
    assert!(!bench.read_only(cx), "Idle again");
    assert!(!bench.present("composer-failed", cx));
    assert!(bench.present("composer-post", cx));
    assert!(bench.editor_focused(cx), "focus is in the editor");
    bench.place(0, cx);
    bench.input("x", cx);
    assert!(bench.text(cx) == format!("x{BODY}"), "editable");
}

/// Escape with text asks "Discard this comment?" (a reply's, "Discard
/// this reply?"); Keep editing, by click or Escape, closes the prompt at
/// once, keeps the text and gives focus back to the editor; Discard emits
/// `Discard`. The reply's Cancel asks the same.
#[gpui_kit::test]
fn esc_with_text_asks_to_discard_and_keep_editing_keeps_it(cx: &mut TestAppContext) {
    let bench = Bench::open(with_body(), cx);
    bench.place(3, cx);
    bench.keys("escape", cx);
    assert_eq!(
        bench.label("composer-discard-dialog", cx).as_deref(),
        Some("Discard this comment?")
    );
    assert_eq!(bench.events(), [], "nothing yet");
    // Drawn "Discard · Keep editing", in that order in the tree too; Keep
    // editing has focus as it opens, and Tab goes on to Discard.
    let left = |id: &'static str, cx: &mut TestAppContext| {
        bench.shell.frame(cx, move |window, _| {
            window.try_find(id).expect("drawn").bounds().origin.x
        })
    };
    assert!(
        left("composer-discard-confirm", cx) < left("composer-discard-keep", cx),
        "Discard is drawn first"
    );
    assert!(
        bench.focused_id("composer-discard-keep-stop", cx),
        "Keep editing has focus as the prompt opens"
    );
    bench.keys("tab", cx);
    assert!(
        bench.focused_id("composer-discard-confirm-button", cx),
        "Tab reaches Discard"
    );
    bench.click("composer-discard-keep", cx);
    assert!(!bench.present("composer-discard-dialog", cx), "closed");
    assert!(bench.text(cx) == BODY, "the text is kept");
    assert!(bench.editor_focused(cx), "focus is back in the editor");
    assert_eq!(bench.events(), []);

    // Enter as the prompt opens keeps the text.
    bench.keys("escape", cx);
    assert!(bench.present("composer-discard-dialog", cx));
    bench.press("enter", cx);
    assert!(!bench.present("composer-discard-dialog", cx), "Enter keeps");
    assert!(bench.text(cx) == BODY);
    assert!(bench.editor_focused(cx));
    assert_eq!(bench.events(), []);

    bench.keys("escape", cx);
    assert!(bench.present("composer-discard-dialog", cx));
    bench.keys("escape", cx);
    assert!(
        !bench.present("composer-discard-dialog", cx),
        "Escape keeps"
    );
    assert!(bench.text(cx) == BODY);
    assert!(bench.editor_focused(cx));
    assert_eq!(bench.events(), []);

    bench.keys("escape", cx);
    bench.click("composer-discard-confirm", cx);
    assert!(!bench.present("composer-discard-dialog", cx));
    assert_eq!(bench.events(), [Seen::Discard]);

    bench.reset(with_body().reply(), cx);
    bench.click("composer-cancel", cx);
    assert_eq!(
        bench.label("composer-discard-dialog", cx).as_deref(),
        Some("Discard this reply?")
    );
    bench.click("composer-discard-confirm", cx);
    assert_eq!(bench.events(), [Seen::Discard]);
}

/// Escape in an empty composer, and the empty reply's Cancel, emit
/// `Cancel` with no prompt.
#[gpui_kit::test]
fn esc_when_empty_cancels_silently(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    bench.place(0, cx);
    bench.keys("escape", cx);
    assert!(!bench.present("composer-discard-dialog", cx));
    assert_eq!(bench.events(), [Seen::Cancel]);

    bench.reset(Setup::new().reply(), cx);
    bench.click("composer-cancel", cx);
    assert!(!bench.present("composer-discard-dialog", cx));
    assert_eq!(bench.events(), [Seen::Cancel]);
    bench.forget();
    bench.place(0, cx);
    bench.keys("escape", cx);
    assert_eq!(bench.events(), [Seen::Cancel]);
}

/// Decision 19: Tab and Shift-Tab indent and outdent a list item in the
/// composer's editor, and focus stays there.
#[gpui_kit::test]
fn tab_indents_inside_the_composer(cx: &mut TestAppContext) {
    const LIST: &str = "- one\n- two\n";
    let bench = Bench::open(
        Setup {
            restored: LIST.to_owned(),
            ..Setup::new()
        },
        cx,
    );
    bench.place(LIST.find("- two").unwrap(), cx);
    bench.keys("tab", cx);
    assert!(bench.text(cx) == "- one\n    - two\n", "Tab indented");
    assert!(bench.editor_focused(cx), "Tab stayed in the editor");
    bench.keys("shift-tab", cx);
    assert!(bench.text(cx) == LIST, "Shift-Tab outdented");
    assert!(bench.editor_focused(cx), "Shift-Tab stayed in the editor");
}

/// Decision 19: F6 and Shift-F6 leave the composer, from its editor and
/// from a toolbar control, and change nothing.
#[gpui_kit::test]
fn f6_leaves_the_composer(cx: &mut TestAppContext) {
    let bench = Bench::open(with_body(), cx);
    let (before, after) = bench
        .stage
        .read_with(cx, |stage, _| (stage.before.clone(), stage.after.clone()));
    for (keys, to) in [("f6", &after), ("shift-f6", &before)] {
        bench.place(3, cx);
        bench.keys(keys, cx);
        assert!(bench.is_focused(to, cx), "{keys} left the editor");
        bench.focus_control(MiniControl::Italic, cx);
        bench.keys(keys, cx);
        assert!(bench.is_focused(to, cx), "{keys} left the toolbar");
    }
    assert!(bench.text(cx) == BODY);
    assert_eq!(bench.events(), []);
}

/// Decision 21: Ctrl K's action in the composer's editor opens the Link
/// control's address field, focused, and Enter there makes the selection a
/// link.
#[gpui_kit::test]
fn insert_link_opens_the_link_field(cx: &mut TestAppContext) {
    let bench = Bench::open(with_body(), cx);
    bench.select(4, 7, cx);
    bench.dispatch(&InsertLink, cx);
    assert!(bench.link_field_focused(cx), "the address field");
    bench.input("https://example.test", cx);
    bench.press("enter", cx);
    assert!(!bench.present("composer-link-field", cx), "closed");
    assert!(bench.text(cx) == "One [two](https://example.test) three");
}

/// A body with a `$$` block between paragraphs.
const BLOCKS: &str = "Before the math.
$$
x^2 + y^2
$$
After the math.

The last paragraph.
";

/// R-66 (11a review I1): a click on a math block lands the caret after
/// it, as the edit host's does, though the click comes from below.
#[gpui_kit::test]
fn a_click_on_math_lands_after_it(cx: &mut TestAppContext) {
    let at = |needle: &str| BLOCKS.find(needle).unwrap();
    let bench = Bench::open(
        Setup {
            restored: BLOCKS.to_owned(),
            ..Setup::new()
        },
        cx,
    );
    assert!(bench.text(cx) == BLOCKS);
    bench.place(at("The last paragraph."), cx);
    let block = bench.point(at("x^2"), cx);
    bench.click_at(block, cx);
    assert_eq!(bench.cursor(cx), at("After the math."), "after the block");
    bench.keys("up", cx);
    assert_eq!(
        bench.cursor(cx),
        at("Before the math.") + "Before the math.".len(),
        "and before it, going up"
    );
    assert!(bench.text(cx) == BLOCKS);
}
