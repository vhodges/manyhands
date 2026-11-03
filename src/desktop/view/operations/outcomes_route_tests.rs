//! A finished share's controls that open a place in the workspace
//! (design, Lane B, Outcomes and routes), Unlock… and Share after Not now
//! (R-3, R-13, R-17), and the possible holder of a busy repository (Part
//! 0, Busy repositories), over the helpers of `outcomes_tests`.

use std::{
    fs,
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{
    repository::{
        SharedKeyId,
        keys::{
            KeySourceToken, PassphraseResponse, SessionCredentialProvider, UnlockReason,
            UnlockRequest,
        },
        transport::SshAuthority,
    },
    results::{Effects, ResultCode},
};

use super::{
    offer_part, share_result, share_route,
    tests::{
        TICKET, answer, assert_row, chosen_line, host_problem, open, open_with, outcome,
        real_share, shared, submit_main,
    },
};
use crate::desktop::{
    model::{
        operations::{ANOTHER_SHARE_HOLDS, KEY_LOCKED, may_be_waiting_for},
        publication::SyncOffer,
        stamp::SettingsPage,
    },
    services::{
        JobKind, ReadKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::item_id,
    view::{
        harness::{Shell, WAIT, key, wait_until},
        item::ItemRoute,
        operations::request_id,
        settings::Shown,
        workspace::Region,
    },
};

// --- Routes ---

/// A key problem's Open Sharing & keys opens the Keys page.
#[gpui_kit::test]
fn no_selected_key_opens_sharing_and_keys(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let envelope = answer(ResultCode::NoSelectedKey, Effects::not_requested());
    let id = shared(&shell, &root, 1, envelope, cx);
    shell.click(
        request_id(id, offer_part(SyncOffer::OpenSharingAndKeys)),
        cx,
    );
    shell.run(cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert!(matches!(shown, Shown::Settings(SettingsPage::Keys)));
}

/// Check host on a host problem checks the host the outcome names, on the
/// Hosts page of the share's repository, though another is selected. No
/// transport is initialized, so the check answers its typed failure.
#[gpui_kit::test]
async fn a_host_code_checks_the_host_with_its_authority(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let ((shell, root), other) = open_with(false, true, cx);
    shell.select(&other.unwrap(), cx);
    let id = shared(
        &shell,
        &root,
        1,
        host_problem(ResultCode::HostApprovalRequired, true),
        cx,
    );
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let route = cx.read(|cx| {
        let result = share_result(&services, id, cx).expect("a finished share");
        share_route(&services, id, &result, SyncOffer::CheckHost, cx)
    });
    let authority = SshAuthority {
        host: "git.example.com".to_owned(),
        port: 2222,
    };
    assert!(matches!(
        &route,
        Some(ItemRoute::CheckHost { root: asked, authority: Some(named) })
            if *asked == root && *named == authority
    ));

    let held = shell.backend.hold_job(JobKind::ObserveHost);
    shell.click(request_id(id, offer_part(SyncOffer::CheckHost)), cx);
    held.wait_reached();
    assert!(shell.present("settings-hosts", cx));
    assert_eq!(
        shell.label("hosts-root", cx),
        Some(chosen_line(&key(&root)))
    );
    held.release();
}

/// K-3: a comment's host stop has no host data; its Check host opens the
/// Hosts page of the comment's repository, checking nothing.
#[gpui_kit::test]
fn a_hostless_comment_stop_opens_the_roots_hosts_page(cx: &mut TestAppContext) {
    let ((shell, root), other) = open_with(false, true, cx);
    shell.select(&other.unwrap(), cx);
    open_ticket(&shell, &root, cx);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("the ticket's tab is shown");
    let asked = root.clone();
    pane.update(cx, |_, cx| {
        cx.emit(ItemRoute::CheckHost {
            root: asked,
            authority: None,
        })
    });
    shell.run(cx);
    assert!(shell.present("settings-hosts", cx));
    assert_eq!(
        shell.label("hosts-root", cx),
        Some(chosen_line(&key(&root)))
    );
    assert!(!shell.backend.jobs_seen().contains(&JobKind::ObserveHost));
}

fn open_ticket(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = crate::desktop::model::navigation::OpenTarget::Item {
        repo: key(root),
        kind: manyhands::repository::ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

// --- Unlock… and Share after Not now ---

/// A provider call for the key `key_id` in flight on a plain thread, as
/// the worker makes one.
fn ask(shell: &Shell, key_id: SharedKeyId, cx: &mut TestAppContext) -> Receiver<&'static str> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("key");
    fs::write(&path, b"opaque fixture bytes").unwrap();
    let request = UnlockRequest {
        key_id,
        label: "primary".to_owned(),
        source: KeySourceToken::observe(&path).unwrap(),
        reason: UnlockReason::ProtectedKey,
    };
    let mut provider = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().provider());
    let (done, answer) = mpsc::channel();
    thread::spawn(move || {
        let _directory = directory;
        let answer = match provider.request_passphrase(&request) {
            PassphraseResponse::Supplied(_) => "supplied",
            PassphraseResponse::Cancelled => "cancelled",
            PassphraseResponse::Unavailable => "unavailable",
        };
        let _ = done.send(answer);
    });
    answer
}

async fn until_prompted(shell: &Shell, cx: &mut TestAppContext) {
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.passphrase().is_some()
    })
    .await;
    shell.run(cx);
}

/// Runs Share main for real, held once its library call has returned;
/// while it is held, the key `key_id` is asked for and declined with Not
/// now, so the attempt is answered `Cancelled` for it; then the share is
/// answered `unlock_required`.
async fn declined_share(
    shell: &Shell,
    root: &Path,
    key_id: SharedKeyId,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let held = shell.backend.hold_job_return_answering(
        JobKind::SynchronizeRepo,
        answer(ResultCode::UnlockRequired, Effects::not_requested()),
    );
    let id = submit_main(shell, root, cx);
    held.wait_reached();
    let asked = ask(shell, key_id, cx);
    until_prompted(shell, cx).await;
    shell.click("passphrase-cancel", cx);
    assert_eq!(asked.recv_timeout(WAIT).unwrap(), "cancelled");
    // Declined: asked again, it answers at once.
    let again = ask(shell, key_id, cx);
    assert_eq!(again.recv_timeout(WAIT).unwrap(), "cancelled");
    held.release();
    shell.until_ended(id, cx).await;
    id
}

/// Asks for `key_id` again: the dialog opens, since its Not now was
/// cleared; Not now answers it.
async fn prompts_again(shell: &Shell, key_id: SharedKeyId, cx: &mut TestAppContext) {
    let asked = ask(shell, key_id, cx);
    until_prompted(shell, cx).await;
    shell.click("passphrase-cancel", cx);
    assert_eq!(asked.recv_timeout(WAIT).unwrap(), "cancelled");
}

/// R-3, R-13: Unlock… on a share whose record is still accepted clears
/// the key its attempt was declined for, and sends the same request
/// again, under its own request ID; the key then prompts again.
#[gpui_kit::test]
async fn unlock_clears_the_declined_key_and_retries_the_same_request(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(true, cx);
    let key_id = SharedKeyId::new();
    let id = declined_share(&shell, &root, key_id, cx).await;
    assert_row(
        &shell,
        id,
        Some(KEY_LOCKED),
        &[SyncOffer::GiveUp, SyncOffer::Unlock],
        cx,
    );
    let first = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(id).cloned())
        .unwrap();

    let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
    shell.click(request_id(id, offer_part(SyncOffer::Unlock)), cx);
    held.wait_reached();
    let retried = shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(id).cloned())
        .unwrap();
    assert_eq!(retried.request_id, first.request_id, "the same request");
    assert_eq!(retried.attempt, first.attempt + 1);
    let mains = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    });
    assert_eq!(mains, [id], "no new request");
    held.release();
    shell.until_ended(id, cx).await;
    prompts_again(&shell, key_id, cx).await;
}

