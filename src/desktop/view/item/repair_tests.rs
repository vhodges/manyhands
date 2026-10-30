//! Repair, over real repositories (design, Part E: Repair; R-25; Cycle
//! contract 11; obligations 1 to 11 of the Task 25 notes): "Needs fixing"
//! and an Inspect tab offer Repair…, and an unmanaged document's banner
//! Adopt…; the repair view is a plain text area over the whole file,
//! exactly as read, that no markdown editor or draft store touches; Preview
//! prepares the repair in the one confirmation dialog, which names Root,
//! Path and ID and says when a new ID will be assigned; a stale preview, an
//! ambiguous identity and a lost answer are each explained, the text kept;
//! a repair made is followed by the tab, by the item's ID; and closing,
//! cancelling or quitting with edited text asks first.
//!
//! Every test waits on the worker, so it allows parking. Typing goes
//! through the window, into the text area holding focus. The ruled words
//! are literals: the tests fix them.

use std::path::{Path, PathBuf};

use git2::Repository;
use gpui_kit::{
    ElementId, Entity, Focusable, TestAppContext,
    component::input::{Position, TextareaState},
    test::TestWindowExt,
};
use manyhands::{
    canonical::{self, CanonicalItem, Document, ItemId},
    repository::{
        DocumentRepairInput, FailurePoint, ItemDto, ItemDtoKind, Mutation, RepositoryService,
    },
    results::{ProblemCode, ResultCode},
};

use super::ItemPane;
use crate::desktop::{
    model::{editor_text::load, navigation::OpenTarget, tabs::TabKey},
    services::{
        JobKind, ReadFailure, ReadKind, ReadResult,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::{
        fixtures::{Fixture, item_id, ticket_path},
        snapshot::RepositorySnapshot,
    },
    view::harness::{Shell, key},
};

/// A ticket's directory, whose file does not parse.
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";
/// The ID a document's repair gives it.
const ADOPTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS1";
/// Another document's.
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS2";
/// A ticket that needs no fixing.
const READABLE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS3";
const MARKER: &str = "docs/marker.md";
const NOTES: &str = "docs/notes.md";
/// `commit_nonconforming_document`'s file.
const MARKER_ONLY: &str = "---\nmanyhands_managed: true\n---\nOnly a marker.\n";
/// What completes the marker-only document's front matter, typed before its
/// closing line.
const COMPLETION: &str =
    "manyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FS1\ntitle: Adopted\n";

const REPAIR: &str = "Repair\u{2026}";
const ADOPT: &str = "Adopt\u{2026}";
const NEW_ID: &str = "A new ID will be assigned";
const NOT_UTF8: &str = "This file is not UTF-8 text, so it cannot be repaired here.";
const DISCARD_TITLE: &str = "Discard your repair?";
const DISCARD_BODY: &str = "Your changes to this file are not saved anywhere.";
const UNMANAGED: &str = "This file isn't a managed document yet. Adopting it adds a title and \
                         an ID at the top; the rest stays exactly as written.";

// ---- the window ----

/// One repository, `a`, built by `build` and refreshed, in an open window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

/// The file at `path`, read by its path on primary.
fn inspect(root: &Path, path: &str) -> OpenTarget {
    OpenTarget::Inspect {
        repo: key(root),
        worktree: key(root),
        path: path.to_owned(),
    }
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
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

/// Whether the element `id` is drawn inside `within`.
fn inside(
    shell: &Shell,
    id: impl Into<ElementId>,
    within: &'static str,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.path().contains(&ElementId::from(within)))
    })
}

/// Whether the element `id` holds focus.
fn focused(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// The repair view's text area, while it is open.
fn text_area(shell: &Shell, cx: &mut TestAppContext) -> Option<Entity<TextareaState>> {
    shell.frame(cx, |_, _| ());
    pane(shell, cx).read_with(cx, |pane, _| pane.repair_state())
}

/// The repair view's text, as it holds it now.
fn repair_text(shell: &Shell, cx: &mut TestAppContext) -> String {
    text_area(shell, cx)
        .expect("the repair view is open")
        .read_with(cx, |state, _| state.value().to_string())
}

/// Whether the repair view's text area holds focus.
fn text_focused(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let state = text_area(shell, cx).expect("the repair view is open");
    let handle = state.read_with(cx, |state, cx| state.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Types `typed` at the start of line `line` of the repair text: the caret
/// set there, the text area focused and a frame drawn first.
fn type_at_line(shell: &Shell, line: u32, typed: &str, cx: &mut TestAppContext) {
    let state = text_area(shell, cx).expect("the repair view is open");
    shell.frame(cx, |window, cx| {
        state.update(cx, |state, cx| {
            state.set_cursor_position(Position::new(line, 0), window, cx)
        })
    });
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
}

/// Opens the repair view from the tab's Repair… (or Adopt…).
fn open_repair(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("item-repair-button", cx);
    assert!(shell.present("repair-source", cx), "the repair view opens");
}

/// Waits for the confirmation's answer to its prepare: a preview with
/// Confirm, or a refusal with its code.
async fn previewed(shell: &Shell, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, "the preview", |shell, cx| {
            (shell.present("confirm-ok", cx) && shell.present("confirm-expires", cx))
                || shell.present("confirm-code", cx)
        })
        .await;
}

/// Presses Preview and waits for the preview.
async fn preview(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("repair-preview-button", cx);
    previewed(shell, cx).await;
}

/// Presses Confirm and waits for the repair's first answer. Gives the
/// request.
async fn confirm(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    shell.click("confirm-ok", cx);
    shell
        .until_drawn(cx, "the repair's answer", |shell, cx| {
            repairs(shell, cx)
                .first()
                .is_some_and(|sent| answer(shell, *sent, cx).1.is_some())
        })
        .await;
    repairs(shell, cx)[0]
}

/// Waits until the tab follows the item `id` and shows it.
async fn follows(
    shell: &Shell,
    root: &Path,
    kind: ItemDtoKind,
    id: &str,
    title: &str,
    cx: &mut TestAppContext,
) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind,
        id: item_id(id),
    };
    shell
        .until_drawn(cx, "the tab following the repaired item", |shell, cx| {
            shell
                .workspace
                .read_with(cx, |workspace, cx| workspace.opened(cx))
                == Some(target.clone())
                && shell.label("item-title", cx).as_deref() == Some(title)
        })
        .await;
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys()),
        [TabKey::for_target(&target)],
        "the same tab, now the item's"
    );
    assert!(!shell.present("item-needs-fixing", cx), "it conforms");
    assert!(!shell.present("repair-source", cx));
}

