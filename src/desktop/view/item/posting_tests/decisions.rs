//! Owner decisions 24 and 25 (Task 13 design checkpoint): Check again
//! that finds no record but the comment listed retires the draft claiming
//! nothing; a tab reopened after an unsaved post finished shows that
//! post's state in its composer. Decision 13 for a post: a saved comment
//! whose synchronization stopped on a conflict raises the item's banner.

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::results::ResultCode;

use super::super::super::discussion_tests::{
    commit_comment, composer_text, new_composer, stored_comments,
};
use super::support::{
    CHIP, KEPT, chip, chip_control, composer_state, failed_actions, listed,
    maybe_saved_without_record, not_saved, open_ticket, outcome, post_request, posting_shell,
    press, reserved, saved_conflict, saved_here, store_comment, submit, until_answered,
    until_post_runs, write_new,
};
use crate::desktop::{
    model::{operations::CONFLICT_BANNER, publication::CommentOutcome},
    services::JobKind,
    test_support::fixtures::item_id,
    view::operations::request_id,
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

/// The elements drawn inside the frame of the banner whose words are
/// `words`, besides those words: none when it offers no action.
fn banner_extras(
    shell: &crate::desktop::view::harness::Shell,
    words: &'static str,
    cx: &mut TestAppContext,
) -> usize {
    shell.frame(cx, |window, _| {
        let words = window.try_find(words).expect("the banner").path().to_vec();
        let frame = &words[..words.len() - 1];
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|element| {
                let path = element.path();
                path.starts_with(frame) && path != frame && path != words.as_slice()
            })
            .count()
    })
}

/// Decision 13 for a post (design, Posting, `SavedConflict`; contract 7,
/// A10): a comment saved whose synchronization stopped on a conflict is
/// listed with its chip, which offers nothing, and raises the item's
/// conflict banner, which offers nothing either. Dismissing the post's
/// entry ends the banner.
#[gpui_kit::test]
async fn a_post_stopped_on_a_conflict_raises_the_items_banner(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Conflicted.", cx);
    assert!(!shell.present("item-conflict", cx));
    // Saved by the real call; the desktop hears its synchronization
    // stopped on a conflict.
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, saved_conflict(&comment));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert!(matches!(
        outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::SavedConflict { .. }, ..))
    ));
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| listed(shell, cx) == 2)
        .await;

    assert_eq!(
        shell.label("item-conflict", cx).as_deref(),
        Some(CONFLICT_BANNER)
    );
    assert_eq!(
        banner_extras(&shell, "item-conflict", cx),
        0,
        "the banner offers no action"
    );
    assert_eq!(shell.label(chip(&comment), cx).as_deref(), Some(CHIP));
    for control in [
        "retry-sharing",
        "unlock",
        "give-up",
        "share",
        "open-keys",
        "check-host",
        "set-identity",
        "check-again",
    ] {
        assert!(
            !shell.present(chip_control(&comment, control), cx),
            "{control}"
        );
    }

    let id = post_request(&shell, &comment, cx).unwrap();
    shell.click(request_id(id, "dismiss"), cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("item-conflict", cx), "dismissed");
}
