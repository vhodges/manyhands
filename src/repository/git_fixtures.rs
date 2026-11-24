//! Git repositories that the discovery, activity and basis tests build.

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::{Repository, RepositoryInitOptions, Signature, Time, WorktreeAddOptions};

pub(super) fn repository_on_main() -> (tempfile::TempDir, Repository) {
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

pub(super) fn checkpoint(repository: &Repository, path: &Path, timestamp: i64) -> git2::Oid {
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

pub(super) fn checkpoint_reference(
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

pub(super) fn active_worktree_at(
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
