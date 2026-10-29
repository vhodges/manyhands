//! The create dialogs over real repositories and a real store (design,
//! Create (decision 3); R-20, R-21, R-22, R-46; obligations 1 to 9 of the
//! Task 23 notes): New document… and New ticket… open on the selected
//! repository; the new item's draft is in the store before its Create
//! prepare runs; a document's path is checked here, then with
//! `observe_path`, under a stamp; Save creates once, and the tab goes on
//! as an ordinary edit session; a new ticket is created with its fields and
//! the dependency and parent its sheet chose; the new tab's header is the
//! draft's; a file at the path at Save keeps the draft; a repository with
//! no initials points to Settings; `secondary-n` opens the shown list's
//! dialog, in the navigation region only.
//!
//! Draft types derive no `Debug`; they are read through `match` and
//! `assert!`. Every test waits on the worker, so it allows parking. Typing
//! goes through the window, into the field holding focus; each editor test
//! sets the caret, focuses the editor and draws a frame first. The words
//! are literals: the tests fix them.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use git2::Repository;
use gpui_kit::{Entity, Focusable, TestAppContext, component::input::InputState};
use manyhands::{
    drafts::{Draft, DraftStore, Entry, Held, ItemKind, is_new_item_base},
    repository::{ItemDto, ItemDtoKind},
    results::ResultCode,
};

use super::{CreateDialog, Inputs, OpenedDialog};
use crate::desktop::{
    model::{edit::Phase, navigation::OpenTarget, stamp::SettingsPage},
    services::{
        JobKind, ReadKind,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        editor::host::EditorHost,
        harness::{Shell, WAIT, key},
        item::ChipField,
        settings::Shown,
    },
};

/// Tickets the fixture commits, for the new ticket's relationships.
const DEPENDENCY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH2";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH3";

// ---- the window ----

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

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<crate::desktop::view::item::ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Option<Phase> {
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())?;
    Some(pane.read_with(cx, |pane, _| pane.edit_phase()))
}

/// The shown tab's edit host, once its editor has opened.
async fn until_editing(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorHost> {
    until(shell, cx, "the edit editor opening", |shell, cx| {
        phase(shell, cx) == Some(Phase::Editing)
            && pane(shell, cx).read_with(cx, |pane, _| pane.edit_host().is_some())
    })
    .await;
    pane(shell, cx)
        .read_with(cx, |pane, _| pane.edit_host())
        .unwrap()
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

/// The item the tab shows, as its target names it.
fn opened(shell: &Shell, cx: &mut TestAppContext) -> Option<OpenTarget> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
}

/// The new item's ID, from its tab.
fn opened_id(shell: &Shell, cx: &mut TestAppContext) -> String {
    match opened(shell, cx) {
        Some(OpenTarget::Item { id, .. }) => id.to_string(),
        _ => panic!("an item's tab is shown"),
    }
}

// ---- the dialog ----

/// The create dialog last opened, while it lives.
fn dialog(shell: &Shell, cx: &mut TestAppContext) -> Entity<CreateDialog> {
    let _ = shell;
    cx.read(|cx| {
        cx.try_global::<OpenedDialog>()
            .and_then(|opened| opened.0.upgrade())
    })
    .expect("a create dialog is open")
}

/// The dialog's input called `name`: title, type, folder or file.
fn input(shell: &Shell, name: &str, cx: &mut TestAppContext) -> Entity<InputState> {
    dialog(shell, cx).read_with(cx, |dialog, _| match (name, &dialog.inputs) {
        ("title", _) => dialog.title.clone(),
        ("type", Inputs::Ticket { ticket_type }) => ticket_type.clone(),
        ("folder", Inputs::Document(place)) => place.folder.clone(),
        ("file", Inputs::Document(place)) => place.file_name.clone(),
        _ => panic!("the dialog has no {name}"),
    })
}

fn value(shell: &Shell, name: &str, cx: &mut TestAppContext) -> String {
    input(shell, name, cx).read_with(cx, |input, _| input.value().to_string())
}

/// Whether the input `name` holds focus.
fn holds_focus(shell: &Shell, name: &str, cx: &mut TestAppContext) -> bool {
    let handle = input(shell, name, cx).read_with(cx, |input, cx| input.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Puts `typed` in the input `name` in place of what it holds, through the
/// window: focus there, select all, type.
fn retype(shell: &Shell, name: &str, typed: &str, cx: &mut TestAppContext) {
    let input = input(shell, name, cx);
    shell.frame(cx, |window, cx| {
        input.update(cx, |input, cx| input.focus(window, cx))
    });
    shell.frame(cx, |_, _| ());
    shell.keys("secondary-a", cx);
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// Whether Create is offered now.
fn offered(shell: &Shell, cx: &mut TestAppContext) -> bool {
    dialog(shell, cx).read_with(cx, |dialog, cx| dialog.offered(cx))
}

/// What the dialog's Create says while it is not offered.
fn create_note(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("create-ok-note", cx)
}

fn dialog_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("create-dialog", cx)
}

/// Opens New document…, titles it `title` and names its file `file`, in
/// the default folder, then waits for the path check to find it free.
async fn new_document(shell: &Shell, title: &str, file: &str, cx: &mut TestAppContext) {
    shell.click("new-document-button", cx);
    assert!(dialog_open(shell, cx), "New document… opens its dialog");
    assert!(holds_focus(shell, "title", cx), "focus starts in the Title");
    shell.visual(cx).simulate_input(title);
    shell.run(cx);
    retype(shell, "file", file, cx);
    until(shell, cx, "the path check", offered).await;
}

// ---- the requests and the store ----

/// This session's `context prepare` requests, oldest first: whether each
/// was a Create's, and its request.
fn prepares(shell: &Shell, cx: &mut TestAppContext) -> Vec<(bool, DesktopRequestId)> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .rev()
            .filter_map(|request| match &request.intent {
                Intent::ContextPrepare { intent, .. } => Some((
                    matches!(intent, manyhands::repository::ContextPrepareIntent::Create),
                    request.id,
                )),
                _ => None,
            })
            .collect()
    })
}

