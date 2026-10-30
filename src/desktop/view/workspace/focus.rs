//! The four focus groups, cycling, fallback and region decoration.

use super::Workspace;
use crate::desktop::model::navigation::Pane;
use gpui_kit::{
    App, Context, InteractiveElement, IntoElement, ParentElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, WindowExt},
    div,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Region {
    Repositories,
    Navigation,
    Content,
    Operations,
}

pub(crate) const REGIONS: [Region; 4] = [
    Region::Repositories,
    Region::Navigation,
    Region::Content,
    Region::Operations,
];

impl Region {
    pub(in crate::desktop::view) fn index(self) -> usize {
        REGIONS
            .iter()
            .position(|region| *region == self)
            .expect("every region is listed")
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Repositories => "region-repositories",
            Self::Navigation => "region-navigation",
            Self::Content => "region-content",
            Self::Operations => "region-operations",
        }
    }

    pub(crate) fn ring_id(self) -> &'static str {
        match self {
            Self::Repositories => "region-repositories-focus-ring",
            Self::Navigation => "region-navigation-focus-ring",
            Self::Content => "region-content-focus-ring",
            Self::Operations => "region-operations-focus-ring",
        }
    }

    fn key_context(self) -> &'static str {
        match self {
            Self::Repositories => "Repositories",
            Self::Navigation => crate::desktop::keys::NAVIGATION_CONTEXT,
            Self::Content => crate::desktop::keys::CONTENT_CONTEXT,
            Self::Operations => "Operations",
        }
    }
}

impl Workspace {
    pub(in crate::desktop::view) fn show_pane(
        &mut self,
        pane: Pane,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.navigation.set_pane(pane);
        if self.focused_region(window, cx) == Some(Region::Navigation) {
            self.focus_region(Region::Navigation, window, cx);
        }
        cx.notify();
    }

    pub(super) fn focused_region(&self, window: &Window, cx: &App) -> Option<Region> {
        REGIONS
            .into_iter()
            .find(|region| self.regions[region.index()].contains_focused(window, cx))
    }

    pub(in crate::desktop::view) fn focus_region(
        &mut self,
        region: Region,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match (region, self.navigation.pane()) {
            (Region::Repositories, _)
                if self
                    .navigation
                    .repositories()
                    .rows()
                    .is_some_and(|rows| !rows.is_empty()) =>
            {
                self.repositories_pane.focus(window, cx)
            }
            (Region::Navigation, Pane::Documents) if self.documents.repo().is_some() => {
                self.documents_pane.focus(window, cx)
            }
            (Region::Navigation, Pane::Tickets) if self.tickets.repo().is_some() => {
                self.tickets_pane.focus(window, cx)
            }
            (Region::Content, _) => self.focus_content(window, cx),
            _ => window.focus(&self.regions[region.index()], cx),
        }
        self.last_region = Some(region);
        cx.notify();
    }

    /// A control disappeared: restore to the nearest remaining region or the
    /// last one, but let an open dialog retain its own focus. Inside the
    /// details sheet, focus goes to the nearest focusable that holds the
    /// control, which the sheet's own host does, so it stays in the sheet's
    /// trap.
    pub(super) fn focus_lost(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let target = window.focus_lost_restore_target(cx);
        if window.has_active_sheet(cx) {
            if let Some(target) = target {
                window.focus(&target, cx);
            }
            return;
        }
        let region = target
            .and_then(|target| {
                REGIONS
                    .into_iter()
                    .find(|region| self.regions[region.index()].contains(&target, window))
            })
            .or(self.last_region);
        if let Some(region) = region {
            self.focus_region(region, window, cx);
        }
    }

    pub(super) fn cycle_region(
        &mut self,
        step: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let next = match self.focused_region(window, cx) {
            Some(region) => {
                let count = REGIONS.len() as isize;
                REGIONS[(region.index() as isize + step).rem_euclid(count) as usize]
            }
            None => Region::Repositories,
        };
        self.focus_region(next, window, cx);
    }

    pub(super) fn region<C: IntoElement>(
        &self,
        region: Region,
        window: &Window,
        cx: &App,
        content: C,
    ) -> impl IntoElement + Styled + use<C> {
        let focused = self.focused_region(window, cx) == Some(region);
        let theme = cx.theme();
        let ring = focused.then(|| {
            div()
                .id(region.ring_id())
                .test_support()
                .absolute()
                .inset_0()
                .border_2()
                .border_color(theme.ring)
        });
        div()
            .id(region.id())
            .test_support()
            .track_focus(&self.regions[region.index()])
            .key_context(region.key_context())
            .relative()
            .p_2()
            .border_1()
            .border_color(theme.border)
            .child(content)
            // Over the content: an item tab lies flush with the border.
            .children(ring)
    }
}
