//! The lifecycle answers and the lifecycle lock's state `Operations` keeps
//! (D5 A2, A3).

use std::path::Path;

use manyhands::{repository::MutationDataDto, results::Envelope};

use super::Operations;
use crate::desktop::{
    model::lifecycle_lock::{LifecycleLocks, LockKey},
    services::requests::{DesktopRequestId, Intent},
};

impl Operations {
    /// The last answer of the lifecycle `id`: a promotion's or a closure's,
    /// or a resume's with lifecycle data. `None` while it is queued or
    /// running again, and for any other request.
    pub(crate) fn lifecycle_answer(
        &self,
        id: DesktopRequestId,
    ) -> Option<&Envelope<MutationDataDto>> {
        self.lifecycles.get(&id)
    }

    /// A finished request of an item's lifecycle, or a resume that
    /// answered with lifecycle data, tells the lock: until the rows are
    /// read again the item awaits, and the answer sets or ends its L-4
    /// exemption (D5 A2; R-21, R-22).
    pub(super) fn lifecycle_answered(
        &mut self,
        id: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
    ) {
        let Some(request) = self.request(id) else {
            return;
        };
        let Some(root) = request.intent.root().and_then(Path::to_str) else {
            return;
        };
        let data = match &envelope.data {
            Some(MutationDataDto::Lifecycle(data)) => Some(data.as_ref()),
            _ => None,
        };
        let item = match (&request.intent, data) {
            (_, Some(data)) => data.item_id.clone(),
            (Intent::TicketClose { id, .. } | Intent::DocumentPromote { id, .. }, None) => {
                id.to_string()
            }
            _ => return,
        };
        let key = LockKey::new(root, &item);
        self.locks.answered(key, id, data);
    }

    /// The lifecycle lock's state.
    pub(crate) fn locks(&self) -> &LifecycleLocks {
        &self.locks
    }

    /// The lifecycle lock's state, to change.
    pub(crate) fn locks_mut(&mut self) -> &mut LifecycleLocks {
        &mut self.locks
    }
}
