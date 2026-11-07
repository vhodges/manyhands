//! Move…, over real repositories (design, Part E: Move; R-23; Cycle
//! contract 11; obligations 1 to 6 and 9 of the Task 24 notes): it is
//! offered on a document sheet's Folder row only while the tab views the
//! document with no session, no draft, no save and no release of its
//! context pending; its dialog checks the destination as it is typed; on
//! Move it reads the document again, observes the destination again and
//! sends `document move` with both observations; a collision and a stale
//! source are refused and explained, and the dialog stays; a move commits
//! both paths and the tab follows the document by its ID; and a move
//! retried after a lost answer shows the moved document.
//!
//! Every test waits on the worker, so it allows parking. Typing goes
//! through the window, into the field holding focus. The ruled words are
//! literals: the tests fix them.

use std::path::{Path, PathBuf};

use git2::Repository;
use gpui_kit::{
    Entity, Focusable, TestAppContext, component::input::InputState, test::TestWindowExt,
};
use manyhands::{
    repository::{
        CancellationToken, DocumentMoveInput, FailurePoint, ItemContextKind, ItemDto, ItemDtoKind,
        Mutation, MutationCall, NoProgress, RepositoryService, RequestId, keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::{MoveDialog, OpenedMove};
use crate::desktop::{
    model::{edit::Phase, navigation::OpenTarget},
    services::{
        JobKind, ReadKind,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, NoPassphrase, item_id},
        snapshot::RepositorySnapshot,
    },
    view::{
        editor::host::EditorHost,
        harness::{Shell, key},
        item::{ChipField, ItemPane},
    },
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const DRAFTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";
const PLAN: &str = "docs/plan.md";

const MOVE: &str = "Move\u{2026}";
const WHILE_VIEWING: &str = "Move is available while viewing, with no draft or save in progress.";
const WAITS: &str = "Move waits while a private copy is removed.";
const STALE: &str = "This document changed since it was read. Close this and try again.";

// ---- the window ----

/// A managed document's source, as another actor commits it.
fn document_source(id: &str, title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n\
         ---\nBody of {title}.\n"
    )
}

/// One repository, `a`, built by `build` and refreshed, in an open window
/// with `a` selected, its lists read.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    (shell, root)
}

/// Opens the tab of `root`'s document `id`, and lets its reads finish.
fn open(shell: &Shell, root: &Path, id: &str, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(id),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

/// Whether the element `id` holds focus.
fn focused(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Opens the details sheet on All details.
fn open_sheet(shell: &Shell, cx: &mut TestAppContext) {
    shell.click(ChipField::All.id(), cx);
    shell.frame(cx, |_, _| ());
}

/// The sheet's Move… is not offered, saying `words`, and a press opens
/// nothing.
fn assert_not_offered(shell: &Shell, words: &str, cx: &mut TestAppContext) {
    open_sheet(shell, cx);
    assert_eq!(shell.label("sheet-move", cx).as_deref(), Some(MOVE));
    assert_eq!(
        shell.label("sheet-move-note", cx).as_deref(),
        Some(words),
        "Move… says why it is not offered"
    );
    shell.click("sheet-move-button", cx);
    assert!(!dialog_open(shell, cx), "a press opens nothing");
    shell.keys("escape", cx);
}

/// The sheet's Move… is offered: no note.
fn assert_offered(shell: &Shell, cx: &mut TestAppContext) {
    open_sheet(shell, cx);
    assert_eq!(shell.label("sheet-move", cx).as_deref(), Some(MOVE));
    assert!(!shell.present("sheet-move-note", cx), "Move… is offered");
}

/// Presses Tab in the open sheet until Move… holds focus.
fn tab_to_move(shell: &Shell, cx: &mut TestAppContext) {
    let mut presses = 0;
    while !focused(shell, "sheet-move-button", cx) {
        presses += 1;
        assert!(presses < 40, "Tab never reached Move…");
        shell.keys("tab", cx);
    }
}

// ---- the dialog ----

fn dialog(cx: &mut TestAppContext) -> Entity<MoveDialog> {
    cx.read(|cx| {
        cx.try_global::<OpenedMove>()
            .and_then(|opened| opened.0.upgrade())
    })
    .expect("a move dialog is open")
}

fn dialog_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("move-dialog", cx)
}

/// The dialog's Folder (`folder`) or File name (`file`).
fn input(name: &str, cx: &mut TestAppContext) -> Entity<InputState> {
    dialog(cx).read_with(cx, |dialog, _| match name {
        "folder" => dialog.folder_input(),
        "file" => dialog.file_input(),
        _ => panic!("the dialog has no {name}"),
    })
}

fn value(name: &str, cx: &mut TestAppContext) -> String {
    input(name, cx).read_with(cx, |input, _| input.value().to_string())
}

fn holds_focus(shell: &Shell, name: &str, cx: &mut TestAppContext) -> bool {
    let handle = input(name, cx).read_with(cx, |input, cx| input.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Puts `typed` in the input `name` in place of what it holds, through the
/// window: focus there, select all, type.
fn retype(shell: &Shell, name: &str, typed: &str, cx: &mut TestAppContext) {
    let input = input(name, cx);
    shell.frame(cx, |window, cx| {
        input.update(cx, |input, cx| input.focus(window, cx))
    });
    shell.frame(cx, |_, _| ());
    shell.keys("secondary-a", cx);
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

fn offered(cx: &mut TestAppContext) -> bool {
    dialog(cx).read_with(cx, |dialog, _| dialog.offered())
}

/// Opens Move… from the sheet, and finds the destination checked.
fn open_move(shell: &Shell, cx: &mut TestAppContext) {
    open_sheet(shell, cx);
    shell.click("sheet-move-button", cx);
    assert!(dialog_open(shell, cx), "Move… opens its dialog");
}

// ---- the requests and the repository ----

/// This session's moves, newest first.
fn moves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentMove { .. })
    })
}

