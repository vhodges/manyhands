//! D1's identity details, provenance, named fields, problems and unknown
//! metadata. An item tab shows them in its details sheet (`sheet`); a
//! file read by its path has no sheet, so its page keeps them, and names
//! the file by its kind and ID, or as nonconforming when it has no ID. The
//! title is the header's, and a context problem the recovery banner's.

use gpui_kit::{
    AnyElement, App, ClipboardItem, Context, ElementId, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt,
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px,
};
use manyhands::repository::{ClosureState, ItemDto, ItemDtoKind, ProblemDto};

use super::{COPY_ID, COPY_SLUG, ItemPane, NONCONFORMING_FILE, banners};
use crate::desktop::{
    model::{
        item::{
            CONTEXT_PROBLEMS, NOT_REPRESENTABLE, context_kind_label, context_problems,
            metadata_value, provenance_label,
        },
        navigation::OpenTarget,
        relationships::RELATIONSHIP_PROBLEMS,
    },
    view::{
        scroll::ContentScroll,
        tickets::{LIFECYCLE_CLOSED, LIFECYCLE_OPEN},
    },
};

/// Shared with relationship rows through the parent view's stable path.
pub(in crate::desktop::view) fn text(
    id: impl Into<ElementId>,
    value: impl Into<String>,
) -> impl IntoElement + Styled + InteractiveElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

/// A named value: its name, muted, in a 120 px column, then the value.
pub(super) fn field(
    id: &'static str,
    name: &'static str,
    value: String,
    cx: &App,
) -> impl IntoElement + use<> {
    h_flex()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .w(px(120.))
                .child(name),
        )
        .child(text(id, value).text_sm())
}

pub(super) fn kind_label(kind: ItemDtoKind) -> &'static str {
    match kind {
        ItemDtoKind::Document => "Document",
        ItemDtoKind::Ticket => "Ticket",
    }
}

/// What D1's Copy controls copy.
#[derive(Clone, Copy)]
pub(super) enum Copied {
    ShortCode,
    Id,
}

impl ItemPane {
    /// The identity of a file read by its path: its kind once read, or that
    /// it is nonconforming, which a file without an ID is; its short code
    /// and ID, each with Copy; and its path.
    pub(super) fn identity(
        &self,
        item: Option<&ItemDto>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let line = item.map(|item| {
            if banners::nonconforming(item) {
                text("item-nonconforming", NONCONFORMING_FILE)
                    .text_xs()
                    .text_color(muted)
            } else {
                text("item-kind", kind_label(item.kind))
                    .text_xs()
                    .text_color(muted)
            }
        });
        let mut codes = h_flex().gap_2();
        if let Some(slug) = item.and_then(|item| item.slug.clone()) {
            codes = codes
                .child(text("item-slug", slug.clone()).text_sm())
                .child(self.copy(Copied::ShortCode, slug, None));
        }
        if let Some(id) = item.and_then(|item| item.id.clone()) {
            codes = codes
                .child(text("item-id", id.clone()).text_sm().text_color(muted))
                .child(self.copy(Copied::Id, id, None));
        }
        let path = match self.model.target() {
            OpenTarget::Inspect { path, .. } => Some(path.clone()),
            OpenTarget::Item { .. } => item.map(|item| item.path.clone()),
        };
        v_flex()
            .gap_1()
            .child(h_flex().gap_2().children(line))
            .child(codes)
            .children(path.map(|path| text("item-path", path).text_xs().text_color(muted)))
    }

