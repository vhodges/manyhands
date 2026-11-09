//! The in-flight record and the instance lock (D6 design B3, R-31).
//!
//! The desktop keeps, in its own data directory and never in a repository, a
//! small versioned record of its unfinished remote work, so a process killed
//! mid-poll can finish what it started. A killed process leaves its remote
//! row active, and only a same-ID restart takes such a row over (Fact 29), so
//! the operation ID is what the record keeps.
//!
//! - **Poll entries**, at most one per repository: its root and the
//!   operation ID. Written before the poll is submitted, replaced when a new
//!   ID is used, and removed after an answer once `polling_status` no longer
//!   names that ID as the active operation, or when the answer is terminal.
//! - **Retirement entries**: the request ID, root, kind and item ID, and the
//!   operation ID from the request's first progress event. Written before
//!   the request is submitted, and removed only when the worker's record of
//!   the request is `Finished` (Fact 36), or on `Removed` or `NotSent`. An
//!   `Accepted` request, one Owed, keeps its entry. Task 11a wires the
//!   retirement intent to these; the rules live here.
//!
//! The record is `<data>/inflight/record.json`, replaced atomically (a
//! temporary file, then a rename). A record that cannot be read is renamed
//! aside, replaced, and reported once ([`InflightStore::take_report`]).
//!
//! Only the holder of the instance lock writes the record. The lock is an
//! advisory exclusive lock on `<data>/inflight/instance.lock`, the same
//! `fs4` call the draft store uses, released when the process dies. A second
//! instance keeps its in-flight work in memory, takes no row over and runs
//! no retirement loop (K-25). The lock is tried at launch and again on
//! window activation ([`InflightStore::try_acquire`]); once it is held, the
//! entries the previous holder left are the lock holder's to finish (L-1).
//!
//! Nothing in the record is a title, a body, a comment or key material.

// Called from Task 5 (the scheduler entity), Task 11a and Task 12: until then only the tests use it.
#![cfg_attr(not(test), allow(dead_code))]

use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

use fs4::fs_std::FileExt;
use manyhands::{
    canonical::ItemId,
    drafts::{DraftErrorKind, folder::Folder},
    repository::{OperationId, RequestId},
};
use serde::{Deserialize, Serialize};

use super::worker::RecordState;
use crate::desktop::model::polling::{Answer, PollAttempt, Root, TargetKind};

/// The record's format. A record of any other version is set aside, not
/// read: a newer app's data is not this one's to rewrite.
const VERSION: u32 = 1;
const FOLDER: &str = "inflight";
const RECORD: &str = "record.json";
const LOCK: &str = "instance.lock";

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct PollEntry {
    root: String,
    operation: String,
}

/// What a retirement entry remembers of an item.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Ticket,
    Document,
}

/// A retirement the desktop sent and has not seen end (R-31). The operation
/// ID is the one the first progress event named: the library's journal row
/// carries no item (Fact 33), so this is how the pending row is tied back to
/// its item.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(crate) struct RetirementEntry {
    request: String,
    root: String,
    kind: Kind,
    item: String,
    operation: Option<String>,
}

impl RetirementEntry {
    /// A retirement of `item` of `root` about to be submitted under
    /// `request`.
    pub(crate) fn new(request: RequestId, root: &Root, kind: TargetKind, item: &ItemId) -> Self {
        Self {
            request: request.to_string(),
            root: root.clone(),
            kind: match kind {
                TargetKind::Ticket => Kind::Ticket,
                TargetKind::Document => Kind::Document,
            },
            item: item.to_string(),
            operation: None,
        }
    }

    pub(crate) fn request(&self) -> Option<RequestId> {
        RequestId::parse(&self.request).ok()
    }

    pub(crate) fn root(&self) -> &Root {
        &self.root
    }

    pub(crate) fn item(&self) -> Option<(TargetKind, ItemId)> {
        let kind = match self.kind {
            Kind::Ticket => TargetKind::Ticket,
            Kind::Document => TargetKind::Document,
        };
        Some((kind, self.item.parse().ok()?))
    }

