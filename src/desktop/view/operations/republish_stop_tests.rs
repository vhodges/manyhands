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
    model::activity::rows::RowKey,
    services::{
        BUSY_RETRY_DELAYS, JobKind,
        requests::{DesktopRequestId, Intent, RequestState},
        worker::GaveUp,
    },
    test_support::fixtures::item_id,
    view::{
        activity_support::act,
        harness::Shell,
        item::LIFECYCLE_ACTION,
        lifecycle::{NOT_STOPPED, OpenedLifecycle},
        operations::{OPEN_LIFECYCLE, resume::tests::until_long},
    },
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

    // Stop is pressed on the Give up's attempt before it is answered: the
    // dialog says nothing of that Stop once the close is sent again.
    let give_up = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        close_answer(
            &root,
            ResultCode::Cancelled,
            Outcome::Cancelled,
            Vec::new(),
            asked,
        ),
    );
    shell.backend.answer_next_prepare("item republish", preview);
    shell.click("lifecycle-stop-and-republish", cx);
    give_up.wait_reached();
    shell
        .until_drawn(cx, "Stop on the Give up", |shell, cx| {
            shell.present("lifecycle-stop", cx)
        })
        .await;
    shell.click("lifecycle-stop", cx);
    assert!(shell.present("lifecycle-stopping", cx));
    give_up.release();
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
    let running = shell.backend.hold_job(JobKind::ItemRepublish);
    // The close's third attempt is held, to see it run.
    let third = shell.backend.hold_job(JobKind::TicketClose);
    shell.click("confirm-ok", cx);
    running.wait_reached();
    // The re-review's UX: the stopped run now points at the republish.
    shell
        .until_drawn(cx, "Republishing…", |shell, cx| {
            shell.present("lifecycle-republishing", cx)
        })
        .await;
    assert!(!shell.present("lifecycle-stopped-ready", cx));
    running.release();
    let republish = requests_of(&shell, cx, is_republish);
    assert_eq!(republish.len(), 1);
    // Review M5: once the confirmation closes, the lifecycle dialog has
    // focus again.
    shell
        .until_drawn(cx, "the confirmation closes", |shell, cx| {
            !shell.present("confirm-ok", cx)
        })
        .await;
    let dialog = cx
        .update(|cx| {
            cx.try_global::<OpenedLifecycle>()
                .and_then(|opened| opened.0.upgrade())
        })
        .expect("the lifecycle dialog is open");
    let focus = dialog.read_with(cx, |dialog, _| dialog.focus().clone());
    assert!(shell.frame(cx, |window, _| focus.is_focused(window)));
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
    // The third attempt is a new run: it has its own Stop, and the Stop
    // pressed on the Give up's attempt is not carried over to it.
    third.wait_reached();
    shell
        .until_drawn(cx, "Stop on the third attempt", |shell, cx| {
            shell.present("lifecycle-stop", cx) || shell.present("lifecycle-stopping", cx)
        })
        .await;
    assert!(
        !shell.present("lifecycle-stopping", cx),
        "Stopping\u{2026} with no Stop button"
    );
    third.release();
    let sent = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    });
    assert_eq!(sent.len(), 1, "the same desktop request");
    assert_eq!(sent[0].request_id, asked, "the same request ID");
    shell.until_ended(close, cx).await;
    shell.run(cx);
    assert!(
        !shell.present("lifecycle-stop-late", cx),
        "no stale 'finished before it could stop'"
    );
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

    // Task 16, P21: the Give up's resend answered `busy`, offering the same
    // request again, as `tests/mutation_remote/c5_lifecycle.rs`
    // `c5_lifecycle_busy_lease_on_reentry_close` answers a re-entry while
    // the repository's lease is held. The sequence stops there: one
    // request, sent once by the Give up and never again by itself.
    let (shell, root) = ticket_shell(cx);
    let close = close_on_a_deleted_branch(&shell, &root, cx).await;
    let asked = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    })[0]
        .request_id;
    // The Give up's re-send is answered `busy` on every take: the first
    // and each of the worker's retries, so the real call never runs and
    // the request ends with `busy` once the delays are used up.
    let busy = close_answer(
        &root,
        ResultCode::Busy,
        Outcome::Blocked,
        vec![RecoveryAction::new(RecoveryActionKind::RequestRetry, [])],
        asked,
    );
    for _ in 0..=BUSY_RETRY_DELAYS.len() {
        shell
            .backend
            .hold_job_return_answering(JobKind::TicketClose, busy.clone())
            .release();
    }
    shell.click("lifecycle-stop-and-republish", cx);
    // The worker's retry delays are real time (`BUSY_RETRY_DELAYS`, 0.7 s
    // in all) and have no seam to inject, so this waits with the shared
    // `until_long`, as `resume_tests` does for its busy answers.
    until_long(&shell, "the busy resend's end", cx, |shell, cx| {
        shell.operations(cx).read_with(cx, |operations, _| {
            operations.request(close).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
    })
    .await;
    let sent = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    });
    assert_eq!(sent.len(), 1, "one request");
    assert_eq!(sent[0].attempt, 2, "no resend after busy");
    assert!(matches!(sent[0].state, RequestState::Finished(..)));
    assert!(!shell.present("confirm-ok", cx), "no Republish…");
    assert!(!shell.present("lifecycle-stopped-ready", cx));
    assert!(requests_of(&shell, cx, is_republish).is_empty());
}

