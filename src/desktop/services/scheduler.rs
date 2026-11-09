//! The polling scheduler (D6 design B3 and B6): the entity `Services`
//! holds ([`Scheduler`], in `entity`) and the pure decisions it takes over
//! the model and the in-flight record (R-4, R-31, L-1 to L-6).
//!
//! - [`plan`] picks the operation a due repository's poll is sent under:
//!   the in-flight record's, for the lock holder (L-1, the takeover of a
//!   stale row), else the model's own wait (R-4's same-ID restart after
//!   `Busy` or an interruption), else a new one. `Busy` from an operation
//!   the record does not hold only waits (L-5): nothing here ever sends an
//!   ID that is not the record's or the model's own.
//! - [`start`] writes the record's entry, then submits the poll (R-31).
//! - [`holding`] says why a repository with a stale row is not taken over
//!   yet (L-2).
//! - [`apply`] takes an answer: the record settles, then the model moves
//!   (A4), except a replayed terminal answer older than this launch, which
//!   counts as no attempt (L-1).

mod entity;
mod react;
mod reconfigure;
mod registration;

use gpui_kit::App;
use manyhands::repository::OperationId;
use time::OffsetDateTime;

pub(crate) use entity::Scheduler;
pub(crate) use reconfigure::heard;

use super::{
    Services,
    inflight::{ActiveRow, InflightStore},
    requests::DesktopRequestId,
};
use crate::desktop::model::polling::{
    AttemptSummary, Now, Phase, PollAttempt, PollSchedule, Reason, Root,
};
use crate::desktop::model::stamp::RepoKey;

/// The operation a poll is sent under, and whether it resends one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Planned {
    pub operation: OperationId,
    /// `restart=true`: the operation's row exists and is taken up again.
    pub restart: bool,
}

/// The operation `root`'s next poll uses (L-1, R-4).
///
/// The in-flight record's entry comes first, for the lock holder: it names
/// the row to finish, at launch or later in the session, whenever the
/// repository is due. Otherwise a wait on R-4's schedule resends its own
/// operation. Otherwise the poll is new.
pub(crate) fn plan(schedule: &PollSchedule, store: &InflightStore, root: &Root) -> Planned {
    if let Some(operation) = store.takeover_of(root) {
        return Planned {
            operation,
            restart: true,
        };
    }
    match schedule.restart_of(root) {
        Some(operation) => Planned {
            operation,
            restart: true,
        },
        None => Planned {
            operation: OperationId::new(),
            restart: false,
        },
    }
}

/// Sends the poll of the due repository `root`: plans it, writes the
/// record's entry, marks the model queued, and only then calls `submit` with
/// the operation and `restart` (R-31: written before the poll is submitted;
/// the model's wait is read before `mark_queued` discards it). A submit
/// that is refused, because quitting closed the queue, takes the entry and
/// the model's queued mark back.
pub(crate) fn start(
    schedule: &mut PollSchedule,
    store: &mut InflightStore,
    root: &Root,
    submit: impl FnOnce(OperationId, bool) -> Option<DesktopRequestId>,
) -> Option<DesktopRequestId> {
    let planned = plan(schedule, store, root);
    store.poll_submitting(root, planned.operation);
    schedule.mark_queued(root);
    let sent = submit(planned.operation, planned.restart);
    if sent.is_none() {
        store.poll_withdrawn(root, planned.operation);
        schedule.abandon(root);
    }
    sent
}

/// Why a repository's stale row is not taken over yet (L-2). The words are
/// Task 7's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(not(test), allow(dead_code))] // Read from Task 7.
pub(crate) enum Holding {
    /// Checking is off for the repository: it finishes once checking is on.
    Off,
    /// Checking is paused: turn checking on to finish it.
    Paused,
    /// Held until recovery: Resume checking to finish it.
    Held,
    /// A key, host or configuration needs the person first; the repository's
    /// attention row says which. Not in the L-2 table: the repository is
    /// eligible, and only the model parks it, so Task 7 words it.
    Attention,
    /// Not due: it finishes at the next check.
    NotDue,
}

