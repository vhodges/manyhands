//! D5's secret sweep (Cycle 05b P12; design A1, A3): a key's private text and
//! a typed passphrase reach nothing the close and resolve surfaces draw. The
//! lifecycle dialog is driven through a close that stops for a locked key,
//! its Unlock… prompt answered with a sentinel passphrase, a Retry and Done,
//! then a remote conflict's sheet is opened and a choice made on it. At each
//! stage every label and value the window gives, the Kit's title, description
//! and footer slots included (each is an observed element with an ID), is
//! read, and neither sentinel is among them.
//!
//! The answers are canned (decision 9), as `credential_tests.rs` and
//! `privacy_remote_tests.rs` drive them. The key is real: it is imported
//! through the library, and its private file holds `PRIVATE_KEY_SENTINEL`,
//! which no desktop control may show or copy.

use std::path::Path;

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        AuthoringKind, ItemDtoKind, LifecycleCleanupPendingReason, LifecycleCleanupState,
        MutationDataDto, OperationId, RequestId, SharedKeyId,
    },
    results::{CleanupEffect, Envelope, Outcome, ResultCode},
};

use super::{
    credential_tests::ask,
    outcome::{CHECK_HOST, ENDING_DONE, UNLOCK},
    outcome_canned_tests::{answer, inspect, reached, reached_effects, retry},
    tests::{TICKET, closes, open, request, saved_ticket, shell, ticket, until_present},
};
use crate::desktop::{
    services::{JobKind, requests::Intent},
    test_support::fixtures::{PRIVATE_KEY_SENTINEL, item_id},
    view::{
        conflict::{entry_tests, tests as conflict},
        harness::{Shell, WAIT, wait_until},
        item::LIFECYCLE_ACTION,
        passphrase::tests::window_text,
    },
};

/// The passphrase typed into the prompt: a test constant, no real secret.
const PASSPHRASE_SENTINEL: &str = "SENTINEL-P12-PASSPHRASE-4d7b";

/// The synchronization `entry_tests::share_conflict` names.
const SYNCHRONIZATION: &str = "01M4CC0VN1KTQFSVR99378TDHV";

