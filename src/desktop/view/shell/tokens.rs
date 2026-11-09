//! The shell's chrome colours (Part A), from `docs/design/07-design-tokens.md`.
//!
//! The title bar, panel headers and footer use these. Only the tokens the
//! shell draws are here; the closed Kit theme does not carry them, so they
//! follow the theme's mode, as the item view's colours do. `accent` and
//! `faint` are also the item view's (`ItemColors` reads them from here).

use gpui_kit::{App, Hsla, component::ActiveTheme, rgb};

/// The chrome colours for one mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChromeColors {
    /// Title bar, panel headers, tab strip, footer.
    pub chrome: Hsla,
    /// Dock panel bodies, the active panel tab, chips.
    pub panel: Hsla,
    /// Every 1 px divider.
    pub border: Hsla,
    /// Button and input borders.
    pub border_strong: Hsla,
    /// Hover, secondary buttons, toggles on.
    pub hover: Hsla,
    /// Hidden-dock icons and the unfocused tab line.
    pub faint: Hsla,
    /// Counts, ages, placeholder text.
    pub muted: Hsla,
    /// Secondary text and icons.
    pub secondary: Hsla,
    /// Primary text.
    pub text: Hsla,
    /// The selected list row.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 4c.
    pub selection: Hsla,
    /// Brand amber: the focused tab line and the active tab icon.
    pub accent: Hsla,
    /// Links, the focus ring, in-progress.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
    pub teal: Hsla,
    /// Success, ready, up to date.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
    pub green: Hsla,
    /// Failure, blocked, conflict, attention.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 7.
    pub red: Hsla,
}

fn hex(value: u32) -> Hsla {
    rgb(value).into()
}

/// The colours for dark (`true`) or light (`false`) mode.
pub(crate) fn colors(dark: bool) -> ChromeColors {
    let pick = |dark_value: u32, light_value: u32| hex(if dark { dark_value } else { light_value });
    ChromeColors {
        chrome: pick(0x1f2226, 0xf3f2ef),
        panel: pick(0x212429, 0xf8f7f5),
        border: pick(0x2e3238, 0xdedbd5),
        border_strong: pick(0x3a3f46, 0xcfcbc4),
        hover: pick(0x2a2e34, 0xebe9e5),
        faint: pick(0x4a5059, 0xb5b0a8),
        muted: pick(0x646b75, 0x767a82),
        secondary: pick(0x8d949e, 0x575c64),
        text: pick(0xd5d8dd, 0x1f2328),
        selection: pick(0x29363a, 0xfbefd6),
        accent: pick(0xe9ad36, 0xa86f00),
        teal: pick(0x5fb4c3, 0x07596a),
        green: pick(0x7cc28f, 0x2c7a43),
        red: pick(0xe3787a, 0xb3393c),
    }
}

/// The colours for the current theme's mode.
pub(crate) fn chrome_colors(cx: &App) -> ChromeColors {
    colors(cx.theme().is_dark())
}
