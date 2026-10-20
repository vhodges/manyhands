//! The operation area's model: the session's desktop requests, newest
//! first, as the worker's events left them, and the operations each
//! repository reports pending.
//!
//! A finished request marks its repository changed, so the views that
//! show it read it again: its lists, its pending operations and the
//! repository list. A preview that found nothing to confirm marks it too.
//! Only a request whose intent can change items marks its items changed,
//! so that its open tabs read again (ruling R-B2).
//!
//! The recovery actions an envelope offers map to the controls the
//! operation area shows; an action D1 does not offer yet is shown as
//! text, unavailable.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use manyhands::{
    repository::{
        OperationAction, OperationDto, OperationFamily, OperationId, OperationListDto,
        OperationNextAction, ProgressStage,
    },
    results::{Outcome, RecoveryAction, RecoveryActionKind},
};

use super::{
    list_state::{ListState, Rows},
    navigation::UNAVAILABLE_CODES,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};
use crate::desktop::services::{
    ReadResult,
    prompts::PromptRequest,
    requests::{DesktopRequest, DesktopRequestId, Intent, RequestState},
    worker::WorkerEvent,
};

/// Why a synchronization's Resume is unavailable.
pub(crate) const SYNCHRONIZATION_UNAVAILABLE: &str = "Synchronization is not available yet";
/// Why a recovery action D1 does not offer yet is unavailable.
pub(crate) const NOT_AVAILABLE_YET: &str = "Not available yet";

impl Rows for OperationListDto {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Default)]
pub(crate) struct Operations {
    requests: Vec<DesktopRequest>,
    /// Each listed repository's pending operations, by root.
    pending: BTreeMap<RepoKey, ListState<OperationListDto>>,
    generations: Generations,
    /// Repositories a finished request may have changed, not yet read
    /// again.
    changed: Changed,
    /// The passphrase the running job asks for, until the workspace takes
    /// it to show. It holds no secret: only where the reply goes.
    prompt: Option<PromptRequest>,
}

/// What finished requests may have changed since the views last read.
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct Changed {
    /// Every repository a finished request named: its lists, pending
    /// operations and place in the repository list are read again.
    pub roots: BTreeSet<RepoKey>,
    /// Those whose request could change items (`Intent::changes_items`,
    /// ruling R-B2): their open tabs are read again too.
    pub items: BTreeSet<RepoKey>,
    /// Those whose registration a finished `repo remove` took out: their
    /// tabs close and their selection goes (design, Repositories).
    pub removed: BTreeSet<RepoKey>,
}

impl Changed {
    pub(crate) fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    fn insert(&mut self, root: RepoKey, items: bool) {
        if items {
            self.items.insert(root.clone());
        }
        self.roots.insert(root);
    }
}

impl Operations {
    /// This session's requests, newest first.
    pub(crate) fn requests(&self) -> &[DesktopRequest] {
        &self.requests
    }

    pub(crate) fn request(&self, id: DesktopRequestId) -> Option<&DesktopRequest> {
        self.requests.iter().find(|request| request.id == id)
    }

    fn request_mut(&mut self, id: DesktopRequestId) -> Option<&mut DesktopRequest> {
        self.requests.iter_mut().find(|request| request.id == id)
    }

