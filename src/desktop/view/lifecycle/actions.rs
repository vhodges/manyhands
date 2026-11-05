//! The lifecycle dialog's actions on a sent run (D5 design, A2, A3): Stop
//! at a safe point; Retry, which sends the same request ID again with no
//! confirmation and no passphrase (`submit_with_request_id`, P7), so a
//! protected key takes D4's unlock path; Unlock…, which first clears the
//! keys the run was declined for (R-3); Prepare again, a new request; the
//! places the way out opens in the workspace; what an ended run offers;
//! and Enter.

use gpui_kit::{AnyElement, Context, Window, component::Root};
use manyhands::{
    repository::{LifecycleBlocker, LifecycleWayOut, MutationDataDto, lifecycle_way_out},
    results::{Envelope, RecoveryActionKind},
};

use super::{
    LifecycleDialog, Phase, button,
    outcome::{Credential, Ending, OPEN_REPOSITORY_DETAILS, Reach, lifecycle_data},
};
use crate::desktop::{
    services::{
        requests::{DesktopRequestId, Lane},
        worker::{CancelResult, RecordState},
    },
    view::{item::ItemRoute, workspace::Workspace},
};

/// What an ended run offers.
#[derive(Default)]
pub(super) struct Offers {
    pub(super) retry: bool,
    pub(super) prepare: bool,
    /// Nothing of the run is kept: the library deleted its request, or
    /// stopped it before any record (P18, review I-1).
    pub(super) no_record: bool,
    pub(super) resume: Option<String>,
    pub(super) credential: Option<Credential>,
    pub(super) inspect: bool,
    pub(super) open_folder: bool,
    /// Retry, Resume and Unlock… wait: the item's lifecycle is in flight.
    pub(super) waiting: bool,
}

impl LifecycleDialog {
    /// A new run is followed: what the last one reached, and its Stop, are
    /// forgotten.
    pub(super) fn following_anew(&mut self) {
        self.reached = None;
        self.stop_pressed = false;
    }

    /// Stop at a safe point: the run's token is cancelled, and it stops at
    /// its next safe point; a queued run is taken off the queue.
    pub(super) fn stop(&mut self, cx: &mut Context<Self>) {
        let Phase::Submitted(id) = self.phase else {
            return;
        };
        match self.services.worker().cancel(id) {
            CancelResult::Signalled => {
                self.stop_pressed = true;
                self.services.operations().update(cx, |operations, cx| {
                    operations.stopping(id);
                    cx.notify();
                });
            }
            CancelResult::Removed => self.stop_pressed = true,
            CancelResult::TooLate => {}
        }
        cx.notify();
    }

    /// Retry: the shown run's request sent again under its own request ID,
    /// with no confirmation and no passphrase (A2), unless the item's
    /// lifecycle is in flight.
    pub(super) fn resend(&mut self, cx: &mut Context<Self>) {
        let Phase::Submitted(shown) = self.phase else {
            return;
        };
        if self.in_flight_with(None) {
            return;
        }
        let Some((intent, request_id)) = self
            .services
            .operations()
            .read(cx)
            .request(shown)
            .map(|request| (request.intent.clone(), request.request_id))
        else {
            return;
        };
        self.following_anew();
        self.phase = Phase::Submitted(self.services.worker().submit_with_request_id(
            intent,
            Lane::User,
            request_id,
        ));
        cx.notify();
    }

    /// Unlock…: the keys the run was declined for prompt again (R-3), and
    /// the same request is sent again.
    pub(super) fn unlock(&mut self, cx: &mut Context<Self>) {
        if let Phase::Submitted(shown) = self.phase {
            self.services.worker().clear_declined_for(shown);
        }
        self.resend(cx);
    }

    /// Prepare again after an ended run: a fresh preview, whose Confirm sends
    /// a new request.
    pub(super) fn prepare_anew(&mut self, cx: &mut Context<Self>) {
        self.confirmed = None;
        self.plan = None;
        self.following_anew();
        self.prepare(cx);
    }

    /// Closes the dialog and opens `route`'s place in the window's
    /// workspace.
    pub(super) fn route(&mut self, window: &mut Window, cx: &mut Context<Self>, route: ItemRoute) {
        self.in_workspace(window, cx, move |workspace, window, cx| {
            workspace.open_route(route, window, cx)
        });
    }

