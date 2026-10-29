//! Tab-owned state, item entities and strip rendering. Workspace's content
//! router coordinates tab actions with page visibility and region focus.
//!
//! Tabs are keyed by repository and item, with reads under their own TabId.
//! Reloads retain titles and source state until results land; a failed load
//! keeps the last title (ruling R-B5). Closing drops only that tab's state.
//!
//! The strip is the app's own (R-50): the Kit's `Tab` overrides the active
//! style. The shown tab is marked by the `tab-active-line` shape, never by
//! colour alone.

use super::{
    item::text,
    item::{
        CloseThisTab, DraftDiscarded, FollowItem, InitialsInSettings, ItemPane, OpenDetails,
        ReviewInDrafts,
        tokens::{ItemColors, item_colors},
    },
    relationships::OpenRelated,
    scroll::ContentScroll,
    workspace::Workspace,
};
use crate::desktop::{
    assets::icon_path,
    model::{
        navigation::OpenTarget,
        stamp::{RepoKey, TabId, ViewScope},
        tabs::{TabKey, Tabs},
    },
    services::Services,
};
use futures::{channel::oneshot, future::Shared};
use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, Entity, FocusHandle, Hsla,
    InteractiveElement, IntoElement, ParentElement, Role, ScrollHandle, StatefulInteractiveElement,
    Styled, Subscription, TestSupportExt, Window,
    component::{
        ActiveTheme, Icon,
        button::{Button, ButtonVariants},
        h_flex,
        tooltip::Tooltip,
    },
    div, px,
};
use manyhands::repository::{ItemContextKind, ItemDtoKind};
use std::{
    cell::Cell,
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
};

pub(crate) const SHOWN: &str = "Shown";
pub(crate) const CLOSE_TAB: &str = "Close";
pub(crate) const OPEN_TABS: &str = "Open tabs";
pub(crate) const UNSAVED_TOOLTIP: &str = "Unsaved changes";

const STRIP_HEIGHT: f32 = 34.;
const MAX_TAB_WIDTH: f32 = 260.;
const TAB_GAP: f32 = 7.;
const ICON_SIZE: f32 = 13.;
const DOT_SIZE: f32 = 7.;
const CODE_SIZE: f32 = 11.;
const TITLE_SIZE: f32 = 13.;
const ACTIVE_LINE: f32 = 2.;
const CLOSE_SIZE: f32 = 20.;
const CLOSE_ICON: f32 = 12.;

pub(crate) fn close_label(title: &str) -> String {
    format!("{CLOSE_TAB} {title}")
}

/// What a tab's item is, for its icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum TabKind {
    #[default]
    Ticket,
    Document,
}

impl TabKind {
    fn icon(self) -> &'static str {
        match self {
            Self::Ticket => "ticket",
            Self::Document => "file-text",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Ticket => "Ticket",
            Self::Document => "Document",
        }
    }
}

/// What the strip draws for one tab. It holds a title, so it has no
/// `Debug`.
#[derive(Clone, Default)]
pub(super) struct TabInfo {
    pub kind: TabKind,
    /// A ticket's short code, when it has one.
    pub code: Option<String>,
    pub title: String,
    /// Unsaved work, or a kept or stored draft (E-29).
    pub unsaved: bool,
    pub tooltip: String,
}

impl TabInfo {
    /// The accessible name: the title, plus ", unsaved" (E-29).
    fn name(&self) -> String {
        if self.unsaved {
            format!("{}, unsaved", self.title)
        } else {
            self.title.clone()
        }
    }
}

/// "Title · repository", and for an item with a work context also its
/// branch (when it has one) and worktree.
pub(super) fn tooltip_text(
    title: &str,
    repository: &str,
    context: Option<(Option<&str>, &str)>,
) -> String {
    let mut text = format!("{title} · {repository}");
    if let Some((branch, worktree)) = context {
        if let Some(branch) = branch {
            text.push_str(&format!(" · branch {branch}"));
        }
        text.push_str(&format!(" · worktree {worktree}"));
    }
    text
}

/// The active tab's top line: amber while the content region has focus,
/// `faint` otherwise, with the name tests read from the line's test-only state element.
pub(super) fn active_line(content_focused: bool, colors: &ItemColors) -> (&'static str, Hsla) {
    if content_focused {
        ("amber", colors.accent)
    } else {
        ("faint", colors.faint)
    }
}

