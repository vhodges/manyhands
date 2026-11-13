//! A needs-attention row's action on the Checking for updates page (D6
//! design C2, C3): the same action the operation area offers for the row's
//! reason, from the repository's own row. Unlock… and Resume checking go to
//! the scheduler; Choose a key…, Approve host… and Open remote settings…
//! send the person to the page that fixes the cause, through the host, and
//! Check now follows when the change finishes.

use std::path::PathBuf;

use gpui_kit::{Context, Window, component::WindowExt};
use manyhands::repository::transport::SshAuthority;

use super::{PollingEvent, PollingPage, Wrapped};
use crate::desktop::model::{
    polling::{
        Phase, Reason,
        attention::{Action, action_of, locked_key, ssh_authority},
        status::Shape,
    },
    stamp::RepoKey,
};

/// What the person asked for: the host opens the page.
pub(crate) struct Attend {
    pub action: Action,
    pub root: PathBuf,
    /// The host a `HostUnapproved` entry is for.
    pub authority: Option<SshAuthority>,
}

impl PollingPage {
    /// The reason `root` is held back for, as its schedule says: a
    /// repository the library holds until recovery is held for it however
    /// the schedule stands.
    pub(super) fn held_for(
        &self,
        root: &RepoKey,
        shape: &Shape,
        cx: &Context<Self>,
    ) -> Option<Reason> {
        let phase = self
            .services
            .scheduler()
            .read(cx)
            .schedule()
            .repo(root)
            .map(|repo| repo.phase.clone());
        match phase {
            Some(Phase::Attention(reason)) => Some(reason),
            _ if *shape == Shape::Held => Some(Reason::RecoveryHeld(root.clone())),
            _ => None,
        }
    }

    /// The action for `reason`, in `root`'s row `ix`. Unlock… is disabled
    /// while no repository of the key reaches authentication.
    pub(super) fn attention_action(
        &mut self,
        ix: usize,
        root: &RepoKey,
        reason: &Reason,
        cx: &mut Context<Self>,
    ) -> Wrapped {
        let action = action_of(reason);
        let disabled = action == Action::Unlock
            && !locked_key(reason).is_some_and(|key| {
                let operations = self.services.operations().read(cx);
                self.services
                    .scheduler()
                    .read(cx)
                    .unlock_target(key, operations)
                    .is_some()
            });
        let (reason, root) = (reason.clone(), root.clone());
        self.button(
            ("polling-attention", "polling-attention-button"),
            ix,
            action.label(),
            disabled,
            move |this, window, cx| this.attend(action, &reason, &root, window, cx),
            cx,
        )
    }

    fn attend(
        &mut self,
        action: Action,
        reason: &Reason,
        root: &RepoKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let scheduler = self.services.scheduler().clone();
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
                scheduler.update(cx, |scheduler, _| {
                    scheduler.check_once_changed(root.clone())
                });
                cx.emit(PollingEvent::Attend(Attend {
                    action,
                    root: PathBuf::from(root),
                    authority: ssh_authority(reason),
                }));
            }
        }
    }
}
