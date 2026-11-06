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
        tabs::TabKey,
    },
    services::{JobKind, ReadKind, created_contexts::ContextKey, requests::Intent},
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id},
    },
    view::{
        drafts::row_id,
        harness::{Shell, is_save, key},
        item::LIFECYCLE_ACTION,
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

/// Presses what the banners' Resume and Retry do, past their disabled
/// buttons, in the handlers themselves.
fn press_owed(shell: &Shell, cx: &mut TestAppContext) {
    pane(shell, cx).update(cx, |pane, cx| pane.press_owed_for_test(cx));
}

/// Presses the lifecycle action of the locked ticket, and waits for the
/// held close's own answer in the dialog: the dialog neither saves first
/// nor closes before that answer is known.
async fn press_lifecycle_locked(shell: &Shell, cx: &mut TestAppContext) {
    let sent = edits_sent(shell, cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(shell, "lifecycle-continue", cx).await;
    shell.run(cx);
    assert_eq!(edits_sent(shell, cx), sent, "no save or prepare of edits");
    assert!(
        !shell.backend.jobs_seen().contains(&JobKind::TicketSave),
        "nothing is saved"
    );
    assert!(shell.present("lifecycle-dialog", cx), "the dialog stays");
}

/// R-23, review V4: the lifecycle action stays available under the lock,
/// but its save first writes nothing into the merged context: a session
/// kept with unsaved text is not saved, and the dialog goes on to the
/// held close's own way on.
#[gpui_kit::test]
async fn the_lifecycle_actions_save_first_writes_nothing_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Held", ""));
    dirty_session(&shell, &root, cx).await;
    locked_from_outside(&shell, &root, cx).await;
    shell
        .until_drawn(cx, "the session kept", |shell, cx| {
            shell.present("item-kept-continue-locked", cx)
        })
        .await;

    press_lifecycle_locked(&shell, cx).await;
    assert!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.edit_session_kept()),
        "the unsaved text stays kept"
    );
}

/// R-23, review V4: a draft of the ticket stored with no session is not
/// continued and saved by the lifecycle action while the ticket is locked.
#[gpui_kit::test]
async fn a_stored_drafts_save_first_writes_nothing_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Held", "");
    fixture.refresh(&root);
    let draft = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Stored body");
    strand_close(&fixture, &root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    until_lock(&shell, &root, LockState::Locked(LockSource::Record), cx).await;

    press_lifecycle_locked(&shell, cx).await;
    assert!(
        crate::desktop::test_support::drafts::stored_body(&shell.fixture.draft_store(), draft)
            .is_some(),
        "the draft stays stored"
    );
}

/// Review V7: the prepare banner's Resume sends nothing while the item is
/// locked, and says so by waiting.
#[gpui_kit::test]
async fn the_prepare_banners_resume_waits_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Pending", "");
    });
    open(&shell, ticket(&root), cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap(),
    );
    shell.click("item-edit", cx);
    let prepare = |shell: &Shell, cx: &mut TestAppContext| {
        shell.requests(cx, |request| {
            matches!(request.intent, Intent::ContextPrepare { .. })
        })
    };
    let request = prepare(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(shell.present("item-prepare-resume", cx));
    let attempt = |shell: &Shell, cx: &mut TestAppContext| {
        shell.operations(cx).read_with(cx, |operations, _| {
            operations.request(request).unwrap().attempt
        })
    };
    let before = attempt(&shell, cx);
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());

    shell.lock_item(&root, TICKET, cx);
    assert!(
        shell.inert_button("item-prepare-resume", cx),
        "Resume waits"
    );
    shell.click("item-prepare-resume", cx);
    press_owed(&shell, cx);
    shell.run(cx);
    assert_eq!(attempt(&shell, cx), before, "nothing was sent again");
    assert_eq!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase()),
        Phase::Loading,
        "no editor opens on a locked item"
    );
    assert!(shell.present("item-prepare-resume", cx), "still owed");
}

/// Review V6: a pending checkpoint's Retry sends nothing while the item is
/// locked: it would commit into the merged context.
#[gpui_kit::test]
async fn a_pending_checkpoints_retry_waits_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Held", ""));
    dirty_session(&shell, &root, cx).await;
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    shell.keys("secondary-s", cx);
    let saves = |shell: &Shell, cx: &mut TestAppContext| shell.requests(cx, |r| is_save(&r.intent));
    let request = saves(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(shell.present("item-save-retry", cx), "kept for Retry");
    let attempt = |shell: &Shell, cx: &mut TestAppContext| {
        shell.operations(cx).read_with(cx, |operations, _| {
            operations.request(request).unwrap().attempt
        })
    };
    let before = attempt(&shell, cx);

    shell.lock_item(&root, TICKET, cx);
    assert!(shell.inert_button("item-save-retry", cx), "Retry waits");
    shell.click("item-save-retry", cx);
    press_owed(&shell, cx);
    shell.run(cx);
    assert_eq!(attempt(&shell, cx), before, "nothing was sent again");
}

