//! Running what an Activity row offers (Wave 04 Cycle 03b, design A1 and
//! Part D). A menu's choice, a button and the test seam all end in
//! `choose_activity`, which checks the row again; `run_activity_action` then
//! calls the handler the action names. Nothing new is sent: each handler is
//! the one today's control calls.

use std::path::{Path, PathBuf};

use gpui_kit::{Context, FocusHandle, Window};
use manyhands::{canonical::ItemId, repository::AuthoringKind};

use crate::desktop::{
    keys::ActivityMenuChoice,
    model::{
        activity::{
            rows::{ActivityAction, RowKey},
            states::dto_kind,
        },
        navigation::OpenTarget,
        operations::LifecycleRow,
    },
    services::requests::{Intent, Lane},
    view::{operations::RetireSubject, workspace::Workspace},
};

impl Workspace {
    /// A choice from a row's menu, its button or the test seam. It runs only
    /// while the row stands as it did when the choice was made (R-46): the
    /// row's revision is the one the choice carries (T11), and the row still
    /// offers that action with that label, enabled (T12). Otherwise nothing
    /// runs, and the person chooses again.
    pub(in crate::desktop::view) fn choose_activity(
        &mut self,
        choice: &ActivityMenuChoice,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.activity().still_offered(
            &choice.key,
            choice.revision,
            &choice.action,
            &choice.shown,
        ) {
            return;
        }
        self.highlight_applied_row(&choice.key);
        self.run_activity_action(&choice.action, origin, window, cx);
    }

    /// The handler `action` names (A1's table). `origin` is where focus goes
    /// back to when the action opens a dialog or sheet, and closes it.
    pub(in crate::desktop::view) fn run_activity_action(
        &mut self,
        action: &ActivityAction,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ActivityAction::Cancel(id) => self.cancel_request(*id, cx),
            ActivityAction::Stop(id) => self.stop_request(*id, cx),
            ActivityAction::Open(id) => self.open_lifecycle(*id, origin, window, cx),
            ActivityAction::Review(id) => self.review_request(*id, origin, window, cx),
            ActivityAction::Offer(id, offer) => {
                self.run_share_offer(*id, *offer, origin, window, cx)
            }
            ActivityAction::Recovery(id, n) => self.run_recovery(*id, *n, origin, window, cx),
            ActivityAction::Dismiss(RowKey::Request(id)) => self.dismiss_request(*id, cx),
            ActivityAction::Dismiss(key) => self.dismiss_activity_row(key, cx),
            ActivityAction::DismissRecovered => self
                .services()
                .drafts()
                .update(cx, |registry, cx| registry.dismiss_recovered(cx)),
            ActivityAction::DismissNote(report) => {
                self.services().operations().update(cx, |operations, cx| {
                    operations.dismiss_record_note(*report);
                    cx.notify();
                })
            }
            // The hold's first root, as today's entry passes it.
            ActivityAction::Attend(action, reason, root) => {
                self.attend(*action, reason, root, window, cx)
            }
            ActivityAction::Repository(root, action) => {
                self.row_action(action, root.clone(), window, cx)
            }
            ActivityAction::OpenItem(item) => self.open_item(
                OpenTarget::Item {
                    repo: item.root.clone(),
                    kind: dto_kind(item.kind),
                    id: item.id.clone(),
                },
                cx,
            ),
            ActivityAction::Share {
                root,
                item: Some((kind, id)),
            } => self.share_unpublished(
                Intent::ItemSync {
                    root: PathBuf::from(root),
                    kind: *kind,
                    id: id.clone(),
                },
                cx,
            ),
            ActivityAction::Share { root, item: None } => {
                let remote = self.publication_remote(root);
                self.share_main(root.clone(), remote, window, cx)
            }
            ActivityAction::Retire { root, kind, item } => self.retire_chosen(
                RetireSubject {
                    root: root.clone(),
                    kind: *kind,
                    item: item.clone(),
                },
                window,
                cx,
            ),
            ActivityAction::Resume { root, operation } => self.resume(root.clone(), operation, cx),
            ActivityAction::ResumeLifecycle {
                root,
                kind,
                item,
                operation,
            } => {
                let row = LifecycleRow::new(*kind, item.clone(), operation.clone());
                self.resume_lifecycle(root, &row, origin, window, cx)
            }
            ActivityAction::ReviewPending { root, operation } => {
                self.review_pending(root, operation, origin, window, cx)
            }
            ActivityAction::Abandon { root, operation } => {
                self.abandon(root.clone(), operation, origin, window, cx)
            }
            ActivityAction::RetryPendingList(root) => self.load_pending(root.clone(), cx),
            ActivityAction::RetryFlush(draft) => self.retry_draft_flush(*draft, cx),
            ActivityAction::ReviewDrafts => self.open_drafts(None, window, cx),
        }
    }

    /// Whether `share` is queued, waiting out `busy` or running: the
    /// worker's own account, as the item header's Share reads it.
    fn sharing(&self, share: &Intent) -> bool {
        let Intent::ItemSync { root, kind, id } = share else {
            return false;
        };
        self.share_in_flight(root, *kind, id)
    }

    /// Whether a share of the item is queued, waiting out `busy` or running:
    /// the one account `sharing` and S6's Share… state both read.
    pub(in crate::desktop::view) fn share_in_flight(
        &self,
        root: &Path,
        kind: AuthoringKind,
        id: &ItemId,
    ) -> bool {
        self.services().worker().in_flight(|intent| {
            matches!(
                intent,
                Intent::ItemSync { root: asked, kind: asked_kind, id: asked_id }
                    if asked == root && *asked_kind == kind && asked_id == id
            )
        })
    }

    /// Share… on an unshared item: sends the share the item's header sends
    /// (`item sync`), on the user lane, unless one is already queued or
    /// running. A poll runs only with a publication remote, so there is no
    /// remote to set up here.
    pub(in crate::desktop::view) fn share_unpublished(
        &mut self,
        share: Intent,
        cx: &mut Context<Self>,
    ) {
        if self.sharing(&share) {
            return;
        }
        self.services()
            .worker()
            .submit(share, Lane::User, None, None);
        cx.notify();
    }
}
