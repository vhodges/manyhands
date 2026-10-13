//! Folder creation: one directory under `docs/` in the primary worktree,
//! with every directory above it that is missing.
//!
//! **Directories only.** No file is written, nothing is staged and
//! nothing is committed. Git does not track an empty directory, so the
//! worktree stays clean, and an empty folder is not in a clone: it is on
//! this machine until a document is saved in it.
//!
//! **What a folder path is.** `DocumentFolderPath` is the one place that
//! decides, and it decides the same on every platform: what a file
//! system of one of them would refuse, or would take to mean another
//! name, is refused on all of them. So a path that is accepted names
//! the same folder wherever the repository is cloned.
//!
//! **No link is followed.** The directories are made by the walk every
//! owned write uses, which opens each directory it passes without
//! following a link. A link in the path is refused before anything is
//! made, whatever it names: nothing is made through one, inside the
//! repository or out of it.
//!
//! **The limits of discovery.** A refresh reads `docs/` sixteen
//! directories deep and 1,024 entries wide, and past either it reports
//! the worktree as not wholly observed. A folder is not made that would
//! be the thing that crosses one: the request is refused and nothing is
//! made.
//!
//! **No row of its own.** Making directories records nothing: it is
//! idempotent, and a call that died part of the way left directories a
//! second call finds and goes on from. The one row is that of the
//! refresh that follows, under the caller's operation ID. A refresh that
//! does not bring the index up to date has its row closed, as that of a
//! call that left nothing behind is, so that it does not stand in the
//! way of the next refresh. The registration is marked stale before
//! anything is made, and stays so until a refresh completes.
//!
//! **Two calls under one operation ID.** The boundary does not keep two
//! calls of one request from running at once: the second raises the
//! record's attempt and goes on beside the first. Each takes the
//! repository lease to make directories, so those are serialized. The
//! refresh holds no lease while it observes, so one call can find the
//! row claimed by the other. It does not close a row under a call that
//! is at work: a row whose heartbeat is fresh is left as it is, as an
//! abandon leaves it, and the call says the index is pending under the
//! operation.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use rusqlite::TransactionBehavior;

use super::super::{
    FailurePoint, MAX_DOCUMENT_DIRECTORY_DEPTH, MAX_DOCUMENT_DIRECTORY_ENTRIES, OperationId,
    RefreshOutcome, RefreshRepositoryRequest, RepositoryError, RepositoryErrorKind,
    RepositoryOperation, RepositoryService,
    bridges::abandon::stale_heartbeat,
    cache_read_guard, cache_write_guard, canonical_repository_root, discovery,
    ensure_safe_owned_parent, mark_document_refresh_required, open_registry,
    recovery::{self, LocalStanding},
    registered_repository_id, registry_root_key, repository_lease,
};

const OPERATION: RepositoryOperation = RepositoryOperation::CreateFolder;

/// The directory every folder is under.
const DOCUMENTS: &str = "docs";

/// The longest a folder's name may be, in bytes of UTF-8: the shortest
/// of the limits the supported file systems put on one name.
const LONGEST_NAME: usize = 255;

/// Why a path is not one a folder can be made at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentFolderPathProblem {
    /// It does not begin with `docs/`, or names `docs` itself.
    NotUnderDocuments,
    /// It is absolute: it begins with a separator or a drive.
    Absolute,
    /// It has an empty segment: two separators together, or one at its
    /// end.
    EmptySegment,
    /// It has a `.` or `..` segment.
    Relative,
    /// It names more directories below `docs` than discovery reads.
    TooDeep,
    /// A segment has a character that is not a name's on every
    /// platform: a backslash, a colon, one of `* ? " < > |`, a control
    /// character, or one a file system ignores when it compares names.
    Character,
    /// A segment ends with a dot or a space, which Windows drops.
    TrailingDotOrSpace,
    /// A segment is a device's name on Windows, with or without an
    /// extension: `CON`, `NUL`, `COM1` and the rest.
    DeviceName,
    /// A segment is Git's own directory, or a name a file system takes
    /// for it.
    GitDirectory,
    /// A segment is longer than a file system lets a name be.
    TooLong,
}