/// The request `id`'s attempt and, once it has one, its answer's code.
fn answer(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> (u32, Option<ResultCode>) {
    shell.operations(cx).read_with(cx, |operations, _| {
        let request = operations.request(id).expect("the request is listed");
        let code = match &request.state {
            RequestState::Finished(_, code, ..) => Some(*code),
            _ => None,
        };
        (request.attempt, code)
    })
}

fn intent(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Intent {
    shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .request(id)
            .map(|request| request.intent.clone())
            .expect("the request is listed")
    })
}

/// How many commits `branch` of `root` has.
fn commits(root: &Path, branch: &str) -> usize {
    let repository = Repository::open(root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push(tip.id()).unwrap();
    walk.count()
}

/// Whether the tip of `branch` (`at` 0) or its parent (`at` 1) holds a
/// file at `path`.
fn holds(root: &Path, branch: &str, at: usize, path: &str) -> bool {
    let repository = Repository::open(root).unwrap();
    let mut commit = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    for _ in 0..at {
        commit = commit.parent(0).unwrap();
    }
    commit.tree().unwrap().get_path(Path::new(path)).is_ok()
}

/// The paths the document tree shows, in its order.
fn tree_paths(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let nodes = shell.rows("doc-node", cx).len();
    (0..nodes)
        .filter_map(|ix| shell.label(("doc-path", ix), cx))
        .collect()
}

fn failing_at(shell: &Shell, point: FailurePoint) {
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(shell.fixture.data_dir(), point)
            .unwrap(),
    );
}

fn working(shell: &Shell) {
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
}

// ---- editing ----

