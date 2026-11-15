//! P5, `focus_returns_to_what_opened_add` (design B9 G7 to G11, G13, G14,
//! G23; B3 step 3; R-25): one row per origin kind, each read after two
//! draws. The origins: the toolbar's `+` (closed the Kit's three ways,
//! Cancel and a successful add, G7 and G14); the lifecycle dialog's own
//! origin (G7); an Activity row by its button, its menu and Enter (G8); a
//! popover row, its footer item shown or hidden by close time (G9); the
//! empty panel's action, gone while the sheet was open (G10); no origin
//! (G11). Then the confirmation opened from the sheet (G13), and a success
//! that lands while an Unlock prompt is open (G23).

use std::fs;

use gpui_kit::TestAppContext;
use manyhands::results::ResultCode;

use super::{
    support::*,
    ways::{
        attention_item, empty_action, inspect_label, inspect_row, open_repository_details,
        press_in_popover, saved_ticket,
    },
};
use crate::desktop::{
    model::{activity::rows::RowKey, stamp::SettingsPage},
    services::JobKind,
    test_support::fixtures::{Fixture, enable_through, identity},
    view::{
        activity::tests::cases::answered,
        activity_support::{button_focus, choose_in_menu, highlighted, list_focus, to_row},
        add_repository::ADDING,
        footer::Control,
        harness::Shell,
        workspace::{Region, add_sheet::AddRoute},
    },
};
use manyhands::repository::RepositoryService;

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    plus_every_way(table, cx).await;
    lifecycle(table, cx).await;
    activity_row(table, cx);
    popover(table, cx);
    gone(table, cx);
    no_origin(table, cx);
    confirmation(table, cx).await;
    while_adding(table, cx).await;
    under_a_prompt(table, cx).await;
    under_a_prompt_from_settings(table, cx).await;
}

/// G7 (and G14): opened by the toolbar's `+`, closed by Escape, the close
/// button, the scrim and Cancel, then by a successful add whose finish
/// closes the confirmation in the same batch: focus on `+`.
async fn plus_every_way(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    for way in KEPT {
        let plus = press_plus(&shell, cx);
        let row = format!("G7 opened by +, closed by {way:?}");
        if !opened(table, &shell, &row, cx) {
            // Today's `+` opens Settings: closed, so the next way starts
            // where this one did.
            shell.keys("escape", cx);
            continue;
        }
        let drawn = close(&shell, way, cx);
        table.fact(
            &format!("{row} (its control drawn)"),
            drawn,
            "the way's control",
        );
        let held = all_closed(&shell, cx) && holds_handle(&shell, &plus, cx);
        table.row(&shell, &row, held, "+", cx);
    }
    let plus = press_plus(&shell, cx);
    let row = "G7 and G14 opened by +, closed by a successful add";
    if !opened(table, &shell, row, cx) {
        shell.keys("escape", cx);
        return;
    }
    let previewed = preview(&shell, &plain, cx);
    table.fact(
        &format!("{row} (its preview)"),
        previewed,
        "the add's preview",
    );
    if !previewed {
        return;
    }
    confirm_and_end(&shell, cx).await;
    let held = all_closed(&shell, cx) && !has_dialog(&shell, cx) && holds_handle(&shell, &plus, cx);
    table.row(
        &shell,
        row,
        held,
        "+, the sheet and its confirmation closed",
        cx,
    );
    until_onboarded(&shell, &plain, cx).await;
}

/// G7: AW-4. The lifecycle dialog focused its own origin, Activity's list,
/// before it routed: that is the sheet's origin.
async fn lifecycle(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, root) = one_repo(cx, saved_ticket);
    let origin = open_repository_details(&shell, &root, cx).await;
    let row = "G7 opened by the lifecycle dialog's Open repository details";
    if !opened(table, &shell, row, cx) {
        return;
    }
    shell.keys("escape", cx);
    let held = all_closed(&shell, cx) && holds_handle(&shell, &origin, cx);
    table.row(
        &shell,
        row,
        held,
        "the dialog's origin, Activity's list",
        cx,
    );
}

