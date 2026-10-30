//! Core actions the walk does not take, each by keyboard alone in a
//! window of its own: Copy, each relationship group, Resume, Cancel on a
//! queued request, Retry and Remove on an unavailable repository, and a
//! nonconforming entry opened from the tree.

use std::{fs, path::PathBuf};

use gpui_kit::TestAppContext;
use manyhands::results::{Outcome, ResultCode};

use super::Walk;
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{
        JobKind,
        requests::{Intent, RequestState},
    },
    test_support::fixtures::Fixture,
    view::{
        harness::{Shell, key},
        operations::request_id,
        workspace::Region,
    },
};

const COPIED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN0";
const COPIED_SLUG: &str = "kb-c0p1e";
const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN1";
const DEPENDENCY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN2";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN3";
const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN4";
const CHILD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN5";
const STRANDED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN6";
const MARKER: &str = "docs/marker.md";

/// A window over the one repository `a` that `build` fills, refreshed.
fn walk(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &std::path::Path)) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    build(&fixture, &a);
    fixture.refresh(&a);
    Walk::over(Shell::open(fixture, cx), a)
}

/// Copy short code and Copy ID, each reached by Tab from the shown tab
/// and pressed by Enter, leave exactly that text on the clipboard.
#[gpui_kit::test]
fn copy_by_keyboard(cx: &mut TestAppContext) {
    let walk = walk(cx, |fixture, a| {
        fixture.commit_ticket(a, COPIED, "Copied", &format!("slug: {COPIED_SLUG}\n"));
    });
    walk.select_a_repository(cx);
    walk.find(COPIED, cx);
    assert_eq!(walk.opened(cx), Some(walk.ticket(COPIED)));
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);
    // E-23: the details sheet, opened by All details, holds the controls.
    walk.tab_to("tab", "chip-all", cx);
    walk.enter(cx);
    let clipboard = |cx: &mut TestAppContext| cx.read_from_clipboard().and_then(|item| item.text());
    for (control, wanted) in [("copy-slug", COPIED_SLUG), ("copy-id", COPIED)] {
        walk.tab_to("tab", control, cx);
        walk.on(control, Region::Content, cx);
        walk.enter(cx);
        assert_eq!(clipboard(cx).as_deref(), Some(wanted), "{control}");
        walk.on(control, Region::Content, cx);
    }
}

/// From the ticket's tab, Tab reaches the first row of its dependents, its
/// parent and its children in turn, and Enter opens each in a tab of its
/// own. (The walk follows a dependency.)
#[gpui_kit::test]
fn each_relationship_group_is_followed_by_keyboard(cx: &mut TestAppContext) {
    let walk = walk(cx, |fixture, a| {
        fixture.create_ticket(a, DEPENDENCY, "Dependency", "open");
        fixture.create_ticket(a, PARENT, "Parent", "open");
        fixture.create_related_ticket(a, MAIN, "Main", "open", &[DEPENDENCY], Some(PARENT));
        fixture.create_ticket_with_deps(a, DEPENDENT, "Dependent", "open", &[MAIN]);
        fixture.create_related_ticket(a, CHILD, "Child", "open", &[], Some(MAIN));
    });
    walk.select_a_repository(cx);
    walk.find(MAIN, cx);
    for (row, target) in [
        ("rel-dependent", DEPENDENT),
        ("rel-parent", PARENT),
        ("rel-child", CHILD),
    ] {
        // Back to Main's tab, the first: `ctrl-tab` wraps to it.
        walk.keys("secondary-3", cx);
        while walk.opened(cx) != Some(walk.ticket(MAIN)) {
            walk.keys("ctrl-tab", cx);
        }
        walk.on(("tab", 0usize), Region::Content, cx);
        // E-23: the details sheet, opened by All details, holds the rows.
        walk.tab_to("tab", "chip-all", cx);
        walk.enter(cx);
        walk.tab_to("tab", (row, 0usize), cx);
        walk.on((row, 0usize), Region::Content, cx);
        walk.enter(cx);
        assert_eq!(walk.opened(cx), Some(walk.ticket(target)), "{row}");
        walk.region(Region::Content, cx);
    }
}

/// Resume on a pending local operation, reached by Tab from the
/// operation area: the operation finishes, its row goes, and focus stays
/// in the operation area.
#[gpui_kit::test]
async fn resume_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    // No refresh after: the pending operation would refuse it.
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_ticket(&a, STRANDED, "Stranded", "open");
    fixture.strand_ticket_save(&a, STRANDED, "Edited");
    let walk = Walk::over(Shell::open(fixture, cx), a);
    walk.keys("secondary-4", cx);
    walk.tab_to("tab", ("pending-resume", 0usize), cx);
    walk.on(("pending-resume", 0usize), Region::Operations, cx);
    walk.enter(cx);
    let ended = walk
        .ended(
            |intent| matches!(intent, Intent::OperationResume { .. }),
            cx,
        )
        .await;
    assert_eq!(ended, Some((Outcome::Success, ResultCode::Ok)));
    walk.until_drawn("pending-empty", cx).await;
    walk.region(Region::Operations, cx);
}

