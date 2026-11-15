//! P8, `late_work_moves_no_focus_after_add_closes` (design B9 G12; B3 step
//! 2; B10; R-26): work that lands after the sheet closed moves no focus and
//! opens no dialog. Add pressed while its check is held, then Escape: the
//! check lands, nothing opens, and reopening shows the landed check with no
//! pending Add. A Browse answer after the close. An add's success after
//! the close: its next open finds the text cleared and the check forgotten
//! (decision 16). A success for an earlier open does not close the sheet
//! opened since: it keeps its text and focus, and forgets only the stale
//! check of the folder just added, so it is checked again (B10, B2 step 7;
//! Task 4 review, Minor 4).

use gpui_kit::TestAppContext;

use super::support::*;
use crate::desktop::{
    services::{JobKind, ReadKind},
    view::harness::{Shell, key},
};

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    pending_check(table, cx);
    browse(table, cx);
    success_after_close(table, cx).await;
    earlier_open(table, cx).await;
}

/// Focus is still on `+`, the origin the close gave it to, and no dialog
/// is open.
fn unchanged(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    plus: &gpui_kit::FocusHandle,
    cx: &mut TestAppContext,
) {
    table.fact(
        &format!("{row} (no dialog)"),
        !has_dialog(shell, cx),
        "no dialog opening",
    );
    let held = all_closed(shell, cx) && holds_handle(shell, plus, cx);
    table.row(shell, row, held, "+, where the close put it", cx);
}

/// G12: the folder check an Add waited for lands after Escape.
fn pending_check(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    let plus = press_plus(&shell, cx);
    let row = "G12 a folder check landing after the close";
    if !opened(table, &shell, row, cx) {
        return;
    }
    set(&shell, "add-path", plain.to_str().unwrap(), cx);
    let held = shell.backend.hold_read(ReadKind::Inspect);
    focus_control(&shell, "add-submit", cx);
    shell.click("add-submit", cx);
    table.fact(
        &format!("{row} (Add waits)"),
        !has_dialog(&shell, cx),
        "Add waiting for its check",
    );
    shell.keys("escape", cx);
    drop(held);
    shell.run(cx);
    shell.keys_landed(cx);
    unchanged(table, &shell, row, &plus, cx);

    press_plus(&shell, cx);
    let row = "G12 reopened after the check landed";
    if !opened(table, &shell, row, cx) {
        return;
    }
    let (checked, pending) = of_sheet(&shell, cx, |sheet| {
        (sheet.checked().map(str::to_owned), sheet.submit_pending())
    })
    .unwrap_or_default();
    table.fact(
        &format!("{row} (the check kept)"),
        checked.as_deref() == Some(key(&plain).as_str()),
        &format!("the landed check shown (it is {checked:?})"),
    );
    table.fact(
        &format!("{row} (no pending Add)"),
        !pending && !has_dialog(&shell, cx),
        "no pending Add, no confirmation",
    );
    shell.keys("escape", cx);
}

/// G12: Browse's answer lands after Escape.
fn browse(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    let plus = press_plus(&shell, cx);
    let row = "G12 a Browse answer after the close";
    if !opened(table, &shell, row, cx) {
        return;
    }
    shell.click("add-browse", cx);
    shell.keys("escape", cx);
    cx.simulate_path_prompt_response(move |_| Some(vec![plain]));
    shell.run(cx);
    shell.keys_landed(cx);
    unchanged(table, &shell, row, &plus, cx);
}

/// G12, B10: the add succeeds after the sheet closed. Its next open finds
/// the typed text cleared and the check forgotten.
async fn success_after_close(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    let plus = press_plus(&shell, cx);
    let row = "G12 an add's success after the close";
    if !opened(table, &shell, row, cx) || !preview(&shell, &plain, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    let held = shell.backend.hold_job(JobKind::RepoEnable);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    // The confirmation, then the sheet, while the add runs.
    shell.keys("escape", cx);
    shell.keys("escape", cx);
    table.fact(
        &format!("{row} (closed first)"),
        all_closed(&shell, cx),
        "the sheet closed",
    );
    held.release();
    let id = enables(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    settle(&shell, cx);
    unchanged(table, &shell, row, &plus, cx);

    press_plus(&shell, cx);
    let row = "G12 reopened after a success after the close";
    if opened(table, &shell, row, cx) {
        let (text, checked) = (
            value(&shell, "add-path", cx),
            of_sheet(&shell, cx, |sheet| sheet.checked().map(str::to_owned)).flatten(),
        );
        table.fact(
            &format!("{row} (decision 16)"),
            text.as_deref() == Some("") && checked.is_none(),
            &format!("the text cleared and the check forgotten (text {text:?}, check {checked:?})"),
        );
        shell.keys("escape", cx);
    }
    until_onboarded(&shell, &plain, cx).await;
}

/// G12, R-26: a success for an earlier open, landing while the sheet is
/// open again, does not close it: it stays open with focus where it was,
/// its text kept and its check of the folder, which said "not yet added",
/// forgotten (B10).
async fn earlier_open(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    press_plus(&shell, cx);
    let row = "G12 a success for an earlier open";
    if !opened(table, &shell, row, cx) || !preview(&shell, &plain, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    let held = shell.backend.hold_job(JobKind::RepoEnable);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    shell.keys("escape", cx);
    press_plus(&shell, cx);
    let reopened = opened(table, &shell, &format!("{row} (reopened)"), cx);
    let plain_text = key(&plain);
    let kept = of_sheet(&shell, cx, |sheet| sheet.checked().map(str::to_owned)).flatten();
    table.fact(
        &format!("{row} (the check kept at the reopen)"),
        kept.as_deref() == Some(plain_text.as_str()),
        &format!("the check of plain kept (it is {kept:?})"),
    );
    held.release();
    let id = enables(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    if !reopened {
        return;
    }
    settle(&shell, cx);
    let (text, checked) = (
        value(&shell, "add-path", cx),
        of_sheet(&shell, cx, |sheet| sheet.checked().map(str::to_owned)).flatten(),
    );
    table.fact(
        &format!("{row} (B10, its check forgotten)"),
        text.as_deref() == Some(plain_text.as_str()) && checked.is_none(),
        &format!("the text kept and the check forgotten (text {text:?}, check {checked:?})"),
    );
    let held_focus =
        add_open(&shell, cx) && !has_dialog(&shell, cx) && holds(&shell, "add-path", cx);
    table.row(
        &shell,
        row,
        held_focus,
        "add-path, the reopened sheet still open",
        cx,
    );
    shell.keys("escape", cx);
    until_onboarded(&shell, &plain, cx).await;
}
