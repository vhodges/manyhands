//! When an item last changed, found without walking its history item by item.
//!
//! An item context finds its commit with a short walk restricted to the
//! item's own paths. The primary keeps the newest commit of every path in one
//! map, advanced over the commits added since it was last built.
#![cfg_attr(not(test), allow(dead_code))] // Consumed by the per-context indexing that follows.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use git2::{Commit, Diff, DiffOptions, Oid, Repository};
use rusqlite::{OptionalExtension, Transaction, params};
use time::OffsetDateTime;

use super::{RepositoryError, discovery::MAX_ACTIVITY_HISTORY_COMMITS};

/// A commit that touched a path, and where it sits on the first-parent chain.
/// A larger ordinal is a newer commit on the same chain.
///
/// An item walk numbers a commit by minus its distance from the head, so the
/// head is 0. A map rebuild numbers the chain from its root, which is 0. An
/// incremental advance numbers its new commits one by one after the largest
/// ordinal the map holds, so ordinals still order the chain but are no longer
/// depths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CommitActivity {
    pub oid: Oid,
    pub at: OffsetDateTime,
    pub ordinal: i64,
}

/// What a walk found, and how many commits it examined to find it.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ItemWalk {
    pub found: Option<CommitActivity>,
    pub examined: usize,
}

/// The newest commit on `head`'s first-parent chain that touches one of the
/// `owned` paths, within the last `MAX_ACTIVITY_HISTORY_COMMITS` commits.
///
/// Each commit is diffed against its first parent, restricted to the owned
/// paths, which are matched literally, and the walk stops at the first match.
/// A root commit is diffed against the empty tree.
pub(super) fn item_commit_activity(
    repository: &Repository,
    head: Oid,
    owned: &[PathBuf],
) -> Result<ItemWalk, git2::Error> {
    let mut walk = ItemWalk {
        found: None,
        examined: 0,
    };
    if owned.is_empty() {
        return Ok(walk);
    }
    let mut options = DiffOptions::new();
    options.disable_pathspec_match(true);
    for path in owned {
        options.pathspec(path.as_path());
    }
    let mut next = Some(repository.find_commit(head)?);
    while let Some(commit) = next {
        if walk.examined == MAX_ACTIVITY_HISTORY_COMMITS {
            break;
        }
        let parent = first_parent(&commit)?;
        let diff = diff_against(repository, &commit, parent.as_ref(), Some(&mut options))?;
        // The pathspec can also match what lies under an owned path.
        let touched = changed_paths(&diff).any(|path| owned.iter().any(|owned| owned == path));
        if touched {
            walk.found = Some(CommitActivity {
                oid: commit.id(),
                at: commit_time(&commit)?,
                ordinal: -(walk.examined as i64),
            });
            walk.examined += 1;
            break;
        }
        walk.examined += 1;
        next = parent;
    }
    Ok(walk)
}

/// The newest first-parent commit of the primary that touched each path under
/// `.manyhands/` and `docs/`, as of `base`.
///
/// It is built once from the whole first-parent chain, then advanced over the
/// commits added since. It has no window: an item untouched for longer than
/// an item walk looks back still has its time.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct PrimaryActivityMap {
    base: Option<Oid>,
    newest: BTreeMap<PathBuf, CommitActivity>,
}

/// What `PrimaryActivityMap::advance` did, and how many commits it diffed.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Advance {
    Unchanged,
    Incremental { diffed: usize },
    Rebuilt { diffed: usize },
}

