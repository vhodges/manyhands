//! A finished share's outcome in its operation area entry (design, Lane
//! B, Outcomes and routes; decision 2): the outcome's fixed words and the
//! controls it offers under its record, in place of the envelope's own
//! recovery actions. The views render `SyncOutcome` and never read the
//! envelope themselves.
//!
//! - Share sends a new request of the same share, and Unlock… the same
//!   request again; each first clears the keys its last attempt was
//!   answered Not now for, so the next request prompts for them (R-3,
//!   R-13, R-17).
//! - Retry sends the same request again; Stop trying to share gives it up
//!   (decision 8).
//! - The rest open a place in the workspace (`ItemRoute`): Open Sharing &
//!   keys, Check host (the host the outcome names, or the repository's
//!   Hosts page, K-3), Set identity, Set up sharing (Remotes), and Show it,
//!   on `busy`, the request that may hold the repository, or the pending
//!   operations with none (Busy repositories).
//!
//! The conflict outcome offers nothing (decision 13).

use std::path::PathBuf;

use gpui_kit::{AnyElement, App, ClickEvent, Context, Window};

use super::{button, request_id};
use crate::desktop::{
    model::{
        item::SHARE,
        operations::sync_text,
        publication::{ShareTarget, SyncOffer, SyncOutcome, SyncResult},
    },
    services::{
        Services,
        requests::{DesktopRequestId, RequestState},
        worker::{GaveUp, RecordState},
    },
    view::{item::ItemRoute, sidebar::RETRY, workspace::Workspace},
};

pub(crate) const UNLOCK: &str = "Unlock…";
/// Give up (decision 8).
pub(crate) const GIVE_UP: &str = "Stop trying to share";
/// A Give up that found its request's record already finished: nothing
/// was cancelled, and the entry shows the stored answer (decision 8).
pub(crate) const ALREADY_FINISHED: &str = "Already finished";
pub(crate) const OPEN_SHARING_AND_KEYS: &str = "Open Sharing & keys";
pub(crate) const CHECK_HOST: &str = "Check host";
pub(crate) const SET_IDENTITY: &str = "Set identity";
pub(crate) const SHOW_IT: &str = "Show it";
pub(crate) const SET_UP_SHARING: &str = "Set up sharing";

/// An offer's label.
pub(crate) fn offer_label(offer: SyncOffer) -> &'static str {
    match offer {
        SyncOffer::Share => SHARE,
        SyncOffer::Retry => RETRY,
        SyncOffer::GiveUp => GIVE_UP,
        SyncOffer::Unlock => UNLOCK,
        SyncOffer::OpenSharingAndKeys => OPEN_SHARING_AND_KEYS,
        SyncOffer::CheckHost => CHECK_HOST,
        SyncOffer::SetIdentity => SET_IDENTITY,
        SyncOffer::ShowHolder => SHOW_IT,
        SyncOffer::SetUpSharing => SET_UP_SHARING,
    }
}

/// The part of an entry's element ids an offer's control takes. Share's
/// is the stopped share's (design, Outcomes and routes).
pub(crate) fn offer_part(offer: SyncOffer) -> &'static str {
    match offer {
        SyncOffer::Share => "share",
        SyncOffer::Retry => "share-retry",
        SyncOffer::GiveUp => "give-up",
        SyncOffer::Unlock => "unlock",
        SyncOffer::OpenSharingAndKeys => "sharing-and-keys",
        SyncOffer::CheckHost => "check-host",
        SyncOffer::SetIdentity => "set-identity",
        SyncOffer::ShowHolder => "show-holder",
        SyncOffer::SetUpSharing => "set-up-sharing",
    }
}

/// The outcome of the finished share `id`, under where its record stood
/// after its last attempt (unknown when the worker kept none) and whether
/// that attempt was a Give up's.
pub(crate) fn share_result(
    services: &Services,
    id: DesktopRequestId,
    cx: &App,
) -> Option<SyncResult> {
    let worker = services.worker();
    let record = worker.record_state(id).unwrap_or(RecordState::Unknown);
    let after_give_up = worker.gave_up(id).is_some();
    services
        .operations()
        .read(cx)
        .share_result(id, record, after_give_up)
}

/// Whether the last Give up of the request `id` found its record already
/// finished, until it is sent again.
pub(crate) fn already_finished(services: &Services, id: DesktopRequestId) -> bool {
    services.worker().gave_up(id) == Some(GaveUp::AlreadyFinished)
}

