//! The single tabs/Settings router. Settings remains the visibility authority.

use super::{ContentArea, Region, Workspace};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        stamp::{RepoKey, SettingsPage, TabId},
    },
    services::requests::DesktopRequestId,
    view::scroll::ContentScroll,
};
use gpui_kit::{AnyElement, App, Context, IntoElement, Window};
use manyhands::repository::transport::SshAuthority;
use std::{collections::BTreeSet, path::PathBuf};

enum ContentPage {
    Tabs,
    Settings,
}

impl ContentArea {
    fn page(&self, cx: &App) -> ContentPage {
        if self.settings.read(cx).is_shown() {
            ContentPage::Settings
        } else {
            ContentPage::Tabs
        }
    }

    fn show_tabs(&mut self, cx: &mut Context<Workspace>) {
        self.settings.update(cx, |settings, cx| settings.hide(cx));
    }
}

impl Workspace {
    #[cfg(test)]
    pub(crate) fn settings(
        &self,
    ) -> &gpui_kit::Entity<crate::desktop::view::settings::SettingsHost> {
        &self.content.settings
    }

    pub(in crate::desktop::view) fn open_settings(
        &mut self,
        page: SettingsPage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selection = self.navigation.selected().map(PathBuf::from);
        self.content.settings.update(cx, |settings, cx| {
            settings.set_selection(selection);
            settings.open(page, window, cx)
        });
        self.focus_region(Region::Content, window, cx);
    }

    pub(crate) fn inspect_in_settings(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.content
            .settings
            .update(cx, |settings, cx| settings.inspect(root, window, cx));
        self.focus_region(Region::Content, window, cx);
    }

    pub(crate) fn identity_in_settings(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.last_region = Some(Region::Content);
        self.content
            .settings
            .update(cx, |settings, cx| settings.identity(root, window, cx));
    }

    pub(crate) fn check_host_in_settings(
        &mut self,
        root: PathBuf,
        authority: SshAuthority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.last_region = Some(Region::Content);
        self.content.settings.update(cx, |settings, cx| {
            settings.check_host(root, authority, window, cx)
        });
    }

    pub(super) fn retry_generation_in_settings(
        &mut self,
        id: DesktopRequestId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.last_region = Some(Region::Content);
        self.content
            .settings
            .update(cx, |settings, cx| settings.retry_generation(id, window, cx));
    }

    pub(crate) fn open_item(&mut self, target: OpenTarget, cx: &mut Context<Self>) {
        self.content.show_tabs(cx);
        self.content.tabs.open(target, &self.services, cx);
        cx.notify();
    }

    pub(in crate::desktop::view) fn show_tab(&mut self, id: TabId, cx: &mut Context<Self>) {
        if self.content.tabs.activate(id) {
            cx.notify();
        }
    }

    pub(in crate::desktop::view) fn close_tab(&mut self, id: TabId, cx: &mut Context<Self>) {
        if self.content.tabs.close(id) {
            cx.notify();
        }
    }

    pub(crate) fn close_repository(&mut self, root: &str, cx: &mut Context<Self>) {
        self.content.tabs.close_repository(root);
        cx.notify();
    }

    pub(super) fn step_tab(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        self.content.show_tabs(cx);
        if self.content.tabs.step(step).is_some() {
            self.focus_region(Region::Content, window, cx);
        }
    }

    pub(super) fn close_shown_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.content.page(cx), ContentPage::Settings) {
            return;
        }
        let Some(id) = self.content.tabs.active() else {
            return;
        };
        self.close_tab(id, cx);
        self.focus_region(Region::Content, window, cx);
    }

    pub(super) fn reload_tabs(&mut self, changed: &BTreeSet<RepoKey>, cx: &mut Context<Self>) {
        self.content.tabs.reload(changed, cx);
    }

    pub(super) fn focus_content(&self, window: &mut Window, cx: &mut Context<Self>) {
        match self.content.page(cx) {
            ContentPage::Settings => {
                if let Some(focus) = self.content.settings.read(cx).first_focus() {
                    window.focus(&focus, cx);
                }
            }
            ContentPage::Tabs => {
                let focus = self
                    .content
                    .tabs
                    .shown_focus()
                    .unwrap_or_else(|| self.regions[Region::Content.index()].clone());
                window.focus(&focus, cx);
            }
        }
    }

    pub(super) fn content_scroll(&self) -> &ContentScroll {
        self.content.tabs.scroll()
    }

    /// Keep selection construction and secret clearing in their existing frame
    /// order; leaving Settings does not replace its cached pages or scroll.
    pub(super) fn content_body(
        &self,
        selection: AnyElement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        match self.content.page(cx) {
            ContentPage::Settings => vec![self.content.settings.clone().into_any_element()],
            ContentPage::Tabs => {
                self.content
                    .settings
                    .update(cx, |settings, cx| settings.forget_passphrase(window, cx));
                let mut body = self.content.tabs.render(window, cx);
                body.push(selection);
                body
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn item_pane(
        &self,
    ) -> Option<gpui_kit::Entity<crate::desktop::view::item::ItemPane>> {
        self.content.tabs.shown()
    }
    #[cfg(test)]
    pub(crate) fn opened(&self, cx: &App) -> Option<OpenTarget> {
        self.content
            .tabs
            .shown()
            .map(|item| item.read(cx).target().clone())
    }
    #[cfg(test)]
    pub(crate) fn tab_keys(&self) -> Vec<crate::desktop::model::tabs::TabKey> {
        self.content.tabs.keys()
    }
    #[cfg(test)]
    pub(crate) fn settings_shown(&self, cx: &App) -> crate::desktop::view::settings::Shown {
        self.content.settings.read(cx).shown()
    }
    #[cfg(test)]
    pub(crate) fn content_offset(&self) -> gpui_kit::Pixels {
        self.content_scroll().handle().offset().y
    }
}
