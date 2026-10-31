//! A draft session: one claimed draft's text in memory, its revisions, and
//! when the newest one is due to be written to the store.
//!
//! A change is due a second after the last edit, and never more than five
//! seconds after the first change not yet taken by a write (ruling R1). A
//! base change is due at once. At most one write is in flight; edits made
//! meanwhile are written together, as the newest revision, once it lands.
//! A failed write keeps what was flushed and is not retried on a timer: the
//! next edit, a Retry or a close or quit flush writes again.
//!
//! The session holds draft text and derives no `Debug`. Its clock is
//! injected; the registry passes the background executor's.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use manyhands::{
    drafts::{
        Draft, DraftBase, DraftClaim, DraftError, DraftErrorKind, DraftFields, DraftId,
        DraftTarget, TargetState, Written, classify,
    },
    repository::ItemDto,
    results::ResultCode,
};
use time::OffsetDateTime;

use super::{editor_text::normalized, names::TicketName};
use crate::desktop::services::requests::DesktopRequestId;

pub(crate) mod create;
pub(crate) mod rows;
pub(crate) mod save;

pub(crate) use rows::{DraftRow, RowActions, RowState};

/// How long after the last edit a change is written.
pub(crate) const IDLE: Duration = Duration::from_secs(1);
/// How long after the first unwritten change it is written, however the
/// typing goes on.
pub(crate) const MAX_UNFLUSHED: Duration = Duration::from_secs(5);

/// Whether the newest text is safe in the store. The UI claims safety only
/// for `Saved`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecoveryState {
    /// The newest revision is written, at this time.
    Saved(OffsetDateTime),
    /// A change waits for its write, or is being written.
    Pending,
    /// The last write failed, and no write has landed since.
    Failed(DraftErrorKind),
}

/// Where a session's save stands (ruling R15: one at a time).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Saving {
    /// The newest revision is being written before the save is sent.
    Flushing,
    /// The save of this revision is sent as this request.
    Submitted(DesktopRequestId, u64),
}

pub(crate) struct DraftSession {
    id: DraftId,
    /// The item the draft is of: where a save goes.
    target: DraftTarget,
    /// The item's title as read when the draft began, or its ID: what the
    /// session is called outside the draft's own row (ruling R25).
    item_title: String,
    base: Option<DraftBase>,
    current: DraftFields,
    /// The newest revision in memory; a base change raises it too (R16).
    revision: u64,
    /// The newest revision written.
    flushed: u64,
    /// When the draft was first stored, as its file says.
    created_at: OffsetDateTime,
    flushed_at: OffsetDateTime,
    /// The revision being written.
    writing: Option<u64>,
    failure: Option<DraftErrorKind>,
    /// The first and last changes not yet taken by a write.
    first_unflushed_at: Option<Instant>,
    last_change_at: Option<Instant>,
    /// A base change not yet taken by a write: due at once.
    rebased_at: Option<Instant>,
    saving: Option<Saving>,
    /// A store call has the claim out of its slot, or is making a new one:
    /// the saved revision retiring, or the text being stored again. No
    /// write is taken until it lands.
    retiring: bool,
    /// The session's draft is gone and its text was not stored again: the
    /// next write stores it again as a new draft, over `base`.
    unclaimed: bool,
    /// Shared with the write in flight; taken out to retire or discard.
    claim: Arc<Mutex<Option<DraftClaim>>>,
}

/// One revision to write, taken from a session. It runs off the
/// foreground; its result goes back to `DraftSession::write_landed`.
pub(crate) struct PendingWrite {
    revision: u64,
    current: DraftFields,
    base: Option<DraftBase>,
    claim: Arc<Mutex<Option<DraftClaim>>>,
}

impl PendingWrite {
    /// Writes the revision through the session's claim. Blocking file
    /// I/O. A claim already taken out answers `NotFound`.
    pub(crate) fn write(self) -> Result<Written, DraftError> {
        let mut claim = self
            .claim
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match claim.as_mut() {
            Some(claim) => claim.write(self.revision, self.current, self.base),
            None => Err(DraftError {
                kind: DraftErrorKind::NotFound,
            }),
        }
    }
}

