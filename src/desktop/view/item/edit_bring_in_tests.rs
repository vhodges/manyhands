//! Unsaved work across a bring-in, over real repositories (D6 design A14,
//! O-4, R-10, Fact 17; Cycle acceptance 5 and 9).
//!
//! A poll that fast-forwards or materializes an item reaches the screen
//! through `Operations::brought_in`, the entry point the scheduler calls
//! when an attempt's answer lands; every tab of the repository then reads
//! again. These tests call that entry point after the item's file has
//! changed on primary, so the whole path from the poll's answer to the
//! editor is the real one. The reload itself (`late_read_landed`, the
//! change banner, `external_change`) is `edit_restored_tests`'s,
//! `edit_done_tests`'s and `review_tests`'s, which start from the tab's own
//! `reload`; what is new here is that a bring-in starts it.
//!
//! The helpers are `edit_tests`'s. Each test types through the window and
//! waits on the worker, so it allows parking. The words are literals: the
//! tests fix them.

use super::*;

const CHANGED_WORDS: &str = "Someone changed this file outside Manyhands. Your edits are kept.";
const NEWER: &str = "A newer body, brought in by a poll.\n";

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

/// The body the shown tab last read of its item.
fn tab_body(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    pane(shell, cx).read_with(cx, |pane, _| match pane.model.item_read() {
        Some(Ok(item)) => item.body.clone(),
        _ => None,
    })
}

/// The save requests of this session, newest first.
fn saves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketSave { .. })
    })
}

/// The ticket `id`'s file changes on disk, in the branch its context holds,
/// as a fast-forward of it leaves it: `body` in place of its own, under a
/// new token.
fn file_moves(shell: &Shell, root: &Path, id: &str, title: &str, body: &str) {
    shell.fixture.save_ticket_with_body(root, id, title, body);
}

/// A poll's answer: `root` had items brought in (R-10).
fn brought_in(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    shell
        .operations(cx)
        .update(cx, |operations, cx| operations.brought_in(key(root), cx));
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// A14, O-4: an open edit session holding unsaved edits, its item's file
/// changed on disk, the repository marked changed as a poll's answer does.
/// The tab reads again and holds what it read; the editor keeps the typed
/// text, the draft keeps it too, and the changed notice says so.
#[gpui_kit::test]
async fn bring_in_keeps_dirty_session(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    let draft = attached(&shell, &root, TICKET, cx).expect("a draft began");
    let typed = format!("{original}x");
    assert!(!shell.present("item-save-changed", cx), "nothing changed");
    assert!(tab_body(&shell, cx).as_deref() == Some(original.as_str()));

    file_moves(&shell, &root, TICKET, "Mine", NEWER);
    assert!(
        tab_body(&shell, cx).as_deref() == Some(original.as_str()),
        "the tab has not read again: only the bring-in makes it"
    );
    brought_in(&shell, &root, cx);

    assert!(
        tab_body(&shell, cx).as_deref() == Some(NEWER),
        "the tab read the brought-in file"
    );
    assert_eq!(phase(&shell, cx), Phase::Editing, "nothing opens by itself");
    assert_eq!(session(&shell, cx), Some(false), "the session lives");
    assert!(edit_text(&host, cx) == typed, "the editor keeps the text");
    assert_eq!(
        shell.label("item-save-changed", cx).as_deref(),
        Some(CHANGED_WORDS),
        "the changed notice"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
    assert_eq!(attached(&shell, &root, TICKET, cx), Some(draft));
    assert!(
        crate::desktop::test_support::drafts::stored_body(&shell.fixture.draft_store(), draft)
            == Some(typed),
        "the draft keeps the text"
    );
}

/// A14, O-4: a bring-in under a kept draft shows the changed banner
/// (`TargetState::Changed`) and leaves the kept text; a clean session takes
/// the brought-in read; and a save over the moved file answers
/// `external_change`, opens the review and keeps the edits.
#[gpui_kit::test]
async fn bring_in_kept_draft_banner(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
        fixture.commit_ticket(root, OTHER, "Clean", "");
        fixture.commit_ticket(root, THIRD, "Saving", "");
    });

    // A kept draft: the banner, and the kept editor's text untouched.
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let kept_text = edit_text(&host, cx);
    shell.click("item-done", cx);
    shell.run(cx);
    assert_eq!(session(&shell, cx), Some(true), "kept");
    assert!(!shell.present("item-kept-changed", cx), "nothing yet");
    file_moves(&shell, &root, TICKET, "Kept", NEWER);
    brought_in(&shell, &root, cx);
    assert_eq!(
        shell.label("item-kept-changed", cx).as_deref(),
        Some(CHANGED_WORDS)
    );
    assert_eq!(session(&shell, cx), Some(true), "still kept");
    assert!(edit_text(&host, cx) == kept_text, "the kept text stays");
    assert!(tab_body(&shell, cx).as_deref() == Some(NEWER));

    // A clean session takes the new read.
    open(&shell, ticket(&root, OTHER), cx);
    let clean = edit(&shell, cx).await;
    assert!(edit_text(&clean, cx) != NEWER);
    file_moves(&shell, &root, OTHER, "Clean", NEWER);
    brought_in(&shell, &root, cx);
    assert!(edit_text(&clean, cx) == NEWER, "the clean editor reloaded");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");
    assert!(!shell.present("item-save-changed", cx), "no notice");

    // A save over the moved file: `external_change`, the edits kept.
    open(&shell, ticket(&root, THIRD), cx);
    let dirty = edit(&shell, cx).await;
    type_at_end(&shell, &dirty, "y", cx);
    let mine = edit_text(&dirty, cx);
    file_moves(&shell, &root, THIRD, "Saving", NEWER);
    brought_in(&shell, &root, cx);
    assert_eq!(
        shell.label("item-save-changed", cx).as_deref(),
        Some(CHANGED_WORDS)
    );
    let before = saves(&shell, cx).len();
    shell.click("item-save", cx);
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), before + 1, "Save sent one save");
    ended(&shell, sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::ExternalChange));
    assert_eq!(phase(&shell, cx), Phase::Review, "the review opens");
    assert!(edit_text(&dirty, cx) == mine, "the edits are kept");
    assert!(
        shown(&shell, &root, THIRD).body.as_deref() == Some(NEWER),
        "their file stands"
    );
}
