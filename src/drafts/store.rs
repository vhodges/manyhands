//! The draft store: atomic revisions, claims, listing, retirement and
//! discard, all through the held `drafts/` folder.
//!
//! A claim holds an exclusive advisory lock on `<id>.lock` for its whole
//! life. The store and its clones share the set of IDs they hold claims on,
//! so `list` never probes a lock held in this process (ruling R19). Only a
//! claim holder deletes a lock file, after the draft file (ruling R20).

use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, Read, Write},
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use fs4::fs_std::FileExt;
use time::OffsetDateTime;

use super::{
    DraftBase, DraftFields, DraftTarget,
    protect::{self, DraftError, DraftErrorKind, Folder},
    schema::{Draft, DraftId, UnreadableReason, decode, encode},
};

/// The exit code of a process ended by `FailureAction::Crash`.
#[doc(hidden)]
pub const CRASH_EXIT_CODE: i32 = 86;

/// A step of writing a revision, at which a test makes the write fail.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailurePoint {
    CreateTemp,
    Write,
    SyncFile,
    Rename,
    SyncFolder,
}

/// What happens at a failure point.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailureAction {
    /// The step returns this error instead of running.
    Fail(DraftErrorKind),
    /// The step runs (for `Write`, half the bytes) and the process ends at
    /// once with `CRASH_EXIT_CODE`, without unwinding.
    Crash,
}

/// The drafts kept in one data directory. Clones share the folder, the set
/// of held IDs and the failure point.
#[derive(Clone)]
pub struct DraftStore {
    shared: Arc<Shared>,
}

struct Shared {
    folder: Mutex<Folder>,
    /// IDs this store and its clones hold claims on. Taken before `folder`
    /// whenever both are held.
    held: Mutex<BTreeSet<DraftId>>,
    failure: Mutex<Option<(FailurePoint, FailureAction)>>,
}

/// One draft file in the folder.
#[derive(Clone)]
pub struct Listing {
    pub id: DraftId,
    pub entry: Entry,
    pub held: Held,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Held {
    No,
    /// This store, or a clone of it, holds the claim.
    Here,
    /// Another process holds the claim.
    Elsewhere,
}

// The approved API returns a listed draft by value.
#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum Entry {
    Draft(Draft),
    Unreadable(UnreadableReason),
}

/// A revision written in full.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Written {
    pub revision: u64,
    pub flushed_at: OffsetDateTime,
}

// The approved API hands a kept claim back by value.
#[allow(clippy::large_enum_variant)]
pub enum Retired {
    /// The draft file and its lock file are gone.
    Deleted,
    /// A newer revision is stored; the claim is handed back.
    Kept(DraftClaim),
}

/// The right to write, retire and discard one draft, for as long as it
/// lives.
pub struct DraftClaim {
    hold: Hold,
    draft: Draft,
}

/// A held lock on `<id>.lock`, with the ID in the store's held set. Drop
/// releases the lock, then frees the ID.
struct Hold {
    store: DraftStore,
    id: DraftId,
    lock: File,
}

impl Drop for Hold {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.lock);
        self.store.held().remove(&self.id);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn error(kind: DraftErrorKind) -> DraftError {
    DraftError::new(kind)
}

fn json_name(id: DraftId) -> String {
    format!("{id}.json")
}

fn lock_name(id: DraftId) -> String {
    format!("{id}.lock")
}

/// `.<id>.<16 random hex>.tmp`.
pub(super) fn temporary_name(id: DraftId) -> String {
    let random = ulid::Ulid::new().random() as u64;
    format!(".{id}.{random:016x}.tmp")
}

