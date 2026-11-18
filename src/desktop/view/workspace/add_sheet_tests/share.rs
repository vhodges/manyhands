//! P14, `a_chosen_remote_is_selected_after_the_add_succeeds` (Wave 04
//! Cycle 05a design C2; B10; decisions 3, 12, 17 and 20; R-26, R-29,
//! R-30; K-10, K-14).
//!
//! The share an add was confirmed with is sent once the add has added the
//! repository, as today's `RemoteSelect` on the user lane, for the root the
//! library registered: what the worker received is read from the request
//! model, and the selection jobs it ran. A remote chosen other than the
//! configuration's is selected; Keep over a configured remote sharing can
//! use is `name: None`; an unchanged choice, a configured remote sharing
//! cannot use or the folder lacks, and an add that ends send nothing.
//! Prepare again then a success, and a Partial then Retry, send one each;
//! so does an add whose discovery is still pending. The share is sent
//! when the sheet closed while the add ran, and not once the app is
//! finishing. A folder already set up, whose Git has no identity, holds
//! the select on Keep, and nothing is sent. A failed selection leaves the
//! repository registered, with its own Activity row.
//!
//! Real repositories with an SSH remote where they give the case; canned
//! checks where they cannot (a configured remote, no identity; design
//! Testing), landed over a real plain repository the add then enables.
//! This file holds the choices and the helpers; `share_attempts` the rows
//! over an add's later attempts, a failed selection and quitting.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, component::select::SelectEvent};
use manyhands::{
    repository::{
        ConfigurationDto, ConfigurationState, IdentityAvailability, RemoteDto,
        RepositoryInspectionDto,
    },
    results::ResultCode,
};

use super::support::*;
use crate::desktop::{
    services::{
        JobKind, ReadResult,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::fixtures::identity,
    view::{
        activity::tests::cases::answered,
        add_repository::SHARE_NEEDS_IDENTITY,
        harness::{Shell, key},
    },
};

const SSH: &str = "ssh://git@example.invalid/team/repo.git";
const HTTPS: &str = "https://example.invalid/team/repo.git";

/// P14's choices; its later attempts and ends are `share_attempts`'.
pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    choices(table, cx).await;
}

/// A plain repository `name` Git has an identity for, not set up, with the
/// SSH remote `origin`.
fn with_origin(shell: &Shell, name: &str) -> PathBuf {
    let root = shell.fixture.plain_repository(name, Some(identity()));
    shell.fixture.add_remote(&root, "origin", SSH);
    root
}

/// A window over repository `a`, with a plain repository `plain` that has
/// the SSH remote `origin`.
pub(super) fn origin_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let (shell, a) = one_repo(cx, |_, _| ());
    shell.select(&a, cx);
    let plain = with_origin(&shell, "plain");
    (shell, plain)
}

fn remote(name: &str, location: &str, eligible: bool) -> RemoteDto {
    RemoteDto {
        name: name.to_owned(),
        fetch_location: location.to_owned(),
        push_location: location.to_owned(),
        publication_eligible: eligible,
        selected_for_publication: false,
    }
}

/// A canned check of `root`: `main` checked out, not registered, Git
/// having an identity, the SSH remote `origin` and the HTTPS remote
/// `backup`; set up on `main` sharing through `configured` when given.
fn canned(root: &Path, configured: Option<&str>) -> RepositoryInspectionDto {
    let set_up = configured.is_some();
    RepositoryInspectionDto {
        selected_path: key(root),
        root: key(root),
        registered: false,
        head_branch: Some("main".to_owned()),
        local_branches: vec!["main".to_owned()],
        configuration: ConfigurationDto {
            state: if set_up {
                ConfigurationState::Valid
            } else {
                ConfigurationState::Missing
            },
            primary_branch: set_up.then(|| "main".to_owned()),
            publication_remote: configured.map(str::to_owned),
            problems: Vec::new(),
        },
        identity_state: IdentityAvailability::Available,
        remotes: vec![remote("origin", SSH, true), remote("backup", HTTPS, false)],
        operations: Vec::new(),
    }
}

