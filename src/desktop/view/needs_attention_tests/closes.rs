//! P25: each way the popover closes, and where focus goes. One window for
//! each case: they change the model and the layers, and a case's end must
//! not be another's start.

use gpui_kit::{Point, TestAppContext, test::TestWindowExt};
use manyhands::results::{Outcome, ResultCode};

use super::*;
use crate::desktop::{
    model::{
        activity::rows::RowKey,
        layout::PanelKind::{Documents, Tickets},
    },
    services::requests::{DesktopRequestId, Intent},
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity::tests::{
            cases::{TICKET, answered, fabricate},
            list_support::{OP_A, OP_B, OP_C, OP_D, pending_key, relist, save},
            pending_cases::{PENDING_ROOT, listing},
        },
        footer::{
            menu::Placement,
            tests::support::{finish, focused, modal_open, open_modal, shell_with_a_ticket},
        },
        harness::Shell,
        workspace::Region,
    },
};

/// A window with one failed request: the one attention row.
fn one_row(cx: &mut TestAppContext) -> Shell {
    let shell = Shell::open(Fixture::new(), cx);
    finish(&shell, 1, Outcome::Error, cx);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell
}

/// The failed request `id` is dismissed, as its Dismiss does.
fn dismiss(shell: &Shell, id: u64, cx: &mut TestAppContext) {
    shell.operations(cx).update(cx, |operations, cx| {
        operations.dismiss(DesktopRequestId(id));
        cx.notify();
    });
    shell.run(cx);
    settle_popover(shell, cx);
}

/// Where E1 sends focus when Needs attention hides: the right dock group's
/// first control, Tickets' toggle in the default layout.
fn on_e1(shell: &Shell, cx: &mut TestAppContext) -> bool {
    focused(shell, Control::Panel(Tickets).id(), cx)
}

/// P25: the count reaching 0, a dialog or sheet, another layer, a press
/// outside, a full click on the item, a dialog from the popover that closes
/// after the item hid, a dialog or menu opened while the popover had focus,
/// and the highlighted row leaving while its button holds focus.
#[gpui_kit::test]
fn the_popover_closes(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    at_zero(cx);
    under_a_modal(cx);
    another_layer(cx);
    a_press_outside(cx);
    a_full_click_on_the_item(cx);
    a_dialog_from_the_popover(cx);
    a_layer_opened_while_it_had_focus(cx);
    the_highlighted_row_leaves(cx);
    the_row_at_the_position_has_no_button(cx);
}

/// The count reaches 0 while the popover is open and has focus: it closes,
/// and focus moves by E1's rule, to the right dock group's first control.
fn at_zero(cx: &mut TestAppContext) {
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    assert!(is_open(&shell, cx) && drawn(&shell, cx));
    assert_eq!(spot(&shell, cx), Spot::List);
    dismiss(&shell, 1, cx);
    assert!(!is_open(&shell, cx), "it closed at 0");
    assert!(!drawn(&shell, cx));
    assert!(
        !shell.present(Control::Attention.id(), cx),
        "the item hid at 0"
    );
    assert!(on_e1(&shell, cx), "focus moved right, to E1's control");
}

/// A dialog or the sheet opening closes it, as it closes the menus.
fn under_a_modal(cx: &mut TestAppContext) {
    for sheet in [false, true] {
        let shell = shell_with_a_ticket(cx);
        finish(&shell, 1, Outcome::Error, cx);
        open_by_click(&shell, cx);
        assert!(is_open(&shell, cx), "open before the modal (sheet {sheet})");
        open_modal(&shell, sheet, cx);
        shell.frame(cx, |_, _| ());
        assert!(!is_open(&shell, cx), "a modal closes it (sheet {sheet})");
        assert!(!drawn(&shell, cx));
        assert!(modal_open(&shell, cx), "the modal stays");
    }
}

/// Another layer opening closes it (R-13; the pairs are in the layers test).
fn another_layer(cx: &mut TestAppContext) {
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    shell.frame(cx, |window, cx| {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.open_dock_menu(Documents, Placement::Pointer(Point::default()), window, cx)
        })
    });
    shell.run(cx);
    assert!(!is_open(&shell, cx), "a dock menu closes it");
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.dock_menu_open()),
        "and is the one open"
    );
}