/// Reads every label and value in the window at `stage`, and fails naming
/// the sentinel and the text that holds it. Each of `slots` is an element the
/// stage must show with a label (a title, a description, a footer button), so
/// the sweep is shown to reach it.
fn sweep(shell: &Shell, stage: &str, slots: &[&'static str], cx: &mut TestAppContext) {
    let text = window_text(shell, cx);
    assert!(!text.is_empty(), "{stage}: the window gave no text");
    for slot in slots {
        let label = shell
            .label(*slot, cx)
            .unwrap_or_else(|| panic!("{stage}: {slot} has no label"));
        assert!(
            text.contains(&label),
            "{stage}: {slot} ({label:?}) is not among the text read"
        );
    }
    for sentinel in [PRIVATE_KEY_SENTINEL, PASSPHRASE_SENTINEL] {
        if let Some(held) = text.iter().find(|text| text.contains(sentinel)) {
            panic!("{stage}: {sentinel} is in {held:?}");
        }
    }
}

/// A close answered as the library answers one stopped for a key: `code`,
/// partial, its tidy-up pending for `reason`, the same request to send
/// again.
fn stopped(
    root: &Path,
    code: ResultCode,
    reason: LifecycleCleanupPendingReason,
    request_id: RequestId,
    inspect_too: bool,
) -> Envelope<MutationDataDto> {
    let mut data = reached(ItemDtoKind::Ticket, TICKET, vec![]);
    data.cleanup = Some(LifecycleCleanupState::Pending);
    data.cleanup_pending = Some(reason);
    let mut recovery = vec![retry(request_id)];
    if inspect_too {
        recovery.push(inspect(root));
    }
    answer(
        "ticket close",
        root,
        code,
        Outcome::Partial,
        data,
        reached_effects(CleanupEffect::Pending),
        recovery,
        request_id,
    )
}

/// P12: a close confirmed, running, stopped for a locked key, unlocked with a
/// typed passphrase, stopped for an unknown host, retried and done, then a
/// remote conflict's sheet. Neither the private key's text nor the typed
/// passphrase is on screen at any point.
#[gpui_kit::test]
async fn close_and_resolve_show_no_secret(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut imported = None;
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Secret", "");
        imported = Some(fixture.import_key("primary"));
    });
    let key_id: SharedKeyId = imported.expect("the key is imported");
    open(&shell, ticket(&root), cx);
    sweep(&shell, "the open ticket", &[], cx);

    // A close confirmed.
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    sweep(
        &shell,
        "confirming",
        &["lifecycle-title", "lifecycle-subtitle", "lifecycle-confirm"],
        cx,
    );

    // Running, then answered unlock_required.
    let named = RequestId::new();
    let held = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        stopped(
            &root,
            ResultCode::UnlockRequired,
            LifecycleCleanupPendingReason::RemoteDeletionUnlock,
            named,
            false,
        ),
    );
    shell.click("lifecycle-confirm", cx);
    held.wait_reached();
    sweep(&shell, "running", &["lifecycle-title"], cx);
    held.release();
    let first = closes(&shell, cx)[0];
    shell.until_ended(first, cx).await;
    until_present(&shell, "lifecycle-unlock", cx).await;
    sweep(
        &shell,
        "stopped for a locked key",
        &["lifecycle-title", "lifecycle-ending", "lifecycle-unlock"],
        cx,
    );
    assert_eq!(shell.label("lifecycle-unlock", cx).as_deref(), Some(UNLOCK));

    // Unlock… sends the close again; its key is asked for, and a passphrase
    // typed and supplied at the prompt.
    let again = shell.backend.hold_job_return_answering(
        JobKind::TicketClose,
        stopped(
            &root,
            ResultCode::HostApprovalRequired,
            LifecycleCleanupPendingReason::RemoteDeletionHost,
            named,
            true,
        ),
    );
    shell.click("lifecycle-unlock", cx);
    again.wait_reached();
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 2, "Unlock… sent the close again");
    assert_eq!(
        request(&shell, sent[0], cx).request_id,
        request(&shell, first, cx).request_id,
        "the same request"
    );
    let asked = ask(&shell, key_id, cx);
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.passphrase().is_some()
    })
    .await;
    shell.run(cx);
    sweep(
        &shell,
        "the prompt shown",
        &["passphrase-title", "passphrase-label", "passphrase-ok"],
        cx,
    );
    // The masked field hides its value from accessibility, so this sweep
    // cannot see the typed sentinel, and `ask` returns a static string, not
    // the typed value: the sweeps here prove no leak into the surfaces
    // they read, not that the field holds the sentinel.
    shell.visual(cx).simulate_input(PASSPHRASE_SENTINEL);
    shell.run(cx);
    sweep(&shell, "the passphrase typed", &["passphrase-ok"], cx);
    shell.keys("enter", cx);
    assert_eq!(asked.recv_timeout(WAIT).unwrap(), "supplied");
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.passphrase().is_none()),
        "the prompt closed"
    );
    sweep(&shell, "the passphrase supplied", &["lifecycle-title"], cx);
    again.release();
    shell.until_ended(sent[0], cx).await;
    until_present(&shell, "lifecycle-retry", cx).await;
    sweep(
        &shell,
        "stopped for an unknown host",
        &[
            "lifecycle-ending",
            "lifecycle-retry",
            "lifecycle-check-host",
        ],
        cx,
    );
    assert_eq!(
        shell.label("lifecycle-check-host", cx).as_deref(),
        Some(CHECK_HOST)
    );

    // Retry, answered done.
    let done = answer(
        "ticket close",
        &root,
        ResultCode::Ok,
        Outcome::Success,
        reached(ItemDtoKind::Ticket, TICKET, vec![]),
        reached_effects(CleanupEffect::Complete),
        vec![],
        named,
    );
    shell
        .backend
        .hold_job_return_answering(JobKind::TicketClose, done)
        .release();
    shell.click("lifecycle-retry", cx);
    let third = closes(&shell, cx)[0];
    shell.until_ended(third, cx).await;
    shell
        .until_drawn(cx, "the ending Done", |shell, cx| {
            shell.label("lifecycle-ending", cx).as_deref() == Some(ENDING_DONE)
        })
        .await;
    sweep(&shell, "done", &["lifecycle-title", "lifecycle-ending"], cx);
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));

    // A remote conflict's sheet: opened, and a part kept.
    let share = Intent::ItemSync {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    };
    entry_tests::fabricate(&shell, 1, share, entry_tests::share_conflict(), cx);
    until_present(&shell, "item-review-changes", cx).await;
    sweep(&shell, "the conflict banner", &["item-review-changes"], cx);
    shell.backend.answer_next_conflict_read(conflict::canned(
        OperationId::parse(SYNCHRONIZATION).unwrap(),
        vec![conflict::canned_path(TICKET, '1')],
    ));
    shell.click("item-review-changes", cx);
    until_present(&shell, "conflict-save", cx).await;
    sweep(
        &shell,
        "the sheet open",
        &["conflict-title", "conflict-description", "conflict-save"],
        cx,
    );
    shell.click(("conflict-keep-mine", 0usize), cx);
    sweep(
        &shell,
        "a part kept",
        &["conflict-title", "conflict-save"],
        cx,
    );
}
