//! The read view (decision 17; R-61, R-62, R-64, R-67), over real
//! repositories: an item tab's body is shown in a read editor on the page,
//! copies as Markdown, takes no edit and begins no draft, lets Tab leave,
//! puts the caret at the end on a click below the text, keeps its caret in
//! view by itself, keeps the cursor across a clean reload, says when a
//! file is not UTF-8, opens a large body, and changes nothing in the
//! repository. An Inspect tab still shows the exact file. A table drag is
//! reverted, and a Control-click on math opens no math menu (Review
//! Focus 5).

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::{
    Bounds, ClipboardItem, Entity, EventEmitter, Focusable, Modifiers, MouseButton, Pixels, Point,
    TestAppContext, point, px, test::TestWindowExt,
};
use manyhands::repository::{ItemDto, ItemDtoKind};
use zorite_editor::{EditorEvent, EditorState};

use super::{ItemPane, SOURCE_NOT_UTF8, chips::ChipField};
use crate::desktop::{
    model::{editor_text::load, navigation::OpenTarget},
    test_support::{
        fixtures::{Fixture, item_id, ticket_path, ticket_source},
        snapshot::RepositorySnapshot,
    },
    view::{
        editor::read_only::ReadOnly,
        harness::{Shell, key},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT0";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT1";
const LONG: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT2";
const LARGE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT3";
const DOCUMENT_PATH: &str = "docs/reading.md";

/// A body with each kind of block the read view renders.
const RICH: &str = "A paragraph to read, with **bold**, `code` and [a link](https://example.com/a).

- [ ] a task to do

| Name | Count |
| --- | --- |
| one | 1 |

Before the math.
$$
x^2 + y^2
$$
After the math.

The last paragraph.
";

/// A managed document's source with `body`.
fn document_source(id: &str, title: &str, body: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n---\n{body}"
    )
}

/// A ticket's source with `body` in place of the fixture's own.
fn ticket_with_body(id: &str, title: &str, body: &str) -> String {
    ticket_source(id, title, "").replace(&format!("Body of {title}.\n"), body)
}

/// Two hundred numbered lines: far taller than the window.
fn long_body() -> String {
    (0..200)
        .map(|line| format!("Line {line} of a long body.\n"))
        .collect()
}

/// One repository, `a`, built by `build` and refreshed, in an open
/// window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn document(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(id),
    }
}

fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

/// The shown tab's read view, after a frame.
fn read_view(shell: &Shell, cx: &mut TestAppContext) -> Option<Entity<ReadOnly>> {
    shell.frame(cx, |_, _| ());
    let pane = pane(shell, cx);
    pane.read_with(cx, |pane, _| pane.read_view())
}

