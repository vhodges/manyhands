//! Which repositories Check all takes (Part E, R-13): a pure function of the
//! facts a row is made from, so the toolbar's enablement and its press say
//! the same, and the menu's Check now and the state agree on what can check.

use manyhands::repository::PollingOutcome;

use super::{
    super::polling::{
        Phase, Reason,
        status::{self, Shape},
    },
    RepoFacts,
};

/// Whether Check all sends a `PollOnce` for this repository: D6's shape is
/// `Polling` (paused included, decision 16), nothing is checking, no live
/// attention waits on a key or a host, and a stored outcome that still
/// applies (`status::stored_applies`) does not say a key or a host is
/// wanted. The sentences and the actions are the row's; this only decides.
pub(crate) fn check_all_applies(facts: &RepoFacts<'_>) -> bool {
    if !matches!(facts.row().shape, Shape::Polling(_)) || facts.checking() {
        return false;
    }
    let waiting_on_key_or_host = matches!(
        facts.schedule.map(|schedule| &schedule.phase),
        Some(Phase::Attention(
            Reason::KeyLocked(_)
                | Reason::KeyProblem(_)
                | Reason::NoKeySelected(_)
                | Reason::HostUnapproved(_)
                | Reason::HostUnidentified(_)
        ))
    );
    let stored = status::stored_applies(&facts.polling_facts(&facts.entry.name()));
    !waiting_on_key_or_host
        && !matches!(
            stored,
            Some(
                PollingOutcome::UnlockRequired
                    | PollingOutcome::SelectedKeyUnavailable
                    | PollingOutcome::HostApprovalRequired
            )
        )
}