impl fmt::Display for DocumentFolderPathProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotUnderDocuments => "a folder is under docs/",
            Self::Absolute => "a folder path is relative to the repository",
            Self::EmptySegment => "a folder path has no empty segment",
            Self::Relative => "a folder path has no . or .. segment",
            Self::TooDeep => "a folder is at most sixteen directories below docs",
            Self::Character => "a folder name has a character that is not portable",
            Self::TrailingDotOrSpace => "a folder name does not end with a dot or a space",
            Self::DeviceName => "a folder name is not a device name",
            Self::GitDirectory => "a folder is not Git's own directory",
            Self::TooLong => "a folder name is at most 255 bytes",
        })
    }
}

impl std::error::Error for DocumentFolderPathProblem {}

/// A path a folder can be made at: `docs/` and one to sixteen names,
/// written with forward slashes.
///
/// The rules are the same on every platform, and are those of the
/// strictest: a path that parses is a path every supported file system
/// takes as written. See `DocumentFolderPathProblem` for each. Two
/// things are not decided here, because only a directory can say:
/// whether a name is there already in another case, which
/// `create_folder` refuses as occupied wherever it runs, and whether
/// the whole path is longer than the file system the repository is on
/// allows, which is that file system's to refuse.
///
/// Unicode normalization is not checked. A name with a letter that can
/// be written composed or decomposed is two names on Linux and on
/// Windows and one on macOS; this build has no normalization table to
/// tell such names by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentFolderPath {
    /// The names below `docs`, outermost first. Never empty.
    names: Vec<String>,
}

impl DocumentFolderPath {
    /// Reads `path` as a folder path, or says why it is not one. Nothing
    /// is read from any file system.
    pub fn parse(path: &str) -> Result<Self, DocumentFolderPathProblem> {
        use DocumentFolderPathProblem as Problem;
        let mut leading = path.chars();
        let drive = matches!(
            (leading.next(), leading.next()),
            (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
        );
        if drive || path.starts_with(['/', '\\']) {
            return Err(Problem::Absolute);
        }
        let mut segments = path.split('/');
        if segments.next() != Some(DOCUMENTS) {
            return Err(Problem::NotUnderDocuments);
        }
        let names: Vec<&str> = segments.collect();
        if names.is_empty() {
            return Err(Problem::NotUnderDocuments);
        }
        for name in &names {
            check_name(name)?;
        }
        if names.len() > MAX_DOCUMENT_DIRECTORY_DEPTH {
            return Err(Problem::TooDeep);
        }
        Ok(Self {
            names: names.into_iter().map(str::to_owned).collect(),
        })
    }

    /// The path below the repository root, with forward slashes.
    pub fn as_string(&self) -> String {
        format!("{DOCUMENTS}/{}", self.names.join("/"))
    }

    /// `docs` and every name below it, outermost first.
    fn segments(&self) -> impl Iterator<Item = &str> {
        std::iter::once(DOCUMENTS).chain(self.names.iter().map(String::as_str))
    }
}

impl fmt::Display for DocumentFolderPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_string())
    }
}

/// The characters HFS+ ignores when it compares names, as Git lists
/// them for the same reason: a name that holds one can be another name
/// to the file system, `.git` among them.
const fn is_ignored_in_comparison(character: char) -> bool {
    matches!(
        character,
        '\u{200c}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{206a}'..='\u{206f}' | '\u{feff}'
    )
}

/// The device names of Windows. A name is a device's whatever follows
/// its first dot, and in any case.
const DEVICE_NAMES: [&str; 6] = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];

fn is_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end_matches(' ');
    let stem = stem.to_uppercase();
    if DEVICE_NAMES.contains(&stem.as_str()) {
        return true;
    }
    // `COM` and `LPT` with one digit, which may be a superscript one.
    let mut characters = stem.chars();
    let prefix: String = characters.by_ref().take(3).collect();
    let number = (characters.next(), characters.next());
    matches!(prefix.as_str(), "COM" | "LPT")
        && matches!(
            number,
            (Some('0'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}'), None)
        )
}

