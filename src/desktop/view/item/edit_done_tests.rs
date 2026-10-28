//! Done and the kept session, over real repositories (design, Done and
//! leaving; decision 15; R-15, R-41, R-58, R-68, R-71, R-73): the design's
//! session-outcome rows for Task 16 (R-76), written first; a revision
//! still waiting on the timer, written before Done decides; the kept
//! session's saved copy and its way back into the same editor; a refresh
//! under a kept session; a session the Drafts page ends; and closing a tab
//! with unsaved work.
//!
//! The helpers are `edit_tests`'s. Each test sets the caret, focuses the
//! editor and draws a frame before typing, and types through the window;
//! each waits on the worker and allows parking. Draft fields and items
//! would print text through `Debug`, so they are compared with `assert!`.
//! The words are literals: the tests fix them.

use super::*;
use crate::desktop::model::edit::{DoneOutcome, done_outcome};
#[cfg(unix)]
use crate::desktop::test_support::drafts::{FolderMode, superuser};

const KEPT: &str = "Viewing \u{b7} unsaved changes kept";
const VIEWING_PRIVATE: &str = "Viewing your private copy";
const KEPT_WORDS: &str = "Your unsaved changes are kept as a draft on this computer.";
const DISCARDED_WORDS: &str = "The draft was discarded.";
const CHANGED_WORDS: &str = "Someone changed this file outside Manyhands. Your edits are kept.";
#[cfg(unix)]
const NOT_KEPT_WORDS: &str = "Your unsaved changes could not be kept as a draft on this \
                              computer; they are only in the editor. Save them, or discard them.";
const NOT_CREATED_WORDS: &str = "This item is not created yet, so there is no saved copy to \
                                 view. Save creates it; Keep draft keeps it on this computer, to \
                                 continue later.";
const CLOSE_KEEP_WORDS: &str = "Keep draft keeps your unsaved changes on this computer, to \
                                continue later. Discard deletes them.";
#[cfg(unix)]
const CLOSE_UNKEPT_WORDS: &str = "Your unsaved changes are not kept as a draft on this \
                                  computer. Discard deletes them; Cancel goes back to the tab.";

/// The save requests of this session, newest first.
fn saves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::TicketSave { .. } | Intent::DocumentSave { .. }
        )
    })
}

/// The `context release` requests of this session: Task 17 sends them.
fn releases(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .requests(cx, |request| {
            matches!(request.intent, Intent::ContextRelease { .. })
        })
        .len()
}

fn edit_text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}

/// Whether the shown tab's edit session lives, and whether it is kept.
fn session(shell: &Shell, cx: &mut TestAppContext) -> Option<bool> {
    pane(shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().map(|session| session.kept())
    })
}

/// The stored body of the draft `id`.
fn body(shell: &Shell, id: DraftId) -> Option<String> {
    stored_body(&shell.fixture.draft_store(), id)
}

/// Presses Done, and lets what it starts finish.
fn done(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("item-done", cx);
    shell.run(cx);
}

/// Undoes in `editor`, at most three times, until it holds `before`:
/// whether undo reaches it.
fn undo_reaches(
    shell: &Shell,
    editor: &Entity<EditorState>,
    before: &str,
    cx: &mut TestAppContext,
) -> bool {
    let end = text(editor, cx).len();
    focus_at(shell, editor, end, cx);
    for _ in 0..3 {
        shell.keys("secondary-z", cx);
        if text(editor, cx) == before {
            return true;
        }
    }
    false
}

/// The Drafts page's Discard… of the draft `id`, which the page lists as
/// held by the tab, confirmed; then the ticket's tab is shown again.
fn discard_from_the_drafts_page(shell: &Shell, root: &Path, id: DraftId, cx: &mut TestAppContext) {
    shell.keys("secondary-shift-d", cx);
    assert_eq!(
        shell.label(row_id(id, "state"), cx).as_deref(),
        Some("Restored in this window"),
        "held by this tab"
    );
    assert!(shell.present(row_id(id, "show"), cx), "it can show the tab");
    shell.click(row_id(id, "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx), "it asks first");
    shell.click("discard-draft-confirm", cx);
    shell.run(cx);
    open(shell, ticket(root, TICKET), cx);
}

/// The session's fields, with `title` as its title.
fn titled(shell: &Shell, title: &str, cx: &mut TestAppContext) -> DraftFields {
    let mut fields = pane(shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().unwrap().base().fields.clone()
    });
    if let DraftFields::Ticket { title: old, .. } | DraftFields::Document { title: old, .. } =
        &mut fields
    {
        *old = title.to_owned();
    }
    fields
}

/// Repository `a` with the ticket `title`, refreshed, in an open window
/// whose draft store did not open: its folder is a link. The link's
/// target goes with the last value.
#[cfg(unix)]
fn shell_without_a_store(
    cx: &mut TestAppContext,
    title: &str,
) -> (Shell, PathBuf, tempfile::TempDir) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, title, "");
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root, elsewhere)
}

/// Edit with crash recovery off: Continue, then the editor.
#[cfg(unix)]
async fn edit_without_a_store(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorHost> {
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    until_editing(shell, cx).await
}

// The design's session-outcome rows for Task 16 (R-76), written first.

/// Row: Done while a save is in flight, and it commits. Done waits for the
/// answer, then applies its rules: the save left the editor clean and the
/// draft retired, so the session ends and the tab is viewing.
#[gpui_kit::test]
async fn done_waits_for_a_committed_save_then_applies_its_rules(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Waited", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(false), "Done waits for the save");
    assert_eq!(phase(&shell, cx), Phase::Saving);

    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    let shown = pane(&shell, cx);
    wait_until(cx, &shown, |pane, _| pane.edit_session().is_none()).await;
    shell.frame(cx, |_, _| ());
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(saves(&shell, cx), [request], "one save");
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert!(stored_bodies(&shell).is_empty(), "nothing is stored");
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_PRIVATE));
    assert!(shell.present("item-read", cx));
    assert!(!shell.present("item-toolbar", cx));
    let item = shown_item(&shell, &root);
    assert!(item.body.as_deref() == Some(format!("{original}x").as_str()));
}

