//! Read-only comments and the fixed unavailable actions of later releases.

use gpui_kit::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    TestSupportExt,
    component::{ActiveTheme, Disableable, button::Button, h_flex, v_flex},
    div, px,
};
use manyhands::repository::CommentListDto;

use super::{COMMENT_INDENT, ItemPane, text};
use crate::desktop::model::item::{LATER, LATER_NOTE, comment_author};

impl ItemPane {
    pub(super) fn discussion(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
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
                .gap_2()
                .child(div().text_sm().child("Discussion"))
                .child(status)
                .children(rows),
        )
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
