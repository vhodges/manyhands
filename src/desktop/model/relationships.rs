//! A ticket's relationships: what it depends on, what depends on it, its
//! parent and its children (design, Item Inspection and View Models).
//!
//! The ticket's own dependencies and parent, each with its resolution,
//! and its relationship problems come from the item read. Its dependents
//! come from `ticket_dependencies` in the `up` direction, to depth 1: the
//! library's `up` is the tickets that depend on it. That read asks for
//! both directions, so the same snapshot also gives each dependency's
//! short code and title. Its children come from `ticket_children`, and the
//! parent's short code and title from a read of the parent itself.
//!
//! A related ticket is named only by what the library returned for it: a
//! row with no library row behind it shows its ID alone, and an
//! unresolved target, which no context holds, shows its ID and opens
//! nothing. Every text here is fixed or the library's own.

use std::collections::HashMap;

use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureState, DependencyDto, DependencyState, DependencyTreeDto, ItemDto, ItemDtoKind,
        ItemListDto, ProblemDto,
    },
    results::ProblemCode,
};

use super::{
    list_state::{ListState, Rows},
    navigation::{NotReloaded, OpenTarget},
    stamp::RepoKey,
    tickets::Lifecycle,
};
use crate::desktop::services::ReadResult;

pub(crate) const RELATIONSHIPS: &str = "Relationships";
pub(crate) const DEPENDENCIES: &str = "Dependencies";
pub(crate) const DEPENDENTS: &str = "Dependents";
pub(crate) const PARENT: &str = "Parent";
pub(crate) const CHILDREN: &str = "Children";
pub(crate) const PROBLEMS: &str = "Relationship problems";
pub(crate) const NONE: &str = "None";
pub(crate) const RESOLVED: &str = "Resolved";
pub(crate) const UNRESOLVED: &str = "Unresolved";
pub(crate) const CANNOT_OPEN: &str = "No ticket with this ID was found, so it cannot be opened";

/// The problems with a ticket's `deps` and `parent`, which the panel lists
/// under its groups.
pub(crate) const RELATIONSHIP_PROBLEMS: [ProblemCode; 7] = [
    ProblemCode::RelationshipWrongType,
    ProblemCode::RelationshipInvalidId,
    ProblemCode::RelationshipSelfReference,
    ProblemCode::DuplicateDependency,
    ProblemCode::RelationshipNotATicket,
    ProblemCode::DependencyCycle,
    ProblemCode::ParentCycle,
];

/// The parent an item read named, to read for its name: a resolved one
/// only.
pub(crate) fn parent_to_read(parent: Option<&DependencyDto>) -> Option<ItemId> {
    let parent = parent?;
    if parent.state == DependencyState::Unresolved {
        return None;
    }
    parent.id.parse().ok()
}

/// A resolution in text.
pub(crate) fn resolution_label(resolved: bool) -> &'static str {
    if resolved { RESOLVED } else { UNRESOLVED }
}

impl Rows for DependencyTreeDto {
    fn is_empty(&self) -> bool {
        self.dependents.is_empty()
    }
}

impl Rows for ItemListDto {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// One related ticket as the panel shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelatedRow {
    pub id: String,
    /// From a library row for the ticket; `None` when there is none.
    pub slug: Option<String>,
    pub title: Option<String>,
    /// `None` for an unresolved target.
    pub lifecycle: Option<Lifecycle>,
    pub resolved: bool,
}

impl RelatedRow {
    fn from_dependency(dependency: &DependencyDto, named: Option<(&Option<String>, &str)>) -> Self {
        let (lifecycle, resolved) = match dependency.state {
            DependencyState::Open => (Some(Lifecycle::Open), true),
            DependencyState::Closed => (Some(Lifecycle::Closed), true),
            DependencyState::Unresolved => (None, false),
        };
        let (slug, title) = match named.filter(|_| resolved) {
            Some((slug, title)) => (slug.clone(), Some(title.to_owned())),
            None => (None, None),
        };
        Self {
            id: dependency.id.clone(),
            slug,
            title,
            lifecycle,
            resolved,
        }
    }

    /// What activating the row opens: the ticket by its ID in `repo`, the
    /// repository of the tab that shows it. Nothing for an unresolved
    /// target.
    pub(crate) fn target(&self, repo: &RepoKey) -> Option<OpenTarget> {
        if !self.resolved {
            return None;
        }
        let id: ItemId = self.id.parse().ok()?;
        Some(OpenTarget::Item {
            repo: repo.clone(),
            kind: ItemDtoKind::Ticket,
            id,
        })
    }
}

fn ready<T: Rows>(
    rows: T,
    complete: bool,
    index: manyhands::repository::IndexStateDto,
) -> ListState<T> {
    ListState::Ready {
        rows,
        complete,
        index: Some(index),
    }
}

/// The ticket's relationships, from its item read and the three reads made
/// beside it.
pub(crate) struct Relationships {
    /// The item's `deps`, in its file's order, each with its resolution.
    dependencies: Vec<DependencyDto>,
    /// `ticket_dependencies` in both directions to depth 1: `dependents`
    /// gives the dependents, `dependencies` names the dependencies.
    dependents: ListState<DependencyTreeDto>,
    parent: Option<DependencyDto>,
    /// The parent's own read, which names it.
    parent_item: Option<ItemDto>,
    children: ListState<ItemListDto>,
    problems: Vec<ProblemDto>,
}

