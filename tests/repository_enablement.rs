use git2::{BranchType, Config, ConfigLevel, Repository};
use manyhands::{
    canonical,
    repository::{
        AddRemoteRequest, CommitIdentity, ConfigurationInspection, CreateRepositoryRequest,
        EnableRepositoryOutcome, EnableRepositoryRequest, FailurePoint, IdentityInspection,
        PublicationRemoteOutcome, REGISTRY_FILE, RegistryConnectionPhase, RemoteOutcome,
        RemoveRegistrationOutcome, RepositoryErrorKind, RepositoryOperation, RepositoryService,
        SetPublicationRemoteRequest,
    },
};
use rusqlite::Connection;
use std::{sync::mpsc, thread, time::Duration};

mod support;

fn isolated_effective_config(source: &str) -> (tempfile::TempDir, Config) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("global.gitconfig");
    std::fs::write(&path, source).unwrap();
    let mut config = Config::new().unwrap();
    config.add_file(&path, ConfigLevel::Global, false).unwrap();
    (directory, config)
}

fn repository_snapshot(repository: &Repository, root: &std::path::Path) -> RepositorySnapshot {
    let mut references = repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            (
                reference.name().unwrap().to_owned(),
                reference.symbolic_target().map(str::to_owned),
                reference.target(),
            )
        })
        .collect::<Vec<_>>();
    references.sort_by(|left, right| left.0.cmp(&right.0));
    let head = repository.find_reference("HEAD").unwrap();

    RepositorySnapshot {
        head: (
            head.name().unwrap().to_owned(),
            head.symbolic_target().map(str::to_owned),
            head.target(),
        ),
        references,
        common_config: std::fs::read(repository.commondir().join("config")).unwrap(),
        exclude: std::fs::read(repository.commondir().join("info/exclude")).ok(),
        fixture: std::fs::read(root.join("fixture.txt")).ok(),
        statuses: repository
            .statuses(None)
            .unwrap()
            .iter()
            .map(|entry| (entry.path().unwrap().to_owned(), entry.status().bits()))
            .collect(),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RepositorySnapshot {
    head: (String, Option<String>, Option<git2::Oid>),
    references: Vec<(String, Option<String>, Option<git2::Oid>)>,
    common_config: Vec<u8>,
    exclude: Option<Vec<u8>>,
    fixture: Option<Vec<u8>>,
    statuses: Vec<(String, u32)>,
}

fn registry_row_count(data: &std::path::Path) -> i64 {
    Connection::open(data.join(REGISTRY_FILE))
        .unwrap()
        .query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))
        .unwrap()
}

fn registry_row(
    data: &std::path::Path,
    root: &std::path::Path,
) -> Option<(String, String, i64, i64)> {
    Connection::open(data.join(REGISTRY_FILE))
        .unwrap()
        .query_row(
            "SELECT accessibility, config_blob_oid, refresh_required, enabled_at
             FROM repositories WHERE root_path = ?1",
            [root.to_str().unwrap()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok()
}

fn enable_request(root: &std::path::Path, primary_branch: &str) -> EnableRepositoryRequest {
    EnableRepositoryRequest {
        root: root.to_owned(),
        primary_branch: primary_branch.to_owned(),
        identity: None,
    }
}

fn create_request(root: &std::path::Path, primary_branch: &str) -> CreateRepositoryRequest {
    CreateRepositoryRequest {
        root: root.to_owned(),
        primary_branch: primary_branch.to_owned(),
        identity: Some(CommitIdentity {
            name: "Created Author".to_owned(),
            email: "created@example.invalid".to_owned(),
        }),
    }
}

fn add_remote_request(root: &std::path::Path, name: &str, url: &str) -> AddRemoteRequest {
    AddRemoteRequest {
        root: root.to_owned(),
        name: name.to_owned(),
        url: url.to_owned(),
    }
}

fn publication_request(root: &std::path::Path, name: Option<&str>) -> SetPublicationRemoteRequest {
    SetPublicationRemoteRequest {
        root: root.to_owned(),
        name: name.map(str::to_owned),
    }
}

fn stage_configuration(fixture: &support::TestRepository) {
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    index.write().unwrap();
}

fn commit_count(repository: &Repository) -> usize {
    let mut walk = repository.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.count()
}

fn head_configuration(repository: &Repository) -> Vec<u8> {
    let tree = repository.head().unwrap().peel_to_tree().unwrap();
    let entry = tree
        .get_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    repository.find_blob(entry.id()).unwrap().content().to_vec()
}

fn head_configuration_oid(repository: &Repository) -> git2::Oid {
    let tree = repository.head().unwrap().peel_to_tree().unwrap();
    tree.get_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap()
        .id()
}

fn remote_names(repository: &Repository) -> Vec<String> {
    let mut names = repository
        .remotes()
        .unwrap()
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn assert_registry_matches_head(data: &std::path::Path, fixture: &support::TestRepository) {
    let row = registry_row(data, &fixture.root).unwrap();
    assert_eq!(
        row.1,
        head_configuration_oid(&fixture.repository).to_string()
    );
}

fn failing_service(data: &std::path::Path, point: FailurePoint) -> RepositoryService {
    support::FailOnce::at(point).open_service(data)
}

#[test]
fn create_nonexistent_project_initializes_enables_and_registers_main() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let root = parent.path().join("project");

    let outcome = service
        .create_and_enable(create_request(&root, "main"))
        .unwrap();

    let EnableRepositoryOutcome::Enabled { commit_oid } = outcome else {
        panic!("expected an initialization commit");
    };
    let repository = Repository::open(&root).unwrap();
    assert_eq!(repository.head().unwrap().shorthand(), Some("main"));
    assert_eq!(support::head_commit(&repository), Some(commit_oid));
    assert_eq!(
        repository
            .config()
            .unwrap()
            .get_string("user.name")
            .unwrap(),
        "Created Author"
    );
    assert_eq!(
        repository
            .config()
            .unwrap()
            .get_string("user.email")
            .unwrap(),
        "created@example.invalid"
    );
    assert_eq!(
        repository.find_commit(commit_oid).unwrap().message(),
        Some("Initialize Manyhands")
    );
    assert_eq!(registry_row_count(data.path()), 1);
}

#[test]
fn create_existing_empty_project_initializes_enables_and_registers_main() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let root = parent.path().join("project");
    std::fs::create_dir(&root).unwrap();

    let outcome = service
        .create_and_enable(create_request(&root, "main"))
        .unwrap();

    assert!(matches!(outcome, EnableRepositoryOutcome::Enabled { .. }));
    let repository = Repository::open(&root).unwrap();
    assert_eq!(repository.head().unwrap().shorthand(), Some("main"));
    assert_eq!(
        Config::open(&repository.path().join("config"))
            .unwrap()
            .get_string("user.name")
            .unwrap(),
        "Created Author"
    );
    assert_eq!(
        repository
            .find_commit(repository.head().unwrap().target().unwrap())
            .unwrap()
            .message(),
        Some("Initialize Manyhands")
    );
    assert_eq!(registry_row_count(data.path()), 1);
}

#[test]
fn create_rejects_invalid_requests_without_creating_a_repository_or_target() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let nonempty = parent.path().join("nonempty");
    std::fs::create_dir(&nonempty).unwrap();
    std::fs::write(nonempty.join("file"), "content").unwrap();
    let missing_parent_target = parent.path().join("missing").join("project");
    let file_target = parent.path().join("file");
    std::fs::write(&file_target, "file\n").unwrap();
    let invalid_branch_target = parent.path().join("invalid-branch");
    let identity_required_target = parent.path().join("identity-required");

    let nonempty_error = service
        .create_and_enable(create_request(&nonempty, "main"))
        .unwrap_err();
    let missing_parent_error = service
        .create_and_enable(create_request(&missing_parent_target, "main"))
        .unwrap_err();
    let file_error = service
        .create_and_enable(create_request(&file_target, "main"))
        .unwrap_err();
    let invalid_branch_error = service
        .create_and_enable(create_request(&invalid_branch_target, "main..bad"))
        .unwrap_err();
    let identity_required = service
        .create_and_enable(CreateRepositoryRequest {
            root: identity_required_target.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
        })
        .unwrap();

    assert_eq!(
        nonempty_error.operation,
        RepositoryOperation::CreateAndEnable
    );
    assert_eq!(nonempty_error.kind, RepositoryErrorKind::InvalidPath);
    assert_eq!(
        missing_parent_error.operation,
        RepositoryOperation::CreateAndEnable
    );
    assert_eq!(missing_parent_error.kind, RepositoryErrorKind::InvalidPath);
    assert_eq!(file_error.operation, RepositoryOperation::CreateAndEnable);
    assert_eq!(file_error.kind, RepositoryErrorKind::InvalidPath);
    assert_eq!(
        invalid_branch_error.operation,
        RepositoryOperation::CreateAndEnable
    );
    assert_eq!(
        invalid_branch_error.kind,
        RepositoryErrorKind::InvalidConfiguration
    );
    assert_eq!(identity_required, EnableRepositoryOutcome::IdentityRequired);
    assert!(!missing_parent_target.exists());
    assert!(!invalid_branch_target.exists());
    assert!(!identity_required_target.exists());
    assert!(nonempty.join("file").is_file());
    assert!(!nonempty.join(".git").exists());
    assert!(file_target.is_file());
    assert_eq!(registry_row_count(data.path()), 0);
}

