//! The settings host's part of a needs-attention row's action (D6 design
//! C3): Choose a key…, Approve host… and Open remote settings… show the
//! page that fixes the cause, with focus where the person works.

use gpui_kit::{Context, Window};

use super::{SettingsHost, polling::Attend};
use crate::desktop::model::{polling::attention::Action, stamp::SettingsPage};

impl SettingsHost {
    pub(super) fn attend(&mut self, attend: &Attend, window: &mut Window, cx: &mut Context<Self>) {
        match attend.action {
            Action::ChooseKey => {
                self.open(SettingsPage::Keys, window, cx);
                if let Some(focus) = self.first_focus() {
                    window.focus(&focus, cx);
                }
            }
            Action::ApproveHost => match &attend.authority {
                // Check host on the host the poll named, as a share's.
                Some(authority) => {
                    self.check_host(attend.root.clone(), authority.clone(), window, cx)
                }
                None => {
                    self.selection = Some(attend.root.clone());
                    self.open(SettingsPage::Hosts, window, cx);
                }
            },
            Action::OpenRemoteSettings => self.set_up_sharing(attend.root.clone(), window, cx),
            // Not pages: the row handled them.
            Action::Unlock | Action::ResumeChecking => {}
        }
    }
}
