//! Link resolution (decision 10) over real repositories, through the item
//! tab whose editor the link is clicked in: an exact path in the linking
//! item's own work context, then on primary; the nearest document of the
//! same file name, said in the linking tab's banner; a title, closest
//! first; and "not found" in a banner. A comment's link resolves from its
//! item's folder. Refused targets never reach the repository.
//!
//! The handler tests feed the editor's own link events to the read
//! editor, a comment's editor and an `EditorHost` built here, unsafe
//! targets first, and check the browser after each: only `http`, `https`
//! and `mailto` reach it.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::{AppContext, Entity, TestAppContext, component::Root};
use manyhands::repository::{ItemContextKind, ItemDtoKind};
use zorite_editor::{EditorEvent, EditorState};

use crate::desktop::{
    model::{links::LinkTarget, navigation::OpenTarget},
    test_support::fixtures::{Fixture, item_id},
    view::{
        editor::host::EditorHost,
        harness::{Shell, key},
        item::{ItemPane, LINK_NOT_FOUND, LINK_NOT_OPENED, NEAREST_MATCH},
    },
};

const LINKING: &str = "01ARZ3NDEKTSV4RRFFQ69G5KA1";
const PRIMARY_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5KB1";
const CONTEXT_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5KC1";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5KD1";
const NEAR: &str = "01ARZ3NDEKTSV4RRFFQ69G5KE1";
const FAR: &str = "01ARZ3NDEKTSV4RRFFQ69G5KF1";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5KG1";

/// The schemes no editor opens, tried before the ones it does.
const UNSAFE: [&str; 6] = [
    "javascript:alert(1)",
    "file:///etc/passwd",
    "data:text/html,<b>x</b>",
    "vbscript:msgbox",
    "ftp://example.invalid/a",
    "<file:///etc/passwd> \"Title\"",
];

/// A managed document's source.
fn document_source(id: &str, title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n---\nBody of {title}.\n"
    )
}

/// Commits the document `id` at `path` on the branch checked out at
/// `root`: the primary branch, or an item worktree's own.
fn commit_document(fixture: &Fixture, root: &Path, id: &str, path: &str, title: &str) {
    fixture.commit_file(root, path, document_source(id, title).as_bytes());
}

/// The worktree of the item `id`'s own context in `root`.
fn context_of(root: &Path, id: &str) -> PathBuf {
    root.join(".manyhands/worktrees").join(id)
}

/// One repository, `a`, built by `build` and refreshed, in an open window,
/// with the document `LINKING` shown in its tab.
fn linking(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(&shell, document(&root, LINKING), cx);
    (shell, root)
}

fn document(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(id),
    }
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

/// The shown tab's read editor, after a frame.
fn read_editor(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorState> {
    shell.frame(cx, |_, _| ());
    let pane = pane(shell, cx);
    pane.read_with(cx, |pane, cx| pane.read_editor(cx))
        .expect("the read view is shown")
}

/// The read editor of the shown tab's comment `id`, after a frame.
fn comment_editor(shell: &Shell, id: &str, cx: &mut TestAppContext) -> Entity<EditorState> {
    shell.frame(cx, |_, _| ());
    let pane = pane(shell, cx);
    let view = pane
        .read_with(cx, |pane, _| pane.comment_view(id))
        .expect("the comment has a read editor");
    view.read_with(cx, |view, _| view.editor().clone())
}

/// The editor emits `event`, as a click on a link makes it, and every
/// read it starts finishes.
fn click(shell: &Shell, editor: &Entity<EditorState>, event: EditorEvent, cx: &mut TestAppContext) {
    editor.update(cx, |_, cx| cx.emit(event));
    shell.run(cx);
}

fn link(raw: &str) -> EditorEvent {
    EditorEvent::OpenLink(raw.to_owned().into())
}

fn wiki(raw: &str) -> EditorEvent {
    EditorEvent::OpenWikiLink(raw.to_owned().into())
}

fn shown(shell: &Shell, cx: &mut TestAppContext) -> Option<OpenTarget> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
}

/// What the shown tab's link banner says, when it shows one.
fn banner(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("item-link", cx)
}

