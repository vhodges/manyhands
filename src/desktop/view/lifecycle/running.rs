//! The lifecycle dialog once a run is sent (D5 design, A2 Running, A3
//! Outcomes), drawn, with its buttons. What it says is `outcome.rs`'s.
//!
//! - **Running.** Each step with its state, lit from L3's stages, and Stop
//!   at a safe point, which cancels the run's token and reads Stopping…
//!   once pressed. Escape hides the dialog; the run goes on, and the
//!   operation area's entry opens it again.
//! - **Done, Failed, Stopped.** How it ended; each step's state; for a
//!   stop, that the cancellation finished and which steps completed
//!   (MH-COLLAB-006); the outcome lines; a failure's blockers, message and
//!   code; and the way out (L1): Retry, which sends the same request ID
//!   again with no confirmation and no passphrase (`submit_with_request_id`;
//!   P7), so a protected key takes D4's unlock path; Resume; Prepare again,
//!   a new request, where the library deleted the request (P18) or its
//!   stored answer stands before main; Open repository details; Open
//!   folder for a dirty main (P19); and D4's credential recovery: Unlock…,
//!   Open Sharing & keys, Check host. The rest of L1 is guidance, its
//!   buttons being later tasks'. Retry, Resume and Unlock… wait while the
//!   item's lifecycle is in flight, which the dialog then says (P22).
//!   Enter retries a failed or stopped run, and closes a done one.

use gpui_kit::{
    AnyElement, App, Context, FontWeight, IntoElement, ParentElement, Styled,
    component::{ActiveTheme, h_flex, v_flex},
};
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleMutationDto, LifecyclePublicationPendingReason, MutationDataDto,
        lifecycle_way_out,
    },
    results::Envelope,
};

use super::{
    LifecycleDialog, NOT_SENT, OPEN_FOLDER, PREPARE_AGAIN, RESUME, RETRY, STARTED, STILL_RUNNING,
    actions::Offers,
    button,
    confirm::{Step, steps},
    outcome::{
        CANCELLATION_FINISHED, CHECK_HOST, Credential, DISMISSED, Ending, FINISHED_BEFORE_STOP,
        NO_RECORD, OPEN_SHARING_AND_KEYS, REMOVED, Reach, STOP, STOPPING, StepState, UNLOCK,
        guidance, lifecycle_data, outcome_lines, running_states, steps_by_kind,
    },
    prepare::blocker_words,
    text,
};
use crate::desktop::{
    model::publication::host_authority,
    services::requests::{DesktopRequestId, RequestState},
    view::item::ItemRoute,
};

