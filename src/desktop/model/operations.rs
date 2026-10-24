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
//! operation area shows; an action that names nothing this session can
//! act on (a request it does not hold, a root or host it cannot read) is
//! shown as text, unavailable.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use manyhands::{
    drafts::{DraftError, DraftErrorKind},
    repository::{
        HostObservationDto, OperationAction, OperationDto, OperationFamily, OperationId,
        OperationListDto, OperationNextAction, ProgressStage, transport::SshAuthority,
    },
    results::{Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode},
};

use super::{
    list_state::{ListState, Rows},
    navigation::UNAVAILABLE_CODES,
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};
use crate::desktop::services::{
    DraftStoreUnavailable, ReadResult,
    prompts::PromptRequest,
    requests::{DesktopRequest, DesktopRequestId, Intent, RequestState},
    worker::WorkerEvent,
};

/// Why a synchronization's Resume is unavailable.
pub(crate) const SYNCHRONIZATION_UNAVAILABLE: &str = "Synchronization is not available yet";
/// Why a recovery action that names nothing this session can act on is
/// unavailable.
pub(crate) const NOT_AVAILABLE_YET: &str = "Not available yet";
/// The persistent entry of a draft store that did not open.
pub(crate) const DRAFTS_UNAVAILABLE: &str = "Recovery copies are unavailable";

/// The store-unavailable entry's text: the fixed sentence and the store's
/// fixed reason, never a path.
pub(crate) fn drafts_unavailable(reason: DraftStoreUnavailable) -> String {
    format!("{DRAFTS_UNAVAILABLE}: {reason}")
}

/// A failing session's entry text: its item's title as read, never the
/// draft's (ruling R25), and the store's fixed reason.
pub(crate) fn draft_flush_failed(title: &str, kind: DraftErrorKind) -> String {
    format!(
        "Recovery copy of {title} not saved: {}",
        DraftError { kind }
    )
}

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
    /// How many removals were recorded (`removals`).
    removals: u64,
    /// The last host observation's answer, by its request: what the Hosts
    /// page that asked reads. One worker observes one host at a time, and
    /// the page waits only for its latest.
    observed: Option<(DesktopRequestId, Envelope<HostObservationDto>)>,
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
    /// Those whose registration this session's `repo remove` took out, or
    /// found already gone (ruling R-T13a): their tabs close and their
    /// selection goes (design, Repositories).
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
                // A removal that succeeded took its registration out, and one
                // answered `already_applied` found it gone (ruling R-T13a); a
                // refused one left it.
                let removed = envelope.outcome == Outcome::Success
                    || envelope.code == ResultCode::AlreadyApplied;
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
                        self.removals += 1;
                    }
                    self.changed.insert(root, items);
                }
            }
            WorkerEvent::Removed(id) => self.set(id, RequestState::Removed),
            WorkerEvent::NotSent(id, why) => self.set(id, RequestState::NotSent(why)),
            // The Hosts page reads it. An observation pins nothing, so no
            // view has anything to read again.
            WorkerEvent::HostObserved(id, envelope) => self.observed = Some((id, envelope)),
            // One worker asks one thing at a time: a prompt still kept
            // when the next comes is one its job no longer waits on.
            WorkerEvent::Prompt(prompt) => self.prompt = Some(prompt),
        }
    }

    /// The answer of the host observation `id`, once it has one.
    pub(crate) fn observation(
        &self,
        id: DesktopRequestId,
    ) -> Option<&Envelope<HostObservationDto>> {
        match &self.observed {
            Some((observed, envelope)) if *observed == id => Some(envelope),
            _ => None,
        }
    }

    /// Whether any host observation has been answered.
    #[cfg(test)]
    pub(crate) fn observed_any(&self) -> bool {
        self.observed.is_some()
    }

    /// Whether the last host observation answered is a success.
    #[cfg(test)]
    pub(crate) fn last_observation_succeeded(&self) -> bool {
        self.observed
            .as_ref()
            .is_some_and(|(_, envelope)| envelope.outcome == Outcome::Success)
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

    /// Says `root`'s registration was removed by this session (ruling
    /// R-T13a): its tabs close and its selection goes.
    pub(crate) fn mark_removed(&mut self, root: RepoKey) {
        self.changed.removed.insert(root.clone());
        self.removals += 1;
        self.changed.insert(root, false);
    }

    /// How many removals this session has recorded: a view that shows
    /// registrations reads them again when it moves on.
    pub(crate) fn removals(&self) -> u64 {
        self.removals
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
    /// Open the Hosts page on this root and check this host: what it
    /// presents decides whether Approve or Replace is offered there
    /// (`host.approve`, `host.replace`).
    CheckHost {
        root: PathBuf,
        authority: SshAuthority,
    },
}

/// The host a recovery action names: `host:port`, with an address that
/// holds a colon in brackets, as the library writes it. `None` for text
/// in any other form.
pub(crate) fn parse_authority(text: &str) -> Option<SshAuthority> {
    let (host, port) = text.rsplit_once(':')?;
    let host = match host.strip_prefix('[') {
        Some(bracketed) => bracketed.strip_suffix(']')?,
        None if host.contains(':') => return None,
        None => host,
    };
    if host.is_empty() {
        return None;
    }
    Some(SshAuthority {
        host: host.to_owned(),
        port: port.parse().ok()?,
    })
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
            RecoveryControl::CheckHost => RecoveryStep::CheckHost {
                root: root()?,
                authority: parse_authority(action.arguments.get("authority")?.as_str()?)?,
            },
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
    /// Open the Hosts page on the root and check the host named.
    CheckHost,
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
        // The action names the host but not the key to pin: the Hosts
        // page checks the host, and offers Approve or Replace for the key
        // it presents.
        RecoveryActionKind::HostApprove | RecoveryActionKind::HostReplace => {
            RecoveryControl::CheckHost
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
