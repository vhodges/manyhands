//! Tabs over real repositories: one tab per repository and item, the
//! keys, closing, reloading on refresh, a repository that goes away, and
//! completions that land on a tab no longer shown.

use std::{fs, path::PathBuf, time::Duration};

use gpui_kit::{Entity, TestAppContext, component::input::TextareaState, test::TestWindowExt};
use manyhands::{
    drafts::{DraftFields, DraftId, Held},
    repository::{ItemDtoKind, LeaseHolderForTesting, LeaseKind, RepositoryService},
    results::ResultCode,
};

use super::SHOWN;
use crate::desktop::{
    model::{navigation::OpenTarget, tabs::TabKey},
    services::{
        ReadKind,
        drafts::DraftRegistry,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::{
        drafts::{held, store_draft, stored_body},
        fixtures::{Fixture, item_id},
    },
    view::{
        close_draft::{DISCARD, KEEP_DRAFT, SAVE},
        confirm::CANCEL,
        harness::{Shell, key},
        item::{EDITOR_LATER, NOT_REFRESHED, NOT_SAVED},
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

/// Review Focus 2 (ruling R-B5): `a`'s folder goes away while it has a
/// tab open. The tab was read before; once `a` is read again, the reload
/// shows its failure with Retry, as a first load would, not the stale
/// item. The tab keeps its title; `b`'s tab is unaffected. With the folder
/// back, Retry reads it.
#[gpui_kit::test]
async fn a_tab_whose_repository_goes_away_shows_its_failure_with_retry(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let Two { shell, a, b } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&b, ONE), cx);
    shell.click(("tab", 0usize), cx);
    assert_eq!(title(&shell, cx).as_deref(), Some("One"), "read before");
    shell.click(("tab", 1usize), cx);

    let away = a.with_file_name("a-away");
    fs::rename(&a, &away).unwrap();
    let reads = item_reads(&shell);
    let refresh = submit_refresh(&shell, &a, cx);
    shell.until_ended(refresh, cx).await;
    shell.run(cx);
    assert_eq!(
        item_reads(&shell) - reads,
        0,
        "the reload stops at its resolution"
    );

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
    assert!(
        !shell.present("item-title", cx),
        "the stale item is not shown"
    );
    assert!(!shell.present("item-not-refreshed", cx));
    assert_eq!(
        strip(&shell, cx),
        ["One", "Other"],
        "the tab keeps its title"
    );

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
        shell.label(("tab-close", 1usize), cx),
        Some(super::close_label("Two"))
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

/// One load of a ticket's tab is one read job, its calls in the design's
/// order: the resolution, the item, its comments, its dependency tree, its
/// children, then its parent. A load whose item read fails makes no read
/// after it, the parent's included.
#[gpui_kit::test]
fn one_tab_load_reads_in_the_designs_order(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_ticket(&a, TWO, "Parent", "open");
    fixture.create_related_ticket(&a, ONE, "One", "open", &[], Some(TWO));
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);

    let before = shell.backend.reads_in_order().len();
    open(&shell, ticket(&a, ONE), cx);
    assert_eq!(
        shell.backend.reads_in_order()[before..],
        [
            ReadKind::Resolve,
            ReadKind::Item,
            ReadKind::Comments,
            ReadKind::Dependencies,
            ReadKind::Children,
            ReadKind::Item,
        ]
    );
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));

    let before = shell.backend.reads_in_order().len();
    open(&shell, ticket(&a, THREE), cx);
    assert_eq!(
        shell.backend.reads_in_order()[before..],
        [ReadKind::Resolve, ReadKind::Item]
    );
    assert_eq!(
        shell.label("item-failure-code", cx).as_deref(),
        Some(ResultCode::ItemNotFound.as_str())
    );
}

/// Holds the registry's cache lease, as a refresh writing it would: every
/// read answers `busy` while it is held.
fn hold_cache_write(shell: &Shell, root: &std::path::Path) -> LeaseHolderForTesting {
    RepositoryService::hold_lease_for_testing(root, shell.fixture.data_dir(), LeaseKind::CacheWrite)
        .unwrap()
}

/// Lets a load's busy retries run out: each call waits at most
/// 100 + 200 + 400 ms.
fn past_the_retries(shell: &Shell, cx: &mut TestAppContext) {
    for _ in 0..6 {
        cx.executor().advance_clock(Duration::from_secs(1));
        shell.run(cx);
    }
}