fn check_name(name: &str) -> Result<(), DocumentFolderPathProblem> {
    use DocumentFolderPathProblem as Problem;
    if name.is_empty() {
        return Err(Problem::EmptySegment);
    }
    if name == "." || name == ".." {
        return Err(Problem::Relative);
    }
    if name.chars().any(|character| {
        character.is_control()
            || is_ignored_in_comparison(character)
            || matches!(character, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        return Err(Problem::Character);
    }
    if name.ends_with(['.', ' ']) {
        return Err(Problem::TrailingDotOrSpace);
    }
    if name.len() > LONGEST_NAME {
        return Err(Problem::TooLong);
    }
    // `git~1` is the short name NTFS gives `.git`.
    if name.eq_ignore_ascii_case(".git") || name.eq_ignore_ascii_case("git~1") {
        return Err(Problem::GitDirectory);
    }
    if is_device_name(name) {
        return Err(Problem::DeviceName);
    }
    Ok(())
}

/// What `create_folder` is asked for.
#[derive(Clone, Debug)]
pub struct CreateFolderRequest {
    /// The root of a registered repository.
    pub root: PathBuf,
    pub path: DocumentFolderPath,
    /// The ID the refresh that follows runs under.
    pub operation_id: OperationId,
}

/// Whether the index holds the folder once `create_folder` has
/// returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDiscovery {
    /// A refresh completed after the directory was there.
    Current,
    /// The refresh did not bring the index up to date, and nothing of
    /// it is left pending under the operation ID: any refresh that
    /// completes brings the index up to date.
    Pending,
    /// As `Pending`, except that the refresh's row was not closed and
    /// may be pending still: it could not be written, or another call
    /// under the same operation ID is at work under it. A call under
    /// that ID takes it up, and until one does, or the call at work
    /// completes it, it refuses every other refresh.
    PendingUnderOperation,
}

/// What `create_folder` found or did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateFolderOutcome {
    /// The directory was made, with any above it that was missing.
    Created(FolderDiscovery),
    /// The directory was there already. Nothing was made.
    AlreadyPresent(FolderDiscovery),
    /// Something that is no directory is at the path or at a directory
    /// above it, or a name there differs from the one asked for only by
    /// case. Nothing was made.
    Occupied,
    /// The folder would be more than discovery reads under `docs/`.
    /// Nothing was made.
    LimitExceeded,
}

/// A `create_folder` that failed.
#[derive(Debug)]
pub struct CreateFolderError {
    pub error: RepositoryError,
    /// A directory the call set out to make is there: the call failed
    /// after it had made one. `false` for a call that made nothing.
    pub left_directories: bool,
}

impl CreateFolderError {
    fn before(error: RepositoryError) -> Self {
        Self {
            error,
            left_directories: false,
        }
    }
}

impl fmt::Display for CreateFolderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for CreateFolderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// What is at one name in a directory, read from the directory's own
/// listing so that the answer is the same on a file system that folds
/// case as on one that does not.
enum Named {
    Directory,
    /// A symbolic link, or what a platform treats as one.
    Link,
    /// A file, or anything else that is neither.
    Other,
    /// No entry has the name, and one has it in another case.
    InAnotherCase,
    Missing,
}

/// Whether two names are one name to a file system that folds case.
fn same_but_for_case(left: &str, right: &str) -> bool {
    left != right
        && (left.to_lowercase() == right.to_lowercase()
            || left.to_uppercase() == right.to_uppercase())
}

fn named(directory: &Path, name: &str) -> std::io::Result<Named> {
    let mut in_another_case = false;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let found = entry.file_name();
        let Some(found) = found.to_str() else {
            continue;
        };
        if found == name {
            let kind = entry.file_type()?;
            return Ok(if kind.is_symlink() {
                Named::Link
            } else if kind.is_dir() {
                Named::Directory
            } else {
                Named::Other
            });
        }
        in_another_case |= same_but_for_case(found, name);
    }
    Ok(if in_another_case {
        Named::InAnotherCase
    } else {
        Named::Missing
    })
}

/// Whether `path`, written with forward slashes, names a file under
/// `root` by the names its directories list: every segment but the last
/// a directory, the last a file, none of them a link, and none of them
/// there only under another case of the name.
///
/// A file system that folds case opens `docs/Notes/a.md` for
/// `docs/notes/a.md`. A write under the first name would put a second
/// path in a commit beside the first, so a caller that is to write where
/// it read asks this of the path it was given, and gets the same answer
/// on every platform.
pub(crate) fn is_listed_file(root: &Path, path: &str) -> std::io::Result<bool> {
    let mut directory = root.to_owned();
    let mut segments = path.split('/').peekable();
    while let Some(name) = segments.next() {
        let last = segments.peek().is_none();
        match named(&directory, name)? {
            Named::Directory if !last => directory.push(name),
            Named::Other if last => return Ok(true),
            Named::Directory
            | Named::Other
            | Named::Link
            | Named::InAnotherCase
            | Named::Missing => {
                return Ok(false);
            }
        }
    }
    Ok(false)
}

