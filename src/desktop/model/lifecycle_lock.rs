//! The lifecycle lock (D5 design, A2 "Read-only while unfinished"; rulings
//! R-21 to R-23): while a close or a merge of an item is unfinished, every
//! tab of it is read-only, with "Closing…" or "Merging…".
//!
//! Three sources lock an item:
//!
//! - **Dialog.** A lifecycle dialog for it is open in any phase before its
//!   run is sent, until the dialog is dropped (R-21).
//! - **Worker.** The worker has a `TicketClose` or `DocumentPromote` of it,
//!   or an `OperationResume` of its lifecycle operation, queued or running
//!   (the caller asks the worker; `ops` names the operations).
//! - **Record.** L2's row for it is in a locking step: every non-terminal
//!   step from `integrating` on (closure `closing` included), or a step the
//!   desktop does not know (R-21). `confirmed` and `context_synchronized`
//!   do not lock (L-1); a finished parent has no row (L-5).
//!
//! The rows are the last ones read for each repository: a busy read keeps
//! them, a failed one clears them, so an unreadable repository gives no
//! record lock. While nothing is known (the repository's first read has not
//! landed, or a lifecycle answer has come and its row read has not), the
//! item is `Awaiting`: edits wait, with no words (R-21).
//!
//! A merged item whose tidy-up was refused because its working copy, its
//! ignored files or its shared branch changed is not locked (L-4, decision
//! 21, K-13): the answer that said so exempts its row, until a later
//! lifecycle answer, the row going, or the row's `updated_at` changing
//! (R-22). Nothing here holds a title, body or path.

use std::collections::{HashMap, HashSet};

use gpui_kit::EntityId;
use manyhands::repository::{
    ItemDtoKind, LifecycleCleanupPendingReason, LifecycleCleanupState, LifecycleMutationDto,
    OperationAction, OperationListDto,
};

use super::stamp::RepoKey;
use crate::desktop::services::{ReadResult, requests::DesktopRequestId};

/// An item, by its repository's root and its ID.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct LockKey {
    pub root: RepoKey,
    pub id: String,
}

impl LockKey {
    pub(crate) fn new(root: &str, id: &str) -> Self {
        Self {
            root: root.to_owned(),
            id: id.to_owned(),
        }
    }
}

/// What holds an item locked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LockSource {
    Dialog,
    Worker,
    Record,
}

/// A tidy-up refused after the merge for a change the person can undo
/// (L-4): the item is not locked, and says so.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Untidied {
    pub reason: LifecycleCleanupPendingReason,
    /// The request whose answer said so: Check again sends it again.
    pub request: DesktopRequestId,
}

/// Where an item stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LockState {
    Open,
    /// Nothing is known yet: edits wait, with no words.
    Awaiting,
    Locked(LockSource),
    Untidied(Untidied),
}

impl LockState {
    /// Whether edits of the item wait.
    pub(crate) fn blocks(self) -> bool {
        matches!(self, Self::Awaiting | Self::Locked(_))
    }
}

/// A lifecycle row of L2, as the lock reads it.
#[derive(Clone, PartialEq, Eq, Debug)]
struct RecordRow {
    /// A closure's is a ticket; a promotion's, a document.
    kind: ItemDtoKind,
    item_id: String,
    step: String,
    operation_id: Option<String>,
    updated_at: Option<String>,
}

/// The steps that leave an item editable: before the closure checkpoint or
/// the merge (L-1).
const UNLOCKED_STEPS: [&str; 2] = ["confirmed", "context_synchronized"];

/// Whether a lifecycle row in `step` locks its item: every step but
/// L-1's, an unknown one included (R-21). Finished steps are never listed.
pub(crate) fn step_locks(step: &str) -> bool {
    !UNLOCKED_STEPS.contains(&step)
}

/// The reasons a tidy-up stop unlocks the item (L-4): the library's
/// supersedable cleanup categories.
pub(crate) fn unlocks_untidied(reason: LifecycleCleanupPendingReason) -> bool {
    matches!(
        reason,
        LifecycleCleanupPendingReason::ContextChanged
            | LifecycleCleanupPendingReason::IgnoredFiles
            | LifecycleCleanupPendingReason::RemoteContextChanged
    )
}

struct Exemption {
    untidied: Untidied,
    /// The row's `updated_at` as first read after the answer.
    stamp: Option<Option<String>>,
}

/// The lock's own state, kept in the request model.
#[derive(Default)]
pub(crate) struct LifecycleLocks {
    /// The open dialogs that lock, by entity.
    dialogs: HashMap<EntityId, LockKey>,
    /// Each repository's last lifecycle rows read.
    records: HashMap<RepoKey, Vec<RecordRow>>,
    /// Items with a lifecycle answer whose rows have not been read since.
    answered: HashSet<LockKey>,
    exemptions: HashMap<LockKey, Exemption>,
}

impl LifecycleLocks {
    /// The dialog `dialog` locks `key`, or no longer does. True when that
    /// changed anything.
    pub(crate) fn set_dialog(&mut self, dialog: EntityId, key: LockKey, holds: bool) -> bool {
        if holds {
            self.dialogs.insert(dialog, key.clone()) != Some(key)
        } else {
            self.release_dialog(dialog)
        }
    }

