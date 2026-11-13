//! What follows a republish, and what waits on one (D5 Part C, A3;
//! orchestrator rulings (a) to (c); re-review 1 to 5): a republish
//! prepared again from its entry, one removed from the queue, one stopped
//! and retried, the whole chain of a republish under a resume (the run,
//! its resume, the request that resume names) read again on reopening, and
//! a share's Republish… while a republish runs.
//!
//! The deleted branch, the republish's previews and its answers are canned
//! (decision 9), each naming the library test that produces the real one.
//! The chain's resume and its resend are real: a close another process
//! stranded at `closing`. Nothing is asserted of the repository in a canned
//! test.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        AuthoringKind, ItemDtoKind, LifecycleMutationDto, LifecycleMutationType, MutationDataDto,
        OperationAction,
    },
    results::{Effects, Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

use super::{
    REPUBLISH, REPUBLISHING,
    stop_tests::{resume_of_operation, resumes, shown, under_a_resume},
    tests::{
        TICKET, branch_deleted, finished, finished_deleted_share, is_republish, republish_preview,
        republished, requests_of, ticket_shell, until_present,
    },
};
use crate::desktop::{
    model::operations::BRANCH_DELETED,
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::fixtures::item_id,
    view::{
        harness::{Shell, key},
        lifecycle::REPUBLISH_STOPPED,
        operations::{recovery_id, request_id, resume::tests as resume},
    },
};

/// A republish answered `code` as `outcome`, offering `recovery`.
fn republish_answer(
    code: ResultCode,
    outcome: Outcome,
    recovery: Vec<RecoveryAction>,
) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        "item republish",
        Scope::default(),
        false,
        code,
        Effects::not_requested(),
        false,
    );
    envelope.outcome = outcome;
    envelope.recovery = recovery;
    envelope
}

/// Opens Republish… from the open dialog and confirms it, its answer
/// canned as `answer`. Gives the republish.
async fn republish_answered(
    shell: &Shell,
    root: &std::path::Path,
    answer: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(shell, root));
    shell.click("lifecycle-republish", cx);
    until_present(shell, "confirm-ok", cx).await;
    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, answer)
        .release();
    shell.click("confirm-ok", cx);
    let republish = requests_of(shell, cx, is_republish)[0].id;
    shell.until_ended(republish, cx).await;
    republish
}

/// Ruling (a), re-review 1, canned: a republish confirmed from a resume's
/// way out is answered `external_change` with `request.prepare`
/// (`tests/mutation_remote/republish.rs` `c4_republish_local_moved`). Its
/// follow-up waits: Prepare again on its entry confirms a new preview under
/// the same request, which is done (`c4_republish_confirmed`), and the
/// lifecycle's operation is resumed.
#[gpui_kit::test]
async fn a_republish_prepared_again_still_sends_its_lifecycle_on(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    under_a_resume(
        &shell,
        &root,
        resume_of_operation(),
        Some(super::tests::OPERATION),
        cx,
    )
    .await;
    let stale = republish_answer(
        ResultCode::ExternalChange,
        Outcome::Blocked,
        vec![RecoveryAction::new(RecoveryActionKind::RequestPrepare, [])],
    );
    let republish = republish_answered(&shell, &root, stale, cx).await;
    until_present(&shell, "confirm-prepare-again", cx).await;
    shell.keys("escape", cx);
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    assert!(resumes(&shell, cx).is_empty());

    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    shell.click(recovery_id(republish, 0), cx);
    until_present(&shell, "confirm-ok", cx).await;
    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    shell.click("confirm-ok", cx);
    shell
        .until(cx, move |operations| {
            operations
                .request(republish)
                .is_some_and(|request| request.attempt == 2)
        })
        .await;
    shell.until_ended(republish, cx).await;
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::OperationResume { .. }))
        })
        .await;
    assert_eq!(resumes(&shell, cx).len(), 1, "the lifecycle is sent on");
}