    /// The operation of the request's journal row, once a progress event
    /// named it.
    pub(crate) fn operation(&self) -> Option<OperationId> {
        OperationId::parse(self.operation.as_deref()?).ok()
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct Record {
    version: u32,
    polls: Vec<PollEntry>,
    retirements: Vec<RetirementEntry>,
}

/// What `polling_status.active_operation_id` named when the caller read it
/// after an answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ActiveRow {
    /// No remote operation holds the repository.
    Nothing,
    /// This operation does.
    Operation(OperationId),
    /// The read failed: nothing is known, so nothing is dropped for it.
    Unread,
}

/// How a retirement request ended, for the rule that removes its entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RetirementEnd {
    /// The worker's record of the request is finished: its answer is stored.
    Finished,
    /// Taken from the queue before it ran.
    Removed,
    /// Never built, so nothing was asked of the library.
    NotSent,
    /// The record is open: the request is Owed, and Resume will resend it.
    Accepted,
    /// Where the record stands is not known.
    Unknown,
}

impl From<RecordState> for RetirementEnd {
    fn from(state: RecordState) -> Self {
        match state {
            RecordState::Finished => Self::Finished,
            RecordState::Accepted => Self::Accepted,
            RecordState::Unknown => Self::Unknown,
        }
    }
}

/// What taking the lock came to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Acquired {
    /// It was held already.
    Already,
    /// It is held now: the record was read, and L-1 runs.
    Now,
    /// Another instance holds it.
    Elsewhere,
    /// There is no directory to lock in, or the lock could not be opened.
    /// Treated as another instance: the desktop runs without a record.
    Nowhere,
}

/// Something about the record the person should hear of once (Activity shows
/// it later).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Report {
    /// The record could not be read. It was set aside, not deleted, and
    /// replaced.
    Unreadable,
    /// The record could not be written; the work in flight is kept in
    /// memory.
    NotSaved,
}

struct Place {
    folder: Folder,
    directory: PathBuf,
}

/// The in-flight record and the lock behind its writes. One per process,
/// owned by the scheduler.
pub(crate) struct InflightStore {
    place: Option<Place>,
    /// The lock file, open and locked while this instance holds the lock.
    held: Option<File>,
    record: Record,
    report: Option<Report>,
    /// The record on disk could not be set aside, so it is not replaced.
    keep_record: bool,
}

impl InflightStore {
    /// The store for the data directory `data`. It holds no lock yet
    /// (`try_acquire`). A folder that cannot be opened leaves a store that
    /// never holds one, as a second instance does.
    pub(crate) fn open(data: &Path) -> Self {
        let place = Folder::open_named(data, FOLDER).ok().map(|folder| Place {
            folder,
            directory: data.join(FOLDER),
        });
        Self::over(place)
    }

    /// A store with nowhere to keep a record: in memory only, never the lock
    /// holder.
    pub(crate) fn in_memory() -> Self {
        Self::over(None)
    }

    fn over(place: Option<Place>) -> Self {
        Self {
            place,
            held: None,
            record: Record {
                version: VERSION,
                ..Record::default()
            },
            report: None,
            keep_record: false,
        }
    }

