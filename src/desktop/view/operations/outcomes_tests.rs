//! A finished share's outcome in its operation area entry (design, Lane
//! B, Outcomes and routes): one test per `SyncOutcome` row, from
//! constructed envelopes. The routes, Unlock… and Busy are
//! `outcomes_route_tests`'.
//!
//! Requests put in the request model through `Operations::apply` have no
//! record the worker kept: their record is unknown. Rows whose controls
//! depend on the record run a real share instead, held once its library
//! call returned and answered with a constructed envelope: with its
//! publication selection committed its record stays accepted, and with
//! the selection left uncommitted the library refuses it before
//! acceptance, so it stands finished.

use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::CONFIG_PATH,
    repository::{
        AuthoringKind, ConflictNextAction, ConflictNextDto, MutationDataDto, RequestId,
        SynchronizationConflictMutationDto, SynchronizationConflictType, transport::SshAuthority,
    },
    results::{
        Effects, Envelope, PublicationEffect, RecoveryAction, RecoveryActionKind, ResultCode, Scope,
    },
};

use super::{offer_label, offer_part};
use crate::desktop::{
    model::{
        operations::{
            ANOTHER_SHARE_HOLDS, BRANCH_DELETED, CHOOSE_A_KEY, CURRENT, KEY_LOCKED,
            NEEDS_ATTENTION, NEEDS_IDENTITY, NOWHERE_TO_SHARE, SHARED, STOPPED_BEFORE_SENDING,
            check_host_text,
        },
        publication::{NOTHING_TO_SHARE, SyncOffer},
        stamp::SettingsPage,
    },
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{Fixture, item_id},
    view::{harness::Shell, operations::request_id, settings::Shown},
};

pub(super) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";

/// The line a settings page names the repository it works on with.
pub(super) fn chosen_line(root: &str) -> String {
    format!("Repository: {root}")
}

/// Fabricated requests stand far above the worker's own numbering.
const FABRICATED: u64 = 2_000_000;

const OFFERS: [SyncOffer; 9] = [
    SyncOffer::Share,
    SyncOffer::Retry,
    SyncOffer::GiveUp,
    SyncOffer::Unlock,
    SyncOffer::OpenSharingAndKeys,
    SyncOffer::CheckHost,
    SyncOffer::SetIdentity,
    SyncOffer::ShowHolder,
    SyncOffer::SetUpSharing,
];

/// Repository `a` holding the ticket, publishing over SSH, indexed, in an
/// open window. With `committed`, the publication selection is committed,
/// so a share is accepted. `false` dirties the configuration, so a share is
/// refused before its acceptance.
pub(super) fn open(committed: bool, cx: &mut TestAppContext) -> (Shell, PathBuf) {
    open_with(committed, false, cx).0
}

/// As `open`, with an empty repository `b` too when `other`.
pub(super) fn open_with(
    committed: bool,
    other: bool,
    cx: &mut TestAppContext,
) -> ((Shell, PathBuf), Option<PathBuf>) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let other = other.then(|| fixture.create_repository("b"));
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    fixture.publish_over_ssh(&root);
    if committed {
        let config = fs::read(root.join(CONFIG_PATH)).unwrap();
        fixture.commit_file(&root, CONFIG_PATH, &config);
    } else {
        // `remote select` commits the selection, and since C5's fix of
        // `01M669FJR2` it leaves the index current, so the tree is clean.
        // An uncommitted edit keeps the configuration dirty, as the stale
        // index left it before that fix.
        let mut config = fs::read(root.join(CONFIG_PATH)).unwrap();
        config.extend_from_slice(b"# An uncommitted edit.\n");
        fs::write(root.join(CONFIG_PATH), config).unwrap();
    }
    fixture.refresh(&root);
    ((Shell::open(fixture, cx), root), other)
}

pub(super) fn item_sync(root: &Path) -> Intent {
    Intent::ItemSync {
        root: root.to_owned(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    }
}

pub(super) fn answer(code: ResultCode, effects: Effects) -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        effects,
        false,
    )
}

