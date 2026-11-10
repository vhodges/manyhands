//! Made ready (D5 design, A3, R-20), Task 8 review: Open ready tickets on
//! the repository already shown, a made-ready ticket deleted before its
//! title is read (K-8), and a title read overtaken by a newer one. Helpers
//! are `made_ready_tests.rs`'s. Canned answers are built from library types
//! and name the library test that produces the real one.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        ClosureFilter, ItemDtoKind, LifecycleCleanupPendingReason, LifecycleCleanupState,
        LifecycleMadeReadyDto, LifecycleMadeReadyState, LifecyclePublicationPendingReason,
        LifecyclePublicationState, ReadinessFilter, RequestId,
    },
    results::{CleanupEffect, Outcome, ResultCode},
};

use super::{
    confirm_tests::confirming,
    made_ready::{COULD_NOT_WORK_OUT, OPEN_READY_TICKETS},
    made_ready_tests::{DEPENDENT, DEPENDENT_TITLE, OTHER, report, until_rows, with_dependent},
    outcome::{ENDING_DONE, ENDING_FAILED},
    outcome_canned_tests::{answer, reached, reached_effects, retry},
    short_id,
    tests::{SLUG, TICKET, closes, dialog, saved_ticket, shell, until_present},
};
use crate::desktop::{
    model::navigation::Pane,
    services::{JobKind, ReadKind},
    test_support::fixtures::ticket_path,
    view::harness::{Shell, key},
};

const OTHER_TITLE: &str = "Ready one";

/// Waits until the dialog has read a title, or none, for `id`.
async fn until_read(shell: &Shell, id: &'static str, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, "the title read", move |_, cx| {
            dialog(cx).read_with(cx, |dialog, _| {
                dialog
                    .made_ready_titles()
                    .iter()
                    .any(|(read, _)| read == id)
            })
        })
        .await;
}

/// Review 1, canned: Open ready tickets on the repository already selected
/// sets Open and Ready on it, reads its ready tickets, and focuses the
/// ticket list itself, which is loaded. The close is real, held once its
/// library call returned and answered `index_unavailable_final`
/// (`tests/mutation_remote/c5_made_ready.rs`
/// `c5_made_ready_failed_read_is_final`).
#[gpui_kit::test]
async fn open_ready_tickets_on_the_shown_repository(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, OTHER_TITLE, "");
        saved_ticket(fixture, root, "Closing", "");
    });
    shell.select(&root, cx);
    shell.use_list(Pane::Tickets, cx);
    shell.workspace.update(cx, |workspace, cx| {
        workspace.set_closure(ClosureFilter::Closed, cx);
        workspace.set_readiness(Some(ReadinessFilter::Blocked), cx);
    });
    shell.run(cx);
    assert!(shell.selected("filter-closed", cx));
    assert!(shell.selected("filter-blocked", cx));

    confirming(&shell, &root, cx).await;
    let request = RequestId::new();
    let mut data = reached(ItemDtoKind::Ticket, TICKET, Vec::new());
    data.made_ready = Some(Vec::new());
    data.made_ready_state = Some(LifecycleMadeReadyState::IndexUnavailableFinal);
    let gate = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        answer(
            "ticket close",
            &root,
            ResultCode::Ok,
            Outcome::Success,
            data,
            reached_effects(CleanupEffect::Complete),
            Vec::new(),
            request,
        ),
    );
    shell.click("lifecycle-confirm", cx);
    let id = closes(&shell, cx)[0];
    gate.release();
    shell.until_ended(id, cx).await;
    until_present(&shell, "lifecycle-made-ready-unknown", cx).await;
    assert_eq!(
        shell.label("lifecycle-made-ready-unknown", cx).as_deref(),
        Some(COULD_NOT_WORK_OUT)
    );
    assert_eq!(
        shell.label("lifecycle-open-ready", cx).as_deref(),
        Some(OPEN_READY_TICKETS)
    );

    shell.click("lifecycle-open-ready", cx);
    shell.run(cx);
    assert!(!shell.present("lifecycle-dialog", cx), "the dialog closed");
    shell
        .until_drawn(cx, "the ready tickets", |shell, cx| {
            shell.rows("ticket-row", cx) == [OTHER_TITLE]
        })
        .await;
    let (selected, pane) = shell.workspace.read_with(cx, |workspace, _| {
        (
            workspace.navigation().selected().cloned(),
            workspace.navigation().pane(),
        )
    });
    assert_eq!((selected, pane), (Some(key(&root)), Pane::Tickets));
    assert!(shell.selected("filter-open", cx));
    assert!(shell.selected("filter-ready", cx));
    let list = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.tickets_list_focus(cx));
    assert!(
        shell.frame(cx, |window, cx| list.contains_focused(window, cx)),
        "the ticket list has focus"
    );
}

