//! Settings as a sheet the workspace keeps (Wave 04 Cycle 04 design Part
//! A). Every way in calls `show_settings` with a route; the sheet goes on
//! Root's one sheet layer, in place of any details sheet, and the
//! workspace owns it (`SheetOwner::Settings`), so `sync_sheet` keeps it
//! whatever item is shown. The Kit closes it on Escape, its close button
//! and a click on the scrim, and its `on_close` runs `settings_closed` at
//! once (A3; Ruling 20); `sync_sheet` is the backstop for a direct
//! `close_sheet`. Show in Activity and every quit path close it quietly,
//! putting focus where they always do (R-20, R-52).
//!
//! Where focus goes is design A6's table; A7 lists what this must survive:
//! - **The origin** is the first of the route's own, the one
//!   `choose_activity` gives for its handler call (`settings_origin`), what
//!   an open Workspace layer returns to (that layer closes), and the focused
//!   control. It is focused before `open_sheet`, so the Kit's own record is
//!   the origin too (R-44), and its region is recorded while it is drawn.
//! - **The close** focuses the origin. Once it is no longer drawn, GPUI
//!   fires `focus_lost` after the next frame, which takes the armed
//!   `settings_return` and focuses the origin's region (R-45). The second
//!   render after the close disarms it.
//! - **The sheet keeps focus** (R-46): while Settings is open and no dialog
//!   is, focus found on a control the workspace drew behind the sheet in
//!   the frame just drawn goes to the shown page's entry, checked after
//!   each frame, and so does a lost focus with no target in the sheet. A
//!   control not drawn there is never taken for one behind (A7 item 1);
//!   Root's overlays are not the workspace's (A7 item 2).

use std::path::PathBuf;

use gpui_kit::{Context, FocusHandle, Window, component::WindowExt};

use super::{
    QuitPhase, Region, ToolbarButton, Workspace, content::SheetOwner, focus::REGIONS,
    render::modal_open,
};
use crate::desktop::{model::stamp::SettingsPage, view::settings::SettingsRoute};

/// A copyable mirror of `SheetOwner`, without the pane, for tests.
#[cfg(all(test, feature = "desktop-test-support"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SheetOwnerKind {
    Details,
    Settings,
}

/// Where a lost focus goes once Settings has closed onto a control that is
/// no longer drawn (A3; R-45): the region the control was in when Settings
/// opened, recorded then and armed by the close.
#[derive(Default)]
pub(super) struct SettingsReturn {
    /// The origin's region, `None` outside every region (the footer).
    region: Option<Region>,
    /// Armed by a close that focused the origin: the renders seen since.
    armed: Option<u8>,
}

impl SettingsReturn {
    /// Settings opened on an origin in `region`.
    fn recorded(region: Option<Region>) -> Self {
        Self {
            region,
            armed: None,
        }
    }

    /// Each render. A `focus_lost` for an origin no longer drawn fires after
    /// the frame of the first render after the close, which arms it outside
    /// a render (`on_close`) or within one (`sync_sheet`); the second render
    /// disarms it. No `on_next_frame`, which tests never run (A7 item 9).
    pub(super) fn tick(&mut self) {
        match self.armed {
            Some(seen) if seen >= 1 => *self = Self::default(),
            Some(seen) => self.armed = Some(seen + 1),
            None => {}
        }
    }

    /// The armed region, taken: `focus_lost` focuses it.
    pub(super) fn take(&mut self) -> Option<Region> {
        self.armed?;
        std::mem::take(self).region
    }
}

/// What a Workspace layer gives focus back to: where a menu opened by the
/// pointer found it, unless a choice was made, else where it was opened
/// from (A2, "The origin", case 3; A7 item 7).
fn returns_to(chosen: bool, restore: Option<FocusHandle>, context: FocusHandle) -> FocusHandle {
    restore.filter(|_| !chosen).unwrap_or(context)
}

