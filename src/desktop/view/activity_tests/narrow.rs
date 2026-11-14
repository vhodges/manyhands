//! `rows_fit_a_narrow_activity` (P21): the one containment test of Activity's
//! rows (K-19). Activity sits in the right dock at 320 px in a window too
//! short to show every row; the message is long and the tag longer; every
//! row, scrolled to with `scroll_to_item` after it was out of the list, has
//! its button inside the list. The tests that press a button no longer check
//! its bounds (E-8): this is where it is checked, for every kind of row.
//!
//! Two windows hold the kinds: the polling double's (a hold with a long name
//! and "+1", requests that offer Retry and Share…, pending Resume rows) and
//! the schedule's (a state row, reminders, a brought-in row, a retirement
//! event, a lifecycle's Resume).

use std::path::PathBuf;

use gpui_kit::{TestAppContext, component::dock::DockPlacement, px, size, test::TestWindowExt};
use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, ContextRetirementState as Retirement, RetirementRemoval as Removal,
    },
    results::ResultCode,
};

use super::{
    cases::{TICKET, answered, fabricate},
    clear::finished_after,
    follow::{found, item, main_branch},
    list_support::{OP_A, OP_B, save, waiting_close},
    pending_cases::{listing, read_pending},
};
use crate::desktop::{
    model::{
        activity::{
            copy::message_of,
            rows::{ActivityAction, ActivityRow, RowKey},
        },
        layout::{PanelKind, Side},
        polling::{Answer, Disposition, TransportFailure},
    },
    services::{requests::Intent, worker::RecordState},
    test_support::retire::answer as retirement,
    view::{
        activity_support::{drawn, refresh},
        dock::tests::support::{dock_panel, edit_area},
        harness::Shell,
        operations::attention_support::Rig,
        settings::polling_support::Rig as ScheduleRig,
    },
};

/// A repository name no row can show in 320 px, and a short one beside it.
const LONG: &str = "a-repository-whose-name-is-far-too-long-for-a-narrow-activity-panel";
const LONGER: &str = "another-repository-with-a-name-longer-still-than-the-first-one-is";

/// A window too short for the rows: about two rows of 28 px show.
const HEIGHT: f32 = 360.;

/// Moves Activity into the right dock at 320 px, in a short window, and
/// checks every row: its message and tag are long enough to matter, the row
/// is out of the list until it is scrolled to, and then it and its button lie
/// inside the list. Returns the rows.
fn fits(shell: &Shell, cx: &mut TestAppContext) -> Vec<ActivityRow> {
    dock_panel(shell, PanelKind::Activity, Side::Right, cx);
    edit_area(shell, cx, |area, window, cx| {
        area.set_dock_size(DockPlacement::Right, px(320.), window, cx)
    });
    shell
        .visual(cx)
        .simulate_resize(size(px(1280.), px(HEIGHT)));
    let settle = |cx: &mut TestAppContext| {
        for _ in 0..3 {
            shell.frame(cx, |_, _| ());
            shell.run(cx);
        }
    };
    settle(cx);
    let width = shell.frame(cx, |window, _| {
        window.find("operation-area").bounds().size.width
    });
    assert!(
        width <= px(330.) && width > px(100.),
        "the list is {width:?} wide"
    );

    let rows = drawn(shell, cx);
    let longest = rows
        .iter()
        .map(|row| message_of(&row.row).len() + row.row.repo.as_ref().map_or(0, String::len))
        .max()
        .unwrap();
    assert!(
        longest > 100,
        "a message and tag too long to fit: {longest}"
    );

    let highlight = |key: &RowKey, cx: &mut TestAppContext| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.highlight_applied_row(key);
            cx.notify();
        });
        settle(cx);
    };
    // At the top, the last row is not drawn: scrolling to it is real.
    highlight(&rows[0].row.key, cx);
    let last = rows.len() - 1;
    assert!(
        !shell.present(("activity-row", last), cx),
        "the last of {} rows is out of the list before it is scrolled to",
        rows.len()
    );

    // Each row scrolled to, then its button against the list.
    for (ix, row) in rows.iter().enumerate() {
        highlight(&row.row.key, cx);
        let (list, row_at, button) = shell.frame(cx, |window, _| {
            (
                window.find("operation-area").bounds(),
                window
                    .try_find(("activity-row", ix))
                    .map(|row| row.bounds()),
                window
                    .try_find(("activity-button", ix))
                    .map(|button| button.bounds()),
            )
        });
        let row_at = row_at.unwrap_or_else(|| panic!("row {ix} is drawn once scrolled to"));
        assert!(
            row_at.left() >= list.left()
                && row_at.right() <= list.right()
                && row_at.top() >= list.top()
                && row_at.bottom() <= list.bottom(),
            "row {ix} {row_at:?} lies outside the list {list:?} once scrolled to"
        );
        match (&row.row.button, button) {
            (None, None) => {}
            (Some(offer), Some(button)) => assert!(
                button.size.width > px(0.)
                    && button.left() >= list.left()
                    && button.right() <= list.right()
                    && button.top() >= list.top()
                    && button.bottom() <= list.bottom(),
                "row {ix}'s {:?} button {button:?} lies inside the list {list:?}",
                offer.label
            ),
            (want, got) => panic!("row {ix}: button {want:?} drawn as {got:?}"),
        }
    }
    rows
}

