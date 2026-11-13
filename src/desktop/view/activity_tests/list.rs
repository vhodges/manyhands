//! Activity's list as a person uses it (Wave 04 Cycle 03b, Part C):
//! `rows_draw_from_the_log_and_filter` (P14, the filter being Task 10's),
//! `activity_by_keyboard` (P15, the Tab clause being Task 10's) and
//! `rows_by_pointer` (P16), with T8's drawn-revision rule in the last two.
//! Rows come from fabricated requests and pending lists: the model is the
//! window's, and what a press sent is read from it.

use gpui_kit::{TestAppContext, point, px, test::TestWindowExt};

use super::{cases::TICKET, list_support::*};
use crate::desktop::{
    model::{
        activity::{activity_time, copy::NOTHING_YET, rows::RowKey},
        tabs::TabKey,
    },
    test_support::fixtures::Fixture,
    view::{
        activity_support::{arrow_to, button_focus, drawn, highlighted, index_of, list_focus},
        confirm::local_offset,
        harness::Shell,
    },
};

/// P14: rows draw from the log, newest first, as one line each: the time,
/// the message as the row's name, and the tooltip `tooltip_lines` builds;
/// with none, the empty state (the filter is Task 10's).
#[gpui_kit::test]
fn rows_draw_from_the_log_and_filter(cx: &mut TestAppContext) {
    // No rows: the empty state says so.
    let empty = Shell::open(Fixture::new(), cx);
    settle(&empty, cx);
    assert!(drawn(&empty, cx).is_empty());
    assert_eq!(
        empty.label("activity-empty", cx).as_deref(),
        Some(NOTHING_YET)
    );
    assert!(!empty.present(("activity-row", 0usize), cx));

    // Rows: the pane draws the log's, in the log's order, which is newest
    // first.
    let rows = rows(cx);
    settle(&rows.shell, cx);
    let shown = drawn(&rows.shell, cx);
    for key in [&rows.plain, &rows.item, &rows.disabled]
        .into_iter()
        .chain(&rows.pending)
    {
        assert!(
            shown.iter().any(|row| &row.row.key == key),
            "{key:?} is drawn: {shown:?}"
        );
    }
    assert!(
        shown.windows(2).all(|pair| pair[0].at >= pair[1].at),
        "newest first"
    );
    assert!(
        !rows.shell.present("activity-empty", cx),
        "no empty state beside rows"
    );
    let tooltips = rows
        .shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_pane().tooltips());
    for (ix, row) in shown.iter().enumerate() {
        let message = match &row.row.description {
            Some(description) => format!("{}. {description}", row.row.title),
            None => row.row.title.clone(),
        };
        // The accessible name is the message, the time is B6's, and the
        // tooltip is the message, the time in full, then the row's detail.
        assert_eq!(
            rows.shell.label(("activity-row", ix), cx).as_deref(),
            Some(message.as_str()),
            "row {ix}'s name"
        );
        let (clock, full) = activity_time(row.at, local_offset(row.at.unix_timestamp()));
        assert_eq!(
            rows.shell.label(("activity-time", ix), cx).as_deref(),
            Some(clock.as_str()),
            "row {ix}'s time"
        );
        let mut expected = vec![message, full];
        expected.extend(row.row.detail.iter().cloned());
        assert_eq!(tooltips[ix], expected, "row {ix}'s tooltip");
        if row.row.key == rows.plain {
            assert!(
                row.row.detail.len() >= 2,
                "a request's detail is in its tooltip: {:?}",
                row.row.detail
            );
        }
    }
    // A row's button is drawn on its row: the pending row's Resume.
    let at = index_of(&rows.shell, &rows.pending[0], cx);
    assert!(rows.shell.present(("activity-button", at), cx));
    let plain = index_of(&rows.shell, &rows.plain, cx);
    assert!(!rows.shell.present(("activity-button", plain), cx));
}