/// The shown tab's read editor, after a frame.
fn read_editor(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorState> {
    let view = read_view(shell, cx).expect("the read view is shown");
    view.read_with(cx, |view, _| view.editor().clone())
}

fn text(editor: &Entity<EditorState>, cx: &mut TestAppContext) -> String {
    editor.read_with(cx, |editor, _| editor.text().to_owned())
}

fn cursor(editor: &Entity<EditorState>, cx: &mut TestAppContext) -> usize {
    editor.read_with(cx, |editor, _| editor.cursor())
}

/// Whether `editor` itself holds focus, after a frame.
fn focused(shell: &Shell, editor: &Entity<EditorState>, cx: &mut TestAppContext) -> bool {
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Whether the element `id` holds focus or contains it.
fn holds_focus(shell: &Shell, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Puts the caret at `at`, focuses the read editor and draws a frame, as
/// the editor tests do before typing.
fn focus_read(shell: &Shell, editor: &Entity<EditorState>, at: usize, cx: &mut TestAppContext) {
    editor.update(cx, |editor, cx| editor.set_cursor(at, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    assert!(focused(shell, editor, cx), "the read editor took focus");
}

fn bounds(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> Bounds<Pixels> {
    shell.frame(cx, |window, _| window.find(id).bounds())
}

/// The item's scroll region's offset, after a frame.
fn offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.item_offset(cx))
}

/// Whether the caret's line lies inside the item's scroll region, from a
/// fresh paint.
fn caret_in_view(shell: &Shell, editor: &Entity<EditorState>, cx: &mut TestAppContext) -> bool {
    let editor = editor.clone();
    shell.frame(cx, move |window, cx| {
        let region = window.find("item-scroll").bounds();
        let caret = editor
            .read(cx)
            .caret_screen_bounds()
            .expect("the caret is laid out");
        caret.top() >= region.top() - px(1.) && caret.bottom() <= region.bottom() + px(1.)
    })
}

/// The middle of the caret's box at `offset`, from a fresh paint.
fn point_at(
    shell: &Shell,
    editor: &Entity<EditorState>,
    offset: usize,
    cx: &mut TestAppContext,
) -> Point<Pixels> {
    let editor = editor.clone();
    shell
        .frame(cx, move |_, cx| editor.read(cx).bounds_for_offset(offset))
        .unwrap_or_else(|| panic!("offset {offset} is not laid out"))
        .center()
}

/// Presses and releases the left button at `position`, with `modifiers`.
fn press(shell: &Shell, position: Point<Pixels>, modifiers: Modifiers, cx: &mut TestAppContext) {
    shell.frame(cx, |_, _| ());
    let mut visual = shell.visual(cx);
    visual.simulate_mouse_move(position, None, modifiers);
    visual.simulate_mouse_down(position, MouseButton::Left, modifiers);
    visual.simulate_mouse_up(position, MouseButton::Left, modifiers);
    shell.frame(cx, |_, _| ());
}

/// Every event `entity` emits from now on.
fn events<T: EventEmitter<EditorEvent>>(
    entity: &Entity<T>,
    cx: &mut TestAppContext,
) -> Rc<RefCell<Vec<EditorEvent>>> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    cx.update({
        let seen = seen.clone();
        |cx| {
            cx.subscribe(entity, move |_, event: &EditorEvent, _| {
                seen.borrow_mut().push(event.clone())
            })
            .detach()
        }
    });
    seen
}

/// A document's body is shown on the page in a read editor, as the editor
/// loads it; the item tab has no source textarea.
#[gpui_kit::test]
fn a_document_body_renders_in_a_read_editor(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = document_source(DOCUMENT, "Reading", RICH);
        fixture.commit_file(root, DOCUMENT_PATH, source.as_bytes());
    });
    open(&shell, document(&root, DOCUMENT), cx);
    let body = shown(&shell, &root, DOCUMENT).body.unwrap();
    assert!(body == RICH, "the library reads the body as written");

    let editor = read_editor(&shell, cx);
    assert!(text(&editor, cx) == load(&body), "the body, as loaded");
    let (page, read) = (
        bounds(&shell, "item-page-column", cx),
        bounds(&shell, "item-read", cx),
    );
    assert!(
        page.contains(&read.origin) && read.size.height > px(0.),
        "the read view is on the page: {page:?} {read:?}"
    );
    assert!(!shell.present("item-source", cx), "no source textarea");
    assert!(!focused(&shell, &editor, cx), "a clean reading view");
}

/// A ticket's body is shown in its read editor too.
#[gpui_kit::test]
fn a_ticket_body_renders(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Plain", RICH);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    open(&shell, ticket(&root, TICKET), cx);
    let body = shown(&shell, &root, TICKET).body.unwrap();
    assert!(body == RICH, "the library reads the body as written");
    let editor = read_editor(&shell, cx);
    assert!(text(&editor, cx) == load(&body), "the body, as loaded");
    assert!(shell.present("item-read", cx));
    assert!(!shell.present("item-source", cx), "no source textarea");
}

/// Select All, then Copy, puts the body's Markdown on the clipboard.
#[gpui_kit::test]
fn select_all_and_copy_put_the_markdown_on_the_clipboard(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = document_source(DOCUMENT, "Reading", RICH);
        fixture.commit_file(root, DOCUMENT_PATH, source.as_bytes());
    });
    open(&shell, document(&root, DOCUMENT), cx);
    let editor = read_editor(&shell, cx);
    focus_read(&shell, &editor, 0, cx);
    shell.keys("secondary-a secondary-c", cx);
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert!(copied.as_deref() == Some(load(RICH).as_str()), "Markdown");
    assert!(text(&editor, cx) == load(RICH), "unchanged");
}

