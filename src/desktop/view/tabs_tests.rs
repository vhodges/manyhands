//! Tabs over real repositories: one tab per repository and item, the
//! keys, closing, reloading on refresh, a repository that goes away, and
//! completions that land on a tab no longer shown.

use std::{fs, path::PathBuf};

use gpui_kit::{Entity, TestAppContext, component::input::TextareaState, test::TestWindowExt};
use manyhands::{repository::ItemDtoKind, results::ResultCode};

use super::{CLOSE_TAB, SHOWN};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{
        ReadKind,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        workspace::{REGIONS, Region},
    },
};

const ONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ0";
const TWO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ1";
const THREE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ2";
const SHARED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ3";
const STRANDED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ4";

/// Repositories `a` and `b`: `a` holds One, Two and Three, `b` holds
/// Other, and both hold a ticket with the ID `SHARED`.
struct Two {
    shell: Shell,
    a: PathBuf,
    b: PathBuf,
}

fn two(cx: &mut TestAppContext) -> Two {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.commit_ticket(&a, ONE, "One", "");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.commit_ticket(&a, THREE, "Three", "");
    fixture.commit_ticket(&a, SHARED, "Shared in a", "");
    fixture.commit_ticket(&b, ONE, "Other", "");
    fixture.commit_ticket(&b, SHARED, "Shared in b", "");
    fixture.refresh(&a);
    fixture.refresh(&b);
    Two {
        shell: Shell::open(fixture, cx),
        a,
        b,
    }
}

fn ticket(root: &std::path::Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

/// The tab strip's labels, in order.
fn strip(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.rows("tab", cx)
}

/// The position of the shown tab, by its element and by its text.
fn shown(shell: &Shell, cx: &mut TestAppContext) -> Option<usize> {
    let count = strip(shell, cx).len();
    let selected: Vec<usize> = (0..count)
        .filter(|ix| shell.selected(("tab", *ix), cx))
        .collect();
    let marked: Vec<usize> = (0..count)
        .filter(|ix| shell.label(("tab-shown", *ix), cx).as_deref() == Some(SHOWN))
        .collect();
    assert_eq!(selected, marked, "the shown tab says so in text too");
    assert!(selected.len() <= 1);
    selected.first().copied()
}

fn title(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("item-title", cx)
}

/// Whether the tab at `ix` holds focus.
fn tab_focused(shell: &Shell, ix: usize, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window.try_find(("tab", ix)).and_then(|tab| tab.focused()) == Some(true)
    })
}

/// The one region drawing its focus ring.
fn ring(shell: &Shell, cx: &mut TestAppContext) -> Vec<Region> {
    REGIONS
        .into_iter()
        .filter(|region| shell.present(region.ring_id(), cx))
        .collect()
}

fn item_reads(shell: &Shell) -> usize {
    shell.backend.reads_seen(ReadKind::Item)
}

fn submit_refresh(
    shell: &Shell,
    root: &std::path::Path,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::IndexRefresh {
                root: root.to_path_buf(),
            },
            Lane::User,
            None,
            None,
        )
    })
}

fn source_state(shell: &Shell, cx: &mut TestAppContext) -> Option<Entity<TextareaState>> {
    shell.frame(cx, |_, _| ());
    shell.workspace.read_with(cx, |workspace, cx| {
        workspace.item_pane()?.read(cx).source_state().cloned()
    })
}

#[gpui_kit::test]
fn opening_an_open_item_shows_its_tab_and_opens_no_second(cx: &mut TestAppContext) {
    let Two { shell, a, .. } = two(cx);
    assert!(strip(&shell, cx).is_empty());
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    assert_eq!(strip(&shell, cx), ["One", "Two"]);
    assert_eq!(shown(&shell, cx), Some(1));
    assert_eq!(title(&shell, cx).as_deref(), Some("Two"));

    let reads = item_reads(&shell);
    open(&shell, ticket(&a, ONE), cx);
    assert_eq!(strip(&shell, cx), ["One", "Two"]);
    assert_eq!(shown(&shell, cx), Some(0));
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
    assert_eq!(item_reads(&shell), reads, "focusing a tab reads nothing");
}

