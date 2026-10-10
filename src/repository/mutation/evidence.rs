//! What a request left in Git: the commit it made, and whether a commit a
//! finished request reported can still be reached.
//!
//! The position recorded at acceptance only bounds where this looks: the
//! commits reachable from the request's branch and not from the recorded
//! commit, or the whole branch if nothing was recorded or the recorded
//! commit is not an ancestor of the branch. Only the request's own path is
//! compared, so unrelated commits on the same branch do not enter into it.
//!
//! A failure to read Git is its own answer, never "no commit": a caller
//! that took it for one would report a save that committed as a no-op.

use std::path::Path;

use git2::{Oid, Repository};

/// The branch a request commits to and where it stood at acceptance. For
/// an item with no editing context yet the branch does not exist, and the
/// commit is primary's head, which the context will be created from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Position {
    /// The full reference name, `refs/heads/...`.
    pub(crate) base_ref: Option<String>,
    pub(crate) base_oid: Option<Oid>,
}

impl Position {
    /// The position of a request that will commit to `base_ref` in the
    /// repository at `root`.
    pub(crate) fn of(root: &Path, base_ref: &str) -> Result<Self, git2::Error> {
        let repository = Repository::open(root)?;
        let base_oid = match repository.refname_to_id(base_ref) {
            Ok(tip) => tip,
            Err(error) if error.code() == git2::ErrorCode::NotFound => {
                repository.head()?.peel_to_commit()?.id()
            }
            Err(error) => return Err(error),
        };
        Ok(Self {
            base_ref: Some(base_ref.to_owned()),
            base_oid: Some(base_oid),
        })
    }

    /// The branch's short name, as a result's scope gives it.
    pub(crate) fn branch(&self) -> Option<&str> {
        self.base_ref
            .as_deref()
            .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name))
    }
}

/// The commits in range, newest first.
fn range(repository: &Repository, position: &Position) -> Result<Vec<Oid>, git2::Error> {
    let Some(base_ref) = position.base_ref.as_deref() else {
        return Ok(Vec::new());
    };
    let tip = match repository.refname_to_id(base_ref) {
        Ok(tip) => tip,
        // The branch was never created: nothing was committed to it.
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut walk = repository.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    walk.push(tip)?;
    if let Some(base) = position.base_oid
        && (base == tip || repository.graph_descendant_of(tip, base).unwrap_or(false))
    {
        walk.hide(base)?;
    }
    walk.collect()
}

/// The blob at `path` in `commit`, if a file is there.
fn blob_at(
    repository: &Repository,
    commit: &git2::Commit<'_>,
    path: &Path,
) -> Result<Option<Oid>, git2::Error> {
    match commit.tree()?.get_path(path) {
        Ok(entry) => Ok(Some(entry.id())),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
    .map(|blob| blob.filter(|blob| repository.find_blob(*blob).is_ok()))
}

/// Whether `commit` left `path` other than its first parent had it.
fn changed_path(repository: &Repository, commit: Oid, path: &Path) -> Result<bool, git2::Error> {
    let commit = repository.find_commit(commit)?;
    let after = blob_at(repository, &commit, path)?;
    let before = match commit.parent(0) {
        Ok(parent) => blob_at(repository, &parent, path)?,
        Err(error) if error.code() == git2::ErrorCode::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok(before != after)
}

/// Git could not be read, so the check has no answer. This is not "not
/// found": a caller must not take it for the absence of a commit. The Git
/// error is dropped here; its text can hold a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Unreadable;

/// What the check says of a commit a domain call named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Claim {
    /// It is in range and it changed the request's path.
    Confirmed,
    /// It is not in range, or did not change the path: a save that
    /// changed nothing can name the head it found.
    NotThisRequests,
    /// Git could not be read. The commit may well be the request's.
    Unreadable,
}

/// Whether `claimed`, the commit a domain call named, is this request's:
/// it is in range and it changed `path`.
pub(crate) fn confirms(root: &Path, position: &Position, claimed: Oid, path: &str) -> Claim {
    let confirmed = || -> Result<bool, git2::Error> {
        let repository = Repository::open(root)?;
        Ok(range(&repository, position)?.contains(&claimed)
            && changed_path(&repository, claimed, Path::new(path))?)
    };
    match confirmed() {
        Ok(true) => Claim::Confirmed,
        Ok(false) => Claim::NotThisRequests,
        Err(_) => Claim::Unreadable,
    }
}

/// The newest commit in range that changed `path` and left it as the
/// request intended, which `intended` decides from the file's bytes.
///
/// Only the newest commit that changed the path is looked at: when a
/// later one left the path as something else, the request's own commit,
/// if there was one, is no longer what the branch holds.
pub(crate) fn intended_commit(
    root: &Path,
    position: &Position,
    path: &str,
    intended: &dyn Fn(&[u8]) -> bool,
) -> Result<Option<Oid>, Unreadable> {
    let found = || -> Result<Option<Oid>, git2::Error> {
        let repository = Repository::open(root)?;
        let path = Path::new(path);
        for commit in range(&repository, position)? {
            if !changed_path(&repository, commit, path)? {
                continue;
            }
            let blob = blob_at(&repository, &repository.find_commit(commit)?, path)?;
            let Some(blob) = blob else {
                return Ok(None);
            };
            let blob = repository.find_blob(blob)?;
            return Ok(intended(blob.content()).then_some(commit));
        }
        Ok(None)
    };
    found().map_err(|_| Unreadable)
}

/// The newest commit in range that changed `path`, whatever it left
/// there. This is what can be said of a request whose input is not at
/// hand: that a commit was made to its path since it was accepted.
pub(crate) fn path_commit(
    root: &Path,
    position: &Position,
    path: &str,
) -> Result<Option<Oid>, Unreadable> {
    let found = || -> Result<Option<Oid>, git2::Error> {
        let repository = Repository::open(root)?;
        for commit in range(&repository, position)? {
            if changed_path(&repository, commit, Path::new(path))? {
                return Ok(Some(commit));
            }
        }
        Ok(None)
    };
    found().map_err(|_| Unreadable)
}

/// Whether `commit` can still be reached from the branch recorded in
/// `base_ref`, or from primary, the branch checked out at `root`, once
/// that branch is gone.
pub(crate) fn still_reachable(root: &Path, base_ref: Option<&str>, commit: Oid) -> bool {
    let reachable = || -> Result<bool, git2::Error> {
        let repository = Repository::open(root)?;
        repository.find_commit(commit)?;
        let branch = match base_ref.map(|name| repository.refname_to_id(name)) {
            Some(Ok(tip)) => Some(tip),
            Some(Err(error)) if error.code() == git2::ErrorCode::NotFound => None,
            Some(Err(error)) => return Err(error),
            None => None,
        };
        let tip = match branch {
            Some(tip) => tip,
            None => repository.head()?.peel_to_commit()?.id(),
        };
        Ok(tip == commit || repository.graph_descendant_of(tip, commit)?)
    };
    reachable().unwrap_or(false)
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
