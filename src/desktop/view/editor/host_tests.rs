//! The edit editor's host (design, "The editor host"), built directly in
//! the test shell inside a stand-in for the item tab while editing: the
//! header's Done, a scroll region of its own, and the region keys the
//! workspace handles. Each test types through the window, after setting
//! the cursor, focusing the editor and drawing a frame.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui_kit::{
    Action, App, AppContext, ClipboardItem, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyContext, Keystroke, Modifiers, MouseButton, ParentElement,
    Pixels, Point, Render, SharedString, StatefulInteractiveElement, Styled, TestAppContext,
    TextStyle, Window, component::Theme, div, px,
};
use zorite_editor::{Dismiss, EditorState};

use super::{EditorHost, EditorHostEvent};
use crate::desktop::{
    keys::{
        EDIT_HOST_CONTEXT, EditorEscape, FocusNavigation, InsertLink, NextRegion, PreviousRegion,
        ToggleRich, bind,
    },
    model::editor_text::{load, tests as editor_text_tests},
    view::{
        editor::tests::{focus_and_draw, show},
        harness::Shell,
        item::tokens::item_colors,
        scroll::ContentScroll,
    },
};

/// The page's body size, which the item tab sets around the editor.
const PAGE_SIZE: f32 = 15.;
/// The scroll region's height and width.
const REGION: (f32, f32) = (200., 600.);

/// A short body of plain text.
const PLAIN: &str = "A paragraph to edit, short and plain.\n\nA second one.\n";

/// What an item tab gives the edit host while editing.
struct Tab {
    /// The header's Done, which Escape focuses.
    done: FocusHandle,
    host: Entity<EditorHost>,
    scroll: ContentScroll,
    /// Where the region keys move focus, as the workspace's would.
    next: FocusHandle,
    /// How often `Dismiss` was dispatched to the editor: a capture
    /// listener above the host sees each dispatch on its way down.
    dismissed: Rc<Cell<usize>>,
}

impl Tab {
    fn new(body: &str, cursor: Option<usize>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let done = cx.focus_handle().tab_stop(true);
        let scroll = ContentScroll::default();
        let host = host_in(body, cursor, &done, &scroll, window, cx);
        Self {
            done,
            host,
            scroll,
            next: cx.focus_handle().tab_stop(true),
            dismissed: Rc::default(),
        }
    }
}

/// A new host over `body`, placed under `done` in `scroll`'s region.
fn host_in(
    body: &str,
    cursor: Option<usize>,
    done: &FocusHandle,
    scroll: &ContentScroll,
    window: &mut Window,
    cx: &mut App,
) -> Entity<EditorHost> {
    cx.new(|cx| {
        let mut host = EditorHost::new(body, cursor, window, cx);
        host.place(done.clone(), scroll, window, cx);
        host
    })
}

impl Render for Tab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dismissed = self.dismissed.clone();
        let (height, width) = REGION;
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_size(px(PAGE_SIZE))
            .capture_action(move |_: &Dismiss, _, _| dismissed.set(dismissed.get() + 1))
            .on_action(cx.listener(|this, _: &NextRegion, window, cx| window.focus(&this.next, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousRegion, window, cx| window.focus(&this.next, cx)),
            )
            .on_action(
                cx.listener(|this, _: &FocusNavigation, window, cx| window.focus(&this.next, cx)),
            )
            .child(
                div()
                    .id("tab-done")
                    .track_focus(&self.done)
                    .h(px(24.))
                    .child("Done"),
            )
            .child(
                div()
                    .id("tab-region")
                    .flex()
                    .flex_col()
                    .flex_none()
                    .h(px(height))
                    .w(px(width))
                    .overflow_y_scroll()
                    .track_scroll(self.scroll.handle())
                    .child(div().flex_none().w_full().child(self.host.clone())),
            )
            .child(
                div()
                    .id("tab-next")
                    .track_focus(&self.next)
                    .h(px(24.))
                    .child("Next region"),
            )
    }
}

/// A host in the test shell, and what it emitted since the last `place`.
struct Editing {
    shell: Shell,
    tab: Entity<Tab>,
    events: Rc<RefCell<Vec<EditorHostEvent>>>,
}

impl Editing {
    /// A host over `body`, its caret at `cursor` (`EditorHost::new`'s),
    /// drawn, not yet focused.
    fn open(body: &str, cursor: Option<usize>, cx: &mut TestAppContext) -> Self {
        let (shell, outer) = show(cx, |window, cx| Tab::new(body, cursor, window, cx));
        let tab = outer.read_with(cx, |outer, _| outer.view.clone());
        let editing = Self {
            shell,
            tab,
            events: Rc::default(),
        };
        editing.watch(cx);
        editing
    }

    /// A new host over `body` in the last one's place.
    fn reopen(&self, body: &str, cursor: Option<usize>, cx: &mut TestAppContext) {
        let tab = self.tab.clone();
        let body = body.to_owned();
        self.shell.frame(cx, move |window, cx| {
            tab.update(cx, |tab, cx| {
                tab.host = host_in(&body, cursor, &tab.done, &tab.scroll, window, cx);
                cx.notify();
            })
        });
        self.watch(cx);
        self.shell.frame(cx, |_, _| ());
    }

