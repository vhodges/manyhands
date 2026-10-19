//! What a test compares to show the desktop changed nothing in a
//! repository: its refs, worktrees, status and Git configuration bytes.

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::{Repository, StatusOptions};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RepositorySnapshot {
    /// Each reference by name, with its target or symbolic target.
    pub refs: Vec<(String, String)>,
    /// Each linked worktree by name, with its path.
    pub worktrees: Vec<(String, PathBuf)>,
    /// Each path with a status, untracked and ignored included.
    pub status: Vec<(String, u32)>,
    /// The bytes of `.git/config`.
    pub config: Vec<u8>,
}

impl RepositorySnapshot {
    /// Reads the repository at `root` without changing it.
    pub(crate) fn take(root: &Path) -> Self {
        let repository = Repository::open(root).unwrap();
        let mut refs: Vec<(String, String)> = repository
            .references()
            .unwrap()
            .map(|reference| {
                let reference = reference.unwrap();
                let name = reference.name().unwrap().to_owned();
                let target = match reference.target() {
                    Some(oid) => oid.to_string(),
                    None => reference.symbolic_target().unwrap_or_default().to_owned(),
                };
                (name, target)
            })
            .collect();
        refs.sort();
        if let Ok(head) = repository.find_reference("HEAD") {
            refs.push((
                "HEAD".to_owned(),
                head.symbolic_target()
                    .map(str::to_owned)
                    .or_else(|| head.target().map(|oid| oid.to_string()))
                    .unwrap_or_default(),
            ));
        }
        let mut worktrees: Vec<(String, PathBuf)> = repository
            .worktrees()
            .unwrap()
            .iter()
            .flatten()
            .map(|name| {
                let worktree = repository.find_worktree(name).unwrap();
                (name.to_owned(), worktree.path().to_owned())
            })
            .collect();
        worktrees.sort();
        let mut options = StatusOptions::new();
        options
            .include_untracked(true)
            .include_ignored(true)
            .recurse_untracked_dirs(true);
        let mut status: Vec<(String, u32)> = repository
            .statuses(Some(&mut options))
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry.path().unwrap_or_default().to_owned(),
                    entry.status().bits(),
                )
            })
            .collect();
        status.sort();
        let config = fs::read(repository.path().join("config")).unwrap();
        Self {
            refs,
            worktrees,
            status,
            config,
        }
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
