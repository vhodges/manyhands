//! The edges of saving (Part G; R-19, R-21, R-37): what the quit writes and
//! what it leaves alone, what the file carries, the notice-less cases, and
//! the clamp of a dock the Kit opens.

use gpui_kit::{
    TestAppContext,
    component::dock::{DockAreaState, DockPlacement, InsertTarget},
    px,
};

use super::{
    restore_tests::{elapse, file, in_file, seed},
    tests::support::{
        act, area, dump, edit_area, item_area_group, layout, panel_id, saves, toggle_panel,
    },
};
use crate::desktop::{
    model::layout::{Layout, PanelKind, Restore, Side, encode, restore, state_for},
    services::layout::MAX_FILE_BYTES,
    test_support::fixtures::Fixture,
    view::harness::Shell,
};
use PanelKind::{Documents, Tickets};

/// P37: a launch with no file saves nothing, and neither does a quit after
/// it; the same holds for a launch from a file nothing changes.
#[gpui_kit::test]
fn a_launch_that_changes_nothing_writes_nothing_even_at_quit(cx: &mut TestAppContext) {
    let shell = Shell::open_as_launched(Fixture::new(), cx);
    assert_eq!(saves(&shell, cx), 0, "no file, no save");
    assert!(file(&shell).is_none());
    elapse(&shell, cx);
    assert!(file(&shell).is_none(), "the delay saved nothing");
    shell.keys("secondary-q", cx);
    assert!(file(&shell).is_none(), "nor did the quit");

    // From a file: the quit leaves its bytes as they were.
    let mut kept = Layout::default();
    kept.toggle_panel(Tickets);
    let bytes = seed(&shell.fixture, &kept, [290., 440., 150.]);
    let shell = shell.relaunch(cx);
    assert_eq!(saves(&shell, cx), 0);
    shell.keys("secondary-q", cx);
    assert_eq!(file(&shell), Some(bytes), "the quit left the file alone");
}

/// A write that failed leaves the change unwritten, so the quit tries it
/// again (K-14).
#[gpui_kit::test]
fn a_failed_write_is_tried_again_at_quit(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let store = shell.fixture.layout_store();
    store.fail_writes_for_testing(true);
    toggle_panel(&shell, Documents, cx);
    elapse(&shell, cx);
    assert!(shell.present("layout-save-notice", cx));
    assert!(file(&shell).is_none(), "the write failed");

    store.fail_writes_for_testing(false);
    shell.keys("secondary-q", cx);
    let shell = shell.relaunch(cx);
    assert!(!layout(&shell, cx).is_on(Documents), "the quit wrote it");
}

/// The file carries the area as it is when it is written, not as it was
/// when the shell last accepted it: a dock moved by less than the
/// tolerance is not accepted, and is not lost.
#[gpui_kit::test]
fn a_dock_nudged_under_the_tolerance_is_saved(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    toggle_panel(&shell, Documents, cx);
    let size = |state: &DockAreaState| state.left_dock.as_ref().unwrap().size().as_f32();
    let before = size(&dump(&shell, cx));
    edit_area(&shell, cx, |area, window, cx| {
        area.set_dock_size(DockPlacement::Left, px(before + 0.8), window, cx)
    });
    let accepted = shell
        .workspace
        .read_with(cx, |workspace, _| size(workspace.dock_accepted()));
    assert!(
        (accepted - before).abs() < 0.1,
        "the nudge was not accepted"
    );

    elapse(&shell, cx);
    let kept = in_file(&shell).size(Side::Left);
    assert!(
        (kept - (before + 0.8)).abs() < 0.1,
        "the file has the area's size {kept}, not the accepted {accepted}"
    );
}

/// A drop on the item area that the shell has not undone yet is not what
/// the quit writes: the accepted state is.
#[gpui_kit::test]
fn a_drop_on_the_item_area_is_not_written(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    toggle_panel(&shell, Documents, cx);
    let area = area(&shell, cx);
    let (id, node) = (panel_id(&area, Tickets, cx), item_area_group(&area, cx));
    act(&shell, cx, |workspace, window, cx| {
        area.update(cx, |area, cx| {
            let target = InsertTarget::Tabs {
                node,
                ix: None,
                activate: false,
            };
            area.move_panel(id, target, window, cx)
        });
        // The event that undoes the drop is handled when this update ends.
        workspace.quit_now(cx);
    });
    let kept = in_file(&shell);
    assert_eq!(
        kept.side_of(Tickets),
        Some(Side::Right),
        "not in the centre"
    );
    assert!(!kept.is_on(Documents), "the change before it is kept");
}

/// R-37: a dock the Kit opens (its edge dragged) is scaled to fit as one
/// the shell opens is. `toggle_dock` stands in for the drag.
#[gpui_kit::test]
fn a_dock_opened_by_its_edge_is_clamped(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let mut wanted = Layout::default();
    wanted.toggle_dock(Side::Bottom);
    seed(&fixture, &wanted, [290., 440., 700.]);
    let shell = Shell::open_as_launched(fixture, cx);
    let height = area(&shell, cx).read_with(cx, |area, _| area.bounds().size.height.as_f32());

    edit_area(&shell, cx, |area, window, cx| {
        area.toggle_dock(DockPlacement::Bottom, window, cx)
    });
    assert!(layout(&shell, cx).is_open(Side::Bottom), "its dock toggle");
    let bottom = dump(&shell, cx).bottom_dock.unwrap().size().as_f32();
    assert!(
        (height - bottom - 200.).abs() <= 1.,
        "the item area keeps 200 px: {bottom} in {height}"
    );
    assert!((layout(&shell, cx).size(Side::Bottom) - bottom).abs() <= 1.);
}

/// A file over the cap is not a layout: the default, with the notice. One
/// exactly at it is read.
#[gpui_kit::test]
fn a_file_over_the_cap_restores_the_default_and_says_so(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let layout = Layout::default();
    let mut bytes = encode(&layout, &state_for(&layout, [290., 440., 150.]));
    bytes.resize(MAX_FILE_BYTES, b' ');
    let store = fixture.layout_store();
    store.write_now(&bytes).unwrap();
    assert!(
        matches!(restore(store.read()), Restore::Saved(_)),
        "a file at the cap is read"
    );
    bytes.push(b' ');
    store.write_now(&bytes).unwrap();
    assert!(store.read().is_err(), "a file over the cap is not");

    let shell = Shell::open_as_launched(fixture, cx);
    assert!(shell.present("layout-restore-notice", cx));
}

/// R-21: a layout store that did not open restores the default with the
/// notice and turns saving off: nothing is written, and no save notice
/// follows.
#[gpui_kit::test]
fn a_store_that_did_not_open_restores_the_default_and_saves_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new().without_layout_store();
    let data = fixture.data_dir().join("layout");
    let shell = Shell::open_as_launched(fixture, cx);
    assert!(shell.present("layout-restore-notice", cx));
    toggle_panel(&shell, Documents, cx);
    assert_eq!(saves(&shell, cx), 1, "the change asked for a save");
    elapse(&shell, cx);
    shell.keys("secondary-q", cx);
    assert!(!shell.present("layout-save-notice", cx), "saving is off");
    assert!(!data.exists(), "nothing was made or written");
}