impl LifecycleDialog {
    fn item_word(&self) -> &'static str {
        match self.heading.kind {
            ItemDtoKind::Ticket => "ticket",
            ItemDtoKind::Document => "document",
        }
    }

    /// The steps shown: the confirmed preview's, or by the item's kind.
    pub(super) fn shown_steps(&self, answer: Option<&LifecycleMutationDto>) -> Vec<Step> {
        match &self.plan {
            Some(plan) => steps(plan),
            None => steps_by_kind(
                self.heading.kind,
                answer,
                &self.primary(),
                self.heading.remote.as_ref().map(Option::as_deref),
            ),
        }
    }

    fn primary(&self) -> String {
        // The confirmed preview's, else the one the workspace knows.
        self.plan
            .as_ref()
            .map(|plan| plan.primary_branch.clone())
            .or_else(|| self.heading.primary.clone())
            .unwrap_or_else(|| "main".to_owned())
    }

    /// The body and buttons of a sent run, by where its request stands.
    pub(super) fn submitted_body(
        &self,
        id: DesktopRequestId,
        cx: &Context<Self>,
    ) -> (AnyElement, Vec<AnyElement>) {
        let operations = self.services.operations().read(cx);
        let state = operations.request(id).map(|request| request.state.clone());
        let answer = operations.lifecycle_answer(id).cloned();
        match (state, answer) {
            (Some(RequestState::Removed), _) => (self.message(REMOVED), Vec::new()),
            (Some(RequestState::NotSent(_)), _) => (self.message(NOT_SENT), Vec::new()),
            // Its answer went with its entry (review O-1).
            (Some(RequestState::Dismissed), None) => (self.message(DISMISSED), Vec::new()),
            (_, Some(envelope)) => self.ended(id, &envelope, cx),
            _ => self.running(cx),
        }
    }

    fn message(&self, words: &'static str) -> AnyElement {
        text("lifecycle-message", words)
            .text_sm()
            .into_any_element()
    }

    fn step_rows(steps: &[Step], states: &[StepState], cx: &App) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_1()
            .children(
                steps
                    .iter()
                    .zip(states)
                    .enumerate()
                    .map(|(ix, (step, state))| {
                        h_flex()
                            .gap_2()
                            .child(
                                text(("lifecycle-step-state", ix), state.words())
                                    .w(gpui_kit::px(84.))
                                    .flex_none()
                                    .text_xs()
                                    .text_color(muted),
                            )
                            .child(text(("lifecycle-step", ix), step.title.clone()).text_sm())
                    }),
            )
            .into_any_element()
    }

    fn running(&self, cx: &Context<Self>) -> (AnyElement, Vec<AnyElement>) {
        let steps = self.shown_steps(None);
        let keys: Vec<_> = steps.iter().map(|step| step.key).collect();
        let body = v_flex()
            .gap_2()
            .child(Self::step_rows(
                &steps,
                &running_states(&keys, self.reached),
                cx,
            ))
            .child(
                text("lifecycle-state", STARTED)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground),
            )
            .into_any_element();
        let stop = if self.stop_pressed {
            text("lifecycle-stopping", STOPPING)
                .text_sm()
                .into_any_element()
        } else {
            button(
                "lifecycle-stop",
                STOP,
                false,
                self.on(cx, |dialog, _, cx| dialog.stop(cx)),
            )
        };
        (body, vec![stop])
    }

    fn ended(
        &self,
        id: DesktopRequestId,
        envelope: &Envelope<MutationDataDto>,
        cx: &Context<Self>,
    ) -> (AnyElement, Vec<AnyElement>) {
        let muted = cx.theme().muted_foreground;
        let ending = Ending::of(envelope);
        let data = lifecycle_data(envelope);
        let reach = Reach::of(envelope, self.reached);
        let way_out = lifecycle_way_out(envelope);
        let offers = self.offers(id, envelope, ending, &reach, &way_out);
        let steps = self.shown_steps(data);
        let keys: Vec<_> = steps.iter().map(|step| step.key).collect();
        let local = data.is_some_and(|data| {
            data.publication_pending == Some(LifecyclePublicationPendingReason::NoPublicationRemote)
        });
        let states = match ending {
            Ending::StillRunning => running_states(&keys, self.reached),
            _ => reach.states(&keys, ending, local),
        };
        let mut body = v_flex()
            .gap_2()
            .child(text("lifecycle-ending", ending.words()).font_weight(FontWeight::SEMIBOLD));
        if ending == Ending::Stopped {
            let completed: Vec<&str> = steps
                .iter()
                .zip(&states)
                .filter(|(_, state)| **state == StepState::Done)
                .map(|(step, _)| step.title.as_str())
                .collect();
            body = body
                .child(text("lifecycle-cancellation", CANCELLATION_FINISHED).text_sm())
                .child(
                    text(
                        "lifecycle-completed",
                        format!("Completed: {}.", completed.join(", ")),
                    )
                    .text_sm(),
                );
        }
        if ending == Ending::Done && self.stop_pressed {
            body = body.child(text("lifecycle-stop-late", FINISHED_BEFORE_STOP).text_sm());
        }
        body = body.child(Self::step_rows(&steps, &states, cx));
        if ending != Ending::StillRunning {
            body = body.children(
                outcome_lines(envelope, &reach, &self.primary())
                    .into_iter()
                    .enumerate()
                    .map(|(ix, line)| text(("lifecycle-outcome", ix), line).text_sm()),
            );
        }
        if let Some(data) = data {
            body = body.children(data.blockers.iter().enumerate().map(|(ix, blocker)| {
                text(("lifecycle-blocker", ix), blocker_words(*blocker)).text_sm()
            }));
        }
        if offers.no_record {
            body = body.child(text("lifecycle-no-record", NO_RECORD).text_sm());
        }
        let item = self.item_word();
        body = body.children(
            way_out
                .iter()
                .filter_map(|step| guidance(step, item))
                .enumerate()
                .map(|(ix, words)| {
                    text(("lifecycle-way-out", ix), words)
                        .text_sm()
                        .text_color(muted)
                }),
        );
        if offers.waiting {
            body = body.child(text("lifecycle-still-running", STILL_RUNNING).text_sm());
        }
        if matches!(ending, Ending::Failed | Ending::StillRunning) {
            body = body
                .child(
                    text("lifecycle-message", envelope.message.clone())
                        .text_sm()
                        .text_color(muted),
                )
                .child(
                    text("lifecycle-code", envelope.code.as_str())
                        .text_xs()
                        .text_color(muted),
                );
        }
        (
            body.into_any_element(),
            self.ended_buttons(envelope, &offers, cx),
        )
    }

    fn ended_buttons(
        &self,
        envelope: &Envelope<MutationDataDto>,
        offers: &Offers,
        cx: &Context<Self>,
    ) -> Vec<AnyElement> {
        let mut buttons = Vec::new();
        if offers.retry && !offers.waiting {
            let retry = self.on(cx, |dialog, _, cx| dialog.resend(cx));
            buttons.push(button("lifecycle-retry", RETRY, true, retry));
        }
        if offers.prepare {
            let again = self.on(cx, |dialog, _, cx| dialog.prepare_anew(cx));
            buttons.push(button(
                "lifecycle-prepare-again",
                PREPARE_AGAIN,
                !offers.retry,
                again,
            ));
        }
        if let Some(operation) = offers.resume.clone().filter(|_| !offers.waiting) {
            let go = self.on(cx, move |dialog, _, cx| dialog.resume_held(&operation, cx));
            buttons.push(button("lifecycle-resume", RESUME, false, go));
        }
        match offers.credential {
            Some(Credential::Locked) if !offers.waiting => {
                let unlock = self.on(cx, |dialog, _, cx| dialog.unlock(cx));
                buttons.push(button("lifecycle-unlock", UNLOCK, true, unlock));
            }
            Some(Credential::Host) => {
                let authority = host_authority(envelope);
                let go = self.on(cx, move |dialog, window, cx| {
                    if let Some(root) = dialog.root().map(|root| root.to_path_buf()) {
                        let authority = authority.clone();
                        dialog.route(window, cx, ItemRoute::CheckHost { root, authority });
                    }
                });
                buttons.push(button("lifecycle-check-host", CHECK_HOST, false, go));
            }
            _ => {}
        }
        if matches!(
            offers.credential,
            Some(Credential::Locked | Credential::Key)
        ) {
            let go = self.on(cx, |dialog, window, cx| {
                dialog.route(window, cx, ItemRoute::SharingAndKeys)
            });
            buttons.push(button(
                "lifecycle-sharing-and-keys",
                OPEN_SHARING_AND_KEYS,
                false,
                go,
            ));
        }
        if offers.inspect {
            buttons.push(self.inspect_button(cx));
        }
        if offers.open_folder
            && let Some(root) = self.root().map(|root| root.to_path_buf())
        {
            buttons.push(button(
                "lifecycle-open-folder",
                OPEN_FOLDER,
                false,
                move |_, _, cx: &mut App| cx.open_with_system(&root),
            ));
        }
        buttons
    }
}
