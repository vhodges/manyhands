//! Owner decisions 24 and 25 (Task 13 design checkpoint): Check again
//! that finds no record but the comment listed retires the draft claiming
//! nothing; a tab reopened after an unsaved post finished shows that
//! post's state in its composer.

use gpui_kit::TestAppContext;
use manyhands::results::ResultCode;

use super::super::super::discussion_tests::{
    commit_comment, composer_text, new_composer, stored_comments,
};
use super::support::{
    KEPT, chip, composer_state, failed_actions, listed, maybe_saved_without_record, not_saved,
    open_ticket, outcome, posting_shell, press, reserved, saved_here, store_comment, submit,
    until_answered, until_post_runs, write_new,
};
use crate::desktop::{
    model::publication::CommentOutcome, services::JobKind, test_support::fixtures::item_id,
};

/// Decision 24: Check again with no record, the comment listed: the draft
/// is retired and the composer cleared, the comment shows in the thread,
/// and neither "Saved on this computer" nor the chip is claimed.
#[gpui_kit::test]
async fn check_again_with_no_record_and_the_comment_listed_retires_quietly(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Listed.\n");
    });
    let comment = item_id(KEPT);
    maybe_saved_without_record(&shell, &root, "Listed.\n", cx).await;
    // Saved meanwhile by other means: committed, its record gone.
    commit_comment(&shell.fixture, &root, KEPT, None, 4, None, "Listed.\n");

    press(&shell, "composer-check-again", cx);
    shell
        .until_drawn(cx, "the draft retired", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| listed(shell, cx) == 2)
        .await;
    assert_eq!(
        composer_state(&new_composer(&shell, cx), cx),
        (String::new(), false)
    );
    assert!(!reserved(&shell, &root, &comment, cx), "nothing claimed");
    assert!(!shell.present(saved_here(&comment), cx));
    assert!(!shell.present(chip(&comment), cx));
}

/// Decision 25: a post that finished unsaved while its tab was closed is
/// shown when the tab opens again: its draft adopted into the untouched,
/// empty composer, with the post's failed state and actions.
#[gpui_kit::test]
async fn a_reopened_tab_shows_an_unsaved_posts_failed_state(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Not saved.", cx);
    let held = shell.backend.hold_job(JobKind::Comment);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    held.wait_reached();
    shell.click(("tab-close", 0usize), cx);
    held.release();
    returned.release();
    until_answered(&shell, &root, &comment, cx).await;
    assert!(matches!(
        outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::NotSaved { .. }, ..))
    ));

    open_ticket(&shell, &root, cx);
    shell
        .until_drawn(cx, "the draft adopted, failed", |shell, cx| {
            shell.present("composer-failed", cx)
        })
        .await;
    let composer = new_composer(&shell, cx);
    assert_eq!(composer_text(&composer, cx), "Not saved.");
    assert_eq!(failed_actions(&shell, cx), vec!["composer-edit-text"]);
}
