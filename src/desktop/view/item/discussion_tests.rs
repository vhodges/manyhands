//! The discussion (design, Discussion; R-53, R-56, R-63, R-67), over real
//! repositories: a file that is no comment stays in view with its path and
//! reason; each comment's body is a read editor, kept by its ID across
//! reloads, that copies as Markdown and takes no edit; the header's
//! Comments {n} and `secondary-shift-c` jump to the discussion and focus
//! its heading; the floating button hides once the discussion shows and
//! when there are no comments; Comment is disabled with its note; and Tab
//! runs through the comments, bringing each caret into view.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::{
    ClipboardItem, ElementId, Entity, Focusable, Pixels, TestAppContext, px, size,
    test::TestWindowExt,
};
use manyhands::{repository::ItemDtoKind, results::ProblemCode};
use zorite_editor::{EditorEvent, EditorState};

use super::{
    ItemPane,
    discussion::{NO_COMMENTS, comments_phrase},
};
use crate::desktop::{
    model::{
        editor_text::load,
        item::{COMMENT, LATER_NOTE},
        navigation::OpenTarget,
    },
    test_support::fixtures::{Fixture, item_id, ticket_path, ticket_source},
    view::{
        editor::read_only::ReadOnly,
        harness::{Shell, key},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
const B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
const C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE9";
const ADA: &str = "Ada Lovelace <ada@example.invalid>";
const BO: &str = "Bo Carter <bo@example.invalid>";
/// A window short enough that the discussion's comments do not all fit
/// its region.
const SHORT_WINDOW: f32 = 700.;

fn comment_path(id: &str) -> String {
    format!(".manyhands/comments/{TICKET}/{id}.md")
}

/// Commits the comment `id` on `TICKET`, `second`s after noon, by
/// `author` when there is one, with `body`.
fn commit_comment(
    fixture: &Fixture,
    root: &Path,
    id: &str,
    parent: Option<&str>,
    second: u32,
    author: Option<&str>,
    body: &str,
) {
    let parent = parent
        .map(|parent| format!("parent_id: {parent}\n"))
        .unwrap_or_default();
    let author = author
        .map(|author| format!("created_by: {author}\n"))
        .unwrap_or_default();
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {TICKET}\n\
         {parent}created_at: 2026-09-30T12:00:{second:02}Z\n{author}---\n{body}"
    );
    fixture.commit_file(root, &comment_path(id), source.as_bytes());
}

/// `TICKET` with a body far taller than the window.
fn commit_long_ticket(fixture: &Fixture, root: &Path) {
    let body: String = (0..200)
        .map(|line| format!("Line {line} of a long body.\n"))
        .collect();
    let source = format!("{}{body}", ticket_source(TICKET, "Long", ""));
    fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
}

/// One repository, `a`, built by `build` and refreshed, in a window
/// `height` tall, with `TICKET` open.
fn shell(
    height: f32,
    cx: &mut TestAppContext,
    build: impl FnOnce(&Fixture, &Path),
) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell
        .visual(cx)
        .simulate_resize(size(px(1440.), px(height)));
    shell.run(cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    (shell, root)
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

/// The read view of the comment `id`'s body, after a frame.
fn comment_view(shell: &Shell, id: &str, cx: &mut TestAppContext) -> Option<Entity<ReadOnly>> {
    shell.frame(cx, |_, _| ());
    let pane = pane(shell, cx);
    pane.read_with(cx, |pane, _| pane.comment_view(id))
}

/// The read editor of the comment `id`'s body, after a frame.
fn comment_editor(shell: &Shell, id: &str, cx: &mut TestAppContext) -> Entity<EditorState> {
    let view = comment_view(shell, id, cx).unwrap_or_else(|| panic!("{id} has a read editor"));
    view.read_with(cx, |view, _| view.editor().clone())
}

/// The page's read editor, after a frame.
fn page_editor(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorState> {
    shell.frame(cx, |_, _| ());
    let pane = pane(shell, cx);
    pane.read_with(cx, |pane, cx| pane.read_editor(cx))
        .expect("the read view is shown")
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

/// Whether the element `id` holds focus.
fn holds_focus(shell: &Shell, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether the element `id` is drawn inside `within`.
fn inside(
    shell: &Shell,
    id: impl Into<ElementId>,
    within: &'static str,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.path().contains(&ElementId::from(within)))
    })
}

/// Puts the caret at `at`, focuses `editor` and draws a frame, as the
/// editor tests do before typing.
fn focus_editor(shell: &Shell, editor: &Entity<EditorState>, at: usize, cx: &mut TestAppContext) {
    editor.update(cx, |editor, cx| editor.set_cursor(at, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    assert!(focused(shell, editor, cx), "the read editor took focus");
}

/// The item's scroll region's offset, after a frame.
fn offset(shell: &Shell, cx: &mut TestAppContext) -> Pixels {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.item_offset(cx))
}

/// The tops of the item's scroll region and of the discussion.
fn tops(shell: &Shell, cx: &mut TestAppContext) -> (Pixels, Pixels) {
    shell.frame(cx, |window, _| {
        (
            window.find("item-scroll").bounds().top(),
            window.find("discussion").bounds().top(),
        )
    })
}

/// Whether the caret's line of `editor` lies inside the item's scroll
/// region, from a fresh paint.
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

/// Every event `entity` emits from now on.
fn events(entity: &Entity<EditorState>, cx: &mut TestAppContext) -> Rc<RefCell<Vec<EditorEvent>>> {
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

/// A file among the comments that is no comment stays in view after them,
/// with its path and the library's reason. It is not counted, and it has
/// no author and no read editor.
#[gpui_kit::test]
fn a_malformed_entry_shows_its_path_and_reason(cx: &mut TestAppContext) {
    let broken = comment_path(BROKEN);
    let (shell, _root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
        fixture.commit_file(
            root,
            &broken,
            b"---\nmanyhands_managed: true\nmanyhands_kind: comment\nkey: 1\nkey: 2\n---\nNo.\n",
        );
    });

    assert_eq!(
        shell.label(("comment-path", 1usize), cx).as_deref(),
        Some(broken.as_str())
    );
    assert_eq!(
        shell.label(("comment-problem", 1usize), cx).as_deref(),
        Some(ProblemCode::MalformedFrontMatter.guidance())
    );
    assert!(inside(&shell, ("comment-path", 1usize), "discussion", cx));
    assert!(!shell.present(("comment-author", 1usize), cx));
    assert!(!shell.present(("comment-body", 1usize), cx));
    assert_eq!(
        shell.label("discussion-count", cx),
        Some(format!("{} · oldest first", comments_phrase(1)))
    );
    // The comment before it is shown as ever.
    assert_eq!(
        shell.label(("comment-author", 0usize), cx).as_deref(),
        Some("Ada Lovelace")
    );
    assert!(comment_view(&shell, A, cx).is_some());
}

/// R-63: each comment's body is a read editor holding the body as loaded.
/// Select All and Copy put one comment's Markdown on the clipboard; typed
/// text, Enter, Backspace and paste change nothing and begin no draft. A
/// reply deeper than one level says whom it answers (R-53).
#[gpui_kit::test]
fn comment_bodies_are_read_editors_and_can_be_copied(cx: &mut TestAppContext) {
    let bodies = [
        (
            A,
            "A **bold** start, with `code` and [a link](https://example.com/a).\n",
        ),
        (B, "Second, in reply.\n"),
        (C, "- [ ] a task in a deeper reply\n"),
    ];
    let (shell, _root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), bodies[0].1);
        commit_comment(fixture, root, B, Some(A), 2, Some(BO), bodies[1].1);
        commit_comment(fixture, root, C, Some(B), 3, None, bodies[2].1);
    });

    for (id, body) in bodies {
        let editor = comment_editor(&shell, id, cx);
        assert!(text(&editor, cx) == load(body), "{id}: the body, as loaded");
    }
    for ix in 0..3usize {
        assert!(shell.present(("comment-body", ix), cx), "{ix}");
    }
    assert!(!shell.present(("comment-reply-to", 1usize), cx));
    assert_eq!(
        shell.label(("comment-reply-to", 2usize), cx).as_deref(),
        Some("Reply to Bo Carter")
    );

    let editor = comment_editor(&shell, A, cx);
    focus_editor(&shell, &editor, 0, cx);
    shell.keys("secondary-a secondary-c", cx);
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert!(
        copied.as_deref() == Some(load(bodies[0].1).as_str()),
        "Markdown"
    );

    focus_editor(&shell, &editor, 4, cx);
    shell.visual(cx).simulate_input("typed");
    cx.write_to_clipboard(ClipboardItem::new_string("pasted".to_owned()));
    shell.keys("enter backspace delete secondary-v", cx);
    assert!(
        text(&editor, cx) == load(bodies[0].1),
        "the text is unchanged"
    );
    assert!(
        shell.fixture.draft_store().list().unwrap().is_empty(),
        "no draft is stored"
    );
}

/// R-63 and R-79: a reload keeps each comment's read editor, by its ID. A
/// body that did not change is not loaded again and keeps its caret; a
/// changed body is loaded into the same editor; a comment that is gone
/// takes its editor with it.
#[gpui_kit::test]
fn a_reload_keeps_each_comments_read_editor(cx: &mut TestAppContext) {
    let (shell, root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
        commit_comment(fixture, root, B, None, 2, Some(BO), "Second.\n");
    });
    let (a, b) = (
        comment_view(&shell, A, cx).unwrap(),
        comment_view(&shell, B, cx).unwrap(),
    );
    let (a_editor, b_editor) = (comment_editor(&shell, A, cx), comment_editor(&shell, B, cx));
    a_editor.update(cx, |editor, cx| editor.set_cursor(3, cx));
    let (a_seen, b_seen) = (events(&a_editor, cx), events(&b_editor, cx));

    let reload = |cx: &mut TestAppContext| {
        shell.fixture.refresh(&root);
        pane(&shell, cx).update(cx, |pane, cx| {
            pane.reload(cx);
        });
        shell.run(cx);
    };
    commit_comment(
        &shell.fixture,
        &root,
        B,
        None,
        2,
        Some(BO),
        "Second, again.\n",
    );
    reload(cx);

    assert_eq!(comment_view(&shell, A, cx), Some(a), "the same read view");
    assert_eq!(
        comment_view(&shell, B, cx).as_ref(),
        Some(&b),
        "the same read view"
    );
    assert!(text(&a_editor, cx) == "First.\n");
    assert_eq!(cursor(&a_editor, cx), 3, "the caret is kept");
    assert!(
        a_seen.borrow().is_empty(),
        "an unchanged body is not loaded again"
    );
    assert!(text(&b_editor, cx) == "Second, again.\n");
    assert!(!b_seen.borrow().is_empty(), "a changed body is loaded");

    shell.fixture.commit_removal(&root, &comment_path(A));
    reload(cx);
    assert!(
        comment_view(&shell, A, cx).is_none(),
        "gone with its comment"
    );
    assert_eq!(comment_view(&shell, B, cx), Some(b));
}