fn shown_item(shell: &Shell, root: &Path) -> ItemDto {
    super::shown(shell, root, TICKET)
}

/// Done stopped: the session lives, not kept, with its text and its draft,
/// and no context was released.
fn done_stopped(shell: &Shell, root: &Path, id: DraftId, typed: &str, cx: &mut TestAppContext) {
    assert_eq!(session(shell, cx), Some(false), "Done stopped");
    let host = pane(shell, cx)
        .read_with(cx, |pane, _| pane.edit_host())
        .unwrap();
    assert!(edit_text(&host, cx) == typed, "the text is kept");
    assert_eq!(
        attached(shell, root, TICKET, cx),
        Some(id),
        "nothing is discarded"
    );
    assert!(
        body(shell, id).as_deref() == Some(typed),
        "the draft is kept"
    );
    assert_eq!(releases(shell, cx), 0, "nothing is released");
}

/// Save with its job held, Done while it is in flight, then the job
/// released: Done's wait ends with the save's answer.
async fn done_during_a_save(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    let before = saves(shell, cx).len();
    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    done(shell, cx);
    assert_eq!(session(shell, cx), Some(false), "Done waits for the save");
    assert_eq!(phase(shell, cx), Phase::Saving);
    held.release();
    let sent = saves(shell, cx);
    assert_eq!(sent.len(), before + 1, "Save sent one save");
    ended(shell, sent[0], cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    sent[0]
}

/// A service over the fixture's data that fails at `point`, in place of
/// the one in use.
fn failing_at(shell: &Shell, point: manyhands::repository::FailurePoint) {
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(shell.fixture.data_dir(), point)
            .unwrap(),
    );
}

/// The fixture's own service, in place of a failing one.
fn working(shell: &Shell) {
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
}

/// Row: Done while a save is in flight, and it answers a refusal, a
/// pending checkpoint or `external_change`. Done stops each time: the
/// banner shows the refusal, or the checkpoint with Retry, or the review
/// opens; nothing is discarded or released.
#[gpui_kit::test]
async fn done_stops_when_the_waited_save_answers_anything_but_committed(cx: &mut TestAppContext) {
    use manyhands::repository::FailurePoint;

    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Stopped", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let typed = edit_text(&host, cx);

    // A refusal: the library stops the request before its domain call, a
    // refusal no desktop check can make first.
    failing_at(&shell, FailurePoint::BeforeRequestDomainCall);
    let refused = done_during_a_save(&shell, cx).await;
    working(&shell);
    assert_eq!(code(&shell, refused, cx), Some(ResultCode::InternalError));
    assert_eq!(
        shell.label("item-save-refused", cx).as_deref(),
        Some("Could not save on this computer. Your edits are kept.")
    );
    assert_eq!(phase(&shell, cx), Phase::Editing);
    done_stopped(&shell, &root, id, &typed, cx);
    assert!(created(&shell, &root, TICKET, cx), "the context is kept");

    // A pending checkpoint: the file is written, the commit is not.
    failing_at(
        &shell,
        FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
    );
    let pending = done_during_a_save(&shell, cx).await;
    assert_eq!(
        shell.label("item-save-failed", cx).as_deref(),
        Some("Could not save a checkpoint on this computer. Your text is kept.")
    );
    assert_eq!(shell.label("item-save-retry", cx).as_deref(), Some("Retry"));
    assert_eq!(phase(&shell, cx), Phase::Editing);
    done_stopped(&shell, &root, id, &typed, cx);
    assert!(created(&shell, &root, TICKET, cx), "the context is kept");

    // Retry commits, with Done no longer waiting: the session goes on.
    working(&shell);
    shell.click("item-save-retry", cx);
    ended(&shell, pending, cx).await;
    shell.run(cx);
    assert_eq!(code(&shell, pending, cx), Some(ResultCode::Ok));
    assert_eq!(session(&shell, cx), Some(false), "still editing");

    // `external_change`: another actor saved since the base.
    type_at_end(&shell, &host, "y", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("a draft began again");
    let typed = edit_text(&host, cx);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    let changed = done_during_a_save(&shell, cx).await;
    assert_eq!(code(&shell, changed, cx), Some(ResultCode::ExternalChange));
    assert_eq!(phase(&shell, cx), Phase::Review, "the review opens");
    assert_eq!(
        shell.label("item-save-changed", cx).as_deref(),
        Some(CHANGED_WORDS)
    );
    done_stopped(&shell, &root, id, &typed, cx);

    // During the review Done is disabled, with its note (step 1).
    assert_eq!(
        shell.label("item-done-note", cx).as_deref(),
        Some("Use these choices or Cancel first")
    );
    done(&shell, cx);
    assert_eq!(phase(&shell, cx), Phase::Review, "Done does nothing");
    assert!(!shell.present("done-draft-dialog", cx));
    assert_eq!(session(&shell, cx), Some(false));
}

/// Minor 1 (review): a pending checkpoint already answered still holds the
/// save. Done stops: the tab stays editing, and the checkpoint's banner,
/// with Retry, stays in view, never hidden under a kept session.
#[gpui_kit::test]
async fn done_with_a_pending_checkpoint_stops_and_shows_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Pending", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let typed = edit_text(&host, cx);
    failing_at(
        &shell,
        manyhands::repository::FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
    );
    shell.click("item-save", cx);
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    working(&shell);
    assert!(
        shell.present("item-save-failed", cx),
        "the checkpoint is pending"
    );

    done(&shell, cx);
    assert_eq!(phase(&shell, cx), Phase::Editing, "Done stops");
    assert!(!shell.present("done-draft-dialog", cx));
    assert_eq!(
        shell.label("item-save-failed", cx).as_deref(),
        Some("Could not save a checkpoint on this computer. Your text is kept.")
    );
    assert_eq!(shell.label("item-save-retry", cx).as_deref(), Some("Retry"));
    assert_ne!(pill(&shell, cx).as_deref(), Some(KEPT));
    done_stopped(&shell, &root, id, &typed, cx);
}

