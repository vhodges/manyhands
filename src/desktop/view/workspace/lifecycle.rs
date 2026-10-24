//! Quit and passphrase orchestration; the existing views retain their UI.

use super::{QuitPhase, Region, Workspace};
use crate::desktop::{
    services::{prompts::PromptReply, worker::QuitState},
    view::{
        operations::draft_flush_id,
        passphrase::open_dialog,
        quit::{DraftsNotSaved, NotSavedChoice},
    },
};
use gpui_kit::{Context, WeakEntity, Window, component::WindowExt};
use manyhands::drafts::{DraftErrorKind, DraftId};

impl Workspace {
    /// The action and window-close request use the same handler. Draft
    /// sessions are flushed first; then D1's quit waits for a job.
    pub(in crate::desktop::view) fn request_quit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.prompt_unavailable(window, cx);
        match self.quit {
            QuitPhase::Quit => return true,
            QuitPhase::SavingDrafts | QuitPhase::Finishing(_) => return false,
            QuitPhase::Running => {}
        }
        if self.services.drafts().read(cx).unsaved() {
            self.save_drafts(window, cx);
            return false;
        }
        self.finish_job(window, cx)
    }

    /// D1's quit: at once with no job running, else "Finishing".
    fn finish_job(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match self.services.worker().quit() {
            QuitState::Idle => {
                self.quit_now(cx);
                true
            }
            QuitState::Finishing(id) => {
                self.wait_to_quit(QuitPhase::Finishing(id), window, cx);
                false
            }
        }
    }

    /// Quitting waits: every dialog closes and Quit now takes focus.
    fn wait_to_quit(&mut self, phase: QuitPhase, window: &mut Window, cx: &mut Context<Self>) {
        self.quit = phase;
        window.close_all_dialogs(cx);
        window.focus(&self.quit_focus, cx);
        self.last_region = Some(Region::Operations);
        cx.notify();
    }

    /// Flushes every draft session before GPUI may quit, under "Saving
    /// drafts for recovery" and Quit now.
    fn save_drafts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.wait_to_quit(QuitPhase::SavingDrafts, window, cx);
        let flush = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.flush_all(cx));
        cx.spawn_in(window, async move |workspace, cx| {
            let failed = flush.await;
            let _ = workspace.update_in(cx, |workspace, window, cx| {
                workspace.drafts_flushed(failed, window, cx)
            });
        })
        .detach();
    }

    /// Every flush answered. Quit now may already have quit.
    fn drafts_flushed(
        &mut self,
        failed: Vec<(DraftId, DraftErrorKind)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.quit != QuitPhase::SavingDrafts {
            return;
        }
        self.quit = QuitPhase::Running;
        cx.notify();
        if failed.is_empty() {
            self.prompt_unavailable(window, cx);
            self.finish_job(window, cx);
            return;
        }
        let failed: Vec<DraftId> = failed.into_iter().map(|(id, _)| id).collect();
        let workspace = cx.entity().downgrade();
        window.close_all_dialogs(cx);
        DraftsNotSaved {
            count: failed.len(),
        }
        .open(window, cx, move |choice, window, cx| {
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.not_saved_chosen(choice, &failed, window, cx)
            });
        });
    }

    /// Quit anyway goes on to D1's quit. Cancel stays, with the first
    /// failed draft's Retry focused, in the operation area's order.
    fn not_saved_chosen(
        &mut self,
        choice: NotSavedChoice,
        failed: &[DraftId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.quit != QuitPhase::Running {
            return;
        }
        match choice {
            NotSavedChoice::QuitAnyway => {
                self.prompt_unavailable(window, cx);
                self.finish_job(window, cx);
            }
            NotSavedChoice::Cancel => {
                let failures = self.services.drafts().read(cx).flush_failures();
                let first = failures
                    .iter()
                    .map(|(id, ..)| *id)
                    .find(|id| failed.contains(id))
                    .or_else(|| failures.first().map(|(id, ..)| *id));
                if let Some(id) = first {
                    let retry = self.control_focus_for(draft_flush_id(id, "retry"), cx);
                    window.focus(&retry, cx);
                    self.last_region = Some(Region::Operations);
                }
                cx.notify();
            }
        }
    }

    pub(in crate::desktop::view) fn quit_now(&mut self, cx: &mut Context<Self>) {
        self.quit = QuitPhase::Quit;
        self.prompt_unavailable_now(cx);
        cx.notify();
        cx.quit();
    }

    pub(super) fn quit_if_finished(&mut self, cx: &mut Context<Self>) {
        if matches!(self.quit, QuitPhase::Finishing(_))
            && self.services.worker().quit() == QuitState::Idle
        {
            self.quit_now(cx);
        }
    }

    /// A single worker asks one prompt at a time. Once quitting has started,
    /// answer Unavailable rather than opening another dialog.
    pub(super) fn show_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open = self
            .passphrase
            .as_ref()
            .and_then(WeakEntity::upgrade)
            .is_some_and(|dialog| dialog.read(cx).waiting());
        if open {
            return;
        }
        let Some(prompt) = self
            .services
            .operations()
            .update(cx, |operations, _| operations.take_prompt())
        else {
            return;
        };
        if self.quit != QuitPhase::Running {
            prompt.answer(PromptReply::Unavailable);
            return;
        }
        self.passphrase = Some(open_dialog(prompt, window, cx));
        cx.notify();
    }

    fn prompt_unavailable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dialog) = self.passphrase.take().and_then(|dialog| dialog.upgrade()) {
            dialog.update(cx, |dialog, cx| dialog.unavailable(window, cx));
        }
        self.prompt_waiting_unavailable(cx);
    }

    fn prompt_unavailable_now(&mut self, cx: &mut Context<Self>) {
        if let Some(dialog) = self.passphrase.take().and_then(|dialog| dialog.upgrade()) {
            dialog.update(cx, |dialog, cx| dialog.unavailable_now(cx));
        }
        self.prompt_waiting_unavailable(cx);
    }

    fn prompt_waiting_unavailable(&mut self, cx: &mut Context<Self>) {
        let waiting = self
            .services
            .operations()
            .update(cx, |operations, _| operations.take_prompt());
        if let Some(prompt) = waiting {
            prompt.answer(PromptReply::Unavailable);
        }
    }
}
