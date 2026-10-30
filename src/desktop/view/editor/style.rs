//! The editor's rich-text style, from the page tokens (design, "The editor
//! host": Style), for read and edit editors alike. `SyntaxStyle` has 25
//! fields and no `Default` (fact 80), so each is set here. The page
//! wrapper sets the font size (15 px) and `page-text`; the editor takes
//! both from the surrounding text style.

use gpui_kit::{App, component::ActiveTheme, font};
use zorite_editor::SyntaxStyle;

use crate::desktop::view::item::tokens::item_colors;

/// The style of every editor on the page:
/// - links and tags in `page-teal`;
/// - inline code in `page-text` on `code-inline`;
/// - quotes in `page-text-secondary`;
/// - markers and rules in `page-border`;
/// - `<mark>` on `selection`;
/// - code in the theme's monospace font;
/// - menus on the chrome's popover surface, since they open over the
///   chrome as well as the page;
/// - alerts in the nearest page tokens: notes teal, tips green, important
///   the lighter teal, warnings amber, and cautions the deeper amber, the
///   page having no red;
/// - no alert icons, block labels, reference counts or property icons,
///   which D3 does not use.
pub(crate) fn page_style(cx: &App) -> SyntaxStyle {
    let page = item_colors(cx);
    let theme = cx.theme();
    SyntaxStyle {
        marker: page.page_border,
        code: page.page_text,
        code_bg: page.code_inline,
        link: page.page_teal,
        tag: page.page_teal,
        quote: page.page_text_secondary,
        alert_note: page.page_teal,
        alert_tip: page.page_green,
        alert_important: page.page_teal_hover,
        alert_warning: page.page_amber,
        alert_caution: page.page_amber_text,
        alert_icons: None,
        rule: page.page_border,
        mark_bg: page.selection,
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
