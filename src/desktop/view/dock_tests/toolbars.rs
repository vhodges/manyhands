//! Panel toolbars (Part G): each button of a panel's header is a tab stop
//! that Enter presses, counts as its panel's region, and works the same
//! when its panel is alone in its group or one tab of several (P24).

use std::path::PathBuf;

use gpui_kit::{Modifiers, MouseButton, TestAppContext, point, px, test::TestWindowExt};

use super::support::*;
use crate::desktop::{
    model::layout::PanelKind::{Documents, Tickets},
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::{
        create::{NEW_DOCUMENT, NEW_TICKET},
        dock::panels::UPDATING,
        documents::{COLLAPSE_ALL, NEW_FOLDER},
        harness::Shell,
        workspace::{REGIONS, Region, ToolbarButton},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT0";
const TOP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT1";
const NESTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT2";
/// What Collapse all leaves: the top-level folders and documents.
const COLLAPSED_ROWS: [&str; 3] = ["empty", "guides", "Top"];
/// `guides` opened again; `deep` stays collapsed.
const GUIDES_OPEN: [&str; 4] = ["empty", "guides", "deep", "Top"];
/// Every folder open: the rows of the tree as loaded.
const OPEN_ROWS: [&str; 6] = ["empty", "guides", "deep", "marker.md", "Nested", "Top"];

/// How many Tab presses a step may take to reach its button.
const TAB_LIMIT: usize = 60;

/// A repository with a ticket, a document at the top, one two folders down,
/// a nonconforming file beside it (which makes the folders, as the
/// primary worktree's committed file does) and an empty folder, then a
/// refresh; the window opened over it with nothing selected.
fn window(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Alpha", "");
    fixture.create_document(&root, TOP, "docs/top.md", "Top");
    fixture.create_document(&root, NESTED, "docs/guides/deep/nested.md", "Nested");
    fixture.commit_nonconforming_document(&root, "docs/guides/deep/marker.md");
    fixture.create_folder(&root, "docs/empty");
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// A button takes focus unless it is disabled: the Kit's snapshot has no
/// disabled flag, but an element that tracks no handle reports no focus.
fn enabled(shell: &Shell, button: ToolbarButton, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| window.find(button.id()).focused().is_some())
}

fn holds(shell: &Shell, button: ToolbarButton, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window.try_find(button.id()).and_then(|e| e.focused()) == Some(true)
    })
}

fn tab_to(shell: &Shell, button: ToolbarButton, cx: &mut TestAppContext) {
    for _ in 0..TAB_LIMIT {
        if holds(shell, button, cx) {
            return;
        }
        shell.keys("tab", cx);
    }
    panic!("Tab never reached {button:?}");
}

/// The region the Workspace says holds focus and the regions that draw
/// their ring.
fn region(shell: &Shell, cx: &mut TestAppContext) -> (Option<Region>, Vec<Region>) {
    shell.frame(cx, |window, cx| {
        let held = shell.workspace.read(cx).held_region(window, cx);
        let rings = REGIONS
            .into_iter()
            .filter(|region| window.try_find(region.ring_id()).is_some())
            .collect();
        (held, rings)
    })
}

/// The panel's region holds focus and alone draws its ring.
fn holds_region(shell: &Shell, region_wanted: Region, cx: &mut TestAppContext) {
    assert_eq!(
        region(shell, cx),
        (Some(region_wanted), vec![region_wanted])
    );
}

#[derive(Debug, PartialEq)]
enum Dialog {
    Ticket,
    Document,
    Folder,
}

fn dialog(shell: &Shell, cx: &mut TestAppContext) -> Option<Dialog> {
    if shell.present("new-folder-dialog", cx) {
        Some(Dialog::Folder)
    } else if shell.present("create-dialog", cx) {
        Some(if shell.present("create-type", cx) {
            Dialog::Ticket
        } else {
            Dialog::Document
        })
    } else {
        None
    }
}

/// Enter on the focused `button` opens `dialog`, and Escape closes it with
/// focus back on the button.
fn enter_opens(shell: &Shell, button: ToolbarButton, opens: Dialog, cx: &mut TestAppContext) {
    assert_eq!(dialog(shell, cx), None, "{button:?}: setup");
    shell.press("enter", cx);
    assert_eq!(dialog(shell, cx), Some(opens), "{button:?}: Enter");
    shell.keys("escape", cx);
    assert_eq!(dialog(shell, cx), None, "{button:?}: Escape");
    assert!(holds(shell, button, cx), "{button:?}: focus comes back");
}

/// Whether a press on the element `id`, a 5 px move and a release start a
/// drag; the release ends it.
fn starts_a_drag(
    shell: &Shell,
    id: impl Into<gpui_kit::ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    let at = shell.frame(cx, |window, _| window.find(id).bounds().center());
    let mut visual = shell.visual(cx);
    visual.simulate_mouse_move(at, None, Modifiers::default());
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
    let moved = at + point(px(5.), px(0.));
    visual.simulate_mouse_move(moved, MouseButton::Left, Modifiers::default());
    let started = visual.update(|_, cx| cx.has_active_drag());
    visual.simulate_mouse_up(moved, MouseButton::Left, Modifiers::default());
    started
}

