//! The item tab's frame (R-47), over a real repository: the tab strip and
//! a header placeholder above the item's own scroll region, which holds
//! the page and then the discussion. The page keys scroll the region while
//! Settings keep D1's single column; the region shows the Kit's scrollbar;
//! the discussion's Back to top returns to the page.

use gpui_kit::{
    Bounds, Modifiers, MouseButton, Pixels, TestAppContext, point, px, size, test::TestWindowExt,
};
use manyhands::repository::ItemDtoKind;

use super::BACK_TO_TOP;
use crate::desktop::{
    model::navigation::OpenTarget,
    test_support::fixtures::{Fixture, item_id, ticket_path, ticket_source},
    view::harness::{Shell, key},
};

/// A ticket whose page is taller than a short window's region.
const LONG: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
/// A ticket whose page is shorter than the region.
const SHORT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";
const LONG_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
const SHORT_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
/// The page's column at its widest (`item_view/01-item-frame.md`).
const COLUMN: f32 = 760.;
/// A window short enough that `LONG` does not fit its region.
const SHORT_WINDOW: f32 = 700.;
/// The harness's own window height.
const TALL_WINDOW: f32 = 1400.;

/// `LONG`, with a long body, and `SHORT`, each with one comment, in one
/// repository, in a window `height` tall: the ticket `open` is opened and
/// the content region focused.
fn shell(height: f32, open: &str, cx: &mut TestAppContext) -> Shell {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let body: String = (0..200)
        .map(|line| format!("Line {line} of a long body.\n"))
        .collect();
    let source = format!("{}{body}", ticket_source(LONG, "Long", ""));
    fixture.commit_file(&root, &ticket_path(LONG), source.as_bytes());
    fixture.commit_ticket(&root, SHORT, "Short", "");
    for (item, comment) in [(LONG, LONG_COMMENT), (SHORT, SHORT_COMMENT)] {
        fixture.commit_comment(&root, item, comment, None, "2026-09-30T12:00:01Z", "");
    }
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell
        .visual(cx)
        .simulate_resize(size(px(1440.), px(height)));
    shell.run(cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(open),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.keys("secondary-3", cx);
    shell
}

/// The item's scroll region's offset, after a frame.
fn offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.item_offset(cx))
}

/// The content region's own column's offset, after a frame.
fn column_offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.content_offset())
}

/// The bounds of each of `ids`, from one frame.
fn bounds<const N: usize>(
    shell: &Shell,
    ids: [&'static str; N],
    cx: &mut TestAppContext,
) -> [Bounds<Pixels>; N] {
    shell.frame(cx, |window, _| ids.map(|id| window.find(id).bounds()))
}

fn focused(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| window.find(id).focused() == Some(true))
}

/// The strip, then the header holding D1's identity line, then the
/// region: PageDown moves the page up inside the region, and neither the
/// strip, the header nor the region moves.
#[gpui_kit::test]
fn the_tab_strip_and_header_stay_in_place_while_the_page_scrolls(cx: &mut TestAppContext) {
    let shell = shell(SHORT_WINDOW, LONG, cx);
    const FRAME: [&str; 5] = [
        "tab-strip",
        "item-header",
        "item-title",
        "item-scroll",
        "item-page",
    ];
    let [strip, header, title, region, page] = bounds(&shell, FRAME, cx);
    assert!(
        header.top() >= strip.bottom(),
        "the header is below the strip: {strip:?} {header:?}"
    );
    assert!(
        region.top() >= header.bottom(),
        "the region starts below the header: {header:?} {region:?}"
    );
    assert!(
        header.contains(&title.origin),
        "the header holds the identity line: {header:?} {title:?}"
    );
    assert_eq!(page.top(), region.top(), "the page starts at the top");

    shell.keys("pagedown", cx);
    let paged = offset(&shell, cx);
    assert!(paged < px(0.), "PageDown scrolled the region: {paged:?}");
    let [
        strip_after,
        header_after,
        title_after,
        region_after,
        page_after,
    ] = bounds(&shell, FRAME, cx);
    assert_eq!(strip_after, strip, "the strip stays");
    assert_eq!(header_after, header, "the header stays");
    assert_eq!(title_after, title, "the identity line stays");
    assert_eq!(region_after, region, "the region stays");
    // Layout snaps to whole pixels.
    let moved = page_after.top() - page.top();
    assert!(
        (moved - paged).abs() < px(1.),
        "the page moved up inside the region by the offset: {moved:?} {paged:?}"
    );
}