/// Typed text, Enter, Backspace, Delete, paste and cut change nothing in
/// the item's tab, and no draft begins: nothing is stored and the tab
/// shows no draft.
#[gpui_kit::test]
fn typing_and_paste_in_the_tab_change_nothing_and_begin_no_draft(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
    });
    let before = RepositorySnapshot::take(&root);
    open(&shell, ticket(&root, TICKET), cx);
    let body = shown(&shell, &root, TICKET).body.unwrap();
    let editor = read_editor(&shell, cx);
    focus_read(&shell, &editor, 5, cx);

    shell.visual(cx).simulate_input("typing");
    cx.write_to_clipboard(ClipboardItem::new_string("pasted".to_owned()));
    shell.keys(
        "enter backspace delete secondary-v shift-right secondary-x",
        cx,
    );

    assert!(text(&editor, cx) == load(&body), "the text is unchanged");
    assert!(
        shell.fixture.draft_store().list().unwrap().is_empty(),
        "no draft is stored"
    );
    for id in ["draft-banner", "draft-section"] {
        assert!(!shell.present(id, cx), "{id}");
    }
    shell.run(cx);
    assert!(shell.fixture.draft_store().list().unwrap().is_empty());
    assert_eq!(RepositorySnapshot::take(&root), before);
}

/// Tab leaves the read editor for the discussion's Back to top, and
/// shift-Tab for the header's All details; neither indents.
#[gpui_kit::test]
fn tab_moves_focus_out_of_the_read_editor(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Walked", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let editor = read_editor(&shell, cx);
    let shown = text(&editor, cx);

    focus_read(&shell, &editor, 5, cx);
    shell.keys("tab", cx);
    assert!(!focused(&shell, &editor, cx), "Tab leaves the read editor");
    assert!(holds_focus(&shell, "back-to-top", cx), "for Back to top");

    focus_read(&shell, &editor, 5, cx);
    shell.keys("shift-tab", cx);
    assert!(!focused(&shell, &editor, cx), "shift-Tab leaves it");
    assert!(
        holds_focus(&shell, ChipField::All.id(), cx),
        "for All details"
    );
    assert!(text(&editor, cx) == shown, "no indent was made");
}

/// A click on the white page below the text focuses the read editor with
/// its caret at the end of the body. The page itself takes no focus, so it
/// draws no ring.
#[gpui_kit::test]
fn a_click_below_the_text_puts_the_caret_at_the_end(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Short", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let editor = read_editor(&shell, cx);
    let (page, read) = (
        bounds(&shell, "item-page", cx),
        bounds(&shell, "item-read", cx),
    );
    assert!(
        page.bottom() > read.bottom() + px(40.),
        "room below the text"
    );
    let below = point(read.center().x, (read.bottom() + page.bottom()) / 2.);
    press(&shell, below, Modifiers::none(), cx);

    assert!(focused(&shell, &editor, cx), "the read editor has focus");
    let end = text(&editor, cx).len();
    assert_eq!(cursor(&editor, cx), end, "the caret is at the end");
    let page_focus = pane(&shell, cx).read_with(cx, |pane, _| pane.page.focus_handle().clone());
    assert!(
        !shell.frame(cx, move |window, _| page_focus.is_focused(window)),
        "the page itself is not focused"
    );
}

/// A press on the page beside the text, in the column's margin and level
/// with the first line, focuses the read editor from the list that held
/// focus, and leaves its caret where it was. The ring Back to top draws
/// around the page goes with the press.
#[gpui_kit::test]
fn a_press_beside_the_text_focuses_the_read_editor(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Beside", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let editor = read_editor(&shell, cx);
    // Back to top, below the fold, by keyboard.
    focus_read(&shell, &editor, 3, cx);
    shell.keys("tab", cx);
    assert!(holds_focus(&shell, "back-to-top", cx));
    shell.press("enter", cx);
    assert!(
        shell.present("item-page-ring", cx),
        "Back to top rings the page"
    );
    shell.keys("secondary-2", cx);
    assert!(holds_focus(&shell, "region-navigation", cx));
    assert!(!shell.present("item-page-ring", cx));

    // Left of the editor by more than its gutter grip, inside the column.
    let read = bounds(&shell, "item-read", cx);
    let beside = point(read.left() - px(40.), read.top() + px(8.));
    press(&shell, beside, Modifiers::none(), cx);

    assert!(focused(&shell, &editor, cx), "the read editor has focus");
    assert!(!holds_focus(&shell, "region-navigation", cx));
    assert_eq!(cursor(&editor, cx), 3, "the caret stays where it was");
    assert!(!shell.present("item-page-ring", cx), "no ring on a press");
}

