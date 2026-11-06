//! D5's privacy sweep (A16) over the surfaces only a remote reaches: a
//! shared conflict reviewed and resolved on the sheet, and a close stopped
//! on its deleted shared branch, stopped and republished from the
//! lifecycle dialog. `privacy_tests.rs` holds the sentinels and the sweep
//! (`Seen`) and drives the real local surfaces.
//!
//! A shared conflict, a deleted shared branch, a republish's preview and
//! their answers come only from a remote, which the SSH harness does not
//! bring to desktop tests: each is canned (decision 9), built from library
//! types, and names the library test that produces the real one. The item
//! is real, with its title, body and comment; a canned answer carries none
//! of them, so these tests show the desktop adds none to what it shows.
//! Nothing is asserted of the repository.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        AuthoringKind, ConflictDto, ConflictNextAction, ConflictNextDto, ConflictPathDto,
        ConflictPathKind, ConflictPathStructure, ConflictReadEligibility, ConflictReadNextAction,
        ConflictStage, ConflictState, ConflictTarget, FailurePoint, ItemDtoKind,
        ItemRepublishPreviewDto, LifecycleMutationDto, LifecycleMutationType, Mutation,
        MutationDataDto, PrepareDataDto, PreviewDetailsDto, RepoRemoveInput, RepositoryService,
        RequestId, SynchronizationConflictMutationDto, SynchronizationConflictType,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, IntegrationEffect, Outcome,
        PublicationEffect, RecoveryAction, RecoveryActionKind, ResultCode, Scope, WriteEffect,
    },
};

use super::{
    privacy_tests::{
        BASE, BODY, Seen, THEIRS, TITLE, commit_sentinel_comment, shows_body_and_comment,
        ticket_with_body,
    },
    tests::{TICKET, closes, open, shell, ticket, until_present},
};
use crate::desktop::{
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{item_id, ticket_path},
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
    },
};

/// The synchronization a canned conflict answer names.
const SYNCHRONIZATION: &str = "01M4CC0VN1KTQFSVR99378TDHV";
/// The close's operation a canned close answer names.
const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5KR1";
const OID: &str = "0123456789012345678901234567890123456789";
/// Fabricated requests stand far above the worker's own numbering.
const FABRICATED: u64 = 5_000_000;

/// The ticket committed on main as "Untitled", with the sentinel comment,
/// then saved with `TITLE` and `BODY`: a working copy of its own. Its tab
/// is open and shows them.
fn ticket_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Untitled", "");
        commit_sentinel_comment(fixture, root, TICKET);
        fixture.refresh(root);
        fixture.save_ticket_with_body(root, TICKET, TITLE, &format!("{BODY} is the body.\n"));
        fixture.refresh(root);
    });
    open(&shell, ticket(&root), cx);
    shows_body_and_comment(&shell, cx);
    (shell, root)
}

/// The request `n` of `intent`, fabricated, queued, started and finished
/// with `envelope`.
fn fabricate(
    shell: &Shell,
    n: u64,
    intent: Intent,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let id = DesktopRequestId(FABRICATED + n);
    let request = DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(request));
        operations.apply(WorkerEvent::Started(id));
        operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
        cx.notify();
    });
    shell.run(cx);
    id
}

/// A share stopped on a conflict of `SYNCHRONIZATION`, as
/// `tests/mutation_remote/merging.rs` `c4_merging_canonical_conflict`
/// answers.
fn share_conflict() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::RecoveryRequired,
        Effects::not_requested(),
        false,
    )
    .with_data(MutationDataDto::SynchronizationConflict(
        SynchronizationConflictMutationDto {
            r#type: SynchronizationConflictType::SynchronizationConflict,
            conflict: ConflictNextDto {
                operation_id: SYNCHRONIZATION.to_owned(),
                next_action: ConflictNextAction::Resolve,
            },
        },
    ))
}

