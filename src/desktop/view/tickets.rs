//! The Tickets pane: the closure filter and the selected repository's
//! tickets in the Kit's list, with its arrow and Enter bindings.
//!
//! Columns: short code, title, type, status, lifecycle and change time.
//! Status is shown as written and lifecycle comes from closure, both as
//! text: an open ticket whose status is `closed` shows status `closed`
//! and lifecycle Open. A nonconforming entry shows its path, the word
//! Nonconforming and its problem code.

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, IndexPath, h_flex,
        list::{List, ListDelegate, ListItem, ListState as KitListState},
        v_flex,
    },
    div,
};
use manyhands::repository::{ClosureFilter, ReadinessFilter};

use super::scroll::ContentScroll;
use crate::desktop::{
    keys::{ScrollPageDown, ScrollPageUp, ScrollToBottom, ScrollToTop, WHY_BLOCKED_CONTEXT},
    model::tickets::{Lifecycle, TicketRow},
};

pub(crate) const LIFECYCLE_OPEN: &str = "Open";
pub(crate) const LIFECYCLE_CLOSED: &str = "Closed";
pub(crate) const NONCONFORMING: &str = "Nonconforming";
pub(crate) const NO_SHORT_CODE: &str = "No short code";
pub(crate) const WHY_BLOCKED: &str = "Why blocked";
pub(crate) const SELECT_BLOCKED: &str = "Select a ticket to see why it is blocked";
pub(crate) const NO_REASONS: &str = "The library gives no reasons for this entry";
/// The readiness choice that asks for none.
pub(crate) const ANY_READINESS: &str = "Any readiness";
/// The Why blocked panel grows to this height, then scrolls.
pub(crate) const WHY_BLOCKED_MAX_HEIGHT: gpui_kit::Pixels = gpui_kit::px(160.);

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

/// The readiness filters, in the order they are offered: their element
/// id, label, what they show and the readiness they ask for. A filter
/// never hides a nonconforming entry: the library keeps it under each.
pub(crate) const READINESS_FILTERS: [(&str, &str, &str, Option<ReadinessFilter>); 3] = [
    (
        "filter-any-readiness",
        ANY_READINESS,
        "Showing tickets ready or not",
        None,
    ),
    (
        "filter-ready",
        "Ready",
        "Showing ready tickets",
        Some(ReadinessFilter::Ready),
    ),
    (
        "filter-blocked",
        "Blocked",
        "Showing blocked tickets and why",
        Some(ReadinessFilter::Blocked),
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
    row.name().title_or(&row.path).to_owned()
}

/// The label of the row `item` makes.
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) fn row_label(item: &manyhands::repository::ItemDto) -> String {
    label(&TicketRow::from_item(item))
}

/// The row's accessible description: its reasons, in full, so a screen
/// reader hears them on landing on the row. None when it has none.
pub(crate) fn row_description(row: &TicketRow) -> Option<String> {
    (!row.reasons.is_empty()).then(|| row.reasons.join("; "))
}

/// The line of why the row at `index` is blocked: each reason in text,
/// on one line, so every row keeps the height the list measured.
fn reasons(row: &TicketRow, index: usize) -> impl IntoElement + use<> {
    h_flex()
        .gap_2()
        .overflow_hidden()
        .text_xs()
        .whitespace_nowrap()
        // A space keeps the line's height when there is no reason.
        .child("\u{a0}")
        .children(row.reasons.iter().enumerate().map(|(n, reason)| {
            div()
                .id(SharedString::from(format!("ticket-reason-{index}-{n}")))
                .test_support()
                .aria_label(reason.clone())
                .child(reason.clone())
        }))
}

