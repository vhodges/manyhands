//! One chosen repository (Wave 04 Cycle 04 design C1; R-14, R-15, R-32,
//! R-56; Part G items 2 and 5). Manage, a route's repository and a panel
//! selection made since the last open choose the repository that the
//! Repositories page, its Sharing section and Trusted servers' Check host
//! work on. Choosing it again, or an open with nothing newer, keeps what is
//! typed in its identity form, and removing it unchooses it on every page.
//! Every open reads the Repositories page again, and a slow identity read
//! fills only the fields still empty.

use std::path::{Path, PathBuf};

use gpui_kit::{Entity, TestAppContext, test::TestWindowExt};

use super::{RepositoriesPage, SettingsRoute, hosts, remotes};
use crate::desktop::{
    services::{ReadKind, requests::Intent},
    test_support::fixtures::{Fixture, identity},
    view::{
        footer::Control,
        harness::{Shell, key},
    },
};

const HTTPS_URL: &str = "https://example.invalid/team/repo.git";
/// Text typed into the identity form's name field.
const TYPED: &str = "Typed Example";

/// Facts read one after another. One that does not hold is kept with what
/// was found instead, and the test fails once at its end, naming each: a
/// run shows every row not yet met.
#[derive(Default)]
struct Rows(Vec<String>);

impl Rows {
    fn fact(&mut self, row: &str, held: bool, found: impl std::fmt::Debug) {
        if !held {
            self.0.push(format!("{row}: found {found:?}"));
        }
    }

    fn done(self) {
        assert!(
            self.0.is_empty(),
            "{} rows did not hold:\n  {}",
            self.0.len(),
            self.0.join("\n  ")
        );
    }
}

/// The repository Settings works on.
fn chosen(shell: &Shell, cx: &mut TestAppContext) -> Option<PathBuf> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_chosen(cx))
}

fn repositories(shell: &Shell, cx: &mut TestAppContext) -> Entity<RepositoriesPage> {
    let host = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.settings().clone());
    host.read_with(cx, |host, _| host.repositories.clone())
        .expect("the Repositories page is made")
}

/// The field `id`'s text, after a frame: a landed read's prefill is in.
fn value(shell: &Shell, id: &str, cx: &mut TestAppContext) -> String {
    shell.frame(cx, |_, _| ());
    let input = repositories(shell, cx).read_with(cx, |page, _| page.input(id));
    input.read_with(cx, |input, _| input.value().to_string())
}

/// Sets the field `id` to `text`, as typing it would.
fn set(shell: &Shell, id: &str, text: &str, cx: &mut TestAppContext) {
    let input = repositories(shell, cx).read_with(cx, |page, _| page.input(id));
    shell.visual(cx).update(|window, cx| {
        input.update(cx, |input, cx| input.set_value(text.to_owned(), window, cx));
    });
    shell.run(cx);
}

/// Manage on `root`'s row of the Repositories page.
fn manage(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let rows = shell.rows("managed-row", cx);
    let ix = rows
        .iter()
        .position(|row| *row == key(root))
        .unwrap_or_else(|| panic!("{} listed in {rows:?}", key(root)));
    shell.click(("managed-choose", ix), cx);
}

/// `route` raised through `show_settings`, as a way in raises it.
fn route(shell: &Shell, route: SettingsRoute, cx: &mut TestAppContext) {
    let workspace = shell.workspace.clone();
    shell.visual(cx).update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.show_settings(route, None, window, cx)
        })
    });
    shell.run(cx);
}

/// The Sharing section's repository line, as the Repositories page draws
/// it.
fn sharing_root(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.frame(cx, |window, _| {
        window
            .within("settings-repositories")
            .try_find("remotes-root")
            .and_then(|line| line.label().map(str::to_owned))
    })
}

/// The repository the Repositories page manages, read on that page.
fn managed(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.click(("settings-page", 0usize), cx);
    shell.label("managed-root", cx)
}

