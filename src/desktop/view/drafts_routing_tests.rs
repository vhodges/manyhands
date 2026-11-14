//! The Drafts page in the content router (ruling R23): `secondary-shift-d`
//! shows it and focuses its first control; Settings opened over it leaves
//! it shown under the sheet and after it closes (Wave 04 Cycle 04, R-31);
//! closing Settings empties a typed passphrase, and the drafts key then
//! shows the page; `secondary-w` closes no tab while it is shown; and tab
//! stepping shows the tabs.

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use super::{
    row_id,
    tests::{ONE, TWO, drafts_shown, open, ticket, with_ticket},
};
use crate::desktop::{
    test_support::{drafts::store_draft, fixtures::Fixture},
    view::{
        harness::Shell,
        settings::{SettingsHost, Shown},
        workspace::Region,
    },
};

/// Repository `a` with tickets One and Two, both open in tabs, Two shown,
/// and a draft of One stored.
fn with_tabs(cx: &mut TestAppContext) -> (Shell, manyhands::drafts::DraftId) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    (shell, id)
}

fn tab_count(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len())
}

fn settings_shown(shell: &Shell, cx: &mut TestAppContext) -> Shown {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}

#[gpui_kit::test]
fn the_drafts_key_shows_the_page_and_focuses_its_first_control(cx: &mut TestAppContext) {
    let (shell, id) = with_tabs(cx);
    shell.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&shell, cx));
    assert!(shell.present("drafts", cx));
    assert!(!shell.present(("tab", 0usize), cx), "in place of the tabs");
    assert_eq!(tab_count(&shell, cx), 2, "the tabs are kept");
    assert!(shell.frame(cx, |window, _| {
        window.find(row_id(id, "row")).focused() == Some(true)
    }));
    assert!(shell.present(Region::Content.ring_id(), cx));
}

/// Settings opened over the page leaves it shown under the sheet; Escape
/// closes Settings, and the page is still shown, not the tabs (R-31).
#[gpui_kit::test]
fn closing_settings_over_drafts_leaves_drafts_shown(cx: &mut TestAppContext) {
    let (shell, _) = with_tabs(cx);
    shell.keys("secondary-shift-d", cx);
    shell.keys("secondary-,", cx);
    assert!(drafts_shown(&shell, cx));
    assert!(matches!(settings_shown(&shell, cx), Shown::Settings(_)));
    assert!(shell.present("drafts", cx));

    shell.keys("escape", cx);
    assert_eq!(settings_shown(&shell, cx), Shown::Tabs);
    assert!(drafts_shown(&shell, cx));
    assert!(!shell.present(("tab", 1usize), cx));
}

/// A passphrase typed on the Keys page is emptied when Settings closes;
/// the drafts key, which does not act behind the sheet, then shows the
/// page.
#[gpui_kit::test]
fn escape_empties_a_typed_passphrase_and_the_drafts_key_then_shows_drafts(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    shell.keys("secondary-,", cx);
    shell.click(("settings-page", 1usize), cx);
    assert!(shell.present("settings-keys", cx));
    let host: gpui_kit::Entity<SettingsHost> = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone());
    let inputs = shell.visual(cx).update(|window, cx| {
        let keys = host.update(cx, |host, cx| host.keys(window, cx));
        let inputs =
            ["key-generate-passphrase", "key-generate-confirm"].map(|id| keys.read(cx).input(id));
        for input in &inputs {
            input.update(cx, |input, cx| {
                input.set_value("sentinel-passphrase-d2a1", window, cx)
            });
        }
        inputs
    });
    shell.run(cx);

    shell.keys("escape", cx);
    assert_eq!(settings_shown(&shell, cx), Shown::Tabs);
    shell.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&shell, cx));
    for input in inputs {
        assert_eq!(
            input.read_with(cx, |input, _| input.value().to_string()),
            ""
        );
    }
}

#[gpui_kit::test]
fn closing_a_tab_does_nothing_while_the_page_is_shown(cx: &mut TestAppContext) {
    let (shell, _) = with_tabs(cx);
    shell.keys("secondary-shift-d", cx);
    shell.keys("secondary-w", cx);
    assert_eq!(tab_count(&shell, cx), 2);
    assert!(drafts_shown(&shell, cx));
}

#[gpui_kit::test]
fn stepping_tabs_from_the_page_shows_the_tabs(cx: &mut TestAppContext) {
    let (shell, _) = with_tabs(cx);
    shell.keys("secondary-shift-d", cx);
    shell.keys("ctrl-tab", cx);
    assert!(!drafts_shown(&shell, cx));
    assert!(shell.present(("tab", 0usize), cx));
    assert_eq!(tab_count(&shell, cx), 2);
}
