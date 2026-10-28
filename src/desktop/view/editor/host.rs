//! The edit editor's host (design, "The editor host"). An item tab edits
//! in an editor of its own from Edit until the session ends, never the
//! read editor made editable (R-62).
//!
//! - **Load.** The body as every editor loads it (`open_editor`), its
//!   caret where the read editor's was when the caller passes one (R-65).
//!   The loaded text is the baseline: `editor_text::load(body)`.
//! - **Modes.** Rich text and Markdown are the same editor, with the page
//!   style set or cleared. Markdown sets the host's font to the monospace
//!   family at 13.5 px, in the page text colour. A switch never touches
//!   the text or its undo history.
//! - **Change** (R-12). On every `Changed` and every notification, the
//!   host compares the text with the last it saw. A difference is
//!   `Edited`, and when it makes the text differ from the baseline or
//!   match it again, `DirtyChanged`. Undo and Redo, which emit no
//!   `Changed`, are seen this way too.
//! - **Baseline.** `mark_saved` moves it to the saved text and nothing
//!   else (R-69); `rebase` puts the review's result in exactly (R-18,
//!   R-79); `reload_clean` loads a new body read into a clean editor
//!   (R-19).
//! - **Blocks** (R-66). In rich text the caret steps over math and
//!   property blocks as in the read editor; Markdown edits them.
//! - **Keys.** Tab and Shift-Tab indent and outdent; F6, Shift-F6 and the
//!   region keys leave. The host is the tab stop that hands focus to the
//!   editor, whose own focus Tab passes by. Escape closes an editor menu
//!   the user just opened, or else focuses the header's Done (R-59). The
//!   link key asks the toolbar for an address (`LinkRequested`).
//! - **Caret** (R-67). Kept in view in the region `place` names.
//! - **Links** (decision 10). A clicked link for the browser opens from
//!   here, and every clicked link goes to the tab as a `LinkTarget`
//!   (`clicked_link`). Markdown mode shows links as text, and the editor
//!   reports no click on them there.
//! - **Images** (decision 9). Given its tab's images (`show_images`), it
//!   draws the local images the text names from them, sharing what the
//!   tab's other editors loaded, and starts the loads each frame asked for
//!   once that frame is drawn.
//!
//! The host holds the user's text, so it derives nothing.

use std::mem;

use gpui_kit::{
    Action, App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseDownEvent, ParentElement, Render, Styled, Subscription, Window,
    component::ActiveTheme, div, px,
};
use zorite_editor::{
    Dismiss, Down, EditorEvent, EditorState, Left, Right, Up, WordLeft, WordRight,
};

use super::{
    CaretInView, clicked_link, entered_by_pointer, open_editor, set_exact, step_over_blocks,
    style::page_style,
};
use crate::desktop::{
    keys::{EDIT_HOST_CONTEXT, EditorEscape, InsertLink, ToggleRich},
    model::{
        editor_text::{crlf_to_lf, load},
        links::LinkTarget,
    },
    services::images::TabImages,
    view::{item::tokens::item_colors, scroll::ContentScroll},
};

/// Markdown mode's font size.
const MARKDOWN_SIZE: f32 = 13.5;

/// What the host tells its tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditorHostEvent {
    /// The text now differs from the baseline (`true`), or matches it
    /// again.
    DirtyChanged(bool),
    /// The text changed, by any means the user has: typing, paste, undo.
    Edited,
    /// The caret or the selection moved.
    SelectionChanged,
    /// The link key was pressed in the editor: the toolbar asks for the
    /// address.
    LinkRequested,
}

/// The edit editor and what it is compared with.
pub(crate) struct EditorHost {
    editor: Entity<EditorState>,
    /// The host's tab stop, which hands focus to the editor.
    focus: FocusHandle,
    /// The text a clean editor holds: the body as loaded, or as saved.
    baseline: String,
    /// The editor's text when the host last looked (R-12).
    last: String,
    rich: bool,
    dirty: bool,
    /// The last input was a pointer press, not a key (R-59, R-66).
    pressed: bool,
    /// A press's own events are still arriving; they do not clear
    /// `pressed`. They have all arrived when the next frame is drawn.
    settling: bool,
    /// The header's Done, which Escape focuses.
    done: Option<FocusHandle>,
    /// The caret reveal in the tab's region, once placed.
    reveal: Option<CaretInView>,
    /// The tab's images, once given.
    images: Option<TabImages>,
    /// The text style the editor was last painted in.
    #[cfg(test)]
    painted: std::rc::Rc<std::cell::RefCell<Option<gpui_kit::TextStyle>>>,
    _subscriptions: [Subscription; 2],
}

