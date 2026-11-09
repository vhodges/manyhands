//! Hiding and showing panels and docks: the layout and the area stay in
//! agreement, and nothing is rebuilt (P2 to P7, P13, P30).

use gpui_kit::{TestAppContext, component::dock::DockPlacement, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use super::support::*;
use crate::desktop::{
    model::{
        layout::{PanelKind, Side},
        navigation::OpenTarget,
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        workspace::Region,
    },
};
use PanelKind::{Activity, Documents, Repositories, Tickets};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";

/// What a step must leave: each dock's open state (left, right, bottom),
/// and the panels on screen (Repositories, Tickets, Documents, Activity),
/// which the layout says and the frame draws.
fn expect(
    shell: &Shell,
    what: &str,
    open: [bool; 3],
    on_screen: [bool; 4],
    cx: &mut TestAppContext,
) {
    let layout = layout(shell, cx);
    assert_eq!(
        Side::ALL.map(|side| layout.is_open(side)),
        open,
        "{what}: open"
    );
    assert_eq!(displayed(shell, cx), on_screen, "{what}: displayed");
    assert_eq!(drawn(shell, cx), on_screen, "{what}: drawn");
    assert_agreement(shell, what, cx);
}

/// Hiding Documents leaves Repositories the dock; hiding both leaves the
/// dock nothing visible, and it closes.
#[gpui_kit::test]
fn hiding_a_panel_gives_its_space_to_its_dock(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let area = area(&shell, cx);
    let saved = saves(&shell, cx);

    toggle_panel(&shell, Documents, cx);
    assert!(
        !shell.present(Region::Documents.id(), cx),
        "its region is gone"
    );
    assert!(shell.present(Region::Repositories.id(), cx));
    assert!(
        !visible(&shell, Documents, cx),
        "the Kit is told it is hidden"
    );
    assert!(visible(&shell, Repositories, cx));
    assert!(
        !area.read_with(cx, |area, cx| area.is_empty(DockPlacement::Left, cx)),
        "the dock still holds a visible panel, which takes its slot"
    );
    assert!(area.read_with(cx, |area, _| area.is_dock_open(DockPlacement::Left)));
    assert!(saves(&shell, cx) > saved, "a save was scheduled");
    assert_agreement(&shell, "Documents hidden", cx);

    toggle_panel(&shell, Repositories, cx);
    assert!(area.read_with(cx, |area, cx| area.is_empty(DockPlacement::Left, cx)));
    assert!(!area.read_with(cx, |area, _| area.is_dock_open(DockPlacement::Left)));
}

/// Panel toggles, and the drags that change what a dock holds, leave the
/// layout and the area in agreement: docks close and open (the bottom one
/// keeps its strip), the next tab that is on is selected, a tab behind
/// another is revealed, and a drag that leaves only panels that are off
/// closes the dock (P3, P4, P13, Part B).
#[gpui_kit::test]
fn panel_toggles_keep_the_area_and_the_layout_in_agreement(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let t = true;
    let f = false;
    expect(&shell, "default", [t, t, t], [t, t, t, t], cx);

    toggle_panel(&shell, Documents, cx);
    expect(&shell, "Documents hidden", [t, t, t], [t, t, f, t], cx);

    toggle_panel(&shell, Repositories, cx);
    expect(
        &shell,
        "the last panel on in Left hidden",
        [f, t, t],
        [f, t, f, t],
        cx,
    );

    toggle_panel(&shell, Repositories, cx);
    expect(&shell, "Repositories revealed", [t, t, t], [t, t, f, t], cx);

    toggle_panel(&shell, Activity, cx);
    expect(&shell, "Activity hidden", [t, t, f], [t, t, f, f], cx);
    assert!(
        shell.present("panel-tab-activity", cx),
        "the closed bottom dock keeps its strip, which names Activity"
    );

    toggle_panel(&shell, Activity, cx);
    expect(&shell, "Activity revealed", [t, t, t], [t, t, f, t], cx);

    toggle_panel(&shell, Documents, cx);
    expect(&shell, "Documents revealed", [t, t, t], [t, t, t, t], cx);

    // A group of three, built by drags: Repositories, Documents, Tickets.
    put_in_group(&shell, Documents, Repositories, false, cx);
    expect(
        &shell,
        "Documents behind Repositories",
        [t, t, t],
        [t, t, f, t],
        cx,
    );
    put_in_group(&shell, Tickets, Repositories, false, cx);
    expect(
        &shell,
        "Tickets in the group, Right gone",
        [t, f, t],
        [f, t, f, t],
        cx,
    );
    let active = |cx: &mut TestAppContext| shape(&dump(&shell, cx))[0].as_ref().unwrap().1[0].1;
    assert_eq!(
        shape(&dump(&shell, cx))[0].as_ref().unwrap().1,
        [(
            vec![
                "RepositoriesPanel".to_owned(),
                "DocumentsPanel".into(),
                "TicketsPanel".into()
            ],
            2
        )],
        "the panel moved in is revealed, so its tab is selected (R-36)"
    );

    // A panel on but behind another tab is revealed by its toggle.
    toggle_panel(&shell, Repositories, cx);
    expect(&shell, "Repositories revealed", [t, f, t], [t, f, f, t], cx);
    assert_eq!(active(cx), 0);

    // Hiding the active tab selects the next that is on.
    toggle_panel(&shell, Repositories, cx);
    expect(&shell, "the active tab hidden", [t, f, t], [f, f, t, t], cx);
    assert_eq!(active(cx), 1);

    toggle_panel(&shell, Tickets, cx);
    expect(&shell, "Tickets revealed", [t, f, t], [f, t, f, t], cx);
    assert_eq!(active(cx), 2);

    // A drag that leaves only a panel that is off closes the dock.
    put_in_group(&shell, Documents, Activity, false, cx);
    put_in_group(&shell, Tickets, Activity, false, cx);
    expect(
        &shell,
        "Left holds only Repositories, off",
        [f, f, t],
        [f, t, f, f],
        cx,
    );
    let area = area(&shell, cx);
    assert!(area.read_with(cx, |area, _| area.has_dock(DockPlacement::Left)));
    assert!(layout(&shell, cx).is_shown(Side::Left));
    assert_eq!(
        shape(&dump(&shell, cx))[2].as_ref().unwrap().1[0].1,
        2,
        "the last panel moved in is the one selected (R-36)"
    );
}

/// P5 and P30: a dock closed and opened again has the same panels, groups,
/// active tabs and sizes, and nothing in the area is rebuilt: focus on a
/// Kit button in the item area is kept, and so are the node ids.
#[gpui_kit::test]
fn a_dock_closed_and_opened_keeps_its_panels_groups_and_sizes(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.keys("secondary-3", cx);
    let focused = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window.try_find("item-edit").and_then(|edit| edit.focused()) == Some(true)
        })
    };
    let mut presses = 0;
    while !focused(&shell, cx) {
        presses += 1;
        assert!(presses < 6, "Tab never reached Edit");
        shell.keys("tab", cx);
    }
    let area = area(&shell, cx);
    // The groups' node ids: a `load` would mint new ones (fact 5).
    let nodes = |cx: &mut TestAppContext| {
        let mut ids: Vec<u64> = PanelKind::ALL
            .into_iter()
            .map(|kind| group_of(&area, kind, cx).as_u64())
            .collect();
        ids.extend(area.read_with(cx, |area, _| {
            [
                DockPlacement::Center,
                DockPlacement::Left,
                DockPlacement::Bottom,
            ]
            .map(|p| area.layout(p).unwrap().root().id().as_u64())
        }));
        ids
    };

    for side in [Side::Left, Side::Bottom] {
        let (before, ids, saved) = (dump(&shell, cx), nodes(cx), saves(&shell, cx));
        toggle_dock(&shell, side, cx);
        let closed = shape(&dump(&shell, cx));
        let ix = Side::ALL.iter().position(|s| *s == side).unwrap();
        assert!(!closed[ix].as_ref().unwrap().0, "{side:?} is closed");
        assert!(focused(&shell, cx), "{side:?} closed: focus is kept");
        toggle_dock(&shell, side, cx);

        let after = dump(&shell, cx);
        assert_eq!(
            shape(&after),
            shape(&before),
            "{side:?}: panels, groups, tabs"
        );
        let (a, b) = (sizes(&after), sizes(&before));
        assert_eq!(a.len(), b.len());
        assert!(
            a.iter().zip(&b).all(|(a, b)| (a - b).abs() <= 1.),
            "{side:?}: sizes {a:?} against {b:?}"
        );
        assert!(focused(&shell, cx), "{side:?} opened: focus is kept");
        assert_eq!(nodes(cx), ids, "{side:?}: nothing in the area was rebuilt");
        assert!(saves(&shell, cx) > saved, "{side:?}: a save was scheduled");
        // Completed in Task 9: the file holds the same layout.
    }
}

