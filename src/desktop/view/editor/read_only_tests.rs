//! The read-only guard (R-61, Review Focus 5) over a real, focused,
//! painted editor holding a paragraph, a link, a checkbox, a table, a `$$`
//! block and a property block. Each test asserts that the text is
//! unchanged and that the `ReadOnly` wrapper emitted nothing; the
//! keystroke tests also assert that no edit reached the editor at all.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use gpui_kit::{
    ClipboardItem, Entity, EntityInputHandler, FocusHandle, Focusable, Modifiers, MouseButton,
    Pixels, Point, RenderImage, TestAppContext, point, px,
};
use zorite_editor::{EditorEvent, EditorState};

use super::ReadOnly;
use crate::desktop::model::editor_text::load;
use crate::desktop::view::{
    editor::tests::{Host, events, focus_and_draw, show},
    harness::Shell,
};

const BODY: &str = "A paragraph to read, with [a link](https://example.com/a) in it.

- [ ] a task to do

| Name | Count |
| --- | --- |
| one | 1 |

Before the math.
$$
x^2 + y^2
$$
After the math.

Before the properties.
status:: open
After the properties.

The last paragraph.
";

/// Where `needle` starts in `BODY`.
fn at(needle: &str) -> usize {
    BODY.find(needle).unwrap()
}

/// Where `needle` ends in `BODY`: for a whole line, the line's end.
fn end_of(needle: &str) -> usize {
    at(needle) + needle.len()
}

struct Reader {
    shell: Shell,
    host: Entity<Host<ReadOnly>>,
    editor: Entity<EditorState>,
    /// What the `ReadOnly` wrapper emitted.
    emitted: Rc<RefCell<Vec<EditorEvent>>>,
    /// What the editor inside it emitted.
    inner: Rc<RefCell<Vec<EditorEvent>>>,
    /// What it must show: `load` of its body.
    shown: String,
}

impl Reader {
    /// A read editor over `BODY`, its caret at `cursor`, focused and
    /// painted, with nothing recorded yet.
    fn open(cursor: usize, cx: &mut TestAppContext) -> Self {
        Self::over(BODY, cursor, cx)
    }

    /// A read editor over `body`, as `open` makes one over `BODY`.
    fn over(body: &str, cursor: usize, cx: &mut TestAppContext) -> Self {
        let (shell, host) = show(cx, |window, cx| ReadOnly::new(body, window, cx));
        let read_only = host.read_with(cx, |host, _| host.view.clone());
        let editor = read_only.read_with(cx, |read_only, _| read_only.editor().clone());
        let emitted = events(&read_only, cx);
        let inner = events(&editor, cx);
        let reader = Self {
            shell,
            host,
            editor,
            emitted,
            inner,
            shown: load(body),
        };
        reader.place(cursor, cx);
        reader
    }

    /// Puts the caret at `cursor`, focuses the editor and paints it, then
    /// forgets what the editor emitted so far.
    fn place(&self, cursor: usize, cx: &mut TestAppContext) {
        self.editor
            .update(cx, |editor, cx| editor.set_cursor(cursor, cx));
        focus_and_draw(&self.shell, &self.editor, cx);
        self.inner.borrow_mut().clear();
    }

    fn text(&self, cx: &mut TestAppContext) -> String {
        self.editor
            .read_with(cx, |editor, _| editor.text().to_owned())
    }

    fn cursor(&self, cx: &mut TestAppContext) -> usize {
        self.editor.read_with(cx, |editor, _| editor.cursor())
    }

    fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        self.shell.visual(cx).simulate_keystrokes(keystrokes);
        self.shell.frame(cx, |_, _| ());
    }

    fn input(&self, text: &str, cx: &mut TestAppContext) {
        self.shell.visual(cx).simulate_input(text);
        self.shell.frame(cx, |_, _| ());
    }

    /// The middle of the caret's box at `offset`, from a fresh paint.
    fn point(&self, offset: usize, cx: &mut TestAppContext) -> Point<Pixels> {
        let editor = self.editor.clone();
        self.shell
            .frame(cx, move |_, cx| editor.read(cx).bounds_for_offset(offset))
            .unwrap_or_else(|| panic!("offset {offset} is not laid out"))
            .center()
    }

    /// Presses and releases `button` at `position`, then paints.
    fn press(&self, position: Point<Pixels>, button: MouseButton, cx: &mut TestAppContext) {
        self.shell.frame(cx, |_, _| ());
        let mut visual = self.shell.visual(cx);
        visual.simulate_mouse_move(position, None, Modifiers::none());
        visual.simulate_mouse_down(position, button, Modifiers::none());
        visual.simulate_mouse_up(position, button, Modifiers::none());
        self.shell.frame(cx, |_, _| ());
    }

    /// How many edits reached the editor since `place`.
    fn edits(&self) -> usize {
        self.inner
            .borrow()
            .iter()
            .filter(|event| **event == EditorEvent::Changed)
            .count()
    }

    fn is_focused(&self, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
        let handle = handle.clone();
        self.shell
            .frame(cx, move |window, _| handle.is_focused(window))
    }

    fn editor_focus(&self, cx: &mut TestAppContext) -> FocusHandle {
        self.editor
            .read_with(cx, |editor, cx| editor.focus_handle(cx))
    }

    fn before(&self, cx: &mut TestAppContext) -> FocusHandle {
        self.host.read_with(cx, |host, _| host.before.clone())
    }

    fn after(&self, cx: &mut TestAppContext) -> FocusHandle {
        self.host.read_with(cx, |host, _| host.after.clone())
    }

    fn clipboard(&self, cx: &mut TestAppContext) -> Option<String> {
        cx.read_from_clipboard().and_then(|item| item.text())
    }

    /// The text is still what the reader loaded, and the wrapper emitted
    /// nothing.
    fn assert_unchanged(&self, cx: &mut TestAppContext) {
        assert!(
            self.text(cx) == self.shown,
            "the read editor's text changed"
        );
        assert!(
            self.emitted.borrow().is_empty(),
            "the ReadOnly wrapper emitted an event"
        );
    }
}

#[gpui_kit::test]
fn typing_enter_backspace_and_delete_change_nothing(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    reader.input("xyz", cx);
    reader.keys("shift-q space enter backspace delete", cx);
    reader.assert_unchanged(cx);
    assert_eq!(reader.edits(), 0, "an edit reached the editor");
    assert_eq!(reader.cursor(cx), 5);
}

#[gpui_kit::test]
fn paste_and_cut_change_nothing_and_copy_still_copies(cx: &mut TestAppContext) {
    let reader = Reader::open(at("paragraph"), cx);
    cx.write_to_clipboard(ClipboardItem::new_string("PASTED".to_owned()));
    reader.keys("ctrl-v cmd-v", cx);
    reader.assert_unchanged(cx);

    reader.keys(&["shift-right"; 9].join(" "), cx);
    reader.keys("ctrl-x cmd-x", cx);
    reader.assert_unchanged(cx);
    assert_eq!(reader.clipboard(cx).as_deref(), Some("PASTED"));

    reader.keys("ctrl-c", cx);
    assert_eq!(reader.clipboard(cx).as_deref(), Some("paragraph"));
    reader.assert_unchanged(cx);
    assert_eq!(reader.edits(), 0, "an edit reached the editor");
}

/// The undo keys find nothing to undo here: this read editor has taken no
/// edit. After an edit lands, its exact revert is one undo step (R-79), and
/// an undo of it would land like any edit, for the backstop to put back.
/// So the undo half proves that undo changes nothing, not that the `Undo`
/// and `Redo` stops fire.
#[gpui_kit::test]
fn the_formatting_keys_and_undo_change_nothing(cx: &mut TestAppContext) {
    let reader = Reader::open(at("paragraph"), cx);
    reader.keys(&["shift-right"; 9].join(" "), cx);
    for keys in [
        "ctrl-b",
        "ctrl-i",
        "ctrl-e",
        "ctrl-shift-x",
        "ctrl-u",
        "cmd-b",
        "cmd-i",
        "cmd-e",
        "cmd-shift-x",
        "cmd-u",
        "ctrl-z",
        "ctrl-shift-z",
        "ctrl-y",
        "cmd-z",
        "cmd-shift-z",
    ] {
        reader.keys(keys, cx);
        reader.assert_unchanged(cx);
    }
    assert_eq!(reader.edits(), 0, "an edit reached the editor");
}

