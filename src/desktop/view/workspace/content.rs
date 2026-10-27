//! The single content router: the tabs, Settings or the Drafts page
//! (ruling R23). Settings and the Drafts page each own whether they are
//! shown; this file is the only one that changes either. The selector is
//! Settings when Settings is shown, else Drafts when the page is shown,
//! else the tabs, and every transition here leaves at most one shown:
//! `open_drafts` hides Settings, each Settings entry point hides the page
//! first, and `show_tabs` hides both.

use super::{ContentArea, Region, Workspace};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        stamp::{RepoKey, SettingsPage, TabId},
    },
    services::requests::DesktopRequestId,
    view::{
        close_draft::{self, CloseWithDraft},
        drafts::DraftsEvent,
        scroll::ContentScroll,
    },
};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Styled, Window,
    component::{WindowExt, v_flex},
};
use manyhands::{drafts::DraftId, repository::transport::SshAuthority};
use std::{collections::BTreeSet, path::PathBuf};

enum ContentPage {
    Tabs,
    Settings,
    Drafts,
}

impl ContentArea {
    fn page(&self, cx: &App) -> ContentPage {
        if self.settings.read(cx).is_shown() {
            ContentPage::Settings
        } else if self.drafts.read(cx).is_shown() {
            ContentPage::Drafts
        } else {
            ContentPage::Tabs
        }
    }

    fn show_tabs(&mut self, cx: &mut Context<Workspace>) {
        self.settings.update(cx, |settings, cx| settings.hide(cx));
        self.hide_drafts(cx);
    }

    /// A Settings entry point: Settings is shown in place of the page.
    fn hide_drafts(&mut self, cx: &mut Context<Workspace>) {
        self.drafts.update(cx, |drafts, cx| drafts.hide(cx));
    }
}

impl Workspace {
    #[cfg(test)]
    pub(crate) fn settings(
        &self,
    ) -> &gpui_kit::Entity<crate::desktop::view::settings::SettingsHost> {
        &self.content.settings
    }

    /// Shows the Drafts page in place of the tabs or Settings, focused on
    /// the row of `focus` when it lists it, else its first row.
    pub(in crate::desktop::view) fn open_drafts(
        &mut self,
        focus: Option<DraftId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.content
            .settings
            .update(cx, |settings, cx| settings.hide(cx));
        self.content
            .drafts
            .update(cx, |drafts, cx| drafts.show(focus, window, cx));
        self.focus_region(Region::Content, window, cx);
    }

    /// A tab's Review in Drafts: the page opens on that draft's row at
    /// the next frame, which supplies its window.
    pub(in crate::desktop::view) fn review_in_drafts(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) {
        self.content.review = Some(id);
        cx.notify();
    }

    /// A tab's restored draft was discarded from its section, which went
    /// with its Discard…: the next frame focuses the tab's content, as a
    /// tab closed with its draft does.
    pub(in crate::desktop::view) fn draft_discarded(&mut self, cx: &mut Context<Self>) {
        self.content.refocus = true;
        cx.notify();
    }

