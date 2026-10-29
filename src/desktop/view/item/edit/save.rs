//! Save (design, Part C, Phases, steps 5 and 6): the edit editor's text
//! saved as a checkpoint on this computer, and the session after a save
//! that committed (R-69).
//!
//! - **Save** is enabled only while dirty and not saving, and never during
//!   the review. `secondary-s` on a clean editor saves nothing and says so
//!   in the toolbar's status item. On an unverified base Save reads
//!   "Review before saving" and opens the review instead (R-36).
//! - **The save** is D2's (`DraftRegistry::save`): it writes the newest
//!   revision first, then sends it, one save at a time; a write that fails
//!   leaves it sent from memory (controller ruling 2). A draft still
//!   beginning is saved once it lands, and a save asked for while D2
//!   finishes the last one, once it is done. With no D2 session (the store did
//!   not open, or the draft could not begin) the fields are sent as they
//!   are, and this tab reads the base the save leaves itself, by D2's rule
//!   (`base_after_save`).
//! - **The answer** (`model::edit::save_state`). A pending checkpoint keeps
//!   the save for Retry, the same request again; `external_change` opens
//!   the review; a refusal is shown with the library's message and code.
//!   The tab's phase goes back to Editing, and the editor takes typing
//!   throughout.
//! - **After a committed save** (`saved`), on D2's `DraftSaved` or this
//!   tab's own read: the session has saved; when the editor still holds the fields
//!   sent, its baseline becomes them (`mark_saved`, keeping undo); the
//!   session's next draft begins on the base the save left, unverified when
//!   another actor changed the item in between; the tab reads the item
//!   again under a new stamp (R-70), which a clean editor takes as a late
//!   read is taken (`late::take_clean_read`, R-79), once D2 has retired the
//!   saved draft; and the remotes are read
//!   (fact 41). Text typed while a save without a D2 session settles waits
//!   in the editor and is drafted once the base is known. D2 rebases newer
//!   text itself, which is no edit to feed.
//! - **The context** this process made leaves `CreatedContexts` as the
//!   commit lands (`services::created_contexts`, R-75).
//!
//! The words, the button and the banners are `save_view`. Nothing here
//! holds text but the fields a save sent, which derive no `Debug`.

use gpui_kit::{Context, Task, Window, component::WindowExt};
use manyhands::{
    drafts::DraftFields,
    results::{Effects, RecoveryAction, RecoveryActionKind, ResultCode},
};

use super::{DraftLink, Phase};
use crate::desktop::{
    model::{
        drafts::save::{SaveVerdict, save_intent, save_verdict},
        edit::save_state::NO_CHANGES_TO_SAVE,
        operations::RecoveryStep,
        stamp::{Generations, Stamp, ViewScope},
    },
    services::requests::{DesktopRequestId, Lane, RequestState},
    view::item::ItemPane,
};

/// A save sent and not yet settled, or one whose checkpoint waits for
/// Retry. It holds the fields sent only when no D2 session holds them.
#[derive(Default)]
pub(super) struct SaveRun {
    /// The request, once sent: `None` while D2 writes the revision first,
    /// while the draft begins, and while D2 finishes the last save.
    pub(super) request: Option<DesktopRequestId>,
    /// D2 was asked to save, and has not yet answered with its request.
    asked: bool,
    /// The attempt of `request` whose answer was taken.
    taken: u32,
    /// Sent with no D2 session: the fields sent.
    pub(super) direct: Option<DraftFields>,
    /// It committed: the base it leaves is being read.
    pub(super) settling: bool,
}

impl SaveRun {
    /// Not yet sent, nor asked of D2: it waits for the draft to begin, or
    /// for D2 to finish the last save.
    pub(super) fn waiting(&self) -> bool {
        self.request.is_none() && !self.asked && self.direct.is_none()
    }
}

/// A save's answer, for its banner. It holds the library's message, never
/// the text saved.
pub(super) struct SaveAnswer {
    pub(super) request: DesktopRequestId,
    pub(super) code: ResultCode,
    pub(super) effects: Effects,
    pub(super) recovery: Vec<RecoveryAction>,
    pub(super) message: String,
}

/// A tab's saves.
pub(super) struct TabSave {
    pub(super) run: Option<SaveRun>,
    /// The last save's answer; a new save clears it.
    pub(super) answer: Option<SaveAnswer>,
    /// A publication remote is selected, as read after the last save.
    pub(super) remote: bool,
    /// The tab's read after a committed save (R-70).
    pub(super) reread: Option<Stamp>,
    /// That read landed, and waits for D2 to retire the saved draft before
    /// a clean session takes it (`saved`).
    pub(super) reread_waits: bool,
    /// Resume of a save's owed hand-off: the request, and its attempt when
    /// it was asked.
    resume: Option<(DesktopRequestId, u32)>,
    pub(super) generations: Generations,
    scope: ViewScope,
    /// The read of the base a save with no D2 session leaves, and of the
    /// remotes; replacing one drops the last.
    pub(super) read: Option<Task<()>>,
    pub(super) remotes: Option<Task<()>>,
}