    /// Tries to take the instance lock, without waiting. Called at launch and
    /// again on window activation (R-31). On taking it, the record on disk is
    /// read (an unreadable one is set aside and reported) and merged under
    /// what this instance already holds in memory, and the result is
    /// written: from then on this instance writes, and L-1 may run.
    pub(crate) fn try_acquire(&mut self) -> Acquired {
        if self.held.is_some() {
            return Acquired::Already;
        }
        let Some(place) = &self.place else {
            return Acquired::Nowhere;
        };
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = match options.open(place.directory.join(LOCK)) {
            Ok(file) => file,
            Err(_) => return Acquired::Nowhere,
        };
        match FileExt::try_lock_exclusive(&file) {
            Ok(true) => {}
            Ok(false) => return Acquired::Elsewhere,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Acquired::Elsewhere;
            }
            Err(_) => return Acquired::Nowhere,
        }
        self.held = Some(file);
        let mut loaded = self.load();
        // What this instance sent before it held the lock is newer than what
        // the previous holder left.
        for own in std::mem::take(&mut self.record.polls) {
            loaded.polls.retain(|entry| entry.root != own.root);
            loaded.polls.push(own);
        }
        for own in std::mem::take(&mut self.record.retirements) {
            loaded
                .retirements
                .retain(|entry| entry.request != own.request);
            loaded.retirements.push(own);
        }
        self.record = loaded;
        self.persist();
        Acquired::Now
    }

    /// Whether this instance holds the lock: only then does it write, take a
    /// stale row over (L-1) or run a retirement loop (L-6).
    pub(crate) fn holds_lock(&self) -> bool {
        self.held.is_some()
    }

    /// Reads the record from disk. A record that cannot be read is renamed
    /// aside and reported, and the store starts from an empty one.
    fn load(&mut self) -> Record {
        let Some(place) = &self.place else {
            return Record::default();
        };
        let unreadable = match place.folder.read(RECORD) {
            Ok(bytes) => match serde_json::from_slice::<Record>(&bytes) {
                Ok(record) if record.version == VERSION => return record,
                _ => true,
            },
            Err(error) if error.kind == DraftErrorKind::NotFound => false,
            Err(_) => true,
        };
        if unreadable {
            let aside = format!("record.unreadable-{}", ulid::Ulid::new());
            let from = place.directory.join(RECORD);
            // A record that cannot be set aside is left where it is, never
            // deleted, and not written over: this instance keeps its work in
            // memory.
            if fs::rename(&from, place.directory.join(aside)).is_err() {
                self.keep_record = true;
            }
            self.report = Some(Report::Unreadable);
        }
        Record {
            version: VERSION,
            ..Record::default()
        }
    }

    /// Writes the record, atomically, when this instance holds the lock. A
    /// failure keeps the work in memory and is reported once.
    fn persist(&mut self) {
        if self.held.is_none() || self.keep_record {
            return;
        }
        let Some(place) = &mut self.place else {
            return;
        };
        let Ok(bytes) = serde_json::to_vec(&self.record) else {
            return;
        };
        if place.folder.replace(RECORD, &bytes).is_err() && self.report.is_none() {
            self.report = Some(Report::NotSaved);
        }
    }

    /// The problem with the record the person has not heard of yet, once.
    pub(crate) fn take_report(&mut self) -> Option<Report> {
        self.report.take()
    }

    /// The repository's poll entry, whether this instance sent it or the
    /// previous holder left it.
    pub(crate) fn poll_entry(&self, root: &Root) -> Option<OperationId> {
        let entry = self.record.polls.iter().find(|entry| &entry.root == root)?;
        OperationId::parse(&entry.operation).ok()
    }

    /// The operation the repository's next poll takes over: its poll entry,
    /// for the lock holder only (L-1, L-6). An instance without the lock
    /// takes nothing over.
    pub(crate) fn takeover_of(&self, root: &Root) -> Option<OperationId> {
        self.holds_lock().then(|| self.poll_entry(root)).flatten()
    }

    /// Every repository with a poll entry, and the operation. For the lock
    /// holder these are the rows to finish (L-1).
    pub(crate) fn polls(&self) -> Vec<(Root, OperationId)> {
        self.record
            .polls
            .iter()
            .filter_map(|entry| {
                Some((
                    entry.root.clone(),
                    OperationId::parse(&entry.operation).ok()?,
                ))
            })
            .collect()
    }

    /// A poll of `root` under `op` is about to be submitted: the repository's
    /// entry is this, replacing any other (at most one per repository). Call
    /// it before the poll is submitted.
    pub(crate) fn poll_submitting(&mut self, root: &Root, op: OperationId) {
        let entry = PollEntry {
            root: root.clone(),
            operation: op.to_string(),
        };
        if self.record.polls.contains(&entry) {
            return;
        }
        self.record.polls.retain(|held| &held.root != root);
        self.record.polls.push(entry);
        self.persist();
    }

    /// The submit of the poll under `op` was refused (quitting closed the
    /// queue): it never ran, so its entry goes, unless a newer one replaced
    /// it.
    pub(crate) fn poll_withdrawn(&mut self, root: &Root, op: OperationId) {
        let before = self.record.polls.len();
        self.record
            .polls
            .retain(|entry| !(&entry.root == root && entry.operation == op.to_string()));
        if self.record.polls.len() != before {
            self.persist();
        }
    }

    /// A poll answered. Its entry goes when the answer is terminal, or when
    /// `active` says the repository's active operation is no longer the
    /// entry's ID (R-31): another operation holds it, a configuration fence
    /// moved the row, or nothing does. It stays while the row is still the
    /// entry's, and while `active` could not be read, so a failed read never
    /// drops a row. An `OperationMismatch` answer drops it too: the row under
    /// the ID is another target's or kind's, not this app's (L-3).
    ///
    /// Never adds an entry: an operation the record does not hold is not
    /// taken over, whatever answered (L-5).
    pub(crate) fn poll_answered(&mut self, attempt: &PollAttempt, active: ActiveRow) {
        let Some(at) = self
            .record
            .polls
            .iter()
            .position(|entry| entry.root == attempt.root)
        else {
            return;
        };
        let entry_operation = self.record.polls[at].operation.clone();
        let ours = entry_operation == attempt.operation.to_string();
        let moved_on = match active {
            ActiveRow::Operation(op) => op.to_string() != entry_operation,
            ActiveRow::Nothing => true,
            ActiveRow::Unread => false,
        };
        let ended = ours && (terminal(attempt) || attempt.answer == Answer::OperationMismatch);
        if moved_on || ended {
            self.record.polls.remove(at);
            self.persist();
        }
    }

    /// A retirement is about to be submitted: its entry is written first.
    pub(crate) fn retirement_submitting(&mut self, entry: RetirementEntry) {
        self.record
            .retirements
            .retain(|held| held.request != entry.request);
        self.record.retirements.push(entry);
        self.persist();
    }

    /// The request's first progress event named `op`. Only the first is
    /// kept: a request sent again names the same row.
    pub(crate) fn retirement_operation(&mut self, request: RequestId, op: OperationId) {
        let request = request.to_string();
        let Some(entry) = self
            .record
            .retirements
            .iter_mut()
            .find(|entry| entry.request == request)
        else {
            return;
        };
        if entry.operation.is_none() {
            entry.operation = Some(op.to_string());
            self.persist();
        }
    }

    /// The request ended as `end` says. Only a finished record, a request
    /// removed from the queue and one never sent end its entry; an Owed
    /// (accepted) request keeps it, since Resume needs the item.
    pub(crate) fn retirement_ended(&mut self, request: RequestId, end: RetirementEnd) {
        if !matches!(
            end,
            RetirementEnd::Finished | RetirementEnd::Removed | RetirementEnd::NotSent
        ) {
            return;
        }
        let request = request.to_string();
        let before = self.record.retirements.len();
        self.record
            .retirements
            .retain(|entry| entry.request != request);
        if self.record.retirements.len() != before {
            self.persist();
        }
    }

    /// The retirements this instance sent, or its predecessor left, that have
    /// not ended.
    pub(crate) fn retirements(&self) -> &[RetirementEntry] {
        &self.record.retirements
    }

    /// Whether this instance may run a retirement loop: only the lock holder
    /// does (L-6, K-25).
    pub(crate) fn runs_retirement_loop(&self) -> bool {
        self.holds_lock()
    }
}

/// An answer that finishes the row it was for: a completion, a live failure
/// (the library records its category), and a cancellation the library
/// reported. A cancellation the desktop made before it called the library
/// (`completed_at` is none) finished nothing.
fn terminal(attempt: &PollAttempt) -> bool {
    match attempt.answer {
        Answer::Completed | Answer::Transport(_) => true,
        Answer::Cancelled(_) => attempt.completed_at.is_some(),
        _ => false,
    }
}

#[cfg(test)]
#[path = "inflight_tests.rs"]
mod tests;
