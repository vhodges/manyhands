//! P6, `add_from_settings_returns_to_settings` (design B2 step 2, B3 step 3,
//! B11; G3 to G6, G17, G21; R-22): Settings opened by the footer's Settings
//! item, whose handle is O_s, the origin Settings records. The sheet opened
//! from Settings' "Add or create a repository…" (AW-3) replaces it; each
//! Kit close and Cancel returns to Settings on that button (G4), a success
//! returns on the new repository (G5), and Settings' own close returns to
//! O_s (G6). A direct call while Sharing & keys holds a typed passphrase
//! empties it (G3). A later open by `+` does not return to Settings. A
//! direct route into Settings closes the sheet quietly (G17), and Open its
//! settings (G21; its button is Task 5's, so `add_closed(ToSettings)` is
//! called as the button will call it) opens Settings on the repository.
//!
//! Two rows are B2's edge cases, as B9 states them (the owner's Ruling 16):
//! Settings opened with no origin, then AW-3 (G4); and an `Inspect` route
//! in place on a sheet opened from Settings (B2 step 5, then G4).

use std::path::Path;

use gpui_kit::{ElementId, Entity, FocusHandle, TestAppContext, component::input::InputState};

use super::support::*;
use crate::desktop::{
    model::stamp::SettingsPage,
    view::{
        footer::Control,
        harness::Shell,
        settings::SettingsRoute,
        workspace::{
            Region,
            add_sheet::{AddClose, AddRoute},
        },
    },
};

const SENTINEL: &str = "sentinel-passphrase-c05a";
const PASSPHRASE: [&str; 2] = ["key-generate-passphrase", "key-generate-confirm"];

/// Opens Settings by the footer's Settings item, focused and pressed with
/// Enter. Gives O_s, that item.
fn open_settings(shell: &Shell, cx: &mut TestAppContext) -> FocusHandle {
    let item = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.footer_focus(Control::Settings));
    focus(shell, &item, cx);
    shell.press("enter", cx);
    assert!(
        settings_on(shell, SettingsPage::Repositories, cx),
        "Settings opened"
    );
    item
}

/// AW-3: Settings' Add or create a repository…, pressed. False, and
/// recorded, when it is not drawn.
fn press_add_button(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) -> bool {
    let drawn = shell.present(ADD_BUTTON.0, cx);
    table.fact(
        &format!("{row} (Settings' button drawn)"),
        drawn,
        "Add or create a repository…",
    );
    if drawn {
        shell.click(ADD_BUTTON.0, cx);
    }
    drawn
}

/// G4: Settings › Repositories on its Add or create a repository…, the
/// Kit button itself.
fn back_on_the_button(shell: &Shell, cx: &mut TestAppContext) -> bool {
    settings_on(shell, SettingsPage::Repositories, cx) && shell.holds_control(ADD_BUTTON, cx)
}

/// G6: Settings closes by Escape onto `origin`.
fn settings_closes_onto(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    origin: &FocusHandle,
    cx: &mut TestAppContext,
) {
    shell.keys("escape", cx);
    let held = all_closed(shell, cx) && holds_handle(shell, origin, cx);
    table.row(
        shell,
        &format!("G6 after {row}, Settings closed"),
        held,
        "O_s",
        cx,
    );
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    every_close(table, cx);
    passphrase(table, cx);
    success_then_plus(table, cx).await;
    route_into_settings(table, cx);
    open_its_settings(table, cx);
    no_settings_origin(table, cx);
    inspect_in_place(table, cx);
}

/// G3 by AW-3's button; G4 for each Kit close and Cancel; G6 after each.
fn every_close(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, _) = plain_shell(cx);
    for way in KEPT {
        let o_s = open_settings(&shell, cx);
        let row = format!("G4 opened from Settings, closed by {way:?}");
        if !press_add_button(table, &shell, &row, cx) {
            shell.keys("escape", cx);
            return;
        }
        if !opened(table, &shell, &row, cx) {
            return;
        }
        let held = holds(&shell, "add-path", cx);
        table.row(
            &shell,
            &format!("G3 AW-3, before {way:?}: G1"),
            held,
            "add-path",
            cx,
        );
        let drawn = close(&shell, way, cx);
        table.fact(
            &format!("{row} (its control drawn)"),
            drawn,
            "the way's control",
        );
        let held = back_on_the_button(&shell, cx);
        table.row(
            &shell,
            &row,
            held,
            "Settings › Repositories, its Add or create a repository…",
            cx,
        );
        settings_closes_onto(table, &shell, &row, &o_s, cx);
    }
}