/// The ticket's file in conflict, live and to be resolved, of the shape
/// `tests/mutation_remote/conflict_read.rs` `c4_conflict_read_canonical`
/// reads: the base titled `BASE`, mine `TITLE` with `BODY`, and theirs
/// `THEIRS`.
fn sentinel_conflict() -> ConflictDto {
    ConflictDto {
        operation_id: SYNCHRONIZATION.to_owned(),
        comment_operation_id: None,
        target: ConflictTarget::Primary,
        item_id: None,
        stage: ConflictStage::Primary,
        state: ConflictState::ConflictPending,
        next_action: ConflictReadNextAction::Resolve,
        request_id: None,
        eligibility: Some(ConflictReadEligibility::EligibleCanonical),
        local_parent: "1".repeat(40),
        incoming_parent: "2".repeat(40),
        observation: Some(format!("conflict-v1:{}", "0".repeat(64))),
        historical: false,
        paths: vec![ConflictPathDto {
            display_path: ticket_path(TICKET),
            token: format!("conflict-path-v1:{}", "1".repeat(64)),
            kind: ConflictPathKind::Ticket,
            structure: ConflictPathStructure::Regular,
            content_available: true,
            base: Some(ticket_with_body(TICKET, BASE, "Body.\n")),
            local: Some(ticket_with_body(
                TICKET,
                TITLE,
                &format!("{BODY} is mine.\n"),
            )),
            incoming: Some(ticket_with_body(TICKET, THEIRS, "Body.\n")),
        }],
    }
}

/// Applied and published, as `tests/mutation_remote/resolution.rs`
/// `c4_resolution_publishes` answers (`conflict/answer_tests.rs`
/// `applied`).
fn applied() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "conflict resolve",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Committed,
            integration: IntegrationEffect::Complete,
            publication: PublicationEffect::Published,
            discovery: DiscoveryEffect::Current,
            commit_oid: Some("3".repeat(40)),
            ..Effects::not_requested()
        },
        false,
    )
}

/// A16, canned (decision 9): a share of the ticket stopped on a conflict
/// raises the conflict banner; Review changes opens the sheet on a remote
/// conflict whose three sides hold sentinels; mine is kept and saved, and
/// once applied the banner goes. No sentinel reaches an entry, an answer,
/// the operation area, the banner or the sheet's own words, while the
/// sheet's title and lines show them.
#[gpui_kit::test]
async fn a16_a_shared_conflict_and_its_resolution_show_no_item_text(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let mut seen = Seen::default();
    seen.window(&shell, cx);

    let share = Intent::ItemSync {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    };
    fabricate(&shell, 1, share, share_conflict(), cx);
    until_present(&shell, "item-review-changes", cx).await;
    seen.window(&shell, cx);
    shell.backend.answer_next_conflict_read(sentinel_conflict());
    shell.click("item-review-changes", cx);
    until_present(&shell, "conflict-save", cx).await;
    seen.window(&shell, cx);
    for at in ["conflict-title", "conflict-line"] {
        assert!(seen.allowed_at(at), "a sentinel is drawn in {at}");
    }
    shell.click(("conflict-keep-mine", 0usize), cx);
    seen.window(&shell, cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    shell.click("conflict-save", cx);
    let resolve = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ConflictResolve { .. })
    });
    shell.until_ended(resolve[0], cx).await;
    shell
        .until_drawn(cx, "the banner's clearing", |shell, cx| {
            !shell.present("item-conflict", cx)
        })
        .await;
    seen.window(&shell, cx);

    seen.holds_none(&[
        "operation entry",
        "operation area",
        "banner",
        "sheet message",
        "drawn",
    ]);
    assert!(seen.read_at("banner", "safe"));
}

/// A close's data naming the ticket and nothing of how far it got.
fn short_of_main() -> LifecycleMutationDto {
    LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind: ItemDtoKind::Ticket,
        item_id: TICKET.to_owned(),
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
    }
}

/// A close answered `code` as `outcome`, offering `recovery`, under
/// `request` (`operations/republish_tests.rs` `close_answer`).
fn close_answer(
    root: &Path,
    code: ResultCode,
    outcome: Outcome,
    recovery: Vec<RecoveryAction>,
    request: RequestId,
) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        "ticket close",
        Scope {
            repository: Some(key(root)),
            item_id: Some(TICKET.to_owned()),
            ..Scope::default()
        },
        code == ResultCode::Cancelled,
        code,
        Effects::not_requested(),
        false,
    );
    envelope.outcome = outcome;
    envelope.recovery = recovery;
    envelope.request_id = Some(request.to_string());
    envelope.operation_id = Some(OPERATION.to_owned());
    envelope.data = Some(MutationDataDto::Lifecycle(Box::new(short_of_main())));
    envelope
}