/// G8: AW-5 by an Activity row's button, its menu and Enter on the list:
/// back to the list, the row still highlighted (Cycle 04's F5).
fn activity_row(table: &mut Table, cx: &mut TestAppContext) {
    for way in ["its button", "its row menu", "Enter"] {
        let (shell, root) = one_repo(cx, |_, _| ());
        let id = inspect_row(&shell, 1, &root, cx);
        let key = RowKey::Request(id);
        to_row(&shell, &key, cx);
        match way {
            "its button" => {
                let button = button_focus(&shell, cx);
                focus(&shell, &button, cx);
                shell.press("enter", cx);
            }
            "its row menu" => choose_in_menu(&shell, &key, inspect_label(), cx),
            _ => shell.keys("enter", cx),
        }
        let row = format!("G8 an Activity row, by {way}");
        if !opened(table, &shell, &row, cx) {
            continue;
        }
        shell.keys("escape", cx);
        let list = list_focus(&shell, cx);
        let held = all_closed(&shell, cx)
            && holds_handle(&shell, &list, cx)
            && highlighted(&shell, cx).as_ref() == Some(&key);
        table.row(
            &shell,
            &row,
            held,
            "Activity's list, the row highlighted",
            cx,
        );
    }
}

/// G9: AW-5 by a popover row, its footer item still shown at close time,
/// then hidden by it (the row dismissed while the sheet is open): the item,
/// else E1's control (Cycle 04's F6 and F7; 03b R-49).
fn popover(table: &mut Table, cx: &mut TestAppContext) {
    for hidden in [false, true] {
        let (shell, root) = one_repo(cx, |_, _| ());
        let id = inspect_row(&shell, 1, &root, cx);
        let item = attention_item(&shell, cx);
        press_in_popover(&shell, &RowKey::Request(id), cx);
        let row = format!("G9 a popover row, its footer item hidden {hidden}");
        if !opened(table, &shell, &row, cx) {
            continue;
        }
        if hidden {
            shell
                .workspace
                .update(cx, |workspace, cx| workspace.dismiss_request(id, cx));
            shell.run(cx);
            settle(&shell, cx);
        }
        let back = shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.attention_restore_target(cx));
        table.fact(
            &format!("{row} (the item as wanted)"),
            (back != item) == hidden,
            "the footer item hidden only when the row went",
        );
        shell.keys("escape", cx);
        let held = all_closed(&shell, cx) && holds_handle(&shell, &back, cx);
        let wanted = if hidden {
            "E1's control"
        } else {
            "the footer's Needs attention"
        };
        table.row(&shell, &row, held, wanted, cx);
    }
}

/// G10: AW-1, the empty panel's action reached by Tab and pressed with
/// Enter. While the sheet is open another actor adds a repository and the
/// list is read again, so the action is no longer drawn: on close, focus
/// goes to the Repositories region, not where the closed sheet's frame
/// would send it (the content region).
fn gone(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let other = fixture.plain_repository("other", Some(identity()));
    let shell = Shell::open(fixture, cx);
    let reached = shell.tab_reaches(empty_action(), 20, cx);
    table.fact(
        "G10 (Tab reaches the empty panel's action)",
        reached,
        "Tab to the action",
    );
    if !reached {
        return;
    }
    shell.press("enter", cx);
    let row = "G10 the empty panel's action, gone while the sheet is open";
    if !opened(table, &shell, row, cx) {
        return;
    }
    let second = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    enable_through(&second, &other);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell.run(cx);
    // The list read, answered busy, waits out its retry on the test clock
    // (Cycle 04 Ruling 52): `keys_landed` advances it.
    shell.keys_landed(cx);
    settle(&shell, cx);
    table.fact(
        &format!("{row} (the action went)"),
        !shell.present(empty_action(), cx),
        "the empty panel's action no longer drawn",
    );
    shell.keys("escape", cx);
    let held = all_closed(&shell, cx) && region(&shell, cx) == Some(Region::Repositories);
    table.row(&shell, row, held, "the Repositories region", cx);
}

