//! Where Settings opens and what it does on arrival (Wave 04 Cycle 04
//! design A2 and Part D): every way in names a `SettingsRoute`, and the
//! host applies it to its pages without moving focus.

use std::path::PathBuf;

use gpui_kit::{Context, FocusHandle, Window};
use manyhands::repository::transport::SshAuthority;

use super::SettingsHost;
use crate::desktop::{model::stamp::SettingsPage, services::requests::DesktopRequestId};

/// Where Settings opens and what it does on arrival (Part D).
#[derive(Clone, Debug)]
pub(crate) enum SettingsRoute {
    /// A page, focused on its menu entry.
    Page(SettingsPage),
    /// The Repositories page, on its Add folder field (W-5, W-6).
    #[allow(dead_code)] // Used from Task 6
    AddRepository,
    /// The Repositories page managing `root` (the repository menu's
    /// Settings…).
    Manage(PathBuf),
    /// The Repositories page, checking `root` in its Add folder
    /// (repo.inspect).
    Inspect(PathBuf),
    /// The Repositories page's identity form for `root`.
    Identity(PathBuf),
    /// Repositories › Sharing for `root`, on the add-remote Name field.
    Remotes(PathBuf),
    /// Sharing & keys, on the key list (R-22).
    #[allow(dead_code)] // Used from Task 6
    Keys,
    /// Sharing & keys › Trusted servers for `root`, on Check host.
    Hosts(PathBuf),
    /// As `Hosts`, checking `authority` (R-T16a).
    CheckHost(PathBuf, SshAuthority),
    /// Sharing & keys, asking again for request `id`'s passphrase (R-T15a).
    RetryGeneration(DesktopRequestId),
}

impl SettingsHost {
    /// Switches to the route's page and applies it: checks a folder or a
    /// host, asks again for a passphrase. Moves no focus. Gives the control
    /// to focus, which the page draws in the next frame; `None` means the
    /// page's menu entry. Until the routes name their fields (Task 6), each
    /// gives the focus today's entry function gave (design fact 10). The
    /// repository a route works on is the host's chosen one, which the
    /// caller chooses first (`choose_on_open`, or `choose` inside the
    /// sheet).
    pub(crate) fn arrive(
        &mut self,
        route: &SettingsRoute,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<FocusHandle> {
        match route {
            SettingsRoute::Page(page) => {
                self.open(*page, window, cx);
                None
            }
            SettingsRoute::AddRepository | SettingsRoute::Manage(_) => {
                self.open(SettingsPage::Repositories, window, cx);
                None
            }
            SettingsRoute::Inspect(root) => {
                self.open(SettingsPage::Repositories, window, cx);
                self.repositories(window, cx)
                    .update(cx, |page, cx| page.inspect_root(root.clone(), window, cx));
                None
            }
            SettingsRoute::Identity(_) => {
                self.open(SettingsPage::Repositories, window, cx);
                Some(self.repositories(window, cx).read(cx).name_focus(cx))
            }
            SettingsRoute::Remotes(_) => {
                self.open(SettingsPage::Remotes, window, cx);
                None
            }
            SettingsRoute::Keys => {
                self.open(SettingsPage::Keys, window, cx);
                None
            }
            SettingsRoute::Hosts(_) => {
                self.open(SettingsPage::Hosts, window, cx);
                None
            }
            SettingsRoute::CheckHost(root, authority) => {
                self.open(SettingsPage::Hosts, window, cx);
                self.hosts(window, cx).update(cx, |page, cx| {
                    page.check_host(root.clone(), authority.clone(), window, cx)
                })
            }
            SettingsRoute::RetryGeneration(id) => {
                self.open(SettingsPage::Keys, window, cx);
                self.keys(window, cx)
                    .update(cx, |page, cx| page.ask_again(*id, window, cx))
            }
        }
    }

    /// Arrives at `route` and focuses the control it gives, else the page's
    /// menu entry: every way in, and the routes the pages raise inside the
    /// sheet (W-31, W-32).
    pub(crate) fn arrive_focused(
        &mut self,
        route: &SettingsRoute,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focus = self
            .arrive(route, window, cx)
            .or_else(|| self.first_focus());
        if let Some(focus) = focus {
            window.focus(&focus, cx);
        }
    }
}
