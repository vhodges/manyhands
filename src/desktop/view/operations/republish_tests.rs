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
    canonical::CONFIG_PATH,
    repository::{
        AuthoringKind, ConfirmationId, ItemDtoKind, ItemRepublishPreviewDto, LifecycleMutationDto,
        LifecycleMutationType, Mutation, MutationDataDto, PrepareDataDto, PreviewDetailsDto,
        RepoRemoveInput, RequestId,
    },
    results::{
        Effects, Envelope, Outcome, PublicationEffect, RecoveryAction, RecoveryActionKind,
        ResultCode, Scope,
    },
};

use super::{REPUBLISH, STOP_AND_REPUBLISH};
use crate::desktop::{
    model::{
        activity::{
            copy::offer_label,
            rows::{ActivityAction, RowKey},
        },
        navigation::OpenTarget,
        operations::BRANCH_DELETED,
        publication::{SyncOffer, SyncOutcome},
    },
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::{GaveUp, RecordState, WorkerEvent},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity_support::{act, label_of, offered, refresh, row, share_outcome},
        harness::{Shell, key},
    },
};

pub(crate) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5KR0";
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
/// removal preview, which every registered repository gives, for a
/// confirmation of the right form, with the republish's command and
/// details.
pub(crate) fn republish_preview(shell: &Shell, root: &Path) -> Envelope<PrepareDataDto> {
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

/// A republish answered `ok`, published, as `c4_republish_confirmed` ends.
pub(crate) fn republished() -> Envelope<MutationDataDto> {
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
pub(crate) fn close_answer(
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
pub(crate) fn branch_deleted(root: &Path, request: RequestId) -> Envelope<MutationDataDto> {
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

/// Waits until the row `key` offers `label`, enabled.
pub(super) async fn until_offered(
    shell: &Shell,
    key: &RowKey,
    label: &'static str,
    cx: &mut TestAppContext,
) {
    shell
        .until_drawn(cx, label, |shell, cx| {
            refresh(shell, cx);
            offered(shell, key, label, cx).is_some_and(|offer| offer.enabled)
        })
        .await;
}

/// Waits until the row `key`, still there, offers no `label`, enabled.
pub(super) async fn until_not_offered(
    shell: &Shell,
    key: &RowKey,
    label: &'static str,
    cx: &mut TestAppContext,
) {
    shell
        .until_drawn(cx, label, |shell, cx| {
            refresh(shell, cx);
            row(shell, key, cx).is_some() && offered(shell, key, label, cx).is_none()
        })
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

/// A share of the ticket answered `remote_branch_deleted`, canned, whose
/// record is finished: it ran against a repository with no publication
/// remote, so the library holds nothing of it open.
pub(super) async fn finished_deleted_share(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let share = Intent::ItemSync {
        root: root.to_owned(),
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
    shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, deleted)
        .release();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let id = worker.submit(share, Lane::User, None, None);
    shell.until_ended(id, cx).await;
    assert_eq!(worker.record_state(id), Some(RecordState::Finished));
    id
}

/// E-6, canned: an item's share answered `remote_branch_deleted`
/// (`tests/mutation_remote.rs` `a_remotely_deleted_item_branch_is_remote_
/// branch_deleted_and_holds_its_reservation`) says the shared copy was
/// deleted, with no "later release", and, its record finished, offers
/// Republish… and never Share. Republish… opens the confirmation on `item
/// republish` of the item.
#[gpui_kit::test]
async fn a_deleted_branch_offers_republish(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let id = finished_deleted_share(&shell, &root, cx).await;
    assert!(!BRANCH_DELETED.contains("later release"));
    let entry = RowKey::Request(id);
    refresh(&shell, cx);
    assert_eq!(share_outcome(&shell, id, cx), SyncOutcome::BranchDeleted);
    assert!(
        offered(&shell, &entry, REPUBLISH, cx)
            .expect("Republish…")
            .enabled
    );
    assert!(offered(&shell, &entry, STOP_AND_REPUBLISH, cx).is_none());
    assert!(offered(&shell, &entry, offer_label(SyncOffer::Share), cx).is_none());

    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    act(&shell, &entry, REPUBLISH, cx);
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
    // Its entry: the republish of the item, finished.
    assert_eq!(done[0].intent.label(), "Republish");
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
    let entry = RowKey::Request(id);
    let retry = ActivityAction::Recovery(id, 0);
    shell
        .until_drawn(cx, "Retry on the entry", |shell, cx| {
            refresh(shell, cx);
            row(shell, &entry, cx).is_some_and(|row| {
                row.row
                    .menu
                    .iter()
                    .any(|offer| offer.action == retry && offer.enabled)
            })
        })
        .await;
    let label = label_of(&shell, &entry, &retry, cx);
    act(&shell, &entry, &label, cx);
    let again = requests_of(&shell, cx, is_republish);
    assert_eq!(again.len(), 1, "the same desktop request");
    assert_eq!(again[0].attempt, 2);
    assert_eq!(again[0].request_id, request, "the same request ID");
    shell.until_ended(id, cx).await;
}

/// An item's share, run against an SSH remote whose transport the test
/// binary never initializes, so the library accepts it and its record stays
/// accepted, its answer canned as its deleted branch. A second repository,
/// `b`, is registered too. Gives the shell, the repositories, the worker
/// and the share.
async fn accepted_deleted_share(
    cx: &mut TestAppContext,
) -> (
    Shell,
    PathBuf,
    PathBuf,
    crate::desktop::services::worker::WorkerHandle,
    DesktopRequestId,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let other = fixture.create_repository("b");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.publish_over_ssh(&root);
    let config = std::fs::read(root.join(CONFIG_PATH)).unwrap();
    fixture.commit_file(&root, CONFIG_PATH, &config);
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Shared");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
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
    shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, deleted)
        .release();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let id = worker.submit(share, Lane::User, None, None);
    shell.until_ended(id, cx).await;
    assert_eq!(worker.record_state(id), Some(RecordState::Accepted));
    (shell, root, other, worker, id)
}

/// R-30 (orchestrator), review M8: a real share of the item, run against
/// an SSH remote whose transport the test binary never initializes, so no
/// host is contacted: the library accepts it, and its record stays
/// accepted. Its answer is canned as its deleted branch
/// (`tests/mutation_remote.rs` `a_remotely_deleted_item_branch_is_remote_
/// branch_deleted_and_holds_its_reservation`). It offers Stop and
/// republish… beside Give up. Pressed, it gives the share up; the Give up's
/// answer canned `cancelled` (`c4_republish_busy`: the republish waits for
/// the share's reservation, which a cancellation lets go), Republish…
/// opens.
#[gpui_kit::test]
async fn an_item_shares_stop_and_republish_opens_republish(cx: &mut TestAppContext) {
    let (shell, root, _, worker, id) = accepted_deleted_share(cx).await;
    let preview = republish_preview(&shell, &root);
    let entry = RowKey::Request(id);
    until_offered(&shell, &entry, STOP_AND_REPUBLISH, cx).await;
    assert!(offered(&shell, &entry, offer_label(SyncOffer::GiveUp), cx).is_some());
    assert!(offered(&shell, &entry, REPUBLISH, cx).is_none());

    let cancelled = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        true,
        ResultCode::Cancelled,
        Effects::not_requested(),
        false,
    );
    shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, cancelled)
        .release();
    shell.backend.answer_next_prepare("item republish", preview);
    act(&shell, &entry, STOP_AND_REPUBLISH, cx);
    until_present(&shell, "confirm-ok", cx).await;
    assert_eq!(worker.gave_up(id), Some(GaveUp::Resent));
    assert!(
        requests_of(&shell, cx, is_republish).is_empty(),
        "not sent yet"
    );
}

/// The command of each confirmation dialog open, the oldest first.
fn commands(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let command = gpui_kit::ElementId::from("confirm-command");
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|snapshot| snapshot.path().last() == Some(&command))
            .filter_map(|snapshot| snapshot.label().map(str::to_owned))
            .collect()
    })
}

