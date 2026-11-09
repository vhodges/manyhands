//! The closed bottom strip, a dock the Kit opens, and focus leaving a
//! panel that is not displayed (P12, P33, P34).

use gpui_kit::{TestAppContext, component::dock::DockPlacement};

use super::support::*;
use crate::desktop::{
    model::layout::{PanelKind, Side},
    test_support::fixtures::Fixture,
    view::{harness::Shell, workspace::Region},
};
use PanelKind::{Activity, Documents, Repositories, Tickets};

/// P33: a closed bottom dock names every panel in it, whether it is one
/// group, split side by side or has every panel off, and a click on a name
/// reveals that panel, which opens the dock.
#[gpui_kit::test]
fn the_closed_strip_names_and_reveals_every_panel(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let area = area(&shell, cx);
    let open = |cx: &mut TestAppContext| {
        area.read_with(cx, |area, _| area.is_dock_open(DockPlacement::Bottom))
    };

    // One group.
    toggle_dock(&shell, Side::Bottom, cx);
    assert!(!open(cx));
    assert!(
        shell.present("panel-tab-activity", cx),
        "the strip names Activity"
    );
    assert!(
        !shell.present(Region::Operations.id(), cx),
        "and draws no body"
    );
    shell.click("panel-tab-activity", cx);
    assert!(open(cx));
    assert!(layout(&shell, cx).is_displayed(Activity));
    assert!(shell.present(Region::Operations.id(), cx));
    assert_eq!(
        focus(&shell, cx),
        Some(Region::Operations),
        "a tab click focuses it"
    );
    assert_agreement(&shell, "one group", cx);

    // Split side by side.
    dock_panel(&shell, Tickets, Side::Bottom, cx);
    toggle_dock(&shell, Side::Bottom, cx);
    assert!(!open(cx));
    for tab in ["panel-tab-activity", "panel-tab-tickets"] {
        assert!(shell.present(tab, cx), "{tab} is on the strip");
    }
    shell.click("panel-tab-tickets", cx);
    assert!(open(cx));
    assert!(layout(&shell, cx).is_displayed(Tickets));
    assert!(
        layout(&shell, cx).is_displayed(Activity),
        "what was on stays on"
    );
    assert_agreement(&shell, "split side by side", cx);

    // Every panel off.
    toggle_panel(&shell, Activity, cx);
    toggle_panel(&shell, Tickets, cx);
    assert!(!open(cx));
    for tab in ["panel-tab-activity", "panel-tab-tickets"] {
        assert!(
            shell.present(tab, cx),
            "{tab} is on the strip, though it is off"
        );
    }
    assert_agreement(&shell, "every panel off", cx);
    shell.click("panel-tab-activity", cx);
    assert!(open(cx));
    assert!(layout(&shell, cx).is_displayed(Activity));
    assert!(
        !layout(&shell, cx).is_on(Tickets),
        "only the named panel is revealed"
    );
    assert_agreement(&shell, "Activity revealed from the strip", cx);
}

/// P34: a dock the Kit opens, as dragging the closed strip's edge does, is
/// the dock toggle: the layout shows it, and a dock with every panel off
/// has them all on. A dock left holding only panels that are off is not
/// mistaken for one the Kit opened.
#[gpui_kit::test]
fn opening_the_strip_by_its_edge_counts_as_its_toggle(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let kit_toggle = |cx: &mut TestAppContext| {
        edit_area(&shell, cx, |area, window, cx| {
            area.toggle_dock(DockPlacement::Bottom, window, cx)
        })
    };

    toggle_dock(&shell, Side::Bottom, cx);
    assert!(!layout(&shell, cx).is_open(Side::Bottom));
    let saved = saves(&shell, cx);
    kit_toggle(cx);
    assert!(layout(&shell, cx).is_open(Side::Bottom), "the dock toggle");
    assert!(layout(&shell, cx).is_displayed(Activity));
    assert!(saves(&shell, cx) > saved);
    assert_agreement(&shell, "opened by the Kit", cx);

    toggle_panel(&shell, Activity, cx);
    assert!(!layout(&shell, cx).is_on(Activity));
    kit_toggle(cx);
    assert!(
        layout(&shell, cx).is_on(Activity),
        "every panel in it on (decision 6)"
    );
    assert!(layout(&shell, cx).is_open(Side::Bottom));
    assert_agreement(&shell, "opened with every panel off", cx);

    // Left keeps Documents, off, when Repositories is dragged away.
    toggle_panel(&shell, Documents, cx);
    put_in_group(&shell, Repositories, Tickets, false, cx);
    let layout = layout(&shell, cx);
    assert!(!layout.is_on(Documents), "nothing turned it on");
    assert!(!layout.is_open(Side::Left));
    assert!(!area(&shell, cx).read_with(cx, |a, _| a.is_dock_open(DockPlacement::Left)));
    assert_agreement(&shell, "Left holds only Documents, off", cx);
}

