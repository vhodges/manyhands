//! The create dialogs' controls (design, Create (decision 3);
//! `05-sheets-and-dialogs.md`): a labelled field over the Kit input, with
//! its note in red under it, and a dialog button that Enter presses and
//! that says, while unavailable, why.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        input::{Input, InputState},
        v_flex,
    },
    div,
};

use crate::desktop::view::item::text;

/// A labelled field `id`: its label, the Kit input `input` in the element
/// `{id}-input`, and, with a `note`, a red border and the note below it, as
/// `{id}-note`.
pub(super) fn field(
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
pub(super) fn button(
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
