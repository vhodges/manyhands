//! What an observation of one context depended on, by stat: enough to tell,
//! without reading a file, whether the context may have changed since.
//!
//! A basis holds the context's head, the branch its HEAD names, where its
//! worktree is registered and the configuration blob of its head, then the
//! stat of every file the observation read and of every directory it listed
//! or looked at, with the entries of each listed directory that the
//! observation's walk uses. An observation records the content hash of each
//! file as it read it; a basis taken by stat alone has none.

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use git2::{ObjectType, Oid};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::activity::CommitActivity;

/// How long after its modification an entry's stat alone is not trusted: on
/// a filesystem with coarse timestamps a file or directory can change again
/// within the same tick without its stat moving.
pub(super) const RACY_WINDOW_NS: i64 = 2_000_000_000;

/// The version `encode` writes and the only one `decode` reads.
const BASIS_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Basis {
    pub(super) head: Option<Oid>,
    pub(super) branch: Option<String>,
    /// The repository root for the primary. For an item's worktree, the path
    /// the repository registers for it, or empty when it registers none.
    pub(super) worktree: PathBuf,
    /// The blob of the configuration file in the head's tree.
    pub(super) config_blob: Option<Oid>,
    /// The commit an item context's own walk found for its item, which an
    /// observation at the same head and owned paths need not look for again.
    /// It is not compared: it follows from the head and the files.
    pub(super) item_commit: Option<CommitActivity>,
    /// In path order, relative to the worktree.
    pub(super) files: Vec<BasisFile>,
    /// In path order, relative to the worktree; the worktree itself is the
    /// empty path.
    pub(super) dirs: Vec<BasisDir>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BasisFile {
    pub(super) path: PathBuf,
    pub(super) size: u64,
    pub(super) inode: Option<u64>,
    pub(super) ctime_ns: Option<i64>,
    pub(super) mtime_ns: i64,
    pub(super) hash: Option<Oid>,
}

/// A directory the observation listed or looked at. Its stat says whether
/// a basis taken later must list it again; what is compared is its listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BasisDir {
    pub(super) path: PathBuf,
    pub(super) inode: Option<u64>,
    pub(super) ctime_ns: Option<i64>,
    pub(super) mtime_ns: i64,
    /// `None` for a directory only looked at, not listed.
    pub(super) listing: Option<BasisListing>,
}

/// What listing a directory gave the walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BasisListing {
    /// The entries the walk uses, in name order: each subdirectory it goes
    /// into, each file it stats or reads, and each entry it reports a
    /// problem for. An entry it passes over is not here.
    pub(super) entries: Vec<BasisEntry>,
    /// How many entries the listing counted against the walk's limit, every
    /// entry passed over included. `None` when the listing did not end: it
    /// met the limit or could not read an entry.
    pub(super) count: Option<u64>,
    /// Whether the directory held no entry at all.
    pub(super) empty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BasisEntry {
    pub(super) name: OsString,
    /// `None` when its type could not be read.
    pub(super) kind: Option<EntryKind>,
}

/// The type of a directory entry, a link not followed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntryKind {
    Dir,
    File,
    Symlink,
    Other,
}

impl EntryKind {
    pub(super) fn is_dir(self) -> bool {
        self == Self::Dir
    }

    pub(super) fn is_file(self) -> bool {
        self == Self::File
    }

    pub(super) fn is_symlink(self) -> bool {
        self == Self::Symlink
    }

    fn of(file_type: fs::FileType) -> Self {
        if file_type.is_symlink() {
            Self::Symlink
        } else if file_type.is_dir() {
            Self::Dir
        } else if file_type.is_file() {
            Self::File
        } else {
            Self::Other
        }
    }

    fn code(kind: Option<Self>) -> &'static str {
        match kind {
            Some(Self::Dir) => "d",
            Some(Self::File) => "f",
            Some(Self::Symlink) => "l",
            Some(Self::Other) => "o",
            None => "?",
        }
    }

    /// `Some(None)` for an entry whose type could not be read, `None` for a
    /// code `code` does not write.
    fn from_code(code: &str) -> Option<Option<Self>> {
        match code {
            "d" => Some(Some(Self::Dir)),
            "f" => Some(Some(Self::File)),
            "l" => Some(Some(Self::Symlink)),
            "o" => Some(Some(Self::Other)),
            "?" => Some(None),
            _ => None,
        }
    }
}

/// An entry of a directory, as a walk lists it.
pub(super) struct Entry {
    path: PathBuf,
    name: OsString,
    kind: Option<EntryKind>,
}

impl Entry {
    pub(super) fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub(super) fn file_name(&self) -> &OsStr {
        &self.name
    }

    /// `None` when its type could not be read.
    pub(super) fn file_type(&self) -> Option<EntryKind> {
        self.kind
    }

    pub(super) fn of(entry: &fs::DirEntry) -> Self {
        Self {
            path: entry.path(),
            name: entry.file_name(),
            kind: entry.file_type().ok().map(EntryKind::of),
        }
    }
}

/// What `Recording::list` gives for a directory.
pub(super) enum Listing {
    /// The directory, listed now.
    Read(fs::ReadDir),
    /// A directory whose stat has not moved since the stored basis listed
    /// it: the entries the walk used then, in name order, and how many
    /// entries the listing counted.
    Carried { entries: Vec<Entry>, count: usize },
}

/// The Git half of a basis: what the context's checkout is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Checkout {
    pub(super) head: Option<Oid>,
    pub(super) branch: Option<String>,
    pub(super) worktree: PathBuf,
    pub(super) config_blob: Option<Oid>,
}

/// What comparing a stored basis with the current one found.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum BasisCheck {
    Same,
    /// Nothing differs, but these files were modified too close to the
    /// indexed time for their stat to be trusted. Each is the same file when
    /// its content still has the stored hash (`settles`).
    Racy(Vec<PathBuf>),
    Moved,
}

