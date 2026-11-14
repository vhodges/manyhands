//! Task 14: after a post, the thread is read again (fact 28), first
//! through an index refresh when the post left discovery pending, with
//! "Updating the list…" in the header; the comments keep their order and
//! parents across an index rebuild and a reopened tab (A6); and the "not
//! shared yet" chip lives in `Submissions` for the session, so a reload
//! keeps it and a share of the item clears it (decision 14).

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, CommentMutationDto, CommentMutationType, CommentPendingReason,
        MutationDataDto,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, PublicationEffect, ResultCode, Scope,
        WriteEffect,
    },
};

use super::super::super::discussion_tests::{
    A, TICKET, flushed, pane, press_reply, reply_composer, type_into,
};
use super::support::{
    CHIP, chip, listed, open_ticket, posting_shell, submit, until_answered, write_new,
};
use crate::desktop::{
    model::{list_state::ListState, publication::CommentOutcome},
    services::{
        JobKind, ReadKind,
        requests::{Intent, Lane},
    },
    view::{harness::Shell, item::pill::UPDATING_LIST},
};

/// A post's answer: its comment saved here, with no publication remote,
/// and the index not yet up to date with it (`discovery: pending`).
fn index_pending(comment: &ItemId) -> Envelope<MutationDataDto> {
    let effects = Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        discovery: DiscoveryEffect::Pending,
        ..Effects::not_requested()
    };
    Envelope::classified_mutation(
        "comment add",
        Scope::default(),
        false,
        ResultCode::DiscoveryPending,
        effects,
        true,
    )
    .with_data(MutationDataDto::Comment(CommentMutationDto {
        r#type: CommentMutationType::Comment,
        item_id: TICKET.to_owned(),
        comment_id: comment.to_string(),
        parent_id: None,
        created_by: None,
        synchronization_id: "01M4CC0VN1KTQFSVR99378TDHC".to_owned(),
        publication: PublicationEffect::NotRequested,
        pending_reason: Some(CommentPendingReason::NoPublicationRemote),
        conflict: None,
    }))
}

/// A share of the item that published it.
fn item_published() -> Envelope<MutationDataDto> {
    let effects = Effects {
        publication: PublicationEffect::Published,
        ..Effects::not_requested()
    };
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::Ok,
        effects,
        false,
    )
}

/// The comments the tab last read: each one's ID and parent, in order.
fn thread_order(shell: &Shell, cx: &mut TestAppContext) -> Vec<(String, Option<String>)> {
    pane(shell, cx).read_with(cx, |pane, _| {
        match pane.model.comments().and_then(ListState::rows) {
            Some(list) => list
                .items
                .iter()
                .map(|comment| {
                    (
                        comment.id.clone().unwrap_or_default(),
                        comment.parent_id.clone(),
                    )
                })
                .collect(),
            None => Vec::new(),
        }
    })
}

