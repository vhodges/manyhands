//! Republish… (D5 Part C; E-6; P17): the intent, a share's deleted
//! branch, the confirmation and a stopped republish. The lifecycle dialog's
//! Stop and republish… and Republish… are `republish_stop_tests.rs`, over
//! these helpers.
//!
//! A deleted shared branch, a republish's preview and its answers come only
//! from a remote, which the SSH harness does not bring to desktop tests:
//! each is canned (decision 9), built from library types, and names the
//! library test that produces the real one; nothing is asserted of the
//! repository.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        AuthoringKind, ConfirmationId, ItemDtoKind, ItemRepublishPreviewDto, LifecycleMutationDto,
        LifecycleMutationType, Mutation, MutationDataDto, PrepareDataDto, PreviewDetailsDto,
        RequestId, TicketCloseInput,
    },
    results::{
        Effects, Envelope, Outcome, PublicationEffect, RecoveryAction, RecoveryActionKind,
        ResultCode, Scope,
    },
};

use super::REPUBLISH;
use crate::desktop::{
    model::{navigation::OpenTarget, operations::BRANCH_DELETED},
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        operations::request_id,
    },
};

pub(super) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KR0";
pub(super) const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5KR1";
const OID: &str = "0123456789012345678901234567890123456789";
/// Fabricated requests stand far above the worker's own numbering.
const FABRICATED: u64 = 4_000_000;

/// One repository, `a`, with a ticket of its own working copy, its tab
/// open.
pub(super) fn ticket_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Republished");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    (shell, root)
}

fn republish_of(root: &Path) -> Intent {
    Intent::ItemRepublish {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    }
}