/// An input method calls the editor's input handler directly, past every
/// key listener (fact 76): the commit lands, and the guard reverts it
/// before anything is painted.
#[gpui_kit::test]
fn an_input_method_commit_is_reverted(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    let editor = reader.editor.clone();
    reader.shell.frame(cx, move |window, cx| {
        editor.update(cx, |editor, cx| {
            editor.replace_text_in_range(None, "漢字", window, cx)
        })
    });
    assert_eq!(reader.edits(), 1, "the commit did not reach the editor");
    reader.assert_unchanged(cx);
    assert_eq!(reader.cursor(cx), 5);
}

#[gpui_kit::test]
fn an_input_method_composition_is_reverted_and_leaves_no_marked_text(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    let editor = reader.editor.clone();
    reader.shell.frame(cx, move |window, cx| {
        editor.update(cx, |editor, cx| {
            editor.replace_and_mark_text_in_range(None, "か", Some(1..1), window, cx)
        })
    });
    assert_eq!(
        reader.edits(),
        1,
        "the composition did not reach the editor"
    );
    reader.assert_unchanged(cx);
    assert_eq!(reader.cursor(cx), 5);
    let editor = reader.editor.clone();
    let marked = reader.shell.frame(cx, move |window, cx| {
        editor.update(cx, |editor, cx| editor.marked_text_range(window, cx))
    });
    assert_eq!(marked, None, "marked text was left behind");

    let editor = reader.editor.clone();
    reader.shell.frame(cx, move |window, cx| {
        editor.update(cx, |editor, cx| {
            editor.replace_text_in_range(None, "火", window, cx)
        })
    });
    reader.assert_unchanged(cx);
}

/// The box sits in the gutter, left of the item's text by the editor's
/// list gap (8 px); a press there toggles it, focused or not
/// (`lib.rs:2929-2944`). Fragile by nature; checklist step 2 also covers
/// it.
#[gpui_kit::test]
fn a_checkbox_click_is_reverted(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    let text = reader.point(at("a task"), cx);
    reader.press(point(text.x - px(10.), text.y), MouseButton::Left, cx);
    assert_eq!(reader.edits(), 1, "the press did not toggle the box");
    reader.assert_unchanged(cx);
}

/// A right press would open the editor's menus, and a middle press would
/// paste where the platform does; both also focus the editor, so focus
/// staying where it was shows the press was stopped.
#[gpui_kit::test]
fn a_right_or_middle_press_is_stopped(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    let before = reader.before(cx);
    let editor = reader.editor_focus(cx);
    let target = reader.point(at("to read"), cx);
    for button in [MouseButton::Right, MouseButton::Middle] {
        reader.shell.frame(cx, {
            let before = before.clone();
            move |window, cx| window.focus(&before, cx)
        });
        reader.press(target, button, cx);
        assert!(reader.is_focused(&before, cx), "{button:?} moved focus");
        assert!(!reader.is_focused(&editor, cx), "{button:?} moved focus");
        reader.assert_unchanged(cx);
    }
    assert_eq!(reader.cursor(cx), 5);
}

#[gpui_kit::test]
fn select_all_copy_and_the_arrows_work(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    reader.keys("right", cx);
    assert_eq!(reader.cursor(cx), 6);
    reader.keys("left", cx);
    assert_eq!(reader.cursor(cx), 5);
    reader.keys("down", cx);
    assert_eq!(
        reader.cursor(cx),
        at("\n") + 1,
        "down reaches the next line"
    );
    reader.keys("up", cx);
    assert_eq!(reader.cursor(cx), 5);

    reader.keys("shift-right shift-right shift-right ctrl-c", cx);
    assert_eq!(reader.clipboard(cx).as_deref(), Some(&BODY[5..8]));
    reader.keys("ctrl-a ctrl-c", cx);
    assert_eq!(reader.clipboard(cx).as_deref(), Some(BODY));
    reader.assert_unchanged(cx);
}

