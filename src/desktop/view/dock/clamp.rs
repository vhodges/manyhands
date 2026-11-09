//! Keeping the item area usable (R-37): docks restored from a larger
//! screen are scaled down to fit.

use gpui_kit::{App, Window, px};

use super::{DockShell, placement};
use crate::desktop::model::layout::{Layout, Side};

/// The narrowest item area the side docks may leave.
const MIN_ITEM_WIDTH: f32 = 320.;
/// The shortest item area the bottom dock may leave.
const MIN_ITEM_HEIGHT: f32 = 200.;
/// No dock is made smaller than this (the Kit's floor).
const MIN_DOCK: f32 = 100.;

/// The sizes `sizes` (left, right, bottom) become in an area `width` by
/// `height`, for the docks that are `open`: both side docks are scaled in
/// proportion until the item area is 320 px wide, and the bottom dock is
/// reduced until it is 200 px tall. A dock that fits is left as it is.
pub(super) fn clamped(width: f32, height: f32, sizes: [f32; 3], open: [bool; 3]) -> [f32; 3] {
    let mut out = sizes;
    let sides = [open[0], open[1]].map(f32::from);
    let total = sizes[0] * sides[0] + sizes[1] * sides[1];
    if total > 0. && width - total < MIN_ITEM_WIDTH {
        let scale = (width - MIN_ITEM_WIDTH).max(0.) / total;
        for side in 0..2 {
            if open[side] {
                out[side] = (sizes[side] * scale).max(MIN_DOCK);
            }
        }
    }
    if open[2] && height - sizes[2] < MIN_ITEM_HEIGHT {
        out[2] = (height - MIN_ITEM_HEIGHT).max(MIN_DOCK);
    }
    out
}

impl DockShell {
    /// Whether the area has been laid out, so it has bounds.
    pub(in crate::desktop::view) fn has_bounds(&self, cx: &App) -> bool {
        let bounds = self.area.read(cx).bounds();
        bounds.size.width > px(0.) && bounds.size.height > px(0.)
    }

    pub(in crate::desktop::view) fn clamped(&self) -> bool {
        self.clamped
    }

    /// The first time the area has bounds: the one clamp of the launch.
    pub(in crate::desktop::view) fn clamp_first(
        &mut self,
        layout: &Layout,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.clamped || !self.has_bounds(cx) {
            return;
        }
        self.clamped = true;
        self.clamp_docks(layout, window, cx);
    }

    /// Scales the open docks to fit the area. Never before the area has
    /// bounds. A dock it changes is a layout change, which the handler
    /// accepts and saves (R-37).
    pub(in crate::desktop::view) fn clamp_docks(
        &self,
        layout: &Layout,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.has_bounds(cx) {
            return;
        }
        let bounds = self.area.read(cx).bounds();
        let current = Side::ALL.map(|side| self.area.read(cx).dock_size(placement(side)));
        let open = Side::ALL.map(|side| current[side as usize].is_some() && layout.is_open(side));
        let sizes = current.map(|size| size.map_or(0., |size| size.as_f32()));
        let to = clamped(
            bounds.size.width.as_f32(),
            bounds.size.height.as_f32(),
            sizes,
            open,
        );
        for side in Side::ALL {
            let ix = side as usize;
            if open[ix] && (to[ix] - sizes[ix]).abs() > 0.5 {
                self.area.update(cx, |area, cx| {
                    area.set_dock_size(placement(side), px(to[ix]), window, cx)
                });
            }
        }
    }
}
