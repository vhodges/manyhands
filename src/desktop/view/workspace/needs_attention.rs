//! The Needs attention popover in the Workspace (Part E2): opening and
//! closing it, the rows it lists, and where focus goes.
//!
//! The popover is a filtered view of the log's rows: the ones that need
//! attention, newest first. One layer at a time (R-13): opening it closes
//! the row menu, ⋯, the repository menu and the dock menu, and each of them
//! closes it.

use gpui_kit::{AnyElement, Bounds, Context, FocusHandle, Pixels, Point, Window};

use super::{Region, Workspace, modal_open};
use crate::desktop::{
    keys::ActivityMenuChoice,
    model::activity::rows::{ActivityAction, ActivityRow, ItemRef, RowKey},
    view::{
        activity::{Step, row::choice_of},
        footer::{Control, focus_after_hiding, plan},
        needs_attention::{AttentionHandles, AttentionPopover},
    },
};

/// What the Workspace keeps for the popover: its three focus handles for its
/// whole life (`attention_focus`, R-50), and the popover while it is open.
pub(in crate::desktop::view) struct AttentionState {
    pub(super) handles: AttentionHandles,
    pub(super) popover: Option<AttentionPopover>,
    /// How many choices the popover's buttons and Enter made (P23).
    #[cfg(test)]
    pub(super) pressed: usize,
}

impl AttentionState {
    pub(in crate::desktop::view) fn new(cx: &mut Context<Workspace>) -> Self {
        Self {
            handles: AttentionHandles::new(cx),
            popover: None,
            #[cfg(test)]
            pressed: 0,
        }
    }
}

impl Workspace {
    /// The log's rows that need attention, newest first: what the footer
    /// counts and the popover lists.
    pub(in crate::desktop::view) fn attention_rows(&self) -> Vec<ActivityRow> {
        self.activity
            .log
            .rows()
            .iter()
            .filter(|row| row.row.attention)
            .cloned()
            .collect()
    }

    /// Where focus goes when the popover has closed and the footer item is
    /// where it belongs: the item while it is shown, else E1's control, the
    /// next shown control to its right (R-49, R-50).
    pub(in crate::desktop::view) fn attention_restore_target(
        &self,
        cx: &gpui_kit::App,
    ) -> FocusHandle {
        let status = self.share_facts(cx).status(self.attention(cx));
        let plan = plan(&self.layout, &status);
        let shown = plan
            .right
            .iter()
            .flatten()
            .any(|control| *control == Control::Attention);
        let control = if shown {
            Control::Attention
        } else {
            focus_after_hiding(Control::Attention, &plan).unwrap_or(Control::Settings)
        };
        self.footer.handle(control).clone()
    }

    /// The footer item pressed: the popover opens, or closes when it is
    /// open (E1). With nothing to show it stays shut. The press is refused
    /// behind a dialog or the sheet by `press`.
    pub(in crate::desktop::view) fn toggle_attention_popover(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.attention.popover.is_some() {
            self.close_attention_popover(window, cx);
        } else {
            self.open_attention_popover(window, cx);
        }
    }