impl EditorHost {
    /// A host editing `body` in rich text, its caret at `cursor`, clamped,
    /// or at the start.
    pub(crate) fn new(
        body: &str,
        cursor: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = open_editor(body, window, cx);
        if let Some(cursor) = cursor {
            editor.update(cx, |editor, cx| editor.set_cursor(cursor, cx));
        }
        let baseline = editor.read(cx).text().to_owned();
        let subscriptions = [
            cx.subscribe(&editor, Self::on_editor_event),
            cx.observe(&editor, |this, _, cx| this.compare(cx)),
        ];
        Self {
            editor,
            focus: cx.focus_handle().tab_stop(true),
            last: baseline.clone(),
            baseline,
            rich: true,
            dirty: false,
            pressed: false,
            settling: false,
            done: None,
            reveal: None,
            images: None,
            #[cfg(test)]
            painted: Default::default(),
            _subscriptions: subscriptions,
        }
    }

    /// Where the host sits in its tab: under the header's `done`, which
    /// Escape focuses, in the region `scroll` scrolls, which keeps the
    /// caret in view.
    pub(crate) fn place(
        &mut self,
        done: FocusHandle,
        scroll: &ContentScroll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.done = Some(done);
        self.reveal = Some(CaretInView::new(&self.editor, scroll, window, cx));
    }

    /// Draws the local images the text names from `images`, the tab's,
    /// which its read and comment editors share.
    pub(crate) fn show_images(&mut self, images: &TabImages, cx: &mut Context<Self>) {
        images.attach(&self.editor, cx);
        self.images = Some(images.clone());
    }

    pub(crate) fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    pub(crate) fn text(&self, cx: &App) -> String {
        self.editor.read(cx).text().to_owned()
    }

    /// Whether the text differs from the host's own baseline. The tab reads
    /// dirty against its session's base, the saved copy, instead (Task 16
    /// fix round 1); the host's flag drives `DirtyChanged`, and tests.
    #[cfg(test)]
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn is_rich(&self) -> bool {
        self.rich
    }

    /// Rich text, or Markdown. The text and its history are untouched.
    pub(crate) fn set_rich(&mut self, rich: bool, cx: &mut Context<Self>) {
        if rich == self.rich {
            return;
        }
        self.rich = rich;
        let style = rich.then(|| page_style(cx));
        self.editor.update(cx, |editor, cx| match style {
            Some(style) => editor.set_markdown_style(style, cx),
            None => editor.clear_markdown_style(cx),
        });
        cx.notify();
    }

    /// R-69: `saved`, the text the editor still holds, was saved. It
    /// becomes the baseline; the text and its undo history stay.
    pub(crate) fn mark_saved(&mut self, saved: &str, cx: &mut Context<Self>) {
        self.baseline = saved.to_owned();
        self.update_dirty(cx);
    }

    /// R-18: the review's choices were used. `text`, its result, goes in
    /// exactly (R-79), which restarts undo; `baseline`, theirs, is what a
    /// clean editor would hold. The caret stays, clamped. The draft was
    /// written already, so this is no edit to feed.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 19.
    pub(crate) fn rebase(&mut self, baseline: String, text: &str, cx: &mut Context<Self>) {
        let cursor = self.editor.read(cx).cursor();
        set_exact(&self.editor, text, cx);
        self.editor
            .update(cx, |editor, cx| editor.set_cursor(cursor, cx));
        self.baseline = baseline;
        self.last = self.text(cx);
        self.update_dirty(cx);
    }

    /// R-19: a clean editor shows `body`, a newer read, loaded as the
    /// editor loads a body read (R-79), with the cursor kept, clamped. The
    /// same body leaves the editor alone, with its undo history: one that
    /// is the text last saved or loaded, or the body first loaded. A
    /// second load of a text the editor holds could change it (fact 81).
    /// A dirty editor is left alone too: its text is the user's.
    pub(crate) fn reload_clean(&mut self, body: &str, cx: &mut Context<Self>) {
        if self.dirty || crlf_to_lf(body) == self.baseline || load(body) == self.baseline {
            return;
        }
        let cursor = self.editor.read(cx).cursor();
        let text = crlf_to_lf(body);
        self.editor.update(cx, |editor, cx| {
            editor.set_text(text, cx);
            editor.set_cursor(cursor, cx);
        });
        self.baseline = self.text(cx);
        self.last = self.baseline.clone();
    }