/// A reload that stays busy keeps what the tab shows: the item, the
/// source's text field with its focus, and the tab's title, and says it
/// could not refresh, with Busy's message and Retry. Retry loads again in
/// place: while its load is held, the item is still shown, not Loading.
#[gpui_kit::test]
fn a_busy_reload_keeps_the_view_and_retry_loads_in_place(cx: &mut TestAppContext) {
    let Two { shell, a, .. } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    let source = source_state(&shell, cx).expect("the source is shown");
    let mut visual = shell.visual(cx);
    visual.update(|window, cx| source.update(cx, |state, cx| state.focus(window, cx)));
    visual.run_until_parked();

    let lease = hold_cache_write(&shell, &a);
    let pane = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .unwrap();
    pane.update(cx, |pane, cx| {
        pane.reload(cx);
    });
    past_the_retries(&shell, cx);

    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
    assert_eq!(strip(&shell, cx), ["One"]);
    assert_eq!(
        shell.label("item-not-refreshed", cx).as_deref(),
        Some(NOT_REFRESHED)
    );
    assert_eq!(
        shell.label("item-not-refreshed-failure", cx).as_deref(),
        Some(ResultCode::Busy.message())
    );
    assert_eq!(
        shell.label("item-not-refreshed-code", cx).as_deref(),
        Some(ResultCode::Busy.as_str())
    );
    assert!(shell.present("item-not-refreshed-retry", cx));
    let kept = source_state(&shell, cx).expect("the source is still shown");
    assert_eq!(kept, source, "the same text field");
    assert!(shell.visual(cx).update(|window, cx| {
        gpui_kit::Focusable::focus_handle(kept.read(cx), cx).is_focused(window)
    }));

    drop(lease);
    let held = shell.backend.hold_read(ReadKind::Item);
    shell.click("item-not-refreshed-retry-button", cx);
    assert_eq!(
        title(&shell, cx).as_deref(),
        Some("One"),
        "not reset to Loading"
    );
    assert!(!shell.present("item-state", cx));
    held.send(()).unwrap();
    shell.run(cx);
    assert!(!shell.present("item-not-refreshed", cx));
    assert_eq!(title(&shell, cx).as_deref(), Some("One"));
    assert_eq!(source_state(&shell, cx), Some(source));
}

/// Focuses the shown tab's source text field, as a click into it does.
fn focus_source(shell: &Shell, cx: &mut TestAppContext) -> Entity<TextareaState> {
    let source = source_state(shell, cx).expect("the source is shown");
    let mut visual = shell.visual(cx);
    visual.update(|window, cx| source.update(cx, |state, cx| state.focus(window, cx)));
    visual.run_until_parked();
    assert!(source_focused(shell, &source, cx));
    source
}

fn source_focused(shell: &Shell, source: &Entity<TextareaState>, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| {
        gpui_kit::Focusable::focus_handle(source.read(cx), cx).is_focused(window)
    })
}

/// The tab keys work with focus in the shown item's read-only source:
/// `ctrl-tab` and `ctrl-shift-tab` move to the next and previous tab and
/// `secondary-w` closes the shown one, each with focus following. Tab
/// and shift-Tab leave the source, which takes no indent.
#[gpui_kit::test]
fn the_tab_keys_work_from_inside_the_source(cx: &mut TestAppContext) {
    let Two { shell, a, .. } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    open(&shell, ticket(&a, THREE), cx);
    shell.click(("tab", 1usize), cx);
    assert_eq!(title(&shell, cx).as_deref(), Some("Two"));

    focus_source(&shell, cx);
    shell.keys("ctrl-tab", cx);
    assert_eq!(shown(&shell, cx), Some(2));
    assert!(tab_focused(&shell, 2, cx));

    focus_source(&shell, cx);
    shell.keys("ctrl-shift-tab", cx);
    assert_eq!(shown(&shell, cx), Some(1));
    assert!(tab_focused(&shell, 1, cx));

    focus_source(&shell, cx);
    shell.keys("secondary-w", cx);
    assert_eq!(strip(&shell, cx), ["One", "Three"]);
    assert_eq!(shown(&shell, cx), Some(1));
    assert!(tab_focused(&shell, 1, cx));

    let source = focus_source(&shell, cx);
    let text = source.read_with(cx, |state, _| state.value().to_string());
    shell.keys("tab", cx);
    assert!(
        !source_focused(&shell, &source, cx),
        "Tab leaves the source"
    );
    assert!(
        shell.frame(cx, |window, cx| window.focused(cx).is_some()),
        "for the next control"
    );
    focus_source(&shell, cx);
    shell.keys("shift-tab", cx);
    assert!(
        !source_focused(&shell, &source, cx),
        "shift-Tab leaves the source"
    );
    assert_eq!(
        source.read_with(cx, |state, _| state.value().to_string()),
        text,
        "no indent was made"
    );
}