/// A press on an Inspect tab's page, away from its controls, takes focus
/// into the content region, as D1's did, and draws no ring around the
/// page.
#[gpui_kit::test]
fn a_press_on_an_inspect_page_focuses_the_content_region(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Inspected", "");
    });
    open(
        &shell,
        OpenTarget::Inspect {
            repo: key(&root),
            worktree: key(&root),
            path: ticket_path(TICKET),
        },
        cx,
    );
    assert!(shell.present("item-source", cx));
    shell.keys("secondary-2", cx);
    assert!(holds_focus(&shell, "region-navigation", cx));

    let page = bounds(&shell, "item-page", cx);
    press(
        &shell,
        point(page.center().x, page.bottom() - px(8.)),
        Modifiers::none(),
        cx,
    );
    assert!(
        holds_focus(&shell, "region-content", cx),
        "the content region"
    );
    assert!(!holds_focus(&shell, "region-navigation", cx));
    assert!(!shell.present("item-page-ring", cx), "no ring on a press");
}

/// R-67's way back in: Shift-Tab from the discussion's Back to top lands
/// on the read view, which hands focus to its editor.
#[gpui_kit::test]
fn shift_tab_from_back_to_top_enters_the_read_view(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Walked", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let editor = read_editor(&shell, cx);
    focus_read(&shell, &editor, 5, cx);
    shell.keys("tab", cx);
    assert!(holds_focus(&shell, "back-to-top", cx));

    shell.keys("shift-tab", cx);
    assert!(focused(&shell, &editor, cx), "the read editor has focus");
    assert_eq!(cursor(&editor, cx), 5, "with its caret where it was");
}

/// R-67: the editor never scrolls, so arrowing down past the fold moves
/// the item's region to keep the caret in view.
#[gpui_kit::test]
fn arrowing_past_the_fold_keeps_the_caret_in_view(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(LONG, "Long", &long_body());
        fixture.commit_file(root, &ticket_path(LONG), source.as_bytes());
    });
    open(&shell, ticket(&root, LONG), cx);
    let editor = read_editor(&shell, cx);
    focus_read(&shell, &editor, 0, cx);
    assert_eq!(offset(&shell, cx), px(0.));

    for line in 1..=80 {
        shell.keys("down", cx);
        assert!(caret_in_view(&shell, &editor, cx), "line {line}");
    }
    assert!(cursor(&editor, cx) > 0, "the caret moved");
    assert!(offset(&shell, cx) < px(0.), "the region followed the caret");
}

/// R-67: no focus-reveal marker sits on the read editor, so a click deep
/// in a long body leaves the region where it is, instead of moving it to
/// the editor's top.
#[gpui_kit::test]
fn a_click_deep_in_a_long_body_does_not_jump_to_its_top(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(LONG, "Long", &long_body());
        fixture.commit_file(root, &ticket_path(LONG), source.as_bytes());
    });
    open(&shell, ticket(&root, LONG), cx);
    let editor = read_editor(&shell, cx);
    shell.keys("secondary-3 pagedown pagedown", cx);
    let scrolled = offset(&shell, cx);
    assert!(scrolled < px(0.), "the region scrolled: {scrolled:?}");

    let region = bounds(&shell, "item-scroll", cx);
    press(&shell, region.center(), Modifiers::none(), cx);
    assert!(focused(&shell, &editor, cx), "the click focused the editor");
    assert!(cursor(&editor, cx) > 0, "the caret is where the click was");
    assert_eq!(offset(&shell, cx), scrolled, "the region did not move");
    assert!(caret_in_view(&shell, &editor, cx));
}