/// P6 and P7: a dock toggle on a dock with every panel off turns them all
/// on; one on a side with no dock changes nothing.
#[gpui_kit::test]
fn dock_toggles_follow_the_rules(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let t = true;
    let f = false;

    toggle_panel(&shell, Repositories, cx);
    toggle_panel(&shell, Documents, cx);
    expect(&shell, "Left, every panel off", [f, t, t], [f, t, f, t], cx);
    toggle_dock(&shell, Side::Left, cx);
    expect(
        &shell,
        "Left shown with every panel on",
        [t, t, t],
        [t, t, t, t],
        cx,
    );
    assert!(layout(&shell, cx).is_on(Repositories) && layout(&shell, cx).is_on(Documents));

    toggle_panel(&shell, Activity, cx);
    expect(
        &shell,
        "Bottom, every panel off",
        [t, t, f],
        [t, t, t, f],
        cx,
    );
    toggle_dock(&shell, Side::Bottom, cx);
    expect(
        &shell,
        "Bottom shown with Activity on",
        [t, t, t],
        [t, t, t, t],
        cx,
    );

    // An open dock hides without changing what is on, and shows again.
    toggle_dock(&shell, Side::Left, cx);
    expect(&shell, "Left hidden", [f, t, t], [f, t, f, t], cx);
    assert!(layout(&shell, cx).is_on(Repositories), "what is on is kept");
    toggle_dock(&shell, Side::Left, cx);
    expect(&shell, "Left shown again", [t, t, t], [t, t, t, t], cx);

    // A side with no panels has no dock and no toggle.
    put_in_group(&shell, Tickets, Repositories, false, cx);
    assert!(!area(&shell, cx).read_with(cx, |area, _| area.has_dock(DockPlacement::Right)));
    let (before, saved) = (layout(&shell, cx), saves(&shell, cx));
    toggle_dock(&shell, Side::Right, cx);
    assert!(layout(&shell, cx) == before, "the layout is unchanged");
    assert_eq!(saves(&shell, cx), saved, "nothing was saved");
    assert_agreement(&shell, "Right has no dock", cx);
}