/// Two repositories hold the same ID: each opens a tab of its own, and
/// each tab reads its own repository.
#[gpui_kit::test]
fn the_same_id_in_two_repositories_opens_two_tabs(cx: &mut TestAppContext) {
    let Two { shell, a, b } = two(cx);
    open(&shell, ticket(&a, SHARED), cx);
    open(&shell, ticket(&b, SHARED), cx);
    assert_eq!(strip(&shell, cx), ["Shared in a", "Shared in b"]);
    assert_eq!(title(&shell, cx).as_deref(), Some("Shared in b"));
    shell.click(("tab", 0usize), cx);
    assert_eq!(shown(&shell, cx), Some(0));
    assert_eq!(title(&shell, cx).as_deref(), Some("Shared in a"));
}

/// Opening a tab of `a` while `b` is selected leaves `b` selected.
#[gpui_kit::test]
fn opening_a_tab_does_not_change_the_selection(cx: &mut TestAppContext) {
    let Two { shell, a, b } = two(cx);
    shell.select(&b, cx);
    open(&shell, ticket(&a, ONE), cx);
    let selected = shell.workspace.read_with(cx, |workspace, _| {
        workspace.navigation().selected().cloned()
    });
    assert_eq!(selected, Some(key(&b)));
    assert_eq!(shell.label("selection-name", cx).as_deref(), Some("b"));
}

/// Refresh of `a` reads every open tab of `a` again, once, in place: the
/// strip keeps its order and the shown tab, `b`'s tab is not read, and the
/// shown tab's source keeps its text field and its focus.
#[gpui_kit::test]
async fn a_refresh_reloads_every_open_tab_of_the_repository_in_place(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let Two { shell, a, b } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&b, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    shell.click(("tab", 0usize), cx);
    assert_eq!(strip(&shell, cx), ["One", "Other", "Two"]);

    let source = source_state(&shell, cx).expect("the source is shown");
    let mut visual = shell.visual(cx);
    visual.update(|window, cx| source.update(cx, |state, cx| state.focus(window, cx)));
    visual.run_until_parked();

    shell.fixture.commit_ticket(&a, ONE, "One again", "");
    shell.fixture.commit_ticket(&a, TWO, "Two again", "");
    shell.fixture.commit_ticket(&b, ONE, "Other again", "");
    shell.select(&a, cx);
    let reads = item_reads(&shell);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| request.lane == Lane::User)[0];
    shell.until_ended(refresh, cx).await;
    shell.run(cx);

    assert_eq!(strip(&shell, cx), ["One again", "Other", "Two again"]);
    assert_eq!(shown(&shell, cx), Some(0));
    assert_eq!(title(&shell, cx).as_deref(), Some("One again"));
    assert_eq!(item_reads(&shell) - reads, 2, "one read per tab of a");
    let after = source_state(&shell, cx).expect("the source is shown");
    assert_eq!(after, source, "the same text field");
    assert!(shell.visual(cx).update(|window, cx| {
        gpui_kit::Focusable::focus_handle(after.read(cx), cx).is_focused(window)
    }));
}

/// Review Focus 1 (jobs): a job for `a` finishes while `b` is selected.
/// `b`'s lists and its open tab are not read again; `a`'s tab is.
#[gpui_kit::test]
async fn a_job_for_a_while_b_is_selected_reloads_only_as_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let Two { shell, a, b } = two(cx);
    shell.click("pane-tickets", cx);
    shell.select(&b, cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&b, ONE), cx);
    let tickets = shell.rows("ticket-row", cx);
    assert!(!tickets.is_empty());
    let lists = |shell: &Shell| {
        [ReadKind::Tickets, ReadKind::Documents, ReadKind::Folders]
            .map(|kind| shell.backend.reads_seen(kind))
    };
    let lists_before = lists(&shell);
    let reads = item_reads(&shell);

    shell.fixture.commit_ticket(&a, ONE, "One again", "");
    let refresh = submit_refresh(&shell, &a, cx);
    shell.until_ended(refresh, cx).await;
    shell.run(cx);

    let selected = shell.workspace.read_with(cx, |workspace, _| {
        workspace.navigation().selected().cloned()
    });
    assert_eq!(selected, Some(key(&b)));
    assert_eq!(lists(&shell), lists_before, "b's lists are not read again");
    assert_eq!(shell.rows("ticket-row", cx), tickets);
    assert_eq!(item_reads(&shell) - reads, 1, "only a's tab is read");
    assert_eq!(strip(&shell, cx), ["One again", "Other"]);
    assert_eq!(shown(&shell, cx), Some(1));
    assert_eq!(title(&shell, cx).as_deref(), Some("Other"));
}

