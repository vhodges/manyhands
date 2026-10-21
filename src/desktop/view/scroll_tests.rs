//! The content region scrolled by keyboard, over a real repository: the
//! page and end keys move it, and a control Tab reaches below the fold is
//! brought into view.

use std::path::{Path, PathBuf};

use gpui_kit::{Bounds, Pixels, TestAppContext, px, size, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use crate::desktop::{
    model::navigation::OpenTarget,
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        workspace::Region,
    },
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
/// The dependencies of `MAIN`, enough that the last lies below the fold.
const DEPS: [&str; 16] = [
    "01ARZ3NDEKTSV4RRFFQ69G5FN0",
    "01ARZ3NDEKTSV4RRFFQ69G5FN1",
    "01ARZ3NDEKTSV4RRFFQ69G5FN2",
    "01ARZ3NDEKTSV4RRFFQ69G5FN3",
    "01ARZ3NDEKTSV4RRFFQ69G5FN4",
    "01ARZ3NDEKTSV4RRFFQ69G5FN5",
    "01ARZ3NDEKTSV4RRFFQ69G5FN6",
    "01ARZ3NDEKTSV4RRFFQ69G5FN7",
    "01ARZ3NDEKTSV4RRFFQ69G5FN8",
    "01ARZ3NDEKTSV4RRFFQ69G5FN9",
    "01ARZ3NDEKTSV4RRFFQ69G5FNA",
    "01ARZ3NDEKTSV4RRFFQ69G5FNB",
    "01ARZ3NDEKTSV4RRFFQ69G5FNC",
    "01ARZ3NDEKTSV4RRFFQ69G5FND",
    "01ARZ3NDEKTSV4RRFFQ69G5FNE",
    "01ARZ3NDEKTSV4RRFFQ69G5FNF",
];

/// `MAIN`, depending on every one of `DEPS`, open in a window short
/// enough that its tab does not fit the content region, which is focused.
fn tall(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    for (ix, dep) in DEPS.iter().enumerate() {
        fixture.commit_ticket(&root, dep, &format!("Dependency {ix}"), "");
    }
    let deps: String = DEPS.iter().map(|dep| format!("  - {dep}\n")).collect();
    fixture.commit_ticket(&root, MAIN, "Main", &format!("deps:\n{deps}"));
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.visual(cx).simulate_resize(size(px(1440.), px(700.)));
    shell.run(cx);
    open(&shell, &root, cx);
    shell.keys("secondary-3", cx);
    (shell, root)
}

fn open(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(MAIN),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.content_offset())
}

/// The visible part of the content region, and the element `id`'s bounds,
/// from the same frame.
fn bounds(
    shell: &Shell,
    id: impl Into<gpui_kit::ElementId>,
    cx: &mut TestAppContext,
) -> (Bounds<Pixels>, Bounds<Pixels>) {
    let id = id.into();
    shell.frame(cx, |window, _| {
        (window.find("content").bounds(), window.find(id).bounds())
    })
}

/// PageDown moves the content region down by most of its height, PageUp
/// back, End to its bottom and Home to its top.
#[gpui_kit::test]
fn the_page_and_end_keys_scroll_the_content_region(cx: &mut TestAppContext) {
    let (shell, _root) = tall(cx);
    assert_eq!(offset(&shell, cx), px(0.));

    shell.keys("pagedown", cx);
    let paged = offset(&shell, cx);
    assert!(paged < px(0.), "PageDown scrolled down: {paged:?}");
    shell.keys("pageup", cx);
    assert_eq!(offset(&shell, cx), px(0.));

    shell.keys("end", cx);
    let bottom = offset(&shell, cx);
    assert!(bottom <= paged, "End reaches the bottom: {bottom:?}");
    shell.keys("pagedown", cx);
    assert_eq!(offset(&shell, cx), bottom, "nothing below the bottom");
    shell.keys("home", cx);
    assert_eq!(offset(&shell, cx), px(0.));
}

/// Tab reaches a relationship row below the fold: the region scrolls
/// until the row is in view.
#[gpui_kit::test]
fn a_row_tab_reaches_below_the_fold_is_scrolled_into_view(cx: &mut TestAppContext) {
    let (shell, _root) = tall(cx);
    let last = ("rel-dependency", DEPS.len() - 1);
    let (viewport, row) = bounds(&shell, last, cx);
    assert!(
        row.top() > viewport.bottom(),
        "the last row starts below the fold: {row:?} {viewport:?}"
    );

    let focused = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| window.find(last).focused() == Some(true))
    };
    let mut presses = 0;
    while !focused(&shell, cx) {
        presses += 1;
        assert!(presses < 80, "Tab never reached the last row");
        shell.keys("tab", cx);
        assert!(shell.frame(cx, |window, _| {
            window.find(Region::Content.id()).focused() == Some(true)
        }));
    }
    // The frame that saw focus arrive moved the region; this one draws it.
    let (viewport, row) = bounds(&shell, last, cx);
    assert!(
        row.intersects(&viewport),
        "the focused row is in view: {row:?} {viewport:?}"
    );
    assert!(offset(&shell, cx) < px(0.));
}
