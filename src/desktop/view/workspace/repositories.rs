//! Repository list, selection and resolution transitions.

use super::Workspace;
use crate::desktop::{
    model::stamp::{RepoKey, Stamp, ViewScope},
    services::{
        ReadKind,
        requests::{Intent, Lane},
    },
};
use gpui_kit::Context;
use std::path::PathBuf;

impl Workspace {
    /// Reads the repository list again.
    #[allow(clippy::result_large_err)]
    pub(crate) fn load_repositories(&mut self, cx: &mut Context<Self>) {
        let stamp = self.navigation.load_repositories();
        self.list_read = Some(self.services.read(
            ReadKind::Repositories,
            stamp,
            |service| service.list_repositories(),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.navigation.apply_repositories(stamp, result) {
                    // Only a replacing list drops roots (ruling R-T13a).
                    for root in this.navigation.take_dropped() {
                        this.drop_repository(&root, cx);
                    }
                    if this.navigation.selected().is_none() {
                        this.clear_lists(cx);
                    }
                    if let Some(key) = this.navigation.take_vanished() {
                        let stamp = this.navigation.retry_resolution(key.clone());
                        this.clear_lists(cx);
                        this.resolve(key, stamp, cx);
                    }
                    this.repositories_loaded(cx);
                    cx.notify();
                    this.refresh_activity(cx);
                }
            },
        ));
        cx.notify();
    }

    pub(super) fn listed(&self) -> Option<Vec<RepoKey>> {
        self.navigation
            .repositories()
            .rows()
            .map(|rows| rows.iter().map(|entry| entry.key().clone()).collect())
    }

    /// Reads every listed repository's pending operations again.
    pub(super) fn reload_pending(&mut self, cx: &mut Context<Self>) {
        for root in self.listed().unwrap_or_default() {
            self.load_pending(root, cx);
        }
    }

    /// Refresh on launch once; read pending operations only for new roots.
    fn repositories_loaded(&mut self, cx: &mut Context<Self>) {
        let Some(roots) = self.listed() else {
            return;
        };
        // The launch poll, and the schedule of a repository that came or
        // went (D6 B6, R-16).
        self.services.repositories_listed(&roots, cx);
        if !self.launched {
            self.launched = true;
            if self.services.launch_refresh() {
                for root in &roots {
                    self.services.worker().submit(
                        Intent::IndexRefresh {
                            root: PathBuf::from(root),
                        },
                        Lane::Background,
                        None,
                        None,
                    );
                }
            }
        }
        self.services.operations().update(cx, |operations, cx| {
            operations.retain_pending(&roots);
            cx.notify();
        });
        self.pending_reads.retain(|root, _| roots.contains(root));
        self.retain_titles(&roots);
        // A removed repository's conflict offers go, and a read of them
        // still running lands on nothing.
        let gone: Vec<RepoKey> = self
            .pending_conflicts
            .keys()
            .chain(self.pending_conflict_reads.keys())
            .filter(|root| !roots.contains(root))
            .cloned()
            .collect();
        for root in gone {
            self.conflict_generations
                .next(ViewScope::PendingConflicts(root.clone()));
            self.pending_conflicts.remove(&root);
            self.pending_conflict_reads.remove(&root);
        }
        for root in roots {
            if !self.pending_reads.contains_key(&root) {
                self.load_pending(root, cx);
            }
        }
    }

    /// Selection also reloads the list to show current configuration/index state.
    pub(crate) fn select_repository(&mut self, key: String, cx: &mut Context<Self>) {
        let stamp = self.navigation.select(key.clone());
        self.clear_lists(cx);
        self.resolve(key, stamp, cx);
        self.load_repositories(cx);
    }

    pub(crate) fn retry_resolution(&mut self, key: String, cx: &mut Context<Self>) {
        let stamp = self.navigation.retry_resolution(key.clone());
        self.clear_lists(cx);
        self.resolve(key, stamp, cx);
        self.load_repositories(cx);
    }

    /// The panel's pane, whose rows a test compares with the model's.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn repositories_pane(
        &self,
    ) -> &crate::desktop::view::repositories::RepositoriesPane {
        &self.repositories_pane
    }

    /// Applies `list` as if a list read had answered it: the incomplete
    /// list the library never answers today (fact 57).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn apply_repository_list(
        &mut self,
        list: manyhands::repository::RepositoryListDto,
        cx: &mut Context<Self>,
    ) {
        let stamp = self.navigation.load_repositories();
        self.navigation
            .apply_repositories(stamp, crate::desktop::services::ReadResult::Ok(list));
        cx.notify();
    }

    #[allow(clippy::result_large_err)]
    fn resolve(&mut self, key: String, stamp: Stamp, cx: &mut Context<Self>) {
        let root = PathBuf::from(key);
        self.resolve_read = Some(self.services.read(
            ReadKind::Resolve,
            stamp,
            move |service| service.resolve_repository(&root),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if this.navigation.apply_resolution(stamp, result) {
                    this.load_lists(cx);
                    cx.notify();
                    this.refresh_activity(cx);
                }
            },
        ));
        cx.notify();
    }
}