/// What the path's directories are, before anything is made.
enum Observed {
    /// Every one is there.
    Present,
    /// `first` is the outermost that is missing, and `to_make` is how
    /// many are, it and every one below it.
    Missing {
        first: PathBuf,
        to_make: usize,
    },
    Occupied,
}

fn observe(root: &Path, path: &DocumentFolderPath) -> Result<Observed, RepositoryError> {
    let io = |error| RepositoryError::io(OPERATION, Some(root.to_owned()), error);
    let total = path.segments().count();
    let mut directory = root.to_owned();
    for (position, name) in path.segments().enumerate() {
        match named(&directory, name).map_err(io)? {
            Named::Directory => directory.push(name),
            Named::Missing => {
                return Ok(Observed::Missing {
                    first: directory.join(name),
                    to_make: total - position,
                });
            }
            Named::Other | Named::InAnotherCase => return Ok(Observed::Occupied),
            Named::Link => {
                return Err(RepositoryError::new(
                    OPERATION,
                    Some(root.to_owned()),
                    RepositoryErrorKind::InvalidPath,
                    "a folder is not made through a symbolic link",
                ));
            }
        }
    }
    Ok(Observed::Present)
}

impl RepositoryService {
    /// Makes the directory `request.path` under `docs/` in the primary
    /// worktree of the registered repository at `request.root`, with
    /// every directory above it that is missing, and then refreshes the
    /// index under `request.operation_id`.
    ///
    /// It is idempotent: a directory that is there is left as it is,
    /// and the refresh still runs, so that the index holds the folder
    /// whichever call made it. Nothing but directories is made; see the
    /// module's description.
    ///
    /// Refused, with nothing made: a path with a symbolic link in it,
    /// `InvalidPath`; a repository on which the refresh would be
    /// refused, for another operation pending or a remote operation's
    /// reservation, `RecoveryRequired`; one that is not registered,
    /// `RepositoryNotRegistered`; one whose lease is held,
    /// `RepositoryBusy`; one whose registration could not be marked
    /// stale, `RegistryRefreshPending`.
    pub fn create_folder(
        &self,
        request: CreateFolderRequest,
    ) -> Result<CreateFolderOutcome, CreateFolderError> {
        let CreateFolderRequest {
            root,
            path,
            operation_id,
        } = request;
        let (repository, root) =
            canonical_repository_root(&root, OPERATION).map_err(CreateFolderError::before)?;
        let created = {
            let _lease = repository_lease(&repository, &root, OPERATION)
                .map_err(CreateFolderError::before)?;
            self.make_directories(&root, &path, operation_id)?
        };
        let created = match created {
            Made::Created => true,
            Made::AlreadyPresent => false,
            Made::Refused(outcome) => return Ok(outcome),
        };
        // From here the directory is there, and the index has said it
        // is behind since before it was: whatever becomes of the
        // refresh, it says so until one completes.
        let discovery = self.refresh_for_folder(&root, operation_id);
        Ok(if created {
            CreateFolderOutcome::Created(discovery)
        } else {
            CreateFolderOutcome::AlreadyPresent(discovery)
        })
    }