/// `root`'s text typed in the folder field, and `check` landed for it as
/// the read lands one. False when the sheet is not made.
fn land(
    shell: &Shell,
    root: &Path,
    check: RepositoryInspectionDto,
    cx: &mut TestAppContext,
) -> bool {
    if !set(shell, "add-path", &key(root), cx) {
        return false;
    }
    let Some(sheet) = sheet(shell, cx) else {
        return false;
    };
    sheet.update(cx, |sheet, cx| {
        sheet.land_check(key(root), ReadResult::Ok(check), cx)
    });
    shell.run(cx);
    true
}

/// The share select confirmed on `to` (a remote's name, or `None` for Keep
/// on this computer), as the Kit's list confirms an item.
pub(super) fn choose(shell: &Shell, to: Option<&str>, cx: &mut TestAppContext) {
    let Some(select) = of_sheet(shell, cx, |sheet| sheet.share_select()) else {
        return;
    };
    let to = to.map(str::to_owned);
    shell.visual(cx).update(|window, cx| {
        select.update(cx, |select, cx| {
            select.set_selected_value(&to, window, cx);
            cx.emit(SelectEvent::Confirm(Some(to.clone())));
        })
    });
    shell.run(cx);
}

/// The share the select has chosen.
fn chosen_share(shell: &Shell, cx: &mut TestAppContext) -> Option<Option<String>> {
    shell.frame(cx, |_, _| ());
    let sheet = sheet(shell, cx)?;
    sheet.read_with(cx, |sheet, cx| sheet.chosen_share(cx))
}

/// Add repository pressed: its confirmation open on its preview, landed.
pub(super) fn press_add(shell: &Shell, cx: &mut TestAppContext) -> bool {
    focus_control(shell, "add-submit", cx);
    shell.click("add-submit", cx);
    preview_landed(shell, cx)
}

/// The selections the worker was sent for `root`, newest first, each by
/// its name (`None`: Keep on this computer) and its lane.
fn sent(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> Vec<(Option<String>, Lane)> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter_map(|request| match &request.intent {
                Intent::RemoteSelect { root: to, name } if to == root => {
                    Some((name.clone(), request.lane))
                }
                _ => None,
            })
            .collect()
    })
}

/// The selection requests of this session, newest first.
pub(super) fn selections(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::RemoteSelect { .. })
    })
}

/// How many selection jobs the worker ran.
fn selections_run(shell: &Shell) -> usize {
    shell
        .backend
        .jobs_seen()
        .into_iter()
        .filter(|kind| *kind == JobKind::RemoteSelect)
        .count()
}

/// The row `row`: the worker was sent `wanted` for `root`, each on the
/// user lane, as Settings sends it.
pub(super) fn expect(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    root: &Path,
    wanted: &[Option<&str>],
    cx: &mut TestAppContext,
) {
    let found = sent(shell, root, cx);
    let wanted: Vec<(Option<String>, Lane)> = wanted
        .iter()
        .map(|to| (to.map(str::to_owned), Lane::User))
        .collect();
    table.fact(
        row,
        found == wanted,
        &format!("the selections {wanted:?} sent (they are {found:?})"),
    );
}

/// Every selection sent has ended, then `roots`' background refreshes: the
/// worker must not outlive the test, and the next preview in the window is
/// not prepared while one of them holds the repository lease, which would
/// answer it busy.
pub(super) async fn ended(shell: &Shell, roots: &[&Path], cx: &mut TestAppContext) {
    for id in selections(shell, cx) {
        shell.until_ended(id, cx).await;
    }
    for root in roots {
        until_onboarded(shell, root, cx).await;
    }
}

/// One choice over a canned check: why, the folder's name, the remote its
/// configuration shares through (none: not set up), the choice made (none:
/// the one the check chose), and what the worker is to be sent.
struct Case {
    why: &'static str,
    name: &'static str,
    configured: Option<&'static str>,
    to: Option<Option<&'static str>>,
    wanted: &'static [Option<&'static str>],
}

