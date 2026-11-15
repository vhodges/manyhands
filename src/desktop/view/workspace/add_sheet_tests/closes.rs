//! P7, `the_add_sheet_closes_every_way` (design B3; B1's quit paths; G15;
//! decision 16): the sheet opened by `+`, text typed in both forms and an
//! Add waiting for its folder's check. Each Kit close and Cancel leaves no
//! owner, no sheet, the open flag down and no pending Add, with the typed
//! text kept; a successful add clears it; Open its settings keeps it; and
//! quitting closes the sheet quietly, as Cycle 04's F12 to F14 for
//! Settings: Quit now with Activity displayed, the origin with it hidden,
//! Quit now with the confirmation open above the sheet, and the EditsNotKept
//! dialog with unkept edits.

use std::path::Path;

use gpui_kit::TestAppContext;

use super::{super::super::settings_sheet::SheetOwnerKind, support::*};
use crate::desktop::{
    model::layout::PanelKind,
    services::{
        JobKind, ReadKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::{delaying::JobGate, fixtures::Fixture},
    view::{
        footer::Control,
        harness::{Shell, key},
        item::ChipField,
        workspace::{QuitPhase, add_sheet::AddClose},
    },
};

/// Text typed in Create's Location, the form the rows do not submit.
const TYPED: &str = "/typed/by/hand";

/// The sheet's own state after a close: its open flag and its pending Add.
fn quiet_state(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) {
    let state = of_sheet(shell, cx, |sheet| {
        (sheet.open_flag(), sheet.submit_pending())
    });
    table.fact(
        &format!("{row} (the sheet's state)"),
        state == Some((false, false)),
        &format!("open flag down and no pending Add (it is {state:?})"),
    );
}

/// The typed text of both forms.
fn typed(shell: &Shell, cx: &mut TestAppContext) -> (Option<String>, Option<String>) {
    (
        value(shell, "add-path", cx),
        value(shell, "create-location", cx),
    )
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    kept(table, cx);
    added(table, cx).await;
    to_settings(table, cx);
    for way in [
        Quit::WithActivity,
        Quit::WithoutActivity,
        Quit::OverConfirmation,
    ] {
        quitting(table, way, cx).await;
    }
    #[cfg(unix)]
    unkept_edits(table, cx).await;
}

/// Each Kit close and Cancel: typed text kept, the pending Add forgotten.
fn kept(table: &mut Table, cx: &mut TestAppContext) {
    for way in KEPT {
        let (shell, _, plain) = plain_shell(cx);
        press_plus(&shell, cx);
        let row = format!("P7 closed by {way:?}");
        if !opened(table, &shell, &row, cx) {
            return;
        }
        set(&shell, "create-location", TYPED, cx);
        set(&shell, "add-path", plain.to_str().unwrap(), cx);
        let held = shell.backend.hold_read(ReadKind::Inspect);
        focus_control(&shell, "add-submit", cx);
        shell.click("add-submit", cx);
        let waits = of_sheet(&shell, cx, |sheet| sheet.submit_pending()) == Some(true);
        table.fact(
            &format!("{row} (an Add waits)"),
            waits,
            "an Add waiting for its check",
        );
        let drawn = close(&shell, way, cx);
        table.fact(
            &format!("{row} (its control drawn)"),
            drawn,
            "the way's control",
        );
        table.fact(
            &format!("{row} (closed)"),
            all_closed(&shell, cx),
            "no owner, no sheet",
        );
        quiet_state(table, &shell, &row, cx);
        let text = typed(&shell, cx);
        let wanted = (Some(key(&plain)), Some(TYPED.to_owned()));
        table.fact(
            &format!("{row} (decision 16)"),
            text == wanted,
            &format!("the typed text kept (it is {text:?})"),
        );
        drop(held);
        shell.run(cx);
    }
}

/// A successful add clears both forms' text and the check (decision 16).
async fn added(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    press_plus(&shell, cx);
    let row = "P7 closed by a successful add";
    if !opened(table, &shell, row, cx) {
        return;
    }
    set(&shell, "create-location", TYPED, cx);
    if !preview(&shell, &plain, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    confirm_and_end(&shell, cx).await;
    settle(&shell, cx);
    table.fact(
        &format!("{row} (closed)"),
        all_closed(&shell, cx),
        "no owner, no sheet",
    );
    quiet_state(table, &shell, row, cx);
    let text = typed(&shell, cx);
    let checked = of_sheet(&shell, cx, |sheet| sheet.checked().map(str::to_owned)).flatten();
    table.fact(
        &format!("{row} (decision 16)"),
        text == (Some(String::new()), Some(String::new())) && checked.is_none(),
        &format!("the typed text and the check cleared (text {text:?}, check {checked:?})"),
    );
    until_onboarded(&shell, &plain, cx).await;
}

/// Open its settings: Settings opens on the repository; the sheet's own
/// state is closed, its typed text kept.
fn to_settings(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, a, _) = plain_shell(cx);
    press_plus(&shell, cx);
    let row = "P7 closed by Open its settings";
    if !opened(table, &shell, row, cx) {
        return;
    }
    set(&shell, "create-location", TYPED, cx);
    route(&shell, cx, move |workspace, window, cx| {
        workspace.add_closed(AddClose::ToSettings(a), window, cx)
    });
    table.fact(
        &format!("{row} (Settings owns the sheet)"),
        owner(&shell, cx) == Some(SheetOwnerKind::Settings),
        "Settings open in its place",
    );
    quiet_state(table, &shell, row, cx);
    table.fact(
        &format!("{row} (decision 16)"),
        value(&shell, "create-location", cx).as_deref() == Some(TYPED),
        "the typed text kept",
    );
    shell.keys("escape", cx);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Quit {
    WithActivity,
    WithoutActivity,
    OverConfirmation,
}

/// A refresh of `root` held on the worker, so quitting waits for it.
fn hold_a_job(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> (JobGate, DesktopRequestId) {
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    let root = root.to_owned();
    let id = shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .services()
            .worker()
            .submit(Intent::IndexRefresh { root }, Lane::User, None, None)
    });
    held.wait_reached();
    (held, id)
}

/// G15: quitting with the sheet open, or with its confirmation above it.
async fn quitting(table: &mut Table, way: Quit, cx: &mut TestAppContext) {
    let (shell, a, plain) = plain_shell(cx);
    if way == Quit::WithoutActivity {
        shell.click(Control::Panel(PanelKind::Activity).id(), cx);
    }
    let (held, id) = hold_a_job(&shell, &a, cx);
    let plus = press_plus(&shell, cx);
    let row = format!("G15 quitting {way:?}");
    let open = opened(table, &shell, &row, cx);
    if open && way == Quit::OverConfirmation {
        let previewed = preview(&shell, &plain, cx);
        table.fact(
            &format!("{row} (its confirmation)"),
            previewed,
            "the add's confirmation open",
        );
    }
    if open {
        shell.keys("secondary-q", cx);
        let phase = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase());
        table.fact(
            &format!("{row} (waits)"),
            phase == QuitPhase::Finishing(id),
            "quitting waits for the job",
        );
        let closed = all_closed(&shell, cx) && !has_dialog(&shell, cx);
        table.fact(
            &format!("{row} (closed quietly)"),
            closed,
            "no owner, no sheet, no dialog",
        );
        quiet_state(table, &shell, &row, cx);
        let (held_focus, wanted) = if way == Quit::WithoutActivity {
            (holds_handle(&shell, &plus, cx), "the origin, + (F13)")
        } else {
            (holds(&shell, "quit-now", cx), "Quit now (F12)")
        };
        table.row(&shell, &row, held_focus, wanted, cx);
    }
    // The worker must not outlive the test: let the held job end.
    held.release();
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Finished(..))
            )
        })
        .await;
}

/// G15 with unkept edits (Cycle 04's F14): a tab's edit no draft holds,
/// crash recovery being off (the drafts folder is a link). Quitting closes
/// the sheet quietly and the EditsNotKept dialog holds focus.
#[cfg(unix)]
async fn unkept_edits(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, MAIN, "Main", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, MAIN), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    shell.until_editing(cx).await;
    shell.click(ChipField::All.id(), cx);
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input("x");
    shell.run(cx);
    shell.keys("escape", cx);
    press_plus(&shell, cx);
    let row = "G15 quitting with unkept edits";
    if !opened(table, &shell, row, cx) {
        return;
    }
    shell.keys("secondary-q", cx);
    table.fact(
        &format!("{row} (asks)"),
        shell.present("edits-not-kept-dialog", cx),
        "the EditsNotKept dialog",
    );
    table.fact(
        &format!("{row} (closed quietly)"),
        owner(&shell, cx).is_none(),
        "no owner",
    );
    quiet_state(table, &shell, row, cx);
    let held = in_context(&shell, "Dialog", cx);
    table.row(&shell, row, held, "the EditsNotKept dialog (F14)", cx);
    if shell.present("edits-not-kept-cancel", cx) {
        shell.click("edits-not-kept-cancel", cx);
    }
}
