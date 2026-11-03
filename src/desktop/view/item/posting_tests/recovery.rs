//! The required outcomes of posting recovery within one session (design,
//! P1-P3, P5-P10, P13, P15-P18): one test per row. Where no fixture
//! reaches a row, the post's real library call still runs and a
//! constructed answer stands in for its answer (`DelayingBackend`); the
//! record state is always the real one, read by the worker.
//!
//! P14 (a Give up whose re-send still answers with the record accepted
//! offers the step that lets it finish, not Give up again) is Lane B's
//! `view::operations::give_up_tests::a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up`,
//! over the same `CommentResult::offers` (Task 3's `after_give_up`).

use gpui_kit::TestAppContext;
use manyhands::{
    repository::RequestId,
    results::{CheckpointEffect, Envelope, PublicationEffect, ResultCode, Scope},
};

use super::super::super::discussion_tests::{
    composer_target, composer_text, flushed, new_composer, stored_comments, type_into,
};
use super::support::{
    ALREADY_POSTING, CHIP, KEPT, MAY_STILL_BE_SAVED, MAYBE_SAVED, checkpoints, chip, chip_control,
    comment_files, comment_posts, comment_source, composer_state, failed_actions, focus_end,
    hold_after_acceptance, internal_error, listed, not_saved, outcome, pending_row, post_request,
    posting_shell, reserved, shared, shares, store_comment, submit, until_adopted, until_answered,
    until_attempt, until_post_runs, write_new, written,
};
use crate::desktop::{
    model::publication::CommentOutcome,
    services::{
        JobKind,
        requests::{Intent, RequestState},
        submissions::SubmissionState,
        worker::{CancelResult, RecordState},
    },
    test_support::fixtures::item_id,
    view::{
        harness::{Shell, WAIT},
        operations::request_id,
    },
};

/// P1: a post whose checkpoint commits and whose publication succeeds is
/// one file and one checkpoint, listed once with no pending row and no
/// chip; its draft is retired and the composer cleared.
#[gpui_kit::test]
async fn p1_a_shared_post_is_one_file_one_checkpoint_shown_once(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Shared once.", cx);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, shared(&comment));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;

    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::Shared)
    );
    let files = comment_files(&root);
    assert_eq!(files.len(), 2, "A and the new comment: {files:?}");
    assert!(files.contains(&format!("{comment}.md")));
    assert_eq!(checkpoints(&root), 1);
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| listed(shell, cx) == 2)
        .await;
    assert!(!shell.present(pending_row(&comment), cx), "no pending row");
    assert!(!shell.present(chip(&comment), cx), "shared: no chip");
    shell
        .until_drawn(cx, "the draft retired", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    let composer = new_composer(&shell, cx);
    assert_eq!(composer_state(&composer, cx), (String::new(), false));
    assert_eq!(composer_target(&shell, &composer, cx), None, "cleared");
}

/// P2 (U-2, real library): over an SSH remote the transport is not
/// initialized, so a post is saved and its publication owed. Retry
/// sharing, pressed twice, sends the same request each time and writes no
/// further file or checkpoint; the chip stays until shared.
#[gpui_kit::test]
async fn p2_retry_sharing_writes_nothing_more(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Owed publication.", cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;

    // U-2: internal_error, committed, publication pending, record open.
    let id = post_request(&shell, &comment, cx).unwrap();
    let state = shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(_, code, effects, ..) => {
                Some((*code, effects.checkpoint, effects.publication))
            }
            _ => None,
        }
    });
    assert_eq!(
        state,
        Some((
            ResultCode::InternalError,
            CheckpointEffect::Committed,
            PublicationEffect::Pending
        ))
    );
    assert_eq!(
        outcome(&shell, &root, &comment, cx),
        Some((
            CommentOutcome::SavedNotShared {
                code: ResultCode::InternalError
            },
            RecordState::Accepted,
            false
        ))
    );
    assert_eq!(shell.label(chip(&comment), cx).as_deref(), Some(CHIP));

    for attempt in [2, 3] {
        shell.click(chip_control(&comment, "retry-sharing"), cx);
        until_attempt(&shell, &root, &comment, attempt, cx).await;
        assert_eq!(shell.label(chip(&comment), cx).as_deref(), Some(CHIP));
    }
    let posts = comment_posts(&shell, cx);
    assert_eq!(
        posts,
        vec![(
            comment.clone(),
            RequestId::parse(&comment.to_string()).unwrap(),
            3
        )],
        "one request, sent three times"
    );
    assert_eq!(comment_files(&root).len(), 2, "no further file");
    assert_eq!(checkpoints(&root), 1, "no further checkpoint");
    assert_eq!(listed(&shell, cx), 2, "listed once");
}