/// A canned preview of the ticket's republish, as `tests/mutation_remote/
/// republish.rs` `c4_republish_preview_fetched` gives one: this repository's
/// close preview, for a confirmation of the right form, with the republish's
/// command and details.
pub(super) fn republish_preview(shell: &Shell, root: &Path) -> Envelope<PrepareDataDto> {
    let mut prepared = shell
        .fixture
        .service()
        .prepare(&Mutation::TicketClose(TicketCloseInput {
            root: root.to_owned(),
            id: item_id(TICKET),
        }));
    let Some(PrepareDataDto::Preview(preview)) = prepared.data.as_mut() else {
        panic!("the close is ready");
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

/// A republish answered `ok`, published, as `c4_republish_confirmed` ends.
pub(super) fn republished() -> Envelope<MutationDataDto> {
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
/// `request`.
pub(super) fn close_answer(
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
pub(super) fn branch_deleted(root: &Path, request: RequestId) -> Envelope<MutationDataDto> {
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

pub(super) fn requests_of(
    shell: &Shell,
    cx: &mut TestAppContext,
    pick: impl Fn(&Intent) -> bool,
) -> Vec<DesktopRequest> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(|request| pick(&request.intent))
            .cloned()
            .collect()
    })
}

pub(super) fn is_republish(intent: &Intent) -> bool {
    matches!(intent, Intent::ItemRepublish { .. })
}

pub(super) async fn until_present(shell: &Shell, id: &'static str, cx: &mut TestAppContext) {
    shell
        .until_drawn(cx, id, |shell, cx| shell.present(id, cx))
        .await;
}

/// Puts the request `n` of `intent` in the request model, finished with
/// `envelope`, through `Operations::apply`.
pub(super) fn finished(
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

/// Part C: `item republish` of the item, confirmed, stopping at a share's
/// safe points, and given up as a share is (P21).
#[test]
fn republish_builds_its_mutation() {
    let root = PathBuf::from("/repo");
    let intent = republish_of(&root);
    let Ok(Mutation::ItemRepublish(input)) = intent.build(None, || panic!("no store")) else {
        panic!("a republish builds `item republish`");
    };
    assert_eq!(input.root, root);
    assert_eq!(input.kind, AuthoringKind::Ticket);
    assert_eq!(input.id.to_string(), TICKET);
    assert!(intent.confirmed());
    assert!(intent.stops_at_safe_points());
    assert!(intent.can_be_given_up());
    assert!(intent.changes_items());
    assert_eq!(intent.job_kind(), JobKind::ItemRepublish);
    assert_eq!(intent.label(), "Republish");
    assert_eq!(intent.root(), Some(root.as_path()));
}

/// E-6, canned: an item's share answered `remote_branch_deleted`
/// (`tests/mutation_remote.rs` `a_remotely_deleted_item_branch_is_remote_
/// branch_deleted_and_holds_its_reservation`) says the shared copy was
/// deleted, with no "later release", and, its record unknown here, offers
/// Republish… and never Share. Republish… opens the confirmation on `item
/// republish` of the item.
#[gpui_kit::test]
async fn a_deleted_branch_offers_republish(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let share = Intent::ItemSync {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    };
    let answer = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::RemoteBranchDeleted,
        Effects::not_requested(),
        false,
    );
    let id = finished(&shell, 1, share, answer, cx);
    assert!(!BRANCH_DELETED.contains("later release"));
    assert_eq!(
        shell.label(request_id(id, "outcome"), cx).as_deref(),
        Some(BRANCH_DELETED)
    );
    assert_eq!(
        shell.label(request_id(id, "republish"), cx).as_deref(),
        Some(REPUBLISH)
    );
    assert!(!shell.present(request_id(id, "stop-and-republish"), cx));
    assert!(!shell.present(request_id(id, "share"), cx));

    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    shell.click(request_id(id, "republish"), cx);
    until_present(&shell, "confirm-ok", cx).await;
    assert!(
        shell
            .rows("confirm-detail", cx)
            .iter()
            .any(|row| row.contains("manyhands/ticket/republished")),
        "{:?}",
        shell.rows("confirm-detail", cx)
    );
    assert!(
        requests_of(&shell, cx, is_republish).is_empty(),
        "not sent yet"
    );
}

/// Part C, canned: Republish… confirmed sends `item republish` of the item
/// once, with the preview's confirmation; answered `ok`, published
/// (`c4_republish_confirmed`), its entry says so.
#[gpui_kit::test]
async fn republish_confirms_then_shares(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let preview = republish_preview(&shell, &root);
    let confirmation = preview
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .map(|preview| preview.confirmation_id.clone())
        .unwrap();
    shell.backend.answer_next_prepare("item republish", preview);
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    shell.visual(cx).update(|window, cx| {
        crate::desktop::view::confirm::confirm(
            republish_of(&root),
            cx.focus_handle(),
            &services,
            window,
            cx,
        )
    });
    until_present(&shell, "confirm-ok", cx).await;
    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, republished())
        .release();
    shell.click("confirm-ok", cx);
    let sent = requests_of(&shell, cx, is_republish);
    assert_eq!(sent.len(), 1, "sent once");
    assert_eq!(
        sent[0].confirmation,
        Some(ConfirmationId::parse(&confirmation).unwrap())
    );
    shell.until_ended(sent[0].id, cx).await;
    let done = requests_of(&shell, cx, is_republish);
    assert!(matches!(
        done[0].state,
        RequestState::Finished(Outcome::Success, ResultCode::Ok, ..)
    ));
    assert!(!shell.present("confirm-ok", cx), "the dialog closed");
}

/// P17, canned: a republish stopped with its record left accepted (a
/// transport stop, `c4_republish_transport_stop`: the same request goes on)
/// offers the library's Retry on its entry, which sends it again under the
/// same request ID.
#[gpui_kit::test]
async fn a_stopped_republish_sends_the_same_request(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let request = RequestId::new();
    let mut stopped = Envelope::<MutationDataDto>::classified_mutation(
        "item republish",
        Scope::default(),
        false,
        ResultCode::TransportUnavailable,
        Effects::not_requested(),
        false,
    );
    stopped.request_id = Some(request.to_string());
    stopped.recovery = vec![RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", request.to_string().into())],
    )];
    shell
        .backend
        .hold_job_return_answering(JobKind::ItemRepublish, stopped)
        .release();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let id = worker.submit_with_request_id(republish_of(&root), Lane::User, request);
    shell.until_ended(id, cx).await;
    let retry = crate::desktop::view::operations::recovery_id(id, 0);
    shell
        .until_drawn(cx, "Retry on the entry", |shell, cx| {
            shell.present(retry.clone(), cx)
        })
        .await;
    shell.click(retry.clone(), cx);
    let again = requests_of(&shell, cx, is_republish);
    assert_eq!(again.len(), 1, "the same desktop request");
    assert_eq!(again[0].attempt, 2);
    assert_eq!(again[0].request_id, request, "the same request ID");
    shell.until_ended(id, cx).await;
}
