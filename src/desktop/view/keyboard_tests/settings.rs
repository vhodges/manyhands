//! One command per settings page, by keyboard alone: add an existing
//! repository by typing its path, add a remote, generate a key, and
//! check a host to its typed failure (no desktop test reaches a network,
//! ruling R11).

use gpui_kit::TestAppContext;
use manyhands::results::{Outcome, ResultCode};

use super::Walk;
use crate::desktop::{
    services::requests::{Intent, Lane, RequestState},
    test_support::fixtures::{Fixture, identity, remote_names},
    view::{
        harness::{Shell, key},
        settings::{PAGES, Shown},
        workspace::Region,
    },
};

/// `secondary-,`, then Tab to the page list's entry `ix` and Enter: the
/// page is shown, with focus on its entry.
fn open_page(walk: &Walk, ix: usize, cx: &mut TestAppContext) {
    walk.keys("secondary-,", cx);
    walk.on(("settings-page", 0usize), Region::Content, cx);
    if ix > 0 {
        walk.tab_to("tab", ("settings-page", ix), cx);
        walk.enter(cx);
    }
    assert_eq!(walk.settings_shown(cx), Shown::Settings(PAGES[ix]));
    walk.on(("settings-page", ix), Region::Content, cx);
}

/// Tab to the page's `submit` control and press it.
fn submit(walk: &Walk, id: &'static str, cx: &mut TestAppContext) {
    walk.tab_to("tab", id, cx);
    walk.on(id, Region::Content, cx);
    walk.enter(cx);
}

/// A window over the repository `a`, selected by keyboard.
fn selected(fixture: Fixture, cx: &mut TestAppContext) -> Walk {
    let a = fixture.create_repository("a");
    fixture.refresh(&a);
    let walk = Walk::over(Shell::open(fixture, cx), a);
    walk.select_a_repository(cx);
    walk
}

/// Repositories: the path of an existing repository typed into Add, Add
/// pressed, and the preview confirmed by Enter registers it.
#[gpui_kit::test]
async fn add_an_existing_repository_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let plain = fixture.plain_repository("plain", Some(identity()));
    let walk = Walk::over(Shell::open(fixture, cx), plain.clone());
    open_page(&walk, 0, cx);
    walk.type_into("add-path", plain.to_str().unwrap(), Region::Content, cx);
    submit(&walk, "add-submit", cx);
    assert!(walk.shell.present("confirm-ok", cx));
    walk.keys("enter", cx);
    let ended = walk
        .ended(|intent| matches!(intent, Intent::RepoEnable { .. }), cx)
        .await;
    assert_eq!(ended, Some((Outcome::Success, ResultCode::Ok)));
    let wanted = plain.clone();
    walk.shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                request.lane == Lane::Background
                    && matches!(&request.intent, Intent::IndexRefresh { root } if *root == wanted)
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    let listed = walk.shell.fixture.service().list_repositories().unwrap();
    assert!(listed.items.iter().any(|item| item.root == key(&plain)));
    walk.region(Region::Content, cx);
}

/// Remotes: the repository chosen by its Manage on the Repositories page,
/// then a name and a URL typed under Sharing, Add pressed: the remote is
/// added to the chosen repository (R-15).
#[gpui_kit::test]
async fn add_a_remote_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.refresh(&a);
    let walk = Walk::over(Shell::open(fixture, cx), a);
    open_page(&walk, 0, cx);
    let manage = ("managed-choose", 0usize);
    walk.tab_to("tab", manage, cx);
    walk.on(manage, Region::Content, cx);
    walk.enter(cx);
    walk.type_into("remote-add-name", "backup", Region::Content, cx);
    walk.type_into(
        "remote-add-url",
        "ssh://git@example.invalid/team/backup.git",
        Region::Content,
        cx,
    );
    submit(&walk, "remote-add-submit", cx);
    let ended = walk
        .ended(|intent| matches!(intent, Intent::RemoteAdd { .. }), cx)
        .await;
    assert_eq!(ended, Some((Outcome::Success, ResultCode::Ok)));
    assert!(remote_names(&walk.a).contains(&"backup".to_owned()));
    walk.region(Region::Content, cx);
}

