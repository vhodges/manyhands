//! The document tree: what the two library reads give, and nothing else;
//! arrows move and Enter opens; folder creation is visibly unavailable.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::repository::ItemDtoKind;

use super::{NONCONFORMING, node_label};
use crate::desktop::{
    model::{
        documents::{TreeNode, build_tree},
        navigation::OpenTarget,
    },
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::harness::{Shell, key},
};

const TOP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const NESTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const MARKER: &str = "docs/guides/deep/marker.md";

/// A document at the top, one in a nested folder, an empty folder and a
/// nonconforming file in the nested folder, then a library refresh. The
/// folder read is of the primary worktree, where the committed file
/// makes the nested folders. The repository is
/// selected; Documents is the first pane.
fn documents(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, TOP, "docs/top.md", "Top");
    fixture.create_document(&root, NESTED, "docs/guides/deep/nested.md", "Nested");
    fixture.commit_nonconforming_document(&root, MARKER);
    fixture.create_folder(&root, "docs/empty");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.click(("repo-row", 0usize), cx);
    (shell, root)
}

/// The tree's nodes in the order they are drawn with every folder open.
fn flatten(nodes: &[TreeNode], into: &mut Vec<String>) {
    for node in nodes {
        into.push(node_label(node));
        if let TreeNode::Folder { children, .. } = node {
            flatten(children, into);
        }
    }
}

fn library_tree(shell: &Shell, root: &Path) -> Vec<TreeNode> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    build_tree(
        &service.list_document_folders(&repo).unwrap(),
        &service.list_documents(&repo).unwrap(),
    )
}

/// No fabrication: the drawn nodes are the tree of the same two library
/// reads, empty folder and nonconforming file among them.
#[gpui_kit::test]
fn the_tree_shows_what_the_library_reads_give(cx: &mut TestAppContext) {
    let (shell, root) = documents(cx);
    let tree = library_tree(&shell, &root);
    let mut wanted = Vec::new();
    flatten(&tree, &mut wanted);
    assert_eq!(shell.rows("doc-node", cx), wanted);
    assert_eq!(
        wanted,
        [
            "empty",
            "guides",
            "deep",
            "docs/guides/deep/marker.md",
            "Nested",
            "Top",
        ]
    );

    // A document shows its path as secondary text; the empty folder and
    // the nonconforming file say so in text.
    let at = |label: &str| wanted.iter().position(|node| node == label).unwrap();
    assert_eq!(
        shell.label(("doc-path", at("Top")), cx).as_deref(),
        Some("docs/top.md")
    );
    assert_eq!(
        shell.label(("doc-empty", at("empty")), cx).as_deref(),
        Some(super::EMPTY_FOLDER)
    );
    let marker = at(MARKER);
    assert_eq!(
        shell.label(("doc-nonconforming", marker), cx).as_deref(),
        Some(NONCONFORMING)
    );
    assert_eq!(
        shell.label(("doc-problem", marker), cx).as_deref(),
        Some("missing_field")
    );
}

#[gpui_kit::test]
fn folder_creation_is_visibly_unavailable(cx: &mut TestAppContext) {
    let (shell, _) = documents(cx);
    assert_eq!(
        shell.label("new-folder-unavailable", cx).as_deref(),
        Some(super::NEW_FOLDER_UNAVAILABLE)
    );
    // The control is shown and does nothing: the tree is as it was.
    let before = shell.rows("doc-node", cx);
    shell.click("new-folder", cx);
    assert_eq!(shell.rows("doc-node", cx), before);
}

/// Arrows move through the drawn nodes; Enter opens a document by ID and
/// a nonconforming file by path.
#[gpui_kit::test]
fn arrows_move_in_the_tree_and_enter_opens(cx: &mut TestAppContext) {
    let (shell, root) = documents(cx);
    let nodes = shell.rows("doc-node", cx);
    let at = |label: &str| nodes.iter().position(|node| node == label).unwrap();
    shell.keys("secondary-2", cx);

    // The Kit's tree moves from the first node when none is selected.
    shell.keys("down", cx);
    assert!(shell.selected(("doc-node", 1usize), cx));
    shell.keys("down", cx);
    assert!(shell.selected(("doc-node", 2usize), cx));
    assert!(!shell.selected(("doc-node", 1usize), cx));
    shell.keys("up", cx);
    assert!(shell.selected(("doc-node", 1usize), cx));

    // Enter on a folder opens nothing.
    shell.keys("enter", cx);
    let opened = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.opened().cloned())
    };
    assert_eq!(opened(&shell, cx), None);

    for _ in 1..at(MARKER) {
        shell.keys("down", cx);
    }
    assert!(shell.selected(("doc-node", at(MARKER)), cx));
    shell.keys("enter", cx);
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let listed = service.list_documents(&repo).unwrap();
    let marker = listed
        .items
        .iter()
        .find(|item| item.path == MARKER)
        .unwrap();
    assert_eq!(
        opened(&shell, cx),
        Some(OpenTarget::Inspect {
            repo: key(&root),
            worktree: marker.context.worktree.clone(),
            path: MARKER.to_owned(),
        })
    );
    assert_eq!(shell.label("open-path", cx).as_deref(), Some(MARKER));

    for _ in at(MARKER)..at("Top") {
        shell.keys("down", cx);
    }
    assert!(shell.selected(("doc-node", at("Top")), cx));
    shell.keys("enter", cx);
    assert_eq!(
        opened(&shell, cx),
        Some(OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: TOP.parse().unwrap(),
        })
    );
    assert_eq!(shell.label("open-kind", cx).as_deref(), Some("Document"));
    assert_eq!(shell.label("open-id", cx).as_deref(), Some(TOP));
}

/// Review Focus 1: A's document read is held while B is selected; B's
/// tree shows, and releasing A's read leaves it.
#[gpui_kit::test]
fn a_held_tree_of_a_never_replaces_bs(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.commit_nonconforming_document(&a, "docs/only-in-a.md");
    fixture.commit_nonconforming_document(&b, "docs/only-in-b.md");
    fixture.refresh(&a);
    fixture.refresh(&b);
    let shell = Shell::open(fixture, cx);
    // Queued after the startup reads, so it holds A's document read.
    let held = shell.backend.hold_read(ReadKind::Documents);
    shell.click(("repo-row", 0usize), cx);
    assert_eq!(shell.rows("doc-node", cx), Vec::<String>::new());
    shell.click(("repo-row", 1usize), cx);
    assert_eq!(shell.rows("doc-node", cx), ["docs/only-in-b.md"]);

    // A's read may already be dropped with its task; either way it is
    // let go.
    let _ = held.send(());
    shell.run(cx);
    assert_eq!(shell.rows("doc-node", cx), ["docs/only-in-b.md"]);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.documents().repo().cloned());
    assert_eq!(shown, Some(key(&b)));
}