/// A close answered `remote_branch_deleted` under a resume, put in the
/// request model and opened from its entry's Open…: its way out offers
/// `resume` (`recovery`), with `operation` as the answer's own.
pub(super) async fn under_a_resume(
    shell: &Shell,
    root: &Path,
    recovery: RecoveryAction,
    operation: Option<&str>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let mut answer = close_answer(
        root,
        ResultCode::RemoteBranchDeleted,
        Outcome::Blocked,
        vec![recovery],
        RequestId::new(),
    );
    answer.command = "operation resume".to_owned();
    answer.operation_id = operation.map(str::to_owned);
    let close = Intent::TicketClose {
        root: root.to_owned(),
        id: item_id(TICKET),
    };
    let id = finished(shell, 1, close, answer, cx);
    act(shell, &RowKey::Request(id), OPEN_LIFECYCLE, cx);
    until_present(shell, "lifecycle-republish", cx).await;
    assert!(!shell.present("lifecycle-stop-and-republish", cx));
    assert!(!shell.present("lifecycle-retry", cx));
    id
}

pub(super) fn resume_of_operation() -> RecoveryAction {
    RecoveryAction::for_operation(RecoveryActionKind::OperationResume, OPERATION, [])
}

pub(super) fn resumes(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    let operation = OperationId::parse(OPERATION).unwrap();
    requests_of(shell, cx, move |intent| {
        matches!(intent, Intent::OperationResume { operation_id, .. } if *operation_id == operation)
    })
    .into_iter()
    .map(|request| request.id)
    .collect()
}

/// The run the open lifecycle dialog shows.
pub(super) fn shown(cx: &mut TestAppContext) -> Option<DesktopRequestId> {
    cx.update(|cx| {
        cx.try_global::<OpenedLifecycle>()
            .and_then(|opened| opened.0.upgrade())
    })
    .and_then(|dialog| dialog.read_with(cx, |dialog, _| dialog.submitted()))
}

/// C5 decision 19, review I-1, canned: under a resume, L1 routes
/// Republish… then the resume, with nothing to cancel
/// (`src/repository/mutation/way_out_tests.rs`
/// `republish_under_a_resume_comes_before_the_resume`). The dialog offers
/// Republish… and no Stop and republish…; once the republish is done, the
/// operation is resumed, and the dialog follows that resume: running, then
/// its end (its real call is refused, the repository holding no such
/// operation).
#[gpui_kit::test]
async fn republish_under_a_resume_then_resumes(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let id = under_a_resume(&shell, &root, resume_of_operation(), Some(OPERATION), cx).await;
    assert_eq!(shown(cx), Some(id));

    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    shell.click("lifecycle-republish", cx);
    until_present(&shell, "confirm-ok", cx).await;
    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    let held = shell.backend.hold_job(JobKind::OperationResume);
    shell.click("confirm-ok", cx);
    let republish = requests_of(&shell, cx, is_republish);
    shell.until_ended(republish[0].id, cx).await;
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::OperationResume { .. }))
        })
        .await;
    let resume = resumes(&shell, cx);
    assert_eq!(resume.len(), 1);
    held.wait_reached();
    shell
        .until_drawn(cx, "the dialog follows the resume", |shell, cx| {
            shown(cx) == Some(resume[0]) && shell.present("lifecycle-stop", cx)
        })
        .await;
    assert!(!shell.present("lifecycle-republish", cx));

    held.release();
    shell.until_ended(resume[0], cx).await;
    shell
        .until_drawn(cx, "the resume's end", |shell, cx| {
            shell.present("lifecycle-code", cx)
        })
        .await;
    assert_eq!(shown(cx), Some(resume[0]));
    assert_eq!(
        shell.label("lifecycle-code", cx).as_deref(),
        Some("operation_not_found")
    );
}

