//! The lifecycle dialog's outcomes that a local-only repository cannot
//! reach (D5 decision 9): a refused remote deletion (P6) and each way out of
//! L1 with its button or guidance. The credential stops, with D4's
//! recovery, are `credential_tests.rs`, over these helpers. Each answer is canned, built from library types, and names the
//! library test that produces the real one; nothing is asserted of the
//! repository.
//!
//! Most are put in the request model as finished requests through
//! `Operations::apply`, and the dialog is opened on them from the
//! operation area's Open…. Two run a real close, held once its library call
//! returned and answered with the canned envelope
//! (`hold_job_return_answering`).

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleBlocker, LifecycleCleanupPendingReason, LifecycleCleanupState,
        LifecycleIntegrationKind, LifecycleMadeReadyState, LifecycleMutationDto,
        LifecycleMutationType, LifecyclePlannedPublication, LifecyclePublicationPendingReason,
        LifecyclePublicationState, LifecycleRecording, Mutation, MutationDataDto, OperationId,
        PreviewDetailsDto, RequestId, TicketCloseInput,
    },
    results::{
        CheckpointEffect, CleanupEffect, DiscoveryEffect, Effects, Envelope, IntegrationEffect,
        Outcome, PublicationEffect, RecoveryAction, RecoveryActionKind, ResultCode, Scope,
        WriteEffect,
    },
};

use super::{
    outcome::{ENDING_FAILED, OPEN_REPOSITORY_DETAILS, SHARED, STATE_DONE, STATE_FAILED, merged},
    tests::{DOCUMENT, TICKET, closes, request, saved_ticket, shell, ticket, until_present},
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::item_id,
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
        operations::{recovery_id, request_id},
        settings::Shown,
    },
};

const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";
const OID: &str = "0123456789012345678901234567890123456789";
/// Fabricated requests stand far above the worker's own numbering.
pub(super) const FABRICATED: u64 = 3_000_000;

/// A lifecycle's data for the item `id` of `kind` that reached main,
/// merged, published and tidied up, naming `blockers`.
pub(super) fn reached(
    kind: ItemDtoKind,
    id: &str,
    blockers: Vec<LifecycleBlocker>,
) -> LifecycleMutationDto {
    let closes = kind == ItemDtoKind::Ticket;
    LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind,
        item_id: id.to_owned(),
        recording: closes.then_some(LifecycleRecording::Record),
        checkpoint_oid: closes.then(|| OID.to_owned()),
        integration: Some(LifecycleIntegrationKind::Merged),
        integration_oid: Some(OID.to_owned()),
        publication: Some(LifecyclePublicationState::Published),
        publication_oid: Some(OID.to_owned()),
        publication_pending: None,
        cleanup: Some(LifecycleCleanupState::Complete),
        cleanup_pending: None,
        blockers,
        made_ready: closes.then(Vec::new),
        made_ready_state: closes.then_some(LifecycleMadeReadyState::IndexPending),
    }
}

/// Data that names the item and nothing of how far it got, with
/// `blockers`: a stop before main was reached.
pub(super) fn short_of_main(
    kind: ItemDtoKind,
    id: &str,
    blockers: Vec<LifecycleBlocker>,
) -> LifecycleMutationDto {
    LifecycleMutationDto {
        recording: None,
        checkpoint_oid: None,
        integration: None,
        integration_oid: None,
        publication: None,
        publication_oid: None,
        cleanup: None,
        made_ready_state: (kind == ItemDtoKind::Ticket)
            .then_some(LifecycleMadeReadyState::NotComputed),
        ..reached(kind, id, blockers)
    }
}

/// Effects of a run that reached main and published it, its tidy-up as
/// `cleanup` says.
pub(super) fn reached_effects(cleanup: CleanupEffect) -> Effects {
    Effects {
        write: WriteEffect::NotRequested,
        checkpoint: CheckpointEffect::Committed,
        discovery: DiscoveryEffect::Pending,
        publication: PublicationEffect::Published,
        integration: IntegrationEffect::Complete,
        cleanup,
        commit_oid: Some(OID.to_owned()),
    }
}

/// A canned answer of `command` in `root`: `code`, as `outcome`, with
/// `data`, `effects` and `recovery`, naming `request`.
#[allow(clippy::too_many_arguments)]
pub(super) fn answer(
    command: &str,
    root: &Path,
    code: ResultCode,
    outcome: Outcome,
    data: LifecycleMutationDto,
    effects: Effects,
    recovery: Vec<RecoveryAction>,
    request: RequestId,
) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        command,
        Scope {
            repository: Some(key(root)),
            item_id: Some(data.item_id.clone()),
            ..Scope::default()
        },
        false,
        code,
        Effects::not_requested(),
        false,
    );
    envelope.outcome = outcome;
    envelope.effects = effects;
    envelope.recovery = recovery;
    envelope.request_id = Some(request.to_string());
    envelope.operation_id = Some(OPERATION.to_owned());
    envelope.data = Some(MutationDataDto::Lifecycle(Box::new(data)));
    envelope
}

