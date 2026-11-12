//! A node's row in the Documents tree (Part F): the indent for its depth, a
//! chevron or a spacer, the icon, the label and the trailing cell, in one
//! 28 px line.
//!
//! The row is a `ListRow`, so the base tree draws it directly. Its name is
//! the label; its tooltip, which is also its description, is `doc_row`'s
//! lines. A folder says whether it is open through `expanded`, since the
//! tree's own wrapper is found only by index.

use gpui_kit::{
    App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    TestSupportExt, WeakEntity, base::TreeEntry, component::Selectable, div,
    prelude::FluentBuilder, px,
};

use super::{EMPTY_TRAILING, Node};
use crate::desktop::{
    model::{
        doc_rows::{DocKind, Trailing},
        ticket_view::UNSAVED_MARKER,
    },
    view::{
        rows::{ListRow, icon, named_icon},
        shell::tokens::chrome_colors,
        workspace::Workspace,
    },
};

/// The left padding of a top-level row, and what each level adds.
const INDENT: f32 = 8.;
const INDENT_STEP: f32 = 14.;
/// The gap between a Documents row's cells (prototype).
const GAP: f32 = 5.;
const CHEVRON_SIZE: f32 = 12.;
const ICON_SIZE: f32 = 14.;
const MARKER_SIZE: f32 = 12.;
const SMALL_TEXT: f32 = 11.;

/// The row for `entry`, the tree's entry number `ix`. `state` is whether it
/// is selected, whether the Documents region holds focus (so the selected
/// row draws its ring), and whether a draft of its document is held.
pub(super) fn document_row(
    ix: usize,
    entry: &TreeEntry,
    node: &Node,
    state: (bool, bool, bool),
    workspace: WeakEntity<Workspace>,
    cx: &App,
) -> ListRow {
    let (selected, focused, held) = state;
    let colors = chrome_colors(cx);
    let open = entry.is_expanded();
    let row = node.row(open);

    let chevron = match row.expanded {
        Some(open) => div().flex_none().child(icon(
            if open {
                "chevron-down"
            } else {
                "chevron-right"
            },
            CHEVRON_SIZE,
            colors.secondary,
        )),
        None => div().flex_none().w(px(CHEVRON_SIZE)),
    };
    let (icon_name, icon_color, label_color) = match row.kind {
        DocKind::Folder => (
            if row.expanded == Some(true) {
                "folder-open"
            } else {
                "folder"
            },
            colors.secondary,
            colors.text,
        ),
        DocKind::Document => ("file-text", colors.secondary, colors.text),
        DocKind::Nonconforming { .. } => ("file-exclamation-point", colors.accent, colors.accent),
        DocKind::Unreadable => ("circle-x", colors.red, colors.red),
    };
    let label = div()
        .flex_1()
        .min_w(px(0.))
        .overflow_hidden()
        .text_ellipsis()
        .whitespace_nowrap()
        .text_color(label_color)
        .child(row.label.clone());

    let trailing = match row.trailing {
        Trailing::Count(count) => Some(
            div()
                .id(("doc-count", ix))
                .test_support()
                .aria_label(count.to_string())
                .flex_none()
                .text_size(px(SMALL_TEXT))
                .text_color(colors.muted)
                .child(count.to_string())
                .into_any_element(),
        ),
        Trailing::Empty => Some(
            div()
                .id(("doc-empty", ix))
                .test_support()
                .aria_label(EMPTY_TRAILING)
                .flex_none()
                .text_size(px(SMALL_TEXT))
                .text_color(colors.muted)
                .child(EMPTY_TRAILING)
                .into_any_element(),
        ),
        Trailing::None => match row.kind {
            DocKind::Nonconforming { .. } => Some(
                named_icon(
                    ("doc-nonconforming", ix),
                    "triangle-alert",
                    MARKER_SIZE,
                    colors.accent,
                    row.tooltip[0].clone(),
                )
                .into_any_element(),
            ),
            DocKind::Unreadable => Some(
                named_icon(
                    ("doc-unreadable", ix),
                    "circle-x",
                    MARKER_SIZE,
                    colors.red,
                    row.tooltip[0].clone(),
                )
                .into_any_element(),
            ),
            DocKind::Document if held => Some(
                named_icon(
                    ("doc-unsaved", ix),
                    "pencil",
                    MARKER_SIZE,
                    colors.accent,
                    UNSAVED_MARKER,
                )
                .into_any_element(),
            ),
            _ => None,
        },
    };

    ListRow::new(("doc-node", ix))
        .indent(px(INDENT + INDENT_STEP * entry.depth() as f32))
        .gap(px(GAP))
        .selected(selected)
        .focused(focused)
        .expanded(row.expanded)
        .label(row.label.clone())
        .description(row.tooltip.join("; "))
        .tooltip(row.tooltip.join("\n"))
        .when_some(
            node.document
                .as_ref()
                .map(|document| document.target.clone()),
            |this, target| {
                this.on_click(move |_, _, cx| {
                    let _ = workspace
                        .update(cx, |workspace, cx| workspace.open_item(target.clone(), cx));
                })
            },
        )
        .child(chevron)
        .child(icon(icon_name, ICON_SIZE, icon_color))
        .child(label)
        .children(trailing)
}
