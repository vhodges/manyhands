//! The arranging journey (A2, A4, A10, A18): the person arranges the
//! workspace with the keyboard alone, the layout is kept over a restart, and
//! the worker is asked for nothing at any step (RFC 80-84).
//!
//! No click and no call into the workspace: Tab and Enter on the footer,
//! `menu` and the arrows for the dock menu, `secondary-j`, `secondary-4`.
//! The window is the application's own (`Shell::open_as_launched`, 1280 x
//! 840, Activity at its default 150 px), and, like every test window, it is
//! inactive: no key here needs an active window.
//!
//! The journey starts over a layout file that cannot be read (a directory
//! named `layout.json`), so both layout notices are met by keyboard as well:
//! Dismiss is reached by Tab, has its name and ring, and takes Enter or
//! Space. That directory makes every write fail, so the person's move is
//! to clear it (`fs::remove_dir`, standing for deleting the bad file) after
//! the save notice has been seen; the restart then reads the file the
//! journey wrote.

use std::fs;

use gpui_kit::{TestAppContext, test::TestWindowExt};

use super::{Walk, support::*};
use crate::desktop::{
    model::layout::{PanelKind, Side},
    test_support::fixtures::Fixture,
    view::{
        dock::restore_tests::{elapse, in_file, same},
        footer::{
            Control,
            tests::support::{read, saves},
        },
        harness::Shell,
        operations::DISMISS,
        workspace::Region,
    },
};

use PanelKind::{Activity, Documents, Tickets};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ0";

/// What the worker has been asked and told to read, the requests the app
/// has made and the tabs open, as one comparable text (D5's A17 reads).
fn sent(walk: &Walk, cx: &mut TestAppContext) -> String {
    format!(
        "jobs {:?}; reads {:?}; requests {:?}; tabs {:?}",
        walk.shell.backend.jobs_seen(),
        walk.shell.backend.reads_in_order(),
        walk.shell.requests(cx, |_| true),
        walk.shell.rows("tab-title", cx),
    )
}

/// Nothing was sent since `before`, and no tab opened, closed or reloaded.
fn quiet(walk: &Walk, before: &str, step: &str, cx: &mut TestAppContext) {
    assert_eq!(sent(walk, cx), before, "{step}: the worker was asked");
}

/// A layout notice and the ids of its parts.
#[derive(Clone, Copy)]
struct Notice {
    notice: &'static str,
    dismiss: &'static str,
    ring: &'static str,
}

const RESTORE: Notice = Notice {
    notice: "layout-restore-notice",
    dismiss: "layout-restore-dismiss",
    ring: "layout-restore-dismiss-ring",
};

const SAVE: Notice = Notice {
    notice: "layout-save-notice",
    dismiss: "layout-save-dismiss",
    ring: "layout-save-dismiss-ring",
};

/// Tab, from the item area, to the notice's Dismiss, which is named
/// "Dismiss" and draws a visible ring; then `key` on it dismisses the notice,
/// which is no need for attention.
fn dismiss(walk: &Walk, which: &Notice, key: &str, cx: &mut TestAppContext) {
    let Notice {
        notice,
        dismiss,
        ring,
    } = *which;
    assert!(walk.shell.present(notice, cx), "{notice} shows");
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", dismiss, cx);
    walk.shell.frame(cx, |window, _| {
        let button = window.find(dismiss);
        assert_eq!(button.focused(), Some(true), "{dismiss} has focus");
        assert_eq!(button.label(), Some(DISMISS), "{dismiss}: its name");
        assert!(
            window.try_find(ring).is_some_and(|ring| ring.visible()),
            "{dismiss} draws a ring"
        );
    });
    walk.shell.press(key, cx);
    assert!(!walk.shell.present(notice, cx), "{key} dismisses {notice}");
    assert!(
        !walk.shell.present("footer-attention", cx),
        "{notice} is not a need for attention"
    );
}

