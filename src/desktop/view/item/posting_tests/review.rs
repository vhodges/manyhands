//! Ruling R-19 (Task 13 review): a composer follows its post, sent from
//! anywhere, so an operation-area Retry never leads to a second comment;
//! decision 24 retires only a draft whose text is the listed comment's;
//! and "may already be saved" never traps the text.

use gpui_kit::TestAppContext;
use manyhands::results::{Envelope, ResultCode, Scope};

use super::super::super::discussion_tests::{
    A, commit_comment, new_composer, reply_composer, stored_comments,
};
use super::support::{
    KEPT, KEPT_REPLY, MAY_STILL_BE_SAVED, comment_files, comment_posts, composer_state,
    failed_actions, focus_end, internal_error, listed, maybe_saved_without_record, outcome,
    post_request, posting_shell, press, reserved, store_comment, submit, until_answered,
    until_attempt,
};
use crate::desktop::{
    model::publication::CommentOutcome, services::JobKind, test_support::fixtures::item_id,
};

/// R-19: a Retry of a "may already be saved" post sent from the operation
/// area, while its composer shows the failure, is followed: the composer
/// shows Posting, offers nothing that could post anew, and takes the
/// Retry's answer. One comment, one request.
#[gpui_kit::test]
async fn an_operation_area_retry_is_followed_and_posts_one_comment(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Once.");
    });
    let comment = item_id(KEPT);
    let composer = super::support::until_adopted(&shell, "Once.", cx).await;
    // Saved by the real call; the desktop hears it may be.
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    returned.release();
    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::MaybeSaved)
    );
    assert!(shell.present("composer-check-again", cx));

    let held = shell.backend.hold_job(JobKind::Comment);
    let id = post_request(&shell, &comment, cx).unwrap();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.retry(id, None), "the operation area's Retry");
    held.wait_reached();
    shell
        .until_drawn(cx, "the composer follows its post", |shell, cx| {
            composer.read_with(cx, |composer, _| composer.is_posting())
                && !shell.present("composer-check-again", cx)
        })
        .await;
    assert!(!shell.present("composer-post-as-new", cx));
    held.release();
    until_attempt(&shell, &root, &comment, 2, cx).await;
    shell
        .until_drawn(cx, "the draft retired", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    assert_eq!(comment_files(&root).len(), 2, "A and one new comment");
    assert_eq!(comment_posts(&shell, cx).len(), 1, "one request");
    assert_eq!(listed(&shell, cx), 2);
    assert_eq!(
        composer_state(&new_composer(&shell, cx), cx),
        (String::new(), false)
    );
}

/// R-19, decision 24: Check again with no record and the comment listed
/// with other text keeps the draft: a mismatch, with Copy text and
/// Discard; nothing is retired or forgotten.
#[gpui_kit::test]
async fn a_listed_comment_of_other_text_is_a_mismatch(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Mine.\n");
    });
    let comment = item_id(KEPT);
    maybe_saved_without_record(&shell, &root, "Mine.\n", cx).await;
    commit_comment(&shell.fixture, &root, KEPT, None, 4, None, "Different.\n");

    press(&shell, "composer-check-again", cx);
    shell
        .until_drawn(cx, "Check again answered", |shell, cx| {
            failed_actions(shell, cx) == vec!["composer-copy-text", "composer-discard"]
        })
        .await;
    assert_eq!(
        composer_state(&new_composer(&shell, cx), cx),
        ("Mine.\n".to_owned(), true)
    );
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1, "the draft is kept");
    assert_eq!(stored[0].1, "Mine.\n");
    assert!(reserved(&shell, &root, &comment, cx));
}

