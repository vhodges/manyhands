//! P9, `folder_checks_are_rows_with_their_actions` (Wave 04 Cycle 05a
//! design B6 item 2; R-31), and P22, `a_folder_that_is_not_text_is_named`
//! (Part G item 4).
//!
//! B6's table row by row: what each check state draws, as rows read by
//! their sentences, and the one action it offers. Real repositories where
//! they give the case (a check in flight, a plain repository, a registered
//! one, a folder that is not a repository, one that is missing); canned
//! checks where they cannot (a configuration that needs fixing, no branch
//! checked out, another main branch, a remote that is not SSH; design
//! Testing); `Busy` landed through the test accessor. Then what each
//! action does, and a check that lands for text since changed is ignored.
//! Create one here's focus (G20) is Task 6's.

use std::{fs, path::Path};

use gpui_kit::TestAppContext;
use manyhands::results::ResultCode;

use super::{
    enter_in, settings,
    support::{
        Table, action, canned, fixing_guidance, holds, land, needs_fixing, press, remote, rows,
        set_up,
    },
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::{ReadKind, ReadResult},
    test_support::fixtures::{Fixture, identity},
    view::{
        add_repository::{
            ALREADY_ADDED, ALREADY_SET_UP, BUSY, CHECK_AGAIN, CHECKING, CHOOSE_ANOTHER, CHOOSING,
            CREATE_HERE, IS_REPOSITORY, NEEDS_FIXING, NO_BRANCH, NOT_REPOSITORY, NOT_SET_UP,
            OPEN_SETTINGS, RETRY, WILL_BE_ADDED, not_ssh_remote, other_main_branch,
        },
        harness::{Shell, key},
        settings::Shown,
    },
};

/// The row `row` draws `wanted`'s sentences, in order, and offers `offers`
/// (its wrapper and label), or no action.
fn expect(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    wanted: &[String],
    offers: Option<(&str, &str)>,
    cx: &mut TestAppContext,
) {
    let found = rows(shell, cx);
    table.fact(
        &format!("{row} (rows)"),
        found == wanted,
        &format!("the rows {wanted:?} (they are {found:?})"),
    );
    let drawn = action(shell, cx);
    let offered = drawn.as_ref().map(|(id, label)| (*id, label.as_str()));
    table.fact(
        &format!("{row} (action)"),
        offered == offers,
        &format!("the action {offers:?} (it is {drawn:?})"),
    );
}

fn owned(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| (*row).to_owned()).collect()
}

/// `root` typed in the folder field, checked by Enter, and its check landed
/// (a busy answer waits out its retry on the test clock).
fn check(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    super::set(shell, "add-path", &key(root), cx);
    enter_in(shell, "add-path", cx);
    shell.keys_landed(cx);
}

/// R-31: the prefix field is drawn, or not.
fn prefix(table: &mut Table, shell: &Shell, row: &str, drawn: bool, cx: &mut TestAppContext) {
    table.fact(
        &format!("{row} (R-31, the prefix)"),
        shell.present("add-prefix", cx) == drawn,
        &format!("the prefix field drawn {drawn}"),
    );
}