/// The header's Comments {n}, by click, and `secondary-shift-c` from the
/// page or the tab strip each scroll the discussion to the top of the
/// item's region and focus its heading.
#[gpui_kit::test]
fn the_comments_button_and_its_key_jump_and_focus_the_heading(cx: &mut TestAppContext) {
    let (shell, _root) = shell(SHORT_WINDOW, cx, |fixture, root| {
        commit_long_ticket(fixture, root);
        commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
        commit_comment(fixture, root, B, Some(A), 2, Some(BO), "Second.\n");
    });
    assert_eq!(
        shell.label("item-comments", cx).as_deref(),
        Some("Comments 2")
    );
    assert!(inside(&shell, "item-comments", "item-header", cx));
    assert_eq!(offset(&shell, cx), px(0.));

    let jumped = |shell: &Shell, how: &str, cx: &mut TestAppContext| {
        assert!(offset(shell, cx) < px(0.), "{how}: the region moved");
        let (region, discussion) = tops(shell, cx);
        assert!(
            (discussion - region).abs() < px(1.),
            "{how}: the discussion is at the region's top: {discussion:?} {region:?}"
        );
        assert!(
            holds_focus(shell, "discussion-heading", cx),
            "{how}: the heading has focus"
        );
    };
    shell.click("item-comments", cx);
    jumped(&shell, "Comments", cx);

    shell.keys("home", cx);
    assert_eq!(offset(&shell, cx), px(0.));
    let editor = page_editor(&shell, cx);
    focus_editor(&shell, &editor, 0, cx);
    shell.keys("secondary-shift-c", cx);
    jumped(&shell, "secondary-shift-c from the page", cx);

    // From the tab strip too: it is part of the item tab (R-47).
    shell.keys("home secondary-3", cx);
    assert_eq!(offset(&shell, cx), px(0.));
    assert!(
        holds_focus(&shell, ("tab", 0usize), cx),
        "the strip has focus"
    );
    shell.keys("secondary-shift-c", cx);
    jumped(&shell, "secondary-shift-c from the strip", cx);
}