impl Basis {
    /// Whether `other` differs from this basis in anything a check compares:
    /// the Git half, which files and directories there are, the stat of a
    /// file, or the listing of a directory. A directory's own stat is not
    /// compared, nor are hashes and the item commit.
    pub(super) fn differs(&self, other: &Self) -> bool {
        self.head != other.head
            || self.branch != other.branch
            || self.worktree != other.worktree
            || self.config_blob != other.config_blob
            || !self
                .files
                .iter()
                .map(BasisFile::stat)
                .eq(other.files.iter().map(BasisFile::stat))
            || !self
                .dirs
                .iter()
                .map(BasisDir::shape)
                .eq(other.dirs.iter().map(BasisDir::shape))
    }

    /// The stored text of the basis.
    ///
    /// A path is stored as text, so one that is not UTF-8 does not read back
    /// as itself: a basis that holds one never checks as the same, and its
    /// context is observed each time.
    pub(super) fn encode(&self) -> String {
        let text = |path: &OsStr| path.to_string_lossy().into_owned();
        let stored = StoredBasis {
            version: BASIS_VERSION,
            head: self.head.map(|oid| oid.to_string()),
            branch: self.branch.clone(),
            worktree: text(self.worktree.as_os_str()),
            config_blob: self.config_blob.map(|oid| oid.to_string()),
            item_commit: self.item_commit.map(|commit| {
                (
                    commit.oid.to_string(),
                    commit.at.unix_timestamp(),
                    commit.ordinal,
                )
            }),
            files: self
                .files
                .iter()
                .map(|file| {
                    (
                        text(file.path.as_os_str()),
                        file.size,
                        file.inode,
                        file.ctime_ns,
                        file.mtime_ns,
                        file.hash.map(|oid| oid.to_string()),
                    )
                })
                .collect(),
            dirs: self
                .dirs
                .iter()
                .map(|dir| {
                    (
                        text(dir.path.as_os_str()),
                        dir.inode,
                        dir.ctime_ns,
                        dir.mtime_ns,
                        dir.listing.as_ref().map(|listing| {
                            (
                                listing
                                    .entries
                                    .iter()
                                    .map(|entry| {
                                        (text(&entry.name), EntryKind::code(entry.kind).to_owned())
                                    })
                                    .collect(),
                                listing.count,
                                listing.empty,
                            )
                        }),
                    )
                })
                .collect(),
        };
        // Strings and numbers only: serializing them cannot fail. Were it to,
        // the empty text reads back as no basis, which is a dirty context.
        serde_json::to_string(&stored).unwrap_or_default()
    }

