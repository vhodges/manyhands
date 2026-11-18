//! The required outcomes of posting recovery, continued (P9, P10, P13,
//! P15-P18): see `recovery`.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::RequestId,
    results::{Envelope, ResultCode, Scope},
};

use super::super::super::discussion_tests::{
    composer_text, flushed, new_composer, stored_comments, type_into,
};
use super::support::{
    ALREADY_POSTING, KEPT, MAYBE_SAVED, chip, comment_posts, comment_source, failed_actions,
    focus_end, internal_error, listed, not_saved, outcome, pending_row, posting_shell, press,
    store_comment, submit, until_adopted, until_answered, until_attempt, until_post_runs,
    write_new, written,
};
use crate::desktop::{
    model::publication::CommentOutcome,
    services::{
        JobKind,
        requests::Intent,
        submissions::SubmissionState,
        worker::{CancelResult, RecordState},
    },
    test_support::fixtures::item_id,
    view::harness::Shell,
};

/// P9: the tab closed while posting, the outcome still lands in
/// `Submissions` from the drain, and the draft is retired there on the
/// committed checkpoint; reopened, the tab lists the comment.
#[gpui_kit::test]
async fn p9_closing_the_tab_while_posting_still_retires_the_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Closed tab.", cx);
    let held = shell.backend.hold_job(JobKind::Comment);
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    held.wait_reached();
    shell.click(("tab-close", 0usize), cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())
            .is_none(),
        "the tab closed while posting"
    );
    held.release();
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedHere)
    );
    shell
        .until_drawn(cx, "the draft retired with no tab", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;

    super::support::open_ticket(&shell, &root, cx);
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| listed(shell, cx) == 2)
        .await;
    assert_eq!(composer_text(&new_composer(&shell, cx), cx), "");
}

/// P10: a second Submit while posting is refused and says so; one
/// request is sent.
#[gpui_kit::test]
async fn p10_a_second_submit_while_posting_is_refused(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, comment) = write_new(&shell, "Once.", cx);
    let held = shell.backend.hold_job(JobKind::Comment);
    submit(&shell, cx);
    assert!(composer.read_with(cx, |composer, _| composer.is_posting()));
    until_post_runs(&shell, cx).await;
    held.wait_reached();
    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    assert_eq!(
        shell.label("composer-refused", cx).as_deref(),
        Some(ALREADY_POSTING)
    );
    assert_eq!(comment_posts(&shell, cx).len(), 1);
    held.release();
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(comment_posts(&shell, cx).len(), 1);
}

/// P13: Edit text after a finished failure re-targets the draft before
/// the new Submit: while the new post is held before its library call,
/// the store holds only the draft of the new comment ID, with the edited
/// text, so a crash then adopts the new ID and never the old.
#[gpui_kit::test]
async fn p13_edited_text_retargets_the_draft_before_posting(cx: &mut TestAppContext) {
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
    type_into(&shell, &composer, " Edited.", cx);
    flushed(&shell, cx);

    let held = shell.backend.hold_job(JobKind::Comment);
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    held.wait_reached();
    let posts = comment_posts(&shell, cx);
    let second = posts.last().unwrap().0.clone();
    assert_ne!(second, first);
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1, "one draft, the new comment ID's");
    assert_eq!(stored[0].0.comment_id, second);
    assert_eq!(stored[0].1, "First. Edited.");
    held.release();
    until_answered(&shell, &root, &second, cx).await;
}

/// P15: `internal_error` on a re-send of a comment ID, here a Retry of an
/// owed post, may be saved: Check again only, never Edit text.
#[gpui_kit::test]
async fn p15_internal_error_on_any_resend_is_maybe_saved(cx: &mut TestAppContext) {
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
    assert!(failed_actions(&shell, cx).contains(&"composer-retry"));

    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    returned.release();
    press(&shell, "composer-retry", cx);
    until_attempt(&shell, &root, &comment, 2, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, _, resend)| (outcome, resend)),
        Some((CommentOutcome::MaybeSaved, true))
    );
    assert_eq!(
        shell.label("composer-failed", cx).as_deref(),
        Some(MAYBE_SAVED)
    );
    assert_eq!(
        failed_actions(&shell, cx),
        vec![
            "composer-check-again",
            "composer-copy-text",
            "composer-discard"
        ],
        "R-19: never trapped"
    );
}