/// R-67: Tab onto a read editor whose caret is below the fold brings the
/// caret into view as the editor takes focus. Moving the caret of an
/// unfocused editor moves nothing.
#[gpui_kit::test]
fn tabbing_onto_a_read_editor_below_the_fold_reveals_its_caret(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(LONG, "Long", &long_body());
        fixture.commit_file(root, &ticket_path(LONG), source.as_bytes());
    });
    open(&shell, ticket(&root, LONG), cx);
    let editor = read_editor(&shell, cx);
    let deep = text(&editor, cx).find("Line 70 ").unwrap();
    editor.update(cx, |editor, cx| editor.set_cursor(deep, cx));
    shell.run(cx);
    assert_eq!(
        offset(&shell, cx),
        px(0.),
        "an unfocused caret moves nothing"
    );
    assert!(
        !caret_in_view(&shell, &editor, cx),
        "the caret is below the fold"
    );

    shell.keys("secondary-3", cx);
    let mut presses = 0;
    while !holds_focus(&shell, ChipField::All.id(), cx) {
        presses += 1;
        assert!(presses < 12, "Tab never reached All details");
        shell.keys("tab", cx);
    }
    shell.keys("tab", cx);
    assert!(focused(&shell, &editor, cx), "Tab reached the read editor");
    assert!(holds_focus(&shell, "item-page", cx), "inside the page");
    assert_eq!(cursor(&editor, cx), deep, "the caret stayed");
    assert!(caret_in_view(&shell, &editor, cx), "the caret was revealed");
    assert!(offset(&shell, cx) < px(0.));
}

/// R-19: a clean reload shows the new body in the same read editor and
/// keeps the cursor, clamped to the new text.
#[gpui_kit::test]
fn a_clean_reload_keeps_the_cursor(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Reloaded", "The first body, as read.\n");
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    open(&shell, ticket(&root, TICKET), cx);
    let editor = read_editor(&shell, cx);
    focus_read(&shell, &editor, 9, cx);

    let reload = |body: &str, cx: &mut TestAppContext| {
        let source = ticket_with_body(TICKET, "Reloaded", body);
        shell
            .fixture
            .commit_file(&root, &ticket_path(TICKET), source.as_bytes());
        shell.fixture.refresh(&root);
        pane(&shell, cx).update(cx, |pane, cx| {
            pane.reload(cx);
        });
        shell.run(cx);
    };
    reload("A second body,\r\nlonger than the first.\n", cx);
    assert_eq!(read_editor(&shell, cx), editor, "the same read editor");
    assert!(text(&editor, cx) == "A second body,\nlonger than the first.\n");
    assert_eq!(cursor(&editor, cx), 9, "the cursor is kept");
    assert!(focused(&shell, &editor, cx), "and the focus");

    reload("Short.\n", cx);
    assert!(text(&editor, cx) == "Short.\n");
    assert_eq!(cursor(&editor, cx), "Short.\n".len(), "clamped");
}

/// A file that is not UTF-8 has no body to show: the item tab says so, in
/// D1's words, and has no read editor.
#[gpui_kit::test]
fn a_non_utf8_item_says_so(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Garbled", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    assert!(shell.present("item-read", cx));

    shell.fixture.write_non_utf8(&root, &ticket_path(TICKET));
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    assert_eq!(
        shell.label("item-source-missing", cx).as_deref(),
        Some(SOURCE_NOT_UTF8)
    );
    assert!(read_view(&shell, cx).is_none(), "no read editor");
    assert!(!shell.present("item-read", cx));
    assert!(!shell.present("item-source", cx));
}

/// R-64: the details sheet's Inspect link opens the item's Inspect tab,
/// which still shows the whole file exactly, in D1's source view, with no
/// read editor.
#[gpui_kit::test]
fn the_inspect_tab_still_shows_the_exact_file(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Inspected", "extra: kept\n");
    });
    open(&shell, ticket(&root, TICKET), cx);
    shell.click(ChipField::All.id(), cx);
    // The link sits below the sheet's fold: Tab reaches it.
    let mut presses = 0;
    while !holds_focus(&shell, "sheet-inspect", cx) {
        presses += 1;
        assert!(presses < 40, "Tab never reached the Inspect link");
        shell.keys("tab", cx);
    }
    shell.press("enter", cx);

    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert!(matches!(opened, Some(OpenTarget::Inspect { .. })));
    let file = std::fs::read_to_string(root.join(ticket_path(TICKET))).unwrap();
    assert!(shell.present("item-source", cx));
    shell.frame(cx, |_, _| ());
    let value = pane(&shell, cx).read_with(cx, |pane, cx| {
        pane.source_state()
            .map(|state| state.read(cx).value().to_string())
    });
    assert!(value.as_deref() == Some(file.as_str()), "the exact file");
    assert!(read_view(&shell, cx).is_none(), "no read editor");
    assert!(!shell.present("item-read", cx));
}

