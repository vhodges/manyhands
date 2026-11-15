//! The Unlock key dialog: what a credential prompt shows (design,
//! Credential prompts), on the common frame (Wave 04 Cycle 05a design Part
//! D; `05`, 2h), 440 px wide.
//!
//! The header has the `lock` tile and "Unlock your key"; its description
//! names what waits, while a share or a post waits on the prompt. The body
//! is the key card (`card`: the key's label and, once the key list has it,
//! its fingerprint), the reason's guidance, the "Passphrase" field, the
//! refusal under it, and the note. The footer is Not now, then Unlock: the
//! action last (R-1, decision 15).
//!
//! The input is masked, with no mask toggle: the Kit's masked input
//! refuses copy and hides its value from accessibility. Submit, by Enter
//! or Unlock, refuses an empty passphrase or one containing NUL, as
//! `SecretPassphrase::new` does, and puts focus back in the field (R-17);
//! otherwise it moves the text into the one `SecretPassphrase`, empties the
//! input, lets the input go and replies `Supplied`. Escape or Not now
//! replies `Cancelled`. Quitting, or closing the window, replies
//! `Unavailable`; a dialog dropped unanswered, with its window, reads as
//! `Unavailable` too.
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
//! While a share or a post waits on the prompt, the dialog names its
//! repository ("Sharing {repository} is waiting for your passphrase.").
//! One worker runs one job at a time, so what waits is the request the
//! model has running, read once the dialog has opened.
//!
//! Text is fixed strings, the key's label and fingerprint (public facts,
//! which Settings' key list shows too), the repository's directory name,
//! and the library's guidance for the reason. The key's ID and its files
//! are never drawn.

mod card;

use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, Entity, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled,
    Subscription, Task, TestSupportExt, WeakEntity, Window,
    assets::IconName,
    component::{
        Icon, Root, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div, px,
};
use manyhands::repository::keys::SecretPassphrase;

use super::{
    frame::{self, Footer, Header, Tone},
    operations::scope_name,
    shell::tokens::chrome_colors,
    workspace::Workspace,
};
use crate::desktop::{
    model::{operations::Progressing, stamp::Generations},
    services::{
        Services,
        prompts::{PromptReply, PromptRequest},
    },
};

pub(crate) const TITLE: &str = "Unlock your key";
pub(crate) const UNLOCK: &str = "Unlock";
/// Cancel, relabelled (design, The passphrase dialog).
pub(crate) const CANCEL: &str = "Not now";
/// The field's label: the key card names the key (decision 14).
pub(crate) const PASSPHRASE: &str = "Passphrase";
/// What the passphrase becomes and what Not now leaves, always shown
/// (decision 14).
pub(crate) const NOT_NOW_NOTE: &str = "Kept in memory only, never written to disk. If you \
     choose Not now, you can keep working and unlock later from Needs attention.";
pub(crate) const REFUSED: &str =
    "Enter the passphrase. It cannot be empty or contain a NUL character.";

/// The subtitle while a share of `repository` waits.
pub(crate) fn waiting_subtitle(repository: &str) -> String {
    format!("Sharing {repository} is waiting for your passphrase.")
}

/// The dialog's width (`05`, 2h).
const WIDTH: f32 = 440.;
/// The header's title and description, by the ids tests find them by.
const TITLE_ID: &str = "passphrase-title";
const SUBTITLE_ID: &str = "passphrase-subtitle";