#[test]
fn create_preinitialization_failure_removes_only_its_own_empty_target() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let created_root = parent.path().join("created");
    let existing_root = parent.path().join("existing");
    std::fs::create_dir(&existing_root).unwrap();

    for root in [&created_root, &existing_root] {
        let service = failing_service(data.path(), FailurePoint::BeforeRepositoryInitialization);

        let error = service
            .create_and_enable(create_request(root, "main"))
            .unwrap_err();

        assert_eq!(error.operation, RepositoryOperation::CreateAndEnable);
        assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    }
    assert!(!created_root.exists());
    assert!(existing_root.is_dir());
    assert!(std::fs::read_dir(&existing_root).unwrap().next().is_none());
    assert_eq!(registry_row_count(data.path()), 0);
}

#[test]
fn service_creates_its_local_registry_in_the_supplied_data_directory() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();

    assert!(data.path().join(REGISTRY_FILE).is_file());
    assert!(!data.path().join(".git").exists());
    #[allow(clippy::drop_non_drop)] // Retained to make future service cleanup explicit.
    drop(service);
}

#[test]
fn service_reports_an_io_error_when_its_data_directory_is_a_file() {
    let data_file = tempfile::NamedTempFile::new().unwrap();

    let error = match RepositoryService::open_at(data_file.path()) {
        Ok(_) => panic!("a regular file cannot be a data directory"),
        Err(error) => error,
    };

    assert_eq!(error.operation, RepositoryOperation::OpenRegistry);
    assert_eq!(error.kind, RepositoryErrorKind::Io);
}

#[test]
fn service_reports_a_sqlite_error_when_its_registry_path_is_a_directory() {
    let data = tempfile::tempdir().unwrap();
    std::fs::create_dir(data.path().join(REGISTRY_FILE)).unwrap();

    let error = match RepositoryService::open_at(data.path()) {
        Ok(_) => panic!("a directory cannot be a SQLite database"),
        Err(error) => error,
    };

    assert_eq!(error.operation, RepositoryOperation::OpenRegistry);
    assert_eq!(error.kind, RepositoryErrorKind::Sqlite);
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn registry_creates_repository_and_discovery_metadata_schema() {
    let data = tempfile::tempdir().unwrap();
    RepositoryService::open_at(data.path()).unwrap();

    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let tables = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let columns = connection
        .prepare("PRAGMA table_info(repositories)")
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let root_path_is_unique = connection
        .prepare("PRAGMA index_list(repositories)")
        .unwrap()
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })
        .unwrap()
        .filter_map(Result::ok)
        .filter(|(_, unique)| *unique == 1)
        .any(|(index, _)| {
            connection
                .prepare(&format!("PRAGMA index_info({index})"))
                .unwrap()
                .query_row([], |row| row.get::<_, String>(2))
                .unwrap()
                == "root_path"
        });

    assert_eq!(
        tables,
        [
            "configuration_observations",
            "contexts",
            "discovered_comments",
            "discovered_items",
            "index_operations",
            "problems",
            "repositories",
        ]
    );
    assert_eq!(
        columns,
        [
            ("id".to_owned(), "INTEGER".to_owned(), 0, 1),
            ("root_path".to_owned(), "TEXT".to_owned(), 1, 0),
            ("enabled_at".to_owned(), "INTEGER".to_owned(), 1, 0),
            ("accessibility".to_owned(), "TEXT".to_owned(), 1, 0),
            ("config_blob_oid".to_owned(), "TEXT".to_owned(), 0, 0),
            ("refresh_required".to_owned(), "INTEGER".to_owned(), 1, 0),
        ]
    );
    assert!(root_path_is_unique);

    for (root_path, refresh_required) in [("/repository/zero", 0), ("/repository/one", 1)] {
        connection
            .execute(
                "INSERT INTO repositories (
                    root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                ) VALUES (?1, 1, 'accessible', 'blob', ?2)",
                (root_path, refresh_required),
            )
            .unwrap();
    }
    for (root_path, refresh_required) in [("/repository/two", 2), ("/repository/negative", -1)] {
        assert!(
            connection
                .execute(
                    "INSERT INTO repositories (
                        root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                    ) VALUES (?1, 1, 'accessible', 'blob', ?2)",
                    (root_path, refresh_required),
                )
                .is_err()
        );
    }
}

#[test]
fn registry_migration_is_idempotent() {
    let data = tempfile::tempdir().unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let schema_before: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'repositories'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);

    RepositoryService::open_at(data.path()).unwrap();

    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    let schema_after: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'repositories'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(schema_after, schema_before);
}

#[test]
fn registry_open_waits_for_a_brief_database_lock() {
    const PHASE_TIMEOUT: Duration = Duration::from_secs(1);
    const LOCKED_WINDOW: Duration = Duration::from_millis(25);

    let data = tempfile::tempdir().unwrap();
    RepositoryService::open_at(data.path()).unwrap();
    let registry_path = data.path().join(REGISTRY_FILE);
    let lock_connection = Connection::open(&registry_path).unwrap();
    lock_connection
        .pragma_update(None, "journal_mode", "DELETE")
        .unwrap();
    lock_connection.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let (phases_sent, phases_received) = mpsc::channel();
    let (result_sent, result_received) = mpsc::sync_channel(1);
    let worker_data_directory = data.path().to_path_buf();

    let open_thread = thread::spawn(move || {
        let result = RepositoryService::open_at_with_registry_phase_observer(
            &worker_data_directory,
            |phase| {
                let _ = phases_sent.send(phase);
            },
        );
        let _ = result_sent.send(result);
    });

    let before_wal = phases_received.recv_timeout(PHASE_TIMEOUT);
    let while_locked = phases_received.recv_timeout(LOCKED_WINDOW);
    let lock_released = lock_connection.execute_batch("COMMIT");
    drop(lock_connection);

    let after_wal = phases_received.recv_timeout(PHASE_TIMEOUT);
    let worker_result = result_received.recv_timeout(PHASE_TIMEOUT);
    let worker_joined = open_thread.join();

    lock_released.unwrap();
    assert_eq!(before_wal.unwrap(), RegistryConnectionPhase::BeforeWal);
    assert!(matches!(while_locked, Err(mpsc::RecvTimeoutError::Timeout)));
    assert_eq!(after_wal.unwrap(), RegistryConnectionPhase::AfterWal);
    worker_joined.unwrap();
    worker_result.unwrap().unwrap();
}

#[test]
fn registry_uses_wal_and_configures_connection_local_safety_pragmas() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();

    let journal_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    let (foreign_keys, busy_timeout) = service
        .with_registry_connection_for_testing(|connection| {
            let foreign_keys = connection
                .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
                .unwrap();
            let busy_timeout = connection
                .pragma_query_value(None, "busy_timeout", |row| row.get::<_, i64>(0))
                .unwrap();
            (foreign_keys, busy_timeout)
        })
        .unwrap();

    assert_eq!(journal_mode, "wal");
    assert_eq!(foreign_keys, 1);
    assert!((1..=5_000).contains(&busy_timeout));
}

#[test]
fn inspect_born_repository_reports_its_local_state() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();

    let inspection = service.inspect(&fixture.root).unwrap();

    assert_eq!(
        inspection.root,
        std::fs::canonicalize(&fixture.root).unwrap()
    );
    assert_eq!(inspection.head_branch.as_deref(), Some("main"));
    assert_eq!(inspection.local_branches, ["main"]);
    assert!(matches!(
        inspection.configuration,
        ConfigurationInspection::Missing
    ));
    assert_eq!(inspection.identity, IdentityInspection::Available);
    assert!(inspection.remotes.is_empty());
}

#[test]
fn inspect_valid_configuration_preserves_source_bytes() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = support::config_source();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &source).unwrap();

    let inspection = service.inspect(&fixture.root).unwrap();

    assert_eq!(
        inspection.configuration,
        ConfigurationInspection::Valid(canonical::parse_repository_config(&source).unwrap())
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
}

#[test]
fn inspect_invalid_configuration_preserves_its_validation_problem_and_source_bytes() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = "format_version = 2\nprimary_branch = \"main\"\n";
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, source).unwrap();
    let expected = canonical::parse_repository_config(source).unwrap_err();

    let inspection = service.inspect(&fixture.root).unwrap();

    assert_eq!(
        inspection.configuration,
        ConfigurationInspection::Invalid(expected)
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
}

#[test]
fn inspect_non_git_path_returns_a_not_repository_error_without_mutation() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let selected = tempfile::tempdir().unwrap();

    let error = service.inspect(selected.path()).unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Inspect);
    assert_eq!(error.kind, RepositoryErrorKind::NotRepository);
    assert_eq!(error.root, None);
    assert!(!selected.path().join(".git").exists());
}

