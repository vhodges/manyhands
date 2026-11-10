//! What awaits the person (Wave 04 Cycle 01, decisions 3 and 14, R-13):
//! the count the footer's Needs attention shows.
//!
//! It reads the existing models and changes none of them.

use std::collections::HashSet;

use manyhands::results::Outcome;

use crate::desktop::{
    model::{list_state::ListState, operations::Operations, polling::PollSchedule},
    services::{drafts::DraftRegistry, requests::RequestState},
};

/// How many things await the person:
///
/// - each request that finished `Partial`, `Blocked` or `Error` and was
///   not dismissed (`Cancelled`, `Success`, `Noop` and a request that was
///   never sent are not counted);
/// - each draft whose last write failed;
/// - one when the draft store is unavailable;
/// - each unfinished operation the pending lists show;
/// - one for each repository whose pending list failed to load (decision
///   14). A read that is loading or busy is not a failure;
/// - one for each entry the polling model lists (decision 11, R-14): a key,
///   a host or a repository that holds checking back, however many
///   repositories it holds back. Paused, off, unreachable and waiting
///   repositories are not listed.
///
/// Three rules keep one thing from counting twice (R-23, W-1): requests and
/// pending rows are grouped by operation ID, so a request and the pending
/// row of its operation are one, and so are several requests of one
/// operation; a request whose answer became attention or a retry was
/// dismissed when it was answered (`Operations::apply`); and the worker's
/// running operation is not a pending row (`Operations::is_running`).
pub(crate) fn needs_attention(
    operations: &Operations,
    drafts: &DraftRegistry,
    polling: &PollSchedule,
) -> usize {
    // The operations the counted requests are about, once each; a request
    // whose answer names none counts alone.
    let mut about: HashSet<String> = HashSet::new();
    let mut alone = 0;
    for request in operations.requests().iter().filter(|request| {
        matches!(
            request.state,
            RequestState::Finished(Outcome::Partial | Outcome::Blocked | Outcome::Error, ..)
        )
    }) {
        match operations.operation_of(request) {
            Some(operation) => {
                about.insert(operation);
            }
            None => alone += 1,
        }
    }
    let mut pending = 0;
    for list in operations.pending().values() {
        match list {
            ListState::Ready { rows, .. } => {
                for row in rows.items.iter().filter(|row| !operations.is_running(row)) {
                    pending += 1;
                    // The row is the request's operation, shown: one thing.
                    if let Some(operation) = &row.operation_id {
                        about.remove(operation);
                    }
                }
            }
            ListState::Failed(_) => pending += 1,
            ListState::Loading | ListState::Busy => {}
        }
    }
    alone
        + about.len()
        + drafts.flush_failures().len()
        + usize::from(drafts.unavailable().is_some())
        + pending
        + polling.attention().len()
}

#[cfg(test)]
#[path = "attention_tests.rs"]
mod tests;