/// R-66: forward moves and clicks land after a block, backward moves
/// before it. A click comes from below here, where the editor's own
/// `at_end` reads like a backward move.
#[gpui_kit::test]
fn the_caret_steps_over_math_and_property_blocks(cx: &mut TestAppContext) {
    let reader = Reader::open(at("Before the math."), cx);
    for (before, after, inside) in [
        ("Before the math.", "After the math.", "x^2"),
        ("Before the properties.", "After the properties.", "status"),
    ] {
        reader.place(at(before), cx);
        reader.keys("down", cx);
        assert_eq!(reader.cursor(cx), at(after), "down over {inside}");
        reader.keys("up", cx);
        assert_eq!(reader.cursor(cx), end_of(before), "up over {inside}");
        reader.keys("right", cx);
        assert_eq!(reader.cursor(cx), at(after), "right over {inside}");
        reader.keys("left", cx);
        assert_eq!(reader.cursor(cx), end_of(before), "left over {inside}");

        reader.place(at("The last paragraph."), cx);
        let block = reader.point(at(inside), cx);
        reader.press(block, MouseButton::Left, cx);
        assert_eq!(reader.cursor(cx), at(after), "a click on {inside}");
        reader.assert_unchanged(cx);
    }
}

#[gpui_kit::test]
fn tab_and_shift_tab_move_focus_out(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    let (before, after) = (reader.before(cx), reader.after(cx));
    reader.keys("tab", cx);
    assert!(reader.is_focused(&after, cx), "tab did not move focus on");
    reader.assert_unchanged(cx);

    reader.place(5, cx);
    reader.keys("shift-tab", cx);
    assert!(
        reader.is_focused(&before, cx),
        "shift-tab did not move focus back"
    );
    reader.assert_unchanged(cx);
}

#[gpui_kit::test]
fn a_link_click_still_emits_open_link(cx: &mut TestAppContext) {
    let reader = Reader::open(at("The last paragraph."), cx);
    let link = reader.point(at("a link") + 2, cx);
    reader.press(link, MouseButton::Left, cx);
    assert!(
        reader
            .inner
            .borrow()
            .contains(&EditorEvent::OpenLink("https://example.com/a".into())),
        "the click opened no link"
    );
    reader.assert_unchanged(cx);
}

/// R-19: a clean reload shows the new body, as the editor loads it, and
/// keeps the cursor, clamped; the guard then holds the new text.
#[gpui_kit::test]
fn a_reload_shows_the_new_body_and_keeps_the_cursor(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    let read_only = reader.host.read_with(cx, |host, _| host.view.clone());
    read_only.update(cx, |read_only, cx| {
        read_only.reload("Another body,\r\nlonger than five.", cx)
    });
    reader.shell.frame(cx, |_, _| ());
    assert_eq!(reader.text(cx), "Another body,\nlonger than five.");
    assert_eq!(
        read_only.read_with(cx, |read_only, cx| read_only.cursor(cx)),
        5
    );

    reader.input("typed", cx);
    assert_eq!(reader.text(cx), "Another body,\nlonger than five.");

    read_only.update(cx, |read_only, cx| read_only.reload("abc", cx));
    assert_eq!(reader.text(cx), "abc");
    assert_eq!(reader.cursor(cx), 3);
    assert!(reader.emitted.borrow().is_empty());
}

/// Fact 81's first shape, then a checkbox. The pin loads its loaded text
/// differently again, so a revert that loaded the shown text again would
/// never settle.
const LOADS_TWICE: &str = "wer $$\na $$b$$ c\n$$ wer\n\n- [ ] task\n";

/// R-79: the revert puts back exactly `load(body)`, and the update settles.
#[gpui_kit::test]
fn a_change_on_a_body_the_pin_loads_differently_twice_is_reverted_exactly(cx: &mut TestAppContext) {
    let reader = Reader::over(LOADS_TWICE, 0, cx);
    assert!(
        load(&reader.shown) != reader.shown,
        "the body no longer has fact 81's shape"
    );
    let task = reader.point(reader.shown.find("task").unwrap(), cx);
    reader.press(point(task.x - px(10.), task.y), MouseButton::Left, cx);
    assert_eq!(reader.edits(), 1, "the press did not toggle the box");
    reader.shell.run(cx);
    reader.assert_unchanged(cx);
}

