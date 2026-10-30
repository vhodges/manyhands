//! The settings in the main window's content region (ruling R-B4):
//! `secondary-,` and the sidebar's Settings entry show them, the tabs are
//! kept as they were behind them, Back to tabs and opening an item show
//! the tabs again, and an open dialog keeps both keys from acting.

use std::path::PathBuf;

use gpui_kit::{TestAppContext, component::WindowExt, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use super::{BACK_TO_TABS, PAGES, SETTINGS_HEADING, Shown, page_title};
use crate::desktop::{
    model::{navigation::OpenTarget, stamp::SettingsPage, tabs::TabKey},
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        tabs::SHOWN,
        workspace::Region,
    },
};

const ONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR0";
const TWO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";

/// Repository `a` with tickets One and Two, both open in tabs, Two shown.
fn with_tabs(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, ONE, "One", "");
    fixture.commit_ticket(&root, TWO, "Two", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    for id in [ONE, TWO] {
        open(&shell, &root, id, cx);
    }
    (shell, root)
}

fn open(shell: &Shell, root: &std::path::Path, id: &str, cx: &mut TestAppContext) {
    let target = ticket(root, id);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn ticket(root: &std::path::Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn shown(shell: &Shell, cx: &mut TestAppContext) -> Shown {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}

fn tab_keys(shell: &Shell, cx: &mut TestAppContext) -> Vec<TabKey> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys())
}

/// `secondary-,` shows the Repositories page in the content region, in
/// place of the tabs, and focuses its entry in the page list, which says
/// it is shown in text. Back to tabs shows the tabs as they were, the
/// same tab shown, and focuses it.
#[gpui_kit::test]
fn the_settings_key_shows_the_settings_and_back_keeps_the_tabs(cx: &mut TestAppContext) {
    let (shell, root) = with_tabs(cx);
    let tabs = tab_keys(&shell, cx);
    assert!(shell.present(("tab-active-line", 1usize), cx));

    shell.keys("secondary-,", cx);
    assert_eq!(
        shown(&shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    assert_eq!(
        shell.label("settings-heading", cx).as_deref(),
        Some(SETTINGS_HEADING)
    );
    assert_eq!(
        shell.label(("settings-page", 0usize), cx).as_deref(),
        Some(page_title(SettingsPage::Repositories))
    );
    assert_eq!(
        shell.label(("settings-page-shown", 0usize), cx).as_deref(),
        Some(SHOWN)
    );
    assert_eq!(PAGES[0], SettingsPage::Repositories);
    assert!(shell.present("settings-repositories", cx));
    // The tabs are not drawn, and are kept.
    assert!(!shell.present(("tab", 0usize), cx));
    assert_eq!(tab_keys(&shell, cx), tabs);
    // Focus is on the page's entry, inside the content region.
    assert!(shell.frame(cx, |window, _| {
        window.find(("settings-page", 0usize)).focused() == Some(true)
    }));
    assert!(shell.present(Region::Content.ring_id(), cx));

    assert_eq!(
        shell.label("settings-close", cx).as_deref(),
        Some(BACK_TO_TABS)
    );
    shell.click("settings-close", cx);
    assert_eq!(shown(&shell, cx), Shown::Tabs);
    assert!(!shell.present("settings", cx));
    assert_eq!(tab_keys(&shell, cx), tabs);
    assert!(shell.present(("tab-active-line", 1usize), cx));
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(opened, Some(ticket(&root, TWO)));
}

/// The sidebar's Settings entry shows the settings too.
#[gpui_kit::test]
fn the_sidebar_entry_shows_the_settings(cx: &mut TestAppContext) {
    let (shell, _) = with_tabs(cx);
    assert!(shell.present("sidebar-settings", cx));
    shell.click("sidebar-settings", cx);
    assert_eq!(
        shown(&shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    assert!(shell.present("settings-repositories", cx));
}

/// Opening an item while the settings are shown shows the tabs, with
/// the item's tab shown.
#[gpui_kit::test]
fn opening_an_item_shows_the_tabs_again(cx: &mut TestAppContext) {
    let (shell, root) = with_tabs(cx);
    shell.keys("secondary-,", cx);
    assert!(shell.present("settings", cx));

    open(&shell, &root, ONE, cx);
    assert_eq!(shown(&shell, cx), Shown::Tabs);
    assert!(shell.present(("tab", 0usize), cx));
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(opened, Some(ticket(&root, ONE)));
}

/// While a dialog is open, neither `secondary-,` nor anything behind it
/// shows the settings (I2 follow-up).
#[gpui_kit::test]
fn the_settings_key_does_nothing_while_a_dialog_is_open(cx: &mut TestAppContext) {
    let (shell, _) = with_tabs(cx);
    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);
    shell.keys("secondary-,", cx);
    assert_eq!(shown(&shell, cx), Shown::Tabs);
    assert!(!shell.present("settings", cx));
}

/// `secondary-3` with the settings shown focuses the shown page's entry,
/// so the content region's own focus is the settings.
#[gpui_kit::test]
fn focusing_the_content_region_focuses_the_settings(cx: &mut TestAppContext) {
    let (shell, _) = with_tabs(cx);
    shell.keys("secondary-,", cx);
    shell.keys("secondary-1", cx);
    assert!(!shell.present(Region::Content.ring_id(), cx));
    shell.keys("secondary-3", cx);
    assert!(shell.frame(cx, |window, _| {
        window.find(("settings-page", 0usize)).focused() == Some(true)
    }));
}
