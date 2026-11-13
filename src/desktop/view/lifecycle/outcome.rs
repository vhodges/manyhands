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
        ItemDtoKind, LifecycleMutationDto, LifecyclePublicationPendingReason, LifecycleWayOut,
        ProgressStage,
    },
    results::ResultCode,
};

use super::confirm::{KEEP_LOCAL, MARK_CLOSED, SAVE_STEP, Step, TIDY_UP};
// The outcome types, words and lines live in the model so Activity's rows
// can read them without a view module (R-41); each keeps its old path.
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) use crate::desktop::model::activity::lifecycle_lines::{
    ENDING_DONE, ENDING_FAILED, ENDING_STOPPED, SHARED, SHARING_PENDING, STATE_DONE, STATE_FAILED,
    STATE_NOT_DONE, STATE_NOT_STARTED, STATE_RUNNING, TIDIED_UP, merged, not_merged,
};
pub(crate) use crate::desktop::model::activity::lifecycle_lines::{
    Ending, Reach, StepKey, StepState, cleanup_reason, lifecycle_data, outcome_lines,
};
use crate::desktop::{
    model::publication::{HOST_CODES, KEY_CODES},
    services::requests::Intent,
};

pub(crate) const STOP: &str = "Stop at a safe point";
pub(crate) const STOPPING: &str = "Stopping\u{2026}";
/// A stop at a safe point answered `cancelled` (MH-COLLAB-006).
pub(crate) const CANCELLATION_FINISHED: &str =
    "The cancellation finished: nothing after the completed steps ran.";
/// Stop was pressed, and the run finished first.
pub(crate) const FINISHED_BEFORE_STOP: &str = "It finished before it could stop.";
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
        LifecycleWayOut::PollOnce
        | LifecycleWayOut::ResolveHistory
        | LifecycleWayOut::SaveItem { .. }
        | LifecycleWayOut::Inspect => {
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

/// The item's ID a lifecycle request names, for its entry.
pub(crate) fn item_of(intent: &Intent) -> Option<String> {
    match intent {
        Intent::TicketClose { id, .. } | Intent::DocumentPromote { id, .. } => {
            Some(super::short_id(&id.to_string()))
        }
        _ => None,
    }
}
