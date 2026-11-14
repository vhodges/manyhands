//! What the lifecycle dialog and Activity say of a run's outcome (D5
//! design, A3 Outcomes), moved out of the view so the model depends on no
//! view module (Wave 04 Cycle 03b, R-41): each step's state, how a run
//! ended, how far it got (`Reach`), and the outcome lines, which report the
//! merge, the sharing and the tidy-up apart.
//!
//! Nothing here holds a title, body or path: only step titles, which may
//! name a remote or the main branch, and fixed words. The dialog's
//! drawing and its buttons are `view/lifecycle/running.rs`; each item here
//! is re-exported from its old path in `view/lifecycle/outcome.rs`.

use manyhands::{
    repository::{
        LifecycleBlocker, LifecycleCleanupPendingReason, LifecycleCleanupState,
        LifecycleIntegrationKind, LifecycleMutationDto, LifecyclePublicationPendingReason,
        LifecyclePublicationState, MutationDataDto,
    },
    results::{
        CleanupEffect, Effects, Envelope, IntegrationEffect, Outcome, PublicationEffect, ResultCode,
    },
};

/// Continue and Resume wait while the item's lifecycle runs (P22).
pub(crate) const STILL_RUNNING: &str = "Still running\u{2026}";
pub(crate) const ENDING_DONE: &str = "Done";
pub(crate) const ENDING_FAILED: &str = "Failed";
pub(crate) const ENDING_STOPPED: &str = "Stopped at a safe point";
pub(crate) const STATE_DONE: &str = "Done";
pub(crate) const STATE_RUNNING: &str = "Running";
pub(crate) const STATE_NOT_STARTED: &str = "Not started";
pub(crate) const STATE_NOT_DONE: &str = "Not done";
pub(crate) const STATE_FAILED: &str = "Failed";
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
            Self::StillRunning => STILL_RUNNING,
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