/// Clicks Edit and waits for the edit editor.
async fn edit(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorHost> {
    shell.click("item-edit", cx);
    shell
        .until_drawn(cx, "the edit editor opening", |shell, cx| {
            pane(shell, cx).read_with(cx, |pane, _| pane.edit_host().is_some())
                && phase(shell, cx) == Phase::Editing
        })
        .await;
    pane(shell, cx)
        .read_with(cx, |pane, _| pane.edit_host())
        .unwrap()
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}

/// Types `typed` at the end of the edit editor's text: the caret set, the
/// editor focused and a frame drawn first.
fn type_at_end(shell: &Shell, host: &Entity<EditorHost>, typed: &str, cx: &mut TestAppContext) {
    let editor = host.read_with(cx, |host, _| host.editor().clone());
    let end = editor.read_with(cx, |editor, _| editor.text().len());
    editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
}

// ---- the tests ----

/// By keyboard: Tab to Move… in the sheet and Enter; focus starts in the
/// Folder, which with the File name is the document's own, so Move waits;
/// another folder, checked free, and Enter moves it. The library holds the
/// document at its new path, one commit on its branch changing both paths;
/// the dialog closes; the tab, the same by its ID, shows the new folder,
/// and the tree the moved document.
#[gpui_kit::test]
async fn move_from_the_sheet_commits_both_paths_and_the_tab_follows(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, PLAN, "Plan");
        fixture.create_folder(root, "docs/archive");
    });
    open(&shell, &root, DOCUMENT, cx);
    let before = shown(&shell, &root, DOCUMENT);
    let branch = before
        .context
        .branch
        .clone()
        .expect("the document's branch");
    let commits_before = commits(&root, &branch);
    open_sheet(&shell, cx);
    assert_eq!(shell.label("item-folder", cx).as_deref(), Some("docs"));
    tab_to_move(&shell, cx);
    shell.press("enter", cx);
    assert!(
        dialog_open(&shell, cx),
        "Move… opens its dialog by keyboard"
    );
    assert!(
        holds_focus(&shell, "folder", cx),
        "focus starts in the Folder"
    );
    assert_eq!(value("folder", cx), "docs");
    assert_eq!(value("file", cx), "plan.md");
    assert!(!offered(cx), "the document is there already");
    assert!(shell.present("move-ok-note", cx), "Move says why it waits");
    shell.keys("enter", cx);
    assert!(moves(&shell, cx).is_empty(), "Enter moves nothing yet");

    retype(&shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    assert!(!shell.present("move-ok-note", cx), "Move says nothing");
    shell.keys("enter", cx);
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !dialog_open(shell, cx)
        })
        .await;

    let sent = moves(&shell, cx);
    assert_eq!(sent.len(), 1, "one move");
    assert_eq!(answer(&shell, sent[0], cx), (1, Some(ResultCode::Ok)));
    let Intent::DocumentMove {
        id,
        observation,
        destination_path,
        destination_observation,
        ..
    } = intent(&shell, sent[0], cx)
    else {
        panic!("a move");
    };
    assert_eq!(id.to_string(), DOCUMENT);
    assert_eq!(observation, before.observation, "the source as read");
    assert_eq!(destination_path, PathBuf::from("docs/archive/plan.md"));
    assert!(
        !destination_observation.is_empty(),
        "the destination's token"
    );

    let after = shown(&shell, &root, DOCUMENT);
    assert_eq!(after.path, "docs/archive/plan.md");
    assert_eq!(commits(&root, &branch), commits_before + 1, "one commit");
    assert!(holds(&root, &branch, 0, "docs/archive/plan.md"));
    assert!(!holds(&root, &branch, 0, PLAN), "the old path is emptied");
    assert!(holds(&root, &branch, 1, PLAN), "in the same commit");

    shell
        .until_drawn(cx, "the tab following", |shell, cx| {
            shell.label("item-folder", cx).as_deref() == Some("docs/archive")
        })
        .await;
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(
        opened,
        Some(OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        }),
        "the same tab, by its ID"
    );
    assert_eq!(
        shell.label("item-file-name", cx).as_deref(),
        Some("plan.md")
    );
    shell.keys("escape", cx);
    shell
        .until_drawn(cx, "the tree reading again", |shell, cx| {
            tree_paths(shell, cx) == ["docs/archive/plan.md"]
        })
        .await;
}

/// A file another actor commits at the destination after it was checked
/// free: Move observes it again, sends nothing, says so, and stays open.
/// Escape then closes it.
#[gpui_kit::test]
async fn a_collision_is_refused_and_explained_and_the_dialog_stays(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "file", "taken.md", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;

    shell.fixture.commit_file(
        &root,
        "docs/taken.md",
        document_source(DRAFTED, "Taken").as_bytes(),
    );
    let before = RepositorySnapshot::take(&root);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the refusal", |shell, cx| {
            shell.label("move-file-note", cx).is_some()
        })
        .await;
    let words = "Another file is already at docs/taken.md.";
    assert_eq!(shell.label("move-file-note", cx).as_deref(), Some(words));
    assert_eq!(shell.label("move-ok-note", cx).as_deref(), Some(words));
    assert!(dialog_open(&shell, cx), "the dialog stays");
    assert!(!offered(cx));
    assert!(moves(&shell, cx).is_empty(), "nothing was sent");
    assert_eq!(RepositorySnapshot::take(&root), before, "nothing changed");
    assert_eq!(shown(&shell, &root, DOCUMENT).path, PLAN);

    // Another name is free again, and Escape closes the dialog.
    retype(&shell, "file", "free.md", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    shell.keys("escape", cx);
    assert!(!dialog_open(&shell, cx), "Escape closes it");
    assert!(moves(&shell, cx).is_empty(), "nothing was sent");
}

/// The document changes after Move read it and before the library takes
/// the move: the library answers `external_change`; the document read
/// again by its ID is still where it was, so the dialog says it changed
/// and stays, and nothing moved.
#[gpui_kit::test]
async fn a_stale_source_is_refused(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;

    let held = shell.backend.hold_job(JobKind::DocumentMove);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the move being sent", |shell, cx| {
            !moves(shell, cx).is_empty()
        })
        .await;
    held.wait_reached();
    shell
        .fixture
        .commit_file(&root, PLAN, document_source(DOCUMENT, "Changed").as_bytes());
    held.release();
    let sent = moves(&shell, cx)[0];
    shell
        .until_drawn(cx, "the stale words", |shell, cx| {
            shell.label("move-failure", cx).as_deref() == Some(STALE)
        })
        .await;
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::ExternalChange))
    );
    assert!(dialog_open(&shell, cx), "the dialog stays");
    assert_eq!(moves(&shell, cx).len(), 1, "nothing more was sent");
    let after = shown(&shell, &root, DOCUMENT);
    assert_eq!(after.path, PLAN, "nothing moved");
    assert_eq!(after.title.as_deref(), Some("Changed"));
}