fn has(rows: &[ActivityRow], kind: impl Fn(&ActivityRow) -> bool, what: &str) {
    assert!(rows.iter().any(kind), "a {what} row: {rows:?}");
}

#[gpui_kit::test]
async fn rows_fit_a_narrow_activity(cx: &mut TestAppContext) {
    cx.executor().allow_parking();

    // The polling double's window: a hold over both repositories ("long +1",
    // Unlock…), requests that offer Retry and Share…, pending Resume rows.
    let rig = Rig::new(&[(LONG, true), ("b", true)], cx);
    let shell = &rig.shell;
    let root = PathBuf::from(&rig.roots[0]);
    let key = rig.lock();
    rig.list(cx);
    shell
        .until_drawn(cx, "the hold", |_, cx| rig.entries(cx).len() == 1)
        .await;
    let _ = key;
    fabricate(
        shell,
        1,
        Intent::RepoSync { root: root.clone() },
        answered("repo sync", ResultCode::InternalError),
        cx,
    );
    fabricate(
        shell,
        2,
        Intent::ItemSync {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: ItemId::generate(),
        },
        answered("item sync", ResultCode::Cancelled),
        cx,
    );
    fabricate(
        shell,
        3,
        Intent::IndexRebuild { root: root.clone() },
        answered("index rebuild", ResultCode::IndexUnavailable),
        cx,
    );
    read_pending(
        shell,
        &rig.roots[0],
        listing(vec![save(OP_A), save(OP_B)]),
        cx,
    );
    refresh(shell, cx);
    let rows = fits(shell, cx);
    let buttons = rows.iter().filter(|row| row.row.button.is_some()).count();
    assert!(
        rows.len() >= 6 && buttons >= 5,
        "rows of each kind: {} rows, {buttons} with a button",
        rows.len()
    );
    has(&rows, |row| matches!(row.row.key, RowKey::Hold(_)), "hold");
    has(
        &rows,
        |row| {
            row.row
                .repo
                .as_deref()
                .is_some_and(|tag| tag.ends_with("+1"))
        },
        "tag with +N",
    );
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::Request(_)),
        "request",
    );
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::Pending { .. }),
        "pending",
    );

    // The schedule's window: reminders (an item and main), a problem, a
    // brought-in row, a retirement offer and event and a lifecycle's Resume
    // of the first long repository; a state row (Retry now) of the second.
    let rig = ScheduleRig::new(&[(LONG, true), (LONGER, true)], cx);
    let shell = &rig.shell;
    // Both repositories are listed before the schedule speaks of them.
    shell
        .until_drawn(cx, "the repositories", |shell, cx| {
            shell.present(("repo-row", 1usize), cx)
        })
        .await;
    rig.schedule(0, cx);
    rig.schedule(1, cx);
    found(
        &rig,
        0,
        vec![
            item(Disposition::Unpublished),
            main_branch(Disposition::Unpublished),
            item(Disposition::Dirty),
            item(Disposition::FastForwarded),
            item(Disposition::Closed),
        ],
        cx,
    );
    rig.answer(1, Answer::Transport(TransportFailure::Unreachable), cx);
    // A request's end reads the pending list again, so the retirement comes
    // first.
    finished_after(
        shell,
        1,
        Intent::ContextRetire {
            root: rig.roots[0].clone(),
            kind: AuthoringKind::Ticket,
            id: crate::desktop::test_support::retire::ITEM.parse().unwrap(),
        },
        RecordState::Finished,
        retirement(Retirement::Retired, ResultCode::Ok, Removal::Observed),
        cx,
    );
    read_pending(
        shell,
        &rig.key(0),
        listing(vec![waiting_close(OP_B, TICKET)]),
        cx,
    );
    refresh(shell, cx);
    let rows = fits(shell, cx);
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::Repository(_)),
        "state",
    );
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::Unshared { .. }),
        "reminder",
    );
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::Problem { .. }),
        "problem",
    );
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::Brought { .. }),
        "brought-in",
    );
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::RetireOffer { .. }),
        "retire offer",
    );
    has(
        &rows,
        |row| matches!(row.row.key, RowKey::Retired(_)),
        "retirement event",
    );
    has(
        &rows,
        |row| {
            row.row
                .button
                .as_ref()
                .is_some_and(|offer| matches!(offer.action, ActivityAction::ResumeLifecycle { .. }))
        },
        "lifecycle Resume",
    );
}
