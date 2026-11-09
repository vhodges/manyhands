//! The shell's layout: which panels are on, which docks are shown, and
//! how the panels are arranged in their docks.
//!
//! Pure state and rules (Wave 04 Cycle 01, Part B). The Workspace owns a
//! [`Layout`], applies the operations here, and reconciles the dock area
//! with what the derived queries say. The only Kit types read are the
//! plain data of a dumped dock area (`arrangement_from`).
//!
//! - A panel is **on** when the person wants it.
//! - A dock is **shown** unless the person hid it.
//! - A dock is **open** when it is shown and holds a panel that is on.
//! - A panel is **displayed** when its dock is open, it is on, and it is
//!   its group's displayed panel: the group's active one, or, when that
//!   is off, the group's first panel that is on (the Kit's own fallback).

use gpui_kit::component::dock::{DockAreaState, DockState, PanelState};

mod restore;

#[cfg(test)]
pub(crate) use restore::state_for;
pub(crate) use restore::{LAYOUT_VERSION, Restore, encode, restore};

/// The stable name of the item area's panel.
pub(crate) const ITEM_AREA_NAME: &str = "ItemAreaPanel";

/// The left dock's two groups, Repositories over Documents.
pub(crate) const DEFAULT_LEFT_SPLIT: [f32; 2] = [0.6, 1.4];

/// The Kit's names for a dock's containers, as written in a dump.
const TAB_GROUP: &str = "TabPanel";
const SPLIT: &str = "StackPanel";

/// The panels the shell docks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PanelKind {
    Repositories,
    Tickets,
    Documents,
    Activity,
}

impl PanelKind {
    pub(crate) const ALL: [PanelKind; 4] = [
        PanelKind::Repositories,
        PanelKind::Tickets,
        PanelKind::Documents,
        PanelKind::Activity,
    ];

    /// The stable name the dock registry and a saved layout know it by.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Repositories => "RepositoriesPanel",
            Self::Tickets => "TicketsPanel",
            Self::Documents => "DocumentsPanel",
            Self::Activity => "ActivityPanel",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Repositories => "Repositories",
            Self::Tickets => "Tickets",
            Self::Documents => "Documents",
            Self::Activity => "Activity",
        }
    }

    /// For a group of more than two tabs (R-32).
    pub(crate) fn short_label(self) -> &'static str {
        match self {
            Self::Repositories => "Repos",
            Self::Tickets => "Tickets",
            Self::Documents => "Docs",
            Self::Activity => "Activity",
        }
    }

    /// The icon's name (R-23).
    pub(crate) fn icon(self) -> &'static str {
        match self {
            Self::Repositories => "folder-git-2",
            Self::Tickets => "ticket",
            Self::Documents => "file-text",
            Self::Activity => "activity",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Where a dock sits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Left,
    Right,
    Bottom,
}

impl Side {
    pub(crate) const ALL: [Side; 3] = [Side::Left, Side::Right, Side::Bottom];

    fn index(self) -> usize {
        self as usize
    }
}

/// A tab group: its panels in order and the active one's index.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Group {
    pub(crate) panels: Vec<PanelKind>,
    pub(crate) active: usize,
}

impl Group {
    /// The selected tab, whether or not it is on.
    pub(crate) fn active_panel(&self) -> Option<PanelKind> {
        self.panels.get(self.active).copied()
    }
}

/// Each side's groups in order: top to bottom in a side dock, left to
/// right in the bottom dock.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct Arrangement {
    sides: [Vec<Group>; 3],
}

impl Arrangement {
    pub(crate) fn groups(&self, side: Side) -> &[Group] {
        &self.sides[side.index()]
    }

    fn push_group(&mut self, side: Side, panels: Vec<PanelKind>, active: usize) {
        if !panels.is_empty() {
            let active = active.min(panels.len() - 1);
            self.sides[side.index()].push(Group { panels, active });
        }
    }

    fn side_of(&self, kind: PanelKind) -> Option<Side> {
        Side::ALL
            .into_iter()
            .find(|side| self.groups(*side).iter().any(|g| g.panels.contains(&kind)))
    }

    fn group_of_mut(&mut self, kind: PanelKind) -> Option<(Side, &mut Group)> {
        let side = self.side_of(kind)?;
        let group = self.sides[side.index()]
            .iter_mut()
            .find(|g| g.panels.contains(&kind))?;
        Some((side, group))
    }

