//! Activity (Wave 04 Cycle 03b, Part C): the panel that lists the log's rows,
//! one line each, in GPUI's `uniform_list`, with its own highlight and keys
//! (R-36), not the Kit `List`, whose click presses Confirm.
//!
//! The Workspace derives the rows into its log; the pane shows them after
//! the frame (`apply`, as the repositories panel's rows are: nothing
//! notifies during render), keeps the highlight on the same key across
//! refreshes and moves focus to the list when the row whose button held it
//! leaves. A click, Enter or a menu choice is built from the pane's applied
//! rows, the drawn ones, with their key and revision, so one made against a
//! row the log has since changed runs nothing (R-55; T8).

mod actions;
pub(crate) mod menu;
pub(crate) mod more;
pub(crate) mod row;

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use gpui_kit::{
    AnyElement, App, Bounds, FocusHandle, InteractiveElement, IntoElement, ParentElement, Pixels,
    ScrollStrategy, StatefulInteractiveElement, Styled, TestSupportExt, UniformListScrollHandle,
    WeakEntity, Window, canvas, div, uniform_list,
};

use super::{shell::tokens::chrome_colors, workspace::Workspace};
use crate::desktop::{
    keys::ACTIVITY_CONTEXT,
    model::activity::{
        copy::{NOTHING_NEEDS_ATTENTION, NOTHING_YET},
        rows::{ActivityRow, RowKey},
    },
};

/// Where each row was drawn last, by key, recorded during prepaint: the
/// anchor a key-opened menu takes (Part D).
pub(crate) type RowBounds = Rc<RefCell<BTreeMap<RowKey, Bounds<Pixels>>>>;

/// A move of the highlight: one row, or to an end (C2). It stops at the
/// ends, with no wrap.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Step {
    Up,
    Down,
    First,
    Last,
}

/// Activity's list: the rows as drawn, the highlighted key, and the handles
/// the panel's keys and Tab use.
pub(crate) struct ActivityPane {
    /// The rows applied: what the list draws and a press acts on.
    rows: Rc<Vec<ActivityRow>>,
    /// The highlighted row's key. It stays on the same key across
    /// refreshes (C2).
    highlight: Option<RowKey>,
    /// The list's own handle, inside the region's: the arrows, Enter and the
    /// menu keys work while it holds focus, and it is where focus goes back
    /// to after a dialog (C3).
    list_focus: FocusHandle,
    /// The highlighted row's button, when it is enabled: the only button
    /// that is a tab stop (R-33).
    button_focus: FocusHandle,
    /// A row to highlight once the rows that hold it are applied: "Show it"
    /// and Drafts not saved's Cancel name a row the pane may not draw yet.
    want: Option<RowKey>,
    scroll: UniformListScrollHandle,
    bounds: RowBounds,
    list_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl ActivityPane {
    pub(crate) fn new(list_focus: FocusHandle, button_focus: FocusHandle) -> Self {
        Self {
            rows: Rc::default(),
            highlight: None,
            want: None,
            list_focus,
            button_focus,
            scroll: UniformListScrollHandle::new(),
            bounds: RowBounds::default(),
            list_bounds: Rc::default(),
        }
    }

    /// The rows as drawn.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn rows(&self) -> &[ActivityRow] {
        &self.rows
    }

    pub(crate) fn highlighted_key(&self) -> Option<&RowKey> {
        self.highlight.as_ref()
    }

    /// The row `key`, as drawn.
    pub(crate) fn row(&self, key: &RowKey) -> Option<&ActivityRow> {
        self.rows.iter().find(|row| &row.row.key == key)
    }

    /// The highlighted row, as drawn.
    pub(crate) fn highlighted(&self) -> Option<&ActivityRow> {
        let key = self.highlight.as_ref()?;
        self.rows.iter().find(|row| &row.row.key == key)
    }

