//! P5, `focus_returns_to_what_opened_settings`: the rows whose origin is
//! not Activity's (design A6 F4, F8, F9 and F10; Part D's origins). Each
//! opens Settings from its origin, closes it by Escape, and reads focus
//! after two draws (Ruling 2): the footer item, a ticket list row, an item
//! header's Share…, a lifecycle dialog's route (the dialog's origin),
//! Ctrl+, in a repository menu and in a dock menu opened by keyboard,
//! W-31 inside the sheet (the first origin), the save banner's Open
//! Settings whose banner has gone (F9), a ticket list whose list has gone
//! (F9 in a region other than Content, added) and no origin (F10).

use std::path::{Path, PathBuf};

use gpui_kit::{FocusHandle, TestAppContext, test::TestWindowExt};
use manyhands::results::ResultCode;

use super::{
    super::SheetOwnerKind,
    support::{
        MAIN, closed, has_sheet, holds, holds_handle, in_context, open, owner, route, settle,
        shell, shown, ticket,
    },
    table::Table,
};
use crate::desktop::{
    model::{
        activity::{copy::OPEN_LIFECYCLE, rows::RowKey},
        layout::PanelKind,
        navigation::Pane,
        stamp::SettingsPage,
    },
    services::requests::Intent,
    test_support::fixtures::item_id,
    view::{
        activity::tests::cases::{answered, fabricate},
        activity_support::{act, list_focus},
        footer::Control,
        harness::{Shell, key, wait_until},
        settings::{Shown, polling_support::Rig},
        workspace::Region,
    },
};

/// The item header's Share (`item::share::SHARE_BUTTON`).
const SHARE: &str = "item-share";

/// Settings is open: it owns the sheet, and the sheet is active.
pub(super) fn settings_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    owner(shell, cx) == Some(SheetOwnerKind::Settings) && has_sheet(shell, cx)
}

/// Escape closes Settings: a step of the row, which must hold.
pub(super) fn escape(shell: &Shell, row: &str, cx: &mut TestAppContext) {
    assert!(settings_open(shell, cx), "{row}: Settings is open");
    shell.keys("escape", cx);
    closed(shell, row, cx);
}

/// The region that holds focus, after two draws.
pub(super) fn region(shell: &Shell, cx: &mut TestAppContext) -> Option<Region> {
    settle(shell, cx);
    let workspace = shell.workspace.clone();
    shell.frame(cx, |window, cx| workspace.read(cx).held_region(window, cx))
}

/// The footer control `control`'s focus handle.
pub(super) fn footer(shell: &Shell, control: Control, cx: &mut TestAppContext) -> FocusHandle {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.footer_focus(control))
}

/// Puts focus on `handle`, as Tab or a click would.
pub(super) fn focus(shell: &Shell, handle: &FocusHandle, cx: &mut TestAppContext) {
    shell.frame(cx, |window, cx| window.focus(handle, cx));
    shell.run(cx);
}

/// Repository `a`, with its ticket `MAIN`, chosen in the panel.
pub(super) fn ticket_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, MAIN, "Main", "");
    });
    shell.select(&root, cx);
    (shell, root)
}

/// Manages `root` on the Repositories page, by its row's Manage.
pub(super) fn manage(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let rows = shell.rows("managed-row", cx);
    let ix = rows
        .iter()
        .position(|row| *row == key(root))
        .expect("the repository is listed");
    shell.click(("managed-choose", ix), cx);
    assert!(shell.present("remove-submit", cx), "it is managed");
}

/// The tickets list's focus handle.
fn tickets_list(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.tickets_list_focus(cx))
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    footer_item(table, cx);
    ticket_list(table, cx);
    share(table, cx);
    lifecycle(table, cx).await;
    repository_menu(table, cx);
    dock_menu(table, cx);
    inside(table, cx).await;
    banner(table, cx).await;
    gone_from_its_region(table, cx).await;
    no_origin(table, cx);
}

