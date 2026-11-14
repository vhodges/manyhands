//! The popover's keys and its stale rows (P23, T8, R-55): Home, End, Tab's
//! three stops and Escape; and a press on a row drawn before the log's took
//! a newer revision, or whose offer changed.

use gpui_kit::{TestAppContext, test::TestWindowExt};

use super::*;
use crate::desktop::{
    model::activity::rows::RowKey,
    view::{
        activity::tests::{
            list_support::{OP_A, OP_B, OP_D, Rows, change_row, press_in, resumes, save},
            pending_cases::{PENDING_ROOT, listing},
        },
        harness::Shell,
    },
};

/// The popover's keys: Home and End move the highlight and it stops at the
/// ends; Tab and Shift-Tab cycle the list, the highlighted row's button and
/// the link, wrapping, and never leave the popover; Escape from any of the
/// three returns to the footer item; Enter on the button runs it once.
#[gpui_kit::test]
fn popover_keys(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
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
/// row took a newer revision run nothing and leave the popover open with the
/// row refreshed. The log's row went A, B, A in one update, so it offers what
/// the popover drew: only its revision refuses the choice. A row that left the
/// log, one whose Resume became disabled, and a choice the row does not offer
/// (right revision, other label: `still_offered`'s offer branch) are refused
/// too, each through a press that arrived (`attention_pressed` counts it).
#[gpui_kit::test]
fn popover_stale_rows(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
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
        let (before, requests) = (pressed(shell, cx), shell.requests(cx, |_| true).len());
        let operations = shell.operations(cx);
        shell.frame(cx, |window, cx| {
            // The popover's own row, before and after the log's changed.
            let revision = |cx: &mut gpui_kit::App| {
                shell.workspace.read_with(cx, |workspace, _| {
                    workspace
                        .attention_popover_rows()
                        .into_iter()
                        .find(|row| row.row.key == stale)
                        .map(|row| row.revision)
                })
            };
            let drawn = revision(cx);
            let (_, current) = change_row(shell, &operations, OP_D, cx);
            assert!(drawn.is_some() && current.is_some(), "the row is in both");
            assert_ne!(drawn, current, "the log's row took a newer revision");
            assert_eq!(revision(cx), drawn, "the popover still holds what it drew");
            if by_click {
                window.click(("needs-attention-button", at), cx);
            } else {
                press_in(window, "enter", cx);
            }
        });
        shell.run(cx);
        assert_eq!(pressed(shell, cx), before + 1, "the drawn choice was built");
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
    row_gone(cx);
    resume_disabled(cx);
    other_offer(cx);
}

/// How many choices the popover built.
fn pressed(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.attention_pressed())
}

/// The highlight is on the pending row D in a popover that is open; the
/// window's other rows are as `rows` made them.
fn on_d(world: &Rows, cx: &mut TestAppContext) -> RowKey {
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
    assert_eq!(cursor(shell, cx).as_ref(), Some(&stale));
    stale
}

/// The row left the log (its Resume went with it): the press arrives, runs
/// nothing, and the popover stays open without the row.
fn row_gone(cx: &mut TestAppContext) {
    let world = rows(cx);
    let shell = &world.shell;
    let stale = on_d(&world, cx);
    let (before, requests) = (pressed(shell, cx), shell.requests(cx, |_| true).len());
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
    assert_eq!(pressed(shell, cx), before + 1, "the press arrived");
    assert_eq!(shell.requests(cx, |_| true).len(), requests, "nothing ran");
    assert_eq!(resumes(shell, OP_D, cx), 0);
    settle_popover(shell, cx);
    assert!(is_open(shell, cx), "the popover stays open");
    assert!(
        !keys_of(&applied(shell, cx)).contains(&stale),
        "with the row gone"
    );
}

