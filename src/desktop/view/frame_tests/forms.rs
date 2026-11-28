//! P1's rows for the form dialogs: New ticket, New document, New folder
//! and Move (Wave 04 Cycle 05a design A3, A4, R-19). Focus is on the first
//! field on open; Escape closes the dialog, with focus back where it was
//! opened from, and waits while its write runs; and a keyed press of the
//! primary button (Tab to it, Enter) closes the dialog once the write is
//! done, which it does only while focus is inside the dialog's body.

use gpui_kit::TestAppContext;

use super::{
    Table,
    dialogs::{PAST_THE_RESTORE, back_at, inside, listed, origin},
};
use crate::desktop::{
    model::navigation::{OpenTarget, Pane},
    services::{JobKind, ReadKind},
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        item::ChipField,
    },
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const PLAN: &str = "docs/plan.md";

/// How many Tab presses a row may take to reach a button.
const TABS: usize = 12;

pub(super) async fn forms_keys(table: &mut Table, cx: &mut TestAppContext) {
    create_keys(table, cx).await;
    new_folder_keys(table, cx).await;
    move_keys(table, cx).await;
}

/// Types `text` into the field holding focus.
fn type_text(shell: &Shell, text: &str, cx: &mut TestAppContext) {
    shell.visual(cx).simulate_input(text);
    shell.run(cx);
}

/// New ticket and New document: opened by `secondary-n` in the list's
/// region. Create goes through a new ID, then the draft.
async fn create_keys(table: &mut Table, cx: &mut TestAppContext) {
    for (row, pane) in [
        ("New ticket", Pane::Tickets),
        ("New document", Pane::Documents),
    ] {
        let (shell, _root) = listed(cx);
        shell.use_list(pane, cx);
        shell.keys("secondary-2", cx);
        let origin = origin(&shell, cx);
        let open = |shell: &Shell, cx: &mut TestAppContext| {
            shell.keys("secondary-n", cx);
            assert!(shell.present("create-dialog", cx), "{row}: it opened");
        };

        open(&shell, cx);
        table.fact(
            &format!("{row}, on open"),
            inside(&shell, "create-title", cx),
            "focus in the Title",
        );
        shell.keys("escape", cx);
        table.fact(
            &format!("{row}, Escape closes it"),
            !shell.present("create-dialog", cx),
            "the dialog closed",
        );
        table.fact(
            &format!("{row}, Escape gives focus back"),
            back_at(&shell, &origin, cx),
            "focus on the list it was opened from",
        );

        // A keyed Create: Tab to the button, Enter. The new ID is held, so
        // the write runs while Escape is pressed.
        open(&shell, cx);
        type_text(&shell, "Plans", cx);
        if pane == Pane::Documents {
            shell.keys("tab tab", cx);
            type_text(&shell, "plans.md", cx);
            shell
                .until_drawn(cx, "the path check", |shell, cx| {
                    !shell.present("create-ok-note", cx)
                })
                .await;
        }
        assert!(
            shell.tab_reaches(("create-ok", 0usize), TABS, cx),
            "{row}: Tab reaches Create"
        );
        let new_id = shell.backend.hold_read(ReadKind::NewId);
        shell.keys("enter", cx);
        shell.keys("escape", cx);
        table.fact(
            &format!("{row}, Escape waits for the write"),
            shell.present("create-dialog", cx),
            "the dialog still open",
        );
        new_id.send(()).unwrap();
        shell
            .until_drawn(cx, "the dialog closing", |shell, cx| {
                !shell.present("create-dialog", cx)
            })
            .await;
        table.fact(
            &format!("{row}, a keyed Create closes it"),
            !shell.present("create-dialog", cx),
            "the dialog closed after its write",
        );
    }
}

