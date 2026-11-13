//! P23: each button the popover can carry, run by keyboard from it. The
//! popover is opened with Enter on the footer item, the row highlighted by
//! the arrows, and Enter presses its button through `choose_activity` with
//! the footer item as the origin, after the popover closes. The rows come
//! from the same fixtures as P18's, so a row of each source is there, and
//! what each action sent is read from the request model, the recording
//! backend or the dialog, sheet or page it opened. A row drawn before the
//! log's took a newer revision, or whose offer changed, runs nothing, and
//! the popover stays open with the row refreshed (T8, R-55).

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{repository::OperationAction, results::ResultCode};

use super::*;
use crate::desktop::{
    model::{
        activity::{
            copy::{
                OPEN_LIFECYCLE, OPEN_SHARING_AND_KEYS, REPUBLISH, REVIEW_CHANGES, SET_UP_SHARING,
                SHOW_IT, UNLOCK,
            },
            rows::RowKey,
        },
        polling::{
            Answer, AttemptTarget, Clock, Disposition, PollAttempt, TargetKind, TransportFailure,
            attention::{
                APPROVE_HOST, CHOOSE_A_KEY, OPEN_REMOTE_SETTINGS, RESUME_CHECKING,
                UNLOCK as UNLOCK_KEY,
            },
        },
        stamp::SettingsPage,
        tabs::TabKey,
    },
    services::{ReadKind, ReadResult, requests::Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity::tests::{
            cases::{OPERATION, TICKET, answered, dialog_open, fabricate, item_sync, settings},
            list_support::{OP_A, OP_B, OP_D, change_row, press_in, resumes, save, waiting_close},
            pending_cases::{PENDING_ROOT, listing, read_pending},
        },
        activity_support::{pending_key, refresh},
        harness::Shell,
        operations::{
            attention_tests::{every_reason, hold_with, shown},
            republish::tests::{finished_deleted_share, republish_preview, ticket_shell},
        },
        settings::{Shown, polling_support::Rig},
    },
};