/// P10 (C1; R-14, R-15, R-56). Manage b shows b's remotes under Sharing;
/// `Remotes(a)` chooses a; a footer open after the panel's selection
/// changed to c chooses c; a footer open with no change since a route open
/// keeps the route's choice and its typed identity text; choosing the
/// chosen repository again, by Manage or by a route, keeps that text;
/// removing the chosen repository unchooses it for Sharing and Check host.
#[gpui_kit::test]
async fn one_repository_is_chosen(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    let c = fixture.create_repository("c");
    fixture.add_remote(&b, "origin", HTTPS_URL);
    let shell = Shell::open(fixture, cx);
    let mut rows = Rows::default();

    shell.keys("secondary-,", cx);
    manage(&shell, &b, cx);
    let found = chosen(&shell, cx);
    rows.fact("Manage b: chosen", found == Some(b.clone()), found);
    let found = sharing_root(&shell, cx);
    let wanted = remotes::chosen_line(&key(&b));
    rows.fact("Manage b: Sharing", found.as_ref() == Some(&wanted), found);
    let found = shell.rows("remote-name", cx);
    rows.fact("Manage b: its remotes", found == ["origin"], found);

    shell.keys("escape", cx);
    route(&shell, SettingsRoute::Remotes(a.clone()), cx);
    let found = chosen(&shell, cx);
    rows.fact("Remotes(a): chosen", found == Some(a.clone()), found);
    let found = managed(&shell, cx);
    rows.fact("Remotes(a): managed", found == Some(key(&a)), found);

    shell.keys("escape", cx);
    shell.select(&c, cx);
    shell.click(Control::Settings.id(), cx);
    let found = chosen(&shell, cx);
    rows.fact(
        "footer, panel now c: chosen",
        found == Some(c.clone()),
        found,
    );
    let found = managed(&shell, cx);
    rows.fact(
        "footer, panel now c: managed",
        found == Some(key(&c)),
        found,
    );
    let found = sharing_root(&shell, cx);
    let wanted = remotes::chosen_line(&key(&c));
    rows.fact("footer, panel now c: Sharing", found == Some(wanted), found);

    // A route open sees the panel's selection, c, too: the footer open
    // after it finds nothing newer.
    shell.keys("escape", cx);
    route(&shell, SettingsRoute::Identity(a.clone()), cx);
    set(&shell, "identity-name", TYPED, cx);
    shell.keys("escape", cx);
    shell.click(Control::Settings.id(), cx);
    let found = chosen(&shell, cx);
    rows.fact("footer, no change: chosen", found == Some(a.clone()), found);
    let found = value(&shell, "identity-name", cx);
    rows.fact("footer, no change: typed text", found == TYPED, found);

    // The chosen repository chosen again, by its Manage and by routes
    // raised inside the open sheet: the form is left as it is.
    manage(&shell, &a, cx);
    let found = value(&shell, "identity-name", cx);
    rows.fact("Manage a again: typed text", found == TYPED, found);
    for again in [
        SettingsRoute::Manage(a.clone()),
        SettingsRoute::Remotes(a.clone()),
    ] {
        let row = format!("{again:?} again: typed text");
        route(&shell, again, cx);
        let found = value(&shell, "identity-name", cx);
        rows.fact(&row, found == TYPED, found);
    }

    shell.click(("settings-page", 3usize), cx);
    let found = shell.label("hosts-root", cx);
    let wanted = hosts::chosen_line(&key(&a));
    rows.fact("Check host on a", found == Some(wanted), found);
    shell.click(("settings-page", 0usize), cx);
    shell.click("remove-submit", cx);
    shell.click("confirm-ok", cx);
    let id = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoRemove { .. })
    })[0];
    shell.until_ended(id, cx).await;
    shell.run(cx);
    let found = chosen(&shell, cx);
    rows.fact("removed: chosen", found.is_none(), found);
    let found = (
        shell.label("remotes-state", cx),
        shell.present("remotes-root", cx),
    );
    let held = found == (Some(remotes::NO_REPOSITORY.to_owned()), false);
    rows.fact("removed: Sharing", held, found);
    shell.click(("settings-page", 3usize), cx);
    let found = (
        shell.label("hosts-check-state", cx),
        shell.present("hosts-root", cx),
    );
    let held = found == (Some(hosts::NO_REPOSITORY.to_owned()), false);
    rows.fact("removed: Check host", held, found);
    rows.done();
}

/// P11 (R-32): a repository registered while Settings is closed, by
/// another actor, is listed when Settings next opens on Repositories.
#[gpui_kit::test]
fn every_page_is_read_again_on_open(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-,", cx);
    assert_eq!(shell.rows("managed-row", cx), [key(&a)]);
    shell.keys("escape", cx);

    let b = shell.fixture.create_repository("b");
    shell.keys("secondary-,", cx);
    let listed = shell.rows("managed-row", cx);
    assert!(listed.contains(&key(&b)), "{listed:?}");
}

/// P25 (Part G item 5; decision 12): a name typed while the identity read
/// is held stays when the read lands, and the email left empty is filled
/// from it.
#[gpui_kit::test]
fn a_slow_identity_read_fills_only_empty_fields(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-,", cx);
    let held = shell.backend.hold_read(ReadKind::Identity);
    manage(&shell, &a, cx);
    set(&shell, "identity-name", TYPED, cx);
    assert_eq!(value(&shell, "identity-email", cx), "", "still being read");

    let _ = held.send(());
    shell.run(cx);
    assert_eq!(value(&shell, "identity-name", cx), TYPED);
    assert_eq!(value(&shell, "identity-email", cx), identity().email);
}
