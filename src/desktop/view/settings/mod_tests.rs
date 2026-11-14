//! The settings, a sheet over the main window (Wave 04 Cycle 04):
//! `secondary-,` and the sidebar's Settings entry show them, the tabs are
//! kept as they were behind them, Escape closes them, and an open dialog
//! keeps both keys from acting.

use std::path::PathBuf;

use gpui_kit::{TestAppContext, component::WindowExt, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use super::{
    CHECKING_PAGE, PAGES, REPOSITORIES_PAGE, SETTINGS_HEADING, SHARING_AND_KEYS_PAGE, Shown,
    page_title,
};
use crate::desktop::{
    model::{navigation::OpenTarget, stamp::SettingsPage, tabs::TabKey},
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
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

/// `secondary-,` shows the Repositories page in the Settings sheet, over
/// the tabs, and focuses its entry in the menu, which announces it
/// selected. Escape closes the sheet and leaves the tabs as they were, the
/// same tab shown.
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
    assert_eq!(PAGES[0], SettingsPage::Repositories);
    assert!(shell.present("settings-repositories", cx));
    // The tabs are kept.
    assert_eq!(tab_keys(&shell, cx), tabs);
    // Focus is on the page's entry, inside the content region.
    assert!(shell.frame(cx, |window, _| {
        window.find(("settings-page", 0usize)).focused() == Some(true)
    }));
    assert!(shell.present(Region::Content.ring_id(), cx));

    shell.keys("escape", cx);
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

/// P16 (B1; R-4, R-27). The menu has three entries in order, with no
/// fourth. Each opens its first page, by Enter, Space or a click, and only
/// the active one is announced selected.
#[gpui_kit::test]
fn settings_has_three_pages(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    shell.keys("secondary-,", cx);

    let titles = [REPOSITORIES_PAGE, SHARING_AND_KEYS_PAGE, CHECKING_PAGE];
    for (ix, title) in titles.into_iter().enumerate() {
        assert_eq!(
            shell.label(("settings-page", ix), cx).as_deref(),
            Some(title)
        );
    }
    assert!(!shell.present(("settings-page", 3usize), cx));
    assert_eq!(
        PAGES.to_vec(),
        [
            SettingsPage::Repositories,
            SettingsPage::Keys,
            SettingsPage::Polling
        ]
    );

    let selected = |shell: &Shell, cx: &mut TestAppContext| -> Vec<bool> {
        (0..3usize)
            .map(|ix| shell.selected(("settings-page", ix), cx))
            .collect()
    };
    assert_eq!(selected(&shell, cx), [true, false, false]);

    // Enter and Space on a focused entry open it, and so does a click.
    for (ix, how) in [(1usize, "enter"), (2, "space"), (0, "enter"), (1, "space")] {
        assert!(shell.tab_reaches(("settings-page", ix), 8, cx), "{ix}");
        shell.press(how, cx);
        assert_eq!(shown(&shell, cx), Shown::Settings(PAGES[ix]), "{how} {ix}");
        let mut only = [false; 3];
        only[ix] = true;
        assert_eq!(selected(&shell, cx), only, "{how} {ix}");
    }
    for ix in [2usize, 0, 1] {
        shell.click(("settings-page", ix), cx);
        assert_eq!(shown(&shell, cx), Shown::Settings(PAGES[ix]), "click {ix}");
        let mut only = [false; 3];
        only[ix] = true;
        assert_eq!(selected(&shell, cx), only, "click {ix}");
    }

    // The shown page's view is the entry's: Sharing & keys draws the keys.
    assert!(shell.present("settings-keys", cx));
}
