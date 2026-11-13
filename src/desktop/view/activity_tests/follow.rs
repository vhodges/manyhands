//! The log follows the models, whether or not Activity is drawn (Wave 04
//! Cycle 03b): `activity_hidden_keeps_the_log` (P29),
//! `a_checking_repository_keeps_its_row` (P30) and `reminders_match_to_share`
//! (P32). Clocks are the test's; nothing waits in real time.

use std::time::Duration;

use gpui_kit::TestAppContext;
use manyhands::{canonical::ItemId, repository::OperationId, results::ResultCode};

use super::{
    cases::{answered, fabricate, label_of},
    list_support::{OP_A, save},
    pending_cases::{PENDING_ROOT, listing, read_pending},
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityAction, ActivityRow, RowKey, Target},
        layout::PanelKind,
        polling::{
            Answer, AttemptTarget, Clock, Disposition, PollAttempt, TargetKind, TransportFailure,
        },
        repository_state::RowAction,
    },
    services::{JobKind, requests::Intent},
    view::{
        activity_support::{act, act_refused, drawn, refresh, row, rows},
        dock::tests::support::toggle_panel,
        footer::{Control, tests::support::read, to_share_text},
        settings::polling_support::Rig,
    },
};

/// The rows of a window as `(key, title, time after `start`)`, newest first:
/// what a person would read there, with times relative to the clock's
/// start so two windows can be compared.
fn log_of(
    rig: &Rig,
    start: time::OffsetDateTime,
    cx: &mut TestAppContext,
) -> Vec<(RowKey, String, time::Duration)> {
    rows(&rig.shell, cx)
        .into_iter()
        .map(|row: ActivityRow| (row.row.key, row.row.title, row.at - start))
        .collect()
}

/// A refresh that succeeded, then five minutes on a failure of the index,
/// then a minute on a pending operation: the same three things in any
/// window, at the same moments of its own clock.
fn arrivals(rig: &Rig, cx: &mut TestAppContext) -> time::OffsetDateTime {
    let shell = &rig.shell;
    let start = rig.clock.utc();
    let refresh_of = || Intent::IndexRefresh {
        root: rig.roots[0].clone(),
    };
    fabricate(
        shell,
        1,
        refresh_of(),
        answered("index refresh", ResultCode::Ok),
        cx,
    );
    rig.clock.advance(Duration::from_secs(300));
    fabricate(
        shell,
        2,
        refresh_of(),
        answered("index refresh", ResultCode::IndexUnavailable),
        cx,
    );
    rig.clock.advance(Duration::from_secs(60));
    read_pending(shell, PENDING_ROOT, listing(vec![save(OP_A)]), cx);
    refresh(shell, cx);
    start
}

/// P29: rows that arrive while Activity is hidden are in the log with the
/// times they arrived at, and the panel shown afterwards draws exactly them:
/// the same rows and times as in a window where it was shown all along.
#[gpui_kit::test]
fn activity_hidden_keeps_the_log(cx: &mut TestAppContext) {
    // The control: Activity shown while the rows arrive.
    let shown = Rig::new(&[("a", false)], cx);
    let start = arrivals(&shown, cx);
    let control = log_of(&shown, start, cx);
    assert_eq!(control.len(), 3, "the three arrivals: {control:?}");
    let offsets: Vec<i64> = control.iter().map(|(.., at)| at.whole_seconds()).collect();
    assert_eq!(offsets, [360, 300, 0], "newest first, each at its arrival");

    // Hidden while they arrive.
    let hidden = Rig::new(&[("a", false)], cx);
    toggle_panel(&hidden.shell, PanelKind::Activity, cx);
    assert!(
        !hidden.shell.present("operation-area", cx),
        "Activity is not drawn"
    );
    let start = arrivals(&hidden, cx);
    assert_eq!(
        log_of(&hidden, start, cx),
        control,
        "the log kept the rows and times while nothing drew them"
    );
    assert!(
        drawn(&hidden.shell, cx).is_empty(),
        "and nothing was applied to a pane that was not there"
    );

    // Shown: the pane draws the log's rows.
    toggle_panel(&hidden.shell, PanelKind::Activity, cx);
    for _ in 0..3 {
        hidden.shell.frame(cx, |_, _| ());
        hidden.shell.run(cx);
    }
    let shown_rows = drawn(&hidden.shell, cx);
    let keys: Vec<RowKey> = shown_rows.iter().map(|row| row.row.key.clone()).collect();
    let expected: Vec<RowKey> = control.iter().map(|(key, ..)| key.clone()).collect();
    assert_eq!(keys, expected, "the pane shows the same rows");
    assert_eq!(
        shown_rows
            .iter()
            .map(|row| row.at - start)
            .collect::<Vec<_>>(),
        control.iter().map(|(.., at)| *at).collect::<Vec<_>>(),
        "at the times they arrived, not the time the panel was shown"
    );
}