/// R-23: Move… is offered while viewing with nothing of the item kept,
/// and not while editing, saving, with a session kept after Done, or with
/// a stored draft of the item. It says why, and a press opens nothing.
/// Offered, its dialog closes on Escape and gives focus back to it.
#[gpui_kit::test]
async fn move_is_offered_only_while_viewing_with_no_draft(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, PLAN, "Plan");
        fixture.create_document(root, DRAFTED, "docs/drafted.md", "Drafted");
        fixture.refresh(root);
        store_draft(
            fixture,
            root,
            ItemDtoKind::Document,
            DRAFTED,
            "From an earlier session.\n",
        );
    });
    open(&shell, &root, DOCUMENT, cx);
    assert_offered(&shell, cx);
    tab_to_move(&shell, cx);
    shell.press("enter", cx);
    assert!(dialog_open(&shell, cx), "offered, Move… opens its dialog");
    shell.keys("escape", cx);
    assert!(!dialog_open(&shell, cx), "Escape closes it");
    assert!(
        focused(&shell, "sheet-move-button", cx),
        "focus goes back to Move…"
    );
    shell.keys("escape", cx);

    // Editing.
    let host = edit(&shell, cx).await;
    assert_not_offered(&shell, WHILE_VIEWING, cx);

    // Saving.
    type_at_end(&shell, &host, " More.", cx);
    let held = shell.backend.hold_job(JobKind::DocumentSave);
    shell.click("item-save", cx);
    held.wait_reached();
    assert_eq!(phase(&shell, cx), Phase::Saving);
    assert_not_offered(&shell, WHILE_VIEWING, cx);
    held.release();
    shell
        .until_drawn(cx, "the save ending", |shell, cx| {
            phase(shell, cx) == Phase::Editing
        })
        .await;

    // A session kept after Done: its text differs and its draft is
    // written.
    type_at_end(&shell, &host, " Again.", cx);
    shell.click("item-done", cx);
    shell
        .until_drawn(cx, "Done keeping the session", |shell, cx| {
            phase(shell, cx) == Phase::Viewing
        })
        .await;
    assert!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.has_session()),
        "the session is kept"
    );
    assert_not_offered(&shell, WHILE_VIEWING, cx);

    // A stored draft of the item, with no session.
    open(&shell, &root, DRAFTED, cx);
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert!(!pane(&shell, cx).read_with(cx, |pane, _| pane.has_session()));
    assert_not_offered(&shell, WHILE_VIEWING, cx);
    assert!(moves(&shell, cx).is_empty(), "nothing was sent");
}

/// R-23: a context this process made, released as its tab leaves clean.
/// While that release is queued or running Move… waits, saying so; once
/// it has ended, Move… is offered again.
#[gpui_kit::test]
async fn move_is_disabled_while_a_release_is_queued_or_running(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    edit(&shell, cx).await;
    let held = shell.backend.hold_job(JobKind::ContextRelease);
    shell.click("item-done", cx);
    let releases = |shell: &Shell, cx: &mut TestAppContext| {
        shell.requests(cx, |request| {
            matches!(request.intent, Intent::ContextRelease { .. })
        })
    };
    shell
        .until_drawn(cx, "the release being sent", |shell, cx| {
            !releases(shell, cx).is_empty()
        })
        .await;
    held.wait_reached();
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert!(!pane(&shell, cx).read_with(cx, |pane, _| pane.has_session()));
    assert_not_offered(&shell, WAITS, cx);

    held.release();
    let release = releases(&shell, cx)[0];
    shell
        .until_drawn(cx, "the release ending", |shell, cx| {
            answer(shell, release, cx).1.is_some()
        })
        .await;
    assert_eq!(answer(&shell, release, cx).1, Some(ResultCode::Ok));
    assert_offered(&shell, cx);
    shell.click("sheet-move-button", cx);
    assert!(dialog_open(&shell, cx), "Move… opens its dialog");
}

/// Whether the draft store's last listing is ready.
fn drafts_listed(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    drafts.read_with(cx, |registry, _| registry.listing().rows().is_some())
}