/// This session's creates, newest first.
fn creates(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::TicketCreate { .. } | Intent::DocumentCreate { .. }
        )
    })
}

/// This session's document saves, newest first.
fn document_saves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentSave { .. })
    })
}

fn request_code(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<ResultCode> {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.state {
            RequestState::Finished(_, code, ..) => Some(*code),
            _ => None,
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

/// Waits until the request `id` has an answer, and gives its code.
async fn answered(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> ResultCode {
    until(shell, cx, "the answer", |shell, cx| {
        request_code(shell, id, cx).is_some()
    })
    .await;
    request_code(shell, id, cx).unwrap()
}

/// The drafts the store lists, readable ones only, with how each is held.
fn stored(store: &DraftStore) -> Vec<(Draft, Held)> {
    store
        .list()
        .unwrap()
        .into_iter()
        .filter_map(|listing| match listing.entry {
            Entry::Draft(draft) => Some((draft, listing.held)),
            Entry::Unreadable(_) => None,
        })
        .collect()
}

fn show(shell: &Shell, root: &Path, id: &str) -> Option<ItemDto> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).ok()
}

/// The file at `path` in the tree of `branch`'s tip in `root`.
fn committed(root: &Path, branch: &str, path: &str) -> Option<String> {
    let repository = Repository::open(root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .ok()?
        .peel_to_commit()
        .ok()?;
    let entry = tip.tree().ok()?.get_path(Path::new(path)).ok()?;
    let blob = repository.find_blob(entry.id()).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

/// Sets `root`'s own `manyhands.initials`, as `git config` would.
fn set_initials(root: &Path, initials: &str) {
    let repository = Repository::open(root).unwrap();
    let mut config = repository
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    config.set_str("manyhands.initials", initials).unwrap();
}

// ---- the tests ----

/// R-20, obligation 1: New document… takes a new ID, writes the new item's
/// draft, and only then sends `context prepare` with the Create intent, of
/// that ID: as the prepare reaches the library, the store already holds the
/// draft, held here, over a new item's base at the checked path. The dialog
/// has closed and the tab is open; once the prepare lands, the edit editor
/// opens on the draft.
#[gpui_kit::test]
async fn new_document_prepares_and_opens_a_draft_already_in_the_store(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, _| {});
    let prepare = shell.backend.hold_job(JobKind::ContextPrepare);
    new_document(&shell, "Plans", "plans.md", cx).await;
    assert_eq!(value(&shell, "folder", cx), "docs", "the default folder");
    assert!(shell.fixture.draft_store().list().unwrap().is_empty());

    // R-20's first step: the new ID. Until it lands, nothing is stored or
    // sent, and the dialog waits, Escape included.
    let new_id = shell.backend.hold_read(ReadKind::NewId);
    shell.click("create-ok", cx);
    assert!(shell.fixture.draft_store().list().unwrap().is_empty());
    assert!(prepares(&shell, cx).is_empty(), "nothing before the new ID");
    assert!(!offered(&shell, cx), "one create at a time");
    shell.keys("escape", cx);
    assert!(dialog_open(&shell, cx), "Escape waits for the create");
    new_id.send(()).unwrap();
    until(&shell, cx, "the prepare", |shell, cx| {
        !prepares(shell, cx).is_empty()
    })
    .await;
    prepare.wait_reached();

    let id = opened_id(&shell, cx);
    let drafts = stored(&shell.fixture.draft_store());
    assert_eq!(drafts.len(), 1, "one draft, stored before the prepare ran");
    let (draft, held) = &drafts[0];
    assert_eq!(*held, Held::Here);
    assert_eq!(draft.target.item_kind, ItemKind::Document);
    assert_eq!(draft.target.item_id.to_string(), id, "the tab's item");
    assert_eq!(draft.target.repository, key(&root));
    let base = draft.base.as_ref().expect("a base");
    assert!(is_new_item_base(base), "a new item's base");
    assert_eq!(base.path, "docs/plans.md");
    assert_eq!(base.context.branch.as_deref(), Some("main"));
    assert!(matches!(
        &draft.current,
        manyhands::drafts::DraftFields::Document { title, body } if title == "Plans" && body.is_empty()
    ));
    let sent = prepares(&shell, cx);
    assert_eq!(sent.len(), 1, "one prepare");
    assert!(sent[0].0, "with the Create intent");
    assert!(matches!(
        intent(&shell, sent[0].1, cx),
        Intent::ContextPrepare { kind: manyhands::repository::AuthoringKind::Document, id: ref of, .. }
            if of.to_string() == id
    ));
    assert!(!dialog_open(&shell, cx), "the dialog closed");
    assert_eq!(phase(&shell, cx), Some(Phase::Preparing(sent[0].1)));

    prepare.release();
    let host = until_editing(&shell, cx).await;
    assert_eq!(request_code(&shell, sent[0].1, cx), Some(ResultCode::Ok));
    assert_eq!(host.read_with(cx, |host, cx| host.text(cx)), "");
    assert!(show(&shell, &root, &id).is_none(), "not created yet");
    assert!(
        creates(&shell, cx).is_empty(),
        "nothing created before Save"
    );
}

/// R-22, obligation 3: the path is checked here first (under `docs`, ending
/// in `.md`, plain), then with `observe_path`, which must find nothing; an
/// answer for an older name never offers Create.
#[gpui_kit::test]
async fn the_path_must_be_under_docs_end_md_and_be_free(cx: &mut TestAppContext) {
    let (shell, _root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, "docs/held.md", b"Another file\n");
    });
    shell.click("new-document-button", cx);
    // Obligation 2: Create waits for the Title, in Task 20's words.
    let untitled = "A document needs a title before it can be saved.";
    assert_eq!(
        shell.label("create-title-note", cx).as_deref(),
        Some(untitled)
    );
    assert_eq!(create_note(&shell, cx).as_deref(), Some(untitled));
    assert!(!offered(&shell, cx));
    shell.visual(cx).simulate_input("Title");
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("create-title-note", cx));
    let not_md = "A document's file name must end in .md.";
    let not_docs = "A document's path must be under docs.";
    assert_eq!(shell.label("create-file-note", cx).as_deref(), Some(not_md));
    assert_eq!(create_note(&shell, cx).as_deref(), Some(not_md));
    assert!(!offered(&shell, cx));

    retype(&shell, "file", "notes.txt", cx);
    assert_eq!(shell.label("create-file-note", cx).as_deref(), Some(not_md));
    retype(&shell, "file", "notes.md", cx);
    for folder in [
        "elsewhere",
        "docs/../elsewhere",
        "/docs",
        "docs//notes",
        "./docs",
    ] {
        retype(&shell, "folder", folder, cx);
        assert_eq!(
            shell.label("create-file-note", cx).as_deref(),
            Some(not_docs),
            "{folder}"
        );
        assert!(!offered(&shell, cx), "{folder}");
    }
    retype(&shell, "folder", "docs", cx);
    retype(&shell, "file", "../notes.md", cx);
    assert_eq!(
        shell.label("create-file-note", cx).as_deref(),
        Some(not_docs)
    );

    // The check of `free.md` is held where it reaches the library; a newer
    // name is checked meanwhile.
    let late = shell.backend.hold_read(ReadKind::ObservePath);
    retype(&shell, "file", "free.md", cx);
    assert!(!offered(&shell, cx), "not offered while it is checked");
    assert!(!shell.present("create-file-note", cx), "nothing said yet");
    retype(&shell, "file", "held.md", cx);
    until(&shell, cx, "the newer check", |shell, cx| {
        shell.label("create-file-note", cx).is_some()
    })
    .await;
    let held = "Another file is already at docs/held.md.";
    assert_eq!(shell.label("create-file-note", cx).as_deref(), Some(held));
    assert_eq!(create_note(&shell, cx).as_deref(), Some(held));
    // The older check was dropped with its gate; were its answer to land,
    // its stamp is not the latest.
    let _ = late.send(());
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(
        !offered(&shell, cx),
        "the older name's answer offers nothing"
    );
    assert_eq!(shell.label("create-file-note", cx).as_deref(), Some(held));
    shell.click("create-ok", cx);
    assert!(prepares(&shell, cx).is_empty(), "Create made nothing");
    assert!(dialog_open(&shell, cx));

    retype(&shell, "folder", "docs/plans/", cx);
    retype(&shell, "file", "free.md", cx);
    until(&shell, cx, "the free path's check", |shell, cx| {
        offered(shell, cx)
    })
    .await;
    assert!(!shell.present("create-file-note", cx));
    assert!(create_note(&shell, cx).is_none(), "Create is offered");
}