    pub(crate) fn list_focus(&self) -> &FocusHandle {
        &self.list_focus
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn button_focus(&self) -> &FocusHandle {
        &self.button_focus
    }

    /// Focuses the list. A list with rows and no highlight starts on the
    /// first.
    pub(crate) fn focus(&mut self, window: &mut Window, cx: &mut App) {
        self.list_focus.focus(window, cx);
        self.start_highlight();
    }

    /// A list with rows and no highlight starts on the first: whichever way
    /// it came to hold focus. An existing highlight is left alone.
    pub(crate) fn start_highlight(&mut self) {
        if self.highlight.is_none() {
            self.highlight = self.rows.first().map(|row| row.row.key.clone());
        }
    }

    /// Whether the pane already shows the log's `rows`, or only the ones
    /// that need attention when the filter is on (C1).
    pub(crate) fn shows(&self, rows: &[ActivityRow], filtered: bool) -> bool {
        self.rows
            .iter()
            .eq(rows.iter().filter(|row| !filtered || row.row.attention))
    }

    /// Shows `rows`, the log's now. The highlight stays on its key; when
    /// that row left, it goes to the row now at its position, else the
    /// last. Nothing is highlighted until the person moves in the list or
    /// focuses it, when it starts on the first row. When the highlighted row's
    /// button held focus and the row left, or the row now highlighted has no
    /// enabled button, focus moves to the list, so Escape and the arrows
    /// keep working (C2). Returns whether the rows changed.
    pub(crate) fn apply(
        &mut self,
        rows: Vec<ActivityRow>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if self.rows.as_slice() == rows.as_slice() {
            return false;
        }
        let held = self.button_focus.contains_focused(window, cx);
        let before = self.highlight.take();
        let at = before
            .as_ref()
            .and_then(|key| self.rows.iter().position(|row| &row.row.key == key));
        self.rows = Rc::new(rows);
        let kept = before
            .as_ref()
            .is_some_and(|key| self.rows.iter().any(|row| &row.row.key == key));
        // Where it goes when its row left: the row now at its position, else
        // the last; the first when nothing was highlighted.
        self.highlight = match (kept, at) {
            (true, _) => before.clone(),
            (false, Some(at)) => self
                .rows
                .get(at)
                .or_else(|| self.rows.last())
                .map(|row| row.row.key.clone()),
            (false, None) => None,
        };
        if let Some(wanted) = self.want.take()
            && self.rows.iter().any(|row| row.row.key == wanted)
        {
            self.highlight = Some(wanted);
            self.scroll_to_highlight();
        }
        if self.list_focus.contains_focused(window, cx) {
            self.start_highlight();
        }
        let kept = kept && self.highlight == before;
        let button = self
            .highlighted()
            .is_some_and(|row| row.row.button.as_ref().is_some_and(|offer| offer.enabled));
        if held && (!kept || !button) {
            self.list_focus.focus(window, cx);
        }
        if !kept && at.is_some() {
            self.scroll_to_highlight();
        }
        true
    }

    /// Moves the highlight; it stops at the ends (C2). Returns whether it
    /// moved.
    pub(crate) fn step(&mut self, step: Step) -> bool {
        if self.rows.is_empty() {
            return false;
        }
        let at = self
            .highlight
            .as_ref()
            .and_then(|key| self.rows.iter().position(|row| &row.row.key == key));
        let last = self.rows.len() - 1;
        let to = match (step, at) {
            (Step::First, _) | (Step::Up | Step::Down, None) => 0,
            (Step::Last, _) => last,
            (Step::Up, Some(at)) => at.saturating_sub(1),
            (Step::Down, Some(at)) => (at + 1).min(last),
        };
        let key = self.rows[to].row.key.clone();
        let moved = self.highlight.as_ref() != Some(&key);
        self.highlight = Some(key);
        self.scroll_to_highlight();
        moved
    }

    /// Highlights the row `key`, when the pane shows it.
    pub(crate) fn highlight(&mut self, key: &RowKey) -> bool {
        if self.rows.iter().any(|row| &row.row.key == key) {
            self.highlight = Some(key.clone());
            self.scroll_to_highlight();
            true
        } else {
            false
        }
    }

    /// Highlights the row `key` now when the pane shows it, else once the
    /// next rows are applied (once: a row that is still not there is
    /// dropped).
    pub(crate) fn highlight_when_shown(&mut self, key: &RowKey) {
        if !self.highlight(key) {
            self.want = Some(key.clone());
        }
    }

    fn scroll_to_highlight(&self) {
        let at = self
            .highlight
            .as_ref()
            .and_then(|key| self.rows.iter().position(|row| &row.row.key == key));
        if let Some(at) = at {
            self.scroll.scroll_to_item(at, ScrollStrategy::Nearest);
        }
    }

    /// Where the row `key` was drawn last, while it is drawn.
    pub(crate) fn row_bounds(&self, key: &RowKey) -> Option<Bounds<Pixels>> {
        self.bounds.borrow().get(key).copied()
    }

    /// Where the list was drawn last.
    pub(crate) fn list_bounds(&self) -> Option<Bounds<Pixels>> {
        self.list_bounds.get()
    }

    /// The tooltip each applied row draws, line by line (B5).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn tooltips(&self) -> Vec<Vec<String>> {
        self.rows.iter().map(row::tooltip).collect()
    }

