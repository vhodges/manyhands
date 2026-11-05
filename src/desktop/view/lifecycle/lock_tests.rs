//! The lifecycle lock while a run of this session goes on (D5 design, A2
//! "Read-only while unfinished"; P20, P23, P25; rulings R-21 to R-23),
//! over real local-only repositories: the dialog's lock, every tab and
//! window of the item, a stop before the closure checkpoint (L-1), a
//! refused tidy-up after the merge (L-3), one refused for a change the
//! person can undo (L-4), and a finished or missing row (L-5). The restart,
//! the CLI and the pre-acceptance answers are `lock_restart_tests.rs`.
//!
//! The stage holds park the worker once the stage is sent, and the library
//! may hold the repository's lease meanwhile: no test reads the repository
//! while one is parked.

use std::path::Path;

use gpui_kit::{
    Entity, Focusable, TestAppContext, VisualTestContext, WindowHandle, component::Root,
    test::TestWindowExt,
};
use manyhands::{
    repository::{
        FailurePoint, ItemDtoKind, LifecycleCleanupPendingReason, LifecycleCleanupState,
        OperationAction, ProgressStage, RepositoryService, RequestId,
    },
    results::{CleanupEffect, Outcome, ResultCode},
};

use super::{
    confirm_tests::{confirming, worktree},
    outcome_canned_tests::{answer, finished, intent, reached, reached_effects, retry},
    outcome_tests::at_stage,
    tests::{TICKET, closes, code, open, saved_ticket, shell, shown, ticket, until_present},
};
use crate::desktop::{
    model::lifecycle_lock::{LockSource, LockState, Untidied},
    services::{JobKind, ReadKind},
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
        workspace::Workspace,
    },
};

pub(super) const CLOSING: &str = "Closing\u{2026}";
pub(super) const MERGING: &str = "Merging\u{2026}";
pub(super) const NOT_TIDIED: &str = "Merged, but not tidied up: the working copy changed";
pub(super) const LOCKED_NOTE: &str = "item-locked-note";
const SHARE_LOCKED: &str = "item-share-locked";

/// Where the ticket of `root` stands under the lock.
pub(super) fn lock(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> LockState {
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let root = key(root);
    cx.read(|cx| services.lifecycle_lock(&root, TICKET, cx))
}

/// Draws frames until the ticket of `root` stands as `expected`.
pub(super) async fn until_lock(
    shell: &Shell,
    root: &Path,
    expected: LockState,
    cx: &mut TestAppContext,
) {
    let root = root.to_owned();
    shell
        .until_drawn(cx, &format!("the lock {expected:?}"), move |shell, cx| {
            lock(shell, &root, cx) == expected
        })
        .await;
}

/// The step of the ticket's lifecycle row, as the library lists it.
pub(super) fn step(shell: &Shell, root: &Path) -> Option<String> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .find(|row| row.action == OperationAction::Close)
        .map(|row| row.state)
}

/// The tab says `words` in its pill, and its Edit, where the item still
/// offers one (a ticket the close has marked closed does not), waits with
/// the same words: pressed, it starts nothing. (The Kit's button reports
/// no disabled flag: `Shell::inert_button`.)
pub(super) fn assert_edits_wait(shell: &Shell, words: &str, cx: &mut TestAppContext) {
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(words));
    if shell.present("item-edit", cx) {
        assert_eq!(shell.label(LOCKED_NOTE, cx).as_deref(), Some(words));
    }
    let prepares = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .requests(cx, |request| {
                matches!(
                    request.intent,
                    crate::desktop::services::requests::Intent::ContextPrepare { .. }
                )
            })
            .len()
    };
    let before = prepares(shell, cx);
    if shell.present("item-edit", cx) {
        shell.click("item-edit", cx);
    }
    assert_eq!(prepares(shell, cx), before, "Edit started nothing");
    assert!(
        super::tests::pane(shell, cx).read_with(cx, |pane, _| pane.edit_host().is_none()),
        "no session"
    );
}

/// The tab's Edit is live, with no lock note.
pub(super) fn assert_edits_live(shell: &Shell, cx: &mut TestAppContext) {
    assert!(shell.present("item-edit", cx), "Edit is offered");
    assert!(!shell.present(LOCKED_NOTE, cx));
}

/// Nothing locks the tab: no lock note (a closed ticket offers no Edit).
pub(super) fn assert_no_lock(shell: &Shell, cx: &mut TestAppContext) {
    assert!(!shell.present(LOCKED_NOTE, cx));
}

