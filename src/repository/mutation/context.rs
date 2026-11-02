//! The names of an item's editing context: the branch its changes are
//! committed to and the worktree that branch is checked out in.
//!
//! This is the one place the boundary writes them. The names are the ones
//! the domain gives a context when it creates one: a request's position
//! is recorded against the branch before the context exists, and its
//! evidence is read from the worktree afterwards.

use std::path::{Path, PathBuf};

use super::super::{AuthoringKind, authoring_kind_segment};

/// Where item worktrees are, below the repository root.
const WORKTREES_DIRECTORY: &str = ".manyhands/worktrees";

/// The names of one item's editing context. Neither need exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContextNames {
    /// The branch's full reference name, `refs/heads/...`.
    pub(crate) branch_ref: String,
    pub(crate) worktree: PathBuf,
}

/// The names of the editing context of the item `id`, of kind `kind`, in
/// the repository at `root`.
pub(crate) fn context_names(root: &Path, kind: &AuthoringKind, id: &str) -> ContextNames {
    ContextNames {
        branch_ref: format!("refs/heads/manyhands/{}/{id}", authoring_kind_segment(kind)),
        worktree: root.join(WORKTREES_DIRECTORY).join(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

    #[test]
    fn a_ticket_s_context_is_named_as_the_domain_names_it() {
        let names = context_names(Path::new("/repository"), &AuthoringKind::Ticket, ID);
        assert_eq!(
            names.branch_ref,
            "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW"
        );
        assert_eq!(
            names.worktree,
            Path::new("/repository/.manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAW")
        );
    }

    #[test]
    fn a_document_s_context_has_its_own_branch_and_the_same_worktree_directory() {
        let names = context_names(Path::new("/repository"), &AuthoringKind::Document, ID);
        assert_eq!(
            names.branch_ref,
            "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAW"
        );
        assert_eq!(
            names.worktree,
            Path::new("/repository/.manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAW")
        );
    }
}
