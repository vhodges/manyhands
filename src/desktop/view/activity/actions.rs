//! Running what an Activity row offers (Wave 04 Cycle 03b, design A1 and
//! Part D). A menu's choice, a button and the test seam all end in
//! `choose_activity`, which checks the row again; `run_activity_action` then
//! calls the handler the action names. Nothing new is sent: each handler is
//! the one today's control calls.

use std::path::PathBuf;

use gpui_kit::{Context, FocusHandle, Window};
use manyhands::repository::{AuthoringKind, ItemDtoKind};

use crate::desktop::{
    keys::ActivityMenuChoice,
    model::{
        activity::rows::{ActivityAction, RowKey},
        navigation::OpenTarget,
        operations::LifecycleRow,
    },
    services::requests::Intent,
    view::{operations::RetireSubject, workspace::Workspace},
};

fn dto_kind(kind: AuthoringKind) -> ItemDtoKind {
    match kind {
        AuthoringKind::Ticket => ItemDtoKind::Ticket,
        AuthoringKind::Document => ItemDtoKind::Document,
    }
}

impl Workspace {
    /// A choice from a row's menu, its button or the test seam. It runs only
    /// while the row stands as it did when the choice was made (R-46): the
    /// row's revision is the one the choice carries (T11), and the row still
    /// offers that action with that label, enabled (T12). Otherwise nothing
    /// runs, and the person chooses again.
    #[allow(dead_code)] // Used from Task 9
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
}
