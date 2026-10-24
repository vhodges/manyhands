//! The passphrase dialog: what a credential prompt shows (design,
//! Credential prompts).
//!
//! The input is masked, with no mask toggle: the Kit's masked input
//! refuses copy and hides its value from accessibility. Submit, by Enter
//! or Unlock, refuses an empty passphrase or one containing NUL, as
//! `SecretPassphrase::new` does; otherwise it moves the text into the one
//! `SecretPassphrase`, empties the input, lets the input go and replies
//! `Supplied`. Escape or Cancel replies `Cancelled`. Quitting, or closing
//! the window, replies `Unavailable`; a dialog dropped unanswered, with
//! its window, reads as `Unavailable` too.
//!
//! A prompt that arrives while the confirmation dialog is open shows
//! above it, so the job that asks, which that dialog may have submitted,
//! is never stuck behind it. The prompt is answered only from its own
//! dialog: each control acts on this dialog's prompt, which is answered
//! once, and the dialog closes itself only when it answered. Closing
//! gives focus back to where it was when the prompt opened, which for a
//! job started from the confirmation dialog is that dialog, and through it
//! the control that opened it.
//!
//! Text is fixed strings, the key's label and ID, and the library's
//! guidance for the reason.

use gpui_kit::{
    App, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, TestSupportExt,
    WeakEntity, Window,
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div,
};
use manyhands::repository::keys::SecretPassphrase;

use crate::desktop::services::prompts::{PromptReply, PromptRequest};

pub(crate) const TITLE: &str = "Unlock key";
pub(crate) const UNLOCK: &str = "Unlock";
pub(crate) const CANCEL: &str = "Cancel";
pub(crate) const REFUSED: &str =
    "Enter the passphrase. It cannot be empty or contain a NUL character.";

/// The input's label: "Passphrase for <label>".
pub(crate) fn label_for(label: &str) -> String {
    format!("Passphrase for {label}")
}

/// Shows `prompt` in a dialog above whatever is open, with focus in its
/// input. Gives the dialog, which the workspace holds only weakly: the
/// dialog layer owns it.
pub(crate) fn open_dialog(
    prompt: PromptRequest,
    window: &mut Window,
    cx: &mut App,
) -> WeakEntity<PassphraseDialog> {
    let origin = window.focused(cx);
    let dialog = cx.new(|cx| PassphraseDialog::new(prompt, origin, window, cx));
    let weak = dialog.downgrade();
    let input = dialog.read(cx).input.clone();
    window.open_dialog(cx, move |base, _, _| {
        let submitting = dialog.clone();
        let cancelling = dialog.clone();
        base.title(TITLE)
            .close_button(false)
            .overlay_closable(false)
            .child(dialog.clone())
            // Enter: the dialog closes itself once it has answered, so the
            // Kit never closes it here.
            .on_ok(move |_, window, cx| {
                submitting.update(cx, |dialog, cx| dialog.submit(window, cx));
                false
            })
            // Escape: the Kit closes the dialog; focus goes back with it.
            .on_cancel(move |_, window, cx| {
                cancelling.update(cx, |dialog, cx| {
                    dialog.answer(PromptReply::Cancelled, window, cx);
                });
                true
            })
    });
    if let Some(input) = input {
        input.update(cx, |input, cx| input.focus(window, cx));
    }
    weak
}

pub(crate) struct PassphraseDialog {
    /// Answered once: taken by the first answer.
    prompt: Option<PromptRequest>,
    label: String,
    key: String,
    guidance: &'static str,
    /// Let go once the dialog has answered.
    input: Option<Entity<InputState>>,
    /// The last submit was refused.
    refused: bool,
    origin: Option<FocusHandle>,
    _enter: Option<Subscription>,
}