/// The row stays, but its Resume is disabled now (the operation is listed as
/// a remote synchronization, which Resume is not offered on): the press
/// arrives and runs nothing; the popover stays open, and its refreshed row
/// shows the button disabled, which Enter then does nothing on.
fn resume_disabled(cx: &mut TestAppContext) {
    let world = rows(cx);
    let shell = &world.shell;
    let stale = on_d(&world, cx);
    let (before, requests) = (pressed(shell, cx), shell.requests(cx, |_| true).len());
    let operations = shell.operations(cx);
    shell.frame(cx, |window, cx| {
        operations.update(cx, |operations, cx| {
            let stamp = operations.load_pending(PENDING_ROOT.to_owned());
            let mut remote = save(OP_D);
            remote.family = manyhands::repository::OperationFamily::Remote;
            assert!(operations.apply_pending(stamp, listing(vec![remote])));
            cx.notify();
        });
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.refresh_activity_now(cx));
        let log = shell.workspace.read_with(cx, |workspace, _| {
            workspace
                .attention_rows()
                .into_iter()
                .find(|row| row.row.key == stale)
        });
        assert!(
            log.is_some_and(|row| row.row.button.is_some_and(|button| !button.enabled)),
            "the log's row stays, its button disabled"
        );
        press_in(window, "enter", cx);
    });
    shell.run(cx);
    assert_eq!(pressed(shell, cx), before + 1, "the press arrived");
    assert_eq!(shell.requests(cx, |_| true).len(), requests, "nothing ran");
    assert_eq!(resumes(shell, OP_D, cx), 0);
    settle_popover(shell, cx);
    assert!(is_open(shell, cx), "the popover stays open");
    let refreshed = applied(shell, cx)
        .into_iter()
        .find(|row| row.row.key == stale)
        .expect("the row stays");
    assert!(
        refreshed.row.button.is_some_and(|button| !button.enabled),
        "refreshed: its button is disabled"
    );
    shell.press("enter", cx);
    shell.run(cx);
    assert_eq!(
        pressed(shell, cx),
        before + 1,
        "Enter on a disabled button is no press"
    );
    assert!(is_open(shell, cx), "and the popover stays open");
}

/// The log's row is as drawn (same revision) and offers its Resume, but the
/// choice names another label: `still_offered`'s offer branch refuses it,
/// the press arrives, nothing runs and the popover stays open. The same
/// choice with the label the row offers runs.
fn other_offer(cx: &mut TestAppContext) {
    use crate::desktop::view::activity::row::choice_of;
    let world = rows(cx);
    let shell = &world.shell;
    let stale = on_d(&world, cx);
    let (before, requests) = (pressed(shell, cx), shell.requests(cx, |_| true).len());
    let choose = |label: Option<&str>, shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, cx| {
            shell.workspace.update(cx, |workspace, cx| {
                let row = workspace
                    .attention_popover_rows()
                    .into_iter()
                    .find(|row| row.row.key == stale)
                    .expect("the row is drawn");
                let log = workspace
                    .activity()
                    .rows()
                    .iter()
                    .find(|log| log.row.key == stale);
                assert_eq!(
                    log.map(|log| log.revision),
                    Some(row.revision),
                    "the row is as drawn: only the offer can refuse"
                );
                let offer = row.row.button.as_ref().expect("a button");
                let mut choice = choice_of(&row, offer);
                if let Some(label) = label {
                    choice.shown = label.to_owned();
                }
                workspace.press_attention(choice, window, cx);
            })
        });
        shell.run(cx);
    };
    choose(Some("Resume now"), shell, cx);
    assert_eq!(pressed(shell, cx), before + 1, "the press arrived");
    assert_eq!(shell.requests(cx, |_| true).len(), requests, "nothing ran");
    assert_eq!(resumes(shell, OP_D, cx), 0);
    assert!(is_open(shell, cx), "the popover stays open");
    choose(None, shell, cx);
    assert_eq!(resumes(shell, OP_D, cx), 1, "the offered label runs");
    assert!(!is_open(shell, cx), "and closes the popover");
}
