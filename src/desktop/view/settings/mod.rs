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
//! Checking for updates (`polling`) is the fifth, in a module of its own so
//! Wave 04 Cycle 04 can fold it into the settings sheet.
//! The host keeps the repository selected when the settings were last
//! opened, for a page that works on one repository (Remotes).

use std::{cell::Cell, path::PathBuf, rc::Rc};

mod arrive;
mod attention;
mod frame;
mod hosts;
mod keys;
mod polling;
mod remotes;
mod repositories;

use gpui_kit::{
    AnyView, AppContext, Context, Entity, EventEmitter, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, StatefulInteractiveElement, Styled, Subscription, TestSupportExt,
    WeakFocusHandle, Window, div,
};

use manyhands::repository::transport::SshAuthority;

pub(crate) use arrive::SettingsRoute;
pub(crate) use hosts::HostsPage;
pub(crate) use keys::KeysPage;
pub(crate) use polling::{PollingEvent, PollingPage};
pub(crate) use remotes::RemotesPage;
pub(crate) use repositories::RepositoriesPage;

use super::scroll::ContentScroll;
use crate::desktop::{model::stamp::SettingsPage, services::Services};

pub(crate) const SETTINGS_HEADING: &str = "Settings";
/// The sheet header's description (design B1; frames 1e and 1f).
pub(crate) const SETTINGS_NOTE: &str = "Applies to this computer only.";

/// The pages there are, in the order the page list shows them.
pub(crate) const PAGES: [SettingsPage; 5] = [
    SettingsPage::Repositories,
    SettingsPage::Remotes,
    SettingsPage::Keys,
    SettingsPage::Hosts,
    SettingsPage::Polling,
];

/// A page's name in the page list.
pub(crate) fn page_title(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::Repositories => "Repositories",
        SettingsPage::Remotes => "Remotes",
        SettingsPage::Keys => "Shared SSH keys",
        SettingsPage::Hosts => "Hosts",
        SettingsPage::Polling => "Checking for updates",
    }
}

/// Whether Settings is closed (`Tabs`, R-3), or the page entity the sheet
/// last arrived at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Shown {
    Tabs,
    Settings(SettingsPage),
}

/// Whether Settings is open, shared by the host with the pages it makes
/// (design A4; R-47): set when Settings opens and cleared at once when it
/// closes, the Kit's close included. A page's landing moves focus or opens
/// a dialog only while it says open.
#[derive(Clone, Default)]
pub(crate) struct OpenFlag(Rc<Cell<bool>>);

impl OpenFlag {
    fn set(&self, open: bool) {
        self.0.set(open);
    }

    /// Settings is open: the flag is set and the sheet is still active, so a
    /// close the Kit made without telling the workspace yet counts as closed
    /// too (A7 item 10).
    pub(crate) fn is_open(&self, window: &mut Window, cx: &mut gpui_kit::App) -> bool {
        use gpui_kit::component::WindowExt;

        self.0.get() && window.has_active_sheet(cx)
    }
}

/// What the settings ask of the workspace.
pub(crate) enum SettingsEvent {
    /// A row of Checking for updates points at the pending operation that
    /// holds it: Settings closes, and the operation area, which lists it,
    /// takes focus (design C3).
    ShowActivity,
}

pub(crate) struct SettingsHost {
    services: Services,
    /// The sheet body's own scroll, a region of the content column's (B2):
    /// every page reveals its focused control in it.
    scroll: ContentScroll,
    shown: Shown,
    /// The focus of each page's entry in the page list, in `PAGES` order.
    entries: Vec<FocusHandle>,
    /// What held focus when the sheet opened, where focus returns when it
    /// closes (design A3).
    origin: Option<WeakFocusHandle>,
    /// Whether Settings is open, which every page is given (A4).
    open: OpenFlag,
    /// The focus of the layer the sheet is drawn in, which the workspace
    /// tracks around Root's sheet layer: focus inside it is in the sheet
    /// (design A3, "The sheet keeps focus").
    layer: FocusHandle,
    repositories: Option<Entity<RepositoriesPage>>,
    remotes: Option<Entity<RemotesPage>>,
    keys: Option<Entity<KeysPage>>,
    hosts: Option<Entity<HostsPage>>,
    polling: Option<Entity<PollingPage>>,
    /// What the Checking for updates page asks of the host.
    _polling_events: Option<Subscription>,
    /// The repository selected when the settings were last opened: the
    /// one the Remotes page works on until another is chosen there, and
    /// the one the Hosts page works on.
    selection: Option<PathBuf>,
}

impl EventEmitter<SettingsEvent> for SettingsHost {}

impl SettingsHost {
    /// `column` is the content region's column: the host scrolls a region
    /// of its own that shares the column's focus reveal (B2).
    pub(crate) fn new(services: Services, column: ContentScroll, cx: &mut Context<Self>) -> Self {
        Self {
            services,
            scroll: column.region(),
            shown: Shown::Tabs,
            entries: PAGES
                .iter()
                .map(|_| cx.focus_handle().tab_stop(true))
                .collect(),
            origin: None,
            open: OpenFlag::default(),
            layer: cx.focus_handle(),
            repositories: None,
            remotes: None,
            keys: None,
            hosts: None,
            polling: None,
            _polling_events: None,
            selection: None,
        }
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn shown(&self) -> Shown {
        self.shown
    }

    /// The repository a page that works on one starts on.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn selection(&self) -> Option<&PathBuf> {
        self.selection.as_ref()
    }