    /// Applies what the worker said. An event for a request this model
    /// never saw queued (a host observation) changes nothing here.
    pub(crate) fn apply(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Queued(request) => match self.request_mut(request.id) {
                // A retry: the same request, in its place.
                Some(existing) => *existing = request,
                None => self.requests.insert(0, request),
            },
            WorkerEvent::Started(id) => self.set(
                id,
                RequestState::Running {
                    stage: None,
                    accepted: false,
                },
            ),
            WorkerEvent::Progress(id, progress) => {
                if let Some(request) = self.request_mut(id)
                    && let RequestState::Running { stage, accepted } = &mut request.state
                {
                    *stage = Some(progress.stage);
                    *accepted |= progress.stage == ProgressStage::Accepted;
                }
            }
            WorkerEvent::Finished(id, envelope) => {
                let envelope = *envelope;
                // Only a removal that succeeded took its registration out:
                // a refused one left it, and one whose end state already
                // held found it gone or another in its place.
                let removed = envelope.outcome == Outcome::Success;
                let root = self.request(id).and_then(|request| {
                    let root = request.intent.root()?.to_str()?.to_owned();
                    let removed = removed && matches!(request.intent, Intent::RepoRemove { .. });
                    Some((root, request.intent.changes_items(), removed))
                });
                self.set(
                    id,
                    RequestState::Finished(
                        envelope.outcome,
                        envelope.code,
                        envelope.effects,
                        envelope.recovery,
                        envelope.message,
                    ),
                );
                if let Some((root, items, removed)) = root {
                    if removed {
                        self.changed.removed.insert(root.clone());
                    }
                    self.changed.insert(root, items);
                }
            }
            WorkerEvent::Removed(id) => self.set(id, RequestState::Removed),
            WorkerEvent::NotSent(id, why) => self.set(id, RequestState::NotSent(why)),
            // The Hosts page reads observations (Task 16).
            WorkerEvent::HostObserved(..) => {}
            // One worker asks one thing at a time: a prompt still kept
            // when the next comes is one its job no longer waits on.
            WorkerEvent::Prompt(prompt) => self.prompt = Some(prompt),
        }
    }

    /// Whether a prompt waits to be shown.
    #[cfg(test)]
    pub(crate) fn has_prompt(&self) -> bool {
        self.prompt.is_some()
    }

    /// The prompt waiting to be shown, taken: whoever takes it answers it.
    pub(crate) fn take_prompt(&mut self) -> Option<PromptRequest> {
        self.prompt.take()
    }

    /// Hides a request that is no longer queued or running.
    pub(crate) fn dismiss(&mut self, id: DesktopRequestId) {
        if let Some(request) = self.request_mut(id)
            && !matches!(
                request.state,
                RequestState::Queued | RequestState::Running { .. }
            )
        {
            request.state = RequestState::Dismissed;
        }
    }

    fn set(&mut self, id: DesktopRequestId, state: RequestState) {
        if let Some(request) = self.request_mut(id) {
            request.state = state;
        }
    }

    /// Says `root` may have changed, so the views that show it read it
    /// again; with `items`, its open tabs too.
    pub(crate) fn mark_changed(&mut self, root: RepoKey, items: bool) {
        self.changed.insert(root, items);
    }

    /// What was marked changed since the last call.
    pub(crate) fn take_changed(&mut self) -> Changed {
        std::mem::take(&mut self.changed)
    }

    /// Each listed repository's pending operations, by root.
    pub(crate) fn pending(&self) -> &BTreeMap<RepoKey, ListState<OperationListDto>> {
        &self.pending
    }

    /// Starts reading `root`'s pending operations; an earlier read is
    /// dropped when it completes. Rows already shown stay until the new
    /// read lands.
    pub(crate) fn load_pending(&mut self, root: RepoKey) -> Stamp {
        self.pending
            .entry(root.clone())
            .or_insert(ListState::Loading);
        self.generations.next(ViewScope::Operations(root))
    }

    /// A pending-operations read completed. A repository that does not
    /// resolve lists nothing. False when the read is no longer wanted.
    pub(crate) fn apply_pending(
        &mut self,
        stamp: Stamp,
        result: ReadResult<OperationListDto>,
    ) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        let ViewScope::Operations(root) = stamp.scope else {
            return false;
        };
        if let ReadResult::Failed(failure) = &result
            && UNAVAILABLE_CODES.contains(&failure.code)
        {
            self.pending.remove(&root);
            return true;
        }
        let state = ListState::from_read(result, |list| ListState::Ready {
            complete: list.complete,
            index: None,
            rows: list,
        });
        self.pending.insert(root, state);
        true
    }

    /// Lets go of every repository not in `roots`; their reads still
    /// running are dropped.
    pub(crate) fn retain_pending(&mut self, roots: &[RepoKey]) {
        let gone: Vec<RepoKey> = self
            .pending
            .keys()
            .filter(|root| !roots.contains(root))
            .cloned()
            .collect();
        for root in gone {
            self.pending.remove(&root);
            self.generations.next(ViewScope::Operations(root));
        }
    }
}

/// What running a recovery action does.
pub(crate) enum RecoveryStep {
    /// Submit this command as a user job.
    Submit(Intent),
    /// Open the confirmation dialog for this command.
    Confirm(Intent),
    /// Submit the request again, under its own request ID.
    Retry(DesktopRequestId),
    /// Open the confirmation dialog for a fresh preview of the request.
    Prepare(DesktopRequestId),
    Cancel(DesktopRequestId),
    /// Open the Repositories page with this root checked.
    Inspect(PathBuf),
    /// Open the identity form of this root.
    IdentitySet(PathBuf),
}