    /// The dialog `dialog` was dropped. True when it locked an item.
    pub(crate) fn release_dialog(&mut self, dialog: EntityId) -> bool {
        self.dialogs.remove(&dialog).is_some()
    }

    /// Whether `root`'s rows have been read at least once.
    pub(crate) fn read(&self, root: &str) -> bool {
        self.records.contains_key(root)
    }

    /// A read of `root`'s pending operations landed (`result` borrowed).
    pub(crate) fn rows_read(&mut self, root: &str, result: &ReadResult<OperationListDto>) {
        let rows: Vec<RecordRow> = match result {
            ReadResult::Ok(list) => list
                .items
                .iter()
                .filter_map(|row| {
                    let kind = match row.action {
                        OperationAction::Close => ItemDtoKind::Ticket,
                        OperationAction::Promote => ItemDtoKind::Document,
                        _ => return None,
                    };
                    Some(RecordRow {
                        kind,
                        item_id: row.item_id.clone()?,
                        step: row.state.clone(),
                        operation_id: row.operation_id.clone(),
                        updated_at: row.updated_at.clone(),
                    })
                })
                .collect(),
            // Busy passes: the last rows stand.
            ReadResult::Busy => return,
            // Nothing is known of the rows, which reads as open (R-21). An
            // exemption is kept: only a row read, gone or changed, ends it
            // (R-22), and the next good read would lock it again.
            ReadResult::Failed(_) => {
                self.answered.retain(|key| key.root != root);
                self.records.insert(root.to_owned(), Vec::new());
                return;
            }
        };
        self.answered.retain(|key| key.root != root);
        self.exemptions.retain(|key, exemption| {
            if key.root != root {
                return true;
            }
            let Some(row) = rows.iter().find(|row| row.item_id == key.id) else {
                return false;
            };
            match &exemption.stamp {
                None => {
                    exemption.stamp = Some(row.updated_at.clone());
                    true
                }
                Some(stamp) => *stamp == row.updated_at,
            }
        });
        self.records.insert(root.to_owned(), rows);
    }

    /// The repository at `root` is no longer listed, or does not resolve.
    pub(crate) fn forget(&mut self, root: &str) {
        self.records.remove(root);
        self.answered.retain(|key| key.root != root);
        self.exemptions.retain(|key, _| key.root != root);
    }

    /// A lifecycle request of the item `key` was answered by `request`,
    /// with `data` when the answer carries lifecycle data: until its rows
    /// are read again the item awaits, and the answer sets or ends the
    /// item's L-4 exemption.
    pub(crate) fn answered(
        &mut self,
        key: LockKey,
        request: DesktopRequestId,
        data: Option<&LifecycleMutationDto>,
    ) {
        if let Some(data) = data {
            match data.cleanup_pending {
                Some(reason)
                    if data.cleanup == Some(LifecycleCleanupState::Pending)
                        && unlocks_untidied(reason) =>
                {
                    self.exemptions.insert(
                        key.clone(),
                        Exemption {
                            untidied: Untidied { reason, request },
                            stamp: None,
                        },
                    );
                }
                _ => {
                    self.exemptions.remove(&key);
                }
            }
        }
        self.answered.insert(key);
    }

    /// The lifecycle operations of `key` that the rows list: a resume of
    /// one of them is the item's lifecycle too.
    pub(crate) fn operations(&self, key: &LockKey) -> Vec<String> {
        self.records
            .get(&key.root)
            .into_iter()
            .flatten()
            .filter(|row| row.item_id == key.id)
            .filter_map(|row| row.operation_id.clone())
            .collect()
    }

    /// The item whose lifecycle the operation `operation_id` of `root` is,
    /// by its kind and ID, as the rows last read list it: a resume of it
    /// that names its holding request sends that close or merge again
    /// (decision 10).
    pub(crate) fn item_of(&self, root: &str, operation_id: &str) -> Option<(ItemDtoKind, &str)> {
        self.records
            .get(root)?
            .iter()
            .find(|row| row.operation_id.as_deref() == Some(operation_id))
            .map(|row| (row.kind, row.item_id.as_str()))
    }

    /// Where `key` stands, given whether its lifecycle is in flight in the
    /// worker and whether its repository's first read is still loading.
    pub(crate) fn state(&self, key: &LockKey, in_flight: bool, loading: bool) -> LockState {
        if self.dialogs.values().any(|held| held == key) {
            return LockState::Locked(LockSource::Dialog);
        }
        if in_flight {
            return LockState::Locked(LockSource::Worker);
        }
        if self.answered.contains(key) || (loading && !self.read(&key.root)) {
            return LockState::Awaiting;
        }
        let locks = self
            .records
            .get(&key.root)
            .into_iter()
            .flatten()
            .any(|row| row.item_id == key.id && step_locks(&row.step));
        if !locks {
            return LockState::Open;
        }
        match self.exemptions.get(key) {
            Some(exemption) => LockState::Untidied(exemption.untidied),
            None => LockState::Locked(LockSource::Record),
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_lock_tests.rs"]
mod tests;