/// R-32: a Stop and republish… from a share's entry, whose Give up answers
/// `cancelled` while another confirmation is submitted and waiting, opens
/// Republish… only once that dialog has closed: stacked over it, the other
/// dialog's answer would close Republish… instead of itself and lose the
/// step. The removal of the second repository stands for that dialog; its
/// job is held, so it is still open when the Give up's answer lands.
#[gpui_kit::test]
async fn republish_waits_for_another_dialog_to_close(cx: &mut TestAppContext) {
    let (shell, root, other, worker, id) = accepted_deleted_share(cx).await;
    let preview = republish_preview(&shell, &root);
    shell.backend.answer_next_prepare("item republish", preview);
    let entry = RowKey::Request(id);
    until_offered(&shell, &entry, STOP_AND_REPUBLISH, cx).await;
    let cancelled = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        true,
        ResultCode::Cancelled,
        Effects::not_requested(),
        false,
    );
    let give_up = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, cancelled);
    act(&shell, &entry, STOP_AND_REPUBLISH, cx);
    give_up.wait_reached();

    // The removal asked for behind the Give up, and left running.
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            let restore = workspace.repositories_list_focus(cx);
            workspace.remove_repository(key(&other), restore, window, cx)
        })
    });
    shell
        .until_drawn(cx, "the removal's preview", |shell, cx| {
            shell.present("confirm-expires", cx)
        })
        .await;
    let removal = shell.backend.hold_job(JobKind::RepoRemove);
    shell.click("confirm-ok", cx);
    give_up.release();
    removal.wait_reached();
    assert_eq!(worker.gave_up(id), Some(GaveUp::Resent));
    shell.run(cx);
    assert_eq!(
        commands(&shell, cx),
        ["Remove repository"],
        "Republish… waits"
    );
    assert!(requests_of(&shell, cx, is_republish).is_empty());

    removal.release();
    shell
        .until_drawn(cx, "Republish…", |shell, cx| {
            commands(shell, cx) == ["Republish"]
        })
        .await;
    assert!(
        requests_of(&shell, cx, is_republish).is_empty(),
        "not sent yet"
    );
}

