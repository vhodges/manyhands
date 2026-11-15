//! Repair locally… and Review changes… in the lifecycle dialog (D5
//! design, B5, B6; L1's `RepairLocally` and `ResolveChildConflict`).
//!
//! - **Repair locally…** observes the item's work context, then sends
//!   `context sync-local` with that observation (`Intent::LocalContextSync`).
//!   A conflict closes the dialog and opens the resolve conflict sheet on
//!   the synchronization, its folder the work context; once its resolution
//!   applies, the dialog opens again on its way on (P9). With a prompt over
//!   the dialog when the conflict lands, the sheet waits on the dialog
//!   until a frame finds focus back inside it (Wave 04 Cycle 05b R-24). It
//!   is not offered while a local synchronization of the item is queued or
//!   running, or while the item's lifecycle is in flight (P22). A clean
//!   result says main is in the working copy, and the dialog's way on stays
//!   offered: Retry where a request holds the lifecycle, Resume where none
//!   does, Prepare again after a blocked prepare. A refusal, of the
//!   observation or of the synchronization, or a stop shows its answer; the
//!   synchronization's pending row then offers Resume (P16).
//! - **Review changes…** closes the dialog and opens the sheet on the
//!   child's conflict, by the operation the way out names (C5 decision 34),
//!   as the close's or merge's child: the one this dialog shows when it is
//!   that request, or else the lifecycle row's (R-10). It goes on only
//!   while focus is inside the dialog: pressed in the frame a prompt opens
//!   over it, it does nothing (Wave 04 Cycle 05b R-24).
//!
//! Each opens the sheet through the workspace's `show_resolve`, after
//! `in_workspace` has closed the dialog (Wave 04 Cycle 05b B2). The Kit
//! cannot say which dialog is on top, and closing the dialog closes the
//! top one, so neither goes on while a prompt is over it.
//!
//! Neither waits for the item's lifecycle lock: they are how a stopped
//! lifecycle goes on (P8, P9). Each keeps its words to fixed text and the
//! library's message and code.

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Entity, FocusHandle, IntoElement, ParentElement, Styled,
    Subscription, Task, Window,
    component::{ActiveTheme, v_flex},
};
use manyhands::{
    repository::{AuthoringKind, ItemDtoKind, LifecycleWayOut, OperationId},
    results::ResultCode,
};

