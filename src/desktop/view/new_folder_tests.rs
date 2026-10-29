//! New folder, over real repositories (design, Part E: New folder; R-24;
//! fact 16; Cycle contract 11; obligations 7 and 9 of the Task 24 notes):
//! the dialog explains that a folder stays on this computer until a
//! document is saved in it, and shows the full path it will make, under
//! the folder selected in the tree or under `docs`; Create sends `folder
//! create`, which makes the directory in the primary worktree with no
//! commit, and the tree reads its folders again, with no refresh of the
//! desktop's own (R-24); a retry after a lost answer reads "Folder
//! exists"; a name that is not a folder's is refused here, and anything
//! else the library refuses is shown with its message and code.
//!
//! Every test waits on the worker, so it allows parking. Typing goes
//! through the window, into the field holding focus. The ruled words are
//! literals: the tests fix them.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use git2::Repository;
use gpui_kit::{Entity, Focusable, TestAppContext, component::input::InputState};
use manyhands::{
    repository::{FailurePoint, RepositoryService},
    results::ResultCode,
};

use super::{NewFolderDialog, OpenedFolder};
use crate::desktop::{
    services::requests::{DesktopRequestId, Intent, RequestState},
    test_support::fixtures::Fixture,
    view::{
        documents::EMPTY_FOLDER,
        harness::{Shell, WAIT},
    },
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN0";

const LOCAL: &str = "A new folder stays on this computer until a document is saved in it.";
const INVALID: &str = "This is not a valid folder name.";
const EXISTS: &str = "Folder exists";

/// One repository, `a`, with a document at the top of `docs`, refreshed,
/// in an open window with `a` selected, its lists read.
fn shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, "docs/top.md", "Top");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    (shell, root)
}

/// Draws frames, with the worker and the reads running, until `done`.
async fn until(
    shell: &Shell,
    cx: &mut TestAppContext,
    what: &str,
    done: impl Fn(&Shell, &mut TestAppContext) -> bool,
) {
    let deadline = Instant::now() + WAIT;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if done(shell, cx) {
            shell.frame(cx, |_, _| ());
            return;
        }
        assert!(Instant::now() < deadline, "{what} did not happen");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

fn dialog(cx: &mut TestAppContext) -> Entity<NewFolderDialog> {
    cx.read(|cx| {
        cx.try_global::<OpenedFolder>()
            .and_then(|opened| opened.0.upgrade())
    })
    .expect("a new folder dialog is open")
}

fn dialog_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("new-folder-dialog", cx)
}

fn name_input(cx: &mut TestAppContext) -> Entity<InputState> {
    dialog(cx).read_with(cx, |dialog, _| dialog.name_input())
}

fn offered(cx: &mut TestAppContext) -> bool {
    dialog(cx).read_with(cx, |dialog, cx| dialog.offered(cx))
}

/// Puts `typed` in the Folder name in place of what it holds, through the
/// window: focus there, select all and delete, type.
fn retype(shell: &Shell, typed: &str, cx: &mut TestAppContext) {
    let input = name_input(cx);
    shell.frame(cx, |window, cx| {
        input.update(cx, |input, cx| input.focus(window, cx))
    });
    shell.frame(cx, |_, _| ());
    shell.keys("secondary-a backspace", cx);
    if !typed.is_empty() {
        shell.visual(cx).simulate_input(typed);
        shell.run(cx);
    }
    shell.frame(cx, |_, _| ());
}

/// Opens New folder from its button.
fn open_dialog(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("new-folder-button", cx);
    assert!(dialog_open(shell, cx), "New folder opens its dialog");
    let handle = name_input(cx).read_with(cx, |input, cx| input.focus_handle(cx));
    assert!(
        shell.frame(cx, move |window, _| handle.is_focused(window)),
        "focus starts in the Folder name"
    );
}

/// This session's folder creates, newest first.
fn creates(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::FolderCreate { .. })
    })
}

/// The finished request `id`'s code.
fn code(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Option<ResultCode> {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.state {
            RequestState::Finished(_, code, ..) => Some(*code),
            _ => None,
        })
}

