//! Target resolution: from a path the caller selected to one registration.
//!
//! A read resolves its repository once and passes the result on, so two
//! reads of one request cannot disagree about the repository they describe.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use git2::{Repository, RepositoryOpenFlags};
use rusqlite::OptionalExtension;

use super::{REBUILD_INDEX_ACTION, ReadError, root_action};
use crate::{
    repository::{RepositoryOperation, RepositoryService},
    results::{ResultCode, Scope, absolute_path_string},
};

/// The recovery action for a path inside a repository: inspect the root it
/// lies in, which is given as the action's argument.
const INSPECT_ROOT_ACTION: &str = "repo.inspect";

/// A registered repository: the target every repository-scoped read takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedRepository {
    registration_id: i64,
    root: PathBuf,
}

impl ResolvedRepository {
    /// The canonical working-directory root, as it is registered.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The scope of a result about this repository.
    pub fn scope(&self) -> Scope {
        root_scope(&self.root)
    }

    pub(super) fn registration_id(&self) -> i64 {
        self.registration_id
    }

    /// Opens the registered root. A root that can no longer be opened is
    /// `repository_inaccessible`.
    pub(super) fn open(&self) -> Result<Repository, ReadError> {
        open_exactly(&self.root).map_err(|error| {
            self.failure(ReadError::new(ResultCode::RepositoryInaccessible).with_source(error))
        })
    }

    /// Places a failure of a read of this repository: the scope names the
    /// repository, and a rebuild is told which root to rebuild from. Every
    /// read that takes a resolved repository returns its failures this way.
    pub(super) fn failure(&self, error: impl Into<ReadError>) -> ReadError {
        at_root(error.into(), &self.root)
    }
}

fn root_scope(root: &Path) -> Scope {
    Scope {
        repository: absolute_path_string(root),
        ..Scope::default()
    }
}

/// `error` as a failure about the repository at `root`. A scope the error
/// already names is kept.
pub(super) fn at_root(mut error: ReadError, root: &Path) -> ReadError {
    let root = absolute_path_string(root);
    if error.code() == ResultCode::IndexUnavailable {
        error.recovery = vec![root_action(REBUILD_INDEX_ACTION, root.as_deref())];
    }
    if error.scope.repository.is_none() {
        error.scope.repository = root;
    }
    error
}

/// Opens the repository at exactly `path`, never one above it.
fn open_exactly(path: &Path) -> Result<Repository, git2::Error> {
    Repository::open_ext(path, RepositoryOpenFlags::NO_SEARCH, &[] as &[&OsStr])
}

/// A failure to open or find a repository. Only "not found" means there is
/// none; a repository Git refuses to open is there and cannot be read.
fn git_failure(error: git2::Error) -> ReadError {
    let code = match error.code() {
        git2::ErrorCode::NotFound => ResultCode::NotRepository,
        _ => ResultCode::RepositoryInaccessible,
    };
    ReadError::new(code).with_source(error)
}

/// A failure to canonicalize the selected path. A path that names nothing
/// is the caller's mistake; one that cannot be reached is not.
fn selected_path_failure(error: io::Error) -> ReadError {
    let code = match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory | io::ErrorKind::InvalidInput => {
            ResultCode::InvalidPath
        }
        _ => ResultCode::RepositoryInaccessible,
    };
    ReadError::new(code).with_source(error)
}

/// A directory Git names that cannot be reached.
fn inaccessible(error: io::Error) -> ReadError {
    ReadError::new(ResultCode::RepositoryInaccessible).with_source(error)
}

/// The canonical working directory of `repository`, or `bare_repository`.
fn working_directory(repository: &Repository) -> Result<PathBuf, ReadError> {
    let Some(directory) = repository.workdir().filter(|_| !repository.is_bare()) else {
        let mut error = ReadError::new(ResultCode::BareRepository);
        error.scope.repository = fs::canonicalize(repository.path())
            .ok()
            .and_then(|path| absolute_path_string(&path));
        return Err(error);
    };
    fs::canonicalize(directory).map_err(inaccessible)
}

