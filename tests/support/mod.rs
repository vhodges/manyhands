#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use git2::{Config, Repository, RepositoryInitOptions, Signature, StatusOptions, Time};
use manyhands::{
    canonical::ItemId,
    repository::{
        EnableRepositoryOutcome, EnableRepositoryRequest, OperationId, RepositoryService,
    },
};

pub struct TestRepository {
    // Fields drop in declaration order, so the repository closes before TempDir removes it.
    pub repository: Repository,
    pub root: PathBuf,
    pub tempdir: tempfile::TempDir,
}

pub struct EnabledRepository {
    pub service: RepositoryService,
    pub data_directory: tempfile::TempDir,
}

pub struct LinkedWorktree {
    pub worktree: PathBuf,
    pub head_branch: String,
    pub head_commit: git2::Oid,
    pub index: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepositoryAndWorktreeSnapshot {
    pub root: WorktreeState,
    pub linked_worktrees: BTreeMap<String, WorktreeState>,
    pub references: BTreeMap<String, String>,
    pub worktrees: BTreeMap<String, PathBuf>,
    pub remotes: BTreeMap<String, (Option<String>, Option<String>)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WorktreeState {
    pub files: BTreeMap<PathBuf, FilesystemEntry>,
    pub config: Option<Vec<u8>>,
    pub head: HeadState,
    pub index: Option<Vec<u8>>,
    pub statuses: BTreeMap<PathBuf, u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FilesystemEntry {
    File { bytes: Vec<u8> },
    Symlink { target: PathBuf },
}

#[derive(Debug, PartialEq, Eq)]
pub enum HeadState {
    Attached {
        symbolic_target: String,
        target: git2::Oid,
    },
    Detached {
        target: git2::Oid,
    },
    Unborn {
        symbolic_target: Option<String>,
    },
    Unavailable,
}

pub fn repository_and_worktree_snapshot(fixture: &TestRepository) -> RepositoryAndWorktreeSnapshot {
    repository_and_worktree_snapshot_at(&fixture.root)
}

pub fn repository_and_worktree_snapshot_at(root: &Path) -> RepositoryAndWorktreeSnapshot {
    let repository = Repository::open(root).unwrap();
    let linked_worktrees = repository
        .worktrees()
        .unwrap()
        .iter()
        .flatten()
        .map(|name| {
            let worktree = repository.find_worktree(name).unwrap();
            let path = worktree.path().canonicalize().unwrap();
            (
                name.to_owned(),
                worktree_state(&Repository::open(&path).unwrap(), &path),
            )
        })
        .collect();
    let references = repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            let name = reference.name().unwrap().to_owned();
            let target = reference
                .symbolic_target()
                .map(str::to_owned)
                .or_else(|| reference.target().map(|oid| oid.to_string()))
                .unwrap_or_default();
            (name, target)
        })
        .collect();
    let worktrees = repository
        .worktrees()
        .unwrap()
        .iter()
        .flatten()
        .map(|name| {
            let worktree = repository.find_worktree(name).unwrap();
            (name.to_owned(), worktree.path().canonicalize().unwrap())
        })
        .collect();
    let remotes = repository
        .remotes()
        .unwrap()
        .iter()
        .flatten()
        .map(|name| {
            let remote = repository.find_remote(name).unwrap();
            (
                name.to_owned(),
                (
                    remote.url().map(str::to_owned),
                    remote.pushurl().map(str::to_owned),
                ),
            )
        })
        .collect();

    RepositoryAndWorktreeSnapshot {
        root: worktree_state(&repository, root),
        linked_worktrees,
        references,
        worktrees,
        remotes,
    }
}

fn worktree_state(repository: &Repository, worktree: &Path) -> WorktreeState {
    let mut statuses = StatusOptions::new();
    statuses
        .include_untracked(true)
        .include_ignored(true)
        .recurse_untracked_dirs(true);
    WorktreeState {
        files: canonical_file_bytes(worktree, worktree),
        config: fs::read(repository.path().join("config")).ok(),
        head: head_state(repository),
        index: index_bytes(repository),
        statuses: repository
            .statuses(Some(&mut statuses))
            .unwrap()
            .iter()
            .filter_map(|entry| {
                entry
                    .path()
                    .map(|path| (PathBuf::from(path), entry.status().bits()))
            })
            .collect(),
    }
}

fn head_state(repository: &Repository) -> HeadState {
    let Ok(head) = repository.find_reference("HEAD") else {
        return HeadState::Unavailable;
    };
    if let Some(symbolic_target) = head.symbolic_target() {
        return match repository.refname_to_id(symbolic_target) {
            Ok(target) => HeadState::Attached {
                symbolic_target: symbolic_target.to_owned(),
                target,
            },
            Err(_) => HeadState::Unborn {
                symbolic_target: Some(symbolic_target.to_owned()),
            },
        };
    }
    match head.target() {
        Some(target) => HeadState::Detached { target },
        None => HeadState::Unborn {
            symbolic_target: None,
        },
    }
}

fn canonical_file_bytes(root: &Path, directory: &Path) -> BTreeMap<PathBuf, FilesystemEntry> {
    let mut files = BTreeMap::new();
    collect_canonical_file_bytes(root, directory, &mut files);
    files
}

fn collect_canonical_file_bytes(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<PathBuf, FilesystemEntry>,
) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap();
        if relative == Path::new(".git") || relative == Path::new(".manyhands/worktrees") {
            continue;
        }
        let file_type = entry.file_type().unwrap();
        if file_type.is_symlink() {
            files.insert(
                relative.to_owned(),
                FilesystemEntry::Symlink {
                    target: fs::read_link(path).unwrap(),
                },
            );
        } else if file_type.is_dir() {
            collect_canonical_file_bytes(root, &path, files);
        } else if file_type.is_file() {
            files.insert(
                relative.to_owned(),
                FilesystemEntry::File {
                    bytes: fs::read(path).unwrap(),
                },
            );
        }
    }
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

