//! P11, `add_is_enabled_only_when_it_can_go_ahead` (Wave 04 Cycle 05a
//! design B10; decision 9; R-T12b; R-27; G22): the footer's Add repository
//! is enabled when the folder field is filled and its check, once landed,
//! lets the add go ahead, B10's table row by row, and pressing it does what
//! the row says. Pressed before its check landed, it checks first and goes
//! on only if the landed check enables it; otherwise focus goes to the
//! row's action, else the folder field, and no confirmation opens (G22).
//! A disabled press is a click on the drawn button that changes nothing.

use std::{fs, path::Path};

use gpui_kit::TestAppContext;

use super::{
    enter_in, settings,
    support::{
        Table, add_enabled, canned, has_dialog, holds, land, land_only, needs_fixing, remote,
        set_up,
    },
};
use crate::desktop::{
    services::{JobKind, ReadKind, ReadResult, requests::Intent},
    test_support::fixtures::{Fixture, identity},
    view::{
        add_repository::ADDING,
        harness::{Shell, key},
    },
};

fn check(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    super::set(shell, "add-path", &key(root), cx);
    enter_in(shell, "add-path", cx);
    shell.keys_landed(cx);
}

/// The row `row`: Add repository is enabled as `enabled` says.
fn enabled(table: &mut Table, shell: &Shell, row: &str, wanted: bool, cx: &mut TestAppContext) {
    let found = add_enabled(shell, cx);
    table.fact(
        &format!("{row} (enabled {wanted})"),
        found == wanted,
        &format!("Add repository enabled {wanted} (it is {found})"),
    );
}

/// A disabled row: pressed, nothing happens (no dialog, no request, no
/// check started for other text).
fn press_does_nothing(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) {
    let before = super::sheet(shell, cx).read_with(cx, |sheet, _| {
        (sheet.inspected().map(str::to_owned), sheet.submit_pending())
    });
    shell.click("add-submit", cx);
    shell.keys_landed(cx);
    let after = super::sheet(shell, cx).read_with(cx, |sheet, _| {
        (sheet.inspected().map(str::to_owned), sheet.submit_pending())
    });
    table.fact(
        &format!("{row} (pressed)"),
        !has_dialog(shell, cx) && shell.requests(cx, |_| true).is_empty() && before == after,
        &format!("nothing done (the check was {before:?}, is {after:?})"),
    );
}

/// The confirmation is open on its preview; then it is closed by Escape.
fn previewed(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) {
    table.fact(
        &format!("{row} (the confirmation)"),
        shell.present("confirm-ok", cx),
        "the add's confirmation on its preview",
    );
    if has_dialog(shell, cx) {
        shell.keys("escape", cx);
    }
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let plain = fixture.plain_repository("plain", Some(identity()));
    let anonymous = fixture.plain_repository("anonymous", None);
    let loose = fixture.empty_directory("loose");
    fs::write(loose.join("notes.txt"), "kept\n").unwrap();
    let shell = settings(fixture, cx);

    let row = "Nothing typed";
    enabled(table, &shell, row, false, cx);
    press_does_nothing(table, &shell, row, cx);

    let row = "None yet for this text";
    super::set(&shell, "add-path", &key(&plain), cx);
    enabled(table, &shell, row, true, cx);

    // Checking: pressed, it waits for the check, then goes on (R-T12b).
    let row = "Checking";
    let held = shell.backend.hold_read(ReadKind::Inspect);
    enter_in(&shell, "add-path", cx);
    enabled(table, &shell, row, true, cx);
    shell.click("add-submit", cx);
    let waits = super::sheet(&shell, cx).read_with(cx, |sheet, _| sheet.submit_pending());
    table.fact(
        &format!("{row} (pressed, it waits)"),
        waits && !has_dialog(&shell, cx),
        "Add waiting for the check, no confirmation yet",
    );
    drop(held);
    shell.run(cx);
    shell.keys_landed(cx);
    previewed(table, &shell, &format!("{row}, then the check lands"), cx);

    let row = "Checked, not registered, not set up, a branch checked out";
    enabled(table, &shell, row, true, cx);
    shell.click("add-submit", cx);
    previewed(table, &shell, row, cx);

    // Identity Required and its fields empty: Add focuses the name.
    check(&shell, &anonymous, cx);
    let row = "Checked, identity Required, its fields empty";
    enabled(table, &shell, row, true, cx);
    shell.click("add-submit", cx);
    table.fact(
        &format!("{row} (pressed)"),
        !has_dialog(&shell, cx) && holds(&shell, "add-name", cx),
        "focus on the name, no confirmation",
    );

    // Valid, on the checked-out branch (canned): enabled.
    land(&shell, &plain, ReadResult::Ok(set_up(&plain, "main")), cx);
    enabled(table, &shell, "Checked, configuration Valid", true, cx);

    let disabled = |table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext| {
        enabled(table, shell, row, false, cx);
        press_does_nothing(table, shell, row, cx);
    };
    check(&shell, &a, cx);
    disabled(table, &shell, "Checked, registered", cx);
    land(&shell, &plain, ReadResult::Ok(needs_fixing(&plain)), cx);
    disabled(table, &shell, "Checked, configuration Invalid", cx);
    let mut detached = canned(&plain);
    detached.head_branch = None;
    land(&shell, &plain, ReadResult::Ok(detached), cx);
    disabled(table, &shell, "Checked, no branch checked out", cx);
    let mut other = set_up(&plain, "trunk");
    other.local_branches = vec!["main".to_owned(), "trunk".to_owned()];
    land(&shell, &plain, ReadResult::Ok(other), cx);
    disabled(
        table,
        &shell,
        "Checked, configuration on another main branch",
        cx,
    );
    let mut shares = set_up(&plain, "main");
    shares.configuration.publication_remote = Some("origin".to_owned());
    shares.remotes = vec![remote("origin", "https://example.invalid/r.git", false)];
    land(&shell, &plain, ReadResult::Ok(shares), cx);
    disabled(
        table,
        &shell,
        "Checked, sharing through a remote not SSH",
        cx,
    );
    check(&shell, &loose, cx);
    disabled(table, &shell, "Failed(_)", cx);
    land(&shell, &plain, ReadResult::Busy, cx);
    disabled(table, &shell, "Busy", cx);

    g22(table, &shell, &a, &plain, cx);
    adding(table, &shell, &plain, cx).await;
}