/// Row: Done, dirty, and the write succeeds. The tab is viewing at once;
/// the session is kept, and Continue editing in the kept-draft banner
/// shows the same editor again, with its text, cursor and undo.
#[gpui_kit::test]
async fn done_with_unsaved_text_writes_the_draft_and_keeps_the_session(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "xyz", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let at = original.len() + 1;
    editor.update(cx, |editor, cx| editor.set_cursor(at, cx));

    done(&shell, cx);
    assert_eq!(phase(&shell, cx), Phase::Viewing, "viewing at once");
    assert_eq!(session(&shell, cx), Some(true), "the session is kept");
    assert!(
        body(&shell, id) == Some(format!("{original}xyz")),
        "written"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(KEPT));
    assert_eq!(shell.label("item-kept", cx).as_deref(), Some(KEPT_WORDS));
    assert_eq!(
        shell.label("item-kept-continue", cx).as_deref(),
        Some("Continue editing")
    );
    assert_eq!(
        shell.label("item-kept-discard", cx).as_deref(),
        Some("Discard\u{2026}")
    );
    assert!(shell.present("item-read", cx), "the saved copy");
    assert!(!shell.present("item-edit-view", cx));
    assert!(!shell.present("item-toolbar", cx));
    assert!(text(&read_editor(&shell, cx), cx) == original);
    assert_eq!(releases(&shell, cx), 0, "nothing is released");

    shell.click("item-kept-continue", cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert_eq!(session(&shell, cx), Some(false));
    let again = pane(&shell, cx)
        .read_with(cx, |pane, _| pane.edit_host())
        .unwrap();
    assert_eq!(again.entity_id(), host.entity_id(), "the same editor");
    assert!(edit_text(&host, cx) == format!("{original}xyz"), "its text");
    assert_eq!(cursor(&editor, cx), at, "its cursor");
    assert!(focused(&shell, &editor, cx), "focus is back in it");
    assert_eq!(prepares(&shell, cx).len(), 1, "nothing is prepared again");
    assert!(undo_reaches(&shell, &editor, &original, cx), "its undo");
}

/// Row: Done, dirty, and the write fails. Done asks Save, Discard or
/// Cancel, with no Keep draft: once with the drafts folder unwritable,
/// where Save commits from the text in memory (Task 15's ruling) and Done
/// then ends the session; and once with crash recovery off, where Discard
/// leaves the saved copy as it was.
#[cfg(unix)]
#[gpui_kit::test]
async fn done_asks_when_the_draft_cannot_be_written(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Unwritten", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    attached(&shell, &root, TICKET, cx).expect("the draft began");
    let _read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    type_at_end(&shell, &host, "y", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(DRAFT_NOT_KEPT)
    );

    done(&shell, cx);
    assert!(shell.present("done-draft-dialog", cx), "Done asks");
    assert_eq!(
        shell.label("done-draft-message", cx).as_deref(),
        Some(NOT_KEPT_WORDS)
    );
    assert_eq!(shell.label("done-draft-save", cx).as_deref(), Some("Save"));
    assert_eq!(
        shell.label("done-draft-discard", cx).as_deref(),
        Some("Discard")
    );
    assert_eq!(
        shell.label("done-draft-cancel", cx).as_deref(),
        Some("Cancel")
    );
    assert!(!shell.present("done-draft-keep", cx), "no Keep draft");
    shell.click("done-draft-cancel-button", cx);
    assert!(!shell.present("done-draft-dialog", cx));
    assert_eq!(session(&shell, cx), Some(false), "Cancel goes on editing");
    assert_eq!(phase(&shell, cx), Phase::Editing);

    done(&shell, cx);
    shell.click("done-draft-save-button", cx);
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 1, "Save saves");
    ended(&shell, sent[0], cx).await;
    let shown = pane(&shell, cx);
    wait_until(cx, &shown, |pane, _| pane.edit_session().is_none()).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert_eq!(phase(&shell, cx), Phase::Viewing, "then Done ends it");
    let item = shown_item(&shell, &root);
    assert!(item.body.as_deref() == Some(format!("{original}xy").as_str()));

    // Crash recovery off: nothing can be written.
    let (shell, root, _elsewhere) = shell_without_a_store(cx, "Unprotected");
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit_without_a_store(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    done(&shell, cx);
    assert!(shell.present("done-draft-dialog", cx), "Done asks");
    assert!(shell.present("done-draft-save", cx));
    assert!(!shell.present("done-draft-keep", cx), "no Keep draft");
    shell.click("done-draft-discard-button", cx);
    shell.run(cx);
    assert_eq!(session(&shell, cx), None, "Discard ends the session");
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert!(saves(&shell, cx).is_empty(), "nothing is saved");
    assert!(text(&read_editor(&shell, cx), cx) == original);
    let item = shown_item(&shell, &root);
    assert!(item.body.as_deref() == Some(original.as_str()));
}

/// Row: Done after typing and undoing, with a stored draft. The draft is
/// discarded, the session ends, and the tab is viewing.
#[gpui_kit::test]
async fn done_after_typing_and_undoing_discards_the_stored_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Undone", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    assert!(body(&shell, id) == Some(format!("{original}x")), "stored");
    shell.keys("secondary-z", cx);
    assert!(edit_text(&host, cx) == original);
    assert!(!host.read_with(cx, |host, _| host.is_dirty()));

    done(&shell, cx);
    assert_eq!(session(&shell, cx), None, "the session ends");
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "the draft is discarded"
    );
    assert!(stored_bodies(&shell).is_empty());
    assert_eq!(attached(&shell, &root, TICKET, cx), None);
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_PRIVATE));
    assert!(!shell.present("item-kept", cx), "nothing is kept");
}

