//! Drawing the Add repository sheet (Wave 04 Cycle 05a design A1, B5 to
//! B10): a Kit sheet 470 px wide, the frame's header, a body in a scroll of
//! its own laid over the Kit's, which takes the page keys (B12), and the
//! frame's footer with Cancel, Add repository and Create repository (B10).
//! The body is Add an existing repository, then Create a repository, each
//! a folder with Browse, its fields and what it said, then why Browse is
//! unavailable, drawn once.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex,
        input::Input,
        scroll::Scrollbar,
        sheet::Sheet,
        v_flex,
    },
    div, px,
};

use super::{
    ADD, ADD_DESCRIPTION, ADD_HEADING, ADD_TITLE, ADDING, AddRepositorySheet, BROWSE, CANCEL,
    CANCEL_ID, CREATE, CREATE_FOLDER, CREATE_HEADING, CREATE_IDENTITY, FOLDER, Form,
    IDENTITY_REQUIRED, PREFIX, PRIMARY_BRANCH,
};
use crate::desktop::{
    keys::{ScrollPageDown, ScrollPageUp, ScrollToBottom, ScrollToTop},
    view::{
        frame::{self, Footer, Header},
        scroll::ContentScroll,
        settings::{EMAIL, NAME, field_column, footer_kit, wrapped},
    },
};

/// The sheet's width (`05`, Add repository; B1).
const WIDTH: f32 = 470.;

/// A piece of text a test reads by `id`.
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

fn heading(id: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .text_base()
        .font_weight(gpui_kit::FontWeight::BOLD)
        .child(label)
}

impl AddRepositorySheet {
    /// A labeled field: its label, then its input in its wrapper.
    fn field(&self, id: &'static str, label: &'static str, cx: &Context<Self>) -> impl IntoElement {
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

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        primary: bool,
        disabled: bool,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let button = Button::new((id, 0usize))
            .label(label)
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)),
            );
        let button = if primary { button.primary() } else { button };
        wrapped(id, label, &self.focus[id], &self.scroll, button)
    }

    /// The folder field with Browse beside it.
    fn folder(&self, form: Form, label: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.browsing.is_some() || self.browse_unavailable.is_some();
        h_flex()
            .gap_2()
            .items_end()
            .child(self.field(form.path(), label, cx))
            .child(self.button(
                form.browse(),
                BROWSE,
                false,
                disabled,
                move |this, window, cx| this.browse(form, window, cx),
                cx,
            ))
    }

    /// A footer submit: a primary Kit button drawn with the sheet's own
    /// handle, which the confirmation gives focus back to (B12, G13). While
    /// an add of this open is pending it reads "Adding…" and is disabled
    /// (R-27): it then tracks no focus, and its wrapper holds the handle
    /// until it is enabled again (Ruling 18).
    fn submit_button(
        &self,
        form: Form,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = form.submit();
        let label = if self.adding { ADDING } else { label };
        let button = Button::new((id, 0usize))
            .label(label)
            .primary()
            .disabled(self.adding)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| this.submit(form, window, cx)),
            );
        footer_kit(id, label, &self.focus[id], (id, 0usize), button)
    }

    /// The footer's Cancel (B10).
    fn cancel_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let button = Button::new((CANCEL_ID, 0usize))
            .label(CANCEL)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.cancel(cx)));
        footer_kit(
            CANCEL_ID,
            CANCEL,
            &self.focus[CANCEL_ID],
            (CANCEL_ID, 0usize),
            button,
        )
    }

    fn message(&self, form: Form) -> Option<impl IntoElement + use<>> {
        self.messages
            .get(&form)
            .map(|why| text(form.message(), *why))
    }

    fn add_form(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let path = self.text(Form::Add.path(), cx);
        let identity = self.inspection.identity_required(&path).then(|| {
            v_flex()
                .gap_2()
                .child(text("add-identity-required", IDENTITY_REQUIRED))
                .child(self.field(Form::Add.name(), NAME, cx))
                .child(self.field(Form::Add.email(), EMAIL, cx))
        });
        v_flex()
            .gap_2()
            .child(heading("add-heading", ADD_HEADING))
            .child(self.folder(Form::Add, FOLDER, cx))
            .children(self.inspection(cx))
            .child(self.field(Form::Add.branch(), PRIMARY_BRANCH, cx))
            .child(self.field(Form::Add.prefix(), PREFIX, cx))
            .children(identity)
            .children(self.message(Form::Add))
    }

    fn create_form(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_2()
            .child(heading("create-heading", CREATE_HEADING))
            .child(self.folder(Form::Create, CREATE_FOLDER, cx))
            .child(self.field(Form::Create.branch(), PRIMARY_BRANCH, cx))
            .child(div().text_xs().text_color(muted).child(CREATE_IDENTITY))
            .child(self.field(Form::Create.name(), NAME, cx))
            .child(self.field(Form::Create.email(), EMAIL, cx))
            .child(self.field(Form::Create.prefix(), PREFIX, cx))
            .children(self.message(Form::Create))
    }

    /// The sheet as this frame draws it. The builder Root keeps calls this
    /// on every render, so all its state is the sheet's.
    pub(crate) fn sheet(
        &mut self,
        sheet: Sheet,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Sheet {
        if let Some(branch) = self.pending_branch.take() {
            self.inputs[Form::Add.branch()]
                .update(cx, |input, cx| input.set_value(branch, window, cx));
        }
        let unavailable = self
            .browse_unavailable
            .map(|why| text("browse-unavailable", why));
        let handle = self.scroll.handle().clone();
        let footer = Footer {
            hint: None,
            buttons: vec![
                self.cancel_button(cx).into_any_element(),
                self.submit_button(Form::Add, ADD, cx).into_any_element(),
                self.submit_button(Form::Create, CREATE, cx)
                    .into_any_element(),
            ],
        };
        sheet
            .size(px(WIDTH))
            .title(frame::sheet_header(
                Header {
                    icon: None,
                    title: ADD_TITLE.into(),
                    description: Some(ADD_DESCRIPTION.into()),
                    ids: ("add-sheet-title", Some("add-sheet-description")),
                },
                cx,
            ))
            .footer(frame::sheet_footer(footer, cx))
            // The Kit's body grows with what it holds; laid over it, the
            // sheet scrolls on its own handle, which reveals the focused
            // control and takes the page keys (B12, R-8).
            .child(
                div()
                    .id("add-sheet")
                    .test_support()
                    .absolute()
                    .inset_0()
                    .on_action(cx.listener(|this, _: &ScrollPageUp, _, cx| {
                        this.scroll_body(|scroll| scroll.page(-1.), cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollPageDown, _, cx| {
                        this.scroll_body(|scroll| scroll.page(1.), cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollToTop, _, cx| {
                        this.scroll_body(ContentScroll::to_top, cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollToBottom, _, cx| {
                        this.scroll_body(ContentScroll::to_bottom, cx)
                    }))
                    .child(
                        div()
                            .id("add-scroll")
                            .test_support()
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&handle)
                            .child(
                                v_flex()
                                    .gap_4()
                                    .pr_4()
                                    .pb_4()
                                    .child(self.add_form(cx))
                                    .child(self.create_form(cx))
                                    .children(unavailable),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .child(Scrollbar::vertical(&handle).id("add-scrollbar")),
                    ),
            )
    }

    /// A page key moves the body, and nothing behind the sheet.
    fn scroll_body(&mut self, scroll: impl Fn(&ContentScroll) -> bool, cx: &mut Context<Self>) {
        if scroll(&self.scroll) {
            cx.notify();
        }
    }
}