/// G22: Add pressed before the check landed, and the landed check disables
/// it: focus on the row's action, a Kit button (Open its settings for a
/// registered folder), else on the folder field (a configuration that
/// needs fixing offers none); no confirmation.
fn g22(table: &mut Table, shell: &Shell, a: &Path, plain: &Path, cx: &mut TestAppContext) {
    super::set(shell, "add-path", &key(a), cx);
    shell.click("add-submit", cx);
    shell.keys_landed(cx);
    table.fact(
        "G22 the landed check is registered",
        !has_dialog(shell, cx) && shell.holds_control(("add-open-settings", 0usize), cx),
        "focus on Open its settings, no confirmation",
    );

    super::set(shell, "add-path", &key(plain), cx);
    let held = shell.backend.hold_read(ReadKind::Inspect);
    shell.click("add-submit", cx);
    land_only(shell, &key(plain), ReadResult::Ok(needs_fixing(plain)), cx);
    drop(held);
    shell.run(cx);
    shell.keys_landed(cx);
    table.fact(
        "G22 the landed check needs fixing",
        !has_dialog(shell, cx) && holds(shell, "add-path", cx),
        "focus on the folder field, no confirmation",
    );
}

/// An add from this open is pending: Add repository reads "Adding…" and is
/// disabled (R-27). Its success closes the sheet.
async fn adding(table: &mut Table, shell: &Shell, plain: &Path, cx: &mut TestAppContext) {
    check(shell, plain, cx);
    shell.click("add-submit", cx);
    if !shell.present("confirm-ok", cx) {
        table.fact("An add pending (its preview)", false, "the add's preview");
        return;
    }
    let held = shell.backend.hold_job(JobKind::RepoEnable);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    let label = shell.label("add-submit", cx);
    let row = "An add from this open is pending";
    enabled(table, shell, row, false, cx);
    table.fact(
        &format!("{row} (Adding…)"),
        label.as_deref() == Some(ADDING),
        &format!("Add repository reading Adding… (it reads {label:?})"),
    );
    held.release();
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoEnable { .. })
    })[0];
    shell.until_ended(id, cx).await;
    let wanted = plain.to_owned();
    shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                matches!(&request.intent, Intent::IndexRefresh { root } if *root == wanted)
                    && matches!(
                        request.state,
                        crate::desktop::services::requests::RequestState::Finished(..)
                    )
            })
        })
        .await;
}
