//! Repository list, selection and resolution transitions.

use super::Workspace;
use crate::desktop::{
    model::stamp::{RepoKey, Stamp},
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

    /// Refresh on launch once; read pending operations only for new roots.
    fn repositories_loaded(&mut self, cx: &mut Context<Self>) {
        let Some(roots) = self.listed() else {
            return;
        };
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
                }
            },
        ));
        cx.notify();
    }
}
