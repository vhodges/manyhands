//! P16, `add_and_create_by_keyboard` (Wave 04 Cycle 05a design P16; A6;
//! B6, B9, B10, B12), in one window, by keystrokes: Tab and Shift+Tab to a
//! control, Enter to press it, and typing into the field that holds focus.
//!
//! **Add with the identity step:** `+` opens the sheet on the folder field
//! (G1); the folder of a repository Git has no identity for is typed; Add
//! repository, pressed before the folder was checked, checks it first and,
//! the identity fields empty, puts focus in the name (B10); the name and
//! the email are typed, Add opens the confirmation and its Confirm adds the
//! repository with that identity; the sheet closes onto `+` (G7). (The test
//! window is never active, so a Kit input emits no Blur here, and Tab out
//! of the folder does not check it as fact 10's blur check would.)
//!
//! **Create:** `+` again, the new mode by its segment, Name, Location and
//! the identity typed; Create repository and Confirm make it; the sheet
//! closes onto `+`.
//!
//! **Each recovery action of B6** (a table): the check that offers it is
//! made with a real folder, or landed through the test seam where a real
//! one cannot give it (`Busy`, no branch checked out, a configuration that
//! needs fixing); Tab reaches the action from the folder field, Enter
//! presses it, and after two draws focus is where the design puts it: Open
//! its settings on Settings' Repositories entry (G21, B9's names), then `+`
//! when Settings closes; Create one here on Name (G20); Choose another with
//! Browse… unavailable on the folder field (B6). Where an action's button
//! goes with the check it starts (Retry, Check again, and Choose another
//! once the chooser answers), focus is on the folder field (Ruling 22): the
//! sheet puts it there, where the Kit sheet's own handle would otherwise
//! take the lost focus.
//!
//! Each add waits for its landed preview, failing fast, and, once it has
//! succeeded, for its own background refresh before the next preview (Task
//! 7's P14 fix); a failed add is a row, not a wait.

use std::path::{Path, PathBuf};

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::results::{Outcome, ResultCode};

use super::{
    ada, checks, finished, focus_in, set,
    support::{
        Table, browse_failed, canned, has_dialog, holds, land, needs_fixing, settle, sheet_open,
    },
    text,
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::{
        ReadResult,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::fixtures::{Fixture, identity},
    view::{
        add_repository::{
            CHECK_AGAIN, CHOOSE_ANOTHER, CHOOSING, CREATE_HERE, IS_REPOSITORY, NOT_SET_UP,
            OPEN_SETTINGS, RETRY,
        },
        confirm::PREPARING,
        harness::{Shell, key},
        settings::Shown,
    },
};

/// How many presses a step may take to reach its control.
const TAB_LIMIT: usize = 40;
/// The Repositories toolbar's `+`.
const PLUS: &str = "add-repository-button";
/// The Kit buttons a step presses, each in its wrapper.
const ADD_SUBMIT: (&str, usize) = ("add-submit", 0);
const CREATE_SUBMIT: (&str, usize) = ("create-submit", 0);
const CONFIRM_OK: (&str, usize) = ("confirm-ok", 0);

/// Whether the element `id` holds focus now.
fn focused(shell: &Shell, id: &ElementId, cx: &mut TestAppContext) -> bool {
    let id = id.clone();
    shell.frame(cx, |window, _| {
        window.try_find(id).and_then(|element| element.focused()) == Some(true)
    })
}

/// Presses `key` until the element `id` holds focus. False, and recorded,
/// when it does not within `TAB_LIMIT` presses.
fn tab_to(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    key: &str,
    id: impl Into<ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    for _ in 0..TAB_LIMIT {
        if focused(shell, &id, cx) {
            return true;
        }
        shell.keys(key, cx);
    }
    let reached = focused(shell, &id, cx);
    table.fact(
        &format!("{row} ({key} reaches {id:?})"),
        reached,
        &format!("focus on {id:?} within {TAB_LIMIT} presses"),
    );
    reached
}

/// Types `text` into the field that holds focus, as the keyboard does.
fn type_text(shell: &Shell, text: &str, cx: &mut TestAppContext) {
    let mut visual = shell.visual(cx);
    visual.simulate_input(text);
    visual.run_until_parked();
}

/// Enter, pressed and released on the control that holds focus.
fn enter(shell: &Shell, cx: &mut TestAppContext) {
    shell.press("enter", cx);
}

/// Where focus is, in words, for a row that does not hold: the deepest
/// focused element's ID, and whether a sheet or a dialog is open.
fn where_focus(shell: &Shell, cx: &mut TestAppContext) -> String {
    use gpui_kit::component::WindowExt;
    shell.frame(cx, |window, cx| {
        let held = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|element| format!("{element:?}").contains("focused: Some(true)"))
            .max_by_key(|element| element.path().len())
            .and_then(|element| element.path().last().cloned());
        format!(
            "held by {held:?}, anything focused {}, sheet {}, dialog {}",
            window.focused(cx).is_some(),
            window.has_active_sheet(cx),
            window.has_active_dialog(cx)
        )
    })
}

