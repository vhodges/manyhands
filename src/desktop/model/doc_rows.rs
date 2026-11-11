//! What each row of the document tree says (Part E): the folders the
//! person collapsed, the row to select after a rebuild, and `doc_row`.

use std::collections::BTreeSet;

use super::{
    documents::{
        TreeNode, document_count, document_id, folder_id, last_segment, parent, unreadable_id,
    },
    item::unmanaged,
    ticket_view::{NEEDS_FIXING_NO_GUIDANCE, NEEDS_FIXING_PREFIX, OPEN_IT_TO_FIX},
    unreadable::unreadable_tooltip,
};

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
            TreeNode::Unreadable(file) => {
                let id = unreadable_id(file);
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
pub(crate) enum Trailing {
    None,
    /// A folder's documents, shown while it is collapsed or has nothing
    /// to open.
    Count(usize),
    /// A folder with nothing in it (decision 14).
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DocKind {
    Folder,
    Document,
    /// A file that is not a managed document. `adoptable` is the item
    /// view's own `unmanaged` test, so the tree and the banner agree.
    Nonconforming {
        adoptable: bool,
    },
    /// A file the library could not read; it does not open (R-38).
    Unreadable,
}

/// What one row says, apart from where it is drawn (Part E).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DocRow {
    pub label: String,
    pub kind: DocKind,
    /// A folder with child nodes: whether it is open. `None` otherwise.
    pub expanded: Option<bool>,
    pub trailing: Trailing,
    /// The tooltip, line by line; the description is the lines joined.
    pub tooltip: Vec<String>,
}

pub(crate) const EMPTY_FOLDER_TOOLTIP: &str = "Empty folder. Shared once it contains a document.";
pub(crate) const ADOPT_TOOLTIP: &str = "Not a managed document yet. Open to adopt it.";

/// The row `node` is drawn as, a folder being `expanded` or not.
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
        TreeNode::Unreadable(file) => DocRow {
            label: last_segment(&file.path).to_owned(),
            kind: DocKind::Unreadable,
            expanded: None,
            trailing: Trailing::None,
            tooltip: unreadable_tooltip(file),
        },
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
