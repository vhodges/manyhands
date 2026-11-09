//! Needs attention in the operation area (D6 design C3, W-6, decision 13):
//! one entry per key, host or repository the scheduler holds back for the
//! person, at the top of "This session". Each names the repositories it
//! holds back and offers the action its reason names; the entry goes when
//! the scheduler clears it, never by dismissal. A row wraps, so its action
//! stays inside the area at the default window.
//!
//! - **Unlock…** clears the key's declined mark and sends Check now through
//!   the key's first repository that reaches authentication, which prompts
//!   through the usual modal. With none, it is text, saying why.
//! - **Choose a key…**, **Approve host…** and **Open remote settings…** open
//!   the page that fixes the cause; Check now follows the change.
//! - **Resume checking** releases a recovery hold, off the UI thread.
//!
//! Text is fixed strings and repository names; no item, comment or key is
//! named.

use std::path::{Path, PathBuf};

use super::{button, scope_name, text, unavailable};
use crate::desktop::{
    model::{
        polling::attention::{
            Action, action_of, headline, holding_back, locked_key, ssh_authority,
        },
        polling::{AttentionEntry, Reason, Root},
        stamp::SettingsPage,
    },
    view::workspace::Workspace,
};
use gpui_kit::{
    AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled, Window,
    component::{ActiveTheme, h_flex},
};

const STEP_ASIDE: &str = "finish the pending operations of these repositories first";

impl Workspace {
    pub(super) fn attention_entries(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let entries = self.services().scheduler().read(cx).schedule().attention();
        let muted = cx.theme().muted_foreground;
        let mut shown = Vec::with_capacity(entries.len());
        for (ix, entry) in entries.into_iter().enumerate() {
            let names: Vec<String> = entry
                .roots
                .iter()
                .map(|root| scope_name(Some(Path::new(root))))
                .collect();
            let action = self.attention_action(ix, &entry, cx);
            // One row that wraps: the action stays inside the area (W-6).
            shown.push(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .items_center()
                    .child(text(("attention", ix), headline(&entry.reason)))
                    .child(
                        text(("attention-roots", ix), holding_back(&names))
                            .text_xs()
                            .text_color(muted),
                    )
                    .child(action)
                    .into_any_element(),
            );
        }
        shown
    }

    /// The action of entry `ix`: a control, or text saying why Unlock…
    /// cannot go.
    fn attention_action(
        &mut self,
        ix: usize,
        entry: &AttentionEntry,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let action = action_of(&entry.reason);
        let id = ("attention-action", ix);
        if action == Action::Unlock {
            let reachable = locked_key(&entry.reason).is_some_and(|key| {
                let operations = self.services().operations().read(cx);
                self.services()
                    .scheduler()
                    .read(cx)
                    .unlock_target(key, operations)
                    .is_some()
            });
            if !reachable {
                return unavailable(id, action.label(), STEP_ASIDE, cx).into_any_element();
            }
        }
        let focus = self.control_focus_for(format!("attention-action-{ix}"), cx);
        let reason = entry.reason.clone();
        let root = entry.roots.first().cloned();
        button(
            id,
            action.label(),
            Some(&focus),
            cx,
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                if let Some(root) = &root {
                    this.attend(action, &reason, root, window, cx);
                }
            }),
        )
    }

    /// The person pressed `action` on an entry of `reason` whose first
    /// repository is `root`.
    fn attend(
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
