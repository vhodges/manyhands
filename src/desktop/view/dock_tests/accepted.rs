//! R-6's first step, pure: what counts as the accepted state again.

use gpui_kit::{
    Axis,
    component::dock::{DockAreaState, DockPlacement, DockState, PanelInfo, PanelState},
    px,
};

use crate::desktop::{
    model::layout::{ITEM_AREA_NAME, Side},
    view::dock::accepted::{centre_is_item_area, open_in, same_layout},
};

fn leaf(name: &str) -> PanelState {
    PanelState::new(name)
}

fn tabs(names: &[&str], active: usize) -> PanelState {
    PanelState {
        panel_name: "TabPanel".to_owned(),
        children: names.iter().map(|name| leaf(name)).collect(),
        info: PanelInfo::tabs(active),
    }
}

fn stack(axis: Axis, sizes: &[f32], children: Vec<PanelState>) -> PanelState {
    PanelState {
        panel_name: "StackPanel".to_owned(),
        children,
        info: PanelInfo::stack(sizes.iter().map(|size| px(*size)).collect(), axis),
    }
}

fn dock(panel: PanelState, placement: DockPlacement, size: f32, open: bool) -> Option<DockState> {
    Some(DockState::new(panel, placement, px(size), open))
}

/// The default layout as the build dumps it: the left split sized 600 and
/// 1400, and a tabbed bottom group.
fn base() -> DockAreaState {
    DockAreaState {
        version: Some(1),
        center: stack(Axis::Horizontal, &[0.], vec![tabs(&[ITEM_AREA_NAME], 0)]),
        left_dock: dock(
            stack(
                Axis::Vertical,
                &[600., 1400.],
                vec![tabs(&["R"], 0), tabs(&["D"], 0)],
            ),
            DockPlacement::Left,
            290.,
            true,
        ),
        right_dock: dock(tabs(&["T"], 0), DockPlacement::Right, 440., true),
        bottom_dock: dock(tabs(&["A", "T2"], 0), DockPlacement::Bottom, 150., true),
    }
}

fn left_sizes(state: &mut DockAreaState, sizes: &[f32]) {
    let dock = state.left_dock.take().unwrap();
    let mut panel = dock.panel().clone();
    panel.info = PanelInfo::stack(sizes.iter().map(|size| px(*size)).collect(), Axis::Vertical);
    state.left_dock = dock_of(panel, &dock, dock.size().as_f32(), dock.open());
}

fn dock_of(panel: PanelState, like: &DockState, size: f32, open: bool) -> Option<DockState> {
    dock(panel, like.placement(), size, open)
}

fn edit_left(state: &mut DockAreaState, f: impl FnOnce(&DockState) -> (f32, bool)) {
    let like = state.left_dock.take().unwrap();
    let (size, open) = f(&like);
    state.left_dock = dock_of(like.panel().clone(), &like, size, open);
}

/// Step 1 stops for the launch's own dump and for a measured one, and
/// goes on for every real change: a divider, a dock size, an open flag, a
/// tab, a panel in the centre.
#[test]
fn same_layout_tells_a_change_from_the_launch() {
    type Edit = fn(&mut DockAreaState);
    let rows: [(&str, Edit, bool); 11] = [
        ("the same dump", |_| {}, true),
        (
            "measured split sizes in the same proportion",
            |state| left_sizes(state, &[90., 210.]),
            true,
        ),
        (
            "a divider moved",
            |state| left_sizes(state, &[100., 200.]),
            false,
        ),
        (
            "a dock one pixel wider",
            |state| edit_left(state, |dock| (dock.size().as_f32() + 1., dock.open())),
            true,
        ),
        (
            "a dock two pixels wider",
            |state| edit_left(state, |dock| (dock.size().as_f32() + 2., dock.open())),
            false,
        ),
        (
            "a dock closed",
            |state| edit_left(state, |dock| (dock.size().as_f32(), false)),
            false,
        ),
        (
            "another tab active",
            |state| {
                let dock = state.bottom_dock.take().unwrap();
                state.bottom_dock = self::dock(
                    tabs(&["A", "T2"], 1),
                    DockPlacement::Bottom,
                    dock.size().as_f32(),
                    dock.open(),
                );
            },
            false,
        ),
        (
            "a panel in the centre",
            |state| state.center.children.push(tabs(&["R"], 0)),
            false,
        ),
        ("a dock removed", |state| state.right_dock = None, false),
        ("another version", |state| state.version = Some(2), false),
        (
            "panels reordered in a group",
            |state| {
                let dock = state.bottom_dock.take().unwrap();
                state.bottom_dock = self::dock(
                    tabs(&["T2", "A"], 0),
                    DockPlacement::Bottom,
                    dock.size().as_f32(),
                    dock.open(),
                );
            },
            false,
        ),
    ];
    for (what, edit, same) in rows {
        let mut changed = base();
        edit(&mut changed);
        assert_eq!(same_layout(&base(), &changed), same, "{what}");
        assert_eq!(same_layout(&changed, &base()), same, "{what}, reversed");
    }
}

/// R-6's second step reads the centre, and the Kit-opened rule reads the
/// open flags.
#[test]
fn the_centre_and_the_open_flags_are_read_from_a_dump() {
    assert!(centre_is_item_area(&base()));
    let mut dropped = base();
    dropped.center.children[0]
        .children
        .push(leaf("RepositoriesPanel"));
    assert!(!centre_is_item_area(&dropped));
    let mut beside = base();
    beside.center.children.push(tabs(&["RepositoriesPanel"], 0));
    assert!(!centre_is_item_area(&beside));
    let mut empty = base();
    empty.center.children.clear();
    assert!(!centre_is_item_area(&empty));

    let mut state = base();
    edit_left(&mut state, |dock| (dock.size().as_f32(), false));
    state.right_dock = None;
    assert!(!open_in(&state, Side::Left), "closed");
    assert!(!open_in(&state, Side::Right), "absent");
    assert!(open_in(&state, Side::Bottom), "open");
}
