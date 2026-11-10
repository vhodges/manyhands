//! Entries the library has settled without this process (D6 review I-C,
//! R-31): a pending retirement's row can be abandoned or finished by the
//! CLI, by a second instance, or by a Resume this process never sent. The
//! in-flight record's entry for it would then hold the item "Retiring…" and
//! read-only for good, and across relaunches.
//!
//! The scheduler asks the library for the repository's pending operations,
//! off the UI thread, and ends an entry whose operation is absent from a
//! list that is complete. It is conservative:
//!
//! - a read that failed, or a list that is not complete, ends nothing;
//! - an entry that changed since the read began (any retirement began,
//!   answered or settled meanwhile, `entry_epoch`) ends nothing: the read
//!   may predate it;
//! - an entry whose retirement the worker holds, queued, running or waiting
//!   out `busy`, or whose request is still unanswered in the request model,
//!   ends nothing;
//! - an entry that has not learned its operation ends nothing: there is no
//!   row to look for.
//!
//! Called at launch, once the in-flight lock is held, and on window
//! activation.

use std::{collections::BTreeSet, path::PathBuf};

use gpui_kit::Context;
use manyhands::repository::{OperationId, OperationListDto};

use super::{entity::Scheduler, retire_loop::retirement_out_in_worker};
use crate::desktop::{
    model::polling::Root,
    services::{inflight::RetirementEnd, requests::RequestState},
};

/// Whether `list`, as read, says the row of `operation` is gone: the list
/// was read, is complete, and does not name it.
pub(super) fn row_gone(list: Option<&OperationListDto>, operation: OperationId) -> bool {
    let wanted = operation.to_string();
    list.is_some_and(|list| {
        list.complete
            && !list
                .items
                .iter()
                .any(|row| row.operation_id.as_deref() == Some(wanted.as_str()))
    })
}

impl Scheduler {
    /// Reads the pending operations of every repository that has an entry
    /// with a known operation, and ends the entries whose rows are gone.
    pub(crate) fn reconcile_entries(&mut self, cx: &mut Context<Self>) {
        if self.quitting || !self.store.holds_lock() {
            return;
        }
        let roots: BTreeSet<Root> = self
            .store
            .retirements()
            .iter()
            .filter(|entry| entry.operation().is_some())
            .map(|entry| entry.root().clone())
            .collect();
        for root in roots {
            let epoch = self.entry_epoch;
            let backend = self.backend.clone();
            let path = PathBuf::from(&root);
            cx.spawn(async move |this, cx| {
                let list = cx
                    .background_executor()
                    .spawn(async move { backend.pending_operations(&path) })
                    .await;
                let _ = this.update(cx, |scheduler, cx| {
                    scheduler.reconciled(&root, epoch, list, cx)
                });
            })
            .detach();
        }
    }

    /// The list read for `root` since `epoch` is in.
    pub(crate) fn reconciled(
        &mut self,
        root: &Root,
        epoch: u64,
        list: Option<OperationListDto>,
        cx: &mut Context<Self>,
    ) {
        if epoch != self.entry_epoch || self.quitting {
            return;
        }
        let gone: Vec<_> = self
            .store
            .retirements()
            .iter()
            .filter(|entry| entry.root() == root)
            .filter_map(|entry| {
                let operation = entry.operation()?;
                let (kind, item) = entry.item()?;
                let request = entry.request()?;
                let answered = self
                    .operations
                    .read_with(cx, |operations, _| {
                        operations.requests().iter().all(|held| {
                            held.request_id != request
                                || !matches!(
                                    held.state,
                                    RequestState::Queued | RequestState::Running { .. }
                                )
                        })
                    })
                    .unwrap_or(false);
                let held = retirement_out_in_worker(&self.worker, root, kind, &item);
                (row_gone(list.as_ref(), operation) && answered && !held)
                    .then_some((request, kind, item))
            })
            .collect();
        if gone.is_empty() {
            return;
        }
        for (request, kind, item) in gone {
            self.store
                .retirement_ended(request, RetirementEnd::Finished);
            self.schedule.retire_ended(root, kind, &item, false);
        }
        // No `entry_epoch` move here: this only ends entries the read just
        // judged, and the reads taken for the other repositories at the same
        // activation say nothing of these. A read that must not be applied
        // is one that began before a retirement began, answered or settled
        // (those move the epoch themselves).
        self.publish(cx);
        self.tick(cx);
    }
}

#[cfg(test)]
mod tests {
    use manyhands::repository::{OperationId, OperationListDto};

    use super::row_gone;
    use crate::desktop::{
        services::{Backend, LibraryBackend},
        test_support::{
            fixtures::Fixture,
            retire::{empty_list, pending_list},
        },
    };

    /// Only a list that was read, is complete and does not name the row says
    /// it is gone.
    #[test]
    fn a_row_is_gone_only_from_a_complete_list_without_it() {
        let operation = OperationId::new();
        let incomplete = OperationListDto {
            items: Vec::new(),
            complete: false,
        };
        assert!(row_gone(Some(&empty_list()), operation));
        assert!(row_gone(
            Some(&pending_list(&OperationId::new().to_string())),
            operation
        ));
        assert!(!row_gone(None, operation), "the read failed");
        assert!(!row_gone(Some(&incomplete), operation), "not complete");
        assert!(!row_gone(
            Some(&pending_list(&operation.to_string())),
            operation
        ));
    }

    /// Review I-2, the default `Backend::pending_operations` the desktop
    /// runs with: a registered repository gives a complete list, here with
    /// nothing pending, and a root the library does not know gives `None`,
    /// from which nothing is concluded.
    #[test]
    fn the_default_pending_read_is_complete_for_a_registered_root_only() {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        let elsewhere = fixture.empty_directory("elsewhere");
        let backend = LibraryBackend::new(
            manyhands::repository::RepositoryService::open_at(fixture.data_dir()).unwrap(),
            Ok(fixture.draft_store()),
            Ok(fixture.resolution_store()),
            Ok(fixture.layout_store()),
        );
        let list = backend
            .pending_operations(&root)
            .expect("a registered root");
        assert!(list.complete);
        assert!(list.items.is_empty());
        assert!(backend.pending_operations(&elsewhere).is_none());
    }
}
