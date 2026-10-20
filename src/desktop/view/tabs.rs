//! The content region: a strip of tabs and the shown tab's item.
//!
//! Each tab is keyed by its repository and item (`TabKey`), and owns an
//! `ItemPane` whose reads are stamped under its own `ViewScope::Tab`, so a
//! read that lands after the user moved to another tab only ever changes
//! the tab that asked. Opening an item that is already open shows its tab.
//!
//! When a finished request may have changed a repository's items, every
//! open tab of it reads its item again in place: the strip keeps its order
//! and the shown tab, and each tab keeps what it shows, its title and its
//! source's text field, until the new read lands. One that stays busy
//! keeps them and says it could not refresh. A tab whose repository has
//! gone shows the failure with Retry, as a first load does, and keeps its
//! title (ruling R-B5); the other tabs are not touched.
//!
//! `ctrl-tab` and `ctrl-shift-tab` move through the tabs, `secondary-w`
//! closes the shown one; focus follows to the shown tab. The shown tab
//! says so in text, not by color alone.

use std::collections::{BTreeSet, HashMap};

use gpui_kit::{
    AnyElement, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled, Subscription,
    TestSupportExt, Window,
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        h_flex,
    },
    div,
};

use super::{
    item::ItemPane, relationships::OpenRelated, scroll::ContentScroll, workspace::Workspace,
};
use crate::desktop::model::{
    navigation::OpenTarget,
    stamp::{RepoKey, TabId, ViewScope},
    tabs::{TabKey, Tabs},
};

/// The shown tab's marker.
pub(crate) const SHOWN: &str = "Shown";
pub(crate) const CLOSE_TAB: &str = "Close tab";

/// The open tabs, each tab's item view and the focus of its strip entry.
#[derive(Default)]
pub(crate) struct Content {
    tabs: Tabs,
    panes: HashMap<TabId, Entity<ItemPane>>,
    focus: HashMap<TabId, FocusHandle>,
    /// Each tab's subscription to its relationship rows.
    related: HashMap<TabId, Subscription>,
    /// The region's scrolling, shared with each tab's item view.
    pub(super) scroll: ContentScroll,
}

impl Content {
    fn shown(&self) -> Option<Entity<ItemPane>> {
        self.panes.get(&self.tabs.active()?).cloned()
    }

    /// The focus of the shown tab's strip entry.
    pub(super) fn shown_focus(&self) -> Option<FocusHandle> {
        self.focus.get(&self.tabs.active()?).cloned()
    }

    fn forget(&mut self, id: TabId) {
        self.panes.remove(&id);
        self.focus.remove(&id);
        self.related.remove(&id);
    }
}

impl Workspace {
    /// The shown tab's item view.
    #[cfg(test)]
    pub(crate) fn item_pane(&self) -> Option<Entity<ItemPane>> {
        self.content.shown()
    }

    /// What the shown tab shows.
    #[cfg(test)]
    pub(crate) fn opened(&self, cx: &gpui_kit::App) -> Option<OpenTarget> {
        self.content
            .shown()
            .map(|item| item.read(cx).target().clone())
    }

    /// The open tabs' keys, in strip order.
    #[cfg(test)]
    pub(crate) fn tab_keys(&self) -> Vec<TabKey> {
        self.content
            .tabs
            .tabs()
            .iter()
            .map(|(_, key)| key.clone())
            .collect()
    }

    /// Shows `target`'s tab, opening it and reading its item when it is
    /// not open yet, in place of the settings if they are shown. Selection
    /// does not change.
    pub(crate) fn open_item(&mut self, target: OpenTarget, cx: &mut Context<Self>) {
        self.settings.update(cx, |settings, cx| settings.hide(cx));
        let (id, opened) = self.content.tabs.open(TabKey::for_target(&target));
        if opened {
            let services = self.services.clone();
            let scroll = self.content.scroll.clone();
            let pane = cx.new(|cx| ItemPane::new(target, ViewScope::Tab(id), services, scroll, cx));
            // A relationship row opens its ticket in a tab of its own.
            let related = cx.subscribe(&pane, |this, _, event: &OpenRelated, cx| {
                this.open_item(event.0.clone(), cx)
            });
            self.content.related.insert(id, related);
            self.content.panes.insert(id, pane);
            self.content.focus.insert(id, cx.focus_handle());
        }
        cx.notify();
    }

    /// Shows the tab `id`.
    fn show_tab(&mut self, id: TabId, cx: &mut Context<Self>) {
        if self.content.tabs.activate(id) {
            cx.notify();
        }
    }

