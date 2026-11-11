//! The document tree: what the two library reads give, and nothing else;
//! arrows move and Enter opens a document or toggles a folder; folders keep
//! the state the person left them in; each list state says so and acts;
//! New folder opens its dialog (E-10).

use std::path::{Path, PathBuf};

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::repository::ItemDtoKind;

use super::EMPTY_TRAILING;
use crate::desktop::{
    model::{
        documents::{TreeNode, build_tree, doc_row},
        layout::PanelKind,
        navigation::OpenTarget,
    },
    services::ReadKind,
    test_support::fixtures::Fixture,
    view::harness::{Shell, key},
};

const TOP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const NESTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const MARKER: &str = "docs/guides/deep/marker.md";
/// What a nonconforming file's row is labeled by: its file name.
const MARKER_NAME: &str = "marker.md";

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
        into.push(doc_row(node, true).label);
        if let TreeNode::Folder { children, .. } = node {
            flatten(children, into);
        }
    }
}

/// The node of the document at `path`, wherever it is in `node`.
fn find_document(node: &TreeNode, path: &str) -> Option<TreeNode> {
    match node {
        TreeNode::Document(item) if item.path == path => Some(node.clone()),
        TreeNode::Folder { children, .. } => {
            children.iter().find_map(|child| find_document(child, path))
        }
        TreeNode::Document(_) => None,
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
        ["empty", "guides", "deep", MARKER_NAME, "Nested", "Top"]
    );

    // The second line is gone (E-8): a document's path and a file's problem
    // are in the row's tooltip, which `doc_row`'s table pins. What stays in
    // the row is text: "empty" where an empty folder's count would be, and
    // the nonconforming file's reason, named by its tooltip's first line.
    let at = |label: &str| wanted.iter().position(|node| node == label).unwrap();
    assert_eq!(
        shell.label(("doc-empty", at("empty")), cx).as_deref(),
        Some(EMPTY_TRAILING)
    );
    let marker = tree
        .iter()
        .find_map(|node| find_document(node, MARKER))
        .expect("the file is in the tree");
    assert_eq!(
        shell
            .label(("doc-nonconforming", at(MARKER_NAME)), cx)
            .as_deref(),
        Some(doc_row(&marker, true).tooltip[0].as_str())
    );
    assert!(!shell.present(("doc-count", at("guides")), cx), "open");

    // What the person collapsed stays collapsed when the tree is rebuilt
    // by a reload (R-12): the folder's contents are not drawn, and the
    // selected row inside it moves to the folder, which stays closed
    // (R-27).
    shell.keys("secondary-2", cx);
    for _ in 0..4 {
        shell.keys("down", cx);
    }
    assert!(shell.selected(("doc-node", at("Nested")), cx));
    shell.workspace.update(cx, |workspace, cx| {
        workspace.documents_mut().set_expanded("docs/guides", false);
        workspace.reload_lists(cx);
    });
    shell.run(cx);
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    assert!(shell.selected(("doc-node", 1usize), cx), "guides");
}

