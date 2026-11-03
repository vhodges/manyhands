//! Owner decisions 26 and 27 (Task 13 re-review): the operations panel
//! offers no re-send of an unsaved comment post, only its composer does,
//! while a saved one keeps Retry sharing; and edited text overtaken by its
//! old comment, sent again from outside the app, is never put under the
//! old ID nor discarded.

use gpui_kit::TestAppContext;
use manyhands::results::{RecoveryAction, RecoveryActionKind, ResultCode};

use super::super::super::discussion_tests::{composer_target, stored_comments, type_into};
use super::support::{
    comment_files, comment_posts, failed_actions, focus_end, not_saved, outcome, post_request,
    posting_shell, press, submit, until_answered, until_attempt, write_new,
};
use crate::desktop::{
    model::publication::CommentOutcome,
    services::JobKind,
    view::{harness::Shell, operations::recovery_id},
};

/// The labels of the recovery controls the operation entry of `id` shows.
fn recovery_labels(
    shell: &Shell,
    id: crate::desktop::services::requests::DesktopRequestId,
    cx: &mut TestAppContext,
) -> Vec<String> {
    (0..4)
        .filter_map(|n| shell.label(recovery_id(id, n), cx))
        .collect()
}

/// Decision 26: a comment post whose answer is not saved, which the
/// library says the same request may retry, offers no Retry in the
/// operations panel: its composer is where it is sent again.
#[gpui_kit::test]
async fn an_unsaved_posts_operation_entry_offers_no_retry(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Not saved.", cx);
    let answer = not_saved(ResultCode::TransportUnavailable).with_recovery(vec![RecoveryAction {
        action: RecoveryActionKind::RequestRetry,
        operation_id: None,
        arguments: serde_json::Map::new(),
    }]);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, answer);
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    let id = post_request(&shell, &comment, cx).unwrap();
    shell.frame(cx, |_, _| ());
    assert!(
        !recovery_labels(&shell, id, cx)
            .iter()
            .any(|label| label.contains("Retry")),
        "{:?}",
        recovery_labels(&shell, id, cx)
    );
}

/// Decision 26: a saved post that is not shared keeps Retry sharing in
/// the operations panel: it only publishes (real library, U-2).
#[gpui_kit::test]
async fn a_saved_unshared_posts_operation_entry_offers_retry_sharing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Owed.", cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert!(matches!(
        outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::SavedNotShared { .. }, ..))
    ));
    let id = post_request(&shell, &comment, cx).unwrap();
    shell.frame(cx, |_, _| ());
    let labels = recovery_labels(&shell, id, cx);
    assert!(
        labels.iter().any(|label| label == "Retry sharing"),
        "{labels:?}"
    );
    assert!(!labels.iter().any(|label| label == "Retry"), "{labels:?}");
}

/// Decision 27: Edit text moves the composer from X to Y and the text is
/// edited; X, sent again from outside the app, saves meanwhile. Submit
/// sends nothing: the edited text stays, under Y, with its draft, saying
/// it was posted meanwhile, and offers Post as a new comment, which sends
/// it as Y.
#[gpui_kit::test]
async fn edited_text_survives_when_its_old_comment_saved_meanwhile(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, first) = write_new(&shell, "First.", cx);
    // Saved by the real call; the desktop hears it was not.
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &first, cx).await;
    press(&shell, "composer-edit-text", cx);
    shell
        .until_drawn(cx, "re-targeted", |_, cx| {
            !composer.read_with(cx, |composer, _| composer.is_read_only())
        })
        .await;
    let second = composer_target(&shell, &composer, cx).unwrap().comment_id;
    assert_ne!(second, first);
    type_into(&shell, &composer, " Edited.", cx);
    super::super::super::discussion_tests::flushed(&shell, cx);

    // X sent again from outside: here the worker's own retry.
    let id = post_request(&shell, &first, cx).unwrap();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.retry(id, None));
    until_attempt(&shell, &root, &first, 2, cx).await;

    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    shell
        .until_drawn(cx, "posted meanwhile", |shell, cx| {
            shell.label("composer-failed", cx).as_deref()
                == Some("Your earlier text was posted meanwhile.")
        })
        .await;
    assert_eq!(
        failed_actions(&shell, cx),
        vec![
            "composer-post-as-new",
            "composer-copy-text",
            "composer-discard"
        ]
    );
    assert_eq!(comment_posts(&shell, cx).len(), 1, "Y was not sent");
    assert_eq!(
        composer_target(&shell, &composer, cx).map(|target| target.comment_id),
        Some(second.clone()),
        "never under X's ID"
    );
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1, "the edited text's draft is kept");
    assert_eq!(stored[0].0.comment_id, second);
    assert_eq!(stored[0].1, "First. Edited.");

    press(&shell, "composer-post-as-new", cx);
    until_answered(&shell, &root, &second, cx).await;
    assert_eq!(comment_files(&root).len(), 3, "A, X and, by choice, Y");
}

