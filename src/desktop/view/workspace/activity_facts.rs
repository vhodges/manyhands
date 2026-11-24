//! What only the Workspace knows of a request, worked out at each refresh for
//! Activity's rows (design A2, R-9; ruling T3-2): the offers its answer
//! shows today, its recovery actions that this session can run, what a
//! republish or a busy holder says, and what a resend, a follow-up and the
//! scheduler record of it.

use gpui_kit::App;
use manyhands::results::RecoveryAction;

use super::Workspace;
use crate::desktop::{
    model::{activity::requests::RequestFacts, operations::recovery_control},
    services::requests::{DesktopRequestId, RequestState},
    view::{
        lifecycle,
        operations::{already_finished, dialog_owns, facts_words, shown_offers},
    },
};

impl Workspace {
    /// The facts of request `id`.
    pub(super) fn request_facts(&self, id: DesktopRequestId, cx: &App) -> RequestFacts {
        let services = &self.services;
        let operations = services.operations().read(cx);
        let Some(request) = operations.request(id) else {
            return RequestFacts::none();
        };
        let remote = request
            .intent
            .root()
            .and_then(|root| root.to_str())
            .and_then(|root| self.navigation().entry(root))
            .and_then(|entry| entry.summary.configuration.publication_remote.clone());
        let finished = matches!(request.state, RequestState::Finished(..));
        let result = operations.share_result(id);
        let recoveries = match (&request.state, result) {
            (RequestState::Finished(_, _, _, recovery, _), None) => {
                let lifecycle = lifecycle::item_of(&request.intent).is_some();
                self.recoveries(id, recovery, lifecycle, cx)
            }
            _ => Vec::new(),
        };
        // A resend follows from the request while its follow-up is shown; it
        // did nothing when that ended Removed or NotSent (N23).
        let follow = services
            .resends()
            .follow(id)
            .and_then(|follow| operations.request(follow))
            .filter(|follow| !matches!(follow.state, RequestState::Dismissed));
        let scheduler = services.scheduler().read(cx);
        RequestFacts {
            stop_offer: operations.stop_offer(id, remote.is_some()),
            shown_offers: result
                .map(|result| shown_offers(services, id, result, cx))
                .unwrap_or_default(),
            share_words: result.and_then(|result| facts_words(services, id, result, cx)),
            already_finished: finished && already_finished(services, id),
            review: finished && self.entry_of(id, cx).is_some(),
            recoveries,
            resend_waits: services.resends().waits_on(id),
            followed: follow.is_some(),
            follow_void: follow.is_some_and(|follow| {
                matches!(
                    follow.state,
                    RequestState::Removed | RequestState::NotSent(_)
                )
            }),
            check_applied: scheduler.check_applied(id),
            superseded: scheduler.check_superseded(id),
        }
    }

    /// The recovery actions of a finished request's answer that its row shows
    /// today: not a lifecycle dialog's own, not one a saved post hides, each
    /// with its words and whether this session can run it.
    fn recoveries(
        &self,
        id: DesktopRequestId,
        recovery: &[RecoveryAction],
        lifecycle: bool,
        cx: &App,
    ) -> Vec<(usize, String, bool)> {
        let operations = self.services.operations().read(cx);
        recovery
            .iter()
            .enumerate()
            .filter(|(_, action)| !(lifecycle && dialog_owns(recovery_control(action.action))))
            .filter(|(_, action)| !operations.hides_recovery(id, action))
            .map(|(n, action)| {
                (
                    n,
                    operations.recovery_words(id, action).to_owned(),
                    operations.recovery_step(id, action).is_some(),
                )
            })
            .collect()
    }
}
