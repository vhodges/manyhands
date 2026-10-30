//! Quitting and repository removal with an edit session whose unsaved text
//! no draft holds (Part C fix 1, I1; the Cycle's contract 6: a dirty
//! editor is never dropped without the user's choice). Crash recovery off,
//! a begin that failed, and a begin still in flight each leave the text in
//! the editor only:
//! - quitting waits for a begin in flight, then asks, with Cancel, the
//!   default, which keeps the tab, and Quit anyway;
//! - the user's removal of the repository says so in its confirmation,
//!   whose Cancel keeps the tab, and whose Confirm is the user's choice;
//! - a repository that leaves the list without that confirmation keeps
//!   such a tab open, and its own close asks.
//!
//! The helpers are `edit_tests`'s. Each test sets the caret, focuses the
//! editor and draws a frame before typing, and types through the window;
//! each waits on the worker and allows parking.

use super::*;
#[cfg(unix)]
use crate::desktop::test_support::drafts::{FolderMode, superuser};
use crate::desktop::{
    test_support::drafts::{Actor, second_actor},
    view::{
        confirm::{CONFIRM, SUBMITTED},
        quit::{EditsNotKept, QuitPhase},
    },
};

const UNKEPT_ONE: &str = "1 tab has unsaved changes that are not kept as a draft";
const REMOVING_CLOSES_IT: &str =
    "1 tab has unsaved changes that are not kept as a draft. Removing closes it.";

fn edit_text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> QuitPhase {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase())
}

fn tab_keys(shell: &Shell, cx: &mut TestAppContext) -> Vec<TabKey> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys())
}

/// The workspace's quit request, as the Quit action and the window's close
/// request make it: whether the window may close now.
fn request_quit(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let quit = shell.frame(cx, |window, cx| {
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.request_quit(window, cx))
    });
    shell.run(cx);
    quit
}

/// Repository `a` with the tickets `TICKET`, "Unkept", and `OTHER`,
/// "Clean", refreshed, in an open window whose draft store did not open:
/// its folder is a link. The link's target goes with the last value.
#[cfg(unix)]
fn shell_without_a_store(cx: &mut TestAppContext) -> (Shell, PathBuf, tempfile::TempDir) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Unkept", "");
    fixture.commit_ticket(&root, OTHER, "Clean", "");
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root, elsewhere)
}

/// Edit with crash recovery off: Continue, then the editor; "x" is typed,
/// and no draft begins. Gives the host and its text.
#[cfg(unix)]
async fn unkept_edit(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> (Entity<EditorHost>, String) {
    open(shell, ticket(root, TICKET), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    let host = until_editing(shell, cx).await;
    type_at_end(shell, &host, "x", cx);
    assert_eq!(attached(shell, root, TICKET, cx), None, "no draft holds it");
    let typed = edit_text(&host, cx);
    assert!(typed.ends_with('x'));
    (host, typed)
}

/// The `repo remove` requests of this session, newest first.
fn removals(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoRemove { .. })
    })
}

/// Opens the removal confirmation of `root`, as the sidebar's Remove does,
/// and waits for its preview.
async fn ask_to_remove(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.remove_repository(key(root), window, cx)
        })
    });
    let deadline = Instant::now() + WAIT;
    loop {
        shell.run(cx);
        if shell.label("confirm-ok", cx).as_deref() == Some(CONFIRM)
            && shell.present("confirm-expires", cx)
        {
            return;
        }
        assert!(Instant::now() < deadline, "the preview did not show");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// Words for one tab and for several, naming no item and no text.
#[test]
fn the_unkept_words_count_tabs() {
    assert_eq!(EditsNotKept { count: 1 }.message(), UNKEPT_ONE);
    assert_eq!(
        EditsNotKept { count: 3 }.message(),
        "3 tabs have unsaved changes that are not kept as drafts"
    );
}

/// (a) Crash recovery off: quitting asks first, and does not quit. Cancel,
/// the default, keeps the tab and its text, shown; Quit anyway quits.
#[cfg(unix)]
#[gpui_kit::test]
async fn quit_asks_first_when_crash_recovery_is_off(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _elsewhere) = shell_without_a_store(cx);
    let (host, typed) = unkept_edit(&shell, &root, cx).await;
    open(&shell, ticket(&root, OTHER), cx);

    assert!(!request_quit(&shell, cx), "the window stays");
    assert!(shell.present("edits-not-kept-dialog", cx), "it asks first");
    assert_eq!(
        shell.label("edits-not-kept-message", cx).as_deref(),
        Some(UNKEPT_ONE)
    );
    assert!(shell.present("edits-not-kept-quit", cx));
    assert_ne!(phase(&shell, cx), QuitPhase::Quit);

    shell.click("edits-not-kept-cancel", cx);
    assert!(!shell.present("edits-not-kept-dialog", cx));
    assert_eq!(phase(&shell, cx), QuitPhase::Running, "it stays");
    assert_eq!(tab_keys(&shell, cx).len(), 2, "the tab stays");
    assert_eq!(
        shell.workspace.read_with(cx, |workspace, cx| workspace
            .opened(cx)
            .map(|target| TabKey::for_target(&target))),
        Some(TabKey::for_target(&ticket(&root, TICKET))),
        "Cancel shows the tab whose text is not kept"
    );
    assert!(edit_text(&host, cx) == typed, "with its text");

    // Enter is Cancel too.
    assert!(!request_quit(&shell, cx));
    shell.press("enter", cx);
    assert!(!shell.present("edits-not-kept-dialog", cx));
    assert_eq!(phase(&shell, cx), QuitPhase::Running);

    assert!(!request_quit(&shell, cx));
    shell.click("edits-not-kept-quit", cx);
    assert_eq!(phase(&shell, cx), QuitPhase::Quit, "Quit anyway quits");
}

