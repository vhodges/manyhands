//! The pending operations each listed repository reports, and what a
//! pending row offers: Resume and Abandon.

use std::collections::BTreeMap;

use manyhands::repository::{
    ItemDtoKind, OperationAction, OperationDto, OperationFamily, OperationListDto,
    OperationNextAction,
};

use super::{Operations, SYNCHRONIZATION_UNAVAILABLE};
use crate::desktop::{
    model::{
        list_state::ListState,
        navigation::UNAVAILABLE_CODES,
        stamp::{RepoKey, Stamp, ViewScope},
    },
    services::ReadResult,
};

impl Operations {
    /// Each listed repository's pending operations, by root.
    pub(crate) fn pending(&self) -> &BTreeMap<RepoKey, ListState<OperationListDto>> {
        &self.pending
    }

    /// Starts reading `root`'s pending operations; an earlier read is
    /// dropped when it completes. Rows already shown stay until the new
    /// read lands.
    pub(crate) fn load_pending(&mut self, root: RepoKey) -> Stamp {
        self.pending
            .entry(root.clone())
            .or_insert(ListState::Loading);
        self.generations.next(ViewScope::Operations(root))
    }

    /// A pending-operations read completed. A repository that does not
    /// resolve lists nothing. False when the read is no longer wanted.
    pub(crate) fn apply_pending(
        &mut self,
        stamp: Stamp,
        result: ReadResult<OperationListDto>,
    ) -> bool {
        if !self.generations.is_current(&stamp) {
            return false;
        }
        let ViewScope::Operations(root) = stamp.scope else {
            return false;
        };
        self.locks.rows_read(&root, &result);
        if let ReadResult::Failed(failure) = &result
            && UNAVAILABLE_CODES.contains(&failure.code)
        {
            self.pending.remove(&root);
            return true;
        }
        let state = ListState::from_read(result, |list| ListState::Ready {
            complete: list.complete,
            index: None,
            rows: list,
        });
        self.pending.insert(root, state);
        true
    }

    /// Lets go of every repository not in `roots`; their reads still
    /// running are dropped.
    pub(crate) fn retain_pending(&mut self, roots: &[RepoKey]) {
        let gone: Vec<RepoKey> = self
            .pending
            .keys()
            .filter(|root| !roots.contains(root))
            .cloned()
            .collect();
        for root in gone {
            self.locks.forget(&root);
            self.pending.remove(&root);
            self.generations.next(ViewScope::Operations(root));
        }
    }
}

/// A lifecycle row Resume takes up: its item's kind and ID, and its
/// operation (D5 decision 10; Wave 04 Cycle 03b S9). Its fields are public so
/// the dialog's resume reads them; `new` rebuilds the row from an action's
/// fields.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct LifecycleRow {
    pub kind: ItemDtoKind,
    pub item: String,
    pub operation: String,
}

impl LifecycleRow {
    /// `ResumeLifecycle` rebuilds the row it was made from.
    #[allow(dead_code)] // Used from Task 6
    pub(crate) fn new(kind: ItemDtoKind, item: String, operation: String) -> Self {
        Self {
            kind,
            item,
            operation,
        }
    }

    /// `operation`, when it is a lifecycle's parent waiting to be resumed.
    pub(crate) fn of(operation: &OperationDto) -> Option<Self> {
        let kind = match operation.action {
            OperationAction::Close => ItemDtoKind::Ticket,
            OperationAction::Promote => ItemDtoKind::Document,
            _ => return None,
        };
        if operation.next_action != Some(OperationNextAction::Resume) {
            return None;
        }
        Some(Self {
            kind,
            item: operation.item_id.clone()?,
            operation: operation.operation_id.clone()?,
        })
    }
}

/// Whether a pending row is a document promotion's or a ticket closure's
/// parent (L2), which the library lists as remote work.
fn is_lifecycle(operation: &OperationDto) -> bool {
    matches!(
        operation.action,
        OperationAction::Promote | OperationAction::Close
    )
}

/// Whether a pending row is a synchronization.
fn is_synchronization(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Remote
        || matches!(
            operation.action,
            OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
        )
}

