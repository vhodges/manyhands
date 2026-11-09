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
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use manyhands::{
    drafts::{DraftError, DraftErrorKind},
    repository::{
        HostObservationDto, MutationDataDto, OperationAction, OperationDto, OperationFamily,
        OperationId, OperationListDto, OperationNextAction, ProgressStage, transport::SshAuthority,
    },
    results::{
        CheckpointEffect, Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode,
    },
};

mod outcomes;
mod review;

pub(crate) use review::{Review, ReviewOf};

#[cfg(test)]
pub(crate) use outcomes::main_branch_deleted;
#[cfg(test)]
pub(crate) use outcomes::{
    ANOTHER_SHARE_HOLDS, BRANCH_DELETED, CHOOSE_A_KEY, CURRENT, KEY_LOCKED, NEEDS_ATTENTION,
    NEEDS_IDENTITY, NOWHERE_TO_SHARE, SHARED, check_host_text, may_be_waiting_for,
};
pub(crate) use outcomes::{
    CONFLICT_BANNER, HISTORY_UNRESOLVED_BLOCKER, HISTORY_UNRESOLVED_GUIDANCE, primary_name,
    primary_subject, share_text,
};

use super::{
    lifecycle_lock::{LifecycleLocks, LockKey},
    list_state::{ListState, Rows},
    navigation::UNAVAILABLE_CODES,
    publication::{ShareAttempt, ShareTarget, SyncResult, sync_outcome},
    stamp::{Generations, RepoKey, Stamp, ViewScope},
};
use crate::desktop::services::{
    DraftStoreUnavailable, ReadResult,
    prompts::PromptRequest,
    requests::{DesktopRequest, DesktopRequestId, Intent, Refusals, RequestState},
    worker::{RecordState, WorkerEvent},
};

/// Why Resume is unavailable for a pending synchronization that no
/// `operation resume` here takes up (D5).
pub(crate) const SYNCHRONIZATION_UNAVAILABLE: &str = "Synchronization is not available yet";
/// Why a recovery action that names nothing this session can act on is
/// unavailable.
pub(crate) const NOT_AVAILABLE_YET: &str = "Not available yet";
/// Why the Retry of a context request is unavailable once a later request
/// of its item was sent (R-78).
pub(crate) const NOT_SENT_AGAIN: &str = "Not sent again after a later Edit or release";
/// A share stopped at a safe point before anything was sent.
pub(crate) const STOPPED_BEFORE_SENDING: &str =
    "Stopped before anything was sent. Your saved work is still on this computer.";
/// A share stopped at a safe point after part of its work finished.
pub(crate) const STOPPED_AFTER_PART: &str = "Stopped after part of the work finished. Your saved work is on this computer; Share sends the rest.";
/// The persistent entry of a draft store that did not open.
pub(crate) const DRAFTS_UNAVAILABLE: &str = "Recovery copies are unavailable";

/// The store-unavailable entry's text: the fixed sentence and the store's
/// fixed reason, never a path.
pub(crate) fn drafts_unavailable(reason: DraftStoreUnavailable) -> String {
    format!("{DRAFTS_UNAVAILABLE}: {reason}")
}

