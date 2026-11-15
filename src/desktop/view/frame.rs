//! The common frame of dialogs and sheets (Wave 04 Cycle 05a design A1;
//! `05`, "Common sheet frame"): a header with an optional icon tile, its
//! title and description; a body with `05`'s padding and gap; and a footer
//! with an optional hint on the left and its buttons right-aligned.
//!
//! A dialog's header goes in the Kit's title slot (`dialog`), and holds no
//! control. Its body and footer are drawn by the dialog's own view, the
//! footer as the last child of its body element (R-19), never in the Kit's
//! `footer` slot: that slot sits outside the body, where the focus-gated
//! closes and the privacy sweeps, which find buttons by their path inside
//! the body, would not reach them. In a dialog taller than the window the
//! footer scrolls with the body (K-11). A sheet's header and footer go in
//! the Kit's sheet slots, whose rows the Kit pads, so their dividers stop
//! short of the edges (K-5).
//!
//! Every element id the caller passes is kept (R-4). The frame sets no
//! `on_ok`, `on_cancel` or `on_close`: each dialog keeps its own (A3).

pub(crate) mod theme;

use gpui_kit::{
    AnyElement, App, Div, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt,
    assets::IconName,
    component::{Icon, dialog::Dialog, h_flex, v_flex},
    div, px,
};

use super::shell::tokens::{ChromeColors, chrome_colors};

/// The title's size and the description's (`05`).
const TITLE_SIZE: f32 = 15.;
const DESCRIPTION_SIZE: f32 = 12.5;
/// The icon tile (`05`, 2c and 2h): its side, its corners, its icon's size
/// (`07`'s dialog icons) and its tint (`07`'s 13%).
const TILE: f32 = 32.;
const TILE_RADIUS: f32 = 8.;
const TILE_ICON: f32 = 16.;
const TILE_TINT: f32 = 0.13;
/// Between the tile and the words, and between the title and description.
const TILE_GAP: f32 = 12.;
const LINE_GAP: f32 = 2.;
/// The header's padding (`05`): top, sides, bottom.
const HEADER_PADDING: (f32, f32, f32) = (18., 20., 14.);
/// The body's padding (`05`): top and bottom, sides; and its gap.
const BODY_PADDING: (f32, f32) = (18., 20.);
const BODY_GAP: f32 = 18.;
/// The footer's padding (`05`): top and bottom, sides; and the gap between
/// its buttons.
const FOOTER_PADDING: (f32, f32) = (12., 20.);
const BUTTON_GAP: f32 = 8.;
/// The gap the Kit draws between a dialog's title slot and its body, which
/// the body's top padding gives back.
const KIT_TITLE_GAP: f32 = 8.;
/// The top and bottom padding the Kit keeps on a sheet's title row, which
/// the header's top padding gives back.
const KIT_SHEET_TITLE_PADDING: f32 = 8.;

/// The hue of a header's icon tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    /// Amber: Unlock key's `lock` (2h).
    Accent,
    /// Red: a step that failed (2e).
    Danger,
    /// Green: done (2c).
    Success,
}

impl Tone {
    fn hue(self, colors: &ChromeColors) -> Hsla {
        match self {
            Self::Accent => colors.accent,
            Self::Danger => colors.red,
            Self::Success => colors.green,
        }
    }
}

/// A dialog's or sheet's header (`05`, "Common sheet frame").
pub(crate) struct Header {
    /// A 32 px icon tile, as `05` draws for Unlock key (2h).
    pub icon: Option<(IconName, Tone)>,
    pub title: SharedString,
    pub description: Option<SharedString>,
    /// The element ids the title and description keep (tests find them).
    pub ids: (&'static str, Option<&'static str>),
}

