//! What the polling scheduler asks of the request model (D6 Task 5).
//!
//! An automatic poll is not a request, so a result it brings in reaches the
//! views through [`Operations::brought_in`], the same `Changed` every
//! finished request leaves (Fact 17, R-10). And the desktop's own automatic
//! polls are known by operation ID, so a pending row of one is not offered
//! a Resume that can only fail (Task 4 review I-2).

use std::collections::HashSet;

use gpui_kit::Context;
use manyhands::repository::{OperationDto, OperationId};

use super::{Operations, ResumeOffer, check_now_left_active, resume_offer};
use crate::desktop::model::stamp::RepoKey;

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

    /// The operations of the desktop's own automatic polls.
    pub(crate) fn automatic_polls(&self) -> &HashSet<OperationId> {
        &self.automatic_polls
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
