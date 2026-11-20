//! What an observation of one context depended on, by stat: enough to tell,
//! without reading a file, whether the context may have changed since.
//!
//! A basis holds the context's head, the branch its HEAD names, where its
//! worktree is registered and the configuration blob of its head, then the
//! stat of every file the observation read and every directory it listed or
//! looked for. An observation records the content hash of each file as it
//! read it; a basis taken by stat alone has none.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use git2::{ObjectType, Oid};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::activity::CommitActivity;

/// How long after its modification a file's stat alone is not trusted: on a
/// filesystem with coarse timestamps a file can change again within the same
/// tick without its stat moving.
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BasisDir {
    pub(super) path: PathBuf,
    pub(super) inode: Option<u64>,
    pub(super) ctime_ns: Option<i64>,
    pub(super) mtime_ns: i64,
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
#[cfg_attr(not(test), allow(dead_code))] // Used by the per-context indexing that follows.
pub(super) enum BasisCheck {
    Same,
    /// The stats are the same, but these files were modified too close to
    /// the indexed time for their stat to be trusted. Each is the same file
    /// when its content still has the stored hash (`settles`).
    Racy(Vec<PathBuf>),
    Moved,
}

impl Basis {
    /// The same files and directories stat'ed again under `root`, with
    /// `checkout` for the Git half: what an observation compares with its
    /// basis to tell whether anything moved while it read. An entry that is
    /// gone, or is no longer of its kind, is left out.
    pub(super) fn restat(&self, root: &Path, checkout: Checkout) -> Self {
        let files = self
            .files
            .iter()
            .filter_map(|file| {
                let metadata = fs::symlink_metadata(root.join(&file.path)).ok()?;
                (!metadata.is_dir()).then(|| BasisFile::of(file.path.clone(), &metadata))
            })
            .collect();
        let dirs = self
            .dirs
            .iter()
            .filter_map(|dir| {
                let metadata = fs::symlink_metadata(root.join(&dir.path)).ok()?;
                metadata
                    .is_dir()
                    .then(|| BasisDir::of(dir.path.clone(), &metadata))
            })
            .collect();
        Self::new(checkout, None, files, dirs)
    }

    /// Whether `other` differs from this basis in anything a check compares:
    /// the Git half, which files and directories there are, or the stat of
    /// one. Hashes and the item commit are not compared.
    pub(super) fn differs(&self, other: &Self) -> bool {
        self.head != other.head
            || self.branch != other.branch
            || self.worktree != other.worktree
            || self.config_blob != other.config_blob
            || self.dirs != other.dirs
            || !self
                .files
                .iter()
                .map(BasisFile::stat)
                .eq(other.files.iter().map(BasisFile::stat))
    }

    /// The stored text of the basis.
    ///
    /// A path is stored as text, so one that is not UTF-8 does not read back
    /// as itself: a basis that holds one never checks as the same, and its
    /// context is observed each time.
    #[cfg_attr(not(test), allow(dead_code))] // Stored by the per-context indexing that follows.
    pub(super) fn encode(&self) -> String {
        let text = |path: &Path| path.to_string_lossy().into_owned();
        let stored = StoredBasis {
            version: BASIS_VERSION,
            head: self.head.map(|oid| oid.to_string()),
            branch: self.branch.clone(),
            worktree: text(&self.worktree),
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
                        text(&file.path),
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
                .map(|dir| (text(&dir.path), dir.inode, dir.ctime_ns, dir.mtime_ns))
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
            .map(|(path, inode, ctime_ns, mtime_ns)| BasisDir {
                path: PathBuf::from(path),
                inode,
                ctime_ns,
                mtime_ns,
            })
            .collect();
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

    fn new(
        checkout: Checkout,
        item_commit: Option<CommitActivity>,
        files: Vec<BasisFile>,
        dirs: Vec<BasisDir>,
    ) -> Self {
        Self {
            head: checkout.head,
            branch: checkout.branch,
            worktree: checkout.worktree,
            config_blob: checkout.config_blob,
            item_commit,
            files,
            dirs,
        }
    }
}

/// Compares the basis stored for a context, indexed at `indexed_at_ns`,
/// with the one it has now.
///
/// A file not modified at least `RACY_WINDOW_NS` before the indexed time is
/// racy even when its stat is the same. A directory is never racy: a basis
/// taken by `stat_basis` lists it again, so an entry it has gained or lost
/// since is in the current basis.
#[cfg_attr(not(test), allow(dead_code))] // Used by the per-context indexing that follows.
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
#[cfg_attr(not(test), allow(dead_code))] // Used by the per-context indexing that follows.
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

/// The files and directories an observation of one context reads, each
/// stat'ed before it is read, gathered into the context's basis.
///
/// A recording that reads also hashes each file as it is read. One that is
/// stat only reads nothing: a walk that uses it lists the same directories
/// and stats the same files as the observation, which is how a basis is
/// taken by stat alone.
pub(super) struct Recording {
    root: PathBuf,
    reads: bool,
    files: BTreeMap<PathBuf, BasisFile>,
    dirs: BTreeMap<PathBuf, BasisDir>,
}

impl Recording {
    /// A recording of an observation of the worktree at `root`.
    pub(super) fn reading(root: &Path) -> Self {
        Self::new(root, true)
    }

    /// A recording that reads no file, for the worktree at `root`.
    pub(super) fn stat_only(root: &Path) -> Self {
        Self::new(root, false)
    }

    fn new(root: &Path, reads: bool) -> Self {
        Self {
            root: root.to_owned(),
            reads,
            files: BTreeMap::new(),
            dirs: BTreeMap::new(),
        }
    }

    /// `fs::symlink_metadata`, recording what is at `path`: the file or
    /// directory, or, when there is nothing, the directory above it, whose
    /// stat moves when something appears there. The first stat of a path is
    /// the one recorded.
    pub(super) fn stat(&mut self, path: &Path) -> io::Result<fs::Metadata> {
        let metadata = fs::symlink_metadata(path);
        match &metadata {
            Ok(metadata) => self.record(path, metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if let Some(parent) = path.parent()
                    && let Ok(above) = fs::symlink_metadata(parent)
                {
                    self.record(parent, &above);
                }
            }
            Err(_) => {}
        }
        metadata
    }

    /// `fs::read_dir`, with the directory's stat recorded first.
    pub(super) fn list(&mut self, path: &Path) -> io::Result<fs::ReadDir> {
        let _ = self.stat(path);
        fs::read_dir(path)
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
        Basis::new(
            checkout,
            item_commit,
            self.files.into_values().collect(),
            self.dirs.into_values().collect(),
        )
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
    fn of(path: PathBuf, metadata: &fs::Metadata) -> Self {
        let (inode, ctime_ns, mtime_ns) = stat_times(metadata);
        Self {
            path,
            inode,
            ctime_ns,
            mtime_ns,
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

/// A stored file: its path, size, inode, change and modification times,
/// and hash.
type StoredFile = (String, u64, Option<u64>, Option<i64>, i64, Option<String>);

/// A stored directory: its path, inode, change and modification times.
type StoredDir = (String, Option<u64>, Option<i64>, i64);

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

    use super::*;
    use crate::repository::{
        activity::PrimaryActivityMap,
        discovery::{
            ContextKey, ContextObservation, ListedContext, clear_scan_hook, list_contexts,
            observe_context, set_context_scan_hook, stat_basis,
        },
        git_fixtures::{active_worktree_at, checkpoint, repository_on_main},
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

    /// A primary that holds a document with a comment, and the worktree of a
    /// ticket that holds the ticket with a comment, all committed.
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

        fn stat(&self, listed: &ListedContext) -> Basis {
            stat_basis(&self.repository, &self.worktrees.root, listed)
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

    fn unchanged(_: &Worktrees) {}

    /// What a row's change shows.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Shows {
        /// Made each time the context has been read, before it is stat'ed
        /// again: it is read again, changes again, and is left dirty.
        LeftDirty,
        /// Made the first time only: it is read again, and that read stands.
        ReadAgain,
        /// Made after the observation: a basis taken by stat moved.
        Moved,
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

            if matches!(shows, Shows::LeftDirty | Shows::ReadAgain) {
                let reads = Rc::new(Cell::new(0));
                let counted = reads.clone();
                let worktrees = fixture.worktrees.clone();
                set_context_scan_hook(listed.worktree.clone(), move || {
                    counted.set(counted.get() + 1);
                    if shows == Shows::LeftDirty || counted.get() == 1 {
                        change(&worktrees);
                    }
                });
                let observation = fixture.observe(&listed);
                clear_scan_hook(&listed.worktree);

                assert_eq!(reads.get(), 2, "{row}: read again");
                assert_eq!(observation.left_dirty, shows == Shows::LeftDirty, "{row}");
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

            change(&fixture.worktrees);

            let now = fixture.stat(&listed);
            match shows {
                Shows::Moved => {
                    assert_eq!(check(&stored, &now, clean_at), BasisCheck::Moved, "{row}");
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
                Shows::LeftDirty | Shows::ReadAgain => unreachable!(),
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
                },
                BasisDir {
                    path: PathBuf::from(".manyhands/tickets"),
                    inode: Some(8),
                    ctime_ns: Some(6),
                    mtime_ns: 6,
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
        for (row, text) in [
            ("another version", other_version.as_str()),
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