/// E-10: New folder opens its dialog, under `docs` with no folder
/// selected; by keyboard, with a folder selected, under that folder.
/// Escape closes it, focus goes back to New folder, and nothing is made.
#[gpui_kit::test]
fn new_folder_opens_its_dialog(cx: &mut TestAppContext) {
    let (shell, root) = documents(cx);
    let before = shell.rows("doc-node", cx);
    let folders = library_tree(&shell, &root);
    shell.click("new-folder-button", cx);
    assert!(shell.present("new-folder-dialog", cx), "its dialog opens");
    assert_eq!(
        shell.label("new-folder-explain", cx).as_deref(),
        Some("A new folder stays on this computer until a document is saved in it.")
    );
    assert_eq!(
        shell.label("new-folder-name", cx).as_deref(),
        Some("Folder name")
    );
    assert_eq!(shell.label("new-folder-path", cx).as_deref(), Some("docs/"));
    shell.keys("escape", cx);
    assert!(!shell.present("new-folder-dialog", cx));

    shell.keys("secondary-2", cx);
    shell.keys("down", cx);
    assert!(shell.selected(("doc-node", 1usize), cx));
    assert_eq!(before[1], "guides");
    shell.keys("shift-tab", cx);
    let focused = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| {
            window
                .try_find("new-folder-button")
                .is_some_and(|element| element.focused() == Some(true))
        })
    };
    assert!(focused(&shell, cx), "Shift-Tab reaches New folder");
    shell.press("enter", cx);
    assert!(shell.present("new-folder-dialog", cx), "Enter opens it");
    assert_eq!(
        shell.label("new-folder-path", cx).as_deref(),
        Some("docs/guides/")
    );
    shell.keys("escape", cx);
    assert!(!shell.present("new-folder-dialog", cx));
    assert!(focused(&shell, cx), "focus goes back to New folder");
    assert_eq!(shell.rows("doc-node", cx), before);
    assert!(library_tree(&shell, &root) == folders, "nothing is made");
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

    // Enter on a folder opens nothing; it toggles the folder (R-24), twice
    // here, so the rows are the ones `nodes` has.
    shell.keys("enter", cx);
    let opened = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.opened(cx))
    };
    assert_eq!(opened(&shell, cx), None);
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    shell.keys("enter", cx);
    assert_eq!(opened(&shell, cx), None);
    assert_eq!(shell.rows("doc-node", cx), nodes);

    for _ in 1..at(MARKER_NAME) {
        shell.keys("down", cx);
    }
    assert!(shell.selected(("doc-node", at(MARKER_NAME)), cx));
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
    assert_eq!(shell.label("item-path", cx).as_deref(), Some(MARKER));

    for _ in at(MARKER_NAME)..at("Top") {
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
    // E-30: the kind and ID are in the details sheet.
    shell.click("chip-all", cx);
    assert_eq!(shell.label("item-kind", cx).as_deref(), Some("Document"));
    assert_eq!(shell.label("item-id", cx).as_deref(), Some(TOP));
    shell.keys("escape", cx);

    // A click on a row opens it too: the nonconforming file by its path,
    // then the document by its ID.
    shell.click(("doc-node", at(MARKER_NAME)), cx);
    assert_eq!(
        opened(&shell, cx),
        Some(OpenTarget::Inspect {
            repo: key(&root),
            worktree: marker.context.worktree.clone(),
            path: MARKER.to_owned(),
        })
    );
    shell.click(("doc-node", at("Top")), cx);
    assert_eq!(
        opened(&shell, cx),
        Some(OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: TOP.parse().unwrap(),
        })
    );
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
    assert_eq!(shell.rows("doc-node", cx), ["only-in-b.md"]);

    // A's read may already be dropped with its task; either way it is
    // let go.
    let _ = held.send(());
    shell.run(cx);
    assert_eq!(shell.rows("doc-node", cx), ["only-in-b.md"]);
    let shown = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.documents().repo().cloned());
    assert_eq!(shown, Some(key(&b)));
}

/// What a step must not change: the reads, the requests and the open tabs
/// (P25).
#[derive(Debug, PartialEq)]
struct Quiet {
    reads: Vec<ReadKind>,
    requests: Vec<crate::desktop::services::requests::DesktopRequestId>,
    tabs: Vec<String>,
}

fn quiet(shell: &Shell, cx: &mut TestAppContext) -> Quiet {
    Quiet {
        reads: shell.backend.reads_in_order(),
        requests: shell.requests(cx, |_| true),
        tabs: shell.rows("tab-title", cx),
    }
}

/// The row `ix`'s own account of whether its folder is open.
fn expanded(shell: &Shell, ix: usize, cx: &mut TestAppContext) -> Option<bool> {
    shell.frame(cx, |window, _| {
        window
            .try_find(("doc-node", ix))
            .and_then(|row| row.expanded())
    })
}

/// What the model keeps of `path`, the folder the person may have closed.
fn kept_open(shell: &Shell, path: &str, cx: &mut TestAppContext) -> bool {
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .documents()
            .expansion()
            .is_some_and(|expansion| expansion.is_expanded(path))
    })
}

fn opened(shell: &Shell, cx: &mut TestAppContext) -> Option<OpenTarget> {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx))
}