/// Review Focus 2: `a`'s folder goes away while it has a tab open. Once
/// `a` is read again its tab shows the failure with Retry; `b`'s tab is
/// unaffected. With the folder back, Retry reads it.
#[gpui_kit::test]
async fn a_tab_whose_repository_goes_away_shows_its_failure_with_retry(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let Two { shell, a, b } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&b, ONE), cx);

    let away = a.with_file_name("a-away");
    fs::rename(&a, &away).unwrap();
    let refresh = submit_refresh(&shell, &a, cx);
    shell.until_ended(refresh, cx).await;
    shell.run(cx);

    assert_eq!(shown(&shell, cx), Some(1));
    assert_eq!(title(&shell, cx).as_deref(), Some("Other"));
    assert!(!shell.present("item-failure", cx));

    shell.click(("tab", 0usize), cx);
    assert_eq!(
        shell.label("item-failure", cx).as_deref(),
        Some(ResultCode::InvalidPath.message())
    );
    assert_eq!(
        shell.label("item-failure-code", cx).as_deref(),
        Some(ResultCode::InvalidPath.as_str())
    );
    assert!(shell.present("item-retry", cx));
    assert!(!shell.present("item-title", cx));
    assert_eq!(strip(&shell, cx).len(), 2, "the tab stays");

    fs::rename(&away, &a).unwrap();
    shell.click("item-retry-button", cx);
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
    shell.click(("tab", 1usize), cx);
    assert_eq!(title(&shell, cx).as_deref(), Some("Other"));
}

/// Closing the shown tab shows the one that takes its place, or the one
/// before it; closing the last empties the content region.
#[gpui_kit::test]
fn closing_the_shown_tab_shows_its_neighbor_and_the_last_empties_the_content(
    cx: &mut TestAppContext,
) {
    let Two { shell, a, .. } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    open(&shell, ticket(&a, THREE), cx);
    shell.click(("tab", 1usize), cx);
    assert_eq!(
        shell.label(("tab-close", 1usize), cx).as_deref(),
        Some(CLOSE_TAB)
    );

    shell.click(("tab-close", 1usize), cx);
    assert_eq!(strip(&shell, cx), ["One", "Three"]);
    assert_eq!(shown(&shell, cx), Some(1));
    assert_eq!(title(&shell, cx).as_deref(), Some("Three"));

    shell.click(("tab-close", 1usize), cx);
    assert_eq!(strip(&shell, cx), ["One"]);
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
    shell.click(("tab-close", 0usize), cx);
    assert!(strip(&shell, cx).is_empty());
    assert!(!shell.present("item-view", cx));
    assert!(!shell.present("item-title", cx));
}

/// `ctrl-tab` and `ctrl-shift-tab` move through the tabs, wrapping, and
/// `secondary-w` closes the shown one; focus follows to the shown tab, and
/// to the content region once none is left.
#[gpui_kit::test]
fn the_tab_keys_move_and_close_and_focus_follows(cx: &mut TestAppContext) {
    let Two { shell, a, .. } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    open(&shell, ticket(&a, THREE), cx);
    assert_eq!(shown(&shell, cx), Some(2));
    assert_eq!(ring(&shell, cx), [Region::Repositories]);

    shell.keys("ctrl-tab", cx);
    assert_eq!(shown(&shell, cx), Some(0));
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
    assert!(tab_focused(&shell, 0, cx));
    assert_eq!(ring(&shell, cx), [Region::Content]);

    shell.keys("ctrl-tab", cx);
    assert_eq!(shown(&shell, cx), Some(1));
    assert!(tab_focused(&shell, 1, cx));
    shell.keys("ctrl-shift-tab ctrl-shift-tab", cx);
    assert_eq!(shown(&shell, cx), Some(2));
    assert_eq!(title(&shell, cx).as_deref(), Some("Three"));
    assert!(tab_focused(&shell, 2, cx));

    shell.keys("secondary-w", cx);
    assert_eq!(strip(&shell, cx), ["One", "Two"]);
    assert_eq!(shown(&shell, cx), Some(1));
    assert!(tab_focused(&shell, 1, cx));
    shell.keys("secondary-w secondary-w", cx);
    assert!(strip(&shell, cx).is_empty());
    assert_eq!(ring(&shell, cx), [Region::Content]);
    // With no tab, the keys do nothing.
    shell.keys("ctrl-tab secondary-w", cx);
    assert_eq!(ring(&shell, cx), [Region::Content]);
}