/// Reads the tab again and waits for its comments to land.
async fn reload_tab(shell: &Shell, cx: &mut TestAppContext) {
    let before = shell.backend.reads_seen(ReadKind::Comments);
    pane(shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell
        .until_drawn(cx, "the reload", |shell, _| {
            shell.backend.reads_seen(ReadKind::Comments) > before
        })
        .await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// A committed post reads the thread again (fact 28): the new comment is
/// listed once its answer lands, with no pending row left.
#[gpui_kit::test]
async fn a_committed_comment_reloads_the_thread(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    assert_eq!(listed(&shell, cx), 1);
    let (_, comment) = write_new(&shell, "Read again.", cx);
    let before = shell.backend.reads_seen(ReadKind::Comments);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;

    shell
        .until_drawn(cx, "the thread read again", |shell, cx| {
            shell.backend.reads_seen(ReadKind::Comments) > before && listed(shell, cx) == 2
        })
        .await;
    let order = thread_order(&shell, cx);
    assert_eq!(order.last(), Some(&(comment.to_string(), None)));
    assert!(!shell.present(super::support::pending_row(&comment), cx));
}

/// A post whose answer leaves discovery pending queues an index refresh
/// first, and the header says "Updating the list…" while it runs; once it
/// has ended the thread is read again and lists the comment.
#[gpui_kit::test]
async fn a_pending_discovery_refreshes_then_reloads(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Index pending.", cx);
    let refresh = shell.backend.hold_job(JobKind::IndexRefresh);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, index_pending(&comment));
    returned.release();
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert_eq!(
        super::support::outcome(&shell, &root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::SavedHere)
    );

    shell
        .until_drawn(cx, "an index refresh queued", |shell, cx| {
            !shell
                .requests(cx, |request| {
                    matches!(&request.intent, Intent::IndexRefresh { root: asked } if asked == &root)
                })
                .is_empty()
        })
        .await;
    refresh.wait_reached();
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some(UPDATING_LIST),
        "the header says the list is being updated"
    );

    let before = shell.backend.reads_seen(ReadKind::Comments);
    refresh.release();
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(id, cx).await;
    shell
        .until_drawn(cx, "the thread read after the refresh", |shell, cx| {
            shell.backend.reads_seen(ReadKind::Comments) > before
                && shell.label("item-pill", cx).as_deref() != Some(UPDATING_LIST)
        })
        .await;
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| listed(shell, cx) == 2)
        .await;
    assert_eq!(
        shell
            .requests(cx, |request| matches!(
                request.intent,
                Intent::IndexRefresh { .. }
            ))
            .len(),
        1,
        "one refresh"
    );
}

/// A6: a new comment and a reply posted here keep their order and their
/// parents once the index is rebuilt and the tab is closed and opened
/// again.
#[gpui_kit::test]
async fn order_and_parents_hold_after_rebuild_and_reopen(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, first) = write_new(&shell, "A new thread.", cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &first, cx).await;
    shell
        .until_drawn(cx, "the new comment listed", |shell, cx| {
            listed(shell, cx) == 2
        })
        .await;

    press_reply(&shell, 0, cx);
    let (thread, reply) = reply_composer(&shell, cx).expect("the reply composer");
    assert_eq!(thread, A);
    type_into(&shell, &reply, "An answer.", cx);
    flushed(&shell, cx);
    let answer = pane(&shell, cx)
        .read_with(cx, |pane, _| pane.composer_target(&reply))
        .expect("the reply's ID")
        .comment_id;
    submit(&shell, cx);
    until_answered(&shell, &root, &answer, cx).await;
    shell
        .until_drawn(cx, "the reply listed", |shell, cx| listed(shell, cx) == 3)
        .await;
    let before = thread_order(&shell, cx);
    assert_eq!(before.len(), 3);
    assert!(before.contains(&(first.to_string(), None)));
    assert!(before.contains(&(answer.to_string(), Some(A.to_owned()))));

    rebuild(&shell, &root, cx).await;
    shell.click(("tab-close", 0usize), cx);
    open_ticket(&shell, &root, cx);
    shell
        .until_drawn(cx, "the reopened thread", |shell, cx| {
            listed(shell, cx) == 3
        })
        .await;
    assert_eq!(thread_order(&shell, cx), before);
}

/// Rebuilds the index of `root` and waits for it.
async fn rebuild(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let id = shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::IndexRebuild {
                root: root.to_owned(),
            },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(id, cx).await;
    shell.run(cx);
}

