//! The header's Share and the repository detail's Share main (design,
//! Lane B, Share controls), over real repositories: Share follows Edit and
//! sends `item sync`, with the remote in its tooltip; with no publication
//! remote it reads Share… and opens Settings › Remotes; while a share of
//! the same target is queued or running it reads Sharing… and queues
//! nothing more; Share main sends `repo sync`.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::repository::{AuthoringKind, ItemDtoKind};

use super::{SHARE_BUTTON, SHARE_NOTE, SHARE_SET_UP, share_tooltip};
use crate::desktop::{
    model::{item::SHARE, navigation::OpenTarget, stamp::SettingsPage},
    services::{
        JobKind,
        requests::{DesktopRequest, Intent, Lane},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        settings::Shown,
        sidebar::{SHARE_MAIN, SHARE_MAIN_BUTTON, SHARE_MAIN_SET_UP, SHARING},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FS0";

/// Repository `a` holding one ticket, with `origin` selected for
/// publication when `remote`, indexed, in an open window.
fn shell(remote: bool, cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    if remote {
        fixture.publish_over_ssh(&root);
    }
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn open_ticket(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn focused(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether `request` is `item sync` of the ticket in `root`.
fn item_sync_of(request: &DesktopRequest, root: &Path) -> bool {
    matches!(
        &request.intent,
        Intent::ItemSync { root: asked, kind: AuthoringKind::Ticket, id }
            if asked == root && *id == item_id(TICKET)
    )
}

fn item_syncs(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> usize {
    let root = root.to_owned();
    shell
        .requests(cx, move |request| item_sync_of(request, &root))
        .len()
}

fn repo_syncs(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> usize {
    let root = root.to_owned();
    shell
        .requests(cx, move |request| {
            matches!(&request.intent, Intent::RepoSync { root: asked } if *asked == root)
        })
        .len()
}

/// With a publication remote, Share comes after Edit, in the tab order
/// too, names the remote in its tooltip, and sends `item sync` of the
/// item.
#[gpui_kit::test]
async fn share_follows_edit_when_a_remote_exists(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
    assert_eq!(
        shell.label(SHARE_NOTE, cx),
        Some(share_tooltip("origin")),
        "the tooltip names the remote"
    );

    shell.keys("secondary-3", cx);
    let mut presses = 0;
    while !focused(&shell, "item-edit", cx) {
        presses += 1;
        assert!(presses < 6, "Tab never reached Edit");
        shell.keys("tab", cx);
    }
    shell.keys("tab", cx);
    assert!(focused(&shell, SHARE_BUTTON, cx), "Share after Edit");

    let held = shell.backend.hold_job(JobKind::SynchronizeItem);
    shell.click(SHARE_BUTTON, cx);
    held.wait_reached();
    let sent = shell.requests(cx, |request| item_sync_of(request, &root));
    assert_eq!(sent.len(), 1, "Share sent item sync of the item");
    held.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
}

/// With no publication remote, the header reads Share…, and pressing it
/// opens Settings › Remotes, sending nothing.
#[gpui_kit::test]
fn share_without_a_remote_reads_share_and_opens_remotes(cx: &mut TestAppContext) {
    let (shell, root) = shell(false, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE_SET_UP));
    assert!(!shell.present(SHARE_NOTE, cx), "no remote to name");

    shell.click(SHARE_BUTTON, cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert!(matches!(shown, Shown::Settings(SettingsPage::Remotes)));
    assert_eq!(item_syncs(&shell, &root, cx), 0, "nothing was sent");
}

/// A remote selected for publication through the request path, as the
/// Remotes page selects one, reaches Share once the request finishes:
/// navigation reloads and gives the tab the remote, with no read of its
/// own.
#[gpui_kit::test]
async fn a_remote_selected_by_request_reaches_share(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Shared", "");
    fixture.add_remote(&root, "origin", "ssh://git@example.invalid/team/repo.git");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open_ticket(&shell, &root, cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE_SET_UP));

    let id = shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::RemoteSelect {
                root: root.clone(),
                name: Some("origin".to_owned()),
            },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(id, cx).await;
    shell.run(cx);
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
    assert_eq!(shell.label(SHARE_NOTE, cx), Some(share_tooltip("origin")));
}

/// While a share of the item is queued or running, Share reads Sharing…
/// and is disabled: a second press, or the `ShareItem` action, queues
/// nothing.
#[gpui_kit::test]
async fn a_second_press_while_sharing_queues_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    open_ticket(&shell, &root, cx);
    let held = shell.backend.hold_job(JobKind::SynchronizeItem);
    shell.click(SHARE_BUTTON, cx);
    held.wait_reached();
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARING));

    shell.click(SHARE_BUTTON, cx);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("the ticket's tab is shown");
    let taken = shell
        .visual(cx)
        .update(|window, cx| pane.update(cx, |pane, cx| pane.share_item(window, cx)));
    assert!(taken, "the action is Share's, though it queues nothing");
    shell.run(cx);
    let sent = shell.requests(cx, |request| item_sync_of(request, &root));
    assert_eq!(sent.len(), 1, "one share queued");

    held.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(shell.label(SHARE_BUTTON, cx).as_deref(), Some(SHARE));
}

/// The repository detail offers Share main, which sends `repo sync` of
/// the repository; while it is running it reads Sharing… and queues
/// nothing more.
#[gpui_kit::test]
async fn share_main_in_the_repository_detail_submits_repo_sync(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    shell.select(&root, cx);
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN)
    );

    let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
    shell.click(SHARE_MAIN_BUTTON, cx);
    held.wait_reached();
    assert_eq!(
        repo_syncs(&shell, &root, cx),
        1,
        "Share main sent repo sync"
    );
    assert_eq!(shell.label(SHARE_MAIN_BUTTON, cx).as_deref(), Some(SHARING));
    // The button is disabled, so a press reaches nothing: Share main's own
    // guard is asked directly, as a press made before the frame is.
    let workspace = shell.workspace.clone();
    let repo = key(&root);
    shell.visual(cx).update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.share_main(repo, true, window, cx)
        })
    });
    shell.run(cx);
    assert_eq!(
        repo_syncs(&shell, &root, cx),
        1,
        "a second press queued nothing"
    );

    let sent = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    });
    held.release();
    shell.until_ended(sent[0], cx).await;
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN)
    );
}