    /// The basis a stored text holds; `None` for text that is not one this
    /// version wrote.
    #[cfg_attr(not(test), allow(dead_code))] // Read by the per-context indexing that follows.
    pub(super) fn decode(stored: &str) -> Option<Self> {
        let stored = serde_json::from_str::<StoredBasis>(stored).ok()?;
        if stored.version != BASIS_VERSION {
            return None;
        }
        let item_commit = match stored.item_commit {
            Some((oid, at, ordinal)) => Some(CommitActivity {
                oid: stored_oid(&oid)?,
                at: OffsetDateTime::from_unix_timestamp(at).ok()?,
                ordinal,
            }),
            None => None,
        };
        let files = stored
            .files
            .into_iter()
            .map(|(path, size, inode, ctime_ns, mtime_ns, hash)| {
                Some(BasisFile {
                    path: PathBuf::from(path),
                    size,
                    inode,
                    ctime_ns,
                    mtime_ns,
                    hash: stored_optional_oid(hash)?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let dirs = stored
            .dirs
            .into_iter()
            .map(|(path, inode, ctime_ns, mtime_ns, listing)| {
                let listing = match listing {
                    Some((entries, count, empty)) => Some(BasisListing {
                        entries: entries
                            .into_iter()
                            .map(|(name, code)| {
                                Some(BasisEntry {
                                    name: OsString::from(name),
                                    kind: EntryKind::from_code(&code)?,
                                })
                            })
                            .collect::<Option<Vec<_>>>()?,
                        count,
                        empty,
                    }),
                    None => None,
                };
                Some(BasisDir {
                    path: PathBuf::from(path),
                    inode,
                    ctime_ns,
                    mtime_ns,
                    listing,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            head: stored_optional_oid(stored.head)?,
            branch: stored.branch,
            worktree: PathBuf::from(stored.worktree),
            config_blob: stored_optional_oid(stored.config_blob)?,
            item_commit,
            files,
            dirs,
        })
    }
}

/// Compares the basis stored for a context, indexed at `indexed_at_ns`,
/// with the one it has now.
///
/// A directory whose listing gives the walk the same entries is the same,
/// whatever its stat: an entry the walk passes over, such as an editor's
/// swap file, is no change. A file not modified at least `RACY_WINDOW_NS`
/// before the indexed time is racy even when its stat is the same. A
/// directory is never racy here: `stat_basis` lists such a directory again.
pub(super) fn check(stored: &Basis, current: &Basis, indexed_at_ns: i64) -> BasisCheck {
    if stored.differs(current) {
        return BasisCheck::Moved;
    }
    let trusted_before = indexed_at_ns.saturating_sub(RACY_WINDOW_NS);
    let racy = stored
        .files
        .iter()
        .filter(|file| file.mtime_ns > trusted_before)
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    if racy.is_empty() {
        BasisCheck::Same
    } else {
        BasisCheck::Racy(racy)
    }
}

/// Whether each of the `racy` files of the worktree still holds what the
/// stored basis hashed. A file the stored basis has no hash for never does.
pub(super) fn settles(stored: &Basis, worktree: &Path, racy: &[PathBuf]) -> bool {
    racy.iter().all(|path| {
        let stored_hash = stored
            .files
            .binary_search_by(|file| file.path.as_path().cmp(path))
            .ok()
            .and_then(|index| stored.files[index].hash);
        stored_hash.is_some()
            && fs::read(worktree.join(path))
                .ok()
                .and_then(|content| Oid::hash_object(ObjectType::Blob, &content).ok())
                == stored_hash
    })
}

#[cfg(test)]
thread_local! { static LISTED: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) }; }

/// The directories this thread's recordings have listed since the last
/// call, in the order they were listed.
#[cfg(test)]
pub(super) fn take_listed() -> Vec<PathBuf> {
    LISTED.with(|listed| std::mem::take(&mut *listed.borrow_mut()))
}

/// The files and directories an observation of one context reads, each
/// stat'ed before it is read, gathered into the context's basis.
///
/// A recording that reads also hashes each file as it is read. One that is
/// stat only reads nothing: a walk that uses it lists the same directories
/// and stats the same files as the observation, which is how a basis is
/// taken by stat alone. Given a stored basis, it lists again only a
/// directory whose stat moved, or was modified within `RACY_WINDOW_NS` of
/// the stored basis's indexed time, and gives the walk the stored entries of
/// every other one.
pub(super) struct Recording<'stored> {
    root: PathBuf,
    reads: bool,
    stored: Option<(&'stored Basis, i64)>,
    files: BTreeMap<PathBuf, BasisFile>,
    dirs: BTreeMap<PathBuf, BasisDir>,
}

impl<'stored> Recording<'stored> {
    /// A recording of an observation of the worktree at `root`.
    pub(super) fn reading(root: &Path) -> Self {
        Self::new(root, true, None)
    }

    /// A recording that reads no file, for the worktree at `root`, that
    /// carries forward what `stored`, indexed at the time it is given with,
    /// listed of a directory that has not moved since.
    pub(super) fn stat_only(root: &Path, stored: Option<(&'stored Basis, i64)>) -> Self {
        Self::new(root, false, stored)
    }

    fn new(root: &Path, reads: bool, stored: Option<(&'stored Basis, i64)>) -> Self {
        Self {
            root: root.to_owned(),
            reads,
            stored,
            files: BTreeMap::new(),
            dirs: BTreeMap::new(),
        }
    }

    /// `fs::symlink_metadata`, recording the file or directory at `path`.
    /// The first stat of a path is the one recorded.
    pub(super) fn stat(&mut self, path: &Path) -> io::Result<fs::Metadata> {
        let metadata = fs::symlink_metadata(path);
        if let Ok(metadata) = &metadata {
            self.record(path, metadata);
        }
        metadata
    }

    /// Lists a directory, its stat recorded first, for a walk that has
    /// `budget` entries left before its limit. A directory a stored basis
    /// listed whole, whose stat has not moved since, which was not modified
    /// within the racy window, and whose entries fit the budget, is carried
    /// forward. Once a listing read now is done, the walk says what it
    /// counted (`listed`); for either kind, it keeps the entries it uses
    /// (`keep`).
    pub(super) fn list(&mut self, path: &Path, budget: usize) -> io::Result<Listing> {
        let metadata = self.stat(path).ok();
        let relative = self.relative(path);
        if let Some(carried) = metadata.and_then(|metadata| self.carried(&relative, &metadata)) {
            let count = carried.count.unwrap_or(u64::MAX);
            if let Ok(count) = usize::try_from(count)
                && count <= budget
            {
                let entries = carried
                    .entries
                    .iter()
                    .map(|entry| Entry {
                        path: path.join(&entry.name),
                        name: entry.name.clone(),
                        kind: entry.kind,
                    })
                    .collect();
                self.start_listing(
                    &relative,
                    BasisListing {
                        entries: Vec::new(),
                        count: carried.count,
                        empty: carried.empty,
                    },
                );
                return Ok(Listing::Carried { entries, count });
            }
        }
        let directory = fs::read_dir(path)?;
        #[cfg(test)]
        LISTED.with(|listed| listed.borrow_mut().push(path.to_owned()));
        self.start_listing(
            &relative,
            BasisListing {
                entries: Vec::new(),
                count: None,
                empty: false,
            },
        );
        Ok(Listing::Read(directory))
    }

    /// Says what a listing read now counted: how many entries, whether it
    /// ended, and whether the directory held none.
    pub(super) fn listed(&mut self, path: &Path, count: usize, ended: bool, empty: bool) {
        let relative = self.relative(path);
        if let Some(listing) = self
            .dirs
            .get_mut(&relative)
            .and_then(|dir| dir.listing.as_mut())
        {
            listing.count = ended.then(|| u64::try_from(count).unwrap_or(u64::MAX));
            listing.empty = empty;
        }
    }

    /// Keeps an entry of a listed directory as one the walk uses.
    pub(super) fn keep(&mut self, entry: &Entry) {
        let Some(directory) = entry.path.parent() else {
            return;
        };
        let relative = self.relative(directory);
        if let Some(listing) = self
            .dirs
            .get_mut(&relative)
            .and_then(|dir| dir.listing.as_mut())
        {
            listing.entries.push(BasisEntry {
                name: entry.name.clone(),
                kind: entry.kind,
            });
        }
    }

    /// `fs::read_to_string` of a file the observation parses, with its stat
    /// recorded first and the hash of what was read after. `None`, having
    /// recorded the stat, when the recording reads nothing.
    pub(super) fn read_to_string(&mut self, path: &Path) -> Option<io::Result<String>> {
        let read = self.read_with(path, |path| fs::read_to_string(path))?;
        if let Ok(source) = &read {
            self.hash(path, source.as_bytes());
        }
        Some(read)
    }

    /// As `read_to_string`, for a file read as bytes.
    pub(super) fn read(&mut self, path: &Path) -> Option<io::Result<Vec<u8>>> {
        let read = self.read_with(path, |path| fs::read(path))?;
        if let Ok(content) = &read {
            self.hash(path, content);
        }
        Some(read)
    }

    /// The basis of what was recorded, with `checkout` for its Git half.
    pub(super) fn finish(self, checkout: Checkout, item_commit: Option<CommitActivity>) -> Basis {
        Basis {
            head: checkout.head,
            branch: checkout.branch,
            worktree: checkout.worktree,
            config_blob: checkout.config_blob,
            item_commit,
            files: self.files.into_values().collect(),
            dirs: self.dirs.into_values().collect(),
        }
    }

    /// The stored listing of a directory that can be carried forward.
    fn carried(&self, relative: &Path, metadata: &fs::Metadata) -> Option<&'stored BasisListing> {
        let (stored, indexed_at_ns) = self.stored?;
        let dir = &stored.dirs[stored
            .dirs
            .binary_search_by(|dir| dir.path.as_path().cmp(relative))
            .ok()?];
        let listing = dir.listing.as_ref()?;
        let (inode, ctime_ns, mtime_ns) = stat_times(metadata);
        let unmoved = metadata.is_dir()
            && (inode, ctime_ns, mtime_ns) == (dir.inode, dir.ctime_ns, dir.mtime_ns)
            && dir.mtime_ns <= indexed_at_ns.saturating_sub(RACY_WINDOW_NS);
        (unmoved && listing.count.is_some()).then_some(listing)
    }

    fn start_listing(&mut self, relative: &Path, listing: BasisListing) {
        if let Some(dir) = self.dirs.get_mut(relative) {
            dir.listing = Some(listing);
        }
    }

    fn read_with<T>(
        &mut self,
        path: &Path,
        read: impl FnOnce(&Path) -> io::Result<T>,
    ) -> Option<io::Result<T>> {
        if !self.files.contains_key(&self.relative(path)) {
            let _ = self.stat(path);
        }
        self.reads.then(|| read(path))
    }

    fn record(&mut self, path: &Path, metadata: &fs::Metadata) {
        let path = self.relative(path);
        if metadata.is_dir() {
            self.dirs
                .entry(path.clone())
                .or_insert_with(|| BasisDir::of(path, metadata));
        } else {
            self.files
                .entry(path.clone())
                .or_insert_with(|| BasisFile::of(path, metadata));
        }
    }

    fn hash(&mut self, path: &Path, content: &[u8]) {
        if let Some(file) = self.files.get_mut(&self.relative(path))
            && file.hash.is_none()
        {
            file.hash = Oid::hash_object(ObjectType::Blob, content).ok();
        }
    }

    fn relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root)
            .map_or_else(|_| path.to_owned(), Path::to_owned)
    }
}

impl BasisFile {
    /// The file's path and stat, without its hash.
    fn stat(&self) -> (&Path, u64, Option<u64>, Option<i64>, i64) {
        (
            &self.path,
            self.size,
            self.inode,
            self.ctime_ns,
            self.mtime_ns,
        )
    }

    fn of(path: PathBuf, metadata: &fs::Metadata) -> Self {
        let (inode, ctime_ns, mtime_ns) = stat_times(metadata);
        Self {
            path,
            size: metadata.len(),
            inode,
            ctime_ns,
            mtime_ns,
            hash: None,
        }
    }
}

impl BasisDir {
    /// The directory's path and what a check compares of its listing: the
    /// entries the walk used, whether it held nothing, and whether the
    /// listing ended.
    fn shape(&self) -> (&Path, Option<ListingShape<'_>>) {
        (
            &self.path,
            self.listing.as_ref().map(|listing| {
                (
                    listing.entries.as_slice(),
                    listing.empty,
                    listing.count.is_some(),
                )
            }),
        )
    }

    fn of(path: PathBuf, metadata: &fs::Metadata) -> Self {
        let (inode, ctime_ns, mtime_ns) = stat_times(metadata);
        Self {
            path,
            inode,
            ctime_ns,
            mtime_ns,
            listing: None,
        }
    }
}

/// The inode, change time and modification time of an entry. The standard
/// library gives no inode or change time on Windows.
fn stat_times(metadata: &fs::Metadata) -> (Option<u64>, Option<i64>, i64) {
    #[cfg(unix)]
    let (inode, ctime_ns) = {
        use std::os::unix::fs::MetadataExt;
        (
            Some(metadata.ino()),
            Some(
                metadata
                    .ctime()
                    .saturating_mul(1_000_000_000)
                    .saturating_add(metadata.ctime_nsec()),
            ),
        )
    };
    #[cfg(not(unix))]
    let (inode, ctime_ns) = (None, None);
    let mtime_ns = metadata.modified().map_or(0, nanoseconds_since_epoch);
    (inode, ctime_ns, mtime_ns)
}

fn nanoseconds_since_epoch(time: SystemTime) -> i64 {
    let nanoseconds =
        |duration: std::time::Duration| i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX);
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => nanoseconds(after),
        Err(before) => -nanoseconds(before.duration()),
    }
}

/// An object ID as `encode` writes one: in full, in lowercase hex.
fn stored_oid(text: &str) -> Option<Oid> {
    let oid = Oid::from_str(text).ok()?;
    (oid.to_string() == text).then_some(oid)
}

/// `Some(None)` for no object ID, `None` for text that is not one.
fn stored_optional_oid(text: Option<String>) -> Option<Option<Oid>> {
    match text {
        Some(text) => stored_oid(&text).map(Some),
        None => Some(None),
    }
}

/// What a check compares of a listing: its entries, whether the directory
/// held nothing, and whether the listing ended.
type ListingShape<'listing> = (&'listing [BasisEntry], bool, bool);

