//! Which of a repository's pending rows hold a conflict waiting to be
//! resolved, here or outside (D5 design, B5): Review changes is offered on
//! them, after a restart too. The rows land first; this read follows, with
//! its own stamp, so neither the rows nor the lifecycle lock (R-21) wait
//! for it. A row it cannot read is left out.
//!
//! - A local synchronization (`context_sync_local`) by its inspection.
//! - A remote synchronization of an item's context, and a closure's or a
//!   promotion's parent, by `show_conflict`, which reads a parent as its
//!   context child (C5 decision 34).
//!
//! A lifecycle's child may be listed beside its parent. Its conflict is
//! offered once, on the parent's row, so that its resolution sends the
//! lifecycle on (R-10): a child's row is left out when a parent's read names
//! the child's operation, or, when a parent of the same item could not be
//! read, by the item.

// A read closure returns the library's `ReadError`, which carries its whole
// scope by value, as the library's own read module allows.
#![allow(clippy::result_large_err)]

use std::path::PathBuf;

use gpui_kit::{Context, Task};
use manyhands::{
    canonical::ItemId,
    repository::{
        ConflictReadNextAction, ItemDtoKind, LocalContextSyncNextAction, OperationAction,
        OperationFamily, OperationId, OperationListDto,
    },
    results::ResultCode,
};

use super::{ReadKind, ReadResult, Services, conflicts::worktree};
use crate::desktop::model::stamp::Stamp;

/// Which pending row holds a conflict, by what the sheet reads it as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PendingSource {
    /// A local synchronization of an item's work context with main.
    Local,
    /// A remote synchronization of an item's work context.
    Synchronization,
    /// A closure's or a promotion's parent, whose child the conflict verbs
    /// take by its ID (C5 decision 34).
    Close,
    Promote,
}

impl PendingSource {
    fn lifecycle(self) -> bool {
        matches!(self, Self::Close | Self::Promote)
    }
}

/// A pending row whose conflict waits to be resolved, here or outside
/// (B5): its operation and item, and the item's kind, short code and work
/// folder, as the item reads. It holds no item text.
#[derive(Clone)]
pub(crate) struct PendingConflict {
    pub operation_id: OperationId,
    pub source: PendingSource,
    pub item: ItemId,
    pub kind: ItemDtoKind,
    pub slug: Option<String>,
    pub folder: Option<PathBuf>,
}

/// A row that may hold a conflict.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Candidate {
    pub operation_id: OperationId,
    pub source: PendingSource,
    pub item: ItemId,
}

/// What a candidate's conflict read found.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Probe {
    /// A conflict waits, of this operation: the row's own, or a parent's
    /// child's.
    Waits(OperationId),
    /// Nothing waits.
    Settled,
    /// The read failed or was busy.
    Unread,
}

/// The rows of `list` that may hold a conflict, in its order.
pub(crate) fn candidates(list: &OperationListDto) -> Vec<Candidate> {
    list.items
        .iter()
        .filter_map(|row| {
            let source = match (row.family, row.action) {
                (OperationFamily::Local, OperationAction::ContextSyncLocal) => PendingSource::Local,
                (OperationFamily::Remote, OperationAction::SynchronizeContext) => {
                    PendingSource::Synchronization
                }
                (OperationFamily::Remote, OperationAction::Close) => PendingSource::Close,
                (OperationFamily::Remote, OperationAction::Promote) => PendingSource::Promote,
                _ => return None,
            };
            Some(Candidate {
                operation_id: OperationId::parse(row.operation_id.as_deref()?).ok()?,
                source,
                item: row.item_id.as_deref()?.parse().ok()?,
            })
        })
        .collect()
}

/// Which candidates, by index, offer Review changes, given each one's
/// `probes`: every one whose conflict waits, but a lifecycle child's row
/// beside its parent's.
pub(crate) fn offered(candidates: &[Candidate], probes: &[Probe]) -> Vec<usize> {
    let parents: Vec<(&Candidate, &Probe)> = candidates
        .iter()
        .zip(probes)
        .filter(|(candidate, _)| candidate.source.lifecycle())
        .collect();
    let child_of_a_parent = |candidate: &Candidate| {
        candidate.source == PendingSource::Synchronization
            && parents.iter().any(|(parent, probe)| match probe {
                Probe::Waits(child) => *child == candidate.operation_id,
                Probe::Unread => parent.item == candidate.item,
                Probe::Settled => false,
            })
    };
    candidates
        .iter()
        .zip(probes)
        .enumerate()
        .filter(|(_, (candidate, probe))| {
            matches!(probe, Probe::Waits(_)) && !child_of_a_parent(candidate)
        })
        .map(|(index, _)| index)
        .collect()
}

