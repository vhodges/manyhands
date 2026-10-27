//! The discussion (design, Discussion; `item_view/04-comments.md`). It
//! follows the page in the item's scroll region, on the `discussion`
//! ground, in a 664 px column.
//!
//! - **Header row.** `message-square`, the "Discussion" heading, "{n}
//!   comments · oldest first", and Back to top, which returns the region
//!   to the page.
//! - **Threads.** One white card per root comment, in the library's order
//!   (`model::discussion`). Every reply sits at the one reply indent, and a
//!   reply deeper than one level says whom it answers (R-53).
//! - **Comment rows.** An avatar coloured from the author (R-56), the name
//!   without its email, the time in local time, and the body in a read
//!   editor (R-63). A file that is no comment shows its path and reason.
//! - **The end.** "No comments yet" when there are none, then Comment,
//!   not available yet.
//!
//! Each body's `ReadOnly` is made when the comment list lands and kept by
//! the comment's ID across reloads: a changed body is loaded into it in
//! place, an unchanged one is left alone (R-79). Each keeps its caret in
//! view (R-67), as the page's read editor does: after a caret move while
//! it has focus, and when it takes focus, which each frame checks.
//!
//! **The jump** (`jump`). The header's Comments {n} and `secondary-shift-c`
//! bring the discussion to the region's top and focus its heading, which
//! is no tab stop. The floating "{n} comments" button over the region does
//! the same; it shows only while there are comments and the discussion's
//! header is below the region (fact 80). The cards and rows are `rows`'s.

mod jump;
mod rows;

use std::collections::{HashMap, HashSet};

use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, Entity, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    TestSupportExt, Window,
    component::{ActiveTheme, Icon, h_flex, v_flex},
    div, px,
};
use manyhands::repository::CommentListDto;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use super::{
    BACK_TO_TOP, ItemPane,
    header::{NoteAt, Weight, later_action},
    text,
    tokens::item_colors,
};
use crate::desktop::{
    assets::icon_path,
    model::{
        discussion::{comment_time, threads},
        item::COMMENT,
        list_state::ListState,
    },
    view::{
        confirm::local_offset,
        editor::{CaretInView, read_only::ReadOnly},
    },
};

pub(crate) const DISCUSSION: &str = "Discussion";
pub(crate) const NO_COMMENTS: &str = "No comments yet";
pub(crate) const OLDEST_FIRST: &str = "oldest first";
pub(crate) const REPLY_TO: &str = "Reply to";
pub(crate) const COMMENTS: &str = "Comments";

/// The discussion's column at its widest, under the page's text.
const COLUMN: f32 = 664.;
/// The section's padding: top, sides and bottom.
const PADDING: (f32, f32, f32) = (28., 48., 48.);
/// Between the header row, the cards and Comment.
const GAP: f32 = 18.;
const HEADING_SIZE: f32 = 16.;
const SMALL_SIZE: f32 = 12.;
const LINK_SIZE: f32 = 12.5;
const ICON_SIZE: f32 = 13.;
/// How much further in than a root a reply's row starts: to 50 px.
pub(super) const REPLY_INDENT: f32 = 36.;
/// A comment's body, read at 14 px.
const BODY_SIZE: f32 = 14.;

/// "1 comment" or "{n} comments".
pub(crate) fn comments_phrase(n: usize) -> String {
    match n {
        1 => "1 comment".to_owned(),
        n => format!("{n} comments"),
    }
}

/// How many comments `list` holds: its entries with an ID. A file that is
/// no comment is not counted.
fn count(list: &CommentListDto) -> usize {
    list.items
        .iter()
        .filter(|comment| comment.id.is_some())
        .count()
}

/// The local offset at `at`, an RFC 3339 time, found as `confirm.rs` finds
/// it (fact 80); none where it is not read, as on non-Unix targets.
pub(super) fn offset_at(at: &str) -> Option<UtcOffset> {
    OffsetDateTime::parse(at, &Rfc3339)
        .ok()
        .and_then(|at| local_offset(at.unix_timestamp()))
}

/// A comment's time `at`, as its row shows it: at `offset`; with none, in
/// UTC and said so, as `confirm.rs` says it. A time that cannot be read is
/// shown as written.
pub(super) fn shown_time(at: &str, now: OffsetDateTime, offset: Option<UtcOffset>) -> String {
    let shown = comment_time(at, now, offset.unwrap_or(UtcOffset::UTC));
    match offset {
        None if OffsetDateTime::parse(at, &Rfc3339).is_ok() => format!("{shown} UTC"),
        _ => shown,
    }
}

/// The discussion's own state in an item pane.
pub(super) struct Discussion {
    /// Each comment's body in its read editor (R-63), by the comment's ID.
    bodies: HashMap<String, Body>,
    /// The "Discussion" heading, which the jump focuses. No tab stop.
    heading: FocusHandle,
    /// The header's Comments {n}, a tab stop.
    button: FocusHandle,
}

/// A comment's body in its read editor. It derives nothing: it holds the
/// body.
struct Body {
    read: Entity<ReadOnly>,
    /// The body as the library read it, last loaded into `read`.
    text: String,
    /// The read editor's caret reveal (R-67).
    reveal: CaretInView,
}

impl Discussion {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            bodies: HashMap::new(),
            heading: cx.focus_handle(),
            button: cx.focus_handle().tab_stop(true),
        }
    }

    /// The read view of the comment `id`'s body.
    #[cfg(test)]
    pub(super) fn body(&self, id: &str) -> Option<&Entity<ReadOnly>> {
        self.bodies.get(id).map(|body| &body.read)
    }
}