    /// Records what the current host emits, from now on.
    fn watch(&self, cx: &mut TestAppContext) {
        self.events.borrow_mut().clear();
        let host = self.host(cx);
        let seen = self.events.clone();
        cx.update(|cx| {
            cx.subscribe(&host, move |_, event: &EditorHostEvent, _| {
                seen.borrow_mut().push(event.clone())
            })
            .detach()
        });
    }

    fn host(&self, cx: &mut TestAppContext) -> Entity<EditorHost> {
        self.tab.read_with(cx, |tab, _| tab.host.clone())
    }

    fn editor(&self, cx: &mut TestAppContext) -> Entity<EditorState> {
        let host = self.host(cx);
        host.read_with(cx, |host, _| host.editor().clone())
    }

    /// Puts the caret at `cursor`, focuses the editor and draws, then
    /// forgets what the host emitted so far.
    fn place(&self, cursor: usize, cx: &mut TestAppContext) {
        let editor = self.editor(cx);
        editor.update(cx, |editor, cx| editor.set_cursor(cursor, cx));
        focus_and_draw(&self.shell, &editor, cx);
        self.events.borrow_mut().clear();
    }

    fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        self.shell.visual(cx).simulate_keystrokes(keystrokes);
        self.shell.frame(cx, |_, _| ());
        self.shell.frame(cx, |_, _| ());
    }

    fn input(&self, text: &str, cx: &mut TestAppContext) {
        self.shell.visual(cx).simulate_input(text);
        self.shell.frame(cx, |_, _| ());
    }

    fn text(&self, cx: &mut TestAppContext) -> String {
        let host = self.host(cx);
        host.read_with(cx, |host, cx| host.text(cx))
    }

    fn cursor(&self, cx: &mut TestAppContext) -> usize {
        let editor = self.editor(cx);
        editor.read_with(cx, |editor, _| editor.cursor())
    }

    fn dirty(&self, cx: &mut TestAppContext) -> bool {
        let host = self.host(cx);
        host.read_with(cx, |host, _| host.is_dirty())
    }

    fn rich(&self, cx: &mut TestAppContext) -> bool {
        let host = self.host(cx);
        host.read_with(cx, |host, _| host.is_rich())
    }

    fn set_rich(&self, rich: bool, cx: &mut TestAppContext) {
        let host = self.host(cx);
        host.update(cx, |host, cx| host.set_rich(rich, cx));
        self.shell.frame(cx, |_, _| ());
    }

    fn events(&self) -> Vec<EditorHostEvent> {
        self.events.borrow().clone()
    }

    /// How many `Edited` the host emitted since `place`.
    fn edits(&self) -> usize {
        self.events
            .borrow()
            .iter()
            .filter(|event| **event == EditorHostEvent::Edited)
            .count()
    }

    fn dismissed(&self, cx: &mut TestAppContext) -> usize {
        self.tab.read_with(cx, |tab, _| tab.dismissed.get())
    }

    fn is_focused(&self, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
        let handle = handle.clone();
        self.shell
            .frame(cx, move |window, _| handle.is_focused(window))
    }

    fn editor_focused(&self, cx: &mut TestAppContext) -> bool {
        let editor = self.editor(cx);
        let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
        self.is_focused(&handle, cx)
    }

    fn done(&self, cx: &mut TestAppContext) -> FocusHandle {
        self.tab.read_with(cx, |tab, _| tab.done.clone())
    }

    fn next(&self, cx: &mut TestAppContext) -> FocusHandle {
        self.tab.read_with(cx, |tab, _| tab.next.clone())
    }

    fn focus(&self, handle: &FocusHandle, cx: &mut TestAppContext) {
        let handle = handle.clone();
        self.shell
            .frame(cx, move |window, cx| window.focus(&handle, cx));
        self.shell.frame(cx, |_, _| ());
    }

    /// The middle of the caret's box at `offset`, from a fresh paint.
    fn point(&self, offset: usize, cx: &mut TestAppContext) -> Point<Pixels> {
        let editor = self.editor(cx);
        self.shell
            .frame(cx, move |_, cx| editor.read(cx).bounds_for_offset(offset))
            .unwrap_or_else(|| panic!("offset {offset} is not laid out"))
            .center()
    }

    /// Presses and releases `button` at `position`, then draws.
    fn press(&self, position: Point<Pixels>, button: MouseButton, cx: &mut TestAppContext) {
        self.shell.frame(cx, |_, _| ());
        let mut visual = self.shell.visual(cx);
        visual.simulate_mouse_move(position, None, Modifiers::none());
        visual.simulate_mouse_down(position, button, Modifiers::none());
        visual.simulate_mouse_up(position, button, Modifiers::none());
        self.shell.frame(cx, |_, _| ());
    }

    /// The region's offset.
    fn offset(&self, cx: &mut TestAppContext) -> Pixels {
        self.tab
            .read_with(cx, |tab, _| tab.scroll.handle().offset().y)
    }

    /// Whether the caret's box lies inside the region, from a fresh paint.
    fn caret_in_view(&self, cx: &mut TestAppContext) -> bool {
        let editor = self.editor(cx);
        let scroll = self.tab.read_with(cx, |tab, _| tab.scroll.clone());
        self.shell.frame(cx, move |_, cx| {
            let region = scroll.handle().bounds();
            let caret = editor
                .read(cx)
                .caret_screen_bounds()
                .expect("the caret is laid out");
            caret.top() >= region.top() - px(1.) && caret.bottom() <= region.bottom() + px(1.)
        })
    }

    /// The text style the editor was last painted in.
    fn painted(&self, cx: &mut TestAppContext) -> TextStyle {
        self.shell.frame(cx, |_, _| ());
        let host = self.host(cx);
        host.read_with(cx, |host, _| host.painted_style())
            .expect("the host was painted")
    }
}

