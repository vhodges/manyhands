//! Quit and passphrase orchestration; the existing views retain their UI.

use super::{QuitPhase, Region, Workspace};
use crate::desktop::{
    model::{activity::rows::RowKey, layout::PanelKind, stamp::TabId},
    services::{prompts::PromptReply, requests::DesktopRequestId, worker::QuitState},
    view::{
        passphrase::open_dialog,
        quit::{DraftsNotSaved, EditsNotKept, NotSavedChoice},
    },
};
use futures::future::join_all;
use gpui_kit::{Context, WeakEntity, Window, component::WindowExt};
use manyhands::drafts::{DraftErrorKind, DraftId};

impl Workspace {
    /// The action and window-close request use the same handler. An edit
    /// session's text that no draft holds is asked about first
    /// (`unkept_asks_first`); then draft
    /// sessions are flushed; then D1's quit waits for a job. Prompts
    /// are answered `Unavailable` only once quitting is committed to D1's
    /// phases: while drafts are saved, and while their dialog asks, a
    /// prompt is held, neither shown nor answered, so that Cancel can
    /// show it again.
    pub(in crate::desktop::view) fn request_quit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match self.quit {
            QuitPhase::Quit => {
                self.prompt_unavailable(window, cx);
                return true;
            }
            QuitPhase::Finishing(_) => {
                self.prompt_unavailable(window, cx);
                return false;
            }
            QuitPhase::SavingDrafts => return false,
            QuitPhase::Running => {}
        }
        // Nothing new starts from here: the unkept ask below opens while
        // `QuitPhase` is still `Running`, and Cancel on it lets go (R-33).
        self.freeze_polling(true, cx);
        // Part C fix 1, I1: an edit session's text that no draft holds is
        // asked about first, once every begin in flight has landed.
        if self.unkept_asks_first(window, cx) {
            return false;
        }
        self.quit_with_drafts(window, cx)
    }

    /// D2's quit: every draft session is flushed first, then D1's quit.
    fn quit_with_drafts(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.services.drafts().read(cx).unsaved() {
            self.save_drafts(window, cx);
            return false;
        }
        self.finish_job(window, cx)
    }

    /// Part C fix 1, I1 (the Cycle's contract 6). A begin still in flight
    /// is waited for, under "Saving drafts for recovery" and Quit now, as
    /// D2's flush is: it stores a draft. Then, when tabs hold unsaved edit
    /// text that no draft holds (crash recovery off, or a begin that
    /// failed), the dialog asks, holding a prompt as D2's does: Cancel, the
    /// default, stays and shows the first such tab; Quit anyway goes on to
    /// D2's quit, whose own question follows when a flush fails. False
    /// when neither applies: quitting goes on.
    fn unkept_asks_first(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let begins = self.content.tabs.begins_in_flight(cx);
        if !begins.is_empty() {
            self.hold_prompt(window, cx);
            self.wait_to_quit(QuitPhase::SavingDrafts, window, cx);
            cx.spawn_in(window, async move |workspace, cx| {
                join_all(begins).await;
                let _ = workspace.update_in(cx, |workspace, window, cx| {
                    workspace.begins_landed(window, cx)
                });
            })
            .detach();
            return true;
        }
        let unkept = self.content.tabs.unkept(cx);
        if unkept.is_empty() {
            return false;
        }
        self.hold_prompt(window, cx);
        window.close_all_dialogs(cx);
        self.close_settings_quietly(window, cx);
        let workspace = cx.entity().downgrade();
        EditsNotKept {
            count: unkept.len(),
        }
        .open(window, cx, move |choice, window, cx| {
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.unkept_chosen(choice, &unkept, window, cx)
            });
        });
        true
    }

    /// Every begin quitting waited for has landed: quitting is asked for
    /// again, unless Quit now quit meanwhile.
    fn begins_landed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.quit != QuitPhase::SavingDrafts {
            return;
        }
        self.set_quit(QuitPhase::Running, cx);
        cx.notify();
        self.request_quit(window, cx);
    }

    /// Quit anyway goes on to D2's quit. Cancel stays, with the first such
    /// tab shown and the content region focused; a prompt held meanwhile
    /// shows once the dialog has closed.
    fn unkept_chosen(
        &mut self,
        choice: NotSavedChoice,
        unkept: &[TabId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.quit != QuitPhase::Running {
            return;
        }
        match choice {
            NotSavedChoice::QuitAnyway => {
                self.quit_with_drafts(window, cx);
            }
            NotSavedChoice::Cancel => {
                self.prompts_held = false;
                self.freeze_polling(false, cx);
                if let Some(&id) = unkept.first() {
                    self.show_tab(id, cx);
                    self.focus_region(Region::Content, window, cx);
                }
                cx.defer_in(window, |this, window, cx| this.show_prompt(window, cx));
                cx.notify();
            }
        }
    }

    /// Sets the phase; the request model knows when drafts are saving, so
    /// no confirmation opens in front of Quit now.
    fn set_quit(&mut self, phase: QuitPhase, cx: &mut Context<Self>) {
        self.quit = phase;
        let saving = phase == QuitPhase::SavingDrafts;
        self.services
            .operations()
            .update(cx, |operations, _| operations.set_saving_drafts(saving));
    }

    /// D1's quit, committed: prompts are answered `Unavailable`, then it
    /// quits at once with no job running, else shows "Finishing".
    fn finish_job(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.prompts_held = false;
        self.prompt_unavailable(window, cx);
        match self.services.worker().quit() {
            QuitState::Idle => {
                self.quit_gracefully(cx);
                true
            }
            QuitState::Finishing(id) => {
                self.wait_to_quit(QuitPhase::Finishing(id), window, cx);
                self.start_still_stopping(id, window, cx);
                false
            }
        }
    }

    /// A graceful exit: the worker thread is joined before the app quits
    /// (A9, O-7). Quit now goes to `quit_now` directly and joins nothing.
    fn quit_gracefully(&mut self, cx: &mut Context<Self>) {
        self.services.worker().shut_down_and_join();
        self.quit_now(cx);
    }

    /// The polling scheduler starts nothing while quitting is asked and not
    /// yet cancelled, through the unkept ask, the flush and the wait for a
    /// job (R-33).
    fn freeze_polling(&mut self, frozen: bool, cx: &mut Context<Self>) {
        self.services
            .scheduler()
            .update(cx, |scheduler, cx| scheduler.set_quitting(frozen, cx));
    }

    /// Waiting for the job `id`: after the injected ten seconds the words
    /// say so (decision 7). The UI stays interactive and nothing is cleared
    /// or reclaimed.
    fn start_still_stopping(
        &mut self,
        id: DesktopRequestId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let operations = self.services.operations().clone();
        let after = operations.update(cx, |operations, _| {
            operations.set_still_stopping(false);
            operations.still_stopping_after()
        });
        cx.spawn_in(window, async move |workspace, cx| {
            cx.background_executor().timer(after).await;
            let _ = workspace.update(cx, |workspace, cx| workspace.still_stopping(id, cx));
        })
        .detach();
    }

    /// Ten seconds have passed while quitting waited for `id`. Quit now may
    /// have quit meanwhile.
    fn still_stopping(&mut self, id: DesktopRequestId, cx: &mut Context<Self>) {
        if self.quit != QuitPhase::Finishing(id) {
            return;
        }
        self.services
            .operations()
            .update(cx, |operations, _| operations.set_still_stopping(true));
        cx.notify();
    }

    /// Quitting waits: every dialog, Settings (quietly, R-52) and the
    /// details sheet close, no sheet a chip asked for opens over it, and
    /// Quit now takes focus. Closing a sheet gives focus back to what held
    /// it as the sheet opened, so Quit now is focused after it. Quit now
    /// lives in Activity: with Activity not displayed focus stays where the
    /// closing left it, and the layout is untouched (R-20).
    fn wait_to_quit(&mut self, phase: QuitPhase, window: &mut Window, cx: &mut Context<Self>) {
        self.set_quit(phase, cx);
        window.close_all_dialogs(cx);
        self.content.details = None;
        self.close_settings_quietly(window, cx);
        self.content.sheet = None;
        if window.has_active_sheet(cx) {
            window.close_sheet(cx);
        }
        if self.layout.is_displayed(PanelKind::Activity) {
            window.focus(&self.quit_focus, cx);
            self.last_region = Some(Region::Operations);
        }
        cx.notify();
    }

    /// Flushes every draft session before GPUI may quit, under "Saving
    /// drafts for recovery" and Quit now.
    fn save_drafts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hold_prompt(window, cx);
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
        self.set_quit(QuitPhase::Running, cx);
        cx.notify();
        if failed.is_empty() {
            self.finish_job(window, cx);
            return;
        }
        let failed: Vec<DraftId> = failed.into_iter().map(|(id, _)| id).collect();
        let workspace = cx.entity().downgrade();
        window.close_all_dialogs(cx);
        self.close_settings_quietly(window, cx);
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
    /// failed draft's Retry focused, in the operation area's order. Retry
    /// lives in Activity: with Activity not displayed, focus goes to the
    /// item area, and Needs attention counts the failure (R-20, P27).
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
                self.finish_job(window, cx);
            }
            NotSavedChoice::Cancel => {
                self.prompts_held = false;
                self.freeze_polling(false, cx);
                let failures = self.services.drafts().read(cx).flush_failures();
                let first = failures
                    .iter()
                    .map(|(id, ..)| *id)
                    .find(|id| failed.contains(id))
                    .or_else(|| failures.first().map(|(id, ..)| *id));
                if !self.layout.is_displayed(PanelKind::Activity) {
                    self.focus_region(Region::Content, window, cx);
                } else if let Some(id) = first {
                    // The failed draft's Flush row is highlighted, and the
                    // list focused (C3).
                    self.activity.pane.highlight_when_shown(&RowKey::Flush(id));
                    self.focus_region(Region::Operations, window, cx);
                }
                // A prompt held meanwhile shows once this dialog has
                // closed (the Kit closes the top one); closing it gives the
                // focus back to the Retry.
                cx.defer_in(window, |this, window, cx| this.show_prompt(window, cx));
                cx.notify();
            }
        }
    }

    /// Every exit passes through here: the layout is written first, on this
    /// thread, so a change made while the quit waited is kept (R-19, P31).
    pub(in crate::desktop::view) fn quit_now(&mut self, cx: &mut Context<Self>) {
        self.dock.write_for_quit(&self.layout, cx);
        self.set_quit(QuitPhase::Quit, cx);
        self.prompts_held = false;
        self.prompt_unavailable_now(cx);
        cx.notify();
        cx.quit();
    }

    pub(super) fn quit_if_finished(&mut self, cx: &mut Context<Self>) {
        if matches!(self.quit, QuitPhase::Finishing(_))
            && self.services.worker().quit() == QuitState::Idle
        {
            self.quit_gracefully(cx);
        }
    }

    /// A single worker asks one prompt at a time. While drafts are saved
    /// for quitting, or their dialog asks, it is held where it waits. Once
    /// D1's quitting has started, answer Unavailable rather than opening
    /// another dialog.
    pub(super) fn show_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.prompts_held {
            return;
        }
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

    /// Drafts are about to be saved: an open prompt's dialog gives its
    /// prompt back unanswered, to wait with the request model until
    /// quitting proceeds (answered `Unavailable`) or is cancelled (shown
    /// again). Its dialog closes with the others.
    fn hold_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prompts_held = true;
        let Some(dialog) = self.passphrase.take().and_then(|dialog| dialog.upgrade()) else {
            return;
        };
        if let Some(prompt) = dialog.update(cx, |dialog, cx| dialog.withdraw(window, cx)) {
            self.services
                .operations()
                .update(cx, |operations, _| operations.hold_prompt(prompt));
        }
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
