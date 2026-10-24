//! The single tabs/Settings router. Settings remains the visibility authority.

use super::{ContentArea, Region, Workspace};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        stamp::{RepoKey, SettingsPage, TabId},
    },
    services::requests::DesktopRequestId,
    view::{
        close_draft::{self, CloseWithDraft},
        scroll::ContentScroll,
    },
};
use gpui_kit::{AnyElement, App, Context, IntoElement, Window, component::WindowExt};
use manyhands::{drafts::DraftId, repository::transport::SshAuthority};
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

    /// Closes the tab `id`. A tab with a restored draft attached asks
    /// first: Keep draft, Discard or Cancel. True when the tab closed now.
    /// While quitting waits for drafts or a job, with Quit now in view,
    /// nothing closes and nothing opens in front of it.
    pub(in crate::desktop::view) fn close_tab(
        &mut self,
        id: TabId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.services.worker().quitting() || self.services.operations().read(cx).saving_drafts()
        {
            return false;
        }
        let attached = self.content.tabs.pane(id).and_then(|pane| {
            let pane = pane.read(cx);
            let draft = self.services.drafts().read(cx).attached(&pane.tab_key())?;
            Some((draft, pane.tab_title()))
        });
        let Some((draft, title)) = attached else {
            let closed = self.content.tabs.close(id);
            if closed {
                cx.notify();
            }
            return closed;
        };
        if window.has_active_dialog(cx) {
            return false;
        }
        let workspace = cx.entity().downgrade();
        close_draft::open(title, window, cx, move |choice, window, cx| {
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.close_with_draft(id, draft, choice, window, cx)
            });
        });
        false
    }

    /// What the close dialog chose for the tab `id` and its draft. Keep
    /// closes the tab once the draft is written and let go of; a failed
    /// write leaves the tab open with the failure. Discard closes it once
    /// the draft is deleted; a failed discard leaves it open and says so,
    /// the draft let go of and listed again. A tab closed here gives the
    /// focus to the content region, as `secondary-w` does.
    fn close_with_draft(
        &mut self,
        id: TabId,
        draft: DraftId,
        choice: CloseWithDraft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drafts = self.services.drafts().clone();
        let closing = match choice {
            CloseWithDraft::Cancel => return,
            CloseWithDraft::KeepDraft => {
                drafts.update(cx, |registry, cx| registry.release(draft, cx))
            }
            CloseWithDraft::Discard => {
                drafts.update(cx, |registry, cx| registry.discard(draft, cx))
            }
        };
        cx.spawn_in(window, async move |workspace, cx| {
            let closed = closing.await;
            let _ = workspace.update_in(cx, |workspace, window, cx| match closed {
                Ok(()) => {
                    if workspace.content.tabs.close(id) {
                        workspace.focus_region(Region::Content, window, cx);
                        cx.notify();
                    }
                }
                Err(kind) if choice == CloseWithDraft::Discard => {
                    if let Some(pane) = workspace.content.tabs.pane(id).cloned() {
                        pane.update(cx, |pane, cx| pane.discard_failed(kind, cx));
                    }
                }
                // The session shows the failed write.
                Err(_) => {}
            });
        })
        .detach();
    }

    /// Closes every tab of `root`. Their restored drafts are written and
    /// let go of first, kept in Drafts, never discarded.
    pub(crate) fn close_repository(&mut self, root: &str, cx: &mut Context<Self>) {
        self.services
            .drafts()
            .update(cx, |registry, cx| registry.release_repository(root, cx));
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
        // A dialog that asks first keeps the focus.
        if self.close_tab(id, window, cx) {
            self.focus_region(Region::Content, window, cx);
        }
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