/// Where `needle` starts in `text`.
fn at(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is in the text"))
}

/// `text` with `inserted` at `offset`.
fn with(text: &str, offset: usize, inserted: &str) -> String {
    format!("{}{inserted}{}", &text[..offset], &text[offset..])
}

/// The mode key switches modes; neither the key nor the call touches the
/// text, and undo and redo reach across a switch: text typed in rich text,
/// then in Markdown, is undone back past the switch to the loaded text,
/// clean (Task 27 fix 1, C2).
#[gpui_kit::test]
fn a_mode_switch_keeps_the_text_and_the_undo_history(cx: &mut TestAppContext) {
    const BODY: &str = "# A heading\n\nA paragraph with **bold** and `code`.\n\n- an item\n";
    let editing = Editing::open(BODY, None, cx);
    assert!(editing.rich(cx), "editing starts in rich text");
    let start = at(BODY, "paragraph");
    editing.place(start, cx);
    editing.input("new", cx);
    let typed = with(BODY, start, "new");
    assert!(editing.text(cx) == typed, "the typing landed");

    editing.keys("secondary-shift-m", cx);
    assert!(!editing.rich(cx), "the key switched to Markdown");
    assert!(editing.text(cx) == typed, "the switch kept the text");
    assert!(editing.editor_focused(cx), "the editor kept focus");
    let item = at(&typed, "an item");
    editing.place(item, cx);
    editing.input("one more, ", cx);
    assert!(
        editing.text(cx) == with(&typed, item, "one more, "),
        "Markdown takes typing"
    );
    editing.keys("ctrl-z", cx);
    assert!(editing.text(cx) == typed, "undo in Markdown");
    editing.keys("ctrl-z", cx);
    assert!(editing.text(cx) == BODY, "undo reached across the switch");
    assert!(!editing.dirty(cx));
    editing.keys("ctrl-shift-z", cx);
    assert!(editing.text(cx) == typed, "redo too");

    editing.keys("secondary-shift-m", cx);
    assert!(editing.rich(cx), "the key switched back");
    editing.set_rich(false, cx);
    assert!(!editing.rich(cx));
    editing.set_rich(true, cx);
    assert!(editing.rich(cx));
    assert!(editing.text(cx) == typed, "the switches kept the text");
    editing.keys("ctrl-z", cx);
    assert!(editing.text(cx) == BODY, "undo after the switches");
}

/// Over every golden fixture: what is typed in rich text is all that
/// changes, a switch to Markdown and back changes nothing more, and undo
/// leads back to the loaded text, clean.
#[gpui_kit::test]
fn typing_in_rich_mode_then_switching_round_trips_each_fixture(cx: &mut TestAppContext) {
    let editing = Editing::open("", None, cx);
    for name in editor_text_tests::every_fixture() {
        let body = editor_text_tests::fixture(name);
        editing.reopen(&body, None, cx);
        let loaded = load(&body);
        assert!(editing.text(cx) == loaded, "{name}: loaded");
        assert!(!editing.dirty(cx), "{name}: clean when loaded");

        editing.place(loaded.len(), cx);
        editing.input("x", cx);
        let typed = format!("{loaded}x");
        assert!(editing.text(cx) == typed, "{name}: typed");
        editing.set_rich(false, cx);
        assert!(editing.text(cx) == typed, "{name}: in Markdown");
        editing.set_rich(true, cx);
        assert!(editing.text(cx) == typed, "{name}: back in rich text");
        assert!(editing.dirty(cx), "{name}: dirty");

        editing.keys("ctrl-z", cx);
        assert!(editing.text(cx) == loaded, "{name}: undone");
        assert!(!editing.dirty(cx), "{name}: clean again");
    }
}

/// R-12: undo emits no `Changed`, yet the host sees the text return to
/// its baseline and says it is clean; redo makes it dirty again. Each
/// difference is one `Edited`, and dirty is said only when it flips.
#[gpui_kit::test]
fn undo_back_to_the_baseline_makes_the_editor_clean(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    editing.place(5, cx);
    editing.input("abc", cx);
    assert!(editing.dirty(cx));
    assert_eq!(editing.edits(), 3, "one edit per typed character");
    assert_eq!(
        editing
            .events()
            .iter()
            .filter(|event| matches!(event, EditorHostEvent::DirtyChanged(_)))
            .collect::<Vec<_>>(),
        [&EditorHostEvent::DirtyChanged(true)]
    );

    editing.events.borrow_mut().clear();
    editing.keys("ctrl-z", cx);
    assert!(editing.text(cx) == PLAIN);
    assert!(!editing.dirty(cx), "undo to the baseline is clean");
    assert!(editing.events().contains(&EditorHostEvent::Edited));
    assert_eq!(
        editing.events().last(),
        Some(&EditorHostEvent::DirtyChanged(false))
    );

    editing.keys("ctrl-shift-z", cx);
    assert!(editing.dirty(cx), "redo is dirty");
    editing.keys("backspace backspace backspace", cx);
    assert!(editing.text(cx) == PLAIN);
    assert!(!editing.dirty(cx), "typing back to the baseline is clean");
}

