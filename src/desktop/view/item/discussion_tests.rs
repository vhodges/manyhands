//! The discussion (design, Discussion; R-53, R-56, R-63, R-67), over real
//! repositories: a file that is no comment stays in view with its path and
//! reason; each comment's body is a read editor, kept by its ID across
//! reloads, that copies as Markdown and takes no edit; the header's
//! Comments {n} and `secondary-shift-c` jump to the discussion and focus
//! its heading; the floating button hides once the discussion shows and
//! when there are no comments; Tab runs through the comments and each
//! one's Reply, bringing each caret into view; and the composers: the new
//! comment's after the threads, the one reply composer in the thread whose
//! Reply was pressed, answering its root and keeping each thread's text;
//! and the new comment's composer collapsed to one Write a comment… stop
//! until it is opened (decision 22).

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::{
    ClipboardItem, ElementId, Entity, Focusable, Pixels, TestAppContext, px, size,
    test::TestWindowExt,
};
use manyhands::{
    drafts::{CommentTarget, DraftFields, Entry, Held},
    repository::ItemDtoKind,
    results::ProblemCode,
};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};
use zorite_editor::{EditorEvent, EditorState};

use super::{
    ItemPane,
    composer::Composer,
    discussion::{NO_COMMENTS, comments_phrase, shown_time},
};
use crate::desktop::{
    model::{drafts::IDLE, editor_text::load, navigation::OpenTarget},
    test_support::fixtures::{Fixture, item_id, ticket_path, ticket_source},
    view::{
        editor::read_only::ReadOnly,
        harness::{Shell, key},
    },
};

pub(super) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
pub(super) const A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
pub(super) const B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
pub(super) const C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
const BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE9";
pub(super) const ADA: &str = "Ada Lovelace <ada@example.invalid>";
pub(super) const BO: &str = "Bo Carter <bo@example.invalid>";
/// A window short enough that the discussion's comments do not all fit
/// its region.
const SHORT_WINDOW: f32 = 700.;

pub(super) fn comment_path(id: &str) -> String {
    format!(".manyhands/comments/{TICKET}/{id}.md")
}

