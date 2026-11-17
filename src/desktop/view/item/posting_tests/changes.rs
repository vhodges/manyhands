//! The design checkpoint's changes (orchestrator, Task 13): a mismatch
//! saved by its record (1); Check again final once a record is found (2;
//! decision 23 is `model::publication::record`'s, since no fixture leaves
//! a finished record that touched nothing); re-targeting at the Edit text press (4) and its refusal
//! (5); the newest draft of a thread wins it (5); a post's draft retired
//! once (6), and retired rather than adopted (7); the listing check (8);
//! and the stored body posted exactly (9).

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    drafts::{DraftErrorKind, FailureAction, FailurePoint, Held},
    repository::RequestId,
    results::{CheckpointEffect, Envelope, ResultCode, Scope},
};

use super::super::super::discussion_tests::{
    A, commit_comment, composer_target, new_composer, stored_comments, type_into,
};
use super::support::{
    KEPT, KEPT_REPLY, comment_posts, composer_state, failed_actions, focus_end, internal_error,
    not_saved, outcome, post_draft, post_through, posting_shell, press, store_comment, submit,
    until_adopted, until_answered, until_attempt, until_post_runs, write_new,
};
use crate::desktop::{
    model::publication::CommentOutcome,
    services::{JobKind, ReadKind},
    test_support::fixtures::item_id,
    view::item::composer::{MAYBE_SAVED, NOT_MOVED, NOT_SAVED_FINAL},
};

/// Removes `root`'s own Git identity: no checkpoint can be committed.
fn remove_identity(root: &Path) {
    let repository = git2::Repository::open(root).unwrap();
    let mut config = git2::Config::open(&repository.path().join("config")).unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
}

/// Change 1: a mismatch whose record is still accepted is saved only if
/// its refusal says the checkpoint committed: here nothing was, so it is
/// not said to be saved, and its draft is kept.
#[gpui_kit::test]
async fn an_accepted_mismatch_is_not_said_to_be_saved_without_its_checkpoint(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        remove_identity(root);
        let posted = post_through(fixture, root, KEPT, "Original.");
        assert_eq!(
            posted.code,
            ResultCode::IdentityRequired,
            "{}",
            posted.message
        );
        store_comment(fixture, root, KEPT, None, "Changed.");
    });
    let comment = item_id(KEPT);
    let composer = until_adopted(&shell, "Changed.", cx).await;
    press(&shell, "composer-retry", cx);
    until_attempt(&shell, &root, &comment, 1, cx).await;
    shell
        .until_drawn(cx, "the mismatch settled", |shell, cx| {
            shell.present("composer-failed", cx)
        })
        .await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::Mismatched { saved: false })
    );
    assert_eq!(
        failed_actions(&shell, cx),
        vec!["composer-copy-text", "composer-discard"]
    );
    assert_eq!(composer_state(&composer, cx), ("Changed.".to_owned(), true));
    assert_eq!(
        stored_comments(&shell.fixture).len(),
        1,
        "the draft is kept"
    );
}

/// Change 2: Check again that finds an accepted record ends there, never
/// in another "may already be saved": Copy text and Discard, and no Retry
/// after an `internal_error`, which a retry would only repeat.
#[gpui_kit::test]
async fn check_again_finding_an_accepted_record_ends_in_a_final_state(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Resent.", cx);
    let returned = shell.backend.hold_job_return_answering(
        JobKind::Comment,
        not_saved(ResultCode::TransportUnavailable),
    );
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    returned.release();
    press(&shell, "composer-retry", cx);
    until_attempt(&shell, &root, &comment, 2, cx).await;
    assert_eq!(
        shell.label("composer-failed", cx).as_deref(),
        Some(MAYBE_SAVED)
    );

    press(&shell, "composer-check-again", cx);
    shell
        .until_drawn(cx, "Check again answered", |shell, cx| {
            shell.label("composer-failed", cx).as_deref() == Some(NOT_SAVED_FINAL)
        })
        .await;
    assert_eq!(
        failed_actions(&shell, cx),
        vec!["composer-copy-text", "composer-discard"]
    );
    assert_eq!(
        comment_posts(&shell, cx).len(),
        1,
        "Check again sent nothing"
    );
}

/// Change 4: Edit text re-targets at the press: the store holds only the
/// draft of a new comment ID, with the text, before anything is typed;
/// later edits go to it.
#[gpui_kit::test]
async fn edit_text_retargets_the_draft_at_the_press(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, first) = write_new(&shell, "First.", cx);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &first, cx).await;

    press(&shell, "composer-edit-text", cx);
    shell
        .until_drawn(cx, "re-targeted", |shell, cx| {
            stored_comments(&shell.fixture)
                .iter()
                .all(|(target, ..)| target.comment_id != first)
                && !composer.read_with(cx, |composer, _| composer.is_read_only())
        })
        .await;
    let target = composer_target(&shell, &composer, cx).expect("a new ID");
    assert_ne!(target.comment_id, first);
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0, target);
    assert_eq!(stored[0].1, "First.");
    type_into(&shell, &composer, " More.", cx);
    super::super::super::discussion_tests::flushed(&shell, cx);
    assert_eq!(stored_comments(&shell.fixture)[0].1, "First. More.");
    assert!(comment_posts(&shell, cx).len() == 1, "nothing posted yet");
}