/// Review Vdraft: a clean session whose draft D2 still holds, when the lock
/// ends the session, discards it, as Done does (D3 Done, step 5), rather
/// than leaving it to show "Restored draft" on a closed item. Review F1:
/// the context Edit made here is left as it is: release on leave is told
/// nothing, so no release of the locked item is sent.
#[gpui_kit::test]
async fn a_clean_session_ended_by_the_lock_discards_its_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Held", "")
    });
    dirty_session(&shell, &root, cx).await;
    let created = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().created_contexts().clone()
    });
    let context = ContextKey::new(&key(&root), ItemDtoKind::Ticket, TICKET);
    assert!(
        created.read_with(cx, |created, _| created.contains(&context)),
        "Edit made the context here"
    );
    let tab = TabKey::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: TICKET.to_owned(),
    };
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    shell
        .until_drawn(cx, "the draft began", |_, cx| {
            drafts.read_with(cx, |registry, _| registry.attached(&tab).is_some())
        })
        .await;
    let draft = drafts
        .read_with(cx, |registry, _| registry.attached(&tab))
        .unwrap();
    // The typed text is undone: the session is clean, its draft still held.
    shell.keys("backspace", cx);

    locked_from_outside(&shell, &root, cx).await;
    shell
        .until_drawn(cx, "the session ended", |shell, cx| {
            !pane(shell, cx).read_with(cx, |pane, _| pane.has_session())
        })
        .await;
    shell
        .until_drawn(cx, "the draft discarded", |shell, _| {
            crate::desktop::test_support::drafts::stored_body(&shell.fixture.draft_store(), draft)
                .is_none()
        })
        .await;
    assert!(
        drafts.read_with(cx, |registry, _| registry.attached(&tab).is_none()),
        "no draft stays attached to the closed item's tab"
    );
    // Release on leave would be decided once the store is listed again.
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let releases = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ContextRelease { .. })
    });
    assert!(releases.is_empty(), "no release of the locked item");
}

/// Review F4: the prepare banner's Retry, offered by a refused prepare,
/// sends nothing while the item is locked, as Resume does not.
#[gpui_kit::test]
async fn the_prepare_banners_retry_waits_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Refused", "");
    });
    open(&shell, ticket(&root), cx);
    let refused = manyhands::results::Envelope::failure(
        "context prepare",
        manyhands::results::Scope::default(),
        ResultCode::InternalError,
        vec![manyhands::results::RecoveryAction {
            action: manyhands::results::RecoveryActionKind::RequestRetry,
            operation_id: None,
            arguments: serde_json::Map::new(),
        }],
    );
    shell
        .backend
        .hold_job_return_answering(JobKind::ContextPrepare, refused)
        .release();
    shell.click("item-edit", cx);
    let request = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ContextPrepare { .. })
    })[0];
    shell.until_ended(request, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(shell.present("item-prepare-retry", cx));
    let attempt = |shell: &Shell, cx: &mut TestAppContext| {
        shell.operations(cx).read_with(cx, |operations, _| {
            operations.request(request).unwrap().attempt
        })
    };
    let before = attempt(&shell, cx);

    shell.lock_item(&root, TICKET, cx);
    assert!(shell.inert_button("item-prepare-retry", cx), "Retry waits");
    shell.click("item-prepare-retry", cx);
    press_owed(&shell, cx);
    shell.run(cx);
    assert_eq!(attempt(&shell, cx), before, "nothing was sent again");
}

/// Review F4: a save's Resume, owed after its hand-off to the index failed,
/// resumes nothing while the item is locked.
#[gpui_kit::test]
async fn a_save_banners_resume_waits_while_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Held", ""));
    dirty_session(&shell, &root, cx).await;
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap(),
    );
    shell.keys("secondary-s", cx);
    let request = shell.requests(cx, |r| is_save(&r.intent))[0];
    shell.until_ended(request, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(shell.present("item-save-resume", cx), "Resume is offered");
    let resumes = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .requests(cx, |r| matches!(r.intent, Intent::OperationResume { .. }))
            .len()
    };
    let before = resumes(&shell, cx);

    shell.lock_item(&root, TICKET, cx);
    assert!(shell.inert_button("item-save-resume", cx), "Resume waits");
    shell.click("item-save-resume", cx);
    press_owed(&shell, cx);
    shell.run(cx);
    assert_eq!(resumes(&shell, cx), before, "nothing was resumed");
}
