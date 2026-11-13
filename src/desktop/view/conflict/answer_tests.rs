//! How the resolve conflict sheet follows an answer (D5 design, B3's table;
//! R-10, R-12, R-25; P8, P15, P24): Retry with the request's kept set,
//! never a new request; another window's set used instead, with a notice;
//! a lifecycle child's resolution sending the lifecycle again, or resuming
//! its row after a restart; and Resume of an interrupted resolution.
//!
//! Local cases are real. A remote answer and a remote conflict are canned
//! (decision 9; `hold_job_return_answering`, `answer_next_conflict_read`),
//! each built from library types and naming the library test that produces
//! the real one: the SSH harness does not reach desktop tests. The real
//! call still runs first, against an operation the repository does not
//! hold, and is refused with no effect. Nothing is asserted of the
//! repository in a canned test.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        CancellationToken, ConflictReadNextAction, ConflictState, FailurePoint,
        LocalConflictResolutionInput, LocalContextResolveInput, Mutation, MutationCall,
        MutationDataDto, NoProgress, OperationId, RepositoryService, RequestId,
        keys::SessionCredentials,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, IntegrationEffect, PublicationEffect,
        RecoveryAction, RecoveryActionKind, ResultCode, Scope, WriteEffect,
    },
};

use super::{
    EARLIER_USED, RETRY, SheetSource,
    tests::{
        TICKET, canned, canned_path, code, open_sheet, opened, remote_shell, request, resolves,
        same_body, services, submitted, until_long, until_present,
    },
};
use crate::desktop::{
    services::{
        JobKind,
        conflicts::ConflictSource,
        requests::{Intent, Lane, PathResolution},
        resolutions::KeptSet,
    },
    test_support::fixtures::{Fixture, NoPassphrase, item_id, ticket_source},
    view::{harness::Shell, lifecycle::OpenedLifecycle},
};

/// A canned `conflict resolve` answer with `code` and `effects`, as
/// `Envelope::classified_mutation` gives the library's.
pub(super) fn answer(code: ResultCode, effects: Effects) -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "conflict resolve",
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        effects,
        false,
    )
}

/// A resolution checkpointed, as `bind/conflict.rs` `resolved` reports it.
pub(super) fn checkpointed() -> Effects {
    Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        integration: IntegrationEffect::Complete,
        publication: PublicationEffect::Pending,
        discovery: DiscoveryEffect::Pending,
        commit_oid: Some("3".repeat(40)),
        ..Effects::not_requested()
    }
}

/// Applied and published, as `tests/mutation_remote/resolution.rs`
/// `c4_resolution_publishes` answers, and as the child's resolution in
/// `tests/mutation_remote/c5_child_conflict.rs` `resolved_then_the_same_request`.
pub(crate) fn applied() -> Envelope<MutationDataDto> {
    answer(
        ResultCode::Ok,
        Effects {
            publication: PublicationEffect::Published,
            discovery: DiscoveryEffect::Current,
            ..checkpointed()
        },
    )
}

/// Opens the sheet on a canned canonical conflict of a fresh operation,
/// keeps mine, and presses Save resolution with `canned` standing in place
/// of the answer. Gives the operation.
pub(super) async fn save_answered(
    shell: &Shell,
    source: SheetSource,
    root: &std::path::Path,
    canned_answer: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> OperationId {
    let operation = OperationId::new();
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    open_sheet(shell, source, root, operation, cx);
    until_present(shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, canned_answer)
        .release();
    shell.click("conflict-save", cx);
    let sent = resolves(shell, cx);
    shell.until_ended(sent[0], cx).await;
    operation
}

/// The operations a resume of this session names, oldest last.
pub(super) fn resumed(shell: &Shell, cx: &mut TestAppContext) -> Vec<OperationId> {
    let ids = shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationResume { .. })
    });
    ids.into_iter()
        .map(|id| match request(shell, id, cx).intent {
            Intent::OperationResume { operation_id, .. } => operation_id,
            _ => unreachable!(),
        })
        .collect()
}

