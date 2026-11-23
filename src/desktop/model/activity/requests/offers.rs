//! A request row's offers and tooltip lines (B2's button and menu, B5's
//! detail): what the row can be asked to do, and what it says of the
//! request in full.

use manyhands::results::{Outcome, RecoveryActionKind};

use super::{Row, offer};
use crate::desktop::{
    model::{
        activity::{
            copy::{
                ALREADY_FINISHED, CANCEL, CANCELLED, FINISHING, LISTS_CATCHING_UP, NOT_SENT,
                OPEN_LIFECYCLE, QUEUED, REPUBLISHING, REVIEW_CHANGES, RUNNING, STOP, STOPPING,
                effects_text, offer_label, outcome_label, scope_name,
            },
            rows::{ActivityAction, Offer},
        },
        operations::StopOffer,
        publication::{SyncOffer, SyncOutcome},
    },
    services::requests::{Intent, RequestState},
};

impl Row<'_> {
    /// The button and the menu without Dismiss (B2): the button's offer
    /// first, then the others in today's control order.
    pub(super) fn offers(&self, owed: bool) -> (Option<Offer>, Vec<Offer>) {
        let id = self.id();
        let (state, facts) = (&self.request.state, &self.facts);
        let single = |action, label, enabled| {
            let button = offer(action, label, enabled);
            (Some(button.clone()), vec![button])
        };
        match state {
            RequestState::Queued
            | RequestState::Running {
                accepted: false, ..
            } => single(ActivityAction::Cancel(id), CANCEL, true),
            RequestState::Running { accepted: true, .. } => match facts.stop_offer {
                StopOffer::Offered => single(ActivityAction::Stop(id), STOP, true),
                StopOffer::Stopping => single(ActivityAction::Stop(id), STOPPING, false),
                StopOffer::NotOffered => single(ActivityAction::Stop(id), FINISHING, false),
            },
            RequestState::Finished(Outcome::Success | Outcome::Noop, ..)
            | RequestState::Removed
            | RequestState::NotSent(_)
            | RequestState::Dismissed => (None, Vec::new()),
            RequestState::Finished(..) if owed => (None, Vec::new()),
            RequestState::Finished(_, _, _, recovery, _) => self.finished_offers(recovery),
        }
    }

    /// A finished request that did not succeed: Review changes, the first
    /// share offer, the first shown recovery action, then Open… (B2). Risky
    /// actions are never the button.
    fn finished_offers(
        &self,
        recovery: &[manyhands::results::RecoveryAction],
    ) -> (Option<Offer>, Vec<Offer>) {
        let id = self.id();
        // (offer, risky), in today's control order: Open…, Review changes,
        // then the share offers or the recovery actions.
        let mut ordered: Vec<(Offer, bool)> = Vec::new();
        let lifecycle = matches!(
            self.request.intent,
            Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
        );
        let open = lifecycle.then(|| {
            ordered.push((offer(ActivityAction::Open(id), OPEN_LIFECYCLE, true), false));
            ordered.len() - 1
        });
        let review = self.facts.review.then(|| {
            ordered.push((
                offer(ActivityAction::Review(id), REVIEW_CHANGES, true),
                false,
            ));
            ordered.len() - 1
        });
        let first = ordered.len();
        match self.operations.share_result(id) {
            Some(result) => {
                // A republish that runs reads Republishing…, disabled, in
                // place of the Republish… it hides.
                let republishing = result.outcome == SyncOutcome::BranchDeleted
                    && self.facts.share_words.is_some();
                if republishing {
                    ordered.push((
                        offer(
                            ActivityAction::Offer(id, SyncOffer::Republish),
                            REPUBLISHING,
                            false,
                        ),
                        false,
                    ));
                }
                for shown in &self.facts.shown_offers {
                    let risky = matches!(shown, SyncOffer::GiveUp | SyncOffer::StopAndRepublish);
                    ordered.push((
                        offer(ActivityAction::Offer(id, *shown), offer_label(*shown), true),
                        risky,
                    ));
                }
            }
            None => {
                for (n, label, enabled) in &self.facts.recoveries {
                    let risky = recovery.get(*n).is_some_and(|action| {
                        action.action == RecoveryActionKind::OperationAbandon
                    });
                    ordered.push((
                        offer(ActivityAction::Recovery(id, *n), label, *enabled),
                        risky,
                    ));
                }
            }
        }
        let share_or_recovery = ordered[first..]
            .iter()
            .position(|(_, risky)| !risky)
            .map(|at| first + at);
        let Some(pick) = review.or(share_or_recovery).or(open) else {
            return (None, ordered.into_iter().map(|(offer, _)| offer).collect());
        };
        let button = ordered[pick].0.clone();
        let mut menu = vec![button.clone()];
        menu.extend(
            ordered
                .into_iter()
                .enumerate()
                .filter(|(at, _)| *at != pick)
                .map(|(_, (offer, _))| offer),
        );
        (Some(button), menu)
    }

    /// The tooltip's lines after the message and the time (B5).
    pub(super) fn detail(&self, owed: Option<bool>) -> Vec<String> {
        let intent = &self.request.intent;
        let mut lines = vec![format!(
            "{} · {}",
            intent.label_in(self.primary.as_deref()),
            scope_name(intent.root())
        )];
        match &self.request.state {
            RequestState::Queued => lines.push(QUEUED.to_owned()),
            RequestState::Running { stage, accepted } => {
                let finishing = *accepted && stage.is_none();
                lines.push(if finishing { FINISHING } else { RUNNING }.to_owned());
            }
            RequestState::Finished(outcome, ..) => lines.push(outcome_label(*outcome).to_owned()),
            RequestState::Removed => lines.push(CANCELLED.to_owned()),
            RequestState::NotSent(_) => lines.push(NOT_SENT.to_owned()),
            RequestState::Dismissed => {}
        }
        if let RequestState::Finished(outcome, code, effects, _, message) = &self.request.state {
            if !matches!(outcome, Outcome::Success | Outcome::Noop) {
                lines.push(format!("{}: {message}", code.as_str()));
            }
            let shown = effects_text(effects);
            if shown != effects_text(&manyhands::results::Effects::not_requested()) {
                lines.push(format!("Effects: {shown}"));
            }
        }
        if let Some(operation) = self.operation() {
            lines.push(format!("Operation {operation}"));
        }
        if let RequestState::Running {
            stage: Some(stage), ..
        } = &self.request.state
            && let Some(words) = self.stage_words(*stage)
        {
            lines.push(words);
        }
        if self.facts.already_finished {
            lines.push(ALREADY_FINISHED.to_owned());
        }
        if owed == Some(true) {
            lines.push(LISTS_CATCHING_UP.to_owned());
        }
        lines
    }
}
