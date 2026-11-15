//! P12, `create_joins_location_and_name` (Wave 04 Cycle 05a design B8,
//! B10; R-13; R-27), and the two pins Task 5's review left for Create: a
//! mode switch forgets a pending Add, and no stale message stays after
//! Browse… fills Location.
//!
//! B8: Create's Name and Location are two fields; `create-target` shows
//! the joined path as typed, when both are filled, and the sheet does not
//! check it; the request's root is that path. Create repository is
//! enabled when Name, Location and Main branch are filled (B10), and reads
//! "Adding…" and is disabled while its add is pending (R-27).

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;

use super::{
    details, enter_in, new_mode, set, settings, sheet,
    support::{Table, has_dialog},
    text, value,
};
use crate::desktop::{
    services::{
        JobKind, ReadKind,
        requests::{Intent, RequestState},
    },
    test_support::fixtures::{Fixture, identity},
    view::{
        add_repository::{ADDING, will_create},
        harness::Shell,
    },
};

/// Create's Location and Name for a folder `root`: its parent and its last
/// part (B8).
pub(super) fn set_target(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    set(shell, "create-location", text(root.parent().unwrap()), cx);
    set(
        shell,
        "create-folder-name",
        root.file_name().unwrap().to_str().unwrap(),
        cx,
    );
}

/// The target line `create-target` draws, if it is drawn.
fn target(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("create-target", cx)
}

/// The line B8 draws for `location` and `name`, joined as typed (R-13).
fn joined(location: &str, name: &str) -> String {
    will_create(&PathBuf::from(location).join(name).display().to_string())
}

/// The row `row`: `create-target` reads `wanted` (or is not drawn), and
/// Create repository is enabled as `enabled` says.
fn expect(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    wanted: Option<String>,
    enabled: bool,
    cx: &mut TestAppContext,
) {
    let found = target(shell, cx);
    table.fact(
        &format!("{row} (the target)"),
        found == wanted,
        &format!("the target {wanted:?} (it is {found:?})"),
    );
    shell.frame(cx, |_, _| ());
    let is = sheet(shell, cx).read_with(cx, |sheet, cx| sheet.create_enabled(cx));
    table.fact(
        &format!("{row} (enabled {enabled})"),
        is == enabled,
        &format!("Create repository enabled {enabled} (it is {is})"),
    );
}

/// Pressed while disabled, Create repository changes nothing: no dialog,
/// no request.
fn pressed_does_nothing(table: &mut Table, shell: &Shell, row: &str, cx: &mut TestAppContext) {
    shell.click("create-submit", cx);
    table.fact(
        &format!("{row} (pressed)"),
        !has_dialog(shell, cx) && shell.requests(cx, |_| true).is_empty(),
        "no confirmation and no request",
    );
}

/// The root of the one `RepoCreate` request, once it was sent.
fn sent_root(shell: &Shell, cx: &mut TestAppContext) -> Option<PathBuf> {
    shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .find_map(|request| match &request.intent {
                Intent::RepoCreate { root, .. } => Some(root.clone()),
                _ => None,
            })
    })
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let parent = fixture.parent();
    let location = parent.to_str().unwrap().to_owned();
    let shell = settings(fixture, cx);
    new_mode(&shell, cx);

    let row = "Nothing typed";
    expect(table, &shell, row, None, false, cx);
    pressed_does_nothing(table, &shell, row, cx);

    set(&shell, "create-folder-name", "fresh", cx);
    let row = "Name only";
    expect(table, &shell, row, None, false, cx);
    pressed_does_nothing(table, &shell, row, cx);

    set(&shell, "create-folder-name", "", cx);
    set(&shell, "create-location", &location, cx);
    let row = "Location only";
    expect(table, &shell, row, None, false, cx);
    pressed_does_nothing(table, &shell, row, cx);

    set(&shell, "create-folder-name", "   ", cx);
    expect(table, &shell, "A name of spaces", None, false, cx);

    set(&shell, "create-folder-name", "fresh", cx);
    let row = "Name and Location";
    expect(
        table,
        &shell,
        row,
        Some(joined(&location, "fresh")),
        true,
        cx,
    );

    // The sheet does not check the path (R-13): a Location that does not
    // exist is shown, joined as typed, and Create is still enabled.
    set(&shell, "create-location", "not/made/yet", cx);
    // Joined as `Path::join` joins it, also where the separator is `\`.
    let wanted = Some(joined("not/made/yet", "fresh"));
    expect(table, &shell, "A Location not made yet", wanted, true, cx);
    set(&shell, "create-location", &location, cx);

    // Main branch is required (B10).
    for (row, branch) in [("Main branch empty", ""), ("Main branch of spaces", "  ")] {
        set(&shell, "create-branch", branch, cx);
        expect(
            table,
            &shell,
            row,
            Some(joined(&location, "fresh")),
            false,
            cx,
        );
        pressed_does_nothing(table, &shell, row, cx);
    }
    set(&shell, "create-branch", "main", cx);
    expect(
        table,
        &shell,
        "Main branch back",
        Some(joined(&location, "fresh")),
        true,
        cx,
    );

    // Pressed, the preview names the joined path as its root (the library
    // reads it from the intent), and Escape makes nothing. Git has no
    // identity here, so the preview needs the optional one.
    set(&shell, "create-name", &identity().name, cx);
    set(&shell, "create-email", &identity().email, cx);
    shell.click("create-submit", cx);
    let lines = details(&shell, cx);
    let root = text(&parent.join("fresh")).to_owned();
    table.fact(
        "Pressed (the preview's root)",
        lines.contains(&format!("Root: {root}")),
        &format!("Root: {root} in {lines:?}"),
    );
    shell.keys("escape", cx);
    table.fact(
        "Escape on the preview",
        !has_dialog(&shell, cx) && shell.requests(cx, |_| true).is_empty(),
        "no confirmation, no request",
    );

    // Confirmed, the request's root is Location joined with Name, and
    // while it runs Create repository reads "Adding…" and is disabled
    // (R-27).
    shell.click("create-submit", cx);
    let held = shell.backend.hold_job(JobKind::RepoCreate);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    let label = shell.label("create-submit", cx);
    table.fact(
        "An add pending (Adding…)",
        label.as_deref() == Some(ADDING),
        &format!("Create repository reading {ADDING} (it reads {label:?})"),
    );
    shell.frame(cx, |_, _| ());
    let enabled = sheet(&shell, cx).read_with(cx, |sheet, cx| sheet.create_enabled(cx));
    table.fact("An add pending (disabled)", !enabled, "Create disabled");
    held.release();
    let sent = sent_root(&shell, cx);
    table.fact(
        "Confirmed (the root sent)",
        sent.as_deref() == Some(parent.join("fresh").as_path()),
        &format!("the root {:?} (it is {sent:?})", parent.join("fresh")),
    );
    if let Some(id) = shell
        .requests(cx, |request| {
            matches!(request.intent, Intent::RepoCreate { .. })
        })
        .first()
        .copied()
    {
        shell.until_ended(id, cx).await;
        let ended = shell.operations(cx).read_with(cx, |operations, _| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Finished(..))
            )
        });
        table.fact("The create ended", ended, "the request finished");
    }
}