/// Opens the popover with Enter on the footer item, moves the highlight to
/// `key` with the arrows, says its button reads `label` and is enabled, and
/// presses it with Enter. The action closes the popover.
fn press_from_popover(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) {
    refresh(shell, cx);
    settle_popover(shell, cx);
    focus_item(shell, cx);
    shell.press("enter", cx);
    assert!(is_open(shell, cx), "Enter on the item opens it ({label})");
    settle_popover(shell, cx);
    let order = keys_of(&applied(shell, cx));
    let target = order
        .iter()
        .position(|row| row == key)
        .unwrap_or_else(|| panic!("{key:?} is not listed: {order:?}"));
    for _ in 0..target {
        shell.keys("down", cx);
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
async fn dialog_and_back(shell: &Shell, key: &RowKey, label: &str, cx: &mut TestAppContext) {
    press_from_popover(shell, key, label, cx);
    shell.until_drawn(cx, "the dialog", dialog_open).await;
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert!(!dialog_open(shell, cx), "{label}: the dialog closed");
    assert_eq!(spot(shell, cx), Spot::Item, "{label}: focus is on the item");
}

/// P23: the table of sources and buttons, and the stale rows.
#[gpui_kit::test]
async fn popover_actions_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    stale_rows(cx);
    holds(cx).await;
    shares(cx);
    dialogs(cx).await;
    pending(cx).await;
    polling(cx).await;
}

/// T8, R-55: the popover's Enter and click on a row drawn before the log's
/// row took a newer revision, or whose offer changed, run nothing and leave
/// the popover open with the row refreshed. The log's row went A, B, A in
/// one update, so it offers what the popover drew: only its revision refuses
/// the choice.
fn stale_rows(cx: &mut TestAppContext) {
    for by_click in [false, true] {
        let world = rows(cx);
        let shell = &world.shell;
        settle(shell, cx);
        open_by_click(shell, cx);
        let drawn_rows = keys_of(&applied(shell, cx));
        let stale = world.pending[3].clone();
        let at = drawn_rows
            .iter()
            .position(|row| row == &stale)
            .expect("the row is listed");
        for _ in 0..at {
            shell.keys("down", cx);
        }
        assert_eq!(cursor(shell, cx).as_ref(), Some(&stale));
        let (before, requests) = (
            shell
                .workspace
                .read_with(cx, |workspace, _| workspace.attention_pressed()),
            shell.requests(cx, |_| true).len(),
        );
        let operations = shell.operations(cx);
        shell.frame(cx, |window, cx| {
            let (drawn, current) = change_row(shell, &operations, OP_D, cx);
            assert!(drawn.is_some() && current.is_some(), "the row is in both");
            assert_ne!(drawn, current, "the log's row took a newer revision");
            if by_click {
                window.click(("needs-attention-button", at), cx);
            } else {
                press_in(window, "enter", cx);
            }
        });
        shell.run(cx);
        let pressed = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.attention_pressed());
        assert_eq!(pressed, before + 1, "the drawn choice was built");
        assert_eq!(
            shell.requests(cx, |_| true).len(),
            requests,
            "and refused: nothing ran (by click {by_click})"
        );
        assert_eq!(resumes(shell, OP_D, cx), 0);
        settle_popover(shell, cx);
        assert!(is_open(shell, cx), "the popover stays open");
        let revisions = |shell: &Shell, cx: &mut TestAppContext| {
            let log = attention_rows(shell, cx)
                .into_iter()
                .find(|row| row.row.key == stale)
                .map(|row| row.revision);
            let popover = applied(shell, cx)
                .into_iter()
                .find(|row| row.row.key == stale)
                .map(|row| row.revision);
            (popover, log)
        };
        let (popover, log) = revisions(shell, cx);
        assert_eq!(popover, log, "with the row refreshed");
        // Pressed again, on the refreshed row, it runs.
        shell.press("enter", cx);
        shell.run(cx);
        assert_eq!(resumes(shell, OP_D, cx), 1, "the second press runs it");
    }

    // The offer changed: the row left the log with its Resume, A, B.
    let world = rows(cx);
    let shell = &world.shell;
    settle(shell, cx);
    open_by_click(shell, cx);
    let stale = world.pending[3].clone();
    let at = keys_of(&applied(shell, cx))
        .iter()
        .position(|row| row == &stale)
        .expect("the row is listed");
    for _ in 0..at {
        shell.keys("down", cx);
    }
    let requests = shell.requests(cx, |_| true).len();
    let operations = shell.operations(cx);
    shell.frame(cx, |window, cx| {
        operations.update(cx, |operations, cx| {
            let stamp = operations.load_pending(PENDING_ROOT.to_owned());
            let list = listing(vec![save(OP_A), save(OP_B)]);
            assert!(operations.apply_pending(stamp, list));
            cx.notify();
        });
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.refresh_activity_now(cx));
        press_in(window, "enter", cx);
    });
    shell.run(cx);
    assert_eq!(shell.requests(cx, |_| true).len(), requests, "nothing ran");
    assert_eq!(resumes(shell, OP_D, cx), 0);
    settle_popover(shell, cx);
    assert!(is_open(shell, cx), "the popover stays open");
    assert!(
        !keys_of(&applied(shell, cx)).contains(&stale),
        "with the row gone"
    );
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
    press_from_popover(shell, &hold_with(&rig, CHOOSE_A_KEY, cx), CHOOSE_A_KEY, cx);
    assert_eq!(page(cx), Shown::Settings(SettingsPage::Keys));
    press_from_popover(shell, &hold_with(&rig, APPROVE_HOST, cx), APPROVE_HOST, cx);
    assert_eq!(page(cx), Shown::Settings(SettingsPage::Hosts));
    let remote = hold_with(&rig, OPEN_REMOTE_SETTINGS, cx);
    press_from_popover(shell, &remote, OPEN_REMOTE_SETTINGS, cx);
    assert_eq!(page(cx), Shown::Settings(SettingsPage::Remotes));
    let resume = hold_with(&rig, RESUME_CHECKING, cx);
    press_from_popover(shell, &resume, RESUME_CHECKING, cx);
    assert_eq!(rig.backend.resumed(), [rig.roots[4].clone()]);
}

/// What a share offer's button did.
enum Effect {
    /// The same request again: its attempt count rose.
    Again,
    /// A Settings page opened.
    Page(SettingsPage),
    /// Activity's list took focus (Show it).
    Holder,
}

/// S1: a failed share of each kind of answer, whose first offer is its
/// button: what the answer's code makes of it, and what the button does.
fn shares(cx: &mut TestAppContext) {
    let table = [
        (ResultCode::UnlockRequired, UNLOCK, Effect::Again),
        (
            ResultCode::NoSelectedKey,
            OPEN_SHARING_AND_KEYS,
            Effect::Page(SettingsPage::Keys),
        ),
        (
            ResultCode::PublicationRemoteRequired,
            SET_UP_SHARING,
            Effect::Page(SettingsPage::Remotes),
        ),
        (ResultCode::Busy, SHOW_IT, Effect::Holder),
        (ResultCode::RemoteUnavailable, "Retry", Effect::Again),
    ];
    for (code, label, effect) in table {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        fixture.commit_ticket(&root, TICKET, "Ticket", "");
        fixture.refresh(&root);
        let shell = Shell::open(fixture, cx);
        let id = fabricate(&shell, 1, item_sync(&root), answered("item sync", code), cx);
        let row = RowKey::Request(id);
        press_from_popover(&shell, &row, label, cx);
        match effect {
            Effect::Again => {
                let attempt =
                    crate::desktop::view::activity_support::request(&shell, id, cx).attempt;
                assert!(attempt > 1, "{label}: the request was sent again");
            }
            Effect::Page(page) => {
                assert_eq!(settings(&shell, cx), Shown::Settings(page), "{label}");
            }
            Effect::Holder => {
                shell.frame(cx, |_, _| ());
                assert!(
                    crate::desktop::view::activity_support::list_focused(&shell, cx),
                    "{label}: Activity's list took focus: the action placed it"
                );
            }
        }
    }
}

