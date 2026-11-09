//! The Kit's side of a menu move (R-5).

use gpui_kit::{
    App, Window,
    component::{Placement, dock::InsertTarget},
    px,
};

use super::{DockPanels, DockShell, Slot, placement};
use crate::desktop::model::layout::{Layout, PanelKind, Side};

impl DockShell {
    /// Puts `kind` at the end of `side` as a new group: beside the dock's
    /// root, below it in a side dock and to its right in the bottom one
    /// (the dock may be closed). A side with no dock gets one at its last
    /// or default size, holding the panel. The layout is not changed: the
    /// caller reads the arrangement back from the area.
    pub(in crate::desktop::view) fn move_to_side(
        &self,
        kind: PanelKind,
        side: Side,
        layout: &mut Layout,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(id) = self.panel_id(kind) else {
            return;
        };
        let at = placement(side);
        let area = &self.area;
        let root = area
            .read(cx)
            .layout(at)
            .filter(|tree| tree.panels().next().is_some())
            .map(|tree| tree.root().id());
        if let Some(node) = root {
            let placement = match side {
                Side::Bottom => Placement::Right,
                Side::Left | Side::Right => Placement::Bottom,
            };
            let target = InsertTarget::Split {
                node,
                placement,
                size: None,
            };
            area.update(cx, |area, cx| area.move_panel(id, target, window, cx));
            return;
        }
        // A dock the area still holds with nothing in it is made again.
        if let Some(size) = area.read(cx).dock_size(at) {
            layout.set_size(side, size.as_f32());
            area.update(cx, |area, cx| area.remove_dock(at, window, cx));
        }
        let Some((_, panel)) = self
            .panels
            .iter()
            .find(|(slot, _)| *slot == Slot::Panel(kind))
        else {
            return;
        };
        let handle = cx
            .global::<DockPanels>()
            .0
            .get(&area.entity_id())
            .and_then(|panels| panels.get(kind.name()))
            .cloned();
        let Some(handle) = handle else {
            return;
        };
        let size = px(layout.size(side));
        area.update(cx, |area, cx| {
            area.remove_panel(panel.clone(), window, cx);
            area.add_panel_view(handle, at, Some(size), window, cx);
        });
    }
}
