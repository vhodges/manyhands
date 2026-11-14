//! P23, `focus_stays_inside_settings` (design A6 F15, F16, F18 and F19;
//! Part H): Tab and Shift-Tab wrap inside the sheet; a landing that would
//! focus the item area leaves focus inside the sheet; a dialog opened from
//! a page gives focus back inside it, also when its origin has gone and
//! when it opened within 0.25 s of a deferred close; the Kit's deferred
//! restore, its clock advanced 250 ms with no real wait, does not put focus
//! behind the sheet; the interval select and an input's context menu keep
//! focus until they close. Each read is after two draws (Ruling 2).

use std::{path::PathBuf, time::Duration};

use gpui_kit::{FocusHandle, TestAppContext};
use manyhands::{
    drafts::{DraftId, ItemKind},
    repository::ItemDtoKind,
    results::ResultCode,
};

use super::{
    returns::{escape, focus, manage, settings_open, ticket_shell},
    support::{
        MAIN, drafts_shown, has_dialog, holds, holds_handle, host, in_context, settle,
        tab_to_create,
    },
    table::Table,
};
use crate::desktop::{
    model::{
        drafts::create::{new_item_draft, ticket_fields, ticket_path},
        navigation::OpenTarget,
    },
    services::requests::Intent,
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id},
    },
    view::{
        activity::tests::cases::{answered, fabricate},
        drafts::row_id,
        harness::{Shell, key},
        settings::polling_support::Rig,
    },
};

/// A new ticket's ID, for the created item that lands.
const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW9";

/// The focus the Repositories page's wrapper `id` tracks.
pub(super) fn page_focus(shell: &Shell, id: &str, cx: &mut TestAppContext) -> FocusHandle {
    let host = host(shell, cx);
    shell.visual(cx).update(|window, cx| {
        let page = host.update(cx, |host, cx| host.repositories(window, cx));
        page.read(cx).control_focus(id)
    })
}

/// Focuses the Add folder's input, inside the sheet.
fn focus_add_path(shell: &Shell, cx: &mut TestAppContext) {
    let host = host(shell, cx);
    shell.visual(cx).update(|window, cx| {
        let page = host.update(cx, |host, cx| host.repositories(window, cx));
        let input = page.read(cx).input("add-path");
        input.update(cx, |input, cx| input.focus(window, cx));
    });
    shell.run(cx);
    assert!(holds(shell, "add-path", cx), "the Add folder holds focus");
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    tab_trap(table, cx);
    landings(table, cx).await;
    dialogs(table, cx);
    deferred(table, cx);
    select(table, cx).await;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    context_menu(table, cx);
}

/// Part H: Tab from the last control, Create repository, reaches the
/// first, the sheet's close button in its header, and Shift-Tab from it the
/// last (the Kit's trap). Create repository's own button holds focus inside
/// its wrapper, which is what is read.
fn tab_trap(table: &mut Table, cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt;

    let (shell, _) = ticket_shell(cx);
    shell.keys("secondary-,", cx);
    tab_to_create(&shell, cx);
    shell.keys("tab", cx);
    settle(&shell, cx);
    let first = shell.frame(cx, |window, _| {
        window
            .within("sheet-content")
            .try_find("close")
            .and_then(|close| close.focused())
            == Some(true)
    });
    table.row(
        &shell,
        "Tab from the last control",
        first,
        "the sheet's close button, the first control",
        cx,
    );
    shell.keys("shift-tab", cx);
    let last = holds(&shell, "create-submit", cx);
    table.row(
        &shell,
        "Shift-Tab from the first control",
        last,
        "Create repository, the last control",
        cx,
    );
    escape(&shell, "the trap", cx);
}

/// F18: an `open_created` landing and a review landing while Settings is
/// open leave focus where it was inside the sheet, the Add folder.
async fn landings(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    // A new ticket's draft, written, and its prepare answered: what a create
    // dialog's task holds when it opens the item.
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

    shell.keys("secondary-,", cx);
    focus_add_path(&shell, cx);
    let created = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id,
    };
    super::support::route(&shell, cx, move |workspace, window, cx| {
        workspace.open_created(created, made, window, cx)
    });
    let held = holds(&shell, "add-path", cx);
    table.row(
        &shell,
        "F18 an open_created landing",
        held,
        "the Add folder, unchanged inside the sheet",
        cx,
    );

    focus_add_path(&shell, cx);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.draft_discarded(cx));
    shell.run(cx);
    let held = holds(&shell, "add-path", cx);
    table.row(
        &shell,
        "F18 a review landing",
        held,
        "the Add folder, unchanged inside the sheet",
        cx,
    );
}

/// F15: Remove repository…'s confirmation, closed by Escape, gives focus
/// back to Remove, inside the sheet; opened again, and Remove gone with its
/// repository while it is open (a removal landing, another actor's), its
/// close gives focus to the page's entry.
fn dialogs(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    shell.keys("secondary-,", cx);
    manage(&shell, &root, cx);
    let remove = page_focus(&shell, "remove-submit", cx);
    focus(&shell, &remove, cx);
    shell.click("remove-submit", cx);
    assert!(has_dialog(&shell, cx), "the confirmation opened");
    shell.keys("escape", cx);
    assert!(!has_dialog(&shell, cx));
    assert!(settings_open(&shell, cx), "Escape closed only the dialog");
    let held = holds_handle(&shell, &remove, cx) && in_context(&shell, "Sheet", cx);
    table.row(
        &shell,
        "F15 a dialog opened from a page",
        held,
        "Remove repository…, its origin, inside the sheet",
        cx,
    );

    focus(&shell, &remove, cx);
    shell.click("remove-submit", cx);
    assert!(has_dialog(&shell, cx), "the confirmation opened again");
    fabricate(
        &shell,
        1,
        Intent::RepoRemove { root: root.clone() },
        answered("repo remove", ResultCode::Ok),
        cx,
    );
    settle(&shell, cx);
    assert!(!shell.present("remove-submit", cx), "Remove went");
    shell.keys("escape", cx);
    assert!(!has_dialog(&shell, cx));
    let held = holds(&shell, ("settings-page", 0usize), cx);
    table.row(
        &shell,
        "F15 a dialog whose origin has gone",
        held,
        "the page's entry",
        cx,
    );
}

