//! The discussion's comment composers (design, Lane C): the new-comment
//! composer after the threads and the one reply composer. `place` places
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
//! - **States** (`ComposerState`), which the owner sets: `Idle`;
//!   `Posting`, read-only, "Posting…", where Submit is refused with
//!   "Already posting" (P10); `Failed`, read-only, a red message and the
//!   actions the owner offers (`failed`), the composer deciding none of
//!   them. Read-only is a guard around the editor (`guard`).
//! - **Events.** Post (Ctrl Enter or the button) emits `Submit`, unless
//!   the text is blank; the owner posts it and sets `Posting`. Cancel
//!   (Escape or the reply's button) with text asks first (`discard`), and
//!   emits `Discard` on Discard; with none it emits `Cancel` at once.
//! - **Pointer** (R-66). A press on the editor lands the caret after a
//!   math or property block, as the edit host's does.
//!
//! Where the composers are, what their text is kept as and their caret
//! kept in view in the discussion's region are `place`'s (design,
//! Placement).
//!
//! The composer holds the user's text, so nothing here derives `Debug`.

mod discard;
mod failed;
mod guard;
mod link;
mod place;
mod render;
pub(crate) mod toolbar;

use std::mem;

use gpui_kit::{App, Context, Entity, EventEmitter, FocusHandle, Focusable, Subscription, Window};
use manyhands::drafts::ItemKind;
use zorite_editor::{EditorEvent, EditorState};

use self::{link::LinkField, toolbar::COUNT};
use crate::desktop::{
    model::drafts::RecoveryState,
    view::editor::{
        clicked_link, entered_by_pointer, open_compact_editor, set_exact, step_over_blocks, style,
    },
};

pub(crate) use self::failed::{FailedAction, FailedActions};
pub(super) use self::place::Composers;
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) use self::place::{MAYBE_SAVED, NOT_MOVED, NOT_SAVED_FINAL};

/// A new comment's placeholder.
const NEW_PLACEHOLDER: &str = "Add a comment. Markdown works here.";
/// A reply's placeholder.
const REPLY_PLACEHOLDER: &str = "Write a reply. Markdown works here.";

/// Which composer this is.
pub(crate) enum ComposerKind {
    /// The new-comment composer, after the threads.
    New,
    /// The reply composer in a thread: `to` is the name of the comment it
    /// answers, and `me` the user's identity, whose avatar it shows.
    Reply { to: String, me: Option<String> },
}

/// What the composer tells its owner. It holds the user's text, so it
/// derives nothing.
pub(crate) enum ComposerEvent {
    /// The text changed, by any means the user has; this is the new text.
    Changed(String),
    /// Post the text (`Composer::text`): it is not blank, and the composer
    /// is `Idle`. The owner sets `Posting`.
    Submit,
    /// Cancelled with no text: close, nothing to keep.
    Cancel,
    /// The user chose Discard in the discard prompt: drop the text.
    Discard,
    /// One of the actions the owner offered in `Failed`.
    Failed(FailedAction),
}

/// Where the composer stands; its owner sets it (`set_state`).
pub(crate) enum ComposerState {
    /// The text can be edited and posted.
    Idle,
    /// The text is being posted: read-only, and Submit is refused.
    Posting,
    /// The owner is checking where the comment stands before it posts or
    /// offers anything (P4, P5): read-only, "Checking…", and Submit is
    /// refused as while posting.
    Checking,
    /// A post failed: read-only, with `message` in red and the `actions`
    /// the owner offers. `discard_note`, if any, is said in the discard
    /// prompt (P7, P18).
    Failed {
        message: String,
        actions: FailedActions,
        discard_note: Option<String>,
    },
}

