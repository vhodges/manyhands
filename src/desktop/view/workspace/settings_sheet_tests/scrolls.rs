//! P8's rows for the sheet's one scroll across entries (Cycle decision 6;
//! ledger Ruling 35, the orchestrator's review ruling): a page keeps its
//! offset when the same entry is shown again, and the offset resets to the
//! top only when the entry shown changes, with a field route's reveal
//! still working after the reset.

use std::path::Path;

use gpui_kit::{TestAppContext, px};

use super::support::{closed, holds, route, settings_offset, shown};
use crate::desktop::{
    model::stamp::SettingsPage,
    view::{harness::Shell, settings::Shown},
};

/// Keys on the fixture's home, so that Sharing & keys draws a page taller
/// than the short window.
pub(super) const KEYS: usize = 12;

/// Run from P8 with Settings open on Repositories, in a short window whose
/// home holds `KEYS` keys.
pub(super) async fn rows(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    // The first visit to Sharing & keys: its read lands, the page is tall.
    shell.click(("settings-page", 1usize), cx);
    shell
        .until_drawn(cx, "every key listed", |shell, cx| {
            shell.present(("key-label", KEYS - 1), cx)
        })
        .await;

    // (a) A later visit, with its last rows still drawn while it reads
    // again: the page shows from its top, not from Repositories' bottom.
    shell.click(("settings-page", 0usize), cx);
    shell.keys("end", cx);
    let bottom = settings_offset(shell, cx);
    assert!(bottom < px(0.), "End scrolled Repositories: {bottom:?}");
    shell.click(("settings-page", 1usize), cx);
    assert!(
        shell.present(("key-label", KEYS - 1), cx),
        "the tall page is drawn"
    );
    assert_eq!(settings_offset(shell, cx), px(0.), "a later visit, the top");

    // (b) The same entry shown again after Escape keeps its offset.
    shell.click(("settings-page", 0usize), cx);
    shell.keys("end", cx);
    let kept = settings_offset(shell, cx);
    assert!(kept < px(0.), "End scrolled Repositories: {kept:?}");
    shell.keys("escape", cx);
    closed(shell, "Escape", cx);
    shell.keys("secondary-,", cx);
    assert_eq!(
        shown(shell, cx),
        Shown::Settings(SettingsPage::Repositories)
    );
    assert_eq!(settings_offset(shell, cx), kept, "the same entry, kept");

    // (c) A different entry, through a route, shows from its top: the
    // Keys page is tall, so a kept offset would show.
    shell.keys("escape", cx);
    closed(shell, "Escape", cx);
    route(shell, cx, |workspace, window, cx| {
        workspace.open_settings(SettingsPage::Keys, window, cx)
    });
    assert_eq!(shown(shell, cx), Shown::Settings(SettingsPage::Keys));
    assert_eq!(settings_offset(shell, cx), px(0.), "another entry, the top");

    // (d) A field route to another entry, Hosts under the keys, still
    // brings its control into view after the reset.
    route(shell, cx, |workspace, window, cx| {
        workspace.open_settings(SettingsPage::Polling, window, cx)
    });
    route(shell, cx, |workspace, window, cx| {
        workspace.open_hosts(root.to_owned(), window, cx)
    });
    shell
        .until_drawn(cx, "the route's control focused", |shell, cx| {
            holds(shell, "hosts-check", cx)
        })
        .await;
    // Below the twelve keys: the route's reveal scrolled to it, after the
    // reset to the top and not undone by it.
    assert!(
        settings_offset(shell, cx) < px(0.),
        "the route scrolled Hosts' control into view"
    );
}