/// Waits until the newest folder create has an answer, and gives it.
async fn answered(shell: &Shell, cx: &mut TestAppContext) -> (DesktopRequestId, ResultCode) {
    until(shell, cx, "the folder create's answer", |shell, cx| {
        creates(shell, cx)
            .first()
            .is_some_and(|id| code(shell, *id, cx).is_some())
    })
    .await;
    let id = creates(shell, cx)[0];
    (id, code(shell, id, cx).unwrap())
}

fn requested_path(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> String {
    shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(id).expect("listed").intent {
            Intent::FolderCreate { path, .. } => path.clone(),
            _ => panic!("a folder create"),
        }
    })
}

/// The folders the library lists for `root`.
fn folders(shell: &Shell, root: &Path) -> Vec<String> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service
        .list_document_folders(&repo)
        .unwrap()
        .items
        .into_iter()
        .map(|folder| folder.path)
        .collect()
}

fn head(root: &Path) -> git2::Oid {
    Repository::open(root)
        .unwrap()
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
}

fn refreshes(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .requests(cx, |request| {
            matches!(
                request.intent,
                Intent::IndexRefresh { .. } | Intent::IndexRebuild { .. }
            )
        })
        .len()
}

/// The explanation and the full path are shown; Enter creates
/// `docs/plans` in the primary worktree, with no commit; the dialog
/// closes; the tree reads its folders again and shows the empty folder,
/// with no refresh sent. A folder selected in the tree is the next one's
/// parent.
#[gpui_kit::test]
async fn new_folder_creates_and_explains_local_folders_and_lists_it(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    let commit = head(&root);
    open_dialog(&shell, cx);
    assert_eq!(
        shell.label("new-folder-explain", cx).as_deref(),
        Some(LOCAL)
    );
    shell.visual(cx).simulate_input("plans");
    shell.run(cx);
    assert_eq!(
        shell.label("new-folder-path", cx).as_deref(),
        Some("docs/plans")
    );
    assert!(offered(cx));
    assert!(!shell.present("new-folder-name-note", cx));
    shell.keys("enter", cx);
    let (sent, answer) = answered(&shell, cx).await;
    assert_eq!(answer, ResultCode::Ok);
    assert_eq!(requested_path(&shell, sent, cx), "docs/plans");
    until(&shell, cx, "the dialog closing", |shell, cx| {
        !dialog_open(shell, cx)
    })
    .await;
    assert!(
        root.join("docs/plans").is_dir(),
        "made in the primary worktree"
    );
    assert_eq!(head(&root), commit, "no commit");
    assert!(folders(&shell, &root).contains(&"docs/plans".to_owned()));
    until(&shell, cx, "the tree listing it", |shell, cx| {
        shell.rows("doc-node", cx).contains(&"plans".to_owned())
    })
    .await;
    let nodes = shell.rows("doc-node", cx);
    let at = nodes.iter().position(|node| node == "plans").unwrap();
    assert_eq!(
        shell.label(("doc-empty", at), cx).as_deref(),
        Some(EMPTY_FOLDER)
    );
    assert_eq!(refreshes(&shell, cx), 0, "no refresh of the desktop's own");

    // The folder selected in the tree is the parent.
    assert_eq!(at, 0, "folders come first");
    shell.keys("secondary-2", cx);
    shell.keys("down", cx);
    shell.keys("up", cx);
    assert!(shell.selected(("doc-node", at), cx), "plans is selected");
    open_dialog(&shell, cx);
    assert_eq!(
        shell.label("new-folder-path", cx).as_deref(),
        Some("docs/plans/")
    );
    shell.visual(cx).simulate_input("drafts");
    shell.run(cx);
    assert_eq!(
        shell.label("new-folder-path", cx).as_deref(),
        Some("docs/plans/drafts")
    );
    shell.keys("enter", cx);
    until(&shell, cx, "the second folder", |shell, cx| {
        creates(shell, cx).len() == 2 && !dialog_open(shell, cx)
    })
    .await;
    let (sent, answer) = answered(&shell, cx).await;
    assert_eq!(answer, ResultCode::Ok);
    assert_eq!(requested_path(&shell, sent, cx), "docs/plans/drafts");
    assert!(root.join("docs/plans/drafts").is_dir());
    assert_eq!(head(&root), commit, "still no commit");
    assert_eq!(refreshes(&shell, cx), 0);
}