// ---- the requests and the repository ----

/// This session's repairs, newest first.
fn repairs(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::DocumentRepair { .. } | Intent::TicketRepair { .. }
        )
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

/// How many commits `branch` of `root` has that `main` has not; `None`
/// when there is no such branch.
fn own_commits(root: &Path, branch: &str) -> Option<usize> {
    let repository = Repository::open(root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .ok()?
        .peel_to_commit()
        .unwrap();
    let main = repository
        .revparse_single("refs/heads/main")
        .unwrap()
        .peel_to_commit()
        .unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push(tip.id()).unwrap();
    walk.hide(main.id()).unwrap();
    Some(walk.count())
}

fn document_branch(id: &str) -> String {
    format!("manyhands/document/{id}")
}

fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

fn on_primary(root: &Path, path: &str) -> String {
    std::fs::read_to_string(root.join(path)).unwrap()
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

/// Opens the marker-only document's repair view, and completes its front
/// matter with `ADOPTED`. Gives the text.
fn completed_marker(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> String {
    open(shell, inspect(root, MARKER), cx);
    open_repair(shell, cx);
    type_at_line(shell, 2, COMPLETION, cx);
    let typed = repair_text(shell, cx);
    assert_eq!(
        typed,
        format!("---\nmanyhands_managed: true\n{COMPLETION}---\nOnly a marker.\n")
    );
    typed
}

/// The canonical document front matter with `id` and `title`, then `body`,
/// as the library writes it.
fn adoption(id: &ItemId, title: &str, body: &str) -> String {
    canonical::serialize_item(&CanonicalItem::Document(Document {
        id: id.clone(),
        title: title.to_owned(),
        body: body.to_owned(),
        unknown: serde_yaml::Mapping::new(),
    }))
    .unwrap()
}

/// Presses Adopt… and waits for the repair view, which opens once its ID
/// is made. Gives its text and the ID that text holds.
async fn adopt(shell: &Shell, cx: &mut TestAppContext) -> (String, ItemId) {
    shell.click("item-repair-button", cx);
    shell
        .until_drawn(cx, "the adoption's view", |shell, cx| {
            shell.present("repair-source", cx)
        })
        .await;
    let text = repair_text(shell, cx);
    let id = match canonical::parse_item(Path::new(NOTES), &text) {
        Ok(CanonicalItem::Document(document)) => document.id,
        _ => panic!("the prefill is a conforming document"),
    };
    (text, id)
}

/// A ticket's file with no type, which does not parse as a ticket: it
/// holds its directory's ID.
fn untyped() -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {BROKEN}\ntitle: Untyped\n\
         status: open\n---\nBody.\n"
    )
}

// ---- the tests ----

/// "Needs fixing" offers Repair…, by keyboard too: Tab reaches it from the
/// tab strip and Enter opens the repair view, its text area holding the
/// whole file exactly as read, with focus in it; the entry goes while the
/// view is open. An Inspect tab of a file that needs no fixing offers no
/// Repair… (fix 1, M3): the library repairs no conforming file. Opening
/// writes nothing.
#[gpui_kit::test]
async fn the_needs_fixing_banner_offers_repair(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_ticket(root, BROKEN);
        fixture.commit_ticket(root, READABLE, "Readable", "");
    });
    open(&shell, inspect(&root, &ticket_path(BROKEN)), cx);
    let before = RepositorySnapshot::take(&root);
    assert_eq!(
        shell.rows("item-problem", cx),
        [ProblemCode::MalformedFrontMatter.as_str()]
    );
    assert_eq!(shell.label("item-repair", cx).as_deref(), Some(REPAIR));
    assert!(inside(&shell, "item-repair", "item-banners", cx));

    shell.keys("secondary-3", cx);
    let mut presses = 0;
    while !focused(&shell, "item-repair-button", cx) {
        presses += 1;
        assert!(presses < 40, "Tab never reached Repair…");
        shell.keys("tab", cx);
    }
    shell.press("enter", cx);
    assert!(shell.present("repair-source", cx), "Enter opens the view");
    assert_eq!(
        repair_text(&shell, cx),
        on_primary(&root, &ticket_path(BROKEN)),
        "the whole file, exactly as read"
    );
    assert!(text_focused(&shell, cx), "focus is in the text area");
    assert!(
        !shell.present("item-repair", cx),
        "the entry goes meanwhile"
    );
    assert_eq!(RepositorySnapshot::take(&root), before);

    open(&shell, inspect(&root, &ticket_path(READABLE)), cx);
    assert!(shell.present("item-source", cx), "the file is read");
    assert!(!shell.present("item-needs-fixing", cx));
    assert!(!shell.present("item-repair", cx), "no Repair… for it");
    assert_eq!(RepositorySnapshot::take(&root), before);
}

