//! What the lifecycle dialog says once a run is sent (D5 design, A2
//! Running; A3 Outcomes): each step's state, lit from L3's stages while it
//! runs and from the answer once it ends; how it ended; the outcome lines,
//! which report the merge, the sharing and the tidy-up apart; and the way
//! out in words where it has no button yet. The dialog's drawing and its
//! buttons are `lifecycle/running.rs`.
//!
//! - **Running.** A stage lights its step (L3): `connecting`, `fetching`,
//!   `updating_local` and `pushing` Share with origin; `recording` Mark as
//!   closed; `integrating` Merge into main; `publishing` Share main;
//!   `cleaning_up` Tidy up. The steps before it are done. Save is done
//!   before the run starts. `integrating` comes only when a merge runs
//!   (R-14), so an already-contained merge is done once a later stage comes.
//! - **Ending.** Success or no change is Done: a local-only run whose
//!   tidy-up completed among them (R-7). `cancelled` is Stopped, `busy` still
//!   running (P22) unless main was already reached with sharing or the
//!   tidy-up pending (Failed), and any other answer Failed: a remote publication or
//!   tidy-up left pending among them.
//! - **Steps at the end.** What the answer reports done is done, and so is
//!   every step before the last stage this dialog saw. The first step not
//!   done failed, or for a stop is not done; the rest are not done.
//!
//! Nothing here holds a title, body or path: only step titles, which may
//! name a remote or the main branch, and fixed words.

use manyhands::{
    repository::{
        ItemDtoKind, LifecycleBlocker, LifecycleCleanupPendingReason, LifecycleCleanupState,
        LifecycleIntegrationKind, LifecycleMutationDto, LifecyclePublicationPendingReason,
        LifecyclePublicationState, LifecycleWayOut, MutationDataDto, ProgressStage,
    },
    results::{
        CleanupEffect, Effects, Envelope, IntegrationEffect, Outcome, PublicationEffect, ResultCode,
    },
};

use super::confirm::{KEEP_LOCAL, MARK_CLOSED, SAVE_STEP, Step, TIDY_UP};
use crate::desktop::{
    model::{
        operations::{Progressing, stage_text},
        publication::{HOST_CODES, KEY_CODES},
    },
    services::requests::Intent,
};

pub(crate) const STOP: &str = "Stop at a safe point";
pub(crate) const STOPPING: &str = "Stopping\u{2026}";
pub(crate) const ENDING_DONE: &str = "Done";
pub(crate) const ENDING_FAILED: &str = "Failed";
pub(crate) const ENDING_STOPPED: &str = "Stopped at a safe point";
pub(crate) const STATE_DONE: &str = "Done";
pub(crate) const STATE_RUNNING: &str = "Running";
pub(crate) const STATE_NOT_STARTED: &str = "Not started";
pub(crate) const STATE_NOT_DONE: &str = "Not done";
pub(crate) const STATE_FAILED: &str = "Failed";
/// A stop at a safe point answered `cancelled` (MH-COLLAB-006).
pub(crate) const CANCELLATION_FINISHED: &str =
    "The cancellation finished: nothing after the completed steps ran.";
/// Stop was pressed, and the run finished first.
pub(crate) const FINISHED_BEFORE_STOP: &str = "It finished before it could stop.";
/// The merge into the primary branch, named `primary` (R-35).
pub(crate) fn merged(primary: &str) -> String {
    format!("Merged into {primary}")
}
/// Nothing reached the primary branch, named `primary`.
pub(crate) fn not_merged(primary: &str) -> String {
    format!("Nothing was merged into {primary}.")
}
pub(crate) const SHARED: &str = "Shared";
pub(crate) const ALREADY_SHARED: &str = "Already shared";
pub(crate) const SHARING_PENDING: &str = "Sharing pending (no remote)";
pub(crate) const TIDIED_UP: &str = "Tidied up";
/// P18: the run stopped with no parent record, and the library deleted its
/// request: a new prepare is what follows.
pub(crate) const NO_RECORD: &str = "This run stopped before it changed anything, and nothing of it is kept. Prepare again to see what it would do now.";
/// The run was taken off the queue before it ran.
pub(crate) const REMOVED: &str = "Cancelled before it ran. Nothing was changed.";
/// The run's entry was dismissed from the operation area, and its answer
/// with it.
pub(crate) const DISMISSED: &str = "This run was dismissed from Activity.";
pub(crate) const UNLOCK: &str = "Unlock\u{2026}";
pub(crate) const OPEN_SHARING_AND_KEYS: &str = "Open Sharing & keys";
pub(crate) const CHECK_HOST: &str = "Check host";
pub(crate) const OPEN_REPOSITORY_DETAILS: &str = "Open repository details";