/// `secondary-3` focuses the shown tab, the content region's first
/// control. When that tab goes, closed with its repository's, focus goes
/// to the tab shown in its place; with none, to the region itself.
#[gpui_kit::test]
fn a_closed_tab_that_held_focus_gives_it_to_the_content_region(cx: &mut TestAppContext) {
    let Two { shell, a, b } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&b, ONE), cx);
    shell.keys("secondary-3", cx);
    assert!(tab_focused(&shell, 1, cx));

    shell
        .workspace
        .update(cx, |workspace, cx| workspace.close_repository(&key(&b), cx));
    shell.run(cx);
    assert_eq!(strip(&shell, cx), ["One"]);
    assert_eq!(ring(&shell, cx), [Region::Content]);
    assert!(tab_focused(&shell, 0, cx));

    shell
        .workspace
        .update(cx, |workspace, cx| workspace.close_repository(&key(&a), cx));
    shell.run(cx);
    assert!(strip(&shell, cx).is_empty());
    assert_eq!(ring(&shell, cx), [Region::Content]);
    assert!(shell.frame(cx, |window, _| {
        window.find(Region::Content.id()).focused() == Some(true)
    }));
}

/// `close_repository` closes every tab of that root and no other; a later
/// open of the same item makes a new tab, read afresh.
#[gpui_kit::test]
fn close_repository_closes_every_tab_of_that_root(cx: &mut TestAppContext) {
    let Two { shell, a, b } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&b, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.close_repository(&key(&a), cx));
    shell.run(cx);
    assert_eq!(strip(&shell, cx), ["Other"]);
    assert_eq!(shown(&shell, cx), Some(0));
    assert_eq!(title(&shell, cx).as_deref(), Some("Other"));

    let reads = item_reads(&shell);
    open(&shell, ticket(&a, ONE), cx);
    assert_eq!(strip(&shell, cx), ["Other", "One"]);
    assert_eq!(item_reads(&shell) - reads, 1);
}

/// Review Focus 1: the first tab's read is held while the user opens a
/// second. Released, it lands on the first tab only: the second still
/// shows its own item.
#[gpui_kit::test]
fn a_held_load_never_shows_in_the_tab_opened_after_it(cx: &mut TestAppContext) {
    let Two { shell, a, .. } = two(cx);
    let held = shell.backend.hold_read(ReadKind::Item);
    open(&shell, ticket(&a, ONE), cx);
    assert!(!shell.present("item-title", cx));
    open(&shell, ticket(&a, TWO), cx);
    assert_eq!(title(&shell, cx).as_deref(), Some("Two"));

    held.send(()).unwrap();
    shell.run(cx);
    assert_eq!(shown(&shell, cx), Some(1));
    assert_eq!(title(&shell, cx).as_deref(), Some("Two"));
    assert_eq!(strip(&shell, cx), ["One", "Two"]);
    shell.click(("tab", 0usize), cx);
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
}

/// Every dialog traps focus: with the Abandon dialog open, the tab keys
/// neither move between tabs nor close one.
#[gpui_kit::test]
fn the_tab_keys_do_nothing_while_a_dialog_is_open(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_ticket(&a, ONE, "One", "");
    fixture.create_ticket(&a, STRANDED, "Stranded", "open");
    let _operation = fixture.strand_ticket_save(&a, STRANDED, "Edited");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&a, STRANDED), cx);
    assert_eq!(shown(&shell, cx), Some(1));

    shell.click(("pending-abandon", 0usize), cx);
    assert!(shell.present("confirm-dialog", cx));
    shell.keys("ctrl-tab", cx);
    shell.keys("secondary-w", cx);
    shell.keys("ctrl-shift-tab", cx);
    assert!(shell.present("confirm-dialog", cx));
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert_eq!(strip(&shell, cx).len(), 2);
    assert_eq!(shown(&shell, cx), Some(1));
}

/// Ruling R-B2: a finished request whose intent cannot change items
/// reads no open tab again; the tab keeps what it shows.
#[gpui_kit::test]
async fn a_finished_request_that_cannot_change_items_reads_no_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let Two { shell, a, .. } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    let reads = item_reads(&shell);
    let id = shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::RemoteSelect {
                root: a.clone(),
                name: None,
            },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(id, cx).await;
    shell.run(cx);
    assert_eq!(item_reads(&shell), reads, "no tab is read again");
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
}