/// The facts of `pane`'s tab.
pub(super) fn tab_info(pane: &ItemPane, cx: &App) -> TabInfo {
    // A new item not created yet is titled by its draft.
    let title = pane.new_item_title(cx).unwrap_or_else(|| pane.tab_title());
    let (repo, kind) = match pane.model.target() {
        OpenTarget::Item { repo, kind, .. } => (repo.as_str(), *kind),
        OpenTarget::Inspect { repo, .. } => (repo.as_str(), ItemDtoKind::Document),
    };
    let item = pane.model.item().rows();
    let code = match kind {
        ItemDtoKind::Ticket => item.and_then(|item| item.slug.clone()),
        _ => None,
    };
    let registry = pane.drafts().read(cx);
    let key = pane.tab_key();
    // Edited repair text is unsaved too.
    let unsaved = pane.repair_edited(cx)
        || match &key {
            TabKey::Item { repo, kind, id } => {
                pane.unsaved_edit(cx)
                    || registry.attached(&key).is_some()
                    || registry.stored_for(repo, *kind, id).is_some()
            }
            TabKey::Inspect { .. } => false,
        };
    let repository = Path::new(repo)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| repo.to_owned());
    let context = item
        .filter(|item| item.context.kind == ItemContextKind::Active)
        .map(|item| {
            (
                item.context.branch.as_deref(),
                item.context.worktree.as_str(),
            )
        });
    let tooltip = tooltip_text(&title, &repository, context);
    TabInfo {
        kind: match kind {
            ItemDtoKind::Ticket => TabKind::Ticket,
            _ => TabKind::Document,
        },
        code,
        title,
        unsaved,
        tooltip,
    }
}

/// Which tabs of a repository stay open as it is closed, of those whose
/// edit session holds unsaved text that no draft holds (Part C fixes 1 and
/// 2).
pub(super) enum Keeping<'a> {
    /// Every one: no removal was confirmed; the repository left the list.
    Unkept,
    /// Those the confirmed removal did not count: they became unkept after
    /// its Confirm.
    Uncounted(&'a HashSet<TabKey>),
}

#[derive(Default)]
pub(crate) struct TabContent {
    tabs: Tabs,
    panes: HashMap<TabId, Entity<ItemPane>>,
    focus: HashMap<TabId, FocusHandle>,
    close_focus: HashMap<TabId, FocusHandle>,
    /// Each pane's related item opened, its Review in Drafts, its restored
    /// draft discarded, its chips' requests for the details sheet, its
    /// create's ask for Settings on its repository, its
    /// own close once its unsaved work was settled, and its repair's item
    /// to follow.
    related: HashMap<TabId, [Subscription; 7]>,
    scroll: ContentScroll,
    /// The strip's own scroll handle, and the tabs the last frame saw shown
    /// and focused, so a change is scrolled into view once.
    strip: ScrollHandle,
    seen: Cell<(Option<TabId>, Option<TabId>)>,
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