    /// Everything `create_folder` does under the repository lease.
    fn make_directories(
        &self,
        root: &Path,
        path: &DocumentFolderPath,
        operation_id: OperationId,
    ) -> Result<Made, CreateFolderError> {
        registered_repository_id(&self.registry_path, root, OPERATION)
            .map_err(CreateFolderError::before)?;
        self.require_refresh_can_begin(root, operation_id)
            .map_err(CreateFolderError::before)?;
        // The mark is written before anything is made, and a call that
        // cannot write it makes nothing: the index never says it is
        // current while it lacks a folder this call made. A mark with
        // no change behind it costs a refresh and misleads no one.
        let mark = || {
            self.check_failure(FailurePoint::BeforeRegistryWrite, OPERATION, root)
                .and_then(|()| mark_document_refresh_required(&self.registry_path, root, OPERATION))
                .map_err(CreateFolderError::before)
        };
        let (first, to_make) = match observe(root, path).map_err(CreateFolderError::before)? {
            Observed::Present => {
                mark()?;
                return Ok(Made::AlreadyPresent);
            }
            Observed::Occupied => return Ok(Made::Refused(CreateFolderOutcome::Occupied)),
            Observed::Missing { first, to_make } => (first, to_make),
        };
        // `docs` is not an entry under `docs`.
        let entries_to_make = if first == root.join(DOCUMENTS) {
            to_make - 1
        } else {
            to_make
        };
        let within = discovery::count_document_entries(root)
            .is_some_and(|entries| entries + entries_to_make <= MAX_DOCUMENT_DIRECTORY_ENTRIES);
        if !within {
            return Ok(Made::Refused(CreateFolderOutcome::LimitExceeded));
        }
        mark()?;
        // The walk makes the parent of the path it is given, so it is
        // given a name in the folder. Nothing is made at that name.
        let inside = Path::new(&path.as_string()).join("inside");
        let made = ensure_safe_owned_parent(root, &inside, OPERATION, root)
            .and_then(|()| self.check_failure(FailurePoint::AfterFolderCreation, OPERATION, root));
        made.map_err(|error| CreateFolderError {
            error,
            left_directories: fs::symlink_metadata(&first)
                .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink()),
        })?;
        Ok(Made::Created)
    }

    /// Refuses a repository on which the refresh that follows would be
    /// refused before it began: the directory would be made, and the
    /// index could not then be told of it. The test is the refresh's
    /// own; see `recovery::require_refresh_can_begin`.
    fn require_refresh_can_begin(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<(), RepositoryError> {
        let _cache_guard = cache_read_guard(&self.registry_path, root, OPERATION)?;
        let connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(OPERATION, root))?;
        recovery::require_refresh_can_begin(&connection, root, OPERATION, operation_id)
            .map_err(|error| error.for_operation(OPERATION, root))
    }

    /// Refreshes the index under `operation_id` and says whether it now
    /// holds what the repository does.
    ///
    /// A refresh that did not complete leaves its row pending, which
    /// refuses every other operation on the repository until a call
    /// under the same ID takes it up. Here nothing would: the folder is
    /// made and the request is over. So the row is closed, as that of a
    /// call that left nothing behind, and the next refresh, under any
    /// ID, is what brings the index up to date.
    fn refresh_for_folder(&self, root: &Path, operation_id: OperationId) -> FolderDiscovery {
        let refreshed = self.refresh_repository(RefreshRepositoryRequest {
            root: root.to_owned(),
            operation_id,
        });
        if matches!(refreshed, Ok(RefreshOutcome::Refreshed { snapshot: _ })) {
            return FolderDiscovery::Current;
        }
        match self.close_refresh(root, operation_id) {
            // Another call under the same ID completed the refresh.
            Ok(Closed::Completed) => FolderDiscovery::Current,
            Ok(Closed::NothingPending) => FolderDiscovery::Pending,
            // The row is, or may be, pending still: a call under the
            // same ID is what takes it up.
            Ok(Closed::AtWork) | Err(_) => FolderDiscovery::PendingUnderOperation,
        }
    }

    /// Closes the row a refresh under `operation_id` left pending, unless
    /// a call is at work under it.
    fn close_refresh(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<Closed, RepositoryError> {
        self.check_failure(FailurePoint::BeforeFolderRefreshClose, OPERATION, root)?;
        let _cache_guard = cache_write_guard(&self.registry_path, root, OPERATION)?;
        let root_path = registry_root_key(root, OPERATION)?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|error| error.for_operation(OPERATION, root))?;
        let sqlite = |error| RepositoryError::sqlite(error).for_operation(OPERATION, root);
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite)?;
        let row = recovery::local_row(&transaction, root_path, operation_id).map_err(sqlite)?;
        let closed = match row {
            None => Closed::NothingPending,
            Some(row) => match row.standing() {
                LocalStanding::Completed => Closed::Completed,
                LocalStanding::Rejected | LocalStanding::Abandoned => Closed::NothingPending,
                // An index owner whose heartbeat is fresh is at work,
                // with no lease held: another call of the same request.
                // Its row is not closed under it.
                LocalStanding::Pending if row.has_live_heartbeat(stale_heartbeat()) => {
                    Closed::AtWork
                }
                LocalStanding::Pending => {
                    recovery::discard_operation(&transaction, row.id)
                        .map_err(|error| error.for_operation(OPERATION, root))?;
                    Closed::NothingPending
                }
            },
        };
        transaction.commit().map_err(sqlite)?;
        Ok(closed)
    }
}

