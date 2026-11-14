//! P20, `routes_by_keyboard` (contract 5; design Part D and H): one
//! trigger of each kind, focused and pressed with Enter: the footer item,
//! an Activity row's button, an Activity row's menu entry, a popover row, a
//! repository row's menu entry, an item header's Share…, a lifecycle
//! dialog's Open Sharing & keys and the save banner's Open Settings. Each
//! lands as Part D says: its page, and focus on its control after two
//! draws (Ruling 2). Each row has a window of its own; the fixtures are
//! P5's. None of them has a key, so the key list lands on Make a new key.

use gpui_kit::{ElementId, TestAppContext};
use manyhands::results::ResultCode;

use super::{
    returns::{SHARE, focus, footer, set_initials, ticket_shell},
    returns_activity::{activity_rows, keys_label, open_popover, press_in_popover},
    routes::entry,
    support::{MAIN, holds, open, shell, shown, ticket},
    table::Table,
};
use crate::desktop::{
    model::{
        activity::{copy::OPEN_LIFECYCLE, rows::RowKey},
        navigation::Pane,
        stamp::SettingsPage,
    },
    services::requests::Intent,
    test_support::fixtures::item_id,
    view::{
        activity::tests::cases::{answered, fabricate},
        activity_support::{act, button_focus, to_row},
        footer::Control,
        harness::Shell,
        repositories::tests::support::highlight,
        settings::Shown,
        sidebar::SHARE_MAIN_SET_UP,
    },
};

/// Make a new key: where the key list lands with no key (R-22).
const MAKE: &str = "keys-make";

/// Row `row` landed on `page`, focus on `focus` after two draws.
fn landed(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    page: SettingsPage,
    focus: impl Into<ElementId>,
    cx: &mut TestAppContext,
) {
    let at = shown(shell, cx);
    table.fact(
        &format!("{row} (its page)"),
        at == Shown::Settings(page),
        &format!("Settings({page:?}), not {at:?},"),
    );
    let focus = focus.into();
    let held = holds(shell, focus.clone(), cx);
    table.row(shell, row, held, &format!("{focus:?}"), cx);
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    footer_item(table, cx);
    activity_row(table, cx);
    popover_row(table, cx);
    repository_menu(table, cx);
    share(table, cx);
    lifecycle(table, cx).await;
    banner(table, cx).await;
}

/// The footer's Settings (W-2): Repositories, its entry.
fn footer_item(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    let settings = footer(&shell, Control::Settings, cx);
    focus(&shell, &settings, cx);
    shell.press("enter", cx);
    landed(
        table,
        &shell,
        "P20 the footer item (W-2)",
        SettingsPage::Repositories,
        entry(0),
        cx,
    );
}

/// A failed share's Open Sharing & keys (W-12), by the row's button, reached
/// by Tab from the list, and by its row menu, opened with `shift-f10` and
/// chosen by the arrows: the key list.
fn activity_row(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    to_row(shell, &rows.keys, cx);
    shell.keys("tab", cx);
    let button = button_focus(shell, cx);
    assert!(
        shell.frame(cx, |window, _| button.is_focused(window)),
        "Tab reaches the row's button"
    );
    shell.press("enter", cx);
    landed(
        table,
        shell,
        "P20 an Activity row's button (W-12)",
        SettingsPage::Keys,
        MAKE,
        cx,
    );

    let rows = activity_rows(cx);
    let shell = &rows.shell;
    to_row(shell, &rows.keys, cx);
    shell.keys("shift-f10", cx);
    highlight(&mut shell.visual(cx), keys_label());
    shell.keys("enter", cx);
    landed(
        table,
        shell,
        "P20 an Activity row's menu entry (W-12)",
        SettingsPage::Keys,
        MAKE,
        cx,
    );
}

/// The same row, from the Needs attention popover opened by Enter on the
/// footer item.
fn popover_row(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    open_popover(shell, cx);
    press_in_popover(shell, &rows.keys, cx);
    landed(
        table,
        shell,
        "P20 a popover row (W-12)",
        SettingsPage::Keys,
        MAKE,
        cx,
    );
}

/// A repository row's menu, opened with `shift-f10` on the Repositories
/// list: Share main… on a repository with no publication remote (W-10).
fn repository_menu(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    shell.keys("secondary-1", cx);
    shell.keys("shift-f10", cx);
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.repository_menu_open());
    assert!(open, "the menu opened by keyboard");
    highlight(&mut shell.visual(cx), SHARE_MAIN_SET_UP);
    shell.keys("enter", cx);
    landed(
        table,
        &shell,
        "P20 a repository row's menu entry (W-10)",
        SettingsPage::Remotes,
        "remote-add-name",
        cx,
    );
}

/// An item header's Share…, with no shared copy, reached by Tab (W-11).
fn share(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-3", cx);
    assert!(shell.tab_reaches(SHARE, 12, cx), "Tab reaches Share…");
    shell.press("enter", cx);
    shell.run(cx);
    landed(
        table,
        &shell,
        "P20 an item header's Share… (W-11)",
        SettingsPage::Remotes,
        "remote-add-name",
        cx,
    );
}

/// A lifecycle dialog's Open Sharing & keys (W-21), reached by Tab in the
/// dialog: the dialog closes onto its origin, then routes (fact 22).
async fn lifecycle(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let close = fabricate(
        &shell,
        1,
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(MAIN),
        },
        answered("ticket close", ResultCode::NoSelectedKey),
        cx,
    );
    act(&shell, &RowKey::Request(close), OPEN_LIFECYCLE, cx);
    shell
        .until_drawn(cx, "the dialog's Open Sharing & keys", |shell, cx| {
            shell.present("lifecycle-sharing-and-keys", cx)
        })
        .await;
    assert!(
        shell.tab_reaches(("lifecycle-sharing-and-keys", 0usize), 16, cx),
        "Tab reaches Open Sharing & keys"
    );
    shell.press("enter", cx);
    assert!(!shell.present("lifecycle-dialog", cx), "the dialog closed");
    landed(
        table,
        &shell,
        "P20 a lifecycle dialog's Open Sharing & keys (W-21)",
        SettingsPage::Keys,
        MAKE,
        cx,
    );
}

/// The save banner's Open Settings (W-30), reached by Tab from the item's
/// region, on a create refused for want of initials: the repository's
/// identity form.
async fn banner(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, root| set_initials(root, "toolong"));
    shell.select(&root, cx);
    shell.use_list(Pane::Tickets, cx);
    shell.click("new-ticket-button", cx);
    shell.visual(cx).simulate_input("Needs initials");
    shell.run(cx);
    shell.click("create-ok", cx);
    shell.until_editing(cx).await;
    shell.click("item-save", cx);
    shell
        .until_drawn(cx, "the banner's Open Settings", |shell, cx| {
            shell.present("item-save-settings", cx)
        })
        .await;
    // Out of the editor, where Tab is text, into the item's region.
    shell.keys("secondary-3", cx);
    assert!(
        shell.tab_reaches("item-save-settings", 40, cx),
        "Tab reaches Open Settings"
    );
    shell.press("enter", cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    landed(
        table,
        &shell,
        "P20 the save banner's Open Settings (W-30)",
        SettingsPage::Repositories,
        "identity-name",
        cx,
    );
}