/// R-17: a share whose record is finished offers Share, not Unlock…; it
/// first clears the key the finished request was declined for, so the
/// new request prompts for it rather than answering `unlock_required`
/// unasked.
#[gpui_kit::test]
async fn share_after_a_finished_locked_key_clears_its_declined_key(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(false, cx);
    let key_id = SharedKeyId::new();
    let id = declined_share(&shell, &root, key_id, cx).await;
    assert_row(&shell, id, Some(KEY_LOCKED), &[SyncOffer::Share], cx);

    let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
    shell.click(request_id(id, offer_part(SyncOffer::Share)), cx);
    held.wait_reached();
    let mains = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    });
    assert_eq!(mains.len(), 2, "Share sent a new request");
    held.release();
    shell.until_ended(mains[0], cx).await;
    prompts_again(&shell, key_id, cx).await;
}

// --- Busy ---

/// Show it, on a share answered `busy` while a share of this session
/// that stopped after reserving still holds the repository: the entry
/// names it, and Show it gives its entry the focus.
#[gpui_kit::test]
async fn show_it_focuses_the_entry_of_the_possible_holder(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(true, cx);
    let holder = real_share(
        &shell,
        &root,
        answer(ResultCode::UnlockRequired, Effects::not_requested()),
        cx,
    )
    .await;
    let busy = shared(
        &shell,
        &root,
        1,
        answer(ResultCode::Busy, Effects::not_requested()),
        cx,
    );
    assert_eq!(
        outcome(&shell, busy, cx),
        Some(may_be_waiting_for("Share main"))
    );
    shell.click(request_id(busy, offer_part(SyncOffer::ShowHolder)), cx);
    shell.run(cx);
    let focused = shell.frame(cx, |window, _| {
        window
            .try_find(request_id(holder, "entry"))
            .is_some_and(|entry| entry.focused() == Some(true))
    });
    assert!(focused, "the holder's entry has the focus");
}