/// R-30: a share on a deleted branch whose record cannot be read (here the
/// registry is unreadable once the call has returned) offers Give up alone,
/// not Republish…, which would answer `busy` while the share holds its
/// reservation. Pressed, it asks the request to stop and sends it again.
#[gpui_kit::test]
async fn an_unreadable_record_offers_give_up_and_it_sends_the_give_up(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let deleted = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::RemoteBranchDeleted,
        Effects::not_requested(),
        false,
    );
    let returned = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, deleted);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let share = Intent::ItemSync {
        root,
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    };
    let id = worker.submit(share, Lane::User, None, None);
    returned.wait_reached();
    std::fs::write(
        shell
            .fixture
            .data_dir()
            .join(manyhands::repository::REGISTRY_FILE),
        b"not a database",
    )
    .unwrap();
    returned.release();
    shell.until_ended(id, cx).await;
    assert_eq!(worker.record_state(id), Some(RecordState::Unknown));
    let entry = RowKey::Request(id);
    until_offered(&shell, &entry, offer_label(SyncOffer::GiveUp), cx).await;
    assert!(offered(&shell, &entry, REPUBLISH, cx).is_none());
    assert!(offered(&shell, &entry, STOP_AND_REPUBLISH, cx).is_none());

    act(&shell, &entry, offer_label(SyncOffer::GiveUp), cx);
    shell
        .until(cx, move |operations| {
            operations
                .request(id)
                .is_some_and(|request| request.attempt == 2)
        })
        .await;
    shell.until_ended(id, cx).await;
    assert_eq!(worker.gave_up(id), Some(GaveUp::Resent));
}

/// R-32: the dialog Republish… waits for may be dismissed by hand, which is
/// no request event. `Root::close_dialog` (and its deferred form, Escape)
/// notifies the Kit's root, which draws its view, the workspace, again;
/// `open_deferred_republish` runs in that render. So the deferred Republish…
/// opens after Cancel, with nothing else happening, and the cancelled
/// dialog sent nothing.
#[gpui_kit::test]
async fn republish_opens_when_the_dialog_it_waits_for_is_cancelled(cx: &mut TestAppContext) {
    let (shell, root, other, worker, id) = accepted_deleted_share(cx).await;
    let preview = republish_preview(&shell, &root);
    shell.backend.answer_next_prepare("item republish", preview);
    let entry = RowKey::Request(id);
    until_offered(&shell, &entry, STOP_AND_REPUBLISH, cx).await;
    let cancelled = Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        true,
        ResultCode::Cancelled,
        Effects::not_requested(),
        false,
    );
    let give_up = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, cancelled);
    act(&shell, &entry, STOP_AND_REPUBLISH, cx);
    give_up.wait_reached();

    // A removal asked for, its preview shown and never confirmed.
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            let restore = workspace.repositories_list_focus(cx);
            workspace.remove_repository(key(&other), restore, window, cx)
        })
    });
    shell
        .until_drawn(cx, "the removal's preview", |shell, cx| {
            shell.present("confirm-expires", cx)
        })
        .await;
    give_up.release();
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    assert_eq!(worker.gave_up(id), Some(GaveUp::Resent));
    shell.run(cx);
    assert_eq!(
        commands(&shell, cx),
        ["Remove repository"],
        "Republish… waits"
    );

    shell.click("confirm-cancel", cx);
    shell
        .until_drawn(cx, "Republish…", |shell, cx| {
            commands(shell, cx) == ["Republish"]
        })
        .await;
    assert!(
        requests_of(&shell, cx, |intent| matches!(
            intent,
            Intent::RepoRemove { .. }
        ))
        .is_empty()
    );
}
