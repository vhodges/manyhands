//! The document tree of the selected repository.
//!
//! `list_document_folders` and `list_documents` run in parallel under one
//! stamp; the tree is ready when both are. It holds the folders the
//! folder read gave, empty ones included, and the documents the document
//! read gave, nonconforming ones included, each in its folder. No folder
//! is made up: a document whose folder the folder read did not give sits
//! under the nearest one it did, or at the top.

use std::collections::{BTreeSet, HashMap};

use manyhands::repository::{DocumentFolderListDto, ItemDto, ItemListDto};

use super::{
    item::unmanaged,
    list_state::{KeptList, ListState},
    stamp::{Generations, RepoKey, Stamp, ViewScope},
    ticket_view::{NEEDS_FIXING_NO_GUIDANCE, NEEDS_FIXING_PREFIX, OPEN_IT_TO_FIX},
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
}

/// How many documents a tree lists, in every folder.
pub(crate) fn document_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            TreeNode::Folder { children, .. } => document_count(children),
            TreeNode::Document(_) => 1,
        })
        .sum()
}

/// The folder a path is directly in: everything before its last `/`.
fn parent(path: &str) -> Option<&str> {
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

/// A folder's id in the tree: the Kit's item id and the app's.
pub(crate) fn folder_id(path: &str) -> String {
    format!("folder:{path}")
}

/// A document's id in the tree. Paths are unique within one context's
/// list; a document in an item worktree has its own.
pub(crate) fn document_id(item: &ItemDto) -> String {
    format!("document:{}:{}", item.context.worktree, item.path)
}

/// The last part of a path.
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
fn last_segment(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, last)| last)
}

/// The folders the person has collapsed, by path (R-12). A folder not in
/// the set is expanded, so a new one starts open, and what is in the set
/// stays there when its folder is gone from a rebuild, so one that comes
/// back is as the person left it. A folder with no child nodes cannot
/// toggle: the tree never shows it expanded, whatever the set says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Expansion {
    collapsed: BTreeSet<String>,
}

impl Expansion {
    pub(crate) fn is_expanded(&self, path: &str) -> bool {
        !self.collapsed.contains(path)
    }

    #[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
    pub(crate) fn set(&mut self, path: &str, expanded: bool) {
        if expanded {
            self.collapsed.remove(path);
        } else {
            self.collapsed.insert(path.to_owned());
        }
    }

    /// Collapses every folder of `nodes`, at every depth.
    #[cfg_attr(not(test), allow(dead_code))] // Used from Task 8.
    pub(crate) fn collapse_all(&mut self, nodes: &[TreeNode]) {
        for node in nodes {
            if let TreeNode::Folder { path, children, .. } = node {
                self.collapsed.insert(path.clone());
                self.collapse_all(children);
            }
        }
    }

    /// Expands every folder above `path`, so what is at `path` is
    /// visible. `path` itself is left as it is.
    pub(crate) fn expand_ancestors(&mut self, path: &str) {
        let mut at = parent(path);
        while let Some(folder) = at {
            self.collapsed.remove(folder);
            at = parent(folder);
        }
    }
}

/// The row to select after a rebuild (R-27): `selected` while it is
/// visible, else the outermost collapsed folder above it. Nothing expands.
/// `None` when the tree has no such row.
pub(crate) fn visible_selection(
    nodes: &[TreeNode],
    expansion: &Expansion,
    selected: &str,
) -> Option<String> {
    for node in nodes {
        match node {
            TreeNode::Folder { path, children, .. } => {
                let id = folder_id(path);
                if id == selected {
                    return Some(id);
                }
                if let Some(inside) = visible_selection(children, expansion, selected) {
                    return Some(if expansion.is_expanded(path) {
                        inside
                    } else {
                        id
                    });
                }
            }
            TreeNode::Document(item) => {
                let id = document_id(item);
                if id == selected {
                    return Some(id);
                }
            }
        }
    }
    None
}

/// What a row's trailing cell says. The unsaved marker is the view's: it
/// comes from the draft registry, not the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
pub(crate) enum Trailing {
    None,
    /// A folder's documents, shown while it is collapsed or has nothing
    /// to open.
    Count(usize),
    /// A folder with nothing in it (decision 14).
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
pub(crate) enum DocKind {
    Folder,
    Document,
    /// A file that is not a managed document. `adoptable` is the item
    /// view's own `unmanaged` test, so the tree and the banner agree.
    Nonconforming {
        adoptable: bool,
    },
}

/// What one row says, apart from where it is drawn (Part E).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
pub(crate) struct DocRow {
    pub label: String,
    pub kind: DocKind,
    /// A folder with child nodes: whether it is open. `None` otherwise.
    pub expanded: Option<bool>,
    pub trailing: Trailing,
    /// The tooltip, line by line; the description is the lines joined.
    pub tooltip: Vec<String>,
}

#[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
pub(crate) const EMPTY_FOLDER_TOOLTIP: &str = "Empty folder. Shared once it contains a document.";
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
pub(crate) const ADOPT_TOOLTIP: &str = "Not a managed document yet. Open to adopt it.";

/// The row `node` is drawn as, a folder being `expanded` or not.
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
pub(crate) fn doc_row(node: &TreeNode, expanded: bool) -> DocRow {
    match node {
        TreeNode::Folder {
            path,
            empty,
            children,
        } => {
            let (expanded, trailing, tooltip) = if !children.is_empty() {
                let trailing = if expanded {
                    Trailing::None
                } else {
                    Trailing::Count(document_count(children))
                };
                (Some(expanded), trailing, vec![path.clone()])
            } else if *empty {
                (None, Trailing::Empty, vec![EMPTY_FOLDER_TOOLTIP.to_owned()])
            } else {
                (None, Trailing::Count(0), vec![path.clone()])
            };
            DocRow {
                label: last_segment(path).to_owned(),
                kind: DocKind::Folder,
                expanded,
                trailing,
                tooltip,
            }
        }
        TreeNode::Document(item) if item.id.is_some() => DocRow {
            label: item
                .title
                .clone()
                .unwrap_or_else(|| last_segment(&item.path).to_owned()),
            kind: DocKind::Document,
            expanded: None,
            trailing: Trailing::None,
            tooltip: vec![item.path.clone()],
        },
        TreeNode::Document(item) => {
            let adoptable = unmanaged(item);
            let reason = if adoptable {
                ADOPT_TOOLTIP.to_owned()
            } else {
                match item.problems.first() {
                    Some(problem) => format!(
                        "{NEEDS_FIXING_PREFIX}{}{OPEN_IT_TO_FIX}",
                        problem.code.guidance()
                    ),
                    None => NEEDS_FIXING_NO_GUIDANCE.to_owned(),
                }
            };
            DocRow {
                label: last_segment(&item.path).to_owned(),
                kind: DocKind::Nonconforming { adoptable },
                expanded: None,
                trailing: Trailing::None,
                tooltip: vec![reason, item.path.clone()],
            }
        }
    }
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
    #[cfg_attr(not(test), allow(dead_code))] // Used from Task 6.
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
                rows: build_tree(folders, documents),
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
