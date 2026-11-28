//! What the polling scheduler asks of the request model (D6 Task 5).
//!
//! An automatic poll is not a request, so a result it brings in reaches the
//! views through [`Operations::brought_in`], the same `Changed` every
//! finished request leaves (Fact 17, R-10). And the desktop's own automatic
//! polls are known by operation ID, so a pending row of one is not offered
//! a Resume that can only fail (Task 4 review I-2).

use std::collections::HashSet;

use gpui_kit::Context;
use manyhands::repository::{OperationAction, OperationDto, OperationFamily, OperationId};

use super::{Operations, ResumeOffer, check_now_left_active, resume_offer};
use crate::desktop::{
    model::{list_state::ListState, stamp::RepoKey},
    services::inflight::Report,
};

impl Operations {
    /// A poll brought items into `root` (fast-forwarded or materialized,
    /// whatever the poll's end): every view of it reads again, its open
    /// tabs included, as after a finished request that changes items.
    pub(crate) fn brought_in(&mut self, root: RepoKey, cx: &mut Context<Self>) {
        self.mark_changed(root, true);
        cx.notify();
    }

    /// The operations of this desktop's own automatic polls: those it
    /// sent, and the ones its in-flight record holds. The scheduler sets
    /// them whenever either changes. True when they changed.
    pub(crate) fn set_automatic_polls(&mut self, polls: HashSet<OperationId>) -> bool {
        if self.automatic_polls == polls {
            return false;
        }
        self.automatic_polls = polls;
        true
    }

    /// The in-flight record's problem, for Activity to show (design B3,
    /// R-31). The record hands it over once; the same one is not shown
    /// twice. True when it was added.
    pub(crate) fn note_record(&mut self, report: Report) -> bool {
        if self.record_notes.contains(&report) {
            return false;
        }
        self.record_notes.push(report);
        true
    }

    /// The record problems Activity shows, oldest first.
    pub(crate) fn record_notes(&self) -> &[Report] {
        &self.record_notes
    }

    /// Hides a record problem, for the session.
    pub(crate) fn dismiss_record_note(&mut self, report: Report) {
        self.record_notes.retain(|shown| *shown != report);
    }

    /// Whether the last read of `root`'s pending operations lists a local
    /// one: a poll there answers `recovery_required` before it
    /// authenticates (Fact 32), so Unlock… has nothing to prove through it.
    /// A read not yet made, or that failed, lists none.
    pub(crate) fn has_pending_local(&self, root: &RepoKey) -> bool {
        match self.pending.get(root) {
            Some(ListState::Ready { rows, .. }) => rows
                .items
                .iter()
                .any(|operation| operation.family == OperationFamily::Local),
            _ => false,
        }
    }

    /// The operations of the desktop's own automatic polls.
    pub(crate) fn automatic_polls(&self) -> &HashSet<OperationId> {
        &self.automatic_polls
    }
}

/// What Activity says of a problem with the in-flight record.
pub(crate) fn record_note_words(report: Report) -> &'static str {
    match report {
        Report::Unreadable => {
            "Unfinished checks from an earlier session could not be read, so they were set aside."
        }
        Report::NotSaved => {
            "Unfinished checks could not be saved to disk. They are kept until you quit."
        }
    }
}

/// [`resume_offer`], except that a poll row of `own` is not offered Resume,
/// and a retirement row is offered it only when it can be resumed.
///
/// A Check now's row left active offers it (L-4); an automatic poll's row,
/// which no request holds, answers `not_resumable`, and the in-flight record
/// finishes it (L-1), so a button there is a dead end. A row of an unknown
/// or foreign operation still offers it: the read model carries no request
/// to tell them apart (a recorded known limit).
///
/// A pending retirement is resumed from the in-flight record's entry, which
/// names the operation (`entries`, R-31, E4): the library answers
/// `original_request_required` and the desktop sends the original again.
/// Without an entry (a retirement a second instance submitted, or one the
/// record lost) Resume is offered only when all the row owes is its index
/// hand-off, which needs no original request; otherwise only Abandon is
/// (K-27, Fact 26).
pub(crate) fn resume_offer_among(
    operation: &OperationDto,
    own: &HashSet<OperationId>,
    entries: &HashSet<OperationId>,
) -> ResumeOffer {
    let id = operation
        .operation_id
        .as_deref()
        .and_then(|id| OperationId::parse(id).ok());
    if id.is_some_and(|id| own.contains(&id)) && check_now_left_active(operation) {
        return ResumeOffer::NotOffered;
    }
    let offer = resume_offer(operation);
    if offer == ResumeOffer::Offered
        && operation.action == OperationAction::RetireContext
        && !id.is_some_and(|id| entries.contains(&id))
        && !owes_only_hand_off(operation)
    {
        return ResumeOffer::NotOffered;
    }
    offer
}

/// Whether a retirement's row owes nothing but its index hand-off: its work
/// is recorded as done and it has handed off to indexing, as the library
/// judges it (`LocalRow::owes_only_its_hand_off`). A resume of such a row
/// needs no original request.
fn owes_only_hand_off(operation: &OperationDto) -> bool {
    matches!(
        operation.completed_step.as_deref(),
        Some(
            "context_branch_released"
                | "context_retirement_absent"
                | "context_retirement_preserved"
        )
    ) && matches!(
        operation.state.as_str(),
        "authoritative_observed" | "indexing" | "index_invalidated" | "failed" | "retry"
    )
}