/// The buttons that open a dialog, each in a window of its own: Open…,
/// Republish…, a recovery that confirms and a share's Review changes. The
/// dialog's close gives focus back to the footer item.
async fn dialogs(cx: &mut TestAppContext) {
    use manyhands::results::{RecoveryAction, RecoveryActionKind};

    use crate::desktop::model::activity::rows::ActivityAction;
    let (shell, root) = ticket_shell(cx);
    let deleted = RowKey::Request(finished_deleted_share(&shell, &root, cx).await);
    let close = RowKey::Request(fabricate(
        &shell,
        1,
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        answered("ticket close", ResultCode::PushRejected),
        cx,
    ));
    let prepare = RowKey::Request(fabricate(
        &shell,
        2,
        Intent::OperationAbandon {
            root: root.clone(),
            operation_id: manyhands::repository::OperationId::parse(OPERATION).unwrap(),
        },
        answered("operation abandon", ResultCode::ExternalChange).with_recovery(vec![
            RecoveryAction::new(RecoveryActionKind::RequestPrepare, []),
        ]),
        cx,
    ));
    refresh(&shell, cx);
    let prepare_again = crate::desktop::view::activity_support::label_of(
        &shell,
        &prepare,
        &ActivityAction::Recovery(
            match &prepare {
                RowKey::Request(id) => *id,
                _ => unreachable!(),
            },
            0,
        ),
        cx,
    );
    shell
        .backend
        .answer_next_prepare("item republish", republish_preview(&shell, &root));
    dialog_and_back(&shell, &deleted, REPUBLISH, cx).await;
    dialog_and_back(&shell, &close, OPEN_LIFECYCLE, cx).await;
    dialog_and_back(&shell, &prepare, &prepare_again, cx).await;

    // Review changes, on a share stopped on a conflict.
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Mine");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let share = RowKey::Request(crate::desktop::view::conflict::entry_tests::fabricate(
        &shell,
        6,
        item_sync(&root),
        crate::desktop::view::conflict::entry_tests::share_conflict(),
        cx,
    ));
    press_from_popover(&shell, &share, REVIEW_CHANGES, cx);
    shell
        .until_drawn(cx, "the sheet", |shell, cx| {
            shell.present("conflict-sheet", cx)
        })
        .await;
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

/// S4 and S5: an unreachable repository's Retry now, and a problem item's
/// Open, from the schedule's own transitions.
async fn polling(cx: &mut TestAppContext) {
    use manyhands::canonical::ItemId;
    let rig = Rig::new(&[("a", true)], cx);
    let root = rig.key(0);
    rig.schedule(0, cx);
    rig.answer(0, Answer::Transport(TransportFailure::Unreachable), cx);
    let shell = &rig.shell;
    refresh(shell, cx);
    let state = RowKey::Repository(root.clone());
    assert!(
        keys_of(&attention_rows(shell, cx)).contains(&state),
        "an unreachable repository needs attention"
    );
    press_from_popover(shell, &state, "Retry now", cx);
    assert_eq!(
        rig.sent(cx, |intent| matches!(intent, Intent::PollOnce { .. }))
            .len(),
        1,
        "Retry now sent a check"
    );

    // A problem item: Open takes the person to it.
    let rig = Rig::new(&[("a", true)], cx);
    let root = rig.key(0);
    rig.schedule(0, cx);
    let problem = ItemId::generate();
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: root.clone(),
        operation: manyhands::repository::OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: vec![AttemptTarget {
            item_id: Some(problem.clone()),
            kind: Some(TargetKind::Ticket),
            disposition: Disposition::Dirty,
            main: false,
        }],
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    rig.shell.run(cx);
    let row = RowKey::Problem {
        root: root.clone(),
        target: crate::desktop::model::activity::rows::Target::Item(problem.clone()),
    };
    assert!(
        keys_of(&attention_rows(&rig.shell, cx)).contains(&row),
        "a problem item needs attention"
    );
    press_from_popover(&rig.shell, &row, "Open", cx);
    let tab = TabKey::Item {
        repo: root,
        kind: manyhands::repository::ItemDtoKind::Ticket,
        id: problem.to_string(),
    };
    let tabs = rig
        .shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys());
    assert!(tabs.contains(&tab), "its tab opened");
}