/// Obligation 7: Save creates the document once; a second Save is an
/// ordinary document save; and the tab goes on as an ordinary edit session
/// of the item, with the same editor.
#[gpui_kit::test]
async fn save_creates_the_document_once_and_the_tab_becomes_an_edit_session(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |_, _| {});
    new_document(&shell, "Plans", "plans.md", cx).await;
    shell.click("create-ok", cx);
    let host = until_editing(&shell, cx).await;
    let id = opened_id(&shell, cx);
    type_at_end(&shell, &host, "First", cx);

    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1, "Save sent one create");
    assert!(matches!(
        intent(&shell, sent[0], cx),
        Intent::DocumentCreate { ref path, ref title, ref body, .. }
            if path == Path::new("docs/plans.md") && title == "Plans" && body == "First"
    ));
    assert_eq!(answered(&shell, sent[0], cx).await, ResultCode::Ok);
    until(&shell, cx, "the draft retiring", |shell, _| {
        shell.fixture.draft_store().list().unwrap().is_empty()
    })
    .await;
    // The item as read: a draft's header has no Updated chip.
    until(&shell, cx, "the item read", |shell, cx| {
        shell.label("item-title", cx).as_deref() == Some("Plans")
            && shell.present(ChipField::Updated.id(), cx)
    })
    .await;
    let shown = show(&shell, &root, &id).expect("the document is created");
    assert_eq!(shown.path, "docs/plans.md");
    assert_eq!(shown.body.as_deref(), Some("First"));
    let branch = shown.context.branch.clone().expect("its branch");
    assert!(
        committed(&root, &branch, "docs/plans.md").is_some(),
        "committed"
    );
    assert_eq!(
        phase(&shell, cx),
        Some(Phase::Editing),
        "the session goes on"
    );

    type_at_end(&shell, &host, " and more", cx);
    shell.click("item-save", cx);
    let saves = document_saves(&shell, cx);
    assert_eq!(saves.len(), 1, "the second Save is a document save");
    assert!(matches!(
        intent(&shell, saves[0], cx),
        Intent::DocumentSave { ref body, ref observation, .. }
            if body == "First and more" && observation.is_some()
    ));
    assert_eq!(answered(&shell, saves[0], cx).await, ResultCode::Ok);
    assert_eq!(creates(&shell, cx).len(), 1, "created once");
    let same = pane(&shell, cx).read_with(cx, |pane, _| pane.edit_host());
    assert!(same.as_ref() == Some(&host), "the same session's editor");
    until(&shell, cx, "the saved read", |shell, _| {
        show(shell, &root, &id)
            .and_then(|item| item.body)
            .as_deref()
            == Some("First and more")
    })
    .await;
    assert_eq!(phase(&shell, cx), Some(Phase::Editing));
}

