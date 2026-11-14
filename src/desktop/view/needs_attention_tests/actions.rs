//! P23: each button the popover can carry, run by keyboard from it. The
//! popover is opened with Enter on the footer item, the row highlighted by
//! the arrows, and Enter presses its button through `choose_activity` with
//! the footer item as the origin, after the popover closes. The rows come
//! from the same fixtures as P18's, so a row of each source is there, and
//! what each action sent is read from the request model, the recording
//! backend or the dialog, sheet or page it opened. A row drawn before the
//! log's took a newer revision, or whose offer changed, runs nothing, and
//! the popover stays open with the row refreshed (T8, R-55).

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{ItemDtoKind, OperationAction},
    results::ResultCode,
};

use super::*;
use crate::desktop::{
    model::{
        activity::{copy::REVIEW_CHANGES, rows::RowKey},
        polling::attention::{
            APPROVE_HOST, CHOOSE_A_KEY, OPEN_REMOTE_SETTINGS, RESUME_CHECKING, UNLOCK as UNLOCK_KEY,
        },
        stamp::SettingsPage,
    },
    services::{ReadKind, ReadResult},
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity::tests::{
            cases::{OPERATION, TICKET, dialog_open},
            list_support::{OP_B, resumes, save, waiting_close},
            pending_cases::{PENDING_ROOT, listing, read_pending, waiting},
        },
        activity_support::{pending_key, refresh},
        harness::Shell,
        operations::{
            attention_tests::{every_reason, hold_with, shown},
            republish::tests::ticket_shell,
        },
        settings::Shown,
    },
};

/// Opens the popover with Enter on the footer item, moves the highlight to
/// `key` with the arrows, says its button reads `label` and is enabled, and
/// presses it with Enter. The action closes the popover.
pub(super) fn press_from_popover(
    shell: &Shell,
    key: &RowKey,
    label: &str,
    cx: &mut TestAppContext,
) {
    refresh(shell, cx);
    settle_popover(shell, cx);
    focus_item(shell, cx);
    shell.press("enter", cx);
    assert!(is_open(shell, cx), "Enter on the item opens it ({label})");
    settle_popover(shell, cx);
    // The arrows, from where the highlight is to the row, read each step: a
    // row the schedule's polls reorder while the popover is open is still
    // reached.
    for _ in 0..32 {
        let order = keys_of(&applied(shell, cx));
        let target = order
            .iter()
            .position(|row| row == key)
            .unwrap_or_else(|| panic!("{key:?} is not listed: {order:?}"));
        let at = cursor(shell, cx).and_then(|held| order.iter().position(|row| *row == held));
        match at {
            Some(at) if at == target => break,
            Some(at) if at > target => shell.keys("up", cx),
            _ => shell.keys("down", cx),
        }
    }
    assert_eq!(
        cursor(shell, cx).as_ref(),
        Some(key),
        "the row is highlighted"
    );
    let row = applied(shell, cx)
        .into_iter()
        .find(|row| &row.row.key == key)
        .expect("the row is drawn");
    let button = row.row.button.expect("the row carries a button");
    assert_eq!(button.label, label, "{key:?}'s button");
    assert!(button.enabled, "{label} is enabled");
    shell.press("enter", cx);
    shell.run(cx);
    assert!(!is_open(shell, cx), "{label} closed the popover");
}

/// The popover's Enter on `key`'s row, then Escape from the dialog it
/// opened: focus is back on the footer item.
pub(super) async fn dialog_and_back(
    shell: &Shell,
    key: &RowKey,
    label: &str,
    cx: &mut TestAppContext,
) {
    press_from_popover(shell, key, label, cx);
    shell.until_drawn(cx, "the dialog", dialog_open).await;
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert!(!dialog_open(shell, cx), "{label}: the dialog closed");
    assert_eq!(spot(shell, cx), Spot::Item, "{label}: focus is on the item");
}

