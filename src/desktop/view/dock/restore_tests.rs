//! The layout file (Part G): restoring it at launch, the fallback and its
//! notice, saving, the quit's write and the clamp (P23 to P25, P31 to P37).

use std::{collections::BTreeMap, fs, path::Path, time::Duration};

use gpui_kit::{TestAppContext, component::dock::DockPlacement, px};

use super::{
    LAYOUT_SAVE_DELAY,
    tests::support::{dock_panel, dump, edit_area, layout, saves, toggle_dock, toggle_panel},
};
use crate::desktop::{
    model::layout::{Layout, PanelKind, Restore, Side, encode, restore, state_for},
    services::{
        JobKind,
        requests::{Intent, Lane},
    },
    test_support::{fixtures::Fixture, snapshot::RepositorySnapshot},
    view::{
        harness::{Shell, wait_until},
        workspace::Region,
    },
};
use PanelKind::{Activity, Documents, Repositories, Tickets};

const SAVE_NOTICE: &str = "Your panel layout couldn't be saved.";

const RESTORE_NOTICE: &str =
    "Your panel layout couldn't be restored, so the default layout is back.";

/// Writes `layout` into the fixture's layout file, as a first session
/// would have left it.
fn seed(fixture: &Fixture, layout: &Layout, sizes: [f32; 3]) -> Vec<u8> {
    let bytes = encode(layout, &state_for(layout, sizes));
    fixture.layout_store().write_now(&bytes).unwrap();
    bytes
}

/// The file's bytes, if there is one.
fn file(shell: &Shell) -> Option<Vec<u8>> {
    shell.fixture.layout_store().read().unwrap()
}

/// Lets the debounce elapse and its write land.
pub(super) fn elapse(shell: &Shell, cx: &mut TestAppContext) {
    cx.executor()
        .advance_clock(LAYOUT_SAVE_DELAY + Duration::from_millis(1));
    shell.run(cx);
}

/// The layout the file holds.
pub(super) fn in_file(shell: &Shell) -> Layout {
    match restore(shell.fixture.layout_store().read()) {
        Restore::Saved(saved) => saved.layout,
        other => panic!("the file holds a layout: {other:?}"),
    }
}

/// The same groups, on flags, shown flags and sizes within 1 px.
pub(super) fn same(a: &Layout, b: &Layout, what: &str) {
    for kind in PanelKind::ALL {
        assert_eq!(a.is_on(kind), b.is_on(kind), "{what}: {kind:?} on");
    }
    for side in Side::ALL {
        assert_eq!(a.groups(side), b.groups(side), "{what}: {side:?} groups");
        assert_eq!(a.is_shown(side), b.is_shown(side), "{what}: {side:?} shown");
        assert!(
            (a.size(side) - b.size(side)).abs() <= 1.,
            "{what}: {side:?} size {} against {}",
            a.size(side),
            b.size(side)
        );
    }
}

/// P23, P24, P37: a valid file is restored as it was written, with nothing
/// saved by the launch, and the focus starts on the first stop when
/// Repositories is not displayed (R-9).
#[gpui_kit::test]
fn a_valid_file_restores_the_layout(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let mut wanted = Layout::default();
    wanted.move_to(Tickets, Side::Left);
    wanted.toggle_panel(Repositories);
    wanted.toggle_dock(Side::Bottom);
    let sizes = [300., 440., 180.];
    let bytes = seed(&fixture, &wanted, sizes);

    let shell = Shell::open_as_launched(fixture, cx);
    let layout = layout(&shell, cx);
    // The file's sizes are the layout's.
    for (side, size) in Side::ALL.into_iter().zip(sizes) {
        wanted.set_size(side, size);
    }
    same(&layout, &wanted, "the launch");
    assert!(!layout.is_on(Repositories) && !layout.is_open(Side::Bottom));
    assert!(!layout.has_panels(Side::Right));

    // The area says the same: the groups, the open flags, the left size.
    let state = dump(&shell, cx);
    assert!(state.right_dock.is_none());
    assert!(!state.bottom_dock.as_ref().unwrap().open());
    assert!(state.left_dock.as_ref().unwrap().open());
    assert!((state.left_dock.as_ref().unwrap().size().as_f32() - 300.).abs() <= 1.);
    // Focus starts on the first stop: Documents, since Repositories is off.
    assert_eq!(
        super::tests::support::focus(&shell, cx),
        Some(Region::Documents)
    );

    assert_eq!(saves(&shell, cx), 0, "the launch asked for no save");
    elapse(&shell, cx);
    assert_eq!(file(&shell), Some(bytes), "the file is as it was");
    assert_eq!(saves(&shell, cx), 0);
}

/// Everything in `dir`, by relative name.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        out.insert(name, fs::read(&path).unwrap_or_default());
    }
    out
}