/// R-69: a save moves the baseline to the saved text and nothing else, so
/// the editor is clean and its undo history is kept: undo then leads to a
/// text that differs from the saved one.
#[gpui_kit::test]
fn mark_saved_makes_the_editor_clean_and_keeps_undo(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    editing.place(5, cx);
    editing.input("abc", cx);
    let saved = editing.text(cx);
    assert!(editing.dirty(cx));

    let host = editing.host(cx);
    editing.events.borrow_mut().clear();
    host.update(cx, |host, cx| host.mark_saved(&saved, cx));
    editing.shell.frame(cx, |_, _| ());
    assert!(!editing.dirty(cx), "the saved text is clean");
    assert!(editing.text(cx) == saved, "the text is the editor's own");
    assert_eq!(editing.events(), [EditorHostEvent::DirtyChanged(false)]);
    assert_eq!(editing.cursor(cx), 8, "the caret stayed");

    editing.keys("ctrl-z", cx);
    assert!(editing.text(cx) == PLAIN, "undo was kept");
    assert!(
        editing.dirty(cx),
        "the loaded text is no longer the baseline"
    );
    editing.keys("ctrl-shift-z", cx);
    assert!(editing.text(cx) == saved);
    assert!(!editing.dirty(cx), "redo to the saved text is clean");
}

/// Bodies the load normalizes: CRLF line endings with a words-mixed `$$`
/// pair, and fact 81's two shapes, which a second pass would change
/// again. The loaded text is the baseline, and mode switches never load
/// it again, so nothing is dirty and nothing is edited.
#[gpui_kit::test]
fn a_normalized_body_is_not_dirty_after_load_and_mode_switches(cx: &mut TestAppContext) {
    let editing = Editing::open("", None, cx);
    let bodies = [(
        "CRLF and a mixed pair",
        "First\r\nWhat if words $$E=mc^2$$ more\r\n".to_owned(),
    )]
    .into_iter()
    .chain(
        editor_text_tests::SECOND_PASS
            .into_iter()
            .map(|name| (name, editor_text_tests::fixture(name))),
    );
    for (name, body) in bodies {
        editing.reopen(&body, None, cx);
        let loaded = load(&body);
        assert!(loaded != body, "{name}: the load normalizes it");
        assert!(editing.text(cx) == loaded, "{name}: loaded");
        editing.place(0, cx);
        for rich in [false, true, false, true] {
            editing.set_rich(rich, cx);
            assert!(editing.text(cx) == loaded, "{name}: rich {rich}");
            assert!(!editing.dirty(cx), "{name}: rich {rich}");
        }
        assert!(
            editing.events().is_empty(),
            "{name}: {:?}",
            editing.events()
        );
    }
}

/// Bold, Italic, Inline code and Strikethrough wrap the selection from
/// their keys, each one edit (fact 72).
#[gpui_kit::test]
fn bold_italic_code_and_strike_work_from_their_keys(cx: &mut TestAppContext) {
    const WORDS: &str = "one two three four\n";
    let editing = Editing::open(WORDS, None, cx);
    for (word, keys, wrapped) in [
        ("one", "ctrl-b", "**one**"),
        ("two", "ctrl-i", "*two*"),
        ("three", "ctrl-e", "`three`"),
        ("four", "ctrl-shift-x", "~~four~~"),
    ] {
        let start = at(&editing.text(cx), word);
        editing.place(start, cx);
        editing.keys(&vec!["shift-right"; word.len()].join(" "), cx);
        editing.keys(keys, cx);
        assert!(editing.text(cx).contains(wrapped), "{keys} gave {wrapped}");
        assert_eq!(editing.edits(), 1, "{keys} is one edit");
    }
    assert_eq!(editing.text(cx), "**one** *two* `three` ~~four~~\n");
    assert!(editing.dirty(cx));
}

/// R-8: underline writes HTML, so its keys do nothing here.
#[gpui_kit::test]
fn ctrl_u_does_not_underline(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    editing.place(2, cx);
    editing.keys("shift-right shift-right shift-right", cx);
    // Each on its own: a second underline would unwrap the first.
    for keys in ["ctrl-u", "cmd-u", "secondary-u"] {
        editing.keys(keys, cx);
        assert!(editing.text(cx) == PLAIN, "{keys} changed the text");
    }
    assert!(!editing.dirty(cx));
    assert_eq!(editing.edits(), 0, "an underline reached the text");
}

