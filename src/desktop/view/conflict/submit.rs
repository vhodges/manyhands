//! The sheet's Save resolution, its kept sets and how it follows an
//! answer (D5 design, B3's table; R-10, R-12, R-25; P8, P11, P15, P24).

use gpui_kit::{App, Context, Window, component::WindowExt};
use manyhands::{
    repository::RequestId,
    results::{CheckpointEffect, Effects, RecoveryAction, ResultCode},
};

use super::{CHANGED, EARLIER_USED, NOT_SENT_AGAIN, ResolveSheet, SheetSource};
use crate::desktop::{
    model::conflict::ConflictModel,
    services::{
        requests::{DesktopRequestId, Intent, Lane, PathResolution, RequestState},
        resolutions::{Followed, Followup, KeptSet, ResolutionStore, ResolutionStoreError},
    },
    view::lifecycle::reopen_repaired,
};

impl ResolveSheet {
    pub(super) fn store(&self) -> Option<&ResolutionStore> {
        self.services.resolutions().ok()
    }

    /// The kept set Retry sends: the one of the request the read names as
    /// its unfinished attempt's, or else another of this conflict.
    pub(super) fn kept_of(&self, model: &ConflictModel, cx: &App) -> Option<KeptSet> {
        let named = model
            .attempt()
            .and_then(|attempt| RequestId::parse(attempt).ok())
            .and_then(|request| self.store()?.kept(request).ok().flatten());
        named.or_else(|| self.kept_elsewhere(cx))
    }

    /// Another set of this conflict, kept and not yet seen answered: none
    /// this sheet saw answered, and none whose request this session holds
    /// finished (review I-2), as R-25 may keep a refused set.
    fn kept_elsewhere(&self, cx: &App) -> Option<KeptSet> {
        let mut except = self.spent.clone();
        except.extend(
            self.services
                .operations()
                .read(cx)
                .requests()
                .iter()
                .filter(|request| matches!(request.state, RequestState::Finished(..)))
                .map(|request| request.request_id),
        );
        self.store()?
            .kept_for(&self.root, self.operation_id, &except)
            .ok()
            .flatten()
    }

    /// K-10: a local attempt has written paths, and no set of it is kept,
    /// so no request can finish it here.
    pub(super) fn lost(&self) -> bool {
        self.source == SheetSource::Local
            && self.kept.is_none()
            && self.store().is_some()
            && self.model.as_ref().is_some_and(ConflictModel::historical)
    }

    /// Resume is offered: the read says the commit is prepared, or the
    /// resolution stopped after its checkpoint (P15).
    pub(super) fn resumable(&self) -> bool {
        self.stopped || self.model.as_ref().is_some_and(ConflictModel::resume)
    }

    /// Whether a request the sheet sent is still out, or the conflict is
    /// being read again: Save waits for either.
    pub(super) fn waiting(&self) -> bool {
        self.following.is_some() || self.reading || self.submitting
    }

