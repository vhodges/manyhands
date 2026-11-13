//! P24 (the popover with Activity hidden) and P27 (nothing acts behind a
//! dialog or sheet).

use gpui_kit::{Point, component::WindowExt};

use super::*;
use crate::desktop::{
    keys::{
        ActivityDown, ActivityEnd, ActivityEnter, ActivityHome, ActivityUp, AttentionDown,
        AttentionEnd, AttentionEnter, AttentionHome, AttentionNext, AttentionPrevious, AttentionUp,
        ClearFinished, CloseAttention, OpenActivityMenu,
    },
    model::{layout::PanelKind::Activity, navigation::OpenTarget},
    view::{
        activity::tests::{
            list_support::{OP_A, OP_B, OP_C, OP_D, Rows, save},
            pending_cases::{PENDING_ROOT, listing, read_pending},
        },
        activity_support::{highlighted, list_focused},
        footer::tests::support::{modal_open, open_modal},
        harness::key,
        workspace::ToolbarButton,
    },
};
use manyhands::repository::ItemDtoKind;

/// The toolbar's filter button pressed, and the panel drawn again.
fn press_filter(shell: &crate::desktop::view::harness::Shell, cx: &mut TestAppContext) {
    shell.click(ToolbarButton::ActivityFilter.id(), cx);
    settle_popover(shell, cx);
}

/// P24: with Activity hidden, the popover opens and lists the log's attention
/// rows, which do not depend on Activity being drawn (A4); its link, by key
/// or by click, closes it, turns the filter off, shows Activity and focuses
/// its list with the first row highlighted.
#[gpui_kit::test]
fn the_popover_works_with_activity_hidden(cx: &mut TestAppContext) {
    for by_click in [false, true] {
        let world = rows(cx);
        let shell = &world.shell;
        settle(shell, cx);
        // The filter is on while Activity is hidden: the link turns it off.
        press_filter(shell, cx);
        assert_eq!(shell.checked("activity-filter", cx), Some(true));
        shell.click(Control::Panel(Activity).id(), cx);
        assert!(!shell.present("operation-area", cx), "Activity is hidden");
        let attention = keys_of(&attention_rows(shell, cx));
        assert!(attention.len() >= 4, "rows: {attention:?}");

        open_by_click(shell, cx);
        assert!(drawn(shell, cx), "it opens with Activity hidden");
        assert_eq!(
            keys_of(&applied(shell, cx)),
            attention,
            "and lists the rows"
        );
        assert!(
            shell.present("needs-attention-show-all", cx),
            "with its link"
        );

        if by_click {
            shell.click("needs-attention-show-all", cx);
        } else {
            // Tab: the list, the highlighted row's button, the link.
            shell.keys("tab", cx);
            assert_eq!(spot(shell, cx), Spot::Button);
            shell.keys("tab", cx);
            assert_eq!(spot(shell, cx), Spot::Link, "Tab's third stop");
            shell.press("enter", cx);
        }
        settle_popover(shell, cx);
        assert!(!is_open(shell, cx), "the link closes the popover");
        assert!(
            shell.present("operation-area", cx),
            "and shows Activity (by click {by_click})"
        );
        let filter = shell.checked("activity-filter", cx);
        assert_eq!(filter, Some(false), "with the filter off");
        assert!(list_focused(shell, cx), "its list has focus");
        let all: Vec<_> = rows_in_log(shell, cx);
        assert!(all.len() > attention.len(), "the filter hid nothing now");
        assert_eq!(
            highlighted(shell, cx),
            all.first().cloned(),
            "the first row is highlighted"
        );
    }
}

/// The log's rows' keys, newest first.
fn rows_in_log(
    shell: &crate::desktop::view::harness::Shell,
    cx: &mut TestAppContext,
) -> Vec<crate::desktop::model::activity::rows::RowKey> {
    keys_of(&crate::desktop::view::activity_support::rows(shell, cx))
}