/// The strip is a tab list. Each tab is a tab stop, and its close
/// control follows it, labeled with the tab it closes: from the shown
/// tab, Tab reaches its own close control, and Enter closes that tab.
#[gpui_kit::test]
fn each_close_control_follows_its_tab_and_names_it(cx: &mut TestAppContext) {
    let Two { shell, a, .. } = two(cx);
    open(&shell, ticket(&a, ONE), cx);
    open(&shell, ticket(&a, TWO), cx);
    open(&shell, ticket(&a, THREE), cx);
    shell.click(("tab", 1usize), cx);
    assert_eq!(
        shell.frame(cx, |window, _| window.find("tab-strip").role()),
        Some(gpui_kit::Role::TabList)
    );
    for (ix, name) in ["One", "Two", "Three"].into_iter().enumerate() {
        assert_eq!(
            shell.label(("tab-close", ix), cx),
            Some(super::close_label(name))
        );
    }

    shell.keys("secondary-3", cx);
    assert!(tab_focused(&shell, 1, cx));
    shell.keys("tab", cx);
    assert!(shell.frame(cx, |window, _| {
        window.find(("tab-close", 1usize)).focused() == Some(true)
    }));
    shell.press("enter", cx);
    assert_eq!(strip(&shell, cx), ["One", "Three"], "the shown tab closed");
    assert_eq!(shown(&shell, cx), Some(1));
    assert!(shell.present(Region::Content.ring_id(), cx));

    // Tab moves from a tab to the next tab's entry past its close control,
    // and Enter there shows it.
    shell.keys("secondary-3", cx);
    shell.keys("shift-tab shift-tab", cx);
    assert!(tab_focused(&shell, 0, cx));
    shell.press("enter", cx);
    assert_eq!(shown(&shell, cx), Some(0));
}

/// Repository `a` holding One and Two, with a stored draft of One that
/// is restored in One's tab; Two's tab is open beside it, shown second.
struct Restored {
    shell: Shell,
    a: PathBuf,
    draft: DraftId,
}

fn restored(cx: &mut TestAppContext) -> Restored {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_ticket(&a, ONE, "One", "");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    let draft = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Draft body");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    shell.click("draft-restore-button", cx);
    assert!(shell.present("draft-section", cx), "the draft is restored");
    open(&shell, ticket(&a, TWO), cx);
    shell.click(("tab", 0usize), cx);
    Restored { shell, a, draft }
}

fn drafts(shell: &Shell, cx: &mut TestAppContext) -> Entity<DraftRegistry> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone())
}

fn close_dialog_shown(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.present("close-draft-dialog", cx)
}

/// The tab's Close and `secondary-w` both ask first for a tab with a
/// draft attached, and close a tab without one at once. The dialog offers
/// Keep draft, Discard and Cancel, and Save unavailable; Cancel, Escape
/// and Save keep the tab and its draft.
#[gpui_kit::test]
fn closing_a_tab_with_a_draft_asks_first_and_one_without_closes_at_once(cx: &mut TestAppContext) {
    let Restored { shell, a, draft } = restored(cx);
    shell.click(("tab-close", 0usize), cx);
    assert!(close_dialog_shown(&shell, cx));
    assert_eq!(
        shell.label("close-draft-keep", cx).as_deref(),
        Some(KEEP_DRAFT)
    );
    assert_eq!(
        shell.label("close-draft-discard", cx).as_deref(),
        Some(DISCARD)
    );
    assert_eq!(
        shell.label("close-draft-cancel", cx).as_deref(),
        Some(CANCEL)
    );
    assert_eq!(shell.label("close-draft-save", cx).as_deref(), Some(SAVE));
    assert_eq!(
        shell.label("close-draft-save-note", cx).as_deref(),
        Some(EDITOR_LATER)
    );
    assert_eq!(strip(&shell, cx), ["One", "Two"], "nothing closed yet");

    shell.click("close-draft-save-button", cx);
    assert!(close_dialog_shown(&shell, cx), "Save does nothing");
    shell.click("close-draft-cancel-button", cx);
    assert!(!close_dialog_shown(&shell, cx));
    assert_eq!(strip(&shell, cx), ["One", "Two"]);

    shell.keys("secondary-3", cx);
    shell.keys("secondary-w", cx);
    assert!(close_dialog_shown(&shell, cx));
    shell.keys("escape", cx);
    assert!(!close_dialog_shown(&shell, cx));
    assert_eq!(strip(&shell, cx), ["One", "Two"]);
    assert!(tab_focused(&shell, 0, cx), "focus goes back to the tab");
    let tab = TabKey::for_target(&ticket(&a, ONE));
    assert_eq!(
        drafts(&shell, cx).read_with(cx, |registry, _| registry.attached(&tab)),
        Some(draft)
    );

    // Two has no draft: it closes at once, by click and by key.
    shell.click(("tab-close", 1usize), cx);
    assert!(!close_dialog_shown(&shell, cx));
    assert_eq!(strip(&shell, cx), ["One"]);
    open(&shell, ticket(&a, TWO), cx);
    shell.keys("secondary-3 secondary-w", cx);
    assert!(!close_dialog_shown(&shell, cx));
    assert_eq!(strip(&shell, cx), ["One"]);
}

