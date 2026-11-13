//! The attempt entry in the operation area (D6 Task 9, A2, A4, R-20, R-23,
//! W-6): one entry per repository for its latest attempt, replaced by the
//! next, absent for a quiet one; each disposition listed with its text and
//! calling no mutation; and Share… on `Unpublished` alone, reachable by the
//! arrows and Tab and pressed by a click on its drawn button. The attempts
//! reach the schedule as the scheduler hands them (`PollSchedule::after`).

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, ItemDtoKind, OperationId, PollDisposition},
};

use crate::desktop::{
    model::{
        activity::{
            copy::SHARING,
            rows::{ActivityRow, Kind, RowKey, Target},
        },
        lifecycle_lock::LockKey,
        polling::{
            Answer, AttemptTarget, Clock, Disposition, PollAttempt, TargetKind, entry::SHARE,
        },
    },
    services::{JobKind, requests::Intent},
    view::{
        activity_support::{
            act, act_refused, button_focus, drawn, highlighted, index_of, offered, refresh, row,
            rows, to_row,
        },
        item::lock_words,
        settings::polling_support::Rig,
    },
};

/// An attempt of `ix`'s repository that completed over `dispositions`, one
/// item each.
fn attempt(
    rig: &Rig,
    ix: usize,
    dispositions: &[Disposition],
    cx: &mut TestAppContext,
) -> Vec<ItemId> {
    let ids: Vec<ItemId> = dispositions.iter().map(|_| ItemId::generate()).collect();
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: rig.key(ix),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: dispositions
            .iter()
            .zip(&ids)
            .map(|(&disposition, id)| AttemptTarget {
                item_id: Some(id.clone()),
                kind: Some(TargetKind::Ticket),
                disposition,
                main: false,
            })
            .collect(),
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    rig.shell.run(cx);
    ids
}

/// The rows that list `ix`'s repository's latest attempt: its problems and
/// its unshared items, in the log's order.
fn entry(rig: &Rig, ix: usize, cx: &mut TestAppContext) -> Vec<ActivityRow> {
    let root = rig.key(ix);
    rows(&rig.shell, cx)
        .into_iter()
        .filter(|row| match &row.row.key {
            RowKey::Problem { root: of, .. } | RowKey::Unshared { root: of, .. } => *of == root,
            _ => false,
        })
        .collect()
}

fn problem(rig: &Rig, ix: usize, id: &ItemId) -> RowKey {
    RowKey::Problem {
        root: rig.key(ix),
        target: Target::Item(id.clone()),
    }
}

fn unshared(rig: &Rig, ix: usize, id: &ItemId) -> RowKey {
    RowKey::Unshared {
        root: rig.key(ix),
        target: Target::Item(id.clone()),
    }
}

fn keys(rows: &[ActivityRow]) -> Vec<RowKey> {
    rows.iter().map(|row| row.row.key.clone()).collect()
}

/// The repository's Brought rows (S2): what the checks brought in.
fn brought(rig: &Rig, ix: usize, cx: &mut TestAppContext) -> usize {
    let root = rig.key(ix);
    rows(&rig.shell, cx)
        .iter()
        .filter(|row| matches!(&row.row.key, RowKey::Brought { root: of, .. } if *of == root))
        .count()
}