/// Tab reaches the editor through the host's tab stop. Inside it, Tab
/// and Shift-Tab indent and outdent and focus stays; F6, Shift-F6 and the
/// region keys leave it, as the workspace moves them.
#[gpui_kit::test]
fn tab_indents_inside_the_editor_and_f6_leaves_it(cx: &mut TestAppContext) {
    const LIST: &str = "- one\n- two\n";
    let editing = Editing::open(LIST, None, cx);
    let done = editing.done(cx);
    editing.focus(&done, cx);
    editing.keys("tab", cx);
    assert!(
        editing.editor_focused(cx),
        "Tab from Done reached the editor"
    );

    let second = at(LIST, "- two");
    editing.place(second, cx);
    editing.keys("tab", cx);
    assert!(editing.text(cx) == "- one\n    - two\n", "Tab indented");
    assert!(editing.editor_focused(cx), "Tab stayed in the editor");
    editing.keys("shift-tab", cx);
    assert!(editing.text(cx) == LIST, "Shift-Tab outdented");
    assert!(editing.editor_focused(cx), "Shift-Tab stayed in the editor");

    let next = editing.next(cx);
    for keys in ["f6", "shift-f6", "secondary-2"] {
        editing.place(second, cx);
        editing.keys(keys, cx);
        assert!(editing.is_focused(&next, cx), "{keys} left the editor");
        assert!(!editing.editor_focused(cx), "{keys} left the editor");
    }
    assert!(editing.text(cx) == LIST);
}

/// R-59: with no pointer press since the last key, Escape moves focus to
/// the header's Done and dismisses nothing; typing first changes nothing
/// of that.
#[gpui_kit::test]
fn escape_focuses_the_headers_done(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    let done = editing.done(cx);
    editing.place(5, cx);
    editing.keys("escape", cx);
    assert!(editing.is_focused(&done, cx), "Escape focused Done");
    assert_eq!(editing.dismissed(cx), 0, "Escape dismissed a menu");

    editing.place(5, cx);
    editing.input("x", cx);
    editing.keys("escape", cx);
    assert!(editing.is_focused(&done, cx), "Escape after typing");
    assert_eq!(editing.dismissed(cx), 0);
    assert!(
        editing.text(cx) == with(PLAIN, 5, "x"),
        "Escape kept the text"
    );
}

/// R-59: an editor menu opens only from a pointer press, so the first
/// Escape after one dispatches the editor's `Dismiss`, and focus stays;
/// the next one focuses Done. A left press that only placed the caret
/// counts too (the cost R-59 names), and so does a right press, which
/// opens the editor's menu. A key after the press, typing or a move,
/// clears it.
#[gpui_kit::test]
fn escape_after_a_pointer_press_dispatches_dismiss_and_a_second_focuses_done(
    cx: &mut TestAppContext,
) {
    let editing = Editing::open(PLAIN, None, cx);
    let done = editing.done(cx);
    let target = editing.point(at(PLAIN, "short"), cx);
    let mut dismissed = 0;
    for button in [MouseButton::Left, MouseButton::Right] {
        editing.place(0, cx);
        editing.press(target, button, cx);
        assert!(
            editing.editor_focused(cx),
            "{button:?}: the editor has focus"
        );
        editing.keys("escape", cx);
        dismissed += 1;
        assert_eq!(editing.dismissed(cx), dismissed, "{button:?}: Dismiss");
        assert!(editing.editor_focused(cx), "{button:?}: focus stayed");
        editing.keys("escape", cx);
        assert_eq!(editing.dismissed(cx), dismissed, "{button:?}: once");
        assert!(editing.is_focused(&done, cx), "{button:?}: then Done");
    }

    for key in ["typing", "right"] {
        editing.place(0, cx);
        editing.press(target, MouseButton::Left, cx);
        if key == "typing" {
            editing.input("x", cx);
        } else {
            editing.keys(key, cx);
        }
        editing.keys("escape", cx);
        assert_eq!(editing.dismissed(cx), dismissed, "{key} cleared the press");
        assert!(editing.is_focused(&done, cx), "{key}: Escape focused Done");
    }
}

/// A body with a `$$` block and a property block between paragraphs.
const BLOCKS: &str = "Before the math.
$$
x^2 + y^2
$$
After the math.

Before the properties.
status:: open
After the properties.

The last paragraph.
";

