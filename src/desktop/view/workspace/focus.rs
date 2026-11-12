//! The five focus groups, cycling, fallback and region decoration.

use super::Workspace;
use crate::desktop::model::{
    layout::{PanelKind, Stop, f6_stops},
    navigation::Pane,
};
use gpui_kit::{
    App, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Styled,
    TestSupportExt, Window,
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

    pub(super) fn of_stop(stop: Stop) -> Self {
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
            Self::Repositories => crate::desktop::keys::REPOSITORIES_CONTEXT,
            Self::Documents | Self::Tickets => crate::desktop::keys::NAVIGATION_CONTEXT,
            Self::Content => crate::desktop::keys::CONTENT_CONTEXT,
            Self::Operations => "Operations",
        }
    }
}

/// A button of a panel's toolbar (Part G): the focus handles the Workspace
/// owns for them, and the panel each belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::desktop::view) enum ToolbarButton {
    NewTicket,
    NewDocument,
    NewFolder,
    CollapseAll,
    AddRepository,
    CheckAll,
    RepositoriesMore,
}

impl ToolbarButton {
    pub(in crate::desktop::view) const ALL: [Self; 7] = [
        Self::NewTicket,
        Self::NewDocument,
        Self::NewFolder,
        Self::CollapseAll,
        Self::AddRepository,
        Self::CheckAll,
        Self::RepositoriesMore,
    ];

    /// The element's id: the body buttons' ids for the three New buttons.
    pub(in crate::desktop::view) fn id(self) -> &'static str {
        match self {
            Self::NewTicket => "new-ticket-button",
            Self::NewDocument => "new-document-button",
            Self::NewFolder => "new-folder-button",
            Self::CollapseAll => "collapse-all-button",
            Self::AddRepository => "add-repository-button",
            Self::CheckAll => "check-all-button",
            Self::RepositoriesMore => "repositories-more-button",
        }
    }

    pub(in crate::desktop::view) fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|button| *button == self)
            .expect("every button is listed")
    }

    /// The panel's region that holds focus while the button does
    /// (decision 18).
    pub(in crate::desktop::view) fn region(self) -> Region {
        match self {
            Self::NewTicket => Region::Tickets,
            Self::NewDocument | Self::NewFolder | Self::CollapseAll => Region::Documents,
            Self::AddRepository | Self::CheckAll | Self::RepositoriesMore => Region::Repositories,
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

    /// Focus in a region that is not displayed goes to the item area
    /// (R-9). It runs after every layout change and when focus enters a
    /// region, so a click on a closed strip outside its tabs, whose frame
    /// tracks the hidden panel's handle, does not leave focus there.
    pub(in crate::desktop::view) fn check_focus(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(region) = self.focused_region(window, cx)
            && !self.is_displayed(region)
        {
            self.focus_region(Region::Content, window, cx);
        }
    }

    /// The region that holds focus: the one whose handle contains it, or the
    /// one whose panel's toolbar button has it (decision 18, R-13), so the
    /// ring shows, F6 moves on from it and `NewItem` makes its kind.
    pub(super) fn focused_region(&self, window: &Window, cx: &App) -> Option<Region> {
        REGIONS
            .into_iter()
            .find(|region| self.regions[region.index()].contains_focused(window, cx))
            .or_else(|| {
                ToolbarButton::ALL
                    .into_iter()
                    .find(|button| self.toolbar_focus(*button).is_focused(window))
                    .map(ToolbarButton::region)
            })
    }

    /// The region whose handle holds `handle`, or whose toolbar button it
    /// is; the item area otherwise.
    pub(super) fn region_of(&self, handle: &FocusHandle, window: &Window) -> Region {
        REGIONS
            .into_iter()
            .find(|region| self.regions[region.index()].contains(handle, window))
            .or_else(|| {
                ToolbarButton::ALL
                    .into_iter()
                    .find(|button| self.toolbar_focus(*button) == handle)
                    .map(ToolbarButton::region)
            })
            .unwrap_or(Region::Content)
    }

    /// `focused_region`, for the keyboard walk (E-14).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(in crate::desktop::view) fn held_region(
        &self,
        window: &Window,
        cx: &App,
    ) -> Option<Region> {
        self.focused_region(window, cx)
    }

    pub(in crate::desktop::view) fn focus_region(
        &mut self,
        region: Region,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A panel that is hidden, in a closed dock or behind another tab is
        // revealed first (R-10); a displayed one is left as it is.
        if let Stop::Panel(kind) = region.stop() {
            self.reveal(kind, window, cx);
        }
        // The list used last (R-8).
        if let Some(pane) = region.pane() {
            self.navigation.set_pane(pane);
        }
        match region {
            // Whenever the panel draws a row, the kept one alone included,
            // so its menu is reached by keyboard (decision 22).
            Region::Repositories
                if self.navigation.kept().is_some()
                    || self
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
            // Outside every region: the first stop, or backwards the last.
            None if step < 0 => stops[stops.len() - 1],
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
