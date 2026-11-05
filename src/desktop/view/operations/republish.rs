//! Republish… and Stop and republish… (D5 Part C, A3; P17, P21), from a
//! share's entry and from the lifecycle dialog's way out.
//!
//! - **Republish…** opens the confirmation dialog on `item republish`
//!   (`confirm_followed`, its Republish arm). Once confirmed from a
//!   lifecycle's way out, the lifecycle goes on when the republish is done
//!   (`Resends::after_republish`): its request again, or a resume of its
//!   operation. A stopped republish offers the library's Retry on its
//!   entry, the same request again (P17).
//! - **Stop and republish…** gives the stopped request up (`Worker::
//!   give_up`): its record is cancelled and the same request sent again,
//!   which lets the reservation go. Only when that Give up was sent on an
//!   accepted record (`GaveUp::Resent`) and answered `cancelled` does
//!   Republish… open; any other answer ends the sequence, and the entry or
//!   the dialog shows that answer, so a `busy` never loops (P21). The
//!   workspace waits for the answer; one a lifecycle dialog asked for ends
//!   when that dialog closes (review M5).

use std::{path::PathBuf, rc::Rc};

use gpui_kit::{Context, FocusHandle, WeakEntity, Window};
use manyhands::{canonical::ItemId, repository::AuthoringKind, results::ResultCode};

use crate::desktop::{
    services::{
        requests::{DesktopRequestId, Intent, RequestState},
        resends::AfterRepublish,
        worker::GaveUp,
    },
    view::{
        confirm::{self, Followed},
        lifecycle::LifecycleDialog,
        workspace::Workspace,
    },
};

pub(crate) const REPUBLISH: &str = "Republish\u{2026}";
pub(crate) const STOP_AND_REPUBLISH: &str = "Stop and republish\u{2026}";
/// A share's deleted branch whose republish is queued or running.
pub(crate) const REPUBLISHING: &str = "Republishing\u{2026}";

/// The item to republish, and how its lifecycle goes on once it is done:
/// `None` for a share's, which nothing follows.
#[derive(Clone)]
pub(crate) struct Republishing {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub after: Option<AfterRepublish>,
}

/// A Stop and republish… waiting for its Give up's answer.
pub(crate) struct StopThenRepublish {
    /// The lifecycle dialog that asked for it, if one did: closed, it ends
    /// the wait, so Republish… never opens out of context (review M5).
    dialog: Option<WeakEntity<LifecycleDialog>>,
    request: DesktopRequestId,
    /// The attempt the Give up sends: an answer to an earlier one is not
    /// its.
    attempt: u32,
    target: Republishing,
    origin: FocusHandle,
}

impl Workspace {
    /// Republish…: the confirmation dialog on `item republish` of the
    /// target's item. Closing it focuses `origin`.
    pub(in crate::desktop::view) fn republish(
        &mut self,
        target: Republishing,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Republishing {
            root,
            kind,
            id,
            after,
        } = target;
        let intent = Intent::ItemRepublish { root, kind, id };
        let services = self.services().clone();
        let Some(dialog) = confirm::confirm_followed(
            intent,
            Rc::new(move |_| origin.clone()),
            &services,
            window,
            cx,
        ) else {
            return;
        };
        if let Some(after) = after {
            let resends = services.resends().clone();
            cx.subscribe_in(&dialog, window, move |_, _, followed: &Followed, _, _| {
                if let Followed::Confirmed { request, .. } = followed {
                    resends.after_republish(*request, after.clone());
                }
            })
            .detach();
        }
    }

    /// Stop and republish…: the request `id` given up, and Republish… of
    /// `target` once it answers stopped. False when the Give up could not
    /// be queued: the request is queued or running, or cannot be given up.
    pub(in crate::desktop::view) fn stop_and_republish(
        &mut self,
        id: DesktopRequestId,
        target: Republishing,
        origin: FocusHandle,
        dialog: Option<WeakEntity<LifecycleDialog>>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(attempt) = self
            .services()
            .operations()
            .read(cx)
            .request(id)
            .map(|request| request.attempt + 1)
        else {
            return false;
        };
        if !self.services().worker().give_up(id) {
            return false;
        }
        self.stopping_to_republish.push(StopThenRepublish {
            dialog,
            request: id,
            attempt,
            target,
            origin,
        });
        cx.notify();
        true
    }

    /// Each Stop and republish… whose Give up has its answer: a stop of an
    /// accepted record opens Republish…, once the dialogs that follow the
    /// answer have done so; anything else ends it.
    pub(in crate::desktop::view) fn stops_to_republish_answered(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.stopping_to_republish.is_empty() {
            return;
        }
        let worker = self.services().worker().clone();
        let operations = self.services().operations().read(cx);
        let mut ready = Vec::new();
        self.stopping_to_republish.retain(|waiting| {
            if waiting
                .dialog
                .as_ref()
                .is_some_and(|dialog| dialog.upgrade().is_none())
            {
                return false;
            }
            let Some(request) = operations.request(waiting.request) else {
                return false;
            };
            // The model holds the earlier attempt until the Give up's
            // `Queued` event arrives.
            if request.attempt < waiting.attempt {
                return true;
            }
            match &request.state {
                RequestState::Queued | RequestState::Running { .. } => true,
                RequestState::Finished(_, code, ..) => {
                    if *code == ResultCode::Cancelled
                        && worker.gave_up(waiting.request) == Some(GaveUp::Resent)
                    {
                        ready.push((waiting.target.clone(), waiting.origin.clone()));
                    }
                    false
                }
                RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_) => false,
            }
        });
        for (target, origin) in ready {
            cx.defer_in(window, move |this, window, cx| {
                this.republish(target, origin, window, cx)
            });
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "republish_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "republish_stop_tests.rs"]
mod stop_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "republish_follow_tests.rs"]
mod follow_tests;
