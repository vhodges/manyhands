use super::{Error, artifacts, prepare, resolution};
use std::{cell::Cell, collections::BTreeSet, path::Path};

fn commit(repo: &git2::Repository, text: &str, parent: Option<git2::Oid>) -> git2::Oid {
    let blob = repo.blob(text.as_bytes()).unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    builder.insert("file.txt", blob, 0o100644).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let signature =
        git2::Signature::new("Fixture", "fixture@example.test", &git2::Time::new(0, 0)).unwrap();
    let parents = parent.map(|oid| repo.find_commit(oid).unwrap());
    repo.commit(
        None,
        &signature,
        &signature,
        "fixture",
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

fn fixture() -> (tempfile::TempDir, git2::Repository, git2::Oid, git2::Oid) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    let base = commit(&repo, "one\ntwo\nthree\nfour\nfive\n", None);
    let local = commit(&repo, "ONE\ntwo\nthree\nfour\nfive\n", Some(base));
    let incoming = commit(&repo, "one\ntwo\nthree\nfour\nFIVE\n", Some(base));
    repo.reference("refs/heads/main", local, true, "fixture")
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    (dir, repo, local, incoming)
}

fn objects(repo: &git2::Repository) -> BTreeSet<git2::Oid> {
    let mut result = BTreeSet::new();
    repo.odb()
        .unwrap()
        .foreach(|oid| {
            result.insert(*oid);
            true
        })
        .unwrap();
    result
}

#[test]
fn memory_preparation_has_no_persistent_objects_refs_or_index_effect() {
    let (dir, repo, local, incoming) = fixture();
    let before = objects(&repo);
    let index = std::fs::read(repo.path().join("index")).unwrap();
    let refs = std::fs::read(repo.path().join("refs/heads/main")).unwrap();
    let worktree = std::fs::read(dir.path().join("file.txt")).unwrap();
    let prepare::MergePreparation::Clean(prepared) =
        prepare::prepare_clean_merge(dir.path(), local, incoming).unwrap()
    else {
        panic!("clean merge expected")
    };
    let expected_blob =
        git2::Oid::hash_object(git2::ObjectType::Blob, b"ONE\ntwo\nthree\nfour\nFIVE\n").unwrap();
    assert!(!repo.odb().unwrap().exists(expected_blob));
    assert_eq!(objects(&repo), before);
    assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(
        std::fs::read(repo.path().join("refs/heads/main")).unwrap(),
        refs
    );
    assert_eq!(
        std::fs::read(dir.path().join("file.txt")).unwrap(),
        worktree
    );
    // Positive control: the separate import really persists the generated blob.
    prepare::import_prepared_tree(&repo, &prepared).unwrap();
    assert!(repo.odb().unwrap().exists(expected_blob));
}

#[test]
fn owner_loss_between_preparation_and_mutation_has_no_git_effect() {
    let (dir, repo, local, incoming) = fixture();
    let authority = Cell::new(true);
    let prepare::MergePreparation::Clean(prepared) =
        prepare::prepare_clean_merge(dir.path(), local, incoming).unwrap()
    else {
        panic!("clean merge expected")
    };
    let _lease = crate::repository::repository_lease(
        &repo,
        dir.path(),
        crate::repository::RepositoryOperation::RepositorySnapshot,
    )
    .unwrap();
    let before = objects(&repo);
    let index = std::fs::read(repo.path().join("index")).unwrap();
    let worktree = std::fs::read(dir.path().join("file.txt")).unwrap();
    authority.set(false);
    let owner = || {
        if authority.get() {
            Ok(())
        } else {
            Err(Error::ExternalChange)
        }
    };
    assert_eq!(
        prepare::import_owned_tree(&repo, &prepared, owner),
        Err(Error::ExternalChange)
    );
    assert_eq!(
        artifacts::fast_forward(&repo, "refs/heads/main", local, incoming, owner, || {}),
        Err(Error::ExternalChange)
    );
    assert_eq!(objects(&repo), before);
    assert_eq!(repo.refname_to_id("refs/heads/main").unwrap(), local);
    assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(
        std::fs::read(dir.path().join("file.txt")).unwrap(),
        worktree
    );
    assert!(!repo.path().join("refs/heads/main.lock").exists());
}

#[test]
fn mutable_path_preflight_refuses_unproven_paths_without_writes() {
    let (dir, repo, _, _) = fixture();
    let before = objects(&repo);
    let index = std::fs::read(repo.path().join("index")).unwrap();
    for path in ["../escape", "/absolute", "", ".git"] {
        assert!(
            resolution::conflict_worktree_digest(dir.path(), Path::new(path)).is_err(),
            "unsafe fixture path accepted: {path}"
        );
        assert!(resolution::resolution_preflight(&repo, &[(0, path.as_bytes())]).is_err());
    }
    // Absence is a valid preimage; token/canonical authorization belongs to the
    // adapter, not this filesystem-image reader.
    assert!(
        resolution::conflict_worktree_digest(dir.path(), Path::new("missing-parent/file")).is_ok()
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("file.txt", dir.path().join("alias")).unwrap();
        assert!(resolution::conflict_worktree_digest(dir.path(), Path::new("alias")).is_err());
        assert!(resolution::resolution_preflight(&repo, &[(0, b"alias")]).is_err());
    }
    assert_eq!(objects(&repo), before);
    assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), index);
}