/// Obligations 6 and 9 (native checklist step 9): a new ticket, made by
/// keyboard in its dialog with its Title and Type, takes a dependency and a
/// parent in the sheet's picker; Save sends `TicketCreate` with the draft's
/// fields, `deps` and `parent`, status `open` and no initials (R-21); the
/// committed file holds them.
#[gpui_kit::test]
async fn new_ticket_creates_with_its_fields_and_dependency(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, DEPENDENCY, "Dependency ticket", "open");
        fixture.create_ticket(root, PARENT, "Parent ticket", "open");
    });
    shell.click("pane-tickets", cx);
    shell.keys("secondary-2", cx);
    shell.keys("secondary-n", cx);
    assert!(dialog_open(&shell, cx), "secondary-n opens New ticket…");
    assert!(holds_focus(&shell, "title", cx));
    assert_eq!(value(&shell, "type", cx), "task", "the default type");
    assert_eq!(
        create_note(&shell, cx).as_deref(),
        Some("A ticket needs a title before it can be saved.")
    );
    shell.visual(cx).simulate_input("Brand new");
    shell.run(cx);
    shell.keys("tab", cx);
    assert!(holds_focus(&shell, "type", cx), "Tab goes to the Type");
    // An emptied Type is required, in Task 20's words.
    shell.keys("secondary-a backspace", cx);
    let untyped = "A ticket needs a type before it can be saved.";
    assert_eq!(
        shell.label("create-type-note", cx).as_deref(),
        Some(untyped)
    );
    assert!(!offered(&shell, cx));
    shell.keys("enter", cx);
    assert!(prepares(&shell, cx).is_empty(), "Enter creates nothing yet");
    shell.visual(cx).simulate_input("bug");
    shell.run(cx);
    assert!(offered(&shell, cx));
    shell.keys("enter", cx);
    let host = until_editing(&shell, cx).await;
    let id = opened_id(&shell, cx);
    assert!(!dialog_open(&shell, cx));
    let _ = host;

    // The sheet's picker chooses the dependency and the parent.
    shell.click(ChipField::All.id(), cx);
    shell.frame(cx, |_, _| ());
    for (opener, filter) in [
        ("rel-deps-add", "Dependency"),
        ("rel-parent-change", "Parent"),
    ] {
        shell.click(opener, cx);
        shell.frame(cx, |_, _| ());
        assert!(shell.present("rel-picker", cx), "{opener} opens the picker");
        shell.visual(cx).simulate_input(filter);
        shell.run(cx);
        until(&shell, cx, "the offer", |shell, cx| {
            shell.present(("rel-pick", 0usize), cx)
        })
        .await;
        shell.click(("rel-pick", 0usize), cx);
        shell.frame(cx, |_, _| ());
    }
    shell.keys("secondary-s", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1, "Save sent one create");
    assert!(matches!(
        intent(&shell, sent[0], cx),
        Intent::TicketCreate {
            ref title, ref ticket_type, ref status, ref project, ref team, ref deps, ref parent, ..
        } if title == "Brand new"
            && ticket_type == "bug"
            && status == "open"
            && project.is_none()
            && team.is_none()
            && deps == &[DEPENDENCY.to_owned()]
            && parent.as_deref() == Some(PARENT)
    ));
    assert_eq!(answered(&shell, sent[0], cx).await, ResultCode::Ok);

    let shown = show(&shell, &root, &id).expect("the ticket is created");
    assert_eq!(shown.title.as_deref(), Some("Brand new"));
    assert_eq!(shown.ticket_type.as_deref(), Some("bug"));
    assert_eq!(shown.status.as_deref(), Some("open"));
    let deps: Vec<&str> = shown.deps.iter().map(|dep| dep.id.as_str()).collect();
    assert_eq!(deps, [DEPENDENCY]);
    assert_eq!(
        shown.parent.as_ref().map(|parent| parent.id.as_str()),
        Some(PARENT)
    );
    let branch = shown.context.branch.clone().expect("its branch");
    let file = committed(
        &root,
        &branch,
        &format!(".manyhands/tickets/{id}/ticket.md"),
    )
    .expect("the ticket's file is committed");
    assert!(file.contains("Brand new") && file.contains("bug"));
    assert!(file.contains(DEPENDENCY) && file.contains(PARENT));
}

