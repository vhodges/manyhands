//! What Sharing & keys' parts share (text, headings, buttons and fields),
//! the page's latest result, the info box (design C2.3), and the page in
//! the order C2 gives: the key list, the two buttons and the form shown,
//! the result, then the info box. Trusted servers follows, the Hosts page's
//! own view (C2.4).

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, button::Button, h_flex, input::Input, v_flex},
    div, px,
};
use manyhands::results::Outcome;

use super::super::{Wrapped, field_column, wrapped};
use super::{COPIED, Copied, KEYS_NOTE, KeysPage, RETRY_OFFER, RETRY_OFFER_ID};
use crate::desktop::{
    model::list_state::LABEL_BUSY,
    services::{KEY_STORE_UNAVAILABLE, requests::RequestState},
    view::{
        operations::{outcome_label, state_label},
        rows::icon,
    },
};

/// The info box's icon size.
const NOTE_ICON: f32 = 14.;

pub(super) fn text(
    id: impl Into<gpui_kit::ElementId>,
    value: impl Into<String>,
) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .text_sm()
        .child(value)
}

pub(super) fn heading(id: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .text_base()
        .font_weight(gpui_kit::FontWeight::BOLD)
        .child(label)
}

impl KeysPage {
    pub(super) fn button(
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

    pub(super) fn field(
        &self,
        id: &'static str,
        label: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        field_column()
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
                .w_full(),
            )
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
                move |this, window, cx| {
                    if let Some(field) = this.ask_again(id, window, cx) {
                        window.focus(&field, cx);
                    }
                },
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

    /// The info box (C2.3; R-19): `05`'s sentence, true of today's
    /// handling (fact 43).
    fn note(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        h_flex()
            .items_start()
            .gap_2()
            .p_3()
            .border_1()
            .border_color(border)
            .rounded(px(8.))
            .child(icon("lock", NOTE_ICON, muted))
            .child(text("keys-note", KEYS_NOTE))
    }
}

impl Render for KeysPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let keys = self.key_rows(cx);
        let copied = self.copied_line(cx);
        let buttons = self.form_buttons(cx);
        let form = self.shown_form(cx);
        let result = self.result(cx);
        let note = self.note(cx);
        v_flex()
            .id("settings-keys")
            .test_support()
            .gap_4()
            .children(
                (!self.store_available)
                    .then(|| text("keys-store-unavailable", KEY_STORE_UNAVAILABLE)),
            )
            .child(keys.children(copied))
            .child(buttons)
            .children(form)
            .children(result)
            .child(note)
    }
}
