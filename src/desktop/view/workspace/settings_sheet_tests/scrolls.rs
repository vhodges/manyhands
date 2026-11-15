//! P8's rows for the sheet's one scroll across entries (Cycle decision 6;
//! ledger Ruling 35, the orchestrator's review ruling): a page keeps its
//! offset when the same entry is shown again, and the offset resets to the
//! top only when the entry shown changes, with a field route's reveal
//! still working after the reset.
//!
//! And a field route's field in view once the page's reads land (A6 F1,
//! R-24, decision 8; the orchestrator's fix ruling): the route moves focus
//! while the page draws what it reads as short lines, and what lands above
//! the field pushes it down; the sheet's region follows it, until the
//! person scrolls away.

use std::path::Path;

use gpui_kit::{ElementId, TestAppContext, px, test::TestWindowExt};

use super::support::{closed, holds, route, settings_offset, settle, shown};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::{
        harness::Shell,
        settings::{SettingsRoute, Shown},
    },
};

/// Keys on the fixture's home, so that Sharing & keys draws a page taller
/// than the short window.
pub(super) const KEYS: usize = 12;

/// Remotes of `b`, the fixture's second repository: Sharing draws a row
/// for each above the add-remote form, taller than the short window.
const REMOTES: usize = 4;

/// `b`, beside the fixture's `a`, with `REMOTES` remotes. Nothing is
/// contacted.
pub(super) fn shared(fixture: &Fixture) {
    let b = fixture.create_repository("b");
    for ix in 0..REMOTES {
        let url = format!("https://example.invalid/team/repo-{ix}.git");
        fixture.add_remote(&b, &format!("remote-{ix}"), &url);
    }
    fixture.refresh(&b);
}

/// Run from P8 with Settings open on Repositories, in a short window whose
/// home holds `KEYS` keys.
pub(super) async fn rows(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    first_visit(shell, root, cx);

    // Sharing & keys by its entry, every key listed: the page is tall.
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
    // reset to the top and not undone by it, and it is in view (E-44).
    assert!(
        settings_offset(shell, cx) < px(0.),
        "the route scrolled the sheet, after the reset"
    );
    in_view(
        shell,
        "hosts-check",
        "(d) Hosts' control after the reset",
        cx,
    );

    remotes(shell, cx);
}

/// (e), an addition: the first visit to Sharing & keys is a Hosts route,
/// raised while the keys are still read, so Check host is drawn just under
/// the keys' one short line and focused there. Once the twelve keys land
/// above it, it is in view. (f), an addition: a scroll away from it is
/// left alone by the draws that follow.
fn first_visit(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    // The page reads the keys when it is made and again as it is shown.
    let held = [(); 2].map(|_| shell.backend.hold_read(ReadKind::Keys));
    route(shell, cx, |workspace, window, cx| {
        workspace.open_hosts(root.to_owned(), window, cx)
    });
    assert!(
        holds(shell, "hosts-check", cx),
        "(e) the Hosts route focuses Check host"
    );
    assert!(
        !shell.present(("key-label", 0usize), cx),
        "(e) the keys are still read"
    );
    drop(held);
    shell.keys_landed(cx);
    assert!(
        shell.present(("key-label", KEYS - 1), cx),
        "(e) every key listed"
    );
    in_view(
        shell,
        "hosts-check",
        "(e) Check host, once the keys land",
        cx,
    );

    // (f) Home, from Check host: the draws that follow leave the sheet at
    // its top, Check host still focused, below the fold.
    shell.keys("home", cx);
    assert_eq!(settings_offset(shell, cx), px(0.), "(f) Home, the top");
    assert!(holds(shell, "hosts-check", cx), "(f) focus has not moved");
    assert_eq!(
        settings_offset(shell, cx),
        px(0.),
        "(f) the scroll away is left alone"
    );
}

/// (g), an addition: a Remotes route on `b`, whose remotes Settings has
/// not shown, raised while they are still read, so the add-remote Name is
/// drawn under Sharing's one short line and focused there. Once the rows
/// land above it, it is in view. (h), an addition: the Identity route on
/// the same repository, its field in view.
fn remotes(shell: &Shell, cx: &mut TestAppContext) {
    let b = shell.fixture.parent().join("b");
    shell.keys("escape", cx);
    closed(shell, "Escape", cx);
    // Choosing `b` reads its remotes, the page's open reads them again, and
    // Trusted servers reads its shared copy.
    let held = [(); 4].map(|_| shell.backend.hold_read(ReadKind::Remotes));
    let on = b.clone();
    route(shell, cx, move |workspace, window, cx| {
        workspace.open_remotes(on, window, cx)
    });
    assert!(
        holds(shell, "remote-add-name", cx),
        "(g) the Remotes route focuses the add-remote Name"
    );
    assert!(
        !shell.present(("remote-name", 0usize), cx),
        "(g) b's remotes are still read"
    );
    drop(held);
    shell.run(cx);
    assert!(
        shell.present(("remote-name", REMOTES - 1), cx),
        "(g) every remote listed"
    );
    in_view(shell, "remote-add-name", "(g) the add-remote Name", cx);

    route(shell, cx, move |workspace, window, cx| {
        workspace.show_settings(SettingsRoute::Identity(b), None, window, cx)
    });
    assert!(
        holds(shell, "identity-name", cx),
        "(h) the Identity route focuses its Name"
    );
    in_view(shell, "identity-name", "(h) the identity Name", cx);
}

/// Asserts that the element `id` lies wholly inside the sheet body's
/// visible part, after two draws.
fn in_view(shell: &Shell, id: impl Into<ElementId>, row: &str, cx: &mut TestAppContext) {
    settle(shell, cx);
    let id = id.into();
    let (viewport, control) = shell.frame(cx, |window, _| {
        (
            window.find("settings-scroll").bounds(),
            window.find(id).bounds(),
        )
    });
    assert!(
        viewport.top() <= control.top() && control.bottom() <= viewport.bottom(),
        "{row} is in view: {control:?} in {viewport:?}"
    );
}