/// The floating "{n} comments" button shows while the discussion is below
/// the fold, jumps to it, and hides once the discussion's header is in
/// the item's region.
#[gpui_kit::test]
fn the_floating_button_hides_once_the_discussion_shows(cx: &mut TestAppContext) {
    let (shell, _root) = shell(SHORT_WINDOW, cx, |fixture, root| {
        commit_long_ticket(fixture, root);
        commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
        commit_comment(fixture, root, B, None, 2, Some(BO), "Second.\n");
    });
    assert_eq!(shell.label("discussion-jump", cx), Some(comments_phrase(2)));
    assert!(inside(&shell, "discussion-jump", "item-region", cx));

    shell.click("discussion-jump", cx);
    assert!(offset(&shell, cx) < px(0.), "the region moved");
    assert!(holds_focus(&shell, "discussion-heading", cx));
    assert!(!shell.present("discussion-jump", cx), "hidden once shown");

    shell.keys("home", cx);
    assert_eq!(offset(&shell, cx), px(0.));
    assert!(shell.present("discussion-jump", cx), "shown again above it");
    shell.keys("end", cx);
    assert!(!shell.present("discussion-jump", cx), "hidden at the end");
}

/// With no comments the discussion says so, and the floating button never
/// shows, though the discussion is below the fold.
#[gpui_kit::test]
fn no_comments_says_so_and_hides_the_floating_button(cx: &mut TestAppContext) {
    let (shell, _root) = shell(SHORT_WINDOW, cx, commit_long_ticket);
    assert_eq!(
        shell.label("discussion-empty", cx).as_deref(),
        Some(NO_COMMENTS)
    );
    assert_eq!(
        shell.label("discussion-count", cx),
        Some(format!("{} · oldest first", comments_phrase(0)))
    );
    assert!(!shell.present(("comment", 0usize), cx));
    let (region, discussion) = tops(&shell, cx);
    assert!(discussion > region + px(100.), "below the fold");
    assert!(!shell.present("discussion-jump", cx));
    shell.keys("secondary-3 end", cx);
    assert!(offset(&shell, cx) < px(0.));
    assert!(!shell.present("discussion-jump", cx));
}