/// A step of the run, in the library's order (R-8).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StepKey {
    Save,
    /// Share with origin, or Keep on this computer.
    ShareContext,
    MarkClosed,
    Merge,
    SharePrimary,
    TidyUp,
}

/// Where a step stands.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepState {
    NotStarted,
    Running,
    Done,
    /// The run ended short of it.
    NotDone,
    /// The run failed here.
    Failed,
}

impl StepState {
    pub(crate) fn words(self) -> &'static str {
        match self {
            Self::NotStarted => STATE_NOT_STARTED,
            Self::Running => STATE_RUNNING,
            Self::Done => STATE_DONE,
            Self::NotDone => STATE_NOT_DONE,
            Self::Failed => STATE_FAILED,
        }
    }
}

/// The step a stage lights (L3); `None` for a stage before the steps, or
/// one no lifecycle reports.
pub(crate) fn stage_step(stage: ProgressStage) -> Option<StepKey> {
    use ProgressStage::{
        Accepted, CleaningUp, Connecting, Fetching, Indexing, Integrating, Observing, Publishing,
        Pushing, Recording, Stopping, UpdatingLocal, Verifying, Writing,
    };
    match stage {
        Connecting | Fetching | UpdatingLocal | Pushing => Some(StepKey::ShareContext),
        Recording => Some(StepKey::MarkClosed),
        Integrating => Some(StepKey::Merge),
        Publishing => Some(StepKey::SharePrimary),
        CleaningUp => Some(StepKey::TidyUp),
        Accepted | Observing | Writing | Verifying | Indexing | Stopping => None,
    }
}

/// The steps of a run whose preview this dialog does not hold (opened
/// again from the operation area, or a Continue or Resume): by the item's
/// kind, naming the main branch `primary` and the publication remote the
/// workspace knows (`remote`: `None` when it knows none, `Some(None)` when the
/// repository has none). Local-only when it has none, or the answer says main
/// waits for one (review M-5).
pub(crate) fn steps_by_kind(
    kind: ItemDtoKind,
    answer: Option<&LifecycleMutationDto>,
    primary: &str,
    remote: Option<Option<&str>>,
) -> Vec<Step> {
    let local = remote == Some(None)
        || answer.is_some_and(|data| {
            data.publication_pending == Some(LifecyclePublicationPendingReason::NoPublicationRemote)
        });
    let named = remote.flatten();
    let mut steps = vec![Step::new(StepKey::Save, SAVE_STEP, "")];
    steps.push(match (local, named) {
        (true, _) => Step::new(StepKey::ShareContext, KEEP_LOCAL, ""),
        (false, Some(remote)) => {
            Step::new(StepKey::ShareContext, format!("Share with {remote}"), "")
        }
        (false, None) => Step::new(StepKey::ShareContext, "Share with the remote", ""),
    });
    if kind == ItemDtoKind::Ticket {
        steps.push(Step::new(StepKey::MarkClosed, MARK_CLOSED, ""));
    }
    steps.push(Step::new(
        StepKey::Merge,
        format!("Merge into {primary}"),
        "",
    ));
    if !local {
        let title = match named {
            Some(remote) => format!("Share {primary} with {remote}"),
            None => format!("Share {primary}"),
        };
        steps.push(Step::new(StepKey::SharePrimary, title, ""));
    }
    steps.push(Step::new(StepKey::TidyUp, TIDY_UP, ""));
    steps
}

/// The states while it runs: Save done, the steps before the last stage
/// seen (`reached`) done, its step running, the rest not started.
pub(crate) fn running_states(keys: &[StepKey], reached: Option<StepKey>) -> Vec<StepState> {
    keys.iter()
        .map(|key| match reached {
            _ if *key == StepKey::Save => StepState::Done,
            Some(reached) if *key < reached => StepState::Done,
            Some(reached) if *key == reached => StepState::Running,
            _ => StepState::NotStarted,
        })
        .collect()
}

/// How a run ended.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ending {
    Done,
    Failed,
    Stopped,
    /// `busy`: another call of it still holds the item (P22).
    StillRunning,
}