#[test]
fn inspect_bare_repository_returns_a_bare_repository_error_without_mutation() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::bare_repository();
    let config_before = std::fs::read(fixture.repository.path().join("config")).unwrap();

    let error = service.inspect(&fixture.root).unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Inspect);
    assert_eq!(error.kind, RepositoryErrorKind::BareRepository);
    assert_eq!(
        error.root,
        Some(std::fs::canonicalize(&fixture.root).unwrap())
    );
    assert_eq!(
        std::fs::read(fixture.repository.path().join("config")).unwrap(),
        config_before
    );
}

#[test]
fn inspect_non_root_repository_path_returns_a_path_validation_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let nested = fixture.root.join("nested");
    std::fs::create_dir(&nested).unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let registry_before = registry_row_count(data.path());

    let error = service.inspect(&nested).unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Inspect);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_eq!(
        error.root,
        Some(std::fs::canonicalize(&fixture.root).unwrap())
    );
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(registry_row_count(data.path()), registry_before);
}

#[test]
fn inspect_file_path_returns_a_path_validation_error() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let selected = tempfile::NamedTempFile::new().unwrap();

    let error = service.inspect(selected.path()).unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Inspect);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
}

#[test]
fn inspect_detached_head_returns_a_recoverable_error_without_mutation() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let head = fixture.repository.head().unwrap().target().unwrap();
    fixture.repository.set_head_detached(head).unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let registry_before = registry_row_count(data.path());

    let error = service.inspect(&fixture.root).unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Inspect);
    assert_eq!(error.kind, RepositoryErrorKind::DetachedHead);
    assert_eq!(
        error.root,
        Some(std::fs::canonicalize(&fixture.root).unwrap())
    );
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(registry_row_count(data.path()), registry_before);
}

#[test]
fn inspect_unborn_repository_reports_its_symbolic_branch_without_creating_a_commit() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::unborn_repository();
    fixture.repository.set_head("refs/heads/main").unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let mut effective_config = Config::new().unwrap();

    let inspection = service
        .inspect_with_identity_config_for_testing(&fixture.root, &mut effective_config)
        .unwrap();

    assert_eq!(inspection.head_branch.as_deref(), Some("main"));
    assert!(inspection.local_branches.is_empty());
    assert_eq!(inspection.identity, IdentityInspection::Required);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert!(
        fixture
            .repository
            .find_reference("HEAD")
            .unwrap()
            .target()
            .is_none()
    );
}

#[test]
fn identity_local_complete_values_are_available() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();

    let inspection = service.inspect(&fixture.root).unwrap();

    assert_eq!(inspection.identity, IdentityInspection::Available);
}

#[test]
fn identity_missing_effective_values_are_required_without_global_config() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::repository_without_local_identity();
    let mut effective_config = git2::Config::new().unwrap();

    let inspection = service
        .inspect_with_identity_config_for_testing(&fixture.root, &mut effective_config)
        .unwrap();

    assert_eq!(inspection.identity, IdentityInspection::Required);
}

#[test]
fn identity_partial_local_values_are_not_completed_by_effective_global_values() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::repository_without_local_identity();
    let local_config_path = fixture.repository.path().join("config");
    let mut local_config = Config::open(&local_config_path).unwrap();
    local_config.set_str("user.name", "Local Name").unwrap();
    let (config_directory, mut effective_config) =
        isolated_effective_config("[user]\nemail = global@example.invalid\n");
    let local_source = config_directory.path().join("local.gitconfig");
    std::fs::write(&local_source, "[user]\nname = Local Name\n").unwrap();
    effective_config
        .add_file(&local_source, ConfigLevel::Local, false)
        .unwrap();

    let inspection = service
        .inspect_with_identity_config_for_testing(&fixture.root, &mut effective_config)
        .unwrap();

    assert_eq!(inspection.identity, IdentityInspection::Required);
}

#[test]
fn identity_complete_same_level_effective_values_are_available() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::repository_without_local_identity();
    let (_config_directory, mut effective_config) =
        isolated_effective_config("[user]\nname = Global Name\nemail = global@example.invalid\n");

    let inspection = service
        .inspect_with_identity_config_for_testing(&fixture.root, &mut effective_config)
        .unwrap();

    assert_eq!(inspection.identity, IdentityInspection::Available);
}

#[test]
fn identity_uses_common_local_config_when_inspecting_a_linked_worktree() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let mut config = fixture.repository.config().unwrap();
    config.set_str("user.name", "Common Local Name").unwrap();
    config
        .set_str("user.email", "common-local@example.invalid")
        .unwrap();
    let worktree_directory = tempfile::tempdir().unwrap();
    let worktree_path = worktree_directory.path().join("linked");
    let worktree = fixture
        .repository
        .worktree("linked", &worktree_path, None)
        .unwrap();
    let linked = Repository::open_from_worktree(&worktree).unwrap();

    let inspection = service.inspect(&worktree_path).unwrap();

    assert!(linked.is_worktree());
    assert_eq!(inspection.identity, IdentityInspection::Available);
}

#[test]
fn enable_born_main_writes_canonical_config_exclusion_and_single_file_commit() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    std::fs::write(fixture.repository.path().join("info/exclude"), b"*.tmp\r\n").unwrap();
    let before = support::head_commit(&fixture.repository).unwrap();

    let outcome = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    let EnableRepositoryOutcome::Enabled { commit_oid } = outcome else {
        panic!("expected an initialization commit");
    };
    assert_ne!(commit_oid, before);
    assert_eq!(
        support::tracked_configuration(&fixture.root).unwrap(),
        canonical::serialize_repository_config(&canonical::RepositoryConfig {
            primary_branch: "main".to_owned(),
            publication_remote: None,
            unknown: toml::Table::new(),
        })
        .unwrap()
        .into_bytes()
    );
    let exclude = support::exclude_bytes(&fixture.repository).unwrap();
    assert_eq!(
        exclude
            .split(|byte| *byte == b'\n')
            .filter(|line| line.strip_suffix(b"\r").unwrap_or(line) == b".manyhands/worktrees/")
            .count(),
        1
    );
    let commit = fixture.repository.find_commit(commit_oid).unwrap();
    assert_eq!(commit.message(), Some("Initialize Manyhands"));
    let parent = commit.parent(0).unwrap();
    let diff = fixture
        .repository
        .diff_tree_to_tree(
            Some(&parent.tree().unwrap()),
            Some(&commit.tree().unwrap()),
            None,
        )
        .unwrap();
    assert_eq!(diff.deltas().len(), 1);
    assert_eq!(
        diff.deltas().next().unwrap().new_file().path(),
        Some(std::path::Path::new(canonical::CONFIG_PATH))
    );
    let root = std::fs::canonicalize(&fixture.root).unwrap();
    let entry = fixture
        .repository
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    let row = registry_row(data.path(), &root).unwrap();
    assert_eq!(registry_row_count(data.path()), 1);
    assert_eq!(row.0, "accessible");
    assert!(!row.1.is_empty());
    assert_eq!(row.1, entry.id().to_string());
    assert_eq!(row.2, 1);
    assert!(row.3 > 0);
}

#[test]
fn registration_retries_existing_configuration_without_another_commit_or_row() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();

    let first = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    let EnableRepositoryOutcome::Enabled { commit_oid } = first else {
        panic!("expected an initialization commit");
    };
    let canonical_root = std::fs::canonicalize(&fixture.root).unwrap();

    let second = service
        .enable(enable_request(&fixture.root.join("."), "main"))
        .unwrap();

    assert_eq!(second, EnableRepositoryOutcome::AlreadyEnabled);
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_eq!(registry_row_count(data.path()), 1);
    assert!(registry_row(data.path(), &canonical_root).is_some());
}

#[test]
fn registration_reconciliation_preserves_the_original_enabled_timestamp() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    let root = std::fs::canonicalize(&fixture.root).unwrap();
    Connection::open(data.path().join(REGISTRY_FILE))
        .unwrap()
        .execute(
            "UPDATE repositories SET enabled_at = 123 WHERE root_path = ?1",
            [root.to_str().unwrap()],
        )
        .unwrap();

    let outcome = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    assert_eq!(outcome, EnableRepositoryOutcome::AlreadyEnabled);
    assert_eq!(registry_row(data.path(), &root).unwrap().3, 123);
}

#[test]
fn remote_listing_addition_and_removal_use_only_local_configuration() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let unreachable = "ssh://git@127.0.0.1:1/never-contacted.git";

    assert_eq!(
        service
            .add_remote(add_remote_request(&fixture.root, "origin", unreachable))
            .unwrap(),
        RemoteOutcome::Changed
    );
    assert_eq!(
        service.list_remotes(&fixture.root).unwrap(),
        vec![manyhands::repository::RemoteInfo {
            name: "origin".to_owned(),
            fetch_url: unreachable.to_owned(),
            push_url: unreachable.to_owned(),
            publication_eligible: true,
        }]
    );
    assert_eq!(
        service
            .add_remote(add_remote_request(&fixture.root, "origin", unreachable))
            .unwrap(),
        RemoteOutcome::NoChange
    );
    let conflict = service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:other.git",
        ))
        .unwrap_err();
    assert_eq!(conflict.kind, RepositoryErrorKind::RemoteNameConflict);
    assert_eq!(
        service.remove_remote(&fixture.root, "origin").unwrap(),
        RemoteOutcome::Changed
    );
    assert_eq!(
        service.remove_remote(&fixture.root, "origin").unwrap(),
        RemoteOutcome::NoChange
    );
}

