//! The one repository Settings works on (Wave 04 Cycle 04 design C1): the
//! Repositories page's identity form and Remove, its Sharing section (the
//! Remotes page) and Trusted servers' Check host all work on the host's
//! `chosen`, and every choice goes through `choose`.
//!
//! - **At each open** (R-14): the route's repository, when it names one
//!   other than `Inspect`'s folder, which may not be registered (R-55);
//!   else the Repositories panel's selection, when it has changed since the
//!   last open; else the previous choice, kept with its typed text. Every
//!   open records the panel's selection it saw.
//! - **Manage** on a row chooses its repository (R-15).
//! - **Choosing the chosen repository** leaves its identity form as it is,
//!   so text being typed is kept (Cycle 03a's R5-2 lesson).
//! - **A repository the Repositories page lets go**, removed or no longer
//!   listed, is chosen no more, on any page (R-56).

use std::path::PathBuf;

use gpui_kit::{Context, Window};

use super::{RepositoriesEvent, SettingsHost, SettingsRoute};

impl SettingsRoute {
    /// The repository the route chooses: none for a page, the Add form,
    /// the key list or a retry; and none for `Inspect`, whose folder may
    /// not be registered (R-55).
    fn chosen_root(&self) -> Option<&PathBuf> {
        match self {
            Self::Manage(root)
            | Self::Identity(root)
            | Self::Remotes(root)
            | Self::Hosts(root)
            | Self::CheckHost(root, _) => Some(root),
            Self::Page(_)
            | Self::AddRepository
            | Self::Inspect(_)
            | Self::Keys
            | Self::RetryGeneration(_) => None,
        }
    }
}

impl SettingsHost {
    /// Settings opens at `route`, the Repositories panel's selection being
    /// `panel` (A2 step 6; R-14): the route's repository is chosen, else
    /// the panel's selection when it has changed since the last open; else
    /// the choice stays. A panel with no selection chooses nothing.
    pub(crate) fn choose_on_open(
        &mut self,
        route: &SettingsRoute,
        panel: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let changed = panel.is_some() && panel != self.seen;
        self.seen = panel.clone();
        if let Some(root) = route.chosen_root() {
            self.choose(Some(root.clone()), window, cx);
        } else if changed {
            self.choose(panel, window, cx);
        }
    }

    /// Chooses `root` for every page that works on one repository. The one
    /// already chosen is left as it is, its identity form's text included.
    /// Another is managed afresh on the Repositories page (its identity
    /// form emptied and read again), and Sharing and Check host move to it;
    /// `None` lets the choice go (R-56).
    pub(super) fn choose(
        &mut self,
        root: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if root == self.chosen {
            return;
        }
        self.chosen = root.clone();
        self.repositories(window, cx)
            .update(cx, |page, cx| match root.clone() {
                Some(root) => page.manage_root(root, window, cx),
                None => page.let_go(cx),
            });
        self.remotes(window, cx)
            .update(cx, |page, cx| page.open_on(root.clone(), cx));
        if let Some(hosts) = self.hosts.clone() {
            hosts.update(cx, |page, cx| page.choose(root, cx));
        }
        cx.notify();
    }

    /// What the Repositories page asks: Manage chooses its row's
    /// repository; the chosen one, once the page let it go, is chosen no
    /// more (R-56).
    pub(super) fn repositories_event(
        &mut self,
        event: &RepositoriesEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            RepositoriesEvent::Manage(root) => self.choose(Some(root.clone()), window, cx),
            RepositoriesEvent::Forgot(root) if self.chosen.as_ref() == Some(root) => {
                self.choose(None, window, cx)
            }
            RepositoriesEvent::Forgot(_) => {}
        }
    }
}