    /// D1's Copy control for `value`: a wrapper that holds the focus of the
    /// Kit's button inside it, revealed in `reveal` when it takes focus.
    pub(super) fn copy(
        &self,
        copied: Copied,
        value: String,
        reveal: Option<&ContentScroll>,
    ) -> impl IntoElement + use<> {
        let (id, button, label, focus) = match copied {
            Copied::ShortCode => ("copy-slug", "copy-slug-button", COPY_SLUG, 0),
            Copied::Id => ("copy-id", "copy-id-button", COPY_ID, 1),
        };
        let focus = &self.copy_focus[focus];
        div()
            .id(id)
            .test_support()
            .aria_label(label)
            .track_focus(focus)
            .relative()
            .child(
                Button::new(button)
                    .ghost()
                    .label(label)
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(value.clone()))
                    }),
            )
            .children(reveal.map(|scroll| scroll.marker_within(focus)))
    }

    /// Context problems replace provenance: the recovery banner says what
    /// needs repair, and no copy is claimed.
    pub(super) fn provenance(&self, item: &ItemDto, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        if !context_problems(item).is_empty() {
            return div().into_any_element();
        }
        let context = &item.context;
        v_flex()
            .id("provenance-section")
            .gap_1()
            .child(
                text("provenance", provenance_label(context))
                    .text_xs()
                    .text_color(muted),
            )
            .child(field(
                "provenance-kind",
                "Context",
                context_kind_label(context.kind).to_owned(),
                cx,
            ))
            .children(
                context
                    .branch
                    .clone()
                    .map(|branch| field("provenance-branch", "Branch", branch, cx)),
            )
            .child(field(
                "provenance-worktree",
                "Worktree",
                context.worktree.clone(),
                cx,
            ))
            .children(
                context
                    .head_oid
                    .clone()
                    .map(|head| field("provenance-head", "Head commit", head, cx)),
            )
            .into_any_element()
    }

    /// The lifecycle, as D1 named it: Open or Closed.
    pub(super) fn lifecycle(item: &ItemDto) -> Option<&'static str> {
        item.closure.as_ref().map(|closure| match closure.state {
            ClosureState::Closed => LIFECYCLE_CLOSED,
            _ => LIFECYCLE_OPEN,
        })
    }

    pub(super) fn fields(
        &self,
        item: &ItemDto,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut fields = v_flex().gap_1();
        if let (Some(lifecycle), Some(closure)) = (Self::lifecycle(item), &item.closure) {
            fields = fields.child(field(
                "item-lifecycle",
                "Lifecycle",
                lifecycle.to_owned(),
                cx,
            ));
            if let Some(at) = closure.closed_at.clone() {
                fields = fields.child(field("item-closed-at", "Closed at", at, cx));
            }
            if let Some(by) = closure.closed_by.clone() {
                fields = fields.child(field("item-closed-by", "Closed by", by, cx));
            }
        }
        for (id, name, value) in [
            ("item-status", "Status", &item.status),
            ("item-type", "Type", &item.ticket_type),
            ("item-project", "Project", &item.project),
            ("item-team", "Team", &item.team),
            ("item-changed", "Changed", &item.changed_at),
        ] {
            if let Some(value) = value.clone() {
                fields = fields.child(field(id, name, value, cx));
            }
        }
        fields
    }

    /// The item's own problems: context and relationship problems are
    /// displayed by their own sections.
    pub(super) fn own_problems<'a>(&self, item: &'a ItemDto) -> Vec<&'a ProblemDto> {
        item.problems
            .iter()
            .filter(|problem| !CONTEXT_PROBLEMS.contains(&problem.code))
            .filter(|problem| {
                self.model.relationships().is_none()
                    || !RELATIONSHIP_PROBLEMS.contains(&problem.code)
            })
            .collect()
    }

    pub(super) fn problems(
        &self,
        item: &ItemDto,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        v_flex().gap_1().children(
            self.own_problems(item)
                .into_iter()
                .enumerate()
                .map(|(ix, problem)| {
                    v_flex()
                        .child(text(("item-problem", ix), problem.code.as_str()).text_sm())
                        .child(
                            text(("item-problem-guidance", ix), problem.guidance())
                                .text_xs()
                                .text_color(muted),
                        )
                        .children(problem.path.clone().map(|path| {
                            text(("item-problem-path", ix), path)
                                .text_xs()
                                .text_color(muted)
                        }))
                })
                .collect::<Vec<_>>(),
        )
    }

    /// Each unknown key but those in `skip`, in the library's order, with
    /// its value as the library's JSON or text. None is editable.
    pub(super) fn metadata(
        &self,
        item: &ItemDto,
        skip: &[&str],
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (muted, mono) = (theme.muted_foreground, theme.mono_font_family.clone());
        v_flex().gap_1().children(
            item.unknown_metadata
                .iter()
                .filter(|(key, _)| !skip.contains(&key.as_str()))
                .enumerate()
                .map(|(ix, (key, value))| {
                    let value = metadata_value(value);
                    let shown = value
                        .clone()
                        .unwrap_or_else(|| NOT_REPRESENTABLE.to_owned());
                    let value = text(("meta-value", ix), shown).text_sm();
                    h_flex()
                        .gap_2()
                        .child(
                            text(("meta-key", ix), key.clone())
                                .text_xs()
                                .font_family(mono.clone())
                                .text_color(muted)
                                .w(px(120.)),
                        )
                        .child(value)
                })
                .collect::<Vec<_>>(),
        )
    }
}
