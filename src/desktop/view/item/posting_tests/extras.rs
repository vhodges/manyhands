//! Posting beyond the P-rows (design, Posting; Body draft separation;
//! contracts 1 and 8; decision 15): saving with no remote, a checkpoint
//! that fails, the pending row, the body draft never committed, the reply
//! composer held while posting, outcomes applied only to their own
//! comment's composer and never over newer text, quitting during an
//! accepted post, no publication without a Submit, and the body editable
//! while a post is held (A8).

use std::path::Path;

use gpui_kit::{TestAppContext, VisualTestContext};
use manyhands::results::{CheckpointEffect, Outcome, ResultCode};

use super::super::super::discussion_tests::{
    BO, C, commit_comment, composer_target, flushed, new_composer, open_new_composer, press_reply,
    reply_composer, stored_comments, type_into,
};
use super::support::{
    REPLY_POSTING, SAVED_HERE, checkpoint_paths, checkpoints, chip, comment_posts, composer_state,
    context, focus_end, hold_after_acceptance, listed, not_saved, open_ticket, outcome,
    pending_row, post_request, posting_shell, saved_here, shares, submit, until_answered,
    until_post_runs, write_new,
};
use crate::desktop::{
    keys::Quit,
    model::publication::CommentOutcome,
    services::{JobKind, requests::RequestState, worker::RecordState},
    test_support::fixtures::ticket_path,
    view::{harness::WAIT, item::composer::Composer, workspace::QuitPhase},
};

/// With no publication remote a post is saved here (real library): the
/// comment is listed with "Saved on this computer" and no chip, one
/// checkpoint, the draft retired and the composer cleared.
#[gpui_kit::test]
async fn saving_locally_without_a_remote_shows_saved_here(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Kept here.", cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::SavedHere, RecordState::Finished, false))
    );
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| listed(shell, cx) == 2)
        .await;
    assert_eq!(
        shell.label(saved_here(&comment), cx).as_deref(),
        Some(SAVED_HERE)
    );
    assert!(!shell.present(chip(&comment), cx));
    assert_eq!(checkpoints(&root), 1);
    shell
        .until_drawn(cx, "the draft retired", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    assert_eq!(
        composer_state(&new_composer(&shell, cx), cx),
        (String::new(), false)
    );
}

/// Removes `root`'s own Git identity: the isolated home holds none, so a
/// checkpoint cannot be committed.
fn remove_identity(root: &Path) {
    let repository = git2::Repository::open(root).unwrap();
    let mut config = git2::Config::open(&repository.path().join("config")).unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
}

/// A checkpoint that fails (real library: no identity to commit as) keeps
/// the text and its draft, commits nothing and adds no row, pending or
/// listed.
#[gpui_kit::test]
async fn a_failed_checkpoint_keeps_the_text_and_adds_no_row(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, root| remove_identity(root));
    let (composer, comment) = write_new(&shell, "Not committed.", cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    let (result, ..) = outcome(&shell, &root, &comment, cx).unwrap();
    assert!(!result.is_saved(), "{result:?}");
    assert_eq!(checkpoints(&root), 0);
    assert_eq!(
        composer_state(&composer, cx),
        ("Not committed.".to_owned(), true)
    );
    assert!(shell.present("composer-failed", cx));
    assert!(!shell.present(pending_row(&comment), cx));
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert_eq!(listed(&shell, cx), 1, "no row added");
    flushed(&shell, cx);
    assert_eq!(
        stored_comments(&shell.fixture).len(),
        1,
        "the draft is kept"
    );
}

/// A post draws a pending row from its text, keyed by its comment ID,
/// until its outcome; then the listed row takes its place.
#[gpui_kit::test]
async fn posting_shows_a_pending_row_until_the_outcome(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Pending.", cx);
    let held = shell.backend.hold_job(JobKind::Comment);
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    held.wait_reached();
    assert!(shell.present(pending_row(&comment), cx), "a pending row");
    assert_eq!(listed(&shell, cx), 1, "not listed yet");
    held.release();
    until_answered(&shell, &root, &comment, cx).await;
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| listed(shell, cx) == 2)
        .await;
    assert!(
        !shell.present(pending_row(&comment), cx),
        "the pending row goes"
    );
}

