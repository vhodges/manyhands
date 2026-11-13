//! D4's discussion by keyboard alone (Cycle A1, contracts 9 and 2; design,
//! Part E), with no publication remote: a user who never touches the
//! pointer writes a comment from Write a comment…, uses Ctrl K and the
//! mode key in it, saves it with Ctrl Enter, replies to it from a
//! comment's Reply, and cancels a third with Escape, keeping and then
//! discarding its text; and the item tab's Tab order, from the strip
//! through the header, its Share, the chips and the page into the
//! discussion, each comment and its Reply, an open reply composer and
//! Write a comment… (decision 22), with Tab indenting inside an open
//! composer and F6 leaving it (decision 19).
//!
//! Every step is a keystroke; after each the walk asserts which element
//! holds focus, that it is on screen and that the content region alone
//! draws its ring, as D1's and D3's walks do. The per-feature tests
//! (`discussion_tests`, `composer_tests`, `posting_tests`) pin each piece;
//! these prove they are reached end to end.

use std::{fs, path::Path};

use gpui_kit::{ElementId, Entity, TestAppContext};
use zorite_editor::EditorState;

use super::Walk;
use crate::desktop::{
    test_support::fixtures::Fixture,
    view::{harness::Shell, workspace::Region},
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK1";
const REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK2";
const LAST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK3";
const ADA: &str = "Ada Lovelace <ada@example.invalid>";

/// What the journey writes.
const COMMENTED: &str = "A comment by keyboard.";
const REPLIED: &str = "A reply by keyboard.";
const DISCARDED: &str = "Not this one.";

/// Commits the comment `id` on `MAIN`, `second`s after noon, by Ada.
fn commit_comment(fixture: &Fixture, root: &Path, id: &str, parent: Option<&str>, second: u32) {
    let parent = parent
        .map(|parent| format!("parent_id: {parent}\n"))
        .unwrap_or_default();
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {MAIN}\n\
         {parent}created_at: 2026-09-30T12:00:{second:02}Z\ncreated_by: {ADA}\n---\nSaid {id}.\n"
    );
    fixture.commit_file(
        root,
        &format!(".manyhands/comments/{MAIN}/{id}.md"),
        source.as_bytes(),
    );
}

/// A tall window over `a`, holding `MAIN` with a project and a team and
/// the comments `comments` (ID and parent), no publication remote.
fn discussed(comments: &[(&str, Option<&str>)], cx: &mut TestAppContext) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_ticket(&a, MAIN, "Main", "project: p\nteam: t\n");
    for (second, (id, parent)) in (1..).zip(comments) {
        commit_comment(&fixture, &a, id, *parent, second);
    }
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    shell.resize_keeping_content(1440., 1400., cx);
    shell.run(cx);
    Walk::over(shell, a)
}

impl Walk {
    /// The new comment's composer's editor, in a frame drawn now.
    fn new_editor(&self, cx: &mut TestAppContext) -> Entity<EditorState> {
        self.shell.frame(cx, |_, _| ());
        let composer = self
            .pane(cx)
            .read_with(cx, |pane, _| pane.new_composer())
            .expect("the new comment's composer");
        composer.read_with(cx, |composer, _| composer.editor().clone())
    }

    /// The reply composer's thread root and editor, in a frame drawn now.
    fn reply_editor(&self, cx: &mut TestAppContext) -> Option<(String, Entity<EditorState>)> {
        self.shell.frame(cx, |_, _| ());
        let (root, composer) = self
            .pane(cx)
            .read_with(cx, |pane, _| pane.reply_composer())?;
        Some((
            root,
            composer.read_with(cx, |composer, _| composer.editor().clone()),
        ))
    }

    /// Whether the new comment's composer is in rich text.
    fn new_is_rich(&self, cx: &mut TestAppContext) -> bool {
        self.pane(cx)
            .read_with(cx, |pane, _| pane.new_composer())
            .expect("the new comment's composer")
            .read_with(cx, |composer, _| composer.is_rich())
    }

    /// The read editor of the comment `id`, in a frame drawn now.
    fn comment_editor(&self, id: &str, cx: &mut TestAppContext) -> Entity<EditorState> {
        self.shell.frame(cx, |_, _| ());
        let view = self
            .pane(cx)
            .read_with(cx, |pane, _| pane.comment_view(id))
            .unwrap_or_else(|| panic!("{id} has a read editor"));
        view.read_with(cx, |view, _| view.editor().clone())
    }