/// An unmanaged document, a file under `docs/` with no front matter, has a
/// banner of its own, in the handoff's words, with Adopt…, and no "Needs
/// fixing". Adopt… opens the repair view once an ID is made for it, on the
/// canonical document front matter, with that ID and the file's first
/// heading as its title, then the file byte for byte (fix 1, I1). The
/// preview names the made ID and says a new ID will be assigned. Nothing
/// is typed.
#[gpui_kit::test]
async fn an_unmanaged_document_offers_adopt(cx: &mut TestAppContext) {
    let written = "Intro.\n\n## Notes ##\n\nWritten by hand.\r\n";
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, NOTES, written.as_bytes());
    });
    open(&shell, inspect(&root, NOTES), cx);
    assert_eq!(
        shell.label("item-unmanaged", cx).as_deref(),
        Some(UNMANAGED)
    );
    assert!(inside(&shell, "item-unmanaged", "item-banners", cx));
    assert!(!shell.present("item-needs-fixing", cx));
    assert_eq!(shell.label("item-repair", cx).as_deref(), Some(ADOPT));
    assert!(inside(&shell, "item-repair", "item-banners", cx));

    let (prefilled, id) = adopt(&shell, cx).await;
    assert_eq!(
        prefilled,
        adoption(&id, "Notes", written),
        "the canonical front matter, then the file byte for byte"
    );
    assert!(prefilled.ends_with(written));
    preview(&shell, cx).await;
    let details = shell.rows("confirm-detail", cx);
    assert!(details[0].starts_with("Root: "), "{details:?}");
    assert_eq!(
        details[1..],
        [format!("Path: {NOTES}"), format!("ID: {id}")]
    );
    assert_eq!(shell.label("confirm-adoption", cx).as_deref(), Some(NEW_ID));
}

/// Adopt… makes its ID once for a read of the file: Cancel, unasked since
/// nothing was edited, then Adopt… again, with the file unchanged, shows
/// the same ID, so the file is never adopted twice as two items. A file
/// with no heading takes its file stem as its title.
#[gpui_kit::test]
async fn adopt_again_on_the_same_read_shows_the_same_id(cx: &mut TestAppContext) {
    let written = "No heading here.\n";
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, NOTES, written.as_bytes());
    });
    open(&shell, inspect(&root, NOTES), cx);
    let (first, id) = adopt(&shell, cx).await;
    assert_eq!(first, adoption(&id, "notes", written));
    shell.click("repair-cancel-button", cx);
    assert!(
        !shell.present("repair-discard-dialog", cx),
        "unedited, unasked"
    );
    assert!(!shell.present("repair-source", cx));

    let (again, same) = adopt(&shell, cx).await;
    assert_eq!(same, id, "the same ID");
    assert_eq!(again, first);
}

/// its front matter completed with a new ID, Tab reaches Preview and Enter
/// prepares it; the preview names Root, Path and that ID and says a new ID
/// will be assigned; Enter confirms. The repair is in the document's own
/// context, one commit on its branch, exactly the text typed, and primary
/// keeps the file as it was. The tab follows the document by its new ID and
/// shows it, conforming.
#[gpui_kit::test]
async fn a_nonconforming_document_is_repaired_after_a_preview_naming_the_new_id(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let typed = completed_marker(&shell, &root, cx);
    assert!(text_focused(&shell, cx));
    shell.keys("tab", cx);
    assert!(
        focused(&shell, "repair-preview-button", cx),
        "Tab reaches Preview"
    );
    shell.press("enter", cx);
    previewed(&shell, cx).await;
    let details = shell.rows("confirm-detail", cx);
    assert_eq!(
        details[1..],
        [format!("Path: {MARKER}"), format!("ID: {ADOPTED}")]
    );
    assert_eq!(shell.label("confirm-adoption", cx).as_deref(), Some(NEW_ID));
    assert!(repairs(&shell, cx).is_empty(), "a preview sends nothing");

    shell.keys("enter", cx);
    shell
        .until_drawn(cx, "the repair's answer", |shell, cx| {
            repairs(shell, cx)
                .first()
                .is_some_and(|sent| answer(shell, *sent, cx).1.is_some())
        })
        .await;
    let sent = repairs(&shell, cx)[0];
    assert_eq!(answer(&shell, sent, cx), (1, Some(ResultCode::Ok)));
    follows(&shell, &root, ItemDtoKind::Document, ADOPTED, "Adopted", cx).await;
    assert_eq!(own_commits(&root, &document_branch(ADOPTED)), Some(1));
    assert_eq!(
        shown(&shell, &root, ADOPTED).source.as_deref(),
        Some(typed.as_str())
    );
    assert_eq!(
        on_primary(&root, MARKER),
        MARKER_ONLY,
        "primary is not written"
    );
}

/// A ticket with no type does not parse as a ticket (design, Not Built:
/// "Choose type"); "Needs fixing" repairs it by adding its type. It keeps
/// the ID it holds, its directory's: the preview names it, with no new-ID
/// line. The ticket's own context holds the repair, one commit; the tab
/// follows the ticket and shows it with its type.
#[gpui_kit::test]
async fn a_nonconforming_ticket_is_repaired(cx: &mut TestAppContext) {
    let untyped = untyped();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, &ticket_path(BROKEN), untyped.as_bytes());
    });
    open(&shell, inspect(&root, &ticket_path(BROKEN)), cx);
    assert!(shell.present("item-needs-fixing", cx));
    open_repair(&shell, cx);
    assert_eq!(repair_text(&shell, cx), untyped);
    type_at_line(&shell, 5, "type: task\n", cx);
    preview(&shell, cx).await;
    let details = shell.rows("confirm-detail", cx);
    assert_eq!(
        details[1..],
        [
            format!("Path: {}", ticket_path(BROKEN)),
            format!("ID: {BROKEN}")
        ]
    );
    assert!(!shell.present("confirm-adoption", cx), "its ID is kept");

    let sent = confirm(&shell, cx).await;
    assert_eq!(answer(&shell, sent, cx), (1, Some(ResultCode::Ok)));
    follows(&shell, &root, ItemDtoKind::Ticket, BROKEN, "Untyped", cx).await;
    assert_eq!(
        own_commits(&root, &format!("manyhands/ticket/{BROKEN}")),
        Some(1)
    );
    assert_eq!(
        shown(&shell, &root, BROKEN).ticket_type.as_deref(),
        Some("task")
    );
}