    /// Opens the Drafts page a tab's Review in Drafts asked for, and
    /// focuses a tab whose draft was discarded. While a
    /// dialog is open the request waits, as D1's actions do not act behind
    /// one; the frame after the dialog closes opens it.
    pub(super) fn apply_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        if std::mem::take(&mut self.content.refocus) {
            self.focus_region(Region::Content, window, cx);
        }
        if let Some(id) = self.content.review.take() {
            self.open_drafts(Some(id), window, cx);
        }
    }

    /// What the Drafts page asks: Show shows and focuses the tab its draft
    /// is restored in; a Restore opens and focuses its item's tab, which
    /// leaves the page.
    pub(super) fn drafts_event(
        &mut self,
        event: &DraftsEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            DraftsEvent::Show(key) => {
                if let Some(id) = self.content.tabs.find(key) {
                    self.show_tab(id, cx);
                    self.focus_region(Region::Content, window, cx);
                }
            }
            DraftsEvent::Open(target) => {
                self.open_item(target.clone(), cx);
                self.focus_region(Region::Content, window, cx);
            }
        }
    }

    pub(in crate::desktop::view) fn open_settings(
        &mut self,
        page: SettingsPage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.content.hide_drafts(cx);
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
        self.content.hide_drafts(cx);
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
        self.content.hide_drafts(cx);
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
        self.content.hide_drafts(cx);
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
        self.content.hide_drafts(cx);
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

    /// Shows the tab `id`, and so the tabs.
    pub(in crate::desktop::view) fn show_tab(&mut self, id: TabId, cx: &mut Context<Self>) {
        if self.content.tabs.activate(id) {
            self.content.show_tabs(cx);
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
        if matches!(
            self.content.page(cx),
            ContentPage::Settings | ContentPage::Drafts
        ) {
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
        let region = || self.regions[Region::Content.index()].clone();
        let focus = match self.content.page(cx) {
            ContentPage::Settings => match self.content.settings.read(cx).first_focus() {
                Some(focus) => focus,
                None => return,
            },
            ContentPage::Drafts => self
                .content
                .drafts
                .read(cx)
                .first_focus()
                .unwrap_or_else(region),
            ContentPage::Tabs => self.content.tabs.shown_focus().unwrap_or_else(region),
        };
        window.focus(&focus, cx);
    }

    /// The content region's column: Settings, the Drafts page and the
    /// empty pane scroll it, as in D1.
    pub(super) fn content_scroll(&self) -> &ContentScroll {
        self.content.tabs.scroll()
    }

    /// What the page keys move: while an item tab is shown, its own scroll
    /// region below the strip and the header (R-47); else the column.
    pub(super) fn scroll_target(&self, cx: &App) -> ContentScroll {
        let shown = match self.content.page(cx) {
            ContentPage::Tabs => self.content.tabs.shown(),
            ContentPage::Settings | ContentPage::Drafts => None,
        };
        match shown {
            Some(pane) => pane.read(cx).region().clone(),
            None => self.content_scroll().clone(),
        }
    }

    /// Keep selection construction and secret clearing in their existing frame
    /// order; leaving Settings does not replace its cached pages or scroll.
    /// Leaving Keys for the tabs or the Drafts page empties a typed
    /// passphrase alike.
    pub(super) fn content_body(
        &self,
        selection: AnyElement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        match self.content.page(cx) {
            ContentPage::Settings => vec![self.content.settings.clone().into_any_element()],
            ContentPage::Drafts => {
                self.content
                    .settings
                    .update(cx, |settings, cx| settings.forget_passphrase(window, cx));
                vec![self.content.drafts.clone().into_any_element()]
            }
            ContentPage::Tabs => {
                self.content
                    .settings
                    .update(cx, |settings, cx| settings.forget_passphrase(window, cx));
                let tabs = self.content.tabs.render(
                    window,
                    self.focused_region(window, cx) == Some(Region::Content),
                    cx,
                );
                if self.content.tabs.shown().is_none() {
                    // The empty pane keeps D1's single scrolling column.
                    let mut body = tabs;
                    body.push(selection);
                    return body;
                }
                // An item tab fills the column: the strip, then the pane,
                // whose header stays above its own scroll region. The
                // selection stays drawn below it.
                vec![
                    v_flex()
                        .id("item-tab")
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .children(tabs)
                        .into_any_element(),
                    selection,
                ]
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn drafts_page(
        &self,
    ) -> &gpui_kit::Entity<crate::desktop::view::drafts::DraftsPage> {
        &self.content.drafts
    }

    #[cfg(test)]
    pub(crate) fn drafts_shown(&self, cx: &App) -> bool {
        matches!(self.content.page(cx), ContentPage::Drafts)
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
    /// The shown item's scroll region's offset.
    #[cfg(test)]
    pub(crate) fn item_offset(&self, cx: &App) -> gpui_kit::Pixels {
        self.content
            .tabs
            .shown()
            .map(|pane| pane.read(cx).region_offset())
            .unwrap_or_default()
    }
}
