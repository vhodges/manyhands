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
        repository.commondir().join("manyhands-operation.lock"),
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
    let name = format!(
        "manyhands-bootstrap-{}.lock",
        blake3::hash(root.as_os_str().as_encoded_bytes())
    );
    acquire_exclusive(data_directory.join(name), root, operation).map(BootstrapLease)
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
