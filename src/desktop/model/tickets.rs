//! The ticket list of the selected repository and the query it is shown
//! under.
//!
//! The list is one `list_tickets` read per load, under the library's
//! default filter: every ticket, closed ones included (R-1). The query,
//! with its lifecycle, readiness, text and sort, is applied in the
//! desktop by `ticket_view::derive`, so a query change never reads again.
//! A filter never removes a nonconforming entry; the library keeps it,
//! and so does the list.
//!
//! Every blocked row carries why it is blocked, in text made from the
//! library's reasons, named from all the rows the read gave. A blocker is
//! named by the short code and title of the row the library gave for it,
//! and only by its ID when no row does: nothing is made up.

use std::collections::HashMap;

use manyhands::{
    canonical::ItemId,
    repository::{
        ClosureFilter, ClosureState, ItemContextKind, ItemDto, ItemListDto, ReadinessFilter,
        ReadinessReasonCode, ReadinessState, TicketFilter,
    },
    results::ProblemCode,
};

use super::{
    list_state::{KeptList, ListState},
    markers::RowContext,
    names::TicketName,
    navigation::{ItemLocator, OpenTarget},
    stamp::{Generations, RepoKey, Stamp, ViewScope},
    ticket_view::{TicketView, derive, sole_match},
    unreadable::UnreadableFile,
};
use crate::desktop::services::ReadResult;

/// A column the list sorts by.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SortColumn {
    Type,
    Code,
    Title,
    Status,
    Updated,
}

/// The column the list is sorted by and which way. For Updated,
/// ascending is newest first.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TicketSort {
    pub column: SortColumn,
    pub ascending: bool,
}

impl Default for TicketSort {
    /// Newest first.
    fn default() -> Self {
        Self {
            column: SortColumn::Updated,
            ascending: true,
        }
    }
}

impl TicketSort {
    /// Sorting by `column`: a new column ascending, the same one reversed.
    pub(crate) fn toggled(self, column: SortColumn) -> Self {
        Self {
            column,
            ascending: self.column != column || !self.ascending,
        }
    }
}

/// How the list is filtered and sorted. Nothing here is a read: the one
/// read is `read_filter`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct TicketQuery {
    pub closure: ClosureFilter,
    pub readiness: Option<ReadinessFilter>,
    pub text: String,
    pub sort: TicketSort,
}

impl Default for TicketQuery {
    /// Open tickets, ready or not, newest first (decision of the Wave).
    fn default() -> Self {
        Self {
            closure: ClosureFilter::Open,
            readiness: None,
            text: String::new(),
            sort: TicketSort::default(),
        }
    }
}

impl TicketQuery {
    /// Whether the text or a readiness choice narrows the list.
    pub(crate) fn narrowed(&self) -> bool {
        !self.text.trim().is_empty() || self.readiness.is_some()
    }

    /// Lists `closure`. Only open tickets are ready or blocked, so Closed
    /// sets Any readiness (R-26).
    pub(crate) fn with_closure(mut self, closure: ClosureFilter) -> Self {
        self.closure = closure;
        if closure == ClosureFilter::Closed {
            self.readiness = None;
        }
        self
    }

    /// Lists `readiness`. Ready or Blocked while Closed is chosen chooses
    /// Open (R-26).
    pub(crate) fn with_readiness(mut self, readiness: Option<ReadinessFilter>) -> Self {
        self.readiness = readiness;
        if readiness.is_some() && self.closure == ClosureFilter::Closed {
            self.closure = ClosureFilter::Open;
        }
        self
    }
}

/// The one read per load: every ticket, whatever the query (R-1).
pub(crate) fn read_filter() -> TicketFilter {
    TicketFilter::default()
}

pub(crate) const OPEN_DEPENDENCY: &str = "Waits for open ticket";
pub(crate) const UNRESOLVED_DEPENDENCY: &str = "Waits for a ticket not found";
pub(crate) const DEPENDENCY_CYCLE: &str = "On a dependency cycle";
pub(crate) const INCOMPLETE: &str = "incomplete";

