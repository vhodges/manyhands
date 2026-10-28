//! Edit's prepare (design, Part C, Phases, steps 2 and 3): `context
//! prepare` with the Edit intent submitted on the user lane, its answer
//! taken, and Resume of a hand-off still owed, which sends the same
//! request again (R-13, R-77), never once a release of the item was sent
//! and not refused (R-78; owner, 2026-10-28).

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

/// Resume or Retry of a prepare that R-78 does not send again: why
/// (Task 17 fix 1, M3), then where its pending row is abandoned, when the
/// operation area lists one, or else to press Edit (fix 2).
pub(crate) const NOT_PREPARED_AGAIN: &str = "Your private copy cannot be made ready again from this request: a release of this item \
     was sent since.";
pub(crate) const ABANDON_THEN_EDIT: &str =
    "Abandon its pending operation in the operation area, then press Edit.";
pub(crate) const PRESS_EDIT: &str = "Press Edit to try again.";

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
    /// sent, and not refused, the worker never sends it again (R-78): the
    /// tab is viewing, says why, and Edit sends a new request.
    pub(super) fn resume(&mut self, cx: &mut Context<Self>) {
        let Some(Stopped::Pending(request)) = self.edit.stopped else {
            return;
        };
        self.edit.phase = Phase::Viewing;
        self.send_again(request, cx);
    }

    /// Retry of a refused prepare whose recovery offers the same request:
    /// sent again, unless a release of the item was sent since, and not
    /// refused (R-78); then the tab stays viewing, and says why.
    pub(super) fn retry_prepare(&mut self, request: DesktopRequestId, cx: &mut Context<Self>) {
        self.send_again(request, cx);
    }

    /// Sends the prepare `request` again, or says R-78 does not.
    fn send_again(&mut self, request: DesktopRequestId, cx: &mut Context<Self>) {
        if self.services.worker().retry(request, None) {
            self.edit.stopped = None;
            self.edit.phase = Phase::Preparing(request);
        } else {
            self.edit.stopped = Some(Stopped::NotSentAgain(request));
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
