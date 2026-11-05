//! The lifecycle dialog (D5 design, A2): Close ticket… on a ticket and
//! Approve and merge… on a document, opened from the item header's
//! lifecycle action. It is its own view over the services, not a
//! `ConfirmDialog` arm (ruling R-1).
//!
//! Its header names the item: the kind's icon, the command, "{short code or
//! short ID} · {title} · {repository}", and the item's context branch
//! (MH-COLLAB-006, A22).
//!
//! - **Saving.** With unsaved edits in the tab, or a draft of the item
//!   stored or restored there (R-17), the tab saves them first (decision
//!   13): "Saving your latest edits…". A saved answer goes on to prepare;
//!   any other answer closes the dialog, and the tab shows that save's
//!   outcome (P1). Escape cancels before the prepare; a stored draft not yet
//!   sent is then not saved.
//! - **Preparing.** The command is prepared (`Services::prepare`), as a
//!   read: nothing runs.
//! - **Confirm.** The preview's summary: steps, changed paths, code note,
//!   Git line and expiry (`lifecycle/confirm.rs`). Its confirmation is held;
//!   Confirm, by click or Enter, sends the first run once with it
//!   (`Worker::submit`), and then the dialog says it started.
//! - **Stale.** The library refused that consent: "Something changed since
//!   you confirmed", and Prepare again, after which Confirm sends the same
//!   request again with the new confirmation (`Worker::reconfirm`; P2, P3).
//!   Enter on it prepares again.
//! - **Blocked.** Each blocker in words, with its way out (L1,
//!   `lifecycle_prepare_way_out`), and the answer's code; a dirty main says
//!   so and offers Open folder (P19). Prepare again prepares once more,
//!   except for a ticket already closed (`ticket_closed`, P14), which
//!   offers nothing to run.
//! - **Held.** An unfinished earlier run holds the item (C5 decision 33):
//!   Continue sends that request again under its own request ID, with no
//!   confirmation, or Resume resumes its operation (P12). Neither is
//!   offered while the item's lifecycle, or that resume, is in flight: the
//!   dialog says it is still running (P22).
//! - **Answered.** Any other answer is shown with its message and code,
//!   and nothing runs.
//! - **Running, Done, Failed, Stopped.** A sent run's steps lit from its
//!   stages, Stop at a safe point, and how it ended, with its way out and
//!   D4's credential recovery (A3, `lifecycle/running.rs`). Retry sends the
//!   same request ID again (P7). The operation area's entry of a failed or
//!   stopped run opens the dialog again on it (`reopen`).
//! - **Made ready.** A closure that reached main lists the tickets it made
//!   ready, with their titles, or says none could be worked out and offers
//!   Open ready tickets (A3, R-20, `lifecycle/made_ready.rs`).
//!
//! Escape closes the dialog, and focus goes back to the control that
//! opened it; a sent run goes on. The dialog holds the item's title for its header and derives
//! no `Debug`.

mod actions;
mod confirm;
mod made_ready;
mod outcome;

pub(crate) use outcome::{cleanup_reason, item_of, progress_words};
pub(crate) use repair::{Reopen, reopen_repaired};
pub(crate) use resumed::follow_resume;
mod prepare;
mod repair;
mod republish;
mod resumed;
mod running;
mod view;

use std::path::Path;

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, ParentElement, Subscription, Task, WeakEntity,
    Window, component::WindowExt, px,
};
use manyhands::{
    repository::{
        ConfirmationId, ItemDto, ItemDtoKind, LifecycleBlocker, LifecyclePreviewDto,
        LifecycleWayOut, PreviewDto,
    },
    results::ResultCode,
};

use self::view::{button, text};
use crate::desktop::{
    model::{lifecycle_lock::LockKey, stamp::Generations},
    services::{
        Services,
        requests::{DesktopRequestId, Intent},
    },
    view::item::ItemPane,
};

pub(crate) const SAVING_FIRST: &str = "Saving your latest edits\u{2026}";
pub(crate) const PREPARING: &str = "Preparing\u{2026}";
pub(crate) const CANCEL: &str = "Cancel";
pub(crate) const CLOSE: &str = "Close";
pub(crate) const PREPARE_AGAIN: &str = "Prepare again";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const CONTINUE: &str = "Continue";
pub(crate) const RESUME: &str = "Resume";
pub(crate) const OPEN_FOLDER: &str = "Open folder";
pub(crate) const SHARE_MAIN: &str = "Share main";
/// L1's `RepairLocally` (B6).
pub(crate) const REPAIR_LOCALLY: &str = "Repair locally\u{2026}";
/// L1's `ResolveChildConflict` (B5).
pub(crate) const RESOLVE_CONFLICT: &str = "Resolve conflict\u{2026}";
/// Continue and Resume wait while the item's lifecycle runs (P22).
pub(crate) const STILL_RUNNING: &str = "Still running\u{2026}";
/// A run is sent: it goes on with the dialog hidden.
pub(crate) const STARTED: &str =
    "It continues in the background if you close this, and shows up in Activity.";