impl ItemPane {
    /// As each frame begins: one read editor per comment body, made when
    /// the list lands and kept by ID across reloads (R-63). A changed body
    /// is loaded in place, keeping the cursor (R-19, R-79); an unchanged
    /// one is left alone; a comment that is gone takes its editor with it.
    /// While a reload is out, the editors stay. Then R-67's focus hook: an
    /// editor that took focus since the last frame reveals its caret once
    /// this frame is drawn.
    pub(super) fn sync_discussion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(list) = self.model.comments().and_then(ListState::rows) else {
            return;
        };
        let listed: HashSet<&str> = list
            .items
            .iter()
            .filter(|comment| comment.body.is_some())
            .filter_map(|comment| comment.id.as_deref())
            .collect();
        let bodies = &mut self.discussion.bodies;
        bodies.retain(|id, _| listed.contains(id.as_str()));
        for comment in &list.items {
            let (Some(id), Some(text)) = (&comment.id, &comment.body) else {
                continue;
            };
            if let Some(body) = bodies.get_mut(id) {
                if body.text != *text {
                    body.read.update(cx, |read, cx| read.reload(text, cx));
                    body.text.clone_from(text);
                }
                continue;
            }
            let read = cx.new(|cx| ReadOnly::new(text, window, cx));
            let editor = read.read(cx).editor().clone();
            let reveal = CaretInView::new(&editor, &self.scroll, window, cx);
            bodies.insert(
                id.clone(),
                Body {
                    read,
                    text: text.clone(),
                    reveal,
                },
            );
        }
        for body in bodies.values_mut() {
            body.reveal.note_focus(window, cx);
        }
    }

    /// The discussion, ending with Comment, the `later`th later action.
    pub(super) fn discussion_section(
        &self,
        later: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let comments = self.model.comments()?;
        let colors = item_colors(cx);
        let status = self.status_in("comments", comments, colors.page_muted, cx);
        let list = comments.rows();
        let header = self.discussion_header(list.map(count), window, cx);
        let now = OffsetDateTime::now_utc();
        let cards: Vec<AnyElement> = list
            .map(|list| {
                threads(list)
                    .iter()
                    .enumerate()
                    .map(|(ix, thread)| self.thread(list, ix, thread, now, cx))
                    .collect()
            })
            .unwrap_or_default();
        let empty = list.filter(|list| list.items.is_empty()).map(|_| {
            text("discussion-empty", NO_COMMENTS)
                .text_size(px(BODY_SIZE))
                .text_color(colors.page_muted)
        });
        let (top, sides, bottom) = PADDING;
        Some(
            v_flex()
                .id("discussion")
                .test_support()
                .flex_none()
                .w_full()
                .items_center()
                .pt(px(top))
                .px(px(sides))
                .pb(px(bottom))
                .bg(colors.discussion)
                .border_t_1()
                .border_color(colors.page_border)
                .text_color(colors.page_text)
                .child(
                    v_flex()
                        .w_full()
                        .max_w(px(COLUMN))
                        .gap(px(GAP))
                        .child(header)
                        .child(status)
                        .children(cards)
                        .children(empty)
                        .child(later_action(
                            later,
                            COMMENT,
                            None,
                            Weight::Plain,
                            NoteAt::BelowOnPage,
                            cx,
                        )),
                ),
        )
    }

    /// `message-square`, the heading, the count once the list is read, and
    /// Back to top on the right.
    fn discussion_header(
        &self,
        count: Option<usize>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = item_colors(cx);
        let ring = self.discussion.heading.is_focused(window).then(|| ring(cx));
        h_flex()
            .gap(px(10.))
            .items_center()
            .child(
                Icon::default()
                    .path(icon_path("message-square"))
                    .w(px(HEADING_SIZE))
                    .h(px(HEADING_SIZE))
                    .text_color(colors.page_text_secondary),
            )
            .child(
                div()
                    .id("discussion-heading")
                    .test_support()
                    .role(Role::Heading)
                    .aria_level(2)
                    .aria_label(DISCUSSION)
                    .track_focus(&self.discussion.heading)
                    .relative()
                    .text_size(px(HEADING_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(DISCUSSION)
                    .children(ring),
            )
            .children(count.map(|n| {
                text(
                    "discussion-count",
                    format!("{} · {OLDEST_FIRST}", comments_phrase(n)),
                )
                .text_size(px(SMALL_SIZE))
                .text_color(colors.page_muted)
            }))
            .child(div().flex_1())
            .child(self.back_to_top_control(window, cx))
    }

    /// Back to top: a tab stop in `page-teal`, pressed by click, Enter or
    /// Space, ringed while focused and revealed when focus reaches it
    /// below the fold.
    fn back_to_top_control(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let teal = item_colors(cx).page_teal;
        let ring = self.back_to_top.is_focused(window).then(|| ring(cx));
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
                    .gap(px(5.))
                    .items_center()
                    .text_size(px(LINK_SIZE))
                    .text_color(teal)
                    .child(
                        Icon::default()
                            .path(icon_path("arrow-up"))
                            .w(px(ICON_SIZE))
                            .h(px(ICON_SIZE)),
                    )
                    .child(BACK_TO_TOP),
            )
            .children(ring)
            .child(self.scroll.marker(&self.back_to_top))
    }
}

/// A focus ring over a `relative()` control.
fn ring(cx: &App) -> impl IntoElement + use<> {
    div()
        .absolute()
        .inset_0()
        .rounded_sm()
        .border_2()
        .border_color(cx.theme().ring)
}
