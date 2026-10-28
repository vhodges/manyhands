//! Worker completions, pending reads and explicit request/recovery actions.

use super::{Region, Workspace};
use crate::desktop::{
    model::{operations::RecoveryStep, stamp::RepoKey},
    services::{
        ReadKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    view::{
        confirm::{confirm, prepare_again},
        operations::recovery_id,
    },
};
use gpui_kit::{App, Context, FocusHandle, Window, component::WindowExt};
use manyhands::{drafts::DraftId, repository::OperationId};
use std::path::PathBuf;

impl Workspace {
    /// Resolves the root before reading its pending operations.
    #[allow(clippy::result_large_err)]
    pub(crate) fn load_pending(&mut self, root: RepoKey, cx: &mut Context<Self>) {
        let stamp = self
            .services
            .operations()
            .update(cx, |operations, _| operations.load_pending(root.clone()));
        let path = PathBuf::from(&root);
        let read = self.services.read(
            ReadKind::Operations,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&path)?;
                service.list_operations(&repo)
            },
            cx,
            |this: &mut Self, stamp, result, cx| {
                this.services.operations().update(cx, |operations, cx| {
                    if operations.apply_pending(stamp, result) {
                        cx.notify();
                    }
                });
            },
        );
        self.pending_reads.insert(root, read);
        cx.notify();
    }

    /// Coalesce each batch's changed roots, while keeping prompt/quit order.
    pub(super) fn operations_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
        // Confirmation observers close their dialogs before a prompt opens.
        // The Kit closes the top dialog only.
        cx.defer_in(window, |this, window, cx| this.show_prompt(window, cx));
        self.quit_if_finished(cx);
        let changed = self
            .services
            .operations()
            .update(cx, |operations, _| operations.take_changed());
        if changed.is_empty() {
            return;
        }
        for root in &changed.removed {
            self.close_repository(root, cx);
            if self.navigation.forget(root) {
                self.clear_lists(cx);
            }
        }
        let listed = self.listed().unwrap_or_default();
        for root in &changed.roots {
            if listed.contains(root) {
                self.load_pending(root.clone(), cx);
            }
        }
        if self
            .navigation
            .selected()
            .is_some_and(|selected| changed.roots.contains(selected))
        {
            self.reload_lists(cx);
        }
        // Only requests that can change items reload tabs (ruling R-B2).
        self.reload_tabs(&changed.items, cx);
        self.load_repositories(cx);
    }

    pub(crate) fn maintain_index(
        &mut self,
        rebuild: bool,
        cx: &mut Context<Self>,
    ) -> Option<DesktopRequestId> {
        let root = PathBuf::from(self.navigation.selected()?);
        let intent = if rebuild {
            Intent::IndexRebuild { root }
        } else {
            Intent::IndexRefresh { root }
        };
        let id = self
            .services
            .worker()
            .submit(intent, Lane::User, None, None);
        cx.notify();
        Some(id)
    }

    pub(in crate::desktop::view) fn remove_repository(
        &mut self,
        root: RepoKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let origin = self.remove_focus.clone();
        // The dialog takes focus before the next frame.
        self.last_region = Some(Region::Content);
        confirm(
            Intent::RepoRemove {
                root: PathBuf::from(root),
            },
            origin,
            &self.services,
            window,
            cx,
        );
    }

    pub(in crate::desktop::view) fn cancel_request(
        &mut self,
        id: DesktopRequestId,
        cx: &mut Context<Self>,
    ) {
        self.services.worker().cancel(id);
        cx.notify();
    }

    pub(in crate::desktop::view) fn dismiss_request(
        &mut self,
        id: DesktopRequestId,
        cx: &mut Context<Self>,
    ) {
        let dismissed = self.services.operations().update(cx, |operations, cx| {
            operations.dismiss(id);
            cx.notify();
            operations
                .request(id)
                .is_some_and(|request| matches!(request.state, RequestState::Dismissed))
        });
        // A pending save dismissed is over (ruling R22).
        if dismissed {
            self.services
                .drafts()
                .update(cx, |registry, cx| registry.dismissed(id, cx));
        }
    }

    /// A failing session's Retry: writes its newest revision now. The
    /// entry shows how it went, and goes once a write lands.
    pub(in crate::desktop::view) fn retry_draft_flush(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) {
        self.services
            .drafts()
            .update(cx, |registry, cx| registry.flush_now(id, cx))
            .detach();
    }

    pub(in crate::desktop::view) fn resume(
        &mut self,
        root: RepoKey,
        operation_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Ok(operation_id) = OperationId::parse(operation_id) else {
            return;
        };
        self.services.worker().submit(
            Intent::OperationResume {
                root: PathBuf::from(root),
                operation_id,
            },
            Lane::User,
            None,
            None,
        );
        cx.notify();
    }

    pub(in crate::desktop::view) fn abandon(
        &mut self,
        root: RepoKey,
        operation_id: &str,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(operation_id) = OperationId::parse(operation_id) else {
            return;
        };
        window.focus(&origin, cx);
        self.last_region = Some(Region::Operations);
        confirm(
            Intent::OperationAbandon {
                root: PathBuf::from(root),
                operation_id,
            },
            origin,
            &self.services,
            window,
            cx,
        );
    }

    pub(in crate::desktop::view) fn abandon_focus_for(
        &mut self,
        root: &RepoKey,
        operation_id: &str,
        cx: &mut Context<Self>,
    ) -> FocusHandle {
        let key = (root.clone(), operation_id.to_owned());
        self.abandon_drawn.insert(key.clone());
        self.abandon_focus
            .entry(key)
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    pub(in crate::desktop::view) fn control_focus_for(
        &mut self,
        id: String,
        cx: &mut Context<Self>,
    ) -> FocusHandle {
        self.control_drawn.insert(id.clone());
        self.control_focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    /// Dropped controls restore focus through the workspace's focus-lost handler.
    pub(super) fn prune_abandon_focus(&mut self) {
        let drawn = std::mem::take(&mut self.abandon_drawn);
        self.abandon_focus.retain(|key, _| drawn.contains(key));
        let drawn = std::mem::take(&mut self.control_drawn);
        self.control_focus.retain(|key, _| drawn.contains(key));
    }

    #[cfg(test)]
    pub(crate) fn abandon_focus(&self, root: &str, operation_id: &str) -> Option<FocusHandle> {
        self.abandon_focus
            .get(&(root.to_owned(), operation_id.to_owned()))
            .cloned()
    }

    fn protected_generation(&self, id: DesktopRequestId, cx: &App) -> bool {
        self.services
            .operations()
            .read(cx)
            .request(id)
            .is_some_and(|request| {
                matches!(
                    request.intent,
                    Intent::KeyGenerate {
                        protected: true,
                        ..
                    }
                )
            })
    }

    pub(in crate::desktop::view) fn run_recovery(
        &mut self,
        id: DesktopRequestId,
        n: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let step = {
            let operations = self.services.operations().read(cx);
            let Some(RequestState::Finished(_, _, _, recovery, _)) =
                operations.request(id).map(|request| &request.state)
            else {
                return;
            };
            let Some(action) = recovery.get(n) else {
                return;
            };
            operations.recovery_step(id, action)
        };
        let origin = self
            .control_focus
            .get(&recovery_id(id, n))
            .cloned()
            .unwrap_or_else(|| self.regions[Region::Operations.index()].clone());
        window.focus(&origin, cx);
        self.last_region = Some(Region::Operations);
        match step {
            Some(RecoveryStep::Submit(intent)) => {
                self.services
                    .worker()
                    .submit(intent, Lane::User, None, None);
            }
            Some(RecoveryStep::Confirm(intent)) => {
                confirm(intent, origin, &self.services, window, cx);
            }
            Some(RecoveryStep::Retry(id)) if self.protected_generation(id, cx) => {
                self.retry_generation_in_settings(id, window, cx);
            }
            Some(RecoveryStep::Retry(id)) => {
                self.services.worker().retry(id, None);
            }
            Some(RecoveryStep::Prepare(id)) => {
                prepare_again(id, origin, &self.services, window, cx);
            }
            Some(RecoveryStep::Cancel(id)) => {
                self.services.worker().cancel(id);
            }
            Some(RecoveryStep::Inspect(root)) => self.inspect_in_settings(root, window, cx),
            Some(RecoveryStep::IdentitySet(root)) => self.identity_in_settings(root, window, cx),
            Some(RecoveryStep::CheckHost { root, authority }) => {
                self.check_host_in_settings(root, authority, window, cx)
            }
            None => {}
        }
        cx.notify();
    }
}
