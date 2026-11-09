//! What the dock tests share: the operations as a person's input reaches
//! them, the area's state as a dump shows it, and the invariant that the
//! layout and the area agree after every step.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    Context, Entity, Subscription, TestAppContext, Window,
    component::dock::{
        DockArea, DockAreaState, DockEvent, DockPlacement, InsertTarget, NodeId, PanelId,
        PanelState,
    },
    test::TestWindowExt,
};

use crate::desktop::{
    model::layout::{ITEM_AREA_NAME, Layout, PanelKind, Side, arrangement_from},
    view::{
        harness::Shell,
        workspace::{REGIONS, Region, Workspace},
    },
};

pub(super) const PLACEMENTS: [DockPlacement; 3] = [
    DockPlacement::Left,
    DockPlacement::Right,
    DockPlacement::Bottom,
];

/// A dock of a dump: whether it is open, and its groups with each one's
/// panel names and active index.
pub(super) type DockShape = Option<(bool, Vec<(Vec<String>, usize)>)>;

pub(super) fn shape(state: &DockAreaState) -> [DockShape; 3] {
    fn walk(node: &PanelState, out: &mut Vec<(Vec<String>, usize)>) {
        if node.panel_name == "TabPanel" {
            let names = node.children.iter().map(|c| c.panel_name.clone()).collect();
            out.push((names, node.info.active_index().unwrap_or(0)));
        } else {
            node.children.iter().for_each(|child| walk(child, out));
        }
    }
    [&state.left_dock, &state.right_dock, &state.bottom_dock].map(|dock| {
        dock.as_ref().map(|dock| {
            let mut groups = Vec::new();
            walk(dock.panel(), &mut groups);
            (dock.open(), groups)
        })
    })
}

/// Every pixel size of a dump, docks and splits in order.
pub(super) fn sizes(state: &DockAreaState) -> Vec<f32> {
    fn walk(node: &PanelState, out: &mut Vec<f32>) {
        if let Some(sizes) = node.info.sizes() {
            out.extend(sizes.iter().map(|size| size.as_f32()));
        }
        node.children.iter().for_each(|child| walk(child, out));
    }
    let mut out = Vec::new();
    for dock in [&state.left_dock, &state.right_dock, &state.bottom_dock]
        .into_iter()
        .flatten()
    {
        out.push(dock.size().as_f32());
        walk(dock.panel(), &mut out);
    }
    out
}

pub(super) fn area(shell: &Shell, cx: &mut TestAppContext) -> Entity<DockArea> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.dock_area().clone())
}

pub(super) fn dump(shell: &Shell, cx: &mut TestAppContext) -> DockAreaState {
    area(shell, cx).read_with(cx, |area, cx| area.dump(cx))
}

pub(super) fn layout(shell: &Shell, cx: &mut TestAppContext) -> Layout {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.layout().clone())
}

/// How many saves the layout has asked for so far (R-31).
pub(super) fn saves(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.layout_saves())
}

/// Runs `f` on the Workspace in a drawn frame, as a key or a click does,
/// and lets the events it causes settle.
pub(super) fn act(
    shell: &Shell,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>),
) {
    shell.frame(cx, |window, cx| {
        shell
            .workspace
            .update(cx, |workspace, cx| f(workspace, window, cx))
    });
    shell.run(cx);
}

pub(super) fn toggle_panel(shell: &Shell, kind: PanelKind, cx: &mut TestAppContext) {
    act(shell, cx, |ws, window, cx| {
        ws.toggle_panel(kind, window, cx)
    });
}

pub(super) fn toggle_dock(shell: &Shell, side: Side, cx: &mut TestAppContext) {
    act(shell, cx, |ws, window, cx| ws.toggle_dock(side, window, cx));
}

pub(super) fn dock_panel(shell: &Shell, kind: PanelKind, side: Side, cx: &mut TestAppContext) {
    act(shell, cx, |ws, window, cx| {
        ws.dock_panel(kind, side, window, cx)
    });
}

/// Runs `f` on the area in a drawn frame: the edit a drag commits.
pub(super) fn edit_area(
    shell: &Shell,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut DockArea, &mut Window, &mut Context<DockArea>),
) {
    let area = area(shell, cx);
    shell.frame(cx, |window, cx| {
        area.update(cx, |area, cx| f(area, window, cx))
    });
    shell.run(cx);
}

pub(super) fn panel_id(
    area: &Entity<DockArea>,
    kind: PanelKind,
    cx: &mut TestAppContext,
) -> PanelId {
    area.read_with(cx, |area, cx| {
        [
            DockPlacement::Left,
            DockPlacement::Right,
            DockPlacement::Bottom,
            DockPlacement::Center,
        ]
        .into_iter()
        .filter_map(|placement| area.layout(placement))
        .flat_map(|tree| tree.panels())
        .find(|id| {
            area.panel(*id)
                .is_some_and(|p| p.panel_name(cx) == kind.name())
        })
        .unwrap_or_else(|| panic!("{} is in the area", kind.name()))
    })
}