/// R-23 (Part E review, M1): a stored draft of the document is known only
/// from the draft store's listing, so Move… waits for it, as a release
/// does. With the launch listing held, Move… is not offered, in R-23's
/// words; once it lands, it is still not, the draft now listed.
#[gpui_kit::test]
async fn move_waits_for_the_draft_listing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, PLAN, "Plan");
    fixture.refresh(&root);
    store_draft(
        &fixture,
        &root,
        ItemDtoKind::Document,
        DOCUMENT,
        "From an earlier session.\n",
    );
    let mut listing = None;
    let shell = Shell::launch(
        fixture,
        |backend| listing = Some(backend.hold_read(ReadKind::Drafts)),
        cx,
    );
    shell.select(&root, cx);
    open(&shell, &root, DOCUMENT, cx);
    assert!(!drafts_listed(&shell, cx), "the listing is held");
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert_not_offered(&shell, WHILE_VIEWING, cx);

    listing.unwrap().send(()).unwrap();
    shell
        .until_drawn(cx, "the draft listing", drafts_listed)
        .await;
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Viewing \u{b7} unsaved changes kept"),
        "the draft is listed"
    );
    assert_not_offered(&shell, WHILE_VIEWING, cx);
    assert!(moves(&shell, cx).is_empty(), "nothing was sent");
}

/// R-23 (Part E fix round 2): a draft store that did not open is never
/// listed, and no draft is read from it or kept in it, so Move… does not
/// wait for a listing then. Its folder is a link, as `edit_tests` makes
/// it. Move… is offered on its other conditions, and a press opens its
/// dialog.
#[cfg(unix)]
#[gpui_kit::test]
async fn move_is_offered_while_the_draft_store_did_not_open(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_file(&root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    open(&shell, &root, DOCUMENT, cx);
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    assert!(
        drafts.read_with(cx, |registry, _| registry.unavailable().is_some()),
        "the store did not open"
    );
    assert!(!drafts_listed(&shell, cx), "it is never listed");
    assert_offered(&shell, cx);
    shell.click("sheet-move-button", cx);
    assert!(dialog_open(&shell, cx), "Move… opens its dialog");
}

/// Moves `root`'s document to `docs/archive` from its tab's sheet, with
/// the answer lost after the move ran (the library's
/// `BeforeRequestSettlement`, as Task 22's `lost_create` loses a create's):
/// the dialog shows the lost answer's code and stays. Gives the request,
/// with the service working again.
async fn lost_move(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> DesktopRequestId {
    open_move(shell, cx);
    retype(shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    failing_at(shell, FailurePoint::BeforeRequestSettlement);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the lost answer", |shell, cx| {
            moves(shell, cx)
                .first()
                .is_some_and(|sent| answer(shell, *sent, cx).1.is_some())
        })
        .await;
    working(shell);
    let sent = moves(shell, cx)[0];
    assert_eq!(
        answer(shell, sent, cx),
        (1, Some(ResultCode::InternalError)),
        "lost"
    );
    assert_eq!(
        shown(shell, root, DOCUMENT).path,
        "docs/archive/plan.md",
        "it ran"
    );
    assert!(dialog_open(shell, cx), "the dialog stays");
    assert_eq!(
        shell.label("move-failure-code", cx).as_deref(),
        Some("internal_error")
    );
    sent
}

/// Sends the request `sent` again under its own request ID, as the
/// worker's retry does, and waits for its second answer.
async fn retried(shell: &Shell, sent: DesktopRequestId, cx: &mut TestAppContext) -> ResultCode {
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.retry(sent, None), "sent again");
    shell
        .until_drawn(cx, "the retried answer", |shell, cx| {
            matches!(answer(shell, sent, cx), (2, Some(_)))
        })
        .await;
    answer(shell, sent, cx).1.unwrap()
}

/// The dialog closed as for a success, the same request the only move,
/// and the tab shows the moved document.
async fn shows_the_moved_document(shell: &Shell, sent: DesktopRequestId, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !dialog_open(shell, cx)
        })
        .await;
    assert_eq!(moves(shell, cx), [sent], "the same request");
    shell
        .until_drawn(cx, "the tab showing the moved document", |shell, cx| {
            shell.label("item-folder", cx).as_deref() == Some("docs/archive")
        })
        .await;
    assert!(!shell.present("move-failure", cx));
}

