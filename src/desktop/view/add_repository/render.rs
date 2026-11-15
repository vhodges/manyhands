//! Drawing the Add repository forms: Add an existing repository, then
//! Create a repository, each a folder with Browse, its fields, what it
//! said and its submit, then why Browse is unavailable, drawn once.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex,
        input::Input,
        v_flex,
    },
    div,
};

use super::{
    ADD, ADD_HEADING, AddRepositorySheet, BROWSE, CREATE, CREATE_FOLDER, CREATE_HEADING,
    CREATE_IDENTITY, FOLDER, Form, IDENTITY_REQUIRED, PREFIX, PRIMARY_BRANCH,
};
use crate::desktop::view::settings::{EMAIL, NAME, field_column, wrapped};

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
            .child(self.button(
                Form::Add.submit(),
                ADD,
                true,
                false,
                |this, window, cx| this.submit(Form::Add, window, cx),
                cx,
            ))
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
            .child(self.button(
                Form::Create.submit(),
                CREATE,
                true,
                false,
                |this, window, cx| this.submit(Form::Create, window, cx),
                cx,
            ))
    }
}

/// Both forms, then why Browse is unavailable, once (fact 14). Spaced as
/// the Repositories page spaces its sections, which it draws them among.
impl Render for AddRepositorySheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(branch) = self.pending_branch.take() {
            self.inputs[Form::Add.branch()]
                .update(cx, |input, cx| input.set_value(branch, window, cx));
        }
        let unavailable = self
            .browse_unavailable
            .map(|why| text("browse-unavailable", why));
        v_flex()
            .gap_4()
            .child(self.add_form(cx))
            .child(self.create_form(cx))
            .children(unavailable)
    }
}