/// Row: Done on a never-created item asks Save, Keep draft, Discard or
/// Cancel; there is no saved copy to view. Cancel goes on editing; Keep
/// draft closes the tab and leaves the draft for the Drafts page. (A new
/// item's base has neither observation nor source, R-20; Task 22 makes
/// such sessions, so the test marks one.)
#[gpui_kit::test]
async fn done_on_a_never_created_item_asks(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Not yet", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    pane(&shell, cx).update(cx, |pane, _| pane.edit_session_mut().unwrap().as_new_item());
    type_at_end(&shell, &host, "x", cx);
    let typed = edit_text(&host, cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");

    done(&shell, cx);
    assert!(shell.present("done-draft-dialog", cx), "Done asks");
    assert_eq!(
        shell.label("done-draft-message", cx).as_deref(),
        Some(NOT_CREATED_WORDS)
    );
    for (part, label) in [
        ("done-draft-save", "Save"),
        ("done-draft-keep", "Keep draft"),
        ("done-draft-discard", "Discard"),
        ("done-draft-cancel", "Cancel"),
    ] {
        assert_eq!(shell.label(part, cx).as_deref(), Some(label), "{part}");
    }
    shell.click("done-draft-cancel-button", cx);
    assert!(!shell.present("done-draft-dialog", cx));
    assert_eq!(session(&shell, cx), Some(false), "Cancel goes on editing");

    done(&shell, cx);
    shell.click("done-draft-keep-button", cx);
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "the tab closes"
    );
    let store = shell.fixture.draft_store();
    assert!(
        matches!(held(&store, id), Some(Held::No)),
        "left for Drafts"
    );
    assert!(stored_body(&store, id) == Some(typed));
}

/// Row (R-71): the Drafts page discards a kept draft. The kept session is
/// dropped, and the tab is viewing the saved copy, saying so.
#[gpui_kit::test]
async fn a_drafts_page_discard_ends_the_kept_session_and_says_so(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discarded kept", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(true), "kept");

    discard_from_the_drafts_page(&shell, &root, id, cx);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert_eq!(session(&shell, cx), None, "the kept session is dropped");
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert_eq!(
        shell.label("item-draft-discarded", cx).as_deref(),
        Some(DISCARDED_WORDS)
    );
    assert!(!shell.present("item-kept", cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_PRIVATE));
    assert!(
        text(&read_editor(&shell, cx), cx) == original,
        "the saved copy"
    );
    assert_eq!(
        shell.label("item-edit", cx).as_deref(),
        Some("Edit"),
        "no draft is left to continue"
    );
}

/// Row (R-71): while a draft's save is in flight, the Drafts page's
/// Discard of it is disabled, so a discard never races the commit; once
/// the save is over it asks again.
#[gpui_kit::test]
async fn the_drafts_pages_discard_is_disabled_while_that_draft_saves(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Raced", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let held_save = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held_save.wait_reached();
    // Newer text: the draft goes on after the commit.
    type_at_end(&shell, &host, "y", cx);

    shell.keys("secondary-shift-d", cx);
    assert!(shell.present(row_id(id, "discard"), cx), "drawn");
    shell.click(row_id(id, "discard"), cx);
    assert!(
        !shell.present("discard-draft-dialog", cx),
        "disabled while the save is in flight"
    );
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::Here)
    ));

    held_save.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    shell.run(cx);
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let id = attached(&shell, &root, TICKET, cx).expect("the newer text is drafted");
    shell.click(row_id(id, "discard"), cx);
    assert!(
        shell.present("discard-draft-dialog", cx),
        "enabled once the save is over"
    );
    shell.click("discard-draft-cancel", cx);
}

// The other tests (decision 15; R-58, R-68, R-71, R-73, R-41).

/// R-73: a revision still waiting on the one-second timer is written
/// before Done decides; the draft then holds the newest text.
#[gpui_kit::test]
async fn done_writes_a_revision_still_waiting_on_the_timer_first(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Timed", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    type_at_end(&shell, &host, "y", cx);
    assert!(
        body(&shell, id) == Some(format!("{original}x")),
        "y waits on the timer"
    );

    done(&shell, cx);
    assert!(
        body(&shell, id) == Some(format!("{original}xy")),
        "written first"
    );
    assert_eq!(session(&shell, cx), Some(true), "then kept");
}