/// A move whose answer is lost after it ran has moved the document; the
/// dialog shows the lost answer's code and stays. Sent again under the
/// same request, as the worker's retry does, it is answered from what is
/// there, with no second commit: the binding works around known defect
/// `01M4PB80T7` for a source that was committed. The dialog closes as for
/// a success, and the tab shows the moved document.
#[gpui_kit::test]
async fn a_move_retried_after_a_lost_answer_shows_the_moved_document(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, PLAN, "Plan");
        fixture.create_folder(root, "docs/archive");
    });
    open(&shell, &root, DOCUMENT, cx);
    let branch = shown(&shell, &root, DOCUMENT)
        .context
        .branch
        .expect("the document's branch");
    let sent = lost_move(&shell, &root, cx).await;
    let moved = commits(&root, &branch);

    let code = retried(&shell, sent, cx).await;
    assert!(
        matches!(code, ResultCode::Ok | ResultCode::AlreadyApplied),
        "answered from what is there: {}",
        code.as_str()
    );
    shows_the_moved_document(&shell, sent, cx).await;
    assert_eq!(commits(&root, &branch), moved, "no second commit");
    assert_eq!(shown(&shell, &root, DOCUMENT).path, "docs/archive/plan.md");
}

/// Runs `mutation` as another actor, through the library, and checks it
/// answered `ok`.
fn by_another(shell: &Shell, mutation: Mutation) {
    let done = shell.fixture.service().execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(done.code, ResultCode::Ok, "{}", done.message);
}

/// Another actor moves `root`'s document to `destination` through the
/// library, as `observe_path` and `show_item` read it now; gives the
/// worktree of its context.
fn move_by_another(shell: &Shell, root: &Path, destination: &str) -> PathBuf {
    let before = shown(shell, root, DOCUMENT);
    let worktree = PathBuf::from(&before.context.worktree);
    let context = (before.context.kind == ItemContextKind::Active).then_some(worktree.as_path());
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let free = service
        .observe_path(&repo, context, Path::new(destination))
        .unwrap();
    by_another(
        shell,
        Mutation::DocumentMove(DocumentMoveInput {
            root: root.to_owned(),
            id: item_id(DOCUMENT),
            observation: before.observation.clone(),
            destination_path: PathBuf::from(destination),
            destination_observation: free.observation,
        }),
    );
    worktree
}

/// The pre-flight ruling on `external_change` (Task 24): while the move is
/// with the worker, another actor moves the document to the same
/// destination and leaves a change to it uncommitted, so the library
/// answers the move `external_change`. Read again by its ID, the document
/// is at the destination: the dialog closes as for a success, with no
/// stale words, and the tab shows the moved document.
#[gpui_kit::test]
async fn a_move_answered_external_change_at_its_destination_is_done(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, PLAN, "Plan");
        fixture.create_folder(root, "docs/archive");
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    let held = shell.backend.hold_job(JobKind::DocumentMove);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the move being sent", |shell, cx| {
            !moves(shell, cx).is_empty()
        })
        .await;
    held.wait_reached();
    let sent = moves(&shell, cx)[0];

    // Another actor moves it there, and changes it without committing.
    let destination = "docs/archive/plan.md";
    let worktree = move_by_another(&shell, &root, destination);
    std::fs::write(
        worktree.join(destination),
        document_source(DOCUMENT, "Plan, edited").as_bytes(),
    )
    .unwrap();

    held.release();
    shell
        .until_drawn(cx, "the move's answer", |shell, cx| {
            answer(shell, sent, cx).1.is_some()
        })
        .await;
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::ExternalChange))
    );
    shows_the_moved_document(&shell, sent, cx).await;
    assert_eq!(shown(&shell, &root, DOCUMENT).path, destination);
}

/// A move whose checkpoint commits while its hand-off to the index is
/// still owed (`discovery_pending`, the library's
/// `BeforeIndexTransactionCommit`) has moved the document: the dialog
/// closes as for a success, saying nothing, and the operation area holds
/// the answer.
#[gpui_kit::test]
async fn a_committed_move_still_owing_its_index_closes_the_dialog(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, PLAN, "Plan");
        fixture.create_folder(root, "docs/archive");
    });
    open(&shell, &root, DOCUMENT, cx);
    let branch = shown(&shell, &root, DOCUMENT)
        .context
        .branch
        .expect("the document's branch");
    let before = commits(&root, &branch);
    open_move(&shell, cx);
    retype(&shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    failing_at(&shell, FailurePoint::BeforeIndexTransactionCommit);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the move's answer", |shell, cx| {
            moves(shell, cx)
                .first()
                .is_some_and(|sent| answer(shell, *sent, cx).1.is_some())
        })
        .await;
    working(&shell);
    let sent = moves(&shell, cx)[0];
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::DiscoveryPending))
    );
    assert_eq!(commits(&root, &branch), before + 1, "committed");
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !dialog_open(shell, cx)
        })
        .await;
    assert!(!shell.present("move-failure", cx));
}