impl DraftSession {
    /// A session over a claimed draft, saved as the store holds it.
    /// Nothing is due until it changes, so `_now` is not kept.
    pub(crate) fn new(claim: DraftClaim, _now: Instant) -> Self {
        let draft = claim.draft();
        let id = draft.target.item_id.to_string();
        let item_title = TicketName {
            slug: None,
            title: draft.base.as_ref().and_then(|base| title(&base.fields)),
        }
        .title_or(&id)
        .to_owned();
        Self {
            id: draft.id,
            target: draft.target.clone(),
            item_title,
            base: draft.base.clone(),
            current: draft.current.clone(),
            revision: draft.revision,
            flushed: draft.revision,
            created_at: draft.created_at,
            flushed_at: draft.flushed_at,
            writing: None,
            failure: None,
            first_unflushed_at: None,
            last_change_at: None,
            rebased_at: None,
            saving: None,
            retiring: false,
            unclaimed: false,
            claim: Arc::new(Mutex::new(Some(claim))),
        }
    }

    pub(crate) fn id(&self) -> DraftId {
        self.id
    }

    /// The item's name outside the draft's own row: its title as read
    /// when the draft began, never the draft's, or its ID.
    pub(crate) fn item_title(&self) -> &str {
        &self.item_title
    }

    /// The newest revision in memory.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// The newest revision written.
    pub(crate) fn flushed(&self) -> u64 {
        self.flushed
    }

    /// When the newest revision written was written, as the store said.
    pub(crate) fn flushed_at(&self) -> OffsetDateTime {
        self.flushed_at
    }

    /// Whether a store call holds the claim: a write in flight, or the
    /// saved revision retiring.
    pub(crate) fn busy(&self) -> bool {
        self.writing.is_some() || self.retiring
    }

    /// Whether the saved revision is retiring.
    pub(crate) fn retiring(&self) -> bool {
        self.retiring
    }

    /// Whether the session's draft is gone with its text unstored: its
    /// next write is a store again.
    pub(crate) fn unclaimed(&self) -> bool {
        self.unclaimed
    }

    /// The text is being stored again as a new draft: it takes the text,
    /// as a write does, and no write is taken until it lands.
    pub(crate) fn begin_store_again(&mut self) {
        self.retiring = true;
        self.first_unflushed_at = None;
        self.last_change_at = None;
        self.rebased_at = None;
    }

    /// The claim's slot, shared with every store call on it: a discard
    /// takes the claim from it off the foreground.
    pub(crate) fn claim_slot(&self) -> Arc<Mutex<Option<DraftClaim>>> {
        self.claim.clone()
    }

    pub(crate) fn target(&self) -> &DraftTarget {
        &self.target
    }

    pub(crate) fn base(&self) -> Option<&DraftBase> {
        self.base.as_ref()
    }

    /// The newest text in memory.
    pub(crate) fn current(&self) -> &DraftFields {
        &self.current
    }

    pub(crate) fn saving(&self) -> Option<Saving> {
        self.saving
    }

    pub(crate) fn set_saving(&mut self, saving: Option<Saving>) {
        self.saving = saving;
    }

    /// The saved revision is to retire: no write is taken until
    /// `retire_kept` or `recreated`. Gives the claim's slot, for the
    /// retirement to take the claim from off the foreground. Edits are
    /// still taken, in memory.
    pub(crate) fn begin_retire(&mut self) -> Arc<Mutex<Option<DraftClaim>>> {
        self.retiring = true;
        self.claim.clone()
    }

    /// The store kept the draft: it holds another revision than the one
    /// saved. The claim goes back in its slot and writes resume.
    pub(crate) fn retire_kept(&mut self, claim: DraftClaim) {
        *self
            .claim
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(claim);
        self.retiring = false;
    }

    /// The saved revision retired, or its retirement failed, while this
    /// session held newer text, and the text was stored again as the new
    /// draft `claim`, made from the session's text at revision `from`. The
    /// session takes its claim, ID and base; what came after `from` is
    /// still to be written.
    pub(crate) fn recreated(&mut self, claim: DraftClaim, from: u64, now: Instant) {
        let draft = claim.draft();
        let newer = self.revision > from;
        self.id = draft.id;
        self.base = draft.base.clone();
        self.created_at = draft.created_at;
        self.flushed = draft.revision;
        self.flushed_at = draft.flushed_at;
        self.revision = draft.revision;
        self.writing = None;
        self.failure = None;
        self.retiring = false;
        self.unclaimed = false;
        self.first_unflushed_at = None;
        self.last_change_at = None;
        self.rebased_at = None;
        self.claim = Arc::new(Mutex::new(Some(claim)));
        if newer {
            // Held only in memory: due at once, as a base change is.
            self.rebased_at = Some(now);
            self.changed(now);
        }
    }

