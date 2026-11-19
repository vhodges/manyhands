//! What a list panel's body shows (Part A): a pure function of a list's
//! state, so each state has its own text and none is shown by colour.

use manyhands::{
    repository::{ClosureFilter, IndexState},
    results::ResultCode,
};

use super::list_state::{KeptList, ListState, Rows};

pub(crate) const SELECT_A_REPOSITORY: &str = "Select a repository.";
pub(crate) const LOADING_TICKETS: &str = "Loading tickets";
pub(crate) const LOADING_DOCUMENTS: &str = "Loading documents";
pub(crate) const REFRESH: &str = "Refresh";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const REBUILD_INDEX: &str = "Rebuild the search index";
pub(crate) const CLEAR_FILTERS: &str = "Clear filters";
pub(crate) const SHOW_ALL: &str = "Show all";
pub(crate) const NEW_TICKET: &str = "New ticket\u{2026}";
pub(crate) const NEW_DOCUMENT: &str = "New document\u{2026}";

/// Which panel a body is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    Tickets,
    Documents,
}

impl ListKind {
    fn plural(self) -> &'static str {
        match self {
            Self::Tickets => "tickets",
            Self::Documents => "documents",
        }
    }
}

/// The accessible name of a first load's skeleton rows.
pub(crate) fn loading_text(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Tickets => LOADING_TICKETS,
        ListKind::Documents => LOADING_DOCUMENTS,
    }
}

/// What an alert's or an empty state's button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListAction {
    /// Refresh the search index (`maintain_index`).
    Refresh,
    /// Read the list again without clearing it.
    Retry,
    RebuildIndex,
    ClearFilters,
    ShowAll,
    /// New ticket… or New document…, by the panel.
    NewItem,
}

impl ListAction {
    /// The button's text. Only `NewItem` opens a dialog, so only it ends
    /// in an ellipsis.
    pub(crate) fn label(self, kind: ListKind) -> &'static str {
        match self {
            Self::Refresh => REFRESH,
            Self::Retry => RETRY,
            Self::RebuildIndex => REBUILD_INDEX,
            Self::ClearFilters => CLEAR_FILTERS,
            Self::ShowAll => SHOW_ALL,
            Self::NewItem => match kind {
                ListKind::Tickets => NEW_TICKET,
                ListKind::Documents => NEW_DOCUMENT,
            },
        }
    }
}

/// Amber for a list that may be wrong, red for one that could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AlertTone {
    Warning,
    Failure,
}

/// The alert row: one line, a second for a failure's message, and one
/// action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Alert {
    pub tone: AlertTone,
    pub text: String,
    pub detail: Option<String>,
    pub action: ListAction,
}

/// What the panel body draws above its list, which stays drawn in every
/// state (R-34).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ListBody {
    NoRepository,
    Skeleton,
    /// The rows, with the alert above them if there is one.
    Rows {
        alert: Option<Alert>,
    },
    Empty {
        text: String,
        action: Option<ListAction>,
    },
    /// An alert and no rows.
    Problem(Alert),
}

/// What `list_body` reads besides the list's state. `closure`,
/// `elsewhere` and `narrowed` are ignored for Documents, which has no
/// filter.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BodyInput<'a> {
    pub kind: ListKind,
    pub repo: Option<&'a str>,
    /// The rows the panel would draw: conforming, nonconforming and
    /// unreadable rows in Tickets; every node in Documents.
    pub listed: usize,
    /// Tickets that match under other filters.
    pub elsewhere: usize,
    pub closure: ClosureFilter,
    /// A filter text or readiness narrows the list.
    pub narrowed: bool,
}

pub(crate) fn busy_over_kept_text(repo: &str) -> String {
    format!("{repo} was busy, so this list wasn't updated.")
}

pub(crate) fn never_refreshed_text(repo: &str) -> String {
    format!("{repo} hasn't been scanned yet, so this list may be empty.")
}

pub(crate) fn stale_text(repo: &str) -> String {
    format!("The list for {repo} may be out of date.")
}

pub(crate) fn incomplete_text(repo: &str) -> String {
    format!("A folder in {repo} couldn't be read, so this list may be missing items.")
}

pub(crate) fn busy_text(repo: &str) -> String {
    format!("{repo} is busy right now.")
}

pub(crate) fn failed_text(kind: ListKind, repo: &str) -> String {
    format!(
        "Couldn't read the {} in {repo}. Nothing was changed.",
        kind.plural()
    )
}