/// Repository `a`, its ticket `MAIN` and a draft of it, with the Drafts
/// page shown. Gives the draft.
fn drafts_shell(cx: &mut TestAppContext) -> (Shell, PathBuf, DraftId) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, MAIN, "Main", "");
    fixture.refresh(&root);
    let draft = store_draft(&fixture, &root, ItemDtoKind::Ticket, MAIN, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&shell, cx));
    assert!(shell.present(row_id(draft, "discard"), cx));
    (shell, root, draft)
}

/// Drafts' Discard… on `draft`, closed by Enter, its default: the Kit
/// closes it now and restores focus 0.25 s later (fact 46).
fn discard_by_enter(shell: &Shell, draft: DraftId, cx: &mut TestAppContext) {
    shell.click(row_id(draft, "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx));
    shell.press("enter", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
}

/// F16: Drafts' Discard… closed with Enter, a draw, Ctrl+,, the clock
/// advanced 250 ms and a draw: focus is on the page's entry, not the Drafts
/// row. F15: a page's dialog opened within that 0.25 s gives focus back to
/// its own origin; and once the restore's 250 ms have passed after it, the
/// restore is F16's again: the page's entry, not the Drafts row.
fn deferred(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, draft) = drafts_shell(cx);
    discard_by_enter(&shell, draft, cx);
    shell.frame(cx, |_, _| ());
    shell.keys("secondary-,", cx);
    assert!(settings_open(&shell, cx));
    cx.executor().advance_clock(Duration::from_millis(250));
    shell.run(cx);
    let entry = holds(&shell, ("settings-page", 0usize), cx);
    let row = holds(&shell, row_id(draft, "row"), cx);
    table.row(
        &shell,
        "F16 the Kit's deferred restore after Drafts' Discard…",
        entry && !row,
        "the page's entry, not the Drafts row",
        cx,
    );

    let (shell, root, draft) = drafts_shell(cx);
    discard_by_enter(&shell, draft, cx);
    shell.frame(cx, |_, _| ());
    shell.keys("secondary-,", cx);
    manage(&shell, &root, cx);
    let remove = page_focus(&shell, "remove-submit", cx);
    focus(&shell, &remove, cx);
    shell.click("remove-submit", cx);
    assert!(has_dialog(&shell, cx), "the confirmation opened");
    shell.keys("escape", cx);
    assert!(!has_dialog(&shell, cx));
    let held = holds_handle(&shell, &remove, cx) && in_context(&shell, "Sheet", cx);
    table.row(
        &shell,
        "F15 a page's dialog opened within 0.25 s of a deferred close",
        held,
        "Remove repository…, its origin, inside the sheet",
        cx,
    );
    cx.executor().advance_clock(Duration::from_millis(250));
    shell.run(cx);
    let entry = holds(&shell, ("settings-page", 0usize), cx);
    let row = holds(&shell, row_id(draft, "row"), cx);
    table.row(
        &shell,
        "F16 the deferred restore, its 250 ms passed after a page's dialog",
        entry && !row,
        "the page's entry, not the Drafts row",
        cx,
    );
}

/// F19: the interval select, reached by Tab and opened with Enter, keeps
/// focus in its list until Escape closes it; focus is then back on it.
async fn select(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.open(1, cx).await;
    let shell = &rig.shell;
    let row = rig.row("a", cx);
    assert!(
        shell.tab_reaches(("polling-interval", row), 30, cx),
        "Tab reaches the interval"
    );
    shell.press("enter", cx);
    let open = in_context(shell, "List", cx) && in_context(shell, "Sheet", cx);
    table.row(
        shell,
        "F19 the interval select opened by keyboard",
        open,
        "the select's list, inside the sheet",
        cx,
    );
    shell.keys("escape", cx);
    let held = holds(shell, ("polling-interval", row), cx) && !in_context(shell, "List", cx);
    table.row(
        shell,
        "F19 the interval select, closed",
        held,
        "the interval select, inside the sheet",
        cx,
    );
}

/// F19: Root's context menu over the Add folder (the Kit's fallback menu,
/// drawn by Root beside the workspace, A7 item 2) keeps focus until Escape
/// closes it; focus is then back on the field.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn context_menu(table: &mut Table, cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt;

    let (shell, _) = ticket_shell(cx);
    shell.keys("secondary-,", cx);
    shell.frame(cx, |window, cx| window.right_click("add-path", cx));
    shell.run(cx);
    let open = in_context(&shell, "PopupMenu", cx);
    table.row(
        &shell,
        "F19 an input's context menu",
        open,
        "the context menu, in Root's overlay",
        cx,
    );
    shell.keys("escape", cx);
    let held = holds(&shell, "add-path", cx) && in_context(&shell, "Sheet", cx);
    table.row(
        &shell,
        "F19 an input's context menu, closed",
        held,
        "the Add folder, inside the sheet",
        cx,
    );
}