impl TabSave {
    pub(super) fn new(scope: ViewScope) -> Self {
        Self {
            run: None,
            answer: None,
            remote: false,
            reread: None,
            reread_waits: false,
            resume: None,
            generations: Generations::default(),
            scope,
            read: None,
            remotes: None,
        }
    }

    pub(super) fn stamp(&mut self) -> Stamp {
        self.generations.next(self.scope.clone())
    }

    /// The session ended: no save of it is followed any more. The reads
    /// after a save find no session to settle.
    pub(super) fn clear(&mut self) {
        self.run = None;
        self.answer = None;
        self.resume = None;
        self.reread = None;
        self.reread_waits = false;
    }
}

impl ItemPane {
    /// Save and `secondary-s`. False when no edit editor is shown, a kept
    /// session's included, so the key goes on.
    pub(in crate::desktop::view::item) fn save_requested(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.shown_session().is_none() {
            return false;
        }
        if !window.has_active_dialog(cx) {
            self.start_save(cx);
        }
        true
    }

    /// Saves the shown session: one save at a time, a pending checkpoint's
    /// waiting for its Retry included, and none during the review. A clean
    /// session says "No changes to save", and one with an empty required
    /// field says which; an unverified base opens the review instead
    /// (R-36). True when a save started. The header's Save, `secondary-s`
    /// in the tab and in the details sheet, a restored draft's Save and
    /// the close dialog's come here.
    pub(super) fn start_save(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session) = self.shown_session() else {
            return false;
        };
        if self.edit.phase == Phase::Review || self.edit.save.run.is_some() {
            return false;
        }
        let toolbar = session.toolbar.clone();
        if !session.dirty(cx) {
            toolbar.update(cx, |toolbar, cx| toolbar.note(NO_CHANGES_TO_SAVE, cx));
            return false;
        }
        if let Some(missing) = self.missing_field(cx) {
            toolbar.update(cx, |toolbar, cx| toolbar.note(missing, cx));
            return false;
        }
        if !self.base_verified() {
            self.open_review(cx);
            return false;
        }
        self.edit.phase = Phase::Saving;
        self.edit.save.answer = None;
        self.edit.save.run = Some(SaveRun::default());
        self.send_save(cx);
        cx.notify();
        true
    }

    /// Whether the session's next draft begins on a verified base.
    pub(in crate::desktop::view::item) fn base_verified(&self) -> bool {
        self.edit
            .session
            .as_ref()
            .is_some_and(|session| session.base.observation.is_some())
    }

    /// Sends the save waiting in `run`: through D2's session, or, with none,
    /// as the fields are. A draft still beginning sends it once it lands
    /// (`draft_begun`); while D2 finishes the last save, once it is done
    /// (`follow_registry`).
    pub(super) fn send_save(&mut self, cx: &mut Context<Self>) {
        let Some(session) = &self.edit.session else {
            return;
        };
        if session.draft == DraftLink::Beginning || self.store_settling(cx) {
            return;
        }
        let Some(id) = self.live_draft(cx) else {
            self.send_direct(cx);
            return;
        };
        if let Some(run) = &mut self.edit.save.run {
            run.asked = true;
        }
        let sent = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.save(id, cx));
        cx.spawn(async move |pane, cx| {
            let request = sent.await;
            let _ = pane.update(cx, |pane, cx| pane.registry_sent(request, cx));
        })
        .detach();
    }

    /// D2 sent the save as `request`, or sent nothing, its base being
    /// unverified or a save of it in flight already. A session D2 ended
    /// meanwhile leaves the fields to send as they are.
    fn registry_sent(&mut self, request: Option<DesktopRequestId>, cx: &mut Context<Self>) {
        let Some(run) = &mut self.edit.save.run else {
            return;
        };
        run.asked = false;
        if let Some(request) = request {
            run.request = Some(request);
            self.save_progressed(cx);
            return;
        }
        if self.live_draft(cx).is_none() {
            self.send_direct(cx);
            return;
        }
        self.end_run(cx);
    }

    /// Sends the editor's fields over the session's base, with no D2
    /// session to write them first.
    fn send_direct(&mut self, cx: &mut Context<Self>) {
        let target = self.draft_target();
        let Some((session, target)) = self.edit.session.as_ref().zip(target) else {
            return self.end_run(cx);
        };
        let fields = session.current(cx);
        let Some(intent) = save_intent(&target, &fields, &session.base) else {
            return self.end_run(cx);
        };
        let request = self
            .services
            .worker()
            .submit(intent, Lane::User, None, None);
        self.edit.save.run = Some(SaveRun {
            request: Some(request),
            direct: Some(fields),
            ..SaveRun::default()
        });
        cx.notify();
    }

    /// The save is over with nothing to settle: back to Editing.
    pub(super) fn end_run(&mut self, cx: &mut Context<Self>) {
        self.edit.save.run = None;
        if self.edit.phase == Phase::Saving {
            self.edit.phase = Phase::Editing;
        }
        cx.notify();
    }

    /// A request moved: the save's answer, once per attempt, and a Resume's.
    pub(in crate::desktop::view::item) fn save_progressed(&mut self, cx: &mut Context<Self>) {
        self.resume_progressed(cx);
        let Some(run) = &self.edit.save.run else {
            return;
        };
        let (Some(request), false) = (run.request, run.settling) else {
            return;
        };
        let found = self
            .services
            .operations()
            .read(cx)
            .request(request)
            .map(|request| (request.attempt, request.state.clone()));
        // Still in the worker's channel.
        let Some((attempt, state)) = found else {
            return;
        };
        match state {
            RequestState::Queued | RequestState::Running { .. } => {}
            RequestState::Finished(outcome, code, effects, recovery, message) => {
                if attempt <= run.taken {
                    return;
                }
                let verdict = save_verdict(outcome, effects.checkpoint);
                self.edit.save.answer = Some(SaveAnswer {
                    request,
                    code,
                    effects,
                    recovery,
                    message,
                });
                let run = self.edit.save.run.as_mut().expect("looked up above");
                run.taken = attempt;
                match verdict {
                    SaveVerdict::Retire => {
                        run.settling = true;
                        if let Some(submitted) = run.direct.clone() {
                            self.read_after_direct(submitted, cx);
                        }
                    }
                    // Kept for Retry, under the same request.
                    SaveVerdict::Pending => self.edit.phase = Phase::Editing,
                    SaveVerdict::Keep => {
                        self.edit.save.run = None;
                        self.edit.phase = Phase::Editing;
                        // The item changed since the base: the review opens
                        // on it, read again.
                        if code == ResultCode::ExternalChange {
                            self.open_review(cx);
                        }
                    }
                }
            }
            RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_) => {
                self.edit.save.answer = None;
                self.end_run(cx);
            }
        }
        cx.notify();
    }

    /// Retry of a pending checkpoint: the same request, under its own
    /// request ID.
    pub(super) fn retry_save(&mut self, cx: &mut Context<Self>) {
        // Not while the review is open: only its Use or Cancel leaves it.
        if self.edit.phase == Phase::Review {
            return;
        }
        let Some(request) = self.edit.save.run.as_ref().and_then(|run| run.request) else {
            return;
        };
        self.edit.save.answer = None;
        self.edit.phase = Phase::Saving;
        self.services.worker().retry(request, None);
        cx.notify();
    }

    /// Resume of a save's owed hand-off: the operation it names resumed, or
    /// else the same request again.
    pub(super) fn resume_save(&mut self, cx: &mut Context<Self>) {
        if self.edit.phase == Phase::Review {
            return;
        }
        let Some(answer) = &self.edit.save.answer else {
            return;
        };
        let request = answer.request;
        let operations = self.services.operations().read(cx);
        let step = answer
            .recovery
            .iter()
            .filter(|action| {
                matches!(
                    action.action,
                    RecoveryActionKind::OperationResume | RecoveryActionKind::RequestRetry
                )
            })
            .find_map(|action| operations.recovery_step(request, action));
        let attempt = operations
            .request(request)
            .map_or(0, |request| request.attempt);
        let resumed = match step {
            Some(RecoveryStep::Submit(intent)) => {
                let resume = self
                    .services
                    .worker()
                    .submit(intent, Lane::User, None, None);
                Some((resume, 0))
            }
            Some(RecoveryStep::Retry(again)) => {
                self.services.worker().retry(again, None);
                Some((again, attempt))
            }
            _ => None,
        };
        self.edit.save.resume = resumed;
        cx.notify();
    }

    /// A Resume finished: done, the banner goes; otherwise it stays, and
    /// the operation area shows why.
    fn resume_progressed(&mut self, cx: &mut Context<Self>) {
        let Some((resume, asked)) = self.edit.save.resume else {
            return;
        };
        let found = self
            .services
            .operations()
            .read(cx)
            .request(resume)
            .map(|request| (request.attempt, request.state.clone()));
        match found {
            Some((attempt, RequestState::Finished(_, code, ..))) if attempt > asked => {
                self.edit.save.resume = None;
                if matches!(code, ResultCode::Ok | ResultCode::AlreadyApplied) {
                    self.edit.save.answer = None;
                }
                cx.notify();
            }
            Some((
                _,
                RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_),
            )) => {
                self.edit.save.resume = None;
                cx.notify();
            }
            _ => {}
        }
    }
}
