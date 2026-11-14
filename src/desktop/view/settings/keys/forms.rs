//! Make a new key and Use a key I already have… (Wave 04 Cycle 04 design
//! C2.2; R-18): each shows today's form below it, the Generate form or the
//! Import form, and hides the other; neither is shown at first, and the
//! button of the shown form hides it. Hiding the Generate form, by either
//! button, empties its passphrase fields (B3, R-13).
//!
//! While the page retries a protected generation, the retry's prompt is
//! the Generate form, so the form is shown however the retry began (R-54):
//! Make a new key leaves it shown, and Use a key I already have… is
//! unavailable, saying why (R-57). Cancel ends the retry; the Generate form
//! stays shown.

use gpui_kit::{
    ClickEvent, Context, IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window,
    component::{
        ActiveTheme, Disableable, WindowExt,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
};

use super::super::wrapped;
use super::render::{heading, text};
use super::{
    CONFIRMATION, CONFIRMATION_FIELD, GENERATE, GENERATE_HEADING, GENERATE_NOTE, GENERATE_SUBMIT,
    IMPORT, IMPORT_HEADING, IMPORT_LABEL_FIELD, IMPORT_NOTE, IMPORT_SUBMIT, KeysPage, LABEL,
    LABEL_FIELD, MAKE_ID, MAKE_KEY, PASSPHRASE, PASSPHRASE_FIELD, PRIVATE_FIELD, PRIVATE_PATH,
    PUBLIC_FIELD, PUBLIC_PATH, RETRY, RETRY_CANCEL, RETRY_CANCEL_ID, RETRY_FIRST, USE_EXISTING,
    USE_EXISTING_ID, retrying_line,
};

/// The form a button shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum KeyForm {
    /// Make a new key's: today's Generate form.
    Generate,
    /// Use a key I already have…'s: today's Import form.
    Import,
}

impl KeysPage {
    /// Whether the Generate form is shown: its button's, or a retry's.
    fn generate_shown(&self) -> bool {
        self.form == Some(KeyForm::Generate) || self.retrying.is_some()
    }

    /// Whether the Import form is shown: its button's, and no retry.
    fn import_shown(&self) -> bool {
        self.form == Some(KeyForm::Import) && self.retrying.is_none()
    }

    /// The button of `form` pressed: it shows `form` and hides the other,
    /// or hides `form` when it is the one shown. While retrying, neither
    /// changes what is shown. The Generate form hiding empties its
    /// passphrase fields.
    pub(super) fn toggle_form(
        &mut self,
        form: KeyForm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || self.retrying.is_some() {
            return;
        }
        let was_generate = self.generate_shown();
        self.form = (self.form != Some(form)).then_some(form);
        if was_generate && !self.generate_shown() {
            self.forget_passphrase(window, cx);
        }
        cx.notify();
    }

    /// The Generate form shown, for a retry's prompt (R-54).
    pub(super) fn show_generate(&mut self) {
        self.form = Some(KeyForm::Generate);
    }

    /// Make a new key (primary) and Use a key I already have…, each saying
    /// whether its form is shown, and why Use a key… is unavailable while
    /// the page retries.
    pub(super) fn form_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let retrying = self.retrying.is_some();
        let make = Button::new((MAKE_ID, 0usize))
            .primary()
            .label(MAKE_KEY)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_form(KeyForm::Generate, window, cx)
            }));
        let make = wrapped(MAKE_ID, MAKE_KEY, &self.focus[MAKE_ID], &self.scroll, make)
            .aria_expanded(self.generate_shown());
        let existing = Button::new((USE_EXISTING_ID, 0usize))
            .label(USE_EXISTING)
            .disabled(retrying)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_form(KeyForm::Import, window, cx)
            }));
        let existing = wrapped(
            USE_EXISTING_ID,
            USE_EXISTING,
            &self.focus[USE_EXISTING_ID],
            &self.scroll,
            existing,
        )
        .aria_expanded(self.import_shown());
        v_flex()
            .gap_0p5()
            .child(h_flex().gap_2().flex_wrap().child(make).child(existing))
            .children(retrying.then(|| {
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(text("keys-use-existing-reason", RETRY_FIRST))
            }))
    }

    /// The form a button shows, if any.
    pub(super) fn shown_form(&mut self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        if self.generate_shown() {
            Some(self.generate_form(cx).into_any_element())
        } else if self.import_shown() {
            Some(self.import_form(cx).into_any_element())
        } else {
            None
        }
    }

    fn generate_form(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let retrying = self
            .retrying
            .as_ref()
            .map(|retrying| retrying.label.clone());
        let submit = {
            let label = if retrying.is_some() { RETRY } else { GENERATE };
            let button =
                Button::new((GENERATE_SUBMIT, 0usize))
                    .primary()
                    .label(label)
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.submit_generate(window, cx)
                    }));
            wrapped(
                GENERATE_SUBMIT,
                label,
                &self.focus[GENERATE_SUBMIT],
                &self.scroll,
                button,
            )
        };
        let cancel = retrying.is_some().then(|| {
            self.button(
                RETRY_CANCEL_ID,
                RETRY_CANCEL_ID.to_owned(),
                RETRY_CANCEL,
                |this, window, cx| this.cancel_retry(window, cx),
                cx,
            )
        });
        let mut form = v_flex()
            .gap_2()
            .child(heading("key-generate-heading", GENERATE_HEADING));
        form = match &retrying {
            Some(label) => form.child(text("key-generate-retrying", retrying_line(label))),
            None => form.child(self.field(LABEL_FIELD, LABEL, cx)),
        };
        form.child(self.field(PASSPHRASE_FIELD, PASSPHRASE, cx))
            .child(self.field(CONFIRMATION_FIELD, CONFIRMATION, cx))
            .child(div().text_xs().text_color(muted).child(GENERATE_NOTE))
            .children(
                self.generate_message
                    .map(|why| text("key-generate-message", why)),
            )
            .child(h_flex().gap_2().child(submit).children(cancel))
    }

    fn import_form(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let button = Button::new((IMPORT_SUBMIT, 0usize)).label(IMPORT).on_click(
            cx.listener(|this, _: &ClickEvent, window, cx| this.submit_import(window, cx)),
        );
        v_flex()
            .gap_2()
            .child(heading("key-import-heading", IMPORT_HEADING))
            .child(self.field(IMPORT_LABEL_FIELD, LABEL, cx))
            .child(self.field(PRIVATE_FIELD, PRIVATE_PATH, cx))
            .child(self.field(PUBLIC_FIELD, PUBLIC_PATH, cx))
            .child(div().text_xs().text_color(muted).child(IMPORT_NOTE))
            .children(
                self.import_message
                    .map(|why| text("key-import-message", why)),
            )
            .child(wrapped(
                IMPORT_SUBMIT,
                IMPORT,
                &self.focus[IMPORT_SUBMIT],
                &self.scroll,
                button,
            ))
    }
}