/// The row whose menu is open, if one is.
fn menu_key_of(shell: &Shell, cx: &mut TestAppContext) -> Option<RowKey> {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_menu_key())
}

/// P15: Activity by keyboard. Up and down stop at the ends, Home and End
/// go to them; Enter presses the button when it is enabled, opens the item
/// when the row has no button, and does nothing for a disabled button or a
/// row with neither; Enter on the focused button runs it once, and the
/// list's own Enter does not also fire; the highlighted row leaving moves
/// focus from its button to the list; a row drawn before the log's took a
/// newer revision runs nothing on Enter (T8); `shift-f10` and `menu` open the
/// row's menu. Tab through the filter and ⋯ is Task 10's.
///
/// A press sends a request, whose end reads the repository's pending list
/// again and so takes the other pending rows away: each step that presses
/// has a window of its own.
#[gpui_kit::test]
fn activity_by_keyboard(cx: &mut TestAppContext) {
    // A Resume the test presses runs on the worker's thread.
    cx.executor().allow_parking();
    // The highlight, and Enter on rows that do nothing, or open the item.
    let rows = rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let order: Vec<RowKey> = drawn(shell, cx)
        .into_iter()
        .map(|row| row.row.key)
        .collect();
    assert_eq!(highlighted(shell, cx), None, "nothing is highlighted yet");

    // `secondary-4` focuses the list, highlighting the first row.
    shell.keys("secondary-4", cx);
    let list = list_focus(shell, cx);
    assert!(shell.frame(cx, |window, _| list.is_focused(window)));
    assert_eq!(highlighted(shell, cx).as_ref(), Some(&order[0]));

    // Up at the first row stays; down walks to the last and stays; Home and
    // End go to the ends.
    shell.keys("up", cx);
    assert_eq!(
        highlighted(shell, cx).as_ref(),
        Some(&order[0]),
        "no wrap up"
    );
    for (at, key) in order.iter().enumerate().skip(1) {
        shell.keys("down", cx);
        assert_eq!(highlighted(shell, cx).as_ref(), Some(key), "down to {at}");
    }
    shell.keys("down", cx);
    assert_eq!(
        highlighted(shell, cx).as_ref(),
        order.last(),
        "no wrap down"
    );
    shell.keys("home", cx);
    assert_eq!(highlighted(shell, cx).as_ref(), Some(&order[0]), "home");
    shell.keys("end", cx);
    assert_eq!(highlighted(shell, cx).as_ref(), order.last(), "end");

    // Enter on a row with no button and no item, and on a disabled button:
    // nothing is sent, opened or chosen.
    let tabs = |cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
    };
    let requests = shell.requests(cx, |_| true).len();
    let before = (pressed(shell, cx), tabs(cx));
    for quiet in [&rows.plain, &rows.disabled] {
        arrow_to(shell, quiet, cx);
        shell.keys("enter", cx);
        assert_eq!(shell.requests(cx, |_| true).len(), requests, "{quiet:?}");
        assert_eq!((pressed(shell, cx), tabs(cx)), before, "{quiet:?}");
    }

    // With an item and no button, Enter opens the item.
    arrow_to(shell, &rows.item, cx);
    shell.keys("enter", cx);
    let opened = tabs(cx);
    assert!(
        opened
            .iter()
            .any(|tab| matches!(tab, TabKey::Item { id, .. } if id == TICKET)),
        "the item's tab opened: {opened:?}"
    );

    // `shift-f10` and `menu` open the highlighted row's menu (the menu's
    // items are P17's).
    for menu_key in ["shift-f10", "menu"] {
        shell.keys("secondary-4", cx);
        arrow_to(shell, &rows.item, cx);
        shell.keys(menu_key, cx);
        assert_eq!(
            menu_key_of(shell, cx),
            Some(rows.item.clone()),
            "{menu_key}"
        );
        shell.keys("escape", cx);
        assert_eq!(menu_key_of(shell, cx), None);
    }

    // With an enabled button, Enter presses it, once.
    shell.keys("secondary-4", cx);
    arrow_to(shell, &rows.pending[0], cx);
    assert_eq!(resumes(shell, OP_A, cx), 0);
    let before = pressed(shell, cx);
    shell.keys("enter", cx);
    assert_eq!(resumes(shell, OP_A, cx), 1, "Enter pressed Resume");
    assert_eq!(pressed(shell, cx), before + 1);

    // Tab moves from the list to the highlighted row's button; Enter there
    // runs its action once, and the list's Enter does not also fire.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    shell.keys("secondary-4", cx);
    arrow_to(shell, &rows.pending[1], cx);
    shell.keys("tab", cx);
    let button = button_focus(shell, cx);
    assert!(
        shell.frame(cx, |window, _| button.is_focused(window)),
        "Tab reaches the highlighted row's button"
    );
    let before = pressed(shell, cx);
    shell.press("enter", cx);
    assert_eq!(pressed(shell, cx), before + 1, "pressed once");
    assert_eq!(resumes(shell, OP_B, cx), 1, "Resume sent once");

    // The highlighted row leaves while its button has focus: the row that
    // takes its place has a button too, and focus goes to the list.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    shell.keys("secondary-4", cx);
    arrow_to(shell, &rows.pending[2], cx);
    shell.keys("tab", cx);
    let button = button_focus(shell, cx);
    let list = list_focus(shell, cx);
    assert!(shell.frame(cx, |window, _| button.is_focused(window)));
    relist(shell, &[OP_A, OP_B, OP_D], cx);
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    assert!(
        shell.frame(cx, |window, _| list.is_focused(window)
            && !button.is_focused(window)),
        "focus moved from the button to the list"
    );
    assert!(
        highlighted(shell, cx).is_some(),
        "the highlight is on a row now"
    );

    // T8 (R-55): a row drawn before the log's took a newer revision runs
    // nothing on Enter, though the choice is built and refused. The log's
    // row went A, B, A in one update: it offers what the pane drew, so only
    // its revision refuses the choice.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    shell.keys("secondary-4", cx);
    arrow_to(shell, &rows.pending[3], cx);
    let (before, requests) = (pressed(shell, cx), shell.requests(cx, |_| true).len());
    let operations = shell.operations(cx);
    shell.frame(cx, |window, cx| {
        let (drawn, current) = change_row(shell, &operations, OP_D, cx);
        assert!(drawn.is_some() && current.is_some(), "the row is in both");
        assert_ne!(drawn, current, "the log's row took a newer revision");
        press_in(window, "enter", cx);
    });
    shell.run(cx);
    assert_eq!(pressed(shell, cx), before + 1, "the drawn choice was built");
    assert_eq!(
        shell.requests(cx, |_| true).len(),
        requests,
        "and refused: nothing ran"
    );
    assert_eq!(resumes(shell, OP_D, cx), 0);

    // T8 on a row with no button: Enter opens its item, unless the log's row
    // took a newer revision since it was drawn (it was retried, A, B, A,
    // and finished as it had). The first Enter opens nothing; once the pane
    // has drawn the row again, Enter opens the item.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    shell.keys("secondary-4", cx);
    arrow_to(shell, &rows.item, cx);
    let RowKey::Request(item) = rows.item.clone() else {
        panic!("a request's row");
    };
    let opened = |cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, _| {
            workspace
                .tab_keys()
                .iter()
                .any(|tab| matches!(tab, TabKey::Item { id, .. } if id == TICKET))
        })
    };
    assert!(!opened(cx), "no tab yet");
    let operations = shell.operations(cx);
    shell.frame(cx, |window, cx| {
        let (drawn, current) = retry_request(shell, &operations, item, cx);
        assert!(drawn.is_some() && current.is_some(), "the row is in both");
        assert_ne!(drawn, current, "the log's row took a newer revision");
        press_in(window, "enter", cx);
    });
    shell.run(cx);
    assert!(!opened(cx), "the stale Enter opened nothing");
    settle(shell, cx);
    shell.keys("enter", cx);
    assert!(opened(cx), "Enter on the row as it now stands opens it");
}

