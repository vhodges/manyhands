//! The five focus groups, cycling, fallback and region decoration.

use super::Workspace;
use crate::desktop::model::{
    layout::{PanelKind, Stop, f6_stops},
    navigation::Pane,
};
use gpui_kit::{
    App, Context, InteractiveElement, IntoElement, ParentElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, WindowExt},
    div,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Region {
    Repositories,
    Documents,
    Content,
    Tickets,
    Operations,
}

/// In F6's order (decision 9).
pub(crate) const REGIONS: [Region; 5] = [
    Region::Repositories,
    Region::Documents,
    Region::Content,
    Region::Tickets,
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
            Self::Documents => "region-documents",
            Self::Content => "region-content",
            Self::Tickets => "region-tickets",
            Self::Operations => "region-operations",
        }
    }

    pub(crate) fn ring_id(self) -> &'static str {
        match self {
            Self::Repositories => "region-repositories-focus-ring",
            Self::Documents => "region-documents-focus-ring",
            Self::Content => "region-content-focus-ring",
            Self::Tickets => "region-tickets-focus-ring",
            Self::Operations => "region-operations-focus-ring",
        }
    }

    /// The region of the list `pane`.
    pub(in crate::desktop::view) fn of_pane(pane: Pane) -> Self {
        match pane {
            Pane::Documents => Self::Documents,
            Pane::Tickets => Self::Tickets,
        }
    }

    /// The list this region holds, when it holds one.
    pub(in crate::desktop::view) fn pane(self) -> Option<Pane> {
        match self {
            Self::Documents => Some(Pane::Documents),
            Self::Tickets => Some(Pane::Tickets),
            Self::Repositories | Self::Content | Self::Operations => None,
        }
    }

    /// The F6 stop the region is.
    fn stop(self) -> Stop {
        match self {
            Self::Repositories => Stop::Panel(PanelKind::Repositories),
            Self::Documents => Stop::Panel(PanelKind::Documents),
            Self::Tickets => Stop::Panel(PanelKind::Tickets),
            Self::Operations => Stop::Panel(PanelKind::Activity),
            Self::Content => Stop::ItemArea,
        }
    }

    fn of_stop(stop: Stop) -> Self {
        match stop {
            Stop::Panel(PanelKind::Repositories) => Self::Repositories,
            Stop::Panel(PanelKind::Documents) => Self::Documents,
            Stop::Panel(PanelKind::Tickets) => Self::Tickets,
            Stop::Panel(PanelKind::Activity) => Self::Operations,
            Stop::ItemArea => Self::Content,
        }
    }

    fn key_context(self) -> &'static str {
        match self {
            Self::Repositories => "Repositories",
            Self::Documents | Self::Tickets => crate::desktop::keys::NAVIGATION_CONTEXT,
            Self::Content => crate::desktop::keys::CONTENT_CONTEXT,
            Self::Operations => "Operations",
        }
    }
}

impl Workspace {
    /// Whether the region is on screen: the item area always is; a panel is
    /// when the layout displays it (R-9).
    fn is_displayed(&self, region: Region) -> bool {
        match region.stop() {
            Stop::ItemArea => true,
            Stop::Panel(kind) => self.layout.is_displayed(kind),
        }
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
        // The list used last (R-8).
        if let Some(pane) = region.pane() {
            self.navigation.set_pane(pane);
        }
        match region {
            Region::Repositories
                if self
                    .navigation
                    .repositories()
                    .rows()
                    .is_some_and(|rows| !rows.is_empty()) =>
            {
                self.repositories_pane.focus(window, cx)
            }
            Region::Documents if self.documents.repo().is_some() => {
                self.documents_pane.focus(window, cx)
            }
            Region::Tickets if self.tickets.repo().is_some() => self.tickets_pane.focus(window, cx),
            Region::Content => self.focus_content(window, cx),
            _ => window.focus(&self.regions[region.index()], cx),
        }
        self.last_region = Some(region);
        cx.notify();
    }

    /// A control disappeared: restore to the nearest remaining region or the
    /// last one. Inside an open dialog or the details sheet, focus goes to
    /// the nearest focusable that held the control (the dialog's or the
    /// sheet's own), so it stays in the modal's trap: a control that goes
    /// by itself, as Stop when a run ends, leaves Tab, Enter and Escape
    /// working (A21).
    pub(super) fn focus_lost(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let target = window.focus_lost_restore_target(cx);
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            if let Some(target) = target {
                window.focus(&target, cx);
            }
            return;
        }
        // A region that is not displayed cannot take focus: the last
        // region, else the item area (R-9).
        let region = target
            .and_then(|target| {
                REGIONS
                    .into_iter()
                    .filter(|region| self.is_displayed(*region))
                    .find(|region| self.regions[region.index()].contains(&target, window))
            })
            .or(self.last_region.filter(|region| self.is_displayed(*region)))
            .unwrap_or(Region::Content);
        self.focus_region(region, window, cx);
    }

    pub(super) fn cycle_region(
        &mut self,
        step: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let stops: Vec<Region> = f6_stops(&self.layout)
            .into_iter()
            .map(Region::of_stop)
            .collect();
        let next = match self
            .focused_region(window, cx)
            .and_then(|region| stops.iter().position(|stop| *stop == region))
        {
            Some(at) => stops[(at as isize + step).rem_euclid(stops.len() as isize) as usize],
            None => stops[0],
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