/// A press outside closes it and still reaches what is under it. It keeps the
/// focus it gave; one that gave none sends focus to the footer item (R-11).
fn a_press_outside(cx: &mut TestAppContext) {
    // A press that gives focus: the item area takes it.
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    shell.click(Region::Content.id(), cx);
    assert!(!is_open(&shell, cx), "a press outside closes it");
    assert!(!drawn(&shell, cx));
    assert!(
        focused(&shell, Region::Content.id(), cx),
        "the press kept the focus it gave"
    );

    // A press that gives none: the title bar's title.
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    shell.click("title-bar-title", cx);
    assert!(!is_open(&shell, cx), "a press outside closes it");
    assert_eq!(spot(&shell, cx), Spot::Item, "else focus is the item's");

    // It still reaches what is under it: a press on a footer toggle closes
    // the popover and toggles the panel.
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    let toggle = Control::Panel(Documents);
    let shown = |shell: &Shell, cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, _| {
            workspace.layout().is_displayed(Documents)
        })
    };
    assert!(shown(&shell, cx));
    shell.click(toggle.id(), cx);
    assert!(!is_open(&shell, cx));
    assert!(!shown(&shell, cx), "and the press acted");
}

/// A full click on the item while the popover is open leaves it closed: the
/// press outside runs first, on mouse down, and ignores a press inside the
/// item's bounds, so the click that follows, on mouse up, closes it once
/// (fact 47).
fn a_full_click_on_the_item(cx: &mut TestAppContext) {
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    assert!(is_open(&shell, cx));
    shell.click(Control::Attention.id(), cx);
    assert!(
        !is_open(&shell, cx),
        "a full click on the item leaves it closed"
    );
    assert!(!drawn(&shell, cx));
    shell.frame(cx, |_, _| ());
    assert!(!is_open(&shell, cx), "and it stays closed");
}

/// A window whose one attention row is a stopped close, whose button, Open…,
/// opens the lifecycle dialog; and a second failed request that keeps the
/// count above 0 when `keep` is set.
fn closing_window(keep: bool, cx: &mut TestAppContext) -> (Shell, DesktopRequestId) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Ticket", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let close = fabricate(
        &shell,
        1,
        Intent::TicketClose {
            root,
            id: item_id(TICKET),
        },
        answered("ticket close", ResultCode::PushRejected),
        cx,
    );
    if keep {
        finish(&shell, 2, Outcome::Error, cx);
    }
    (shell, close)
}

/// A dialog a popover action opens returns focus to the footer item; when
/// the item has hidden by the time it closes, focus lands on E1's control
/// (R-49).
fn a_dialog_from_the_popover(cx: &mut TestAppContext) {
    for keep in [true, false] {
        let (shell, close) = closing_window(keep, cx);
        open_by_click(&shell, cx);
        let first = cursor(&shell, cx).expect("a row is highlighted");
        if keep {
            // The close is the older row: move to it.
            shell.keys("end", cx);
        }
        assert_eq!(
            cursor(&shell, cx),
            Some(RowKey::Request(close)),
            "the stopped close is highlighted ({first:?})"
        );
        shell.keys("enter", cx);
        shell.frame(cx, |_, _| ());
        assert!(!is_open(&shell, cx), "the action closed the popover");
        assert!(
            modal_open(&shell, cx),
            "Open… opened the dialog (keep {keep})"
        );
        if !keep {
            dismiss(&shell, close.0, cx);
        }
        shell.keys("escape", cx);
        shell.frame(cx, |_, _| ());
        assert!(!modal_open(&shell, cx), "the dialog closed");
        if keep {
            assert_eq!(spot(&shell, cx), Spot::Item, "focus is on the item");
        } else {
            assert!(
                !shell.present(Control::Attention.id(), cx),
                "the item hid while the dialog was open"
            );
            assert!(on_e1(&shell, cx), "focus lands on E1's control");
        }
    }
}

/// A dialog or menu opened while the popover had focus, then closed: focus
/// lands on the footer item, or on E1's control when it has hidden (R-50,
/// U-8). A credential prompt takes the focused handle, the popover's list,
/// as its origin; so does a dock menu opened by a right press.
fn a_layer_opened_while_it_had_focus(cx: &mut TestAppContext) {
    // A prompt, answered by Escape.
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    assert_eq!(spot(&shell, cx), Spot::List);
    let _answer = shell.prompt(cx);
    assert!(modal_open(&shell, cx), "the prompt is open");
    assert!(!is_open(&shell, cx), "and the popover is gone");
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert!(!modal_open(&shell, cx));
    assert_eq!(spot(&shell, cx), Spot::Item, "focus is on the item");

    // The same, the item hidden while the prompt was open.
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    let _answer = shell.prompt(cx);
    dismiss(&shell, 1, cx);
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert!(!modal_open(&shell, cx));
    assert!(on_e1(&shell, cx), "focus lands on E1's control");

    // A dock menu, opened by a right press on a panel toggle, closed by
    // Escape.
    let shell = one_row(cx);
    open_by_click(&shell, cx);
    shell.frame(cx, |window, cx| {
        window.right_click(Control::Panel(Documents).id(), cx)
    });
    shell.run(cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.dock_menu_open()),
        "the menu opened"
    );
    assert!(!is_open(&shell, cx), "and the popover closed");
    shell.keys("escape", cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(spot(&shell, cx), Spot::Item, "focus is on the item");
}

