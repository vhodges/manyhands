//! The create dialogs' drawing (design, Create (decision 3);
//! `05-sheets-and-dialogs.md`): the dialog's body, from a labelled field over
//! the Kit input, with its note in red under it, and a dialog button that
//! Enter presses and that says, while unavailable, why. Move… and New
//! folder draw their dialogs with the same two (Task 24).

use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, Role, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
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
pub(in crate::desktop::view) fn field(
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
pub(in crate::desktop::view) fn button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    enabled: bool,
    note: Option<String>,
    press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    wrapped(id, label, primary, enabled, note, None, press)
}

/// A dialog's primary button `id`, as `button` draws it, whose wrapper
/// also tracks `focus`: no tab stop of its own, but where a keyed
/// press of the button parks focus while the button disables
/// (`keep_focus_in`), ringed while it holds it, and where Enter presses the
/// button again.
pub(in crate::desktop::view) fn held_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    note: Option<String>,
    (focus, window, cx): (&FocusHandle, &Window, &App),
    press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let ring = focus.is_focused(window).then(|| {
        div()
            .id(ElementId::from(format!("{id}-focus-ring")))
            .test_support()
            .absolute()
            .inset_0()
            .rounded_sm()
            .border_2()
            .border_color(cx.theme().ring)
            .into_any_element()
    });
    wrapped(id, label, true, enabled, note, Some((focus, ring)), press)
}

/// `button`, its wrapper tracking `held`'s focus and drawing its ring when
/// given.
fn wrapped(
    id: &'static str,
    label: &'static str,
    primary: bool,
    enabled: bool,
    note: Option<String>,
    held: Option<(&FocusHandle, Option<AnyElement>)>,
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
    let wrapper = match held {
        Some((focus, ring)) => wrapper.track_focus(focus).relative().children(ring),
        None => wrapper,
    };
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

/// A dialog's primary button was pressed: it disables while the dialog
/// works, and a disabled button holds no focus. When focus is on it, in
/// the dialog's own `focus` but in none of its `fields` (a click leaves
/// focus in the field it was in), focus moves to the button's wrapper,
/// `primary` (`held_button`). So it stays in the dialog, which closes itself once
/// done only while focus is in it.
pub(in crate::desktop::view) fn keep_focus_in(
    focus: &FocusHandle,
    primary: &FocusHandle,
    fields: &[&Entity<InputState>],
    window: &mut Window,
    cx: &mut App,
) {
    let in_field = fields
        .iter()
        .any(|field| field.read(cx).focus_handle(cx).contains_focused(window, cx));
    if focus.contains_focused(window, cx) && !in_field {
        window.focus(primary, cx);
    }
}

/// After a keyed press that was refused, with the dialog still open
/// (fix round 1, M4): while focus waits on the primary button's wrapper,
/// `primary`, and the refusal's note is under the field `noted`, focus
/// goes to that field. Otherwise it stays on the button, which Enter
/// presses again once it is offered.
pub(in crate::desktop::view) fn focus_the_note(
    primary: &FocusHandle,
    noted: Option<&Entity<InputState>>,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(field) = noted
        && primary.is_focused(window)
    {
        let field = field.clone();
        window.defer(cx, move |window, cx| {
            field.update(cx, |input, cx| input.focus(window, cx))
        });
    }
}

/// The dialog: its fields, each with its note, what the last Create said,
/// and Cancel and Create.
impl Render for CreateDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fields = self.fields(cx);
        let note = |field| required_message(&fields, field).map(str::to_owned);
        let noted = match &self.inputs {
            _ if note(Field::Title).is_some() => Some(&self.title),
            Inputs::Ticket { ticket_type } => note(Field::Type).is_some().then_some(ticket_type),
            Inputs::Document(place) => self.path_note().is_some().then_some(&place.file_name),
        };
        focus_the_note(&self.primary, noted, window, cx);
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
        let create = held_button(
            "create-ok",
            CREATE,
            self.offered(cx),
            self.reason(cx),
            (&self.primary, window, cx),
            move |_, window, cx| {
                let _ = entity.update(cx, |dialog, cx| {
                    let fields = match &dialog.inputs {
                        Inputs::Ticket { ticket_type } => vec![&dialog.title, ticket_type],
                        Inputs::Document(place) => {
                            vec![&dialog.title, &place.folder, &place.file_name]
                        }
                    };
                    keep_focus_in(&dialog.focus, &dialog.primary, &fields, window, cx);
                    dialog.create(window, cx)
                });
            },
        );
        body.children(failure)
            .child(h_flex().gap_2().justify_end().child(cancel).child(create))
    }
}
