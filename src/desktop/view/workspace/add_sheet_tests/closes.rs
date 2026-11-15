//! P7, `the_add_sheet_closes_every_way` (design B3; B1's quit paths; G15;
//! decision 16): the sheet opened by `+`, text typed in both forms and an
//! Add waiting for its folder's check. Each Kit close and Cancel leaves no
//! owner, no sheet, the open flag down and no pending Add, with the typed
//! text kept; a successful add clears it; Open its settings keeps it; and
//! quitting closes the sheet quietly, as Cycle 04's F12 to F14 for
//! Settings: Quit now with Activity displayed, the origin with it hidden,
//! Quit now with the confirmation open above the sheet, and the EditsNotKept
//! dialog with unkept edits. Then two states the review named (finding 2):
//! the Kit's sheet gone under a dialog, whose close waits and settles; and
//! two closes in one frame, whose second changes no focus.

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
    gone_under_a_dialog(table, cx);
    closed_twice(table, cx);
}

/// A dialog over the open sheet, as a confirmation is.
fn probe_dialog(shell: &Shell, cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;
    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);
}

/// The dialog over the sheet, closed directly.
fn close_probe(shell: &Shell, cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;
    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);
}

/// Review finding 2: the Kit's sheet gone (a direct `close_sheet`) while a
/// dialog is over it and the workspace still owns it. Each frame finds the
/// close waiting (Ruling 17) and asks for no other; once the dialog closes,
/// it runs once, onto the origin.
fn gone_under_a_dialog(table: &mut Table, cx: &mut TestAppContext) {
    use std::{cell::Cell, rc::Rc};

    let (shell, _, _) = plain_shell(cx);
    let plus = press_plus(&shell, cx);
    let row = "P7 the Kit's sheet gone under a dialog";
    if !opened(table, &shell, row, cx) {
        return;
    }
    probe_dialog(&shell, cx);
    route(&shell, cx, |_, window, cx| {
        use gpui_kit::component::WindowExt;
        window.close_sheet(cx)
    });
    let notified = Rc::new(Cell::new(0usize));
    let count = notified.clone();
    let _watch =
        cx.update(|cx| cx.observe(&shell.workspace, move |_, _| count.set(count.get() + 1)));
    for _ in 0..4 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    let waits = owner(&shell, cx) == Some(SheetOwnerKind::AddRepository);
    table.fact(
        &format!("{row} (it settles)"),
        waits && notified.get() <= 1,
        &format!(
            "the close waiting, the owner kept, at most one notification over four frames \
             (owner {:?}, {} notifications)",
            owner(&shell, cx),
            notified.get()
        ),
    );
    close_probe(&shell, cx);
    let held = all_closed(&shell, cx) && holds_handle(&shell, &plus, cx);
    table.row(&shell, &format!("{row}, the dialog closed"), held, "+", cx);
}

/// Review finding 2's guard: a success's close waits under a dialog; the
/// dialog closes, the next frame defers the waiting close, and in that
/// frame a Kept close runs first (two closes in one frame). The deferred
/// close then finds the sheet closed: focus stays on the origin, and the
/// success still clears the typed text (decision 16; B10).
fn closed_twice(table: &mut Table, cx: &mut TestAppContext) {
    use gpui_kit::{component::WindowExt, test::TestWindowExt};

    let (shell, _, plain) = plain_shell(cx);
    let plus = press_plus(&shell, cx);
    let row = "P7 a Kept close, then a deferred Added close";
    if !opened(table, &shell, row, cx) {
        return;
    }
    set(&shell, "create-location", TYPED, cx);
    probe_dialog(&shell, cx);
    route(&shell, cx, move |workspace, window, cx| {
        workspace.add_closed(AddClose::Added(plain), window, cx)
    });
    let workspace = shell.workspace.clone();
    shell.visual(cx).update(|window, cx| {
        window.close_dialog(cx);
        // This frame's render defers the waiting close; the Kept close
        // runs before it.
        window.render_frame(cx);
        workspace.update(cx, |workspace, cx| {
            workspace.add_closed(AddClose::Kept, window, cx)
        })
    });
    shell.run(cx);
    let held = all_closed(&shell, cx) && holds_handle(&shell, &plus, cx);
    table.row(&shell, row, held, "+, the origin, kept", cx);
    let text = value(&shell, "create-location", cx);
    table.fact(
        &format!("{row} (decision 16)"),
        text.as_deref() == Some(""),
        &format!("the typed text cleared (it is {text:?})"),
    );
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