    /// Presses `key`; then `editor` holds focus, in the content region.
    fn step_into(
        &self,
        key: &str,
        editor: &Entity<EditorState>,
        what: &str,
        cx: &mut TestAppContext,
    ) {
        self.keys(key, cx);
        assert!(self.editor_focused(editor, cx), "{key} reached {what}");
        self.region(Region::Content, cx);
    }

    /// Waits, the worker running, until the discussion lists `count`
    /// comments.
    async fn until_listed(&self, count: usize, cx: &mut TestAppContext) {
        self.shell
            .until_drawn(cx, "the comment was listed", move |shell, cx| {
                shell.present(("comment", count - 1), cx) && !shell.present(("comment", count), cx)
            })
            .await;
    }

    /// The comment files of `MAIN` in its editing context, where a post
    /// writes them, but for those committed before: each one's source.
    fn posted(&self) -> Vec<String> {
        let dir = self
            .a
            .join(".manyhands/worktrees")
            .join(MAIN)
            .join(".manyhands/comments")
            .join(MAIN);
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| fs::read_to_string(entry.ok()?.path()).ok())
            .filter(|source| !source.contains("\nSaid "))
            .collect()
    }

    /// `secondary-shift-c`: the discussion's heading takes focus.
    fn to_discussion(&self, cx: &mut TestAppContext) {
        self.keys("secondary-shift-c", cx);
        self.on("discussion-heading", Region::Content, cx);
    }
}

/// A1, with no remote: the ticket found and its tab focused by keyboard;
/// the discussion by its key; Write a comment… by Tab and Enter; Ctrl K
/// opens the link field and Escape closes it; the mode key switches to
/// Markdown and back; Ctrl Enter saves the comment, which is listed and
/// leaves the composer empty. Then a comment's Reply by Tab and Enter,
/// focus in the reply composer, which names the comment; Ctrl Enter saves
/// the reply, answering that comment. Then Escape with text asks first:
/// Enter keeps the text (Keep editing is the default), Shift-Tab and Enter
/// discard it, and Escape with no text collapses the composer to Write a
/// comment…, focused. Two comment files are written, nothing more.
#[gpui_kit::test]
async fn a_keyboard_only_user_comments_replies_and_posts(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = discussed(&[(FIRST, None)], cx);
    walk.select_a_repository(cx);
    walk.find(MAIN, cx);
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);

    // Write a comment…, by Tab and Enter.
    walk.to_discussion(cx);
    walk.tab_to("tab", "new-comment-start", cx);
    walk.on("new-comment-start", Region::Content, cx);
    assert_eq!(
        walk.shell.label("new-comment-start", cx).as_deref(),
        Some("Write a comment\u{2026}")
    );
    walk.enter(cx);
    let editor = walk.new_editor(cx);
    assert!(walk.editor_focused(&editor, cx), "Enter opened it, focused");
    walk.region(Region::Content, cx);

    // Ctrl K opens the link field, focused; Escape closes it and gives
    // focus back to the editor, the composer still open.
    walk.keys("secondary-k", cx);
    assert!(walk.shell.present("composer-link-field", cx), "Ctrl K");
    walk.keys("escape", cx);
    assert!(!walk.shell.present("composer-link-field", cx), "closed");
    assert!(walk.editor_focused(&editor, cx), "focus back in the editor");
    assert!(!walk.shell.present("new-comment-start", cx), "still open");

    // Typed, the mode key there and back, then Ctrl Enter.
    walk.type_text(COMMENTED, cx);
    walk.keys("secondary-shift-m", cx);
    assert!(!walk.new_is_rich(cx), "Markdown");
    walk.keys("secondary-shift-m", cx);
    assert!(walk.new_is_rich(cx), "rich text again");
    assert!(walk.editor_focused(&editor, cx));
    walk.keys("secondary-enter", cx);
    walk.until_listed(2, cx).await;
    let editor = walk.new_editor(cx);
    assert!(
        editor.read_with(cx, |editor, _| editor.text().is_empty()),
        "the composer is cleared"
    );

    // A comment's Reply, by Tab and Enter: the reply composer, focused.
    walk.to_discussion(cx);
    walk.tab_to("tab", ("comment-reply", 0usize), cx);
    walk.on(("comment-reply", 0usize), Region::Content, cx);
    walk.enter(cx);
    let (root, reply) = walk.reply_editor(cx).expect("Enter opened the reply");
    assert_eq!(root, FIRST);
    assert!(walk.editor_focused(&reply, cx), "focus is in the reply");
    walk.region(Region::Content, cx);
    assert_eq!(
        walk.shell.label("composer-replying-to", cx).as_deref(),
        Some("Replying to Ada Lovelace")
    );
    walk.type_text(REPLIED, cx);
    walk.keys("secondary-enter", cx);
    walk.until_listed(3, cx).await;

    // Escape with text asks; Enter keeps the text.
    walk.to_discussion(cx);
    walk.tab_to("tab", "new-comment-start", cx);
    walk.enter(cx);
    let editor = walk.new_editor(cx);
    assert!(walk.editor_focused(&editor, cx));
    walk.type_text(DISCARDED, cx);
    walk.keys("escape", cx);
    assert!(walk.shell.present("composer-discard-dialog", cx), "asked");
    assert!(
        walk.focus_inside("composer-discard-keep-stop", cx),
        "Keep editing is the default"
    );
    walk.enter(cx);
    assert!(!walk.shell.present("composer-discard-dialog", cx));
    assert!(
        editor.read_with(cx, |editor, _| editor.text() == DISCARDED),
        "kept"
    );

    // Escape again; Shift-Tab to Discard and Enter: the text goes, and the
    // composer stays open, empty, with focus.
    walk.keys("escape", cx);
    assert!(walk.shell.present("composer-discard-dialog", cx));
    walk.keys("shift-tab", cx);
    assert!(walk.focus_inside("composer-discard-confirm", cx), "Discard");
    walk.enter(cx);
    assert!(!walk.shell.present("composer-discard-dialog", cx));
    let editor = walk.new_editor(cx);
    assert!(
        editor.read_with(cx, |editor, _| editor.text().is_empty()),
        "discarded"
    );
    assert!(walk.editor_focused(&editor, cx));

    // Escape with no text: collapsed, Write a comment… focused.
    walk.keys("escape", cx);
    walk.on("new-comment-start", Region::Content, cx);

    // One file for the comment, one for the reply, answering its root.
    let posted = walk.posted();
    assert_eq!(posted.len(), 2, "two comments were written");
    let comment = posted.iter().find(|source| source.contains(COMMENTED));
    let reply = posted.iter().find(|source| source.contains(REPLIED));
    assert!(comment.is_some_and(|source| !source.contains("parent_id:")));
    assert!(reply.is_some_and(|source| source.contains(&format!("parent_id: {FIRST}"))));
    assert!(posted.iter().all(|source| !source.contains(DISCARDED)));
}