/// The discussion ends with Comment, disabled, with "Available in a later
/// release" written below it. Pressing it does nothing.
#[gpui_kit::test]
fn comment_is_disabled_with_its_note(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
    });
    // After the header's Edit, Share and Close ticket….
    let comment = 3usize;
    assert_eq!(
        shell.label(("later-action", comment), cx).as_deref(),
        Some(COMMENT)
    );
    assert!(inside(&shell, ("later-action", comment), "discussion", cx));
    assert_eq!(
        shell.label(("later-note", comment), cx).as_deref(),
        Some(LATER_NOTE)
    );
    assert!(inside(&shell, ("later-note", comment), "discussion", cx));
    assert!(shell.inert_button(("later-button", comment), cx));
    assert!(!shell.present(("later-action", comment + 1), cx));

    // Below the fold: brought into view, then pressed.
    shell.keys("secondary-3 end", cx);
    let before = shell.rows("comment-author", cx);
    shell.click(("later-button", comment), cx);
    assert_eq!(shell.rows("comment-author", cx), before);
    assert!(shell.fixture.draft_store().list().unwrap().is_empty());
    assert!(comment_view(&shell, A, cx).is_some());
}

/// R-67 and the Tab order: from the page, Tab reaches Back to top, then
/// each comment in the library's order, each one's caret brought into
/// view; Shift-Tab goes back.
#[gpui_kit::test]
fn tab_runs_through_each_comment_and_reveals_its_caret(cx: &mut TestAppContext) {
    let long = |name: &str| -> String {
        (0..30)
            .map(|line| format!("Line {line} of {name}.\n"))
            .collect()
    };
    let (shell, _root) = shell(SHORT_WINDOW, cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), &long("A"));
        commit_comment(fixture, root, B, Some(A), 2, Some(BO), &long("B"));
        commit_comment(fixture, root, C, None, 3, None, &long("C"));
    });
    let page = page_editor(&shell, cx);
    focus_editor(&shell, &page, 0, cx);
    shell.keys("tab", cx);
    assert!(holds_focus(&shell, "back-to-top", cx), "Back to top first");

    for id in [A, B, C] {
        shell.keys("tab", cx);
        let editor = comment_editor(&shell, id, cx);
        assert!(focused(&shell, &editor, cx), "Tab reached {id}");
        assert_eq!(cursor(&editor, cx), 0, "{id}: the caret stayed");
        assert!(
            caret_in_view(&shell, &editor, cx),
            "{id}: the caret was revealed"
        );
    }
    assert!(offset(&shell, cx) < px(0.));

    shell.keys("shift-tab", cx);
    let b = comment_editor(&shell, B, cx);
    assert!(focused(&shell, &b, cx), "Shift-Tab went back to B");
    assert!(caret_in_view(&shell, &b, cx));
}