/// R-58: while the session is kept, the page, the header, its chips, the
/// details sheet and the tab's title show the saved copy, though the draft
/// holds another body, title and type; the tab keeps its unsaved mark.
#[gpui_kit::test]
async fn the_kept_session_shows_the_saved_copy_in_page_header_sheet_and_title(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Saved title", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let chip = shell
        .label(ChipField::Type.id(), cx)
        .expect("the type chip");
    assert!(chip.contains("task"), "{chip}");
    let mut fields = titled(&shell, "Draft title", cx);
    if let DraftFields::Ticket { ticket_type, .. } = &mut fields {
        *ticket_type = "bug".to_owned();
    }
    pane(&shell, cx).update(cx, |pane, cx| pane.change_fields(fields, cx));
    shell.run(cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");

    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(true), "kept");
    let draft = registry(&shell, cx).read_with(cx, |registry, _| {
        registry
            .session(id)
            .map(|session| session.current().clone())
    });
    assert!(
        matches!(&draft, Some(DraftFields::Ticket { title, ticket_type, body, .. })
            if title == "Draft title" && ticket_type == "bug" && *body == format!("{original}x")),
        "the draft holds the other title, type and body"
    );
    assert!(text(&read_editor(&shell, cx), cx) == original, "the page");
    assert_eq!(
        shell.label("item-title", cx).as_deref(),
        Some("Saved title"),
        "the header"
    );
    assert_eq!(
        shell.label(ChipField::Type.id(), cx).as_deref(),
        Some(chip.as_str()),
        "the chip"
    );
    assert_eq!(
        shell.rows("tab", cx),
        ["Saved title, unsaved"],
        "the tab's title, with its unsaved mark"
    );
    shell.click(ChipField::All.id(), cx);
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Saved title"),
        "the sheet"
    );
}

/// R-58, R-35: Continue editing in the header, `e` on the page and
/// `secondary-shift-e` each show the kept editor again, with its text,
/// cursor and undo; nothing is prepared again.
#[gpui_kit::test]
async fn continue_editing_edit_and_e_restore_text_cursor_and_undo(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Continued", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "xyz", cx);
    let typed = edit_text(&host, cx);
    let at = original.len() + 2;
    editor.update(cx, |editor, cx| editor.set_cursor(at, cx));
    let resumed = |shell: &Shell, cx: &mut TestAppContext| {
        assert_eq!(session(shell, cx), Some(false), "shown again");
        assert_eq!(phase(shell, cx), Phase::Editing);
        let again = pane(shell, cx)
            .read_with(cx, |pane, _| pane.edit_host())
            .unwrap();
        assert_eq!(again.entity_id(), host.entity_id(), "the same editor");
        assert!(edit_text(&host, cx) == typed, "its text");
        assert_eq!(cursor(&editor, cx), at, "its cursor");
        assert!(focused(shell, &editor, cx), "focused");
    };

    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(true));
    assert_eq!(
        shell.label("item-edit", cx).as_deref(),
        Some("Continue editing")
    );
    shell.click("item-edit", cx);
    resumed(&shell, cx);

    done(&shell, cx);
    let read = read_editor(&shell, cx);
    focus_at(&shell, &read, 0, cx);
    shell.keys("e", cx);
    resumed(&shell, cx);

    done(&shell, cx);
    let read = read_editor(&shell, cx);
    focus_at(&shell, &read, 0, cx);
    shell.keys("secondary-shift-e", cx);
    resumed(&shell, cx);

    assert_eq!(prepares(&shell, cx).len(), 1, "nothing is prepared again");
    assert!(attached(&shell, &root, TICKET, cx).is_some());
    assert!(undo_reaches(&shell, &editor, &original, cx), "its undo");
}

/// R-68: a refresh that finds the item changed under a kept session shows
/// the change banner, with Compare…; nothing opens by itself, and the kept
/// editor's text is untouched while the page shows their saved copy.
#[gpui_kit::test]
async fn a_refresh_while_kept_shows_the_change_banner_and_opens_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let typed = edit_text(&host, cx);
    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(true), "kept");
    assert!(
        !shell.present("item-kept-changed", cx),
        "nothing changed yet"
    );

    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    assert_eq!(
        shell.label("item-kept-changed", cx).as_deref(),
        Some(CHANGED_WORDS)
    );
    assert_eq!(
        shell.label("item-kept-compare", cx).as_deref(),
        Some("Compare\u{2026}")
    );
    assert_eq!(session(&shell, cx), Some(true), "still kept");
    assert_eq!(phase(&shell, cx), Phase::Viewing, "nothing opens");
    assert!(!shell.present("item-edit-view", cx));
    assert!(edit_text(&host, cx) == typed, "the kept text is untouched");
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Theirs"));
    assert_eq!(pill(&shell, cx).as_deref(), Some(KEPT));
}

/// R-71: the Drafts page discards the draft of a session still editing.
/// The session ends, and the tab is viewing the saved copy, saying so.
#[gpui_kit::test]
async fn a_drafts_page_discard_ends_a_live_session_and_says_so(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discarded live", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    assert_eq!(session(&shell, cx), Some(false), "editing");

    discard_from_the_drafts_page(&shell, &root, id, cx);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert_eq!(session(&shell, cx), None, "the session ends");
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert_eq!(
        shell.label("item-draft-discarded", cx).as_deref(),
        Some(DISCARDED_WORDS)
    );
    assert!(!shell.present("item-toolbar", cx));
    assert!(!shell.present("item-edit-view", cx));
    assert!(
        text(&read_editor(&shell, cx), cx) == original,
        "the saved copy"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_PRIVATE));
}