/// (b) A begin that failed (the drafts folder cannot be written): the text
/// is the editor's only, and quitting asks the same way.
#[cfg(unix)]
#[gpui_kit::test]
async fn quit_asks_first_when_the_draft_could_not_begin(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Unbegun", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    type_at_end(&shell, &host, "x", cx);
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "the begin failed"
    );
    assert_eq!(status(&shell, cx), DraftState::Failed);
    let typed = edit_text(&host, cx);

    // The begin has landed: nothing is waited for, and it asks at once.
    let (quit, asked) = shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            let quit = workspace.request_quit(window, cx);
            (quit, workspace.quit_phase())
        })
    });
    assert!(!quit);
    assert_eq!(asked, QuitPhase::Running, "no begin is waited for");
    shell.run(cx);
    assert_eq!(
        shell.label("edits-not-kept-message", cx).as_deref(),
        Some(UNKEPT_ONE)
    );
    assert_ne!(phase(&shell, cx), QuitPhase::Quit);
    shell.click("edits-not-kept-cancel", cx);
    assert_eq!(phase(&shell, cx), QuitPhase::Running);
    assert_eq!(tab_keys(&shell, cx).len(), 1, "the tab stays");
    assert!(edit_text(&host, cx) == typed, "with its text");

    assert!(!request_quit(&shell, cx));
    shell.click("edits-not-kept-quit", cx);
    assert_eq!(phase(&shell, cx), QuitPhase::Quit, "Quit anyway quits");
    drop(read_only);
}

/// A begin still in flight is waited for, under "Saving drafts for
/// recovery": no dialog and no quit meanwhile. Once it fails, quitting
/// asks.
#[cfg(unix)]
#[gpui_kit::test]
async fn quit_waits_for_a_begin_in_flight_then_asks_when_it_failed(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Beginning", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    let begin = shell.backend.hold_read(ReadKind::Drafts);
    type_at_end(&shell, &host, "x", cx);
    let typed = edit_text(&host, cx);

    assert!(!request_quit(&shell, cx));
    assert_eq!(phase(&shell, cx), QuitPhase::SavingDrafts, "it waits");
    assert!(
        !shell.present("edits-not-kept-dialog", cx),
        "nothing asks yet"
    );

    drop(begin);
    shell.run(cx);
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "the begin failed"
    );
    assert!(shell.present("edits-not-kept-dialog", cx), "now it asks");
    assert_eq!(phase(&shell, cx), QuitPhase::Running);
    shell.click("edits-not-kept-cancel", cx);
    assert_eq!(phase(&shell, cx), QuitPhase::Running);
    assert!(edit_text(&host, cx) == typed, "the text stays");
    drop(read_only);
}

/// A begin in flight that lands holds the text in a draft: quitting goes
/// on as D2's does, with nothing asked, and the text is stored.
#[gpui_kit::test]
async fn quit_waits_for_a_begin_in_flight_then_quits_once_it_is_kept(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Beginning", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let begin = shell.backend.hold_read(ReadKind::Drafts);
    type_at_end(&shell, &host, "x", cx);
    let typed = edit_text(&host, cx);

    assert!(!request_quit(&shell, cx));
    assert_eq!(phase(&shell, cx), QuitPhase::SavingDrafts, "it waits");
    drop(begin);
    shell.run(cx);
    assert!(!shell.present("edits-not-kept-dialog", cx), "nothing asks");
    assert_eq!(phase(&shell, cx), QuitPhase::Quit);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    assert!(stored_body(&shell.fixture.draft_store(), id) == Some(typed));
}

/// (c) The user's removal of the repository says that the tab whose text
/// no draft holds closes. Cancel keeps the tab, with its text; Confirm is
/// the user's choice, and the tab closes.
#[cfg(unix)]
#[gpui_kit::test]
async fn removing_the_repository_says_an_unkept_tab_closes(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _elsewhere) = shell_without_a_store(cx);
    let (host, typed) = unkept_edit(&shell, &root, cx).await;

    ask_to_remove(&shell, &root, cx).await;
    assert_eq!(
        shell.label("confirm-unkept-note", cx).as_deref(),
        Some(REMOVING_CLOSES_IT)
    );
    shell.click("confirm-cancel", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(removals(&shell, cx).is_empty(), "nothing is removed");
    assert_eq!(tab_keys(&shell, cx).len(), 1, "Cancel keeps the tab");
    assert!(edit_text(&host, cx) == typed, "with its text");

    ask_to_remove(&shell, &root, cx).await;
    assert_eq!(
        shell.label("confirm-unkept-note", cx).as_deref(),
        Some(REMOVING_CLOSES_IT)
    );
    shell.click("confirm-ok", cx);
    let id = removals(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    shell.run(cx);
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok));
    assert!(tab_keys(&shell, cx).is_empty(), "Confirm closes it");
}

