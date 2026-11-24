//! The lifecycle dialog once a run is sent (D5 design, A2 Running, A3
//! Outcomes), drawn, with its buttons. What it says is `outcome.rs`'s.
//!
//! - **Running.** The lead, then each step with its state, lit from L3's
//!   stages, and the stage's words as the Running step's note; and Stop at
//!   a safe point, which cancels the run's token, alone in the footer. Once
//!   pressed, "Stopping…" takes its place, a Tab stop of its own that does
//!   nothing, so Tab stays in the dialog and Escape still reaches it
//!   (R-23). Escape, or a click on the backdrop, hides the dialog; the run
//!   goes on, and the operation area's entry opens it again.
//! - **Done, Failed, Stopped.** Each step's state, with a failed step's
//!   message as its note; then the outcome box: how it ended; for a stop,
//!   that the cancellation finished and which steps completed
//!   (MH-COLLAB-006); the outcome lines; a failure's blockers, its message
//!   when no step shows it, and its code; and the way out (L1): Retry,
//!   which sends the same request ID again with no confirmation and no
//!   passphrase (`submit_with_request_id`; P7), so a protected key takes
//!   D4's unlock path; Resume; Prepare again,
//!   a new request, where the library deleted the request (P18) or its
//!   stored answer stands before main; Open repository details; Open
//!   folder for a dirty main (P19); and D4's credential recovery: Unlock…,
//!   Open Sharing & keys…, Check host…; and Repair locally… and Review
//!   changes… beside their guidance (`repair.rs`); Stop and republish… and
//!   Republish… beside theirs (`republish.rs`). The rest of L1 is guidance.
//!   Retry, Resume and Unlock… wait while the item's lifecycle is in
//!   flight, which the dialog then says (P22).
//!   Enter retries a failed or stopped run, and closes a done one.
//! - **Made ready.** Below the outcome box of a closure that reached
//!   main, `made_ready.rs`'s part, and its Open ready tickets.

use gpui_kit::{
    AnyElement, App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled,
    component::{ActiveTheme, v_flex},
};
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleMutationDto, LifecyclePublicationPendingReason, MutationDataDto,
        ProgressStage, lifecycle_way_out,
    },
    results::{Envelope, ResultCode},
};

use super::{
    LifecycleDialog, NOT_SENT, OPEN_FOLDER, PREPARE_AGAIN, RESUME, RETRY, RUNNING_LEAD,
    STILL_RUNNING,
    actions::Offers,
    button,
    confirm::{Step, git_row, steps},
    outcome::{
        CANCELLATION_FINISHED, CHECK_HOST, Credential, DISMISSED, Ending, FINISHED_BEFORE_STOP,
        NO_RECORD, OPEN_SHARING_AND_KEYS, REMOVED, Reach, STOP, STOPPING, StepState, UNLOCK,
        guidance, lifecycle_data, outcome_lines, running_states, steps_by_kind,
    },
    prepare::blocker_words,
    rows::{Mark, Note, StepRow, outcome_box, step_rows},
    text,
};
use crate::desktop::{
    model::{
        operations::{primary_name, stage_words},
        publication::host_authority,
    },
    services::requests::{DesktopRequestId, RequestState},
    view::item::ItemRoute,
};