/// A blocked prepare's lead.
pub(crate) const BLOCKED_LEAD: &str =
    "Nothing was changed. Each thing below stands in the way; deal with it, then prepare again.";
/// A held item's lead.
pub(crate) const HELD_LEAD: &str =
    "An earlier run of this is not finished, and it holds the item. Nothing new was started.";
/// Share main was sent from a blocked prepare.
pub(crate) const SHARING_MAIN: &str = "Sharing main. Prepare again once it has finished.";
/// The request could not be made: no command could be built, or a held
/// request's ID could not be read.
pub(crate) const NOT_SENT: &str = "This could not be sent.";

/// Who the dialog speaks of (A22): the item, its repository and its context
/// branch. It holds the item's title, and derives no `Debug`.
#[derive(Clone)]
pub(crate) struct Heading {
    pub kind: ItemDtoKind,
    /// The ticket's short code, or the item's short ID.
    pub short: String,
    pub title: Option<String>,
    pub repository: String,
    /// The item's context branch as the tab read it; a preview's names it
    /// once one is shown.
    pub branch: Option<String>,
    /// The repository's main branch, as the workspace knows it, for a run
    /// shown without its preview (review M-5).
    pub primary: Option<String>,
    /// Its publication remote as the workspace knows it: `None` unknown,
    /// `Some(None)` none.
    pub remote: Option<Option<String>>,
}

impl Heading {
    /// The heading of `item`, whose ID is `id`, in the repository named
    /// `repository`.
    pub(crate) fn of(item: &ItemDto, id: &str, repository: String) -> Self {
        Self {
            kind: item.kind,
            short: item.slug.clone().unwrap_or_else(|| short_id(id)),
            title: item.title.clone(),
            repository,
            branch: item.context.branch.clone(),
            primary: None,
            remote: None,
        }
    }

    /// "{short} · {title} · {repository}".
    pub(crate) fn subtitle(&self) -> String {
        [Some(self.short.as_str()), self.title.as_deref()]
            .into_iter()
            .flatten()
            .chain([self.repository.as_str()])
            .collect::<Vec<_>>()
            .join(" \u{b7} ")
    }
}

/// An item's short ID: the last eight characters of its ULID, which tell
/// items made in the same millisecond apart.
pub(crate) fn short_id(id: &str) -> String {
    let start = id.len().saturating_sub(8);
    id.get(start..).unwrap_or(id).to_owned()
}

/// What opening the dialog takes: the command, who it speaks of, where focus
/// goes back, and the tab saving first, when it is.
pub(crate) struct Opening {
    pub intent: Intent,
    pub heading: Heading,
    pub origin: FocusHandle,
    /// The tab whose save runs first (decision 13); `None` when nothing was
    /// unsaved.
    pub saving: Option<WeakEntity<ItemPane>>,
    /// A run of this session the dialog shows, opened again from the
    /// operation area (A3): nothing is prepared.
    pub following: Option<DesktopRequestId>,
    /// Opened again once a local resolution its Repair locally… led to
    /// applied (B6, P9): it says main is in the working copy and offers
    /// the way on, Prepare again where no run is shown.
    pub repaired: bool,
}

