#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::{Config, Repository, RepositoryInitOptions, Signature, Time};

pub struct TestRepository {
    // Fields drop in declaration order, so the repository closes before TempDir removes it.
    pub repository: Repository,
    pub root: PathBuf,
    pub tempdir: tempfile::TempDir,
}

pub fn unborn_repository() -> TestRepository {
    let tempdir = tempfile::tempdir().unwrap();
    let root = tempdir.path().to_owned();
    let repository = Repository::init(&root).unwrap();

    TestRepository {
        repository,
        root,
        tempdir,
    }
}

pub struct FailOnce {
    point: manyhands::repository::FailurePoint,
}

impl FailOnce {
    pub fn at(point: manyhands::repository::FailurePoint) -> Self {
        Self { point }
    }

    pub fn open_service(self, data_directory: &Path) -> manyhands::repository::RepositoryService {
        manyhands::repository::RepositoryService::open_at_with_failure_point_for_testing(
            data_directory,
            self.point,
        )
        .unwrap()
    }
}

pub fn born_repository() -> TestRepository {
    let tempdir = tempfile::tempdir().unwrap();
    let root = tempdir.path().to_owned();
    let mut options = RepositoryInitOptions::new();
    options.initial_head("main");
    let repository = Repository::init_opts(&root, &options).unwrap();
    let mut config = Config::open(&repository.path().join("config")).unwrap();
    config.set_str("user.name", "Manyhands Test").unwrap();
    config
        .set_str("user.email", "manyhands-test@example.invalid")
        .unwrap();

    fs::write(root.join("fixture.txt"), "fixture\n").unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(Path::new("fixture.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(0, 0),
    )
    .unwrap();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial fixture commit",
            &tree,
            &[],
        )
        .unwrap();
    drop(tree);
    repository.index().unwrap().write().unwrap();

    TestRepository {
        repository,
        root,
        tempdir,
    }
}

pub fn bare_repository() -> TestRepository {
    let tempdir = tempfile::tempdir().unwrap();
    let root = tempdir.path().to_owned();
    let repository = Repository::init_bare(&root).unwrap();

    TestRepository {
        repository,
        root,
        tempdir,
    }
}

pub fn repository_without_local_identity() -> TestRepository {
    let repository = born_repository();
    let mut config = Config::open(&repository.repository.path().join("config")).unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    drop(config);

    repository
}

pub fn exclude_bytes(repository: &Repository) -> Option<Vec<u8>> {
    fs::read(repository.commondir().join("info/exclude")).ok()
}

pub fn tracked_configuration(root: &Path) -> Option<Vec<u8>> {
    fs::read(root.join(".manyhands/config.toml")).ok()
}

pub fn head_commit(repository: &Repository) -> Option<git2::Oid> {
    repository.head().ok().and_then(|head| head.target())
}

pub fn index_bytes(repository: &Repository) -> Option<Vec<u8>> {
    fs::read(repository.path().join("index")).ok()
}

pub fn conflict_primary_worktree(fixture: &TestRepository) {
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(0, 0),
    )
    .unwrap();
    let blob = fixture.repository.blob(b"other\n").unwrap();
    let mut builder = fixture
        .repository
        .treebuilder(Some(&head.tree().unwrap()))
        .unwrap();
    builder.insert("fixture.txt", blob, 0o100644).unwrap();
    let tree = fixture
        .repository
        .find_tree(builder.write().unwrap())
        .unwrap();
    fixture
        .repository
        .commit(
            Some("refs/heads/other"),
            &signature,
            &signature,
            "Other change",
            &tree,
            &[&head],
        )
        .unwrap();
    drop(tree);
    drop(head);

    fs::write(fixture.root.join("fixture.txt"), "local\n").unwrap();
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new("fixture.txt")).unwrap();
    let tree = fixture
        .repository
        .find_tree(index.write_tree().unwrap())
        .unwrap();
    let parent = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture
        .repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Local change",
            &tree,
            &[&parent],
        )
        .unwrap();
    drop(tree);
    drop(parent);

    let other = fixture
        .repository
        .find_reference("refs/heads/other")
        .unwrap();
    let annotated = fixture
        .repository
        .reference_to_annotated_commit(&other)
        .unwrap();
    fixture.repository.merge(&[&annotated], None, None).unwrap();
}

pub fn commit_tracked_configuration(fixture: &TestRepository, source: &str) {
    let path = fixture.root.join(".manyhands/config.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, source).unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(0, 0),
    )
    .unwrap();
    let mut index = fixture.repository.index().unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    let tree = fixture
        .repository
        .find_tree(index.write_tree().unwrap())
        .unwrap();
    let parent = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture
        .repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Existing configuration",
            &tree,
            &[&parent],
        )
        .unwrap();
    index.write().unwrap();
}

pub fn config_source() -> String {
    "format_version = 1\nprimary_branch = \"main\"\n".to_owned()
}

pub fn document_source() -> String {
    "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Fixture document\n---\n"
        .to_owned()
}

pub fn ticket_source() -> String {
    "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAW\ntitle: Fixture ticket\ntype: task\nstatus: open\n---\n"
        .to_owned()
}

pub fn root_comment_source() -> String {
    "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAX\nitem_id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ncreated_at: 2026-09-30T12:00:00Z\n---\n"
        .to_owned()
}

pub fn reply_source() -> String {
    "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAY\nitem_id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\nparent_id: 01ARZ3NDEKTSV4RRFFQ69G5FAX\ncreated_at: 2026-09-30T12:01:00Z\n---\n"
        .to_owned()
}