/// P23, S3, S9, S10 and S11: the holds, the pending operations and the
/// failing draft's flush. S1 is `requests.rs`, S4 and S5 `states.rs`, and
/// the keys and the stale rows `stale.rs`.
#[gpui_kit::test]
async fn popover_actions_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    holds(cx).await;
    pending(cx).await;
    review_pending(cx).await;
    flush(cx).await;
}

/// S3: the five holds, one for each reason, each button pressed from the
/// popover.
async fn holds(cx: &mut TestAppContext) {
    let rig = every_reason(cx).await;
    let shell = &rig.shell;
    let listed: Vec<RowKey> = keys_of(&attention_rows(shell, cx));
    for label in [
        UNLOCK_KEY,
        CHOOSE_A_KEY,
        APPROVE_HOST,
        OPEN_REMOTE_SETTINGS,
        RESUME_CHECKING,
    ] {
        let row = hold_with(&rig, label, cx);
        assert!(listed.contains(&row), "{label}'s hold needs attention");
    }
    let page = |cx: &mut TestAppContext| shown(&rig, cx);
    press_from_popover(shell, &hold_with(&rig, UNLOCK_KEY, cx), UNLOCK_KEY, cx);
    assert_eq!(rig.checks(cx), [std::path::PathBuf::from(&rig.roots[0])]);
    // Settings is a sheet, behind which the footer item cannot be pressed:
    // Escape closes it after each press that opens it.
    // Each lands on the control Wave 04 Cycle 04's Part D names.
    press_from_popover(shell, &hold_with(&rig, CHOOSE_A_KEY, cx), CHOOSE_A_KEY, cx);
    assert_eq!(page(cx), Shown::Settings(SettingsPage::Keys));
    assert!(shell.holds_focus("keys-make", cx), "W-23, no key");
    shell.keys("escape", cx);
    press_from_popover(shell, &hold_with(&rig, APPROVE_HOST, cx), APPROVE_HOST, cx);
    assert_eq!(page(cx), Shown::Settings(SettingsPage::Hosts));
    assert!(shell.holds_focus("hosts-check", cx), "W-25");
    shell.keys("escape", cx);
    let remote = hold_with(&rig, OPEN_REMOTE_SETTINGS, cx);
    press_from_popover(shell, &remote, OPEN_REMOTE_SETTINGS, cx);
    assert_eq!(page(cx), Shown::Settings(SettingsPage::Remotes));
    assert!(shell.holds_focus("remote-add-name", cx), "W-24");
    shell.keys("escape", cx);
    let resume = hold_with(&rig, RESUME_CHECKING, cx);
    press_from_popover(shell, &resume, RESUME_CHECKING, cx);
    assert_eq!(rig.backend.resumed(), [rig.roots[4].clone()]);
}

/// S9 and S10: a pending operation's Resume, a lifecycle's Resume (which
/// also opens its dialog), and a list that failed to read, whose Retry
/// reads it again.
async fn pending(cx: &mut TestAppContext) {
    let (shell, _) = ticket_shell(cx);
    read_pending(
        &shell,
        PENDING_ROOT,
        listing(vec![waiting_close(OPERATION, TICKET), save(OP_B)]),
        cx,
    );
    let plain = pending_key(
        &shell,
        PENDING_ROOT,
        OperationAction::SaveTicket.as_str(),
        cx,
    );
    press_from_popover(&shell, &plain, "Resume", cx);
    assert_eq!(
        resumes(&shell, OP_B, cx),
        1,
        "Resume sent `operation resume`"
    );

    let (shell, _) = ticket_shell(cx);
    read_pending(
        &shell,
        PENDING_ROOT,
        listing(vec![waiting_close(OPERATION, TICKET)]),
        cx,
    );
    let close = pending_key(&shell, PENDING_ROOT, OperationAction::Close.as_str(), cx);
    press_from_popover(&shell, &close, "Resume", cx);
    assert_eq!(
        resumes(&shell, OPERATION, cx),
        1,
        "a lifecycle's Resume resumes"
    );
    assert!(dialog_open(&shell, cx), "and opens its dialog");
    shell.keys("escape", cx);

    // A list that failed to read.
    let shell = Shell::open(Fixture::new(), cx);
    let unreadable = "/fixture/b";
    let failed = crate::desktop::services::ReadFailure {
        code: ResultCode::IndexUnavailable,
        recovery: Vec::new(),
    };
    read_pending(&shell, unreadable, ReadResult::Failed(failed), cx);
    let reads = shell.backend.reads_seen(ReadKind::Operations);
    press_from_popover(
        &shell,
        &RowKey::PendingList(unreadable.to_owned()),
        "Retry",
        cx,
    );
    assert!(
        shell.backend.reads_seen(ReadKind::Operations) > reads,
        "the list was read again"
    );
}

