//! The settings, shown in the main window's content region in place of
//! the tabs (ruling R-B4): `secondary-,` or the sidebar's Settings entry
//! shows them, Back to tabs, or opening an item, shows the tabs again as
//! they were. Showing them in the main window keeps one dialog layer, the
//! workspace's focus fallback and Quit working for every page.
//!
//! `SettingsHost` is the entity the workspace holds. It draws the page
//! list and the shown page, each page an entity of its own made the
//! first time it is shown. A page reads and acts through `Services`
//! only; what the workspace must do, it hears as a `SettingsEvent`. A
//! page's controls sit in focus-tracking wrappers with a scroll marker, so
//! one that takes focus below the fold is brought into view (`scroll`).
//!
//! A page arrives by adding it to `PAGES`, a field and an arm in `page`.
//! The host keeps the repository selected when the settings were last
//! opened, for a page that works on one repository (Remotes).

use std::path::PathBuf;

mod hosts;
mod keys;
mod remotes;
mod repositories;

use gpui_kit::{
    AnyElement, AnyView, AppContext, ClickEvent, Context, Entity, EventEmitter, FocusHandle,
    InteractiveElement, IntoElement, ParentElement, Render, Role, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    component::{ActiveTheme, button::Button, h_flex, v_flex},
    div,
};

use manyhands::{repository::transport::SshAuthority, results::ResultCode};

pub(crate) use hosts::HostsPage;
pub(crate) use keys::KeysPage;
pub(crate) use remotes::RemotesPage;
pub(crate) use repositories::RepositoriesPage;

use super::{scroll::ContentScroll, tabs::SHOWN};
use crate::desktop::{
    model::stamp::SettingsPage,
    services::{ReadResult, Services, requests::DesktopRequestId},
};

pub(crate) const SETTINGS_HEADING: &str = "Settings";
pub(crate) const BACK_TO_TABS: &str = "Back to tabs";

/// The pages there are, in the order the page list shows them.
pub(crate) const PAGES: [SettingsPage; 4] = [
    SettingsPage::Repositories,
    SettingsPage::Remotes,
    SettingsPage::Keys,
    SettingsPage::Hosts,
];

/// A page's name in the page list.
pub(crate) fn page_title(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::Repositories => "Repositories",
        SettingsPage::Remotes => "Remotes",
        SettingsPage::Keys => "Shared SSH keys",
        SettingsPage::Hosts => "Hosts",
    }
}

/// What the content region shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Shown {
    Tabs,
    Settings(SettingsPage),
}

/// What the settings ask of the workspace.
pub(crate) enum SettingsEvent {
    /// Back to tabs: the tabs are to be shown and focused.
    Closed,
}

pub(crate) struct SettingsHost {
    services: Services,
    scroll: ContentScroll,
    shown: Shown,
    /// The focus of each page's entry in the page list, in `PAGES` order.
    entries: Vec<FocusHandle>,
    /// Around Back to tabs, whose own focus is the Kit's.
    close: FocusHandle,
    repositories: Option<Entity<RepositoriesPage>>,
    remotes: Option<Entity<RemotesPage>>,
    keys: Option<Entity<KeysPage>>,
    hosts: Option<Entity<HostsPage>>,
    /// The repository selected when the settings were last opened: the
    /// one the Remotes page works on until another is chosen there, and
    /// the one the Hosts page works on.
    selection: Option<PathBuf>,
}

impl EventEmitter<SettingsEvent> for SettingsHost {}