/// The launch's entry: how many drafts it found that no one here holds.
pub(crate) fn recovered_drafts(count: usize) -> String {
    format!("Recovered drafts: {count}")
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
    /// When each request's last attempt was queued, in the order of every
    /// attempt (R-78), and how many have been.
    sent: HashMap<DesktopRequestId, u64>,
    sends: u64,
    /// What each request's attempts were answered (R-78's exemption).
    answers: HashMap<DesktopRequestId, Refusals>,
    /// The IDs a ticket request's last answer rejected it for, for its
    /// relationships (`TicketMutationDto::rejected`): IDs, never text.
    rejected: HashMap<DesktopRequestId, Vec<String>>,
    /// Each listed repository's pending operations, by root.
    pending: BTreeMap<RepoKey, ListState<OperationListDto>>,
    generations: Generations,
    /// Repositories a finished request may have changed, not yet read
    /// again.
    changed: Changed,
    /// The passphrase the running job asks for, until the workspace takes
    /// it to show. It holds no secret: only where the reply goes.
    prompt: Option<PromptRequest>,
    /// Quitting is saving drafts (`QuitPhase::SavingDrafts`).
    saving_drafts: bool,
    /// How many removals were recorded (`removals`).
    removals: u64,
    /// The last host observation's answer, by its request: what the Hosts
    /// page that asked reads. One worker observes one host at a time, and
    /// the page waits only for its latest.
    observed: Option<(DesktopRequestId, Envelope<HostObservationDto>)>,
    /// The running requests whose Stop was pressed and signalled their
    /// token, until they end.
    stopping: HashSet<DesktopRequestId>,
    /// Each finished share's last answer, which its outcome is classified
    /// from (`share_result`).
    shares: HashMap<DesktopRequestId, Envelope<MutationDataDto>>,
    /// Each finished lifecycle's last answer, which the lifecycle dialog
    /// shows and takes its way out from (D5, A3): a promotion's or a
    /// closure's, or a resume's that answered with lifecycle data.
    lifecycles: HashMap<DesktopRequestId, Envelope<MutationDataDto>>,
    /// The lifecycle lock's own state (D5 A2; R-21, R-22).
    locks: LifecycleLocks,
    /// The operation each finished request's last answer stopped on with a
    /// conflict, whose sheet its entry offers (D5 B5, `review`).
    reviews: HashMap<DesktopRequestId, Review>,
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
    /// Those of `removed` whose `repo remove` request the user confirmed
    /// and the worker ran. A preview that found the registration gone was
    /// never confirmed: a tab whose edit session holds unsaved text that no
    /// draft holds stays open, as when a repository leaves the list (Part C
    /// fix 1, I1).
    pub confirmed: BTreeSet<RepoKey>,
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

    /// The IDs the last answer to the ticket request `id` rejected it for,
    /// in ID order: a relationship cycle's tickets, or the IDs that are not
    /// tickets. Empty for any other answer.
    pub(crate) fn rejected(&self, id: DesktopRequestId) -> &[String] {
        self.rejected.get(&id).map_or(&[], Vec::as_slice)
    }

    fn request_mut(&mut self, id: DesktopRequestId) -> Option<&mut DesktopRequest> {
        self.requests.iter_mut().find(|request| request.id == id)
    }

    /// Applies what the worker said. An event for a request this model
    /// never saw queued (a host observation) changes nothing here.
    pub(crate) fn apply(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Queued(request) => {
                self.stopping.remove(&request.id);
                self.shares.remove(&request.id);
                self.lifecycles.remove(&request.id);
                self.reviews.remove(&request.id);
                self.sends += 1;
                self.sent.insert(request.id, self.sends);
                self.answers.entry(request.id).or_default().queued();
                match self.request_mut(request.id) {
                    // A retry: the same request, in its place.
                    Some(existing) => *existing = request,
                    None => self.requests.insert(0, request),
                }
            }
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
                    // Every stage comes after acceptance, and a same-ID
                    // re-entry never reports `Accepted` (fact 10): any
                    // stage marks it accepted, as the worker does (R-14).
                    *accepted = true;
                }
            }
            WorkerEvent::Finished(id, envelope) => {
                let envelope = *envelope;
                self.stopping.remove(&id);
                match review::conflict_of(&envelope) {
                    Some(review) => self.reviews.insert(id, review),
                    None => self.reviews.remove(&id),
                };
                // A resolution applied: no entry offers its conflict again,
                // and the banner clears (B3).
                if let Some(resolved) = self
                    .request(id)
                    .and_then(|request| review::resolved_by(&request.intent, &envelope))
                {
                    self.reviews
                        .retain(|_, review| review.operation_id != resolved);
                }
                if self
                    .request(id)
                    .is_some_and(|request| ShareTarget::of(&request.intent).is_some())
                {
                    self.shares.insert(id, envelope.clone());
                }
                if self.request(id).is_some_and(|request| {
                    matches!(
                        request.intent,
                        Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
                    )
                }) || matches!(envelope.data, Some(MutationDataDto::Lifecycle(_)))
                {
                    self.lifecycles.insert(id, envelope.clone());
                }
                self.lifecycle_answered(id, &envelope);
                match &envelope.data {
                    Some(MutationDataDto::Ticket(ticket)) if !ticket.rejected.is_empty() => {
                        self.rejected.insert(id, ticket.rejected.clone());
                    }
                    _ => {
                        self.rejected.remove(&id);
                    }
                }
                // What it was answered, for R-78's exemption (`Refusals`).
                self.answers
                    .entry(id)
                    .or_default()
                    .finished(Intent::refused_answer(
                        envelope.outcome,
                        envelope.code,
                        &envelope.recovery,
                    ));
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
                        self.changed.confirmed.insert(root.clone());
                        self.removals += 1;
                    }
                    self.changed.insert(root, items);
                }
            }
            WorkerEvent::Removed(id) => {
                self.stopping.remove(&id);
                self.set(id, RequestState::Removed);
            }
            WorkerEvent::NotSent(id, why) => {
                self.stopping.remove(&id);
                self.set(id, RequestState::NotSent(why));
            }
            // The Hosts page reads it. An observation pins nothing, so no
            // view has anything to read again.
            WorkerEvent::HostObserved(id, envelope) => self.observed = Some((id, envelope)),
            // One worker asks one thing at a time: a prompt still kept
            // when the next comes is one its job no longer waits on.
            WorkerEvent::Prompt(prompt) => self.prompt = Some(prompt),
            // A poll is not a request: the drain takes its attempt before
            // `apply` and hands it to the poll inbox (D6 B1).
            WorkerEvent::Polled(..) => {}
        }
    }

    /// The request running now, if the model has heard it start: one
    /// worker runs one at a time.
    pub(crate) fn running(&self) -> Option<&DesktopRequest> {
        self.requests
            .iter()
            .find(|request| matches!(request.state, RequestState::Running { .. }))
    }

    /// Stop was pressed on the running request `id` and signalled its
    /// token: it reads Stopping… until it ends.
    pub(crate) fn stopping(&mut self, id: DesktopRequestId) {
        if self
            .request(id)
            .is_some_and(|request| matches!(request.state, RequestState::Running { .. }))
        {
            self.stopping.insert(id);
        }
    }

    /// What the request `id` offers to stop it once accepted (R-1, R-15):
    /// Stop at a safe point while an accepted command that stops at safe
    /// points runs in a repository with a publication remote (`remote`),
    /// Stopping… once pressed, and nothing otherwise. A resume stops at
    /// safe points only when it takes up a synchronization: the operation
    /// it names is listed pending in its repository as a remote item or
    /// repository synchronization (`takes_up_remote_work`); any other
    /// resume ignores its token. Once pressed it reads Stopping… until it
    /// ends, whatever a later read of the pending operations lists.
    pub(crate) fn stop_offer(&self, id: DesktopRequestId, remote: bool) -> StopOffer {
        let Some(request) = self.request(id) else {
            return StopOffer::NotOffered;
        };
        if !matches!(request.state, RequestState::Running { accepted: true, .. }) {
            return StopOffer::NotOffered;
        }
        // Signalled: it reads Stopping… until it ends, even when a read
        // since no longer lists the operation a resume took up.
        if self.stopping.contains(&id) {
            return StopOffer::Stopping;
        }
        if !request.intent.stops_at_safe_points()
            || !remote
            || !self.resumes_remote_work(&request.intent)
        {
            return StopOffer::NotOffered;
        }
        StopOffer::Offered
    }

    /// Whether `intent`, if it is a resume, names an operation its
    /// repository lists pending as remote work a resume takes up at safe
    /// points (`takes_up_remote_work`). True for any other intent.
    fn resumes_remote_work(&self, intent: &Intent) -> bool {
        let Intent::OperationResume { root, operation_id } = intent else {
            return true;
        };
        let named = operation_id.to_string();
        root.to_str()
            .and_then(|root| self.pending.get(root))
            .and_then(ListState::rows)
            .is_some_and(|list| {
                list.items.iter().any(|row| {
                    row.operation_id.as_deref() == Some(named.as_str()) && takes_up_remote_work(row)
                })
            })
    }

    /// The outcome of the finished share `id`, classified from its last
    /// answer under where its record stood (`record`) and whether that
    /// answer was a Give up's. `None` for any other request, or a share
    /// not finished.
    pub(crate) fn share_result(
        &self,
        id: DesktopRequestId,
        record: RecordState,
        after_give_up: bool,
    ) -> Option<SyncResult> {
        let request = self.request(id)?;
        if !matches!(request.state, RequestState::Finished(..)) {
            return None;
        }
        let target = ShareTarget::of(&request.intent)?;
        let envelope = self.shares.get(&id)?;
        Some(sync_outcome(
            envelope,
            ShareAttempt {
                target,
                record,
                after_give_up,
            },
        ))
    }

    /// The operation whose conflict the finished request `id` last stopped
    /// on, which Review changes opens the sheet on (D5 B5).
    pub(crate) fn review(&self, id: DesktopRequestId) -> Option<&Review> {
        self.reviews.get(&id)
    }

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
    fn lifecycle_answered(&mut self, id: DesktopRequestId, envelope: &Envelope<MutationDataDto>) {
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
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn observed_any(&self) -> bool {
        self.observed.is_some()
    }

    /// Whether the last host observation answered is a success.
    #[cfg(all(test, feature = "desktop-test-support"))]
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

    /// A prompt taken and withdrawn unanswered from its dialog, to be
    /// shown or answered later. One worker asks one prompt at a time, so
    /// nothing else waits here.
    pub(crate) fn hold_prompt(&mut self, prompt: PromptRequest) {
        self.prompt = Some(prompt);
    }

    /// Whether quitting is saving drafts: no confirmation opens in front
    /// of Quit now.
    pub(crate) fn saving_drafts(&self) -> bool {
        self.saving_drafts
    }

    pub(crate) fn set_saving_drafts(&mut self, saving: bool) {
        self.saving_drafts = saving;
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
            // Nothing shows a dismissed lifecycle's answer, or offers its
            // conflict, again.
            self.lifecycles.remove(&id);
            self.reviews.remove(&id);
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
        self.locks.rows_read(&root, &result);
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
            self.locks.forget(&root);
            self.pending.remove(&root);
            self.generations.next(ViewScope::Operations(root));
        }
    }
}

