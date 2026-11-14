//! The Repositories panel's three toolbar buttons, over the window the
//! parent module opens (E-11): Add, More, the menu keys and the guards.
//! Check all is tested in `check_all_checks_each_repository`.

use std::path::Path;

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use super::{
    super::support::act,
    {TICKET, dialog, handle, holds_region, tab_to},
};
use crate::desktop::{
    model::{
        list_view::{REBUILD_INDEX, REFRESH},
        navigation::OpenTarget,
        repository_state::REMOVE_FROM_MANYHANDS,
        stamp::SettingsPage,
    },
    services::requests::Intent,
    test_support::fixtures::item_id,
    view::{
        harness::{Shell, key},
        repositories::tests::support as menu,
        settings::Shown as Page,
        workspace::{Region, ToolbarButton},
    },
};

fn index_requests(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> Vec<crate::desktop::services::requests::DesktopRequestId> {
    shell.requests(
        cx,
        |request| matches!(&request.intent, Intent::IndexRefresh { root: asked } if asked == root),
    )
}

/// The Repositories panel's three buttons (P13, P28, P39, E-11): Add by Tab
/// and Enter opens Settings > Repositories; `secondary-n` from a button
/// opens nothing; `menu` and `shift-f10` on a button open the chosen
/// repository's menu, not the Kit cursor's, and Esc returns focus to the
/// button; More opens its menu below itself and Refresh sends
/// `IndexRefresh` for the chosen repository; and all of them are refused
/// behind a dialog and a sheet.
pub(super) async fn repositories_toolbar(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let chosen = key(root);
    let (add, more) = (
        handle(shell, ToolbarButton::AddRepository, cx),
        handle(shell, ToolbarButton::RepositoriesMore, cx),
    );
    let page = |cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
    };
    let cursor = |cx: &mut TestAppContext| {
        shell.workspace.read_with(cx, |workspace, cx| {
            workspace.repositories_pane().highlighted(cx)
        })
    };

    // Add: its region holds focus with the ring; Enter opens Settings.
    tab_to(shell, ToolbarButton::AddRepository, cx);
    holds_region(shell, Region::Repositories, cx);
    shell.press("enter", cx);
    assert_eq!(page(cx), Page::Settings(SettingsPage::Repositories));
    assert!(
        shell.holds_focus("add-path", cx),
        "W-6 lands on the Add folder (Part D)"
    );
    // Settings is a sheet: it closes before the toolbar is used again.
    shell.keys("escape", cx);

    // `secondary-n` from a Repositories button opens nothing (R-14).
    tab_to(shell, ToolbarButton::AddRepository, cx);
    shell.keys("secondary-n", cx);
    assert_eq!(dialog(shell, cx), None, "secondary-n opens no dialog");
    assert!(!shell.present("popup-menu", cx), "nor a menu");

    // The Kit cursor on the other row; `shift-f10` and `menu` on Add open
    // the chosen repository's menu, and Esc gives focus back to Add.
    shell.select(root, cx);
    shell.keys("secondary-1", cx);
    shell.keys("down", cx);
    if cursor(cx) == Some(chosen.clone()) {
        shell.keys("up", cx);
    }
    assert_ne!(cursor(cx), Some(chosen.clone()), "the cursor is elsewhere");
    for keystroke in ["shift-f10", "menu"] {
        tab_to(shell, ToolbarButton::AddRepository, cx);
        shell.keys(keystroke, cx);
        assert_eq!(
            menu::open_root(shell, cx),
            Some(chosen.clone()),
            "{keystroke}"
        );
        assert_ne!(cursor(cx), Some(chosen.clone()), "{keystroke}: cursor");
        assert!(
            menu::press_and_hold(shell, "escape", &add, cx),
            "{keystroke}: Esc returns to Add"
        );
        assert_eq!(menu::open_root(shell, cx), None);
    }

    // More: its menu opens for the chosen repository, below the button,
    // whatever row the cursor rests on; Refresh sends `IndexRefresh` for
    // it and focus stays on the button.
    tab_to(shell, ToolbarButton::RepositoriesMore, cx);
    holds_region(shell, Region::Repositories, cx);
    shell.press("enter", cx);
    assert_eq!(menu::open_root(shell, cx), Some(chosen.clone()), "More");
    assert_eq!(
        menu::labels(shell, cx),
        [REFRESH, REBUILD_INDEX, REMOVE_FROM_MANYHANDS]
    );
    let below = shell.frame(cx, |window, _| {
        window
            .find(ToolbarButton::RepositoriesMore.id())
            .bounds()
            .bottom_left()
    });
    let at = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.repository_menu_at());
    assert_eq!(at, Some(below), "below the More button");
    let known = index_requests(shell, root, cx);
    assert!(menu::choose_and_hold(shell, REFRESH, &more, cx), "focus");
    let sent: Vec<_> = index_requests(shell, root, cx)
        .into_iter()
        .filter(|id| !known.contains(id))
        .collect();
    assert_eq!(sent.len(), 1, "Refresh sends IndexRefresh");
    shell.until_ended(sent[0], cx).await;
    shell.run(cx);

    // Behind a dialog and behind the details sheet, no button acts (P12).
    let target = OpenTarget::Item {
        repo: chosen.clone(),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.select(root, cx);
    let shown = page(cx);
    for sheet in [false, true] {
        crate::desktop::view::footer::tests::support::open_modal(shell, sheet, cx);
        let sent = shell.requests(cx, |_| true).len();
        for button in [
            ToolbarButton::AddRepository,
            ToolbarButton::CheckAll,
            ToolbarButton::RepositoriesMore,
        ] {
            act(shell, cx, |workspace, window, cx| {
                workspace.press_toolbar(button, window, cx)
            });
            assert!(
                !shell.present("popup-menu", cx),
                "{button:?}: sheet {sheet}"
            );
        }
        assert_eq!(page(cx), shown, "sheet {sheet}: Add opened nothing");
        assert_eq!(shell.requests(cx, |_| true).len(), sent, "sheet {sheet}");
        shell.frame(cx, |window, cx| {
            use gpui_kit::component::WindowExt;
            window.close_all_dialogs(cx);
            window.close_sheet(cx);
        });
        shell.run(cx);
    }
}
