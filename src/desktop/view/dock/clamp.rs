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

/// A change smaller than this is none: the Kit measures in fractions.
const NO_CHANGE: f32 = 0.5;

/// The sizes `sizes` (left, right, bottom) become in an area `width` by
/// `height`, for the docks that are `open`: both side docks are scaled in
/// proportion until the item area is 320 px wide, and the bottom dock is
/// reduced until it is 200 px tall. `None` for a dock that fits, that is
/// closed, or that would change by under half a pixel.
pub(super) fn clamped(
    width: f32,
    height: f32,
    sizes: [f32; 3],
    open: [bool; 3],
) -> [Option<f32>; 3] {
    let mut out = sizes;
    let total = [0, 1]
        .into_iter()
        .filter(|side| open[*side])
        .map(|side| sizes[side])
        .sum::<f32>();
    if total > 0. && width - total < MIN_ITEM_WIDTH {
        let scale = (width - MIN_ITEM_WIDTH).max(0.) / total;
        for side in [0, 1] {
            if open[side] {
                out[side] = (sizes[side] * scale).max(MIN_DOCK);
            }
        }
    }
    if open[2] && height - sizes[2] < MIN_ITEM_HEIGHT {
        out[2] = (height - MIN_ITEM_HEIGHT).max(MIN_DOCK);
    }
    [0, 1, 2].map(|side| {
        (open[side] && (out[side] - sizes[side]).abs() > NO_CHANGE).then_some(out[side])
    })
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
            if let Some(size) = to[side as usize] {
                self.area.update(cx, |area, cx| {
                    area.set_dock_size(placement(side), px(size), window, cx)
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "clamp_tests.rs"]
mod tests;