impl Default for Relationships {
    fn default() -> Self {
        Self {
            dependencies: Vec::new(),
            dependents: ListState::Loading,
            parent: None,
            parent_item: None,
            children: ListState::Loading,
            problems: Vec::new(),
        }
    }
}

impl Relationships {
    /// The item read gave the ticket: its dependencies, parent and
    /// relationship problems. A parent that changed is no longer named.
    pub(crate) fn set_item(&mut self, item: &ItemDto) {
        self.dependencies = item.deps.clone();
        if self.parent.as_ref().map(|parent| &parent.id)
            != item.parent.as_ref().map(|parent| &parent.id)
        {
            self.parent_item = None;
        }
        self.parent = item.parent.clone();
        self.problems = item
            .problems
            .iter()
            .filter(|problem| RELATIONSHIP_PROBLEMS.contains(&problem.code))
            .cloned()
            .collect();
    }

    /// The dependency tree read completed. One that gave nothing while
    /// rows are shown keeps them, and says why.
    pub(crate) fn apply_dependents(
        &mut self,
        result: ReadResult<DependencyTreeDto>,
    ) -> Option<NotReloaded> {
        self.dependents.apply_keeping(result, |tree| {
            let (complete, index) = (tree.complete, tree.index.clone());
            ready(tree, complete, index)
        })
    }

    /// The children read completed. One that gave nothing while rows are
    /// shown keeps them, and says why.
    pub(crate) fn apply_children(
        &mut self,
        result: ReadResult<ItemListDto>,
    ) -> Option<NotReloaded> {
        self.children.apply_keeping(result, |list| {
            let (complete, index) = (list.complete, list.index.clone());
            ready(list, complete, index)
        })
    }

    /// The parent's read completed: it names the parent when it gave that
    /// ticket, and one that gave another names it no more. One that stayed
    /// busy or failed keeps the name shown, which `set_item` has already
    /// dropped if the parent's ID changed; with none, the parent shows by
    /// its ID.
    pub(crate) fn apply_parent(&mut self, result: ReadResult<ItemDto>) {
        match result {
            ReadResult::Ok(item)
                if item.id.is_some()
                    && item.id.as_ref() == self.parent.as_ref().map(|parent| &parent.id) =>
            {
                self.parent_item = Some(item);
            }
            ReadResult::Ok(_) => self.parent_item = None,
            ReadResult::Busy | ReadResult::Failed(_) => {}
        }
    }

    /// The parent to read for its name: a resolved one only. The view
    /// reads it from the item read itself (`parent_to_read` above).
    #[cfg(test)]
    pub(crate) fn parent_to_read(&self) -> Option<ItemId> {
        parent_to_read(self.parent.as_ref())
    }

    pub(crate) fn dependents(&self) -> &ListState<DependencyTreeDto> {
        &self.dependents
    }

    pub(crate) fn children(&self) -> &ListState<ItemListDto> {
        &self.children
    }

    pub(crate) fn problems(&self) -> &[ProblemDto] {
        &self.problems
    }

    /// The dependencies in the ticket's order, each named by its line of
    /// the tree once that has landed.
    pub(crate) fn dependency_rows(&self) -> Vec<RelatedRow> {
        let names: HashMap<&str, (&Option<String>, &str)> = self
            .dependents
            .rows()
            .map(|tree| {
                tree.dependencies
                    .iter()
                    .filter(|line| line.depth == 1)
                    .filter_map(|line| {
                        let title = line.title.as_deref()?;
                        Some((line.id.as_str(), (&line.slug, title)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.dependencies
            .iter()
            .map(|dependency| {
                RelatedRow::from_dependency(dependency, names.get(dependency.id.as_str()).copied())
            })
            .collect()
    }

    /// The tickets that depend on this one directly, once read.
    pub(crate) fn dependent_rows(&self) -> Vec<RelatedRow> {
        let Some(tree) = self.dependents.rows() else {
            return Vec::new();
        };
        tree.dependents
            .iter()
            .filter(|line| line.depth == 1)
            .map(|line| {
                let (lifecycle, resolved) = match line.state {
                    DependencyState::Open => (Some(Lifecycle::Open), true),
                    DependencyState::Closed => (Some(Lifecycle::Closed), true),
                    DependencyState::Unresolved => (None, false),
                };
                RelatedRow {
                    id: line.id.clone(),
                    slug: line.slug.clone(),
                    title: line.title.clone(),
                    lifecycle,
                    resolved,
                }
            })
            .collect()
    }

    /// The parent, named by its own read once that has landed.
    pub(crate) fn parent_row(&self) -> Option<RelatedRow> {
        let parent = self.parent.as_ref()?;
        let named = self
            .parent_item
            .as_ref()
            .and_then(|item| Some((&item.slug, item.title.as_deref()?)));
        Some(RelatedRow::from_dependency(parent, named))
    }

    /// The children, in the ticket list's order, once read.
    pub(crate) fn child_rows(&self) -> Vec<RelatedRow> {
        let Some(list) = self.children.rows() else {
            return Vec::new();
        };
        list.items
            .iter()
            .filter_map(|item| {
                Some(RelatedRow {
                    id: item.id.clone()?,
                    slug: item.slug.clone(),
                    title: item.title.clone(),
                    lifecycle: item.closure.as_ref().map(|closure| match closure.state {
                        ClosureState::Closed => Lifecycle::Closed,
                        ClosureState::Open => Lifecycle::Open,
                    }),
                    resolved: true,
                })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "relationships_tests.rs"]
mod tests;
