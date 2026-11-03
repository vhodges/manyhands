//! The discussion's comment composers (design, Lane C): the new-comment
//! composer after the threads and the one reply composer. Task 12 places
//! them; this builds the `Composer` entity, which knows nothing of drafts
//! or posting and tells its owner what happens through its events.
//!
//! - **Editor.** A small editor of its own (`open_compact_editor`), never
//!   wrapped in the edit host, whose mode key and Escape would shadow the
//!   composer's. Rich text from the start; the mode switch sets the page
//!   style or clears it, and Markdown draws in the monospace family, as
//!   the body editor does. Its text area is at least 70 px for a new
//!   comment and 64 px for a reply, and grows with its text.
//! - **Load** (R-10). A restored text, which an editor loaded once, goes in
//!   exactly (`set_exact`), and that load is no change: only a text that
//!   differs from the last one seen is `Changed`.
//! - **Keys** (decision 19). Tab and Shift-Tab indent and outdent in the
//!   editor, as it binds them; F6 and Shift-F6 leave. The mode key works
//!   from the editor and from a toolbar control. The composer is one tab
//!   stop that hands focus to its editor, as the edit host is.
//! - **Mini toolbar** (`toolbar`): "Replying to {name}", the inline marks,
//!   Link, Bulleted list, the mode switch and the draft indicator, which
//!   says what the owner set (`set_recovery`).
//! - **Footer** (`render`, R-9): its words and buttons follow the remote,
//!   the item's kind and the composer's.
//!
//! The composer holds the user's text, so nothing here derives `Debug`.

// Every item in these is first called from Task 12, which places the
// composers.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
mod link;
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
mod render;
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
pub(crate) mod toolbar;

use gpui_kit::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Subscription, Window,
};
use manyhands::drafts::ItemKind;
use zorite_editor::{EditorEvent, EditorState};

use self::{link::LinkField, toolbar::COUNT};
use super::ItemPane;
use crate::desktop::{
    model::drafts::RecoveryState,
    view::editor::{clicked_link, open_compact_editor, set_exact, step_over_blocks, style},
};

/// A new comment's placeholder.
const NEW_PLACEHOLDER: &str = "Add a comment. Markdown works here.";
/// A reply's placeholder.
const REPLY_PLACEHOLDER: &str = "Write a reply. Markdown works here.";

/// The tab's composers.
#[derive(Default)]
pub(super) struct Composers {}

impl ItemPane {
    /// The composers, at the discussion's end, after the threads and the
    /// posts being sent. Nothing yet.
    pub(super) fn composers_view(
        &self,
        _window: &Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }
}

/// Which composer this is.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
pub(crate) enum ComposerKind {
    /// The new-comment composer, after the threads.
    New,
    /// The reply composer in a thread: `to` is the name of the comment it
    /// answers, and `me` the user's identity, whose avatar it shows.
    Reply { to: String, me: Option<String> },
}

/// What the composer tells its owner. It holds the user's text, so it
/// derives nothing.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
pub(crate) enum ComposerEvent {
    /// The text changed, by any means the user has; this is the new text.
    Changed(String),
}

/// One comment composer.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
pub(crate) struct Composer {
    kind: ComposerKind,
    item: ItemKind,
    /// Whether the repository has a publication remote.
    remote: bool,
    editor: Entity<EditorState>,
    /// The composer's tab stop, which hands focus to the editor.
    focus: FocusHandle,
    rich: bool,
    /// What the owner says of the draft, if anything.
    recovery: Option<RecoveryState>,
    /// The editor's text when the composer last looked (R-10).
    last: String,
    /// One per toolbar control (`toolbar::CONTROLS`).
    controls: [FocusHandle; COUNT],
    /// The Link control's address field, while it is open.
    link: Option<LinkField>,
    _subscriptions: [Subscription; 2],
}