/// Why `item` is blocked, one line per reason the library gave, in its
/// order: an open dependency by the blocker's short code and title, an
/// unresolved one by the ID it names, and a cycle by the tickets on it.
/// A ticket is named from `by_id`, the rows the library returned, keyed by
/// ID; one not there is named by its ID alone, and one with no title by
/// its short code when it has one. A reason the library marks
/// incomplete says so. Empty for a ticket that is not blocked.
pub(crate) fn blocked_reasons(item: &ItemDto, by_id: &HashMap<String, ItemDto>) -> Vec<String> {
    let Some(readiness) = &item.readiness else {
        return Vec::new();
    };
    let name = |id: &String| {
        let item = by_id.get(id);
        TicketName {
            slug: item.and_then(|item| item.slug.as_deref()),
            title: item.and_then(|item| item.title.as_deref()),
        }
    };
    let short = |id: &String| name(id).short_or(id).to_owned();
    let named = |id: &String| name(id).with_short(id);
    readiness
        .reasons
        .iter()
        .map(|reason| {
            let text = match reason.code {
                ReadinessReasonCode::OpenDependency => format!(
                    "{OPEN_DEPENDENCY}: {}",
                    reason.ids.iter().map(named).collect::<Vec<_>>().join(", ")
                ),
                ReadinessReasonCode::UnresolvedDependency => {
                    format!("{UNRESOLVED_DEPENDENCY}: {}", reason.ids.join(", "))
                }
                ReadinessReasonCode::DependencyCycle => format!(
                    "{DEPENDENCY_CYCLE}: {}",
                    reason.ids.iter().map(short).collect::<Vec<_>>().join(", ")
                ),
            };
            if reason.complete {
                text
            } else {
                format!("{text} ({INCOMPLETE})")
            }
        })
        .collect()
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
    locator: ItemLocator,
    /// `None` for a nonconforming entry.
    pub id: Option<ItemId>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub ticket_type: Option<String>,
    pub status: Option<String>,
    /// Its project and team, which the details sheet suggests (R-57).
    pub project: Option<String>,
    pub team: Option<String>,
    /// `None` for a nonconforming entry, which has no closure.
    pub lifecycle: Option<Lifecycle>,
    pub changed_at: Option<String>,
    /// The first problem of an entry with no ID.
    pub nonconforming: Option<ProblemCode>,
    pub path: String,
    pub worktree: String,
    /// Ready, blocked or closed, as the library gave it; `None` for a
    /// nonconforming entry.
    pub readiness: Option<ReadinessState>,
    /// The copy the row was read from.
    pub context: RowContext,
    /// Why it is blocked, when it is: see `blocked_reasons`. Filled when
    /// the read is applied. Empty otherwise.
    pub reasons: Vec<String>,
    /// The file, for a row that is one the library could not read (Part
    /// I). It has nothing else of a ticket's, and it does not open.
    pub unreadable: Option<UnreadableFile>,
}

impl TicketRow {
    pub(crate) fn from_item(item: &ItemDto) -> Self {
        let id = item.id.as_deref().and_then(|id| id.parse().ok());
        Self {
            locator: ItemLocator::for_item(item),
            nonconforming: match id {
                Some(_) => None,
                None => item.problems.first().map(|problem| problem.code),
            },
            id,
            slug: item.slug.clone(),
            title: item.title.clone(),
            ticket_type: item.ticket_type.clone(),
            status: item.status.clone(),
            project: item.project.clone(),
            team: item.team.clone(),
            lifecycle: item.closure.as_ref().map(|closure| match closure.state {
                ClosureState::Open => Lifecycle::Open,
                ClosureState::Closed => Lifecycle::Closed,
            }),
            changed_at: item.changed_at.clone(),
            path: item.path.clone(),
            worktree: item.context.worktree.clone(),
            readiness: item.readiness.as_ref().map(|readiness| readiness.state),
            context: RowContext::of(item),
            reasons: Vec::new(),
            unreadable: None,
        }
    }

    /// The row for a file the library could not read: its path and
    /// worktree, and nothing made up. Its address is the file's, but
    /// `open_target` gives none (R-38).
    pub(crate) fn unreadable(file: &UnreadableFile) -> Self {
        Self {
            locator: ItemLocator::Inspect {
                worktree: file.worktree.clone(),
                path: file.path.clone(),
            },
            id: None,
            slug: None,
            title: None,
            ticket_type: None,
            status: None,
            project: None,
            team: None,
            lifecycle: None,
            changed_at: None,
            nonconforming: None,
            path: file.path.clone(),
            worktree: file.worktree.clone(),
            readiness: None,
            // Nothing reads this for an unreadable row; the copy's kind and
            // branch are not known.
            context: RowContext {
                kind: if file.primary {
                    ItemContextKind::Primary
                } else {
                    ItemContextKind::Unverified
                },
                branch: None,
                worktree: file.worktree.clone(),
            },
            reasons: Vec::new(),
            unreadable: Some(file.clone()),
        }
    }

    /// The row's address in `repo`, whether or not it opens.
    pub(crate) fn target(&self, repo: &RepoKey) -> OpenTarget {
        self.locator.in_repository(repo)
    }

    /// What activating the row opens: nothing for an unreadable file,
    /// whose open would blame the whole repository (R-38).
    pub(crate) fn open_target(&self, repo: &RepoKey) -> Option<OpenTarget> {
        self.unreadable.is_none().then(|| self.target(repo))
    }

    pub(crate) fn name(&self) -> TicketName<'_> {
        TicketName {
            slug: self.slug.as_deref(),
            title: self.title.as_deref(),
        }
    }
}

/// The selected repository's tickets under the query.
#[derive(Default)]
pub(crate) struct Tickets {
    repo: Option<RepoKey>,
    /// The repository last loaded, which a clear does not reset: a load
    /// of another one is a change of repository (R-35).
    last_loaded: Option<RepoKey>,
    query: TicketQuery,
    list: KeptList<Vec<TicketRow>>,
    /// The files the last status read could not read, beside the rows
    /// under the same rules (Part I): replaced when a status read lands,
    /// kept over a busy or failed one and during a reload, dropped on a
    /// change of repository.
    unreadable: Vec<UnreadableFile>,
    view: TicketView,
    generations: Generations,
}