/// R-66: in rich text, forward moves and clicks land after a block and
/// backward moves before it, as in the read editor; a click comes from
/// below, where the editor's own `at_end` reads like a backward move. In
/// Markdown mode the caret enters the block, which is edited there.
#[gpui_kit::test]
fn the_caret_steps_over_math_and_property_blocks(cx: &mut TestAppContext) {
    let end_of = |needle: &str| at(BLOCKS, needle) + needle.len();
    let editing = Editing::open(BLOCKS, None, cx);
    for (before, after, inside) in [
        ("Before the math.", "After the math.", "x^2"),
        ("Before the properties.", "After the properties.", "status"),
    ] {
        editing.place(at(BLOCKS, before), cx);
        editing.keys("down", cx);
        assert_eq!(editing.cursor(cx), at(BLOCKS, after), "down over {inside}");
        editing.keys("up", cx);
        assert_eq!(editing.cursor(cx), end_of(before), "up over {inside}");
        editing.keys("right", cx);
        assert_eq!(editing.cursor(cx), at(BLOCKS, after), "right over {inside}");
        editing.keys("left", cx);
        assert_eq!(editing.cursor(cx), end_of(before), "left over {inside}");

        editing.place(at(BLOCKS, "The last paragraph."), cx);
        let block = editing.point(at(BLOCKS, inside), cx);
        editing.press(block, MouseButton::Left, cx);
        assert_eq!(editing.cursor(cx), at(BLOCKS, after), "a click on {inside}");
        editing.keys("up", cx);
        assert_eq!(editing.cursor(cx), end_of(before), "up after the click");

        // A backward move right after a click on text still lands before
        // the block: the move clears the press before the editor moves.
        let line = editing.point(at(BLOCKS, after), cx);
        editing.press(line, MouseButton::Left, cx);
        assert_eq!(
            editing.cursor(cx),
            at(BLOCKS, after),
            "the click on {after}"
        );
        editing.keys("up", cx);
        assert_eq!(editing.cursor(cx), end_of(before), "up from {after}");
    }
    assert!(editing.text(cx) == BLOCKS);
    assert!(!editing.dirty(cx));

    editing.set_rich(false, cx);
    editing.place(at(BLOCKS, "Before the math."), cx);
    editing.keys("down", cx);
    assert_eq!(
        editing.cursor(cx),
        end_of("Before the math.") + 1,
        "in Markdown the caret enters the block"
    );
    editing.input("y", cx);
    assert!(editing.text(cx) == with(BLOCKS, end_of("Before the math.") + 1, "y"));
}

/// Markdown mode sets the host's font to the theme's monospace family at
/// 13.5 px in the page text colour; rich text takes the page's own. The
/// test platform has no fonts, so the Kit names the UI font for both
/// families; the theme here names a monospace one of its own.
#[gpui_kit::test]
fn markdown_mode_uses_the_monospace_font(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    let mono = SharedString::from("Test Mono");
    cx.update(|cx| Theme::global_mut(cx).mono_font_family = mono.clone());
    editing.place(0, cx);
    let page_text = cx.update(|cx| item_colors(cx).page_text);
    let size = |style: &TextStyle, cx: &mut TestAppContext| {
        let style = style.clone();
        editing.shell.frame(cx, move |window, _| {
            style.font_size.to_pixels(window.rem_size())
        })
    };

    let rich = editing.painted(cx);
    assert!(rich.font_family != mono, "rich text is not monospace");
    assert_eq!(
        size(&rich, cx),
        px(PAGE_SIZE),
        "rich text takes the page's size"
    );

    editing.set_rich(false, cx);
    let markdown = editing.painted(cx);
    assert_eq!(markdown.font_family, mono, "Markdown is monospace");
    assert_eq!(size(&markdown, cx), px(13.5));
    assert_eq!(markdown.color, page_text, "in the page text colour");

    editing.set_rich(true, cx);
    assert!(editing.painted(cx).font_family != mono, "rich text again");
}

/// The editor's own paste takes plain text, in rich text and in Markdown.
#[gpui_kit::test]
fn plain_text_paste_works_in_both_modes(cx: &mut TestAppContext) {
    const PASTED: &str = "Pasted *plain* text,\nover two lines.";
    let editing = Editing::open(PLAIN, None, cx);
    cx.write_to_clipboard(ClipboardItem::new_string(PASTED.to_owned()));
    editing.place(5, cx);
    editing.keys("ctrl-v", cx);
    let once = with(PLAIN, 5, PASTED);
    assert!(editing.text(cx) == once, "pasted in rich text");
    assert!(editing.dirty(cx));
    assert_eq!(editing.cursor(cx), 5 + PASTED.len());

    editing.set_rich(false, cx);
    editing.keys("ctrl-v", cx);
    assert!(
        editing.text(cx) == with(&once, 5 + PASTED.len(), PASTED),
        "pasted in Markdown"
    );
}

/// The basic editing keys edit and move as a text field's do, and each
/// edit feeds the host.
#[gpui_kit::test]
fn enter_backspace_and_arrows_edit_as_expected(cx: &mut TestAppContext) {
    const TWO: &str = "abc def\nghi jkl\n";
    let editing = Editing::open(TWO, None, cx);
    editing.place(3, cx);
    editing.keys("enter", cx);
    assert_eq!(editing.text(cx), "abc\n def\nghi jkl\n");
    assert_eq!(editing.cursor(cx), 4);
    editing.keys("backspace", cx);
    assert_eq!(editing.text(cx), TWO);
    assert_eq!(editing.cursor(cx), 3);

    editing.keys("right right", cx);
    assert_eq!(editing.cursor(cx), 5);
    editing.keys("delete", cx);
    assert_eq!(editing.text(cx), "abc df\nghi jkl\n");
    editing.keys("down", cx);
    assert_eq!(editing.cursor(cx), 7 + 5, "down keeps the column");
    editing.keys("up", cx);
    assert_eq!(editing.cursor(cx), 5, "up returns");
    editing.keys("left", cx);
    assert_eq!(editing.cursor(cx), 4);
    editing.keys("end", cx);
    assert_eq!(editing.cursor(cx), 6);
    editing.keys("home", cx);
    assert_eq!(editing.cursor(cx), 0);
    editing.input("!", cx);
    assert_eq!(editing.text(cx), "!abc df\nghi jkl\n");
    assert_eq!(
        editing.edits(),
        4,
        "enter, backspace, delete and the typing"
    );
    assert!(editing.dirty(cx));
}