impl Ending {
    pub(crate) fn of(envelope: &Envelope<MutationDataDto>) -> Self {
        match (envelope.outcome, envelope.code) {
            (Outcome::Success | Outcome::Noop, _) => Self::Done,
            (_, ResultCode::Cancelled) => Self::Stopped,
            (_, ResultCode::Busy) if Self::reached_main_pending(envelope) => Self::Failed,
            (_, ResultCode::Busy) => Self::StillRunning,
            _ => Self::Failed,
        }
    }

    /// A `busy` answer of a run that already merged into main and left its
    /// sharing with the remote pending (a push refused because another
    /// process holds the repository): nothing else is running it, and
    /// what is pending is Failed (A3, R-7), not a run in flight. A
    /// repository with no remote has no sharing to leave pending, so its
    /// `busy` is still another call holding the item (P22).
    fn reached_main_pending(envelope: &Envelope<MutationDataDto>) -> bool {
        lifecycle_data(envelope).is_some_and(|data| {
            data.integration.is_some()
                && data.publication == Some(LifecyclePublicationState::Pending)
                && data.publication_pending
                    != Some(LifecyclePublicationPendingReason::NoPublicationRemote)
        })
    }

    pub(crate) fn words(self) -> &'static str {
        match self {
            Self::Done => ENDING_DONE,
            Self::Failed => ENDING_FAILED,
            Self::Stopped => ENDING_STOPPED,
            Self::StillRunning => super::STILL_RUNNING,
        }
    }
}

/// The lifecycle data of an answer.
pub(crate) fn lifecycle_data(
    envelope: &Envelope<MutationDataDto>,
) -> Option<&LifecycleMutationDto> {
    match &envelope.data {
        Some(MutationDataDto::Lifecycle(data)) => Some(data),
        _ => None,
    }
}

/// How far an ended run got, from its answer and the last stage seen.
pub(crate) struct Reach {
    /// Main holds the item: its data says how, or its effects report a
    /// merge with a tidy-up asked for, which only follows one into main
    /// (before it, an integration is the context's own).
    pub integrated: bool,
    pub recorded: bool,
    pub shared: bool,
    pub tidied: bool,
    reached: Option<StepKey>,
    /// The step the answer's blockers stand in the way of (review M-1).
    blocked_at: Option<StepKey>,
}

impl Reach {
    pub(crate) fn of(envelope: &Envelope<MutationDataDto>, reached: Option<StepKey>) -> Self {
        let data = lifecycle_data(envelope);
        let effects = &envelope.effects;
        let past = |key: StepKey| reached.is_some_and(|reached| reached > key);
        let integrated = data.is_some_and(|data| data.integration.is_some())
            || (effects.integration == IntegrationEffect::Complete
                && effects.cleanup != CleanupEffect::NotRequested)
            || past(StepKey::Merge);
        let shared = data.is_some_and(|data| {
            matches!(
                data.publication,
                Some(
                    LifecyclePublicationState::Published
                        | LifecyclePublicationState::AlreadyCurrent
                )
            )
        }) || (integrated
            && effects.cleanup != CleanupEffect::NotRequested
            && matches!(
                effects.publication,
                PublicationEffect::Published | PublicationEffect::Current
            ))
            || past(StepKey::SharePrimary);
        Self {
            integrated,
            recorded: data.is_some_and(|data| data.recording.is_some())
                || effects.checkpoint == manyhands::results::CheckpointEffect::Committed
                || integrated
                || past(StepKey::MarkClosed),
            shared,
            tidied: data.is_some_and(|data| data.cleanup == Some(LifecycleCleanupState::Complete))
                || effects.cleanup == CleanupEffect::Complete,
            reached,
            blocked_at: data.and_then(|data| blocked_step(&data.blockers)),
        }
    }

    fn done(&self, key: StepKey, local: bool) -> bool {
        match key {
            StepKey::Save => true,
            StepKey::ShareContext => {
                self.recorded
                    || self.integrated
                    || self
                        .reached
                        .is_some_and(|reached| reached > StepKey::ShareContext)
            }
            StepKey::MarkClosed => self.recorded,
            StepKey::Merge => self.integrated,
            // Local-only, main waits for a remote: nothing to do now.
            StepKey::SharePrimary => self.shared || (local && self.integrated),
            StepKey::TidyUp => self.tidied,
        }
    }