/// The design's Create: the new tab opens editing, with its header built
/// from the draft's fields: its title, pill and chips, and its strip title,
/// while its Create prepare runs and once the editor is open; D1's
/// not-found line and D2's restored section are not shown.
#[gpui_kit::test]
async fn the_new_tab_opens_editing_with_its_header_from_the_draft(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, _| {});
    let prepare = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("pane-tickets", cx);
    shell.click("new-ticket-button", cx);
    shell.visual(cx).simulate_input("Header title");
    shell.run(cx);
    retype(&shell, "type", "feature", cx);
    shell.click("create-ok", cx);
    until(&shell, cx, "the prepare", |shell, cx| {
        !prepares(shell, cx).is_empty()
    })
    .await;
    prepare.wait_reached();
    let id = opened_id(&shell, cx);

    let header = |shell: &Shell, cx: &mut TestAppContext| {
        (
            shell.label("item-title", cx),
            shell.label(ChipField::Type.id(), cx),
            shell.label(ChipField::Status.id(), cx),
            shell.present(ChipField::All.id(), cx),
        )
    };
    let expected = (
        Some("Header title".to_owned()),
        Some("Type feature".to_owned()),
        Some("Status open".to_owned()),
        true,
    );
    assert_eq!(header(&shell, cx), expected, "the header, while preparing");
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Getting a private copy ready\u{2026}")
    );
    assert_eq!(shell.rows("tab", cx), ["Header title, unsaved"]);
    assert_eq!(
        shell.label("item-breadcrumb", cx),
        Some(format!("a \u{203a} Tickets \u{203a} {id}"))
    );
    assert!(!shell.present("item-failure", cx), "not said to be missing");
    assert!(!shell.present("draft-section", cx), "no restored section");
    assert!(!shell.present("draft-continue", cx));

    prepare.release();
    until_editing(&shell, cx).await;
    assert_eq!(header(&shell, cx), expected, "the header, while editing");
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Editing \u{b7} unsaved changes")
    );
    assert_eq!(shell.label("item-save", cx).as_deref(), Some("Save"));
    assert!(!shell.present("item-failure", cx));
    // The edit editor is the draft section now (R-38); D2's is not shown.
    assert!(!shell.present("draft-continue", cx));
    assert!(show(&shell, &root, &id).is_none(), "not created yet");

    // The sheet shows the draft's fields too.
    shell.click(ChipField::All.id(), cx);
    shell.frame(cx, |_, _| ());
    assert!(
        shell.present("sheet-title", cx),
        "the sheet opens on the draft"
    );
}