/// Documents hidden from the footer, Tickets moved left by its dock menu,
/// the bottom dock hidden and shown by `secondary-j`, Activity hidden from
/// the footer and brought back by `secondary-4`; the layout is in the file
/// after the save delay and is the layout after a restart. Nothing is sent
/// at any step.
#[gpui_kit::test]
fn arrange_the_workspace_by_keyboard_alone(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Ticket", "");
    fixture.refresh(&root);
    // A layout file that cannot be read, on any platform.
    fixture.layout_store();
    let bad = fixture.data_dir().join("layout").join("layout.json");
    fs::create_dir(&bad).unwrap();

    let walk = Walk::over(Shell::open_as_launched(fixture, cx), root);
    let at_launch = sent(&walk, cx);
    assert!(
        walk.shell.present("layout-restore-notice", cx),
        "the unreadable file is said"
    );

    // The restore notice, by keyboard.
    dismiss(&walk, &RESTORE, "enter", cx);
    walk.select_a_repository(cx);
    // Find, the Tickets list, and the ticket in a tab: a panel that reloaded
    // or closed it would show in what is sent and in the tabs below.
    walk.keys("secondary-p secondary-2 down enter", cx);
    assert_eq!(walk.shell.rows("tab-title", cx), ["Ticket"], "setup: a tab");
    let before = sent(&walk, cx);

    // Hide Documents from the footer.
    reach(&walk, Control::Panel(Documents), cx);
    assert_eq!(
        read(&walk.shell, Control::Panel(Documents), cx),
        Some(("Hide Documents".to_owned(), Some(true))),
        "Documents' toggle starts on"
    );
    walk.shell.press("enter", cx);
    assert!(
        !layout_of(&walk, cx).is_displayed(Documents),
        "Enter hides Documents"
    );
    assert!(
        !walk.shell.present(Region::Documents.id(), cx),
        "Documents is no longer drawn"
    );
    assert_eq!(
        read(&walk.shell, Control::Panel(Documents), cx),
        Some(("Show Documents".to_owned(), Some(false))),
        "Documents' toggle reads off"
    );
    on_control(&walk, Control::Panel(Documents), cx);
    quiet(&walk, &before, "Documents hidden", cx);

    // The write fails, since the file is a directory: the save notice, by
    // keyboard. The person then clears the bad file.
    elapse(&walk.shell, cx);
    dismiss(&walk, &SAVE, "space", cx);
    fs::remove_dir(&bad).unwrap();
    quiet(&walk, &before, "the save notice", cx);

    // Tickets to the left dock, by its dock menu.
    reach(&walk, Control::Panel(Tickets), cx);
    walk.keys("menu", cx);
    assert!(walk.shell.present("popup-menu", cx), "the menu opens");
    walk.keys("down enter", cx);
    assert!(!walk.shell.present("popup-menu", cx), "a choice closes it");
    let moved = layout_of(&walk, cx);
    assert_eq!(moved.side_of(Tickets), Some(Side::Left), "Tickets is left");
    assert!(moved.is_displayed(Tickets), "the move reveals Tickets");
    assert!(
        walk.shell.present(Region::Tickets.id(), cx),
        "Tickets is drawn"
    );
    on_control(&walk, Control::Panel(Tickets), cx);
    quiet(&walk, &before, "Tickets moved left", cx);

    // The bottom dock, hidden and shown.
    for open in [false, true] {
        walk.keys("secondary-j", cx);
        assert_eq!(
            layout_of(&walk, cx).is_open(Side::Bottom),
            open,
            "secondary-j: the bottom dock's open flag"
        );
        assert_eq!(
            walk.shell.present(Region::Operations.id(), cx),
            open,
            "secondary-j: Activity is drawn exactly when the dock is open"
        );
        quiet(&walk, &before, "secondary-j", cx);
    }

    // Activity hidden from the footer, then focused by its key, which
    // reveals it.
    reach(&walk, Control::Panel(Activity), cx);
    walk.shell.press("enter", cx);
    assert!(
        !layout_of(&walk, cx).is_displayed(Activity),
        "Enter hides Activity"
    );
    assert!(
        !walk.shell.present(Region::Operations.id(), cx),
        "Activity is no longer drawn"
    );
    quiet(&walk, &before, "Activity hidden", cx);
    walk.keys("secondary-4", cx);
    assert!(
        layout_of(&walk, cx).is_displayed(Activity),
        "secondary-4 reveals Activity"
    );
    ringed_on(&walk, Region::Operations, cx);
    quiet(&walk, &before, "secondary-4", cx);

    // The layout is in the file once the save delay has run out, and the
    // write landed: no save notice.
    let arranged = layout_of(&walk, cx);
    elapse(&walk.shell, cx);
    assert!(
        !walk.shell.present("layout-save-notice", cx),
        "the file could be written"
    );
    same(&in_file(&walk.shell), &arranged, "the file");

    // A restart finds the same layout, over a readable file: no notice,
    // no save, and the launch sent what a launch sends.
    let walk = Walk::over(walk.shell.relaunch(cx), walk.a);
    same(&layout_of(&walk, cx), &arranged, "the restart");
    assert!(
        !walk.shell.present("layout-restore-notice", cx),
        "a readable file gives no restore notice"
    );
    assert!(
        !walk.shell.present("layout-save-notice", cx),
        "a restart writes nothing"
    );
    assert_eq!(saves(&walk.shell, cx), 0, "the restart asked for no save");
    assert_eq!(
        read(&walk.shell, Control::Panel(Documents), cx),
        Some(("Show Documents".to_owned(), Some(false))),
        "after the restart Documents' toggle reads off"
    );
    assert!(
        !walk.shell.present(Region::Documents.id(), cx),
        "after the restart Documents is hidden"
    );
    assert!(
        walk.shell.present(Region::Tickets.id(), cx),
        "after the restart Tickets is drawn"
    );
    assert!(
        walk.shell.present(Region::Operations.id(), cx),
        "after the restart Activity is drawn"
    );
    quiet(&walk, &at_launch, "the restart", cx);
}