#[test]
fn remote_mutations_refresh_registered_rows_and_preserve_authoritative_git_on_registry_failure() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    let root = std::fs::canonicalize(&fixture.root).unwrap();
    let registry = data.path().join(REGISTRY_FILE);

    Connection::open(&registry)
        .unwrap()
        .execute(
            "UPDATE repositories SET refresh_required = 0 WHERE root_path = ?1",
            [root.to_str().unwrap()],
        )
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    assert_eq!(registry_row(data.path(), &root).unwrap().2, 1);
    Connection::open(&registry)
        .unwrap()
        .execute(
            "UPDATE repositories SET refresh_required = 0 WHERE root_path = ?1",
            [root.to_str().unwrap()],
        )
        .unwrap();
    service.remove_remote(&fixture.root, "origin").unwrap();
    assert_eq!(registry_row(data.path(), &root).unwrap().2, 1);

    std::fs::remove_file(&registry).unwrap();
    std::fs::create_dir(&registry).unwrap();
    let add_error = service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap_err();
    assert_eq!(add_error.kind, RepositoryErrorKind::RegistryRefreshPending);
    assert!(fixture.repository.find_remote("origin").is_ok());

    let remove_error = service.remove_remote(&fixture.root, "origin").unwrap_err();
    assert_eq!(
        remove_error.kind,
        RepositoryErrorKind::RegistryRefreshPending
    );
    assert!(fixture.repository.find_remote("origin").is_err());
}

#[test]
fn publication_remote_requires_ssh_fetch_and_effective_push_urls() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    for (name, url) in [
        ("ssh", "ssh://git@example.invalid/group/project.git"),
        ("scp", "git@example.invalid:group/project.git"),
        ("http", "https://example.invalid/group/project.git"),
        ("file", "file:///tmp/project.git"),
        ("local", "/tmp/project.git"),
        ("malformed", "ssh:///project.git"),
        ("drive", "C:/project.git"),
        ("drive-relative", "C:project.git"),
        ("drive-backslash", "C:\\project.git"),
        ("empty-port", "ssh://git@example.invalid:/project.git"),
        ("text-port", "ssh://git@example.invalid:abc/project.git"),
        ("large-port", "ssh://git@example.invalid:65536/project.git"),
        ("file-scp", "file:repository.git"),
        ("https-scp", "https:repository.git"),
        ("http-scp", "http:repository.git"),
        ("local-colon", "/tmp:repository.git"),
        ("empty-user", "@example.invalid:repository.git"),
    ] {
        service
            .add_remote(add_remote_request(&fixture.root, name, url))
            .unwrap();
    }
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "ssh-push-http",
            "git@example.invalid:fetch.git",
        ))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "http-push-ssh",
            "https://example.invalid/fetch.git",
        ))
        .unwrap();
    let mut config = fixture.repository.config().unwrap();
    config
        .set_str(
            "remote.ssh-push-http.pushurl",
            "https://example.invalid/push.git",
        )
        .unwrap();
    config
        .set_str(
            "remote.http-push-ssh.pushurl",
            "git@example.invalid:push.git",
        )
        .unwrap();
    stage_configuration(&fixture);

    for name in ["ssh", "scp"] {
        assert!(matches!(
            service.set_publication_remote(publication_request(&fixture.root, Some(name))),
            Ok(PublicationRemoteOutcome::Changed { .. })
        ));
        let mut index = fixture.repository.index().unwrap();
        index
            .add_path(std::path::Path::new(canonical::CONFIG_PATH))
            .unwrap();
        index.write().unwrap();
    }
    for name in [
        "http",
        "file",
        "local",
        "malformed",
        "drive",
        "drive-relative",
        "drive-backslash",
        "empty-port",
        "text-port",
        "large-port",
        "file-scp",
        "https-scp",
        "http-scp",
        "local-colon",
        "empty-user",
        "ssh-push-http",
        "http-push-ssh",
    ] {
        let error = service
            .set_publication_remote(publication_request(&fixture.root, Some(name)))
            .unwrap_err();
        assert_eq!(error.kind, RepositoryErrorKind::InvalidPublicationRemote);
    }
}

#[test]
fn publication_selection_commits_only_canonical_configuration_changes() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    let before = support::head_commit(&fixture.repository).unwrap();

    let PublicationRemoteOutcome::Changed { commit_oid } = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap()
    else {
        panic!("expected publication commit");
    };
    let commit = fixture.repository.find_commit(commit_oid).unwrap();
    assert_eq!(
        commit.message(),
        Some("Configure Manyhands publication remote")
    );
    assert_eq!(commit.parent_id(0).unwrap(), before);
    assert!(
        support::tracked_configuration(&fixture.root)
            .unwrap()
            .windows("publication_remote = \"origin\"".len())
            .any(|line| line == b"publication_remote = \"origin\"")
    );
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    index.write().unwrap();
    assert_eq!(
        service
            .set_publication_remote(publication_request(&fixture.root, Some("origin")))
            .unwrap(),
        PublicationRemoteOutcome::NoChange
    );
    assert!(matches!(
        service.set_publication_remote(publication_request(&fixture.root, None)),
        Ok(PublicationRemoteOutcome::Changed { .. })
    ));
}

#[test]
fn publication_repeated_changes_accept_the_expected_stale_live_index() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    for name in ["origin", "backup"] {
        service
            .add_remote(add_remote_request(
                &fixture.root,
                name,
                "git@example.invalid:project.git",
            ))
            .unwrap();
    }
    stage_configuration(&fixture);

    let PublicationRemoteOutcome::Changed {
        commit_oid: selected,
    } = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap()
    else {
        panic!("expected selection commit");
    };
    let PublicationRemoteOutcome::Changed {
        commit_oid: replaced,
    } = service
        .set_publication_remote(publication_request(&fixture.root, Some("backup")))
        .unwrap()
    else {
        panic!("expected replacement commit");
    };
    let PublicationRemoteOutcome::Changed {
        commit_oid: cleared,
    } = service
        .set_publication_remote(publication_request(&fixture.root, None))
        .unwrap()
    else {
        panic!("expected clearing commit");
    };

    assert_ne!(selected, replaced);
    assert_ne!(replaced, cleared);
    assert_eq!(
        service
            .set_publication_remote(publication_request(&fixture.root, None))
            .unwrap(),
        PublicationRemoteOutcome::NoChange
    );
}

#[test]
fn publication_selection_rejects_a_missing_live_index_configuration_entry() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    let head_before = support::head_commit(&fixture.repository);

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
}

#[test]
fn publication_selection_rejects_a_staged_malformed_ancestor_configuration_blob() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    support::commit_tracked_configuration(
        &fixture,
        "format_version = 9\nprimary_branch = \"main\"\n",
    );
    let malformed = support::head_commit(&fixture.repository).unwrap();
    support::commit_tracked_configuration(&fixture, &support::config_source());
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    let entry = fixture
        .repository
        .find_commit(malformed)
        .unwrap()
        .tree()
        .unwrap()
        .get_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: entry.id(),
            flags: 0,
            flags_extended: 0,
            path: canonical::CONFIG_PATH.as_bytes().to_vec(),
        })
        .unwrap();
    index.write().unwrap();

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
}

#[test]
fn publication_selection_rejects_a_staged_valid_but_noncanonical_ancestor_configuration_blob() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    support::commit_tracked_configuration(
        &fixture,
        "primary_branch = \"main\"\nformat_version = 1\n",
    );
    let noncanonical = support::head_commit(&fixture.repository).unwrap();
    support::commit_tracked_configuration(&fixture, &support::config_source());
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    let entry = fixture
        .repository
        .find_commit(noncanonical)
        .unwrap()
        .tree()
        .unwrap()
        .get_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: entry.id(),
            flags: 0,
            flags_extended: 0,
            path: canonical::CONFIG_PATH.as_bytes().to_vec(),
        })
        .unwrap();
    index.write().unwrap();

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
}

#[test]
fn publication_pending_commit_retries_registration_without_another_commit() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    let registry = data.path().join(REGISTRY_FILE);
    std::fs::remove_file(&registry).unwrap();
    std::fs::create_dir(&registry).unwrap();

    let PublicationRemoteOutcome::RegistrationPending { commit_oid } = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap()
    else {
        panic!("expected pending registration");
    };
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert!(
        support::tracked_configuration(&fixture.root)
            .unwrap()
            .windows("publication_remote = \"origin\"".len())
            .any(|line| line == b"publication_remote = \"origin\"")
    );

    std::fs::remove_dir(&registry).unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    index.write().unwrap();
    let retry_service = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        retry_service
            .set_publication_remote(publication_request(&fixture.root, Some("origin")))
            .unwrap(),
        PublicationRemoteOutcome::NoChange
    );
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    let root = std::fs::canonicalize(&fixture.root).unwrap();
    assert_eq!(
        registry_row(data.path(), &root).unwrap().1,
        fixture
            .repository
            .find_commit(commit_oid)
            .unwrap()
            .tree()
            .unwrap()
            .get_path(std::path::Path::new(canonical::CONFIG_PATH))
            .unwrap()
            .id()
            .to_string()
    );

    std::fs::remove_file(&registry).unwrap();
    std::fs::create_dir(&registry).unwrap();
    let pending = retry_service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();
    assert_eq!(pending.kind, RepositoryErrorKind::RegistryRefreshPending);
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
}