impl PrimaryActivityMap {
    /// The map stored for a registration: the entries of `primary_activity`
    /// and the base its primary context records. An entry that cannot be read
    /// discards the map, which the next `advance` rebuilds.
    pub(super) fn load(tx: &Transaction<'_>, repository_id: i64) -> Result<Self, RepositoryError> {
        let base = tx
            .query_row(
                "SELECT activity_base FROM contexts
                 WHERE repository_id = ?1 AND kind = 'primary' ORDER BY id LIMIT 1",
                [repository_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(RepositoryError::sqlite)?
            .flatten();
        let mut statement = tx
            .prepare(
                "SELECT path, commit_oid, committed_at, ordinal FROM primary_activity
                 WHERE repository_id = ?1",
            )
            .map_err(RepositoryError::sqlite)?;
        let rows = statement
            .query_map([repository_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(RepositoryError::sqlite)?;
        let mut newest = BTreeMap::new();
        for row in rows {
            let (path, oid, committed_at, ordinal) = row.map_err(RepositoryError::sqlite)?;
            let (Ok(oid), Ok(at)) = (
                Oid::from_str(&oid),
                OffsetDateTime::from_unix_timestamp(committed_at),
            ) else {
                return Ok(Self::default());
            };
            newest.insert(PathBuf::from(path), CommitActivity { oid, at, ordinal });
        }
        Ok(Self {
            base: base.and_then(|base| Oid::from_str(&base).ok()),
            newest,
        })
    }

    /// Replaces the stored entries and the base of the registration's primary
    /// context. Without a primary context the entries are stored and the base
    /// is not, so the map read back has no base and rebuilds.
    pub(super) fn store(
        &self,
        tx: &Transaction<'_>,
        repository_id: i64,
    ) -> Result<(), RepositoryError> {
        tx.execute(
            "DELETE FROM primary_activity WHERE repository_id = ?1",
            [repository_id],
        )
        .map_err(RepositoryError::sqlite)?;
        let mut insert = tx
            .prepare(
                "INSERT INTO primary_activity (repository_id, path, commit_oid, committed_at, ordinal)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(RepositoryError::sqlite)?;
        for (path, activity) in &self.newest {
            // An item cannot own a path that is not UTF-8.
            let Some(path) = path.to_str() else {
                continue;
            };
            insert
                .execute(params![
                    repository_id,
                    path,
                    activity.oid.to_string(),
                    activity.at.unix_timestamp(),
                    activity.ordinal
                ])
                .map_err(RepositoryError::sqlite)?;
        }
        tx.execute(
            "UPDATE contexts SET activity_base = ?2 WHERE repository_id = ?1 AND kind = 'primary'",
            params![repository_id, self.base.map(|base| base.to_string())],
        )
        .map_err(RepositoryError::sqlite)?;
        Ok(())
    }

    /// Brings the map to `head`. It diffs only the commits since its base
    /// when the base is on `head`'s first-parent chain. Otherwise (a reset, an
    /// amend, or a head that reaches the base only through a second parent)
    /// it is built again from the whole chain, one diff per commit. The map
    /// is unchanged when a commit cannot be read.
    pub(super) fn advance(
        &mut self,
        repository: &Repository,
        head: Oid,
    ) -> Result<Advance, git2::Error> {
        if self.base == Some(head) {
            return Ok(Advance::Unchanged);
        }
        // The chain from `head`, newest first, up to the base if it is on it:
        // being on the chain makes the base an ancestor of `head`.
        let mut chain = Vec::new();
        let mut incremental = false;
        let mut next = Some(repository.find_commit(head)?);
        while let Some(commit) = next {
            if Some(commit.id()) == self.base {
                incremental = true;
                break;
            }
            chain.push(commit.id());
            next = first_parent(&commit)?;
        }
        let first = if incremental {
            self.newest
                .values()
                .map(|activity| activity.ordinal)
                .max()
                .map_or(0, |newest| newest + 1)
        } else {
            0
        };
        let mut updated = BTreeMap::new();
        for (offset, oid) in chain.iter().rev().enumerate() {
            let commit = repository.find_commit(*oid)?;
            let activity = CommitActivity {
                oid: *oid,
                at: commit_time(&commit)?,
                ordinal: first + offset as i64,
            };
            let parent = first_parent(&commit)?;
            let diff = diff_against(repository, &commit, parent.as_ref(), None)?;
            for path in changed_paths(&diff).filter(|path| is_kept(path)) {
                updated.insert(path.to_owned(), activity);
            }
        }
        let diffed = chain.len();
        self.base = Some(head);
        if incremental {
            self.newest.extend(updated);
            Ok(Advance::Incremental { diffed })
        } else {
            self.newest = updated;
            Ok(Advance::Rebuilt { diffed })
        }
    }

    /// The newest commit that touched any of the `owned` paths: the one with
    /// the largest ordinal, which is the first an item walk would meet.
    pub(super) fn newest_over(&self, owned: &[PathBuf]) -> Option<CommitActivity> {
        owned
            .iter()
            .filter_map(|path| self.newest.get(path))
            .max_by_key(|activity| activity.ordinal)
            .copied()
    }
}

/// The paths a map keeps.
fn is_kept(path: &Path) -> bool {
    path.starts_with(".manyhands") || path.starts_with("docs")
}

fn commit_time(commit: &Commit<'_>) -> Result<OffsetDateTime, git2::Error> {
    OffsetDateTime::from_unix_timestamp(commit.time().seconds())
        .map_err(|error| git2::Error::from_str(&error.to_string()))
}

/// A commit's first parent; a root commit has none.
fn first_parent<'repository>(
    commit: &Commit<'repository>,
) -> Result<Option<Commit<'repository>>, git2::Error> {
    (commit.parent_count() > 0)
        .then(|| commit.parent(0))
        .transpose()
}

/// What a commit changed against its first parent, or against the empty tree
/// when it is a root.
fn diff_against<'repository>(
    repository: &'repository Repository,
    commit: &Commit<'repository>,
    parent: Option<&Commit<'repository>>,
    options: Option<&mut DiffOptions>,
) -> Result<Diff<'repository>, git2::Error> {
    let parent_tree = parent.map(Commit::tree).transpose()?;
    repository.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit.tree()?), options)
}