pub fn enabled_repository(fixture: &TestRepository) -> EnabledRepository {
    let data_directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    assert!(matches!(
        service
            .enable(EnableRepositoryRequest {
                root: fixture.root.clone(),
                primary_branch: "main".to_owned(),
                identity: None,
                operation_id: operation_id(),
            })
            .unwrap(),
        EnableRepositoryOutcome::Enabled { .. },
    ));

    EnabledRepository {
        service,
        data_directory,
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

pub fn commit_tree_path(
    repository: &Repository,
    commit_oid: git2::Oid,
    path: impl AsRef<Path>,
) -> Option<Vec<u8>> {
    let commit = repository.find_commit(commit_oid).ok()?;
    let tree = commit.tree().ok()?;
    let entry = tree.get_path(path.as_ref()).ok()?;
    repository
        .find_blob(entry.id())
        .ok()
        .map(|blob| blob.content().to_vec())
}

pub fn open_linked_worktree(path: &Path) -> LinkedWorktree {
    let repository = Repository::open(path).unwrap();
    let head = repository.head().unwrap();
    let head_branch = head
        .name()
        .and_then(|name| name.strip_prefix("refs/heads/"))
        .unwrap()
        .to_owned();

    LinkedWorktree {
        worktree: repository.workdir().unwrap().to_owned(),
        head_branch,
        head_commit: head.target().unwrap(),
        index: index_bytes(&repository).unwrap(),
    }
}

pub fn document_id() -> ItemId {
    ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap()
}

pub fn operation_id() -> OperationId {
    OperationId::new()
}

pub fn parse_operation_id(value: &str) -> OperationId {
    OperationId::parse(value).unwrap()
}

pub fn ticket_id() -> ItemId {
    ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAW").unwrap()
}

pub fn root_comment_id() -> ItemId {
    ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAX").unwrap()
}

pub fn reply_id() -> ItemId {
    ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAY").unwrap()
}

pub fn write_document_source(root: &Path, path: impl AsRef<Path>) -> PathBuf {
    write_source(root, path.as_ref(), &document_source())
}

pub fn write_ticket_source(root: &Path, path: impl AsRef<Path>) -> PathBuf {
    write_source(root, path.as_ref(), &ticket_source())
}

fn write_source(root: &Path, path: &Path, source: &str) -> PathBuf {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, source).unwrap();
    path
}

pub fn conflict_primary_worktree(fixture: &TestRepository) {
    conflict_worktree(&fixture.repository, &fixture.root);
}

pub fn conflict_worktree(repository: &Repository, root: &Path) {
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(0, 0),
    )
    .unwrap();
    let blob = repository.blob(b"other\n").unwrap();
    let mut builder = repository.treebuilder(Some(&head.tree().unwrap())).unwrap();
    builder.insert("fixture.txt", blob, 0o100644).unwrap();
    let tree = repository.find_tree(builder.write().unwrap()).unwrap();
    repository
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

    fs::write(root.join("fixture.txt"), "local\n").unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(Path::new("fixture.txt")).unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repository.head().unwrap().peel_to_commit().unwrap();
    repository
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

    let other = repository.find_reference("refs/heads/other").unwrap();
    let annotated = repository.reference_to_annotated_commit(&other).unwrap();
    repository.merge(&[&annotated], None, None).unwrap();
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
