#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    str::FromStr,
    time::{Duration, Instant},
};

use git2::{Config, Repository, RepositoryInitOptions, Signature, StatusOptions, Time};
use manyhands::{
    canonical::ItemId,
    repository::{
        EnableRepositoryOutcome, EnableRepositoryRequest, OperationId, RepositoryService,
    },
};
use rusqlite::{Connection, params};

pub mod credentials;
pub mod golden;
pub mod items;
pub mod mutation;
#[cfg(target_os = "linux")]
pub mod open_watch;
pub mod operations;
pub mod schema;

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

/// Includes objects, FETCH_HEAD, reflogs, and linked-worktree administrative files.
pub fn repository_git_file_bytes(fixture: &TestRepository) -> BTreeMap<PathBuf, FilesystemEntry> {
    canonical_file_bytes(fixture.repository.path(), fixture.repository.path())
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
    // Source fixtures declare LF checkout policy independently of the user's Git config.
    Config::open(&repository.path().join("config"))
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();

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
    // Set this before staging initial content; individual filter controls may override it locally.
    config.set_bool("core.autocrlf", false).unwrap();
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

pub fn new_operation_id() -> OperationId {
    operation_id()
}

pub struct LeaseHolder {
    child: Child,
    release: PathBuf,
    _synchronization: tempfile::TempDir,
}

impl LeaseHolder {
    pub fn release(mut self) {
        fs::write(&self.release, b"release").unwrap();
        assert!(self.child.wait().unwrap().success());
    }

    pub fn terminate(mut self) {
        self.child.kill().unwrap();
        assert!(!self.child.wait().unwrap().success());
    }
}

impl Drop for LeaseHolder {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            let _ = fs::write(&self.release, b"release");
            let _ = self.child.wait();
        }
    }
}

pub fn hold_lease_in_child(
    root: &Path,
    data_directory: &Path,
    kind: manyhands::repository::LeaseKind,
) -> LeaseHolder {
    hold_lease_in_child_for_test(root, data_directory, kind, "common_git_lease_child")
}

pub fn hold_lease_in_child_for_test(
    root: &Path,
    data_directory: &Path,
    kind: manyhands::repository::LeaseKind,
    child_test: &str,
) -> LeaseHolder {
    let synchronization = tempfile::tempdir().unwrap();
    let ready = synchronization.path().join("ready");
    let release = synchronization.path().join("release");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(child_test)
        .arg("--nocapture")
        .env("MANYHANDS_LEASE_ROOT", root)
        .env("MANYHANDS_LEASE_DATA_DIRECTORY", data_directory)
        .env("MANYHANDS_LEASE_KIND", kind.as_str())
        .env("MANYHANDS_LEASE_READY", &ready)
        .env("MANYHANDS_LEASE_RELEASE", &release)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_path(&mut child, &ready);
    LeaseHolder {
        child,
        release,
        _synchronization: synchronization,
    }
}

