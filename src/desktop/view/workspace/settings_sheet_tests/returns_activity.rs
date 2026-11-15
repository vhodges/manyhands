//! P5, `focus_returns_to_what_opened_settings`: the rows whose origin is
//! Activity's list or the footer's Needs attention (design A6 F5 to F8;
//! Part D's origins; Ruling 8). The rows are a failed share with no key to
//! share with, whose button is Open Sharing & keys and which needs
//! attention, and a finished refresh above it that needs nothing; both are
//! put in the request model (`fabricate`), so no worker runs. Each read is
//! after two draws (Ruling 2).

use gpui_kit::TestAppContext;
use manyhands::results::ResultCode;

use super::{
    returns::{escape, focus, footer, settings_open},
    support::{holds_handle, in_context, settle, shell, shown},
    table::Table,
};
use crate::desktop::{
    model::{
        activity::{copy::offer_label, rows::RowKey},
        publication::SyncOffer,
        stamp::SettingsPage,
    },
    services::requests::{DesktopRequestId, Intent},
    view::{
        activity::tests::cases::{TICKET, answered, fabricate, item_sync},
        activity_support::{
            button_focus, choose_in_menu, drawn, highlighted, index_of, list_focus, refresh, to_row,
        },
        footer::Control,
        harness::Shell,
        settings::Shown,
        workspace::ToolbarButton,
    },
};

/// The share row's button.
pub(super) fn keys_label() -> &'static str {
    offer_label(SyncOffer::OpenSharingAndKeys)
}

/// The window and its two Activity rows.
pub(super) struct Rows {
    pub(super) shell: Shell,
    /// The failed share, Open Sharing & keys.
    pub(super) keys: RowKey,
    pub(super) id: DesktopRequestId,
}

pub(super) fn activity_rows(cx: &mut TestAppContext) -> Rows {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Ticket", "");
    });
    let id = fabricate(
        &shell,
        1,
        item_sync(&root),
        answered("item sync", ResultCode::NoSelectedKey),
        cx,
    );
    fabricate(
        &shell,
        2,
        Intent::IndexRefresh { root: root.clone() },
        answered("index refresh", ResultCode::Ok),
        cx,
    );
    refresh(&shell, cx);
    settle(&shell, cx);
    Rows {
        shell,
        keys: RowKey::Request(id),
        id,
    }
}

/// Activity's list holds focus with `row` highlighted, after two draws.
fn on_row(shell: &Shell, row: Option<&RowKey>, cx: &mut TestAppContext) -> bool {
    let list = list_focus(shell, cx);
    holds_handle(shell, &list, cx) && highlighted(shell, cx).as_ref() == row
}

/// Sharing & keys opened.
fn on_keys(shell: &Shell, row: &str, cx: &mut TestAppContext) {
    assert_eq!(
        shown(shell, cx),
        Shown::Settings(SettingsPage::Keys),
        "{row}: Open Sharing & keys opened Sharing & keys"
    );
}

pub(super) fn rows(table: &mut Table, cx: &mut TestAppContext) {
    activity_row(table, cx);
    row_gone(table, cx);
    popover_row(table, cx);
    popover_item_hidden(table, cx);
    in_popover(table, cx);
    in_row_menu(table, cx);
    in_more(table, cx);
}

/// F5: an Activity row's Open Sharing & keys, by its button (focused and
/// pressed with Enter), by its row menu and by Enter on the list. Each
/// gives focus back to the list, the row still highlighted.
fn activity_row(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;

    to_row(shell, &rows.keys, cx);
    let button = button_focus(shell, cx);
    focus(shell, &button, cx);
    assert!(shell.frame(cx, |window, _| button.is_focused(window)));
    shell.press("enter", cx);
    on_keys(shell, "the button", cx);
    escape(shell, "the button", cx);
    let held = on_row(shell, Some(&rows.keys), cx);
    table.row(
        shell,
        "F5 an Activity row, by its button",
        held,
        "Activity's list, the row highlighted",
        cx,
    );

    choose_in_menu(shell, &rows.keys, keys_label(), cx);
    on_keys(shell, "the row menu", cx);
    escape(shell, "the row menu", cx);
    let held = on_row(shell, Some(&rows.keys), cx);
    table.row(
        shell,
        "F5 an Activity row, by its menu",
        held,
        "Activity's list, the row highlighted",
        cx,
    );

    to_row(shell, &rows.keys, cx);
    shell.keys("enter", cx);
    on_keys(shell, "Enter", cx);
    escape(shell, "Enter", cx);
    let held = on_row(shell, Some(&rows.keys), cx);
    table.row(
        shell,
        "F5 an Activity row, by Enter",
        held,
        "Activity's list, the row highlighted",
        cx,
    );
}

/// F5: the row goes while Settings is open (it is dismissed, as a landing
/// would). Focus goes to the list, highlighted by 03b's cursor rule: the
/// row now at its place, else the last.
fn row_gone(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    to_row(shell, &rows.keys, cx);
    let at = index_of(shell, &rows.keys, cx);
    shell.keys("enter", cx);
    on_keys(shell, "the row that goes", cx);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.dismiss_request(rows.id, cx));
    shell.run(cx);
    settle(shell, cx);
    let now = drawn(shell, cx);
    assert!(
        now.iter().all(|row| row.row.key != rows.keys),
        "the row went"
    );
    escape(shell, "the row that goes", cx);
    let wanted = now
        .get(at)
        .or_else(|| now.last())
        .map(|row| row.row.key.clone());
    let held = on_row(shell, wanted.as_ref(), cx);
    table.row(
        shell,
        "F5 an Activity row whose row has gone",
        held,
        "Activity's list, highlighted by the cursor rule",
        cx,
    );
}

