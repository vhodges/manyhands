//! Find: one input, read as a ULID or a short code (design, Discovery /
//! Find).
//!
//! Text that parses as a ULID is an item ID, and `show_item` opens it in a
//! tab, a lifecycle-closed ticket as any other. Anything else not blank is
//! a short code: `find_tickets_by_slug` lists every ticket carrying it, and
//! choosing one opens it by its ID. A short code is a label people read
//! and never identifies anything: it is never a tab key or a mutation's
//! input. Each search moves the scope's generation on, so the result of an
//! earlier one, or one made for a repository no longer selected, is
//! dropped when it lands.

use manyhands::{
    canonical::ItemId,
    repository::{ItemDto, ItemListDto},
    results::ResultCode,
};

use super::{
    navigation::OpenTarget,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};
use crate::desktop::services::{ReadFailure, ReadResult};

/// What the Find input's text asks for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum FindQuery {
    Id(ItemId),
    Slug(String),
}

impl FindQuery {
    /// The trimmed text as a ULID when it is one, in either case, and
    /// otherwise as a short code. Blank text asks for nothing.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        Some(match text.to_ascii_uppercase().parse::<ItemId>() {
            Ok(id) => Self::Id(id),
            Err(_) => Self::Slug(text.to_owned()),
        })
    }
}

/// What a search found.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum FindResult {
    /// The ID named an item, now shown in its tab.
    Opened,
    /// The tickets carrying the short code, in the library's order, for the
    /// user to choose between.
    Matches(Vec<ItemDto>),
    /// Nothing has that ID or short code.
    None,
}

/// Where Find is.
#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) enum FindState {
    /// Nothing searched since the selection last changed.
    #[default]
    Idle,
    /// No resolved repository is selected, so there is nothing to search.
    NoRepository,
    Searching,
    Done(FindResult),
    /// Still busy after every retry.
    Busy,
    Failed(ReadFailure),
}

#[derive(Default)]
pub(crate) struct Find {
    /// The repository the last search was made in.
    repo: Option<RepoKey>,
    state: FindState,
    generations: Generations,
}

impl Find {
    pub(crate) fn state(&self) -> &FindState {
        &self.state
    }

    /// Starts a search in `repo`. Searches made before are dropped when
    /// they complete.
    pub(crate) fn start(&mut self, repo: RepoKey) -> Stamp {
        self.repo = Some(repo);
        self.state = FindState::Searching;
        self.generations.next(ViewScope::Find)
    }

    /// Nothing is searched: a search still running is dropped, and what the
    /// last one found goes.
    pub(crate) fn clear(&mut self) {
        self.generations.next(ViewScope::Find);
        self.repo = None;
        self.state = FindState::Idle;
    }

    /// A search was asked for with no resolved repository selected: a
    /// search still running is dropped.
    pub(crate) fn no_repository(&mut self) {
        self.clear();
        self.state = FindState::NoRepository;
    }

    /// The repository of a result that is still wanted.
    fn wanted(&self, stamp: &Stamp) -> Option<RepoKey> {
        self.generations
            .is_current(stamp)
            .then(|| self.repo.clone())
            .flatten()
    }

    /// The item an ID search read. What to open, when it is still wanted
    /// and the library found it; an item the library does not find is no
    /// match.
    pub(crate) fn apply_item(
        &mut self,
        stamp: Stamp,
        result: ReadResult<ItemDto>,
    ) -> Option<OpenTarget> {
        let repo = self.wanted(&stamp)?;
        match result {
            ReadResult::Ok(item) => {
                self.state = FindState::Done(FindResult::Opened);
                Some(OpenTarget::for_item(&repo, &item))
            }
            ReadResult::Failed(failure) if failure.code == ResultCode::ItemNotFound => {
                self.state = FindState::Done(FindResult::None);
                None
            }
            other => {
                self.fail(other);
                None
            }
        }
    }

    /// The tickets a short-code search found. False when it is no longer
    /// wanted.
    pub(crate) fn apply_matches(&mut self, stamp: Stamp, result: ReadResult<ItemListDto>) -> bool {
        if self.wanted(&stamp).is_none() {
            return false;
        }
        match result {
            ReadResult::Ok(list) if list.items.is_empty() => {
                self.state = FindState::Done(FindResult::None);
            }
            ReadResult::Ok(list) => {
                self.state = FindState::Done(FindResult::Matches(list.items));
            }
            other => self.fail(other),
        }
        true
    }

    fn fail<T>(&mut self, result: ReadResult<T>) {
        self.state = match result {
            ReadResult::Failed(failure) => FindState::Failed(failure),
            _ => FindState::Busy,
        };
    }

    /// What choosing a match opens: the ticket by its ID, in the
    /// repository it was found in. Never by its short code.
    pub(crate) fn target(&self, item: &ItemDto) -> Option<OpenTarget> {
        let repo = self.repo.as_ref()?;
        match OpenTarget::for_item(repo, item) {
            target @ OpenTarget::Item { .. } => Some(target),
            OpenTarget::Inspect { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "find_tests.rs"]
mod tests;