#[test]
fn publication_selection_blocks_selected_removal_and_dirty_configuration_path() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap();

    let removal = service.remove_remote(&fixture.root, "origin").unwrap_err();
    assert_eq!(removal.kind, RepositoryErrorKind::SelectedRemoteRemoval);
    let mut dirty_source =
        String::from_utf8(support::tracked_configuration(&fixture.root).unwrap()).unwrap();
    dirty_source.push_str("unrelated = true\n");
    std::fs::write(fixture.root.join(canonical::CONFIG_PATH), dirty_source).unwrap();
    let dirty = service
        .set_publication_remote(publication_request(&fixture.root, None))
        .unwrap_err();
    assert_eq!(dirty.kind, RepositoryErrorKind::DirtyConfigurationPath);
}

#[test]
fn publication_selection_rejects_a_staged_configuration_edit_without_mutation() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = std::fs::read(&path).unwrap();
    let mut staged = source.clone();
    staged.extend_from_slice(b"staged = true\n");
    std::fs::write(&path, &staged).unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    index.write().unwrap();
    std::fs::write(&path, &source).unwrap();
    let head_before = support::head_commit(&fixture.repository);

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(std::fs::read(path).unwrap(), source);
}

#[test]
fn publication_selection_rejects_a_staged_configuration_type_change_without_mutation() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = std::fs::read(&path).unwrap();
    let mut index = fixture.repository.index().unwrap();
    let blob = fixture.repository.blob(&source).unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o120000,
            uid: 0,
            gid: 0,
            file_size: source.len() as u32,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: canonical::CONFIG_PATH.as_bytes().to_vec(),
        })
        .unwrap();
    index.write().unwrap();
    let head_before = support::head_commit(&fixture.repository);

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(std::fs::read(path).unwrap(), source);
}

#[test]
fn publication_selection_rejects_a_staged_configuration_deletion_with_restored_worktree() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = std::fs::read(&path).unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .remove_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    index.write().unwrap();
    std::fs::write(&path, &source).unwrap();
    let head_before = support::head_commit(&fixture.repository);

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(std::fs::read(path).unwrap(), source);
}

#[cfg(unix)]
#[test]
fn publication_selection_rejects_an_executable_configuration_file_without_mutation() {
    use std::os::unix::fs::PermissionsExt;

    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = std::fs::read(&path).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&path, permissions).unwrap();
    let head_before = support::head_commit(&fixture.repository);
    let index_before = support::index_bytes(&fixture.repository);
    let registry_before = registry_row(data.path(), &std::fs::canonicalize(&fixture.root).unwrap());

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(std::fs::read(path).unwrap(), source);
    assert_eq!(
        registry_row(data.path(), &std::fs::canonicalize(&fixture.root).unwrap()),
        registry_before
    );
    assert_eq!(
        fixture.repository.find_remote("origin").unwrap().url(),
        Some("git@example.invalid:project.git")
    );
}

#[cfg(unix)]
#[test]
fn publication_selection_rejects_a_symlink_configuration_file_without_mutation() {
    use std::os::unix::fs::symlink;

    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = std::fs::read(&path).unwrap();
    let external = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(external.path(), &source).unwrap();
    std::fs::remove_file(&path).unwrap();
    symlink(external.path(), &path).unwrap();
    let head_before = support::head_commit(&fixture.repository);
    let index_before = support::index_bytes(&fixture.repository);
    let registry_before = registry_row(data.path(), &std::fs::canonicalize(&fixture.root).unwrap());

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(std::fs::read(path).unwrap(), source);
    assert_eq!(
        registry_row(data.path(), &std::fs::canonicalize(&fixture.root).unwrap()),
        registry_before
    );
    assert_eq!(
        fixture.repository.find_remote("origin").unwrap().url(),
        Some("git@example.invalid:project.git")
    );
}

#[test]
fn publication_noop_selection_rejects_a_staged_configuration_change() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap();
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = std::fs::read(&path).unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    index.write().unwrap();
    let mut staged = source.clone();
    staged.extend_from_slice(b"staged = true\n");
    std::fs::write(&path, &staged).unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
    index.write().unwrap();
    std::fs::write(&path, &source).unwrap();
    let head_before = support::head_commit(&fixture.repository);

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(std::fs::read(path).unwrap(), source);
}

#[test]
fn publication_noop_clear_rejects_an_unstaged_configuration_change() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let mut dirty = std::fs::read(&path).unwrap();
    dirty.extend_from_slice(b"unstaged = true\n");
    std::fs::write(&path, &dirty).unwrap();
    let head_before = support::head_commit(&fixture.repository);

    let error = service
        .set_publication_remote(publication_request(&fixture.root, None))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(std::fs::read(path).unwrap(), dirty);
}

#[cfg(target_os = "linux")]
#[test]
fn registration_rejects_a_non_utf8_canonical_root_before_enablement_mutation() {
    use std::os::unix::ffi::OsStrExt;

    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let root = data
        .path()
        .join(std::ffi::OsStr::from_bytes(b"repository-\xff"));
    std::fs::create_dir(&root).unwrap();
    let repository = Repository::init(&root).unwrap();
    let before = repository_snapshot(&repository, &root);

    let error = service
        .enable(EnableRepositoryRequest {
            root: root.clone(),
            primary_branch: "main".to_owned(),
            identity: Some(CommitIdentity {
                name: "Rejected Author".to_owned(),
                email: "rejected@example.invalid".to_owned(),
            }),
        })
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Enable);
    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_eq!(error.root, Some(std::fs::canonicalize(&root).unwrap()));
    assert_eq!(repository_snapshot(&repository, &root), before);
    assert_eq!(registry_row_count(data.path()), 0);
}

#[test]
fn registration_failure_after_commit_is_pending_and_a_retry_registers_without_committing() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let registry = data.path().join(REGISTRY_FILE);
    std::fs::remove_file(&registry).unwrap();
    std::fs::create_dir(&registry).unwrap();

    let first = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    let EnableRepositoryOutcome::RegistrationPending { commit_oid } = first else {
        panic!("expected the committed initialization to remain pending registration");
    };
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));

    let error = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap_err();
    assert_eq!(error.operation, RepositoryOperation::Enable);
    assert_eq!(error.kind, RepositoryErrorKind::Sqlite);

    std::fs::remove_dir(&registry).unwrap();
    let retry = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    assert_eq!(retry, EnableRepositoryOutcome::AlreadyEnabled);
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_eq!(registry_row_count(data.path()), 1);
}

#[test]
fn remove_registration_only_deletes_its_canonical_registry_row() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);

    let removed = service
        .remove_registration(&fixture.root.join("."))
        .unwrap();
    let retried = service.remove_registration(&fixture.root).unwrap();

    assert_eq!(removed, RemoveRegistrationOutcome::Removed);
    assert_eq!(retried, RemoveRegistrationOutcome::NotRegistered);
    assert_eq!(registry_row_count(data.path()), 0);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
}

#[test]
fn remove_registration_rejects_missing_and_file_roots_without_deleting_existing_rows() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();
    let missing = fixture.root.join("missing");
    let file = fixture.root.join("file");
    std::fs::write(&file, "file\n").unwrap();

    for root in [&missing, &file] {
        let error = service.remove_registration(root).unwrap_err();
        assert_eq!(error.operation, RepositoryOperation::RemoveRegistration);
        assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
        assert_eq!(registry_row_count(data.path()), 1);
    }
}

#[test]
fn enable_unborn_trunk_creates_its_first_commit_without_master() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::unborn_repository();

    let outcome = service
        .enable(EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "trunk".to_owned(),
            identity: Some(CommitIdentity {
                name: "Trunk Author".to_owned(),
                email: "trunk@example.invalid".to_owned(),
            }),
        })
        .unwrap();

    assert!(matches!(outcome, EnableRepositoryOutcome::Enabled { .. }));
    assert_eq!(
        fixture.repository.head().unwrap().shorthand(),
        Some("trunk")
    );
    assert!(support::head_commit(&fixture.repository).is_some());
    assert!(
        fixture
            .repository
            .find_branch("trunk", BranchType::Local)
            .is_ok()
    );
    assert!(
        fixture
            .repository
            .find_branch("master", BranchType::Local)
            .is_err()
    );
    assert_eq!(
        canonical::parse_repository_config(
            std::str::from_utf8(&support::tracked_configuration(&fixture.root).unwrap()).unwrap()
        )
        .unwrap()
        .primary_branch,
        "trunk"
    );
}

