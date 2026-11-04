//! The lifecycle dialog (D5 design, A2): Close ticket… on a ticket and
//! Approve and merge… on a document, opened from the item header's
//! lifecycle action. It is its own view over the services, not a
//! `ConfirmDialog` arm (ruling R-1).
//!
//! Its header names the item: the kind's icon, the command, "{short code or
//! short ID} · {title} · {repository}", and the item's context branch
//! (MH-COLLAB-006, A22).
//!
//! - **Saving.** With unsaved edits in the tab, the tab saves them first
//!   (decision 13): "Saving your latest edits…". A saved answer goes on to
//!   prepare; any other answer closes the dialog, and the tab shows that
//!   save's outcome (P1). Escape cancels before the prepare.
//! - **Preparing.** The command is prepared (`Services::prepare`), as a
//!   read: nothing runs.
//! - **Confirm.** The preview's confirmation is held; Confirm, by click or
//!   Enter, sends the first run once with it (`Worker::submit`), and then
//!   the dialog says it started. The summary is Task 6b's.
//! - **Blocked.** Each blocker in words, with its way out (L1,
//!   `lifecycle_prepare_way_out`), and the answer's code; a dirty main says
//!   so and offers Open folder (P19). Prepare again prepares once more,
//!   except for a ticket already closed (`ticket_closed`, P14), which
//!   offers nothing to run.
//! - **Held.** An unfinished earlier run holds the item (C5 decision 33):
//!   Continue sends that request again under its own request ID, with no
//!   confirmation, or Resume resumes its operation (P12). Neither is
//!   offered while the item's lifecycle is already in flight.
//! - **Answered.** Any other answer is shown with its message and code,
//!   and nothing runs.
//!
//! Escape closes the dialog, and focus goes back to the control that
//! opened it. The dialog holds the item's title for its header and derives
//! no `Debug`.

mod prepare;
mod view;

use std::path::Path;

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, ParentElement, Subscription, Task, WeakEntity,
    Window, component::WindowExt, px,
};
use manyhands::{
    repository::{
        ConfirmationId, ItemDto, ItemDtoKind, LifecycleBlocker, LifecycleWayOut, PreviewDto,
    },
    results::ResultCode,
};

use self::view::{button, text};
use crate::desktop::{
    model::stamp::Generations,
    services::{
        Services,
        requests::{DesktopRequestId, Intent, Lane},
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
/// Confirm sent the first run: what runs next is Task 7's to show.
pub(crate) const STARTED: &str =
    "Started. It continues in the background and shows up in Activity.";
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
pub(crate) struct Heading {
    pub kind: ItemDtoKind,
    /// The ticket's short code, or the item's short ID.
    pub short: String,
    pub title: Option<String>,
    pub repository: String,
    /// The item's context branch as the tab read it; a preview's names it
    /// once one is shown.
    pub branch: Option<String>,
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
            // Enter: Confirm, when it is offered. The Kit never closes the
            // dialog here.
            .on_ok(move |_, _, cx| {
                confirming.update(cx, |dialog, cx| dialog.confirm(cx));
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
    /// The prepare gave no preview: a refusal, or nothing to do.
    Answered {
        code: ResultCode,
        message: String,
    },
    /// Nothing was asked of the library.
    Message(&'static str),
    /// A run was sent: the first, a Continue or a Resume.
    Submitted(#[cfg_attr(not(test), allow(dead_code))] DesktopRequestId), // Called from Task 7.
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
        } = opening;
        let mut subscriptions = Vec::new();
        if let Some(pane) = saving.as_ref().and_then(WeakEntity::upgrade) {
            subscriptions.push(cx.observe_in(&pane, window, |this, _, window, cx| {
                this.saving_changed(window, cx)
            }));
            subscriptions.push(
                cx.observe_in(services.drafts(), window, |this, _, window, cx| {
                    this.saving_changed(window, cx)
                }),
            );
        }
        let mut dialog = Self {
            intent,
            heading,
            services,
            origin,
            phase: Phase::Saving,
            generations: Generations::default(),
            prepare_read: None,
            saving,
            _subscriptions: subscriptions,
        };
        if dialog.saving.is_none() {
            dialog.prepare(cx);
        }
        dialog
    }

    /// The command's repository.
    fn root(&self) -> Option<&Path> {
        self.intent.root()
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        window.focus(&self.origin, cx);
    }

    /// Confirm: the first run, sent once with the preview's confirmation.
    /// Anything but a held preview does nothing.
    pub(crate) fn confirm(&mut self, cx: &mut Context<Self>) {
        let Phase::Confirm { confirmation, .. } = &self.phase else {
            return;
        };
        let confirmation = *confirmation;
        let request = self.services.worker().submit(
            self.intent.clone(),
            Lane::User,
            Some(confirmation),
            None,
        );
        self.phase = Phase::Submitted(request);
        cx.notify();
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