/// Review 3, K-8, real: a made-ready ticket deleted from main after the
/// close reported it, before its title is read, is shown by its short code
/// alone. The close's answer is the library's own, held once it returned.
#[gpui_kit::test]
async fn a_deleted_made_ready_ticket_shows_without_a_title(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        with_dependent(fixture, root, &format!("slug: {SLUG}\n"));
    });
    confirming(&shell, &root, cx).await;
    let gate = shell.backend.hold_job_return(JobKind::TicketClose);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    shell.fixture.commit_removal(&root, &ticket_path(DEPENDENT));
    shell.fixture.refresh(&root);
    gate.release();
    let id = closes(&shell, cx)[0];
    shell.until_ended(id, cx).await;
    assert_eq!(
        report(&shell, id, cx),
        (
            Some(vec![LifecycleMadeReadyDto {
                id: DEPENDENT.to_owned(),
                slug: Some(SLUG.to_owned()),
            }]),
            Some(LifecycleMadeReadyState::Current)
        ),
        "the library read it before the deletion"
    );
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_DONE)
    );

    until_read(&shell, DEPENDENT, cx).await;
    assert_eq!(
        dialog(cx).read_with(cx, |dialog, _| dialog.made_ready_titles()),
        [(DEPENDENT.to_owned(), None)]
    );
    assert_eq!(shell.rows("lifecycle-made-ready", cx), [SLUG]);
}

/// Review 3, canned: a title read that lands after a newer one has started
/// is dropped. A partial close lists `DEPENDENT`, and its title read is held;
/// Retry's answer lists `OTHER`, whose title is read and shown; the held read
/// then lands and changes nothing. The first answer is
/// `tests/mutation_remote/c5_made_ready.rs`
/// `c5_made_ready_partial_reports_and_retry_reads_again` (partial,
/// `recovery_required`, `current`, the dependent); the second
/// `c5_made_ready_many_in_id_order`'s shape (`current`, a null slug). Both
/// runs are real, each answered with its canned envelope.
#[gpui_kit::test]
async fn a_stale_title_read_is_dropped(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, OTHER_TITLE, "");
        with_dependent(fixture, root, &format!("slug: {SLUG}\n"));
    });
    confirming(&shell, &root, cx).await;

    let canned = RequestId::new();
    let mut partial = reached(ItemDtoKind::Ticket, TICKET, Vec::new());
    partial.publication = Some(LifecyclePublicationState::Pending);
    partial.publication_oid = None;
    partial.publication_pending = Some(LifecyclePublicationPendingReason::NoPublicationRemote);
    partial.cleanup = Some(LifecycleCleanupState::Pending);
    partial.cleanup_pending = Some(LifecycleCleanupPendingReason::ContextChanged);
    partial.made_ready = Some(vec![LifecycleMadeReadyDto {
        id: DEPENDENT.to_owned(),
        slug: Some(SLUG.to_owned()),
    }]);
    partial.made_ready_state = Some(LifecycleMadeReadyState::Current);
    let first = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        answer(
            "ticket close",
            &root,
            ResultCode::RecoveryRequired,
            Outcome::Partial,
            partial,
            reached_effects(CleanupEffect::Pending),
            vec![retry(canned)],
            canned,
        ),
    );
    let held = shell.backend.hold_read(ReadKind::MadeReady);
    shell.click("lifecycle-confirm", cx);
    let sent = closes(&shell, cx)[0];
    first.release();
    shell.until_ended(sent, cx).await;
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_FAILED)
    );
    shell.run(cx);
    assert_eq!(
        shell.rows("lifecycle-made-ready", cx),
        [SLUG],
        "its title read is held"
    );

    let mut done = reached(ItemDtoKind::Ticket, TICKET, Vec::new());
    done.publication = Some(LifecyclePublicationState::Pending);
    done.publication_oid = None;
    done.publication_pending = Some(LifecyclePublicationPendingReason::NoPublicationRemote);
    done.made_ready = Some(vec![LifecycleMadeReadyDto {
        id: OTHER.to_owned(),
        slug: None,
    }]);
    done.made_ready_state = Some(LifecycleMadeReadyState::Current);
    let second = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        answer(
            "ticket close",
            &root,
            ResultCode::Ok,
            Outcome::Success,
            done,
            reached_effects(CleanupEffect::Complete),
            Vec::new(),
            canned,
        ),
    );
    shell.click("lifecycle-retry", cx);
    let again = closes(&shell, cx)[0];
    assert_ne!(again, sent, "Retry sent the close again");
    second.release();
    shell.until_ended(again, cx).await;
    let newer = vec![format!("{} \u{b7} {OTHER_TITLE}", short_id(OTHER))];
    until_rows(&shell, newer.clone(), cx).await;

    let _ = held.send(());
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(
        dialog(cx).read_with(cx, |dialog, _| dialog.made_ready_titles()),
        [(OTHER.to_owned(), Some(OTHER_TITLE.to_owned()))],
        "the stale read changed nothing"
    );
    assert_eq!(shell.rows("lifecycle-made-ready", cx), newer);
    assert!(
        !shell
            .rows("lifecycle-made-ready", cx)
            .iter()
            .any(|row| row.contains(DEPENDENT_TITLE))
    );
}
