//! R-10 after a restart, then decision 10 (D5 B3's table, Part C; the Task
//! 11b note): a lifecycle child's resolution applied with no lifecycle
//! request in this session resumes the lifecycle's row; that resume names
//! the request that holds it, and the desktop sends that close again under
//! that request ID, with the sheet closed.
//!
//! The close is real: another process stranded it at `closing`, its request
//! accepted, before the window opened. The conflict read and the
//! resolution's answer are canned (decision 9): the read as
//! `tests/mutation_remote/conflict_read.rs` `c4_conflict_read_canonical`
//! gives one, the answer as `tests/mutation_remote/c5_child_conflict.rs`
//! `resolved_then_the_same_request` resolves the child. The resolution's
//! real call is refused, the close's operation holding no conflict.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        CancellationToken, ConfirmationId, FailurePoint, Mutation, MutationCall, NoProgress,
        OperationAction, OperationId, PrepareDataDto, RepositoryService, RequestId,
        TicketCloseInput, keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::{
    SheetSource,
    answer_tests::applied,
    tests::{
        TICKET, canned, canned_path, local_shell, open_sheet, request, services, until_present,
    },
};
use crate::desktop::{
    model::lifecycle_lock::{LockSource, LockState},
    services::{
        JobKind,
        requests::{Intent, RequestState},
    },
    test_support::fixtures::{NoPassphrase, item_id},
    view::{harness::key, lifecycle::OpenedLifecycle},
};

#[gpui_kit::test]
async fn a_child_resolutions_resume_sends_the_holding_close_again(cx: &mut TestAppContext) {
    let holder = RequestId::new();
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Untitled", "");
        fixture.refresh(root);
        fixture.save_ticket(root, TICKET, "Held");
        fixture.refresh(root);
        let failing = RepositoryService::open_at_with_failure_point_for_testing(
            fixture.data_dir(),
            FailurePoint::ClosureAfterClosingRecorded,
        )
        .unwrap();
        let mutation = || {
            Mutation::TicketClose(TicketCloseInput {
                root: root.to_owned(),
                id: item_id(TICKET),
            })
        };
        let prepared = failing.prepare(&mutation());
        let preview = prepared
            .data
            .as_ref()
            .and_then(PrepareDataDto::preview)
            .expect("the close is ready");
        let stopped = failing.execute(
            MutationCall {
                request_id: holder,
                confirmation: Some(ConfirmationId::parse(&preview.confirmation_id).unwrap()),
                mutation: mutation(),
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        );
        assert_ne!(stopped.code, ResultCode::Ok, "{}", stopped.message);
        let service = fixture.service();
        let repo = service.resolve_repository(root).unwrap();
        let row = service
            .list_operations(&repo)
            .unwrap()
            .items
            .into_iter()
            .find(|row| row.action == OperationAction::Close)
            .expect("the close is listed");
        OperationId::parse(row.operation_id.as_deref().unwrap()).unwrap()
    });
    // The rows are read: the lock knows the close's operation.
    let services = services(&shell, cx);
    let repo = key(&root);
    shell
        .until_drawn(cx, "the close's rows", move |_, cx| {
            cx.read(|cx| services.lifecycle_lock(&repo, TICKET, cx))
                == LockState::Locked(LockSource::Record)
        })
        .await;

    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    open_sheet(&shell, SheetSource::Close(None), &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    shell.click("conflict-save", cx);

    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::TicketClose { .. }))
        })
        .await;
    let close = shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketClose { .. })
    })[0];
    let resumed = shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationResume { operation_id, .. }
            if operation_id == operation)
    });
    assert_eq!(resumed.len(), 1, "the row resumed once");
    assert_eq!(request(&shell, close, cx).request_id, holder);
    shell.until_ended(close, cx).await;
    assert!(matches!(
        request(&shell, close, cx).state,
        RequestState::Finished(_, ResultCode::Ok, ..)
    ));
    // M-b: the sheet opens no lifecycle dialog for a resume; the operation
    // area shows the resume and the close it sent.
    assert!(
        cx.update(|cx| cx
            .try_global::<OpenedLifecycle>()
            .and_then(|opened| opened.0.upgrade())
            .is_none()),
        "no lifecycle dialog"
    );
}
