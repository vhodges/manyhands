//! What a list row says besides its text: unsaved work, a conflict, a
//! closed ticket among open ones. Shared by Tickets and Documents (R-8).
//! Everything here is in-memory state; nothing reads a file.

use manyhands::repository::{ClosureFilter, ItemContextKind, ItemDto};

use super::{
    item::{branch_label, context_kind_label},
    polling::retire_loop::Held,
};

pub(crate) const GIT_PREFIX: &str = "Git: ";

/// Where a row's copy was read from: its kind, branch and worktree, for
/// the tooltip (decision 8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowContext {
    pub kind: ItemContextKind,
    pub branch: Option<String>,
    pub worktree: String,
}

impl RowContext {
    pub(crate) fn of(item: &ItemDto) -> RowContext {
        RowContext {
            kind: item.context.kind,
            branch: item.context.branch.clone(),
            worktree: item.context.worktree.clone(),
        }
    }

    /// The copy's kind, in the details sheet's words.
    #[cfg_attr(not(test), allow(dead_code))] // Used from Task 3.
    pub(crate) fn label(&self) -> &'static str {
        context_kind_label(self.kind)
    }

    /// `Git: branch {branch} · worktree {worktree}`; a missing branch is
    /// `NO_BRANCH`, as the details sheet says it.
    #[cfg_attr(not(test), allow(dead_code))] // Used from Task 3.
    pub(crate) fn git_line(&self) -> String {
        let branch = branch_label(self.branch.as_deref());
        format!("{GIT_PREFIX}{branch} \u{b7} worktree {}", self.worktree)
    }
}

/// The marks a row carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 3.
pub(crate) struct Markers {
    /// A draft of the item, or of a comment on it, is held.
    pub unsaved: bool,
    /// The repository's pending conflicts name the row.
    pub conflict: bool,
    /// Closed, among open tickets under All.
    pub closed: bool,
}

/// The markers of a row. `Held::Unknown` marks nothing, so a store that
/// cannot be read does not mark every row (R-8). A closed row is marked
/// only under All, where it stands among open ones.
#[cfg_attr(not(test), allow(dead_code))] // Used from Task 3.
pub(crate) fn row_markers(
    held: Held,
    conflict: bool,
    closed: bool,
    closure: ClosureFilter,
) -> Markers {
    Markers {
        unsaved: held == Held::Yes,
        conflict,
        closed: closed && closure == ClosureFilter::All,
    }
}

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;