impl Workspace {
    /// Settings is open: it owns the sheet, and the sheet is active (R-36).
    pub(super) fn settings_open(&self, window: &mut Window, cx: &mut gpui_kit::App) -> bool {
        matches!(self.content.sheet, Some(SheetOwner::Settings)) && window.has_active_sheet(cx)
    }

    /// Sets the origin a handler that opens Settings gives back to, for the
    /// length of one call (R-43), and gives the one it replaces.
    pub(in crate::desktop::view) fn set_settings_origin(
        &mut self,
        origin: Option<FocusHandle>,
    ) -> Option<FocusHandle> {
        std::mem::replace(&mut self.settings_origin, origin)
    }

    /// Opens Settings at `route` (Part D). `origin` is where focus returns
    /// on close; with none, A2's next ways to it (`settings_origin`, an open
    /// Workspace layer, the control focused now). Nothing opens under a
    /// dialog or while quitting (R-9, R-29). One the Kit closed without the
    /// workspace seeing it yet closes quietly first (A2 step 2; R-36). Open
    /// already, the route arrives in place and the origin is kept (step 3).
    /// A details sheet asked for, or open, gives way (R-51): its edits are
    /// already in the draft, and Root gives focus back to its chip (facts
    /// 28, 32, 34), which is then the origin. Drafts stays shown under the
    /// sheet (R-31).
    pub(in crate::desktop::view) fn show_settings(
        &mut self,
        route: SettingsRoute,
        origin: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || self.quit != QuitPhase::Running {
            return;
        }
        if matches!(self.content.sheet, Some(SheetOwner::Settings)) && !window.has_active_sheet(cx)
        {
            self.settings_closed(true, window, cx);
        }
        // The repository the route works on (A2 step 6): until one chosen
        // repository (C1), the selection today's entry functions set.
        let selection = match &route {
            SettingsRoute::Page(_) | SettingsRoute::Keys | SettingsRoute::AddRepository => {
                Some(self.navigation.selected().map(PathBuf::from))
            }
            SettingsRoute::Manage(root)
            | SettingsRoute::Remotes(root)
            | SettingsRoute::Hosts(root) => Some(Some(root.clone())),
            SettingsRoute::Inspect(_)
            | SettingsRoute::Identity(_)
            | SettingsRoute::CheckHost(..)
            | SettingsRoute::RetryGeneration(_) => None,
        };
        if let Some(selection) = selection {
            self.content
                .settings
                .update(cx, |settings, _| settings.set_selection(selection));
        }
        if self.settings_open(window, cx) {
            self.content.settings.update(cx, |settings, cx| {
                settings.arrive_focused(&route, window, cx)
            });
            return;
        }
        self.content.details = None;
        if window.has_active_sheet(cx) {
            self.content.sheet = None;
            window.close_sheet(cx);
        }
        let origin = origin
            .or_else(|| self.settings_origin.clone())
            .or_else(|| self.take_layer_origin(cx))
            .or_else(|| window.focused(cx));
        let region = origin
            .as_ref()
            .and_then(|origin| self.origin_region(origin, window));
        self.settings_return = SettingsReturn::recorded(region);
        if let Some(origin) = origin.as_ref().filter(|origin| !origin.is_focused(window)) {
            window.focus(origin, cx);
        }
        self.content.settings.update(cx, |settings, _| {
            settings.set_origin(origin.as_ref());
            settings.set_open(true);
        });
        self.content.sheet = Some(SheetOwner::Settings);
        let settings = self.content.settings.clone();
        let workspace = cx.entity().downgrade();
        window.open_sheet(cx, move |sheet, window, cx| {
            // Escape, the close button and the scrim: the Kit has closed
            // the sheet, and Settings closes now (A3; A7 item 10).
            let workspace = workspace.clone();
            let sheet = sheet.on_close(move |_, window, cx| {
                let _ = workspace.update(cx, |workspace, cx| {
                    if matches!(workspace.content.sheet, Some(SheetOwner::Settings)) {
                        workspace.settings_closed(false, window, cx);
                    }
                });
            });
            settings.update(cx, |settings, cx| settings.sheet(sheet, window, cx))
        });
        // After `open_sheet` has focused Root's own handle, as the details
        // sheet does.
        self.content.settings.update(cx, |settings, cx| {
            settings.arrive_focused(&route, window, cx)
        });
        cx.notify();
    }