pub(super) fn rows_and_actions(table: &mut Table, cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let plain = fixture.plain_repository("plain", Some(identity()));
    let loose = fixture.empty_directory("loose");
    fs::write(loose.join("notes.txt"), "kept\n").unwrap();
    let missing = fixture.parent().join("missing");
    let not_repository = fixture
        .service()
        .inspect_repository(&loose)
        .expect_err("not a repository")
        .code();
    let failed = fixture
        .service()
        .inspect_repository(&missing)
        .expect_err("missing")
        .code();
    assert_eq!(not_repository, ResultCode::NotRepository);
    assert_ne!(failed, ResultCode::NotRepository);
    let shell = settings(fixture, cx);

    // Checking.
    super::set(&shell, "add-path", &key(&plain), cx);
    let held = shell.backend.hold_read(ReadKind::Inspect);
    enter_in(&shell, "add-path", cx);
    expect(table, &shell, "Checking", &owned(&[CHECKING]), None, cx);
    drop(held);
    shell.run(cx);
    shell.keys_landed(cx);

    // Checked, not registered, then configuration Missing.
    let row = "Checked, not registered, configuration Missing";
    expect(
        table,
        &shell,
        row,
        &owned(&[IS_REPOSITORY, NOT_SET_UP]),
        None,
        cx,
    );
    prefix(table, &shell, row, true, cx);

    // Checked, registered, its configuration Valid (decision 18).
    check(&shell, &a, cx);
    let row = "Checked, registered, configuration Valid";
    let wanted = owned(&[ALREADY_ADDED, ALREADY_SET_UP]);
    let offers = Some(("add-open-settings", OPEN_SETTINGS));
    expect(table, &shell, row, &wanted, offers, cx);
    prefix(table, &shell, row, false, cx);

    // Registered, its configuration's problems shown too (canned).
    let mut check_of_a = needs_fixing(&a);
    check_of_a.registered = true;
    land(&shell, &a, ReadResult::Ok(check_of_a), cx);
    let wanted = vec![
        ALREADY_ADDED.to_owned(),
        NEEDS_FIXING.to_owned(),
        fixing_guidance(),
    ];
    let row = "Checked, registered, configuration Invalid";
    expect(table, &shell, row, &wanted, offers, cx);

    // Not registered, configuration Valid (canned).
    land(&shell, &plain, ReadResult::Ok(set_up(&plain, "main")), cx);
    let row = "Checked, not registered, configuration Valid";
    let wanted = owned(&[IS_REPOSITORY, ALREADY_SET_UP, WILL_BE_ADDED]);
    expect(table, &shell, row, &wanted, None, cx);
    prefix(table, &shell, row, false, cx);

    // Configuration Invalid (canned).
    land(&shell, &plain, ReadResult::Ok(needs_fixing(&plain)), cx);
    let row = "Checked, configuration Invalid";
    let wanted = vec![
        IS_REPOSITORY.to_owned(),
        NEEDS_FIXING.to_owned(),
        fixing_guidance(),
    ];
    expect(table, &shell, row, &wanted, None, cx);
    prefix(table, &shell, row, true, cx);

    // No branch checked out (canned).
    let mut detached = canned(&plain);
    detached.head_branch = None;
    land(&shell, &plain, ReadResult::Ok(detached), cx);
    let again = Some(("add-retry", CHECK_AGAIN));
    let wanted = owned(&[IS_REPOSITORY, NOT_SET_UP, NO_BRANCH]);
    expect(
        table,
        &shell,
        "Checked, no branch checked out",
        &wanted,
        again,
        cx,
    );
    // Check again runs the check again: the real one lands.
    press(table, &shell, "Check again", ("add-retry", 0usize), cx);
    shell.keys_landed(cx);
    let wanted = owned(&[IS_REPOSITORY, NOT_SET_UP]);
    expect(table, &shell, "Check again, pressed", &wanted, None, cx);

    // Configuration Valid on another main branch (canned).
    let mut other = set_up(&plain, "trunk");
    other.local_branches = vec!["main".to_owned(), "trunk".to_owned()];
    land(&shell, &plain, ReadResult::Ok(other), cx);
    let wanted = vec![
        IS_REPOSITORY.to_owned(),
        ALREADY_SET_UP.to_owned(),
        WILL_BE_ADDED.to_owned(),
        other_main_branch("trunk"),
    ];
    let row = "Checked, configuration Valid on another main branch";
    expect(table, &shell, row, &wanted, again, cx);

    // Configuration Valid, sharing through a remote that is not SSH
    // (canned), and through one that is.
    for (eligible, last) in [(false, Some(not_ssh_remote("origin"))), (true, None)] {
        let mut shares = set_up(&plain, "main");
        shares.configuration.publication_remote = Some("origin".to_owned());
        shares.remotes = vec![remote("origin", "https://example.invalid/r.git", eligible)];
        land(&shell, &plain, ReadResult::Ok(shares), cx);
        let mut wanted = owned(&[IS_REPOSITORY, ALREADY_SET_UP, WILL_BE_ADDED]);
        wanted.extend(last);
        let row = format!("Checked, configuration Valid, its remote eligible {eligible}");
        let offers = (!eligible).then_some(("add-retry", CHECK_AGAIN));
        expect(table, &shell, &row, &wanted, offers, cx);
    }

    // Failed(NotRepository), and Create one here (G20 is Task 6's).
    check(&shell, &loose, cx);
    let offers = Some(("add-create-here", CREATE_HERE));
    let row = "Failed(NotRepository)";
    expect(table, &shell, row, &owned(&[NOT_REPOSITORY]), offers, cx);
    press(
        table,
        &shell,
        "Create one here",
        ("add-create-here", 0usize),
        cx,
    );
    table.fact(
        "Create one here, pressed",
        shell.selected("add-mode-new", cx) && !shell.selected("add-mode-existing", cx),
        "the new mode shown",
    );
    press(
        table,
        &shell,
        "Back to the existing folder",
        "add-mode-existing",
        cx,
    );

    // Failed(other): the code's message, the code in the tooltip.
    check(&shell, &missing, cx);
    let offers = Some(("add-choose", CHOOSE_ANOTHER));
    let row = "Failed(other)";
    expect(table, &shell, row, &owned(&[failed.message()]), offers, cx);
    let tips = super::sheet(&shell, cx).read_with(cx, |sheet, _| sheet.check_tips());
    table.fact(
        &format!("{row} (its tooltip)"),
        tips == [Some(failed.as_str())],
        &format!("the tooltip {} (they are {tips:?})", failed.as_str()),
    );
    // Choose another runs Browse… while it is available.
    press(table, &shell, "Choose another", ("add-choose", 0usize), cx);
    table.fact(
        "Choose another, Browse… available",
        cx.did_prompt_for_paths()
            && shell.label("add-browse-note", cx).as_deref() == Some(CHOOSING),
        "the folder chooser open",
    );
    if cx.did_prompt_for_paths() {
        cx.simulate_path_prompt_response(|_| None);
        shell.run(cx);
    }

    // Busy, set through the test accessor, and Retry.
    land(&shell, &plain, ReadResult::Busy, cx);
    let offers = Some(("add-retry", RETRY));
    expect(table, &shell, "Busy", &owned(&[BUSY]), offers, cx);
    press(table, &shell, "Retry", ("add-retry", 0usize), cx);
    shell.keys_landed(cx);
    let wanted = owned(&[IS_REPOSITORY, NOT_SET_UP]);
    expect(table, &shell, "Retry, pressed", &wanted, None, cx);

    // A check that lands for text the person has since changed, by typing
    // in the field Enter left focus in, is ignored (generations).
    super::set(&shell, "add-path", &key(&loose), cx);
    let held = shell.backend.hold_read(ReadKind::Inspect);
    enter_in(&shell, "add-path", cx);
    shell.visual(cx).simulate_input("-typed");
    shell.run(cx);
    drop(held);
    shell.run(cx);
    shell.keys_landed(cx);
    let inspected =
        super::sheet(&shell, cx).read_with(cx, |sheet, _| sheet.inspected().map(str::to_owned));
    table.fact(
        "A check landing for text since changed",
        rows(&shell, cx).is_empty() && inspected.is_none(),
        &format!("no rows and no check (the check is of {inspected:?})"),
    );

    // Choose another focuses the folder when Browse… is unavailable.
    super::support::browse_failed(&shell, cx);
    check(&shell, &missing, cx);
    // Focus starts elsewhere, so only the action can put it on the folder.
    let browse = super::control_focus(&shell, "add-browse", cx);
    shell.frame(cx, |window, cx| window.focus(&browse, cx));
    shell.run(cx);
    press(
        table,
        &shell,
        "Choose another, unavailable",
        ("add-choose", 0usize),
        cx,
    );
    table.fact(
        "Choose another, Browse… unavailable",
        !cx.did_prompt_for_paths() && holds(&shell, "add-path", cx),
        "focus on the folder field, no chooser",
    );

    // Open its settings closes the sheet into Settings › Repositories on
    // the repository (decision 18; G21 is P6's).
    check(&shell, &a, cx);
    press(
        table,
        &shell,
        "Open its settings",
        ("add-open-settings", 0usize),
        cx,
    );
    super::support::settle(&shell, cx);
    let (shown, chosen) = shell.workspace.read_with(cx, |workspace, cx| {
        (workspace.settings_shown(cx), workspace.settings_chosen(cx))
    });
    table.fact(
        "Open its settings, pressed",
        !shell.present("add-sheet", cx)
            && shown == Shown::Settings(SettingsPage::Repositories)
            && chosen.as_deref() == Some(a.as_path()),
        &format!("Settings › Repositories on a (it is {shown:?} on {chosen:?})"),
    );
}