/// P12 (B8; B10; R-13; R-27): Create's target line, its enablement, the
/// root it sends. The rows are above.
#[gpui_kit::test]
async fn create_joins_location_and_name(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut table = Table::new("P12");
    rows(&mut table, cx).await;
    table.done();
}

/// Task 5's review, pointer (a): switching the mode forgets an Add waiting
/// for its folder's check. The landed check opens no confirmation, and
/// coming back to the existing folder does not revive the wait.
#[gpui_kit::test]
fn a_mode_switch_forgets_a_pending_add(cx: &mut TestAppContext) {
    let mut table = Table::new("Task 5 pointer (a)");
    let fixture = Fixture::new();
    let plain = fixture.plain_repository("plain", Some(identity()));
    let shell = settings(fixture, cx);
    let folder = plain.to_str().unwrap().to_owned();

    for (row, back) in [
        ("Switched to Create, the check lands", false),
        ("Switched there and back, the check lands", true),
    ] {
        set(&shell, "add-path", &folder, cx);
        let held = shell.backend.hold_read(ReadKind::Inspect);
        enter_in(&shell, "add-path", cx);
        shell.click("add-submit", cx);
        let waits = sheet(&shell, cx).read_with(cx, |sheet, _| sheet.submit_pending());
        table.fact(&format!("{row} (Add waits)"), waits, "Add waiting");
        new_mode(&shell, cx);
        if back {
            shell.click("add-mode-existing", cx);
        }
        drop(held);
        shell.run(cx);
        shell.keys_landed(cx);
        let waits = sheet(&shell, cx).read_with(cx, |sheet, _| sheet.submit_pending());
        table.fact(
            &format!("{row} (nothing opened)"),
            !has_dialog(&shell, cx) && !waits && shell.requests(cx, |_| true).is_empty(),
            "no confirmation, no wait, no request",
        );
        // The next round starts from a clean field.
        if has_dialog(&shell, cx) {
            shell.keys("escape", cx);
        }
        shell.click("add-mode-existing", cx);
        set(&shell, "add-path", "", cx);
    }
    table.done();
}

/// Task 5's review, pointer (b): Create says nothing of what is missing,
/// since its button is disabled instead (B10), so no message is left over
/// after Browse… fills Location, or at any other time.
#[gpui_kit::test]
fn create_leaves_no_message_behind(cx: &mut TestAppContext) {
    let mut table = Table::new("Task 5 pointer (b)");
    let fixture = Fixture::new();
    let chosen = fixture.parent();
    let shell = settings(fixture, cx);
    new_mode(&shell, cx);
    shell.click("create-submit", cx);
    table.fact(
        "Create pressed with nothing typed",
        !shell.present("create-message", cx) && !has_dialog(&shell, cx),
        "no message and no dialog",
    );

    shell.click("create-browse", cx);
    let answer = chosen.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![answer]));
    shell.run(cx);
    table.fact(
        "Browse… chose Location",
        value(&shell, "create-location", cx) == text(&chosen),
        "Location filled by the choice",
    );
    table.fact(
        "Browse… chose Location (no message stays)",
        !shell.present("create-message", cx) && target(&shell, cx).is_none(),
        "no message, and no target while Name is empty",
    );
    table.done();
}