/// After a lost answer the user presses Move again: the document, read
/// again by its ID, is at the destination already, so the dialog closes as
/// for a success with nothing more sent, and the tab and the tree read the
/// repository again and show the moved document.
#[gpui_kit::test]
async fn move_pressed_again_after_a_lost_answer_finds_the_document_moved(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, PLAN, "Plan");
        fixture.create_folder(root, "docs/archive");
    });
    open(&shell, &root, DOCUMENT, cx);
    let sent = lost_move(&shell, &root, cx).await;
    assert!(offered(cx), "Move is offered again");
    shell.click("move-ok", cx);
    shows_the_moved_document(&shell, sent, cx).await;
    shell.keys("escape", cx);
    shell
        .until_drawn(cx, "the tree reading again", |shell, cx| {
            tree_paths(shell, cx) == ["docs/archive/plan.md"]
        })
        .await;
}

// ---- fix round 1 ----

/// Fix 1, M2: the dialog's observe at Move finds the destination free;
/// another actor commits a file there before the library takes the move.
/// The library answers `external_change`, its token of the destination no
/// longer holding; the document is where it was read, and the destination
/// observed again holds a file, so the dialog says so under the File name,
/// not that the document changed, and stays.
#[gpui_kit::test]
async fn a_destination_taken_while_the_move_waits_is_explained_and_the_dialog_stays(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "file", "taken.md", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    let held = shell.backend.hold_job(JobKind::DocumentMove);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the move being sent", |shell, cx| {
            !moves(shell, cx).is_empty()
        })
        .await;
    held.wait_reached();
    shell.fixture.commit_file(
        &root,
        "docs/taken.md",
        document_source(DRAFTED, "Taken").as_bytes(),
    );
    held.release();
    let sent = moves(&shell, cx)[0];
    shell
        .until_drawn(cx, "the move's answer", |shell, cx| {
            answer(shell, sent, cx).1.is_some()
        })
        .await;
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::ExternalChange))
    );
    let words = "Another file is already at docs/taken.md.";
    shell
        .until_drawn(cx, "the collision's words", |shell, cx| {
            shell.label("move-file-note", cx).as_deref() == Some(words)
        })
        .await;
    assert!(dialog_open(&shell, cx), "the dialog stays");
    assert!(!offered(cx));
    assert!(!shell.present("move-failure", cx), "not the stale words");
    assert_eq!(shown(&shell, &root, DOCUMENT).path, PLAN, "nothing moved");
}

/// Fix 1, M2: the library's own `occupied_path`, which this dialog cannot
/// draw from the library (its observe at Move refuses a file there first,
/// and one put there later makes the library answer `external_change`),
/// given to the dialog as an answer: the destination is held, said under
/// the File name and on Move, with no code, and the dialog stays.
#[gpui_kit::test]
async fn an_occupied_path_answer_holds_the_destination(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "file", "taken.md", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    dialog(cx).update(cx, |dialog, cx| {
        let code = ResultCode::OccupiedPath;
        let message = code.message().to_owned();
        dialog.answered(code, false, message, "docs/taken.md".to_owned(), cx)
    });
    let words = "Another file is already at docs/taken.md.";
    assert_eq!(shell.label("move-file-note", cx).as_deref(), Some(words));
    assert_eq!(shell.label("move-ok-note", cx).as_deref(), Some(words));
    assert!(!shell.present("move-failure-code", cx));
    assert!(dialog_open(&shell, cx), "the dialog stays");
    assert!(!offered(cx));
}

/// Fix 1, M2: another actor moves the document to the destination before
/// Move is pressed. Read again, it is there: the dialog closes with no
/// move of this session's, and the tab and the tree, which no answer told,
/// read the repository again and follow it.
#[gpui_kit::test]
async fn a_document_another_moved_there_is_found_moved_and_the_tab_follows(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, DOCUMENT, PLAN, "Plan");
        fixture.create_folder(root, "docs/archive");
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    assert_eq!(shell.label("item-folder", cx).as_deref(), Some("docs"));
    retype(&shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    move_by_another(&shell, &root, "docs/archive/plan.md");
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !dialog_open(shell, cx)
        })
        .await;
    assert!(moves(&shell, cx).is_empty(), "this session moved nothing");
    shell
        .until_drawn(cx, "the tab following", |shell, cx| {
            shell.label("item-folder", cx).as_deref() == Some("docs/archive")
        })
        .await;
    shell.keys("escape", cx);
    shell
        .until_drawn(cx, "the tree following", |shell, cx| {
            tree_paths(shell, cx) == ["docs/archive/plan.md"]
        })
        .await;
}