    /// Takes `kind` out of its group and puts it at the end of `side` as a
    /// new group (R-5). The group keeps the same panel active, unless `kind`
    /// was the active one: then the tab that took its place is.
    fn move_panel(&mut self, kind: PanelKind, side: Side) {
        if let Some((from, _)) = self.group_of_mut(kind) {
            let groups = &mut self.sides[from.index()];
            let at = groups
                .iter()
                .position(|g| g.panels.contains(&kind))
                .expect("the group was just found");
            let group = &mut groups[at];
            let tab = group.panels.iter().position(|p| *p == kind).unwrap();
            group.panels.remove(tab);
            if group.panels.is_empty() {
                groups.remove(at);
            } else {
                if tab < group.active {
                    group.active -= 1;
                }
                group.active = group.active.min(group.panels.len() - 1);
            }
        }
        self.push_group(side, vec![kind], 0);
    }
}

/// An F6 stop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Stop {
    Panel(PanelKind),
    ItemArea,
}

/// The layout the person sees.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Layout {
    on: [bool; 4],
    shown: [bool; 3],
    arrangement: Arrangement,
    /// Each dock's size in pixels, the last known one for a dock with no
    /// panels.
    sizes: [f32; 3],
}

/// Wave decision 1's 1b: left Repositories over Documents, right Tickets,
/// bottom Activity, all on and shown, sized 290, 440 and 150 px (decision
/// 15).
impl Default for Layout {
    fn default() -> Self {
        use PanelKind::*;
        let mut arrangement = Arrangement::default();
        arrangement.push_group(Side::Left, vec![Repositories], 0);
        arrangement.push_group(Side::Left, vec![Documents], 0);
        arrangement.push_group(Side::Right, vec![Tickets], 0);
        arrangement.push_group(Side::Bottom, vec![Activity], 0);
        Self {
            on: [true; 4],
            shown: [true; 3],
            arrangement,
            sizes: [290., 440., 150.],
        }
    }
}

impl Layout {
    pub(crate) fn is_on(&self, kind: PanelKind) -> bool {
        self.on[kind.index()]
    }

    pub(crate) fn is_shown(&self, side: Side) -> bool {
        self.shown[side.index()]
    }

    pub(crate) fn size(&self, side: Side) -> f32 {
        self.sizes[side.index()]
    }

    /// Remembers the size of `side`'s dock, for a dock that is built again.
    pub(crate) fn set_size(&mut self, side: Side, size: f32) {
        self.sizes[side.index()] = size;
    }

    pub(crate) fn groups(&self, side: Side) -> &[Group] {
        self.arrangement.groups(side)
    }

    /// The side `kind` is docked on.
    pub(crate) fn side_of(&self, kind: PanelKind) -> Option<Side> {
        self.arrangement.side_of(kind)
    }

    /// Whether `side` holds any panel, on or off. A side with none has no
    /// dock and no toggle (decision 6).
    pub(crate) fn has_panels(&self, side: Side) -> bool {
        !self.groups(side).is_empty()
    }

    /// Shown, and holding a panel that is on.
    pub(crate) fn is_open(&self, side: Side) -> bool {
        self.is_shown(side)
            && self
                .groups(side)
                .iter()
                .any(|g| g.panels.iter().any(|p| self.is_on(*p)))
    }

    /// The panel `group` draws: its active one, else its first that is on.
    pub(crate) fn displayed_in(&self, group: &Group) -> Option<PanelKind> {
        group
            .active_panel()
            .filter(|p| self.is_on(*p))
            .or_else(|| group.panels.iter().copied().find(|p| self.is_on(*p)))
    }

    pub(crate) fn is_displayed(&self, kind: PanelKind) -> bool {
        self.side_of(kind).is_some_and(|side| {
            self.is_open(side)
                && self
                    .groups(side)
                    .iter()
                    .any(|g| self.displayed_in(g) == Some(kind))
        })
    }
}

impl Layout {
    /// A panel toggle: a displayed panel goes off, and the next that is on
    /// in its group becomes active; any other panel is revealed.
    pub(crate) fn toggle_panel(&mut self, kind: PanelKind) {
        if !self.is_displayed(kind) {
            return self.reveal(kind);
        }
        let on = self.on;
        let displayed = self
            .groups(self.side_of(kind).unwrap())
            .iter()
            .position(|g| self.displayed_in(g) == Some(kind))
            .unwrap();
        self.on[kind.index()] = false;
        let (side, _) = self.arrangement.group_of_mut(kind).unwrap();
        let group = &mut self.arrangement.sides[side.index()][displayed];
        let len = group.panels.len();
        let from = group.panels.iter().position(|p| *p == kind).unwrap();
        if let Some(next) = (1..len)
            .map(|step| (from + step) % len)
            .find(|ix| on[group.panels[*ix].index()])
        {
            group.active = next;
        }
    }

