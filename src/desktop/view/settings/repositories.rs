//! The Repositories settings page.

use gpui_kit::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestSupportExt,
    Window, component::v_flex, div,
};

use crate::desktop::{services::Services, view::scroll::ContentScroll};

pub(crate) struct RepositoriesPage {
    _services: Services,
    _scroll: ContentScroll,
}

impl RepositoriesPage {
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            _services: services,
            _scroll: scroll,
        }
    }
}

impl Render for RepositoriesPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("settings-repositories")
            .test_support()
            .gap_3()
            .child(div())
    }
}