/// Confirm on the open preview of `root`'s add, the add until it has ended
/// and been handled, then its selection and its background refresh until
/// they have ended, before the next row prepares another preview. A press
/// that sent no add is the row's failure.
async fn add(table: &mut Table, shell: &Shell, row: &str, root: &Path, cx: &mut TestAppContext) {
    let sent = confirm_and_end(shell, cx).await;
    table.fact(&format!("{row} (its add)"), sent.is_some(), "the add sent");
    if sent.is_some() {
        settle(shell, cx);
        ended(shell, &[root], cx).await;
    }
}

/// The table of choices, each added in one window: a chosen remote over a
/// root typed another way (R-29), Keep over a configured remote, unchanged
/// choices, a configured remote the folder lacks, one sharing cannot use, a
/// folder set up with no identity (R-30), an add that ends, and the sheet
/// closed while the add runs (decision 12).
async fn choices(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, a) = one_repo(cx, |_, _| ());
    shell.select(&a, cx);

    // R-29: the root the library registered, not the one typed.
    let registered = with_origin(&shell, "registered");
    let typed = shell
        .fixture
        .empty_directory("sub")
        .join("..")
        .join("registered");
    press_plus(&shell, cx);
    let row = "P14 a chosen remote, typed by another path";
    if opened(table, &shell, row, cx) && checked_folder(&shell, &typed, cx) {
        choose(&shell, Some("origin"), cx);
        if press_add(&shell, cx) {
            add(table, &shell, row, &registered, cx).await;
        }
    }
    expect(table, &shell, row, &registered, &[Some("origin")], cx);
    expect(
        table,
        &shell,
        &format!("{row} (not typed)"),
        &typed,
        &[],
        cx,
    );

    // Each choice over a canned check, then the add.
    let cases = [
        Case {
            why: "Keep over a configured SSH remote",
            name: "keep",
            configured: Some("origin"),
            to: Some(None),
            wanted: &[None],
        },
        Case {
            why: "the configured remote unchanged",
            name: "kept",
            configured: Some("origin"),
            to: None,
            wanted: &[],
        },
        Case {
            why: "Keep unchanged, nothing configured",
            name: "local",
            configured: None,
            to: None,
            wanted: &[],
        },
        Case {
            why: "a configured remote the folder lacks",
            name: "lacks",
            configured: Some("gone"),
            to: None,
            wanted: &[],
        },
    ];
    for Case {
        why,
        name,
        configured,
        to,
        wanted,
    } in cases
    {
        let root = with_origin(&shell, name);
        press_plus(&shell, cx);
        let row = format!("P14 {why}");
        if opened(table, &shell, &row, cx) && land(&shell, &root, canned(&root, configured), cx) {
            if let Some(to) = to {
                choose(&shell, to, cx);
            }
            if press_add(&shell, cx) {
                add(table, &shell, &row, &root, cx).await;
            }
        }
        expect(table, &shell, &row, &root, wanted, cx);
    }

    ineligible(table, &shell, cx);
    held(table, &shell, cx).await;
    ends(table, &shell, cx).await;
    closed_meanwhile(table, &shell, cx).await;

    let run = selections_run(&shell);
    table.fact(
        "P14 the choices: the worker ran each selection sent, once",
        run == 3,
        &format!("3 selection jobs run (it ran {run})"),
    );
}

/// A configured remote sharing cannot use: Add is disabled (B6), so no add
/// and nothing is sent, and the share it would send is none (C2).
fn ineligible(table: &mut Table, shell: &Shell, cx: &mut TestAppContext) {
    let root = with_origin(shell, "ineligible");
    press_plus(shell, cx);
    let row = "P14 a configured remote that is not SSH";
    if !opened(table, shell, row, cx) || !land(shell, &root, canned(&root, Some("backup")), cx) {
        return;
    }
    let share = of_sheet(shell, cx, |sheet| sheet.share_to_send()).flatten();
    table.fact(
        &format!("{row} (the share)"),
        share.is_none(),
        &format!("no share (it is {share:?})"),
    );
    let adds = enables(shell, cx).len();
    focus_control(shell, "add-submit", cx);
    shell.click("add-submit", cx);
    table.fact(
        &format!("{row} (Add disabled)"),
        !has_dialog(shell, cx) && enables(shell, cx).len() == adds,
        "no confirmation, no add",
    );
    expect(table, shell, row, &root, &[], cx);
    shell.keys("escape", cx);
}