    /// Save resolution, by click or Ctrl+Enter: Retry with the kept set
    /// when there is one; another window's set of this conflict, when one
    /// is kept; otherwise a new set, once every part is chosen.
    pub(super) fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.waiting() || self.resumable() || self.lost() {
            return;
        }
        self.notice = None;
        self.answer = None;
        if let Some(kept) = self.kept.clone() {
            self.send_kept(kept, window, cx);
            return;
        }
        let Some(model) = &self.model else {
            return;
        };
        let (Some(resolutions), Some(observation)) = (model.resolutions(), model.observation())
        else {
            return;
        };
        let observation = observation.to_owned();
        let earlier = self.kept_elsewhere(cx);
        if let Some(earlier) = earlier {
            if !same(&earlier.resolutions, &resolutions) {
                self.notice = Some(EARLIER_USED);
            }
            self.send_kept(earlier, window, cx);
            return;
        }
        let set = KeptSet {
            request_id: RequestId::new(),
            root: self.root.clone(),
            operation_id: self.operation_id,
            item: self.item.clone(),
            source: self.source.conflict(),
            observation,
            resolutions,
        };
        self.submit(set, window, cx);
    }

    /// Sends `set` again under its own request ID: follows its request
    /// while this session has it out, retries it when it has its answer,
    /// and submits it otherwise (after a restart).
    pub(super) fn send_kept(&mut self, set: KeptSet, window: &mut Window, cx: &mut Context<Self>) {
        let held = self
            .services
            .operations()
            .read(cx)
            .requests()
            .iter()
            .find_map(|request| {
                (request.request_id == set.request_id)
                    .then(|| (request.id, request.attempt, request.state.clone()))
            });
        match held {
            Some((id, attempt, RequestState::Queued | RequestState::Running { .. })) => {
                self.follow(set.request_id, id, attempt);
            }
            Some((id, attempt, _)) => {
                // Registered first: the drain reads it when the retry ends.
                self.register(set.request_id);
                if self.services.worker().retry(id, None) {
                    self.following = Some((id, attempt + 1));
                } else {
                    // M-c: quitting closed the queue, or a later request of
                    // the item stands in its way.
                    self.error = Some(NOT_SENT_AGAIN.to_owned());
                }
            }
            None => self.submit(set, window, cx),
        }
        cx.notify();
    }

    /// Follows the request `id`, of `request`, from its attempt `from`.
    fn follow(&mut self, request: RequestId, id: DesktopRequestId, from: u32) {
        self.register(request);
        self.following = Some((id, from));
    }

    /// A lifecycle child's resolution `request` sends the lifecycle on once
    /// applied, whether or not this sheet is still open (R-10, I-3): the
    /// drain runs it.
    fn register(&self, request: RequestId) {
        let lifecycle = match self.source {
            SheetSource::Close(lifecycle) | SheetSource::Merge(lifecycle) => lifecycle,
            SheetSource::Share | SheetSource::Post | SheetSource::Local => return,
        };
        self.services.followups().register(
            request,
            Followup {
                lifecycle,
                root: self.root.clone(),
                operation_id: self.operation_id,
            },
        );
    }

    /// Keeps `set`, then sends it; one that cannot be kept is not sent.
    pub(super) fn submit(&mut self, set: KeptSet, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        self.submitting = true;
        let request = set.request_id;
        // Registered before it is sent, so no answer comes before it; a
        // set that could not be kept takes it away again.
        self.register(request);
        let submitted = self.services.submit_resolution(set, cx);
        cx.spawn_in(window, async move |sheet, cx| {
            let submitted = submitted.await;
            let _ = sheet.update_in(cx, |sheet, window, cx| {
                sheet.submitted(request, submitted, window, cx)
            });
        })
        .detach();
    }

    /// The set of `request` was kept and its request queued, or not. The
    /// answer may have come already, while the set was being kept: it is
    /// taken now (review I-A).
    pub(super) fn submitted(
        &mut self,
        request: RequestId,
        submitted: Result<DesktopRequestId, ResolutionStoreError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.submitting = false;
        match submitted {
            Ok(id) => {
                self.following = Some((id, 1));
                self.operations_changed(window, cx);
            }
            Err(error) => {
                self.services.followups().forget(request);
                self.error = Some(error.to_string());
            }
        }
        cx.notify();
    }

    /// The followed request's answer, once per attempt.
    pub(super) fn operations_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, from)) = self.following else {
            return;
        };
        // A request the drain has not recorded yet stays followed: the
        // request model never drops one it holds (Task 12 review).
        let Some(request) = self.services.operations().read(cx).request(id).cloned() else {
            return;
        };
        match request.state {
            RequestState::Finished(_, code, effects, recovery, message)
                if request.attempt >= from =>
            {
                self.following = None;
                self.answered(
                    request.request_id,
                    code,
                    &effects,
                    &recovery,
                    message,
                    window,
                    cx,
                );
            }
            RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_) => {
                self.following = None;
                cx.notify();
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn answered(
        &mut self,
        request: RequestId,
        code: ResultCode,
        effects: &Effects,
        recovery: &[RecoveryAction],
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.spent.push(request);
        self.answer = None;
        match code {
            ResultCode::Ok | ResultCode::AlreadyApplied => self.applied(request, window, cx),
            ResultCode::Cancelled if effects.checkpoint == CheckpointEffect::Committed => {
                self.stopped = true;
            }
            ResultCode::ExternalChange => {
                self.notice = Some(CHANGED);
                self.kept = None;
                self.read_conflict(cx);
            }
            ResultCode::OriginalRequestRequired => {
                let named = recovery
                    .iter()
                    .find_map(|action| action.arguments.get("request_id")?.as_str())
                    .and_then(|named| RequestId::parse(named).ok());
                self.kept = named.and_then(|named| self.store()?.kept(named).ok().flatten());
                if self.kept.is_none() {
                    self.answer = Some((code, message));
                }
            }
            _ => {
                self.answer = Some((code, message));
                self.kept = None;
                self.read_conflict(cx);
            }
        }
        cx.notify();
    }

    /// Applied (B3's table): the sheet closes. A lifecycle child's is sent
    /// on by the drain's follow-up (R-10, I-3), which says what it did
    /// (M-b): the lifecycle's request sent again, whose run its dialog then
    /// shows; or the operation resumed, which the operation area shows, and
    /// never the earlier run. When that resume names the request that holds
    /// the lifecycle, the drain sends that request again (D5 Part C,
    /// `Resends`), which the operation area shows too.
    pub(super) fn applied(
        &mut self,
        resolution: RequestId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close(window, cx);
        // A local resolution its Repair locally… led to: the lifecycle
        // dialog again, on its way on (B6, P9).
        if let (SheetSource::Local, Some(reopen)) = (self.source, self.reopen.take()) {
            reopen_repaired(reopen, self.origin.clone(), &self.services, window, cx);
            return;
        }
        let followed = self.services.followups().take_followed(resolution);
        let (
            Some(Followed::Resent),
            SheetSource::Close(Some(lifecycle)) | SheetSource::Merge(Some(lifecycle)),
        ) = (followed, self.source)
        else {
            return;
        };
        let request = self
            .services
            .operations()
            .read(cx)
            .request(lifecycle)
            .cloned();
        if let Some(request) = request {
            crate::desktop::view::lifecycle::reopen_as(
                &request,
                self.heading.clone(),
                self.origin.clone(),
                &self.services,
                window,
                cx,
            );
        }
    }

    /// Resume: the synchronization goes on with no input (P15), and the
    /// sheet closes.
    pub(super) fn resume(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.resume_operation();
        self.close(window, cx);
    }

    /// Sends `operation resume` of the operation the conflict was read by.
    fn resume_operation(&self) {
        self.services.worker().submit(
            Intent::OperationResume {
                root: self.root.clone(),
                operation_id: self.operation_id,
            },
            Lane::User,
            None,
            None,
        );
    }

    pub(super) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        window.focus(&self.origin, cx);
    }
}

/// Whether two sets submit the same paths with the same results.
fn same(a: &[PathResolution], b: &[PathResolution]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.token == b.token && a.source == b.source)
}
