//! The read-only guard (R-61). A read editor is a real editor, styled as
//! rich text, and the editor has no read-only switch (fact 71), so this
//! wrapper keeps every edit out, in four layers:
//! - **Actions.** The capture phase stops each editing action (fact 72).
//!   Indent and Outdent move focus on and back instead, so Tab leaves.
//! - **Keystrokes.** The capture phase stops any keystroke that would
//!   insert text, so typed text never reaches the editor's input (fact 76).
//! - **Mouse.** The capture phase stops right and middle presses, so the
//!   editor's menus never open and a middle-click paste never lands.
//! - **Backstop.** Any text change that still lands — a checkbox click, an
//!   input-method commit or composition, a table drag, an image resize — is
//!   reverted in the same update, before anything is painted (fact 76).
//!
//! Moving, selecting, Select All, Copy and link clicks work. The wrapper
//! emits nothing: a writer subscribes to it, never to the inner editor's
//! own `Changed`.

use gpui_kit::{
    Action, App, Context, Entity, EventEmitter, InteractiveElement, IntoElement, KeyDownEvent,
    Keystroke, MouseButton, MouseDownEvent, ParentElement, Render, Subscription, Window, div,
};
use zorite_editor::{
    Backspace, Bold, Code, Cut, Delete, Down, EditorEvent, EditorState, Indent, Italic, Left,
    Newline, Outdent, Paste, Redo, Right, ShowCharacterPalette, Strike, Underline, Undo, Up,
    WordLeft, WordRight,
};

use super::{open_editor, step_over_blocks};
use crate::desktop::model::editor_text::crlf_to_lf;

/// A read editor and its guard.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
pub(crate) struct ReadOnly {
    editor: Entity<EditorState>,
    /// The text the editor shows: `editor_text::load` of the body.
    shown: String,
    /// The caret while the text was `shown`, restored with it.
    cursor: usize,
    /// The last input that can enter a block was a pointer press, not a
    /// key (R-66).
    pressed: bool,
    _subscriptions: [Subscription; 2],
}

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
impl ReadOnly {
    pub(crate) fn new(body: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = open_editor(body, window, cx);
        let (shown, cursor) = {
            let editor = editor.read(cx);
            (editor.text().to_owned(), editor.cursor())
        };
        let subscriptions = [
            cx.subscribe(&editor, Self::on_editor_event),
            cx.observe(&editor, |this, _, cx| this.hold(cx)),
        ];
        Self {
            editor,
            shown,
            cursor,
            pressed: false,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// R-19: a clean reload shows `body`, loaded as the editor loads it,
    /// and keeps the cursor, clamped.
    pub(crate) fn reload(&mut self, body: &str, cx: &mut Context<Self>) {
        let cursor = self.cursor(cx);
        let text = crlf_to_lf(body);
        self.editor.update(cx, |editor, cx| {
            editor.set_text(text, cx);
            editor.set_cursor(cursor, cx);
        });
        let editor = self.editor.read(cx);
        self.shown = editor.text().to_owned();
        self.cursor = editor.cursor();
    }

    /// The caret's byte offset, which Edit copies into the edit editor
    /// (R-65).
    pub(crate) fn cursor(&self, cx: &App) -> usize {
        self.editor.read(cx).cursor()
    }

    fn on_editor_event(
        &mut self,
        _: Entity<EditorState>,
        event: &EditorEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            EditorEvent::Changed => self.hold(cx),
            EditorEvent::EditMath { .. }
            | EditorEvent::EditProperties { .. }
            | EditorEvent::PreviewImage(_) => {
                let event = if std::mem::take(&mut self.pressed) {
                    entered_by_pointer(event)
                } else {
                    event.clone()
                };
                step_over_blocks(&self.editor, &event, cx);
            }
            _ => {}
        }
    }

    /// The backstop: text that is not `shown` is put back, with the caret
    /// where it was; while the text is `shown`, its caret is remembered.
    /// Subscriptions run before the next frame, so a revert is never
    /// painted.
    fn hold(&mut self, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        if editor.text() == self.shown {
            self.cursor = editor.cursor();
            return;
        }
        let (shown, cursor) = (self.shown.clone(), self.cursor);
        self.editor.update(cx, |editor, cx| {
            editor.set_text(shown, cx);
            editor.set_cursor(cursor, cx);
        });
    }
}

impl EventEmitter<EditorEvent> for ReadOnly {}

impl Render for ReadOnly {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .capture_action(stop::<Backspace>)
            .capture_action(stop::<Delete>)
            .capture_action(stop::<Newline>)
            .capture_action(stop::<Paste>)
            .capture_action(stop::<Cut>)
            .capture_action(stop::<ShowCharacterPalette>)
            .capture_action(stop::<Undo>)
            .capture_action(stop::<Redo>)
            .capture_action(stop::<Bold>)
            .capture_action(stop::<Italic>)
            .capture_action(stop::<Underline>)
            .capture_action(stop::<Strike>)
            .capture_action(stop::<Code>)
            .capture_action(|_: &Indent, window, cx| {
                cx.stop_propagation();
                window.focus_next(cx);
            })
            .capture_action(|_: &Outdent, window, cx| {
                cx.stop_propagation();
                window.focus_prev(cx);
            })
            .capture_action(moved::<Left>(cx))
            .capture_action(moved::<Right>(cx))
            .capture_action(moved::<Up>(cx))
            .capture_action(moved::<Down>(cx))
            .capture_action(moved::<WordLeft>(cx))
            .capture_action(moved::<WordRight>(cx))
            .capture_key_down(|event: &KeyDownEvent, _, cx| {
                if inserts_text(&event.keystroke) {
                    cx.stop_propagation();
                }
            })
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                match event.button {
                    MouseButton::Left => this.pressed = true,
                    MouseButton::Right | MouseButton::Middle => cx.stop_propagation(),
                    _ => {}
                }
            }))
            .child(self.editor.clone())
    }
}

/// Stops an editing action before the editor sees it.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
fn stop<A: Action>(_: &A, _: &mut Window, cx: &mut App) {
    cx.stop_propagation();
}

/// Notes a key move, one of those that can enter a block, and lets the
/// editor make it.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
fn moved<A: Action>(cx: &mut Context<ReadOnly>) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    cx.listener(|this, _: &A, _, _| this.pressed = false)
}

/// A keystroke inserts text when it carries a character and no Control,
/// Command or Function modifier.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
fn inserts_text(keystroke: &Keystroke) -> bool {
    let modifiers = &keystroke.modifiers;
    keystroke.key_char.is_some() && !modifiers.control && !modifiers.platform && !modifiers.function
}

/// The block event a pointer press caused, read as a forward entry: the
/// editor's own `at_end` cannot tell a click from a backward move (R-66).
fn entered_by_pointer(event: &EditorEvent) -> EditorEvent {
    match event.clone() {
        EditorEvent::EditMath {
            range,
            source,
            inline,
            ..
        } => EditorEvent::EditMath {
            range,
            source,
            at_end: false,
            inline,
        },
        EditorEvent::EditProperties {
            range, source, row, ..
        } => EditorEvent::EditProperties {
            range,
            source,
            at_end: false,
            row,
        },
        other => other,
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "read_only_tests.rs"]
mod tests;