/// Fix 1, M4: a change of the fields clears the last refusal of the
/// destination they gave, and a held destination is held again when the
/// fields give it again.
#[gpui_kit::test]
async fn a_field_change_clears_the_last_refusal_and_a_held_path_stays_held(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "file", "taken.md", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    shell.fixture.commit_file(
        &root,
        "docs/taken.md",
        document_source(DRAFTED, "Taken").as_bytes(),
    );
    shell.click("move-ok", cx);
    let words = "Another file is already at docs/taken.md.";
    shell
        .until_drawn(cx, "the collision", |shell, cx| {
            shell.label("move-file-note", cx).as_deref() == Some(words)
        })
        .await;
    retype(&shell, "file", "free.md", cx);
    shell
        .until_drawn(cx, "another name checked", |_, cx| offered(cx))
        .await;
    assert!(!shell.present("move-file-note", cx));
    retype(&shell, "file", "taken.md", cx);
    shell
        .until_drawn(cx, "the held name held again", |shell, cx| {
            shell.label("move-file-note", cx).as_deref() == Some(words)
        })
        .await;
    assert!(!offered(cx));

    // A library refusal: the answer lost, its code shown, then cleared.
    retype(&shell, "file", "free.md", cx);
    shell
        .until_drawn(cx, "another name checked", |_, cx| offered(cx))
        .await;
    failing_at(&shell, FailurePoint::BeforeRequestSettlement);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the lost answer's code", |shell, cx| {
            shell.label("move-failure-code", cx).as_deref() == Some("internal_error")
        })
        .await;
    working(&shell);
    retype(&shell, "file", "other.md", cx);
    assert!(!shell.present("move-failure", cx), "cleared by the change");
    assert!(!shell.present("move-failure-code", cx));
}

/// Fix 1, M4: after the stale words Move is not offered again in that
/// dialog, whatever the fields give, and says why; Move… opened again
/// reads afresh and offers it.
#[gpui_kit::test]
async fn a_stale_move_is_not_offered_again_until_reopened(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    let held = shell.backend.hold_job(JobKind::DocumentMove);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the move being sent", |shell, cx| {
            !moves(shell, cx).is_empty()
        })
        .await;
    held.wait_reached();
    shell
        .fixture
        .commit_file(&root, PLAN, document_source(DOCUMENT, "Changed").as_bytes());
    held.release();
    shell
        .until_drawn(cx, "the stale words", |shell, cx| {
            shell.label("move-failure", cx).as_deref() == Some(STALE)
        })
        .await;
    assert!(!offered(cx), "not offered after the stale words");
    assert_eq!(shell.label("move-ok-note", cx).as_deref(), Some(STALE));
    retype(&shell, "folder", "docs/elsewhere", cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert!(!offered(cx), "nor for another destination");
    assert_eq!(
        shell.label("move-failure", cx).as_deref(),
        Some(STALE),
        "the words stay"
    );
    shell.keys("enter", cx);
    shell.click("move-ok", cx);
    assert_eq!(moves(&shell, cx).len(), 1, "nothing more is sent");

    shell.keys("escape", cx);
    assert!(!dialog_open(&shell, cx), "Escape closes it");
    shell.click("sheet-move-button", cx);
    assert!(dialog_open(&shell, cx), "Move… opens again");
    retype(&shell, "folder", "docs/archive", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    assert!(!shell.present("move-failure", cx), "read afresh");
}

/// Part E review, M5: while a move runs, Cancel is disabled, as Move is,
/// so Tab, which reaches it before, passes it by. The dialog stays until
/// the move lands, then closes.
#[gpui_kit::test]
async fn cancel_is_disabled_while_a_move_runs(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, PLAN, document_source(DOCUMENT, "Plan").as_bytes());
    });
    open(&shell, &root, DOCUMENT, cx);
    open_move(&shell, cx);
    retype(&shell, "file", "moved.md", cx);
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;
    let cancel = ("move-cancel", 0usize);
    assert!(shell.tab_reaches(cancel, 12, cx), "Tab reaches Cancel");
    retype(&shell, "file", "moved.md", cx);
    assert!(
        holds_focus(&shell, "file", cx),
        "focus back in the File name"
    );
    shell
        .until_drawn(cx, "the destination check", |_, cx| offered(cx))
        .await;

    let held = shell.backend.hold_job(JobKind::DocumentMove);
    shell.click("move-ok", cx);
    shell
        .until_drawn(cx, "the move being sent", |shell, cx| {
            !moves(shell, cx).is_empty()
        })
        .await;
    held.wait_reached();
    assert!(
        !shell.tab_reaches(cancel, 12, cx),
        "Cancel is disabled while the move runs"
    );
    assert!(dialog_open(&shell, cx), "the dialog waits");

    held.release();
    shell
        .until_drawn(cx, "the dialog closing", |shell, cx| {
            !dialog_open(shell, cx)
        })
        .await;
    assert_eq!(shown(&shell, &root, DOCUMENT).path, "docs/moved.md");
}
