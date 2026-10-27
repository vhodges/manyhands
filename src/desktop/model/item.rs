//! The read-only item view's state: the item, read by ID or, for a
//! nonconforming entry, by path, its comments and, for a ticket, its
//! relationships.
//!
//! Its stamps are under the scope of the tab that shows it. Every text
//! here is fixed or the library's own; nothing is formatted with `Debug`.

use manyhands::{
    repository::{
        CommentDto, CommentListDto, DependencyTreeDto, ItemContextDto, ItemContextKind, ItemDto,
        ItemDtoKind, ItemListDto, ProblemDto,
    },
    results::{ProblemCode, ResultCode},
};
use serde_json::Value;

use super::{
    list_state::{ListState, Rows},
    navigation::{NotReloaded, OpenTarget},
    relationships::Relationships,
    stamp::{Generations, Stamp, ViewScope},
};
use crate::desktop::services::ReadResult;

pub(crate) const KIND_PRIMARY: &str = "Primary";
pub(crate) const KIND_ACTIVE: &str = "Active item worktree";
pub(crate) const KIND_UNVERIFIED: &str = "Unverified";
pub(crate) const NO_BRANCH: &str = "no branch";
pub(crate) const NOT_REPRESENTABLE: &str = "Not representable";
pub(crate) const UNKNOWN_AUTHOR: &str = "unknown author";

/// The design's action labels, each by name.
pub(crate) const EDIT: &str = "Edit";
pub(crate) const COMMENT: &str = "Comment";
pub(crate) const SHARE: &str = "Share";
pub(crate) const APPROVE_AND_MERGE: &str = "Approve and merge…";
pub(crate) const CLOSE_TICKET: &str = "Close ticket…";
pub(crate) const REPAIR: &str = "Repair";

