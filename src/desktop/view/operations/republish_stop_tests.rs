//! Stop and republish… and Republish… from the lifecycle dialog (D5 A3,
//! Part C; P21, A24; C5 decision 19), over `republish_tests.rs`'s helpers.
//!
//! The deleted branch, the republish's preview and its answers are canned
//! (decision 9), each naming the library test that produces the real one.
//! Where a run's record must be accepted, a real close stops at
//! `ClosureAfterClosingRecorded` through a service put in place of the
//! backend's, and its answer is canned; nothing is asserted of the
//! repository.

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{AuthoringKind, FailurePoint, OperationId, RepositoryService, RequestId},
    results::{Outcome, RecoveryAction, RecoveryActionKind, ResultCode},
};

use super::{
    STOP_AND_REPUBLISH,
    tests::{
        OPERATION, TICKET, branch_deleted, close_answer, finished, is_republish, republish_preview,
        republished, requests_of, ticket_shell, until_present,
    },
};
use crate::desktop::{
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, RequestState},
        worker::GaveUp,
    },
    test_support::fixtures::item_id,
    view::{harness::Shell, item::LIFECYCLE_ACTION, operations::request_id},
};

/// Runs a close from the ticket's dialog through a service that stops it at
/// `ClosureAfterClosingRecorded`, leaving its record accepted, its answer
/// canned as its deleted branch: the dialog offers Stop and republish….
/// Gives the close.
async fn close_on_a_deleted_branch(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(shell, "lifecycle-confirm", cx).await;
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::ClosureAfterClosingRecorded,
        )
        .unwrap(),
    );
    shell
        .backend
        .hold_job_return_answering(JobKind::TicketClose, branch_deleted(root, RequestId::new()))
        .release();
    shell.click("lifecycle-confirm", cx);
    let close = requests_of(shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    })[0]
        .id;
    shell.until_ended(close, cx).await;
    until_present(shell, "lifecycle-stop-and-republish", cx).await;
    assert_eq!(
        shell.label("lifecycle-stop-and-republish", cx).as_deref(),
        Some(STOP_AND_REPUBLISH)
    );
    assert!(!shell.present("lifecycle-retry", cx), "no Retry");
    assert!(!shell.present("lifecycle-republish", cx), "one button");
    close
}

/// P21, A24, A3, canned: Stop and republish… gives the close up: its
/// record is cancelled and the same request sent again, answered
/// `cancelled`, which lets the child's reservation go. Then Republish…
/// opens; confirmed and done, the close is sent again under its own
/// request, its third attempt (`tests/mutation_remote/c5_cancel.rs`
/// `request_during_failed_child`: cancelled, republished `ok`, then the
/// same request finishes).
#[gpui_kit::test]
async fn stop_and_republish_cancels_republishes_then_resends(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    // The preview is canned from the ticket before its close runs.
    let preview = republish_preview(&shell, &root);
    let close = close_on_a_deleted_branch(&shell, &root, cx).await;
    let asked = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    })[0]
        .request_id;

    shell
        .backend
        .hold_job_return_answering(
            JobKind::TicketClose,
            close_answer(
                &root,
                ResultCode::Cancelled,
                Outcome::Cancelled,
                Vec::new(),
                asked,
            ),
        )
        .release();
    shell.backend.answer_next_prepare("item republish", preview);
    shell.click("lifecycle-stop-and-republish", cx);
    until_present(&shell, "confirm-ok", cx).await;
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert_eq!(worker.gave_up(close), Some(GaveUp::Resent));
    assert!(shell.present("lifecycle-stopped-ready", cx));

    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    shell.click("confirm-ok", cx);
    let republish = requests_of(&shell, cx, is_republish);
    assert_eq!(republish.len(), 1);
    assert!(matches!(
        &republish[0].intent,
        Intent::ItemRepublish { root: asked_root, kind: AuthoringKind::Ticket, id }
            if *asked_root == root && id.to_string() == TICKET
    ));
    shell.until_ended(republish[0].id, cx).await;
    shell
        .until(cx, move |operations| {
            operations
                .request(close)
                .is_some_and(|request| request.attempt == 3)
        })
        .await;
    let sent = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    });
    assert_eq!(sent.len(), 1, "the same desktop request");
    assert_eq!(sent[0].request_id, asked, "the same request ID");
    shell.until_ended(close, cx).await;
}

