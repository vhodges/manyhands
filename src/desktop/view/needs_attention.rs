//! The Needs attention popover (Wave 04 Cycle 03b, Part E2): a filtered view
//! of the Activity log's rows, the ones that need the person, opened from the
//! footer's Needs attention item.
//!
//! The popover keeps its rows the way Activity's list does, in an
//! `ActivityPane` it owns: the rows applied are the ones drawn, the highlight
//! follows its `RowKey`, and when the highlighted row changes while its
//! button holds focus, focus moves to the popover's list (C2, E2). A click
//! or Enter builds its choice from the drawn row, with its key and revision
//! (R-55), so one made against a row the log has since changed runs nothing
//! (T8). The three focus handles (the list, the highlighted row's button and
//! the link) belong to the Workspace for its whole life, as
//! `attention_focus`: a dialog or menu opened from the popover remembers the
//! list as the place focus returns to, and `Workspace::focus_lost` sends a
//! focus lost on any of them to the footer (R-50, U-8).

use gpui_kit::{Context, FocusHandle};

use super::{activity::ActivityPane, workspace::Workspace};

/// The popover's three tab stops, which the Workspace keeps across opens.
#[derive(Clone)]
pub(crate) struct AttentionHandles {
    /// The list: it holds the highlight, and is `attention_focus`.
    pub(crate) list: FocusHandle,
    /// The highlighted row's enabled button.
    pub(crate) button: FocusHandle,
    /// "Show everything in Activity".
    pub(crate) link: FocusHandle,
}

impl AttentionHandles {
    pub(crate) fn new(cx: &mut Context<Workspace>) -> Self {
        Self {
            list: cx.focus_handle().tab_stop(true),
            button: cx.focus_handle().tab_stop(true),
            link: cx.focus_handle().tab_stop(true),
        }
    }

    /// Whether `handle` is one of the three.
    pub(crate) fn owns(&self, handle: &FocusHandle) -> bool {
        [&self.list, &self.button, &self.link].contains(&handle)
    }
}

/// An open popover. Its focus handles are the Workspace's
/// (`AttentionHandles`), and the place focus goes back to is the footer
/// item's, which the Workspace holds: neither is kept here.
pub(crate) struct AttentionPopover {
    /// The rows applied (the drawn ones), the highlight by key (the
    /// cursor) and the scroll: Activity's own list state, over the
    /// attention rows.
    pub(crate) pane: ActivityPane,
}

impl AttentionPopover {
    pub(crate) fn new(handles: &AttentionHandles) -> Self {
        Self {
            pane: ActivityPane::new(handles.list.clone(), handles.button.clone()),
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "needs_attention_tests.rs"]
mod tests;
