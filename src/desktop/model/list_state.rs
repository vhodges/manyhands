//! What a list shows: loading, its rows, busy or failed.
//!
//! Every state carries a text label, so none is shown by color alone. An
//! empty list is `Ready` with no rows and is labeled differently from a
//! list whose index was never refreshed. An unavailable index is
//! `Failed` with `index_unavailable`.

use manyhands::{
    repository::{IndexState, IndexStateDto},
    results::ResultCode,
};

use super::navigation::NotReloaded;
use crate::desktop::services::{ReadFailure, ReadResult};

pub(crate) const LABEL_LOADING: &str = "Loading";
pub(crate) const LABEL_READY: &str = "Up to date";
pub(crate) const LABEL_EMPTY: &str = "Nothing to show";
pub(crate) const LABEL_NEVER_REFRESHED: &str = "Index never refreshed";
pub(crate) const LABEL_STALE: &str = "Index out of date";
pub(crate) const LABEL_CURRENT: &str = "Index current";
pub(crate) const LABEL_INCOMPLETE: &str = "List incomplete";
pub(crate) const LABEL_BUSY: &str = "Busy";
pub(crate) const LABEL_FAILED: &str = "Could not load";
pub(crate) const LABEL_INDEX_UNAVAILABLE: &str = "Index unavailable";

/// Rows a list can say it has none of.
pub(crate) trait Rows {
    fn is_empty(&self) -> bool;
}

impl<T> Rows for Vec<T> {
    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ListState<T> {
    Loading,
    /// `index` is the state of the index the rows came from; `None` for a
    /// list that is not of one repository's index, such as the repository
    /// list itself.
    Ready {
        rows: T,
        complete: bool,
        index: Option<IndexStateDto>,
    },
    /// Still busy after every retry.
    Busy,
    Failed(ReadFailure),
}

impl<T> ListState<T> {
    /// The rows, when the list is ready.
    pub(crate) fn rows(&self) -> Option<&T> {
        match self {
            Self::Ready { rows, .. } => Some(rows),
            _ => None,
        }
    }

    /// A finished read as a list state: `ready` makes the ready state
    /// from the read's value.
    pub(crate) fn from_read<V>(result: ReadResult<V>, ready: impl FnOnce(V) -> Self) -> Self {
        match result {
            ReadResult::Ok(value) => ready(value),
            ReadResult::Busy => Self::Busy,
            ReadResult::Failed(failure) => Self::Failed(failure),
        }
    }

    /// Applies a finished read again over what is shown. One that gave its
    /// value replaces the state. One that stayed busy while rows are shown
    /// keeps them and says so, since busy passes. Any other, a failure
    /// included, replaces the state with its answer, as a first read does
    /// (ruling R-B5).
    pub(crate) fn apply_keeping<V>(
        &mut self,
        result: ReadResult<V>,
        ready: impl FnOnce(V) -> Self,
    ) -> Option<NotReloaded> {
        match result {
            ReadResult::Ok(value) => {
                *self = ready(value);
                None
            }
            ReadResult::Busy if self.rows().is_some() => Some(NotReloaded::Busy),
            ReadResult::Busy => {
                *self = Self::Busy;
                None
            }
            ReadResult::Failed(failure) => {
                *self = Self::Failed(failure);
                None
            }
        }
    }
}

impl<T: Rows> ListState<T> {
    /// The list's state in fixed text. A never-refreshed or stale index
    /// says so before anything about the rows.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Loading => LABEL_LOADING,
            Self::Busy => LABEL_BUSY,
            Self::Failed(failure) if failure.code == ResultCode::IndexUnavailable => {
                LABEL_INDEX_UNAVAILABLE
            }
            Self::Failed(_) => LABEL_FAILED,
            Self::Ready {
                rows,
                complete,
                index,
            } => match index.as_ref().map(|index| &index.state) {
                Some(IndexState::NeverRefreshed) => LABEL_NEVER_REFRESHED,
                Some(IndexState::Stale) => LABEL_STALE,
                _ if !complete => LABEL_INCOMPLETE,
                _ if rows.is_empty() => LABEL_EMPTY,
                _ => LABEL_READY,
            },
        }
    }
}