/// Shows the linking tab again, which is already open.
fn back_to_linking(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    open(shell, document(root, LINKING), cx);
    assert_eq!(shown(shell, cx), Some(document(root, LINKING)));
}

/// The linking document lives in its own work context, as a new document
/// does; `docs/b.md` there is another document than on primary. The
/// context's copy opens.
#[gpui_kit::test]
fn an_exact_path_in_the_items_context_wins_over_primary(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        fixture.create_document(root, LINKING, "docs/a.md", "Linking");
        commit_document(fixture, root, PRIMARY_B, "docs/b.md", "On primary");
        let context = context_of(root, LINKING);
        commit_document(fixture, &context, CONTEXT_B, "docs/b.md", "In the context");
    });
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let from = service.show_item(&repo, &item_id(LINKING)).unwrap();
    assert_eq!(from.context.kind, ItemContextKind::Active);
    let on_primary = service
        .show_path(&repo, None, Path::new("docs/b.md"))
        .unwrap();
    assert_eq!(on_primary.id.as_deref(), Some(PRIMARY_B));

    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link("b.md"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, CONTEXT_B)));

    back_to_linking(&shell, &root, cx);
    assert_eq!(banner(&shell, cx), None, "an exact match says nothing");
}

/// The linking document's context does not hold the path: primary does,
/// and its document opens, by a relative link or one from the root.
#[gpui_kit::test]
fn an_exact_path_on_primary_opens_its_tab(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        fixture.create_document(root, LINKING, "docs/a.md", "Linking");
        commit_document(fixture, root, OTHER, "docs/sub/c.md", "Only on primary");
    });
    assert!(!context_of(&root, LINKING).join("docs/sub/c.md").exists());

    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link("sub/c.md"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, OTHER)));
    back_to_linking(&shell, &root, cx);
    assert_eq!(banner(&shell, cx), None);

    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link("</docs/sub/c.md#part> \"Title\""), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, OTHER)));
}

/// Nothing is at `docs/guides/gone/target.md`. Two documents named
/// `target.md` are one folder step away, and one is two: the nearer ones
/// tie, and path order picks one. The linking tab says which path opened,
/// a path the repository holds, never the link's own text. The next link
/// clears it.
#[gpui_kit::test]
fn a_missing_path_falls_back_to_the_nearest_same_name_and_says_so(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/guides/a.md", "Linking");
        commit_document(fixture, root, OTHER, "docs/target.md", "Up one");
        commit_document(fixture, root, NEAR, "docs/guides/x/target.md", "Down one");
        commit_document(fixture, root, FAR, "docs/other/target.md", "Two away");
    });
    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link("gone/target.md"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, NEAR)));
    assert_eq!(
        banner(&shell, cx),
        None,
        "the opened tab has no link banner"
    );

    back_to_linking(&shell, &root, cx);
    let said = banner(&shell, cx).expect("the linking tab says so");
    assert_eq!(said, format!("{NEAREST_MATCH} docs/guides/x/target.md"));
    assert!(!said.contains("gone"), "the link's own text is not shown");

    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link("x/target.md"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, NEAR)));
    back_to_linking(&shell, &root, cx);
    assert_eq!(banner(&shell, cx), None, "the next link cleared it");
}

/// Three documents share a title, in any case. Path order would pick the
/// farthest; the closest opens, its alias and heading ignored, and says
/// nothing. A title no document has is not found.
#[gpui_kit::test]
fn a_title_link_resolves_closest_first(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/guides/a.md", "Linking");
        commit_document(fixture, root, FAR, "docs/far/away/s1.md", "Shared Title");
        commit_document(fixture, root, NEAR, "docs/guides/s2.md", "shared title");
        commit_document(fixture, root, OTHER, "docs/s3.md", "SHARED TITLE");
    });
    let editor = read_editor(&shell, cx);
    click(
        &shell,
        &editor,
        wiki("Shared Title#A heading|the alias"),
        cx,
    );
    assert_eq!(shown(&shell, cx), Some(document(&root, NEAR)));
    back_to_linking(&shell, &root, cx);
    assert_eq!(banner(&shell, cx), None);

    let editor = read_editor(&shell, cx);
    click(&shell, &editor, wiki("No Document Has This Title"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, LINKING)));
    assert_eq!(banner(&shell, cx).as_deref(), Some(LINK_NOT_FOUND));
}

