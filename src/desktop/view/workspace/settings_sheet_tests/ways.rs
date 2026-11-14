//! P4, `each_route_lands_on_its_page_and_control`: the ways into Settings
//! no other test raises (design Part D: W-9, W-14, W-16 to W-20, W-25 and
//! W-33 to W-35, W-32's Approve host… with no server named; Task 0's
//! W-36), each through its own handler, as its trigger calls it:
//! `row_action`, `open_route`, `attend`, `choose_activity` for an Activity
//! row, or the Checking for updates page's own button inside the sheet.
//! Each lands as `routes` reads it. Approve host… with no server named
//! works on its own row's repository, not the Repositories panel's
//! selection (decision 9; R-23).

use gpui_kit::TestAppContext;
use manyhands::repository::{OperationId, transport::SshAuthority};

use super::{
    routes::{Landing, entry, lands, raise, rig},
    support::{holds_handle, host, route},
    table::Table,
};
use crate::desktop::{
    model::{
        activity::{
            copy::SHARE_MAIN,
            rows::{RowKey, Target},
        },
        polling::{
            Answer, AttemptTarget, Clock, Disposition, PollAttempt, Reason, TransportFailure,
            attention::{APPROVE_HOST, Action},
        },
        repository_state::RowAction,
        stamp::SettingsPage,
    },
    view::{
        activity_support::{act, list_focus},
        harness::{Shell, key},
        item::ItemRoute,
        settings::{SettingsRoute, polling_support::Rig},
    },
};

/// Calls a repository row's action `action` on `root`, as its menu entry
/// or its Activity row does (`row_action`).
fn row_action(shell: &Shell, action: RowAction, root: &std::path::Path, cx: &mut TestAppContext) {
    let root = key(root);
    route(shell, cx, move |workspace, window, cx| {
        workspace.row_action(&action, root, window, cx)
    });
}

/// Opens the place an item's control asks for (`open_route`).
fn open_route(shell: &Shell, item_route: ItemRoute, cx: &mut TestAppContext) {
    route(shell, cx, move |workspace, window, cx| {
        workspace.open_route(item_route, window, cx)
    });
}

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    let rig = rig(cx);
    let shell = &rig.shell;
    let [a, b, c] = [0, 1, 2].map(|ix| rig.roots[ix].clone());
    let authority = SshAuthority {
        host: "example.invalid".to_owned(),
        port: 22,
    };

    row_action(shell, RowAction::SetUpSharing, &b, cx);
    lands(
        table,
        shell,
        "W-9 a repository row's Set up sharing…",
        Landing::new(SettingsPage::Remotes, "remote-add-name", Some(&b)),
        cx,
    );
    open_route(shell, ItemRoute::SetIdentity(a.clone()), cx);
    lands(
        table,
        shell,
        "W-14 a share outcome's Set identity",
        Landing::new(SettingsPage::Repositories, "identity-name", Some(&a)),
        cx,
    );
    // The Keys page is made here, so its keys are still read on arrival.
    open_route(shell, ItemRoute::SharingAndKeys, cx);
    lands(
        table,
        shell,
        "W-16 a comment offer's Open Sharing & keys: the first key's radio",
        Landing::new(SettingsPage::Keys, ("key-radio", 0usize), Some(&a)),
        cx,
    );

    // From here the Repositories panel's selection is `c`.
    shell.select(&c, cx);
    open_route(
        shell,
        ItemRoute::CheckHost {
            root: a.clone(),
            authority: None,
        },
        cx,
    );
    lands(
        table,
        shell,
        "W-17 a comment offer's Check host, no host named",
        Landing::new(SettingsPage::Hosts, "hosts-check", Some(&a)),
        cx,
    );
    open_route(shell, ItemRoute::SetIdentity(b.clone()), cx);
    lands(
        table,
        shell,
        "W-18 a comment offer's Set identity",
        Landing::new(SettingsPage::Repositories, "identity-name", Some(&b)),
        cx,
    );
    open_route(shell, ItemRoute::SetIdentity(a.clone()), cx);
    lands(
        table,
        shell,
        "W-19 the composer's Set identity",
        Landing::new(SettingsPage::Repositories, "identity-name", Some(&a)),
        cx,
    );
    open_route(
        shell,
        ItemRoute::CheckHost {
            root: a.clone(),
            authority: Some(authority),
        },
        cx,
    );
    lands(
        table,
        shell,
        "W-20 a lifecycle dialog's Check host, its host named",
        Landing::new(SettingsPage::Hosts, "hosts-check", Some(&a)),
        cx,
    );
    // The worker must not outlive the test: the check W-20 started ends
    // before another repository is chosen, which lets its answer go.
    let host = host(shell, cx);
    let page = shell
        .visual(cx)
        .update(|window, cx| host.update(cx, |host, cx| host.hosts(window, cx)));
    shell
        .until_drawn(cx, "Check host's answer", |_, cx| {
            page.read_with(cx, |page, _| page.observed_code().is_some())
        })
        .await;

    // W-33: a repository's attention cause, through `attend`.
    let reason = Reason::NoKeySelected(key(&c));
    row_action(shell, RowAction::Attend(Action::ChooseKey, reason), &c, cx);
    lands(
        table,
        shell,
        "W-33 a repository row's Choose a key…: the key list",
        Landing::new(SettingsPage::Keys, ("key-radio", 0usize), Some(&a)),
        cx,
    );
    let reason = Reason::HostUnidentified(key(&b));
    row_action(
        shell,
        RowAction::Attend(Action::ApproveHost, reason),
        &b,
        cx,
    );
    lands(
        table,
        shell,
        "W-33 a repository row's Approve host…, no server named: its repository (R-23)",
        Landing::new(SettingsPage::Hosts, "hosts-check", Some(&b)),
        cx,
    );
    let reason = Reason::ConfigurationRequired(key(&c));
    let action = RowAction::Attend(Action::OpenRemoteSettings, reason);
    row_action(shell, action, &c, cx);
    lands(
        table,
        shell,
        "W-33 a repository row's Open remote settings…",
        Landing::new(SettingsPage::Remotes, "remote-add-name", Some(&c)),
        cx,
    );
    // W-25: a hold's Approve host…, no server named, which `attend` runs.
    let (reason, root) = (Reason::HostUnidentified(key(&a)), key(&a));
    route(shell, cx, move |workspace, window, cx| {
        workspace.attend(Action::ApproveHost, &reason, &root, window, cx)
    });
    lands(
        table,
        shell,
        "W-25 a hold's Approve host…, no server named: its repository (R-23)",
        Landing::new(SettingsPage::Hosts, "hosts-check", Some(&a)),
        cx,
    );

    // W-34 and W-35: an Activity or popover state row's action.
    row_action(shell, RowAction::Settings, &b, cx);
    lands(
        table,
        shell,
        "W-34 a state row's Settings…",
        Landing::new(SettingsPage::Repositories, entry(0), Some(&b)),
        cx,
    );
    row_action(shell, RowAction::SetUpSharing, &c, cx);
    lands(
        table,
        shell,
        "W-35 a state row's Set up sharing…",
        Landing::new(SettingsPage::Remotes, "remote-add-name", Some(&c)),
        cx,
    );

    share_main(table, &rig, cx);
    inside(table, &rig, cx).await;
}