/// R-19: Check again that proves nothing (its refresh failed) offers Copy
/// text and Discard, with P7's note, beside Check again.
#[gpui_kit::test]
async fn an_unproven_check_again_keeps_copy_and_discard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Unproven.");
    });
    maybe_saved_without_record(&shell, &root, "Unproven.", cx).await;
    let failed = shell.backend.hold_job_return_answering(
        JobKind::IndexRefresh,
        Envelope::failure(
            "index refresh",
            Scope::default(),
            ResultCode::InternalError,
            vec![],
        ),
    );
    failed.release();
    press(&shell, "composer-check-again", cx);
    shell
        .until_drawn(cx, "Check again answered", |shell, cx| {
            shell.present("composer-check-again", cx)
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
    press(&shell, "composer-discard", cx);
    assert_eq!(
        shell.label("composer-discard-note", cx).as_deref(),
        Some(MAY_STILL_BE_SAVED)
    );
}

/// R-19: a reply that may already be saved can be closed: Discard is
/// offered, and confirmed, closes the reply composer.
#[gpui_kit::test]
async fn a_reply_that_may_be_saved_can_be_discarded(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT_REPLY, Some(A), "A reply.");
    });
    let comment = item_id(KEPT_REPLY);
    super::super::super::discussion_tests::press_reply(&shell, 0, cx);
    let (_, reply) = reply_composer(&shell, cx).expect("the reply reopens");
    shell
        .until_drawn(cx, "the reply's record looked at", |_, cx| {
            reply.read_with(cx, |composer, cx| {
                composer.text(cx) == "A reply." && !composer.is_read_only()
            })
        })
        .await;
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    returned.release();
    focus_end(&shell, &reply, cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::MaybeSaved)
    );
    assert!(failed_actions(&shell, cx).contains(&"composer-discard"));
    press(&shell, "composer-discard", cx);
    shell.click("composer-discard-confirm", cx);
    assert!(reply_composer(&shell, cx).is_none(), "the reply closed");
}

/// R-19, sequence A: Edit text moves the composer from X to a new ID;
/// then X is retried from the operation area and saves. Submit then sends
/// nothing new: X's saved answer stands, the new draft goes and the
/// composer clears. One comment.
#[gpui_kit::test]
async fn a_retarget_overtaken_by_its_old_comment_sends_nothing_new(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, first) = super::support::write_new(&shell, "First.", cx);
    // Saved by the real call; the desktop hears it was not.
    let returned = shell.backend.hold_job_return_answering(
        JobKind::Comment,
        super::support::not_saved(ResultCode::ItemNotFound),
    );
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &first, cx).await;
    press(&shell, "composer-edit-text", cx);
    shell
        .until_drawn(cx, "re-targeted", |shell, cx| {
            super::support::composer_target_of(shell, &composer, cx)
                .is_some_and(|target| target.comment_id != first)
                && !composer.read_with(cx, |composer, _| composer.is_read_only())
        })
        .await;

    let id = post_request(&shell, &first, cx).unwrap();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.retry(id, None), "the operation area's Retry of X");
    until_attempt(&shell, &root, &first, 2, cx).await;
    assert_eq!(
        outcome(&shell, &root, &first, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedHere)
    );

    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    shell
        .until_drawn(cx, "the composer cleared", |shell, cx| {
            stored_comments(&shell.fixture).is_empty()
                && composer_state(&new_composer(shell, cx), cx) == (String::new(), false)
        })
        .await;
    assert_eq!(comment_posts(&shell, cx).len(), 1, "only X was ever sent");
    assert_eq!(comment_files(&root).len(), 2, "A and X: one comment");
}

/// R-19, sequence B: Post as a new comment from "may already be saved";
/// while the old draft's retirement is awaited, X is retried and saves.
/// The new comment is never sent. One comment.
#[gpui_kit::test]
async fn post_as_new_overtaken_during_retirement_sends_nothing_new(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Fresh.");
    });
    let comment = item_id(KEPT);
    maybe_saved_without_record(&shell, &root, "Fresh.", cx).await;
    press(&shell, "composer-check-again", cx);
    shell
        .until_drawn(cx, "Post as a new comment offered", |shell, cx| {
            shell.present("composer-post-as-new", cx)
        })
        .await;

    // The old draft's retirement waits at its store write.
    let write = shell
        .backend
        .hold_read(crate::desktop::services::ReadKind::DraftWrite);
    press(&shell, "composer-post-as-new", cx);
    let id = post_request(&shell, &comment, cx).unwrap();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.retry(id, None), "the operation area's Retry of X");
    until_attempt(&shell, &root, &comment, 2, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedHere)
    );
    drop(write);
    shell
        .until_drawn(cx, "the composer cleared", |shell, cx| {
            stored_comments(&shell.fixture).is_empty()
                && composer_state(&new_composer(shell, cx), cx) == (String::new(), false)
        })
        .await;
    assert_eq!(comment_posts(&shell, cx).len(), 1, "only X was ever sent");
    assert_eq!(comment_files(&root).len(), 2, "A and X: one comment");
}