    /// The control the Workspace layer open now gives focus back to, and
    /// that layer closed (A2, "The origin", case 3; Ruling 8): the Needs
    /// attention popover's footer item, else E1's control; a row menu's,
    /// a repository menu's or a dock menu's `restore`, else the row's list,
    /// the button or the toggle it opened from; the ⋯ menu's button.
    fn take_layer_origin(&mut self, cx: &mut Context<Self>) -> Option<FocusHandle> {
        if self.attention.popover.take().is_some() {
            return Some(self.attention_restore_target(cx));
        }
        if let Some(menu) = self.activity.menu.take() {
            return Some(returns_to(menu.chosen, menu.restore, menu.context));
        }
        if let Some(more) = self.activity.more.take() {
            return Some(more.button);
        }
        if let Some(menu) = self.repository_menu.take() {
            return Some(returns_to(menu.chosen, menu.restore, menu.context));
        }
        if let Some(menu) = self.footer.menu.take() {
            return Some(returns_to(menu.chosen, menu.restore, menu.toggle));
        }
        None
    }

    /// The region `handle` is in, in the frame drawn last, or whose toolbar
    /// button it is; `None` outside every region, as the footer's controls
    /// are (A7 item 8: `region_of` would say Content).
    fn origin_region(&self, handle: &FocusHandle, window: &Window) -> Option<Region> {
        REGIONS
            .into_iter()
            .find(|region| self.regions[region.index()].contains(handle, window))
            .or_else(|| {
                ToolbarButton::ALL
                    .into_iter()
                    .find(|button| self.toolbar_focus(*button) == handle)
                    .map(ToolbarButton::region)
            })
    }

    /// Settings closed (A3). A close the Kit did not make closes the Kit's
    /// sheet first, before any focus moves or dialog opens (A7 item 5). The
    /// host hides, which empties a typed passphrase (B3) and an Add waiting
    /// for its check (R-48), and its pages' landings stop (A4). Unless
    /// `quiet`, or a dialog or sheet is open by now, focus goes back to the
    /// origin, and `settings_return` is armed for an origin no longer drawn;
    /// with the origin gone, its region; with none, the content region.
    pub(super) fn settings_closed(
        &mut self,
        quiet: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let owned = matches!(self.content.sheet, Some(SheetOwner::Settings));
        if owned && window.has_active_sheet(cx) {
            window.close_sheet(cx);
        }
        if owned {
            self.content.sheet = None;
        }
        let origin = self.content.settings.update(cx, |settings, cx| {
            settings.set_open(false);
            settings.hide(window, cx);
            settings.take_origin()
        });
        let region = std::mem::take(&mut self.settings_return).region;
        cx.notify();
        if quiet || modal_open(window, cx) {
            return;
        }
        match origin {
            Some(origin) => {
                window.focus(&origin, cx);
                self.settings_return = SettingsReturn {
                    region,
                    armed: Some(0),
                };
            }
            None => self.focus_region(region.unwrap_or(Region::Content), window, cx),
        }
    }

    /// The shown page's entry in the menu, focused.
    fn focus_settings_entry(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.content.settings.read(cx).first_focus() {
            window.focus(&entry, cx);
        }
    }

    /// Whether `focused` is a control the workspace drew behind the sheet
    /// in the frame drawn last: in a region, the item area outside the
    /// sheet's layer included, a panel's toolbar or the footer. A control
    /// not drawn yet is not (A7 item 1), nor is one in Root's own overlays,
    /// such as an input's context menu (A7 item 2).
    fn drawn_behind(&self, focused: &FocusHandle, window: &Window, cx: &gpui_kit::App) -> bool {
        let layer = self.content.settings.read(cx).layer();
        !layer.contains(focused, window)
            && (REGIONS
                .into_iter()
                .any(|region| self.regions[region.index()].contains(focused, window))
                || ToolbarButton::ALL
                    .into_iter()
                    .any(|button| self.toolbar_focus(button) == focused)
                || self.footer.focus.values().any(|handle| handle == focused))
    }

