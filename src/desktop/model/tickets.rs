//! The ticket list of the selected repository and the query it is read
//! under.
//!
//! The list is `list_tickets` under the query's `TicketFilter`. A filter
//! never removes a nonconforming entry; the library keeps it, and so does
//! the list. A query change moves the scope's generation on, so a read
//! made under the last query is dropped when it completes.

use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureFilter, ClosureState, ItemDto, ItemListDto, ReadinessFilter, TicketFilter,
    },
    results::ProblemCode,
};

use super::{
    list_state::ListState,
    navigation::OpenTarget,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};
use crate::desktop::services::ReadResult;

/// Which tickets the list asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TicketQuery {
    pub closure: ClosureFilter,
    pub readiness: Option<ReadinessFilter>,
}

impl Default for TicketQuery {
    /// Open tickets, ready or not (decision of the Wave).
    fn default() -> Self {
        Self {
            closure: ClosureFilter::Open,
            readiness: None,
        }
    }
}

impl TicketQuery {
    pub(crate) fn to_filter(self) -> TicketFilter {
        TicketFilter {
            closure: self.closure,
            readiness: self.readiness,
            ..TicketFilter::default()
        }
    }
}

/// Whether a ticket carries lifecycle closure metadata, whatever its
/// status text says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Lifecycle {
    Open,
    Closed,
}

/// One row of the list, as the library listed it. Nothing is made up: a
/// nonconforming entry has no title, type, status or lifecycle, only its
/// path and its problem.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TicketRow {
    /// `None` for a nonconforming entry.
    pub id: Option<ItemId>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub ticket_type: Option<String>,
    pub status: Option<String>,
    /// `None` for a nonconforming entry, which has no closure.
    pub lifecycle: Option<Lifecycle>,
    pub changed_at: Option<String>,
    /// The first problem of an entry with no ID.
    pub nonconforming: Option<ProblemCode>,
    pub path: String,
    pub worktree: String,
}

impl TicketRow {
    pub(crate) fn from_item(item: &ItemDto) -> Self {
        let id = item.id.as_deref().and_then(|id| id.parse().ok());
        Self {
            nonconforming: match id {
                Some(_) => None,
                None => item.problems.first().map(|problem| problem.code),
            },
            id,
            slug: item.slug.clone(),
            title: item.title.clone(),
            ticket_type: item.ticket_type.clone(),
            status: item.status.clone(),
            lifecycle: item.closure.as_ref().map(|closure| match closure.state {
                ClosureState::Open => Lifecycle::Open,
                ClosureState::Closed => Lifecycle::Closed,
            }),
            changed_at: item.changed_at.clone(),
            path: item.path.clone(),
            worktree: item.context.worktree.clone(),
        }
    }

    /// What activating the row opens.
    pub(crate) fn target(&self, repo: &RepoKey) -> OpenTarget {
        match &self.id {
            Some(id) => OpenTarget::Item {
                repo: repo.clone(),
                kind: manyhands::repository::ItemDtoKind::Ticket,
                id: id.clone(),
            },
            None => OpenTarget::Inspect {
                repo: repo.clone(),
                worktree: self.worktree.clone(),
                path: self.path.clone(),
            },
        }
    }
}

/// The selected repository's tickets under the query.
pub(crate) struct Tickets {
    repo: Option<RepoKey>,
    query: TicketQuery,
    list: ListState<Vec<TicketRow>>,
    generations: Generations,
}

impl Default for Tickets {
    fn default() -> Self {
        Self {
            repo: None,
            query: TicketQuery::default(),
            list: ListState::Loading,
            generations: Generations::default(),
        }
    }
}

impl Tickets {
    pub(crate) fn repo(&self) -> Option<&RepoKey> {
        self.repo.as_ref()
    }

    pub(crate) fn query(&self) -> TicketQuery {
        self.query
    }

    pub(crate) fn state(&self) -> &ListState<Vec<TicketRow>> {
        &self.list
    }

    /// Starts reading `repo`'s tickets under the query. Reads made for an
    /// earlier load or query are dropped when they complete.
    pub(crate) fn load(&mut self, repo: RepoKey) -> Stamp {
        self.forget();
        self.repo = Some(repo.clone());
        self.generations.next(ViewScope::Tickets(repo))
    }

    /// No repository is loaded: reads still running are dropped.
    pub(crate) fn clear(&mut self) {
        self.forget();
        self.repo = None;
    }

    /// Lists `closure` from now on. The stamp to read under, when a
    /// repository is loaded.
    pub(crate) fn set_closure(&mut self, closure: ClosureFilter) -> Option<Stamp> {
        self.query.closure = closure;
        let repo = self.repo.clone()?;
        Some(self.load(repo))
    }

    fn forget(&mut self) {
        if let Some(repo) = self.repo.take() {
            self.generations.next(ViewScope::Tickets(repo));
        }
        self.list = ListState::Loading;
    }

    /// A read completed. False when it is no longer wanted.
    pub(crate) fn apply(&mut self, stamp: Stamp, result: ReadResult<ItemListDto>) -> bool {
        let wanted = self.generations.is_current(&stamp)
            && self
                .repo
                .as_ref()
                .is_some_and(|repo| stamp.scope == ViewScope::Tickets(repo.clone()));
        if !wanted {
            return false;
        }
        self.list = ListState::from_read(result, |list| ListState::Ready {
            rows: list.items.iter().map(TicketRow::from_item).collect(),
            complete: list.complete,
            index: Some(list.index),
        });
        true
    }
}

#[cfg(test)]
#[path = "tickets_tests.rs"]
mod tests;