    /// Closes the tab `id`; the tab taking its place is shown.
    pub(super) fn close_tab(&mut self, id: TabId, cx: &mut Context<Self>) {
        if self.content.tabs.close(id) {
            self.content.forget(id);
            cx.notify();
        }
    }

    /// Closes every tab of `root`, as the removal of its registration
    /// does, so a later registration there inherits none of them.
    pub(crate) fn close_repository(&mut self, root: &str, cx: &mut Context<Self>) {
        for id in self.content.tabs.close_repository(root) {
            self.content.forget(id);
        }
        cx.notify();
    }

    /// `ctrl-tab` (`step` 1) or `ctrl-shift-tab` (-1): shows the next or
    /// previous tab, and focuses it. With the settings shown, the tabs are
    /// shown again first.
    pub(super) fn step_tab(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.update(cx, |settings, cx| settings.hide(cx));
        let moved = if step > 0 {
            self.content.tabs.next()
        } else {
            self.content.tabs.previous()
        };
        if moved.is_some() {
            self.focus_shown_tab(window, cx);
        }
    }

    /// `secondary-w`: closes the shown tab and focuses the one shown in
    /// its place, or the content region when none is left. With the
    /// settings shown, no tab is shown to close.
    pub(super) fn close_shown_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.read(cx).is_shown() {
            return;
        }
        let Some(id) = self.content.tabs.active() else {
            return;
        };
        self.close_tab(id, cx);
        self.focus_shown_tab(window, cx);
    }

    fn focus_shown_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_region(super::workspace::Region::Content, window, cx);
    }

    /// Every open tab of a repository in `changed` reads its item again,
    /// in place: one load per tab, however many requests changed it.
    pub(super) fn reload_tabs(&mut self, changed: &BTreeSet<RepoKey>, cx: &mut Context<Self>) {
        for (id, key) in self.content.tabs.tabs() {
            if !changed.contains(key.repo()) {
                continue;
            }
            if let Some(pane) = self.content.panes.get(id) {
                pane.update(cx, |pane, cx| {
                    pane.reload(cx);
                });
            }
        }
    }

    /// The tab strip, in order, then the shown tab's item.
    pub(super) fn content(&self, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme();
        let (ring, border) = (theme.ring, theme.border);
        let shown = self.content.tabs.active();
        let count = self.content.tabs.tabs().len();
        let mut strip = h_flex().id("tab-strip").gap_1().flex_wrap();
        for (ix, (id, _)) in self.content.tabs.tabs().iter().enumerate() {
            let id = *id;
            let active = shown == Some(id);
            let title = self
                .content
                .panes
                .get(&id)
                .map(|pane| pane.read(cx).tab_title())
                .unwrap_or_default();
            let focus = self.content.focus.get(&id).cloned();
            let focused = focus.as_ref().is_some_and(|focus| focus.is_focused(window));
            let mut tab = div()
                .id(("tab", ix))
                .test_support()
                .role(Role::Tab)
                .aria_label(title.clone())
                .aria_selected(active)
                .px_2()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(border)
                .when_some_focus(focus.as_ref())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_tab(id, cx)))
                .child(
                    h_flex()
                        .gap_1()
                        .child(div().text_sm().child(title))
                        .children(active.then(|| {
                            div()
                                .id(("tab-shown", ix))
                                .test_support()
                                .aria_label(SHOWN)
                                .text_xs()
                                .child(SHOWN)
                        })),
                );
            if focused {
                tab = tab.border_2().border_color(ring);
            }
            if let Some(focus) = &focus {
                tab = tab.relative().child(self.content.scroll.marker(focus));
            }
            strip = strip.child(
                h_flex().gap_0p5().child(tab).child(
                    div()
                        .id(("tab-close", ix))
                        .test_support()
                        .aria_label(CLOSE_TAB)
                        .child(
                            Button::new(("tab-close-button", ix))
                                .ghost()
                                .label("×")
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.close_tab(id, cx)
                                })),
                        ),
                ),
            );
        }
        let mut content = Vec::new();
        if count > 0 {
            content.push(strip.into_any_element());
        }
        if let Some(pane) = self.content.shown() {
            content.push(pane.into_any_element());
        }
        content
    }
}

/// Tracks `focus` when there is one.
trait FocusSome {
    fn when_some_focus(self, focus: Option<&FocusHandle>) -> Self;
}

impl<E: InteractiveElement> FocusSome for E {
    fn when_some_focus(self, focus: Option<&FocusHandle>) -> Self {
        match focus {
            Some(focus) => self.track_focus(focus),
            None => self,
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "tabs_tests.rs"]
mod tests;
