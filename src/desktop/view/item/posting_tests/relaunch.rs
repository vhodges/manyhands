//! Posting after a relaunch (design, P4, P11, P12): what a session that
//! ended left is a stored comment draft, with or without a request record
//! under its comment ID (R-16), made here before the window opens.
//!
//! - An adopted draft whose comment has no record is posted after a
//!   refreshed listing is checked (R-8, P4): the same request when the
//!   comment is not listed, a retirement when it is.
//! - One whose comment has a record was submitted: it stays read-only and
//!   its same request is sent again first, before any listing (P11). Its
//!   answer is rendered: a finished record replays it. The re-send waits
//!   for the user's Retry, since opening a tab submits nothing (A16).

use gpui_kit::TestAppContext;
use manyhands::{
    repository::RequestId,
    results::{CheckpointEffect, ResultCode},
};

use super::super::super::discussion_tests::{
    composer_text, new_composer, press_reply, reply_composer, stored_comments,
};
use super::support::{
    KEPT, KEPT_REPLY, comment_posts, composer_state, failed_actions, focus_end, listed, outcome,
    post_through, posting_shell, press, store_comment, submit, until_adopted, until_answered,
    until_attempt,
};
use crate::desktop::{
    model::publication::CommentOutcome,
    services::{JobKind, requests::Intent},
    test_support::fixtures::item_id,
};

/// P4, R-8: an adopted draft whose comment has no record is posted only
/// after a refreshed listing: held, the refresh runs before any post. The
/// post is then the same request, its comment ID, and a re-send. A draft
/// whose comment the listing already shows is retired, and not posted.
#[gpui_kit::test]
async fn p4_an_adopted_draft_is_listed_first_then_posted_as_the_same_request(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Kept new.");
        // Saved by the session that ended, its record lost.
        super::super::super::discussion_tests::commit_comment(
            fixture,
            root,
            KEPT_REPLY,
            Some(super::super::super::discussion_tests::A),
            2,
            None,
            "Kept reply.\n",
        );
        store_comment(
            fixture,
            root,
            KEPT_REPLY,
            Some(super::super::super::discussion_tests::A),
            "Kept reply.\n",
        );
    });
    let comment = item_id(KEPT);
    let composer = until_adopted(&shell, "Kept new.", cx).await;
    assert_eq!(
        composer_state(&composer, cx),
        ("Kept new.".to_owned(), false)
    );

    let refresh = shell.backend.hold_job(JobKind::IndexRefresh);
    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::IndexRefresh { .. }))
        })
        .await;
    refresh.wait_reached();
    assert!(
        comment_posts(&shell, cx).is_empty(),
        "nothing is posted before the listing"
    );
    assert!(composer.read_with(cx, |composer, _| composer.is_read_only()));
    refresh.release();
    until_answered(&shell, &root, &comment, cx).await;
    let jobs = shell.backend.jobs_seen();
    let refreshed = jobs.iter().position(|kind| *kind == JobKind::IndexRefresh);
    let posted = jobs.iter().position(|kind| *kind == JobKind::Comment);
    assert!(refreshed.is_some() && refreshed < posted, "{jobs:?}");
    assert_eq!(
        comment_posts(&shell, cx),
        vec![(comment.clone(), RequestId::parse(KEPT).unwrap(), 1)]
    );
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, _, resend)| (outcome, resend)),
        Some((CommentOutcome::SavedHere, true))
    );

    // The reply's comment is listed already: its draft is retired.
    let posts = comment_posts(&shell, cx).len();
    press_reply(&shell, 0, cx);
    let (_, reply) = reply_composer(&shell, cx).expect("the reply reopens");
    assert_eq!(composer_text(&reply, cx), "Kept reply.\n");
    focus_end(&shell, &reply, cx);
    submit(&shell, cx);
    shell
        .until_drawn(cx, "the listed draft retired", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    assert_eq!(comment_posts(&shell, cx).len(), posts, "not posted");
    assert_eq!(listed(&shell, cx), 3, "A, the reply and the new comment");
}

/// P11: an adopted draft whose comment has a record was submitted: it is
/// read-only, and Retry sends the same request again before any listing
/// check. Its finished record replays its saved answer; the draft is
/// retired and the composer cleared, and the comment is listed once.
#[gpui_kit::test]
async fn p11_a_relaunch_resends_the_same_request_before_listing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        let posted = post_through(fixture, root, KEPT, "Relaunched.");
        assert_eq!(posted.effects.checkpoint, CheckpointEffect::Committed);
        store_comment(fixture, root, KEPT, None, "Relaunched.");
    });
    let comment = item_id(KEPT);
    let composer = until_adopted(&shell, "Relaunched.", cx).await;
    assert!(
        composer.read_with(cx, |composer, _| composer.is_read_only()),
        "submitted before: read-only until its request is sent again"
    );
    assert!(
        comment_posts(&shell, cx).is_empty(),
        "opening sends nothing"
    );
    assert!(failed_actions(&shell, cx).contains(&"composer-retry"));

    press(&shell, "composer-retry", cx);
    until_attempt(&shell, &root, &comment, 1, cx).await;
    assert_eq!(
        shell.backend.jobs_seen(),
        vec![JobKind::Comment],
        "the same request first, no refresh"
    );
    assert_eq!(
        comment_posts(&shell, cx),
        vec![(comment.clone(), RequestId::parse(KEPT).unwrap(), 1)]
    );
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, _, resend)| (outcome, resend)),
        Some((CommentOutcome::SavedHere, true))
    );
    shell
        .until_drawn(cx, "the draft retired", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    assert_eq!(
        composer_state(&new_composer(&shell, cx), cx),
        (String::new(), false)
    );
    assert_eq!(listed(&shell, cx), 2, "listed once");
}

/// P12: the same request whose text differs from its record answers
/// `request_mismatch`: never Retry or Give up, which would loop. The text
/// is kept with Copy text and Discard, and the comment the record saved is
/// listed.
#[gpui_kit::test]
async fn p12_a_mismatch_keeps_the_text_and_offers_no_retry(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        let posted = post_through(fixture, root, KEPT, "Original.");
        assert_eq!(posted.code, ResultCode::Ok, "{}", posted.message);
        store_comment(fixture, root, KEPT, None, "Changed.");
    });
    let comment = item_id(KEPT);
    let composer = until_adopted(&shell, "Changed.", cx).await;
    assert!(composer.read_with(cx, |composer, _| composer.is_read_only()));
    press(&shell, "composer-retry", cx);
    until_attempt(&shell, &root, &comment, 1, cx).await;

    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::Mismatched { saved: true })
    );
    assert_eq!(
        failed_actions(&shell, cx),
        vec!["composer-copy-text", "composer-discard"]
    );
    assert_eq!(composer_state(&composer, cx), ("Changed.".to_owned(), true));
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1, "the text's draft is kept");
    assert_eq!(listed(&shell, cx), 2, "the saved comment is shown");
}