/// Review Focus 4: a 200 KiB body opens in the read view. A smoke test;
/// lag is the native checklist's step 10.
#[gpui_kit::test]
fn a_large_body_opens(cx: &mut TestAppContext) {
    let body: String = (0..)
        .map(|line| format!("Line {line} of a large document, with words enough to fill it.\n"))
        .scan(0, |size, line| {
            (*size < 200 * 1024).then(|| {
                *size += line.len();
                line
            })
        })
        .collect();
    assert!(body.len() >= 200 * 1024);
    let (shell, root) = shell(cx, |fixture, root| {
        let source = document_source(LARGE, "Large", &body);
        fixture.commit_file(root, "docs/large.md", source.as_bytes());
    });
    open(&shell, document(&root, LARGE), cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Large"));
    let editor = read_editor(&shell, cx);
    assert!(text(&editor, cx) == load(&body), "the whole body");
    assert!(shell.present("item-read", cx));
}

/// Viewing writes nothing: opening a ticket and a document, clicking,
/// moving, selecting, copying, typing and reloading leave the repository
/// and the files as they were.
#[gpui_kit::test]
fn viewing_changes_nothing_in_the_repository(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Viewed", RICH);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
        let source = document_source(DOCUMENT, "Reading", RICH);
        fixture.commit_file(root, DOCUMENT_PATH, source.as_bytes());
    });
    let before = RepositorySnapshot::take(&root);
    let files = [ticket_path(TICKET), DOCUMENT_PATH.to_owned()]
        .map(|path| std::fs::read(root.join(path)).unwrap());

    for target in [ticket(&root, TICKET), document(&root, DOCUMENT)] {
        open(&shell, target, cx);
        let editor = read_editor(&shell, cx);
        let at = point_at(&shell, &editor, 5, cx);
        press(&shell, at, Modifiers::none(), cx);
        shell.keys(
            "down right shift-down secondary-c secondary-a secondary-c",
            cx,
        );
        shell.visual(cx).simulate_input("typing");
        shell.keys("enter backspace delete secondary-v secondary-x", cx);
        assert!(text(&editor, cx) == load(RICH), "unchanged");
        pane(&shell, cx).update(cx, |pane, cx| {
            pane.reload(cx);
        });
        shell.run(cx);
    }

    assert_eq!(RepositorySnapshot::take(&root), before);
    let after = [ticket_path(TICKET), DOCUMENT_PATH.to_owned()]
        .map(|path| std::fs::read(root.join(path)).unwrap());
    assert!(after == files, "the files are as they were");
    assert!(shell.fixture.draft_store().list().unwrap().is_empty());
}

