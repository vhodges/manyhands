//! A row's menu (Wave 04 Cycle 03b, Part D): `the_row_menu_by_pointer_and_
//! keyboard` (P17; the menu keys of P15 and the right click of P16 are
//! clauses of it). It opens by a right press and by `shift-f10` and `menu`;
//! its first items are the full message and the detail lines, disabled, then
//! the row's offers; its choice carries the revision the row had when the
//! menu opened, and runs only while the log's row still offers it (T8,
//! T11); it is dropped under a modal and when its row leaves, and it is the
//! only layer open (R-13).

use gpui_kit::{TestAppContext, component::WindowExt, test::TestWindowExt};

use super::{cases::TICKET, list_support::*};
use crate::desktop::{
    keys::{ActivityMenuChoice, OpenActivityMenu},
    model::{
        activity::{
            copy::message_of,
            rows::{ActivityRow, RowKey},
        },
        navigation::OpenTarget,
    },
    test_support::fixtures::item_id,
    view::{
        activity_support::{highlighted, index_of, list_focus, list_focused, refresh, row, to_row},
        footer::{
            Control,
            tests::support::{modal_open, open_modal},
        },
        harness::{Shell, key},
        repositories::tests::support::{
            choose, choose_and_hold, labels, menu_open as repository_menu_open, press_and_hold,
            right_click as repository_right_click,
        },
    },
};

/// The open row menu's items in the last frame, in order, by label; a
/// separator has none. (The Kit reports no disabled flag for a menu item:
/// a disabled item is read by choosing it.)
fn items(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    labels(shell, cx)
}

fn menu_key(shell: &Shell, cx: &mut TestAppContext) -> Option<RowKey> {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_menu_key())
}

fn menu_open(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.activity_menu_open())
}

/// A right click on the drawn row `key`.
fn right_click(shell: &Shell, key: &RowKey, cx: &mut TestAppContext) {
    let at = index_of(shell, key, cx);
    shell.frame(cx, |window, cx| {
        window.right_click(("activity-row", at), cx)
    });
    shell.run(cx);
}

/// What the menu of `row` lists: its message, its detail lines, then its
/// offers.
fn expected(row: &ActivityRow) -> Vec<String> {
    std::iter::once(message_of(&row.row))
        .chain(row.row.detail.iter().cloned())
        .chain(row.row.menu.iter().map(|offer| offer.label.clone()))
        .collect()
}

