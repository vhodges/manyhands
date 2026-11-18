//! The Repositories settings page (design, Settings / Repositories): the
//! registered repositories, with a chosen one's identity and Remove
//! (`manage`), then its remotes under Sharing (the Remotes page's view,
//! Wave 04 Cycle 04 design C1), then Add or create a repository…, which
//! opens the Add repository sheet in Settings' place (Wave 04 Cycle 05a
//! B11, AW-3).
//!
//! Every control sits in a focus-tracking wrapper with a scroll marker,
//! so a control that takes focus below the fold is brought into view.
//! Text is fixed strings, the library's own values, and
//! `ResultCode::message()` with its code.

use std::collections::HashMap;

use gpui_kit::{
    AnyView, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, TestSupportExt,
    Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div,
};

mod manage;

pub(crate) use manage::RepositoriesEvent;

use super::{field_column, wrapped, wrapped_kit};
use crate::desktop::{
    services::Services,
    view::{repositories::ADD_OR_CREATE, scroll::ContentScroll},
};

pub(crate) const NAME: &str = "Name";
pub(crate) const EMAIL: &str = "Email";

/// Add or create a repository…'s wrapper, and its own Kit button inside it
/// (B11): where `SettingsRoute::AddButton` lands (G4).
const ADD_BUTTON: &str = "settings-add-repository";

pub(crate) struct RepositoriesPage {
    services: Services,
    scroll: ContentScroll,
    inputs: HashMap<&'static str, Entity<InputState>>,
    /// The focus each control's wrapper tracks, by its ID.
    focus: HashMap<&'static str, FocusHandle>,
    /// The registered repositories, their identity and their removal
    /// (`manage`).
    managed: manage::Managed,
    /// The Remotes page, drawn under Sharing for the chosen repository.
    sharing: AnyView,
    _inputs: Vec<Subscription>,
    _operations: Subscription,
}

impl RepositoriesPage {
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        sharing: AnyView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut inputs = HashMap::new();
        let mut subscriptions = Vec::new();
        let mut focus = HashMap::new();
        for id in manage::FIELDS {
            let input = cx.new(|cx| InputState::new(window, cx));
            subscriptions.push(cx.subscribe_in(
                &input,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        this.submit_identity(window, cx)
                    }
                },
            ));
            inputs.insert(id, input);
        }
        for id in manage::FIELDS
            .into_iter()
            .chain(manage::CONTROLS)
            .chain([ADD_BUTTON])
        {
            focus.insert(id, cx.focus_handle());
        }
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        });
        let managed = manage::Managed::new(&services, cx);
        // The list is read when the page is shown (`opened`).
        Self {
            services,
            scroll,
            inputs,
            focus,
            managed,
            sharing,
            _inputs: subscriptions,
            _operations: operations,
        }
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn input(&self, id: &str) -> Entity<InputState> {
        self.inputs[id].clone()
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn control_focus(&self, id: &str) -> FocusHandle {
        self.focus[id].clone()
    }

    /// Add or create a repository…'s focus, the Kit button's own (B11):
    /// where the `AddButton` route lands. Always drawn.
    pub(crate) fn add_button_focus(&self) -> FocusHandle {
        self.focus[ADD_BUTTON].clone()
    }

    fn text(&self, id: &str, cx: &Context<Self>) -> String {
        self.inputs[id].read(cx).value().trim().to_owned()
    }
}

/// A piece of text a test reads by `id`.
fn text(id: impl Into<gpui_kit::ElementId>, value: impl Into<String>) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .text_sm()
        .child(value)
}

impl RepositoriesPage {
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

    fn heading(id: &'static str, label: &'static str) -> impl IntoElement {
        div()
            .id(id)
            .test_support()
            .aria_label(label)
            .text_base()
            .font_weight(gpui_kit::FontWeight::BOLD)
            .child(label)
    }
}

impl RepositoriesPage {
    /// Add or create a repository…, a Kit button drawn with the page's own
    /// handle (`wrapped_kit`), so the `AddButton` route's focus draws its
    /// ring and Enter presses it (B11, B12; G4).
    fn add_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let button = Button::new((ADD_BUTTON, 0usize))
            .label(ADD_OR_CREATE)
            .on_click(
                cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(RepositoriesEvent::AddRepository)),
            );
        wrapped_kit(
            ADD_BUTTON,
            ADD_OR_CREATE,
            &self.focus[ADD_BUTTON],
            &self.scroll,
            (ADD_BUTTON, 0usize),
            button,
        )
    }
}

impl Render for RepositoriesPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_prefill(window, cx);
        v_flex()
            .id("settings-repositories")
            .test_support()
            .gap_4()
            .child(self.managed_section(cx))
            .child(self.sharing.clone())
            .child(div().child(self.add_button(cx)))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "repositories_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "repositories_manage_tests.rs"]
mod manage_tests;

/// Settings' copy (P18); here because `manage` is private to this module.
#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "copy_tests.rs"]
mod copy_tests;