#[gpui_kit::test]
async fn dispositions_listed(cx: &mut TestAppContext) {
    use Disposition::*;
    let rig = Rig::new(&[("a", true), ("b", true)], cx);
    rig.schedule(0, cx);
    rig.schedule(1, cx);
    assert!(entry(&rig, 0, cx).is_empty(), "no attempt, no entry");

    // A quiet attempt lists nothing that needs a look: its one item ahead of
    // the shared copy is a reminder to share, not a problem.
    let quiet = attempt(&rig, 0, &[Current, LocalAhead], cx);
    assert_eq!(
        keys(&entry(&rig, 0, cx)),
        [unshared(&rig, 0, &quiet[1])],
        "quiet: no problem"
    );

    // Everything the poll leaves alone is listed, in the poll's order. The
    // problems keep their order among themselves, the unshared ones follow,
    // and only the unpublished one is checked for Share….
    let left_alone = [
        Dirty,
        Diverged,
        Conflicted,
        Inaccessible,
        BlockedByLocalState,
        MaterializationIncomplete,
        Nonconforming,
        Malformed,
        RemotelyDeleted,
        HistoryUnknown,
        Unpublished,
        NotReached,
    ];
    let ids = attempt(&rig, 0, &left_alone, cx);
    assert!(entry(&rig, 1, cx).is_empty(), "one entry per repository");
    let listed = entry(&rig, 0, cx);
    let mut expected: Vec<RowKey> = Vec::new();
    for (&disposition, id) in left_alone.iter().zip(&ids) {
        if !matches!(disposition, Diverged | Unpublished) {
            expected.push(problem(&rig, 0, id));
        }
    }
    expected.push(unshared(&rig, 0, &ids[1]));
    expected.push(unshared(&rig, 0, &ids[10]));
    assert_eq!(keys(&listed), expected, "twelve lines, in the poll's order");
    let guidance = [
        (0, PollDisposition::Dirty),
        (2, PollDisposition::Conflicted),
        (3, PollDisposition::Inaccessible),
        (4, PollDisposition::BlockedByLocalState),
        (5, PollDisposition::MaterializationIncomplete),
        (6, PollDisposition::Nonconforming),
        (7, PollDisposition::Malformed),
        (8, PollDisposition::RemotelyDeleted),
        (9, PollDisposition::HistoryUnknown),
        (11, PollDisposition::NotReached),
    ];
    for (i, disposition) in guidance {
        let key = problem(&rig, 0, &ids[i]);
        let row = listed
            .iter()
            .find(|row| row.row.key == key)
            .expect("listed");
        assert_eq!(row.row.kind, Kind::Failure, "line {i}");
        assert!(
            row.row.detail[0].ends_with(disposition.guidance()),
            "line {i} carries the library's guidance: {:?}",
            row.row.detail
        );
        assert!(
            offered(&rig.shell, &key, SHARE, cx).is_none(),
            "line {i} offers no Share"
        );
    }
    let unpublished = unshared(&rig, 0, &ids[10]);
    let share = offered(&rig.shell, &unpublished, SHARE, cx).expect("Share… on Unpublished");
    assert!(share.enabled);
    assert_eq!(brought(&rig, 0, cx), 0, "nothing brought in");

    // A disposition is only a row: nothing was sent.
    assert!(rig.sent(cx, |_| true).is_empty(), "no request, no mutation");

    // The next attempt replaces the entry; a quiet one removes it. What it
    // brought in stays as a row of its own.
    let next = attempt(&rig, 0, &[Dirty, FastForwarded], cx);
    assert_eq!(brought(&rig, 0, cx), 1, "Brought in 1 change");
    let listed = entry(&rig, 0, cx);
    assert_eq!(keys(&listed), [problem(&rig, 0, &next[0])], "replaced");
    assert!(
        listed[0].row.detail[0].ends_with(PollDisposition::Dirty.guidance()),
        "{:?}",
        listed[0].row.detail
    );
    attempt(&rig, 0, &[Current], cx);
    assert!(entry(&rig, 0, cx).is_empty(), "quiet again: no entry");

    // Another repository's entry is its own.
    let other = attempt(&rig, 1, &[Malformed], cx);
    let listed = entry(&rig, 1, cx);
    assert_eq!(keys(&listed), [problem(&rig, 1, &other[0])]);
    assert!(
        listed[0].row.detail[0].ends_with(PollDisposition::Malformed.guidance()),
        "{:?}",
        listed[0].row.detail
    );
    assert!(entry(&rig, 0, cx).is_empty());
    assert!(rig.sent(cx, |_| true).is_empty(), "still no mutation");
}

/// W-6: Share… is reachable by the arrows to its row, where the list
/// scrolls it into view, and by a click on its button; Enter on the row's
/// button presses it. Each sends the share of its own item, as the item
/// header's Share does, and nothing else; while a share runs, its button
/// reads "Sharing…" and offers no second press. Thirty rows come first, so
/// the two Share… rows are below the first screenful of the list until the
/// arrows reach them.
#[gpui_kit::test]
async fn attempt_entry_actions_hit(cx: &mut TestAppContext) {
    use Disposition::*;
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    let mut dispositions = vec![Dirty; 30];
    dispositions.extend([Unpublished, Unpublished]);
    let ids = attempt(&rig, 0, &dispositions, cx);
    let (first, second) = (unshared(&rig, 0, &ids[30]), unshared(&rig, 0, &ids[31]));
    let shares = |rig: &Rig, cx: &mut TestAppContext| -> Vec<ItemId> {
        rig.sent(cx, |_| true)
            .into_iter()
            .map(|request| match request.intent {
                Intent::ItemSync { root, kind, id } => {
                    assert_eq!(root, rig.roots[0]);
                    assert_eq!(kind, AuthoringKind::Ticket);
                    id
                }
                _ => panic!("not a share"),
            })
            .collect()
    };
    assert!(shares(&rig, cx).is_empty());
    assert_eq!(
        offered(&rig.shell, &first, SHARE, cx).map(|offer| offer.enabled),
        Some(true)
    );

    // A click on the first row's button, once the arrows have brought the
    // row into view.
    let held = rig.shell.backend.hold_job(JobKind::SynchronizeItem);
    to_row(&rig.shell, &first, cx);
    let at = index_of(&rig.shell, &first, cx);
    rig.shell.click(("activity-button", at), cx);
    assert_eq!(shares(&rig, cx), [ids[30].clone()]);
    held.wait_reached();
    rig.shell.run(cx);
    refresh(&rig.shell, cx);
    let sharing = row(&rig.shell, &first, cx)
        .and_then(|row| row.row.button)
        .expect("its button");
    assert_eq!(sharing.label, SHARING, "while it runs");
    assert!(!sharing.enabled, "no second press while it runs");

    // Enter on the second row's button.
    to_row(&rig.shell, &second, cx);
    rig.shell.keys("tab", cx);
    rig.shell.press("enter", cx);
    assert_eq!(shares(&rig, cx), [ids[30].clone(), ids[31].clone()]);
    held.release();
}