/// A focus row, read after two draws: `held` says whether focus is on
/// `wanted`; when it is not, the row says where it is.
fn focus_row(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    held: bool,
    wanted: &str,
    cx: &mut TestAppContext,
) {
    let what = if held {
        wanted.to_owned()
    } else {
        format!("{wanted} (focus is {})", where_focus(shell, cx))
    };
    table.fact(row, held, &what);
}

/// The requests of this session that `pick` picks.
fn sent(
    shell: &Shell,
    pick: fn(&Intent) -> bool,
    cx: &mut TestAppContext,
) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| pick(&request.intent))
}

/// The confirmation shows its landed preview: a prepare answered busy
/// retries on the test clock, which `keys_landed` advances (Task 7's
/// `preview_landed`).
fn preview_landed(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let state = |cx: &mut TestAppContext| shell.label("confirm-state", cx);
    if state(cx).as_deref() == Some(PREPARING) {
        shell.keys_landed(cx);
    }
    shell.present("confirm-ok", cx) && state(cx).as_deref() != Some(PREPARING)
}

/// Tab to the confirmation's Confirm and Enter: the request `pick` picks
/// is sent and ends, its outcome and code given; `None` when none was sent.
async fn confirm(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    pick: fn(&Intent) -> bool,
    cx: &mut TestAppContext,
) -> Option<(Outcome, ResultCode)> {
    let landed = preview_landed(shell, cx);
    table.fact(&format!("{row} (its preview)"), landed, "it landed");
    if !landed || !tab_to(table, shell, row, "tab", CONFIRM_OK, cx) {
        return None;
    }
    let before = sent(shell, pick, cx);
    enter(shell, cx);
    let id = sent(shell, pick, cx)
        .into_iter()
        .find(|id| !before.contains(id))?;
    shell.until_ended(id, cx).await;
    Some(finished(shell, id, cx))
}

/// Waits until the background refresh of `root` an add submits has ended:
/// the next preview then meets no lock it holds.
async fn onboarded(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let wanted = root.to_owned();
    shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                request.lane == Lane::Background
                    && matches!(&request.intent, Intent::IndexRefresh { root } if *root == wanted)
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
}

/// Enter on `+`: the sheet opens on the folder field (G1).
fn open_on_path(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) {
    enter(shell, cx);
    let held = sheet_open(shell, cx) && holds(shell, "add-path", cx);
    let step = format!("{row}: Enter on + opens the sheet on the folder (G1)");
    focus_row(
        table,
        shell,
        &step,
        held,
        "the sheet, focus on add-path",
        cx,
    );
}

/// The sheet closed by its add's success, focus back on `+` (G7).
fn closed_onto_plus(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) {
    settle(shell, cx);
    let held = !sheet_open(shell, cx) && holds(shell, PLUS, cx);
    let row = format!("{row} (the sheet closes onto +, G7)");
    focus_row(table, shell, &row, held, "no sheet, focus on +", cx);
}

/// Add with the identity step, by keyboard, from the Repositories region.
async fn add(table: &mut Table, shell: &Shell, plain: &Path, cx: &mut TestAppContext) -> bool {
    let row = "Add";
    shell.keys("secondary-1", cx);
    if !tab_to(table, shell, row, "tab", PLUS, cx) {
        return false;
    }
    open_on_path(table, shell, row, cx);
    type_text(shell, text(plain), cx);
    // Tab out of the folder does not check it here (no Blur, the window
    // inactive): Add, pressed before the check, checks first (B10).
    if !tab_to(table, shell, row, "tab", ADD_SUBMIT, cx) {
        return false;
    }
    enter(shell, cx);
    shell.keys_landed(cx);
    table.fact(
        "Add: pressed, it checks the folder first and asks the identity",
        checks(shell, cx) == [IS_REPOSITORY, NOT_SET_UP] && shell.present("add-name", cx),
        "the plain rows and the identity fields",
    );
    let held = holds(shell, "add-name", cx) && !has_dialog(shell, cx);
    let step = "Add: pressed with the identity empty, focus in the name (B10)";
    focus_row(table, shell, step, held, "add-name, no dialog", cx);
    type_text(shell, &ada().name, cx);
    shell.keys("tab", cx);
    let held = holds(shell, "add-email", cx);
    let step = "Add: Tab from the name reaches the email";
    focus_row(table, shell, step, held, "add-email", cx);
    type_text(shell, &ada().email, cx);
    if !tab_to(table, shell, row, "tab", ADD_SUBMIT, cx) {
        return false;
    }
    enter(shell, cx);
    let enables = |intent: &Intent| matches!(intent, Intent::RepoEnable { .. });
    let ended = confirm(table, shell, row, enables, cx).await;
    let added = ended == Some((Outcome::Success, ResultCode::Ok));
    table.fact(
        "Add: Confirm adds the repository",
        added,
        &format!("a successful add (it is {ended:?})"),
    );
    let local = git2::Repository::open(plain)
        .and_then(|repository| repository.config()?.open_level(git2::ConfigLevel::Local))
        .and_then(|config| config.get_string("user.name"));
    table.fact(
        "Add: the identity typed is the repository's own",
        local.ok() == Some(ada().name),
        "user.name in the repository's configuration",
    );
    // A failed add is its row above: no refresh follows it to wait for.
    if !added {
        return false;
    }
    onboarded(shell, plain, cx).await;
    closed_onto_plus(table, shell, row, cx);
    true
}