/// Ruling (a), re-review 2, canned: a republish taken off the queue before
/// it ran will not be done: the dialog offers Republish… again.
#[gpui_kit::test]
async fn a_removed_republish_offers_republish_again(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    under_a_resume(
        &shell,
        &root,
        resume_of_operation(),
        Some(super::tests::OPERATION),
        cx,
    )
    .await;
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    // A job ahead of it keeps the republish queued.
    let ahead = shell.backend.hold_job(JobKind::IndexRefresh);
    worker.submit(
        Intent::IndexRefresh { root: root.clone() },
        Lane::User,
        None,
        None,
    );
    ahead.wait_reached();
    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    shell.click("lifecycle-republish", cx);
    until_present(&shell, "confirm-ok", cx).await;
    shell.click("confirm-ok", cx);
    let republish = requests_of(&shell, cx, is_republish)[0].id;
    shell
        .until_drawn(cx, "no Republish… while it waits", |shell, cx| {
            !shell.present("lifecycle-republish", cx)
        })
        .await;
    worker.cancel(republish);
    shell
        .until(cx, move |operations| {
            matches!(
                operations.request(republish).map(|request| &request.state),
                Some(RequestState::Removed)
            )
        })
        .await;
    shell
        .until_drawn(cx, "Republish… again", |shell, cx| {
            shell.present("lifecycle-republish", cx)
        })
        .await;
    ahead.release();
}

/// P17, P21, canned: a republish stopped with its record accepted
/// (`c4_republish_transport_stop`: `request.retry`) keeps its follow-up.
/// While it waits, not running, the dialog offers no Republish… (I-2's
/// "waits" half). Retried from its entry and done, it sends the lifecycle
/// on.
#[gpui_kit::test]
async fn a_stopped_republish_retried_still_sends_its_lifecycle_on(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    under_a_resume(
        &shell,
        &root,
        resume_of_operation(),
        Some(super::tests::OPERATION),
        cx,
    )
    .await;
    let stopped = republish_answer(
        ResultCode::TransportUnavailable,
        Outcome::Error,
        vec![RecoveryAction::new(RecoveryActionKind::RequestRetry, [])],
    );
    let republish = republish_answered(&shell, &root, stopped, cx).await;
    shell
        .until_drawn(cx, "the confirmation closes", |shell, cx| {
            !shell.present("confirm-ok", cx)
        })
        .await;
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    assert!(
        !services.republish_running(&root, &item_id(TICKET)),
        "not running"
    );
    assert!(
        services.republishing(&root, &item_id(TICKET)),
        "but waiting"
    );
    assert!(services.resends().pending_for(shown(cx).unwrap()));
    assert!(!shell.present("lifecycle-republish", cx), "it waits");
    shell.keys("escape", cx);

    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    shell.click(recovery_id(republish, 0), cx);
    shell
        .until(cx, move |operations| {
            operations
                .request(republish)
                .is_some_and(|request| request.attempt == 2)
        })
        .await;
    shell.until_ended(republish, cx).await;
    shell
        .until(cx, |operations| {
            operations
                .requests()
                .iter()
                .any(|request| matches!(request.intent, Intent::OperationResume { .. }))
        })
        .await;
    assert_eq!(resumes(&shell, cx).len(), 1);
}

/// Ruling (a), I-2's "waits" half, canned as above: the stopped
/// republish's follow-up hides Republish…, and the dialog says the
/// republish stopped; once its own entry is dismissed, the run's Open…
/// offers Republish… again.
#[gpui_kit::test]
async fn a_waiting_republish_hides_republish_until_dismissed(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let run = under_a_resume(
        &shell,
        &root,
        resume_of_operation(),
        Some(super::tests::OPERATION),
        cx,
    )
    .await;
    let stopped = republish_answer(
        ResultCode::TransportUnavailable,
        Outcome::Error,
        vec![RecoveryAction::new(RecoveryActionKind::RequestRetry, [])],
    );
    let republish = republish_answered(&shell, &root, stopped, cx).await;
    shell
        .until_drawn(cx, "it waits", |shell, cx| {
            !shell.present("confirm-ok", cx) && !shell.present("lifecycle-republish", cx)
        })
        .await;
    // Last review, minor 4: the dialog says why.
    assert_eq!(
        shell.label("lifecycle-republish-stopped", cx).as_deref(),
        Some(REPUBLISH_STOPPED)
    );
    shell.keys("escape", cx);
    shell.click(request_id(republish, "dismiss"), cx);
    shell.click(request_id(run, "open"), cx);
    until_present(&shell, "lifecycle-republish", cx).await;
}

