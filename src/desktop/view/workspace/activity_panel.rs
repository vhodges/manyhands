//! Activity's list in the Workspace (Part C): showing the log's rows after
//! the frame, the keys that move the highlight and press its row, and a
//! click's or a button's choice. A choice is made against the pane's
//! applied row, the drawn one (R-55), and `choose_activity` re-checks it
//! against the log; focus goes back to the list after a dialog (C3).

use gpui_kit::{AnyElement, Context, Window};

use super::{Workspace, modal_open};
use crate::desktop::{
    keys::ActivityMenuChoice,
    model::activity::rows::{ActivityAction, ActivityRow, ItemRef, RowKey},
    view::activity::{Step, row::choice_of},
};

impl Workspace {
    /// The panel's list: the rows as drawn, the highlight and the handles.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn activity_pane(&self) -> &crate::desktop::view::activity::ActivityPane {
        &self.activity.pane
    }

    /// The list, drawn from the pane's applied rows. When the log holds
    /// others, they are applied after this frame: nothing notifies while
    /// the window draws (C2).
    pub(in crate::desktop::view) fn activity_list(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // The list holds focus, however it came to: it starts on its first
        // row (an existing highlight stays).
        if self.activity.pane.list_focus().contains_focused(window, cx) {
            self.activity.pane.start_highlight();
        }
        let filtered = self.activity.filter;
        if !self.activity.pane.shows(self.activity.log.rows(), filtered) {
            cx.defer_in(window, |this, window, cx| {
                this.apply_activity_rows(window, cx)
            });
        }
        self.activity
            .pane
            .render(cx.entity().downgrade(), filtered, cx)
    }

    /// The log's rows the list shows: all of them, or with the filter on
    /// only those that need attention (C1).
    fn shown_activity_rows(&self) -> Vec<ActivityRow> {
        self.activity
            .log
            .rows()
            .iter()
            .filter(|row| !self.activity.filter || row.row.attention)
            .cloned()
            .collect()
    }

    /// Applies the log's rows, filtered, to the pane, and draws once more
    /// when they differ from those shown.
    fn apply_activity_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.shown_activity_rows();
        if self.activity.pane.apply(rows, window, cx) {
            cx.notify();
        }
    }

    /// Highlights the row `key`, when the pane draws it: a choice made on a
    /// row leaves it the highlighted one, whether it came from its button,
    /// its menu or Enter (C3).
    pub(in crate::desktop::view) fn highlight_applied_row(&mut self, key: &RowKey) {
        self.activity.pane.highlight(key);
    }

    /// A left press on a row: it is highlighted and the list has focus.
    pub(in crate::desktop::view) fn highlight_activity(
        &mut self,
        key: RowKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        self.activity.pane.highlight(&key);
        self.activity.pane.focus(window, cx);
        cx.notify();
    }

    /// Up, down, Home or End in the list.
    pub(in crate::desktop::view) fn activity_step(&mut self, step: Step, cx: &mut Context<Self>) {
        if self.activity.pane.step(step) {
            cx.notify();
        }
    }

    /// Enter on the list: the highlighted row's button when it is enabled;
    /// with no button, the row's item; with a disabled button, or with
    /// neither, nothing (decisions 11 and 24). With focus on the button
    /// itself, the button handles its own Enter, once.
    pub(in crate::desktop::view) fn activity_enter(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.activity.pane.list_focus().is_focused(window) {
            cx.propagate();
            return;
        }
        self.activity.pane.start_highlight();
        let Some(row) = self.activity.pane.highlighted().cloned() else {
            return;
        };
        match (&row.row.button, &row.row.item) {
            (Some(offer), _) if offer.enabled => {
                self.press_activity(choice_of(&row, offer), window, cx)
            }
            (None, Some(item)) => self.open_activity_item(&row, item.clone(), window, cx),
            _ => {}
        }
    }

    /// A button pressed, or Enter on its row: the choice drawn on the row,
    /// with its revision, through `choose_activity`, which runs it only
    /// while the log's row still offers it (T8). The row is highlighted,
    /// and the list is where focus goes back to (C3).
    pub(in crate::desktop::view) fn press_activity(
        &mut self,
        choice: ActivityMenuChoice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        #[cfg(test)]
        {
            self.activity.pressed += 1;
        }
        self.activity.pane.highlight(&choice.key);
        let origin = self.activity.pane.list_focus().clone();
        self.choose_activity(&choice, origin, window, cx);
        cx.notify();
    }

    /// Enter on a row with no button: its item opens, unless the log's row
    /// has changed since it was drawn (T8).
    fn open_activity_item(
        &mut self,
        row: &ActivityRow,
        item: ItemRef,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) || !self.activity.log.is_current(&row.row.key, row.revision) {
            return;
        }
        #[cfg(test)]
        {
            self.activity.pressed += 1;
        }
        let origin = self.activity.pane.list_focus().clone();
        self.run_activity_action(&ActivityAction::OpenItem(item), origin, window, cx);
    }

    /// How many choices the list made.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn activity_pressed(&self) -> usize {
        self.activity.pressed
    }
}