/// Review Focus 5 (carried from Task 3): the editor's gutter grip drags a
/// table block, from a window-level listener outside the guard's bounds.
/// The drag lands, and the guard puts the text back.
#[gpui_kit::test]
fn a_table_drag_is_reverted(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = document_source(DOCUMENT, "Reading", RICH);
        fixture.commit_file(root, DOCUMENT_PATH, source.as_bytes());
    });
    open(&shell, document(&root, DOCUMENT), cx);
    let editor = read_editor(&shell, cx);
    let inner = events(&editor, cx);
    let table = text(&editor, cx).find("| Name").unwrap();
    let left = point_at(&shell, &editor, 0, cx).x;
    let row = point_at(&shell, &editor, table, cx).y;
    // The grip is 14 px square, 22 px left of the editor (zorite
    // `grip_left`).
    let grip = point(left - px(15.), row);
    let read = bounds(&shell, "item-read", cx);
    let below = point(read.center().x, read.bottom() + px(8.));

    shell.frame(cx, |_, _| ());
    let mut visual = shell.visual(cx);
    visual.simulate_mouse_move(grip, None, Modifiers::none());
    shell.frame(cx, |_, _| ());
    let mut visual = shell.visual(cx);
    visual.simulate_mouse_down(grip, MouseButton::Left, Modifiers::none());
    shell.frame(cx, |_, _| ());
    let mut visual = shell.visual(cx);
    visual.simulate_mouse_move(below, Some(MouseButton::Left), Modifiers::none());
    shell.frame(cx, |_, _| ());
    let mut visual = shell.visual(cx);
    visual.simulate_mouse_up(below, MouseButton::Left, Modifiers::none());
    shell.run(cx);

    let changed = inner
        .borrow()
        .iter()
        .filter(|event| **event == EditorEvent::Changed)
        .count();
    assert!(changed >= 1, "the drag did not land");
    assert!(text(&editor, cx) == load(RICH), "the drag was reverted");
    assert!(shell.fixture.draft_store().list().unwrap().is_empty());
}

/// Carried from Task 3's review: a Control-left press on a `$$` block makes
/// the inner editor ask for its math menu. A read editor opens none: no
/// math editor is seated, the read view emits nothing, and the text stays.
#[gpui_kit::test]
fn a_control_click_on_math_opens_no_math_menu(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = document_source(DOCUMENT, "Reading", RICH);
        fixture.commit_file(root, DOCUMENT_PATH, source.as_bytes());
    });
    open(&shell, document(&root, DOCUMENT), cx);
    let view = read_view(&shell, cx).expect("the read view is shown");
    let editor = read_editor(&shell, cx);
    let (emitted, inner) = (events(&view, cx), events(&editor, cx));
    let formula = text(&editor, cx).find("x^2").unwrap();
    let math = point_at(&shell, &editor, formula, cx);
    press(&shell, math, Modifiers::control(), cx);

    assert!(
        inner
            .borrow()
            .iter()
            .any(|event| matches!(event, EditorEvent::MathMenu { .. })),
        "the press did not reach the math block"
    );
    assert!(emitted.borrow().is_empty(), "the read view emitted nothing");
    let seated = editor.read_with(cx, |editor, _| editor.editing_block_range());
    assert_eq!(seated, None, "no math editor is seated");
    assert!(text(&editor, cx) == load(RICH));
}

/// Typing, paste and cut in the item's tab queue no request.
#[gpui_kit::test]
fn typing_and_paste_in_the_tab_queue_no_request(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
    });
    let requests = shell.requests(cx, |_| true).len();
    open(&shell, ticket(&root, TICKET), cx);
    let editor = read_editor(&shell, cx);
    focus_read(&shell, &editor, 5, cx);

    shell.visual(cx).simulate_input("typing");
    cx.write_to_clipboard(ClipboardItem::new_string("pasted".to_owned()));
    shell.keys(
        "enter backspace delete secondary-v shift-right secondary-x",
        cx,
    );
    shell.run(cx);
    assert_eq!(shell.requests(cx, |_| true).len(), requests);
}

/// Viewing a ticket and a document, clicking, typing and reloading queue
/// no request.
#[gpui_kit::test]
fn viewing_queues_no_request(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Viewed", RICH);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
        let source = document_source(DOCUMENT, "Reading", RICH);
        fixture.commit_file(root, DOCUMENT_PATH, source.as_bytes());
    });
    let requests = shell.requests(cx, |_| true).len();

    for target in [ticket(&root, TICKET), document(&root, DOCUMENT)] {
        open(&shell, target, cx);
        let editor = read_editor(&shell, cx);
        let at = point_at(&shell, &editor, 5, cx);
        press(&shell, at, Modifiers::none(), cx);
        shell.keys("down right shift-down secondary-c", cx);
        shell.visual(cx).simulate_input("typing");
        shell.keys("enter backspace delete secondary-v secondary-x", cx);
        pane(&shell, cx).update(cx, |pane, cx| {
            pane.reload(cx);
        });
        shell.run(cx);
    }
    assert_eq!(shell.requests(cx, |_| true).len(), requests);
}