impl ComposerState {
    fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

/// What Submit while posting says (P10).
const ALREADY_POSTING: &str = "Already posting";

/// One comment composer.
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
    state: ComposerState,
    /// The item's local copy is being retired (D6 decision 19): no new
    /// text, and so no new comment draft. Apart from `state`, which is the
    /// owner's account of a post.
    locked: bool,
    /// What the composer says of a refused Submit (P10), until the state
    /// changes.
    refused: Option<&'static str>,
    /// The editor's caret while the text was `last`, kept while read-only.
    cursor: usize,
    /// The last input was a pointer press on the editor, not a key (R-66).
    pressed: bool,
    /// A press's own events are still arriving; they do not clear
    /// `pressed`. They have all arrived when the next frame is drawn.
    settling: bool,
    _subscriptions: [Subscription; 2],
}

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
        let (last, cursor) = {
            let editor = editor.read(cx);
            (editor.text().to_owned(), editor.cursor())
        };
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
            state: ComposerState::Idle,
            locked: false,
            refused: None,
            cursor,
            pressed: false,
            settling: false,
            _subscriptions: subscriptions,
        }
    }

    /// Whether the text is being posted, or checked before it is: it
    /// cannot move (design, Lane D, Placement).
    pub(crate) fn is_posting(&self) -> bool {
        matches!(self.state, ComposerState::Posting | ComposerState::Checking)
    }

    /// Whether the owner is checking where the comment stands.
    #[cfg_attr(not(test), allow(dead_code))] // Read by the tests.
    pub(crate) fn is_checking(&self) -> bool {
        matches!(self.state, ComposerState::Checking)
    }

    /// Says that Submit, or an action, was refused while the comment is
    /// posting (P10, R-19).
    pub(crate) fn refuse_while_posting(&mut self, cx: &mut Context<Self>) {
        self.refused = Some(ALREADY_POSTING);
        cx.notify();
    }

    /// Says why a Submit was refused.
    pub(crate) fn say_refused(&mut self, words: &'static str, cx: &mut Context<Self>) {
        self.refused = Some(words);
        cx.notify();
    }

    /// Whether the owner offers a failed post's actions.
    pub(crate) fn is_failed(&self) -> bool {
        matches!(self.state, ComposerState::Failed { .. })
    }

    /// A reply's "Replying to {name}": the comment Reply was pressed on.
    pub(crate) fn set_replying_to(&mut self, name: String, cx: &mut Context<Self>) {
        if let ComposerKind::Reply { to, .. } = &mut self.kind
            && *to != name
        {
            *to = name;
            cx.notify();
        }
    }

    /// Whether the text cannot be edited: while posting, or failed, or
    /// while the item's local copy is being retired.
    pub(crate) fn is_read_only(&self) -> bool {
        !self.state.is_idle() || self.locked
    }

    /// The item's local copy is being retired, or no longer is: the text
    /// is read-only meanwhile, put back as it was if anything lands. The
    /// owner says so on every frame, so the state follows the lock.
    pub(crate) fn set_locked(&mut self, locked: bool, cx: &mut Context<Self>) {
        if locked != self.locked {
            self.locked = locked;
            cx.notify();
        }
    }

    /// Sets where the composer stands. A read-only state closes the link
    /// field, whose Enter would edit the text.
    pub(crate) fn set_state(
        &mut self,
        state: ComposerState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state = state;
        self.refused = None;
        if self.is_read_only() {
            self.close_link(window, cx);
        }
        cx.notify();
    }

    /// Ctrl Enter or the post button: `Submit` for a text that is not
    /// blank while `Idle`. While posting it is refused, and says so (P10);
    /// a failed composer posts only through its owner's actions.
    pub(crate) fn post(&mut self, cx: &mut Context<Self>) {
        match self.state {
            ComposerState::Idle => {
                if !self.text(cx).trim().is_empty() {
                    cx.emit(ComposerEvent::Submit);
                }
            }
            ComposerState::Posting | ComposerState::Checking => {
                self.refused = Some(ALREADY_POSTING);
                cx.notify();
            }
            ComposerState::Failed { .. } => {}
        }
    }

    /// Escape or the reply's Cancel. `Idle`: with text, the discard prompt
    /// asks first; with none, `Cancel` at once. `Failed`: the discard
    /// prompt, when the owner offers Discard. `Posting`: nothing.
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match &self.state {
            ComposerState::Idle if self.text(cx).trim().is_empty() => {
                cx.emit(ComposerEvent::Cancel);
            }
            ComposerState::Idle => self.ask_discard(window, cx),
            ComposerState::Failed { actions, .. } if actions.discard => {
                self.ask_discard(window, cx);
            }
            ComposerState::Failed { .. } | ComposerState::Posting | ComposerState::Checking => {}
        }
    }

    pub(crate) fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// Whether a comment is being written here: the text is not blank, or
    /// it is posting, being checked or failed. The retirement loop leaves
    /// such an item alone (RT-3). It reads the state, not `is_read_only`,
    /// which the "Retiring…" guard also sets.
    pub(crate) fn is_writing(&self, cx: &App) -> bool {
        !self.state.is_idle() || !self.text(cx).trim().is_empty()
    }

    pub(crate) fn text(&self, cx: &App) -> String {
        self.editor.read(cx).text().to_owned()
    }

    /// What the empty text area says.
    pub(crate) fn placeholder(&self) -> &'static str {
        placeholder(&self.kind)
    }

    #[cfg_attr(not(test), allow(dead_code))] // Read by the tests.
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
            EditorEvent::Changed => {
                self.input_seen();
                self.compare(cx);
            }
            EditorEvent::SelectionChanged => self.input_seen(),
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
                // A link for the browser opens; the composer follows no
                // other link, since it is no page.
                clicked_link(event, cx);
            }
            _ => {}
        }
    }

    /// R-66: a change of text or caret clears a press, unless it is the
    /// press's own.
    fn input_seen(&mut self) {
        if !self.settling {
            self.pressed = false;
        }
    }

    /// R-10: a text that differs from the last one seen is a change. While
    /// read-only it is put back instead (`guard`).
    fn compare(&mut self, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        if editor.text() == self.last {
            self.cursor = editor.cursor();
            return;
        }
        if self.is_read_only() {
            self.hold(cx);
            return;
        }
        let text = editor.text().to_owned();
        self.cursor = editor.cursor();
        self.last = text;
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
#[path = "composer/states_tests.rs"]
mod states_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "composer_tests.rs"]
mod tests;
