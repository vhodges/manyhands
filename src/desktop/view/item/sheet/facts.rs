//! The Facts (design, Details sheet): what is never edited here. For a
//! ticket: D1's kind, its short code and ID, each with Copy, D1's
//! lifecycle and the closure, when it was updated, its file, and D1's
//! provenance (the context kind, branch and worktree). For a document:
//! its kind, ID with Copy, file name, update and provenance. The design's
//! "Main version" is not built: no read reports it.

use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, h_flex, v_flex},
    div, px,
};
use manyhands::repository::{ClosureState, ItemDto, ItemDtoKind};

use super::{
    super::{
        details::{Copied, field, kind_label},
        header::folder_and_file,
    },
    FACTS, LABEL_SIZE, NOT_CLOSED,
};
use crate::desktop::view::item::{ChipField, ItemPane, text};

/// A closed ticket's closure: who closed it and when, as far as the file
/// says.
pub(crate) fn closed_sentence(by: Option<&str>, at: Option<&str>) -> String {
    let mut sentence = "Closed".to_owned();
    if let Some(by) = by {
        sentence.push_str(&format!(" by {by}"));
    }
    if let Some(at) = at {
        sentence.push_str(&format!(" on {at}"));
    }
    sentence
}

/// A fact's name, in the 120 px column `field` uses, then `content`,
/// which takes the rest of the row.
fn named(name: &'static str, content: impl IntoElement, cx: &App) -> impl IntoElement {
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .w(px(120.))
                .flex_none()
                .child(name),
        )
        .child(div().flex_1().min_w_0().child(content))
}

/// A code and its Copy, which wraps below a code too long to share the
/// row with it, as a ULID in the 440 px sheet is.
fn code(value: impl IntoElement, copy: impl IntoElement) -> impl IntoElement {
    h_flex()
        .flex_wrap()
        .gap_x_2()
        .items_center()
        .child(value)
        .child(copy)
}

/// "Not closed. Use Close ticket… to close it.", or "Closed by … on …",
/// each part D1's own element.
fn closure(item: &ItemDto, cx: &App) -> Option<AnyElement> {
    let closure = item.closure.as_ref()?;
    let sentence = if closure.state == ClosureState::Closed {
        let (by, at) = (closure.closed_by.clone(), closure.closed_at.clone());
        h_flex()
            .id("sheet-closure")
            .test_support()
            .aria_label(closed_sentence(by.as_deref(), at.as_deref()))
            .flex_wrap()
            .gap_1()
            .text_sm()
            .child("Closed")
            .children(by.map(|by| {
                h_flex()
                    .gap_1()
                    .child("by")
                    .child(text("item-closed-by", by))
            }))
            .children(at.map(|at| {
                h_flex()
                    .gap_1()
                    .child("on")
                    .child(text("item-closed-at", at))
            }))
            .into_any_element()
    } else {
        text("sheet-closure", NOT_CLOSED)
            .text_sm()
            .into_any_element()
    };
    Some(named("Closure", sentence, cx).into_any_element())
}

impl ItemPane {
    pub(super) fn facts(
        &self,
        item: &ItemDto,
        kind: ItemDtoKind,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let fill = cx.theme().secondary;
        let reveal = Some(&self.sheet_scroll);
        let mut facts = v_flex()
            .id("sheet-facts")
            .test_support()
            .role(Role::Group)
            .aria_label(FACTS)
            .gap_2()
            .p_3()
            .rounded(px(6.))
            .bg(fill)
            .child(
                div()
                    .text_size(px(LABEL_SIZE))
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .child(FACTS),
            )
            .child(field("item-kind", "Kind", kind_label(kind).to_owned(), cx));
        if let (ItemDtoKind::Ticket, Some(slug)) = (kind, &item.slug) {
            let slug = code(
                text("item-slug", slug.clone()).text_sm(),
                self.copy(Copied::ShortCode, slug.clone(), reveal),
            );
            facts = facts.child(named("Short code", slug, cx));
        }
        if let Some(id) = &item.id {
            let id = code(
                text("item-id", id.clone()).text_sm(),
                self.copy(Copied::Id, id.clone(), reveal),
            );
            facts = facts.child(named("ID", id, cx));
        }
        if let (ItemDtoKind::Ticket, Some(lifecycle)) = (kind, Self::lifecycle(item)) {
            facts = facts
                .child(field(
                    "item-lifecycle",
                    "Lifecycle",
                    lifecycle.to_owned(),
                    cx,
                ))
                .children(closure(item, cx));
        }
        if let Some(changed) = &item.changed_at {
            facts = facts.child(self.updated(changed, window, cx));
        }
        facts = match kind {
            ItemDtoKind::Ticket => facts.child(field("item-path", "File", item.path.clone(), cx)),
            ItemDtoKind::Document => facts.child(field(
                "item-file-name",
                "File name",
                folder_and_file(&item.path).1,
                cx,
            )),
        };
        facts.child(self.provenance(item, cx)).into_any_element()
    }

    /// When the index saw the item change, as D1 showed it: the field the
    /// Updated chip opens the sheet on, so a tab stop.
    fn updated(&self, changed: &str, window: &Window, cx: &App) -> AnyElement {
        let focus = &self.sheet_focus[ChipField::Updated.index()];
        let ring = focus.is_focused(window).then(|| {
            div()
                .absolute()
                .inset_0()
                .rounded(px(4.))
                .border_2()
                .border_color(cx.theme().ring)
        });
        div()
            .id("sheet-changed")
            .test_support()
            .track_focus(focus)
            .relative()
            .child(field("item-changed", "Updated", changed.to_owned(), cx))
            .children(ring)
            .child(self.sheet_scroll.marker(focus))
            .into_any_element()
    }
}
