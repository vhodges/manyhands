//! The Tickets pane: the Kit's list of the rows the panel shows under its
//! query, with the arrow and Enter bindings, Home and End, and the Why
//! blocked panel.
//!
//! The filter bar, the column header and the rows are drawn from
//! `filter_bar`, `columns` and `row`. A row is one line: type, short code,
//! title with its markers, status pill and age (Part C). Status is shown
//! as written, and an open ticket whose status is `closed` shows status
//! `closed`: lifecycle shows only as the Closed marker under All. A
//! nonconforming entry shows its path, and "needs fixing".

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, IndexPath, Selectable,
        list::{List, ListDelegate, ListState as KitListState},
        v_flex,
    },
    div,
};
use time::OffsetDateTime;

use self::row::ticket_row;
use super::{rows::ListRow, scroll::ContentScroll};
use crate::desktop::{
    keys::{ScrollPageDown, ScrollPageUp, ScrollToBottom, ScrollToTop, WHY_BLOCKED_CONTEXT},
    model::{
        markers::Markers,
        tickets::{Lifecycle, TicketRow},
    },
};

mod columns;
mod filter_bar;
mod row;

pub(crate) use filter_bar::{FILTER_PLACEHOLDER, choice_ids};

pub(crate) const LIFECYCLE_OPEN: &str = "Open";
pub(crate) const LIFECYCLE_CLOSED: &str = "Closed";
pub(crate) const WHY_BLOCKED: &str = "Why blocked";
pub(crate) const SELECT_BLOCKED: &str = "Select a ticket to see why it is blocked";
pub(crate) const NO_REASONS: &str = "The library gives no reasons for this entry";
/// The Why blocked panel grows to this height, then scrolls.
pub(crate) const WHY_BLOCKED_MAX_HEIGHT: gpui_kit::Pixels = gpui_kit::px(160.);

pub(crate) fn lifecycle_label(lifecycle: Lifecycle) -> &'static str {
    match lifecycle {
        Lifecycle::Open => LIFECYCLE_OPEN,
        Lifecycle::Closed => LIFECYCLE_CLOSED,
    }
}

/// The label of the row `item` makes.
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) fn row_label(item: &manyhands::repository::ItemDto) -> String {
    row::label(&TicketRow::from_item(item))
}

/// A row the panel shows, with the marks it carries: what the list draws,
/// and what it compares to know whether anything changed (R-32).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShownTicket {
    pub row: TicketRow,
    pub markers: Markers,
}

/// The Kit list's delegate: the rows shown and the one selected.
pub(crate) struct TicketDelegate {
    rows: Vec<ShownTicket>,
    selected: Option<IndexPath>,
    /// The repository's display name, for each row's tooltip.
    repo: String,
    /// The list's own focus, so the selected row draws its ring while the
    /// list holds focus.
    focus: Option<FocusHandle>,
}

impl TicketDelegate {
    pub(crate) fn row(&self, ix: IndexPath) -> Option<&TicketRow> {
        self.rows.get(ix.row).map(|shown| &shown.row)
    }
}

impl ListDelegate for TicketDelegate {
    type Item = ListRow;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<KitListState<Self>>,
    ) -> Option<ListRow> {
        let shown = self.rows.get(ix.row)?;
        let selected = self.selected.is_some_and(|selected| selected.eq_row(ix));
        let focused = self
            .focus
            .as_ref()
            .is_some_and(|focus| focus.is_focused(window));
        Some(
            ticket_row(ix.row, shown, &self.repo, OffsetDateTime::now_utc(), cx)
                .selected(selected)
                .focused(focused),
        )
    }

    /// An empty list draws nothing of the Kit's: the panel's body says what
    /// there is to say (R-34).
    fn render_empty(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<KitListState<Self>>,
    ) -> impl IntoElement {
        div()
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
            selected: None,
            repo: String::new(),
            focus: None,
        };
        let list = cx.new(|cx| KitListState::new(delegate, window, cx));
        list.update(cx, |list, cx| {
            let focus = list.focus_handle(cx);
            list.delegate_mut().focus = Some(focus);
        });
        Self {
            list,
            why_focus: cx.focus_handle().tab_stop(true),
            why_scroll: ContentScroll::default(),
        }
    }

    pub(crate) fn list(&self) -> &Entity<KitListState<TicketDelegate>> {
        &self.list
    }

    /// The repository's display name, which the rows' tooltips carry.
    pub(crate) fn set_repository(&self, name: &str, cx: &mut App) {
        if self.list.read(cx).delegate().repo != name {
            self.list.update(cx, |list, cx| {
                list.delegate_mut().repo = name.to_owned();
                cx.notify();
            });
        }
    }

    /// Shows `rows`. The selected row stays selected where the new rows
    /// still have it, by ID, or for an entry with no ID by worktree and
    /// path; where they do not, nothing is selected, so Enter never opens
    /// another ticket (Review Focus 1). Nothing happens when the rows are
    /// the ones shown.
    pub(crate) fn set(&self, rows: Vec<ShownTicket>, window: &mut Window, cx: &mut App) {
        if self.list.read(cx).delegate().rows == rows {
            return;
        }
        self.list.update(cx, |list, cx| {
            let selected = list
                .selected_index()
                .and_then(|ix| list.delegate().row(ix))
                .map(|row| (row.id.clone(), row.worktree.clone(), row.path.clone()));
            let at = selected.and_then(|(id, worktree, path)| {
                rows.iter().position(|shown| match (&shown.row.id, &id) {
                    (Some(shown), Some(selected)) => shown == selected,
                    (None, None) => shown.row.worktree == worktree && shown.row.path == path,
                    _ => false,
                })
            });
            list.delegate_mut().rows = rows;
            list.set_selected_index(at.map(IndexPath::new), window, cx);
            cx.notify();
        });
    }

    /// Selects the first or the last row and scrolls to it (R-15). Nothing
    /// when there is none.
    pub(crate) fn select_edge(&self, last: bool, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| {
            let count = list.delegate().rows.len();
            if count == 0 {
                return;
            }
            let row = if last { count - 1 } else { 0 };
            list.set_selected_index(Some(IndexPath::new(row)), window, cx);
            list.scroll_to_selected_item(window, cx);
        });
    }

    /// The rows shown, for tests.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn rows(&self, cx: &App) -> Vec<TicketRow> {
        let list = self.list.read(cx);
        list.delegate()
            .rows
            .iter()
            .map(|shown| shown.row.clone())
            .collect()
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

    /// The list, always drawn while a repository is selected (R-34).
    pub(crate) fn render(&self) -> impl InteractiveElement + IntoElement + use<> {
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