/// P18 to P21, P25, P26 and the Documents half of P15. Left collapses a
/// folder and shows its document count, Right expands it, Enter toggles it
/// (R-24) and a click does; the model keeps what each did, so a finished
/// refresh job's reload (P19), and hiding and showing the panel (P26),
/// leave a collapsed folder collapsed with the selection where it was.
/// Home and End select the first and the last row (P18), and Enter on a
/// document opens its tab. None of it but the refresh reads or sends
/// anything (P25).
#[gpui_kit::test]
async fn folders_keep_their_state_and_toggle_by_key(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = documents(cx);
    shell.keys("secondary-2", cx);
    let before = quiet(&shell, cx);

    // Down to `guides`, open, with no count.
    shell.keys("down", cx);
    assert!(shell.selected(("doc-node", 1usize), cx));
    assert_eq!(expanded(&shell, 1, cx), Some(true));
    assert!(!shell.present(("doc-count", 1usize), cx));

    // Left collapses it: its two documents are counted in its row, which
    // says it is closed, and the model keeps that (a collapse by key is
    // written back by the Kit's event).
    shell.keys("left", cx);
    assert_eq!(expanded(&shell, 1, cx), Some(false));
    assert_eq!(shell.label(("doc-count", 1usize), cx).as_deref(), Some("2"));
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    assert!(!kept_open(&shell, "docs/guides", cx));
    assert!(shell.selected(("doc-node", 1usize), cx));

    // Right expands it, with no count.
    shell.keys("right", cx);
    assert_eq!(expanded(&shell, 1, cx), Some(true));
    assert!(!shell.present(("doc-count", 1usize), cx));
    assert_eq!(shell.rows("doc-node", cx).len(), 6);
    assert!(kept_open(&shell, "docs/guides", cx));

    // Enter collapses it (R-24), through the model.
    shell.keys("enter", cx);
    assert_eq!(expanded(&shell, 1, cx), Some(false));
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    assert!(!kept_open(&shell, "docs/guides", cx));
    assert!(shell.selected(("doc-node", 1usize), cx));
    // An empty folder has nothing to toggle: Enter writes nothing to the
    // model, which would otherwise record it closed.
    shell.keys("up", cx);
    assert!(shell.selected(("doc-node", 0usize), cx));
    shell.keys("enter", cx);
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    assert!(kept_open(&shell, "docs/empty", cx));
    shell.keys("down", cx);
    assert!(shell.selected(("doc-node", 1usize), cx));

    // A click on it expands it.
    shell.click(("doc-node", 1usize), cx);
    assert_eq!(expanded(&shell, 1, cx), Some(true));
    assert_eq!(shell.rows("doc-node", cx).len(), 6);
    assert!(kept_open(&shell, "docs/guides", cx));
    assert_eq!(quiet(&shell, cx), before);

    // Collapsed again, it stays collapsed over a finished refresh job's
    // reload (P19), and keeps the selection.
    shell.keys("secondary-2", cx);
    shell.keys("left", cx);
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    assert_eq!(quiet(&shell, cx), before);
    let reads = shell.backend.reads_seen(ReadKind::Documents);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(
            request.intent,
            crate::desktop::services::requests::Intent::IndexRefresh { .. }
        )
    })[0];
    shell.until_ended(refresh, cx).await;
    shell
        .until_drawn(cx, "the reload after the refresh", |shell, _| {
            shell.backend.reads_seen(ReadKind::Documents) > reads
        })
        .await;
    shell.run(cx);
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    assert_eq!(expanded(&shell, 1, cx), Some(false));
    assert!(shell.selected(("doc-node", 1usize), cx));
    // From here on nothing is read or sent, refresh included (P25).
    let settled = quiet(&shell, cx);

    // Hiding Documents from the footer and showing it again changes nothing
    // (P26).
    shell.click("footer-panel-documents", cx);
    assert!(!shell.present("region-documents", cx), "hidden");
    shell.click("footer-panel-documents", cx);
    assert!(shell.present("region-documents", cx), "shown");
    assert_eq!(shell.rows("doc-node", cx), ["empty", "guides", "Top"]);
    assert_eq!(expanded(&shell, 1, cx), Some(false));
    assert!(shell.selected(("doc-node", 1usize), cx));
    assert!(!kept_open(&shell, "docs/guides", cx));

    // Home and End select the first and the last row (P18).
    shell.keys("secondary-2", cx);
    shell.keys("end", cx);
    assert!(shell.selected(("doc-node", 2usize), cx), "End: the last");
    assert!(!shell.selected(("doc-node", 1usize), cx));
    shell.keys("home", cx);
    assert!(shell.selected(("doc-node", 0usize), cx), "Home: the first");
    assert!(!shell.selected(("doc-node", 2usize), cx));
    shell.keys("end", cx);
    assert_eq!(quiet(&shell, cx), settled);

    // Enter on the document there opens its tab.
    assert_eq!(opened(&shell, cx), None);
    shell.keys("enter", cx);
    assert_eq!(
        opened(&shell, cx),
        Some(OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: TOP.parse().unwrap(),
        })
    );
}

