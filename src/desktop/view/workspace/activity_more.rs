//! Activity's toolbar in the Workspace (Part C1): the filter, the ⋯ menu and
//! "Clear finished". Every entry point refuses behind a dialog or the
//! details sheet (R-60): the toolbar's press through `press_toolbar`, the
//! menu's choice through `unless_dialog`. One layer at a time (R-13):
//! opening ⋯ closes the row menu, the repository menu and the dock menu, and
//! each of them closes it.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{AnyElement, Bounds, Context, Pixels, Window};

use super::{ToolbarButton, Workspace, modal_open};
use crate::desktop::{model::activity::rows::RowKey, view::activity::more::ActivityMore};

impl Workspace {
    /// Whether the list shows only the rows that need attention.
    pub(in crate::desktop::view) fn activity_filter(&self) -> bool {
        self.activity.filter
    }

    /// The filter on or off: the list follows after this frame. A menu that
    /// is open closes with it, since it may stand on a row the filter hides.
    pub(in crate::desktop::view) fn toggle_activity_filter(&mut self, cx: &mut Context<Self>) {
        self.activity.filter = !self.activity.filter;
        self.activity.menu = None;
        self.activity.more = None;
        cx.notify();
    }

    /// The filter turned off, when it hides `key` (a row shown by "Show it"
    /// must be drawn).
    pub(in crate::desktop::view) fn show_row_through_filter(&mut self, key: &RowKey) {
        let hidden = self
            .activity
            .log
            .rows()
            .iter()
            .any(|row| &row.row.key == key && !row.row.attention);
        if hidden {
            self.activity.filter = false;
        }
    }

    /// The cell the ⋯ button records where it was drawn into.
    pub(in crate::desktop::view) fn activity_more_bounds_cell(
        &self,
    ) -> Rc<Cell<Option<Bounds<Pixels>>>> {
        self.activity.more_bounds.clone()
    }

    /// The ⋯ button's press: its menu below the button, with the one item
    /// enabled when a row is clearable. Focus belongs to the button.
    pub(in crate::desktop::view) fn open_activity_more(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        let button = self.toolbar_focus(ToolbarButton::ActivityMore).clone();
        let clearable = !self.activity.log.clearable().is_empty();
        let bounds = self.activity.more_bounds.get();
        self.activity.menu = None;
        self.repository_menu = None;
        self.footer.menu = None;
        self.attention.popover = None;
        self.activity.more = Some(ActivityMore::open(clearable, button, bounds, window, cx));
        cx.notify();
    }

    /// "Clear finished" (L8): every clearable row goes, a request through
    /// `dismiss_request` (a failure settled included) and the others
    /// (brought-in rows, the retirement events) through the log. Rows the log
    /// does not call clearable stay.
    pub(in crate::desktop::view) fn clear_finished(&mut self, cx: &mut Context<Self>) {
        for key in self.activity.log.clearable() {
            match key {
                RowKey::Request(id) => self.dismiss_request(id, cx),
                other => self.activity.log.dismiss(&other),
            }
        }
        self.refresh_activity(cx);
    }

    /// The menu closed: by a choice, Escape or a click elsewhere. The Kit has
    /// put focus back on the button.
    pub(in crate::desktop::view) fn activity_more_dismissed(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.activity.more.take().is_some() {
            cx.notify();
        }
    }

    /// The open ⋯ menu's layer, drawn by the root. A dialog or the sheet that
    /// opens over it closes it without a choice.
    pub(in crate::desktop::view) fn activity_more_layer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if modal_open(window, cx) {
            self.activity.more = None;
            return None;
        }
        self.activity
            .more
            .as_ref()
            .map(|more| more.layer(window, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl Workspace {
    /// Whether the ⋯ menu is open.
    pub(crate) fn activity_more_open(&self) -> bool {
        self.activity.more.is_some()
    }
}