/// Decision 14: the chip of a saved post that is not shared lives in
/// `Submissions` for the session, so a reload of the tab keeps it; a
/// share of the item that publishes clears it, and a later reload does
/// not bring it back.
#[gpui_kit::test]
async fn the_chip_survives_a_reload_and_clears_when_shared(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(true, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Not shared yet.", cx);
    submit(&shell, cx);
    until_answered(&shell, &root, &comment, cx).await;
    assert!(matches!(
        super::support::outcome(&shell, &root, &comment, cx),
        Some((CommentOutcome::SavedNotShared { .. }, ..))
    ));
    shell
        .until_drawn(cx, "the chip", |shell, cx| {
            shell.label(chip(&comment), cx).as_deref() == Some(CHIP)
        })
        .await;

    reload_tab(&shell, cx).await;
    assert_eq!(shell.label(chip(&comment), cx).as_deref(), Some(CHIP));

    let shared = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, item_published());
    shared.release();
    shell.click(crate::desktop::view::item::share::SHARE_BUTTON, cx);
    let share = shell
        .requests(cx, |request| {
            matches!(request.intent, Intent::ItemSync { .. })
        })
        .first()
        .copied()
        .expect("Share sent an item sync");
    shell.until_ended(share, cx).await;
    shell
        .until_drawn(cx, "the chip cleared", |shell, cx| {
            !shell.present(chip(&comment), cx)
        })
        .await;

    reload_tab(&shell, cx).await;
    assert!(
        !shell.present(chip(&comment), cx),
        "cleared for the session"
    );
    assert_eq!(listed(&shell, cx), 2);
}

/// D4 K-19, D5 K-15: any share of the item that answers Published clears
/// the chip of its saved posts, whatever sent it: a Republish… (the branch
/// the remote deleted pushed again), and a close or an approval that
/// publishes the work it merges, as a Share does.
#[gpui_kit::test]
async fn a_republish_or_a_lifecycle_that_publishes_clears_the_chip(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    for kind in [
        JobKind::ItemRepublish,
        JobKind::TicketClose,
        JobKind::DocumentPromote,
    ] {
        let (shell, root) = posting_shell(true, cx, |_, _| {});
        let (_, comment) = write_new(&shell, "Not shared yet.", cx);
        submit(&shell, cx);
        until_answered(&shell, &root, &comment, cx).await;
        shell
            .until_drawn(cx, "the chip", |shell, cx| {
                shell.label(chip(&comment), cx).as_deref() == Some(CHIP)
            })
            .await;

        let item: ItemId = TICKET.parse().unwrap();
        let (root, id) = (root.clone(), item.clone());
        let intent = match kind {
            JobKind::ItemRepublish => Intent::ItemRepublish {
                root,
                kind: AuthoringKind::Ticket,
                id,
            },
            JobKind::TicketClose => Intent::TicketClose { root, id },
            _ => Intent::DocumentPromote { root, id },
        };
        shell
            .backend
            .hold_job_return_answering(kind, item_published())
            .release();
        let worker = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.services().worker().clone());
        let sent = worker.submit(intent, Lane::User, None, None);
        shell.until_ended(sent, cx).await;
        shell
            .until_drawn(cx, "the chip cleared", |shell, cx| {
                !shell.present(chip(&comment), cx)
            })
            .await;
    }
}

/// P9 with pending discovery: the tab closed while its post runs and
/// opened again before the answer; the answer, with discovery pending,
/// still queues the refresh from the reopened tab and the header says so.
#[gpui_kit::test]
async fn a_tab_reopened_while_posting_refreshes_on_pending_discovery(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = posting_shell(false, cx, |_, _| {});
    let (_, comment) = write_new(&shell, "Reopened meanwhile.", cx);
    let held = shell.backend.hold_job(JobKind::Comment);
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::Comment, index_pending(&comment));
    returned.release();
    let refresh = shell.backend.hold_job(JobKind::IndexRefresh);
    submit(&shell, cx);
    super::support::until_post_runs(&shell, cx).await;
    held.wait_reached();
    shell.click(("tab-close", 0usize), cx);
    open_ticket(&shell, &root, cx);

    held.release();
    until_answered(&shell, &root, &comment, cx).await;
    shell
        .until_drawn(cx, "an index refresh queued", |shell, cx| {
            !shell
                .requests(cx, |request| {
                    matches!(&request.intent, Intent::IndexRefresh { root: asked } if asked == &root)
                })
                .is_empty()
        })
        .await;
    refresh.wait_reached();
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(UPDATING_LIST));
    refresh.release();
    shell
        .until_drawn(cx, "the comment listed", |shell, cx| {
            listed(shell, cx) == 2 && shell.label("item-pill", cx).as_deref() != Some(UPDATING_LIST)
        })
        .await;
}