/// The label of `id` in `window`, after a frame.
fn label_in(window: WindowHandle<Root>, id: &str, cx: &mut TestAppContext) -> Option<String> {
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    let id = gpui_kit::ElementId::from(id.to_owned());
    visual.update(|window, cx| {
        window.render_frame(cx);
        window
            .try_find(id)
            .and_then(|element| element.label().map(str::to_owned))
    })
}

/// A2, R-21: the dialog locks the item from Saving through Preparing to
/// Confirm, and lets go once it is dropped by Escape.
#[gpui_kit::test]
async fn the_dialog_locks_from_saving_and_releases_when_dropped(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Held", ""));
    open(&shell, ticket(&root), cx);
    assert_eq!(lock(&shell, &root, cx), LockState::Open);
    let host = super::tests::edit(&shell, cx).await;
    shell.type_at_end(&host, "x", cx);

    let save = shell.backend.hold_job(JobKind::TicketSave);
    shell.click(LIFECYCLE_ACTION, cx);
    save.wait_reached();
    let dialog = LockState::Locked(LockSource::Dialog);
    assert_eq!(lock(&shell, &root, cx), dialog, "Saving");
    let prepare = shell.backend.hold_read(ReadKind::Prepare);
    save.release();
    shell
        .until_drawn(cx, "the prepare", |shell, cx| {
            shell.label("lifecycle-state", cx).as_deref() == Some(super::PREPARING)
        })
        .await;
    assert_eq!(lock(&shell, &root, cx), dialog, "Preparing");
    drop(prepare);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert_eq!(lock(&shell, &root, cx), dialog, "Confirm");

    shell.keys("escape", cx);
    until_lock(&shell, &root, LockState::Open, cx).await;
    assert!(closes(&shell, cx).is_empty());
}

/// P20, R-23, real: while the close runs, the ticket's tab in this window
/// and in a second window over the same services is read-only, saying
/// "Closing…": Edit and Share wait. Once the close is done, neither tab
/// says so.
#[gpui_kit::test]
async fn every_tab_and_window_of_the_item_is_read_only(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Windows", "")
    });
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let second = cx.update(|cx| crate::desktop::open_window_over_services(services, cx));
    let other: Entity<Workspace> = second
        .update(cx, |root, _, _| root.view().clone().downcast().unwrap())
        .unwrap();
    other.update(cx, |workspace, cx| workspace.open_item(ticket(&root), cx));
    VisualTestContext::from_window(second.into(), cx).run_until_parked();
    assert_eq!(label_in(second, LOCKED_NOTE, cx), None);

    confirming(&shell, &root, cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Recording);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = closes(&shell, cx)[0];
    at_stage(&shell, id, ProgressStage::Recording, cx).await;
    assert_eq!(
        lock(&shell, &root, cx),
        LockState::Locked(LockSource::Worker)
    );
    assert_edits_wait(&shell, CLOSING, cx);
    assert_eq!(
        shell.label(SHARE_LOCKED, cx).as_deref(),
        Some(CLOSING),
        "Share waits"
    );
    assert_eq!(label_in(second, LOCKED_NOTE, cx).as_deref(), Some(CLOSING));
    assert_eq!(label_in(second, SHARE_LOCKED, cx).as_deref(), Some(CLOSING));
    assert_eq!(label_in(second, "item-pill", cx).as_deref(), Some(CLOSING));

    gate.release();
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok));
    until_lock(&shell, &root, LockState::Open, cx).await;
    shell.keys("escape", cx);
    assert_no_lock(&shell, cx);
    assert_eq!(label_in(second, LOCKED_NOTE, cx), None);
    assert_eq!(label_in(second, SHARE_LOCKED, cx), None);
}

