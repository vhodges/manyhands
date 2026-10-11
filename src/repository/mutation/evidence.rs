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

use super::super::ExpectedPathObservation;

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

/// The commits a request's evidence is looked for among.
struct Range {
    /// Newest first.
    commits: Vec<Oid>,
    /// The branch's tip, when the branch exists.
    tip: Option<Oid>,
    /// The recorded commit, when it bounds the range: it is the tip or an
    /// ancestor of it. Without it the range is the whole branch, which
    /// holds history older than the request.
    base: Option<Oid>,
}

fn range_of(repository: &Repository, position: &Position) -> Result<Range, git2::Error> {
    let nothing = Range {
        commits: Vec::new(),
        tip: None,
        base: None,
    };
    let Some(base_ref) = position.base_ref.as_deref() else {
        return Ok(nothing);
    };
    let tip = match repository.refname_to_id(base_ref) {
        Ok(tip) => tip,
        // The branch was never created: nothing was committed to it.
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(nothing),
        Err(error) => return Err(error),
    };
    let mut walk = repository.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    walk.push(tip)?;
    let mut bound = None;
    if let Some(base) = position.base_oid
        && (base == tip || is_ancestor(repository, base, tip)?)
    {
        walk.hide(base)?;
        bound = Some(base);
    }
    Ok(Range {
        commits: walk.collect::<Result<_, _>>()?,
        tip: Some(tip),
        base: bound,
    })
}

/// The commits in range, newest first.
fn range(repository: &Repository, position: &Position) -> Result<Vec<Oid>, git2::Error> {
    range_of(repository, position).map(|range| range.commits)
}