/// A stored file: its path, size, inode, change and modification times,
/// and hash.
type StoredFile = (String, u64, Option<u64>, Option<i64>, i64, Option<String>);

/// A stored listing: each entry's name and type code, the count and whether
/// the directory was empty.
type StoredListing = (Vec<(String, String)>, Option<u64>, bool);

/// A stored directory: its path, inode, change and modification times, and
/// listing.
type StoredDir = (String, Option<u64>, Option<i64>, i64, Option<StoredListing>);

/// The stored form of a basis. The files and directories are tuples, to keep
/// the text of the primary's basis, which names every directory and file it
/// read, short.
#[derive(Serialize, Deserialize)]
struct StoredBasis {
    #[serde(rename = "v")]
    version: u32,
    head: Option<String>,
    branch: Option<String>,
    worktree: String,
    #[serde(rename = "config")]
    config_blob: Option<String>,
    /// Its object ID, commit time in seconds and ordinal.
    #[serde(rename = "commit")]
    item_commit: Option<(String, i64, i64)>,
    files: Vec<StoredFile>,
    dirs: Vec<StoredDir>,
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        fs,
        path::{Path, PathBuf},
        rc::Rc,
    };

    use git2::Repository;
    use rusqlite::{Connection, params};

    use super::*;
    use crate::repository::{
        activity::PrimaryActivityMap,
        discovery::{
            ContextKey, ContextObservation, ListedContext, clear_scan_hook, list_contexts,
            migrate_registry, observe_context, open_registry, set_context_scan_hook, stat_basis,
        },
        git_fixtures::{active_worktree_at, checkpoint, repository_on_main},
        persist_context_index,
    };

    const DOCUMENT_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const TICKET_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    const CONFIG: &str = ".manyhands/config.toml";
    const DOCUMENT: &str = "docs/document.md";
    const DOCUMENT_COMMENT: &str =
        ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md";
    const TICKET: &str = ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md";
    const TICKET_COMMENT: &str =
        ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAW/01ARZ3NDEKTSV4RRFFQ69G5FAY.md";
    const NEW_TICKET_COMMENT: &str =
        ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAW/01ARZ3NDEKTSV4RRFFQ69G5FAZ.md";
    /// Files no Manyhands walk reads, in directories the managed-marker walk
    /// lists.
    const SOURCE: &str = "src/code.rs";
    const NESTED_SOURCE: &str = "src/nested/lib.rs";

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn comment(id: &str, item_id: &str) -> String {
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item_id}\ncreated_at: 2026-09-30T12:00:00Z\n---\n"
        )
    }

    /// Which context a row observes.
    #[derive(Clone, Copy, Debug)]
    enum Of {
        Primary,
        Ticket,
    }

    /// The worktrees a row changes.
    #[derive(Clone)]
    struct Worktrees {
        root: PathBuf,
        ticket: PathBuf,
    }

    /// A primary that holds a document with a comment, all committed, and
    /// source files that are not; and the worktree of a ticket that holds the
    /// ticket with a comment, committed.
    struct Fixture {
        _directory: tempfile::TempDir,
        repository: Repository,
        worktrees: Worktrees,
    }

    impl Fixture {
        fn new() -> Self {
            let (directory, repository) = repository_on_main();
            let root = directory.path().to_owned();
            write(
                &root,
                CONFIG,
                "format_version = 1\nprimary_branch = \"main\"\n",
            );
            write(
                &root,
                DOCUMENT,
                "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Fixture document\n---\n",
            );
            write(
                &root,
                DOCUMENT_COMMENT,
                &comment("01ARZ3NDEKTSV4RRFFQ69G5FAX", DOCUMENT_ID),
            );
            for (path, timestamp) in [(CONFIG, 10), (DOCUMENT, 20), (DOCUMENT_COMMENT, 30)] {
                checkpoint(&repository, Path::new(path), timestamp);
            }
            write(&root, SOURCE, "fn main() {}\n");
            write(&root, NESTED_SOURCE, "pub fn nested() {}\n");
            let ticket = active_worktree_at(&repository, &root, "ticket", TICKET_ID, TICKET_ID);
            let linked = Repository::open(&ticket).unwrap();
            write(
                &ticket,
                TICKET,
                "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAW\ntitle: Fixture ticket\ntype: task\nstatus: open\n---\n",
            );
            write(
                &ticket,
                TICKET_COMMENT,
                &comment("01ARZ3NDEKTSV4RRFFQ69G5FAY", TICKET_ID),
            );
            checkpoint(&linked, Path::new(TICKET), 40);
            checkpoint(&linked, Path::new(TICKET_COMMENT), 50);
            Self {
                _directory: directory,
                repository,
                worktrees: Worktrees { root, ticket },
            }
        }

        fn listed(&self, of: Of) -> ListedContext {
            let key = match of {
                Of::Primary => ContextKey::Primary,
                Of::Ticket => ContextKey::Item(TICKET_ID.parse().unwrap()),
            };
            list_contexts(&self.repository, &self.worktrees.root)
                .unwrap()
                .into_iter()
                .find(|listed| listed.key == key)
                .unwrap()
        }

        fn observe(&self, listed: &ListedContext) -> ContextObservation {
            observe_context(
                &self.repository,
                &self.worktrees.root,
                listed,
                &mut PrimaryActivityMap::default(),
            )
        }

        /// A basis taken by stat alone, every directory listed.
        fn stat(&self, listed: &ListedContext) -> Basis {
            stat_basis(&self.repository, &self.worktrees.root, listed, None, 0)
        }

        /// A basis taken by stat alone, from `stored` indexed at
        /// `indexed_at_ns`.
        fn stat_since(&self, listed: &ListedContext, stored: &Basis, indexed_at_ns: i64) -> Basis {
            stat_basis(
                &self.repository,
                &self.worktrees.root,
                listed,
                Some(stored),
                indexed_at_ns,
            )
        }
    }

    /// An index holding one registration, for the context of the worktree
    /// at `root`, that each observation is stored to as it is indexed.
    struct Index {
        _directory: tempfile::TempDir,
        connection: Connection,
    }

    /// What the index holds for its one context: the row, with what it
    /// records of its last indexing, and the titles of its items.
    type Stored = ((i64, Option<String>, Option<i64>), Vec<String>);

    impl Index {
        fn new(root: &Path) -> Self {
            let directory = tempfile::tempdir().unwrap();
            let mut connection =
                open_registry(&directory.path().join("index.sqlite3"), &mut |_| {}).unwrap();
            migrate_registry(&mut connection).unwrap();
            connection
                .execute(
                    "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                     VALUES (1, ?1, 1, 'accessible', 0)",
                    params![root.to_str()],
                )
                .unwrap();
            Self {
                _directory: directory,
                connection,
            }
        }

        fn store(&mut self, observation: &ContextObservation) {
            let transaction = self.connection.transaction().unwrap();
            persist_context_index(&transaction, 1, observation).unwrap();
            transaction.commit().unwrap();
        }

        fn stored(&self) -> Stored {
            let row = self
                .connection
                .query_row("SELECT id, basis, indexed_at FROM contexts", [], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .unwrap();
            let titles = self
                .connection
                .prepare("SELECT title FROM discovered_items ORDER BY id")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            (row, titles)
        }

        fn dirty(&self) -> i64 {
            self.connection
                .query_row("SELECT dirty FROM contexts", [], |row| row.get(0))
                .unwrap()
        }
    }

    fn edit_document(worktrees: &Worktrees) {
        let path = worktrees.root.join(DOCUMENT);
        let source = fs::read_to_string(&path).unwrap();
        fs::write(&path, format!("{source}edited\n")).unwrap();
    }

    fn same_size_edit(worktrees: &Worktrees) {
        let path = worktrees.root.join(DOCUMENT);
        let source = fs::read_to_string(&path).unwrap();
        let edited = source.replace("Fixture document", "Fixture DOCUMENT");
        assert_eq!(edited.len(), source.len());
        fs::write(&path, edited).unwrap();
    }

    /// A same-size edit written over the file, which then gets back its
    /// modification time, as `cp -p` gives it.
    #[cfg(unix)]
    fn mtime_preserving_copy(worktrees: &Worktrees) {
        use std::os::unix::fs::MetadataExt;

        let path = worktrees.root.join(DOCUMENT);
        let before = fs::metadata(&path).unwrap();
        same_size_edit(worktrees);
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(before.modified().unwrap())
            .unwrap();
        let after = fs::metadata(&path).unwrap();
        assert_eq!(
            (after.len(), after.ino(), after.modified().unwrap()),
            (before.len(), before.ino(), before.modified().unwrap())
        );
    }

    fn switch_branch(worktrees: &Worktrees) {
        let repository = Repository::open(&worktrees.root).unwrap();
        let head = repository.head().unwrap().peel_to_commit().unwrap();
        repository.branch("other", &head, false).unwrap();
        repository.set_head("refs/heads/other").unwrap();
    }

    fn add_comment(worktrees: &Worktrees) {
        write(
            &worktrees.ticket,
            NEW_TICKET_COMMENT,
            &comment("01ARZ3NDEKTSV4RRFFQ69G5FAZ", TICKET_ID),
        );
    }

    fn remove_comment(worktrees: &Worktrees) {
        fs::remove_file(worktrees.ticket.join(TICKET_COMMENT)).unwrap();
    }

    fn make_folder(worktrees: &Worktrees) {
        fs::create_dir(worktrees.root.join("docs/folder")).unwrap();
    }

    fn edit_configuration(worktrees: &Worktrees) {
        write(
            &worktrees.root,
            CONFIG,
            "format_version = 1\nprimary_branch = \"main\"\n# edited\n",
        );
    }

    /// Saves a source file as many editors save: written beside it, then
    /// renamed over it.
    fn save_source_by_rename(worktrees: &Worktrees) {
        let saved = worktrees.root.join("src/.code.rs.tmp");
        fs::write(&saved, "fn main() { saved(); }\n").unwrap();
        fs::rename(&saved, worktrees.root.join(SOURCE)).unwrap();
    }

    fn add_swap_file(worktrees: &Worktrees) {
        write(&worktrees.root, "docs/.document.md.swp", "swap\n");
    }

    fn add_subdirectory(worktrees: &Worktrees) {
        fs::create_dir(worktrees.root.join("src/new")).unwrap();
    }

    fn add_markdown_file(worktrees: &Worktrees) {
        write(&worktrees.root, "src/notes.md", "# Notes\n");
    }

    fn unchanged(_: &Worktrees) {}

    /// What a row's change shows.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Shows {
        /// Made each time the context has been read, before it is stat'ed
        /// again: it is read again, changes again, and is left dirty.
        LeftDirty,
        /// Made the first time only: it is read again, and that read stands.
        ReadAgain,
        /// Made each time the context has been read, to an entry the walk
        /// passes over: it is not read again.
        NotReadAgain,
        /// Made after the observation: a basis taken by stat moved.
        Moved,
        /// Made after the observation, to an entry the walk passes over: a
        /// basis taken by stat is the same, and one taken from the stored
        /// basis lists again only these directories, carrying forward the
        /// listing of every other one, its subdirectories included.
        Same(&'static [&'static str]),
        /// Made after the observation, within a tick of a coarse
        /// filesystem: the stat it gives is the one before, so the file is
        /// racy, and its hash shows the change.
        Racy,
        /// Nothing changes: the racy files settle by their hash, and once the
        /// basis is recorded clean they are not racy, so not hashed, again.
        SettledOnce,
    }

    type Row = (&'static str, Of, fn(&Worktrees), Shows);

    #[test]
    fn observation_and_basis_detect_every_kind_of_change() {
        let mut rows: Vec<Row> = vec![
            (
                "an_edit_during_observation_keeps_the_previous_rows_and_marks_the_context_dirty",
                Of::Primary,
                edit_document,
                Shows::LeftDirty,
            ),
            (
                "an edit during the first read is read again",
                Of::Primary,
                edit_document,
                Shows::ReadAgain,
            ),
            (
                "a same-second same-size edit",
                Of::Primary,
                same_size_edit,
                Shows::Racy,
            ),
            (
                "a branch switch at the same OID",
                Of::Primary,
                switch_branch,
                Shows::Moved,
            ),
            ("a new comment file", Of::Ticket, add_comment, Shows::Moved),
            (
                "a removed comment file",
                Of::Ticket,
                remove_comment,
                Shows::Moved,
            ),
            (
                "a folder made outside Manyhands",
                Of::Primary,
                make_folder,
                Shows::Moved,
            ),
            (
                "a configuration edit",
                Of::Primary,
                edit_configuration,
                Shows::Moved,
            ),
            (
                "a racy entry settled once and not hashed again",
                Of::Primary,
                unchanged,
                Shows::SettledOnce,
            ),
            (
                "a source file saved by rename during the read",
                Of::Primary,
                save_source_by_rename,
                Shows::NotReadAgain,
            ),
            (
                "a source file saved by rename",
                Of::Primary,
                save_source_by_rename,
                Shows::Same(&["src"]),
            ),
            (
                "an editor's swap file beside the document",
                Of::Primary,
                add_swap_file,
                Shows::Same(&["docs"]),
            ),
            (
                "a new subdirectory in a walked directory",
                Of::Primary,
                add_subdirectory,
                Shows::Moved,
            ),
            (
                "a new Markdown file in a walked directory",
                Of::Primary,
                add_markdown_file,
                Shows::Moved,
            ),
        ];
        #[cfg(unix)]
        rows.push((
            "an mtime-preserving copy",
            Of::Primary,
            mtime_preserving_copy,
            Shows::Moved,
        ));

        for (row, of, change, shows) in rows {
            let fixture = Fixture::new();
            let listed = fixture.listed(of);

            if matches!(
                shows,
                Shows::LeftDirty | Shows::ReadAgain | Shows::NotReadAgain
            ) {
                // The context as it was indexed before the edit.
                let previous = (shows == Shows::LeftDirty).then(|| {
                    let mut index = Index::new(&fixture.worktrees.root);
                    let observation = fixture.observe(&listed);
                    assert!(!observation.left_dirty, "{row}");
                    index.store(&observation);
                    assert_eq!(index.dirty(), 0, "{row}");
                    index
                });
                let reads = Rc::new(Cell::new(0));
                let counted = reads.clone();
                let worktrees = fixture.worktrees.clone();
                set_context_scan_hook(listed.worktree.clone(), move || {
                    counted.set(counted.get() + 1);
                    if shows != Shows::ReadAgain || counted.get() == 1 {
                        change(&worktrees);
                    }
                });
                let observation = fixture.observe(&listed);
                clear_scan_hook(&listed.worktree);

                let expected_reads = if shows == Shows::NotReadAgain { 1 } else { 2 };
                assert_eq!(reads.get(), expected_reads, "{row}: reads");
                assert_eq!(observation.left_dirty, shows == Shows::LeftDirty, "{row}");
                if let Some(mut index) = previous {
                    // Storing it keeps the rows indexed before, marked as owed
                    // an indexing.
                    let before = index.stored();
                    assert_eq!(before.1, ["Fixture document"], "{row}");
                    index.store(&observation);
                    assert_eq!(index.stored(), before, "{row}: the previous rows");
                    assert_eq!(index.dirty(), 1, "{row}: the mark");
                }
                if shows == Shows::ReadAgain {
                    assert!(
                        observation.sources.iter().any(|(path, source)| {
                            path == Path::new(DOCUMENT) && source.ends_with("edited\n")
                        }),
                        "{row}: the edit is read"
                    );
                    assert_eq!(
                        check(
                            &observation.basis,
                            &fixture.stat(&listed),
                            observation.started_at_ns + RACY_WINDOW_NS
                        ),
                        BasisCheck::Same,
                        "{row}: the basis is of what was read"
                    );
                }
                continue;
            }

            let observation = fixture.observe(&listed);
            assert!(!observation.left_dirty, "{row}");
            assert!(observation.valid, "{row}");
            // The basis as the index stores it.
            let stored = Basis::decode(&observation.basis.encode()).unwrap();
            assert_eq!(stored, observation.basis, "{row}: stored");
            let clean_at = observation.started_at_ns + RACY_WINDOW_NS;
            // An indexed time as close to the document's writing as can be.
            let written_at = stored
                .files
                .iter()
                .find(|file| file.path == Path::new(DOCUMENT))
                .map(|file| file.mtime_ns);
            let before = fixture.stat(&listed);
            assert_eq!(
                check(&stored, &before, clean_at),
                BasisCheck::Same,
                "{row}: before the change"
            );
            take_listed();
            assert_eq!(
                check(
                    &stored,
                    &fixture.stat_since(&listed, &stored, clean_at),
                    clean_at
                ),
                BasisCheck::Same,
                "{row}: before the change, from the stored basis"
            );
            assert_eq!(take_listed(), Vec::<PathBuf>::new(), "{row}: listed again");

            change(&fixture.worktrees);

            let now = fixture.stat(&listed);
            take_listed();
            let since = fixture.stat_since(&listed, &stored, clean_at);
            let listed_again = take_listed();
            match shows {
                Shows::Moved => {
                    assert_eq!(check(&stored, &now, clean_at), BasisCheck::Moved, "{row}");
                    assert_eq!(
                        check(&stored, &since, clean_at),
                        BasisCheck::Moved,
                        "{row}: from the stored basis"
                    );
                }
                Shows::Same(directories) => {
                    assert_eq!(check(&stored, &now, clean_at), BasisCheck::Same, "{row}");
                    assert_eq!(
                        check(&stored, &since, clean_at),
                        BasisCheck::Same,
                        "{row}: from the stored basis"
                    );
                    assert_eq!(
                        listed_again,
                        directories
                            .iter()
                            .map(|directory| listed.worktree.join(directory))
                            .collect::<Vec<_>>(),
                        "{row}: listed again"
                    );
                }
                Shows::Racy => {
                    let BasisCheck::Racy(racy) = check(&stored, &before, written_at.unwrap())
                    else {
                        panic!("{row}: not racy");
                    };
                    assert!(racy.contains(&PathBuf::from(DOCUMENT)), "{row}: {racy:?}");
                    assert!(
                        !settles(&stored, &listed.worktree, &racy),
                        "{row}: the hash shows the edit"
                    );
                    // A filesystem with finer timestamps shows it by stat.
                    assert_eq!(check(&stored, &now, clean_at), BasisCheck::Moved, "{row}");
                }
                Shows::SettledOnce => {
                    let BasisCheck::Racy(racy) = check(&stored, &now, written_at.unwrap()) else {
                        panic!("{row}: not racy");
                    };
                    assert!(racy.contains(&PathBuf::from(DOCUMENT)), "{row}: {racy:?}");
                    assert!(
                        settles(&stored, &listed.worktree, &racy),
                        "{row}: settled by hash"
                    );
                    // Recorded clean with an indexed time past the window of
                    // every file it holds.
                    let recorded_at = stored.files.iter().map(|file| file.mtime_ns).max().unwrap()
                        + RACY_WINDOW_NS;
                    assert_eq!(
                        check(&stored, &now, recorded_at),
                        BasisCheck::Same,
                        "{row}: not racy again"
                    );
                }
                Shows::LeftDirty | Shows::ReadAgain | Shows::NotReadAgain => unreachable!(),
            }
        }
    }

    #[test]
    fn a_basis_reads_back_only_from_the_text_this_version_writes() {
        let oid = |byte: u8| Oid::from_bytes(&[byte; 20]).unwrap();
        let full = Basis {
            head: Some(oid(1)),
            branch: Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW".to_owned()),
            worktree: PathBuf::from("/repository/.manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAW"),
            config_blob: Some(oid(2)),
            item_commit: Some(CommitActivity {
                oid: oid(3),
                at: OffsetDateTime::from_unix_timestamp(30).unwrap(),
                ordinal: -2,
            }),
            files: vec![BasisFile {
                path: PathBuf::from(TICKET),
                size: 12,
                inode: Some(7),
                ctime_ns: Some(-1),
                mtime_ns: 1_700_000_000_123_456_789,
                hash: Some(oid(4)),
            }],
            dirs: vec![
                BasisDir {
                    path: PathBuf::new(),
                    inode: None,
                    ctime_ns: None,
                    mtime_ns: 5,
                    listing: None,
                },
                BasisDir {
                    path: PathBuf::from(".manyhands/tickets"),
                    inode: Some(8),
                    ctime_ns: Some(6),
                    mtime_ns: 6,
                    listing: Some(BasisListing {
                        entries: [
                            ("01ARZ3NDEKTSV4RRFFQ69G5FAW", Some(EntryKind::Dir)),
                            ("device", Some(EntryKind::Other)),
                            ("link", Some(EntryKind::Symlink)),
                            ("unreadable", None),
                            ("x.md", Some(EntryKind::File)),
                        ]
                        .into_iter()
                        .map(|(name, kind)| BasisEntry {
                            name: OsString::from(name),
                            kind,
                        })
                        .collect(),
                        count: Some(9),
                        empty: false,
                    }),
                },
                BasisDir {
                    path: PathBuf::from("docs"),
                    inode: Some(9),
                    ctime_ns: Some(7),
                    mtime_ns: 7,
                    listing: Some(BasisListing {
                        entries: Vec::new(),
                        count: None,
                        empty: true,
                    }),
                },
            ],
        };
        let empty = Basis::default();
        for basis in [&full, &empty] {
            assert_eq!(Basis::decode(&basis.encode()).as_ref(), Some(basis));
        }

        let encoded = full.encode();
        let other_version = encoded.replacen("\"v\":1", "\"v\":2", 1);
        let short_head = encoded.replacen(&oid(1).to_string(), "0101", 1);
        let uppercase_hash = encoded.replacen(&oid(4).to_string(), &"0A".repeat(20), 1);
        let commit_out_of_range = encoded.replacen(",30,-2]", &format!(",{},-2]", i64::MAX), 1);
        let unknown_kind = encoded.replacen("[\"link\",\"l\"]", "[\"link\",\"x\"]", 1);
        for (row, text) in [
            ("an entry of a type no basis names", unknown_kind.as_str()),
            ("another version", &other_version),
            ("a short object ID", &short_head),
            ("an object ID not written in lowercase", &uppercase_hash),
            ("a commit time out of range", &commit_out_of_range),
            ("no version", "{}"),
            ("not JSON", "basis"),
            ("nothing", ""),
        ] {
            assert_ne!(text, encoded, "{row}: the text is not changed");
            assert_eq!(Basis::decode(text), None, "{row}");
        }
    }
}
