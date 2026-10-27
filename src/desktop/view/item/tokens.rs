//! The item view's colours (R-51), from `docs/design/item_view/05-tokens-additions.md`.
//!
//! The header tokens follow the theme's mode; the page tokens are the same
//! in both modes, since the page is white in both. Chrome colours (chips,
//! buttons) come from the current Kit theme. `accent` and `faint` are the
//! workspace handoff's chrome tokens for the tab strip's amber and its
//! unfocused line, which the closed Kit theme does not carry.

use gpui_kit::{App, Hsla, component::ActiveTheme, rgb};

/// The item view's colours for one mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ItemColors {
    pub header_block: Hsla,
    pub header_border: Hsla,
    pub header_title: Hsla,
    /// Brand amber: the tab strip's active icon and focused line.
    pub accent: Hsla,
    /// The unfocused tab line.
    pub faint: Hsla,
    pub page: Hsla,
    pub page_text: Hsla,
    pub page_text_secondary: Hsla,
    pub page_muted: Hsla,
    pub page_border: Hsla,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 8.
    pub page_border_soft: Hsla,
    pub page_border_strong: Hsla,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 10b.
    pub page_fill: Hsla,
    pub code_inline: Hsla,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 8.
    pub discussion: Hsla,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 10b.
    pub toolbar_hover: Hsla,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 9.
    pub segmented: Hsla,
    pub selection: Hsla,
    pub page_teal: Hsla,
    pub page_teal_hover: Hsla,
    pub page_amber: Hsla,
    pub page_amber_text: Hsla,
    pub page_green: Hsla,
}

fn hex(value: u32) -> Hsla {
    rgb(value).into()
}

/// The colours for dark (`true`) or light (`false`) mode.
pub(crate) fn colors(dark: bool) -> ItemColors {
    let (header_block, header_border, header_title, accent, faint) = if dark {
        (0x262a30, 0x353a41, 0xeef0f2, 0xe9ad36, 0x4a5059)
    } else {
        (0xebe8e3, 0xe2dfda, 0x15181c, 0xa86f00, 0xb5b0a8)
    };
    ItemColors {
        header_block: hex(header_block),
        header_border: hex(header_border),
        header_title: hex(header_title),
        accent: hex(accent),
        faint: hex(faint),
        page: hex(0xffffff),
        page_text: hex(0x1f2328),
        page_text_secondary: hex(0x575c64),
        page_muted: hex(0x767a82),
        page_border: hex(0xdedbd5),
        page_border_soft: hex(0xefede9),
        page_border_strong: hex(0xcfcbc4),
        page_fill: hex(0xf6f5f2),
        code_inline: hex(0xf3f2ef),
        discussion: hex(0xf8f7f5),
        toolbar_hover: hex(0xe2dfda),
        segmented: hex(0xebe9e5),
        selection: hex(0xfbefd6),
        page_teal: hex(0x07596a),
        page_teal_hover: hex(0x0a7a90),
        page_amber: hex(0xa86f00),
        page_amber_text: hex(0x8f5e00),
        page_green: hex(0x2c7a43),
    }
}

/// The colours for the current theme's mode.
pub(crate) fn item_colors(cx: &App) -> ItemColors {
    colors(cx.theme().is_dark())
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "tokens_tests.rs"]
mod tests;
