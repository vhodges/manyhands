//! Quit and passphrase orchestration; the existing views retain their UI.

use super::{QuitPhase, Region, Workspace};
use crate::desktop::{
    services::{prompts::PromptReply, worker::QuitState},
    view::passphrase::open_dialog,
};
use gpui_kit::{Context, WeakEntity, Window, component::WindowExt};

impl Workspace {
    /// The action and window-close request use the same handler.
    pub(in crate::desktop::view) fn request_quit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.prompt_unavailable(window, cx);
        match self.quit {
            QuitPhase::Quit => return true,
            QuitPhase::Finishing(_) => return false,
            QuitPhase::Running => {}
        }
        match self.services.worker().quit() {
            QuitState::Idle => {
                self.quit_now(cx);
                true
            }
            QuitState::Finishing(id) => {
                self.quit = QuitPhase::Finishing(id);
                window.close_all_dialogs(cx);
                window.focus(&self.quit_focus, cx);
                self.last_region = Some(Region::Operations);
                cx.notify();
                false
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