#[test]
fn enable_unborn_untracked_worktree_rejects_without_mutation() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::unborn_repository();
    std::fs::write(fixture.root.join("untracked.txt"), "untracked\n").unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let config_before = support::tracked_configuration(&fixture.root);
    let exclude_before = support::exclude_bytes(&fixture.repository);
    let index_before = support::index_bytes(&fixture.repository);

    let error = service
        .enable(EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "trunk".to_owned(),
            identity: Some(CommitIdentity {
                name: "Untracked Author".to_owned(),
                email: "untracked@example.invalid".to_owned(),
            }),
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::DirtyWorktree);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(support::tracked_configuration(&fixture.root), config_before);
    assert_eq!(support::exclude_bytes(&fixture.repository), exclude_before);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(registry_row_count(data.path()), 0);
}

#[test]
fn enable_unborn_configuration_obstacle_rolls_back_all_precommit_mutations() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::unborn_repository();
    let mut config = fixture.repository.config().unwrap();
    config.set_str("user.name", "Configured Author").unwrap();
    config
        .set_str("user.email", "configured@example.invalid")
        .unwrap();
    drop(config);
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b".manyhands\r\nexisting\r\n",
    )
    .unwrap();
    std::fs::write(fixture.root.join(".manyhands"), "obstacle\n").unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let config_before = support::tracked_configuration(&fixture.root);
    let exclude_before = support::exclude_bytes(&fixture.repository);
    let index_before = support::index_bytes(&fixture.repository);
    let local_config_before = std::fs::read(fixture.repository.commondir().join("config")).unwrap();

    let error = service
        .enable(enable_request(&fixture.root, "trunk"))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::Io);
    assert_eq!(error.operation, RepositoryOperation::Enable);
    assert_eq!(error.root.as_deref(), Some(fixture.root.as_path()));
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(support::tracked_configuration(&fixture.root), config_before);
    assert_eq!(support::exclude_bytes(&fixture.repository), exclude_before);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(
        std::fs::read(fixture.repository.commondir().join("config")).unwrap(),
        local_config_before
    );
    assert_eq!(
        std::fs::read(fixture.root.join(".manyhands")).unwrap(),
        b"obstacle\n"
    );
    assert!(
        fixture
            .repository
            .find_branch("trunk", BranchType::Local)
            .is_err()
    );
    assert_eq!(registry_row_count(data.path()), 0);
}

#[test]
fn enable_unborn_obstacle_restores_supplied_local_identity() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::unborn_repository();
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b".manyhands\n",
    )
    .unwrap();
    std::fs::write(fixture.root.join(".manyhands"), "obstacle\n").unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let local_config_before = std::fs::read(fixture.repository.commondir().join("config")).unwrap();
    let index_before = support::index_bytes(&fixture.repository);

    let error = service
        .enable(EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "trunk".to_owned(),
            identity: Some(CommitIdentity {
                name: "Temporary Author".to_owned(),
                email: "temporary@example.invalid".to_owned(),
            }),
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::Io);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(
        std::fs::read(fixture.repository.commondir().join("config")).unwrap(),
        local_config_before
    );
    assert_eq!(registry_row_count(data.path()), 0);
}

#[test]
fn enable_unborn_commit_lock_removes_owned_configuration_directory() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::unborn_repository();
    let mut config = fixture.repository.config().unwrap();
    config.set_str("user.name", "Configured Author").unwrap();
    config
        .set_str("user.email", "configured@example.invalid")
        .unwrap();
    drop(config);
    let exclude_path = fixture.repository.path().join("info/exclude");
    std::fs::write(&exclude_path, b"before\r\n").unwrap();
    let lock = fixture.repository.path().join("refs/heads/trunk.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(&lock, "lock\n").unwrap();
    let exclude_before = support::exclude_bytes(&fixture.repository);

    let error = service
        .enable(enable_request(&fixture.root, "trunk"))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::Git);
    assert_eq!(support::tracked_configuration(&fixture.root), None);
    assert_eq!(support::exclude_bytes(&fixture.repository), exclude_before);
    assert!(!fixture.root.join(".manyhands").exists());
    assert!(
        fixture
            .repository
            .find_branch("trunk", BranchType::Local)
            .is_err()
    );
    assert!(lock.exists());
    assert_eq!(registry_row_count(data.path()), 0);
    std::fs::remove_file(lock).unwrap();
}

#[test]
fn enable_unborn_commit_lock_preserves_non_owned_empty_configuration_directory() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::unborn_repository();
    let mut config = fixture.repository.config().unwrap();
    config.set_str("user.name", "Configured Author").unwrap();
    config
        .set_str("user.email", "configured@example.invalid")
        .unwrap();
    drop(config);
    std::fs::create_dir(fixture.root.join(".manyhands")).unwrap();
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b".manyhands/\n",
    )
    .unwrap();
    let lock = fixture.repository.path().join("refs/heads/trunk.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(&lock, "lock\n").unwrap();

    let error = service
        .enable(enable_request(&fixture.root, "trunk"))
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::Git);
    assert_eq!(support::tracked_configuration(&fixture.root), None);
    assert!(fixture.root.join(".manyhands").is_dir());
    assert!(lock.exists());
    std::fs::remove_file(lock).unwrap();
}

#[test]
fn enable_linked_worktree_writes_shared_common_exclude() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch("trunk", &head, false).unwrap();
    drop(head);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("linked");
    let reference = fixture
        .repository
        .find_reference("refs/heads/trunk")
        .unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    let worktree = fixture
        .repository
        .worktree("linked", &path, Some(&options))
        .unwrap();
    let linked = Repository::open_from_worktree(&worktree).unwrap();
    let common_exclude = linked.commondir().join("info/exclude");
    let per_worktree_exclude = linked.path().join("info/exclude");

    let outcome = service.enable(enable_request(&path, "trunk")).unwrap();

    assert!(matches!(outcome, EnableRepositoryOutcome::Enabled { .. }));
    assert!(support::tracked_configuration(&path).is_some());
    assert_eq!(
        std::fs::read(&common_exclude)
            .unwrap()
            .split(|byte| *byte == b'\n')
            .filter(|line| line.strip_suffix(b"\r").unwrap_or(line) == b".manyhands/worktrees/")
            .count(),
        1
    );
    assert_ne!(per_worktree_exclude, common_exclude);
    assert_eq!(registry_row_count(data.path()), 1);
}

#[test]
fn enable_wrong_checked_out_branch_preserves_repository_and_registry() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch("trunk", &head, false).unwrap();
    drop(head);
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let index_before = support::index_bytes(&fixture.repository);
    let registry_before = registry_row(data.path(), &fixture.root);

    let error = service
        .enable(enable_request(&fixture.root, "trunk"))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Enable);
    assert_eq!(error.kind, RepositoryErrorKind::WrongCheckedOutBranch);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(registry_row(data.path(), &fixture.root), registry_before);
}

#[test]
fn enable_rejects_dirty_staged_untracked_deleted_and_conflicted_worktrees_without_mutation() {
    for state in ["dirty", "staged", "untracked", "deleted", "conflicted"] {
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let fixture = support::born_repository();
        match state {
            "dirty" => std::fs::write(fixture.root.join("fixture.txt"), "changed\n").unwrap(),
            "staged" => {
                std::fs::write(fixture.root.join("fixture.txt"), "staged\n").unwrap();
                let mut index = fixture.repository.index().unwrap();
                index.add_path(std::path::Path::new("fixture.txt")).unwrap();
                index.write().unwrap();
            }
            "untracked" => std::fs::write(fixture.root.join("untracked.txt"), "new\n").unwrap(),
            "deleted" => std::fs::remove_file(fixture.root.join("fixture.txt")).unwrap(),
            "conflicted" => support::conflict_primary_worktree(&fixture),
            _ => unreachable!(),
        }
        let before = repository_snapshot(&fixture.repository, &fixture.root);
        let config_before = support::tracked_configuration(&fixture.root);
        let exclude_before = support::exclude_bytes(&fixture.repository);

        let error = service
            .enable(enable_request(&fixture.root, "main"))
            .unwrap_err();

        assert_eq!(error.operation, RepositoryOperation::Enable, "{state}");
        assert_eq!(
            error.kind,
            if state == "conflicted" {
                RepositoryErrorKind::ConflictedWorktree
            } else {
                RepositoryErrorKind::DirtyWorktree
            },
            "{state}"
        );
        assert_eq!(
            repository_snapshot(&fixture.repository, &fixture.root),
            before,
            "{state}"
        );
        assert_eq!(
            support::tracked_configuration(&fixture.root),
            config_before,
            "{state}"
        );
        assert_eq!(
            support::exclude_bytes(&fixture.repository),
            exclude_before,
            "{state}"
        );
        assert_eq!(registry_row_count(data.path()), 0, "{state}");
    }
}

#[test]
fn enable_identity_required_precedes_exclude_configuration_and_commit_writes() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::repository_without_local_identity();
    let mut effective_config = Config::new().unwrap();
    let before = repository_snapshot(&fixture.repository, &fixture.root);

    let outcome = service
        .enable_with_identity_config_for_testing(
            enable_request(&fixture.root, "main"),
            &mut effective_config,
        )
        .unwrap();

    assert_eq!(outcome, EnableRepositoryOutcome::IdentityRequired);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(support::tracked_configuration(&fixture.root), None);
    assert_eq!(registry_row_count(data.path()), 0);
}

