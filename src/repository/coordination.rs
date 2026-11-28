use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use fs4::fs_std::FileExt;
use git2::Repository;

use super::{RepositoryError, RepositoryErrorKind, RepositoryOperation};

pub(super) const LEASE_TIMEOUT: Duration = Duration::from_millis(250);
const RETRY_INTERVAL: Duration = Duration::from_millis(5);

pub(super) struct RepositoryLease(File);
/// A setup operation's lock (design Part D, R-4): its own file beside the
/// repository lease's, held for the operation's whole run, the network
/// included.
pub(super) struct SetupLock(File);
pub(super) struct BootstrapLease(File);
pub(super) struct CacheReadGuard(File);
pub(super) struct CacheWriteGuard(File);

impl RepositoryLease {
    /// Reopening Git after taking the cache guard must still name this lease's
    /// file, not a replacement at the same pathname. Metadata reads do not read
    /// the locked bytes (important for Windows mandatory byte-range locks).
    pub(super) fn still_names(&self, repository: &Repository) -> io::Result<bool> {
        let current = match File::open(repository.commondir().join(REPOSITORY_LOCK_FILE)) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        Ok(file_identity(&self.0)? == file_identity(&current)?)
    }
}

/// The identity of an open file: its device and inode, or on Windows its
/// volume serial number and file ID. A file put in place of another at the
/// same path has another identity.
#[cfg(unix)]
pub(super) fn file_identity(file: &File) -> io::Result<(u64, u128)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok((metadata.dev(), u128::from(metadata.ino())))
}

/// The identity of the file at `path`, as `file_identity` gives it, read
/// with no descriptor of it opened: on Unix, closing any descriptor of a
/// file drops every POSIX lock this process holds on it, and SQLite's lock
/// on an open database is one (review C1 of Task 20b).
#[cfg(unix)]
pub(super) fn path_identity(path: &Path) -> io::Result<(u64, u128)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path)?;
    Ok((metadata.dev(), u128::from(metadata.ino())))
}

/// The identity of the file at `path`, from a handle of its own. A
/// Windows byte-range lock belongs to the handle that took it, so closing
/// this one leaves SQLite's alone.
#[cfg(windows)]
pub(super) fn path_identity(path: &Path) -> io::Result<(u64, u128)> {
    file_identity(&File::open(path)?)
}

/// Whether this process may open the file at `path` to read and write it,
/// asked with no descriptor of it opened on Unix (see `path_identity`): an
/// `access(2)` check, which a missing file fails.
#[cfg(unix)]
pub(super) fn may_read_and_write(path: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: `path` is a NUL-terminated string that outlives the call,
    // which only reads it.
    if unsafe { libc::access(path.as_ptr(), libc::R_OK | libc::W_OK) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Whether this process may open the file at `path` to read and write it:
/// it is opened so, without being created. Closing this handle leaves the
/// locks of SQLite's alone (see `path_identity`).
#[cfg(windows)]
pub(super) fn may_read_and_write(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map(|_| ())
}

#[cfg(windows)]
pub(super) fn file_identity(file: &File) -> io::Result<(u64, u128)> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ID_INFO, FileIdInfo, GetFileInformationByHandleEx,
    };
    let mut id: FILE_ID_INFO = unsafe { std::mem::zeroed() };
    // SAFETY: the live file owns the handle; the buffer is sized and aligned
    // for FILE_ID_INFO and remains valid until the synchronous call returns.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok((
        id.VolumeSerialNumber,
        u128::from_le_bytes(id.FileId.Identifier),
    ))
}

macro_rules! unlock_on_drop {
    ($guard:ident) => {
        impl Drop for $guard {
            fn drop(&mut self) {
                let _ = FileExt::unlock(&self.0);
            }
        }
    };
}

unlock_on_drop!(RepositoryLease);
unlock_on_drop!(SetupLock);
unlock_on_drop!(BootstrapLease);
unlock_on_drop!(CacheReadGuard);
unlock_on_drop!(CacheWriteGuard);