/// A repaired item whose own tab is open already, found missing while the
/// file was not a ticket: that tab is shown, reading the item again, and
/// the repair's tab closes.
#[gpui_kit::test]
async fn a_repaired_item_with_its_tab_open_is_shown_there(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, &ticket_path(BROKEN), untyped().as_bytes());
    });
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(BROKEN),
    };
    open(&shell, target, cx);
    assert!(!shell.present("item-title", cx), "not a ticket yet");
    open(&shell, inspect(&root, &ticket_path(BROKEN)), cx);
    open_repair(&shell, cx);
    type_at_line(&shell, 5, "type: task\n", cx);
    preview(&shell, cx).await;
    let sent = confirm(&shell, cx).await;
    assert_eq!(answer(&shell, sent, cx), (1, Some(ResultCode::Ok)));
    follows(&shell, &root, ItemDtoKind::Ticket, BROKEN, "Untyped", cx).await;
}

/// A preview the file has changed under since: another actor commits the
/// file after the preview. Confirm is refused `external_change`. The
/// request carries the token of the read the text came from, so a fresh
/// preview of it would be refused the same way: no Prepare again is
/// offered (fix 1, M5). The dialog closes, and the view shows the
/// library's message and code and keeps the text, which can still be
/// edited; starting again from the file as it is now is Cancel, then
/// Repair… again. Nothing is written.
#[gpui_kit::test]
async fn a_preview_the_file_changed_under_offers_no_prepare_again(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let typed = completed_marker(&shell, &root, cx);
    preview(&shell, cx).await;
    assert!(shell.present("confirm-ok", cx));
    shell.fixture.commit_file(
        &root,
        MARKER,
        b"---\nmanyhands_managed: true\n---\nChanged by another.\n",
    );

    let sent = confirm(&shell, cx).await;
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::ExternalChange))
    );
    let message = shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(sent).unwrap().state {
            RequestState::Finished(_, _, _, _, message) => message.clone(),
            _ => panic!("answered"),
        }
    });
    assert!(!shell.present("confirm-dialog", cx), "the dialog closes");
    assert!(!shell.present("confirm-prepare-again", cx));
    assert_eq!(
        shell.label("repair-failure", cx).as_deref(),
        Some(message.as_str())
    );
    assert_eq!(
        shell.label("repair-failure-code", cx).as_deref(),
        Some(ResultCode::ExternalChange.as_str())
    );
    assert_eq!(own_commits(&root, &document_branch(ADOPTED)), None);
    assert_eq!(repair_text(&shell, cx), typed, "the text is kept");
    type_at_line(&shell, 0, "x", cx);
    assert_eq!(repair_text(&shell, cx), format!("x{typed}"), "and editable");
}

/// A corrected ID another document holds is an ambiguous identity: the
/// preview is refused with the library's message and code, no Confirm is
/// offered, and nothing is written. Escape closes the dialog, focus goes
/// back to Preview, which no repair running leaves enabled (Task 27 fix 1,
/// B5), and the text is kept.
#[gpui_kit::test]
async fn an_ambiguous_identity_blocks_repair_with_its_reason(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_document(root, OTHER, "docs/other.md", "Other");
        fixture.commit_nonconforming_document(root, MARKER);
    });
    open(&shell, inspect(&root, MARKER), cx);
    open_repair(&shell, cx);
    type_at_line(
        &shell,
        2,
        &format!("manyhands_kind: document\nid: {OTHER}\ntitle: Taken\n"),
        cx,
    );
    let typed = repair_text(&shell, cx);
    let before = RepositorySnapshot::take(&root);
    shell.keys("tab", cx);
    shell.press("enter", cx);
    previewed(&shell, cx).await;

    // The library's own answer to the same preview.
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let observation = service
        .show_path(&repo, None, Path::new(MARKER))
        .unwrap()
        .observation
        .unwrap();
    let theirs = service.prepare(&Mutation::DocumentRepair(DocumentRepairInput {
        root: root.clone(),
        path: PathBuf::from(MARKER),
        observation,
        source: typed.clone(),
    }));
    assert_eq!(theirs.code, ResultCode::IdentityAmbiguous);
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(ResultCode::IdentityAmbiguous.as_str())
    );
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(theirs.message.as_str())
    );
    assert!(!shell.present("confirm-ok", cx), "no Confirm");
    assert_eq!(RepositorySnapshot::take(&root), before);
    assert!(repairs(&shell, cx).is_empty());

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(
        focused(&shell, "repair-preview-button", cx),
        "focus is back"
    );
    assert_eq!(repair_text(&shell, cx), typed, "the text is kept");
}

/// A repair whose answer is lost after it ran (the library's
/// `BeforeRequestSettlement`, as Task 22's `lost_create` loses a create's)
/// has repaired the file: the dialog closes on `internal_error`, which the
/// view says, and the text is kept. Sent again under the same request, as
/// the worker's retry does, it is answered from what the library stored,
/// with no second commit. The tab then follows the document.
#[gpui_kit::test]
async fn a_repair_retried_after_a_lost_answer_answers_once(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let typed = completed_marker(&shell, &root, cx);
    preview(&shell, cx).await;
    failing_at(&shell, FailurePoint::BeforeRequestSettlement);
    let sent = confirm(&shell, cx).await;
    working(&shell);
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::InternalError)),
        "lost"
    );
    assert_eq!(
        own_commits(&root, &document_branch(ADOPTED)),
        Some(1),
        "it ran"
    );
    assert!(!shell.present("confirm-dialog", cx));
    assert_eq!(
        shell.label("repair-failure-code", cx).as_deref(),
        Some(ResultCode::InternalError.as_str())
    );
    assert_eq!(repair_text(&shell, cx), typed, "the text is kept");

    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.retry(sent, None), "sent again");
    shell
        .until_drawn(cx, "the retried answer", |shell, cx| {
            matches!(answer(shell, sent, cx), (2, Some(_)))
        })
        .await;
    let code = answer(&shell, sent, cx).1.unwrap();
    assert!(
        matches!(code, ResultCode::Ok | ResultCode::AlreadyApplied),
        "answered from what it stored: {}",
        code.as_str()
    );
    follows(&shell, &root, ItemDtoKind::Document, ADOPTED, "Adopted", cx).await;
    assert_eq!(repairs(&shell, cx), [sent], "one request");
    assert_eq!(
        own_commits(&root, &document_branch(ADOPTED)),
        Some(1),
        "made once"
    );
}

