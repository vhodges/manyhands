//! The edit editor's toolbar (design, "The toolbar";
//! `item_view/02-editor-and-toolbar.md`), fixed under the header while
//! editing. Every control reaches its tab's `EditorHost`:
//! - **Mode**, Rich text | Markdown: the host's `set_rich`.
//! - **Editor actions**, Undo, Redo, Bold, Italic, Strikethrough and Inline
//!   code: dispatched through the editor's focus handle, since focus is on
//!   the control and the handlers are private (fact 80). The marks do
//!   nothing on an empty selection, as their keys do (fact 72).
//! - **Host edits**, Style, Link, the lists, Code block and Table: a pure
//!   function of the text and the selection (`markdown_edit`, R-55), put
//!   in with one `replace_range`, so it is one undo step, and the host sees
//!   it as it sees typing. The selection is read once, in UTF-16, and
//!   converted.
//! - **Table controls**, drawn only while the caret is in a pipe table: the
//!   editor's table methods (`table`).
//!
//! **Active state**, the Style control's name for the caret line and the
//! marks around the selection, is read again when the host says the caret
//! or the text moved, not as each frame is drawn. In Markdown mode the
//! formatting controls are drawn at 35% and keep working.
//!
//! **Keys** (fact 67). Each control is focusable, named, and has a tooltip
//! naming its key. Left and Right move along the toolbar, Home and End go
//! to its ends, and Tab enters it at its first control, its one tab stop.
//! A click leaves focus where it was, in the editor. The mode key works
//! from a control too: the toolbar forwards it to the host.
//!
//! The status item on the right is an empty slot until Task 14.

#![cfg_attr(not(test), allow(dead_code))] // Called from Task 14.

pub(crate) mod controls;
mod popovers;
mod render;
mod table;

#[cfg(test)]
use std::collections::HashMap;

use gpui_kit::{
    Action, App, Context, Entity, EntityInputHandler, FocusHandle, Focusable, Subscription, Window,
};
use zorite_editor::{Bold, Code, EditorState, Italic, Redo, Strike, Undo};

use self::{
    controls::{COUNT, Control, MAIN, TABLE},
    popovers::Popover,
};
use super::host::{EditorHost, EditorHostEvent};
use crate::desktop::model::markdown_edit::{
    BlockStyle, HostEdit, LinePrefix, Mark, Selection, active_marks, block_style_at, code_block,
    insert_table, selection_from_utf16, table_at, toggle_prefix,
};

/// What the toolbar shows of the caret and the selection.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Active {
    /// The caret line's style, as the Style control names it.
    style: BlockStyle,
    /// The marks the editor's toggles would take off the selection.
    marks: Vec<Mark>,
    /// The caret is on a row of a pipe table.
    in_table: bool,
}

/// A move along the toolbar, or through a menu.
#[derive(Clone, Copy)]
enum Step {
    Next,
    Previous,
    First,
    Last,
}

impl Step {
    /// Where the move from `at` lands among `len` items, stopping at the
    /// ends.
    fn apply(self, at: usize, len: usize) -> usize {
        match self {
            Self::Next => (at + 1).min(len.saturating_sub(1)),
            Self::Previous => at.saturating_sub(1),
            Self::First => 0,
            Self::Last => len.saturating_sub(1),
        }
    }
}

/// The edit toolbar of one host.
pub(crate) struct Toolbar {
    host: Entity<EditorHost>,
    editor: Entity<EditorState>,
    /// One per control; the first control's is the toolbar's tab stop.
    handles: Vec<FocusHandle>,
    active: Active,
    popover: Option<Popover>,
    /// The opacity each control was drawn at in the last frame.
    #[cfg(test)]
    drawn: HashMap<Control, f32>,
    /// How often the active state was read.
    #[cfg(test)]
    refreshes: usize,
    _subscriptions: [Subscription; 2],
}

impl Toolbar {
    /// The toolbar of `host`, its state read from the host's editor.
    pub(crate) fn new(
        host: Entity<EditorHost>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = host.read(cx).editor().clone();
        let subscriptions = [
            cx.subscribe_in(&host, window, Self::on_host_event),
            // The mode, and anything else the host draws by.
            cx.observe(&host, |_, _, cx| cx.notify()),
        ];
        let mut toolbar = Self {
            host,
            editor,
            handles: (0..COUNT)
                .map(|ix| cx.focus_handle().tab_stop(ix == 0))
                .collect(),
            active: Active {
                style: BlockStyle::Paragraph,
                marks: Vec::new(),
                in_table: false,
            },
            popover: None,
            #[cfg(test)]
            drawn: HashMap::new(),
            #[cfg(test)]
            refreshes: 0,
            _subscriptions: subscriptions,
        };
        toolbar.refresh(window, cx);
        toolbar
    }