pub(crate) fn index_unavailable_text(repo: &str) -> String {
    format!("The search index for {repo} can't be read. Nothing was changed.")
}

/// The empty state of a list that no filter narrows.
pub(crate) fn empty_text(kind: ListKind, closure: ClosureFilter, repo: &str) -> String {
    match (kind, closure) {
        (ListKind::Documents, _) => format!("No documents in {repo} yet."),
        (ListKind::Tickets, ClosureFilter::Open) => format!("No open tickets in {repo}."),
        (ListKind::Tickets, ClosureFilter::Closed) => format!("No closed tickets in {repo}."),
        (ListKind::Tickets, ClosureFilter::All) => format!("No tickets in {repo} yet."),
    }
}

/// A narrowed empty ticket list with no match under other filters.
pub(crate) const NO_TICKETS_MATCH: &str = "No tickets match these filters.";

/// A narrowed empty ticket list that matches under other filters.
pub(crate) fn matches_elsewhere_text(elsewhere: usize) -> String {
    if elsewhere == 1 {
        "No tickets here match. 1 more ticket matches with other filters.".to_string()
    } else {
        format!("No tickets here match. {elsewhere} more tickets match with other filters.")
    }
}

fn alert(tone: AlertTone, text: String, action: ListAction) -> Alert {
    Alert {
        tone,
        text,
        detail: None,
        action,
    }
}

/// The body a list's state gives. Busy over kept rows comes first; then
/// never refreshed, stale and incomplete, in that order, as
/// `ListState::label` does. One alert at a time.
pub(crate) fn list_body<T: Rows>(list: &KeptList<T>, input: BodyInput<'_>) -> ListBody {
    let Some(repo) = input.repo else {
        return ListBody::NoRepository;
    };
    let kind = input.kind;
    match list.state() {
        ListState::Loading if list.reloading() => ListBody::Rows { alert: None },
        ListState::Loading => ListBody::Skeleton,
        ListState::Busy => ListBody::Problem(alert(
            AlertTone::Warning,
            busy_text(repo),
            ListAction::Retry,
        )),
        ListState::Failed(failure) if failure.code == ResultCode::IndexUnavailable => {
            ListBody::Problem(alert(
                AlertTone::Failure,
                index_unavailable_text(repo),
                ListAction::RebuildIndex,
            ))
        }
        ListState::Failed(failure) => ListBody::Problem(Alert {
            detail: Some(failure.code.message().to_string()),
            ..alert(
                AlertTone::Failure,
                failed_text(kind, repo),
                ListAction::Retry,
            )
        }),
        ListState::Ready {
            complete, index, ..
        } => {
            let warning = if list.busy_over_kept() {
                Some(alert(
                    AlertTone::Warning,
                    busy_over_kept_text(repo),
                    ListAction::Retry,
                ))
            } else {
                let text = match index.as_ref().map(|index| &index.state) {
                    Some(IndexState::NeverRefreshed) => Some(never_refreshed_text(repo)),
                    Some(IndexState::Stale) => Some(stale_text(repo)),
                    _ if !complete => Some(incomplete_text(repo)),
                    _ => None,
                };
                text.map(|text| alert(AlertTone::Warning, text, ListAction::Refresh))
            };
            if warning.is_some() || input.listed > 0 {
                return ListBody::Rows { alert: warning };
            }
            empty_body(input, repo)
        }
    }
}

/// A ready list with nothing to draw and nothing to warn of.
fn empty_body(input: BodyInput<'_>, repo: &str) -> ListBody {
    let kind = input.kind;
    // Documents have no filter to narrow them.
    if !(input.narrowed && kind == ListKind::Tickets) {
        let action = match (kind, input.closure) {
            (ListKind::Tickets, ClosureFilter::Closed) => None,
            _ => Some(ListAction::NewItem),
        };
        return ListBody::Empty {
            text: empty_text(kind, input.closure, repo),
            action,
        };
    }
    if input.elsewhere == 0 {
        ListBody::Empty {
            text: NO_TICKETS_MATCH.to_string(),
            action: Some(ListAction::ClearFilters),
        }
    } else {
        ListBody::Empty {
            text: matches_elsewhere_text(input.elsewhere),
            action: Some(ListAction::ShowAll),
        }
    }
}

#[cfg(test)]
#[path = "list_view_tests.rs"]
mod tests;
