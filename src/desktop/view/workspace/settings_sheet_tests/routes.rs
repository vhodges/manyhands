//! P4, `each_route_lands_on_its_page_and_control` (design Part D; A6 F1 and
//! F2; R-14, R-22, R-24, R-53): the nine routes, each raised through
//! `show_settings`. A row reads the page Settings arrived at
//! (`settings_shown`), the control focus is on after two draws (Ruling 2)
//! and the repository Settings works on (`settings_chosen`), then Escape
//! closes Settings. The six rows of Ruling 3's F1 and F2 test are here as
//! they were. The ways in that no other test raises, each through its own
//! handler, are in `ways`.
//!
//! The key list (R-22) lands on the radio of the key in use, else the first
//! key's, else Make a new key. While the keys are still read, nothing of it
//! is drawn, so focus goes to the page's entry (F2, a field not drawn), and
//! moves once, when the read lands, if the entry still holds it.
//!
//! A landing on a Kit button, Make a new key or Check host while it can
//! run, is read on the button itself (`Landing::control`), not on its
//! wrapper, and Enter presses it. Check host drawn unavailable keeps focus
//! through its wrapper (R-53) until it can run.

use std::path::{Path, PathBuf};

use gpui_kit::{ElementId, TestAppContext};
use manyhands::{
    repository::{SharedKeyId, transport::SshAuthority},
    results::ResultCode,
};

use super::{
    returns::{escape, settings_open},
    support::{ADD_BUTTON, holds, host, route, shown},
    table::Table,
};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::{
        ReadKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::{
        Fixture, clear_key_through, select_key_through, unregister_key_through,
    },
    view::{
        activity::tests::cases::{answered, fabricate},
        harness::{CHECK_HOST, MAKE_A_NEW_KEY, Shell},
        settings::{SettingsRoute, Shown, polling_support::Rig},
    },
};

/// Where a route lands: the page, the control holding focus and the
/// repository Settings works on.
pub(super) struct Landing {
    pub(super) page: SettingsPage,
    pub(super) focus: ElementId,
    /// Whether `focus` is a Kit button's own ID, read as the control itself
    /// holding focus (`holds_control`), not as an element focus is in.
    pub(super) control: bool,
    pub(super) chosen: Option<PathBuf>,
}

impl Landing {
    pub(super) fn new(
        page: SettingsPage,
        focus: impl Into<ElementId>,
        chosen: Option<&Path>,
    ) -> Self {
        Self {
            page,
            focus: focus.into(),
            control: false,
            chosen: chosen.map(Path::to_owned),
        }
    }

    /// A landing on the Kit button `control` itself, not on its wrapper: its
    /// focus ring drawn and Enter pressing it.
    pub(super) fn control(
        page: SettingsPage,
        control: impl Into<ElementId>,
        chosen: Option<&Path>,
    ) -> Self {
        Self {
            control: true,
            ..Self::new(page, control, chosen)
        }
    }
}

/// The page's menu entry `ix`.
pub(super) fn entry(ix: usize) -> ElementId {
    ("settings-page", ix).into()
}

/// Repositories `a`, shared over SSH, and `b` and `c`, with no shared copy;
/// the keys `k1` and `k2`, neither in use. The clock moves by hand.
pub(super) fn rig(cx: &mut TestAppContext) -> Rig {
    let fixture = Fixture::new();
    let roots: Vec<PathBuf> = ["a", "b", "c"]
        .into_iter()
        .map(|name| fixture.create_repository(name))
        .collect();
    fixture.publish_over_ssh(&roots[0]);
    for root in &roots {
        fixture.refresh(root);
    }
    fixture.generate_key("k1");
    fixture.generate_key("k2");
    Rig::over(fixture, roots, cx)
}

/// Raises `route` through `show_settings`, as every way in does.
pub(super) fn raise(shell: &Shell, settings_route: SettingsRoute, cx: &mut TestAppContext) {
    route(shell, cx, move |workspace, window, cx| {
        workspace.show_settings(settings_route, None, window, cx)
    });
}

/// Reads where Settings landed for row `row`: its page, the control holding
/// focus after two draws, and the repository it works on. Settings stays
/// open.
pub(super) fn land(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    wanted: &Landing,
    cx: &mut TestAppContext,
) {
    assert!(settings_open(shell, cx), "{row}: Settings opened");
    let page = shown(shell, cx);
    table.fact(
        &format!("{row} (its page)"),
        page == Shown::Settings(wanted.page),
        &format!("Settings({:?}), not {page:?},", wanted.page),
    );
    if wanted.page == SettingsPage::Keys {
        shell.keys_landed(cx);
    }
    let held = if wanted.control {
        shell.holds_control(wanted.focus.clone(), cx)
    } else {
        holds(shell, wanted.focus.clone(), cx)
    };
    table.row(shell, row, held, &format!("{:?}", wanted.focus), cx);
    let chosen = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_chosen(cx));
    table.fact(
        &format!("{row} (its repository)"),
        chosen == wanted.chosen,
        &format!("chosen {:?}, not {chosen:?},", wanted.chosen),
    );
}