fn rows(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.rows("doc-node", cx)
}

/// P24, P20, P29, R-13, R-37, decision 18: the toolbars of a single panel
/// and of a tabbed group, by Tab and Enter, with the region, ring, F6 and
/// `secondary-n` working from a button, a press that drags nothing, the
/// disabled states, and nothing acting behind a dialog.
#[gpui_kit::test]
async fn panel_toolbars_work_by_tab_and_enter(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = window(cx);
    with_no_repository(&shell, cx);
    shell.select(&root, cx);
    for button in ToolbarButton::ALL {
        assert!(enabled(&shell, button, cx), "{button:?} with a repository");
    }
    single_panels(&shell, cx);
    behind_a_dialog(&shell, cx);
    a_tabbed_group(&shell, cx).await;
}

/// Each button is drawn, named as the design names it, and disabled: with no
/// repository there is nothing to make, and no folder to collapse.
fn with_no_repository(shell: &Shell, cx: &mut TestAppContext) {
    let names = [
        (ToolbarButton::NewTicket, NEW_TICKET),
        (ToolbarButton::NewDocument, NEW_DOCUMENT),
        (ToolbarButton::NewFolder, NEW_FOLDER),
        (ToolbarButton::CollapseAll, COLLAPSE_ALL),
    ];
    for (button, name) in names {
        assert_eq!(
            shell.label(button.id(), cx).as_deref(),
            Some(name),
            "{button:?} is named"
        );
        assert!(!enabled(shell, button, cx), "{button:?} is disabled");
    }
}

/// Tickets and Documents each alone in their group (the default layout).
fn single_panels(shell: &Shell, cx: &mut TestAppContext) {
    // Tab reaches New ticket…: its panel's region holds focus and shows its
    // ring; Enter and `secondary-n` open the dialog.
    tab_to(shell, ToolbarButton::NewTicket, cx);
    holds_region(shell, Region::Tickets, cx);
    enter_opens(shell, ToolbarButton::NewTicket, Dialog::Ticket, cx);
    shell.keys("secondary-n", cx);
    assert_eq!(dialog(shell, cx), Some(Dialog::Ticket), "secondary-n");
    shell.keys("escape", cx);
    assert_eq!(dialog(shell, cx), None);

    // F6 moves on from the toolbar to the next region (C1 decision 9).
    shell.keys("f6", cx);
    assert_eq!(
        region(shell, cx),
        (Some(Region::Operations), vec![Region::Operations])
    );

    // Documents: each button by Tab and Enter, with Documents' region held.
    tab_to(shell, ToolbarButton::NewDocument, cx);
    holds_region(shell, Region::Documents, cx);
    enter_opens(shell, ToolbarButton::NewDocument, Dialog::Document, cx);
    tab_to(shell, ToolbarButton::NewFolder, cx);
    holds_region(shell, Region::Documents, cx);
    enter_opens(shell, ToolbarButton::NewFolder, Dialog::Folder, cx);
    // `secondary-n` from any Documents button makes a document.
    shell.keys("secondary-n", cx);
    assert_eq!(dialog(shell, cx), Some(Dialog::Document), "secondary-n");
    shell.keys("escape", cx);

    // A press and a small move starts no drag of the panel; the press is a
    // click, which opens the dialog. The same press on the title does start
    // one, so the small move is enough to drag (R-37). Tickets, alone in its
    // dock, cannot be dragged: Documents' group can.
    let layout_before = dump(shell, cx);
    let node = group_of(&area(shell, cx), Documents, cx).as_u64();
    assert!(
        starts_a_drag(shell, ("panel-title", node), cx),
        "a small move on the title drags the panel"
    );
    assert_eq!(dump(shell, cx), layout_before, "dropped where it was");
    assert!(
        !starts_a_drag(shell, ToolbarButton::NewDocument.id(), cx),
        "no panel drag started from the button"
    );
    assert_eq!(dump(shell, cx), layout_before, "the layout did not change");
    assert_eq!(dialog(shell, cx), Some(Dialog::Document), "it was a press");
    shell.keys("escape", cx);
    assert!(holds(shell, ToolbarButton::NewDocument, cx));

    // Collapse all (P20): with the selection inside a folder, every folder
    // is collapsed, the selection is on its top-level folder, the button is
    // disabled and focus is in the tree.
    shell.keys("secondary-2", cx);
    assert_eq!(rows(shell, cx), OPEN_ROWS);
    shell.keys("down down down down", cx);
    assert!(shell.selected(("doc-node", 4usize), cx), "Nested");
    shell.keys("shift-tab", cx);
    assert!(holds(shell, ToolbarButton::CollapseAll, cx), "shift-Tab");
    holds_region(shell, Region::Documents, cx);
    shell.press("enter", cx);
    assert_eq!(rows(shell, cx), COLLAPSED_ROWS);
    assert!(shell.selected(("doc-node", 1usize), cx), "its top folder");
    assert!(!enabled(shell, ToolbarButton::CollapseAll, cx), "disabled");
    holds_region(shell, Region::Documents, cx);
    assert!(!holds(shell, ToolbarButton::CollapseAll, cx));
    shell.keys("down", cx);
    assert!(
        shell.selected(("doc-node", 2usize), cx),
        "focus is in the tree"
    );
}

