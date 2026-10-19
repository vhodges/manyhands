//! The Tickets pane: the closure filter and the selected repository's
//! tickets in the Kit's list, with its arrow and Enter bindings.
//!
//! Columns: short code, title, type, status, lifecycle and change time.
//! Status is shown as written and lifecycle comes from closure, both as
//! text: an open ticket whose status is `closed` shows status `closed`
//! and lifecycle Open. A nonconforming entry shows its path, the word
//! Nonconforming and its problem code.

use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, IndexPath, h_flex,
        list::{List, ListDelegate, ListItem, ListState as KitListState},
        v_flex,
    },
    div,
};
use manyhands::repository::ClosureFilter;

use crate::desktop::model::tickets::{Lifecycle, TicketRow};

pub(crate) const LIFECYCLE_OPEN: &str = "Open";
pub(crate) const LIFECYCLE_CLOSED: &str = "Closed";
pub(crate) const NONCONFORMING: &str = "Nonconforming";
pub(crate) const NO_SHORT_CODE: &str = "No short code";

/// The closure filters, in the order they are offered: their element id,
/// label and what they show.
pub(crate) const CLOSURE_FILTERS: [(&str, &str, &str, ClosureFilter); 3] = [
    (
        "filter-open",
        "Open",
        "Showing open tickets",
        ClosureFilter::Open,
    ),
    (
        "filter-closed",
        "Closed",
        "Showing closed tickets",
        ClosureFilter::Closed,
    ),
    (
        "filter-all",
        "All",
        "Showing all tickets",
        ClosureFilter::All,
    ),
];

pub(crate) fn lifecycle_label(lifecycle: Lifecycle) -> &'static str {
    match lifecycle {
        Lifecycle::Open => LIFECYCLE_OPEN,
        Lifecycle::Closed => LIFECYCLE_CLOSED,
    }
}

/// What a row is labeled by: its title, or for a nonconforming entry,
/// which has none, its path.
fn label(row: &TicketRow) -> String {
    row.title.clone().unwrap_or_else(|| row.path.clone())
}

/// The label of the row `item` makes.
#[cfg(test)]
pub(crate) fn row_label(item: &manyhands::repository::ItemDto) -> String {
    label(&TicketRow::from_item(item))
}

/// The Kit list's delegate: the rows shown and the one selected.
pub(crate) struct TicketDelegate {
    rows: Vec<TicketRow>,
    selected: Option<IndexPath>,
}

impl TicketDelegate {
    pub(crate) fn row(&self, ix: IndexPath) -> Option<&TicketRow> {
        self.rows.get(ix.row)
    }
}

impl ListDelegate for TicketDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<KitListState<Self>>,
    ) -> Option<ListItem> {
        let row = self.rows.get(ix.row)?;
        let index = ix.row;
        let selected = self.selected.is_some_and(|selected| selected.eq_row(ix));
        let muted = cx.theme().muted_foreground;
        let cell = |name: &'static str, text: String| {
            div()
                .id((name, index))
                .test_support()
                .aria_label(text.clone())
                .text_xs()
                .child(text)
        };
        let columns = match row.nonconforming {
            Some(problem) => h_flex()
                .gap_2()
                .child(cell("ticket-nonconforming", NONCONFORMING.to_owned()))
                .child(cell("ticket-problem", problem.as_str().to_owned())),
            None => h_flex()
                .gap_2()
                .child(cell(
                    "ticket-slug",
                    row.slug.clone().unwrap_or_else(|| NO_SHORT_CODE.to_owned()),
                ))
                .children(
                    row.ticket_type
                        .clone()
                        .map(|text| cell("ticket-type", text)),
                )
                .children(row.status.clone().map(|text| cell("ticket-status", text)))
                .children(row.lifecycle.map(|lifecycle| {
                    cell("ticket-lifecycle", lifecycle_label(lifecycle).to_owned())
                }))
                .children(
                    row.changed_at
                        .clone()
                        .map(|text| cell("ticket-changed", text)),
                ),
        };
        Some(
            ListItem::new(("ticket-item", index)).child(
                v_flex()
                    .id(("ticket-row", index))
                    .test_support()
                    .aria_label(label(row))
                    .aria_selected(selected)
                    .child(div().text_sm().child(label(row)))
                    .child(columns.text_color(muted)),
            ),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<KitListState<Self>>,
    ) {
        self.selected = ix;
        cx.notify();
    }
}

/// The list's Kit state.
pub(crate) struct TicketsPane {
    list: Entity<KitListState<TicketDelegate>>,
}

impl TicketsPane {
    pub(crate) fn new(window: &mut Window, cx: &mut App) -> Self {
        let delegate = TicketDelegate {
            rows: Vec::new(),
            selected: None,
        };
        Self {
            list: cx.new(|cx| KitListState::new(delegate, window, cx)),
        }
    }

    pub(crate) fn list(&self) -> &Entity<KitListState<TicketDelegate>> {
        &self.list
    }

    /// Shows `rows`, none selected.
    pub(crate) fn set(&self, rows: Vec<TicketRow>, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| {
            list.delegate_mut().rows = rows;
            list.set_selected_index(None, window, cx);
            cx.notify();
        });
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| list.focus(window, cx));
    }

    pub(crate) fn render(&self) -> impl IntoElement + use<> {
        div()
            .id("tickets-list")
            .test_support()
            .flex_1()
            .min_h_0()
            .child(List::new(&self.list))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "tickets_tests.rs"]
mod tests;