/// P16: Post as a new comment is offered only after a successful
/// `IndexRefresh`, a complete current listing without the comment, and
/// `show_request` answering no record. A post stopped before its
/// acceptance leaves no record; its re-send's `internal_error` may be
/// saved. Check again whose refresh fails offers no Post as new; one whose
/// refresh succeeds does, and it posts the same text under a new ID.
#[gpui_kit::test]
async fn p16_post_as_new_needs_a_complete_current_listing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Fresh.");
    });
    let comment = item_id(KEPT);
    let composer = until_adopted(&shell, "Fresh.", cx).await;
    let held = shell.backend.hold_job(JobKind::Comment);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    let id = until_post_runs(&shell, cx).await;
    held.wait_reached();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert_eq!(worker.cancel(id), CancelResult::Signalled);
    held.release();
    returned.release();
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        shell
            .fixture
            .service()
            .show_request(RequestId::parse(KEPT).unwrap())
            .code,
        ResultCode::RequestNotFound,
        "stopped before its acceptance, the post left no record"
    );
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::MaybeSaved)
    );
    assert!(!shell.present("composer-post-as-new", cx));

    // A refresh that fails: no complete current listing.
    let refreshes = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .requests(cx, |request| {
                matches!(request.intent, Intent::IndexRefresh { .. })
            })
            .len()
    };
    let before = refreshes(&shell, cx);
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
    assert_eq!(refreshes(&shell, cx), before + 1);
    assert!(!shell.present("composer-post-as-new", cx));
    assert!(shell.present("composer-check-again", cx));

    // A refresh that succeeds, no record and the comment not listed.
    press(&shell, "composer-check-again", cx);
    shell
        .until_drawn(cx, "Post as a new comment offered", |shell, cx| {
            shell.present("composer-post-as-new", cx)
        })
        .await;
    assert_eq!(refreshes(&shell, cx), before + 2);
    press(&shell, "composer-post-as-new", cx);
    let posts = comment_posts(&shell, cx);
    let (fresh, request, _) = posts.last().unwrap().clone();
    assert_ne!(fresh, comment, "a new comment ID");
    assert_eq!(request, RequestId::parse(&fresh.to_string()).unwrap());
    until_answered(&shell, &root, &fresh, cx).await;
    assert!(comment_source(&root, &fresh).is_some_and(|source| source.contains("Fresh.")));
}

/// P17: where `show_request` fails after an attempt the record's state is
/// unknown: Retry and Check again only. The worker reads the record while
/// another holder has the index's recovery lock, which no read waits for.
#[gpui_kit::test]
async fn p17_an_unknown_record_offers_retry_and_check_only(cx: &mut TestAppContext) {
    use fs4::fs_std::FileExt;

    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Unknown.", cx);
    let returned = shell.backend.hold_job_return_answering(
        JobKind::Comment,
        not_saved(ResultCode::TransportUnavailable),
    );
    submit(&shell, cx);
    until_post_runs(&shell, cx).await;
    returned.wait_reached();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(
            shell
                .fixture
                .data_dir()
                .join("manyhands.sqlite3.recovery.lock"),
        )
        .unwrap();
    assert!(FileExt::try_lock_exclusive(&lock).unwrap());
    returned.release();
    until_answered(&shell, &root, &comment, cx).await;
    drop(lock);
    shell.run(cx);
    assert_eq!(
        outcome(&shell, &root, &comment, cx),
        Some((
            CommentOutcome::NotSavedOwed {
                code: ResultCode::TransportUnavailable
            },
            RecordState::Unknown,
            false
        ))
    );
    assert_eq!(
        failed_actions(&shell, cx),
        vec!["composer-retry", "composer-check-again"]
    );
}

/// P18: Discard after a written, uncommitted file says the file stays on
/// this computer and may still appear in the thread; no chip is drawn.
#[gpui_kit::test]
async fn p18_discard_after_a_written_file_says_it_stays(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Written.", cx);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, written(ResultCode::TransportUnavailable));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert!(matches!(
        outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::NotSavedOwed { .. }, _, _))
    ));
    press(&shell, "composer-discard", cx);
    let note = shell
        .label("composer-discard-note", cx)
        .expect("Discard says what happens to the file");
    assert!(note.contains("stays on this computer"), "{note}");
    assert!(note.contains("may still appear in the thread"), "{note}");
    shell.click("composer-discard-confirm", cx);
    assert!(!shell.present(chip(&comment), cx), "no chip");
    assert!(!shell.present(pending_row(&comment), cx));
    let submissions = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().submissions().clone()
    });
    submissions.read_with(cx, |submissions, _| {
        assert!(!matches!(
            submissions.get(&root, &comment).map(|post| &post.state),
            Some(SubmissionState::Posting)
        ));
    });
}
