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
//!   reverted exactly (R-79) in the same update, before anything is
//!   painted (fact 76).
//!
//! Moving, selecting, Select All, Copy and link clicks work. The wrapper
//! emits no editor event: a writer subscribes to it, never to the inner
//! editor's own `Changed`. A clicked link opens in the browser from here
//! when it is for the browser, and every clicked link goes to the owner as
//! a `LinkTarget` (`clicked_link`, decision 10).
//!
//! The wrapper is the read view's tab stop. The editor makes its own
//! focus with GPUI's default, which Tab passes by, so Tab lands on the
//! wrapper, which hands focus to the editor. Shift-Tab leaves from the
//! wrapper's place, or it would land on the wrapper again.
//!
//! Given its tab's images (`show_images`), it draws the local images its
//! text names from them, and starts the loads each frame asked for once
//! that frame is drawn (decision 9).

use gpui_kit::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, Keystroke, MouseButton, MouseDownEvent, ParentElement, Render, Subscription,
    Window, div,
};
use zorite_editor::{
    Backspace, Bold, Code, Cut, Delete, Down, EditorEvent, EditorState, Indent, Italic, Left,
    Newline, Outdent, Paste, Redo, Right, ShowCharacterPalette, Strike, Underline, Undo, Up,
    WordLeft, WordRight,
};

use super::{clicked_link, entered_by_pointer, open_editor, step_over_blocks};
use crate::desktop::{
    model::{editor_text::crlf_to_lf, links::LinkTarget},
    services::images::TabImages,
};

/// A read editor and its guard.
pub(crate) struct ReadOnly {
    editor: Entity<EditorState>,
    /// The read view's tab stop, which hands focus to the editor.
    focus: FocusHandle,
    /// The text the editor shows: `editor_text::load` of the body.
    shown: String,
    /// The caret while the text was `shown`, restored with it.
    cursor: usize,
    /// The last input that can enter a block was a pointer press, not a
    /// key (R-66).
    pressed: bool,
    /// The tab's images, once given.
    images: Option<TabImages>,
    _subscriptions: [Subscription; 2],
}

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
            focus: cx.focus_handle().tab_stop(true),
            shown,
            cursor,
            pressed: false,
            images: None,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// Draws the local images the text names from `images`, the tab's.
    pub(crate) fn show_images(&mut self, images: &TabImages, cx: &mut Context<Self>) {
        images.attach(&self.editor, cx);
        self.images = Some(images.clone());
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
            EditorEvent::OpenLink(_) | EditorEvent::OpenWikiLink(_) => {
                if let Some(target) = clicked_link(event, cx) {
                    cx.emit(target);
                }
            }
            _ => {}
        }
    }

    /// The backstop: text that is not `shown` is put back, with the caret
    /// where it was; while the text is `shown`, its caret is remembered.
    /// Subscriptions run before the next frame, so a revert is never
    /// painted.
    ///
    /// The revert is exact (R-79): a whole-text `replace_range`, which does
    /// not normalize. `set_text` would load `shown` again, and the pinned
    /// rewrite is not idempotent (fact 81), so on some bodies the text
    /// would never equal `shown` and the revert would never stop. The step
    /// `replace_range` records is unreachable, because undo is stopped.
    fn hold(&mut self, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        if editor.text() == self.shown {
            self.cursor = editor.cursor();
            return;
        }
        let whole = 0..editor.text().len();
        let (shown, cursor) = (self.shown.clone(), self.cursor);
        self.editor.update(cx, |editor, cx| {
            editor.replace_range(whole, &shown, cx);
            editor.set_cursor(cursor, cx);
        });
    }
}

impl EventEmitter<EditorEvent> for ReadOnly {}

/// A clicked link, for the owner. One for the browser is open already.
impl EventEmitter<LinkTarget> for ReadOnly {}

impl Render for ReadOnly {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus.is_focused(window) {
            // Tab landed on the wrapper: focus belongs in the editor. It
            // moves once this frame is drawn, as a focus listener's would;
            // GPUI sends those only to an active window.
            cx.defer_in(window, |this, window, cx| {
                if this.focus.is_focused(window) {
                    this.editor
                        .update(cx, |editor, cx| editor.focus(window, cx));
                }
            });
        }
        if let Some(images) = &self.images {
            images.after_frame(cx);
        }
        div()
            .track_focus(&self.focus)
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
            // From the wrapper's place: the previous tab stop from the
            // editor's own is the wrapper.
            .capture_action(cx.listener(|this, _: &Outdent, window, cx| {
                cx.stop_propagation();
                window.focus(&this.focus, cx);
                window.focus_prev(cx);
            }))
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
fn stop<A: Action>(_: &A, _: &mut Window, cx: &mut App) {
    cx.stop_propagation();
}

/// Notes a key move, one of those that can enter a block, and lets the
/// editor make it.
fn moved<A: Action>(cx: &mut Context<ReadOnly>) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    cx.listener(|this, _: &A, _, _| this.pressed = false)
}

/// A keystroke inserts text when it carries a character and no Control,
/// Command or Function modifier.
fn inserts_text(keystroke: &Keystroke) -> bool {
    let modifiers = &keystroke.modifiers;
    keystroke.key_char.is_some() && !modifiers.control && !modifiers.platform && !modifiers.function
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "read_only_tests.rs"]
mod tests;
