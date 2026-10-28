//! The edit toolbar (design, "The toolbar"), drawn in the test shell over a
//! real `EditorHost`, inside a stand-in for the item tab while editing: the
//! header's Done, the toolbar, the host's scroll region and a read editor,
//! all in the content region, whose Home and End a counter catches. Each
//! test types through the window, after setting the cursor, focusing the
//! editor and drawing a frame. Each inline-mark case selects text first.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui_kit::{
    App, AppContext, Context, ElementId, Entity, EntityInputHandler, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled,
    TestAppContext, Window, div, px,
    test::{ElementSnapshot, TestWindowExt},
};
use zorite_editor::{CellAlign, EditorState};

use super::{
    Toolbar,
    controls::{Control, MAIN, TABLE},
};
use crate::desktop::{
    keys::{CONTENT_CONTEXT, ScrollToBottom, ScrollToTop},
    model::markdown_edit::{Selection, selection_from_utf16},
    view::{
        editor::{
            host::{EditorHost, EditorHostEvent},
            read_only::ReadOnly,
            tests::{focus_and_draw, show},
        },
        harness::Shell,
        scroll::ContentScroll,
    },
};

/// The scroll region's height and width.
const REGION: (f32, f32) = (300., 700.);

/// Two lines; the cases select "two".
const BODY: &str = "One two three\nFour five\n";
/// "two" in `BODY`.
const TWO: (usize, usize) = (4, 7);
/// The address typed into the link popover.
const ADDRESS: &str = "https://example.test";
/// A paragraph and a pipe table of two columns.
const TABLE_BODY: &str = "Intro\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n";
/// Three short lines, for edits over a selection of lines.
const LINES: &str = "alpha\nbeta\ngamma\n";

/// What a formatting control makes of `BODY` with "two" selected. The
/// Style case picks Heading 2.
fn formatted(control: Control) -> &'static str {
    match control {
        Control::Style => "## One two three\nFour five\n",
        Control::Bold => "One **two** three\nFour five\n",
        Control::Italic => "One *two* three\nFour five\n",
        Control::Strike => "One ~~two~~ three\nFour five\n",
        Control::Code => "One `two` three\nFour five\n",
        Control::Link => "One [two](https://example.test) three\nFour five\n",
        Control::Bullets => "- One two three\nFour five\n",
        Control::Numbered => "1. One two three\nFour five\n",
        Control::Checklist => "- [ ] One two three\nFour five\n",
        Control::Quote => "> One two three\nFour five\n",
        Control::CodeBlock => "```\nOne two three\n```\nFour five\n",
        Control::Table => {
            "One two three\n\n| Column 1 | Column 2 |\n| --- | --- |\n|  |  |\n\nFour five\n"
        }
        other => panic!("{other:?} is not a formatting case"),
    }
}

/// How a control is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum How {
    /// A click, which leaves focus where it was.
    Pointer,
    /// Focus on the control, then Enter.
    Keyboard,
}

/// What an item tab gives the toolbar and its host while editing.
struct Tab {
    done: FocusHandle,
    host: Entity<EditorHost>,
    toolbar: Entity<Toolbar>,
    scroll: ContentScroll,
    /// A read editor beside the edit host, as the discussion's comments are.
    read: Entity<ReadOnly>,
    /// How often the content region's Home or End scrolled it.
    scrolled: Rc<Cell<usize>>,
}

impl Tab {
    fn new(body: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let done = cx.focus_handle().tab_stop(true);
        let scroll = ContentScroll::default();
        let (host, toolbar) = editing_in(body, &done, &scroll, window, cx);
        Self {
            done,
            host,
            toolbar,
            scroll,
            read: cx.new(|cx| ReadOnly::new("A read paragraph.\n", window, cx)),
            scrolled: Rc::default(),
        }
    }
}

/// A new host over `body`, placed under `done` in `scroll`'s region, and
/// its toolbar.
fn editing_in(
    body: &str,
    done: &FocusHandle,
    scroll: &ContentScroll,
    window: &mut Window,
    cx: &mut App,
) -> (Entity<EditorHost>, Entity<Toolbar>) {
    let host = cx.new(|cx| {
        let mut host = EditorHost::new(body, None, window, cx);
        host.place(done.clone(), scroll, window, cx);
        host
    });
    let toolbar = cx.new(|cx| Toolbar::new(host.clone(), window, cx));
    (host, toolbar)
}

impl Render for Tab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (top, bottom) = (self.scrolled.clone(), self.scrolled.clone());
        let (height, width) = REGION;
        div()
            .key_context(CONTENT_CONTEXT)
            .size_full()
            .flex()
            .flex_col()
            .text_size(px(15.))
            .on_action(move |_: &ScrollToTop, _, _| top.set(top.get() + 1))
            .on_action(move |_: &ScrollToBottom, _, _| bottom.set(bottom.get() + 1))
            .child(
                div()
                    .id("tab-done")
                    .track_focus(&self.done)
                    .h(px(24.))
                    .child("Done"),
            )
            .child(self.toolbar.clone())
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
            .child(div().flex_none().w(px(width)).child(self.read.clone()))
    }
}

/// A toolbar and its host in the test shell, and what the host emitted
/// since the last `place`.
struct Bar {
    shell: Shell,
    tab: Entity<Tab>,
    events: Rc<RefCell<Vec<EditorHostEvent>>>,
}