/// The item tab's Tab order (design, Part E): the strip, the header's
/// Comments, Edit and Share, the chips, the page and its read view, Back
/// to top, each comment and its Reply, then Write a comment…, one stop.
/// With a reply composer open in the first thread, Tab goes from that
/// thread's last Reply into the composer, its toolbar's one stop and then
/// its editor, where Tab indents and F6 leaves; from Write a comment…,
/// Shift-Tab comes back through the next thread and that composer's
/// footer into its editor. Write a comment… ends the item: nothing of the
/// repository follows it. Opened, the new comment's composer takes Tab
/// too: Tab indents in it and F6 leaves it. While editing, the edit
/// toolbar comes between the chips and the page.
#[gpui_kit::test]
async fn tab_order_runs_header_share_chips_page_discussion_composer(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = discussed(&[(FIRST, None), (REPLY, Some(FIRST)), (LAST, None)], cx);
    walk.select_a_repository(cx);
    walk.find(MAIN, cx);
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);

    let header: [ElementId; 11] = [
        ("tab-close-button", 0usize).into(),
        "item-comments".into(),
        "item-edit".into(),
        "item-share".into(),
        "item-lifecycle-action".into(),
        "chip-type".into(),
        "chip-status".into(),
        "chip-project".into(),
        "chip-team".into(),
        "chip-all".into(),
        "item-page".into(),
    ];
    for id in header {
        walk.step("tab", id, Region::Content, cx);
    }
    walk.shell.frame(cx, |_, _| ());
    let read = walk
        .pane(cx)
        .read_with(cx, |pane, cx| pane.read_editor(cx))
        .expect("the read view");
    assert!(walk.editor_focused(&read, cx), "the page's read view");
    walk.step("tab", "back-to-top", Region::Content, cx);
    for (ix, id) in [FIRST, REPLY, LAST].into_iter().enumerate() {
        let editor = walk.comment_editor(id, cx);
        walk.step_into("tab", &editor, id, cx);
        walk.step("tab", ("comment-reply", ix), Region::Content, cx);
    }
    walk.step("tab", "new-comment-start", Region::Content, cx);
    assert!(!walk.shell.present("composer-editor", cx), "collapsed");
    // Write a comment… ends the item: nothing of the repository follows
    // it, so Tab leaves the item area, and Shift-Tab comes back.
    walk.keys("tab", cx);
    let region = walk
        .shell
        .frame(cx, |window, cx| walk.held_region(window, cx));
    assert_ne!(region, Some(Region::Content), "Tab left the item");
    walk.step("shift-tab", "new-comment-start", Region::Content, cx);

    // A reply composer in the first thread, with a list in it, so that
    // Tab has something to indent; F6 leaves it.
    walk.tab_to("shift-tab", ("comment-reply", 1usize), cx);
    walk.enter(cx);
    let (root, reply) = walk.reply_editor(cx).expect("a reply composer");
    assert_eq!(root, FIRST);
    assert!(walk.editor_focused(&reply, cx));
    walk.type_text("- one\n", cx);
    walk.type_text("two", cx);
    let listed = reply.read_with(cx, |editor, _| editor.text().to_owned());
    walk.keys("tab", cx);
    let indented = reply.read_with(cx, |editor, _| editor.text().to_owned());
    assert!(indented != listed, "Tab indented: {indented:?}");
    assert!(walk.editor_focused(&reply, cx), "Tab stayed in the editor");
    walk.keys("f6", cx);
    walk.region(Region::Tickets, cx);

    // The order with it open: the thread's last Reply, its toolbar's stop,
    // its editor; and back from Write a comment… into its editor.
    walk.keys("secondary-3", cx);
    walk.to_discussion(cx);
    walk.step("tab", "back-to-top", Region::Content, cx);
    for (ix, id) in [FIRST, REPLY].into_iter().enumerate() {
        let editor = walk.comment_editor(id, cx);
        walk.step_into("tab", &editor, id, cx);
        walk.step("tab", ("comment-reply", ix), Region::Content, cx);
    }
    walk.step("tab", "composer-bold", Region::Content, cx);
    walk.step_into("tab", &reply, "the reply's editor", cx);
    walk.keys("f6", cx);
    walk.region(Region::Tickets, cx);
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);
    walk.tab_to("shift-tab", "new-comment-start", cx);
    walk.on("new-comment-start", Region::Content, cx);
    let last = walk.comment_editor(LAST, cx);
    walk.step("shift-tab", ("comment-reply", 2usize), Region::Content, cx);
    walk.step_into("shift-tab", &last, LAST, cx);
    // Its footer, Save reply locally and Cancel, then its editor.
    for button in ["composer-post", "composer-cancel"] {
        walk.keys("shift-tab", cx);
        assert!(walk.focus_inside(button, cx), "Shift-Tab reached {button}");
        walk.region(Region::Content, cx);
    }
    walk.step_into("shift-tab", &reply, "the reply's editor", cx);

    // Write a comment…, opened by Enter: Tab indents there, F6 leaves it.
    walk.keys("f6", cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("shift-tab", "new-comment-start", cx);
    walk.enter(cx);
    let editor = walk.new_editor(cx);
    assert!(walk.editor_focused(&editor, cx));
    walk.type_text("- one\n", cx);
    walk.type_text("two", cx);
    let listed = editor.read_with(cx, |editor, _| editor.text().to_owned());
    walk.keys("tab", cx);
    let indented = editor.read_with(cx, |editor, _| editor.text().to_owned());
    assert!(indented != listed, "Tab indented: {indented:?}");
    assert!(walk.editor_focused(&editor, cx), "Tab stayed in the editor");
    walk.keys("f6", cx);
    walk.region(Region::Tickets, cx);
    assert!(
        !walk.shell.present("new-comment-start", cx),
        "open: it holds text"
    );

    // Editing: Share stays in the header, and the chips are followed by
    // the edit toolbar's stop, then the page's editor.
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "item-edit", cx);
    walk.enter(cx);
    walk.shell.until_editing(cx).await;
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);
    walk.tab_to("tab", "item-share", cx);
    for id in [
        "item-lifecycle-action",
        "chip-type",
        "chip-status",
        "chip-project",
        "chip-team",
        "chip-all",
    ] {
        walk.step("tab", id, Region::Content, cx);
    }
    walk.step("tab", "toolbar-rich", Region::Content, cx);
    walk.step("tab", "item-page", Region::Content, cx);
    let host = walk.shell.until_editing(cx).await;
    let editor = host.read_with(cx, |host, _| host.editor().clone());
    assert!(walk.editor_focused(&editor, cx), "the page's editor");
}