/// New folder, from the Documents pane's button.
async fn new_folder_keys(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _root) = listed(cx);
    shell.use_list(Pane::Documents, cx);
    shell.keys("secondary-2", cx);
    // The toolbar's New folder button, by Shift-Tab from the tree and
    // Enter, as the keyboard walk opens it.
    let mut presses = 0;
    while !shell.holds_focus("new-folder-button", cx) {
        presses += 1;
        assert!(presses < 40, "Shift-Tab never reached New folder");
        shell.keys("shift-tab", cx);
    }
    let open = |shell: &Shell, cx: &mut TestAppContext| {
        shell.press("enter", cx);
        assert!(
            shell.present("new-folder-dialog", cx),
            "New folder: it opened"
        );
    };

    open(&shell, cx);
    table.fact(
        "New folder, on open",
        inside(&shell, "new-folder-name", cx),
        "focus in the Folder name",
    );
    shell.keys("escape", cx);
    table.fact(
        "New folder, Escape closes it",
        !shell.present("new-folder-dialog", cx),
        "the dialog closed",
    );
    // The toolbar's own handle takes focus back, as the keyboard walk
    // reads it: the button, by its id.
    cx.executor().advance_clock(PAST_THE_RESTORE);
    table.fact(
        "New folder, Escape gives focus back",
        shell.holds_focus("new-folder-button", cx),
        "focus on the New folder button it was opened from",
    );

    open(&shell, cx);
    type_text(&shell, "plans", cx);
    assert!(
        shell.tab_reaches(("new-folder-ok", 0usize), TABS, cx),
        "New folder: Tab reaches Create"
    );
    let held = shell.backend.hold_job(JobKind::FolderCreate);
    shell.keys("enter", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    table.fact(
        "New folder, Escape waits for the write",
        shell.present("new-folder-dialog", cx),
        "the dialog still open",
    );
    held.release();
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !shell.present("new-folder-dialog", cx)
        })
        .await;
    table.fact(
        "New folder, a keyed Create closes it",
        !shell.present("new-folder-dialog", cx),
        "the dialog closed after its write",
    );
}

/// Move, from the details sheet's Move… by Tab and Enter.
async fn move_keys(table: &mut Table, cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, PLAN, "Plan");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: manyhands::repository::ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.click(ChipField::All.id(), cx);
    shell.frame(cx, |_, _| ());
    assert!(
        shell.tab_reaches("sheet-move-button", 40, cx),
        "Move: Tab reaches Move…"
    );
    let origin = origin(&shell, cx);

    shell.press("enter", cx);
    assert!(shell.present("move-dialog", cx), "Move: it opened");
    table.fact(
        "Move, on open",
        inside(&shell, "move-folder", cx),
        "focus in the Folder",
    );
    shell.keys("escape", cx);
    table.fact(
        "Move, Escape closes it",
        !shell.present("move-dialog", cx),
        "the dialog closed",
    );
    table.fact(
        "Move, Escape gives focus back",
        back_at(&shell, &origin, cx),
        "focus on the Move… it was opened from",
    );

    shell.press("enter", cx);
    assert!(shell.present("move-dialog", cx), "Move: it opened again");
    shell.keys("tab", cx);
    shell.keys("secondary-a", cx);
    type_text(&shell, "renamed.md", cx);
    shell
        .until_drawn(cx, "the destination check", |shell, cx| {
            !shell.present("move-ok-note", cx)
        })
        .await;
    assert!(
        shell.tab_reaches(("move-ok", 0usize), TABS, cx),
        "Move: Tab reaches Move"
    );
    let held = shell.backend.hold_job(JobKind::DocumentMove);
    shell.keys("enter", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    table.fact(
        "Move, Escape waits for the write",
        shell.present("move-dialog", cx),
        "the dialog still open",
    );
    held.release();
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !shell.present("move-dialog", cx)
        })
        .await;
    table.fact(
        "Move, a keyed Move closes it",
        !shell.present("move-dialog", cx),
        "the dialog closed after its write",
    );
}