/// B3's table, R-12, canned: Save is answered `original_request_required`
/// naming an earlier request R, whose set is kept. The sheet offers Retry,
/// which sends R's set under R, never the sheet's own choices.
///
/// Canned: the answer `tests/mutation_remote/resolution.rs`
/// `c4_resolution_new_request_after_a_written_path_is_edited` produces
/// (Retry of the attempt's own request, nothing written); the conflict as
/// `c4_conflict_read_canonical` reads it.
#[gpui_kit::test]
async fn original_request_required_retries_with_that_requests_kept_set(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    open_sheet(&shell, SheetSource::Share, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-theirs", 0usize), cx);

    let earlier = KeptSet {
        request_id: RequestId::new(),
        root: root.clone(),
        operation_id: operation,
        item: item_id(TICKET),
        source: ConflictSource::Remote,
        observation: format!("conflict-v1:{}", "0".repeat(64)),
        resolutions: vec![PathResolution {
            token: format!("conflict-path-v1:{}", "1".repeat(64)),
            source: same_body(&ticket_source(TICKET, "Earlier", ""), "Earlier"),
        }],
    };
    let held = shell.backend.hold_job_return_answering(
        JobKind::ConflictResolve,
        answer(
            ResultCode::OriginalRequestRequired,
            Effects::not_requested(),
        )
        .with_recovery(vec![RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [("request_id", earlier.request_id.to_string().into())],
        )]),
    );
    shell.click("conflict-save", cx);
    held.wait_reached();
    // The earlier request's set, as a session before this one kept it.
    shell.fixture.resolution_store().keep(&earlier).unwrap();
    held.release();

    until_present(&shell, "conflict-retry", cx).await;
    assert_eq!(shell.label("conflict-retry", cx).as_deref(), Some(RETRY));
    assert!(!shell.present("conflict-save", cx));
    shell.click("conflict-retry", cx);
    let sent = resolves(&shell, cx);
    assert_eq!(sent.len(), 2);
    let retried = request(&shell, sent[0], cx);
    assert_eq!(retried.request_id, earlier.request_id, "the same request");
    let (_, paths) = submitted(&retried.intent);
    assert_eq!(
        paths[0].1,
        same_body(&ticket_source(TICKET, "Earlier", ""), "Earlier"),
        "its set"
    );
    shell.until_ended(sent[0], cx).await;
}