/// Whether a pending row is remote work a resume takes up, stopping at
/// its safe points (decision 11): a remote item or repository
/// synchronization. A local-context synchronization, which has no remote,
/// is not remote work.
///
/// No row the library lists is a comment's publication waiting to be
/// resumed: the synchronization that publishes a comment is listed with
/// no next action (`read/status.rs`, `remote_standing`), and the
/// comment's `submit_comment` row is listed only while its local work is
/// incomplete or waits for its registration, when a resume does that
/// local work, ignoring its token; a resume takes up a publication only
/// once that row has completed, when it is no longer listed
/// (`bind/resume.rs`, `found_comment`).
pub(super) fn takes_up_remote_work(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Remote
        && matches!(
            operation.action,
            OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
        )
}

/// Whether a pending row is a synchronization with no remote to publish
/// to: the library lists it as a local operation of the remote name
/// (`OperationDto::action`).
fn local_synchronization(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Local
        && matches!(
            operation.action,
            OperationAction::SynchronizeContext | OperationAction::SynchronizePrimary
        )
}

/// Whether a pending row is a poll's remote row that still holds the
/// repository's reservation: what a Check now left active by Quit now or a
/// crash looks like at launch (D6 L-4, Fact 34). The library lists such a
/// row with no next action, since its store does not say whether anything
/// still runs it, and offers it no Resume of its own. Resume sends
/// `operation resume`, which the library answers `original_request_required`
/// naming the request that holds a Check now's row; an automatic poll's row,
/// which no request holds, is answered `not_resumable`, and the in-flight
/// record's entry is what finishes that one (L-1).
pub(super) fn check_now_left_active(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Remote
        && operation.action == OperationAction::Poll
        && operation.operation_id.is_some()
        && operation.next_action.is_none()
        && matches!(
            operation.state.as_str(),
            "reserved"
                | "advertising"
                | "persisting"
                | "fetch_prepared"
                | "fetch_observed"
                | "local_prepared"
                | "local_fast_forwarded"
                | "push_prepared"
                | "push_returned"
                | "push_verified"
                | "reconciling"
        )
}

/// What a pending row offers for Resume.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ResumeOffer {
    /// An operation waiting to be resumed: Resume runs it.
    Offered,
    /// Shown disabled, with why.
    Unavailable(&'static str),
    NotOffered,
}

/// Resume is offered for a Check now's row left active (D6 L-4), and
/// otherwise when the row waits to be resumed and is a local
/// command (a comment's incomplete `submit_comment` and a `context
/// sync-local` among them), a remote item or repository synchronization,
/// or a local-context or primary synchronization with no remote (D5 B6,
/// E-5), by its operation ID (decision 11). Any other synchronization
/// shows it unavailable. The library lists no comment publication waiting
/// to be resumed (`takes_up_remote_work`), so none is offered. A
/// promotion's or closure's parent (L2) is no synchronization: it is not
/// offered here, but by its own row, whose resume opens the lifecycle
/// dialog (D5 Part C, `view/operations/resume.rs`).
pub(crate) fn resume_offer(operation: &OperationDto) -> ResumeOffer {
    if check_now_left_active(operation) {
        return ResumeOffer::Offered;
    }
    if operation.next_action != Some(OperationNextAction::Resume) {
        return ResumeOffer::NotOffered;
    }
    if is_lifecycle(operation) {
        return ResumeOffer::NotOffered;
    }
    if takes_up_remote_work(operation) || local_synchronization(operation) {
        return match operation.operation_id {
            Some(_) => ResumeOffer::Offered,
            None => ResumeOffer::NotOffered,
        };
    }
    if is_synchronization(operation) {
        return ResumeOffer::Unavailable(SYNCHRONIZATION_UNAVAILABLE);
    }
    match (operation.family, &operation.operation_id) {
        (OperationFamily::Local, Some(_)) => ResumeOffer::Offered,
        _ => ResumeOffer::NotOffered,
    }
}

/// Abandon is offered for a pending local operation with an ID.
pub(crate) fn abandon_offered(operation: &OperationDto) -> bool {
    operation.family == OperationFamily::Local
        && operation.operation_id.is_some()
        && !is_synchronization(operation)
}