/// R-22 at Save: another file reached the path after the dialog checked it;
/// the create is refused, the draft is kept, and the tab says so.
#[gpui_kit::test]
async fn an_occupied_path_at_save_keeps_the_draft(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, _| {});
    new_document(&shell, "Taken", "taken.md", cx).await;
    shell.click("create-ok", cx);
    let host = until_editing(&shell, cx).await;
    type_at_end(&shell, &host, "Mine", cx);
    shell
        .fixture
        .commit_file(&root, "docs/taken.md", b"Another file\n");

    // The registry reads the path for whether this draft's own item is
    // there (Task 22); until it has, the tab names no other file.
    let check = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1);
    assert_eq!(
        answered(&shell, sent[0], cx).await,
        ResultCode::OccupiedPath
    );
    shell.frame(cx, |_, _| ());
    assert!(
        !shell.present("item-save-refused", cx),
        "nothing said while the path is read"
    );
    check.send(()).unwrap();
    let kept = "Another file is already at docs/taken.md. The draft is kept.";
    until(&shell, cx, "the words", |shell, cx| {
        shell.label("item-save-refused", cx).as_deref() == Some(kept)
    })
    .await;
    assert_eq!(phase(&shell, cx), Some(Phase::Editing), "no review");
    let drafts = stored(&shell.fixture.draft_store());
    assert_eq!(drafts.len(), 1, "the draft is kept");
    let (draft, held) = &drafts[0];
    assert_eq!(*held, Held::Here);
    assert!(
        draft.base.as_ref().is_some_and(is_new_item_base),
        "still new"
    );
    assert!(matches!(
        &draft.current,
        manyhands::drafts::DraftFields::Document { body, .. } if body == "Mine"
    ));
    assert_eq!(host.read_with(cx, |host, cx| host.text(cx)), "Mine");
}