/// S9's Review changes (`ReviewPending`), on a pending operation whose
/// conflict waits to be resolved: the sheet opens, and closing it gives
/// focus back to the footer item.
async fn review_pending(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let shell = Shell::open(fixture, cx);
    let root = crate::desktop::view::harness::key(&root);
    let row = pending_key(&shell, &root, "context_sync_local", cx);
    // The library lists this operation with a Resume. Listed as a remote
    // synchronization (which Resume is not offered on: it takes the
    // request that made it) its conflict, still waiting, is the button.
    let RowKey::Pending { operation, .. } = row.clone() else {
        panic!("a pending row");
    };
    let mut remote = waiting(OperationAction::SaveTicket, &operation, None);
    remote.family = manyhands::repository::OperationFamily::Remote;
    read_pending(&shell, &root, listing(vec![remote]), cx);
    refresh(&shell, cx);
    assert!(!shell.present("conflict-sheet", cx));
    press_from_popover(&shell, &row, REVIEW_CHANGES, cx);
    shell
        .until_drawn(cx, "the sheet", |shell, cx| {
            shell.present("conflict-sheet", cx)
        })
        .await;
    // Closing the sheet gives focus back to the footer item.
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("conflict-sheet", cx), "the sheet closed");
    assert_eq!(spot(&shell, cx), Spot::Item, "focus is on the item");
}

/// S11's Retry, on a draft whose flush failed: the newest text is written
/// now. A draft session, not a request: the worker is not involved.
async fn flush(cx: &mut TestAppContext) {
    use manyhands::drafts::{DraftErrorKind, DraftFields, FailureAction, FailurePoint};

    use crate::desktop::{
        model::{navigation::OpenTarget, tabs::TabKey},
        test_support::drafts::store_draft,
    };
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "One", "");
    fixture.refresh(&root);
    store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Mine");
    let shell = Shell::open(fixture, cx);
    let key = crate::desktop::view::harness::key(&root);
    let target = OpenTarget::Item {
        repo: key.clone(),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.click("draft-restore-button", cx);
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    let tab = TabKey::Item {
        repo: key,
        kind: ItemDtoKind::Ticket,
        id: TICKET.to_owned(),
    };
    let id = drafts
        .read_with(cx, |registry, _| registry.attached(&tab))
        .expect("the draft is restored");
    let mut fields = drafts.read_with(cx, |registry, _| {
        registry.session(id).expect("its session").current().clone()
    });
    if let DraftFields::Ticket { body, .. } = &mut fields {
        body.push_str(" more");
    }
    drafts.update(cx, |registry, cx| registry.edit(id, fields, cx));
    let failing = shell.fixture.draft_store().with_failure_point_for_testing(
        FailurePoint::CreateTemp,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );
    let flushed = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    shell.run(cx);
    flushed.await.expect_err("the flush fails");
    shell.run(cx);
    let flush = RowKey::Flush(id);
    assert!(
        keys_of(&attention_rows(&shell, cx)).contains(&flush),
        "a failed flush needs attention"
    );
    drop(failing);
    let writes = shell.backend.reads_seen(ReadKind::DraftWrite);
    press_from_popover(&shell, &flush, "Retry", cx);
    assert!(
        shell.backend.reads_seen(ReadKind::DraftWrite) > writes,
        "Retry wrote the newest text now"
    );
}
