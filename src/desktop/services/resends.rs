//! What follows a lifecycle's resume or its republish, run by the drain
//! whether or not a view still shows it (D5 decision 10, Part C, A3).
//!
//! - **Resume, then resend.** `operation resume` of a promotion's or a
//!   closure's operation that a request still holds is answered
//!   `original_request_required`, naming that request. The close or merge
//!   is sent again under it, rebuilt from the item the lifecycle rows name
//!   (`LifecycleLocks::item_of`): this session's request of it again when
//!   it holds one, or else a new desktop request under that request ID,
//!   with no confirmation and no passphrase (A2). A view that showed the
//!   resume follows the run sent (`resent`). The resume from a row, from
//!   the lifecycle dialog and the one an applied lifecycle child's
//!   resolution sends (R-10) all go this way.
//! - **Republish, then resend.** A republish confirmed from a lifecycle's
//!   way out (A3's Stop and republish…, L1's Republish…) sends that
//!   lifecycle on once it is answered with success: its request again, or
//!   a resume of its operation (C5 decision 19). Any other answer leaves it
//!   waiting, so a stopped republish sent again under its own request
//!   (P17) still sends the lifecycle on.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use manyhands::{
    canonical::ItemId,
    repository::{ItemDtoKind, MutationDataDto, OperationId, RequestId},
    results::{Envelope, Outcome, RecoveryActionKind, ResultCode},
};

use super::{
    requests::{DesktopRequestId, Intent, Lane, RequestState},
    worker::WorkerHandle,
};
use crate::desktop::model::operations::Operations;

/// How a lifecycle goes on once the republish its way out asked for is
/// done.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum AfterRepublish {
    /// The lifecycle's request again, under its own request ID.
    Retry(DesktopRequestId),
    /// A resume of the lifecycle's operation.
    Resume {
        root: PathBuf,
        operation_id: OperationId,
    },
}

#[derive(Default)]
struct Registry {
    /// Each resume answered `original_request_required`, and the run sent
    /// for it.
    resent: HashMap<DesktopRequestId, DesktopRequestId>,
    /// Each republish confirmed from a lifecycle's way out, and how that
    /// lifecycle goes on.
    republishes: HashMap<DesktopRequestId, AfterRepublish>,
}

/// The follow-ups of lifecycle resumes and republishes.
#[derive(Clone, Default)]
pub(crate) struct Resends(Arc<Mutex<Registry>>);

impl Resends {
    /// The run the resume `resume` was sent on as, once its answer named
    /// the request that holds the lifecycle.
    pub(crate) fn resent(&self, resume: DesktopRequestId) -> Option<DesktopRequestId> {
        self.lock().resent.get(&resume).copied()
    }

    /// Once the republish `republish` is done, its lifecycle goes on as
    /// `after` says.
    pub(crate) fn after_republish(&self, republish: DesktopRequestId, after: AfterRepublish) {
        self.lock().republishes.insert(republish, after);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Registry> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The request `id` finished with `envelope`: a lifecycle's resume
    /// answered `original_request_required` sends the named request again,
    /// and a republish done sends its lifecycle on. Called before the
    /// request model applies the answer.
    pub(in crate::desktop::services) fn ended(
        &self,
        id: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
        operations: &Operations,
        worker: &WorkerHandle,
    ) {
        let Some(request) = operations.request(id) else {
            return;
        };
        match &request.intent {
            Intent::OperationResume { root, operation_id }
                if envelope.code == ResultCode::OriginalRequestRequired =>
            {
                let Some(sent) = resend(root, *operation_id, envelope, operations, worker) else {
                    return;
                };
                self.lock().resent.insert(id, sent);
            }
            Intent::ItemRepublish { .. }
                if matches!(envelope.outcome, Outcome::Success | Outcome::Noop) =>
            {
                let Some(after) = self.lock().republishes.remove(&id) else {
                    return;
                };
                match after {
                    AfterRepublish::Retry(lifecycle) => {
                        worker.retry(lifecycle, None);
                    }
                    AfterRepublish::Resume { root, operation_id } => {
                        worker.submit(
                            Intent::OperationResume { root, operation_id },
                            Lane::User,
                            None,
                            None,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

/// The close or merge of the lifecycle `operation_id` of `root`, sent again
/// under the request `envelope` names: this session's request of it when
/// it holds one (left running when it runs), else a new desktop request.
/// `None` when the answer names no request, or the rows name no item for
/// the operation.
fn resend(
    root: &std::path::Path,
    operation_id: OperationId,
    envelope: &Envelope<MutationDataDto>,
    operations: &Operations,
    worker: &WorkerHandle,
) -> Option<DesktopRequestId> {
    let named = envelope
        .recovery
        .iter()
        .filter(|action| action.action == RecoveryActionKind::RequestRetry)
        .find_map(|action| action.arguments.get("request_id")?.as_str())
        .and_then(|named| RequestId::parse(named).ok())?;
    let (kind, item) = operations
        .locks()
        .item_of(root.to_str()?, &operation_id.to_string())?;
    let id: ItemId = item.parse().ok()?;
    let root = root.to_path_buf();
    let intent = match kind {
        ItemDtoKind::Ticket => Intent::TicketClose { root, id },
        ItemDtoKind::Document => Intent::DocumentPromote { root, id },
    };
    let held = operations.requests().iter().find(|request| {
        request.request_id == named
            && matches!(
                request.intent,
                Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
            )
    });
    if let Some(held) = held {
        let running = matches!(
            held.state,
            RequestState::Queued | RequestState::Running { .. }
        );
        if running || worker.retry(held.id, None) {
            return Some(held.id);
        }
    }
    Some(worker.submit_with_request_id(intent, Lane::User, named))
}
