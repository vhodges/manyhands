//! The page and the item's scroll region (R-47; design, Layout and Page).
//!
//! The region sits below the tab strip and the header, which stay in
//! place, and holds the page, then the discussion, then the workspace's
//! details of the selected repository (owner, 2026-10-27). The page is white in
//! both themes, its column at most 760 px wide and centred, and it is at
//! least as tall as the region was in the last frame, so the white reaches
//! the bottom and the discussion starts below the fold. For now it holds
//! D1's content; Task 7 puts the read editor in it.
//!
//! The region shows the Kit's vertical scrollbar on its own handle, thumb
//! in `page-border-strong` at rest, hovered and dragged, laid over the
//! region as the Kit's own scrollables lay it.

use gpui_kit::{
    AnyElement, App, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, scroll::Scrollbar, v_flex},
    div, px,
};

use super::tokens::item_colors;
use crate::desktop::view::scroll::ContentScroll;

/// The page's column at its widest.
const COLUMN: f32 = 760.;
/// The column's padding: top, sides and bottom.
const PADDING: (f32, f32, f32) = (36., 48., 40.);
/// Body text on the page.
const BODY_SIZE: f32 = 15.;

/// The item's page.
pub(crate) struct Page {
    /// Back to top focuses the page. It is no tab stop of its own: Tab
    /// goes on to what the page holds.
    focus: FocusHandle,
}

impl Page {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle(),
        }
    }

    pub(super) fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// The page, holding `content`, in the region `scroll` scrolls.
    pub(super) fn render<C: IntoElement>(
        &self,
        content: C,
        scroll: &ContentScroll,
        window: &Window,
        cx: &App,
    ) -> impl IntoElement + use<C> {
        let colors = item_colors(cx);
        let height = scroll.height();
        let ring = self.focus.is_focused(window).then(|| {
            div()
                .absolute()
                .inset_0()
                .border_2()
                .border_color(cx.theme().ring)
        });
        let (top, sides, bottom) = PADDING;
        v_flex()
            .id("item-page")
            .test_support()
            .track_focus(&self.focus)
            .relative()
            .flex_none()
            .w_full()
            .min_h(height)
            .items_center()
            .bg(colors.page)
            .text_color(colors.page_text)
            .text_size(px(BODY_SIZE))
            .child(
                div()
                    .id("item-page-column")
                    .test_support()
                    .w_full()
                    .max_w(px(COLUMN))
                    .pt(px(top))
                    .px(px(sides))
                    .pb(px(bottom))
                    .child(content),
            )
            .child(scroll.follow_height(height))
            .children(ring)
    }
}

/// The item's scroll region, holding `page`, then `discussion`, then
/// `end`, the selected repository's details, with the Kit's scrollbar
/// over it.
pub(super) fn region<P: IntoElement, D: IntoElement>(
    scroll: &ContentScroll,
    page: P,
    discussion: Option<D>,
    end: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement + use<P, D> {
    // One light thumb at rest, under the pointer and while dragged, so it
    // stays light against the page in either theme.
    let thumb = item_colors(cx).page_border_strong;
    let end = end.map(|end| {
        div()
            .id("item-region-end")
            .w_full()
            .pt_3()
            .pb_3()
            .child(end)
    });
    div()
        .id("item-region")
        .relative()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .w_full()
        .child(
            v_flex()
                .id("item-scroll")
                .test_support()
                .size_full()
                .overflow_y_scroll()
                .track_scroll(scroll.handle())
                .child(page)
                .children(discussion)
                .children(end),
        )
        .child(
            div()
                .id("item-scrollbar")
                .test_support()
                .absolute()
                .inset_0()
                .child(
                    Scrollbar::vertical(scroll.handle())
                        .id("item-scrollbar-bar")
                        .viewport_from_layout()
                        .styles(|styles| {
                            styles
                                .thumb(|style| style.bg(thumb))
                                .thumb_hover(|style| style.bg(thumb))
                                .thumb_active(|style| style.bg(thumb))
                        }),
                ),
        )
}
