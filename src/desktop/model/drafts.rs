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

use manyhands::drafts::{
    DraftBase, DraftClaim, DraftError, DraftErrorKind, DraftFields, DraftId, Written,
};
use time::OffsetDateTime;

use super::names::TicketName;

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

pub(crate) struct DraftSession {
    id: DraftId,
    /// The item's title as read when the draft began, or its ID: what the
    /// session is called outside the draft's own row (ruling R25).
    item_title: String,
    base: Option<DraftBase>,
    current: DraftFields,
    /// The newest revision in memory; a base change raises it too (R16).
    revision: u64,
    /// The newest revision written.
    flushed: u64,
    flushed_at: OffsetDateTime,
    /// The revision being written.
    writing: Option<u64>,
    failure: Option<DraftErrorKind>,
    /// The first and last changes not yet taken by a write.
    first_unflushed_at: Option<Instant>,
    last_change_at: Option<Instant>,
    /// A base change not yet taken by a write: due at once.
    rebased_at: Option<Instant>,
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
            item_title,
            base: draft.base.clone(),
            current: draft.current.clone(),
            revision: draft.revision,
            flushed: draft.revision,
            flushed_at: draft.flushed_at,
            writing: None,
            failure: None,
            first_unflushed_at: None,
            last_change_at: None,
            rebased_at: None,
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
    #[cfg_attr(not(test), allow(dead_code))] // Compared by Task 7's retirement; tests until then.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// The newest revision written.
    #[cfg_attr(not(test), allow(dead_code))] // Compared by Task 7's retirement; tests until then.
    pub(crate) fn flushed(&self) -> u64 {
        self.flushed
    }

    /// Whether a write is in flight.
    pub(crate) fn writing(&self) -> bool {
        self.writing.is_some()
    }

    /// The editor's newest text. Returns the new revision.
    pub(crate) fn edit(&mut self, current: DraftFields, now: Instant) -> u64 {
        self.current = current;
        self.changed(now)
    }

    /// The base moved after a save. It raises the revision too, so the
    /// move is written, and is due at once. Returns the new revision.
    #[cfg_attr(not(test), allow(dead_code))] // Called by Task 7's save completion; tests until then.
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
    /// (its landing schedules what came meanwhile), and after a failure
    /// until the next change.
    pub(crate) fn flush_due(&self) -> Option<Instant> {
        if self.writing.is_some() || self.revision <= self.flushed {
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
    /// when it is written already or a write is in flight.
    pub(crate) fn next_write(&mut self) -> Option<PendingWrite> {
        if self.writing.is_some() || self.revision <= self.flushed {
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