/// P21, canned: the sequence goes on only from a Give up sent on an
/// accepted record and answered `cancelled`. Answered otherwise (the same
/// request refused the same way, as `child_remote_branch_deleted` resends
/// it), or with the record already finished (`GaveUp::AlreadyFinished`, a
/// real close whose answer was canned), nothing opens and nothing is
/// republished: no loop.
#[gpui_kit::test]
async fn stop_and_republish_stops_unless_give_up_resent_and_cancelled(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let close = close_on_a_deleted_branch(&shell, &root, cx).await;
    shell
        .backend
        .hold_job_return_answering(
            JobKind::TicketClose,
            branch_deleted(&root, RequestId::new()),
        )
        .release();
    shell.click("lifecycle-stop-and-republish", cx);
    shell
        .until(cx, move |operations| {
            operations.request(close).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    until_present(&shell, "lifecycle-stop-and-republish", cx).await;
    assert!(!shell.present("confirm-ok", cx), "no Republish…");
    assert!(!shell.present("lifecycle-stopped-ready", cx));
    assert!(requests_of(&shell, cx, is_republish).is_empty());

    // A real close, whose record is finished; its answer canned.
    let (shell, root) = ticket_shell(cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell
        .backend
        .hold_job_return_answering(
            JobKind::TicketClose,
            branch_deleted(&root, RequestId::new()),
        )
        .release();
    shell.click("lifecycle-confirm", cx);
    let close = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    })[0]
        .id;
    shell.until_ended(close, cx).await;
    until_present(&shell, "lifecycle-stop-and-republish", cx).await;
    shell.click("lifecycle-stop-and-republish", cx);
    shell
        .until(cx, move |operations| {
            operations.request(close).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    shell.run(cx);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert_eq!(worker.gave_up(close), Some(GaveUp::AlreadyFinished));
    assert!(!shell.present("confirm-ok", cx), "no Republish…");
    assert!(requests_of(&shell, cx, is_republish).is_empty());
}

/// C5 decision 19, canned: under a resume, L1 routes Republish… then the
/// resume, with nothing to cancel (`src/repository/mutation/way_out_tests.rs`
/// `republish_under_a_resume_comes_before_the_resume`). The dialog offers
/// Republish… and no Stop and republish…; once the republish is done, the
/// operation is resumed.
#[gpui_kit::test]
async fn republish_under_a_resume_then_resumes(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let mut answer = close_answer(
        &root,
        ResultCode::RemoteBranchDeleted,
        Outcome::Blocked,
        vec![RecoveryAction::for_operation(
            RecoveryActionKind::OperationResume,
            OPERATION,
            [],
        )],
        RequestId::new(),
    );
    answer.command = "operation resume".to_owned();
    let close = Intent::TicketClose {
        root: root.clone(),
        id: item_id(TICKET),
    };
    let id = finished(&shell, 1, close, answer, cx);
    shell.click(request_id(id, "open"), cx);
    until_present(&shell, "lifecycle-republish", cx).await;
    assert!(!shell.present("lifecycle-stop-and-republish", cx));
    assert!(!shell.present("lifecycle-retry", cx));

    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    shell.click("lifecycle-republish", cx);
    until_present(&shell, "confirm-ok", cx).await;
    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    shell.click("confirm-ok", cx);
    let republish = requests_of(&shell, cx, is_republish);
    shell.until_ended(republish[0].id, cx).await;
    let operation = OperationId::parse(OPERATION).unwrap();
    shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                matches!(&request.intent, Intent::OperationResume { operation_id, .. }
                    if *operation_id == operation)
            })
        })
        .await;
}