/// P3: a post stopped during its publication is saved and its publication
/// cancelled, which is final: the chip offers Share, a new `item sync`,
/// and never the same request nor Give up.
#[gpui_kit::test]
async fn p3_a_stopped_post_offers_share_not_retry(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Stopped.", cx);
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
    shell.click(request_id(id, "stop"), cx);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert!(worker.running_job().is_some_and(|job| job.cancelled));
    drop(release);
    until_answered(&shell, &root, &comment, cx).await;

    assert_eq!(
        outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::SavedStopped, RecordState::Finished, false))
    );
    assert_eq!(checkpoints(&root), 1);
    assert_eq!(shell.label(chip(&comment), cx).as_deref(), Some(CHIP));
    assert!(shell.present(chip_control(&comment, "share"), cx));
    assert!(!shell.present(chip_control(&comment, "retry-sharing"), cx));
    assert!(!shell.present(chip_control(&comment, "unlock"), cx));
    assert!(!shell.present(chip_control(&comment, "give-up"), cx));

    let before = shares(&shell, cx);
    shell.click(chip_control(&comment, "share"), cx);
    assert_eq!(
        shares(&shell, cx),
        before + 1,
        "Share sends a new item sync"
    );
    assert_eq!(comment_posts(&shell, cx).len(), 1, "and no post again");
}

/// P5: `MaybeSaved` says "This comment may already be saved." and offers
/// Check again only. Check again finds the record and the listed comment,
/// so no Post as a new comment is offered: the same request is sent again,
/// replays its saved answer and the draft is retired.
#[gpui_kit::test]
async fn p5_post_as_new_needs_no_record_and_no_listed_comment(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        store_comment(fixture, root, KEPT, None, "Maybe.");
    });
    let comment = item_id(KEPT);
    let composer = until_adopted(&shell, "Maybe.", cx).await;
    // The real call saves it here; the answer the desktop sees says it
    // failed inside the library.
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, internal_error());
    returned.release();
    focus_end(&shell, &composer, cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;

    assert_eq!(
        outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::MaybeSaved, RecordState::Finished, true))
    );
    assert_eq!(
        shell.label("composer-failed", cx).as_deref(),
        Some(MAYBE_SAVED)
    );
    assert_eq!(failed_actions(&shell, cx), vec!["composer-check-again"]);

    shell.click("composer-check-again", cx);
    until_attempt(&shell, &root, &comment, 2, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedHere)
    );
    assert!(!shell.present("composer-post-as-new", cx));
    assert_eq!(comment_posts(&shell, cx).len(), 1, "no new comment ID");
    shell
        .until_drawn(cx, "the draft retired", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;
    assert_eq!(listed(&shell, cx), 2);
}

/// P6: a post that wrote nothing and whose record is finished offers Edit
/// text; the edited text posts under a new comment ID, and so a new
/// request; the old post is forgotten.
#[gpui_kit::test]
async fn p6_edited_text_after_a_finished_failure_posts_under_a_new_comment_id(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, first) = write_new(&shell, "First.", cx);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::ItemNotFound));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &first, cx).await;
    assert_eq!(
        outcome(&shell, &root, &first, cx),
        Some((
            CommentOutcome::NotSaved {
                code: ResultCode::ItemNotFound
            },
            RecordState::Finished,
            false
        ))
    );
    assert_eq!(failed_actions(&shell, cx), vec!["composer-edit-text"]);

    shell.click("composer-edit-text", cx);
    assert_eq!(composer_state(&composer, cx), ("First.".to_owned(), false));
    type_into(&shell, &composer, " Edited.", cx);
    flushed(&shell, cx);
    submit(&shell, cx);
    let posts = comment_posts(&shell, cx);
    assert_eq!(posts.len(), 2, "{posts:?}");
    let (second, request, _) = posts[1].clone();
    assert_ne!(second, first, "a new comment ID");
    assert_eq!(request, RequestId::parse(&second.to_string()).unwrap());
    until_answered(&shell, &root, &second, cx).await;
    assert!(
        !reserved(&shell, &root, &first, cx),
        "the old post is forgotten"
    );
    assert_eq!(
        outcome(&shell, &root, &second, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedHere)
    );
    assert!(
        comment_source(&root, &second).is_some_and(|source| source.contains("First. Edited.")),
        "the edited text is posted"
    );
}