/// P12: focus in a panel that stops being displayed goes to the item area:
/// its toggle, its dock (the bottom one too), another tab revealed, a click
/// on the closed strip outside its tabs. A tab click focuses the panel it
/// selects, and a refused operation moves no focus.
#[gpui_kit::test]
fn focus_leaves_a_panel_that_is_not_displayed(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let focus_on = |region: Region, cx: &mut TestAppContext| {
        act(&shell, cx, |ws, window, cx| {
            ws.focus_region(region, window, cx)
        });
        shell.frame(cx, |_, _| ());
        assert_eq!(
            focus(&shell, cx),
            Some(region),
            "setup: {region:?} has focus"
        );
    };

    focus_on(Region::Tickets, cx);
    toggle_panel(&shell, Tickets, cx);
    assert_eq!(focus(&shell, cx), Some(Region::Content), "its toggle");
    toggle_panel(&shell, Tickets, cx);

    focus_on(Region::Repositories, cx);
    toggle_dock(&shell, Side::Left, cx);
    assert_eq!(focus(&shell, cx), Some(Region::Content), "its dock");
    toggle_dock(&shell, Side::Left, cx);

    focus_on(Region::Operations, cx);
    toggle_dock(&shell, Side::Bottom, cx);
    assert_eq!(focus(&shell, cx), Some(Region::Content), "the bottom dock");

    // A click on the closed strip, outside its tabs.
    focus_on(Region::Repositories, cx);
    let node = group_of(&area(&shell, cx), Activity, cx).as_u64();
    // Focus listeners run only in an active window, which a test window is
    // not until it is activated.
    shell.frame(cx, |window, _| window.activate_window());
    shell.run(cx);
    shell.click(("panel-strip", node), cx);
    shell.frame(cx, |_, _| ());
    let seen = focus(&shell, cx);
    assert!(
        seen == Some(Region::Content),
        "focus left the strip's hidden panel for the item area: {seen:?}"
    );
    toggle_dock(&shell, Side::Bottom, cx);

    // Another tab revealed by its toggle.
    put_in_group(&shell, Documents, Repositories, false, cx);
    focus_on(Region::Repositories, cx);
    toggle_panel(&shell, Documents, cx);
    assert_eq!(
        focus(&shell, cx),
        Some(Region::Content),
        "another tab selected"
    );

    // A tab click focuses the panel it selects. The window is active, so
    // the focus move is seen by the hook, which must find the tab already
    // selected in the layout.
    shell.click_settled("panel-tab-repositories", cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(focus(&shell, cx), Some(Region::Repositories), "a tab click");

    // A refused operation moves no focus.
    let saved = saves(&shell, cx);
    dock_panel(&shell, Repositories, Side::Left, cx);
    assert_eq!(
        focus(&shell, cx),
        Some(Region::Repositories),
        "a refused move"
    );
    assert_eq!(saves(&shell, cx), saved);
}

/// The terms of the rule that counts a dock the Kit opened as its toggle:
/// a dock the layout closed stays closed through an edit elsewhere, and a
/// dock the shell opens is not toggled again when the same flush moves the
/// panel that was keeping it open.
#[gpui_kit::test]
fn a_kit_edit_elsewhere_does_not_open_a_closed_dock(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let left_open = |cx: &mut TestAppContext| {
        area(&shell, cx).read_with(cx, |a, _| a.is_dock_open(DockPlacement::Left))
    };

    // Left holds Repositories, off, and Documents, on; the person closes it.
    toggle_panel(&shell, Repositories, cx);
    toggle_dock(&shell, Side::Left, cx);
    assert!(!layout(&shell, cx).is_shown(Side::Left));

    // A drag elsewhere: the Kit's dump says Left is closed, the accepted
    // state says so, and the layout too.
    put_in_group(&shell, Tickets, Activity, false, cx);
    let after = layout(&shell, cx);
    assert!(!after.is_shown(Side::Left), "the edit did not show Left");
    assert!(!after.is_open(Side::Left) && !left_open(cx));
    assert!(!after.is_on(Repositories));
    assert_agreement(&shell, "an edit elsewhere", cx);

    // The shell opens Left, and in the same flush a drag takes Documents
    // out: Left is left holding only Repositories, off, so it closes, and
    // the Kit's open flag is not taken for the person's toggle, which would
    // turn Repositories on.
    let area = area(&shell, cx);
    let id = panel_id(&area, Documents, cx);
    let node = group_of(&area, Activity, cx);
    shell.frame(cx, |window, cx| {
        shell
            .workspace
            .update(cx, |ws, cx| ws.toggle_dock(Side::Left, window, cx));
        area.update(cx, |area, cx| {
            area.move_panel(
                id,
                gpui_kit::component::dock::InsertTarget::Tabs {
                    node,
                    ix: None,
                    activate: false,
                },
                window,
                cx,
            )
        });
    });
    shell.run(cx);
    let after = layout(&shell, cx);
    assert!(!after.is_on(Repositories), "nothing turned it on");
    assert!(!after.is_open(Side::Left) && !left_open(cx));
    assert_agreement(&shell, "Left opened as its panel left", cx);
}