/// The id of `name`'s child `part`, as the alert row and the empty state
/// give their action and detail.
fn child(name: &'static str, part: &'static str) -> ElementId {
    (ElementId::from(name), part).into()
}

/// Draws frames until `done`, moving the executor's clock on at each look:
/// a read that met a repository busy with the worker's own job waits on the
/// timer to try again, and no real time is waited for it.
async fn until_settled(
    shell: &Shell,
    cx: &mut TestAppContext,
    what: &str,
    done: impl Fn(&Shell, &mut TestAppContext) -> bool,
) {
    shell
        .until_drawn(cx, what, |shell, cx| {
            cx.executor()
                .advance_clock(std::time::Duration::from_secs(1));
            done(shell, cx)
        })
        .await;
}

fn count(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    crate::desktop::view::dock::tests::support::panel_count(shell, PanelKind::Documents, cx)
}

/// Whether the tree has the keys: Down moves the selection by one, which it
/// does only when the tree holds focus.
fn tree_has_the_keys(shell: &Shell, cx: &mut TestAppContext) -> bool {
    let before = (0..8usize).find(|ix| shell.selected(("doc-node", *ix), cx));
    shell.keys("down", cx);
    let after = (0..8usize).find(|ix| shell.selected(("doc-node", *ix), cx));
    before != after
}