    /// The open tab of `key`.
    pub(super) fn find(&self, key: &TabKey) -> Option<TabId> {
        self.tabs
            .tabs()
            .iter()
            .find(|(_, open)| open == key)
            .map(|(id, _)| *id)
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
            self.attach(id, target, services, cx);
            self.focus.insert(id, cx.focus_handle().tab_stop(true));
            self.close_focus.insert(id, cx.focus_handle());
        }
    }

    /// The tab `id` follows its item to `target`, once a repair has made it
    /// (`FollowItem`): it shows `target` from now on, in its place in the
    /// strip, with its own focus, in a pane that reads it there. When a tab
    /// is open on `target` already, that one is shown and `id` closes. Gives
    /// the tab that shows `target`.
    pub(super) fn follow(
        &mut self,
        id: TabId,
        target: OpenTarget,
        services: &Services,
        cx: &mut Context<Workspace>,
    ) -> TabId {
        let key = TabKey::for_target(&target);
        if let Some(open) = self.find(&key) {
            if open != id {
                self.close(id);
                self.tabs.activate(open);
            }
            return open;
        }
        if self.tabs.replace(id, key) {
            self.panes.remove(&id);
            self.related.remove(&id);
            self.attach(id, target, services, cx);
        }
        id
    }

    /// A pane for the tab `id`, reading `target`, and what the workspace
    /// hears from it.
    fn attach(
        &mut self,
        id: TabId,
        target: OpenTarget,
        services: &Services,
        cx: &mut Context<Workspace>,
    ) {
        let services = services.clone();
        let scroll = self.scroll.clone();
        let pane = cx.new(|cx| ItemPane::new(target, ViewScope::Tab(id), services, scroll, cx));
        let related = cx.subscribe(&pane, |this, _, event: &OpenRelated, cx| {
            this.open_item(event.0.clone(), cx)
        });
        let review = cx.subscribe(&pane, |this, _, event: &ReviewInDrafts, cx| {
            this.review_in_drafts(event.0, cx)
        });
        let discarded = cx.subscribe(&pane, |this, _, _: &DraftDiscarded, cx| {
            this.draft_discarded(cx)
        });
        let details = cx.subscribe(&pane, |this, pane, event: &OpenDetails, cx| {
            this.ask_details(&pane, event.0, cx)
        });
        let settings = cx.subscribe(&pane, |this, _, event: &InitialsInSettings, cx| {
            this.settings_for_initials(event.0.clone(), cx)
        });
        let close = cx.subscribe(&pane, move |this, _, _: &CloseThisTab, cx| {
            this.close_settled(id, cx)
        });
        let follow = cx.subscribe(&pane, move |this, _, event: &FollowItem, cx| {
            this.follow_tab(id, event.0.clone(), cx)
        });
        self.related.insert(
            id,
            [related, review, discarded, details, settings, close, follow],
        );
        self.panes.insert(id, pane);
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

    /// Closes every tab of `root` but those `keeping` keeps: a tab whose
    /// edit session holds unsaved text that no draft holds may stay open,
    /// for its own close to ask (Part C fixes 1 and 2).
    pub(super) fn close_repository(&mut self, root: &str, keeping: Keeping<'_>, cx: &App) {
        let kept: HashSet<TabId> = self
            .tabs
            .tabs()
            .iter()
            .filter(|(_, key)| key.repo() == root)
            .filter(|(id, key)| {
                let unkept = self
                    .panes
                    .get(id)
                    .is_some_and(|pane| pane.read(cx).unkept_edit(cx));
                unkept
                    && match keeping {
                        Keeping::Unkept => true,
                        Keeping::Uncounted(counted) => !counted.contains(key),
                    }
            })
            .map(|(id, _)| *id)
            .collect();
        if kept.is_empty() {
            for id in self.tabs.close_repository(root) {
                self.forget(id);
            }
            return;
        }
        for id in self.tabs.of_repository(root) {
            if !kept.contains(&id) && self.tabs.close(id) {
                self.forget(id);
            }
        }
    }

    /// The tabs whose edit session holds unsaved text that no draft holds,
    /// in strip order (Part C fix 1, I1).
    pub(super) fn unkept(&self, cx: &App) -> Vec<TabId> {
        self.tabs
            .tabs()
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| {
                self.panes
                    .get(id)
                    .is_some_and(|pane| pane.read(cx).unkept_edit(cx))
            })
            .collect()
    }

    /// Each tab's begin still in flight, which resolves once it has landed.
    pub(super) fn begins_in_flight(&self, cx: &App) -> Vec<Shared<oneshot::Receiver<()>>> {
        self.panes
            .values()
            .filter_map(|pane| pane.read(cx).begin_in_flight())
            .collect()
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

    /// Contract 13: a tab that is newly shown, or whose control takes
    /// focus, is scrolled into view in the strip. Focus is checked as each
    /// frame is drawn (as the item view does), since focus events reach
    /// only an active window. The strip applies the request when it has
    /// the tab's bounds, so a tab opened this frame is reached next frame.
    fn reveal(&self, window: &Window) {
        let focused = self.tabs.tabs().iter().map(|(id, _)| *id).find(|id| {
            [&self.focus, &self.close_focus]
                .iter()
                .any(|map| map.get(id).is_some_and(|focus| focus.is_focused(window)))
        });
        let (was_shown, was_focused) = self.seen.get();
        let shown = self.tabs.active();
        let target = if focused.is_some() && focused != was_focused {
            focused
        } else if shown != was_shown {
            shown
        } else {
            None
        };
        if let Some(ix) =
            target.and_then(|id| self.tabs.tabs().iter().position(|(tab, _)| *tab == id))
        {
            self.strip.scroll_to_item(ix);
        }
        self.seen.set((shown, focused));
    }

    pub(super) fn render(
        &self,
        window: &Window,
        content_focused: bool,
        cx: &mut Context<Workspace>,
    ) -> Vec<AnyElement> {
        let theme = cx.theme();
        let (ring, border, chrome, page) =
            (theme.ring, theme.border, theme.tab_bar, theme.background);
        let (foreground, muted) = (theme.foreground, theme.muted_foreground);
        let secondary = theme.secondary_foreground;
        let mono = theme.mono_font_family.clone();
        let colors = item_colors(cx);
        let shown = self.tabs.active();
        let count = self.tabs.tabs().len();
        self.reveal(window);
        let mut strip = h_flex()
            .id("tab-strip")
            .track_scroll(&self.strip)
            .test_support()
            .role(Role::TabList)
            .aria_label(OPEN_TABS)
            .flex_nowrap()
            .h(px(STRIP_HEIGHT))
            .flex_none()
            .w_full()
            .min_w(px(0.))
            .overflow_x_scroll()
            .bg(chrome)
            .border_b_1()
            .border_color(border);
        for (ix, (id, _)) in self.tabs.tabs().iter().enumerate() {
            let id = *id;
            let active = shown == Some(id);
            let info = self
                .panes
                .get(&id)
                .map(|pane| tab_info(pane.read(cx), cx))
                .unwrap_or_default();
            let name = info.name();
            let focus = self.focus.get(&id).cloned();
            let focused = focus.as_ref().is_some_and(|focus| focus.is_focused(window));
            let icon_color = if active { colors.accent } else { muted };
            let icon = div()
                .id(("tab-icon", ix))
                .test_support()
                .aria_label(info.kind.label())
                .flex_none()
                .child(
                    Icon::default()
                        .path(icon_path(info.kind.icon()))
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE))
                        .text_color(icon_color),
                );
            let code = info.code.clone().map(|code| {
                text(("tab-code", ix), code)
                    .flex_none()
                    .font_family(mono.clone())
                    .text_size(px(CODE_SIZE))
                    .whitespace_nowrap()
                    .text_color(secondary)
            });
            let dot = info.unsaved.then(|| {
                div()
                    .id(("tab-unsaved", ix))
                    .test_support()
                    .aria_label(UNSAVED_TOOLTIP)
                    .flex_none()
                    .w(px(DOT_SIZE))
                    .h(px(DOT_SIZE))
                    .rounded_full()
                    .bg(colors.accent)
                    .tooltip(|window, cx| Tooltip::new(UNSAVED_TOOLTIP).build(window, cx))
            });
            let tooltip = info.tooltip.clone();
            let mut tab = div()
                .id(("tab", ix))
                .test_support()
                .role(Role::Tab)
                .aria_label(name)
                .aria_selected(active)
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .pl(px(12.))
                .pr(px(2.))
                .when_some_focus(focus.as_ref())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_tab(id, cx)))
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .child(
                    h_flex()
                        .h_full()
                        .gap(px(TAB_GAP))
                        .items_center()
                        .child(icon)
                        .children(code)
                        .child(
                            text(("tab-title", ix), info.title.clone())
                                .min_w(px(0.))
                                .truncate()
                                .text_size(px(TITLE_SIZE)),
                        )
                        .children(dot),
                );
            if focused {
                tab = tab.border_2().border_color(ring);
            }
            if let Some(focus) = &focus {
                tab = tab.relative().child(self.scroll.marker(focus));
            }
            let mut cell = h_flex()
                .relative()
                .flex_none()
                .max_w(px(MAX_TAB_WIDTH))
                .h_full()
                .items_center()
                .pr(px(6.))
                .border_r_1()
                .border_color(border)
                .text_color(if active { foreground } else { muted });
            if active {
                let (line_name, line_color) = active_line(content_focused, &colors);
                let line = div()
                    .id(("tab-active-line", ix))
                    .test_support()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(ACTIVE_LINE))
                    .bg(line_color);
                // The colour's name is for tests only: a decorative line has
                // no accessible value, so no screen reader announces a colour.
                #[cfg(test)]
                let line = line.child(
                    div()
                        .id(("tab-active-line-state", ix))
                        .test_support()
                        .aria_label(line_name),
                );
                #[cfg(not(test))]
                let _ = line_name;
                cell = cell.bg(page).child(line);
            }
            strip = strip.child(
                cell.child(tab).child(
                    div()
                        .id(("tab-close", ix))
                        .test_support()
                        .aria_label(close_label(&info.title))
                        .flex_none()
                        .w(px(CLOSE_SIZE))
                        .h(px(CLOSE_SIZE))
                        .when_some_focus(self.close_focus.get(&id))
                        .child(
                            Button::new(("tab-close-button", ix))
                                .ghost()
                                .w(px(CLOSE_SIZE))
                                .h(px(CLOSE_SIZE))
                                .p_0()
                                .child(
                                    Icon::default()
                                        .path(icon_path("close"))
                                        .w(px(CLOSE_ICON))
                                        .h(px(CLOSE_ICON)),
                                )
                                .accessibility_label(close_label(&info.title))
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
