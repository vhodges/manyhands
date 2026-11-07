//! A conflict's reads (D5 design, B1), made into the sheet's model on the
//! background executor and returned with the stamp they were asked under,
//! so a late read never replaces a newer one.
//!
//! - Remote: `show_conflict` of the operation: a synchronization's, a
//!   comment's or a lifecycle's (C5 decision 34).
//! - Local: `inspect_local_context_sync` for state, eligibility and
//!   observation, then `read_local_context_conflict` of each token whose
//!   content is given, unless the set is already external.
//!
//! And what the sheet's entry points read (B5, B6): the observation Repair
//! locally… sends its local synchronization with, and the item's work
//! folder. Which pending rows hold a conflict is `pending_conflicts`.

// A read closure returns the library's `ReadError`, which carries its whole
// scope by value, as the library's own read module allows.
#![allow(clippy::result_large_err)]

use std::path::{Path, PathBuf};

use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, LocalContextConflictContentDto, OperationId},
};

use super::{ReadKind, ReadResult, Reader, Services};
use crate::desktop::model::{
    conflict::{ConflictModel, local_set_is_external},
    stamp::Stamp,
};

/// Which synchronization a conflict is of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ConflictSource {
    /// A share's, a post's or a lifecycle child's, through the remote.
    Remote,
    /// A local synchronization of an item's work context with main.
    Local,
}

impl Services {
    /// Reads the conflict of `operation_id` in the repository at `root`,
    /// and hands its model to `apply` on the asking entity, with `stamp`.
    pub(crate) fn read_conflict<V: 'static>(
        &self,
        source: ConflictSource,
        root: PathBuf,
        operation_id: OperationId,
        stamp: Stamp,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, ReadResult<ConflictModel>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        self.read_job(
            stamp,
            move |reader| async move {
                match source {
                    ConflictSource::Remote => remote(&reader, &root, operation_id).await,
                    ConflictSource::Local => local(&reader, &root, operation_id).await,
                }
            },
            cx,
            apply,
        )
    }
}

/// What `observe_local_context_sync` gave: the observation Repair locally…
/// sends its synchronization with (B6), and the item's work context.
pub(crate) struct LocalObservation {
    pub observation: String,
    pub worktree: PathBuf,
}

impl Services {
    /// Observes the work context of the item `id` of `kind` for a local
    /// synchronization (B6), and hands it to `apply`, with `stamp`.
    pub(crate) fn observe_local_sync<V: 'static>(
        &self,
        root: PathBuf,
        kind: AuthoringKind,
        id: ItemId,
        stamp: Stamp,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, ReadResult<LocalObservation>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        self.read(
            ReadKind::Conflict,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                let observed = service.observe_local_context_sync(&repo, kind, &id)?;
                Ok(LocalObservation {
                    observation: observed.observation,
                    worktree: PathBuf::from(observed.context_worktree),
                })
            },
            cx,
            apply,
        )
    }

    /// Reads the work folder of the item `id` of `root`, and hands it to
    /// `apply`, with `stamp`: a read of its own kind, apart from the
    /// conflict's and the tabs' item reads.
    pub(crate) fn read_item_folder<V: 'static>(
        &self,
        root: PathBuf,
        id: ItemId,
        stamp: Stamp,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, ReadResult<Option<PathBuf>>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        self.read(
            ReadKind::ConflictFolder,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                Ok(worktree(&service.show_item(&repo, &id)?.context.worktree))
            },
            cx,
            apply,
        )
    }
}

/// A work folder as the library names it: none when it names none.
pub(crate) fn worktree(path: &str) -> Option<PathBuf> {
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// `Ok`'s value, or the same answer for the read that was not made.
macro_rules! ok_or_return {
    ($read:expr) => {
        match $read {
            ReadResult::Ok(value) => value,
            other => return other.stopped().expect("not a value"),
        }
    };
}

async fn remote(
    reader: &Reader,
    root: &Path,
    operation_id: OperationId,
) -> ReadResult<ConflictModel> {
    let repo = ok_or_return!(
        reader
            .call(ReadKind::Conflict, |service| service
                .resolve_repository(root))
            .await
    );
    let conflict = ok_or_return!(reader.show_conflict(&repo, operation_id).await);
    ReadResult::Ok(ConflictModel::from_remote(&conflict))
}

async fn local(
    reader: &Reader,
    root: &Path,
    operation_id: OperationId,
) -> ReadResult<ConflictModel> {
    let repo = ok_or_return!(
        reader
            .call(ReadKind::Conflict, |service| service
                .resolve_repository(root))
            .await
    );
    let inspection = ok_or_return!(
        reader
            .call(ReadKind::Conflict, |service| {
                service.inspect_local_context_sync(&repo, operation_id)
            })
            .await
    );
    // An external set's content is not read: a failed content read must
    // not take the place of its guidance.
    let read = !local_set_is_external(&inspection);
    let mut contents: Vec<LocalContextConflictContentDto> = Vec::new();
    for entry in inspection
        .paths
        .iter()
        .filter(|entry| read && entry.content_available)
    {
        contents.push(ok_or_return!(
            reader
                .call(ReadKind::Conflict, |service| {
                    service.read_local_context_conflict(&repo, operation_id, &entry.token)
                })
                .await
        ));
    }
    ReadResult::Ok(ConflictModel::from_local(&inspection, &contents))
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflicts_tests.rs"]
mod tests;
