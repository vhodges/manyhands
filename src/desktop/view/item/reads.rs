//! One ordered load per tab, stamped by the existing ItemView model. A load
//! that lands has an attached draft's target state computed again.

use std::path::PathBuf;

use gpui_kit::Context;
use manyhands::repository::{DependencyDirection, ItemDtoKind};

use super::ItemPane;
use crate::desktop::{
    model::{item::ItemReads, navigation::OpenTarget, relationships::parent_to_read, stamp::Stamp},
    services::{
        ReadKind, ReadResult, Reader,
        requests::{Intent, Lane, RequestState},
    },
};

/// Resolve once, then item, comments, dependencies, children and parent. An
/// unsuccessful resolution/item read stops the sequence. The dependency tree
/// includes both directions at depth one, as in the existing read contract.
#[allow(clippy::result_large_err)]
async fn read_tab(reader: Reader, target: OpenTarget) -> ItemReads {
    let root = match &target {
        OpenTarget::Item { repo, .. } | OpenTarget::Inspect { repo, .. } => PathBuf::from(repo),
    };
    let repo = match reader
        .call(ReadKind::Resolve, |service| {
            service.resolve_repository(&root)
        })
        .await
    {
        ReadResult::Ok(repo) => repo,
        ReadResult::Busy => return ItemReads::stopped(&target, ReadResult::Busy),
        ReadResult::Failed(failure) => {
            return ItemReads::stopped(&target, ReadResult::Failed(failure));
        }
    };
    let (kind, id) = match &target {
        OpenTarget::Item { kind, id, .. } => (*kind, id),
        OpenTarget::Inspect { worktree, path, .. } => {
            let (worktree, path) = (PathBuf::from(worktree), PathBuf::from(path));
            let item = reader
                .call(ReadKind::Item, |service| {
                    service.show_path(&repo, Some(&worktree), &path)
                })
                .await;
            return ItemReads::stopped(&target, item);
        }
    };
    let item = reader
        .call(ReadKind::Item, |service| service.show_item(&repo, id))
        .await;
    let parent = match &item {
        ReadResult::Ok(item) => parent_to_read(item.parent.as_ref()),
        _ => return ItemReads::stopped(&target, item),
    };
    let comments = reader
        .call(ReadKind::Comments, |service| {
            service.list_comments(&repo, id)
        })
        .await;
    if kind != ItemDtoKind::Ticket {
        return ItemReads {
            item,
            comments: Some(comments),
            dependents: None,
            children: None,
            parent: None,
        };
    }
    let dependents = reader
        .call(ReadKind::Dependencies, |service| {
            service.ticket_dependencies(&repo, id, DependencyDirection::Both, Some(1))
        })
        .await;
    let children = reader
        .call(ReadKind::Children, |service| {
            service.ticket_children(&repo, id)
        })
        .await;
    let parent = match parent {
        Some(parent) => Some(
            reader
                .call(ReadKind::Item, |service| service.show_item(&repo, &parent))
                .await,
        ),
        None => None,
    };
    ItemReads {
        item,
        comments: Some(comments),
        dependents: Some(dependents),
        children: Some(children),
        parent,
    }
}

impl ItemPane {
    /// Reload keeps the shown state until the replacement arrives. The
    /// tab's images are cleared, so each is read again as it is now.
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) -> Stamp {
        self.images.clear(cx);
        let stamp = self.model.reload();
        self.read(stamp.clone(), cx);
        stamp
    }

    pub(super) fn read(&mut self, stamp: Stamp, cx: &mut Context<Self>) {
        let target = self.model.target().clone();
        self.read = Some(self.services.read_job(
            stamp,
            move |reader| read_tab(reader, target),
            cx,
            |this: &mut Self, stamp, reads, cx| {
                if this.model.apply_reads(stamp, reads) {
                    // An attached draft is classified against the new read;
                    // its text and base stay as they are.
                    this.draft.reads_landed();
                    cx.notify();
                }
            },
        ));
        cx.notify();
    }

    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing.is_some() {
            return;
        }
        let root = PathBuf::from(self.repo());
        let id =
            self.services
                .worker()
                .submit(Intent::IndexRefresh { root }, Lane::User, None, None);
        self.refreshing = Some(id);
        cx.notify();
    }

    /// Finished requests trigger Workspace's tab reload; unsent/cancelled ones
    /// reload here. A missing request is still queued in the worker channel.
    pub(super) fn operations_changed(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.refreshing else {
            return;
        };
        let state = self
            .services
            .operations()
            .read(cx)
            .request(id)
            .map(|request| request.state.clone());
        match state {
            None | Some(RequestState::Queued | RequestState::Running { .. }) => {}
            Some(RequestState::Finished(..)) => {
                self.refreshing = None;
                cx.notify();
            }
            Some(_) => {
                self.refreshing = None;
                self.reload(cx);
            }
        }
    }
}
