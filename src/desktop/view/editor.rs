//! The Markdown editor: how one is made, its keys, and the read-only guard
//! every read editor sits in.
//!
//! Every editor loads `editor_text::crlf_to_lf(body)`; the editor applies
//! its own math-fence rewrite, so `text()` is `editor_text::load(body)`.
//! An editor is styled as rich text from the start.

pub(crate) mod keys;
pub(crate) mod read_only;

use std::ops::Range;

use gpui_kit::{App, AppContext, Entity, Window, component::ActiveTheme, font};
use zorite_editor::{EditorEvent, EditorState, SyntaxStyle};

use crate::desktop::model::editor_text::crlf_to_lf;

/// A new editor holding `body` as rich text, its caret at the start.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 14.
pub(crate) fn open_editor(body: &str, window: &mut Window, cx: &mut App) -> Entity<EditorState> {
    let text = crlf_to_lf(body);
    let style = rich_style(cx);
    cx.new(|cx| {
        let mut editor = EditorState::new(window, cx).with_text(text);
        editor.set_markdown_style(style, cx);
        editor
    })
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

/// Where the caret lands past `range` (see `step_over_blocks`). A block's
/// range runs from the start of its first line to the end of its last.
fn beyond(text: &str, range: Range<usize>, inline: bool, forward: bool) -> Option<usize> {
    match (inline, forward) {
        (true, true) => Some(range.end),
        (true, false) => Some(range.start),
        (false, true) => text[range.end..].starts_with('\n').then_some(range.end + 1),
        (false, false) => range.start.checked_sub(1),
    }
}

/// The rich-text style until the page tokens arrive (Task 7's
/// `page_style`): the Kit theme's colours and its monospace font.
fn rich_style(cx: &App) -> SyntaxStyle {
    let theme = cx.theme();
    SyntaxStyle {
        marker: theme.muted_foreground,
        code: theme.foreground,
        code_bg: theme.muted,
        link: theme.link,
        tag: theme.link,
        quote: theme.muted_foreground,
        alert_note: theme.info,
        alert_tip: theme.success,
        alert_important: theme.primary,
        alert_warning: theme.warning,
        alert_caution: theme.danger,
        alert_icons: None,
        rule: theme.border,
        mark_bg: theme.selection,
        block_label: None,
        block_label_gen: 0,
        block_ref_count: None,
        popover_bg: theme.popover,
        popover_border: theme.border,
        popover_fg: theme.popover_foreground,
        popover_hover: theme.accent,
        popover_divider: theme.border,
        popover_danger: theme.danger,
        mono: font(theme.mono_font_family.clone()),
        property_icon: None,
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "editor_tests.rs"]
mod tests;