/// While an item tab is shown, PageDown, PageUp, End and Home move its
/// scroll region and not the content region's column. Settings keep
/// D1's single column, which the same keys move, and the item keeps its
/// place behind them.
#[gpui_kit::test]
fn page_keys_scroll_the_items_region_and_settings_keep_theirs(cx: &mut TestAppContext) {
    let shell = shell(SHORT_WINDOW, LONG, cx);
    assert_eq!(offset(&shell, cx), px(0.));
    shell.keys("pagedown", cx);
    let paged = offset(&shell, cx);
    assert!(paged < px(0.), "PageDown scrolled the region: {paged:?}");
    assert_eq!(column_offset(&shell, cx), px(0.), "the column stays");
    shell.keys("pageup", cx);
    assert_eq!(offset(&shell, cx), px(0.));
    shell.keys("end", cx);
    let bottom = offset(&shell, cx);
    assert!(bottom < paged, "End reaches the bottom: {bottom:?}");
    shell.keys("home", cx);
    assert_eq!(offset(&shell, cx), px(0.));
    shell.keys("pagedown", cx);
    assert_eq!(offset(&shell, cx), paged);

    shell.keys("secondary-,", cx);
    assert_eq!(column_offset(&shell, cx), px(0.));
    shell.keys("pagedown", cx);
    let settings = column_offset(&shell, cx);
    assert!(
        settings < px(0.),
        "PageDown scrolled Settings: {settings:?}"
    );
    assert_eq!(offset(&shell, cx), paged, "the item's region did not move");
    shell.keys("home", cx);
    assert_eq!(column_offset(&shell, cx), px(0.));

    shell.click("settings-close", cx);
    assert_eq!(offset(&shell, cx), paged, "the item kept its place");
}

/// The page spans the region and is at least as tall as it, so the white
/// reaches the bottom even for a short ticket, and the discussion starts
/// below the fold. Its column is at most 760 px wide and centred. After
/// the window grows, the page grows with the region.
#[gpui_kit::test]
fn the_page_fills_the_region_and_the_discussion_starts_below_it(cx: &mut TestAppContext) {
    let shell = shell(TALL_WINDOW, SHORT, cx);
    const PAGE: [&str; 4] = ["item-scroll", "item-page", "item-page-column", "discussion"];
    let check = |shell: &Shell, cx: &mut TestAppContext| {
        // The page takes its height from the region in the frame before.
        shell.frame(cx, |_, _| ());
        let [region, page, column, discussion] = bounds(shell, PAGE, cx);
        assert_eq!(
            (page.left(), page.right(), page.top()),
            (region.left(), region.right(), region.top()),
            "the page spans the region from its top: {page:?} {region:?}"
        );
        assert!(
            page.size.height >= region.size.height,
            "the page fills the region: {page:?} {region:?}"
        );
        assert!(
            discussion.top() >= region.bottom(),
            "the discussion starts below the fold: {discussion:?} {region:?}"
        );
        assert_eq!(column.size.width, px(COLUMN), "{column:?}");
        let centre = (column.center().x - page.center().x).abs();
        assert!(
            centre < px(1.),
            "the column is centred: {column:?} {page:?}"
        );
        region
    };
    let region = check(&shell, cx);

    shell
        .visual(cx)
        .simulate_resize(size(px(1440.), px(TALL_WINDOW + 200.)));
    shell.run(cx);
    let grown = check(&shell, cx);
    assert!(
        grown.size.height > region.size.height,
        "{grown:?} {region:?}"
    );
}

/// The Kit's scrollbar lies over the region, below the header, and works
/// the region's own handle: a press on its track near the foot moves the
/// region there.
#[gpui_kit::test]
fn the_region_shows_a_scrollbar(cx: &mut TestAppContext) {
    let shell = shell(SHORT_WINDOW, LONG, cx);
    let [header, region, bar] =
        bounds(&shell, ["item-header", "item-scroll", "item-scrollbar"], cx);
    assert_eq!(bar, region, "the bar lies over the region");
    assert!(bar.top() >= header.bottom(), "{bar:?} {header:?}");

    // Scrolling shows the bar, which then takes a press.
    shell.keys("pagedown", cx);
    let paged = offset(&shell, cx);
    assert!(paged < px(0.));
    let foot = point(region.right() - px(4.), region.bottom() - px(4.));
    let mut visual = shell.visual(cx);
    visual.simulate_mouse_down(foot, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(foot, MouseButton::Left, Modifiers::none());
    let pressed = offset(&shell, cx);
    assert!(
        pressed < paged,
        "a press near the bar's foot scrolled the region down: {pressed:?} {paged:?}"
    );
}

/// Back to top, by click or by keyboard, moves the region to its top and
/// focuses the page.
#[gpui_kit::test]
fn back_to_top_returns_and_focuses_the_page(cx: &mut TestAppContext) {
    let shell = shell(SHORT_WINDOW, LONG, cx);
    assert_eq!(shell.label("back-to-top", cx).as_deref(), Some(BACK_TO_TOP));
    shell.keys("secondary-shift-c", cx);
    assert!(offset(&shell, cx) < px(0.));
    shell.click("back-to-top", cx);
    assert_eq!(offset(&shell, cx), px(0.), "back at the top");
    assert!(focused(&shell, "item-page", cx), "the page has focus");

    shell.keys("end", cx);
    let mut presses = 0;
    while !focused(&shell, "back-to-top", cx) {
        presses += 1;
        assert!(presses < 80, "Tab never reached Back to top");
        shell.keys("tab", cx);
    }
    assert!(offset(&shell, cx) < px(0.), "Back to top is below the page");
    shell.press("enter", cx);
    assert_eq!(offset(&shell, cx), px(0.), "back at the top by keyboard");
    assert!(focused(&shell, "item-page", cx), "the page has focus");
}