impl Bar {
    fn open(body: &str, cx: &mut TestAppContext) -> Self {
        let (shell, outer) = show(cx, |window, cx| Tab::new(body, window, cx));
        let tab = outer.read_with(cx, |outer, _| outer.view.clone());
        let bar = Self {
            shell,
            tab,
            events: Rc::default(),
        };
        bar.watch(cx);
        bar
    }

    /// A new host over `body`, and its toolbar, in the last ones' place,
    /// in rich text or Markdown.
    fn reset(&self, body: &str, rich: bool, cx: &mut TestAppContext) {
        let tab = self.tab.clone();
        let body = body.to_owned();
        self.shell.frame(cx, move |window, cx| {
            tab.update(cx, |tab, cx| {
                (tab.host, tab.toolbar) = editing_in(&body, &tab.done, &tab.scroll, window, cx);
                cx.notify();
            })
        });
        self.watch(cx);
        self.shell.frame(cx, |_, _| ());
        if !rich {
            self.set_rich(false, cx);
        }
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

    fn toolbar(&self, cx: &mut TestAppContext) -> Entity<Toolbar> {
        self.tab.read_with(cx, |tab, _| tab.toolbar.clone())
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
        self.shell.frame(cx, |_, _| ());
        self.events.borrow_mut().clear();
    }

    /// Selects `start..end` of one line, from the keyboard, and checks the
    /// editor holds that selection.
    fn select(&self, start: usize, end: usize, cx: &mut TestAppContext) {
        self.place(start, cx);
        let keys = vec!["shift-right"; end - start].join(" ");
        self.keys(&keys, cx);
        assert_eq!(
            self.selection(cx),
            Selection { start, end },
            "the keys selected {start}..{end}"
        );
        self.events.borrow_mut().clear();
    }

    /// The editor's selection, as the toolbar reads it.
    fn selection(&self, cx: &mut TestAppContext) -> Selection {
        let editor = self.editor(cx);
        self.shell.frame(cx, move |window, cx| {
            let utf16 = editor
                .update(cx, |editor, cx| {
                    editor.selected_text_range(false, window, cx)
                })
                .map_or(0..0, |selection| selection.range);
            selection_from_utf16(editor.read(cx).text(), utf16)
        })
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

    /// Presses and releases `key`, as a keyboard does: GPUI clicks a
    /// focused control on the release of Enter or Space.
    fn press(&self, key: &str, cx: &mut TestAppContext) {
        self.shell.press(key, cx);
        self.shell.frame(cx, |_, _| ());
        self.shell.frame(cx, |_, _| ());
    }

    fn click(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        self.shell.click(id, cx);
        self.shell.frame(cx, |_, _| ());
        self.shell.frame(cx, |_, _| ());
    }

    fn handle(&self, control: Control, cx: &mut TestAppContext) -> FocusHandle {
        let toolbar = self.toolbar(cx);
        toolbar.read_with(cx, |toolbar, _| toolbar.handle(control))
    }

    fn focus(&self, handle: &FocusHandle, cx: &mut TestAppContext) {
        let handle = handle.clone();
        self.shell
            .frame(cx, move |window, cx| window.focus(&handle, cx));
        self.shell.frame(cx, |_, _| ());
    }

    fn focus_control(&self, control: Control, cx: &mut TestAppContext) {
        let handle = self.handle(control, cx);
        self.focus(&handle, cx);
    }

    /// Uses `control` by pointer or by keyboard.
    fn activate(&self, control: Control, how: How, cx: &mut TestAppContext) {
        match how {
            How::Pointer => self.click(control.id(), cx),
            How::Keyboard => {
                self.focus_control(control, cx);
                self.press("enter", cx);
            }
        }
    }

    /// After `control` was used: a click left focus in the editor, and
    /// Enter left it on the control, a popover's included, which gives it
    /// back when it closes.
    fn kept_focus(&self, control: Control, how: How, cx: &mut TestAppContext) {
        match how {
            How::Pointer => assert!(self.editor_focused(cx), "{control:?}: a click"),
            How::Keyboard => assert!(self.focused(control, cx), "{control:?}: Enter"),
        }
    }

    fn is_focused(&self, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
        let handle = handle.clone();
        self.shell
            .frame(cx, move |window, _| handle.is_focused(window))
    }

    fn focused(&self, control: Control, cx: &mut TestAppContext) -> bool {
        let handle = self.handle(control, cx);
        self.is_focused(&handle, cx)
    }

    fn editor_focused(&self, cx: &mut TestAppContext) -> bool {
        let editor = self.editor(cx);
        let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
        self.is_focused(&handle, cx)
    }

    fn done(&self, cx: &mut TestAppContext) -> FocusHandle {
        self.tab.read_with(cx, |tab, _| tab.done.clone())
    }

    fn text(&self, cx: &mut TestAppContext) -> String {
        let host = self.host(cx);
        host.read_with(cx, |host, cx| host.text(cx))
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

    fn dirty(&self, cx: &mut TestAppContext) -> bool {
        let host = self.host(cx);
        host.read_with(cx, |host, _| host.is_dirty())
    }

    /// How many `Edited` the host emitted since `place`.
    fn edits(&self) -> usize {
        self.events
            .borrow()
            .iter()
            .filter(|event| **event == EditorHostEvent::Edited)
            .count()
    }

    fn scrolled(&self, cx: &mut TestAppContext) -> usize {
        self.tab.read_with(cx, |tab, _| tab.scrolled.get())
    }

    fn present(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        self.shell.present(id, cx)
    }

    /// What the last frame says of the element `id`.
    fn snapshot<R>(
        &self,
        id: impl Into<ElementId>,
        read: impl FnOnce(&ElementSnapshot) -> R,
        cx: &mut TestAppContext,
    ) -> Option<R> {
        let id = id.into();
        self.shell.frame(cx, move |window, _| {
            window.try_find(id).map(|found| read(&found))
        })
    }

    /// Whether `control` is drawn as active (pressed).
    fn active(&self, control: Control, cx: &mut TestAppContext) -> bool {
        self.snapshot(control.id(), |found| found.checked(), cx)
            .flatten()
            == Some(true)
    }

    /// The Style control's value: the caret line's style.
    fn style(&self, cx: &mut TestAppContext) -> Option<String> {
        self.snapshot(
            Control::Style.id(),
            |found| found.value().map(str::to_owned),
            cx,
        )
        .flatten()
    }

    /// The opacity `control` was drawn at in the last frame.
    fn drawn(&self, control: Control, cx: &mut TestAppContext) -> Option<f32> {
        self.shell.frame(cx, |_, _| ());
        let toolbar = self.toolbar(cx);
        toolbar.read_with(cx, |toolbar, _| toolbar.drawn(control))
    }

    /// Whether the link popover's address field has focus.
    fn link_field_focused(&self, cx: &mut TestAppContext) -> bool {
        let toolbar = self.toolbar(cx);
        let Some(input) = toolbar.read_with(cx, |toolbar, _| toolbar.link_input()) else {
            return false;
        };
        let handle = input.read_with(cx, |input, cx| input.focus_handle(cx));
        self.is_focused(&handle, cx)
    }

    /// What the editor's own table method `op` makes of `TABLE_BODY` with
    /// the caret in its cell "1", in rich text: the oracle for the table
    /// controls in both modes.
    fn table_oracle(
        &self,
        op: impl FnOnce(&mut EditorState, &mut Context<EditorState>),
        cx: &mut TestAppContext,
    ) -> String {
        self.table_oracle_at(at(TABLE_BODY, "1"), op, cx)
    }

    /// The same, with the caret at `cursor`.
    fn table_oracle_at(
        &self,
        cursor: usize,
        op: impl FnOnce(&mut EditorState, &mut Context<EditorState>),
        cx: &mut TestAppContext,
    ) -> String {
        self.reset(TABLE_BODY, true, cx);
        self.place(cursor, cx);
        let editor = self.editor(cx);
        editor.update(cx, op);
        self.shell.frame(cx, |_, _| ());
        let text = self.text(cx);
        assert!(text != TABLE_BODY, "the oracle changed the table");
        text
    }
}

/// Where `needle` starts in `text`.
fn at(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is in the text"))
}

/// Uses a formatting control on `BODY` with "two" selected. Style picks
/// Heading 2 from its menu and Link types the address into its popover,
/// each in the way `how` names.
fn format(bar: &Bar, control: Control, how: How, cx: &mut TestAppContext) {
    bar.activate(control, how, cx);
    match (control, how) {
        (Control::Style, How::Pointer) => bar.click(("toolbar-style-option", 2usize), cx),
        (Control::Style, How::Keyboard) => {
            bar.keys("down down", cx);
            bar.press("enter", cx);
        }
        (Control::Link, _) => {
            assert!(bar.link_field_focused(cx), "the address field has focus");
            bar.input(ADDRESS, cx);
            bar.press("enter", cx);
        }
        _ => {}
    }
}

/// Every control does its work by a click and by Enter on it, in rich
/// text and in Markdown: the mode switch, Undo and Redo, Style, the
/// inline marks, Link, the lists, the blocks and, inside a table, the
/// table controls, whose work is the editor's own table methods.
#[gpui_kit::test]
fn every_control_edits_by_pointer_and_by_keyboard_in_both_modes(cx: &mut TestAppContext) {
    let bar = Bar::open(BODY, cx);
    let table_cases: [(Control, String); 7] = [
        (
            Control::RowBelow,
            bar.table_oracle(|editor, cx| editor.insert_table_row(true, cx), cx),
        ),
        (
            Control::ColumnRight,
            bar.table_oracle(|editor, cx| editor.insert_table_column(true, cx), cx),
        ),
        (
            Control::DeleteRow,
            bar.table_oracle(|editor, cx| editor.delete_table_row(cx), cx),
        ),
        (
            Control::DeleteColumn,
            bar.table_oracle(|editor, cx| editor.delete_table_column(cx), cx),
        ),
        (
            Control::AlignLeft,
            bar.table_oracle(
                |editor, cx| editor.set_caret_table_align(CellAlign::Left, cx),
                cx,
            ),
        ),
        (
            Control::AlignCentre,
            bar.table_oracle(
                |editor, cx| editor.set_caret_table_align(CellAlign::Center, cx),
                cx,
            ),
        ),
        (
            Control::AlignRight,
            bar.table_oracle(
                |editor, cx| editor.set_caret_table_align(CellAlign::Right, cx),
                cx,
            ),
        ),
    ];
    for rich in [true, false] {
        for how in [How::Pointer, How::Keyboard] {
            let case = |control: Control| format!("{control:?}, {how:?}, rich {rich}");

            // The mode switch: each segment sets its mode; the text stays.
            bar.reset(BODY, rich, cx);
            bar.place(TWO.0, cx);
            let other = if rich {
                Control::Markdown
            } else {
                Control::Rich
            };
            bar.activate(other, how, cx);
            assert_eq!(bar.rich(cx), !rich, "{}", case(other));
            let same = if rich {
                Control::Rich
            } else {
                Control::Markdown
            };
            bar.activate(same, how, cx);
            assert_eq!(bar.rich(cx), rich, "{}", case(same));
            bar.kept_focus(same, how, cx);
            assert!(bar.text(cx) == BODY, "the switch kept the text");

            // Undo and Redo.
            bar.reset(BODY, rich, cx);
            bar.place(0, cx);
            bar.input("x", cx);
            let typed = format!("x{BODY}");
            bar.activate(Control::Undo, how, cx);
            assert!(bar.text(cx) == BODY, "{}", case(Control::Undo));
            bar.activate(Control::Redo, how, cx);
            assert!(bar.text(cx) == typed, "{}", case(Control::Redo));
            bar.kept_focus(Control::Redo, how, cx);

            // Style, the marks, Link, the lists and the blocks.
            for control in MAIN.into_iter().filter(|control| {
                !matches!(
                    control,
                    Control::Rich | Control::Markdown | Control::Undo | Control::Redo
                )
            }) {
                bar.reset(BODY, rich, cx);
                bar.select(TWO.0, TWO.1, cx);
                format(&bar, control, how, cx);
                assert_eq!(bar.text(cx), formatted(control), "{}", case(control));
                assert_eq!(bar.rich(cx), rich, "{}: the mode stayed", case(control));
                bar.kept_focus(control, how, cx);
            }

            // The table controls, with the caret in the cell "1".
            for (control, expected) in &table_cases {
                bar.reset(TABLE_BODY, rich, cx);
                bar.place(at(TABLE_BODY, "1"), cx);
                bar.activate(*control, how, cx);
                assert_eq!(&bar.text(cx), expected, "{}", case(*control));
                bar.kept_focus(*control, how, cx);
            }
        }
    }
}

/// With a toolbar control focused, as a keyboard user leaves it: the
/// editor's own actions reach the editor through its focus handle, focus
/// stays on the control, and the mode key reaches the host. Like their
/// keys, the inline marks do nothing on an empty selection.
#[gpui_kit::test]
fn editor_actions_reach_the_editor_while_a_button_has_focus(cx: &mut TestAppContext) {
    let bar = Bar::open(BODY, cx);
    bar.select(TWO.0, TWO.1, cx);

    // Tab into the toolbar from Done, then along it to Bold.
    let done = bar.done(cx);
    bar.focus(&done, cx);
    bar.keys("tab", cx);
    assert!(
        bar.focused(Control::Rich, cx),
        "Tab enters at the first control"
    );
    bar.keys("right right right right right", cx);
    assert!(bar.focused(Control::Bold, cx));

    bar.press("space", cx);
    assert_eq!(bar.text(cx), formatted(Control::Bold), "Space on Bold");
    assert!(bar.focused(Control::Bold, cx), "focus stayed on Bold");
    assert!(!bar.editor_focused(cx));
    bar.press("enter", cx);
    assert_eq!(bar.text(cx), BODY, "Enter on Bold unwrapped the selection");

    for control in [Control::Italic, Control::Strike, Control::Code] {
        bar.keys("right", cx);
        assert!(bar.focused(control, cx), "{control:?}");
        bar.press("enter", cx);
        assert_eq!(bar.text(cx), formatted(control), "{control:?}");
        assert!(bar.focused(control, cx), "{control:?}: focus stayed");
        bar.press("enter", cx);
        assert_eq!(bar.text(cx), BODY, "{control:?} again");
    }

    // Undo and Redo, from Home along the toolbar.
    bar.keys("home right right", cx);
    assert!(bar.focused(Control::Undo, cx));
    bar.press("enter", cx);
    assert_eq!(bar.text(cx), formatted(Control::Code), "Undo");
    bar.keys("right", cx);
    bar.press("enter", cx);
    assert_eq!(bar.text(cx), BODY, "Redo");
    assert!(bar.focused(Control::Redo, cx));

    // The mode key, from a control.
    bar.keys("secondary-shift-m", cx);
    assert!(!bar.rich(cx), "the mode key switched to Markdown from Redo");
    assert!(bar.focused(Control::Redo, cx));
    bar.keys("secondary-shift-m", cx);
    assert!(bar.rich(cx), "and back");

    // An empty selection: nothing to wrap.
    bar.place(TWO.0, cx);
    for control in [
        Control::Bold,
        Control::Italic,
        Control::Strike,
        Control::Code,
    ] {
        bar.focus_control(control, cx);
        bar.press("enter", cx);
        assert_eq!(bar.text(cx), BODY, "{control:?} on a caret");
    }
}

/// Each host edit is one `replace_range`: one edit the host sees, so the
/// draft hears of it as of typing, dirty, and one undo step, which the
/// editor's key and the toolbar's Undo both take back whole. Typing after
/// it is a step of its own.
#[gpui_kit::test]
fn a_host_edit_is_one_undo_step(cx: &mut TestAppContext) {
    let bar = Bar::open(LINES, cx);
    for control in [
        Control::Style,
        Control::Link,
        Control::Bullets,
        Control::Numbered,
        Control::Checklist,
        Control::Quote,
        Control::CodeBlock,
        Control::Table,
    ] {
        bar.reset(LINES, true, cx);
        // "alpha" and "beta", across the line break.
        bar.place(0, cx);
        bar.keys(
            "shift-down shift-right shift-right shift-right shift-right",
            cx,
        );
        assert_eq!(bar.selection(cx), Selection { start: 0, end: 10 });
        bar.events.borrow_mut().clear();

        format(&bar, control, How::Pointer, cx);
        let edited = bar.text(cx);
        assert!(edited != LINES, "{control:?} edited");
        assert_eq!(bar.edits(), 1, "{control:?} is one edit");
        assert!(
            bar.events
                .borrow()
                .contains(&EditorHostEvent::DirtyChanged(true)),
            "{control:?} made the editor dirty"
        );
        assert!(bar.dirty(cx));

        let editor = bar.editor(cx);
        focus_and_draw(&bar.shell, &editor, cx);
        bar.keys("ctrl-z", cx);
        assert!(bar.text(cx) == LINES, "{control:?}: one undo");
        assert!(!bar.dirty(cx), "{control:?}: clean again");
        bar.keys("ctrl-shift-z", cx);
        assert!(bar.text(cx) == edited, "{control:?}: redo");
        bar.click(Control::Undo.id(), cx);
        assert!(bar.text(cx) == LINES, "{control:?}: the toolbar's Undo");
    }

    // Typing after a host edit undoes on its own.
    bar.reset(LINES, true, cx);
    bar.place(0, cx);
    bar.click(Control::Bullets.id(), cx);
    let listed = bar.text(cx);
    assert_eq!(listed, "- alpha\nbeta\ngamma\n");
    bar.input("x", cx);
    assert_eq!(bar.text(cx), "- xalpha\nbeta\ngamma\n");
    bar.keys("ctrl-z", cx);
    assert!(bar.text(cx) == listed, "undo took the typing only");
    bar.keys("ctrl-z", cx);
    assert!(bar.text(cx) == LINES, "then the list");
}

/// Fact 67: Left and Right move along the toolbar, Home and End go to its
/// ends, and Tab leaves it for the editor. These keys never reach the
/// editor or the content region while a control has focus; on Done, the
/// region's own Home still scrolls it.
#[gpui_kit::test]
fn left_right_home_and_end_move_along_the_toolbar(cx: &mut TestAppContext) {
    let bar = Bar::open(BODY, cx);
    bar.place(TWO.0, cx);
    let done = bar.done(cx);
    bar.focus(&done, cx);
    assert_eq!(
        bar.shell.label("toolbar-hint", cx).as_deref(),
        Some("Esc: Done \u{b7} F6: next region"),
        "the hint under the toolbar says how to leave the editor"
    );
    bar.keys("tab", cx);
    assert!(
        bar.focused(Control::Rich, cx),
        "Tab enters at the first control"
    );

    for pair in MAIN.windows(2) {
        bar.keys("right", cx);
        assert!(bar.focused(pair[1], cx), "Right from {:?}", pair[0]);
    }
    bar.keys("right", cx);
    assert!(bar.focused(Control::Table, cx), "Right stops at the last");
    bar.keys("home", cx);
    assert!(bar.focused(Control::Rich, cx), "Home");
    bar.keys("left", cx);
    assert!(bar.focused(Control::Rich, cx), "Left stops at the first");
    bar.keys("end", cx);
    assert!(bar.focused(Control::Table, cx), "End");
    bar.keys("left", cx);
    assert!(bar.focused(Control::CodeBlock, cx), "Left");

    assert_eq!(bar.scrolled(cx), 0, "Home and End stayed in the toolbar");
    let editor = bar.editor(cx);
    let cursor = editor.read_with(cx, |editor, _| editor.cursor());
    assert_eq!(cursor, TWO.0, "the arrows never reached the editor");
    assert_eq!(bar.text(cx), BODY);

    // The tab stop roves: Shift-Tab from a control leaves in one press, and
    // Tab from outside still enters at the first control.
    bar.keys("shift-tab", cx);
    assert!(bar.is_focused(&done, cx), "Shift-Tab leaves the toolbar");
    bar.keys("tab", cx);
    assert!(
        bar.focused(Control::Rich, cx),
        "Tab enters at the first again"
    );
    bar.keys("end left", cx);
    assert!(bar.focused(Control::CodeBlock, cx));

    bar.keys("tab", cx);
    assert!(
        bar.editor_focused(cx),
        "Tab leaves the toolbar for the editor"
    );

    // Up and Down move through the Style menu, stopping at its ends, and
    // Escape closes it, giving focus back to Style.
    bar.focus_control(Control::Style, cx);
    bar.press("enter", cx);
    assert!(
        bar.present("toolbar-style-menu", cx),
        "Enter opened the menu"
    );
    let on = |bar: &Bar, ix: usize, cx: &mut TestAppContext| {
        bar.snapshot(("toolbar-style-option", ix), |found| found.focused(), cx)
            .flatten()
            == Some(true)
    };
    assert!(on(&bar, 0, cx), "on the caret line's style");
    bar.keys("up", cx);
    assert!(on(&bar, 0, cx), "Up stops at the first");
    bar.keys("down down down down", cx);
    assert!(on(&bar, 3, cx), "Down stops at the last");
    bar.keys("left", cx);
    assert!(on(&bar, 3, cx), "Left does nothing in the menu");
    bar.keys("right right", cx);
    assert!(on(&bar, 3, cx), "nor does Right");
    assert!(bar.present("toolbar-style-menu", cx), "the menu stays open");
    bar.keys("home", cx);
    assert!(on(&bar, 0, cx), "Home in the menu");
    bar.keys("end", cx);
    assert!(on(&bar, 3, cx), "End in the menu");
    bar.keys("escape", cx);
    assert!(!bar.present("toolbar-style-menu", cx), "Escape closed it");
    assert!(bar.focused(Control::Style, cx), "focus is back on Style");
    assert_eq!(bar.text(cx), BODY);
    assert_eq!(bar.scrolled(cx), 0, "Home and End stayed in the menu");

    // Inside a table, End reaches the last table control.
    bar.reset(TABLE_BODY, true, cx);
    bar.place(at(TABLE_BODY, "1"), cx);
    bar.focus_control(Control::Rich, cx);
    bar.keys("end", cx);
    assert!(bar.focused(Control::AlignRight, cx), "End in a table");
    bar.keys("left", cx);
    assert!(bar.focused(Control::AlignCentre, cx));
    bar.keys("home", cx);
    assert!(bar.focused(Control::Rich, cx));
    assert_eq!(bar.scrolled(cx), 0);

    // The counter does count: Home on Done scrolls the region.
    let done = bar.done(cx);
    bar.focus(&done, cx);
    bar.keys("home", cx);
    assert_eq!(bar.scrolled(cx), 1, "Home on Done reached the region");
}

/// R-55: the Style control names the caret line's style, and the inline
/// marks show as active by the editor's own unwrap rule. Both follow the
/// caret, an edit and an undo, which moves no caret event; drawing alone
/// reads nothing again.
#[gpui_kit::test]
fn the_style_label_and_active_marks_follow_the_caret(cx: &mut TestAppContext) {
    const MARKED: &str =
        "# Title\n\nSome **bold** and *it* and ~~gone~~ and `code` here.\n\n### Small\n";
    let bar = Bar::open(MARKED, cx);
    let marks = [
        Control::Bold,
        Control::Italic,
        Control::Strike,
        Control::Code,
    ];
    let active = |bar: &Bar, cx: &mut TestAppContext| -> Vec<Control> {
        marks
            .into_iter()
            .filter(|mark| bar.active(*mark, cx))
            .collect()
    };

    bar.place(at(MARKED, "Title"), cx);
    assert_eq!(bar.style(cx).as_deref(), Some("Heading 1"));
    bar.place(at(MARKED, "Some"), cx);
    assert_eq!(bar.style(cx).as_deref(), Some("Paragraph"));
    bar.place(at(MARKED, "Small"), cx);
    assert_eq!(bar.style(cx).as_deref(), Some("Heading 3"));
    assert!(active(&bar, cx).is_empty(), "a caret has no marks");

    let word = |needle: &str| (at(MARKED, needle), at(MARKED, needle) + needle.len());
    let (start, end) = word("bold");
    bar.select(start, end, cx);
    // The editor's Italic would unwrap one star of "**" too.
    assert_eq!(active(&bar, cx), [Control::Bold, Control::Italic]);
    assert_eq!(bar.style(cx).as_deref(), Some("Paragraph"));
    let start = at(MARKED, "*it*") + 1;
    bar.select(start, start + 2, cx);
    assert_eq!(active(&bar, cx), [Control::Italic]);
    let (start, end) = word("gone");
    bar.select(start, end, cx);
    assert_eq!(active(&bar, cx), [Control::Strike]);
    let (start, end) = word("code");
    bar.select(start, end, cx);
    assert_eq!(active(&bar, cx), [Control::Code]);
    let (start, end) = word("here");
    bar.select(start, end, cx);
    assert!(active(&bar, cx).is_empty(), "plain text");

    // After an edit, which keeps the word selected, and after undo.
    bar.click(Control::Bold.id(), cx);
    assert_eq!(active(&bar, cx), [Control::Bold, Control::Italic]);
    bar.click(Control::Undo.id(), cx);
    assert!(active(&bar, cx).is_empty(), "undo, with no caret event");

    // A host edit, then its undo.
    bar.place(at(MARKED, "Some"), cx);
    bar.click(Control::Style.id(), cx);
    bar.click(("toolbar-style-option", 2usize), cx);
    assert_eq!(bar.style(cx).as_deref(), Some("Heading 2"));
    bar.click(Control::Undo.id(), cx);
    assert_eq!(bar.style(cx).as_deref(), Some("Paragraph"));

    // Frames alone read nothing again.
    let toolbar = bar.toolbar(cx);
    let before = toolbar.read_with(cx, |toolbar, _| toolbar.refreshes());
    for _ in 0..3 {
        bar.shell.frame(cx, |_, _| ());
    }
    let after = toolbar.read_with(cx, |toolbar, _| toolbar.refreshes());
    assert_eq!(before, after, "drawing does not recompute the state");
}

/// The table controls appear only while the caret is on a row of a pipe
/// table, never over one inside a code fence; Insert row below adds a row
/// by click and by Enter, in both modes. A focused table control that goes
/// away leaves focus on Table.
#[gpui_kit::test]
fn table_controls_show_only_inside_a_table_and_insert_a_row(cx: &mut TestAppContext) {
    const FENCED: &str = "```\n| A | B |\n| --- | --- |\n| 1 | 2 |\n```\n";
    let bar = Bar::open(TABLE_BODY, cx);
    let shown = |bar: &Bar, cx: &mut TestAppContext| -> Vec<bool> {
        TABLE
            .into_iter()
            .map(|control| bar.present(control.id(), cx))
            .collect()
    };
    for rich in [true, false] {
        bar.reset(TABLE_BODY, rich, cx);
        bar.place(at(TABLE_BODY, "Intro"), cx);
        assert_eq!(shown(&bar, cx), [false; 7], "rich {rich}: a paragraph");
        bar.place(at(TABLE_BODY, "1"), cx);
        assert_eq!(shown(&bar, cx), [true; 7], "rich {rich}: a body row");
        bar.place(at(TABLE_BODY, "A"), cx);
        assert_eq!(shown(&bar, cx), [true; 7], "rich {rich}: the header row");

        bar.place(at(TABLE_BODY, "1"), cx);
        bar.click(Control::RowBelow.id(), cx);
        let one = format!("{TABLE_BODY}|  |  |\n");
        assert_eq!(bar.text(cx), one, "rich {rich}: a click");
        bar.focus_control(Control::RowBelow, cx);
        bar.press("enter", cx);
        assert_eq!(
            bar.text(cx),
            format!("{one}|  |  |\n"),
            "rich {rich}: Enter"
        );

        // The caret leaves the table: the controls go, and focus with
        // them moves to Table.
        let editor = bar.editor(cx);
        editor.update(cx, |editor, cx| editor.set_cursor(0, cx));
        bar.shell.frame(cx, |_, _| ());
        bar.shell.frame(cx, |_, _| ());
        assert_eq!(shown(&bar, cx), [false; 7], "rich {rich}: out again");
        assert!(
            bar.focused(Control::Table, cx),
            "rich {rich}: focus on Table"
        );
    }

    // The Table control inserts one, and its controls follow the caret in.
    bar.reset("Intro\n", true, cx);
    bar.place(0, cx);
    bar.click(Control::Table.id(), cx);
    assert_eq!(shown(&bar, cx), [true; 7], "inside the new table");

    // Not over a fence.
    bar.reset(FENCED, true, cx);
    bar.place(at(FENCED, "1"), cx);
    assert_eq!(shown(&bar, cx), [false; 7], "inside a code fence");
}

/// The link key opens the link popover from the edit editor only: the
/// address typed there makes the selection a link, or is the text with no
/// selection, as one undo step; Escape closes it and changes nothing.
/// Neither a read editor nor a toolbar control takes the key.
#[gpui_kit::test]
fn secondary_k_opens_the_link_popover_only_in_the_edit_host(cx: &mut TestAppContext) {
    let bar = Bar::open(BODY, cx);
    bar.select(TWO.0, TWO.1, cx);
    assert!(!bar.present("toolbar-link-popover", cx));
    bar.keys("secondary-k", cx);
    assert!(bar.present("toolbar-link-popover", cx), "the key opened it");
    assert!(bar.link_field_focused(cx), "on its address field");
    bar.input(ADDRESS, cx);
    // Home and End move in the field: they never reach the toolbar or the
    // content region.
    bar.keys("home end home end", cx);
    assert!(
        bar.link_field_focused(cx),
        "Home and End stayed in the field"
    );
    assert!(bar.present("toolbar-link-popover", cx));
    assert_eq!(
        bar.scrolled(cx),
        0,
        "Home and End never scrolled the region"
    );
    // A dialog traps focus: Tab and Shift-Tab go between the field and
    // Insert, and the address stays.
    let on_insert = |cx: &mut TestAppContext| {
        bar.snapshot("toolbar-link-insert", |found| found.focused(), cx)
            .flatten()
            == Some(true)
    };
    for (key, insert) in [
        ("tab", true),
        ("tab", false),
        ("shift-tab", true),
        ("shift-tab", false),
    ] {
        bar.keys(key, cx);
        assert!(
            bar.present("toolbar-link-popover", cx),
            "{key} kept it open"
        );
        assert_eq!(on_insert(cx), insert, "{key}: on Insert");
        assert_eq!(bar.link_field_focused(cx), !insert, "{key}: on the field");
    }
    bar.press("enter", cx);
    assert_eq!(bar.text(cx), formatted(Control::Link));
    assert!(!bar.present("toolbar-link-popover", cx), "Enter closed it");
    assert!(bar.editor_focused(cx), "focus went back to the editor");
    bar.keys("ctrl-z", cx);
    assert!(bar.text(cx) == BODY, "one undo step");

    // No selection: the address is the text.
    bar.place(0, cx);
    bar.keys("secondary-k", cx);
    bar.input(ADDRESS, cx);
    bar.press("enter", cx);
    assert_eq!(bar.text(cx), format!("[{ADDRESS}]({ADDRESS}){BODY}"));

    // Escape.
    bar.reset(BODY, true, cx);
    bar.place(TWO.0, cx);
    bar.keys("secondary-k", cx);
    assert!(bar.present("toolbar-link-popover", cx));
    bar.input("https://never", cx);
    bar.keys("escape", cx);
    assert!(!bar.present("toolbar-link-popover", cx), "Escape closed it");
    assert!(bar.text(cx) == BODY, "and changed nothing");
    assert!(bar.editor_focused(cx));
    // From Insert too.
    bar.keys("secondary-k", cx);
    bar.input("https://never", cx);
    bar.keys("tab", cx);
    assert!(on_insert(cx));
    bar.keys("escape", cx);
    assert!(!bar.present("toolbar-link-popover", cx), "Escape on Insert");
    assert!(bar.text(cx) == BODY);
    assert!(bar.editor_focused(cx), "focus went back to the editor");

    // Focus moving out of it closes it too.
    bar.keys("secondary-k", cx);
    assert!(bar.present("toolbar-link-popover", cx));
    let editor = bar.editor(cx);
    focus_and_draw(&bar.shell, &editor, cx);
    bar.shell.frame(cx, |_, _| ());
    assert!(
        !bar.present("toolbar-link-popover", cx),
        "focus left, it closed"
    );
    assert!(bar.editor_focused(cx), "and focus stayed where it went");

    // A read editor.
    let read = bar.tab.read_with(cx, |tab, _| tab.read.clone());
    let read_editor = read.read_with(cx, |read, _| read.editor().clone());
    read_editor.update(cx, |editor, cx| editor.set_cursor(2, cx));
    focus_and_draw(&bar.shell, &read_editor, cx);
    bar.keys("secondary-k", cx);
    assert!(
        !bar.present("toolbar-link-popover", cx),
        "not from a read editor"
    );
    let read_text = read_editor.read_with(cx, |editor, _| editor.text().to_owned());
    assert_eq!(read_text, "A read paragraph.\n");

    // A toolbar control.
    bar.focus_control(Control::Bold, cx);
    bar.keys("secondary-k", cx);
    assert!(
        !bar.present("toolbar-link-popover", cx),
        "not from a control"
    );
}

/// In Markdown, the formatting controls are drawn at 35% and still work;
/// the mode switch, Undo and Redo keep full strength. Rich text draws all
/// at full strength.
#[gpui_kit::test]
fn markdown_mode_dims_the_formatting_controls_and_keeps_them_working(cx: &mut TestAppContext) {
    let bar = Bar::open(BODY, cx);
    bar.place(TWO.0, cx);
    for control in MAIN {
        assert_eq!(bar.drawn(control, cx), Some(1.), "rich: {control:?}");
    }
    bar.click(Control::Markdown.id(), cx);
    assert!(!bar.rich(cx));
    for control in MAIN {
        let expected = if control.formatting() { 0.35 } else { 1. };
        assert_eq!(
            bar.drawn(control, cx),
            Some(expected),
            "Markdown: {control:?}"
        );
    }

    bar.select(TWO.0, TWO.1, cx);
    bar.click(Control::Bold.id(), cx);
    assert_eq!(bar.text(cx), formatted(Control::Bold), "a dimmed Bold");
    bar.focus_control(Control::Bullets, cx);
    bar.press("enter", cx);
    assert_eq!(
        bar.text(cx),
        "- One **two** three\nFour five\n",
        "a dimmed list"
    );
    bar.click(Control::Undo.id(), cx);
    assert_eq!(
        bar.text(cx),
        formatted(Control::Bold),
        "Undo at full strength"
    );

    // The table controls dim too, and align a column in Markdown.
    bar.reset(TABLE_BODY, false, cx);
    bar.place(at(TABLE_BODY, "1"), cx);
    for control in TABLE {
        assert_eq!(bar.drawn(control, cx), Some(0.35), "Markdown: {control:?}");
    }
    bar.click(Control::AlignCentre.id(), cx);
    assert_eq!(
        bar.text(cx),
        "Intro\n\n| A | B |\n| :-: | :-- |\n| 1 | 2 |\n",
        "a dimmed alignment"
    );

    // Past a row's last cell, before its closing pipe or at its end, the
    // caret aligns the last column, as the editor's own lookup clamps: on
    // a body row and on the header row, against the editor's rich-text
    // result with the caret in that row's last cell.
    let aligns = [
        (Control::AlignLeft, CellAlign::Left),
        (Control::AlignCentre, CellAlign::Center),
        (Control::AlignRight, CellAlign::Right),
    ];
    for (row, last) in [("| A | B |", "B"), ("| 1 | 2 |", "2")] {
        let end = at(TABLE_BODY, row) + row.len();
        for (control, align) in aligns {
            let expected = bar.table_oracle_at(
                at(TABLE_BODY, last),
                move |editor, cx| editor.set_caret_table_align(align, cx),
                cx,
            );
            for cursor in [end - 1, end] {
                bar.reset(TABLE_BODY, false, cx);
                bar.place(cursor, cx);
                bar.click(control.id(), cx);
                assert_eq!(
                    bar.text(cx),
                    expected,
                    "{control:?} at {cursor}, past the last cell of {row:?}"
                );
            }
        }
    }

    bar.reset(TABLE_BODY, false, cx);
    bar.place(at(TABLE_BODY, "1"), cx);
    bar.click(Control::Rich.id(), cx);
    for control in MAIN.into_iter().chain(TABLE) {
        assert_eq!(bar.drawn(control, cx), Some(1.), "rich again: {control:?}");
    }
}