/// A list's state with the rows of its last ready read kept (ruling R-17).
///
/// A reload shows the kept rows until it lands; a read that stays busy
/// over kept rows keeps them and says so; a failure replaces them, as D1's
/// ruling R-B5 says. `clear` drops them, as a change of repository does.
#[derive(Clone, Debug)]
pub(crate) struct KeptList<T> {
    state: ListState<T>,
    /// The last `Ready` state, while the list is `Loading` over it.
    kept: Option<ListState<T>>,
    busy_over_kept: bool,
}

impl<T> Default for KeptList<T> {
    fn default() -> Self {
        Self {
            state: ListState::Loading,
            kept: None,
            busy_over_kept: false,
        }
    }
}

impl<T> KeptList<T> {
    /// Loading again, keeping the ready rows. A list that has none keeps
    /// none.
    pub(crate) fn reload(&mut self) {
        self.busy_over_kept = false;
        if matches!(self.state, ListState::Ready { .. }) {
            self.kept = Some(std::mem::replace(&mut self.state, ListState::Loading));
        } else {
            self.state = ListState::Loading;
            // A `Loading` list keeps what it kept; any other state had none.
        }
    }

    /// Loading with nothing kept.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Applies a finished state. `Ready` replaces. `Busy` over kept rows
    /// keeps them; `Busy` with none, and `Failed`, replace.
    pub(crate) fn apply_state(&mut self, state: ListState<T>) {
        match state {
            ListState::Busy => {
                if let Some(kept) = self.kept.take() {
                    self.state = kept;
                    self.busy_over_kept = true;
                } else if matches!(self.state, ListState::Ready { .. }) {
                    self.busy_over_kept = true;
                } else {
                    self.state = ListState::Busy;
                    self.busy_over_kept = false;
                }
            }
            state => {
                self.state = state;
                self.kept = None;
                self.busy_over_kept = false;
            }
        }
    }

    /// Applies a finished read through `apply_state`.
    pub(crate) fn apply<V>(
        &mut self,
        result: ReadResult<V>,
        ready: impl FnOnce(V) -> ListState<T>,
    ) {
        self.apply_state(ListState::from_read(result, ready));
    }

    pub(crate) fn state(&self) -> &ListState<T> {
        &self.state
    }

    /// The ready rows, or the kept rows while loading over them.
    pub(crate) fn rows(&self) -> Option<&T> {
        self.state
            .rows()
            .or_else(|| self.kept.as_ref().and_then(ListState::rows))
    }

    /// Changes the ready rows, or the kept rows while loading over them:
    /// what `rows` would give.
    pub(crate) fn map_rows(&mut self, change: impl FnOnce(&mut T)) {
        let state = match &mut self.state {
            ListState::Ready { .. } => Some(&mut self.state),
            _ => self.kept.as_mut(),
        };
        if let Some(ListState::Ready { rows, .. }) = state {
            change(rows);
        }
    }

    /// Loading over kept rows.
    pub(crate) fn reloading(&self) -> bool {
        matches!(self.state, ListState::Loading) && self.kept.is_some()
    }

    /// The last read stayed busy and the rows shown are the earlier ones.
    pub(crate) fn busy_over_kept(&self) -> bool {
        self.busy_over_kept
    }
}

/// An index state in fixed text.
pub(crate) fn index_label(index: &IndexStateDto) -> &'static str {
    match index.state {
        IndexState::Current => LABEL_CURRENT,
        IndexState::Stale => LABEL_STALE,
        IndexState::NeverRefreshed => LABEL_NEVER_REFRESHED,
    }
}

#[cfg(test)]
#[path = "list_state_tests.rs"]
mod tests;
