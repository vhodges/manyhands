//! The document tree of the selected repository.
//!
//! `list_document_folders` and `list_documents` run in parallel under one
//! stamp; the tree is ready when both are. It holds the folders the
//! folder read gave, empty ones included, and the documents the document
//! read gave, nonconforming ones included, each in its folder. No folder
//! is made up: a document whose folder the folder read did not give sits
//! under the nearest one it did, or at the top.

use std::collections::HashMap;

use manyhands::repository::{DocumentFolderListDto, ItemDto, ItemListDto};

use super::{
    doc_rows::Expansion,
    list_state::{KeptList, ListState},
    stamp::{Generations, RepoKey, Stamp, ViewScope},
    unreadable::UnreadableFile,
};
use crate::desktop::services::ReadResult;

/// The folder every document path starts in. It is not itself listed.
const DOCUMENTS_ROOT: &str = "docs";

// A document row is the read's own entry, unboxed, as the plan gives it;
// a tree holds one per document.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TreeNode {
    /// A folder under `docs/`: its folders first, by path, then its
    /// documents in the read's order.
    Folder {
        path: String,
        empty: bool,
        children: Vec<TreeNode>,
    },
    /// A document or a nonconforming entry, as the read gave it.
    Document(ItemDto),
    /// A file the library could not read (Part I). It selects but does not
    /// open (R-38).
    Unreadable(UnreadableFile),
}

/// How many documents a tree lists, in every folder.
pub(crate) fn document_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            TreeNode::Folder { children, .. } => document_count(children),
            TreeNode::Document(_) => 1,
            TreeNode::Unreadable(_) => 1,
        })
        .sum()
}

/// How many nodes a tree lists, folders and documents in every folder: the
/// rows the panel would draw (`BodyInput::listed`, P27).
pub(crate) fn node_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            TreeNode::Folder { children, .. } => 1 + node_count(children),
            TreeNode::Document(_) => 1,
            TreeNode::Unreadable(_) => 1,
        })
        .sum()
}

/// The folder a path is directly in: everything before its last `/`.
pub(super) fn parent(path: &str) -> Option<&str> {
    path.rsplit_once('/').map(|(parent, _)| parent)
}

/// The tree of `folders` and `documents`: at each level folders before
/// documents, each in its read's order.
pub(crate) fn build_tree(
    folders: &DocumentFolderListDto,
    documents: &ItemListDto,
) -> Vec<TreeNode> {
    let paths: Vec<&str> = folders
        .items
        .iter()
        .map(|folder| folder.path.as_str())
        .collect();
    // The nearest listed folder that holds `path`, by index.
    let holder = |path: &str| {
        let mut at = parent(path);
        while let Some(folder) = at {
            if folder == DOCUMENTS_ROOT {
                return None;
            }
            if let Some(index) = paths.iter().position(|listed| *listed == folder) {
                return Some(index);
            }
            at = parent(folder);
        }
        None
    };
    let folder_parents: Vec<Option<usize>> = paths.iter().map(|path| holder(path)).collect();
    let mut documents_in: Vec<Vec<&ItemDto>> = vec![Vec::new(); paths.len()];
    let mut top_documents = Vec::new();
    for item in &documents.items {
        match holder(&item.path) {
            Some(index) => documents_in[index].push(item),
            None => top_documents.push(item),
        }
    }

    fn level(
        holder: Option<usize>,
        folders: &DocumentFolderListDto,
        folder_parents: &[Option<usize>],
        documents_in: &[Vec<&ItemDto>],
        documents: &[&ItemDto],
    ) -> Vec<TreeNode> {
        let mut nodes: Vec<TreeNode> = folders
            .items
            .iter()
            .enumerate()
            .filter(|(index, _)| folder_parents[*index] == holder)
            .map(|(index, folder)| TreeNode::Folder {
                path: folder.path.clone(),
                empty: folder.empty,
                children: level(
                    Some(index),
                    folders,
                    folder_parents,
                    documents_in,
                    &documents_in[index],
                ),
            })
            .collect();
        nodes.extend(
            documents
                .iter()
                .map(|item| TreeNode::Document((*item).clone())),
        );
        nodes
    }

    level(
        None,
        folders,
        &folder_parents,
        &documents_in,
        &top_documents,
    )
}