/// A removal of a repository whose tabs hold no such text says nothing of
/// it.
#[gpui_kit::test]
async fn removing_a_repository_with_no_unkept_tab_says_nothing_of_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    assert!(
        attached(&shell, &root, TICKET, cx).is_some(),
        "a draft holds it"
    );

    ask_to_remove(&shell, &root, cx).await;
    assert!(!shell.present("confirm-unkept-note", cx));
    shell.click("confirm-cancel", cx);
}

/// (d) A repository that drops from the list, its registration removed by
/// another process: the tab whose text no draft holds stays, with its
/// text, and the repository's other tabs close as before. The tab's own
/// close then asks. Its Save needs the registration: the library refuses
/// it as `repository_not_registered`, and the text and the tab stay.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_repository_that_drops_from_the_list_keeps_an_unkept_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _elsewhere) = shell_without_a_store(cx);
    let (host, typed) = unkept_edit(&shell, &root, cx).await;
    open(&shell, ticket(&root, OTHER), cx);
    assert_eq!(tab_keys(&shell, cx).len(), 2);

    second_actor(&shell.fixture, Actor::Unregister(&root));
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.load_repositories(cx));
    shell.run(cx);
    assert_eq!(
        tab_keys(&shell, cx),
        [TabKey::for_target(&ticket(&root, TICKET))],
        "the unkept tab stays, and the clean one closes"
    );
    assert!(edit_text(&host, cx) == typed, "with its text");

    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "its close asks");
    shell.click("close-draft-cancel-button", cx);
    assert_eq!(tab_keys(&shell, cx).len(), 1);

    // With no registration, its Save is the library's to refuse: the text
    // stays, and so does the tab.
    shell.click("item-save", cx);
    let save = shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketSave { .. })
    })[0];
    ended(&shell, save, cx).await;
    assert_eq!(
        code(&shell, save, cx),
        Some(ResultCode::RepositoryNotRegistered)
    );
    assert!(edit_text(&host, cx) == typed, "the text stays");
    assert_eq!(tab_keys(&shell, cx).len(), 1, "and the tab");
}

/// A removal the user asked for, whose preview found the registration
/// already gone, was never confirmed: the tab whose text no draft holds
/// stays, as for a repository that drops from the list.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_removal_found_already_applied_keeps_an_unkept_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _elsewhere) = shell_without_a_store(cx);
    let (host, typed) = unkept_edit(&shell, &root, cx).await;

    second_actor(&shell.fixture, Actor::Unregister(&root));
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.remove_repository(key(&root), window, cx)
        })
    });
    let deadline = Instant::now() + WAIT;
    while shell.label("confirm-code", cx).as_deref() != Some(ResultCode::AlreadyApplied.as_str()) {
        assert!(Instant::now() < deadline, "the preview did not answer");
        shell.run(cx);
        cx.executor().timer(Duration::from_millis(10)).await;
    }
    shell.run(cx);
    assert_eq!(
        tab_keys(&shell, cx),
        [TabKey::for_target(&ticket(&root, TICKET))],
        "the unkept tab stays"
    );
    assert!(edit_text(&host, cx) == typed, "with its text");
}

/// Part C fix 2, N1: the tabs the confirmation counted at Confirm are the
/// user's choice. Here it counted none. While the removal waits, its dialog
/// shows "Submitted", and its Cancel closes the dialog; the user then types
/// in the tab, with crash recovery off. When the removal lands, that tab,
/// which became unkept after Confirm, stays open with its text, as for a
/// repository that drops from the list.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_tab_unkept_after_confirm_stays_when_the_removal_lands(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _elsewhere) = shell_without_a_store(cx);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    let host = until_editing(&shell, cx).await;

    ask_to_remove(&shell, &root, cx).await;
    assert!(!shell.present("confirm-unkept-note", cx), "none to count");
    let held = shell.backend.hold_job(JobKind::RepoRemove);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    assert_eq!(shell.label("confirm-state", cx).as_deref(), Some(SUBMITTED));
    shell.click("confirm-cancel", cx);
    assert!(!shell.present("confirm-dialog", cx));
    type_at_end(&shell, &host, "x", cx);
    let typed = edit_text(&host, cx);

    held.release();
    let id = removals(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    shell.run(cx);
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok), "removed");
    assert_eq!(
        tab_keys(&shell, cx),
        [TabKey::for_target(&ticket(&root, TICKET))],
        "the tab unkept after Confirm stays"
    );
    assert!(edit_text(&host, cx) == typed, "with its text");
}
