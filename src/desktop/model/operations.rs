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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use manyhands::{
    drafts::{DraftError, DraftErrorKind},
    repository::{HostObservationDto, MutationDataDto, OperationId, OperationListDto},
    results::{Envelope, Outcome, ResultCode},
};

mod lifecycle;
mod outcomes;
mod pending;
mod polls;
mod progress;
mod quit_wait;
mod recovery;
mod retire;
pub(crate) use retire::{Entered, RetireEvent, RetiringKey};
mod review;

pub(crate) use pending::{LifecycleRow, ResumeOffer, abandon_offered, resume_offer};
use pending::{check_now_left_active, takes_up_remote_work};
pub(crate) use polls::{record_note_words, resume_offer_among};
pub(crate) use progress::{Progressing, progress_words, stage_text, stage_words, stopped_text};
#[cfg(test)]
pub(crate) use recovery::recovery_label;
pub(crate) use recovery::{RecoveryControl, RecoveryStep, parse_authority, recovery_control};
pub(crate) use review::{Review, ReviewOf};

#[cfg(test)]
pub(crate) use outcomes::main_branch_deleted;
pub(crate) use outcomes::{
    ANOTHER_SHARE_HOLDS, CONFLICT_BANNER, HISTORY_UNRESOLVED_BLOCKER, HISTORY_UNRESOLVED_GUIDANCE,
    may_be_waiting_for, primary_name, primary_subject, share_text,
};
#[cfg(test)]
pub(crate) use outcomes::{
    BRANCH_DELETED, CHOOSE_A_KEY, CURRENT, KEY_LOCKED, NEEDS_ATTENTION, NEEDS_IDENTITY,
    NOWHERE_TO_SHARE, SHARED, check_host_text,
};

