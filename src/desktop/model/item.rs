//! The read-only item view's state: the item, read by ID or, for a
//! nonconforming entry, by path, and its comments.
//!
//! Its stamps are under the scope of the tab that shows it. Every text
//! here is fixed or the library's own; nothing is formatted with `Debug`.

use manyhands::{
    repository::{
        CommentDto, CommentListDto, ItemContextDto, ItemContextKind, ItemDto, ProblemDto,
    },
    results::ProblemCode,
};
use serde_json::Value;

use super::{
    list_state::{ListState, Rows},
    navigation::OpenTarget,
    stamp::{Generations, Stamp, ViewScope},
};
use crate::desktop::services::ReadResult;

pub(crate) const KIND_PRIMARY: &str = "Primary";
pub(crate) const KIND_ACTIVE: &str = "Active item worktree";
pub(crate) const KIND_UNVERIFIED: &str = "Unverified";
pub(crate) const NO_BRANCH: &str = "no branch";
pub(crate) const NOT_REPRESENTABLE: &str = "Not representable";
pub(crate) const UNKNOWN_AUTHOR: &str = "unknown author";

/// The actions the item view shows, none of them available in D1.
pub(crate) const LATER: [&str; 6] = [
    "Edit",
    "Comment",
    "Sync",
    "Approve and merge",
    "Close ticket",
    "Repair",
];
pub(crate) const LATER_NOTE: &str = "Available in a later release";

/// The problems that make the header a recovery view: the item's branch
/// or worktree needs repair, and no context is offered in its place.
/// The library does not yet report context problems on items (ticket 01M5B163PQTE9C1T6NMHATR3KX).
pub(crate) const CONTEXT_PROBLEMS: [ProblemCode; 2] =
    [ProblemCode::ContextProblem, ProblemCode::BranchProblem];

/// A context kind in text.
pub(crate) fn context_kind_label(kind: ItemContextKind) -> &'static str {
    match kind {
        ItemContextKind::Primary => KIND_PRIMARY,
        ItemContextKind::Active => KIND_ACTIVE,
        ItemContextKind::Unverified => KIND_UNVERIFIED,
    }
}

/// Where the item was read from: the context kind, its branch and its
/// worktree. It says nothing of how the copy compares with the primary.
pub(crate) fn provenance_label(ctx: &ItemContextDto) -> String {
    let kind = context_kind_label(ctx.kind);
    let branch = match &ctx.branch {
        Some(branch) => format!("branch {branch}"),
        None => NO_BRANCH.to_owned(),
    };
    format!("{kind}, {branch}, worktree {}", ctx.worktree)
}

/// The item's context and branch problems, in the item's order.
pub(crate) fn context_problems(item: &ItemDto) -> Vec<&ProblemDto> {
    item.problems
        .iter()
        .filter(|problem| CONTEXT_PROBLEMS.contains(&problem.code))
        .collect()
}

/// An unknown metadata value as shown: text as itself, anything else as
/// the library's JSON, and `None` for the null of a value JSON cannot
/// express.
pub(crate) fn metadata_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

/// Who wrote a comment, or "unknown author".
pub(crate) fn comment_author(comment: &CommentDto) -> &str {
    comment.author.as_deref().unwrap_or(UNKNOWN_AUTHOR)
}

/// An item read is one row, never none.
impl Rows for ItemDto {
    fn is_empty(&self) -> bool {
        false
    }
}

impl Rows for CommentListDto {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub(crate) struct ItemView {
    target: OpenTarget,
    scope: ViewScope,
    item: ListState<ItemDto>,
    /// `None` for a nonconforming entry, which has no ID to list comments
    /// of.
    comments: Option<ListState<CommentListDto>>,
    generations: Generations,
}

impl ItemView {
    /// The view of `target`, whose reads are stamped under `scope`.
    pub(crate) fn new(target: OpenTarget, scope: ViewScope) -> Self {
        let comments = Self::comments_for(&target);
        Self {
            target,
            scope,
            item: ListState::Loading,
            comments,
            generations: Generations::default(),
        }
    }

    fn comments_for(target: &OpenTarget) -> Option<ListState<CommentListDto>> {
        match target {
            OpenTarget::Item { .. } => Some(ListState::Loading),
            OpenTarget::Inspect { .. } => None,
        }
    }

    pub(crate) fn target(&self) -> &OpenTarget {
        &self.target
    }

    pub(crate) fn item(&self) -> &ListState<ItemDto> {
        &self.item
    }

    pub(crate) fn comments(&self) -> Option<&ListState<CommentListDto>> {
        self.comments.as_ref()
    }

    /// Starts reading the item and its comments again, both under the
    /// returned stamp. A read asked before is dropped when it completes.
    pub(crate) fn load(&mut self) -> Stamp {
        self.item = ListState::Loading;
        self.comments = Self::comments_for(&self.target);
        self.generations.next(self.scope.clone())
    }

    /// Starts reading the item and its comments again after its
    /// repository changed, under the returned stamp. What is shown stays
    /// until the reads land, so the source keeps its selection and focus;
    /// a read asked before is dropped when it completes.
    pub(crate) fn reload(&mut self) -> Stamp {
        self.generations.next(self.scope.clone())
    }

    /// The item read completed. False when it is no longer wanted.
    pub(crate) fn apply_item(&mut self, stamp: Stamp, result: ReadResult<ItemDto>) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        self.item = ListState::from_read(result, |item| {
            let index = Some(item.index.clone());
            ListState::Ready {
                rows: item,
                complete: true,
                index,
            }
        });
        true
    }

    /// The comment read completed. False when it is no longer wanted.
    pub(crate) fn apply_comments(
        &mut self,
        stamp: Stamp,
        result: ReadResult<CommentListDto>,
    ) -> bool {
        if !self.generations.is_current(&stamp) || self.comments.is_none() {
            return false;
        }
        self.comments = Some(ListState::from_read(result, |list| {
            let complete = list.complete;
            let index = Some(list.index.clone());
            ListState::Ready {
                rows: list,
                complete,
                index,
            }
        }));
        true
    }
}

#[cfg(test)]
#[path = "item_tests.rs"]
mod tests;