/// Shows `prompt` in a dialog above whatever is open, with focus in its
/// input, and reads the key list for its key's fingerprint through
/// `services`. Gives the dialog, which the workspace holds only weakly: the
/// dialog layer owns it.
pub(crate) fn open_dialog(
    prompt: PromptRequest,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> WeakEntity<PassphraseDialog> {
    let origin = window.focused(cx);
    let dialog = cx.new(|cx| PassphraseDialog::new(prompt, origin, services, window, cx));
    let weak = dialog.downgrade();
    let input = dialog.read(cx).input.clone();
    window.open_dialog(cx, move |base, _, cx| {
        let submitting = dialog.clone();
        let cancelling = dialog.clone();
        let header = header(dialog.read(cx).waiting.as_deref());
        frame::dialog(base.w(px(WIDTH)), header, cx)
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

/// The header: the `lock` tile, the title and, while a share of
/// `waiting` waits, what waits (Part D item 1).
fn header(waiting: Option<&str>) -> Header {
    Header {
        icon: Some((IconName::Lock, Tone::Accent)),
        title: TITLE.into(),
        description: waiting.map(|repository| waiting_subtitle(repository).into()),
        ids: (TITLE_ID, Some(SUBTITLE_ID)),
    }
}

pub(crate) struct PassphraseDialog {
    /// Answered once: taken by the first answer.
    prompt: Option<PromptRequest>,
    /// The key's label, as the prompt names it.
    label: String,
    /// The prompt's key, by the ID the key list gives it: never drawn.
    key_id: String,
    /// The key's fingerprint, once the key list has it (`card`).
    fingerprint: Option<String>,
    generations: Generations,
    /// The key list's read, while it runs; dropped once the prompt is
    /// answered or withdrawn.
    read: Option<Task<()>>,
    guidance: &'static str,
    /// Let go once the dialog has answered.
    input: Option<Entity<InputState>>,
    /// The last submit was refused.
    refused: bool,
    /// The directory name of the repository whose share or post waits on
    /// the prompt, once found.
    waiting: Option<String>,
    origin: Option<FocusHandle>,
    _enter: Option<Subscription>,
}

impl PassphraseDialog {
    fn new(
        prompt: PromptRequest,
        origin: Option<FocusHandle>,
        services: &Services,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).masked(true));
        let enter = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.submit(window, cx);
            }
        });
        // The workspace that opens the dialog is being updated: what waits
        // is read once that update has ended.
        cx.defer_in(window, |this, window, cx| {
            this.waiting = waiting_share(window, cx);
            cx.notify();
        });
        let mut dialog = Self {
            label: prompt.label.clone(),
            key_id: prompt.key_id.to_string(),
            fingerprint: None,
            generations: Generations::default(),
            read: None,
            guidance: prompt.reason.guidance(),
            prompt: Some(prompt),
            input: Some(input),
            refused: false,
            waiting: None,
            origin,
            _enter: Some(enter),
        };
        dialog.read_key(services, cx);
        dialog
    }

    /// Whether its prompt is still unanswered.
    pub(super) fn waiting(&self) -> bool {
        self.prompt.is_some()
    }

    /// The input, while the dialog holds it.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn input(&self) -> Option<Entity<InputState>> {
        self.input.clone()
    }

    /// Submit: an acceptable passphrase becomes the one `SecretPassphrase`
    /// and is supplied; any other is refused, nothing is replied, and focus
    /// goes back to the field, also when Unlock was pressed (R-17).
    pub(crate) fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.input.clone() else {
            return;
        };
        let typed = input.read(cx).text().to_string();
        let Ok(passphrase) = SecretPassphrase::new(typed) else {
            self.refused = true;
            input.update(cx, |input, cx| input.focus(window, cx));
            cx.notify();
            return;
        };
        self.answer(PromptReply::Supplied(passphrase), window, cx);
        self.close(window, cx);
    }

    /// Not now: replies `Cancelled` and closes.
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
        self.drop_read();
        self.input = None;
        self._enter = None;
        cx.notify();
    }

    /// Quitting saves drafts first: empties the input, lets it go and
    /// gives the prompt back unanswered, to be held while the drafts are
    /// saved; the key list's read is dropped. The caller closes the dialog.
    pub(crate) fn withdraw(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PromptRequest> {
        self._enter = None;
        self.drop_read();
        if let Some(input) = self.input.take() {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
        self.prompt.take()
    }

    /// Empties the input and lets it go, drops the key list's read, then
    /// replies, once. False when the prompt was already answered.
    fn answer(&mut self, reply: PromptReply, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self._enter = None;
        self.drop_read();
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

/// The repository of the share or post the window's workspace has
/// running, by its directory name: what waits on the prompt.
fn waiting_share(window: &Window, cx: &App) -> Option<String> {
    let root = window.root::<Root>()??;
    let workspace = root.read(cx).view().clone().downcast::<Workspace>().ok()?;
    let operations = workspace.read(cx).services().operations().read(cx);
    let request = operations.running()?;
    // Not a Check now: it may prompt too, but it is not a share.
    matches!(
        Progressing::of(&request.intent)?,
        Progressing::Share | Progressing::Post
    )
    .then_some(())?;
    Some(scope_name(Some(request.intent.root()?)))
}

fn text(id: &'static str, value: impl Into<String>) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

/// A footer button. Enter on it presses it: its wrapper takes the dialog's
/// Enter (`Confirm`, dispatched on key down) before the dialog, whose
/// `on_ok` would submit. Enter in the input, or with no button focused,
/// still submits.
fn button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
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
        .into_any_element()
}

/// The note, always shown, with its `shield-check` (Part D item 6).
fn note(cx: &App) -> impl IntoElement + use<> {
    let colors = chrome_colors(cx);
    h_flex()
        .id("passphrase-note")
        .test_support()
        .aria_label(NOT_NOW_NOTE)
        .items_start()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(colors.secondary)
        .child(
            Icon::new(IconName::ShieldCheck)
                .text_color(colors.teal)
                .w(px(14.))
                .h(px(14.)),
        )
        .child(div().flex_1().min_w_0().child(NOT_NOW_NOTE))
}

impl Render for PassphraseDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = chrome_colors(cx);
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
        let field = v_flex()
            .gap(px(6.))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(text("passphrase-label", PASSPHRASE)),
            )
            .children(self.input.as_ref().map(|input| {
                div()
                    .id("passphrase-input")
                    .test_support()
                    .aria_label(PASSPHRASE)
                    .child(Input::new(input).aria_label(PASSPHRASE))
            }))
            .children(self.refused.then(|| {
                div()
                    .text_sm()
                    .text_color(colors.red)
                    .child(text("passphrase-refused", REFUSED))
            }));
        let content = v_flex()
            .gap_3()
            .child(self.card(cx))
            .child(
                div()
                    .text_sm()
                    .child(text("passphrase-reason", self.guidance)),
            )
            .child(field)
            .child(note(cx));
        // Not now first, the action last (Part D item 7; R-1).
        let buttons = vec![
            button("passphrase-cancel", CANCEL, false, cancel),
            button("passphrase-ok", UNLOCK, true, unlock),
        ];
        // The footer is the last child of the body element, which privacy
        // sweeps and focus reads find the buttons inside (R-19).
        v_flex()
            .id("passphrase-dialog")
            .test_support()
            .child(frame::body(content))
            .child(frame::footer(
                Footer {
                    hint: None,
                    buttons,
                },
                cx,
            ))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "passphrase_tests.rs"]
mod tests;