pub fn wait_for_path(child: &mut Child, path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            let stdout = child
                .stdout
                .take()
                .map(read_child_output)
                .unwrap_or_default();
            let stderr = child
                .stderr
                .take()
                .map(read_child_output)
                .unwrap_or_default();
            panic!(
                "lease holder child exited before ready ({status}):\nstdout:\n{stdout}\nstderr:\n{stderr}"
            );
        }
        assert!(Instant::now() < deadline, "timed out waiting for {path:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn read_child_output(mut output: impl Read) -> String {
    let mut bytes = Vec::new();
    let _ = output.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn enable_request(root: &Path) -> EnableRepositoryRequest {
    enable_request_with_operation_id(root, operation_id())
}

pub fn enable_request_with_operation_id(
    root: &Path,
    operation_id: OperationId,
) -> EnableRepositoryRequest {
    EnableRepositoryRequest {
        root: root.to_owned(),
        primary_branch: "main".to_owned(),
        identity: None,
        operation_id,
    }
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

pub fn create_cycle_04_registry(data_directory: &Path, root: &Path, operation: &str, state: &str) {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE repositories (
            id INTEGER PRIMARY KEY,
            root_path TEXT NOT NULL UNIQUE,
            enabled_at INTEGER NOT NULL,
            accessibility TEXT NOT NULL,
            config_blob_oid TEXT,
            refresh_required INTEGER NOT NULL
        );
        CREATE TABLE index_operations (
            id INTEGER PRIMARY KEY,
            repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            operation TEXT NOT NULL,
            state TEXT NOT NULL DEFAULT 'completed',
            context_path TEXT,
            persisted_context_count INTEGER NOT NULL DEFAULT 0,
            observed_at INTEGER NOT NULL
        );
        CREATE TABLE index_operation_contexts (
            operation_id INTEGER NOT NULL REFERENCES index_operations(id) ON DELETE CASCADE,
            worktree_path TEXT NOT NULL,
            observation_fingerprint TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (operation_id, worktree_path)
        );",
        )
        .unwrap();
    let root = root.canonicalize().unwrap();
    connection.execute(
        "INSERT INTO repositories (root_path, enabled_at, accessibility, config_blob_oid, refresh_required)
         VALUES (?1, 1, 'accessible', NULL, 1)",
        [root.to_str().unwrap()],
    ).unwrap();
    let repository_id = connection.last_insert_rowid();
    connection.execute(
        "INSERT INTO index_operations (repository_id, operation, state, context_path, persisted_context_count, observed_at)
         VALUES (?1, ?2, ?3, ?4, 2, 1)",
        params![repository_id, operation, state, root.to_str().unwrap()],
    ).unwrap();
    let operation_id = connection.last_insert_rowid();
    connection.execute(
        "INSERT INTO index_operation_contexts (operation_id, worktree_path, observation_fingerprint)
          VALUES (?1, ?2, 'legacy-fingerprint-BLAKE3-PRIVATE-MARKDOWN')",
        params![operation_id, root.to_str().unwrap()],
    ).unwrap();
    connection.execute(
        "INSERT INTO index_operation_contexts (operation_id, worktree_path, observation_fingerprint)
          VALUES (?1, ?2, 'another-legacy-fingerprint-GIT-BLOB-OID')",
        params![operation_id, root.join(".manyhands/worktrees/second").to_str().unwrap()],
    ).unwrap();
    connection
        .execute(
            "INSERT INTO index_operations (repository_id, operation, state, observed_at)
         VALUES (?1, 'rebuild', 'completed', 2)",
            [repository_id],
        )
        .unwrap();
}

pub fn assert_operation_records_hold_no_content(data_directory: &Path) {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    let schema: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'operation_records'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!schema.to_ascii_lowercase().contains("digest"));
    assert!(!schema.to_ascii_lowercase().contains("fingerprint"));
    assert!(!schema.to_ascii_lowercase().contains("blake3"));
    assert!(!schema.to_ascii_lowercase().contains("oid"));
    assert!(!schema.to_ascii_lowercase().contains("content"));
    assert!(!schema.to_ascii_lowercase().contains("draft"));
}