/// R-21, obligation 4: a repository whose initials do not fit refuses the
/// create `initials_required`; the tab says the repository needs initials,
/// keeps the draft, and its Open Settings opens Settings on the repository.
/// Once initials are set, Save creates the ticket.
#[gpui_kit::test]
async fn initials_required_points_to_settings(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, root| set_initials(root, "toolong"));
    shell.click("pane-tickets", cx);
    shell.click("new-ticket-button", cx);
    shell.visual(cx).simulate_input("Needs initials");
    shell.run(cx);
    shell.click("create-ok", cx);
    until_editing(&shell, cx).await;
    let id = opened_id(&shell, cx);

    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(
        answered(&shell, sent[0], cx).await,
        ResultCode::InitialsRequired
    );
    until(&shell, cx, "the words", |shell, cx| {
        shell.label("item-save-refused", cx).as_deref()
            == Some("This repository needs initials before it can create tickets.")
    })
    .await;
    assert!(matches!(
        intent(&shell, sent[0], cx),
        Intent::TicketCreate { ref status, .. } if status == "open"
    ));
    let drafts = stored(&shell.fixture.draft_store());
    assert_eq!(drafts.len(), 1, "the draft is kept");
    assert!(drafts[0].0.base.as_ref().is_some_and(is_new_item_base));
    assert_eq!(phase(&shell, cx), Some(Phase::Editing));
    assert!(show(&shell, &root, &id).is_none(), "nothing created");

    shell.click("item-save-settings", cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx)),
        Shown::Settings(SettingsPage::Repositories)
    );
    assert_eq!(shell.label("managed-root", cx), Some(key(&root)));

    set_initials(&root, "ae");
    shell.click("settings-close-button", cx);
    until(&shell, cx, "the tab again", |shell, cx| {
        shell.present("item-save", cx)
    })
    .await;
    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 2, "Save sends the create again");
    assert_eq!(answered(&shell, sent[0], cx).await, ResultCode::Ok);
    let shown = show(&shell, &root, &id).expect("the ticket is created");
    assert_eq!(shown.title.as_deref(), Some("Needs initials"));
    assert!(
        shown
            .slug
            .as_deref()
            .is_some_and(|slug| slug.contains("ae")),
        "its short code takes the initials"
    );
}

