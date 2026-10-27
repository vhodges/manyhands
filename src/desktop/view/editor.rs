//! The Markdown editor: how one is made, its style, its keys, the
//! read-only guard every read editor sits in, the edit editor's host and
//! its toolbar, and the caret reveal every editor on the page uses.
//!
//! Every editor loads `editor_text::crlf_to_lf(body)`; the editor applies
//! its own math-fence rewrite, so `text()` is `editor_text::load(body)`.
//! An editor is styled as rich text from the start, from the page tokens
//! (`style`). A link clicked in any of them goes through `clicked_link`.

pub(crate) mod host;
pub(crate) mod keys;
pub(crate) mod read_only;
pub(crate) mod style;
pub(crate) mod toolbar;

use std::ops::Range;

use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, Pixels, Subscription, WeakEntity, Window, point,
    px,
};
use zorite_editor::{EditorEvent, EditorState};

use crate::desktop::{
    model::{
        editor_text::crlf_to_lf,
        links::{LinkTarget, parse_link, parse_wiki_link},
    },
    view::scroll::ContentScroll,
};

/// A new editor holding `body` as rich text, its caret at the start.
pub(crate) fn open_editor(body: &str, window: &mut Window, cx: &mut App) -> Entity<EditorState> {
    let text = crlf_to_lf(body);
    let style = style::page_style(cx);
    cx.new(|cx| {
        let mut editor = EditorState::new(window, cx).with_text(text);
        editor.set_markdown_style(style, cx);
        editor
    })
}

/// R-67: keeps `editor`'s caret in view in the region `scroll` scrolls.
/// The editor never scrolls itself (fact 70), and no focus-reveal marker
/// sits on it, since a marker would move a tall editor to its top on
/// every click (fact 80). So this moves the region the least that shows
/// the caret's line: its top to the region's top when the line is above
/// it, its bottom to the region's bottom when below. Nothing moves while
/// the caret is in view, or before the first paint.
///
/// Positions are the last paint's, which a caret move or focus keeps
/// valid. The caller's own change asks for the frame that draws the new
/// offset.
pub(crate) fn reveal_caret(editor: &Entity<EditorState>, scroll: &ContentScroll, cx: &mut App) {
    let Some((top, bottom)) = caret_line(editor.read(cx)) else {
        return;
    };
    let handle = scroll.handle();
    let viewport = handle.bounds();
    if viewport.size.height <= px(0.) {
        return;
    }
    let offset = handle.offset();
    let y = if top < viewport.top() {
        offset.y + (viewport.top() - top)
    } else if bottom > viewport.bottom() {
        offset.y - (bottom - viewport.bottom())
    } else {
        return;
    };
    let y = y.min(px(0.)).max(-handle.max_offset().y);
    if y != offset.y {
        handle.set_offset(point(offset.x, y));
    }
}

/// R-67 for one editor in a scroll region: its caret is brought into view
/// after each caret move or change of text while it has focus, and when
/// it takes focus. Other notifications, such as the pointer passing over
/// it, move nothing, and nothing moves while it is unfocused.
///
/// The owner makes one beside its editor and calls `note_focus` as each
/// of its frames is drawn: GPUI sends focus events only to an active
/// window, which a test window never is, so a frame notes the focus
/// instead, and the reveal runs once that frame is drawn, from its
/// positions.
pub(crate) struct CaretInView {
    editor: WeakEntity<EditorState>,
    scroll: ContentScroll,
    /// Whether the editor had focus in the last frame.
    focused: bool,
    _moved: Subscription,
}

impl CaretInView {
    pub(crate) fn new<T: 'static>(
        editor: &Entity<EditorState>,
        scroll: &ContentScroll,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> Self {
        let mut seen = caret(editor.read(cx));
        let region = scroll.clone();
        let moved = cx.observe_in(editor, window, move |_, editor, window, cx| {
            let now = caret(editor.read(cx));
            if now == seen {
                return;
            }
            seen = now;
            if editor.read(cx).focus_handle(cx).is_focused(window) {
                reveal_caret(&editor, &region, cx);
            }
        });
        Self {
            editor: editor.downgrade(),
            scroll: scroll.clone(),
            focused: false,
            _moved: moved,
        }
    }

    /// As the owner's frame is drawn: when the editor has taken focus
    /// since the last frame, its caret is revealed once this frame is
    /// drawn, and the owner is notified for the frame that shows it.
    pub(crate) fn note_focus<T: 'static>(&mut self, window: &Window, cx: &mut Context<T>) {
        let focused = self
            .editor
            .upgrade()
            .is_some_and(|editor| editor.read(cx).focus_handle(cx).is_focused(window));
        if focused && !self.focused {
            let (editor, scroll) = (self.editor.clone(), self.scroll.clone());
            cx.defer_in(window, move |_, _, cx| {
                if let Some(editor) = editor.upgrade() {
                    reveal_caret(&editor, &scroll, cx);
                    cx.notify();
                }
            });
        }
        self.focused = focused;
    }
}

