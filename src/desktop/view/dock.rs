//! The dock area around the item area (Wave 04 Cycle 01, Part C): the Kit's
//! `DockArea` holding the five panels, built from the layout model.

mod header;
mod panels;

use std::{collections::HashMap, rc::Rc, sync::Arc};

use gpui_kit::{
    App, AppContext as _, Context, Entity, EntityId, FocusHandle, Global, Pixels, Subscription,
    Window,
    component::dock::{
        BasePanelView, DockArea, DockAreaState, DockLayout, DockPlacement, DockSkin, panel_handle,
        register_panel,
    },
    px,
};

use self::header::ShellRenderer;
pub(crate) use self::panels::{RegionPanel, Slot};
use super::workspace::Workspace;
use crate::desktop::model::layout::{DEFAULT_LEFT_SPLIT, Group, Layout, Side, arrangement_from};

/// The version written into the area's dump (Part G).
pub(crate) const LAYOUT_VERSION: usize = 1;

/// The panels of every dock area, by the area's entity id (R-34).
///
/// The registry's builders look their handle up here when an area loads a
/// saved layout, from the area's id alone: they run inside the area's own
/// update, so they must not read the area or the Workspace. A window's
/// entry is inserted before its area's first load and removed when its
/// Workspace is released, so two windows never swap panels and a load never
/// makes a second copy.
#[derive(Default)]
struct DockPanels(HashMap<EntityId, HashMap<&'static str, Arc<dyn BasePanelView>>>);

impl Global for DockPanels {}

/// Registers one builder per stable name, once per App.
fn register_builders(cx: &mut App) {
    if cx.has_global::<DockPanels>() {
        return;
    }
    cx.set_global(DockPanels::default());
    for slot in Slot::ALL {
        let name = slot.name();
        register_panel(cx, name, move |context, _, cx| {
            let area = context.dock_area().entity_id();
            cx.global::<DockPanels>()
                .0
                .get(&area)
                .and_then(|panels| panels.get(name))
                .cloned()
                .expect("a dock area loads only the panels of its own window")
        });
    }
}

fn placement(side: Side) -> DockPlacement {
    match side {
        Side::Left => DockPlacement::Left,
        Side::Right => DockPlacement::Right,
        Side::Bottom => DockPlacement::Bottom,
    }
}

/// The dock area and what the Workspace keeps beside it.
pub(super) struct DockShell {
    area: Entity<DockArea>,
    /// The panels, to turn them off and on.
    #[allow(dead_code)] // Called from Task 5.
    panels: Vec<(Slot, Entity<RegionPanel>)>,
    /// The last dump the shell accepted as the layout (R-6, R-38).
    #[allow(dead_code)] // Called from Task 5.
    accepted: DockAreaState,
    _released: Subscription,
}

impl DockShell {
    /// Builds the area with `layout`'s groups and sizes, around the item
    /// area. In the same update the layout's arrangement and the accepted
    /// state are taken from a dump of the new area, so the launch's own
    /// `LayoutChanged` events change nothing (R-38).
    pub(super) fn build(
        regions: &[FocusHandle; 5],
        layout: &mut Layout,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        register_builders(cx);
        let workspace = cx.entity();
        let panels: Vec<(Slot, Entity<RegionPanel>)> = Slot::ALL
            .into_iter()
            .map(|slot| {
                let focus = regions[slot.region().index()].clone();
                (slot, RegionPanel::new(&workspace, slot, focus, cx))
            })
            .collect();
        let handles: HashMap<&'static str, Arc<dyn BasePanelView>> = panels
            .iter()
            .map(|(slot, panel)| (slot.name(), panel_handle(panel.clone())))
            .collect();

        // The renderer is built inside the area's constructor, since the
        // skin needs the area's own context (as `DockSkin::dock_area` does).
        let area = cx.new(|cx| {
            let skin = DockSkin::new(cx);
            DockArea::new("workspace-dock", Some(LAYOUT_VERSION), window, cx)
                .with_renderer(Rc::new(ShellRenderer::new(skin)))
        });
        let area_id = area.entity_id();
        cx.global_mut::<DockPanels>()
            .0
            .insert(area_id, handles.clone());
        let released = cx.on_release(move |_, cx| {
            if cx.has_global::<DockPanels>() {
                cx.global_mut::<DockPanels>().0.remove(&area_id);
            }
        });

        let center = DockLayout::tabs().panel_view(handles[Slot::ItemArea.name()].clone(), cx);
        let docks: Vec<(Side, DockLayout)> = Side::ALL
            .into_iter()
            .filter(|side| layout.has_panels(*side))
            .map(|side| (side, side_layout(layout, side, &handles, cx)))
            .collect();
        let sizes: Vec<(Side, Pixels)> = docks
            .iter()
            .map(|(side, _)| (*side, px(layout.size(*side))))
            .collect();
        area.update(cx, |area, cx| {
            area.set_center(center, window, cx);
            for (side, dock) in docks {
                area.set_dock(placement(side), dock, window, cx);
            }
            for (side, size) in sizes {
                area.set_dock_size(placement(side), size, window, cx);
            }
        });
        let accepted = area.read(cx).dump(cx);
        layout.set_arrangement(arrangement_from(&accepted));

        Self {
            area,
            panels,
            accepted,
            _released: released,
        }
    }

    pub(super) fn area(&self) -> &Entity<DockArea> {
        &self.area
    }
}

/// One dock: its group, or its groups stacked in a side dock and side by
/// side in the bottom one. The left dock's two groups divide 0.6 : 1.4
/// (fact 3: sizes are pixels, rescaled to the container in proportion).
fn side_layout(
    layout: &Layout,
    side: Side,
    handles: &HashMap<&'static str, Arc<dyn BasePanelView>>,
    cx: &App,
) -> DockLayout {
    let tabs = |group: &Group| {
        group
            .panels
            .iter()
            .fold(DockLayout::tabs(), |tabs, kind| {
                tabs.panel_view(handles[kind.name()].clone(), cx)
            })
            .active_index(group.active)
    };
    match layout.groups(side) {
        [only] => tabs(only),
        groups => {
            let split = match side {
                Side::Bottom => DockLayout::h_split(),
                Side::Left | Side::Right => DockLayout::v_split(),
            };
            groups.iter().enumerate().fold(split, |split, (ix, group)| {
                let size = (side == Side::Left && groups.len() == DEFAULT_LEFT_SPLIT.len())
                    .then(|| px(DEFAULT_LEFT_SPLIT[ix] * 1000.));
                split.child(tabs(group), size)
            })
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "dock_tests.rs"]
mod tests;