/// Create, by keyboard, from `+`.
async fn create(table: &mut Table, shell: &Shell, made: &Path, cx: &mut TestAppContext) {
    let row = "Create";
    open_on_path(table, shell, row, cx);
    if !tab_to(table, shell, row, "shift-tab", "add-mode-new", cx) {
        return;
    }
    enter(shell, cx);
    table.fact(
        "Create: Enter on its segment shows the new mode",
        shell.selected("add-mode-new", cx),
        "the new mode chosen",
    );
    let (location, name, who) = (made.parent().unwrap(), made.file_name().unwrap(), ada());
    for (id, typed) in [
        ("create-folder-name", name.to_str().unwrap()),
        ("create-location", text(location)),
        ("create-name", who.name.as_str()),
        ("create-email", who.email.as_str()),
    ] {
        if !tab_to(table, shell, row, "tab", id, cx) {
            return;
        }
        type_text(shell, typed, cx);
    }
    if !tab_to(table, shell, row, "tab", CREATE_SUBMIT, cx) {
        return;
    }
    enter(shell, cx);
    let creates = |intent: &Intent| matches!(intent, Intent::RepoCreate { .. });
    let ended = confirm(table, shell, row, creates, cx).await;
    let created = ended == Some((Outcome::Success, ResultCode::Ok));
    table.fact(
        "Create: Confirm makes the repository",
        created && made.join(manyhands::canonical::CONFIG_PATH).is_file(),
        &format!("a successful create (it is {ended:?})"),
    );
    // A failed create is its row above: no refresh follows it to wait for.
    if created {
        onboarded(shell, made, cx).await;
        closed_onto_plus(table, shell, row, cx);
    }
}

/// The folder `root` typed in the field and checked by Enter there; focus
/// stays in the field.
fn check(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    set(shell, "add-path", &key(root), cx);
    focus_in(shell, "add-path", cx);
    enter(shell, cx);
    shell.keys_landed(cx);
}

/// Tab from the folder field to the action `id` offers, labelled `label`,
/// and Enter. False, and recorded, when it is not reached.
fn press_action(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    id: &'static str,
    label: &str,
    cx: &mut TestAppContext,
) -> bool {
    focus_in(shell, "add-path", cx);
    let drawn = shell.label(id, cx);
    table.fact(
        &format!("{row} (offered)"),
        drawn.as_deref() == Some(label),
        &format!("{id} reading {label:?} (it is {drawn:?})"),
    );
    if drawn.is_none() || !tab_to(table, shell, row, "tab", (id, 0usize), cx) {
        return false;
    }
    enter(shell, cx);
    shell.keys_landed(cx);
    true
}

