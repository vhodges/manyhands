//! The document tree: folders, empty ones included, before documents;
//! nested folders; nonconforming documents in their folders. And the
//! model's stamps: an old read never replaces a newer view.

use manyhands::{
    repository::{
        DocumentFolderDto, DocumentFolderListDto, IndexState, IndexStateDto, ItemContextDto,
        ItemContextKind, ItemDto, ItemDtoKind, ItemListDto, ProblemDto,
    },
    results::{ProblemCode, ResultCode},
};

use super::{
    DocKind, DocRow, Documents, Expansion, Trailing, TreeNode, build_tree, doc_row,
    visible_selection,
};
use crate::desktop::{
    model::list_state::ListState,
    services::{ReadFailure, ReadResult},
};

fn index() -> IndexStateDto {
    IndexStateDto {
        state: IndexState::Current,
        refreshed_at: None,
    }
}

fn folders(paths: &[(&str, bool)]) -> DocumentFolderListDto {
    DocumentFolderListDto {
        items: paths
            .iter()
            .map(|(path, empty)| DocumentFolderDto {
                path: (*path).to_owned(),
                empty: *empty,
            })
            .collect(),
        complete: true,
        index: index(),
    }
}

/// A document as a list gives it; `id` `None` makes it nonconforming.
fn document(path: &str, id: Option<&str>) -> ItemDto {
    ItemDto {
        id: id.map(str::to_owned),
        kind: ItemDtoKind::Document,
        path: path.to_owned(),
        title: id.map(|_| format!("Title of {path}")),
        ticket_type: None,
        status: None,
        project: None,
        team: None,
        closure: None,
        slug: None,
        parent: None,
        deps: Vec::new(),
        readiness: None,
        unknown_metadata: serde_json::Map::new(),
        body: None,
        source: None,
        observation: None,
        context: ItemContextDto {
            kind: ItemContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: "/repo".to_owned(),
            head_oid: None,
        },
        changed_at: None,
        change_source: None,
        problems: match id {
            Some(_) => Vec::new(),
            None => vec![ProblemDto {
                code: ProblemCode::MissingField,
                path: Some(path.to_owned()),
                target_id: None,
            }],
        },
        index: index(),
    }
}

fn documents(items: Vec<ItemDto>) -> ItemListDto {
    ItemListDto {
        items,
        complete: true,
        index: index(),
    }
}

const ID_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const ID_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const ID_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";

/// A node in short: a folder by path, emptiness and children, or a
/// document by path.
#[derive(Debug, PartialEq)]
enum Shape {
    Folder(String, bool, Vec<Shape>),
    Document(String),
}

fn shape(nodes: &[TreeNode]) -> Vec<Shape> {
    nodes
        .iter()
        .map(|node| match node {
            TreeNode::Folder {
                path,
                empty,
                children,
            } => Shape::Folder(path.clone(), *empty, shape(children)),
            TreeNode::Document(item) => Shape::Document(item.path.clone()),
        })
        .collect()
}

fn folder(path: &str, empty: bool, children: Vec<Shape>) -> Shape {
    Shape::Folder(path.to_owned(), empty, children)
}

fn doc(path: &str) -> Shape {
    Shape::Document(path.to_owned())
}

#[test]
fn an_empty_folder_is_shown() {
    let tree = build_tree(&folders(&[("docs/empty", true)]), &documents(Vec::new()));
    assert_eq!(shape(&tree), [folder("docs/empty", true, Vec::new())]);
}

#[test]
fn folders_nest_by_path() {
    let tree = build_tree(
        &folders(&[
            ("docs/outer", false),
            ("docs/outer/inner", false),
            ("docs/outer/inner/leaf", true),
        ]),
        &documents(vec![document("docs/outer/inner/a.md", Some(ID_A))]),
    );
    assert_eq!(
        shape(&tree),
        [folder(
            "docs/outer",
            false,
            vec![folder(
                "docs/outer/inner",
                false,
                vec![
                    folder("docs/outer/inner/leaf", true, Vec::new()),
                    doc("docs/outer/inner/a.md"),
                ],
            )],
        )]
    );
}