    /// Opens the popover over the rows that need attention now, with focus in
    /// its list and the first row highlighted. Any other layer closes (R-13).
    fn open_attention_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.attention_rows();
        if rows.is_empty() {
            return;
        }
        self.activity.menu = None;
        self.activity.more = None;
        self.repository_menu = None;
        self.footer.menu = None;
        let mut popover = AttentionPopover::new(&self.attention.handles);
        popover.pane.apply(rows, window, cx);
        popover.pane.focus(window, cx);
        self.attention.popover = Some(popover);
        cx.notify();
    }

    /// Where the footer item was drawn last.
    fn attention_item_bounds(&self) -> Option<Bounds<Pixels>> {
        self.footer
            .bounds
            .borrow()
            .get(Control::Attention.id())
            .copied()
    }

    /// The popover's layer, drawn by the root. A dialog or the sheet that
    /// opens over it closes it without a choice (R-60); so does the count
    /// reaching 0, with focus moved by E1's rule when it was inside. When
    /// the log's rows differ from the ones drawn, they are applied after
    /// this frame (nothing notifies while the window draws).
    pub(in crate::desktop::view) fn attention_layer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if modal_open(window, cx) {
            self.attention.popover = None;
            return None;
        }
        self.attention.popover.as_ref()?;
        let item = self.attention_item_bounds();
        if self.attention(cx) == 0 || item.is_none() {
            self.attention.popover = None;
            let inside = window
                .focused(cx)
                .is_some_and(|focused| self.attention.handles.owns(&focused));
            if inside {
                // Now, not deferred: `focus_lost` fires in this draw and only
                // sends focus to the handles' footer fallback.
                let target = self.attention_restore_target(cx);
                window.focus(&target, cx);
            }
            return None;
        }
        let popover = self.attention.popover.as_ref()?;
        if !popover.pane.shows(self.activity.log.rows(), true) {
            cx.defer_in(window, |this, window, cx| {
                this.apply_attention_rows(window, cx)
            });
        }
        let weak = cx.entity().downgrade();
        Some(popover.layer(item?, &self.attention.handles, weak, window, cx))
    }

    /// Applies the log's rows that need attention to the popover: the
    /// refresh after a frame, and after a refused choice.
    fn apply_attention_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.attention_rows();
        let Some(popover) = &mut self.attention.popover else {
            return;
        };
        if popover.pane.apply(rows, window, cx) {
            cx.notify();
        }
    }

    /// A left press on a row: it is highlighted and the list has focus.
    pub(in crate::desktop::view) fn highlight_attention(
        &mut self,
        key: RowKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        let Some(popover) = &mut self.attention.popover else {
            return;
        };
        popover.pane.highlight(&key);
        popover.pane.focus(window, cx);
        cx.notify();
    }

    /// Up, down, Home or End in the popover. Focus on a button whose row
    /// the highlight leaves for one with none moves to the list.
    pub(in crate::desktop::view) fn attention_step(
        &mut self,
        step: Step,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(popover) = &mut self.attention.popover else {
            return;
        };
        if !popover.pane.step(step) {
            return;
        }
        let button = popover
            .pane
            .highlighted()
            .is_some_and(|row| row.row.button.as_ref().is_some_and(|offer| offer.enabled));
        if !button && self.attention.handles.button.is_focused(window) {
            self.attention.handles.list.focus(window, cx);
        }
        cx.notify();
    }

    /// Enter on the popover's list: the highlighted row's button when it is
    /// enabled; with no button, the row's item; with a disabled button, or
    /// neither, nothing. With focus on the button or the link, the control
    /// handles its own Enter, once.
    pub(in crate::desktop::view) fn attention_enter(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.attention.handles.list.is_focused(window) {
            cx.propagate();
            return;
        }
        let Some(row) = self
            .attention
            .popover
            .as_ref()
            .and_then(|popover| popover.pane.highlighted().cloned())
        else {
            return;
        };
        match (&row.row.button, &row.row.item) {
            (Some(offer), _) if offer.enabled => {
                self.press_attention(choice_of(&row, offer), window, cx)
            }
            (None, Some(item)) => self.open_attention_item(&row, item.clone(), window, cx),
            _ => {}
        }
    }

    /// A button pressed, or Enter on its row: the choice drawn on the row,
    /// with its revision. It runs only while the log's row still offers it
    /// (T8, R-55); otherwise nothing runs, and the popover stays open with
    /// the row refreshed.
    pub(in crate::desktop::view) fn press_attention(
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
            self.attention.pressed += 1;
        }
        if let Some(popover) = &mut self.attention.popover {
            popover.pane.highlight(&choice.key);
        }
        if !self.activity().still_offered(
            &choice.key,
            choice.revision,
            &choice.action,
            &choice.shown,
        ) {
            self.apply_attention_rows(window, cx);
            cx.notify();
            return;
        }
        // The popover closes and focus goes to the footer item; then the
        // action runs through `choose_activity` with the item as its origin.
        // An action that places focus itself does so after, and wins; a
        // dialog it opens gives focus back to the item, or to E1's control
        // once the item has hidden (R-49).
        let origin = self.footer.handle(Control::Attention).clone();
        self.attention.popover = None;
        window.focus(&origin, cx);
        self.choose_activity(&choice, origin, window, cx);
        cx.notify();
    }

    /// Enter on a row with no button: its item opens, unless the log's row
    /// has changed since it was drawn (T8), when nothing runs and the popover
    /// stays open with the row refreshed.
    fn open_attention_item(
        &mut self,
        row: &ActivityRow,
        item: ItemRef,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        #[cfg(test)]
        {
            self.attention.pressed += 1;
        }
        if !self.activity.log.is_current(&row.row.key, row.revision) {
            self.apply_attention_rows(window, cx);
            cx.notify();
            return;
        }
        let origin = self.footer.handle(Control::Attention).clone();
        self.attention.popover = None;
        window.focus(&origin, cx);
        self.activity.pane.highlight(&row.row.key);
        self.run_activity_action(&ActivityAction::OpenItem(item), origin, window, cx);
        cx.notify();
    }

    /// Tab and Shift-Tab: the popover's list, the highlighted row's button
    /// when it is enabled, and the link, wrapping (R-12).
    pub(in crate::desktop::view) fn attention_cycle(
        &mut self,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(popover) = &self.attention.popover else {
            return;
        };
        let handles = &self.attention.handles;
        let button = popover
            .pane
            .highlighted()
            .is_some_and(|row| row.row.button.as_ref().is_some_and(|offer| offer.enabled));
        let mut stops = vec![&handles.list];
        if button {
            stops.push(&handles.button);
        }
        stops.push(&handles.link);
        let at = stops.iter().position(|stop| stop.is_focused(window));
        let next = match (at, forward) {
            (None, true) => 0,
            (None, false) => stops.len() - 1,
            (Some(at), true) => (at + 1) % stops.len(),
            (Some(at), false) => (at + stops.len() - 1) % stops.len(),
        };
        let target = stops[next].clone();
        window.focus(&target, cx);
        cx.notify();
    }

    /// "Show everything in Activity": the popover closes, the filter goes
    /// off, Activity is revealed and its list takes focus with the first
    /// row highlighted (E2).
    pub(in crate::desktop::view) fn show_everything_in_activity(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if modal_open(window, cx) {
            return;
        }
        self.attention.popover = None;
        self.activity.filter = false;
        self.activity.menu = None;
        self.activity.more = None;
        // The pane shows every row now, and starts on the first.
        self.apply_activity_rows(window, cx);
        if let Some(first) = self.activity.log.rows().first() {
            let key = first.row.key.clone();
            self.activity.pane.highlight_when_shown(&key);
        }
        self.focus_region(Region::Operations, window, cx);
    }

    /// A press outside the popover closes it, and still reaches what is
    /// under it (R-11). A press inside the footer item's bounds is ignored:
    /// the item's click, on mouse up, toggles, so a full click closes it once
    /// (fact 47).
    pub(in crate::desktop::view) fn attention_press_outside(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.attention.popover.is_none()
            || modal_open(window, cx)
            || self
                .attention_item_bounds()
                .is_some_and(|bounds| bounds.contains(&position))
        {
            return;
        }
        // The press keeps the focus it gave. One that gave none leaves
        // focus on the list, which is no longer drawn: `focus_lost` sends it
        // to the footer item, or E1's control (R-50).
        self.attention.popover = None;
        cx.notify();
    }

    /// Closes the popover, focus going to the footer item, or E1's control
    /// when the item has hidden: Escape, a second press on the item, and the
    /// popover's own actions that place no focus themselves.
    pub(in crate::desktop::view) fn close_attention_popover(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.attention.popover.take().is_none() {
            return;
        }
        let target = self.attention_restore_target(cx);
        window.focus(&target, cx);
        cx.notify();
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl Workspace {
    /// Whether the popover is open.
    pub(crate) fn attention_popover_open(&self) -> bool {
        self.attention.popover.is_some()
    }

    /// The rows the popover has applied, as drawn.
    pub(crate) fn attention_popover_rows(&self) -> Vec<ActivityRow> {
        self.attention
            .popover
            .as_ref()
            .map(|popover| popover.pane.rows().to_vec())
            .unwrap_or_default()
    }

    /// The popover's highlighted row.
    pub(crate) fn attention_popover_cursor(&self) -> Option<RowKey> {
        self.attention
            .popover
            .as_ref()
            .and_then(|popover| popover.pane.highlighted_key().cloned())
    }

    /// How many choices the popover made.
    pub(crate) fn attention_pressed(&self) -> usize {
        self.attention.pressed
    }

    /// The popover's three focus handles: the list, the button, the link.
    pub(crate) fn attention_handles(&self) -> [FocusHandle; 3] {
        let handles = &self.attention.handles;
        [
            handles.list.clone(),
            handles.button.clone(),
            handles.link.clone(),
        ]
    }
}
