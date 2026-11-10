//! What awaits the person (Wave 04 Cycle 01, decisions 3 and 14, R-13):
//! the count the footer's Needs attention shows.
//!
//! It reads the existing models and changes none of them.

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
pub(crate) fn needs_attention(
    operations: &Operations,
    drafts: &DraftRegistry,
    polling: &PollSchedule,
) -> usize {
    let requests = operations
        .requests()
        .iter()
        .filter(|request| {
            matches!(
                request.state,
                RequestState::Finished(Outcome::Partial | Outcome::Blocked | Outcome::Error, ..)
            )
        })
        .count();
    let pending: usize = operations
        .pending()
        .values()
        .map(|list| match list {
            ListState::Ready { rows, .. } => rows.items.len(),
            ListState::Failed(_) => 1,
            ListState::Loading | ListState::Busy => 0,
        })
        .sum();
    requests
        + drafts.flush_failures().len()
        + usize::from(drafts.unavailable().is_some())
        + pending
        + polling.attention().len()
}

#[cfg(test)]
#[path = "attention_tests.rs"]
mod tests;