#[cfg_attr(not(test), allow(dead_code))] // Called from Task 12.
impl Composer {
    /// A composer of `kind` on an item of kind `item`, holding `restored`,
    /// a restored draft's text, exactly, with the caret at its end; an
    /// empty `restored` is a new composer.
    pub(crate) fn new(
        kind: ComposerKind,
        item: ItemKind,
        remote: bool,
        restored: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = placeholder(&kind);
        let editor = open_compact_editor("", placeholder, window, cx);
        if !restored.is_empty() {
            set_exact(&editor, restored, cx);
            editor.update(cx, |editor, cx| editor.set_cursor(restored.len(), cx));
        }
        let last = editor.read(cx).text().to_owned();
        let subscriptions = [
            cx.subscribe(&editor, Self::on_editor_event),
            // Undo and Redo emit no `Changed`; a notification shows them.
            cx.observe(&editor, |this, _, cx| this.compare(cx)),
        ];
        Self {
            kind,
            item,
            remote,
            editor,
            focus: cx.focus_handle().tab_stop(true),
            rich: true,
            recovery: None,
            last,
            controls: [(); COUNT].map(|_| cx.focus_handle()),
            link: None,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    pub(crate) fn text(&self, cx: &App) -> String {
        self.editor.read(cx).text().to_owned()
    }

    /// What the empty text area says.
    pub(crate) fn placeholder(&self) -> &'static str {
        placeholder(&self.kind)
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
        let style = rich.then(|| style::page_style(cx));
        self.editor.update(cx, |editor, cx| match style {
            Some(style) => editor.set_markdown_style(style, cx),
            None => editor.clear_markdown_style(cx),
        });
        cx.notify();
    }

    /// Whether the repository has a publication remote: the footer and the
    /// post button say what posting does.
    pub(crate) fn set_remote(&mut self, remote: bool, cx: &mut Context<Self>) {
        if remote != self.remote {
            self.remote = remote;
            cx.notify();
        }
    }

    /// What the draft indicator says: the owner's state of the draft, or
    /// nothing before one.
    pub(crate) fn set_recovery(&mut self, state: Option<RecoveryState>, cx: &mut Context<Self>) {
        if state != self.recovery {
            self.recovery = state;
            cx.notify();
        }
    }

    fn on_editor_event(
        &mut self,
        _: Entity<EditorState>,
        event: &EditorEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            EditorEvent::Changed => self.compare(cx),
            EditorEvent::EditMath { .. }
            | EditorEvent::EditProperties { .. }
            | EditorEvent::PreviewImage(_) => {
                step_over_blocks(&self.editor, event, cx);
            }
            EditorEvent::OpenLink(_) | EditorEvent::OpenWikiLink(_) => {
                // A link for the browser opens; the composer follows no
                // other link, since it is no page.
                clicked_link(event, cx);
            }
            _ => {}
        }
    }

    /// R-10: a text that differs from the last one seen is a change.
    fn compare(&mut self, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).text();
        if text == self.last {
            return;
        }
        self.last = text.to_owned();
        cx.emit(ComposerEvent::Changed(self.last.clone()));
        cx.notify();
    }

    /// Whether focus is anywhere in the composer: its stop, its editor, a
    /// toolbar control or the address field.
    fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.is_focused(window)
            || self.editor.read(cx).focus_handle(cx).is_focused(window)
            || self.controls.iter().any(|handle| handle.is_focused(window))
            || self
                .link
                .as_ref()
                .is_some_and(|link| link.contains_focus(window, cx))
    }
}

impl EventEmitter<ComposerEvent> for Composer {}

impl Focusable for Composer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// The placeholder of a composer of `kind`.
fn placeholder(kind: &ComposerKind) -> &'static str {
    match kind {
        ComposerKind::New => NEW_PLACEHOLDER,
        ComposerKind::Reply { .. } => REPLY_PLACEHOLDER,
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
mod test_bench;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "composer_tests.rs"]
mod tests;