/// Obligation 5: `secondary-n` in the navigation region opens the shown
/// list's dialog, New document… on Documents and New ticket… on Tickets,
/// with focus in its Title; Escape closes it and focus goes back where it
/// was. It opens nothing in front of a dialog, from the content region, or
/// from the details sheet.
#[gpui_kit::test]
async fn secondary_n_opens_the_shown_lists_dialog(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, DEPENDENCY, "Listed", "open");
    });
    let focused =
        |shell: &Shell, cx: &mut TestAppContext| shell.frame(cx, |window, cx| window.focused(cx));
    let kind = |shell: &Shell, cx: &mut TestAppContext| {
        dialog(shell, cx).read_with(cx, |dialog, _| dialog.kind())
    };

    shell.keys("secondary-2", cx);
    let tree = focused(&shell, cx).expect("the document tree holds focus");
    shell.keys("secondary-n", cx);
    assert!(dialog_open(&shell, cx), "New document…");
    assert_eq!(kind(&shell, cx), ItemDtoKind::Document);
    assert!(shell.present("create-folder", cx) && shell.present("create-file", cx));
    assert!(holds_focus(&shell, "title", cx));
    shell.keys("secondary-n", cx);
    shell.keys("escape", cx);
    assert!(!dialog_open(&shell, cx), "one dialog, closed by Escape");
    assert_eq!(focused(&shell, cx), Some(tree), "focus goes back");

    shell.click("pane-tickets", cx);
    shell.keys("secondary-2", cx);
    let list = focused(&shell, cx).expect("the ticket list holds focus");
    shell.keys("secondary-n", cx);
    assert!(dialog_open(&shell, cx), "New ticket…");
    assert_eq!(kind(&shell, cx), ItemDtoKind::Ticket);
    assert!(shell.present("create-type", cx));
    assert!(holds_focus(&shell, "title", cx));
    shell.keys("escape", cx);
    assert!(!dialog_open(&shell, cx));
    assert_eq!(focused(&shell, cx), Some(list), "focus goes back");

    // Not from the content region, nor from the details sheet.
    shell.workspace.update(cx, |workspace, cx| {
        workspace.open_item(
            OpenTarget::Item {
                repo: key(&root),
                kind: ItemDtoKind::Ticket,
                id: item_id(DEPENDENCY),
            },
            cx,
        )
    });
    shell.run(cx);
    shell.keys("secondary-3", cx);
    shell.keys("secondary-n", cx);
    assert!(!dialog_open(&shell, cx), "nothing from the content region");
    shell.click(ChipField::All.id(), cx);
    assert!(shell.frame(cx, |window, cx| {
        gpui_kit::component::WindowExt::has_active_sheet(window, cx)
    }));
    shell.keys("secondary-n", cx);
    assert!(!dialog_open(&shell, cx), "nothing from the sheet");
}

/// R-46, obligation 1: with no draft store, Create writes nothing and
/// provisions nothing; the dialog stays open, in Task 22's words.
#[cfg(unix)]
#[gpui_kit::test]
async fn create_without_a_draft_store_says_why_and_stays_open(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.click("pane-tickets", cx);
    shell.click("new-ticket-button", cx);
    shell.visual(cx).simulate_input("No store");
    shell.run(cx);
    shell.click("create-ok", cx);
    until(&shell, cx, "the words", |shell, cx| {
        shell.label("create-failure", cx).is_some()
    })
    .await;
    assert_eq!(
        shell.label("create-failure", cx).as_deref(),
        Some("Crash recovery is off, so a new item cannot be created.")
    );
    assert!(dialog_open(&shell, cx), "the dialog stays open");
    assert!(prepares(&shell, cx).is_empty(), "nothing is provisioned");
    assert!(opened(&shell, cx).is_none(), "no tab opens");
    assert!(
        !root.join(".manyhands/worktrees").exists()
            || std::fs::read_dir(root.join(".manyhands/worktrees"))
                .unwrap()
                .next()
                .is_none(),
        "no context is made"
    );
    assert!(offered(&shell, cx), "Create may be pressed again");
}

/// The tab hook (`edit_created`) follows a Create prepare whatever its
/// answer's timing: one already answered when the tab opens, its answer
/// having reached the request model before the dialog's own task did,
/// still opens the editor on the new item's draft.
#[gpui_kit::test]
async fn a_create_prepare_answered_before_its_tab_opens_still_opens_the_editor(
    cx: &mut TestAppContext,
) {
    use crate::desktop::model::drafts::create::{new_item_draft, ticket_fields, ticket_path};

    let (shell, root) = shell(cx, |_, _| {});
    let id = item_id("01ARZ3NDEKTSV4RRFFQ69G5FH4");
    let fields = ticket_fields("Early", "task");
    let (target, base) = new_item_draft(
        &key(&root),
        Some("main".to_owned()),
        ItemKind::Ticket,
        id.clone(),
        ticket_path(&id),
        &fields,
    );
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    let made = drafts.update(cx, |registry, cx| {
        registry.new_item(target, base, fields, cx)
    });
    shell.run(cx);
    let Ok(made) = made.await else {
        panic!("the new item's draft must be written");
    };
    shell.until_ended(made.prepare, cx).await;
    assert_eq!(request_code(&shell, made.prepare, cx), Some(ResultCode::Ok));

    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id,
    };
    shell.visual(cx).update(|window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.open_created(target, made, window, cx)
        })
    });
    let host = until_editing(&shell, cx).await;
    assert_eq!(host.read_with(cx, |host, cx| host.text(cx)), "");
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Early"));
}
