//! The popover's keys and its stale rows (P23, T8, R-55): Home, End, Tab's
//! three stops and Escape; and a press on a row drawn before the log's took
//! a newer revision, or whose offer changed.

use gpui_kit::{TestAppContext, test::TestWindowExt};

use super::*;
use crate::desktop::{
    model::activity::rows::RowKey,
    view::{
        activity::tests::{
            list_support::{OP_A, OP_B, OP_D, change_row, press_in, resumes, save},
            pending_cases::{PENDING_ROOT, listing},
        },
        harness::Shell,
    },
};

/// The popover's keys: Home and End move the highlight and it stops at the
/// ends; Tab and Shift-Tab cycle the list, the highlighted row's button and
/// the link, wrapping, and never leave the popover; Escape from any of the
/// three returns to the footer item; Enter on the button runs it once.
pub(super) fn keys(cx: &mut TestAppContext) {
    let world = rows(cx);
    let shell = &world.shell;
    settle(shell, cx);
    let order = keys_of(&attention_rows(shell, cx));
    open_by_click(shell, cx);
    assert_eq!(cursor(shell, cx).as_ref(), order.first());
    shell.keys("up", cx);
    assert_eq!(cursor(shell, cx).as_ref(), order.first(), "no wrap above");
    shell.keys("end", cx);
    assert_eq!(cursor(shell, cx).as_ref(), order.last());
    shell.keys("down", cx);
    assert_eq!(cursor(shell, cx).as_ref(), order.last(), "no wrap below");
    shell.keys("home", cx);
    assert_eq!(cursor(shell, cx).as_ref(), order.first());
    shell.keys("down", cx);
    shell.keys("up", cx);
    assert_eq!(cursor(shell, cx).as_ref(), order.first());

    // Three stops, wrapping, both ways.
    let forward = [Spot::Button, Spot::Link, Spot::List, Spot::Button];
    for expected in forward {
        shell.keys("tab", cx);
        assert_eq!(spot(shell, cx), expected, "Tab");
    }
    shell.keys("shift-tab", cx);
    assert_eq!(spot(shell, cx), Spot::List, "Shift-Tab");
    shell.keys("shift-tab", cx);
    assert_eq!(spot(shell, cx), Spot::Link, "Shift-Tab wraps");
    shell.keys("shift-tab", cx);
    assert_eq!(spot(shell, cx), Spot::Button);
    assert!(is_open(shell, cx), "Tab never left the popover");

    // Escape from each stop.
    for stop in 0..3 {
        shell.keys("escape", cx);
        assert!(!is_open(shell, cx), "Escape closes it");
        assert_eq!(spot(shell, cx), Spot::Item, "focus is the item's");
        open_by_click(shell, cx);
        for _ in 0..=stop {
            shell.keys("tab", cx);
        }
        assert!(is_open(shell, cx));
    }
    shell.keys("escape", cx);

    // Enter on the focused button runs it once: Tab, then Enter.
    open_by_click(shell, cx);
    let first = cursor(shell, cx).expect("a highlighted row");
    shell.keys("tab", cx);
    assert_eq!(spot(shell, cx), Spot::Button);
    let before = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_pressed());
    shell.press("enter", cx);
    let after = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_pressed());
    assert_eq!(after, before + 1, "the button's Enter ran once");
    assert!(!is_open(shell, cx));
    let RowKey::Pending { operation, .. } = first else {
        panic!("the first row is a pending one");
    };
    assert_eq!(resumes(shell, &operation, cx), 1, "Resume was sent");
}

/// T8, R-55: the popover's Enter and click on a row drawn before the log's
/// row took a newer revision, or whose offer changed, run nothing and leave
/// the popover open with the row refreshed. The log's row went A, B, A in
/// one update, so it offers what the popover drew: only its revision refuses
/// the choice.
pub(super) fn stale_rows(cx: &mut TestAppContext) {
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
