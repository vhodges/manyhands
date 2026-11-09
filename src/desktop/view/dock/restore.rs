//! The launch: the saved layout, or the default and why (Part G, R-38).

use gpui_kit::{
    App, Window,
    component::dock::{DockAreaState, DockState},
};

use super::{DockShell, accepted::centre_is_item_area};
use crate::desktop::{
    model::layout::{Layout, Restore, Side, arrangement_from, restore},
    services::{Backend, layout::LayoutStore},
};

/// What the launch found: the store, if it opened, and what `restore`
/// made of its file.
pub(in crate::desktop::view) struct Launch {
    pub(super) store: Option<LayoutStore>,
    restore: Restore,
}

impl Launch {
    /// Reads the file, once, on the thread that builds the window: a few
    /// kilobytes, like the stores' own opening. A store that did not open
    /// restores the default with the notice, and turns saving off.
    pub(in crate::desktop::view) fn read(backend: &dyn Backend) -> Self {
        match backend.layout() {
            Ok(store) => Self {
                restore: restore(store.read()),
                store: Some(store.clone()),
            },
            Err(error) => Self {
                restore: restore(Err(error)),
                store: None,
            },
        }
    }
}

/// `area` with every dock open as `layout` says it is: the file's own flag
/// is written and ignored (Part G).
fn opened_as(area: DockAreaState, layout: &Layout) -> DockAreaState {
    let mut area = area;
    for (side, dock) in [
        (Side::Left, &mut area.left_dock),
        (Side::Right, &mut area.right_dock),
        (Side::Bottom, &mut area.bottom_dock),
    ] {
        if let Some(state) = dock {
            *dock = Some(DockState::new(
                state.panel().clone(),
                state.placement(),
                state.size(),
                layout.is_open(side),
            ));
        }
    }
    area
}

impl DockShell {
    /// Part G's four steps, the first of which `build` has done (the area
    /// holds the default layout, and `default_state` is its dump):
    /// load the file's layout with its docks open as `on` and `shown` say,
    /// run the centre check once, and fall back to the default state with
    /// the notice if it fails; then reconcile and take the accepted state
    /// and the arrangement from a dump, in this same update, so the
    /// launch's own events change nothing and save nothing (R-38, P37).
    pub(super) fn apply_launch(
        &mut self,
        launch: Launch,
        default_state: DockAreaState,
        layout: &mut Layout,
        window: &mut Window,
        cx: &mut App,
    ) {
        match launch.restore {
            Restore::Default { notice } => self.notices.restore = notice,
            Restore::Saved(saved) => {
                let state = opened_as(saved.area, &saved.layout);
                let loaded = self
                    .area
                    .update(cx, |area, cx| area.load(state, window, cx));
                debug_assert!(loaded.is_ok(), "the Kit's load always answers Ok");
                if centre_is_item_area(&self.area.read(cx).dump(cx)) {
                    *layout = saved.layout;
                } else {
                    let loaded = self
                        .area
                        .update(cx, |area, cx| area.load(default_state, window, cx));
                    debug_assert!(loaded.is_ok(), "the Kit's load always answers Ok");
                    *layout = Layout::default();
                    self.notices.restore = true;
                }
            }
        }
        self.reconcile(layout, window, cx);
        let accepted = self.area.read(cx).dump(cx);
        layout.set_arrangement(arrangement_from(&accepted));
        self.accepted = accepted;
    }
}
