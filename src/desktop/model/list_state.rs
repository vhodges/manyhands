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