pub(super) fn repository_lease(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<RepositoryLease, RepositoryError> {
    repository_lease_waiting(repository, root, operation, LEASE_TIMEOUT)
}

/// `repository_lease`, waiting `wait` for a holder instead of the usual
/// timeout, so that a test that needs the busy answer spends no real time.
pub(super) fn repository_lease_waiting(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
    wait: Duration,
) -> Result<RepositoryLease, RepositoryError> {
    acquire(
        repository.commondir().join(REPOSITORY_LOCK_FILE),
        root,
        operation,
        FileExt::try_lock_exclusive,
        wait,
    )
    .map(RepositoryLease)
}

/// The setup lock of the repository `repository`, with the same timeout as
/// the repository lease: a second setup operation meeting it is `busy`. It
/// is taken only by whoever has read a setup record first, so a repository
/// with no record never gets the file.
pub(super) fn setup_lock(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<SetupLock, RepositoryError> {
    setup_lock_waiting(repository, root, operation, LEASE_TIMEOUT)
}

/// `setup_lock`, waiting `wait` for a holder instead of the usual timeout.
pub(super) fn setup_lock_waiting(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
    wait: Duration,
) -> Result<SetupLock, RepositoryError> {
    acquire(
        repository.commondir().join(SETUP_LOCK_FILE),
        root,
        operation,
        FileExt::try_lock_exclusive,
        wait,
    )
    .map(SetupLock)
}

pub(super) fn bootstrap_lease(
    data_directory: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<BootstrapLease, RepositoryError> {
    acquire_exclusive(
        data_directory.join(bootstrap_lease_name(root)),
        root,
        operation,
    )
    .map(BootstrapLease)
}

pub(super) fn cache_read_guard(
    registry_path: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<CacheReadGuard, RepositoryError> {
    acquire_shared(cache_lock_path(registry_path), root, operation).map(CacheReadGuard)
}

pub(super) fn cache_write_guard(
    registry_path: &Path,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<CacheWriteGuard, RepositoryError> {
    acquire_exclusive(cache_lock_path(registry_path), root, operation).map(CacheWriteGuard)
}

/// Whether no call holds the repository's lease or the bootstrap lease
/// of its root at this instant. Neither is waited for, and each is let go
/// at once: this is a look, for a read that must not offer to end an
/// operation a call is at work under. A lease whose file is not there has
/// no holder, and looking never makes the file. A lease that cannot be
/// looked at for any other reason is taken to be held.
pub(super) fn leases_are_free(data_directory: &Path, root: &Path) -> bool {
    let free = |path: std::path::PathBuf| -> bool {
        let file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            // No file: no call has ever taken this lease, or its root
            // is gone. None is made here.
            Err(error) if error.kind() == io::ErrorKind::NotFound => return true,
            Err(_) => return false,
        };
        match FileExt::try_lock_exclusive(&file) {
            Ok(true) => {
                let _ = FileExt::unlock(&file);
                true
            }
            Ok(false) | Err(_) => false,
        }
    };
    if !free(data_directory.join(bootstrap_lease_name(root))) {
        return false;
    }
    match Repository::open_ext(
        root,
        git2::RepositoryOpenFlags::NO_SEARCH,
        &[] as &[&std::ffi::OsStr],
    ) {
        Ok(repository) => free(repository.commondir().join(REPOSITORY_LOCK_FILE)),
        // No repository is there to have a lease.
        Err(_) => true,
    }
}

/// The file the repository lease is taken on, in the repository's common
/// directory.
const REPOSITORY_LOCK_FILE: &str = "manyhands-operation.lock";

/// The file the setup lock is taken on, in the repository's common
/// directory.
const SETUP_LOCK_FILE: &str = "manyhands-setup.lock";

/// Whether this process holds the read lock SQLite's unix VFS keeps on a
/// database's shared byte range (`SHARED_FIRST`, 510 bytes from offset
/// 2^30 + 2) while a connection of it has the database open in WAL mode,
/// as `/proc/locks` lists the file's device and inode.
#[cfg(all(test, target_os = "linux"))]
pub(super) fn sqlite_shared_lock_held(database: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    const SHARED_FIRST: u64 = 0x4000_0000 + 2;
    let metadata = std::fs::metadata(database).unwrap();
    let dev = metadata.dev();
    // The kernel's MAJOR:MINOR of a userspace device number, in hex.
    let major = ((dev >> 8) & 0xfff) | ((dev >> 32) & !0xfff);
    let minor = (dev & 0xff) | ((dev >> 12) & !0xff);
    let file = format!("{major:02x}:{minor:02x}:{}", metadata.ino());
    let pid = std::process::id().to_string();
    std::fs::read_to_string("/proc/locks")
        .unwrap()
        .lines()
        .filter(|line| !line.contains("->"))
        .any(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            matches!(
                fields.as_slice(),
                [_, "POSIX", _, "READ", owner, held, start, end]
                    if *owner == pid
                        && *held == file
                        && start.parse::<u64>().is_ok_and(|start| start <= SHARED_FIRST)
                        && (*end == "EOF"
                            || end.parse::<u64>().is_ok_and(|end| end >= SHARED_FIRST))
            )
        })
}

/// Full-content snapshot for repository fixtures, including the empty lease file.
#[cfg(test)]
pub(super) fn snapshot_file_hash(root: &Path, path: &Path) -> [u8; 32] {
    if path == root.join(".git").join(REPOSITORY_LOCK_FILE)
        || path == root.join(".git").join(SETUP_LOCK_FILE)
    {
        // Windows mandatory byte locks forbid a second handle's read even in
        // this process. This exact coordination file is never written to. Prove
        // its entire content is empty using metadata instead; keep its entry in
        // the snapshot so removal is detected too. Never exempt a namesake file.
        let metadata = std::fs::symlink_metadata(path).unwrap();
        assert!(metadata.is_file(), "a lock file must be a regular file");
        assert_eq!(metadata.len(), 0, "a lock file must remain empty");
        return *blake3::hash(b"").as_bytes();
    }
    *blake3::hash(&std::fs::read(path).unwrap()).as_bytes()
}

fn bootstrap_lease_name(root: &Path) -> String {
    format!(
        "manyhands-bootstrap-{}.lock",
        blake3::hash(root.as_os_str().as_encoded_bytes())
    )
}

fn cache_lock_path(registry_path: &Path) -> std::path::PathBuf {
    registry_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("manyhands.sqlite3.recovery.lock")
}

fn acquire_exclusive(
    path: std::path::PathBuf,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<File, RepositoryError> {
    acquire(
        path,
        root,
        operation,
        FileExt::try_lock_exclusive,
        LEASE_TIMEOUT,
    )
}

fn acquire_shared(
    path: std::path::PathBuf,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<File, RepositoryError> {
    acquire(
        path,
        root,
        operation,
        FileExt::try_lock_shared,
        LEASE_TIMEOUT,
    )
}

fn acquire(
    path: std::path::PathBuf,
    root: &Path,
    operation: RepositoryOperation,
    lock: impl Fn(&File) -> io::Result<bool>,
    wait: Duration,
) -> Result<File, RepositoryError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?;
    let deadline = Instant::now() + wait;
    loop {
        match lock(&file) {
            Ok(true) => return Ok(file),
            Ok(false) if Instant::now() < deadline => {
                thread::sleep(RETRY_INTERVAL);
            }
            Ok(false) => {
                return Err(RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::RepositoryBusy,
                    "another repository operation is in progress",
                ));
            }
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(RETRY_INTERVAL);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Err(RepositoryError::new(
                    operation,
                    Some(root.to_owned()),
                    RepositoryErrorKind::RepositoryBusy,
                    "another repository operation is in progress",
                ));
            }
            Err(error) => return Err(RepositoryError::io(operation, Some(root.to_owned()), error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_lease_snapshot_checks_all_protected_files_and_empty_lease() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let root = directory.path().canonicalize().unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = Repository::init(&root).unwrap();
        let git_lease_path = repo.commondir().join(REPOSITORY_LOCK_FILE);
        // Snapshot callers walk from their own root spelling. On Windows,
        // canonicalize uses a verbatim prefix that libgit2's exported paths
        // strip. Prove filesystem identity separately; do not feed libgit2's
        // spelling to a helper whose contract is deliberately lexical.
        let lease_path = root.join(".git").join(REPOSITORY_LOCK_FILE);
        let lease = repository_lease(&repo, &root, RepositoryOperation::Read).unwrap();
        assert_eq!(
            git_lease_path.canonicalize().unwrap(),
            lease_path.canonicalize().unwrap()
        );
        assert!(!leases_are_free(data.path(), &root));
        assert_eq!(
            snapshot_file_hash(&root, &lease_path),
            *blake3::hash(b"").as_bytes()
        );
        assert!(!leases_are_free(data.path(), &root));
        for relative in [
            ".git/index",
            ".git/refs/heads/protected",
            ".git/config",
            "manyhands.sqlite3",
            "ordinary",
            "nested/manyhands-operation.lock",
        ] {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"before").unwrap();
            let before = snapshot_file_hash(&root, &path);
            std::fs::write(&path, b"after").unwrap();
            assert_ne!(snapshot_file_hash(&root, &path), before, "{relative}");
        }
        drop(lease);
        std::fs::write(&lease_path, b"unexpected protected contents").unwrap();
        assert!(std::panic::catch_unwind(|| snapshot_file_hash(&root, &lease_path)).is_err());
        std::fs::remove_file(&lease_path).unwrap();
        assert!(std::panic::catch_unwind(|| snapshot_file_hash(&root, &lease_path)).is_err());
    }

    #[test]
    fn the_setup_lock_is_exclusive_beside_the_lease_and_exempt_from_snapshots() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let repo = Repository::init(&root).unwrap();
        let operation = RepositoryOperation::Enable;
        let lock_path = root.join(".git").join(SETUP_LOCK_FILE);
        assert!(
            !lock_path.exists(),
            "nothing makes the file before it is asked for"
        );

        let held = setup_lock(&repo, &root, operation).unwrap();
        assert!(lock_path.is_file());
        // A second setup operation meets it as busy, even in this process.
        let busy = setup_lock_waiting(&repo, &root, operation, Duration::ZERO)
            .err()
            .unwrap();
        assert_eq!(busy.kind, RepositoryErrorKind::RepositoryBusy);
        // It is not the repository lease: item writers take that one, and
        // neither blocks the other.
        let lease = repository_lease(&repo, &root, operation).unwrap();
        drop(lease);
        drop(held);
        let held = setup_lock(&repo, &root, operation).unwrap();
        let lease = repository_lease(&repo, &root, operation).unwrap();
        assert_eq!(
            setup_lock_waiting(&repo, &root, operation, Duration::ZERO)
                .err()
                .unwrap()
                .kind,
            RepositoryErrorKind::RepositoryBusy
        );
        drop((held, lease));
        // A released lock is taken again.
        drop(setup_lock(&repo, &root, operation).unwrap());

        // A snapshot proves it empty from its metadata, as it does the
        // lease, and refuses it with content in it.
        assert_eq!(
            snapshot_file_hash(&root, &lock_path),
            *blake3::hash(b"").as_bytes()
        );
        std::fs::write(&lock_path, b"unexpected protected contents").unwrap();
        assert!(std::panic::catch_unwind(|| snapshot_file_hash(&root, &lock_path)).is_err());
        // A namesake elsewhere is read as any file is.
        let namesake = root.join("nested").join(SETUP_LOCK_FILE);
        std::fs::create_dir_all(namesake.parent().unwrap()).unwrap();
        std::fs::write(&namesake, b"before").unwrap();
        let before = snapshot_file_hash(&root, &namesake);
        std::fs::write(&namesake, b"after").unwrap();
        assert_ne!(snapshot_file_hash(&root, &namesake), before);
    }

    fn files(directory: &Path) -> Vec<std::ffi::OsString> {
        let mut names: Vec<_> = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_look_at_the_leases_makes_no_file_and_sees_who_holds_one() {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let root_path = root.path().canonicalize().unwrap();
        let operation = RepositoryOperation::Inspect;

        // No repository and no lock file: free, and nothing is made.
        assert!(leases_are_free(data.path(), &root_path));
        assert!(files(data.path()).is_empty());

        let repository = Repository::init(&root_path).unwrap();
        let git_before = files(repository.path());
        assert!(leases_are_free(data.path(), &root_path));
        assert_eq!(files(repository.path()), git_before, "no lock file is made");
        assert!(files(data.path()).is_empty());

        // The repository lease, held and then let go.
        let held = repository_lease(&repository, &root_path, operation).unwrap();
        assert!(repository.path().join(REPOSITORY_LOCK_FILE).exists());
        assert!(!leases_are_free(data.path(), &root_path));
        drop(held);
        assert!(leases_are_free(data.path(), &root_path));
        // The look let the lease go: it can be taken at once.
        drop(repository_lease(&repository, &root_path, operation).unwrap());

        // The bootstrap lease, held and then let go.
        let held = bootstrap_lease(data.path(), &root_path, operation).unwrap();
        assert!(!leases_are_free(data.path(), &root_path));
        // Another root's bootstrap lease is not this one's.
        assert!(leases_are_free(data.path(), &root_path.join("elsewhere")));
        drop(held);
        assert!(leases_are_free(data.path(), &root_path));
        drop(bootstrap_lease(data.path(), &root_path, operation).unwrap());
    }
}