/// What an accepted request offers to stop it (R-1, R-15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StopOffer {
    /// Stop at a safe point.
    Offered,
    /// Stop was pressed; the request has not ended yet.
    Stopping,
    NotOffered,
}

/// What a share or a post is, for the words of its stages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Progressing {
    Share,
    Post,
}

impl Progressing {
    /// A share (`item sync`, `repo sync`) or a post (`comment add`).
    pub(crate) fn of(intent: &Intent) -> Option<Self> {
        match intent {
            Intent::ItemSync { .. } | Intent::RepoSync { .. } => Some(Self::Share),
            Intent::CommentAdd { .. } => Some(Self::Post),
            _ => None,
        }
    }
}

impl ShareTarget {
    /// Which share `intent` is, if it is one.
    pub(crate) fn of(intent: &Intent) -> Option<Self> {
        match intent {
            Intent::ItemSync { .. } => Some(Self::Item),
            Intent::RepoSync { .. } => Some(Self::Main),
            _ => None,
        }
    }
}

/// A stage's words for a share or a post (design, Stage text), naming the
/// publication remote `remote` while connecting. `None` for a stage the
/// design gives no words: a share does not write. A lifecycle's merge and
/// sharing name the primary branch, `primary` when known (R-35).
pub(crate) fn stage_text(
    kind: Progressing,
    stage: ProgressStage,
    remote: Option<&str>,
    primary: Option<&str>,
) -> Option<String> {
    let words = match (stage, kind) {
        (ProgressStage::Accepted | ProgressStage::Observing, Progressing::Share) => {
            "Getting ready…"
        }
        (ProgressStage::Accepted | ProgressStage::Observing, Progressing::Post) => {
            "Getting a private copy ready…"
        }
        (ProgressStage::Writing, Progressing::Share) => return None,
        (ProgressStage::Writing, Progressing::Post) => "Saving…",
        (ProgressStage::Connecting, _) => {
            return Some(match remote {
                Some(remote) => format!("Connecting to {remote}…"),
                None => "Connecting…".to_owned(),
            });
        }
        (ProgressStage::Fetching, _) => "Bringing in changes…",
        (ProgressStage::UpdatingLocal, _) => "Merging changes…",
        (ProgressStage::Pushing, _) => "Sending your saved changes…",
        (ProgressStage::Verifying, _) => "Checking it arrived…",
        (ProgressStage::Indexing, _) => "Updating the list…",
        (ProgressStage::Recording, _) => "Marking it closed…",
        (ProgressStage::Integrating, _) => {
            return Some(format!("Merging into {}…", primary_name(primary)));
        }
        (ProgressStage::Publishing, _) => {
            return Some(format!("Sharing {}…", primary_name(primary)));
        }
        (ProgressStage::CleaningUp, _) => "Tidying up…",
        (ProgressStage::Stopping, _) => "Stopping at a safe point…",
    };
    Some(words.to_owned())
}