/// P16: a left click on a row highlights it, focuses the list and sends
/// nothing; a left click on its button runs the button once and highlights
/// its row; a disabled button does nothing; a click on a button drawn
/// before the log's row took a newer revision runs nothing (T8); a right
/// click highlights its row and opens its menu.
#[gpui_kit::test]
fn rows_by_pointer(cx: &mut TestAppContext) {
    // A Resume the test presses runs on the worker's thread.
    cx.executor().allow_parking();
    let rows = rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let list = list_focus(shell, cx);
    let [a, ..] = rows.pending.clone();

    // A click on the row, away from its button: highlight and focus only.
    let at = index_of(shell, &rows.plain, cx);
    assert_eq!(highlighted(shell, cx), None);
    let before = (pressed(shell, cx), shell.requests(cx, |_| true).len());
    let focused = shell.frame(cx, |window, cx| {
        window.click_at(("activity-row", at), point(px(10.), px(10.)), cx);
        list.is_focused(window)
    });
    shell.run(cx);
    assert!(focused, "the list took focus");
    assert_eq!(highlighted(shell, cx).as_ref(), Some(&rows.plain));
    assert_eq!(
        (pressed(shell, cx), shell.requests(cx, |_| true).len()),
        before,
        "a click on a row sends nothing"
    );

    // A disabled button takes the click and does nothing.
    let at = index_of(shell, &rows.disabled, cx);
    let before = (pressed(shell, cx), shell.requests(cx, |_| true).len());
    shell.click(("activity-button", at), cx);
    assert_eq!(
        (pressed(shell, cx), shell.requests(cx, |_| true).len()),
        before
    );

    // A right click highlights its row and opens its menu.
    let at = index_of(shell, &rows.plain, cx);
    shell.frame(cx, |window, cx| {
        window.right_click(("activity-row", at), cx)
    });
    shell.run(cx);
    assert_eq!(menu_key_of(shell, cx), Some(rows.plain.clone()));
    assert_eq!(highlighted(shell, cx).as_ref(), Some(&rows.plain));
    shell.keys("escape", cx);

    // A click on a button runs it once, and its row is highlighted.
    let at = index_of(shell, &a, cx);
    let before = pressed(shell, cx);
    let highlight = shell.frame(cx, |window, cx| {
        window.click(("activity-button", at), cx);
        shell.workspace.read_with(cx, |workspace, _| {
            workspace.activity_pane().highlighted_key().cloned()
        })
    });
    shell.run(cx);
    assert_eq!(resumes(shell, OP_A, cx), 1, "Resume sent once");
    assert_eq!(pressed(shell, cx), before + 1);
    assert_eq!(highlight.as_ref(), Some(&a), "its row was highlighted");

    // T8 (R-55): the log's row took a newer revision after the pane drew it
    // (A, B, A in one update: it offers what the pane drew). The click is the
    // drawn button's: built, refused by its revision, nothing sent.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let at = index_of(shell, &rows.pending[3], cx);
    let (before, requests) = (pressed(shell, cx), shell.requests(cx, |_| true).len());
    let operations = shell.operations(cx);
    shell.frame(cx, |window, cx| {
        let (drawn, current) = change_row(shell, &operations, OP_D, cx);
        assert!(drawn.is_some() && current.is_some(), "the row is in both");
        assert_ne!(drawn, current, "the log's row took a newer revision");
        window.click(("activity-button", at), cx);
    });
    shell.run(cx);
    assert_eq!(pressed(shell, cx), before + 1, "the drawn choice was built");
    assert_eq!(
        shell.requests(cx, |_| true).len(),
        requests,
        "and refused: nothing ran"
    );
    assert_eq!(resumes(shell, OP_D, cx), 0);
}