/// R-23, A10, real: while the close runs, a comment's Submit is refused
/// with "Closing…", nothing is posted, and its text is kept: typing goes
/// on.
#[gpui_kit::test]
async fn a_comment_waits_while_the_close_runs(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Comment", "")
    });
    open(&shell, ticket(&root), cx);
    shell.click("item-comments", cx);
    shell.click("new-comment-start", cx);
    let composer = super::tests::pane(&shell, cx)
        .read_with(cx, |pane, _| pane.new_composer())
        .expect("the new comment's composer");
    let editor = composer.read_with(cx, |composer, _| composer.editor().clone());
    editor.update(cx, |editor, cx| editor.set_cursor(0, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input("Kept");
    shell.run(cx);

    confirming(&shell, &root, cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Recording);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = closes(&shell, cx)[0];
    at_stage(&shell, id, ProgressStage::Recording, cx).await;
    composer.update(cx, |composer, cx| composer.post(cx));
    assert_eq!(
        shell.label("composer-refused", cx).as_deref(),
        Some(CLOSING)
    );
    assert_eq!(
        composer.read_with(cx, |composer, cx| composer.text(cx)),
        "Kept",
        "the comment's text is kept"
    );
    assert!(
        shell
            .requests(cx, |request| matches!(
                request.intent,
                crate::desktop::services::requests::Intent::CommentAdd { .. }
            ))
            .is_empty(),
        "no comment is posted"
    );
    gate.release();
    shell.until_ended(id, cx).await;
}

/// L-1, P25, real: a stop before the closure checkpoint, held at
/// `observing` and stopped there, leaves no locking row: the run's own
/// lock goes once it ends, and the ticket is editable again.
#[gpui_kit::test]
async fn a_stop_before_closing_unlocks(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Stopped", "")
    });
    confirming(&shell, &root, cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Observing);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = closes(&shell, cx)[0];
    at_stage(&shell, id, ProgressStage::Observing, cx).await;
    assert_eq!(
        lock(&shell, &root, cx),
        LockState::Locked(LockSource::Worker)
    );
    until_present(&shell, "lifecycle-stop", cx).await;
    shell.click("lifecycle-stop", cx);
    gate.release();
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Cancelled));
    let left = step(&shell, &root);
    assert!(
        matches!(
            left.as_deref(),
            None | Some("confirmed" | "context_synchronized")
        ),
        "stopped before closing: {left:?}"
    );

    until_lock(&shell, &root, LockState::Open, cx).await;
    shell.keys("escape", cx);
    assert_edits_live(&shell, cx);
}

/// L-3, P23, real: after the merge, a tidy-up the library refuses (a
/// service that fails while removing the working copy) leaves the row at
/// `worktree_removing`: the ticket stays read-only, Retry stops there
/// again, and the working copy's branch never moves meanwhile. Canned, as
/// a remote outcome the desktop cannot reach: a refused remote deletion
/// over that row (`tests/mutation_remote/c5_lifecycle.rs`
/// `deletion_rejected`, `tests/fixtures/mutation_v1/
/// ticket_close_deletion_rejected.json`) does not unlock it either; nothing
/// is asserted of the repository after it.
#[gpui_kit::test]
async fn after_integration_a_refused_remote_deletion_stays_locked(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Refused", "")
    });
    let branch = shown(&shell, &root, TICKET).context.branch.unwrap();
    let context = worktree(&shell, &root);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::DuringWorktreeRemoval,
        )
        .unwrap(),
    );
    confirming(&shell, &root, cx).await;
    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_ne!(code(&shell, id, cx), Some(ResultCode::Ok));
    assert_eq!(step(&shell, &root).as_deref(), Some("worktree_removing"));
    let tip = git2::Repository::open(&root)
        .unwrap()
        .refname_to_id(&format!("refs/heads/{branch}"))
        .unwrap();

    let locked = LockState::Locked(LockSource::Record);
    until_lock(&shell, &root, locked, cx).await;
    until_present(&shell, "lifecycle-retry", cx).await;
    // The failure point fires once a service: a new one stops the Retry
    // at the same place.
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::DuringWorktreeRemoval,
        )
        .unwrap(),
    );
    shell.click("lifecycle-retry", cx);
    // Retry sends the same request ID again, as a request of its own.
    let again = closes(&shell, cx)[0];
    assert_ne!(again, id);
    shell.until_ended(again, cx).await;
    assert_ne!(code(&shell, again, cx), Some(ResultCode::Ok));
    until_lock(&shell, &root, locked, cx).await;
    shell.keys("escape", cx);
    assert_edits_wait(&shell, CLOSING, cx);
    assert_eq!(step(&shell, &root).as_deref(), Some("worktree_removing"));
    let now = git2::Repository::open(&root)
        .unwrap()
        .refname_to_id(&format!("refs/heads/{branch}"))
        .unwrap();
    assert_eq!(now, tip, "the working copy's branch never moved");
    assert!(context.exists(), "the working copy is kept");

    let request = RequestId::new();
    let mut data = reached(ItemDtoKind::Ticket, TICKET, Vec::new());
    data.cleanup = Some(LifecycleCleanupState::Pending);
    data.cleanup_pending = Some(LifecycleCleanupPendingReason::RemoteDeletionRejected);
    let refused = answer(
        "ticket close",
        &root,
        ResultCode::PushRejected,
        Outcome::Partial,
        data,
        reached_effects(CleanupEffect::Pending),
        vec![retry(request)],
        request,
    );
    finished(
        &shell,
        1,
        intent(&root, ItemDtoKind::Ticket),
        request,
        refused,
        cx,
    );
    until_lock(&shell, &root, locked, cx).await;
    assert!(!shell.present("item-not-tidied", cx));
}

