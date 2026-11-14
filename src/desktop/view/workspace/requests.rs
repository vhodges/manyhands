//! Worker completions, pending reads and explicit request/recovery actions.

use super::{Region, Workspace};
use crate::desktop::{
    model::{
        activity::rows::RowKey,
        operations::RecoveryStep,
        stamp::{RepoKey, SettingsPage, ViewScope},
    },
    services::{
        ReadKind, ReadResult,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
        worker::CancelResult,
    },
    view::{
        confirm::{confirm, prepare_again},
        item::ItemRoute,
        settings::SettingsRoute,
    },
};
use gpui_kit::{
    App, Context, FocusHandle, Window,
    component::{Root, WindowExt},
};
use manyhands::{
    drafts::DraftId,
    repository::{OperationId, OperationListDto},
};
use std::path::PathBuf;

/// `route`, owned, for the window's update that opens it.
fn owned(route: &ItemRoute) -> ItemRoute {
    match route {
        ItemRoute::Remotes(root) => ItemRoute::Remotes(root.clone()),
        ItemRoute::SharingAndKeys => ItemRoute::SharingAndKeys,
        ItemRoute::CheckHost { root, authority } => ItemRoute::CheckHost {
            root: root.clone(),
            authority: authority.clone(),
        },
        ItemRoute::SetIdentity(root) => ItemRoute::SetIdentity(root.clone()),
        ItemRoute::Holder { root, request } => ItemRoute::Holder {
            root: root.clone(),
            request: *request,
        },
    }
}