    /// Each step's state once the run ended as `ending`.
    pub(crate) fn states(&self, keys: &[StepKey], ending: Ending, local: bool) -> Vec<StepState> {
        // Where the blockers stand in the way, that step failed; with none,
        // the first step not done.
        let at = self.blocked_at.filter(|at| keys.contains(at));
        let mut failed = false;
        keys.iter()
            .map(|key| {
                if self.done(*key, local) || ending == Ending::Done {
                    StepState::Done
                } else if !failed && ending == Ending::Failed && at.is_none_or(|at| at == *key) {
                    failed = true;
                    StepState::Failed
                } else {
                    StepState::NotDone
                }
            })
            .collect()
    }
}

/// The step a stop's blockers stand in the way of (review M-1): a dirty
/// main or a refused merge, the merge; a conflict on the item's own copy,
/// its sharing. The earliest, when they name more than one; `None` when
/// none of them names a step.
fn blocked_step(blockers: &[LifecycleBlocker]) -> Option<StepKey> {
    blockers
        .iter()
        .filter_map(|blocker| match blocker {
            LifecycleBlocker::PrimaryDirty | LifecycleBlocker::IntegrationConflict => {
                Some(StepKey::Merge)
            }
            LifecycleBlocker::ContextConflicted | LifecycleBlocker::ConflictPending => {
                Some(StepKey::ShareContext)
            }
            _ => None,
        })
        .min()
}

/// The outcome lines (A3): the merge, the sharing and the tidy-up, each
/// apart; before main, that nothing was merged.
pub(crate) fn outcome_lines(
    envelope: &Envelope<MutationDataDto>,
    reach: &Reach,
    primary: &str,
) -> Vec<String> {
    if !reach.integrated {
        return vec![not_merged(primary)];
    }
    let data = lifecycle_data(envelope);
    let effects: &Effects = &envelope.effects;
    let mut lines = vec![match data.and_then(|data| data.integration) {
        Some(LifecycleIntegrationKind::AlreadyContained) => format!("Already in {primary}"),
        _ => merged(primary),
    }];
    let publication = match data.and_then(|data| data.publication) {
        Some(LifecyclePublicationState::Published) => Some(SHARED.to_owned()),
        Some(LifecyclePublicationState::AlreadyCurrent) => Some(ALREADY_SHARED.to_owned()),
        Some(LifecyclePublicationState::Pending) => {
            match data.and_then(|data| data.publication_pending) {
                Some(LifecyclePublicationPendingReason::NoPublicationRemote) => {
                    Some(SHARING_PENDING.to_owned())
                }
                reason => Some(format!(
                    "Not shared: {}",
                    publication_reason(reason, primary)
                )),
            }
        }
        None => match effects.publication {
            PublicationEffect::Published => Some(SHARED.to_owned()),
            PublicationEffect::Current => Some(ALREADY_SHARED.to_owned()),
            PublicationEffect::NotRequested | PublicationEffect::Pending => None,
        },
    };
    lines.extend(publication);
    let cleanup = match data.and_then(|data| data.cleanup) {
        Some(LifecycleCleanupState::Complete) => Some(TIDIED_UP.to_owned()),
        Some(LifecycleCleanupState::Pending) => Some(format!(
            "Not tidied up yet: {}",
            cleanup_reason(data.and_then(|data| data.cleanup_pending), primary)
        )),
        None => match effects.cleanup {
            CleanupEffect::Complete => Some(TIDIED_UP.to_owned()),
            CleanupEffect::Pending => Some("Not tidied up yet.".to_owned()),
            CleanupEffect::NotRequested => None,
        },
    };
    lines.extend(cleanup);
    lines
}

/// Why sharing is pending, in words; `primary` names the primary branch.
fn publication_reason(reason: Option<LifecyclePublicationPendingReason>, primary: &str) -> String {
    use LifecyclePublicationPendingReason as R;
    let words = match reason {
        Some(R::NoPublicationRemote) => "this repository has no remote",
        Some(R::Conflict) => return format!("{primary} on the remote has changes that conflict"),
        Some(R::Rejected) => "the remote refused it",
        Some(R::Transport) => "the remote could not be reached",
        Some(R::Identity) => "your name and email are not set",
        Some(R::Busy) => "another operation holds the repository",
        Some(R::Yielding) => "a check for others' changes is running",
        Some(R::RecoveryRequired) => "the repository needs attention",
        Some(R::Cancelled) => "it was stopped",
        None => "it did not finish",
    };
    words.to_owned()
}

