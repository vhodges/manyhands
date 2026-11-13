//! The Needs attention popover in the Workspace (Part E2): opening and
//! closing it, the rows it lists, and where focus goes.
//!
//! The popover is a filtered view of the log's rows: the ones that need
//! attention, newest first. One layer at a time (R-13): opening it closes
//! the row menu, ⋯, the repository menu and the dock menu, and each of them
//! closes it.

use gpui_kit::{Context, FocusHandle, Window};

use super::Workspace;
#[cfg(all(test, feature = "desktop-test-support"))]
use crate::desktop::model::activity::rows::RowKey;
use crate::desktop::{
    model::activity::rows::ActivityRow,
    view::{
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