/// Review I-2, canned as above: while the republish runs, the dialog offers
/// no second Republish…; one republish is sent, and one resume after it.
#[gpui_kit::test]
async fn a_republish_in_flight_offers_no_second_republish(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    under_a_resume(&shell, &root, resume_of_operation(), Some(OPERATION), cx).await;
    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    shell.click("lifecycle-republish", cx);
    until_present(&shell, "confirm-ok", cx).await;
    let held = shell.backend.hold_job(JobKind::ItemRepublish);
    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell
        .until_drawn(cx, "no Republish… while it runs", |shell, cx| {
            !shell.present("lifecycle-republish", cx)
        })
        .await;
    held.release();
    let republish = requests_of(&shell, cx, is_republish);
    assert_eq!(republish.len(), 1, "one republish");
    shell.until_ended(republish[0].id, cx).await;
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::OperationResume { .. }))
        })
        .await;
    let resume = resumes(&shell, cx);
    shell.until_ended(resume[0], cx).await;
    shell.run(cx);
    assert_eq!(requests_of(&shell, cx, is_republish).len(), 1);
    assert_eq!(resumes(&shell, cx).len(), 1, "one resume after it");
}

/// Review M2, canned: a resume's way out that names no operation routes
/// Republish… alone (`way_out.rs` `resume`, `Some(None)`). Republish…
/// opens and republishes, and nothing is sent after it.
#[gpui_kit::test]
async fn a_republish_with_nothing_after_sends_nothing_after(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let nameless = RecoveryAction::new(RecoveryActionKind::OperationResume, []);
    under_a_resume(&shell, &root, nameless, None, cx).await;
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
    assert_eq!(republish.len(), 1);
    shell.until_ended(republish[0].id, cx).await;
    shell.run(cx);
    assert!(
        requests_of(&shell, cx, |intent| matches!(
            intent,
            Intent::OperationResume { .. }
        ))
        .is_empty(),
        "nothing after it"
    );
}

/// A3, review M8, canned: Stop and republish… on a run this session's worker
/// never took cannot give it up: the dialog says so, and nothing opens.
#[gpui_kit::test]
async fn a_stop_that_cannot_be_queued_says_so(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let close = Intent::TicketClose {
        root: root.clone(),
        id: item_id(TICKET),
    };
    let id = finished(
        &shell,
        1,
        close,
        branch_deleted(&root, RequestId::new()),
        cx,
    );
    act(&shell, &RowKey::Request(id), OPEN_LIFECYCLE, cx);
    until_present(&shell, "lifecycle-stop-and-republish", cx).await;
    shell.click("lifecycle-stop-and-republish", cx);
    until_present(&shell, "lifecycle-stop-refused", cx).await;
    assert_eq!(
        shell.label("lifecycle-stop-refused", cx).as_deref(),
        Some(NOT_STOPPED)
    );
    assert!(!shell.present("confirm-ok", cx));
    assert!(requests_of(&shell, cx, is_republish).is_empty());
}

/// Review M5, canned: a Stop and republish… whose dialog is closed before
/// its Give up answers opens no Republish… later, out of context.
#[gpui_kit::test]
async fn a_closed_dialog_ends_its_stop_and_republish(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let preview = republish_preview(&shell, &root);
    let close = close_on_a_deleted_branch(&shell, &root, cx).await;
    let asked = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    })[0]
        .request_id;
    let held = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        close_answer(
            &root,
            ResultCode::Cancelled,
            Outcome::Cancelled,
            Vec::new(),
            asked,
        ),
    );
    shell.backend.answer_next_prepare("item republish", preview);
    shell.click("lifecycle-stop-and-republish", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    held.release();
    shell
        .until(cx, move |operations| {
            operations.request(close).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    shell.run(cx);
    shell.run(cx);
    assert!(
        !shell.present("confirm-ok", cx),
        "no Republish… out of context"
    );
    assert!(requests_of(&shell, cx, is_republish).is_empty());
}