impl Header {
    /// The tile, the title and the description, without the padding and
    /// divider each slot draws.
    fn content(self, colors: &ChromeColors) -> Div {
        let (title_id, description_id) = self.ids;
        let words = v_flex()
            .flex_1()
            .min_w_0()
            .gap(px(LINE_GAP))
            .child(
                div()
                    .text_size(px(TITLE_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(line(Some(title_id), self.title)),
            )
            .children(self.description.map(|description| {
                // The Kit's title slot is semibold: the description says
                // its own weight.
                div()
                    .text_size(px(DESCRIPTION_SIZE))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(colors.secondary)
                    .child(line(description_id, description))
            }));
        h_flex()
            .items_start()
            .gap(px(TILE_GAP))
            .children(self.icon.map(|(icon, tone)| tile(icon, tone, colors)))
            .child(words)
    }
}

/// A footer: an optional hint on the left, then the buttons, right-aligned.
pub(crate) struct Footer {
    pub hint: Option<&'static str>,
    /// Drawn in this order (A3).
    pub buttons: Vec<AnyElement>,
}

impl Footer {
    /// The hint and the buttons in one row, without the padding and divider
    /// each slot draws.
    fn row(self, colors: &ChromeColors) -> Div {
        h_flex()
            .w_full()
            .justify_end()
            .items_center()
            .gap(px(BUTTON_GAP))
            .children(self.hint.map(|hint| {
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(colors.muted)
                    .child(hint)
            }))
            .children(self.buttons)
    }
}

/// `value`, drawn with the id `id` and labelled with itself, so a test
/// reads it; plain where it has no id.
fn line(id: Option<&'static str>, value: SharedString) -> AnyElement {
    match id {
        Some(id) => div()
            .id(id)
            .test_support()
            .aria_label(value.clone())
            .child(value)
            .into_any_element(),
        None => div().child(value).into_any_element(),
    }
}

/// The 32 px tile `icon` is drawn on, in `tone`.
fn tile(icon: IconName, tone: Tone, colors: &ChromeColors) -> Div {
    let hue = tone.hue(colors);
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(TILE))
        .rounded(px(TILE_RADIUS))
        .bg(hue.opacity(TILE_TINT))
        .child(
            Icon::new(icon)
                .w(px(TILE_ICON))
                .h(px(TILE_ICON))
                .text_color(hue),
        )
}

/// Puts `header` into a Kit `Dialog`'s title slot and zeroes the dialog's
/// padding, so the dialog's own view draws `05`'s body and footer edge to
/// edge (`body`, `footer`).
pub(crate) fn dialog(base: Dialog, header: Header, cx: &App) -> Dialog {
    let colors = chrome_colors(cx);
    let (top, sides, bottom) = HEADER_PADDING;
    base.p_0().title(
        header
            .content(&colors)
            .w_full()
            .pt(px(top))
            .px(px(sides))
            .pb(px(bottom))
            .border_b_1()
            .border_color(colors.border),
    )
}

/// The body part of a dialog's own view: `05`'s padding and gap. Its top
/// padding gives back the gap the Kit draws below the title slot.
pub(crate) fn body(content: impl IntoElement) -> Div {
    let (vertical, sides) = BODY_PADDING;
    v_flex()
        .gap(px(BODY_GAP))
        .pt(px(vertical - KIT_TITLE_GAP))
        .pb(px(vertical))
        .px(px(sides))
        .child(content)
}

/// The footer part of a dialog's own view, drawn as its last child (R-19):
/// a top border, the hint on the left, the buttons right-aligned.
pub(crate) fn footer(footer: Footer, cx: &App) -> Div {
    let colors = chrome_colors(cx);
    let (vertical, sides) = FOOTER_PADDING;
    footer
        .row(&colors)
        .py(px(vertical))
        .px(px(sides))
        .border_t_1()
        .border_color(colors.border)
}

/// The header element for a Kit `Sheet`'s title slot. The Kit pads its
/// title row and draws the close button beside it, so the divider stops
/// short of the edges and of the close button (K-5).
pub(crate) fn sheet_header(header: Header, cx: &App) -> impl IntoElement {
    let colors = chrome_colors(cx);
    let (top, _, bottom) = HEADER_PADDING;
    header
        .content(&colors)
        .flex_1()
        .min_w_0()
        .pt(px(top - KIT_SHEET_TITLE_PADDING))
        .pb(px(bottom))
        .border_b_1()
        .border_color(colors.border)
}

/// A right-aligned footer element for a Kit `Sheet`'s footer slot. The Kit
/// pads its footer row, so the divider stops short of the edges (K-5).
pub(crate) fn sheet_footer(footer: Footer, cx: &App) -> impl IntoElement {
    let colors = chrome_colors(cx);
    let (vertical, _) = FOOTER_PADDING;
    footer
        .row(&colors)
        .pt(px(vertical))
        .border_t_1()
        .border_color(colors.border)
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "frame_tests.rs"]
pub(in crate::desktop::view) mod tests;