/// A create whose answer is lost after it ran (the library's
/// `BeforeRequestSettlement`): the dialog shows its code and stays, and
/// the folder is there. Create again answers `already_applied`, which
/// reads "Folder exists"; one directory, and the tree lists it.
#[gpui_kit::test]
async fn a_folder_retry_reads_folder_exists(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    open_dialog(&shell, cx);
    shell.visual(cx).simulate_input("plans");
    shell.run(cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeRequestSettlement,
        )
        .unwrap(),
    );
    shell.keys("enter", cx);
    let (lost, answer) = answered(&shell, cx).await;
    assert_eq!(answer, ResultCode::InternalError, "lost");
    assert!(dialog_open(&shell, cx), "the dialog stays");
    assert_eq!(
        shell.label("new-folder-answer-code", cx).as_deref(),
        Some("internal_error")
    );
    assert!(root.join("docs/plans").is_dir(), "it ran");

    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    assert!(offered(cx), "Create is offered again");
    shell.keys("enter", cx);
    until(&shell, cx, "the retry's answer", |shell, cx| {
        creates(shell, cx).len() == 2 && code(shell, creates(shell, cx)[0], cx).is_some()
    })
    .await;
    let retried = creates(&shell, cx)[0];
    assert_ne!(retried, lost);
    assert_eq!(code(&shell, retried, cx), Some(ResultCode::AlreadyApplied));
    until(&shell, cx, "Folder exists", |shell, cx| {
        shell.label("new-folder-answer", cx).as_deref() == Some(EXISTS)
    })
    .await;
    assert!(dialog_open(&shell, cx), "the dialog says so");
    assert!(!shell.present("new-folder-answer-code", cx));
    assert!(!offered(cx), "nothing more to create under that name");
    let made: Vec<String> = folders(&shell, &root)
        .into_iter()
        .filter(|folder| folder == "docs/plans")
        .collect();
    assert_eq!(made.len(), 1, "one folder");
    until(&shell, cx, "the tree listing it", |shell, cx| {
        shell.rows("doc-node", cx).contains(&"plans".to_owned())
    })
    .await;
    shell.keys("escape", cx);
    assert!(!dialog_open(&shell, cx));
}

/// A name that is empty, holds a slash or a backslash, is `.` or `..`, or
/// starts with a dot is refused here, in the ruled words, and Enter sends
/// nothing. One the library refuses (a device name) is sent, and its
/// refusal shown with the library's message and code; the dialog stays,
/// and nothing is made.
#[gpui_kit::test]
async fn an_invalid_folder_name_is_refused(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx);
    open_dialog(&shell, cx);
    for name in ["", "a/b", "a\\b", ".", "..", ".hidden"] {
        retype(&shell, name, cx);
        assert_eq!(
            shell.label("new-folder-name-note", cx).as_deref(),
            Some(INVALID),
            "{name:?}"
        );
        assert_eq!(
            shell.label("new-folder-ok-note", cx).as_deref(),
            Some(INVALID),
            "{name:?}"
        );
        assert!(!offered(cx), "{name:?}");
        shell.keys("enter", cx);
        assert!(creates(&shell, cx).is_empty(), "{name:?}: nothing sent");
    }

    retype(&shell, "con", cx);
    assert!(!shell.present("new-folder-name-note", cx));
    assert!(offered(cx));
    shell.keys("enter", cx);
    let (sent, answer) = answered(&shell, cx).await;
    assert_eq!(answer, ResultCode::InvalidPath);
    until(&shell, cx, "the refusal", |shell, cx| {
        shell.label("new-folder-answer-code", cx).as_deref() == Some("invalid_path")
    })
    .await;
    let message = shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(sent).expect("listed").state {
            RequestState::Finished(.., message) => message.clone(),
            _ => panic!("finished"),
        }
    });
    assert_eq!(
        shell.label("new-folder-answer", cx),
        Some(message),
        "the library's message"
    );
    assert!(dialog_open(&shell, cx), "the dialog stays");
    assert!(!root.join("docs/con").exists(), "nothing is made");
}