/// L-4, decision 21, K-13, real: a file left in the working copy while the
/// close tidies up stops the tidy-up after the merge (`context_changed`,
/// the recipe of `tests/mutation_remote/c5_cancel.rs`
/// `stopped_on_context_changed`). The ticket is editable, and says
/// "Merged, but not tidied up" with Open folder and Check again. Check
/// again with the file still there says so again; with it gone, the close
/// is tidied up and the item reads as any other.
#[gpui_kit::test]
async fn a_changed_context_cleanup_unlocks_with_check_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Untidied", "")
    });
    let context = worktree(&shell, &root);
    confirming(&shell, &root, cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::CleaningUp);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let left = context.join("untracked.txt");
    std::fs::write(&left, b"the owner's\n").unwrap();
    gate.release();
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::RecoveryRequired));
    let untidied = LockState::Untidied(Untidied {
        reason: LifecycleCleanupPendingReason::ContextChanged,
        request: id,
    });
    until_lock(&shell, &root, untidied, cx).await;
    shell.keys("escape", cx);
    assert_no_lock(&shell, cx);
    assert_eq!(
        shell.label("item-not-tidied", cx).as_deref(),
        Some(NOT_TIDIED)
    );
    // Open folder opens the working copy, not main (M-8).
    shell.click("item-not-tidied-open-folder", cx);
    let opened = cx.read(|cx| {
        cx.try_global::<crate::desktop::view::item::OpenedFolders>()
            .map(|opened| opened.0.clone())
    });
    assert_eq!(opened, Some(vec![context.clone()]));
    assert_ne!(context, root);

    shell.click("item-not-tidied-check-again", cx);
    let attempt = |n: u32| {
        move |operations: &crate::desktop::model::operations::Operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == n
                    && matches!(
                        request.state,
                        crate::desktop::services::requests::RequestState::Finished(..)
                    )
            })
        }
    };
    shell.until(cx, attempt(2)).await;
    until_lock(&shell, &root, untidied, cx).await;
    assert_eq!(
        shell.label("item-not-tidied", cx).as_deref(),
        Some(NOT_TIDIED)
    );

    std::fs::remove_file(&left).unwrap();
    shell.click("item-not-tidied-check-again", cx);
    shell.until(cx, attempt(3)).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok));
    until_lock(&shell, &root, LockState::Open, cx).await;
    assert!(!shell.present("item-not-tidied", cx));
    assert_no_lock(&shell, cx);
    assert_eq!(step(&shell, &root), None);
}

/// L-5, real: a ticket with no lifecycle row is editable; while its close
/// runs it is not; once the close is complete its row is gone and it is
/// editable again.
#[gpui_kit::test]
async fn a_finished_or_missing_row_unlocks(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Finished", "")
    });
    open(&shell, ticket(&root), cx);
    assert_eq!(step(&shell, &root), None);
    assert_eq!(lock(&shell, &root, cx), LockState::Open, "no row");
    assert_edits_live(&shell, cx);

    confirming(&shell, &root, cx).await;
    let gate = shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let id = closes(&shell, cx)[0];
    at_stage(&shell, id, ProgressStage::Integrating, cx).await;
    assert_eq!(
        lock(&shell, &root, cx),
        LockState::Locked(LockSource::Worker)
    );
    gate.release();
    shell.until_ended(id, cx).await;
    assert_eq!(code(&shell, id, cx), Some(ResultCode::Ok));
    assert_eq!(step(&shell, &root), None, "a complete parent is not listed");
    until_lock(&shell, &root, LockState::Open, cx).await;
    shell.keys("escape", cx);
    assert_no_lock(&shell, cx);
}