    /// The text could not be stored again, for `kind`. It stays in memory,
    /// failed for that reason, over `base`, the base the save left; the
    /// next write (an edit's timer, Retry, a flush or a save) stores it
    /// again.
    pub(crate) fn recreate_failed(&mut self, kind: DraftErrorKind, base: Option<DraftBase>) {
        self.retiring = false;
        self.unclaimed = true;
        self.base = base;
        self.failure = Some(kind);
    }

    /// The editor's newest text. Returns the new revision.
    pub(crate) fn edit(&mut self, current: DraftFields, now: Instant) -> u64 {
        self.current = current;
        self.changed(now)
    }

    /// The base moved after a save. It raises the revision too, so the
    /// move is written, and is due at once. Returns the new revision.
    pub(crate) fn rebase(&mut self, base: DraftBase, now: Instant) -> u64 {
        self.base = Some(base);
        self.rebased_at.get_or_insert(now);
        self.changed(now)
    }

    fn changed(&mut self, now: Instant) -> u64 {
        self.revision += 1;
        self.first_unflushed_at.get_or_insert(now);
        self.last_change_at = Some(now);
        self.revision
    }

    /// When the newest revision is due to be written: the earliest of a
    /// second after the last change, five seconds after the first, and a
    /// base change. `None` when nothing waits, while a write is in flight
    /// (its landing schedules what came meanwhile) or the saved revision
    /// retires, and after a failure until the next change.
    pub(crate) fn flush_due(&self) -> Option<Instant> {
        if self.busy() || self.revision <= self.flushed {
            return None;
        }
        [
            self.last_change_at.map(|at| at + IDLE),
            self.first_unflushed_at.map(|at| at + MAX_UNFLUSHED),
            self.rebased_at,
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// The newest revision to write now, whether or not it is due. `None`
    /// when it is written already, a write is in flight, the saved
    /// revision retires, or there is no claim to write through
    /// (`unclaimed`: the registry stores the text again instead).
    pub(crate) fn next_write(&mut self) -> Option<PendingWrite> {
        if self.busy() || self.unclaimed || self.revision <= self.flushed {
            return None;
        }
        self.writing = Some(self.revision);
        self.first_unflushed_at = None;
        self.last_change_at = None;
        self.rebased_at = None;
        Some(PendingWrite {
            revision: self.revision,
            current: self.current.clone(),
            base: self.base.clone(),
            claim: self.claim.clone(),
        })
    }

    /// The write in flight ended. A failure keeps `flushed`: the store may
    /// hold the revision, but it is not proven durable, and a retry
    /// writes it again.
    pub(crate) fn write_landed(&mut self, result: Result<Written, DraftError>) {
        self.writing = None;
        match result {
            Ok(written) => {
                if written.revision >= self.flushed {
                    self.flushed = written.revision;
                    self.flushed_at = written.flushed_at;
                }
                self.failure = None;
            }
            Err(error) => self.failure = Some(error.kind),
        }
    }

    /// The session's newest text as a draft: what Export writes. Its
    /// revision is the newest in memory, which may not yet be stored.
    pub(crate) fn draft(&self) -> Draft {
        Draft {
            id: self.id,
            revision: self.revision,
            created_at: self.created_at,
            flushed_at: self.flushed_at,
            target: self.target.clone(),
            base: self.base.clone(),
            current: self.current.clone(),
        }
    }

    /// Where the item stands against this session's newest text and base,
    /// as `item` reads it now, its body as the editor loads it and its
    /// `deps` in ID order, as the base holds them (R-11, R-72): a difference
    /// of normalization alone is never a change. Only an item's draft has a
    /// session.
    pub(crate) fn classify(&self, item: Result<&ItemDto, ResultCode>) -> TargetState {
        let item = item.map(normalized);
        classify(&self.draft(), item.as_ref().map_err(|code| *code))
    }

    pub(crate) fn recovery_state(&self) -> RecoveryState {
        match self.failure {
            Some(kind) => RecoveryState::Failed(kind),
            None if self.flushed == self.revision => RecoveryState::Saved(self.flushed_at),
            None => RecoveryState::Pending,
        }
    }
}

/// An item's title among its fields; a comment has none.
fn title(fields: &DraftFields) -> Option<&str> {
    match fields {
        DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. } => Some(title),
        DraftFields::Comment { .. } => None,
    }
}

#[cfg(test)]
#[path = "drafts_tests.rs"]
mod tests;