impl PassphraseDialog {
    fn new(
        prompt: PromptRequest,
        origin: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).masked(true));
        let enter = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.submit(window, cx);
            }
        });
        Self {
            label: label_for(&prompt.label),
            key: prompt.key_id.to_string(),
            guidance: prompt.reason.guidance(),
            prompt: Some(prompt),
            input: Some(input),
            refused: false,
            origin,
            _enter: Some(enter),
        }
    }

    /// Whether its prompt is still unanswered.
    pub(super) fn waiting(&self) -> bool {
        self.prompt.is_some()
    }

    /// The input, while the dialog holds it.
    #[cfg(test)]
    pub(crate) fn input(&self) -> Option<Entity<InputState>> {
        self.input.clone()
    }

    /// Submit: an acceptable passphrase becomes the one `SecretPassphrase`
    /// and is supplied; any other is refused, and nothing is replied.
    pub(crate) fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = &self.input else {
            return;
        };
        let typed = input.read(cx).text().to_string();
        let Ok(passphrase) = SecretPassphrase::new(typed) else {
            self.refused = true;
            cx.notify();
            return;
        };
        self.answer(PromptReply::Supplied(passphrase), window, cx);
        self.close(window, cx);
    }

    /// Cancel: replies `Cancelled` and closes.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.answer(PromptReply::Cancelled, window, cx) {
            self.close(window, cx);
        }
    }

    /// Quitting or closing the window: replies `Unavailable` and closes.
    pub(crate) fn unavailable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.answer(PromptReply::Unavailable, window, cx) {
            self.close(window, cx);
        }
    }

    /// Quitting at once: replies `Unavailable`, with no window to close it
    /// in. The app is ending.
    pub(crate) fn unavailable_now(&mut self, cx: &mut Context<Self>) {
        if let Some(prompt) = self.prompt.take() {
            prompt.answer(PromptReply::Unavailable);
        }
        self.input = None;
        self._enter = None;
        cx.notify();
    }

    /// Quitting saves drafts first: empties the input, lets it go and
    /// gives the prompt back unanswered, to be held while the drafts are
    /// saved. The caller closes the dialog.
    pub(crate) fn withdraw(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PromptRequest> {
        self._enter = None;
        if let Some(input) = self.input.take() {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
        self.prompt.take()
    }

    /// Empties the input and lets it go, then replies, once. False when
    /// the prompt was already answered.
    fn answer(&mut self, reply: PromptReply, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self._enter = None;
        if let Some(input) = self.input.take() {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
        match self.prompt.take() {
            Some(prompt) => {
                prompt.answer(reply);
                true
            }
            None => false,
        }
    }

    /// Closes this dialog, the top one, and gives focus back.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        if let Some(origin) = &self.origin {
            window.focus(origin, cx);
        }
    }
}

fn text(id: &'static str, value: impl Into<String>) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

/// A dialog button. Enter on it presses it: its wrapper takes the dialog's
/// Enter (`Confirm`, dispatched on key down) before the dialog, whose
/// `on_ok` would submit. Enter in the input, or with no button focused,
/// still submits.
fn button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_click = std::rc::Rc::new(on_click);
    let on_enter = on_click.clone();
    let button = Button::new((id, 0usize))
        .label(label)
        .on_click(move |event, window, cx| on_click(event, window, cx));
    let button = if primary { button.primary() } else { button };
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .on_action(move |_: &Confirm, window, cx| on_enter(&ClickEvent::default(), window, cx))
        .child(button)
}

impl Render for PassphraseDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let unlock = {
            let entity = cx.entity().downgrade();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |dialog, cx| dialog.submit(window, cx));
            }
        };
        let cancel = {
            let entity = cx.entity().downgrade();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |dialog, cx| dialog.cancel(window, cx));
            }
        };
        v_flex()
            .id("passphrase-dialog")
            .test_support()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .child(text("passphrase-label", self.label.clone())),
            )
            .child(
                div()
                    .text_sm()
                    .child(text("passphrase-reason", self.guidance)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(text("passphrase-key", format!("Key {}", self.key))),
            )
            .children(self.input.as_ref().map(|input| {
                div()
                    .id("passphrase-input")
                    .test_support()
                    .aria_label(self.label.clone())
                    .child(Input::new(input).aria_label(self.label.clone()))
            }))
            .children(
                self.refused
                    .then(|| div().text_sm().child(text("passphrase-refused", REFUSED))),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(button("passphrase-ok", UNLOCK, true, unlock))
                    .child(button("passphrase-cancel", CANCEL, false, cancel)),
            )
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "passphrase_tests.rs"]
mod tests;