/// The root of the repository that owns the linked worktree `worktree`.
///
/// The common Git directory names a working directory, but not reliably:
/// without `core.worktree` Git takes it to be the directory's parent, which
/// may be nothing, or another repository. So the root is accepted only when
/// the repository opened there has the same common Git directory.
fn owner_root(worktree: &Repository) -> Result<PathBuf, ReadError> {
    let common_directory = fs::canonicalize(worktree.commondir()).map_err(inaccessible)?;
    // The worktree exists and names this directory, so failing to open it
    // is a repository that cannot be read, never the absence of one.
    let common = open_exactly(&common_directory)
        .map_err(|error| ReadError::new(ResultCode::RepositoryInaccessible).with_source(error))?;
    let root = working_directory(&common)?;
    let owner = open_exactly(&root).map_err(git_failure)?;
    if owner.is_worktree()
        || fs::canonicalize(owner.commondir()).map_err(inaccessible)? != common_directory
    {
        return Err(ReadError::new(ResultCode::NotRepository));
    }
    Ok(root)
}

/// What a selected path names: itself, canonical, and the root of the
/// repository it selects.
pub(super) struct SelectedRepository {
    pub(super) selected: PathBuf,
    pub(super) root: PathBuf,
}

/// The repository `path` selects, registered or not: the first four steps
/// of resolution.
///
/// `path` must be a working-directory root. A linked worktree root selects
/// the repository that owns it, through their common Git directory.
pub(super) fn selected_repository(path: &Path) -> Result<SelectedRepository, ReadError> {
    let selected = fs::canonicalize(path).map_err(selected_path_failure)?;
    repository_root(&selected)
        .map(|root| SelectedRepository {
            root,
            selected: selected.clone(),
        })
        // What cannot be read is, so far, only the path that was selected.
        .map_err(|error| match error.code() {
            ResultCode::RepositoryInaccessible => at_root(error, &selected),
            _ => error,
        })
}

fn repository_root(selected: &Path) -> Result<PathBuf, ReadError> {
    let repository = match open_exactly(selected) {
        Ok(repository) => repository,
        // Whatever is found above is not at the selected path, so the
        // comparison below reports it with its root.
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            Repository::discover(selected).map_err(git_failure)?
        }
        Err(error) => return Err(git_failure(error)),
    };
    let directory = working_directory(&repository)?;
    if directory != selected {
        let root = absolute_path_string(&directory);
        return Err(ReadError::new(ResultCode::NotRepositoryRoot)
            .with_recovery(vec![root_action(INSPECT_ROOT_ACTION, root.as_deref())])
            .with_scope(root_scope(&directory)));
    }
    if repository.is_worktree() {
        // No owner is known, so the failure is about the worktree itself.
        owner_root(&repository).map_err(|error| at_root(error, selected))
    } else {
        Ok(directory)
    }
}

impl RepositoryService {
    /// Resolves the repository a caller selected by path.
    ///
    /// The path must be the root of a working directory. A linked worktree
    /// root, including an item worktree under `.manyhands/worktrees`,
    /// resolves to the repository that owns it. Nothing above the path is
    /// searched for: a path inside a repository is `not_repository_root`,
    /// and its recovery names the root.
    pub fn resolve_repository(&self, path: &Path) -> Result<ResolvedRepository, ReadError> {
        let root = selected_repository(path)?.root;
        match self.registration_id(&root) {
            Ok(Some(registration_id)) => Ok(ResolvedRepository {
                registration_id,
                root,
            }),
            Ok(None) => Err(at_root(
                ReadError::new(ResultCode::RepositoryNotRegistered),
                &root,
            )),
            Err(error) => Err(at_root(error, &root)),
        }
    }

    /// The registration whose stored root is exactly `root`, the canonical
    /// string stored at enablement.
    pub(super) fn registration_id(&self, root: &Path) -> Result<Option<i64>, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            // A root that is not UTF-8 cannot have been stored.
            let Some(root_path) = root.to_str() else {
                return Ok(None);
            };
            Ok(connection
                .query_row(
                    "SELECT id FROM repositories WHERE root_path = ?1",
                    [root_path],
                    |row| row.get(0),
                )
                .optional()?)
        })
    }
}