/// A paragraph with inline math. The test gives the editor a math
/// renderer, so the formula is drawn and can be clicked; D3 installs
/// none, and without one a click there only places the caret.
const INLINE: &str = "Inline $a+b$ math here.\n\nThe end.\n";

/// R-66 for inline math: the caret lands after the formula on a forward
/// move or a click, and before it on a backward move.
#[gpui_kit::test]
fn the_caret_steps_over_inline_math(cx: &mut TestAppContext) {
    let reader = Reader::over(INLINE, 0, cx);
    reader.editor.update(cx, |editor, _| {
        let raster = Arc::new(RenderImage::new(vec![image::Frame::new(
            image::RgbaImage::new(8, 8),
        )]));
        editor.set_block_math_provider(move |_| Some((raster.clone(), 8., 8.)));
        editor.set_block_math_em(16.);
    });
    let (start, end) = (INLINE.find("$a").unwrap(), INLINE.find(" math").unwrap());

    reader.place(start, cx);
    reader.keys("right", cx);
    assert_eq!(reader.cursor(cx), end, "right over the formula");
    reader.keys("left", cx);
    assert_eq!(reader.cursor(cx), start, "left over the formula");

    reader.place(INLINE.find("The end.").unwrap(), cx);
    let formula = reader.point(start, cx);
    reader.press(point(formula.x + px(4.), formula.y), MouseButton::Left, cx);
    assert_eq!(reader.cursor(cx), end, "a click on the formula");
    reader.assert_unchanged(cx);
}

/// R-66 for word moves (Alt-Left and Alt-Right): they step over blocks
/// too. A backward one right after a click still lands before the block,
/// because the move clears the press.
#[gpui_kit::test]
fn word_moves_step_over_blocks(cx: &mut TestAppContext) {
    let reader = Reader::open(at("Before the math."), cx);
    for (before, after) in [
        ("Before the math.", "After the math."),
        ("Before the properties.", "After the properties."),
    ] {
        reader.place(end_of(before), cx);
        reader.keys("alt-right", cx);
        assert_eq!(reader.cursor(cx), at(after), "alt-right after {before}");

        let line = reader.point(at(after), cx);
        reader.press(line, MouseButton::Left, cx);
        assert_eq!(reader.cursor(cx), at(after), "the click on {after}");
        reader.keys("alt-left", cx);
        assert_eq!(reader.cursor(cx), end_of(before), "alt-left from {after}");
    }
    reader.assert_unchanged(cx);
}

/// The guard's `Undo` stop, proven (mutation: with it removed this fails): a checkbox click puts two
/// snapshots on the inner editor's history (the toggle, and the guard's
/// revert), so an undo that reached the editor would notify it. The undo
/// and redo keys, pressed after the click, notify it not at all and send no
/// `Changed`. The `Redo` stop cannot be
/// proven here: only a leaked undo fills the redo stack, and the guard's
/// revert then clears it, so without the stop the keys find nothing.
#[gpui_kit::test]
fn undo_and_redo_with_a_history_never_reach_the_editor(cx: &mut TestAppContext) {
    let reader = Reader::open(5, cx);
    let text = reader.point(at("a task"), cx);
    reader.press(point(text.x - px(10.), text.y), MouseButton::Left, cx);
    assert_eq!(reader.edits(), 1, "the press did not toggle the box");
    reader.assert_unchanged(cx);

    let notified = Rc::new(RefCell::new(0usize));
    cx.update(|cx| {
        let notified = notified.clone();
        cx.observe(&reader.editor, move |_, _| *notified.borrow_mut() += 1)
            .detach()
    });
    reader.inner.borrow_mut().clear();
    for keys in ["ctrl-z", "ctrl-shift-z", "ctrl-y", "cmd-z", "cmd-shift-z"] {
        reader.keys(keys, cx);
        reader.assert_unchanged(cx);
    }
    assert_eq!(*notified.borrow(), 0, "an undo or redo reached the editor");
    assert_eq!(reader.edits(), 0, "an edit reached the editor");
}
