//! The Markdown editor: how one is made, its style, its keys, the
//! read-only guard every read editor sits in, and the caret reveal.
//!
//! Every editor loads `editor_text::crlf_to_lf(body)`; the editor applies
//! its own math-fence rewrite, so `text()` is `editor_text::load(body)`.
//! An editor is styled as rich text from the start, from the page tokens
//! (`style`).

pub(crate) mod keys;
pub(crate) mod read_only;
pub(crate) mod style;

use std::ops::Range;

use gpui_kit::{App, AppContext, Entity, Pixels, Window, point, px};
use zorite_editor::{EditorEvent, EditorState};

use crate::desktop::{model::editor_text::crlf_to_lf, view::scroll::ContentScroll};

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