use super::{
    Heading, LifecycleDialog, NOT_SENT, Phase, REPAIR_LOCALLY, REVIEW_CHANGES, button, text,
};
use crate::desktop::{
    model::{
        operations::primary_subject,
        stamp::{Stamp, ViewScope},
    },
    services::{
        ReadResult, Services,
        conflicts::LocalObservation,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    view::conflict::{Opening, ResolveSheet, SheetSource},
};

/// Where Repair locally… stands.
enum State {
    Observing,
    Syncing(DesktopRequestId),
    /// Main was brought in with no conflict.
    Repaired,
    /// Refused or stopped: its code and message.
    Answered(ResultCode, String),
    /// Stopped on the conflict of this synchronization, whose sheet waits
    /// until focus is inside the dialog (R-24).
    Conflicted(OperationId),
}

/// Repair locally…'s state in the dialog, until the dialog moves on.
#[derive(Default)]
pub(super) struct Repair {
    state: Option<State>,
    /// The work context the observation named: the sheet's folder.
    worktree: Option<PathBuf>,
    read: Option<Task<()>>,
    watch: Option<Subscription>,
}

impl Repair {
    /// Repaired already, for a dialog opened again once a local
    /// resolution applied (P9).
    pub(super) fn repaired(repaired: bool) -> Self {
        Self {
            state: repaired.then_some(State::Repaired),
            ..Self::default()
        }
    }
}

/// What opens the lifecycle dialog again once a local resolution that its
/// Repair locally… led to applies (B6, P9): the dialog's command and
/// heading, and the run it showed, if it showed one.
pub(crate) struct Reopen {
    intent: Intent,
    heading: Heading,
    following: Option<DesktopRequestId>,
}

/// Opens the lifecycle dialog again on `reopen`, saying main is in the
/// working copy, with its way on: the shown run's Retry or Resume, or
/// Prepare again after a blocked prepare. Focus goes back to `origin`.
pub(crate) fn reopen_repaired(
    reopen: Reopen,
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<LifecycleDialog>> {
    let opening = super::Opening {
        intent: reopen.intent,
        heading: reopen.heading,
        origin,
        saving: None,
        following: reopen.following,
        repaired: true,
    };
    super::open(opening, services, window, cx)
}

impl LifecycleDialog {
    pub(super) fn authoring_kind(&self) -> AuthoringKind {
        match self.heading.kind {
            ItemDtoKind::Ticket => AuthoringKind::Ticket,
            ItemDtoKind::Document => AuthoringKind::Document,
        }
    }

    /// Whether this dialog's Repair locally… is observing or its
    /// synchronization is out: the dialog keeps it as it moves on, so the
    /// synchronization's conflict still opens the sheet.
    pub(super) fn repairing(&self) -> bool {
        matches!(
            self.repair.state,
            Some(State::Observing | State::Syncing(_) | State::Conflicted(_))
        )
    }

    /// Whether a local synchronization of the item is queued or running,
    /// this dialog's or another's: its entry shows how it ends (M1).
    fn syncing(&self) -> bool {
        let (Some(root), Some(id)) = (self.root(), self.item_id()) else {
            return false;
        };
        self.services.worker().in_flight(|intent| {
            matches!(intent, Intent::LocalContextSync { root: asked, id: asked_id, .. }
                if asked.as_path() == root && asked_id == id)
        })
    }

    /// Repair locally…: the work context observed, then synchronized with
    /// main from that observation.
    pub(super) fn repair_locally(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.repairing() {
            return;
        }
        let (Some(root), Some(id)) = (self.root().map(PathBuf::from), self.item_id().cloned())
        else {
            return;
        };
        let stamp = self.generations.next(ViewScope::Conflict);
        self.repair.state = Some(State::Observing);
        self.repair.read = Some(self.services.observe_local_sync(
            root,
            self.authoring_kind(),
            id,
            stamp,
            cx,
            Self::observed,
        ));
        self.repair.watch = Some(cx.observe_in(
            self.services.operations(),
            window,
            |dialog, _, window, cx| dialog.repair_progressed(window, cx),
        ));
        cx.notify();
    }

    fn observed(
        &mut self,
        stamp: Stamp,
        result: ReadResult<LocalObservation>,
        cx: &mut Context<Self>,
    ) {
        if !self.generations.is_current(&stamp)
            || !matches!(self.repair.state, Some(State::Observing))
        {
            return;
        }
        let observed = match result {
            ReadResult::Ok(observed) => observed,
            ReadResult::Busy => return self.repair_answered(ResultCode::Busy, cx),
            ReadResult::Failed(failure) => return self.repair_answered(failure.code, cx),
        };
        let (Some(root), Some(id)) = (self.root().map(PathBuf::from), self.item_id().cloned())
        else {
            return;
        };
        let intent = Intent::LocalContextSync {
            root,
            kind: self.authoring_kind(),
            id,
            observation: observed.observation,
        };
        self.repair.worktree = Some(observed.worktree);
        let sync = self
            .services
            .worker()
            .submit(intent, Lane::User, None, None);
        self.repair.state = Some(State::Syncing(sync));
        cx.notify();
    }

    fn repair_answered(&mut self, code: ResultCode, cx: &mut Context<Self>) {
        self.repair.state = Some(State::Answered(code, code.message().to_owned()));
        cx.notify();
    }

    /// The synchronization's answer: its conflict opens the sheet; any
    /// other answer is shown.
    fn repair_progressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(State::Syncing(sync)) = self.repair.state else {
            return;
        };
        let operations = self.services.operations().read(cx);
        let Some(request) = operations.request(sync) else {
            return;
        };
        let state = match &request.state {
            RequestState::Finished(_, code, _, _, message) => match operations.review(sync) {
                Some(review) => {
                    self.repair.state = Some(State::Conflicted(review.operation_id));
                    return self.open_conflicted(window, cx);
                }
                None if *code == ResultCode::Ok => State::Repaired,
                None => State::Answered(*code, message.clone()),
            },
            RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_) => {
                State::Answered(ResultCode::Cancelled, NOT_SENT.to_owned())
            }
            RequestState::Queued | RequestState::Running { .. } => return,
        };
        self.repair.state = Some(state);
        cx.notify();
    }

    /// Whether focus is inside this dialog: in its own element, or on the
    /// Kit's dialog around it, which holds focus as the dialog opens. A
    /// prompt opened over the dialog holds it outside, from the update that
    /// opens the prompt (R-24).
    fn focus_inside(&self, window: &Window, cx: &mut App) -> bool {
        self.focus.contains_focused(window, cx) || self.focus.within_focused(window, cx)
    }

    /// A press of a way into the sheet: as `on`, but it goes on only while
    /// focus was inside the dialog before the press moves it there, so a
    /// press made in the frame a prompt opens over the dialog does nothing
    /// (R-24).
    fn on_inside<F>(
        &self,
        cx: &Context<Self>,
        f: F,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + use<F>
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    {
        let entity = cx.entity().downgrade();
        move |_, window, cx| {
            let _ = entity.update(cx, |dialog, cx| {
                if !dialog.focus_inside(window, cx) {
                    return;
                }
                window.focus(&dialog.focus, cx);
                f(dialog, window, cx)
            });
        }
    }

    /// Each frame of a dialog whose synchronization stopped on its conflict
    /// with a prompt over it: once the frame is drawn, the sheet opens if
    /// focus is back inside the dialog, that is, once the prompt has closed
    /// (R-24, U-10).
    pub(super) fn conflicted_waits(&self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.repair.state, Some(State::Conflicted(_))) {
            cx.defer_in(window, |dialog, window, cx| {
                dialog.open_conflicted(window, cx)
            });
        }
    }

    /// The local synchronization's conflict, in the sheet in place of this
    /// dialog, while focus is inside the dialog; otherwise it waits on the
    /// dialog (`conflicted_waits`).
    fn open_conflicted(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(State::Conflicted(operation)) = self.repair.state else {
            return;
        };
        if self.focus_inside(window, cx) {
            self.open_local_sheet(operation, window, cx);
        }
    }

    /// The local synchronization's conflict, in the sheet in place of this
    /// dialog.
    ///
    /// Once its resolution applies, the sheet opens this dialog again on
    /// its way on (P9).
    fn open_local_sheet(
        &mut self,
        operation_id: OperationId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let folder = std::mem::take(&mut self.repair).worktree;
        let reopen = Reopen {
            intent: self.intent.clone(),
            heading: self.heading.clone(),
            following: match self.phase {
                Phase::Submitted(shown) => Some(shown),
                _ => None,
            },
        };
        if let Some(sheet) = self.open_sheet(SheetSource::Local, operation_id, folder, window, cx) {
            sheet.update(cx, |sheet, _| sheet.reopen_after(reopen));
        }
    }

    /// Review changes…: the child's conflict, by `operation`, in the sheet
    /// in place of this dialog.
    pub(super) fn resolve_child_conflict(
        &mut self,
        operation: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(operation_id) = OperationId::parse(operation) else {
            return;
        };
        // The close or merge this dialog shows, when it is one this
        // session holds: a resume's child resumes the row instead (R-10).
        let held = match self.phase {
            Phase::Submitted(shown) => self
                .services
                .operations()
                .read(cx)
                .request(shown)
                .filter(|request| {
                    matches!(
                        request.intent,
                        Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
                    )
                })
                .map(|request| request.id),
            _ => None,
        };
        let source = match self.heading.kind {
            ItemDtoKind::Ticket => SheetSource::Close(held),
            ItemDtoKind::Document => SheetSource::Merge(held),
        };
        self.open_sheet(source, operation_id, None, window, cx);
    }

    /// The sheet on `operation_id`, from `source`, in place of this dialog:
    /// `in_workspace` closes the dialog, and the workspace opens the sheet.
    fn open_sheet(
        &mut self,
        source: SheetSource,
        operation_id: OperationId,
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<ResolveSheet>> {
        let (Some(root), Some(item)) = (self.root().map(PathBuf::from), self.item_id().cloned())
        else {
            return None;
        };
        // The sheet names the primary branch as this dialog does (R-35).
        let mut heading = self.heading.clone();
        if let Some(plan) = &self.plan {
            heading.primary = Some(plan.primary_branch.clone());
        }
        let opening = Opening {
            source,
            root,
            operation_id,
            item,
            heading,
            folder,
            origin: self.origin.clone(),
        };
        self.in_workspace(window, cx, move |workspace, window, cx| {
            workspace.show_resolve(opening, window, cx)
        })
        .flatten()
    }

    /// Repair locally… and Review changes…, neither primary, where the
    /// way out names them. Repair locally… is not offered while it runs,
    /// while any local synchronization of the item is queued or running, or
    /// while the item's lifecycle is in flight (`waiting`, P22).
    pub(super) fn repair_buttons(
        &self,
        way_out: &[LifecycleWayOut],
        waiting: bool,
        cx: &Context<Self>,
    ) -> Vec<(bool, AnyElement)> {
        let mut buttons = Vec::new();
        let repair = !waiting && !self.repairing() && !self.syncing();
        for step in way_out {
            match step {
                LifecycleWayOut::RepairLocally { .. } if repair => {
                    let go = self.on(cx, |dialog, window, cx| dialog.repair_locally(window, cx));
                    buttons.push(button(
                        "lifecycle-repair-locally",
                        REPAIR_LOCALLY,
                        false,
                        go,
                    ));
                }
                LifecycleWayOut::ResolveChildConflict { operation_id } => {
                    let operation = operation_id.clone();
                    let go = self.on_inside(cx, move |dialog, window, cx| {
                        dialog.resolve_child_conflict(&operation, window, cx)
                    });
                    buttons.push(button(
                        "lifecycle-resolve-conflict",
                        REVIEW_CHANGES,
                        false,
                        go,
                    ));
                }
                _ => {}
            }
        }
        buttons
    }

    /// Where Repair locally… stands, in words; its answer's code too.
    pub(super) fn repair_line(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let item = self.item_word();
        let primary = self.primary();
        let (words, code) = match self.repair.state.as_ref()? {
            State::Observing | State::Syncing(_) | State::Conflicted(_) => (
                format!("Bringing {primary} into this {item}'s working copy\u{2026}"),
                None,
            ),
            State::Repaired => (
                format!(
                    "{} is in this {item}'s working copy now.",
                    primary_subject(self.known_primary().as_deref())
                ),
                None,
            ),
            State::Answered(code, message) => (message.clone(), Some(*code)),
        };
        let muted = cx.theme().muted_foreground;
        Some(
            v_flex()
                .gap_1()
                .child(text("lifecycle-repair", words).text_sm())
                .children(code.map(|code| {
                    text("lifecycle-repair-code", code.as_str())
                        .text_xs()
                        .text_color(muted)
                }))
                .into_any_element(),
        )
    }
}