impl SettingsHost {
    pub(crate) fn new(services: Services, scroll: ContentScroll, cx: &mut Context<Self>) -> Self {
        Self {
            services,
            scroll,
            shown: Shown::Tabs,
            entries: PAGES
                .iter()
                .map(|_| cx.focus_handle().tab_stop(true))
                .collect(),
            close: cx.focus_handle(),
            repositories: None,
            remotes: None,
            keys: None,
            hosts: None,
            selection: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn shown(&self) -> Shown {
        self.shown
    }

    /// Whether a settings page is shown in place of the tabs.
    pub(crate) fn is_shown(&self) -> bool {
        matches!(self.shown, Shown::Settings(_))
    }

    /// Shows `page`, making it the first time.
    pub(crate) fn open(&mut self, page: SettingsPage, window: &mut Window, cx: &mut Context<Self>) {
        self.shown = Shown::Settings(page);
        self.page(page, window, cx);
        if page == SettingsPage::Remotes {
            let selection = self.selection.clone();
            self.remotes(window, cx)
                .update(cx, |page, cx| page.opened(selection, cx));
        }
        if page == SettingsPage::Keys {
            self.keys(window, cx).update(cx, |page, cx| page.opened(cx));
        }
        if page == SettingsPage::Hosts {
            let selection = self.selection.clone();
            self.hosts(window, cx)
                .update(cx, |page, cx| page.opened(selection, cx));
        }
        cx.notify();
    }

    /// The repository selected in the main window: a page that works on
    /// one repository starts on it.
    pub(crate) fn set_selection(&mut self, selection: Option<PathBuf>) {
        self.selection = selection;
    }

    /// Empties the Keys page's masked inputs unless that page is the shown
    /// one: a typed passphrase does not outlive the page being shown.
    pub(crate) fn forget_passphrase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shown != Shown::Settings(SettingsPage::Keys)
            && let Some(keys) = self.keys.clone()
        {
            keys.update(cx, |page, cx| page.forget_passphrase(window, cx));
        }
    }

    /// Shows the tabs again. The pages are kept, with what they hold,
    /// except the Keys page's typed passphrase, which the next draw of
    /// the workspace empties (`forget_passphrase`).
    pub(crate) fn hide(&mut self, cx: &mut Context<Self>) {
        if self.is_shown() {
            self.shown = Shown::Tabs;
            cx.notify();
        }
    }

    /// The focus of the shown page's entry in the page list: the
    /// settings' first control.
    pub(crate) fn first_focus(&self) -> Option<FocusHandle> {
        let Shown::Settings(page) = self.shown else {
            return None;
        };
        let ix = PAGES.iter().position(|shown| *shown == page)?;
        Some(self.entries[ix].clone())
    }

    /// The Repositories page, made the first time it is asked for.
    pub(crate) fn repositories(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<RepositoriesPage> {
        let (services, scroll) = (self.services.clone(), self.scroll.clone());
        self.repositories
            .get_or_insert_with(|| cx.new(|cx| RepositoriesPage::new(services, scroll, window, cx)))
            .clone()
    }

    /// The Remotes page, made the first time it is asked for, on the
    /// selected repository.
    pub(crate) fn remotes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<RemotesPage> {
        let (services, scroll) = (self.services.clone(), self.scroll.clone());
        let selection = self.selection.clone();
        self.remotes
            .get_or_insert_with(|| {
                cx.new(|cx| RemotesPage::new(services, scroll, selection, window, cx))
            })
            .clone()
    }

    /// The Keys page, made the first time it is asked for.
    pub(crate) fn keys(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Entity<KeysPage> {
        let (services, scroll) = (self.services.clone(), self.scroll.clone());
        self.keys
            .get_or_insert_with(|| cx.new(|cx| KeysPage::new(services, scroll, window, cx)))
            .clone()
    }

    /// The Hosts page, made the first time it is asked for, on the selected
    /// repository.
    pub(crate) fn hosts(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<HostsPage> {
        let (services, scroll) = (self.services.clone(), self.scroll.clone());
        let selection = self.selection.clone();
        self.hosts
            .get_or_insert_with(|| {
                cx.new(|cx| HostsPage::new(services, scroll, selection, window, cx))
            })
            .clone()
    }

    /// The `host.approve` and `host.replace` recovery: the Hosts page on
    /// `root`, checking the host `authority`, with focus on Check host.
    pub(crate) fn check_host(
        &mut self,
        root: PathBuf,
        authority: SshAuthority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open(SettingsPage::Hosts, window, cx);
        self.hosts(window, cx)
            .update(cx, |page, cx| page.check_host(root, authority, window, cx));
    }

    /// The operation area's Retry of a protected key generation: the Keys
    /// page, asking for the passphrase of the request `id` again, with
    /// focus in its field.
    pub(crate) fn retry_generation(
        &mut self,
        id: DesktopRequestId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open(SettingsPage::Keys, window, cx);
        self.keys(window, cx)
            .update(cx, |page, cx| page.ask_again(id, window, cx));
    }

    /// The view of `page`, made the first time it is asked for. A view,
    /// not an element: `open` asks for it outside a frame.
    fn page(
        &mut self,
        page: SettingsPage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyView> {
        match page {
            SettingsPage::Repositories => Some(self.repositories(window, cx).into()),
            SettingsPage::Remotes => Some(self.remotes(window, cx).into()),
            SettingsPage::Keys => Some(self.keys(window, cx).into()),
            SettingsPage::Hosts => Some(self.hosts(window, cx).into()),
        }
    }

    /// The `repo.inspect` recovery: the Repositories page, with `root`
    /// checked in its Add folder.
    pub(crate) fn inspect(
        &mut self,
        root: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open(SettingsPage::Repositories, window, cx);
        self.repositories(window, cx)
            .update(cx, |page, cx| page.inspect_root(root, window, cx));
    }

    /// The `repo.identity_set` recovery: the Repositories page, with the
    /// identity form of `root` and focus in its name field.
    pub(crate) fn identity(
        &mut self,
        root: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open(SettingsPage::Repositories, window, cx);
        self.repositories(window, cx)
            .update(cx, |page, cx| page.manage_root(root, true, window, cx));
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.hide(cx);
        cx.emit(SettingsEvent::Closed);
    }

    /// The page list: one entry per page, the shown one saying so in text.
    fn entries(&self, shown: SettingsPage, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (ring, border) = (theme.ring, theme.border);
        let mut list = h_flex().id("settings-pages").gap_1().flex_wrap();
        for (ix, page) in PAGES.iter().copied().enumerate() {
            let focus = &self.entries[ix];
            let active = page == shown;
            let mut entry =
                div()
                    .id(("settings-page", ix))
                    .test_support()
                    .role(Role::Tab)
                    .aria_label(page_title(page))
                    .aria_selected(active)
                    .relative()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(border)
                    .track_focus(focus)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.open(page, window, cx)
                    }))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(div().text_sm().child(page_title(page)))
                            .children(active.then(|| {
                                div()
                                    .id(("settings-page-shown", ix))
                                    .test_support()
                                    .aria_label(SHOWN)
                                    .text_xs()
                                    .child(SHOWN)
                            })),
                    )
                    .child(self.scroll.marker(focus));
            if focus.is_focused(window) {
                entry = entry.border_2().border_color(ring);
            }
            list = list.child(entry);
        }
        list.into_any_element()
    }
}

impl Render for SettingsHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.forget_passphrase(window, cx);
        let Shown::Settings(shown) = self.shown else {
            return div().id("settings-hidden").into_any_element();
        };
        let page = self.page(shown, window, cx);
        let entries = self.entries(shown, window, cx);
        let close = wrapped(
            "settings-close",
            BACK_TO_TABS,
            &self.close,
            &self.scroll,
            Button::new("settings-close-button")
                .label(BACK_TO_TABS)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.back(cx))),
        );
        div()
            .id("settings")
            .test_support()
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .id("settings-heading")
                                    .test_support()
                                    .aria_label(SETTINGS_HEADING)
                                    .text_lg()
                                    .font_weight(gpui_kit::FontWeight::BOLD)
                                    .child(SETTINGS_HEADING),
                            )
                            .child(close),
                    )
                    .child(entries)
                    .children(page),
            )
            .into_any_element()
    }
}

/// A read's state, as a page shows it.
enum Read<T> {
    Loading,
    Ready(T),
    Busy,
    Failed(ResultCode),
}

impl<T> Read<T> {
    fn from_result<U>(result: ReadResult<U>, f: impl FnOnce(U) -> T) -> Self {
        match result {
            ReadResult::Ok(value) => Self::Ready(f(value)),
            ReadResult::Busy => Self::Busy,
            ReadResult::Failed(failure) => Self::Failed(failure.code),
        }
    }
}

/// A control in its wrapper.
pub(crate) type Wrapped = gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>>;

/// `control` inside a wrapper a test reads by `id`, labeled `label`, that
/// tracks `focus` and carries the scroll marker: focus on the control,
/// whose own focus may be private, brings the wrapper into view.
pub(crate) fn wrapped(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<gpui_kit::SharedString>,
    focus: &FocusHandle,
    scroll: &ContentScroll,
    control: impl IntoElement,
) -> Wrapped {
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .relative()
        .track_focus(focus)
        .child(control)
        .child(scroll.marker_within(focus))
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "mod_tests.rs"]
mod tests;
