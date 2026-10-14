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

const LEASE_TIMEOUT: Duration = Duration::from_millis(250);
const RETRY_INTERVAL: Duration = Duration::from_millis(5);

pub(super) struct RepositoryLease(File);
pub(super) struct BootstrapLease(File);
pub(super) struct CacheReadGuard(File);
pub(super) struct CacheWriteGuard(File);

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
unlock_on_drop!(BootstrapLease);
unlock_on_drop!(CacheReadGuard);
unlock_on_drop!(CacheWriteGuard);

pub(super) fn repository_lease(
    repository: &Repository,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<RepositoryLease, RepositoryError> {
    acquire_exclusive(
        repository.commondir().join(REPOSITORY_LOCK_FILE),
        root,
        operation,
    )
    .map(RepositoryLease)
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
    acquire(path, root, operation, FileExt::try_lock_exclusive)
}

fn acquire_shared(
    path: std::path::PathBuf,
    root: &Path,
    operation: RepositoryOperation,
) -> Result<File, RepositoryError> {
    acquire(path, root, operation, FileExt::try_lock_shared)
}

fn acquire(
    path: std::path::PathBuf,
    root: &Path,
    operation: RepositoryOperation,
    lock: impl Fn(&File) -> io::Result<bool>,
) -> Result<File, RepositoryError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| RepositoryError::io(operation, Some(root.to_owned()), error))?;
    let deadline = Instant::now() + LEASE_TIMEOUT;
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
