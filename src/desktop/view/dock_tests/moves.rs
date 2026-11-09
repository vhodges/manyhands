//! Moves between docks and groups, by the menu and by drag (P8 to P11),
//! and the drop past the last tab.

use gpui_kit::component::Placement;
use gpui_kit::{
    TestAppContext,
    component::dock::{DockPlacement, InsertTarget},
    test::TestWindowExt,
};

use super::support::*;
use crate::desktop::{
    model::layout::{ITEM_AREA_NAME, PanelKind, Side},
    test_support::fixtures::Fixture,
    view::{
        dock::{accepted::centre_is_item_area, placement},
        harness::Shell,
        workspace::Region,
    },
};
use PanelKind::{Activity, Documents, Repositories, Tickets};

/// P8, P9 and P7: every panel docks to every side through the menu, one
/// step after another on one window, covering a target dock that is open,
/// closed or absent, and a source dock that is open or closed, left with
/// panels, with none on, or with none. `Layout::move_to` is the pure
/// model of a move, and each step holds it to what the Kit did.
#[gpui_kit::test]
fn each_panel_docks_to_each_side(cx: &mut TestAppContext) {
    use Side::{Bottom, Left, Right};
    enum Do {
        Move(PanelKind, Side),
        Close(Side),
        Off(PanelKind),
    }
    #[derive(Debug)]
    enum Dock {
        Gone,
        Closed,
        Open,
    }
    use Do::*;
    use Dock::*;
    // Each step, what it must leave of the docks, and a dock size to check
    // (Activity is 520 px in the harness).
    type Step = (Do, &'static [(Side, Dock)], Option<(Side, f32)>);
    let steps: [Step; 12] = [
        (Move(Documents, Right), &[(Left, Open), (Right, Open)], None),
        (
            Move(Documents, Bottom),
            &[(Right, Open), (Bottom, Open)],
            None,
        ),
        (Move(Documents, Left), &[(Left, Open), (Bottom, Open)], None),
        (Move(Tickets, Left), &[(Right, Gone), (Left, Open)], None),
        (Move(Tickets, Right), &[(Right, Open)], Some((Right, 440.))),
        (Move(Activity, Left), &[(Bottom, Gone)], None),
        (
            Move(Activity, Bottom),
            &[(Bottom, Open)],
            Some((Bottom, 520.)),
        ),
        (Close(Right), &[(Right, Closed)], None),
        (Move(Repositories, Right), &[(Right, Open)], None),
        (Off(Tickets), &[(Right, Open)], None),
        (Move(Repositories, Left), &[(Right, Closed)], None),
        (
            Move(Tickets, Bottom),
            &[(Right, Gone), (Bottom, Open)],
            None,
        ),
    ];
    let shell = Shell::open(Fixture::new(), cx);
    let area = area(&shell, cx);
    for (n, (step, docks, size)) in steps.into_iter().enumerate() {
        let what = format!("step {}", n + 1);
        let mut oracle = layout(&shell, cx);
        match step {
            Move(kind, side) => {
                oracle.move_to(kind, side);
                dock_panel(&shell, kind, side, cx);
                let layout = layout(&shell, cx);
                assert!(layout.is_displayed(kind), "{what}: {kind:?} is revealed");
                assert!(layout.is_open(side), "{what}: {side:?} is open");
                for s in Side::ALL {
                    assert_eq!(
                        layout.groups(s),
                        oracle.groups(s),
                        "{what}: {s:?}, the model's move"
                    );
                    assert_eq!(
                        layout.is_shown(s),
                        oracle.is_shown(s),
                        "{what}: {s:?} shown"
                    );
                }
                for k in PanelKind::ALL {
                    assert_eq!(layout.is_on(k), oracle.is_on(k), "{what}: {k:?} on");
                }
            }
            Close(side) => toggle_dock(&shell, side, cx),
            Off(kind) => toggle_panel(&shell, kind, cx),
        }
        for (side, dock) in docks {
            let at = placement(*side);
            let (has, open) = area.read_with(cx, |a, _| (a.has_dock(at), a.is_dock_open(at)));
            let seen = match (has, open) {
                (false, _) => Gone,
                (true, false) => Closed,
                (true, true) => Open,
            };
            assert_eq!(
                format!("{seen:?}"),
                format!("{dock:?}"),
                "{what}: the {side:?} dock"
            );
        }
        if let Some((side, wanted)) = size {
            let size = area.read_with(cx, |a, _| a.dock_size(placement(side)).unwrap());
            assert!(
                (size.as_f32() - wanted).abs() <= 1.,
                "{what}: {side:?} is {size:?}, its last size"
            );
        }
        assert_agreement(&shell, &what, cx);
    }

    // P9: the panel's own side changes nothing.
    let (before, saved) = (layout(&shell, cx), saves(&shell, cx));
    dock_panel(&shell, Tickets, Bottom, cx);
    assert!(layout(&shell, cx) == before, "the layout is unchanged");
    assert_eq!(saves(&shell, cx), saved, "nothing was saved");
}

/// P10: a panel dropped on the item area is put back, with the docks that
/// were closed still closed, and nothing is saved. The Kit has no drop
/// predicate (fact 10), so the area's `LayoutChanged` for the drop and the
/// reload it causes are both seen. A drop commits `move_panel` with a tab
/// or a split target; one row drags the title for real.
#[gpui_kit::test]
fn a_drop_on_the_item_area_is_undone(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let area = area(&shell, cx);
    let (changes, _guard) = count_changes(&area, cx);

    enum Drop {
        Tabs(PanelKind),
        Split(PanelKind),
        Drag,
        Close(Side),
    }
    let rows = [
        ("dragged for real", Drop::Drag),
        ("a tab drop", Drop::Tabs(Tickets)),
        ("a split beside the item area", Drop::Split(Activity)),
        ("a tab drop with Left closed", Drop::Close(Side::Left)),
    ];
    for (what, drop) in rows {
        // A reload mints new node ids, so the target is looked up each time.
        let centre = item_area_group(&area, cx);
        let (before, saved, seen) = (dump(&shell, cx), saves(&shell, cx), changes.get());
        match drop {
            Drop::Tabs(kind) | Drop::Split(kind) => {
                let id = panel_id(&area, kind, cx);
                let split = matches!(drop, Drop::Split(_));
                edit_area(&shell, cx, |area, window, cx| {
                    let target = if split {
                        InsertTarget::Split {
                            node: centre,
                            placement: Placement::Right,
                            size: None,
                        }
                    } else {
                        InsertTarget::Tabs {
                            node: centre,
                            ix: None,
                            activate: true,
                        }
                    };
                    area.move_panel(id, target, window, cx)
                });
            }
            Drop::Drag => {
                let node = group_of(&area, Repositories, cx).as_u64();
                shell.frame(cx, |window, cx| {
                    window.drag_to(("panel-title", node), Region::Content.id(), cx)
                });
                shell.run(cx);
            }
            Drop::Close(side) => {
                toggle_dock(&shell, side, cx);
                let (closed, saved) = (shape(&dump(&shell, cx)), saves(&shell, cx));
                assert!(!closed[0].as_ref().unwrap().0);
                let id = panel_id(&area, Tickets, cx);
                edit_area(&shell, cx, |area, window, cx| {
                    area.move_panel(
                        id,
                        InsertTarget::Tabs {
                            node: centre,
                            ix: None,
                            activate: true,
                        },
                        window,
                        cx,
                    )
                });
                assert!(
                    changes.get() >= seen + 3,
                    "{what}: the close, the drop and the reload"
                );
                assert_eq!(shape(&dump(&shell, cx)), closed, "{what}");
                assert_eq!(saves(&shell, cx), saved, "{what}: nothing saved");
                assert!(centre_is_item_area(&dump(&shell, cx)), "{what}");
                assert_agreement(&shell, what, cx);
                continue;
            }
        }
        let after = dump(&shell, cx);
        assert!(
            changes.get() >= seen + 2,
            "{what}: the drop and the reload were seen"
        );
        assert_eq!(shape(&after), shape(&before), "{what}");
        assert!(
            centre_is_item_area(&after),
            "{what}: the centre holds only {ITEM_AREA_NAME}"
        );
        assert_eq!(saves(&shell, cx), saved, "{what}: nothing was saved");
        assert_agreement(&shell, what, cx);
    }
}

/// P11: a panel dragged into another dock's group is kept: the arrangement
/// follows, it is revealed, and the dock it left is gone.
#[gpui_kit::test]
fn a_panel_moved_into_a_tab_group_is_kept(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let saved = saves(&shell, cx);
    toggle_panel(&shell, Tickets, cx);
    assert!(!layout(&shell, cx).is_on(Tickets));

    put_in_group(&shell, Tickets, Repositories, false, cx);

    let layout = layout(&shell, cx);
    let left = layout.groups(Side::Left);
    assert_eq!(left[0].panels, [Repositories, Tickets]);
    assert!(layout.is_on(Tickets), "revealed by its new side (R-36)");
    assert!(layout.is_displayed(Tickets) && !layout.is_displayed(Repositories));
    assert!(!layout.has_panels(Side::Right));
    assert!(!area(&shell, cx).read_with(cx, |a, _| a.has_dock(DockPlacement::Right)));
    assert!(shell.present(Region::Tickets.id(), cx));
    assert!(saves(&shell, cx) > saved);
    assert_agreement(&shell, "Tickets in Repositories' group", cx);
    // Completed in Task 9: the file holds it, and `Shell::relaunch` restores it.
}

/// A panel dropped past the last tab of a group that holds a hidden panel
/// lands after the last tab (Task 4c's review).
#[gpui_kit::test]
fn a_drop_past_the_last_tab_lands_after_it(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    toggle_panel(&shell, Documents, cx);
    put_in_group(&shell, Documents, Repositories, false, cx);
    put_in_group(&shell, Tickets, Repositories, false, cx);
    let names =
        |cx: &mut TestAppContext| shape(&dump(&shell, cx))[0].as_ref().unwrap().1[0].0.clone();
    assert_eq!(
        names(cx),
        ["RepositoriesPanel", "DocumentsPanel", "TicketsPanel"]
    );
    assert!(
        !layout(&shell, cx).is_on(Documents),
        "the middle tab is hidden"
    );

    let area = area(&shell, cx);
    let node = group_of(&area, Repositories, cx).as_u64();
    shell.frame(cx, |window, cx| {
        window
            .within(("panel-group", node))
            .drag_to("panel-tab-repositories", "panel-tabs-end", cx)
    });
    shell.run(cx);

    assert_eq!(
        names(cx),
        ["DocumentsPanel", "TicketsPanel", "RepositoriesPanel"]
    );
    assert_agreement(&shell, "Repositories moved past the last tab", cx);
}
