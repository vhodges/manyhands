//! Telling a change in the dock area from the events of its own launch
//! and of the shell's own edits (R-6, R-38).

use gpui_kit::component::dock::{DockAreaState, DockState, PanelInfo, PanelState};

use crate::desktop::model::layout::{ITEM_AREA_NAME, Side};

/// Dock sizes within this many pixels are the same size (fact 3).
const SIZE_TOLERANCE: f32 = 1.;
/// A split's slots are compared as shares of the split, within this much.
/// The Kit keeps a split's proportions, not its pixels: a dump before the
/// first layout pass holds the tree's sizes, a later one the measured
/// sizes, and a window resize rescales them without an event
/// (`B/dock/dock_area.rs:827-848`, `:1470-1481`).
const SHARE_TOLERANCE: f32 = 0.005;

/// Whether `a` and `b` are the same arrangement: the same trees, tabs,
/// open flags, dock sizes and split proportions.
pub(in crate::desktop::view) fn same_layout(a: &DockAreaState, b: &DockAreaState) -> bool {
    a.version == b.version
        && same_tree(&a.center, &b.center)
        && same_dock(&a.left_dock, &b.left_dock)
        && same_dock(&a.right_dock, &b.right_dock)
        && same_dock(&a.bottom_dock, &b.bottom_dock)
}

fn same_dock(a: &Option<DockState>, b: &Option<DockState>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.placement() == b.placement()
                && a.open() == b.open()
                && (a.size().as_f32() - b.size().as_f32()).abs() <= SIZE_TOLERANCE
                && same_tree(a.panel(), b.panel())
        }
        _ => false,
    }
}

fn same_tree(a: &PanelState, b: &PanelState) -> bool {
    a.panel_name == b.panel_name
        && a.children.len() == b.children.len()
        && same_info(&a.info, &b.info)
        && a.children
            .iter()
            .zip(&b.children)
            .all(|(a, b)| same_tree(a, b))
}

fn same_info(a: &PanelInfo, b: &PanelInfo) -> bool {
    match (a, b) {
        (PanelInfo::Stack { sizes: a, axis: x }, PanelInfo::Stack { sizes: b, axis: y }) => {
            x == y
                && a.len() == b.len()
                && shares(a)
                    .zip(shares(b))
                    .all(|(a, b)| (a - b).abs() <= SHARE_TOLERANCE)
        }
        (a, b) => a == b,
    }
}

/// Each slot's share of the split.
fn shares(sizes: &[gpui_kit::Pixels]) -> impl Iterator<Item = f32> + '_ {
    let total: f32 = sizes.iter().map(|size| size.as_f32()).sum();
    sizes.iter().map(move |size| {
        if total > 0. {
            size.as_f32() / total
        } else {
            0.
        }
    })
}

/// Whether the centre holds exactly the item area: a drop on it leaves
/// anything else there (R-6, step 2).
pub(in crate::desktop::view) fn centre_is_item_area(state: &DockAreaState) -> bool {
    fn leaves<'a>(node: &'a PanelState, out: &mut Vec<&'a str>) {
        if matches!(node.info, PanelInfo::Panel(_)) {
            out.push(&node.panel_name);
        }
        node.children.iter().for_each(|child| leaves(child, out));
    }
    let mut names = Vec::new();
    leaves(&state.center, &mut names);
    names == [ITEM_AREA_NAME]
}

/// Whether `state` has a dock on `side` and it is open.
pub(in crate::desktop::view) fn open_in(state: &DockAreaState, side: Side) -> bool {
    side.dock(state).as_ref().is_some_and(DockState::open)
}