/// Unedited repair text closes with its tab, unasked. Edited, closing the
/// tab asks in the ruled words: Cancel, the default (Enter), and Escape
/// keep the tab and its text; the view's own Cancel asks the same, and its
/// Discard closes the view, the tab staying. Quitting counts the tab among
/// those whose changes no draft keeps, and Cancel stays. Discard closes the
/// tab. Nothing is written.
#[gpui_kit::test]
async fn closing_or_quitting_with_edited_repair_text_asks_first(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let before = RepositorySnapshot::take(&root);
    let tabs = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .len()
    };
    open(&shell, inspect(&root, MARKER), cx);
    open_repair(&shell, cx);
    shell.keys("secondary-w", cx);
    assert!(!shell.present("repair-discard-dialog", cx), "unasked");
    assert_eq!(tabs(&shell, cx), 0, "unedited, it closes");

    open(&shell, inspect(&root, MARKER), cx);
    open_repair(&shell, cx);
    type_at_line(&shell, 0, "x", cx);
    let typed = repair_text(&shell, cx);
    assert_eq!(typed, format!("x{MARKER_ONLY}"));
    for key in ["enter", "escape"] {
        shell.keys("secondary-w", cx);
        assert_eq!(
            shell.label("repair-discard-dialog", cx).as_deref(),
            Some(DISCARD_TITLE)
        );
        assert_eq!(
            shell.label("repair-discard-message", cx).as_deref(),
            Some(DISCARD_BODY)
        );
        assert_eq!(
            shell.label("repair-discard-discard", cx).as_deref(),
            Some("Discard")
        );
        assert_eq!(
            shell.label("repair-discard-cancel", cx).as_deref(),
            Some("Cancel")
        );
        shell.keys(key, cx);
        assert!(!shell.present("repair-discard-dialog", cx), "{key}");
        assert_eq!(tabs(&shell, cx), 1, "{key} keeps the tab");
        assert_eq!(repair_text(&shell, cx), typed, "{key} keeps the text");
        assert!(
            text_focused(&shell, cx),
            "{key} gives focus back to the text"
        );
    }

    shell.click("repair-cancel-button", cx);
    assert!(
        shell.present("repair-discard-dialog", cx),
        "Cancel asks too"
    );
    shell.click("repair-discard-discard-button", cx);
    assert!(!shell.present("repair-source", cx), "the view closes");
    assert!(shell.present("item-source", cx), "the tab shows the file");
    assert_eq!(tabs(&shell, cx), 1);

    open_repair(&shell, cx);
    type_at_line(&shell, 0, "y", cx);
    let quit = shell.frame(cx, |window, cx| {
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.request_quit(window, cx))
    });
    shell.run(cx);
    assert!(!quit, "quitting asks first");
    assert_eq!(
        shell.label("edits-not-kept-message", cx).as_deref(),
        Some("1 tab has unsaved changes that are not kept as a draft")
    );
    shell.keys("enter", cx);
    assert!(!shell.present("edits-not-kept-dialog", cx));
    assert_eq!(repair_text(&shell, cx), format!("y{MARKER_ONLY}"));

    shell.keys("secondary-w", cx);
    shell.click("repair-discard-discard-button", cx);
    assert_eq!(tabs(&shell, cx), 0, "Discard closes the tab");
    assert_eq!(RepositorySnapshot::take(&root), before);
}

/// A file that is not UTF-8 has no text to repair: "Needs fixing" says it
/// cannot be repaired here, in the ruled words, and offers no Repair….
#[gpui_kit::test]
async fn a_non_utf8_file_says_it_cannot_be_repaired_here(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    shell.fixture.write_non_utf8(&root, MARKER);
    open(&shell, inspect(&root, MARKER), cx);
    assert!(shell.present("item-needs-fixing", cx));
    assert_eq!(
        shell.label("item-repair-not-utf8", cx).as_deref(),
        Some(NOT_UTF8)
    );
    assert!(inside(&shell, "item-repair-not-utf8", "item-banners", cx));
    assert!(!shell.present("item-repair", cx));
    assert!(!shell.present("item-repair-button", cx));
}

/// R-25: the repair view is a plain text area, never the markdown editor.
/// A body the editor's load would rewrite (CRLF line endings, a `$$` pair)
/// is held exactly as read and repaired exactly as typed; no edit editor
/// and no read view is made, and no draft holds the text.
#[gpui_kit::test]
async fn the_repair_view_never_passes_through_the_markdown_editor(cx: &mut TestAppContext) {
    let body = "First\r\nWhat if words $$E=mc^2$$ more\r\n";
    assert_ne!(load(body), body, "the editor would rewrite it");
    let file = format!("---\nmanyhands_managed: true\n---\n{body}");
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, MARKER, file.as_bytes());
    });
    open(&shell, inspect(&root, MARKER), cx);
    open_repair(&shell, cx);
    assert_eq!(repair_text(&shell, cx), file, "exactly as read");
    type_at_line(
        &shell,
        2,
        &format!("manyhands_kind: document\nid: {ADOPTED}\ntitle: Raw\n"),
        cx,
    );
    let typed = repair_text(&shell, cx);
    assert_eq!(
        typed,
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {ADOPTED}\n\
             title: Raw\n---\n{body}"
        )
    );
    let (host, read) = pane(&shell, cx).read_with(cx, |pane, _| {
        (pane.edit_host().is_some(), pane.read_view().is_some())
    });
    assert!(!host && !read, "no markdown editor");
    let tab = TabKey::for_target(&inspect(&root, MARKER));
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    assert!(drafts.read_with(cx, |registry, _| registry.attached(&tab).is_none()));
    assert!(
        shell.fixture.draft_store().list().unwrap().is_empty(),
        "no draft"
    );

    preview(&shell, cx).await;
    let sent = confirm(&shell, cx).await;
    assert_eq!(answer(&shell, sent, cx), (1, Some(ResultCode::Ok)));
    follows(&shell, &root, ItemDtoKind::Document, ADOPTED, "Raw", cx).await;
    assert_eq!(
        shown(&shell, &root, ADOPTED).source.as_deref(),
        Some(typed.as_str()),
        "repaired exactly as typed"
    );
}