    /// After each frame drawn while Settings is open, deferred from the
    /// workspace's render (A3, "The sheet keeps focus"; R-46): while Settings
    /// is open and no dialog is, focus on a control drawn behind the sheet,
    /// a deferred restore's or a landing's, also one a view behind the sheet
    /// moved while it drew (A6 F16; A7 item 4), goes to the shown page's
    /// entry. It reads the frame just drawn, and moves only focus it finds
    /// there behind the sheet, so focus not drawn there is left alone (A7
    /// item 1), and so is focus in a Kit popup inside the sheet (A7 item 3)
    /// or in Root's overlays (F19; A7 item 2). Focus on nothing drawn is
    /// `focus_lost`'s. Focus it moves is inside the sheet at the next check,
    /// so it moves once.
    pub(super) fn keep_focus_in_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settings_open(window, cx) || window.has_active_dialog(cx) {
            return;
        }
        let behind = window
            .focused(cx)
            .is_some_and(|focused| self.drawn_behind(&focused, window, cx));
        if behind {
            self.focus_settings_entry(window, cx);
        }
    }

    /// A lost focus while Settings is open and no dialog is (A6 F15; A7
    /// item 6): its target when the sheet holds it, else the shown page's
    /// entry, also when there is no target. True when it applied.
    pub(super) fn settings_keeps_lost_focus(
        &mut self,
        target: Option<&FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.settings_open(window, cx) || window.has_active_dialog(cx) {
            return false;
        }
        let layer = self.content.settings.read(cx).layer().clone();
        match target.filter(|target| layer.contains(target, window)) {
            Some(target) => window.focus(target, cx),
            None => self.focus_settings_entry(window, cx),
        }
        true
    }

    /// Settings closes quietly, if it is open: every quit path, after it
    /// closes the dialogs and before it asks or waits (R-52).
    pub(super) fn close_settings_quietly(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.content.sheet, Some(SheetOwner::Settings))
            || self.content.settings.read(cx).is_shown()
        {
            self.settings_closed(true, window, cx);
        }
    }

    /// Ctrl+, (A5; R-8): nothing under a dialog or over Settings; over the
    /// details sheet, Settings opens in its place; otherwise Repositories.
    /// The one key that acts under a sheet (contract 7).
    pub(super) fn settings_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) || self.settings_open(window, cx) {
            return;
        }
        self.show_settings(
            SettingsRoute::Page(SettingsPage::Repositories),
            None,
            window,
            cx,
        );
    }

    /// Show in Activity (C3; R-20): Settings closes quietly, then Activity is
    /// revealed and its list focused.
    pub(super) fn settings_show_activity(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_closed(true, window, cx);
        self.focus_region(Region::Operations, window, cx);
    }

    /// Who keeps the sheet open.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn sheet_owner(&self) -> Option<SheetOwnerKind> {
        self.content.sheet.as_ref().map(|owner| match owner {
            SheetOwner::Details(_) => SheetOwnerKind::Details,
            SheetOwner::Settings => SheetOwnerKind::Settings,
        })
    }

    /// The repository Settings works on.
    #[cfg(all(test, feature = "desktop-test-support"))]
    #[allow(dead_code)] // Used from Task 3
    pub(crate) fn settings_chosen(&self, cx: &gpui_kit::App) -> Option<PathBuf> {
        self.content.settings.read(cx).selection().cloned()
    }

    /// Settings' own scroll offset.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn settings_offset(&self, cx: &gpui_kit::App) -> gpui_kit::Pixels {
        self.content.settings.read(cx).scroll_offset()
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "settings_sheet_tests.rs"]
mod tests;
