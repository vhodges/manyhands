//! The create dialogs' drawing (design, Create (decision 3);
//! `05-sheets-and-dialogs.md`): the dialog's body, from a labelled field over
//! the Kit input, with its note in red under it, and a dialog button that
//! Enter presses and that says, while unavailable, why.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, Role, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex,
        input::{Input, InputState},
        v_flex,
    },
    div,
};

use super::{CANCEL, CREATE, CreateDialog, FILE_NAME, FOLDER, Inputs, TITLE, TYPE};
use crate::desktop::{
    model::edit::fields::{Field, required_message},
    view::item::text,
};

/// A labelled field `id`: its label, the Kit input `input` in the element
/// `{id}-input`, and, with a `note`, a red border and the note below it, as
/// `{id}-note`.
fn field(
    id: &'static str,
    label: &'static str,
    input: &Entity<InputState>,
    note: Option<String>,
    cx: &App,
) -> impl IntoElement {
    let danger = cx.theme().danger;
    let mut control = Input::new(input).aria_label(label);
    if note.is_some() {
        control = control.border_color(danger);
    }
    v_flex()
        .id(id)
        .test_support()
        .role(Role::Group)
        .aria_label(label)
        .gap_1()
        .child(div().text_sm().child(label))
        .child(
            div()
                .id(ElementId::from(format!("{id}-input")))
                .test_support()
                .aria_label(label)
                .child(control),
        )
        .children(note.map(|note| {
            text(ElementId::from(format!("{id}-note")), note)
                .text_xs()
                .text_color(danger)
        }))
}

/// A dialog button `id`, the dialog's own action when `primary`. Enter on
/// it presses it: its wrapper takes the dialog's Enter before the dialog
/// does. While it is not `enabled`, it says why with `note`, in the element
/// `{id}-note`, as its tooltip and as its description.
fn button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    enabled: bool,
    note: Option<String>,
    press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let press = Rc::new(press);
    let on_enter = press.clone();
    let button = Button::new((id, 0usize))
        .label(label)
        .disabled(!enabled)
        .on_click(move |event, window, cx| press(event, window, cx));
    let button = if primary { button.primary() } else { button };
    let wrapper = div()
        .id(id)
        .test_support()
        .role(Role::Group)
        .aria_label(label)
        .on_action(move |_: &Confirm, window, cx| {
            if enabled {
                on_enter(&ClickEvent::default(), window, cx)
            }
        });
    match note.filter(|_| !enabled) {
        Some(note) => wrapper.aria_description(note.clone()).child(
            div()
                .id(ElementId::from(format!("{id}-note")))
                .test_support()
                .aria_label(note.clone())
                .child(button.tooltip(note)),
        ),
        None => wrapper.child(button),
    }
}

/// The dialog: its fields, each with its note, what the last Create said,
/// and Cancel and Create.
impl Render for CreateDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fields = self.fields(cx);
        let note = |field| required_message(&fields, field).map(str::to_owned);
        let body = v_flex()
            .id("create-dialog")
            .test_support()
            .track_focus(&self.focus)
            .gap_3()
            .child(field(
                "create-title",
                TITLE,
                &self.title,
                note(Field::Title),
                cx,
            ));
        let body = match &self.inputs {
            Inputs::Ticket { ticket_type } => body.child(field(
                "create-type",
                TYPE,
                ticket_type,
                note(Field::Type),
                cx,
            )),
            Inputs::Document(place) => body
                .child(field("create-folder", FOLDER, &place.folder, None, cx))
                .child(field(
                    "create-file",
                    FILE_NAME,
                    &place.file_name,
                    self.path_note(),
                    cx,
                )),
        };
        let danger = cx.theme().danger;
        let failure = self
            .failure
            .clone()
            .map(|words| text("create-failure", words).text_sm().text_color(danger));
        let entity = cx.entity().downgrade();
        let cancel = button("create-cancel", CANCEL, false, true, None, {
            let entity = entity.clone();
            move |_, window, cx| {
                let _ = entity.update(cx, |dialog, cx| dialog.cancel(window, cx));
            }
        });
        let create = button(
            "create-ok",
            CREATE,
            true,
            self.offered(cx),
            self.reason(cx),
            move |_, window, cx| {
                let _ = entity.update(cx, |dialog, cx| dialog.create(window, cx));
            },
        );
        body.children(failure)
            .child(h_flex().gap_2().justify_end().child(cancel).child(create))
    }
}