#[test]
fn a_nonconforming_document_is_placed_in_its_folder() {
    let marker = document("docs/full/marker.md", None);
    let tree = build_tree(
        &folders(&[("docs/full", false)]),
        &documents(vec![document("docs/full/a.md", Some(ID_A)), marker.clone()]),
    );
    assert_eq!(
        shape(&tree),
        [folder(
            "docs/full",
            false,
            vec![doc("docs/full/a.md"), doc("docs/full/marker.md")],
        )]
    );
    let TreeNode::Folder { children, .. } = &tree[0] else {
        panic!("a folder");
    };
    // The row is the read's own entry, problem and all.
    assert!(matches!(&children[1], TreeNode::Document(item) if *item == marker));
}

#[test]
fn folders_come_before_documents_and_the_reads_order_is_kept() {
    let tree = build_tree(
        &folders(&[("docs/b", false), ("docs/z", true)]),
        &documents(vec![
            document("docs/a.md", Some(ID_A)),
            document("docs/b/x.md", Some(ID_B)),
            document("docs/c.md", Some(ID_C)),
        ]),
    );
    assert_eq!(
        shape(&tree),
        [
            folder("docs/b", false, vec![doc("docs/b/x.md")]),
            folder("docs/z", true, Vec::new()),
            doc("docs/a.md"),
            doc("docs/c.md"),
        ]
    );
}

/// No folder is made up: a document whose folder the folder read did not
/// give sits under the nearest one it did, or at the top.
#[test]
fn a_document_in_an_unlisted_folder_is_not_given_a_made_up_folder() {
    let tree = build_tree(
        &folders(&[("docs/outer", false)]),
        &documents(vec![
            document("docs/outer/gone/a.md", Some(ID_A)),
            document("docs/elsewhere/b.md", Some(ID_B)),
        ]),
    );
    assert_eq!(
        shape(&tree),
        [
            folder("docs/outer", false, vec![doc("docs/outer/gone/a.md")]),
            doc("docs/elsewhere/b.md"),
        ]
    );
}

fn ok<T>(value: T) -> ReadResult<T> {
    ReadResult::Ok(value)
}

#[test]
fn the_tree_is_ready_once_both_reads_are() {
    let mut model = Documents::default();
    let (stamp, _) = model.load("/a".to_owned());
    assert!(matches!(model.state(), ListState::Loading));
    assert!(model.apply_folders(stamp.clone(), ok(folders(&[("docs/x", true)]))));
    assert!(matches!(model.state(), ListState::Loading));
    assert!(model.apply_documents(
        stamp,
        ok(documents(vec![document("docs/a.md", Some(ID_A))]))
    ));
    let ListState::Ready {
        rows,
        complete,
        index: Some(_),
    } = model.state()
    else {
        panic!("ready with the documents' index");
    };
    assert!(*complete);
    assert_eq!(
        shape(rows),
        [folder("docs/x", true, Vec::new()), doc("docs/a.md")]
    );
}

#[test]
fn either_read_failing_fails_the_tree() {
    let mut model = Documents::default();
    let (stamp, _) = model.load("/a".to_owned());
    model.apply_folders(stamp.clone(), ok(folders(&[])));
    model.apply_documents(
        stamp,
        ReadResult::Failed(ReadFailure {
            code: ResultCode::IndexUnavailable,
            recovery: Vec::new(),
        }),
    );
    assert!(
        matches!(model.state(), ListState::Failed(failure) if failure.code == ResultCode::IndexUnavailable)
    );
}

/// Review Focus 1: a read for A that completes after B was loaded, or
/// after the selection was cleared, is dropped.
#[test]
fn an_old_repositorys_reads_are_dropped() {
    let mut model = Documents::default();
    let (a, _) = model.load("/a".to_owned());
    let (b, _) = model.load("/b".to_owned());
    assert!(!model.apply_documents(a.clone(), ok(documents(Vec::new()))));
    assert!(!model.apply_folders(a, ok(folders(&[]))));
    assert!(matches!(model.state(), ListState::Loading));
    assert_eq!(model.repo().map(String::as_str), Some("/b"));

    model.clear();
    assert!(!model.apply_documents(b, ok(documents(Vec::new()))));
    assert_eq!(model.repo(), None);
}