/// What `close_refresh` found of the refresh's row.
enum Closed {
    /// The row has completed: the index was brought up to date under
    /// the ID, by another call.
    Completed,
    /// No row is pending under the ID: there was none, or it was closed
    /// here or before.
    NothingPending,
    /// The row is pending and a call is at work under it. It was left
    /// as it is.
    AtWork,
}

/// What `make_directories` came to.
enum Made {
    Created,
    AlreadyPresent,
    Refused(CreateFolderOutcome),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{DocumentFolderPath, DocumentFolderPathProblem as Problem, is_listed_file};

    fn problem(path: &str) -> Option<Problem> {
        DocumentFolderPath::parse(path).err()
    }

    #[test]
    fn a_folder_path_is_docs_and_one_to_sixteen_portable_names() {
        for path in [
            "docs/a",
            "docs/a/b",
            "docs/Notes 2026/Q3",
            "docs/.hidden",
            "docs/a.b",
            "docs/ leading-space",
            "docs/docs",
            "docs/r\u{e9}sum\u{e9}/\u{65e5}\u{672c}\u{8a9e}",
            // Not devices: a device's name is the whole stem.
            "docs/console",
            "docs/com",
            "docs/com10",
            "docs/nul-device",
            "docs/git",
            "docs/.github",
        ] {
            let parsed = DocumentFolderPath::parse(path).unwrap_or_else(|problem| {
                panic!("{path:?}: {problem}");
            });
            assert_eq!(parsed.as_string(), path);
            assert_eq!(parsed.to_string(), path);
        }
        let sixteen = format!("docs/{}", ["n"; 16].join("/"));
        assert!(DocumentFolderPath::parse(&sixteen).is_ok());
        let longest = format!("docs/{}", "n".repeat(255));
        assert!(DocumentFolderPath::parse(&longest).is_ok());
    }