/// `observe_path` refuses a path no item can be at, and no document has
/// its file name: not found, and no tab opens.
#[gpui_kit::test]
fn a_link_to_a_non_item_path_is_not_found(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/a.md", "Linking");
        fixture.commit_file(root, "src/lib.rs", b"pub fn secret() {}\n");
        fixture.commit_file(root, "README.md", b"# Read me\n");
    });
    for raw in ["../src/lib.rs", "/README.md", "../README.md"] {
        let editor = read_editor(&shell, cx);
        click(&shell, &editor, link(raw), cx);
        assert_eq!(shown(&shell, cx), Some(document(&root, LINKING)), "{raw}");
        assert_eq!(banner(&shell, cx).as_deref(), Some(LINK_NOT_FOUND), "{raw}");
        let tabs = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys().len());
        assert_eq!(tabs, 1, "{raw} opened no tab");
    }
}

/// Not found is said in the linking tab's banner, which names no target,
/// and nowhere else: no request, so nothing in the operation area. A
/// scheme no editor opens says so in the same banner.
#[gpui_kit::test]
fn an_unresolved_link_says_not_found_in_a_banner(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/a.md", "Linking");
    });
    assert_eq!(banner(&shell, cx), None);
    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link("nothing/here-SECRET.md"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, LINKING)));
    assert_eq!(banner(&shell, cx).as_deref(), Some(LINK_NOT_FOUND));
    assert!(
        shell.requests(cx, |_| true).is_empty(),
        "no request was made"
    );

    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link("javascript:alert('SECRET')"), cx);
    assert_eq!(banner(&shell, cx).as_deref(), Some(LINK_NOT_OPENED));
    assert_eq!(cx.opened_url(), None);
    assert!(shell.requests(cx, |_| true).is_empty());
    assert_eq!(shown(&shell, cx), Some(document(&root, LINKING)));
}

/// The comment's file is under `.manyhands/comments/`, but its link is
/// read from its item's folder, `docs/guides`: `b.md` there is an exact
/// match. Read from the comment's folder, it would be the nearest
/// `docs/b.md`, said in the banner.
#[gpui_kit::test]
fn a_comments_link_resolves_from_its_items_folder(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/guides/a.md", "Linking");
        commit_document(fixture, root, NEAR, "docs/guides/b.md", "Beside it");
        commit_document(fixture, root, FAR, "docs/b.md", "Up one");
        fixture.commit_comment(
            root,
            LINKING,
            COMMENT,
            None,
            "2026-10-20T10:14:00Z",
            "created_by: Ada Lovelace <ada@example.invalid>\n",
        );
    });
    let editor = comment_editor(&shell, COMMENT, cx);
    click(&shell, &editor, link("b.md"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, NEAR)));
    back_to_linking(&shell, &root, cx);
    assert_eq!(banner(&shell, cx), None, "an exact match, not the nearest");

    let editor = comment_editor(&shell, COMMENT, cx);
    click(&shell, &editor, wiki("Up one"), cx);
    assert_eq!(shown(&shell, cx), Some(document(&root, FAR)));
}

/// Containment: a backslash, a drive letter, a leading `//` and a `..`
/// past the root are not found, and no read is made for them, not even
/// the nearest name's, though `docs/x.md` would answer it.
#[gpui_kit::test]
fn refused_targets_never_reach_the_repository(cx: &mut TestAppContext) {
    let (shell, root) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/a.md", "Linking");
        commit_document(fixture, root, OTHER, "docs/x.md", "Same name");
    });
    for raw in [
        "sub\\x.md",
        "sub%5Cx.md",
        "C:/x.md",
        "c:x.md",
        "//host/share/x.md",
        "../../x.md",
        "/../x.md",
    ] {
        let editor = read_editor(&shell, cx);
        let before = shell.backend.reads_in_order().len();
        click(&shell, &editor, link(raw), cx);
        assert_eq!(
            shell.backend.reads_in_order().len(),
            before,
            "{raw} made a read"
        );
        assert!(shell.requests(cx, |_| true).is_empty(), "{raw}");
        assert_eq!(shown(&shell, cx), Some(document(&root, LINKING)), "{raw}");
        assert_eq!(banner(&shell, cx).as_deref(), Some(LINK_NOT_FOUND), "{raw}");
    }
}

