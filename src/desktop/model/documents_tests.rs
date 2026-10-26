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

use super::{Documents, TreeNode, build_tree};
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
    let stamp = model.load("/a".to_owned());
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
    let stamp = model.load("/a".to_owned());
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
    let a = model.load("/a".to_owned());
    let b = model.load("/b".to_owned());
    assert!(!model.apply_documents(a.clone(), ok(documents(Vec::new()))));
    assert!(!model.apply_folders(a, ok(folders(&[]))));
    assert!(matches!(model.state(), ListState::Loading));
    assert_eq!(model.repo().map(String::as_str), Some("/b"));

    model.clear();
    assert!(!model.apply_documents(b, ok(documents(Vec::new()))));
    assert_eq!(model.repo(), None);
}