    /// The text style the editor was last painted in.
    #[cfg(test)]
    pub(super) fn painted_style(&self) -> Option<gpui_kit::TextStyle> {
        self.painted.borrow().clone()
    }

    fn on_editor_event(
        &mut self,
        _: Entity<EditorState>,
        event: &EditorEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            EditorEvent::Changed => {
                self.input_seen();
                self.compare(cx);
            }
            EditorEvent::SelectionChanged => {
                self.input_seen();
                cx.emit(EditorHostEvent::SelectionChanged);
            }
            EditorEvent::EditMath { .. }
            | EditorEvent::EditProperties { .. }
            | EditorEvent::PreviewImage(_) => {
                let event = if mem::take(&mut self.pressed) {
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

    /// R-59: a change of text or caret clears a press, unless it is the
    /// press's own.
    fn input_seen(&mut self) {
        if !self.settling {
            self.pressed = false;
        }
    }

    /// R-12: a text that differs from the last one seen is an edit. String
    /// comparison looks at the lengths first.
    fn compare(&mut self, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).text();
        if text == self.last {
            return;
        }
        self.last = text.to_owned();
        cx.emit(EditorHostEvent::Edited);
        self.update_dirty(cx);
    }

    fn update_dirty(&mut self, cx: &mut Context<Self>) {
        let dirty = self.last != self.baseline;
        if dirty != self.dirty {
            self.dirty = dirty;
            cx.emit(EditorHostEvent::DirtyChanged(dirty));
        }
    }

    /// R-59: after a pointer press, Escape closes the editor's menu, if the
    /// press opened one, through `Dismiss`, which needs no binding;
    /// otherwise it focuses Done. Unplaced, it goes on to the bindings
    /// around the host.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if mem::take(&mut self.pressed) {
            let editor = self.editor.read(cx).focus_handle(cx);
            editor.dispatch_action(&Dismiss, window, cx);
        } else if let Some(done) = &self.done {
            window.focus(done, cx);
        } else {
            cx.propagate();
        }
    }

    /// An empty canvas beside the editor that notes the text style the
    /// editor is painted in.
    #[cfg(test)]
    fn probe(&self) -> impl IntoElement + use<> {
        let painted = self.painted.clone();
        gpui_kit::canvas(
            move |_, window, _| *painted.borrow_mut() = Some(window.text_style()),
            |_, _, _, _| {},
        )
        .absolute()
        .size_0()
    }
}

impl EventEmitter<EditorHostEvent> for EditorHost {}

/// A clicked link, for its tab; one for the browser is open already. It
/// is body text, so it is not an `EditorHostEvent`, which derives `Debug`.
impl EventEmitter<LinkTarget> for EditorHost {}

impl Render for EditorHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.settling = false;
        if self.focus.is_focused(window) {
            // Tab landed on the host: focus belongs in the editor. It moves
            // once this frame is drawn, as a focus listener's would; GPUI
            // sends those only to an active window.
            cx.defer_in(window, |this, window, cx| {
                if this.focus.is_focused(window) {
                    this.editor
                        .update(cx, |editor, cx| editor.focus(window, cx));
                }
            });
        }
        if let Some(reveal) = &mut self.reveal {
            reveal.note_focus(window, cx);
        }
        if let Some(images) = &self.images {
            images.after_frame(cx);
        }
        let mut host = div()
            .key_context(EDIT_HOST_CONTEXT)
            .track_focus(&self.focus)
            .w_full()
            .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
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
            .capture_action(moved::<WordRight>(cx))
            .on_action(cx.listener(|this, _: &EditorEscape, window, cx| this.escape(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleRich, _, cx| this.set_rich(!this.rich, cx)))
            .on_action(
                cx.listener(|_, _: &InsertLink, _, cx| cx.emit(EditorHostEvent::LinkRequested)),
            );
        if !self.rich {
            host = host
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(MARKDOWN_SIZE))
                .text_color(item_colors(cx).page_text);
        }
        #[cfg(test)]
        let host = host.child(self.probe());
        host.child(self.editor.clone())
    }
}

/// Notes a key move, one of those that can enter a block, before the
/// editor makes it: the last input is then a key (R-66).
fn moved<A: Action>(cx: &mut Context<EditorHost>) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    cx.listener(|this, _: &A, _, _| this.pressed = false)
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "host_tests.rs"]
mod tests;