pub fn assert_operation_records_exclude(data_directory: &Path, forbidden: &[&str]) {
    assert_operation_records_hold_no_content(data_directory);
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT COALESCE(operation_ulid, ''), action, target, state, completed_step, \
                    COALESCE(item_id, ''), COALESCE(context_path, ''), \
                    CAST(observed_at AS TEXT), CAST(persisted_context_count AS TEXT), \
                    COALESCE(redacted_error, '') \
             FROM operation_records",
        )
        .unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((0..10)
                .map(|column| row.get::<_, String>(column))
                .collect::<Result<Vec<_>, _>>()?
                .join("\n"))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for value in rows {
        for forbidden in forbidden {
            assert!(
                !value.contains(forbidden),
                "operation record retained forbidden value {forbidden:?}"
            );
        }
        assert!(!value.to_ascii_lowercase().contains("blake3"));
        assert!(!value.to_ascii_lowercase().contains("credential"));
        assert!(!value.to_ascii_lowercase().contains("password"));
        assert!(!value.to_ascii_lowercase().contains("private key"));
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RegistryCacheState {
    pub repositories: i64,
    pub contexts: i64,
    pub items: i64,
    pub comments: i64,
    pub problems: i64,
    pub operations: Vec<(String, String)>,
}

pub fn registry_cache_state(data_directory: &Path) -> RegistryCacheState {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    let count = |table: &str| {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    };
    let operations = connection
        .prepare("SELECT action, state FROM operation_records ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    RegistryCacheState {
        repositories: count("repositories"),
        contexts: count("contexts"),
        items: count("discovered_items"),
        comments: count("discovered_comments"),
        problems: count("problems"),
        operations,
    }
}

pub fn assert_corrupt_diagnostics(
    data_directory: &Path,
    expected: &[(&str, &[u8])],
    forbidden: &[&str],
) {
    let prefix = format!("{}.corrupt-", manyhands::repository::REGISTRY_FILE);
    let diagnostics = fs::read_dir(data_directory)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), expected.len());
    for (suffix, bytes) in expected {
        let artifact = diagnostics.iter().find(|(name, _)| {
            if suffix.is_empty() {
                name.starts_with(&prefix) && !name.ends_with("-wal") && !name.ends_with("-shm")
            } else {
                name.ends_with(suffix)
            }
        });
        assert_eq!(artifact.map(|(_, bytes)| bytes.as_slice()), Some(*bytes));
    }
    for (_, bytes) in diagnostics {
        assert_diagnostic_bytes_exclude(&bytes, forbidden);
    }
}

pub fn assert_diagnostic_output_excludes(values: &[String], forbidden: &[&str]) {
    for value in values {
        assert_diagnostic_bytes_exclude(value.as_bytes(), forbidden);
    }
}

fn assert_diagnostic_bytes_exclude(bytes: &[u8], forbidden: &[&str]) {
    let value = String::from_utf8_lossy(bytes);
    for forbidden in forbidden {
        assert!(
            !value.contains(forbidden),
            "diagnostic retained forbidden value {forbidden:?}"
        );
    }
}

pub fn assert_legacy_operation_records_are_redacted_and_reset(data_directory: &Path) {
    let connection =
        Connection::open(data_directory.join(manyhands::repository::REGISTRY_FILE)).unwrap();
    assert_operation_records_hold_no_content(data_directory);
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM operation_records", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT state, context_path, persisted_context_count
                 FROM operation_records WHERE state != 'completed'",
                [],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?
                )),
            )
            .unwrap(),
        ("created".to_owned(), None, 0)
    );
    let root: String = connection
        .query_row(
            "SELECT root_path FROM operation_records WHERE state != 'completed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let contexts = connection
        .prepare(
            "SELECT operation_record_contexts.worktree_path
             FROM operation_record_contexts
             JOIN operation_records ON operation_records.id = operation_record_contexts.operation_record_id
             WHERE operation_records.state != 'completed'
             ORDER BY operation_record_contexts.worktree_path",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        contexts,
        vec![
            root.clone(),
            PathBuf::from(&root)
                .join(".manyhands")
                .join("worktrees")
                .join("second")
                .to_str()
                .unwrap()
                .to_owned(),
        ]
    );
    let context_schema: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'operation_record_contexts'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!context_schema.to_ascii_lowercase().contains("fingerprint"));
    assert!(!context_schema.to_ascii_lowercase().contains("digest"));
    for table in ["index_operations", "index_operation_contexts"] {
        assert!(
            !connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap(),
            "legacy table {table} remains after migration"
        );
    }
}