    #[test]
    fn a_file_is_listed_only_under_the_names_its_directories_give_it() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("docs/notes")).unwrap();
        fs::write(root.join("docs/notes/marker.md"), "text").unwrap();

        assert!(is_listed_file(root, "docs/notes/marker.md").unwrap());
        // Another case of a directory's name or of the file's: a file
        // system that folds case would open the file all the same.
        for path in [
            "docs/Notes/marker.md",
            "Docs/notes/marker.md",
            "docs/notes/Marker.md",
            "docs/notes/MARKER.MD",
        ] {
            assert!(!is_listed_file(root, path).unwrap(), "{path}");
        }
        // Nothing there, a directory where the file is to be, and a
        // file where a directory is to be.
        for path in [
            "docs/notes/absent.md",
            "docs/absent/marker.md",
            "docs/notes",
            "docs/notes/marker.md/more.md",
        ] {
            assert!(!is_listed_file(root, path).unwrap(), "{path}");
        }
    }

    /// Unix only: a symbolic link is made.
    #[cfg(unix)]
    #[test]
    fn a_file_reached_through_a_link_is_not_listed() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("docs/notes")).unwrap();
        fs::write(root.join("docs/notes/marker.md"), "text").unwrap();
        std::os::unix::fs::symlink(root.join("docs/notes"), root.join("docs/linked")).unwrap();
        std::os::unix::fs::symlink(
            root.join("docs/notes/marker.md"),
            root.join("docs/notes/link.md"),
        )
        .unwrap();

        assert!(!is_listed_file(root, "docs/linked/marker.md").unwrap());
        assert!(!is_listed_file(root, "docs/notes/link.md").unwrap());
        assert!(is_listed_file(root, "docs/notes/marker.md").unwrap());
    }

    #[test]
    fn what_is_not_a_folder_path_is_refused_for_what_it_is() {
        let seventeen = format!("docs/{}", ["n"; 17].join("/"));
        let too_long = format!("docs/{}", "n".repeat(256));
        // Two bytes each: 128 of them are 256 bytes.
        let too_long_in_bytes = format!("docs/{}", "\u{e9}".repeat(128));
        let cases: Vec<(&str, Problem)> = vec![
            ("", Problem::NotUnderDocuments),
            ("docs", Problem::NotUnderDocuments),
            ("notes/a", Problem::NotUnderDocuments),
            ("Docs/a", Problem::NotUnderDocuments),
            ("DOCS/a", Problem::NotUnderDocuments),
            ("docsx/a", Problem::NotUnderDocuments),
            (".manyhands/tickets/a", Problem::NotUnderDocuments),
            ("./docs/a", Problem::NotUnderDocuments),
            ("../docs/a", Problem::NotUnderDocuments),
            ("docs\\a", Problem::NotUnderDocuments),
            ("/docs/a", Problem::Absolute),
            ("/", Problem::Absolute),
            ("\\docs\\a", Problem::Absolute),
            ("\\\\server\\share\\docs", Problem::Absolute),
            ("C:/docs/a", Problem::Absolute),
            ("c:docs/a", Problem::Absolute),
            ("C:\\docs\\a", Problem::Absolute),
            ("docs/", Problem::EmptySegment),
            ("docs//a", Problem::EmptySegment),
            ("docs/a/", Problem::EmptySegment),
            ("docs/a//b", Problem::EmptySegment),
            ("docs/..", Problem::Relative),
            ("docs/../a", Problem::Relative),
            ("docs/a/..", Problem::Relative),
            ("docs/a/../../../etc", Problem::Relative),
            ("docs/.", Problem::Relative),
            ("docs/a/./b", Problem::Relative),
            (&seventeen, Problem::TooDeep),
            ("docs/a\\b", Problem::Character),
            ("docs/a\\..\\..\\b", Problem::Character),
            ("docs/C:", Problem::Character),
            ("docs/a:stream", Problem::Character),
            ("docs/a::$DATA", Problem::Character),
            ("docs/a*", Problem::Character),
            ("docs/a?", Problem::Character),
            ("docs/\"a\"", Problem::Character),
            ("docs/<a>", Problem::Character),
            ("docs/a|b", Problem::Character),
            ("docs/a\0b", Problem::Character),
            ("docs/a\nb", Problem::Character),
            ("docs/a\tb", Problem::Character),
            ("docs/a\u{7f}", Problem::Character),
            ("docs/a\u{85}b", Problem::Character),
            ("docs/.g\u{200c}it", Problem::Character),
            ("docs/a\u{202e}b", Problem::Character),
            ("docs/\u{feff}a", Problem::Character),
            ("docs/a.", Problem::TrailingDotOrSpace),
            ("docs/a ", Problem::TrailingDotOrSpace),
            ("docs/a. .", Problem::TrailingDotOrSpace),
            ("docs/...", Problem::TrailingDotOrSpace),
            ("docs/.git.", Problem::TrailingDotOrSpace),
            ("docs/a /b", Problem::TrailingDotOrSpace),
            (&too_long, Problem::TooLong),
            (&too_long_in_bytes, Problem::TooLong),
            ("docs/.git", Problem::GitDirectory),
            ("docs/.GIT", Problem::GitDirectory),
            ("docs/a/.Git/b", Problem::GitDirectory),
            ("docs/GIT~1", Problem::GitDirectory),
            ("docs/CON", Problem::DeviceName),
            ("docs/con", Problem::DeviceName),
            ("docs/Nul", Problem::DeviceName),
            ("docs/nul.txt", Problem::DeviceName),
            ("docs/NUL.tar.gz", Problem::DeviceName),
            ("docs/aux", Problem::DeviceName),
            ("docs/PRN", Problem::DeviceName),
            ("docs/COM1", Problem::DeviceName),
            ("docs/com9.log", Problem::DeviceName),
            ("docs/COM0", Problem::DeviceName),
            ("docs/lpt1", Problem::DeviceName),
            ("docs/LPT\u{b9}", Problem::DeviceName),
            ("docs/com\u{b2}", Problem::DeviceName),
            ("docs/CONIN$", Problem::DeviceName),
            ("docs/conout$", Problem::DeviceName),
            ("docs/con .txt", Problem::DeviceName),
            ("docs/a/nul/b", Problem::DeviceName),
        ];
        for (path, expected) in cases {
            assert_eq!(problem(path), Some(expected), "{path:?}");
        }
    }
}