use super::{
    lifecycle_lock::LifecycleLocks,
    list_state::{ListState, Rows},
    publication::{ShareAttempt, ShareTarget, SyncResult, sync_outcome},
    stamp::{Generations, RepoKey},
};
use crate::desktop::services::{
    DraftStoreUnavailable,
    inflight::Report,
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
    /// How long quitting has waited for a job (Part D).
    quit_wait: quit_wait::QuitWait,
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
    /// Where each share's record stood, and whether its answer was a Give
    /// up's, as the drain noted them before the share's `Finished` (F2,
    /// R-8). A retry clears it.
    share_attempts: HashMap<DesktopRequestId, (RecordState, bool)>,
    /// Each finished share's outcome, classified once when it finished
    /// from `share_attempts` (F2). A retry clears it.
    share_results: HashMap<DesktopRequestId, SyncResult>,
    /// Each finished lifecycle's last answer, which the lifecycle dialog
    /// shows and takes its way out from (D5, A3): a promotion's or a
    /// closure's, or a resume's that answered with lifecycle data.
    lifecycles: HashMap<DesktopRequestId, Envelope<MutationDataDto>>,
    /// The lifecycle lock's own state (D5 A2; R-21, R-22).
    locks: LifecycleLocks,
    /// The operation each finished request's last answer stopped on with a
    /// conflict, whose sheet its entry offers (D5 B5, `review`).
    reviews: HashMap<DesktopRequestId, Review>,
    /// The operations of this desktop's own automatic polls (D6 Task 5):
    /// their pending rows are not offered Resume (`polls`).
    automatic_polls: HashSet<OperationId>,
    /// What the in-flight record reported of itself, until dismissed
    /// (`polls`).
    record_notes: Vec<Report>,
    /// Retirement answers and the operations running now (`retire`).
    retirements: retire::Retirements,
    /// The sequence number of the next retirement event
    /// (`retire_events`); events are never pruned, so it only grows.
    next_retire_event: u64,
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
    /// Where a finished New folder made a folder or a Move put a document,
    /// by repository and path from the repository root: the Documents tree
    /// opens the folders above each, so the result is visible (R-27).
    pub reveal: Vec<(RepoKey, String)>,
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

    /// Where the request `id`'s last attempt stands in the order every attempt
    /// was queued in (R-78; Activity's `Group.sent`). `None` for a request this
    /// model never saw queued.
    pub(crate) fn sent(&self, id: DesktopRequestId) -> Option<u64> {
        self.sent.get(&id).copied()
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
                self.share_attempts.remove(&request.id);
                self.share_results.remove(&request.id);
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
                self.progressed(id, &progress);
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
                self.left_running(id);
                // A retirement is sorted (E3) before the request is set.
                let retirement = self.retired(id, &envelope);
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
                    // Classified once, from the attempt the drain noted;
                    // a `Finished` it never noted is the worker's defaults
                    // for a request it never ran (fact 13).
                    let (record, after_give_up) = self
                        .share_attempts
                        .get(&id)
                        .copied()
                        .unwrap_or((RecordState::Unknown, false));
                    if let Some(result) = self.classified(id, record, after_give_up) {
                        self.share_results.insert(id, result);
                    }
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
                    // A retirement changes the items of its repository only
                    // when it removed something (A20).
                    let items = retirement
                        .as_ref()
                        .map_or(request.intent.changes_items(), |answer| answer.changed);
                    Some((root, items, removed))
                });
                // A folder made, or a document moved, wants its place seen.
                let reveal = matches!(
                    envelope.code,
                    ResultCode::Ok | ResultCode::DiscoveryPending | ResultCode::AlreadyApplied
                )
                .then(|| self.request(id))
                .flatten()
                .and_then(|request| match &request.intent {
                    Intent::FolderCreate { root, path } => {
                        Some((root.to_str()?.to_owned(), path.clone()))
                    }
                    Intent::DocumentMove {
                        root,
                        destination_path,
                        ..
                    } => Some((
                        root.to_str()?.to_owned(),
                        destination_path.to_str()?.to_owned(),
                    )),
                    _ => None,
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
                self.changed.reveal.extend(reveal);
                // Attention or the next attempt covers an answer of the
                // table's attention and retry rows, and a retirement that
                // retired: it is neither shown nor counted (R-23).
                // An answer that dismisses its request leaves one event first,
                // so Activity shows it once its request is gone (R-51).
                if let Some(answer) = retirement.as_ref() {
                    self.note_retire_event(id, answer);
                }
                if retirement.is_some_and(|answer| answer.verdict.dismissed()) {
                    self.dismiss(id);
                }
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
                self.left_running(id);
                self.set(id, RequestState::Removed);
            }
            WorkerEvent::NotSent(id, why) => {
                self.stopping.remove(&id);
                self.left_running(id);
                self.set(id, RequestState::NotSent(why));
            }
            // The Hosts page reads it. An observation pins nothing, so no
            // view has anything to read again.
            WorkerEvent::HostObserved(id, envelope) => self.observed = Some((id, envelope)),
            // One worker asks one thing at a time: a prompt still kept
            // when the next comes is one its job no longer waits on.
            WorkerEvent::Prompt(prompt) => self.prompt = Some(prompt),
            // A poll is not a request: the drain takes its attempt before
            // `apply` and hands it to the poll inbox (D6 B1), and starts the
            // cancel task of one the worker was asked to stop (B2).
            WorkerEvent::Polled(..) | WorkerEvent::PreemptPoll { .. } => {}
        }
    }

    /// The request running now, if the model has heard it start: one
    /// worker runs one at a time.
    pub(crate) fn running(&self) -> Option<&DesktopRequest> {
        self.requests
            .iter()
            .find(|request| matches!(request.state, RequestState::Running { .. }))
    }

    /// Whether Stop was pressed on the running request `id`.
    pub(crate) fn is_stopping(&self, id: DesktopRequestId) -> bool {
        self.stopping.contains(&id)
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

    /// Notes where the record of the share `id` stood when its last
    /// attempt finished, and whether that attempt was a Give up's. The
    /// drain calls it before it applies the share's `Finished`, which
    /// classifies the share once from it (F2, R-8).
    pub(crate) fn note_share_attempt(
        &mut self,
        id: DesktopRequestId,
        record: RecordState,
        after_give_up: bool,
    ) {
        self.share_attempts.insert(id, (record, after_give_up));
    }

    /// The outcome of the finished share `id`, classified once when it
    /// finished. `None` for any other request, or a share not finished.
    pub(crate) fn share_result(&self, id: DesktopRequestId) -> Option<&SyncResult> {
        match self.request(id)?.state {
            RequestState::Finished(..) => self.share_results.get(&id),
            _ => None,
        }
    }

    /// The finished share `id`'s last answer classified again under
    /// `record` and `after_give_up`, for the one test that re-classifies
    /// under another record on purpose (E-1).
    #[cfg(test)]
    pub(crate) fn classify_share(
        &self,
        id: DesktopRequestId,
        record: RecordState,
        after_give_up: bool,
    ) -> Option<SyncResult> {
        if !matches!(self.request(id)?.state, RequestState::Finished(..)) {
            return None;
        }
        self.classified(id, record, after_give_up)
    }

    /// The last answer of the share `id`, classified under `record` and
    /// `after_give_up`: the one body of both the classification when a
    /// share finishes and `classify_share`. `None` for a request that is
    /// not a share, or holds no answer.
    fn classified(
        &self,
        id: DesktopRequestId,
        record: RecordState,
        after_give_up: bool,
    ) -> Option<SyncResult> {
        let target = ShareTarget::of(&self.request(id)?.intent)?;
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

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