/// The adopted title (fix 1, I1): the first ATX heading's text, its closing
/// hashes dropped, and not one inside a fenced code block, nor a `#` with
/// no space after it; else the file stem.
#[test]
fn the_adopted_title_is_the_first_heading_or_else_the_file_stem() {
    assert_eq!(
        super::adopt_title("docs/a/plan.md", "x\n# Plan\n## Later\n"),
        "Plan"
    );
    assert_eq!(
        super::adopt_title("docs/plan.md", "   ### Spaced ###  \r\n"),
        "Spaced"
    );
    assert_eq!(
        super::adopt_title("docs/plan.md", "```\n# not\n```\n#tag\n#\n"),
        "plan"
    );
}

/// After an answer whose effect is unknown, a lost one, the text stays
/// read-only, as while the request ran, and Preview prepares that same
/// request again: the library answers from what it stored
/// (`already_applied`), and the tab follows the repair made, with no new
/// request and one commit (fix 1, M1).
#[gpui_kit::test]
async fn preview_after_a_lost_answer_follows_the_repair_made(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let typed = completed_marker(&shell, &root, cx);
    preview(&shell, cx).await;
    failing_at(&shell, FailurePoint::BeforeRequestSettlement);
    let sent = confirm(&shell, cx).await;
    working(&shell);
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::InternalError))
    );
    type_at_line(&shell, 0, "x", cx);
    assert_eq!(repair_text(&shell, cx), typed, "read-only");

    preview(&shell, cx).await;
    assert_eq!(
        shell.label("confirm-code", cx).as_deref(),
        Some(ResultCode::AlreadyApplied.as_str())
    );
    follows(&shell, &root, ItemDtoKind::Document, ADOPTED, "Adopted", cx).await;
    assert_eq!(repairs(&shell, cx), [sent], "no new request");
    assert_eq!(own_commits(&root, &document_branch(ADOPTED)), Some(1));
}

/// A repair that lands while another tab is shown switches no tab: its
/// tab closes for the item's own open tab, and the shown tab stays (fix
/// 1, M2).
#[gpui_kit::test]
async fn a_repair_finishing_in_a_background_tab_switches_no_tab(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, &ticket_path(BROKEN), untyped().as_bytes());
        fixture.commit_ticket(root, READABLE, "Readable", "");
    });
    let ticket = |id: &str| OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    };
    open(&shell, ticket(BROKEN), cx);
    open(&shell, ticket(READABLE), cx);
    open(&shell, inspect(&root, &ticket_path(BROKEN)), cx);
    open_repair(&shell, cx);
    type_at_line(&shell, 5, "type: task\n", cx);
    preview(&shell, cx).await;

    let held = shell.backend.hold_job(JobKind::TicketRepair);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    shell.keys("ctrl-shift-tab", cx);
    let shown = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.opened(cx))
    };
    assert_eq!(shown(&shell, cx), Some(ticket(READABLE)));

    held.release();
    shell
        .until_drawn(cx, "the repair's tab closing", |shell, cx| {
            shell
                .workspace
                .read_with(cx, |workspace, _| workspace.tab_keys())
                .len()
                == 2
        })
        .await;
    let sent = repairs(&shell, cx)[0];
    assert_eq!(answer(&shell, sent, cx), (1, Some(ResultCode::Ok)));
    assert_eq!(shown(&shell, cx), Some(ticket(READABLE)), "no switch");
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys()),
        [
            TabKey::for_target(&ticket(BROKEN)),
            TabKey::for_target(&ticket(READABLE))
        ]
    );
}

/// A repository's removal counts a tab whose repair text is edited among
/// those that removing closes, as quitting does (fix 1, M4). Cancel keeps
/// the tab and its text.
#[gpui_kit::test]
async fn a_removal_counts_edited_repair_text(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    open(&shell, inspect(&root, MARKER), cx);
    open_repair(&shell, cx);
    type_at_line(&shell, 0, "x", cx);
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.remove_repository(key(&root), window, cx)
        })
    });
    previewed(&shell, cx).await;
    assert_eq!(
        shell.label("confirm-unkept-note", cx).as_deref(),
        Some("1 tab has unsaved changes that are not kept as a draft. Removing closes it.")
    );
    shell.click("confirm-cancel", cx);
    assert_eq!(repair_text(&shell, cx), format!("x{MARKER_ONLY}"));
}

/// A clock a test moves on, which the library times its records by.
struct MovedClock(std::sync::Mutex<time::OffsetDateTime>);

impl manyhands::repository::Clock for MovedClock {
    fn now(&self) -> time::OffsetDateTime {
        *self.0.lock().unwrap()
    }
}