/// The data directory's entry names, less the library's own index and
/// its sidecar files, which come and go as it reads.
fn entries(data: &Path) -> std::collections::BTreeSet<String> {
    fs::read_dir(data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with("manyhands"))
        .collect()
}

/// P24, P25: a file that cannot be read restores the default and says so;
/// the notice is not a need for attention, Dismiss removes it, and no other
/// store or repository changes.
#[gpui_kit::test]
fn a_file_that_cannot_be_read_restores_the_default_and_says_so(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.layout_store();
    fixture.draft_store();
    fixture.resolution_store();
    let data = fixture.data_dir().to_owned();
    // A directory named `layout.json` cannot be read on any platform.
    fs::create_dir(data.join("layout").join("layout.json")).unwrap();
    fs::write(data.join("drafts").join("sentinel"), b"drafts").unwrap();
    fs::write(data.join("resolutions").join("sentinel"), b"resolutions").unwrap();
    let before = (
        tree(&data.join("drafts")),
        tree(&data.join("resolutions")),
        entries(&data),
        RepositorySnapshot::take(&root),
    );

    let shell = Shell::open_as_launched(fixture, cx);
    same(
        &layout(&shell, cx),
        &Layout::default(),
        "the default layout",
    );
    assert_eq!(
        shell.label("layout-restore-notice", cx).as_deref(),
        Some(RESTORE_NOTICE)
    );
    assert!(
        !shell.present("footer-attention", cx),
        "the notice is not counted"
    );
    assert!(
        !shell.present("layout-save-notice", cx),
        "nothing was saved"
    );

    shell.click("layout-restore-dismiss", cx);
    assert!(!shell.present("layout-restore-notice", cx));
    assert!(!shell.present("footer-attention", cx));

    let after = (
        tree(&data.join("drafts")),
        tree(&data.join("resolutions")),
        entries(&data),
        RepositorySnapshot::take(&root),
    );
    assert_eq!(before, after, "no other store or repository changed");
}

/// P23: hide Documents, close the bottom dock, move Tickets left and size
/// the left dock; after the save delay the file holds it, and a relaunch
/// restores it.
#[gpui_kit::test]
fn the_layout_survives_a_restart(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    toggle_panel(&shell, Documents, cx);
    toggle_dock(&shell, Side::Bottom, cx);
    dock_panel(&shell, Tickets, Side::Left, cx);
    edit_area(&shell, cx, |area, window, cx| {
        area.set_dock_size(DockPlacement::Left, px(310.), window, cx)
    });
    elapse(&shell, cx);

    let path = shell.fixture.data_dir().join("layout").join("layout.json");
    assert!(path.is_file(), "the file is <data>/layout/layout.json");
    let before = layout(&shell, cx);
    same(&in_file(&shell), &before, "the file");
    assert!(!before.is_on(Documents) && !before.is_open(Side::Bottom));
    assert_eq!(before.side_of(Tickets), Some(Side::Left));
    assert!((before.size(Side::Left) - 310.).abs() <= 1.);

    let shell = shell.relaunch(cx);
    same(&layout(&shell, cx), &before, "the restart");
    assert_eq!(saves(&shell, cx), 0, "the restart asked for no save");
}

/// P31: a change, then a quit that exits at once with no clock advance,
/// is in the file at the next launch.
#[gpui_kit::test]
fn a_change_just_before_a_quit_that_exits_at_once_is_kept(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    toggle_panel(&shell, Documents, cx);
    let before = layout(&shell, cx);
    assert!(file(&shell).is_none(), "nothing was written yet");

    shell.keys("secondary-q", cx);
    assert_eq!(
        shell.workspace.read_with(cx, |ws, _| ws.quit_phase()),
        crate::desktop::view::quit::QuitPhase::Quit
    );
    let shell = shell.relaunch(cx);
    same(&layout(&shell, cx), &before, "the quit's write");
    assert!(!layout(&shell, cx).is_on(Documents));
}

/// P31: a change made while a quit waits for a job is kept when the job
/// finishes and the app exits.
#[gpui_kit::test]
async fn a_change_during_a_quit_wait_is_kept(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    // Activity is hidden, so the wait shows nothing.
    toggle_panel(&shell, Activity, cx);
    elapse(&shell, cx);
    assert!(!in_file(&shell).is_on(Activity));
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .services()
            .worker()
            .submit(Intent::IndexRefresh { root }, Lane::User, None, None)
    });
    held.wait_reached();
    shell.frame(cx, |window, cx| {
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.request_quit(window, cx))
    });
    shell.run(cx);
    assert!(
        matches!(
            shell.workspace.read_with(cx, |ws, _| ws.quit_phase()),
            crate::desktop::view::quit::QuitPhase::Finishing(_)
        ),
        "the quit waits"
    );

    // The wait may let the save delay run out. Its write is held, so the
    // quit's is the only one that can put the change in the file.
    let _hold = shell.fixture.layout_store().hold_writes_for_testing();
    toggle_panel(&shell, Activity, cx);
    assert!(layout(&shell, cx).is_displayed(Activity));
    held.release();
    wait_until(&shell.visual(cx), &shell.workspace, |workspace, _| {
        workspace.quit_phase() == crate::desktop::view::quit::QuitPhase::Quit
    })
    .await;

    let shell = shell.relaunch(cx);
    assert!(
        layout(&shell, cx).is_displayed(Activity),
        "Activity is open"
    );
}