/// A close's child stopped on its deleted shared branch, offering `request
/// cancel` of the close and the repository's details, as
/// `tests/mutation_remote/c5_lifecycle.rs` `child_remote_branch_deleted`
/// answers (its record stays accepted).
fn branch_deleted(root: &Path, request: RequestId) -> Envelope<MutationDataDto> {
    let cancel = RecoveryAction::new(
        RecoveryActionKind::RequestCancel,
        [("request_id", request.to_string().into())],
    );
    let inspect = RecoveryAction::new(
        RecoveryActionKind::RepoInspect,
        [("root", key(root).into())],
    );
    close_answer(
        root,
        ResultCode::RemoteBranchDeleted,
        Outcome::Blocked,
        vec![cancel, inspect],
        request,
    )
}

/// A preview of the ticket's republish, as `tests/mutation_remote/
/// republish.rs` `c4_republish_preview_fetched` gives one: this
/// repository's removal preview, for a confirmation of the right form,
/// with the republish's command and details.
fn republish_preview(shell: &Shell, root: &Path) -> Envelope<PrepareDataDto> {
    let mut prepared = shell
        .fixture
        .service()
        .prepare(&Mutation::RepoRemove(RepoRemoveInput {
            root: root.to_owned(),
        }));
    let Some(PrepareDataDto::Preview(preview)) = prepared.data.as_mut() else {
        panic!("the removal previews");
    };
    preview.command = "item republish".to_owned();
    preview.effects = Vec::new();
    preview.details = PreviewDetailsDto::Republish(ItemRepublishPreviewDto {
        root: key(root),
        kind: ItemDtoKind::Ticket,
        item_id: TICKET.to_owned(),
        branch: "manyhands/ticket/republished".to_owned(),
        remote: "origin".to_owned(),
        last_published_oid: Some(OID.to_owned()),
        local_oid: OID.to_owned(),
    });
    prepared
}

/// A republish answered `ok`, published, as
/// `tests/mutation_remote/republish.rs` `c4_republish_confirmed` ends.
fn republished() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item republish",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects {
            publication: PublicationEffect::Published,
            ..Effects::not_requested()
        },
        false,
    )
}

/// A16, canned (decision 9), as `operations/republish_stop_tests.rs`
/// `stop_and_republish_cancels_republishes_then_resends` drives it: a close
/// of the ticket, through a service that stops it at
/// `ClosureAfterClosingRecorded` so its record stays accepted, is answered
/// as stopped on its deleted shared branch; Stop and republish… gives it up
/// (answered `cancelled`), Republish… is confirmed and answered `ok`
/// (`tests/mutation_remote/c5_cancel.rs` `request_during_failed_child`),
/// and the close is sent again. No sentinel reaches an entry, an answer,
/// the operation area, the dialog's messages and outcome, or the
/// republish's confirmation.
#[gpui_kit::test]
async fn a16_stop_and_republish_shows_no_item_text(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let mut seen = Seen::default();
    let preview = republish_preview(&shell, &root);

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    seen.window(&shell, cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::ClosureAfterClosingRecorded,
        )
        .unwrap(),
    );
    shell
        .backend
        .hold_job_return_answering(
            JobKind::TicketClose,
            branch_deleted(&root, RequestId::new()),
        )
        .release();
    shell.click("lifecycle-confirm", cx);
    let close = closes(&shell, cx)[0];
    shell.until_ended(close, cx).await;
    until_present(&shell, "lifecycle-stop-and-republish", cx).await;
    seen.window(&shell, cx);
    let asked = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(close).cloned())
        .expect("the close is listed")
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
    seen.window(&shell, cx);

    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    shell.click("confirm-ok", cx);
    let republish = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ItemRepublish { .. })
    });
    assert_eq!(republish.len(), 1, "republished once");
    shell.until_ended(republish[0], cx).await;
    shell
        .until(cx, move |operations| {
            operations
                .request(close)
                .is_some_and(|request| request.attempt == 3)
        })
        .await;
    shell.until_ended(close, cx).await;
    shell.run(cx);
    seen.window(&shell, cx);

    seen.holds_none(&[
        "operation entry",
        "operation area",
        "dialog message",
        "confirmation message",
        "drawn",
    ]);
    assert!(seen.read_at("operation entry", "Republish"));
    assert!(seen.allowed_at("lifecycle-subtitle"));
}
