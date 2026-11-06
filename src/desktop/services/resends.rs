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
//!   a resume of its operation (C5 decision 19), which the run shown then
//!   is followed by (`resent`). It waits until the republish succeeds; an
//!   answer that offers neither the same request again (`request.retry`,
//!   P17) nor a new preview under it (`request.prepare`, which the
//!   operation area's Prepare again confirms) ends it, and so does the
//!   republish being removed from the queue or not sent, or its own entry
//!   being dismissed (orchestrator ruling (a)). Dismissing the lifecycle's
//!   run does not end it: the drain sends the lifecycle on whether or not
//!   a view shows it (ruling (b)).
//!
//! A chain is read, not taken, so every view of it follows it; a cycle is
//! broken as it is made (a resend that is a run the chain already leads
//! from), and a dismissed run's link goes (`forget`).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use manyhands::{
    canonical::ItemId,
    repository::{ItemDtoKind, MutationDataDto, OperationId, RequestId},
    results::{Envelope, Outcome, RecoveryActionKind, ResultCode},
};

use super::{
    requests::{DesktopRequestId, Intent, Lane, RequestState},
    resolutions::Followed,
    worker::WorkerHandle,
};
use crate::desktop::model::{lifecycle_lock::LockKey, operations::Operations};

/// How a lifecycle goes on once the republish its way out asked for is
/// done.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum AfterRepublish {
    /// The lifecycle's request again, under its own request ID: the run
    /// shown.
    Retry(DesktopRequestId),
    /// A resume of the lifecycle's operation, which then follows `shown`,
    /// the run whose way out asked for the republish.
    Resume {
        shown: DesktopRequestId,
        root: PathBuf,
        operation_id: OperationId,
    },
}

impl AfterRepublish {
    /// The run whose way out asked for the republish.
    fn shown(&self) -> DesktopRequestId {
        match self {
            Self::Retry(shown) | Self::Resume { shown, .. } => *shown,
        }
    }
}

/// The most runs one follow walks (R → X → C is two): a guard, not a
/// limit any chain reaches.
const HOPS: usize = 8;

#[derive(Default)]
struct Registry {
    /// Each run followed by another: a resume answered
    /// `original_request_required` by the run sent for it, and a run whose
    /// republish was done by the resume sent after it.
    resent: HashMap<DesktopRequestId, DesktopRequestId>,
    /// Each republish confirmed from a lifecycle's way out: how that
    /// lifecycle goes on, and the item it republishes.
    republishes: HashMap<DesktopRequestId, Waiting>,
}

/// A republish that waits to be done.
struct Waiting {
    after: AfterRepublish,
    root: PathBuf,
    item: ItemId,
}

/// The follow-ups of lifecycle resumes and republishes.
#[derive(Clone, Default)]
pub(crate) struct Resends(Arc<Mutex<Registry>>);

impl Resends {
    /// The run that follows `shown`: the last of the chain that leads from
    /// it (a resume, then the request it named), as far as the drain has
    /// sent. `None` when nothing follows it.
    pub(crate) fn follow(&self, shown: DesktopRequestId) -> Option<DesktopRequestId> {
        let registry = self.lock();
        let mut last = None;
        let mut at = shown;
        for _ in 0..HOPS {
            let Some(next) = registry.resent.get(&at).copied() else {
                break;
            };
            if next == shown {
                break;
            }
            last = Some(next);
            at = next;
        }
        last
    }

    /// Once the republish `republish` of the item `item` of `root` is done,
    /// its lifecycle goes on as `after` says.
    pub(crate) fn after_republish(
        &self,
        republish: DesktopRequestId,
        after: AfterRepublish,
        root: PathBuf,
        item: ItemId,
    ) {
        self.lock()
            .republishes
            .insert(republish, Waiting { after, root, item });
    }

    /// Whether a republish confirmed from the way out of `run` waits to be
    /// done: Republish… is not offered again meanwhile (review I-2).
    pub(crate) fn pending_for(&self, run: DesktopRequestId) -> bool {
        self.lock()
            .republishes
            .values()
            .any(|waiting| waiting.after.shown() == run)
    }

    /// Whether a republish of the item `item` of `root` waits to be done,
    /// from any run's way out: no other Republish… of it is offered
    /// meanwhile (R-30; last review 1).
    pub(crate) fn waiting_for(&self, root: &Path, item: &ItemId) -> bool {
        self.lock()
            .republishes
            .values()
            .any(|waiting| waiting.root == root && waiting.item == *item)
    }

    /// The run `id` was queued again: what it was followed by before is
    /// not what follows it now (last review, minor 1).
    pub(in crate::desktop::services) fn requeued(&self, id: DesktopRequestId) {
        self.lock().resent.remove(&id);
    }