/// Loads `repo` again and lands both reads, as a reload does.
fn reload(
    model: &mut Documents,
    repo: &str,
    folder_list: DocumentFolderListDto,
    document_list: ItemListDto,
) {
    let (stamp, _) = model.load(repo.to_owned());
    assert!(model.apply_folders(stamp.clone(), ok(folder_list)));
    assert!(model.apply_documents(stamp, ok(document_list)));
}

/// The folders with child nodes that the repository's expansion calls
/// expanded, in tree order.
fn open_folders(model: &Documents) -> Vec<String> {
    fn walk(nodes: &[TreeNode], expansion: &Expansion, out: &mut Vec<String>) {
        for node in nodes {
            if let TreeNode::Folder { path, children, .. } = node {
                if !children.is_empty() && expansion.is_expanded(path) {
                    out.push(path.clone());
                }
                walk(children, expansion, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(
        model.state().rows().expect("a ready tree"),
        model.expansion().expect("a loaded repository"),
        &mut out,
    );
    out
}

fn paths(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

/// Review Focus 5, P19 and P20: what the person collapsed is kept across
/// rebuilds, a folder that goes and comes back included; a new folder
/// starts expanded; Collapse all collapses every folder; a repository's
/// expansion is its own; the selection rule never expands.
#[test]
fn expansion_survives_rebuilds_and_collapse_all() {
    let base_folders = [
        ("guides", false),
        ("guides/deep", false),
        ("plans", false),
        ("empty", true),
    ];
    let base_documents = || {
        vec![
            document("guides/deep/a.md", Some(ID_A)),
            document("plans/p.md", Some(ID_B)),
        ]
    };
    let mut model = Documents::default();

    // Nothing collapsed: every folder with child nodes is expanded.
    reload(
        &mut model,
        "/a",
        folders(&base_folders),
        documents(base_documents()),
    );
    assert_eq!(
        open_folders(&model),
        paths(&["guides", "guides/deep", "plans"])
    );

    // Collapse `guides`; a rebuild with a new folder keeps that and starts
    // the new folder expanded.
    model.set_expanded("guides", false);
    let mut with_notes = base_folders.to_vec();
    with_notes.push(("notes", false));
    let mut notes_documents = base_documents();
    notes_documents.push(document("notes/n.md", Some(ID_C)));
    reload(
        &mut model,
        "/a",
        folders(&with_notes),
        documents(notes_documents.clone()),
    );
    let expansion = model.expansion().expect("loaded");
    assert!(!expansion.is_expanded("guides"));
    assert!(expansion.is_expanded("notes"));
    assert_eq!(
        open_folders(&model),
        paths(&["guides/deep", "plans", "notes"])
    );

    // A folder that disappears and comes back keeps the person's choice.
    model.set_expanded("plans", false);
    let without_plans = [("guides", false), ("guides/deep", false), ("notes", false)];
    reload(
        &mut model,
        "/a",
        folders(&without_plans),
        documents(vec![
            document("guides/deep/a.md", Some(ID_A)),
            document("notes/n.md", Some(ID_C)),
        ]),
    );
    assert_eq!(open_folders(&model), paths(&["guides/deep", "notes"]));
    reload(
        &mut model,
        "/a",
        folders(&with_notes),
        documents(notes_documents),
    );
    assert!(!model.expansion().expect("loaded").is_expanded("plans"));
    assert_eq!(open_folders(&model), paths(&["guides/deep", "notes"]));

    // Setting a folder expanded takes it out of the collapsed ones.
    model.set_expanded("plans", true);
    assert!(model.expansion().expect("loaded").is_expanded("plans"));

    // Another repository's expansion is its own, and comes back as left.
    reload(
        &mut model,
        "/b",
        folders(&base_folders),
        documents(base_documents()),
    );
    assert_eq!(
        open_folders(&model),
        paths(&["guides", "guides/deep", "plans"])
    );
    model.set_expanded("plans", false);
    reload(
        &mut model,
        "/a",
        folders(&with_notes),
        documents(vec![
            document("guides/deep/a.md", Some(ID_A)),
            document("plans/p.md", Some(ID_B)),
            document("notes/n.md", Some(ID_C)),
        ]),
    );
    assert!(!model.expansion().expect("loaded").is_expanded("guides"));
    assert!(model.expansion().expect("loaded").is_expanded("plans"));

    // Collapse all: every folder, the ones that cannot toggle included.
    model.collapse_all();
    let expansion = model.expansion().expect("loaded");
    for path in ["guides", "guides/deep", "plans", "empty", "notes"] {
        assert!(!expansion.is_expanded(path), "{path} is collapsed");
    }
    assert!(open_folders(&model).is_empty());

    // A folder made or moved into is revealed in its own repository: its
    // ancestors expand, and it stays as it was.
    model.reveal(&"/a".to_owned(), "plans");
    assert!(
        !model.expansion().expect("loaded").is_expanded("plans"),
        "only what is above the path opens"
    );
    model.reveal(&"/a".to_owned(), "guides/deep/new");
    let expansion = model.expansion().expect("loaded");
    assert!(expansion.is_expanded("guides"));
    assert!(expansion.is_expanded("guides/deep"));
    assert!(!expansion.is_expanded("plans"));
    model.reveal(&"/b".to_owned(), "plans/x.md");
    reload(
        &mut model,
        "/b",
        folders(&base_folders),
        documents(base_documents()),
    );
    // `/b` had `plans` collapsed; its reveal opened it, and `/a`'s stays.
    assert_eq!(
        open_folders(&model),
        paths(&["guides", "guides/deep", "plans"])
    );
    reload(
        &mut model,
        "/a",
        folders(&base_folders),
        documents(base_documents()),
    );
    assert!(!model.expansion().expect("loaded").is_expanded("plans"));

    the_selection_goes_to_the_nearest_visible_row();
}

/// The selected row after a rebuild (R-27): itself while visible, else the
/// outermost collapsed folder above it; the expansion is only read.
fn the_selection_goes_to_the_nearest_visible_row() {
    let tree = build_tree(
        &folders(&[("guides", false), ("guides/deep", false), ("plans", false)]),
        &documents(vec![
            document("guides/deep/a.md", Some(ID_A)),
            document("plans/p.md", Some(ID_B)),
            document("top.md", Some(ID_C)),
        ]),
    );
    let deep_document = "document:/repo:guides/deep/a.md";
    let mut expansion = Expansion::default();

    assert_eq!(
        visible_selection(&tree, &expansion, deep_document).as_deref(),
        Some(deep_document)
    );
    assert_eq!(
        visible_selection(&tree, &expansion, "folder:guides/deep").as_deref(),
        Some("folder:guides/deep")
    );

    // Only the inner folder collapsed: it is the nearest visible row.
    expansion.set("guides/deep", false);
    assert_eq!(
        visible_selection(&tree, &expansion, deep_document).as_deref(),
        Some("folder:guides/deep")
    );
    assert_eq!(
        visible_selection(&tree, &expansion, "folder:guides/deep").as_deref(),
        Some("folder:guides/deep")
    );

    // Both collapsed: the outer one, whichever row was selected inside it.
    expansion.set("guides", false);
    assert_eq!(
        visible_selection(&tree, &expansion, deep_document).as_deref(),
        Some("folder:guides")
    );
    assert_eq!(
        visible_selection(&tree, &expansion, "folder:guides/deep").as_deref(),
        Some("folder:guides")
    );
    assert_eq!(
        visible_selection(&tree, &expansion, "folder:guides").as_deref(),
        Some("folder:guides")
    );

    // Rows elsewhere are not affected; a row the tree does not have gives
    // nothing.
    assert_eq!(
        visible_selection(&tree, &expansion, "document:/repo:plans/p.md").as_deref(),
        Some("document:/repo:plans/p.md")
    );
    assert_eq!(
        visible_selection(&tree, &expansion, "document:/repo:top.md").as_deref(),
        Some("document:/repo:top.md")
    );
    assert_eq!(visible_selection(&tree, &expansion, "folder:gone"), None);
}

/// A document under `folder`, nonconforming for `codes`.
fn broken(path: &str, codes: &[ProblemCode]) -> TreeNode {
    let mut item = document(path, None);
    item.problems = codes
        .iter()
        .map(|code| ProblemDto {
            code: *code,
            path: Some(path.to_owned()),
            target_id: None,
        })
        .collect();
    TreeNode::Document(item)
}

fn folder_node(path: &str, empty: bool, children: Vec<TreeNode>) -> TreeNode {
    TreeNode::Folder {
        path: path.to_owned(),
        empty,
        children,
    }
}

fn document_node(path: &str) -> TreeNode {
    TreeNode::Document(document(path, Some(ID_A)))
}

fn row(
    label: &str,
    kind: DocKind,
    expanded: Option<bool>,
    trailing: Trailing,
    tooltip: &[&str],
) -> DocRow {
    DocRow {
        label: label.to_owned(),
        kind,
        expanded,
        trailing,
        tooltip: tooltip.iter().map(|line| (*line).to_owned()).collect(),
    }
}

/// Part E's row facts for the seven kinds before Part I (P22, P23).
#[test]
fn rows_read_as_the_design_writes_them() {
    const ADOPT: &str = "Not a managed document yet. Open to adopt it.";
    const FIX_FIELD: &str = "Needs fixing: Add the required front matter field. Open it to fix.";
    let three_documents = folder_node(
        "docs/guides",
        false,
        vec![
            folder_node(
                "docs/guides/deep",
                false,
                vec![
                    document_node("docs/guides/deep/a.md"),
                    broken("docs/guides/deep/b.md", &[ProblemCode::MissingField]),
                ],
            ),
            document_node("docs/guides/c.md"),
        ],
    );
    let table = [
        (
            "an expanded folder with child nodes",
            folder_node("docs/plans", false, vec![document_node("docs/plans/p.md")]),
            true,
            row(
                "plans",
                DocKind::Folder,
                Some(true),
                Trailing::None,
                &["docs/plans"],
            ),
        ),
        (
            "a collapsed folder counts its documents at every depth, nonconforming included",
            three_documents.clone(),
            false,
            row(
                "guides",
                DocKind::Folder,
                Some(false),
                Trailing::Count(3),
                &["docs/guides"],
            ),
        ),
        (
            "the same folder expanded shows no count",
            three_documents,
            true,
            row(
                "guides",
                DocKind::Folder,
                Some(true),
                Trailing::None,
                &["docs/guides"],
            ),
        ),
        (
            "a folder holding a subfolder of no documents is still a folder with children",
            folder_node(
                "docs/outer",
                false,
                vec![folder_node("docs/outer/inner", true, Vec::new())],
            ),
            false,
            row(
                "outer",
                DocKind::Folder,
                Some(false),
                Trailing::Count(0),
                &["docs/outer"],
            ),
        ),
        (
            "a leaf folder holding only a file that is not a document",
            folder_node("docs/misc", false, Vec::new()),
            true,
            row(
                "misc",
                DocKind::Folder,
                None,
                Trailing::Count(0),
                &["docs/misc"],
            ),
        ),
        (
            "an empty folder",
            folder_node("docs/empty", true, Vec::new()),
            true,
            row(
                "empty",
                DocKind::Folder,
                None,
                Trailing::Empty,
                &["Empty folder. Shared once it contains a document."],
            ),
        ),
        (
            "a document is its title, with its path",
            document_node("docs/plans/p.md"),
            false,
            row(
                "Title of docs/plans/p.md",
                DocKind::Document,
                None,
                Trailing::None,
                &["docs/plans/p.md"],
            ),
        ),
        (
            "a document with no title is its file name",
            {
                let mut item = document("docs/plans/untitled.md", Some(ID_A));
                item.title = None;
                TreeNode::Document(item)
            },
            false,
            row(
                "untitled.md",
                DocKind::Document,
                None,
                Trailing::None,
                &["docs/plans/untitled.md"],
            ),
        ),
        (
            "a nonconforming file with no front matter can be adopted",
            broken("docs/guides/notes.md", &[ProblemCode::MissingFrontMatter]),
            false,
            row(
                "notes.md",
                DocKind::Nonconforming { adoptable: true },
                None,
                Trailing::None,
                &[ADOPT, "docs/guides/notes.md"],
            ),
        ),
        (
            "a nonconforming file with a missing field is fixed, not adopted",
            broken("docs/guides/field.md", &[ProblemCode::MissingField]),
            false,
            row(
                "field.md",
                DocKind::Nonconforming { adoptable: false },
                None,
                Trailing::None,
                &[FIX_FIELD, "docs/guides/field.md"],
            ),
        ),
        (
            "no front matter and another problem is not adoptable",
            broken(
                "docs/guides/two.md",
                &[ProblemCode::MissingFrontMatter, ProblemCode::MissingField],
            ),
            false,
            row(
                "two.md",
                DocKind::Nonconforming { adoptable: false },
                None,
                Trailing::None,
                &[
                    "Needs fixing: Add YAML front matter to the file. Open it to fix.",
                    "docs/guides/two.md",
                ],
            ),
        ),
        (
            "no front matter outside docs is not adoptable",
            broken("notes/loose.md", &[ProblemCode::MissingFrontMatter]),
            false,
            row(
                "loose.md",
                DocKind::Nonconforming { adoptable: false },
                None,
                Trailing::None,
                &[
                    "Needs fixing: Add YAML front matter to the file. Open it to fix.",
                    "notes/loose.md",
                ],
            ),
        ),
    ];
    for (name, node, expanded, want) in table {
        assert_eq!(doc_row(&node, expanded), want, "{name}");
    }
}

/// R-17 and R-35 for the tree: a reload of the repository last loaded keeps
/// its rows until the replacement lands, a busy replacement keeps them, a
/// failed one drops them; a clear keeps nothing but remembers the
/// repository, and another repository keeps nothing.
#[test]
fn the_tree_is_kept_over_a_reload_of_the_same_repository() {
    let kept_rows = |model: &Documents| model.list().rows().map(|rows| shape(rows));
    let one_folder = || {
        shape(&build_tree(
            &folders(&[("docs/x", true)]),
            &documents(Vec::new()),
        ))
    };
    let mut model = Documents::default();

    let (stamp, changed) = model.load("/a".to_owned());
    assert!(changed, "the first load is a change");
    assert_eq!(kept_rows(&model), None);
    model.apply_folders(stamp.clone(), ok(folders(&[("docs/x", true)])));
    model.apply_documents(stamp, ok(documents(Vec::new())));
    assert_eq!(kept_rows(&model), Some(one_folder()));

    // A reload: loading over the kept rows; one read landing changes
    // nothing; a busy one keeps them and says so.
    let (stamp, changed) = model.load("/a".to_owned());
    assert!(!changed);
    assert!(matches!(model.state(), ListState::Loading));
    assert!(model.list().reloading());
    assert_eq!(kept_rows(&model), Some(one_folder()));
    model.apply_folders(stamp.clone(), ok(folders(&[])));
    assert!(model.list().reloading(), "the other read is still to land");
    assert_eq!(kept_rows(&model), Some(one_folder()));
    model.apply_documents(stamp, ReadResult::Busy);
    assert!(model.list().busy_over_kept());
    assert_eq!(kept_rows(&model), Some(one_folder()));

    // A failed reload replaces the rows.
    let (stamp, _) = model.load("/a".to_owned());
    model.apply_folders(stamp.clone(), ok(folders(&[])));
    model.apply_documents(
        stamp,
        ReadResult::Failed(ReadFailure {
            code: ResultCode::IndexUnavailable,
            recovery: Vec::new(),
        }),
    );
    assert!(matches!(model.state(), ListState::Failed(_)));
    assert_eq!(kept_rows(&model), None);

    // A clear keeps no rows, and a load of the same repository after it is
    // not a change; another repository is.
    let (stamp, _) = model.load("/a".to_owned());
    model.apply_folders(stamp.clone(), ok(folders(&[("docs/x", true)])));
    model.apply_documents(stamp, ok(documents(Vec::new())));
    model.clear();
    assert_eq!(kept_rows(&model), None);
    let (_, changed) = model.load("/a".to_owned());
    assert!(!changed, "a clear does not forget the last repository");
    assert_eq!(kept_rows(&model), None);
    let (stamp, _) = model.load("/a".to_owned());
    model.apply_folders(stamp.clone(), ok(folders(&[("docs/x", true)])));
    model.apply_documents(stamp, ok(documents(Vec::new())));
    let (_, changed) = model.load("/b".to_owned());
    assert!(changed);
    assert_eq!(kept_rows(&model), None, "another repository keeps no rows");
}
