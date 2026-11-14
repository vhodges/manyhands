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
//! - An item whose shared copy was deleted offers Republish…, confirmed,
//!   or, while the share's record is accepted, Stop and republish…, which
//!   gives the share up and then opens Republish… (D5 Part C,
//!   `republish.rs`).
//! - The rest open a place in the workspace (`ItemRoute`): Open Sharing &
//!   keys, Check host (the host the outcome names, or the repository's
//!   Hosts page, K-3), Set identity, Set up sharing (Remotes), and Show it,
//!   on `busy`, the request that may hold the repository, or the pending
//!   operations with none (Busy repositories).
//!
//! The conflict outcome offers nothing (decision 13).

use std::path::PathBuf;

use gpui_kit::{App, Context, FocusHandle, Window};

use super::republish::{REPUBLISHING, Republishing};
use crate::desktop::{
    model::{
        operations::{may_be_waiting_for, share_text},
        publication::{ShareTarget, SyncOffer, SyncOutcome, SyncResult},
    },
    services::{
        Services,
        requests::{DesktopRequestId, Intent, RequestState},
        worker::GaveUp,
    },
    view::{item::ItemRoute, workspace::Workspace},
};

// The offers' words live in the model, where Activity's rows read them
// (R-44); each keeps its old path where a test still names it.
#[cfg(test)]
pub(crate) use crate::desktop::model::activity::copy::offer_label;

/// The outcome of the finished share `id`, classified once when it
/// finished from where its record stood after its last attempt (unknown
/// when the worker kept none) and whether that attempt was a Give up's.
pub(crate) fn share_result(
    services: &Services,
    id: DesktopRequestId,
    cx: &App,
) -> Option<SyncResult> {
    services.operations().read(cx).share_result(id).cloned()
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

/// Whether the share `id` is of an item a republish of which is queued or
/// running, or, with `waiting`, waits to be done from a lifecycle's way out
/// (R-30; review 4).
fn republishing(services: &Services, id: DesktopRequestId, waiting: bool, cx: &App) -> bool {
    match services
        .operations()
        .read(cx)
        .request(id)
        .map(|request| &request.intent)
    {
        Some(Intent::ItemSync { root, id, .. }) if waiting => services.republishing(root, id),
        Some(Intent::ItemSync { root, id, .. }) => services.republish_running(root, id),
        _ => false,
    }
}

/// The offers the finished share `id`, of outcome `result`, shows: its
/// own, but for Republish… and Stop and republish… while a republish of
/// its item is queued, running, or waits to be done (R-30; review 4).
pub(crate) fn shown_offers(
    services: &Services,
    id: DesktopRequestId,
    result: &SyncResult,
    cx: &App,
) -> Vec<SyncOffer> {
    let republishing = republishing(services, id, true, cx);
    result
        .offers()
        .into_iter()
        .filter(|offer| {
            !(republishing && matches!(offer, SyncOffer::Republish | SyncOffer::StopAndRepublish))
        })
        .collect()
}

/// What Activity's row of the finished share `id` is told of what only the
/// Workspace knows (R-9): that a republish of its deleted branch is queued
/// or running, or the words naming the request that may hold a busy
/// repository. Any `Some` for a deleted branch means "republish running".
pub(in crate::desktop::view) fn facts_words(
    services: &Services,
    id: DesktopRequestId,
    result: &SyncResult,
    cx: &App,
) -> Option<String> {
    match result.outcome {
        SyncOutcome::BranchDeleted => {
            republishing(services, id, false, cx).then(|| REPUBLISHING.to_owned())
        }
        SyncOutcome::Busy => busy_holder(services, id, cx)
            .and_then(|holder| services.operations().read(cx).request(holder))
            .map(|holder| may_be_waiting_for(holder.intent.label())),
        _ => None,
    }
}

/// What the finished share `id`, of outcome `result`, says; `None` for a
/// failure, whose message and code the entry shows. A deleted branch
/// whose republish is queued or running says so, on the entry and the
/// item's pill. A share of the primary branch names it `primary`, when
/// known (R-35).
pub(crate) fn share_words(
    services: &Services,
    id: DesktopRequestId,
    result: &SyncResult,
    primary: Option<&str>,
    cx: &App,
) -> Option<String> {
    if result.outcome == SyncOutcome::BranchDeleted && republishing(services, id, false, cx) {
        return Some(REPUBLISHING.to_owned());
    }
    let holder = matches!(result.outcome, SyncOutcome::Busy)
        .then(|| busy_holder(services, id, cx))
        .flatten()
        .and_then(|holder| services.operations().read(cx).request(holder))
        .map(|holder| holder.intent.label());
    share_text(&result.outcome, result.attempt.target, holder, primary)
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
        SyncOffer::Share
        | SyncOffer::Retry
        | SyncOffer::GiveUp
        | SyncOffer::Unlock
        | SyncOffer::Republish
        | SyncOffer::StopAndRepublish => {
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

    /// The item the share `id` is of, to republish: no lifecycle follows.
    fn share_republished(&self, id: DesktopRequestId, cx: &App) -> Option<Republishing> {
        match &self.services().operations().read(cx).request(id)?.intent {
            Intent::ItemSync { root, kind, id } => Some(Republishing {
                root: root.clone(),
                kind: *kind,
                id: id.clone(),
                after: None,
            }),
            _ => None,
        }
    }

    /// Runs the offer `offer` of the finished share `id`. Republish…'s
    /// dialog gives focus back to `origin`.
    pub(in crate::desktop::view) fn run_share_offer(
        &mut self,
        id: DesktopRequestId,
        offer: SyncOffer,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = self.finished_share(id, cx) else {
            return;
        };
        if !shown_offers(self.services(), id, &result, cx).contains(&offer) {
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
            SyncOffer::Republish | SyncOffer::StopAndRepublish => {
                let Some(target) = self.share_republished(id, cx) else {
                    return;
                };
                if offer == SyncOffer::Republish {
                    self.republish(target, origin, window, cx);
                } else {
                    self.stop_and_republish(id, target, origin, None, cx);
                }
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
pub(in crate::desktop::view) mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "outcomes_route_tests.rs"]
mod route_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "give_up_tests.rs"]
mod give_up_tests;