/// Body draft separation (contract 2, A12, review focus 3): a post while
/// the body has unsaved text commits only the comment's file; the body in
/// the context is unchanged.
#[gpui_kit::test]
async fn posting_never_commits_the_dirty_body(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    shell.click("item-edit", cx);
    let host = shell.until_editing(cx).await;
    shell.type_at_end(&host, "Unsaved body words.", cx);
    flushed(&shell, cx);

    let (_, comment) = write_new(&shell, "A comment.", cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert!(outcome(&shell, &root, &comment, cx).is_some_and(|(outcome, ..)| outcome.is_saved()));
    assert_eq!(
        checkpoint_paths(&root),
        vec![format!(
            ".manyhands/comments/{}/{comment}.md",
            super::super::super::discussion_tests::TICKET
        )]
    );
    let body = std::fs::read_to_string(
        context(&root).join(ticket_path(super::super::super::discussion_tests::TICKET)),
    )
    .unwrap();
    assert!(!body.contains("Unsaved body words."));
}

/// The reply composer does not move while it is posting: Reply elsewhere
/// leaves it in its thread and says why (design, Placement).
#[gpui_kit::test]
async fn the_reply_composer_cannot_move_while_posting(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        commit_comment(fixture, root, C, None, 3, Some(BO), "Another.\n");
    });
    press_reply(&shell, 0, cx);
    let (thread, reply) = reply_composer(&shell, cx).unwrap();
    type_into(&shell, &reply, "Reply.", cx);
    flushed(&shell, cx);
    let comment = composer_target(&shell, &reply, cx).unwrap().comment_id;
    let held = shell.backend.hold_job(JobKind::Comment);
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    held.wait_reached();

    press_reply(&shell, 1, cx);
    let (still, same) = reply_composer(&shell, cx).unwrap();
    assert_eq!(still, thread);
    assert_eq!(same, reply);
    assert_eq!(
        shell.label("reply-posting-note", cx).as_deref(),
        Some(REPLY_POSTING)
    );
    held.release();
    until_answered(&shell, &root, &comment, cx).await;
}

fn is_failed(composer: &gpui_kit::Entity<Composer>, cx: &mut TestAppContext) -> bool {
    composer.read_with(cx, |composer, _| {
        composer.is_read_only() && !composer.is_posting()
    })
}

/// Round 2, I7: an outcome applies only to the composer whose comment ID
/// is its own. A reply's failure leaves the new comment's composer, and
/// its text, as they were.
#[gpui_kit::test]
async fn an_outcome_applies_only_to_its_comments_composer(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    open_new_composer(&shell, cx);
    let new = new_composer(&shell, cx);
    type_into(&shell, &new, "New.", cx);
    press_reply(&shell, 0, cx);
    let (_, reply) = reply_composer(&shell, cx).unwrap();
    type_into(&shell, &reply, "Reply.", cx);
    flushed(&shell, cx);
    let comment = composer_target(&shell, &reply, cx).unwrap().comment_id;
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    returned.release();
    focus_end(&shell, &reply, cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;

    assert!(is_failed(&reply, cx), "the reply's composer failed");
    assert_eq!(composer_state(&new, cx), ("New.".to_owned(), false));
    assert_eq!(new_composer(&shell, cx), new);
}

/// Contract 8, A17: an outcome landing after the tab was closed and opened
/// again, and new text typed there, never replaces that text: the old
/// post's draft is not adopted over it, and its failure is not shown in
/// the new composer.
#[gpui_kit::test]
async fn an_outcome_after_a_tab_switch_or_reopen_never_replaces_newer_text(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, older) = write_new(&shell, "Older.", cx);
    let held = shell.backend.hold_job(JobKind::Comment);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    held.wait_reached();
    shell.click(("tab-close", 0usize), cx);
    open_ticket(&shell, &root, cx);
    open_new_composer(&shell, cx);
    let newer = new_composer(&shell, cx);
    type_into(&shell, &newer, "Newer.", cx);
    flushed(&shell, cx);

    held.release();
    returned.release();
    until_answered(&shell, &root, &older, cx).await;
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(new_composer(&shell, cx), newer);
    assert_eq!(composer_state(&newer, cx), ("Newer.".to_owned(), false));
    assert_ne!(
        composer_target(&shell, &newer, cx).map(|target| target.comment_id),
        Some(older)
    );
    assert!(!shell.present("composer-failed", cx));
}