fn synced(publication: PublicationEffect) -> Effects {
    Effects {
        publication,
        ..Effects::not_requested()
    }
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

/// The item share `n`, finished with `code` and `effects`.
pub(super) fn shared(
    shell: &Shell,
    root: &Path,
    n: u64,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    finished(shell, n, item_sync(root), envelope, cx)
}

/// The labels of the offers the entry of `id` shows, in order of `OFFERS`.
pub(super) fn offered(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Vec<&'static str> {
    let mut shown = Vec::new();
    for offer in OFFERS {
        let part = request_id(id, offer_part(offer));
        if shell.present(part.clone(), cx) {
            assert_eq!(shell.label(part, cx).as_deref(), Some(offer_label(offer)));
            shown.push(offer_label(offer));
        }
    }
    shown
}

pub(super) fn outcome(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<String> {
    shell.label(request_id(id, "outcome"), cx)
}

pub(super) fn assert_row(
    shell: &Shell,
    id: DesktopRequestId,
    words: Option<&str>,
    offers: &[SyncOffer],
    cx: &mut TestAppContext,
) {
    assert_eq!(outcome(shell, id, cx).as_deref(), words);
    let expected: Vec<&str> = offers.iter().map(|offer| offer_label(*offer)).collect();
    assert_eq!(offered(shell, id, cx), expected);
    assert!(shell.present(request_id(id, "dismiss"), cx));
}

// --- One test per `SyncOutcome` row ---

#[gpui_kit::test]
fn a_published_share_reads_shared(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = answer(ResultCode::Ok, synced(PublicationEffect::Published));
    let id = shared(&shell, &root, 1, envelope, cx);
    assert_row(&shell, id, Some(SHARED), &[], cx);
}

#[gpui_kit::test]
fn a_current_share_reads_the_same_as_the_shared_copy(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = answer(ResultCode::Ok, synced(PublicationEffect::Current));
    let id = shared(&shell, &root, 1, envelope, cx);
    assert_row(&shell, id, Some(CURRENT), &[], cx);
}

#[gpui_kit::test]
fn a_stopped_share_offers_share(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = answer(ResultCode::Cancelled, Effects::not_requested());
    let id = shared(&shell, &root, 1, envelope, cx);
    assert_row(
        &shell,
        id,
        Some(STOPPED_BEFORE_SENDING),
        &[SyncOffer::Share],
        cx,
    );
}

fn conflict() -> Envelope<MutationDataDto> {
    answer(ResultCode::WorktreeConflicted, Effects::not_requested()).with_data(
        MutationDataDto::SynchronizationConflict(SynchronizationConflictMutationDto {
            r#type: SynchronizationConflictType::SynchronizationConflict,
            conflict: ConflictNextDto {
                operation_id: "01M4CC0VN1KTQFSVR99378TDHV".to_owned(),
                next_action: ConflictNextAction::ALL[0],
            },
        }),
    )
}

/// The conflict offers nothing (decision 13), not even the envelope's
/// own recovery actions.
#[gpui_kit::test]
fn a_conflicted_share_needs_attention_and_offers_nothing(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = conflict().with_recovery(vec![RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [],
    )]);
    let id = shared(&shell, &root, 1, envelope, cx);
    assert_row(&shell, id, Some(NEEDS_ATTENTION), &[], cx);
    assert!(!shell.present(super::super::recovery_id(id, 0), cx));
}

#[gpui_kit::test]
fn a_locked_key_offers_unlock(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    for (n, code) in [ResultCode::UnlockRequired, ResultCode::UnlockFailed]
        .into_iter()
        .enumerate()
    {
        let id = shared(
            &shell,
            &root,
            n as u64,
            answer(code, Effects::not_requested()),
            cx,
        );
        assert_row(&shell, id, Some(KEY_LOCKED), &[SyncOffer::Unlock], cx);
    }
}

#[gpui_kit::test]
fn a_key_problem_offers_sharing_and_keys(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    for (n, code) in [
        ResultCode::NoSelectedKey,
        ResultCode::KeyUnavailable,
        ResultCode::KeyRejected,
    ]
    .into_iter()
    .enumerate()
    {
        let id = shared(
            &shell,
            &root,
            n as u64,
            answer(code, Effects::not_requested()),
            cx,
        );
        assert_row(
            &shell,
            id,
            Some(CHOOSE_A_KEY),
            &[SyncOffer::OpenSharingAndKeys],
            cx,
        );
    }
}

pub(super) fn host_problem(code: ResultCode, named: bool) -> Envelope<MutationDataDto> {
    let envelope = answer(code, Effects::not_requested());
    if !named {
        return envelope;
    }
    envelope.with_recovery(vec![RecoveryAction::new(
        RecoveryActionKind::HostApprove,
        [
            ("authority", "git.example.com:2222".into()),
            ("root", "/fixture/repo".into()),
        ],
    )])
}

#[gpui_kit::test]
fn a_host_problem_names_the_host_and_offers_check_host(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let named = shared(
        &shell,
        &root,
        1,
        host_problem(ResultCode::HostApprovalRequired, true),
        cx,
    );
    assert_eq!(
        outcome(&shell, named, cx).as_deref(),
        Some("Check git.example.com before sharing")
    );
    assert_row(
        &shell,
        named,
        Some(
            check_host_text(Some(&SshAuthority {
                host: "git.example.com".to_owned(),
                port: 2222,
            }))
            .as_str(),
        ),
        &[SyncOffer::CheckHost],
        cx,
    );
    let unnamed = shared(
        &shell,
        &root,
        2,
        host_problem(ResultCode::HostReplacementRequired, false),
        cx,
    );
    assert_row(
        &shell,
        unnamed,
        Some("Check the host before sharing"),
        &[SyncOffer::CheckHost],
        cx,
    );
}

#[gpui_kit::test]
fn an_identity_problem_offers_set_identity_and_retry(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = answer(ResultCode::IdentityRequired, Effects::not_requested());
    let id = shared(&shell, &root, 1, envelope, cx);
    assert_row(
        &shell,
        id,
        Some(NEEDS_IDENTITY),
        &[SyncOffer::Retry, SyncOffer::SetIdentity],
        cx,
    );
}

#[gpui_kit::test]
fn a_busy_share_offers_show_it_and_retry(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let id = shared(
        &shell,
        &root,
        1,
        answer(ResultCode::Busy, Effects::not_requested()),
        cx,
    );
    assert_row(
        &shell,
        id,
        Some(ANOTHER_SHARE_HOLDS),
        &[SyncOffer::Retry, SyncOffer::ShowHolder],
        cx,
    );
}

#[gpui_kit::test]
fn a_share_with_no_remote_offers_set_up_sharing(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = answer(
        ResultCode::PublicationRemoteRequired,
        Effects::not_requested(),
    );
    let id = shared(&shell, &root, 1, envelope, cx);
    assert_row(
        &shell,
        id,
        Some(NOWHERE_TO_SHARE),
        &[SyncOffer::SetUpSharing],
        cx,
    );

    shell.click(request_id(id, offer_part(SyncOffer::SetUpSharing)), cx);
    shell.run(cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert!(matches!(shown, Shown::Settings(SettingsPage::Remotes)));
}

/// A deleted shared branch offers Give up while the record is accepted,
/// and never Share: republishing is D5's (decision 8).
#[gpui_kit::test]
async fn a_deleted_branch_offers_give_up_and_no_share(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(true, cx);
    let id = real_share(
        &shell,
        &root,
        answer(ResultCode::RemoteBranchDeleted, Effects::not_requested()),
        cx,
    )
    .await;
    assert_row(&shell, id, Some(BRANCH_DELETED), &[SyncOffer::GiveUp], cx);
    assert!(!shell.present(request_id(id, offer_part(SyncOffer::Share)), cx));
}

/// Share on an item that has nothing saved: the library answers
/// `item_not_found`, read as nothing to share yet (decision 18), with
/// nothing to offer. Share main's `item_not_found` is a failure.
#[gpui_kit::test]
async fn an_item_with_nothing_saved_has_nothing_to_share(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(false, cx);
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let id = worker.submit(item_sync(&root), Lane::User, None, None);
    shell.until_ended(id, cx).await;
    assert_eq!(
        shell.label(request_id(id, "code"), cx).as_deref(),
        Some("item_not_found")
    );
    assert_row(&shell, id, Some(NOTHING_TO_SHARE), &[], cx);

    let main = finished(
        &shell,
        1,
        Intent::RepoSync { root: root.clone() },
        answer(ResultCode::ItemNotFound, Effects::not_requested()),
        cx,
    );
    assert_eq!(outcome(&shell, main, cx), None);
}

/// Any other failure says nothing of its own: the entry shows the
/// library's message and code. It offers Retry while the record is not
/// finished.
#[gpui_kit::test]
fn another_failure_shows_its_message_and_offers_retry(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = answer(ResultCode::PushRejected, Effects::not_requested());
    let id = shared(&shell, &root, 1, envelope, cx);
    assert_row(&shell, id, None, &[SyncOffer::Retry], cx);
    assert_eq!(
        shell.label(request_id(id, "code"), cx).as_deref(),
        Some("push_rejected")
    );
}

// --- Helpers shared with the route tests ---

/// Runs Share main for real, held once its library call has returned,
/// then answered `envelope`; gives its request once it has finished.
pub(super) async fn real_share(
    shell: &Shell,
    root: &Path,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let held = shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeRepo, envelope);
    let id = submit_main(shell, root, cx);
    held.wait_reached();
    held.release();
    shell.until_ended(id, cx).await;
    id
}

pub(super) fn submit_main(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> DesktopRequestId {
    let root = root.to_owned();
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .services()
            .worker()
            .submit(Intent::RepoSync { root }, Lane::User, None, None)
    })
}
