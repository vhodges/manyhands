//! What the polling scheduler asks of the request model (D6 Task 5).
//!
//! An automatic poll is not a request, so a result it brings in reaches the
//! views through [`Operations::brought_in`], the same `Changed` every
//! finished request leaves (Fact 17, R-10). And the desktop's own automatic
//! polls are known by operation ID, so a pending row of one is not offered
//! a Resume that can only fail (Task 4 review I-2).

use std::collections::HashSet;

use gpui_kit::Context;
use manyhands::repository::{OperationDto, OperationFamily, OperationId};

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

/// [`resume_offer`], except that a poll row of `own` is not offered Resume.
///
/// A Check now's row left active offers it (L-4); an automatic poll's row,
/// which no request holds, answers `not_resumable`, and the in-flight record
/// finishes it (L-1), so a button there is a dead end. A row of an unknown,
/// foreign or retirement-observer operation still offers it: the read model
/// carries no request or kind to tell them apart (a recorded known limit).
pub(crate) fn resume_offer_among(
    operation: &OperationDto,
    own: &HashSet<OperationId>,
) -> ResumeOffer {
    let ours = operation
        .operation_id
        .as_deref()
        .and_then(|id| OperationId::parse(id).ok())
        .is_some_and(|id| own.contains(&id));
    if ours && check_now_left_active(operation) {
        return ResumeOffer::NotOffered;
    }
    resume_offer(operation)
}