/// Decision 26: no control that could complete an unsaved post is
/// offered in the operations panel: neither Retry, Resume nor Prepare
/// again, even when its answer carries them.
#[gpui_kit::test]
async fn an_unsaved_posts_operation_entry_offers_no_resume_or_prepare(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Not saved.", cx);
    let action = |action, operation_id: Option<&str>| RecoveryAction {
        action,
        operation_id: operation_id.map(str::to_owned),
        arguments: serde_json::Map::new(),
    };
    let answer = not_saved(ResultCode::TransportUnavailable).with_recovery(vec![
        action(
            RecoveryActionKind::OperationResume,
            Some("01M4CC0VN1KTQFSVR99378TDHV"),
        ),
        action(RecoveryActionKind::RequestPrepare, None),
        action(RecoveryActionKind::RequestRetry, None),
    ]);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, answer);
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    let id = post_request(&shell, &comment, cx).unwrap();
    shell.frame(cx, |_, _| ());
    let labels = recovery_labels(&shell, id, cx);
    assert!(
        !labels.iter().any(|label| label.contains("Retry")
            || label.contains("Resume")
            || label.contains("Prepare")),
        "{labels:?}"
    );
}

/// Decision 27 with P7/P18: Edit text moves the composer from X to Y; X,
/// sent again from outside, answers with its file written and not
/// committed, which may still come to be saved. Y is held, not sent:
/// "Your earlier text may still be saved", with Check again, Copy text and
/// Discard. Check again finds X's record saved it, with Y's very text:
/// the composer clears. One comment.
#[gpui_kit::test]
async fn new_text_is_held_while_its_old_comment_may_still_be_saved(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, first) = write_new(&shell, "First.", cx);
    // Saved by the real call; the desktop hears it was not.
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &first, cx).await;
    press(&shell, "composer-edit-text", cx);
    shell
        .until_drawn(cx, "re-targeted", |_, cx| {
            !composer.read_with(cx, |composer, _| composer.is_read_only())
        })
        .await;

    // X sent again from outside; its answer: written, not committed.
    let returned = shell.backend.hold_job_return_answering(
        JobKind::Comment,
        super::support::written(ResultCode::TransportUnavailable),
    );
    returned.release();
    let id = post_request(&shell, &first, cx).unwrap();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.retry(id, None));
    until_attempt(&shell, &root, &first, 2, cx).await;

    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    shell
        .until_drawn(cx, "held", |shell, cx| {
            shell.label("composer-failed", cx).as_deref()
                == Some("Your earlier text may still be saved.")
        })
        .await;
    assert_eq!(
        failed_actions(&shell, cx),
        vec![
            "composer-check-again",
            "composer-copy-text",
            "composer-discard"
        ]
    );
    assert_eq!(comment_posts(&shell, cx).len(), 1, "Y was not sent");

    press(&shell, "composer-check-again", cx);
    shell
        .until_drawn(cx, "settled: X is this text", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    assert_eq!(
        super::support::composer_state(
            &super::super::super::discussion_tests::new_composer(&shell, cx),
            cx
        ),
        (String::new(), false)
    );
    assert_eq!(comment_posts(&shell, cx).len(), 1);
    assert_eq!(comment_files(&root).len(), 2, "A and X: one comment");
}