/// P17: the menu by pointer and by keyboard; a choice whose row changed in
/// between runs nothing, and the menu closes (T8, T11). With P15's menu keys
/// and P16's right click: the right click highlights its row and opens its
/// menu at the pointer, the choice and Escape send nothing, and `shift-f10`
/// and `menu` open the highlighted row's below it.
#[gpui_kit::test]
async fn the_row_menu_by_pointer_and_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let rows = rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let [a, ..] = rows.pending.clone();
    let list = list_focus(shell, cx);
    let sent = |cx: &mut TestAppContext| shell.requests(cx, |_| true).len();

    // A right click: the row is highlighted and its menu opens at the
    // pointer, with focus elsewhere before it, so Escape's restore shows.
    let settings = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.footer_focus(Control::Settings));
    shell.frame(cx, |window, cx| settings.focus(window, cx));
    let (requests, pressed) = (sent(cx), self::pressed(shell, cx));
    right_click(shell, &a, cx);
    assert_eq!(menu_key(shell, cx).as_ref(), Some(&a), "a's menu");
    assert_eq!(highlighted(shell, cx).as_ref(), Some(&a), "highlighted");
    let log_row = row(shell, &a, cx).expect("its row");
    assert_eq!(items(shell, cx), expected(&log_row));
    // The message and the detail lines are disabled: choosing each sends
    // nothing and the menu stays.
    for line in &items(shell, cx)[..=log_row.row.detail.len()] {
        choose(shell, line, cx);
        assert!(menu_open(shell, cx), "{line} is read only");
    }
    assert_eq!((sent(cx), pressed), (requests, self::pressed(shell, cx)));
    shell.keys("down", cx);
    shell.keys("escape", cx);
    assert!(
        shell.frame(cx, |window, _| settings.is_focused(window)),
        "Escape restores focus to where it was"
    );
    assert!(!menu_open(shell, cx));
    assert_eq!((sent(cx), pressed), (requests, self::pressed(shell, cx)));

    // By keyboard: `shift-f10` below the highlighted row, Escape back to
    // the list, `menu` again, then a choice by click: Resume is sent once
    // and focus is back on the list.
    to_row(shell, &a, cx);
    shell.keys("shift-f10", cx);
    assert_eq!(menu_key(shell, cx).as_ref(), Some(&a), "shift-f10");
    let anchored = shell.workspace.read_with(cx, |workspace, _| {
        (
            workspace.activity_menu_at(),
            workspace
                .activity_pane()
                .row_bounds(&a)
                .map(|bounds| bounds.bottom_left()),
        )
    });
    assert!(anchored.1.is_some(), "a's row was drawn");
    assert_eq!(anchored.0, anchored.1, "below a's row");
    assert!(
        press_and_hold(shell, "escape", &list, cx),
        "Escape: the list"
    );
    assert!(!menu_open(shell, cx));
    shell.keys("menu", cx);
    assert_eq!(menu_key(shell, cx).as_ref(), Some(&a), "menu");
    assert!(
        choose_and_hold(shell, "Resume", &list, cx),
        "focus is back on the list"
    );
    assert!(!menu_open(shell, cx), "a choice closes it");
    assert_eq!(resumes(shell, OP_A, cx), 1, "Resume was sent once");

    // A choice whose row changed in between: the menu was opened on the row
    // as it stood; it changed (its Resume went); the item chosen runs
    // nothing, and the menu closes. A dispatched choice of an offer the row
    // no longer has does the same.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let d = rows.pending[3].clone();
    right_click(shell, &d, cx);
    assert!(menu_open(shell, cx));
    let operations = shell.operations(cx);
    shell.frame(cx, |_, cx| {
        let (drawn, current) = change_row(shell, &operations, OP_D, cx);
        assert_ne!(drawn, current, "the log's row took a newer revision");
    });
    shell.run(cx);
    refresh(shell, cx);
    let requests = shell.requests(cx, |_| true).len();
    choose(shell, "Resume", cx);
    assert!(!menu_open(shell, cx), "it closed");
    assert_eq!(shell.requests(cx, |_| true).len(), requests, "nothing ran");
    assert_eq!(resumes(shell, OP_D, cx), 0);

    // Behind a dialog, and behind the details sheet: a menu that is open
    // is dropped, and the keys, a right press and a dispatched choice do
    // nothing (R-60).
    for sheet in [false, true] {
        let rows = self::rows(cx);
        let shell = &rows.shell;
        let a = rows.pending[0].clone();
        settle(shell, cx);
        let target = OpenTarget::Item {
            repo: key(&rows.root),
            kind: manyhands::repository::ItemDtoKind::Ticket,
            id: item_id(TICKET),
        };
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.open_item(target, cx));
        shell.run(cx);
        right_click(shell, &a, cx);
        assert!(menu_open(shell, cx));
        open_modal(shell, sheet, cx);
        assert!(!menu_open(shell, cx), "sheet {sheet}: it closed");
        let requests = shell.requests(cx, |_| true).len();
        let revision = row(shell, &a, cx).expect("its row").revision;
        let resume = ActivityMenuChoice {
            key: a.clone(),
            action: row(shell, &a, cx).unwrap().row.button.unwrap().action,
            shown: "Resume".to_owned(),
            revision,
        };
        shell.frame(cx, |window, cx| {
            window.dispatch_action(Box::new(OpenActivityMenu), cx);
            window.dispatch_action(Box::new(resume), cx);
        });
        shell.run(cx);
        let at = index_of(shell, &a, cx);
        shell.frame(cx, |window, cx| {
            window.right_click(("activity-row", at), cx)
        });
        shell.run(cx);
        assert!(!menu_open(shell, cx), "sheet {sheet}: none opens");
        assert_eq!(
            shell.requests(cx, |_| true).len(),
            requests,
            "sheet {sheet}"
        );
        assert!(modal_open(shell, cx), "sheet {sheet}: the modal stays");
        shell.frame(cx, |window, cx| {
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
    }

    // One layer at a time (R-13): the repository menu closes the row menu,
    // and the row menu closes the repository menu.
    let rows = self::rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);
    let a = rows.pending[0].clone();
    right_click(shell, &a, cx);
    assert!(menu_open(shell, cx));
    repository_right_click(shell, "a", cx);
    assert!(repository_menu_open(shell, cx), "the repository's menu");
    assert!(!menu_open(shell, cx), "it closed the row menu");
    right_click(shell, &a, cx);
    assert!(menu_open(shell, cx));
    assert!(
        !repository_menu_open(shell, cx),
        "and the row menu closed it"
    );

    // Its row leaves the log: the menu goes, and focus is on the list.
    relist(shell, &[OP_B, OP_C, OP_D], cx);
    for _ in 0..2 {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
    }
    assert!(!menu_open(shell, cx), "its row left");
    assert!(list_focused(shell, cx), "focus is on the list");
}