/// The caret and the text's length, which change when the caret moves or
/// the text changes.
fn caret(editor: &EditorState) -> (usize, usize) {
    (editor.cursor(), editor.text().len())
}

/// The window-space top and bottom of the caret's line, from the last
/// paint. A line far outside the window is laid out but not shaped
/// (fact 70), so the caret has no box there; its line's top and its first
/// row's height stand in.
fn caret_line(editor: &EditorState) -> Option<(Pixels, Pixels)> {
    if let Some(caret) = editor.caret_screen_bounds() {
        return Some((caret.top(), caret.bottom()));
    }
    let cursor = editor.cursor();
    let top = editor.offset_screen_top(cursor)?;
    let row = editor.text().as_bytes()[..cursor]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count();
    let (_, height) = editor.row_layout().get(row).copied()?;
    Some((top, top + height))
}

/// R-66: in rich text the editor cannot move its caret across a math or
/// property block by itself; it emits `EditMath` or `EditProperties` and
/// stays where it is (fact 77). This moves the caret across the block the
/// event names, and does nothing for `PreviewImage`. It answers whether
/// `event` was one of those three.
///
/// `at_end: false` is a forward entry, which lands after the block: at the
/// start of the line after it, or after an inline formula. `at_end: true`
/// is a backward one, which lands before it: at the end of the line before
/// it, or before an inline formula. The editor also says `at_end: true`
/// for a click on math, so a host that saw a pointer press passes the
/// event with `at_end: false`. A block with no line beyond it on that
/// side leaves the caret where it is.
pub(crate) fn step_over_blocks(
    editor: &Entity<EditorState>,
    event: &EditorEvent,
    cx: &mut App,
) -> bool {
    let (range, inline, forward) = match event {
        EditorEvent::EditMath {
            range,
            at_end,
            inline,
            ..
        } => (range.clone(), *inline, !at_end),
        EditorEvent::EditProperties { range, at_end, .. } => (range.clone(), false, !at_end),
        EditorEvent::PreviewImage(_) => return true,
        _ => return false,
    };
    editor.update(cx, |editor, cx| {
        if let Some(target) = beyond(editor.text(), range, inline, forward) {
            editor.set_cursor(target, cx);
        }
    });
    true
}

/// Decision 10, in every editor: a click on a link (`OpenLink`, parsed as
/// a link's text, or `OpenWikiLink`, as a title) is opened here when it is
/// for the browser, `http`, `https` or `mailto`, and dropped when it names
/// nothing to open. Any other target, a path, a title or a scheme that is
/// not opened, is the editor's owner's, which resolves it or says why
/// nothing opened. Any other event gives none. Links open by pointer
/// only: the editor emits these on a left click alone (fact 35).
pub(crate) fn clicked_link(event: &EditorEvent, cx: &mut App) -> Option<LinkTarget> {
    let target = match event {
        EditorEvent::OpenLink(raw) => parse_link(raw),
        EditorEvent::OpenWikiLink(raw) => parse_wiki_link(raw),
        _ => return None,
    };
    match target {
        LinkTarget::External(url) => {
            cx.open_url(&url);
            None
        }
        LinkTarget::Ignored => None,
        target => Some(target),
    }
}

/// The block event a pointer press caused, read as a forward entry: the
/// editor's own `at_end` cannot tell a click from a backward move (R-66).
/// A host that tracks its last input passes this to `step_over_blocks`
/// after a press.
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

/// R-79: puts `text`, which an editor already loaded once (a restored
/// draft, the review's result), into `editor` exactly. `set_text` loads
/// its argument again, and the pinned rewrite is not idempotent (fact 81),
/// so when that changes `text`, a whole-text `replace_range`, which does
/// not normalize, puts `text` back.
///
/// Undo restarts, except after that replace: then one undo step leads to
/// the editor's own second pass of `text` (the design's Known Limits).
/// The caret is at the start after `set_text` alone, and at the end after
/// the replace; a caller that cares sets it.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 18.
pub(crate) fn set_exact(editor: &Entity<EditorState>, text: &str, cx: &mut App) {
    editor.update(cx, |editor, cx| {
        editor.set_text(text, cx);
        if editor.text() != text {
            let whole = 0..editor.text().len();
            editor.replace_range(whole, text, cx);
        }
    });
}

/// Where the caret lands past `range` (see `step_over_blocks`). A block's
/// range runs from the start of its first line to the end of its last. A
/// range that no longer fits `text`, from an event older than the last
/// edit, lands nowhere.
fn beyond(text: &str, range: Range<usize>, inline: bool, forward: bool) -> Option<usize> {
    text.get(range.clone())?;
    match (inline, forward) {
        (true, true) => Some(range.end),
        (true, false) => Some(range.start),
        (false, true) => text
            .get(range.end..)
            .is_some_and(|rest| rest.starts_with('\n'))
            .then_some(range.end + 1),
        (false, false) => range.start.checked_sub(1),
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "editor_tests.rs"]
mod tests;