/// P22 (Part G item 4): an `Inspect` route whose root is not text names it
/// with a row and Choose another, the field empty and Add disabled; Choose
/// another runs Browse….
#[cfg(unix)]
pub(super) fn not_text(table: &mut Table, cx: &mut TestAppContext) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    use crate::desktop::view::{add_repository::NOT_TEXT, workspace::AddRoute};

    let fixture = Fixture::new();
    let root = fixture.parent().join(OsStr::from_bytes(b"not-\xff-text"));
    let shell = Shell::open(fixture, cx);
    shell.visual(cx).update(|window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.show_add_repository(AddRoute::Inspect(root), None, window, cx)
        })
    });
    shell.run(cx);
    table.fact(
        "P22 the sheet opened",
        shell.present("add-sheet", cx),
        "the sheet",
    );
    let offers = Some(("add-choose", CHOOSE_ANOTHER));
    expect(table, &shell, "P22", &owned(&[NOT_TEXT]), offers, cx);
    table.fact(
        "P22 (the field)",
        super::value(&shell, "add-path", cx).is_empty(),
        "the folder field empty",
    );
    table.fact(
        "P22 (Add)",
        !super::support::add_enabled(&shell, cx),
        "Add repository disabled",
    );
    press(
        table,
        &shell,
        "P22 Choose another",
        ("add-choose", 0usize),
        cx,
    );
    table.fact(
        "P22 Choose another, pressed",
        cx.did_prompt_for_paths()
            && shell.label("add-browse-note", cx).as_deref() == Some(CHOOSING),
        "the folder chooser open",
    );
    if cx.did_prompt_for_paths() {
        cx.simulate_path_prompt_response(|_| None);
        shell.run(cx);
    }
}