/// F4: the footer's Settings, focused and pressed with Enter.
fn footer_item(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    let settings = footer(&shell, Control::Settings, cx);
    focus(&shell, &settings, cx);
    shell.press("enter", cx);
    assert_eq!(
        shown(&shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    escape(&shell, "the footer item", cx);
    let held = holds_handle(&shell, &settings, cx);
    table.row(
        &shell,
        "F4 the footer item",
        held,
        "the footer's Settings",
        cx,
    );
}

/// F4: Ctrl+, from the tickets list.
fn ticket_list(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    shell.use_list(Pane::Tickets, cx);
    let list = tickets_list(&shell, cx);
    assert!(shell.frame(cx, |window, _| list.is_focused(window)));
    shell.keys("secondary-,", cx);
    escape(&shell, "a ticket list row", cx);
    let held = holds_handle(&shell, &list, cx);
    table.row(&shell, "F4 a ticket list row", held, "the tickets list", cx);
}

/// F4: an item header's Share…, with no shared copy, reached by Tab and
/// pressed with Enter: Sharing opens (W-11).
fn share(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.keys("secondary-3", cx);
    let on_share = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window.try_find(SHARE).and_then(|share| share.focused()) == Some(true)
        })
    };
    let mut presses = 0;
    while !on_share(&shell, cx) {
        presses += 1;
        assert!(presses < 12, "Tab never reached Share…");
        shell.keys("tab", cx);
    }
    shell.press("enter", cx);
    shell.run(cx);
    assert_eq!(shown(&shell, cx), Shown::Settings(SettingsPage::Remotes));
    escape(&shell, "Share…", cx);
    let held = holds(&shell, SHARE, cx);
    table.row(&shell, "F4 an item header's Share…", held, "Share…", cx);
}

/// F4: a lifecycle dialog's Open Sharing & keys (W-21). The dialog closes
/// onto its own origin, Activity's list, which its Open… gave it, then
/// routes (fact 22).
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
    let origin = list_focus(&shell, cx);
    shell.click("lifecycle-sharing-and-keys", cx);
    assert!(!shell.present("lifecycle-dialog", cx), "the dialog closed");
    assert_eq!(shown(&shell, cx), Shown::Settings(SettingsPage::Keys));
    escape(&shell, "a lifecycle dialog's route", cx);
    let held = holds_handle(&shell, &origin, cx);
    table.row(
        &shell,
        "F4 a lifecycle dialog's route",
        held,
        "the dialog's origin, Activity's list",
        cx,
    );
}

/// F8: Ctrl+, in a repository's menu opened by keyboard, which keeps no
/// `restore`: back to its context, the Repositories list.
fn repository_menu(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    shell.keys("secondary-1", cx);
    let list = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.repositories_list_focus(cx));
    assert!(shell.frame(cx, |window, _| list.is_focused(window)));
    shell.keys("shift-f10", cx);
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.repository_menu_open());
    assert!(open, "the menu opened by keyboard");
    assert!(in_context(&shell, "PopupMenu", cx), "the menu holds focus");
    shell.keys("secondary-,", cx);
    assert!(settings_open(&shell, cx), "Ctrl+, opened Settings");
    escape(&shell, "a repository menu", cx);
    let held = holds_handle(&shell, &list, cx);
    table.row(
        &shell,
        "F8 Ctrl+, in a repository menu opened by keyboard",
        held,
        "the Repositories list, the menu's context",
        cx,
    );
}

/// F8: Ctrl+, in a dock menu opened by keyboard on Activity's footer
/// toggle: back to the toggle, where the menu returns.
fn dock_menu(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    let toggle = footer(&shell, Control::Panel(PanelKind::Activity), cx);
    focus(&shell, &toggle, cx);
    shell.keys("shift-f10", cx);
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.dock_menu_open());
    assert!(open, "the dock menu opened by keyboard");
    assert!(in_context(&shell, "PopupMenu", cx), "the menu holds focus");
    shell.keys("secondary-,", cx);
    assert!(settings_open(&shell, cx), "Ctrl+, opened Settings");
    escape(&shell, "a dock menu", cx);
    let held = holds_handle(&shell, &toggle, cx);
    table.row(
        &shell,
        "F8 Ctrl+, in a dock menu opened by keyboard",
        held,
        "Activity's footer toggle, which the menu returns to",
        cx,
    );
}