    pub(super) fn in_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>),
    ) {
        self.close(window, cx);
        let Some(Some(root)) = window.root::<Root>() else {
            return;
        };
        let Ok(workspace) = root.read(cx).view().clone().downcast::<Workspace>() else {
            return;
        };
        workspace.update(cx, |workspace, cx| f(workspace, window, cx));
    }

    /// Open repository details: Settings › Repositories, checking the
    /// repository (L1's `Inspect`).
    pub(super) fn inspect_button(&self, cx: &Context<Self>) -> AnyElement {
        let go = self.on(cx, |dialog, window, cx| {
            if let Some(root) = dialog.root().map(|root| root.to_path_buf()) {
                dialog.in_workspace(window, cx, move |workspace, window, cx| {
                    workspace.inspect_in_settings(root, window, cx)
                });
            }
        });
        button("lifecycle-inspect", OPEN_REPOSITORY_DETAILS, false, go)
    }

    /// What the ended run `id`, answered `envelope`, offers.
    pub(super) fn offers(
        &self,
        id: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
        ending: Ending,
        reach: &Reach,
        way_out: &[LifecycleWayOut],
    ) -> Offers {
        let offered = |kind| envelope.recovery.iter().any(|action| action.action == kind);
        let record = self.services.worker().record_state(id);
        let credential = Credential::of(envelope.code);
        // Republish… and Stop and republish… go first (Task 14): the same
        // request cannot get past a deleted branch.
        let republish = way_out.iter().any(|step| {
            matches!(
                step,
                LifecycleWayOut::Republish { .. } | LifecycleWayOut::CancelThenSendAgain { .. }
            )
        });
        // A run that ended short of main with its record finished or gone
        // has nothing left to continue: a stored refusal, or a stop with no
        // parent, whose request the library deleted (review I-1).
        let ended_short = matches!(ending, Ending::Failed | Ending::Stopped)
            && !reach.integrated
            && record == Some(RecordState::Finished)
            && !offered(RecoveryActionKind::RequestRetry);
        let prepare = offered(RecoveryActionKind::RequestPrepare) || ended_short;
        // A stored refusal names its blockers; one with none kept nothing.
        let blocked = lifecycle_data(envelope).is_some_and(|data| !data.blockers.is_empty());
        let no_record = offered(RecoveryActionKind::RequestPrepare) || (ended_short && !blocked);
        let retry = ending != Ending::Done
            && !prepare
            && !republish
            && credential != Some(Credential::Locked)
            && (offered(RecoveryActionKind::RequestRetry)
                || record == Some(RecordState::Accepted)
                || way_out.contains(&LifecycleWayOut::SendAgain));
        let resume = way_out.iter().find_map(|step| match step {
            LifecycleWayOut::ResumeOperation { operation_id } => Some(operation_id.clone()),
            _ => None,
        });
        // Retry and Resume both go on with the run: the one L1 routes is
        // offered, and Retry when it routes neither (review O-4).
        let retry = retry && (resume.is_none() || way_out.contains(&LifecycleWayOut::SendAgain));
        let resume = resume.filter(|_| !retry);
        let waiting = self.in_flight_with(resume.as_deref());
        Offers {
            retry: ending != Ending::Done && retry,
            prepare,
            no_record,
            resume,
            credential: credential.filter(|_| ending != Ending::Done),
            inspect: way_out.contains(&LifecycleWayOut::Inspect),
            open_folder: lifecycle_data(envelope)
                .is_some_and(|data| data.blockers.contains(&LifecycleBlocker::PrimaryDirty)),
            waiting,
        }
    }

    /// Enter on a sent run: a done one closes, a failed or stopped one
    /// retries, unlocks a locked key, or prepares again where that is its
    /// way on.
    pub(super) fn enter_submitted(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Submitted(id) = self.phase else {
            return;
        };
        let Some(envelope) = self
            .services
            .operations()
            .read(cx)
            .lifecycle_answer(id)
            .cloned()
        else {
            return;
        };
        let ending = Ending::of(&envelope);
        if ending == Ending::Done {
            self.close(window, cx);
            return;
        }
        let reach = Reach::of(&envelope, self.reached);
        let offers = self.offers(id, &envelope, ending, &reach, &lifecycle_way_out(&envelope));
        if offers.retry && !offers.waiting {
            self.resend(cx);
        } else if offers.credential == Some(Credential::Locked) && !offers.waiting {
            // Unlock… is a locked key's retry (review M-6).
            self.unlock(cx);
        } else if offers.prepare {
            self.prepare_anew(cx);
        }
    }
}