/// P35: a background write still in flight when the quit writes loses to
/// it: the file holds the quit's newer layout.
#[gpui_kit::test]
fn a_stale_write_loses_to_the_quits(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let hold = shell.fixture.layout_store().hold_writes_for_testing();
    toggle_panel(&shell, Documents, cx);
    elapse(&shell, cx);
    assert!(file(&shell).is_none(), "the write is held");

    toggle_panel(&shell, Tickets, cx);
    shell.keys("secondary-q", cx);
    let quit = in_file(&shell);
    assert!(!quit.is_on(Documents) && !quit.is_on(Tickets), "the quit's");

    drop(hold);
    shell.run(cx);
    // The held write, released, found a newer one and skipped itself.
    let after = in_file(&shell);
    assert!(!after.is_on(Documents) && !after.is_on(Tickets));
    same(&after, &quit, "the file after the stale write");
}

/// P32: a failed write shows the save notice, which Dismiss removes and a
/// later failure shows again; a later successful write removes it.
#[gpui_kit::test]
fn a_failed_write_shows_the_save_notice(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let store = shell.fixture.layout_store();
    store.fail_writes_for_testing(true);
    toggle_panel(&shell, Documents, cx);
    assert!(!shell.present("layout-save-notice", cx), "nothing yet");
    elapse(&shell, cx);
    assert_eq!(
        shell.label("layout-save-notice", cx).as_deref(),
        Some(SAVE_NOTICE)
    );
    assert!(
        !shell.present("footer-attention", cx),
        "the notice is not counted"
    );
    assert!(!shell.present("layout-restore-notice", cx));

    shell.click("layout-save-dismiss", cx);
    assert!(!shell.present("layout-save-notice", cx));
    toggle_panel(&shell, Documents, cx);
    elapse(&shell, cx);
    assert!(shell.present("layout-save-notice", cx), "another failure");

    store.fail_writes_for_testing(false);
    toggle_panel(&shell, Documents, cx);
    elapse(&shell, cx);
    assert!(!shell.present("layout-save-notice", cx), "a write landed");
    assert!(file(&shell).is_some());
}

/// P36: saved docks too large for the window are scaled down together once
/// the area has bounds, so the item area keeps 320 px, and that is saved. A
/// closed oversized dock is scaled when it opens.
#[gpui_kit::test]
fn a_dock_too_large_for_the_window_is_clamped(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let mut wanted = Layout::default();
    wanted.toggle_dock(Side::Bottom);
    seed(&fixture, &wanted, [1000., 800., 700.]);

    let shell = Shell::open_as_launched(fixture, cx);
    let bounds = super::tests::support::area(&shell, cx).read_with(cx, |area, _| area.bounds());
    let (width, height) = (bounds.size.width.as_f32(), bounds.size.height.as_f32());
    assert!(width > 0. && height > 0., "the area has been laid out");
    let sizes = |cx: &mut TestAppContext| {
        let dump = dump(&shell, cx);
        [
            dump.left_dock.map(|dock| dock.size().as_f32()),
            dump.right_dock.map(|dock| dock.size().as_f32()),
            dump.bottom_dock.map(|dock| dock.size().as_f32()),
        ]
        .map(Option::unwrap)
    };

    let [left, right, bottom] = sizes(cx);
    assert!(
        (width - left - right - 320.).abs() <= 1.,
        "the item area keeps 320 px: {left} and {right} in {width}"
    );
    assert!(
        (left / right - 1000. / 800.).abs() < 0.01,
        "in proportion: {left} to {right}"
    );
    assert!(
        (bottom - 700.).abs() <= 1.,
        "a closed dock is not scaled yet"
    );
    assert_eq!(saves(&shell, cx), 1, "the clamp is one layout change");
    elapse(&shell, cx);
    let kept = in_file(&shell);
    assert!((kept.size(Side::Left) - left).abs() <= 1.);
    assert!((kept.size(Side::Right) - right).abs() <= 1.);

    // Opening the bottom dock scales it so the item area keeps 200 px.
    toggle_dock(&shell, Side::Bottom, cx);
    let [_, _, bottom] = sizes(cx);
    assert!(
        (height - bottom - 200.).abs() <= 1.,
        "the item area keeps 200 px: {bottom} in {height}"
    );
    elapse(&shell, cx);
    assert!((in_file(&shell).size(Side::Bottom) - bottom).abs() <= 1.);
}