/// The footer's Needs attention item.
fn attention_item(shell: &Shell, cx: &mut TestAppContext) -> gpui_kit::FocusHandle {
    footer(shell, Control::Attention, cx)
}

/// Opens the popover by Enter on the footer item, and lets its rows draw.
pub(super) fn open_popover(shell: &Shell, cx: &mut TestAppContext) {
    refresh(shell, cx);
    settle(shell, cx);
    let item = attention_item(shell, cx);
    focus(shell, &item, cx);
    shell.press("enter", cx);
    for _ in 0..3 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_open());
    assert!(open, "Enter on the footer item opened the popover");
}

/// Presses the popover's highlighted row with Enter: it must be `row`.
pub(super) fn press_in_popover(shell: &Shell, row: &RowKey, cx: &mut TestAppContext) {
    let cursor = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_popover_cursor());
    assert_eq!(cursor.as_ref(), Some(row), "the row is highlighted");
    shell.press("enter", cx);
    shell.run(cx);
}

/// Where the footer's Needs attention gives focus back, now: the item, or
/// E1's control once it has hidden.
fn attention_back(shell: &Shell, cx: &mut TestAppContext) -> gpui_kit::FocusHandle {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.attention_restore_target(cx))
}

/// F6: a popover row's Open Sharing & keys, with the footer item still
/// shown at close time.
fn popover_row(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    let item = attention_item(shell, cx);
    open_popover(shell, cx);
    press_in_popover(shell, &rows.keys, cx);
    on_keys(shell, "a popover row", cx);
    escape(shell, "a popover row", cx);
    let held = holds_handle(shell, &item, cx);
    table.row(
        shell,
        "F6 a popover row",
        held,
        "the footer's Needs attention",
        cx,
    );
}

/// F7: as F6, the row dismissed while Settings is open, so nothing needs
/// attention and the footer item has hidden by close time.
fn popover_item_hidden(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    let item = attention_item(shell, cx);
    open_popover(shell, cx);
    press_in_popover(shell, &rows.keys, cx);
    on_keys(shell, "a popover row, its item hidden", cx);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.dismiss_request(rows.id, cx));
    shell.run(cx);
    settle(shell, cx);
    let e1 = attention_back(shell, cx);
    assert!(e1 != item, "the footer item has hidden");
    escape(shell, "a popover row, its item hidden", cx);
    let held = holds_handle(shell, &e1, cx);
    table.row(
        shell,
        "F7 a popover row, its footer item hidden by close time",
        held,
        "E1's control",
        cx,
    );
}

/// F8: Ctrl+, while the popover is open, with focus in its list: back to
/// where the popover returns, the footer item.
fn in_popover(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    open_popover(shell, cx);
    let [list, _, _] = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_handles());
    assert!(shell.frame(cx, |window, _| list.is_focused(window)));
    shell.keys("secondary-,", cx);
    assert!(settings_open(shell, cx), "Ctrl+, opened Settings");
    escape(shell, "Ctrl+, in the popover", cx);
    let back = attention_back(shell, cx);
    let held = holds_handle(shell, &back, cx);
    table.row(
        shell,
        "F8 Ctrl+, in the Needs attention popover",
        held,
        "the footer's Needs attention, where the popover returns",
        cx,
    );
}

/// F8: Ctrl+, in a row's menu opened by keyboard, which keeps no
/// `restore`: back to its row, on the list.
fn in_row_menu(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    to_row(shell, &rows.keys, cx);
    shell.keys("shift-f10", cx);
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_menu_open());
    assert!(open, "the row menu opened by keyboard");
    assert!(in_context(shell, "PopupMenu", cx), "the menu holds focus");
    shell.keys("secondary-,", cx);
    assert!(settings_open(shell, cx), "Ctrl+, opened Settings");
    escape(shell, "Ctrl+, in a row menu", cx);
    let held = on_row(shell, Some(&rows.keys), cx);
    table.row(
        shell,
        "F8 Ctrl+, in a row menu opened by keyboard",
        held,
        "Activity's list, its row highlighted",
        cx,
    );
}

/// F8, Ruling 8: Ctrl+, in Activity's ⋯ menu, opened by Enter on its
/// button: back to the button, where the menu returns.
fn in_more(table: &mut Table, cx: &mut TestAppContext) {
    let rows = activity_rows(cx);
    let shell = &rows.shell;
    let more = shell.workspace.read_with(cx, |workspace, _| {
        workspace.toolbar_focus(ToolbarButton::ActivityMore).clone()
    });
    focus(shell, &more, cx);
    shell.press("enter", cx);
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_more_open());
    assert!(open, "⋯ opened by Enter");
    assert!(in_context(shell, "PopupMenu", cx), "the menu holds focus");
    shell.keys("secondary-,", cx);
    assert!(settings_open(shell, cx), "Ctrl+, opened Settings");
    escape(shell, "Ctrl+, in ⋯", cx);
    let held = holds_handle(shell, &more, cx);
    table.row(
        shell,
        "F8 Ctrl+, in Activity's ⋯ menu",
        held,
        "the ⋯ button, where the menu returns",
        cx,
    );
}
