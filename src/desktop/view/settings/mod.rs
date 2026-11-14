//! The settings, a sheet over the main window (Wave 04 Cycle 04):
//! `secondary-,` or the sidebar's Settings entry shows them over the
//! tabs, which are kept as they were, and Escape closes them. A menu of
//! three entries, Repositories, Sharing & keys and Checking for updates,
//! down the left shows the page each opens.
//!
//! `SettingsHost` is the entity the workspace holds. It draws the menu
//! and the shown page, each page an entity of its own made the first
//! time it is shown. A page reads and acts through `Services`
//! only; what the workspace must do, it hears as a `SettingsEvent`. A
//! page's controls sit in focus-tracking wrappers with a scroll marker, so
//! one that takes focus below the fold is brought into view (`scroll`).
//!
//! A page arrives by a `SettingsPage` variant, a field, its `nav` and an
//! arm in `views`; a new menu entry also adds its first page to `PAGES`.
//! The host keeps one chosen repository, which the Repositories page, its
//! Sharing section (the Remotes page) and Trusted servers' Check host work
//! on (`chosen`; Wave 04 Cycle 04 design C1).

use std::{cell::Cell, path::PathBuf, rc::Rc};

mod arrive;
mod attention;
mod chosen;
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
pub(crate) use repositories::{RepositoriesEvent, RepositoriesPage};

use super::scroll::ContentScroll;
use crate::desktop::{
    model::stamp::{SettingsNav, SettingsPage},
    services::Services,
};

pub(crate) const SETTINGS_HEADING: &str = "Settings";
/// The sheet header's description (design B1; frames 1e and 1f).
pub(crate) const SETTINGS_NOTE: &str = "Applies to this computer only.";

/// The menu's entry names (design Part E).
pub(crate) const REPOSITORIES_PAGE: &str = "Repositories";
pub(crate) const SHARING_AND_KEYS_PAGE: &str = "Sharing & keys";
pub(crate) const CHECKING_PAGE: &str = "Checking for updates";

/// The menu's entries, in order, each as the page it opens: its first
/// page (design B1). Repositories also shows Remotes, under Sharing, and
/// Sharing & keys also shows Hosts, Trusted servers (`SettingsPage::nav`).
pub(crate) const PAGES: [SettingsPage; 3] = [
    SettingsPage::Repositories,
    SettingsPage::Keys,
    SettingsPage::Polling,
];

