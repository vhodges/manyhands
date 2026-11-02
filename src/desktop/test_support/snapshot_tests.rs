use super::RepositorySnapshot;
use crate::desktop::test_support::fixtures::Fixture;

#[test]
fn an_untouched_repository_snapshots_the_same() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    assert_eq!(
        RepositorySnapshot::take(&root),
        RepositorySnapshot::take(&root)
    );
}

/// The snapshot sees each thing it compares change.
#[test]
fn a_commit_a_worktree_a_status_or_a_config_change_is_seen() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let before = RepositorySnapshot::take(&root);

    fixture.commit_file(&root, "notes.txt", b"one");
    let committed = RepositorySnapshot::take(&root);
    assert_ne!(before.refs, committed.refs);

    std::fs::write(root.join("untracked.txt"), b"two").unwrap();
    let dirty = RepositorySnapshot::take(&root);
    assert_ne!(committed.status, dirty.status);

    let repository = git2::Repository::open(&root).unwrap();
    repository
        .config()
        .unwrap()
        .set_str("manyhands.test", "changed")
        .unwrap();
    let configured = RepositorySnapshot::take(&root);
    assert_ne!(dirty.config, configured.config);

    let head = repository.head().unwrap().peel_to_commit().unwrap();
    repository.branch("side", &head, false).unwrap();
    let worktree_path = fixture.home().join("side-worktree");
    let side = repository
        .find_branch("side", git2::BranchType::Local)
        .unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(side.get()));
    repository
        .worktree("side", &worktree_path, Some(&options))
        .unwrap();
    let with_worktree = RepositorySnapshot::take(&root);
    assert_ne!(configured.worktrees, with_worktree.worktrees);
}
