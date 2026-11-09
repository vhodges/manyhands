//! The dock area around the item area (Wave 04 Cycle 01, Part C): the Kit's
//! `DockArea` holding the five panels, built from the layout model.

pub(super) mod accepted;
mod clamp;
mod header;
mod moves;
mod panels;
mod reconcile;
mod restore;
mod save;

use std::{collections::HashMap, rc::Rc, sync::Arc};

use gpui_kit::{
    App, AppContext as _, Context, Entity, EntityId, FocusHandle, Global, Pixels, Subscription,
    Window,
    component::dock::{
        BasePanelView, DockArea, DockAreaState, DockEvent, DockLayout, DockSkin, panel_handle,
        register_panel,
    },
    px,
};

use self::header::ShellRenderer;
pub(crate) use self::panels::{RegionPanel, Slot};
pub(super) use self::restore::Launch;
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) use self::save::LAYOUT_SAVE_DELAY;
use self::save::SaveState;
use super::workspace::Workspace;
use crate::desktop::{
    model::layout::{DEFAULT_LEFT_SPLIT, Group, LAYOUT_VERSION, Layout, Side},
    services::layout::LayoutStore,
};

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

/// The dock area and what the Workspace keeps beside it.
pub(super) struct DockShell {
    area: Entity<DockArea>,
    /// The panels, to turn them off and on.
    panels: Vec<(Slot, Entity<RegionPanel>)>,
    /// The last dump the shell accepted as the layout (R-6, R-38).
    accepted: DockAreaState,
    /// Where the layout is kept; `None` when the store did not open, which
    /// turns saving off for the session (R-21).
    store: Option<LayoutStore>,
    /// The debounced save (R-19) and its writes.
    save: SaveState,
    /// How many saves the layout has asked for (R-31); it says which of
    /// them a write has since covered.
    saves: usize,
    /// The restore and save notices in Activity (R-21).
    notices: Notices,
    /// Whether the clamp for the first non-zero bounds has run (R-37).
    clamped: bool,
    /// A `clamp_first` is queued and has not run yet.
    clamp_queued: bool,
    _events: Subscription,
    _focus_in: Vec<Subscription>,
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
        launch: Launch,
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
                .with_renderer(Rc::new(ShellRenderer::new(skin, workspace.downgrade())))
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
                area.set_dock(side.placement(), dock, window, cx);
            }
            for (side, size) in sizes {
                area.set_dock_size(side.placement(), size, window, cx);
            }
        });
        let default_state = area.read(cx).dump(cx);
        let events = cx.subscribe_in(&area, window, |this, _, event: &DockEvent, window, cx| {
            this.dock_event(event, window, cx)
        });

        // Focus entering a region that is not displayed, such as a click on
        // the closed strip outside its tabs (R-9), goes to the item area.
        let focus_in = Slot::ALL
            .into_iter()
            .filter(|slot| *slot != Slot::ItemArea)
            .map(|slot| {
                cx.on_focus_in(
                    &regions[slot.region().index()],
                    window,
                    |this, window, cx| this.check_focus(window, cx),
                )
            })
            .collect();

        let mut shell = Self {
            area,
            panels,
            accepted: default_state.clone(),
            store: launch.store.clone(),
            save: SaveState::default(),
            saves: 0,
            notices: Notices::new(cx),
            clamped: false,
            clamp_queued: false,
            _events: events,
            _focus_in: focus_in,
            _released: released,
        };
        shell.apply_launch(launch, default_state, layout, window, cx);
        shell
    }

    pub(super) fn area(&self) -> &Entity<DockArea> {
        &self.area
    }

    pub(super) fn accepted(&self) -> &DockAreaState {
        &self.accepted
    }

    pub(super) fn accept(&mut self, state: DockAreaState) {
        self.accepted = state;
    }

    pub(super) fn notices(&self) -> &Notices {
        &self.notices
    }

    pub(super) fn notices_mut(&mut self) -> &mut Notices {
        &mut self.notices
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(super) fn saves(&self) -> usize {
        self.saves
    }
}

/// The two notices the layout file can leave in Activity (R-21), and the
/// focus handles of their Dismiss buttons. Each lasts for the session.
pub(in crate::desktop::view) struct Notices {
    /// The saved layout could not be used, so the default is back.
    pub(in crate::desktop::view) restore: bool,
    /// The last write failed.
    pub(in crate::desktop::view) save: bool,
    pub(in crate::desktop::view) restore_focus: FocusHandle,
    pub(in crate::desktop::view) save_focus: FocusHandle,
}

impl Notices {
    fn new(cx: &mut Context<Workspace>) -> Self {
        Self {
            restore: false,
            save: false,
            restore_focus: cx.focus_handle().tab_stop(true),
            save_focus: cx.focus_handle().tab_stop(true),
        }
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
#[path = "dock_tests/mod.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "dock/restore_tests.rs"]
pub(in crate::desktop::view) mod restore_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "dock/save_tests.rs"]
mod save_tests;