/// Keep draft, the default, flushes and releases the claim: the tab
/// closes, the store lists the draft unclaimed with its newest text, and
/// opening the item again offers it.
#[gpui_kit::test]
fn keep_draft_releases_the_claim_and_the_draft_is_listed_again(cx: &mut TestAppContext) {
    let Restored { shell, a, draft } = restored(cx);
    let registry = drafts(&shell, cx);
    let mut fields = registry.read_with(cx, |registry, _| {
        registry.session(draft).unwrap().current().clone()
    });
    if let DraftFields::Ticket { body, .. } = &mut fields {
        *body = "Kept body".to_owned();
    }
    registry.update(cx, |registry, cx| registry.edit(draft, fields, cx));

    shell.click(("tab-close", 0usize), cx);
    shell.press("enter", cx);
    shell.run(cx);
    assert!(!close_dialog_shown(&shell, cx));
    assert_eq!(strip(&shell, cx), ["Two"]);
    let store = shell.fixture.draft_store();
    assert!(matches!(held(&store, draft), Some(Held::No)));
    assert_eq!(stored_body(&store, draft).as_deref(), Some("Kept body"));
    assert!(registry.read_with(cx, |registry, _| registry.session(draft).is_none()));

    open(&shell, ticket(&a, ONE), cx);
    assert!(
        shell.present("draft-banner", cx),
        "the draft is offered again"
    );
    assert!(!shell.present("draft-section", cx));
}

/// Discard deletes that draft only: another item's stored draft stays.
#[gpui_kit::test]
fn discard_deletes_only_that_draft(cx: &mut TestAppContext) {
    let Restored { shell, a, draft } = restored(cx);
    let other = store_draft(&shell.fixture, &a, ItemDtoKind::Ticket, TWO, "Other body");

    shell.click(("tab-close", 0usize), cx);
    shell.click("close-draft-discard-button", cx);
    shell.run(cx);
    assert!(!close_dialog_shown(&shell, cx));
    assert_eq!(strip(&shell, cx), ["Two"]);
    let store = shell.fixture.draft_store();
    assert!(held(&store, draft).is_none(), "the draft is gone");
    assert!(matches!(held(&store, other), Some(Held::No)));
    assert_eq!(stored_body(&store, other).as_deref(), Some("Other body"));
}

/// Keep with a drafts folder that cannot be written: the flush fails, so
/// the tab stays open and its section shows the failure, with the text
/// still in memory.
#[cfg(unix)]
#[gpui_kit::test]
fn keep_with_an_unwritable_store_leaves_the_tab_open_with_the_failure(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    let Restored { shell, a, draft } = restored(cx);
    let registry = drafts(&shell, cx);
    let mut fields = registry.read_with(cx, |registry, _| {
        registry.session(draft).unwrap().current().clone()
    });
    if let DraftFields::Ticket { body, .. } = &mut fields {
        *body = "Unsaved body".to_owned();
    }
    registry.update(cx, |registry, cx| registry.edit(draft, fields, cx));
    let folder = shell.fixture.data_dir().join("drafts");
    let mode = fs::metadata(&folder).unwrap().permissions().mode();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o500)).unwrap();

    shell.click(("tab-close", 0usize), cx);
    shell.click("close-draft-keep-button", cx);
    shell.run(cx);
    fs::set_permissions(&folder, fs::Permissions::from_mode(mode)).unwrap();
    assert!(!close_dialog_shown(&shell, cx));
    assert_eq!(strip(&shell, cx), ["One", "Two"], "the tab stays open");
    let recovery = shell.label("draft-recovery", cx).unwrap();
    assert!(recovery.starts_with(NOT_SAVED), "{recovery}");
    assert!(shell.present("draft-recovery-retry", cx));
    assert_eq!(
        shell.label("draft-body", cx).as_deref(),
        Some("Unsaved body")
    );
    let tab = TabKey::for_target(&ticket(&a, ONE));
    assert_eq!(
        registry.read_with(cx, |registry, _| registry.attached(&tab)),
        Some(draft)
    );
    assert!(matches!(
        held(&shell.fixture.draft_store(), draft),
        Some(Held::Here)
    ));
}
