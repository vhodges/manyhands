//! Reconcile (R-3): brings the dock area in line with the layout.

use gpui_kit::{App, Window, component::dock::PanelId};

use super::{DockShell, Slot};
use crate::desktop::model::layout::{Layout, PanelKind, Side, arrangement_from};

impl DockShell {
    /// The panel entity's id in the area.
    pub(super) fn panel_id(&self, kind: PanelKind) -> Option<PanelId> {
        self.panels
            .iter()
            .find(|(slot, _)| *slot == Slot::Panel(kind))
            .map(|(_, panel)| PanelId::from(panel.entity_id()))
    }

    /// Makes the area say what `layout` says, touching only what differs,
    /// so a second call does nothing and the events the first causes
    /// settle (R-6):
    ///
    /// 1. a dock with no panels leaves the area, its size remembered;
    /// 2. a panel is visible when it is on; while the bottom dock is
    ///    closed every panel in it is, so its strip names each one
    ///    (R-35);
    /// 3. a group's active tab is the one the layout says;
    /// 4. a dock is open when the layout says it is.
    ///
    /// Returns the docks it opened, which the caller scales to fit the
    /// area (R-37).
    pub(in crate::desktop::view) fn reconcile(
        &self,
        layout: &mut Layout,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<Side> {
        let area = &self.area;
        for side in Side::ALL {
            let at = side.placement();
            if !layout.has_panels(side) && area.read(cx).has_dock(at) {
                if let Some(size) = area.read(cx).dock_size(at) {
                    layout.set_size(side, size.as_f32());
                }
                area.update(cx, |area, cx| area.remove_dock(at, window, cx));
            }
        }

        let strip = !layout.is_open(Side::Bottom);
        for (slot, panel) in &self.panels {
            if let Slot::Panel(kind) = slot {
                let visible =
                    layout.is_on(*kind) || (strip && layout.side_of(*kind) == Some(Side::Bottom));
                panel.update(cx, |panel, cx| panel.set_visible(visible, cx));
            }
        }

        let seen = arrangement_from(&area.read(cx).dump(cx));
        for side in Side::ALL {
            for (want, have) in layout.groups(side).iter().zip(seen.groups(side)) {
                if want.panels == have.panels
                    && want.active != have.active
                    && let Some(id) = want.active_panel().and_then(|kind| self.panel_id(kind))
                {
                    area.update(cx, |area, cx| area.select_panel(id, window, cx));
                }
            }
        }

        let mut opened = Vec::new();
        for side in Side::ALL {
            let at = side.placement();
            if layout.has_panels(side) && area.read(cx).has_dock(at) {
                let open = layout.is_open(side);
                // The Kit refuses to close a dock that is not collapsible. Every
                // dock here is (its default; the shell never changes it), so the
                // toggle always flips it.
                if area.read(cx).is_dock_open(at) != open {
                    area.update(cx, |area, cx| area.toggle_dock(at, window, cx));
                    if open {
                        opened.push(side);
                    }
                }
            }
        }
        area.update(cx, |_, cx| cx.notify());
        opened
    }
}