/// `file` put in `nodes`: under the deepest listed folder above its path,
/// among that folder's documents in path order, after the same path of
/// another worktree. Nothing is made up: no folder is made for it.
fn insert_unreadable(nodes: &mut Vec<TreeNode>, file: &UnreadableFile) {
    let inside = nodes.iter_mut().find_map(|node| match node {
        TreeNode::Folder { path, children, .. }
            if file
                .path
                .strip_prefix(path.as_str())
                .is_some_and(|rest| rest.starts_with('/')) =>
        {
            Some(children)
        }
        _ => None,
    });
    if let Some(children) = inside {
        return insert_unreadable(children, file);
    }
    let at = nodes
        .iter()
        .position(|node| match node {
            TreeNode::Folder { .. } => false,
            TreeNode::Document(item) => item.path > file.path,
            TreeNode::Unreadable(other) => other.path > file.path,
        })
        .unwrap_or(nodes.len());
    nodes.insert(at, TreeNode::Unreadable(file.clone()));
}

/// `nodes` without the unreadable files, at every depth.
fn remove_unreadable(nodes: &mut Vec<TreeNode>) {
    nodes.retain(|node| !matches!(node, TreeNode::Unreadable(_)));
    for node in nodes {
        if let TreeNode::Folder { children, .. } = node {
            remove_unreadable(children);
        }
    }
}

/// A folder's id in the tree: the Kit's item id and the app's.
pub(crate) fn folder_id(path: &str) -> String {
    format!("{FOLDER_PREFIX}{path}")
}

const FOLDER_PREFIX: &str = "folder:";

/// The path of the folder a tree id names; `None` for a document's.
pub(crate) fn folder_path(id: &str) -> Option<&str> {
    id.strip_prefix(FOLDER_PREFIX)
}

/// A document's id in the tree. Paths are unique within one context's
/// list; a document in an item worktree has its own.
pub(crate) fn document_id(item: &ItemDto) -> String {
    leaf_id(&item.context.worktree, &item.path)
}

/// An unreadable file's id: a row is keyed by worktree and path, as a
/// document is.
pub(crate) fn unreadable_id(file: &UnreadableFile) -> String {
    leaf_id(&file.worktree, &file.path)
}

fn leaf_id(worktree: &str, path: &str) -> String {
    format!("document:{worktree}:{path}")
}

/// The last part of a path.
pub(super) fn last_segment(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, last)| last)
}

/// The selected repository's tree, and the two reads it is made of.
pub(crate) struct Documents {
    repo: Option<RepoKey>,
    /// The repository last loaded, which a clear does not reset: a load
    /// of another one is a change of repository (R-35).
    last_loaded: Option<RepoKey>,
    folders: ListState<DocumentFolderListDto>,
    documents: ListState<ItemListDto>,
    tree: KeptList<Vec<TreeNode>>,
    /// The files the last status read could not read, which sit in `tree`
    /// beside its documents (Part I). Held under the tree's rules:
    /// replaced when a status read lands, kept over a busy or failed one
    /// and during a reload, dropped on a change of repository (R-35).
    unreadable: Vec<UnreadableFile>,
    /// What the person collapsed, per repository, for the session. A
    /// clear or a load of another repository does not forget it (R-12).
    expansions: HashMap<RepoKey, Expansion>,
    generations: Generations,
}

impl Default for Documents {
    fn default() -> Self {
        Self {
            repo: None,
            last_loaded: None,
            folders: ListState::Loading,
            documents: ListState::Loading,
            tree: KeptList::default(),
            unreadable: Vec::new(),
            expansions: HashMap::new(),
            generations: Generations::default(),
        }
    }
}

impl Documents {
    /// The repository the tree is of, while one is loaded.
    pub(crate) fn repo(&self) -> Option<&RepoKey> {
        self.repo.as_ref()
    }

    /// The tree's state, as the last read left it.
    pub(crate) fn state(&self) -> &ListState<Vec<TreeNode>> {
        self.tree.state()
    }

    /// The tree with the rows it kept over a reload.
    pub(crate) fn list(&self) -> &KeptList<Vec<TreeNode>> {
        &self.tree
    }

    /// What the selected repository's person collapsed. `Some` whenever a
    /// repository is loaded.
    pub(crate) fn expansion(&self) -> Option<&Expansion> {
        self.expansions.get(self.repo.as_ref()?)
    }

    /// A folder of the selected repository opened or closed. The tree is
    /// rebuilt from the set by the view.
    pub(crate) fn set_expanded(&mut self, path: &str, expanded: bool) {
        if let Some(repo) = self.repo.clone() {
            self.expansions.entry(repo).or_default().set(path, expanded);
        }
    }

    /// Every folder of the selected repository's tree collapsed.
    #[cfg_attr(not(test), allow(dead_code))] // Used from Task 8.
    pub(crate) fn collapse_all(&mut self) {
        let (Some(repo), Some(rows)) = (self.repo.clone(), self.tree.rows()) else {
            return;
        };
        self.expansions.entry(repo).or_default().collapse_all(rows);
    }