/// Decision 10, re-review 3 and 5, real chain: a close stranded at
/// `closing` by another process; a run of it answered, canned, as its
/// deleted branch under a resume (`way_out_tests.rs`
/// `republish_under_a_resume_comes_before_the_resume`), whose Republish…
/// is done (canned, `c4_republish_confirmed`). The drain resumes the
/// close's operation (real), which names the stranded request
/// (`original_request_required`), and sends the close again under it
/// (real). The dialog follows run, resume and close to its end; opened
/// again on the run, it shows the close again, offering no Republish….
#[gpui_kit::test]
async fn the_whole_chain_is_followed_and_read_again_on_reopening(cx: &mut TestAppContext) {
    let (shell, root, holder) = resume::stranded(cx);
    resume::close_row(&shell, cx).await;
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let operation = service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .find(|row| row.action == OperationAction::Close)
        .and_then(|row| row.operation_id)
        .expect("the close is listed");
    let mut answer = Envelope::<MutationDataDto>::classified_mutation(
        "operation resume",
        Scope {
            repository: Some(key(&root)),
            item_id: Some(resume::TICKET.to_owned()),
            ..Scope::default()
        },
        false,
        ResultCode::RemoteBranchDeleted,
        Effects::not_requested(),
        false,
    );
    answer.outcome = Outcome::Blocked;
    answer.recovery = vec![RecoveryAction::for_operation(
        RecoveryActionKind::OperationResume,
        &operation,
        [],
    )];
    answer.operation_id = Some(operation.clone());
    answer.data = Some(MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind: ItemDtoKind::Ticket,
        item_id: resume::TICKET.to_owned(),
        recording: None,
        checkpoint_oid: None,
        integration: None,
        integration_oid: None,
        publication: None,
        publication_oid: None,
        publication_pending: None,
        cleanup: None,
        cleanup_pending: None,
        blockers: Vec::new(),
        made_ready: None,
        made_ready_state: None,
    })));
    let close = Intent::TicketClose {
        root: root.clone(),
        id: item_id(resume::TICKET),
    };
    let run = finished(&shell, 1, close, answer, cx);
    shell.click(request_id(run, "open"), cx);
    until_present(&shell, "lifecycle-republish", cx).await;
    republish_answered(&shell, &root, republished(), cx).await;

    shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                request.request_id == holder && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    let sent = requests_of(&shell, cx, |intent| {
        matches!(intent, Intent::TicketClose { .. })
    })
    .into_iter()
    .find(|request| request.request_id == holder)
    .unwrap()
    .id;
    let resumed = shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationResume { .. })
    });
    assert_eq!(resumed.len(), 1);
    assert!(matches!(
        shell
            .operations(cx)
            .read_with(cx, |operations, _| operations.request(resumed[0]).cloned())
            .unwrap()
            .state,
        RequestState::Finished(_, ResultCode::OriginalRequestRequired, ..)
    ));
    shell
        .until_drawn(cx, "the dialog follows the chain", move |shell, cx| {
            shown(cx) == Some(sent) && shell.present("lifecycle-ending", cx)
        })
        .await;
    assert_eq!(shell.label("lifecycle-ending", cx).as_deref(), Some("Done"));

    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    // Open… on the run, as its entry does (the entry is scrolled out of
    // view below the newer ones).
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let request = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(run).cloned())
        .unwrap();
    shell.visual(cx).update(|window, cx| {
        let origin = cx.focus_handle();
        crate::desktop::view::lifecycle::reopen(
            &request,
            (None, None),
            origin,
            &services,
            window,
            cx,
        )
    });
    shell
        .until_drawn(cx, "the chain read again", move |shell, cx| {
            shown(cx) == Some(sent) && shell.present("lifecycle-ending", cx)
        })
        .await;
    assert!(!shell.present("lifecycle-republish", cx));
}