/// `land`, then Escape.
pub(super) fn lands(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    wanted: Landing,
    cx: &mut TestAppContext,
) {
    land(table, shell, row, &wanted, cx);
    escape(shell, row, cx);
}

/// The menu shows entry `ix` selected, and the page's entry, where focus
/// falls back to, is entry `ix` (Task 5 review, Minor 4). Moves focus.
fn entry_is(table: &mut Table, shell: &Shell, row: &str, ix: usize, cx: &mut TestAppContext) {
    let selected: Vec<bool> = (0..3usize)
        .map(|at| shell.selected(entry(at), cx))
        .collect();
    let mut only = [false; 3];
    only[ix] = true;
    table.fact(
        &format!("{row} (the menu)"),
        selected == only,
        &format!("only entry {ix} selected, not {selected:?},"),
    );
    let host = host(shell, cx);
    shell.frame(cx, |window, cx| {
        if let Some(first) = host.read(cx).first_focus() {
            window.focus(&first, cx);
        }
    });
    let held = holds(shell, entry(ix), cx);
    table.row(
        shell,
        &format!("{row} (the page's entry)"),
        held,
        &format!("{:?}", entry(ix)),
        cx,
    );
}

/// Puts focus on the shown page's menu entry, as a person can.
fn focus_entry(shell: &Shell, cx: &mut TestAppContext) {
    let host = host(shell, cx);
    shell.frame(cx, |window, cx| {
        let entry = host.read(cx).first_focus();
        window.focus(&entry.expect("the page's entry"), cx);
    });
}

/// Row `row`, Enter pressed where a route landed: the control does what
/// `acted` reads, as its click does.
pub(super) fn enter_acts(
    table: &mut Table,
    shell: &Shell,
    row: &str,
    acted: impl FnOnce(&Shell, &mut TestAppContext) -> bool,
    cx: &mut TestAppContext,
) {
    shell.press("enter", cx);
    let done = acted(shell, cx);
    table.fact(
        &format!("{row} (Enter)"),
        done,
        "Enter pressing the control",
    );
}

/// Make a new key's Generate form is shown (C2.2).
pub(super) fn generate_shown(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("key-generate-heading", cx)
}

/// The Hosts page's latest Check host, if any.
fn observing(shell: &Shell, cx: &mut TestAppContext) -> Option<DesktopRequestId> {
    let host = host(shell, cx);
    let page = shell
        .visual(cx)
        .update(|window, cx| host.update(cx, |host, cx| host.hosts(window, cx)));
    page.read_with(cx, |page, _| page.observing_id())
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let rig = rig(cx);
    keys(table, &rig, cx);
    pages(table, &rig, cx);
    fields(table, &rig, cx);
    // The worker must not outlive the test: the check that `CheckHost`
    // started ends first.
    let host = host(&rig.shell, cx);
    let page = rig
        .shell
        .visual(cx)
        .update(|window, cx| host.update(cx, |host, cx| host.hosts(window, cx)));
    rig.shell
        .until_drawn(cx, "Check host's answer", |_, cx| {
            page.read_with(cx, |page, _| page.observed_code().is_some())
        })
        .await;
}

