//! The Repositories settings page (design, Settings / Repositories): the
//! registered repositories, with a chosen one's identity and Remove
//! (`manage`), then its remotes under Sharing (the Remotes page's view,
//! Wave 04 Cycle 04 design C1), then Add an existing repository and Create
//! a repository: the Add repository forms, an entity of their own
//! (`view::add_repository`, Wave 04 Cycle 05a design B4), drawn here, in
//! the page's scroll, until their own sheet holds them.
//!
//! Every control sits in a focus-tracking wrapper with a scroll marker,
//! so a control that takes focus below the fold is brought into view.
//! Text is fixed strings, the library's own values, and
//! `ResultCode::message()` with its code.

use std::{collections::HashMap, path::PathBuf};

use gpui_kit::{
    AnyView, App, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Subscription,
    TestSupportExt, Window,
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

use super::{OpenFlag, field_column, wrapped};
use crate::desktop::{
    services::Services,
    view::{add_repository::AddRepositorySheet, scroll::ContentScroll},
};

pub(crate) const NAME: &str = "Name";
pub(crate) const EMAIL: &str = "Email";

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
    /// The Add repository forms, drawn after Sharing (design B4).
    add: Entity<AddRepositorySheet>,
    /// The Add forms' inputs and control focus, which `input` and
    /// `control_focus` give the tests that read the forms through the page
    /// (Ruling 9; Task 4 removes it).
    #[cfg(all(test, feature = "desktop-test-support"))]
    add_controls: crate::desktop::view::add_repository::Controls,
    _inputs: Vec<Subscription>,
    _operations: Subscription,
}

impl RepositoriesPage {
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        open: OpenFlag,
        sharing: AnyView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let add = {
            let (services, scroll) = (services.clone(), scroll.clone());
            cx.new(|cx| AddRepositorySheet::new(services, scroll, open, window, cx))
        };
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
        for id in manage::FIELDS.into_iter().chain(manage::CONTROLS) {
            focus.insert(id, cx.focus_handle());
        }
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        });
        let managed = manage::Managed::new(&services, cx);
        #[cfg(all(test, feature = "desktop-test-support"))]
        let add_controls = add.read(cx).controls();
        // The list is read when the page is shown (`opened`).
        Self {
            services,
            scroll,
            inputs,
            focus,
            managed,
            sharing,
            add,
            #[cfg(all(test, feature = "desktop-test-support"))]
            add_controls,
            _inputs: subscriptions,
            _operations: operations,
        }
    }

    /// The input `id`: the page's, or the Add forms' (Ruling 9).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn input(&self, id: &str) -> Entity<InputState> {
        self.inputs
            .get(id)
            .or_else(|| self.add_controls.inputs.get(id))
            .unwrap_or_else(|| panic!("no input {id}"))
            .clone()
    }

    /// The focus the wrapper `id` tracks: the page's, or the Add forms'
    /// (Ruling 9).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn control_focus(&self, id: &str) -> FocusHandle {
        self.focus
            .get(id)
            .or_else(|| self.add_controls.focus.get(id))
            .unwrap_or_else(|| panic!("no control {id}"))
            .clone()
    }

    /// The Add repository forms the page draws.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn add_sheet(&self) -> Entity<AddRepositorySheet> {
        self.add.clone()
    }

    fn text(&self, id: &str, cx: &Context<Self>) -> String {
        self.inputs[id].read(cx).value().trim().to_owned()
    }
}

/// What Settings' routes and its close still ask of the Add forms through
/// the page that draws them (Ruling 9). Task 4 removes them, once the Add
/// repository sheet holds the forms.
impl RepositoriesPage {
    /// The Add folder field's focus, where the Add and Inspect routes land
    /// (design Part D; R-24): always drawn.
    pub(crate) fn add_path_focus(&self, cx: &App) -> FocusHandle {
        self.add.read(cx).add_path_focus(cx)
    }

    /// The `repo.inspect` recovery: `root` in the Add folder, checked.
    pub(crate) fn inspect_root(&self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.add
            .update(cx, |add, cx| add.inspect_root(root, window, cx));
    }

    /// Settings closed: an Add waiting for its folder's check does not go on
    /// when the check lands (R-48).
    pub(crate) fn forget_pending_submit(&self, cx: &mut Context<Self>) {
        self.add.update(cx, |add, _| add.forget_pending_submit());
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

impl Render for RepositoriesPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_prefill(window, cx);
        v_flex()
            .id("settings-repositories")
            .test_support()
            .gap_4()
            .child(self.managed_section(cx))
            .child(self.sharing.clone())
            .child(self.add.clone())
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