/// R-67: the editor never scrolls, so the host moves its region the
/// least that shows the caret: after each move and each change of text
/// while it has focus, and when it takes focus. A caret moved while it is
/// unfocused moves nothing.
#[gpui_kit::test]
fn the_caret_is_kept_visible_in_the_scroll_region(cx: &mut TestAppContext) {
    let body: String = (1..=80)
        .map(|line| format!("Line {line} of a long body.\n"))
        .collect();
    let editing = Editing::open(&body, None, cx);
    editing.place(0, cx);
    assert_eq!(editing.offset(cx), px(0.));
    for line in 1..=60 {
        editing.keys("down", cx);
        assert!(editing.caret_in_view(cx), "line {line}");
    }
    assert!(editing.offset(cx) < px(0.), "the region followed the caret");
    for press in 1..=8 {
        editing.keys("enter", cx);
        assert!(editing.caret_in_view(cx), "Enter {press}");
    }

    let done = editing.done(cx);
    editing.focus(&done, cx);
    let scroll = editing.tab.read_with(cx, |tab, _| tab.scroll.clone());
    scroll.to_top();
    editing.shell.frame(cx, |_, _| ());
    let editor = editing.editor(cx);
    let end = editing.text(cx).len();
    editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
    editing.shell.frame(cx, |_, _| ());
    assert_eq!(
        editing.offset(cx),
        px(0.),
        "an unfocused caret moves nothing"
    );

    editing.keys("tab", cx);
    assert!(editing.editor_focused(cx), "Tab reached the editor");
    assert_eq!(editing.cursor(cx), end, "the caret stayed where it was");
    assert!(editing.caret_in_view(cx), "taking focus revealed the caret");
}

/// R-65: the cursor given is where editing starts, clamped to the text
/// and to a character boundary; with none, editing starts at the top.
#[gpui_kit::test]
fn the_cursor_given_is_where_editing_starts(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, Some(5), cx);
    assert_eq!(editing.cursor(cx), 5);
    let editor = editing.editor(cx);
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    editing.focus(&handle, cx);
    editing.input("x", cx);
    assert!(
        editing.text(cx) == with(PLAIN, 5, "x"),
        "typing starts there"
    );

    editing.reopen(PLAIN, Some(10_000), cx);
    assert_eq!(editing.cursor(cx), PLAIN.len(), "clamped to the end");
    editing.reopen(PLAIN, None, cx);
    assert_eq!(editing.cursor(cx), 0, "no cursor: the top");
    editing.reopen("héllo", Some(2), cx);
    assert_eq!(editing.cursor(cx), 1, "inside a character: before it");
    assert!(!editing.dirty(cx), "a cursor is no edit");
}

/// R-18 with R-79: the review's result goes in exactly, the baseline
/// becomes theirs, and undo restarts. The caret is kept, clamped.
#[gpui_kit::test]
fn rebase_puts_the_result_in_exactly_and_restarts_undo(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    editing.place(5, cx);
    editing.input("abc", cx);
    editing.events.borrow_mut().clear();
    let host = editing.host(cx);
    for name in editor_text_tests::SECOND_PASS {
        let result = load(&editor_text_tests::fixture(name));
        host.update(cx, |host, cx| host.rebase(PLAIN.to_owned(), &result, cx));
        editing.shell.frame(cx, |_, _| ());
        assert!(editing.text(cx) == result, "{name}: exactly");
        assert!(editing.dirty(cx), "{name}: the result is not theirs");
        assert_eq!(editing.cursor(cx), 8, "{name}: the caret stayed");
    }

    // Shorter than the caret's offset, 8.
    let theirs = "Theirs\n";
    host.update(cx, |host, cx| host.rebase(theirs.to_owned(), theirs, cx));
    editing.shell.frame(cx, |_, _| ());
    assert!(!editing.dirty(cx), "their body is clean");
    assert_eq!(editing.cursor(cx), theirs.len(), "the caret, clamped");
    editing.keys("ctrl-z", cx);
    assert!(editing.text(cx) == theirs, "undo restarted");
    assert_eq!(editing.edits(), 0, "rebase is not an edit to feed again");
}