/// R-71: a discard from the Drafts page ends a never-created item's
/// session, which has no saved copy to view: its tab closes. (The session
/// is marked as a new item's, as in `done_on_a_never_created_item_asks`.)
#[gpui_kit::test]
async fn a_drafts_page_discard_closes_a_never_created_items_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Never created", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    pane(&shell, cx).update(cx, |pane, _| pane.edit_session_mut().unwrap().as_new_item());
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");

    shell.keys("secondary-shift-d", cx);
    shell.click(row_id(id, "discard"), cx);
    shell.click("discard-draft-confirm", cx);
    shell.run(cx);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "the tab closes"
    );
}

/// Closing a tab whose session is kept asks first, with Keep draft; Cancel
/// keeps the tab and the session, and Keep draft closes it, the draft let
/// go of with its text and listed for the Drafts page.
#[gpui_kit::test]
async fn closing_a_tab_with_a_kept_session_asks_first(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Closed kept", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let typed = edit_text(&host, cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(true), "kept");

    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    assert_eq!(
        shell.label("close-draft-message", cx).as_deref(),
        Some(CLOSE_KEEP_WORDS)
    );
    assert_eq!(
        shell.label("close-draft-keep", cx).as_deref(),
        Some("Keep draft")
    );
    assert!(shell.present("close-draft-discard", cx));
    assert!(shell.present("close-draft-cancel", cx));
    shell.click("close-draft-cancel-button", cx);
    assert!(!shell.present("close-draft-dialog", cx));
    assert_eq!(
        session(&shell, cx),
        Some(true),
        "the tab and its session stay"
    );

    shell.keys("secondary-w", cx);
    shell.click("close-draft-keep-button", cx);
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "the tab closes"
    );
    let store = shell.fixture.draft_store();
    assert!(matches!(held(&store, id), Some(Held::No)), "let go of");
    assert!(stored_body(&store, id) == Some(typed), "with its text");
}

/// R-15: a metadata change alone makes the tab dirty, so closing it asks,
/// though the body is untouched and, with crash recovery off, no draft is
/// stored. The tab is marked unsaved all the same. The same session clean
/// closes at once.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_metadata_change_alone_makes_closing_ask(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _elsewhere) = shell_without_a_store(cx, "Metadata");
    open(&shell, ticket(&root, TICKET), cx);
    edit_without_a_store(&shell, cx).await;
    shell.keys("secondary-w", cx);
    assert!(
        !shell.present("close-draft-dialog", cx),
        "clean: no question"
    );
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "closed at once"
    );

    open(&shell, ticket(&root, TICKET), cx);
    let host = edit_without_a_store(&shell, cx).await;
    let original = edit_text(&host, cx);
    assert_eq!(shell.rows("tab", cx), ["Metadata"], "clean: no mark");
    let fields = titled(&shell, "Another title", cx);
    pane(&shell, cx).update(cx, |pane, cx| pane.change_fields(fields, cx));
    shell.run(cx);
    assert!(edit_text(&host, cx) == original, "the body is untouched");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
    assert_eq!(
        shell.rows("tab", cx),
        ["Metadata, unsaved"],
        "the unsaved mark, with no draft stored"
    );
    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    assert!(!shell.present("close-draft-keep", cx), "no draft to keep");
    shell.click("close-draft-cancel-button", cx);
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .len(),
        1,
        "the tab stays"
    );
}

/// R-41: leaving decides on unsaved work, not on a stored draft alone.
/// With crash recovery off, a dirty tab asks first, offers no Keep draft,
/// and Discard closes it. With the draft's first write held, closing asks
/// first, and Keep draft waits for the draft to be stored, then keeps it.
#[cfg(unix)]
#[gpui_kit::test]
async fn closing_a_dirty_tab_with_no_stored_draft_asks_first(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _elsewhere) = shell_without_a_store(cx, "Unstored");
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit_without_a_store(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    assert_eq!(
        shell.label("close-draft-message", cx).as_deref(),
        Some(CLOSE_UNKEPT_WORDS)
    );
    assert!(!shell.present("close-draft-keep", cx), "no Keep draft");
    shell.click("close-draft-discard-button", cx);
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "Discard closes it"
    );

    // The first write held: nothing is stored yet.
    let (shell, root) = super::shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Storing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let begin = shell.backend.hold_read(ReadKind::Drafts);
    type_at_end(&shell, &host, "x", cx);
    let typed = edit_text(&host, cx);
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "not yet stored");
    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    assert!(shell.present("close-draft-keep", cx), "Keep draft waits");
    shell.click("close-draft-keep-button", cx);
    assert!(
        shell.present("close-draft-dialog", cx),
        "it waits for the draft to be stored"
    );
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .len(),
        1
    );

    drop(begin);
    shell.run(cx);
    let id = attached(&shell, &root, TICKET, cx).expect("stored now");
    shell.click("close-draft-keep-button", cx);
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "Keep draft closes it"
    );
    let store = shell.fixture.draft_store();
    assert!(matches!(held(&store, id), Some(Held::No)));
    assert!(stored_body(&store, id) == Some(typed));
}

// Fix round 1.