/// P15 and P16 for Documents: the first load draws a skeleton over the
/// tree; a reload keeps the rows and says "Updating…"; a busy one keeps
/// them under an alert whose Retry reads once without clearing them; a
/// repository with nothing offers New document…; a stale list keeps its
/// rows and a never-refreshed one has none, each offering Refresh; a failed
/// read says so with its message, and an index that cannot be read offers
/// to rebuild it.
#[gpui_kit::test]
async fn the_tree_says_each_list_state_and_acts_on_it(cx: &mut TestAppContext) {
    use manyhands::repository::{LeaseKind, RepositoryService};

    use crate::desktop::{
        model::list_view::{
            ListKind, busy_over_kept_text, empty_text, failed_text, index_unavailable_text,
            loading_text, never_refreshed_text, stale_text,
        },
        services::requests::Intent,
    };

    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_document(&a, TOP, "docs/alpha.md", "Alpha");
    fixture.create_document(&a, NESTED, "docs/beta.md", "Beta");
    fixture.refresh(&a);
    let b = fixture.create_repository("b");
    fixture.refresh(&b);
    let folders_only = fixture.create_repository("f");
    fixture.create_folder(&folders_only, "docs/only");
    fixture.refresh(&folders_only);
    // Stale, and never refreshed: states the library makes only from its
    // registry, which a test reaches beside the service's own connection.
    let stale = fixture.create_repository("s");
    fixture.create_document(&stale, TOP, "docs/kept.md", "Kept");
    fixture.refresh(&stale);
    let never = fixture.create_repository("n");
    fixture.create_document(&never, TOP, "docs/gone.md", "Gone");
    fixture.refresh(&never);
    {
        let registry = rusqlite::Connection::open(
            fixture
                .data_dir()
                .join(manyhands::repository::REGISTRY_FILE),
        )
        .unwrap();
        let stale_rows = registry
            .execute(
                "UPDATE repositories SET refresh_required = 1 WHERE root_path LIKE '%/s'",
                [],
            )
            .unwrap();
        let never_rows = registry
            .execute(
                "UPDATE repositories SET refreshed_at = NULL WHERE root_path LIKE '%/n'",
                [],
            )
            .unwrap();
        registry
            .execute(
                "DELETE FROM contexts WHERE repository_id IN \
                 (SELECT id FROM repositories WHERE root_path LIKE '%/n')",
                [],
            )
            .unwrap();
        assert_eq!((stale_rows, never_rows), (1, 1));
    }
    let shell = Shell::open(fixture, cx);
    let rows = |shell: &Shell, cx: &mut TestAppContext| shell.rows("doc-node", cx);

    // The first load: a skeleton over a tree that is drawn with no rows,
    // while the document read is held.
    let held = shell.backend.hold_read(ReadKind::Documents);
    shell.select(&a, cx);
    assert!(shell.present("documents-skeleton", cx));
    assert_eq!(
        shell.label("documents-skeleton", cx).as_deref(),
        Some(loading_text(ListKind::Documents))
    );
    assert!(shell.present("documents-tree", cx), "the tree stays drawn");
    assert!(rows(&shell, cx).is_empty());
    assert_eq!(count(&shell, cx), None, "no count on a first load");
    let _ = held.send(());
    until_settled(&shell, cx, "the tree", |shell, cx| {
        !shell.rows("doc-node", cx).is_empty()
    })
    .await;
    assert!(!shell.present("documents-skeleton", cx));
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);
    assert_eq!(count(&shell, cx).as_deref(), Some("2"));
    assert!(!shell.present("documents-alert", cx));

    // A reload that stays busy keeps the rows under the busy alert.
    shell.use_list(crate::desktop::model::navigation::Pane::Documents, cx);
    let folders = shell.backend.hold_read(ReadKind::Folders);
    let documents = shell.backend.hold_read(ReadKind::Documents);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    shell.run(cx);
    assert_eq!(
        count(&shell, cx).as_deref(),
        Some("Updating\u{2026}"),
        "a reload over kept rows"
    );
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);
    assert!(!shell.present("documents-skeleton", cx));
    let lease = RepositoryService::hold_lease_for_testing(
        &a,
        shell.fixture.data_dir(),
        LeaseKind::CacheWrite,
    )
    .unwrap();
    let _ = folders.send(());
    let _ = documents.send(());
    until_settled(&shell, cx, "the busy alert", |shell, cx| {
        shell.present("documents-alert", cx)
    })
    .await;
    assert_eq!(
        shell.label("documents-alert", cx),
        Some(busy_over_kept_text("a"))
    );
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"], "the rows are kept");

    // Retry reads both lists once, without clearing: while its reads are
    // held the rows stay, there is no skeleton, the count says Updating…,
    // and the tree has the keys.
    drop(lease);
    let reads = (
        shell.backend.reads_seen(ReadKind::Folders),
        shell.backend.reads_seen(ReadKind::Documents),
    );
    let folders = shell.backend.hold_read(ReadKind::Folders);
    let documents = shell.backend.hold_read(ReadKind::Documents);
    shell.click(child("documents-alert", "action"), cx);
    assert_eq!(
        (
            shell.backend.reads_seen(ReadKind::Folders),
            shell.backend.reads_seen(ReadKind::Documents)
        ),
        (reads.0 + 1, reads.1 + 1)
    );
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"], "kept while held");
    assert!(!shell.present("documents-skeleton", cx));
    assert_eq!(count(&shell, cx).as_deref(), Some("Updating\u{2026}"));
    assert!(tree_has_the_keys(&shell, cx), "Retry focuses the tree");
    let _ = folders.send(());
    let _ = documents.send(());
    until_settled(&shell, cx, "Retry landed", |shell, cx| {
        !shell.present("documents-alert", cx)
    })
    .await;
    assert_eq!(rows(&shell, cx), ["Alpha", "Beta"]);
    assert_eq!(count(&shell, cx).as_deref(), Some("2"));

    // An empty repository: its empty state offers New document…, which
    // opens the dialog.
    shell.select(&b, cx);
    until_settled(&shell, cx, "the empty state", |shell, cx| {
        shell.present("documents-empty", cx)
    })
    .await;
    assert_eq!(
        shell.label("documents-empty", cx),
        Some(empty_text(
            ListKind::Documents,
            manyhands::repository::ClosureFilter::Open,
            "b"
        ))
    );
    assert!(shell.present("documents-tree", cx), "the tree stays drawn");
    shell.click(child("documents-empty", "action"), cx);
    // The document's dialog, which has a folder field.
    assert!(shell.present("create-dialog", cx));
    assert!(shell.present("create-folder", cx));
    shell.keys("escape", cx);
    assert!(!shell.present("create-dialog", cx));

    // A tree of folders only has rows, so it is not the empty state (P27).
    shell.select(&folders_only, cx);
    until_settled(&shell, cx, "the folder's row", |shell, cx| {
        !shell.rows("doc-node", cx).is_empty()
    })
    .await;
    assert_eq!(rows(&shell, cx), ["only"]);
    assert!(!shell.present("documents-empty", cx));

    // A stale list keeps its rows under an alert, and a list never
    // refreshed has none; each offers Refresh, which submits a refresh and
    // gives the tree the keys.
    for (name, repo, alert, kept) in [
        ("s", &stale, stale_text("s"), true),
        ("n", &never, never_refreshed_text("n"), false),
    ] {
        shell.select(repo, cx);
        until_settled(&shell, cx, "the alert", |shell, cx| {
            shell.present("documents-alert", cx)
        })
        .await;
        assert_eq!(shell.label("documents-alert", cx), Some(alert), "{name}");
        assert_eq!(rows(&shell, cx).len(), usize::from(kept), "{name}");
        let refreshes = |shell: &Shell, cx: &mut TestAppContext| {
            shell.requests(cx, |request| {
                matches!(request.intent, Intent::IndexRefresh { .. })
            })
        };
        let before = refreshes(&shell, cx).len();
        shell.click(child("documents-alert", "action"), cx);
        let after = refreshes(&shell, cx);
        assert_eq!(
            after.len(),
            before + 1,
            "{name}: Refresh submitted a refresh"
        );
        assert!(
            !kept || tree_has_the_keys(&shell, cx),
            "{name}: focus is on the tree"
        );
        shell.until_ended(after[0], cx).await;
    }

    // Back to `a`, whose lists the next two states replace.
    shell.select(&a, cx);
    until_settled(&shell, cx, "a's rows", |shell, cx| {
        !shell.rows("doc-node", cx).is_empty()
    })
    .await;

    // A failed read says so, with its message on a visible line, and no row.
    let elsewhere = shell.fixture.empty_directory("other-data");
    shell
        .backend
        .replace_service(RepositoryService::open_at(&elsewhere).unwrap());
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the failure", |shell, cx| {
        shell.present("documents-alert", cx)
    })
    .await;
    assert_eq!(
        shell.label("documents-alert", cx),
        Some(failed_text(ListKind::Documents, "a"))
    );
    let message =
        shell
            .workspace
            .read_with(cx, |workspace, _| match workspace.documents().state() {
                crate::desktop::model::list_state::ListState::Failed(failure) => {
                    failure.code.message().to_string()
                }
                other => panic!("the tree did not fail: {other:?}"),
            });
    assert!(!message.is_empty());
    assert_eq!(
        shell.label(child("documents-alert", "detail"), cx),
        Some(message),
        "the failure's own message is on the visible line"
    );
    assert!(rows(&shell, cx).is_empty());
    assert!(shell.present("documents-tree", cx), "the tree stays drawn");

    // An index that cannot be read offers to rebuild it. Its job is held at
    // the library, so no rebuild runs on the unreadable index.
    let degraded = shell.fixture.empty_directory("degraded-data");
    std::fs::write(degraded.join("manyhands.sqlite3"), b"not sqlite").unwrap();
    shell
        .backend
        .replace_service(RepositoryService::open_at(&degraded).unwrap());
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.reload_lists(cx));
    until_settled(&shell, cx, "the unavailable index", |shell, cx| {
        shell.label("documents-alert", cx) == Some(index_unavailable_text("a"))
    })
    .await;
    let gate = shell
        .backend
        .hold_job(crate::desktop::services::JobKind::IndexRebuild);
    shell.click(child("documents-alert", "action"), cx);
    let rebuild = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRebuild { .. })
    });
    assert_eq!(
        rebuild.len(),
        1,
        "Rebuild the search index submitted a rebuild"
    );
    gate.release();
    shell.until_ended(rebuild[0], cx).await;
}

