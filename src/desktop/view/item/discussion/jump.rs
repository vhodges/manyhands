//! The jump to the discussion: the header's Comments {n}, the floating
//! "{n} comments" button over the item's region, and `secondary-shift-c`,
//! which the workspace sends to the shown item tab.

use gpui_kit::{
    AnyElement, BoxShadow, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    Role, ScrollHandle, StatefulInteractiveElement, Styled, TestSupportExt, Window, canvas,
    component::{ActiveTheme, Icon, h_flex, tooltip::Tooltip},
    div, hsla, point, px,
};
use manyhands::repository::ItemDtoKind;

use super::{COMMENTS, ICON_SIZE, LINK_SIZE, SMALL_SIZE, comments_phrase, count, ring};
use crate::desktop::{
    assets::icon_path,
    keys::{CONTENT_CONTEXT, JumpToDiscussion},
    view::item::{ItemPane, page::DISCUSSION_CHILD, tokens::item_colors},
};

/// The floating button's place: in from the region's right and bottom.
const JUMP_INSET: f32 = 16.;
/// The discussion's header row is inside the region once this much of the
/// discussion is: its top padding and the row.
const HEADER_IN_VIEW: f32 = 60.;

/// Whether the discussion's header row is inside the region: the
/// discussion's laid-out top (`bounds_for_item`), moved by the region's
/// offset, is far enough above the region's bottom (fact 80).
fn header_in_view(handle: &ScrollHandle) -> bool {
    let Some(discussion) = handle.bounds_for_item(DISCUSSION_CHILD) else {
        return false;
    };
    discussion.top() + handle.offset().y + px(HEADER_IN_VIEW) <= handle.bounds().bottom()
}

impl ItemPane {
    /// The jump (Comments {n}, `secondary-shift-c`, the floating button):
    /// the discussion to the top of the region with
    /// `scroll_to_top_of_item`, as the next frame lays it out (fact 80),
    /// and its heading focused.
    pub(in crate::desktop::view) fn jump_to_discussion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.model.comments().is_none() {
            return;
        }
        self.scroll.handle().scroll_to_top_of_item(DISCUSSION_CHILD);
        window.focus(&self.discussion.heading, cx);
        cx.notify();
    }

    /// The header's Comments {n}, a button that jumps to the discussion,
    /// with the count once the list is read; none for a tab without one.
    pub(in crate::desktop::view::item) fn comments_button(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let count = self.model.comments()?.rows().map(count);
        let theme = cx.theme();
        let label = match count {
            Some(n) => format!("{COMMENTS} {n}"),
            None => COMMENTS.to_owned(),
        };
        let noun = match self.shown_kind(None) {
            Some(ItemDtoKind::Document) => "document",
            _ => "ticket",
        };
        let tooltip = format!("Jump to the discussion below the {noun}");
        let focus = &self.discussion.button;
        let ring = focus.is_focused(window).then(|| ring(cx));
        let badge = count.map(|n| {
            div()
                .px(px(6.))
                .rounded(px(8.))
                .bg(theme.muted)
                .text_color(theme.muted_foreground)
                .text_size(px(11.))
                .child(n.to_string())
        });
        let hover = theme.info;
        Some(
            h_flex()
                .id("item-comments")
                .test_support()
                .role(Role::Button)
                .aria_label(label)
                .track_focus(focus)
                .relative()
                .flex_none()
                .gap(px(6.))
                .items_center()
                .px(px(10.))
                .py(px(5.))
                .rounded(px(5.))
                .border_1()
                .border_color(theme.input)
                .bg(theme.secondary)
                .text_color(theme.foreground)
                .text_size(px(SMALL_SIZE))
                .whitespace_nowrap()
                .cursor_pointer()
                .hover(move |style| style.border_color(hover))
                .tooltip(move |window, cx| {
                    Tooltip::new(tooltip.clone())
                        .action(&JumpToDiscussion, Some(CONTENT_CONTEXT))
                        .build(window, cx)
                })
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.jump_to_discussion(window, cx)
                }))
                .child(
                    Icon::default()
                        .path(icon_path("message-square"))
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE)),
                )
                .child(COMMENTS)
                .children(badge)
                .children(ring)
                .into_any_element(),
        )
    }

    /// What lies over the item's region: the floating "{n} comments"
    /// button at its bottom right, while there are comments and the
    /// discussion's header is below the region, and a check that draws
    /// again when the region moved as it was laid out, as a jump moves it.
    pub(in crate::desktop::view::item) fn over_region(
        &self,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let Some(comments) = self.model.comments() else {
            return Vec::new();
        };
        let n = comments.rows().map(count).unwrap_or(0);
        let handle = self.scroll.handle().clone();
        let shown = header_in_view(&handle);
        let pane = cx.entity().downgrade();
        let follow = canvas(
            move |_, _, cx| {
                if header_in_view(&handle) != shown {
                    let pane = pane.clone();
                    cx.defer(move |cx| {
                        let _ = pane.update(cx, |_, cx| cx.notify());
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_0()
        .into_any_element();
        let colors = item_colors(cx);
        let button = (n > 0 && !shown).then(|| {
            h_flex()
                .id("discussion-jump")
                .test_support()
                .role(Role::Button)
                .aria_label(comments_phrase(n))
                .occlude()
                .absolute()
                .right(px(JUMP_INSET))
                .bottom(px(JUMP_INSET))
                .gap(px(6.))
                .items_center()
                .px(px(12.))
                .py(px(7.))
                .rounded(px(18.))
                .border_1()
                .border_color(colors.page_border_strong)
                .bg(colors.page)
                .text_color(colors.page_text)
                .text_size(px(LINK_SIZE))
                .whitespace_nowrap()
                .cursor_pointer()
                .shadow(vec![BoxShadow {
                    color: hsla(0., 0., 0., 0.14),
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(14.),
                    spread_radius: px(0.),
                    inset: false,
                }])
                .tooltip(|window, cx| {
                    Tooltip::new("Jump to comments")
                        .action(&JumpToDiscussion, Some(CONTENT_CONTEXT))
                        .build(window, cx)
                })
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.jump_to_discussion(window, cx)
                }))
                .child(
                    Icon::default()
                        .path(icon_path("message-square"))
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE))
                        .text_color(colors.page_teal),
                )
                .child(comments_phrase(n))
                .child(
                    Icon::default()
                        .path(icon_path("arrow-down"))
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE))
                        .text_color(colors.page_text_secondary),
                )
                .into_any_element()
        });
        std::iter::once(follow).chain(button).collect()
    }
}
