//! Settings as a sheet the workspace keeps (Wave 04 Cycle 04 design Part
//! A). Every way in calls `show_settings` with a route; the sheet goes on
//! Root's one sheet layer, in place of any details sheet, and the
//! workspace owns it (`SheetOwner::Settings`), so `sync_sheet` keeps it
//! whatever item is shown. The Kit closes it on Escape, its close button
//! and a click on the scrim; `sync_sheet` sees that and runs
//! `settings_closed` (R-7). Show in Activity and every quit path close it
//! quietly, putting focus where they always do (R-20, R-52).

use std::path::PathBuf;

use gpui_kit::{Context, FocusHandle, Window, component::WindowExt};

use super::{QuitPhase, Region, Workspace, content::SheetOwner, render::modal_open};
use crate::desktop::{model::stamp::SettingsPage, view::settings::SettingsRoute};

/// A copyable mirror of `SheetOwner`, without the pane, for tests.
#[cfg(all(test, feature = "desktop-test-support"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SheetOwnerKind {
    Details,
    Settings,
}

impl Workspace {
    /// Settings is open: it owns the sheet, and the sheet is active (R-36).
    pub(super) fn settings_open(&self, window: &mut Window, cx: &mut gpui_kit::App) -> bool {
        matches!(self.content.sheet, Some(SheetOwner::Settings)) && window.has_active_sheet(cx)
    }

    /// Opens Settings at `route` (Part D). `origin` is where focus returns
    /// on close; with none, the control focused now (A2, "The origin").
    /// Nothing opens under a dialog or while quitting (R-9, R-29). Open
    /// already, the route arrives in place and the origin is kept. A
    /// details sheet asked for, or open, gives way (R-51): its edits are
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
        let origin = origin.or_else(|| window.focused(cx));
        self.content
            .settings
            .update(cx, |settings, _| settings.set_origin(origin.as_ref()));
        self.content.sheet = Some(SheetOwner::Settings);
        let settings = self.content.settings.clone();
        window.open_sheet(cx, move |sheet, window, cx| {
            settings.update(cx, |settings, cx| settings.sheet(sheet, window, cx))
        });
        // After `open_sheet` has focused Root's own handle, as the details
        // sheet does.
        self.content.settings.update(cx, |settings, cx| {
            settings.arrive_focused(&route, window, cx)
        });
        cx.notify();
    }

    /// Settings closed (A3). A close the Kit did not make closes the Kit's
    /// sheet first. The host hides, which empties a typed passphrase (B3).
    /// Unless `quiet`, or a dialog or sheet is open by now, focus goes back
    /// to the origin, else the content region: the Kit's own close has
    /// already given it back there when the origin still exists.
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
            settings.hide(window, cx);
            settings.take_origin()
        });
        cx.notify();
        if quiet || modal_open(window, cx) {
            return;
        }
        match origin {
            Some(origin) => window.focus(&origin, cx),
            None => self.focus_region(Region::Content, window, cx),
        }
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
