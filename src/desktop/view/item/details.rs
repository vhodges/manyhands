//! Identity details, provenance, named fields, problems and unknown
//! metadata, on the page until the details sheet takes them (Task 6). The
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
use manyhands::repository::{ClosureState, ItemDto, ItemDtoKind};

use super::{COPY_ID, COPY_SLUG, ItemPane, NONCONFORMING_FILE};
use crate::desktop::{
    model::{
        item::{
            CONTEXT_PROBLEMS, NOT_REPRESENTABLE, context_kind_label, context_problems,
            metadata_value, provenance_label,
        },
        navigation::OpenTarget,
        relationships::RELATIONSHIP_PROBLEMS,
    },
    view::tickets::{LIFECYCLE_CLOSED, LIFECYCLE_OPEN},
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

fn field(
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

fn kind_label(kind: ItemDtoKind) -> &'static str {
    match kind {
        ItemDtoKind::Document => "Document",
        ItemDtoKind::Ticket => "Ticket",
    }
}

impl ItemPane {
    pub(super) fn identity(
        &self,
        item: Option<&ItemDto>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let mut header = v_flex().gap_1();
        let (kind, id, path) = match self.model.target() {
            OpenTarget::Item { kind, id, .. } => (
                Some(kind_label(*kind)),
                Some(id.to_string()),
                item.map(|item| item.path.clone()),
            ),
            OpenTarget::Inspect { path, .. } => (None, None, Some(path.clone())),
        };
        let mut line = h_flex().gap_2();
        line = match kind {
            Some(kind) => line.child(text("item-kind", kind).text_xs().text_color(muted)),
            None => line.child(
                text("item-nonconforming", NONCONFORMING_FILE)
                    .text_xs()
                    .text_color(muted),
            ),
        };
        header = header.child(line);
        let slug = item.and_then(|item| item.slug.clone());
        let mut codes = h_flex().gap_2();
        if let Some(slug) = slug.clone() {
            codes = codes
                .child(text("item-slug", slug.clone()).text_sm())
                .child(
                    div()
                        .id("copy-slug")
                        .test_support()
                        .aria_label(COPY_SLUG)
                        .track_focus(&self.copy_focus[0])
                        .child(
                            Button::new("copy-slug-button")
                                .ghost()
                                .label(COPY_SLUG)
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(slug.clone()))
                                }),
                        ),
                );
        }
        if let Some(id) = id {
            let copied = id.clone();
            codes = codes
                .child(text("item-id", id).text_sm().text_color(muted))
                .child(
                    div()
                        .id("copy-id")
                        .test_support()
                        .aria_label(COPY_ID)
                        .track_focus(&self.copy_focus[1])
                        .child(
                            Button::new("copy-id-button")
                                .ghost()
                                .label(COPY_ID)
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()))
                                }),
                        ),
                );
        }
        header = header.child(codes);
        if let Some(path) = path {
            header = header.child(text("item-path", path).text_xs().text_color(muted));
        }
        header
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

    pub(super) fn fields(
        &self,
        item: &ItemDto,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut fields = v_flex().gap_1();
        if let Some(closure) = &item.closure {
            let lifecycle = match closure.state {
                ClosureState::Closed => LIFECYCLE_CLOSED,
                _ => LIFECYCLE_OPEN,
            };
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

    /// Context and relationship problems are displayed by their own sections.
    pub(super) fn problems(
        &self,
        item: &ItemDto,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        v_flex().gap_1().children(
            item.problems
                .iter()
                .filter(|problem| !CONTEXT_PROBLEMS.contains(&problem.code))
                .filter(|problem| {
                    self.model.relationships().is_none()
                        || !RELATIONSHIP_PROBLEMS.contains(&problem.code)
                })
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

    pub(super) fn metadata(
        &self,
        item: &ItemDto,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        v_flex().gap_1().children(
            item.unknown_metadata
                .iter()
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
                                .text_color(muted)
                                .w(px(120.)),
                        )
                        .child(value)
                })
                .collect::<Vec<_>>(),
        )
    }
}