/// P24, R-12, real local: a resolution stopped after writing a path, and
/// its kept set, outlive the session. Reopening the sheet offers Retry
/// with that set, not a new resolution, and the same request finishes it.
#[gpui_kit::test]
async fn reopening_after_a_partial_local_resolution_offers_retry_not_new(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let (inspection, contents) = fixture.read_local_conflict(&root, operation);
    let set = KeptSet {
        request_id: RequestId::new(),
        root: root.clone(),
        operation_id: operation,
        item: item_id(TICKET),
        source: ConflictSource::Local,
        observation: inspection.observation.clone().unwrap(),
        resolutions: vec![PathResolution {
            token: contents[0].token.clone(),
            source: contents[0].local.clone().unwrap(),
        }],
    };
    fixture.resolution_store().keep(&set).unwrap();
    // The session that ended: the library's failure point after the
    // resolution wrote its path (`tests/mutation_replay/local_sync_spec.rs`
    // `partial`).
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        fixture.data_dir(),
        FailurePoint::LocalSyncAfterResolutionPath,
    )
    .unwrap();
    let stopped = failing.execute(
        MutationCall {
            request_id: set.request_id,
            confirmation: None,
            mutation: Mutation::LocalContextResolve(LocalContextResolveInput {
                root: root.clone(),
                operation_id: operation,
                observation: set.observation.clone(),
                resolutions: vec![LocalConflictResolutionInput {
                    token: set.resolutions[0].token.clone(),
                    source: set.resolutions[0].source.clone(),
                }],
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_ne!(stopped.code, ResultCode::Ok);
    drop(failing);
    let shell = Shell::open(fixture, cx);

    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-retry", cx).await;
    assert!(!shell.present("conflict-save", cx), "no new resolution");
    shell.click("conflict-retry", cx);
    let sent = resolves(&shell, cx);
    assert_eq!(sent.len(), 1);
    let retried = request(&shell, sent[0], cx);
    assert_eq!(retried.request_id, set.request_id);
    until_long(&shell, "the retry's answer", cx, |shell, cx| {
        code(shell, sent[0], cx).is_some()
    })
    .await;
    assert_eq!(
        code(&shell, sent[0], cx),
        Some(ResultCode::Ok),
        "P24: it finishes"
    );
}

/// R-12 (Task 11a's note), real local: another window saved a set for the
/// same conflict, still being saved. This sheet's Save resolution sends no
/// new request: it says the earlier one is used instead, and follows it.
#[gpui_kit::test]
async fn another_windows_choices_yield_to_the_kept_set_with_a_notice(cx: &mut TestAppContext) {
    let (shell, root, operation) = super::tests::local_shell(cx, |fixture, root| {
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-theirs", 0usize), cx);

    // The other window keeps mine.
    let (inspection, contents) = shell.fixture.read_local_conflict(&root, operation);
    let other = KeptSet {
        request_id: RequestId::new(),
        root: root.clone(),
        operation_id: operation,
        item: item_id(TICKET),
        source: ConflictSource::Local,
        observation: inspection.observation.unwrap(),
        resolutions: vec![PathResolution {
            token: contents[0].token.clone(),
            source: contents[0].local.clone().unwrap(),
        }],
    };
    let held = shell.backend.hold_job(JobKind::LocalContextResolve);
    let services = services(&shell, cx);
    let submitted_other = cx.update(|cx| services.submit_resolution(other.clone(), cx));
    shell.run(cx);
    let other_id = match submitted_other.await {
        Ok(id) => id,
        Err(error) => panic!("the other window's set is kept: {error}"),
    };
    held.wait_reached();

    shell.click("conflict-save", cx);
    until_present(&shell, "conflict-notice", cx).await;
    assert_eq!(
        shell.label("conflict-notice", cx).as_deref(),
        Some(EARLIER_USED)
    );
    assert_eq!(resolves(&shell, cx), [other_id], "no new request");
    held.release();
    until_long(
        &shell,
        "the earlier resolution's answer",
        cx,
        |shell, cx| code(shell, other_id, cx).is_some(),
    )
    .await;
    assert_eq!(code(&shell, other_id, cx), Some(ResultCode::Ok));
    assert_eq!(resolves(&shell, cx), [other_id]);
    until_long(&shell, "the sheet's closing", cx, |shell, cx| {
        !shell.present("conflict-sheet", cx)
    })
    .await;
}

/// P8, R-10, canned: a lifecycle child's conflict, resolved, sends the
/// lifecycle's own request again, and the lifecycle dialog shows that run.
///
/// Canned: the child's resolution applied, as
/// `tests/mutation_remote/c5_child_conflict.rs`
/// `resolved_then_the_same_request` resolves it; its conflict the shape
/// `c4_conflict_read_canonical` reads. The close this session sent is
/// refused for its missing confirmation, which leaves no record: only its
/// request, and that it is sent again, are asserted.
#[gpui_kit::test]
async fn a_lifecycle_child_resolution_sends_the_lifecycle_again(cx: &mut TestAppContext) {
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
    let first = request(&shell, close, cx);

    save_answered(
        &shell,
        SheetSource::Close(Some(close)),
        &root,
        applied(),
        cx,
    )
    .await;

    shell
        .until(cx, move |operations| {
            operations
                .request(close)
                .is_some_and(|request| request.attempt == 2)
        })
        .await;
    let again = request(&shell, close, cx);
    assert_eq!(again.request_id, first.request_id, "the same request");
    until_long(&shell, "the lifecycle dialog", cx, |_, cx| {
        cx.update(|cx| {
            cx.try_global::<OpenedLifecycle>()
                .and_then(|opened| opened.0.upgrade())
                .is_some()
        })
    })
    .await;
    let dialog = cx
        .update(|cx| cx.global::<OpenedLifecycle>().0.upgrade())
        .unwrap();
    assert_eq!(
        dialog.read_with(cx, |dialog, _| dialog.submitted()),
        Some(close),
        "it shows the run"
    );
    // Review: the sheet's heading reaches the dialog, its title too.
    assert_eq!(
        shell.label("lifecycle-subtitle", cx).as_deref(),
        Some("ae-c10s3 \u{b7} Mine \u{b7} a")
    );
    assert!(opened(cx).is_none(), "the sheet closed");
    shell.until_ended(close, cx).await;
}

/// R-10, U-3, canned: after a restart the session holds no lifecycle
/// request. The child's resolution, read by the lifecycle row's operation
/// (B1), resumes that row.
///
/// Canned: as `a_lifecycle_child_resolution_sends_the_lifecycle_again`.
/// The resume's real call is refused, as the repository holds no such
/// operation.
#[gpui_kit::test]
async fn after_a_restart_a_child_resolution_resumes_the_parent_row(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = save_answered(&shell, SheetSource::Close(None), &root, applied(), cx).await;
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::OperationResume { .. }))
        })
        .await;
    assert_eq!(resumed(&shell, cx), [operation]);
}

/// P15, canned: an interrupted remote resolution is resumed from the
/// sheet. A read whose commit is prepared offers Resume and no Save;
/// so does a Save stopped at its token after the checkpoint.
///
/// Canned: the read `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_commit_prepared` gives (prepared, `resume`, no
/// paths); the answer `tests/mutation_remote/resolution.rs`
/// `c4_resolution_cancelled_before_publication` gives (`cancelled`, the
/// resolution checkpointed). The resumes' real calls are refused.
#[gpui_kit::test]
async fn an_interrupted_remote_resolution_resumes(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let prepared = OperationId::new();
    let mut conflict = canned(prepared, Vec::new());
    conflict.state = ConflictState::ResolutionPrepared;
    conflict.next_action = ConflictReadNextAction::Resume;
    conflict.eligibility = None;
    conflict.observation = None;
    shell.backend.answer_next_conflict_read(conflict);
    open_sheet(&shell, SheetSource::Share, &root, prepared, cx);
    until_present(&shell, "conflict-resume", cx).await;
    assert!(!shell.present("conflict-save", cx));
    shell.click("conflict-resume", cx);
    assert_eq!(resumed(&shell, cx), [prepared]);

    let stopped = save_answered(
        &shell,
        SheetSource::Share,
        &root,
        answer(ResultCode::Cancelled, checkpointed()),
        cx,
    )
    .await;
    until_present(&shell, "conflict-resume", cx).await;
    shell.click("conflict-resume", cx);
    assert_eq!(resumed(&shell, cx), [stopped, prepared]);
}

/// R-12, R-25, canned: a refused resolution whose set stays kept, then
/// dismissed from the operation area, is not another window's set: a new
/// sheet over the same conflict starts afresh, with Save resolution for
/// the choices on its screen, and does not send the old set again.
///
/// Canned: the refusal `invalid_input`, which `remote_answer` keeps; the
/// conflict as `c4_conflict_read_canonical` reads it.
#[gpui_kit::test]
async fn a_dismissed_refusal_is_not_another_windows_set(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = save_answered(
        &shell,
        SheetSource::Share,
        &root,
        answer(ResultCode::InvalidInput, Effects::not_requested()),
        cx,
    )
    .await;
    let refused = resolves(&shell, cx)[0];
    let kept = request(&shell, refused, cx).request_id;
    assert!(
        shell
            .fixture
            .resolution_store()
            .kept(kept)
            .unwrap()
            .is_some(),
        "the refused set stays kept"
    );
    shell
        .visual(cx)
        .update(gpui_kit::component::WindowExt::close_dialog);
    shell.run(cx);
    shell
        .operations(cx)
        .update(cx, |operations, _| operations.dismiss(refused));

    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    open_sheet(&shell, SheetSource::Share, &root, operation, cx);
    until_present(&shell, "conflict-progress", cx).await;
    assert!(!shell.present("conflict-retry", cx), "no old set to retry");
    assert!(shell.present("conflict-save", cx), "a new Save resolution");
}