#[test]
fn enable_caller_identity_is_local_and_authors_initialization_commit() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::repository_without_local_identity();
    let identity = CommitIdentity {
        name: "Confirmed Author".to_owned(),
        email: "confirmed@example.invalid".to_owned(),
    };

    let outcome = service
        .enable(EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: Some(identity),
        })
        .unwrap();

    let EnableRepositoryOutcome::Enabled { commit_oid } = outcome else {
        panic!("expected an initialization commit");
    };
    let config = fixture.repository.config().unwrap();
    assert_eq!(config.get_string("user.name").unwrap(), "Confirmed Author");
    assert_eq!(
        config.get_string("user.email").unwrap(),
        "confirmed@example.invalid"
    );
    let commit = fixture.repository.find_commit(commit_oid).unwrap();
    assert_eq!(commit.author().name(), Some("Confirmed Author"));
    assert_eq!(commit.author().email(), Some("confirmed@example.invalid"));
    assert_eq!(commit.committer().name(), commit.author().name());
    assert_eq!(commit.committer().email(), commit.author().email());
}

#[test]
fn enable_existing_valid_configuration_preserves_unknown_source_and_creates_no_commit() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    let path = fixture.root.join(canonical::CONFIG_PATH);
    let source = "format_version = 1\nprimary_branch = \"main\"\nfuture_value = \"preserve\"\n";
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, source).unwrap();
    let signature =
        git2::Signature::now("Manyhands Test", "manyhands-test@example.invalid").unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(canonical::CONFIG_PATH))
        .unwrap();
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
    drop(tree);
    drop(parent);
    let head_before = support::head_commit(&fixture.repository);

    let outcome = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    assert_eq!(outcome, EnableRepositoryOutcome::AlreadyEnabled);
    assert_eq!(
        support::tracked_configuration(&fixture.root),
        Some(source.as_bytes().to_vec())
    );
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(registry_row_count(data.path()), 1);
}

#[test]
fn enable_existing_configuration_deduplicates_crlf_exclusion_rules() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    support::commit_tracked_configuration(
        &fixture,
        "format_version = 1\nprimary_branch = \"main\"\n",
    );
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b"first\r\n.manyhands/worktrees/\r\nsecond\n.manyhands/worktrees/\n",
    )
    .unwrap();

    let outcome = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    assert_eq!(outcome, EnableRepositoryOutcome::AlreadyEnabled);
    assert_eq!(
        support::exclude_bytes(&fixture.repository),
        Some(b"first\r\nsecond\n.manyhands/worktrees/\r\n".to_vec())
    );
}

#[test]
fn enable_existing_configuration_delimits_a_lone_unterminated_exclusion_rule() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::born_repository();
    support::commit_tracked_configuration(
        &fixture,
        "format_version = 1\nprimary_branch = \"main\"\n",
    );
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b"first\r\n.manyhands/worktrees/",
    )
    .unwrap();

    let outcome = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    assert_eq!(outcome, EnableRepositoryOutcome::AlreadyEnabled);
    assert_eq!(
        support::exclude_bytes(&fixture.repository),
        Some(b"first\r\n.manyhands/worktrees/\r\n".to_vec())
    );
}

#[test]
fn enable_invalid_configuration_with_supplied_identity_preserves_all_state() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::repository_without_local_identity();
    support::commit_tracked_configuration(
        &fixture,
        "format_version = 2\nprimary_branch = \"main\"\n",
    );
    let before = repository_snapshot(&fixture.repository, &fixture.root);
    let index_before = support::index_bytes(&fixture.repository);
    let registry_before = registry_row(data.path(), &fixture.root);

    let error = service
        .enable(EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: Some(CommitIdentity {
                name: "Must Not Persist".to_owned(),
                email: "must-not-persist@example.invalid".to_owned(),
            }),
        })
        .unwrap_err();

    assert_eq!(error.kind, RepositoryErrorKind::InvalidConfiguration);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        before
    );
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(registry_row(data.path(), &fixture.root), registry_before);
}

#[test]
fn enable_existing_valid_configuration_needs_no_identity_and_writes_no_identity_config() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let fixture = support::repository_without_local_identity();
    support::commit_tracked_configuration(
        &fixture,
        "format_version = 1\nprimary_branch = \"main\"\n",
    );
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b".manyhands/worktrees/\n.manyhands/worktrees/\n",
    )
    .unwrap();
    let head_before = support::head_commit(&fixture.repository);
    let configuration_before = support::tracked_configuration(&fixture.root);
    let local_config_before = std::fs::read(fixture.repository.commondir().join("config")).unwrap();
    let mut effective_config = Config::new().unwrap();

    let outcome = service
        .enable_with_identity_config_for_testing(
            enable_request(&fixture.root, "main"),
            &mut effective_config,
        )
        .unwrap();

    assert_eq!(outcome, EnableRepositoryOutcome::AlreadyEnabled);
    assert_eq!(support::head_commit(&fixture.repository), head_before);
    assert_eq!(
        std::fs::read(fixture.repository.commondir().join("config")).unwrap(),
        local_config_before
    );
    assert_eq!(
        support::exclude_bytes(&fixture.repository),
        Some(b".manyhands/worktrees/\n".to_vec())
    );
    assert_eq!(
        support::tracked_configuration(&fixture.root),
        configuration_before
    );
    assert_eq!(registry_row_count(data.path()), 1);
}

#[test]
fn recovery_before_configuration_write_restores_unborn_state_then_retries() {
    let data = tempfile::tempdir().unwrap();
    let fixture = support::unborn_repository();
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b"before\r\n",
    )
    .unwrap();
    let repository_before = repository_snapshot(&fixture.repository, &fixture.root);
    let index_before = support::index_bytes(&fixture.repository);
    let remotes_before = remote_names(&fixture.repository);
    let exclude_before = support::exclude_bytes(&fixture.repository);
    let service = failing_service(data.path(), FailurePoint::BeforeConfigurationWrite);

    let error = service
        .enable(enable_request(&fixture.root, "trunk"))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Enable);
    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        repository_before
    );
    assert_eq!(support::tracked_configuration(&fixture.root), None);
    assert_eq!(support::exclude_bytes(&fixture.repository), exclude_before);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(remote_names(&fixture.repository), remotes_before);
    assert!(fixture.repository.is_empty().unwrap());
    assert!(
        fixture
            .repository
            .find_branch("trunk", BranchType::Local)
            .is_err()
    );
    assert_eq!(registry_row_count(data.path()), 0);

    assert!(matches!(
        service
            .enable(enable_request(&fixture.root, "trunk"))
            .unwrap(),
        EnableRepositoryOutcome::Enabled { .. }
    ));
    assert_eq!(commit_count(&fixture.repository), 1);
    assert_eq!(
        head_configuration(&fixture.repository),
        support::tracked_configuration(&fixture.root).unwrap()
    );
    assert_eq!(
        support::exclude_bytes(&fixture.repository),
        Some(b"before\r\n.manyhands/worktrees/\r\n".to_vec())
    );
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(remote_names(&fixture.repository), remotes_before);
    assert_eq!(registry_row_count(data.path()), 1);
    assert_registry_matches_head(data.path(), &fixture);
}

#[test]
fn recovery_before_initialization_commit_restores_unborn_state_then_retries() {
    let data = tempfile::tempdir().unwrap();
    let fixture = support::unborn_repository();
    std::fs::write(
        fixture.repository.path().join("info/exclude"),
        b"before\r\n",
    )
    .unwrap();
    let repository_before = repository_snapshot(&fixture.repository, &fixture.root);
    let index_before = support::index_bytes(&fixture.repository);
    let remotes_before = remote_names(&fixture.repository);
    let exclude_before = support::exclude_bytes(&fixture.repository);
    let service = failing_service(data.path(), FailurePoint::BeforeInitializationCommit);

    let error = service
        .enable(enable_request(&fixture.root, "trunk"))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::Enable);
    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    assert_eq!(
        repository_snapshot(&fixture.repository, &fixture.root),
        repository_before
    );
    assert_eq!(support::tracked_configuration(&fixture.root), None);
    assert_eq!(support::exclude_bytes(&fixture.repository), exclude_before);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(remote_names(&fixture.repository), remotes_before);
    assert!(fixture.repository.is_empty().unwrap());
    assert!(
        fixture
            .repository
            .find_branch("trunk", BranchType::Local)
            .is_err()
    );
    assert_eq!(registry_row_count(data.path()), 0);

    assert!(matches!(
        service
            .enable(enable_request(&fixture.root, "trunk"))
            .unwrap(),
        EnableRepositoryOutcome::Enabled { .. }
    ));
    assert_eq!(commit_count(&fixture.repository), 1);
    assert_eq!(
        head_configuration(&fixture.repository),
        support::tracked_configuration(&fixture.root).unwrap()
    );
    assert_eq!(
        support::exclude_bytes(&fixture.repository),
        Some(b"before\r\n.manyhands/worktrees/\r\n".to_vec())
    );
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(remote_names(&fixture.repository), remotes_before);
    assert_eq!(registry_row_count(data.path()), 1);
    assert_registry_matches_head(data.path(), &fixture);
}

