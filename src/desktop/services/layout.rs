//! The saved panel layout (Wave 04 Cycle 01, Part G; R-17, R-19).
//!
//! One file, `layout.json`, in `<data_dir>/layout`, through the library's
//! protected folder by name: the drafts' protection, an owner-only file and
//! an atomic replace. It is never written into a repository. The file's
//! contents are the model's (`model::layout::restore`); this store reads
//! and writes bytes.
//!
//! Writes carry generations from a counter the store owns. The counter, the
//! newest generation that reached the file, the lock and the test seams sit
//! behind one `Arc`, so every clone shares them and a relaunch never starts
//! behind. Under the lock a write older than the newest is skipped, so the
//! quit's write, made later, is never overwritten by a slower earlier one.
//!
//! The errors carry no path and no text.

use std::{
    fmt,
    path::Path,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use manyhands::drafts::{DraftError, DraftErrorKind, folder::Folder};

#[cfg(all(test, feature = "desktop-test-support"))]
use super::ReadGate;

/// The folder's name in the data directory.
const LAYOUT: &str = "layout";

/// The file in that folder.
const FILE: &str = "layout.json";

/// The largest file read. A layout is a few kilobytes; a bigger file is not
/// one, and is treated as unreadable.
pub(crate) const MAX_FILE_BYTES: usize = 1 << 20;

/// Why the layout store failed. It carries no path and no text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LayoutStoreError {
    /// The platform gave no application data directory.
    Unavailable,
    /// A link, a reparse point, another owner, an other-writable parent, or
    /// a file that is not a regular file.
    ProtectionUnavailable,
    /// Permission was denied.
    Access,
    /// The disk or the quota is full.
    NoSpace,
    /// Anything else.
    Io,
}

impl fmt::Display for LayoutStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "No application data directory is available for the layout.",
            Self::ProtectionUnavailable => "The layout folder could not be protected.",
            Self::Access => "Access to the layout folder was denied.",
            Self::NoSpace => "There is no space left to keep the layout.",
            Self::Io => "The layout folder could not be read or written.",
        })
    }
}

impl From<DraftError> for LayoutStoreError {
    fn from(error: DraftError) -> Self {
        match error.kind {
            DraftErrorKind::Unavailable => Self::Unavailable,
            DraftErrorKind::ProtectionUnavailable => Self::ProtectionUnavailable,
            DraftErrorKind::Access => Self::Access,
            DraftErrorKind::NoSpace => Self::NoSpace,
            DraftErrorKind::OpenElsewhere
            | DraftErrorKind::HeldHere
            | DraftErrorKind::NotFound
            | DraftErrorKind::Io => Self::Io,
        }
    }
}

/// What became of a write that was not refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Landed {
    /// The file now holds these bytes.
    Written,
    /// A newer write had already reached the file, so this one was skipped.
    Stale,
}

struct State {
    folder: Folder,
    /// The newest generation that reached the lock, written or failed.
    newest: u64,
}

struct Shared {
    state: Mutex<State>,
    /// The last generation handed out.
    issued: AtomicU64,
    /// Writes fail while set (P32).
    #[cfg(all(test, feature = "desktop-test-support"))]
    fail: std::sync::atomic::AtomicBool,
    /// Holds for the next background writes, oldest first (P35).
    #[cfg(all(test, feature = "desktop-test-support"))]
    holds: Mutex<std::collections::VecDeque<ReadGate>>,
}

/// The layout file, in `<data_dir>/layout`. Cloning it shares the open
/// folder, the counter and the seams.
#[derive(Clone)]
pub(crate) struct LayoutStore {
    shared: Arc<Shared>,
}

impl LayoutStore {
    /// `<data>/layout`, made or checked and held open: the application
    /// gives the data directory its service opened, and tests their
    /// fixture's.
    pub(crate) fn open_in(data: &Path) -> Result<Self, LayoutStoreError> {
        let folder = Folder::open_named(data, LAYOUT)?;
        Ok(Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State { folder, newest: 0 }),
                issued: AtomicU64::new(0),
                #[cfg(all(test, feature = "desktop-test-support"))]
                fail: std::sync::atomic::AtomicBool::new(false),
                #[cfg(all(test, feature = "desktop-test-support"))]
                holds: Mutex::default(),
            }),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // A panic while holding the folder leaves it as usable as before.
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn next_generation(&self) -> u64 {
        self.shared.issued.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// The file's bytes, or `None` when there is none (a first launch).
    /// Anything else that stops it reading, such as a directory of that
    /// name or a file over `MAX_FILE_BYTES`, is an error.
    pub(crate) fn read(&self) -> Result<Option<Vec<u8>>, LayoutStoreError> {
        match self.state().folder.read(FILE) {
            Ok(bytes) if bytes.len() > MAX_FILE_BYTES => Err(LayoutStoreError::Io),
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind == DraftErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// A write for the background: its generation is allocated now. Its
    /// `run` waits for a test's hold first, off the lock.
    pub(crate) fn begin(&self) -> Write {
        Write {
            store: self.clone(),
            generation: self.next_generation(),
            #[cfg(all(test, feature = "desktop-test-support"))]
            hold: self.shared.holds.lock().unwrap().pop_front(),
        }
    }

    /// The quit's write: the next generation, written now on this thread.
    /// It takes no hold, so it never waits on a held background write.
    pub(crate) fn write_now(&self, bytes: &[u8]) -> Result<Landed, LayoutStoreError> {
        self.write(self.next_generation(), bytes)
    }

    fn write(&self, generation: u64, bytes: &[u8]) -> Result<Landed, LayoutStoreError> {
        let mut state = self.state();
        if generation < state.newest {
            return Ok(Landed::Stale);
        }
        state.newest = generation;
        #[cfg(all(test, feature = "desktop-test-support"))]
        if self.shared.fail.load(Ordering::SeqCst) {
            return Err(LayoutStoreError::Io);
        }
        state.folder.replace(FILE, bytes)?;
        Ok(Landed::Written)
    }

    /// Makes every write fail, or stop failing (P32).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn fail_writes_for_testing(&self, fail: bool) {
        self.shared.fail.store(fail, Ordering::SeqCst);
    }

    /// Keeps the next background write from reaching the lock until the
    /// sender sends or is dropped (P35). It parks before the lock, as
    /// `Backend::read_gate` is awaited before a blocking call, so the
    /// quit's write on the test thread never waits on it.
    #[cfg(all(test, feature = "desktop-test-support"))]
    #[must_use = "dropping the handle releases the hold"]
    pub(crate) fn hold_writes_for_testing(&self) -> futures::channel::oneshot::Sender<()> {
        let (sender, receiver) = futures::channel::oneshot::channel();
        self.shared.holds.lock().unwrap().push_back(receiver);
        sender
    }
}

/// A background write: its generation, and a test's hold if one was set.
pub(crate) struct Write {
    store: LayoutStore,
    generation: u64,
    #[cfg(all(test, feature = "desktop-test-support"))]
    hold: Option<ReadGate>,
}

impl Write {
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Writes `bytes` unless a newer write got there first.
    pub(crate) async fn run(self, bytes: Vec<u8>) -> Result<Landed, LayoutStoreError> {
        #[cfg(all(test, feature = "desktop-test-support"))]
        if let Some(hold) = self.hold {
            // A dropped sender releases the write as a send does.
            let _ = hold.await;
        }
        self.store.write(self.generation, &bytes)
    }
}
