//! Resolve conflict as a sheet the workspace keeps (Wave 04 Cycle 05b
//! design B1, B2). Every way in calls `show_resolve` (R-9): the item
//! banner's Review changes, through the tab's `ItemRoute::Resolve`;
//! Activity's and Needs attention's, on a request's entry or a pending
//! row; and the lifecycle dialog's Review changes… and Repair locally…,
//! through `in_workspace`, which closes the dialog first. The sheet goes on
//! Root's one sheet layer, and the workspace owns it
//! (`SheetOwner::Resolve`) from the update that opens it, so no render of
//! `sync_sheet` sees it unowned; `sync_sheet` then keeps it whatever item
//! is shown, and clears the owner once it has closed.
//!
//! It never opens over a dialog or another sheet: no way in can fire with
//! a sheet open (Wave 03 D3 R-60), so a refusal loses nothing (R-21).

use gpui_kit::{App, Context, Entity, Window, component::WindowExt};

use super::{Workspace, content::SheetOwner};
use crate::desktop::view::conflict::{self, Opening, ResolveSheet};

impl Workspace {
    /// Resolve conflict is open: it owns the sheet, and the sheet is
    /// active (B1). Read by every refusal and wait, so a close the Kit made
    /// this frame is seen before `sync_sheet` clears the owner.
    pub(super) fn resolve_open(&self, window: &mut Window, cx: &mut App) -> bool {
        matches!(self.content.sheet, Some(SheetOwner::Resolve(_))) && window.has_active_sheet(cx)
    }

    /// One of the workspace's own sheets is open: Settings, the Add
    /// repository sheet or Resolve conflict (R-25).
    #[allow(dead_code)] // Used from Task 6
    pub(super) fn workspace_sheet_open(&self, window: &mut Window, cx: &mut App) -> bool {
        self.sheet_held(window, cx) || self.resolve_open(window, cx)
    }

    /// Opens Resolve conflict on `opening` (B2). `None` when refused: while
    /// a dialog is open or any sheet is active, and, through
    /// `conflict::open`, while quitting or saving drafts. The owner is set
    /// in the update that opens the sheet, and the sheet's own handle is
    /// focused after `open_sheet` has focused Root's (H1).
    pub(in crate::desktop::view) fn show_resolve(
        &mut self,
        opening: Opening,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<ResolveSheet>> {
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            return None;
        }
        let sheet = conflict::open(opening, &self.services, window, cx)?;
        self.content.sheet = Some(SheetOwner::Resolve(sheet.downgrade()));
        let focus = sheet.read(cx).own_focus().clone();
        window.focus(&focus, cx);
        cx.notify();
        Some(sheet)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resolve_sheet_tests.rs"]
mod tests;