/// R-30, re-review 4, canned: while a republish of the item runs, its
/// share's entry offers no Republish…, and the entry and the item's pill
/// say it is republishing; once it has ended, Republish… is offered again.
/// The share's answer is `tests/mutation_remote.rs`
/// `a_remotely_deleted_item_branch_is_remote_branch_deleted_and_holds_its_reservation`'s.
#[gpui_kit::test]
async fn a_share_offers_no_republish_while_one_runs(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let id = finished_deleted_share(&shell, &root, cx).await;
    let offer = request_id(id, "republish");
    assert_eq!(shell.label(offer.clone(), cx).as_deref(), Some(REPUBLISH));
    assert_eq!(
        shell.label("item-pill", cx).as_deref(),
        Some(BRANCH_DELETED)
    );

    let held = shell.backend.hold_job(JobKind::ItemRepublish);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let republish = worker.submit(
        Intent::ItemRepublish {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item_id(TICKET),
        },
        Lane::User,
        None,
        None,
    );
    held.wait_reached();
    let hidden = offer.clone();
    shell
        .until_drawn(cx, "no Republish… while it runs", move |shell, cx| {
            !shell.present(hidden.clone(), cx)
        })
        .await;
    assert_eq!(
        shell.label(request_id(id, "outcome"), cx).as_deref(),
        Some(REPUBLISHING)
    );
    assert_eq!(shell.label("item-pill", cx).as_deref(), Some(REPUBLISHING));
    assert_eq!(requests_of(&shell, cx, is_republish).len(), 1);

    held.release();
    shell.until_ended(republish, cx).await;
    shell
        .until_drawn(cx, "Republish… again", move |shell, cx| {
            shell.present(offer.clone(), cx)
        })
        .await;
}

/// R-30, last review 1, canned as above: while a lifecycle's republish of
/// the item waits, stopped with `request.retry`, neither a share of the
/// item nor another run of its lifecycle offers Republish… or Stop and
/// republish…; the other run says the republish stopped.
#[gpui_kit::test]
async fn a_waiting_republish_hides_republish_elsewhere(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    under_a_resume(
        &shell,
        &root,
        resume_of_operation(),
        Some(super::tests::OPERATION),
        cx,
    )
    .await;
    let stopped = republish_answer(
        ResultCode::TransportUnavailable,
        Outcome::Error,
        vec![RecoveryAction::new(RecoveryActionKind::RequestRetry, [])],
    );
    republish_answered(&shell, &root, stopped, cx).await;
    shell
        .until_drawn(cx, "the confirmation closes", |shell, cx| {
            !shell.present("confirm-ok", cx)
        })
        .await;
    shell.keys("escape", cx);

    let share = Intent::ItemSync {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    };
    let deleted = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::RemoteBranchDeleted,
        Effects::not_requested(),
        false,
    );
    let shared = finished(&shell, 2, share, deleted, cx);
    assert!(shell.present(request_id(shared, "dismiss"), cx));
    assert!(!shell.present(request_id(shared, "republish"), cx));
    assert!(!shell.present(request_id(shared, "stop-and-republish"), cx));

    let other = finished(
        &shell,
        3,
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        branch_deleted(&root, manyhands::repository::RequestId::new()),
        cx,
    );
    shell.click(request_id(other, "open"), cx);
    until_present(&shell, "lifecycle-ending", cx).await;
    assert!(!shell.present("lifecycle-stop-and-republish", cx));
    assert!(!shell.present("lifecycle-republish", cx));
    assert_eq!(
        shell.label("lifecycle-republish-stopped", cx).as_deref(),
        Some(REPUBLISH_STOPPED)
    );
}