    fn on_host_event(
        &mut self,
        _: &Entity<EditorHost>,
        event: &EditorHostEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            EditorHostEvent::Edited | EditorHostEvent::SelectionChanged => self.refresh(window, cx),
            EditorHostEvent::LinkRequested => self.toggle_link(window, cx),
            EditorHostEvent::DirtyChanged(_) => {}
        }
    }

    /// Reads the active state again from the editor's text and selection.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selection = self.selection(window, cx);
        let editor = self.editor.read(cx);
        let text = editor.text();
        let active = Active {
            style: block_style_at(text, selection.start),
            marks: active_marks(text, selection),
            in_table: table_at(text, editor.cursor()),
        };
        #[cfg(test)]
        {
            self.refreshes += 1;
        }
        if active != self.active {
            self.active = active;
            cx.notify();
        }
    }

    /// The editor's selection, read in UTF-16 and converted once.
    fn selection(&self, window: &mut Window, cx: &mut App) -> Selection {
        let utf16 = self
            .editor
            .update(cx, |editor, cx| {
                editor.selected_text_range(false, window, cx)
            })
            .map_or(0..0, |selection| selection.range);
        selection_from_utf16(self.editor.read(cx).text(), utf16)
    }

    fn handle(&self, control: Control) -> FocusHandle {
        self.handles[control.index()].clone()
    }

    /// The controls drawn now, in order.
    fn visible(&self) -> Vec<Control> {
        let table = TABLE.iter().filter(|_| self.active.in_table);
        MAIN.iter().chain(table).copied().collect()
    }

    /// Does what `control` names.
    fn activate(&mut self, control: Control, window: &mut Window, cx: &mut Context<Self>) {
        let prefix = |prefix| move |text: &str, selection| toggle_prefix(text, selection, prefix);
        match control {
            Control::Rich | Control::Markdown => {
                let rich = control == Control::Rich;
                self.host.update(cx, |host, cx| host.set_rich(rich, cx));
            }
            Control::Undo => self.dispatch(&Undo, window, cx),
            Control::Redo => self.dispatch(&Redo, window, cx),
            Control::Bold => self.dispatch(&Bold, window, cx),
            Control::Italic => self.dispatch(&Italic, window, cx),
            Control::Strike => self.dispatch(&Strike, window, cx),
            Control::Code => self.dispatch(&Code, window, cx),
            Control::Style => self.toggle_style(window, cx),
            Control::Link => self.toggle_link(window, cx),
            Control::Bullets => self.host_edit(window, cx, prefix(LinePrefix::Bullet)),
            Control::Numbered => self.host_edit(window, cx, prefix(LinePrefix::Numbered)),
            Control::Checklist => self.host_edit(window, cx, prefix(LinePrefix::Task)),
            Control::Quote => self.host_edit(window, cx, prefix(LinePrefix::Quote)),
            Control::CodeBlock => self.host_edit(window, cx, code_block),
            Control::Table => self.host_edit(window, cx, insert_table),
            Control::RowBelow
            | Control::ColumnRight
            | Control::DeleteRow
            | Control::DeleteColumn
            | Control::AlignLeft
            | Control::AlignCentre
            | Control::AlignRight => self.table(control, cx),
        }
    }

    /// One of the editor's own actions, on the editor wherever focus is.
    fn dispatch(&self, action: &dyn Action, window: &mut Window, cx: &mut App) {
        let editor = self.editor.read(cx).focus_handle(cx);
        editor.dispatch_action(action, window, cx);
    }

    /// The host edit `edit` makes of the text and the selection.
    fn host_edit(
        &self,
        window: &mut Window,
        cx: &mut App,
        edit: impl FnOnce(&str, Selection) -> HostEdit,
    ) {
        let selection = self.selection(window, cx);
        let edit = edit(self.editor.read(cx).text(), selection);
        apply(&self.editor, edit, cx);
    }

    /// Moves focus along the drawn controls, from the focused one.
    fn step(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let visible = self.visible();
        let Some(at) = visible
            .iter()
            .position(|control| self.handles[control.index()].is_focused(window))
        else {
            return;
        };
        let to = visible[step.apply(at, visible.len())];
        window.focus(&self.handle(to), cx);
    }

    /// As a frame is drawn: a focused table control the caret has left goes
    /// with the table's other controls, so focus moves to Table.
    fn keep_focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        let lost = !self.active.in_table
            && TABLE
                .iter()
                .any(|control| self.handles[control.index()].is_focused(window));
        if lost {
            let table = self.handle(Control::Table);
            cx.defer_in(window, move |_, window, cx| window.focus(&table, cx));
        }
    }

    #[cfg(test)]
    fn drawn(&self, control: Control) -> Option<f32> {
        self.drawn.get(&control).copied()
    }

    #[cfg(test)]
    fn refreshes(&self) -> usize {
        self.refreshes
    }
}

/// Puts `edit` in as one `replace_range`, one undo step, and the caret
/// where it says. An edit that changes nothing is left out, so it leaves
/// no empty step to undo.
fn apply(editor: &Entity<EditorState>, edit: HostEdit, cx: &mut App) {
    editor.update(cx, |editor, cx| {
        if editor.text().get(edit.range.clone()) == Some(edit.replacement.as_str()) {
            return;
        }
        editor.replace_range(edit.range, &edit.replacement, cx);
        editor.set_cursor(edit.cursor, cx);
    });
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "toolbar_tests.rs"]
mod tests;
