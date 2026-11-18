//! P15's landings (design B9 G18 and G19; R-24, Cycle 04's R-34 and R-46
//! extended to the owner `AddRepository`). G18: the Kit's deferred restore
//! after Drafts' Discard…, its 0.25 s reached by the test clock, and a
//! landing Drafts applies in its own render, would each put focus behind
//! the open sheet: it goes to `add-path`. G19: an `open_created` landing
//! and a review landing, with focus on Add's prefix field inside the sheet,
//! leave it there; `add-path` would be G18's answer, so a landing that put
//! focus behind the sheet first fails the row. A lost focus (review finding
//! 4) keeps a target inside the Add sheet and gives `add-path` for one
//! behind it.

use std::time::Duration;

use gpui_kit::TestAppContext;
use manyhands::{
    drafts::{DraftId, ItemKind},
    repository::ItemDtoKind,
};

use super::support::*;
use crate::desktop::{
    model::{
        drafts::create::{new_item_draft, ticket_fields, ticket_path},
        navigation::OpenTarget,
    },
    services::ReadKind,
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id},
    },
    view::{
        drafts::row_id,
        harness::{Shell, key},
    },
};

/// A new ticket's ID, for the created item that lands.
const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW9";

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    deferred(table, cx);
    drawn_behind(table, cx);
    lost_focus(table, cx);
    landings(table, cx).await;
}

/// A lost focus over the open sheet (Cycle 04's F15, extended by R-24;
/// review finding 4), its target decided directly: a target inside the Add
/// sheet is honoured, from `add-path`; one behind it gives `add-path`
/// (G18), from the prefix field.
fn lost_focus(table: &mut Table, cx: &mut TestAppContext) {
    use gpui_kit::Focusable;

    let (shell, root) = one_repo(cx, |_, _| {});
    shell.select(&root, cx);
    let plus = press_plus(&shell, cx);
    if !opened(table, &shell, "G18 a lost focus", cx) {
        return;
    }
    let Some(prefix) =
        input(&shell, "add-prefix", cx).map(|input| input.read_with(cx, Focusable::focus_handle))
    else {
        table.fact("G18 a lost focus (the prefix)", false, "add-prefix drawn");
        return;
    };
    for (row, from, target, wanted) in [
        (
            "a lost focus, its target inside the Add sheet",
            "add-path",
            prefix,
            "add-prefix",
        ),
        (
            "G18 a lost focus, its target behind the sheet",
            "add-prefix",
            plus,
            "add-path",
        ),
    ] {
        focus_in(&shell, from, cx);
        route(&shell, cx, move |workspace, window, cx| {
            workspace.sheet_keeps_lost_focus(Some(&target), window, cx);
        });
        let held = add_open(&shell, cx) && holds(&shell, wanted, cx);
        table.row(&shell, row, held, wanted, cx);
    }
    shell.keys("escape", cx);
}

/// Repository `a`, its tickets `MAIN` and `OTHER` and a draft of each, with
/// the Drafts page shown. Gives the drafts.
fn drafts_shell(cx: &mut TestAppContext) -> (Shell, DraftId, DraftId) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, MAIN, "Main", "");
    fixture.commit_ticket(&root, OTHER, "Other", "");
    fixture.refresh(&root);
    let main = store_draft(&fixture, &root, ItemDtoKind::Ticket, MAIN, "Mine");
    let other = store_draft(&fixture, &root, ItemDtoKind::Ticket, OTHER, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.keys("secondary-shift-d", cx);
    assert!(
        shell.present(row_id(other, "row"), cx),
        "both drafts listed"
    );
    (shell, main, other)
}

/// G18: Drafts' Discard… closed by Enter, its default: the Kit closes it
/// now and restores focus to the Drafts row 0.25 s later (fact 34). The
/// sheet opens meanwhile, then the clock passes 250 ms.
fn deferred(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, main, other) = drafts_shell(cx);
    shell.click(row_id(main, "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx));
    shell.press("enter", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    shell.frame(cx, |_, _| ());
    press_plus(&shell, cx);
    let row = "G18 the Kit's deferred restore after Drafts' Discard…";
    if !opened(table, &shell, row, cx) {
        return;
    }
    cx.executor().advance_clock(Duration::from_millis(250));
    shell.run(cx);
    let on_path = holds(&shell, "add-path", cx);
    let on_row = holds(&shell, row_id(main, "row"), cx) || holds(&shell, row_id(other, "row"), cx);
    table.row(
        &shell,
        row,
        on_path && !on_row,
        "add-path, not a Drafts row",
        cx,
    );
}

/// G18: Drafts' Discard… confirmed, its write held; the sheet opened, and a
/// dialog over it. The write lands while the dialog is open, so Drafts
/// focuses the neighbour row in its own render, behind the sheet, once the
/// dialog closes. Read after GPUI's own draws only, as Cycle 04's
/// `behind` reads it.
fn drawn_behind(table: &mut Table, cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;

    let (shell, main, _) = drafts_shell(cx);
    shell.click(row_id(main, "discard"), cx);
    let held = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.click("discard-draft-confirm", cx);
    press_plus(&shell, cx);
    let row = "G18 a landing Drafts applies in its own render";
    if !opened(table, &shell, row, cx) {
        drop(held);
        shell.run(cx);
        return;
    }
    let path = input(&shell, "add-path", cx)
        .map(|input| input.read_with(cx, gpui_kit::Focusable::focus_handle));
    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);
    drop(held);
    shell.run(cx);
    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);
    let on_path = shell
        .visual(cx)
        .update(|window, _| path.as_ref().is_some_and(|path| path.is_focused(window)));
    table.row(&shell, row, on_path, "add-path, not behind the sheet", cx);
}

/// G19: an `open_created` landing and a review landing, with focus on the
/// prefix field inside the sheet: focus stays there.
async fn landings(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = one_repo(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "");
    });
    shell.select(&root, cx);
    // A new ticket's draft, written, and its prepare answered: what a
    // create dialog's task holds when it opens the item.
    let id = item_id(NEW);
    let fields = ticket_fields("Landed", "task");
    let (target, base) = new_item_draft(
        &key(&root),
        Some("main".to_owned()),
        ItemKind::Ticket,
        id.clone(),
        ticket_path(&id),
        &fields,
    );
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    let made = drafts.update(cx, |registry, cx| {
        registry.new_item(target, base, fields, cx)
    });
    shell.run(cx);
    let Ok(made) = made.await else {
        panic!("the new item's draft must be written");
    };
    shell.until_ended(made.prepare, cx).await;

    press_plus(&shell, cx);
    if !opened(table, &shell, "G19", cx) {
        return;
    }
    focus_in(&shell, "add-prefix", cx);
    let created = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id,
    };
    route(&shell, cx, move |workspace, window, cx| {
        workspace.open_created(created, made, window, cx)
    });
    let held = holds(&shell, "add-prefix", cx);
    table.row(
        &shell,
        "G19 an open_created landing",
        held,
        "add-prefix, unchanged",
        cx,
    );

    focus_in(&shell, "add-prefix", cx);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.draft_discarded(cx));
    shell.run(cx);
    let held = holds(&shell, "add-prefix", cx);
    table.row(
        &shell,
        "G19 a review landing",
        held,
        "add-prefix, unchanged",
        cx,
    );
    shell.keys("escape", cx);
}