/// Decision 15, K-10: quitting while an accepted post publishes cancels
/// its token; it stops at a safe point, saved with its checkpoint
/// committed and its publication cancelled, which is final: no Resume is
/// offered for it.
#[gpui_kit::test]
async fn quitting_during_an_accepted_post_leaves_it_saved_and_cancelled(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Quit while sharing.", cx);
    let (reached, release) = hold_after_acceptance(&shell);
    submit(&shell, cx);
    reached.recv_timeout(WAIT).expect("the post was accepted");
    let id = post_request(&shell, &comment, cx).unwrap();
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Running { accepted: true, .. })
            )
        })
        .await;
    let mut visual = VisualTestContext::from_window(shell.window.into(), cx);
    visual.dispatch_action(Quit);
    visual.run_until_parked();
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase()),
        QuitPhase::Finishing(id)
    );
    drop(release);
    until_answered(&shell, &root, &comment, cx).await;

    let state = shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(outcome, _, effects, ..) => Some((*outcome, effects.checkpoint)),
            _ => None,
        }
    });
    assert_eq!(
        state,
        Some((Outcome::Cancelled, CheckpointEffect::Committed))
    );
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedStopped)
    );
    assert_eq!(checkpoints(&root), 1);
    assert!(!shell.present(("pending-resume", 0usize), cx));
}

/// Contract 1, A16: typing a comment, navigating away and back, saving and
/// finishing the body, closing the tab and quitting submit no comment and
/// no share.
#[gpui_kit::test]
async fn navigation_save_done_close_and_quit_submit_no_publication(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (composer, _) = write_new(&shell, "Never posted.", cx);
    assert!(!composer.read_with(cx, |composer, _| composer.is_read_only()));

    shell.select(&root, cx);
    open_ticket(&shell, &root, cx);
    shell.click("item-edit", cx);
    let host = shell.until_editing(cx).await;
    shell.type_at_end(&host, " More body.", cx);
    shell.click("item-save", cx);
    shell
        .until(cx, |operations| {
            operations.requests().iter().any(|request| {
                crate::desktop::view::harness::is_save(&request.intent)
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    shell.click("item-done", cx);
    shell.run(cx);
    shell.click(("tab-close", 0usize), cx);
    shell.run(cx);
    let mut visual = VisualTestContext::from_window(shell.window.into(), cx);
    visual.dispatch_action(Quit);
    visual.run_until_parked();

    assert!(comment_posts(&shell, cx).is_empty(), "no comment posted");
    assert_eq!(shares(&shell, cx), 0, "nothing shared");
}

/// A8: while an accepted post is held before its domain call, typing in
/// the item's body editor lands at once and the window stays responsive;
/// the post completes once let go.
#[gpui_kit::test]
async fn the_body_stays_editable_while_a_post_is_held(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    shell.click("item-edit", cx);
    let host = shell.until_editing(cx).await;
    let (_, comment) = write_new(&shell, "Held post.", cx);
    let (reached, release) = hold_after_acceptance(&shell);
    submit(&shell, cx);
    reached.recv_timeout(WAIT).expect("the post was accepted");
    let id = post_request(&shell, &comment, cx).unwrap();
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(id).map(|request| &request.state),
                Some(RequestState::Running { accepted: true, .. })
            )
        })
        .await;

    shell.type_at_end(&host, " Typed while held.", cx);
    let text = host.read_with(cx, |host, cx| host.editor().read(cx).text().to_string());
    assert!(text.ends_with(" Typed while held."), "{text:?}");
    assert!(
        outcome(&shell, &root, &comment, cx).is_none(),
        "the post is still held"
    );

    drop(release);
    until_answered(&shell, &root, &comment, cx).await;
    assert!(outcome(&shell, &root, &comment, cx).is_some_and(|(outcome, ..)| outcome.is_saved()));
    let text = host.read_with(cx, |host, cx| host.editor().read(cx).text().to_string());
    assert!(text.ends_with(" Typed while held."), "{text:?}");
}
