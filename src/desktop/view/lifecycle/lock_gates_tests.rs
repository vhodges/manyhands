//! The lifecycle lock's gates on the item's other edits (D5 design, A2;
//! rulings R-21, R-23), over real local-only repositories: Save while
//! nothing is known yet, a dirty session kept when a close locks the item,
//! the close dialog's Save, D2's restored draft's Save and Continue
//! editing, and Move…. A close "of another process" is the library's,
//! through a service of its own, as `lock_restart_tests.rs` makes one.

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{FailurePoint, ItemDtoKind, RepositoryService, RequestId},
    results::ResultCode,
};

use super::{
    lock_restart_tests::{focus_again, promote_through},
    lock_tests::{CLOSING, MERGING, lock, until_lock},
    outcome_canned_tests::{finished, intent},
    tests::{
        DOCUMENT, TICKET, edit, open, pane, saved_ticket, shell, strand_close, ticket,
        until_present,
    },
};
use crate::desktop::{
    model::{
        edit::Phase,
        lifecycle_lock::{LockSource, LockState},
        navigation::OpenTarget,
    },
    services::{ReadKind, requests::Intent},
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id},
    },
    view::{
        drafts::row_id,
        harness::{Shell, is_save, key},
    },
};

/// The saves and context prepares this session sent.
fn edits_sent(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .requests(cx, |request| {
            is_save(&request.intent) || matches!(request.intent, Intent::ContextPrepare { .. })
        })
        .len()
}

/// The ticket's tab with a session holding unsaved text: Edit, then typed.
async fn dirty_session(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    open(shell, ticket(root), cx);
    let host = edit(shell, cx).await;
    shell.type_at_end(&host, "x", cx);
    assert_eq!(
        pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase()),
        Phase::Editing
    );
}

/// The CLI strands a close of the ticket, and the window takes focus
/// again: the ticket's record locks it.
async fn locked_from_outside(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    strand_close(&shell.fixture, root);
    focus_again(shell, cx);
    until_lock(shell, root, LockState::Locked(LockSource::Record), cx).await;
}

/// R-21, M-4: while a lifecycle answer of the ticket waits for its rows to
/// be read again, nothing is known of its lock, and the session's Save
/// (`secondary-s`) sends nothing; once the rows are read, it saves. The
/// answer is canned, before acceptance (`busy`,
/// `tests/mutation_remote/c5_lifecycle.rs`
/// `c5_lifecycle_busy_with_no_parent_close`); nothing is asserted of the
/// repository by it.
#[gpui_kit::test]
async fn save_waits_while_nothing_is_known(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Unknown", "")
    });
    dirty_session(&shell, &root, cx).await;
    let rows = shell.backend.hold_read(ReadKind::Operations);
    let answer = super::lock_restart_tests::refused_busy(&root);
    finished(
        &shell,
        1,
        intent(&root, ItemDtoKind::Ticket),
        RequestId::new(),
        answer,
        cx,
    );
    assert_eq!(lock(&shell, &root, cx), LockState::Awaiting);
    let sent = edits_sent(&shell, cx);
    shell.keys("secondary-s", cx);
    assert_eq!(edits_sent(&shell, cx), sent, "Save waits");

    drop(rows);
    until_lock(&shell, &root, LockState::Open, cx).await;
    shell.keys("secondary-s", cx);
    assert_eq!(edits_sent(&shell, cx), sent + 1, "then it saves");
}