/// What a press of anything in Activity or the popover would change: the
/// requests, the layers, the filter, the highlight and the choices made.
fn footprint(world: &Rows, cx: &mut TestAppContext) -> String {
    let shell = &world.shell;
    let requests = shell.requests(cx, |_| true).len();
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len());
    let layers = shell.workspace.read_with(cx, |workspace, cx| {
        format!(
            "popover {} menu {} more {} dock {} filter {} pressed {}/{} {:?} {:?}",
            workspace.attention_popover_open(),
            workspace.activity_menu_open(),
            workspace.activity_more_open(),
            workspace.dock_menu_open(),
            workspace.activity_filter(),
            workspace.activity_pressed(),
            workspace.attention_pressed(),
            workspace.settings_shown(cx),
            workspace.layout().clone(),
        )
    });
    format!("{requests} requests, {tabs} tabs, {layers}")
}

/// P27: with a dialog open, and with the details sheet open, nothing acts:
/// the toolbar, the row menu and its key, Enter in the list and the other
/// list keys, the footer item and the popover's keys do nothing, and a
/// popover that was open is gone. Once the modal closes, the same presses
/// act, so the refusal is the modal's.
#[gpui_kit::test]
fn nothing_acts_behind_a_dialog_or_sheet(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    for sheet in [false, true] {
        let world = rows(cx);
        let shell = &world.shell;
        shell.select(&world.root, cx);
        let target = OpenTarget::Item {
            repo: key(&world.root),
            kind: ItemDtoKind::Ticket,
            id: crate::desktop::test_support::fixtures::item_id(
                crate::desktop::view::activity::tests::cases::TICKET,
            ),
        };
        shell
            .workspace
            .update(cx, |workspace, cx| workspace.open_item(target, cx));
        shell.run(cx);
        // Selecting the repository read its pending list again: the rows
        // the window shows are the fixture's once more.
        read_pending(
            shell,
            PENDING_ROOT,
            listing([OP_A, OP_B, OP_C, OP_D].map(save).to_vec()),
            cx,
        );
        settle(shell, cx);
        // The popover is open when the modal opens, and the list has been
        // used: a highlighted row to press.
        shell.keys("secondary-4", cx);
        open_by_click(shell, cx);
        assert!(is_open(shell, cx));
        let row = world.pending[0].clone();
        open_modal(shell, sheet, cx);
        shell.frame(cx, |_, _| ());
        assert!(!is_open(shell, cx), "the modal closed the popover");
        let before = footprint(&world, cx);

        shell.frame(cx, |window, cx| {
            shell.workspace.update(cx, |workspace, cx| {
                let at = Point::default();
                workspace.open_activity_menu_at(row.clone(), at, window, cx);
                workspace.press_toolbar(ToolbarButton::ActivityFilter, window, cx);
                workspace.press_toolbar(ToolbarButton::ActivityMore, window, cx);
                workspace.press(Control::Attention, window, cx);
            });
            window.dispatch_action(Box::new(ActivityEnter), cx);
            window.dispatch_action(Box::new(ActivityDown), cx);
            window.dispatch_action(Box::new(ActivityUp), cx);
            window.dispatch_action(Box::new(ActivityHome), cx);
            window.dispatch_action(Box::new(ActivityEnd), cx);
            window.dispatch_action(Box::new(OpenActivityMenu), cx);
            window.dispatch_action(Box::new(ClearFinished), cx);
            window.dispatch_action(Box::new(AttentionEnter), cx);
            window.dispatch_action(Box::new(AttentionDown), cx);
            window.dispatch_action(Box::new(AttentionUp), cx);
            window.dispatch_action(Box::new(AttentionHome), cx);
            window.dispatch_action(Box::new(AttentionEnd), cx);
            window.dispatch_action(Box::new(AttentionNext), cx);
            window.dispatch_action(Box::new(AttentionPrevious), cx);
            window.dispatch_action(Box::new(CloseAttention), cx);
        });
        shell.run(cx);
        assert!(modal_open(shell, cx), "the modal stays (sheet {sheet})");
        assert_eq!(
            footprint(&world, cx),
            before,
            "nothing acted (sheet {sheet})"
        );

        // Closed, the footer item and the list act.
        shell.frame(cx, |window, cx| {
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
        assert!(!modal_open(shell, cx));
        shell.frame(cx, |window, cx| {
            shell.workspace.update(cx, |workspace, cx| {
                workspace.press(Control::Attention, window, cx)
            })
        });
        shell.run(cx);
        assert!(is_open(shell, cx), "the item acts once the modal is closed");
    }
}