/// The group holding `kind`.
pub(super) fn group_of(
    area: &Entity<DockArea>,
    kind: PanelKind,
    cx: &mut TestAppContext,
) -> NodeId {
    let id = panel_id(area, kind, cx);
    area.read_with(cx, |area, _| {
        [
            DockPlacement::Left,
            DockPlacement::Right,
            DockPlacement::Bottom,
        ]
        .into_iter()
        .filter_map(|placement| area.layout(placement))
        .find_map(|tree| tree.find_panel_node(id))
        .expect("the panel is in a group")
    })
}

/// The Kit's own edit for a panel dropped into `with`'s group, at the end.
pub(super) fn put_in_group(
    shell: &Shell,
    kind: PanelKind,
    with: PanelKind,
    activate: bool,
    cx: &mut TestAppContext,
) {
    let area = area(shell, cx);
    let (id, node) = (panel_id(&area, kind, cx), group_of(&area, with, cx));
    edit_area(shell, cx, |area, window, cx| {
        area.move_panel(
            id,
            InsertTarget::Tabs {
                node,
                ix: None,
                activate,
            },
            window,
            cx,
        )
    });
}

/// The group holding the item area, in the centre.
pub(super) fn item_area_group(area: &Entity<DockArea>, cx: &mut TestAppContext) -> NodeId {
    area.read_with(cx, |area, cx| {
        let tree = area.layout(DockPlacement::Center).unwrap();
        let id = tree
            .panels()
            .find(|id| {
                area.panel(*id)
                    .is_some_and(|p| p.panel_name(cx) == ITEM_AREA_NAME)
            })
            .expect("the item area is in the centre");
        tree.find_panel_node(id).unwrap()
    })
}

/// Counts the area's `LayoutChanged` events while the returned guard lives.
pub(super) fn count_changes(
    area: &Entity<DockArea>,
    cx: &mut TestAppContext,
) -> (Rc<Cell<usize>>, Subscription) {
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(area, move |_, event: &DockEvent, _| {
            if matches!(event, DockEvent::LayoutChanged) {
                seen.set(seen.get() + 1);
            }
        })
    });
    (count, subscription)
}

/// Whether the panel is visible to the Kit.
pub(super) fn visible(shell: &Shell, kind: PanelKind, cx: &mut TestAppContext) -> bool {
    let area = area(shell, cx);
    let id = panel_id(&area, kind, cx);
    area.read_with(cx, |area, cx| area.panel(id).unwrap().visible(cx))
}

/// The layout and the area say the same: each dock's open flag, its groups
/// with their active tabs, and each panel's visible flag (a panel is
/// visible when it is on, and every panel of a closed bottom dock is, so
/// its strip names them: R-35).
pub(super) fn assert_agreement(shell: &Shell, what: &str, cx: &mut TestAppContext) {
    let layout = layout(shell, cx);
    let area = area(shell, cx);
    let state = area.read_with(cx, |area, cx| area.dump(cx));
    let seen = arrangement_from(&state);
    for (side, p) in Side::ALL.into_iter().zip(PLACEMENTS) {
        assert_eq!(
            seen.groups(side),
            layout.groups(side),
            "{what}: {side:?} groups"
        );
        assert_eq!(
            area.read_with(cx, |area, _| area.has_dock(p)),
            layout.has_panels(side),
            "{what}: {side:?} has a dock exactly when it has panels"
        );
        if layout.has_panels(side) {
            assert_eq!(
                area.read_with(cx, |area, _| area.is_dock_open(p)),
                layout.is_open(side),
                "{what}: {side:?} open"
            );
        }
    }
    for kind in PanelKind::ALL {
        let strip = layout.side_of(kind) == Some(Side::Bottom) && !layout.is_open(Side::Bottom);
        assert_eq!(
            visible(shell, kind, cx),
            layout.is_on(kind) || strip,
            "{what}: {kind:?} visible"
        );
    }
}

/// Which regions are drawn, in `PanelKind::ALL` order (Repositories,
/// Tickets, Documents, Activity).
pub(super) fn drawn(shell: &Shell, cx: &mut TestAppContext) -> [bool; 4] {
    [
        Region::Repositories,
        Region::Tickets,
        Region::Documents,
        Region::Operations,
    ]
    .map(|region| shell.present(region.id(), cx))
}

/// The region that holds focus, among those drawn.
pub(super) fn focus(shell: &Shell, cx: &mut TestAppContext) -> Option<Region> {
    shell.frame(cx, |window, _| {
        REGIONS
            .into_iter()
            .find(|region| window.try_find(region.id()).and_then(|e| e.focused()) == Some(true))
    })
}

pub(super) fn displayed(shell: &Shell, cx: &mut TestAppContext) -> [bool; 4] {
    let layout = layout(shell, cx);
    PanelKind::ALL.map(|kind| layout.is_displayed(kind))
}