/// What a running share or post says: "Sharing · {stage}" or
/// "Posting · {stage}". Before its first stage, or at one with no words,
/// it says it is getting ready.
pub(crate) fn progress_words(
    kind: Progressing,
    stage: Option<ProgressStage>,
    remote: Option<&str>,
) -> String {
    let prefix = match kind {
        Progressing::Share => "Sharing",
        Progressing::Post => "Posting",
    };
    let words = stage
        .and_then(|stage| stage_text(kind, stage, remote, None))
        .or_else(|| stage_text(kind, ProgressStage::Accepted, remote, None))
        .unwrap_or_default();
    format!("{prefix} · {words}")
}

/// A stopped share's words (design, Stop at a safe point): whether part
/// of its work finished.
pub(crate) fn stopped_text(completed: bool) -> &'static str {
    if completed {
        STOPPED_AFTER_PART
    } else {
        STOPPED_BEFORE_SENDING
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
    /// The request this session holds by the request ID `action` names;
    /// `id` itself when it names none.
    fn named(&self, id: DesktopRequestId, action: &RecoveryAction) -> Option<DesktopRequestId> {
        match action.arguments.get("request_id") {
            None => Some(id),
            Some(named) => {
                let named = named.as_str()?;
                self.requests
                    .iter()
                    .find(|request| request.request_id.to_string() == named)
                    .map(|request| request.id)
            }
        }
    }

    /// R-78: whether the request `id` is never sent again, a later context
    /// request of its item having been sent (`Intent::kept_from_again_by`):
    /// a release once the item was prepared again, a prepare once its
    /// context was released, unless the library refused that release,
    /// touching nothing (owner, 2026-10-28). "Later" is the order the
    /// worker queued attempts in, as their `Queued` events came. The worker
    /// refuses it too.
    pub(crate) fn not_sent_again(&self, id: DesktopRequestId) -> bool {
        let (Some(request), Some(at)) = (self.request(id), self.sent.get(&id)) else {
            return false;
        };
        self.requests.iter().any(|later| {
            let refused = self
                .answers
                .get(&later.id)
                .is_some_and(Refusals::all_refused);
            self.sent.get(&later.id).is_some_and(|sent| sent > at)
                && request.intent.kept_from_again_by(&later.intent, refused)
        })
    }

    /// Whether the request `id`'s last answer named a pending operation of
    /// its own, offered for Abandon, and the operation area lists it for
    /// the request's repository (Task 17 fix 2).
    pub(crate) fn pending_row_listed(&self, id: DesktopRequestId) -> bool {
        let Some(request) = self.request(id) else {
            return false;
        };
        let RequestState::Finished(_, _, _, recovery, _) = &request.state else {
            return false;
        };
        let Some(operation) = recovery
            .iter()
            .find(|action| action.action == RecoveryActionKind::OperationAbandon)
            .and_then(|action| action.operation_id.as_deref())
        else {
            return false;
        };
        request
            .intent
            .root()
            .and_then(Path::to_str)
            .and_then(|root| self.pending.get(root))
            .and_then(ListState::rows)
            .is_some_and(|list| {
                list.items
                    .iter()
                    .any(|row| row.operation_id.as_deref() == Some(operation))
            })
    }

    /// For a comment's post, whether its last answer saved it; `None` for
    /// any other request, or a post that has not answered.
    pub(crate) fn post_saved(&self, id: DesktopRequestId) -> Option<bool> {
        let request = self.request(id)?;
        if !matches!(request.intent, Intent::CommentAdd { .. }) {
            return None;
        }
        match &request.state {
            RequestState::Finished(_, _, effects, ..) => {
                Some(effects.checkpoint == CheckpointEffect::Committed)
            }
            _ => None,
        }
    }

    /// Whether the recovery action `action` of the request `id` could
    /// complete a comment post that is not saved: Retry, Resume or Prepare
    /// again. The operation area never offers one; the post's composer is
    /// where it is sent again (decision 26). Abandon stays. The library
    /// does attach Resume to a pending operation's answer
    /// (`mutation/outcome.rs`, `DiscoveryPending` and the resumable
    /// operations it lists), so this is no mere defence.
    pub(crate) fn hides_recovery(&self, id: DesktopRequestId, action: &RecoveryAction) -> bool {
        matches!(
            recovery_control(action.action),
            RecoveryControl::Retry | RecoveryControl::Resume | RecoveryControl::Prepare
        ) && self.post_saved(id) == Some(false)
    }

    /// A recovery action's words on the request `id`: a saved comment's
    /// Retry only shares it, so it reads Retry sharing (decision 26).
    pub(crate) fn recovery_words(
        &self,
        id: DesktopRequestId,
        action: &RecoveryAction,
    ) -> &'static str {
        if recovery_control(action.action) == RecoveryControl::Retry
            && self.post_saved(id) == Some(true)
        {
            RETRY_SHARING
        } else {
            recovery_label(action.action)
        }
    }

    /// Whether `action`, offered on the request `id`, is a Retry that R-78
    /// forbids: the operation area says so.
    pub(crate) fn retry_not_sent_again(
        &self,
        id: DesktopRequestId,
        action: &RecoveryAction,
    ) -> bool {
        recovery_control(action.action) == RecoveryControl::Retry
            && self
                .named(id, action)
                .is_some_and(|named| self.not_sent_again(named))
    }

    /// What the recovery action `action`, offered on the request `id`,
    /// runs; `None` when D1 does not offer it, it names nothing this
    /// session can act on, or it is a Retry that R-78 forbids.
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
        let named = || self.named(id, action);
        // Decision 26: nothing here completes an unsaved post.
        if self.hides_recovery(id, action) {
            return None;
        }
        Some(match recovery_control(action.action) {
            RecoveryControl::IndexRefresh => {
                RecoveryStep::Submit(Intent::IndexRefresh { root: root()? })
            }
            RecoveryControl::IndexRebuild => {
                RecoveryStep::Submit(Intent::IndexRebuild { root: root()? })
            }
            RecoveryControl::Retry => {
                let named = named()?;
                if self.not_sent_again(named) {
                    return None;
                }
                RecoveryStep::Retry(named)
            }
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

/// Whether a pending row is a document promotion's or a ticket closure's
/// parent (L2), which the library lists as remote work.
fn is_lifecycle(operation: &OperationDto) -> bool {
    matches!(
        operation.action,
        OperationAction::Promote | OperationAction::Close
    )
}

/// Whether a pending row is a synchronization.
fn is_synchronization(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Remote
        || matches!(
            operation.action,
            OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
        )
}

/// Whether a pending row is remote work a resume takes up, stopping at
/// its safe points (decision 11): a remote item or repository
/// synchronization. A local-context synchronization, which has no remote,
/// is not remote work.
///
/// No row the library lists is a comment's publication waiting to be
/// resumed: the synchronization that publishes a comment is listed with
/// no next action (`read/status.rs`, `remote_standing`), and the
/// comment's `submit_comment` row is listed only while its local work is
/// incomplete or waits for its registration, when a resume does that
/// local work, ignoring its token; a resume takes up a publication only
/// once that row has completed, when it is no longer listed
/// (`bind/resume.rs`, `found_comment`).
fn takes_up_remote_work(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Remote
        && matches!(
            operation.action,
            OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
        )
}

/// Whether a pending row is a synchronization with no remote to publish
/// to: the library lists it as a local operation of the remote name
/// (`OperationDto::action`).
fn local_synchronization(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Local
        && matches!(
            operation.action,
            OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
        )
}

/// What a pending row offers for Resume.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ResumeOffer {
    /// An operation waiting to be resumed: Resume runs it.
    Offered,
    /// Shown disabled, with why.
    Unavailable(&'static str),
    NotOffered,
}

/// Resume is offered when the row waits to be resumed and is a local
/// command (a comment's incomplete `submit_comment` and a `context
/// sync-local` among them), a remote item or repository synchronization,
/// or a local-context or primary synchronization with no remote (D5 B6,
/// E-5), by its operation ID (decision 11). Any other synchronization
/// shows it unavailable. The library lists no comment publication waiting
/// to be resumed (`takes_up_remote_work`), so none is offered. A
/// promotion's or closure's parent (L2) is no synchronization: it is not
/// offered here, but by its own row, whose resume opens the lifecycle
/// dialog (D5 Part C, `view/operations/resume.rs`).
pub(crate) fn resume_offer(operation: &OperationDto) -> ResumeOffer {
    if operation.next_action != Some(OperationNextAction::Resume) {
        return ResumeOffer::NotOffered;
    }
    if is_lifecycle(operation) {
        return ResumeOffer::NotOffered;
    }
    if takes_up_remote_work(operation) || local_synchronization(operation) {
        return match operation.operation_id {
            Some(_) => ResumeOffer::Offered,
            None => ResumeOffer::NotOffered,
        };
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
        // `remote.history` has no button: repair runs in `manyhands-cli
        // remote history` until the repair sheet (vh-ez4y9). Inspect opens
        // the repository's details; the guidance text names the command.
        RecoveryActionKind::RepoInspect | RecoveryActionKind::RemoteHistory => {
            RecoveryControl::Inspect
        }
        RecoveryActionKind::RepoIdentitySet => RecoveryControl::IdentitySet,
        // The action names the host but not the key to pin: the Hosts
        // page checks the host, and offers Approve or Replace for the key
        // it presents.
        RecoveryActionKind::HostApprove | RecoveryActionKind::HostReplace => {
            RecoveryControl::CheckHost
        }
    }
}

/// A saved comment's Retry, which only shares it (decision 26).
pub(crate) const RETRY_SHARING: &str = "Retry sharing";

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
        RecoveryActionKind::RepoInspect | RecoveryActionKind::RemoteHistory => "Inspect repository",
    }
}

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
