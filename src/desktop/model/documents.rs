//! The document tree of the selected repository.
//!
//! `list_document_folders` and `list_documents` run in parallel under one
//! stamp; the tree is ready when both are. It holds the folders the
//! folder read gave, empty ones included, and the documents the document
//! read gave, nonconforming ones included, each in its folder. No folder
//! is made up: a document whose folder the folder read did not give sits
//! under the nearest one it did, or at the top.

use manyhands::repository::{DocumentFolderListDto, ItemDto, ItemListDto};

use super::{
    list_state::ListState,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
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

/// The selected repository's tree, and the two reads it is made of.
pub(crate) struct Documents {
    repo: Option<RepoKey>,
    folders: ListState<DocumentFolderListDto>,
    documents: ListState<ItemListDto>,
    tree: ListState<Vec<TreeNode>>,
    generations: Generations,
}

impl Default for Documents {
    fn default() -> Self {
        Self {
            repo: None,
            folders: ListState::Loading,
            documents: ListState::Loading,
            tree: ListState::Loading,
            generations: Generations::default(),
        }
    }
}

impl Documents {
    /// The repository the tree is of, while one is loaded.
    pub(crate) fn repo(&self) -> Option<&RepoKey> {
        self.repo.as_ref()
    }

    pub(crate) fn state(&self) -> &ListState<Vec<TreeNode>> {
        &self.tree
    }

    /// Starts loading `repo`'s tree: the stamp both reads are made under.
    /// Reads made for an earlier load are dropped when they complete.
    pub(crate) fn load(&mut self, repo: RepoKey) -> Stamp {
        self.forget();
        self.repo = Some(repo.clone());
        self.generations.next(ViewScope::Documents(repo))
    }

    /// No repository is loaded: reads still running are dropped.
    pub(crate) fn clear(&mut self) {
        self.forget();
        self.repo = None;
    }

    fn forget(&mut self) {
        if let Some(repo) = self.repo.take() {
            self.generations.next(ViewScope::Documents(repo));
        }
        self.folders = ListState::Loading;
        self.documents = ListState::Loading;
        self.tree = ListState::Loading;
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
        self.tree = match (&self.folders, &self.documents) {
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
    }
}

#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
