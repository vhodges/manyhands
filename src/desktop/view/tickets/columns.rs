//! The Tickets list's column header (Part C, R-2): the app's own row of sort
//! buttons, outside the scrolling list so it stays in place.
//!
//! Each button is a tab stop that Enter or Space activates. Activating a new
//! column sorts it ascending; activating the active one reverses it. The
//! active column shows an arrow up or down, the others a faint pair; the
//! order is in the button's name, since there is no `aria_sort`.

use gpui_kit::{
    InteractiveElement, IntoElement, ParentElement, Role, SharedString, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    component::{Icon, h_flex, tooltip::Tooltip},
    div,
    prelude::FluentBuilder,
    px,
};

use super::filter_bar::inset_ring;
use crate::desktop::{
    assets::icon_path,
    model::tickets::{SortColumn, TicketSort},
    view::{shell::tokens::chrome_colors, workspace::Workspace},
};

/// The header's height.
const HEIGHT: f32 = 24.;
const TEXT_SIZE: f32 = 11.;
const ARROW_SIZE: f32 = 12.;

/// The columns, in the order they are drawn: element id, the column, its
/// heading (none for the type column, which shows only its arrow) and the
/// word its name says.
const COLUMNS: [(&str, SortColumn, Option<&str>, &str); 5] = [
    ("sort-type", SortColumn::Type, None, "type"),
    ("sort-code", SortColumn::Code, Some("CODE"), "code"),
    ("sort-title", SortColumn::Title, Some("TITLE"), "title"),
    ("sort-status", SortColumn::Status, Some("STATUS"), "status"),
    ("sort-updated", SortColumn::Updated, Some("UPD."), "updated"),
];

/// The ids of the five buttons.
pub(super) const SORT_IDS: [&str; 5] = [
    COLUMNS[0].0,
    COLUMNS[1].0,
    COLUMNS[2].0,
    COLUMNS[3].0,
    COLUMNS[4].0,
];

/// A column button's accessible name: the active one says its order,
/// "Sort by code, A to Z", "Sort by updated, newest first"; another only
/// "Sort by code". For Updated, ascending is newest first.
pub(crate) fn sort_name(word: &str, column: SortColumn, sort: TicketSort) -> String {
    if sort.column != column {
        return format!("Sort by {word}");
    }
    let order = match (column, sort.ascending) {
        (SortColumn::Updated, true) => "newest first",
        (SortColumn::Updated, false) => "oldest first",
        (_, true) => "A to Z",
        (_, false) => "Z to A",
    };
    format!("Sort by {word}, {order}")
}

impl Workspace {
    /// The column header, drawn when a repository is selected.
    pub(in crate::desktop::view) fn column_header(
        &self,
        window: &Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = chrome_colors(cx);
        let sort = self.tickets().query().sort;
        h_flex()
            .id("tickets-columns")
            .flex_none()
            .w_full()
            .h(px(HEIGHT))
            .pl(px(12.))
            .pr(px(10.))
            .gap(px(8.))
            .items_center()
            .border_b_1()
            .border_color(colors.border)
            .children(COLUMNS.map(|(id, column, heading, word)| {
                let workspace = cx.entity().downgrade();
                let active = sort.column == column;
                let name = sort_name(word, column, sort);
                let tooltip: SharedString = format!("Sort by {word}").into();
                let focus = self.choice_focus(id);
                let focused = focus.is_some_and(|focus| focus.is_focused(window));
                let arrow = match (active, sort.ascending) {
                    (false, _) => "arrow-up-down",
                    (true, true) => "arrow-up",
                    (true, false) => "arrow-down",
                };
                let tone = if active { colors.text } else { colors.muted };
                let mut button = div().id(id).test_support();
                if let Some(focus) = focus {
                    button = button.track_focus(focus);
                }
                let cell = button
                    .role(Role::Button)
                    .aria_label(name)
                    .aria_selected(active)
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .h_full()
                    .text_size(px(TEXT_SIZE))
                    .text_color(tone)
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .when(!active, |cell| {
                        cell.hover(|style| style.text_color(colors.text))
                    })
                    .on_click(move |_, _, cx| {
                        let _ = workspace.update(cx, |this, cx| this.set_sort(column, cx));
                    })
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                    .children(heading)
                    .child(
                        Icon::default()
                            .path(icon_path(arrow))
                            .w(px(ARROW_SIZE))
                            .h(px(ARROW_SIZE))
                            .flex_none()
                            .text_color(tone)
                            .when(!active, |icon| icon.opacity(0.5)),
                    )
                    .when(focused, |cell| cell.child(inset_ring(cx)));
                // Each column has the width its cells have below.
                match column {
                    SortColumn::Type => div().flex_none().w(px(14.)).child(cell),
                    SortColumn::Code => div().flex_none().w(px(78.)).child(cell),
                    SortColumn::Title => div().flex_1().min_w(px(96.)).child(cell),
                    SortColumn::Status => div().flex_none().w(px(92.)).child(cell),
                    SortColumn::Updated => div()
                        .flex_none()
                        .w(px(30.))
                        .flex()
                        .justify_end()
                        .child(cell),
                }
            }))
    }
}
