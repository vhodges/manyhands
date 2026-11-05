//! Stop and republish… and Republish… in the lifecycle dialog (D5 design,
//! A3; Part C; P21), where L1's way out names them.
//!
//! - **Stop and republish…** (`CancelThenSendAgain`, the one button for a
//!   deleted shared branch): the run is given up, its record cancelled and
//!   the same request sent again (`Worker::give_up`), and the dialog follows
//!   that attempt. Only a Give up sent on an accepted record and answered
//!   `cancelled` goes on: the dialog says "Stopped; ready to republish", and
//!   Republish… opens over it. Any other answer is shown and ends there.
//! - **Republish…** (`Republish`, with no cancellation to ask for, as under
//!   a resume; or once stopped): the confirmation dialog on `item
//!   republish`. Once it is done, the drain sends the run on: the same
//!   request again, or the resume L1 names (C5 decision 19).
//!
//! Neither is offered while the item's lifecycle is in flight (P22), while
//! a republish of the item is queued or running, or while one confirmed
//! from this run's way out waits to be done (review I-2): one republish,
//! and one resend after it. A Give up that could not be queued says so.
//! Republish… opens over the dialog, and gives focus back to it. The
//! workspace runs both (`view/operations/republish.rs`).

use gpui_kit::{AnyElement, Context, IntoElement, Styled, Window, component::Root};
use manyhands::{
    repository::{LifecycleWayOut, MutationDataDto, OperationId},
    results::{Envelope, ResultCode},
};

use super::{LifecycleDialog, Phase, button, text};
use crate::desktop::{
    services::{
        requests::{DesktopRequestId, Intent},
        resends::AfterRepublish,
        worker::GaveUp,
    },
    view::{
        operations::republish::{REPUBLISH, Republishing, STOP_AND_REPUBLISH},
        workspace::Workspace,
    },
};

/// A run given up by Stop and republish…, stopped: Republish… is next.
pub(crate) const STOPPED_READY: &str = "Stopped; ready to republish.";
/// Stop and republish… could not give the run up: it is queued or running,
/// or this session does not hold it.
pub(crate) const NOT_STOPPED: &str = "This run could not be stopped now.";

impl LifecycleDialog {
    /// Whether the run `id`, answered `envelope`, is a Give up of an
    /// accepted record that stopped: what Stop and republish… waits for.
    fn stopped_to_republish(
        &self,
        id: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
    ) -> bool {
        envelope.code == ResultCode::Cancelled
            && self.services.worker().gave_up(id) == Some(GaveUp::Resent)
    }

    /// "Stopped; ready to republish", once Stop and republish… stopped the
    /// run `id`.
    pub(super) fn republish_line(
        &self,
        id: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
    ) -> Option<AnyElement> {
        if self.stop_refused {
            return Some(
                text("lifecycle-stop-refused", NOT_STOPPED)
                    .text_sm()
                    .into_any_element(),
            );
        }
        self.stopped_to_republish(id, envelope).then(|| {
            text("lifecycle-stopped-ready", STOPPED_READY)
                .text_sm()
                .into_any_element()
        })
    }

    /// Whether a republish of the item is queued or running, or one
    /// confirmed from the way out of `run` waits to be done.
    fn republishing_now(&self, run: DesktopRequestId) -> bool {
        if self.services.resends().pending_for(run) {
            return true;
        }
        let (Some(root), Some(item)) = (self.root(), self.item_id()) else {
            return false;
        };
        self.services.worker().in_flight(|intent| {
            matches!(intent, Intent::ItemRepublish { root: asked, id, .. }
                if asked.as_path() == root && id == item)
        })
    }

    /// Stop and republish… or Republish…, where the way out of the run
    /// `id`, answered `envelope`, names them; none while the item's
    /// lifecycle is in flight (`waiting`).
    pub(super) fn republish_buttons(
        &self,
        id: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
        way_out: &[LifecycleWayOut],
        waiting: bool,
        cx: &Context<Self>,
    ) -> Vec<AnyElement> {
        if waiting || self.republishing_now(id) {
            return Vec::new();
        }
        if self.stopped_to_republish(id, envelope) {
            let after = AfterRepublish::Retry(id);
            let go = self.on(cx, move |dialog, window, cx| {
                dialog.republish(Some(after.clone()), window, cx)
            });
            return vec![button("lifecycle-republish", REPUBLISH, true, go)];
        }
        if way_out
            .iter()
            .any(|step| matches!(step, LifecycleWayOut::CancelThenSendAgain { .. }))
        {
            let go = self.on(cx, |dialog, window, cx| {
                dialog.stop_and_republish(window, cx)
            });
            return vec![button(
                "lifecycle-stop-and-republish",
                STOP_AND_REPUBLISH,
                true,
                go,
            )];
        }
        if !way_out
            .iter()
            .any(|step| matches!(step, LifecycleWayOut::Republish { .. }))
        {
            return Vec::new();
        }
        // The republish, then what L1 names after it; with nothing named
        // (a resume's way out that names no operation), the republish alone
        // (review M2).
        let after = way_out.iter().find_map(|step| match step {
            LifecycleWayOut::SendAgain => Some(AfterRepublish::Retry(id)),
            LifecycleWayOut::ResumeOperation { operation_id } => {
                let root = self.root()?.to_path_buf();
                let operation_id = OperationId::parse(operation_id).ok()?;
                Some(AfterRepublish::Resume {
                    shown: id,
                    root,
                    operation_id,
                })
            }
            _ => None,
        });
        let go = self.on(cx, move |dialog, window, cx| {
            dialog.republish(after.clone(), window, cx)
        });
        vec![button("lifecycle-republish", REPUBLISH, true, go)]
    }

    /// The item to republish, the run going on as `after` says.
    fn republishing(&self, after: Option<AfterRepublish>) -> Option<Republishing> {
        Some(Republishing {
            root: self.root()?.to_path_buf(),
            kind: self.authoring_kind(),
            id: self.item_id()?.clone(),
            after,
        })
    }

    /// Republish…, over the dialog, which has focus again once it closes.
    fn republish(
        &mut self,
        after: Option<AfterRepublish>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self.republishing(after) else {
            return;
        };
        let origin = self.focus.clone();
        self.with_workspace(window, cx, move |workspace, window, cx| {
            workspace.republish(target, origin, window, cx);
        });
    }

    /// Stop and republish…: the shown run given up; the workspace opens
    /// Republish… once it has stopped.
    fn stop_and_republish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Submitted(shown) = self.phase else {
            return;
        };
        if self.in_flight_with(None) {
            return;
        }
        let Some(target) = self.republishing(Some(AfterRepublish::Retry(shown))) else {
            return;
        };
        let origin = self.focus.clone();
        let dialog = cx.entity().downgrade();
        let mut queued = false;
        self.with_workspace(window, cx, |workspace, _, cx| {
            queued = workspace.stop_and_republish(shown, target, origin, Some(dialog), cx);
        });
        if queued {
            self.following_anew();
        } else {
            // A3: the sequence stops, and says so.
            self.stop_refused = true;
        }
        cx.notify();
    }

    /// Runs `f` on the window's workspace, the dialog staying open.
    fn with_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>),
    ) {
        let Some(Some(root)) = window.root::<Root>() else {
            return;
        };
        let Ok(workspace) = root.read(cx).view().clone().downcast::<Workspace>() else {
            return;
        };
        workspace.update(cx, |workspace, cx| f(workspace, window, cx));
    }
}
