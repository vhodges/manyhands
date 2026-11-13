//! The ID read behind the filter input's Enter (design, Part D).
//!
//! Text that parses as a ULID is an item ID, and `show_item` opens it in a
//! tab, a lifecycle-closed ticket as any other, a document too. Anything
//! else the filter input holds is a filter, not a lookup: a short code is
//! a label people read, never a tab key or a mutation's input. Each read
//! moves the scope's generation on, so the result of an earlier one, or
//! one made for a repository no longer selected, is dropped when it lands.

use manyhands::{canonical::ItemId, repository::ItemDto, results::ResultCode};

use super::{
    navigation::OpenTarget,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};
use crate::desktop::services::{ReadFailure, ReadResult};

/// The trimmed text as an item ID, when it is a ULID, in either case.
pub(crate) fn parse_id(text: &str) -> Option<ItemId> {
    text.trim().to_ascii_uppercase().parse().ok()
}

/// What an ID read found.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum FindResult {
    /// The ID named an item, now shown in its tab.
    Opened,
    /// Nothing has that ID.
    None,
}

/// Where the ID read is.
#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) enum FindState {
    /// Nothing read since the selection last changed.
    #[default]
    Idle,
    /// No resolved repository is selected, so there is nothing to read.
    NoRepository,
    Searching,
    Done(FindResult),
    /// Still busy after every retry.
    Busy,
    Failed(ReadFailure),
}

#[derive(Default)]
pub(crate) struct Find {
    /// The repository the last read was made in.
    repo: Option<RepoKey>,
    state: FindState,
    generations: Generations,
}

impl Find {
    pub(crate) fn state(&self) -> &FindState {
        &self.state
    }

    /// The repository the last read was made in, while one is wanted.
    pub(crate) fn repo(&self) -> Option<&RepoKey> {
        self.repo.as_ref()
    }

    /// Starts a read in `repo`. Reads made before are dropped when they
    /// complete.
    pub(crate) fn start(&mut self, repo: RepoKey) -> Stamp {
        self.repo = Some(repo);
        self.state = FindState::Searching;
        self.generations.next(ViewScope::Find)
    }

    /// Nothing is read: a read still running is dropped, and what the last
    /// one found goes.
    pub(crate) fn clear(&mut self) {
        self.generations.next(ViewScope::Find);
        self.repo = None;
        self.state = FindState::Idle;
    }

    /// A read was asked for with no resolved repository selected: a read
    /// still running is dropped.
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

    /// The item an ID read gave. What to open, when it is still wanted
    /// and the library found it; an item the library does not find is
    /// `FindResult::None`.
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

    fn fail<T>(&mut self, result: ReadResult<T>) {
        self.state = match result {
            ReadResult::Failed(failure) => FindState::Failed(failure),
            _ => FindState::Busy,
        };
    }
}

#[cfg(test)]
#[path = "find_tests.rs"]
mod tests;
