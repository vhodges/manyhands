//! The settings host's part of a needs-attention row's action (D6 design
//! C3; Wave 04 Cycle 04 W-32): Choose a key…, Approve host… and Open remote
//! settings… arrive inside the sheet as their routes do from outside it,
//! on the control Part D names, keeping the origin Settings opened with.

use gpui_kit::{Context, Window};

use super::{SettingsHost, SettingsRoute, polling::Attend};
use crate::desktop::model::polling::attention::Action;

impl SettingsHost {
    pub(super) fn attend(&mut self, attend: &Attend, window: &mut Window, cx: &mut Context<Self>) {
        match attend.action {
            Action::ChooseKey => self.arrive_focused(&SettingsRoute::Keys, window, cx),
            Action::ApproveHost => match &attend.authority {
                // Check host on the host the poll named, as a share's.
                Some(authority) => {
                    self.check_host(attend.root.clone(), authority.clone(), window, cx)
                }
                // With none named, Check host on the row's own repository
                // (decision 9; R-23).
                None => {
                    self.choose(Some(attend.root.clone()), window, cx);
                    self.arrive_focused(&SettingsRoute::Hosts(attend.root.clone()), window, cx);
                }
            },
            Action::OpenRemoteSettings => self.set_up_sharing(attend.root.clone(), window, cx),
            // Not pages: the row handled them.
            Action::Unlock | Action::ResumeChecking => {}
        }
    }
}
