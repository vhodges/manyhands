//! Navigation: the repository list, the selected repository and the
//! active pane.
//!
//! An entry's availability comes only from resolving it, on selection
//! and on Retry, never in a loop and never from the accessibility the
//! index stored (defect 01M4EHGE2K).

use std::path::Path;

use manyhands::{
    canonical::ItemId,
    repository::{
        ItemDto, ItemDtoKind, RepositoryListDto, RepositorySummaryDto, ResolvedRepository,
    },
    results::ResultCode,
};

use super::{
    list_state::ListState,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};
use crate::desktop::services::{ReadFailure, ReadResult};

/// The resolution failures that make an entry Unavailable, with Retry
/// and Remove.
pub(crate) const UNAVAILABLE_CODES: [ResultCode; 4] = [
    ResultCode::InvalidPath,
    ResultCode::NotRepository,
    ResultCode::RepositoryInaccessible,
    ResultCode::RepositoryNotRegistered,
];

/// What resolving an entry last gave.
#[derive(Clone, Debug)]
pub(crate) enum Resolution {
    /// Not resolved since it was listed, selected or retried.
    Unknown,
    Resolved(ResolvedRepository),
    /// The repository cannot be found or read where it is registered, or
    /// is no longer registered.
    Unavailable(ResultCode),
    /// Still busy after every retry.
    Busy,
    /// Another failure, shown by its code with Retry.
    Failed(ReadFailure),
}

impl Resolution {
    fn from_read(result: ReadResult<ResolvedRepository>) -> Self {
        match result {
            ReadResult::Ok(resolved) => Self::Resolved(resolved),
            ReadResult::Busy => Self::Busy,
            ReadResult::Failed(failure) if UNAVAILABLE_CODES.contains(&failure.code) => {
                Self::Unavailable(failure.code)
            }
            ReadResult::Failed(failure) => Self::Failed(failure),
        }
    }
}

/// One registration and what resolving it gave.
#[derive(Clone, Debug)]
pub(crate) struct RepositoryEntry {
    pub summary: RepositorySummaryDto,
    pub resolution: Resolution,
}

impl RepositoryEntry {
    /// The root, as the list returned it.
    pub(crate) fn key(&self) -> &RepoKey {
        &self.summary.root
    }

    /// The root's directory name, or the root when it has none.
    pub(crate) fn name(&self) -> String {
        Path::new(&self.summary.root)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.summary.root.clone())
    }
}

/// What activating a row opens: an item by its ID, or a nonconforming
/// entry, which has none, by its path in its context (ruling R4).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum OpenTarget {
    Item {
        repo: RepoKey,
        kind: ItemDtoKind,
        id: ItemId,
    },
    Inspect {
        repo: RepoKey,
        worktree: String,
        path: String,
    },
}

impl OpenTarget {
    /// What `item`, listed in `repo`, opens as: by ID when it has a
    /// well-formed one, otherwise by its path.
    pub(crate) fn for_item(repo: &RepoKey, item: &ItemDto) -> Self {
        match item.id.as_deref().map(str::parse::<ItemId>) {
            Some(Ok(id)) => Self::Item {
                repo: repo.clone(),
                kind: item.kind,
                id,
            },
            _ => Self::Inspect {
                repo: repo.clone(),
                worktree: item.context.worktree.clone(),
                path: item.path.clone(),
            },
        }
    }
}

/// The navigation region's pane.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Pane {
    #[default]
    Documents,
    Tickets,
}

pub(crate) struct Navigation {
    repositories: ListState<Vec<RepositoryEntry>>,
    selected: Option<RepoKey>,
    /// The selected entry, when the last list read did not give it: kept,
    /// out of the list, so its resolution can still say why it is gone.
    /// Selecting another entry lets it go.
    kept: Option<RepositoryEntry>,
    /// The last list read did not give the selected entry, which the one
    /// before it did: it is to be resolved again.
    vanished: bool,
    pane: Pane,
    generations: Generations,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            repositories: ListState::Loading,
            selected: None,
            kept: None,
            vanished: false,
            pane: Pane::default(),
            generations: Generations::default(),
        }
    }
}

impl Navigation {
    pub(crate) fn repositories(&self) -> &ListState<Vec<RepositoryEntry>> {
        &self.repositories
    }

    pub(crate) fn selected(&self) -> Option<&RepoKey> {
        self.selected.as_ref()
    }

    pub(crate) fn pane(&self) -> Pane {
        self.pane
    }