    /// The request `id` was dismissed: its link in a chain goes, and so
    /// does what waits on it as a republish. What waits on it as the run a
    /// republish sends on stays (ruling (b)).
    pub(crate) fn forget(&self, id: DesktopRequestId) {
        let mut registry = self.lock();
        registry.resent.remove(&id);
        registry.resent.retain(|_, next| *next != id);
        registry.republishes.remove(&id);
    }

    /// The request `id` was removed from the queue, or not sent: a
    /// republish of it will not be done.
    pub(in crate::desktop::services) fn not_run(&self, id: DesktopRequestId) {
        self.lock().republishes.remove(&id);
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
                let mut registry = self.lock();
                // The run sent again is one the chain leads from: it no
                // longer leads on (review 3).
                registry.resent.remove(&sent);
                registry.resent.insert(id, sent);
            }
            Intent::ItemRepublish { .. } => {
                let done = matches!(envelope.outcome, Outcome::Success | Outcome::Noop);
                // A stop the same request goes on from (P17), or a preview
                // to confirm again under it, keeps it waiting; any other
                // answer ends it (ruling (a)).
                let again = envelope.recovery.iter().any(|action| {
                    matches!(
                        action.action,
                        RecoveryActionKind::RequestRetry | RecoveryActionKind::RequestPrepare
                    )
                });
                let mut registry = self.lock();
                if !done {
                    if !again {
                        registry.republishes.remove(&id);
                    }
                    return;
                }
                let Some(Waiting { after, root, item }) = registry.republishes.remove(&id) else {
                    return;
                };
                match after {
                    AfterRepublish::Retry(lifecycle) => {
                        // Its operation, by its last answer or the rows.
                        let resume = lifecycle_operation(lifecycle, &root, &item, operations)
                            .map(|operation_id| (root, operation_id));
                        if let Some(Followed::Resumed(resume)) =
                            send_lifecycle_on(Some(lifecycle), resume, operations, worker)
                        {
                            registry.resent.insert(lifecycle, resume);
                        }
                    }
                    AfterRepublish::Resume {
                        shown,
                        root,
                        operation_id,
                    } => {
                        let resume = worker.submit(
                            Intent::OperationResume { root, operation_id },
                            Lane::User,
                            None,
                            None,
                        );
                        registry.resent.insert(shown, resume);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Sends a lifecycle on (R-10; C5 decision 19): its request `lifecycle`
/// again, or left as it is while queued or running, since it goes on by
/// itself. When this session holds none, or it cannot be sent again (the
/// queue closed while quitting, or a later request of the item stands in
/// its way), `operation resume` of the operation `resume` names is sent
/// instead. `None` when neither could be sent.
pub(in crate::desktop::services) fn send_lifecycle_on(
    lifecycle: Option<DesktopRequestId>,
    resume: Option<(PathBuf, OperationId)>,
    operations: &Operations,
    worker: &WorkerHandle,
) -> Option<Followed> {
    if let Some(lifecycle) = lifecycle {
        let running = operations.request(lifecycle).is_some_and(|request| {
            matches!(
                request.state,
                RequestState::Queued | RequestState::Running { .. }
            )
        });
        if running || worker.retry(lifecycle, None) {
            return Some(Followed::Resent);
        }
    }
    let (root, operation_id) = resume?;
    Some(Followed::Resumed(worker.submit(
        Intent::OperationResume { root, operation_id },
        Lane::User,
        None,
        None,
    )))
}

/// The lifecycle operation of the run `lifecycle`, of the item `item` of
/// `root`: its last answer's, or else the one the rows list for the item.
fn lifecycle_operation(
    lifecycle: DesktopRequestId,
    root: &Path,
    item: &ItemId,
    operations: &Operations,
) -> Option<OperationId> {
    let answered = operations
        .lifecycle_answer(lifecycle)
        .and_then(|envelope| envelope.operation_id.as_deref())
        .and_then(|operation| OperationId::parse(operation).ok());
    answered.or_else(|| {
        let key = LockKey::new(root.to_str()?, &item.to_string());
        let listed = operations.locks().operations(&key);
        match listed.as_slice() {
            [only] => OperationId::parse(only).ok(),
            _ => None,
        }
    })
}

/// The close or merge of the lifecycle `operation_id` of `root`, sent again
/// under the request `envelope` names: this session's request of it when
/// it holds one (left running when it runs), else a new desktop request.
/// `None` when the answer names no request, the rows name no item for the
/// operation, or this session's request cannot be queued again: the
/// resume's own answer is then what shows.
fn resend(
    root: &Path,
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
    // Never a second desktop request under the same request ID (review M1).
    if let Some(held) = held {
        let running = matches!(
            held.state,
            RequestState::Queued | RequestState::Running { .. }
        );
        return (running || worker.retry(held.id, None)).then_some(held.id);
    }
    Some(worker.submit_with_request_id(intent, Lane::User, named))
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resends_tests.rs"]
mod tests;
