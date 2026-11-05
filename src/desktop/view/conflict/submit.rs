//! The sheet's Save resolution, its kept sets and how it follows an
//! answer (D5 design, B3's table; R-10, R-12, R-25; P8, P11, P15, P24).

use gpui_kit::{Context, Window, component::WindowExt};
use manyhands::{
    repository::RequestId,
    results::{CheckpointEffect, Effects, RecoveryAction, ResultCode},
};

use super::{CHANGED, EARLIER_USED, ResolveSheet, SheetSource};
use crate::desktop::{
    model::conflict::ConflictModel,
    services::{
        requests::{Intent, Lane, PathResolution, RequestState},
        resolutions::{KeptSet, ResolutionStore},
    },
};

impl ResolveSheet {
    pub(super) fn store(&self) -> Option<&ResolutionStore> {
        self.services.resolutions()
    }

    /// The kept set Retry sends: the one of the request the read names as
    /// its unfinished attempt's, or else another of this conflict that this
    /// sheet has not seen answered.
    pub(super) fn kept_of(&self, model: &ConflictModel) -> Option<KeptSet> {
        let store = self.store()?;
        let named = model
            .attempt()
            .and_then(|attempt| RequestId::parse(attempt).ok())
            .and_then(|request| store.kept(request).ok().flatten());
        named.or_else(|| {
            store
                .kept_for(&self.root, self.operation_id, &self.spent)
                .ok()
                .flatten()
        })
    }

    /// K-10: a local attempt has written paths, and no set of it is kept,
    /// so no request can finish it here.
    pub(super) fn lost(&self) -> bool {
        self.source == SheetSource::Local
            && self.kept.is_none()
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
        self.following.is_some() || self.reading
    }

    /// Save resolution, by click or Ctrl+Enter: Retry with the kept set
    /// when there is one; another window's set of this conflict, when one
    /// is kept; otherwise a new set, once every part is chosen.
    pub(super) fn save(&mut self, cx: &mut Context<Self>) {
        if self.waiting() || self.resumable() || self.lost() {
            return;
        }
        self.notice = None;
        self.answer = None;
        if let Some(kept) = self.kept.clone() {
            self.send_kept(kept, cx);
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
        let earlier = self.store().and_then(|store| {
            store
                .kept_for(&self.root, self.operation_id, &self.spent)
                .ok()
                .flatten()
        });
        if let Some(earlier) = earlier {
            if !same(&earlier.resolutions, &resolutions) {
                self.notice = Some(EARLIER_USED);
            }
            self.send_kept(earlier, cx);
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
        self.submit(set, cx);
    }

    /// Sends `set` again under its own request ID: follows its request
    /// while this session has it out, retries it when it has its answer,
    /// and submits it otherwise (after a restart).
    pub(super) fn send_kept(&mut self, set: KeptSet, cx: &mut Context<Self>) {
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
                self.following = Some((id, attempt));
            }
            Some((id, attempt, _)) if self.services.worker().retry(id, None) => {
                self.following = Some((id, attempt + 1));
            }
            _ => self.submit(set, cx),
        }
        cx.notify();
    }

    /// Keeps `set`, then sends it; one that cannot be kept is not sent.
    pub(super) fn submit(&mut self, set: KeptSet, cx: &mut Context<Self>) {
        self.error = None;
        let submitted = self.services.submit_resolution(set, cx);
        cx.spawn(async move |sheet, cx| {
            let submitted = submitted.await;
            let _ = sheet.update(cx, |sheet, cx| {
                match submitted {
                    Ok(id) => sheet.following = Some((id, 1)),
                    Err(error) => sheet.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The followed request's answer, once per attempt.
    pub(super) fn operations_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, from)) = self.following else {
            return;
        };
        let Some(request) = self.services.operations().read(cx).request(id).cloned() else {
            self.following = None;
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
            ResultCode::Ok | ResultCode::AlreadyApplied => self.applied(window, cx),
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

    /// Applied (B3's table): the sheet closes. A lifecycle child's sends the
    /// lifecycle's request again, and its dialog shows the run, when this
    /// session holds it; otherwise the lifecycle row is resumed (R-10).
    pub(super) fn applied(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
        match self.source {
            SheetSource::Close(Some(lifecycle)) | SheetSource::Merge(Some(lifecycle)) => {
                if !self.services.worker().retry(lifecycle, None) {
                    return;
                }
                let request = self
                    .services
                    .operations()
                    .read(cx)
                    .request(lifecycle)
                    .cloned();
                if let Some(request) = request {
                    crate::desktop::view::lifecycle::reopen(
                        &request,
                        (None, None),
                        self.origin.clone(),
                        &self.services,
                        window,
                        cx,
                    );
                }
            }
            SheetSource::Close(None) | SheetSource::Merge(None) => self.resume_operation(),
            SheetSource::Share | SheetSource::Post | SheetSource::Local => {}
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