    pub(crate) fn set_pane(&mut self, pane: Pane) {
        self.pane = pane;
    }

    /// The listed entry for `key`, or the kept selection.
    pub(crate) fn entry(&self, key: &str) -> Option<&RepositoryEntry> {
        self.repositories
            .rows()
            .and_then(|rows| rows.iter().find(|entry| entry.key() == key))
            .or(self.kept.as_ref().filter(|entry| entry.key() == key))
    }

    pub(crate) fn selected_entry(&self) -> Option<&RepositoryEntry> {
        self.entry(self.selected.as_deref()?)
    }

    fn entry_mut(&mut self, key: &str) -> Option<&mut RepositoryEntry> {
        let listed = match &mut self.repositories {
            ListState::Ready { rows, .. } => rows.iter_mut().find(|entry| entry.key() == key),
            _ => None,
        };
        listed.or(self.kept.as_mut().filter(|entry| entry.key() == key))
    }

    /// Starts reading the list; any earlier list read is dropped when it
    /// completes.
    pub(crate) fn load_repositories(&mut self) -> Stamp {
        self.generations.next(ViewScope::Repositories)
    }

    /// A list read completed. Each entry still listed keeps what it last
    /// resolved to. A selection the read did not give is kept out of the
    /// list, or cleared when there is no entry to keep. One the read
    /// before gave is no longer known to resolve: it is reset to Unknown,
    /// and `take_vanished` gives it to be resolved again. False when the
    /// read is no longer wanted.
    pub(crate) fn apply_repositories(
        &mut self,
        stamp: Stamp,
        result: ReadResult<RepositoryListDto>,
    ) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        let listed_before = self.selected.as_ref().is_some_and(|key| {
            self.repositories
                .rows()
                .is_some_and(|rows| rows.iter().any(|entry| entry.key() == key))
        });
        let previous = std::mem::replace(&mut self.repositories, ListState::Loading);
        let mut previous = match previous {
            ListState::Ready { rows, .. } => rows,
            _ => Vec::new(),
        };
        previous.extend(self.kept.take());
        self.repositories = ListState::from_read(result, |list| ListState::Ready {
            rows: list
                .items
                .into_iter()
                .map(|summary| {
                    let resolution = previous
                        .iter()
                        .position(|entry| entry.summary.root == summary.root)
                        .map(|at| previous.swap_remove(at).resolution)
                        .unwrap_or(Resolution::Unknown);
                    RepositoryEntry {
                        summary,
                        resolution,
                    }
                })
                .collect(),
            complete: list.complete,
            index: None,
        });
        if let Some(key) = self.selected.clone()
            && self.entry(&key).is_none()
        {
            self.kept = previous.into_iter().find(|entry| *entry.key() == key);
            if let Some(kept) = self.kept.as_mut()
                && listed_before
            {
                kept.resolution = Resolution::Unknown;
                self.vanished = true;
            }
            if self.kept.is_none() {
                self.selected = None;
                // A resolution still running is for an entry not here.
                self.generations.next(ViewScope::Selection);
            }
        }
        true
    }

    /// The selected entry, when the last list read let it go from the list
    /// and it is to be resolved again; once.
    pub(crate) fn take_vanished(&mut self) -> Option<RepoKey> {
        if !std::mem::take(&mut self.vanished) {
            return None;
        }
        self.selected.clone()
    }

    /// Selects `key` and starts resolving it. A resolution started for an
    /// earlier selection is dropped when it completes.
    pub(crate) fn select(&mut self, key: RepoKey) -> Stamp {
        if self.kept.as_ref().is_some_and(|kept| *kept.key() != key) {
            self.kept = None;
        }
        if let Some(entry) = self.entry_mut(&key) {
            entry.resolution = Resolution::Unknown;
        }
        // Selecting resolves: nothing is left to resolve again.
        self.vanished = false;
        self.selected = Some(key);
        self.generations.next(ViewScope::Selection)
    }

    /// Resolves `key` again: Retry. It is selected as well.
    pub(crate) fn retry_resolution(&mut self, key: RepoKey) -> Stamp {
        self.select(key)
    }

    /// The selection's resolution completed. False when it is no longer
    /// wanted.
    pub(crate) fn apply_resolution(
        &mut self,
        stamp: Stamp,
        result: ReadResult<ResolvedRepository>,
    ) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        let Some(key) = self.selected.clone() else {
            return false;
        };
        let Some(entry) = self.entry_mut(&key) else {
            return false;
        };
        entry.resolution = Resolution::from_read(result);
        true
    }
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;