pub(super) fn retry(request: RequestId) -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", request.to_string().into())],
    )
}

fn cancel(request: RequestId) -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestCancel,
        [("request_id", request.to_string().into())],
    )
}

pub(super) fn inspect(root: &Path) -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RepoInspect,
        [("root", key(root).into())],
    )
}

fn resume() -> RecoveryAction {
    RecoveryAction::for_operation(RecoveryActionKind::OperationResume, OPERATION, [])
}

pub(super) fn intent(root: &Path, kind: ItemDtoKind) -> Intent {
    match kind {
        ItemDtoKind::Ticket => Intent::TicketClose {
            root: root.to_owned(),
            id: item_id(TICKET),
        },
        ItemDtoKind::Document => Intent::DocumentPromote {
            root: root.to_owned(),
            id: item_id(DOCUMENT),
        },
    }
}

/// Puts the request `n` of `intent`, under `request_id`, in the request
/// model, finished with `envelope`, through `Operations::apply`.
pub(super) fn finished(
    shell: &Shell,
    n: u64,
    intent: Intent,
    request_id: RequestId,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let id = DesktopRequestId(FABRICATED + n);
    let request = DesktopRequest {
        id,
        request_id,
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

/// Opens the dialog on the finished request `id` from its entry's Open….
pub(super) async fn reopen(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) {
    shell.click(request_id(id, "open"), cx);
    until_present(shell, "lifecycle-ending", cx).await;
}

/// The buttons of an outcome that a way out or a credential stop offers,
/// by their IDs, in this order, as drawn.
pub(super) fn buttons(shell: &Shell, cx: &mut TestAppContext) -> Vec<&'static str> {
    [
        "lifecycle-retry",
        "lifecycle-repair-locally",
        "lifecycle-resolve-conflict",
        "lifecycle-resume",
        "lifecycle-prepare-again",
        "lifecycle-unlock",
        "lifecycle-sharing-and-keys",
        "lifecycle-check-host",
        "lifecycle-inspect",
        "lifecycle-open-folder",
        "lifecycle-stop-and-republish",
        "lifecycle-republish",
    ]
    .into_iter()
    .filter(|id| shell.present(*id, cx))
    .collect()
}

/// L1, A3, canned: each way out of an execute result shows as the dialog's
/// button, beside its guidance where it has some, or as guidance where its
/// button is a later task's. The answers
/// are `src/repository/mutation/way_out_tests.rs`'s, one per route, whose
/// real ones are:
/// - a refused closure checkpoint merge, `RepairLocally` then the same
///   request (`tests/mutation_remote/c5_lifecycle.rs`, the
///   `ticket_close_checkpoint_merge_refused` golden): guidance, Retry,
///   Repair locally… and Open repository details;
/// - a document's child conflict under a resume
///   (`tests/mutation_remote/c5_child_conflict.rs`): guidance, Resolve
///   conflict… and Resume;
/// - a deleted shared branch (`c5_lifecycle.rs` `child_remote_branch_deleted`):
///   guidance, Stop and republish… (Task 14) and no Republish… beside it, and
///   no Retry, which the same request cannot get past;
/// - a document's refused merge (`way_out_tests.rs`
///   `an_execute_result_for_a_documents_integration_conflict_is_repair_outside`):
///   guidance to repair outside, then Retry;
/// - the repository's details alone (`c5_lifecycle.rs` `blocked_at_run`):
///   Open repository details, which opens Settings › Repositories.
#[gpui_kit::test]
async fn each_way_out_offers_its_button(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Ways", ""));
    let request = RequestId::new();
    let partial = |code, kind, blockers, recovery| {
        answer(
            "ticket close",
            &root,
            code,
            Outcome::Partial,
            short_of_main(kind, TICKET, blockers),
            Effects::not_requested(),
            recovery,
            request,
        )
    };
    let (ticket, document) = (ItemDtoKind::Ticket, ItemDtoKind::Document);
    let mut merge_refused = partial(
        ResultCode::RecoveryRequired,
        ticket,
        vec![LifecycleBlocker::IntegrationConflict],
        vec![retry(request), inspect(&root)],
    );
    if let Some(MutationDataDto::Lifecycle(data)) = merge_refused.data.as_mut() {
        data.recording = Some(LifecycleRecording::Record);
    }
    let rows = [
        (
            ticket,
            merge_refused,
            vec![
                "lifecycle-retry",
                "lifecycle-repair-locally",
                "lifecycle-inspect",
            ],
            2,
        ),
        (
            document,
            partial(
                ResultCode::RecoveryRequired,
                document,
                vec![
                    LifecycleBlocker::ContextConflicted,
                    LifecycleBlocker::ConflictPending,
                ],
                vec![resume()],
            ),
            vec!["lifecycle-resolve-conflict", "lifecycle-resume"],
            1,
        ),
        (
            ticket,
            partial(
                ResultCode::RemoteBranchDeleted,
                ticket,
                vec![],
                vec![cancel(request), inspect(&root)],
            ),
            vec!["lifecycle-inspect", "lifecycle-stop-and-republish"],
            3,
        ),
        (
            document,
            partial(
                ResultCode::RecoveryRequired,
                document,
                vec![LifecycleBlocker::IntegrationConflict],
                vec![retry(request), inspect(&root)],
            ),
            vec!["lifecycle-retry", "lifecycle-inspect"],
            2,
        ),
    ];
    for (n, (kind, envelope, expected, guidance)) in rows.into_iter().enumerate() {
        let id = finished(&shell, n as u64, intent(&root, kind), request, envelope, cx);
        reopen(&shell, id, cx).await;
        assert_eq!(buttons(&shell, cx), expected, "row {n}");
        assert_eq!(
            shell.rows("lifecycle-way-out", cx).len(),
            guidance,
            "row {n}: {:?}",
            shell.rows("lifecycle-way-out", cx)
        );
        shell.keys("escape", cx);
    }

    // Resume resumes the operation the answer names.
    let id = finished(
        &shell,
        10,
        intent(&root, document),
        request,
        partial(
            ResultCode::RecoveryRequired,
            document,
            vec![
                LifecycleBlocker::ContextConflicted,
                LifecycleBlocker::ConflictPending,
            ],
            vec![resume()],
        ),
        cx,
    );
    reopen(&shell, id, cx).await;
    shell.click("lifecycle-resume", cx);
    let resumed = shell.requests(cx, |request| {
        matches!(&request.intent, Intent::OperationResume { operation_id, .. }
            if *operation_id == OperationId::parse(OPERATION).unwrap())
    });
    assert_eq!(resumed.len(), 1, "Resume sent its operation");
    shell.until_ended(resumed[0], cx).await;
    shell.keys("escape", cx);

    // Open repository details opens the repository's details.
    let id = finished(
        &shell,
        11,
        intent(&root, ticket),
        request,
        answer(
            "ticket close",
            &root,
            ResultCode::RecoveryRequired,
            Outcome::Blocked,
            short_of_main(ticket, TICKET, vec![LifecycleBlocker::PrimaryDirty]),
            Effects::not_requested(),
            vec![inspect(&root)],
            request,
        ),
        cx,
    );
    reopen(&shell, id, cx).await;
    assert_eq!(
        shell.label("lifecycle-inspect", cx).as_deref(),
        Some(OPEN_REPOSITORY_DETAILS)
    );
    shell.click("lifecycle-inspect", cx);
    shell.run(cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert!(matches!(shown, Shown::Settings(SettingsPage::Repositories)));
}

/// P6, canned (decision 9): a remote that refuses the shared branch's
/// deletion cannot be reached here. The answer is the library's
/// (`tests/mutation_remote/c5_lifecycle.rs` `deletion_rejected`, the
/// `ticket_close_deletion_rejected` golden: `push_rejected`, partial, merged,
/// published, cleanup pending `remote_deletion_rejected`, `request.retry`).
/// The preview is this repository's, with a remote as
/// `a_remote_preview_shares_with_origin` cans it. The dialog says Failed,
/// with every step checked but the tidy-up, which failed; merged and shared
/// are reported apart from the tidy-up, and Enter retries the same request.
#[gpui_kit::test]
async fn cleanup_refused_is_failed_with_retry(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Refused", "")
    });
    let mut prepared = shell
        .fixture
        .service()
        .prepare(&Mutation::TicketClose(TicketCloseInput {
            root: root.clone(),
            id: item_id(TICKET),
        }));
    let Some(manyhands::repository::PrepareDataDto::Preview(preview)) = prepared.data.as_mut()
    else {
        panic!("the close is ready");
    };
    let PreviewDetailsDto::Lifecycle(details) = &mut preview.details else {
        panic!("a lifecycle preview");
    };
    details.remote = Some("origin".to_owned());
    details.publication = LifecyclePlannedPublication::Publish;
    details.cleanup.remote_branch = true;
    shell.backend.answer_next_prepare("ticket close", prepared);
    super::tests::open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;

    let mut data = reached(ItemDtoKind::Ticket, TICKET, vec![]);
    data.cleanup = Some(LifecycleCleanupState::Pending);
    data.cleanup_pending = Some(LifecycleCleanupPendingReason::RemoteDeletionRejected);
    // The canned answer names a request of its own: the dialog retries the
    // request it sent, whatever the answer names.
    let canned = RequestId::new();
    let gate = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        answer(
            "ticket close",
            &root,
            ResultCode::PushRejected,
            Outcome::Partial,
            data,
            reached_effects(CleanupEffect::Pending),
            vec![retry(canned)],
            canned,
        ),
    );
    shell.click("lifecycle-confirm", cx);
    let first = closes(&shell, cx)[0];
    let asked = request(&shell, first, cx).request_id;
    gate.release();
    shell.until_ended(first, cx).await;
    until_present(&shell, "lifecycle-ending", cx).await;

    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_FAILED)
    );
    assert_eq!(
        shell.rows("lifecycle-step", cx),
        [
            "Save your latest edits",
            "Share with origin",
            "Mark as closed",
            "Merge into main",
            "Share main with origin",
            "Tidy up",
        ]
    );
    assert_eq!(
        shell.rows("lifecycle-step-state", cx),
        [
            STATE_DONE,
            STATE_DONE,
            STATE_DONE,
            STATE_DONE,
            STATE_DONE,
            STATE_FAILED
        ]
    );
    assert_eq!(
        shell.rows("lifecycle-outcome", cx),
        [
            merged("main"),
            SHARED.to_owned(),
            "Not tidied up yet: the remote refused to delete the shared branch".to_owned(),
        ]
    );
    assert!(shell.present("lifecycle-retry", cx));

    shell.keys("enter", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2, "Enter retried");
    assert_eq!(
        request(&shell, sent[0], cx).request_id,
        asked,
        "the same request"
    );
    shell.until_ended(sent[0], cx).await;
}