/// The name of the menu entry that shows `page`.
pub(crate) fn page_title(page: SettingsPage) -> &'static str {
    match page.nav() {
        SettingsNav::Repositories => REPOSITORIES_PAGE,
        SettingsNav::SharingAndKeys => SHARING_AND_KEYS_PAGE,
        SettingsNav::Checking => CHECKING_PAGE,
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
    /// The focus of each menu entry, in `PAGES` order.
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
    /// What the Repositories page asks of the host: Manage, and a
    /// repository it let go.
    _repositories_events: Option<Subscription>,
    /// The repository the Repositories page (its identity form, Remove and
    /// Sharing) and Trusted servers' Check host work on (design C1). Every
    /// choice goes through `choose`.
    chosen: Option<PathBuf>,
    /// The Repositories panel's selection the last open saw: an open
    /// follows the panel only when it has changed since (R-14).
    seen: Option<PathBuf>,
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
            _repositories_events: None,
            chosen: None,
            seen: None,
        }
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn shown(&self) -> Shown {
        self.shown
    }

    /// The Repositories page, once it is made.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn made_repositories(&self) -> Option<Entity<RepositoriesPage>> {
        self.repositories.clone()
    }

    /// The repository the pages that work on one work on.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn chosen(&self) -> Option<&PathBuf> {
        self.chosen.as_ref()
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

    /// Shows `page`, making the pages its menu entry draws the first time,
    /// and reads again what they show (R-32). Repositories and Remotes draw
    /// one page: the registrations, then the chosen repository's remotes
    /// under Sharing (C1). Keys and Hosts draw Sharing & keys: the keys,
    /// then Trusted servers (C2). A typed passphrase does not outlive
    /// Sharing & keys (B3).
    pub(crate) fn open(&mut self, page: SettingsPage, window: &mut Window, cx: &mut Context<Self>) {
        self.shown = Shown::Settings(page);
        self.forget_passphrase(window, cx);
        match page.nav() {
            SettingsNav::Repositories => {
                self.repositories(window, cx)
                    .update(cx, |page, cx| page.opened(cx));
                self.remotes(window, cx)
                    .update(cx, |page, cx| page.opened(cx));
            }
            SettingsNav::SharingAndKeys => {
                self.keys(window, cx).update(cx, |page, cx| page.opened(cx));
                let chosen = self.chosen.clone();
                self.hosts(window, cx)
                    .update(cx, |page, cx| page.opened(chosen, cx));
            }
            SettingsNav::Checking => {
                self.polling(window, cx)
                    .update(cx, |page, cx| page.opened(cx));
            }
        }
        cx.notify();
    }

    /// Empties the Keys page's masked inputs unless Sharing & keys is shown,
    /// by either of its pages (B3, R-13): a typed passphrase does not
    /// outlive Sharing & keys, and Settings closed shows no page. A
    /// key-list route's retarget (R-22) goes with it.
    pub(crate) fn forget_passphrase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sharing = matches!(
            self.shown,
            Shown::Settings(page) if page.nav() == SettingsNav::SharingAndKeys
        );
        if !sharing && let Some(keys) = self.keys.clone() {
            keys.update(cx, |page, cx| {
                page.forget_passphrase(window, cx);
                page.forget_retarget();
            });
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

    /// The focus of the shown page's menu entry: the settings' first
    /// control.
    pub(crate) fn first_focus(&self) -> Option<FocusHandle> {
        let Shown::Settings(page) = self.shown else {
            return None;
        };
        let ix = PAGES.iter().position(|first| first.nav() == page.nav())?;
        Some(self.entries[ix].clone())
    }

    /// The Repositories page, made the first time it is asked for, with the
    /// Remotes page drawn in it under Sharing (design C1).
    pub(crate) fn repositories(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<RepositoriesPage> {
        if self.repositories.is_none() {
            let sharing = self.remotes(window, cx).into();
            let (services, scroll) = (self.services.clone(), self.scroll.clone());
            let open = self.open.clone();
            let page =
                cx.new(|cx| RepositoriesPage::new(services, scroll, open, sharing, window, cx));
            self._repositories_events = Some(cx.subscribe_in(
                &page,
                window,
                |this, _, event: &RepositoriesEvent, window, cx| {
                    this.repositories_event(event, window, cx)
                },
            ));
            self.repositories = Some(page);
        }
        self.repositories.clone().expect("the page was just made")
    }

    /// The Remotes page, made the first time it is asked for, on the
    /// chosen repository.
    pub(crate) fn remotes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<RemotesPage> {
        let (services, scroll) = (self.services.clone(), self.scroll.clone());
        let chosen = self.chosen.clone();
        self.remotes
            .get_or_insert_with(|| {
                cx.new(|cx| RemotesPage::new(services, scroll, chosen, window, cx))
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

    /// The Hosts page, made the first time it is asked for, on the chosen
    /// repository.
    pub(crate) fn hosts(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<HostsPage> {
        let (services, scroll) = (self.services.clone(), self.scroll.clone());
        let open = self.open.clone();
        let chosen = self.chosen.clone();
        self.hosts
            .get_or_insert_with(|| {
                cx.new(|cx| HostsPage::new(services, scroll, open, chosen, window, cx))
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

    /// Set up sharing…, inside the sheet (W-31): `root` chosen, and the
    /// Remotes page on it, as its route arrives, so the person is where a
    /// publication remote is chosen.
    fn set_up_sharing(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.choose(Some(root.clone()), window, cx);
        self.arrive_focused(&SettingsRoute::Remotes(root), window, cx);
    }

    /// Approve host… on the host a poll named, inside the sheet (W-32):
    /// `root` chosen, and the Hosts page on it, checking the host
    /// `authority`, with focus on Check host, as its route arrives.
    pub(super) fn check_host(
        &mut self,
        root: PathBuf,
        authority: SshAuthority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.choose(Some(root.clone()), window, cx);
        self.arrive_focused(&SettingsRoute::CheckHost(root, authority), window, cx);
    }

    /// The views the menu entry that shows `page` draws, in order, each made
    /// the first time it is asked for. Remotes is drawn in the Repositories
    /// page, under Sharing (design C1); Sharing & keys is the Keys page's
    /// view, then the Hosts page's, Trusted servers (C2).
    fn views(
        &mut self,
        page: SettingsPage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyView> {
        match page.nav() {
            SettingsNav::Repositories => vec![self.repositories(window, cx).into()],
            SettingsNav::SharingAndKeys => {
                vec![self.keys(window, cx).into(), self.hosts(window, cx).into()]
            }
            SettingsNav::Checking => vec![self.polling(window, cx).into()],
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

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "chosen_tests.rs"]
mod chosen_tests;