#[test]
fn recovery_before_repository_initialization_removes_only_owned_target_then_retries() {
    let data = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let created = parent.path().join("created");
    let existing = parent.path().join("existing");
    std::fs::create_dir(&existing).unwrap();

    for root in [&created, &existing] {
        let service = failing_service(data.path(), FailurePoint::BeforeRepositoryInitialization);
        let registrations_before = registry_row_count(data.path());
        let error = service
            .create_and_enable(create_request(root, "main"))
            .unwrap_err();

        assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
        assert_eq!(registry_row_count(data.path()), registrations_before);
        if root == &created {
            assert!(!root.exists());
        } else {
            assert!(root.is_dir());
            assert!(std::fs::read_dir(root).unwrap().next().is_none());
        }
        assert!(matches!(
            service
                .create_and_enable(create_request(root, "main"))
                .unwrap(),
            EnableRepositoryOutcome::Enabled { .. }
        ));
        let repository = Repository::open(root).unwrap();
        assert!(!repository.is_bare());
        assert_eq!(repository.head().unwrap().shorthand(), Some("main"));
        assert!(repository.find_branch("main", BranchType::Local).is_ok());
        assert_eq!(commit_count(&repository), 1);
        let config_bytes = support::tracked_configuration(root).unwrap();
        let config =
            canonical::parse_repository_config(std::str::from_utf8(&config_bytes).unwrap())
                .unwrap();
        assert_eq!(config.primary_branch, "main");
        assert_eq!(config.publication_remote, None);
        assert!(config.unknown.is_empty());
        assert_eq!(
            config_bytes,
            canonical::serialize_repository_config(&config)
                .unwrap()
                .into_bytes()
        );
        let exclude = support::exclude_bytes(&repository).unwrap();
        assert!(exclude.ends_with(b".manyhands/worktrees/\n"));
        assert_eq!(
            exclude
                .windows(b".manyhands/worktrees/".len())
                .filter(|entry| *entry == b".manyhands/worktrees/")
                .count(),
            1
        );
        if root == &created {
            assert_eq!(
                exclude,
                b"# File patterns to ignore; see `git help ignore` for more information.\n# Lines that start with '#' are comments.\n.manyhands/worktrees/\n"
            );
        }
        let registry = registry_row(data.path(), root).unwrap();
        assert_eq!(registry.0, "accessible");
        assert_eq!(registry.2, 1);
        assert!(!registry.1.is_empty());
        assert_eq!(registry.1, head_configuration_oid(&repository).to_string());
        assert_eq!(registry_row_count(data.path()), registrations_before + 1);
    }
    assert_eq!(registry_row_count(data.path()), 2);
}

#[test]
fn recovery_before_publication_configuration_commit_restores_config_and_live_index() {
    let data = tempfile::tempdir().unwrap();
    let fixture = support::born_repository();
    let setup = RepositoryService::open_at(data.path()).unwrap();
    setup.enable(enable_request(&fixture.root, "main")).unwrap();
    setup
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&fixture);
    std::fs::write(fixture.root.join("fixture.txt"), "staged but unrelated\n").unwrap();
    let mut index = fixture.repository.index().unwrap();
    index.add_path(std::path::Path::new("fixture.txt")).unwrap();
    index.write().unwrap();
    let config_before = support::tracked_configuration(&fixture.root);
    let index_before = support::index_bytes(&fixture.repository);
    let commits_before = commit_count(&fixture.repository);
    let remotes_before = remote_names(&fixture.repository);
    let registry_before = registry_row(data.path(), &fixture.root);
    let service = failing_service(
        data.path(),
        FailurePoint::BeforePublicationConfigurationCommit,
    );

    let error = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap_err();

    assert_eq!(error.operation, RepositoryOperation::SetPublicationRemote);
    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    assert_eq!(support::tracked_configuration(&fixture.root), config_before);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(commit_count(&fixture.repository), commits_before);
    assert_eq!(remote_names(&fixture.repository), remotes_before);
    assert_eq!(registry_row(data.path(), &fixture.root), registry_before);
    assert_eq!(
        canonical::parse_repository_config(
            std::str::from_utf8(&support::tracked_configuration(&fixture.root).unwrap()).unwrap()
        )
        .unwrap()
        .publication_remote,
        None
    );

    let PublicationRemoteOutcome::Changed { commit_oid } = service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap()
    else {
        panic!("expected the unfinished publication commit");
    };
    assert_eq!(commit_count(&fixture.repository), commits_before + 1);
    assert_eq!(support::index_bytes(&fixture.repository), index_before);
    assert_eq!(remote_names(&fixture.repository), remotes_before);
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_eq!(
        head_configuration(&fixture.repository),
        support::tracked_configuration(&fixture.root).unwrap()
    );
    let worktree_config = canonical::parse_repository_config(
        std::str::from_utf8(&support::tracked_configuration(&fixture.root).unwrap()).unwrap(),
    )
    .unwrap();
    let head_config = canonical::parse_repository_config(
        std::str::from_utf8(&head_configuration(&fixture.repository)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        worktree_config.publication_remote.as_deref(),
        Some("origin")
    );
    assert_eq!(head_config.publication_remote.as_deref(), Some("origin"));
    let registry = registry_row(data.path(), &fixture.root).unwrap();
    assert_eq!(registry_row_count(data.path()), 1);
    assert_eq!(
        registry.1,
        head_configuration_oid(&fixture.repository).to_string()
    );
    assert_eq!(registry.2, 1);
}

#[test]
fn recovery_before_registry_write_preserves_authoritative_commit_then_retries_registration() {
    let data = tempfile::tempdir().unwrap();
    let fixture = support::born_repository();
    let service = failing_service(data.path(), FailurePoint::BeforeRegistryWrite);

    let outcome = service
        .enable(enable_request(&fixture.root, "main"))
        .unwrap();

    let EnableRepositoryOutcome::RegistrationPending { commit_oid } = outcome else {
        panic!("expected registration pending after the authoritative commit");
    };
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert!(support::tracked_configuration(&fixture.root).is_some());
    assert_eq!(commit_count(&fixture.repository), 2);
    assert_eq!(registry_row_count(data.path()), 0);

    assert_eq!(
        service
            .enable(enable_request(&fixture.root, "main"))
            .unwrap(),
        EnableRepositoryOutcome::AlreadyEnabled
    );
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_eq!(commit_count(&fixture.repository), 2);
    assert_eq!(registry_row_count(data.path()), 1);

    let publication_data = tempfile::tempdir().unwrap();
    let publication_fixture = support::born_repository();
    let setup = RepositoryService::open_at(publication_data.path()).unwrap();
    setup
        .enable(enable_request(&publication_fixture.root, "main"))
        .unwrap();
    setup
        .add_remote(add_remote_request(
            &publication_fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    stage_configuration(&publication_fixture);
    let publication_service =
        failing_service(publication_data.path(), FailurePoint::BeforeRegistryWrite);

    let outcome = publication_service
        .set_publication_remote(publication_request(
            &publication_fixture.root,
            Some("origin"),
        ))
        .unwrap();

    let PublicationRemoteOutcome::RegistrationPending { commit_oid } = outcome else {
        panic!("expected registration pending after the publication commit");
    };
    assert_eq!(
        support::head_commit(&publication_fixture.repository),
        Some(commit_oid)
    );
    assert_eq!(commit_count(&publication_fixture.repository), 3);
    assert_eq!(registry_row_count(publication_data.path()), 1);
    let pending_config = support::tracked_configuration(&publication_fixture.root).unwrap();
    let pending_index = support::index_bytes(&publication_fixture.repository);
    let pending_remotes = remote_names(&publication_fixture.repository);
    let pending_registry =
        registry_row(publication_data.path(), &publication_fixture.root).unwrap();
    assert_eq!(
        head_configuration(&publication_fixture.repository),
        pending_config
    );
    assert_eq!(
        canonical::parse_repository_config(std::str::from_utf8(&pending_config).unwrap())
            .unwrap()
            .publication_remote,
        Some("origin".to_owned())
    );
    assert_eq!(
        publication_service
            .set_publication_remote(publication_request(
                &publication_fixture.root,
                Some("origin")
            ))
            .unwrap(),
        PublicationRemoteOutcome::NoChange
    );
    assert_eq!(
        support::head_commit(&publication_fixture.repository),
        Some(commit_oid)
    );
    assert_eq!(commit_count(&publication_fixture.repository), 3);
    assert_eq!(
        support::tracked_configuration(&publication_fixture.root),
        Some(pending_config)
    );
    assert_eq!(
        head_configuration_oid(&publication_fixture.repository).to_string(),
        registry_row(publication_data.path(), &publication_fixture.root)
            .unwrap()
            .1
    );
    assert_ne!(
        pending_registry.1,
        head_configuration_oid(&publication_fixture.repository).to_string()
    );
    assert_eq!(
        support::index_bytes(&publication_fixture.repository),
        pending_index
    );
    assert_eq!(
        remote_names(&publication_fixture.repository),
        pending_remotes
    );
    assert_eq!(registry_row_count(publication_data.path()), 1);
}