/// P30: a repository's state row stays while a check of it runs, with every
/// offer disabled and its time unchanged (L3), and then leaves or stays by
/// the answer.
#[gpui_kit::test]
async fn a_checking_repository_keeps_its_row(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.answer(0, Answer::Transport(TransportFailure::Unreachable), cx);
    let shell = &rig.shell;
    refresh(shell, cx);
    let at = RowKey::Repository(rig.key(0));
    let before = row(shell, &at, cx).expect("the repository has a state row");
    let retry = label_of(
        shell,
        &at,
        |action| matches!(action, ActivityAction::Repository(_, RowAction::RetryNow)),
        cx,
    );
    let checks =
        |cx: &mut TestAppContext| rig.sent(cx, |intent| matches!(intent, Intent::PollOnce { .. }));

    // A retry that starts: the check is held in the library.
    let gate = shell.backend.hold_job(JobKind::PollOnce);
    act(shell, &at, &retry, cx);
    gate.wait_reached();
    let running = checks(cx);
    assert_eq!(running.len(), 1, "Retry now sent one check");
    refresh(shell, cx);
    let kept = row(shell, &at, cx).expect("the row stays through the check");
    assert_eq!(kept.at, before.at, "its time does not change");
    assert!(
        kept.row
            .menu
            .iter()
            .all(|offer| offer.label == "Dismiss" || !offer.enabled),
        "every offer is disabled while it checks: {:?}",
        kept.row.menu
    );
    let button = kept.row.button.as_ref().expect("a button reads Checking…");
    assert_eq!(button.label, "Checking\u{2026}");
    act_refused(shell, &at, &button.label, cx);
    assert_eq!(checks(cx).len(), 1, "pressing it sent no second check");

    // The check ends, and the answer decides: unreachable again keeps the
    // row, enabled, at its first time.
    gate.release();
    shell.until_ended(running[0].id, cx).await;
    rig.answer(0, Answer::Transport(TransportFailure::Unreachable), cx);
    refresh(shell, cx);
    let again = row(shell, &at, cx).expect("an unreachable answer keeps the row");
    assert_eq!(again.at, before.at, "at its first time");
    assert!(
        again.row.button.as_ref().is_some_and(|offer| offer.enabled),
        "and offers its button again: {:?}",
        again.row.button
    );

    // A second retry that is answered completed: the row leaves.
    let gate = shell.backend.hold_job(JobKind::PollOnce);
    act(shell, &at, &retry, cx);
    gate.wait_reached();
    refresh(shell, cx);
    assert!(
        row(shell, &at, cx).is_some(),
        "the row stays through the second check"
    );
    gate.release();
    let second = checks(cx).pop().expect("the second check");
    shell.until_ended(second.id, cx).await;
    rig.answer(0, Answer::Completed, cx);
    refresh(shell, cx);
    assert!(
        row(shell, &at, cx).is_none(),
        "a completed answer takes the row"
    );
}

/// An attempt of `rig`'s repository that found `targets` unshared.
fn found(rig: &Rig, ix: usize, targets: Vec<AttemptTarget>, cx: &mut TestAppContext) {
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: rig.key(ix),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets,
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    rig.shell.run(cx);
}

fn item(disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: Some(ItemId::generate()),
        kind: Some(TargetKind::Ticket),
        disposition,
        main: false,
    }
}

fn main_branch(disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: None,
        kind: None,
        disposition,
        main: true,
    }
}

/// The footer's "N to share" as it reads.
fn to_share(rig: &Rig, cx: &mut TestAppContext) -> Option<String> {
    read(&rig.shell, Control::ToShare, cx).map(|read| read.0)
}

/// P32: the footer's "N to share" is Cycle 03a's total of the repositories'
/// unshared lists; Activity has one S6 row for each of them, main counted as
/// one. They agree as the lists grow, shrink and move between repositories.
#[gpui_kit::test]
fn reminders_match_to_share(cx: &mut TestAppContext) {
    let rig = Rig::new(&[("a", true), ("b", true)], cx);
    rig.schedule(0, cx);
    rig.schedule(1, cx);
    let agree = |step: &str, expected: usize, cx: &mut TestAppContext| {
        refresh(&rig.shell, cx);
        let reminders = rows(&rig.shell, cx)
            .iter()
            .filter(|row| matches!(row.row.key, RowKey::Unshared { .. }))
            .count();
        assert_eq!(reminders, expected, "{step}: the S6 rows");
        // The item is not drawn at none.
        match (to_share(&rig, cx), expected) {
            (None, 0) => {}
            (Some(name), n) if n > 0 => assert!(
                name.starts_with(&to_share_text(n)),
                "{step}: the footer says {name:?}, not {}",
                to_share_text(n)
            ),
            (shown, n) => panic!("{step}: the footer shows {shown:?} for {n} reminders"),
        }
    };

    // Items and main in the first repository, main in the second.
    found(
        &rig,
        0,
        vec![
            item(Disposition::Unpublished),
            item(Disposition::LocalAhead),
            main_branch(Disposition::LocalAhead),
        ],
        cx,
    );
    found(
        &rig,
        1,
        vec![
            main_branch(Disposition::Unpublished),
            item(Disposition::Diverged),
        ],
        cx,
    );
    agree("items and main", 5, cx);
    let main = RowKey::Unshared {
        root: rig.key(0),
        target: Target::Main,
    };
    assert!(row(&rig.shell, &main, cx).is_some(), "main is one row");

    // Fewer: the first repository shared everything but main.
    found(&rig, 0, vec![main_branch(Disposition::LocalAhead)], cx);
    agree("the first repository shared its items", 3, cx);

    // None: both are current.
    found(&rig, 0, vec![item(Disposition::Current)], cx);
    found(&rig, 1, vec![item(Disposition::Current)], cx);
    agree("nothing to share", 0, cx);
}