/// The highlighted row leaves the log while its button holds focus: the
/// popover stays open, focus moves to its list, and Escape and the arrows
/// still work (E2, P25).
fn the_highlighted_row_leaves(cx: &mut TestAppContext) {
    let world = rows(cx);
    let shell = &world.shell;
    settle(shell, cx);
    open_by_click(shell, cx);
    let first = cursor(shell, cx).expect("the first row is highlighted");
    shell.keys("tab", cx);
    assert_eq!(spot(shell, cx), Spot::Button, "Tab reaches the button");
    let leaving = world
        .pending
        .iter()
        .position(|key| key == &first)
        .expect("the first attention row is a pending one");
    let ops = [OP_A, OP_B, OP_C, OP_D];
    let staying: Vec<&str> = ops
        .iter()
        .copied()
        .filter(|operation| pending_key(operation) != first)
        .collect();
    assert_eq!(staying.len(), 3, "{leaving}");
    relist(shell, &staying, cx);
    settle_popover(shell, cx);
    assert!(is_open(shell, cx), "it stays open");
    assert!(
        !keys_of(&applied(shell, cx)).contains(&first),
        "the row left"
    );
    assert_eq!(spot(shell, cx), Spot::List, "focus moved to its list");
    let moved = cursor(shell, cx);
    shell.keys("down", cx);
    assert_ne!(cursor(shell, cx), moved, "the arrows still work");
    shell.keys("escape", cx);
    assert!(!is_open(shell, cx), "and Escape");
    assert_eq!(spot(shell, cx), Spot::Item);
}

/// The row at the highlight's position has no enabled button while its button
/// holds focus: the popover stays open, focus moves to its list, and the
/// arrows, Home and End and Escape still work from there (E2, P25). The
/// highlighted row stays in the log (so `kept`, the other half of the rule,
/// does not move focus) but is listed as a remote synchronization now, whose
/// Resume is disabled.
fn the_row_at_the_position_has_no_button(cx: &mut TestAppContext) {
    let world = rows(cx);
    let shell = &world.shell;
    settle(shell, cx);
    open_by_click(shell, cx);
    let highlighted = world.pending[3].clone();
    let at = keys_of(&applied(shell, cx))
        .iter()
        .position(|row| row == &highlighted)
        .expect("the row is listed");
    for _ in 0..at {
        shell.keys("down", cx);
    }
    assert_eq!(cursor(shell, cx).as_ref(), Some(&highlighted));
    shell.keys("tab", cx);
    assert_eq!(spot(shell, cx), Spot::Button, "Tab reaches the button");
    let before = applied(shell, cx);
    assert!(
        before
            .iter()
            .find(|row| row.row.key == highlighted)
            .and_then(|row| row.row.button.as_ref())
            .is_some_and(|button| button.enabled),
        "it has a button that can be pressed"
    );
    let mut remote = save(OP_D);
    remote.family = manyhands::repository::OperationFamily::Remote;
    let list = vec![save(OP_A), save(OP_B), save(OP_C), remote];
    let operations = shell.operations(cx);
    operations.update(cx, |operations, cx| {
        let stamp = operations.load_pending(PENDING_ROOT.to_owned());
        assert!(operations.apply_pending(stamp, listing(list)));
        cx.notify();
    });
    shell.run(cx);
    settle_popover(shell, cx);
    assert!(is_open(shell, cx), "it stays open");
    let listed = applied(shell, cx);
    assert_eq!(
        cursor(shell, cx).as_ref(),
        Some(&highlighted),
        "the row stayed"
    );
    assert!(
        listed
            .iter()
            .find(|row| row.row.key == highlighted)
            .and_then(|row| row.row.button.as_ref())
            .is_some_and(|button| !button.enabled),
        "its button cannot be pressed now"
    );
    assert_eq!(spot(shell, cx), Spot::List, "focus moved to its list");
    shell.keys("home", cx);
    let first = keys_of(&listed);
    assert_eq!(cursor(shell, cx).as_ref(), first.first());
    shell.keys("down", cx);
    assert_eq!(cursor(shell, cx).as_ref(), first.get(1), "the arrows work");
    assert_eq!(spot(shell, cx), Spot::List);
    shell.keys("escape", cx);
    assert!(!is_open(shell, cx), "Escape closes it");
    assert_eq!(
        spot(shell, cx),
        Spot::Item,
        "focus goes to the restore target"
    );
}
