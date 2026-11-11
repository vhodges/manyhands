//! The panels of the dock area: thin entities that draw the Workspace's
//! regions (R-1).
//!
//! A panel holds a weak handle on the Workspace, which keeps every body and
//! all state. `render` updates the Workspace to build its region: GPUI
//! renders a child view after its parent's render has returned, so the
//! Workspace is not leased then (fact 12). `visible` and `focus_handle`
//! read only the panel's own fields, because the Kit calls them inside its
//! own updates (facts 2 and 5), where reading the Workspace could panic.

use gpui_kit::{
    App, AppContext as _, Context, Empty, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, Render, SharedString, Subscription, WeakEntity, Window,
    component::dock::{BasePanel, Panel, PanelControl, PanelEvent},
};

use crate::desktop::{
    model::{
        documents::document_count,
        layout::{ITEM_AREA_NAME, PanelKind},
    },
    view::workspace::{Region, Workspace},
};

/// What a header says in place of a count while a list reloads over the
/// rows it kept (R-36).
pub(crate) const UPDATING: &str = "Updating\u{2026}";

/// What a panel draws.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Slot {
    Panel(PanelKind),
    /// The centre of the area: the content region.
    ItemArea,
}

impl Slot {
    /// The five, in the order the shell lists its regions.
    pub(crate) const ALL: [Slot; 5] = [
        Slot::Panel(PanelKind::Repositories),
        Slot::Panel(PanelKind::Documents),
        Slot::ItemArea,
        Slot::Panel(PanelKind::Tickets),
        Slot::Panel(PanelKind::Activity),
    ];

    /// The name the registry and a saved layout know the panel by.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Panel(kind) => kind.name(),
            Self::ItemArea => ITEM_AREA_NAME,
        }
    }

    /// The region the panel draws; Activity draws the operation area.
    pub(crate) fn region(self) -> Region {
        match self {
            Self::Panel(PanelKind::Repositories) => Region::Repositories,
            Self::Panel(PanelKind::Documents) => Region::Documents,
            Self::Panel(PanelKind::Tickets) => Region::Tickets,
            Self::Panel(PanelKind::Activity) => Region::Operations,
            Self::ItemArea => Region::Content,
        }
    }
}

pub(crate) struct RegionPanel {
    workspace: WeakEntity<Workspace>,
    slot: Slot,
    /// Whether the person wants the panel; a panel that is off keeps its
    /// place and state in the area (R-27).
    visible: bool,
    /// The Workspace's handle for the region, cloned.
    focus: FocusHandle,
    /// Redraws the panel whenever the Workspace changes, whatever caching
    /// the dock applies.
    _workspace_changed: Subscription,
}

impl RegionPanel {
    pub(super) fn new(
        workspace: &Entity<Workspace>,
        slot: Slot,
        focus: FocusHandle,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            workspace: workspace.downgrade(),
            slot,
            visible: true,
            focus,
            _workspace_changed: cx.observe(workspace, |_, _, cx| cx.notify()),
        })
    }

    /// Turns the panel off or on; its slot gives its space to the rest of
    /// its dock while it is off (fact 8).
    pub(crate) fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
}

impl BasePanel for RegionPanel {
    fn panel_name(&self) -> &'static str {
        self.slot.name()
    }

    fn visible(&self, _: &App) -> bool {
        self.visible
    }

    fn closable(&self, _: &App) -> bool {
        false
    }

    fn zoomable(&self, _: &App) -> bool {
        false
    }
}

impl Panel for RegionPanel {
    fn tab_name(&self, _: &App) -> Option<SharedString> {
        match self.slot {
            Slot::Panel(kind) => Some(kind.short_label().into()),
            Slot::ItemArea => None,
        }
    }

    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        match self.slot {
            Slot::Panel(kind) => kind.label(),
            Slot::ItemArea => "",
        }
    }

    fn zoom_control(&self, _: &App) -> Option<PanelControl> {
        None
    }

    /// The item area draws its own tab strip.
    fn title_bar(&self, _: &App) -> bool {
        self.slot != Slot::ItemArea
    }
}

/// What a panel's header says beside its title: how many rows its list
/// has, or that the list is updating.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Count {
    Rows(usize),
    Updating,
}

impl Count {
    /// The count as the header draws it, and as its accessible name.
    pub(super) fn text(self) -> String {
        match self {
            Self::Rows(rows) => rows.to_string(),
            Self::Updating => UPDATING.to_owned(),
        }
    }
}

/// The count the header shows (R-22, R-11, R-36): repositories listed,
/// tickets listed under the filters (nonconforming ones included),
/// documents listed. While a list reloads over the rows it kept, it says
/// "Updating…"; on a first load, and for Activity, there is none.
pub(super) fn count(workspace: &Workspace, kind: PanelKind) -> Option<Count> {
    match kind {
        PanelKind::Repositories => workspace
            .navigation()
            .repositories()
            .rows()
            .map(|rows| Count::Rows(rows.len())),
        PanelKind::Tickets => {
            let tickets = workspace.tickets();
            if tickets.list().reloading() {
                return Some(Count::Updating);
            }
            tickets
                .state()
                .rows()
                .map(|_| Count::Rows(tickets.view().listed))
        }
        PanelKind::Documents => workspace
            .documents()
            .state()
            .rows()
            .map(|rows| Count::Rows(document_count(rows))),
        PanelKind::Activity => None,
    }
}

impl EventEmitter<PanelEvent> for RegionPanel {}

impl Focusable for RegionPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for RegionPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let slot = self.slot;
        match self.workspace.upgrade() {
            Some(workspace) => workspace
                .update(cx, |workspace, cx| workspace.region_body(slot, window, cx))
                .into_any_element(),
            None => Empty.into_any_element(),
        }
    }
}