/// With a dialog open no toolbar button acts (P24): the press reaches the
/// Workspace, which refuses it, as it refuses the footer's.
fn behind_a_dialog(shell: &Shell, cx: &mut TestAppContext) {
    // Open `guides` again, by the keyboard, so Collapse all has work.
    shell.keys("secondary-2", cx);
    shell.keys("up right", cx);
    assert_eq!(rows(shell, cx), GUIDES_OPEN);
    assert!(enabled(shell, ToolbarButton::CollapseAll, cx));
    crate::desktop::view::footer::tests::support::open_modal(shell, false, cx);
    for button in ToolbarButton::ALL {
        act(shell, cx, |workspace, window, cx| {
            workspace.press_toolbar(button, window, cx)
        });
        assert_eq!(rows(shell, cx), GUIDES_OPEN, "{button:?}");
        assert_eq!(dialog(shell, cx), None, "{button:?} opened nothing");
    }
    shell.frame(cx, |window, cx| {
        use gpui_kit::component::WindowExt;
        window.close_all_dialogs(cx)
    });
    shell.run(cx);
    // Behind no dialog the same press collapses them.
    act(shell, cx, |workspace, window, cx| {
        workspace.press_toolbar(ToolbarButton::CollapseAll, window, cx)
    });
    assert_eq!(rows(shell, cx), COLLAPSED_ROWS);
}

/// Documents moved into Tickets' group as a tab: the active tab's toolbar is
/// drawn at the right of the strip, whichever tab it is, and works by Tab
/// and Enter; a reload of Tickets reads "Tickets · Updating…" on its tab
/// (P29).
async fn a_tabbed_group(shell: &Shell, cx: &mut TestAppContext) {
    let documents = [
        ToolbarButton::NewDocument,
        ToolbarButton::NewFolder,
        ToolbarButton::CollapseAll,
    ];
    // The move to another dock's group reveals Documents (R-36): it is the
    // active tab, with its own toolbar and not Tickets'.
    put_in_group(shell, Documents, Tickets, false, cx);
    assert!(shell.present("panel-tab-tickets", cx), "a strip of tabs");
    assert!(shell.selected("panel-tab-documents", cx));
    assert!(!shell.present(ToolbarButton::NewTicket.id(), cx));
    for button in documents {
        assert!(shell.present(button.id(), cx), "{button:?} is drawn");
    }
    tab_to(shell, ToolbarButton::NewDocument, cx);
    holds_region(shell, Region::Documents, cx);
    enter_opens(shell, ToolbarButton::NewDocument, Dialog::Document, cx);
    tab_to(shell, ToolbarButton::NewFolder, cx);
    holds_region(shell, Region::Documents, cx);
    enter_opens(shell, ToolbarButton::NewFolder, Dialog::Folder, cx);

    // Tickets' tab: its toolbar and not Documents'.
    shell.click("panel-tab-tickets", cx);
    assert!(shell.selected("panel-tab-tickets", cx));
    assert!(shell.present(ToolbarButton::NewTicket.id(), cx));
    for button in documents {
        assert!(!shell.present(button.id(), cx), "{button:?} is not drawn");
    }
    tab_to(shell, ToolbarButton::NewTicket, cx);
    holds_region(shell, Region::Tickets, cx);
    enter_opens(shell, ToolbarButton::NewTicket, Dialog::Ticket, cx);
    shell.keys("secondary-n", cx);
    assert_eq!(dialog(shell, cx), Some(Dialog::Ticket), "secondary-n");
    shell.keys("escape", cx);

    // A reload of Tickets, held: its tab says so (P29), and the toolbar
    // stays.
    let held = shell.backend.hold_read(ReadKind::Tickets);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    let updating = format!("Tickets \u{b7} {UPDATING}");
    shell
        .until_drawn(cx, "the tab says Updating", |shell, cx| {
            shell.label("panel-tab-tickets", cx).as_deref() == Some(updating.as_str())
        })
        .await;
    assert!(shell.present(ToolbarButton::NewTicket.id(), cx));
    let _ = held.send(());
    shell
        .until_drawn(cx, "the tab stops saying Updating", |shell, cx| {
            shell.label("panel-tab-tickets", cx).as_deref() == Some("Tickets")
        })
        .await;

    // Back to Documents' tab: its toolbar again.
    shell.click("panel-tab-documents", cx);
    for button in documents {
        assert!(shell.present(button.id(), cx), "{button:?} is drawn again");
    }
    assert!(!shell.present(ToolbarButton::NewTicket.id(), cx));
}