    /// The folders above `path` in `repo`, which a folder made or a
    /// document moved there needs open (R-27). `repo` need not be the one
    /// shown.
    pub(crate) fn reveal(&mut self, repo: &RepoKey, path: &str) {
        self.expansions
            .entry(repo.clone())
            .or_default()
            .expand_ancestors(path);
    }

    /// Starts loading `repo`'s tree: the stamp both reads are made under.
    /// Reads made for an earlier load are dropped when they complete. The
    /// tree read before is kept while this one runs, unless `repo` is not
    /// the last one loaded: then it goes, and the second answer is true
    /// (R-35).
    pub(crate) fn load(&mut self, repo: RepoKey) -> (Stamp, bool) {
        let changed = self.last_loaded.as_ref() != Some(&repo);
        self.forget();
        if changed {
            self.tree.clear();
            self.unreadable.clear();
        } else {
            self.tree.reload();
        }
        self.repo = Some(repo.clone());
        self.last_loaded = Some(repo.clone());
        self.expansions.entry(repo.clone()).or_default();
        (self.generations.next(ViewScope::Documents(repo)), changed)
    }

    /// No repository is loaded: reads still running are dropped, and no
    /// rows are kept. The last-loaded repository is remembered.
    pub(crate) fn clear(&mut self) {
        self.forget();
        self.tree.clear();
        self.unreadable.clear();
        self.repo = None;
    }

    fn forget(&mut self) {
        if let Some(repo) = self.repo.take() {
            self.generations.next(ViewScope::Documents(repo));
        }
        self.folders = ListState::Loading;
        self.documents = ListState::Loading;
    }

    fn wanted(&self, stamp: &Stamp) -> bool {
        self.generations.is_current(stamp)
            && self
                .repo
                .as_ref()
                .is_some_and(|repo| stamp.scope == ViewScope::Documents(repo.clone()))
    }

    /// The files the last status read could not read.
    #[cfg(test)]
    pub(crate) fn unreadable(&self) -> &[UnreadableFile] {
        &self.unreadable
    }

    /// The status read completed. False when it is no longer wanted. One
    /// that gave its files replaces them, in the tree shown or the one
    /// kept over a reload; a busy or failed one keeps them, and never
    /// fails the tree.
    pub(crate) fn apply_unreadable(
        &mut self,
        stamp: Stamp,
        result: ReadResult<Vec<UnreadableFile>>,
    ) -> bool {
        if !self.wanted(&stamp) {
            return false;
        }
        if let ReadResult::Ok(files) = result {
            self.unreadable = files;
            let files = &self.unreadable;
            self.tree.map_rows(|rows| {
                remove_unreadable(rows);
                for file in files {
                    insert_unreadable(rows, file);
                }
            });
        }
        true
    }

    /// The folder read completed. False when it is no longer wanted.
    pub(crate) fn apply_folders(
        &mut self,
        stamp: Stamp,
        result: ReadResult<DocumentFolderListDto>,
    ) -> bool {
        if !self.wanted(&stamp) {
            return false;
        }
        self.folders = ListState::from_read(result, |list| ListState::Ready {
            complete: list.complete,
            index: Some(list.index.clone()),
            rows: list,
        });
        self.combine();
        true
    }

    /// The document read completed. False when it is no longer wanted.
    pub(crate) fn apply_documents(
        &mut self,
        stamp: Stamp,
        result: ReadResult<ItemListDto>,
    ) -> bool {
        if !self.wanted(&stamp) {
            return false;
        }
        self.documents = ListState::from_read(result, |list| ListState::Ready {
            complete: list.complete,
            index: Some(list.index.clone()),
            rows: list,
        });
        self.combine();
        true
    }

    /// The tree from the two reads: failed or busy if either is, ready
    /// when both are, with the document read's index.
    fn combine(&mut self) {
        let state = match (&self.folders, &self.documents) {
            (ListState::Failed(failure), _) | (_, ListState::Failed(failure)) => {
                ListState::Failed(failure.clone())
            }
            (ListState::Busy, _) | (_, ListState::Busy) => ListState::Busy,
            (
                ListState::Ready {
                    rows: folders,
                    complete: folders_complete,
                    ..
                },
                ListState::Ready {
                    rows: documents,
                    complete: documents_complete,
                    index,
                },
            ) => ListState::Ready {
                rows: {
                    let mut tree = build_tree(folders, documents);
                    for file in &self.unreadable {
                        insert_unreadable(&mut tree, file);
                    }
                    tree
                },
                complete: *folders_complete && *documents_complete,
                index: index.clone(),
            },
            _ => ListState::Loading,
        };
        // One read landed and the other has not: the tree is still
        // loading, and the rows kept over the reload stay.
        if !matches!(state, ListState::Loading) {
            self.tree.apply_state(state);
        }
    }
}

#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