/// With no publication remote, the repository detail reads Share main…,
/// and pressing it opens Settings › Remotes, sending nothing.
#[gpui_kit::test]
fn share_main_without_a_remote_reads_share_main_and_opens_remotes(cx: &mut TestAppContext) {
    let (shell, root) = shell(false, cx);
    shell.select(&root, cx);
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN_SET_UP)
    );

    shell.click(SHARE_MAIN_BUTTON, cx);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx));
    assert!(matches!(shown, Shown::Settings(SettingsPage::Remotes)));
    assert_eq!(repo_syncs(&shell, &root, cx), 0, "nothing was sent");
}

/// While an item tab is shown, the repository detail at the end of its
/// scroll region offers Share main too, and it sends `repo sync`.
#[gpui_kit::test]
async fn share_main_at_the_end_of_an_item_tab_submits_repo_sync(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(true, cx);
    shell.select(&root, cx);
    open_ticket(&shell, &root, cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())
            .is_some(),
        "the ticket's tab is shown"
    );
    // The detail ends the item's scroll region: End brings it on screen.
    shell.keys("secondary-3", cx);
    shell.keys("end", cx);
    assert_eq!(
        shell.label(SHARE_MAIN_BUTTON, cx).as_deref(),
        Some(SHARE_MAIN)
    );

    let held = shell.backend.hold_job(JobKind::SynchronizeRepo);
    shell.click(SHARE_MAIN_BUTTON, cx);
    held.wait_reached();
    assert_eq!(
        repo_syncs(&shell, &root, cx),
        1,
        "Share main sent repo sync"
    );
    assert_eq!(shell.label(SHARE_MAIN_BUTTON, cx).as_deref(), Some(SHARING));
    assert_eq!(item_syncs(&shell, &root, cx), 0, "not the item's share");

    let sent = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoSync { .. })
    });
    held.release();
    shell.until_ended(sent[0], cx).await;
}