/// The lock on `ix`'s item `id`, as an open close dialog holds it, or not.
fn lock(rig: &Rig, ix: usize, id: &ItemId, held: bool, cx: &mut TestAppContext) {
    let holder = rig.shell.workspace.entity_id();
    let locked = LockKey::new(&rig.key(ix), &id.to_string());
    rig.shell.operations(cx).update(cx, |operations, cx| {
        operations.locks_mut().set_dialog(holder, locked, held);
        cx.notify();
    });
    rig.shell.run(cx);
}

/// D5 A2, R-23: Share… waits while the item's close or merge is
/// unfinished, as the item header's Share does: a stale `Unpublished`
/// line offers no live Share… over a locked item, pressing what is shown
/// sends nothing, and it comes back when the lock clears.
#[gpui_kit::test]
async fn attempt_entry_share_waits_for_the_lock(cx: &mut TestAppContext) {
    use Disposition::*;
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    let ids = attempt(&rig, 0, &[Unpublished], cx);
    let key = unshared(&rig, 0, &ids[0]);
    assert!(
        offered(&rig.shell, &key, SHARE, cx)
            .expect("Share…")
            .enabled
    );

    lock(&rig, 0, &ids[0], true, cx);
    refresh(&rig.shell, cx);
    let waiting = offered(&rig.shell, &key, SHARE, cx).expect("the row still offers Share…");
    assert!(!waiting.enabled, "it waits");
    let held = lock_words(ItemDtoKind::Ticket);
    let row = rows(&rig.shell, cx)
        .into_iter()
        .find(|row| row.row.key == key)
        .expect("the row stays");
    assert!(
        row.row.detail.iter().any(|line| line.contains(held)),
        "it says why: {:?}",
        row.row.detail
    );
    // A disabled button is no tab stop: Tab from the list passes it by.
    to_row(&rig.shell, &key, cx);
    rig.shell.keys("tab", cx);
    let button = button_focus(&rig.shell, cx);
    assert!(
        !rig.shell.frame(cx, |window, _| button.is_focused(window)),
        "not focusable"
    );
    act_refused(&rig.shell, &key, SHARE, cx);
    assert!(rig.sent(cx, |_| true).is_empty(), "nothing is sent");

    lock(&rig, 0, &ids[0], false, cx);
    refresh(&rig.shell, cx);
    assert!(
        offered(&rig.shell, &key, SHARE, cx)
            .expect("Share…")
            .enabled
    );
    act(&rig.shell, &key, SHARE, cx);
    assert_eq!(rig.sent(cx, |_| true).len(), 1);
}

/// R-23: a row is keyed by its repository and its item, not its place, so
/// a focused Share… button stays where it is, and on its own item, when an
/// earlier repository's rows disappear.
#[gpui_kit::test]
async fn attempt_entry_ids_follow_the_repository(cx: &mut TestAppContext) {
    use Disposition::*;
    let rig = Rig::new(&[("a", true), ("b", true)], cx);
    rig.schedule(0, cx);
    rig.schedule(1, cx);
    attempt(&rig, 0, &[Unpublished], cx);
    let theirs = attempt(&rig, 1, &[Unpublished], cx);
    let key = unshared(&rig, 1, &theirs[0]);
    to_row(&rig.shell, &key, cx);
    rig.shell.keys("tab", cx);
    let button = button_focus(&rig.shell, cx);
    assert!(rig.shell.frame(cx, |window, _| button.is_focused(window)));

    // The first repository's rows go: the second keeps its row, its
    // highlight, its focus and its item.
    attempt(&rig, 0, &[Current], cx);
    refresh(&rig.shell, cx);
    for _ in 0..2 {
        drawn(&rig.shell, cx);
    }
    assert!(
        entry(&rig, 0, cx).is_empty(),
        "the first repository's rows went"
    );
    assert_eq!(
        offered(&rig.shell, &key, SHARE, cx).map(|offer| offer.enabled),
        Some(true)
    );
    assert_eq!(highlighted(&rig.shell, cx).as_ref(), Some(&key));
    assert!(
        rig.shell.frame(cx, |window, _| button.is_focused(window)),
        "focus stays on its button"
    );
    rig.shell.press("enter", cx);
    let sent = rig.sent(cx, |_| true);
    assert_eq!(sent.len(), 1);
    assert!(matches!(&sent[0].intent, Intent::ItemSync { id, .. } if *id == theirs[0]));
}
