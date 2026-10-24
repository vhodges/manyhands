//! Tab-owned state, item entities and strip rendering. Workspace's content
//! router coordinates tab actions with page visibility and region focus.
//!
//! Tabs are keyed by repository and item, with reads under their own TabId.
//! Reloads retain titles and source state until results land; a failed load
//! keeps the last title (ruling R-B5). Closing drops only that tab's state.

use super::{
    item::ItemPane, relationships::OpenRelated, scroll::ContentScroll, workspace::Workspace,
};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        stamp::{RepoKey, TabId, ViewScope},
        tabs::{TabKey, Tabs},
    },
    services::Services,
};
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
use std::collections::{BTreeSet, HashMap};

pub(crate) const SHOWN: &str = "Shown";
pub(crate) const CLOSE_TAB: &str = "Close";
pub(crate) const OPEN_TABS: &str = "Open tabs";

pub(crate) fn close_label(title: &str) -> String {
    format!("{CLOSE_TAB} {title}")
}

#[derive(Default)]
pub(crate) struct TabContent {
    tabs: Tabs,
    panes: HashMap<TabId, Entity<ItemPane>>,
    focus: HashMap<TabId, FocusHandle>,
    close_focus: HashMap<TabId, FocusHandle>,
    related: HashMap<TabId, Subscription>,
    scroll: ContentScroll,
}

impl TabContent {
    pub(super) fn shown(&self) -> Option<Entity<ItemPane>> {
        self.panes.get(&self.tabs.active()?).cloned()
    }
    pub(super) fn shown_focus(&self) -> Option<FocusHandle> {
        self.focus.get(&self.tabs.active()?).cloned()
    }
    pub(super) fn scroll(&self) -> &ContentScroll {
        &self.scroll
    }
    pub(super) fn active(&self) -> Option<TabId> {
        self.tabs.active()
    }
    pub(super) fn pane(&self, id: TabId) -> Option<&Entity<ItemPane>> {
        self.panes.get(&id)
    }

    #[cfg(test)]
    pub(super) fn keys(&self) -> Vec<TabKey> {
        self.tabs
            .tabs()
            .iter()
            .map(|(_, key)| key.clone())
            .collect()
    }

    fn forget(&mut self, id: TabId) {
        self.panes.remove(&id);
        self.focus.remove(&id);
        self.close_focus.remove(&id);
        self.related.remove(&id);
    }

    pub(super) fn open(
        &mut self,
        target: OpenTarget,
        services: &Services,
        cx: &mut Context<Workspace>,
    ) {
        let (id, opened) = self.tabs.open(TabKey::for_target(&target));
        if opened {
            let services = services.clone();
            let scroll = self.scroll.clone();
            let pane = cx.new(|cx| ItemPane::new(target, ViewScope::Tab(id), services, scroll, cx));
            let related = cx.subscribe(&pane, |this, _, event: &OpenRelated, cx| {
                this.open_item(event.0.clone(), cx)
            });
            self.related.insert(id, related);
            self.panes.insert(id, pane);
            self.focus.insert(id, cx.focus_handle().tab_stop(true));
            self.close_focus.insert(id, cx.focus_handle());
        }
    }

    pub(super) fn activate(&mut self, id: TabId) -> bool {
        self.tabs.activate(id)
    }

    pub(super) fn close(&mut self, id: TabId) -> bool {
        if self.tabs.close(id) {
            self.forget(id);
            true
        } else {
            false
        }
    }

    pub(super) fn close_repository(&mut self, root: &str) {
        for id in self.tabs.close_repository(root) {
            self.forget(id);
        }
    }

    pub(super) fn step(&mut self, step: isize) -> Option<TabId> {
        if step > 0 {
            self.tabs.next()
        } else {
            self.tabs.previous()
        }
    }

    pub(super) fn reload(&mut self, changed: &BTreeSet<RepoKey>, cx: &mut Context<Workspace>) {
        for (id, key) in self.tabs.tabs() {
            if !changed.contains(key.repo()) {
                continue;
            }
            if let Some(pane) = self.panes.get(id) {
                pane.update(cx, |pane, cx| {
                    pane.reload(cx);
                });
            }
        }
    }

    pub(super) fn render(&self, window: &Window, cx: &mut Context<Workspace>) -> Vec<AnyElement> {
        let theme = cx.theme();
        let (ring, border) = (theme.ring, theme.border);
        let shown = self.tabs.active();
        let count = self.tabs.tabs().len();
        let mut strip = h_flex()
            .id("tab-strip")
            .test_support()
            .role(Role::TabList)
            .aria_label(OPEN_TABS)
            .gap_1()
            .flex_wrap();
        for (ix, (id, _)) in self.tabs.tabs().iter().enumerate() {
            let id = *id;
            let active = shown == Some(id);
            let title = self
                .panes
                .get(&id)
                .map(|pane| pane.read(cx).tab_title())
                .unwrap_or_default();
            let focus = self.focus.get(&id).cloned();
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
                        .child(div().text_sm().child(title.clone()))
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
                tab = tab.relative().child(self.scroll.marker(focus));
            }
            strip = strip.child(
                h_flex().gap_0p5().child(tab).child(
                    div()
                        .id(("tab-close", ix))
                        .test_support()
                        .aria_label(close_label(&title))
                        .when_some_focus(self.close_focus.get(&id))
                        .child(
                            Button::new(("tab-close-button", ix))
                                .ghost()
                                .label("×")
                                .accessibility_label(close_label(&title))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.close_tab(id, window, cx);
                                })),
                        ),
                ),
            );
        }
        let mut content = Vec::new();
        if count > 0 {
            content.push(strip.into_any_element());
        }
        if let Some(pane) = self.shown() {
            content.push(pane.into_any_element());
        }
        content
    }
}

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
