//! P22, `late_work_moves_no_focus` (design A6 F17; A2 step 2; A4): work
//! that lands after Settings closed moves no focus and opens nothing, with
//! the panel's selection unchanged throughout, and reopening shows its
//! result. A route raised after the Kit closed the sheet but before any
//! render empties a typed passphrase first and records its own origin. The
//! key-removal row is P24's (Task 4). Each focus read is after two draws
//! (Ruling 2).

use gpui_kit::{TestAppContext, component::WindowExt};

use super::{
    returns::{escape, footer, settings_open, ticket_shell},
    support::{emptied, has_dialog, holds_handle, host, open_on_keys, route, settle, shown},
    table::Table,
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::ReadKind,
    test_support::fixtures::{Fixture, identity},
    view::{
        footer::Control,
        harness::Shell,
        settings::{SettingsRoute, Shown},
    },
};

pub(super) fn rows(table: &mut Table, cx: &mut TestAppContext) {
    pending_add(table, cx);
    hosts_landing(table, cx);
    route_before_render(table, cx);
}

/// F17, R-48: a folder typed and Add pressed while its check is held, then
/// Escape; the check lands after the close. No confirmation opens, focus is
/// where the close put it, and reopening shows the check's result and no
/// pending Add.
fn pending_add(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.refresh(&root);
    let plain = fixture.plain_repository("plain", Some(identity()));
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    let origin = shell
        .frame(cx, |window, cx| window.focused(cx))
        .expect("something holds focus");
    shell.keys("secondary-,", cx);
    let host = host(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        let page = host.update(cx, |host, cx| host.repositories(window, cx));
        let input = page.read(cx).input("add-path");
        let text = plain.to_str().expect("a UTF-8 path").to_owned();
        input.update(cx, |input, cx| input.set_value(text, window, cx));
    });
    shell.run(cx);
    let held = shell.backend.hold_read(ReadKind::Inspect);
    shell.click("add-submit", cx);
    assert!(!has_dialog(&shell, cx), "Add waits for its folder's check");
    escape(&shell, "the pending Add", cx);
    assert!(
        holds_handle(&shell, &origin, cx),
        "the close gave focus back"
    );

    drop(held);
    shell.run(cx);
    settle(&shell, cx);
    let opened = has_dialog(&shell, cx);
    table.fact(
        "F17 a folder check landing after the close",
        !opened,
        "no confirmation opening",
    );
    let held = holds_handle(&shell, &origin, cx);
    table.row(
        &shell,
        "F17 a folder check landing after the close",
        held,
        "the origin the close gave focus to, unchanged",
        cx,
    );
    if opened {
        // Today's confirmation, closed so that Settings can open again.
        shell.keys("escape", cx);
        assert!(!has_dialog(&shell, cx));
    }

    shell.keys("secondary-,", cx);
    assert!(settings_open(&shell, cx), "Settings opens again");
    settle(&shell, cx);
    table.fact(
        "F17 reopening after the folder check landed shows its result",
        shell.present("add-inspection", cx),
        "the check's result on the Add form",
    );
    table.fact(
        "F17 reopening after the folder check landed opens no pending Add",
        !has_dialog(&shell, cx),
        "no pending Add opening its confirmation",
    );
    escape(&shell, "the reopened page", cx);
}

/// F17: a Check host landing delivered to the Hosts page's handler
/// (`refocus_gone`) with the page closed and focus empty, in one update.
/// Its own focus call is read in that update (Testing: an assertion about
/// that update's own focus call, not an A6 proof); the A6 read is after two
/// draws. Reopening shows the page's Check host.
fn hosts_landing(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let hosts_route = SettingsRoute::Hosts(root.clone());
    route(&shell, cx, move |workspace, window, cx| {
        workspace.show_settings(hosts_route, None, window, cx)
    });
    assert_eq!(shown(&shell, cx), Shown::Settings(SettingsPage::Hosts));
    escape(&shell, "the Hosts page", cx);

    let host = host(&shell, cx);
    let (page, check) = shell.visual(cx).update(|window, cx| {
        let page = host.update(cx, |host, cx| host.hosts(window, cx));
        let check = page.read(cx).control_focus("hosts-check");
        (page, check)
    });
    // Not an A6 proof (Ruling 2): read inside the update that delivered the
    // landing, about that update's own focus call (design Testing).
    let moved = shell.visual(cx).update(|window, cx| {
        window.blur(cx);
        page.update(cx, |page, cx| page.land_refocus_gone(window, cx));
        check.is_focused(window)
    });
    table.fact(
        "F17 a Check host landing, its handler's own focus call",
        !moved,
        "Check host, in the closed sheet, not focused",
    );
    // This read cannot fail after a draw: an empty focus and a focus on the
    // undrawn Check host both leave the frame's focus path empty, so GPUI
    // fires `focus_lost` either way (gpui-pre window.rs:3360-3369), and the
    // workspace sends both to the same region (`focus.rs`'s region
    // fallback). Kept as A6 asks (orchestrator, phase A).
    settle(&shell, cx);
    let on_check = shell.frame(cx, |window, _| check.is_focused(window));
    table.fact(
        "F17 a Check host landing, read after two draws",
        !on_check,
        "Check host, in the closed sheet, not focused",
    );

    let hosts_route = SettingsRoute::Hosts(root.clone());
    route(&shell, cx, move |workspace, window, cx| {
        workspace.show_settings(hosts_route, None, window, cx)
    });
    assert!(shell.present("hosts-check", cx), "reopened, Check host");
    escape(&shell, "the reopened Hosts page", cx);
}

/// A2 step 2: the Kit closes the sheet itself (no `on_close`), and before
/// any render a route opens Settings again with an origin of its own. The
/// typed passphrase is emptied first, and the route's origin is the one
/// recorded.
fn route_before_render(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    let inputs = open_on_keys(&shell, cx);
    let origin = footer(&shell, Control::Settings, cx);
    let given = origin.clone();
    route(&shell, cx, move |workspace, window, cx| {
        window.close_sheet(cx);
        workspace.show_settings(
            SettingsRoute::Page(SettingsPage::Keys),
            Some(given),
            window,
            cx,
        );
    });
    table.fact(
        "A2 step 2 a route after the Kit's close, before a render",
        emptied(&inputs, cx),
        "the typed passphrase emptied first",
    );
    assert!(settings_open(&shell, cx), "the route opened Settings");
    settle(&shell, cx);
    table.fact(
        "A2 step 2 a route after the Kit's close, after two draws",
        emptied(&inputs, cx),
        "the typed passphrase still empty",
    );
    escape(&shell, "the route after the Kit's close", cx);
    let held = holds_handle(&shell, &origin, cx);
    table.row(
        &shell,
        "A2 step 2 a route after the Kit's close records its origin",
        held,
        "the route's origin, the footer's Settings",
        cx,
    );
}
