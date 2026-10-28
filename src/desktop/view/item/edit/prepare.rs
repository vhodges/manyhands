//! Edit's prepare (design, Part C, Phases, steps 2 and 3): `context
//! prepare` with the Edit intent submitted on the user lane, its answer
//! taken, and Resume of a hand-off still owed, which sends the same
//! request again (R-13, R-77), never once a release of the item was sent
//! (R-78).

use std::path::PathBuf;

use gpui_kit::Context;
use manyhands::{
    repository::{AuthoringKind, ContextPrepareIntent, ItemDtoKind},
    results::{RecoveryActionKind, ResultCode},
};

use super::Stopped;
use crate::desktop::{
    model::{edit::Phase, navigation::OpenTarget},
    services::requests::{DesktopRequestId, Intent, Lane, RequestState},
    view::item::ItemPane,
};

impl ItemPane {
    /// Submits `context prepare` with the Edit intent for this tab's item.
    pub(super) fn prepare(&mut self, cx: &mut Context<Self>) {
        let OpenTarget::Item { repo, kind, id } = self.model.target().clone() else {
            return;
        };
        if self.edit.phase != Phase::Viewing {
            return;
        }
        let kind = match kind {
            ItemDtoKind::Document => AuthoringKind::Document,
            ItemDtoKind::Ticket => AuthoringKind::Ticket,
        };
        let request = self.services.worker().submit(
            Intent::ContextPrepare {
                root: PathBuf::from(repo),
                kind,
                id,
                intent: ContextPrepareIntent::Edit,
            },
            Lane::User,
            None,
            None,
        );
        self.edit.phase = Phase::Preparing(request);
        self.edit.stopped = None;
        cx.notify();
    }

    /// Resume of a prepare whose hand-off is owed: the same request again,
    /// under its own request ID (R-77). Once a release of the item was
    /// sent, the worker never sends it again (R-78): the tab is viewing,
    /// and Edit sends a new request.
    pub(super) fn resume(&mut self, cx: &mut Context<Self>) {
        let Some(Stopped::Pending(request)) = self.edit.stopped else {
            return;
        };
        self.edit.stopped = None;
        self.edit.phase = if self.services.worker().retry(request, None) {
            Phase::Preparing(request)
        } else {
            Phase::Viewing
        };
        cx.notify();
    }

    /// Retry of a refused prepare whose recovery offers the same request:
    /// sent again, unless a release of the item was sent since (R-78); then
    /// the tab stays viewing.
    pub(super) fn retry_prepare(&mut self, request: DesktopRequestId, cx: &mut Context<Self>) {
        self.edit.stopped = None;
        if self.services.worker().retry(request, None) {
            self.edit.phase = Phase::Preparing(request);
        }
        cx.notify();
    }

    /// The prepare's request moved: on `ok` the item is read again under a
    /// new stamp (R-70); `discovery_pending` waits for Resume; anything
    /// else is a refusal, shown, and the tab stays viewing.
    pub(in crate::desktop::view::item) fn prepare_progressed(&mut self, cx: &mut Context<Self>) {
        let Phase::Preparing(request) = self.edit.phase else {
            return;
        };
        let state = self
            .services
            .operations()
            .read(cx)
            .request(request)
            .map(|request| request.state.clone());
        match state {
            None | Some(RequestState::Queued | RequestState::Running { .. }) => return,
            Some(RequestState::Finished(_, ResultCode::Ok, ..)) => {
                self.edit.phase = Phase::Loading;
                self.edit.loaded = false;
                self.reload(cx);
            }
            // R-13, R-77: editing waits in Loading for Resume (controller
            // ruling, 2026-10-28). No read opens the editor meanwhile.
            Some(RequestState::Finished(_, ResultCode::DiscoveryPending, ..)) => {
                self.edit.phase = Phase::Loading;
                self.edit.loaded = false;
                self.edit.stopped = Some(Stopped::Pending(request));
            }
            Some(RequestState::Finished(_, code, _, recovery, message)) => {
                let retry = recovery
                    .iter()
                    .any(|action| action.action == RecoveryActionKind::RequestRetry)
                    .then_some(request);
                self.edit.phase = Phase::Viewing;
                self.edit.stopped = Some(Stopped::Refused {
                    code,
                    message,
                    retry,
                });
            }
            Some(RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_)) => {
                self.edit.phase = Phase::Viewing;
            }
        }
        cx.notify();
    }
}
