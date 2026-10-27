//! The discussion's cards and rows: one white card per thread, a row per
//! comment, with its avatar, name, time, whom a deeper reply answers, and
//! its body's read editor, and a row for a file that is no comment.

use gpui_kit::{
    AnyElement, App, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled,
    TestSupportExt,
    component::{ActiveTheme, Icon, h_flex, v_flex},
    div, px, white,
};
use manyhands::repository::{CommentDto, CommentListDto, ProblemDto};
use time::OffsetDateTime;

use super::{BODY_SIZE, ICON_SIZE, LINK_SIZE, REPLY_INDENT, REPLY_TO, SMALL_SIZE, offset_at};
use crate::desktop::{
    assets::icon_path,
    model::{
        discussion::{Reply, Thread, avatar_color, comment_time, display_name, initials},
        item::comment_author,
    },
    view::item::{ItemPane, text, tokens::item_colors},
};

const CARD_RADIUS: f32 = 8.;
/// A comment row's padding, vertical and horizontal.
const ROW_PADDING: (f32, f32) = (12., 14.);
const AVATAR: f32 = 26.;
const AVATAR_TEXT: f32 = 10.5;
const NAME_SIZE: f32 = 13.;

impl ItemPane {
    /// One white card: the root's row, then each reply's.
    pub(super) fn thread(
        &self,
        list: &CommentListDto,
        ix: usize,
        thread: &Thread,
        now: OffsetDateTime,
        cx: &App,
    ) -> AnyElement {
        let colors = item_colors(cx);
        v_flex()
            .id(("thread", ix))
            .test_support()
            .w_full()
            .bg(colors.page)
            .border_1()
            .border_color(colors.page_border)
            .rounded(px(CARD_RADIUS))
            .child(self.comment_row(list, thread.root, None, now, cx))
            .children(
                thread
                    .replies
                    .iter()
                    .map(|reply| self.comment_row(list, reply.ix, Some(reply), now, cx)),
            )
            .into_any_element()
    }

    /// The row of the `ix`th entry of `list`; a reply's sits at the reply
    /// indent, below a soft rule.
    fn comment_row(
        &self,
        list: &CommentListDto,
        ix: usize,
        reply: Option<&Reply>,
        now: OffsetDateTime,
        cx: &App,
    ) -> AnyElement {
        let colors = item_colors(cx);
        let comment = &list.items[ix];
        let (vertical, horizontal) = ROW_PADDING;
        let indent = if reply.is_some() { REPLY_INDENT } else { 0. };
        let mut row = div()
            .w_full()
            .py(px(vertical))
            .pr(px(horizontal))
            .pl(px(horizontal + indent));
        if reply.is_some() {
            row = row.border_t_1().border_color(colors.page_border_soft);
        }
        let content = match &comment.id {
            Some(id) => self.comment(ix, id, comment, reply, now, cx),
            None => malformed(ix, comment, cx),
        };
        row.child(content).into_any_element()
    }

    /// A comment: its avatar, then its name, time and, for a deeper reply,
    /// whom it answers, above its body's read editor. The avatar's colour
    /// comes from the whole `created_by` value (R-56), so two people who
    /// share a name, but not an address, differ; its initials are the
    /// name's, or "?" with no author.
    fn comment(
        &self,
        ix: usize,
        id: &str,
        comment: &CommentDto,
        reply: Option<&Reply>,
        now: OffsetDateTime,
        cx: &App,
    ) -> AnyElement {
        let colors = item_colors(cx);
        let name = display_name(comment.author.as_deref());
        let letters = match comment.author {
            Some(_) => initials(&name),
            None => "?".to_owned(),
        };
        let avatar = div()
            .flex()
            .flex_none()
            .size(px(AVATAR))
            .rounded_full()
            .items_center()
            .justify_center()
            .bg(avatar_color(comment_author(comment)))
            .text_color(white())
            .text_size(px(AVATAR_TEXT))
            .font_weight(FontWeight::SEMIBOLD)
            .child(letters);
        let small = |id: (&'static str, usize), value: String| {
            text(id, value)
                .text_size(px(SMALL_SIZE))
                .text_color(colors.page_muted)
        };
        let time = comment
            .created_at
            .as_deref()
            .map(|at| small(("comment-time", ix), comment_time(at, now, offset_at(at))));
        let answers = reply
            .and_then(|reply| reply.replying_to.as_deref())
            .map(|to| small(("comment-reply-to", ix), format!("{REPLY_TO} {to}")));
        let line = h_flex()
            .flex_wrap()
            .items_baseline()
            .gap(px(8.))
            .child(
                text(("comment-author", ix), name)
                    .text_size(px(NAME_SIZE))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .children(time)
            .children(answers);
        let body = self.discussion.bodies.get(id).map(|body| {
            div()
                .id(("comment-body", ix))
                .test_support()
                .w_full()
                .text_size(px(BODY_SIZE))
                .child(body.read.clone())
        });
        h_flex()
            .id(("comment", ix))
            .test_support()
            .w_full()
            .items_start()
            .gap(px(10.))
            .child(avatar)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(3.))
                    .child(line)
                    .children(body),
            )
            .into_any_element()
    }
}

/// A file among the comments that is no comment: its path and the
/// library's reason, never hidden.
fn malformed(ix: usize, comment: &CommentDto, cx: &App) -> AnyElement {
    let colors = item_colors(cx);
    let reason = comment
        .problems
        .iter()
        .map(ProblemDto::guidance)
        .collect::<Vec<_>>()
        .join(" ");
    v_flex()
        .id(("comment", ix))
        .test_support()
        .w_full()
        .gap(px(3.))
        .child(
            h_flex()
                .gap(px(6.))
                .items_center()
                .child(
                    Icon::default()
                        .path(icon_path("circle-alert"))
                        .flex_none()
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE))
                        .text_color(colors.page_amber_text),
                )
                .child(
                    text(("comment-path", ix), comment.path.clone())
                        .min_w_0()
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_size(px(SMALL_SIZE)),
                ),
        )
        .child(
            text(("comment-problem", ix), reason)
                .text_size(px(LINK_SIZE))
                .text_color(colors.page_text_secondary),
        )
        .into_any_element()
}