/// With no possible holder in this session, Show it points to the
/// pending operations: the operation area takes the focus.
#[gpui_kit::test]
fn busy_without_a_known_holder_points_to_pending_operations(cx: &mut TestAppContext) {
    let (shell, root) = open(false, cx);
    let id = shared(
        &shell,
        &root,
        1,
        answer(ResultCode::Busy, Effects::not_requested()),
        cx,
    );
    assert_eq!(
        outcome(&shell, id, cx).as_deref(),
        Some(ANOTHER_SHARE_HOLDS)
    );
    let before = shell.backend.reads_seen(ReadKind::Operations);
    shell.click(request_id(id, offer_part(SyncOffer::ShowHolder)), cx);
    shell.run(cx);
    assert!(shell.present(Region::Operations.ring_id(), cx));
    assert!(
        shell.backend.reads_seen(ReadKind::Operations) > before,
        "the pending operations were read again"
    );
    assert_eq!(
        shell.label("pending-empty", cx).as_deref(),
        Some(super::super::NOTHING_PENDING)
    );
}

// --- The record ---

/// R-17: a failure the same request cannot get past, its record
/// finished, offers Share, a new request, and not Retry, which would only
/// replay the answer.
#[gpui_kit::test]
async fn a_final_failure_on_a_finished_record_offers_share_not_retry(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = open(false, cx);
    let id = real_share(
        &shell,
        &root,
        answer(ResultCode::PushRejected, Effects::not_requested()),
        cx,
    )
    .await;
    assert_row(&shell, id, None, &[SyncOffer::Share], cx);
    assert!(!shell.present(request_id(id, offer_part(SyncOffer::Retry)), cx));
}