/// G3, by a direct call while Sharing & keys holds a typed passphrase:
/// Settings closes and the passphrase is emptied; the close returns to
/// Settings › Repositories on its button (G4), whatever page was shown.
fn passphrase(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, _) = plain_shell(cx);
    let o_s = open_settings(&shell, cx);
    shell.click(("settings-page", 1usize), cx);
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone());
    let inputs: [Entity<InputState>; 2] = shell.visual(cx).update(|window, cx| {
        let keys = host.update(cx, |host, cx| host.keys(window, cx));
        let inputs = PASSPHRASE.map(|id| keys.read(cx).input(id));
        for input in &inputs {
            input.update(cx, |input, cx| input.set_value(SENTINEL, window, cx));
        }
        inputs
    });
    shell.run(cx);
    let emptied = |cx: &mut TestAppContext| {
        inputs
            .iter()
            .all(|input| input.read_with(cx, |input, _| input.value().is_empty()))
    };
    assert!(!emptied(cx), "the passphrase is typed");
    route(&shell, cx, |workspace, window, cx| {
        workspace.show_add_repository(AddRoute::New, None, window, cx)
    });
    let row = "G3 a direct call over Sharing & keys";
    table.fact(
        &format!("{row} (the passphrase)"),
        emptied(cx),
        "the typed passphrase emptied",
    );
    if !opened(table, &shell, row, cx) {
        return;
    }
    let held = holds(&shell, "add-path", cx);
    table.row(&shell, &format!("{row}: G1"), held, "add-path", cx);
    shell.keys("escape", cx);
    let held = back_on_the_button(&shell, cx);
    table.row(
        &shell,
        "G4 after G3's direct call, closed by Escape",
        held,
        "Settings › Repositories, its button",
        cx,
    );
    settings_closes_onto(table, &shell, "G3's direct call", &o_s, cx);
}

/// G5 (and G14): a successful add returns to Settings › Repositories, the
/// new repository chosen, on its entry (the `Manage` route's); G6. Then a
/// later open by `+` does not return to Settings.
async fn success_then_plus(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    let o_s = open_settings(&shell, cx);
    let row = "G5 opened from Settings, a successful add";
    if !press_add_button(table, &shell, row, cx) || !opened(table, &shell, row, cx) {
        return;
    }
    if !preview(&shell, &plain, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    confirm_and_end(&shell, cx).await;
    let held = settings_on(&shell, SettingsPage::Repositories, cx)
        && chosen(&shell, cx) == Some(plain.clone())
        && holds(&shell, ("settings-page", 0usize), cx);
    table.row(
        &shell,
        row,
        held,
        "Settings › Repositories on the new repository, its entry",
        cx,
    );
    settings_closes_onto(table, &shell, row, &o_s, cx);
    until_onboarded(&shell, &plain, cx).await;

    let plus = press_plus(&shell, cx);
    let row = "P6 a later open by +";
    if !opened(table, &shell, row, cx) {
        return;
    }
    shell.keys("escape", cx);
    let held = all_closed(&shell, cx) && holds_handle(&shell, &plus, cx);
    table.row(&shell, row, held, "+, not Settings", cx);
}

/// G17 (R-22), by a direct call: a route into Settings while the sheet is
/// open lands as Cycle 04's F1 or F2, and the sheet is closed quietly.
fn route_into_settings(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, a, _) = plain_shell(cx);
    let routes = [
        (
            "F1 Identity(root)",
            SettingsRoute::Identity(a.clone()),
            SettingsPage::Repositories,
            ElementId::from("identity-name"),
        ),
        (
            "F2 Page(Polling)",
            SettingsRoute::Page(SettingsPage::Polling),
            SettingsPage::Polling,
            ElementId::from(("settings-page", 2usize)),
        ),
    ];
    for (name, settings_route, page, target) in routes {
        let plus = press_plus(&shell, cx);
        let row = format!("G17 {name} while the sheet is open");
        if !opened(table, &shell, &row, cx) {
            return;
        }
        route(&shell, cx, move |workspace, window, cx| {
            workspace.show_settings(settings_route, None, window, cx)
        });
        let flag = of_sheet(&shell, cx, |sheet| sheet.open_flag());
        table.fact(
            &format!("{row} (the sheet closed quietly)"),
            flag == Some(false),
            "the sheet's open flag down",
        );
        let held = settings_on(&shell, page, cx) && holds(&shell, target, cx);
        table.row(
            &shell,
            &row,
            held,
            &format!("Settings' route, as Cycle 04's {name}"),
            cx,
        );
        shell.keys("escape", cx);
        let held = all_closed(&shell, cx) && holds_handle(&shell, &plus, cx);
        table.row(
            &shell,
            &format!("{row}, Settings closed"),
            held,
            "+, the sheet's origin",
            cx,
        );
    }
}