/// What a candidate's read found. A parent with no child conflict answers
/// `invalid_input` (`Lookup::NoConflict`), and an operation that is gone
/// `operation_not_found`: neither has a conflict waiting. Busy and any
/// other failure leave it unread.
pub(crate) fn probe_of(read: ReadResult<Option<OperationId>>) -> Probe {
    match read {
        ReadResult::Ok(Some(conflict)) => Probe::Waits(conflict),
        ReadResult::Ok(None) => Probe::Settled,
        ReadResult::Failed(failure)
            if matches!(
                failure.code,
                ResultCode::InvalidInput | ResultCode::OperationNotFound
            ) =>
        {
            Probe::Settled
        }
        ReadResult::Busy | ReadResult::Failed(_) => Probe::Unread,
    }
}

fn waits_local(next: LocalContextSyncNextAction) -> bool {
    matches!(
        next,
        LocalContextSyncNextAction::Resolve
            | LocalContextSyncNextAction::RetryResolution
            | LocalContextSyncNextAction::ExternalRepair
    )
}

fn waits_remote(next: ConflictReadNextAction) -> bool {
    matches!(
        next,
        ConflictReadNextAction::Resolve
            | ConflictReadNextAction::RetryResolution
            | ConflictReadNextAction::ExternalRepair
    )
}

impl Services {
    /// Reads which rows of `list`, the pending operations of `root`, hold
    /// a conflict to review, and hands them to `apply`, with `stamp`. A
    /// repository that cannot be read gives none.
    pub(crate) fn read_pending_conflicts<V: 'static>(
        &self,
        root: PathBuf,
        list: OperationListDto,
        stamp: Stamp,
        cx: &mut Context<V>,
        apply: impl FnOnce(&mut V, Stamp, Vec<PendingConflict>, &mut Context<V>) + 'static,
    ) -> Task<()> {
        self.read_job(
            stamp,
            move |reader| async move {
                let candidates = candidates(&list);
                if candidates.is_empty() {
                    return Vec::new();
                }
                let kind = ReadKind::PendingConflicts;
                let ReadResult::Ok(repo) = reader
                    .call(kind, |service| service.resolve_repository(&root))
                    .await
                else {
                    return Vec::new();
                };
                let mut probes = Vec::new();
                for candidate in &candidates {
                    let operation = candidate.operation_id;
                    let read = match candidate.source {
                        PendingSource::Local => {
                            reader
                                .call(kind, |service| {
                                    let inspection =
                                        service.inspect_local_context_sync(&repo, operation)?;
                                    Ok(waits_local(inspection.next_action).then_some(operation))
                                })
                                .await
                        }
                        // Through the reader, as the sheet's read is, so a
                        // test's canned conflict reaches it (Task 16, fix 5).
                        _ => match reader.show_conflict_as(kind, &repo, operation).await {
                            ReadResult::Ok(conflict) => {
                                let named = OperationId::parse(&conflict.operation_id).ok();
                                ReadResult::Ok(
                                    waits_remote(conflict.next_action)
                                        .then_some(named.unwrap_or(operation)),
                                )
                            }
                            ReadResult::Busy => ReadResult::Busy,
                            ReadResult::Failed(failure) => ReadResult::Failed(failure),
                        },
                    };
                    probes.push(probe_of(read));
                }
                let mut conflicts = Vec::new();
                for index in offered(&candidates, &probes) {
                    let candidate = &candidates[index];
                    let item = candidate.item.clone();
                    let ReadResult::Ok(shown) = reader
                        .call(kind, |service| service.show_item(&repo, &item))
                        .await
                    else {
                        continue;
                    };
                    conflicts.push(PendingConflict {
                        operation_id: candidate.operation_id,
                        source: candidate.source,
                        item: candidate.item.clone(),
                        kind: shown.kind,
                        slug: shown.slug,
                        folder: worktree(&shown.context.worktree),
                    });
                }
                conflicts
            },
            cx,
            apply,
        )
    }
}

#[cfg(test)]
#[path = "pending_conflicts_tests.rs"]
mod tests;
