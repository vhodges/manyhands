//! Key rows, forms and the page's latest result.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        h_flex,
        input::Input,
        v_flex,
    },
    div, px,
};
use manyhands::{
    repository::{KeyDto, KeyOwnership, KeyPrivateSourceState, SharedKeyId},
    results::Outcome,
};

use super::super::{Wrapped, wrapped};
use super::{
    CLEAR, CLEAR_ID, CONFIRMATION, CONFIRMATION_FIELD, COPIED, COPY, Copied, DELETE, GENERATE,
    GENERATE_HEADING, GENERATE_NOTE, GENERATE_SUBMIT, IMPORT, IMPORT_HEADING, IMPORT_LABEL_FIELD,
    IMPORT_NOTE, IMPORT_SUBMIT, KEYS_HEADING, KIND_GENERATED, KIND_IMPORTED, KeysPage, LABEL,
    LABEL_FIELD, NO_FINGERPRINT, NO_KEYS, PASSPHRASE, PASSPHRASE_FIELD, PRIVATE_FIELD,
    PRIVATE_PATH, PUBLIC_FIELD, PUBLIC_PATH, READING_KEYS, RETRY, RETRY_CANCEL, RETRY_CANCEL_ID,
    RETRY_OFFER, RETRY_OFFER_ID, SELECT, SELECTED, SELECTION_NONE, SOURCE_MISSING,
    SOURCE_UNAVAILABLE, UNREGISTER, retrying_line, selection_line,
};
use crate::desktop::{
    model::{list_state::LABEL_BUSY, read_state::Read},
    services::{KEY_STORE_UNAVAILABLE, requests::RequestState},
    view::operations::{outcome_label, state_label},
};

fn text(id: impl Into<gpui_kit::ElementId>, value: impl Into<String>) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .text_sm()
        .child(value)
}

fn heading(id: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .text_base()
        .font_weight(gpui_kit::FontWeight::BOLD)
        .child(label)
}