/// "b" typed and saved, with "c" typed while the save is in flight: the
/// save commits "b", and the session goes on, dirty, over the saved copy.
/// Gives the editor and the text before "b".
async fn saved_under_newer_typing(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> (Entity<EditorState>, String) {
    open(shell, ticket(root, TICKET), cx);
    let host = edit(shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(shell, &host, "b", cx);
    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    type_at_end(shell, &host, "c", cx);
    held.release();
    let request = saves(shell, cx)[0];
    ended(shell, request, cx).await;
    shell.run(cx);
    assert_eq!(code(shell, request, cx), Some(ResultCode::Ok));
    let item = shown_item(shell, root);
    assert!(item.body.as_deref() == Some(format!("{original}b").as_str()));
    assert!(edit_text(&host, cx) == format!("{original}bc"));
    (host_editor(&host, cx), original)
}

/// Important 1 (review): dirty is measured against the saved copy. After a
/// save that newer typing outlived, both edits are undone, back to the
/// text before the save. That differs from the saved copy, so the tab is
/// still dirty, and Done keeps the session with the draft holding it.
#[gpui_kit::test]
async fn undoing_past_a_save_that_newer_typing_outlived_stays_dirty(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Outlived", "");
    });
    let (editor, original) = saved_under_newer_typing(&shell, &root, cx).await;
    assert!(undo_reaches(&shell, &editor, &original, cx), "undone");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));

    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(true), "kept, not discarded");
    let id = attached(&shell, &root, TICKET, cx).expect("the draft is kept");
    assert!(body(&shell, id) == Some(original.clone()), "with that text");
    assert!(text(&read_editor(&shell, cx), cx) == format!("{original}b"));
}

/// Important 1's converse: after the same save, only the newer typing is
/// undone, back to the saved copy. The tab is clean, and Done discards the
/// draft that holds it and ends the session.
#[gpui_kit::test]
async fn undoing_back_to_the_saved_copy_after_a_save_is_clean(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Back to saved", "");
    });
    let (editor, original) = saved_under_newer_typing(&shell, &root, cx).await;
    let id = attached(&shell, &root, TICKET, cx).expect("the newer text is drafted");
    let saved = format!("{original}b");
    assert!(undo_reaches(&shell, &editor, &saved, cx), "undone");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    done(&shell, cx);
    assert_eq!(session(&shell, cx), None, "the session ends");
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert!(stored_bodies(&shell).is_empty());
}

/// Re-review N1: the tab's read after a save leaves a dirty editor alone.
/// "b" is typed and saved, and "c" typed while the save is in flight. The
/// tab's reads after it are held while both edits are undone, back to the
/// text before the save. When they land, the editor keeps that text and
/// its undo history (redo reaches "bc" again); the draft holds it, and the
/// tab reads dirty.
#[gpui_kit::test]
async fn the_read_after_a_save_leaves_a_dirty_editor_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Read late", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "b", cx);
    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    type_at_end(&shell, &host, "c", cx);
    // The workspace's reload as the save finishes, and the tab's own read
    // after it (R-70): each holds at its item read.
    let reads = [
        shell.backend.hold_read(ReadKind::Item),
        shell.backend.hold_read(ReadKind::Item),
    ];
    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    shell.run(cx);
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown_item(&shell, &root);
    assert!(item.body.as_deref() == Some(format!("{original}b").as_str()));
    let id = attached(&shell, &root, TICKET, cx).expect("the newer text is drafted");

    assert!(undo_reaches(&shell, &editor, &original, cx), "undone");
    drop(reads);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(
        edit_text(&host, cx) == original,
        "the editor keeps its text"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert!(
        body(&shell, id) == Some(original.clone()),
        "the draft holds it"
    );
    let end = edit_text(&host, cx).len();
    focus_at(&shell, &editor, end, cx);
    shell.keys("secondary-shift-z secondary-shift-z", cx);
    assert!(
        edit_text(&host, cx) == format!("{original}bc"),
        "its undo history is kept"
    );
}

/// Minor 5 (review): text typed while Done writes the draft is written too
/// before Done keeps the session: the draft holds the newest revision.
#[gpui_kit::test]
async fn text_typed_while_done_writes_is_written_before_it_keeps(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Typed during", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    type_at_end(&shell, &host, "y", cx);
    let write = shell.backend.hold_read(ReadKind::DraftWrite);
    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(false), "Done is writing");
    type_at_end(&shell, &host, "z", cx);
    drop(write);
    shell.run(cx);
    assert_eq!(session(&shell, cx), Some(true), "then kept");
    assert!(
        body(&shell, id) == Some(format!("{original}xyz")),
        "the newest"
    );
}

/// Minor 3 (review): a Drafts-page discard that the store refuses (its
/// folder cannot be written) still ends the tab's session, whose draft D2
/// let go of, but the tab does not say it was discarded: D2's banner
/// offers the draft again.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_failed_drafts_page_discard_does_not_say_discarded(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Not discarded", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    done(&shell, cx);
    assert_eq!(session(&shell, cx), Some(true), "kept");

    let read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    shell.keys("secondary-shift-d", cx);
    shell.click(row_id(id, "discard"), cx);
    shell.click("discard-draft-confirm", cx);
    shell.run(cx);
    open(&shell, ticket(&root, TICKET), cx);
    assert_eq!(session(&shell, cx), None, "D2 let the session go");
    assert!(
        !shell.present("item-draft-discarded", cx),
        "not said to be discarded"
    );
    assert!(shell.present("draft-banner", cx), "the draft is offered");
    drop(read_only);
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::No)
    ));
}

/// Minor 4 (review): while a save is in flight, the close dialog's Keep
/// draft waits, as its Discard does; once the save is over, Keep draft
/// keeps the newer text.
#[gpui_kit::test]
async fn closing_while_a_save_is_in_flight_waits_to_keep_or_discard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Closed saving", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let held_save = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held_save.wait_reached();
    type_at_end(&shell, &host, "y", cx);
    let typed = edit_text(&host, cx);

    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    for part in ["close-draft-keep-button", "close-draft-discard-button"] {
        shell.click(part, cx);
        assert!(shell.present("close-draft-dialog", cx), "{part} waits");
    }

    held_save.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    shell.run(cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the newer text is drafted");
    shell.click("close-draft-keep-button", cx);
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "Keep draft closes it"
    );
    let store = shell.fixture.draft_store();
    assert!(matches!(held(&store, id), Some(Held::No)));
    assert!(stored_body(&store, id) == Some(typed));
}