/// Cancel on a queued refresh, reached by Tab: it is removed before the
/// library sees it, and focus stays in the operation area.
#[gpui_kit::test]
async fn cancel_a_queued_request_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = walk(cx, |_, _| {});
    walk.select_a_repository(cx);
    let held = walk.shell.backend.hold_job(JobKind::IndexRefresh);
    walk.keys("secondary-r", cx);
    held.wait_reached();
    walk.keys("secondary-r", cx);
    let refreshes = walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    });
    assert_eq!(refreshes.len(), 2);
    let (queued, running) = (refreshes[0], refreshes[1]);
    let operations = walk.shell.operations(cx);
    let state = |cx: &mut TestAppContext| {
        operations.read_with(cx, |operations, _| {
            operations
                .request(queued)
                .map(|request| request.state.clone())
        })
    };
    assert!(matches!(state(cx), Some(RequestState::Queued)));

    walk.keys("secondary-4", cx);
    let cancel = request_id(queued, "cancel");
    walk.tab_to("tab", cancel.clone(), cx);
    walk.on(cancel.clone(), Region::Operations, cx);
    walk.enter(cx);
    walk.shell.until_ended(queued, cx).await;
    assert!(matches!(state(cx), Some(RequestState::Removed)));
    assert!(!walk.shell.present(cancel, cx));
    walk.region(Region::Operations, cx);

    held.release();
    walk.shell.until_ended(running, cx).await;
}

/// `a`'s folder, renamed away: the selection is unavailable.
fn unavailable(cx: &mut TestAppContext) -> (Walk, PathBuf) {
    let walk = walk(cx, |_, _| {});
    let away = walk.a.with_file_name("a-away");
    fs::rename(&walk.a, &away).unwrap();
    walk.select_a_repository(cx);
    assert!(walk.shell.present("selection-unavailable-state", cx));
    walk.keys("secondary-3", cx);
    walk.region(Region::Content, cx);
    (walk, away)
}

/// Retry on an unavailable repository, reached by Tab from the content
/// region: once the folder is back, it resolves.
#[gpui_kit::test]
fn retry_an_unavailable_repository_by_keyboard(cx: &mut TestAppContext) {
    let (walk, away) = unavailable(cx);
    walk.tab_to("tab", "selection-retry", cx);
    walk.on("selection-retry", Region::Content, cx);
    fs::rename(&away, &walk.a).unwrap();
    walk.enter(cx);
    assert!(!walk.shell.present("selection-unavailable-state", cx));
    assert!(walk.shell.present("selection-configuration", cx));
    // Retry went with the unavailable state; focus stays in the region.
    walk.region(Region::Content, cx);
}

/// Remove on an unavailable repository, reached by Tab and pressed by
/// Enter, opens its confirmation; Enter confirms. The registration goes
/// and the folder is left as it was.
#[gpui_kit::test]
async fn remove_an_unavailable_repository_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (walk, away) = unavailable(cx);
    walk.tab_to("tab", "selection-remove", cx);
    walk.on("selection-remove", Region::Content, cx);
    walk.enter(cx);
    assert!(walk.shell.present("confirm-ok", cx));
    walk.keys("enter", cx);
    let ended = walk
        .ended(|intent| matches!(intent, Intent::RepoRemove { .. }), cx)
        .await;
    assert_eq!(ended, Some((Outcome::Success, ResultCode::Ok)));
    let listed = walk.shell.fixture.service().list_repositories().unwrap();
    assert!(listed.items.iter().all(|item| item.root != key(&walk.a)));
    assert!(away.join(".git").exists(), "the folder is untouched");
}

/// A nonconforming file in the document tree: the arrows reach it and
/// Enter opens it by its path in an inspection tab.
#[gpui_kit::test]
fn a_nonconforming_entry_opens_by_keyboard(cx: &mut TestAppContext) {
    let walk = walk(cx, |fixture, a| {
        fixture.commit_nonconforming_document(a, MARKER)
    });
    walk.select_a_repository(cx);
    walk.keys("secondary-2", cx);
    walk.region(Region::Navigation, cx);
    let nodes = walk.shell.rows("doc-node", cx);
    let at = nodes
        .iter()
        .position(|node| node == MARKER)
        .expect("the file is listed");
    let mut presses = 0;
    while !walk.shell.selected(("doc-node", at), cx) {
        presses += 1;
        assert!(presses <= nodes.len() + 1, "the arrows never reached it");
        walk.keys("down", cx);
    }
    walk.enter(cx);
    let Some(OpenTarget::Inspect { repo, path, .. }) = walk.opened(cx) else {
        panic!("opened by path");
    };
    assert_eq!((repo, path.as_str()), (key(&walk.a), MARKER));
    assert_eq!(walk.shell.label("item-path", cx).as_deref(), Some(MARKER));
    walk.region(Region::Navigation, cx);
}