/// A stale preview, one that has expired, is refused, and the dialog
/// offers Prepare again, which prepares the same request afresh; Confirm
/// then sends that request again with the new confirmation, the repair is
/// made, and the tab follows, with one request and one commit (fix 1, M5).
#[gpui_kit::test]
async fn a_stale_preview_is_refused_and_prepare_again_is_offered(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let clock = std::sync::Arc::new(MovedClock(std::sync::Mutex::new(
        time::OffsetDateTime::now_utc(),
    )));
    shell.backend.replace_service(
        RepositoryService::open_at(shell.fixture.data_dir())
            .unwrap()
            .with_clock_for_testing(clock.clone()),
    );
    completed_marker(&shell, &root, cx);
    preview(&shell, cx).await;
    assert!(shell.present("confirm-ok", cx));
    *clock.0.lock().unwrap() += time::Duration::minutes(11);

    let sent = confirm(&shell, cx).await;
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::ConfirmationExpired))
    );
    assert_eq!(
        shell.label("confirm-prepare-again", cx).as_deref(),
        Some("Prepare again")
    );
    shell.click("confirm-prepare-again", cx);
    previewed(&shell, cx).await;
    assert!(shell.present("confirm-ok", cx), "a fresh preview");
    shell.click("confirm-ok", cx);
    shell
        .until_drawn(cx, "the second answer", |shell, cx| {
            matches!(answer(shell, sent, cx), (2, Some(_)))
        })
        .await;
    assert_eq!(answer(&shell, sent, cx), (2, Some(ResultCode::Ok)));
    follows(&shell, &root, ItemDtoKind::Document, ADOPTED, "Adopted", cx).await;
    assert_eq!(repairs(&shell, cx), [sent], "one request");
    assert_eq!(own_commits(&root, &document_branch(ADOPTED)), Some(1));
}

/// Adopt…'s ID read failing says why beside Adopt…, the library's message
/// and code, and opens no view; a later answer of the read it replaced is
/// dropped. A later Adopt… clears it and opens the view (fix 2, N1). The
/// library's `new_item_id` cannot fail, so the failed answer is given to
/// the pane.
#[gpui_kit::test]
async fn a_failed_id_read_says_why_beside_adopt(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, NOTES, b"# Notes\n");
    });
    open(&shell, inspect(&root, NOTES), cx);
    let held = shell.backend.hold_read(ReadKind::NewId);
    shell.click("item-repair-button", cx);
    let failure = ReadFailure {
        code: ResultCode::RepositoryInaccessible,
        recovery: Vec::new(),
    };
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.answer_adoption_for_testing(ReadResult::Failed(failure), cx)
    });
    shell.run(cx);
    assert_eq!(
        shell.label("repair-failure", cx).as_deref(),
        Some(ResultCode::RepositoryInaccessible.message())
    );
    assert_eq!(
        shell.label("repair-failure-code", cx).as_deref(),
        Some(ResultCode::RepositoryInaccessible.as_str())
    );
    assert!(inside(&shell, "repair-failure", "item-banners", cx));
    drop(held);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("repair-source", cx), "no view opens");

    pane(&shell, cx).update(cx, |pane, cx| {
        pane.answer_adoption_for_testing(ReadResult::Busy, cx)
    });
    shell.run(cx);
    assert_eq!(
        shell.label("repair-failure-code", cx).as_deref(),
        Some(ResultCode::Busy.as_str())
    );

    let (_, _) = adopt(&shell, cx).await;
    assert!(!shell.present("repair-failure", cx), "Adopt… cleared it");
}

/// A first heading that cannot be a title, holding a NUL, gives way to the
/// file stem; the file is still adopted with its front matter, byte for
/// byte after it (fix 2, N2).
#[gpui_kit::test]
async fn a_heading_that_cannot_be_a_title_gives_way_to_the_file_stem(cx: &mut TestAppContext) {
    let written = "# Bad\u{0}title\n\nBody.\n";
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(root, NOTES, written.as_bytes());
    });
    open(&shell, inspect(&root, NOTES), cx);
    assert_eq!(super::adopt_title(NOTES, written), "Bad\u{0}title");
    let (prefilled, id) = adopt(&shell, cx).await;
    assert_eq!(prefilled, adoption(&id, "notes", written));
}

/// Edited repair text is unsaved in the header's pill too, in its words,
/// as the strip marks it (Part E review, I1): unedited, the pill is the
/// viewing state; typed in, "Editing · unsaved changes"; Cancel, then
/// Discard, gives the viewing state back.
#[gpui_kit::test]
async fn edited_repair_text_is_unsaved_in_the_pill(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    open(&shell, inspect(&root, MARKER), cx);
    let viewing = Some("Viewing the shared copy");
    assert_eq!(shell.label("item-pill", cx).as_deref(), viewing);
    open_repair(&shell, cx);
    assert_eq!(shell.label("item-pill", cx).as_deref(), viewing, "unedited");

    type_at_line(&shell, 0, "x", cx);
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Editing \u{b7} unsaved changes")
    );

    shell.click("repair-cancel-button", cx);
    shell.click("repair-discard-discard-button", cx);
    assert!(!shell.present("repair-source", cx), "the view closes");
    assert_eq!(shell.label("item-pill", cx).as_deref(), viewing);
}

/// Escape from the confirmation while the repair it sent runs gives focus
/// back to the text area, read-only but focusable, not to Preview, which
/// waits and holds no focus (Part E review, M2). Meanwhile the pill says
/// the text is unsaved (I1). Once the repair lands, the tab follows.
#[gpui_kit::test]
async fn escape_while_the_repair_runs_gives_focus_back_to_the_text(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    completed_marker(&shell, &root, cx);
    shell.keys("tab", cx);
    assert!(focused(&shell, "repair-preview-button", cx));
    shell.press("enter", cx);
    previewed(&shell, cx).await;
    assert!(shell.present("confirm-ok", cx), "a preview");

    let held = shell.backend.hold_job(JobKind::DocumentRepair);
    shell.keys("enter", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx), "Escape closes it");
    assert!(text_focused(&shell, cx), "focus is in the text area");
    assert!(shell.present("repair-source", cx));
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Editing \u{b7} unsaved changes"),
        "while the repair runs"
    );

    held.release();
    follows(&shell, &root, ItemDtoKind::Document, ADOPTED, "Adopted", cx).await;
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Viewing your private copy"),
        "the repaired document, viewed"
    );
}

// Task 27 fix 1.

/// A3's words (ruling D2): what the question says once the repair was
/// sent, on closing the tab and on the view's Cancel alike.
const SENT_BODY: &str =
    "Your repair was sent and may already be made. Discarding does not undo it.";

fn tab_count(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys())
        .len()
}