/// Why tidying up is pending, in words; `primary` names the primary branch.
pub(crate) fn cleanup_reason(
    reason: Option<LifecycleCleanupPendingReason>,
    primary: &str,
) -> String {
    use LifecycleCleanupPendingReason as R;
    let words = match reason {
        Some(R::AwaitingPublication) => return format!("{primary} is not shared yet"),
        Some(R::ContextChanged) => "the working copy changed",
        Some(R::IgnoredFiles) => "the working copy holds ignored files",
        Some(R::RemoteContextChanged) => "the shared branch changed",
        Some(R::RemoteDeletionTransport) => "the remote could not be reached",
        Some(R::RemoteDeletionRejected) => "the remote refused to delete the shared branch",
        Some(R::RemoteDeletionConfiguration) => "the remote is not set up to delete it",
        Some(R::RemoteDeletionKey) => "no key could be used",
        Some(R::RemoteDeletionUnlock) => "the key must be unlocked",
        Some(R::RemoteDeletionHost) => "the host must be approved",
        Some(R::RecoveryRequired) => "the repository needs attention",
        Some(R::Cancelled) => "it was stopped",
        None => "it did not finish",
    };
    words.to_owned()
}

/// A way out of an execute result in words, for one with no button here;
/// `None` for one the dialog offers as a button (Retry, Resume, Continue,
/// Share main). Repair locally… and Resolve conflict… are buttons beside
/// their words (`repair.rs`), and so are Republish… and Stop and
/// republish… (`republish.rs`).
pub(crate) fn guidance(
    way_out: &LifecycleWayOut,
    item: &'static str,
    primary: &str,
) -> Option<String> {
    Some(match way_out {
        LifecycleWayOut::RepairLocally { .. } => {
            format!("Bring {primary} into this {item}'s working copy, then retry.")
        }
        LifecycleWayOut::ResolveChildConflict { .. } => {
            format!("This {item}'s shared copy has a conflict to resolve first, then retry.")
        }
        LifecycleWayOut::Republish { .. } => {
            format!("This {item}'s shared copy was deleted: share it again, then retry.")
        }
        LifecycleWayOut::CancelThenSendAgain { .. } => {
            format!("Stop this run first, then share this {item} again.")
        }
        LifecycleWayOut::RepairOutside => {
            format!("Fix the conflict with {primary} outside Manyhands, then retry.")
        }
        LifecycleWayOut::PollOnce | LifecycleWayOut::SaveItem { .. } | LifecycleWayOut::Inspect => {
            return super::prepare::guidance(way_out, item, primary);
        }
        LifecycleWayOut::SendAgain
        | LifecycleWayOut::RetryRequest { .. }
        | LifecycleWayOut::ResumeOperation { .. }
        | LifecycleWayOut::SharePrimary => return None,
    })
}

/// A credential stop, with D4's recovery (A3).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Credential {
    /// Unlock… and Open Sharing & keys.
    Locked,
    /// Open Sharing & keys, then Retry.
    Key,
    /// Check host, then Retry.
    Host,
}

impl Credential {
    pub(crate) fn of(code: ResultCode) -> Option<Self> {
        // D4's routing of a share's credential codes (`publication.rs`).
        match code {
            ResultCode::UnlockRequired | ResultCode::UnlockFailed => Some(Self::Locked),
            code if KEY_CODES.contains(&code) => Some(Self::Key),
            code if HOST_CODES.contains(&code) => Some(Self::Host),
            _ => None,
        }
    }
}

/// What a running lifecycle's operation area entry says: "Closing · …" or
/// "Merging · …", by its latest stage, naming the primary branch `primary`
/// when known. `None` for any other request.
pub(crate) fn progress_words(
    intent: &Intent,
    stage: Option<ProgressStage>,
    primary: Option<&str>,
) -> Option<String> {
    let prefix = match intent {
        Intent::TicketClose { .. } => "Closing",
        Intent::DocumentPromote { .. } => "Merging",
        _ => return None,
    };
    // A lifecycle's stages have the words a share's have.
    let words = stage
        .and_then(|stage| stage_text(Progressing::Share, stage, None, primary))
        .or_else(|| stage_text(Progressing::Share, ProgressStage::Accepted, None, primary))
        .unwrap_or_default();
    Some(format!("{prefix} \u{b7} {words}"))
}

/// The item's ID a lifecycle request names, for its entry.
pub(crate) fn item_of(intent: &Intent) -> Option<String> {
    match intent {
        Intent::TicketClose { id, .. } | Intent::DocumentPromote { id, .. } => {
            Some(super::short_id(&id.to_string()))
        }
        _ => None,
    }
}