/// Shared SSH keys: a label typed and Generate pressed, with no
/// passphrase: the key is generated and listed.
#[gpui_kit::test]
async fn generate_a_key_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = selected(Fixture::new(), cx);
    open_page(&walk, 2, cx);
    walk.type_into("key-generate-label", "keyboard", Region::Content, cx);
    submit(&walk, "key-generate-submit", cx);
    let ended = walk
        .ended(|intent| matches!(intent, Intent::KeyGenerate { .. }), cx)
        .await;
    assert_eq!(ended, Some((Outcome::Success, ResultCode::Ok)));
    walk.until_drawn(("key-label", 0usize), cx).await;
    assert_eq!(
        walk.shell.label(("key-label", 0usize), cx).as_deref(),
        Some("keyboard")
    );
    walk.region(Region::Content, cx);
}

/// Hosts: Check host on a repository that publishes over SSH, pressed by
/// Enter, ends in its typed failure: no transport here.
#[gpui_kit::test]
async fn check_a_host_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    let walk = Walk::over(Shell::open(fixture, cx), a);
    walk.select_a_repository(cx);
    open_page(&walk, 3, cx);
    submit(&walk, "hosts-check", cx);
    walk.until_drawn("hosts-observation-code", cx).await;
    let code = walk.shell.label("hosts-observation-code", cx).unwrap();
    assert_ne!(code, ResultCode::Ok.as_str());
    walk.region(Region::Content, cx);
}

/// Checking for updates: the interval, Check now and the switch are each
/// reached by Tab, show focus on screen, and are operated by the keyboard
/// alone. The interval opens with Enter, Down moves and Enter picks; Check
/// now is Enter; the switch is Space. Each sends its request on the user
/// lane (D6 contract 9, E-2).
#[gpui_kit::test]
async fn checking_for_updates_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let root = key(&a);
    services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().add(
            root,
            crate::desktop::view::settings::polling_support::policy(),
            None,
        );
        cx.notify();
    });
    let walk = Walk::over(shell, a.clone());
    walk.select_a_repository(cx);
    open_page(&walk, 4, cx);
    walk.until_drawn(("polling-name", 0usize), cx).await;

    let sent = |pick: fn(&Intent) -> bool, cx: &mut TestAppContext| {
        walk.shell.requests(cx, |request| {
            request.lane == Lane::User && pick(&request.intent)
        })
    };
    // The interval: Enter opens it, Down moves to Every 10 minutes, Enter
    // picks it.
    walk.tab_to("tab", ("polling-interval", 0usize), cx);
    walk.on(("polling-interval", 0usize), Region::Content, cx);
    walk.enter(cx);
    walk.keys("down", cx);
    walk.enter(cx);
    walk.until_drawn(("polling-interval", 0usize), cx).await;
    let picked = sent(
        |intent| {
            matches!(
                intent,
                Intent::PollConfigure {
                    interval_seconds: 600,
                    ..
                }
            )
        },
        cx,
    );
    assert_eq!(picked.len(), 1, "Every 10 minutes");

    // Check now: Enter.
    walk.tab_to("tab", ("polling-check", 0usize), cx);
    walk.on(("polling-check", 0usize), Region::Content, cx);
    walk.enter(cx);
    assert_eq!(sent(|i| matches!(i, Intent::PollOnce { .. }), cx).len(), 1);

    // The switch: Space turns it off, which pauses.
    walk.tab_to("tab", ("polling-switch", 0usize), cx);
    walk.on(("polling-switch", 0usize), Region::Content, cx);
    walk.shell.press("space", cx);
    assert_eq!(sent(|i| matches!(i, Intent::PollPause { .. }), cx).len(), 1);
    walk.region(Region::Content, cx);
}
