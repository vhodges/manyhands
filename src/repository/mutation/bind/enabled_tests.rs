//! `never_enabled`: whether a Git repository that is not registered was
//! ever enabled, read from Git alone.

use std::path::Path;

use git2::{Repository, Signature, Time};

use super::never_enabled;
use crate::canonical::CONFIG_PATH;

const CONFIGURATION: &str = "format_version = 1\nprimary_branch = \"main\"\n";

fn signature() -> Signature<'static> {
    Signature::new("Test", "test@example.invalid", &Time::new(0, 0)).expect("a signature")
}

/// Commits `files` on top of `HEAD`, or as the first commit.
fn commit(repository: &Repository, files: &[(&str, &str)]) -> git2::Oid {
    let root = repository.workdir().expect("a working tree").to_owned();
    let mut index = repository.index().expect("an index");
    for (path, content) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
        std::fs::write(&file, content).expect("a file");
        index.add_path(Path::new(path)).expect("a staged file");
    }
    index.write().expect("a written index");
    let tree = repository
        .find_tree(index.write_tree().expect("a tree"))
        .expect("the tree");
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repository
        .commit(
            Some("HEAD"),
            &signature(),
            &signature(),
            "commit",
            &tree,
            &parents,
        )
        .expect("a commit")
}

/// A repository on `main` with one commit and no configuration.
fn born() -> (tempfile::TempDir, Repository, git2::Oid) {
    let directory = tempfile::tempdir().expect("a directory");
    let mut options = git2::RepositoryInitOptions::new();
    options.initial_head("main");
    let repository = Repository::init_opts(directory.path(), &options).expect("a repository");
    let first = commit(&repository, &[("fixture.txt", "fixture\n")]);
    (directory, repository, first)
}

#[test]
fn a_repository_with_no_configuration_anywhere_was_never_enabled() {
    let (directory, repository, first) = born();
    assert!(never_enabled(directory.path()));
    // Another branch without one changes nothing.
    let commit = repository.find_commit(first).expect("the commit");
    repository
        .branch("other", &commit, false)
        .expect("a branch");
    assert!(never_enabled(directory.path()));
}

#[test]
fn an_unborn_repository_with_no_configuration_was_never_enabled() {
    let directory = tempfile::tempdir().expect("a directory");
    Repository::init(directory.path()).expect("a repository");
    assert!(never_enabled(directory.path()));
}

/// A bare repository and a directory that is no repository are refused
/// as what they are, by what refuses them already: neither is called
/// never enabled.
#[test]
fn a_bare_repository_and_no_repository_are_not_called_never_enabled() {
    let bare = tempfile::tempdir().expect("a directory");
    Repository::init_bare(bare.path()).expect("a repository");
    assert!(!never_enabled(bare.path()));
    let nothing = tempfile::tempdir().expect("a directory");
    assert!(!never_enabled(nothing.path()));
}

/// An enablement that wrote its configuration and has not committed it.
#[test]
fn a_configuration_written_and_not_committed_is_an_enabled_repository() {
    let (directory, _repository, _) = born();
    let file = directory.path().join(CONFIG_PATH);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
    std::fs::write(&file, CONFIGURATION).expect("a file");
    assert!(!never_enabled(directory.path()));
}

#[test]
fn a_configuration_at_the_head_is_an_enabled_repository() {
    let (directory, repository, _) = born();
    commit(&repository, &[(CONFIG_PATH, CONFIGURATION)]);
    // Also with the file gone from the working tree.
    std::fs::remove_file(directory.path().join(CONFIG_PATH)).expect("removed");
    assert!(!never_enabled(directory.path()));
}

/// The root is on a branch from before enabling: the configuration is
/// at the tip of a local branch that is not checked out.
#[test]
fn a_configuration_only_on_a_branch_that_is_not_checked_out_is_an_enabled_repository() {
    let (directory, repository, first) = born();
    commit(&repository, &[(CONFIG_PATH, CONFIGURATION)]);
    let old = repository.find_commit(first).expect("the commit");
    repository.branch("old", &old, false).expect("a branch");
    repository.set_head("refs/heads/old").expect("a head");
    repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .expect("a checkout");
    assert!(!directory.path().join(CONFIG_PATH).exists());
    assert!(!never_enabled(directory.path()));
}

/// The root is detached at a commit from before enabling.
#[test]
fn a_detached_head_from_before_enabling_is_an_enabled_repository() {
    let (directory, repository, first) = born();
    commit(&repository, &[(CONFIG_PATH, CONFIGURATION)]);
    repository.set_head_detached(first).expect("a head");
    repository
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .expect("a checkout");
    assert!(!directory.path().join(CONFIG_PATH).exists());
    assert!(!never_enabled(directory.path()));
}