/// R-19: a clean editor reloads a new body read as the editor loads one,
/// keeping the cursor, clamped; the same body leaves it alone, and so
/// does a dirty editor, whose text is the user's.
#[gpui_kit::test]
fn a_clean_reload_shows_the_new_body_and_keeps_the_cursor(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    editing.place(5, cx);
    let host = editing.host(cx);
    let body = "Another body,\r\nwith $$a$$ pair.\r\n";
    host.update(cx, |host, cx| host.reload_clean(body, cx));
    editing.shell.frame(cx, |_, _| ());
    assert!(editing.text(cx) == load(body), "loaded as the editor loads");
    assert_eq!(editing.cursor(cx), 5, "the cursor was kept");
    assert!(!editing.dirty(cx), "the new body is the baseline");
    assert_eq!(editing.edits(), 0, "a reload is no edit");

    editing.input("x", cx);
    editing.keys("ctrl-z", cx);
    assert!(editing.text(cx) == load(body));
    let editor = editing.editor(cx);
    let notified = Rc::new(Cell::new(0usize));
    cx.update(|cx| {
        let notified = notified.clone();
        cx.observe(&editor, move |_, _| notified.set(notified.get() + 1))
            .detach()
    });
    host.update(cx, |host, cx| host.reload_clean(body, cx));
    editing.shell.frame(cx, |_, _| ());
    assert_eq!(notified.get(), 0, "the same body left the editor alone");
    editing.keys("ctrl-shift-z", cx);
    assert!(
        editing.text(cx) == with(&load(body), 5, "x"),
        "its history too"
    );

    host.update(cx, |host, cx| host.reload_clean("abc", cx));
    editing.shell.frame(cx, |_, _| ());
    assert!(editing.dirty(cx));
    assert!(
        editing.text(cx) == with(&load(body), 5, "x"),
        "a dirty editor is kept"
    );

    editing.keys("ctrl-z", cx);
    assert!(!editing.dirty(cx));
    host.update(cx, |host, cx| host.reload_clean("abc", cx));
    editing.shell.frame(cx, |_, _| ());
    assert_eq!(editing.text(cx), "abc");
    assert_eq!(editing.cursor(cx), 3, "the cursor, clamped");
}

/// R-79 with R-19 and R-69: after a save, the re-read body is the text
/// the editor holds, byte for byte. For fact 81's shapes a second load
/// would change it, so the editor is left alone: its text, its undo
/// history back to the edit before the save, and clean.
#[gpui_kit::test]
fn a_clean_reload_of_the_text_the_editor_holds_leaves_it_alone(cx: &mut TestAppContext) {
    let editing = Editing::open(PLAIN, None, cx);
    for name in editor_text_tests::SECOND_PASS {
        let saved = load(&editor_text_tests::fixture(name));
        assert!(load(&saved) != saved, "{name}: a second load changes it");
        editing.reopen(PLAIN, None, cx);
        // The edit: Markdown pastes plain text as it is, with no rewrite.
        editing.set_rich(false, cx);
        cx.write_to_clipboard(ClipboardItem::new_string(saved.clone()));
        editing.place(0, cx);
        editing.keys("ctrl-a ctrl-v", cx);
        editing.set_rich(true, cx);
        assert!(editing.text(cx) == saved, "{name}: the edit landed exactly");

        let host = editing.host(cx);
        host.update(cx, |host, cx| host.mark_saved(&saved, cx));
        let editor = editing.editor(cx);
        let notified = Rc::new(Cell::new(0usize));
        cx.update(|cx| {
            let notified = notified.clone();
            cx.observe(&editor, move |_, _| notified.set(notified.get() + 1))
                .detach()
        });
        host.update(cx, |host, cx| host.reload_clean(&saved, cx));
        editing.shell.frame(cx, |_, _| ());
        assert!(editing.text(cx) == saved, "{name}: the text is unchanged");
        assert_eq!(notified.get(), 0, "{name}: the editor was left alone");
        assert!(!editing.dirty(cx), "{name}: clean");

        editing.keys("ctrl-z", cx);
        assert!(editing.text(cx) == PLAIN, "{name}: undo reaches the edit");
    }
}

/// The keys the edit host binds (R-59; Keyboard map additions): Escape
/// and the link key act in the edit editor alone, never in a read or
/// comment editor, and the mode key in the host. Each is the only binding
/// of its key there, so nothing of D1's map or Task 8's collides, and
/// Escape in a read editor is neither the host's nor the editor's own
/// `Dismiss`.
#[gpui_kit::test]
fn the_edit_host_keys_act_only_in_the_edit_host(cx: &mut TestAppContext) {
    cx.update(|cx| {
        bind(cx);
        super::super::keys::bind(cx);
    });
    let stack = |contexts: &[&str]| -> Vec<KeyContext> {
        contexts
            .iter()
            .map(|context| KeyContext::parse(context).unwrap())
            .collect()
    };
    let editing = stack(&["Workspace", "Content", EDIT_HOST_CONTEXT, "Editor"]);
    let reading = stack(&["Workspace", "Content", "Editor"]);
    let table: [(&str, Box<dyn Action>); 3] = [
        ("escape", Box::new(EditorEscape)),
        ("secondary-k", Box::new(InsertLink)),
        ("secondary-shift-m", Box::new(ToggleRich)),
    ];
    cx.update(|cx| {
        let keymap = cx.key_bindings();
        let keymap = keymap.borrow();
        for (keys, action) in &table {
            let keystroke = Keystroke::parse(keys).unwrap();
            let (bound, _) = keymap.bindings_for_input(std::slice::from_ref(&keystroke), &editing);
            let names: Vec<_> = bound
                .iter()
                .map(|binding| binding.action().name())
                .collect();
            assert_eq!(names, [action.name()], "{keys} while editing");
            let (read, _) = keymap.bindings_for_input(&[keystroke], &reading);
            let names: Vec<_> = read.iter().map(|binding| binding.action().name()).collect();
            assert!(names.is_empty(), "{keys} in a read editor: {names:?}");
        }
    });
}