/// The old and the new path of every change, so a deletion counts as touching
/// its path.
fn changed_paths<'diff>(diff: &'diff Diff<'_>) -> impl Iterator<Item = &'diff Path> {
    diff.deltas()
        .flat_map(|delta| [delta.old_file().path(), delta.new_file().path()])
        .flatten()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use git2::{
        IndexEntry, IndexTime, ObjectType, Repository, RepositoryInitOptions, ResetType, Signature,
        Time, WorktreeAddOptions,
    };
    use time::OffsetDateTime;

    use super::*;
    use crate::repository::{
        DiscoveryActivitySource,
        discovery::{
            ActivityHistory, MAX_ACTIVITY_HISTORY_COMMITS, activity_with_uncommitted,
            commit_touches_paths, first_parent_commits, latest_commit_touching, migrate_registry,
            observe_activity, open_registry,
        },
    };

    const ITEM_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    const DOCUMENT: &str = "docs/document.md";
    const TICKET: &str = ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md";
    const COMMENT: &str =
        ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAW/01ARZ3NDEKTSV4RRFFQ69G5FAX.md";

    fn time_at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(seconds).unwrap()
    }

    fn owned(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn repository_on_main() -> (tempfile::TempDir, Repository) {
        let directory = tempfile::tempdir().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(directory.path(), &options).unwrap();
        fs::write(directory.path().join("fixture.txt"), "fixture\n").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("fixture.txt")).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = Signature::new("Test", "test@example.invalid", &Time::new(0, 0)).unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
            .unwrap();
        drop(tree);
        index.write().unwrap();
        (directory, repository)
    }

    fn checkpoint(repository: &Repository, path: &Path, timestamp: i64) -> git2::Oid {
        let mut index = repository.index().unwrap();
        index.add_path(path).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repository.head().unwrap().peel_to_commit().unwrap();
        let signature =
            Signature::new("Test", "test@example.invalid", &Time::new(timestamp, 0)).unwrap();
        let oid = repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Checkpoint",
                &tree,
                &[&parent],
            )
            .unwrap();
        drop(tree);
        index.write().unwrap();
        oid
    }

    fn checkpoint_reference(
        repository: &Repository,
        path: &Path,
        reference: &str,
        parent: git2::Oid,
        timestamp: i64,
    ) -> git2::Oid {
        let mut index = repository.index().unwrap();
        index.add_path(path).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repository.find_commit(parent).unwrap();
        let signature =
            Signature::new("Test", "test@example.invalid", &Time::new(timestamp, 0)).unwrap();
        let oid = repository
            .commit(
                Some(reference),
                &signature,
                &signature,
                "Side checkpoint",
                &tree,
                &[&parent],
            )
            .unwrap();
        drop(tree);
        index.write().unwrap();
        oid
    }

    fn active_worktree_at(
        repository: &Repository,
        root: &Path,
        kind: &str,
        branch_item_id: &str,
        worktree_item_id: &str,
    ) -> PathBuf {
        let branch = format!("manyhands/{kind}/{branch_item_id}");
        let head = repository.head().unwrap().peel_to_commit().unwrap();
        let reference = repository
            .branch(&branch, &head, false)
            .unwrap()
            .into_reference();
        let worktree = root.join(".manyhands/worktrees").join(worktree_item_id);
        fs::create_dir_all(worktree.parent().unwrap()).unwrap();
        let mut options = WorktreeAddOptions::new();
        options.reference(Some(&reference));
        repository
            .worktree(worktree_item_id, &worktree, Some(&options))
            .unwrap();
        worktree
    }

    /// A path and its new content and mode, or `None` for a deletion.
    type Edit<'a> = (&'a str, Option<(&'a str, u32)>);

    /// A row's name and how to build its repository.
    type Row<T> = (&'static str, fn() -> T);

    /// Commits `edits` onto the first parent's tree (an empty tree for a root
    /// commit) without touching the checkout.
    fn commit_edits(
        repository: &Repository,
        reference: &str,
        parents: &[git2::Oid],
        edits: &[Edit<'_>],
        timestamp: i64,
    ) -> git2::Oid {
        let parents = parents
            .iter()
            .map(|parent| repository.find_commit(*parent).unwrap())
            .collect::<Vec<_>>();
        let mut index = git2::Index::new().unwrap();
        if let Some(first) = parents.first() {
            index.read_tree(&first.tree().unwrap()).unwrap();
        }
        for (path, edit) in edits {
            match edit {
                Some((content, mode)) => index
                    .add(&IndexEntry {
                        ctime: IndexTime::new(0, 0),
                        mtime: IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode: *mode,
                        uid: 0,
                        gid: 0,
                        file_size: content.len() as u32,
                        id: repository.blob(content.as_bytes()).unwrap(),
                        flags: 0,
                        flags_extended: 0,
                        path: path.as_bytes().to_vec(),
                    })
                    .unwrap(),
                None => index.remove_path(Path::new(path)).unwrap(),
            }
        }
        let tree = repository
            .find_tree(index.write_tree_to(repository).unwrap())
            .unwrap();
        let signature =
            Signature::new("Test", "test@example.invalid", &Time::new(timestamp, 0)).unwrap();
        let parents = parents.iter().collect::<Vec<_>>();
        repository
            .commit(
                Some(reference),
                &signature,
                &signature,
                "Edit",
                &tree,
                &parents,
            )
            .unwrap()
    }

    /// A commit on HEAD that changes `path` to content naming `timestamp`.
    fn touch(repository: &Repository, path: &str, timestamp: i64) -> git2::Oid {
        let tip = repository.head().unwrap().target().unwrap();
        let content = format!("{path} at {timestamp}\n");
        commit_edits(
            repository,
            "HEAD",
            &[tip],
            &[(path, Some((&content, 0o100644)))],
            timestamp,
        )
    }

    /// Makes the index and files match HEAD after commits made in memory.
    fn sync_checkout(repository: &Repository) {
        let head = repository.head().unwrap().peel(ObjectType::Commit).unwrap();
        repository.reset(&head, ResetType::Hard, None).unwrap();
    }

    /// A repository whose HEAD is the head under test, with the files its
    /// checkout holds at `root` and the paths an item owns.
    struct History {
        _directory: tempfile::TempDir,
        repository: Repository,
        root: PathBuf,
        owned: Vec<PathBuf>,
        /// The commit time the oracle must find, in seconds.
        commit: Option<i64>,
        /// Whether an uncommitted edit of an owned path is newer.
        uncommitted: bool,
    }

    fn on_main(
        owned_paths: &[&str],
        commit: Option<i64>,
        build: impl FnOnce(&Path, &Repository),
    ) -> History {
        let (directory, repository) = repository_on_main();
        build(directory.path(), &repository);
        History {
            root: directory.path().to_owned(),
            _directory: directory,
            repository,
            owned: owned(owned_paths),
            commit,
            uncommitted: false,
        }
    }

    fn documents_then_comment(owned_paths: &[&str]) -> History {
        on_main(owned_paths, Some(20), |root, repository| {
            write(root, DOCUMENT, "document\n");
            checkpoint(repository, Path::new(DOCUMENT), 10);
            write(root, COMMENT, "comment\n");
            checkpoint(repository, Path::new(COMMENT), 20);
        })
    }

    fn side_branch_only() -> History {
        on_main(&[DOCUMENT], Some(10), |root, repository| {
            write(root, DOCUMENT, "document\n");
            let base = checkpoint(repository, Path::new(DOCUMENT), 10);
            repository
                .branch("side", &repository.find_commit(base).unwrap(), false)
                .unwrap();
            write(root, DOCUMENT, "document\nside branch\n");
            let side = checkpoint_reference(
                repository,
                Path::new(DOCUMENT),
                "refs/heads/side",
                base,
                100,
            );
            let base_object = repository.find_object(base, None).unwrap();
            repository
                .reset(&base_object, ResetType::Hard, None)
                .unwrap();
            write(root, "fixture.txt", "main\n");
            let main = checkpoint(repository, Path::new("fixture.txt"), 20);
            // The merge keeps main's tree, so the side's change never lands.
            commit_edits(repository, "HEAD", &[main, side], &[], 30);
        })
    }

    fn mode_only_change() -> History {
        on_main(&[DOCUMENT], Some(20), |root, repository| {
            write(root, DOCUMENT, "document\n");
            let tip = checkpoint(repository, Path::new(DOCUMENT), 10);
            commit_edits(
                repository,
                "HEAD",
                &[tip],
                &[(DOCUMENT, Some(("document\n", 0o100755)))],
                20,
            );
            sync_checkout(repository);
        })
    }

    /// `docs/a[b]*?.md` is touched once, between two commits of `docs/abXY.md`,
    /// which a glob of it would match.
    fn glob_characters(owned_path: &str, commit: i64) -> History {
        on_main(&[owned_path], Some(commit), |_, repository| {
            touch(repository, "docs/abXY.md", 100);
            touch(repository, "docs/a[b]*?.md", 10);
            touch(repository, "docs/abXY.md", 200);
        })
    }

    fn root_commit() -> History {
        let directory = tempfile::tempdir().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(directory.path(), &options).unwrap();
        let root = commit_edits(
            &repository,
            "HEAD",
            &[],
            &[(DOCUMENT, Some(("document\n", 0o100644)))],
            5,
        );
        commit_edits(
            &repository,
            "HEAD",
            &[root],
            &[("fixture.txt", Some(("fixture\n", 0o100644)))],
            6,
        );
        History {
            root: directory.path().to_owned(),
            _directory: directory,
            repository,
            owned: owned(&[DOCUMENT]),
            commit: Some(5),
            uncommitted: false,
        }
    }

    fn merged_item_on_main() -> History {
        on_main(&[TICKET], Some(30), |root, repository| {
            let base = repository.head().unwrap().target().unwrap();
            let branch = format!("refs/heads/manyhands/ticket/{ITEM_ID}");
            let ticket = Some(("ticket\n", 0o100644));
            let item = commit_edits(repository, &branch, &[base], &[(TICKET, ticket)], 100);
            write(root, "fixture.txt", "moved\n");
            let main = checkpoint(repository, Path::new("fixture.txt"), 20);
            commit_edits(repository, "HEAD", &[main, item], &[(TICKET, ticket)], 30);
            sync_checkout(repository);
        })
    }

    /// An item branch that merges the primary after the primary moved on. The
    /// merge also changes the item's own file when `changes_ticket` is set.
    fn primary_merged_into_item_branch(changes_ticket: bool) -> History {
        let (directory, repository) = repository_on_main();
        let worktree =
            active_worktree_at(&repository, directory.path(), "ticket", ITEM_ID, ITEM_ID);
        let item = Repository::open(&worktree).unwrap();
        write(&worktree, TICKET, "ticket\n");
        let item_tip = checkpoint(&item, Path::new(TICKET), 10);
        write(directory.path(), "fixture.txt", "moved\n");
        let main_tip = checkpoint(&repository, Path::new("fixture.txt"), 20);
        let mut edits = vec![("fixture.txt", Some(("moved\n", 0o100644)))];
        if changes_ticket {
            edits.push((TICKET, Some(("ticket from the primary\n", 0o100644))));
        }
        commit_edits(&item, "HEAD", &[item_tip, main_tip], &edits, 30);
        sync_checkout(&item);
        History {
            root: worktree,
            _directory: directory,
            repository: item,
            owned: owned(&[TICKET]),
            commit: Some(if changes_ticket { 30 } else { 10 }),
            uncommitted: false,
        }
    }

    fn uncommitted_edit() -> History {
        let mut history = on_main(&[DOCUMENT], Some(10), |root, repository| {
            write(root, DOCUMENT, "document\n");
            checkpoint(repository, Path::new(DOCUMENT), 10);
            write(root, DOCUMENT, "document\nuncommitted\n");
        });
        history.uncommitted = true;
        history
    }

    #[test]
    fn activity_matches_latest_commit_touching_for_every_history() {
        let rows: [Row<History>; 16] = [
            ("direct commits", || {
                on_main(&[DOCUMENT], Some(30), |root, repository| {
                    write(root, DOCUMENT, "one\n");
                    checkpoint(repository, Path::new(DOCUMENT), 10);
                    write(root, "fixture.txt", "two\n");
                    checkpoint(repository, Path::new("fixture.txt"), 20);
                    write(root, DOCUMENT, "three\n");
                    checkpoint(repository, Path::new(DOCUMENT), 30);
                    write(root, "fixture.txt", "four\n");
                    checkpoint(repository, Path::new("fixture.txt"), 40);
                })
            }),
            ("a backdated first-parent commit", || {
                on_main(&[DOCUMENT], Some(10), |root, repository| {
                    write(root, DOCUMENT, "one\n");
                    checkpoint(repository, Path::new(DOCUMENT), 100);
                    write(root, DOCUMENT, "two\n");
                    checkpoint(repository, Path::new(DOCUMENT), 10);
                })
            }),
            ("a backdated commit across two owned paths", || {
                on_main(&[DOCUMENT, COMMENT], Some(5), |root, repository| {
                    write(root, DOCUMENT, "document\n");
                    checkpoint(repository, Path::new(DOCUMENT), 100);
                    write(root, COMMENT, "comment\n");
                    checkpoint(repository, Path::new(COMMENT), 5);
                })
            }),
            ("a side-branch-only change", side_branch_only),
            ("a comment added", || {
                documents_then_comment(&[DOCUMENT, COMMENT])
            }),
            ("a comment invalid for its missing parent", || {
                let mut history = documents_then_comment(&[DOCUMENT]);
                history.commit = Some(10);
                history
            }),
            ("a mode-only change", mode_only_change),
            (
                "a path that holds brackets, a star and a question mark",
                || glob_characters("docs/a[b]*?.md", 10),
            ),
            ("a path a glob of another would match", || {
                glob_characters("docs/abXY.md", 200)
            }),
            (
                "an owned path that only a directory of that name held",
                || {
                    on_main(&[DOCUMENT], None, |_, repository| {
                        touch(repository, "docs/document.md/part.md", 10);
                    })
                },
            ),
            ("a root commit", root_commit),
            ("a merged item on main", merged_item_on_main),
            ("the primary merged into an item branch", || {
                primary_merged_into_item_branch(false)
            }),
            ("the primary merged in with a change to the item", || {
                primary_merged_into_item_branch(true)
            }),
            ("an uncommitted edit over a commit", uncommitted_edit),
            ("no commit touches the owned paths", || {
                on_main(&[DOCUMENT], None, |root, repository| {
                    write(root, "fixture.txt", "changed\n");
                    checkpoint(repository, Path::new("fixture.txt"), 10);
                })
            }),
        ];

        for (row, build) in rows {
            let History {
                repository,
                root,
                owned,
                commit,
                uncommitted,
                ..
            } = &build();
            let head = repository.head().unwrap().target().unwrap();

            // The oracle is the walk the observation makes today.
            let oracle_commit = latest_commit_touching(repository, owned);
            assert_eq!(oracle_commit, commit.map(time_at), "{row}: oracle commit");
            let commits = first_parent_commits(repository, MAX_ACTIVITY_HISTORY_COMMITS);
            let oracle_examined = commits
                .iter()
                .position(|commit| commit_touches_paths(repository, commit, owned))
                .map_or(commits.len(), |position| position + 1);
            let oracle = observe_activity(repository, root, ActivityHistory::Checkout, owned);
            assert_eq!(
                oracle.map(|(_, source)| source),
                if *uncommitted {
                    Some(DiscoveryActivitySource::UncommittedFilesystem)
                } else {
                    commit.map(|_| DiscoveryActivitySource::GitCommit)
                },
                "{row}: oracle source"
            );

            // An item context finds the same commit, with no more commits examined.
            let walk = item_commit_activity(repository, head, owned).unwrap();
            assert_eq!(
                walk.found.map(|found| found.at),
                oracle_commit,
                "{row}: walk"
            );
            assert!(
                walk.examined <= oracle_examined,
                "{row}: examined {} of the oracle's {oracle_examined}",
                walk.examined
            );
            assert_eq!(
                observe_activity(repository, root, ActivityHistory::Item(Some(head)), owned),
                oracle,
                "{row}: item observation"
            );

            // The primary's map, built from no map, answers the same.
            let mut map = PrimaryActivityMap::default();
            assert_eq!(
                map.advance(repository, head).unwrap(),
                Advance::Rebuilt {
                    diffed: first_parent_commits(repository, usize::MAX).len()
                },
                "{row}: advance"
            );
            let newest = map.newest_over(owned).map(|found| found.at);
            assert_eq!(newest, oracle_commit, "{row}: map");
            assert_eq!(
                activity_with_uncommitted(repository, root, owned, newest),
                oracle,
                "{row}: map observation"
            );
        }
    }

    /// The paths the map tests ask about.
    const MAP_PATHS: [&str; 6] = [
        "docs/a.md",
        "docs/b.md",
        "docs/c.md",
        "docs/f.md",
        "docs/h.md",
        "docs/m.md",
    ];

    /// Every one of `MAP_PATHS`, alone and with its neighbour, is answered
    /// as the oracle answers it at the repository's HEAD.
    fn assert_map_matches_oracle(map: &PrimaryActivityMap, repository: &Repository, row: &str) {
        for window in MAP_PATHS.windows(2) {
            for paths in [&window[..1], &window[1..], window] {
                let paths = owned(paths);
                assert_eq!(
                    map.newest_over(&paths).map(|found| found.at),
                    latest_commit_touching(repository, &paths),
                    "{row}: {paths:?}"
                );
            }
        }
    }

    #[test]
    fn the_primary_map_advances_incrementally_over_new_first_parent_commits() {
        let (_directory, repository) = repository_on_main();
        touch(&repository, "docs/a.md", 10);
        let b = touch(&repository, "docs/b.md", 20);
        let mut map = PrimaryActivityMap::default();

        assert_eq!(
            map.advance(&repository, b).unwrap(),
            Advance::Rebuilt { diffed: 3 }
        );
        assert_eq!(map.base, Some(b));
        assert_eq!(map.advance(&repository, b).unwrap(), Advance::Unchanged);

        touch(&repository, "docs/a.md", 30);
        touch(&repository, "src/code.rs", 40);
        // A backdated commit is the newest on the chain whatever its time.
        let tip = touch(&repository, "docs/b.md", 5);
        assert_eq!(
            map.advance(&repository, tip).unwrap(),
            Advance::Incremental { diffed: 3 }
        );

        assert_eq!(map.base, Some(tip));
        assert!(!map.newest.contains_key(Path::new("src/code.rs")));
        assert_map_matches_oracle(&map, &repository, "after an advance");
        let mut rebuilt = PrimaryActivityMap::default();
        rebuilt.advance(&repository, tip).unwrap();
        for paths in [
            &["docs/a.md"][..],
            &["docs/b.md"],
            &["docs/a.md", "docs/b.md"],
        ] {
            let paths = owned(paths);
            let found = |map: &PrimaryActivityMap| {
                map.newest_over(&paths).map(|found| (found.oid, found.at))
            };
            assert_eq!(found(&map), found(&rebuilt), "{paths:?}");
        }
        assert_eq!(map.advance(&repository, tip).unwrap(), Advance::Unchanged);
    }

    /// A repository with a map advanced to `base`, then HEAD moved to `head`.
    struct Moved {
        _directory: tempfile::TempDir,
        repository: Repository,
        map: PrimaryActivityMap,
        head: git2::Oid,
        /// The commits on `head`'s first-parent chain.
        chain: usize,
    }

    fn moved(build: impl FnOnce(&Repository) -> (git2::Oid, git2::Oid, usize)) -> Moved {
        let (directory, repository) = repository_on_main();
        let (base, head, chain) = build(&repository);
        let mut map = PrimaryActivityMap::default();
        map.advance(&repository, base).unwrap();
        assert_map_matches_oracle(&map, &repository, "at the base");
        repository
            .reference("refs/heads/main", head, true, "the new head")
            .unwrap();
        Moved {
            _directory: directory,
            repository,
            map,
            head,
            chain,
        }
    }

    #[test]
    fn the_primary_map_rebuilds_after_a_reset_amend_or_second_parent_fast_forward() {
        let rows: [Row<Moved>; 3] = [
            ("a reset to an older commit", || {
                moved(|repository| {
                    let a = touch(repository, "docs/a.md", 10);
                    touch(repository, "docs/b.md", 20);
                    let c = touch(repository, "docs/a.md", 30);
                    (c, a, 2)
                })
            }),
            ("an amend of the newest commit", || {
                moved(|repository| {
                    touch(repository, "docs/a.md", 10);
                    let b = touch(repository, "docs/b.md", 20);
                    let c = touch(repository, "docs/a.md", 30);
                    let amended = commit_edits(
                        repository,
                        "refs/heads/amended",
                        &[b],
                        &[("docs/c.md", Some(("amended\n", 0o100644)))],
                        31,
                    );
                    (c, amended, 4)
                })
            }),
            // The base is an ancestor of the new head, as its second parent,
            // but not on its first-parent chain.
            (
                "a fast-forward to a merge of the base as a second parent",
                || {
                    moved(|repository| {
                        let a = touch(repository, "docs/a.md", 10);
                        let m = touch(repository, "docs/m.md", 20);
                        let feature = commit_edits(
                            repository,
                            "refs/heads/feature",
                            &[a],
                            &[("docs/f.md", Some(("feature\n", 0o100644)))],
                            15,
                        );
                        let merge = commit_edits(
                            repository,
                            "refs/heads/feature",
                            &[feature, m],
                            &[("docs/h.md", Some(("merge\n", 0o100644)))],
                            40,
                        );
                        assert!(repository.graph_descendant_of(merge, m).unwrap());
                        (m, merge, 4)
                    })
                },
            ),
        ];

        for (row, build) in rows {
            let mut moved = build();

            assert_eq!(
                moved.map.advance(&moved.repository, moved.head).unwrap(),
                Advance::Rebuilt {
                    diffed: moved.chain
                },
                "{row}"
            );

            assert_eq!(moved.map.base, Some(moved.head), "{row}");
            assert_map_matches_oracle(&moved.map, &moved.repository, row);
        }
    }

    #[test]
    fn the_primary_map_drops_a_path_that_leaves_the_owned_set() {
        let (_directory, repository) = repository_on_main();
        touch(&repository, "docs/a.md", 10);
        touch(&repository, "docs/b.md", 20);
        touch(&repository, "src/code.rs", 30);
        let tip = repository.head().unwrap().target().unwrap();
        let mut map = PrimaryActivityMap::default();
        map.advance(&repository, tip).unwrap();

        for (paths, expected) in [
            (vec!["docs/a.md", "docs/b.md"], Some(20)),
            // docs/b.md is no longer owned: its newer commit no longer counts.
            (vec!["docs/a.md"], Some(10)),
            (vec!["docs/b.md"], Some(20)),
            (vec!["src/code.rs"], None),
            (vec!["docs/a.md", "docs/missing.md"], Some(10)),
            (vec![], None),
        ] {
            let paths = owned(&paths);
            let found = map.newest_over(&paths).map(|found| found.at);
            assert_eq!(found, expected.map(time_at), "{paths:?}");
            if expected.is_some() {
                assert_eq!(found, latest_commit_touching(&repository, &paths));
            }
        }
    }

    #[test]
    fn the_primary_map_keeps_an_item_older_than_the_window() {
        let (directory, repository) = repository_on_main();
        write(directory.path(), DOCUMENT, "document\n");
        checkpoint(&repository, Path::new(DOCUMENT), 5);
        for timestamp in 10..10 + 1030 {
            touch(&repository, "fixture.txt", timestamp);
        }
        let head = repository.head().unwrap().target().unwrap();
        let paths = owned(&[DOCUMENT]);

        // Today's walk gives up before it reaches the commit, so the
        // observation drops the item.
        assert_eq!(
            observe_activity(
                &repository,
                directory.path(),
                ActivityHistory::Checkout,
                &paths
            ),
            None
        );
        let walk = item_commit_activity(&repository, head, &paths).unwrap();
        assert_eq!(walk.found, None);
        assert_eq!(walk.examined, MAX_ACTIVITY_HISTORY_COMMITS);

        let mut map = PrimaryActivityMap::default();
        assert_eq!(
            map.advance(&repository, head).unwrap(),
            Advance::Rebuilt { diffed: 1032 }
        );
        let found = map.newest_over(&paths).unwrap();
        assert_eq!(found.at, time_at(5));
        assert_eq!(
            activity_with_uncommitted(&repository, directory.path(), &paths, Some(found.at)),
            Some((time_at(5), DiscoveryActivitySource::GitCommit))
        );
    }

    #[test]
    fn the_primary_map_round_trips_through_the_registry() {
        let (_directory, repository) = repository_on_main();
        let a = touch(&repository, "docs/a.md", 10);
        let b = touch(&repository, "docs/b.md", 20);
        let mut map = PrimaryActivityMap::default();
        map.advance(&repository, b).unwrap();
        assert_eq!((map.base, map.newest.len()), (Some(b), 2));

        let registry = tempfile::tempdir().unwrap();
        let mut connection =
            open_registry(&registry.path().join("index.sqlite3"), &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required)
                 VALUES (1, '/one', 1, 'accessible', 0), (2, '/two', 1, 'accessible', 0),
                        (3, '/three', 1, 'accessible', 0);
                 INSERT INTO contexts (id, repository_id, kind, worktree_path, activity_base)
                 VALUES (1, 1, 'primary', '/one', NULL),
                        (2, 1, 'active', '/one/.manyhands/worktrees/a', 'item'),
                        (3, 2, 'primary', '/two', 'other');
                 INSERT INTO primary_activity (repository_id, path, commit_oid, committed_at, ordinal)
                 VALUES (2, 'docs/z.md', 'zz', 1, 1);",
            )
            .unwrap();

        // What a registration has stored is loaded; nothing stored is an empty map.
        let transaction = connection.transaction().unwrap();
        assert_eq!(
            PrimaryActivityMap::load(&transaction, 1).unwrap(),
            PrimaryActivityMap::default()
        );
        map.store(&transaction, 1).unwrap();
        transaction.commit().unwrap();
        let transaction = connection.transaction().unwrap();
        assert_eq!(PrimaryActivityMap::load(&transaction, 1).unwrap(), map);
        drop(transaction);

        // Storing replaces the entries: a path the new map lacks is gone.
        repository
            .reference("refs/heads/main", a, true, "reset")
            .unwrap();
        let mut older = PrimaryActivityMap::default();
        older.advance(&repository, a).unwrap();
        assert_eq!((older.base, older.newest.len()), (Some(a), 1));
        let transaction = connection.transaction().unwrap();
        older.store(&transaction, 1).unwrap();
        transaction.commit().unwrap();
        let transaction = connection.transaction().unwrap();
        let loaded = PrimaryActivityMap::load(&transaction, 1).unwrap();
        assert_eq!(loaded, older);
        assert!(!loaded.newest.contains_key(Path::new("docs/b.md")));
        // Another registration's rows and an item context's base stay as they were.
        let (item_base, other_base): (String, String) = transaction
            .query_row(
                "SELECT (SELECT activity_base FROM contexts WHERE id = 2),
                        (SELECT activity_base FROM contexts WHERE id = 3)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((item_base.as_str(), other_base.as_str()), ("item", "other"));
        // An entry that cannot be read leaves an empty map, which rebuilds.
        assert_eq!(
            PrimaryActivityMap::load(&transaction, 2).unwrap(),
            PrimaryActivityMap::default()
        );

        // With no primary context the entries are stored and the base is not,
        // so the map read back rebuilds on its next advance.
        map.store(&transaction, 3).unwrap();
        let loaded = PrimaryActivityMap::load(&transaction, 3).unwrap();
        assert_eq!(loaded.base, None);
        assert_eq!(loaded.newest, map.newest);
    }
}