/// Whether `root` has a stale row the record holds that cannot be taken over
/// yet, and why (L-2). `None` when there is no entry, when this instance does
/// not hold the lock (it takes nothing over, L-6), when the repository is
/// eligible (L-1 applies when it is due), and while the policy must be read
/// again first.
///
/// The causes come in the library's order: off, paused, held, then not due
/// (Fact 7).
pub(crate) fn holding(
    schedule: &PollSchedule,
    store: &InflightStore,
    root: &Root,
    now: &Now,
) -> Option<Holding> {
    store.takeover_of(root)?;
    let repo = schedule.repo(root)?;
    if !repo.policy.enabled {
        return Some(Holding::Off);
    }
    if repo.policy.paused {
        return Some(Holding::Paused);
    }
    if repo.policy.recovery_suspended
        || repo.phase == Phase::Attention(Reason::RecoveryHeld(root.clone()))
    {
        return Some(Holding::Held);
    }
    if matches!(repo.phase, Phase::Attention(_)) {
        return Some(Holding::Attention);
    }
    if repo.stale {
        return None;
    }
    match repo.phase {
        Phase::Idle | Phase::Waiting { .. } if !repo.ready(now) => Some(Holding::NotDue),
        _ => None,
    }
}

/// Whether `attempt` counts as an attempt at all: a replayed terminal answer
/// whose time is older than this launch is the previous process's, not this
/// one's, and has no status line and starts no loop (L-1).
///
/// The library stores `poll_completed_at` in whole unix seconds, so the
/// launch time is compared to the second: a poll that finishes in the same
/// wall second as the launch reads back as that second, not as older.
fn counts(attempt: &PollAttempt, launched_at: OffsetDateTime) -> bool {
    let launched = launched_at.replace_nanosecond(0).unwrap_or(launched_at);
    attempt
        .completed_at
        .is_none_or(|completed| completed >= launched)
}

/// Takes a poll's answer. `active` is `polling_status.active_operation_id`
/// read after it: the record settles on it (`InflightStore::poll_answered`),
/// then the model moves from the answer (A4) and gives back what it summed
/// to. An answer that counts as no attempt (a replay older than
/// `launched_at`) gives `None` and leaves the repository `Idle`, its policy
/// to be read again.
pub(crate) fn apply(
    schedule: &mut PollSchedule,
    store: &mut InflightStore,
    now: &Now,
    attempt: &PollAttempt,
    active: ActiveRow,
    launched_at: OffsetDateTime,
) -> Option<AttemptSummary> {
    store.poll_answered(attempt, active);
    if !counts(attempt, launched_at) {
        schedule.abandon(&attempt.root);
        return None;
    }
    schedule.after(now, attempt)
}

/// What views and the window tell the scheduler. Each does nothing until
/// polling is switched on (`Services::with_polling`, R-27).
impl Services {
    /// The repository list loaded or changed. The first call is the launch
    /// poll (B6): the one call beside the launch refresh.
    pub(crate) fn repositories_listed(&self, roots: &[RepoKey], cx: &mut App) {
        if self.polling() {
            self.scheduler
                .update(cx, |scheduler, cx| scheduler.listed(roots, cx));
        }
    }

    /// The window became active: policies and remotes are read again and
    /// the in-flight lock is tried again. Nothing is polled by this.
    pub(crate) fn window_activated(&self, cx: &mut App) {
        if self.polling() {
            self.scheduler
                .update(cx, |scheduler, cx| scheduler.activated(cx));
        }
    }
}

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "scheduler_support.rs"]
mod support;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "scheduler_entity_tests.rs"]
mod entity_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "scheduler_extra_tests.rs"]
mod extra_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "scheduler_real_tests.rs"]
mod real_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "scheduler_intent_tests.rs"]
mod intent_tests;