impl Workspace {
    /// Resolves the root before reading its pending operations.
    #[allow(clippy::result_large_err)]
    pub(crate) fn load_pending(&mut self, root: RepoKey, cx: &mut Context<Self>) {
        let stamp = self
            .services
            .operations()
            .update(cx, |operations, _| operations.load_pending(root.clone()));
        let path = PathBuf::from(&root);
        let read_root = root.clone();
        let read = self.services.read(
            ReadKind::Operations,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&path)?;
                service.list_operations(&repo)
            },
            cx,
            move |this: &mut Self, stamp, result, cx| {
                let list = match &result {
                    ReadResult::Ok(list) => Some(list.clone()),
                    ReadResult::Busy | ReadResult::Failed(_) => None,
                };
                let applied = this.services.operations().update(cx, |operations, cx| {
                    let applied = operations.apply_pending(stamp, result);
                    if applied {
                        cx.notify();
                    }
                    applied
                });
                if applied {
                    this.read_pending_conflicts(read_root, list, cx);
                }
            },
        );
        self.pending_reads.insert(root, read);
        cx.notify();
    }

    /// Which of `root`'s pending rows, `list` as just read, hold a conflict
    /// to review (D5 B5): a read of its own after the rows, stamped, so
    /// the rows and the lifecycle lock never wait for it. Rows that could
    /// not be read offer none.
    fn read_pending_conflicts(
        &mut self,
        root: RepoKey,
        list: Option<OperationListDto>,
        cx: &mut Context<Self>,
    ) {
        let stamp = self
            .conflict_generations
            .next(ViewScope::PendingConflicts(root.clone()));
        let Some(list) = list else {
            self.pending_conflicts.remove(&root);
            self.pending_conflict_reads.remove(&root);
            return;
        };
        let read = self.services.read_pending_conflicts(
            PathBuf::from(&root),
            list,
            stamp,
            cx,
            |this: &mut Self, stamp, conflicts, cx| {
                if !this.conflict_generations.is_current(&stamp) {
                    return;
                }
                if let ViewScope::PendingConflicts(root) = stamp.scope {
                    this.pending_conflicts.insert(root, conflicts);
                    cx.notify();
                    // A conflict that waits changes the row that offers its
                    // review (N10).
                    this.refresh_activity(cx);
                }
            },
        );
        self.pending_conflict_reads.insert(root, read);
    }

    /// Coalesce each batch's changed roots, while keeping prompt/quit order.
    pub(super) fn operations_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
        self.refresh_activity(cx);
        // Confirmation observers close their dialogs before a prompt opens.
        // The Kit closes the top dialog only.
        cx.defer_in(window, |this, window, cx| this.show_prompt(window, cx));
        self.stops_to_republish_answered(window, cx);
        self.quit_if_finished(cx);
        let changed = self
            .services
            .operations()
            .update(cx, |operations, _| operations.take_changed());
        if changed.is_empty() {
            return;
        }
        for root in &changed.removed {
            // The user's confirmation said which tabs close (Part C fixes 1
            // and 2); a removal found already done was not confirmed.
            if changed.confirmed.contains(root) {
                self.repository_removed(root, cx);
            } else {
                self.drop_repository(root, cx);
            }
            if self.navigation.forget(root) {
                self.clear_lists(cx);
            }
        }
        // Before the lists reload, so the rebuilt tree shows each place
        // (R-27).
        for (root, path) in &changed.reveal {
            self.documents.reveal(root, path);
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

    /// A tab's share or post control asked for a place in the workspace
    /// (`ItemRoute`). The subscription brings no window, so the place
    /// opens once the update that asked has ended, in this workspace's
    /// window (`open_route`).
    pub(in crate::desktop::view) fn item_route(
        &mut self,
        route: &ItemRoute,
        cx: &mut Context<Self>,
    ) {
        let route = owned(route);
        self.in_window(cx, move |workspace, window, cx| {
            workspace.open_route(route, window, cx)
        });
    }

    /// Opens the place `route` names (design, Outcomes and routes): Share…
    /// and Set up sharing, Settings › Remotes on the repository; Open
    /// Sharing & keys, the Keys page; Check host, the host named, or with
    /// none the repository's Hosts page (K-3); Set identity, its identity
    /// form; Show it, the entry of the request that may hold the
    /// repository, or with none its pending operations.
    pub(in crate::desktop::view) fn open_route(
        &mut self,
        route: ItemRoute,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match route {
            ItemRoute::Remotes(root) => self.open_remotes(root, window, cx),
            ItemRoute::SharingAndKeys => self.open_settings(SettingsPage::Keys, window, cx),
            ItemRoute::CheckHost {
                root,
                authority: Some(authority),
            } => self.check_host_in_settings(root, authority, window, cx),
            ItemRoute::CheckHost {
                root,
                authority: None,
            } => self.open_hosts(root, window, cx),
            ItemRoute::SetIdentity(root) => self.identity_in_settings(root, window, cx),
            ItemRoute::Holder { root, request } => self.show_holder(root, request, window, cx),
        }
    }

    /// The Hosts page on the repository at `root`, naming no host to
    /// check: a comment's host stop has no host data (K-3).
    fn open_hosts(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.show_settings(SettingsRoute::Hosts(root), None, window, cx);
    }

    /// Show it, on `busy` (Busy repositories): reveals Activity and
    /// highlights, and focuses the list on, the row of `request`, the
    /// request of this session that may hold the repository (a running
    /// retirement is its S1 row, R-51); with none, or one no longer listed,
    /// the first pending row of `root`, whose pending operations are read
    /// again (C3). The filter, when it hides the row, is turned off here.
    pub(in crate::desktop::view) fn show_holder(
        &mut self,
        root: PathBuf,
        request: Option<DesktopRequestId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let listed = |key: &RowKey| {
            self.activity
                .log
                .rows()
                .iter()
                .any(|row| &row.row.key == key)
        };
        let holder = request
            .map(RowKey::Request)
            .filter(|key| listed(key))
            .or_else(|| {
                let root = root.to_str()?;
                self.activity
                    .log
                    .rows()
                    .iter()
                    .map(|row| &row.row.key)
                    .find(|key| matches!(key, RowKey::Pending { root: of, .. } if of == root))
                    .cloned()
            });
        if let Some(key) = &holder {
            self.show_row_through_filter(key);
            self.activity.pane.highlight_when_shown(key);
        }
        if request.is_none_or(|id| holder != Some(RowKey::Request(id)))
            && let Some(root) = root.to_str()
        {
            self.load_pending(root.to_owned(), cx);
        }
        // Activity may be hidden, or behind another tab (R-10): focusing its
        // region reveals it and focuses the list.
        self.focus_region(Region::Operations, window, cx);
        cx.notify();
    }

    /// Runs `f` on this workspace with its window, once the update under
    /// way has ended: the window whose root view is this workspace. Root is
    /// not updated around `f`, which may open a sheet or a dialog.
    fn in_window(
        &self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let id = cx.entity_id();
        let workspace = cx.entity().downgrade();
        let Some(window) = cx.windows().into_iter().find(|window| {
            window.downcast::<Root>().is_some_and(|root| {
                root.read(cx)
                    .is_ok_and(|root| root.view().entity_id() == id)
            })
        }) else {
            return;
        };
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                workspace.update(cx, |workspace, cx| f(workspace, window, cx))
            });
        });
    }

    /// Settings › Remotes, on the repository at `root` even when the page
    /// works on another repository (O-1).
    pub(in crate::desktop::view) fn open_remotes(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_settings(SettingsRoute::Remotes(root), None, window, cx);
    }

    /// Whether a share of the primary branch of `root` is queued, waiting
    /// out `busy`, or running: the worker's own account, so a press made
    /// before the request model has heard of the last one still finds it.
    pub(in crate::desktop::view) fn sharing_main(&self, root: &str) -> bool {
        let root = PathBuf::from(root);
        self.services.worker().in_flight(
            |intent| matches!(intent, Intent::RepoSync { root: asked } if *asked == root),
        )
    }

    /// Share main pressed on the repository at `root`: `repo sync` when it
    /// has a publication remote (`remote`) and no share of it is queued or
    /// running; Settings › Remotes when it has none.
    pub(in crate::desktop::view) fn share_main(
        &mut self,
        root: RepoKey,
        remote: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !remote {
            self.open_remotes(PathBuf::from(root), window, cx);
            return;
        }
        if self.sharing_main(&root) {
            return;
        }
        self.services.worker().submit(
            Intent::RepoSync {
                root: PathBuf::from(root),
            },
            Lane::User,
            None,
            None,
        );
        cx.notify();
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

    /// Remove from Manyhands… of the repository at `root`: the dialog,
    /// which gives focus back to `restore`, its caller's (R-12). It refuses
    /// only under a dialog; its entry points refuse under the sheet too.
    pub(in crate::desktop::view) fn remove_repository(
        &mut self,
        root: RepoKey,
        restore: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        // The dialog takes focus before the next frame: the fallback is the
        // caller's region.
        self.last_region = Some(self.region_of(&restore, window));
        confirm(
            Intent::RepoRemove {
                root: PathBuf::from(root),
            },
            restore,
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

    /// Stop at a safe point (R-1): cancels the accepted request's token,
    /// so it stops at its next safe point. Once signalled it reads
    /// Stopping… until it ends.
    pub(in crate::desktop::view) fn stop_request(
        &mut self,
        id: DesktopRequestId,
        cx: &mut Context<Self>,
    ) {
        if self.services.worker().cancel(id) == CancelResult::Signalled {
            self.services.operations().update(cx, |operations, cx| {
                operations.stopping(id);
                cx.notify();
            });
        }
        cx.notify();
    }

    /// A stopped share's Share: a new request of the same share, under a
    /// new request ID, since a stop is final (fact 12). Nothing is sent
    /// while a share of the same target is queued or running.
    pub(in crate::desktop::view) fn share_request_again(
        &mut self,
        id: DesktopRequestId,
        cx: &mut Context<Self>,
    ) {
        let Some(intent) = self
            .services
            .operations()
            .read(cx)
            .request(id)
            .map(|request| request.intent.clone())
        else {
            return;
        };
        let same = |asked: &Intent| match (asked, &intent) {
            (Intent::RepoSync { root: asked }, Intent::RepoSync { root }) => asked == root,
            (
                Intent::ItemSync {
                    root: asked,
                    kind: asked_kind,
                    id: asked_id,
                },
                Intent::ItemSync { root, kind, id },
            ) => asked == root && asked_kind == kind && asked_id == id,
            _ => false,
        };
        if !matches!(intent, Intent::RepoSync { .. } | Intent::ItemSync { .. })
            || self.services.worker().in_flight(same)
        {
            return;
        }
        self.services
            .worker()
            .submit(intent, Lane::User, None, None);
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
        // A dismissed run's follow-ups go (review M4), but for one a
        // republish sends on (ruling (b)).
        if dismissed {
            self.services.resends().forget(id);
        }
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
        self.services.resume(&root, operation_id, cx);
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
        origin: FocusHandle,
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