impl Tickets {
    pub(crate) fn repo(&self) -> Option<&RepoKey> {
        self.repo.as_ref()
    }

    pub(crate) fn query(&self) -> &TicketQuery {
        &self.query
    }

    /// The list's state, as the last read left it.
    pub(crate) fn state(&self) -> &ListState<Vec<TicketRow>> {
        self.list.state()
    }

    /// The list with the rows it kept over a reload.
    pub(crate) fn list(&self) -> &KeptList<Vec<TicketRow>> {
        &self.list
    }

    /// The rows under the query, derived whenever the read or the query
    /// changes, so it is current the moment a key is handled (R-32).
    pub(crate) fn view(&self) -> &TicketView {
        &self.view
    }

    /// The one conforming ticket the query's text matches under any
    /// lifecycle and readiness, among the rows kept (P9).
    pub(crate) fn sole_match(&self) -> Option<&TicketRow> {
        sole_match(self.list.rows()?, &self.query.text)
    }

    /// Whether a load of `selected` would be a change of repository: it is
    /// not the one last loaded, which a clear does not forget (R-35).
    pub(crate) fn is_change_to(&self, selected: Option<&RepoKey>) -> bool {
        self.last_loaded.as_ref() != selected
    }

    /// Shows the tickets `query` asks for from now on. No read is made:
    /// the one read holds every ticket.
    pub(crate) fn set_query(&mut self, query: TicketQuery) {
        self.query = query;
        self.derive();
    }

    /// Starts reading `repo`'s tickets. Reads made for an earlier load are
    /// dropped when they complete. The rows read before are kept while
    /// this one runs, unless `repo` is not the last one loaded: then they
    /// go, and the second answer is true (R-35).
    pub(crate) fn load(&mut self, repo: RepoKey) -> (Stamp, bool) {
        let changed = self.last_loaded.as_ref() != Some(&repo);
        if changed {
            self.list.clear();
            self.unreadable.clear();
        } else {
            self.list.reload();
        }
        self.drop_reads();
        self.repo = Some(repo.clone());
        self.last_loaded = Some(repo.clone());
        self.derive();
        (self.generations.next(ViewScope::Tickets(repo)), changed)
    }

    /// No repository is loaded: reads still running are dropped. The
    /// last-loaded repository is remembered.
    pub(crate) fn clear(&mut self) {
        self.drop_reads();
        self.repo = None;
        self.list.clear();
        self.unreadable.clear();
        self.derive();
    }

    fn drop_reads(&mut self) {
        if let Some(repo) = self.repo.take() {
            self.generations.next(ViewScope::Tickets(repo));
        }
    }

    /// The view of the rows, and the files that could not be read after
    /// them. Nothing is derived of files alone: with no rows there is no
    /// list to add them to.
    fn derive(&mut self) {
        let Some(rows) = self.list.rows() else {
            self.view = TicketView::default();
            return;
        };
        self.view = if self.unreadable.is_empty() {
            derive(Some(rows), &self.query)
        } else {
            let all: Vec<TicketRow> = rows
                .iter()
                .cloned()
                .chain(self.unreadable.iter().map(TicketRow::unreadable))
                .collect();
            derive(Some(&all), &self.query)
        };
    }

    fn wanted(&self, stamp: &Stamp) -> bool {
        self.generations.is_current(stamp)
            && self
                .repo
                .as_ref()
                .is_some_and(|repo| stamp.scope == ViewScope::Tickets(repo.clone()))
    }

    /// The files the last status read could not read.
    #[cfg(test)]
    pub(crate) fn unreadable(&self) -> &[UnreadableFile] {
        &self.unreadable
    }

    /// The status read completed. False when it is no longer wanted. One
    /// that gave its files replaces them; a busy or failed one keeps them,
    /// and never fails the list.
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
            self.derive();
        }
        true
    }

    /// A read completed. False when it is no longer wanted. Each blocked
    /// row carries its reasons, named from all the rows the read gave.
    pub(crate) fn apply(&mut self, stamp: Stamp, result: ReadResult<ItemListDto>) -> bool {
        if !self.wanted(&stamp) {
            return false;
        }
        self.list.apply(result, |list| {
            let by_id: HashMap<String, ItemDto> = list
                .items
                .iter()
                .filter_map(|item| Some((item.id.clone()?, item.clone())))
                .collect();
            ListState::Ready {
                rows: list
                    .items
                    .iter()
                    .map(|item| {
                        let mut row = TicketRow::from_item(item);
                        if row.readiness == Some(ReadinessState::Blocked) {
                            row.reasons = blocked_reasons(item, &by_id);
                        }
                        row
                    })
                    .collect(),
                complete: list.complete,
                index: Some(list.index),
            }
        });
        self.derive();
        true
    }
}

#[cfg(test)]
#[path = "tickets_tests.rs"]
mod tests;