/// A3, R-7, canned (the library's: `bind/lifecycle.rs` `reached_code`, a
/// push to origin refused because another process holds the repository:
/// `busy`, partial, merged, publication pending `busy`, `request.retry`).
/// `busy` before main is reached is still running (P22); once the merge
/// happened the answer is a pending remote publication, so Failed: merged,
/// not shared, with Retry. Without a merge, `busy` is still running.
#[gpui_kit::test]
async fn busy_after_the_merge_is_failed_not_still_running(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Busy", ""));
    let ticket = ItemDtoKind::Ticket;
    let request = RequestId::new();
    let mut after_merge = reached(ticket, TICKET, vec![]);
    after_merge.publication = Some(LifecyclePublicationState::Pending);
    after_merge.publication_oid = None;
    after_merge.publication_pending = Some(LifecyclePublicationPendingReason::Busy);
    after_merge.cleanup = Some(LifecycleCleanupState::Pending);
    after_merge.cleanup_pending = Some(LifecycleCleanupPendingReason::AwaitingPublication);
    let mut effects = reached_effects(CleanupEffect::Pending);
    effects.publication = PublicationEffect::Pending;
    let id = finished(
        &shell,
        1,
        intent(&root, ticket),
        request,
        answer(
            "ticket close",
            &root,
            ResultCode::Busy,
            Outcome::Partial,
            after_merge,
            effects,
            vec![retry(request)],
            request,
        ),
        cx,
    );
    reopen(&shell, id, cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_FAILED)
    );
    assert!(!shell.present("lifecycle-still-running", cx));
    assert_eq!(
        shell.rows("lifecycle-outcome", cx),
        [
            merged("main"),
            "Not shared: another operation holds the repository".to_owned(),
            "Not tidied up yet: main is not shared yet".to_owned(),
        ]
    );
    assert!(shell.present("lifecycle-retry", cx));
}