/// F4, W-31 inside the sheet: opened from the footer's Settings, then
/// Checking for updates' Set up sharing… arrives on Sharing. Escape gives
/// focus to the first origin.
async fn inside(table: &mut Table, cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", false)], cx);
    let shell = &rig.shell;
    let settings = footer(shell, Control::Settings, cx);
    focus(shell, &settings, cx);
    shell.press("enter", cx);
    assert!(settings_open(shell, cx));
    shell.click(("settings-page", 4usize), cx);
    shell
        .until_drawn(cx, "the page's rows", |shell, cx| {
            shell.present(("polling-name", 0usize), cx)
        })
        .await;
    let row = rig.row("a", cx);
    shell.click(("polling-setup", row), cx);
    assert_eq!(shown(shell, cx), Shown::Settings(SettingsPage::Remotes));
    escape(shell, "W-31", cx);
    let held = holds_handle(shell, &settings, cx);
    table.row(
        shell,
        "F4 W-31 inside the sheet",
        held,
        "the first origin, the footer's Settings",
        cx,
    );
}

/// The repository's initials, in its own Git configuration.
fn set_initials(root: &Path, initials: &str) {
    let repository = git2::Repository::open(root).unwrap();
    let mut config = repository
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    config.set_str("manyhands.initials", initials).unwrap();
}

/// F9: the save banner's Open Settings (W-30), on a create refused for
/// want of initials. While Settings is open the banner goes: its tab
/// settles and closes, as a landing does (the banner does not leave when
/// the identity is set: nothing saves again, and Save cannot be pressed
/// behind the sheet). Focus goes to the content region the banner was in.
/// This row cannot fail: Content is also where every fallback lands, so
/// `gone_from_its_region` is F9's discriminating row (orchestrator, phase
/// A).
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
    shell.click("item-save-settings", cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert_eq!(
        shown(&shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    let tab = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.shown_tab())
        .expect("the new ticket's tab is shown");
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.close_settled(tab, cx));
    shell.run(cx);
    settle(&shell, cx);
    assert!(!shell.present("item-save-settings", cx), "the banner went");
    escape(&shell, "the save banner", cx);
    let held = region(&shell, cx) == Some(Region::Content);
    table.row(
        &shell,
        "F9 the save banner's Open Settings, its banner gone",
        held,
        "the content region the banner was in",
        cx,
    );
}

/// F9, added: an origin in a region other than Content. Ctrl+, from the
/// tickets list; the chosen repository is removed in Settings, so the
/// list is no longer drawn. Focus goes to the Tickets region.
async fn gone_from_its_region(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    shell.use_list(Pane::Tickets, cx);
    let list = tickets_list(&shell, cx);
    assert!(shell.frame(cx, |window, _| list.is_focused(window)));
    shell.keys("secondary-,", cx);
    manage(&shell, &root, cx);
    shell.click("remove-submit", cx);
    shell.click("confirm-ok", cx);
    let removal = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoRemove { .. })
    })[0];
    shell.until_ended(removal, cx).await;
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.navigation().selected().is_none()
    })
    .await;
    settle(&shell, cx);
    escape(&shell, "a list that has gone", cx);
    let held = region(&shell, cx) == Some(Region::Tickets);
    table.row(
        &shell,
        "F9 (added) a ticket list row whose list has gone",
        held,
        "the Tickets region",
        cx,
    );
}

/// F10: a route raised with nothing focused, so there is no origin.
fn no_origin(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    route(&shell, cx, |workspace, window, cx| {
        window.blur(cx);
        workspace.open_settings(SettingsPage::Polling, window, cx)
    });
    escape(&shell, "no origin", cx);
    let held = region(&shell, cx) == Some(Region::Content);
    table.row(&shell, "F10 no origin", held, "the content region", cx);
}