/// Whether `name` is one of `id`'s temporary files.
pub(super) fn is_temporary_of(name: &str, id: DraftId) -> bool {
    name.strip_prefix(&format!(".{id}."))
        .and_then(|rest| rest.strip_suffix(".tmp"))
        .is_some_and(|random| random.len() == 16 && random.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The ID a draft file's name gives, if it is `<ULID>.json`.
pub(super) fn draft_file_id(name: &str) -> Option<DraftId> {
    name.strip_suffix(".json")?.parse().ok()
}

/// Reads one draft file. `None` when the file is gone; a file whose ID is
/// not its name's is damaged.
pub(super) fn read_entry(folder: &Folder, id: DraftId) -> Option<Entry> {
    let mut file = match folder.open_file(&json_name(id)) {
        Ok(Some(file)) => file,
        Ok(None) => return None,
        Err(reason) => return Some(Entry::Unreadable(reason)),
    };
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return Some(Entry::Unreadable(UnreadableReason::Access));
    }
    Some(match decode(&bytes) {
        Ok(draft) if draft.id == id => Entry::Draft(draft),
        Ok(_) => Entry::Unreadable(UnreadableReason::Damaged),
        Err(reason) => Entry::Unreadable(reason),
    })
}

/// Drafts by latest flush, then ID; unreadable entries after them by ID.
pub(super) fn sort(listings: &mut [Listing]) {
    listings.sort_by(|left, right| match (&left.entry, &right.entry) {
        (Entry::Draft(l), Entry::Draft(r)) => r
            .flushed_at
            .cmp(&l.flushed_at)
            .then_with(|| left.id.cmp(&right.id)),
        (Entry::Draft(_), Entry::Unreadable(_)) => std::cmp::Ordering::Less,
        (Entry::Unreadable(_), Entry::Draft(_)) => std::cmp::Ordering::Greater,
        (Entry::Unreadable(_), Entry::Unreadable(_)) => left.id.cmp(&right.id),
    });
}

/// What a draft that cannot be read answers to a claim or an export.
fn unreadable_error(reason: UnreadableReason) -> DraftError {
    error(match reason {
        UnreadableReason::Access | UnreadableReason::NotOwned => DraftErrorKind::Access,
        UnreadableReason::Damaged
        | UnreadableReason::NewerFormat { .. }
        | UnreadableReason::NotAFile => DraftErrorKind::Io,
    })
}

/// Takes the lock on `file` without waiting.
fn try_lock(file: &File) -> Result<(), DraftError> {
    match FileExt::try_lock_exclusive(file) {
        Ok(true) => Ok(()),
        Ok(false) => Err(error(DraftErrorKind::OpenElsewhere)),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
            Err(error(DraftErrorKind::OpenElsewhere))
        }
        Err(e) => Err(e.into()),
    }
}

impl DraftStore {
    /// The store in the application data directory.
    pub fn open_default() -> Result<Self, DraftError> {
        Self::open_in(&protect::default_data_directory()?)
    }

    /// The store in `<data_directory>/drafts`, made if absent.
    pub fn open_in(data_directory: &Path) -> Result<Self, DraftError> {
        Ok(Self {
            shared: Arc::new(Shared {
                folder: Mutex::new(Folder::open(data_directory)?),
                held: Mutex::new(BTreeSet::new()),
                failure: Mutex::new(None),
            }),
        })
    }

    /// Makes `point` act as `action` on every write of this store and of
    /// every clone of it.
    #[doc(hidden)]
    pub fn with_failure_point_for_testing(
        self,
        point: FailurePoint,
        action: FailureAction,
    ) -> Self {
        *lock(&self.shared.failure) = Some((point, action));
        self
    }