/// Minor 6 (review): the close dialog's message follows what it offers.
/// While the first write is held it offers Keep draft, and says so; once
/// that write fails (the drafts folder cannot be written), Keep draft goes,
/// and so do its words.
#[cfg(unix)]
#[gpui_kit::test]
async fn the_close_dialogs_message_follows_its_offers(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Message", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let begin = shell.backend.hold_read(ReadKind::Drafts);
    let read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    type_at_end(&shell, &host, "x", cx);
    shell.keys("secondary-w", cx);
    assert_eq!(
        shell.label("close-draft-message", cx).as_deref(),
        Some(CLOSE_KEEP_WORDS)
    );
    assert!(shell.present("close-draft-keep", cx), "Keep draft waits");

    drop(begin);
    shell.run(cx);
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "the write failed"
    );
    assert!(!shell.present("close-draft-keep", cx), "Keep draft goes");
    assert_eq!(
        shell.label("close-draft-message", cx).as_deref(),
        Some(CLOSE_UNKEPT_WORDS)
    );
    shell.click("close-draft-cancel-button", cx);
    drop(read_only);
}

/// Minor 8 (review): the never-created prompt's Discard discards the draft
/// and closes the tab, since there is no saved copy to view.
#[gpui_kit::test]
async fn discard_on_a_never_created_item_closes_the_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discarded new", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    pane(&shell, cx).update(cx, |pane, _| pane.edit_session_mut().unwrap().as_new_item());
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");

    done(&shell, cx);
    shell.click("done-draft-discard-button", cx);
    shell.run(cx);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "the tab closes"
    );
}

/// Done's rules in the design's order (Done and leaving, steps 3 and 5): a
/// never-created item asks first; then a clean session ends, discarding a
/// stored draft, and a dirty one is kept when its draft is written, and
/// asks otherwise.
#[test]
fn done_follows_the_designs_order() {
    use DoneOutcome::*;
    for (never_created, dirty, stored, written, outcome) in [
        (true, true, true, true, AskNotCreated),
        (true, true, false, false, AskNotCreated),
        (false, false, false, false, End),
        (false, false, true, true, Discard),
        (false, false, true, false, Discard),
        (false, true, true, true, Keep),
        (false, true, true, false, AskNotKept),
        (false, true, false, false, AskNotKept),
    ] {
        assert_eq!(
            done_outcome(never_created, dirty, stored, written),
            outcome,
            "{never_created} {dirty} {stored} {written}"
        );
    }
}

// Part C fix 1, M2: true words on close.

const DRAFT_TITLE: &str = "Close a tab with a draft";
const UNSAVED_TITLE: &str = "Close a tab with unsaved changes";
#[cfg(unix)]
const KEEP_FAILED_WORDS: &str = "Your newest changes could not be kept as a draft on this \
                                 computer; they are only in the editor. Keep draft tries again \
                                 and closes the tab once they are kept. Discard deletes them.";

/// M2: typed, written, then undone to the loaded text, the session is
/// clean, and its draft is stored. Closing asks, as a stored draft does,
/// in D2's words for a tab with a draft, not of unsaved changes; Keep
/// draft keeps the draft.
#[gpui_kit::test]
async fn closing_a_clean_session_with_a_stored_draft_says_it_has_a_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Undone", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    shell.keys("secondary-z", cx);
    assert!(edit_text(&host, cx) == original, "undone");
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");

    shell.keys("secondary-w", cx);
    assert!(
        shell.present("close-draft-dialog", cx),
        "a stored draft asks"
    );
    assert_eq!(
        shell.label("close-draft-message", cx).as_deref(),
        Some("Undone has a restored draft. Keep it to restore it later, or discard it.")
    );
    assert_eq!(
        shell.label("close-draft-dialog", cx).as_deref(),
        Some(DRAFT_TITLE)
    );
    shell.click("close-draft-keep-button", cx);
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "Keep draft closes it"
    );
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::No)
    ));
}

/// M2: while the draft's last write failed, the close dialog still offers
/// Keep draft, which writes again; its words say the newest changes are
/// only in the editor, not that Keep draft keeps them. A Keep draft whose
/// write fails again keeps the tab; once the store can write, it keeps
/// the newest text and closes.
#[cfg(unix)]
#[gpui_kit::test]
async fn the_close_dialog_says_when_the_last_write_failed(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Failing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    type_at_end(&shell, &host, "y", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(status(&shell, cx), DraftState::Failed, "the write failed");

    shell.keys("secondary-w", cx);
    assert_eq!(
        shell.label("close-draft-message", cx).as_deref(),
        Some(KEEP_FAILED_WORDS)
    );
    assert_eq!(
        shell.label("close-draft-dialog", cx).as_deref(),
        Some(UNSAVED_TITLE)
    );
    assert!(
        shell.present("close-draft-keep", cx),
        "Keep draft writes again"
    );
    shell.click("close-draft-keep-button", cx);
    shell.run(cx);
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .len(),
        1,
        "the write failed again: the tab stays"
    );

    drop(read_only);
    shell.keys("secondary-w", cx);
    shell.click("close-draft-keep-button", cx);
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "kept, and closed"
    );
    let typed = stored_body(&shell.fixture.draft_store(), id).expect("stored");
    assert!(typed.ends_with("xy"), "the newest text");
}