/// R-1, V8, canned (`identity_required` of a lifecycle run carries only
/// `repo.identity_set`): the operation area's entry of a close leaves Retry
/// and Prepare again to its dialog, and keeps drawing what the dialog does
/// not own, Set identity.
#[gpui_kit::test]
async fn a_lifecycle_entry_keeps_the_recovery_its_dialog_does_not_own(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Who", ""));
    let ticket = ItemDtoKind::Ticket;
    let request = RequestId::new();
    let identity = RecoveryAction::new(
        RecoveryActionKind::RepoIdentitySet,
        [("root", key(&root).into())],
    );
    let prepare = RecoveryAction::new(RecoveryActionKind::RequestPrepare, []);
    let id = finished(
        &shell,
        1,
        intent(&root, ticket),
        request,
        answer(
            "ticket close",
            &root,
            ResultCode::IdentityRequired,
            Outcome::Blocked,
            short_of_main(ticket, TICKET, vec![]),
            Effects::not_requested(),
            vec![identity, retry(request), prepare],
            request,
        ),
        cx,
    );
    assert!(shell.present(request_id(id, "open"), cx));
    assert!(shell.present(recovery_id(id, 0), cx), "Set identity");
    assert!(!shell.present(recovery_id(id, 1), cx), "no raw Retry");
    assert!(
        !shell.present(recovery_id(id, 2), cx),
        "no raw Prepare again"
    );
}