/// G21: Open its settings (`add_closed(ToSettings(root))`, as its button
/// calls it) opens Settings › Repositories on `root`, on its entry; on its
/// close, the sheet's origin, or O_s when the sheet came from Settings.
fn open_its_settings(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, a, _) = plain_shell(cx);
    let to_settings = |shell: &Shell, a: &Path, cx: &mut TestAppContext| {
        let a = a.to_owned();
        route(shell, cx, move |workspace, window, cx| {
            workspace.add_closed(AddClose::ToSettings(a), window, cx)
        });
    };
    for from_settings in [false, true] {
        let origin = if from_settings {
            open_settings(&shell, cx)
        } else {
            press_plus(&shell, cx)
        };
        let row = format!("G21 Open its settings, from Settings {from_settings}");
        if from_settings && !press_add_button(table, &shell, &row, cx) {
            shell.keys("escape", cx);
            return;
        }
        if !opened(table, &shell, &row, cx) {
            return;
        }
        to_settings(&shell, &a, cx);
        let held = settings_on(&shell, SettingsPage::Repositories, cx)
            && chosen(&shell, cx) == Some(a.clone())
            && holds(&shell, ("settings-page", 0usize), cx);
        table.row(
            &shell,
            &row,
            held,
            "Settings › Repositories on the repository, its entry",
            cx,
        );
        shell.keys("escape", cx);
        let held = all_closed(&shell, cx) && holds_handle(&shell, &origin, cx);
        let wanted = if from_settings {
            "O_s"
        } else {
            "the sheet's origin, +"
        };
        table.row(&shell, &format!("{row}, Settings closed"), held, wanted, cx);
    }
}

/// G4, B2 edge (Ruling 16): Settings opened with nothing focused, so it
/// records no origin; then AW-3, and Escape. G4: back to Settings on its
/// button; G6: Settings' close, with no origin, the content region.
fn no_settings_origin(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, _) = plain_shell(cx);
    route(&shell, cx, |workspace, window, cx| {
        window.blur(cx);
        workspace.open_settings(SettingsPage::Repositories, window, cx)
    });
    let row = "G4 (edge) Settings with no origin, then AW-3, closed by Escape";
    if !press_add_button(table, &shell, row, cx) || !opened(table, &shell, row, cx) {
        return;
    }
    shell.keys("escape", cx);
    let held = back_on_the_button(&shell, cx);
    table.row(&shell, row, held, "Settings › Repositories, its button", cx);
    shell.keys("escape", cx);
    let held = all_closed(&shell, cx) && region(&shell, cx) == Some(Region::Content);
    table.row(
        &shell,
        &format!("G6 after {row}"),
        held,
        "the content region (no origin)",
        cx,
    );
}

/// G4, B2 edge (Ruling 16): opened from Settings, then an `Inspect` route
/// applies in place (B2 step 5); Escape still returns to Settings.
fn inspect_in_place(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, a, _) = plain_shell(cx);
    let o_s = open_settings(&shell, cx);
    let row = "G4 (edge) opened from Settings, an Inspect route in place, closed by Escape";
    if !press_add_button(table, &shell, row, cx) || !opened(table, &shell, row, cx) {
        return;
    }
    route(&shell, cx, move |workspace, window, cx| {
        workspace.show_add_repository(AddRoute::Inspect(a), None, window, cx)
    });
    table.fact(
        &format!("{row} (in place)"),
        add_open(&shell, cx),
        "the sheet still open",
    );
    shell.keys("escape", cx);
    let held = back_on_the_button(&shell, cx);
    table.row(&shell, row, held, "Settings › Repositories, its button", cx);
    settings_closes_onto(table, &shell, row, &o_s, cx);
}
