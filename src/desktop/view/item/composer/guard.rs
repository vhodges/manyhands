//! The composer's text area as it listens around its editor: a pointer
//! press noted for R-66, as the edit host notes one, and, while posting or
//! failed, the read-only guard. The editor has no read-only switch, so the
//! guard keeps edits out as the read editor's does (R-61), in four layers:
//! editing actions stopped in the capture phase, keystrokes that would
//! insert text stopped, right and middle presses stopped, and a backstop
//! that puts back any text that still lands, exactly (R-79), before it is
//! painted. Tab and Shift-Tab, which indent while it is editable, move
//! focus on and back instead, so the failed actions can be reached.

use gpui_kit::{
    Action, App, Context, InteractiveElement, KeyDownEvent, Keystroke, MouseButton, MouseDownEvent,
    Window,
};
use zorite_editor::{
    Backspace, Bold, Code, Cut, Delete, Down, Indent, Italic, Left, Newline, Outdent, Paste, Redo,
    Right, ShowCharacterPalette, Strike, Underline, Undo, Up, WordLeft, WordRight,
};

use super::Composer;

impl Composer {
    /// `area`, the editor's wrapper, with the press listener and, while
    /// read-only, the guard.
    pub(super) fn guarded<E: InteractiveElement>(&self, area: E, cx: &mut Context<Self>) -> E {
        let area = area
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                if this.is_read_only()
                    && matches!(event.button, MouseButton::Right | MouseButton::Middle)
                {
                    cx.stop_propagation();
                    return;
                }
                this.pressed = true;
                this.settling = true;
                // A frame follows every press, so `settling` ends with it,
                // even after a press the editor draws nothing for.
                cx.notify();
            }))
            .capture_action(moved::<Left>(cx))
            .capture_action(moved::<Right>(cx))
            .capture_action(moved::<Up>(cx))
            .capture_action(moved::<Down>(cx))
            .capture_action(moved::<WordLeft>(cx))
            .capture_action(moved::<WordRight>(cx));
        if !self.is_read_only() {
            return area;
        }
        area.capture_action(stop::<Backspace>)
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
            // From the area's place: the previous tab stop from the
            // editor's own is the area.
            .capture_action(cx.listener(|this, _: &Outdent, window, cx| {
                cx.stop_propagation();
                window.focus(&this.focus, cx);
                window.focus_prev(cx);
            }))
            .capture_key_down(|event: &KeyDownEvent, _, cx| {
                if inserts_text(&event.keystroke) {
                    cx.stop_propagation();
                }
            })
    }

    /// The backstop: the text is put back to the last one seen, with the
    /// caret where it was, as one whole-text `replace_range`, which does
    /// not normalize (R-79).
    pub(super) fn hold(&mut self, cx: &mut Context<Self>) {
        let whole = 0..self.editor.read(cx).text().len();
        let (last, cursor) = (self.last.clone(), self.cursor);
        self.editor.update(cx, |editor, cx| {
            editor.replace_range(whole, &last, cx);
            editor.set_cursor(cursor, cx);
        });
    }
}

// `stop`, `moved` and `inserts_text` mirror view/editor/read_only.rs.

/// Stops an editing action before the editor sees it.
fn stop<A: Action>(_: &A, _: &mut Window, cx: &mut App) {
    cx.stop_propagation();
}

/// Notes a key move, one of those that can enter a block, before the
/// editor makes it: the last input is then a key (R-66).
fn moved<A: Action>(cx: &mut Context<Composer>) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    cx.listener(|this, _: &A, _, _| this.pressed = false)
}

/// A keystroke inserts text when it carries a character and no Control,
/// Command or Function modifier.
fn inserts_text(keystroke: &Keystroke) -> bool {
    let modifiers = &keystroke.modifiers;
    keystroke.key_char.is_some() && !modifiers.control && !modifiers.platform && !modifiers.function
}