    /// The list: a container that keeps the id `"operation-area"` (R-20),
    /// the list's key context and handle, and in it the rows, one line each,
    /// or the empty state.
    pub(crate) fn render(
        &self,
        workspace: WeakEntity<Workspace>,
        filtered: bool,
        cx: &App,
    ) -> AnyElement {
        let colors = chrome_colors(cx);
        let drawn = self.list_bounds.clone();
        let bounds = self.bounds.clone();
        let container = div()
            .id("operation-area")
            .test_support()
            .key_context(ACTIVITY_CONTEXT)
            .track_focus(&self.list_focus)
            .relative()
            .w_full()
            .flex_1()
            .min_h_0()
            // Prepainted before the rows, whose drawn ones record their
            // bounds after it: a row not drawn this frame, scrolled out or
            // gone, has none, so a key-opened menu anchors on the list.
            .child(canvas(
                move |_, _, _| bounds.borrow_mut().clear(),
                |_, (), _, _| {},
            ));
        let body = if self.rows.is_empty() {
            let empty = if filtered {
                NOTHING_NEEDS_ATTENTION
            } else {
                NOTHING_YET
            };
            div()
                .id("activity-empty")
                .test_support()
                .aria_label(empty)
                .p_2()
                .text_sm()
                .text_color(colors.muted)
                .child(empty)
                .into_any_element()
        } else {
            let (rows, highlight) = (self.rows.clone(), self.highlight.clone());
            let (list_focus, button_focus) = (self.list_focus.clone(), self.button_focus.clone());
            let bounds = self.bounds.clone();
            let listed = rows.len();
            uniform_list("activity-list", listed, move |range, window, cx| {
                let focused = list_focus.is_focused(window);
                range
                    .filter_map(|ix| {
                        let row = rows.get(ix)?;
                        Some(row::element(
                            row::Drawn {
                                ix,
                                highlighted: highlight.as_ref() == Some(&row.row.key),
                                list_focused: focused,
                                button_focus: button_focus.clone(),
                                bounds: bounds.clone(),
                                workspace: workspace.clone(),
                            },
                            row,
                            cx,
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element()
        };
        container
            .child(body)
            .child(
                canvas(move |at, _, _| drawn.set(Some(at)), |_, (), _, _| {})
                    .absolute()
                    .size_full(),
            )
            .into_any_element()
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "activity_tests.rs"]
pub(in crate::desktop::view) mod tests;