/// P14 for Documents: a document with unsaved work is marked in its row at
/// once, with no read of the list.
#[gpui_kit::test]
async fn a_document_with_unsaved_work_is_marked_at_once(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = documents(cx);
    let at = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .rows("doc-node", cx)
            .iter()
            .position(|label| label == "Top")
            .expect("Top is listed")
    };
    assert!(!shell.present(("doc-unsaved", at(&shell, cx)), cx));

    // Opening the document, editing it and typing a change begins its
    // draft session, which holds unsaved work.
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Document,
        id: TOP.parse().unwrap(),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.click("item-edit", cx);
    let host = shell.until_editing(cx).await;
    shell.run(cx);
    let reads = shell.backend.reads_seen(ReadKind::Documents);
    shell.type_at_end(&host, " more", cx);
    shell
        .until_drawn(cx, "the unsaved marker", |shell, cx| {
            let ix = at(shell, cx);
            shell.present(("doc-unsaved", ix), cx)
        })
        .await;
    let ix = at(&shell, cx);
    assert_eq!(
        shell.label(("doc-unsaved", ix), cx).as_deref(),
        Some(crate::desktop::model::ticket_view::UNSAVED_MARKER)
    );
    // No other row is marked, and the list was not read again.
    for other in (0..shell.rows("doc-node", cx).len()).filter(|other| *other != ix) {
        assert!(!shell.present(("doc-unsaved", other), cx), "{other}");
    }
    assert_eq!(shell.backend.reads_seen(ReadKind::Documents), reads);
}