/// P7: a post whose record is still open (here a transient code) offers
/// Retry, Discard and Copy text, never Edit text; Discard says the comment
/// may still be saved, and the comment ID stays reserved: the next text
/// gets a new one. A not-saved `identity_required` adds Set identity.
#[gpui_kit::test]
async fn p7_an_owed_failure_offers_retry_discard_and_copy(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (composer, comment) = write_new(&shell, "Owed.", cx);
    let returned = shell.backend.hold_job_return_answering(
        JobKind::Comment,
        not_saved(ResultCode::TransportUnavailable),
    );
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx),
        Some((
            CommentOutcome::NotSavedOwed {
                code: ResultCode::TransportUnavailable
            },
            RecordState::Accepted,
            false
        ))
    );
    assert_eq!(
        failed_actions(&shell, cx),
        vec!["composer-retry", "composer-copy-text", "composer-discard"]
    );
    assert_eq!(composer_state(&composer, cx), ("Owed.".to_owned(), true));

    shell.click("composer-copy-text", cx);
    assert_eq!(
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some("Owed.")
    );
    shell.click("composer-discard", cx);
    assert_eq!(
        shell.label("composer-discard-note", cx).as_deref(),
        Some(MAY_STILL_BE_SAVED)
    );
    shell.click("composer-discard-confirm", cx);
    assert!(
        reserved(&shell, &root, &comment, cx),
        "the comment ID stays reserved"
    );
    let next = new_composer(&shell, cx);
    assert_eq!(composer_text(&next, cx), "");
    type_into(&shell, &next, "Next.", cx);
    flushed(&shell, cx);
    let target = composer_target(&shell, &next, cx).expect("a new comment ID");
    assert_ne!(target.comment_id, comment);

    // identity_required, not saved: Set identity, then Retry.
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::IdentityRequired));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &target.comment_id, cx).await;
    assert_eq!(
        failed_actions(&shell, cx),
        vec![
            "composer-retry",
            "composer-set-identity",
            "composer-copy-text",
            "composer-discard"
        ]
    );
}

/// P8: `occupied_path` keeps the text and its draft and offers Discard
/// only; nothing retires the draft.
#[gpui_kit::test]
async fn p8_occupied_path_keeps_the_text_and_offers_discard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (composer, comment) = write_new(&shell, "Occupied.", cx);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, not_saved(ResultCode::OccupiedPath));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::NotSavedConflicting)
    );
    assert_eq!(failed_actions(&shell, cx), vec!["composer-discard"]);
    assert_eq!(
        composer_state(&composer, cx),
        ("Occupied.".to_owned(), true)
    );
    flushed(&shell, cx);
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1, "the draft is kept");
    assert_eq!(stored[0].0.comment_id, comment);
    assert_eq!(stored[0].1, "Occupied.");
}

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
    shell.click("composer-edit-text", cx);
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
    shell.click("composer-retry", cx);
    until_attempt(&shell, &root, &comment, 2, cx).await;
    assert_eq!(
        outcome(&shell, &root, &comment, cx).map(|(outcome, _, resend)| (outcome, resend)),
        Some((CommentOutcome::MaybeSaved, true))
    );
    assert_eq!(
        shell.label("composer-failed", cx).as_deref(),
        Some(MAYBE_SAVED)
    );
    assert_eq!(failed_actions(&shell, cx), vec!["composer-check-again"]);
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
    shell.click("composer-check-again", cx);
    shell
        .until(cx, |operations| {
            operations.requests().iter().any(|request| {
                matches!(request.intent, Intent::IndexRefresh { .. })
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    shell.run(cx);
    assert_eq!(refreshes(&shell, cx), before + 1);
    assert!(!shell.present("composer-post-as-new", cx));
    assert!(shell.present("composer-check-again", cx));

    // A refresh that succeeds, no record and the comment not listed.
    shell.click("composer-check-again", cx);
    shell
        .until_drawn(cx, "Post as a new comment offered", |shell, cx| {
            shell.present("composer-post-as-new", cx)
        })
        .await;
    assert_eq!(refreshes(&shell, cx), before + 2);
    shell.click("composer-post-as-new", cx);
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
    shell.click("composer-discard", cx);
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
