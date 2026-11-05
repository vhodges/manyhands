//! Task 11b's review fixes: one request however often Save is pressed
//! while its set is kept (I-1); a set this session saw answered is not
//! retried by a later sheet (I-2); a lifecycle child's applied resolution
//! sends the lifecycle on after its sheet closed (I-3), or resumes the row
//! when the lifecycle cannot be sent again (M-3).
//!
//! Remote answers and reads are canned as `answer_tests.rs`'s are, citing
//! the same library tests; the local case is real.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::OperationId,
    results::{Effects, ResultCode, WriteEffect},
};

use super::{
    SheetSource,
    answer_tests::{answer, applied, resumed, save_answered},
    tests::{
        TICKET, canned, canned_path, code, local_shell, open_sheet, remote_shell, request,
        resolves, services, until_long, until_present,
    },
};
use crate::desktop::{
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::fixtures::item_id,
};

/// I-1, real local: Save pressed twice before the set is kept sends one
/// request.
#[gpui_kit::test]
async fn pressing_save_twice_sends_one_request(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    let sheet = open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell.visual(cx).update(|window, cx| {
        sheet.update(cx, |sheet, cx| {
            sheet.save(window, cx);
            sheet.save(window, cx);
        })
    });
    until_long(&shell, "the resolution's answer", cx, |shell, cx| {
        resolves(shell, cx)
            .first()
            .is_some_and(|id| code(shell, *id, cx).is_some())
    })
    .await;
    assert_eq!(resolves(&shell, cx).len(), 1, "one request");
}

/// I-2, canned: a Save refused after its path was written keeps its set
/// (R-25; the answer `tests/mutation_remote/resolution.rs`
/// `c4_resolution_after_paths_written` gives, failed with the path
/// written). The sheet is cancelled and opened again on a read that names
/// no attempt (`c4_conflict_read_canonical`): it offers Save resolution,
/// not Retry with the set this session saw refused.
#[gpui_kit::test]
async fn a_refused_set_is_not_retried_by_a_new_sheet(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = save_answered(
        &shell,
        SheetSource::Share,
        &root,
        answer(
            ResultCode::InternalError,
            Effects {
                write: WriteEffect::Written,
                ..Effects::not_requested()
            },
        ),
        cx,
    )
    .await;
    assert!(
        !shell
            .fixture
            .resolution_store()
            .requests()
            .unwrap()
            .is_empty(),
        "R-25 keeps the set"
    );
    until_present(&shell, "conflict-message", cx).await;
    shell.click("conflict-cancel", cx);
    assert!(!shell.present("conflict-sheet", cx));

    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    open_sheet(&shell, SheetSource::Share, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    assert!(!shell.present("conflict-retry", cx), "no Retry of it");
}

/// I-3, canned as `a_lifecycle_child_resolution_sends_the_lifecycle_again`:
/// the sheet is closed while the child's resolution runs; once applied, the
/// lifecycle's request is still sent again.
#[gpui_kit::test]
async fn closing_the_sheet_still_resends_the_lifecycle(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let services = services(&shell, cx);
    let close = services.worker().submit(
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        Lane::User,
        None,
        None,
    );
    shell.until_ended(close, cx).await;
    let operation = OperationId::new();
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    open_sheet(
        &shell,
        SheetSource::Close(Some(close)),
        &root,
        operation,
        cx,
    );
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    let held = shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied());
    shell.click("conflict-save", cx);
    held.wait_reached();
    shell.click("conflict-cancel", cx);
    assert!(!shell.present("conflict-sheet", cx), "the sheet is closed");
    held.release();

    shell
        .until(cx, move |operations| {
            operations
                .request(close)
                .is_some_and(|request| request.attempt == 2)
        })
        .await;
    assert_eq!(request(&shell, close, cx).attempt, 2, "sent again");
    shell.until_ended(close, cx).await;
}

/// M-3, canned as above: a lifecycle request that cannot be sent again
/// (here, one the worker never took) resumes the operation instead of
/// doing nothing.
#[gpui_kit::test]
async fn a_lifecycle_that_cannot_be_sent_again_resumes_its_row(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let unknown = DesktopRequestId(9_999);
    let operation = save_answered(
        &shell,
        SheetSource::Close(Some(unknown)),
        &root,
        applied(),
        cx,
    )
    .await;
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::OperationResume { .. }))
        })
        .await;
    assert_eq!(resumed(&shell, cx), [operation]);
    // M-b: the sheet shows the resume, not an earlier run's dialog.
    assert!(
        cx.update(|cx| cx
            .try_global::<crate::desktop::view::lifecycle::OpenedLifecycle>()
            .and_then(|opened| opened.0.upgrade())
            .is_none()),
        "no lifecycle dialog"
    );
    assert!(!shell.present("conflict-sheet", cx));
}