/// Commits the comment `id` on `TICKET`, `second`s after noon, by
/// `author` when there is one, with `body`.
pub(super) fn commit_comment(
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
pub(super) fn shell(
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

pub(super) fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
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
pub(super) fn comment_editor(
    shell: &Shell,
    id: &str,
    cx: &mut TestAppContext,
) -> Entity<EditorState> {
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

pub(super) fn text(editor: &Entity<EditorState>, cx: &mut TestAppContext) -> String {
    editor.read_with(cx, |editor, _| editor.text().to_owned())
}

fn cursor(editor: &Entity<EditorState>, cx: &mut TestAppContext) -> usize {
    editor.read_with(cx, |editor, _| editor.cursor())
}

/// Whether `editor` itself holds focus, after a frame.
pub(super) fn focused(
    shell: &Shell,
    editor: &Entity<EditorState>,
    cx: &mut TestAppContext,
) -> bool {
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Whether the element `id` holds focus.
pub(super) fn holds_focus(
    shell: &Shell,
    id: impl Into<ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether the element `id` is drawn inside `within`.
pub(super) fn inside(
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
pub(super) fn focus_editor(
    shell: &Shell,
    editor: &Entity<EditorState>,
    at: usize,
    cx: &mut TestAppContext,
) {
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
    // The heading is no tab stop: one Tab from it reaches Back to top.
    shell.keys("tab", cx);
    assert!(
        holds_focus(&shell, "back-to-top", cx),
        "Tab from the heading"
    );

    shell.keys("home", cx);
    assert_eq!(offset(&shell, cx), px(0.));
    // Behind the open details sheet the key does nothing (R-60).
    shell.click("chip-all", cx);
    assert!(shell.present("item-id", cx), "the sheet is open");
    shell.keys("secondary-shift-c", cx);
    assert_eq!(offset(&shell, cx), px(0.), "no jump behind the sheet");
    assert!(!holds_focus(&shell, "discussion-heading", cx));
    shell.keys("escape", cx);
    assert!(!shell.present("item-id", cx), "the sheet is closed");
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

    for (ix, id) in [A, B, C].into_iter().enumerate() {
        shell.keys("tab", cx);
        let editor = comment_editor(&shell, id, cx);
        assert!(focused(&shell, &editor, cx), "Tab reached {id}");
        assert_eq!(cursor(&editor, cx), 0, "{id}: the caret stayed");
        assert!(
            caret_in_view(&shell, &editor, cx),
            "{id}: the caret was revealed"
        );
        // E-11: the comment's Reply comes after it.
        shell.keys("tab", cx);
        assert!(
            holds_focus(&shell, ("comment-reply", ix), cx),
            "Tab reached {id}'s Reply"
        );
    }
    assert!(offset(&shell, cx) < px(0.));

    shell.keys("shift-tab shift-tab shift-tab", cx);
    let b = comment_editor(&shell, B, cx);
    assert!(focused(&shell, &b, cx), "Shift-Tab went back to B");
    assert!(caret_in_view(&shell, &b, cx));
}

/// A comment's time is shown at the local offset passed in. Where no local
/// offset is read, as on non-Unix targets, it is in UTC and says so, as
/// `confirm.rs` does; a time that cannot be read is shown as written.
#[test]
fn a_time_with_no_local_offset_is_marked_utc() {
    let now = OffsetDateTime::parse("2026-10-22T20:00:00Z", &Rfc3339).unwrap();
    let at = "2026-10-20T10:14:00Z";
    let two = UtcOffset::from_hms(2, 0, 0).unwrap();
    assert_eq!(shown_time(at, now, Some(two)), "Oct 20, 12:14");
    assert_eq!(shown_time(at, now, Some(UtcOffset::UTC)), "Oct 20, 10:14");
    assert_eq!(shown_time(at, now, None), "Oct 20, 10:14 UTC");
    assert_eq!(shown_time("not a time", now, None), "not a time");
}

/// Select All, Copy, typed text, Enter, Backspace and paste in a comment's
/// read editor queue no request.
#[gpui_kit::test]
fn comment_bodies_queue_no_request(cx: &mut TestAppContext) {
    let body = "A **bold** start, with `code`.\n";
    let (shell, _root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), body);
    });
    let requests = shell.requests(cx, |_| true).len();
    let editor = comment_editor(&shell, A, cx);
    focus_editor(&shell, &editor, 0, cx);
    shell.keys("secondary-a secondary-c", cx);
    focus_editor(&shell, &editor, 4, cx);
    shell.visual(cx).simulate_input("typed");
    cx.write_to_clipboard(ClipboardItem::new_string("pasted".to_owned()));
    shell.keys("enter backspace delete secondary-v", cx);
    shell.run(cx);
    assert_eq!(shell.requests(cx, |_| true).len(), requests);
}

/// The new comment's composer, after a frame.
pub(super) fn new_composer(shell: &Shell, cx: &mut TestAppContext) -> Entity<Composer> {
    shell.frame(cx, |_, _| ());
    pane(shell, cx)
        .read_with(cx, |pane, _| pane.new_composer())
        .expect("the new comment's composer is shown")
}

/// The reply composer and the root of the thread it is in, after a frame.
pub(super) fn reply_composer(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Option<(String, Entity<Composer>)> {
    shell.frame(cx, |_, _| ());
    pane(shell, cx).read_with(cx, |pane, _| pane.reply_composer())
}

/// The comment `composer` writes: its ID and parent, once allocated.
pub(super) fn composer_target(
    shell: &Shell,
    composer: &Entity<Composer>,
    cx: &mut TestAppContext,
) -> Option<CommentTarget> {
    pane(shell, cx).read_with(cx, |pane, _| pane.composer_target(composer))
}

pub(super) fn composer_text(composer: &Entity<Composer>, cx: &mut TestAppContext) -> String {
    composer.read_with(cx, |composer, cx| composer.text(cx))
}

/// Types `typed` at the end of `composer`'s text, through the window: the
/// caret set there, the editor focused and a frame drawn first.
pub(super) fn type_into(
    shell: &Shell,
    composer: &Entity<Composer>,
    typed: &str,
    cx: &mut TestAppContext,
) {
    let editor = composer.read_with(cx, |composer, _| composer.editor().clone());
    let end = editor.read_with(cx, |editor, _| editor.text().len());
    focus_editor(shell, &editor, end, cx);
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
}

/// Lets the idle delay pass and every draft write land.
pub(super) fn flushed(shell: &Shell, cx: &mut TestAppContext) {
    shell.run(cx);
    cx.executor().advance_clock(IDLE * 2);
    shell.run(cx);
}

/// The comment drafts in the store: each one's target, text and holder,
/// in ID order.
pub(super) fn stored_comments(fixture: &Fixture) -> Vec<(CommentTarget, String, Held)> {
    fixture
        .draft_store()
        .list()
        .unwrap()
        .into_iter()
        .filter_map(|listing| match listing.entry {
            Entry::Draft(draft) => match (draft.target.comment, draft.current) {
                (Some(target), DraftFields::Comment { body }) => Some((target, body, listing.held)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Opens the new comment's collapsed composer by Write a comment…, the
/// discussion brought into the region first by the header's Comments.
pub(super) fn open_new_composer(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("item-comments", cx);
    shell.click("new-comment-start", cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
}

/// Presses the Reply of the `ix`th comment, the discussion brought into
/// the region first by the header's Comments.
pub(super) fn press_reply(shell: &Shell, ix: usize, cx: &mut TestAppContext) {
    shell.click("item-comments", cx);
    shell.click(("comment-reply", ix), cx);
}

/// The window-space bounds of the element `id`, and the IDs of the
/// elements it is drawn inside.
fn placed(
    shell: &Shell,
    id: impl Into<ElementId>,
    cx: &mut TestAppContext,
) -> Option<(gpui_kit::Bounds<Pixels>, Vec<ElementId>)> {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .map(|element| (element.bounds(), element.path().to_vec()))
    })
}

/// Whether the element `id` is drawn inside the element `within`, and
/// below the element `after`.
fn placed_after(
    shell: &Shell,
    id: impl Into<ElementId>,
    within: impl Into<ElementId>,
    after: impl Into<ElementId>,
    cx: &mut TestAppContext,
) -> bool {
    let (Some((bounds, path)), Some((before, _))) =
        (placed(shell, id, cx), placed(shell, after, cx))
    else {
        return false;
    };
    path.contains(&within.into()) && bounds.top() >= before.bottom() - px(1.)
}

/// Comments A (Ada), B (Bo, replying to A) and C (no author, a root).
fn two_threads(fixture: &Fixture, root: &Path) {
    fixture.commit_ticket(root, TICKET, "Discussed", "");
    commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
    commit_comment(fixture, root, B, Some(A), 2, Some(BO), "In reply.\n");
    commit_comment(fixture, root, C, None, 3, Some(BO), "Another.\n");
}

/// E-7: the new comment's composer follows the threads, inside the
/// discussion, with its placeholder, and no reply composer is open.
#[gpui_kit::test]
fn the_new_comment_composer_follows_the_threads(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    assert!(placed_after(
        &shell,
        "new-comment-composer",
        "discussion",
        ("thread", 1usize),
        cx
    ));
    let composer = new_composer(&shell, cx);
    assert_eq!(
        composer.read_with(cx, |composer, _| composer.placeholder()),
        "Add a comment. Markdown works here."
    );
    assert!(composer_text(&composer, cx).is_empty());
    assert!(reply_composer(&shell, cx).is_none());
    assert!(!shell.present("reply-composer", cx));
}

/// Reply opens the one reply composer in its thread, after its last
/// reply, with focus in it; Reply in another thread moves it there.
#[gpui_kit::test]
fn reply_opens_one_composer_in_its_thread(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    press_reply(&shell, 0, cx);
    let (root, composer) = reply_composer(&shell, cx).expect("a reply composer is open");
    assert_eq!(root, A);
    assert!(placed_after(
        &shell,
        "reply-composer",
        ("thread", 0usize),
        ("comment", 1usize),
        cx
    ));
    let editor = composer.read_with(cx, |composer, _| composer.editor().clone());
    assert!(focused(&shell, &editor, cx), "focus is in the composer");
    assert_eq!(
        composer.read_with(cx, |composer, _| composer.placeholder()),
        "Write a reply. Markdown works here."
    );
    assert_eq!(
        shell.label("composer-replying-to", cx).as_deref(),
        Some("Replying to Ada Lovelace")
    );

    press_reply(&shell, 2, cx);
    let (root, moved) = reply_composer(&shell, cx).expect("a reply composer is open");
    assert_eq!(root, C);
    assert!(placed_after(
        &shell,
        "reply-composer",
        ("thread", 1usize),
        ("comment", 2usize),
        cx
    ));
    let in_first = placed(&shell, "reply-composer", cx)
        .is_some_and(|(_, path)| path.contains(&ElementId::from(("thread", 0usize))));
    assert!(!in_first, "one reply composer, in the second thread");
    let editor = moved.read_with(cx, |composer, _| composer.editor().clone());
    assert!(focused(&shell, &editor, cx));
    // The new comment's composer stays after the threads.
    assert!(shell.present("new-comment-composer", cx));
}

/// Decision 5: Reply on a reply answers the thread's root, and "Replying
/// to" names the comment it was pressed on.
#[gpui_kit::test]
fn reply_on_a_reply_targets_its_root_and_names_the_comment(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    press_reply(&shell, 1, cx);
    let (root, composer) = reply_composer(&shell, cx).expect("a reply composer is open");
    assert_eq!(root, A, "in A's thread");
    assert_eq!(
        shell.label("composer-replying-to", cx).as_deref(),
        Some("Replying to Bo Carter")
    );
    type_into(&shell, &composer, "Agreed.", cx);
    flushed(&shell, cx);
    let target = composer_target(&shell, &composer, cx).expect("a comment ID is allocated");
    assert_eq!(target.parent_id, Some(item_id(A)), "the root is the parent");
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0, target);
    assert_eq!(stored[0].1, "Agreed.");
}

/// Moving the reply composer keeps the thread it leaves's text and draft,
/// and brings back the thread it reaches's.
#[gpui_kit::test]
fn moving_the_reply_keeps_each_threads_draft(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    press_reply(&shell, 0, cx);
    let (_, first) = reply_composer(&shell, cx).unwrap();
    type_into(&shell, &first, "For A.", cx);
    let for_a = composer_target(&shell, &first, cx).unwrap();

    press_reply(&shell, 2, cx);
    let (_, second) = reply_composer(&shell, cx).unwrap();
    assert!(composer_text(&second, cx).is_empty(), "C's thread has none");
    type_into(&shell, &second, "For C.", cx);
    let for_c = composer_target(&shell, &second, cx).unwrap();
    assert_ne!(for_a.comment_id, for_c.comment_id);
    assert_eq!(for_c.parent_id, Some(item_id(C)));

    // Back in A's thread from B's Reply: A's text, its comment, B named.
    press_reply(&shell, 1, cx);
    let (root, back) = reply_composer(&shell, cx).unwrap();
    assert_eq!(root, A);
    assert_eq!(composer_text(&back, cx), "For A.");
    assert_eq!(composer_target(&shell, &back, cx), Some(for_a.clone()));
    assert_eq!(
        shell.label("composer-replying-to", cx).as_deref(),
        Some("Replying to Bo Carter")
    );
    press_reply(&shell, 2, cx);
    let (_, again) = reply_composer(&shell, cx).unwrap();
    assert_eq!(composer_text(&again, cx), "For C.");
    assert_eq!(composer_target(&shell, &again, cx), Some(for_c.clone()));

    flushed(&shell, cx);
    let stored: Vec<_> = stored_comments(&shell.fixture)
        .into_iter()
        .map(|(target, body, _)| (target, body))
        .collect();
    assert_eq!(stored.len(), 2, "each thread's draft is kept");
    assert!(stored.contains(&(for_a, "For A.".to_owned())));
    assert!(stored.contains(&(for_c, "For C.".to_owned())));
}

/// Decision 22: an empty new-comment composer is one tab stop, Write a
/// comment…, with no editor drawn; Tab passes on from it, and Shift-Tab
/// comes back to it.
#[gpui_kit::test]
fn an_empty_new_comment_composer_is_one_tab_stop(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
    });
    assert_eq!(
        shell.label("new-comment-start", cx).as_deref(),
        Some("Write a comment\u{2026}")
    );
    assert!(!shell.present("composer-editor", cx), "no editor is drawn");
    shell.click("item-comments", cx);
    shell.keys("tab", cx);
    assert!(holds_focus(&shell, "back-to-top", cx));
    shell.keys("tab", cx);
    assert!(holds_focus(&shell, "new-comment-start", cx), "one stop");
    shell.keys("tab", cx);
    assert!(!holds_focus(&shell, "new-comment-start", cx));
    let in_composer = shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|element| {
                element.focused() == Some(true)
                    && element
                        .path()
                        .contains(&ElementId::from("new-comment-composer"))
            })
    });
    assert!(!in_composer, "Tab passed on from it");
    assert!(!shell.present("composer-editor", cx), "still collapsed");
    shell.keys("shift-tab", cx);
    assert!(
        holds_focus(&shell, "new-comment-start", cx),
        "Shift-Tab back"
    );
}

/// Decision 22: Enter or a click on Write a comment… opens the composer
/// with focus in its editor. With text it stays open when focus leaves;
/// empty, it collapses again.
#[gpui_kit::test]
fn write_a_comment_opens_the_composer_with_focus(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    shell.click("item-comments", cx);
    assert!(shell.tab_reaches("new-comment-start", 12, cx));
    shell.press("enter", cx);
    let composer = new_composer(&shell, cx);
    let editor = composer.read_with(cx, |composer, _| composer.editor().clone());
    assert!(focused(&shell, &editor, cx), "focus is in the editor");
    assert!(!shell.present("new-comment-start", cx), "open");
    assert!(placed_after(
        &shell,
        "composer-editor",
        "new-comment-composer",
        ("thread", 1usize),
        cx
    ));

    // Empty, it collapses once focus leaves it.
    shell.keys("f6", cx);
    assert!(shell.present("new-comment-start", cx), "collapsed again");

    // By click; with text it stays open when focus leaves.
    open_new_composer(&shell, cx);
    assert!(focused(&shell, &editor, cx), "a click opens it with focus");
    shell.visual(cx).simulate_input("Kept open.");
    shell.run(cx);
    shell.keys("f6", cx);
    assert!(!shell.present("new-comment-start", cx), "still open");
    assert_eq!(composer_text(&composer, cx), "Kept open.");
}

/// Space presses Reply and Write a comment…, as Enter and a click do.
#[gpui_kit::test]
fn reply_and_write_a_comment_open_by_space(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    shell.click("item-comments", cx);
    assert!(shell.tab_reaches(("comment-reply", 0usize), 4, cx));
    shell.press("space", cx);
    let (root, reply) = reply_composer(&shell, cx).expect("Space opened the reply");
    assert_eq!(root, A);
    let editor = reply.read_with(cx, |composer, _| composer.editor().clone());
    assert!(focused(&shell, &editor, cx));

    // An empty Cancel closes it, focus back on its Reply.
    shell.keys("escape", cx);
    assert!(reply_composer(&shell, cx).is_none());
    assert!(holds_focus(&shell, ("comment-reply", 0usize), cx));
    assert!(shell.tab_reaches("new-comment-start", 8, cx));
    shell.press("space", cx);
    let composer = new_composer(&shell, cx);
    let editor = composer.read_with(cx, |composer, _| composer.editor().clone());
    assert!(focused(&shell, &editor, cx), "Space opened the composer");
    assert!(!shell.present("new-comment-start", cx));
}

/// Text typed and then deleted leaves a draft with no text: an empty
/// Cancel discards it, and the composer starts again collapsed, with focus
/// on Write a comment….
#[gpui_kit::test]
fn an_empty_cancel_discards_the_new_comments_draft_and_collapses(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    open_new_composer(&shell, cx);
    let composer = new_composer(&shell, cx);
    type_into(&shell, &composer, "Gone.", cx);
    flushed(&shell, cx);
    assert_eq!(stored_comments(&shell.fixture).len(), 1);
    shell.keys("secondary-a backspace", cx);
    flushed(&shell, cx);
    assert!(composer_text(&composer, cx).is_empty());

    shell.keys("escape", cx);
    flushed(&shell, cx);
    assert!(stored_comments(&shell.fixture).is_empty(), "the draft went");
    assert!(shell.present("new-comment-start", cx), "collapsed");
    assert!(holds_focus(&shell, "new-comment-start", cx));
    let fresh = new_composer(&shell, cx);
    assert!(composer_text(&fresh, cx).is_empty());
    assert_eq!(composer_target(&shell, &fresh, cx), None, "no comment ID");
}

/// A reply composer moved away with no text, but a draft, is not kept:
/// its draft goes, and its thread's next reply starts afresh.
#[gpui_kit::test]
fn moving_an_emptied_reply_away_discards_its_draft(cx: &mut TestAppContext) {
    let (shell, _root) = shell(1400., cx, two_threads);
    press_reply(&shell, 0, cx);
    let (_, reply) = reply_composer(&shell, cx).unwrap();
    type_into(&shell, &reply, "Gone.", cx);
    flushed(&shell, cx);
    assert_eq!(stored_comments(&shell.fixture).len(), 1);
    shell.keys("secondary-a backspace", cx);
    flushed(&shell, cx);

    press_reply(&shell, 2, cx);
    flushed(&shell, cx);
    assert!(stored_comments(&shell.fixture).is_empty(), "the draft went");
    press_reply(&shell, 0, cx);
    let (root, again) = reply_composer(&shell, cx).unwrap();
    assert_eq!(root, A);
    assert!(composer_text(&again, cx).is_empty());
    assert_eq!(composer_target(&shell, &again, cx), None, "afresh");
}
