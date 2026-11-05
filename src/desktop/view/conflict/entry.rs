//! The sheet's entry points (D5 design, B5; decision 16): what opening it
//! takes, from an operation area entry whose answer stopped on a conflict
//! (a share, a post, a local synchronization or a lifecycle, L1's
//! `ResolveChildConflict`) or from a pending row whose conflict waits to be
//! resolved, after a restart too. The item tab's banner and the lifecycle
//! dialog know their item, and build their `Opening` themselves.
//!
//! An entry names the item by its short code, or its short ID, and never
//! its title (Privacy); the sheet reads the item's work folder when the
//! entry does not know it.

use std::path::PathBuf;

use gpui_kit::{App, Entity, FocusHandle, Window};
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, ItemDtoKind, MutationDataDto, OperationId},
};

use super::{Opening, ResolveSheet, SheetSource, open};
use crate::desktop::{
    model::operations::Operations,
    services::{
        Services,
        conflicts::{PendingConflict, PendingSource},
        requests::{DesktopRequest, Intent},
    },
    view::{
        lifecycle::{Heading, short_id},
        operations::scope_name,
    },
};

/// What an entry opens the sheet on.
pub(crate) struct Entry {
    source: SheetSource,
    root: PathBuf,
    operation_id: OperationId,
    item: ItemId,
    kind: ItemDtoKind,
    slug: Option<String>,
    folder: Option<PathBuf>,
}

fn kind_of(kind: AuthoringKind) -> ItemDtoKind {
    match kind {
        AuthoringKind::Document => ItemDtoKind::Document,
        AuthoringKind::Ticket => ItemDtoKind::Ticket,
    }
}

impl Entry {
    /// The conflict the finished request `request` last stopped on, when
    /// it did: a share's, a post's, a local synchronization's, or a
    /// lifecycle's child, as the close's or merge's this session holds, or,
    /// for a resume, as the lifecycle row's (R-10).
    pub(crate) fn of_request(request: &DesktopRequest, operations: &Operations) -> Option<Self> {
        let operation_id = operations.review(request.id)?;
        let (source, root, kind, item) = match &request.intent {
            Intent::ItemSync { root, kind, id } => (SheetSource::Share, root, kind_of(*kind), id),
            Intent::CommentAdd { root, kind, id, .. } => {
                (SheetSource::Post, root, kind_of(*kind), id)
            }
            Intent::LocalContextSync { root, kind, id, .. } => {
                (SheetSource::Local, root, kind_of(*kind), id)
            }
            Intent::TicketClose { root, id } => (
                SheetSource::Close(Some(request.id)),
                root,
                ItemDtoKind::Ticket,
                id,
            ),
            Intent::DocumentPromote { root, id } => (
                SheetSource::Merge(Some(request.id)),
                root,
                ItemDtoKind::Document,
                id,
            ),
            Intent::OperationResume { root, .. } => {
                let Some(MutationDataDto::Lifecycle(data)) = operations
                    .lifecycle_answer(request.id)
                    .and_then(|envelope| envelope.data.as_ref())
                else {
                    return None;
                };
                let item: ItemId = data.item_id.parse().ok()?;
                let source = match data.kind {
                    ItemDtoKind::Ticket => SheetSource::Close(None),
                    ItemDtoKind::Document => SheetSource::Merge(None),
                };
                return Some(Self {
                    source,
                    root: root.clone(),
                    operation_id,
                    item,
                    kind: data.kind,
                    slug: None,
                    folder: None,
                });
            }
            _ => return None,
        };
        Some(Self {
            source,
            root: root.clone(),
            operation_id,
            item: item.clone(),
            kind,
            slug: None,
            folder: None,
        })
    }

    /// A pending row of `root` whose conflict waits to be resolved. A
    /// lifecycle's parent is read as its child's, whose resolution resumes
    /// the row (R-10).
    pub(crate) fn of_pending(root: PathBuf, conflict: &PendingConflict) -> Self {
        let source = match conflict.source {
            PendingSource::Local => SheetSource::Local,
            PendingSource::Synchronization => SheetSource::Share,
            PendingSource::Close => SheetSource::Close(None),
            PendingSource::Promote => SheetSource::Merge(None),
        };
        Self {
            source,
            root,
            operation_id: conflict.operation_id,
            item: conflict.item.clone(),
            kind: conflict.kind,
            slug: conflict.slug.clone(),
            folder: conflict.folder.clone(),
        }
    }

    /// The repository the conflict is in.
    pub(crate) fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Opens the sheet on the entry, in the repository whose main branch
    /// and publication remote the workspace knows as `known`; focus goes
    /// back to `origin`.
    pub(crate) fn open(
        self,
        (primary, remote): (Option<String>, Option<Option<String>>),
        origin: FocusHandle,
        services: &Services,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Entity<ResolveSheet>> {
        let heading = Heading {
            kind: self.kind,
            short: self
                .slug
                .unwrap_or_else(|| short_id(&self.item.to_string())),
            title: None,
            repository: scope_name(Some(&self.root)),
            branch: None,
            primary,
            remote,
        };
        let opening = Opening {
            source: self.source,
            root: self.root,
            operation_id: self.operation_id,
            item: self.item,
            heading,
            folder: self.folder,
            origin,
        };
        open(opening, services, window, cx)
    }
}