/// The Kit list's delegate: the rows shown and the one selected.
pub(crate) struct TicketDelegate {
    rows: Vec<TicketRow>,
    /// Some row gives reasons. The Kit's list measures one row for all,
    /// so then every row has a line for them, empty where it gives none.
    reasons_shown: bool,
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
                    row.name().short_or(NO_SHORT_CODE).to_owned(),
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
        let mut entry = v_flex()
            .id(("ticket-row", index))
            .test_support()
            .aria_label(label(row))
            .aria_selected(selected);
        if let Some(description) = row_description(row) {
            entry = entry.aria_description(description);
        }
        Some(
            ListItem::new(("ticket-item", index)).child(
                entry
                    .child(div().text_sm().child(label(row)))
                    .child(columns.text_color(muted))
                    .children(self.reasons_shown.then(|| reasons(row, index))),
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
    /// The Why blocked panel's focus, a tab stop, and its scrolling.
    why_focus: FocusHandle,
    why_scroll: ContentScroll,
}

impl TicketsPane {
    pub(crate) fn new(window: &mut Window, cx: &mut App) -> Self {
        let delegate = TicketDelegate {
            rows: Vec::new(),
            reasons_shown: false,
            selected: None,
        };
        Self {
            list: cx.new(|cx| KitListState::new(delegate, window, cx)),
            why_focus: cx.focus_handle().tab_stop(true),
            why_scroll: ContentScroll::default(),
        }
    }

    pub(crate) fn list(&self) -> &Entity<KitListState<TicketDelegate>> {
        &self.list
    }

    /// Shows `rows`, none selected.
    pub(crate) fn set(&self, rows: Vec<TicketRow>, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| {
            // The selected row stays selected where the new rows still
            // have it, by ID or, for a nonconforming entry, by path.
            let selected = list
                .selected_index()
                .and_then(|ix| list.delegate().rows.get(ix.row))
                .map(|row| (row.id.clone(), row.path.clone()));
            let at = selected.and_then(|(id, path)| {
                rows.iter()
                    .position(|row| row.id == id && (id.is_some() || row.path == path))
            });
            list.delegate_mut().reasons_shown = rows.iter().any(|row| !row.reasons.is_empty());
            list.delegate_mut().rows = rows;
            list.set_selected_index(at.map(IndexPath::new), window, cx);
            cx.notify();
        });
    }

    /// The rows shown, for tests.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn rows(&self, cx: &App) -> Vec<TicketRow> {
        self.list.read(cx).delegate().rows.clone()
    }

    /// The selected row, if any.
    pub(crate) fn selected_row(&self, cx: &App) -> Option<TicketRow> {
        let list = self.list.read(cx);
        let ix = list.selected_index()?;
        list.delegate().row(ix).cloned()
    }

    /// The Why blocked panel, outside the Kit's virtual list so it can
    /// wrap: the selected row's reasons in full, one per line, as
    /// `blocked_reasons` gave them for this read; a fixed hint when no row
    /// is selected, or the selected one has none.
    ///
    /// It grows to `WHY_BLOCKED_MAX_HEIGHT` and then scrolls. It is a tab
    /// stop, ringed while focused, and PageUp, PageDown, Home and End
    /// scroll it.
    pub(crate) fn why_blocked(&self, window: &Window, cx: &App) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let focused = self.why_focus.is_focused(window);
        let ring = focused.then(|| {
            div()
                .id("why-blocked-focus-ring")
                .test_support()
                .absolute()
                .inset_0()
                .border_2()
                .border_color(cx.theme().ring)
        });
        let scroll = |pages: Option<f32>, end: bool| {
            let scroll = self.why_scroll.clone();
            move |window: &mut Window| {
                let moved = match pages {
                    Some(pages) => scroll.page(pages),
                    None if end => scroll.to_bottom(),
                    None => scroll.to_top(),
                };
                if moved {
                    window.refresh();
                }
            }
        };
        let (up, down, top, bottom) = (
            scroll(Some(-1.), false),
            scroll(Some(1.), false),
            scroll(None, false),
            scroll(None, true),
        );
        let selected = self.selected_row(cx);
        let hint = match &selected {
            None => Some(SELECT_BLOCKED),
            Some(row) if row.reasons.is_empty() => Some(NO_REASONS),
            Some(_) => None,
        };
        let reasons = selected.map(|row| row.reasons).unwrap_or_default();
        let body = v_flex()
            .id("why-blocked-body")
            .gap_1()
            .p_1()
            .max_h(WHY_BLOCKED_MAX_HEIGHT)
            .overflow_y_scroll()
            .track_scroll(self.why_scroll.handle())
            .child(
                div()
                    .id("why-blocked-title")
                    .test_support()
                    .aria_label(WHY_BLOCKED)
                    .text_sm()
                    .child(WHY_BLOCKED),
            )
            .children(hint.map(|hint| {
                div()
                    .id("why-blocked-hint")
                    .test_support()
                    .aria_label(hint)
                    .text_xs()
                    .text_color(muted)
                    .child(hint)
            }))
            .children(reasons.into_iter().enumerate().map(|(n, reason)| {
                div()
                    .id(("why-blocked-reason", n))
                    .test_support()
                    .aria_label(reason.clone())
                    .text_xs()
                    .whitespace_normal()
                    .child(reason)
            }));
        div()
            .id("why-blocked")
            .test_support()
            .aria_label(WHY_BLOCKED)
            .track_focus(&self.why_focus)
            .key_context(WHY_BLOCKED_CONTEXT)
            .on_action(move |_: &ScrollPageUp, window, _| up(window))
            .on_action(move |_: &ScrollPageDown, window, _| down(window))
            .on_action(move |_: &ScrollToTop, window, _| top(window))
            .on_action(move |_: &ScrollToBottom, window, _| bottom(window))
            .relative()
            .border_1()
            .border_color(cx.theme().border)
            .child(body)
            .children(ring)
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
