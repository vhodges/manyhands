//! Resolve conflict as a sheet the workspace keeps (Wave 04 Cycle 05b
//! design B1 to B3). Every way in calls `show_resolve` (R-9): the item
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
//! a sheet open (Wave 03 D3 R-60), so a refusal loses nothing (R-21). The
//! sheet closes itself only while the workspace keeps it (`close_resolve`,
//! R-22); a save it hears answered under a dialog waits for none to be
//! open (`resolve_waiting`, R-26); and focus taken behind it comes back to
//! its own handle (`keep_focus_in_resolve`, R-11).

use gpui_kit::{App, Context, Entity, WeakEntity, Window, component::WindowExt};
use manyhands::repository::RequestId;

use super::{Workspace, content::SheetOwner};
use crate::desktop::view::conflict::{self, Opening, ResolveSheet};

/// A sheet's applied resolution that landed under a dialog, for the rest
/// of `applied` (close, focus, reopen) once none is open (R-26).
pub(super) struct ResolveWaiting {
    sheet: WeakEntity<ResolveSheet>,
    resolution: RequestId,
}

impl Workspace {
    /// Resolve conflict is open: it owns the sheet, and the sheet is
    /// active (B1). Every refusal and wait reads it (`settings_key`,
    /// `show_settings`, `show_add_repository`, the deferred Republish…,
    /// `apply_review`'s requests, `sync_sheet`'s details sheet,
    /// `keep_focus_in_resolve`), so a close the Kit made this frame is seen
    /// before `sync_sheet` clears the owner.
    pub(in crate::desktop::view) fn resolve_open(&self, window: &mut Window, cx: &mut App) -> bool {
        matches!(self.content.sheet, Some(SheetOwner::Resolve(_))) && window.has_active_sheet(cx)
    }

    /// One of the workspace's own sheets is open: Settings, the Add
    /// repository sheet or Resolve conflict (R-25).
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

    /// Closes `sheet`'s Kit sheet, only while the workspace keeps that
    /// sheet open: an answer that lands after Escape closes nothing opened
    /// since (B3, R-22). Gives whether it closed; on false the sheet does
    /// nothing more (R-27). The owner is cleared at once.
    pub(in crate::desktop::view) fn close_resolve(
        &mut self,
        sheet: &Entity<ResolveSheet>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let kept = matches!(
            &self.content.sheet,
            Some(SheetOwner::Resolve(owner)) if owner.entity_id() == sheet.entity_id()
        );
        if !kept || !window.has_active_sheet(cx) {
            return false;
        }
        self.content.sheet = None;
        window.close_sheet(cx);
        cx.notify();
        true
    }

    /// `sheet`'s resolution `resolution` applied while a dialog is open
    /// (an Unlock prompt over the sheet): its close waits for none to be
    /// (R-26).
    pub(in crate::desktop::view) fn wait_to_close_resolve(
        &mut self,
        sheet: WeakEntity<ResolveSheet>,
        resolution: RequestId,
        cx: &mut Context<Self>,
    ) {
        self.content.resolve_waiting = Some(ResolveWaiting { sheet, resolution });
        cx.notify();
    }

    /// The waiting close, once no dialog is open, called from the render
    /// before `sync_sheet`: the sheet runs the rest of `applied` (close,
    /// focus, reopen) through `cx.defer_in`, so the workspace is not
    /// entered while it draws (R-26; Wave 04 Cycle 05a's
    /// `run_waiting_add_close`).
    pub(super) fn run_waiting_resolve_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.content.resolve_waiting.is_none() || window.has_active_dialog(cx) {
            return;
        }
        let Some(ResolveWaiting { sheet, resolution }) = self.content.resolve_waiting.take() else {
            return;
        };
        if let Some(sheet) = sheet.upgrade() {
            sheet.update(cx, |_, cx| {
                cx.defer_in(window, move |sheet, window, cx| {
                    sheet.applied(resolution, window, cx)
                })
            });
        }
    }

    /// After each frame drawn while Resolve conflict is open, deferred from
    /// the workspace's render beside `keep_focus_in_sheet` (B3, R-11): with
    /// no dialog open, focus on a control drawn behind the sheet, a
    /// deferred restore's or a landing's, goes to the sheet's own handle.
    /// Focus inside the sheet, in a popup over it or in Root's overlays is
    /// left alone, as Settings' is; focus on nothing drawn is
    /// `focus_lost`'s.
    pub(super) fn keep_focus_in_resolve(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.resolve_open(window, cx) || window.has_active_dialog(cx) {
            return;
        }
        let Some(SheetOwner::Resolve(sheet)) = &self.content.sheet else {
            return;
        };
        let Some(sheet) = sheet.upgrade() else {
            return;
        };
        let behind = window
            .focused(cx)
            .is_some_and(|focused| self.drawn_behind(&focused, window, cx));
        if behind {
            let own = sheet.read(cx).own_focus().clone();
            window.focus(&own, cx);
            cx.notify();
        }
    }

    /// Resolve conflict closes quietly, if open, as quitting asks about
    /// edits no draft keeps: as Settings closes there, and as the dialog
    /// closed with the others; its choices go with it (O-10).
    pub(super) fn close_resolve_quietly(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.content.sheet, Some(SheetOwner::Resolve(_))) {
            return;
        }
        self.content.sheet = None;
        if window.has_active_sheet(cx) {
            window.close_sheet(cx);
        }
        cx.notify();
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resolve_sheet_tests.rs"]
mod tests;