/// G11: a way in raised with nothing focused, so there is no origin.
fn no_origin(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _) = one_repo(cx, |_, _| ());
    route(&shell, cx, |workspace, window, cx| {
        window.blur(cx);
        workspace.show_add_repository(AddRoute::New, None, window, cx)
    });
    let row = "G11 no origin";
    if !opened(table, &shell, row, cx) {
        return;
    }
    shell.keys("escape", cx);
    let held = all_closed(&shell, cx) && region(&shell, cx) == Some(Region::Content);
    table.row(&shell, row, held, "the content region", cx);
}

/// G13: the confirmation opened from the sheet, closed by Escape and by
/// Cancel on its preview, by Close on a refused preview, and by an add's
/// finish that ends it: focus inside the sheet, on the Kit button that
/// opened it. The button disabled as "Adding…" while its add runs is
/// `while_adding`'s (Ruling 18).
async fn confirmation(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    press_plus(&shell, cx);
    if !opened(table, &shell, "G13", cx) {
        return;
    }
    for (way, id) in [("Escape", None), ("Cancel", Some("confirm-cancel"))] {
        let row = format!("G13 the preview closed by {way}");
        let previewed = preview(&shell, &plain, cx);
        table.fact(
            &format!("{row} (its preview)"),
            previewed,
            "the add's preview",
        );
        if !previewed {
            return;
        }
        match id {
            Some(id) => shell.click(id, cx),
            None => shell.keys("escape", cx),
        }
        let held = !has_dialog(&shell, cx)
            && add_open(&shell, cx)
            && shell.holds_control(ADD_SUBMIT, cx)
            && in_context(&shell, "Sheet", cx);
        table.row(
            &shell,
            &row,
            held,
            "Add repository's own button, in the sheet",
            cx,
        );
    }

    // Close, on a refused preview: Create in a folder that is not empty.
    if shell.present("add-mode-new", cx) {
        shell.click("add-mode-new", cx);
    }
    let full = shell.fixture.empty_directory("full");
    fs::write(full.join("notes.txt"), "kept\n").unwrap();
    let (ada, location) = (identity(), shell.fixture.parent());
    let filled = set(&shell, "create-location", location.to_str().unwrap(), cx)
        && set(&shell, "create-folder-name", "full", cx)
        && set(&shell, "create-name", &ada.name, cx)
        && set(&shell, "create-email", &ada.email, cx);
    focus_control(&shell, "create-submit", cx);
    shell.click("create-submit", cx);
    let refused = filled && has_dialog(&shell, cx) && !shell.present("confirm-ok", cx);
    table.fact(
        "G13 (a refused preview)",
        refused,
        "Create's preview refused, with Close",
    );
    if refused {
        shell.click("confirm-cancel", cx);
        let held = !has_dialog(&shell, cx) && shell.holds_control(CREATE_SUBMIT, cx);
        table.row(
            &shell,
            "G13 a refused preview closed by Close",
            held,
            "Create repository's own button",
            cx,
        );
    }

    // A finish that ends the add: the dialog closes itself. Add is drawn in
    // the existing-folder mode.
    if shell.present("add-mode-existing", cx) {
        shell.click("add-mode-existing", cx);
    }
    shell.backend.answer_job(
        JobKind::RepoEnable,
        answered("repo enable", ResultCode::InternalError),
    );
    let row = "G13 the confirmation closed by a finish that ends the add";
    if !preview(&shell, &plain, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    confirm_and_end(&shell, cx).await;
    let held =
        !has_dialog(&shell, cx) && add_open(&shell, cx) && shell.holds_control(ADD_SUBMIT, cx);
    table.row(
        &shell,
        row,
        held,
        "Add repository's own button, the sheet still open",
        cx,
    );
}

/// G13 while the add runs (Ruling 18): Confirm with the add held, its
/// answer a canned failure, so the add ends without closing the sheet.
/// Escape closes the confirmation while Add repository reads "Adding…" and
/// is disabled: focus is on its own handle, which its wrapper holds, with
/// no ring and no Tab stop. Once the add ends the button is enabled and
/// holds focus itself, with no further move.
async fn while_adding(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    press_plus(&shell, cx);
    let row = "G13 the confirmation closed while its add runs";
    if !opened(table, &shell, row, cx) || !preview(&shell, &plain, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    shell.backend.answer_job(
        JobKind::RepoEnable,
        answered("repo enable", ResultCode::InternalError),
    );
    let held = shell.backend.hold_job(JobKind::RepoEnable);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    let adding = shell.label("add-submit", cx);
    table.fact(
        &format!("{row} (Adding…)"),
        !has_dialog(&shell, cx) && adding.as_deref() == Some(ADDING),
        &format!("the confirmation closed, Add repository reading Adding… (it reads {adding:?})"),
    );
    let on_wrapper = holds(&shell, "add-submit", cx) && !shell.holds_control(ADD_SUBMIT, cx);
    table.row(
        &shell,
        row,
        on_wrapper,
        "Add repository's own handle, held by its wrapper while the button is disabled",
        cx,
    );
    held.release();
    let id = enables(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    let enabled = add_open(&shell, cx) && shell.holds_control(ADD_SUBMIT, cx);
    table.row(
        &shell,
        &format!("{row}, then the add ends"),
        enabled,
        "Add repository's own button, enabled, with no move",
        cx,
    );
}

/// Confirm on the open preview, with the add held on the worker; Escape
/// then closes the confirmation while the add runs, and an Unlock prompt
/// opens. Gives the hold and the prompt's answer.
fn confirmed_then_prompt(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> (
    crate::desktop::test_support::delaying::JobGate,
    futures::channel::oneshot::Receiver<crate::desktop::services::prompts::PromptReply>,
) {
    let held = shell.backend.hold_job(JobKind::RepoEnable);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    let answer = shell.prompt(cx);
    (held, answer)
}

/// G23: the add succeeds while an Unlock prompt is open over the sheet:
/// focus stays in the prompt and the sheet stays open; once the prompt
/// closes, the sheet closes onto its origin (G7).
async fn under_a_prompt(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    let plus = press_plus(&shell, cx);
    let row = "G23 a success while an Unlock prompt is open";
    if !opened(table, &shell, row, cx) || !preview(&shell, &plain, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    let (held, _answer) = confirmed_then_prompt(&shell, cx);
    held.release();
    let id = enables(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    settle(&shell, cx);
    let held_focus = shell.prompt_focused(cx) && add_open(&shell, cx);
    table.row(
        &shell,
        row,
        held_focus,
        "the prompt's input, the sheet still open",
        cx,
    );
    shell.keys("escape", cx);
    let held = all_closed(&shell, cx) && holds_handle(&shell, &plus, cx);
    table.row(
        &shell,
        &format!("{row}, then the prompt closes"),
        held,
        "+",
        cx,
    );
    until_onboarded(&shell, &plain, cx).await;
}

/// G23, opened from Settings: the return to Settings waits too, then lands
/// as G5 does.
async fn under_a_prompt_from_settings(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, _, plain) = plain_shell(cx);
    let settings = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.footer_focus(Control::Settings));
    focus(&shell, &settings, cx);
    shell.press("enter", cx);
    let row = "G23 from Settings, a success while an Unlock prompt is open";
    if !shell.present(ADD_BUTTON.0, cx) {
        table.fact(
            &format!("{row} (Settings' button drawn)"),
            false,
            "Add or create a repository…",
        );
        shell.keys("escape", cx);
        return;
    }
    shell.click(ADD_BUTTON.0, cx);
    if !opened(table, &shell, row, cx) || !preview(&shell, &plain, cx) {
        return;
    }
    let (held, _answer) = confirmed_then_prompt(&shell, cx);
    held.release();
    let id = enables(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    settle(&shell, cx);
    let held_focus = shell.prompt_focused(cx) && add_open(&shell, cx);
    table.row(
        &shell,
        row,
        held_focus,
        "the prompt's input, the sheet still open",
        cx,
    );
    shell.keys("escape", cx);
    let held = settings_on(&shell, SettingsPage::Repositories, cx)
        && chosen(&shell, cx) == Some(plain.clone())
        && holds(&shell, ("settings-page", 0usize), cx);
    table.row(
        &shell,
        &format!("{row}, then the prompt closes"),
        held,
        "Settings › Repositories on the new repository, its entry",
        cx,
    );
    until_onboarded(&shell, &plain, cx).await;
}