/// R-30 (decision 20): a folder set up to share through `origin`, whose Git
/// has no identity. The select is held on Keep with its reason and refuses
/// a `Confirm` naming `origin`; the add, with an identity typed, sends
/// nothing, since a selection would commit with no identity.
async fn held(table: &mut Table, shell: &Shell, cx: &mut TestAppContext) {
    let root = with_origin(shell, "held");
    let mut check = canned(&root, Some("origin"));
    check.identity_state = IdentityAvailability::Required;
    press_plus(shell, cx);
    let row = "P14 set up with no identity (R-30)";
    if !opened(table, shell, row, cx) || !land(shell, &root, check, cx) {
        return;
    }
    let (is_held, chosen, line) = (
        of_sheet(shell, cx, |sheet| sheet.share_held()) == Some(true),
        chosen_share(shell, cx),
        shell.label("add-share-held", cx),
    );
    table.fact(
        &format!("{row} (held on Keep)"),
        is_held && chosen == Some(None) && line.as_deref() == Some(SHARE_NEEDS_IDENTITY),
        &format!("held on Keep with its line (held {is_held}, {chosen:?}, {line:?})"),
    );
    choose(shell, Some("origin"), cx);
    let chosen = chosen_share(shell, cx);
    table.fact(
        &format!("{row} (a Confirm naming origin)"),
        chosen == Some(None),
        &format!("still Keep (it is {chosen:?})"),
    );
    let ada = identity();
    let typed = set(shell, "add-name", &ada.name, cx) && set(shell, "add-email", &ada.email, cx);
    if !typed || !press_add(shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    add(table, shell, row, &root, cx).await;
    expect(
        table,
        shell,
        &format!("{row} (nothing sent)"),
        &root,
        &[],
        cx,
    );
}

/// An add that ends (a canned failure): the sheet stays, nothing is sent.
async fn ends(table: &mut Table, shell: &Shell, cx: &mut TestAppContext) {
    let root = with_origin(shell, "ends");
    press_plus(shell, cx);
    let row = "P14 an add that ends";
    if !opened(table, shell, row, cx) || !checked_folder(shell, &root, cx) {
        return;
    }
    choose(shell, Some("origin"), cx);
    shell.backend.answer_job(
        JobKind::RepoEnable,
        answered("repo enable", ResultCode::InternalError),
    );
    if !press_add(shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    confirm_and_end(shell, cx).await;
    settle(shell, cx);
    table.fact(
        &format!("{row} (the sheet stays)"),
        add_open(shell, cx),
        "the sheet open",
    );
    expect(table, shell, row, &root, &[], cx);
    shell.keys("escape", cx);
}

/// Decision 12: the sheet closed while the add runs, then the add
/// succeeds: the share is still sent.
async fn closed_meanwhile(table: &mut Table, shell: &Shell, cx: &mut TestAppContext) {
    let root = with_origin(shell, "closed");
    press_plus(shell, cx);
    let row = "P14 the sheet closed while the add runs";
    if !opened(table, shell, row, cx) || !checked_folder(shell, &root, cx) {
        return;
    }
    choose(shell, Some("origin"), cx);
    if !press_add(shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    let held = shell.backend.hold_job(JobKind::RepoEnable);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    shell.keys("escape", cx);
    table.fact(
        &format!("{row} (closed first)"),
        all_closed(shell, cx),
        "the sheet closed",
    );
    held.release();
    let id = enables(shell, cx)[0];
    shell.until_ended(id, cx).await;
    settle(shell, cx);
    expect(table, shell, row, &root, &[Some("origin")], cx);
    ended(shell, &[&root], cx).await;
}