    fn folder(&self) -> MutexGuard<'_, Folder> {
        lock(&self.shared.folder)
    }

    fn held(&self) -> MutexGuard<'_, BTreeSet<DraftId>> {
        lock(&self.shared.held)
    }

    /// Every `<ULID>.json` in the folder. It deletes nothing, and probes
    /// only the locks not held here.
    pub fn list(&self) -> Result<Vec<Listing>, DraftError> {
        let entries: Vec<(DraftId, Entry)> = {
            let folder = self.folder();
            let mut ids: Vec<DraftId> = folder
                .names()?
                .iter()
                .filter_map(|name| draft_file_id(name))
                .collect();
            ids.sort();
            ids.into_iter()
                .filter_map(|id| read_entry(&folder, id).map(|entry| (id, entry)))
                .collect()
        };
        let mut listings: Vec<Listing> = entries
            .into_iter()
            .map(|(id, entry)| Listing {
                id,
                entry,
                held: self.probe(id),
            })
            .collect();
        sort(&mut listings);
        Ok(listings)
    }

    /// Who holds `id`'s claim. The held set stays locked during a probe, so
    /// no claim of this store can take the lock while the probe has it.
    fn probe(&self, id: DraftId) -> Held {
        let held = self.held();
        if held.contains(&id) {
            return Held::Here;
        }
        let lock = match self.folder().open_lock(&lock_name(id)) {
            Ok(lock) => lock,
            Err(_) => return Held::No,
        };
        let answer = match try_lock(&lock) {
            Ok(()) => {
                let _ = FileExt::unlock(&lock);
                Held::No
            }
            Err(e) if e.kind == DraftErrorKind::OpenElsewhere => Held::Elsewhere,
            Err(_) => Held::No,
        };
        drop(held);
        answer
    }

    /// Takes `id`'s lock without waiting.
    fn hold(&self, id: DraftId) -> Result<Hold, DraftError> {
        if !self.held().insert(id) {
            return Err(error(DraftErrorKind::HeldHere));
        }
        let taken = self
            .folder()
            .open_lock(&lock_name(id))
            .and_then(|lock| try_lock(&lock).map(|()| lock));
        match taken {
            Ok(lock) => Ok(Hold {
                store: self.clone(),
                id,
                lock,
            }),
            Err(e) => {
                self.held().remove(&id);
                Err(e)
            }
        }
    }

    /// A new draft at revision 1, claimed.
    pub fn create(
        &self,
        target: DraftTarget,
        base: Option<DraftBase>,
        current: DraftFields,
    ) -> Result<DraftClaim, DraftError> {
        let id = DraftId::generate();
        let hold = self.hold(id)?;
        let now = OffsetDateTime::now_utc();
        let draft = Draft {
            id,
            revision: 1,
            created_at: now,
            flushed_at: now,
            target,
            base,
            current,
        };
        match self.write_file(&draft) {
            Ok(()) => Ok(DraftClaim { hold, draft }),
            Err(failure) => {
                // The error carries no ID, so a revision 1 left behind
                // could be neither found nor cleaned: the caller keeps the
                // text, and the file goes.
                if failure.replaced {
                    let _ = hold.delete();
                }
                Err(failure.error)
            }
        }
    }

    /// Claims an existing draft and removes its stray temporary files.
    pub fn claim(&self, id: DraftId) -> Result<DraftClaim, DraftError> {
        let hold = self.hold(id)?;
        let folder = self.folder();
        let draft = match read_entry(&folder, id) {
            None => return Err(error(DraftErrorKind::NotFound)),
            Some(Entry::Unreadable(reason)) => return Err(unreadable_error(reason)),
            Some(Entry::Draft(draft)) => draft,
        };
        // Only a dead writer can have left them: the lock is ours.
        for name in folder.names()? {
            if is_temporary_of(&name, id) {
                let _ = folder.remove(&name);
            }
        }
        drop(folder);
        Ok(DraftClaim { hold, draft })
    }

    /// Deletes the draft with that ID whether or not it is readable,
    /// taking its lock first, so a draft claimed here or elsewhere is
    /// refused.
    pub fn discard_unreadable(&self, id: DraftId) -> Result<(), DraftError> {
        let hold = self.hold(id)?;
        hold.delete()
    }

    /// The bytes of `id`'s file, as they are, for a raw export.
    pub fn raw_bytes(&self, id: DraftId) -> Result<Vec<u8>, DraftError> {
        let mut file = match self.folder().open_file(&json_name(id)) {
            Ok(Some(file)) => file,
            Ok(None) => return Err(error(DraftErrorKind::NotFound)),
            Err(reason) => return Err(unreadable_error(reason)),
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// The test action at `point`, if any.
    fn failure_at(&self, point: FailurePoint) -> Option<FailureAction> {
        match *lock(&self.shared.failure) {
            Some((at, action)) if at == point => Some(action),
            _ => None,
        }
    }

    /// Runs `step` as `point`: replaced by an injected error, or followed by
    /// an injected crash.
    fn step<T>(
        &self,
        point: FailurePoint,
        step: impl FnOnce() -> Result<T, DraftError>,
    ) -> Result<T, DraftError> {
        match self.failure_at(point) {
            Some(FailureAction::Fail(kind)) => Err(error(kind)),
            Some(FailureAction::Crash) => {
                let _ = step();
                std::process::exit(CRASH_EXIT_CODE)
            }
            None => step(),
        }
    }

    /// Writes `draft` as its file by atomic replacement: a new temporary
    /// file, written in full and synced, renamed over the draft file, then
    /// the folder synced. Any failure before the rename removes the
    /// temporary file and leaves the previous file as it was.
    fn write_file(&self, draft: &Draft) -> Result<(), WriteFailure> {
        let before = |error| WriteFailure {
            error,
            replaced: false,
        };
        let mut folder = self.folder();
        let bytes = encode(draft);
        let temporary = temporary_name(draft.id);
        // `create_new` checks the folder again.
        let mut file = self
            .step(FailurePoint::CreateTemp, || folder.create_new(&temporary))
            .map_err(before)?;
        let written = match self.failure_at(FailurePoint::Write) {
            Some(FailureAction::Fail(kind)) => Err(error(kind)),
            Some(FailureAction::Crash) => {
                let _ = file.write_all(&bytes[..bytes.len() / 2]);
                std::process::exit(CRASH_EXIT_CODE)
            }
            None => file.write_all(&bytes).map_err(DraftError::from),
        }
        .and_then(|()| self.step(FailurePoint::SyncFile, || Ok(file.sync_all()?)));
        drop(file);
        if let Err(e) = written.and_then(|()| {
            self.step(FailurePoint::Rename, || {
                folder.rename(&temporary, &json_name(draft.id))
            })
        }) {
            let _ = folder.remove(&temporary);
            return Err(before(e));
        }
        self.step(FailurePoint::SyncFolder, || folder.sync())
            .map_err(|error| WriteFailure {
                error,
                replaced: true,
            })
    }
}

/// A failed write, and whether the new revision replaced the old one
/// before it failed.
struct WriteFailure {
    error: DraftError,
    replaced: bool,
}

impl Hold {
    /// Deletes the draft file, then its temporary files, then the lock
    /// file, then releases the lock. A draft file already gone is no
    /// failure; the temporary files go on a best-effort basis, and a lock
    /// file that stays is harmless.
    fn delete(self) -> Result<(), DraftError> {
        {
            let folder = self.store.folder();
            match folder.remove(&json_name(self.id)) {
                Ok(()) => {}
                Err(e) if e.kind == DraftErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            // A dead writer's temporary file can hold the draft's text.
            if let Ok(names) = folder.names() {
                for name in names {
                    if is_temporary_of(&name, self.id) {
                        let _ = folder.remove(&name);
                    }
                }
            }
            let _ = folder.remove(&lock_name(self.id));
        }
        drop(self);
        Ok(())
    }
}

impl DraftClaim {
    pub fn draft(&self) -> &Draft {
        &self.draft
    }

    /// Writes `revision`, which may not be below the stored one. If the
    /// folder sync fails after the rename, the new revision is stored and
    /// the claim knows it, but the failure is still returned: the revision
    /// is not proven durable.
    pub fn write(
        &mut self,
        revision: u64,
        current: DraftFields,
        base: Option<DraftBase>,
    ) -> Result<Written, DraftError> {
        if revision < self.draft.revision {
            return Err(error(DraftErrorKind::Io));
        }
        let draft = Draft {
            id: self.draft.id,
            revision,
            created_at: self.draft.created_at,
            flushed_at: OffsetDateTime::now_utc(),
            target: self.draft.target.clone(),
            base,
            current,
        };
        let result = self.hold.store.write_file(&draft);
        let written = Written {
            revision,
            flushed_at: draft.flushed_at,
        };
        match result {
            Ok(()) => {
                self.draft = draft;
                Ok(written)
            }
            Err(failure) => {
                if failure.replaced {
                    self.draft = draft;
                }
                Err(failure.error)
            }
        }
    }

    /// Deletes the draft if `saved_revision` is the stored revision;
    /// otherwise hands the claim back.
    ///
    /// It consumes the claim even when it fails: a failed retirement
    /// leaves the draft on disk and releases it. No text is lost, but the
    /// caller must treat the session as released.
    pub fn retire(self, saved_revision: u64) -> Result<Retired, DraftError> {
        if self.draft.revision != saved_revision {
            return Ok(Retired::Kept(self));
        }
        self.hold.delete()?;
        Ok(Retired::Deleted)
    }

    /// Deletes the draft, whatever its revision.
    ///
    /// It consumes the claim even when it fails: a failed discard leaves
    /// the draft on disk and releases it. No text is lost, but the caller
    /// must treat the session as released.
    pub fn discard(self) -> Result<(), DraftError> {
        self.hold.delete()
    }
}