/// W-32, inside the sheet: Checking for updates' Approve host… on `a`,
/// whose poll answered that a host needs approval but named none. Check
/// host on `a`, the row's own repository, not the panel's selection, `c`
/// (decision 9; R-23).
async fn inside(table: &mut Table, rig: &Rig, cx: &mut TestAppContext) {
    let shell = &rig.shell;
    let a = rig.roots[0].clone();
    rig.schedule(0, cx);
    let unnamed = TransportFailure::HostUnapproved { authority: None };
    rig.answer(0, Answer::Transport(unnamed), cx);
    raise(shell, SettingsRoute::Page(SettingsPage::Polling), cx);
    shell
        .until_drawn(cx, "the page's rows", |shell, cx| {
            shell.present(("polling-name", 1usize), cx)
        })
        .await;
    let row = rig.row("a", cx);
    assert_eq!(
        shell.label(("polling-attention", row), cx).as_deref(),
        Some(APPROVE_HOST)
    );
    shell.click(("polling-attention", row), cx);
    lands(
        table,
        shell,
        "W-32 Checking for updates' Approve host…, no server named: its repository (R-23)",
        Landing::new(SettingsPage::Hosts, "hosts-check", Some(&a)),
        cx,
    );
}

/// W-36 (Task 0): Activity's Share main on `b`, which has no publication
/// remote when it is pressed, chosen through `choose_activity`: Sharing on
/// `b`, its add-remote Name. Escape gives focus back to the Activity list,
/// the origin `choose_activity` gives (`settings_origin`).
fn share_main(table: &mut Table, rig: &Rig, cx: &mut TestAppContext) {
    let shell = &rig.shell;
    let b = rig.roots[1].clone();
    rig.schedule(1, cx);
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: rig.key(1),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: vec![AttemptTarget {
            item_id: None,
            kind: None,
            disposition: Disposition::LocalAhead,
            main: true,
        }],
        index_pending: false,
    };
    rig.services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
    shell.run(cx);
    let main = RowKey::Unshared {
        root: rig.key(1),
        target: Target::Main,
    };
    act(shell, &main, SHARE_MAIN, cx);
    let row = "W-36 Activity's Share main, no publication remote at the press";
    lands(
        table,
        shell,
        row,
        Landing::new(SettingsPage::Remotes, "remote-add-name", Some(&b)),
        cx,
    );
    let list = list_focus(shell, cx);
    let held = holds_handle(shell, &list, cx);
    table.row(
        shell,
        &format!("{row}, closed: the Activity list"),
        held,
        "Activity's list, the origin",
        cx,
    );
}
