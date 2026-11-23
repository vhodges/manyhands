//! Task 11b's third review round: an answer that lands before the set's
//! keep returns (I-A), and the lifecycle follow-up's lifetime and record
//! (M-a, M-b). Canned as `answer_tests.rs` is, citing the same library
//! tests; nothing is asserted of the repository.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{OperationId, RequestId},
    results::{Effects, ResultCode, WriteEffect},
};

use super::{
    SheetSource,
    answer_tests::{answer, applied, save_answered},
    tests::{
        TICKET, canned, canned_path, code, open_sheet, remote_shell, request, resolves, services,
        until_long, until_present,
    },
};
use crate::desktop::{
    services::{
        JobKind,
        conflicts::ConflictSource,
        requests::{DesktopRequestId, Intent, Lane, PathResolution},
        resolutions::KeptSet,
    },
    test_support::fixtures::item_id,
    view::harness::Shell,
};

/// A refusal after the path was written, which keeps the set (R-25), as
/// `tests/mutation_remote/resolution.rs` `c4_resolution_after_paths_written`
/// answers.
fn written_failure() -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    answer(
        ResultCode::InternalError,
        Effects {
            write: WriteEffect::Written,
            ..Effects::not_requested()
        },
    )
}

/// A close this session sent and holds: refused for its missing
/// confirmation, which leaves no record.
async fn a_close(
    shell: &Shell,
    root: &std::path::Path,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let close = services(shell, cx).worker().submit(
        Intent::TicketClose {
            root: root.to_owned(),
            id: item_id(TICKET),
        },
        Lane::User,
        None,
        None,
    );
    shell.until_ended(close, cx).await;
    close
}

async fn until_attempt(shell: &Shell, id: DesktopRequestId, attempt: u32, cx: &mut TestAppContext) {
    shell
        .until(cx, move |operations| {
            operations
                .request(id)
                .is_some_and(|request| request.attempt == attempt)
        })
        .await;
}

/// I-A, canned: the resolution's answer lands before the keep's
/// continuation runs. The continuation takes it then, so the answer shows
/// and Save is offered again, never left waiting.
#[gpui_kit::test]
async fn an_answer_before_the_keep_returns_is_still_taken(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    let sheet = open_sheet(&shell, SheetSource::Share, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);

    // The same set, kept and sent and answered before the sheet hears.
    let set = KeptSet {
        request_id: RequestId::new(),
        root: root.clone(),
        operation_id: operation,
        item: item_id(TICKET),
        source: ConflictSource::Remote,
        observation: format!("conflict-v1:{}", "0".repeat(64)),
        resolutions: vec![PathResolution {
            token: format!("conflict-path-v1:{}", "1".repeat(64)),
            source: "x".to_owned(),
        }],
    };
    let request = set.request_id;
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, written_failure())
        .release();
    let services = services(&shell, cx);
    let submitted = cx.update(|cx| services.submit_resolution(set, cx));
    shell.run(cx);
    let id = submitted.await.expect("kept");
    shell.until_ended(id, cx).await;
    shell.visual(cx).update(|window, cx| {
        sheet.update(cx, |sheet, cx| {
            sheet.submitting = true;
            sheet.submitted(request, Ok(id), window, cx);
        })
    });
    shell.run(cx);
    until_present(&shell, "conflict-message", cx).await;
    until_long(&shell, "Save offered again", cx, |_, cx| {
        sheet.read_with(cx, |sheet, _| !sheet.waiting())
    })
    .await;
}

/// M-a, canned: a resolution refused with its set kept keeps its
/// follow-up, so a Retry from the operation area, with the sheet closed,
/// still sends the lifecycle on once applied.
#[gpui_kit::test]
async fn a_retry_from_activity_after_a_refusal_resends_the_lifecycle(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let close = a_close(&shell, &root, cx).await;
    save_answered(
        &shell,
        SheetSource::Close(Some(close)),
        &root,
        written_failure(),
        cx,
    )
    .await;
    let resolution = resolves(&shell, cx)[0];
    assert_eq!(
        code(&shell, resolution, cx),
        Some(ResultCode::InternalError)
    );
    until_present(&shell, "conflict-message", cx).await;
    shell.click("conflict-cancel", cx);
    assert!(!shell.present("conflict-sheet", cx));

    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    assert!(services(&shell, cx).worker().retry(resolution, None));
    until_attempt(&shell, close, 2, cx).await;
    assert_eq!(request(&shell, close, cx).attempt, 2, "sent again");
    shell.until_ended(close, cx).await;
    // Task 16, fix 3: with no sheet open to take it, what the drain did is
    // not recorded.
    let request_id = request(&shell, resolution, cx).request_id;
    assert!(!services(&shell, cx).followups().records(request_id));
}

/// M-a, canned: a resolution removed from the queue before it ran has its
/// set deleted (R-25: the read names no attempt of it, as
/// `c4_conflict_read_canonical` reads), and its follow-up goes with it.
#[gpui_kit::test]
async fn a_removed_resolution_leaves_no_followup(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let services = services(&shell, cx);
    // A close holds the worker, so the resolution waits in the queue.
    let held = shell.backend.hold_job(JobKind::TicketClose);
    let close = services.worker().submit(
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        Lane::User,
        None,
        None,
    );
    held.wait_reached();
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
    shell.click("conflict-save", cx);
    let resolution = resolves(&shell, cx)[0];
    let request_id = request(&shell, resolution, cx).request_id;
    assert!(services.followups().waits_on(request_id));

    // The settle's read of the conflict names no attempt of it.
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    services.worker().cancel(resolution);
    shell.until_ended(resolution, cx).await;
    held.release();
    until_long(&shell, "the follow-up's end", cx, |_, _| {
        !services.followups().waits_on(request_id)
    })
    .await;
    assert!(
        shell
            .fixture
            .resolution_store()
            .kept(request_id)
            .unwrap()
            .is_none()
    );
    shell.until_ended(close, cx).await;
}

/// Task 12 review, canned (the refusal `written_failure` gives): the set's
/// request is named to the sheet before the drain records it in the
/// request model. The sheet still follows it, and shows its answer once it
/// lands.
#[gpui_kit::test]
async fn a_request_not_yet_recorded_stays_followed(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    let sheet = open_sheet(&shell, SheetSource::Share, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    let held = shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, written_failure());
    let services = services(&shell, cx);
    let request_id = RequestId::new();
    shell.visual(cx).update(|window, cx| {
        let id = services.worker().submit_with_request_id(
            Intent::ConflictResolve {
                root: root.clone(),
                operation_id: operation,
                observation: format!("conflict-v1:{}", "0".repeat(64)),
                resolutions: vec![PathResolution {
                    token: format!("conflict-path-v1:{}", "1".repeat(64)),
                    source: "x".to_owned(),
                }],
            },
            Lane::User,
            request_id,
        );
        assert!(
            services.operations().read(cx).request(id).is_none(),
            "the drain has not recorded it yet"
        );
        sheet.update(cx, |sheet, cx| {
            sheet.submitted(request_id, Ok(id), window, cx)
        });
    });
    assert!(sheet.read_with(cx, |sheet, _| sheet.waiting()), "followed");
    held.release();
    until_present(&shell, "conflict-message", cx).await;
}