/// R-22: the key list, before the Keys page was ever made.
fn keys(table: &mut Table, rig: &Rig, cx: &mut TestAppContext) {
    let shell = &rig.shell;
    let listed = shell.fixture.service().list_keys().unwrap().items;
    let id = |label: &str| {
        let key = listed
            .iter()
            .find(|key| key.label == label)
            .expect("listed");
        SharedKeyId::parse(&key.id).unwrap()
    };
    let (k1, k2) = (id("k1"), id("k2"));

    // While the keys are read, the entry; once the read lands, the radio.
    // The page reads them when it is made and again as it is shown.
    let held = [
        shell.backend.hold_read(ReadKind::Keys),
        shell.backend.hold_read(ReadKind::Keys),
    ];
    raise(shell, SettingsRoute::Keys, cx);
    let row = "F2 Keys while the keys are still read (a field not drawn)";
    land(
        table,
        shell,
        row,
        &Landing::new(SettingsPage::Keys, entry(1), None),
        cx,
    );
    drop(held);
    shell.keys_landed(cx);
    let held = holds(shell, ("key-radio", 0usize), cx);
    table.row(
        shell,
        "R-22 Keys, once the read lands: the first key's radio, none in use",
        held,
        "(key-radio, 0), k1's",
        cx,
    );
    // Once: with focus put back on the entry, a later read of the keys, as
    // a key command's finishing starts, moves it no more.
    focus_entry(shell, cx);
    let clear = answered("key clear", ResultCode::Ok);
    fabricate(shell, 2, Intent::KeyClear, clear, cx);
    shell.keys_landed(cx);
    let held = holds(shell, entry(1), cx);
    table.row(
        shell,
        "R-22 Keys, a second read after the retarget: focus moves once",
        held,
        "(settings-page, 1), where it was put back",
        cx,
    );
    escape(shell, row, cx);

    // The page reads the keys again on every open, and its last list is
    // drawn meanwhile. The one retarget waits for focus left where the
    // route put it: moved before the read lands, it stays, though a second
    // actor put `k2` in use meanwhile.
    let read = shell.backend.hold_read(ReadKind::Keys);
    raise(shell, SettingsRoute::Keys, cx);
    let host = host(shell, cx);
    shell.frame(cx, |window, cx| {
        let entry = host.read(cx).first_focus();
        window.focus(&entry.expect("the page's entry"), cx);
    });
    select_key_through(shell.fixture.service(), k2);
    drop(read);
    shell.keys_landed(cx);
    let held = holds(shell, entry(1), cx);
    table.row(
        shell,
        "R-22 Keys, focus moved before the read lands: it stays",
        held,
        "(settings-page, 1), where it was moved",
        cx,
    );
    escape(shell, "R-22 focus moved", cx);

    // The key in use: its radio (`k2`, the second).
    raise(shell, SettingsRoute::Keys, cx);
    lands(
        table,
        shell,
        "F1 Keys, a key in use: its radio",
        Landing::new(SettingsPage::Keys, ("key-radio", 1usize), None),
        cx,
    );

    // A second actor took `k2` out of use since the page's last read: the
    // read that lands moves focus once, to the radio it gives, the first.
    clear_key_through(shell.fixture.service());
    raise(shell, SettingsRoute::Keys, cx);
    lands(
        table,
        shell,
        "R-22 Keys, the key in use changed since the last read: the read's radio",
        Landing::new(SettingsPage::Keys, ("key-radio", 0usize), None),
        cx,
    );

    // No key, once the read lands: Make a new key, the Kit button itself,
    // which Enter presses.
    unregister_key_through(shell.fixture.service(), k1);
    unregister_key_through(shell.fixture.service(), k2);
    raise(shell, SettingsRoute::Keys, cx);
    let row = "F1 Keys, no key: Make a new key";
    land(
        table,
        shell,
        row,
        &Landing::control(SettingsPage::Keys, MAKE_A_NEW_KEY, None),
        cx,
    );
    enter_acts(table, shell, row, generate_shown, cx);
    escape(shell, row, cx);
}

/// F2: the routes that name no field land on the page's entry; R-14: one
/// naming no repository follows a panel selection made since the last
/// open, and only then.
fn pages(table: &mut Table, rig: &Rig, cx: &mut TestAppContext) {
    let shell = &rig.shell;
    let [a, b, _] = [0, 1, 2].map(|ix| rig.roots[ix].clone());
    shell.select(&b, cx);
    raise(shell, SettingsRoute::Page(SettingsPage::Repositories), cx);
    lands(
        table,
        shell,
        "F2 Page(Repositories), the panel's selection changed since (R-14)",
        Landing::new(SettingsPage::Repositories, entry(0), Some(&b)),
        cx,
    );
    raise(shell, SettingsRoute::Manage(a.clone()), cx);
    lands(
        table,
        shell,
        "F2 Manage(root)",
        Landing::new(SettingsPage::Repositories, entry(0), Some(&a)),
        cx,
    );
    raise(shell, SettingsRoute::Page(SettingsPage::Repositories), cx);
    lands(
        table,
        shell,
        "F2 Page(Repositories), the panel's selection unchanged (R-14)",
        Landing::new(SettingsPage::Repositories, entry(0), Some(&a)),
        cx,
    );
    raise(shell, SettingsRoute::Page(SettingsPage::Polling), cx);
    lands(
        table,
        shell,
        "F2 Page(Polling)",
        Landing::new(SettingsPage::Polling, entry(2), Some(&a)),
        cx,
    );
}