impl KeysPage {
    fn button(
        &mut self,
        id: impl Into<gpui_kit::ElementId>,
        key: String,
        label: &'static str,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Wrapped {
        let focus = self.focus(key.clone(), cx);
        let button = Button::new(gpui_kit::ElementId::Name(format!("{key}-button").into()))
            .label(label)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)),
            );
        wrapped(id, label, &focus, &self.scroll, button)
    }

    fn field(&self, id: &'static str, label: &'static str, cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_0p5()
            .child(div().text_xs().text_color(muted).child(label))
            .child(
                wrapped(
                    id,
                    label,
                    &self.focus[id],
                    &self.scroll,
                    Input::new(&self.inputs[id]).aria_label(label),
                )
                .w(px(480.)),
            )
    }

    fn key_row(&mut self, ix: usize, key: &KeyDto, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let kind = match key.ownership {
            KeyOwnership::Generated => KIND_GENERATED,
            KeyOwnership::Imported => KIND_IMPORTED,
        };
        let source = match key.private_source_state {
            KeyPrivateSourceState::Available => None,
            KeyPrivateSourceState::Missing => Some(SOURCE_MISSING),
            KeyPrivateSourceState::Unavailable => Some(SOURCE_UNAVAILABLE),
        };
        let mut actions = h_flex().gap_2().flex_wrap();
        if let Ok(id) = SharedKeyId::parse(&key.id) {
            if key.public_key_path.is_some() {
                actions = actions.child(self.button(
                    ("key-copy", ix),
                    format!("key-copy-{ix}"),
                    COPY,
                    move |this, window, cx| this.copy(id, window, cx),
                    cx,
                ));
            }
            if !key.selected {
                actions = actions.child(self.button(
                    ("key-select", ix),
                    format!("key-select-{ix}"),
                    SELECT,
                    move |this, window, cx| this.submit_select(id, window, cx),
                    cx,
                ));
            }
            actions = actions.child(self.button(
                ("key-unregister", ix),
                format!("key-unregister-{ix}"),
                UNREGISTER,
                move |this, window, cx| this.submit_unregister(id, window, cx),
                cx,
            ));
            if key.ownership == KeyOwnership::Generated {
                let focus = self.focus(format!("key-delete-{ix}"), cx);
                let button = Button::new(("key-delete-button", ix))
                    .danger()
                    .label(DELETE)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.submit_delete(id, ix, window, cx)
                    }));
                actions = actions.child(wrapped(
                    ("key-delete", ix),
                    DELETE,
                    &focus,
                    &self.scroll,
                    button,
                ));
            }
        }
        v_flex()
            .gap_0p5()
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(text(("key-label", ix), key.label.clone())),
            )
            .child(text(
                ("key-fingerprint", ix),
                key.fingerprint
                    .clone()
                    .unwrap_or_else(|| NO_FINGERPRINT.to_owned()),
            ))
            .child(div().text_color(muted).child(text(("key-kind", ix), kind)))
            .children(key.selected.then(|| text(("key-selected", ix), SELECTED)))
            .children(source.map(|source| text(("key-source", ix), source)))
            .child(actions)
    }

    fn key_rows(&mut self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let muted = cx.theme().muted_foreground;
        let body = v_flex()
            .gap_2()
            .child(heading("keys-heading", KEYS_HEADING));
        let keys = match &self.keys {
            Read::None => return body,
            Read::Loading => return body.child(text("keys-state", READING_KEYS)),
            Read::Busy => return body.child(text("keys-state", LABEL_BUSY)),
            Read::Failed(code) => {
                return body
                    .child(text("keys-state", code.message()))
                    .child(div().text_xs().text_color(muted).child(code.as_str()));
            }
            Read::Ready(keys) => keys.clone(),
        };
        let selection = match keys.iter().find(|key| key.selected) {
            Some(key) => selection_line(&key.label),
            None => SELECTION_NONE.to_owned(),
        };
        let mut body = body.child(text("keys-selection", selection));
        if keys.is_empty() {
            body = body.child(text("keys-state", NO_KEYS));
        }
        for (ix, key) in keys.iter().enumerate() {
            body = body.child(self.key_row(ix, key, cx));
        }
        let clear = self.button(
            CLEAR_ID,
            CLEAR_ID.to_owned(),
            CLEAR,
            |this, window, cx| this.submit_clear(window, cx),
            cx,
        );
        body.child(clear).children(self.copied_line(cx))
    }

    fn copied_line(&self, cx: &Context<Self>) -> Option<impl IntoElement + use<>> {
        let muted = cx.theme().muted_foreground;
        let line = match self.copied.as_ref()? {
            Copied::Done => v_flex().child(text("keys-copied", COPIED)),
            Copied::Busy => v_flex().child(text("keys-copied", LABEL_BUSY)),
            Copied::Failed(code) => v_flex().child(text("keys-copied", code.message())).child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(text("keys-copy-code", code.as_str())),
            ),
        };
        Some(line)
    }

    fn result(&mut self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let id = self.submitted?;
        let muted = cx.theme().muted_foreground;
        let (line, code) = {
            let operations = self.services.operations().read(cx);
            let state = &operations.request(id)?.state;
            match state {
                RequestState::Finished(outcome, code, _, _, message) => {
                    let line = if *outcome == Outcome::Success {
                        outcome_label(*outcome).to_owned()
                    } else {
                        format!("{}: {message}", outcome_label(*outcome))
                    };
                    (line, Some(code.as_str()))
                }
                RequestState::NotSent(_) => (state_label(state), None),
                _ => return None,
            }
        };
        let retry = self.retry_offered(cx).then(|| {
            self.button(
                RETRY_OFFER_ID,
                RETRY_OFFER_ID.to_owned(),
                RETRY_OFFER,
                move |this, window, cx| this.ask_again(id, window, cx),
                cx,
            )
        });
        Some(
            v_flex()
                .gap_0p5()
                .child(text("keys-result", line))
                .children(code.map(|code| {
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(text("keys-result-code", code))
                }))
                .children(retry),
        )
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

impl Render for KeysPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let keys = self.key_rows(cx);
        let generate = self.generate_form(cx);
        let import = self.import_form(cx);
        let result = self.result(cx);
        v_flex()
            .id("settings-keys")
            .test_support()
            .gap_4()
            .children(
                (!self.store_available)
                    .then(|| text("keys-store-unavailable", KEY_STORE_UNAVAILABLE)),
            )
            .child(keys)
            .child(generate)
            .child(import)
            .children(result)
    }
}
