//! The dock area around the item area: where each region is drawn and the
//! default layout.

use gpui_kit::{
    TestAppContext,
    component::dock::{DockAreaState, DockPlacement, DockState, PanelState},
    px,
    test::TestWindowExt,
};

use crate::desktop::{
    model::layout::{ITEM_AREA_NAME, PanelKind},
    test_support::fixtures::Fixture,
    view::harness::Shell,
    view::workspace::{REGIONS, Region},
};

const OPERATION_AREA: &str = "operation-area";

/// The groups of a dump's dock, top to bottom or left to right: each as its
/// panels' names.
fn groups(dock: &Option<DockState>) -> Vec<Vec<String>> {
    fn walk(node: &PanelState, out: &mut Vec<Vec<String>>) {
        if node.panel_name == "TabPanel" {
            out.push(node.children.iter().map(|c| c.panel_name.clone()).collect());
        } else {
            node.children.iter().for_each(|child| walk(child, out));
        }
    }
    let mut out = Vec::new();
    if let Some(dock) = dock {
        walk(dock.panel(), &mut out);
    }
    out
}

/// Every region of the shell is drawn by a panel of the dock area, each in
/// its own group, and the Workspace's fixed row is gone. This is U-4: a
/// panel's render updates the Workspace to build its body.
#[gpui_kit::test]
fn each_panel_draws_its_region_inside_the_dock(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let area = shell
        .workspace
        .read_with(cx, |ws, _| ws.dock_area().clone());
    // The node of the group holding each panel, by the panel's name.
    let group_of = |name: &str, cx: &mut TestAppContext| {
        area.read_with(cx, |area, cx| {
            [
                DockPlacement::Left,
                DockPlacement::Right,
                DockPlacement::Bottom,
                DockPlacement::Center,
            ]
            .into_iter()
            .filter_map(|placement| area.layout(placement))
            .find_map(|tree| {
                tree.panels()
                    .find(|id| {
                        area.panel(*id)
                            .is_some_and(|panel| panel.panel_name(cx) == name)
                    })
                    .and_then(|id| tree.find_panel_node(id))
            })
            .unwrap_or_else(|| panic!("{name} is in a group"))
        })
    };
    let wanted: Vec<(&str, &str)> = vec![
        (Region::Repositories.id(), PanelKind::Repositories.name()),
        (Region::Documents.id(), PanelKind::Documents.name()),
        (Region::Content.id(), ITEM_AREA_NAME),
        (Region::Tickets.id(), PanelKind::Tickets.name()),
        (Region::Operations.id(), PanelKind::Activity.name()),
        (OPERATION_AREA, PanelKind::Activity.name()),
    ];
    assert_eq!(wanted.len(), REGIONS.len() + 1);
    let nodes: Vec<_> = wanted
        .iter()
        .map(|(_, name)| group_of(name, cx).as_u64())
        .collect();
    shell.frame(cx, |window, _| {
        for ((id, name), node) in wanted.iter().zip(&nodes) {
            let mut dock = window.within("dock-area");
            let mut group = dock.within(("panel-group", *node));
            // The operation area is a container with no observed facts of
            // its own: its scope exists when something inside it is drawn.
            if *id == OPERATION_AREA {
                group.within(*id);
            } else {
                assert!(
                    group.try_find(*id).is_some(),
                    "{id} is in {name}'s own group inside the dock"
                );
            }
        }
    });
    let distinct: std::collections::HashSet<_> = nodes.iter().collect();
    assert_eq!(distinct.len(), 5, "five panels, five groups: {nodes:?}");
}

/// P1 and U-6: the launch shows Wave decision 1's 1b.
#[gpui_kit::test]
fn the_default_layout_is_1b(cx: &mut TestAppContext) {
    let shell = Shell::open_as_launched(Fixture::new(), cx);
    // Two frames: the first lays the splits out, the second reads them.
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    let area = shell
        .workspace
        .read_with(cx, |ws, _| ws.dock_area().clone());
    let state: DockAreaState = area.read_with(cx, |area, cx| area.dump(cx));

    assert_eq!(
        groups(&state.left_dock),
        vec![vec!["RepositoriesPanel"], vec!["DocumentsPanel"]]
    );
    assert_eq!(groups(&state.right_dock), vec![vec!["TicketsPanel"]]);
    assert_eq!(groups(&state.bottom_dock), vec![vec!["ActivityPanel"]]);
    assert_eq!(
        groups(&Some(DockState::new(
            state.center.clone(),
            DockPlacement::Center,
            px(0.),
            true
        ))),
        vec![vec!["ItemAreaPanel"]]
    );

    let size = |dock: &Option<DockState>| dock.as_ref().map(|dock| dock.size().as_f32());
    assert_eq!(size(&state.left_dock), Some(290.));
    assert_eq!(size(&state.right_dock), Some(440.));
    assert_eq!(size(&state.bottom_dock), Some(150.));
    assert!(
        [&state.left_dock, &state.right_dock, &state.bottom_dock]
            .into_iter()
            .all(|dock| dock.as_ref().is_some_and(DockState::open))
    );

    // Repositories over Documents in proportion 0.6 : 1.4 (U-6).
    let left = state.left_dock.as_ref().unwrap();
    let sizes = left.panel().info.sizes().expect("the left dock is a split");
    let (top, bottom) = (sizes[0].as_f32(), sizes[1].as_f32());
    let total = top + bottom;
    assert!(total > 0.);
    assert!(
        (top - total * 0.3).abs() <= 1. && (bottom - total * 0.7).abs() <= 1.,
        "{top} over {bottom}"
    );

    let focused: Vec<Region> = shell.frame(cx, |window, _| {
        REGIONS
            .into_iter()
            .filter(|region| window.find(region.id()).focused() == Some(true))
            .collect()
    });
    assert_eq!(focused, vec![Region::Repositories]);
}