/// The request of this session that may hold the repository the share
/// `id` was answered `busy` for (Busy repositories).
pub(crate) fn busy_holder(
    services: &Services,
    id: DesktopRequestId,
    cx: &App,
) -> Option<DesktopRequestId> {
    let worker = services.worker();
    services
        .operations()
        .read(cx)
        .busy_holder(id, |holder| worker.record_state(holder))
}

/// What the finished share `id`, of outcome `result`, says; `None` for a
/// failure, whose message and code the entry shows.
pub(crate) fn share_words(
    services: &Services,
    id: DesktopRequestId,
    result: &SyncResult,
    cx: &App,
) -> Option<String> {
    let holder = matches!(result.outcome, SyncOutcome::Busy)
        .then(|| busy_holder(services, id, cx))
        .flatten()
        .and_then(|holder| services.operations().read(cx).request(holder))
        .map(|holder| holder.intent.label());
    sync_text(&result.outcome, holder)
}

/// Where the offer `offer` of the finished share `id` goes in the
/// workspace, for an offer that opens a place; `None` for the rest.
pub(crate) fn share_route(
    services: &Services,
    id: DesktopRequestId,
    result: &SyncResult,
    offer: SyncOffer,
    cx: &App,
) -> Option<ItemRoute> {
    let root = || -> Option<PathBuf> {
        Some(
            services
                .operations()
                .read(cx)
                .request(id)?
                .intent
                .root()?
                .to_owned(),
        )
    };
    Some(match offer {
        SyncOffer::OpenSharingAndKeys => ItemRoute::SharingAndKeys,
        SyncOffer::CheckHost => ItemRoute::CheckHost {
            root: root()?,
            authority: match &result.outcome {
                SyncOutcome::NeedsHost { authority } => authority.clone(),
                _ => None,
            },
        },
        SyncOffer::SetIdentity => ItemRoute::SetIdentity(root()?),
        SyncOffer::SetUpSharing => ItemRoute::Remotes(root()?),
        SyncOffer::ShowHolder => ItemRoute::Holder {
            root: root()?,
            request: busy_holder(services, id, cx),
        },
        SyncOffer::Share | SyncOffer::Retry | SyncOffer::GiveUp | SyncOffer::Unlock => {
            return None;
        }
    })
}

impl Workspace {
    /// The outcome of the finished request `id`, if it is a share.
    pub(super) fn finished_share(&self, id: DesktopRequestId, cx: &App) -> Option<SyncResult> {
        let request = self.services().operations().read(cx).request(id)?;
        if !matches!(request.state, RequestState::Finished(..))
            || ShareTarget::of(&request.intent).is_none()
        {
            return None;
        }
        share_result(self.services(), id, cx)
    }

    /// An offer's control on the entry of the finished share `id`.
    pub(super) fn share_offer(
        &mut self,
        id: DesktopRequestId,
        offer: SyncOffer,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let part = request_id(id, offer_part(offer));
        let focus = self.control_focus_for(part.clone(), cx);
        button(
            part,
            offer_label(offer),
            Some(&focus),
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.run_share_offer(id, offer, window, cx)
            }),
        )
    }

    /// Runs the offer `offer` of the finished share `id`.
    pub(in crate::desktop::view) fn run_share_offer(
        &mut self,
        id: DesktopRequestId,
        offer: SyncOffer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = self.finished_share(id, cx) else {
            return;
        };
        if !result.offers().contains(&offer) {
            return;
        }
        let worker = self.services().worker().clone();
        match offer {
            // R-17: the new request prompts for a key the last declined.
            SyncOffer::Share => {
                worker.clear_declined_for(id);
                self.share_request_again(id, cx);
            }
            SyncOffer::Retry => {
                worker.retry(id, None);
            }
            // R-3: the same request, prompting again.
            SyncOffer::Unlock => {
                worker.clear_declined_for(id);
                worker.retry(id, None);
            }
            SyncOffer::GiveUp => {
                worker.give_up(id);
            }
            _ => {
                if let Some(route) = share_route(self.services(), id, &result, offer, cx) {
                    self.open_route(route, window, cx);
                }
            }
        }
        cx.notify();
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "outcomes_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "outcomes_route_tests.rs"]
mod route_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "give_up_tests.rs"]
mod give_up_tests;