    /// The vertical offset of the scroll the pages are drawn in.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn scroll_offset(&self) -> gpui_kit::Pixels {
        self.scroll.handle().offset().y
    }

    /// Whether the host shows a page: Settings is open, or closing.
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
        if page == SettingsPage::Polling {
            self.polling(window, cx)
                .update(cx, |page, cx| page.opened(cx));
        }
        cx.notify();
    }

    /// Share… and Share main…: the Remotes page on `root`, even when it
    /// already works on another repository, so a publication remote is
    /// chosen for the repository shared.
    pub(crate) fn open_remotes_on(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shown = Shown::Settings(SettingsPage::Remotes);
        self.remotes(window, cx)
            .update(cx, |page, cx| page.open_on(root, cx));
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

    /// Settings closed. The pages are kept, with what they hold, except
    /// the Keys page's typed passphrase, emptied now (design B3), and an
    /// Add waiting for its folder's check, which no longer goes on (R-48).
    pub(crate) fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_shown() {
            self.shown = Shown::Tabs;
            cx.notify();
        }
        self.forget_passphrase(window, cx);
        if let Some(repositories) = &self.repositories {
            repositories.update(cx, |page, _| page.forget_pending_submit());
        }
    }

    /// Settings opens (`true`) or has closed (`false`): what every page's
    /// landing reads (A4).
    pub(crate) fn set_open(&self, open: bool) {
        self.open.set(open);
    }

    /// The focus the workspace tracks around Root's sheet layer.
    pub(crate) fn layer(&self) -> &FocusHandle {
        &self.layer
    }

    /// Where focus returns when Settings closes: `origin`, held weakly.
    pub(crate) fn set_origin(&mut self, origin: Option<&FocusHandle>) {
        self.origin = origin.map(FocusHandle::downgrade);
    }

    /// The origin set when Settings opened, if it still exists; taken.
    pub(crate) fn take_origin(&mut self) -> Option<FocusHandle> {
        self.origin.take().and_then(|origin| origin.upgrade())
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
        let open = self.open.clone();
        self.repositories
            .get_or_insert_with(|| {
                cx.new(|cx| RepositoriesPage::new(services, scroll, open, window, cx))
            })
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
        let open = self.open.clone();
        self.keys
            .get_or_insert_with(|| cx.new(|cx| KeysPage::new(services, scroll, open, window, cx)))
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
        let open = self.open.clone();
        let selection = self.selection.clone();
        self.hosts
            .get_or_insert_with(|| {
                cx.new(|cx| HostsPage::new(services, scroll, open, selection, window, cx))
            })
            .clone()
    }

    /// The Checking for updates page, made the first time it is asked for.
    pub(crate) fn polling(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PollingPage> {
        if self.polling.is_none() {
            let (services, scroll) = (self.services.clone(), self.scroll.clone());
            let page = cx.new(|cx| PollingPage::new(services, scroll, window, cx));
            self._polling_events = Some(cx.subscribe_in(
                &page,
                window,
                |this, _, event: &PollingEvent, window, cx| match event {
                    PollingEvent::SetUpSharing(root) => {
                        this.set_up_sharing(root.clone(), window, cx)
                    }
                    PollingEvent::Attend(attend) => this.attend(attend, window, cx),
                    PollingEvent::ShowActivity => cx.emit(SettingsEvent::ShowActivity),
                },
            ));
            self.polling = Some(page);
        }
        self.polling.clone().expect("the page was just made")
    }

    /// Set up sharing…, inside the sheet (W-31): the Remotes page on
    /// `root`, as its route arrives, so the person is where a publication
    /// remote is chosen.
    fn set_up_sharing(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.selection = Some(root.clone());
        self.arrive_focused(&SettingsRoute::Remotes(root), window, cx);
    }

    /// Approve host… on the host a poll named, inside the sheet (W-32): the
    /// Hosts page on `root`, checking the host `authority`, with focus on
    /// Check host, as its route arrives.
    pub(super) fn check_host(
        &mut self,
        root: PathBuf,
        authority: SshAuthority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.arrive_focused(&SettingsRoute::CheckHost(root, authority), window, cx);
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
            SettingsPage::Polling => Some(self.polling(window, cx).into()),
        }
    }
}

/// The widest a text field is drawn (fact 17).
const FIELD_WIDTH: f32 = 480.;

/// A labelled field's column: it fills the sheet body's width, up to the
/// widest a field is drawn, and its wrapped input fills it (B1; R-39).
pub(crate) fn field_column() -> gpui_kit::Div {
    gpui_kit::component::v_flex()
        .w_full()
        .max_w(gpui_kit::px(FIELD_WIDTH))
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

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "polling_support.rs"]
pub(crate) mod polling_support;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "polling_tests.rs"]
mod polling_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "polling_real_tests.rs"]
mod polling_real_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "attention_tests.rs"]
mod attention_tests;