/// R-23: a session holding unsaved text when another process's close locks
/// the ticket is kept, not ended: the tab views the saved copy, and its
/// Continue editing waits, saying "Closing…".
#[gpui_kit::test]
async fn a_dirty_session_is_kept_when_a_close_locks_the_item(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Kept", ""));
    dirty_session(&shell, &root, cx).await;
    locked_from_outside(&shell, &root, cx).await;
    shell
        .until_drawn(cx, "the session kept", |shell, cx| {
            shell.present("item-kept-continue-locked", cx)
        })
        .await;
    let (phase, held) =
        pane(&shell, cx).read_with(cx, |pane, _| (pane.edit_phase(), pane.edit_session_kept()));
    assert_eq!(phase, Phase::Viewing);
    assert!(held, "the session is kept, with its text");
    assert_eq!(
        shell.label("item-kept-continue-locked", cx).as_deref(),
        Some(CLOSING)
    );
    shell.click("item-kept-continue", cx);
    assert_eq!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase()),
        Phase::Viewing,
        "Continue editing waits"
    );
}

/// I-1, R-23: closing the tab of a kept session while the ticket is locked
/// asks first, as ever, but its Save waits, saying "Closing…", and a press
/// sends nothing.
#[gpui_kit::test]
async fn the_close_dialogs_save_waits_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Closed tab", "")
    });
    dirty_session(&shell, &root, cx).await;
    locked_from_outside(&shell, &root, cx).await;
    shell
        .until_drawn(cx, "the session kept", |shell, cx| {
            shell.present("item-kept-continue-locked", cx)
        })
        .await;
    let sent = edits_sent(&shell, cx);

    shell.click(("tab-close", 0usize), cx);
    until_present(&shell, "close-draft-dialog", cx).await;
    assert_eq!(
        shell.label("close-draft-save-state", cx).as_deref(),
        Some(CLOSING)
    );
    shell.click("close-draft-save-button", cx);
    shell.run(cx);
    assert_eq!(edits_sent(&shell, cx), sent, "Save sent nothing");
    assert!(shell.present("close-draft-dialog", cx), "the tab stays");
}

/// I-1, M-3, R-23: a draft restored into the ticket's tab before another
/// process's close locks it offers Save and Continue editing, both
/// waiting with "Closing…"; a press sends nothing.
#[gpui_kit::test]
async fn a_restored_drafts_save_and_continue_wait_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Restored", "");
    fixture.refresh(&root);
    let draft = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    shell.click(row_id(draft, "restore"), cx);
    until_present(&shell, "draft-save", cx).await;
    locked_from_outside(&shell, &root, cx).await;
    let sent = edits_sent(&shell, cx);

    assert_eq!(shell.label("draft-save-note", cx).as_deref(), Some(CLOSING));
    assert_eq!(
        shell.label("draft-continue-note", cx).as_deref(),
        Some(CLOSING)
    );
    shell.click("draft-save-button", cx);
    shell.click("draft-continue-button", cx);
    shell.run(cx);
    assert_eq!(edits_sent(&shell, cx), sent, "nothing is sent");
}

/// R-23: while a document's merge, stopped after main, is unfinished, its
/// sheet's Move… waits, saying "Merging…", and a press opens nothing.
#[gpui_kit::test]
async fn move_waits_while_the_merge_is_unfinished(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, "docs/plan.md", "Plan");
    fixture.refresh(&root);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        fixture.data_dir(),
        FailurePoint::AfterPublication,
    )
    .unwrap();
    let stopped = promote_through(&failing, &root);
    assert_ne!(stopped.code, ResultCode::Ok, "{}", stopped.message);
    drop(failing);
    let shell = Shell::open(fixture, cx);
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let repo = key(&root);
    shell
        .until_drawn(cx, "the merge's lock", move |_, cx| {
            cx.read(|cx| services.lifecycle_lock(&repo, DOCUMENT, cx))
                == LockState::Locked(LockSource::Record)
        })
        .await;

    shell.click("chip-all", cx);
    assert_eq!(shell.label("sheet-move-note", cx).as_deref(), Some(MERGING));
    shell.click("sheet-move-button", cx);
    let open = shell.frame(cx, |window, cx| {
        gpui_kit::component::WindowExt::has_active_dialog(window, cx)
    });
    assert!(!open, "a press opens nothing");
}