/// What a sent run shows (`LifecycleDialog::shown`).
pub(super) enum Shown<'a> {
    /// A message only: taken off the queue, never sent, or dismissed with
    /// its answer.
    Message(&'static str),
    /// Answered with lifecycle data.
    Ended(&'a Envelope<MutationDataDto>),
    /// A resume answered with no lifecycle data.
    Resumed(ResultCode, &'a str),
    /// Sent, and no answer yet.
    Running,
}

impl LifecycleDialog {
    pub(super) fn item_word(&self) -> &'static str {
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

    /// The primary branch's name (R-35): the confirmed preview's, else the
    /// one the workspace knows; `None` when neither is.
    pub(super) fn known_primary(&self) -> Option<String> {
        self.plan
            .as_ref()
            .map(|plan| plan.primary_branch.clone())
            .or_else(|| self.heading.primary.clone())
    }

    /// `known_primary`, or `PRIMARY_UNKNOWN`'s words.
    pub(super) fn primary(&self) -> String {
        primary_name(self.known_primary().as_deref()).to_owned()
    }

    /// What a sent run shows, by where its request stands. The stage and
    /// the body both read it, so they cannot disagree (R-1).
    pub(super) fn shown<'a>(&self, id: DesktopRequestId, cx: &'a App) -> Shown<'a> {
        let operations = self.services.operations().read(cx);
        let state = operations.request(id).map(|request| &request.state);
        match (state, operations.lifecycle_answer(id)) {
            (Some(RequestState::Removed), _) => Shown::Message(REMOVED),
            (Some(RequestState::NotSent(_)), _) => Shown::Message(NOT_SENT),
            // Its answer went with its entry (review O-1).
            (Some(RequestState::Dismissed), None) => Shown::Message(DISMISSED),
            (_, Some(envelope)) => Shown::Ended(envelope),
            // A resume answered with no lifecycle data (P22).
            (Some(RequestState::Finished(_, code, _, _, message)), None) => {
                Shown::Resumed(*code, message)
            }
            _ => Shown::Running,
        }
    }

    /// The body and buttons of a sent run, by where its request stands;
    /// each button with whether it is primary.
    pub(super) fn submitted_body(
        &self,
        id: DesktopRequestId,
        cx: &Context<Self>,
    ) -> (AnyElement, Vec<(bool, AnyElement)>) {
        match self.shown(id, cx) {
            Shown::Message(words) => (self.message(words), Vec::new()),
            Shown::Ended(envelope) => self.ended(id, envelope, cx),
            Shown::Resumed(code, message) => self.resume_answered(code, message, cx),
            Shown::Running => self.running(id, cx),
        }
    }

    fn message(&self, words: &'static str) -> AnyElement {
        text("lifecycle-message", words)
            .text_sm()
            .into_any_element()
    }

    /// The stage the running request `id` last reported.
    fn stage_at(&self, id: DesktopRequestId, cx: &App) -> Option<ProgressStage> {
        match self
            .services
            .operations()
            .read(cx)
            .request(id)
            .map(|request| &request.state)
        {
            Some(RequestState::Running { stage, .. }) => *stage,
            _ => None,
        }
    }

    fn running(
        &self,
        id: DesktopRequestId,
        cx: &Context<Self>,
    ) -> (AnyElement, Vec<(bool, AnyElement)>) {
        let steps = self.shown_steps(None);
        let keys: Vec<_> = steps.iter().map(|step| step.key).collect();
        let states = running_states(&keys, self.reached);
        // The Running step says which stage the run is at, with or without
        // the preview; the other steps keep the preview's notes (A3).
        let stage = self.stage_at(id, cx);
        let primary = self.known_primary();
        let rows = steps
            .iter()
            .zip(&states)
            .map(|(step, state)| StepRow {
                title: &step.title,
                mark: Mark::State(*state),
                note: Note::Words(match (state, stage) {
                    (StepState::Running, Some(stage)) => stage_words(stage, primary.as_deref()),
                    _ => step.note.clone(),
                }),
            })
            .collect();
        let body = v_flex()
            .gap_2()
            .child(text("lifecycle-state", RUNNING_LEAD).text_sm())
            .child(step_rows(rows, cx))
            .children(self.plan.as_deref().map(|plan| git_row(plan, cx)))
            .into_any_element();
        let stop = if self.stop_pressed {
            // A Tab stop that does nothing: with none, Tab would leave the
            // dialog, and Escape, bound inside it, would not reach it (R-23).
            let stopping = text("lifecycle-stopping", STOPPING)
                .track_focus(&self.stopping)
                .text_sm()
                .into_any_element();
            (false, stopping)
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
    ) -> (AnyElement, Vec<(bool, AnyElement)>) {
        let muted = cx.theme().muted_foreground;
        let ending = Ending::of(envelope);
        let data = lifecycle_data(envelope);
        let reach = Reach::of(envelope, self.reached);
        let way_out = lifecycle_way_out(envelope);
        let offers = self.offers(id, envelope, ending, &reach, &way_out, cx);
        let steps = self.shown_steps(data);
        let keys: Vec<_> = steps.iter().map(|step| step.key).collect();
        let local = data.is_some_and(|data| {
            data.publication_pending == Some(LifecyclePublicationPendingReason::NoPublicationRemote)
        });
        let states = match ending {
            Ending::StillRunning => running_states(&keys, self.reached),
            _ => reach.states(&keys, ending, local),
        };
        // The failed step shows the message in place of its own note; with
        // none (Waiting, or every step done) the box shows it (A3, A6).
        let failed = states.iter().position(|state| *state == StepState::Failed);
        let rows = steps
            .iter()
            .zip(&states)
            .enumerate()
            .map(|(ix, (step, state))| StepRow {
                title: &step.title,
                mark: Mark::State(*state),
                note: match failed {
                    Some(failed) if failed == ix => Note::Failure(envelope.message.clone()),
                    _ => Note::Words(step.note.clone()),
                },
            })
            .collect();
        let mut lines: Vec<AnyElement> = vec![
            text("lifecycle-ending", ending.words())
                .font_weight(FontWeight::SEMIBOLD)
                .into_any_element(),
        ];
        if ending == Ending::Stopped {
            let completed: Vec<&str> = steps
                .iter()
                .zip(&states)
                .filter(|(_, state)| **state == StepState::Done)
                .map(|(step, _)| step.title.as_str())
                .collect();
            lines.push(
                text("lifecycle-cancellation", CANCELLATION_FINISHED)
                    .text_sm()
                    .into_any_element(),
            );
            lines.push(
                text(
                    "lifecycle-completed",
                    format!("Completed: {}.", completed.join(", ")),
                )
                .text_sm()
                .into_any_element(),
            );
        }
        if ending == Ending::Done && self.stop_pressed {
            lines.push(
                text("lifecycle-stop-late", FINISHED_BEFORE_STOP)
                    .text_sm()
                    .into_any_element(),
            );
        }
        let mut made_ready = None;
        let mut open_ready = None;
        if ending != Ending::StillRunning {
            lines.extend(
                outcome_lines(envelope, &reach, &self.primary())
                    .into_iter()
                    .enumerate()
                    .map(|(ix, line)| {
                        text(("lifecycle-outcome", ix), line)
                            .text_sm()
                            .into_any_element()
                    }),
            );
            // A closure that reached main: the tickets it made ready (R-20).
            let (part, button) = self.made_ready_part(envelope, cx);
            made_ready = part;
            open_ready = button;
        }
        let (primary, known) = (self.primary(), self.known_primary());
        if let Some(data) = data {
            lines.extend(data.blockers.iter().enumerate().map(|(ix, blocker)| {
                text(
                    ("lifecycle-blocker", ix),
                    blocker_words(*blocker, known.as_deref()),
                )
                .text_sm()
                .into_any_element()
            }));
        }
        if offers.no_record {
            lines.push(
                text("lifecycle-no-record", NO_RECORD)
                    .text_sm()
                    .into_any_element(),
            );
        }
        lines.extend(self.republish_line(id, envelope, cx));
        let item = self.item_word();
        lines.extend(
            way_out
                .iter()
                .filter_map(|step| guidance(step, item, &primary))
                .enumerate()
                .map(|(ix, words)| {
                    text(("lifecycle-way-out", ix), words)
                        .text_sm()
                        .text_color(muted)
                        .into_any_element()
                }),
        );
        if offers.waiting {
            lines.push(
                text("lifecycle-still-running", STILL_RUNNING)
                    .text_sm()
                    .into_any_element(),
            );
        }
        if matches!(ending, Ending::Failed | Ending::StillRunning) {
            if failed.is_none() {
                lines.push(
                    text("lifecycle-message", envelope.message.clone())
                        .text_sm()
                        .text_color(muted)
                        .into_any_element(),
                );
            }
            lines.push(
                text("lifecycle-code", envelope.code.as_str())
                    .text_xs()
                    .text_color(muted)
                    .into_any_element(),
            );
        }
        // The preview this run was confirmed with names its branch (A6).
        let body = v_flex()
            .gap_2()
            .child(step_rows(rows, cx))
            .child(outcome_box(ending, lines, cx))
            .children(made_ready)
            .children(self.plan.as_deref().map(|plan| git_row(plan, cx)));
        let mut buttons = self.republish_buttons(id, envelope, &way_out, offers.waiting, cx);
        buttons.extend(self.ended_buttons(envelope, &offers, cx));
        buttons.extend(open_ready);
        (body.into_any_element(), buttons)
    }

    fn ended_buttons(
        &self,
        envelope: &Envelope<MutationDataDto>,
        offers: &Offers,
        cx: &Context<Self>,
    ) -> Vec<(bool, AnyElement)> {
        let mut buttons = self.repair_buttons(&lifecycle_way_out(envelope), offers.waiting, cx);
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
