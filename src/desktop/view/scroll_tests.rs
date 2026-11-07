//! The item's scroll region scrolled by keyboard, over a real repository:
//! the page and end keys move it, and a control Tab reaches below the fold
//! is brought into view.

use std::path::{Path, PathBuf};

use gpui_kit::{Bounds, Pixels, TestAppContext, px, size, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use crate::desktop::{
    model::navigation::OpenTarget,
    test_support::fixtures::{Fixture, item_id, ticket_path, ticket_source},
    view::{
        harness::{Shell, key},
        workspace::Region,
    },
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
/// Lines in `MAIN`'s body, enough that the discussion lies below the fold.
const LINES: usize = 200;

/// `MAIN`, with a long body, open in a window short enough that its tab
/// does not fit the item's scroll region, with the content region focused.
fn tall(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let body: String = (0..LINES)
        .map(|line| format!("Line {line} of a long body.\n"))
        .collect();
    let source = format!("{}{body}", ticket_source(MAIN, "Main", ""));
    fixture.commit_file(&root, &ticket_path(MAIN), source.as_bytes());
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
        .read_with(cx, |workspace, cx| workspace.item_offset(cx))
}

/// The visible part of the item's scroll region, and the element `id`'s
/// bounds, from the same frame.
fn bounds(
    shell: &Shell,
    id: impl Into<gpui_kit::ElementId>,
    cx: &mut TestAppContext,
) -> (Bounds<Pixels>, Bounds<Pixels>) {
    let id = id.into();
    shell.frame(cx, |window, _| {
        (
            window.find("item-scroll").bounds(),
            window.find(id).bounds(),
        )
    })
}

/// PageDown moves the item's scroll region down by most of its height,
/// PageUp back, End to its bottom and Home to its top.
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

/// Tab reaches the discussion's Back to top below the fold, under a long
/// body: the region scrolls until it is in view.
#[gpui_kit::test]
fn a_row_tab_reaches_below_the_fold_is_scrolled_into_view(cx: &mut TestAppContext) {
    let (shell, _root) = tall(cx);
    let last = "back-to-top";
    let (viewport, row) = bounds(&shell, last, cx);
    assert!(
        row.top() > viewport.bottom(),
        "Back to top starts below the fold: {row:?} {viewport:?}"
    );

    let focused = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| window.find(last).focused() == Some(true))
    };
    let mut presses = 0;
    while !focused(&shell, cx) {
        presses += 1;
        assert!(presses < 80, "Tab never reached Back to top");
        shell.keys("tab", cx);
        assert!(shell.frame(cx, |window, _| {
            window.find(Region::Content.id()).focused() == Some(true)
        }));
    }
    // The frame that saw focus arrive moved the region; this one draws it.
    let (viewport, row) = bounds(&shell, last, cx);
    assert!(
        row.intersects(&viewport),
        "Back to top is in view once focused: {row:?} {viewport:?}"
    );
    assert!(offset(&shell, cx) < px(0.));
}