/// Opens the lifecycle dialog. `None` while quitting, or while drafts are
/// being saved for it: nothing opens in front of that.
pub(crate) fn open(
    opening: Opening,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<LifecycleDialog>> {
    if services.worker().quitting() || services.operations().read(cx).saving_drafts() {
        return None;
    }
    let origin = opening.origin.clone();
    let dialog = cx.new(|cx| LifecycleDialog::new(opening, services.clone(), window, cx));
    let opened = dialog.clone();
    window.open_dialog(cx, move |base, _, _| {
        let confirming = dialog.clone();
        let origin = origin.clone();
        base.w(px(580.))
            .close_button(false)
            .overlay_closable(false)
            .child(dialog.clone())
            // Enter: Confirm, or Prepare again on a refused consent. The
            // Kit never closes the dialog here.
            .on_ok(move |_, window, cx| {
                confirming.update(cx, |dialog, cx| dialog.enter(window, cx));
                false
            })
            // Escape: close; what was sent goes on.
            .on_cancel(|_, _, _| true)
            .on_close(move |_, window, cx| window.focus(&origin, cx))
    });
    #[cfg(all(test, feature = "desktop-test-support"))]
    cx.set_global(OpenedLifecycle(opened.downgrade()));
    Some(opened)
}

/// Opens the lifecycle dialog on the run `request` of this session, from
/// its operation area entry (A3, Open…): its item by its short ID, in its
/// repository, whose main branch and publication remote the workspace knows
/// as `primary` and `remote` (`Heading`); focus goes back to `origin`.
/// `None` for a request that is no lifecycle, or while quitting.
pub(crate) fn reopen(
    request: &crate::desktop::services::requests::DesktopRequest,
    (primary, remote): (Option<String>, Option<Option<String>>),
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<LifecycleDialog>> {
    let (kind, id) = match &request.intent {
        Intent::TicketClose { id, .. } => (ItemDtoKind::Ticket, id),
        Intent::DocumentPromote { id, .. } => (ItemDtoKind::Document, id),
        _ => return None,
    };
    let heading = Heading {
        kind,
        short: short_id(&id.to_string()),
        title: None,
        repository: crate::desktop::view::operations::scope_name(request.intent.root()),
        branch: None,
        primary,
        remote,
    };
    reopen_as(request, heading, origin, services, window, cx)
}

/// `reopen`, with who the dialog speaks of already known: from the resolve
/// conflict sheet, its item's short code and title too (Task 11b review).
/// `None` for a request that is no lifecycle, or while quitting.
pub(crate) fn reopen_as(
    request: &crate::desktop::services::requests::DesktopRequest,
    heading: Heading,
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<LifecycleDialog>> {
    if !matches!(
        request.intent,
        Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
    ) {
        return None;
    }
    let opening = Opening {
        intent: request.intent.clone(),
        heading,
        origin,
        saving: None,
        following: Some(request.id),
        repaired: false,
    };
    open(opening, services, window, cx)
}

/// The lifecycle dialog last opened, for a test to read what it holds.
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) struct OpenedLifecycle(pub(crate) WeakEntity<LifecycleDialog>);

#[cfg(all(test, feature = "desktop-test-support"))]
impl gpui_kit::Global for OpenedLifecycle {}

/// Where the dialog stands.
enum Phase {
    /// The tab saves its unsaved edits first.
    Saving,
    Preparing,
    /// The preview is held, and Confirm is offered.
    Confirm {
        preview: Box<PreviewDto>,
        confirmation: ConfirmationId,
    },
    /// The prepare was refused for its blockers, answered `code`.
    Blocked {
        code: ResultCode,
        blockers: Vec<LifecycleBlocker>,
        way_out: Vec<LifecycleWayOut>,
        /// Share main was sent from here.
        shared: bool,
    },
    /// An unfinished earlier run holds the item: its one way on.
    Held {
        way_out: LifecycleWayOut,
    },
    /// The confirmed run's consent was refused: nothing ran, and Prepare
    /// again is offered.
    Stale {
        code: ResultCode,
        message: String,
    },
    /// The prepare gave no preview: a refusal, or nothing to do.
    Answered {
        code: ResultCode,
        message: String,
    },
    /// Nothing was asked of the library.
    Message(&'static str),
    /// A local resolution that Repair locally… led to applied, after a
    /// blocked prepare: Prepare again is the way on (B6).
    Repaired,
    /// A run was sent: the first, a Continue or a Resume.
    Submitted(DesktopRequestId),
}

pub(crate) struct LifecycleDialog {
    intent: Intent,
    heading: Heading,
    services: Services,
    origin: FocusHandle,
    phase: Phase,
    generations: Generations,
    prepare_read: Option<Task<()>>,
    /// The tab saving first, while the dialog waits for it.
    saving: Option<WeakEntity<ItemPane>>,
    /// The request Confirm sent, and its attempt: an answer to it that
    /// refuses the consent offers Prepare again.
    confirmed: Option<(DesktopRequestId, u32)>,
    /// The preview Confirm sent: the steps a run shows (A2).
    plan: Option<Box<LifecyclePreviewDto>>,
    /// The step of the last stage the shown run reported (L3).
    reached: Option<outcome::StepKey>,
    /// Stop at a safe point was pressed on the shown run.
    stop_pressed: bool,
    /// The titles of the tickets the shown run made ready (A3).
    titles: made_ready::Titles,
    /// Repair locally… since the dialog last moved on (B6).
    repair: repair::Repair,
    _subscriptions: Vec<Subscription>,
}

impl LifecycleDialog {
    fn new(
        opening: Opening,
        services: Services,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let Opening {
            intent,
            heading,
            origin,
            saving,
            following,
            repaired,
        } = opening;
        let mut subscriptions = vec![cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        })];
        if let Some(pane) = saving.as_ref().and_then(WeakEntity::upgrade) {
            subscriptions.push(cx.observe_in(&pane, window, |this, _, window, cx| {
                this.saving_changed(window, cx)
            }));
            subscriptions.push(
                cx.observe_in(services.drafts(), window, |this, _, window, cx| {
                    this.saving_changed(window, cx)
                }),
            );
            // Dismissed while it waits for the save first: nothing the
            // person has not seen is saved for it (review M-1).
            subscriptions.push(cx.on_release(|this: &mut Self, cx| {
                if matches!(this.phase, Phase::Saving)
                    && let Some(pane) = this.saving.as_ref().and_then(WeakEntity::upgrade)
                {
                    pane.update(cx, |pane, cx| pane.stop_save_first(cx));
                }
            }));
        }
        let mut dialog = Self {
            intent,
            heading,
            services,
            origin,
            phase: match (following, repaired) {
                (Some(request), _) => Phase::Submitted(request),
                (None, true) => Phase::Repaired,
                (None, false) => Phase::Saving,
            },
            generations: Generations::default(),
            prepare_read: None,
            saving,
            confirmed: None,
            plan: None,
            reached: None,
            stop_pressed: false,
            titles: made_ready::Titles::default(),
            repair: repair::Repair::repaired(repaired),
            _subscriptions: subscriptions,
        };
        // The dialog locks its item until it is dropped, in every phase
        // before its run is sent (A2, R-21).
        let entity = cx.entity_id();
        let operations = dialog.services.operations().downgrade();
        cx.on_release(move |_, cx| {
            let _ = operations.update(cx, |operations, cx| {
                if operations.locks_mut().release_dialog(entity) {
                    cx.notify();
                }
            });
        })
        .detach();
        dialog.sync_lock(cx);
        // Opened on a resume already sent on as the request it named.
        dialog.follow_resend(cx);
        // Opened again on a run that has its answer already.
        dialog.read_made_ready(cx);
        if dialog.saving.is_none() && following.is_none() && !repaired {
            dialog.prepare(cx);
        }
        dialog
    }

    /// The command's repository.
    fn root(&self) -> Option<&Path> {
        self.intent.root()
    }

    /// Moves the dialog to `phase`, and tells the lock whether it holds
    /// the item now: in every phase before its run is sent (R-21).
    fn set_phase(&mut self, phase: Phase, cx: &mut Context<Self>) {
        self.phase = phase;
        // A repair under way is kept: its conflict still opens the sheet.
        if !self.repairing() {
            self.repair = repair::Repair::default();
        }
        self.sync_lock(cx);
    }

    fn sync_lock(&mut self, cx: &mut Context<Self>) {
        let (Some(root), Some(id)) = (self.root().and_then(Path::to_str), self.item_id()) else {
            return;
        };
        let key = LockKey::new(root, &id.to_string());
        let holds = !matches!(self.phase, Phase::Submitted(_));
        let entity = cx.entity_id();
        self.services.operations().update(cx, |operations, cx| {
            if operations.locks_mut().set_dialog(entity, key, holds) {
                cx.notify();
            }
        });
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        window.focus(&self.origin, cx);
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl LifecycleDialog {
    /// The confirmation of the preview the dialog holds.
    pub(crate) fn confirmation(&self) -> Option<ConfirmationId> {
        match &self.phase {
            Phase::Confirm { confirmation, .. } => Some(*confirmation),
            _ => None,
        }
    }

    /// The run the dialog sent.
    pub(crate) fn submitted(&self) -> Option<DesktopRequestId> {
        match self.phase {
            Phase::Submitted(request) => Some(request),
            _ => None,
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/save_first_tests.rs"]
mod save_first_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/action_tests.rs"]
mod action_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/confirm_tests.rs"]
mod confirm_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/stale_tests.rs"]
mod stale_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/outcome_tests.rs"]
mod outcome_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/outcome_canned_tests.rs"]
mod outcome_canned_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/credential_tests.rs"]
mod credential_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/no_record_tests.rs"]
mod no_record_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/made_ready_tests.rs"]
mod made_ready_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/made_ready_review_tests.rs"]
mod made_ready_review_tests;

#[cfg(test)]
#[path = "lifecycle/steps_tests.rs"]
mod steps_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/lock_tests.rs"]
mod lock_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/lock_restart_tests.rs"]
mod lock_restart_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/after_tests.rs"]
mod after_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "lifecycle/lock_gates_tests.rs"]
mod lock_gates_tests;
