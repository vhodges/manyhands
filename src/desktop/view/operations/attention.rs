//! A hold's action (D6 design C3, W-6, decision 13; Wave 04 Cycle 03b): the
//! one a hold's Activity row, or a repository's, offers. The rows are the
//! model's (S3, S4); what stays here is what pressing the action does.
//!
//! - **Unlock…** clears the key's declined mark and sends Check now through
//!   the key's first repository that reaches authentication, which prompts
//!   through the usual modal. With none, it is text, saying why.
//! - **Choose a key…**, **Approve host…** and **Open remote settings…** open
//!   the page that fixes the cause; Check now follows the change.
//! - **Resume checking** releases a recovery hold, off the UI thread.

use std::path::PathBuf;

use crate::desktop::{
    model::{
        polling::attention::{Action, locked_key, ssh_authority},
        polling::{Reason, Root},
        stamp::SettingsPage,
    },
    view::workspace::Workspace,
};
use gpui_kit::{Context, Window};

impl Workspace {
    /// The person pressed `action` on an entry of `reason` whose first
    /// repository is `root`, or chose it from that repository's menu.
    pub(in crate::desktop::view) fn attend(
        &mut self,
        action: Action,
        reason: &Reason,
        root: &Root,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let scheduler = self.services().scheduler().clone();
        match action {
            Action::Unlock => {
                if let Some(key) = locked_key(reason) {
                    scheduler.update(cx, |scheduler, cx| scheduler.unlock(key, cx));
                }
            }
            Action::ResumeChecking => {
                scheduler.update(cx, |scheduler, cx| {
                    scheduler.resume_checking(root.clone(), cx)
                });
            }
            Action::ChooseKey | Action::ApproveHost | Action::OpenRemoteSettings => {
                // Check now follows the change.
                scheduler.update(cx, |scheduler, _| {
                    scheduler.check_once_changed(root.clone())
                });
                let path = PathBuf::from(root);
                match action {
                    Action::ChooseKey => self.open_settings(SettingsPage::Keys, window, cx),
                    Action::OpenRemoteSettings => self.open_remotes(path, window, cx),
                    _ => match ssh_authority(reason) {
                        Some(authority) => self.check_host_in_settings(path, authority, window, cx),
                        None => self.open_settings(SettingsPage::Hosts, window, cx),
                    },
                }
            }
        }
    }
}