    /// A dock toggle. An open dock is hidden, whatever is on. Any other
    /// dock is shown, and if none of its panels is on, they all go on
    /// (decision 6). A side with no panels has no toggle.
    pub(crate) fn toggle_dock(&mut self, side: Side) {
        if !self.has_panels(side) {
            return;
        }
        if self.is_open(side) {
            self.shown[side.index()] = false;
            return;
        }
        self.shown[side.index()] = true;
        if !self.is_open(side) {
            let panels: Vec<_> = self
                .groups(side)
                .iter()
                .flat_map(|g| g.panels.clone())
                .collect();
            for kind in panels {
                self.on[kind.index()] = true;
            }
        }
    }

    /// The panel goes on, its dock is shown and its tab is selected.
    pub(crate) fn reveal(&mut self, kind: PanelKind) {
        self.on[kind.index()] = true;
        if let Some((side, group)) = self.arrangement.group_of_mut(kind) {
            group.active = group.panels.iter().position(|p| *p == kind).unwrap();
            self.shown[side.index()] = true;
        }
    }

    /// The dock menu: the panel moves to the end of `side` as a new group
    /// and is revealed (R-5). Its current side changes nothing.
    // The pure model of a menu move: `dock_panel` reads the arrangement back
    // from the area, and a test holds the two together (Task 5).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn move_to(&mut self, kind: PanelKind, side: Side) {
        if self.side_of(kind) == Some(side) {
            return;
        }
        let mut arrangement = self.arrangement.clone();
        arrangement.move_panel(kind, side);
        self.set_arrangement(arrangement);
    }

    /// The arrangement the dock area reports. A panel whose side changed
    /// is revealed (R-36).
    pub(crate) fn set_arrangement(&mut self, arrangement: Arrangement) {
        let moved: Vec<_> = PanelKind::ALL
            .into_iter()
            .filter(|kind| {
                let (was, now) = (self.side_of(*kind), arrangement.side_of(*kind));
                was.is_some() && now.is_some() && was != now
            })
            .collect();
        self.arrangement = arrangement;
        for kind in moved {
            self.reveal(kind);
        }
    }
}

/// F6's stops (decision 9): the left dock's displayed panels top to
/// bottom, the item area, the right dock's, then the bottom dock's from
/// left to right. Only a tabbed group's displayed tab is a stop.
pub(crate) fn f6_stops(layout: &Layout) -> Vec<Stop> {
    let displayed = |side: Side| {
        layout
            .groups(side)
            .iter()
            .filter(|_| layout.is_open(side))
            .filter_map(|group| layout.displayed_in(group))
            .map(Stop::Panel)
            .collect::<Vec<_>>()
    };
    let mut stops = displayed(Side::Left);
    stops.push(Stop::ItemArea);
    stops.extend(displayed(Side::Right));
    stops.extend(displayed(Side::Bottom));
    stops
}

/// Reads each dock's groups from a dumped dock area: the leaf panels'
/// names in order, with each group's active index. Anything that is not
/// one of the four panels is left out.
pub(crate) fn arrangement_from(state: &DockAreaState) -> Arrangement {
    let mut arrangement = Arrangement::default();
    let docks = [
        (Side::Left, &state.left_dock),
        (Side::Right, &state.right_dock),
        (Side::Bottom, &state.bottom_dock),
    ];
    for (side, dock) in docks {
        if let Some(dock) = dock {
            read_node(&mut arrangement, side, DockState::panel(dock));
        }
    }
    arrangement
}

fn read_node(arrangement: &mut Arrangement, side: Side, node: &PanelState) {
    match node.panel_name.as_str() {
        SPLIT => node
            .children
            .iter()
            .for_each(|child| read_node(arrangement, side, child)),
        TAB_GROUP => {
            let mut active = node.info.active_index().unwrap_or(0);
            let mut panels = Vec::new();
            for (ix, child) in node.children.iter().enumerate() {
                if let Some(kind) = PanelKind::from_name(&child.panel_name) {
                    if ix == active {
                        active = panels.len();
                    }
                    panels.push(kind);
                }
            }
            arrangement.push_group(side, panels, active);
        }
        name => {
            // A bare panel at a dock's root is a group of one.
            if let Some(kind) = PanelKind::from_name(name) {
                arrangement.push_group(side, vec![kind], 0);
            }
        }
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