/// Feeds every unsafe target, then a path and a title, to `editor`, and
/// checks the browser opened none; then the browser links, each opened.
fn only_browser_links_open(shell: &Shell, editor: &Entity<EditorState>, cx: &mut TestAppContext) {
    for raw in UNSAFE {
        click(shell, editor, link(raw), cx);
        assert_eq!(cx.opened_url(), None, "{raw}");
    }
    for event in [
        link("missing.md"),
        link("#a-heading"),
        wiki("No Such Title"),
        wiki("#^block"),
        wiki("refs:^block"),
    ] {
        click(shell, editor, event, cx);
        assert_eq!(cx.opened_url(), None);
    }
    for (raw, opened) in [
        ("https://example.com/a#part", "https://example.com/a#part"),
        ("<http://example.com/b> \"Title\"", "http://example.com/b"),
        (
            "mailto:ada@example.invalid?cc=x@example.invalid&subject=Hi&body=Text",
            "mailto:ada@example.invalid?subject=Hi&body=Text",
        ),
    ] {
        click(shell, editor, link(raw), cx);
        assert_eq!(cx.opened_url().as_deref(), Some(opened), "{raw}");
    }
}

#[gpui_kit::test]
fn the_read_editor_opens_only_browser_links(cx: &mut TestAppContext) {
    let (shell, _) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/a.md", "Linking");
    });
    let editor = read_editor(&shell, cx);
    click(&shell, &editor, link(UNSAFE[0]), cx);
    assert_eq!(banner(&shell, cx).as_deref(), Some(LINK_NOT_OPENED));
    only_browser_links_open(&shell, &editor, cx);
}

#[gpui_kit::test]
fn a_comment_editor_opens_only_browser_links(cx: &mut TestAppContext) {
    let (shell, _) = linking(cx, |fixture, root| {
        commit_document(fixture, root, LINKING, "docs/a.md", "Linking");
        fixture.commit_comment(root, LINKING, COMMENT, None, "2026-10-20T10:14:00Z", "");
    });
    let editor = comment_editor(&shell, COMMENT, cx);
    click(&shell, &editor, link(UNSAFE[0]), cx);
    assert_eq!(banner(&shell, cx).as_deref(), Some(LINK_NOT_OPENED));
    only_browser_links_open(&shell, &editor, cx);
}

/// An `EditorHost` in a window of its own, as Task 14 will place one: it
/// opens browser links itself, and passes every other target it does not
/// ignore to its owner.
#[gpui_kit::test]
fn an_edit_host_opens_only_browser_links(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let made = Rc::new(RefCell::new(None));
    shell
        .window
        .update(cx, {
            let made = made.clone();
            move |_, window, cx| {
                window.replace_root(cx, |window, cx| {
                    let host = cx.new(|cx| EditorHost::new("A body.\n", None, window, cx));
                    *made.borrow_mut() = Some(host.clone());
                    Root::new(host, window, cx)
                });
            }
        })
        .unwrap();
    let host: Entity<EditorHost> = made.borrow_mut().take().unwrap();
    shell.run(cx);
    let passed = Rc::new(RefCell::new(Vec::new()));
    cx.update({
        let passed = passed.clone();
        |cx| {
            cx.subscribe(&host, move |_, target: &LinkTarget, _| {
                passed.borrow_mut().push(target.clone())
            })
            .detach()
        }
    });
    let editor = host.read_with(cx, |host, _| host.editor().clone());
    only_browser_links_open(&shell, &editor, cx);
    let passed = passed.borrow();
    let unsafe_passed = passed
        .iter()
        .filter(|target| **target == LinkTarget::Unsafe)
        .count();
    assert_eq!(
        unsafe_passed,
        UNSAFE.len(),
        "each unsafe link goes to the owner"
    );
    assert!(passed.contains(&LinkTarget::Path {
        path: "missing.md".to_owned(),
        from_root: false,
    }));
    assert!(passed.contains(&LinkTarget::Title("No Such Title".to_owned())));
    assert_eq!(
        passed.len(),
        UNSAFE.len() + 2,
        "ignored and browser links stay with the host"
    );
}
