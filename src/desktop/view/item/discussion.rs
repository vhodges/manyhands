//! Read-only comments and the fixed unavailable actions of later releases.
//! The discussion's header row ends with Back to top, which returns the
//! item's scroll region to the page.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Disableable, Icon, button::Button, h_flex, v_flex},
    div, px,
};
use manyhands::repository::CommentListDto;

use super::{BACK_TO_TOP, COMMENT_INDENT, ItemPane, text};
use crate::desktop::{
    assets::icon_path,
    model::item::{LATER, LATER_NOTE, comment_author},
};

/// Back to top's `arrow-up`.
const ARROW_SIZE: f32 = 14.;

impl ItemPane {
    pub(super) fn discussion(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let comments = self.model.comments()?;
        let muted = cx.theme().muted_foreground;
        let status = self.status("comments", comments, cx);
        let rows = comments
            .rows()
            .map(|list: &CommentListDto| {
                list.items
                    .iter()
                    .enumerate()
                    .map(|(ix, comment)| {
                        let mut entry = v_flex().id(("comment", ix)).test_support().gap_1().child(
                            h_flex()
                                .gap_2()
                                .child(
                                    text(("comment-author", ix), comment_author(comment)).text_sm(),
                                )
                                .children(comment.created_at.clone().map(|at| {
                                    text(("comment-time", ix), at).text_xs().text_color(muted)
                                })),
                        );
                        if let Some(body) = comment.body.clone() {
                            entry = entry.child(text(("comment-body", ix), body).text_sm());
                        }
                        if comment.id.is_none() {
                            entry = entry
                                .child(text(("comment-path", ix), comment.path.clone()).text_xs())
                                .child(
                                    text(
                                        ("comment-problem", ix),
                                        comment
                                            .problems
                                            .iter()
                                            .map(|problem| problem.code.as_str())
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                    )
                                    .text_xs(),
                                );
                        }
                        div()
                            .pl(px(COMMENT_INDENT * comment.depth as f32))
                            .child(entry)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Some(
            v_flex()
                .id("discussion")
                .test_support()
                .gap_2()
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(div().text_sm().child("Discussion"))
                        .child(self.back_to_top_control(window, cx)),
                )
                .child(status)
                .children(rows),
        )
    }

    /// Back to top: a tab stop, pressed by click, Enter or Space, ringed
    /// while focused and revealed when focus reaches it below the fold.
    /// It takes the theme's link colour until the discussion has its own
    /// background (Task 8).
    fn back_to_top_control(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let ring = self.back_to_top.is_focused(window).then(|| {
            div()
                .absolute()
                .inset_0()
                .rounded_sm()
                .border_2()
                .border_color(theme.ring)
        });
        div()
            .id("back-to-top")
            .test_support()
            .role(Role::Button)
            .aria_label(BACK_TO_TOP)
            .track_focus(&self.back_to_top)
            .relative()
            .px_1()
            .cursor_pointer()
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.back_to_top(window, cx)))
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .text_sm()
                    .text_color(theme.link)
                    .child(
                        Icon::default()
                            .path(icon_path("arrow-up"))
                            .w(px(ARROW_SIZE))
                            .h(px(ARROW_SIZE)),
                    )
                    .child(BACK_TO_TOP),
            )
            .children(ring)
            .child(self.scroll.marker(&self.back_to_top))
    }

    pub(super) fn later(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        h_flex()
            .flex_wrap()
            .gap_2()
            .children(LATER.iter().enumerate().map(|(ix, action)| {
                v_flex()
                    .child(
                        div()
                            .id(("later-action", ix))
                            .test_support()
                            .aria_label(*action)
                            .child(
                                Button::new(("later-button", ix))
                                    .label(*action)
                                    .disabled(true),
                            ),
                    )
                    .child(
                        text(("later-note", ix), LATER_NOTE)
                            .text_xs()
                            .text_color(muted),
                    )
            }))
    }
}