/// Change 5: a re-target whose new draft cannot be begun refuses: the old
/// comment ID, text and draft stay, with the failed post's actions.
#[gpui_kit::test]
async fn a_retarget_that_cannot_begin_keeps_the_old_comment(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, first) = write_new(&shell, "First.", cx);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &first, cx).await;

    let _failing = shell.fixture.draft_store().with_failure_point_for_testing(
        FailurePoint::CreateTemp,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );
    press(&shell, "composer-edit-text", cx);
    shell
        .until_drawn(cx, "the re-target refused", |shell, cx| {
            shell.label("composer-failed", cx).as_deref() == Some(NOT_MOVED)
        })
        .await;
    assert_eq!(
        composer_target(&shell, &composer, cx).map(|target| target.comment_id),
        Some(first.clone())
    );
    assert_eq!(composer_state(&composer, cx), ("First.".to_owned(), true));
    assert_eq!(failed_actions(&shell, cx), vec!["composer-edit-text"]);
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0.comment_id, first, "nothing retired");
}

/// Change 5: of two comment drafts for one thread, the newest is adopted;
/// the other stays on the Drafts page, not held here.
#[gpui_kit::test]
async fn the_newest_draft_of_a_thread_wins_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, _) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT_REPLY, None, "Older.");
        // A draft's ID orders by the millisecond it was made in.
        std::thread::sleep(std::time::Duration::from_millis(5));
        store_comment(fixture, root, KEPT, None, "Newer.");
    });
    until_adopted(&shell, "Newer.", cx).await;
    let stored = stored_comments(&shell.fixture);
    let older = stored
        .iter()
        .find(|(target, ..)| target.comment_id == item_id(KEPT_REPLY))
        .expect("the older draft stays");
    assert_ne!(older.2, Held::Here);
}

/// Change 6: a saved post's draft is retired once: `Submissions` then
/// holds no draft to retire for it.
#[gpui_kit::test]
async fn a_saved_posts_draft_is_retired_once(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Once.", cx);
    assert!(stored_comments(&shell.fixture).len() == 1);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    shell
        .until_drawn(cx, "the draft retired", |shell, cx| {
            post_draft(shell, &root, &comment, cx) == Some(None)
        })
        .await;
    assert!(stored_comments(&shell.fixture).is_empty());
}

/// Change 7: a saved post whose draft's retirement has not landed is not
/// adopted when its tab opens again: it is retired instead, and the
/// composer stays empty.
#[gpui_kit::test]
async fn a_saved_post_awaiting_retirement_is_retired_not_adopted(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Saved.", cx);
    let held = shell.backend.hold_job(JobKind::Comment);
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    held.wait_reached();
    shell.click(("tab-close", 0usize), cx);
    // The drain's retirement waits at its store write.
    let write = shell.backend.hold_read(ReadKind::DraftWrite);
    held.release();
    until_answered(&shell, &root, &comment, cx).await;
    assert!(matches!(
        post_draft(&shell, &root, &comment, cx),
        Some(Some(_))
    ));

    super::support::open_ticket(&shell, &root, cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let composer = new_composer(&shell, cx);
    assert_eq!(composer_state(&composer, cx), (String::new(), false));
    drop(write);
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

/// Change 8: an adopted draft whose listing check cannot refresh is posted
/// anyway, as the same request.
#[gpui_kit::test]
async fn a_failed_refresh_posts_the_adopted_draft_anyway(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Posted anyway.");
    });
    let comment = item_id(KEPT);
    let composer = until_adopted(&shell, "Posted anyway.", cx).await;
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
    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        comment_posts(&shell, cx),
        vec![(comment.clone(), RequestId::parse(KEPT).unwrap(), 1)]
    );
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, _, resend)| (outcome, resend)),
        Some((CommentOutcome::SavedHere, true))
    );
}

/// Change 8: a listed comment retires the adopted draft only when its body
/// is the draft's byte for byte; a listed comment of other text is posted
/// to, and the library answers for it.
#[gpui_kit::test]
async fn a_listed_comment_of_other_text_is_still_posted(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        commit_comment(fixture, root, KEPT_REPLY, Some(A), 2, None, "Other.\n");
        store_comment(fixture, root, KEPT_REPLY, Some(A), "Mine.\n");
    });
    let comment = item_id(KEPT_REPLY);
    super::super::super::discussion_tests::press_reply(&shell, 0, cx);
    let (_, reply) = super::super::super::discussion_tests::reply_composer(&shell, cx)
        .expect("the reply reopens");
    shell
        .until_drawn(cx, "the reply's record looked at", |_, cx| {
            reply.read_with(cx, |composer, cx| {
                composer.text(cx) == "Mine.\n" && !composer.is_read_only()
            })
        })
        .await;
    focus_end(&shell, &reply, cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        comment_posts(&shell, cx).first().map(|(id, ..)| id.clone()),
        Some(comment),
        "posted, not retired"
    );
    assert_eq!(
        stored_comments(&shell.fixture).len(),
        1,
        "the draft is kept"
    );
}

/// Change 9: an adopted draft that was submitted posts its stored body
/// byte for byte, not the editor's loaded text: here one the editor's load
/// normalizes. The same request replays its saved answer, no mismatch.
#[gpui_kit::test]
async fn an_adopted_draft_posts_its_stored_body_exactly(cx: &mut TestAppContext) {
    const BODY: &str = "Line one.\r\nThen $$x$$ inline.\n";
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        let posted = post_through(fixture, root, KEPT, BODY);
        assert_eq!(posted.effects.checkpoint, CheckpointEffect::Committed);
        store_comment(fixture, root, KEPT, None, BODY);
    });
    let comment = item_id(KEPT);
    shell
        .until_drawn(cx, "the draft adopted", |shell, cx| {
            shell.present("composer-retry", cx)
        })
        .await;
    press(&shell, "composer-retry", cx);
    until_attempt(&shell, &root, &comment, 1, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedHere),
        "the stored body is the posted one"
    );
}