impl Operations {
    /// What the recovery action `action`, offered on the request `id`,
    /// runs; `None` when D1 does not offer it, or it names nothing this
    /// session can act on.
    pub(crate) fn recovery_step(
        &self,
        id: DesktopRequestId,
        action: &RecoveryAction,
    ) -> Option<RecoveryStep> {
        let request = self.request(id)?;
        let root = || -> Option<PathBuf> {
            match action.arguments.get("root") {
                Some(root) => root.as_str().map(PathBuf::from),
                None => request.intent.root().map(Path::to_owned),
            }
        };
        let operation = || {
            action
                .operation_id
                .as_deref()
                .and_then(|id| OperationId::parse(id).ok())
        };
        // A request this session holds, by the request ID the action
        // names; this one when it names none.
        let named = || match action.arguments.get("request_id") {
            None => Some(id),
            Some(named) => {
                let named = named.as_str()?;
                self.requests
                    .iter()
                    .find(|request| request.request_id.to_string() == named)
                    .map(|request| request.id)
            }
        };
        Some(match recovery_control(action.action) {
            RecoveryControl::IndexRefresh => {
                RecoveryStep::Submit(Intent::IndexRefresh { root: root()? })
            }
            RecoveryControl::IndexRebuild => {
                RecoveryStep::Submit(Intent::IndexRebuild { root: root()? })
            }
            RecoveryControl::Retry => RecoveryStep::Retry(named()?),
            RecoveryControl::Prepare => {
                if !request.intent.confirmed() {
                    return None;
                }
                RecoveryStep::Prepare(id)
            }
            RecoveryControl::Cancel => RecoveryStep::Cancel(named()?),
            RecoveryControl::Resume => RecoveryStep::Submit(Intent::OperationResume {
                root: root()?,
                operation_id: operation()?,
            }),
            RecoveryControl::Abandon => RecoveryStep::Confirm(Intent::OperationAbandon {
                root: root()?,
                operation_id: operation()?,
            }),
            RecoveryControl::Inspect => RecoveryStep::Inspect(root()?),
            RecoveryControl::IdentitySet => RecoveryStep::IdentitySet(root()?),
            RecoveryControl::Unavailable => return None,
        })
    }
}

/// Whether a pending row is a synchronization, which D1 cannot run.
fn is_synchronization(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Remote
        || matches!(
            operation.action,
            OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
        )
}

/// What a pending row offers for Resume.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ResumeOffer {
    /// A local operation waiting to be resumed: Resume runs it.
    Offered,
    /// Shown disabled, with why.
    Unavailable(&'static str),
    NotOffered,
}

/// Resume is offered when the row waits to be resumed and is a local
/// command; a synchronization shows it unavailable.
pub(crate) fn resume_offer(operation: &OperationDto) -> ResumeOffer {
    if operation.next_action != Some(OperationNextAction::Resume) {
        return ResumeOffer::NotOffered;
    }
    if is_synchronization(operation) {
        return ResumeOffer::Unavailable(SYNCHRONIZATION_UNAVAILABLE);
    }
    match (operation.family, &operation.operation_id) {
        (OperationFamily::Local, Some(_)) => ResumeOffer::Offered,
        _ => ResumeOffer::NotOffered,
    }
}

/// Abandon is offered for a pending local operation with an ID.
pub(crate) fn abandon_offered(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Local
        && operation.operation_id.is_some()
        && !is_synchronization(operation)
}

/// The control a recovery action shows as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecoveryControl {
    IndexRefresh,
    IndexRebuild,
    Retry,
    Prepare,
    Cancel,
    Resume,
    Abandon,
    /// Open the Repositories page with the root checked.
    Inspect,
    /// Open the identity form of the root.
    IdentitySet,
    /// D1 does not offer it yet: shown as text, disabled.
    Unavailable,
}

pub(crate) fn recovery_control(kind: RecoveryActionKind) -> RecoveryControl {
    match kind {
        RecoveryActionKind::IndexRefresh => RecoveryControl::IndexRefresh,
        RecoveryActionKind::IndexRebuild => RecoveryControl::IndexRebuild,
        RecoveryActionKind::RequestRetry => RecoveryControl::Retry,
        RecoveryActionKind::RequestPrepare => RecoveryControl::Prepare,
        RecoveryActionKind::RequestCancel => RecoveryControl::Cancel,
        RecoveryActionKind::OperationResume => RecoveryControl::Resume,
        RecoveryActionKind::OperationAbandon => RecoveryControl::Abandon,
        RecoveryActionKind::RepoInspect => RecoveryControl::Inspect,
        RecoveryActionKind::RepoIdentitySet => RecoveryControl::IdentitySet,
        // The Hosts page that runs these arrives with Task 16.
        RecoveryActionKind::HostApprove | RecoveryActionKind::HostReplace => {
            RecoveryControl::Unavailable
        }
    }
}

/// A recovery action's control, in fixed text.
pub(crate) fn recovery_label(kind: RecoveryActionKind) -> &'static str {
    match kind {
        RecoveryActionKind::IndexRefresh => "Refresh index",
        RecoveryActionKind::IndexRebuild => "Rebuild index",
        RecoveryActionKind::RequestRetry => "Retry",
        RecoveryActionKind::RequestPrepare => "Prepare again",
        RecoveryActionKind::RequestCancel => "Cancel",
        RecoveryActionKind::OperationResume => "Resume",
        RecoveryActionKind::OperationAbandon => "Abandon",
        RecoveryActionKind::RepoIdentitySet => "Set identity",
        RecoveryActionKind::HostApprove => "Approve host",
        RecoveryActionKind::HostReplace => "Replace host key",
        RecoveryActionKind::RepoInspect => "Inspect repository",
    }
}

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