/// The actions the item view shows that are not available yet, with the
/// design's labels: the header's Edit, Share and primary action, the
/// discussion's Comment, and the "Needs fixing" banner's Repair.
pub(crate) const LATER: [&str; 6] = [
    EDIT,
    COMMENT,
    SHARE,
    APPROVE_AND_MERGE,
    CLOSE_TICKET,
    REPAIR,
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

/// What one load of a tab read, in the design's order: the item, its
/// comments, and for a ticket its dependency tree, its children and its
/// parent. `None` for a read the target does not make, or one not made.
pub(crate) struct ItemReads {
    pub item: ReadResult<ItemDto>,
    pub comments: Option<ReadResult<CommentListDto>>,
    pub dependents: Option<ReadResult<DependencyTreeDto>>,
    pub children: Option<ReadResult<ItemListDto>>,
    pub parent: Option<ReadResult<ItemDto>>,
}

impl ItemReads {
    /// A load that stopped at `item`, which gave nothing: every read the
    /// target makes after it gives the same answer, and the parent is
    /// not read.
    pub(crate) fn stopped(target: &OpenTarget, item: ReadResult<ItemDto>) -> Self {
        let (comments, ticket) = match target {
            OpenTarget::Item { kind, .. } => (true, *kind == ItemDtoKind::Ticket),
            OpenTarget::Inspect { .. } => (false, false),
        };
        Self {
            comments: comments.then(|| item.stopped()).flatten(),
            dependents: ticket.then(|| item.stopped()).flatten(),
            children: ticket.then(|| item.stopped()).flatten(),
            parent: None,
            item,
        }
    }
}

pub(crate) struct ItemView {
    target: OpenTarget,
    scope: ViewScope,
    item: ListState<ItemDto>,
    /// `None` for a nonconforming entry, which has no ID to list comments
    /// of.
    comments: Option<ListState<CommentListDto>>,
    /// `None` for anything but a ticket by ID.
    relationships: Option<Relationships>,
    /// Why what is shown is from an earlier load: the last load stayed
    /// busy while rows were shown, and they were kept (ruling R-B5).
    not_refreshed: Option<NotReloaded>,
    /// The title the last read that gave the item gave it, kept for the
    /// tab when a later load fails.
    last_title: Option<String>,
    generations: Generations,
}

impl ItemView {
    /// The view of `target`, whose reads are stamped under `scope`.
    pub(crate) fn new(target: OpenTarget, scope: ViewScope) -> Self {
        let comments = Self::comments_for(&target);
        let relationships = Self::relationships_for(&target);
        Self {
            target,
            scope,
            item: ListState::Loading,
            comments,
            relationships,
            not_refreshed: None,
            last_title: None,
            generations: Generations::default(),
        }
    }

    fn comments_for(target: &OpenTarget) -> Option<ListState<CommentListDto>> {
        match target {
            OpenTarget::Item { .. } => Some(ListState::Loading),
            OpenTarget::Inspect { .. } => None,
        }
    }

    fn relationships_for(target: &OpenTarget) -> Option<Relationships> {
        match target {
            OpenTarget::Item {
                kind: ItemDtoKind::Ticket,
                ..
            } => Some(Relationships::default()),
            _ => None,
        }
    }

    /// A ticket's relationships; `None` for a document or a nonconforming
    /// entry.
    pub(crate) fn relationships(&self) -> Option<&Relationships> {
        self.relationships.as_ref()
    }

    /// The ticket's parent to read for its name, once the item read gave a
    /// resolved one.
    #[cfg(test)]
    pub(crate) fn parent_to_read(&self) -> Option<manyhands::canonical::ItemId> {
        self.relationships.as_ref()?.parent_to_read()
    }

    pub(crate) fn target(&self) -> &OpenTarget {
        &self.target
    }

    pub(crate) fn item(&self) -> &ListState<ItemDto> {
        &self.item
    }

    /// The item as the last load gave it, as a draft's target state takes
    /// it: the item, or the code of the read that failed. `None` while the
    /// first load is in flight or stayed busy.
    pub(crate) fn item_read(&self) -> Option<Result<&ItemDto, ResultCode>> {
        match &self.item {
            ListState::Ready { rows, .. } => Some(Ok(rows)),
            ListState::Failed(failure) => Some(Err(failure.code)),
            _ => None,
        }
    }

    pub(crate) fn comments(&self) -> Option<&ListState<CommentListDto>> {
        self.comments.as_ref()
    }

    /// The title the last read that gave the item gave it.
    pub(crate) fn last_title(&self) -> Option<&str> {
        self.last_title.as_deref()
    }

    /// Why what is shown is from an earlier load, when it is.
    pub(crate) fn not_refreshed(&self) -> Option<&NotReloaded> {
        self.not_refreshed.as_ref()
    }

    /// Starts the first load: everything shows Loading until the load
    /// lands, under the returned stamp. A load asked before is dropped
    /// when it completes.
    pub(crate) fn load(&mut self) -> Stamp {
        self.item = ListState::Loading;
        self.comments = Self::comments_for(&self.target);
        self.relationships = Self::relationships_for(&self.target);
        self.not_refreshed = None;
        self.generations.next(self.scope.clone())
    }

    /// Starts loading again, after the repository may have changed or on
    /// Retry, under the returned stamp. What is shown stays until the load
    /// lands, so the source keeps its selection and focus; a load asked
    /// before is dropped when it completes.
    pub(crate) fn reload(&mut self) -> Stamp {
        self.generations.next(self.scope.clone())
    }

    /// A load completed. Each read that gave its value replaces what was
    /// shown; one that stayed busy while rows were shown keeps them, and
    /// `not_refreshed` says so; any other, a failure included, shows its
    /// answer as a first load does (ruling R-B5). A parent read that gave
    /// nothing keeps the parent's name while its ID is unchanged. False
    /// when the load is no longer wanted.
    pub(crate) fn apply_reads(&mut self, stamp: Stamp, reads: ItemReads) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        let ItemReads {
            item,
            comments,
            dependents,
            children,
            parent,
        } = reads;
        if let ReadResult::Ok(item) = &item {
            self.last_title = item.title.clone();
            if let Some(relationships) = &mut self.relationships {
                relationships.set_item(item);
            }
        }
        let mut why = self.item.apply_keeping(item, |item| {
            let index = Some(item.index.clone());
            ListState::Ready {
                rows: item,
                complete: true,
                index,
            }
        });
        if let (Some(state), Some(result)) = (&mut self.comments, comments) {
            let kept = state.apply_keeping(result, |list| {
                let complete = list.complete;
                let index = Some(list.index.clone());
                ListState::Ready {
                    rows: list,
                    complete,
                    index,
                }
            });
            why = why.or(kept);
        }
        if let Some(relationships) = &mut self.relationships {
            if let Some(result) = dependents {
                let kept = relationships.apply_dependents(result);
                why = why.or(kept);
            }
            if let Some(result) = children {
                let kept = relationships.apply_children(result);
                why = why.or(kept);
            }
            if let Some(result) = parent {
                relationships.apply_parent(result);
            }
        }
        self.not_refreshed = why;
        true
    }
}

#[cfg(test)]
#[path = "item_tests.rs"]
mod tests;