/// F1: the routes that name a field land on it (R-24).
fn fields(table: &mut Table, rig: &Rig, cx: &mut TestAppContext) {
    let shell = &rig.shell;
    let [a, b, c] = [0, 1, 2].map(|ix| rig.roots[ix].clone());
    // E-12 (Wave 04 Cycle 05a R-6): the route the Add repository sheet
    // returns by lands on Add or create a repository…, the Kit button
    // itself. The Add folder's routes left Settings with the forms (R-5);
    // Inspect's R-55 row is now the Add sheet's (P4, E-17c).
    raise(shell, SettingsRoute::AddButton, cx);
    lands(
        table,
        shell,
        "F1 AddButton: Add or create a repository…",
        Landing::control(SettingsPage::Repositories, ADD_BUTTON, Some(&a)),
        cx,
    );

    // From here the panel's selection is `c`.
    shell.select(&c, cx);
    raise(shell, SettingsRoute::Identity(a.clone()), cx);
    lands(
        table,
        shell,
        "F1 Identity(root)",
        Landing::new(SettingsPage::Repositories, "identity-name", Some(&a)),
        cx,
    );

    let row = "F1 Remotes(root): Sharing's add-remote Name";
    raise(shell, SettingsRoute::Remotes(b.clone()), cx);
    land(
        table,
        shell,
        row,
        &Landing::new(SettingsPage::Remotes, "remote-add-name", Some(&b)),
        cx,
    );
    entry_is(table, shell, row, 0, cx);
    escape(shell, row, cx);

    // The panel's selection is `c`; the route's repository is chosen (R-23).
    let row = "F1 Hosts(root), the panel's selection on another repository";
    raise(shell, SettingsRoute::Hosts(a.clone()), cx);
    land(
        table,
        shell,
        row,
        &Landing::control(SettingsPage::Hosts, CHECK_HOST, Some(&a)),
        cx,
    );
    entry_is(table, shell, row, 1, cx);
    escape(shell, row, cx);
    raise(shell, SettingsRoute::Hosts(b.clone()), cx);
    lands(
        table,
        shell,
        "F1 Hosts(root), no shared copy over SSH: Check host, unavailable (R-53)",
        Landing::new(SettingsPage::Hosts, "hosts-check", Some(&b)),
        cx,
    );

    // While `a`'s shared copy is read, Check host is drawn unavailable and
    // focus is on it (R-53). Once the read lands it is the control itself,
    // focus not having moved, and Enter checks.
    let held = [(); 4].map(|_| shell.backend.hold_read(ReadKind::Remotes));
    raise(shell, SettingsRoute::Hosts(a.clone()), cx);
    let row = "F1 Hosts(root) while its shared copy is read: Check host, unavailable (R-53)";
    land(
        table,
        shell,
        row,
        &Landing::new(SettingsPage::Hosts, "hosts-check", Some(&a)),
        cx,
    );
    let reading = shell.present("hosts-check-state", cx);
    table.fact(
        &format!("{row} (read)"),
        reading,
        "its shared copy still read",
    );
    drop(held);
    shell.run(cx);
    let row = "R-53 Hosts(root), once its shared copy is read: Check host itself";
    let offered = !shell.present("hosts-check-state", cx);
    table.fact(&format!("{row} (offered)"), offered, "Check host offered");
    let before = observing(shell, cx);
    let held = shell.holds_control(CHECK_HOST, cx);
    table.row(shell, row, held, CHECK_HOST, cx);
    enter_acts(
        table,
        shell,
        row,
        |shell, cx| observing(shell, cx).is_some_and(|now| Some(now) != before),
        cx,
    );
    escape(shell, row, cx);

    let authority = SshAuthority {
        host: "example.invalid".to_owned(),
        port: 22,
    };
    raise(shell, SettingsRoute::CheckHost(a.clone(), authority), cx);
    lands(
        table,
        shell,
        "F1 CheckHost(root, authority)",
        Landing::new(SettingsPage::Hosts, "hosts-check", Some(&a)),
        cx,
    );

    // A protected generation that a retry asks the passphrase of again.
    let retry = fabricate(
        shell,
        1,
        Intent::KeyGenerate {
            label: "locked".to_owned(),
            protected: true,
        },
        answered("key generate", ResultCode::Busy),
        cx,
    );
    raise(shell, SettingsRoute::RetryGeneration(retry), cx);
    lands(
        table,
        shell,
        "F1 RetryGeneration(id)",
        Landing::new(SettingsPage::Keys, "key-generate-passphrase", Some(&a)),
        cx,
    );
}