/// Each recovery action of B6 by keyboard, and where focus lands.
fn recovery(table: &mut Table, shell: &Shell, folders: &Folders, cx: &mut TestAppContext) {
    open_on_path(table, shell, "Recovery", cx);
    // Open its settings, on the folder Add has just added (G21).
    check(shell, &folders.plain, cx);
    let row = "Open its settings";
    if press_action(table, shell, row, "add-open-settings", OPEN_SETTINGS, cx) {
        let entry = holds(shell, ("settings-page", 0usize), cx);
        let (shown, chosen) = shell.workspace.read_with(cx, |workspace, cx| {
            (workspace.settings_shown(cx), workspace.settings_chosen(cx))
        });
        let held = entry
            && shown == Shown::Settings(SettingsPage::Repositories)
            && chosen.as_deref() == Some(folders.plain.as_path());
        let step = "Open its settings: Settings › Repositories on it, its entry focused (G21)";
        let wanted = format!("the entry of {shown:?} on {chosen:?}");
        focus_row(table, shell, step, held, &wanted, cx);
        shell.keys("escape", cx);
        let held = !sheet_open(shell, cx) && holds(shell, PLUS, cx);
        let step = "Open its settings: Settings closes onto + (G21)";
        focus_row(table, shell, step, held, "+, the sheet's origin", cx);
        enter(shell, cx);
    }

    // Create one here, on a folder that is not a repository (G20).
    check(shell, &folders.loose, cx);
    let row = "Create one here";
    if press_action(table, shell, row, "add-create-here", CREATE_HERE, cx) {
        let held = shell.selected("add-mode-new", cx) && holds(shell, "create-folder-name", cx);
        let step = "Create one here: the new mode, focus on Name (G20)";
        focus_row(table, shell, step, held, "create-folder-name", cx);
        if tab_to(table, shell, "Back", "shift-tab", "add-mode-existing", cx) {
            enter(shell, cx);
        }
    }

    // Retry, on a check answered Busy (through the test seam).
    land(shell, &folders.other, ReadResult::Busy, cx);
    if press_action(table, shell, "Retry", "add-retry", RETRY, cx) {
        lost(table, shell, "Retry", cx);
    }

    // Check again, on a check that found no branch checked out, and on a
    // configuration that needs fixing (canned; owner Ruling 25).
    let mut detached = canned(&folders.other);
    detached.head_branch = None;
    let fixing = needs_fixing(&folders.other);
    for (row, check) in [
        ("Check again", detached),
        ("Check again, needs fixing", fixing),
    ] {
        land(shell, &folders.other, ReadResult::Ok(check), cx);
        if press_action(table, shell, row, "add-retry", CHECK_AGAIN, cx) {
            lost(table, shell, row, cx);
        }
    }

    // Choose another, Browse… available: the chooser opens, and its answer
    // is checked.
    check(shell, &folders.missing, cx);
    let row = "Choose another, Browse… available";
    if press_action(table, shell, row, "add-choose", CHOOSE_ANOTHER, cx) {
        let held = cx.did_prompt_for_paths()
            && shell.label("add-browse-note", cx).as_deref() == Some(CHOOSING)
            && holds(shell, ("add-choose", 0usize), cx);
        let step = format!("{row}: the chooser open, focus kept on the action");
        focus_row(table, shell, &step, held, "the chooser, add-choose", cx);
        if cx.did_prompt_for_paths() {
            let chosen = folders.other.clone();
            cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
            shell.run(cx);
            shell.keys_landed(cx);
            lost(table, shell, row, cx);
        }
    }

    // Choose another, Browse… unavailable: focus on the folder (B6).
    browse_failed(shell, cx);
    check(shell, &folders.missing, cx);
    let row = "Choose another, Browse… unavailable";
    if press_action(table, shell, row, "add-choose", CHOOSE_ANOTHER, cx) {
        let held = !cx.did_prompt_for_paths() && holds(shell, "add-path", cx);
        let step = format!("{row}: focus on the folder, no chooser (B6)");
        focus_row(table, shell, &step, held, "no chooser, add-path", cx);
    }
    shell.keys("escape", cx);
}

/// An action whose button went with the check it started: the check ran
/// again, and focus is on the folder field (Ruling 22).
fn lost(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) {
    table.fact(
        &format!("{row}: the check ran again"),
        checks(shell, cx) == [IS_REPOSITORY, NOT_SET_UP],
        "the folder's own rows, no action",
    );
    let held = holds(shell, "add-path", cx);
    let step = format!("{row}: focus on the folder field");
    focus_row(table, shell, &step, held, "add-path", cx);
}

/// The folders the journeys use: a repository Git has no identity for
/// (Add's), one Git has an identity for (never added), a folder that is not
/// a repository (a file in it), and a folder that does not exist.
struct Folders {
    plain: PathBuf,
    other: PathBuf,
    loose: PathBuf,
    missing: PathBuf,
}

/// P16 (A6; B6, B10, B12): Add with the identity step and Create by Tab,
/// Enter and typing, and each recovery action of B6 by keyboard, with
/// where focus lands, read after two draws.
#[gpui_kit::test]
async fn add_and_create_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P16");
    let fixture = Fixture::new();
    let folders = Folders {
        plain: fixture.plain_repository("plain", None),
        other: fixture.plain_repository("other", Some(identity())),
        loose: fixture.empty_directory("loose"),
        missing: fixture.parent().join("missing"),
    };
    std::fs::write(folders.loose.join("notes.txt"), "kept\n").unwrap();
    let made = fixture.empty_directory("made");
    let shell = Shell::open(fixture, cx);

    if add(&mut table, &shell, &folders.plain, cx).await {
        create(&mut table, &shell, &made, cx).await;
        recovery(&mut table, &shell, &folders, cx);
    }
    table.done();
}