/// Whether the recorded commit `base` is an ancestor of `tip`.
///
/// A commit the repository does not hold is not an ancestor of anything in
/// it. Any other failure to read is an error and never "no": that answer
/// widens the range to the whole branch, where a commit made before the
/// request was accepted could be taken for the request's.
fn is_ancestor(repository: &Repository, base: Oid, tip: Oid) -> Result<bool, git2::Error> {
    match repository.find_commit(base) {
        Ok(_) => repository.graph_descendant_of(tip, base),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// The blob at `path` in `commit`, if a file is there. Whether one is
/// there is read from the tree: the blob itself is not opened, so a blob
/// that cannot be read is still a file, and reading it fails where its
/// content is asked for.
fn blob_at(commit: &git2::Commit<'_>, path: &Path) -> Result<Option<Oid>, git2::Error> {
    match commit.tree()?.get_path(path) {
        Ok(entry) => Ok((entry.kind() == Some(git2::ObjectType::Blob)).then(|| entry.id())),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Whether `commit` left `path` other than its first parent had it.
fn changed_path(repository: &Repository, commit: Oid, path: &Path) -> Result<bool, git2::Error> {
    let commit = repository.find_commit(commit)?;
    let after = blob_at(&commit, path)?;
    // A commit with no parent is a root. A parent that is named and
    // cannot be found is a failure to read, not a root.
    let before = if commit.parent_count() == 0 {
        None
    } else {
        blob_at(&commit.parent(0)?, path)?
    };
    Ok(before != after)
}

/// What is at `path` in `commit`, as a request's expectation states it:
/// the digest of the file's bytes, or that no file is there.
fn observed_at(
    repository: &Repository,
    commit: &git2::Commit<'_>,
    path: &Path,
) -> Result<ExpectedPathObservation, git2::Error> {
    Ok(match blob_at(commit, path)? {
        Some(blob) => ExpectedPathObservation::from_bytes(repository.find_blob(blob)?.content()),
        None => ExpectedPathObservation::Missing,
    })
}

/// What Git shows of a request's path since the request was accepted: the
/// commits in range, and of those the ones that changed the path, each
/// with whether it left the path as the request intended and whether it
/// changed it from the state the request was accepted against.
///
/// Re-entry reads this before and after its domain call. It is never a
/// guess: a range, commit or file that could not be read is `Unreadable`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathEvidence {
    /// Every commit in range, newest first.
    range: Vec<Oid>,
    /// The commits in range that changed the path, newest first.
    changes: Vec<PathChange>,
    /// The range is the whole branch: nothing was recorded, or the
    /// recorded commit is no longer an ancestor of the branch. Such a
    /// range holds commits older than the request. It can show that the
    /// path was changed from elsewhere; it cannot show a commit to be the
    /// request's own.
    widened: bool,
    /// The file was committed when the request was accepted: what the
    /// recorded commit holds at the path is what the request expected.
    /// Then the request's own commit is one that changed the path from
    /// exactly that state. When it was not (the file had been edited and
    /// not committed), no commit's parent holds the expected state, and
    /// the parent is not asked.
    anchored: bool,
    /// The file as it was committed at acceptance already was what the
    /// request intends: the request had nothing to commit, so no commit
    /// is its own, whatever the range holds. Another request's commit
    /// can be made from exactly that state and leave every field this
    /// request sets as it intends, by changing one it does not set.
    /// Never so for a create, which is accepted against no file.
    settled_at_acceptance: bool,
    /// The branch's tip still holds at the path what the request
    /// expected: whatever history shows, the file has not been changed
    /// from elsewhere.
    tip_as_expected: bool,
    /// The commits that can be the request's own at another path the
    /// request changes in the same commit, when it changes one: a commit
    /// is the request's own only if it is among them too.
    together_with: Option<Vec<Oid>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PathChange {
    commit: Oid,
    /// Whether the commit left the path as the request intended.
    intended: bool,
    /// Whether the commit's first parent held at the path what the
    /// request expected.
    from_expected: bool,
}

impl PathEvidence {
    /// The evidence of a bounded range that holds exactly `changes`,
    /// newest first, each with whether it left the path as intended, for
    /// a request whose file was not committed when it was accepted.
    #[cfg(test)]
    pub(crate) fn of_changes_for_testing(changes: &[(Oid, bool)]) -> Self {
        Self {
            range: changes.iter().map(|(commit, _)| *commit).collect(),
            changes: changes
                .iter()
                .map(|(commit, intended)| PathChange {
                    commit: *commit,
                    intended: *intended,
                    from_expected: false,
                })
                .collect(),
            widened: false,
            anchored: false,
            settled_at_acceptance: false,
            tip_as_expected: false,
            together_with: None,
        }
    }

    /// This evidence, for a request that changes the path of `other` in
    /// the same commit, as a move changes the path it moves from: a
    /// commit can be the request's own only if it can be at both paths.
    /// One that brought this path to what the request intends and left
    /// the other as it was is not the request's.
    pub(crate) fn together_with(mut self, other: &Self) -> Self {
        self.together_with = Some(other.candidates().collect());
        self
    }

    /// The commits that can be the request's own, oldest first: each left
    /// the path as intended and, when the file was committed at
    /// acceptance, changed it from exactly the state the request was
    /// accepted against. A commit of the same content made over anything
    /// else, by another request or by hand, is not the request's. None at
    /// all when the request had nothing to commit.
    fn candidates(&self) -> impl Iterator<Item = Oid> + '_ {
        self.changes
            .iter()
            .rev()
            .filter(|change| {
                !self.settled_at_acceptance
                    && change.intended
                    && (!self.anchored || change.from_expected)
                    && self
                        .together_with
                        .as_ref()
                        .is_none_or(|both| both.contains(&change.commit))
            })
            .map(|change| change.commit)
    }

    /// The commit that is the request's own, if an attempt of the request
    /// committed: the oldest candidate in range. None from a range widened
    /// to the whole branch.
    pub(crate) fn own(&self) -> Option<Oid> {
        if self.widened {
            return None;
        }
        self.candidates().next()
    }

    /// Whether the path was changed from elsewhere: the newest commit in
    /// range that changed it left it as something the request did not
    /// intend.
    ///
    /// With one exception. While the branch's tip holds at the path what
    /// the request expected, and the range shows no commit of the
    /// request's own, nothing foreign stands at the path: what the range
    /// shows is history older than the request, which a widened range
    /// holds, or changes that came and went and left the file as the
    /// request found it. When the request's own commit is in range and
    /// the tip is back at what the request expected, someone undid that
    /// commit, and that is a change from elsewhere.
    ///
    /// `committing` says whether an earlier attempt of the request may
    /// have committed, and a commit is the request's own here only then,
    /// as it is wherever a commit is counted or reported. For a request
    /// that cannot have committed, a commit in range that looks exactly
    /// like its own is someone else's: undone, it leaves nothing of the
    /// request's undone, and the exception holds.
    pub(crate) fn superseded(&self, committing: bool) -> bool {
        let newest_is_foreign = self.changes.first().is_some_and(|change| !change.intended);
        let own = self.own().filter(|_| committing);
        newest_is_foreign && !(self.tip_as_expected && own.is_none())
    }

    /// The commit a call reports, given this evidence read after its
    /// domain call and `before` read ahead of it.
    ///
    /// A commit is reported only if it can be the request's own (see
    /// `candidates`), and one of two things holds. `committing`: before
    /// the call the operation's journal row showed that an earlier attempt
    /// of this request may have committed, and the range is bounded by the
    /// recorded commit; the commit is then the oldest candidate. Or the
    /// commit was not there before the call, so this call made it.
    /// Otherwise no commit is the request's: identical content that was
    /// there before is someone else's.
    pub(crate) fn reported(&self, committing: bool, before: &Self) -> Option<Oid> {
        self.candidates()
            .find(|commit| (committing && !self.widened) || !before.range.contains(commit))
    }
}

/// Reads what Git shows of `path` since `position`. `expected` is what the
/// request expected at the path when it was accepted; `intended` decides
/// from a file's bytes whether they are what the request intends.
pub(crate) fn path_evidence(
    root: &Path,
    position: &Position,
    path: &str,
    expected: &ExpectedPathObservation,
    intended: &dyn Fn(&[u8]) -> bool,
) -> Result<PathEvidence, Unreadable> {
    path_evidence_of(root, position, path, expected, &|file| {
        file.is_some_and(intended)
    })
}

/// As `path_evidence`, for a request that may intend the path to hold no
/// file: `intended` is asked with `None` of a commit that has none
/// there. A move intends that of the path it moves its document from.
pub(crate) fn path_evidence_of(
    root: &Path,
    position: &Position,
    path: &str,
    expected: &ExpectedPathObservation,
    intended: &dyn Fn(Option<&[u8]>) -> bool,
) -> Result<PathEvidence, Unreadable> {
    let read = || -> Result<PathEvidence, git2::Error> {
        let repository = Repository::open(root)?;
        let path = Path::new(path);
        let Range { commits, tip, base } = range_of(&repository, position)?;
        let holds_expected = |commit: Oid| -> Result<bool, git2::Error> {
            Ok(observed_at(&repository, &repository.find_commit(commit)?, path)? == *expected)
        };
        let mut changes = Vec::new();
        for commit in &commits {
            if !changed_path(&repository, *commit, path)? {
                continue;
            }
            let found = repository.find_commit(*commit)?;
            changes.push(PathChange {
                commit: *commit,
                intended: match blob_at(&found, path)? {
                    Some(blob) => intended(Some(repository.find_blob(blob)?.content())),
                    // The commit removed the file.
                    None => intended(None),
                },
                // A root commit changed the path from nothing.
                from_expected: if found.parent_count() == 0 {
                    *expected == ExpectedPathObservation::Missing
                } else {
                    observed_at(&repository, &found.parent(0)?, path)? == *expected
                },
            });
        }
        let anchored = base.map(holds_expected).transpose()?.unwrap_or(false);
        let base_intended = match base {
            Some(base) => match blob_at(&repository.find_commit(base)?, path)? {
                Some(blob) => intended(Some(repository.find_blob(blob)?.content())),
                None => intended(None),
            },
            None => false,
        };
        Ok(PathEvidence {
            range: commits,
            changes,
            widened: tip.is_some() && base.is_none(),
            anchored,
            settled_at_acceptance: anchored && base_intended,
            tip_as_expected: tip.map(holds_expected).transpose()?.unwrap_or(false),
            together_with: None,
        })
    };
    read().map_err(|_| Unreadable)
}

/// The bytes of the file at `path` in `commit`. `None` when no file is
/// there, or the repository does not hold the commit.
pub(crate) fn file_at(root: &Path, commit: Oid, path: &str) -> Result<Option<Vec<u8>>, Unreadable> {
    let read = || -> Result<Option<Vec<u8>>, git2::Error> {
        let repository = Repository::open(root)?;
        let commit = match repository.find_commit(commit) {
            Ok(commit) => commit,
            Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        match blob_at(&commit, Path::new(path))? {
            Some(blob) => Ok(Some(repository.find_blob(blob)?.content().to_vec())),
            None => Ok(None),
        }
    };
    read().map_err(|_| Unreadable)
}

/// The files `commit` holds below the directory `under`, each with its
/// path from the root, with forward slashes. None when the repository
/// does not hold the commit or the commit has no such directory.
pub(crate) fn files_under(
    root: &Path,
    commit: Oid,
    under: &str,
) -> Result<Vec<(String, Vec<u8>)>, Unreadable> {
    let read = || -> Result<Vec<(String, Vec<u8>)>, git2::Error> {
        let repository = Repository::open(root)?;
        let tree = match repository.find_commit(commit) {
            Ok(commit) => commit.tree()?,
            Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let prefix = format!("{under}/");
        let mut blobs = Vec::new();
        tree.walk(git2::TreeWalkMode::PreOrder, |directory, entry| {
            let name = String::from_utf8_lossy(entry.name_bytes());
            let path = format!("{directory}{name}");
            match entry.kind() {
                Some(git2::ObjectType::Blob) if path.starts_with(&prefix) => {
                    blobs.push((path, entry.id()));
                    git2::TreeWalkResult::Ok
                }
                Some(git2::ObjectType::Tree) if path != under && !path.starts_with(&prefix) => {
                    git2::TreeWalkResult::Skip
                }
                _ => git2::TreeWalkResult::Ok,
            }
        })?;
        blobs
            .into_iter()
            .map(|(path, blob)| Ok((path, repository.find_blob(blob)?.content().to_vec())))
            .collect()
    };
    read().map_err(|_| Unreadable)
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
            let blob = blob_at(&repository.find_commit(commit)?, path)?;
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