/// The prompt's title, message and buttons, as closing the tab draws them.
fn discard_prompt(shell: &Shell, cx: &mut TestAppContext) -> [Option<String>; 4] {
    [
        "repair-discard-dialog",
        "repair-discard-message",
        "repair-discard-discard",
        "repair-discard-cancel",
    ]
    .map(|id| shell.label(id, cx))
}

fn prompt_saying(body: &str) -> [Option<String>; 4] {
    [DISCARD_TITLE, body, "Discard", "Cancel"].map(|label| Some(label.to_owned()))
}

/// A3: closing the tab while its repair runs asks under the same title,
/// with Discard and Cancel, but says the repair was sent and may already be
/// made, which closing does not undo. Discard closes the tab. The repair
/// goes on, and its answer, landing for a closed tab, opens nothing: the
/// file is repaired once.
#[gpui_kit::test]
async fn closing_while_the_repair_runs_says_it_was_sent(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    completed_marker(&shell, &root, cx);
    preview(&shell, cx).await;
    let held = shell.backend.hold_job(JobKind::DocumentRepair);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));

    shell.keys("secondary-w", cx);
    assert_eq!(discard_prompt(&shell, cx), prompt_saying(SENT_BODY));
    shell.click("repair-discard-discard-button", cx);
    assert_eq!(tab_count(&shell, cx), 0, "Discard closes the tab");

    let sent = repairs(&shell, cx)[0];
    held.release();
    shell.until_ended(sent, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(answer(&shell, sent, cx), (1, Some(ResultCode::Ok)));
    assert_eq!(tab_count(&shell, cx), 0, "nothing opens");
    assert_eq!(repairs(&shell, cx), [sent], "one request");
    assert_eq!(
        own_commits(&root, &document_branch(ADOPTED)),
        Some(1),
        "repaired once"
    );
}

/// A3: after an answer whose effect is unknown (`BeforeRequestSettlement`),
/// closing the tab says the repair was sent and may already be made, and
/// so does the view's own Cancel (ruling D2). Cancel keeps the tab and the
/// text.
#[gpui_kit::test]
async fn closing_after_a_lost_repair_answer_says_it_was_sent(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    let typed = completed_marker(&shell, &root, cx);
    preview(&shell, cx).await;
    failing_at(&shell, FailurePoint::BeforeRequestSettlement);
    let sent = confirm(&shell, cx).await;
    working(&shell);
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::InternalError)),
        "lost"
    );

    shell.keys("secondary-w", cx);
    assert_eq!(discard_prompt(&shell, cx), prompt_saying(SENT_BODY));
    shell.keys("escape", cx);
    assert!(!shell.present("repair-discard-dialog", cx));
    assert_eq!(tab_count(&shell, cx), 1, "Cancel keeps the tab");
    assert_eq!(repair_text(&shell, cx), typed, "and the text");

    shell.click("repair-cancel-button", cx);
    assert_eq!(discard_prompt(&shell, cx), prompt_saying(SENT_BODY));
    shell.keys("escape", cx);
    assert!(shell.present("repair-source", cx), "the view stays");
}

/// A3: every other state keeps the first words. Edited and never sent,
/// closing the tab and the view's Cancel both say the changes are not
/// saved anywhere; so does closing after a refusal, the repair sent but
/// not made.
#[gpui_kit::test]
async fn closing_unsent_or_refused_repair_text_says_it_is_not_saved(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    completed_marker(&shell, &root, cx);
    shell.keys("secondary-w", cx);
    assert_eq!(discard_prompt(&shell, cx), prompt_saying(DISCARD_BODY));
    shell.keys("escape", cx);
    shell.click("repair-cancel-button", cx);
    assert_eq!(
        shell.label("repair-discard-message", cx).as_deref(),
        Some(DISCARD_BODY)
    );
    shell.keys("escape", cx);

    preview(&shell, cx).await;
    shell.fixture.commit_file(
        &root,
        MARKER,
        b"---\nmanyhands_managed: true\n---\nChanged by another.\n",
    );
    let sent = confirm(&shell, cx).await;
    assert_eq!(
        answer(&shell, sent, cx),
        (1, Some(ResultCode::ExternalChange)),
        "refused"
    );
    shell.keys("secondary-w", cx);
    assert_eq!(discard_prompt(&shell, cx), prompt_saying(DISCARD_BODY));
    shell.keys("escape", cx);
    assert_eq!(tab_count(&shell, cx), 1);
}

/// C3 (contract 6: a reload or refresh never replaces unsaved repair
/// text): the repair text typed in, the file changed by another, and the
/// tab read again by a refresh, `secondary-r`. The view stays open with the
/// typed text, and the pill still says it is unsaved.
#[gpui_kit::test]
async fn a_refresh_never_replaces_edited_repair_text(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_nonconforming_document(root, MARKER);
    });
    shell.select(&root, cx);
    open(&shell, inspect(&root, MARKER), cx);
    open_repair(&shell, cx);
    type_at_line(&shell, 0, "x", cx);
    let typed = repair_text(&shell, cx);
    assert_eq!(typed, format!("x{MARKER_ONLY}"));
    shell.fixture.commit_file(
        &root,
        MARKER,
        b"---\nmanyhands_managed: true\n---\nChanged by another.\n",
    );

    let reads = shell.backend.reads_seen(ReadKind::Item);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    });
    assert_eq!(refresh.len(), 1, "secondary-r refreshes");
    shell.until_ended(refresh[0], cx).await;
    shell
        .until_drawn(cx, "the tab read again", |shell, cx| {
            shell.backend.reads_seen(ReadKind::Item) > reads
                && pane(shell, cx).read_with(cx, |pane, _| {
                    pane.model
                        .item()
                        .rows()
                        .and_then(|item| item.source.as_deref())
                        == Some("---\nmanyhands_managed: true\n---\nChanged by another.\n")
                })
        })
        .await;
    assert!(shell.present("repair-source", cx), "the view stays open");
    assert_eq!(repair_text(&shell, cx), typed, "with the typed text");
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some("Editing \u{b7} unsaved changes")
    );
}
