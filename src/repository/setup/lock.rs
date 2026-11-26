//! Beginning a setup operation: the record is read, the setup lock is
//! taken, and the record is read again under it (design Part D, R-4).

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use git2::Repository;
use rusqlite::OptionalExtension;

use super::{
    publish::SetupError,
    record::{self, SetupKind, SetupRecord},
};
use crate::repository::{
    RepositoryError, RepositoryOperation, RepositoryService,
    coordination::{SetupLock, setup_lock, setup_lock_waiting},
};

/// What a setup operation holds from its second read of the record to its
/// end: the lock, the repository, and the record as the lock found it.
pub(super) struct Held {
    pub(super) lock: SetupLock,
    pub(super) repository: Repository,
    pub(super) repository_id: i64,
    pub(super) record: SetupRecord,
}

/// Where a beginning stands.
pub(super) enum Begun {
    /// The lock is held and the record is there.
    Held(Box<Held>),
    /// There is no setup: it ended elsewhere, and its request was finished
    /// in the transaction that ended it.
    Gone,
}

/// The operation a setup of `kind` reports its errors under.
pub(super) const fn operation_of(kind: SetupKind) -> RepositoryOperation {
    match kind {
        SetupKind::Enable => RepositoryOperation::Enable,
        SetupKind::Change => RepositoryOperation::SetPublicationRemote,
    }
}

/// The canonical root, as the registry holds it.
pub(super) fn canonical_root(root: &Path) -> Result<PathBuf, SetupError> {
    std::fs::canonicalize(root).map_err(|error| {
        SetupError::Repository(RepositoryError::io(
            RepositoryOperation::Read,
            Some(root.to_owned()),
            error,
        ))
    })
}

/// The setup of the repository registered at `root` and the registration's
/// ID, if it has one.
pub(super) fn read_record(
    service: &RepositoryService,
    root: &Path,
) -> Result<Option<(i64, SetupRecord)>, SetupError> {
    let key = root.to_str().map(str::to_owned).ok_or_else(|| {
        SetupError::Repository(RepositoryError::new(
            RepositoryOperation::Read,
            Some(root.to_owned()),
            crate::repository::RepositoryErrorKind::InvalidPath,
            "the repository root is not text",
        ))
    })?;
    service
        .read_session(RepositoryOperation::Read, |connection| {
            let id: Option<i64> = connection
                .query_row(
                    "SELECT id FROM repositories WHERE root_path = ?1",
                    [key],
                    |row| row.get(0),
                )
                .optional()?;
            match id {
                None => Ok(None),
                Some(id) => Ok(record::read(connection, id)?.map(|record| (id, record))),
            }
        })
        .map_err(SetupError::Registry)
}

/// Reads the record, takes the lock, reads the record again. A repository
/// with no record never creates the lock file. `wait` is how long a held
/// lock is waited for: `None` is the usual timeout. `between` runs after the
/// first read and before the lock, for a test to end the setup there.
pub(super) fn begin_with(
    service: &RepositoryService,
    root: &Path,
    wait: Option<Duration>,
    between: impl FnOnce(),
) -> Result<Begun, SetupError> {
    let root = canonical_root(root)?;
    let Some((_, first)) = read_record(service, &root)? else {
        return Ok(Begun::Gone);
    };
    between();
    let operation = operation_of(first.kind);
    let repository = Repository::open(&root)
        .map_err(|error| RepositoryError::git(operation, Some(root.clone()), error))?;
    let lock = match wait {
        None => setup_lock(&repository, &root, operation)?,
        Some(wait) => setup_lock_waiting(&repository, &root, operation, wait)?,
    };
    let Some((repository_id, record)) = read_record(service, &root)? else {
        return Ok(Begun::Gone);
    };
    Ok(Begun::Held(Box::new(Held {
        lock,
        repository,
        repository_id,
        record,
    })))
}
